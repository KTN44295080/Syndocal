use super::*;
use crate::output_lease::{
    OutputLeaseOwner, OutputLeaseRequest, OutputLeaseRequestAction, OutputLeaseResource,
    OutputLeaseResources,
};

#[test]
fn output_control_identity_survives_renderer_and_native_restart() {
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let label = "media-asset-a6";
    let mut registry = state.output_lease_registry.lock().unwrap();
    let owner = OutputLeaseOwner::new(
        "local-operator",
        label,
        registry.process_session_incarnation(),
        1,
    )
    .unwrap();
    let request = OutputLeaseRequest::from_action(
        "local-operator",
        crate::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        200,
        OutputLeaseRequestAction::Acquire {
            owner,
            resources: OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Video,
            ])
            .unwrap(),
            project_identity: crate::output_lease_project_identity(0),
            ttl_ms: crate::output_lease::MAX_OUTPUT_LEASE_TTL_MS,
        },
    )
    .unwrap();
    let receipt = registry.submit_request(&request, 1).unwrap();
    let reset_renderer_request = OutputLeaseRequest::from_action(
        "local-operator",
        crate::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        1,
        request.action.as_ref().unwrap().clone(),
    )
    .unwrap();
    assert!(
        registry
            .preflight_request(&reset_renderer_request, 1)
            .is_err(),
        "the former renderer counter restarts below the retained request high water"
    );
    drop(registry);
    assert_eq!(allocate(state, label).unwrap(), 201);
    state
        .output_lease_durable_receipts
        .lock()
        .unwrap()
        .record(&receipt)
        .unwrap();
    let path = std::env::temp_dir().join(format!(
        "syndocal-request-id-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    crate::persist_output_lease_receipt_state_to_path(
        &path,
        &state.output_lease_durable_receipts.lock().unwrap().state,
    )
    .unwrap();
    let restarted = crate::tests::MediaAssetA6CommandHarness::new();
    restarted
        .state
        .output_lease_durable_receipts
        .lock()
        .unwrap()
        .install_path(path.clone())
        .unwrap();
    assert_eq!(allocate(&restarted.state, label).unwrap(), 201);
    assert_eq!(allocate(&restarted.state, label).unwrap(), 202);
    assert_eq!(allocate(&restarted.state, label).unwrap(), 203);
    std::fs::remove_file(path).unwrap();
}

#[test]
fn output_control_identity_rejects_invalid_window_and_exhaustion() {
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    assert!(allocate(&harness.state, "").is_err());
    harness
        .state
        .next_output_lease_request_id
        .store(MAX_SAFE_JAVASCRIPT_INTEGER, Ordering::Release);
    assert_eq!(
        allocate(&harness.state, "media-asset-a6").unwrap(),
        MAX_SAFE_JAVASCRIPT_INTEGER
    );
    assert!(allocate(&harness.state, "media-asset-a6")
        .unwrap_err()
        .contains("exhausted"));
}
