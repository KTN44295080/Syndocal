// Deterministic worker-retirement tests; no SDK or external device is opened.
struct RetirementRelease(Option<mpsc::Sender<()>>);
impl Drop for RetirementRelease {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _ = release.send(());
        }
    }
}

#[test]
fn spout_blocked_retirement_returns_bounded_and_retains_owner_until_join() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let mut transport = SpoutTransportState::new(
        Arc::new(Mutex::new(HashMap::new())),
        #[cfg(feature = "ndi")]
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    );
    let (release, wait) = mpsc::channel();
    let (entered, ready) = mpsc::channel();
    let closed = Arc::new(AtomicBool::new(false));
    let worker_closed = Arc::clone(&closed);
    let handle = std::thread::spawn(move || {
        entered.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        worker_closed.store(true, Ordering::Release);
        Ok(())
    });
    transport.output_workers.insert(980_501, SpoutRouteWorker {
        endpoint_name: Some("Retained Spout".to_string()),
        stop: Arc::new(AtomicBool::new(false)),
        worker: Some(handle),
        failure: Arc::new(Mutex::new(None)),
        teardown_lease: Arc::new(Mutex::new(None)),
        start_signal: None,
        creation_lease: None,
    });
    let mut release = RetirementRelease(Some(release));
    ready.recv_timeout(Duration::from_secs(1)).unwrap();
    let reservation = engine.reserve_output_ownership_retirement(
        protocol::MachineOutputRole::Standby, &transport.output_stop_signals(),
    ).unwrap();
    let started = Instant::now();
    let error = transport.retire_output_worker(
        980_501, &engine, Duration::from_millis(15),
    ).expect_err("blocked worker must return a pending retirement failure");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.message.contains("remain pending"));
    assert!(transport.output_workers.contains_key(&980_501));
    assert!(transport.pending_output_retirement_leases.contains_key(&980_501));
    assert!(!closed.load(Ordering::Acquire));
    reservation.fail(error.message);
    assert!(engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both).is_err());
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(transport.output_workers.contains_key(&980_501));
    assert!(transport.has_output_worker_named("Retained Spout"));
    release.0.take().unwrap().send(()).unwrap();
    assert!(wait_for_output_worker(Duration::from_secs(1), || {
        transport.output_workers[&980_501].has_finished()
    }));
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(closed.load(Ordering::Acquire));
    assert!(transport.output_workers.is_empty());
    assert!(transport.pending_output_retirement_leases.is_empty());
    engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both)
        .unwrap().complete().unwrap();
    assert!(engine.acquire_video_output().is_ok());
}

struct StopCountingSpoutSender {
    sends: Arc<AtomicUsize>,
    dropped: Arc<AtomicBool>,
}
impl SpoutOutputSender for StopCountingSpoutSender {
    fn sender_name(&self) -> String { "Stop regression Spout".to_string() }
    fn send_image(&mut self, _: &[u8], _: u32, _: u32) -> Result<(), String> {
        self.sends.fetch_add(1, Ordering::AcqRel);
        Ok(())
    }
}
impl Drop for StopCountingSpoutSender {
    fn drop(&mut self) { self.dropped.store(true, Ordering::Release); }
}

#[test]
fn spout_stop_during_render_or_admission_emits_no_send_or_follow_fault() {
    for mode in 0..3 {
        let output_id = 980_502 + mode;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        let reservation = engine.reserve_output_ownership_retirement(
            protocol::MachineOutputRole::Standby, &[],
        ).unwrap();
        let before = engine.output_ownership_status();
        let stop = Arc::new(AtomicBool::new(false));
        let render_stop = Arc::clone(&stop);
        let sends = Arc::new(AtomicUsize::new(0));
        let dropped = Arc::new(AtomicBool::new(false));
        let teardown = Arc::new(Mutex::new(None));
        if mode == 2 {
            external_output_retirement::stop_after_denied_admission_for_test(None);
        }
        let result = run_spout_output_worker(
            output_id, "Stop regression Spout", engine.clone(), stop,
            StopCountingSpoutSender {
                sends: Arc::clone(&sends),
                dropped: Arc::clone(&dropped),
            },
            (Arc::clone(&teardown), Arc::new(Mutex::new(None))),
            None,
            move |_| {
                if mode != 2 { render_stop.store(true, Ordering::Release); }
                if mode == 0 { return Err("expected cancelled render".to_string()); }
                let mut frame = injected_spout_render_decision(output_id, authority.ownership_epoch());
                if let TimelineFollowOutputRenderDecision::Frame { follow_identity, .. } = &mut frame {
                    *follow_identity = Some(TimelineFollowActiveIdentity {
                        epoch: authority.ownership_epoch(),
                        generation: 77,
                        source_timeline_id: protocol::TimelineId(71),
                        target_timeline_id: protocol::TimelineId(72),
                    });
                }
                Ok((frame, Some(authority.clone())))
            },
        );
        assert!(result.is_ok(), "planned stop is not a worker fault: {result:?}");
        assert_eq!(sends.load(Ordering::Acquire), 0);
        assert!(dropped.load(Ordering::Acquire));
        assert_eq!(engine.output_ownership_status(), before,
            "stop cannot create a failure epoch or fabricate a Follow deadline");
        drop(teardown.lock().unwrap().take());
        reservation.finish_retirement().unwrap().complete().unwrap();
    }
}

#[test]
fn spout_earlier_render_failure_lease_survives_concurrent_stop() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let stop = Arc::new(AtomicBool::new(false));
    let render_stop = Arc::clone(&stop);
    let failure_slot = Arc::new(Mutex::new(None));
    let render_slot = Arc::clone(&failure_slot);
    let render_engine = engine.clone();
    let teardown = Arc::new(Mutex::new(None));
    let sends = Arc::new(AtomicUsize::new(0));
    let result = run_spout_output_worker(
        980_505, "Prior failure Spout", engine.clone(), stop,
        StopCountingSpoutSender {
            sends: Arc::clone(&sends),
            dropped: Arc::new(AtomicBool::new(false)),
        },
        (Arc::clone(&teardown), failure_slot), None,
        move |_| {
            *render_slot.lock().unwrap() = Some(
                render_engine.begin_output_ownership_failure_fence("prior genuine render failure"),
            );
            render_stop.store(true, Ordering::Release);
            Err("prior genuine render failure".to_string())
        },
    ).expect_err("a pre-existing failure must remain visible through intentional stop");
    assert!(result.message.contains("prior genuine render failure"));
    assert_eq!(sends.load(Ordering::Acquire), 0);
    assert!(teardown.lock().unwrap().is_some());
    assert!(engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both).is_err());
    drop(teardown.lock().unwrap().take());
    engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both)
        .unwrap().complete().unwrap();
}
