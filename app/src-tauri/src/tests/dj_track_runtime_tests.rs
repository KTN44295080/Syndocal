use super::*;

#[test]
fn operator_return_candidate_clears_only_dj_owner_and_preserves_timeline_truth() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let empty = DjLinkRuntime::from_coordinator(&coordinator);
    assert_eq!(
        empty.operator_return_candidate(&engine.snapshot()),
        Err("dj_control_return_owner_unavailable")
    );

    let mut runtime = empty;
    runtime.state_generation = 12;
    runtime.track_active = true;
    runtime.playing = true;
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.released = true;
    runtime.active_dedupe_key = Some("old-dedupe".to_string());
    runtime.track_deck_number = Some(2);
    runtime.track_deck_id = Some("rekordbox-deck-2".to_string());
    runtime.track_content_id = Some("old-content".to_string());
    runtime.track_playing = true;
    runtime.play_session_id = Some("old-session".to_string());
    runtime.pedal_owner = Some("timeline".to_string());
    runtime.release_event_id = Some("old-release".to_string());
    runtime.timeline_id = Some("999".to_string());
    runtime.loop_active = true;
    runtime
        .seen_play_sessions
        .insert("retained-session-receipt".to_string(), Instant::now());

    let mut snapshot = engine.snapshot();
    assert_eq!(
        runtime.operator_return_candidate(&snapshot),
        Err("dj_control_return_requires_running_timeline")
    );
    snapshot.timeline.id = TimelineId(77);
    snapshot.timeline.playing = true;
    snapshot.timeline.position_ms = 4_000;
    snapshot.timeline.loop_runtime.status = protocol::TimelineLoopRuntimeStatus::Looping;
    let next = runtime.operator_return_candidate(&snapshot).unwrap();
    assert_eq!(next.state_generation, 13);
    assert!(!next.track_active);
    assert!(next.playing);
    assert!(!next.released);
    assert_eq!(next.active_dedupe_key, None);
    assert_eq!(next.track_deck_number, None);
    assert_eq!(next.track_deck_id, None);
    assert_eq!(next.track_content_id, None);
    assert!(!next.track_playing);
    assert_eq!(next.play_session_id, None);
    assert_eq!(next.pedal_owner, None);
    assert_eq!(next.release_event_id, None);
    assert_eq!(next.timeline_id.as_deref(), Some("77"));
    assert!(next.loop_active);
    assert_eq!(next.mappings, runtime.mappings);
    assert_eq!(next.seen_play_sessions, runtime.seen_play_sessions);
    let state = dj_link_timeline_state_from_snapshot(&next, &snapshot, "pending", 1);
    assert_eq!(state.state, protocol::DjLinkTimelineStateValue::Running);
    assert!(state.loop_active);
    assert_eq!(state.play_session_id, None);
    assert_eq!(state.pedal_owner, None);

    // A lost queued frame can be explicitly retried without inventing an
    // owner or touching transport position.
    assert!(next.operator_return_candidate(&snapshot).is_ok());
}

#[test]
fn operator_return_runtime_commits_only_after_the_correlated_frame_is_queued() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut runtime = DjLinkRuntime::from_coordinator(&coordinator);
    runtime.track_active = true;
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.play_session_id = Some("owned-session".to_string());
    runtime.pedal_owner = Some("timeline".to_string());
    runtime.timeline_id = Some("77".to_string());
    let before = runtime.clone();
    let mut snapshot = engine.snapshot();
    snapshot.timeline.playing = true;
    let next = runtime.operator_return_candidate(&snapshot).unwrap();
    let outbound = dj_link_timeline_state_from_snapshot(&next, &snapshot, "pending", 1);

    let rejected = queue_and_commit_dj_link_operator_return(
        &mut runtime,
        next.clone(),
        outbound.clone(),
        |_| Ok(None),
    );
    assert!(rejected.is_err());
    assert_eq!(runtime, before);

    let failed = queue_and_commit_dj_link_operator_return(
        &mut runtime,
        next.clone(),
        outbound.clone(),
        |_| Err("queue failed".to_string()),
    );
    assert_eq!(failed, Err("queue failed".to_string()));
    assert_eq!(runtime, before);

    let committed =
        queue_and_commit_dj_link_operator_return(&mut runtime, next.clone(), outbound, |_| {
            Ok(Some("syndocal-dj-operator-return-9".to_string()))
        });
    assert_eq!(committed, Ok(()));
    assert_eq!(
        runtime.pending_operator_return_request_id.as_deref(),
        Some("syndocal-dj-operator-return-9")
    );
    let mut expected = next;
    expected.pending_operator_return_request_id = Some("syndocal-dj-operator-return-9".to_string());
    assert_eq!(runtime, expected);
}

