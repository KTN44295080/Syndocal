use super::*;

fn write_timeline_audio_test_wav(path: &Path) {
    let sample_rate = 8_000_u32;
    let sample_count = sample_rate * 2;
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
    );
    let mut prepared = prepared.pop().unwrap().unwrap();
    assert_eq!(prepared.request.source_position_ms, 100);
    assert!(prepared.decoder.is_none());
    prepared.stop();
    let _ = fs::remove_file(path);
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
