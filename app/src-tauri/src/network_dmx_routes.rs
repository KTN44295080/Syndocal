//! Local lease-bound network DMX configuration; USB worker ownership is unchanged.
use super::*;
use protocol::network_dmx_routes::{NetworkDmxRoute, validate_network_dmx_routes};

pub(super) fn validate_current(
    state: &AppState,
    routes: &[NetworkDmxRoute],
) -> Result<Vec<DmxOutputConfig>, String> {
    validate_network_dmx_routes(routes)?;
    let (outputs, primary) = state.engine.dmx_outputs_and_output_snapshot();
    let staged = DmxOutputConfig {
        enabled: false,
        ..primary.clone()
    };
    if !primary.enabled
        || !is_exact_staged_show_artnet_loopback_route(&staged)
        || outputs.first() != Some(&primary)
    {
        return Err("Prepare U0 output before applying network universes".into());
    }
    if outputs.iter().skip(1).any(|r| {
        !matches!(
            r.protocol,
            DmxOutputProtocol::ArtNet | DmxOutputProtocol::Sacn
        )
    }) {
        return Err("Network routes cannot replace a legacy serial device route".into());
    }
    let next = std::iter::once(primary)
        .chain(routes.iter().map(NetworkDmxRoute::config))
        .collect::<Vec<_>>();
    validate_dmx_output_routes(&next)?;
    Ok(outputs)
}

pub(super) fn set_with_output_control_fence(
    state: &AppState,
    routes: &[NetworkDmxRoute],
    expected_fence: &OutputControlFenceV1,
    lease_request: &OutputLeaseRequest,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    let _lifecycle = state
        .standby_sync_lifecycle
        .lock()
        .map_err(|_| "Standby lifecycle lock poisoned")?;
    let _external = lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(lock_project_coordinator(state)?);
    let revalidate = || -> Result<Vec<DmxOutputConfig>, String> {
        let mut c = coordinator.borrow_mut();
        reconcile_project_checkpoint_for_coordinator(state, &mut *c)?;
        if !control_plane_runtime::exact_output_control_fence_matches(state, &*c, expected_fence) {
            return Err("Project or output authority changed before DMX route publication".into());
        }
        ensure_no_pending_project_transaction(&*c)?;
        let ownership = state.engine.output_ownership_status();
        if ownership.state != protocol::OutputOwnershipState::Ready
            || ownership.effective_role != MachineOutputRole::Both
            || ownership.desired_role != MachineOutputRole::Both
            || !ownership.lighting_allowed
        {
            return Err("Network DMX configuration requires active Both output authority".into());
        }
        let safety = state.engine.safety_blackout_authority();
        if safety.engaged
            || safety.epoch != expected_fence.safety_blackout_epoch
            || safety.generation != expected_fence.safety_blackout_generation
        {
            return Err("Safety authority changed before network DMX configuration".into());
        }
        validate_current(state, routes)
    };
    with_revalidated_output_transition(
        || lock_output_ownership_transition(state),
        &revalidate,
        |_guard, prior| {
            let authorization = lease_request
                .action
                .as_ref()
                .and_then(OutputLeaseRequestAction::managed_exact_both_lease_id)
                .map(|id| {
                    state
                        .output_lease_keepalive
                        .begin_managed_exact_both_ordinary_authorization(&id.encode())
                })
                .transpose()?;
            let mut registry = state
                .output_lease_registry
                .lock()
                .map_err(|_| "Output lease registry poisoned")?;
            let now = state.output_lease_now_ms()?;
            let commit = || {
                if revalidate()? != prior {
                    return Err("DMX routes changed before final publication".into());
                }
                let next = std::iter::once(prior[0].clone())
                    .chain(routes.iter().map(NetworkDmxRoute::config))
                    .collect::<Vec<_>>();
                if next == prior {
                    return Ok(false);
                }
                state.engine.set_network_dmx_routes_published(
                    prior.clone(),
                    routes.to_vec(),
                    expected_fence.safety_blackout_epoch,
                    expected_fence.safety_blackout_generation,
                    Instant::now() + Duration::from_secs(2),
                )?;
                Ok(true)
            };
            let (applied, receipt) = if let Some(authorization) = authorization.as_ref() {
                submit_managed_exact_both_candidate_with_commit(
                    state,
                    &mut registry,
                    lease_request,
                    authorization,
                    now,
                    "network DMX route configuration",
                    None,
                    commit,
                )?
            } else {
                submit_output_lease_candidate_with_commit(
                    state,
                    &mut registry,
                    lease_request,
                    now,
                    "network DMX route configuration",
                    commit,
                )?
            };
            let after = if applied {
                let mut c = coordinator.borrow_mut();
                let checkpoint = reconcile_project_checkpoint_for_coordinator(state, &mut *c)?;
                let safety = state.engine.safety_blackout_authority();
                control_plane_runtime::committed_output_control_fence(
                    expected_fence,
                    control_plane_runtime::CommittedOutputControlFenceValues {
                        project_epoch: c.epoch,
                        project_revision: checkpoint.revision,
                        project_checkpoint_hash: &checkpoint.hash,
                        project_publication_generation: c.publication_generation,
                        output_epoch: expected_fence.output_epoch,
                        output_generation: expected_fence.output_generation,
                        safety_blackout_epoch: safety.epoch,
                        safety_blackout_generation: safety.generation,
                    },
                )
            } else {
                expected_fence.clone()
            };
            Ok((applied, after, receipt))
        },
    )
}
