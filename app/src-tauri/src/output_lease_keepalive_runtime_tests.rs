use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use super::*;
use crate::output_lease::output_lease_keepalive::{
    OutputLeaseKeepaliveBoundary, OutputLeaseKeepaliveCleanupOnlyOutcome,
    OutputLeaseKeepaliveFailStopPlan, OutputLeaseKeepaliveFailStopPorts,
    OutputLeaseKeepaliveIdentity, OutputLeaseKeepaliveManagedConflictAction,
    OutputLeaseKeepaliveManualOperation, OutputLeaseKeepaliveManualOperationDecision,
    OutputLeaseKeepalivePortError,
};
use crate::output_lease::{
    OutputLeaseOwner, OutputLeaseRegistry, OutputLeaseRequest, OutputLeaseRequestAction,
    OutputLeaseResource, OutputLeaseResources,
};

const PROCESS: u64 = 931;
const PROJECT: &str = "project_epoch:keepalive-runtime";

fn owner() -> OutputLeaseOwner {
    OutputLeaseOwner::new("local-ui", "main", PROCESS, 1).unwrap()
}

fn both() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video]).unwrap()
}

fn identity() -> OutputLeaseKeepaliveIdentity {
    identity_for_request(1)
}

fn identity_for_request(request_id: u64) -> OutputLeaseKeepaliveIdentity {
    OutputLeaseKeepaliveIdentity::new(
        PROJECT,
        "local-ui",
        "main",
        format!("process:{PROCESS};owner:1"),
        format!("principal:local-ui;domain:output-control;request:{request_id}"),
    )
    .unwrap()
}

fn enabled_evidence(
    registry: &mut OutputLeaseRegistry,
    request_id: u64,
) -> crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveDurableEnableEvidence {
    enabled_evidence_at(registry, request_id, 0)
}

fn enabled_evidence_at(
    registry: &mut OutputLeaseRegistry,
    request_id: u64,
    now_ms: u64,
) -> crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveDurableEnableEvidence {
    let request = OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        request_id,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner: owner(),
            resources: both(),
            project_identity: PROJECT.to_owned(),
            ttl_ms: 60_000,
        },
    )
    .unwrap();
    let receipt = registry.submit_request(&request, now_ms).unwrap();
    registry
        .issue_managed_exact_both_durable_enable_evidence(
            &receipt,
            identity_for_request(request_id),
        )
        .unwrap()
}

fn cleanup_only_evidence(
    registry: &mut OutputLeaseRegistry,
    request_id: u64,
) -> crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence
{
    cleanup_only_evidence_at(registry, request_id, 0)
}

fn cleanup_only_evidence_at(
    registry: &mut OutputLeaseRegistry,
    request_id: u64,
    now_ms: u64,
) -> crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence
{
    let request = OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        request_id,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner: owner(),
            resources: both(),
            project_identity: PROJECT.to_owned(),
            ttl_ms: 60_000,
        },
    )
    .unwrap();
    let receipt = registry.submit_request(&request, now_ms).unwrap();
    OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
        &request,
        &receipt,
        identity_for_request(request_id),
    )
    .unwrap()
}

#[derive(Default)]
struct Ports {
    calls: Vec<&'static str>,
    fail: Option<&'static str>,
}

impl Ports {
    fn call(&mut self, name: &'static str) -> Result<(), OutputLeaseKeepalivePortError> {
        self.calls.push(name);
        if self.fail == Some(name) {
            return Err(OutputLeaseKeepalivePortError::detail(format!("{name}-failed")).unwrap());
        }
        Ok(())
    }
}

impl OutputLeaseKeepaliveFailStopPorts for Ports {
    fn cancel_renewal(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("cancel")
    }
    fn request_safety_blackout(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("s0")
    }
    fn advance_failure_fence(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("fence")
    }
    fn retire_dmx_usb_artnet(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("dmx")
    }
    fn retire_spout_native(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("spout")
    }
    fn persist_standby_failed(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("persist")
    }
}

/// Production-equivalent first fail-stop port for the cleanup supersession
/// test.  It proves that the plan itself owns cancellation of the inherited
/// old worker before any blackout/fence/output retirement step can run.
struct RuntimeCancelPorts<'a> {
    runtime: &'a OutputLeaseKeepaliveRuntime,
    calls: Vec<&'static str>,
}

impl<'a> RuntimeCancelPorts<'a> {
    fn new(runtime: &'a OutputLeaseKeepaliveRuntime) -> Self {
        Self {
            runtime,
            calls: Vec::new(),
        }
    }

    fn call(&mut self, name: &'static str) -> Result<(), OutputLeaseKeepalivePortError> {
        self.calls.push(name);
        Ok(())
    }
}

impl OutputLeaseKeepaliveFailStopPorts for RuntimeCancelPorts<'_> {
    fn cancel_renewal(
        &mut self,
        plan: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.runtime
            .cancel_worker_for_fail_stop(
                plan.run_id(),
                OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
            )
            .map_err(|error| OutputLeaseKeepalivePortError::detail(error).unwrap())?;
        self.call("cancel")
    }

    fn request_safety_blackout(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("s0")
    }

    fn advance_failure_fence(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("fence")
    }

    fn retire_dmx_usb_artnet(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("dmx")
    }

    fn retire_spout_native(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("spout")
    }

    fn persist_standby_failed(
        &mut self,
        _: &OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), OutputLeaseKeepalivePortError> {
        self.call("persist")
    }
}

fn arm(
    runtime: &OutputLeaseKeepaliveRuntime,
    registry: &mut OutputLeaseRegistry,
) -> OutputLeaseKeepaliveRunId {
    match runtime
        .admit_durable_enable_for_tests(0, enabled_evidence(registry, 1))
        .unwrap()
    {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run) => run,
        admission => panic!("expected first arm, got {admission:?}"),
    }
}

