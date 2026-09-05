use super::*;

use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        mpsc, Arc, Condvar, Mutex, RwLock,
    },
    thread,
    time::Duration,
};

fn managed_fail_stop_live_runtime() -> (
    EngineRuntime,
    Arc<EngineSharedTelemetry>,
    io::serial_dmx::OpenDmxTestSerialObservation,
    SafetyBlackoutAuthority,
    OutputOwnershipTeardownLease,
    u64,
) {
    managed_fail_stop_live_runtime_with_before_write(|| {})
}

fn managed_fail_stop_live_runtime_with_before_write(
    before_write_all: impl Fn() + Send + Sync + 'static,
) -> (
    EngineRuntime,
    Arc<EngineSharedTelemetry>,
    io::serial_dmx::OpenDmxTestSerialObservation,
    SafetyBlackoutAuthority,
    OutputOwnershipTeardownLease,
    u64,
) {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) =
        io::serial_dmx::OpenDmxTestSerialPort::new_with_before_write_all(before_write_all);
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("test Open DMX sender must start");
    let mut runtime = EngineRuntime::new_with_shared_telemetry(
        DmxOutputConfig {
            enabled: true,
            ..staged_show_artnet_loopback_output()
        },
        Arc::clone(&shared),
    );
    runtime.dmx_sender = Some(DmxSender::TestExactArtNetRoute);
    runtime.show_serial_dmx_route = Some(RuntimeShowSerialDmxRoute {
        sender: Some(DmxSender::EnttecOpenDmx(sender)),
    });
    // This fixture bypasses the production start path, so publish its live worker truth.
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus {
        active: true,
        worker_shutdown_completed: false,
        ..ShowSerialDmxRouteStatus::default()
    });
    let (_, safety) = shared
        .engage_safety_blackout()
        .expect("test S0 engagement must succeed");
    runtime.safety_blackout_engaged = true;
    let failure_lease = runtime
        .output_ownership_gate
        .begin_failure_fence("managed fail-stop test failure fence");
    let failure_epoch = runtime.output_ownership_gate.status().epoch;
    (
        runtime,
        shared,
        observation,
        safety,
        failure_lease,
        failure_epoch,
    )
}

fn assert_public_indoubt_cleanup_is_all_deny(
    runtime: &mut EngineRuntime,
    shared: &Arc<EngineSharedTelemetry>,
    observation: &io::serial_dmx::OpenDmxTestSerialObservation,
    safety: SafetyBlackoutAuthority,
    require_usb_zero: bool,
) {
    let status = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("post-cleanup serial status must remain queryable");
    assert!(!status.status.active);
    assert!(!status.status.live_frame_queued);
    assert!(status.status.faulted);
    assert!(!status.status.artnet_mirror_live);
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert!(runtime.dmx_sender.is_none());
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(runtime
        .additional_dmx_outputs
        .iter()
        .all(|output| output.sender.is_none()));
    assert!(!runtime
        .dmx_input_frames
        .contains_key(&SHOW_ARTNET_LOOPBACK_UNIVERSE));
    assert!(shared.safety_blackout_authority().engaged);
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("S0 latch remains observable after cleanup"));
    assert!(shared
        .release_safety_blackout(safety.epoch, safety.generation)
        .is_err());

    if require_usb_zero {
        assert!(status.status.zero_frame_physical_write_completed);
        assert!(status.status.worker_shutdown_completed);
    }

    let operations_before_tick = observation
        .operations()
        .expect("test serial observation remains available");
    if require_usb_zero {
        let zero_write = operations_before_tick
            .iter()
            .position(|operation| {
                matches!(
                    operation,
                    io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload)
                        if payload[1..].iter().all(|value| *value == 0)
                )
            })
            .expect("the retained USB worker must receive a physical zero before cleanup");
        assert!(operations_before_tick[zero_write + 1..]
            .iter()
            .any(|operation| matches!(
                operation,
                io::serial_dmx::OpenDmxTestSerialOperation::Flush
            )));
    }
    let snapshot = RwLock::new(runtime.build_snapshot(0));
    runtime.tick(0, &snapshot);
    assert_eq!(
        observation
            .operations()
            .expect("test serial observation remains available after the next tick"),
        operations_before_tick,
        "all-deny cleanup must not retain a USB worker that can retransmit on a later DMX tick"
    );
    assert!(runtime.dmx_sender.is_none());
    assert!(runtime.show_serial_dmx_route.is_none());
}

