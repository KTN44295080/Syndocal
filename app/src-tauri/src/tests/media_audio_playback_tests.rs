use super::*;

fn write_timeline_audio_test_wav_with_duration(path: &Path, duration_secs: u32) {
    let sample_rate = 8_000_u32;
    let sample_count = sample_rate
        .checked_mul(duration_secs)
        .expect("timeline audio test duration must fit a WAV data chunk");
    let data_bytes = sample_count * 2;
    let mut wav = Vec::with_capacity((44 + data_bytes) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    wav.resize((44 + data_bytes) as usize, 0);
    fs::write(path, wav).unwrap();
}

fn write_timeline_audio_test_wav(path: &Path) {
    write_timeline_audio_test_wav_with_duration(path, 2);
}

fn completed_timeline_audio_prepare_job(
    context: TimelineAudioPrepareContext,
    result: Result<engine::TimelineAudioProjectionAuthority, TimelineAudioPrepareFault>,
) -> TimelineAudioPrepareJob {
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = sender.send(result);
    });
    TimelineAudioPrepareJob {
        context,
        started_at: Instant::now(),
        receiver,
        worker: Some(worker),
        result: None,
        timed_out: false,
        commit_state: Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
    }
}

fn auto_vj_action(sequence: u64, layer_id: VideoLayerId) -> protocol::AutoVjAction {
    protocol::AutoVjAction {
        sequence,
        boundary_index: sequence,
        beat: sequence.saturating_mul(4),
        layer_id,
        transition_ms: 250,
        selection_token: sequence.saturating_mul(17),
        seed: 42,
        show_revision: 7,
        trigger: protocol::AutoVjTrigger::LiveAudioOnset,
        live_audio_feature_sequence: Some(sequence),
    }
}

fn timeline_cue_test_runtime(
    engine_identity: TimelineCueEngineIdentity,
    canonical_anchor_frame: u64,
) -> TimelineCueAudioRuntime {
    timeline_cue_test_runtime_with_observer(engine_identity, canonical_anchor_frame).0
}

fn timeline_cue_test_runtime_with_observer(
    engine_identity: TimelineCueEngineIdentity,
    canonical_anchor_frame: u64,
) -> (TimelineCueAudioRuntime, rodio::mixer::MixerSource) {
    let runtime = TimelineCueAudioRuntime::default();
    let (mixer, observer) = rodio::mixer::mixer(2, 48_000);
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (control, source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        timeline_cue_audio::TimelineCueAuthority {
            fence: timeline_cue_audio::TimelineCueFence {
                output_clock_epoch: 1,
                schedule_generation: engine_identity.schedule_generation,
                source_fence: 1,
            },
            clock: timeline_cue_audio::TimelineCueClockMap {
                canonical_anchor_frame,
                output_anchor_frame: 0,
            },
        },
        1.0,
        1.0,
    )
    .unwrap();
    let source_instance_token = next_timeline_cue_audio_source_instance_token().unwrap();
    let source_test_counters = TimelineCueAudioSourceTestCounters::default();
    source_test_counters.activations.store(1, Ordering::Relaxed);
    mixer.add(source);
    let mut state = runtime.state.lock().unwrap();
    state.attachment = Some(TimelineCueAudioAttachment {
        control,
        output: TimelineCueAudioAttachmentOutput::Legacy {
            mixer,
            _explicit_stream: None,
        },
        pending_source: None,
        source_instance_token,
        source_test_counters,
        cue_test_sinks: Vec::new(),
        route: timeline_cue_audio::TimelineCueAudioRoute::FollowProgram,
        settings_revision: state.settings_revision,
        program_device_generation: None,
        engine_identity,
        output_clock_epoch: 1,
        canonical_anchor_frame,
        source_fence: 1,
        next_sequence: 1,
        last_click_key: None,
        last_guide_generation: None,
        last_guide_sequence: 0,
        blocked_event_identity: None,
        last_observed_output_frame: 0,
        last_output_progress_at: Instant::now(),
    });
    state.lifecycle = TimelineCueAudioLifecycle::Running;
    drop(state);
    (runtime, observer)
}

fn timeline_cue_test_click(
    sample_frame: u64,
    measure: u64,
    beat: u16,
    identity: &TimelineCueEngineIdentity,
) -> protocol::TimelineClickEventSummary {
    protocol::TimelineClickEventSummary {
        sample_frame,
        measure,
        beat,
        numerator: 4,
        denominator: 4,
        downbeat: beat == 1,
        frequency_hz: if beat == 1 { 880 } else { 440 },
        duration_frames: 2_400,
        epoch: identity.epoch,
        transport_generation: identity.transport_generation,
        schedule_generation: identity.schedule_generation,
        source: identity.source,
        ..protocol::TimelineClickEventSummary::default()
    }
}

fn timeline_cue_test_guide(
    sample_frame: u64,
    sequence: u64,
    generation: u64,
    identity: &TimelineCueEngineIdentity,
) -> protocol::TimelineGuideCueSummary {
    protocol::TimelineGuideCueSummary {
        generation,
        sequence,
        at_ms: sample_frame / 48,
        label: format!("Guide {sequence}"),
        cue: protocol::TimelineGuideCueKind::Looping,
        asset: protocol::TimelineGuideAssetKey::Looping,
        playback_rate_milli: 1_000,
        sample_frame,
        epoch: identity.epoch,
        transport_generation: identity.transport_generation,
        schedule_generation: identity.schedule_generation,
        source: identity.source,
    }
}

fn timeline_cue_test_snapshot(
    engine_identity: &TimelineCueEngineIdentity,
    position_ms: u64,
    click_events: Vec<protocol::TimelineClickEventSummary>,
    guide_cues: Vec<protocol::TimelineGuideCueSummary>,
) -> engine::TimelineAudioRuntimeSnapshot {
    engine::TimelineAudioRuntimeSnapshot {
        playing: engine_identity.playing,
        position_ms,
        transport_revision: engine_identity.audio_transport_revision,
        metronome_enabled: engine_identity.metronome_enabled,
        guide_enabled: engine_identity.guide_enabled,
        click_schedule_generation: engine_identity.schedule_generation,
        source_projection_authority: engine::TimelineAudioProjectionAuthority {
            epoch: engine_identity.epoch,
            generation: engine_identity.transport_generation,
        },
        click_events,
        guide_cues,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    }
}

fn timeline_cue_test_identity(playing: bool, transport_revision: u64) -> TimelineCueEngineIdentity {
    TimelineCueEngineIdentity {
        epoch: 41,
        transport_generation: 73,
        schedule_generation: 3,
        source: TimelineScheduleSource::Root,
        audio_transport_revision: transport_revision,
        metronome_enabled: true,
        guide_enabled: true,
        playing,
    }
}

#[test]
fn timeline_cue_rotation_skips_history_and_keeps_exact_and_future_events_once() {
    let paused_identity = timeline_cue_test_identity(false, 1);
    let runtime = timeline_cue_test_runtime(paused_identity.clone(), 0);
    let initial = timeline_cue_test_snapshot(
        &paused_identity,
        0,
        vec![timeline_cue_test_click(0, 1, 1, &paused_identity)],
        vec![timeline_cue_test_guide(0, 1, 1, &paused_identity)],
    );
    runtime.feed_attachment(&initial, None);

    let playing_identity = timeline_cue_test_identity(true, 2);
    let playing = timeline_cue_test_snapshot(
        &playing_identity,
        100,
        vec![
            timeline_cue_test_click(0, 1, 1, &playing_identity),
            timeline_cue_test_click(4_800, 2, 1, &playing_identity),
            timeline_cue_test_click(9_600, 3, 1, &playing_identity),
        ],
        vec![
            timeline_cue_test_guide(0, 1, 1, &playing_identity),
            timeline_cue_test_guide(4_800, 2, 1, &playing_identity),
            timeline_cue_test_guide(9_600, 3, 1, &playing_identity),
        ],
    );
    runtime.feed_attachment(&playing, None);

    {
        let state = runtime.state.lock().unwrap();
        let attachment = state.attachment.as_ref().expect("rotated attachment");
        assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
        assert_eq!(attachment.canonical_anchor_frame, 4_800);
        assert_eq!(attachment.last_click_key, Some((9_600, 3, 1, false)));
        assert_eq!(attachment.last_guide_generation, Some(1));
        assert_eq!(attachment.last_guide_sequence, 3);
        assert_eq!(attachment.next_sequence, 5);
        assert!(attachment.blocked_event_identity.is_none());
    }

    // Re-observing the same snapshot must not enqueue the future pair again.
    runtime.feed_attachment(&playing, None);
    let state = runtime.state.lock().unwrap();
    let attachment = state.attachment.as_ref().expect("attachment remains live");
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
    assert_eq!(attachment.next_sequence, 5);
    assert!(state.last_error.is_none());
}

#[test]
fn timeline_cue_history_only_tick_advances_watermarks_without_reconsidering_history() {
    let identity = timeline_cue_test_identity(true, 2);
    let runtime = timeline_cue_test_runtime(identity.clone(), 4_800);
    let history_only = timeline_cue_test_snapshot(
        &identity,
        100,
        vec![timeline_cue_test_click(0, 1, 1, &identity)],
        vec![timeline_cue_test_guide(0, 1, 1, &identity)],
    );

    runtime.feed_attachment(&history_only, None);
    {
        let state = runtime.state.lock().unwrap();
        let attachment = state.attachment.as_ref().expect("attachment remains live");
        assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
        assert_eq!(attachment.last_click_key, Some((0, 1, 1, false)));
        assert_eq!(attachment.last_guide_generation, Some(1));
        assert_eq!(attachment.last_guide_sequence, 1);
        assert_eq!(attachment.next_sequence, 1);
        assert!(state.last_error.is_none());
    }

    runtime.feed_attachment(&history_only, None);
    let state = runtime.state.lock().unwrap();
    let attachment = state.attachment.as_ref().expect("attachment remains live");
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
    assert_eq!(attachment.next_sequence, 1);
    assert!(state.last_error.is_none());
}

#[test]
fn timeline_cue_seek_rotation_keeps_exact_anchor_and_future_events() {
    let initial_identity = timeline_cue_test_identity(true, 1);
    let runtime = timeline_cue_test_runtime(initial_identity.clone(), 0);
    let initial = timeline_cue_test_snapshot(
        &initial_identity,
        0,
        vec![timeline_cue_test_click(0, 1, 1, &initial_identity)],
        vec![timeline_cue_test_guide(0, 1, 1, &initial_identity)],
    );
    runtime.feed_attachment(&initial, None);

    let seek_identity = TimelineCueEngineIdentity {
        epoch: 42,
        transport_generation: 74,
        schedule_generation: 4,
        source: TimelineScheduleSource::Root,
        audio_transport_revision: 2,
        metronome_enabled: true,
        guide_enabled: true,
        playing: true,
    };
    let seeked = timeline_cue_test_snapshot(
        &seek_identity,
        200,
        vec![
            timeline_cue_test_click(0, 1, 1, &seek_identity),
            timeline_cue_test_click(9_600, 2, 1, &seek_identity),
            timeline_cue_test_click(14_400, 3, 1, &seek_identity),
        ],
        vec![
            timeline_cue_test_guide(0, 1, 2, &seek_identity),
            timeline_cue_test_guide(9_600, 2, 2, &seek_identity),
            timeline_cue_test_guide(14_400, 3, 2, &seek_identity),
        ],
    );
    runtime.feed_attachment(&seeked, None);

    let state = runtime.state.lock().unwrap();
    let attachment = state.attachment.as_ref().expect("seeked attachment");
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
    assert_eq!(attachment.canonical_anchor_frame, 9_600);
    assert_eq!(attachment.last_click_key, Some((14_400, 3, 1, false)));
    assert_eq!(attachment.last_guide_generation, Some(2));
    assert_eq!(attachment.last_guide_sequence, 3);
    assert_eq!(attachment.next_sequence, 5);
    assert!(state.last_error.is_none());
}

#[test]
fn timeline_cue_enqueue_failure_retires_active_source_without_watermark_commit() {
    let identity = timeline_cue_test_identity(true, 1);
    let (runtime, mut observer) = timeline_cue_test_runtime_with_observer(identity.clone(), 0);
    let control_for_assert = {
        let state = runtime.state.lock().unwrap();
        state
            .attachment
            .as_ref()
            .expect("active attachment")
            .control
            .clone()
    };
    assert!(observer.next().is_some());
    assert!(observer.next().is_some());
    assert_eq!(control_for_assert.next_output_frame(), 1);
    let click_events = (0..=timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY)
        .map(|index| timeline_cue_test_click(index as u64, index as u64 + 1, 1, &identity))
        .collect();
    let overflowing = timeline_cue_test_snapshot(&identity, 0, click_events, Vec::new());

    runtime.feed_attachment(&overflowing, None);
    let state = runtime.state.lock().unwrap();
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(state.attachment.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("queue is full")));
    assert_eq!(control_for_assert.next_output_frame(), 1);
    drop(state);
    // Retiring the attachment stops the active mixer source before any stale
    // queued event can be rendered.  The next mixer poll removes that source.
    assert!(observer.next().is_none());

    // A later scheduler tick cannot retry the failed batch because no active
    // attachment remains and its watermarks were never committed.
    runtime.feed_attachment(&overflowing, None);
    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert_eq!(control_for_assert.next_output_frame(), 1);
}

#[test]
fn timeline_cue_source_admission_precedes_callback_progress() {
    let identity = timeline_cue_test_identity(true, 1);
    let authority = timeline_cue_audio::TimelineCueAuthority {
        fence: timeline_cue_audio::TimelineCueFence {
            output_clock_epoch: 1,
            schedule_generation: identity.schedule_generation,
            source_fence: 1,
        },
        clock: timeline_cue_audio::TimelineCueClockMap {
            canonical_anchor_frame: 0,
            output_anchor_frame: 0,
        },
    };
    let exact_event = timeline_cue_audio::TimelineCueEvent {
        canonical_frame: 0,
        sequence: 1,
        kind: timeline_cue_audio::TimelineCueEventKind::Click { accented: true },
        playback_rate_milli: 1_000,
    };

    // This is the rejected ordering: one stereo callback frame starts before
    // admission, so the exact-anchor event is deterministically PastDue.  It
    // is the race that the runtime boundary below must make unreachable.
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (old_control, mut old_source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        authority,
        1.0,
        1.0,
    )
    .unwrap();
    assert!(old_source.next().is_some());
    assert!(old_source.next().is_some());
    assert_eq!(old_control.next_output_frame(), 1);
    let old_error = old_control
        .enqueue_batch(authority.fence, &[exact_event])
        .expect_err("an already-started callback must reject the exact anchor");
    assert!(old_error.contains("lookahead"));

    // The live path keeps its source detached, admits the exact anchor, and
    // only then adds it to the mixer.  Polling the observer after feed proves
    // that the physical callback source is still usable after admission.
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    let (mixer, mut observer) = rodio::mixer::mixer(2, 48_000);
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (control, source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        authority,
        1.0,
        1.0,
    )
    .unwrap();
    let control_for_assert = control.clone();
    let source_instance_token = next_timeline_cue_audio_source_instance_token().unwrap();
    let source_test_counters = TimelineCueAudioSourceTestCounters::default();
    let mut state = runtime.state.lock().unwrap();
    state.attachment = Some(TimelineCueAudioAttachment {
        control,
        output: TimelineCueAudioAttachmentOutput::Legacy {
            mixer,
            _explicit_stream: None,
        },
        pending_source: Some(source),
        source_instance_token,
        source_test_counters,
        cue_test_sinks: Vec::new(),
        route: timeline_cue_audio::TimelineCueAudioRoute::FollowProgram,
        settings_revision: state.settings_revision,
        program_device_generation: None,
        engine_identity: identity.clone(),
        output_clock_epoch: authority.fence.output_clock_epoch,
        canonical_anchor_frame: authority.clock.canonical_anchor_frame,
        source_fence: authority.fence.source_fence,
        next_sequence: 1,
        last_click_key: None,
        last_guide_generation: None,
        last_guide_sequence: 0,
        blocked_event_identity: None,
        last_observed_output_frame: 0,
        last_output_progress_at: Instant::now(),
    });
    state.lifecycle = TimelineCueAudioLifecycle::Running;
    drop(state);

    let snapshot = timeline_cue_test_snapshot(
        &identity,
        0,
        vec![timeline_cue_test_click(0, 1, 1, &identity)],
        Vec::new(),
    );
    let activation_gate = Arc::new((Mutex::new(false), Condvar::new()));
    *TIMELINE_CUE_AUDIO_SOURCE_ACTIVATION_HOOK.lock().unwrap() =
        Some(TimelineCueAudioSourceActivationHook {
            target_source_instance_token: source_instance_token,
            gate: Arc::clone(&activation_gate),
        });
    // A separate runtime with the same value-equal engine identity must not
    // be able to release the target's admission gate.  The source token, not
    // the snapshot identity, is the ownership boundary.
    let second_runtime = Arc::new(TimelineCueAudioRuntime::default());
    let (second_mixer, _second_observer) = rodio::mixer::mixer(2, 48_000);
    let second_assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (second_control, second_source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        second_assets,
        authority,
        1.0,
        1.0,
    )
    .unwrap();
    let second_token = next_timeline_cue_audio_source_instance_token().unwrap();
    assert_ne!(second_token, source_instance_token);
    let second_counters = TimelineCueAudioSourceTestCounters::default();
    let mut second_state = second_runtime.state.lock().unwrap();
    second_state.attachment = Some(TimelineCueAudioAttachment {
        control: second_control,
        output: TimelineCueAudioAttachmentOutput::Legacy {
            mixer: second_mixer,
            _explicit_stream: None,
        },
        pending_source: Some(second_source),
        source_instance_token: second_token,
        source_test_counters: second_counters.clone(),
        cue_test_sinks: Vec::new(),
        route: timeline_cue_audio::TimelineCueAudioRoute::FollowProgram,
        settings_revision: second_state.settings_revision,
        program_device_generation: None,
        engine_identity: identity.clone(),
        output_clock_epoch: authority.fence.output_clock_epoch,
        canonical_anchor_frame: authority.clock.canonical_anchor_frame,
        source_fence: authority.fence.source_fence,
        next_sequence: 1,
        last_click_key: None,
        last_guide_generation: None,
        last_guide_sequence: 0,
        blocked_event_identity: None,
        last_observed_output_frame: 0,
        last_output_progress_at: Instant::now(),
    });
    second_state.lifecycle = TimelineCueAudioLifecycle::Applying;
    drop(second_state);
    second_runtime.feed_attachment(&snapshot, None);
    assert_eq!(second_counters.activations.load(Ordering::Relaxed), 1);
    let (gate_state, _) = &*activation_gate;
    assert!(!*gate_state.lock().unwrap());

    let runtime_for_feed = Arc::clone(&runtime);
    let feed_thread = std::thread::spawn(move || {
        runtime_for_feed.feed_attachment(&snapshot, None);
    });
    let (gate_state, gate_ready) = &*activation_gate;
    let gate_guard = gate_state.lock().unwrap();
    let (gate_guard, _) = gate_ready
        .wait_timeout_while(gate_guard, Duration::from_secs(1), |released| !*released)
        .unwrap();
    let source_was_activated = *gate_guard;
    drop(gate_guard);
    let callback_observed = if source_was_activated {
        let first = observer.next().is_some();
        let second = observer.next().is_some();
        first && second
    } else {
        false
    };
    let callback_frame = control_for_assert.next_output_frame();
    let mut gate_guard = gate_state.lock().unwrap();
    *gate_guard = false;
    gate_ready.notify_all();
    drop(gate_guard);
    feed_thread.join().unwrap();
    *TIMELINE_CUE_AUDIO_SOURCE_ACTIVATION_HOOK.lock().unwrap() = None;
    assert!(
        source_was_activated,
        "source activation hook was not reached"
    );
    assert!(callback_observed, "activated source did not reach callback");
    assert_eq!(callback_frame, 1);

    let state = runtime.state.lock().unwrap();
    let attachment = state.attachment.as_ref().expect("attachment remains live");
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Running);
    assert!(attachment.pending_source.is_none());
    assert_eq!(attachment.last_click_key, Some((0, 1, 1, false)));
    assert_eq!(control_for_assert.next_output_frame(), callback_frame);
    drop(state);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn timeline_cue_source_activation_failure_retires_without_watermark_commit() {
    let identity = timeline_cue_test_identity(true, 1);
    let output_identity = asio_timeline_output::TimelineOutputIdentity::new(
        1,
        1,
        protocol::TimelineAudioOutputBus::Cue,
    )
    .unwrap();
    let authority = timeline_cue_audio::TimelineCueAuthority {
        fence: timeline_cue_audio::TimelineCueFence {
            output_clock_epoch: 1,
            schedule_generation: identity.schedule_generation,
            source_fence: 1,
        },
        clock: timeline_cue_audio::TimelineCueClockMap {
            canonical_anchor_frame: 0,
            output_anchor_frame: 0,
        },
    };
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (control, source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        authority,
        1.0,
        1.0,
    )
    .unwrap();
    let control_for_assert = control.clone();
    let source_instance_token = next_timeline_cue_audio_source_instance_token().unwrap();
    let source_test_counters = TimelineCueAudioSourceTestCounters::default();
    let runtime = TimelineCueAudioRuntime::default();
    let (mixer, _observer) = rodio::mixer::mixer(2, 48_000);
    let mut state = runtime.state.lock().unwrap();
    state.attachment = Some(TimelineCueAudioAttachment {
        control,
        output: TimelineCueAudioAttachmentOutput::AsioCue {
            identity: output_identity,
            sink: None,
        },
        pending_source: Some(source),
        source_instance_token,
        source_test_counters,
        cue_test_sinks: Vec::new(),
        route: timeline_cue_audio::TimelineCueAudioRoute::ExplicitDevice,
        settings_revision: state.settings_revision,
        program_device_generation: None,
        engine_identity: identity.clone(),
        output_clock_epoch: authority.fence.output_clock_epoch,
        canonical_anchor_frame: authority.clock.canonical_anchor_frame,
        source_fence: authority.fence.source_fence,
        next_sequence: 1,
        last_click_key: None,
        last_guide_generation: None,
        last_guide_sequence: 0,
        blocked_event_identity: None,
        last_observed_output_frame: 0,
        last_output_progress_at: Instant::now(),
    });
    state.lifecycle = TimelineCueAudioLifecycle::Running;
    drop(state);

    let snapshot = timeline_cue_test_snapshot(
        &identity,
        0,
        vec![timeline_cue_test_click(0, 1, 1, &identity)],
        Vec::new(),
    );
    // `feed_attachment` has no ASIO owner, so admission succeeds but source
    // activation cannot.  The attachment is retired instead of being kept
    // with advanced watermarks or a detached source that could be retried.
    runtime.feed_attachment(&snapshot, None);

    let state = runtime.state.lock().unwrap();
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(state.attachment.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("ASIO output runtime owner")));
    assert_eq!(control_for_assert.next_output_frame(), 0);
    drop(state);
    drop(mixer);
}

