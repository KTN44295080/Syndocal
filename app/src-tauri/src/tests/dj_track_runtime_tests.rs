use super::*;

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
    let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
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
                _ => unreachable!("not a Stage 2 command"),
            }
        };
    let stage2_command_types = [
        protocol::DjLinkMessageType::TimelineBeatJump,
        protocol::DjLinkMessageType::TimelineLoopSet,
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

    // Authority admission must not turn an engine-side validation failure
    // into a runtime projection.  A valid Release is present here, but the
    // authored beat-grid rejects any bar count other than +/-4.
    let before_engine_rejection = engine.snapshot();
    let before_runtime_rejection = runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            &runtime,
            protocol::DjLinkMessageType::TimelineBeatJump,
            7,
            "stage2-engine-rejected-beat",
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id,
                "bars": 3
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "engine_publication_rejected"
    ));
    assert_eq!(engine.snapshot(), before_engine_rejection);
    assert_eq!(*runtime.lock().unwrap(), before_runtime_rejection);

    // Exercise the same post-release fence for an authorized loop request,
    // while forcing the canonical engine preflight to reject an unsafe
    // division.  Neither side may be projected on that error.
    let mut invalid_loop_runtime = released_runtime.clone();
    invalid_loop_runtime.loop_division = Some(64);
    let invalid_loop_runtime = Mutex::new(invalid_loop_runtime);
    let before_engine_loop_rejection = engine.snapshot();
    let before_runtime_loop_rejection = invalid_loop_runtime.lock().unwrap().clone();
    assert!(matches!(
        dispatch(
            &invalid_loop_runtime,
            protocol::DjLinkMessageType::TimelineLoopSet,
            8,
            "stage2-engine-rejected-loop",
            json!({
                "timelineId": timeline_id,
                "playSessionId": play_session_id,
                "active": true
            }),
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. }
            if code == "engine_publication_rejected"
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
    assert_eq!(loop_off_generation, loop_generation + 1);
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
