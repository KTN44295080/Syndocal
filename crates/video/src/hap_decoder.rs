use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use hap_parser::TextureFormat;
use mp4::{Mp4Reader, TrackType};
use protocol::{VideoLayerId, VideoSourceKind};
use serde::Serialize;

use super::{
    render_input_cache_owner_matches, FfmpegCliFrameDecoder, LibavFrameDecoder,
    StillImageSignature, VideoDecodeError, VideoFrame, VideoFrameDecoder, VideoFrameRequest,
    VideoPixelFormat, VideoRenderInput,
};

#[derive(Debug, Clone, Copy, Default, Serialize, PartialEq, Eq)]
pub struct VideoDecoderDiagnostics {
    pub total_requests: u64,
    pub hap_requests: u64,
    pub hap_successes: u64,
    pub hap_failures: u64,
    pub libav_requests: u64,
    pub libav_successes: u64,
    pub libav_failures: u64,
    pub cli_fallback_requests: u64,
    pub cli_fallback_successes: u64,
    pub cli_fallback_failures: u64,
    pub deferred_requests: u64,
    pub decode_failures: u64,
    pub hap_cache_len: usize,
    pub libav_cache_len: usize,
    pub libav_session_count: usize,
    pub libav_session_open_count: u64,
    pub libav_session_reset_count: u64,
    pub libav_sequential_continue_count: u64,
    pub libav_frame_reuse_count: u64,
    pub libav_working_set_eviction_count: u64,
    pub libav_session_error_count: u64,
    pub libav_hardware_session_count: usize,
    pub libav_hardware_frame_count: u64,
    pub libav_hardware_error_count: u64,
    pub cli_cache_len: usize,
}

pub struct PreferredVideoFrameDecoder {
    hap: HapMovFrameDecoder,
    libav: LibavFrameDecoder,
    fallback: FfmpegCliFrameDecoder,
    diagnostics: VideoDecoderDiagnostics,
}

pub struct HapMovFrameDecoder {
    movies: Vec<HapMovie>,
    frames: Vec<HapFrameCacheEntry>,
}

struct HapMovie {
    key: protocol::VideoRenderInputKey,
    path: PathBuf,
    signature: StillImageSignature,
    reader: Mp4Reader<BufReader<File>>,
    track_id: u32,
    width: u32,
    height: u32,
    timescale: u32,
    sample_count: u32,
    duration_ms: u64,
}

#[derive(Clone)]
struct HapFrameCacheEntry {
    key: protocol::VideoRenderInputKey,
    layer_id: VideoLayerId,
    path: PathBuf,
    position_ms: u64,
    signature: StillImageSignature,
    frame: VideoFrame,
}

impl Default for HapMovFrameDecoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PreferredVideoFrameDecoder {
    fn default() -> Self {
        Self::from_env()
    }
}

impl PreferredVideoFrameDecoder {
    #[cfg(all(test, feature = "libav"))]
    pub(crate) fn with_software_libav_for_tests(fallback: FfmpegCliFrameDecoder) -> Self {
        Self {
            libav: LibavFrameDecoder::software_for_tests(),
            ..Self::new(fallback)
        }
    }

    pub fn new(fallback: FfmpegCliFrameDecoder) -> Self {
        Self {
            hap: HapMovFrameDecoder::new(),
            libav: LibavFrameDecoder::new(),
            fallback,
            diagnostics: VideoDecoderDiagnostics::default(),
        }
    }

    pub fn from_env() -> Self {
        Self::new(FfmpegCliFrameDecoder::from_env())
    }

    pub fn hap(&self) -> &HapMovFrameDecoder {
        &self.hap
    }

    pub fn cache_len(&self) -> usize {
        self.hap.frame_cache_len() + self.libav.cache_len() + self.fallback.cache_len()
    }

    pub fn release_layer(&mut self, layer_id: VideoLayerId) {
        self.hap.release_layer(layer_id);
        self.libav.release_layer(layer_id);
        VideoFrameDecoder::release_layer(&mut self.fallback, layer_id);
    }

