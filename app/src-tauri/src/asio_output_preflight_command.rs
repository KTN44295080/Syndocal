//! Strict wire boundary for operator ASIO output tests and bus solo controls.
//!
//! This module deliberately contains no Tauri or application state. The native
//! command handlers admit the exact request here, then apply the resulting
//! logical target to the already-active ASIO runtime under its session and
//! transport identity fences.

use crate::asio_program_cue::{PreflightSolo, PreflightTarget, PREFLIGHT_MAX_DURATION_MS};
use serde::Deserialize;

/// A short, bounded tone is long enough to identify one physical output while
/// still self-clearing if the operator cannot immediately press Off.
pub(crate) const OPERATOR_TEST_DURATION_MS: u64 = 1_500;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum TestTargetWire {
    Off,
    ProgramLeft,
    ProgramRight,
    ProgramStereo,
    Cue,
    Spare,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AsioOutputTestRequest {
    test: TestTargetWire,
}

impl AsioOutputTestRequest {
    pub(crate) const fn target(self) -> PreflightTarget {
        match self.test {
            TestTargetWire::Off => PreflightTarget::Off,
            TestTargetWire::ProgramLeft => PreflightTarget::ProgramLeft,
            TestTargetWire::ProgramRight => PreflightTarget::ProgramRight,
            TestTargetWire::ProgramStereo => PreflightTarget::ProgramStereo,
            TestTargetWire::Cue => PreflightTarget::Cue,
            TestTargetWire::Spare => PreflightTarget::Spare,
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
enum SoloModeWire {
    None,
    ProgramOnly,
    CueOnly,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AsioOutputSoloRequest {
    mode: SoloModeWire,
}

impl AsioOutputSoloRequest {
    pub(crate) const fn mode(self) -> PreflightSolo {
        match self.mode {
            SoloModeWire::None => PreflightSolo::None,
            SoloModeWire::ProgramOnly => PreflightSolo::ProgramOnly,
            SoloModeWire::CueOnly => PreflightSolo::CueOnly,
        }
    }
}

const _: () = assert!(OPERATOR_TEST_DURATION_MS > 0);
const _: () = assert!(OPERATOR_TEST_DURATION_MS <= PREFLIGHT_MAX_DURATION_MS);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_test_targets_map_to_logical_runtime_targets() {
        let cases = [
            (r#"{"test":"off"}"#, PreflightTarget::Off),
            (r#"{"test":"program-left"}"#, PreflightTarget::ProgramLeft),
            (r#"{"test":"program-right"}"#, PreflightTarget::ProgramRight),
            (
                r#"{"test":"program-stereo"}"#,
                PreflightTarget::ProgramStereo,
            ),
            (r#"{"test":"cue"}"#, PreflightTarget::Cue),
            (r#"{"test":"spare"}"#, PreflightTarget::Spare),
        ];

        for (json, expected) in cases {
            let request: AsioOutputTestRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.target(), expected);
        }
    }

    #[test]
    fn exact_solo_modes_map_to_logical_runtime_modes() {
        let cases = [
            (r#"{"mode":"none"}"#, PreflightSolo::None),
            (r#"{"mode":"program-only"}"#, PreflightSolo::ProgramOnly),
            (r#"{"mode":"cue-only"}"#, PreflightSolo::CueOnly),
        ];

        for (json, expected) in cases {
            let request: AsioOutputSoloRequest = serde_json::from_str(json).unwrap();
            assert_eq!(request.mode(), expected);
        }
    }

    #[test]
    fn malformed_legacy_and_ambiguous_requests_fail_closed() {
        for json in [
            r#"{}"#,
            r#"{"test":"program"}"#,
            r#"{"test":"PROGRAM-LEFT"}"#,
            r#"{"test":"cue","durationMs":30000}"#,
            r#"{"mode":"program"}"#,
            r#"{"mode":"off"}"#,
            r#"{"mode":"none","test":"off"}"#,
        ] {
            let accepted = serde_json::from_str::<AsioOutputTestRequest>(json).is_ok()
                || serde_json::from_str::<AsioOutputSoloRequest>(json).is_ok();
            assert!(!accepted, "request must fail closed: {json}");
        }
    }

    #[test]
    fn operator_test_duration_stays_inside_the_runtime_bound() {
        assert!(OPERATOR_TEST_DURATION_MS > 0);
        assert!(OPERATOR_TEST_DURATION_MS <= PREFLIGHT_MAX_DURATION_MS);
    }
}