#[test]
fn managed_fail_stop_sends_one_exact_artnet_zero_before_usb_zero_and_seals_terminal_receipt() {
    let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let expected_packet = build_art_dmx_packet(SHOW_ARTNET_LOOPBACK_UNIVERSE, &[0; 512]);
    let mut send = |packet: &[u8; 530]| {
        assert!(observation
            .operations()
            .expect("test serial observation remains available")
            .iter()
            .all(|operation| !matches!(
                operation,
                io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(_)
            )));
        sends.fetch_add(1, Ordering::SeqCst);
        assert_eq!(packet, &expected_packet);
        Ok(packet.len())
    };

    let receipt = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect("exact route must complete managed fail-stop");

    assert_eq!(sends.load(Ordering::SeqCst), 1);
    assert!(receipt.artnet_zero_accepted());
    assert_eq!(receipt.safety_authority(), safety);
    assert_eq!(receipt.failure_epoch(), failure_epoch);
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert_eq!(runtime.output_ownership_role, MachineOutputRole::Standby);
    let fence = runtime.output_ownership_gate.status();
    assert_eq!(fence.state, OutputOwnershipState::Failed);
    assert_eq!(fence.effective_role, MachineOutputRole::Standby);
    assert_eq!(fence.desired_role, MachineOutputRole::Both);
    assert!(!fence.lighting_allowed);
    assert_eq!(fence.epoch, failure_epoch);
    let serial = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("terminal serial status must be queryable");
    assert!(!serial.status.active);
    assert!(!serial.status.live_frame_queued);
    assert!(serial.status.zero_frame_physical_write_completed);
    assert!(serial.status.worker_shutdown_completed);
    assert!(!serial.status.faulted);
    assert!(!serial.status.artnet_mirror_live);
    assert_eq!(receipt.serial_status_revision(), serial.revision);
    let operations = observation
        .operations()
        .expect("test observations available");
    assert!(operations.iter().any(|operation| matches!(
        operation,
        io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload)
            if payload[1..].iter().all(|value| *value == 0)
    )));
}

#[test]
fn managed_fail_stop_sends_one_exact_zero_for_the_exact_disabled_and_absent_artnet_route() {
    let (mut runtime, _shared, observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime.output.enabled = false;
    runtime.dmx_sender = None;
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let expected_packet = build_art_dmx_packet(SHOW_ARTNET_LOOPBACK_UNIVERSE, &[0; 512]);
    let mut send = |packet: &[u8; 530]| {
        assert_eq!(packet, &expected_packet);
        assert!(observation
            .operations()
            .expect("test serial observation remains available")
            .iter()
            .all(|operation| !matches!(
                operation,
                io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(_)
            )));
        sends.fetch_add(1, Ordering::SeqCst);
        Ok(530)
    };

    let receipt = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect("already-disabled exact route still sends its one-shot zero before USB retirement");
    assert!(receipt.artnet_zero_accepted());
    assert_eq!(sends.load(Ordering::SeqCst), 1);
    assert!(runtime.managed_show_dmx_terminal_absent());
}

#[test]
fn managed_fail_stop_marks_ambiguous_u0_indoubt_only_after_commit() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime.dmx_input_frames.insert(
        SHOW_ARTNET_LOOPBACK_UNIVERSE,
        RuntimeDmxInputFrame {
            values: Box::new([0; 512]),
            merge_mode: DmxMergeMode::Htp,
        },
    );
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let mut send = |_packet: &[u8; 530]| {
        sends.fetch_add(1, Ordering::SeqCst);
        Ok(530)
    };

    let error = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect_err("U0 merge state is unprovable only after the irreversible operation boundary");
    assert!(matches!(error, ManagedShowDmxFailStopError::InDoubt(_)));
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    assert!(runtime.show_serial_dmx_route.is_some());
    assert!(operation.is_committing());
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error("U0 merge test cleanup");
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert!(
        shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("fault status remains queryable")
            .status
            .faulted
    );
}

#[test]
fn managed_fail_stop_marks_wrong_primary_artnet_sender_variant_indoubt_after_commit_and_all_deny() {
    let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    assert!(show_artnet_loopback_route_is_exact_enabled(&runtime.output));
    runtime.dmx_sender = Some(DmxSender::Sacn(
        io::sacn::SacnSender::new(SHOW_ARTNET_LOOPBACK_TARGET_IP, SHOW_ARTNET_LOOPBACK_PORT)
            .expect("test sACN sender must construct without wire publication"),
    ));
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let mut send = |_packet: &[u8; 530]| {
        sends.fetch_add(1, Ordering::SeqCst);
        Ok(530)
    };

    let error = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect_err(
            "a non-Art-Net primary sender must become in-doubt only after the operation commits",
        );
    assert!(matches!(error, ManagedShowDmxFailStopError::InDoubt(_)));
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    assert!(operation.is_committing());
    assert!(runtime.show_serial_dmx_route.is_some());

    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error(
        "wrong primary Art-Net sender variant cleanup",
    );
    assert_public_indoubt_cleanup_is_all_deny(&mut runtime, &shared, &observation, safety, true);
}

#[test]
fn managed_fail_stop_rejects_stale_safety_or_failure_fence_before_artnet_or_usb_commit() {
    for stale_kind in ["safety", "failure"] {
        let (mut runtime, _shared, _observation, safety, _failure_lease, failure_epoch) =
            managed_fail_stop_live_runtime();
        let operation = ManagedShowDmxFailStopOperationInner::new();
        assert!(operation.admit());
        let sends = AtomicUsize::new(0);
        let mut send = |_packet: &[u8; 530]| {
            sends.fetch_add(1, Ordering::SeqCst);
            Ok(530)
        };
        let (expected_safety_epoch, expected_failure_epoch) = match stale_kind {
            "safety" => (safety.epoch.saturating_sub(1), failure_epoch),
            "failure" => (safety.epoch, failure_epoch.saturating_sub(1)),
            _ => unreachable!(),
        };

        let error = runtime
            .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
                expected_safety_epoch,
                safety.generation,
                expected_failure_epoch,
                &operation,
                &mut send,
            )
            .expect_err("stale authority must reject before either physical boundary");
        assert!(matches!(
            error,
            ManagedShowDmxFailStopError::RejectedBeforeCommit(_)
        ));
        assert_eq!(sends.load(Ordering::SeqCst), 0);
        assert!(runtime.show_serial_dmx_route.is_some());
        assert!(!operation.is_committing());
    }
}