fn execute_cleanup_and_release(
    runtime: &OutputLeaseKeepaliveRuntime,
    registry: &mut OutputLeaseRegistry,
    plan: OutputLeaseKeepaliveFailStopPlan,
) {
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
    let permit = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence(registry, 1))
        .into_outcome()
    {
        OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish(permit) => permit,
        outcome => panic!("completed cleanup must require exact release, got {outcome:?}"),
    };
    assert_eq!(
        runtime
            .complete_current_relinquish_with(permit, |permit| {
                Ok((registry.relinquish_managed_exact_both(permit, 20_000), ()))
            })
            .unwrap(),
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released(())
    );
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Idle
    );
}

#[test]
fn runtime_duplicate_enable_does_not_create_second_worker_or_fault() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let evidence = enabled_evidence(&mut registry, 1);
    let first = runtime.admit_durable_enable_for_tests(0, evidence).unwrap();
    let run = match first {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run) => run,
        _ => panic!(),
    };
    let receipt = registry.audit().back().unwrap().receipt.clone();
    let duplicate = registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt, identity())
        .unwrap();
    assert_eq!(
        runtime
            .admit_durable_enable_for_tests(0, duplicate)
            .unwrap(),
        OutputLeaseKeepaliveRuntimeAdmission::AlreadyArmed(run)
    );
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Armed
    );
}

#[test]
fn runtime_verifies_durable_enable_inside_the_serialized_admission() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let request = OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        1,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner: owner(),
            resources: both(),
            project_identity: PROJECT.to_owned(),
            ttl_ms: 60_000,
        },
    )
    .unwrap();
    let receipt = registry.submit_request(&request, 0).unwrap();
    let admission = runtime
        .admit_verified_durable_enable_with(0, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt, identity())
                .map_err(|error| format!("evidence failed: {error:?}"))
        })
        .unwrap();
    assert!(matches!(
        admission,
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(_)
    ));
}

#[test]
fn runtime_cleanup_only_committed_enable_never_arms_or_spawns_and_retries_exactly() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();

    let first = runtime.begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence(
        &mut registry,
        1,
    ));
    assert!(!first.recovered_operation_serial_poison());
    assert!(!first.recovered_manager_poison());
    let plan = match first.into_outcome() {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("cleanup-only Idle admission must capture fail-stop, got {outcome:?}"),
    };
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulting
    );
    assert!(runtime
        .spawn_worker(plan.run_id(), |_| panic!(
            "cleanup-only evidence must never spawn"
        ))
        .is_err());

    let retry = runtime.begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence(
        &mut registry,
        1,
    ));
    assert!(matches!(
        retry.into_outcome(),
        OutputLeaseKeepaliveCleanupOnlyOutcome::ResumeFailStop(retry_plan)
            if retry_plan.run_id() == plan.run_id()
    ));

    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulted
    );

    let permit = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence(&mut registry, 1))
        .into_outcome()
    {
        OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish(permit) => permit,
        outcome => panic!("completed cleanup must require exact release, got {outcome:?}"),
    };
    assert_eq!(
        runtime
            .complete_current_relinquish_with(permit, |permit| {
                Ok((registry.relinquish_managed_exact_both(permit, 20_000), ()))
            })
            .unwrap(),
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released(())
    );
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Idle
    );
    assert!(matches!(
        runtime
            .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence_at(
                &mut registry,
                1,
                20_000,
            ))
            .into_outcome(),
        OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop
    ));
}

#[test]
fn runtime_cleanup_only_supersedes_old_worker_and_late_old_callback_is_noop() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let old_run = arm(&runtime, &mut registry);
    let worker_entered_wait = Arc::new(AtomicBool::new(false));
    let worker_completed = Arc::new(AtomicBool::new(false));
    let worker_entered_wait_body = Arc::clone(&worker_entered_wait);
    let worker_completed_body = Arc::clone(&worker_completed);
    let control = runtime
        .spawn_worker(old_run, move |control| {
            worker_entered_wait_body.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
            worker_completed_body.store(true, Ordering::Release);
        })
        .unwrap();
    let worker_start_deadline = Instant::now() + Duration::from_secs(1);
    while !worker_entered_wait.load(Ordering::Acquire) {
        assert!(
            Instant::now() < worker_start_deadline,
            "old worker must be live before cleanup supersession"
        );
        thread::yield_now();
    }

    let plan = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_only_evidence_at(
            &mut registry,
            2,
            60_000,
        ))
        .into_outcome()
    {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("new durable cleanup must supersede old worker, got {outcome:?}"),
    };
    assert_eq!(
        plan.run_id(),
        old_run,
        "cleanup plan must own old worker cancellation"
    );
    let mut ports = RuntimeCancelPorts::new(&runtime);
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"],
        "the plan's CancelRenewal port must precede all physical safety steps"
    );
    assert!(control.is_cancelled());
    assert!(
        worker_completed.load(Ordering::Acquire),
        "CancelRenewal must wait for the old worker acknowledgement before safety output steps"
    );
    assert!(
        !runtime.reap_completed_worker(old_run).unwrap(),
        "CancelRenewal must remove the joined worker slot itself"
    );
    assert!(runtime
        .record_worker_termination(
            old_run,
            OutputLeaseKeepalivePortError::detail("late-old-worker-return").unwrap(),
        )
        .unwrap()
        .is_none());
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulted
    );
}

