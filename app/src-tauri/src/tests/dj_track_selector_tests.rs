use super::*;

#[test]
fn dj_link_title_contains_admission_and_deck_one_fallback_are_fail_closed() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let mut snapshot = engine.snapshot();
    snapshot.timeline.phases = vec![TimelinePhaseSummary {
        id: protocol::TimelinePhaseId(1),
        label: "title contains admission".to_string(),
        role: protocol::TimelinePhaseRole::Intro,
        start_ms: 0,
        end_ms: 16_000,
    }];
    snapshot.timeline.loop_region = Some(TimelineLoopRegionSummary {
        a_ms: 0,
        b_ms: 16_000,
        enabled: false,
        musical_length_beats: None,
    });
    engine
        .apply_timeline_bank_published(vec![snapshot.timeline.clone()], snapshot.timeline.id, false)
        .unwrap();
    let timeline_id = engine.snapshot().timeline.id;
    let title_contains = || {
        let mut mapping = dj_link_test_mapping(
            "jinsei-over-title",
            protocol::DjTrackSelector {
                content_id: None,
                title: None,
                artist: None,
                title_contains: Some(
                    "\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc}".to_string(),
                ),
                fallback_deck: Some(1),
            },
        );
        mapping.timeline_id = timeline_id;
        mapping
    };
    let exact_title = || {
        let mut mapping = dj_link_test_mapping(
            "exact-remix-title",
            protocol::DjTrackSelector {
                content_id: None,
                title: Some("\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc} (Remix)".to_string()),
                artist: Some("ignored for title contains".to_string()),
                title_contains: None,
                fallback_deck: None,
            },
        );
        mapping.timeline_id = timeline_id;
        mapping
    };
    let exact_primary = || {
        let mut mapping = dj_link_test_mapping(
            "exact-primary-wins",
            protocol::DjTrackSelector {
                content_id: None,
                title: Some("exact primary title".to_string()),
                artist: Some("exact primary artist".to_string()),
                title_contains: None,
                fallback_deck: None,
            },
        );
        mapping.timeline_id = timeline_id;
        mapping
    };
    let second_fallback = || {
        let mut mapping = dj_link_test_mapping(
            "second-deck-one-fallback",
            protocol::DjTrackSelector {
                content_id: None,
                title: None,
                artist: None,
                title_contains: Some("different title selector".to_string()),
                fallback_deck: Some(1),
            },
        );
        mapping.timeline_id = timeline_id;
        mapping
    };
    let title_payload = || protocol::DjLinkTrackPayload {
        deck: 2,
        deck_id: "rekordbox-deck-2".to_string(),
        content_id: None,
        title: Some("\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc} (Remix)".to_string()),
        artist: Some("ignored for title contains".to_string()),
        track_bpm: None,
        position_at_send_sec: 0.0,
        effective_bpm: 120.0,
        position_revision: 1,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-27T00:00:00Z".to_string(),
        play_session_id: "title-contains-session".to_string(),
        loop_state: None,
    };
    let content_payload = |deck| protocol::DjLinkTrackPayload {
        deck,
        deck_id: format!("rekordbox-deck-{deck}"),
        content_id: Some("content-only-without-title".to_string()),
        title: None,
        artist: None,
        track_bpm: None,
        position_at_send_sec: 0.0,
        effective_bpm: 120.0,
        position_revision: 1,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-27T00:00:00Z".to_string(),
        play_session_id: format!("content-only-deck-{deck}"),
        loop_state: None,
    };
    let nonmatching_title_payload = |deck| protocol::DjLinkTrackPayload {
        deck,
        deck_id: format!("rekordbox-deck-{deck}"),
        content_id: None,
        title: Some("unrelated title".to_string()),
        artist: Some("unrelated artist".to_string()),
        track_bpm: None,
        position_at_send_sec: 0.0,
        effective_bpm: 120.0,
        position_revision: 1,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-27T00:00:00Z".to_string(),
        play_session_id: format!("nonmatching-title-deck-{deck}"),
        loop_state: None,
    };
    let exact_primary_payload = protocol::DjLinkTrackPayload {
        deck: 1,
        deck_id: "rekordbox-deck-1".to_string(),
        content_id: None,
        title: Some("exact primary title".to_string()),
        artist: Some("exact primary artist".to_string()),
        track_bpm: None,
        position_at_send_sec: 0.0,
        effective_bpm: 120.0,
        position_revision: 1,
        sample_age_ms: 0,
        is_playing: true,
        started_at: "2026-08-27T00:00:00Z".to_string(),
        play_session_id: "exact-primary-session".to_string(),
        loop_state: None,
    };
    let make_runtime = |mappings| {
        let mut coordinator = project_coordinator_for_initial_snapshot(engine.snapshot());
        coordinator.mappings.dj_track_triggers = mappings;
        DjLinkRuntime::from_coordinator(&coordinator)
    };

    assert!(matches!(
        dj_track_selector::resolve_track_mapping(&[title_contains()], &title_payload()),
        dj_track_selector::DjTrackMappingResolution::Unique(mapping)
            if mapping.id == "jinsei-over-title"
    ));
    let case_sensitive_selector = dj_link_test_mapping(
        "case-sensitive-title-contains",
        protocol::DjTrackSelector {
            content_id: None,
            title: None,
            artist: None,
            title_contains: Some("REMIX".to_string()),
            fallback_deck: None,
        },
    );
    let lower_case_remix = protocol::DjLinkTrackPayload {
        title: Some("\u{4eba}\u{751f}\u{30aa}\u{30fc}\u{30d0}\u{30fc} (remix)".to_string()),
        ..title_payload()
    };
    assert_eq!(
        dj_track_selector::resolve_track_mapping(&[case_sensitive_selector], &lower_case_remix),
        dj_track_selector::DjTrackMappingResolution::NoMapping
    );
    let no_match = nonmatching_title_payload(2);
    assert_eq!(
        dj_track_selector::resolve_track_mapping(&[title_contains()], &no_match),
        dj_track_selector::DjTrackMappingResolution::NoMapping
    );
    assert!(matches!(
        dj_track_selector::resolve_track_mapping(&[title_contains()], &content_payload(1)),
        dj_track_selector::DjTrackMappingResolution::Unique(mapping)
            if mapping.id == "jinsei-over-title"
    ));
    assert!(matches!(
        dj_track_selector::resolve_track_mapping(
            &[title_contains()],
            &nonmatching_title_payload(1)
        ),
        dj_track_selector::DjTrackMappingResolution::Unique(mapping)
            if mapping.id == "jinsei-over-title"
    ));
    assert_eq!(
        dj_track_selector::resolve_track_mapping(&[title_contains()], &content_payload(2)),
        dj_track_selector::DjTrackMappingResolution::NoMapping
    );
    assert!(matches!(
        dj_track_selector::resolve_track_mapping(
            &[title_contains(), exact_primary()],
            &exact_primary_payload
        ),
        dj_track_selector::DjTrackMappingResolution::Unique(mapping)
            if mapping.id == "exact-primary-wins"
    ));
    assert_eq!(
        dj_track_selector::resolve_track_mapping(
            &[title_contains(), second_fallback()],
            &nonmatching_title_payload(1)
        ),
        dj_track_selector::DjTrackMappingResolution::Ambiguous
    );
    assert_eq!(
        dj_track_selector::resolve_track_mapping(
            &[title_contains(), exact_title()],
            &title_payload()
        ),
        dj_track_selector::DjTrackMappingResolution::Ambiguous
    );

    let mut runtime = make_runtime(vec![title_contains()]);
    let runtime_generation = runtime.state_generation;
    assert!(matches!(
        dj_track_runtime::dispatch_active(
            title_payload(),
            &engine,
            &mut runtime,
            runtime_generation,
            "title-contains-active",
            1,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let synced_title_payload = protocol::DjLinkTrackPayload {
        position_at_send_sec: 1.0,
        position_revision: 2,
        ..title_payload()
    };
    let sync_generation = runtime.state_generation;
    assert!(matches!(
        dj_track_runtime::dispatch_sync(
            synced_title_payload.clone(),
            &engine,
            &mut runtime,
            sync_generation,
            "title-contains-same-identity-sync",
            2,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));
    let mismatched_title_payload = protocol::DjLinkTrackPayload {
        title: Some("different title identity".to_string()),
        artist: Some("different artist identity".to_string()),
        position_revision: 3,
        ..synced_title_payload
    };
    let engine_before_identity_mismatch = engine.snapshot();
    let runtime_before_identity_mismatch = runtime.clone();
    let mismatch_generation = runtime.state_generation;
    assert!(matches!(
        dj_track_runtime::dispatch_sync(
            mismatched_title_payload,
            &engine,
            &mut runtime,
            mismatch_generation,
            "title-contains-different-identity-sync",
            3,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "track_sync_context_mismatch"
    ));
    assert_eq!(engine.snapshot(), engine_before_identity_mismatch);
    assert_eq!(runtime, runtime_before_identity_mismatch);

    let mut no_match_runtime = make_runtime(vec![title_contains()]);
    let no_match_generation = no_match_runtime.state_generation;
    let before_no_match = engine.snapshot();
    assert!(matches!(
        dj_track_runtime::dispatch_active(
            no_match,
            &engine,
            &mut no_match_runtime,
            no_match_generation,
            "title-contains-no-match",
            4,
        ),
        DjLinkDispatchOutcome::NoMapping { .. }
    ));
    assert_eq!(engine.snapshot(), before_no_match);

    let mut fallback_runtime = make_runtime(vec![title_contains()]);
    let fallback_generation = fallback_runtime.state_generation;
    assert!(matches!(
        dj_track_runtime::dispatch_active(
            nonmatching_title_payload(1),
            &engine,
            &mut fallback_runtime,
            fallback_generation,
            "title-contains-deck-one-fallback",
            5,
        ),
        DjLinkDispatchOutcome::TimelineState { .. }
    ));

    let mut deck_two_runtime = make_runtime(vec![title_contains()]);
    let deck_two_generation = deck_two_runtime.state_generation;
    let before_deck_two = engine.snapshot();
    assert!(matches!(
        dj_track_runtime::dispatch_active(
            nonmatching_title_payload(2),
            &engine,
            &mut deck_two_runtime,
            deck_two_generation,
            "title-contains-deck-two-no-fallback",
            6,
        ),
        DjLinkDispatchOutcome::NoMapping { .. }
    ));
    assert_eq!(engine.snapshot(), before_deck_two);

    let mut ambiguous_runtime = make_runtime(vec![title_contains(), exact_title()]);
    let ambiguous_generation = ambiguous_runtime.state_generation;
    let before_ambiguous = engine.snapshot();
    let runtime_before_ambiguous = ambiguous_runtime.clone();
    assert!(matches!(
        dj_track_runtime::dispatch_active(
            title_payload(),
            &engine,
            &mut ambiguous_runtime,
            ambiguous_generation,
            "title-contains-ambiguous",
            7,
        ),
        DjLinkDispatchOutcome::Rejected { ref code, .. } if code == "track_mapping_ambiguous"
    ));
    assert_eq!(engine.snapshot(), before_ambiguous);
    assert_eq!(ambiguous_runtime, runtime_before_ambiguous);
}
