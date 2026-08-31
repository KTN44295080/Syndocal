use super::super::{
    OutputLeaseError, OutputLeaseGrant, OutputLeaseId, OutputLeaseOwner, OutputLeasePhase,
    OutputLeaseRegistry, OutputLeaseRequest, OutputLeaseRequestAction, OutputLeaseRequestReceipt,
    OutputLeaseResource, OutputLeaseResources,
};
use super::*;

const PROCESS: u64 = 91;
const PROJECT: &str = "project:keepalive";

fn owner() -> OutputLeaseOwner {
    OutputLeaseOwner::new("local-ui", "main", PROCESS, 1).unwrap()
}

fn both() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video]).unwrap()
}

fn exact_both_enable_request(request_id: u64, ttl_ms: u64) -> OutputLeaseRequest {
    OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        request_id,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner: owner(),
            resources: both(),
            project_identity: PROJECT.to_owned(),
            ttl_ms,
        },
    )
    .unwrap()
}

fn enable_exact_both(
    registry: &mut OutputLeaseRegistry,
    request_id: u64,
    now_ms: u64,
    ttl_ms: u64,
) -> (OutputLeaseGrant, OutputLeaseRequestReceipt) {
    let request = exact_both_enable_request(request_id, ttl_ms);
    let receipt = registry.submit_request(&request, now_ms).unwrap();
    let lease_id = receipt.lease_id.expect("canonical Enable returns a lease");
    let snapshot = registry.lease_view(lease_id).unwrap().snapshot;
    (OutputLeaseGrant { lease_id, snapshot }, receipt)
}

fn registry_with_grant(
    ttl_ms: u64,
) -> (
    OutputLeaseRegistry,
    OutputLeaseGrant,
    OutputLeaseRequestReceipt,
) {
    let mut registry = OutputLeaseRegistry::fresh_process(PROCESS).unwrap();
    let (grant, receipt) = enable_exact_both(&mut registry, 1, 0, ttl_ms);
    (registry, grant, receipt)
}

fn identity(request: u64) -> OutputLeaseKeepaliveIdentity {
    OutputLeaseKeepaliveIdentity::new(
        PROJECT,
        "local-ui",
        "main",
        format!("process:{PROCESS};owner:1"),
        format!("principal:local-ui;domain:output-control;request:{request}"),
    )
    .unwrap()
}

fn cleanup_identity(
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> OutputLeaseKeepaliveIdentity {
    OutputLeaseKeepaliveIdentity::new(
        PROJECT,
        "local-ui",
        "main",
        format!("process:{};owner:1", receipt.process_session_incarnation),
        format!(
            "principal:{};domain:{};request:{}",
            request.key.principal, request.key.domain, request.key.request_id
        ),
    )
    .unwrap()
}

fn cleanup_evidence(
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence {
    OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
        request,
        receipt,
        cleanup_identity(request, receipt),
    )
    .unwrap()
}

fn evidence(
    registry: &OutputLeaseRegistry,
    receipt: &OutputLeaseRequestReceipt,
    request: u64,
) -> OutputLeaseKeepaliveDurableEnableEvidence {
    registry
        .issue_managed_exact_both_durable_enable_evidence(receipt, identity(request))
        .unwrap()
}

fn arm(
    registry: &OutputLeaseRegistry,
    receipt: &OutputLeaseRequestReceipt,
    request: u64,
) -> (OutputLeaseKeepaliveManager, OutputLeaseKeepaliveRunId) {
    let mut manager = OutputLeaseKeepaliveManager::default();
    let run =
        match manager.arm_or_return_bound_fail_stop_plan(0, evidence(registry, receipt, request)) {
            OutputLeaseKeepaliveArmOutcome::Armed(run) => run,
            outcome => panic!("valid initial durable Enable must arm, got {outcome:?}"),
        };
    (manager, run)
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
            Err(OutputLeaseKeepalivePortError::detail(format!("{name} failed")).unwrap())
        } else {
            Ok(())
        }
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

fn execute_current_plan(
    manager: &mut OutputLeaseKeepaliveManager,
    ports: &mut Ports,
    plan: OutputLeaseKeepaliveFailStopPlan,
) {
    let OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(execution) =
        execute_fail_stop_plan(ports, plan)
    else {
        panic!("current plan must be executable exactly once");
    };
    assert_eq!(
        manager.complete_fail_stop(execution).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
}

fn fault_and_relinquish(
    registry: &mut OutputLeaseRegistry,
    manager: &mut OutputLeaseKeepaliveManager,
    run: OutputLeaseKeepaliveRunId,
    lease_id: OutputLeaseId,
) {
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(manager, &mut ports, plan);
    let permit = match manager.manual_operation_decision(
        &lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        decision => panic!("expected exact permit, got {decision:?}"),
    };
    let receipt = registry.relinquish_managed_exact_both(permit, 1);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(receipt),
        OutputLeaseKeepaliveRelinquishCompletion::Completed
    );
}

fn complete_exact_relinquish(
    registry: &mut OutputLeaseRegistry,
    manager: &mut OutputLeaseKeepaliveManager,
    permit: OutputLeaseKeepaliveRelinquishPermit,
    now_ms: u64,
) {
    let receipt = registry.relinquish_managed_exact_both(permit, now_ms);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(receipt),
        OutputLeaseKeepaliveRelinquishCompletion::Completed
    );
}

