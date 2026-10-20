//! Fixed-size native broker payload for ISF audio; never a project DTO.
use protocol::{LiveAudioTextureFrame, LIVE_AUDIO_TEXTURE_SAMPLES};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

pub(super) const PAYLOAD_BYTES: usize = LIVE_AUDIO_TEXTURE_SAMPLES * 2 * 4;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Header {
    generation: u64,
    feature_sequence: u64,
    sample_rate: u32,
}

pub(super) fn header(frame: Option<&LiveAudioTextureFrame>) -> Result<Option<Header>, String> {
    frame
        .map(|frame| {
            if !frame.is_valid() {
                return Err("Recording audio texture frame is invalid".to_string());
            }
            Ok(Header {
                generation: frame.generation,
                feature_sequence: frame.feature_sequence,
                sample_rate: frame.sample_rate,
            })
        })
        .transpose()
}

pub(super) fn append(frame: Option<&LiveAudioTextureFrame>, payload: &mut Vec<u8>) {
    if let Some(frame) = frame {
        for value in frame.waveform.iter().chain(frame.fft.iter()) {
            payload.extend_from_slice(&value.to_le_bytes());
        }
    }
}

pub(super) fn decode(
    header: Option<Header>,
    payload: &[u8],
    offset: &mut usize,
) -> Result<Option<Arc<LiveAudioTextureFrame>>, String> {
    let Some(header) = header else {
        return Ok(None);
    };
    let end = offset
        .checked_add(PAYLOAD_BYTES)
        .ok_or_else(|| "Recording audio texture length overflow".to_string())?;
    let bytes = payload
        .get(*offset..end)
        .ok_or_else(|| "Recording audio texture payload is truncated".to_string())?;
    let mut frame = LiveAudioTextureFrame {
        generation: header.generation,
        feature_sequence: header.feature_sequence,
        sample_rate: header.sample_rate,
        waveform: [0.0; LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [0.0; LIVE_AUDIO_TEXTURE_SAMPLES],
    };
    for (value, bytes) in frame
        .waveform
        .iter_mut()
        .chain(frame.fft.iter_mut())
        .zip(bytes.chunks_exact(4))
    {
        *value = f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
    }
    if !frame.is_valid() {
        return Err("Recording audio texture frame is invalid".to_string());
    }
    *offset = end;
    Ok(Some(Arc::new(frame)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recording_audio_binary_roundtrip_is_fixed_size_and_rejects_invalid_payloads() {
        let frame = LiveAudioTextureFrame {
            generation: 3,
            feature_sequence: 7,
            sample_rate: 48_000,
            waveform: std::array::from_fn(|index| index as f32 / 511.0 * 2.0 - 1.0),
            fft: [0.25; LIVE_AUDIO_TEXTURE_SAMPLES],
        };
        let mut bytes = vec![1, 2, 3];
        append(Some(&frame), &mut bytes);
        assert_eq!(bytes.len(), 3 + PAYLOAD_BYTES);
        let mut offset = 3;
        let decoded = decode(header(Some(&frame)).unwrap(), &bytes, &mut offset)
            .unwrap()
            .unwrap();
        assert_eq!(decoded.as_ref(), &frame);
        assert_eq!(offset, bytes.len());
        assert!(decode(
            header(Some(&frame)).unwrap(),
            &bytes[..bytes.len() - 1],
            &mut 3
        )
        .is_err());
        bytes[3..7].copy_from_slice(&f32::NAN.to_le_bytes());
        assert!(decode(header(Some(&frame)).unwrap(), &bytes, &mut 3).is_err());
        assert!(decode(None, &[], &mut 0).unwrap().is_none());
    }
}
