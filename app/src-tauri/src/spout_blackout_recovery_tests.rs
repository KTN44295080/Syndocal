// Included inside spout_transport::tests. These tests use the production
// authority, renderer, materialization, and worker-loop seams while replacing
// only the SDK sender and video frame provider with deterministic fakes.

const RECOVERY_PIXEL: [u8; 4] = [24, 96, 180, 255];

#[derive(Default)]
struct RecordingSpoutSender {
    frames: Vec<(Vec<u8>, u32, u32)>,
}

impl SpoutOutputSender for RecordingSpoutSender {
    fn sender_name(&self) -> String {
        "Injected Spout recovery".to_string()
    }

    fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        self.frames.push((pixels.to_vec(), width, height));
        Ok(())
    }
}

struct SharedRecoverySpoutSender {
    frames: Arc<Mutex<Vec<(Vec<u8>, u32, u32)>>>,
    admitted: Arc<AtomicBool>,
}

impl SpoutOutputSender for SharedRecoverySpoutSender {
    fn sender_name(&self) -> String {
        "Injected Spout worker recovery".to_string()
    }

    fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        match self.frames.lock() {
            Ok(mut frames) => frames.push((pixels.to_vec(), width, height)),
            Err(poisoned) => poisoned.into_inner().push((pixels.to_vec(), width, height)),
        }
        self.admitted.store(true, Ordering::Release);
        Ok(())
    }
}

struct SolidRecoveryVideoFrameProvider;

impl video::VideoFrameProvider for SolidRecoveryVideoFrameProvider {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn frame_for_layer(
        &mut self,
        layer: &protocol::VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<video::VideoFrame, video::VideoFrameProviderError> {
        Ok(video::VideoFrame {
            layer_id: layer.id,
            width,
            height,
            pts_ms: layer.state.position_ms,
            duration_ms: 16,
            format: video::VideoPixelFormat::Rgba8,
            data: RECOVERY_PIXEL.repeat(width as usize * height as usize),
        })
    }
}

fn recovery_engine(output_id: VideoOutputId) -> EngineHandle {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let layer_id = engine.allocate_video_layer_id();
    engine
        .send(engine::EngineCommand::AddVideoLayer {
            layer_id,
            label: "Spout blackout recovery layer".to_string(),
            source: protocol::VideoSourceSummary {
                kind: protocol::VideoSourceKind::File,
                path: Some("memory://spout-blackout-recovery.mp4".to_string()),
                name: None,
                codec: None,
                metadata: None,
            },
        })
        .unwrap();
    install_injected_spout_output_extent(&engine, output_id, 2, 1);
    assert!(
        engine
            .snapshot()
            .video
            .layers
            .iter()
            .any(|layer| layer.id == layer_id),
        "the routed recovery layer must publish before the Spout output"
    );
    engine
}

fn render_recovery_frame(
    engine: &EngineHandle,
    output_id: VideoOutputId,
    renderer: &mut video::VideoPreviewRenderer<SolidRecoveryVideoFrameProvider>,
) -> Result<
    (
        TimelineFollowOutputRenderDecision,
        OutputPresentationAuthority,
    ),
    String,
> {
    let sample = engine.video_presentation_sample();
    let authority = capture_output_presentation_authority(
        "Spout",
        protocol::VideoOutputKind::SpoutSender,
        &sample,
        engine.output_ownership_status(),
        engine.safety_blackout_authority(),
        output_id,
        &injected_spout_endpoint(output_id),
    )?;
    if authority.project_blackout() {
        return Err("recovery callback captured a blackout authority".to_string());
    }
    revalidate_output_presentation_authority(engine, "Spout", &authority)?;
    let artistic = renderer
        .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
            &sample.snapshot.video,
            spout_output_effect_render_context(&sample.snapshot, authority.ownership_epoch()),
            &sample.snapshot.video_transition_runtime,
            output_id,
            authority.output_summary().width,
            authority.output_summary().height,
        )
        .map_err(|error| format!("Spout recovery artistic render failed: {error:?}"))?;
    if artistic.evidence.freshness != video::VideoOutputRenderFreshness::Fresh {
        return Err(format!(
            "Spout recovery artistic render was not fresh: {:?}",
            artistic.evidence.freshness
        ));
    }
    if matches!(
        &artistic.payload,
        video::VideoOutputArtisticPayload::HardBlackout
    ) {
        return Err("Spout recovery artistic render produced hard black".to_string());
    }
    let materialized = materialize_output_artistic_result(artistic, &authority)?;
    let key = TimelineFollowOutputFrameKey {
        epoch: authority.ownership_epoch(),
        generation: 0,
        output_id,
        width: materialized.width,
        height: materialized.height,
    };
    Ok((
        TimelineFollowOutputRenderDecision::Frame {
            key,
            frame: materialized,
            result: TimelineFollowSettlementAckResult::Applied,
            follow_identity: None,
        },
        authority,
    ))
}