#[test]
fn exact_canonical_authority_validation_rejects_ambiguous_tokens() {
    assert!(OutputLeaseKeepaliveExactBothLease::from_verified_exact_both("", 1).is_err());
    assert!(
        OutputLeaseKeepaliveExactBothLease::from_verified_exact_both("lease-0000000000000000", 1)
            .is_err()
    );
    assert!(
        OutputLeaseKeepaliveExactBothLease::from_verified_exact_both("lease-0000000000000001", 0)
            .is_err()
    );
    assert!(OutputLeaseKeepaliveIdentity::new("", "owner", "main", "session", "request").is_err());
}

#[test]
fn durable_enable_evidence_requires_current_exact_registry_receipt() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let mut wrong = receipt.clone();
    wrong.owner = Some(OutputLeaseOwner::new("other", "main", PROCESS, 1).unwrap());
    assert_eq!(
        registry.issue_managed_exact_both_durable_enable_evidence(&wrong, identity(1)),
        Err(OutputLeaseError::InvalidRequest)
    );
    let mut wrong_issuance = receipt;
    wrong_issuance.audit_sequence = 0;
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&wrong_issuance, identity(1))
        .is_err());
}

#[test]
fn downstream_managed_cas_rejects_tampered_session_or_request_correlation() {
    for invalid_identity in [
        OutputLeaseKeepaliveIdentity::new(
            PROJECT,
            "local-ui",
            "main",
            "process:91;owner:999",
            "principal:local-ui;domain:output-control;request:1",
        )
        .unwrap(),
        OutputLeaseKeepaliveIdentity::new(
            PROJECT,
            "local-ui",
            "main",
            "process:91;owner:1",
            "principal:local-ui;domain:output-control;request:999",
        )
        .unwrap(),
    ] {
        let (mut registry, _grant, receipt) = registry_with_grant(60_000);
        let (mut manager, run) = arm(&registry, &receipt, 1);
        let mut capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
        capability.identity = invalid_identity;
        let before = registry.clone();

        assert!(matches!(
            registry.renew_managed_exact_both(capability, 20_000),
            OutputLeaseKeepaliveRenewalCasReceipt::Rejected(_)
        ));
        assert_eq!(
            registry, before,
            "the shared downstream CAS binding must reject identity tampering before mutation"
        );
    }
}

#[test]
fn durable_enable_issuance_highwater_rejects_replay_after_idle() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    fault_and_relinquish(&mut registry, &mut manager, run_a, grant_a.lease_id);
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let first = evidence(&registry, &receipt_b, 2);
    let replay = evidence(&registry, &receipt_b, 2);
    let run_b = match manager.arm_or_return_bound_fail_stop_plan(1, first) {
        OutputLeaseKeepaliveArmOutcome::Armed(run) => run,
        outcome => panic!("fresh durable receipt must arm, got {outcome:?}"),
    };
    fault_and_relinquish(&mut registry, &mut manager, run_b, grant_b.lease_id);
    assert_eq!(
        manager.arm_or_return_bound_fail_stop_plan(2, replay),
        OutputLeaseKeepaliveArmOutcome::StaleNoop
    );
    let (_grant_c, receipt_c) = enable_exact_both(&mut registry, 3, 2, 60_000);
    let fresh = evidence(&registry, &receipt_c, 3);
    assert!(matches!(
        manager.arm_or_return_bound_fail_stop_plan(2, fresh),
        OutputLeaseKeepaliveArmOutcome::Armed(_)
    ));
}

