use super::*;

fn apply_strict_root_loop_command(
    runtime: &mut EngineRuntime,
    expected_epoch: u64,
    expected_generation: u64,
    expected_loop_generation: u64,
    expected_follow_generation: u64,
    fail_publication: bool,
) -> (
    Result<TimelineLoopRuntimeAck, String>,
    Option<TimelineLoopRuntimeAck>,
    EngineSnapshot,
    EngineSnapshot,
) {
    apply_strict_root_loop_command_with_options(
        runtime,
        expected_epoch,
        expected_generation,
        expected_loop_generation,
        expected_follow_generation,
        fail_publication,
        false,
        Instant::now() + Duration::from_secs(1),
    )
}

fn apply_strict_root_loop_command_with_options(
    runtime: &mut EngineRuntime,
    expected_epoch: u64,
    expected_generation: u64,
    expected_loop_generation: u64,
    expected_follow_generation: u64,
    fail_publication: bool,
    poison_outcome: bool,
    expires_at: Instant,
) -> (
    Result<TimelineLoopRuntimeAck, String>,
    Option<TimelineLoopRuntimeAck>,
    EngineSnapshot,
    EngineSnapshot,
) {
    let (ack, receiver) = mpsc::sync_channel(1);
    let outcome = Arc::new(Mutex::new(None));
    // The shared image must begin at the exact pre-command A state. Building
    // it after `apply_command` would hide a worker-local partial mutation
    // from a forced-publication rollback assertion.
    let published_a = runtime.build_snapshot(0);
    let published = RwLock::new(published_a.clone());
    if poison_outcome {
        let poison_target = Arc::clone(&outcome);
        let _ = std::panic::catch_unwind(move || {
            let _guard = poison_target
                .lock()
                .expect("fresh strict root-loop outcome mutex must lock");
            panic!("deterministic strict root-loop acknowledgement poison");
        });
    }
    runtime.apply_command(EngineCommand::ApplyTimelineLoopRuntimePublished {
        expected_epoch,
        expected_generation,
        expected_loop_generation,
        expected_follow_generation,
        action: protocol::control_plane_command::TimelineLoopRuntimeActionV1::SetEnabled {
            enabled: false,
        },
        expires_at,
        completion: TimelineLoopRuntimePublicationCompletion {
            ack,
            outcome: Arc::clone(&outcome),
        },
    });
    if fail_publication {
        runtime.fail_next_pending_publication = true;
    }
    runtime.publish_pending_command_acks(0, &published);
    let result = receiver
        .recv()
        .expect("strict root-loop command must resolve");
    let outcome = outcome.lock().ok().and_then(|mut slot| slot.take());
    let result = result.and_then(|()| {
        outcome.ok_or_else(|| {
            "strict root-loop command published without an acknowledgement image".to_string()
        })
    });
    let published_after = published
        .into_inner()
        .expect("strict root-loop published snapshot must not poison");
    (result, outcome, published_a, published_after)
}

fn terminal_wait_for_pedal_follow_runtime() -> EngineRuntime {
    let mut runtime = timeline_follow_ltl5_runtime(protocol::TimelineFollowFaultPolicy::Hold);
    let mut source = runtime.timeline_bank[0].clone();
    let follow = source.follow.as_mut().expect("test source has Follow");
    follow.preroll_ms = 0;
    follow.destination_start_mode = TimelineFollowDestinationStartMode::WaitForPedal;
    follow.hold_first_destination_measure = false;
    let target = runtime.timeline_bank[1].clone();
    runtime
        .apply_timeline_bank_state(vec![source, target], TimelineId(8_101), true)
        .expect("test Follow bank must apply");
    runtime.timeline_playing = true;
    runtime.timeline_count_in_until = None;
    runtime.timeline_follow_natural_boundary_armed = true;
    runtime
}