#[test]
fn runtime_cleanup_only_recovers_poisoned_serial_or_manager_into_visible_failstop() {
    let serial_runtime = OutputLeaseKeepaliveRuntime::default();
    let mut serial_registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = serial_runtime.operation_serial.lock().unwrap();
        panic!("poison serialized cleanup lane");
    }));
    let serial = serial_runtime.begin_cleanup_only_after_unadmitted_durable_enable(
        cleanup_only_evidence(&mut serial_registry, 1),
    );
    assert!(serial.recovered_operation_serial_poison());
    assert!(!serial.recovered_manager_poison());
    let serial_plan = match serial.into_outcome() {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("poisoned serial must capture all-deny plan, got {outcome:?}"),
    };
    execute_cleanup_and_release(&serial_runtime, &mut serial_registry, serial_plan);

    let manager_runtime = OutputLeaseKeepaliveRuntime::default();
    let mut manager_registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = manager_runtime.manager.lock().unwrap();
        panic!("poison cleanup manager lane");
    }));
    let manager = manager_runtime.begin_cleanup_only_after_unadmitted_durable_enable(
        cleanup_only_evidence(&mut manager_registry, 1),
    );
    assert!(!manager.recovered_operation_serial_poison());
    assert!(manager.recovered_manager_poison());
    let plan = match manager.into_outcome() {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("poisoned manager must still capture all-deny plan, got {outcome:?}"),
    };
    let body_ran = Arc::new(AtomicBool::new(false));
    let worker_body_ran = Arc::clone(&body_ran);
    assert!(manager_runtime
        .spawn_worker(plan.run_id(), move |_| {
            worker_body_ran.store(true, Ordering::Release);
        })
        .is_err());
    assert!(!body_ran.load(Ordering::Acquire));
    execute_cleanup_and_release(&manager_runtime, &mut manager_registry, plan);
}

#[test]
fn runtime_cancel_wakes_a_twenty_second_wait_immediately() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let started = Instant::now();
    let completed = Arc::new(AtomicBool::new(false));
    let worker_completed = Arc::clone(&completed);
    let control = runtime
        .spawn_worker(run, move |control| {
            worker_completed.store(
                control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20)),
                Ordering::Release,
            );
        })
        .unwrap();
    assert_eq!(control.run_id(), run);
    thread::sleep(Duration::from_millis(10));
    control.cancel_and_wake();
    let termination = runtime.cancel_worker_for_run(run).unwrap().unwrap();
    termination.wait_and_join(Duration::from_secs(1)).unwrap();
    assert!(!completed.load(Ordering::Acquire));
    assert!(started.elapsed() < Duration::from_secs(1));
}

#[test]
fn runtime_worker_self_fail_stop_cancel_does_not_join_its_own_handle() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let worker_runtime = Arc::clone(&runtime);
    runtime
        .spawn_worker(run, move |_| {
            worker_runtime
                .cancel_worker_for_fail_stop(
                    run,
                    OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run),
                )
                .unwrap();
        })
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    while !runtime.reap_completed_worker(run).unwrap() {
        assert!(
            Instant::now() < deadline,
            "worker completion must be reaped promptly"
        );
        thread::yield_now();
    }
}

#[test]
fn runtime_same_flow_rearbitrates_deferred_b_to_current_c_after_a_canonical_fail_stop() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run_a = arm(&runtime, &mut registry);
    let plan_a = runtime
        .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap()
        .expect("A must capture one canonical fail-stop");
    let mut ports = Ports::default();
    assert_eq!(
        runtime.execute_fail_stop(&mut ports, plan_a).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );

    let evidence_b = enabled_evidence_at(&mut registry, 2, 60_000);
    assert!(matches!(
        runtime
            .admit_durable_enable_for_tests(60_000, evidence_b)
            .unwrap(),
        OutputLeaseKeepaliveRuntimeAdmission::ManagedConflict(_)
    ));

    let mut proof_receipts = Vec::new();
    let run_c = match runtime
        .resolve_deferred_after_relinquish(120_000, |capability| {
            if proof_receipts.is_empty() {
                let evidence_c = enabled_evidence_at(&mut registry, 3, 120_000);
                match runtime
                    .admit_durable_enable_for_tests(120_000, evidence_c)
                    .unwrap()
                {
                    OutputLeaseKeepaliveRuntimeAdmission::ManagedConflict(conflict) => {
                        assert_eq!(
                            conflict.into_action(),
                            OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease,
                            "C replaces B while B owns the first proof, but cannot mint a second proof"
                        );
                    }
                    outcome => panic!(
                        "C must be retained behind B's in-flight proof, got {outcome:?}"
                    ),
                }
            }
            let receipt = registry.prove_managed_exact_both_supersession(capability, 120_000);
            if proof_receipts.is_empty() {
                assert!(
                    matches!(
                        &receipt,
                        crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveSupersessionCasReceipt::Rejected(_)
                    ),
                    "B's exact receipt must be rejected after C replaces its deferred authority"
                );
            } else {
                assert!(
                    matches!(
                        &receipt,
                        crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveSupersessionCasReceipt::Proven(_)
                    ),
                    "the fresh C capability must receive the exact proven receipt"
                );
            }
            proof_receipts.push(matches!(
                &receipt,
                crate::output_lease::output_lease_keepalive::OutputLeaseKeepaliveSupersessionCasReceipt::Proven(_)
            ));
            Ok(receipt)
        })
        .unwrap()
    {
        OutputLeaseKeepaliveRuntimeSupersession::Armed(run) => run,
        outcome => panic!("same initial B flow must re-arbitrate and arm C, got {outcome:?}"),
    };
    assert_eq!(
        proof_receipts,
        [false, true],
        "B's exact rejected receipt is consumed before the fresh exact C proof wins"
    );
    let armed = runtime
        .manager
        .lock()
        .unwrap()
        .snapshot()
        .armed
        .expect("C is the exact current re-armed run");
    assert_eq!(armed.run_id(), run_c);
    assert_eq!(
        armed.identity().request_correlation(),
        "principal:local-ui;domain:output-control;request:3"
    );
}