#[test]
fn managed_fail_stop_missing_usb_sender_preflight_is_pure_until_postcommit_cleanup() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime
        .show_serial_dmx_route
        .as_mut()
        .expect("test route exists")
        .sender = None;
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus {
        active: true,
        zero_frame_queued: false,
        zero_frame_physical_write_completed: false,
        live_frame_queued: true,
        worker_shutdown_completed: false,
        faulted: false,
        artnet_mirror_live: true,
        artnet_mirror_detail: "stale test record".to_string(),
        detail: "stale Active/non-faulted test record".to_string(),
    });
    let before = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("stale test status remains queryable");
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let mut send = |_packet: &[u8; 530]| {
        sends.fetch_add(1, Ordering::SeqCst);
        Ok(530)
    };

    let error = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect_err("a route slot without a retained USB sender is underdetermined");
    assert!(matches!(error, ManagedShowDmxFailStopError::InDoubt(_)));
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    assert!(operation.is_committing());
    assert_eq!(
        shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("pure topology preflight must not publish a fault"),
        before
    );
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error(
        "missing USB sender test cleanup",
    );
    let status = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("fault status remains queryable");
    assert!(!status.status.active);
    assert!(status.status.faulted);
    assert!(shared
        .release_safety_blackout(safety.epoch, safety.generation)
        .is_err());
}

#[test]
fn managed_fail_stop_missing_usb_route_preflight_is_pure_until_postcommit_cleanup() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime.show_serial_dmx_route = None;
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus {
        active: true,
        zero_frame_queued: false,
        zero_frame_physical_write_completed: false,
        live_frame_queued: true,
        worker_shutdown_completed: false,
        faulted: false,
        artnet_mirror_live: true,
        artnet_mirror_detail: "stale missing-route test record".to_string(),
        detail: "stale Active/non-faulted missing-route record".to_string(),
    });
    let before = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("stale missing-route status remains queryable");
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let sends = AtomicUsize::new(0);
    let mut send = |_packet: &[u8; 530]| {
        sends.fetch_add(1, Ordering::SeqCst);
        Ok(530)
    };

    let error = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect_err(
            "a missing USB route is underdetermined after the physical transaction commits",
        );
    assert!(matches!(error, ManagedShowDmxFailStopError::InDoubt(_)));
    assert_eq!(sends.load(Ordering::SeqCst), 0);
    assert!(operation.is_committing());
    assert_eq!(
        shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("pure topology preflight must not publish a fault"),
        before
    );
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error(
        "missing USB route test cleanup",
    );
    let status = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("fault status remains queryable");
    assert!(!status.status.active);
    assert!(status.status.faulted);
    assert!(shared
        .release_safety_blackout(safety.epoch, safety.generation)
        .is_err());
}

#[test]
fn managed_fail_stop_short_artnet_write_is_indoubt_and_failure_cleanup_never_returns_success() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let mut send = |_packet: &[u8; 530]| Ok(529);

    let error = runtime
        .apply_managed_show_dmx_fail_stop_with_test_artnet_transport(
            safety.epoch,
            safety.generation,
            failure_epoch,
            &operation,
            &mut send,
        )
        .expect_err("a short UDP write is physically ambiguous");
    assert!(matches!(error, ManagedShowDmxFailStopError::InDoubt(_)));
    assert!(operation.is_committing());
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error("short write test cleanup");
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert!(shared.safety_blackout_authority().engaged);
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("S0 latch remains observable"));
}

#[test]
fn managed_fail_stop_operation_timeout_cancels_only_before_commit() {
    let operation = ManagedShowDmxFailStopOperation {
        inner: Arc::new(ManagedShowDmxFailStopOperationInner::new()),
    };
    let result = operation.wait(Duration::ZERO);
    assert!(matches!(
        result,
        Err(ManagedShowDmxFailStopError::CancelledBeforeCommit(_))
    ));
}

#[test]
fn managed_fail_stop_public_priority_queue_honors_precommit_cancellation_without_mutation() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);
    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("pre-commit priority queue admission must be available");
    assert_eq!(handle.safety_queue.len(), 1);
    assert!(matches!(
        operation.wait(Duration::ZERO),
        Err(ManagedShowDmxFailStopError::CancelledBeforeCommit(_))
    ));
    runtime.consume_commands(&handle.safety_queue);
    assert!(runtime.show_serial_dmx_route.is_some());
    assert!(runtime.dmx_sender.is_some());
}