#[test]
fn strict_root_loop_release_at_terminal_converges_once_then_waits_for_pedal() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = runtime.timeline_duration_ms();
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 71,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(0),
        b_ms: Some(runtime.timeline_duration_ms()),
        musical_length_millibeats: Some(4_000),
        wrap_count: 3,
    };
    runtime.timeline_follow_runtime.generation = 29;
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let rollback_a = (
        runtime.timeline_id,
        runtime.timeline_playing,
        runtime.timeline_position_ms,
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.clone(),
        runtime.timeline_follow_runtime.clone(),
        runtime.timeline_follow_natural_boundary_armed,
        runtime.timeline_follow_transition.is_some(),
        runtime.timeline_guide_cues.clone(),
        runtime.timeline_click_scheduler.identity(),
        runtime.timeline_audio_transport_revision,
    );

    let (failed, _failed_outcome, published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        true,
    );
    assert!(failed
        .expect_err("forced publication failure must reject")
        .contains("rolled back"));
    assert_eq!(
        (
            runtime.timeline_id,
            runtime.timeline_playing,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.clone(),
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_follow_natural_boundary_armed,
            runtime.timeline_follow_transition.is_some(),
            runtime.timeline_guide_cues.clone(),
            runtime.timeline_click_scheduler.identity(),
            runtime.timeline_audio_transport_revision,
        ),
        rollback_a,
        "publication rollback must restore the exact pre-release source image"
    );
    assert_eq!(
        published_after, published_a,
        "forced publication failure must leave the shared snapshot at exact A"
    );

    let (applied, outcome, _published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    let acknowledged = applied.expect("terminal root-loop release must publish");
    assert_eq!(
        outcome,
        Some(acknowledged),
        "the receipt must carry the final, not intermediate, authority image"
    );
    assert_eq!(
        acknowledged.disposition,
        TimelineLoopRuntimeDisposition::Applied
    );
    assert_eq!(acknowledged.epoch_after, runtime.timeline_transport_epoch);
    assert_eq!(
        acknowledged.generation_after,
        runtime.timeline_transport_generation
    );
    assert_eq!(
        acknowledged.loop_generation_after,
        runtime.timeline_loop_runtime.generation
    );
    assert_eq!(
        acknowledged.follow_generation_after,
        runtime.timeline_follow_runtime.generation
    );
    assert!(!runtime.timeline_playing);
    assert!(matches!(
        runtime.timeline_loop_runtime.status,
        TimelineLoopRuntimeStatus::Disabled
    ));
    assert!(matches!(
        runtime.timeline_follow_runtime.status,
        protocol::TimelineFollowRuntimeStatus::Transitioning
    ));
    assert_eq!(
        runtime.timeline_follow_runtime.admission_reason,
        Some(protocol::TimelineFollowAdmissionReason::NaturalPlaybackBoundary)
    );
    assert_eq!(
        published_after,
        runtime.build_snapshot(0),
        "the success receipt must publish the final terminal/Follow image, not loop-off B"
    );
    assert_eq!(
        published_after.timeline.transport_epoch,
        acknowledged.epoch_after
    );
    assert_eq!(
        published_after.timeline.transport_generation,
        acknowledged.generation_after
    );
    assert_eq!(
        published_after.timeline.loop_runtime.generation,
        acknowledged.loop_generation_after
    );
    assert_eq!(
        published_after.timeline.follow_runtime.generation,
        acknowledged.follow_generation_after
    );
    let identity = runtime.timeline_click_scheduler.identity();
    let breaks = runtime
        .timeline_guide_cues
        .iter()
        .filter(|cue| matches!(cue.cue, TimelineGuideCueKind::Break))
        .collect::<Vec<_>>();
    assert_eq!(breaks.len(), 1, "the root release emits one Break only");
    assert_eq!(breaks[0].epoch, identity.epoch);
    assert_eq!(
        breaks[0].transport_generation,
        identity.transport_generation
    );
    assert_eq!(breaks[0].schedule_generation, identity.schedule_generation);

    let terminal_at = runtime
        .timeline_follow_transition
        .as_ref()
        .map(|transition| transition.started_at + transition.duration)
        .expect("terminal release must admit one Follow transition");
    runtime.advance_timeline_follow(terminal_at);
    assert_eq!(runtime.timeline_id, TimelineId(8_102));
    assert!(!runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, 0);
    assert!(matches!(
        runtime.timeline_loop_runtime.status,
        TimelineLoopRuntimeStatus::Disabled
    ));
    assert!(runtime.timeline_follow_runtime.waiting_for_pedal_start);
    assert_eq!(
        runtime.timeline_follow_runtime.outcome,
        Some(protocol::TimelineFollowOutcome::Completed)
    );
    assert_eq!(
        runtime
            .timeline_guide_cues
            .iter()
            .filter(|cue| matches!(cue.cue, TimelineGuideCueKind::Complete))
            .count(),
        1,
        "the terminal Follow completion is unique"
    );
    let successor_image = (
        runtime.timeline_id,
        runtime.timeline_playing,
        runtime.timeline_position_ms,
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.clone(),
        runtime.timeline_follow_runtime.clone(),
        runtime.timeline_guide_cues.clone(),
    );
    let (stale, stale_outcome, _published_a, _published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    assert_eq!(
        stale.expect_err("the predecessor root-loop release must be stale"),
        "Timeline transport authority is stale"
    );
    assert!(stale_outcome.is_none());
    assert_eq!(
        (
            runtime.timeline_id,
            runtime.timeline_playing,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.clone(),
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_guide_cues.clone(),
        ),
        successor_image,
        "a stale source command must not mutate the installed wait target"
    );

    let follow_generation = runtime.timeline_follow_runtime.generation;
    runtime
        .start_waiting_follow_target_runtime(
            TimelineId(8_101),
            TimelineId(8_102),
            follow_generation,
        )
        .expect("only the explicit pedal path may start the waiting target");
    assert!(runtime.timeline_playing);
    assert_eq!(runtime.timeline_position_ms, 0);
    assert!(!runtime.timeline_follow_runtime.waiting_for_pedal_start);
    assert_eq!(
        runtime
            .start_waiting_follow_target_runtime(
                TimelineId(8_101),
                TimelineId(8_102),
                follow_generation
            )
            .expect_err("the one-shot pedal start must reject replay"),
        "DJ Link waiting Follow target admission is stale"
    );
}

#[test]
fn natural_terminal_before_root_loop_release_rejects_old_authority_without_mutation() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = 900;
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 17,
        status: TimelineLoopRuntimeStatus::Armed,
        a_ms: Some(100),
        b_ms: Some(900),
        musical_length_millibeats: Some(4_000),
        wrap_count: 0,
    };
    runtime.timeline_follow_runtime.generation = 31;
    let predecessor = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );

    runtime.last_tick_interval = Duration::from_millis(100);
    runtime.advance_timeline(Instant::now());
    assert!(!runtime.timeline_playing);
    assert!(runtime.timeline_follow_transition.is_none());
    assert_eq!(
        runtime.timeline_follow_runtime.generation, predecessor.3,
        "an armed predecessor loop cannot admit Follow before strict release"
    );
    let terminal_image = (
        runtime.timeline_id,
        runtime.timeline_position_ms,
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.clone(),
        runtime.timeline_follow_runtime.clone(),
        runtime.timeline_guide_cues.clone(),
    );

    let (stale, outcome, _published_a, _published_after) = apply_strict_root_loop_command(
        &mut runtime,
        predecessor.0,
        predecessor.1,
        predecessor.2,
        predecessor.3,
        false,
    );
    assert_eq!(
        stale.expect_err("terminal successor must fence the old loop release"),
        "Timeline transport authority is stale"
    );
    assert!(outcome.is_none());
    assert_eq!(
        (
            runtime.timeline_id,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.clone(),
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_guide_cues.clone(),
        ),
        terminal_image,
        "reverse ordering cannot create a second terminal or mutate Follow"
    );
}

