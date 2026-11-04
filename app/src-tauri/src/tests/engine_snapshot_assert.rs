//! Engine-image equality for tests that read the live engine across ticks.
use protocol::EngineSnapshot;

/// The engine keeps ticking between two reads, so the clock phase, the
/// external sync age and tick telemetry may move; every other field of the
/// asserted image must be unchanged. Slow runners (macOS CI) tick between the
/// reads that a fast local machine usually does not.
#[track_caller]
pub(crate) fn assert_engine_snapshot_unchanged(
    mut actual: EngineSnapshot,
    expected: &EngineSnapshot,
    context: &str,
) {
    actual.clock.beat_phase = expected.clock.beat_phase;
    actual.clock.external_sync_age_ms = expected.clock.external_sync_age_ms;
    actual.telemetry = expected.telemetry.clone();
    assert_eq!(&actual, expected, "{context}");
}