#[test]
fn empty_audio_monitor_status_does_not_open_an_output_device() {
    let mut playback = MediaAudioPlayback::default();
    playback.stop(99);

    let status = playback.status();

    assert!(!status.output_open);
    assert!(status.active_layer_ids.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn normal_program_retirement_owner_remains_reachable_after_media_lock_poison() {
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let poison_target = Arc::clone(&audio);
    let _ = std::panic::catch_unwind(move || {
        let _guard = poison_target.lock().unwrap();
        panic!("poison media playback for retirement-owner regression proof");
    });

    with_media_audio_retirement_owner(&audio, |playback| {
        playback.last_sync_error = Some("retirement owner retained".to_string());
    });
    let playback = match audio.lock() {
        Ok(_) => panic!("media playback mutex unexpectedly recovered its poison state"),
        Err(poisoned) => poisoned.into_inner(),
    };
    assert_eq!(
        playback.last_sync_error.as_deref(),
        Some("retirement owner retained")
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn rejected_program_install_keeps_prepared_output_owned_by_retirement_guard() {
    let (active_mixer, _) = rodio::mixer::mixer(2, 48_000);
    let mut playback = MediaAudioPlayback::default();
    playback
        .sinks
        .insert(7, rodio::Sink::connect_new(&active_mixer));
    let (prepared_mixer, _) = rodio::mixer::mixer(2, 48_000);
    let mut prepared = Some(PreparedMediaAudioOutput::Test {
        mixer: prepared_mixer,
    });

    let error = playback.install_prepared_output(&mut prepared).unwrap_err();

    assert!(error.contains("cannot stop active sinks"));
    assert!(
        prepared.is_some(),
        "rejected lease must remain with its guard"
    );
    assert_eq!(playback.sinks.len(), 1);
}

#[test]
fn audio_device_generation_exhaustion_is_fail_closed_without_runtime_delta() {
    let mut playback = MediaAudioPlayback {
        audio_device_generation: u64::MAX,
        requested_device_name: Some("Existing output".to_string()),
        timeline_source_projection_authority: Some(engine::TimelineAudioProjectionAuthority {
            epoch: 7,
            generation: 9,
        }),
        ..MediaAudioPlayback::default()
    };
    playback.timeline_failures.insert(
        TimelineAudioSinkKey::Root(1),
        TimelineAudioPlaybackFailure {
            source: TimelineAudioSourceConfig {
                path: PathBuf::from("existing.wav"),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 1_000,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
            error: "existing failure".to_string(),
        },
    );
    let before = (
        playback.audio_device_generation,
        playback.requested_device_name.clone(),
        playback.timeline_source_projection_authority,
        playback.timeline_failures.clone(),
    );

    let error = playback.next_audio_device_generation().unwrap_err();

    assert!(error.contains("generation is exhausted"));
    assert_eq!(
        (
            playback.audio_device_generation,
            playback.requested_device_name.clone(),
            playback.timeline_source_projection_authority,
            playback.timeline_failures.clone(),
        ),
        before,
        "overflow must be rejected before stream/sink/source mutation"
    );
}

#[test]
fn timeline_audio_prepare_stall_releases_control_lock_and_rejects_device_aba() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let worker_engine = engine.clone();
    let worker_audio = Arc::clone(&audio);
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        sync_timeline_audio_without_blocking_playback_lock(
            &worker_engine,
            &worker_audio,
            #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
            None,
            #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
            None,
            #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
            None,
            &timeline,
            &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
            || {
                entered_tx.send(()).unwrap();
                release_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            },
            || {},
        )
    });
    entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
    let lock_started = Instant::now();
    {
        let mut playback = audio
            .try_lock()
            .expect("prepare hook must not hold the media_audio mutex");
        assert!(lock_started.elapsed() < Duration::from_millis(50));
        playback.audio_device_generation = playback.audio_device_generation.wrapping_add(1).max(1);
    }
    release_tx.send(()).unwrap();
    let error = worker.join().unwrap().unwrap_err();
    assert!(error
        .to_string()
        .contains("output device changed during preparation"));
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.timeline_start_attempt_count, 0);
}

#[test]
fn timeline_audio_real_wav_prepare_survives_multiple_position_ticks_and_installs_once() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!(
        "syndocal-timeline-audio-semantic-generation-{suffix}.wav"
    ));
    write_timeline_audio_test_wav(&path);
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    engine
        .add_timeline_layer(protocol::TimelineLayerSummary {
            id: 44,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: true,
            kind: TimelineLayerKind::Audio,
        })
        .unwrap();
    engine
        .add_timeline_audio_clip(TimelineAudioClipSummary {
            id: 701,
            layer_id: 44,
            media_asset_id: None,
            path: path.to_string_lossy().into_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        })
        .unwrap();
    let transport = engine.snapshot().timeline;
    engine
        .set_timeline_playing_published(
            transport.transport_epoch,
            transport.transport_generation,
            true,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(timeline.playing);
    assert_eq!(timeline.clips.len(), 1);
    let captured_generation = timeline.publication_generation;

    let (mixer, _unconsumed_source) = rodio::mixer::mixer(1, 8_000);
    let audio = Arc::new(Mutex::new(MediaAudioPlayback {
        timeline_test_mixer: Some(mixer),
        ..MediaAudioPlayback::default()
    }));
    let latest_position_ms = AtomicU64::new(0);
    let mut contention = None;
    sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {
            std::thread::sleep(Duration::from_millis(120));
            let current = engine.video_audio_runtime_snapshot().timeline_audio;
            assert!(current.position_ms > timeline.position_ms);
            latest_position_ms.store(current.position_ms, Ordering::Release);
            assert_eq!(
                current.publication_generation, captured_generation,
                "ordinary 44 Hz ticks inside the same active WAV must not invalidate prepare"
            );
        },
        || {
            let held_audio = Arc::clone(&audio);
            let (entered_tx, entered_rx) = mpsc::sync_channel(1);
            contention = Some(std::thread::spawn(move || {
                let _guard = held_audio.lock().unwrap();
                entered_tx.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(200));
            }));
            entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        },
    )
    .unwrap();
    contention.take().unwrap().join().unwrap();
    latest_position_ms.store(
        engine
            .video_audio_runtime_snapshot()
            .timeline_audio
            .position_ms,
        Ordering::Release,
    );
    let playback = audio.lock().unwrap();
    assert_eq!(playback.timeline_start_attempt_count, 1);
    assert_eq!(playback.timeline_sinks.len(), 1);
    let installed_source_position_ms = playback
        .timeline_last_install_source_position_ms
        .get(&TimelineAudioSinkKey::Root(701))
        .copied()
        .unwrap();
    assert!(
        installed_source_position_ms.abs_diff(latest_position_ms.load(Ordering::Acquire)) <= 50,
        "a decoder that spans ordinary ticks must start from the latest position"
    );
    drop(playback);
    let _ = fs::remove_file(path);
}

