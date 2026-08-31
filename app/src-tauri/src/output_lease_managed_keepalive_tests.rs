use super::output_lease_keepalive::{
    execute_fail_stop_plan, OutputLeaseKeepaliveArmOutcome, OutputLeaseKeepaliveCleanupOnlyOutcome,
    OutputLeaseKeepaliveFailStopCompletion, OutputLeaseKeepaliveFailStopExecutionOutcome,
    OutputLeaseKeepaliveFailStopPlan, OutputLeaseKeepaliveFailStopPorts,
    OutputLeaseKeepaliveIdentity, OutputLeaseKeepaliveManager, OutputLeaseKeepaliveManualOperation,
    OutputLeaseKeepaliveManualOperationDecision, OutputLeaseKeepalivePortError,
    OutputLeaseKeepaliveRelinquishCompletion, OutputLeaseKeepaliveRenewalCompletion,
    OutputLeaseKeepaliveState, OutputLeaseKeepaliveSupersessionCompletion,
};
use super::*;

const PROCESS: u64 = 71;
const PROJECT: &str = "project_epoch:701";

fn owner(incarnation: u64) -> OutputLeaseOwner {
    OutputLeaseOwner::new("local-ui", "main", PROCESS, incarnation).unwrap()
}

fn both() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting, OutputLeaseResource::Video]).unwrap()
}

fn lighting() -> OutputLeaseResources {
    OutputLeaseResources::new(&[OutputLeaseResource::Lighting]).unwrap()
}

fn exact_both_enable_request(request_id: u64, ttl_ms: u64) -> OutputLeaseRequest {
    OutputLeaseRequest::from_action(
        "local-ui",
        "output-control",
        request_id,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner: owner(1),
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

fn active_registry(
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
) -> super::output_lease_keepalive::OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence {
    OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
        request,
        receipt,
        cleanup_identity(request, receipt),
    )
    .unwrap()
}

#[test]
fn normal_and_cleanup_only_evidence_reject_arbitrary_session_and_request_correlations() {
    let (registry, _grant, receipt) = active_registry(60_000);
    let request = exact_both_enable_request(1, 60_000);
    let before = registry.clone();

    for invalid_identity in [
        OutputLeaseKeepaliveIdentity::new(
            PROJECT,
            "local-ui",
            "main",
            "process:71;owner:999",
            "principal:local-ui;domain:output-control;request:1",
        )
        .unwrap(),
        OutputLeaseKeepaliveIdentity::new(
            PROJECT,
            "local-ui",
            "main",
            "process:71;owner:1",
            "principal:local-ui;domain:output-control;request:999",
        )
        .unwrap(),
    ] {
        assert!(
            registry
                .issue_managed_exact_both_durable_enable_evidence(
                    &receipt,
                    invalid_identity.clone(),
                )
                .is_err(),
            "normal arm must bind canonical session and request correlation"
        );
        assert!(
            OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
                &request,
                &receipt,
                invalid_identity,
            )
            .is_err(),
            "cleanup-only must reject the identical arbitrary identity values"
        );
    }
    assert_eq!(
        registry, before,
        "rejected identity evidence cannot mutate registry truth"
    );
}

fn arm_manager(
    registry: &OutputLeaseRegistry,
    receipt: &OutputLeaseRequestReceipt,
) -> (
    OutputLeaseKeepaliveManager,
    super::output_lease_keepalive::OutputLeaseKeepaliveRunId,
) {
    let evidence = registry
        .issue_managed_exact_both_durable_enable_evidence(receipt, identity())
        .unwrap();
    let mut manager = OutputLeaseKeepaliveManager::default();
    let run = match manager.arm_or_return_bound_fail_stop_plan(0, evidence) {
        OutputLeaseKeepaliveArmOutcome::Armed(run) => run,
        outcome => panic!("valid initial durable Enable must arm, got {outcome:?}"),
    };
    (manager, run)
}

#[derive(Default)]
struct Ports {
    calls: Vec<&'static str>,
}