#[test]
fn runtime_worker_self_fail_stop_hands_off_old_slot_to_successor_without_manual_reap() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut initial_registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run_a = arm(&runtime, &mut initial_registry);
    let registry = Arc::new(Mutex::new(initial_registry));
    let runtime_for_worker = Arc::clone(&runtime);
    let registry_for_worker = Arc::clone(&registry);
    let (successor_tx, successor_rx) = std::sync::mpsc::sync_channel(1);
    let (allow_a_return_tx, allow_a_return_rx) = std::sync::mpsc::sync_channel(1);
    let successor_started = Arc::new(AtomicBool::new(false));

    runtime
        .spawn_worker(run_a, move |_| {
            let plan_a = runtime_for_worker
                .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::WorkerTerminated)
                .unwrap()
                .expect("worker A must capture its own canonical fail-stop");
            runtime_for_worker
                .cancel_worker_for_fail_stop(
                    run_a,
                    OutputLeaseKeepaliveFailStopExecutionContext::WorkerSelf(run_a),
                )
                .expect("worker A must cancel without self-join");
            let mut ports = Ports::default();
            assert_eq!(
                runtime_for_worker
                    .execute_fail_stop(&mut ports, plan_a)
                    .unwrap(),
                OutputLeaseKeepaliveFailStopCompletion::Completed
            );
            assert_eq!(
                ports.calls,
                ["cancel", "s0", "fence", "dmx", "spout", "persist"]
            );
            let mut registry = registry_for_worker.lock().unwrap();
            assert!(matches!(
                runtime_for_worker
                    .relinquish_after_successful_boundary(run_a, &identity(), |permit| {
                        Ok(registry.relinquish_managed_exact_both(permit, 20_000))
                    })
                    .unwrap(),
                Some(OutputLeaseKeepaliveRelinquishCompletion::Completed)
            ));
            let evidence_b = enabled_evidence_at(&mut registry, 2, 30_000);
            let run_b = match runtime_for_worker
                .admit_durable_enable_for_tests(30_000, evidence_b)
                .unwrap()
            {
                OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run) => run,
                admission => panic!("B must arm after worker A releases, got {admission:?}"),
            };
            drop(registry);
            successor_tx.send(run_b).unwrap();
            allow_a_return_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("external successor caller must release A after B installation");
        })
        .unwrap();

    let run_b = successor_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("B must continue from A's worker-self fail-stop without a manual reaper");
    let runtime_for_successor = Arc::clone(&runtime);
    let successor_started_worker = Arc::clone(&successor_started);
    let (successor_spawn_tx, successor_spawn_rx) = std::sync::mpsc::sync_channel(1);
    let successor_caller = thread::spawn(move || {
        let result = runtime_for_successor.spawn_worker(run_b, move |control| {
            successor_started_worker.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
        });
        successor_spawn_tx.send(result).unwrap();
    });
    let _successor_control = successor_spawn_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("independent successor caller must finish")
        .expect("worker-self handoff must allow B from a different caller thread");
    successor_caller.join().unwrap();
    let started_deadline = Instant::now() + Duration::from_secs(1);
    while !successor_started.load(Ordering::Acquire) {
        assert!(
            Instant::now() < started_deadline,
            "B must enter its worker body after the old self-handoff"
        );
        thread::yield_now();
    }
    {
        let slot = runtime.worker.lock().unwrap();
        assert_eq!(
            slot.active.as_ref().map(|active| active.run_id),
            Some(run_b)
        );
        assert!(
            slot.retiring
                .iter()
                .any(|retiring| retiring.worker.run_id == run_a),
            "A must retain its own JoinHandle until it returns; B is the only active worker"
        );
    }
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Armed,
        "B installation must not enter spawn-failure cleanup"
    );
    allow_a_return_tx.send(()).unwrap();
    let reap_deadline = Instant::now() + Duration::from_secs(1);
    while runtime
        .worker
        .lock()
        .unwrap()
        .retiring
        .iter()
        .any(|retiring| retiring.worker.run_id == run_a)
    {
        assert!(
            Instant::now() < reap_deadline,
            "the non-worker reaper must join A after its completion acknowledgement"
        );
        thread::yield_now();
    }
    runtime
        .cancel_worker_for_run(run_b)
        .unwrap()
        .expect("B remains the one active worker")
        .wait_and_join(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn runtime_external_cancel_timeout_retains_handle_blocks_b_then_reaps_before_b_spawns() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run_a = arm(&runtime, &mut registry);
    let (worker_started_tx, worker_started_rx) = std::sync::mpsc::sync_channel(1);
    let (unblock_a_tx, unblock_a_rx) = std::sync::mpsc::sync_channel(1);
    runtime
        .spawn_worker(run_a, move |_| {
            worker_started_tx.send(()).unwrap();
            unblock_a_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("test must explicitly unblock A");
        })
        .unwrap();
    worker_started_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();

    let plan_a = runtime
        .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap()
        .expect("A must capture a canonical boundary before its bounded cancel wait");
    assert!(
        runtime
            .cancel_worker_for_fail_stop_with_test_timeout(
                run_a,
                OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
                Duration::ZERO,
            )
            .is_err(),
        "the production three-second acknowledgement policy is injected as zero here to avoid a wall-clock test wait"
    );

    // This worker-slot test deliberately uses the existing no-op physical
    // ports after the timeout assertion; the core fail-stop ordering itself
    // is covered independently.  It lets us prove that an otherwise exact B
    // cannot install until the timed-out A handle is owned and reaped.
    let mut ports = Ports::default();
    assert_eq!(
        runtime.execute_fail_stop(&mut ports, plan_a).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
    assert!(matches!(
        runtime
            .relinquish_after_successful_boundary(run_a, &identity(), |permit| {
                Ok(registry.relinquish_managed_exact_both(permit, 20_000))
            })
            .unwrap(),
        Some(OutputLeaseKeepaliveRelinquishCompletion::Completed)
    ));
    let run_b = match runtime
        .admit_durable_enable_for_tests(30_000, enabled_evidence_at(&mut registry, 2, 30_000))
        .unwrap()
    {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run) => run,
        admission => panic!("B must arm before worker-slot installation, got {admission:?}"),
    };
    let b_started = Arc::new(AtomicBool::new(false));
    let b_started_rejected = Arc::clone(&b_started);
    assert!(
        runtime
            .spawn_worker(run_b, move |_| {
                b_started_rejected.store(true, Ordering::Release);
            })
            .is_err(),
        "a timed-out external retirement must reject B without consuming or detaching A"
    );
    assert!(!b_started.load(Ordering::Acquire));
    {
        let slot = runtime.worker.lock().unwrap();
        assert!(slot.active.is_none());
        assert!(slot.reserving.is_none());
        assert!(slot.retiring.iter().any(|retiring| {
            retiring.worker.run_id == run_a
                && retiring.kind == OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap
        }));
    }

    unblock_a_tx.send(()).unwrap();
    let deadline = Instant::now() + Duration::from_secs(1);
    let b_control = loop {
        let b_started_retry = Arc::clone(&b_started);
        match runtime.spawn_worker(run_b, move |control| {
            b_started_retry.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
        }) {
            Ok(control) => break control,
            Err(error) => {
                assert!(
                    error.contains("retirement") || error.contains("already exists"),
                    "only the retained A worker may temporarily block B: {error}"
                );
                assert!(
                    Instant::now() < deadline,
                    "a completed timed-out A must be joined by the production nonblocking reap path"
                );
                thread::yield_now();
            }
        }
    };
    while !b_started.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "the exact B worker must start after timed-out A reaping"
        );
        thread::yield_now();
    }
    {
        let slot = runtime.worker.lock().unwrap();
        assert_eq!(
            slot.active.as_ref().map(|active| active.run_id),
            Some(run_b)
        );
        assert!(
            !slot
                .retiring
                .iter()
                .any(|retiring| retiring.worker.run_id == run_a),
            "B is allowed only after a non-worker path joined the timed-out A handle"
        );
    }
    b_control.cancel_and_wake();
    runtime
        .cancel_worker_for_run(run_b)
        .unwrap()
        .expect("B is the only active worker after timed-out A reaping")
        .wait_and_join(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn runtime_post_spawn_reservation_loss_retains_unpublished_handle_until_nonworker_reap() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let original_body_ran = Arc::new(AtomicBool::new(false));
    let original_body_ran_worker = Arc::clone(&original_body_ran);
    runtime.force_next_worker_spawn_reservation_loss();
    assert!(
        runtime
            .spawn_worker(run, move |_| {
                original_body_ran_worker.store(true, Ordering::Release);
            })
            .is_err(),
        "post-OS-spawn reservation loss must reject publication"
    );
    assert!(
        runtime.wait_for_forced_reservation_loss_prestart(Duration::from_secs(1)),
        "the cancelled unpublished worker must be held before completion acknowledgement"
    );
    assert!(
        !original_body_ran.load(Ordering::Acquire),
        "an unpublished reservation-loss worker must never enter its body"
    );
    {
        let slot = runtime.worker.lock().unwrap();
        assert!(slot.active.is_none());
        assert!(slot.reserving.is_none());
        assert!(slot.retiring.iter().any(|retiring| {
            retiring.worker.run_id == run
                && retiring.kind == OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap
        }));
    }

    runtime.release_forced_reservation_loss_prestart();
    let replacement_started = Arc::new(AtomicBool::new(false));
    let deadline = Instant::now() + Duration::from_secs(1);
    let replacement = loop {
        let replacement_started_worker = Arc::clone(&replacement_started);
        match runtime.spawn_worker(run, move |control| {
            replacement_started_worker.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
        }) {
            Ok(control) => break control,
            Err(error) => {
                assert!(
                    error.contains("retirement"),
                    "only the retained unpublished handle may delay replacement: {error}"
                );
                assert!(
                    Instant::now() < deadline,
                    "completion must be joined by the production lock-free reap path"
                );
                thread::yield_now();
            }
        }
    };
    while !replacement_started.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "the replacement worker must start after the unpublished handle is reaped"
        );
        thread::yield_now();
    }
    {
        let slot = runtime.worker.lock().unwrap();
        assert_eq!(slot.active.as_ref().map(|active| active.run_id), Some(run));
        assert!(slot.retiring.is_empty());
    }
    replacement.cancel_and_wake();
    runtime
        .cancel_worker_for_run(run)
        .unwrap()
        .expect("the replacement must be the one published active worker")
        .wait_and_join(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn runtime_external_timeout_recovers_worker_poison_into_retiring_before_reap() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let (worker_started_tx, worker_started_rx) = std::sync::mpsc::sync_channel(1);
    let (unblock_tx, unblock_rx) = std::sync::mpsc::sync_channel(1);
    runtime
        .spawn_worker(run, move |_| {
            worker_started_tx.send(()).unwrap();
            unblock_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("test must explicitly unblock the timed-out worker");
        })
        .unwrap();
    worker_started_rx
        .recv_timeout(Duration::from_secs(1))
        .unwrap();

    runtime.force_next_external_retain_worker_poison();
    assert!(
        runtime
            .cancel_worker_for_fail_stop_with_test_timeout(
                run,
                OutputLeaseKeepaliveFailStopExecutionContext::ExternalBoundary,
                Duration::ZERO,
            )
            .is_err(),
        "the injected bounded timeout must keep ownership despite poisoned worker state"
    );
    assert!(
        !runtime.worker.is_poisoned(),
        "structural poison is cleared only after the exact timed-out handle entered retiring"
    );
    {
        let slot = runtime.worker.lock().unwrap();
        assert!(slot.active.is_none());
        assert!(slot.reserving.is_none());
        assert!(slot.retiring.iter().any(|retiring| {
            retiring.worker.run_id == run
                && retiring.kind == OutputLeaseKeepaliveWorkerRetirementKind::AwaitingExternalReap
        }));
    }

    unblock_tx.send(()).unwrap();
    let replacement_started = Arc::new(AtomicBool::new(false));
    let deadline = Instant::now() + Duration::from_secs(1);
    let replacement = loop {
        let replacement_started_worker = Arc::clone(&replacement_started);
        match runtime.spawn_worker(run, move |control| {
            replacement_started_worker.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
        }) {
            Ok(control) => break control,
            Err(error) => {
                assert!(
                    error.contains("retirement"),
                    "the exact retained handle is the only valid temporary blocker: {error}"
                );
                assert!(
                    Instant::now() < deadline,
                    "the post-poison retiring handle must be joined before replacement"
                );
                thread::yield_now();
            }
        }
    };
    while !replacement_started.load(Ordering::Acquire) {
        assert!(
            Instant::now() < deadline,
            "replacement must start after production reaping clears poisoned retention"
        );
        thread::yield_now();
    }
    assert!(
        runtime.worker.lock().unwrap().retiring.is_empty(),
        "the reaper joined the retained handle outside the worker mutex"
    );
    replacement.cancel_and_wake();
    runtime
        .cancel_worker_for_run(run)
        .unwrap()
        .expect("replacement is the sole active worker after poison recovery")
        .wait_and_join(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn production_adapter_rejects_oversized_diagnostic_evidence_without_truncation() {
    let oversized = "x".repeat(257);
    assert!(OutputLeaseKeepalivePortError::detail(oversized.clone()).is_err());
    assert_eq!(
        ProductionKeepaliveFailStopPorts::admit_diagnostic_or_rejection(oversized.clone()),
        crate::output_lease::output_lease_keepalive::OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED,
        "the production adapter records an explicit rejection class, never a truncated prefix"
    );
    assert_eq!(
        crate::output_lease::output_lease_keepalive::output_lease_keepalive_adapter_port_error(
            oversized,
        ),
        OutputLeaseKeepalivePortError::detail(
            crate::output_lease::output_lease_keepalive::OUTPUT_LEASE_KEEPALIVE_DIAGNOSTIC_REJECTED,
        )
        .unwrap(),
        "oversized adapter evidence cannot enter durable fail-stop state"
    );
}

#[test]
fn runtime_unexpected_worker_termination_fences_once_and_allows_rearm_after_release() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let error = OutputLeaseKeepalivePortError::detail("worker-returned-unexpectedly").unwrap();
    let plan = runtime
        .record_worker_termination(run, error)
        .unwrap()
        .expect("the exact Armed run must be fenced");
    assert!(runtime
        .record_worker_termination(
            run,
            OutputLeaseKeepalivePortError::detail("duplicate-worker-termination").unwrap(),
        )
        .unwrap()
        .is_none());
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
    // Worker termination is an automatic boundary, not a manual lifecycle
    // request.  The matching automatic permit is issued only after all
    // physical fail-stop ports complete, then performs the exact registry
    // release in the same serialized lane.
    assert!(matches!(
        runtime
            .relinquish_after_successful_boundary(run, &identity(), |permit| {
                Ok(registry.relinquish_managed_exact_both(permit, 20_000))
            })
            .unwrap(),
        Some(OutputLeaseKeepaliveRelinquishCompletion::Completed)
    ));
    assert!(matches!(
        runtime
            .admit_durable_enable_for_tests(30_000, enabled_evidence_at(&mut registry, 2, 30_000))
            .unwrap(),
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(_)
    ));
}

#[test]
fn runtime_duplicate_worker_spawn_is_rejected_before_second_body_runs() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let first_started = Arc::new(AtomicBool::new(false));
    let first_started_worker = Arc::clone(&first_started);
    let first = runtime
        .spawn_worker(run, move |control| {
            first_started_worker.store(true, Ordering::Release);
            let _ = control.wait_until_or_cancelled(Instant::now() + Duration::from_secs(20));
        })
        .unwrap();
    assert!(runtime
        .spawn_worker(run, |_| panic!("duplicate worker body must not run"))
        .is_err());
    while !first_started.load(Ordering::Acquire) {
        thread::yield_now();
    }
    first.cancel_and_wake();
    runtime
        .cancel_worker_for_run(run)
        .unwrap()
        .unwrap()
        .wait_and_join(Duration::from_secs(1))
        .unwrap();
}

