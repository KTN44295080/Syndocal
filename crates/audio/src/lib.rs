use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use protocol::{AudioAnalysisSummary, AudioWaveformPoint};
use thiserror::Error;

const DEFAULT_WAVEFORM_POINTS: usize = 512;
const DEFAULT_FFMPEG_SAMPLE_RATE: u32 = 44_100;
const ONSET_WINDOW_MS: u64 = 50;
const MIN_BPM: f32 = 60.0;
const MAX_BPM: f32 = 200.0;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("failed to read audio file: {0}")]
    Read(#[from] std::io::Error),
    #[error("audio file is not a RIFF/WAVE file")]
    NotWave,
    #[error("audio file is missing a fmt chunk")]
    MissingFormat,
    #[error("audio file is missing a data chunk")]
    MissingData,
    #[error("unsupported WAV format {0}")]
    UnsupportedFormat(u16),
    #[error("unsupported WAV bit depth {0}")]
    UnsupportedBitDepth(u16),
    #[error("failed to run ffmpeg: {0}")]
    FfmpegIo(String),
    #[error("ffmpeg failed to decode audio: {0}")]
    FfmpegDecode(String),
    #[error("ffmpeg returned malformed f32 PCM audio")]
    InvalidDecodedAudio,
    #[error("invalid WAV file: {0}")]
    Invalid(&'static str),
}

#[derive(Debug, Clone, Copy)]
struct WavFormat {
    audio_format: u16,
    channels: u16,
    sample_rate: u32,
    block_align: u16,
    bits_per_sample: u16,
}

pub fn analyze_wav_file(path: impl AsRef<Path>) -> Result<AudioAnalysisSummary, AudioError> {
    analyze_wav_file_with_points(path, DEFAULT_WAVEFORM_POINTS)
}

pub fn analyze_audio_file(path: impl AsRef<Path>) -> Result<AudioAnalysisSummary, AudioError> {
    analyze_audio_file_with_points(path, DEFAULT_WAVEFORM_POINTS)
}

pub fn analyze_audio_file_with_points(
    path: impl AsRef<Path>,
    max_points: usize,
) -> Result<AudioAnalysisSummary, AudioError> {
    let path = path.as_ref();
    if path.extension().is_some() && !is_wav_path(path) {
        return analyze_audio_file_with_ffmpeg_binary(path, max_points, ffmpeg_binary_from_env());
    }
    match analyze_wav_file_with_points(path, max_points) {
        Ok(analysis) => Ok(analysis),
        Err(wav_error) if should_try_ffmpeg_after_wav_error(path, &wav_error) => {
            let ffmpeg_result =
                analyze_audio_file_with_ffmpeg_binary(path, max_points, ffmpeg_binary_from_env());
            if is_wav_path(path) {
                ffmpeg_result.or(Err(wav_error))
            } else {
                ffmpeg_result
            }
        }
        Err(wav_error) => Err(wav_error),
    }
}

pub fn analyze_wav_file_with_points(
    path: impl AsRef<Path>,
    max_points: usize,
) -> Result<AudioAnalysisSummary, AudioError> {
    let path = path.as_ref();
    let bytes = fs::read(path)?;
    let (format, data) = parse_wav(&bytes)?;
    validate_format(format)?;
    let samples = decode_mono_samples(data, format)?;
    let frame_count = samples.len();
    let duration_ms = frames_to_ms(frame_count, format.sample_rate);
    Ok(analyze_mono_samples(
        path,
        format.sample_rate,
        format.channels,
        duration_ms,
        samples,
        max_points,
    ))
}

pub fn analyze_audio_file_with_ffmpeg_binary(
    path: impl AsRef<Path>,
    max_points: usize,
    binary: impl AsRef<Path>,
) -> Result<AudioAnalysisSummary, AudioError> {
    let path = path.as_ref();
    let sample_rate = DEFAULT_FFMPEG_SAMPLE_RATE;
    let output = Command::new(binary.as_ref())
        .args(["-hide_banner", "-loglevel", "error", "-i"])
        .arg(path)
        .args([
            "-f",
            "f32le",
            "-ac",
            "1",
            "-ar",
            &sample_rate.to_string(),
            "pipe:1",
        ])
        .output()
        .map_err(|error| AudioError::FfmpegIo(error.to_string()))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(AudioError::FfmpegDecode(if stderr.is_empty() {
            format!("process exited with {}", output.status)
        } else {
            stderr
        }));
    }
    let samples = decode_f32le_mono_samples(&output.stdout)?;
    let duration_ms = frames_to_ms(samples.len(), sample_rate);
    Ok(analyze_mono_samples(
        path,
        sample_rate,
        1,
        duration_ms,
        samples,
        max_points,
    ))
}

