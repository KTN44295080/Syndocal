use serde::{Deserialize, Serialize};

/// Version of the authored Timeline audio policy payload.  The project file
/// itself remains at version 1; this is an independently versioned additive
/// field so old projects can be read without rewriting unrelated data.
pub const TIMELINE_AUDIO_POLICY_VERSION: u8 = 1;
pub const TIMELINE_AUDIO_MIN_SLEW_PPM: u32 = 1;
pub const TIMELINE_AUDIO_MAX_SLEW_PPM: u32 = 100_000;
pub const TIMELINE_AUDIO_MIN_DRIFT_MS: u64 = 1;
pub const TIMELINE_AUDIO_MAX_DRIFT_MS: u64 = 10_000;
pub const TIMELINE_AUDIO_MAX_RESYNC_COOLDOWN_MS: u64 = 60_000;

/// ShowClock is the only supported authored master.  Audio PTS follows the
/// published Timeline position; a local device clock never arbitrates show
/// time.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioClockMaster {
    #[default]
    ShowClock,
}

/// Device-rate correction is bounded and explicit.  The backend may apply a
/// device-specific correction within this limit; it must not silently choose a
/// different format or an unbounded rate.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioResamplingPolicy {
    #[default]
    BoundedDeviceSlew,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioSeekPolicy {
    #[default]
    ReanchorToShowClock,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioLoopPolicy {
    #[default]
    WrapToAuthoredRange,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioUnderrunPolicy {
    #[default]
    RetireAndHold,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TimelineAudioDeviceFaultPolicy {
    #[default]
    RetireAndHold,
}

/// Authored, portable policy for every Timeline audio lane.  Physical device
/// identity is intentionally absent; that belongs to the machine-local output
/// router and cannot be restored from a project file.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimelineAudioPolicy {
    #[serde(default = "default_policy_version")]
    pub version: u8,
    #[serde(default)]
    pub clock_master: TimelineAudioClockMaster,
    #[serde(default)]
    pub resampling: TimelineAudioResamplingPolicy,
    #[serde(default)]
    pub seek: TimelineAudioSeekPolicy,
    #[serde(default)]
    pub loop_policy: TimelineAudioLoopPolicy,
    #[serde(default)]
    pub underrun: TimelineAudioUnderrunPolicy,
    #[serde(default)]
    pub device_fault: TimelineAudioDeviceFaultPolicy,
    #[serde(default = "default_max_slew_ppm")]
    pub max_slew_ppm: u32,
    #[serde(default = "default_max_drift_ms")]
    pub max_drift_ms: u64,
    #[serde(default = "default_resync_cooldown_ms")]
    pub resync_cooldown_ms: u64,
}

impl Default for TimelineAudioPolicy {
    fn default() -> Self {
        Self {
            version: default_policy_version(),
            clock_master: TimelineAudioClockMaster::default(),
            resampling: TimelineAudioResamplingPolicy::default(),
            seek: TimelineAudioSeekPolicy::default(),
            loop_policy: TimelineAudioLoopPolicy::default(),
            underrun: TimelineAudioUnderrunPolicy::default(),
            device_fault: TimelineAudioDeviceFaultPolicy::default(),
            max_slew_ppm: default_max_slew_ppm(),
            max_drift_ms: default_max_drift_ms(),
            resync_cooldown_ms: 250,
        }
    }
}

const fn default_policy_version() -> u8 {
    TIMELINE_AUDIO_POLICY_VERSION
}

const fn default_max_slew_ppm() -> u32 {
    2_500
}

const fn default_max_drift_ms() -> u64 {
    75
}

const fn default_resync_cooldown_ms() -> u64 {
    250
}

/// Validate the independent audio policy before it can enter runtime state.
/// Missing legacy JSON is materialized as the current policy by serde; an
/// explicit future version or out-of-range correction is rejected.
pub fn validate_timeline_audio_policy(policy: &TimelineAudioPolicy) -> Result<(), String> {
    if policy.version != TIMELINE_AUDIO_POLICY_VERSION {
        return Err(format!(
            "Timeline audio policy version {} is unsupported; expected {}",
            policy.version, TIMELINE_AUDIO_POLICY_VERSION
        ));
    }
    if !(TIMELINE_AUDIO_MIN_SLEW_PPM..=TIMELINE_AUDIO_MAX_SLEW_PPM).contains(&policy.max_slew_ppm) {
        return Err(format!(
            "Timeline audio max slew must be within {TIMELINE_AUDIO_MIN_SLEW_PPM}..={TIMELINE_AUDIO_MAX_SLEW_PPM} ppm"
        ));
    }
    if !(TIMELINE_AUDIO_MIN_DRIFT_MS..=TIMELINE_AUDIO_MAX_DRIFT_MS).contains(&policy.max_drift_ms) {
        return Err(format!(
            "Timeline audio max drift must be within {TIMELINE_AUDIO_MIN_DRIFT_MS}..={TIMELINE_AUDIO_MAX_DRIFT_MS} ms"
        ));
    }
    if policy.resync_cooldown_ms > TIMELINE_AUDIO_MAX_RESYNC_COOLDOWN_MS {
        return Err(format!(
            "Timeline audio resync cooldown exceeds {TIMELINE_AUDIO_MAX_RESYNC_COOLDOWN_MS} ms"
        ));
    }
    Ok(())
}

/// Return the source PTS selected by the authored ShowClock coordinate.  A
/// position before a clip is not a valid source PTS, while addition overflow
/// is rejected instead of saturating into a different authored point.
pub fn timeline_audio_source_pts_ms(
    show_clock_position_ms: u64,
    clip_start_ms: u64,
    clip_offset_ms: u64,
) -> Option<u64> {
    show_clock_position_ms
        .checked_sub(clip_start_ms)
        .and_then(|local_ms| local_ms.checked_add(clip_offset_ms))
}

/// Decide whether a sink must be re-anchored to the ShowClock PTS.  This is a
/// pure integer rule shared by the native worker and deterministic tests.
pub fn timeline_audio_resync_required(
    policy: &TimelineAudioPolicy,
    drift_ms: i64,
    cooldown_elapsed_ms: u64,
) -> Result<bool, String> {
    validate_timeline_audio_policy(policy)?;
    Ok(drift_ms.unsigned_abs() > policy.max_drift_ms
        && cooldown_elapsed_ms >= policy.resync_cooldown_ms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn legacy_missing_policy_materializes_current_defaults_without_serializing_it() {
        let policy: TimelineAudioPolicy = serde_json::from_value(json!({})).unwrap();
        assert_eq!(policy, TimelineAudioPolicy::default());
        assert!(validate_timeline_audio_policy(&policy).is_ok());
    }

    #[test]
    fn future_and_invalid_policy_values_fail_closed() {
        let mut future = TimelineAudioPolicy::default();
        future.version += 1;
        assert!(validate_timeline_audio_policy(&future)
            .unwrap_err()
            .contains("unsupported"));

        let mut invalid_slew = TimelineAudioPolicy::default();
        invalid_slew.max_slew_ppm = 0;
        assert!(validate_timeline_audio_policy(&invalid_slew)
            .unwrap_err()
            .contains("max slew"));

        let error = serde_json::from_value::<TimelineAudioPolicy>(json!({
            "clock_master": "AUDIO_PTS"
        }))
        .unwrap_err();
        assert!(error.to_string().contains("unknown variant"));
    }

    #[test]
    fn show_clock_pts_and_bounded_resync_rule_are_integer_and_deterministic() {
        assert_eq!(timeline_audio_source_pts_ms(999, 1_000, 0), None);
        assert_eq!(timeline_audio_source_pts_ms(1_250, 1_000, 375), Some(625));
        assert_eq!(timeline_audio_source_pts_ms(u64::MAX, 0, u64::MAX), None);

        let policy = TimelineAudioPolicy::default();
        assert!(!timeline_audio_resync_required(&policy, 75, 1_000).unwrap());
        assert!(timeline_audio_resync_required(&policy, -76, 250).unwrap());
        assert!(!timeline_audio_resync_required(&policy, 500, 249).unwrap());
    }
}