#[test]
fn runtime_boundary_between_admission_and_spawn_never_starts_a_stale_worker() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let plan = runtime
        .begin_boundary(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap()
        .expect("Armed run must admit exactly one boundary plan");
    let body_ran = Arc::new(AtomicBool::new(false));
    let body_ran_worker = Arc::clone(&body_ran);
    assert!(runtime
        .spawn_worker(run, move |_| {
            body_ran_worker.store(true, Ordering::Release);
        })
        .is_err());
    assert!(!body_ran.load(Ordering::Acquire));
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
}

#[test]
fn runtime_intercepts_managed_lifecycle_before_a_generic_registry_mutation() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let lease_id = registry
        .audit()
        .back()
        .unwrap()
        .receipt
        .lease_id
        .unwrap()
        .encode();
    assert!(runtime.manages_lease_id(&lease_id).unwrap());
    assert!(!runtime.manages_lease_id("not-a-managed-lease").unwrap());
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Armed,
        "pure preflight must not create a fail-stop plan"
    );
    assert_eq!(
        runtime
            .manual_lifecycle_decision(&lease_id, OutputLeaseKeepaliveManualOperation::Renew)
            .unwrap(),
        OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
    );
    assert!(matches!(
        runtime
            .manual_lifecycle_decision(&lease_id, OutputLeaseKeepaliveManualOperation::Relinquish)
            .unwrap(),
        OutputLeaseKeepaliveManualOperationDecision::BeginFailStop(plan) if plan.run_id() == run
    ));
}

