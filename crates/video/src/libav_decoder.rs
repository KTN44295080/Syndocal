use std::path::PathBuf;

use protocol::{VideoLayerId, VideoSourceKind};

#[cfg(feature = "libav")]
use super::VideoPixelFormat;
use super::{
    StillImageSignature, VideoDecodeError, VideoFrame, VideoFrameDecoder, VideoFrameRequest,
};

pub struct LibavFrameDecoder {
    entries: Vec<LibavFrameCacheEntry>,
}

#[derive(Clone)]
#[cfg_attr(not(feature = "libav"), allow(dead_code))]
struct LibavFrameCacheEntry {
    layer_id: VideoLayerId,
    path: PathBuf,
    width: u32,
    height: u32,
    position_ms: u64,
    signature: StillImageSignature,
    frame: VideoFrame,
}

impl Default for LibavFrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl LibavFrameDecoder {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    pub const fn is_built() -> bool {
        cfg!(feature = "libav")
    }

    pub fn cache_len(&self) -> usize {
        self.entries.len()
    }

    #[cfg(feature = "libav")]
    fn decode_enabled(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let path = request
            .source
            .path
            .as_deref()
            .filter(|path| !path.trim().is_empty())
            .ok_or_else(|| VideoDecodeError::MissingSourcePath {
                layer_id: request.layer_id,
                label: request.label.clone(),
            })?;
        let path = PathBuf::from(path);
        let signature = StillImageSignature::from_path(&path);
        if let Some(entry) = self.entries.iter().find(|entry| {
            entry.layer_id == request.layer_id
                && entry.path == path
                && entry.width == request.width
                && entry.height == request.height
                && entry.position_ms == request.position_ms
                && entry.signature == signature
        }) {
            return Ok(Some(entry.frame.clone()));
        }

        let frame = decode_libav_frame(request, &path)?;
        self.entries.retain(|entry| {
            !(entry.layer_id == request.layer_id
                && entry.path == path
                && entry.width == request.width
                && entry.height == request.height
                && entry.position_ms == request.position_ms)
        });
        self.entries.push(LibavFrameCacheEntry {
            layer_id: request.layer_id,
            path,
            width: request.width,
            height: request.height,
            position_ms: request.position_ms,
            signature,
            frame: frame.clone(),
        });
        Ok(Some(frame))
    }
}

impl VideoFrameDecoder for LibavFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.entries
            .retain(|entry| layer_ids.contains(&entry.layer_id));
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        if request.source.kind != VideoSourceKind::File {
            return Ok(None);
        }
        #[cfg(feature = "libav")]
        {
            self.decode_enabled(request)
        }
        #[cfg(not(feature = "libav"))]
        {
            Ok(None)
        }
    }
}

#[cfg(feature = "libav")]
fn decode_libav_frame(
    request: &VideoFrameRequest,
    path: &std::path::Path,
) -> Result<VideoFrame, VideoDecodeError> {
    use ffmpeg::{
        format::Pixel,
        media::Type,
        software::scaling::{context::Context, flag::Flags},
    };
    use ffmpeg_next as ffmpeg;

    ffmpeg::init().map_err(|error| decode_error(request, error))?;
    let mut input = ffmpeg::format::input(path).map_err(|error| decode_error(request, error))?;
    let stream = input
        .streams()
        .best(Type::Video)
        .ok_or_else(|| decode_message(request, "video stream not found"))?;
    let stream_index = stream.index();
    let time_base = stream.time_base();
    let context = ffmpeg::codec::context::Context::from_parameters(stream.parameters())
        .map_err(|error| decode_error(request, error))?;
    let mut decoder = context
        .decoder()
        .video()
        .map_err(|error| decode_error(request, error))?;
    let mut scaler = Context::get(
        decoder.format(),
        decoder.width(),
        decoder.height(),
        Pixel::RGBA,
        request.width,
        request.height,
        Flags::BILINEAR,
    )
    .map_err(|error| decode_error(request, error))?;

    let target_us = request
        .position_ms
        .saturating_mul(1_000)
        .min(i64::MAX as u64) as i64;
    if target_us > 0 && input.seek(target_us, ..target_us).is_ok() {
        decoder.flush();
    }
    let mut candidate = None;
    for (packet_stream, packet) in input.packets() {
        if packet_stream.index() != stream_index {
            continue;
        }
        decoder
            .send_packet(&packet)
            .map_err(|error| decode_error(request, error))?;
        if let Some(frame) = receive_target_frame(
            request,
            &mut decoder,
            &mut scaler,
            time_base,
            &mut candidate,
        )? {
            return Ok(frame);
        }
    }
    decoder
        .send_eof()
        .map_err(|error| decode_error(request, error))?;
    if let Some(frame) = receive_target_frame(
        request,
        &mut decoder,
        &mut scaler,
        time_base,
        &mut candidate,
    )? {
        return Ok(frame);
    }
    candidate.ok_or_else(|| decode_message(request, "decoder produced no video frame"))
}

