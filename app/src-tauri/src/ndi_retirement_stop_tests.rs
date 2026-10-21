// Uses a blocked Rust owner only; does not construct or discover an NDI sender.
#[test]
fn ndi_blocked_retirement_returns_bounded_and_retains_owner_until_join() {
    struct Release(Option<mpsc::Sender<()>>);
    impl Drop for Release {
        fn drop(&mut self) {
            if let Some(tx) = self.0.take() { let _ = tx.send(()); }
        }
    }
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    let mut transport = NdiTransportState::new(
        Arc::new(Mutex::new(HashMap::new())),
        Arc::new(Mutex::new(HashMap::new())),
    );
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
    transport.outputs.insert(980_511, NdiOutputWorker {
        authoring_teardown: Arc::new(Mutex::new(None)),
        stop: Arc::new(AtomicBool::new(false)),
        worker: Some(worker),
        failure: Arc::new(Mutex::new(None)),
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
        980_511, &engine, Duration::from_millis(15),
    ).expect_err("blocked owner must remain registered after a bounded failure");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.message.contains("remain pending"));
    reservation.fail(error.message);
    assert!(!closed.load(Ordering::Acquire));
    assert!(transport.outputs.contains_key(&980_511));
    assert!(transport.pending_output_retirement_leases.contains_key(&980_511));
    assert!(engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both).is_err());
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(transport.outputs.contains_key(&980_511));
    release.0.take().unwrap().send(()).unwrap();
    assert!(wait_for_output_worker(Duration::from_secs(1), || {
        transport.outputs[&980_511].has_finished()
    }));
    transport.harvest_failed_workers(&engine).unwrap();
    assert!(closed.load(Ordering::Acquire));
    assert!(transport.outputs.is_empty());
    assert!(transport.pending_output_retirement_leases.is_empty());
    engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both)
        .unwrap().complete().unwrap();
    assert!(engine.acquire_video_output().is_ok());
}