#[test]
fn managed_fail_stop_admit_then_cancel_race_is_pure_even_for_missing_usb_topology() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime
        .show_serial_dmx_route
        .as_mut()
        .expect("test USB route exists")
        .sender = None;
    runtime.dmx_input_frames.insert(
        SHOW_ARTNET_LOOPBACK_UNIVERSE,
        RuntimeDmxInputFrame {
            values: Box::new([0; 512]),
            merge_mode: DmxMergeMode::Htp,
        },
    );
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus {
        active: true,
        zero_frame_queued: false,
        zero_frame_physical_write_completed: false,
        live_frame_queued: true,
        worker_shutdown_completed: false,
        faulted: false,
        artnet_mirror_live: true,
        artnet_mirror_detail: "admit/cancel race fixture".to_string(),
        detail: "unmutated before consumed commit".to_string(),
    });
    let status_before = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("test status remains queryable");
    let safety_before = shared.safety_blackout_authority();
    let packets = Arc::new(Mutex::new(Vec::new()));
    runtime.install_managed_show_dmx_fail_stop_test_transport(
        Arc::clone(&packets),
        Arc::new(Mutex::new(Vec::new())),
    );
    let (admitted_sender, admitted_receiver) = mpsc::sync_channel(1);
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let release_at_barrier = Arc::clone(&release);
    runtime.install_managed_show_dmx_fail_stop_test_after_admit_before_commit(Arc::new(
        move || {
            admitted_sender
                .send(())
                .expect("caller remains present while the post-admit barrier waits");
            let (lock, changed) = &*release_at_barrier;
            let mut released = lock
                .lock()
                .expect("post-admit release lock remains available");
            while !*released {
                released = changed
                    .wait(released)
                    .expect("post-admit release condition remains available");
            }
        },
    ));
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);
    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("priority command must queue before its post-admit cancellation race");
    let observer = ManagedShowDmxFailStopOperation {
        inner: Arc::clone(&operation.inner),
    };
    let safety_queue = Arc::clone(&handle.safety_queue);

    thread::scope(|scope| {
        let worker = scope.spawn(|| runtime.consume_commands(&safety_queue));
        admitted_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("worker must admit before the deterministic cancellation boundary");
        assert!(matches!(
            observer.wait(Duration::ZERO),
            Err(ManagedShowDmxFailStopError::CancelledBeforeCommit(_))
        ));
        let (lock, changed) = &*release;
        *lock
            .lock()
            .expect("post-admit release lock remains available") = true;
        changed.notify_one();
        worker
            .join()
            .expect("cancelled worker must return without topology cleanup");
    });

    assert_eq!(
        shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("cancellation must preserve status"),
        status_before
    );
    assert_eq!(shared.safety_blackout_authority(), safety_before);
    assert!(runtime.dmx_sender.is_some());
    assert!(runtime
        .show_serial_dmx_route
        .as_ref()
        .is_some_and(|route| route.sender.is_none()));
    assert!(runtime
        .dmx_input_frames
        .contains_key(&SHOW_ARTNET_LOOPBACK_UNIVERSE));
    assert!(packets
        .lock()
        .expect("cancelled command must not send")
        .is_empty());
}