#[test]
fn cleanup_only_supersession_clears_all_poisoned_state_slots_and_fences_old_gates() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);

    // Manufacture the kind of partial state a poisoned manager lock can leave:
    // Armed, Faulting, and Faulted all coexist with individually exported
    // capabilities. This is deliberately impossible through normal APIs.
    let mut stale_armed = manager.armed.clone().unwrap();
    let stale_renewal_gate = OutputLeaseKeepaliveRenewalGate::issued();
    stale_armed.renewal_in_flight = true;
    stale_armed.renewal_attempt = 41;
    stale_armed.renewal_gate = Some(std::sync::Arc::clone(&stale_renewal_gate));
    let stale_renewal = OutputLeaseKeepaliveRenewalCapability {
        run_id: stale_armed.run_id,
        renewal_attempt: stale_armed.renewal_attempt,
        identity: stale_armed.identity.clone(),
        lease: stale_armed.lease.clone(),
        registry_claim: stale_armed.registry_claim.clone(),
        gate: stale_renewal_gate,
    };

    let active_plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut stale_faulting = manager.faulting.clone().unwrap();
    stale_faulting.fail_stop_gate = OutputLeaseKeepaliveFailStopExecutionGate::issued();
    let stale_faulting_plan =
        OutputLeaseKeepaliveManager::fail_stop_plan_from_fault(&stale_faulting);

    let mut completed_ports = Ports::default();
    execute_current_plan(&mut manager, &mut completed_ports, active_plan);
    let stale_permit = match manager.manual_operation_decision(
        &grant_a.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        decision => panic!("faulted A must have one exact permit, got {decision:?}"),
    };

    manager.armed = Some(stale_armed.clone());
    manager.faulting = Some(stale_faulting);
    manager.deferred = Some(OutputLeaseKeepaliveDeferredDurableEnable {
        identity: stale_armed.identity.clone(),
        lease: stale_armed.lease.clone(),
        registry_claim: stale_armed.registry_claim.clone(),
    });

    assert!(registry.observe_expiry(grant_a.lease_id, 60_000).unwrap());
    let request_b = exact_both_enable_request(2, 60_000);
    let receipt_b = registry.submit_request(&request_b, 60_000).unwrap();
    let cleanup_plan = match manager.begin_cleanup_only_after_unadmitted_durable_enable(
        cleanup_evidence(&request_b, &receipt_b),
    ) {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("cleanup B must replace all poisoned A slots, got {outcome:?}"),
    };

    // The prior Armed ID wins cancellation ownership, but no old state slot or
    // deferred evidence survives alongside the new B cleanup fault.
    assert_eq!(cleanup_plan.run_id(), run_a);
    assert!(manager.armed.is_none());
    assert!(manager.faulted.is_none());
    assert!(manager.deferred.is_none());
    assert_eq!(
        manager.snapshot().state,
        OutputLeaseKeepaliveState::Faulting
    );
    assert_eq!(
        manager.snapshot().fault.unwrap().lease().generation(),
        receipt_b.generation_after.unwrap()
    );

    let before = registry.clone();
    let mut stale_ports = Ports::default();
    assert_eq!(
        execute_fail_stop_plan(&mut stale_ports, stale_faulting_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    );
    assert!(stale_ports.calls.is_empty());

    let stale_renewal_receipt = registry.renew_managed_exact_both(stale_renewal, 60_000);
    assert_eq!(registry, before);
    assert_eq!(
        manager
            .complete_renewal_receipt(60_000, stale_renewal_receipt)
            .unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::StaleNoop
    );

    let stale_release_receipt = registry.relinquish_managed_exact_both(stale_permit, 60_000);
    assert_eq!(registry, before);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(stale_release_receipt),
        OutputLeaseKeepaliveRelinquishCompletion::StaleNoop
    );
}

#[test]
fn arm_requires_idle_and_preserves_the_current_authority() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, _) = arm(&registry, &receipt, 1);
    assert!(matches!(
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt, 1)),
        OutputLeaseKeepaliveArmOutcome::AlreadyArmed(_)
    ));
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Armed);
}

#[test]
fn renewal_expiry_then_recovered_b_requires_a_physical_stop_and_opaque_proof_before_b_arms() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(10_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);

    // The due A renewal observes real authority expiry. The managed CAS
    // publishes the orphaned successor (generation 2) and returns the
    // manager-owned physical fail-stop obligation for A; it does not release
    // or mutate a future B.
    let renewal = manager.prepare_due_renewal_for_run(run_a, 20_000).unwrap();
    let renewal_receipt = registry.renew_managed_exact_both(renewal, 20_000);
    let renewal_plan = match manager
        .complete_renewal_receipt(20_000, renewal_receipt)
        .unwrap()
    {
        OutputLeaseKeepaliveRenewalCompletion::FailStop(plan) => plan,
        other => panic!("expired A must fault into a bound plan, got {other:?}"),
    };
    let orphaned = registry.lease_view(grant_a.lease_id).unwrap().snapshot;
    assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
    assert_eq!(orphaned.generation, 2);

    // A fresh explicit B Enable can recover only that exact same orphaned
    // authority (generation 3). It arrives while A is still Faulting, so the
    // manager returns the immutable A plan and retains B opaque/unarmed.
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 20_000, 60_000);
    assert_eq!(grant_b.lease_id, grant_a.lease_id);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(20_000, evidence(&registry, &receipt_b, 2))
    else {
        panic!("recovered B must remain deferred behind Faulting A")
    };
    let action = conflict.into_action();
    let OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(resume_a_plan) = action else {
        panic!("Faulting A must expose only its existing immutable plan")
    };
    assert_eq!(resume_a_plan.lease().lease_id(), grant_a.lease_id.encode());
    assert_eq!(
        manager.observe_clock_for_run(run_a, 20_000),
        Err(OutputLeaseKeepaliveError::NotArmed)
    );
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, resume_a_plan);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    // The first immutable A plan was never executed. The resume pass consumed
    // the shared gate, so an interrupted A adapter cannot physically touch B
    // after recovery/arm either.
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, renewal_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());

    // A subsequent B hook reports the explicit proof action rather than
    // spawning B. The manager retained B itself and mints exactly one proof
    // capability only after the physical A stop is complete.
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(20_000, evidence(&registry, &receipt_b, 2))
    else {
        panic!("Faulted A must still require a B supersession proof")
    };
    let action = conflict.into_action();
    assert_eq!(
        action,
        OutputLeaseKeepaliveManagedConflictAction::PrepareDeferredSupersession
    );
    let supersession = manager
        .prepare_deferred_supersession()
        .expect("successful A stop and newer B must issue one proof capability");

    // The proof observes B as the exact current durable authority and A as a
    // stale generation. It does not perform an A relinquish CAS or mutate B's
    // registry snapshot/audit/index truth.
    let before_proof = registry.clone();
    let proof = registry.prove_managed_exact_both_supersession(supersession, 20_000);
    assert_eq!(registry, before_proof);
    let run_b = match manager.complete_deferred_supersession_receipt(20_000, proof) {
        OutputLeaseKeepaliveSupersessionCompletion::Armed(run) => run,
        outcome => panic!("only a matching opaque proof may arm B, got {outcome:?}"),
    };
    assert_ne!(run_a, run_b);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Armed);
    let armed_b = manager.snapshot().armed.expect("proof installed B only");
    assert_eq!(armed_b.run_id(), run_b);
    assert_eq!(armed_b.lease().lease_id(), grant_b.lease_id.encode());
    assert_eq!(armed_b.lease().generation(), 3);

    // B now has a normal independent renewal path. This proves the recovery
    // was not a permanent A Faulted strand and no stale A CAS was needed.
    let b_renewal = manager.prepare_due_renewal_for_run(run_b, 40_000).unwrap();
    let b_renewal_receipt = registry.renew_managed_exact_both(b_renewal, 40_000);
    assert_eq!(
        manager
            .complete_renewal_receipt(40_000, b_renewal_receipt)
            .unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::Renewed
    );
    assert_eq!(
        registry
            .lease_view(grant_b.lease_id)
            .unwrap()
            .snapshot
            .generation,
        4
    );
}