impl Ports {
    fn call(&mut self, step: &'static str) -> Result<(), OutputLeaseKeepalivePortError> {
        self.calls.push(step);
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

fn execute_and_complete(
    manager: &mut OutputLeaseKeepaliveManager,
    ports: &mut Ports,
    plan: OutputLeaseKeepaliveFailStopPlan,
) {
    let OutputLeaseKeepaliveFailStopExecutionOutcome::Executed(execution) =
        execute_fail_stop_plan(ports, plan)
    else {
        panic!("current fail-stop plan must win the one-shot execution claim");
    };
    assert_eq!(
        manager.complete_fail_stop(execution).unwrap(),
        OutputLeaseKeepaliveFailStopCompletion::Completed
    );
}

fn exact_relinquish_permit(
    manager: &mut OutputLeaseKeepaliveManager,
    lease_id: OutputLeaseId,
) -> super::output_lease_keepalive::OutputLeaseKeepaliveRelinquishPermit {
    match manager.manual_operation_decision(
        &lease_id.encode(),
        OutputLeaseKeepaliveManualOperation::Relinquish,
    ) {
        OutputLeaseKeepaliveManualOperationDecision::AllowRelinquish(permit) => permit,
        decision => panic!("expected matching exact relinquish permit, got {decision:?}"),
    }
}

fn complete_exact_relinquish(
    registry: &mut OutputLeaseRegistry,
    manager: &mut OutputLeaseKeepaliveManager,
    permit: super::output_lease_keepalive::OutputLeaseKeepaliveRelinquishPermit,
    now_ms: u64,
) {
    let receipt = registry.relinquish_managed_exact_both(permit, now_ms);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(receipt),
        OutputLeaseKeepaliveRelinquishCompletion::Completed
    );
}

#[test]
fn cleanup_only_durable_enable_never_arms_and_releases_only_after_its_canonical_plan() {
    let (mut registry, grant, receipt) = active_registry(60_000);
    let request = exact_both_enable_request(1, 60_000);
    let evidence = cleanup_evidence(&request, &receipt);
    let mut manager = OutputLeaseKeepaliveManager::default();

    let plan = match manager.begin_cleanup_only_after_unadmitted_durable_enable(evidence) {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("cleanup-only durable Enable must create a fail-stop, got {outcome:?}"),
    };
    assert!(plan.is_admission_failure());
    assert_eq!(
        plan.initial_error(),
        Some("durable exact-Both Enable could not enter managed keepalive admission")
    );
    assert_eq!(
        manager.snapshot().state,
        OutputLeaseKeepaliveState::Faulting
    );
    assert!(manager.snapshot().armed.is_none());
    assert_eq!(
        manager.prepare_due_renewal_for_run(plan.run_id(), 20_000),
        Err(super::output_lease_keepalive::OutputLeaseKeepaliveError::NotArmed)
    );

    let run_id = plan.run_id();
    let identity = plan.identity().clone();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    assert_eq!(
        ports.calls,
        vec!["cancel", "s0", "fence", "dmx", "spout", "persist"]
    );
    let permit = manager
        .permit_relinquish_after_successful_boundary(run_id, &identity)
        .expect("a successful cleanup-only all-deny must issue one exact release permit");
    complete_exact_relinquish(&mut registry, &mut manager, permit, 1);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert_eq!(
        registry.lease_view(grant.lease_id).unwrap().snapshot.phase,
        OutputLeasePhase::Unclaimed
    );

    // The same durable issuance cannot recreate a cleanup plan after the
    // manager reached Idle; the monotonic high-water stays authoritative.
    assert_eq!(
        manager.begin_cleanup_only_after_unadmitted_durable_enable(cleanup_evidence(
            &request, &receipt
        )),
        OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop
    );
}

#[test]
fn cleanup_only_static_issuer_rejects_forged_request_receipt_and_identity_shapes() {
    let (registry, _grant, receipt) = active_registry(60_000);
    let request = exact_both_enable_request(1, 60_000);
    let before = registry.clone();

    let mut wrong_owner = receipt.clone();
    wrong_owner.owner = Some(owner(2));
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &wrong_owner,
            cleanup_identity(&request, &receipt),
        )
        .is_err()
    );

    let mut wrong_scope = receipt.clone();
    wrong_scope.resources = Some(lighting());
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &wrong_scope,
            cleanup_identity(&request, &receipt),
        )
        .is_err()
    );

    let mut wrong_audit = receipt.clone();
    wrong_audit.audit_sequence = 0;
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &wrong_audit,
            cleanup_identity(&request, &receipt),
        )
        .is_err()
    );

    let wrong_request = exact_both_enable_request(2, 60_000);
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &wrong_request,
            &receipt,
            cleanup_identity(&request, &receipt),
        )
        .is_err()
    );

    let wrong_identity = OutputLeaseKeepaliveIdentity::new(
        "project_epoch:forged",
        "local-ui",
        "main",
        format!("process:{};owner:1", receipt.process_session_incarnation),
        format!(
            "principal:{};domain:{};request:{}",
            request.key.principal, request.key.domain, request.key.request_id
        ),
    )
    .unwrap();
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &receipt,
            wrong_identity,
        )
        .is_err()
    );
    assert_eq!(registry, before);
}

