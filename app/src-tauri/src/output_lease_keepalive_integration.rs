//! Application integration for the exact managed `Lighting + Video` output
//! lease keepalive.  This is intentionally the only Tauri/AppState adapter:
//! the manager remains a pure serialized authority machine and never owns an
//! application, project, registry, or physical-output lock.

use std::sync::Arc;

use tauri::{AppHandle, Manager};

use super::{
    output_lease::{
        output_lease_keepalive::{
            output_lease_keepalive_adapter_port_error, OutputLeaseKeepaliveBoundary,
            OutputLeaseKeepaliveCleanupOnlyOutcome,
            OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence,
            OutputLeaseKeepaliveExactRelinquishResponseEvidence,
            OutputLeaseKeepaliveManagedConflictAction, OutputLeaseKeepalivePortError,
        },
        OutputLeaseRequest, OutputLeaseRequestReceipt,
    },
    output_lease_keepalive_runtime::{
        OutputLeaseKeepaliveRuntimeAdmission, OutputLeaseKeepaliveRuntimeCurrentBoundary,
        OutputLeaseKeepaliveRuntimeManualRelinquish, OutputLeaseKeepaliveRuntimeSupersession,
        ProductionKeepaliveFailStopPorts,
    },
    output_lease_project_identity, AppState,
};

/// Identity input captured by the public output-control fence.  The registry
/// evidence factory independently rechecks every authority component before
/// it can issue arm evidence, so this caller-derived form is never authority
/// by itself.
#[derive(Debug, Clone)]
pub(crate) struct DurableEnableIdentityInput<'a> {
    pub(crate) principal: &'a str,
    pub(crate) window_label: &'a str,
    pub(crate) owner_incarnation: u64,
    pub(crate) project_epoch: u64,
}

fn port_error(detail: impl Into<String>) -> OutputLeaseKeepalivePortError {
    output_lease_keepalive_adapter_port_error(detail)
}