#[test]
fn managed_fail_stop_public_priority_queue_rejects_stale_safety_or_failure_epoch_without_send() {
    for stale_kind in ["safety", "failure"] {
        let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
            managed_fail_stop_live_runtime();
        let packets = Arc::new(Mutex::new(Vec::new()));
        runtime.install_managed_show_dmx_fail_stop_test_transport(
            Arc::clone(&packets),
            Arc::new(Mutex::new(Vec::new())),
        );
        let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
        let mut handle = allocator_test_handle(Arc::clone(&snapshot));
        handle.shared_telemetry = Arc::clone(&shared);
        let expected_safety = if stale_kind == "safety" {
            SafetyBlackoutAuthority {
                engaged: true,
                epoch: safety.epoch.saturating_sub(1),
                generation: safety.generation,
            }
        } else {
            safety
        };
        let expected_failure_epoch = if stale_kind == "failure" {
            failure_epoch.saturating_sub(1)
        } else {
            failure_epoch
        };
        let operation = handle
            .begin_retire_managed_show_dmx_after_safety_blackout(
                expected_safety,
                expected_failure_epoch,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("stale values are admitted only as a queued request");

        runtime.consume_commands(&handle.safety_queue);
        assert!(matches!(
            operation.wait(Duration::from_secs(1)),
            Err(ManagedShowDmxFailStopError::RejectedBeforeCommit(_))
        ));
        assert!(packets
            .lock()
            .expect("test packet log remains readable")
            .is_empty());
        assert!(runtime.show_serial_dmx_route.is_some());
    }
}

#[test]
fn managed_fail_stop_public_current_authority_unprovable_topologies_commit_all_deny() {
    for topology in [
        "U0 input/merge",
        "additional DMX output",
        "wrong Art-Net topology",
        "missing USB sender",
        "missing USB route",
    ] {
        let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
            managed_fail_stop_live_runtime();
        match topology {
            "U0 input/merge" => {
                runtime.dmx_input_frames.insert(
                    SHOW_ARTNET_LOOPBACK_UNIVERSE,
                    RuntimeDmxInputFrame {
                        values: Box::new([0; 512]),
                        merge_mode: DmxMergeMode::Htp,
                    },
                );
            }
            "additional DMX output" => {
                let mut config = runtime.output.clone();
                config.universe = 1;
                runtime.additional_dmx_outputs.push(RuntimeDmxOutput {
                    config,
                    sender: Some(DmxSender::TestExactArtNetRoute),
                    recovery: DmxRouteRecovery::default(),
                });
            }
            "wrong Art-Net topology" => {
                runtime.output.target_ip = "127.0.0.2".to_string();
            }
            "missing USB sender" => {
                runtime
                    .show_serial_dmx_route
                    .as_mut()
                    .expect("test USB route exists")
                    .sender = None;
            }
            "missing USB route" => runtime.show_serial_dmx_route = None,
            _ => unreachable!(),
        }

        let packets = Arc::new(Mutex::new(Vec::new()));
        runtime.install_managed_show_dmx_fail_stop_test_transport(
            Arc::clone(&packets),
            Arc::new(Mutex::new(Vec::new())),
        );
        let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
        let mut handle = allocator_test_handle(Arc::clone(&snapshot));
        handle.shared_telemetry = Arc::clone(&shared);
        let operation = handle
            .begin_retire_managed_show_dmx_after_safety_blackout(
                safety,
                failure_epoch,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("exact authority still admits its consumed priority operation");

        runtime.consume_commands(&handle.safety_queue);
        assert!(matches!(
            operation.wait(Duration::from_secs(1)),
            Err(ManagedShowDmxFailStopError::InDoubt(_))
        ));
        assert!(packets
            .lock()
            .expect("test packet log remains readable")
            .is_empty());
        assert_public_indoubt_cleanup_is_all_deny(
            &mut runtime,
            &shared,
            &observation,
            safety,
            !matches!(topology, "missing USB sender" | "missing USB route"),
        );
    }
}

#[test]
fn managed_fail_stop_public_socket_bind_failure_commits_all_deny_cleanup() {
    let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime.install_managed_show_dmx_fail_stop_test_socket_bind_error(
        "injected deterministic local bind failure",
    );
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);
    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("current exact authority must enqueue before its bind boundary");

    runtime.consume_commands(&handle.safety_queue);
    assert!(matches!(
        operation.wait(Duration::from_secs(1)),
        Err(ManagedShowDmxFailStopError::InDoubt(_))
    ));
    assert_public_indoubt_cleanup_is_all_deny(&mut runtime, &shared, &observation, safety, true);
}

#[test]
fn managed_fail_stop_public_priority_queue_success_proves_artnet_usb_shutdown_order_and_replay_rejects(
) {
    let event_log = Arc::new(Mutex::new(Vec::new()));
    let write_event_log = Arc::clone(&event_log);
    let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime_with_before_write(move || {
            let events = write_event_log
                .lock()
                .expect("managed fail-stop event log remains readable at USB write");
            assert_eq!(
                events.as_slice(),
                &[ManagedShowDmxFailStopTestEvent::ArtNetZeroAccepted],
                "the real Open DMX zero write must not begin before local Art-Net zero acceptance"
            );
        });
    let packets = Arc::new(Mutex::new(Vec::new()));
    runtime.install_managed_show_dmx_fail_stop_test_transport(
        Arc::clone(&packets),
        Arc::clone(&event_log),
    );
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);

    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("public priority queue must admit the exact transaction");
    runtime.consume_commands(&handle.safety_queue);
    let receipt = operation
        .wait(Duration::from_secs(1))
        .expect("public worker path must return its one terminal receipt");

    assert!(receipt.artnet_zero_accepted());
    assert_eq!(
        packets
            .lock()
            .expect("test packet log remains readable")
            .as_slice(),
        &[build_art_dmx_packet(
            SHOW_ARTNET_LOOPBACK_UNIVERSE,
            &[0; 512]
        )]
    );
    assert_eq!(
        event_log
            .lock()
            .expect("test event log remains readable")
            .as_slice(),
        &[
            ManagedShowDmxFailStopTestEvent::ArtNetZeroAccepted,
            ManagedShowDmxFailStopTestEvent::UsbZeroPhysicalCompleted,
            ManagedShowDmxFailStopTestEvent::WorkerShutdownCompleted,
        ]
    );
    let operations = observation
        .operations()
        .expect("test serial observation remains available");
    let zero_write = operations
        .iter()
        .position(|operation| {
            matches!(
                operation,
                io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload)
                    if payload[1..].iter().all(|value| *value == 0)
            )
        })
        .expect("the public worker path must physically write USB zero");
    assert!(operations[zero_write + 1..]
        .iter()
        .any(|operation| matches!(operation, io::serial_dmx::OpenDmxTestSerialOperation::Flush)));
    let terminal_status = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("completed terminal status remains queryable");
    let terminal_safety = shared.safety_blackout_authority();
    let terminal_completion = runtime.managed_show_dmx_fail_stop_completion;
    let terminal_events = event_log
        .lock()
        .expect("test event log remains readable")
        .clone();

    // A second queued command carrying the same consumed authority cannot
    // replay a receipt or recreate any sender. Its exact completion evidence
    // distinguishes intentional terminal absence from a missing worker, so
    // it must also leave S0/status/evidence completely untouched.
    let replay = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("queue admission itself remains transport-free");
    runtime.consume_commands(&handle.safety_queue);
    assert!(matches!(
        replay.wait(Duration::from_secs(1)),
        Err(ManagedShowDmxFailStopError::RejectedBeforeCommit(_))
    ));
    assert_eq!(
        packets
            .lock()
            .expect("test packet log remains readable")
            .len(),
        1,
        "replay must not send a second Art-Net zero"
    );
    let after_replay_status = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("replay must preserve the terminal status snapshot");
    assert_eq!(after_replay_status, terminal_status);
    assert!(!after_replay_status.status.faulted);
    assert_eq!(shared.safety_blackout_authority(), terminal_safety);
    assert_eq!(
        runtime.managed_show_dmx_fail_stop_completion,
        terminal_completion
    );
    assert_eq!(
        event_log
            .lock()
            .expect("test event log remains readable")
            .as_slice(),
        terminal_events.as_slice(),
    );
    assert!(runtime.managed_show_dmx_terminal_absent());
}