#[test]
fn strict_root_loop_release_from_destination_hold_reissues_break_after_successor_commit() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = 0;
    runtime.timeline_follow_runtime.generation = 41;
    runtime.timeline_follow_runtime.transition_hold_active = true;
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 43,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(0),
        b_ms: Some(1_000),
        musical_length_millibeats: Some(4_000),
        wrap_count: 2,
    };
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );

    let rollback_a = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.clone(),
        runtime.timeline_follow_runtime.clone(),
        runtime.timeline_guide_cues.clone(),
        runtime.timeline_click_scheduler.identity(),
    );
    let (failed, _failed_outcome, published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        true,
    );
    assert!(failed
        .expect_err("hold release publication failure must reject")
        .contains("rolled back"));
    assert_eq!(
        (
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.clone(),
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_guide_cues.clone(),
            runtime.timeline_click_scheduler.identity(),
        ),
        rollback_a,
        "hold release rollback must restore bounds, Guide, and authority exactly"
    );
    assert_eq!(
        published_after, published_a,
        "hold publication failure must keep the shared snapshot at exact A"
    );

    let (applied, outcome, _published_a, _published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    let acknowledged = applied.expect("destination hold release must publish");
    assert_eq!(outcome, Some(acknowledged));
    assert_eq!(
        acknowledged.disposition,
        TimelineLoopRuntimeDisposition::Applied
    );
    assert!(!runtime.timeline_follow_runtime.transition_hold_active);
    assert!(matches!(
        runtime.timeline_loop_runtime.status,
        TimelineLoopRuntimeStatus::Disabled
    ));
    assert_eq!(runtime.timeline_loop_runtime.a_ms, None);
    assert_eq!(runtime.timeline_loop_runtime.b_ms, None);
    let identity = runtime.timeline_click_scheduler.identity();
    let breaks = runtime
        .timeline_guide_cues
        .iter()
        .filter(|cue| matches!(cue.cue, TimelineGuideCueKind::Break))
        .collect::<Vec<_>>();
    assert_eq!(
        breaks.len(),
        1,
        "hold release must not lose Break at commit"
    );
    assert_eq!(breaks[0].epoch, identity.epoch);
    assert_eq!(
        breaks[0].transport_generation,
        identity.transport_generation
    );
    assert_eq!(breaks[0].schedule_generation, identity.schedule_generation);
}