fn analyze_mono_samples(
    path: &Path,
    sample_rate: u32,
    channels: u16,
    duration_ms: u64,
    samples: Vec<f32>,
    max_points: usize,
) -> AudioAnalysisSummary {
    let waveform = build_waveform(&samples, sample_rate, max_points.max(1));
    let beats = detect_beats(&samples, sample_rate);
    let estimated_bpm = estimate_bpm(&beats);

    AudioAnalysisSummary {
        path: path.to_string_lossy().to_string(),
        sample_rate,
        channels,
        duration_ms,
        estimated_bpm,
        waveform,
        beats,
    }
}

fn decode_f32le_mono_samples(bytes: &[u8]) -> Result<Vec<f32>, AudioError> {
    if bytes.len() % 4 != 0 {
        return Err(AudioError::InvalidDecodedAudio);
    }
    Ok(bytes
        .chunks_exact(4)
        .map(|chunk| {
            let value = f32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
            if value.is_finite() {
                value.clamp(-1.0, 1.0)
            } else {
                0.0
            }
        })
        .collect())
}

fn should_try_ffmpeg_after_wav_error(path: &Path, error: &AudioError) -> bool {
    match error {
        AudioError::Read(_) | AudioError::MissingFormat | AudioError::MissingData => false,
        AudioError::Invalid(_) if is_wav_path(path) => false,
        AudioError::NotWave
        | AudioError::UnsupportedFormat(_)
        | AudioError::UnsupportedBitDepth(_)
        | AudioError::Invalid(_) => true,
        AudioError::FfmpegIo(_) | AudioError::FfmpegDecode(_) | AudioError::InvalidDecodedAudio => {
            false
        }
    }
}

fn is_wav_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            extension.eq_ignore_ascii_case("wav") || extension.eq_ignore_ascii_case("wave")
        })
        .unwrap_or(false)
}

fn ffmpeg_binary_from_env() -> PathBuf {
    std::env::var_os("RAYARD_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| "ffmpeg".into())
}

fn parse_wav(bytes: &[u8]) -> Result<(WavFormat, &[u8]), AudioError> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err(AudioError::NotWave);
    }

    let mut offset = 12usize;
    let mut format = None;
    let mut data = None;
    while offset.saturating_add(8) <= bytes.len() {
        let id = &bytes[offset..offset + 4];
        let size = read_u32(bytes, offset + 4)? as usize;
        offset += 8;
        if offset.saturating_add(size) > bytes.len() {
            return Err(AudioError::Invalid("chunk extends past end of file"));
        }
        let chunk = &bytes[offset..offset + size];
        match id {
            b"fmt " => format = Some(parse_format_chunk(chunk)?),
            b"data" => data = Some(chunk),
            _ => {}
        }
        offset += size + (size % 2);
    }

    Ok((
        format.ok_or(AudioError::MissingFormat)?,
        data.ok_or(AudioError::MissingData)?,
    ))
}