#[test]
fn rejected_deferred_b_proof_keeps_a_faulted_and_cannot_mutate_b() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .expect("A must enter a bound physical retirement before B can defer");

    // An external authority transition makes a newer B durable receipt
    // current while A's captured physical retirement still owns the manager.
    registry
        .relinquish_output_lease(grant_a.lease_id, &owner(), 1, 1)
        .unwrap();
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt_b, 2))
    else {
        panic!("B must remain deferred while A is Faulting")
    };
    let action = conflict.into_action();
    let OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(resume) = action else {
        panic!("Faulting A must expose only its current plan")
    };
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, resume);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    let capability = manager
        .prepare_deferred_supersession()
        .expect("only a successful completed A may issue the proof capability");

    // Remove B after the capability was issued. The opaque registry proof
    // must reject without a second mutation and cannot turn Faulted A into
    // Idle/Armed on a missing B authority.
    registry
        .relinquish_output_lease(grant_b.lease_id, &owner(), 1, 2)
        .unwrap();
    let before_proof = registry.clone();
    let proof = registry.prove_managed_exact_both_supersession(capability, 2);
    assert_eq!(registry, before_proof);
    assert_eq!(
        manager.complete_deferred_supersession_receipt(2, proof),
        OutputLeaseKeepaliveSupersessionCompletion::RetainedFaulted
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    // The unexecuted original A plan shares the completed gate and cannot
    // reopen physical ports after the rejected proof.
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());
}

#[test]
fn a_permit_then_newer_b_cas_race_retains_b_and_arms_it_without_user_retry() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .expect("A must enter a canonical stop before its automatic permit");
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    let permit_a = manager
        .permit_relinquish_after_successful_boundary(run_a, &identity(1))
        .expect("completed A owns one outbound exact release CAS");

    // The permit has left the manager lock. A newer B becomes registry truth
    // before that old CAS returns; B must be retained by the manager even
    // though the only admissible immediate action is to await the A receipt.
    registry
        .relinquish_output_lease(grant_a.lease_id, &owner(), 1, 1)
        .unwrap();
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt_b, 2))
    else {
        panic!("B must remain manager-retained while A CAS is outstanding")
    };
    assert_eq!(
        conflict.into_action(),
        OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease
    );

    // A's exact generation is stale against B. The failed CAS does not change
    // B, clears only the old permit state, and leaves retained B immediately
    // eligible for its opaque proof—no second user Enable is required.
    let before_old_a_cas = registry.clone();
    let old_a_receipt = registry.relinquish_managed_exact_both(permit_a, 2);
    assert_eq!(registry, before_old_a_cas);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(old_a_receipt),
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    let proof_capability = manager
        .prepare_deferred_supersession()
        .expect("the retained B must proceed automatically after stale A CAS");
    let proof = registry.prove_managed_exact_both_supersession(proof_capability, 2);
    let run_b = match manager.complete_deferred_supersession_receipt(2, proof) {
        OutputLeaseKeepaliveSupersessionCompletion::Armed(run) => run,
        outcome => panic!("retained B must arm after a successful proof, got {outcome:?}"),
    };
    assert_eq!(manager.snapshot().armed.unwrap().run_id(), run_b);
    assert_eq!(
        manager.snapshot().armed.unwrap().lease().lease_id(),
        grant_b.lease_id.encode()
    );
}

