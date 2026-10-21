// All tests use fake native owners and never load the macOS Syphon framework.
#[test]
fn syphon_blocked_retirement_returns_bounded_and_retains_owner_until_join() {
    struct Release(Option<mpsc::Sender<()>>);
    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() { let _ = tx.send(()); }
        }
    }
    let engine = test_engine(980_521);
    let mut transport = test_transport();
    let (release, wait) = mpsc::channel();
    let (entered, ready) = mpsc::channel();
    let closed = Arc::new(AtomicBool::new(false));
    let worker_closed = Arc::clone(&closed);
    let worker = std::thread::spawn(move || {
        entered.send(()).unwrap();
        wait.recv_timeout(Duration::from_secs(5)).unwrap();
        worker_closed.store(true, Ordering::Release);
        Ok(())
    });
    transport.output_workers.insert(980_521, SyphonRouteWorker {
        endpoint_name: "Retained Syphon".to_string(),
        lifecycle: SyphonOutputLifecycle::new(),
        stop: Arc::new(AtomicBool::new(false)),
        worker: Some(worker),
        failure: Arc::new(Mutex::new(None)),
        teardown_lease: Arc::new(Mutex::new(None)),
        start_signal: None,
        creation_lease: None,
    });
    let mut release = Release(Some(release));
    ready.recv_timeout(Duration::from_secs(1)).unwrap();
    let reservation = engine.reserve_output_ownership_retirement(
        protocol::MachineOutputRole::Standby, &transport.output_stop_signals(),
    ).unwrap();
    let started = Instant::now();
    let error = transport.retire_output_worker(
        980_521, &engine, Duration::from_millis(15),
    ).expect_err("blocked owner must remain registered after a bounded failure");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.message.contains("remain pending"));
    reservation.fail(error.message);
    assert!(!closed.load(Ordering::Acquire));
    assert!(transport.output_workers.contains_key(&980_521));
    assert!(transport.pending_output_retirement_leases.contains_key(&980_521));
    assert!(engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both).is_err());
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(transport.output_workers.contains_key(&980_521));
    release.0.take().unwrap().send(()).unwrap();
    assert!(wait_for_output_worker(Duration::from_secs(1), || {
        transport.output_workers[&980_521].has_finished()
    }));
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(closed.load(Ordering::Acquire));
    assert!(transport.output_workers.is_empty());
    assert!(transport.pending_output_retirement_leases.is_empty());
    engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both)
        .unwrap().complete().unwrap();
    assert!(engine.acquire_video_output().is_ok());
}

#[test]
fn syphon_stop_during_render_or_admission_emits_no_send_or_follow_fault() {
    for mode in 0..3 {
        let output_id = 980_522 + mode;
        let engine = test_engine(output_id);
        let authority = test_authority(&engine, output_id);
        let reservation = engine.reserve_output_ownership_retirement(
            protocol::MachineOutputRole::Standby, &[],
        ).unwrap();
        let before = engine.output_ownership_status();
        let stop = Arc::new(AtomicBool::new(false));
        let render_stop = Arc::clone(&stop);
        let sends = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let teardown = Arc::new(Mutex::new(None));
        if mode == 2 {
            external_output_retirement::stop_after_denied_admission_for_test(None);
        }
        let result = run_syphon_output_worker(
            output_id, "Stop regression Syphon", engine.clone(), stop,
            FakeSender::new(Arc::clone(&sends), Arc::clone(&closed)),
            (Arc::clone(&teardown), Arc::new(Mutex::new(None))),
            move |_| {
                if mode != 2 { render_stop.store(true, Ordering::Release); }
                if mode == 0 { return Err("expected cancelled render".to_string()); }
                Ok((TimelineFollowOutputRenderDecision::Frame {
                    key: TimelineFollowOutputFrameKey {
                        epoch: authority.ownership_epoch(), generation: 77, output_id,
                        width: 1, height: 1,
                    },
                    frame: video::VideoFrame {
                        layer_id: 0, width: 1, height: 1, pts_ms: 0, duration_ms: 0,
                        format: video::VideoPixelFormat::Rgba8, data: vec![0, 0, 0, 255],
                    },
                    result: TimelineFollowSettlementAckResult::Applied,
                    follow_identity: Some(TimelineFollowActiveIdentity {
                        epoch: authority.ownership_epoch(), generation: 77,
                        source_timeline_id: protocol::TimelineId(71),
                        target_timeline_id: protocol::TimelineId(72),
                    }),
                }, Some(authority.clone())))
            },
        );
        assert!(result.is_ok(), "planned stop is not a worker fault: {result:?}");
        assert_eq!(sends.load(Ordering::Acquire), 0);
        assert!(closed.load(Ordering::Acquire));
        assert_eq!(engine.output_ownership_status(), before);
        drop(teardown.lock().unwrap().take());
        reservation.finish_retirement().unwrap().complete().unwrap();
    }
}