fn durable_enable_identity(
    input: &DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveIdentity, String> {
    // Request key is the durable correlation; `lookup(request)` below also
    // proves its complete canonical shape, preventing a key-only replay.
    super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveIdentity::new(
        output_lease_project_identity(input.project_epoch),
        // The registry independently binds the owner identity to the durable
        // receipt principal.  Keep the incarnation in the session component;
        // putting it in this field would make every valid exact-Both evidence
        // fail the registry's principal equality check.
        input.principal,
        input.window_label,
        format!(
            "process:{};owner:{}",
            receipt.process_session_incarnation, input.owner_incarnation
        ),
        format!(
            "principal:{};domain:{};request:{}",
            request.key.principal, request.key.domain, request.key.request_id
        ),
    )
    .map_err(|error| format!("Managed durable Enable identity is invalid: {error:?}"))
}

fn require_exact_durable_terminal(
    durable: &super::OutputLeaseDurableReceiptJournal,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<(), String> {
    let retained = durable
        .lookup(request)
        .map_err(|error| format!("Managed durable Enable lookup failed: {error:?}"))?;
    if retained.as_ref() != Some(receipt) {
        return Err(
            "Managed durable Enable receipt is not exact durable terminal evidence".to_string(),
        );
    }
    Ok(())
}

/// Construct exact durable evidence only while the runtime serial lane is
/// held.  Registry and journal locks are short-lived and no physical work can
/// happen while either is held.
fn admit_verified_durable_enable(
    state: &AppState,
    input: &DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<OutputLeaseKeepaliveRuntimeAdmission, String> {
    let now_ms = state.output_lease_now_ms()?;
    let identity = durable_enable_identity(input, request, receipt)?;
    let expected_request = request.clone();
    let expected_receipt = receipt.clone();
    state
        .output_lease_keepalive
        .admit_verified_durable_enable_with(now_ms, || {
            // The durable terminal is immutable.  Take the mutable registry
            // first, then prove that the record it currently exposes is the
            // exact receipt retained by the journal; no manager guard is live
            // during either registry/journal acquisition.
            let registry = state.output_lease_registry.lock().map_err(|_| {
                "Managed durable Enable registry lock was poisoned".to_string()
            })?;
            let durable = state.output_lease_durable_receipts.lock().map_err(|_| {
                "Managed durable Enable journal lock was poisoned".to_string()
            })?;
            require_exact_durable_terminal(&durable, &expected_request, &expected_receipt)?;
            registry
                .issue_managed_exact_both_durable_enable_evidence(&expected_receipt, identity)
                .map_err(|error| {
                    format!("Managed durable Enable registry authority is not current exact Both: {error:?}")
                })
        })
}

/// Issue cleanup-only authority at the new durable-commit boundary, before
/// normal evidence issuance reads the registry or durable journal.  This is
/// deliberately unavailable to replays: an old retained receipt cannot safely
/// retire a newer current output authority merely because its normal reproof
/// failed.
fn cleanup_only_evidence_after_new_durable_enable_commit(
    input: &DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence, String> {
    let identity = durable_enable_identity(input, request, receipt)?;
    super::output_lease::OutputLeaseRegistry::issue_managed_exact_both_cleanup_only_evidence_after_durable_commit(
        request,
        receipt,
        identity,
    )
    .map_err(|error| {
        format!(
            "Managed newly committed Enable could not construct cleanup-only authority: {error:?}"
        )
    })
}

fn execute_fail_stop(
    app: &AppHandle,
    state: &AppState,
    detail: &str,
    plan: super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopPlan,
) -> Result<
    (
        super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRunId,
        super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveIdentity,
    ),
    String,
> {
    // `ProductionKeepaliveFailStopPorts` is deliberately constructed only
    // after the plan is captured.  Its first port cancels/joins a worker, and
    // the remaining ports touch engine/DMX/native/persistence, so this scope
    // must never contain registry, durable, coordinator, lifecycle, or owner
    // authority guards.
    let run_id = plan.run_id();
    let identity = plan.identity().clone();
    let mut ports =
        ProductionKeepaliveFailStopPorts::new(app, state, &state.output_lease_keepalive, detail);
    match state.output_lease_keepalive.execute_fail_stop(&mut ports, plan)? {
        super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopCompletion::Completed => {
            Ok((run_id, identity))
        }
        super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveFailStopCompletion::StaleNoop => {
            Err("Managed fail-stop plan became stale before canonical completion".to_string())
        }
    }
}

fn prove_deferred_now(app: &AppHandle, state: &AppState, detail: &str) -> Result<(), String> {
    let now_ms = state.output_lease_now_ms()?;
    let outcome = state
        .output_lease_keepalive
        .resolve_deferred_after_relinquish(now_ms, |capability| {
            let mut registry = match state.output_lease_registry.lock() {
                Ok(registry) => registry,
                Err(_) => {
                    return Err((
                        capability,
                        port_error(
                            "Managed deferred supersession registry lock was poisoned before proof",
                        ),
                    ));
                }
            };
            Ok(registry.prove_managed_exact_both_supersession(capability, now_ms))
        })?;
    match outcome {
        OutputLeaseKeepaliveRuntimeSupersession::Armed(run_id) => {
            spawn_production_worker(app, state, run_id, detail)
        }
        OutputLeaseKeepaliveRuntimeSupersession::FailStop(plan) => {
            // An arm-admission failure is itself a B-bound fail-stop.  Finish
            // its B/C exact release/supersession chain now; it owns the only
            // safe terminal action for that B/C.
            let (run_id, identity) = execute_fail_stop(app, state, detail, plan)?;
            release_after_automatic_boundary(app, state, detail, run_id, &identity)?;
            Err(
                "Managed deferred Enable could not install a worker and was fail-stopped"
                    .to_string(),
            )
        }
        OutputLeaseKeepaliveRuntimeSupersession::RetainedFaulted => Err(
            "Managed deferred Enable supersession was rejected; authority remains fail-stopped"
                .to_string(),
        ),
        OutputLeaseKeepaliveRuntimeSupersession::StaleNoop => {
            Err("Managed deferred Enable supersession became stale before exact proof".to_string())
        }
    }
}

fn release_after_automatic_boundary(
    app: &AppHandle,
    state: &AppState,
    detail: &str,
    run_id: super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRunId,
    identity: &super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveIdentity,
) -> Result<(), String> {
    let now_ms = state.output_lease_now_ms()?;
    let outcome = state
        .output_lease_keepalive
        .relinquish_after_successful_boundary(run_id, identity, |permit| {
            let mut registry = match state.output_lease_registry.lock() {
                Ok(registry) => registry,
                Err(_) => {
                    return Err((
                        permit,
                        port_error("Managed automatic-boundary registry lock was poisoned before exact release"),
                    ));
                }
            };
            Ok(registry.relinquish_managed_exact_both(permit, now_ms))
        })?;
    match outcome {
        Some(super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::Completed) => Ok(()),
        Some(super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::RetainedForRetry) => {
            // The newest deferred B/C is manager-retained and must be proven
            // immediately, not on a later Enable request.
            prove_deferred_now(app, state, detail)
        }
        Some(super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRelinquishCompletion::StaleNoop) => Err(
            "Managed automatic-boundary exact release became stale".to_string(),
        ),
        None => Err(
            "Managed automatic-boundary fail-stop did not authorize exact release".to_string(),
        ),
    }
}

fn spawn_production_worker(
    app: &AppHandle,
    state: &AppState,
    run_id: super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveRunId,
    detail: &str,
) -> Result<(), String> {
    let worker_app = app.clone();
    let worker_runtime = Arc::clone(&state.output_lease_keepalive);
    let detail_owned = detail.to_string();
    match state.output_lease_keepalive.spawn_worker(run_id, move |control| {
        // The runtime's production loop owns every renewal outcome.  This
        // outer monitor converts an unexpected early return/panic to the same
        // canonical plan; it never joins its own worker.
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            super::output_lease_keepalive_runtime::OutputLeaseKeepaliveRuntime::run_production_worker(
                worker_app.clone(),
                Arc::clone(&worker_runtime),
                Arc::clone(&control),
            );
        }));
        let unexpected = result.is_err()
            || (!control.is_cancelled()
                && worker_runtime
                    .snapshot_state()
                    .is_ok_and(|state| {
                        state
                            == super::output_lease::output_lease_keepalive::OutputLeaseKeepaliveState::Armed
                    }));
        if !unexpected {
            return;
        }
        let worker_detail = if result.is_err() {
            "Managed output-lease keepalive worker panicked"
        } else {
            "Managed output-lease keepalive worker exited while still Armed"
        };
        let error = port_error(worker_detail);
        match worker_runtime.record_worker_termination(run_id, error) {
            Ok(Some(plan)) => {
                let worker_state = worker_app.state::<AppState>();
                match execute_fail_stop(&worker_app, &worker_state, &detail_owned, plan) {
                    Ok((failed_run, identity)) => {
                        if let Err(error) = release_after_automatic_boundary(
                            &worker_app,
                            &worker_state,
                            &detail_owned,
                            failed_run,
                            &identity,
                        ) {
                            eprintln!(
                                "{detail_owned}; worker termination exact release/supersession remains Faulted: {error}"
                            );
                        }
                    }
                    Err(error) => eprintln!(
                        "{detail_owned}; worker termination canonical fail-stop remains Faulted: {error}"
                    ),
                }
            }
            Ok(None) => {}
            Err(error) => eprintln!(
                "{detail_owned}; worker termination could not be recorded and requires explicit recovery: {error}"
            ),
        }
    }) {
        Ok(_) => Ok(()),
        Err(error) => {
            let plan = state
                .output_lease_keepalive
                .record_worker_spawn_failure(run_id, port_error(error))?;
            if let Some(plan) = plan {
                let (failed_run, identity) = execute_fail_stop(app, state, detail, plan)?;
                if let Err(release_error) = release_after_automatic_boundary(
                    app,
                    state,
                    detail,
                    failed_run,
                    &identity,
                ) {
                    return Err(format!(
                        "Managed durable Enable worker installation failed; all-deny fail-stop completed but exact release/supersession remains Faulted: {release_error}"
                    ));
                }
            }
            Err("Managed durable Enable worker installation failed and was fail-stopped".to_string())
        }
    }
}

fn exact_manual_relinquish(
    state: &AppState,
    lease_id: &str,
) -> Result<
    OutputLeaseKeepaliveRuntimeManualRelinquish<
        OutputLeaseKeepaliveExactRelinquishResponseEvidence,
    >,
    String,
> {
    let now_ms = state.output_lease_now_ms()?;
    state.output_lease_keepalive.manual_relinquish_with(lease_id, |permit| {
        let mut registry = match state.output_lease_registry.lock() {
            Ok(registry) => registry,
            Err(_) => {
                return Err((
                    permit,
                    port_error("Managed manual relinquish registry lock was poisoned before exact release"),
                ));
            }
        };
        let receipt = registry.relinquish_managed_exact_both(permit, now_ms);
        let Some(evidence) = receipt.exact_release_response_evidence() else {
            // A registry-rejected receipt must flow back into the manager so
            // it remains Faulted/RetainedForRetry; never manufacture public
            // success evidence for it.
            return Ok((receipt, None));
        };
        Ok((receipt, Some(evidence)))
    })
    .and_then(|outcome| match outcome {
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released(Some(evidence)) => Ok(
            OutputLeaseKeepaliveRuntimeManualRelinquish::Released(evidence),
        ),
        OutputLeaseKeepaliveRuntimeManualRelinquish::Released(None) => Err(
            "Managed exact release was accepted without success evidence".to_string(),
        ),
        OutputLeaseKeepaliveRuntimeManualRelinquish::AllowUnmanaged => {
            Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::AllowUnmanaged)
        }
        OutputLeaseKeepaliveRuntimeManualRelinquish::RejectManagedLease => {
            Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::RejectManagedLease)
        }
        OutputLeaseKeepaliveRuntimeManualRelinquish::FailStop(plan) => {
            Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::FailStop(plan))
        }
        OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry => {
            Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry)
        }
        OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop => {
            Ok(OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop)
        }
    })
}