#[test]
fn later_c_replaces_deferred_b_and_a_late_b_proof_cannot_arm_after_c() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let old_a_plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .expect("A must first enter Faulting");

    registry
        .relinquish_output_lease(grant_a.lease_id, &owner(), 1, 1)
        .unwrap();
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict_b) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt_b, 2))
    else {
        panic!("B must defer behind Faulting A")
    };
    let OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(resume_a) =
        conflict_b.into_action()
    else {
        panic!("Faulting A must expose its current stop plan")
    };
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, resume_a);
    let stale_b_evidence = evidence(&registry, &receipt_b, 2);
    let b_capability = manager
        .prepare_deferred_supersession()
        .expect("B is the first retained deferred candidate");
    assert_eq!(
        manager.prepare_deferred_supersession(),
        Err(OutputLeaseKeepaliveError::UnexpectedState)
    );
    let b_proof = registry.prove_managed_exact_both_supersession(b_capability, 1);

    // B was current at proof time. Before the delayed receipt commits, a
    // later durable C becomes exact registry truth. C replaces retained B and
    // advances the manager high-water while B's out-of-lock receipt remains
    // incapable of arm completion.
    registry
        .relinquish_output_lease(grant_b.lease_id, &owner(), 1, 2)
        .unwrap();
    let (grant_c, receipt_c) = enable_exact_both(&mut registry, 3, 2, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict_c) =
        manager.arm_or_return_bound_fail_stop_plan(2, evidence(&registry, &receipt_c, 3))
    else {
        panic!("later C must replace B while its proof is in flight")
    };
    assert_eq!(
        conflict_c.into_action(),
        OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease
    );
    assert_eq!(
        manager.complete_deferred_supersession_receipt(2, b_proof),
        OutputLeaseKeepaliveSupersessionCompletion::StaleNoop
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    assert_eq!(
        manager.arm_or_return_bound_fail_stop_plan(2, stale_b_evidence),
        OutputLeaseKeepaliveArmOutcome::StaleNoop
    );

    // Only the retained latest C can now receive a fresh proof and arm. The
    // original A plan still loses its one-shot gate and makes no port calls.
    let c_capability = manager
        .prepare_deferred_supersession()
        .expect("C is the sole latest deferred proof candidate");
    let c_proof = registry.prove_managed_exact_both_supersession(c_capability, 2);
    let run_c = match manager.complete_deferred_supersession_receipt(2, c_proof) {
        OutputLeaseKeepaliveSupersessionCompletion::Armed(run) => run,
        outcome => panic!("only current C may arm, got {outcome:?}"),
    };
    assert_eq!(manager.snapshot().armed.unwrap().run_id(), run_c);
    assert_eq!(
        manager.snapshot().armed.unwrap().lease().lease_id(),
        grant_c.lease_id.encode()
    );
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, old_a_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());
}

#[test]
fn newer_b_evidence_cannot_make_an_unexecuted_faulting_a_plan_touch_b_or_mint_a_release() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let unexecuted_a_plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .expect("A must first be retained as a manager-owned Faulting run");

    // Simulate an external durable B authority while A's original adapter was
    // interrupted before it ever claimed the captured physical plan.
    registry
        .relinquish_output_lease(grant_a.lease_id, &owner(), 1, 1)
        .unwrap();
    let (_grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt_b, 2))
    else {
        panic!("newer B must remain deferred behind Faulting A")
    };
    let action = conflict.into_action();
    let OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(resume_a_plan) = action else {
        panic!("Faulting A must expose only its same immutable resume plan")
    };
    let mut resumed_ports = Ports::default();
    execute_current_plan(&mut manager, &mut resumed_ports, resume_a_plan);

    // `unexecuted_a_plan` is still an object the interrupted adapter could
    // hold, but B evidence has arrived and the shared A gate was completed by
    // the sole resumed pass. It cannot make a single physical port call.
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, unexecuted_a_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());

    // B's registry record is already current. The manager retained B, so it
    // must not mint an old-A registry release permit that could race B. Only
    // the opaque supersession proof is admissible from this state.
    assert!(manager
        .permit_relinquish_after_successful_boundary(run_a, &identity(1))
        .is_none());
    assert!(manager.prepare_deferred_supersession().is_ok());
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn adapter_abort_before_registry_fences_the_exact_renewal_and_preserves_registry_truth() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let before_registry = registry.clone();

    let plan = match manager.abort_due_renewal_before_registry(
        capability,
        OutputLeaseKeepalivePortError::detail("registry lock poisoned before renewal").unwrap(),
    ) {
        OutputLeaseKeepaliveRenewalCompletion::FailStop(plan) => plan,
        outcome => panic!("matching adapter abort must fail-stop the current run, got {outcome:?}"),
    };
    assert_eq!(registry, before_registry);
    assert_eq!(plan.run_id(), run);
    assert_eq!(plan.boundary(), OutputLeaseKeepaliveBoundary::RenewalFailed);
    assert_eq!(
        manager.snapshot().state,
        OutputLeaseKeepaliveState::Faulting
    );

    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn adapter_abort_before_registry_relinquish_retains_faulted_then_allows_one_exact_retry() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    let permit = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        outcome => panic!("completed Relinquish run must issue one permit, got {outcome:?}"),
    };
    let before_registry = registry.clone();

    assert_eq!(
        manager.abort_relinquish_before_registry(
            permit,
            OutputLeaseKeepalivePortError::detail("registry lock poisoned before relinquish")
                .unwrap(),
        ),
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    );
    assert_eq!(registry, before_registry);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    let retry = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        outcome => panic!("aborted permit must allow one exact retry, got {outcome:?}"),
    };
    complete_exact_relinquish(&mut registry, &mut manager, retry, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
}