fn parse_format_chunk(chunk: &[u8]) -> Result<WavFormat, AudioError> {
    if chunk.len() < 16 {
        return Err(AudioError::Invalid("fmt chunk is too short"));
    }
    Ok(WavFormat {
        audio_format: read_u16(chunk, 0)?,
        channels: read_u16(chunk, 2)?,
        sample_rate: read_u32(chunk, 4)?,
        block_align: read_u16(chunk, 12)?,
        bits_per_sample: read_u16(chunk, 14)?,
    })
}

fn validate_format(format: WavFormat) -> Result<(), AudioError> {
    if !matches!(format.audio_format, 1 | 3) {
        return Err(AudioError::UnsupportedFormat(format.audio_format));
    }
    if format.channels == 0 {
        return Err(AudioError::Invalid("channel count is zero"));
    }
    if format.sample_rate == 0 {
        return Err(AudioError::Invalid("sample rate is zero"));
    }
    let bytes_per_sample = bytes_per_sample(format.bits_per_sample)?;
    let expected_align = format.channels as usize * bytes_per_sample;
    if format.block_align as usize != expected_align {
        return Err(AudioError::Invalid("block alignment does not match format"));
    }
    if format.audio_format == 3 && format.bits_per_sample != 32 {
        return Err(AudioError::UnsupportedBitDepth(format.bits_per_sample));
    }
    Ok(())
}

fn decode_mono_samples(data: &[u8], format: WavFormat) -> Result<Vec<f32>, AudioError> {
    let frame_size = format.block_align as usize;
    if frame_size == 0 || data.len() < frame_size {
        return Ok(Vec::new());
    }
    let bytes_per_sample = bytes_per_sample(format.bits_per_sample)?;
    let frames = data.len() / frame_size;
    let mut samples = Vec::with_capacity(frames);
    for frame_index in 0..frames {
        let frame_offset = frame_index * frame_size;
        let mut mixed = 0.0;
        for channel in 0..format.channels as usize {
            let sample_offset = frame_offset + channel * bytes_per_sample;
            mixed += decode_sample(
                &data[sample_offset..sample_offset + bytes_per_sample],
                format.audio_format,
                format.bits_per_sample,
            )?;
        }
        samples.push((mixed / format.channels as f32).clamp(-1.0, 1.0));
    }
    Ok(samples)
}

fn decode_sample(bytes: &[u8], audio_format: u16, bits_per_sample: u16) -> Result<f32, AudioError> {
    match (audio_format, bits_per_sample) {
        (1, 8) => Ok((bytes[0] as f32 - 128.0) / 128.0),
        (1, 16) => Ok(i16::from_le_bytes([bytes[0], bytes[1]]) as f32 / i16::MAX as f32),
        (1, 24) => {
            let value =
                ((bytes[0] as i32) << 8) | ((bytes[1] as i32) << 16) | ((bytes[2] as i32) << 24);
            Ok((value >> 8) as f32 / 8_388_607.0)
        }
        (1, 32) => Ok(
            i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as f32 / i32::MAX as f32,
        ),
        (3, 32) => {
            Ok(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]).clamp(-1.0, 1.0))
        }
        (_, _) => Err(AudioError::UnsupportedBitDepth(bits_per_sample)),
    }
}

fn build_waveform(samples: &[f32], sample_rate: u32, max_points: usize) -> Vec<AudioWaveformPoint> {
    if samples.is_empty() {
        return Vec::new();
    }
    let bucket_size = samples.len().div_ceil(max_points).max(1);
    samples
        .chunks(bucket_size)
        .enumerate()
        .map(|(index, chunk)| {
            let mut peak = 0.0_f32;
            let mut sum_square = 0.0_f32;
            for sample in chunk {
                let abs = sample.abs();
                peak = peak.max(abs);
                sum_square += sample * sample;
            }
            AudioWaveformPoint {
                time_ms: frames_to_ms(index * bucket_size, sample_rate),
                peak: peak.clamp(0.0, 1.0),
                rms: (sum_square / chunk.len() as f32).sqrt().clamp(0.0, 1.0),
            }
        })
        .collect()
}

