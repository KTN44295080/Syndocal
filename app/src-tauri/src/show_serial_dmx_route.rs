//! Machine-local Enttec Open DMX route orchestration.
//!
//! This deliberately stays outside the authored `.sdc` model: a generic FTDI
//! adapter is never inferred as Open DMX, and COM/PnP identity is confirmed
//! and retained only in the machine-local binding store.

use super::*;

/// The engine emits this only for a real Open DMX route-state transition. Its
/// payload is intentionally just the process-local revision fence; the UI
/// immediately invalidates to Unknown and rereads both authoritative endpoints.
pub(super) const SHOW_SERIAL_DMX_ROUTE_STATUS_EVENT: &str =
    "syndocal://show-serial-dmx-route-status-v1";

pub(super) fn serial_dmx_machine_binding_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|directory| serial_dmx_machine::serial_dmx_machine_binding_path(&directory))
        .map_err(|error| format!("Unable to resolve machine-local USB-DMX selection path: {error}"))
}

/// The binding and route status endpoints are read separately by Tauri. This
/// fence is captured after the binding read, so a concurrent route transition
/// yields mismatched revisions rather than a fabricated coherent Active pair.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct SerialDmxMachineBindingStatusWithRouteRevisionV1 {
    state: serial_dmx_machine::SerialDmxMachineBindingStateV1,
    selected: Option<serial_dmx_machine::SerialDmxMachineBindingIdentityV1>,
    detail: String,
    route_status_revision: String,
}

impl SerialDmxMachineBindingStatusWithRouteRevisionV1 {
    fn from_status(
        status: serial_dmx_machine::SerialDmxMachineBindingStatusV1,
        route_status_revision: u64,
    ) -> Self {
        Self {
            state: status.state,
            selected: status.selected,
            detail: status.detail,
            route_status_revision: route_status_revision.to_string(),
        }
    }
}

/// Read-only host-local binding status. This does not read or write `.sdc`.
pub(super) fn get_machine_binding_status(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<SerialDmxMachineBindingStatusWithRouteRevisionV1, String> {
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
    let binding = serial_dmx_machine::binding_status_from_path(
        &serial_dmx_machine_binding_path(app)?,
        &ports,
    );
    let route = state
        .engine
        .try_show_serial_dmx_safety_blackout_route_status_snapshot()
        .map_err(|error| {
            format!("USB-DMX route status is unavailable; binding remains fail-closed: {error}")
        })?;
    Ok(SerialDmxMachineBindingStatusWithRouteRevisionV1::from_status(binding, route.revision))
}

/// The explicit Confirm step for a generic serial adapter. It can bind only
/// one currently enumerated full identity; it never infers a protocol from
/// FTDI VID/PID and never writes a project file.
pub(super) fn select_machine_binding(
    state: &AppState,
    app: &tauri::AppHandle,
    request: serial_dmx_machine::SelectSerialDmxMachineBindingRequestV1,
) -> Result<SerialDmxMachineBindingStatusWithRouteRevisionV1, String> {
    // Match the route lifecycle order: lifecycle -> external admission ->
    // coordinator -> output transition. The write and both safety checks stay
    // inside this interval, so activation cannot swap A/B around persistence.
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before USB-DMX binding confirmation"
            .to_string()
    })?;
    let _external_admission = super::lock_project_external_command_admission(state)?;
    let mut coordinator = super::lock_project_coordinator(state)?;
    if super::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || super::ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        return Err(
            "USB-DMX binding confirmation is unavailable while project/output authority is not coherent"
                .to_string(),
        );
    }
    let _transition_guard = super::lock_output_ownership_transition(state)?;
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
    let (binding, route) = select_machine_binding_from_ports_with_route_status(
        &serial_dmx_machine_binding_path(app)?,
        &request,
        &ports,
        || {
            let snapshot = state
                .engine
                .try_show_serial_dmx_safety_blackout_route_status_snapshot()
                .map_err(|error| {
                    format!(
                        "USB-DMX route status is unavailable; binding mutation was rejected fail-closed: {error}"
                    )
                })?;
            let safety_authority_engaged = state
                .engine
                .try_safety_blackout_authority()
                .map_err(|error| {
                    format!(
                        "USB-DMX S0 authority is unavailable; binding mutation was rejected fail-closed: {error}"
                    )
                })?
                .engaged;
            let open_dmx_safety_latched = state
                .engine
                .open_dmx_safety_blackout_latched()
                .map_err(|error| {
                    format!(
                        "USB-DMX physical S0 latch is unavailable; binding mutation was rejected fail-closed: {error}"
                    )
                })?;
            Ok(MachineBindingRouteAdmission {
                snapshot,
                safety_authority_engaged,
                open_dmx_safety_latched,
            })
        },
    )?;
    Ok(SerialDmxMachineBindingStatusWithRouteRevisionV1::from_status(binding, route.revision))
}