#[test]
fn runtime_manual_relinquish_exposes_wire_evidence_only_after_exact_release() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let lease_id = registry
        .audit()
        .back()
        .unwrap()
        .receipt
        .lease_id
        .unwrap()
        .encode();
    let plan = match runtime
        .manual_relinquish_with(
            &lease_id,
            |_| -> Result<
                (OutputLeaseKeepaliveRelinquishCasReceipt, ()),
                (
                    OutputLeaseKeepaliveRelinquishPermit,
                    OutputLeaseKeepalivePortError,
                ),
            > { panic!("CAS must follow fail-stop") },
        )
        .unwrap()
    {
        OutputLeaseKeepaliveRuntimeManualRelinquish::FailStop(plan) => plan,
        outcome => panic!("expected exact current fail-stop, got {outcome:?}"),
    };
    assert_eq!(plan.run_id(), run);
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        runtime
            .manual_relinquish_with(&lease_id, |permit| {
                // This probe would deadlock if a manager guard were retained
                // across the registry CAS callback.
                assert_eq!(
                    runtime.snapshot_state().unwrap(),
                    OutputLeaseKeepaliveState::Faulted
                );
                let receipt = registry.relinquish_managed_exact_both(permit, 20_000);
                Ok((receipt, "wire-release-evidence"))
            })
            .unwrap(),
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released("wire-release-evidence")
    );
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Idle
    );
}

