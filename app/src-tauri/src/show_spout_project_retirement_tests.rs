// Included inside spout_transport::tests to exercise the real worker and join
// path with only the SDK sender replaced. No physical output device is opened.

fn project_retirement_test_worker(
    engine: &EngineHandle,
    output_id: u64,
    send_error: Option<String>,
) -> (SpoutRouteWorker, mpsc::Sender<()>, Arc<AtomicBool>) {
    let authority = injected_spout_authority(engine, output_id);
    let stop = Arc::new(AtomicBool::new(false));
    let teardown_lease = Arc::new(Mutex::new(None));
    let failure = Arc::new(Mutex::new(None));
    let dropped = Arc::new(AtomicBool::new(false));
    let (start_tx, start_rx) = mpsc::channel();
    let worker_engine = engine.clone();
    let worker_stop = Arc::clone(&stop);
    let worker_lease = Arc::clone(&teardown_lease);
    let worker_failure = Arc::clone(&failure);
    let worker_dropped = Arc::clone(&dropped);
    let worker = std::thread::spawn(move || {
        start_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        let result = run_spout_output_worker(
            output_id,
            "Project retirement test Spout",
            worker_engine,
            worker_stop,
            InjectedSpoutSender {
                send_error,
                drop_started: Arc::new(AtomicBool::new(false)),
                allow_drop: Arc::new(AtomicBool::new(true)),
                dropped: worker_dropped,
            },
            (worker_lease, Arc::new(Mutex::new(None))),
            None,
            move |_| Ok((
                injected_spout_render_decision(output_id, authority.ownership_epoch()),
                Some(authority.clone()),
            )),
        );
        if let Err(error) = &result {
            record_spout_worker_failure(&worker_failure, error.clone());
        }
        result
    });
    (
        SpoutRouteWorker {
            endpoint_name: Some(injected_spout_endpoint(output_id)),
            stop,
            worker: Some(worker),
            failure,
            teardown_lease,
            start_signal: None,
            creation_lease: None,
        },
        start_tx,
        dropped,
    )
}

#[test]
fn project_retirement_spout_pair_joins_before_quiescence_and_preserves_outputs() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    install_injected_spout_output(&engine, 98_401);
    install_injected_spout_output(&engine, 98_402);
    let before = engine.snapshot().video.outputs;
    let (foreground, start_foreground, foreground_dropped) =
        project_retirement_test_worker(&engine, 98_401, None);
    let (background, start_background, background_dropped) =
        project_retirement_test_worker(&engine, 98_402, None);
    // A render admitted before ProjectOpen must not cause the reservation to
    // wait for teardown leases that can only be released by the later join.
    let old_frame = engine.acquire_video_output().unwrap();
    let reservation = engine.reserve_output_ownership_retirement(
        protocol::MachineOutputRole::Both,
        &[foreground.stop_signal(), background.stop_signal()],
    ).unwrap();
    assert_eq!(engine.output_ownership_status().state, OutputOwnershipState::Transitioning);
    assert!(foreground.stop.load(Ordering::Acquire));
    assert!(background.stop.load(Ordering::Acquire));
    assert!(engine.acquire_video_output().is_err());
    engine.fence_output_ownership().unwrap();
    start_foreground.send(()).unwrap();
    start_background.send(()).unwrap();
    foreground.stop().unwrap();
    background.stop().unwrap();
    assert!(foreground_dropped.load(Ordering::Acquire));
    assert!(background_dropped.load(Ordering::Acquire));
    drop(old_frame);
    let status = reservation.finish_retirement().unwrap()
        .complete_project_swap_disarmed().unwrap();
    assert_eq!(status.state, OutputOwnershipState::Ready);
    assert_eq!(status.effective_role, protocol::MachineOutputRole::Standby);
    assert_eq!(engine.snapshot().video.outputs, before);
}

#[test]
fn project_retirement_spout_unreserved_stop_is_not_relabelled_success() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    install_injected_spout_output(&engine, 98_403);
    let (worker, start, dropped) = project_retirement_test_worker(&engine, 98_403, None);
    // Reproduce the original unsafe order deterministically: stop is visible
    // while the ownership gate is still Ready. The fix must reserve first,
    // rather than masking this real teardown-admission failure.
    worker.request_stop();
    start.send(()).unwrap();
    assert!(worker.stop().is_err());
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(engine.output_ownership_status().state, OutputOwnershipState::Failed);
}

#[test]
fn project_retirement_spout_real_send_failure_survives_stop_and_join() {
    let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    install_injected_spout_output(&engine, 98_404);
    let (worker, start, dropped) = project_retirement_test_worker(
        &engine, 98_404, Some("injected genuine SDK send failure".to_string()),
    );
    start.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    while !worker.worker.as_ref().unwrap().is_finished() {
        assert!(Instant::now() < deadline, "SDK failure worker did not finish");
        std::thread::sleep(Duration::from_millis(1));
    }
    let error = worker.stop().expect_err("intentional stop cannot erase an earlier SDK failure");
    assert!(error.message.contains("injected genuine SDK send failure"), "{error}");
    assert!(dropped.load(Ordering::Acquire));
    assert_eq!(engine.output_ownership_status().state, OutputOwnershipState::Failed);
    assert_eq!(engine.snapshot().video.outputs.len(), 1);
}