#[test]
fn cleanup_only_release_revalidates_registry_after_normal_current_proof_failed() {
    let (mut registry, grant, receipt) = active_registry(60_000);
    let request = exact_both_enable_request(1, 60_000);
    assert!(registry.observe_expiry(grant.lease_id, 60_000).unwrap());
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt, identity())
        .is_err());

    let mut manager = OutputLeaseKeepaliveManager::default();
    let plan = match manager
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_evidence(&request, &receipt))
    {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("durable cleanup fallback must fence the route, got {outcome:?}"),
    };
    let run_id = plan.run_id();
    let identity = plan.identity().clone();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    let permit = manager
        .permit_relinquish_after_successful_boundary(run_id, &identity)
        .expect("physical cleanup still produces one exact registry retry permit");
    let receipt = registry.relinquish_managed_exact_both(permit, 60_000);
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
fn newer_cleanup_authority_supersedes_old_run_and_fences_late_plan_and_renewal() {
    let (mut registry, grant_a, receipt_a) = active_registry(60_000);
    let (mut manager, run_a) = arm_manager(&registry, &receipt_a);
    let old_renewal = manager.prepare_due_renewal_for_run(run_a, 20_000).unwrap();
    let old_plan = manager
        .begin_fail_stop_for_run(
            run_a,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();

    assert!(registry.observe_expiry(grant_a.lease_id, 60_000).unwrap());
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 60_000, 60_000);
    let request_b = exact_both_enable_request(2, 60_000);
    let cleanup_plan = match manager.begin_cleanup_only_after_unadmitted_durable_enable(
        cleanup_evidence(&request_b, &receipt_b),
    ) {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("new durable cleanup authority must supersede A, got {outcome:?}"),
    };
    assert_eq!(cleanup_plan.run_id(), run_a);
    assert_eq!(
        cleanup_plan.lease().generation(),
        receipt_b.generation_after.unwrap()
    );

    let mut old_ports = Ports::default();
    assert_eq!(
        execute_fail_stop_plan(&mut old_ports, old_plan),
        OutputLeaseKeepaliveFailStopExecutionOutcome::StaleNoop
    );
    assert!(old_ports.calls.is_empty());
    assert_eq!(
        manager
            .complete_renewal_receipt(
                60_000,
                registry.renew_managed_exact_both(old_renewal, 60_000),
            )
            .unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::StaleNoop
    );

    let identity = cleanup_plan.identity().clone();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, cleanup_plan);
    let permit = manager
        .permit_relinquish_after_successful_boundary(run_a, &identity)
        .expect("only B's cleanup fault may issue the final exact release permit");
    complete_exact_relinquish(&mut registry, &mut manager, permit, 60_000);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert_eq!(
        registry
            .lease_view(grant_b.lease_id)
            .unwrap()
            .snapshot
            .phase,
        OutputLeasePhase::Unclaimed
    );
}

#[test]
fn cleanup_invalidates_an_issued_deferred_supersession_before_registry_ingress() {
    let (mut registry, grant_a, receipt_a) = active_registry(60_000);
    let (mut manager, run_a) = arm_manager(&registry, &receipt_a);
    let plan_a = manager
        .begin_fail_stop_for_run(
            run_a,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();
    let mut ports_a = Ports::default();
    execute_and_complete(&mut manager, &mut ports_a, plan_a);

    assert!(registry.observe_expiry(grant_a.lease_id, 60_000).unwrap());
    let (grant_b, receipt_b) = enable_exact_both(&mut registry, 2, 60_000, 60_000);
    let evidence_b = registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt_b, identity_for_request(2))
        .unwrap();
    assert!(matches!(
        manager.arm_or_return_bound_fail_stop_plan(60_000, evidence_b),
        OutputLeaseKeepaliveArmOutcome::ManagedConflict(_)
    ));
    let late_b_proof = manager
        .prepare_deferred_supersession()
        .expect("completed A plus deferred B must issue one proof capability");

    assert!(registry.observe_expiry(grant_b.lease_id, 120_000).unwrap());
    let (grant_c, receipt_c) = enable_exact_both(&mut registry, 3, 120_000, 60_000);
    let request_c = exact_both_enable_request(3, 60_000);
    let cleanup_c = match manager.begin_cleanup_only_after_unadmitted_durable_enable(
        cleanup_evidence(&request_c, &receipt_c),
    ) {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("new durable C cleanup must replace A/B proof state, got {outcome:?}"),
    };
    assert_eq!(cleanup_c.run_id(), run_a);
    assert_eq!(
        cleanup_c.lease().generation(),
        receipt_c.generation_after.unwrap()
    );

    let before_late_proof = registry.clone();
    let late_b_receipt = registry.prove_managed_exact_both_supersession(late_b_proof, 120_000);
    assert!(matches!(
        &late_b_receipt,
        super::output_lease_keepalive::OutputLeaseKeepaliveSupersessionCasReceipt::Rejected(_)
    ));
    assert_eq!(
        registry, before_late_proof,
        "an invalidated proof capability cannot inspect or mutate registry truth"
    );
    assert_eq!(
        manager.complete_deferred_supersession_receipt(120_000, late_b_receipt),
        OutputLeaseKeepaliveSupersessionCompletion::StaleNoop
    );
    assert_eq!(
        manager.snapshot().state,
        OutputLeaseKeepaliveState::Faulting
    );
    assert_eq!(
        manager.snapshot().fault.unwrap().lease().lease_id(),
        grant_c.lease_id.encode()
    );
}