    pub fn release_input(&mut self, input: &VideoRenderInput) {
        self.hap.release_input_key(input.key, input.layer_id());
        self.libav.release_input(input);
        VideoFrameDecoder::release_input(&mut self.fallback, input);
    }

    pub fn diagnostics(&self) -> VideoDecoderDiagnostics {
        let libav_sessions = self.libav.session_diagnostics();
        VideoDecoderDiagnostics {
            hap_cache_len: self.hap.frame_cache_len(),
            libav_cache_len: self.libav.cache_len(),
            libav_session_count: libav_sessions.active_sessions,
            libav_session_open_count: libav_sessions.opens,
            libav_session_reset_count: libav_sessions.resets,
            libav_sequential_continue_count: libav_sessions.sequential_continues,
            libav_frame_reuse_count: libav_sessions.frame_reuses,
            libav_working_set_eviction_count: libav_sessions.evictions,
            libav_session_error_count: libav_sessions.errors,
            libav_hardware_session_count: libav_sessions.hardware_sessions,
            libav_hardware_frame_count: libav_sessions.hardware_frames,
            libav_hardware_error_count: libav_sessions.hardware_errors,
            cli_cache_len: self.fallback.cache_len(),
            ..self.diagnostics
        }
    }

    fn decode_cli_fallback_input(
        &mut self,
        input: &VideoRenderInput,
        primary_error: Option<VideoDecodeError>,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.diagnostics.cli_fallback_requests =
            self.diagnostics.cli_fallback_requests.saturating_add(1);
        match self.fallback.decode_input_frame(input) {
            Ok(Some(frame)) => {
                self.diagnostics.cli_fallback_successes =
                    self.diagnostics.cli_fallback_successes.saturating_add(1);
                Ok(Some(frame))
            }
            Ok(None) => {
                self.diagnostics.deferred_requests =
                    self.diagnostics.deferred_requests.saturating_add(1);
                Ok(None)
            }
            Err(fallback_error) => {
                self.diagnostics.cli_fallback_failures =
                    self.diagnostics.cli_fallback_failures.saturating_add(1);
                self.diagnostics.decode_failures =
                    self.diagnostics.decode_failures.saturating_add(1);
                Err(primary_error.unwrap_or(fallback_error))
            }
        }
    }

    fn decode_general_input(
        &mut self,
        input: &VideoRenderInput,
        primary_error: Option<VideoDecodeError>,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let request = &input.request;
        if request.source.kind != VideoSourceKind::File {
            self.diagnostics.deferred_requests =
                self.diagnostics.deferred_requests.saturating_add(1);
            return match primary_error {
                Some(error) => Err(error),
                None => Ok(None),
            };
        }
        self.diagnostics.libav_requests = self.diagnostics.libav_requests.saturating_add(1);
        match self.libav.decode_input_frame(input) {
            Ok(Some(frame)) => {
                self.diagnostics.libav_successes =
                    self.diagnostics.libav_successes.saturating_add(1);
                Ok(Some(frame))
            }
            Ok(None) => self.decode_cli_fallback_input(input, primary_error),
            Err(libav_error) => {
                self.diagnostics.libav_failures = self.diagnostics.libav_failures.saturating_add(1);
                if matches!(libav_error, VideoDecodeError::HardwareDecode { .. }) {
                    self.diagnostics.decode_failures =
                        self.diagnostics.decode_failures.saturating_add(1);
                    return Err(libav_error);
                }
                self.decode_cli_fallback_input(input, primary_error.or(Some(libav_error)))
            }
        }
    }
}