#[test]
fn terminal_root_loop_release_preflights_both_successors_before_any_mutation() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = runtime.timeline_duration_ms();
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 53,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(0),
        b_ms: Some(runtime.timeline_duration_ms()),
        musical_length_millibeats: Some(4_000),
        wrap_count: 0,
    };
    runtime.timeline_follow_runtime.generation = 59;
    let max = protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
    runtime.timeline_transport_epoch = max;
    runtime.timeline_transport_generation = max - 1;
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let image_before = (
        runtime.timeline_playing,
        runtime.timeline_position_ms,
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.clone(),
        runtime.timeline_follow_runtime.clone(),
        runtime.timeline_follow_transition.is_some(),
        runtime.timeline_guide_cues.clone(),
        runtime.timeline_click_scheduler.identity(),
    );

    let (rejected, outcome, published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    assert_eq!(
        rejected.expect_err("terminal release needs two authority successors"),
        "Timeline transport authority is exhausted"
    );
    assert!(outcome.is_none());
    assert_eq!(
        (
            runtime.timeline_playing,
            runtime.timeline_position_ms,
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.clone(),
            runtime.timeline_follow_runtime.clone(),
            runtime.timeline_follow_transition.is_some(),
            runtime.timeline_guide_cues.clone(),
            runtime.timeline_click_scheduler.identity(),
        ),
        image_before,
        "an unavailable terminal successor must not leave a loop-off partial mutation"
    );
    assert_eq!(
        published_after, published_a,
        "rejected preflight must leave the shared snapshot at exact A"
    );
}

#[test]
fn terminal_root_loop_release_preflights_every_click_schedule_successor_before_mutation() {
    for schedule_generation in [u64::MAX, u64::MAX - 1] {
        let mut runtime = terminal_wait_for_pedal_follow_runtime();
        runtime.timeline_position_ms = runtime.timeline_duration_ms();
        runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
            generation: 61,
            status: TimelineLoopRuntimeStatus::Looping,
            a_ms: Some(0),
            b_ms: Some(runtime.timeline_duration_ms()),
            musical_length_millibeats: Some(4_000),
            wrap_count: 0,
        };
        runtime.timeline_follow_runtime.generation = 67;
        runtime
            .timeline_click_scheduler
            .identity
            .schedule_generation = schedule_generation;
        let expected = (
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.generation,
            runtime.timeline_follow_runtime.generation,
        );
        let runtime_a = runtime.build_snapshot(0);

        let (rejected, outcome, published_a, published_after) = apply_strict_root_loop_command(
            &mut runtime,
            expected.0,
            expected.1,
            expected.2,
            expected.3,
            false,
        );
        assert_eq!(
            rejected.expect_err("terminal release needs two click schedule successors"),
            "Timeline click schedule generation is exhausted"
        );
        assert!(outcome.is_none());
        assert_eq!(
            runtime.build_snapshot(0),
            runtime_a,
            "schedule generation {schedule_generation} must reject before loop-off mutates A"
        );
        assert_eq!(
            published_after, published_a,
            "schedule generation {schedule_generation} must leave shared A published"
        );
    }
}