/// Consume the cleanup-only bridge after normal admission failed for an
/// Enable that just committed its durable exact-`Both` receipt.  The runtime
/// drops its recovered serial/manager guards before returning a plan; all
/// worker joins and physical ports therefore remain outside every authority
/// lock.  This path is terminal-only: it cannot arm or renew a worker.
fn cleanup_newly_committed_durable_enable_after_admission_failure(
    app: &AppHandle,
    state: &AppState,
    evidence: OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence,
    admission_error: String,
) -> Result<(), String> {
    let cleanup = state
        .output_lease_keepalive
        .begin_cleanup_only_after_unadmitted_durable_enable(evidence);
    let recovered_operation_serial_poison = cleanup.recovered_operation_serial_poison();
    let recovered_manager_poison = cleanup.recovered_manager_poison();
    let recovery_detail = match (recovered_operation_serial_poison, recovered_manager_poison) {
        (false, false) => String::new(),
        (true, false) => {
            "; cleanup recovered a poisoned keepalive operation-serial mutex".to_string()
        }
        (false, true) => "; cleanup recovered a poisoned keepalive manager mutex".to_string(),
        (true, true) => {
            "; cleanup recovered poisoned keepalive operation-serial and manager mutexes"
                .to_string()
        }
    };
    if !recovery_detail.is_empty() {
        eprintln!(
            "Managed newly committed Enable admission failed; canonical cleanup is proceeding{}",
            recovery_detail
        );
    }
    let detail = format!(
        "Managed newly committed Enable admission failed: {admission_error}{recovery_detail}"
    );
    match cleanup.into_outcome() {
        OutputLeaseKeepaliveCleanupOnlyOutcome::FailStop(plan)
        | OutputLeaseKeepaliveCleanupOnlyOutcome::ResumeFailStop(plan) => {
            let (run_id, identity) = execute_fail_stop(app, state, &detail, plan)?;
            release_after_automatic_boundary(app, state, &detail, run_id, &identity)
        }
        OutputLeaseKeepaliveCleanupOnlyOutcome::RequireCurrentRelinquish(permit) => {
            match state.output_lease_keepalive.complete_current_relinquish_with(
                permit,
                |permit| {
                    let now_ms = match state.output_lease_now_ms() {
                        Ok(now_ms) => now_ms,
                        Err(error) => {
                            return Err((
                                permit,
                                port_error(format!(
                                    "Managed newly committed Enable cleanup clock failed before exact release: {error}"
                                )),
                            ));
                        }
                    };
                    let mut registry = match state.output_lease_registry.lock() {
                        Ok(registry) => registry,
                        Err(_) => {
                            return Err((
                                permit,
                                port_error(
                                    "Managed newly committed Enable cleanup registry lock was poisoned before exact release",
                                ),
                            ));
                        }
                    };
                    Ok((registry.relinquish_managed_exact_both(permit, now_ms), ()))
                },
            )? {
                OutputLeaseKeepaliveRuntimeManualRelinquish::Released(()) => Ok(()),
                OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry => Err(
                    "Managed newly committed Enable cleanup exact release was rejected; authority remains Faulted"
                        .to_string(),
                ),
                OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop => Err(
                    "Managed newly committed Enable cleanup exact release became stale; authority remains Faulted"
                        .to_string(),
                ),
                other => Err(format!(
                    "Managed newly committed Enable cleanup returned an invalid terminal release outcome: {other:?}"
                )),
            }
        }
        OutputLeaseKeepaliveCleanupOnlyOutcome::AwaitCurrentRelease => Err(
            "Managed newly committed Enable cleanup is awaiting an earlier exact release; authority remains Faulted"
                .to_string(),
        ),
        OutputLeaseKeepaliveCleanupOnlyOutcome::StaleNoop => Err(
            "Managed newly committed Enable cleanup evidence was stale; no physical ports were invoked"
                .to_string(),
        ),
    }
}

