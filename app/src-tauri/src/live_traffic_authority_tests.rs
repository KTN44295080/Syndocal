use super::*;
use engine::EngineCommand;
use protocol::{AttributeValueSummary, ClockSource, CueFixtureTarget, CueSummary};

const PROJECT_BPM: f32 = 120.0;
const CUE_DIMMER: u16 = 40_000;

/// One real fixture, one disabled LFO effect and one Cue that recalls a
/// Dimmer level, installed as the coordinator's clean baseline.
fn seeded_live_show() -> (
    MediaAssetA6CommandHarness,
    ControlPlaneQueryState,
    protocol::EffectId,
    protocol::CueId,
) {
    let harness = MediaAssetA6CommandHarness::new();
    let fixture_id = harness.state.engine.allocate_fixture_id();
    let effect_id = harness.state.engine.allocate_effect_id();
    let cue_id = harness.state.engine.allocate_cue_id();
    let mut snapshot = harness.state.engine.snapshot();
    snapshot.fixtures.push(real_fixture(fixture_id));
    snapshot
        .effects
        .push(real_lfo_effect(effect_id, fixture_id, false));
    snapshot.cues.push(CueSummary {
        id: cue_id,
        cue_list_id: snapshot.cue_lists[0].id,
        label: "Live look".to_string(),
        targets: vec![CueFixtureTarget {
            fixture_id,
            values: vec![AttributeValueSummary {
                attribute: "Dimmer".to_string(),
                value: CUE_DIMMER,
            }],
        }],
        ..CueSummary::default()
    });
    snapshot.clock.bpm = PROJECT_BPM;
    harness
        .state
        .engine
        .load_project_snapshot_and_wait(snapshot)
        .expect("install live show fixture");
    *harness.state.project_coordinator.lock().unwrap() =
        crate::project_coordinator_for_initial_snapshot(harness.state.engine.snapshot());
    (
        harness,
        ControlPlaneQueryState::new().unwrap(),
        effect_id,
        cue_id,
    )
}

/// The exact identity a frontend holds after an authored commit, plus the
/// Undo entry that commit recorded.
struct CommittedToken {
    authority: crate::VjPreviewProjectAuthorityExpectation,
    undo_entry_id: u64,
}

fn commit_effect_enabled(
    state: &AppState,
    query: &ControlPlaneQueryState,
    effect_id: protocol::EffectId,
) -> CommittedToken {
    let applied = set_effect_enabled_authoritative_for_window_label(
        state,
        query,
        PRIMARY_WINDOW,
        issued_request(state, query, PRIMARY_WINDOW, effect_id, true, 301),
    );
    assert!(matches!(
        terminal_outcome(&applied),
        SetEffectEnabledOutcomeV1::Applied(_)
    ));
    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(coordinator.history.undo.len(), 1);
    CommittedToken {
        authority: crate::VjPreviewProjectAuthorityExpectation {
            epoch: coordinator.epoch,
            revision: coordinator.revision,
            checkpoint_hash: coordinator.checkpoint_hash.clone(),
        },
        undo_entry_id: coordinator.history.undo[0].entry_id,
    }
}

/// The production reconcile followed by the VJ Preview exact-authority
/// comparison, under the same admission -> coordinator lock order as the
/// `stage_vj_preview_layer` command body.
fn exact_preview_authority(
    state: &AppState,
    expected: &crate::VjPreviewProjectAuthorityExpectation,
) -> Result<(), String> {
    let _external_admission = crate::lock_project_external_command_admission(state)?;
    let mut coordinator = crate::lock_project_coordinator(state)?;
    crate::ensure_no_pending_project_transaction(&coordinator)?;
    crate::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
    crate::with_exact_vj_preview_project_authority(&coordinator, expected, || Ok(()))
}

fn undo_with_exact_token(
    state: &AppState,
    token: &CommittedToken,
) -> Result<crate::ProjectHistoryNavigationResult, String> {
    let _lifecycle = crate::lock_standby_sync_lifecycle_for_project_swap(state)?;
    crate::stop_standby_sync_for_project_swap(state)?;
    let _external_admission = crate::lock_project_external_command_admission(state)?;
    let mut coordinator = crate::lock_project_coordinator(state)?;
    crate::navigate_project_history_with_coordinator(
        state,
        &mut coordinator,
        true,
        Some(token.authority.epoch),
        Some(token.undo_entry_id),
        Some(token.authority.checkpoint_hash.clone()),
        replace_history_snapshot_in_real_headless_harness,
    )
}

/// Send real MIDI Clock pulses through the engine queue at a non-120 BPM
/// cadence and wait until the published clock follows them.
fn drive_midi_clock(state: &AppState) {
    for _ in 0..32 {
        state.engine.send(EngineCommand::MidiClockPulse).unwrap();
        thread::sleep(Duration::from_millis(15));
    }
    state.engine.await_snapshot_publication().unwrap();
    let live = state.engine.snapshot().clock;
    assert_eq!(live.source, ClockSource::MidiClock);
    assert_ne!(
        live.bpm, PROJECT_BPM,
        "the MIDI Clock estimate drives the live tempo"
    );
}