#[test]
fn terminal_root_loop_release_rejects_follow_target_incompatibility_without_partial_loop_off() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = runtime.timeline_duration_ms();
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 71,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(0),
        b_ms: Some(runtime.timeline_duration_ms()),
        musical_length_millibeats: Some(4_000),
        wrap_count: 0,
    };
    runtime.timeline_follow_runtime.generation = 73;
    // Exactly two transport successors are available: loop-off and terminal
    // settlement. The invalid target must reject the whole command instead
    // of falling into a third failure-settlement successor after B begins.
    let max = protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
    runtime.timeline_transport_epoch = max;
    runtime.timeline_transport_generation = max - 2;
    runtime.timeline_bank[1].id = TimelineId(8_199);
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let runtime_a = runtime.build_snapshot(0);

    let (rejected, outcome, published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    assert_eq!(
        rejected.expect_err("incompatible Follow target must reject the strict command"),
        "Timeline Follow target is no longer the next Timeline in bank order"
    );
    assert!(outcome.is_none());
    assert_eq!(runtime.build_snapshot(0), runtime_a);
    assert_eq!(published_after, published_a);
    assert!(runtime.timeline_follow_transition.is_none());
}

#[test]
fn terminal_root_loop_release_rejects_follow_prepare_or_audio_failure_at_exact_a() {
    for exhaust_audio_revision in [false, true] {
        let mut runtime = terminal_wait_for_pedal_follow_runtime();
        runtime.timeline_position_ms = runtime.timeline_duration_ms();
        runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
            generation: 79,
            status: TimelineLoopRuntimeStatus::Looping,
            a_ms: Some(0),
            b_ms: Some(runtime.timeline_duration_ms()),
            musical_length_millibeats: Some(4_000),
            wrap_count: 0,
        };
        runtime.timeline_follow_runtime.generation = 83;
        if exhaust_audio_revision {
            runtime.timeline_audio_transport_revision = u64::MAX;
        } else {
            runtime.timeline_bank[1]
                .audio_clips
                .push(TimelineAudioClipSummary {
                    id: 8_201,
                    path: "invalid-follow-target.wav".to_string(),
                    duration_ms: 0,
                    ..TimelineAudioClipSummary::default()
                });
        }
        let presenter = Arc::new(RwLock::new(None));
        runtime.timeline_follow_video_render_snapshot = Some(Arc::clone(&presenter));
        let expected = (
            runtime.timeline_transport_epoch,
            runtime.timeline_transport_generation,
            runtime.timeline_loop_runtime.generation,
            runtime.timeline_follow_runtime.generation,
        );
        let runtime_a = runtime.build_snapshot(0);
        let presenter_a = presenter
            .read()
            .expect("Follow presenter A must not poison")
            .clone();
        assert!(
            presenter_a.is_none(),
            "this rejection fixture starts with no active Follow presenter"
        );

        let (rejected, outcome, published_a, published_after) = apply_strict_root_loop_command(
            &mut runtime,
            expected.0,
            expected.1,
            expected.2,
            expected.3,
            false,
        );
        let error = rejected.expect_err("Follow admission failure must reject strict loop-off");
        if exhaust_audio_revision {
            assert_eq!(error, "Timeline audio transport revision is exhausted");
        } else {
            assert!(
                error.contains("Timeline audio clip 8201 duration must be greater than zero"),
                "unexpected prepare failure: {error}"
            );
        }
        assert!(outcome.is_none());
        assert_eq!(runtime.build_snapshot(0), runtime_a);
        assert_eq!(published_after, published_a);
        assert!(
            presenter
                .read()
                .expect("Follow presenter after reject must not poison")
                .is_none(),
            "strict reject must restore the Handle-only Follow presenter to its exact None A"
        );
        assert!(runtime.timeline_follow_transition.is_none());
    }
}

