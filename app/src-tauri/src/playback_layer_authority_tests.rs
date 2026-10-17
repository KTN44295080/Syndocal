// DEC-PLAYBACK-LAYER-001 on the real headless harness: Cue playback keeps the
// authored checkpoint, exact tokens and Undo; Undo keeps the live look; a file
// saved with a live Cue opens with no Cue live and says so.
use super::*;
use engine::EngineCommand;
use engine::PlaybackLoadPolicy;
use protocol::{AttributeValueSummary, CueFixtureTarget, CueSummary, ProjectFile};

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

fn live_dimmer(state: &AppState) -> Option<u16> {
    state.engine.snapshot().fixtures[0]
        .attribute_values
        .iter()
        .find(|value| value.attribute == "Dimmer")
        .map(|value| value.value)
}

fn wait_for_live_cue(state: &AppState, cue_id: protocol::CueId) {
    for _ in 0..40 {
        let live = state.engine.snapshot();
        if live.active_cue_id == Some(cue_id) && live_dimmer(state) == Some(CUE_DIMMER) {
            return;
        }
        thread::sleep(Duration::from_millis(25));
    }
    panic!("Cue {cue_id} never became live");
}

#[test]
fn cue_trigger_keeps_the_exact_token_and_undo() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    let token = commit_effect_enabled(state, &query, effect_id);

    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .unwrap();
    wait_for_live_cue(state, cue_id);

    // The saved image carries authored state only.
    let saved = crate::project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert_eq!(saved.active_cue_id, None);
    assert!(saved.fixtures[0]
        .attribute_values
        .iter()
        .all(|value| value.value != CUE_DIMMER));

    // The production reconcile sees no change: the token stays exact, the
    // revision stays put and Undo survives.
    exact_preview_authority(state, &token.authority).unwrap();
    {
        let coordinator = state.project_coordinator.lock().unwrap();
        assert_eq!(coordinator.revision, token.authority.revision);
        assert_eq!(coordinator.checkpoint_hash, token.authority.checkpoint_hash);
        assert_eq!(coordinator.history.undo.len(), 1);
    }

    // Undo reverts the authored edit and keeps the live look.
    undo_with_exact_token(state, &token).unwrap();
    let after = state.engine.persistence_snapshot().unwrap();
    assert!(after
        .effects
        .iter()
        .any(|effect| effect.id == effect_id && !effect.enabled));
    assert_eq!(state.engine.snapshot().active_cue_id, Some(cue_id));
    assert_eq!(live_dimmer(state), Some(CUE_DIMMER));
    // The live layer did not leak into the restored checkpoint either.
    let coordinator = state.project_coordinator.lock().unwrap();
    let reconciled = crate::project_checkpoint_hash(
        &crate::project_file_for_save_from_parts(after, &coordinator.ancillary),
        &coordinator.mappings,
    )
    .unwrap();
    assert_eq!(reconciled, coordinator.checkpoint_hash);
}