#[test]
fn operator_return_intent_survives_project_mapping_publication_without_stale_owner() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    coordinator.epoch = 41;
    let mut runtime = DjLinkRuntime::from_coordinator(&coordinator);
    runtime.pending_operator_return_request_id =
        Some("syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-7".to_string());
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.timeline_id = Some("77".to_string());
    runtime.position_bars = 19;
    runtime.loop_active = true;
    runtime.track_active = true;
    runtime.track_deck_number = Some(2);
    runtime.track_deck_id = Some("retired-deck".to_string());
    runtime.play_session_id = Some("retired-session".to_string());
    runtime.pedal_owner = Some("timeline".to_string());
    runtime.release_event_id = Some("retired-release".to_string());
    runtime.active_dedupe_key = Some("retired-dedupe".to_string());

    coordinator.epoch = 42;
    coordinator.mappings.dj_track_triggers = vec![protocol::DjTrackTriggerMapping {
        id: "replacement-mapping".to_string(),
        selector: protocol::DjTrackSelector {
            content_id: Some("current-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
        timeline_id: TimelineId(77),
        retrigger: protocol::DjTrackRetriggerPolicy::OncePerPlaySession,
    }];
    runtime.sync_project(&coordinator);

    assert_eq!(runtime.project_epoch, 42);
    assert_eq!(runtime.mappings, coordinator.mappings.dj_track_triggers);
    assert_eq!(
        runtime.pending_operator_return_request_id.as_deref(),
        Some("syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-7")
    );
    assert_eq!(
        runtime.authoritative_state,
        protocol::DjLinkTimelineStateValue::Running
    );
    assert_eq!(runtime.timeline_id.as_deref(), Some("77"));
    assert_eq!(runtime.position_bars, 19);
    assert!(runtime.loop_active);
    assert!(!runtime.track_active);
    assert_eq!(runtime.track_deck_number, None);
    assert_eq!(runtime.track_deck_id, None);
    assert_eq!(runtime.play_session_id, None);
    assert_eq!(runtime.pedal_owner, None);
    assert_eq!(runtime.release_event_id, None);
    assert_eq!(runtime.active_dedupe_key, None);

    let mut no_pending = runtime;
    no_pending.pending_operator_return_request_id = None;
    no_pending.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    no_pending.timeline_id = Some("77".to_string());
    coordinator.epoch = 43;
    no_pending.sync_project(&coordinator);
    assert_eq!(
        no_pending.authoritative_state,
        protocol::DjLinkTimelineStateValue::Idle
    );
    assert_eq!(no_pending.timeline_id, None);
    assert_eq!(no_pending.position_bars, 0);
    assert!(!no_pending.loop_active);
}

#[test]
fn operator_return_mapping_publication_core_preserves_intent_retires_owner_and_admits_fresh_active()
{
    // This enters the same post-admission publication core as
    // `set_project_control_mappings`; only the Webview owner/transaction
    // preamble is outside this test fixture.  Keep the engine live so the
    // new mapping is checked against the actual authored Timeline bank.
    let harness = MediaAssetA6CommandHarness::new();
    let initial = harness.state.engine.snapshot();
    let timeline_id = initial.timeline.id;
    harness
        .state
        .engine
        .apply_timeline_bank_published(vec![initial.timeline.clone()], timeline_id, false)
        .unwrap();
    harness
        .state
        .engine
        .dj_link_start_timeline_at_with_canonical_snapshot(timeline_id, 4_000)
        .unwrap();
    let engine_before = harness.state.engine.snapshot();

    let mut coordinator = lock_project_coordinator(&harness.state).unwrap();
    reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator).unwrap();
    let expected_epoch = coordinator.epoch;
    let expected_revision = coordinator.revision;
    let status_before = project_control_mappings_status(&coordinator);
    let callback_epoch_before = harness
        .state
        .project_mapping_callback_epoch
        .load(Ordering::Acquire);

    let operator_return_id =
        "syndocal-dj-operator-return-0123456789abcdef0123456789abcdef-7".to_string();
    let runtime_before_invalid = {
        let mut runtime = harness.state.dj_link_runtime.lock().unwrap();
        runtime.sync_project(&coordinator);
        runtime.state_generation = 22;
        runtime.pending_operator_return_request_id = Some(operator_return_id.clone());
        runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
        runtime.timeline_id = Some(timeline_id.0.to_string());
        runtime.position_bars = dj_link_engine_position_bars(&engine_before);
        runtime.loop_active = true;
        runtime.track_active = true;
        runtime.playing = true;
        runtime.released = true;
        runtime.unmapped_active_blocked = true;
        runtime.active_dedupe_key = Some("retired-dedupe".to_string());
        runtime.track_content_id = Some("retired-content".to_string());
        runtime.track_deck_number = Some(2);
        runtime.track_deck_id = Some("rekordbox-deck-2".to_string());
        runtime.track_playing = true;
        runtime.play_session_id = Some("retired-session".to_string());
        runtime.pedal_owner = Some("timeline".to_string());
        runtime.release_event_id = Some("retired-release".to_string());
        runtime
            .seen_play_sessions
            .insert("retired-dedupe".to_string(), Instant::now());
        runtime.clone()
    };

    // Snapshot validation fails before callback reservation, worker
    // retirement, coordinator mutation, or DJ runtime mutation.
    let invalid_mapping = protocol::DjTrackTriggerMapping {
        id: "invalid-current-timeline".to_string(),
        selector: protocol::DjTrackSelector {
            content_id: Some("return-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
        timeline_id: TimelineId(timeline_id.0 + 1),
        retrigger: protocol::DjTrackRetriggerPolicy::OncePerPlaySession,
    };
    let rejected = publish_project_control_mappings_after_validation(
        &harness.state,
        &mut coordinator,
        expected_epoch,
        expected_revision,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![invalid_mapping],
    );
    assert_eq!(
        rejected,
        Err(format!(
            "DJ track mapping 'invalid-current-timeline' targets missing authored TimelineId {}",
            timeline_id.0 + 1
        ))
    );
    assert_eq!(project_control_mappings_status(&coordinator), status_before);
    assert_eq!(
        harness
            .state
            .project_mapping_callback_epoch
            .load(Ordering::Acquire),
        callback_epoch_before
    );
    assert_eq!(
        *harness.state.dj_link_runtime.lock().unwrap(),
        runtime_before_invalid
    );

    let current_mapping = protocol::DjTrackTriggerMapping {
        id: "fresh-return-candidate".to_string(),
        selector: protocol::DjTrackSelector {
            content_id: Some("return-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
        timeline_id,
        retrigger: protocol::DjTrackRetriggerPolicy::OncePerPlaySession,
    };
    let published = publish_project_control_mappings_after_validation(
        &harness.state,
        &mut coordinator,
        expected_epoch,
        expected_revision,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        vec![current_mapping.clone()],
    )
    .unwrap();
    assert!(published.mapping_runtimes_retired);
    assert_eq!(published.project_epoch, expected_epoch);
    assert_eq!(published.project_revision, expected_revision + 1);
    assert_eq!(
        published.history_generation,
        status_before.history_generation + 1
    );
    assert_eq!(published.dj_track_triggers, vec![current_mapping.clone()]);
    assert_eq!(
        harness
            .state
            .project_mapping_callback_epoch
            .load(Ordering::Acquire),
        callback_epoch_before + 1
    );

    let mismatch = protocol::DjLinkTrackPayload {
        deck: 1,
        deck_id: "rekordbox-deck-1".to_string(),
        content_id: Some("wrong-current-content".to_string()),
        title: None,
        artist: None,
        track_bpm: Some(120.0),
        position_at_send_sec: 4.0,
        effective_bpm: 120.0,
        position_revision: 8,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-28T00:00:00Z".to_string(),
        play_session_id: "fresh-return-session".to_string(),
        loop_state: None,
    };
    let accepted = {
        let mut runtime = harness.state.dj_link_runtime.lock().unwrap();
        assert_eq!(
            runtime.pending_operator_return_request_id.as_deref(),
            Some(operator_return_id.as_str())
        );
        assert_eq!(
            runtime.authoritative_state,
            protocol::DjLinkTimelineStateValue::Running
        );
        assert_eq!(
            runtime.timeline_id.as_deref(),
            Some(timeline_id.0.to_string().as_str())
        );
        assert_eq!(
            runtime.position_bars,
            dj_link_engine_position_bars(&engine_before)
        );
        assert!(runtime.loop_active);
        assert!(!runtime.track_active);
        assert!(!runtime.playing);
        assert!(!runtime.released);
        assert!(!runtime.unmapped_active_blocked);
        assert_eq!(runtime.active_dedupe_key, None);
        assert_eq!(runtime.track_content_id, None);
        assert_eq!(runtime.track_deck_number, None);
        assert_eq!(runtime.track_deck_id, None);
        assert_eq!(runtime.play_session_id, None);
        assert_eq!(runtime.pedal_owner, None);
        assert_eq!(runtime.release_event_id, None);
        assert!(runtime.seen_play_sessions.is_empty());

        assert!(matches!(
            dj_track_runtime::dispatch_active(
                mismatch,
                &harness.state.engine,
                &mut runtime,
                22,
                "operator-return-mismatch",
                9,
            ),
            DjLinkDispatchOutcome::NoMapping {
                state_generation: 22
            }
        ));
        assert_eq!(
            runtime.pending_operator_return_request_id.as_deref(),
            Some(operator_return_id.as_str())
        );
        assert!(!runtime.track_active);

        dj_track_runtime::dispatch_active(
            protocol::DjLinkTrackPayload {
                content_id: Some("return-content".to_string()),
                ..mismatch_payload_for_operator_return()
            },
            &harness.state.engine,
            &mut runtime,
            22,
            "operator-return-fresh-active",
            10,
        )
    };
    let DjLinkDispatchOutcome::TimelineState {
        state_generation,
        state,
    } = accepted
    else {
        panic!("the fresh mapped candidate must be admitted after publication");
    };
    assert_eq!(state_generation, 23);
    assert_eq!(harness.state.engine.snapshot(), engine_before);
    let runtime = harness.state.dj_link_runtime.lock().unwrap();
    assert!(runtime.track_active);
    assert!(!runtime.released);
    assert_eq!(runtime.track_deck_number, Some(1));
    assert_eq!(runtime.track_deck_id.as_deref(), Some("rekordbox-deck-1"));
    assert_eq!(
        runtime.play_session_id.as_deref(),
        Some("fresh-return-session")
    );
    assert_eq!(runtime.pedal_owner.as_deref(), Some("dj"));
    assert_eq!(runtime.pending_operator_return_request_id, None);
    assert_eq!(state.timeline_id, timeline_id.0.to_string());
    assert_eq!(state.operator_return_request_id, None);
}

fn mismatch_payload_for_operator_return() -> protocol::DjLinkTrackPayload {
    protocol::DjLinkTrackPayload {
        deck: 1,
        deck_id: "rekordbox-deck-1".to_string(),
        content_id: None,
        title: None,
        artist: None,
        track_bpm: Some(120.0),
        position_at_send_sec: 4.0,
        effective_bpm: 120.0,
        position_revision: 8,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-28T00:00:00Z".to_string(),
        play_session_id: "fresh-return-session".to_string(),
        loop_state: None,
    }
}

#[test]
fn operator_return_reannouncement_claims_the_same_timeline_without_transport_mutation() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let initial = engine.snapshot();
    let timeline_id = initial.timeline.id;
    engine
        .apply_timeline_bank_published(vec![initial.timeline.clone()], timeline_id, false)
        .unwrap();
    engine
        .dj_link_start_timeline_at_with_canonical_snapshot(timeline_id, 4_000)
        .unwrap();
    let engine_before = engine.snapshot();
    let coordinator = project_coordinator_for_initial_snapshot(engine_before.clone());
    let mapping = protocol::DjTrackTriggerMapping {
        id: "operator-return-track".to_string(),
        selector: protocol::DjTrackSelector {
            content_id: Some("return-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
        timeline_id,
        retrigger: protocol::DjTrackRetriggerPolicy::OncePerPlaySession,
    };
    let mut runtime = DjLinkRuntime::from_coordinator(&coordinator);
    runtime.mappings = vec![mapping.clone()];
    runtime.state_generation = 9;
    runtime.playing = true;
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.timeline_id = Some(timeline_id.0.to_string());
    runtime.position_bars = dj_link_engine_position_bars(&engine_before);
    runtime.pending_operator_return_request_id = Some("syndocal-dj-operator-return-9".to_string());
    let replacement_sync = protocol::DjLinkTrackStateSyncPayload {
        released: false,
        owner_deck: Some(1),
        owner_deck_id: Some("rekordbox-deck-1".to_string()),
        active_play_session_id: Some("same-play-session".to_string()),
    };
    assert_eq!(
        dj_track_runtime::state_sync_owner_context_rejection(&runtime, &replacement_sync),
        None,
        "a replacement peer must pass STATE_SYNC while the correlated return remains pending"
    );
    let mut without_pending_return = runtime.clone();
    without_pending_return.pending_operator_return_request_id = None;
    assert_eq!(
        dj_track_runtime::state_sync_owner_context_rejection(
            &without_pending_return,
            &replacement_sync,
        ),
        Some("state_sync_owner_context_mismatch"),
        "the owner mismatch exception must not escape the bounded pending-return window"
    );
    let dedupe_key = format!(
        "{}:{}:{}:{}:{}",
        runtime.project_epoch, mapping.id, 1, "rekordbox-deck-1", "same-play-session"
    );
    runtime
        .seen_play_sessions
        .insert(dedupe_key.clone(), Instant::now());
    let payload = protocol::DjLinkTrackPayload {
        deck: 1,
        deck_id: "rekordbox-deck-1".to_string(),
        content_id: Some("return-content".to_string()),
        title: None,
        artist: None,
        track_bpm: Some(120.0),
        position_at_send_sec: 4.0,
        effective_bpm: 120.0,
        position_revision: 8,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-28T00:00:00Z".to_string(),
        play_session_id: "same-play-session".to_string(),
        loop_state: None,
    };

    let outcome = dj_track_runtime::dispatch_active(
        payload,
        &engine,
        &mut runtime,
        9,
        "operator-return-active",
        10,
    );
    let DjLinkDispatchOutcome::TimelineState {
        state_generation,
        state,
    } = outcome
    else {
        panic!("operator return candidate must claim the existing Timeline");
    };
    assert_eq!(state_generation, 10);
    assert_eq!(engine.snapshot(), engine_before);
    assert!(runtime.track_active);
    assert!(!runtime.released);
    assert_eq!(
        runtime.active_dedupe_key.as_deref(),
        Some(dedupe_key.as_str())
    );
    assert_eq!(
        runtime.play_session_id.as_deref(),
        Some("same-play-session")
    );
    assert_eq!(runtime.pedal_owner.as_deref(), Some("dj"));
    assert_eq!(runtime.pending_operator_return_request_id, None);
    assert_eq!(state.timeline_id, timeline_id.0.to_string());
    assert_eq!(
        state.position_bars,
        dj_link_engine_position_bars(&engine_before)
    );
    assert_eq!(state.operator_return_request_id, None);
}

#[test]
fn completed_follow_rebases_only_the_exact_released_stage2_receipt() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut runtime = DjLinkRuntime::from_coordinator(&coordinator);
    runtime.state_generation = 7;
    runtime.track_active = true;
    runtime.released = true;
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.pedal_owner = Some("timeline".to_string());
    runtime.release_event_id = Some("release-exact".to_string());
    runtime.play_session_id = Some("play-exact".to_string());
    runtime.timeline_id = Some("801".to_string());

    let completed_snapshot = || EngineSnapshot {
        timeline: TimelineSnapshot {
            id: TimelineId(802),
            playing: true,
            follow_runtime: protocol::TimelineFollowRuntimeSummary {
                generation: 19,
                status: protocol::TimelineFollowRuntimeStatus::Idle,
                outcome: Some(protocol::TimelineFollowOutcome::Completed),
                source_timeline_id: Some(TimelineId(801)),
                target_timeline_id: Some(TimelineId(802)),
                transition_hold_active: true,
                ..protocol::TimelineFollowRuntimeSummary::default()
            },
            loop_runtime: protocol::TimelineLoopRuntimeSummary {
                generation: 20,
                status: protocol::TimelineLoopRuntimeStatus::Looping,
                a_ms: Some(0),
                b_ms: Some(1_000),
                ..protocol::TimelineLoopRuntimeSummary::default()
            },
            ..TimelineSnapshot::default()
        },
        ..EngineSnapshot::default()
    };

    let snapshot = completed_snapshot();
    assert!(dj_track_runtime::rebase_released_follow_completion(
        &mut runtime,
        &snapshot,
        Some("802"),
        Some("play-exact"),
    ));
    assert_eq!(runtime.timeline_id.as_deref(), Some("802"));
    assert_eq!(runtime.state_generation, 8);
    assert!(runtime.loop_active);
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(&runtime, "802", "play-exact", &snapshot),
        None
    );
    assert_eq!(
        dj_track_runtime::timeline_loop_set_authority_rejection(&runtime, &snapshot, false),
        None
    );
    assert_eq!(
        dj_track_runtime::timeline_loop_set_authority_rejection(&runtime, &snapshot, true),
        Some("timeline_loop_state_unchanged")
    );
    let mut mismatched_loop_runtime = runtime.clone();
    mismatched_loop_runtime.loop_active = false;
    assert_eq!(
        dj_track_runtime::timeline_loop_set_authority_rejection(
            &mismatched_loop_runtime,
            &snapshot,
            false,
        ),
        Some("timeline_loop_state_mismatch")
    );
    let state = dj_link_timeline_state_from_snapshot(&runtime, &snapshot, "state", 1);
    assert!(state.transition_hold_active);

    let mut settling = snapshot.clone();
    settling.timeline.follow_runtime.status = protocol::TimelineFollowRuntimeStatus::Settling;
    settling.timeline.follow_runtime.outcome = None;
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(&runtime, "802", "play-exact", &settling),
        Some("timeline_follow_settling")
    );

    // The exact completion is idempotent; a lost outbound state reply never
    // advances the runtime generation or replays the rebase.
    assert!(!dj_track_runtime::rebase_released_follow_completion(
        &mut runtime,
        &snapshot,
        Some("802"),
        Some("play-exact"),
    ));
    assert_eq!(runtime.state_generation, 8);

    fn fault_outcome(snapshot: &mut EngineSnapshot) {
        snapshot.timeline.follow_runtime.outcome = Some(protocol::TimelineFollowOutcome::Fault);
    }
    fn mismatched_target(snapshot: &mut EngineSnapshot) {
        snapshot.timeline.id = TimelineId(999);
    }
    fn stale_generation(snapshot: &mut EngineSnapshot) {
        snapshot.timeline.follow_runtime.generation = 0;
    }
    for mutate in [
        fault_outcome as fn(&mut EngineSnapshot),
        mismatched_target,
        stale_generation,
    ] {
        let mut candidate = DjLinkRuntime::from_coordinator(&coordinator);
        candidate.state_generation = 7;
        candidate.track_active = true;
        candidate.released = true;
        candidate.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
        candidate.pedal_owner = Some("timeline".to_string());
        candidate.release_event_id = Some("release-exact".to_string());
        candidate.play_session_id = Some("play-exact".to_string());
        candidate.timeline_id = Some("801".to_string());
        let mut rejected = completed_snapshot();
        mutate(&mut rejected);
        let before = candidate.clone();
        assert!(!dj_track_runtime::rebase_released_follow_completion(
            &mut candidate,
            &rejected,
            Some("802"),
            Some("play-exact"),
        ));
        assert_eq!(candidate, before);
    }

    let mut wrong_session = DjLinkRuntime::from_coordinator(&coordinator);
    wrong_session.state_generation = 7;
    wrong_session.track_active = true;
    wrong_session.released = true;
    wrong_session.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    wrong_session.pedal_owner = Some("timeline".to_string());
    wrong_session.release_event_id = Some("release-exact".to_string());
    wrong_session.play_session_id = Some("play-exact".to_string());
    wrong_session.timeline_id = Some("801".to_string());
    let before = wrong_session.clone();
    assert!(!dj_track_runtime::rebase_released_follow_completion(
        &mut wrong_session,
        &completed_snapshot(),
        Some("802"),
        Some("other-session"),
    ));
    assert_eq!(wrong_session, before);
}

#[test]
fn waiting_follow_pedal_start_requires_exact_rebased_paused_target() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut runtime = DjLinkRuntime::from_coordinator(&coordinator);
    runtime.track_active = true;
    runtime.released = true;
    runtime.authoritative_state = protocol::DjLinkTimelineStateValue::Running;
    runtime.pedal_owner = Some("timeline".to_string());
    runtime.release_event_id = Some("release-1".to_string());
    runtime.play_session_id = Some("session-1".to_string());
    runtime.timeline_id = Some("802".to_string());
    runtime.last_follow_rebase = Some((19, "801".to_string(), "802".to_string()));
    let snapshot = EngineSnapshot {
        timeline: TimelineSnapshot {
            id: TimelineId(802),
            playing: false,
            position_ms: 0,
            follow_runtime: protocol::TimelineFollowRuntimeSummary {
                generation: 19,
                status: protocol::TimelineFollowRuntimeStatus::Idle,
                outcome: Some(protocol::TimelineFollowOutcome::Completed),
                source_timeline_id: Some(TimelineId(801)),
                target_timeline_id: Some(TimelineId(802)),
                waiting_for_pedal_start: true,
                ..protocol::TimelineFollowRuntimeSummary::default()
            },
            loop_runtime: protocol::TimelineLoopRuntimeSummary {
                generation: 20,
                status: protocol::TimelineLoopRuntimeStatus::Disabled,
                ..protocol::TimelineLoopRuntimeSummary::default()
            },
            ..TimelineSnapshot::default()
        },
        ..EngineSnapshot::default()
    };
    let waiting_state = dj_link_timeline_state_from_snapshot(&runtime, &snapshot, "wait-state", 1);
    assert_eq!(
        waiting_state.state,
        protocol::DjLinkTimelineStateValue::Running,
        "the paused target must retain Timeline control rather than masquerading as stopped"
    );
    assert!(!waiting_state.loop_active);
    assert!(!waiting_state.transition_hold_active);
    assert_eq!(waiting_state.timeline_id, "802");
    assert_eq!(waiting_state.play_session_id.as_deref(), Some("session-1"));
    assert_eq!(waiting_state.pedal_owner.as_deref(), Some("timeline"));
    assert_eq!(waiting_state.release_event_id.as_deref(), Some("release-1"));

    let mut ordinary_pause = snapshot.clone();
    ordinary_pause
        .timeline
        .follow_runtime
        .waiting_for_pedal_start = false;
    assert_eq!(
        dj_link_timeline_state_from_snapshot(&runtime, &ordinary_pause, "ordinary-pause", 2).state,
        protocol::DjLinkTimelineStateValue::Stopped,
        "only the exact runtime wait is allowed to retain Timeline control"
    );
    let mut bare_wait = snapshot.clone();
    bare_wait.timeline.follow_runtime.outcome = None;
    assert_eq!(
        dj_link_timeline_state_from_snapshot(&runtime, &bare_wait, "bare-wait", 3).state,
        protocol::DjLinkTimelineStateValue::Stopped,
        "a bare waiting flag without the exact completed Follow receipt is never Running"
    );
    assert_eq!(
        dj_track_runtime::waiting_follow_pedal_start_authority_rejection(
            &runtime,
            &snapshot,
            "802",
            "session-1",
        ),
        None
    );

    fn playing(candidate: &mut EngineSnapshot) {
        candidate.timeline.playing = true;
    }
    fn moved(candidate: &mut EngineSnapshot) {
        candidate.timeline.position_ms = 1;
    }
    fn loop_armed(candidate: &mut EngineSnapshot) {
        candidate.timeline.loop_runtime.status = protocol::TimelineLoopRuntimeStatus::Armed;
    }
    fn wait_consumed(candidate: &mut EngineSnapshot) {
        candidate.timeline.follow_runtime.waiting_for_pedal_start = false;
    }
    for mutate in [
        playing as fn(&mut EngineSnapshot),
        moved,
        loop_armed,
        wait_consumed,
    ] {
        let mut stale = snapshot.clone();
        mutate(&mut stale);
        let before = runtime.clone();
        assert!(
            dj_track_runtime::waiting_follow_pedal_start_authority_rejection(
                &runtime,
                &stale,
                "802",
                "session-1",
            )
            .is_some()
        );
        assert_eq!(
            runtime, before,
            "rejected wait state must not mutate DJ runtime"
        );
    }
    assert_eq!(
        dj_track_runtime::waiting_follow_pedal_start_authority_rejection(
            &runtime,
            &snapshot,
            "802",
            "wrong-session",
        ),
        Some("timeline_waiting_pedal_context_mismatch")
    );
}

#[test]
fn dj_link_waiting_follow_loop_set_starts_once_then_returns_to_normal_stage2_loop_control() {
    let source_timeline_id = TimelineId(9_201);
    let target_timeline_id = TimelineId(9_202);
    let play_session_id = "wait-follow-play-session";
    let source_timeline_id_string = source_timeline_id.0.to_string();
    let target_timeline_id_string = target_timeline_id.0.to_string();
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut source = TimelineSnapshot {
        id: source_timeline_id,
        label: "Wait Follow source".to_string(),
        phases: vec![TimelinePhaseSummary {
            id: protocol::TimelinePhaseId(9_203),
            label: "Wait Follow source body".to_string(),
            role: protocol::TimelinePhaseRole::Verse,
            start_ms: 0,
            end_ms: 50,
        }],
        duration_ms: 50,
        tempo_meter_map: vec![protocol::TimelineTempoMeterPoint {
            position_sixteenth_steps: 0,
            bpm: 300.0,
            numerator: 4,
            denominator: 4,
            interpolation: protocol::TimelineTempoInterpolation::Step,
            measure_number: None,
        }],
        ..TimelineSnapshot::default()
    };
    source.follow = Some(TimelineFollowSummary {
        enabled: true,
        next_timeline_id: target_timeline_id,
        duration: VideoClipTakeDuration::milliseconds(1),
        curve: VideoLayerTransitionCurve::Linear,
        video_kind: VideoClipTakeKind::Crossfade,
        lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
        destination_bpm: Some(300.0),
        preroll_ms: 0,
        trans_cadence_bars: 4,
        trans_target_measures: Vec::new(),
        hold_first_destination_measure: false,
        destination_start_mode: protocol::TimelineFollowDestinationStartMode::WaitForPedal,
        fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
    });
    let target = TimelineSnapshot {
        id: target_timeline_id,
        label: "Wait Follow target".to_string(),
        phases: vec![TimelinePhaseSummary {
            id: protocol::TimelinePhaseId(9_204),
            label: "Wait Follow target body".to_string(),
            role: protocol::TimelinePhaseRole::Verse,
            start_ms: 0,
            end_ms: 10_000,
        }],
        duration_ms: 10_000,
        loop_region: Some(TimelineLoopRegionSummary {
            a_ms: 0,
            b_ms: 1_000,
            enabled: true,
            musical_length_beats: Some(5.0),
        }),
        ..TimelineSnapshot::default()
    };
    engine
        .apply_timeline_bank_published(vec![source, target], source_timeline_id, true)
        .unwrap();

    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut mapping = dj_link_test_mapping(
        "wait-follow-mapping",
        protocol::DjTrackSelector {
            content_id: Some("wait-follow-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = source_timeline_id;
    coordinator.mappings.dj_track_triggers = vec![mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let dispatch = |message_type, sequence, event_id, payload| {
        dispatch_dj_link_event(
            dj_link_test_envelope(message_type, sequence, event_id, payload),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        )
    };

    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TrackActive,
            1,
            "wait-follow-active",
            json!({
                "deck": 1,
                "deckId": "rekordbox-deck-1",
                "contentId": "wait-follow-content",
                "positionAtSendSec": 0.0,
                "effectiveBpm": 300.0,
                "positionRevision": 1,
                "sampleAgeMs": 0,
                "isPlaying": true,
                "startedAt": "2026-08-27T00:00:00Z",
                "playSessionId": play_session_id,
                "loop": null
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::Release,
            2,
            "wait-follow-release",
            json!({
                "state": "released",
                "timelineId": source_timeline_id_string,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));

    let waiting_snapshot = {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let snapshot = engine.snapshot();
            if snapshot.timeline.id == target_timeline_id
                && !snapshot.timeline.playing
                && snapshot.timeline.position_ms == 0
                && snapshot.timeline.follow_runtime.waiting_for_pedal_start
                && matches!(
                    snapshot.timeline.loop_runtime.status,
                    protocol::TimelineLoopRuntimeStatus::Disabled
                )
            {
                break snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "Follow did not settle into the exact pedal wait: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };

    let waiting_state = dispatch(
        protocol::DjLinkMessageType::TimelineStateRequest,
        3,
        "wait-follow-state-request",
        json!({}),
    );
    match waiting_state {
        DjLinkDispatchOutcome::TimelineState { state, .. } => {
            assert_eq!(state.state, protocol::DjLinkTimelineStateValue::Running);
            assert!(!state.loop_active);
            assert!(!state.transition_hold_active);
            assert_eq!(state.timeline_id, target_timeline_id_string);
            assert_eq!(state.play_session_id.as_deref(), Some(play_session_id));
            assert_eq!(state.pedal_owner.as_deref(), Some("timeline"));
            assert_eq!(
                state.release_event_id.as_deref(),
                Some("wait-follow-release")
            );
        }
        outcome => panic!("wait state request should remain in Timeline control: {outcome:?}"),
    }

    let before_wrong_session = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TimelineLoopSet,
            4,
            "wait-follow-wrong-session",
            json!({
                "timelineId": target_timeline_id_string,
                "playSessionId": "wrong-session",
                "active": true
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "timeline_waiting_pedal_context_mismatch"
    ));
    assert_eq!(engine.snapshot(), waiting_snapshot);
    assert_eq!(*runtime.lock().unwrap(), before_wrong_session);

    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TimelineLoopSet,
            5,
            "wait-follow-inactive-edge",
            json!({
                "timelineId": target_timeline_id_string,
                "playSessionId": play_session_id,
                "active": false
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "timeline_waiting_pedal_start_requires_active"
    ));
    assert_eq!(engine.snapshot(), waiting_snapshot);

    let start = dispatch(
        protocol::DjLinkMessageType::TimelineLoopSet,
        6,
        "wait-follow-start",
        json!({
            "timelineId": target_timeline_id_string,
            "playSessionId": play_session_id,
            "active": true
        }),
    );
    let start_generation = match start {
        DjLinkDispatchOutcome::TimelineState {
            state_generation,
            state,
        } => {
            assert_eq!(state.timeline_id, target_timeline_id_string);
            assert!(!state.loop_active);
            state_generation
        }
        outcome => panic!("wait pedal start should be accepted: {outcome:?}"),
    };
    let started_snapshot = engine.snapshot();
    let started_runtime = runtime.lock().unwrap().clone();
    assert!(started_snapshot.timeline.playing);
    assert_eq!(started_snapshot.timeline.position_ms, 0);
    assert!(
        !started_snapshot
            .timeline
            .follow_runtime
            .waiting_for_pedal_start
    );
    assert!(matches!(
        started_snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    ));

    let replay = dispatch(
        protocol::DjLinkMessageType::TimelineLoopSet,
        6,
        "wait-follow-start",
        json!({
            "timelineId": target_timeline_id_string,
            "playSessionId": play_session_id,
            "active": true
        }),
    );
    assert!(matches!(
        replay,
        DjLinkDispatchOutcome::TimelineState { state_generation, .. }
            if state_generation == start_generation
    ));
    assert_eq!(engine.snapshot(), started_snapshot);
    assert_eq!(*runtime.lock().unwrap(), started_runtime);

    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TimelineLoopSet,
            7,
            "wait-follow-normal-loop-on",
            json!({
                "timelineId": target_timeline_id_string,
                "playSessionId": play_session_id,
                "active": true
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert!(runtime.lock().unwrap().loop_active);
    assert!(matches!(
        engine.snapshot().timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Armed | protocol::TimelineLoopRuntimeStatus::Looping
    ));
    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TimelineLoopSet,
            8,
            "wait-follow-normal-loop-off",
            json!({
                "timelineId": target_timeline_id_string,
                "playSessionId": play_session_id,
                "active": false
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert!(!runtime.lock().unwrap().loop_active);
    assert!(matches!(
        engine.snapshot().timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    ));
}

#[test]
fn completed_follow_hold_rebases_before_canonical_loop_off_and_resumes_target() {
    let source_timeline_id = TimelineId(9_101);
    let target_timeline_id = TimelineId(9_102);
    let play_session_id = "follow-hold-play-session";
    let release_event_id = "follow-hold-release";
    let loop_off_event_id = "follow-hold-loop-off";
    let source_timeline_id_string = source_timeline_id.0.to_string();
    let target_timeline_id_string = target_timeline_id.0.to_string();
    // Let RELEASE hand the source clock back to normal playback and cross a
    // real natural boundary.  Seeking to the terminal millisecond is a
    // manual transport operation and correctly aborts Follow.
    let source_duration_ms = 50;

    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut source = TimelineSnapshot {
        id: source_timeline_id,
        label: "Follow hold source".to_string(),
        phases: vec![TimelinePhaseSummary {
            id: protocol::TimelinePhaseId(9_103),
            label: "Follow hold source body".to_string(),
            role: protocol::TimelinePhaseRole::Verse,
            start_ms: 0,
            end_ms: source_duration_ms,
        }],
        duration_ms: source_duration_ms,
        ..TimelineSnapshot::default()
    };
    source.follow = Some(TimelineFollowSummary {
        enabled: true,
        next_timeline_id: target_timeline_id,
        duration: VideoClipTakeDuration::milliseconds(1),
        curve: VideoLayerTransitionCurve::Linear,
        video_kind: VideoClipTakeKind::Crossfade,
        lighting_policy: protocol::TimelineFollowLightingPolicy::HoldThenCut,
        // Keep the retained first-measure hold short enough to prove a real
        // post-boundary F13 re-entry and subsequent wrap without relying on
        // a manual seek (which would correctly abort Follow).
        destination_bpm: Some(300.0),
        preroll_ms: 0,
        trans_cadence_bars: 4,
        trans_target_measures: Vec::new(),
        hold_first_destination_measure: true,
        destination_start_mode: protocol::TimelineFollowDestinationStartMode::Play,
        fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
    });
    let target = TimelineSnapshot {
        id: target_timeline_id,
        label: "Follow hold target".to_string(),
        phases: vec![TimelinePhaseSummary {
            id: protocol::TimelinePhaseId(9_104),
            label: "Follow hold target body".to_string(),
            role: protocol::TimelinePhaseRole::Verse,
            start_ms: 0,
            end_ms: 10_000,
        }],
        duration_ms: 10_000,
        ..TimelineSnapshot::default()
    };
    engine
        .apply_timeline_bank_published(vec![source, target], source_timeline_id, true)
        .unwrap();

    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut mapping = dj_link_test_mapping(
        "follow-hold-mapping",
        protocol::DjTrackSelector {
            content_id: Some("follow-hold-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = source_timeline_id;
    coordinator.mappings.dj_track_triggers = vec![mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let dispatch = |message_type, sequence, event_id, payload| {
        dispatch_dj_link_event(
            dj_link_test_envelope(message_type, sequence, event_id, payload),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        )
    };

    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TrackActive,
            1,
            "follow-hold-active",
            json!({
                "deck": 1,
                "deckId": "rekordbox-deck-1",
                "contentId": "follow-hold-content",
                "positionAtSendSec": 0.0,
                "effectiveBpm": 120.0,
                "positionRevision": 1,
                "sampleAgeMs": 0,
                "isPlaying": true,
                "startedAt": "2026-08-27T00:00:00Z",
                "playSessionId": play_session_id,
                "loop": null
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::Release,
            2,
            release_event_id,
            json!({
                "state": "released",
                "timelineId": source_timeline_id_string,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let released_generation = runtime.lock().unwrap().state_generation;
    assert_eq!(
        runtime.lock().unwrap().timeline_id.as_deref(),
        Some(source_timeline_id_string.as_str())
    );
    assert_eq!(
        runtime.lock().unwrap().release_event_id.as_deref(),
        Some(release_event_id)
    );
    assert_eq!(
        runtime.lock().unwrap().play_session_id.as_deref(),
        Some(play_session_id)
    );

    let target_snapshot = {
        let deadline = Instant::now() + Duration::from_secs(3);
        loop {
            let snapshot = engine.snapshot();
            if snapshot.timeline.id == target_timeline_id
                && snapshot.timeline.playing
                && snapshot.timeline.follow_runtime.status
                    == protocol::TimelineFollowRuntimeStatus::Idle
                && snapshot.timeline.follow_runtime.outcome
                    == Some(protocol::TimelineFollowOutcome::Completed)
                && snapshot.timeline.follow_runtime.transition_hold_active
            {
                break snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "Follow did not settle into the target hold: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    assert_eq!(
        target_snapshot.timeline.follow_runtime.source_timeline_id,
        Some(source_timeline_id)
    );
    assert_eq!(
        target_snapshot.timeline.follow_runtime.target_timeline_id,
        Some(target_timeline_id)
    );
    assert_eq!(
        target_snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Looping
    );
    let retained_hold_bounds = (
        target_snapshot.timeline.loop_runtime.a_ms,
        target_snapshot.timeline.loop_runtime.b_ms,
        target_snapshot
            .timeline
            .loop_runtime
            .musical_length_millibeats,
    );
    assert_eq!(
        runtime.lock().unwrap().timeline_id.as_deref(),
        Some(source_timeline_id_string.as_str())
    );
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(
            &runtime.lock().unwrap(),
            &target_timeline_id_string,
            play_session_id,
            &target_snapshot,
        ),
        Some("timeline_identity_mismatch")
    );

    let loop_off_outcome = dispatch(
        protocol::DjLinkMessageType::TimelineLoopSet,
        3,
        loop_off_event_id,
        json!({
            "timelineId": target_timeline_id_string,
            "playSessionId": play_session_id,
            "active": false
        }),
    );
    let (loop_off_state, loop_off_generation) = match loop_off_outcome {
        DjLinkDispatchOutcome::TimelineState {
            state,
            state_generation,
        } => (state, state_generation),
        outcome => panic!("Follow hold loop-off should be accepted: {outcome:?}"),
    };
    assert_eq!(loop_off_generation, released_generation + 2);
    assert_eq!(loop_off_state.timeline_id, target_timeline_id_string);
    assert_eq!(
        loop_off_state.play_session_id.as_deref(),
        Some(play_session_id)
    );
    assert!(!loop_off_state.loop_active);
    assert!(!loop_off_state.transition_hold_active);

    let after_loop_off = engine.snapshot();
    assert_eq!(after_loop_off.timeline.id, target_timeline_id);
    assert!(after_loop_off.timeline.playing);
    assert!(
        !after_loop_off
            .timeline
            .follow_runtime
            .transition_hold_active
    );
    assert_eq!(
        after_loop_off.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    );
    assert_eq!(
        (
            after_loop_off.timeline.loop_runtime.a_ms,
            after_loop_off.timeline.loop_runtime.b_ms,
            after_loop_off
                .timeline
                .loop_runtime
                .musical_length_millibeats,
        ),
        retained_hold_bounds,
        "F13 OFF retains the first-measure hold bounds for a later F13 ON"
    );
    let runtime_after_loop_off = runtime.lock().unwrap().clone();
    assert_eq!(
        runtime_after_loop_off.timeline_id.as_deref(),
        Some(target_timeline_id_string.as_str())
    );
    assert_eq!(
        runtime_after_loop_off.release_event_id.as_deref(),
        Some(release_event_id)
    );
    assert_eq!(
        runtime_after_loop_off.play_session_id.as_deref(),
        Some(play_session_id)
    );
    assert!(!runtime_after_loop_off.loop_active);
    assert_eq!(
        runtime_after_loop_off.last_event_id.as_deref(),
        Some(loop_off_event_id)
    );
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(
            &runtime_after_loop_off,
            target_timeline_id_string.as_str(),
            play_session_id,
            &after_loop_off,
        ),
        None
    );

    let before_stale_loop_off_engine = engine.snapshot();
    let before_stale_loop_off_runtime = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            protocol::DjLinkMessageType::TimelineLoopSet,
            4,
            "follow-hold-stale-loop-off",
            json!({
                "timelineId": target_timeline_id_string,
                "playSessionId": play_session_id,
                "active": false
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "timeline_loop_state_unchanged"
    ));
    assert_eq!(engine.snapshot(), before_stale_loop_off_engine);
    assert_eq!(*runtime.lock().unwrap(), before_stale_loop_off_runtime);

    let retained_loop_duration_ms = retained_hold_bounds
        .1
        .zip(retained_hold_bounds.0)
        .map(|(b_ms, a_ms)| b_ms - a_ms)
        .expect("completed Follow hold retains valid A-B bounds");
    let advanced_snapshot = {
        let deadline = Instant::now()
            + Duration::from_millis(retained_loop_duration_ms.saturating_mul(3).max(1_000));
        loop {
            let snapshot = engine.snapshot();
            if snapshot.timeline.position_ms >= retained_hold_bounds.1.unwrap() {
                break snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "target Timeline did not advance past retained B after F13 OFF: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    assert_eq!(advanced_snapshot.timeline.id, target_timeline_id);
    assert!(advanced_snapshot.timeline.playing);
    assert!(
        !advanced_snapshot
            .timeline
            .follow_runtime
            .transition_hold_active
    );
    assert_eq!(
        advanced_snapshot.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Disabled
    );

    let loop_on_outcome = dispatch(
        protocol::DjLinkMessageType::TimelineLoopSet,
        5,
        "follow-hold-loop-on",
        json!({
            "timelineId": target_timeline_id_string,
            "playSessionId": play_session_id,
            "active": true
        }),
    );
    assert!(matches!(
        loop_on_outcome,
        DjLinkDispatchOutcome::TimelineState { ref state, .. } if state.loop_active
    ));
    let after_loop_on = engine.snapshot();
    assert_eq!(
        (
            after_loop_on.timeline.loop_runtime.a_ms,
            after_loop_on.timeline.loop_runtime.b_ms,
            after_loop_on
                .timeline
                .loop_runtime
                .musical_length_millibeats,
        ),
        retained_hold_bounds
    );
    assert_eq!(
        after_loop_on.timeline.position_ms,
        retained_hold_bounds.0.unwrap(),
        "F13 ON after B must re-enter the retained first-measure hold at A"
    );
    assert_eq!(
        after_loop_on.timeline.loop_runtime.status,
        protocol::TimelineLoopRuntimeStatus::Looping
    );
    let wraps_before_reentry_cycle = after_loop_on.timeline.loop_runtime.wrap_count;
    let wrapped_snapshot = {
        let deadline = Instant::now()
            + Duration::from_millis(retained_loop_duration_ms.saturating_mul(3).max(1_000));
        loop {
            let snapshot = engine.snapshot();
            if snapshot.timeline.loop_runtime.wrap_count > wraps_before_reentry_cycle {
                break snapshot;
            }
            assert!(
                Instant::now() < deadline,
                "F13 re-entered hold did not wrap at retained B: {snapshot:?}"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
    };
    assert!(
        wrapped_snapshot.timeline.position_ms >= retained_hold_bounds.0.unwrap()
            && wrapped_snapshot.timeline.position_ms < retained_hold_bounds.1.unwrap(),
        "post-wrap playhead must remain inside the re-entered A-B loop"
    );
}

#[test]
fn dj_link_any_deck_track_owner_is_exact_deduped_and_loop_correlated() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut initial_snapshot = engine.snapshot();
    initial_snapshot.timeline.phases = vec![TimelinePhaseSummary {
        id: protocol::TimelinePhaseId(1),
        label: "DJ Link any-deck test phase".to_string(),
        role: protocol::TimelinePhaseRole::Intro,
        start_ms: 0,
        end_ms: 16_000,
    }];
    initial_snapshot.timeline.loop_region = Some(TimelineLoopRegionSummary {
        a_ms: 0,
        b_ms: 16_000,
        enabled: false,
        musical_length_beats: None,
    });
    engine
        .apply_timeline_bank_published(
            vec![initial_snapshot.timeline.clone()],
            initial_snapshot.timeline.id,
            false,
        )
        .unwrap();
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut mapping = dj_link_test_mapping(
        "any-deck-content",
        protocol::DjTrackSelector {
            content_id: Some("any-deck-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = engine.snapshot().timeline.id;
    let mut alternate_mapping = dj_link_test_mapping(
        "any-deck-alternate-content",
        protocol::DjTrackSelector {
            content_id: Some("any-deck-alternate-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    alternate_mapping.timeline_id = mapping.timeline_id;
    coordinator.mappings.dj_track_triggers = vec![mapping, alternate_mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let active_payload = json!({
        "deck": 2, "deckId": "rekordbox-deck-2", "contentId": "any-deck-content",
        "positionAtSendSec": 1.0, "effectiveBpm": 120.0, "positionRevision": 1,
        "sampleAgeMs": 0, "isPlaying": true, "startedAt": "2026-08-26T00:00:00Z",
        "playSessionId": "same-session", "loop": null
    });
    let engine_before_reconnect_state = engine.snapshot();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::StateSync,
                1,
                "restart-owner-diagnostic",
                json!({
                    "released": false,
                    "ownerDeck": 2,
                    "ownerDeckId": "rekordbox-deck-2",
                    "activePlaySessionId": "same-session"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    assert_eq!(engine.snapshot(), engine_before_reconnect_state);
    assert!(runtime.lock().unwrap().play_session_id.is_none());
    let active_outcome = dispatch_dj_link_event(
        dj_link_test_envelope(
            protocol::DjLinkMessageType::TrackActive,
            1,
            "any-deck-active",
            active_payload.clone(),
        ),
        &engine,
        &coordinator,
        &runtime,
        &admission,
        &transaction_active,
    );
    assert!(
        matches!(active_outcome, DjLinkDispatchOutcome::TimelineState { .. }),
        "generic mapped ACTIVE must start the authored timeline: {active_outcome:?}"
    );
    let owner_before_foreign = runtime.lock().unwrap().clone();
    let engine_before_foreign = engine.snapshot();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                2,
                "any-deck-duplicate",
                active_payload.clone(),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    assert_eq!(engine.snapshot(), engine_before_foreign);
    assert_eq!(*runtime.lock().unwrap(), owner_before_foreign);

    let mut unmapped_active = active_payload.clone();
    unmapped_active["deck"] = json!(3);
    unmapped_active["deckId"] = json!("rekordbox-deck-3");
    unmapped_active["contentId"] = json!("unmapped-content");
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                3,
                "foreign-unmapped",
                unmapped_active,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::NoMapping { .. }
    ));
    assert_eq!(engine.snapshot(), engine_before_foreign);
    assert_eq!(*runtime.lock().unwrap(), owner_before_foreign);

    let mut foreign_active = active_payload.clone();
    foreign_active["deck"] = json!(3);
    foreign_active["deckId"] = json!("rekordbox-deck-3");
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                4,
                "foreign-same-session",
                foreign_active,
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "track_owner_unreleased"
    ));
    assert_eq!(engine.snapshot(), engine_before_foreign);
    assert_eq!(*runtime.lock().unwrap(), owner_before_foreign);

    let mut conflicting_sync = active_payload.clone();
    conflicting_sync["contentId"] = json!("other-content");
    conflicting_sync["positionRevision"] = json!(2);
    let engine_before_conflict = engine.snapshot();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackSync,
                5,
                "same-session-cross-identity",
                conflicting_sync,
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "track_sync_context_mismatch"
    ));
    assert_eq!(engine.snapshot(), engine_before_conflict);

    let mut newer_sync = active_payload.clone();
    newer_sync["positionAtSendSec"] = json!(2.0);
    newer_sync["positionRevision"] = json!(2);
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackSync,
                6,
                "same-session-newer-position",
                newer_sync.clone(),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let engine_after_newer_sync = engine.snapshot();
    let runtime_after_newer_sync = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackSync,
                7,
                "same-session-stale-position",
                active_payload.clone(),
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "stale_position_revision"
    ));
    let mut conflicting_position = newer_sync;
    conflicting_position["positionAtSendSec"] = json!(2.1);
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackSync,
                8,
                "same-session-conflicting-position",
                conflicting_position,
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "position_revision_conflict"
    ));
    assert_eq!(engine.snapshot(), engine_after_newer_sync);
    assert_eq!(*runtime.lock().unwrap(), runtime_after_newer_sync);

    let inactive_loop = json!({
        "deck": 2, "deckId": "rekordbox-deck-2", "playSessionId": "same-session",
        "loop": { "active": false, "startBeat": null, "endBeat": null,
            "lengthBeats": null, "revision": 1, "sampleAgeMs": 0,
            "source": "rekordbox-hook-measured" }
    });
    let mut foreign_loop = inactive_loop.clone();
    foreign_loop["deck"] = json!(3);
    foreign_loop["deckId"] = json!("rekordbox-deck-3");
    let engine_before_foreign_loop = engine.snapshot();
    let runtime_before_foreign_loop = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                9,
                "foreign-loop",
                foreign_loop,
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "track_loop_context_mismatch"
    ));
    assert_eq!(engine.snapshot(), engine_before_foreign_loop);
    assert_eq!(*runtime.lock().unwrap(), runtime_before_foreign_loop);
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                9,
                "inactive-base",
                inactive_loop.clone()
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    let fallback = json!({
        "deck": 2, "deckId": "rekordbox-deck-2", "playSessionId": "same-session",
        "pedalIntentId": 1, "baseMeasuredLoopRevision": 1, "baseLoopDivision": null,
        "targetLengthBeats": 8.0, "responseWindowMs": 50,
        "source": "pedal-no-response-predicted"
    });
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopFallback,
                10,
                "first-fallback",
                fallback
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    {
        let runtime = runtime.lock().unwrap();
        assert_eq!(runtime.loop_revision, Some(1));
        assert_eq!(runtime.loop_division, Some(0));
    }
    let newer_loop = json!({
        "deck": 2, "deckId": "rekordbox-deck-2", "playSessionId": "same-session",
        "loop": { "active": true, "startBeat": 0.0, "endBeat": 8.0,
            "lengthBeats": 8.0, "revision": 2, "sampleAgeMs": 0,
            "source": "rekordbox-hook-measured" }
    });
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                11,
                "newer-loop",
                newer_loop,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    let stale_loop = inactive_loop;
    let engine_before_stale_loop = engine.snapshot();
    let runtime_before_stale_loop = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                12,
                "stale-loop",
                stale_loop,
            ),
            &engine, &coordinator, &runtime, &admission, &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "stale_loop_revision"
    ));
    assert_eq!(engine.snapshot(), engine_before_stale_loop);
    assert_eq!(*runtime.lock().unwrap(), runtime_before_stale_loop);

    let timeline_id = {
        let runtime = runtime.lock().unwrap();
        assert_eq!(runtime.track_deck_id.as_deref(), Some("rekordbox-deck-2"));
        assert_eq!(runtime.track_deck_number, Some(2));
        assert_eq!(runtime.loop_revision, Some(2));
        assert_eq!(runtime.loop_division, Some(0));
        runtime.timeline_id.clone().unwrap()
    };
    let wrong_release = json!({
        "state": "released", "timelineId": timeline_id, "playSessionId": "foreign-session"
    });
    let engine_before_wrong_release = engine.snapshot();
    let wrong_release_outcome = dispatch_dj_link_event(
        dj_link_test_envelope(
            protocol::DjLinkMessageType::Release,
            13,
            "foreign-release",
            wrong_release,
        ),
        &engine,
        &coordinator,
        &runtime,
        &admission,
        &transaction_active,
    );
    assert!(
        matches!(
            wrong_release_outcome,
            DjLinkDispatchOutcome::Rejected { ref code, .. }
                if code == "release_context_mismatch"
        ),
        "unexpected wrong-release outcome: {wrong_release_outcome:?}"
    );
    assert_eq!(engine.snapshot(), engine_before_wrong_release);
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::Release,
                14,
                "owner-release",
                json!({
                    "state": "released", "timelineId": timeline_id,
                    "playSessionId": "same-session"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert!(runtime.lock().unwrap().released);
    engine
        .send(EngineCommand::SetTimelinePlaying(false))
        .unwrap();
    std::thread::sleep(Duration::from_millis(25));

    // Simulate expiry of the bounded replay cache. Release authority and
    // its exact owner/identity/mapping receipt must remain terminal even
    // when the auxiliary seen-session entry is no longer retained.
    runtime.lock().unwrap().seen_play_sessions.clear();

    let engine_before_reannounce = engine.snapshot();
    let runtime_before_reannounce = runtime.lock().unwrap().clone();
    let mut exact_reannounce = active_payload.clone();
    exact_reannounce["positionAtSendSec"] = json!(3.0);
    exact_reannounce["positionRevision"] = json!(3);
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                15,
                "released-exact-reannounce",
                exact_reannounce,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    assert_eq!(
        engine.snapshot(),
        engine_before_reannounce,
        "a released generic owner's exact identity and mapping reannounce must not restart"
    );
    assert_eq!(
        *runtime.lock().unwrap(),
        runtime_before_reannounce,
        "a released generic owner's exact identity and mapping reannounce must be a full no-op"
    );

    for (event_id, content_id) in [
        ("released-unmapped-identity", "unmapped-after-release"),
        ("released-alternate-mapping", "any-deck-alternate-content"),
    ] {
        let engine_before_mismatch = engine.snapshot();
        let runtime_before_mismatch = runtime.lock().unwrap().clone();
        let mut mismatch = active_payload.clone();
        mismatch["contentId"] = json!(content_id);
        mismatch["positionAtSendSec"] = json!(4.0);
        mismatch["positionRevision"] = json!(4);
        assert!(matches!(
            dispatch_dj_link_event(
                dj_link_test_envelope(
                    protocol::DjLinkMessageType::TrackActive,
                    16,
                    event_id,
                    mismatch,
                ),
                &engine,
                &coordinator,
                &runtime,
                &admission,
                &transaction_active,
            ),
            DjLinkDispatchOutcome::Rejected { ref code, .. }
                if code == "released_track_reannounce_mismatch"
        ));
        assert_eq!(
            engine.snapshot(),
            engine_before_mismatch,
            "a released generic identity or mapping mismatch must not reach the engine"
        );
        assert_eq!(
            *runtime.lock().unwrap(),
            runtime_before_mismatch,
            "a released generic identity or mapping mismatch must preserve runtime authority"
        );
    }
}

#[test]
fn dj_link_any_deck_fallback_profile_saturates_and_rebases_after_measured_authority() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut initial_snapshot = engine.snapshot();
    initial_snapshot.timeline.phases = vec![TimelinePhaseSummary {
        id: protocol::TimelinePhaseId(1),
        label: "DJ Link generic fallback profile test".to_string(),
        role: protocol::TimelinePhaseRole::Intro,
        start_ms: 0,
        end_ms: 16_000,
    }];
    initial_snapshot.timeline.loop_region = Some(TimelineLoopRegionSummary {
        a_ms: 0,
        b_ms: 16_000,
        enabled: false,
        musical_length_beats: None,
    });
    engine
        .apply_timeline_bank_published(
            vec![initial_snapshot.timeline.clone()],
            initial_snapshot.timeline.id,
            false,
        )
        .unwrap();
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut mapping = dj_link_test_mapping(
        "generic-fallback-profile",
        protocol::DjTrackSelector {
            content_id: Some("generic-fallback-profile-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = engine.snapshot().timeline.id;
    coordinator.mappings.dj_track_triggers = vec![mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let active_payload = json!({
        "deck": 2,
        "deckId": "rekordbox-deck-2",
        "contentId": "generic-fallback-profile-content",
        "positionAtSendSec": 0.0,
        "effectiveBpm": 120.0,
        "positionRevision": 1,
        "sampleAgeMs": 0,
        "isPlaying": true,
        "startedAt": "2026-08-26T00:00:00Z",
        "playSessionId": "generic-fallback-profile-session",
        "loop": null
    });
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                1,
                "generic-fallback-profile-active",
                active_payload,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    engine
        .send(EngineCommand::SetTimelinePlaying(false))
        .unwrap();
    std::thread::sleep(Duration::from_millis(25));

    let inactive_measurement = json!({
        "deck": 2,
        "deckId": "rekordbox-deck-2",
        "playSessionId": "generic-fallback-profile-session",
        "loop": {
            "active": false,
            "startBeat": null,
            "endBeat": null,
            "lengthBeats": null,
            "revision": 1,
            "sampleAgeMs": 0,
            "source": "rekordbox-hook-measured"
        }
    });
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                2,
                "generic-fallback-profile-inactive",
                inactive_measurement,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    {
        let runtime = runtime.lock().unwrap();
        assert_eq!(
            (
                runtime.loop_revision,
                runtime.loop_division,
                runtime.loop_active
            ),
            (Some(1), None, false),
            "the initial inactive measurement must establish the first fallback base"
        );
    }

    for (index, target_length_beats) in protocol::DJ_LINK_LOOP_PROFILE_LENGTH_BEATS
        .into_iter()
        .enumerate()
    {
        let engine_generation_before = engine.snapshot().timeline.loop_runtime.generation;
        let outcome = dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopFallback,
                u64::try_from(index + 3).unwrap(),
                &format!("generic-fallback-profile-{index}"),
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "playSessionId": "generic-fallback-profile-session",
                    "pedalIntentId": u64::try_from(index + 1).unwrap(),
                    "baseMeasuredLoopRevision": 1,
                    "baseLoopDivision": if index == 0 { Value::Null } else { json!(index - 1) },
                    "targetLengthBeats": target_length_beats,
                    "responseWindowMs": 50,
                    "source": "pedal-no-response-predicted"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        );
        assert!(
            matches!(outcome, DjLinkDispatchOutcome::Accepted { .. }),
            "generic fallback profile index {index} must be accepted: {outcome:?}"
        );
        assert!(
            engine.snapshot().timeline.loop_runtime.generation > engine_generation_before,
            "every non-saturated generic fallback profile step must reach the engine"
        );
        let runtime = runtime.lock().unwrap();
        assert_eq!(
            (
                runtime.loop_revision,
                runtime.loop_division,
                runtime.loop_active
            ),
            (Some(1), Some(index as u8), true),
            "generic fallback profile index {index} must establish its exact division"
        );
    }

    let engine_at_floor = engine.snapshot();
    let runtime_at_floor = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopFallback,
                13,
                "generic-fallback-profile-saturated",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "playSessionId": "generic-fallback-profile-session",
                    "pedalIntentId": 11,
                    "baseMeasuredLoopRevision": 1,
                    "baseLoopDivision": 9,
                    "targetLengthBeats": 1.0 / 64.0,
                    "responseWindowMs": 50,
                    "source": "pedal-no-response-predicted"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { state_generation }
            if state_generation == runtime_at_floor.state_generation + 1
    ));
    assert_eq!(
        engine.snapshot(),
        engine_at_floor,
        "the saturated generic 1/64 fallback must consume its intent without an engine mutation"
    );
    assert_eq!(
        runtime.lock().unwrap().last_loop_fallback_intent_id,
        Some(11)
    );

    let engine_before_fresh_measurement = engine.snapshot();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopState,
                14,
                "generic-fallback-profile-fresh-measurement",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "playSessionId": "generic-fallback-profile-session",
                    "loop": {
                        "active": true,
                        "startBeat": 0.0,
                        "endBeat": 2.0,
                        "lengthBeats": 2.0,
                        "revision": 2,
                        "sampleAgeMs": 0,
                        "source": "rekordbox-hook-measured"
                    }
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    assert!(
        engine.snapshot().timeline.loop_runtime.generation
            > engine_before_fresh_measurement
                .timeline
                .loop_runtime
                .generation,
        "a newer measured loop must rebase the generic fallback authority"
    );
    {
        let runtime = runtime.lock().unwrap();
        assert_eq!(
            (
                runtime.loop_revision,
                runtime.loop_division,
                runtime.loop_active
            ),
            (Some(2), Some(2), true)
        );
    }

    let engine_after_fresh_measurement = engine.snapshot();
    let runtime_after_fresh_measurement = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopFallback,
                15,
                "generic-fallback-profile-stale-delayed",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "playSessionId": "generic-fallback-profile-session",
                    "pedalIntentId": 12,
                    "baseMeasuredLoopRevision": 1,
                    "baseLoopDivision": 9,
                    "targetLengthBeats": 1.0 / 64.0,
                    "responseWindowMs": 50,
                    "source": "pedal-no-response-predicted"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "loop_fallback_base_revision_mismatch"
    ));
    assert_eq!(engine.snapshot(), engine_after_fresh_measurement);
    assert_eq!(*runtime.lock().unwrap(), runtime_after_fresh_measurement);

    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::LoopFallback,
                16,
                "generic-fallback-profile-rebased",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "playSessionId": "generic-fallback-profile-session",
                    "pedalIntentId": 12,
                    "baseMeasuredLoopRevision": 2,
                    "baseLoopDivision": 2,
                    "targetLengthBeats": 1.0,
                    "responseWindowMs": 50,
                    "source": "pedal-no-response-predicted"
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Accepted { .. }
    ));
    {
        let runtime = runtime.lock().unwrap();
        assert_eq!(
            (
                runtime.loop_revision,
                runtime.loop_division,
                runtime.loop_active
            ),
            (Some(2), Some(3), true),
            "only a fallback based on the newer measurement may advance the generic profile"
        );
        assert_eq!(runtime.last_loop_fallback_intent_id, Some(12));
    }
}

#[test]
fn dj_link_generic_active_capacity_is_fail_closed_without_mutation() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let mut mapping = dj_link_test_mapping(
        "capacity-generic",
        protocol::DjTrackSelector {
            content_id: Some("capacity-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = engine.snapshot().timeline.id;
    coordinator.mappings.dj_track_triggers = vec![mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    {
        let mut runtime = runtime.lock().unwrap();
        for index in 0..DJ_LINK_DEDUPE_LIMIT {
            runtime
                .seen_play_sessions
                .insert(format!("retained-{index}"), Instant::now());
        }
    }
    let engine_before = engine.snapshot();
    let runtime_before = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                1,
                "generic-capacity-full",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "contentId": "capacity-content",
                    "positionAtSendSec": 0.0,
                    "effectiveBpm": 120.0,
                    "positionRevision": 1,
                    "sampleAgeMs": 0,
                    "isPlaying": true,
                    "startedAt": "2026-08-27T00:00:00Z",
                    "playSessionId": "capacity-session",
                    "loop": null
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "play_session_capacity"
    ));
    assert_eq!(engine.snapshot(), engine_before);
    assert_eq!(*runtime.lock().unwrap(), runtime_before);
}

#[test]
fn dj_link_generic_unmapped_active_latches_all_mutating_followups() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let coordinator = Mutex::new(project_coordinator_for_initial_snapshot(engine.snapshot()));
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(
        &coordinator.lock().unwrap(),
    ));
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let unmapped_active = json!({
        "deck": 2,
        "deckId": "rekordbox-deck-2",
        "contentId": "unmapped-content",
        "positionAtSendSec": 0.0,
        "effectiveBpm": 120.0,
        "positionRevision": 1,
        "sampleAgeMs": 0,
        "isPlaying": true,
        "startedAt": "2026-08-27T00:00:00Z",
        "playSessionId": "unmapped-session",
        "loop": null
    });
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                1,
                "generic-unmapped-active",
                unmapped_active,
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::NoMapping { .. }
    ));
    assert!(runtime.lock().unwrap().unmapped_active_blocked);
    let engine_before = engine.snapshot();
    let runtime_before = runtime.lock().unwrap().clone();
    let cases = [
        (
            protocol::DjLinkMessageType::StateSync,
            "generic-unmapped-state-sync",
            json!({ "released": false }),
        ),
        (
            protocol::DjLinkMessageType::TrackSync,
            "generic-unmapped-sync",
            json!({
                "deck": 2, "deckId": "rekordbox-deck-2", "contentId": "unmapped-content",
                "positionAtSendSec": 1.0, "effectiveBpm": 120.0, "positionRevision": 2,
                "sampleAgeMs": 0, "isPlaying": true, "startedAt": "2026-08-27T00:00:01Z",
                "playSessionId": "unmapped-session", "loop": null
            }),
        ),
        (
            protocol::DjLinkMessageType::LoopState,
            "generic-unmapped-loop",
            json!({
                "deck": 2, "deckId": "rekordbox-deck-2", "playSessionId": "unmapped-session",
                "loop": { "active": false, "startBeat": null, "endBeat": null,
                    "lengthBeats": null, "revision": 1, "sampleAgeMs": 0,
                    "source": "rekordbox-hook-measured" }
            }),
        ),
        (
            protocol::DjLinkMessageType::LoopFallback,
            "generic-unmapped-fallback",
            json!({
                "deck": 2, "deckId": "rekordbox-deck-2", "playSessionId": "unmapped-session",
                "pedalIntentId": 1, "baseMeasuredLoopRevision": null, "baseLoopDivision": null,
                "targetLengthBeats": 8.0, "responseWindowMs": 50,
                "source": "pedal-no-response-predicted"
            }),
        ),
        (
            protocol::DjLinkMessageType::Release,
            "generic-unmapped-release",
            json!({
                "state": "released", "timelineId": "1", "playSessionId": "unmapped-session"
            }),
        ),
    ];
    for (offset, (message_type, event_id, payload)) in cases.into_iter().enumerate() {
        assert!(matches!(
            dispatch_dj_link_event(
                dj_link_test_envelope(message_type, u64::try_from(offset + 2).unwrap(), event_id, payload),
                &engine,
                &coordinator,
                &runtime,
                &admission,
                &transaction_active,
            ),
            DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "dj_link_unmapped_active"
        ));
        assert_eq!(engine.snapshot(), engine_before);
        assert_eq!(*runtime.lock().unwrap(), runtime_before);
    }
}

#[test]
fn dj_link_generic_engine_rejection_preserves_runtime_authority_atomically() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let initial_snapshot = wait_for_test_engine_startup(&engine);
    let mut coordinator = project_coordinator_for_initial_snapshot(initial_snapshot);
    coordinator.mappings.dj_track_triggers = vec![dj_link_test_mapping(
        "engine-rejection-generic",
        protocol::DjTrackSelector {
            content_id: Some("engine-rejection-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    )];
    coordinator.mappings.dj_track_triggers[0].timeline_id = TimelineId(u64::MAX);
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let engine_before = engine.snapshot();
    let runtime_before = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch_dj_link_event(
            dj_link_test_envelope(
                protocol::DjLinkMessageType::TrackActive,
                1,
                "generic-engine-rejection",
                json!({
                    "deck": 2,
                    "deckId": "rekordbox-deck-2",
                    "contentId": "engine-rejection-content",
                    "positionAtSendSec": 0.0,
                    "effectiveBpm": 120.0,
                    "positionRevision": 1,
                    "sampleAgeMs": 0,
                    "isPlaying": true,
                    "startedAt": "2026-08-27T00:00:00Z",
                    "playSessionId": "engine-rejection-session",
                    "loop": null
                }),
            ),
            &engine,
            &coordinator,
            &runtime,
            &admission,
            &transaction_active,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "engine_publication_rejected"
    ));
    assert_eq!(engine.snapshot(), engine_before);
    assert_eq!(*runtime.lock().unwrap(), runtime_before);
}

#[test]
fn dj_link_stage2_commands_require_release_authority_and_exact_correlations() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut initial_snapshot = engine.snapshot();
    initial_snapshot.timeline.phases = vec![TimelinePhaseSummary {
        id: protocol::TimelinePhaseId(1),
        label: "DJ Link Stage 2 authority test phase".to_string(),
        role: protocol::TimelinePhaseRole::Intro,
        start_ms: 0,
        end_ms: 16_000,
    }];
    initial_snapshot.timeline.loop_region = Some(TimelineLoopRegionSummary {
        a_ms: 0,
        b_ms: 16_000,
        enabled: false,
        musical_length_beats: None,
    });
    engine
        .apply_timeline_bank_published(
            vec![initial_snapshot.timeline.clone()],
            initial_snapshot.timeline.id,
            false,
        )
        .unwrap();
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
    let timeline_id = engine.snapshot().timeline.id.0.to_string();
    let play_session_id = "stage2-play-session";
    let mut mapping = dj_link_test_mapping(
        "stage2-mapping",
        protocol::DjTrackSelector {
            content_id: Some("stage2-content".to_string()),
            title: None,
            artist: None,
            title_contains: None,
            fallback_deck: None,
        },
    );
    mapping.timeline_id = engine.snapshot().timeline.id;
    coordinator.mappings.dj_track_triggers = vec![mapping];
    let runtime = Mutex::new(DjLinkRuntime::from_coordinator(&coordinator));
    let coordinator = Mutex::new(coordinator);
    let admission = ProjectExternalCommandAdmission::default();
    let transaction_active = AtomicBool::new(false);
    let active_payload = json!({
        "deck": 2,
        "deckId": "rekordbox-deck-2",
        "contentId": "stage2-content",
        "positionAtSendSec": 1.0,
        "effectiveBpm": 120.0,
        "positionRevision": 1,
        "sampleAgeMs": 0,
        "isPlaying": true,
        "startedAt": "2026-08-27T00:00:00Z",
        "playSessionId": play_session_id,
        "loop": null
    });
    let stage2_payload = |timeline_id: &str, play_session_id: &str| {
        json!({
            "timelineId": timeline_id,
            "playSessionId": play_session_id,
            "bars": 4
        })
    };
    let stage2_command_payload =
        |message_type: protocol::DjLinkMessageType, timeline_id: &str, play_session_id: &str| {
            match message_type {
                protocol::DjLinkMessageType::TimelineBeatJump => {
                    stage2_payload(timeline_id, play_session_id)
                }
                protocol::DjLinkMessageType::TimelineLoopSet => json!({
                    "timelineId": timeline_id,
                    "playSessionId": play_session_id,
                    "active": true
                }),
                protocol::DjLinkMessageType::TimelineLoopHalf => json!({
                    "timelineId": timeline_id,
                    "playSessionId": play_session_id,
                }),
                _ => unreachable!("not a Stage 2 command"),
            }
        };
    let stage2_command_types = [
        protocol::DjLinkMessageType::TimelineBeatJump,
        protocol::DjLinkMessageType::TimelineLoopSet,
        protocol::DjLinkMessageType::TimelineLoopHalf,
    ];
    let dispatch = |runtime: &Mutex<DjLinkRuntime>, message_type, sequence, event_id, payload| {
        dispatch_dj_link_event(
            dj_link_test_envelope(message_type, sequence, event_id, payload),
            &engine,
            &coordinator,
            runtime,
            &admission,
            &transaction_active,
        )
    };

    assert!(matches!(
        dispatch(
            &runtime,
            protocol::DjLinkMessageType::TrackActive,
            1,
            "stage2-active",
            active_payload,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let active_runtime = runtime.lock().unwrap().clone();
    assert!(!active_runtime.released);
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(
            &active_runtime,
            &timeline_id,
            play_session_id,
            &engine.snapshot(),
        ),
        Some("timeline_stage2_not_authorized")
    );
    for (sequence, message_type, payload) in [
        (
            2,
            protocol::DjLinkMessageType::TimelineBeatJump,
            stage2_payload(&timeline_id, play_session_id),
        ),
        (
            3,
            protocol::DjLinkMessageType::TimelineLoopSet,
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id,
                "active": true
            }),
        ),
        (
            4,
            protocol::DjLinkMessageType::TimelineLoopHalf,
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id,
            }),
        ),
    ] {
        let before = engine.snapshot();
        assert!(matches!(
            dispatch(
                &runtime,
                message_type,
                sequence,
                "stage2-pre-release",
                payload,
            ),
            DjLinkDispatchOutcome::Rejected { ref code, .. }
                if code == "timeline_stage2_not_authorized"
        ));
        assert_eq!(engine.snapshot(), before);
    }

    // RELEASE itself is admitted only from the exact active DJ-owned
    // running session.  These boundary failures must not enqueue an engine
    // command or mutate the candidate runtime projection.
    let release_rejection_cases: [(&str, fn(&mut DjLinkRuntime), &str, &str); 3] = [
        (
            "inactive-track",
            |candidate: &mut DjLinkRuntime| candidate.track_active = false,
            timeline_id.as_str(),
            "timeline_not_active",
        ),
        (
            "non-running",
            |candidate: &mut DjLinkRuntime| {
                candidate.authoritative_state = protocol::DjLinkTimelineStateValue::Idle
            },
            timeline_id.as_str(),
            "timeline_state_not_running",
        ),
        (
            "wrong-owner",
            |candidate: &mut DjLinkRuntime| candidate.pedal_owner = Some("timeline".to_string()),
            timeline_id.as_str(),
            "dj_link_pedal_owner_mismatch",
        ),
    ];
    for (case_name, mutate, release_timeline_id, expected_code) in release_rejection_cases {
        let mut candidate_runtime = active_runtime.clone();
        mutate(&mut candidate_runtime);
        let candidate = Mutex::new(candidate_runtime);
        let before_engine = engine.snapshot();
        let before_runtime = candidate.lock().unwrap().clone();
        let release_event_id = match case_name {
            "inactive-track" => "stage2-release-inactive",
            "non-running" => "stage2-release-non-running",
            "wrong-owner" => "stage2-release-wrong-owner",
            _ => unreachable!("unknown release rejection case"),
        };
        assert!(matches!(
            dispatch(
                &candidate,
                protocol::DjLinkMessageType::Release,
                4,
                &release_event_id,
                json!({
                    "state": "released",
                    "timelineId": release_timeline_id,
                    "playSessionId": play_session_id
                }),
            ),
            DjLinkDispatchOutcome::Rejected { ref code, .. } if code == expected_code
        ));
        assert_eq!(
            engine.snapshot(),
            before_engine,
            "{case_name} engine mutation"
        );
        assert_eq!(
            *candidate.lock().unwrap(),
            before_runtime,
            "{case_name} runtime mutation"
        );
    }
    for (case_name, release_timeline_id, release_play_session_id) in [
        ("wrong-timeline", "999999", play_session_id),
        ("wrong-session", timeline_id.as_str(), "other-play-session"),
    ] {
        let candidate = Mutex::new(active_runtime.clone());
        let before_engine = engine.snapshot();
        let before_runtime = candidate.lock().unwrap().clone();
        let release_event_id = match case_name {
            "wrong-timeline" => "stage2-release-wrong-timeline",
            "wrong-session" => "stage2-release-wrong-session",
            _ => unreachable!("unknown release rejection case"),
        };
        let outcome = dispatch(
            &candidate,
            protocol::DjLinkMessageType::Release,
            4,
            &release_event_id,
            json!({
                "state": "released",
                "timelineId": release_timeline_id,
                "playSessionId": release_play_session_id
            }),
        );
        assert!(
            matches!(
                outcome,
                DjLinkDispatchOutcome::Rejected { ref code, .. }
                    if code == "release_context_mismatch"
            ),
            "{case_name} outcome: {outcome:?}"
        );
        assert_eq!(
            engine.snapshot(),
            before_engine,
            "{case_name} engine mutation"
        );
        assert_eq!(
            *candidate.lock().unwrap(),
            before_runtime,
            "{case_name} runtime mutation"
        );
    }

    assert!(matches!(
        dispatch(
            &runtime,
            protocol::DjLinkMessageType::Release,
            4,
            "stage2-release",
            json!({
                "state": "released",
                "timelineId": timeline_id,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let released_runtime = runtime.lock().unwrap().clone();
    assert!(released_runtime.released);
    assert_eq!(
        released_runtime.authoritative_state,
        protocol::DjLinkTimelineStateValue::Running
    );
    assert_eq!(released_runtime.pedal_owner.as_deref(), Some("timeline"));
    assert_eq!(
        released_runtime.release_event_id.as_deref(),
        Some("stage2-release")
    );
    assert_eq!(
        dj_track_runtime::stage2_authority_rejection(
            &released_runtime,
            &timeline_id,
            play_session_id,
            &engine.snapshot(),
        ),
        None
    );

    let before_replay = engine.snapshot();
    let before_replay_runtime = released_runtime.clone();
    assert!(matches!(
        dispatch(
            &runtime,
            protocol::DjLinkMessageType::Release,
            5,
            "stage2-release-replay",
            json!({
                "state": "released",
                "timelineId": timeline_id,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    assert_eq!(engine.snapshot(), before_replay);
    assert_eq!(
        runtime.lock().unwrap().release_event_id,
        before_replay_runtime.release_event_id
    );

    let mut invalid_replay_runtime = released_runtime.clone();
    invalid_replay_runtime.loop_active = true;
    let invalid_replay_runtime = Mutex::new(invalid_replay_runtime);
    let before_invalid_replay_engine = engine.snapshot();
    assert!(matches!(
        dispatch(
            &invalid_replay_runtime,
            protocol::DjLinkMessageType::Release,
            6,
            "stage2-release-invalid-replay",
            json!({
                "state": "released",
                "timelineId": timeline_id,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "timeline_release_state_invalid"
    ));
    assert_eq!(engine.snapshot(), before_invalid_replay_engine);

    let released_generation = released_runtime.state_generation;
    let before_jump = engine.snapshot();
    let beat_jump_generation = match dispatch(
        &runtime,
        protocol::DjLinkMessageType::TimelineBeatJump,
        6,
        "stage2-beat-jump",
        stage2_payload(&timeline_id, play_session_id),
    ) {
        DjLinkDispatchOutcome::TimelineState {
            state_generation, ..
        } => state_generation,
        outcome => panic!("Stage 2 beat jump should be accepted after release: {outcome:?}"),
    };
    assert_eq!(beat_jump_generation, released_generation + 1);
    assert_eq!(
        runtime.lock().unwrap().state_generation,
        beat_jump_generation
    );
    assert_ne!(
        engine.snapshot().timeline.position_ms,
        before_jump.timeline.position_ms
    );

    // The retired -4 command is rejected before engine admission, so neither
    // clock nor runtime authority can mutate after a valid release.
    let before_engine_rejection = engine.snapshot();
    let before_runtime_rejection = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            &runtime,
            protocol::DjLinkMessageType::TimelineBeatJump,
            7,
            "stage2-retired-minus-four",
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id,
                "bars": -4
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "invalid_timeline_beat_jump_payload"
    ));
    assert_eq!(engine.snapshot(), before_engine_rejection);
    assert_eq!(*runtime.lock().unwrap(), before_runtime_rejection);

    // F14 is not a generic scale request: before F13 activates a current
    // runtime loop, it must fail at the app/engine active-state fence without
    // publishing either projection.
    let invalid_loop_runtime = Mutex::new(released_runtime.clone());
    let before_engine_loop_rejection = engine.snapshot();
    let before_runtime_loop_rejection = invalid_loop_runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            &invalid_loop_runtime,
            protocol::DjLinkMessageType::TimelineLoopHalf,
            8,
            "stage2-inactive-loop-half",
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "timeline_loop_inactive"
    ));
    assert_eq!(engine.snapshot(), before_engine_loop_rejection);
    assert_eq!(
        *invalid_loop_runtime.lock().unwrap(),
        before_runtime_loop_rejection
    );

    let loop_set = json!({
        "timelineId": timeline_id,
        "playSessionId": play_session_id,
        "active": true
    });
    let loop_outcome = dispatch(
        &runtime,
        protocol::DjLinkMessageType::TimelineLoopSet,
        7,
        "stage2-loop-set",
        loop_set,
    );
    let (loop_state, loop_generation) = match loop_outcome {
        DjLinkDispatchOutcome::TimelineState {
            state,
            state_generation,
        } => (state, state_generation),
        outcome => panic!("Stage 2 loop set should be accepted after release: {outcome:?}"),
    };
    assert_eq!(loop_generation, beat_jump_generation + 1);
    assert_eq!(runtime.lock().unwrap().state_generation, loop_generation);
    assert!(runtime.lock().unwrap().loop_active);
    assert!(loop_state.loop_active);

    let length_before_half = engine
        .snapshot()
        .timeline
        .loop_runtime
        .b_ms
        .unwrap()
        .saturating_sub(engine.snapshot().timeline.loop_runtime.a_ms.unwrap());
    let half_outcome = dispatch(
        &runtime,
        protocol::DjLinkMessageType::TimelineLoopHalf,
        9,
        "stage2-loop-half",
        json!({
            "timelineId": timeline_id,
            "playSessionId": play_session_id,
        }),
    );
    let half_generation = match half_outcome {
        DjLinkDispatchOutcome::TimelineState {
            state,
            state_generation,
        } => {
            assert!(state.loop_active);
            state_generation
        }
        outcome => panic!("Stage 2 loop half should be accepted: {outcome:?}"),
    };
    assert_eq!(half_generation, loop_generation + 1);
    let half_snapshot = engine.snapshot();
    assert_eq!(
        half_snapshot.timeline.loop_runtime.b_ms.unwrap()
            - half_snapshot.timeline.loop_runtime.a_ms.unwrap(),
        (length_before_half / 2).max(1)
    );

    let loop_off_outcome = dispatch(
        &runtime,
        protocol::DjLinkMessageType::TimelineLoopSet,
        10,
        "stage2-loop-clear",
        json!({
            "timelineId": timeline_id,
            "playSessionId": play_session_id,
            "active": false
        }),
    );
    let (loop_off_state, loop_off_generation) = match loop_off_outcome {
        DjLinkDispatchOutcome::TimelineState {
            state,
            state_generation,
        } => (state, state_generation),
        outcome => panic!("Stage 2 loop clear should be accepted: {outcome:?}"),
    };
    assert_eq!(loop_off_generation, half_generation + 1);
    assert_eq!(
        runtime.lock().unwrap().state_generation,
        loop_off_generation
    );
    assert!(!runtime.lock().unwrap().loop_active);
    assert!(!loop_off_state.loop_active);

    let rejection_cases: [(&str, fn(&mut DjLinkRuntime), &str); 6] = [
        (
            "inactive-track",
            |candidate: &mut DjLinkRuntime| {
                candidate.track_active = false;
            },
            "timeline_not_active",
        ),
        (
            "unmapped",
            |candidate: &mut DjLinkRuntime| {
                candidate.unmapped_active_blocked = true;
            },
            "dj_link_unmapped_active",
        ),
        (
            "wrong-owner",
            |candidate: &mut DjLinkRuntime| {
                candidate.pedal_owner = Some("dj".to_string());
            },
            "timeline_pedal_owner_mismatch",
        ),
        (
            "non-running",
            |candidate: &mut DjLinkRuntime| {
                candidate.authoritative_state = protocol::DjLinkTimelineStateValue::Idle;
            },
            "timeline_state_not_running",
        ),
        (
            "missing-release",
            |candidate: &mut DjLinkRuntime| {
                candidate.release_event_id = None;
            },
            "timeline_release_missing",
        ),
        (
            "empty-release",
            |candidate: &mut DjLinkRuntime| {
                candidate.release_event_id = Some(String::new());
            },
            "timeline_release_missing",
        ),
    ];
    for (index, (_case_name, mutate, expected_code)) in rejection_cases.into_iter().enumerate() {
        let candidate = released_runtime.clone();
        let mut candidate = candidate;
        mutate(&mut candidate);
        let candidate = Mutex::new(candidate);
        for (command_index, message_type) in stage2_command_types.iter().copied().enumerate() {
            let before = engine.snapshot();
            let command_event_id = match message_type {
                protocol::DjLinkMessageType::TimelineBeatJump => "stage2-rejection-beat",
                protocol::DjLinkMessageType::TimelineLoopSet => "stage2-rejection-loop",
                protocol::DjLinkMessageType::TimelineLoopHalf => "stage2-rejection-half",
                _ => unreachable!("not a Stage 2 command"),
            };
            assert!(matches!(
                dispatch(
                    &candidate,
                    message_type,
                    u64::try_from(8 + index * 2 + command_index).unwrap(),
                    command_event_id,
                    stage2_command_payload(message_type, &timeline_id, play_session_id),
                ),
                DjLinkDispatchOutcome::Rejected { ref code, .. } if code == expected_code
            ));
            assert_eq!(engine.snapshot(), before);
        }
    }

    for (index, (_case_name, wrong_timeline_id, wrong_session_id, expected_code)) in [
        (
            "wrong-timeline",
            "999999",
            play_session_id,
            "timeline_identity_mismatch",
        ),
        (
            "wrong-session",
            timeline_id.as_str(),
            "other-play-session",
            "timeline_play_session_mismatch",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let candidate = Mutex::new(released_runtime.clone());
        for (command_index, message_type) in stage2_command_types.iter().copied().enumerate() {
            let before = engine.snapshot();
            let command_event_id = match message_type {
                protocol::DjLinkMessageType::TimelineBeatJump => {
                    "stage2-correlation-rejection-beat"
                }
                protocol::DjLinkMessageType::TimelineLoopSet => "stage2-correlation-rejection-loop",
                protocol::DjLinkMessageType::TimelineLoopHalf => {
                    "stage2-correlation-rejection-half"
                }
                _ => unreachable!("not a Stage 2 command"),
            };
            assert!(matches!(
                dispatch(
                    &candidate,
                    message_type,
                    u64::try_from(20 + index * 2 + command_index).unwrap(),
                    command_event_id,
                    stage2_command_payload(message_type, wrong_timeline_id, wrong_session_id),
                ),
                DjLinkDispatchOutcome::Rejected { ref code, .. } if code == expected_code
            ));
            assert_eq!(engine.snapshot(), before);
        }
    }

    engine
        .send(EngineCommand::SetTimelinePlaying(false))
        .unwrap();
    std::thread::sleep(Duration::from_millis(25));
    let stopped_state = match dispatch(
        &runtime,
        protocol::DjLinkMessageType::TimelineStateRequest,
        30,
        "stage2-stopped-state-request",
        json!({}),
    ) {
        DjLinkDispatchOutcome::TimelineState { state, .. } => state,
        outcome => panic!("stopped Timeline state request should return a state: {outcome:?}"),
    };
    assert_eq!(
        stopped_state.state,
        protocol::DjLinkTimelineStateValue::Stopped
    );
    for (command_index, message_type) in stage2_command_types.iter().copied().enumerate() {
        let before_engine = engine.snapshot();
        let before_runtime = runtime.lock().unwrap().clone();
        assert!(matches!(
            dispatch(
                &runtime,
                message_type,
                u64::try_from(31 + command_index).unwrap(),
                "stage2-stopped-command",
                stage2_command_payload(message_type, &timeline_id, play_session_id),
            ),
            DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "timeline_not_playing"
        ));
        assert_eq!(engine.snapshot(), before_engine);
        assert_eq!(*runtime.lock().unwrap(), before_runtime);
    }
}
