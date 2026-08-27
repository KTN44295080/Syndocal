//! Pure timing and loop-plan helpers for the runtime-only Follow destination
//! hold.  This module intentionally has no engine mutation or persistence
//! access: a caller captures the plan at Follow admission, then applies it
//! only after the same generation has settled successfully.

use super::TimelineTempoMeterAuthority;
use protocol::{
    TimelineSnapshot, VideoClipTakeDuration, VideoClipTakeDurationUnit, VideoClipTakeKind,
};

/// Runtime-only image of the destination's first complete measure.  The
/// authored Timeline is never modified to create this range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TimelineFollowHoldPlan {
    pub(crate) end_ms: u64,
    pub(crate) musical_length_millibeats: u64,
}

/// Resolve a Follow duration at its admission point.  Unlike generic Clip
/// Takes, `Bars` means authored measures under the source Timeline's current
/// tempo/meter authority, not a hard-coded four quarter-note approximation.
pub(crate) fn resolve_follow_duration_ms(
    kind: VideoClipTakeKind,
    duration: VideoClipTakeDuration,
    authority: &TimelineTempoMeterAuthority,
    admission_position_ms: u64,
) -> Result<u64, String> {
    if !matches!(duration.unit, VideoClipTakeDurationUnit::Bars) {
        return super::resolve_video_clip_take_duration_ms(
            kind,
            duration,
            &protocol::ClockSnapshot {
                bpm: authority.fallback_bpm() as f32,
                ..protocol::ClockSnapshot::default()
            },
        );
    }
    if matches!(kind, VideoClipTakeKind::Cut) {
        if duration.value_milliunits == 0 {
            return Ok(0);
        }
        return Err("Video clip Cut duration must be zero".to_string());
    }
    if duration.value_milliunits == 0 {
        return Err("Video clip transition duration must be greater than zero".to_string());
    }
    let start_quarter =
        authority.quarter_beat_at_seconds(admission_position_ms as f64 / 1_000.0)?;
    let measure_quarters = measure_quarters_at(authority, start_quarter)?;
    let span_quarters = measure_quarters * duration.value_milliunits as f64 / 1_000.0;
    let end_quarter = start_quarter + span_quarters;
    let elapsed_seconds = authority.seconds_at_quarter_beat(end_quarter)?
        - authority.seconds_at_quarter_beat(start_quarter)?;
    millis_from_seconds(elapsed_seconds, "Timeline Follow Bars duration")
}

/// Resolve exactly one source Timeline measure at Follow admission.  This is
/// deliberately distinct from authored Follow duration: an enabled
/// destination hold clean-breaks arbitrary milliseconds, beats, and Bars
/// values so the source settlement is always one admitted measure.
pub(crate) fn source_admission_measure_duration_ms(
    authority: &TimelineTempoMeterAuthority,
    admission_position_ms: u64,
) -> Result<u64, String> {
    let start_quarter =
        authority.quarter_beat_at_seconds(admission_position_ms as f64 / 1_000.0)?;
    let end_quarter = start_quarter + measure_quarters_at(authority, start_quarter)?;
    let elapsed_seconds = authority.seconds_at_quarter_beat(end_quarter)?
        - authority.seconds_at_quarter_beat(start_quarter)?;
    millis_from_seconds(elapsed_seconds, "Timeline Follow source admission measure")
}

/// Capture the exact destination-first-measure span using the destination's
/// own `TimelineTempoMeterAuthority`.  A partial authored Timeline is
/// rejected rather than silently clamped into a shorter loop.
pub(crate) fn destination_first_measure_hold_plan(
    target: &TimelineSnapshot,
    fallback_bpm: f32,
) -> Result<TimelineFollowHoldPlan, String> {
    let authority = TimelineTempoMeterAuthority::from_timeline(target, f64::from(fallback_bpm))?;
    let measure_quarters = measure_quarters_at(&authority, 0.0)?;
    let end_ms = millis_from_seconds(
        authority.seconds_at_quarter_beat(measure_quarters)?,
        "Timeline Follow destination first measure",
    )?;
    if end_ms > target.duration_ms {
        return Err(
            "Timeline Follow destination first measure exceeds the authored duration".to_string(),
        );
    }
    let musical_length_millibeats = (measure_quarters * 1_000.0).round();
    if !musical_length_millibeats.is_finite()
        || !(1.0..=u64::MAX as f64).contains(&musical_length_millibeats)
    {
        return Err("Timeline Follow destination first measure is invalid".to_string());
    }
    Ok(TimelineFollowHoldPlan {
        end_ms,
        musical_length_millibeats: musical_length_millibeats as u64,
    })
}

fn measure_quarters_at(
    authority: &TimelineTempoMeterAuthority,
    quarter_beat: f64,
) -> Result<f64, String> {
    let (numerator, denominator) = authority.meter_at_quarter_beat(quarter_beat)?;
    Ok(f64::from(numerator) * 4.0 / f64::from(denominator))
}

fn millis_from_seconds(seconds: f64, label: &str) -> Result<u64, String> {
    let milliseconds = (seconds * 1_000.0).round();
    if !milliseconds.is_finite() || !(1.0..=u64::MAX as f64).contains(&milliseconds) {
        return Err(format!("{label} resolves to an invalid duration"));
    }
    Ok(milliseconds as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{TimelineTempoInterpolation, TimelineTempoMeterPoint};

    fn point(numerator: u8, denominator: u8) -> TimelineTempoMeterPoint {
        TimelineTempoMeterPoint {
            position_sixteenth_steps: 0,
            bpm: 120.0,
            numerator,
            denominator,
            interpolation: TimelineTempoInterpolation::Step,
            ..TimelineTempoMeterPoint::default()
        }
    }

    #[test]
    fn first_destination_measure_is_meter_aware_and_never_four_beat_assumed() {
        let target = TimelineSnapshot {
            duration_ms: 2_500,
            tempo_meter_map: vec![point(5, 4)],
            ..TimelineSnapshot::default()
        };
        assert_eq!(
            destination_first_measure_hold_plan(&target, 120.0).unwrap(),
            TimelineFollowHoldPlan {
                end_ms: 2_500,
                musical_length_millibeats: 5_000,
            }
        );
    }

    #[test]
    fn bars_follow_duration_captures_the_source_meter_at_admission() {
        let authority = TimelineTempoMeterAuthority::new(120.0, vec![point(5, 4)]).unwrap();
        assert_eq!(
            resolve_follow_duration_ms(
                VideoClipTakeKind::Crossfade,
                VideoClipTakeDuration {
                    unit: VideoClipTakeDurationUnit::Bars,
                    value_milliunits: 1_000,
                },
                &authority,
                0,
            )
            .unwrap(),
            2_500
        );
    }

    #[test]
    fn source_admission_measure_ignores_any_authored_follow_duration() {
        let authority = TimelineTempoMeterAuthority::new(120.0, vec![point(5, 4)]).unwrap();
        assert_eq!(
            source_admission_measure_duration_ms(&authority, 0).unwrap(),
            2_500
        );
    }
}
