//! Regression for successful managed Show activation poisoning later GUI BO.
use super::*;
use crate::tests::{MediaAssetA6CommandHarness, MEDIA_ASSET_A6_OWNER};

fn arm_managed(state: &AppState) -> (output_lease::OutputLeaseId, u64) {
    let now = state.output_lease_now_ms().unwrap();
    let mut registry = state.output_lease_registry.lock().unwrap();
    let process = registry.process_session_incarnation();
    let owner = OutputLeaseOwner::new(MEDIA_ASSET_A6_OWNER, "media-asset-a6", process, 1).unwrap();
    let request = OutputLeaseRequest::from_action(
        MEDIA_ASSET_A6_OWNER,
        OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN,
        501,
        OutputLeaseRequestAction::EnableAcquireOrRecover {
            owner,
            resources: OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Video,
            ])
            .unwrap(),
            project_identity: output_lease_project_identity(0),
            ttl_ms: output_lease::MAX_OUTPUT_LEASE_TTL_MS,
        },
    )
    .unwrap();
    let receipt = registry.submit_request(&request, now).unwrap();
    state
        .output_lease_durable_receipts
        .lock()
        .unwrap()
        .record(&receipt)
        .unwrap();
    let identity = output_lease::output_lease_keepalive::OutputLeaseKeepaliveIdentity::new(
        output_lease_project_identity(0), MEDIA_ASSET_A6_OWNER, "media-asset-a6",
        format!("process:{process};owner:1"),
        format!("principal:{MEDIA_ASSET_A6_OWNER};domain:{OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN};request:501"),
    ).unwrap();
    let evidence = registry
        .issue_managed_exact_both_durable_enable_evidence(&receipt, identity)
        .unwrap();
    drop(registry);
    state
        .output_lease_keepalive
        .admit_verified_durable_enable_with(now, || Ok(evidence))
        .unwrap();
    (receipt.lease_id.unwrap(), receipt.generation_after.unwrap())
}

#[test]
fn successful_managed_show_route_closes_its_receipt_and_allows_each_bo_target() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (lease_id, generation) = arm_managed(state);
    let lease = protocol::control_plane_command::OutputLeaseAuthorityV1 {
        lease_id: lease_id.encode(),
        generation,
    };
    let action = protocol::control_plane_command::OutputControlActionV2::EnableShowSerialDmxSafetyBlackoutRoute { lease: lease.clone() };
    let (request, now) = build_output_lease_authorization_request(
        state,
        MEDIA_ASSET_A6_OWNER,
        "media-asset-a6",
        1,
        502,
        &action,
        0,
    )
    .unwrap();
    let authorization = state
        .output_lease_keepalive
        .begin_managed_exact_both_ordinary_authorization(&lease_id.encode())
        .unwrap();
    let mut commits = 0;
    let result = {
        let mut registry = state.output_lease_registry.lock().unwrap();
        submit_show_output_candidate_with_commit(
            state,
            &mut registry,
            &request,
            Some(&authorization),
            now,
            "serial test",
            || {
                commits += 1;
                Ok(true)
            },
        )
    };
    let (_, receipt) = result.expect("successful managed Show route must record its terminal without a BO/Display public identity");
    assert_eq!(commits, 1);
    drop(authorization);
    let journal = state.output_lease_durable_receipts.lock().unwrap();
    assert_eq!(journal.lookup(&request), Ok(Some(receipt)));
    assert!(journal.state.pending.is_empty());
    assert!(journal
        .state
        .managed_exact_both_output_control_pending_terminals
        .is_empty());
    drop(journal);
    for (index, target) in [
        protocol::control_plane_command::OutputControlTargetRoleV1::Lighting,
        protocol::control_plane_command::OutputControlTargetRoleV1::Video,
        protocol::control_plane_command::OutputControlTargetRoleV1::Both,
    ]
    .into_iter()
    .enumerate()
    {
        let blackout = protocol::control_plane_command::OutputControlActionV2::SetBlackout {
            target,
            enabled: true,
            lease: lease.clone(),
        };
        preflight_output_lease_for_control_action(
            state,
            MEDIA_ASSET_A6_OWNER,
            "media-asset-a6",
            1,
            503 + index as u64,
            &blackout,
            0,
        )
        .expect("successful USB activation must not poison subsequent GUI BO admission");
    }
}

#[test]
fn failed_managed_show_route_aborts_only_its_unapplied_pending_receipt() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let (lease_id, generation) = arm_managed(state);
    let action = protocol::control_plane_command::OutputControlActionV2::EnableShowSerialDmxSafetyBlackoutRoute {
        lease: protocol::control_plane_command::OutputLeaseAuthorityV1 { lease_id: lease_id.encode(), generation },
    };
    let (request, now) = build_output_lease_authorization_request(
        state,
        MEDIA_ASSET_A6_OWNER,
        "media-asset-a6",
        1,
        502,
        &action,
        0,
    )
    .unwrap();
    let authorization = state
        .output_lease_keepalive
        .begin_managed_exact_both_ordinary_authorization(&lease_id.encode())
        .unwrap();
    let mut registry = state.output_lease_registry.lock().unwrap();
    let failure = submit_show_output_candidate_with_commit::<(), _>(
        state,
        &mut registry,
        &request,
        Some(&authorization),
        now,
        "serial test",
        || Err("device did not open".into()),
    )
    .unwrap_err();
    assert!(failure.contains("device did not open"));
    let journal = state.output_lease_durable_receipts.lock().unwrap();
    assert!(journal.state.pending.is_empty());
    assert!(
        journal.lookup(&request).is_err(),
        "failed request remains non-replayable; no fake success receipt"
    );
}
