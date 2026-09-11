use super::*;
use crate::output_lease::{OutputLeaseOwner, OutputLeaseResource, OutputLeaseResources};
use crate::tests::MediaAssetA6CommandHarness;

fn fence(state: &AppState) -> OutputControlFenceV1 {
    let coordinator = state.project_coordinator.lock().unwrap();
    let output = state.engine.output_ownership_status();
    let safety = state.engine.safety_blackout_authority();
    OutputControlFenceV1 {
        process_incarnation: 1,
        session_incarnation: 1,
        project_epoch: coordinator.epoch,
        project_revision: coordinator.revision,
        project_checkpoint_hash: coordinator.checkpoint_hash.clone(),
        project_publication_generation: coordinator.publication_generation,
        output_epoch: output.epoch + 1,
        output_generation: output.generation + 1,
        safety_blackout_epoch: safety.epoch,
        safety_blackout_generation: safety.generation,
    }
}

fn authorize(state: &AppState, resources: &[OutputLeaseResource]) -> OutputLeaseRequest {
    let mut registry = state.output_lease_registry.lock().unwrap();
    let owner = OutputLeaseOwner::new(
        "blackout-test",
        "main",
        registry.process_session_incarnation(),
        1,
    )
    .unwrap();
    let resources = OutputLeaseResources::new(resources).unwrap();
    let acquire = OutputLeaseRequest::from_action(
        "blackout-test",
        crate::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        1,
        OutputLeaseRequestAction::Acquire {
            owner: owner.clone(),
            resources: resources.clone(),
            project_identity: crate::output_lease_project_identity(0),
            ttl_ms: crate::output_lease::MAX_OUTPUT_LEASE_TTL_MS,
        },
    )
    .unwrap();
    let receipt = registry
        .submit_request(&acquire, state.output_lease_now_ms().unwrap())
        .unwrap();
    OutputLeaseRequest::from_action(
        "blackout-test",
        crate::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        2,
        OutputLeaseRequestAction::AuthorizeOrdinary {
            lease_id: receipt.lease_id.unwrap(),
            owner,
            expected_generation: receipt.generation_after.unwrap(),
            exact_resources: OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Video,
            ])
            .unwrap(),
        },
    )
    .unwrap()
}

#[test]
fn target_blackout_stale_fence_and_split_lease_cannot_publish() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let request = authorize(state, &[OutputLeaseResource::Lighting]);
    let current = fence(state);
    let mut stale = current.clone();
    stale.project_revision += 1;
    let before = state.engine.snapshot();
    assert!(set_blackout_with_output_control_fence(
        state,
        OutputControlTargetRoleV1::Lighting,
        true,
        &stale,
        &request,
        None
    )
    .is_err());
    assert!(set_blackout_with_output_control_fence(
        state,
        OutputControlTargetRoleV1::Lighting,
        true,
        &current,
        &request,
        None
    )
    .is_err());
    let after = state.engine.snapshot();
    assert_eq!(after.blackout, before.blackout);
    assert_eq!(after.video.blackout, before.video.blackout);
    assert_eq!(fence(state), current);
}