impl VideoFrameDecoder for PreferredVideoFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.hap.retain_layers(layer_ids);
        self.libav.retain_layers(layer_ids);
        self.fallback.retain_layers(layer_ids);
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.hap.retain_inputs(inputs);
        self.libav.retain_inputs(inputs);
        self.fallback.retain_inputs(inputs);
    }

    fn release_layer(&mut self, layer_id: VideoLayerId) {
        PreferredVideoFrameDecoder::release_layer(self, layer_id);
    }

    fn release_input(&mut self, input: &VideoRenderInput) {
        PreferredVideoFrameDecoder::release_input(self, input);
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input_frame(&VideoRenderInput::legacy(request.clone()))
    }

    fn decode_input_frame(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let request = &input.request;
        self.diagnostics.total_requests = self.diagnostics.total_requests.saturating_add(1);
        let has_file_path = request.source.kind == VideoSourceKind::File
            && request
                .source
                .path
                .as_deref()
                .is_some_and(|path| !path.trim().is_empty());
        if !has_file_path {
            self.release_input(input);
        }
        if HapMovFrameDecoder::supports_request(request) {
            self.libav.release_input(input);
            VideoFrameDecoder::release_input(&mut self.fallback, input);
            self.diagnostics.hap_requests = self.diagnostics.hap_requests.saturating_add(1);
            return match self.hap.decode_input_frame(input) {
                Ok(Some(frame)) => {
                    self.diagnostics.hap_successes =
                        self.diagnostics.hap_successes.saturating_add(1);
                    Ok(Some(frame))
                }
                Ok(None) => {
                    self.diagnostics.deferred_requests =
                        self.diagnostics.deferred_requests.saturating_add(1);
                    Ok(None)
                }
                Err(error) => {
                    self.diagnostics.hap_failures = self.diagnostics.hap_failures.saturating_add(1);
                    self.decode_general_input(input, Some(error))
                }
            };
        }
        self.decode_general_input(input, None)
    }
}

impl HapMovFrameDecoder {
    pub fn new() -> Self {
        Self {
            movies: Vec::new(),
            frames: Vec::new(),
        }
    }

    fn release_layer(&mut self, layer_id: VideoLayerId) {
        self.frames.retain(|frame| frame.layer_id != layer_id);
        self.retain_movies_referenced_by_frames();
    }

    fn release_input_key(&mut self, key: protocol::VideoRenderInputKey, layer_id: VideoLayerId) {
        self.frames.retain(|frame| {
            !render_input_cache_owner_matches(key, layer_id, frame.key, frame.layer_id)
        });
        self.retain_movies_referenced_by_frames();
    }

    fn retain_movies_referenced_by_frames(&mut self) {
        self.movies.retain(|movie| {
            self.frames.iter().any(|frame| {
                frame.key == movie.key
                    && frame.path == movie.path
                    && frame.signature == movie.signature
            })
        });
    }

    pub fn supports_request(request: &VideoFrameRequest) -> bool {
        if request.source.kind != VideoSourceKind::File {
            return false;
        }
        if request
            .source
            .codec
            .as_deref()
            .is_some_and(|codec| codec.to_ascii_lowercase().contains("hap"))
        {
            return true;
        }
        request
            .source
            .path
            .as_deref()
            .and_then(|path| Path::new(path).extension())
            .is_some_and(|extension| {
                matches!(
                    extension.to_string_lossy().to_ascii_lowercase().as_str(),
                    "hap" | "hapq"
                )
            })
    }

    pub fn movie_cache_len(&self) -> usize {
        self.movies.len()
    }

    pub fn frame_cache_len(&self) -> usize {
        self.frames.len()
    }

    fn movie_index(
        &mut self,
        key: protocol::VideoRenderInputKey,
        path: &Path,
        signature: &StillImageSignature,
        request: &VideoFrameRequest,
    ) -> Result<usize, VideoDecodeError> {
        if let Some(index) = self.movies.iter().position(|movie| {
            movie.key == key && movie.path == path && movie.signature == *signature
        }) {
            return Ok(index);
        }

        self.movies
            .retain(|movie| movie.key != key || movie.path != path);
        let file = File::open(path).map_err(|error| decode_error(request, error.to_string()))?;
        let size = file
            .metadata()
            .map_err(|error| decode_error(request, error.to_string()))?
            .len();
        let reader = Mp4Reader::read_header(BufReader::new(file), size)
            .map_err(|error| decode_error(request, format!("invalid HAP MOV/MP4: {error}")))?;
        let track = reader
            .tracks()
            .values()
            .find(|track| track.track_type().ok() == Some(TrackType::Video))
            .ok_or_else(|| decode_error(request, "HAP container has no video track"))?;
        let track_id = track.track_id();
        let width = u32::from(track.width());
        let height = u32::from(track.height());
        let timescale = track.timescale();
        let sample_count = track.sample_count();
        let duration_ms = track.duration().as_millis() as u64;
        if width == 0 || height == 0 || timescale == 0 || sample_count == 0 {
            return Err(decode_error(
                request,
                "HAP video track has invalid dimensions, timescale, or sample count",
            ));
        }
        self.movies.push(HapMovie {
            key,
            path: path.to_path_buf(),
            signature: signature.clone(),
            reader,
            track_id,
            width,
            height,
            timescale,
            sample_count,
            duration_ms,
        });
        Ok(self.movies.len() - 1)
    }