#[test]
fn strict_root_loop_noop_does_not_consume_terminal_authority_or_click_schedule() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = runtime.timeline_duration_ms();
    runtime.timeline_loop_runtime.status = TimelineLoopRuntimeStatus::Disabled;
    runtime.timeline_loop_runtime.generation = 89;
    runtime.timeline_follow_runtime.generation = 97;
    runtime
        .timeline_click_scheduler
        .identity
        .schedule_generation = u64::MAX;
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let runtime_a = runtime.build_snapshot(0);

    let (result, outcome, published_a, published_after) = apply_strict_root_loop_command(
        &mut runtime,
        expected.0,
        expected.1,
        expected.2,
        expected.3,
        false,
    );
    let acknowledgement = result.expect("unchanged strict loop-off is a typed NoOp");
    assert_eq!(
        acknowledgement.disposition,
        TimelineLoopRuntimeDisposition::NoOp
    );
    assert_eq!(outcome, Some(acknowledgement));
    assert_eq!(runtime.build_snapshot(0), runtime_a);
    assert_eq!(published_after, published_a);
}

#[test]
fn strict_root_loop_completion_poison_after_applied_b_restores_exact_a_without_late_publication() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = 500;
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 101,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(100),
        b_ms: Some(900),
        musical_length_millibeats: Some(4_000),
        wrap_count: 2,
    };
    runtime.timeline_follow_runtime.generation = 103;
    let presenter = Arc::new(RwLock::new(None));
    runtime.timeline_follow_video_render_snapshot = Some(Arc::clone(&presenter));
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let runtime_a = runtime.build_snapshot(0);

    let (rejected, outcome, published_a, published_after) =
        apply_strict_root_loop_command_with_options(
            &mut runtime,
            expected.0,
            expected.1,
            expected.2,
            expected.3,
            false,
            true,
            Instant::now() + Duration::from_secs(1),
        );
    assert_eq!(
        rejected.expect_err("poisoned completion must reject after B mutation"),
        "Timeline loop acknowledgement state was poisoned"
    );
    assert!(
        outcome.is_none(),
        "poisoned completion cannot expose an ACK image"
    );
    assert_eq!(runtime.build_snapshot(0), runtime_a);
    assert_eq!(published_after, published_a);
    assert!(
        presenter
            .read()
            .expect("Follow presenter after poison must not poison")
            .is_none(),
        "poison rollback must regenerate the Handle-only Follow presenter at exact None A"
    );
    assert!(
        runtime.pending_command_acks.is_empty(),
        "a poisoned completion must not leave an Applied B acknowledgement for a later tick"
    );

    // A zero-delta next tick is intentionally non-terminal. It proves there
    // is no delayed command receipt which can publish the rejected loop-off
    // B after the error ACK already resolved.
    runtime.timeline_playhead_boundary_armed = false;
    runtime.last_tick_interval = Duration::ZERO;
    runtime.advance_timeline(Instant::now());
    let shared_after_tick = RwLock::new(published_after.clone());
    runtime.publish_pending_command_acks(0, &shared_after_tick);
    assert_eq!(
        shared_after_tick
            .into_inner()
            .expect("shared snapshot after tick must not poison"),
        published_after,
        "the following tick must not publish delayed B"
    );
}

#[test]
fn expired_strict_root_loop_command_preserves_runtime_presenter_and_shared_a() {
    let mut runtime = terminal_wait_for_pedal_follow_runtime();
    runtime.timeline_position_ms = 500;
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 107,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(100),
        b_ms: Some(900),
        musical_length_millibeats: Some(4_000),
        wrap_count: 0,
    };
    runtime.timeline_follow_runtime.generation = 109;
    let presenter = Arc::new(RwLock::new(None));
    runtime.timeline_follow_video_render_snapshot = Some(Arc::clone(&presenter));
    let expected = (
        runtime.timeline_transport_epoch,
        runtime.timeline_transport_generation,
        runtime.timeline_loop_runtime.generation,
        runtime.timeline_follow_runtime.generation,
    );
    let runtime_a = runtime.build_snapshot(0);

    let (rejected, outcome, published_a, published_after) =
        apply_strict_root_loop_command_with_options(
            &mut runtime,
            expected.0,
            expected.1,
            expected.2,
            expected.3,
            false,
            false,
            Instant::now() - Duration::from_millis(1),
        );
    assert_eq!(
        rejected.expect_err("expired strict command must reject before mutation"),
        "Timeline loop command expired before engine execution"
    );
    assert!(outcome.is_none());
    assert_eq!(runtime.build_snapshot(0), runtime_a);
    assert_eq!(published_after, published_a);
    assert!(
        presenter
            .read()
            .expect("Follow presenter after expiry must not poison")
            .is_none(),
        "expiry must not touch the Handle-only Follow presenter"
    );
}

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