#[derive(Debug, Clone)]
pub(super) struct MachineBindingRouteAdmission {
    pub snapshot: engine::ShowSerialDmxRouteStatusSnapshot,
    pub safety_authority_engaged: bool,
    pub open_dmx_safety_latched: bool,
}

fn ensure_machine_binding_mutation_admissible(
    admission: &MachineBindingRouteAdmission,
) -> Result<(), String> {
    let route = &admission.snapshot.status;
    if route.active || route.live_frame_queued {
        return Err(
            "USB-DMX machine-local binding cannot change while an Open DMX worker is active; stop it under S0 and wait for its physical zero receipt"
                .to_string(),
        );
    }
    if !route.worker_shutdown_completed {
        return Err(
            "USB-DMX machine-local binding cannot change until the previous worker has completed bounded shutdown"
                .to_string(),
        );
    }
    // Once a faulted worker has joined, a missing old-A zero receipt is not an
    // in-flight transition. Keeping it as an absolute reject would deadlock a
    // disconnect recovery before explicit B selection can perform its own
    // initial physical S0 write. A non-faulted inconsistency remains closed.
    if route.zero_frame_queued && !route.zero_frame_physical_write_completed && !route.faulted {
        return Err(
            "USB-DMX machine-local binding cannot change while the S0 physical zero transaction is incomplete"
                .to_string(),
        );
    }
    if !route.faulted {
        return Ok(());
    }
    // Fault recovery changes only the host-local expectation. It emits no
    // bytes and does not clear the sticky fault: a fresh exact B re-arm must
    // independently open/revalidate/zero-write before Release Blackout can
    // succeed. The latch and stopped worker make this limited recovery safe.
    if !admission.safety_authority_engaged || !admission.open_dmx_safety_latched {
        return Err(
            "USB-DMX fault recovery binding requires both logical and physical S0 latches to remain engaged"
                .to_string(),
        );
    }
    Ok(())
}

/// Keep a failed post-persist admission from changing the retained machine
/// binding. The supplied route snapshot must be coherent; an error/poison is
/// treated as an unsafe mutation and the exact previous bytes are restored.
pub(super) fn select_machine_binding_from_ports_with_route_status<F>(
    path: &Path,
    request: &serial_dmx_machine::SelectSerialDmxMachineBindingRequestV1,
    ports: &[SerialPortSummary],
    mut route_status: F,
) -> Result<
    (
        serial_dmx_machine::SerialDmxMachineBindingStatusV1,
        engine::ShowSerialDmxRouteStatusSnapshot,
    ),
    String,