    fn decode_uncached(
        &mut self,
        key: protocol::VideoRenderInputKey,
        request: &VideoFrameRequest,
        path: &Path,
        signature: &StillImageSignature,
    ) -> Result<VideoFrame, VideoDecodeError> {
        let movie_index = self.movie_index(key, path, signature, request)?;
        let movie = &mut self.movies[movie_index];
        let bounded_position_ms = if movie.duration_ms == 0 {
            0
        } else {
            request.position_ms.min(movie.duration_ms.saturating_sub(1))
        };
        let mut sample_id = if movie.duration_ms == 0 {
            1
        } else {
            ((bounded_position_ms as u128 * movie.sample_count as u128) / movie.duration_ms as u128)
                as u32
                + 1
        }
        .clamp(1, movie.sample_count);
        let target_time = (bounded_position_ms as u128 * movie.timescale as u128 / 1000) as u64;

        let sample = loop {
            let sample = movie
                .reader
                .read_sample(movie.track_id, sample_id)
                .map_err(|error| {
                    decode_error(request, format!("failed to read HAP sample: {error}"))
                })?
                .ok_or_else(|| {
                    decode_error(request, format!("HAP sample {sample_id} is missing"))
                })?;
            if target_time < sample.start_time && sample_id > 1 {
                sample_id -= 1;
                continue;
            }
            if target_time >= sample.start_time.saturating_add(u64::from(sample.duration))
                && sample_id < movie.sample_count
            {
                sample_id += 1;
                continue;
            }
            break sample;
        };

        let parsed = hap_parser::parse_frame(&sample.bytes)
            .map_err(|error| decode_error(request, format!("invalid HAP frame: {error}")))?;
        let pts_ms = sample.start_time.saturating_mul(1000) / u64::from(movie.timescale);
        let duration_ms =
            (u64::from(sample.duration).saturating_mul(1000) / u64::from(movie.timescale)).max(1);
        if let Some(alpha) = parsed.alpha {
            if parsed.format != TextureFormat::YcoCgDxt5
                || alpha.format != TextureFormat::AlphaRgtc1
            {
                return Err(decode_error(
                    request,
                    "HAP dual-plane frame must contain YCoCg DXT5 color and RGTC1 alpha",
                ));
            }
            let expected_color_len = parsed.format.frame_size(movie.width, movie.height);
            let expected_alpha_len = alpha.format.frame_size(movie.width, movie.height);
            if parsed.data.len() != expected_color_len || alpha.data.len() != expected_alpha_len {
                return Err(decode_error(
                    request,
                    format!(
                        "HAP Q Alpha planes have color/alpha lengths {}/{}, expected {expected_color_len}/{expected_alpha_len}",
                        parsed.data.len(),
                        alpha.data.len()
                    ),
                ));
            }
            let compressed = VideoFrame {
                layer_id: request.layer_id,
                width: movie.width,
                height: movie.height,
                pts_ms,
                duration_ms,
                format: VideoPixelFormat::YcoCgDxt5,
                data: parsed.data,
            };
            let mut rgba = super::convert_frame_to_rgba8(&compressed).map_err(|error| {
                decode_error(request, format!("HAP Q decode failed: {error:?}"))
            })?;
            apply_rgtc1_alpha(
                &mut rgba.data,
                &alpha.data,
                movie.width,
                movie.height,
                request,
            )?;
            return Ok(rgba);
        }
        let expected_len = parsed.format.frame_size(movie.width, movie.height);
        if parsed.data.len() != expected_len {
            return Err(decode_error(
                request,
                format!(
                    "HAP frame has {} decompressed bytes, expected {expected_len} for {}x{}",
                    parsed.data.len(),
                    movie.width,
                    movie.height
                ),
            ));
        }
        if parsed.format == TextureFormat::RgbaBc7 {
            return Ok(VideoFrame {
                layer_id: request.layer_id,
                width: movie.width,
                height: movie.height,
                pts_ms,
                duration_ms,
                format: VideoPixelFormat::Bc7,
                data: parsed.data,
            });
        }
        let format = match parsed.format {
            TextureFormat::RgbDxt1 => VideoPixelFormat::Dxt1,
            TextureFormat::RgbaDxt5 => VideoPixelFormat::Dxt5,
            TextureFormat::YcoCgDxt5 => VideoPixelFormat::YcoCgDxt5,
            TextureFormat::AlphaRgtc1 => {
                return Err(decode_error(
                    request,
                    "HAP alpha-only BC4 frames are not supported by this decoder stage",
                ));
            }
            TextureFormat::RgbaBc7 => unreachable!("BC7 returned as RGBA8 above"),
        };

        Ok(VideoFrame {
            layer_id: request.layer_id,
            width: movie.width,
            height: movie.height,
            pts_ms,
            duration_ms,
            format,
            data: parsed.data,
        })
    }
}