#[cfg(feature = "libav")]
fn receive_target_frame(
    request: &VideoFrameRequest,
    decoder: &mut ffmpeg_next::decoder::Video,
    scaler: &mut ffmpeg_next::software::scaling::context::Context,
    time_base: ffmpeg_next::Rational,
    candidate: &mut Option<VideoFrame>,
) -> Result<Option<VideoFrame>, VideoDecodeError> {
    use ffmpeg_next::util::frame::video::Video;

    let mut decoded = Video::empty();
    while decoder.receive_frame(&mut decoded).is_ok() {
        let mut rgba = Video::empty();
        scaler
            .run(&decoded, &mut rgba)
            .map_err(|error| decode_error(request, error))?;
        let frame = copy_rgba_frame(request, &rgba, time_base)?;
        if frame.pts_ms >= request.position_ms {
            return Ok(Some(frame));
        }
        *candidate = Some(frame);
    }
    Ok(None)
}

#[cfg(feature = "libav")]
fn copy_rgba_frame(
    request: &VideoFrameRequest,
    frame: &ffmpeg_next::util::frame::video::Video,
    time_base: ffmpeg_next::Rational,
) -> Result<VideoFrame, VideoDecodeError> {
    let row_bytes = (request.width as usize)
        .checked_mul(4)
        .ok_or_else(|| decode_message(request, "RGBA row size overflow"))?;
    let data_len = row_bytes
        .checked_mul(request.height as usize)
        .ok_or_else(|| decode_message(request, "RGBA frame size overflow"))?;
    let source = frame.data(0);
    let stride = frame.stride(0);
    let mut data = vec![0; data_len];
    for row in 0..request.height as usize {
        let source_start = row
            .checked_mul(stride)
            .ok_or_else(|| decode_message(request, "RGBA source offset overflow"))?;
        let source_end = source_start
            .checked_add(row_bytes)
            .ok_or_else(|| decode_message(request, "RGBA source row overflow"))?;
        let destination_start = row * row_bytes;
        let source_row = source
            .get(source_start..source_end)
            .ok_or_else(|| decode_message(request, "RGBA source stride is too small"))?;
        data[destination_start..destination_start + row_bytes].copy_from_slice(source_row);
    }
    let pts_ms = frame
        .timestamp()
        .and_then(|timestamp| timestamp_ms(timestamp, time_base))
        .unwrap_or(request.position_ms);
    let duration_ms = request
        .source
        .metadata
        .and_then(|metadata| metadata.frame_rate)
        .filter(|rate| rate.is_finite() && *rate > 0.0)
        .map(|rate| (1_000.0 / rate).round().max(1.0) as u64)
        .unwrap_or(33);
    Ok(VideoFrame {
        layer_id: request.layer_id,
        width: request.width,
        height: request.height,
        pts_ms,
        duration_ms,
        format: VideoPixelFormat::Rgba8,
        data,
    })
}

#[cfg(feature = "libav")]
fn timestamp_ms(timestamp: i64, time_base: ffmpeg_next::Rational) -> Option<u64> {
    let numerator = i128::from(time_base.numerator());
    let denominator = i128::from(time_base.denominator());
    if timestamp < 0 || numerator <= 0 || denominator <= 0 {
        return None;
    }
    let milliseconds = i128::from(timestamp)
        .checked_mul(numerator)?
        .checked_mul(1_000)?
        .checked_div(denominator)?;
    u64::try_from(milliseconds).ok()
}

#[cfg(feature = "libav")]
fn decode_error(request: &VideoFrameRequest, error: impl std::fmt::Display) -> VideoDecodeError {
    decode_message(request, error.to_string())
}