>
where
    F: FnMut() -> Result<MachineBindingRouteAdmission, String>,
{
    ensure_machine_binding_mutation_admissible(&route_status()?)?;
    // Recheck immediately before persisting. Enumeration can be slow and the
    // first answer never authorizes the eventual local write by itself.
    ensure_machine_binding_mutation_admissible(&route_status()?)?;
    let previous = serial_dmx_machine::capture_binding_file_snapshot(path)?;
    let selected = serial_dmx_machine::select_binding_from_ports(path, request, ports)?;
    let post = route_status().and_then(|admission| {
        ensure_machine_binding_mutation_admissible(&admission)?;
        Ok(admission.snapshot)
    });
    match post {
        Ok(route) => Ok((selected, route)),
        Err(admission_error) => {
            serial_dmx_machine::restore_binding_file_snapshot(path, &previous).map_err(
                |restore_error| {
                    format!(
                        "USB-DMX binding admission changed after persistence ({admission_error}); exact previous binding restoration also failed: {restore_error}. Binding remains blocked."
                    )
                },
            )?;
            Err(format!(
                "USB-DMX binding admission changed after persistence ({admission_error}); the exact previous machine-local binding was restored and no replacement was retained"
            ))
        }
    }
}

/// Read-only worker state. Frame queue fields are intentionally not physical
/// fixture/wire acceptance claims.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ShowSerialDmxSafetyBlackoutRouteStatusV1 {
    route_status_revision: String,
    active: bool,
    zero_frame_queued: bool,
    zero_frame_physical_write_completed: bool,
    live_frame_queued: bool,
    worker_shutdown_completed: bool,
    faulted: bool,
    artnet_mirror_live: bool,
    artnet_mirror_detail: String,
    detail: String,
}