fn detect_beats(samples: &[f32], sample_rate: u32) -> Vec<u64> {
    if samples.is_empty() || sample_rate == 0 {
        return Vec::new();
    }
    let window_size = ((u64::from(sample_rate) * ONSET_WINDOW_MS) / 1_000)
        .try_into()
        .unwrap_or(usize::MAX)
        .max(1);
    let energies = samples
        .chunks(window_size)
        .map(|chunk| chunk.iter().map(|sample| sample * sample).sum::<f32>() / chunk.len() as f32)
        .collect::<Vec<_>>();
    if energies.len() < 4 {
        return Vec::new();
    }
    let mean = energies.iter().sum::<f32>() / energies.len() as f32;
    let variance = energies
        .iter()
        .map(|energy| {
            let delta = energy - mean;
            delta * delta
        })
        .sum::<f32>()
        / energies.len() as f32;
    let threshold = (mean + variance.sqrt()).max(mean * 1.8);
    let min_interval_ms = (60_000.0 / MAX_BPM) as u64;
    let mut beats = Vec::new();
    let mut last_beat_ms = None;
    for index in 1..energies.len().saturating_sub(1) {
        let energy = energies[index];
        if energy <= threshold || energy < energies[index - 1] || energy < energies[index + 1] {
            continue;
        }
        let time_ms = frames_to_ms(index * window_size, sample_rate);
        if last_beat_ms
            .map(|last| time_ms.saturating_sub(last) >= min_interval_ms)
            .unwrap_or(true)
        {
            beats.push(time_ms);
            last_beat_ms = Some(time_ms);
        }
    }
    beats
}

fn estimate_bpm(beats: &[u64]) -> Option<f32> {
    if beats.len() < 2 {
        return None;
    }
    let mut intervals = beats
        .windows(2)
        .filter_map(|pair| pair[1].checked_sub(pair[0]))
        .filter(|interval| *interval > 0)
        .collect::<Vec<_>>();
    if intervals.is_empty() {
        return None;
    }
    intervals.sort_unstable();
    let median = intervals[intervals.len() / 2] as f32;
    let mut bpm = 60_000.0 / median;
    while bpm < MIN_BPM {
        bpm *= 2.0;
    }
    while bpm > MAX_BPM {
        bpm *= 0.5;
    }
    bpm.is_finite().then_some(bpm)
}

fn bytes_per_sample(bits_per_sample: u16) -> Result<usize, AudioError> {
    match bits_per_sample {
        8 | 16 | 24 | 32 => Ok(bits_per_sample as usize / 8),
        _ => Err(AudioError::UnsupportedBitDepth(bits_per_sample)),
    }
}

