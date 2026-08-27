use super::*;

#[test]
fn dj_link_release_clock_handoff_preserves_position_and_fails_closed_off_contract() {
    let mut runtime =
        direct_child_static_test_runtime(vec![direct_child_static_event(201, 2, 100, 500)]);
    let signature = |runtime: &EngineRuntime| {
        (
            runtime.timeline_playing,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_external_sync_source.clone(),
            runtime
                .timeline_dj_link_observation
                .map(|observation| observation.position_ms),
            runtime.timeline_loop_runtime.clone(),
        )
    };
    let apply_release_command = |runtime: &mut EngineRuntime| {
        let admission = ProjectSnapshotLoadAdmission::new_dj_link();
        let (ack, _receiver) = mpsc::sync_channel(1);
        runtime.apply_command(EngineCommand::DjLinkRelease {
            expected_timeline_id: TimelineId(7_701),
            expires_at: Instant::now() + Duration::from_secs(1),
            admission,
            ack,
        });
    };
    let now = Instant::now();
    runtime
        .set_cue_child_timeline_state(
            2,
            Some(ChildTimelineSummary {
                events: vec![direct_child_static_event(301, 3, 100, 100)],
                duration_ms: 300,
                ..ChildTimelineSummary::default()
            }),
        )
        .unwrap();
    runtime.timeline_events = vec![timeline_test_event(200, 1, 0, 0, 2_000, 1)];
    runtime.rebuild_effect_activations(now);
    runtime.timeline_id = TimelineId(7_701);
    runtime.timeline_playing = true;
    runtime.timeline_position_ms = 350;
    runtime.timeline_external_sync_source = Some(ClockSource::DjLink);
    runtime.timeline_dj_link_observation = Some(DjLinkTimelineObservation {
        position_ms: 350,
        observed_at: now,
    });
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 7,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(100),
        b_ms: Some(900),
        musical_length_millibeats: None,
        wrap_count: 3,
    };
    runtime.establish_child_transports_at_position(now);
    runtime.start_cue(1, now, PendingCueTriggerSource::Manual);
    runtime.timeline_follow_runtime.generation = 11;
    runtime.timeline_follow_runtime.status = protocol::TimelineFollowRuntimeStatus::Armed;
    assert_eq!(runtime.child_transports.len(), 1);
    assert_eq!(runtime.direct_child_transports.len(), 2);
    assert_eq!(runtime.nested_child_transports.len(), 2);
    let position_before = runtime.timeline_position_ms;
    let transport_signature = |transport: &RuntimeChildTransport| {
        (
            transport.owner_cue_id,
            transport.parent_event_id,
            transport.active,
            transport.previous_position_ms,
            transport.position_ms,
            transport.direct_generation,
            transport.activation_generation,
        )
    };
    let children_before = runtime
        .child_transports
        .iter()
        .map(transport_signature)
        .collect::<Vec<_>>();
    let direct_children_before = runtime
        .direct_child_transports
        .iter()
        .map(transport_signature)
        .collect::<Vec<_>>();
    let nested_children_before = runtime
        .nested_child_transports
        .iter()
        .map(transport_signature)
        .collect::<Vec<_>>();
    let follow_before = runtime.timeline_follow_runtime.clone();

    apply_release_command(&mut runtime);
    assert!(runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, position_before);
    assert_eq!(
        runtime
            .child_transports
            .iter()
            .map(transport_signature)
            .collect::<Vec<_>>(),
        children_before
    );
    assert_eq!(
        runtime
            .direct_child_transports
            .iter()
            .map(transport_signature)
            .collect::<Vec<_>>(),
        direct_children_before
    );
    assert_eq!(
        runtime
            .nested_child_transports
            .iter()
            .map(transport_signature)
            .collect::<Vec<_>>(),
        nested_children_before
    );
    assert_eq!(runtime.timeline_follow_runtime, follow_before);
    assert!(runtime.timeline_external_sync_source.is_none());
    assert!(runtime.timeline_dj_link_observation.is_none());
    assert!(matches!(
        runtime.timeline_loop_runtime.status,
        TimelineLoopRuntimeStatus::Disabled
    ));

    let internally_owned_state = signature(&runtime);
    runtime
        .apply_dj_link_release_without_transport_change()
        .unwrap();
    assert_eq!(signature(&runtime), internally_owned_state);

    runtime.timeline_external_sync_source = Some(ClockSource::MidiClock);
    runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Armed;
    let foreign_state = signature(&runtime);
    assert_eq!(
        runtime
            .apply_dj_link_release_without_transport_change()
            .unwrap_err(),
        "DJ Link release cannot take ownership from another external clock"
    );
    assert_eq!(signature(&runtime), foreign_state);
    apply_release_command(&mut runtime);
    assert_eq!(signature(&runtime), foreign_state);

    runtime.timeline_external_sync_source = Some(ClockSource::DjLink);
    runtime.timeline_playing = false;
    runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Armed;
    let stopped_state = signature(&runtime);
    assert_eq!(
        runtime
            .apply_dj_link_release_without_transport_change()
            .unwrap_err(),
        "DJ Link release requires an already-playing Timeline"
    );
    assert_eq!(signature(&runtime), stopped_state);
    apply_release_command(&mut runtime);
    assert_eq!(signature(&runtime), stopped_state);

    runtime.timeline_playing = true;
    runtime.timeline_transport_epoch = protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
    runtime.timeline_transport_generation =
        protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
    let exhausted_state = signature(&runtime);
    assert_eq!(
        runtime
            .apply_dj_link_release_without_transport_change()
            .unwrap_err(),
        "Timeline transport authority is exhausted"
    );
    assert_eq!(signature(&runtime), exhausted_state);
    apply_release_command(&mut runtime);
    assert_eq!(signature(&runtime), exhausted_state);
}

