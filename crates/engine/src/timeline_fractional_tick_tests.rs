use super::*;

fn fractional_runtime() -> EngineRuntime {
    let mut runtime = runtime_with_lfo_effects(&[]);
    runtime.timeline_phases = vec![TimelinePhaseSummary {
        id: TimelinePhaseId(73_101),
        label: "Fractional clock proof".into(),
        role: protocol::TimelinePhaseRole::Verse,
        start_ms: 0,
        end_ms: 1_000_000,
    }];
    runtime.timeline_playing = true;
    runtime.timeline_count_in_until = None;
    runtime.timeline_external_sync_source = None;
    runtime
}

fn advance(runtime: &mut EngineRuntime, delta: Duration) {
    runtime.last_tick_interval = delta;
    runtime.advance_timeline(Instant::now());
}

#[test]
fn timeline_fractional_ticks_match_elapsed_time_at_44hz_and_with_jitter() {
    for intervals in [
        vec![Duration::from_micros(22_727); 4_400],
        [100, 22_799, 23_005, 900, 40_321, 17_555]
            .into_iter()
            .map(Duration::from_micros)
            .cycle()
            .take(4_400)
            .collect(),
    ] {
        let mut runtime = fractional_runtime();
        let mut elapsed = Duration::ZERO;
        for delta in intervals {
            elapsed += delta;
            advance(&mut runtime, delta);
            assert_eq!(runtime.timeline_position_ms, elapsed.as_millis() as u64);
            assert_eq!(
                runtime.timeline_tick_remainder.unwrap().nanoseconds,
                (elapsed.as_nanos() % 1_000_000) as u32
            );
        }
    }
}

#[test]
fn timeline_fractional_ticks_reset_on_seek_pause_count_in_external_sync_and_restart() {
    let mut runtime = fractional_runtime();
    advance(&mut runtime, Duration::from_micros(750));
    runtime.apply_command(EngineCommand::SeekTimeline(0));
    advance(&mut runtime, Duration::from_micros(500));
    assert_eq!(runtime.timeline_position_ms, 0, "even a same-position seek retires carry");

    runtime.apply_command(EngineCommand::SetTimelinePlaying(false));
    advance(&mut runtime, Duration::from_secs(30));
    assert_eq!(runtime.timeline_position_ms, 0);
    assert!(runtime.timeline_tick_remainder.is_none());
    runtime.apply_command(EngineCommand::SetTimelinePlaying(true));
    advance(&mut runtime, Duration::from_micros(750));
    assert_eq!(runtime.timeline_position_ms, 0);

    let now = Instant::now();
    runtime.timeline_count_in_until = Some(now + Duration::from_secs(1));
    runtime.advance_timeline(now);
    assert!(runtime.timeline_tick_remainder.is_none());
    runtime.advance_timeline(now + Duration::from_secs(2));
    assert_eq!(runtime.timeline_position_ms, 0);
    advance(&mut runtime, Duration::from_micros(750));
    assert_eq!(runtime.timeline_position_ms, 0);

    runtime.timeline_external_sync_source = Some(ClockSource::MidiTimecode);
    advance(&mut runtime, Duration::from_secs(30));
    assert_eq!(runtime.timeline_position_ms, 0);
    runtime.apply_command(EngineCommand::SetTimelinePlaying(true));
    advance(&mut runtime, Duration::from_micros(750));
    assert_eq!(runtime.timeline_position_ms, 0);

    runtime.apply_command(EngineCommand::SeekTimeline(0));
    advance(&mut runtime, Duration::from_millis(10));
    assert_eq!(runtime.timeline_position_ms, 10, "restart keeps the original rate");
}

#[test]
fn timeline_fractional_ticks_keep_carry_after_failed_transport_publication() {
    let mut runtime = fractional_runtime();
    advance(&mut runtime, Duration::from_micros(750));
    let rollback = runtime.timeline_transport_rollback();
    runtime.apply_timeline_playing_command(false, true).unwrap();
    runtime.rollback_pending_command(rollback);
    advance(&mut runtime, Duration::from_micros(500));
    assert_eq!(runtime.timeline_position_ms, 1);
    assert_eq!(runtime.timeline_tick_remainder.unwrap().nanoseconds, 250_000);
}

