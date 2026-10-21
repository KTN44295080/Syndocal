//! Project-qualified observations for ordinary video output authoring.
//! Expired claims remain visible without changing their generation or index.
use super::*;

#[cfg(test)]
#[path = "video_authoring_lease_native_tests.rs"]
mod tests;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct VideoAuthoringLeaseAuthority {
    operation_id: &'static str,
    fence: OutputControlFenceV1,
    status: &'static str,
    authority: Option<protocol::control_plane_command::OutputLeaseAuthorityV1>,
    resources: Vec<protocol::control_plane_command::OutputControlTargetRoleV1>,
    project_matches: Option<bool>,
    managed: bool,
}

pub(crate) fn query_for_window(
    state: &AppState, query: &ControlPlaneQueryState, label: &str,
) -> Result<VideoAuthoringLeaseAuthority, String> {
    state.ensure_window_authority_not_blocked(label)?;
    let fence = query.issue_output_control_fence_for_window(label, state)
        .map_err(|error| format!("Video authoring authority is unavailable: {error:?}"))?;
    let _owner_rotation = state.project_transaction_owner_rotation.try_lock()
        .map_err(|_| "Video authoring authority is busy".to_string())?;
    let registry = state.output_lease_registry.try_lock()
        .map_err(|_| "Video authoring authority registry is busy".to_string())?;
    let coordinator = state.project_coordinator.try_lock()
        .map_err(|_| "Video authoring project is busy".to_string())?;
    if state.project_transaction_active.load(Ordering::Acquire)
        || !control_plane_runtime::exact_output_control_fence_matches(state, &coordinator, &fence) {
        return Err("Video authoring project authority changed; refresh before editing".to_string());
    }
    let principal = state.project_transaction_owners.lock()
        .map_err(|_| "Video authoring owner is unavailable".to_string())?
        .get(label).cloned().ok_or_else(|| "Video authoring window is not registered".to_string())?;
    let incarnation = *state.project_transaction_owner_incarnations.lock()
        .map_err(|_| "Video authoring owner incarnation is unavailable".to_string())?
        .get(label).ok_or_else(|| "Video authoring owner is unavailable".to_string())?;
    if registry.process_session_incarnation() != query.process_incarnation() {
        return Err("Video authoring process authority is stale".to_string());
    }
    let owner = OutputLeaseOwner::new(principal, label.to_string(), query.process_incarnation(), incarnation)
        .map_err(|error| format!("Video authoring owner is invalid: {error:?}"))?;
    let now = state.output_lease_now_ms()?;
    let view = registry.query_video_authoring_lease(&owner, now)
        .map_err(|error| format!("Video authoring lease observation failed: {error:?}"))?;
    let mut response = VideoAuthoringLeaseAuthority {
        operation_id: "syndocal.query.output.video.authoring.lease.v1", fence,
        status: "unavailable", authority: None, resources: Vec::new(), project_matches: None, managed: false,
    };
    if let Some(view) = view {
        response.status = match view.snapshot.phase {
            output_lease::OutputLeasePhase::HeldActive if view.snapshot.expires_at_monotonic_ms.is_some_and(|deadline| now >= deadline) => "expired",
            output_lease::OutputLeasePhase::HeldActive => "held_active",
            output_lease::OutputLeasePhase::HeldOrphaned => "held_orphaned",
            _ => return Err("Video authoring lease observation is inconsistent".to_string()),
        };
        let lease_id = view.lease_id.encode();
        response.project_matches = Some(view.project_identity == output_lease_project_identity(coordinator.epoch));
        response.resources = view.snapshot.resources.as_ref()
            .ok_or_else(|| "Video authoring lease scope is unavailable".to_string())?.as_slice().iter()
            .map(|resource| match resource {
                output_lease::OutputLeaseResource::Lighting => protocol::control_plane_command::OutputControlTargetRoleV1::Lighting,
                output_lease::OutputLeaseResource::Video => protocol::control_plane_command::OutputControlTargetRoleV1::Video,
            }).collect();
        response.authority = Some(protocol::control_plane_command::OutputLeaseAuthorityV1 {
            lease_id, generation: view.snapshot.generation,
        });
    }
    // Never wait for the manager while retaining registry/coordinator locks.
    drop(coordinator);
    drop(registry);
    drop(_owner_rotation);
    if let Some(authority) = response.authority.as_ref() {
        response.managed = state.output_lease_keepalive.manages_lease_id(&authority.lease_id)?;
    }
    query.validate_output_control_fence_window(label, &response.fence, incarnation)
        .map_err(|_| "Video authoring authority changed during observation".to_string())?;
    Ok(response)
}

pub(crate) fn submit_replacement(
    state: &AppState, coordinator: &ProjectCoordinator, request: &OutputLeaseRequest, fence: &OutputControlFenceV1,
) -> Result<output_lease::OutputLeaseRequestReceipt, output_lease::OutputLeaseError> {
    use output_lease::OutputLeaseError as Error;
    let Some(output_lease::OutputLeaseRequestAction::ReplacePreviousProjectForAuthoring { lease_id, project_identity, .. }) = request.action.as_ref() else {
        return Err(Error::InvalidRequest);
    };
    let _transition = lock_output_ownership_transition(state).map_err(|_| Error::Busy)?;
    if state.project_transaction_active.load(Ordering::Acquire)
        || !control_plane_runtime::exact_output_control_fence_matches(state, coordinator, fence)
        || ensure_no_pending_project_transaction(coordinator).is_err()
        || project_identity != &output_lease_project_identity(fence.project_epoch)
        || state.output_lease_keepalive.try_manages_lease_id(&lease_id.encode()).map_err(|_| Error::Busy)? {
        return Err(Error::Busy);
    }
    // An idle permit count alone cannot discard a paused physical owner.
    // Pending SDK-only teardown is independently counted by the Engine gate.
    if !output_owner_retirement::stop_signals(state).map_err(|_| Error::Busy)?.is_empty() {
        return Err(Error::Busy);
    }
    let mut registry = state.output_lease_registry.lock().map_err(|_| Error::RequestCapacity)?;
    let now = state.output_lease_now_ms().map_err(|_| Error::ClockRollback)?;
    let mut durable = state.output_lease_durable_receipts.lock().map_err(|_| Error::RequestCapacity)?;
    if let Some(receipt) = durable.lookup(request)? { return Ok(receipt); }
    // Public control fences reserve zero and project Engine counters as +1.
    // Preserve the exact projection when entering the locked Engine boundary.
    let output_epoch = fence.output_epoch.checked_sub(1).ok_or(Error::InvalidRequest)?;
    let output_generation = fence.output_generation.checked_sub(1).ok_or(Error::InvalidRequest)?;
    state.engine.with_retired_output_ownership(output_epoch, output_generation, || {
        registry.submit_request_with_commit(request, now, |receipt| durable.record(receipt))
    }).map_err(|_| Error::Busy)?
}

#[tauri::command]
pub(crate) async fn query_video_authoring_lease_authority_v1(
    app: tauri::AppHandle, window: WebviewWindow,
) -> Result<VideoAuthoringLeaseAuthority, String> {
    let label = window.label().to_string();
    tauri::async_runtime::spawn_blocking(move || {
        query_for_window(&app.state::<AppState>(), &app.state::<ControlPlaneQueryState>(), &label)
    }).await.map_err(|error| format!("Video authoring authority query worker failed: {error}"))?
}