#[test]
fn timeline_audio_decoder_seek_before_append_does_not_wait_for_mixer_callback() {
    let suffix = current_unix_ms();
    let path =
        std::env::temp_dir().join(format!("syndocal-timeline-audio-bounded-seek-{suffix}.wav"));
    write_timeline_audio_test_wav(&path);
    let (mixer, _unconsumed_source) = rodio::mixer::mixer(1, 8_000);
    let prepared = prepare_timeline_audio_clip(
        TimelineAudioPrepareRequest {
            key: TimelineAudioSinkKey::Root(702),
            clip: TimelineAudioClipSummary {
                id: 702,
                layer_id: 44,
                media_asset_id: None,
                path: path.to_string_lossy().into_owned(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 2_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
            source_position_ms: 100,
            volume: 1.0,
            source: TimelineAudioSourceConfig {
                path: path.clone(),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 2_000,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
        },
        &mixer,
    )
    .unwrap();
    let started = Instant::now();
    let mut prepared = seek_prepared_timeline_audio_decoders(vec![Ok(prepared)]);
    assert!(started.elapsed() < Duration::from_millis(250));
    prepared = append_prepared_timeline_audio_decoders(
        prepared,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
    );
    let mut prepared = prepared.pop().unwrap().unwrap();
    assert_eq!(prepared.request.source_position_ms, 100);
    assert_eq!(prepared.sink.as_ref().unwrap().speed(), 2.0);
    assert!(prepared.decoder.is_none());
    prepared.stop();
    let _ = fs::remove_file(path);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn asio_timeline_audio_attachment_applies_authoritative_varispeed() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!("syndocal-asio-timeline-varispeed-{suffix}.wav"));
    write_timeline_audio_test_wav(&path);
    let (runtime, _context, _program, _cue) =
        asio_output_runtime::AsioOutputRuntime::active_in_memory(714, 73).unwrap();
    let runtime = Arc::new(Mutex::new(runtime));
    let request = TimelineAudioPrepareRequest {
        key: TimelineAudioSinkKey::Root(704),
        clip: TimelineAudioClipSummary {
            id: 704,
            layer_id: 44,
            media_asset_id: None,
            path: path.to_string_lossy().into_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        },
        source_position_ms: 0,
        volume: 1.0,
        source: TimelineAudioSourceConfig {
            path: path.clone(),
            gain: 1.0,
            offset_ms: 0,
            speed_milli: 1_500,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        },
    };

    let prepared = prepare_asio_timeline_audio_clip(request, &runtime).unwrap();
    let mut results = append_prepared_timeline_audio_decoders(
        seek_prepared_timeline_audio_decoders(vec![Ok(prepared)]),
        Some(&runtime),
        None,
        None,
    );
    let mut prepared = results.pop().unwrap().unwrap();
    assert_eq!(prepared.sink.as_ref().unwrap().speed(), 1.5);
    prepared.stop();
    let _ = fs::remove_file(path);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_stop_between_append_and_publication_retires_the_cue_sink() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!(
        "syndocal-explicit-wdm-stop-append-race-{suffix}.wav"
    ));
    write_timeline_audio_test_wav(&path);

    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::ExplicitWdm {
            device_name: "Race Headphones".to_string(),
            topology_fingerprint: "race-topology".to_string(),
        })
        .unwrap();

    let output_clock_epoch = 23;
    let (mixer, _unconsumed_source) = rodio::mixer::mixer(2, 48_000);
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (control, cue_source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        timeline_cue_audio::TimelineCueAuthority {
            fence: timeline_cue_audio::TimelineCueFence {
                output_clock_epoch,
                schedule_generation: 1,
                source_fence: 1,
            },
            clock: timeline_cue_audio::TimelineCueClockMap {
                canonical_anchor_frame: 0,
                output_anchor_frame: 0,
            },
        },
        1.0,
        1.0,
    )
    .unwrap();
    mixer.add(cue_source);
    {
        let mut state = runtime.state.lock().unwrap();
        state.resolved_device_name = Some("Race Headphones".to_owned());
        state.observed_topology_fingerprint = Some("race-topology".to_owned());
        state.attachment = Some(TimelineCueAudioAttachment {
            control,
            output: TimelineCueAudioAttachmentOutput::Legacy {
                mixer: mixer.clone(),
                _explicit_stream: None,
            },
            pending_source: None,
            source_instance_token: next_timeline_cue_audio_source_instance_token().unwrap(),
            source_test_counters: TimelineCueAudioSourceTestCounters::default(),
            cue_test_sinks: Vec::new(),
            route: timeline_cue_audio::TimelineCueAudioRoute::ExplicitDevice,
            settings_revision: state.settings_revision,
            program_device_generation: None,
            engine_identity: TimelineCueEngineIdentity {
                epoch: 1,
                transport_generation: 1,
                schedule_generation: 1,
                source: TimelineScheduleSource::Root,
                audio_transport_revision: 1,
                metronome_enabled: true,
                guide_enabled: true,
                playing: true,
            },
            output_clock_epoch,
            canonical_anchor_frame: 0,
            source_fence: 1,
            next_sequence: 1,
            last_click_key: None,
            last_guide_generation: None,
            last_guide_sequence: 0,
            blocked_event_identity: None,
            last_observed_output_frame: 0,
            last_output_progress_at: Instant::now(),
        });
        state.next_output_clock_epoch = output_clock_epoch;
        state.lifecycle = TimelineCueAudioLifecycle::Running;
    }

    let output = runtime.explicit_wdm_output_snapshot().unwrap();
    let prepared = prepare_explicit_wdm_timeline_audio_clip(
        TimelineAudioPrepareRequest {
            key: TimelineAudioSinkKey::Root(703),
            clip: TimelineAudioClipSummary {
                id: 703,
                layer_id: 44,
                media_asset_id: None,
                path: path.to_string_lossy().into_owned(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 2_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Cue,
            },
            source_position_ms: 0,
            volume: 1.0,
            source: TimelineAudioSourceConfig {
                path: path.clone(),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 500,
                output_bus: protocol::TimelineAudioOutputBus::Cue,
            },
        },
        &output,
    )
    .unwrap();
    assert_eq!(prepared.sink.as_ref().unwrap().speed(), 0.5);
    let prepared = seek_prepared_timeline_audio_decoders(vec![Ok(prepared)]);
    let runtime_for_stop = Arc::clone(&runtime);
    let mut stop_between_append_and_postcheck = move || {
        runtime_for_stop
            .close_normal_routes_for_asio_start()
            .unwrap();
    };
    let mut results = append_prepared_timeline_audio_decoders(
        prepared,
        None,
        Some(&runtime),
        Some(&mut stop_between_append_and_postcheck),
    );
    let error = match results.pop().unwrap() {
        Ok(mut prepared) => {
            prepared.stop();
            panic!("a retired Explicit WDM CUE route must reject the prepared sink")
        }
        Err(error) => error.1,
    };
    assert!(error.contains("changed during Timeline source attachment"));
    assert!(runtime.explicit_wdm_output_snapshot().is_none());
    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(!state.normal_admission_open);
    drop(state);
    let _ = fs::remove_file(path);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
fn published_explicit_wdm_cue_test_runtime() -> Arc<TimelineCueAudioRuntime> {
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::ExplicitWdm {
            device_name: "Test Headphones".to_owned(),
            topology_fingerprint: "test-topology".to_owned(),
        })
        .unwrap();

    let output_clock_epoch = 23;
    let (mixer, _unconsumed_source) = rodio::mixer::mixer(2, 48_000);
    let assets = timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap();
    let (control, cue_source) = timeline_cue_audio::create_timeline_cue_audio_source(
        48_000,
        2,
        timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY,
        assets,
        timeline_cue_audio::TimelineCueAuthority {
            fence: timeline_cue_audio::TimelineCueFence {
                output_clock_epoch,
                schedule_generation: 1,
                source_fence: 1,
            },
            clock: timeline_cue_audio::TimelineCueClockMap {
                canonical_anchor_frame: 0,
                output_anchor_frame: 0,
            },
        },
        1.0,
        1.0,
    )
    .unwrap();
    mixer.add(cue_source);
    let mut state = runtime.state.lock().unwrap();
    state.resolved_device_name = Some("Test Headphones".to_owned());
    state.observed_topology_fingerprint = Some("test-topology".to_owned());
    state.next_output_clock_epoch = output_clock_epoch;
    state.attachment = Some(TimelineCueAudioAttachment {
        control,
        output: TimelineCueAudioAttachmentOutput::Legacy {
            mixer,
            _explicit_stream: None,
        },
        pending_source: None,
        source_instance_token: next_timeline_cue_audio_source_instance_token().unwrap(),
        source_test_counters: TimelineCueAudioSourceTestCounters::default(),
        cue_test_sinks: Vec::new(),
        route: timeline_cue_audio::TimelineCueAudioRoute::ExplicitDevice,
        settings_revision: state.settings_revision,
        program_device_generation: None,
        engine_identity: TimelineCueEngineIdentity {
            epoch: 1,
            transport_generation: 1,
            schedule_generation: 1,
            source: TimelineScheduleSource::Root,
            audio_transport_revision: 1,
            metronome_enabled: true,
            guide_enabled: true,
            playing: false,
        },
        output_clock_epoch,
        canonical_anchor_frame: 0,
        source_fence: 1,
        next_sequence: 1,
        last_click_key: None,
        last_guide_generation: None,
        last_guide_sequence: 0,
        blocked_event_identity: None,
        last_observed_output_frame: 0,
        last_output_progress_at: Instant::now(),
    });
    state.lifecycle = TimelineCueAudioLifecycle::Running;
    drop(state);
    runtime
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn external_wdm_timeline_owner_separates_normal_authoring_from_asio_cue() {
    let asio_cue = published_explicit_wdm_cue_test_runtime();
    let asio_snapshot = asio_cue
        .explicit_wdm_output_snapshot()
        .expect("published Explicit WDM CUE output must be observable");
    assert_eq!(asio_snapshot.owner, ExplicitWdmTimelineOwner::AsioCue);
    assert_eq!(asio_snapshot.output_clock_epoch, 23);
    assert!(matches!(
        asio_cue.external_wdm_timeline_route_fence(),
        Some(ExternalWdmTimelineRouteFence {
            owner: Some(ExplicitWdmTimelineOwner::AsioCue),
            generation: 23,
        })
    ));

    let normal_authoring = published_explicit_wdm_cue_test_runtime();
    {
        let mut state = normal_authoring.state.lock().unwrap();
        state.active_asio_cue_delivery = None;
        state.normal_admission_open = true;
        let settings = timeline_cue_audio::MachineTimelineCueAudioSettingsV1 {
            route: timeline_cue_audio::TimelineCueAudioRoute::ExplicitDevice,
            device_name: Some("Test Headphones".to_owned()),
            topology_fingerprint: Some("test-topology".to_owned()),
            ..timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()
        };
        state.desired = settings.clone();
        state.applied = Some(settings);
        state.blocked_settings_revision = None;
        state.resolved_device_name = Some("Test Headphones".to_owned());
        state.observed_topology_fingerprint = Some("test-topology".to_owned());
        state.lifecycle = TimelineCueAudioLifecycle::Running;
    }

    let normal_snapshot = match normal_authoring
        .normal_authoring_route_snapshot_blocking()
        .expect("Normal authoring route must be observable")
    {
        NormalAuthoringRouteSnapshot::ExplicitWdm(Some(snapshot)) => snapshot,
        NormalAuthoringRouteSnapshot::FollowProgram => {
            panic!("explicit Normal authoring route must not follow Program")
        }
        NormalAuthoringRouteSnapshot::ExplicitWdm(None) => {
            panic!("exact Normal authoring WDM output must be observable")
        }
    };
    assert_eq!(
        normal_snapshot.owner,
        ExplicitWdmTimelineOwner::NormalAuthoring
    );
    assert_eq!(normal_snapshot.output_clock_epoch, 23);
    assert!(matches!(
        normal_authoring.external_wdm_timeline_route_fence(),
        Some(ExternalWdmTimelineRouteFence {
            owner: Some(ExplicitWdmTimelineOwner::NormalAuthoring),
            generation: 23,
        })
    ));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn normal_authoring_monitor_routes_program_and_cue_media_to_one_explicit_wdm_owner() {
    let suffix = current_unix_ms();
    let program_path =
        std::env::temp_dir().join(format!("syndocal-authoring-monitor-program-{suffix}.wav"));
    let cue_path =
        std::env::temp_dir().join(format!("syndocal-authoring-monitor-cue-{suffix}.wav"));
    write_timeline_audio_test_wav(&program_path);
    write_timeline_audio_test_wav(&cue_path);

    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    for (id, label, order) in [(44, "Program", 0), (45, "Cue", 1)] {
        engine
            .add_timeline_layer(protocol::TimelineLayerSummary {
                id,
                label: label.to_owned(),
                order,
                muted: false,
                locked: false,
                solo: false,
                expanded: true,
                kind: TimelineLayerKind::Audio,
            })
            .unwrap();
    }
    for (id, layer_id, path, output_bus) in [
        (
            711,
            44,
            &program_path,
            protocol::TimelineAudioOutputBus::Program,
        ),
        (712, 45, &cue_path, protocol::TimelineAudioOutputBus::Cue),
    ] {
        engine
            .add_timeline_audio_clip(TimelineAudioClipSummary {
                id,
                layer_id,
                media_asset_id: None,
                path: path.to_string_lossy().into_owned(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 2_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus,
            })
            .unwrap();
    }
    let transport = engine.snapshot().timeline;
    engine
        .set_timeline_playing_published(
            transport.transport_epoch,
            transport.transport_generation,
            true,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;

    let timeline_cue_audio = published_explicit_wdm_cue_test_runtime();
    {
        let mut state = timeline_cue_audio.state.lock().unwrap();
        state.active_asio_cue_delivery = None;
        state.normal_admission_open = true;
        let settings = timeline_cue_audio::MachineTimelineCueAudioSettingsV1 {
            route: timeline_cue_audio::TimelineCueAudioRoute::ExplicitDevice,
            device_name: Some("Test Headphones".to_owned()),
            topology_fingerprint: Some("test-topology".to_owned()),
            ..timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()
        };
        state.desired = settings.clone();
        state.applied = Some(settings);
        state.blocked_settings_revision = None;
        state.last_topology_probe_at = Instant::now();
        state.lifecycle = TimelineCueAudioLifecycle::Running;
    }
    let router = audio_output_router::RouterSlot::test_slot();
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    // Media-only Timelines still require the exact authoring WDM attachment;
    // Guide and Click are optional and both are disabled in this fixture.
    timeline_cue_audio.sync_normal(&timeline, &audio);
    assert!(timeline_cue_audio
        .state
        .lock()
        .unwrap()
        .attachment
        .is_some());
    let authoring_fence = timeline_cue_audio
        .external_wdm_timeline_route_fence()
        .expect("the exact authoring monitor publishes its route fence");
    audio
        .lock()
        .unwrap()
        .observe_external_wdm_timeline_route(Some(authoring_fence));
    sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        Some(router),
        None,
        Some(Arc::clone(&timeline_cue_audio)),
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {},
        || {},
    )
    .unwrap();

    let playback = audio.lock().unwrap();
    assert!(playback.normal_output.is_none());
    assert_eq!(playback.timeline_sinks.len(), 2);
    assert_eq!(playback.timeline_sources.len(), 2);
    assert!(playback
        .timeline_sources
        .values()
        .any(|source| { source.output_bus == protocol::TimelineAudioOutputBus::Program }));
    assert!(playback
        .timeline_sources
        .values()
        .any(|source| source.output_bus == protocol::TimelineAudioOutputBus::Cue));
    assert_eq!(
        playback.external_wdm_timeline_owner,
        Some(ExplicitWdmTimelineOwner::NormalAuthoring)
    );
    assert_eq!(
        playback.hybrid_cue_output_generation,
        authoring_fence.generation
    );
    drop(playback);

    timeline_cue_audio
        .close_normal_routes_for_asio_start()
        .unwrap();
    retire_active_normal_program_output(&audio, "authoring monitor ASIO Start proof", true)
        .unwrap();
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert!(playback.timeline_sources.is_empty());
    assert_eq!(playback.external_wdm_timeline_owner, None);
    assert_eq!(playback.hybrid_cue_output_generation, 0);
    drop(playback);

    let _ = fs::remove_file(program_path);
    let _ = fs::remove_file(cue_path);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
fn poison_timeline_cue_audio_state(runtime: &Arc<TimelineCueAudioRuntime>) {
    let runtime = Arc::clone(runtime);
    let result = std::panic::catch_unwind(move || {
        let _state = runtime.state.lock().unwrap();
        panic!("poison Timeline CUE audio state for retirement proof");
    });
    assert!(result.is_err());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
fn poison_timeline_cue_audio_admission(runtime: &Arc<TimelineCueAudioRuntime>) {
    let runtime = Arc::clone(runtime);
    let result = std::panic::catch_unwind(move || {
        let _admission = runtime.settings_update.lock().unwrap();
        panic!("poison Timeline CUE audio admission for Stop proof");
    });
    assert!(result.is_err());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
fn poison_asio_output_runtime(runtime: &Arc<Mutex<asio_output_runtime::AsioOutputRuntime>>) {
    let runtime = Arc::clone(runtime);
    let result = std::panic::catch_unwind(move || {
        let _runtime = runtime.lock().unwrap();
        panic!("poison ASIO output runtime for Stop proof");
    });
    assert!(result.is_err());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
fn add_test_timeline_audio_sink(playback: &mut MediaAudioPlayback, key: u64) {
    let (mixer, _) = rodio::mixer::mixer(2, 48_000);
    let sink = rodio::Sink::connect_new(&mixer);
    sink.append(rodio::buffer::SamplesBuffer::new(
        2,
        48_000,
        vec![0.25_f32; 9_600],
    ));
    sink.play();
    playback
        .timeline_sinks
        .insert(TimelineAudioSinkKey::Root(key), sink);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn faulted_timeline_cue_state_retires_stale_media_timeline_sinks() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    {
        let mut playback = audio.lock().unwrap();
        playback.hybrid_cue_output_generation = 23;
        playback.external_wdm_timeline_owner = Some(ExplicitWdmTimelineOwner::AsioCue);
        add_test_timeline_audio_sink(&mut playback, 996);
    }
    runtime.state.lock().unwrap().lifecycle = TimelineCueAudioLifecycle::Fault;
    assert_eq!(runtime.external_wdm_timeline_route_fence(), None);

    let fence = runtime.external_wdm_timeline_route_fence();
    audio
        .lock()
        .unwrap()
        .observe_external_wdm_timeline_route(fence);
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.hybrid_cue_output_generation, 0);
    assert_eq!(playback.external_wdm_timeline_owner, None);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_timeline_cue_state_retires_stale_media_timeline_sinks() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    {
        let mut playback = audio.lock().unwrap();
        playback.hybrid_cue_output_generation = 23;
        playback.external_wdm_timeline_owner = Some(ExplicitWdmTimelineOwner::AsioCue);
        add_test_timeline_audio_sink(&mut playback, 997);
    }
    poison_timeline_cue_audio_state(&runtime);
    assert_eq!(runtime.external_wdm_timeline_route_fence(), None);

    audio
        .lock()
        .unwrap()
        .observe_external_wdm_timeline_route(None);
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.hybrid_cue_output_generation, 0);
    assert_eq!(playback.external_wdm_timeline_owner, None);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn external_wdm_commit_rejects_an_old_playback_fence_after_observer_delay() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let stale_fence = runtime
        .external_wdm_timeline_route_fence()
        .expect("published Explicit WDM route must have a fence");
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    {
        let mut playback = audio.lock().unwrap();
        playback.observe_external_wdm_timeline_route(Some(stale_fence));
        add_test_timeline_audio_sink(&mut playback, 998);
        playback.timeline_sources.insert(
            TimelineAudioSinkKey::Root(998),
            TimelineAudioSourceConfig {
                path: PathBuf::from("cue.wav"),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 1_000,
                output_bus: protocol::TimelineAudioOutputBus::Cue,
            },
        );
    }

    // Model the observer being delayed while the runtime rotates its exact
    // output epoch. The old playback owner/plan must not be admitted through
    // the commit fence during that interval.
    runtime
        .state
        .lock()
        .unwrap()
        .attachment
        .as_mut()
        .unwrap()
        .output_clock_epoch = 24;
    let error = runtime
        .lock_external_wdm_timeline_commit(stale_fence)
        .expect_err("a stale observer fence must reject Timeline sink commit");
    assert!(error.contains("changed before sink commit"));
    {
        let playback = audio.lock().unwrap();
        assert_eq!(
            playback.hybrid_cue_output_generation,
            stale_fence.generation
        );
        assert_eq!(playback.external_wdm_timeline_owner, stale_fence.owner);
        assert_eq!(playback.timeline_sinks.len(), 1);
    }

    // Once the observer catches up, the generation transition retires the
    // old CUE sink and leaves the playback state fail-closed until republish.
    let current_fence = runtime
        .external_wdm_timeline_route_fence()
        .expect("the rotated Running route remains observable");
    audio
        .lock()
        .unwrap()
        .observe_external_wdm_timeline_route(Some(current_fence));
    assert!(audio.lock().unwrap().timeline_sinks.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn settings_writer_linearizes_before_an_waiting_external_wdm_media_commit() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let stale_fence = runtime
        .external_wdm_timeline_route_fence()
        .expect("published Explicit WDM route must have a fence");
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));

    // Freeze the lifecycle state so the real settings writer can acquire the
    // route gate and block on state. A final media commit started afterward
    // must wait for that writer and then reject the retired fence.
    let state = runtime.state.lock().unwrap();
    let mut next_settings = state.desired.clone();
    next_settings.click_gain = 0.75;
    let writer_runtime = Arc::clone(&runtime);
    let writer = std::thread::spawn(move || writer_runtime.publish_settings(next_settings));

    let wait_started = Instant::now();
    loop {
        match runtime.external_wdm_timeline_route_gate.try_lock() {
            Err(TryLockError::WouldBlock) => break,
            Err(TryLockError::Poisoned(_)) => panic!("route gate was poisoned"),
            Ok(route_gate) => drop(route_gate),
        }
        assert!(
            wait_started.elapsed() < Duration::from_secs(1),
            "settings writer did not acquire the route gate"
        );
        std::thread::yield_now();
    }

    let commit_runtime = Arc::clone(&runtime);
    let commit_audio = Arc::clone(&audio);
    let commit = std::thread::spawn(move || {
        let guard = commit_runtime.lock_external_wdm_timeline_commit(stale_fence)?;
        add_test_timeline_audio_sink(&mut commit_audio.lock().unwrap(), 1_001);
        drop(guard);
        Ok::<(), String>(())
    });

    drop(state);
    assert!(writer.join().unwrap().is_ok());
    let error = commit
        .join()
        .unwrap()
        .expect_err("commit must observe the settings writer's retired route");
    assert!(error.contains("changed before sink commit"));
    assert!(audio.lock().unwrap().timeline_sinks.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn worker_tick_route_gate_contention_drains_then_republishes_the_exact_fence() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let published = runtime
        .external_wdm_timeline_route_fence()
        .expect("published Explicit WDM route must have a fence");
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    {
        let mut playback = audio.lock().unwrap();
        playback.observe_external_wdm_timeline_route(Some(published));
        add_test_timeline_audio_sink(&mut playback, 1_002);
    }

    let route_gate = runtime.external_wdm_timeline_route_gate.lock().unwrap();
    runtime.observe_external_wdm_timeline_route_on_worker_tick(&audio);
    {
        let playback = audio.lock().unwrap();
        assert!(playback.timeline_sinks.is_empty());
        assert_eq!(playback.external_wdm_timeline_owner, None);
        assert_eq!(playback.hybrid_cue_output_generation, 0);
    }
    drop(route_gate);

    runtime.observe_external_wdm_timeline_route_on_worker_tick(&audio);
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.external_wdm_timeline_owner, published.owner);
    assert_eq!(playback.hybrid_cue_output_generation, published.generation);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn normal_program_replacement_preserves_only_the_independent_authoring_timeline() {
    let mut authoring = MediaAudioPlayback {
        hybrid_cue_output_generation: 23,
        external_wdm_timeline_owner: Some(ExplicitWdmTimelineOwner::NormalAuthoring),
        ..MediaAudioPlayback::default()
    };
    add_test_timeline_audio_sink(&mut authoring, 999);
    authoring.timeline_sources.insert(
        TimelineAudioSinkKey::Root(999),
        TimelineAudioSourceConfig {
            path: PathBuf::from("authoring.wav"),
            gain: 1.0,
            offset_ms: 0,
            speed_milli: 1_000,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        },
    );
    let (output, detached) = take_active_normal_program_output(&mut authoring, false).unwrap();
    assert!(output.is_none());
    assert!(detached.is_empty());
    assert_eq!(authoring.timeline_sinks.len(), 1);
    assert_eq!(authoring.timeline_sources.len(), 1);
    assert_eq!(
        authoring.external_wdm_timeline_owner,
        Some(ExplicitWdmTimelineOwner::NormalAuthoring)
    );
    assert_eq!(authoring.hybrid_cue_output_generation, 23);

    let (output, detached) = take_active_normal_program_output(&mut authoring, true).unwrap();
    assert!(output.is_none());
    assert_eq!(detached.len(), 1);
    assert!(authoring.timeline_sinks.is_empty());
    assert!(authoring.timeline_sources.is_empty());
    assert_eq!(authoring.external_wdm_timeline_owner, None);
    assert_eq!(authoring.hybrid_cue_output_generation, 0);
    for sink in detached {
        sink.stop();
    }

    let mut program_only = MediaAudioPlayback::default();
    add_test_timeline_audio_sink(&mut program_only, 1_000);
    let (_, detached) = take_active_normal_program_output(&mut program_only, false).unwrap();
    assert_eq!(detached.len(), 1);
    assert!(program_only.timeline_sinks.is_empty());
    for sink in detached {
        sink.stop();
    }
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_cue_test_requires_exact_active_published_route() {
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    let error = runtime
        .append_explicit_wdm_cue_test_tone()
        .expect_err("missing delivery must be rejected");
    assert!(error.contains("active ExplicitWdm"));

    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::SameAsio { cue_output: 1 })
        .unwrap();
    let error = runtime
        .append_explicit_wdm_cue_test_tone()
        .expect_err("same-ASIO delivery must be rejected");
    assert!(error.contains("active ExplicitWdm"));

    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::ExplicitWdm {
            device_name: "Test Headphones".to_owned(),
            topology_fingerprint: "test-topology".to_owned(),
        })
        .unwrap();
    let error = runtime
        .append_explicit_wdm_cue_test_tone()
        .expect_err("unpublished WDM attachment must be rejected");
    assert!(error.contains("published CUE attachment"));

    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime
        .state
        .lock()
        .unwrap()
        .attachment
        .as_mut()
        .unwrap()
        .route = timeline_cue_audio::TimelineCueAudioRoute::FollowProgram;
    let error = runtime
        .append_explicit_wdm_cue_test_tone()
        .expect_err("non-explicit attachment route must be rejected");
    assert!(error.contains("ExplicitDevice route"));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn no_cue_project_keeps_exact_split_device_ready_for_start_and_stop_test() {
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::ExplicitWdm {
            device_name: "Test Headphones".to_owned(),
            topology_fingerprint: "test-topology".to_owned(),
        })
        .unwrap();
    let settings_revision = runtime.state.lock().unwrap().settings_revision;
    let (mixer, _) = rodio::mixer::mixer(2, 48_000);
    let destination = PreparedTimelineCueDestination {
        mixer,
        output_sample_rate: 48_000,
        channels: 2,
        program_device_generation: None,
        resolved_device_name: Some("Test Headphones".to_owned()),
        topology_fingerprint: Some("test-topology".to_owned()),
        explicit_stream: None,
        assets: timeline_cue_audio::GuideAssetBank::prepare_embedded(48_000).unwrap(),
    };
    let timeline = engine::TimelineAudioRuntimeSnapshot::default();
    assert!(!timeline.metronome_enabled);
    assert!(!timeline.guide_enabled);
    assert!(timeline.clips.is_empty());

    runtime.complete_prepare(
        TimelineCueAudioPrepareFingerprint {
            settings_revision,
            program_device_generation: None,
        },
        Ok((
            destination,
            vec![TimelineCueAudioEndpointSummary {
                name: "Test Headphones".to_owned(),
                occurrences: 1,
                selectable: true,
            }],
        )),
        &timeline,
        None,
    );
    assert!(runtime.explicit_wdm_output_snapshot().is_some());

    // The no-CUE scheduler tick must preserve the explicit endpoint so the
    // operator can verify it before adding click/guide/media content.
    runtime.sync_normal(
        &timeline,
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
    );
    assert!(runtime.explicit_wdm_output_snapshot().is_some());
    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("exact selected split endpoint accepts the bounded CUE test");
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .cue_test_sinks
            .len(),
        1
    );
    runtime.stop_explicit_wdm_cue_test_tones().unwrap();
    assert!(runtime
        .state
        .lock()
        .unwrap()
        .attachment
        .as_ref()
        .unwrap()
        .cue_test_sinks
        .is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_cue_test_is_bounded_replaced_and_retired_with_attachment() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("exact published WDM route accepts the bounded CUE tone");
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .cue_test_sinks
            .len(),
        1
    );

    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("a repeated CUE test replaces the prior bounded tone");
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .cue_test_sinks
            .len(),
        1,
        "concurrent/repeated test presses must not accumulate sinks"
    );

    runtime.stop_explicit_wdm_cue_test_tones().unwrap();
    assert!(runtime
        .state
        .lock()
        .unwrap()
        .attachment
        .as_ref()
        .unwrap()
        .cue_test_sinks
        .is_empty());

    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("route remains usable after a test stop");
    runtime.retire_timeline_cue_attachment();
    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    drop(state);
    assert!(runtime.explicit_wdm_output_snapshot().is_none());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn concurrent_explicit_wdm_cue_tests_leave_one_owned_sink() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let runtime = Arc::clone(&runtime);
        let barrier = Arc::clone(&barrier);
        workers.push(std::thread::spawn(move || {
            barrier.wait();
            runtime.append_explicit_wdm_cue_test_tone()
        }));
    }
    barrier.wait();
    for worker in workers {
        worker.join().unwrap().unwrap();
    }
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .cue_test_sinks
            .len(),
        1
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn control_plane_and_ui_play_share_test_retirement_and_fence_a_racing_append() {
    let timeline_cue_audio = published_explicit_wdm_cue_test_runtime();
    timeline_cue_audio
        .append_explicit_wdm_cue_test_tone()
        .expect("published WDM route accepts the pre-play test tone");
    let (asio_runtime, _context, _program, _cue) =
        asio_output_runtime::AsioOutputRuntime::active_in_memory(211, 77).unwrap();
    let asio_output_runtime = Arc::new(Mutex::new(asio_runtime));

    let mut runtime_guard = asio_output_runtime.lock().unwrap();
    let (attempted_sender, attempted_receiver) = mpsc::sync_channel(1);
    let racing_runtime = Arc::clone(&asio_output_runtime);
    let racing_cue = Arc::clone(&timeline_cue_audio);
    let racing_append = std::thread::spawn(move || {
        attempted_sender.send(()).unwrap();
        let mut runtime = racing_runtime.lock().unwrap();
        crate::append_explicit_wdm_cue_test_for_idle_timeline(&mut runtime, &racing_cue, false)
    });
    attempted_receiver.recv().unwrap();

    let identity = runtime_guard.current_preflight_identity().unwrap();
    crate::admit_asio_timeline_live_playback(&mut runtime_guard, &timeline_cue_audio, identity)
        .expect("shared runtime Play admission retires the prior test tone");
    assert!(timeline_cue_audio
        .state
        .lock()
        .unwrap()
        .attachment
        .as_ref()
        .unwrap()
        .cue_test_sinks
        .is_empty());
    drop(runtime_guard);

    let error = racing_append
        .join()
        .unwrap()
        .expect_err("a CUE test racing after Play must observe the live gate");
    assert!(error.contains("blocked while Timeline/live playback is active"));
    assert!(asio_output_runtime
        .lock()
        .unwrap()
        .preflight_status(asio_program_cue_render::preflight_now_ms())
        .unwrap()
        .live_playback_active());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_timeline_cue_state_router_fault_retires_wdm_attachment_and_reports_fault() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("published WDM route accepts the test tone before poisoning");
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .unwrap()
            .cue_test_sinks
            .len(),
        1
    );

    poison_timeline_cue_audio_state(&runtime);
    runtime.retire_timeline_cue_attachment_for_router_state(
        TimelineCueAudioLifecycle::Fault,
        "router fault test".to_owned(),
    );

    let state = runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
    drop(state);
    let status_error = runtime
        .status()
        .expect_err("poisoned state must remain visibly faulted");
    assert!(status_error.contains("state lock was poisoned"));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_router_retirement_drops_prepare_receiver_and_reaps_the_late_worker() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let (release_sender, release_receiver) = mpsc::sync_channel::<()>(1);
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    let send_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_send_failed = Arc::clone(&send_failed);
    let worker = std::thread::spawn(move || {
        release_receiver.recv().unwrap();
        worker_send_failed.store(
            result_sender
                .send(Err("late prepared destination".to_owned()))
                .is_err(),
            std::sync::atomic::Ordering::Release,
        );
    });
    let fingerprint = {
        let mut state = runtime.state.lock().unwrap();
        let fingerprint = TimelineCueAudioPrepareFingerprint {
            settings_revision: state.settings_revision,
            program_device_generation: None,
        };
        state.prepare_job = Some(TimelineCueAudioPrepareJob {
            fingerprint,
            started_at: Instant::now(),
            receiver: result_receiver,
            timed_out: false,
            worker: Some(worker),
        });
        fingerprint
    };
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .prepare_job
            .as_ref()
            .unwrap()
            .fingerprint,
        fingerprint
    );

    poison_timeline_cue_audio_state(&runtime);
    runtime.retire_timeline_cue_attachment_for_router_state(
        TimelineCueAudioLifecycle::Fault,
        "router fault with in-flight prepare".to_owned(),
    );

    {
        let state = runtime
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        assert!(state.attachment.is_none());
        assert!(state.prepare_job.is_none());
        assert!(state.topology_probe.is_none());
        assert_eq!(state.retired_workers.len(), 1);
        assert!(!state.normal_admission_open);
        assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    }
    release_sender.send(()).unwrap();
    for _ in 0..100 {
        if send_failed.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(
        send_failed.load(std::sync::atomic::Ordering::Acquire),
        "the late worker result receiver must be dropped during poison retirement"
    );
    runtime.terminal_fault_and_retire_normal_runtime(
        "late worker reap proof",
        "late worker reap proof".to_owned(),
    );
    assert!(runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .retired_workers
        .is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn locked_router_retires_explicit_wdm_and_drops_the_inflight_prepare_receiver() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    assert!(slot
        .into_stop_with(|| Ok(()))
        .unwrap()
        .perform()
        .succeeded());
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let (release_sender, release_receiver) = mpsc::sync_channel::<()>(1);
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    let send_failed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let worker_send_failed = Arc::clone(&send_failed);
    let worker = std::thread::spawn(move || {
        if release_receiver.recv().is_err() {
            return;
        }
        worker_send_failed.store(
            result_sender
                .send(Err("late WDM prepare result".to_owned()))
                .is_err(),
            std::sync::atomic::Ordering::Release,
        );
    });
    {
        let mut state = runtime.state.lock().unwrap();
        let fingerprint = TimelineCueAudioPrepareFingerprint {
            settings_revision: state.settings_revision,
            program_device_generation: None,
        };
        state.prepare_job = Some(TimelineCueAudioPrepareJob {
            fingerprint,
            started_at: Instant::now(),
            receiver: result_receiver,
            timed_out: false,
            worker: Some(worker),
        });
    }

    runtime.sync_with_output_router(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
        Some(&slot),
        Some(&asio_output_runtime),
    );

    {
        let state = runtime.state.lock().unwrap();
        assert!(state.attachment.is_none());
        assert!(state.prepare_job.is_none());
        assert!(state.topology_probe.is_none());
        assert_eq!(state.retired_workers.len(), 1);
        assert_eq!(
            state.lifecycle,
            TimelineCueAudioLifecycle::WaitingForProgramOutput
        );
        assert!(!state.normal_admission_open);
        assert!(state.active_asio_cue_delivery.is_none());
        assert!(state.asio_retry_barrier.is_none());
    }
    release_sender.send(()).unwrap();
    for _ in 0..100 {
        if send_failed.load(std::sync::atomic::Ordering::Acquire) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(send_failed.load(std::sync::atomic::Ordering::Acquire));
    for _ in 0..100 {
        if runtime.state.lock().unwrap().retired_workers[0].is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(runtime.state.lock().unwrap().retired_workers[0].is_finished());
    assert!(!runtime.reap_finished_timeline_cue_workers());
    assert!(runtime.state.lock().unwrap().retired_workers.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn state_poison_between_prepare_spawn_and_publication_is_terminal_and_receiverless() {
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    let settings_revision = runtime.state.lock().unwrap().settings_revision;
    let (mixer, _) = rodio::mixer::mixer(2, 48_000);
    let poison_runtime = Arc::clone(&runtime);

    let error = runtime
        .spawn_prepare_with_after_worker_spawn(
            TimelineCueAudioPrepareFingerprint {
                settings_revision,
                program_device_generation: Some(7),
            },
            timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default(),
            Some(TimelineCueProgramDestination {
                mixer,
                output_sample_rate: 48_000,
                channels: 2,
                device_generation: 7,
                resolved_device_name: Some("Program Test".to_owned()),
            }),
            move || poison_timeline_cue_audio_state(&poison_runtime),
        )
        .expect_err("post-spawn poison must reject prepare publication");
    assert!(error.contains("poisoned before prepare worker publication"));

    for _ in 0..100 {
        let finished = runtime
            .state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .retired_workers
            .iter()
            .all(|worker| worker.is_finished());
        if finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.terminal_fault_and_retire_normal_runtime(
        "post-spawn poison reap proof",
        "post-spawn poison reap proof".to_owned(),
    );
    let state = runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(state.attachment.is_none());
    assert!(state.prepare_job.is_none());
    assert!(state.topology_probe.is_none());
    assert!(state.retired_workers.is_empty());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(state.asio_retry_barrier.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_enqueue_failure_retires_the_route_and_blocks_automatic_reopen() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let identity = runtime
        .state
        .lock()
        .unwrap()
        .attachment
        .as_ref()
        .unwrap()
        .engine_identity
        .clone();
    let click_events = (0..=timeline_cue_audio::MAX_TIMELINE_CUE_QUEUE_CAPACITY)
        .map(|index| protocol::TimelineClickEventSummary {
            epoch: identity.epoch,
            transport_generation: identity.transport_generation,
            sample_frame: index as u64,
            measure: 1,
            beat: (index % 4 + 1) as u16,
            schedule_generation: identity.schedule_generation,
            source: identity.source,
            ..protocol::TimelineClickEventSummary::default()
        })
        .collect();
    let timeline = engine::TimelineAudioRuntimeSnapshot {
        playing: identity.playing,
        metronome_enabled: identity.metronome_enabled,
        guide_enabled: identity.guide_enabled,
        transport_revision: identity.audio_transport_revision,
        click_schedule_generation: identity.schedule_generation,
        source_projection_authority: engine::TimelineAudioProjectionAuthority {
            epoch: identity.epoch,
            generation: identity.transport_generation,
        },
        click_events,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    runtime.feed_attachment(&timeline, None);

    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert!(state.applied.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert_eq!(
        state.blocked_settings_revision,
        Some(state.settings_revision)
    );
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("queue is full")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn prepare_timeout_retires_explicit_wdm_and_skips_feed_on_the_timeout_tick() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let (release_sender, release_receiver) = mpsc::sync_channel::<()>(1);
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        release_receiver.recv().unwrap();
        let _ = result_sender.send(Err("late prepare result".to_owned()));
    });
    {
        let mut state = runtime.state.lock().unwrap();
        let fingerprint = TimelineCueAudioPrepareFingerprint {
            settings_revision: state.settings_revision,
            program_device_generation: None,
        };
        state.prepare_job = Some(TimelineCueAudioPrepareJob {
            fingerprint,
            started_at: Instant::now()
                .checked_sub(TIMELINE_CUE_AUDIO_PREPARE_TIMEOUT + Duration::from_millis(1))
                .unwrap(),
            receiver: result_receiver,
            timed_out: false,
            worker: Some(worker),
        });
    }

    runtime.sync_normal(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
    );

    {
        let state = runtime.state.lock().unwrap();
        assert!(state.attachment.is_none());
        assert!(state.applied.is_none());
        assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Stalled);
        assert_eq!(
            state.blocked_settings_revision,
            Some(state.settings_revision)
        );
        assert!(state.prepare_job.as_ref().is_some_and(|job| job.timed_out));
    }
    release_sender.send(()).unwrap();
    for _ in 0..100 {
        let finished = runtime
            .state
            .lock()
            .unwrap()
            .prepare_job
            .as_ref()
            .and_then(|job| job.worker.as_ref())
            .is_some_and(|worker| worker.is_finished());
        if finished {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    runtime.sync_normal(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
    );
    assert!(runtime.state.lock().unwrap().prepare_job.is_none());
    assert!(!runtime.reap_finished_timeline_cue_workers());
    assert!(runtime.state.lock().unwrap().retired_workers.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn topology_result_without_worker_handle_retires_the_existing_explicit_wdm_route() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let (sender, receiver) = mpsc::sync_channel(1);
    sender
        .send(Ok(("test-topology".to_owned(), Vec::new())))
        .unwrap();
    runtime.state.lock().unwrap().topology_probe = Some(TimelineCueAudioTopologyProbe {
        started_at: Instant::now(),
        timed_out: false,
        receiver,
        worker: None,
    });

    runtime.poll_or_spawn_topology_probe();

    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert!(state.applied.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert_eq!(
        state.blocked_settings_revision,
        Some(state.settings_revision)
    );
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("worker handle was already consumed")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_timeline_cue_state_stop_retires_wdm_attachment_and_reports_fault() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime
        .append_explicit_wdm_cue_test_tone()
        .expect("published WDM route accepts the test tone before poisoning");
    assert_eq!(
        runtime
            .state
            .lock()
            .unwrap()
            .attachment
            .as_ref()
            .expect("published Explicit WDM attachment")
            .cue_test_sinks
            .len(),
        1
    );

    poison_timeline_cue_audio_state(&runtime);
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let stop_close_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_close_called_by_bridge = Arc::clone(&stop_close_called);
    crate::stop_close_asio_program_cue_output_inner_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        move |_| {
            stop_close_called_by_bridge.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        },
    )
    .expect("production Stop/Close must continue after recovering poisoned CUE state");

    let state = runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );
    assert!(stop_close_called.load(std::sync::atomic::Ordering::Acquire));
    assert!(machine
        .lock()
        .unwrap()
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_asio_runtime_and_media_stop_still_fence_router_and_silence_cue() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    add_test_timeline_audio_sink(&mut media_audio.lock().unwrap(), 991);
    poison_asio_output_runtime(&asio_output_runtime);
    let poisoned_media = Arc::clone(&media_audio);
    assert!(std::panic::catch_unwind(move || {
        let _media = poisoned_media.lock().unwrap();
        panic!("poison media audio for Stop proof");
    })
    .is_err());

    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let stop_close_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let called = Arc::clone(&stop_close_called);
    let error = crate::stop_close_asio_program_cue_output_inner_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        move |_| {
            called.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        },
    )
    .expect_err("media poison remains visible after fail-closed sink retirement");

    assert!(error.contains("runtime lock was poisoned"));
    assert!(error.contains("Media audio playback lock was poisoned"));
    assert!(stop_close_called.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );
    assert!(runtime.state.lock().unwrap().attachment.is_none());
    assert!(media_audio
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .timeline_sinks
        .is_empty());
    assert!(machine
        .lock()
        .unwrap()
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("Media audio playback lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn cue_activation_failure_rollback_recovers_poison_and_never_leaves_program_active() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    add_test_timeline_audio_sink(&mut media_audio.lock().unwrap(), 993);
    poison_asio_output_runtime(&asio_output_runtime);
    let poisoned_media = Arc::clone(&media_audio);
    assert!(std::panic::catch_unwind(move || {
        let _media = poisoned_media.lock().unwrap();
        panic!("poison media audio before CUE activation rollback");
    })
    .is_err());
    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let stop_close_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let called = Arc::clone(&stop_close_called);

    let error = crate::rollback_asio_program_after_cue_activation_failure_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        "injected CUE activation failure",
        move |_| {
            called.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        },
    );

    assert!(error.contains("CUE activation failed"));
    assert!(error.contains("runtime lock was poisoned"));
    assert!(error.contains("Media audio playback lock was poisoned"));
    assert!(stop_close_called.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );
    assert!(runtime.state.lock().unwrap().attachment.is_none());
    assert!(media_audio
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .timeline_sinks
        .is_empty());
    assert!(machine
        .lock()
        .unwrap()
        .last_error
        .as_deref()
        .is_some_and(|message| message.contains("CUE activation failed")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_disappearance_after_enumeration_rolls_program_back_before_active_reply() {
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.close_normal_routes_for_asio_start().unwrap();
    runtime
        .activate_asio_cue_delivery(&asio_program_cue::CueDelivery::ExplicitWdm {
            device_name: "Disappearing Headphones".to_owned(),
            topology_fingerprint: "enumerated-topology".to_owned(),
        })
        .unwrap();
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let stop_close_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let called = Arc::clone(&stop_close_called);

    let error = crate::finalize_explicit_wdm_cue_start_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        || Err("selected WDM endpoint disappeared before open".to_owned()),
        move |_| {
            called.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        },
    )
    .expect_err("a post-enumeration WDM open failure must roll PROGRAM back");

    assert!(error.contains("endpoint failed to open after enumeration"));
    assert!(error.contains("PROGRAM was rolled back to Locked"));
    assert!(stop_close_called.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );
    let cue_state = runtime.state.lock().unwrap();
    assert!(cue_state.active_asio_cue_delivery.is_none());
    assert!(cue_state.attachment.is_none());
    assert!(!cue_state.normal_admission_open);
    drop(cue_state);
    assert!(machine
        .lock()
        .unwrap()
        .last_error
        .as_deref()
        .is_some_and(|message| message.contains("endpoint failed to open after enumeration")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_cue_admission_and_bridge_failure_remain_faulted_visible_and_silent() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    poison_timeline_cue_audio_admission(&runtime);
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    add_test_timeline_audio_sink(&mut media_audio.lock().unwrap(), 992);
    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let error = crate::stop_close_asio_program_cue_output_inner_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        |_| Err("injected bridge Stop/Close failure".to_owned()),
    )
    .expect_err("bridge failure remains terminal after CUE retirement");

    assert!(error.contains("admission lock was poisoned"));
    assert!(error.contains("Stop/Close failed"));
    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert!(state.asio_retry_barrier.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("admission lock was poisoned")));
    drop(state);
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Fault
    );
    assert!(media_audio.lock().unwrap().timeline_sinks.is_empty());
    assert!(machine
        .lock()
        .unwrap()
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("Stop/Close failed")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_normal_scheduler_state_retires_explicit_wdm_and_closes_admission() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    poison_timeline_cue_audio_state(&runtime);

    runtime.sync_normal(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
    );

    let state = runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(state.attachment.is_none());
    assert!(state.prepare_job.is_none());
    assert!(state.topology_probe.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert_eq!(
        state.blocked_settings_revision,
        Some(state.settings_revision)
    );
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(state.asio_retry_barrier.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn state_poison_after_normal_initial_check_still_retires_explicit_wdm() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    let poison_runtime = Arc::clone(&runtime);

    runtime.sync_normal_with_after_initial_state(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
        move || poison_timeline_cue_audio_state(&poison_runtime),
    );

    let state = runtime
        .state
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(state.asio_retry_barrier.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("state lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn poisoned_normal_topology_admission_retires_explicit_wdm_and_closes_admission() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();
    runtime.state.lock().unwrap().last_topology_probe_at = Instant::now() - Duration::from_secs(3);
    poison_timeline_cue_audio_admission(&runtime);

    runtime.sync_normal(
        &engine::TimelineAudioRuntimeSnapshot::default(),
        &Arc::new(Mutex::new(MediaAudioPlayback::default())),
    );

    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(!state.normal_admission_open);
    assert_eq!(
        state.blocked_settings_revision,
        Some(state.settings_revision)
    );
    assert!(state.active_asio_cue_delivery.is_none());
    assert!(state.asio_retry_barrier.is_none());
    assert!(state
        .last_error
        .as_deref()
        .is_some_and(|error| error.contains("admission lock was poisoned")));
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn normal_prepare_failure_retires_the_owned_route_before_publishing_fault() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    runtime.append_explicit_wdm_cue_test_tone().unwrap();

    runtime.fail_prepare("injected normal prepare failure".to_owned());

    let state = runtime.state.lock().unwrap();
    assert!(state.attachment.is_none());
    assert!(state.applied.is_none());
    assert_eq!(state.lifecycle, TimelineCueAudioLifecycle::Fault);
    assert!(state.normal_admission_open);
    assert_eq!(
        state.blocked_settings_revision,
        Some(state.settings_revision)
    );
    assert!(matches!(
        state.active_asio_cue_delivery,
        Some(ActiveAsioCueDelivery::ExplicitWdm { .. })
    ));
    assert_eq!(
        state.last_error.as_deref(),
        Some("injected normal prepare failure")
    );
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn running_cue_prepare_worker_cannot_delay_program_stop_close() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    let (release_sender, release_receiver) = mpsc::sync_channel::<()>(1);
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = release_receiver.recv();
        let _ = result_sender.send(Err("late topology result".to_owned()));
    });
    runtime.state.lock().unwrap().topology_probe = Some(TimelineCueAudioTopologyProbe {
        started_at: Instant::now(),
        timed_out: false,
        receiver: result_receiver,
        worker: Some(worker),
    });
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let machine = Arc::new(Mutex::new(AsioOutputMachineState::default()));
    let stop_close_called = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let called = Arc::clone(&stop_close_called);

    crate::stop_close_asio_program_cue_output_inner_with(
        &runtime,
        &asio_output_runtime,
        &slot,
        &media_audio,
        &machine,
        move |_| {
            called.store(true, std::sync::atomic::Ordering::Release);
            Ok(())
        },
    )
    .expect("unfinished device worker must be quarantined before bridge Stop/Close");

    assert!(stop_close_called.load(std::sync::atomic::Ordering::Acquire));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );
    {
        let state = runtime.state.lock().unwrap();
        assert!(state.topology_probe.is_none());
        assert_eq!(state.retired_workers.len(), 1);
        assert!(!state.normal_admission_open);
    }
    release_sender.send(()).unwrap();
    for _ in 0..100 {
        if runtime.state.lock().unwrap().retired_workers[0].is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(runtime.state.lock().unwrap().retired_workers[0].is_finished());
    assert!(!runtime.reap_finished_timeline_cue_workers());
    assert!(runtime.state.lock().unwrap().retired_workers.is_empty());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn explicit_wdm_timeline_commit_fence_rejects_after_stop_and_drains_before_stop() {
    let retired_before_commit = published_explicit_wdm_cue_test_runtime();
    let retired_fence = retired_before_commit
        .external_wdm_timeline_route_fence()
        .expect("the current WDM route publishes a Timeline commit fence");
    assert!(retired_before_commit.close_routes_for_asio_stop().is_none());
    let error = retired_before_commit
        .lock_explicit_wdm_timeline_commit(retired_fence)
        .expect_err("a Stop-first WDM epoch must not admit Timeline sink commit");
    assert!(error.contains("changed before sink commit"));

    let commit_before_stop = published_explicit_wdm_cue_test_runtime();
    let media_audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let commit_fence = commit_before_stop
        .external_wdm_timeline_route_fence()
        .expect("the current WDM route publishes a Timeline commit fence");
    let commit_guard = commit_before_stop
        .lock_explicit_wdm_timeline_commit(commit_fence)
        .expect("the current WDM epoch admits the final commit fence");
    let (stop_started_sender, stop_started_receiver) = mpsc::sync_channel(1);
    let stop_runtime = Arc::clone(&commit_before_stop);
    let stop_audio = Arc::clone(&media_audio);
    let stop_worker = std::thread::spawn(move || {
        stop_started_sender.send(()).unwrap();
        assert!(stop_runtime.close_routes_for_asio_stop().is_none());
        let mut playback = stop_audio.lock().unwrap();
        playback.stop_all();
        playback.stop_all_timeline();
    });
    stop_started_receiver.recv().unwrap();

    // This models the production commit section after decoder append and its
    // postcheck. Stop is already attempting the same admission fence, so it
    // cannot retire the route until the sink becomes visible to playback.
    add_test_timeline_audio_sink(&mut media_audio.lock().unwrap(), 994);
    assert_eq!(media_audio.lock().unwrap().timeline_sinks.len(), 1);
    drop(commit_guard);
    stop_worker.join().unwrap();

    assert!(media_audio.lock().unwrap().timeline_sinks.is_empty());
    assert!(commit_before_stop
        .state
        .lock()
        .unwrap()
        .attachment
        .is_none());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn transient_cue_state_contention_fails_closed_and_retires_stale_routes() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (_engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let runtime = published_explicit_wdm_cue_test_runtime();
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    {
        let mut playback = audio.lock().unwrap();
        playback.hybrid_cue_output_generation = 23;
        add_test_timeline_audio_sink(&mut playback, 995);
    }
    let timeline = engine::TimelineAudioRuntimeSnapshot::default();

    let mut state = runtime.state.lock().unwrap();
    assert!(matches!(
        runtime.asio_cue_route_snapshot(),
        AsioCueRouteSnapshot::Busy
    ));
    assert_eq!(runtime.external_wdm_timeline_route_fence(), None);
    // A busy lifecycle observer is not proof that the old external route is
    // still owned. The playback boundary must retire it immediately.
    audio
        .lock()
        .unwrap()
        .observe_external_wdm_timeline_route(None);
    assert_eq!(audio.lock().unwrap().hybrid_cue_output_generation, 0);
    assert!(audio.lock().unwrap().timeline_sinks.is_empty());

    runtime.sync_with_output_router(&timeline, &audio, Some(&slot), Some(&asio_output_runtime));
    assert_eq!(state.attachment.as_ref().unwrap().output_clock_epoch, 23);

    state.active_asio_cue_delivery = Some(ActiveAsioCueDelivery::SameAsio);
    runtime.sync_with_output_router(&timeline, &audio, Some(&slot), Some(&asio_output_runtime));
    assert_eq!(state.attachment.as_ref().unwrap().output_clock_epoch, 23);
    drop(state);

    assert!(matches!(
        runtime.external_wdm_timeline_route_fence(),
        Some(ExternalWdmTimelineRouteFence {
            owner: None,
            generation: 0,
        })
    ));
    assert!(runtime.state.lock().unwrap().attachment.is_some());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn blocking_cue_observers_wait_for_contention_then_preserve_the_published_epoch() {
    let runtime = published_explicit_wdm_cue_test_runtime();
    let state = runtime.state.lock().unwrap();
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let (result_sender, result_receiver) = mpsc::sync_channel(1);
    let observed_runtime = Arc::clone(&runtime);
    let observer = std::thread::spawn(move || {
        started_sender.send(()).unwrap();
        let result = observed_runtime.asio_cue_route_snapshot_blocking();
        result_sender.send(result).unwrap();
    });
    started_receiver.recv().unwrap();
    assert!(matches!(
        result_receiver.recv_timeout(Duration::from_millis(25)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    assert_eq!(state.attachment.as_ref().unwrap().output_clock_epoch, 23);
    drop(state);

    let observed = result_receiver.recv().unwrap().unwrap();
    assert!(matches!(
        observed,
        AsioCueRouteSnapshot::ExplicitWdm(Some(ExplicitWdmTimelineOutputSnapshot {
            output_clock_epoch: 23,
            ..
        }))
    ));
    observer.join().unwrap();

    let timeline = engine::TimelineAudioRuntimeSnapshot::default();
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let state = runtime.state.lock().unwrap();
    let (started_sender, started_receiver) = mpsc::sync_channel(1);
    let (finished_sender, finished_receiver) = mpsc::sync_channel(1);
    let sync_runtime = Arc::clone(&runtime);
    let sync_audio = Arc::clone(&audio);
    let sync_worker = std::thread::spawn(move || {
        started_sender.send(()).unwrap();
        sync_runtime.sync_normal(&timeline, &sync_audio);
        finished_sender.send(()).unwrap();
    });
    started_receiver.recv().unwrap();
    assert!(matches!(
        finished_receiver.recv_timeout(Duration::from_millis(25)),
        Err(mpsc::RecvTimeoutError::Timeout)
    ));
    assert_eq!(state.attachment.as_ref().unwrap().output_clock_epoch, 23);
    drop(state);
    finished_receiver.recv().unwrap();
    sync_worker.join().unwrap();
    assert!(runtime.state.lock().unwrap().attachment.is_some());
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn normal_selection_keeps_router_locked_until_retired_cue_workers_are_reaped() {
    let slot = audio_output_router::RouterSlot::test_slot();
    slot.quiesce().unwrap();
    assert!(slot
        .into_admitted_start_with(|| Ok(()))
        .unwrap()
        .perform()
        .succeeded());
    assert!(slot
        .into_stop_with(|| Ok(()))
        .unwrap()
        .perform()
        .succeeded());
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked
    );

    let runtime = Arc::new(TimelineCueAudioRuntime::default());
    runtime.initialize(
        PathBuf::from("C:/test/timeline-cue-audio-settings.json"),
        Ok(timeline_cue_audio::MachineTimelineCueAudioSettingsV1::default()),
    );
    runtime.state.lock().unwrap().normal_admission_open = false;
    let (release_sender, release_receiver) = mpsc::sync_channel::<()>(1);
    let worker = std::thread::spawn(move || {
        let _ = release_receiver.recv();
    });
    runtime.state.lock().unwrap().retired_workers.push(worker);

    let error = crate::select_normal_audio_output_with_cue_preflight(&slot, &runtime)
        .expect_err("a running retired CUE worker must block Normal selection");
    assert!(error.contains("retired route is still owned"));
    assert_eq!(
        slot.snapshot().unwrap().state,
        audio_output_router::State::Locked,
        "failed CUE preflight must not consume the only Locked -> Normal transition"
    );

    release_sender.send(()).unwrap();
    for _ in 0..100 {
        if runtime.state.lock().unwrap().retired_workers[0].is_finished() {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(runtime.state.lock().unwrap().retired_workers[0].is_finished());
    let snapshot = crate::select_normal_audio_output_with_cue_preflight(&slot, &runtime)
        .expect("a finished retired worker is reaped before Normal selection");
    assert_eq!(snapshot.state, audio_output_router::State::Normal);
    assert!(runtime.state.lock().unwrap().normal_admission_open);
}

#[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
#[test]
fn asio_timeline_play_retires_wdm_test_before_live_ack() {
    let _router_test_guard = crate::tests::TIMELINE_CUE_ASIO_ROUTER_TEST_LOCK
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let (engine, slot, asio_output_runtime) = crate::tests::active_asio_command_parts();
    let timeline_cue_audio = published_explicit_wdm_cue_test_runtime();
    timeline_cue_audio
        .append_explicit_wdm_cue_test_tone()
        .expect("published WDM route accepts the pre-play test tone");

    crate::set_timeline_playing_asio_linearized(
        &engine,
        &slot,
        &asio_output_runtime,
        &timeline_cue_audio,
        true,
    )
    .expect("Timeline Play should acknowledge after retiring the WDM test tone");

    assert!(timeline_cue_audio
        .state
        .lock()
        .unwrap()
        .attachment
        .as_ref()
        .unwrap()
        .cue_test_sinks
        .is_empty());
    assert!(asio_output_runtime
        .lock()
        .unwrap()
        .preflight_status(asio_program_cue_render::preflight_now_ms())
        .unwrap()
        .live_playback_active());
}

#[test]
fn timeline_audio_prepare_transaction_deadline_is_global_across_clips() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let current = engine.video_audio_runtime_snapshot().timeline_audio;
    let timeline = engine::TimelineAudioRuntimeSnapshot {
        clips: vec![
            TimelineAudioClipSummary {
                id: 801,
                layer_id: 44,
                media_asset_id: None,
                path: r"C:\syndocal-missing\deadline-a.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 2_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
            TimelineAudioClipSummary {
                id: 802,
                layer_id: 45,
                media_asset_id: None,
                path: r"C:\syndocal-missing\deadline-b.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 2_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
        ],
        playing: true,
        position_ms: 100,
        source_projection_authority: current.source_projection_authority,
        publication_generation: current.publication_generation,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };
    let (mixer, _unconsumed_source) = rodio::mixer::mixer(1, 8_000);
    let audio = Arc::new(Mutex::new(MediaAudioPlayback {
        timeline_test_mixer: Some(mixer),
        ..MediaAudioPlayback::default()
    }));
    let started = Instant::now();
    let mut contention = None;
    let error = sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {},
        || {
            let held_audio = Arc::clone(&audio);
            let (entered_tx, entered_rx) = mpsc::sync_channel(1);
            contention = Some(std::thread::spawn(move || {
                let _guard = held_audio.lock().unwrap();
                entered_tx.send(()).unwrap();
                std::thread::sleep(Duration::from_millis(800));
            }));
            entered_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        },
    )
    .unwrap_err();
    contention.take().unwrap().join().unwrap();
    assert!(matches!(
        error,
        TimelineAudioPrepareFault::BudgetExhausted(_)
    ));
    assert!(started.elapsed() < Duration::from_millis(1_100));
    let playback = audio.lock().unwrap();
    assert_eq!(playback.timeline_start_attempt_count, 2);
    assert!(playback.timeline_sinks.is_empty());
    assert!(playback.timeline_sources.is_empty());
}

#[test]
fn timeline_audio_prepare_single_flight_quarantines_timeout_and_reaps_exactly_once() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let entered = Arc::new(AtomicU64::new(0));
    let hook_gate = Arc::clone(&gate);
    let hook_entered = Arc::clone(&entered);
    let mut coordinator = TimelineAudioPrepareCoordinator {
        before_prepare: Some(Arc::new(move || {
            hook_entered.fetch_add(1, Ordering::AcqRel);
            let (lock, wake) = &*hook_gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        })),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Pending
    ));
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while entered.load(Ordering::Acquire) == 0 {
        assert!(Instant::now() < entered_deadline);
        std::thread::yield_now();
    }
    std::thread::sleep(TIMELINE_AUDIO_PREPARE_TRANSACTION_BUDGET);
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::TimedOut { .. }
    ));
    for _ in 0..100 {
        assert!(matches!(
            coordinator.poll_or_spawn(&engine, &audio, &timeline),
            TimelineAudioPreparePoll::Quarantined
        ));
    }
    assert_eq!(coordinator.spawn_count, 1);
    assert_eq!(coordinator.reap_count, 0);
    assert!(audio.lock().unwrap().timeline_sinks.is_empty());

    {
        let (lock, wake) = &*gate;
        *lock.lock().unwrap() = true;
        wake.notify_all();
    }
    let reap_deadline = Instant::now() + Duration::from_secs(2);
    while coordinator.active.is_some() {
        let _ = coordinator.poll_or_spawn(&engine, &audio, &timeline);
        assert!(Instant::now() < reap_deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(coordinator.reap_count, 1);
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Quarantined
    ));
    assert_eq!(coordinator.spawn_count, 1);

    let transport = engine.snapshot().timeline;
    engine
        .set_timeline_playing_published(
            transport.transport_epoch,
            transport.transport_generation,
            true,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    let next = engine.video_audio_runtime_snapshot().timeline_audio;
    assert_ne!(next.publication_generation, timeline.publication_generation);
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &next),
        TimelineAudioPreparePoll::Pending
    ));
    assert_eq!(coordinator.spawn_count, 2);
    let finish_deadline = Instant::now() + Duration::from_secs(2);
    while coordinator.active.is_some() {
        let _ = coordinator.poll_or_spawn(&engine, &audio, &next);
        assert!(Instant::now() < finish_deadline);
        std::thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(coordinator.reap_count, 2);
    assert!(audio.lock().unwrap().timeline_sinks.is_empty());
}

#[test]
fn timeline_audio_internal_budget_fault_blocks_exact_fingerprint_without_respawn() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let hook_calls = Arc::new(AtomicU64::new(0));
    let worker_hook_calls = Arc::clone(&hook_calls);
    let mut coordinator = TimelineAudioPrepareCoordinator {
        before_commit_lock: Some(Arc::new(move || {
            if worker_hook_calls.fetch_add(1, Ordering::AcqRel) == 0 {
                std::thread::sleep(
                    TIMELINE_AUDIO_PREPARE_TRANSACTION_BUDGET + Duration::from_millis(20),
                );
            }
        })),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Pending
    ));
    let finish_deadline = Instant::now() + Duration::from_secs(2);
    while !coordinator
        .active
        .as_ref()
        .and_then(|job| job.worker.as_ref())
        .is_some_and(|worker| worker.is_finished())
    {
        assert!(Instant::now() < finish_deadline);
        std::thread::yield_now();
    }
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::TimedOut { .. }
    ));
    assert!(coordinator.active.is_none());
    for _ in 0..100 {
        assert!(matches!(
            coordinator.poll_or_spawn(&engine, &audio, &timeline),
            TimelineAudioPreparePoll::Quarantined
        ));
    }
    assert_eq!(coordinator.spawn_count, 1);
    assert_eq!(hook_calls.load(Ordering::Acquire), 1);

    {
        let mut playback = audio.lock().unwrap();
        playback.audio_device_generation = playback.audio_device_generation.checked_add(1).unwrap();
        playback.requested_device_name = Some("Post-budget output".to_string());
    }
    let rotated = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &rotated),
        TimelineAudioPreparePoll::Pending
    ));
    assert_eq!(coordinator.spawn_count, 2);
    let recovery_deadline = Instant::now() + Duration::from_secs(1);
    loop {
        match coordinator.poll_or_spawn(&engine, &audio, &rotated) {
            TimelineAudioPreparePoll::Completed { result, .. } => {
                result.unwrap();
                break;
            }
            TimelineAudioPreparePoll::Pending => {
                assert!(Instant::now() < recovery_deadline);
                std::thread::yield_now();
            }
            other => panic!("rotated output generation must recover exactly once: {other:?}"),
        }
    }
    assert_eq!(hook_calls.load(Ordering::Acquire), 2);
}

#[test]
fn timeline_audio_prepare_outcomes_never_cross_follow_contexts() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let current = TimelineAudioPrepareCoordinator::context(&engine, &audio, &timeline)
        .expect("idle playback exposes a stable output fingerprint");

    for result in [
        Ok(current.fingerprint.authority),
        Err(TimelineAudioPrepareFault::ordinary("captured A failed")),
    ] {
        let mut captured_a = current.clone();
        captured_a.follow.generation = captured_a.follow.generation.saturating_add(1);
        captured_a.follow.source_timeline_id = Some(TimelineId(41));
        captured_a.follow.target_timeline_id = Some(TimelineId(42));
        let mut coordinator = TimelineAudioPrepareCoordinator {
            active: Some(completed_timeline_audio_prepare_job(captured_a, result)),
            ..TimelineAudioPrepareCoordinator::default()
        };
        let deadline = Instant::now() + Duration::from_secs(1);
        loop {
            match coordinator.poll_or_spawn(&engine, &audio, &timeline) {
                TimelineAudioPreparePoll::Pending => {
                    assert!(Instant::now() < deadline);
                    std::thread::yield_now();
                }
                TimelineAudioPreparePoll::Obsolete => break,
                other => {
                    panic!("captured A outcome must not be attributed to current B: {other:?}")
                }
            }
        }
        assert!(coordinator.active.is_none());
    }

    let mut captured_a = current.clone();
    captured_a.follow.generation = captured_a.follow.generation.saturating_add(1);
    captured_a.follow.source_timeline_id = Some(TimelineId(51));
    captured_a.follow.target_timeline_id = Some(TimelineId(52));
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (sender, receiver) = mpsc::sync_channel(1);
    let authority = captured_a.fingerprint.authority;
    let worker = std::thread::spawn(move || {
        let _ = release_rx.recv_timeout(Duration::from_secs(1));
        let _ = sender.send(Ok(authority));
    });
    let mut coordinator = TimelineAudioPrepareCoordinator {
        active: Some(TimelineAudioPrepareJob {
            context: captured_a,
            started_at: Instant::now() - TIMELINE_AUDIO_PREPARE_TRANSACTION_BUDGET,
            receiver,
            worker: Some(worker),
            result: None,
            timed_out: false,
            commit_state: Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        }),
        ..TimelineAudioPrepareCoordinator::default()
    };
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Obsolete
    ));
    assert!(coordinator.active.as_ref().unwrap().timed_out);
    assert!(
        coordinator
            .active
            .as_ref()
            .unwrap()
            .commit_state
            .lock()
            .unwrap()
            .cancelled
    );
    release_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while coordinator.active.is_some() {
        let _ = coordinator.poll_or_spawn(&engine, &audio, &timeline);
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
}

#[test]
fn timeline_audio_prepare_shutdown_cancels_and_joins_nested_worker() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let context = TimelineAudioPrepareCoordinator::context(&engine, &audio, &timeline)
        .expect("idle playback exposes a stable output fingerprint");
    let authority = context.fingerprint.authority;
    let commit_state = Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default()));
    let observed_commit_state = Arc::clone(&commit_state);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (result_tx, result_rx) = mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = release_rx.recv();
        let _ = result_tx.send(Ok(authority));
    });
    let mut coordinator = TimelineAudioPrepareCoordinator {
        active: Some(TimelineAudioPrepareJob {
            context,
            started_at: Instant::now(),
            receiver: result_rx,
            worker: Some(worker),
            result: None,
            timed_out: false,
            commit_state,
        }),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let (shutdown_done_tx, shutdown_done_rx) = mpsc::sync_channel(1);
    let shutdown = std::thread::spawn(move || {
        coordinator.shutdown();
        shutdown_done_tx
            .send((
                coordinator.active.is_none(),
                coordinator.blocked.is_none(),
                coordinator.reap_count,
            ))
            .unwrap();
    });

    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        if observed_commit_state.lock().unwrap().cancelled {
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::yield_now();
    }
    assert!(matches!(
        shutdown_done_rx.try_recv(),
        Err(mpsc::TryRecvError::Empty)
    ));
    release_tx.send(()).unwrap();
    let (active_cleared, block_cleared, reap_count) = shutdown_done_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();
    shutdown.join().unwrap();
    assert!(active_cleared);
    assert!(block_cleared);
    assert_eq!(reap_count, 1);
}

#[test]
fn timeline_audio_prepare_timeout_unblocks_only_after_output_device_rotation() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let entered = Arc::new(AtomicBool::new(false));
    let hook_gate = Arc::clone(&gate);
    let hook_entered = Arc::clone(&entered);
    let mut coordinator = TimelineAudioPrepareCoordinator {
        before_prepare: Some(Arc::new(move || {
            hook_entered.store(true, Ordering::Release);
            let (lock, wake) = &*hook_gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        })),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Pending
    ));
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < entered_deadline);
        std::thread::yield_now();
    }
    coordinator.active.as_mut().unwrap().started_at =
        Instant::now() - TIMELINE_AUDIO_PREPARE_TRANSACTION_BUDGET;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::TimedOut { .. }
    ));
    for _ in 0..10 {
        assert!(matches!(
            coordinator.poll_or_spawn(&engine, &audio, &timeline),
            TimelineAudioPreparePoll::Quarantined
        ));
    }
    assert_eq!(coordinator.spawn_count, 1);

    {
        let mut playback = audio.lock().unwrap();
        playback.audio_device_generation = playback.audio_device_generation.checked_add(1).unwrap();
        playback.requested_device_name = Some("Recovered output".to_string());
    }
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Quarantined
    ));
    let (lock, wake) = &*gate;
    *lock.lock().unwrap() = true;
    wake.notify_all();
    let reap_deadline = Instant::now() + Duration::from_secs(1);
    while coordinator.active.is_some() {
        let _ = coordinator.poll_or_spawn(&engine, &audio, &timeline);
        assert!(Instant::now() < reap_deadline);
        std::thread::yield_now();
    }
    assert_eq!(coordinator.reap_count, 1);
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Pending
    ));
    assert_eq!(coordinator.spawn_count, 2);
    let finish_deadline = Instant::now() + Duration::from_secs(1);
    let completed_context = loop {
        match coordinator.poll_or_spawn(&engine, &audio, &timeline) {
            TimelineAudioPreparePoll::Completed { context, result } => {
                result.unwrap();
                break context;
            }
            TimelineAudioPreparePoll::Pending => {
                assert!(Instant::now() < finish_deadline);
                std::thread::yield_now();
            }
            TimelineAudioPreparePoll::Obsolete => {
                // The isolated test engine may rotate its independent Follow
                // context while the recovered-device worker is finishing.
                // Production correctly discards that stale result; keep
                // polling until the same recovered device fingerprint gets a
                // job under the current Follow context.
                assert!(Instant::now() < finish_deadline);
                std::thread::yield_now();
            }
            other => panic!("recovered device requires its own exact job: {other:?}"),
        }
    };
    assert_eq!(completed_context.fingerprint.audio_device_generation, 1);
    assert_eq!(
        completed_context
            .fingerprint
            .requested_device_name
            .as_deref(),
        Some("Recovered output")
    );
}