fn saved_clock_bpm(state: &AppState) -> f32 {
    crate::project_snapshot_for_save(state.engine.persistence_snapshot().unwrap())
        .clock
        .bpm
}

#[test]
fn live_midi_clock_and_link_keep_the_committed_token_and_undo() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id, _) = seeded_live_show();
    let state = &harness.state;
    let token = commit_effect_enabled(state, &query, effect_id);
    let committed_image = engine_persistence_bytes(state);

    drive_midi_clock(state);
    state
        .engine
        .send(EngineCommand::SyncExternalClock {
            bpm: 128.0,
            beat_phase: 0.25,
            source: ClockSource::AbletonLink,
        })
        .unwrap();
    state.engine.await_snapshot_publication().unwrap();
    assert_eq!(state.engine.snapshot().clock.bpm, 128.0);

    // External tempo is followed live but is not the project's tempo.
    assert_eq!(saved_clock_bpm(state), PROJECT_BPM);
    assert_eq!(engine_persistence_bytes(state), committed_image);
    exact_preview_authority(state, &token.authority)
        .expect("live clock traffic leaves the committed exact token current");
    {
        let coordinator = state.project_coordinator.lock().unwrap();
        assert_eq!(coordinator.revision, token.authority.revision);
        assert_eq!(
            coordinator.history.undo.len(),
            1,
            "Undo survives live clock"
        );
    }
    let undone = undo_with_exact_token(state, &token)
        .expect("the exact Undo token is still valid after live clock traffic");
    assert_eq!(undone.history_status.undo_depth, 0);
    assert!(!effect_enabled(state, effect_id));
}

#[test]
fn manual_tempo_outside_a_transaction_still_retires_the_token() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id, _) = seeded_live_show();
    let state = &harness.state;
    let token = commit_effect_enabled(state, &query, effect_id);
    drive_midi_clock(state);
    exact_preview_authority(state, &token.authority).unwrap();

    // A manual tempo is project data. Sent outside a ticket it is an
    // unrecorded authored edit and must still fail closed.
    crate::set_bpm_for_state(state, 137.0).unwrap();
    state.engine.await_snapshot_publication().unwrap();
    assert_eq!(saved_clock_bpm(state), 137.0);
    let error = exact_preview_authority(state, &token.authority).unwrap_err();
    assert!(
        error.contains("Project changed before VJ Preview staging"),
        "{error}"
    );
    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(coordinator.revision, token.authority.revision + 1);
    assert!(coordinator.history.undo.is_empty());
}

/// DEC-PLAYBACK-LAYER-001 replaced the fail-closed Cue contract: a Cue and
/// live clock traffic together leave the saved image, the exact token and
/// Undo untouched, and Undo keeps the live look.
#[test]
fn cue_trigger_and_live_clock_keep_the_committed_token_and_undo() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    let token = commit_effect_enabled(state, &query, effect_id);
    let committed_image = engine_persistence_bytes(state);

    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .unwrap();
    drive_midi_clock(state);
    let live = state.engine.snapshot();
    assert_eq!(live.active_cue_id, Some(cue_id));
    assert!(live.fixtures[0]
        .attribute_values
        .iter()
        .any(|value| value.attribute == "Dimmer" && value.value == CUE_DIMMER));

    // The saved image is the authored one: no live Cue, no Cue level, the
    // project tempo.
    assert_eq!(engine_persistence_bytes(state), committed_image);
    let saved = crate::project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert_eq!(saved.active_cue_id, None);
    assert!(saved.fixtures[0]
        .attribute_values
        .iter()
        .all(|value| value.value != CUE_DIMMER));
    assert_eq!(saved.clock.bpm, PROJECT_BPM);

    exact_preview_authority(state, &token.authority)
        .expect("Cue and clock traffic leave the committed exact token current");
    {
        let coordinator = state.project_coordinator.lock().unwrap();
        assert_eq!(coordinator.revision, token.authority.revision);
        assert_eq!(coordinator.history.undo.len(), 1, "Undo survives playback");
    }
    let undone = undo_with_exact_token(state, &token)
        .expect("the exact Undo token is still valid after Cue and clock traffic");
    assert_eq!(undone.history_status.undo_depth, 0);
    assert!(!effect_enabled(state, effect_id));
    let after = state.engine.snapshot();
    assert_eq!(after.active_cue_id, Some(cue_id), "Undo keeps the live Cue");
    assert!(after.fixtures[0]
        .attribute_values
        .iter()
        .any(|value| value.attribute == "Dimmer" && value.value == CUE_DIMMER));
}