#[test]
fn dj_link_release_is_idempotent_without_generation_drift() {
    let mut runtime = runtime_with_lfo_effects(&[]);
    runtime.timeline_playing = true;
    runtime.timeline_loop_runtime.a_ms = Some(100);
    runtime.timeline_loop_runtime.b_ms = Some(900);
    runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Armed;
    let generation_before_release = runtime.timeline_loop_runtime.generation;
    runtime
        .apply_dj_link_release_without_transport_change()
        .unwrap();
    let generation_after_release = runtime.timeline_loop_runtime.generation;
    runtime
        .apply_dj_link_release_without_transport_change()
        .unwrap();
    assert_eq!(
        runtime.timeline_loop_runtime.generation,
        generation_after_release
    );
    assert!(generation_after_release > generation_before_release);
}

#[test]
fn dj_link_release_publication_rollback_clears_private_observation_fail_closed() {
    let mut runtime = dj_link_sync_cadence_test_runtime();
    runtime
        .start_dj_link_timeline_runtime_at(TimelineId(7_701), 3_500)
        .unwrap();
    let rollback = runtime.timeline_transport_rollback();
    let public_state_before = (
        runtime.timeline_playing,
        runtime.timeline_position_ms,
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_external_sync_source.clone(),
        runtime.timeline_loop_runtime.clone(),
    );
    assert!(runtime.timeline_dj_link_observation.is_some());

    runtime
        .apply_dj_link_release_without_transport_change()
        .unwrap();
    runtime.rollback_pending_command(rollback);

    assert_eq!(
        (
            runtime.timeline_playing,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_external_sync_source.clone(),
            runtime.timeline_loop_runtime.clone(),
        ),
        public_state_before,
        "publication rollback restores the public transport image"
    );
    assert!(
        runtime.timeline_dj_link_observation.is_none(),
        "the private DJ baseline must fail closed after rollback; the next SYNC is classified raw"
    );
}
