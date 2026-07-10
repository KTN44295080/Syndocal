use std::{
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use hap_parser::TextureFormat;
use mp4::{Mp4Reader, TrackType};
use protocol::{VideoLayerId, VideoSourceKind};

use super::{
    FfmpegCliFrameDecoder, StillImageSignature, VideoDecodeError, VideoFrame, VideoFrameDecoder,
    VideoFrameRequest, VideoPixelFormat,
};

pub struct PreferredVideoFrameDecoder {
    hap: HapMovFrameDecoder,
    fallback: FfmpegCliFrameDecoder,
}

pub struct HapMovFrameDecoder {
    movies: Vec<HapMovie>,
    frames: Vec<HapFrameCacheEntry>,
}

struct HapMovie {
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
    pub fn new(fallback: FfmpegCliFrameDecoder) -> Self {
        Self {
            hap: HapMovFrameDecoder::new(),
            fallback,
        }
    }

    pub fn from_env() -> Self {
        Self::new(FfmpegCliFrameDecoder::from_env())
    }

    pub fn hap(&self) -> &HapMovFrameDecoder {
        &self.hap
    }

    pub fn cache_len(&self) -> usize {
        self.hap.frame_cache_len() + self.fallback.cache_len()
    }
}

impl VideoFrameDecoder for PreferredVideoFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.hap.retain_layers(layer_ids);
        self.fallback.retain_layers(layer_ids);
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        if HapMovFrameDecoder::supports_request(request) {
            self.hap.decode_frame(request)
        } else {
            self.fallback.decode_frame(request)
        }
    }
}

impl HapMovFrameDecoder {
    pub fn new() -> Self {
        Self {
            movies: Vec::new(),
            frames: Vec::new(),
        }
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
        path: &Path,
        signature: &StillImageSignature,
        request: &VideoFrameRequest,
    ) -> Result<usize, VideoDecodeError> {
        if let Some(index) = self
            .movies
            .iter()
            .position(|movie| movie.path == path && movie.signature == *signature)
        {
            return Ok(index);
        }

        self.movies.retain(|movie| movie.path != path);
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
        request: &VideoFrameRequest,
        path: &Path,
        signature: &StillImageSignature,
    ) -> Result<VideoFrame, VideoDecodeError> {
        let movie_index = self.movie_index(path, signature, request)?;
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
        if parsed.alpha.is_some() {
            return Err(decode_error(
                request,
                "HAP Q Alpha dual-plane frames are not supported by this decoder stage",
            ));
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
            TextureFormat::RgbaBc7 => {
                return Err(decode_error(
                    request,
                    "HAP R BC7 frames are not supported by this decoder stage",
                ));
            }
        };
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

        Ok(VideoFrame {
            layer_id: request.layer_id,
            width: movie.width,
            height: movie.height,
            pts_ms: sample.start_time.saturating_mul(1000) / u64::from(movie.timescale),
            duration_ms: (u64::from(sample.duration).saturating_mul(1000)
                / u64::from(movie.timescale))
            .max(1),
            format,
            data: parsed.data,
        })
    }
}

impl VideoFrameDecoder for HapMovFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        self.frames
            .retain(|entry| layer_ids.contains(&entry.layer_id));
        self.movies
            .retain(|movie| self.frames.iter().any(|entry| entry.path == movie.path));
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
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
            entry.layer_id == request.layer_id
                && entry.path == path
                && entry.position_ms == request.position_ms
                && entry.signature == signature
        }) {
            return Ok(Some(entry.frame.clone()));
        }
        let frame = self.decode_uncached(request, &path, &signature)?;
        self.frames
            .retain(|entry| entry.layer_id != request.layer_id);
        self.frames.push(HapFrameCacheEntry {
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

    fn write_hap_movie(frames: &[Vec<u8>], width: u16, height: u16) -> PathBuf {
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
                bytes[index..index + 4].copy_from_slice(b"Hap1");
                replacements += 1;
            }
        }
        assert_eq!(replacements, 1);
        let serial = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "rayard-hap-decoder-{}-{serial}-{}.mov",
            std::process::id(),
            NEXT_TEST_FILE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::write(&path, bytes).unwrap();
        path
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
    fn decodes_hap_q_as_explicit_ycocg_dxt5() {
        let path = write_hap_movie(&[hap_section(0xAF, &[0; 16])], 4, 4);
        let mut decoder = HapMovFrameDecoder::new();

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::YcoCgDxt5);
        assert_eq!(frame.data, vec![0; 16]);
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
            std::env::temp_dir().join(format!("rayard-missing-ffmpeg-{}", std::process::id()));
        let mut decoder =
            PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::new(missing_ffmpeg));

        let frame = decoder.decode_frame(&request(&path, 0)).unwrap().unwrap();

        assert_eq!(frame.format, VideoPixelFormat::Dxt1);
        assert_eq!(decoder.hap().movie_cache_len(), 1);
        assert_eq!(decoder.cache_len(), 1);
        fs::remove_file(path).unwrap();
    }
}