/// Complete a just-committed Enable only after runtime admission installed its
/// precise worker or returned the exact already-installed run.  The returned
/// receipt is a trusted new durable-commit result, so cleanup-only evidence is
/// deliberately issued before normal journal/registry reproof can fail.
pub(crate) fn complete_newly_committed_durable_enable(
    app: &AppHandle,
    state: &AppState,
    input: DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<(), String> {
    let cleanup_evidence =
        cleanup_only_evidence_after_new_durable_enable_commit(&input, request, receipt)?;
    complete_durable_enable_with_optional_post_commit_cleanup(
        app,
        state,
        input,
        request,
        receipt,
        Some(cleanup_evidence),
    )
}

/// Complete an exact durable replay.  Replays intentionally have no
/// cleanup-only capability: if their normal exact reproof fails, a newer
/// current authority may exist and replay cleanup would incorrectly blackout
/// that authority.
pub(crate) fn complete_durable_enable_replay(
    app: &AppHandle,
    state: &AppState,
    input: DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
) -> Result<(), String> {
    complete_durable_enable_with_optional_post_commit_cleanup(
        app, state, input, request, receipt, None,
    )
}

fn complete_durable_enable_with_optional_post_commit_cleanup(
    app: &AppHandle,
    state: &AppState,
    input: DurableEnableIdentityInput<'_>,
    request: &OutputLeaseRequest,
    receipt: &OutputLeaseRequestReceipt,
    cleanup_evidence: Option<OutputLeaseKeepaliveDurableEnableCleanupOnlyEvidence>,
) -> Result<(), String> {
    let admission = match admit_verified_durable_enable(state, &input, request, receipt) {
        Ok(admission) => admission,
        Err(admission_error) => {
            let Some(cleanup_evidence) = cleanup_evidence else {
                return Err(admission_error);
            };
            return match cleanup_newly_committed_durable_enable_after_admission_failure(
                app,
                state,
                cleanup_evidence,
                admission_error.clone(),
            ) {
                Ok(()) => Err(format!(
                    "{admission_error}; canonical cleanup completed and public Enable remains rejected"
                )),
                Err(cleanup_error) => Err(format!(
                    "{admission_error}; canonical cleanup remains Faulted: {cleanup_error}"
                )),
            };
        }
    };
    match admission {
        OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run_id) => spawn_production_worker(
            app,
            state,
            run_id,
            "Managed durable Enable worker installation",
        ),
        OutputLeaseKeepaliveRuntimeAdmission::AlreadyArmed(_) => Ok(()),
        OutputLeaseKeepaliveRuntimeAdmission::FailStop(plan) => {
            let (run_id, identity) = execute_fail_stop(
                app,
                state,
                "Managed durable Enable admission fail-stop",
                plan,
            )?;
            release_after_automatic_boundary(
                app,
                state,
                "Managed durable Enable admission fail-stop",
                run_id,
                &identity,
            )?;
            Err(
                "Managed durable Enable admission was fail-stopped before worker installation"
                    .to_string(),
            )
        }
        OutputLeaseKeepaliveRuntimeAdmission::RequireCurrentRelinquish(permit) => {
            let outcome = state.output_lease_keepalive.complete_current_relinquish_with(
                permit,
                |permit| {
                    let now_ms = match state.output_lease_now_ms() {
                        Ok(now_ms) => now_ms,
                        Err(error) => {
                            return Err((
                                permit,
                                port_error(format!(
                                    "Managed durable Enable clock failed before exact release: {error}"
                                )),
                            ));
                        }
                    };
                    let mut registry = match state.output_lease_registry.lock() {
                        Ok(registry) => registry,
                        Err(_) => return Err((permit, port_error("Managed durable Enable registry lock was poisoned before exact release"))),
                    };
                    let receipt = registry.relinquish_managed_exact_both(permit, now_ms);
                    Ok((receipt, ()))
                },
            )?;
            if matches!(
                outcome,
                OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry
            ) {
                prove_deferred_now(app, state, "Managed durable Enable deferred supersession")?;
            }
            Err("Managed durable Enable replay reached a terminal fail-stop release".to_string())
        }
        OutputLeaseKeepaliveRuntimeAdmission::ManagedConflict(conflict) => {
            match conflict.into_action() {
                OutputLeaseKeepaliveManagedConflictAction::ResumeFailStop(plan) => {
                    let (run_id, identity) = execute_fail_stop(
                        app,
                        state,
                        "Managed conflicting durable Enable fail-stop",
                        plan,
                    )?;
                    release_after_automatic_boundary(
                        app,
                        state,
                        "Managed conflicting durable Enable fail-stop",
                        run_id,
                        &identity,
                    )?;
                }
                OutputLeaseKeepaliveManagedConflictAction::PrepareDeferredSupersession => {
                    prove_deferred_now(
                        app,
                        state,
                        "Managed conflicting durable Enable deferred supersession",
                    )?;
                }
                OutputLeaseKeepaliveManagedConflictAction::AwaitCurrentRelease => {
                    return Err(
                        "Managed conflicting durable Enable is awaiting the current exact release"
                            .to_string(),
                    );
                }
            }
            // Re-prove this request after B/C arbitration.  If C replaced B
            // while its proof was out of lock, this cannot return success for
            // stale B; if B is still current it is AlreadyArmed/SpawnWorker.
            match admit_verified_durable_enable(state, &input, request, receipt)? {
                OutputLeaseKeepaliveRuntimeAdmission::AlreadyArmed(_) => Ok(()),
                OutputLeaseKeepaliveRuntimeAdmission::SpawnWorker(run_id) => {
                    spawn_production_worker(
                        app,
                        state,
                        run_id,
                        "Managed durable Enable post-supersession worker installation",
                    )
                }
                _ => Err(
                    "Managed durable Enable was superseded or did not reach installed state"
                        .to_string(),
                ),
            }
        }
        OutputLeaseKeepaliveRuntimeAdmission::AwaitCurrentRelease => {
            Err("Managed durable Enable is awaiting the current exact release".to_string())
        }
        OutputLeaseKeepaliveRuntimeAdmission::StaleNoop => Err(
            "Managed durable Enable durable receipt is stale for the current manager authority"
                .to_string(),
        ),
    }
}

