use std::{fs, path::Path};

use protocol::{AudioAnalysisSummary, AudioWaveformPoint};
use thiserror::Error;

const DEFAULT_WAVEFORM_POINTS: usize = 512;
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
    let waveform = build_waveform(&samples, format.sample_rate, max_points.max(1));
    let beats = detect_beats(&samples, format.sample_rate);
    let estimated_bpm = estimate_bpm(&beats);

    Ok(AudioAnalysisSummary {
        path: path.to_string_lossy().to_string(),
        sample_rate: format.sample_rate,
        channels: format.channels,
        duration_ms,
        estimated_bpm,
        waveform,
        beats,
    })
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
}