#[test]
fn managed_fail_stop_completion_evidence_treats_aba_as_postcommit_indoubt_and_faults() {
    for changed_boundary in ["serial status", "DMX route generation"] {
        let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
            managed_fail_stop_live_runtime();
        let packets = Arc::new(Mutex::new(Vec::new()));
        runtime.install_managed_show_dmx_fail_stop_test_transport(
            Arc::clone(&packets),
            Arc::new(Mutex::new(Vec::new())),
        );
        let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
        let mut handle = allocator_test_handle(Arc::clone(&snapshot));
        handle.shared_telemetry = Arc::clone(&shared);
        let initial = handle
            .begin_retire_managed_show_dmx_after_safety_blackout(
                safety,
                failure_epoch,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("initial exact transaction must enqueue");
        runtime.consume_commands(&handle.safety_queue);
        initial
            .wait(Duration::from_secs(1))
            .expect("initial transaction must complete before ABA mutation");
        match changed_boundary {
            "serial status" => {
                let mut changed = shared
                    .try_show_serial_dmx_route_status_snapshot()
                    .expect("terminal status remains queryable")
                    .status;
                changed
                    .detail
                    .push_str(" external terminal-status mutation");
                shared.set_show_serial_dmx_route_status(changed);
            }
            "DMX route generation" => runtime.bump_dmx_route_configuration_generation(),
            _ => unreachable!(),
        }

        let replay = handle
            .begin_retire_managed_show_dmx_after_safety_blackout(
                safety,
                failure_epoch,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("stale completion evidence is still a queued request");
        runtime.consume_commands(&handle.safety_queue);
        assert!(matches!(
            replay.wait(Duration::from_secs(1)),
            Err(ManagedShowDmxFailStopError::InDoubt(_))
        ));
        assert_eq!(
            packets
                .lock()
                .expect("test packet log remains readable")
                .len(),
            1,
            "{changed_boundary} must block a second Art-Net send"
        );
        let faulted = shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("fault status remains queryable");
        assert!(
            faulted.status.faulted,
            "{changed_boundary} must not launder ABA"
        );
        assert!(runtime.managed_show_dmx_fail_stop_completion.is_none());
        assert!(shared
            .release_safety_blackout(safety.epoch, safety.generation)
            .is_err());
    }
}

#[test]
fn managed_fail_stop_public_artnet_error_or_short_write_runs_usb_fault_cleanup_and_republishes_sender_absence(
) {
    for (case, planned_result) in [
        (
            "send error",
            Err("injected local UDP send error".to_string()),
        ),
        ("short write", Ok(529)),
    ] {
        let (mut runtime, shared, observation, safety, _failure_lease, failure_epoch) =
            managed_fail_stop_live_runtime();
        let packets = Arc::new(Mutex::new(Vec::new()));
        runtime.install_managed_show_dmx_fail_stop_test_transport_result(
            Arc::clone(&packets),
            planned_result,
        );
        let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
        let mut handle = allocator_test_handle(Arc::clone(&snapshot));
        handle.shared_telemetry = Arc::clone(&shared);
        let operation = handle
            .begin_retire_managed_show_dmx_after_safety_blackout(
                safety,
                failure_epoch,
                Instant::now() + Duration::from_secs(1),
            )
            .expect("public priority queue must admit the physical-boundary test");

        runtime.consume_commands(&handle.safety_queue);
        assert!(matches!(
            operation.wait(Duration::from_secs(1)),
            Err(ManagedShowDmxFailStopError::InDoubt(_))
        ));
        assert_eq!(
            packets
                .lock()
                .expect("test packet log remains readable")
                .as_slice(),
            &[build_art_dmx_packet(
                SHOW_ARTNET_LOOPBACK_UNIVERSE,
                &[0; 512]
            )],
            "{case} must still have reached exactly one attempted local zero send"
        );
        let operations = observation
            .operations()
            .expect("test serial observation remains available");
        let zero_write = operations
            .iter()
            .position(|operation| {
                matches!(
                    operation,
                    io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload)
                        if payload[1..].iter().all(|value| *value == 0)
                )
            })
            .expect("{case} cleanup must attempt its actual USB zero write");
        assert!(
            operations[zero_write + 1..]
                .iter()
                .any(|operation| matches!(
                    operation,
                    io::serial_dmx::OpenDmxTestSerialOperation::Flush
                )),
            "{case} cleanup must flush the USB zero transaction before bounded shutdown"
        );
        let status = shared
            .try_show_serial_dmx_route_status_snapshot()
            .expect("post-cleanup USB status must remain queryable");
        assert!(
            !status.status.active,
            "{case} cleanup must not retain USB active"
        );
        assert!(!status.status.live_frame_queued);
        assert!(status.status.zero_frame_physical_write_completed);
        assert!(status.status.worker_shutdown_completed);
        assert!(status.status.faulted);
        assert!(!status.status.artnet_mirror_live);
        assert!(status.status.artnet_mirror_detail.contains("absent"));
        assert!(runtime.managed_show_dmx_fail_stop_completion.is_none());
        assert!(runtime.managed_show_dmx_terminal_absent());
        assert!(shared.safety_blackout_authority().engaged);
        assert!(shared
            .release_safety_blackout(safety.epoch, safety.generation)
            .is_err());
    }
}

#[test]
fn managed_fail_stop_public_postcommit_wait_is_indoubt_then_worker_completes_without_resend() {
    let packets = Arc::new(Mutex::new(Vec::new()));
    let (entered_sender, entered_receiver) = mpsc::sync_channel(1);
    let release = Arc::new((Mutex::new(false), Condvar::new()));
    let release_at_transport = Arc::clone(&release);
    let expected_packet = build_art_dmx_packet(SHOW_ARTNET_LOOPBACK_UNIVERSE, &[0; 512]);
    let barrier = Arc::new(move |packet: &[u8; 530]| {
        assert_eq!(packet, &expected_packet);
        entered_sender
            .send(())
            .expect("test caller remains present while the transport is blocked");
        let (lock, changed) = &*release_at_transport;
        let mut released = lock
            .lock()
            .expect("test transport release lock remains available");
        while !*released {
            released = changed
                .wait(released)
                .expect("test transport release condition remains available");
        }
    });
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    runtime
        .install_managed_show_dmx_fail_stop_test_transport_barrier(Arc::clone(&packets), barrier);
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);
    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("public priority queue must admit the post-commit timeout proof");
    let observer = ManagedShowDmxFailStopOperation {
        inner: Arc::clone(&operation.inner),
    };
    let safety_queue = Arc::clone(&handle.safety_queue);

    thread::scope(|scope| {
        let worker = scope.spawn(|| runtime.consume_commands(&safety_queue));
        entered_receiver
            .recv_timeout(Duration::from_secs(1))
            .expect("public queue handler must reach its post-commit transport barrier");
        assert!(matches!(
            observer.wait(Duration::ZERO),
            Err(ManagedShowDmxFailStopError::InDoubt(_))
        ));
        assert!(operation.inner.is_committing());
        let (lock, changed) = &*release;
        *lock
            .lock()
            .expect("test transport release lock remains available") = true;
        changed.notify_one();
        worker
            .join()
            .expect("public priority worker must complete after transport release");
    });

    let receipt = operation
        .wait(Duration::from_secs(1))
        .expect("the original caller receives the one eventual terminal receipt");
    assert!(receipt.artnet_zero_accepted());
    assert_eq!(
        packets
            .lock()
            .expect("test packet log remains readable")
            .as_slice(),
        &[build_art_dmx_packet(
            SHOW_ARTNET_LOOPBACK_UNIVERSE,
            &[0; 512]
        )]
    );
    assert!(runtime.managed_show_dmx_terminal_absent());
}

