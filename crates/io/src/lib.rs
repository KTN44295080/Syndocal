use protocol::ClockSource;

pub mod artnet;
pub mod midi;
#[cfg(feature = "ndi")]
pub mod ndi;
pub mod osc;
pub mod remote_ws;
pub mod sacn;
pub mod serial_dmx;

pub(crate) fn parse_clock_source_label(source: &str) -> Option<ClockSource> {
    match source.trim().to_ascii_lowercase().as_str() {
        "manual" => Some(ClockSource::Manual),
        "tap" => Some(ClockSource::Tap),
        "midi_clock" | "midiclock" | "midi-clock" => Some(ClockSource::MidiClock),
        "midi_timecode" | "miditimecode" | "mtc" | "midi-timecode" => {
            Some(ClockSource::MidiTimecode)
        }
        "ltc" => Some(ClockSource::Ltc),
        "ableton_link" | "abletonlink" | "ableton-link" | "link" => Some(ClockSource::AbletonLink),
        _ => None,
    }
}

pub(crate) fn parse_timecode_position_ms(value: &str) -> Option<u64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(position_ms) = trimmed.parse::<u64>() {
        return Some(position_ms);
    }

    let (time_part, frame_rate) = match trimmed.split_once('@') {
        Some((time, rate)) => {
            let rate = rate.trim().parse::<f64>().ok()?;
            if !rate.is_finite() || rate <= 0.0 {
                return None;
            }
            (time.trim(), rate)
        }
        None => (trimmed, 30.0),
    };
    let parts = time_part.split(':').collect::<Vec<_>>();
    if !(2..=4).contains(&parts.len()) || parts.iter().any(|part| part.trim().is_empty()) {
        return None;
    }

    let (hours, minutes, seconds, frame_ms) = match parts.len() {
        2 => {
            let minutes = parts[0].trim().parse::<u64>().ok()?;
            let seconds = parse_timecode_seconds_ms(parts[1].trim())?;
            (0, minutes, seconds.whole_seconds, seconds.millis)
        }
        3 => {
            let hours = parts[0].trim().parse::<u64>().ok()?;
            let minutes = parts[1].trim().parse::<u64>().ok()?;
            let seconds = parse_timecode_seconds_ms(parts[2].trim())?;
            (hours, minutes, seconds.whole_seconds, seconds.millis)
        }
        4 => {
            let hours = parts[0].trim().parse::<u64>().ok()?;
            let minutes = parts[1].trim().parse::<u64>().ok()?;
            let seconds = parts[2].trim().parse::<u64>().ok()?;
            let frames = parts[3].trim().parse::<u64>().ok()?;
            let max_frame = frame_rate.ceil() as u64;
            if max_frame == 0 || frames >= max_frame {
                return None;
            }
            let frame_ms = ((frames as f64 / frame_rate) * 1000.0).round() as u64;
            (hours, minutes, seconds, frame_ms)
        }
        _ => return None,
    };
    if minutes >= 60 || seconds >= 60 {
        return None;
    }
    Some(((hours * 60 + minutes) * 60 + seconds) * 1000 + frame_ms)
}

struct ParsedSeconds {
    whole_seconds: u64,
    millis: u64,
}

fn parse_timecode_seconds_ms(value: &str) -> Option<ParsedSeconds> {
    let seconds = value.parse::<f64>().ok()?;
    if !seconds.is_finite() || !(0.0..60.0).contains(&seconds) {
        return None;
    }
    let total_ms = (seconds * 1000.0).round() as u64;
    Some(ParsedSeconds {
        whole_seconds: total_ms / 1000,
        millis: total_ms % 1000,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_clock_source_aliases() {
        assert_eq!(
            parse_clock_source_label("link"),
            Some(ClockSource::AbletonLink)
        );
        assert_eq!(
            parse_clock_source_label("mtc"),
            Some(ClockSource::MidiTimecode)
        );
        assert_eq!(
            parse_clock_source_label("midi-clock"),
            Some(ClockSource::MidiClock)
        );
        assert_eq!(parse_clock_source_label("ltc"), Some(ClockSource::Ltc));
        assert_eq!(parse_clock_source_label("unknown"), None);
    }

    #[test]
    fn parses_timecode_position_strings() {
        assert_eq!(parse_timecode_position_ms("12345"), Some(12_345));
        assert_eq!(parse_timecode_position_ms("01:02:03.500"), Some(3_723_500));
        assert_eq!(parse_timecode_position_ms("02:03.250"), Some(123_250));
        assert_eq!(
            parse_timecode_position_ms("01:02:03:15@30"),
            Some(3_723_500)
        );
        assert_eq!(
            parse_timecode_position_ms("01:02:03:12@24"),
            Some(3_723_500)
        );
        assert_eq!(parse_timecode_position_ms("01:60:00"), None);
        assert_eq!(parse_timecode_position_ms("00:00:00:30@30"), None);
    }
}