/// Managed lifecycle routing bypasses the generic registry request path.
/// Call only after normal public fence/admission is complete and after every
/// application/coordinator/owner guard has been dropped.
pub(crate) fn relinquish_managed_after_public_admission(
    app: &AppHandle,
    state: &AppState,
    lease_id: &str,
) -> Result<OutputLeaseKeepaliveExactRelinquishResponseEvidence, String> {
    for _ in 0..2 {
        match exact_manual_relinquish(state, lease_id)? {
            OutputLeaseKeepaliveRuntimeManualRelinquish::Released(evidence) => return Ok(evidence),
            OutputLeaseKeepaliveRuntimeManualRelinquish::FailStop(plan) => {
                let _ = execute_fail_stop(app, state, "Managed manual Relinquish fail-stop", plan)?;
            }
            OutputLeaseKeepaliveRuntimeManualRelinquish::RetainedForRetry => {
                // Required same-flow B/C path: proof and its worker/fail-stop
                // happen now before this public request receives any result.
                prove_deferred_now(
                    app,
                    state,
                    "Managed manual Relinquish deferred supersession",
                )?;
                return Err(
                    "Managed Relinquish was superseded by a newer durable Enable".to_string(),
                );
            }
            OutputLeaseKeepaliveRuntimeManualRelinquish::AllowUnmanaged => {
                return Err("Managed runtime no longer owns this lease".to_string())
            }
            OutputLeaseKeepaliveRuntimeManualRelinquish::RejectManagedLease => {
                return Err("Managed lease lifecycle transition is rejected".to_string())
            }
            OutputLeaseKeepaliveRuntimeManualRelinquish::StaleNoop => {
                return Err("Managed lease relinquish became stale".to_string())
            }
        }
    }
    Err("Managed lease relinquish did not reach an exact terminal CAS".to_string())
}