#[test]
fn adapter_abort_before_registry_supersession_keeps_latest_b_for_a_fresh_proof() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    registry
        .relinquish_output_lease(grant_a.lease_id, &owner(), 1, 1)
        .unwrap();
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 1, 60_000);
    let OutputLeaseKeepaliveArmOutcome::ManagedConflict(conflict) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt_b, 2))
    else {
        panic!("newer B must remain deferred while A stops")
    };
    let OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(resume) = conflict.into_action()
    else {
        panic!("Faulting A must expose its existing plan")
    };
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, resume);
    let capability = manager.prepare_deferred_supersession().unwrap();
    let before_registry = registry.clone();

    assert_eq!(
        manager.abort_deferred_supersession_before_registry(
            capability,
            OutputLeaseKeepalivePortError::detail("registry lock poisoned before proof").unwrap(),
        ),
        OutputLeaseKeepaliveSupersessionCompletion::RetainedFaulted
    );
    assert_eq!(registry, before_registry);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    let retry = manager
        .prepare_deferred_supersession()
        .expect("aborted pre-CAS proof must permit a fresh proof for retained B");
    let proof = registry.prove_managed_exact_both_supersession(retry, 1);
    let run_b = match manager.complete_deferred_supersession_receipt(1, proof) {
        OutputLeaseKeepaliveSupersessionCompletion::Armed(run) => run,
        outcome => panic!("fresh proof must arm retained B, got {outcome:?}"),
    };
    assert_eq!(manager.snapshot().armed.unwrap().run_id(), run_b);
    assert_eq!(
        manager.snapshot().armed.unwrap().lease().lease_id(),
        grant_b.lease_id.encode()
    );

    // The originally captured physical A plan and the aborted B capability
    // cannot be replayed; A's shared execution gate was consumed by `resume`.
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());
}

#[test]
fn worker_panic_or_early_exit_never_leaves_the_exact_run_armed() {
    for detail in ["worker panicked", "worker exited unexpectedly"] {
        let (registry, _grant, receipt) = registry_with_grant(60_000);
        let (mut manager, run) = arm(&registry, &receipt, 1);
        let plan = manager
            .record_worker_termination(run, OutputLeaseKeepalivePortError::detail(detail).unwrap())
            .expect("an exact Armed worker termination must fence the run");
        assert_eq!(
            plan.boundary(),
            OutputLeaseKeepaliveBoundary::WorkerTerminated
        );
        assert_eq!(
            manager.snapshot().state,
            OutputLeaseKeepaliveState::Faulting
        );

        let mut ports = Ports::default();
        execute_current_plan(&mut manager, &mut ports, plan);
        assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
        assert!(manager
            .record_worker_termination(run, OutputLeaseKeepalivePortError::detail(detail).unwrap(),)
            .is_none());
    }
}

#[test]
fn durable_enable_arm_failure_returns_a_bound_fail_stop_without_idle_preflight() {
    let (mut registry, _grant, receipt) = registry_with_grant(60_000);
    let first = evidence(&registry, &receipt, 1);
    let replay = evidence(&registry, &receipt, 1);
    let mut manager = OutputLeaseKeepaliveManager::default();

    let OutputLeaseKeepaliveArmOutcome::FailStop(plan) =
        manager.arm_or_return_bound_fail_stop_plan(u64::MAX, first)
    else {
        panic!("clock-overflow admission must fail-stop instead of returning a bare error")
    };
    assert!(plan.is_admission_failure());
    assert_eq!(
        plan.boundary(),
        OutputLeaseKeepaliveBoundary::ArmAdmissionFailed
    );
    assert!(plan.initial_error().is_some());
    assert_eq!(
        manager.snapshot().state,
        OutputLeaseKeepaliveState::Faulting
    );
    let run = plan.run_id();

    let mut ports = Ports::default();
    let OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(execution) =
        execute_fail_stop_plan(&mut ports, plan)
    else {
        panic!("bound admission failure plan must execute")
    };
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
    assert_eq!(
        manager.complete_fail_stop(execution).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
    let permit = manager
        .permit_relinquish_after_successful_boundary(run, &identity(1))
        .expect("successful admission fail-stop must reach exact registry release");
    complete_exact_relinquish(&mut registry, &mut manager, permit, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert_eq!(
        manager.arm_or_return_bound_fail_stop_plan(0, replay),
        OutputLeaseKeepaliveArmOutcome::StaleNoop
    );
}

#[test]
fn duplicate_durable_enable_while_faulting_resumes_the_captured_plan_after_adapter_interrupt() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let captured = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .expect("A must enter Faulting before an adapter can be interrupted");

    // The first adapter dropped `captured` before any physical port call. A
    // duplicate durable hook must make the same current plan resumable, not
    // leave the run Faulting forever and not spawn another worker.
    let OutputLeaseKeepaliveArmOutcome::ResumeFailStop(resume) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt, 1))
    else {
        panic!("matching Faulting durable evidence must resume the current plan")
    };
    assert_eq!(resume.run_id(), run);
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, resume);
    assert!(matches!(
        execute_fail_stop_plan(&mut ports, captured),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert_eq!(ports.calls.len(), 6);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn matching_faulted_durable_enable_requires_exact_release_not_direct_rearm() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);

    let OutputLeaseKeepaliveArmOutcome::RequireCurrentRelinquish(permit) =
        manager.arm_or_return_bound_fail_stop_plan(1, evidence(&registry, &receipt, 1))
    else {
        panic!("matching Faulted durable evidence must require the current CAS release")
    };
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    complete_exact_relinquish(&mut registry, &mut manager, permit, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert_eq!(
        registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
        OutputLeasePhase::Unclaimed
    );
}

#[test]
fn rejected_relinquish_receipt_reopens_only_a_matching_faulted_run_for_retry() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    let first_permit = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        other => panic!("successful current fault must issue one permit, got {other:?}"),
    };

    // A rollback error is a CAS rejection with no registry mutation. Its
    // opaque receipt alone may reopen the exact current Faulted run once.
    registry.last_now_ms = 2;
    let rejected = registry.relinquish_managed_exact_both(first_permit, 1);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(rejected),
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    assert_eq!(
        registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
        OutputLeasePhase::HeldActive
    );
    let retry_permit = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        other => {
            panic!("matching rejected receipt must permit one serialized retry, got {other:?}")
        }
    };
    complete_exact_relinquish(&mut registry, &mut manager, retry_permit, 2);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
}

