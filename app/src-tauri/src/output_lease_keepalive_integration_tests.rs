use super::super::output_lease::{
    OutputLeaseOwner, OutputLeaseRegistry, OutputLeaseRequest, OutputLeaseRequestAction,
    OutputLeaseResource, OutputLeaseResources,
};
use super::super::output_lease_keepalive_runtime::{
    OutputLeaseKeepaliveRuntime, OutputLeaseKeepaliveRuntimeAdmission,
};

#[derive(Default)]
struct CanonicalPorts(Vec<&'static str>);

impl super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPorts
    for CanonicalPorts
{
    fn cancel_renewal(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("cancel");
        Ok(())
    }

    fn request_safety_blackout(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("safety");
        Ok(())
    }

    fn advance_failure_fence(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("fence");
        Ok(())
    }

    fn retire_dmx_usb_artnet(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("dmx-usb-artnet");
        Ok(())
    }

    fn retire_spout_native(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("spout-native");
        Ok(())
    }

    fn persist_standby_failed(
        &mut self,
        _: &super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
    ) -> Result<(), super::super::output_lease::output_lease_keepalive::OutputLeaseKeepalivePortError>
    {
        self.0.push("persist");
        Ok(())
    }
}
use super::{durable_enable_identity, require_exact_durable_terminal, DurableEnableIdentityInput};

fn exact_both_enable_fixture() -> (
    OutputLeaseRegistry,
    OutputLeaseRequest,
    super::super::output_lease::OutputLeaseRequestReceipt,
) {
    exact_both_enable_fixture_at(991, 10, 60_000)
}

fn exact_both_enable_fixture_at(
    request_id: u64,
    now_ms: u64,
    ttl_ms: u64,
) -> (
    OutputLeaseRegistry,
    OutputLeaseRequest,
    super::super::output_lease::OutputLeaseRequestReceipt,
) {
    let owner = OutputLeaseOwner::new("operator-a", "main", 3, 7).expect("fixture owner is valid");
    let request = OutputLeaseRequest::from_action(
        "operator-a",
        "output-control",
        request_id,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner,
            resources: OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Video,
            ])
            .expect("Both resources are valid"),
            project_identity: "project_epoch:11".to_string(),
            ttl_ms,
        },
    )
    .expect("fixture enable request is valid");
    let mut registry = OutputLeaseRegistry::fresh_process(3).expect("fixture registry is valid");
    let receipt = registry
        .submit_request(&request, now_ms)
        .expect("fixture registry submission completes");
    (registry, request, receipt)
}

#[test]
fn durable_enable_identity_binds_the_exact_registry_both_receipt() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity = durable_enable_identity(&input, &request, &receipt)
        .expect("integration identity is bounded and complete");

    assert_eq!(identity.project_identity(), "project_epoch:11");
    assert_eq!(identity.owner_identity(), "operator-a");
    assert_eq!(identity.window_label(), "main");
    assert_eq!(identity.session_identity(), "process:3;owner:7");
    assert_eq!(
        identity.request_correlation(),
        "principal:operator-a;domain:output-control;request:991"
    );
    registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
        .expect("only current exact Both authority may issue managed evidence");
}

#[test]
fn stale_project_identity_cannot_issue_evidence_or_arm_a_managed_run() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let stale_project = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 12,
    };
    let identity = durable_enable_identity(&stale_project, &request, &receipt)
        .expect("identity construction itself remains total");

    assert!(
        registry
            .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
            .is_err(),
        "a stale project identity must fail before runtime admission can arm"
    );
}

#[test]
fn journal_receipt_mismatch_is_rejected_before_registry_evidence_or_worker_arm() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let mut journal = super::super::OutputLeaseDurableReceiptJournal::in_memory();
    journal
        .record(&receipt)
        .expect("fixture journal records terminal receipt");
    let mut mismatched = receipt.clone();
    mismatched.audit_sequence = mismatched
        .audit_sequence
        .checked_add(1)
        .expect("fixture audit sequence remains in range");

    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity = durable_enable_identity(&input, &request, &mismatched)
        .expect("identity construction itself is not authority");
    assert!(
        runtime
            .admit_verified_durable_enable_with(11, || {
                require_exact_durable_terminal(&journal, &request, &mismatched)?;
                registry
                    .issue_managed_exact_both_durable_enable_evidence(&mismatched, identity)
                    .map_err(|error| format!("fixture evidence failed: {error:?}"))
            })
            .is_err(),
        "a non-exact durable receipt must fail before registry evidence or worker installation"
    );
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("failed evidence must leave runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle,
        "failed durable evidence must not arm a worker run"
    );
}