#[test]
fn opening_a_file_saved_with_a_live_cue_starts_with_no_cue_live_and_says_so() {
    let _serial = serialized_authored_test_guard();
    let (harness, _query, _effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    // A version-1 file written before DEC-PLAYBACK-LAYER-001 carried the live
    // Cue references.
    let mut legacy = state.engine.persistence_snapshot().unwrap();
    legacy.active_cue_id = Some(cue_id);
    legacy.cue_lists[0].active_cue_id = Some(cue_id);
    let project = |snapshot| ProjectFile {
        version: crate::PROJECT_FILE_VERSION,
        app: crate::APP_NAME.to_string(),
        operator_policy: None,
        custom_profiles: Vec::new(),
        fixture_groups: Vec::new(),
        snapshot,
    };
    let prepared = crate::prepare_project_load(
        project(legacy.clone()),
        crate::ProjectControlMappings::default(),
        "legacy.sdc".to_string(),
        None,
    )
    .unwrap();
    assert!(!protocol::has_active_cue_references(&prepared.snapshot));
    assert_eq!(prepared.playback, PlaybackLoadPolicy::Discard);
    assert!(
        prepared.result.warnings.iter().any(|warning| warning
            == "Saved live Cue \"Live look\" was not restored; a show opens with no Cue live"),
        "{:?}",
        prepared.result.warnings
    );

    // A malformed reference is still rejected, never silently cleared (FC-27).
    let mut malformed = legacy;
    malformed.cue_lists[0].active_cue_id = Some(999);
    let error = crate::prepare_project_load(
        project(malformed),
        crate::ProjectControlMappings::default(),
        "malformed.sdc".to_string(),
        None,
    )
    .err()
    .expect("malformed active reference must be rejected");
    assert!(error.contains("reference integrity"), "{error}");
}

#[test]
fn project_file_saved_with_a_live_cue_reopens_as_its_authored_image() {
    let _serial = serialized_authored_test_guard();
    let (harness, _query, _effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .unwrap();
    wait_for_live_cue(state, cue_id);
    // The version-1 writer before DEC-PLAYBACK-LAYER-001 saved the live look
    // together with its active Cue references.
    let live = state.engine.live_look_snapshot().unwrap();
    let mut old_saved = crate::project_snapshot_for_save(live.clone());
    old_saved.active_cue_id = live.active_cue_id;
    old_saved.cue_lists = live.cue_lists.clone();
    old_saved.active_group_cue_ids = live.active_group_cue_ids.clone();
    assert!(protocol::has_active_cue_references(&old_saved));
    let bytes = crate::project_json_for_write_with_control_mappings_and_dj(
        &ProjectFile {
            version: crate::PROJECT_FILE_VERSION,
            app: crate::APP_NAME.to_string(),
            operator_policy: None,
            custom_profiles: Vec::new(),
            fixture_groups: Vec::new(),
            snapshot: old_saved,
        },
        Vec::new(),
        Vec::new(),
        Vec::new(),
        Vec::new(),
    )
    .unwrap()
    .into_bytes();

    // The current reader opens it through the production ingress.
    let value = crate::parse_project_json_bytes(&bytes).unwrap();
    let (file, mappings) = crate::project_and_control_mappings_from_value(value).unwrap();
    let prepared =
        crate::prepare_project_load(file, mappings, "legacy.sdc".to_string(), None).unwrap();
    assert!(prepared
        .result
        .warnings
        .iter()
        .any(|warning| warning.starts_with("Saved live Cue \"Live look\" was not restored")));
    crate::request_project_snapshot_publication(
        &state.engine,
        prepared.snapshot.clone(),
        PlaybackLoadPolicy::Discard,
    )
    .unwrap();

    // No Cue is live; the saved levels open as authored state.
    assert_eq!(state.engine.snapshot().active_cue_id, None);
    assert_eq!(live_dimmer(state), Some(CUE_DIMMER));
    // Saving again writes exactly the prepared authored image, without
    // references, so the migration is one-way and stable.
    let resaved = crate::project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert!(!protocol::has_active_cue_references(&resaved));
    assert_eq!(
        serde_json::to_value(&resaved).unwrap(),
        serde_json::to_value(&prepared.snapshot).unwrap()
    );
}

fn live_effect_enabled(state: &AppState, effect_id: protocol::EffectId) -> bool {
    state
        .engine
        .snapshot()
        .effects
        .iter()
        .find(|effect| effect.id == effect_id)
        .expect("fixture effect remains present")
        .enabled
}

#[test]
fn effect_toggle_takes_a_cue_held_flag_back_without_moving_the_token() {
    let _serial = serialized_authored_test_guard();
    let (harness, query, effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    // The Cue also turns the authored-off effect on (a legacy effect target).
    let mut snapshot = state.engine.persistence_snapshot().unwrap();
    snapshot
        .cues
        .iter_mut()
        .find(|cue| cue.id == cue_id)
        .unwrap()
        .effect_targets
        .push(protocol::CueEffectTarget {
            effect_id,
            enabled: true,
            params: None,
            transition_ms: None,
        });
    state
        .engine
        .load_project_snapshot_and_wait(snapshot)
        .expect("install effect-target Cue");
    *state.project_coordinator.lock().unwrap() = crate::project_coordinator_for_initial_snapshot(
        state.engine.persistence_snapshot().unwrap(),
    );
    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .unwrap();
    wait_for_live_cue(state, cue_id);
    assert!(live_effect_enabled(state, effect_id));
    let before = {
        let coordinator = state.project_coordinator.lock().unwrap();
        (
            coordinator.epoch,
            coordinator.revision,
            coordinator.checkpoint_hash.clone(),
        )
    };

    // Off is already the authored value, yet the operator sees the effect
    // on. The toggle turns it off live and leaves authored state alone.
    let applied = set_effect_enabled_authoritative_for_window_label(
        state,
        &query,
        PRIMARY_WINDOW,
        issued_request(state, &query, PRIMARY_WINDOW, effect_id, false, 302),
    );
    // Authored state did not change, so the receipt is a no-op.
    assert!(matches!(
        terminal_outcome(&applied),
        SetEffectEnabledOutcomeV1::NoOp(_)
    ));
    assert!(!live_effect_enabled(state, effect_id));
    assert_eq!(state.engine.snapshot().active_cue_id, Some(cue_id));
    let coordinator = state.project_coordinator.lock().unwrap();
    assert_eq!(
        (
            coordinator.epoch,
            coordinator.revision,
            coordinator.checkpoint_hash.clone()
        ),
        before
    );
    assert!(coordinator.history.undo.is_empty());
}

/// A generation of the current authored project; the decision under test
/// reads only its project, mappings and manifest checksum.
fn standby_checkpoint(project: ProjectFile) -> crate::StandbyCheckpoint {
    crate::StandbyCheckpoint {
        manifest: crate::StandbySyncManifest {
            version: crate::STANDBY_SYNC_VERSION,
            app: crate::APP_NAME.to_string(),
            session_id: "primary-a".to_string(),
            generation: 1,
            written_at_unix_ms: 1,
            project_file: crate::standby_project_file_name("primary-a", 1),
            project_bytes: 0,
            project_checksum: String::new(),
            playback_file: None,
            playback_bytes: None,
            playback_checksum: None,
        },
        project,
        mappings: crate::ProjectControlMappings::default(),
        playback: crate::StandbyPlayback::Absent,
    }
}

#[test]
fn standby_judges_playback_by_what_it_last_synced() {
    let _serial = serialized_authored_test_guard();
    let (harness, _query, _effect_id, cue_id) = seeded_live_show();
    let state = &harness.state;
    let authored = {
        let coordinator = state.project_coordinator.lock().unwrap();
        crate::project_file_for_save_from_parts(
            state.engine.persistence_snapshot().unwrap(),
            &coordinator.ancillary,
        )
    };
    let requires = |project: &ProjectFile, key: &str, synced: Option<&str>| {
        crate::standby_checkpoint_requires_identity_apply(
            state,
            &standby_checkpoint(project.clone()),
            key,
            synced,
        )
        .unwrap()
    };
    // A generation without a sidecar compares as an empty layer, exactly as
    // a current Primary with no Cue live writes it.
    let idle = crate::standby_playback_key(&standby_checkpoint(authored.clone()).manifest).unwrap();
    assert_eq!(
        idle,
        crate::standby_checksum(
            &crate::standby_playback_sidecar_bytes(
                &state.engine.playback_layer_snapshot().unwrap()
            )
            .unwrap()
        )
    );

    // The worker's first generation compares with what the Standby shows.
    assert!(!requires(&authored, &idle, None));
    state
        .engine
        .send(EngineCommand::TriggerCue(cue_id))
        .unwrap();
    wait_for_live_cue(state, cue_id);
    assert!(requires(&authored, &idle, None));

    // Later generations compare with what the worker last synced, so a layer
    // the Standby could not apply reloads once, not on every heartbeat.
    assert!(!requires(&authored, &idle, Some(&idle)));
    assert!(requires(&authored, "live-layer", Some(&idle)));
    assert!(!requires(&authored, "live-layer", Some("live-layer")));

    // An older Primary still sends live Cue references with otherwise
    // identical content; the load drops them, so they are not a change.
    let mut older = authored.clone();
    older.snapshot.active_cue_id = Some(cue_id);
    older.snapshot.cue_lists[0].active_cue_id = Some(cue_id);
    assert!(!requires(&older, &idle, Some(&idle)));

    // Authored changes still apply.
    let mut edited = authored;
    edited.snapshot.cues[0].label = "Edited".to_string();
    assert!(requires(&edited, &idle, Some(&idle)));
}