#[test]
fn old_a_plan_and_renewal_capability_cannot_touch_b_after_exact_a_release() {
    let (mut registry, grant_a, receipt_a) = registry_with_grant(60_000);
    let (mut manager, run_a) = arm(&registry, &receipt_a, 1);
    let old_a_renewal = manager.prepare_due_renewal_for_run(run_a, 20_000).unwrap();
    let old_a_plan = manager
        .begin_fail_stop_for_run(run_a, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let current_a_plan = manager.resume_fail_stop_for_run(run_a).unwrap();
    let mut current_ports = Ports::default();
    execute_current_plan(&mut manager, &mut current_ports, current_a_plan);
    let permit_a = match manager.manual_operation_decision(
        &grant_a.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        other => panic!("completed A must issue its exact release permit, got {other:?}"),
    };
    complete_exact_relinquish(&mut registry, &mut manager, permit_a, 20_000);

    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 20_000, 60_000);
    let run_b = match manager
        .arm_or_return_bound_fail_stop_plan(20_000, evidence(&registry, &receipt_b, 2))
    {
        OutputLeaseKeepaliveArmOutcome::Armed(run) => run,
        other => panic!("B may arm only after A's exact release, got {other:?}"),
    };
    let before_b = registry.clone();
    let stale_a_renewal = registry.renew_managed_exact_both(old_a_renewal, 20_000);
    assert_eq!(
        manager
            .complete_renewal_receipt(20_000, stale_a_renewal)
            .unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::StaleNoop
    );
    assert_eq!(registry, before_b);
    assert_eq!(manager.snapshot().armed.unwrap().run_id(), run_b);
    assert_eq!(
        manager.snapshot().armed.unwrap().lease().lease_id(),
        grant_b.lease_id.encode()
    );

    // The old plan object itself was never executed. The sibling resume plan
    // consumed its shared gate before B armed, so A can no longer touch any
    // physical port after the ABA transition.
    let mut stale_ports = Ports::default();
    assert!(matches!(
        execute_fail_stop_plan(&mut stale_ports, old_a_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert!(stale_ports.calls.is_empty());
}

#[test]
fn prepare_due_renewal_is_the_only_capability_issuer() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    assert_eq!(
        manager.prepare_due_renewal_for_run(run, 19_999),
        Err(OutputLeaseKeepaliveError::UnexpectedState)
    );
    let _capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    assert_eq!(
        manager.prepare_due_renewal_for_run(run, 20_000),
        Err(OutputLeaseKeepaliveError::RenewalAlreadyInFlight)
    );
}

#[test]
fn invalidated_renewal_capability_cannot_mutate_the_registry_after_fail_stop() {
    let (mut registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    let before = registry.clone();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    assert_eq!(
        manager.complete_renewal_receipt(20_000, receipt).unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::StaleNoop
    );
    assert_eq!(registry, before);
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
}

#[test]
fn registry_consumes_one_due_capability_and_receipt_completes_it() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    assert_eq!(
        manager.complete_renewal_receipt(20_000, receipt).unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::Renewed
    );
    assert_eq!(
        registry
            .lease_view(grant.lease_id)
            .unwrap()
            .snapshot
            .generation,
        2
    );
}

#[test]
fn expired_registry_renewal_returns_a_receipt_that_faults_the_same_run() {
    let (mut registry, grant, receipt) = registry_with_grant(10_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    let plan = match manager.complete_renewal_receipt(20_000, receipt).unwrap() {
        OutputLeaseKeepaliveRenewalCompletion::FailStop(plan) => plan,
        other => panic!("expiry must return a fail-stop plan, got {other:?}"),
    };
    assert_eq!(
        registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
        OutputLeasePhase::HeldOrphaned
    );
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
}

#[test]
fn fail_stop_executes_s0_before_retirement_in_canonical_order() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    assert_eq!(
        ports.calls,
        ["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
}

#[test]
fn stale_fail_stop_plan_loses_the_execution_claim_without_port_calls() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let old = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let current = manager.resume_fail_stop_for_run(run).unwrap();
    let mut ports = Ports::default();
    let OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(execution) =
        execute_fail_stop_plan(&mut ports, current)
    else {
        panic!("fresh plan must execute");
    };
    assert!(matches!(
        execute_fail_stop_plan(&mut ports, old),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    ));
    assert_eq!(ports.calls.len(), 6);
    assert_eq!(
        manager.complete_fail_stop(execution).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
}

#[test]
fn failed_fail_stop_retries_only_the_same_run_with_a_new_claim() {
    let (registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut failing = Ports {
        fail: Some("dmx"),
        ..Ports::default()
    };
    execute_current_plan(&mut manager, &mut failing, plan);
    let retry = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::RequireFailStop(plan) => plan,
        other => panic!("failed report must retry same run, got {other:?}"),
    };
    let mut recovered = Ports::default();
    execute_current_plan(&mut manager, &mut recovered, retry);
    assert!(matches!(
        manager.manual_operation_decision(
            &grant.lease_id.encode(),
            OutputLeaseKeepaliveManualOperation::Relinquish,
        ),
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(_)
    ));
}

#[test]
fn manual_renew_recover_and_transfer_are_rejected_for_managed_faults() {
    let (registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    for operation in [
        OutputLeaseKeepaliveManualOperation::Renew,
        OutputLeaseKeepaliveManualOperation::Recover,
        OutputLeaseKeepaliveManualOperation::ForceTransfer,
    ] {
        assert_eq!(
            manager.manual_operation_decision(&grant.lease_id.encode(), operation),
            OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
        );
    }
    let _ = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    assert_eq!(
        manager.manual_operation_decision(
            &grant.lease_id.encode(),
            OutputLeaseKeepaliveManualOperation::Renew,
        ),
        OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
    );
}

#[test]
fn automatic_boundary_requires_its_own_one_shot_exact_relinquish_permit() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Standby)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);

    // The manual Relinquish route cannot launder an automatic Standby fault.
    assert_eq!(
        manager.manual_operation_decision(
            &grant.lease_id.encode(),
            OutputLeaseKeepaliveManualOperation::Relinquish,
        ),
        OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
    );
    assert!(manager
        .permit_relinquish_after_successful_boundary(run, &identity(99))
        .is_none());
    let permit = manager
        .permit_relinquish_after_successful_boundary(run, &identity(1))
        .expect("exact automatic boundary may issue one permit after zero-failure stop");
    assert!(manager.admits_relinquish_permit(&permit));
    // A second permit cannot be minted while the first is outstanding.
    assert!(manager
        .permit_relinquish_after_successful_boundary(run, &identity(1))
        .is_none());
    complete_exact_relinquish(&mut registry, &mut manager, permit, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert!(manager
        .permit_relinquish_after_successful_boundary(run, &identity(1))
        .is_none());
}

#[test]
fn relinquish_requires_registry_success_before_faulted_run_becomes_idle() {
    let (mut registry, grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    let permit = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        other => panic!("expected permit, got {other:?}"),
    };
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    complete_exact_relinquish(&mut registry, &mut manager, permit, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
}

#[test]
fn relinquish_cas_expiry_leaves_faulted_and_publishes_orphan_truth() {
    let (mut registry, grant, receipt) = registry_with_grant(1);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let plan = manager
        .begin_fail_stop_for_run(run, OutputLeaseKeepaliveBoundary::Relinquish)
        .unwrap();
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    let permit = match manager.manual_operation_decision(
        &grant.lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        other => panic!("expected permit, got {other:?}"),
    };
    let receipt = registry.relinquish_managed_exact_both(permit, 1);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(receipt),
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    assert_eq!(
        registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
        OutputLeasePhase::HeldOrphaned
    );
}

#[test]
fn identity_change_fences_only_the_captured_run() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    let changed = OutputLeaseKeepaliveIdentity::new(
        PROJECT,
        "local-ui",
        "other-window",
        "session:91",
        "request:1",
    )
    .unwrap();
    let plan = manager.reconcile_identity(&changed).unwrap();
    assert_eq!(plan.run_id(), run);
    let mut ports = Ports::default();
    execute_current_plan(&mut manager, &mut ports, plan);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn clock_and_generation_overflow_fail_closed() {
    let (registry, _grant, receipt) = registry_with_grant(60_000);
    let (mut manager, run) = arm(&registry, &receipt, 1);
    assert_eq!(
        manager.observe_clock_for_run(run, 19_999),
        Ok(OutputLeaseKeepaliveTimerAction::Sleep(
            OutputLeaseKeepaliveWait {
                run_id: run,
                deadline_ms: 20_000,
            }
        ))
    );
    assert_eq!(
        manager.observe_clock_for_run(run, 19_998),
        Err(OutputLeaseKeepaliveError::ClockRollback)
    );
    assert!(
        OutputLeaseKeepaliveExactBothLease::from_verified_exact_both(
            "lease-ffffffffffffffff",
            u64::MAX
        )
        .is_ok()
    );
}

#[test]
fn stored_identity_and_error_details_enforce_utf8_byte_bounds_without_truncation() {
    assert_eq!(
        OutputLeaseKeepaliveIdentity::new("x".repeat(257), "owner", "main", "session", "request"),
        Err(OutputLeaseKeepaliveError::InvalidIdentity)
    );
    assert_eq!(
        OutputLeaseKeepaliveFailureDetail::new("x".repeat(257)),
        Err(OutputLeaseKeepaliveError::InvalidErrorDetail)
    );
}