#[test]
fn managed_exact_both_renewal_advances_one_generation_and_deadline_only() {
    assert_eq!(OUTPUT_LEASE_MANAGED_EXACT_BOTH_TTL_MS, 60_000);
    let (mut registry, grant, receipt) = active_registry(60_000);
    let before = registry.lease_view(grant.lease_id).unwrap();
    let (mut manager, run) = arm_manager(&registry, &receipt);

    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    assert_eq!(
        manager.complete_renewal_receipt(20_000, receipt).unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::Renewed
    );

    let after = registry.lease_view(grant.lease_id).unwrap();
    assert_eq!(after.snapshot.phase, OutputLeasePhase::HeldActive);
    assert_eq!(after.snapshot.generation, 2);
    assert_eq!(after.snapshot.owner, before.snapshot.owner);
    assert_eq!(after.snapshot.resources, before.snapshot.resources);
    assert_eq!(after.snapshot.expires_at_monotonic_ms, Some(80_000));
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Armed);
}

#[test]
fn managed_exact_both_renewal_rejects_every_identity_or_scope_mismatch_atomically() {
    let (registry, grant, receipt) = active_registry(60_000);
    let before = registry.clone();
    let mut bad_owner = receipt.clone();
    bad_owner.owner = Some(owner(2));
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&bad_owner, identity())
        .is_err());
    let mut bad_scope = receipt.clone();
    bad_scope.resources = Some(lighting());
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&bad_scope, identity())
        .is_err());
    let mut bad_generation = receipt.clone();
    bad_generation.generation_after = Some(2);
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&bad_generation, identity())
        .is_err());
    let mut bad_lease = receipt.clone();
    bad_lease.lease_id = Some(OutputLeaseId(grant.lease_id.0.saturating_add(1)));
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&bad_lease, identity())
        .is_err());
    let wrong_project = OutputLeaseKeepaliveIdentity::new(
        "project_epoch:wrong",
        "local-ui",
        "main",
        "session:71",
        "request:1",
    )
    .unwrap();
    assert!(registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt, wrong_project,)
        .is_err());
    assert_eq!(registry, before);
}

