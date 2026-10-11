use super::*;

fn ready_gate() -> OutputOwnershipGate {
    let gate = OutputOwnershipGate::startup_denied();
    gate.publish_runtime_role(MachineOutputRole::Both);
    gate
}

#[test]
fn retirement_reservation_signals_under_fence_and_does_not_wait_for_worker_join() {
    let gate = ready_gate();
    assert!(gate.acquire_teardown_lease().is_err());
    let permit = gate.acquire(OutputCapability::Video).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let worker_gate = gate.clone();
    let worker_stop = stop.clone();
    let worker = std::thread::spawn(move || {
        while !worker_stop.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        assert_eq!(
            worker_gate.status().state,
            OutputOwnershipState::Transitioning
        );
        worker_gate.acquire_teardown_lease().unwrap()
    });
    let retirement = gate
        .reserve_retirement(MachineOutputRole::Both, &[stop])
        .unwrap();
    drop(permit);
    let teardown = worker.join().unwrap();
    assert_eq!(gate.inner.state.lock().unwrap().in_flight, 1);
    drop(teardown);
    retirement
        .finish_retirement()
        .unwrap()
        .complete_project_swap_disarmed()
        .unwrap();
    assert_eq!(gate.status().state, OutputOwnershipState::Ready);
    assert!(!gate.status().video_allowed);
}

#[test]
fn retirement_requires_no_remaining_lease_and_rejects_superseding_fault() {
    for inject_fault in [false, true] {
        let gate = ready_gate();
        let retirement = gate
            .reserve_retirement(MachineOutputRole::Both, &[])
            .unwrap();
        let lease = if inject_fault {
            gate.begin_failure_fence("real worker failure")
        } else {
            gate.acquire_teardown_lease().unwrap()
        };
        assert!(output_ownership_retirement::finish_retirement_with_timeout(
            retirement,
            Duration::ZERO,
        )
        .is_err());
        drop(lease);
        assert_eq!(gate.status().state, OutputOwnershipState::Failed);
    }
}

fn wait_for_retirement_condvar<T>(gate: &OutputOwnershipGate, worker: &thread::JoinHandle<T>) {
    let deadline = Instant::now() + Duration::from_secs(2);
    while !gate.inner.retirement_waiting.load(Ordering::Acquire) {
        assert!(
            !worker.is_finished(),
            "retirement returned before its admitted frame drained"
        );
        assert!(
            Instant::now() < deadline,
            "retirement never entered its condvar wait"
        );
        thread::yield_now();
    }
}

#[test]
fn retirement_joins_an_admitted_frame_before_project_publication() {
    let gate = ready_gate();
    let permit = gate.acquire(OutputCapability::Lighting).unwrap();
    let retirement = gate
        .reserve_retirement(MachineOutputRole::Both, &[])
        .unwrap();
    let worker = thread::spawn(move || retirement.finish_retirement());
    wait_for_retirement_condvar(&gate, &worker);
    assert!(gate.acquire(OutputCapability::Lighting).is_err());
    assert!(gate.acquire(OutputCapability::Video).is_err());
    assert!(gate.begin_transition(MachineOutputRole::Standby).is_err());
    drop(permit);
    worker
        .join()
        .unwrap()
        .unwrap()
        .complete_project_swap_disarmed()
        .unwrap();
    assert_eq!(gate.status().state, OutputOwnershipState::Ready);
    assert!(!gate.status().lighting_allowed && !gate.status().video_allowed);
}

#[test]
fn retirement_drain_timeout_retains_fence_and_the_specific_error_until_frame_release() {
    let gate = ready_gate();
    let permit = gate.acquire(OutputCapability::Lighting).unwrap();
    let retirement = gate
        .reserve_retirement(MachineOutputRole::Both, &[])
        .unwrap();
    let error = output_ownership_retirement::finish_retirement_with_timeout(
        retirement,
        Duration::from_millis(10),
    )
    .unwrap_err();
    assert!(error.contains("timed out"));
    assert_eq!(gate.status().error.as_deref(), Some(error.as_str()));
    assert!(gate.begin_transition(MachineOutputRole::Video).is_err());
    assert!(gate.acquire(OutputCapability::Lighting).is_err());
    drop(permit);
    assert_eq!(gate.status().state, OutputOwnershipState::Failed);
    assert_eq!(gate.status().error.as_deref(), Some(error.as_str()));
    gate.begin_transition(MachineOutputRole::Video)
        .unwrap()
        .complete()
        .unwrap();
}

#[test]
fn retirement_wait_rejects_superseding_fault_without_overwriting_its_epoch_or_error() {
    let gate = ready_gate();
    let permit = gate.acquire(OutputCapability::Lighting).unwrap();
    let retirement = gate
        .reserve_retirement(MachineOutputRole::Both, &[])
        .unwrap();
    let worker = thread::spawn(move || retirement.finish_retirement());
    wait_for_retirement_condvar(&gate, &worker);
    let teardown = gate.begin_failure_fence("newer worker fault");
    let fault_epoch = gate.status().epoch;
    assert!(worker.join().unwrap().unwrap_err().contains("superseded"));
    assert_eq!(gate.status().epoch, fault_epoch);
    assert_eq!(gate.status().error.as_deref(), Some("newer worker fault"));
    drop(permit);
    assert!(gate.begin_transition(MachineOutputRole::Video).is_err());
    drop(teardown);
    assert_eq!(gate.status().state, OutputOwnershipState::Failed);
    gate.begin_transition(MachineOutputRole::Video)
        .unwrap()
        .complete()
        .unwrap();
}

#[test]
fn retirement_failure_fence_signals_before_safety_change_and_retains_teardown() {
    let gate = ready_gate();
    let stop = Arc::new(AtomicBool::new(false));
    let prior_epoch = gate.status().epoch;
    let lease = gate.begin_failure_fence_stopping("managed retirement", &[stop.clone()]);
    assert!(stop.load(Ordering::Acquire));
    assert_eq!(gate.status().epoch, prior_epoch + 1);
    assert_eq!(gate.status().state, OutputOwnershipState::Failed);
    let teardown = gate.acquire_teardown_lease().unwrap();
    drop(teardown);
    assert_eq!(gate.inner.state.lock().unwrap().in_flight, 1);
    drop(lease);
}