#[test]
fn exact_durable_replay_is_already_armed_not_a_second_run() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity =
        durable_enable_identity(&input, &request, &receipt).expect("fixture identity is valid");
    let first = runtime
        .admit_verified_durable_enable_with(11, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
                .map_err(|error| format!("fixture evidence failed: {error:?}"))
        })
        .expect("first durable receipt admits exactly one run");
    assert!(matches!(
        first,
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(_)
    ));

    let replay_identity = durable_enable_identity(&input, &request, &receipt)
        .expect("replay identity is deterministic");
    let replay = runtime
        .admit_verified_durable_enable_with(12, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt, replay_identity)
                .map_err(|error| format!("fixture replay evidence failed: {error:?}"))
        })
        .expect("exact replay remains valid evidence");
    assert!(matches!(
        replay,
        OutputLeaseKeepaliveRuntimeAdmission::AlreadyArmed(_)
    ));
}

#[test]
fn managed_non_relinquish_lifecycle_is_rejected_before_any_generic_request() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let lease_id = receipt
        .lease_id
        .expect("exact Both fixture receipt carries a lease")
        .encode();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity =
        durable_enable_identity(&input, &request, &receipt).expect("fixture identity is valid");
    runtime
        .admit_verified_durable_enable_with(11, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
                .map_err(|error| format!("fixture evidence failed: {error:?}"))
        })
        .expect("fixture managed run arms");

    assert!(runtime
        .manages_lease_id(&lease_id)
        .expect("managed lease preflight is pure"));
    assert!(matches!(
        runtime
            .manual_lifecycle_decision(
                &lease_id,
                super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveManualOperation::Renew,
            )
            .expect("managed lifecycle decision is available"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveManualOperationDecision::RejectManagedLease
    ));
}

#[test]
fn canonical_boundary_ports_precede_exact_release_and_wire_dto() {
    let (mut registry, request, receipt) = exact_both_enable_fixture();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity =
        durable_enable_identity(&input, &request, &receipt).expect("fixture identity is valid");
    let run_id = match runtime
        .admit_verified_durable_enable_with(11, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
                .map_err(|error| format!("fixture evidence failed: {error:?}"))
        })
        .expect("fixture managed run arms")
    {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run_id) => run_id,
        outcome => panic!("fixture must create a new run, got {outcome:?}"),
    };
    let plan = match runtime
        .begin_current_boundary(
            super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveBoundary::ProjectIdentityChanged,
        )
        .expect("boundary capture succeeds")
    {
        super::super::output_lease_keepalive_runtime::OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured(plan) => plan,
        outcome => panic!("fixture must capture its exact run, got {outcome:?}"),
    };
    assert_eq!(plan.run_id(), run_id);
    let plan_identity = plan.identity().clone();
    let mut ports = CanonicalPorts::default();
    assert!(matches!(
        runtime
            .execute_fail_stop(&mut ports, plan)
            .expect("canonical ports complete"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopCompletion::Completed
    ));
    assert_eq!(
        ports.0,
        vec![
            "cancel",
            "safety",
            "fence",
            "dmx-usb-artnet",
            "spout-native",
            "persist"
        ],
        "the exact CAS is forbidden until all six physical ports have completed"
    );
    let mut response_evidence = None;
    assert!(matches!(
        runtime
            .relinquish_after_successful_boundary(run_id, &plan_identity, |permit| {
                let cas = registry.relinquish_managed_exact_both(permit, 12);
                response_evidence = cas.exact_release_response_evidence();
                Ok(cas)
            })
            .expect("exact release bridge completes"),
        Some(super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::Completed)
    ));
    let lease_result = super::super::control_plane_runtime::output_control_lease_result_from_managed_exact_relinquish(
        &response_evidence.expect("only exact success exposes wire evidence"),
    )
    .expect("managed exact release DTO validates");
    assert_eq!(
        lease_result.outcome,
        protocol::control_plane_command::OutputLeaseReceiptOutcomeV2::Relinquished
    );
    assert_eq!(
        lease_result.phase,
        protocol::control_plane_command::OutputLeaseReceiptPhaseV2::Unclaimed
    );
    assert_eq!(lease_result.changes.len(), 1);
}