#[test]
fn target_blackout_toggle_is_published_and_exact_retry_never_reapplies() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let request = authorize(
        state,
        &[OutputLeaseResource::Lighting, OutputLeaseResource::Video],
    );
    let current = fence(state);
    use protocol::control_plane_command::{
        OutputControlActionV2, OutputControlCommandRequestV2, OutputControlReceiptOutcomeV2,
        OutputControlReceiptV2, OutputControlResponseV2, OutputLeaseAuthorityV1,
        OUTPUT_BLACKOUT_SET_OPERATION_ID,
    };
    let (lease_id, generation) = match request.action.as_ref() {
        Some(crate::output_lease::OutputLeaseRequestAction::AuthorizeOrdinary {
            lease_id,
            expected_generation,
            ..
        }) => (*lease_id, *expected_generation),
        action => panic!("unexpected blackout authorization action: {action:?}"),
    };
    let public = OutputControlCommandRequestV2 {
        operation_id: OUTPUT_BLACKOUT_SET_OPERATION_ID.to_string(),
        request_id: request.key.request_id,
        expected_fence: current.clone(),
        action: OutputControlActionV2::SetBlackout {
            target: OutputControlTargetRoleV1::Lighting,
            enabled: true,
            lease: OutputLeaseAuthorityV1 {
                lease_id: lease_id.encode(),
                generation,
            },
        },
    };
    let shape = crate::sha256_hex(&public.canonical_shape_bytes().unwrap());
    let fingerprint = crate::sha256_hex(&public.argument_fingerprint_bytes().unwrap());
    let terminal_identity = crate::ManagedExactBothOutputControlTerminalIdentity {
        principal: "blackout-test",
        window_label: "main",
        operation_id: &public.operation_id,
        request_id: public.request_id,
        shape_sha256: &shape,
        argument_fingerprint: &fingerprint,
    };
    let (applied, after, receipt) = set_blackout_with_output_control_fence(
        state,
        OutputControlTargetRoleV1::Lighting,
        true,
        &current,
        &request,
        Some(terminal_identity),
    )
    .unwrap();
    assert!(applied);
    assert_eq!(after.project_revision, current.project_revision + 1);
    assert_eq!(
        after.project_publication_generation,
        current.project_publication_generation + 1
    );
    assert_ne!(
        after.project_checkpoint_hash,
        current.project_checkpoint_hash
    );
    assert_eq!(after.output_epoch, current.output_epoch);
    assert_eq!(after.output_generation, current.output_generation);
    assert_eq!(after.safety_blackout_epoch, current.safety_blackout_epoch);
    let snapshot = state.engine.snapshot();
    assert!(snapshot.blackout);
    assert!(!snapshot.video.blackout);
    assert_eq!(fence(state), after);

    // Exercise the real public terminal store/replay seam used before adapter
    // admission. The old request fence is intentionally stale after commit.
    let response = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
        operation_id: public.operation_id.clone(),
        request_id: public.request_id,
        shape_sha256: shape.clone(),
        argument_fingerprint: fingerprint.clone(),
        audit_sequence: 1,
        fence_before: current,
        fence_after: after.clone(),
        outcome: OutputControlReceiptOutcomeV2::Applied,
        lease_result: Some(
            control_plane_runtime::output_control_lease_result_from_registry_receipt(
                &public.action,
                &receipt,
            )
            .unwrap(),
        ),
    }));
    response.validate().unwrap();
    crate::record_durable_managed_exact_both_output_control_terminal(
        state,
        "blackout-test",
        "main",
        &public,
        &shape,
        &fingerprint,
        &receipt,
        &response,
    )
    .unwrap();
    let retried = crate::replay_durable_managed_exact_both_output_control_terminal(
        state,
        "blackout-test",
        "main",
        &public,
        &shape,
        &fingerprint,
    )
    .unwrap();
    assert_eq!(retried, Some(response));
    assert_eq!(fence(state), after);
    let mut conflicting = public;
    if let OutputControlActionV2::SetBlackout { target, .. } = &mut conflicting.action {
        *target = OutputControlTargetRoleV1::Video;
    }
    assert!(
        crate::replay_durable_managed_exact_both_output_control_terminal(
            state,
            "blackout-test",
            "main",
            &conflicting,
            &crate::sha256_hex(&conflicting.canonical_shape_bytes().unwrap()),
            &crate::sha256_hex(&conflicting.argument_fingerprint_bytes().unwrap())
        )
        .is_err()
    );
    assert_eq!(fence(state), after);
}

#[test]
fn target_blackout_noop_keeps_project_fence_and_history() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let request = authorize(
        state,
        &[OutputLeaseResource::Lighting, OutputLeaseResource::Video],
    );
    let before = fence(state);
    let history_before = state.project_coordinator.lock().unwrap().history_generation;
    let (applied, after, _) = set_blackout_with_output_control_fence(
        state,
        OutputControlTargetRoleV1::Both,
        false,
        &before,
        &request,
        None,
    )
    .unwrap();
    assert!(!applied);
    assert_eq!(after, before);
    assert_eq!(
        state.project_coordinator.lock().unwrap().history_generation,
        history_before
    );
    assert!(!state
        .project_transaction_active
        .load(std::sync::atomic::Ordering::Acquire));
}

#[test]
fn target_blackout_each_target_and_direction_commits_exact_persisted_image() {
    for target in [
        OutputControlTargetRoleV1::Lighting,
        OutputControlTargetRoleV1::Video,
        OutputControlTargetRoleV1::Both,
    ] {
        for enabled in [false, true] {
            let harness = MediaAssetA6CommandHarness::new();
            let state = &harness.state;
            state
                .engine
                .set_output_blackout_published(
                    OutputControlTargetRoleV1::Both,
                    !enabled,
                    Instant::now() + Duration::from_secs(2),
                )
                .unwrap();
            {
                let mut coordinator = state.project_coordinator.lock().unwrap();
                crate::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)
                    .unwrap();
            }
            let request = authorize(
                state,
                &[OutputLeaseResource::Lighting, OutputLeaseResource::Video],
            );
            let before = fence(state);
            let (applied, after, _) =
                set_blackout_with_output_control_fence(
                    state,
                    target,
                    enabled,
                    &before,
                    &request,
                    None,
                )
                    .unwrap();
            assert!(applied, "{target:?} -> {enabled}");
            assert_eq!(after.project_revision, before.project_revision + 1);
            assert_eq!(
                after.project_publication_generation,
                before.project_publication_generation + 1
            );
            assert_ne!(
                after.project_checkpoint_hash,
                before.project_checkpoint_hash
            );
            assert_eq!(after.safety_blackout_epoch, before.safety_blackout_epoch);
            let mut persisted = state.engine.persistence_snapshot().unwrap();
            crate::use_authored_video_snapshot(&mut persisted);
            let lighting = if target == OutputControlTargetRoleV1::Video {
                !enabled
            } else {
                enabled
            };
            let video = if target == OutputControlTargetRoleV1::Lighting {
                !enabled
            } else {
                enabled
            };
            assert_eq!(persisted.blackout, lighting);
            assert_eq!(persisted.video.blackout, video);
            let coordinator = state.project_coordinator.lock().unwrap();
            let actual = crate::project_checkpoint_for_coordinator(state, &coordinator).unwrap();
            assert_eq!(actual.hash, after.project_checkpoint_hash);
        }
    }
}