#[test]
fn timeline_audio_prepare_timeout_cancels_exact_precommit_before_install() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!(
        "syndocal-timeline-audio-timeout-commit-fence-{suffix}.wav"
    ));
    write_timeline_audio_test_wav(&path);
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    engine
        .add_timeline_layer(protocol::TimelineLayerSummary {
            id: 74,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: true,
            kind: TimelineLayerKind::Audio,
        })
        .unwrap();
    engine
        .add_timeline_audio_clip(TimelineAudioClipSummary {
            id: 7401,
            layer_id: 74,
            media_asset_id: None,
            path: path.to_string_lossy().into_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        })
        .unwrap();
    let transport = engine.snapshot().timeline;
    engine
        .set_timeline_playing_published(
            transport.transport_epoch,
            transport.transport_generation,
            true,
            Instant::now() + Duration::from_secs(1),
        )
        .unwrap();
    let (mixer, _unconsumed_source) = rodio::mixer::mixer(1, 8_000);
    let audio = Arc::new(Mutex::new(MediaAudioPlayback {
        timeline_test_mixer: Some(mixer),
        ..MediaAudioPlayback::default()
    }));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let entered = Arc::new(AtomicBool::new(false));
    let hook_gate = Arc::clone(&gate);
    let hook_entered = Arc::clone(&entered);
    let mut coordinator = TimelineAudioPrepareCoordinator {
        before_commit_lock: Some(Arc::new(move || {
            hook_entered.store(true, Ordering::Release);
            let (lock, wake) = &*hook_gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        })),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::Pending
    ));
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < entered_deadline);
        std::thread::yield_now();
    }
    coordinator.active.as_mut().unwrap().started_at =
        Instant::now() - TIMELINE_AUDIO_PREPARE_TRANSACTION_BUDGET;
    assert!(matches!(
        coordinator.poll_or_spawn(&engine, &audio, &timeline),
        TimelineAudioPreparePoll::TimedOut { .. }
    ));
    let (lock, wake) = &*gate;
    *lock.lock().unwrap() = true;
    wake.notify_all();
    let reap_deadline = Instant::now() + Duration::from_secs(1);
    while coordinator.active.is_some() {
        let _ = coordinator.poll_or_spawn(&engine, &audio, &timeline);
        assert!(Instant::now() < reap_deadline);
        std::thread::yield_now();
    }
    let playback = audio.lock().unwrap();
    assert_eq!(
        playback.timeline_start_attempt_count, 1,
        "planning may observe the clip once, but timeout must prevent install"
    );
    assert!(playback.timeline_sinks.is_empty());
    assert!(playback.timeline_sources.is_empty());
    assert!(playback.timeline_last_install_source_position_ms.is_empty());
    drop(playback);
    let _ = fs::remove_file(path);
}