fn frames_to_ms(frames: usize, sample_rate: u32) -> u64 {
    if sample_rate == 0 {
        return 0;
    }
    ((frames as u128 * 1_000) / u128::from(sample_rate)) as u64
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, AudioError> {
    let slice = bytes
        .get(offset..offset + 2)
        .ok_or(AudioError::Invalid("unexpected end of file"))?;
    Ok(u16::from_le_bytes([slice[0], slice[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, AudioError> {
    let slice = bytes
        .get(offset..offset + 4)
        .ok_or(AudioError::Invalid("unexpected end of file"))?;
    Ok(u32::from_le_bytes([slice[0], slice[1], slice[2], slice[3]]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analyzes_pcm16_wav_waveform_and_bpm() {
        let wav = test_wav_with_clicks(120.0, 4_000, 4);
        let (format, data) = parse_wav(&wav).unwrap();
        let samples = decode_mono_samples(data, format).unwrap();
        let waveform = build_waveform(&samples, format.sample_rate, 32);
        let beats = detect_beats(&samples, format.sample_rate);
        let bpm = estimate_bpm(&beats).unwrap();

        assert_eq!(format.channels, 1);
        assert!(!waveform.is_empty());
        assert!(waveform.iter().any(|point| point.peak > 0.9));
        assert!(beats.len() >= 3);
        assert!((bpm - 120.0).abs() < 0.1);
    }

    #[test]
    fn rejects_non_wave_data() {
        assert!(matches!(parse_wav(b"nope"), Err(AudioError::NotWave)));
    }

    #[test]
    fn decodes_ffmpeg_f32le_output_for_audio_analysis() {
        let binary = fake_ffmpeg_binary();
        let analysis =
            analyze_audio_file_with_ffmpeg_binary("ignored-by-fake-ffmpeg.mp3", 8, &binary)
                .unwrap();
        let _ = std::fs::remove_file(&binary);

        assert_eq!(analysis.sample_rate, DEFAULT_FFMPEG_SAMPLE_RATE);
        assert_eq!(analysis.channels, 1);
        assert_eq!(analysis.waveform.len(), 2);
        assert!(analysis.waveform.iter().all(|point| point.peak <= 1.0));
    }

    #[test]
    fn rejects_misaligned_ffmpeg_pcm_output() {
        assert!(matches!(
            decode_f32le_mono_samples(b"abc"),
            Err(AudioError::InvalidDecodedAudio)
        ));
    }

    fn test_wav_with_clicks(bpm: f32, duration_ms: u64, beats: usize) -> Vec<u8> {
        let sample_rate = 8_000u32;
        let frames = (u64::from(sample_rate) * duration_ms / 1_000) as usize;
        let beat_interval = (60_000.0 / bpm) as usize;
        let mut samples = vec![0i16; frames];
        for beat in 0..beats {
            let start = beat * beat_interval * sample_rate as usize / 1_000;
            for offset in 0..80 {
                if let Some(sample) = samples.get_mut(start + offset) {
                    *sample = i16::MAX;
                }
            }
        }
        let data = samples
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>();
        build_wav(sample_rate, 1, 16, &data)
    }

    fn build_wav(sample_rate: u32, channels: u16, bits: u16, data: &[u8]) -> Vec<u8> {
        let byte_rate = sample_rate * u32::from(channels) * u32::from(bits) / 8;
        let block_align = channels * bits / 8;
        let riff_size = 36 + data.len() as u32;
        let mut wav = Vec::new();
        wav.extend_from_slice(b"RIFF");
        wav.extend_from_slice(&riff_size.to_le_bytes());
        wav.extend_from_slice(b"WAVE");
        wav.extend_from_slice(b"fmt ");
        wav.extend_from_slice(&16u32.to_le_bytes());
        wav.extend_from_slice(&1u16.to_le_bytes());
        wav.extend_from_slice(&channels.to_le_bytes());
        wav.extend_from_slice(&sample_rate.to_le_bytes());
        wav.extend_from_slice(&byte_rate.to_le_bytes());
        wav.extend_from_slice(&block_align.to_le_bytes());
        wav.extend_from_slice(&bits.to_le_bytes());
        wav.extend_from_slice(b"data");
        wav.extend_from_slice(&(data.len() as u32).to_le_bytes());
        wav.extend_from_slice(data);
        wav
    }

    fn fake_ffmpeg_binary() -> PathBuf {
        let extension = if cfg!(windows) { "cmd" } else { "sh" };
        let path = std::env::temp_dir().join(format!(
            "rayard-fake-audio-ffmpeg-{}.{extension}",
            std::process::id()
        ));
        #[cfg(windows)]
        std::fs::write(
            &path,
            b"@echo off\r\n<nul set /p dummy=ABCDABCD\r\nexit /b 0\r\n",
        )
        .unwrap();
        #[cfg(not(windows))]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&path, b"#!/bin/sh\nprintf ABCDABCD\n").unwrap();
            let mut permissions = std::fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o755);
            std::fs::set_permissions(&path, permissions).unwrap();
        }
        path
    }
}