impl ShowSerialDmxSafetyBlackoutRouteStatusV1 {
    fn from_snapshot(snapshot: engine::ShowSerialDmxRouteStatusSnapshot) -> Self {
        let status = snapshot.status;
        Self {
            route_status_revision: snapshot.revision.to_string(),
            active: status.active,
            zero_frame_queued: status.zero_frame_queued,
            zero_frame_physical_write_completed: status.zero_frame_physical_write_completed,
            live_frame_queued: status.live_frame_queued,
            worker_shutdown_completed: status.worker_shutdown_completed,
            faulted: status.faulted,
            artnet_mirror_live: status.artnet_mirror_live,
            artnet_mirror_detail: status.artnet_mirror_detail,
            detail: status.detail,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct ShowSerialDmxRouteStatusEventV1 {
    route_status_revision: String,
}

pub(super) fn route_status_event(
    snapshot: engine::ShowSerialDmxRouteStatusSnapshot,
) -> ShowSerialDmxRouteStatusEventV1 {
    ShowSerialDmxRouteStatusEventV1 {
        route_status_revision: snapshot.revision.to_string(),
    }
}

pub(super) fn route_status(
    state: &AppState,
) -> Result<ShowSerialDmxSafetyBlackoutRouteStatusV1, String> {
    let snapshot = state
        .engine
        .try_show_serial_dmx_safety_blackout_route_status_snapshot()
        .map_err(|error| {
            format!("USB-DMX route status is unavailable and remains fail-closed: {error}")
        })?;
    Ok(ShowSerialDmxSafetyBlackoutRouteStatusV1::from_snapshot(
        snapshot,
    ))
}

/// USB-DMX is a physical mirror, not a substitute for the fixed Unity route.
/// Keep this check local to the USB orchestration module so `main.rs` need not
/// grow a second output-control surface.
pub(super) fn is_exact_enabled_show_artnet_loopback_route(route: &DmxOutputConfig) -> bool {
    route.enabled
        && route.protocol == DmxOutputProtocol::ArtNet
        && route.target_ip
            == show_artnet_acceptance_probe::DSF2026_ARTNET_ACCEPTANCE_PROBE_TARGET_IP
        && route.port == show_artnet_acceptance_probe::DSF2026_ARTNET_ACCEPTANCE_PROBE_PORT
        && route.universe == show_artnet_acceptance_probe::DSF2026_ARTNET_ACCEPTANCE_PROBE_UNIVERSE
        && route.serial_port.is_empty()
}

fn validate_current_enabled_show_artnet_loopback_route(state: &AppState) -> Result<(), String> {
    let (dmx_outputs, output) = state.engine.dmx_outputs_and_output_snapshot();
    let [route] = dmx_outputs.as_slice() else {
        return Err(
            "Show serial DMX activation requires exactly one enabled Art-Net Unity mirror route"
                .to_string(),
        );
    };
    if output != *route || !is_exact_enabled_show_artnet_loopback_route(route) {
        return Err(
            "Show serial DMX activation requires the exact enabled Art-Net 127.0.0.1:6454/U0 Unity mirror"
                .to_string(),
        );
    }
    Ok(())
}

pub(super) async fn enable_command(
    app: tauri::AppHandle,
    window: WebviewWindow,
    request: OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    super::execute_output_control_off_event_loop(
        app,
        window,
        OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_ENABLE_OPERATION_ID,
        request,
    )
    .await
}

pub(super) async fn stop_command(
    app: tauri::AppHandle,
    window: WebviewWindow,
    request: OutputControlCommandRequestV2,
) -> OutputControlResponseV2 {
    super::execute_output_control_off_event_loop(
        app,
        window,
        OUTPUT_SHOW_SERIAL_DMX_SAFETY_BLACKOUT_ROUTE_STOP_OPERATION_ID,
        request,
    )
    .await
}

pub(super) fn validate_exact_show_serial_dmx_device_matches(
    ports: &[SerialPortSummary],
    selected: &serial_dmx_machine::SerialDmxMachineBindingIdentityV1,
) -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String> {
    validate_exact_show_serial_dmx_device_matches_with_capture(ports, selected, |port| {
        io::serial_dmx::VerifiedUsbSerialPortIdentity::from_summary_with_windows_com_binding(port)
            .map_err(|error| format!("Show serial DMX device identity is incomplete: {error}"))
    })
}

pub(super) fn validate_exact_show_serial_dmx_device_matches_with_capture<F>(
    ports: &[SerialPortSummary],
    selected: &serial_dmx_machine::SerialDmxMachineBindingIdentityV1,
    capture_windows_identity: F,
) -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String>
where
    F: FnOnce(&SerialPortSummary) -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String>,
{
    let matches = ports
        .iter()
        .filter(|port| selected.matches_summary(port))
        .collect::<Vec<_>>();
    let [port] = matches.as_slice() else {
        return Err(if matches.is_empty() {
            "Selected machine-local USB-DMX interface is absent or stale; refresh and explicitly reselect it. No interface was substituted.".to_string()
        } else {
            "Selected machine-local USB-DMX interface is ambiguous; output remains disabled until explicit reselect.".to_string()
        });
    };
    let identity = capture_windows_identity(port)?;
    if identity.port_name != selected.port_name
        || identity.port_type != selected.port_type
        || identity.usb_vid != selected.usb_vid
        || identity.usb_pid != selected.usb_pid
        || identity.serial_number != selected.serial_number
        || identity.manufacturer != selected.manufacturer
        || identity.product != selected.product
        || identity.windows_device_instance_id.as_deref()
            != Some(selected.windows_device_instance_id.as_str())
    {
        return Err(format!(
            "Show serial DMX hardware identity changed after selection; expected {}, observed {:?}. Output remains disabled.",
            selected.label(), identity
        ));
    }
    Ok(identity)
}

fn verify_current_show_serial_dmx_device(
    app: &tauri::AppHandle,
) -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String> {
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("Show serial DMX device enumeration failed: {error}"))?;
    let selected = serial_dmx_machine::resolve_selected_identity_from_path(
        &serial_dmx_machine_binding_path(app)?,
        &ports,
    )?;
    validate_exact_show_serial_dmx_device_matches(&ports, &selected)
}

/// The local R4 path for the fixed runtime-only Open DMX worker. It accepts
/// neither COM/PnP data nor DMX values on the control plane: the selection is
/// reread from its separate local store and then recaptured at final commit.
pub(super) fn enable_with_output_control_fence(
    app: &tauri::AppHandle,
    state: &AppState,
    expected_fence: &OutputControlFenceV1,
    lease_request: &output_lease::OutputLeaseRequest,
    _lease_now_ms: u64,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before show serial DMX activation"
            .to_string()
    })?;
    let _external_admission = super::lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(super::lock_project_coordinator(state)?);
    super::with_revalidated_output_transition(
        || super::lock_output_ownership_transition(state),
        || {
            {
                let mut coordinator = coordinator.borrow_mut();
                if super::reconcile_project_checkpoint_for_coordinator(state, &mut *coordinator)
                    .is_err()
                    || !control_plane_runtime::exact_output_control_fence_matches(
                        state,
                        &*coordinator,
                        expected_fence,
                    )
                    || super::ensure_no_pending_project_transaction(&*coordinator).is_err()
                {
                    return Err(
                        "Output control fence changed before show serial DMX activation"
                            .to_string(),
                    );
                }
            }
            let ownership = state.engine.output_ownership_status();
            if ownership.state != protocol::OutputOwnershipState::Ready
                || ownership.effective_role != MachineOutputRole::Both
                || ownership.desired_role != MachineOutputRole::Both
                || !ownership.lighting_allowed
            {
                return Err(
                    "Show serial DMX activation requires the active local Both output authority"
                        .to_string(),
                );
            }
            let safety = state.engine.safety_blackout_authority();
            if !safety.engaged
                || safety.epoch != expected_fence.safety_blackout_epoch
                || safety.generation != expected_fence.safety_blackout_generation
            {
                return Err("Show serial DMX activation requires the exact currently-engaged S0 safety blackout authority".to_string());
            }
            validate_current_enabled_show_artnet_loopback_route(state)?;
            verify_current_show_serial_dmx_device(app)
        },
        |_transition_guard, device| {
            // Keep a managed exact-Both guard through the complete candidate
            // and bounded worker callback.  The generic candidate seam
            // rejects its private action without this opaque capability.
            let managed_authorization = lease_request
                .action
                .as_ref()
                .and_then(output_lease::OutputLeaseRequestAction::managed_exact_both_lease_id)
                .map(|lease_id| {
                    state
                        .output_lease_keepalive
                        .begin_managed_exact_both_ordinary_authorization(&lease_id.encode())
                })
                .transpose()?;
            let mut lease_registry = state.output_lease_registry.lock().map_err(|_| {
                "Output lease registry lock was poisoned before show serial DMX activation"
                    .to_string()
            })?;
            let final_lease_now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) = super::submit_show_output_candidate_with_commit(
                state,
                &mut lease_registry,
                lease_request,
                managed_authorization.as_ref(),
                final_lease_now_ms,
                "show serial DMX S0 worker activation",
                || {
                    {
                        let mut coordinator = coordinator.borrow_mut();
                        if super::reconcile_project_checkpoint_for_coordinator(
                            state,
                            &mut *coordinator,
                        )
                        .is_err()
                            || !control_plane_runtime::exact_output_control_fence_matches(
                                state,
                                &*coordinator,
                                expected_fence,
                            )
                            || super::ensure_no_pending_project_transaction(&*coordinator).is_err()
                        {
                            return Err(
                                "Output control fence changed before show serial DMX publication"
                                    .to_string(),
                            );
                        }
                    }
                    let ownership = state.engine.output_ownership_status();
                    if ownership.state != protocol::OutputOwnershipState::Ready
                        || ownership.effective_role != MachineOutputRole::Both
                        || ownership.desired_role != MachineOutputRole::Both
                        || !ownership.lighting_allowed
                    {
                        return Err(
                            "Show serial DMX activation lost its local Both output authority"
                                .to_string(),
                        );
                    }
                    let safety = state.engine.safety_blackout_authority();
                    if !safety.engaged
                        || safety.epoch != expected_fence.safety_blackout_epoch
                        || safety.generation != expected_fence.safety_blackout_generation
                    {
                        return Err("Show serial DMX activation was superseded by an S0 safety blackout authority change".to_string());
                    }
                    validate_current_enabled_show_artnet_loopback_route(state)?;
                    let current_device = verify_current_show_serial_dmx_device(app)?;
                    if current_device != device {
                        return Err(
                            "Show serial DMX machine-local device changed before final activation"
                                .to_string(),
                        );
                    }
                    state
                        .engine
                        .enable_show_serial_dmx_safety_blackout_route(
                            current_device,
                            expected_fence.safety_blackout_epoch,
                            expected_fence.safety_blackout_generation,
                            Instant::now() + Duration::from_secs(2),
                        )
                        .map_err(|error| {
                            format!("Show serial DMX worker activation failed: {error}")
                        })?;
                    Ok(true)
                },
            )?;
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}

/// Safely retire the exact runtime-only worker while the same S0 authority is
/// still held. It never changes the authored Art-Net route or a project file.
pub(super) fn stop_with_output_control_fence(
    state: &AppState,
    expected_fence: &OutputControlFenceV1,
    lease_request: &output_lease::OutputLeaseRequest,
    _lease_now_ms: u64,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before show serial DMX stop"
            .to_string()
    })?;
    let _external_admission = super::lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(super::lock_project_coordinator(state)?);
    super::with_revalidated_output_transition(
        || super::lock_output_ownership_transition(state),
        || {
            let mut coordinator = coordinator.borrow_mut();
            if super::reconcile_project_checkpoint_for_coordinator(state, &mut *coordinator)
                .is_err()
                || !control_plane_runtime::exact_output_control_fence_matches(
                    state,
                    &*coordinator,
                    expected_fence,
                )
                || super::ensure_no_pending_project_transaction(&*coordinator).is_err()
            {
                return Err("Output control fence changed before show serial DMX stop".to_string());
            }
            let safety = state.engine.safety_blackout_authority();
            if !safety.engaged
                || safety.epoch != expected_fence.safety_blackout_epoch
                || safety.generation != expected_fence.safety_blackout_generation
            {
                return Err("Show serial DMX stop requires the exact currently-engaged S0 safety blackout authority".to_string());
            }
            Ok(())
        },
        |_transition_guard, ()| {
            let managed_authorization = lease_request
                .action
                .as_ref()
                .and_then(output_lease::OutputLeaseRequestAction::managed_exact_both_lease_id)
                .map(|lease_id| {
                    state
                        .output_lease_keepalive
                        .begin_managed_exact_both_ordinary_authorization(&lease_id.encode())
                })
                .transpose()?;
            let mut lease_registry = state.output_lease_registry.lock().map_err(|_| {
                "Output lease registry lock was poisoned before show serial DMX stop".to_string()
            })?;
            let final_lease_now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) = super::submit_show_output_candidate_with_commit(
                state,
                &mut lease_registry,
                lease_request,
                managed_authorization.as_ref(),
                final_lease_now_ms,
                "show serial DMX S0 worker stop",
                || {
                    let safety = state.engine.safety_blackout_authority();
                    if !safety.engaged
                        || safety.epoch != expected_fence.safety_blackout_epoch
                        || safety.generation != expected_fence.safety_blackout_generation
                    {
                        return Err("Show serial DMX stop was superseded by an S0 safety blackout authority change".to_string());
                    }
                    state
                        .engine
                        .stop_show_serial_dmx_safety_blackout_route(
                            expected_fence.safety_blackout_epoch,
                            expected_fence.safety_blackout_generation,
                            Instant::now() + Duration::from_secs(2),
                        )
                        .map_err(|error| format!("Show serial DMX worker stop failed: {error}"))?;
                    Ok(true)
                },
            )?;
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}