#[test]
fn timeline_audio_prepare_single_flight_drop_never_joins_unfinished_worker() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let entered = Arc::new(AtomicBool::new(false));
    let hook_gate = Arc::clone(&gate);
    let hook_entered = Arc::clone(&entered);
    let mut coordinator = TimelineAudioPrepareCoordinator {
        before_prepare: Some(Arc::new(move || {
            hook_entered.store(true, Ordering::Release);
            let (lock, wake) = &*hook_gate;
            let mut released = lock.lock().unwrap();
            while !*released {
                released = wake.wait(released).unwrap();
            }
        })),
        ..TimelineAudioPrepareCoordinator::default()
    };
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let _ = coordinator.poll_or_spawn(&engine, &audio, &timeline);
    let entered_deadline = Instant::now() + Duration::from_secs(1);
    while !entered.load(Ordering::Acquire) {
        assert!(Instant::now() < entered_deadline);
        std::thread::yield_now();
    }
    let drop_started = Instant::now();
    drop(coordinator);
    assert!(drop_started.elapsed() < Duration::from_millis(50));
    let (lock, wake) = &*gate;
    *lock.lock().unwrap() = true;
    wake.notify_all();
}

#[test]
fn timeline_audio_commit_fence_rejects_publication_after_final_snapshot_read() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let authority = timeline.source_projection_authority;
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));

    let error = sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {},
        || engine.set_timeline_audio_master(0, true).unwrap(),
    )
    .unwrap_err();
    assert!(error.to_string().contains("changed before sink install"));
    assert_ne!(
        engine
            .video_audio_runtime_snapshot()
            .timeline_audio
            .source_projection_authority,
        authority
    );
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.timeline_start_attempt_count, 0);
}