#[test]
fn managed_fail_stop_safety_queue_reaches_artnet_before_a_concurrently_enqueued_normal_marker() {
    let (mut runtime, shared, _observation, safety, _failure_lease, failure_epoch) =
        managed_fail_stop_live_runtime();
    let packets = Arc::new(Mutex::new(Vec::new()));
    let (marker_sender, marker_receiver) = mpsc::sync_channel(1);
    let marker_receiver = Arc::new(Mutex::new(marker_receiver));
    let marker_at_transport = Arc::clone(&marker_receiver);
    let transport_observer = Arc::new(move |packet: &[u8; 530]| {
        assert_eq!(
            packet,
            &build_art_dmx_packet(SHOW_ARTNET_LOOPBACK_UNIVERSE, &[0; 512])
        );
        assert!(matches!(
            marker_at_transport
                .lock()
                .expect("normal marker receiver remains available")
                .try_recv(),
            Err(mpsc::TryRecvError::Empty)
        ));
    });
    runtime.install_managed_show_dmx_fail_stop_test_transport_barrier(
        Arc::clone(&packets),
        transport_observer,
    );
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let mut handle = allocator_test_handle(Arc::clone(&snapshot));
    handle.shared_telemetry = Arc::clone(&shared);
    // Queue a benign normal marker first, then the safety operation. The
    // engine run loop drains exactly these two queues in safety-first order.
    handle
        .send(EngineCommand::RequestPersistenceSnapshot {
            response: marker_sender,
        })
        .expect("normal marker must enqueue");
    let operation = handle
        .begin_retire_managed_show_dmx_after_safety_blackout(
            safety,
            failure_epoch,
            Instant::now() + Duration::from_secs(1),
        )
        .expect("priority fail-stop must enqueue after the normal marker");

    runtime.consume_commands(&handle.safety_queue);
    assert!(operation
        .wait(Duration::from_secs(1))
        .expect("safety command completes before normal drain")
        .artnet_zero_accepted());
    assert_eq!(
        packets
            .lock()
            .expect("test packet log remains readable")
            .len(),
        1
    );
    runtime.consume_normal_commands(&handle.queue);
    marker_receiver
        .lock()
        .expect("normal marker receiver remains available")
        .recv_timeout(Duration::from_secs(1))
        .expect("normal marker runs only after the safety drain");
}

