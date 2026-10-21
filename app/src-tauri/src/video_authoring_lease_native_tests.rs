use super::*;
use crate::tests::MediaAssetA6CommandHarness;

const WINDOW: &str = "media-asset-a6";

fn fixture() -> (MediaAssetA6CommandHarness, ControlPlaneQueryState, OutputLeaseRequest) {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    state.engine.set_output_ownership_role(MachineOutputRole::Standby).unwrap();
    let mut registry = state.output_lease_registry.lock().unwrap();
    let process = registry.process_session_incarnation();
    let principal = state.project_transaction_owners.lock().unwrap()[WINDOW].clone();
    let incarnation = state.project_transaction_owner_incarnations.lock().unwrap()[WINDOW];
    let owner = OutputLeaseOwner::new(principal.clone(), WINDOW, process, incarnation).unwrap();
    let acquire = OutputLeaseRequest::from_action(principal.clone(), "authoring-previous-project", 1,
        output_lease::OutputLeaseRequestAction::Acquire { owner: owner.clone(),
            resources: OutputLeaseResources::new(&[OutputLeaseResource::Video]).unwrap(),
            project_identity: "project_epoch:99".into(), ttl_ms: output_lease::MAX_OUTPUT_LEASE_TTL_MS }).unwrap();
    let acquired = registry.submit_request(&acquire, state.output_lease_now_ms().unwrap()).unwrap();
    let replacement = OutputLeaseRequest::from_action(principal, "authoring-replacement", 1,
        output_lease::OutputLeaseRequestAction::ReplacePreviousProjectForAuthoring {
            lease_id: acquired.lease_id.unwrap(), owner, expected_generation: acquired.generation_after.unwrap(),
            project_identity: output_lease_project_identity(state.project_coordinator.lock().unwrap().epoch),
            ttl_ms: output_lease::MAX_OUTPUT_LEASE_TTL_MS,
        }).unwrap();
    drop(registry);
    (harness, ControlPlaneQueryState::for_process_incarnation(process).unwrap(), replacement)
}

#[test]
fn video_authoring_native_observation_and_exchange_preserve_project_and_standby() {
    let (harness, query, request) = fixture();
    let state = &harness.state;
    let before = state.output_lease_registry_snapshot().unwrap();
    let observation = query_for_window(state, &query, WINDOW).unwrap();
    assert_eq!(observation.project_matches, Some(false));
    assert_eq!(observation.status, "held_active");
    assert!(!observation.managed);
    assert_eq!(state.output_lease_registry_snapshot().unwrap(), before, "query must not orphan or release authority");
    let ownership = state.engine.output_ownership_status();
    let coordinator = state.project_coordinator.lock().unwrap();
    let old_epoch = coordinator.epoch;
    let receipt = submit_replacement(state, &coordinator, &request, &observation.fence).unwrap();
    assert_eq!(receipt.outcome, Ok(output_lease::OutputLeaseOperationOutcome::Acquired));
    assert_ne!(receipt.lease_id.unwrap().encode(), observation.authority.unwrap().lease_id);
    assert_eq!(coordinator.epoch, old_epoch);
    assert_eq!(state.engine.output_ownership_status(), ownership, "replacement never arms outputs or rotates their fences");
    drop(coordinator);
    let after = query_for_window(state, &query, WINDOW).unwrap();
    assert_eq!(after.project_matches, Some(true));
    assert_eq!(after.authority.unwrap().lease_id, receipt.lease_id.unwrap().encode());
}

#[test]
fn video_authoring_native_replacement_rejects_active_stale_or_pending_authority() {
    for boundary in ["active", "stale-output", "stale-project", "pending", "failed-sdk"] {
        let (harness, query, request) = fixture();
        let state = &harness.state;
        let mut fence = query_for_window(state, &query, WINDOW).unwrap().fence;
        let before = state.output_lease_registry_snapshot().unwrap();
        let mut teardown = None;
        if boundary == "active" {
            let transition = state.engine.begin_output_ownership_transition(MachineOutputRole::Video).unwrap();
            transition.complete().unwrap();
            let current = state.engine.output_ownership_status();
            fence.output_epoch = current.epoch + 1; fence.output_generation = current.generation + 1;
        } else if boundary == "stale-output" {
            fence.output_generation += 1;
        } else if boundary == "pending" {
            state.project_transaction_active.store(true, Ordering::Release);
        } else if boundary == "failed-sdk" {
            teardown = Some(state.engine.begin_output_ownership_failure_fence("SDK close pending"));
            let current = state.engine.output_ownership_status();
            fence.output_epoch = current.epoch + 1; fence.output_generation = current.generation + 1;
        }
        let mut coordinator = state.project_coordinator.lock().unwrap();
        if boundary == "stale-project" { coordinator.revision += 1; }
        assert!(submit_replacement(state, &coordinator, &request, &fence).is_err(), "{boundary}");
        assert_eq!(state.output_lease_registry_snapshot().unwrap(), before, "{boundary}: rejection cannot release the old claim");
        state.project_transaction_active.store(false, Ordering::Release);
        drop(coordinator); drop(teardown);
    }
}