struct RecoveryWorkerGuard {
    worker: Option<std::thread::JoinHandle<Result<(), SpoutOutputWorkerStopError>>>,
    stop: Arc<AtomicBool>,
    release_first_render: Arc<AtomicBool>,
    release_fresh_send: Arc<AtomicBool>,
    release_after_send: Arc<AtomicBool>,
    teardown_lease: Arc<Mutex<Option<engine::OutputOwnershipTeardownLease>>>,
    failure_lease: Arc<Mutex<Option<engine::OutputOwnershipTeardownLease>>>,
}

impl RecoveryWorkerGuard {
    fn join(&mut self) -> Result<(), String> {
        self.stop.store(true, Ordering::Release);
        self.release_first_render.store(true, Ordering::Release);
        self.release_fresh_send.store(true, Ordering::Release);
        self.release_after_send.store(true, Ordering::Release);
        let worker = self
            .worker
            .take()
            .ok_or_else(|| "Spout recovery worker was already joined".to_string())?;
        let result = worker
            .join()
            .map_err(|_| "Spout recovery worker panicked".to_string())?;
        result.map_err(|error| error.message)
    }

    fn take_teardown_lease(&self) -> Option<engine::OutputOwnershipTeardownLease> {
        match self.teardown_lease.lock() {
            Ok(mut slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }

    fn take_failure_lease(&self) -> Option<engine::OutputOwnershipTeardownLease> {
        match self.failure_lease.lock() {
            Ok(mut slot) => slot.take(),
            Err(poisoned) => poisoned.into_inner().take(),
        }
    }
}

impl Drop for RecoveryWorkerGuard {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        self.release_first_render.store(true, Ordering::Release);
        self.release_fresh_send.store(true, Ordering::Release);
        self.release_after_send.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        drop(self.take_teardown_lease());
        drop(self.take_failure_lease());
    }
}

fn wait_for_recovery_flag(flag: &AtomicBool, description: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !flag.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "timed out waiting for {description}"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn spout_video_blackout_release_rerenders_fresh_artistic_frame() {
    let output_id = 98_209;
    let engine = recovery_engine(output_id);
    let target = protocol::control_plane_command::OutputControlTargetRoleV1::Video;
    let old_authority = injected_spout_authority(&engine, output_id);
    assert!(!old_authority.project_blackout());
    let mut sender = RecordingSpoutSender::default();

    engine
        .set_output_blackout_published(target, true, Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(engine.snapshot().video.blackout);
    let error = send_frame_if_authorized("Spout", &engine, &old_authority, None, || {
        sender.send_image(&RECOVERY_PIXEL, 2, 1)
    })
    .expect_err("the old authority must be revoked while Video blackout is on");
    assert!(matches!(error, PhysicalOutputSendError::Revoked(_)));
    assert!(
        sender.frames.is_empty(),
        "a revoked authority must not enter the Spout SDK sender"
    );

    engine
        .set_output_blackout_published(target, false, Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(!engine.snapshot().video.blackout);
    assert_ne!(
        engine.video_presentation_config_token(),
        old_authority.presentation_config_token(),
        "Video blackout ON then OFF must leave the old presentation authority stale"
    );
    let error = send_frame_if_authorized("Spout", &engine, &old_authority, None, || {
        sender.send_image(&RECOVERY_PIXEL, 2, 1)
    })
    .expect_err("turning Video blackout off must not revive the old authority");
    assert!(matches!(error, PhysicalOutputSendError::Revoked(_)));
    assert!(
        sender.frames.is_empty(),
        "the stale post-release authority must not enter the Spout SDK sender"
    );

    let fresh_authority = injected_spout_authority(&engine, output_id);
    assert!(!fresh_authority.project_blackout());
    let sample = engine.video_presentation_sample();
    let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
        video::VideoRuntimeConfig {
            frame_queue_capacity: 2,
            preview_width: 2,
            preview_height: 1,
        },
        SolidRecoveryVideoFrameProvider,
    );
    let expected_pixels = RECOVERY_PIXEL.repeat(2);
    let artistic = renderer
        .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
            &sample.snapshot.video,
            spout_output_effect_render_context(&sample.snapshot, fresh_authority.ownership_epoch()),
            &sample.snapshot.video_transition_runtime,
            output_id,
            fresh_authority.output_summary().width,
            fresh_authority.output_summary().height,
        )
        .expect("the released Video blackout must use the artistic renderer");
    assert_eq!(
        artistic.evidence.freshness,
        video::VideoOutputRenderFreshness::Fresh
    );
    match &artistic.payload {
        video::VideoOutputArtisticPayload::Frame(frame) => {
            assert_eq!(frame.data.as_slice(), expected_pixels.as_slice());
        }
        video::VideoOutputArtisticPayload::HardBlackout => {
            panic!("released Video blackout must not produce a hard-black payload")
        }
    }

    let materialized = materialize_output_artistic_result(artistic, &fresh_authority)
        .expect("fresh artistic result must materialize at the Spout boundary");
    send_frame_if_authorized("Spout", &engine, &fresh_authority, None, || {
        sender.send_image(&materialized.data, materialized.width, materialized.height)
    })
    .expect("the fresh authority must admit the artistic Spout frame");
    assert_eq!(sender.frames.len(), 1);
    let (pixels, width, height) = &sender.frames[0];
    assert_eq!((*width, *height), (2, 1));
    assert_eq!(pixels.as_slice(), materialized.data.as_slice());
    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel != [0, 0, 0, 255]),
        "the released Video blackout must send artistic pixels, not hard black"
    );
}