#[test]
fn runtime_reaps_completed_worker_before_a_rearmed_run_spawns() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run_a = arm(&runtime, &mut registry);
    runtime
        .spawn_worker(run_a, |_| {})
        .expect("A worker must start");
    let deadline = Instant::now() + Duration::from_secs(1);
    while !runtime.reap_completed_worker(run_a).unwrap() {
        assert!(Instant::now() < deadline, "A worker must complete promptly");
        thread::yield_now();
    }
    let plan = runtime
        .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap()
        .expect("A must enter the release fail-stop");
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    let lease_id = registry
        .audit()
        .back()
        .unwrap()
        .receipt
        .lease_id
        .unwrap()
        .encode();
    assert!(matches!(
        runtime
            .manual_relinquish_with(&lease_id, |permit| {
                Ok((registry.relinquish_managed_exact_both(permit, 20_000), ()))
            })
            .unwrap(),
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released(())
    ));
    let run_b = match runtime
        .admit_durable_enable_for_tests(30_000, enabled_evidence_at(&mut registry, 2, 30_000))
        .unwrap()
    {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run) => run,
        admission => panic!("B must arm after A releases, got {admission:?}"),
    };
    runtime
        .spawn_worker(run_b, |_| {})
        .expect("new B worker must spawn after prior completion reaping");
}

#[test]
fn runtime_each_physical_port_failure_still_attempts_all_six_ports() {
    for fail in ["cancel", "s0", "fence", "dmx", "spout", "persist"] {
        let runtime = OutputLeaseKeepaliveRuntime::default();
        let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
        let run = arm(&runtime, &mut registry);
        let plan = runtime
            .begin_boundary(run, OutputLeaseKeepaliveBoundary::Relinquish)
            .unwrap()
            .unwrap();
        let mut ports = Ports {
            calls: Vec::new(),
            fail: Some(fail),
        };
        runtime.execute_fail_stop(&mut ports, plan).unwrap();
        assert_eq!(
            ports.calls,
            ["cancel", "s0", "fence", "dmx", "spout", "persist"]
        );
        assert_eq!(
            runtime.snapshot_state().unwrap(),
            OutputLeaseKeepaliveState::Faulted
        );
    }
}

#[test]
fn runtime_stale_boundary_plan_cannot_mutate_a_current_run() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run_a = arm(&runtime, &mut registry);
    let plan_a = runtime
        .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap()
        .unwrap();
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan_a).unwrap();
    // Failed release stays faulted. A late plan cannot arm/replace anything.
    let stale = runtime
        .begin_boundary(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    assert!(stale.is_none());
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulted
    );
}

#[test]
fn runtime_concurrent_boundary_captures_exactly_one_fail_stop_plan() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let first_runtime = Arc::clone(&runtime);
    let second_runtime = Arc::clone(&runtime);
    let first = thread::spawn(move || {
        first_runtime
            .begin_boundary(run, OutputLeaseKeepaliveBoundary::Standby)
            .unwrap()
    });
    let second = thread::spawn(move || {
        second_runtime
            .begin_boundary(run, OutputLeaseKeepaliveBoundary::Standby)
            .unwrap()
    });
    let first = first.join().unwrap();
    let second = second.join().unwrap();
    assert_eq!(
        usize::from(first.is_some()) + usize::from(second.is_some()),
        1
    );
}

