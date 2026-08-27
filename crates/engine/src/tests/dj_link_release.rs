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
            runtime.timeline_loop_region.clone(),
            runtime.timeline_audio_transport_revision,
            runtime.timeline_follow_natural_boundary_armed,
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_follow_transition.is_some(),
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
    runtime.timeline_follow_natural_boundary_armed = false;
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

    let assert_ineligible_follow_remains_disarmed =
        |position_ms: u64, disable_follow: bool, abort_fence: bool| {
            let mut runtime =
                timeline_follow_ltl5_runtime(protocol::TimelineFollowFaultPolicy::Hold);
            runtime
                .start_dj_link_timeline_runtime_at(TimelineId(8_101), position_ms)
                .unwrap();
            if disable_follow {
                runtime.timeline_follow.as_mut().unwrap().enabled = false;
            }
            if abort_fence {
                runtime.timeline_follow_abort_ticks_remaining = 1;
                runtime.timeline_follow_runtime.status =
                    protocol::TimelineFollowRuntimeStatus::Aborting;
            }
            runtime
                .apply_dj_link_release_without_transport_change()
                .unwrap();
            assert!(!runtime.timeline_follow_natural_boundary_armed);
            assert!(!matches!(
                runtime.timeline_follow_runtime.status,
                protocol::TimelineFollowRuntimeStatus::Armed
            ));
        };
    assert_ineligible_follow_remains_disarmed(900, true, false);
    assert_ineligible_follow_remains_disarmed(900, false, true);
    assert_ineligible_follow_remains_disarmed(1_000, false, false);
}

#[test]
fn dj_link_release_is_idempotent_without_generation_drift() {
    let mut runtime = runtime_with_lfo_effects(&[]);
    runtime.timeline_playing = true;
    runtime.timeline_loop_runtime.a_ms = Some(100);
    runtime.timeline_loop_runtime.b_ms = Some(900);
    runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Armed;
    runtime.timeline_follow_natural_boundary_armed = false;
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
    assert!(!runtime.timeline_follow_natural_boundary_armed);
}

#[test]
fn dj_link_release_rearms_follow_at_next_natural_boundary_with_runtime_only_destination_hold() {
    let mut runtime = timeline_follow_ltl5_runtime(protocol::TimelineFollowFaultPolicy::Hold);
    runtime.clock.bpm = 120.0;
    let mut source = runtime.timeline_bank[0].clone();
    source.duration_ms = 1_000;
    source.tempo_meter_map = vec![TimelineTempoMeterPoint {
        position_sixteenth_steps: 0,
        bpm: 120.0,
        numerator: 4,
        denominator: 4,
        interpolation: TimelineTempoInterpolation::Step,
        ..TimelineTempoMeterPoint::default()
    }];
    let follow = source.follow.as_mut().unwrap();
    follow.preroll_ms = 0;
    follow.hold_first_destination_measure = true;
    follow.destination_bpm = Some(120.0);
    follow.duration = VideoClipTakeDuration {
        unit: VideoClipTakeDurationUnit::Bars,
        value_milliunits: 1_000,
    };

    let mut target = runtime.timeline_bank[1].clone();
    target.duration_ms = 2_500;
    target.phases[0].end_ms = 2_500;
    target.tempo_meter_map = vec![TimelineTempoMeterPoint {
        position_sixteenth_steps: 0,
        bpm: 120.0,
        numerator: 5,
        denominator: 4,
        interpolation: TimelineTempoInterpolation::Step,
        ..TimelineTempoMeterPoint::default()
    }];
    runtime
        .apply_timeline_bank_state(vec![source, target], TimelineId(8_101), true)
        .unwrap();
    let authored_before = runtime.build_persistence_snapshot();

    runtime
        .start_dj_link_timeline_runtime_at(TimelineId(8_101), 900)
        .unwrap();
    assert!(runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, 900);
    assert!(!runtime.timeline_follow_natural_boundary_armed);
    let source_audio_revision_before_release = runtime.timeline_audio_transport_revision;
    let source_click_identity_before_release = runtime.timeline_click_scheduler.identity();

    let admission = ProjectSnapshotLoadAdmission::new_dj_link();
    let (ack, _receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::DjLinkRelease {
        expected_timeline_id: TimelineId(8_101),
        expires_at: Instant::now() + Duration::from_secs(1),
        admission,
        ack,
    });

    // RELEASE alone returns clock ownership; it neither locates nor restarts
    // the source. The only subsequent action is its ordinary 100 ms tick.
    assert!(runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, 900);
    assert!(runtime.timeline_external_sync_source.is_none());
    assert!(runtime.timeline_dj_link_observation.is_none());
    assert!(runtime.timeline_follow_natural_boundary_armed);
    assert!(runtime.timeline_follow_transition.is_none());
    assert_eq!(
        runtime.timeline_audio_transport_revision,
        source_audio_revision_before_release
    );
    assert_eq!(
        runtime.timeline_click_scheduler.identity().epoch,
        runtime.timeline_transport_epoch
    );
    assert_eq!(
        runtime
            .timeline_click_scheduler
            .identity()
            .transport_generation,
        runtime.timeline_transport_generation
    );
    assert_eq!(
        runtime
            .timeline_click_scheduler
            .identity()
            .schedule_generation,
        source_click_identity_before_release
            .schedule_generation
            .checked_add(1)
            .unwrap()
    );
    assert_eq!(
        runtime.build_persistence_snapshot().timeline_bank,
        authored_before.timeline_bank
    );
    assert_eq!(
        runtime.build_persistence_snapshot().timeline.loop_region,
        authored_before.timeline.loop_region
    );
    assert!(matches!(
        runtime.timeline_follow_runtime.status,
        protocol::TimelineFollowRuntimeStatus::Armed
    ));

    let boundary_at = Instant::now() + Duration::from_millis(100);
    runtime.last_tick_interval = Duration::from_millis(100);
    runtime.advance_timeline(boundary_at);
    let transition = runtime.timeline_follow_transition.as_ref().unwrap();
    assert_eq!(runtime.timeline_id, TimelineId(8_101));
    assert!(matches!(
        runtime.timeline_follow_runtime.status,
        protocol::TimelineFollowRuntimeStatus::Transitioning
    ));
    assert_eq!(
        runtime.timeline_follow_runtime.admission_reason,
        Some(protocol::TimelineFollowAdmissionReason::NaturalPlaybackBoundary)
    );
    assert_eq!(transition.duration, Duration::from_millis(2_000));
    assert_eq!(
        transition.destination_hold,
        Some(TimelineFollowHoldPlan {
            end_ms: 2_500,
            musical_length_millibeats: 5_000,
        })
    );

    runtime.advance_timeline_follow(boundary_at + Duration::from_millis(2_000));
    assert_eq!(runtime.timeline_id, TimelineId(8_102));
    assert!(runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, 0);
    assert!(runtime.timeline_follow_runtime.transition_hold_active);
    assert!(matches!(
        runtime.timeline_loop_runtime.status,
        TimelineLoopRuntimeStatus::Looping
    ));
    assert_eq!(runtime.timeline_loop_runtime.a_ms, Some(0));
    assert_eq!(runtime.timeline_loop_runtime.b_ms, Some(2_500));
    assert_eq!(
        runtime.timeline_loop_runtime.musical_length_millibeats,
        Some(5_000)
    );
    let persistence = runtime.build_persistence_snapshot();
    assert_eq!(persistence.timeline_bank, authored_before.timeline_bank);
    assert!(persistence.timeline.loop_region.is_none());
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