#[test]
fn timeline_audio_commit_fence_rejects_same_authority_transport_publication() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let authority = timeline.source_projection_authority;
    let publication_generation = timeline.publication_generation;
    let transport = engine.snapshot().timeline;
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));

    let error = sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {},
        || {
            engine
                .set_timeline_playing_published(
                    transport.transport_epoch,
                    transport.transport_generation,
                    true,
                    Instant::now() + Duration::from_secs(1),
                )
                .unwrap();
        },
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("transport changed before sink install"));
    let current = engine.video_audio_runtime_snapshot().timeline_audio;
    assert_eq!(current.source_projection_authority, authority);
    assert_ne!(current.publication_generation, publication_generation);
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.timeline_start_attempt_count, 0);
}

#[test]
fn timeline_audio_commit_fence_rejects_same_authority_seek_publication() {
    let engine = EngineHandle::start_for_tests(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let timeline = engine.video_audio_runtime_snapshot().timeline_audio;
    let authority = timeline.source_projection_authority;
    let publication_generation = timeline.publication_generation;
    let audio = Arc::new(Mutex::new(MediaAudioPlayback::default()));

    let error = sync_timeline_audio_without_blocking_playback_lock(
        &engine,
        &audio,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        None,
        &timeline,
        &Arc::new(Mutex::new(TimelineAudioPrepareCommitState::default())),
        || {},
        || {
            engine.send(engine::EngineCommand::SeekTimeline(1)).unwrap();
            let deadline = Instant::now() + Duration::from_secs(1);
            while engine
                .video_audio_runtime_snapshot()
                .timeline_audio
                .publication_generation
                == publication_generation
            {
                assert!(
                    Instant::now() < deadline,
                    "seek publication did not reach the audio commit fence"
                );
                std::thread::yield_now();
            }
        },
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("transport changed before sink install"));
    let current = engine.video_audio_runtime_snapshot().timeline_audio;
    assert_eq!(current.source_projection_authority, authority);
    assert_ne!(current.publication_generation, publication_generation);
    let playback = audio.lock().unwrap();
    assert!(playback.timeline_sinks.is_empty());
    assert_eq!(playback.timeline_start_attempt_count, 0);
}

#[test]
fn audio_resync_threshold_and_cooldown_prevent_seek_thrash() {
    assert!(MEDIA_AUDIO_SYNC_INTERVAL < Duration::from_millis(MEDIA_AUDIO_RESYNC_THRESHOLD_MS));
    assert!(MEDIA_AUDIO_SYNC_IDLE_INTERVAL >= MEDIA_AUDIO_SYNC_INTERVAL);
    assert!(!media_audio_resync_required(
        MEDIA_AUDIO_RESYNC_THRESHOLD_MS as i64,
        MEDIA_AUDIO_RESYNC_COOLDOWN
    ));
    assert!(!media_audio_resync_required(
        -(MEDIA_AUDIO_RESYNC_THRESHOLD_MS as i64 + 1),
        MEDIA_AUDIO_RESYNC_COOLDOWN - Duration::from_millis(1)
    ));
    assert!(media_audio_resync_required(
        MEDIA_AUDIO_RESYNC_THRESHOLD_MS as i64 + 1,
        MEDIA_AUDIO_RESYNC_COOLDOWN
    ));
    assert!(media_audio_resync_required(
        -(MEDIA_AUDIO_RESYNC_THRESHOLD_MS as i64 + 1),
        Duration::MAX
    ));
}

#[test]
fn timeline_audio_playback_lifecycle_recues_on_play_and_seek_and_stops_on_pause() {
    let started_at = Instant::now();
    assert_eq!(
        timeline_audio_transport_action(
            TimelineAudioTransportState::default(),
            true,
            1_000,
            4,
            started_at,
        ),
        TimelineAudioTransportAction::Recue
    );

    let playing = TimelineAudioTransportState {
        playing: true,
        position_ms: 1_000,
        transport_revision: 4,
        synced_at: Some(started_at),
    };
    assert_eq!(
        timeline_audio_transport_action(
            playing,
            true,
            1_025,
            4,
            started_at + Duration::from_millis(25),
        ),
        TimelineAudioTransportAction::Sync
    );
    assert_eq!(
        timeline_audio_transport_action(
            playing,
            true,
            1_050,
            5,
            started_at + Duration::from_millis(25),
        ),
        TimelineAudioTransportAction::Recue
    );
    assert_eq!(
        timeline_audio_transport_action(playing, false, 1_025, 4, started_at),
        TimelineAudioTransportAction::Stop
    );
}

#[test]
fn invalid_timeline_audio_rate_is_visible_and_cue_only_does_not_fault_program_settlement() {
    let snapshot_for = |output_bus, playback_rate_milli| engine::TimelineAudioRuntimeSnapshot {
        child_clips: vec![engine::ChildTimelineAudioRuntimeClip {
            root: engine::ChildTimelineAudioRuntimeRoot::Direct {
                parent_cue_id: 17,
                generation: 3,
            },
            path: Arc::from(Vec::<ChildTimelineTransportPathSegment>::new()),
            position_ms: 100,
            playback_rate_milli,
            clip: TimelineAudioClipSummary {
                id: 44,
                layer_id: 5,
                media_asset_id: None,
                path: "rate-test.wav".to_owned(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 1_000,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus,
            },
        }],
        playing: true,
        position_ms: 100,
        transport_revision: 1,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    let mut cue_playback = MediaAudioPlayback::default();
    let cue_plan = cue_playback
        .plan_timeline_audio_sync(&snapshot_for(protocol::TimelineAudioOutputBus::Cue, 0))
        .expect("invalid CUE rate must stay isolated from PROGRAM settlement");
    assert!(cue_plan.prepares.is_empty());
    assert_eq!(cue_plan.errors, Vec::<String>::new());
    assert_eq!(cue_plan.cue_errors.len(), 1);
    assert!(cue_plan.cue_errors[0].contains("invalid-rate sentinel"));
    assert!(cue_playback.timeline_sinks.is_empty());

    let mut program_playback = MediaAudioPlayback::default();
    let error = match program_playback
        .plan_timeline_audio_sync(&snapshot_for(protocol::TimelineAudioOutputBus::Program, 0))
    {
        Ok(_) => panic!("invalid PROGRAM rate must reject Timeline settlement"),
        Err(error) => error,
    };
    assert!(error.contains("invalid-rate sentinel"));
    assert!(program_playback.timeline_sinks.is_empty());

    let mut cue_evidenced = MediaAudioPlayback::default();
    cue_evidenced
        .sync_to_timeline_audio_evidenced(&snapshot_for(protocol::TimelineAudioOutputBus::Cue, 0))
        .expect("test-only sync must retain production CUE-only settlement isolation");
    assert!(cue_evidenced
        .timeline_last_sync_error
        .as_deref()
        .is_some_and(|error| error.contains("invalid-rate sentinel")));

    let mut program_evidenced = MediaAudioPlayback::default();
    assert!(program_evidenced
        .sync_to_timeline_audio_evidenced(&snapshot_for(
            protocol::TimelineAudioOutputBus::Program,
            0,
        ))
        .is_err());
}

#[test]
fn timeline_audio_source_clock_uses_source_time_for_half_double_and_post_seek_resync() {
    let half = TimelineAudioSourceClock {
        source_anchor_ms: 1_000,
        output_anchor_ms: 200,
    };
    let half_normal = timeline_audio_source_position_at_output_time(half, 400, 500).unwrap();
    assert_eq!(half_normal, 1_100);
    assert_eq!(timeline_audio_source_drift_ms(half_normal, 1_100), 0);
    assert!(
        !media_audio_resync_required(
            timeline_audio_source_drift_ms(half_normal, 1_100),
            Duration::MAX,
        ),
        "a 0.5x normal output tick must not be mistaken for source drift"
    );
    assert!(media_audio_resync_required(
        timeline_audio_source_drift_ms(half_normal, 1_200),
        Duration::MAX,
    ));

    let double = TimelineAudioSourceClock {
        source_anchor_ms: 2_500,
        output_anchor_ms: 700,
    };
    let double_normal = timeline_audio_source_position_at_output_time(double, 825, 2_000).unwrap();
    assert_eq!(double_normal, 2_750);
    assert_eq!(timeline_audio_source_drift_ms(double_normal, 2_750), 0);
    assert!(
        !media_audio_resync_required(
            timeline_audio_source_drift_ms(double_normal, 2_750),
            Duration::MAX,
        ),
        "a 2x normal output tick must not be mistaken for source drift"
    );
    assert!(media_audio_resync_required(
        timeline_audio_source_drift_ms(double_normal, 2_650),
        Duration::MAX,
    ));

    let post_seek = TimelineAudioSourceClock {
        source_anchor_ms: 6_000,
        output_anchor_ms: 900,
    };
    assert_eq!(
        timeline_audio_source_position_at_output_time(post_seek, 975, 2_000).unwrap(),
        6_150,
        "a nonzero seek must replace both source and output anchors"
    );
    assert_eq!(
        timeline_audio_source_position_at_output_time(post_seek, 1_100, 2_000).unwrap(),
        6_400,
        "source-time conversion must stay correct after the reseek"
    );
}

#[test]
fn canonical_fractional_timeline_audio_rate_does_not_false_resync_after_190_seconds() {
    // Engine canonicalizes authored 1.2346x to the published 1235 millirate
    // before its child position clock advances. The Sink receives that same
    // 1.235x speed, so its output-time coordinate maps back to this exact
    // source position rather than the raw-float 1.2346x position.
    let canonical_source = timeline_audio_source_position_at_output_time(
        TimelineAudioSourceClock {
            source_anchor_ms: 0,
            output_anchor_ms: 0,
        },
        190_000,
        1_235,
    )
    .unwrap();
    assert_eq!(canonical_source, 234_650);
    assert!(
        !media_audio_resync_required(
            timeline_audio_source_drift_ms(canonical_source, 234_650),
            Duration::MAX,
        ),
        "the engine source clock and Sink clock must share canonical 1.235x"
    );

    let raw_authored_source = 234_574_u64;
    assert!(
        media_audio_resync_required(
            timeline_audio_source_drift_ms(canonical_source, raw_authored_source),
            Duration::MAX,
        ),
        "the old 1.2346x clock would drift 76 ms and trigger a false reseek"
    );
}

#[test]
fn nested_canonical_fractional_timeline_audio_rate_does_not_false_resync_after_339_seconds() {
    // The engine's nested 1.2346x * 1.2346x projection publishes 1525 and
    // the matching runtime-only audio source coordinate 516_975. Lighting's
    // legacy recursive local-rate position is 517_051, so using it here would
    // create a false 76 ms Sink re-seek.
    let canonical_source = timeline_audio_source_position_at_output_time(
        TimelineAudioSourceClock {
            source_anchor_ms: 0,
            output_anchor_ms: 0,
        },
        339_000,
        1_525,
    )
    .unwrap();
    assert_eq!(canonical_source, 516_975);
    assert!(
        !media_audio_resync_required(
            timeline_audio_source_drift_ms(canonical_source, 516_975),
            Duration::MAX,
        ),
        "the nested audio source projection must match the cumulative Sink clock"
    );
    assert!(
        media_audio_resync_required(
            timeline_audio_source_drift_ms(canonical_source, 517_051),
            Duration::MAX,
        ),
        "the legacy recursive lighting position must not drive audio resync"
    );
}

#[test]
fn timeline_audio_sink_seek_converts_authoritative_source_time_to_output_time() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!(
        "syndocal-timeline-audio-sink-seek-coordinate-{suffix}.wav"
    ));
    write_timeline_audio_test_wav_with_duration(&path, 16);

    let (mixer, mut mixer_source) = rodio::mixer::mixer(1, 8_000);
    let stop_consumer = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let stop_consumer_worker = Arc::clone(&stop_consumer);
    let consumer = std::thread::spawn(move || {
        while !stop_consumer_worker.load(std::sync::atomic::Ordering::Acquire) {
            let _ = mixer_source.next();
            std::thread::yield_now();
        }
    });

    for (speed_milli, expected_output_ms) in [(2_000, 3_000), (500, 12_000)] {
        let sink = rodio::Sink::connect_new(&mixer);
        let decoder = rodio::Decoder::try_from(fs::File::open(&path).unwrap()).unwrap();
        sink.append(decoder);
        sink.set_speed(validate_timeline_audio_speed(speed_milli).unwrap());
        sink.pause();

        seek_timeline_audio_sink_to_source_ms(&sink, 6_000, speed_milli).unwrap();
        assert_eq!(
            timeline_audio_sink_output_position_ms(&sink),
            expected_output_ms,
            "Rodio Sink must receive output-time, not raw 6000 ms source-time"
        );

        let post_seek = TimelineAudioSourceClock {
            source_anchor_ms: 6_000,
            output_anchor_ms: expected_output_ms,
        };
        assert_eq!(
            timeline_audio_source_position_at_output_time(
                post_seek,
                timeline_audio_sink_output_position_ms(&sink),
                speed_milli,
            )
            .unwrap(),
            6_000,
            "the installed source/output clock must remain exact immediately after seek"
        );
        sink.stop();
    }

    stop_consumer.store(true, std::sync::atomic::Ordering::Release);
    consumer.join().unwrap();
    fs::remove_file(path).unwrap();
}

#[test]
fn timeline_audio_sink_seek_coordinate_rounds_down_and_rejects_invalid_rates() {
    assert_eq!(
        timeline_audio_sink_output_position_for_source_ms(0, 500).unwrap(),
        0
    );
    assert_eq!(
        timeline_audio_sink_output_position_for_source_ms(6_000, 2_000).unwrap(),
        3_000
    );
    assert_eq!(
        timeline_audio_sink_output_position_for_source_ms(6_000, 500).unwrap(),
        12_000
    );
    assert_eq!(
        timeline_audio_sink_output_position_for_source_ms(1_001, 1_500).unwrap(),
        667,
        "fractional output coordinates round down"
    );
    assert_eq!(
        timeline_audio_source_position_at_output_time(
            TimelineAudioSourceClock {
                source_anchor_ms: 0,
                output_anchor_ms: 0,
            },
            667,
            1_500,
        )
        .unwrap(),
        1_000,
        "rounding down must not place the fixed-point source coordinate after the request"
    );
    assert_eq!(
        timeline_audio_sink_output_position_for_source_ms(u64::MAX, 250).unwrap(),
        u64::MAX,
        "overflow clamps at the only representable Sink millisecond coordinate"
    );
    for invalid in [0, 249, 4_001] {
        assert!(timeline_audio_sink_output_position_for_source_ms(1, invalid).is_err());
    }
}

#[test]
fn stale_rate_prepared_clip_is_retired_without_attaching_old_speed() {
    let suffix = current_unix_ms();
    let path =
        std::env::temp_dir().join(format!("syndocal-timeline-audio-stale-speed-{suffix}.wav"));
    write_timeline_audio_test_wav(&path);
    let (mixer, _mixer_source) = rodio::mixer::mixer(2, 48_000);
    let authority = engine::TimelineAudioProjectionAuthority {
        epoch: 3,
        generation: 8,
    };
    let key = TimelineAudioSinkKey::DirectChild {
        parent_cue_id: 17,
        generation: 9,
        path: Arc::from(Vec::<ChildTimelineTransportPathSegment>::new()),
        clip_id: 44,
    };
    let clip = TimelineAudioClipSummary {
        id: 44,
        layer_id: 5,
        media_asset_id: None,
        path: path.to_string_lossy().into_owned(),
        start_ms: 0,
        offset_ms: 125,
        duration_ms: 1_000,
        gain: 1.0,
        fade_in_ms: 0,
        fade_out_ms: 0,
        output_bus: protocol::TimelineAudioOutputBus::Program,
    };
    let stale_source = TimelineAudioSourceConfig {
        path: path.clone(),
        gain: 1.0,
        offset_ms: 125,
        speed_milli: 500,
        output_bus: protocol::TimelineAudioOutputBus::Program,
    };
    let stale_request = TimelineAudioPrepareRequest {
        key: key.clone(),
        clip: clip.clone(),
        source_position_ms: 225,
        volume: 1.0,
        source: stale_source,
    };
    let prepared = prepare_timeline_audio_clip(stale_request, &mixer).unwrap();
    let current = engine::TimelineAudioRuntimeSnapshot {
        child_clips: vec![engine::ChildTimelineAudioRuntimeClip {
            root: engine::ChildTimelineAudioRuntimeRoot::Direct {
                parent_cue_id: 17,
                generation: 9,
            },
            path: Arc::from(Vec::<ChildTimelineTransportPathSegment>::new()),
            position_ms: 100,
            playback_rate_milli: 2_000,
            clip,
        }],
        playing: true,
        position_ms: 100,
        transport_revision: 1,
        source_projection_authority: authority,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };
    let mut playback = MediaAudioPlayback {
        timeline_source_projection_authority: Some(authority),
        ..MediaAudioPlayback::default()
    };
    let plan = TimelineAudioSyncPlan {
        authority,
        device_generation: playback.audio_device_generation,
        hybrid_cue_output_generation: playback.hybrid_cue_output_generation,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        external_wdm_timeline_owner: playback.external_wdm_timeline_owner,
        requested_device_name: playback.requested_device_name.clone(),
        mixer: None,
        prepares: Vec::new(),
        seeks: Vec::new(),
        errors: Vec::new(),
        cue_errors: Vec::new(),
    };
    let mut prepared_output = None;
    playback
        .commit_timeline_audio_sync(
            plan,
            &current,
            &mut prepared_output,
            vec![Ok(prepared)],
            Vec::new(),
        )
        .unwrap();
    assert!(
        !playback.timeline_sinks.contains_key(&key),
        "a prepared 0.5x sink may not attach after authoritative 2x publication"
    );
    assert!(!playback.timeline_sources.contains_key(&key));
    assert!(!playback.timeline_source_clocks.contains_key(&key));
    let _ = fs::remove_file(path);
}

#[test]
fn timeline_audio_sink_domain_recreates_clip_id_on_seek_and_stops_on_pause_or_mute() {
    let (mixer, _mixer_source) = rodio::mixer::mixer(2, 48_000);
    let mut playback = MediaAudioPlayback::default();
    let install_clip_sink = |playback: &mut MediaAudioPlayback| {
        playback.timeline_sinks.insert(
            TimelineAudioSinkKey::Root(70),
            rodio::Sink::connect_new(&mixer),
        );
        playback.timeline_sources.insert(
            TimelineAudioSinkKey::Root(70),
            TimelineAudioSourceConfig {
                path: PathBuf::from("fixture.wav"),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 1_000,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
        );
    };

    assert!(playback.apply_timeline_transport_barrier(TimelineAudioTransportAction::Recue, false,));
    install_clip_sink(&mut playback);
    assert!(playback
        .timeline_sinks
        .contains_key(&TimelineAudioSinkKey::Root(70)));

    assert!(playback.apply_timeline_transport_barrier(TimelineAudioTransportAction::Sync, false,));
    assert!(playback
        .timeline_sinks
        .contains_key(&TimelineAudioSinkKey::Root(70)));

    assert!(playback.apply_timeline_transport_barrier(TimelineAudioTransportAction::Recue, false,));
    assert!(!playback
        .timeline_sinks
        .contains_key(&TimelineAudioSinkKey::Root(70)));
    install_clip_sink(&mut playback);
    assert!(playback
        .timeline_sinks
        .contains_key(&TimelineAudioSinkKey::Root(70)));

    assert!(!playback.apply_timeline_transport_barrier(TimelineAudioTransportAction::Sync, true,));
    assert!(playback.timeline_sinks.is_empty());
    install_clip_sink(&mut playback);
    assert!(!playback.apply_timeline_transport_barrier(TimelineAudioTransportAction::Stop, false,));
    assert!(playback.timeline_sinks.is_empty());
}

#[test]
fn timeline_audio_lane_audibility_stops_once_and_rearms_only_on_reappearance() {
    let (mixer, _mixer_source) = rodio::mixer::mixer(2, 48_000);
    let mut playback = MediaAudioPlayback::default();
    let key_a = TimelineAudioSinkKey::Root(70);
    let key_b = TimelineAudioSinkKey::Root(71);
    for (key, path) in [(key_a.clone(), "lane-a.wav"), (key_b.clone(), "lane-b.wav")] {
        playback
            .timeline_sinks
            .insert(key.clone(), rodio::Sink::connect_new(&mixer));
        playback.timeline_sources.insert(
            key,
            TimelineAudioSourceConfig {
                path: PathBuf::from(path),
                gain: 1.0,
                offset_ms: 0,
                speed_milli: 1_000,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            },
        );
    }
    playback.timeline_transport = TimelineAudioTransportState {
        playing: true,
        position_ms: 500,
        transport_revision: 7,
        synced_at: Some(Instant::now()),
    };
    let clip = |id, path: &str| TimelineAudioClipSummary {
        id,
        layer_id: id as u32,
        media_asset_id: Some(900),
        path: path.to_string(),
        start_ms: 0,
        offset_ms: 0,
        duration_ms: 2_000,
        gain: 1.0,
        fade_in_ms: 0,
        fade_out_ms: 0,
        output_bus: protocol::TimelineAudioOutputBus::Program,
    };
    let lane_a = clip(70, r"C:\syndocal-missing\audio-lane-a.wav");
    let lane_b = clip(71, "lane-b.wav");
    let mut audible = engine::TimelineAudioRuntimeSnapshot {
        clips: vec![lane_b.clone()],
        playing: true,
        position_ms: 500,
        transport_revision: 7,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    playback.sync_to_timeline_audio_evidenced(&audible).unwrap();
    assert_eq!(playback.timeline_stop_count, 1);
    assert!(!playback.timeline_sinks.contains_key(&key_a));
    assert!(playback.timeline_sinks.contains_key(&key_b));

    playback.sync_to_timeline_audio_evidenced(&audible).unwrap();
    assert_eq!(playback.timeline_stop_count, 1);
    assert_eq!(playback.timeline_start_attempt_count, 0);

    // Isolate the re-arm phase after proving B survived A's retirement;
    // the already-running B sink is not part of A's generation count.
    playback.timeline_sinks.remove(&key_b);
    playback.timeline_sources.remove(&key_b);
    audible.clips = vec![lane_a];
    playback.timeline_transport.synced_at = Some(Instant::now());
    let _ = playback.sync_to_timeline_audio_evidenced(&audible);
    assert_eq!(playback.timeline_start_attempt_count, 1);
    playback.timeline_transport.synced_at = Some(Instant::now());
    let _ = playback.sync_to_timeline_audio_evidenced(&audible);
    assert_eq!(
        playback.timeline_start_attempt_count, 1,
        "an unchanged visible lane must not be regenerated after the first source attempt"
    );

    audible.clips.clear();
    playback.timeline_transport.synced_at = Some(Instant::now());
    playback.sync_to_timeline_audio_evidenced(&audible).unwrap();
    assert_eq!(playback.timeline_stop_count, 1);
}

#[test]
fn timeline_audio_projection_gap_retires_once_and_rearms_cached_failure_once() {
    let (mixer, _mixer_source) = rodio::mixer::mixer(2, 48_000);
    let mut playback = MediaAudioPlayback::default();
    let key = TimelineAudioSinkKey::Root(90);
    playback
        .timeline_sinks
        .insert(key.clone(), rodio::Sink::connect_new(&mixer));
    playback.timeline_sources.insert(
        key,
        TimelineAudioSourceConfig {
            path: PathBuf::from("same-source.wav"),
            gain: 1.0,
            offset_ms: 0,
            speed_milli: 1_000,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        },
    );
    playback.timeline_source_projection_authority =
        Some(engine::TimelineAudioProjectionAuthority {
            epoch: 1,
            generation: 1,
        });
    playback.timeline_transport = TimelineAudioTransportState {
        playing: true,
        position_ms: 500,
        transport_revision: 2,
        synced_at: Some(Instant::now()),
    };
    let timeline = engine::TimelineAudioRuntimeSnapshot {
        clips: vec![TimelineAudioClipSummary {
            id: 90,
            layer_id: 9,
            media_asset_id: None,
            path: r"C:\syndocal-missing\same-source.wav".to_string(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        }],
        playing: true,
        position_ms: 500,
        transport_revision: 2,
        source_projection_authority: engine::TimelineAudioProjectionAuthority {
            epoch: 1,
            generation: 3,
        },
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    assert!(playback
        .sync_to_timeline_audio_evidenced(&timeline)
        .is_err());
    assert_eq!(playback.timeline_stop_count, 1);
    assert_eq!(playback.timeline_start_attempt_count, 1);
    playback.timeline_transport.synced_at = Some(Instant::now());
    assert!(playback
        .sync_to_timeline_audio_evidenced(&timeline)
        .is_err());
    assert_eq!(playback.timeline_stop_count, 1);
    assert_eq!(
        playback.timeline_start_attempt_count, 1,
        "the current generation must retain exactly one cached open failure"
    );

    let mut stale = timeline;
    stale.source_projection_authority = engine::TimelineAudioProjectionAuthority {
        epoch: 1,
        generation: 2,
    };
    assert!(playback.sync_to_timeline_audio_evidenced(&stale).is_err());
    assert!(playback.timeline_sinks.is_empty());
    assert!(playback.timeline_failures.is_empty());
}

#[test]
fn timeline_audio_decoder_fence_rejects_stale_source_before_sink_start() {
    let suffix = current_unix_ms();
    let path = std::env::temp_dir().join(format!(
        "syndocal-timeline-audio-projection-fence-{suffix}.wav"
    ));
    write_timeline_audio_test_wav(&path);
    let mut playback = MediaAudioPlayback::default();
    let timeline = engine::TimelineAudioRuntimeSnapshot {
        clips: vec![TimelineAudioClipSummary {
            id: 91,
            layer_id: 9,
            media_asset_id: None,
            path: path.to_string_lossy().into_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        }],
        playing: true,
        position_ms: 100,
        source_projection_authority: engine::TimelineAudioProjectionAuthority {
            epoch: 2,
            generation: 4,
        },
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    let error = playback
        .sync_to_timeline_audio_evidenced_with_fence(&timeline, || false)
        .unwrap_err();
    assert!(error.contains("changed while its decoder was opening"));
    assert!(playback.timeline_sinks.is_empty());
    assert!(playback.timeline_sources.is_empty());
    let _ = fs::remove_file(path);
}

#[test]
fn timeline_audio_output_bus_change_retires_old_sink_before_each_logical_rebuild() {
    let suffix = current_unix_ms();
    let path =
        std::env::temp_dir().join(format!("syndocal-timeline-audio-logical-bus-{suffix}.wav"));
    write_timeline_audio_test_wav(&path);
    let (mixer, _mixed) = rodio::mixer::mixer(1, 8_000);
    let mut playback = MediaAudioPlayback {
        timeline_test_mixer: Some(mixer),
        ..MediaAudioPlayback::default()
    };
    let mut timeline = engine::TimelineAudioRuntimeSnapshot {
        clips: vec![TimelineAudioClipSummary {
            id: 92,
            layer_id: 9,
            media_asset_id: None,
            path: path.to_string_lossy().into_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 2_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        }],
        playing: true,
        // This test proves logical bus retirement/rebuild, not seeking. The
        // injected mixer has no callback thread, so a non-zero Sink::try_seek
        // would wait forever for an audio consumer that does not exist in
        // this in-memory seam.
        position_ms: 0,
        source_projection_authority: engine::TimelineAudioProjectionAuthority {
            epoch: 2,
            generation: 4,
        },
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };

    playback
        .sync_to_timeline_audio_evidenced(&timeline)
        .unwrap();
    let key = TimelineAudioSinkKey::Root(92);
    assert_eq!(playback.timeline_start_attempt_count, 1);
    assert_eq!(playback.timeline_stop_count, 0);
    assert_eq!(
        playback.timeline_sources.get(&key).unwrap().output_bus,
        protocol::TimelineAudioOutputBus::Program
    );

    timeline.clips[0].output_bus = protocol::TimelineAudioOutputBus::Cue;
    timeline.source_projection_authority.generation = 5;
    playback
        .sync_to_timeline_audio_evidenced(&timeline)
        .unwrap();
    assert_eq!(playback.timeline_stop_count, 1);
    assert_eq!(playback.timeline_start_attempt_count, 2);
    assert_eq!(
        playback.timeline_sources.get(&key).unwrap().output_bus,
        protocol::TimelineAudioOutputBus::Cue,
        "the committed CUE reconstruction cannot retain an old PROGRAM source"
    );

    timeline.clips[0].output_bus = protocol::TimelineAudioOutputBus::Program;
    timeline.source_projection_authority.generation = 6;
    playback
        .sync_to_timeline_audio_evidenced(&timeline)
        .unwrap();
    assert_eq!(playback.timeline_stop_count, 2);
    assert_eq!(playback.timeline_start_attempt_count, 3);
    assert_eq!(
        playback.timeline_sources.get(&key).unwrap().output_bus,
        protocol::TimelineAudioOutputBus::Program,
        "the committed PROGRAM reconstruction cannot retain an old CUE source"
    );
    let _ = fs::remove_file(path);
}

#[test]
fn cue_only_prepare_failure_stays_visible_without_faulting_program_follow_settlement() {
    let (mixer, _mixed) = rodio::mixer::mixer(2, 48_000);
    let program_key = TimelineAudioSinkKey::Root(201);
    let cue_key = TimelineAudioSinkKey::Root(202);
    let authority = engine::TimelineAudioProjectionAuthority {
        epoch: 8,
        generation: 13,
    };
    let mut playback = MediaAudioPlayback {
        timeline_source_projection_authority: Some(authority),
        ..MediaAudioPlayback::default()
    };
    playback
        .timeline_sinks
        .insert(program_key.clone(), rodio::Sink::connect_new(&mixer));
    playback.timeline_sources.insert(
        program_key.clone(),
        TimelineAudioSourceConfig {
            path: PathBuf::from("program.wav"),
            gain: 1.0,
            offset_ms: 0,
            speed_milli: 1_000,
            output_bus: protocol::TimelineAudioOutputBus::Program,
        },
    );
    let request = TimelineAudioPrepareRequest {
        key: cue_key.clone(),
        clip: TimelineAudioClipSummary {
            id: 202,
            layer_id: 2,
            media_asset_id: None,
            path: "cue.wav".to_owned(),
            start_ms: 0,
            offset_ms: 0,
            duration_ms: 1_000,
            gain: 1.0,
            fade_in_ms: 0,
            fade_out_ms: 0,
            output_bus: protocol::TimelineAudioOutputBus::Cue,
        },
        source_position_ms: 0,
        volume: 1.0,
        source: TimelineAudioSourceConfig {
            path: PathBuf::from("cue.wav"),
            gain: 1.0,
            offset_ms: 0,
            speed_milli: 1_000,
            output_bus: protocol::TimelineAudioOutputBus::Cue,
        },
    };
    let plan = TimelineAudioSyncPlan {
        authority,
        device_generation: playback.audio_device_generation,
        hybrid_cue_output_generation: playback.hybrid_cue_output_generation,
        #[cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]
        external_wdm_timeline_owner: playback.external_wdm_timeline_owner,
        requested_device_name: playback.requested_device_name.clone(),
        mixer: None,
        prepares: Vec::new(),
        seeks: Vec::new(),
        errors: Vec::new(),
        cue_errors: Vec::new(),
    };
    let current = engine::TimelineAudioRuntimeSnapshot {
        source_projection_authority: authority,
        ..engine::TimelineAudioRuntimeSnapshot::default()
    };
    let mut prepared_output = None;

    playback
        .commit_timeline_audio_sync(
            plan,
            &current,
            &mut prepared_output,
            vec![Err(Box::new((
                request,
                "Explicit WDM CUE output is selected but is not publishable".to_owned(),
            )))],
            Vec::new(),
        )
        .expect("a CUE-only fault must not fail PROGRAM/Follow settlement");

    assert!(playback.timeline_sinks.contains_key(&program_key));
    assert!(playback.timeline_sources.contains_key(&program_key));
    assert!(playback.timeline_failures.contains_key(&cue_key));
    assert!(playback
        .timeline_last_sync_error
        .as_deref()
        .is_some_and(|error| error.contains("not publishable")));
}

#[test]
fn child_timeline_audio_sink_keys_isolate_parent_activation_and_clip_id() {
    let root_path = Arc::from([]);
    let follow_target_root = TimelineAudioSinkKey::Follow {
        source_timeline_id: TimelineId(10),
        target_timeline_id: TimelineId(11),
        generation: 4,
        target_root: true,
        path: Arc::clone(&root_path),
        clip_id: 7,
    };
    let follow_child_root = TimelineAudioSinkKey::Follow {
        source_timeline_id: TimelineId(10),
        target_timeline_id: TimelineId(11),
        generation: 4,
        target_root: false,
        path: Arc::clone(&root_path),
        clip_id: 7,
    };
    let nested_left = Arc::from([ChildTimelineTransportPathSegment {
        event_id: 200,
        iteration: 0,
    }]);
    let nested_right = Arc::from([ChildTimelineTransportPathSegment {
        event_id: 201,
        iteration: 0,
    }]);
    let mut keys = HashSet::from([
        TimelineAudioSinkKey::Root(7),
        TimelineAudioSinkKey::Child {
            parent_event_id: 100,
            parent_iteration: 0,
            path: Arc::clone(&root_path),
            clip_id: 7,
        },
        follow_target_root.clone(),
        follow_child_root,
        TimelineAudioSinkKey::Child {
            parent_event_id: 101,
            parent_iteration: 0,
            path: Arc::clone(&root_path),
            clip_id: 7,
        },
        TimelineAudioSinkKey::Child {
            parent_event_id: 100,
            parent_iteration: 1,
            path: Arc::clone(&root_path),
            clip_id: 7,
        },
        TimelineAudioSinkKey::Child {
            parent_event_id: 100,
            parent_iteration: 0,
            path: nested_left,
            clip_id: 7,
        },
        TimelineAudioSinkKey::Child {
            parent_event_id: 100,
            parent_iteration: 0,
            path: nested_right,
            clip_id: 7,
        },
        TimelineAudioSinkKey::DirectChild {
            parent_cue_id: 12,
            generation: 3,
            path: root_path,
            clip_id: 7,
        },
    ]);
    assert_eq!(keys.len(), 9);
    assert!(
        !keys.insert(follow_target_root),
        "an exact Follow provenance retry must resolve to the same sink key"
    );
    assert_eq!(keys.len(), 9);
}

#[test]
fn timeline_audio_clip_gain_envelope_combines_fades_and_gain() {
    let clip = TimelineAudioClipSummary {
        id: 7,
        layer_id: 10,
        media_asset_id: None,
        path: "fixture.wav".to_string(),
        start_ms: 1_000,
        offset_ms: 0,
        duration_ms: 2_000,
        gain: 1.5,
        fade_in_ms: 500,
        fade_out_ms: 500,
        output_bus: protocol::TimelineAudioOutputBus::Program,
    };

    assert_eq!(timeline_audio_clip_volume(&clip, 1_000), 0.0);
    assert!((timeline_audio_clip_volume(&clip, 1_250) - 0.75).abs() < 0.001);
    assert!((timeline_audio_clip_volume(&clip, 2_000) - 1.5).abs() < 0.001);
    assert!((timeline_audio_clip_volume(&clip, 2_750) - 0.75).abs() < 0.001);
    assert_eq!(timeline_audio_clip_volume(&clip, 3_000), 0.0);
}

#[test]
fn program_audio_handoff_plans_each_auto_action_exactly_once() {
    let config =
        ProgramAudioHandoffConfig::validated(true, 1.25, Some("  ASIO Main  ".to_string()))
            .unwrap();
    let mut handoff = ProgramAudioHandoffState::default();
    handoff.configure(config, None);
    let first = auto_vj_action(1, 5);
    let mut status = protocol::AutoVjStatus {
        armed: true,
        last_action: Some(first.clone()),
        ..protocol::AutoVjStatus::default()
    };

    assert_eq!(
        handoff.next_plan(&status),
        Some(ProgramAudioHandoffPlan {
            layer_id: 5,
            volume: 1.25,
            device_name: Some("ASIO Main".to_string()),
        })
    );
    assert_eq!(handoff.next_plan(&status), None);

    status.last_action = Some(auto_vj_action(2, 9));
    assert_eq!(handoff.next_plan(&status).unwrap().layer_id, 9);
    assert_eq!(handoff.next_plan(&status), None);
}

#[test]
fn manual_take_barrier_suppresses_an_unobserved_auto_audio_action() {
    let mut handoff = ProgramAudioHandoffState::default();
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 0.8, None).unwrap(),
        None,
    );
    let action = auto_vj_action(3, 7);
    handoff.suppress_through(Some(&action));
    let status = protocol::AutoVjStatus {
        armed: true,
        hold: true,
        last_action: Some(action),
        ..protocol::AutoVjStatus::default()
    };

    assert_eq!(handoff.next_plan(&status), None);
}

#[test]
fn stopping_one_layer_does_not_consume_another_layers_unseen_action() {
    let previous = auto_vj_action(3, 7);
    let unseen = auto_vj_action(4, 9);
    let mut handoff = ProgramAudioHandoffState::default();
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 0.8, None).unwrap(),
        Some(&previous),
    );
    handoff.suppress_stopped_layer_action(previous.layer_id, Some(&unseen));
    let status = protocol::AutoVjStatus {
        armed: true,
        last_action: Some(unseen),
        ..protocol::AutoVjStatus::default()
    };

    assert_eq!(handoff.next_plan(&status).unwrap().layer_id, 9);
}

#[test]
fn program_audio_config_rejects_non_finite_volume_and_does_not_replay_current_action() {
    assert!(ProgramAudioHandoffConfig::validated(true, f32::NAN, None).is_err());
    let action = auto_vj_action(4, 11);
    let mut handoff = ProgramAudioHandoffState::default();
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 3.0, Some("  ".to_string())).unwrap(),
        Some(&action),
    );
    let status = protocol::AutoVjStatus {
        armed: true,
        last_action: Some(action),
        ..protocol::AutoVjStatus::default()
    };

    assert_eq!(handoff.config.volume, 2.0);
    assert_eq!(handoff.config.device_name, None);
    assert_eq!(handoff.next_plan(&status), None);
    assert_eq!(
        handoff.manual_take_plan(12),
        Some(ProgramAudioHandoffPlan {
            layer_id: 12,
            volume: 2.0,
            device_name: None,
        })
    );
}