fn apply_rgtc1_alpha(
    rgba: &mut [u8],
    blocks: &[u8],
    width: u32,
    height: u32,
    request: &VideoFrameRequest,
) -> Result<(), VideoDecodeError> {
    let blocks_x = width.div_ceil(4) as usize;
    let blocks_y = height.div_ceil(4) as usize;
    let expected_len = blocks_x
        .checked_mul(blocks_y)
        .and_then(|count| count.checked_mul(8))
        .ok_or_else(|| decode_error(request, "HAP Q Alpha dimensions overflow"))?;
    if width == 0
        || height == 0
        || blocks.len() != expected_len
        || rgba.len() != width as usize * height as usize * 4
    {
        return Err(decode_error(
            request,
            "HAP Q Alpha BC4 plane size does not match the color frame",
        ));
    }
    for (block_index, block) in blocks.chunks_exact(8).enumerate() {
        let block_x = block_index % blocks_x;
        let block_y = block_index / blocks_x;
        let palette = super::dxt5_alpha_palette(block[0], block[1]);
        let mut indices = 0_u64;
        for (index, byte) in block[2..8].iter().enumerate() {
            indices |= u64::from(*byte) << (index * 8);
        }
        for local_y in 0..4_usize {
            let y = block_y * 4 + local_y;
            if y >= height as usize {
                continue;
            }
            for local_x in 0..4_usize {
                let x = block_x * 4 + local_x;
                if x >= width as usize {
                    continue;
                }
                let pixel_index = local_y * 4 + local_x;
                let alpha_index = ((indices >> (pixel_index * 3)) & 0x07) as usize;
                rgba[(y * width as usize + x) * 4 + 3] = palette[alpha_index];
            }
        }
    }
    Ok(())
}

impl VideoFrameDecoder for HapMovFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.frames
            .retain(|entry| layer_ids.contains(&entry.layer_id));
        self.movies
            .retain(|movie| self.frames.iter().any(|entry| entry.path == movie.path));
    }

    fn retain_inputs(&mut self, inputs: &[VideoRenderInput]) {
        self.frames.retain(|entry| {
            inputs.iter().any(|input| {
                render_input_cache_owner_matches(
                    input.key,
                    input.layer_id(),
                    entry.key,
                    entry.layer_id,
                )
            })
        });
        self.movies.retain(|movie| {
            self.frames.iter().any(|frame| {
                frame.key == movie.key
                    && frame.path == movie.path
                    && frame.signature == movie.signature
            })
        });
    }

    fn release_input(&mut self, input: &VideoRenderInput) {
        self.release_input_key(input.key, input.layer_id());
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input_frame(&VideoRenderInput::legacy(request.clone()))
    }

    fn decode_input_frame(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        let request = &input.request;
        if !Self::supports_request(request) {
            return Ok(None);
        }
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
        if let Some(entry) = self.frames.iter().find(|entry| {
            entry.key == input.key
                && entry.layer_id == request.layer_id
                && entry.path == path
                && entry.position_ms == request.position_ms
                && entry.signature == signature
        }) {
            return Ok(Some(entry.frame.clone()));
        }
        let frame = self.decode_uncached(input.key, request, &path, &signature)?;
        self.frames.retain(|entry| {
            !render_input_cache_owner_matches(
                input.key,
                request.layer_id,
                entry.key,
                entry.layer_id,
            )
        });
        self.frames.push(HapFrameCacheEntry {
            key: input.key,
            layer_id: request.layer_id,
            path,
            position_ms: request.position_ms,
            signature,
            frame: frame.clone(),
        });
        Ok(Some(frame))
    }
}

