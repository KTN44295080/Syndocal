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
        assert_eq!(worker_gate.status().state, OutputOwnershipState::Transitioning);
        worker_gate.acquire_teardown_lease().unwrap()
    });
    let retirement = gate.reserve_retirement(MachineOutputRole::Both, &[stop]).unwrap();
    drop(permit);
    let teardown = worker.join().unwrap();
    assert_eq!(gate.inner.state.lock().unwrap().in_flight, 1);
    drop(teardown);
    retirement.finish_retirement().unwrap().complete_project_swap_disarmed().unwrap();
    assert_eq!(gate.status().state, OutputOwnershipState::Ready);
    assert!(!gate.status().video_allowed);
}

#[test]
fn retirement_requires_no_remaining_lease_and_rejects_superseding_fault() {
    for inject_fault in [false, true] {
        let gate = ready_gate();
        let retirement = gate.reserve_retirement(MachineOutputRole::Both, &[]).unwrap();
        let lease = if inject_fault {
            gate.begin_failure_fence("real worker failure")
        } else {
            gate.acquire_teardown_lease().unwrap()
        };
        assert!(retirement.finish_retirement().is_err());
        drop(lease);
        assert_eq!(gate.status().state, OutputOwnershipState::Failed);
    }
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