#[test]
fn managed_exact_both_expiry_commits_orphan_truth_and_all_other_errors_are_noop() {
    let (mut registry, grant, receipt) = active_registry(10_000);
    let (mut manager, run) = arm_manager(&registry, &receipt);
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    let plan = match manager.complete_renewal_receipt(20_000, receipt).unwrap() {
        OutputLeaseKeepaliveRenewalCompletion::FailStop(plan) => plan,
        completion => panic!("expiry must fail-stop, got {completion:?}"),
    };
    let observed = registry.lease_view(grant.lease_id).unwrap().snapshot;
    assert_eq!(observed.phase, OutputLeasePhase::HeldOrphaned);
    assert_eq!(observed.generation, 2);
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn managed_exact_both_relinquish_releases_only_the_exact_active_or_orphaned_lease() {
    let (mut registry, grant, receipt) = active_registry(60_000);
    let (mut manager, run) = arm_manager(&registry, &receipt);
    let plan = manager
        .begin_fail_stop_for_run(
            run,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    let permit = exact_relinquish_permit(&mut manager, grant.lease_id);
    complete_exact_relinquish(&mut registry, &mut manager, permit, 20_000);
    let released = registry.lease_view(grant.lease_id).unwrap().snapshot;
    assert_eq!(released.phase, OutputLeasePhase::Unclaimed);
    assert_eq!(released.generation, 2);
    assert!(released.owner.is_none());
    assert!(released.resources.is_none());
    assert!(released.expires_at_monotonic_ms.is_none());
    assert!(registry
        .resource_index
        .values()
        .all(|ids| !ids.contains(&grant.lease_id)));
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
}

#[test]
fn managed_exact_both_successful_relinquish_receipt_exposes_only_wire_safe_response_evidence_before_completion(
) {
    let (mut registry, grant, durable_enable_receipt) = active_registry(60_000);
    let (mut manager, run) = arm_manager(&registry, &durable_enable_receipt);
    let plan = manager
        .begin_fail_stop_for_run(
            run,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    let permit = exact_relinquish_permit(&mut manager, grant.lease_id);

    let cas_receipt = registry.relinquish_managed_exact_both(permit, 20_000);
    let response = cas_receipt
        .exact_release_response_evidence()
        .expect("only exact registry success may expose a managed-release response");
    assert_eq!(response.lease_id(), grant.lease_id.encode());
    assert_eq!(response.prior_generation(), 1);
    assert_eq!(response.released_generation(), 2);
    assert_eq!(
        response.durable_enable_audit_sequence(),
        durable_enable_receipt.audit_sequence
    );
    assert!(response.durable_enable_audit_sequence() > 0);

    // Response evidence is detached, non-authorizing data. The opaque CAS
    // receipt still must be consumed by the manager before the route becomes
    // Idle; capturing this value alone cannot complete the transition.
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(cas_receipt),
        OutputLeaseKeepaliveRelinquishCompletion::Completed
    );
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Idle);
    assert_eq!(response.lease_id(), grant.lease_id.encode());
    assert_eq!(response.prior_generation(), 1);
    assert_eq!(response.released_generation(), 2);
}

#[test]
fn managed_exact_both_relinquish_publishes_expiry_truth_and_rejects_all_other_cas_misses_atomically(
) {
    let (mut registry, grant, receipt) = active_registry(10_000);
    let (mut manager, run) = arm_manager(&registry, &receipt);
    let plan = manager
        .begin_fail_stop_for_run(
            run,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    let permit = exact_relinquish_permit(&mut manager, grant.lease_id);
    let receipt = registry.relinquish_managed_exact_both(permit, 10_000);
    assert_eq!(
        manager.complete_relinquish_cas_receipt(receipt),
        OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry
    );
    let orphaned = registry.lease_view(grant.lease_id).unwrap().snapshot;
    assert_eq!(orphaned.phase, OutputLeasePhase::HeldOrphaned);
    assert_eq!(manager.snapshot().state, OutputLeaseKeepaliveState::Faulted);
}

#[test]
fn managed_exact_both_relinquish_bypasses_but_does_not_mutate_public_admission_state() {
    let (mut registry, grant, receipt) = active_registry(60_000);
    let (mut manager, run) = arm_manager(&registry, &receipt);
    let token_buckets = registry.token_buckets.clone();
    let lanes = registry.operation_lanes.clone();
    let receipts = registry.receipts.clone();
    let plan = manager
        .begin_fail_stop_for_run(
            run,
            super::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Relinquish,
        )
        .unwrap();
    let mut ports = Ports::default();
    execute_and_complete(&mut manager, &mut ports, plan);
    let permit = exact_relinquish_permit(&mut manager, grant.lease_id);
    complete_exact_relinquish(&mut registry, &mut manager, permit, 20_000);
    assert_eq!(registry.token_buckets, token_buckets);
    assert_eq!(registry.operation_lanes, lanes);
    assert_eq!(registry.receipts, receipts);
}

#[test]
fn managed_exact_both_renewal_bypasses_but_does_not_mutate_public_admission_state() {
    let (mut registry, _grant, receipt) = active_registry(60_000);
    let (mut manager, run) = arm_manager(&registry, &receipt);
    let token_buckets = registry.token_buckets.clone();
    let lanes = registry.operation_lanes.clone();
    let receipts = registry.receipts.clone();
    let capability = manager.prepare_due_renewal_for_run(run, 20_000).unwrap();
    let receipt = registry.renew_managed_exact_both(capability, 20_000);
    assert_eq!(
        manager.complete_renewal_receipt(20_000, receipt).unwrap(),
        OutputLeaseKeepaliveRenewalCompletion::Renewed
    );
    assert_eq!(registry.token_buckets, token_buckets);
    assert_eq!(registry.operation_lanes, lanes);
    assert_eq!(registry.receipts, receipts);
}