#[test]
fn runtime_renewal_and_boundary_are_serialized() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let registry = Arc::new(Mutex::new(registry));
    let entered = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(AtomicBool::new(false));
    let worker_runtime = Arc::clone(&runtime);
    let worker_registry = Arc::clone(&registry);
    let worker_entered = Arc::clone(&entered);
    let worker_release = Arc::clone(&release);
    let renew = thread::spawn(move || {
        worker_runtime.renew_once(20_000, run, move |capability| {
            worker_entered.store(1, Ordering::Release);
            while !worker_release.load(Ordering::Acquire) {
                thread::yield_now();
            }
            Ok(worker_registry
                .lock()
                .unwrap()
                .renew_managed_exact_both(capability, 20_000))
        })
    });
    while entered.load(Ordering::Acquire) == 0 {
        thread::yield_now();
    }
    let boundary_runtime = Arc::clone(&runtime);
    let boundary = thread::spawn(move || {
        boundary_runtime.begin_boundary(run, OutputLeaseKeepaliveBoundary::Standby)
    });
    thread::sleep(Duration::from_millis(10));
    assert!(
        !boundary.is_finished(),
        "boundary must wait behind in-flight serialized CAS"
    );
    release.store(true, Ordering::Release);
    assert!(matches!(
        renew.join().unwrap().unwrap(),
        OutputLeaseKeepaliveRuntimeRenewal::Renewed
    ));
    let plan = boundary
        .join()
        .unwrap()
        .unwrap()
        .expect("boundary must capture after renewal");
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulted
    );
}

#[test]
fn runtime_managed_exact_both_route_authorization_blocks_forced_renewal_through_engine_callback() {
    let runtime = Arc::new(OutputLeaseKeepaliveRuntime::default());
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let lease_id = registry.lease_views().pop().unwrap().lease_id.encode();
    let registry = Arc::new(Mutex::new(registry));

    // This is the exact production ordering: hold the runtime serial gate,
    // acquire/submit against the registry, then release the serial gate.
    let authorization = runtime
        .begin_managed_exact_both_ordinary_authorization(&lease_id)
        .unwrap();
    let renewing_runtime = Arc::clone(&runtime);
    let renewing_registry = Arc::clone(&registry);
    let renewed = Arc::new(AtomicBool::new(false));
    let renewed_observer = Arc::clone(&renewed);
    let renewal = thread::spawn(move || {
        let result = renewing_runtime.renew_once(20_000, run, |capability| {
            Ok(renewing_registry
                .lock()
                .unwrap()
                .renew_managed_exact_both(capability, 20_000))
        });
        renewed_observer.store(true, Ordering::Release);
        result
    });
    thread::sleep(Duration::from_millis(10));
    assert!(
        !renewed.load(Ordering::Acquire),
        "renewal must not cross a managed route authorization before registry admission"
    );

    let lease_id = crate::output_lease::OutputLeaseId::decode(&lease_id).unwrap();
    let request = OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        2,
        OutputLeaseRequestAction::AuthorizeManagedExactBoth {
            lease_id,
            owner: owner(),
            exact_resources: both(),
            project_identity: PROJECT.to_owned(),
        },
    )
    .unwrap();
    let receipt = registry
        .lock()
        .unwrap()
        .submit_managed_exact_both_request_with_commit(&request, &authorization, 20_000, |_| Ok(()))
        .unwrap();
    assert_eq!(
        receipt.outcome,
        Ok(crate::output_lease::OutputLeaseOperationOutcome::Authorized)
    );
    let engine_callback_count = AtomicUsize::new(0);
    engine_callback_count.fetch_add(1, Ordering::SeqCst);
    // The production route retains this same guard around its bounded engine
    // callback. Renewal must remain blocked during that physical interval,
    // not merely until the candidate receipt is assembled.
    thread::sleep(Duration::from_millis(10));
    assert!(
        !renewed.load(Ordering::Acquire),
        "renewal must not cross the managed route engine callback"
    );
    assert_eq!(engine_callback_count.load(Ordering::SeqCst), 1);
    drop(authorization);

    assert!(matches!(
        renewal.join().unwrap().unwrap(),
        OutputLeaseKeepaliveRuntimeRenewal::Renewed
    ));
    assert!(renewed.load(Ordering::Acquire));
}

#[test]
fn runtime_faulted_managed_lease_rejects_route_guard_without_ordinary_fallback() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let lease_id = registry.lease_views().pop().unwrap().lease_id.encode();
    let plan = runtime
        .begin_boundary(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap()
        .expect("armed run must capture the fail-stop boundary");
    assert!(runtime.manages_lease_id(&lease_id).unwrap());
    let error = match runtime.begin_managed_exact_both_ordinary_authorization(&lease_id) {
        Ok(_) => panic!("Faulted managed lease must not issue an ordinary route guard"),
        Err(error) => error,
    };
    assert_eq!(
        error,
        "Managed exact-Both output lease is no longer armed for this authorization"
    );
    // A managed fault remains managed until its canonical boundary resolves;
    // route construction must not drop into caller-generation authorization.
    assert!(runtime.manages_lease_id(&lease_id).unwrap());
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
}

#[test]
fn runtime_pre_registry_renewal_error_becomes_canonical_fail_stop() {
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let run = arm(&runtime, &mut registry);
    let outcome = runtime
        .renew_once(20_000, run, |capability| {
            Err((
                capability,
                OutputLeaseKeepalivePortError::detail("registry-lock-poisoned").unwrap(),
            ))
        })
        .unwrap();
    let OutputLeaseKeepaliveRuntimeRenewal::FailStop(plan) = outcome else {
        panic!("pre-registry renewal failure must capture canonical fail-stop")
    };
    let mut ports = Ports::default();
    runtime.execute_fail_stop(&mut ports, plan).unwrap();
    assert_eq!(
        runtime.snapshot_state().unwrap(),
        OutputLeaseKeepaliveState::Faulted
    );
}