#[test]
fn managed_fail_stop_wait_after_commit_is_indoubt_and_never_cancels_the_worker() {
    let inner = Arc::new(ManagedShowDmxFailStopOperationInner::new());
    assert!(inner.admit());
    assert!(inner.begin_commit());
    let operation = ManagedShowDmxFailStopOperation {
        inner: Arc::clone(&inner),
    };
    assert!(matches!(
        operation.wait(Duration::ZERO),
        Err(ManagedShowDmxFailStopError::InDoubt(_))
    ));
    assert!(inner.is_committing());
    inner.finish(Err(ManagedShowDmxFailStopError::in_doubt(
        "test worker terminal result",
    )));
}

#[test]
fn managed_fail_stop_without_usb_handles_artnet_and_no_dmx_without_fabricated_usb_proof() {
    for artnet in [false, true] {
        let shared = Arc::new(EngineSharedTelemetry::new());
        let mut runtime = EngineRuntime::new_with_shared_telemetry(
            staged_show_artnet_loopback_output(),
            shared.clone(),
        );
        if artnet {
            runtime.output.enabled = true;
            runtime.dmx_sender = Some(DmxSender::TestExactArtNetRoute);
        }
        let (_, safety) = shared.engage_safety_blackout().unwrap();
        runtime.safety_blackout_engaged = true;
        let _lease = runtime
            .output_ownership_gate
            .begin_failure_fence("no USB test");
        let epoch = runtime.output_ownership_gate.status().epoch;
        let operation = ManagedShowDmxFailStopOperationInner::new();
        assert!(operation.admit());
        let mut sends = 0;
        let mut send = |packet: &[u8; 530]| {
            sends += 1;
            Ok(packet.len())
        };
        let receipt = runtime
            .apply_managed_show_dmx_fail_stop_with_transport(
                safety.epoch,
                safety.generation,
                epoch,
                &operation,
                &mut send,
            )
            .unwrap();
        assert_eq!(receipt.artnet_zero_accepted(), artnet);
        assert_eq!(sends, usize::from(artnet));
        assert!(runtime.managed_show_dmx_terminal_absent());
        let status = shared
            .try_show_serial_dmx_route_status_snapshot()
            .unwrap()
            .status;
        assert!(!status.zero_frame_physical_write_completed);
        assert!(!status.zero_frame_queued);
        assert!(!status.faulted);
        let replay = ManagedShowDmxFailStopOperationInner::new();
        assert!(replay.admit());
        assert!(runtime
            .apply_managed_show_dmx_fail_stop_with_transport(
                safety.epoch,
                safety.generation,
                epoch,
                &replay,
                &mut |_| panic!("replay must not send")
            )
            .is_err());
    }
}

#[test]
fn managed_fail_stop_missing_usb_rejects_prior_worker_history_and_uncertain_status() {
    for variant in 0..6 {
        let shared = Arc::new(EngineSharedTelemetry::new());
        let mut runtime = EngineRuntime::new_with_shared_telemetry(
            staged_show_artnet_loopback_output(),
            shared.clone(),
        );
        let (_, _) = shared.engage_safety_blackout().unwrap();
        let mut status = ShowSerialDmxRouteStatus::default();
        match variant {
            0 => status.active = true,
            1 => status.live_frame_queued = true,
            2 => status.zero_frame_queued = true,
            3 => status.zero_frame_physical_write_completed = true,
            4 => status.worker_shutdown_completed = false,
            _ => status.faulted = true,
        }
        shared.set_show_serial_dmx_route_status(status);
        assert!(runtime
            .managed_show_dmx_fail_stop_topology_preflight()
            .is_err());
        runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error("missing worker");
        assert!(shared.show_serial_dmx_route_status().faulted);
    }
}

#[test]
fn managed_fail_stop_no_usb_cleanup_does_not_invent_usb_fault() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = EngineRuntime::new_with_shared_telemetry(
        staged_show_artnet_loopback_output(),
        shared.clone(),
    );
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error("Art-Net failure");
    assert!(runtime.managed_show_dmx_terminal_absent());
    let status = shared.show_serial_dmx_route_status();
    assert!(!status.faulted);
    assert!(!status.zero_frame_physical_write_completed);
}
