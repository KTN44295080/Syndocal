//! Ordinary target blackout is an authored project mutation; S0 remains runtime-only.
//! Reuse exact-Both authorization and the preflighted project publication contract.
use super::{
    control_plane_runtime, ensure_no_pending_project_transaction, lock_output_ownership_transition,
    lock_project_coordinator, lock_project_external_command_admission,
    managed_exact_both_request_after_confirmation,
    output_lease::{OutputLeaseRequest, OutputLeaseRequestAction, OutputLeaseRequestReceipt},
    output_lease_project_identity, reconcile_project_checkpoint_for_coordinator,
    submit_output_lease_candidate_with_classified_commit_and_durable_record_for_pending_window_inner,
    validate_managed_exact_both_candidate_bridge, AppState, OutputLeaseCandidateCommitFailure,
};
use protocol::control_plane_command::{OutputControlFenceV1, OutputControlTargetRoleV1};
use std::{
    sync::atomic::Ordering,
    time::{Duration, Instant},
};

#[path = "output_blackout_project_commit.rs"]
mod project_commit;

pub(crate) fn set_blackout_with_output_control_fence(
    state: &AppState,
    target: OutputControlTargetRoleV1,
    enabled: bool,
    expected_fence: &OutputControlFenceV1,
    lease_request: &OutputLeaseRequest,
) -> Result<(bool, OutputControlFenceV1, OutputLeaseRequestReceipt), String> {
    let _lifecycle = state
        .standby_sync_lifecycle
        .lock()
        .map_err(|_| "Standby synchronization lifecycle lock was poisoned".to_string())?;
    let _external = lock_project_external_command_admission(state)?;
    let mut coordinator = lock_project_coordinator(state)?;
    let _transition = lock_output_ownership_transition(state)?;
    // Hold the engine's persistence writer gate from exact A capture through
    // acknowledged B verification. Concurrent authored writers cannot enter.
    let submission = state
        .engine
        .begin_persistence_mutation_submission()
        .map_err(|error| error.to_string())?;
    // Reconciliation uses a persistence read; that read does not re-enter the
    // writer gate held above (the existing Display assignment uses this order).
    if reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || !control_plane_runtime::exact_output_control_fence_matches(
            state,
            &coordinator,
            expected_fence,
        )
        || ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        return Err("Output control fence changed before target blackout".to_string());
    }
    // Public exact retries are resolved by the runtime's durable terminal
    // replay before this mutation seam. A private receipt alone cannot prove
    // the target/boolean or the post-commit project fence.
    let plan = project_commit::prepare(&submission, &coordinator, target, enabled)?;
    let managed_request = managed_exact_both_request_after_confirmation(
        state,
        lease_request,
        Some(&output_lease_project_identity(expected_fence.project_epoch)),
    )?;
    let candidate_request = managed_request.as_ref().unwrap_or(lease_request);
    let managed_authorization = candidate_request
        .action
        .as_ref()
        .and_then(OutputLeaseRequestAction::managed_exact_both_lease_id)
        .map(|lease_id| {
            state
                .output_lease_keepalive
                .begin_managed_exact_both_ordinary_authorization(&lease_id.encode())
        })
        .transpose()?;
    validate_managed_exact_both_candidate_bridge(
        candidate_request,
        managed_authorization.as_ref(),
    )?;
    let mut registry = state.output_lease_registry.lock().map_err(|_| {
        "Output lease registry lock was poisoned before target blackout".to_string()
    })?;
    let now = state.output_lease_now_ms()?;
    let publication = submit_output_lease_candidate_with_classified_commit_and_durable_record_for_pending_window_inner(
        state, &mut registry, candidate_request, managed_authorization.as_ref(),
        lease_request, now, "target blackout", None,
        || {
            if !plan.changed() {
                return Ok((false, expected_fence.clone()));
            }
            state.project_transaction_active.store(true, Ordering::Release);
            let applied = match submission.set_output_blackout_published(
                target, enabled, Instant::now() + Duration::from_secs(2),
            ) {
                Ok(applied) => applied,
                Err(error) => {
                    state.project_transaction_active.store(false, Ordering::Release);
                    return Err(OutputLeaseCandidateCommitFailure::safe(error));
                }
            };
            if let Err(error) = plan.verify(&submission, &coordinator, applied) {
                state.project_external_command_admission.project_transaction_publication_faulted
                    .store(true, Ordering::Release);
                return Err(OutputLeaseCandidateCommitFailure::in_doubt(error));
            }
            plan.commit(&mut coordinator);
            Ok((true, control_plane_runtime::committed_output_control_fence(
                expected_fence,
                control_plane_runtime::CommittedOutputControlFenceValues {
                    project_epoch: coordinator.epoch,
                    project_revision: coordinator.revision,
                    project_checkpoint_hash: &coordinator.checkpoint_hash,
                    project_publication_generation: coordinator.publication_generation,
                    output_epoch: expected_fence.output_epoch,
                    output_generation: expected_fence.output_generation,
                    safety_blackout_epoch: expected_fence.safety_blackout_epoch,
                    safety_blackout_generation: expected_fence.safety_blackout_generation,
                },
            )))
        },
        |durable, receipt| durable.record(receipt).map_err(|error| format!("{error:?}")),
    );
    match publication {
        Ok(((applied, after), receipt)) => {
            state
                .project_transaction_active
                .store(false, Ordering::Release);
            Ok((applied, after, receipt))
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
#[path = "output_blackout_control_tests.rs"]
mod tests;