#[cfg(feature = "libav")]
fn decode_message(request: &VideoFrameRequest, message: impl Into<String>) -> VideoDecodeError {
    VideoDecodeError::Decode {
        layer_id: request.layer_id,
        label: request.label.clone(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use protocol::{VideoSourceKind, VideoSourceSummary};

    use super::*;
    #[cfg(feature = "libav")]
    use crate::{FfmpegCliFrameDecoder, PreferredVideoFrameDecoder, VideoDecoderDiagnostics};

    fn request() -> VideoFrameRequest {
        VideoFrameRequest {
            layer_id: 1,
            label: "General codec".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("clip.mp4".to_string()),
                name: None,
                codec: Some("h264".to_string()),
                metadata: None,
            },
            position_ms: 0,
            width: 16,
            height: 16,
        }
    }

    #[cfg(not(feature = "libav"))]
    #[test]
    fn disabled_backend_defers_to_compatibility_decoder() {
        let mut decoder = LibavFrameDecoder::new();
        assert!(!LibavFrameDecoder::is_built());
        assert_eq!(decoder.decode_frame(&request()), Ok(None));
    }

    #[cfg(feature = "libav")]
    #[test]
    fn decodes_h264_h265_and_prores_in_process() {
        use std::process::Command;

        let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
        let encoder_output = Command::new(&ffmpeg)
            .args(["-hide_banner", "-encoders"])
            .output()
            .expect("FFmpeg encoder inventory must be available");
        assert!(encoder_output.status.success());
        let encoder_inventory = String::from_utf8_lossy(&encoder_output.stdout);
        let available_encoders = encoder_inventory
            .lines()
            .filter_map(|line| line.split_whitespace().nth(1))
            .collect::<std::collections::HashSet<_>>();
        assert!(LibavFrameDecoder::is_built());
        for (codec, encoder_candidates, extension, pixel_format) in [
            (
                "h264",
                &["libx264", "libopenh264"] as &[&str],
                "mp4",
                "yuv420p",
            ),
            (
                "hevc",
                &["libx265", "libkvazaar"] as &[&str],
                "mp4",
                "yuv420p",
            ),
            ("prores", &["prores_ks"] as &[&str], "mov", "yuv422p10le"),
        ] {
            let encoder = encoder_candidates
                .iter()
                .find(|candidate| available_encoders.contains(**candidate))
                .unwrap_or_else(|| panic!("no QA encoder is available for {codec}"));
            let path = std::env::temp_dir().join(format!(
                "syndocal-libav-{codec}-{}-{}.{}",
                std::process::id(),
                request().layer_id,
                extension
            ));
            let mut command = Command::new(&ffmpeg);
            command.args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=16x16:d=1:r=2",
                "-c:v",
                encoder,
                "-pix_fmt",
                pixel_format,
            ]);
            if *encoder == "libx265" {
                command.args(["-x265-params", "pools=1:frame-threads=1"]);
            }
            command.arg("-y").arg(&path);
            let output = command
                .output()
                .expect("FFmpeg QA fixture generator must be available");
            assert!(
                output.status.success(),
                "{codec} fixture generation failed: {}",
                String::from_utf8_lossy(&output.stderr)
            );
            let mut request = request();
            request.source.path = Some(path.to_string_lossy().into_owned());
            request.source.codec = Some(codec.to_string());
            request.position_ms = 500;
            request.source.metadata = Some(protocol::VideoMediaMetadata {
                duration_ms: Some(1_000),
                width: Some(16),
                height: Some(16),
                frame_rate: Some(2.0),
                has_audio: false,
            });
            let mut decoder = LibavFrameDecoder::new();

            let frame = decoder.decode_frame(&request).unwrap().unwrap();

            assert_eq!(frame.format, VideoPixelFormat::Rgba8, "{codec}");
            assert_eq!((frame.width, frame.height), (16, 16), "{codec}");
            assert_eq!(frame.data.len(), 16 * 16 * 4, "{codec}");
            assert!(frame.pts_ms >= 500, "{codec}: pts {}", frame.pts_ms);
            assert!(frame.data[0] > 200, "{codec}");
            assert!(frame.data[1] < 40, "{codec}");
            assert!(frame.data[2] < 40, "{codec}");
            assert_eq!(decoder.cache_len(), 1, "{codec}");
            assert_eq!(decoder.decode_frame(&request).unwrap().unwrap(), frame);

            let missing_ffmpeg = std::env::temp_dir().join(format!(
                "syndocal-missing-ffmpeg-{codec}-{}",
                std::process::id()
            ));
            let mut preferred =
                PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::new(missing_ffmpeg));
            assert_eq!(preferred.decode_frame(&request).unwrap().unwrap(), frame);
            assert_eq!(
                preferred.diagnostics(),
                VideoDecoderDiagnostics {
                    total_requests: 1,
                    libav_requests: 1,
                    libav_successes: 1,
                    libav_cache_len: 1,
                    ..VideoDecoderDiagnostics::default()
                },
                "{codec}"
            );
            let _ = std::fs::remove_file(path);
        }
    }
}