#[test]
fn timeline_fractional_ticks_discard_submillisecond_remainder_after_armed_jump() {
    let mut runtime = fractional_runtime();
    let mut source = timeline_test_event(10, 1, 0, 0, 100, 1);
    source.jump_to_event_id = Some(20);
    let mut destination = timeline_test_event(20, 2, 200, 0, 100, 1);
    // The test only needs transport semantics, not a fixture/cue dispatch.
    destination.layer_muted_effective = true;
    runtime.timeline_events = vec![source, destination];
    runtime.timeline_position_ms = 100;
    runtime.timeline_playhead_boundary_armed = true;
    advance(&mut runtime, Duration::from_micros(750));
    assert_eq!(runtime.timeline_position_ms, 200);
    assert_eq!(runtime.timeline_jump_landed_event_id, Some(20));
    assert!(runtime.timeline_tick_remainder.is_none());
    advance(&mut runtime, Duration::from_micros(500));
    assert_eq!(runtime.timeline_position_ms, 200, "jump destination cannot inherit discarded source time");
}

fn enable_one_ms_loop(runtime: &mut EngineRuntime) {
    runtime.timeline_position_ms = 10;
    runtime.timeline_loop_runtime = TimelineLoopRuntimeSummary {
        generation: 1,
        status: TimelineLoopRuntimeStatus::Looping,
        a_ms: Some(10),
        b_ms: Some(11),
        musical_length_millibeats: None,
        wrap_count: 0,
    };
}

#[test]
fn timeline_fractional_ticks_preserve_carry_across_wraps_and_reserve_exact_authority() {
    let mut runtime = fractional_runtime();
    enable_one_ms_loop(&mut runtime);
    for _ in 0..3 {
        advance(&mut runtime, Duration::from_micros(750));
    }
    assert_eq!(runtime.timeline_position_ms, 10);
    assert_eq!(runtime.timeline_loop_runtime.wrap_count, 2);
    assert_eq!(runtime.timeline_tick_remainder.unwrap().nanoseconds, 250_000);

    let max = protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER;
    let mut exhausted = fractional_runtime();
    enable_one_ms_loop(&mut exhausted);
    exhausted.timeline_transport_epoch = max;
    exhausted.timeline_transport_generation = max - 1;
    advance(&mut exhausted, Duration::from_micros(750));
    exhausted.last_tick_interval = Duration::from_micros(1_500);
    let before = exhausted.timeline_tick_remainder;
    assert_eq!(exhausted.timeline_loop_wraps_for_next_tick(), 2);
    exhausted.advance_timeline(Instant::now());
    assert_eq!(exhausted.timeline_position_ms, 10);
    assert_eq!(exhausted.timeline_loop_runtime.wrap_count, 0);
    assert_eq!(exhausted.timeline_transport_generation, max - 1);
    assert_eq!(exhausted.timeline_tick_remainder, before, "failed reservation cannot consume carry");
    assert!(exhausted.last_error.as_deref().unwrap().contains("exhausted"));
}

#[test]
fn timeline_fractional_ticks_keep_millisecond_rate_across_bpm_changes_and_saturate() {
    let mut runtime = fractional_runtime();
    for bpm in [60.0, 137.0, 300.0] {
        runtime.apply_command(EngineCommand::SetBpm(bpm));
        let before = runtime.timeline_position_ms;
        for _ in 0..10 {
            advance(&mut runtime, Duration::from_micros(22_727));
        }
        assert_eq!(runtime.timeline_position_ms - before, 227, "BPM does not scale Timeline milliseconds");
    }
    runtime.last_tick_interval = Duration::MAX;
    assert_eq!(runtime.timeline_tick_delta(), (u64::MAX, 0));
}