#[test]
fn enabled_program_audio_config_change_preserves_an_unseen_action() {
    let previous_action = auto_vj_action(5, 3);
    let unseen_action = auto_vj_action(6, 8);
    let mut handoff = ProgramAudioHandoffState::default();
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 0.8, None).unwrap(),
        Some(&previous_action),
    );
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 1.1, Some("Main".to_string())).unwrap(),
        Some(&unseen_action),
    );
    let status = protocol::AutoVjStatus {
        armed: true,
        last_action: Some(unseen_action),
        ..protocol::AutoVjStatus::default()
    };

    let plan = handoff.next_plan(&status).unwrap();
    assert_eq!(plan.layer_id, 8);
    assert_eq!(plan.volume, 1.1);
    assert_eq!(plan.device_name.as_deref(), Some("Main"));
}

#[test]
fn program_audio_gain_reconfiguration_preserves_mix_ratio() {
    assert_eq!(reconfigured_program_audio_volume(0.4, 0.8, 1.6, 1.0), 0.8);
    assert_eq!(reconfigured_program_audio_volume(0.0, 0.0, 1.0, 0.25), 0.5);
    assert_eq!(reconfigured_program_audio_volume(2.0, 0.5, 2.0, 1.0), 2.0);
}

#[test]
fn newer_program_audio_job_retires_the_previous_generation() {
    let coordinator = ProgramAudioHandoffCoordinator::default();
    let mut handoff = coordinator.state.lock().unwrap();
    let first_generation = coordinator.queue_locked(
        &mut handoff,
        ProgramAudioJobKind::Handoff(ProgramAudioHandoffPlan {
            layer_id: 5,
            volume: 0.8,
            device_name: None,
        }),
    );
    let second_generation = coordinator.queue_locked(&mut handoff, ProgramAudioJobKind::StopAll);

    assert!(second_generation > first_generation);
    assert_eq!(
        coordinator.generation.load(Ordering::Acquire),
        second_generation
    );
    assert_eq!(
        handoff.pending_job,
        Some(ProgramAudioJob {
            generation: second_generation,
            kind: ProgramAudioJobKind::StopAll,
        })
    );
}

#[test]
fn rapid_program_audio_reenable_keeps_the_pending_stop_barrier() {
    let current = auto_vj_action(8, 13);
    let coordinator = ProgramAudioHandoffCoordinator::default();
    let mut handoff = coordinator.state.lock().unwrap();
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 0.8, None).unwrap(),
        None,
    );
    handoff.configure(
        ProgramAudioHandoffConfig::validated(false, 0.8, None).unwrap(),
        Some(&current),
    );
    let generation = coordinator.queue_locked(&mut handoff, ProgramAudioJobKind::StopAll);
    handoff.configure(
        ProgramAudioHandoffConfig::validated(true, 0.8, None).unwrap(),
        Some(&current),
    );

    assert_eq!(coordinator.generation.load(Ordering::Acquire), generation);
    assert_eq!(
        handoff.pending_job,
        Some(ProgramAudioJob {
            generation,
            kind: ProgramAudioJobKind::StopAll,
        })
    );
}
