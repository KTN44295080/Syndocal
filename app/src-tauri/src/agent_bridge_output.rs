//! Compatibility intent becomes one canonical native command; no GUI retry or consent path.
use super::{
    agent_bridge::{AgentBridge, AgentBridgeDispatch},
    native_adapter_error::NativeAdapterError,
    AppState, ControlPlaneQueryState,
};
use protocol::control_plane_command::{
    OutputControlActionV2, OutputControlCommandRequestV2, OutputControlErrorCodeV2,
    OutputControlResponseV2, OutputControlTargetRoleV1, OutputLeaseAuthorityQueryStatusV1,
    MAX_SAFE_JAVASCRIPT_INTEGER,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::Manager;

pub(crate) fn authorize(
    app: &tauri::AppHandle,
    dispatch: &AgentBridgeDispatch,
) -> Result<(), OutputControlErrorCodeV2> {
    app.state::<AgentBridge>()
        .authority("main")
        .and_then(|authority| {
            authority.authorize_bridge_request(
                &dispatch.principal_id,
                dispatch.principal_incarnation,
                &dispatch.method,
                &dispatch.params,
                &dispatch.request_id,
            )
        })
        .map_err(|code| {
            if matches!(
                code.as_str(),
                "agent_authority_state_poisoned" | "agent_credential_store_poisoned"
            ) {
                OutputControlErrorCodeV2::Internal
            } else {
                OutputControlErrorCodeV2::Forbidden
            }
        })
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ExpectedProject {
    project_epoch: u64,
    project_revision: u64,
    checkpoint_hash: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct VideoBlackoutIntent {
    enabled: bool,
    expected_project: ExpectedProject,
}

fn parse(params: Value) -> Result<VideoBlackoutIntent, NativeAdapterError> {
    let value: VideoBlackoutIntent =
        serde_json::from_value(params).map_err(|_| "agent_video_blackout_arguments_invalid")?;
    let p = &value.expected_project;
    if p.project_epoch > MAX_SAFE_JAVASCRIPT_INTEGER
        || p.project_revision > MAX_SAFE_JAVASCRIPT_INTEGER
        || p.checkpoint_hash.len() != 64
        || !p
            .checkpoint_hash
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
    {
        return Err("agent_video_blackout_arguments_invalid".into());
    }
    Ok(value)
}

pub(crate) const RECEIPT_DOMAIN: &str = "external-video-blackout";
pub(crate) const MANAGED_RECEIPT_DOMAIN: &str = "external-video-blackout-managed";

pub(crate) fn receipt_domain_matches_operation(domain: &str, operation: &str) -> bool {
    domain == super::OUTPUT_LEASE_OUTPUT_CONTROL_DOMAIN
        || domain == RECEIPT_DOMAIN && operation == "syndocal.output.blackout.set.v2"
}

fn request_id(state: &AppState) -> Result<u64, NativeAdapterError> {
    // The immutable broker UUID owns replay. The domain request must be monotonic,
    // and a native range keeps its public receipts distinct from GUI counters.
    state
        .next_output_lease_request_id
        .fetch_max(1_u64 << 52, std::sync::atomic::Ordering::AcqRel);
    super::allocate_output_lease_request_id(state)
        .map_err(NativeAdapterError::from)
        .and_then(|id| {
            if id <= MAX_SAFE_JAVASCRIPT_INTEGER {
                Ok(id)
            } else {
                Err("agent_video_blackout_request_identity_exhausted".into())
            }
        })
}

pub(crate) fn scope_receipt(
    request: &mut super::output_lease::OutputLeaseRequest,
) -> Result<(), String> {
    request.key = super::output_lease::OutputLeaseRequestKey::new(
        request.key.principal.clone(),
        RECEIPT_DOMAIN,
        request.key.request_id,
    )
    .map_err(|error| format!("Native Video BO receipt identity is invalid: {error:?}"))?;
    Ok(())
}

fn prepare(
    label: &str,
    state: &AppState,
    query: &ControlPlaneQueryState,
    intent: &VideoBlackoutIntent,
) -> Result<OutputControlCommandRequestV2, NativeAdapterError> {
    let authority = super::control_plane_runtime::issue_output_control_authority_for_window_label(
        label, state, query,
    )?;
    let fence = authority.fence;
    let expected = &intent.expected_project;
    if fence.project_epoch != expected.project_epoch
        || fence.project_revision != expected.project_revision
        || fence.project_checkpoint_hash != expected.checkpoint_hash
    {
        return Err("agent_video_blackout_project_changed".into());
    }
    let leases = query.query_output_lease_authority_for_window(label, state)?;
    let mut matches = leases
        .statuses
        .into_iter()
        .filter_map(|status| match status {
            OutputLeaseAuthorityQueryStatusV1::HeldActive {
                authority,
                resources,
            } if resources
                == [
                    OutputControlTargetRoleV1::Lighting,
                    OutputControlTargetRoleV1::Video,
                ] =>
            {
                Some(authority)
            }
            _ => None,
        });
    let lease = matches
        .next()
        .ok_or("agent_video_blackout_active_both_lease_required")?;
    if matches.next().is_some() {
        return Err("agent_video_blackout_active_both_lease_required".into());
    }
    let action = OutputControlActionV2::SetBlackout {
        target: OutputControlTargetRoleV1::Video,
        enabled: intent.enabled,
        lease,
    };
    let request = OutputControlCommandRequestV2 {
        operation_id: action.operation_id().into(),
        request_id: request_id(state)?,
        expected_fence: fence,
        action,
    };
    request
        .validate()
        .map_err(|_| "agent_video_blackout_arguments_invalid")?;
    Ok(request)
}

pub(crate) fn execute_video_blackout(
    app: &tauri::AppHandle,
    window: &tauri::WebviewWindow,
    dispatch: &AgentBridgeDispatch,
) -> Result<Value, NativeAdapterError> {
    if dispatch.method != "output.set_video_blackout" {
        return Err("native_operation_not_supported".into());
    }
    let intent = parse(dispatch.params.clone())?;
    let state = app.state::<AppState>();
    let query = app.state::<ControlPlaneQueryState>();
    let request = prepare(window.label(), &state, &query, &intent)?;
    let response = super::control_plane_runtime::execute_external_video_blackout(
        app,
        window,
        &state,
        &query,
        request,
        &|| authorize(app, dispatch),
    );
    let receipt = match response {
        OutputControlResponseV2::Receipt(receipt) => receipt,
        rejected => {
            return Ok(
                json!({"ok":false,"operation_id":"syndocal.output.blackout.set.v2","result":rejected}),
            )
        }
    };
    // Confirm the actual persisted target under the canonical receipt's exact successor.
    let _admission = super::lock_project_external_command_admission(&state)?;
    let mut coordinator = super::lock_project_coordinator(&state)?;
    super::ensure_no_pending_project_transaction(&coordinator)?;
    super::reconcile_project_checkpoint_for_coordinator(&state, &mut coordinator)?;
    if !super::control_plane_runtime::exact_output_control_fence_matches(
        &state,
        &coordinator,
        &receipt.fence_after,
    ) {
        return Err("agent_video_blackout_verification_failed".into());
    }
    let bundle = super::project_authority_bundle_from_coordinator(&state, &coordinator);
    if bundle.snapshot.video.blackout != intent.enabled {
        return Err("agent_video_blackout_verification_failed".into());
    }
    Ok(
        json!({"ok":true,"project":{"project_epoch":bundle.project_epoch,"project_revision":bundle.project_revision,
        "checkpoint_hash":bundle.checkpoint_hash},"video":{"blackout":bundle.snapshot.video.blackout,
        "authored_blackout":bundle.snapshot.video.blackout,"safety_blackout_engaged":bundle.snapshot.safety_blackout_engaged},
        "receipt":receipt,"verification":"committed_project_state"}),
    )
}

#[cfg(test)]
#[path = "agent_bridge_output_guard_tests.rs"]
mod agent_bridge_output_guard_tests;