/// Capture an application lifecycle boundary and perform the whole fail-stop
/// plus exact automatic release after the manager has supplied a plan.
pub(crate) fn execute_current_boundary(
    app: &AppHandle,
    state: &AppState,
    boundary: OutputLeaseKeepaliveBoundary,
    detail: &str,
) -> Result<(), String> {
    match state
        .output_lease_keepalive
        .begin_current_boundary(boundary)?
    {
        OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle => Ok(()),
        OutputLeaseKeepaliveRuntimeCurrentBoundary::RetainedFaulted => {
            Err("Managed output authority is already Faulted and remains retained".to_string())
        }
        OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured(plan) => {
            let (run_id, identity) = execute_fail_stop(app, state, detail, plan)?;
            release_after_automatic_boundary(app, state, detail, run_id, &identity)
        }
    }
}

pub(crate) fn execute_window_destroyed_boundary(
    app: &AppHandle,
    state: &AppState,
    window_label: &str,
) -> Result<(), String> {
    match state
        .output_lease_keepalive
        .begin_window_destroyed_boundary_if_matches(window_label)?
    {
        OutputLeaseKeepaliveRuntimeCurrentBoundary::Idle => Ok(()),
        OutputLeaseKeepaliveRuntimeCurrentBoundary::RetainedFaulted => {
            Err("Managed output authority is already Faulted and remains retained".to_string())
        }
        OutputLeaseKeepaliveRuntimeCurrentBoundary::Captured(plan) => {
            let (run_id, identity) =
                execute_fail_stop(app, state, "Managed window-destroyed output boundary", plan)?;
            release_after_automatic_boundary(
                app,
                state,
                "Managed window-destroyed output boundary",
                run_id,
                &identity,
            )
        }
    }
}

#[cfg(test)]
#[path = "output_lease_keepalive_integration_tests.rs"]
mod output_lease_keepalive_integration_tests;