#[test]
fn spout_worker_video_blackout_release_reacquires_authority_and_sends_artistic_frame() {
    let output_id = 98_210;
    let engine = recovery_engine(output_id);
    let stop = Arc::new(AtomicBool::new(false));
    let release_first_render = Arc::new(AtomicBool::new(false));
    let release_fresh_send = Arc::new(AtomicBool::new(false));
    let release_after_send = Arc::new(AtomicBool::new(false));
    let first_render_ready = Arc::new(AtomicBool::new(false));
    let fresh_render_ready = Arc::new(AtomicBool::new(false));
    let after_send_ready = Arc::new(AtomicBool::new(false));
    let admitted_send = Arc::new(AtomicBool::new(false));
    let render_count = Arc::new(AtomicUsize::new(0));
    let first_token = Arc::new(Mutex::new(None));
    let fresh_token = Arc::new(Mutex::new(None));
    let frames = Arc::new(Mutex::new(Vec::new()));
    let teardown_lease = Arc::new(Mutex::new(None));
    let failure_lease = Arc::new(Mutex::new(None));

    let worker_engine = engine.clone();
    let render_engine = engine.clone();
    let worker_stop = Arc::clone(&stop);
    let render_stop = Arc::clone(&stop);
    let worker_release_first = Arc::clone(&release_first_render);
    let worker_release_fresh = Arc::clone(&release_fresh_send);
    let worker_release_after_send = Arc::clone(&release_after_send);
    let worker_first_ready = Arc::clone(&first_render_ready);
    let worker_fresh_ready = Arc::clone(&fresh_render_ready);
    let worker_after_send_ready = Arc::clone(&after_send_ready);
    let worker_render_count = Arc::clone(&render_count);
    let worker_first_token = Arc::clone(&first_token);
    let worker_fresh_token = Arc::clone(&fresh_token);
    let worker_teardown_lease = Arc::clone(&teardown_lease);
    let worker_failure_lease = Arc::clone(&failure_lease);
    let worker_frames = Arc::clone(&frames);
    let worker_admitted_send = Arc::clone(&admitted_send);
    let worker = std::thread::spawn(move || {
        let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
            video::VideoRuntimeConfig {
                frame_queue_capacity: 2,
                preview_width: 2,
                preview_height: 1,
            },
            SolidRecoveryVideoFrameProvider,
        );
        run_spout_output_worker(
            output_id,
            "Injected Spout worker recovery",
            worker_engine,
            worker_stop,
            SharedRecoverySpoutSender {
                frames: worker_frames,
                admitted: worker_admitted_send,
            },
            (worker_teardown_lease, worker_failure_lease),
            None,
            move |_| {
                let (decision, authority) =
                    render_recovery_frame(&render_engine, output_id, &mut renderer)?;
                let invocation = worker_render_count.fetch_add(1, Ordering::AcqRel);
                if invocation == 0 {
                    *worker_first_token.lock().unwrap() =
                        Some(authority.presentation_config_token());
                    worker_first_ready.store(true, Ordering::Release);
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !worker_release_first.load(Ordering::Acquire)
                        && !render_stop.load(Ordering::Acquire)
                    {
                        if Instant::now() >= deadline {
                            return Err(
                                "timed out waiting to release the stale Spout render".to_string()
                            );
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    if !worker_release_first.load(Ordering::Acquire) {
                        return Err(
                            "Spout recovery worker was stopped before stale render release"
                                .to_string(),
                        );
                    }
                } else if invocation == 1 {
                    *worker_fresh_token.lock().unwrap() =
                        Some(authority.presentation_config_token());
                    worker_fresh_ready.store(true, Ordering::Release);
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !worker_release_fresh.load(Ordering::Acquire)
                        && !render_stop.load(Ordering::Acquire)
                    {
                        if Instant::now() >= deadline {
                            return Err(
                                "timed out waiting to release the fresh Spout render".to_string()
                            );
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    if !worker_release_fresh.load(Ordering::Acquire) {
                        return Err(
                            "Spout recovery worker was stopped before fresh send release"
                                .to_string(),
                        );
                    }
                } else if invocation == 2 {
                    worker_after_send_ready.store(true, Ordering::Release);
                    let deadline = Instant::now() + Duration::from_secs(5);
                    while !worker_release_after_send.load(Ordering::Acquire) {
                        if Instant::now() >= deadline {
                            return Err(
                                "timed out waiting to retire the admitted Spout worker".to_string()
                            );
                        }
                        std::thread::sleep(Duration::from_millis(1));
                    }
                    return Err("Spout recovery worker stopping after admitted frame".to_string());
                }
                Ok((decision, Some(authority)))
            },
        )
    });
    let mut guard = RecoveryWorkerGuard {
        worker: Some(worker),
        stop: Arc::clone(&stop),
        release_first_render: Arc::clone(&release_first_render),
        release_fresh_send: Arc::clone(&release_fresh_send),
        release_after_send: Arc::clone(&release_after_send),
        teardown_lease: Arc::clone(&teardown_lease),
        failure_lease: Arc::clone(&failure_lease),
    };

    wait_for_recovery_flag(&first_render_ready, "the worker's first Spout render");
    let captured_first_token = first_token
        .lock()
        .unwrap()
        .expect("the first worker render must capture an authority token");
    let target = protocol::control_plane_command::OutputControlTargetRoleV1::Video;
    engine
        .set_output_blackout_published(target, true, Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(engine.snapshot().video.blackout);
    engine
        .set_output_blackout_published(target, false, Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(!engine.snapshot().video.blackout);
    assert_ne!(
        engine.video_presentation_config_token(),
        captured_first_token,
        "Video blackout ON then OFF must invalidate the worker's first authority"
    );

    release_first_render.store(true, Ordering::Release);
    wait_for_recovery_flag(&fresh_render_ready, "the worker's fresh Spout render");
    assert_eq!(render_count.load(Ordering::Acquire), 2);
    let captured_fresh_token = fresh_token
        .lock()
        .unwrap()
        .expect("the second worker render must capture a fresh authority token");
    assert_ne!(
        captured_fresh_token, captured_first_token,
        "the worker's second render must reacquire after the blackout mutation"
    );
    assert!(
        frames.lock().unwrap().is_empty(),
        "the stale first authority must be rejected before any Spout sender entry"
    );

    release_fresh_send.store(true, Ordering::Release);
    wait_for_recovery_flag(&admitted_send, "the fresh Spout sender admission");
    wait_for_recovery_flag(&after_send_ready, "the post-send Spout worker callback");
    assert_eq!(render_count.load(Ordering::Acquire), 3);
    assert_eq!(frames.lock().unwrap().len(), 1);
    let transition = engine
        .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
        .expect("the joined worker must retire during an active all-deny transition");
    stop.store(true, Ordering::Release);
    release_after_send.store(true, Ordering::Release);
    let worker_result = guard.join();
    assert!(
        worker_result.is_ok(),
        "worker should stop after fresh send: {worker_result:?}"
    );
    assert_eq!(render_count.load(Ordering::Acquire), 3);
    let recorded = frames.lock().unwrap();
    assert_eq!(
        recorded.len(),
        1,
        "only the fresh authority may reach Spout"
    );
    let (pixels, width, height) = &recorded[0];
    assert_eq!((*width, *height), (2, 1));
    let expected_pixels = RECOVERY_PIXEL.repeat(2);
    assert_eq!(pixels.as_slice(), expected_pixels.as_slice());
    assert!(
        pixels.chunks_exact(4).any(|pixel| pixel != [0, 0, 0, 255]),
        "the fresh Video presentation must send artistic pixels"
    );
    drop(recorded);
    assert!(
        guard.take_failure_lease().is_none(),
        "normal worker retirement must not leave a failure lease"
    );
    let teardown = guard
        .take_teardown_lease()
        .expect("worker join must publish its teardown lease");
    drop(teardown);
    let status = transition.complete().unwrap();
    assert_eq!(status.state, protocol::OutputOwnershipState::Ready);
    assert_eq!(status.effective_role, protocol::MachineOutputRole::Standby);
}

#[test]
fn spout_paused_keepalive_aba_revalidation_is_reacquirable() {
    let output_id = 98_211;
    let engine = recovery_engine(output_id);
    let authority = injected_spout_authority(&engine, output_id);
    let target = protocol::control_plane_command::OutputControlTargetRoleV1::Video;
    engine
        .set_output_blackout_published(target, true, Instant::now() + Duration::from_secs(2))
        .unwrap();
    engine
        .set_output_blackout_published(target, false, Instant::now() + Duration::from_secs(2))
        .unwrap();
    assert!(!engine.snapshot().video.blackout);
    let mut sender = RecordingSpoutSender::default();
    let send_error = send_frame_if_authorized("Spout", &engine, &authority, None, || {
        sender.send_image(&RECOVERY_PIXEL, 2, 1)
    })
    .expect_err("the paused keepalive's pre-ABA authority must be rejected");
    assert!(matches!(send_error, PhysicalOutputSendError::Revoked(_)));
    assert!(
        sender.frames.is_empty(),
        "a stale paused keepalive must not enter the Spout sender"
    );
    let classified = classify_spout_keepalive_send_error(&engine, &authority, send_error);
    assert!(
        matches!(
            &classified,
            OutputPresentationRevalidationError::PresentationChanged(_)
        ),
        "a token-only ABA must be discarded for keepalive reacquisition: {classified:?}"
    );
    let (decision, authority) = spout_render_error_decision(classified, None, output_id)
        .expect("a token-only ABA must retry the no-Follow Spout render");
    assert!(authority.is_none());
    assert!(matches!(
        decision,
        TimelineFollowOutputRenderDecision::Retry { key: None, .. }
    ));
    assert!(spout_render_error_decision(
        OutputPresentationRevalidationError::Other("unrecoverable Spout render error".to_string()),
        None,
        output_id,
    )
    .is_err());
}