fn decode_error(request: &VideoFrameRequest, message: impl Into<String>) -> VideoDecodeError {
    VideoDecodeError::Decode {
        layer_id: request.layer_id,
        label: request.label.clone(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        io::Cursor,
        sync::atomic::{AtomicU64, Ordering},
        time::SystemTime,
    };

    use mp4::{AvcConfig, Bytes, FourCC, Mp4Config, Mp4Sample, Mp4Writer, TrackConfig};
    use protocol::{VideoMediaMetadata, VideoSourceSummary};

    use super::*;

    static NEXT_TEST_FILE: AtomicU64 = AtomicU64::new(1);

    fn hap_section(section_type: u8, payload: &[u8]) -> Vec<u8> {
        let len = payload.len();
        let mut bytes = vec![len as u8, (len >> 8) as u8, (len >> 16) as u8, section_type];
        bytes.extend_from_slice(payload);
        bytes
    }

    fn write_hap_movie_with_fourcc(
        frames: &[Vec<u8>],
        width: u16,
        height: u16,
        sample_entry: [u8; 4],
    ) -> PathBuf {
        let config = Mp4Config {
            major_brand: FourCC::from(*b"isom"),
            minor_version: 512,
            compatible_brands: vec![FourCC::from(*b"isom"), FourCC::from(*b"mp41")],
            timescale: 1000,
        };
        let mut writer = Mp4Writer::write_start(Cursor::new(Vec::new()), &config).unwrap();
        writer
            .add_track(&TrackConfig::from(AvcConfig {
                width,
                height,
                seq_param_set: vec![0x67, 0x42, 0x00, 0x1e],
                pic_param_set: vec![0x68, 0xce, 0x06, 0xe2],
            }))
            .unwrap();
        for (index, frame) in frames.iter().enumerate() {
            writer
                .write_sample(
                    1,
                    &Mp4Sample {
                        start_time: index as u64 * 40,
                        duration: 40,
                        rendering_offset: 0,
                        is_sync: true,
                        bytes: Bytes::copy_from_slice(frame),
                    },
                )
                .unwrap();
        }
        writer.write_end().unwrap();
        let mut bytes = writer.into_writer().into_inner();
        let mut replacements = 0;
        for index in 0..bytes.len().saturating_sub(3) {
            if bytes[index..index + 4] == *b"avc1" {
                bytes[index..index + 4].copy_from_slice(&sample_entry);
                replacements += 1;
            }
        }
        assert_eq!(replacements, 1);
        let serial = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "syndocal-hap-decoder-{}-{serial}-{}.mov",
            std::process::id(),
            NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, bytes).unwrap();
        path
    }

    fn write_hap_movie(frames: &[Vec<u8>], width: u16, height: u16) -> PathBuf {
        write_hap_movie_with_fourcc(frames, width, height, *b"Hap1")
    }

    fn write_bits(block: &mut [u8; 16], cursor: &mut usize, value: u32, bit_count: usize) {
        for bit in 0..bit_count {
            if value & (1 << bit) != 0 {
                block[*cursor / 8] |= 1 << (*cursor % 8);
            }
            *cursor += 1;
        }
    }

    fn solid_bc7_mode6(rgba: [u8; 4]) -> [u8; 16] {
        assert!(rgba.iter().all(|channel| channel & 1 == rgba[0] & 1));
        let mut block = [0_u8; 16];
        let mut cursor = 0;
        write_bits(&mut block, &mut cursor, 1 << 6, 7);
        for channel in rgba {
            write_bits(&mut block, &mut cursor, u32::from(channel >> 1), 7);
            write_bits(&mut block, &mut cursor, u32::from(channel >> 1), 7);
        }
        write_bits(&mut block, &mut cursor, u32::from(rgba[0] & 1), 1);
        write_bits(&mut block, &mut cursor, u32::from(rgba[0] & 1), 1);
        write_bits(&mut block, &mut cursor, 0, 3);
        for _ in 1..16 {
            write_bits(&mut block, &mut cursor, 0, 4);
        }
        assert_eq!(cursor, 128);
        block
    }

    fn request(path: &Path, position_ms: u64) -> VideoFrameRequest {
        VideoFrameRequest {
            layer_id: 7,
            label: "HAP test".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(path.display().to_string()),
                name: None,
                codec: Some("hap".to_string()),
                metadata: Some(VideoMediaMetadata {
                    duration_ms: Some(80),
                    width: Some(4),
                    height: Some(4),
                    frame_rate: Some(25.0),
                    has_audio: false,
                }),
            },
            position_ms,
            width: 1920,
            height: 1080,
        }
    }

    #[test]
    fn decodes_hap_mov_samples_in_process_at_native_size() {
        let first = hap_section(0xAB, &[1, 2, 3, 4, 5, 6, 7, 8]);
        let second = hap_section(0xAB, &[8, 7, 6, 5, 4, 3, 2, 1]);
        let path = write_hap_movie(&[first, second], 4, 4);
        let mut decoder = HapMovFrameDecoder::new();

        let frame = decoder.decode_frame(&request(&path, 55)).unwrap().unwrap();

        assert_eq!(frame.width, 4);
        assert_eq!(frame.height, 4);
        assert_eq!(frame.pts_ms, 40);
        assert_eq!(frame.duration_ms, 40);
        assert_eq!(frame.format, VideoPixelFormat::Dxt1);
        assert_eq!(frame.data, vec![8, 7, 6, 5, 4, 3, 2, 1]);
        assert_eq!(decoder.movie_cache_len(), 1);
        assert_eq!(decoder.frame_cache_len(), 1);
        let cached = decoder.decode_frame(&request(&path, 55)).unwrap().unwrap();
        assert_eq!(cached, frame);
        assert_eq!(decoder.movie_cache_len(), 1);

        fs::remove_file(path).unwrap();
    }

    #[test]
    fn legacy_hap_cache_retains_and_releases_by_layer_owner() {
        let path = write_hap_movie(&[hap_section(0xAB, &[1, 2, 3, 4, 5, 6, 7, 8])], 4, 4);
        let mut decoder = HapMovFrameDecoder::new();
        let first_request = request(&path, 0);
        let mut second_request = first_request.clone();
        second_request.layer_id = 8;

        decoder.decode_frame(&first_request).unwrap().unwrap();
        decoder.decode_frame(&second_request).unwrap().unwrap();
        assert_eq!(decoder.movie_cache_len(), 1);
        assert_eq!(decoder.frame_cache_len(), 2);

        decoder.retain_inputs(&[VideoRenderInput::legacy(second_request.clone())]);
        assert_eq!(decoder.movie_cache_len(), 1);
        assert_eq!(decoder.frame_cache_len(), 1);

        decoder.release_layer(8);
        assert_eq!(decoder.movie_cache_len(), 0);
        assert_eq!(decoder.frame_cache_len(), 0);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn decodes_hap_q_as_explicit_ycocg_dxt5() {
        let path = write_hap_movie(&[hap_section(0xAF, &[0; 16])], 4, 4);
        let mut decoder = HapMovFrameDecoder::new();

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::YcoCgDxt5);
        assert_eq!(frame.data, vec![0; 16]);
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn decodes_hap_q_alpha_color_and_bc4_plane_to_rgba() {
        let color = hap_section(0xAF, &[0; 16]);
        let alpha = hap_section(0xA1, &[64, 0, 0, 0, 0, 0, 0, 0]);
        let mut planes = color;
        planes.extend_from_slice(&alpha);
        let path = write_hap_movie(&[hap_section(0x0D, &planes)], 4, 4);
        let mut decoder = HapMovFrameDecoder::new();

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::Rgba8);
        assert_eq!(frame.data.len(), 4 * 4 * 4);
        assert!(frame.data.chunks_exact(4).all(|pixel| pixel[3] == 64));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn decodes_bc7_blocks_to_cropped_rgba8() {
        let color = [200, 100, 50, 128];
        let blocks = solid_bc7_mode6(color).repeat(2);
        let frame = VideoFrame {
            layer_id: 7,
            width: 5,
            height: 3,
            pts_ms: 0,
            duration_ms: 40,
            format: VideoPixelFormat::Bc7,
            data: blocks.clone(),
        };

        let rgba = super::super::convert_frame_to_rgba8(&frame).unwrap();

        assert_eq!(rgba.data.len(), 5 * 3 * 4);
        assert!(rgba.data.chunks_exact(4).all(|pixel| pixel == color));
        let invalid = VideoFrame {
            data: blocks[..16].to_vec(),
            ..frame
        };
        assert!(super::super::convert_frame_to_rgba8(&invalid).is_err());
    }

    #[test]
    fn decodes_hap_r_bc7_movie_in_process_with_alpha() {
        let color = [200, 100, 50, 128];
        let path = write_hap_movie_with_fourcc(
            &[hap_section(0xAC, &solid_bc7_mode6(color))],
            4,
            4,
            *b"Hap7",
        );
        let mut decoder = HapMovFrameDecoder::new();

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::Bc7);
        assert_eq!(frame.data.len(), 16);
        let rgba = super::super::convert_frame_to_rgba8(&frame).unwrap();
        assert!(rgba.data.chunks_exact(4).all(|pixel| pixel == color));
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn ignores_non_hap_sources() {
        let mut request = request(Path::new("clip.mp4"), 0);
        request.source.codec = Some("h264".to_string());
        let mut decoder = HapMovFrameDecoder::new();

        assert_eq!(decoder.decode_frame(&request).unwrap(), None);
    }

    #[test]
    fn preferred_decoder_routes_hap_without_invoking_ffmpeg() {
        let path = write_hap_movie(&[hap_section(0xAB, &[0; 8])], 4, 4);
        let missing_ffmpeg =
            std::env::temp_dir().join(format!("syndocal-missing-ffmpeg-{}", std::process::id()));
        let mut decoder =
            PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::new(missing_ffmpeg));

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::Dxt1);
        assert_eq!(decoder.hap().movie_cache_len(), 1);
        assert_eq!(decoder.cache_len(), 1);
        assert_eq!(
            decoder.diagnostics(),
            VideoDecoderDiagnostics {
                total_requests: 1,
                hap_requests: 1,
                hap_successes: 1,
                hap_cache_len: 1,
                ..VideoDecoderDiagnostics::default()
            }
        );
        fs::remove_file(path).unwrap();
    }

    #[test]
    fn preferred_decoder_falls_back_when_in_process_hap_decode_fails() {
        let missing_media =
            std::env::temp_dir().join(format!("syndocal-missing-hap-media-{}", std::process::id()));
        let missing_ffmpeg =
            std::env::temp_dir().join(format!("syndocal-missing-ffmpeg-{}", std::process::id()));
        let mut decoder =
            PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::new(missing_ffmpeg));

        assert!(decoder.decode_frame(&request(&missing_media, 0)).is_err());

        let diagnostics = decoder.diagnostics();
        assert_eq!(diagnostics.hap_requests, 1);
        assert_eq!(diagnostics.hap_failures, 1);
        assert_eq!(diagnostics.libav_requests, 1);
        assert_eq!(diagnostics.cli_fallback_requests, 1);
        assert_eq!(diagnostics.cli_fallback_failures, 1);
        assert_eq!(diagnostics.decode_failures, 1);
    }
}