#[test]
fn newly_committed_enable_journal_failure_enters_all_deny_before_exact_release() {
    let (mut registry, request, receipt) = exact_both_enable_fixture();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };

    assert!(
        runtime
            .admit_verified_durable_enable_with(11, || {
                Err("injected durable journal failure after committed Enable".to_string())
            })
            .is_err(),
        "normal evidence failure is injected before any manager arm"
    );
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("failed normal evidence leaves runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle
    );

    let identity = durable_enable_identity(&input, &request, &receipt)
        .expect("committed receipt has a deterministic exact identity");
    let cleanup_evidence =
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request, &receipt, identity,
        )
        .expect(
            "new durable commit constructs cleanup-only evidence without registry or journal read",
        );
    let cleanup = runtime.begin_cleanup_only_after_unadmitted_durable_enable(cleanup_evidence);
    assert!(!cleanup.recovered_operation_serial_poison());
    assert!(!cleanup.recovered_manager_poison());
    let plan = match cleanup.into_outcome() {
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("new committed cleanup must issue an all-deny plan, got {outcome:?}"),
    };
    let run_id = plan.run_id();
    let identity = plan.identity().clone();
    let mut ports = CanonicalPorts::default();
    assert!(matches!(
        runtime
            .execute_fail_stop(&mut ports, plan)
            .expect("canonical cleanup ports complete"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopCompletion::Completed
    ));
    assert_eq!(
        ports.0,
        vec![
            "cancel",
            "safety",
            "fence",
            "dmx-usb-artnet",
            "spout-native",
            "persist"
        ],
        "exact registry release is forbidden until every canonical port has run"
    );
    assert!(matches!(
        runtime
            .relinquish_after_successful_boundary(run_id, &identity, |permit| {
                Ok(registry.relinquish_managed_exact_both(permit, 12))
            })
            .expect("all-deny completion issues its exact release bridge"),
        Some(super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::Completed)
    ));
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("exact release leaves manager readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle
    );
}

#[test]
fn post_permit_clock_failure_retains_cleanup_authority_for_exact_retry() {
    let (registry, request, receipt) = exact_both_enable_fixture();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity = durable_enable_identity(&input, &request, &receipt)
        .expect("committed receipt has a deterministic exact identity");
    let cleanup_evidence =
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request, &receipt, identity,
        )
        .expect("new durable commit constructs cleanup-only evidence");
    let plan = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_evidence)
        .into_outcome()
    {
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("new committed cleanup must issue an all-deny plan, got {outcome:?}"),
    };
    let mut ports = CanonicalPorts::default();
    runtime
        .execute_fail_stop(&mut ports, plan)
        .expect("canonical ports complete before release attempt");
    let registry_before = registry.clone();
    let retry_identity = durable_enable_identity(&input, &request, &receipt)
        .expect("retry has the same exact committed identity");
    let retry_evidence =
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &receipt,
            retry_identity,
        )
        .expect("matching Faulted cleanup can request its exact release permit");
    let permit = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(retry_evidence)
        .into_outcome()
    {
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish(permit) => permit,
        outcome => panic!("successful all-deny must issue one exact release permit, got {outcome:?}"),
    };
    assert!(matches!(
        runtime
            .complete_current_relinquish_with(permit, |permit| {
                Err::<
                    (
                        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCasReceipt,
                        (),
                    ),
                    _,
                >((
                    permit,
                    super::port_error("injected clock failure after permit issuance before registry CAS"),
                ))
            })
            .expect("post-permit abort is retained instead of dropping authority"),
        super::super::output_lease_keepalive_runtime::OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry
    ));
    let retry_again_identity =
        durable_enable_identity(&input, &request, &receipt).expect("retry identity remains exact");
    let retry_again_evidence =
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request,
            &receipt,
            retry_again_identity,
        )
        .expect("retry evidence remains valid after a pre-CAS abort");
    assert!(matches!(
        runtime
            .begin_cleanup_only_after_unadmitted_durable_enable(retry_again_evidence)
            .into_outcome(),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish(_)
    ), "the exact permit can be issued again after the post-permit clock failure");
    assert_eq!(
        registry, registry_before,
        "the injected pre-CAS failure did not access or mutate registry truth"
    );
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("failed release leaves runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Faulted,
        "all-deny was complete but the pre-CAS failure remains explicitly Faulted"
    );
}

