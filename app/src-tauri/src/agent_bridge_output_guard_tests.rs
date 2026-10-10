use super::*;
use std::cell::Cell;

#[test]
fn video_blackout_original_contract_rejects_extra_or_noncanonical_identity() {
    let valid = json!({"enabled":true,"expectedProject":{"project_epoch":1,"project_revision":2,"checkpoint_hash":"a".repeat(64)}});
    assert!(parse(valid.clone()).is_ok());
    for value in [
        json!({"enabled":true}),
        json!({"enabled":"true","expectedProject":valid["expectedProject"]}),
        json!({"enabled":true,"expectedProject":{"project_epoch":1,"project_revision":2,"checkpoint_hash":"A".repeat(64)}}),
        json!({"enabled":true,"expectedProject":valid["expectedProject"],"request":{}}),
        json!({"enabled":true,"expectedProject":{"project_epoch":1,"project_revision":2,"checkpoint_hash":"a".repeat(64),"principal":"other"}}),
    ] {
        assert_eq!(
            parse(value).err(),
            Some("agent_video_blackout_arguments_invalid".into())
        );
    }
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    let id = request_id(&harness.state).unwrap();
    assert_eq!(request_id(&harness.state).unwrap(), id + 1);
    assert!((1..=MAX_SAFE_JAVASCRIPT_INTEGER).contains(&id));
    harness.state.next_output_lease_request_id.store(
        MAX_SAFE_JAVASCRIPT_INTEGER + 1,
        std::sync::atomic::Ordering::Release,
    );
    assert_eq!(
        request_id(&harness.state).unwrap_err(),
        NativeAdapterError::from("agent_video_blackout_request_identity_exhausted")
    );
}

#[test]
fn video_blackout_native_preparation_refuses_stale_project_and_absent_active_both_lease() {
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let query = ControlPlaneQueryState::for_process_incarnation(
        state
            .output_lease_registry
            .lock()
            .unwrap()
            .process_session_incarnation(),
    )
    .unwrap();
    let label = "media-asset-a6";
    let fence = query
        .issue_output_control_fence_for_window(label, state)
        .unwrap();
    let intent = VideoBlackoutIntent {
        enabled: true,
        expected_project: ExpectedProject {
            project_epoch: fence.project_epoch,
            project_revision: fence.project_revision,
            checkpoint_hash: fence.project_checkpoint_hash,
        },
    };
    let runtime = state.engine.control_plane_runtime_snapshot();
    let output = state.engine.output_ownership_status();
    assert_eq!(
        prepare(label, state, &query, &intent).err(),
        Some("agent_video_blackout_active_both_lease_required".into())
    );
    let stale = VideoBlackoutIntent {
        enabled: false,
        expected_project: ExpectedProject {
            project_epoch: intent.expected_project.project_epoch,
            project_revision: intent.expected_project.project_revision + 1,
            checkpoint_hash: intent.expected_project.checkpoint_hash,
        },
    };
    assert_eq!(
        prepare(label, state, &query, &stale).err(),
        Some("agent_video_blackout_project_changed".into())
    );
    assert_eq!(state.engine.control_plane_runtime_snapshot(), runtime);
    assert_eq!(state.engine.output_ownership_status(), output);
}

#[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
#[test]
fn reset_final_external_authorization_denial_precedes_physical_retirement_after_real_locks() {
    let harness = crate::tests::MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let query = ControlPlaneQueryState::for_process_incarnation(
        state
            .output_lease_registry
            .lock()
            .unwrap()
            .process_session_incarnation(),
    )
    .unwrap();
    let label = "media-asset-a6";
    let fence = query
        .issue_output_control_fence_for_window(label, state)
        .unwrap();
    let before = state.engine.video_outputs_and_compositions_snapshot();
    let output = state.engine.output_ownership_status();
    let owner = state
        .project_transaction_owners
        .lock()
        .unwrap()
        .get(label)
        .cloned()
        .unwrap();
    let incarnation = *state
        .project_transaction_owner_incarnations
        .lock()
        .unwrap()
        .get(label)
        .unwrap();
    let checks = Cell::new(0);
    let retirements = Cell::new(0);
    let result =
        crate::reset_show_spout_outputs_without_output_lease_with_physical_retirement_authorized(
            state,
            &fence,
            &owner,
            label,
            incarnation,
            |_| {
                retirements.set(retirements.get() + 1);
                Ok(())
            },
            &|| {
                checks.set(checks.get() + 1);
                Err("revoked at final boundary".into())
            },
        );
    assert_eq!(result, Err("revoked at final boundary".into()));
    assert_eq!(checks.get(), 1);
    assert_eq!(retirements.get(), 0);
    assert_eq!(
        state.engine.video_outputs_and_compositions_snapshot(),
        before
    );
    assert_eq!(state.engine.output_ownership_status(), output);
    let success =
        crate::reset_show_spout_outputs_without_output_lease_with_physical_retirement_authorized(
            state,
            &fence,
            &owner,
            label,
            incarnation,
            |_| {
                retirements.set(retirements.get() + 1);
                Ok(())
            },
            &|| Ok(()),
        )
        .unwrap();
    assert_eq!(success, (false, fence));
    assert_eq!(retirements.get(), 0);
}
