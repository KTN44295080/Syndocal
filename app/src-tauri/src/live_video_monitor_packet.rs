use serde::Deserialize;
use std::time::Instant;

#[cfg(test)]
#[path = "live_video_monitor_bench_tests.rs"]
mod bench_tests;

pub(crate) const LIVE_VIDEO_MONITOR_MAGIC: [u8; 4] = *b"SYLV";
pub(crate) const LIVE_VIDEO_MONITOR_VERSION: u8 = 2;
pub(crate) const LIVE_VIDEO_MONITOR_STATUS_FRAME: u8 = 0;
pub(crate) const LIVE_VIDEO_MONITOR_STATUS_BUSY: u8 = 1;
pub(crate) const LIVE_VIDEO_MONITOR_HEADER_LEN: usize = 40;
pub(crate) const LIVE_VIDEO_MONITOR_MAX_WIDTH: u32 = 640;
pub(crate) const LIVE_VIDEO_MONITOR_MAX_HEIGHT: u32 = 360;
pub(crate) const LIVE_VIDEO_MONITOR_MAX_PIXELS: u32 =
    LIVE_VIDEO_MONITOR_MAX_WIDTH * LIVE_VIDEO_MONITOR_MAX_HEIGHT;
pub(crate) const LIVE_VIDEO_MONITOR_DEFAULT_JPEG_QUALITY: u8 = 68;
pub(crate) const LIVE_VIDEO_MONITOR_MIN_JPEG_QUALITY: u8 = 40;
pub(crate) const LIVE_VIDEO_MONITOR_MAX_JPEG_QUALITY: u8 = 90;

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LiveVideoMonitorKind {
    Program,
    Preview,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub(crate) enum LiveVideoMonitorPixelFormat {
    #[default]
    Jpeg,
    Rgba,
}

impl LiveVideoMonitorPixelFormat {
    pub(crate) fn packet_value(self) -> u8 {
        match self {
            Self::Jpeg => 0,
            Self::Rgba => 1,
        }
    }
}

impl LiveVideoMonitorKind {
    pub(crate) fn packet_value(self) -> u8 {
        match self {
            Self::Program => 0,
            Self::Preview => 1,
        }
    }
}

pub(crate) fn live_video_monitor_dimensions(width: u32, height: u32) -> (u32, u32) {
    let width = width.clamp(1, LIVE_VIDEO_MONITOR_MAX_WIDTH);
    let height = height.clamp(1, LIVE_VIDEO_MONITOR_MAX_HEIGHT);
    if width.saturating_mul(height) <= LIVE_VIDEO_MONITOR_MAX_PIXELS {
        return (width, height);
    }
    (width, (LIVE_VIDEO_MONITOR_MAX_PIXELS / width).max(1))
}

pub(crate) fn live_video_monitor_quality(quality: Option<u8>) -> u8 {
    quality
        .unwrap_or(LIVE_VIDEO_MONITOR_DEFAULT_JPEG_QUALITY)
        .clamp(
            LIVE_VIDEO_MONITOR_MIN_JPEG_QUALITY,
            LIVE_VIDEO_MONITOR_MAX_JPEG_QUALITY,
        )
}

pub(crate) fn live_video_monitor_elapsed_us(started: Instant) -> u32 {
    started.elapsed().as_micros().min(u128::from(u32::MAX)) as u32
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn live_video_monitor_packet(
    status: u8,
    kind: LiveVideoMonitorKind,
    sequence: u64,
    pts_ms: u64,
    render_us: u32,
    encode_us: u32,
    width: u32,
    height: u32,
    payload: &[u8],
    pixel_format: LiveVideoMonitorPixelFormat,
) -> Result<Vec<u8>, String> {
    if status > LIVE_VIDEO_MONITOR_STATUS_BUSY {
        return Err("Invalid live video monitor packet status".to_string());
    }
    if status == LIVE_VIDEO_MONITOR_STATUS_BUSY && !payload.is_empty() {
        return Err("Busy live video monitor packets must not contain image data".to_string());
    }
    if status == LIVE_VIDEO_MONITOR_STATUS_FRAME {
        if width == 0
            || height == 0
            || width > LIVE_VIDEO_MONITOR_MAX_WIDTH
            || height > LIVE_VIDEO_MONITOR_MAX_HEIGHT
        {
            return Err("Invalid live video monitor frame dimensions".to_string());
        }
        match pixel_format {
            LiveVideoMonitorPixelFormat::Rgba
                if payload.len() as u64 != u64::from(width) * u64::from(height) * 4 =>
            {
                return Err("Live video monitor RGBA payload length mismatch".to_string());
            }
            LiveVideoMonitorPixelFormat::Jpeg
                if payload.len() < 4 || !payload.starts_with(&[0xff, 0xd8]) =>
            {
                return Err("Live video monitor payload is not JPEG".to_string());
            }
            _ => {}
        }
    }
    let width = u16::try_from(width)
        .map_err(|_| "Live video monitor frame width exceeded the packet format".to_string())?;
    let height = u16::try_from(height)
        .map_err(|_| "Live video monitor frame height exceeded the packet format".to_string())?;
    let payload_len = u32::try_from(payload.len())
        .map_err(|_| "Live video monitor payload exceeded the packet format".to_string())?;
    let mut packet = Vec::with_capacity(LIVE_VIDEO_MONITOR_HEADER_LEN + payload.len());
    packet.extend_from_slice(&LIVE_VIDEO_MONITOR_MAGIC);
    packet.push(LIVE_VIDEO_MONITOR_VERSION);
    packet.push(status);
    packet.push(kind.packet_value());
    packet.push(pixel_format.packet_value());
    packet.extend_from_slice(&sequence.to_le_bytes());
    packet.extend_from_slice(&pts_ms.to_le_bytes());
    packet.extend_from_slice(&render_us.to_le_bytes());
    packet.extend_from_slice(&encode_us.to_le_bytes());
    packet.extend_from_slice(&width.to_le_bytes());
    packet.extend_from_slice(&height.to_le_bytes());
    packet.extend_from_slice(&payload_len.to_le_bytes());
    debug_assert_eq!(packet.len(), LIVE_VIDEO_MONITOR_HEADER_LEN);
    packet.extend_from_slice(payload);
    Ok(packet)
}

pub(crate) fn encode_live_video_monitor_jpeg(
    frame: &video::VideoFrame,
    quality: u8,
) -> Result<Vec<u8>, String> {
    if frame.format != video::VideoPixelFormat::Rgba8 {
        return Err(format!(
            "Live video monitor expected RGBA8, received {:?}",
            frame.format
        ));
    }
    let expected_len = u64::from(frame.width)
        .saturating_mul(u64::from(frame.height))
        .saturating_mul(4);
    if expected_len != frame.data.len() as u64 {
        return Err(format!(
            "Live video monitor RGBA8 buffer has {} bytes; expected {expected_len}",
            frame.data.len()
        ));
    }
    let mut jpeg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, quality)
        .encode(
            &frame.data,
            frame.width,
            frame.height,
            image::ColorType::Rgba8,
        )
        .map_err(|error| format!("Could not encode live video monitor JPEG: {error}"))?;
    Ok(jpeg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_video_monitor_raw_packet_is_lossless_and_rejects_invalid_images() {
        let pixels = [255, 0, 7, 255, 0, 120, 255, 0];
        let packet = live_video_monitor_packet(
            0,
            LiveVideoMonitorKind::Program,
            12,
            345,
            7,
            0,
            2,
            1,
            &pixels,
            LiveVideoMonitorPixelFormat::Rgba,
        )
        .unwrap();
        assert_eq!(packet[4], 2);
        assert_eq!(packet[7], 1);
        assert_eq!(&packet[40..], pixels);
        assert!(live_video_monitor_packet(
            0,
            LiveVideoMonitorKind::Program,
            12,
            345,
            7,
            0,
            2,
            1,
            &pixels[..7],
            LiveVideoMonitorPixelFormat::Rgba
        )
        .is_err());
        assert!(live_video_monitor_packet(
            1,
            LiveVideoMonitorKind::Program,
            12,
            345,
            7,
            0,
            2,
            1,
            &pixels,
            LiveVideoMonitorPixelFormat::Rgba
        )
        .is_err());
        assert!(live_video_monitor_packet(
            0,
            LiveVideoMonitorKind::Program,
            12,
            345,
            7,
            0,
            641,
            1,
            &pixels,
            LiveVideoMonitorPixelFormat::Rgba
        )
        .is_err());
        assert!(serde_json::from_str::<LiveVideoMonitorPixelFormat>("\"future\"").is_err());
    }
}