#[test]
fn newer_committed_enable_supersedes_old_run_and_stales_its_late_plan() {
    let (mut registry, request_a, receipt_a) = exact_both_enable_fixture_at(991, 0, 10_000);
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let input_a = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity_a =
        durable_enable_identity(&input_a, &request_a, &receipt_a).expect("A identity is valid");
    let run_a = match runtime
        .admit_verified_durable_enable_with(1, || {
            registry
                .issue_managed_exact_both_durable_enable_evidence(&receipt_a, identity_a)
                .map_err(|error| format!("A evidence failed: {error:?}"))
        })
        .expect("A starts armed")
    {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run_id) => run_id,
        outcome => panic!("A must create a worker run, got {outcome:?}"),
    };
    let late_a_plan = match runtime
        .begin_current_boundary(
            super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveBoundary::Standby,
        )
        .expect("A boundary plan is captured")
    {
        super::super::output_lease_keepalive_runtime::OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured(plan) => plan,
        outcome => panic!("A boundary must capture its exact run, got {outcome:?}"),
    };

    let (_, request_b, _) = exact_both_enable_fixture_at(992, 20_000, 60_000);
    let receipt_b = registry
        .submit_request(&request_b, 20_000)
        .expect("expired A is recovered by current durable B");
    let input_b = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity_b =
        durable_enable_identity(&input_b, &request_b, &receipt_b).expect("B identity is valid");
    let cleanup_b =
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request_b, &receipt_b, identity_b,
        )
        .expect("new durable B constructs cleanup-only authority without a current reproof");
    let plan_b = match runtime
        .begin_cleanup_only_after_unadmitted_durable_enable(cleanup_b)
        .into_outcome()
    {
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan) => plan,
        outcome => panic!("B must supersede stale A with one cleanup plan, got {outcome:?}"),
    };
    assert_eq!(
        plan_b.run_id(),
        run_a,
        "B inherits A's run only to cancel its worker"
    );
    assert_eq!(
        plan_b.identity().request_correlation(),
        "principal:operator-a;domain:output-control;request:992",
        "the replacement fail-stop/release authority is durable B, not A"
    );
    let run_b_cleanup = plan_b.run_id();
    let identity_b = plan_b.identity().clone();
    let mut ports_b = CanonicalPorts::default();
    runtime
        .execute_fail_stop(&mut ports_b, plan_b)
        .expect("B cleanup all-deny completes");
    let mut late_a_ports = CanonicalPorts::default();
    assert!(matches!(
        runtime
            .execute_fail_stop(&mut late_a_ports, late_a_plan)
            .expect("late A plan is observed"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopCompletion::StaleNoop
    ));
    assert!(
        late_a_ports.0.is_empty(),
        "late A cannot touch physical ports after B superseded it"
    );
    assert!(matches!(
        runtime
            .relinquish_after_successful_boundary(run_b_cleanup, &identity_b, |permit| {
                Ok(registry.relinquish_managed_exact_both(permit, 20_001))
            })
            .expect("only B can release after the inherited-run all-deny"),
        Some(super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::Completed)
    ));
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("B exact release leaves runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle
    );
}

#[test]
fn old_replay_with_newer_current_authority_rejects_without_cleanup_ports_or_cas() {
    let (mut registry, replay_request, replay_receipt) =
        exact_both_enable_fixture_at(991, 0, 10_000);
    let (_, newer_request, _) = exact_both_enable_fixture_at(992, 20_000, 60_000);
    registry
        .submit_request(&newer_request, 20_000)
        .expect("expired replay authority is superseded by a newer current receipt");
    let registry_before_replay = registry.clone();
    let runtime = OutputLeaseKeepaliveRuntime::default();
    let replay_input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let replay_identity = durable_enable_identity(&replay_input, &replay_request, &replay_receipt)
        .expect("old receipt can still describe its historical identity");

    assert!(
        runtime
            .admit_verified_durable_enable_with(20_000, || {
                registry
                    .issue_managed_exact_both_durable_enable_evidence(
                        &replay_receipt,
                        replay_identity,
                    )
                    .map_err(|error| format!("old replay exact reproof failed: {error:?}"))
            })
            .is_err(),
        "the old replay receipt must fail exact current-authority reproof"
    );
    assert_eq!(
        registry, registry_before_replay,
        "replay reproof is read-only and cannot release the newer current lease"
    );
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("replay rejection leaves runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle,
        "a replay carries no cleanup-only capability and cannot create a physical plan"
    );
    let ports = CanonicalPorts::default();
    assert!(
        ports.0.is_empty(),
        "replay rejection invokes no physical cleanup ports"
    );
}

#[test]
fn malformed_new_commit_cannot_issue_cleanup_authority_or_physical_plan() {
    let (_registry, request, mut receipt) = exact_both_enable_fixture();
    receipt.audit_sequence = 0;
    let input = DurableEnableIdentityInput {
        principal: "operator-a",
        window_label: "main",
        owner_incarnation: 7,
        project_epoch: 11,
    };
    let identity = durable_enable_identity(&input, &request, &receipt)
        .expect("identity construction is not cleanup authority");
    assert!(
        OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
            &request, &receipt, identity,
        )
        .is_err(),
        "only a valid returned durable-commit receipt can authorize cleanup"
    );
    let runtime = OutputLeaseKeepaliveRuntime::default();
    assert_eq!(
        runtime
            .snapshot_state()
            .expect("invalid cleanup evidence leaves runtime readable"),
        super::super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Idle
    );
    let ports = CanonicalPorts::default();
    assert!(
        ports.0.is_empty(),
        "invalid cleanup evidence invokes no physical ports"
    );
}
