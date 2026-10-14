//! Machine-local USB-DMX cables for universes other than Universe 0.
//!
//! Each binding maps one universe to one exact cable identity in its own
//! machine-local file; projects never carry COM/PnP identity. The Universe 0
//! cable keeps its own binding (`serial_dmx_machine`) and Unity-mirror route.
//! Starting or stopping a cable goes through the OutputControl lease/fence
//! like the Universe 0 cable, and the engine keeps every cable zero-first
//! and S0-bound.

use super::*;
use serial_dmx_machine::{SerialDmxMachineBindingIdentityV2, SerialDmxMachineBindingStateV1};

pub const USB_DMX_UNIVERSE_BINDINGS_FILE: &str = "usb-dmx-universe-bindings-v1.json";
const USB_DMX_UNIVERSE_BINDINGS_VERSION: u32 = 1;
const MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES: u64 = 8 * 1024;
const MAX_USB_DMX_CABLE_UNIVERSE: u16 = 32_767;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbDmxUniverseBindingV1 {
    pub universe: u16,
    pub identity: SerialDmxMachineBindingIdentityV2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UsbDmxUniverseBindingsFileV1 {
    version: u32,
    universes: Vec<UsbDmxUniverseBindingV1>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectUsbDmxUniverseBindingRequestV1 {
    pub universe: u16,
    pub port_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub windows_device_instance_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub macos_device_instance_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ClearUsbDmxUniverseBindingRequestV1 {
    pub universe: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDmxUniverseBindingStatusV1 {
    universe: u16,
    state: SerialDmxMachineBindingStateV1,
    identity: SerialDmxMachineBindingIdentityV2,
    detail: String,
}

/// One engine cable slot. Queue fields are not fixture or wire acceptance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDmxCableStatusV1 {
    slot: usize,
    universe: Option<u16>,
    route_status_revision: String,
    active: bool,
    zero_frame_queued: bool,
    zero_frame_physical_write_completed: bool,
    live_frame_queued: bool,
    worker_shutdown_completed: bool,
    faulted: bool,
    detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDmxUniversesStatusV1 {
    bindings: Vec<UsbDmxUniverseBindingStatusV1>,
    cables: Vec<UsbDmxCableStatusV1>,
    /// Set when the binding file cannot be read; every cable stays blocked.
    persistence_error: Option<String>,
}

pub(super) fn bindings_path(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_local_data_dir()
        .map(|directory| directory.join(USB_DMX_UNIVERSE_BINDINGS_FILE))
        .map_err(|error| format!("Unable to resolve machine-local USB-DMX cable path: {error}"))
}

fn validate_universe(universe: u16) -> Result<(), String> {
    if universe == 0 || universe > MAX_USB_DMX_CABLE_UNIVERSE {
        return Err(format!(
            "USB-DMX cable universe must be 1–{MAX_USB_DMX_CABLE_UNIVERSE}; Universe 0 uses Show DMX"
        ));
    }
    Ok(())
}

/// The same physical cable, by any stable identity.
pub(super) fn same_cable(
    left: &SerialDmxMachineBindingIdentityV2,
    right: &SerialDmxMachineBindingIdentityV2,
) -> bool {
    let same = |left: &Option<String>, right: &Option<String>| matches!((left, right), (Some(left), Some(right)) if left.eq_ignore_ascii_case(right));
    left.port_name.eq_ignore_ascii_case(&right.port_name)
        || same(
            &left.windows_device_instance_id,
            &right.windows_device_instance_id,
        )
        || same(
            &left.macos_device_instance_id,
            &right.macos_device_instance_id,
        )
        || (left.usb_vid == right.usb_vid
            && left.usb_pid == right.usb_pid
            && left.serial_number == right.serial_number)
}

fn validate_bindings(bindings: &[UsbDmxUniverseBindingV1]) -> Result<(), String> {
    if bindings.len() > engine::MAX_INDEPENDENT_USB_DMX_ROUTES {
        return Err(format!(
            "At most {} USB-DMX cables can be bound besides Universe 0",
            engine::MAX_INDEPENDENT_USB_DMX_ROUTES
        ));
    }
    for (index, binding) in bindings.iter().enumerate() {
        validate_universe(binding.universe)?;
        for other in &bindings[index + 1..] {
            if other.universe == binding.universe {
                return Err(format!(
                    "Universe {} has more than one USB-DMX cable",
                    binding.universe
                ));
            }
            if same_cable(&other.identity, &binding.identity) {
                return Err(format!(
                    "One USB-DMX cable is bound to both Universe {} and Universe {}",
                    binding.universe, other.universe
                ));
            }
        }
    }
    Ok(())
}

/// Read and validate the binding file; a missing file is no binding. Every
/// identity is rebuilt through the strict constructor.
pub(super) fn load_bindings(path: &Path) -> Result<Vec<UsbDmxUniverseBindingV1>, String> {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "Unable to read machine-local USB-DMX cables {}: {error}",
                path.display()
            ))
        }
    };
    if bytes.len() as u64 > MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES {
        return Err("Machine-local USB-DMX cables exceed their safety limit".to_string());
    }
    let file: UsbDmxUniverseBindingsFileV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Machine-local USB-DMX cables are invalid: {error}"))?;
    if file.version != USB_DMX_UNIVERSE_BINDINGS_VERSION {
        return Err(format!(
            "Machine-local USB-DMX cable file version {} is unsupported",
            file.version
        ));
    }
    let universes = file
        .universes
        .into_iter()
        .map(|binding| {
            let identity = &binding.identity;
            let summary = SerialPortSummary {
                name: identity.port_name.clone(),
                port_type: identity.port_type.clone(),
                usb_vid: Some(identity.usb_vid),
                usb_pid: Some(identity.usb_pid),
                serial_number: Some(identity.serial_number.clone()),
                manufacturer: Some(identity.manufacturer.clone()),
                product: Some(identity.product.clone()),
                macos_device_instance_id: identity.macos_device_instance_id.clone(),
                windows_device_instance_id: identity.windows_device_instance_id.clone(),
                recommended_protocol: None,
            };
            Ok(UsbDmxUniverseBindingV1 {
                universe: binding.universe,
                identity: SerialDmxMachineBindingIdentityV2::from_summary(&summary)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    validate_bindings(&universes)?;
    Ok(universes)
}

fn persist_bindings(path: &Path, bindings: &[UsbDmxUniverseBindingV1]) -> Result<(), String> {
    validate_bindings(bindings)?;
    let mut universes = bindings.to_vec();
    universes.sort_by_key(|binding| binding.universe);
    let bytes = serde_json::to_vec(&UsbDmxUniverseBindingsFileV1 {
        version: USB_DMX_UNIVERSE_BINDINGS_VERSION,
        universes,
    })
    .map_err(|error| format!("Unable to encode machine-local USB-DMX cables: {error}"))?;
    if bytes.len() as u64 > MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES {
        return Err("Machine-local USB-DMX cables exceed their safety limit".to_string());
    }
    serial_dmx_machine::persist_binding_bytes_to_path(path, &bytes)
}

fn binding_status(
    binding: &UsbDmxUniverseBindingV1,
    ports: &[SerialPortSummary],
) -> UsbDmxUniverseBindingStatusV1 {
    let matches = ports
        .iter()
        .filter(|port| binding.identity.matches_summary(port))
        .count();
    let (state, detail) = match matches {
        1 => (
            SerialDmxMachineBindingStateV1::SelectedAndPresent,
            "The cable is present. Starting it revalidates the device handle again.".to_string(),
        ),
        0 => (
            SerialDmxMachineBindingStateV1::StaleOrMissing,
            format!(
                "The cable is not present exactly ({}). Reconnect it, or bind the universe again; no other cable is substituted.",
                binding.identity.label()
            ),
        ),
        _ => (
            SerialDmxMachineBindingStateV1::Ambiguous,
            format!(
                "The cable identity is ambiguous ({}); it stays off until one exact cable can be bound.",
                binding.identity.label()
            ),
        ),
    };
    UsbDmxUniverseBindingStatusV1 {
        universe: binding.universe,
        state,
        identity: binding.identity.clone(),
        detail,
    }
}

fn cable_statuses(state: &AppState) -> Result<Vec<UsbDmxCableStatusV1>, String> {
    Ok(state
        .engine
        .try_independent_usb_dmx_route_statuses()
        .map_err(|error| {
            format!("USB-DMX cable status is unavailable and remains fail-closed: {error}")
        })?
        .into_iter()
        .map(|snapshot| UsbDmxCableStatusV1 {
            slot: snapshot.slot,
            universe: snapshot.universe,
            route_status_revision: snapshot.revision.to_string(),
            active: snapshot.status.active,
            zero_frame_queued: snapshot.status.zero_frame_queued,
            zero_frame_physical_write_completed: snapshot
                .status
                .zero_frame_physical_write_completed,
            live_frame_queued: snapshot.status.live_frame_queued,
            worker_shutdown_completed: snapshot.status.worker_shutdown_completed,
            faulted: snapshot.status.faulted,
            detail: snapshot.status.detail,
        })
        .collect())
}

/// Read-only: bindings with presence, and every cable slot's status.
pub(super) fn status(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<UsbDmxUniversesStatusV1, String> {
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
    let cables = cable_statuses(state)?;
    let (bindings, persistence_error) = match load_bindings(&bindings_path(app)?) {
        Ok(bindings) => (
            bindings
                .iter()
                .map(|binding| binding_status(binding, &ports))
                .collect(),
            None,
        ),
        Err(error) => (Vec::new(), Some(error)),
    };
    Ok(UsbDmxUniversesStatusV1 {
        bindings,
        cables,
        persistence_error,
    })
}

/// A universe's binding may change only while its cable is stopped and
/// joined (a faulted, joined cable only under both S0 latches), exactly as
/// for the Universe 0 cable.
fn ensure_universe_binding_mutation_admissible(
    state: &AppState,
    universe: u16,
) -> Result<(), String> {
    let cables = state
        .engine
        .try_independent_usb_dmx_route_statuses()
        .map_err(|error| {
            format!("USB-DMX cable status is unavailable; the binding change was rejected fail-closed: {error}")
        })?;
    let Some(cable) = cables
        .into_iter()
        .find(|cable| cable.universe == Some(universe))
    else {
        return Ok(());
    };
    let status = &cable.status;
    if status.active || status.live_frame_queued {
        return Err(format!(
            "Universe {universe}'s USB-DMX cable is running; stop it under S0 before changing its binding"
        ));
    }
    if !status.worker_shutdown_completed {
        return Err(format!(
            "Universe {universe}'s USB-DMX cable has not completed bounded shutdown; its binding cannot change"
        ));
    }
    if status.zero_frame_queued && !status.zero_frame_physical_write_completed && !status.faulted {
        return Err(format!(
            "Universe {universe}'s USB-DMX cable has an incomplete S0 zero transaction"
        ));
    }
    if status.faulted {
        let safety = state.engine.try_safety_blackout_authority()?;
        let latched = state.engine.open_dmx_safety_blackout_latched()?;
        if !safety.engaged || !latched {
            return Err(
                "A faulted USB-DMX cable's binding changes only while both S0 latches are engaged"
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn with_binding_mutation_locks<T>(
    state: &AppState,
    operation: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before a USB-DMX cable change"
            .to_string()
    })?;
    let _external_admission = super::lock_project_external_command_admission(state)?;
    let mut coordinator = super::lock_project_coordinator(state)?;
    if super::reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).is_err()
        || super::ensure_no_pending_project_transaction(&coordinator).is_err()
    {
        return Err(
            "USB-DMX cable binding is unavailable while project/output authority is not coherent"
                .to_string(),
        );
    }
    let _transition_guard = super::lock_output_ownership_transition(state)?;
    operation()
}

/// Write the new bindings, then re-check admission; a changed admission
/// restores the exact previous file.
fn persist_admitted(
    state: &AppState,
    path: &Path,
    universe: u16,
    next: &[UsbDmxUniverseBindingV1],
) -> Result<(), String> {
    ensure_universe_binding_mutation_admissible(state, universe)?;
    let previous = serial_dmx_machine::capture_binding_file_snapshot(path)?;
    persist_bindings(path, next)?;
    if let Err(admission_error) = ensure_universe_binding_mutation_admissible(state, universe) {
        serial_dmx_machine::restore_binding_file_snapshot(path, &previous).map_err(
            |restore_error| {
                format!(
                    "USB-DMX cable admission changed after persistence ({admission_error}); restoring the previous bindings also failed: {restore_error}. Bindings remain blocked."
                )
            },
        )?;
        return Err(format!(
            "USB-DMX cable admission changed after persistence ({admission_error}); the previous bindings were restored"
        ));
    }
    Ok(())
}

/// Bind one exact, currently enumerated cable to a universe. The cable must
/// not already serve Universe 0 or another universe.
pub(super) fn select_binding(
    state: &AppState,
    app: &tauri::AppHandle,
    request: SelectUsbDmxUniverseBindingRequestV1,
) -> Result<UsbDmxUniversesStatusV1, String> {
    validate_universe(request.universe)?;
    with_binding_mutation_locks(state, || {
        let ports = io::serial_dmx::list_serial_ports()
            .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
        let port_name = request.port_name.trim();
        let windows = request.windows_device_instance_id.as_deref();
        let macos = request.macos_device_instance_id.as_deref();
        if port_name.is_empty()
            || windows.is_some() == macos.is_some()
            || windows.or(macos).is_none_or(|id| id.trim().is_empty())
        {
            return Err(
                "USB-DMX cable binding requires a port and exactly one Windows PnP or macOS IOKit instance"
                    .to_string(),
            );
        }
        let matches = ports
            .iter()
            .filter(|port| {
                port.name == port_name
                    && port.windows_device_instance_id.as_deref() == windows
                    && port.macos_device_instance_id.as_deref() == macos
            })
            .collect::<Vec<_>>();
        let identity =
            match matches.as_slice() {
                [port] => SerialDmxMachineBindingIdentityV2::from_summary(port)?,
                [] => return Err(
                    "The selected USB-DMX cable is no longer enumerated; refresh and select again"
                        .to_string(),
                ),
                _ => return Err(
                    "The selected USB-DMX COM alias and instance are ambiguous; nothing was bound"
                        .to_string(),
                ),
            };
        let u0 = serial_dmx_machine::binding_status_from_path(
            &show_serial_dmx_route::serial_dmx_machine_binding_path(app)?,
            &ports,
        );
        if u0
            .selected
            .as_ref()
            .is_some_and(|selected| same_cable(selected, &identity))
        {
            return Err(
                "This USB-DMX cable is the Universe 0 (Show DMX) cable; one cable outputs one universe"
                    .to_string(),
            );
        }
        let path = bindings_path(app)?;
        let mut bindings = load_bindings(&path)?;
        if let Some(other) = bindings.iter().find(|binding| {
            binding.universe != request.universe && same_cable(&binding.identity, &identity)
        }) {
            return Err(format!(
                "This USB-DMX cable is already bound to Universe {}; one cable outputs one universe",
                other.universe
            ));
        }
        bindings.retain(|binding| binding.universe != request.universe);
        bindings.push(UsbDmxUniverseBindingV1 {
            universe: request.universe,
            identity,
        });
        persist_admitted(state, &path, request.universe, &bindings)?;
        status(state, app)
    })
}

/// Remove a universe's cable binding (its cable must be stopped).
pub(super) fn clear_binding(
    state: &AppState,
    app: &tauri::AppHandle,
    request: ClearUsbDmxUniverseBindingRequestV1,
) -> Result<UsbDmxUniversesStatusV1, String> {
    validate_universe(request.universe)?;
    with_binding_mutation_locks(state, || {
        let path = bindings_path(app)?;
        let mut bindings = load_bindings(&path)?;
        let before = bindings.len();
        bindings.retain(|binding| binding.universe != request.universe);
        if bindings.len() != before {
            persist_admitted(state, &path, request.universe, &bindings)?;
        }
        status(state, app)
    })
}

/// The Universe 0 cable must not take a cable already bound to a universe.
pub(super) fn ensure_cable_not_bound_to_a_universe(
    app: &tauri::AppHandle,
    identity: &SerialDmxMachineBindingIdentityV2,
) -> Result<(), String> {
    let bindings = load_bindings(&bindings_path(app)?)?;
    if let Some(binding) = bindings
        .iter()
        .find(|binding| same_cable(&binding.identity, identity))
    {
        return Err(format!(
            "This USB-DMX cable is bound to Universe {}; one cable outputs one universe",
            binding.universe
        ));
    }
    Ok(())
}

/// Exact current identity of a universe's bound cable.
fn verify_current_universe_cable(
    app: &tauri::AppHandle,
    universe: u16,
) -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String> {
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
    let binding = load_bindings(&bindings_path(app)?)?
        .into_iter()
        .find(|binding| binding.universe == universe)
        .ok_or_else(|| format!("Universe {universe} has no bound USB-DMX cable"))?;
    let status = binding_status(&binding, &ports);
    if status.state != SerialDmxMachineBindingStateV1::SelectedAndPresent {
        return Err(status.detail);
    }
    show_serial_dmx_route::validate_exact_show_serial_dmx_device_matches(&ports, &binding.identity)
}

/// Start one universe's cable under the OutputControl lease/fence. The
/// engine opens it zero-first and keeps it only after its physical zero.
pub(super) fn enable_with_output_control_fence(
    app: &tauri::AppHandle,
    state: &AppState,
    universe: u16,
    expected_fence: &OutputControlFenceV1,
    lease_request: &output_lease::OutputLeaseRequest,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    validate_universe(universe)?;
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before USB-DMX cable start".to_string()
    })?;
    let _external_admission = super::lock_project_external_command_admission(state)?;
    let coordinator = RefCell::new(super::lock_project_coordinator(state)?);
    let revalidate = || -> Result<io::serial_dmx::VerifiedUsbSerialPortIdentity, String> {
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
                return Err("Output control fence changed before USB-DMX cable start".to_string());
            }
        }
        let ownership = state.engine.output_ownership_status();
        if ownership.state != protocol::OutputOwnershipState::Ready
            || ownership.effective_role != MachineOutputRole::Both
            || ownership.desired_role != MachineOutputRole::Both
            || !ownership.lighting_allowed
        {
            return Err(
                "USB-DMX cable start requires the active local Both output authority".to_string(),
            );
        }
        let safety = state.engine.safety_blackout_authority();
        if safety.epoch != expected_fence.safety_blackout_epoch
            || safety.generation != expected_fence.safety_blackout_generation
        {
            return Err(
                "USB-DMX cable start requires the exact currently-observed S0 safety blackout authority"
                    .to_string(),
            );
        }
        verify_current_universe_cable(app, universe)
    };
    super::with_revalidated_output_transition(
        || super::lock_output_ownership_transition(state),
        &revalidate,
        |_transition_guard, device| {
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
                "Output lease registry lock was poisoned before USB-DMX cable start".to_string()
            })?;
            let final_lease_now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) = super::submit_show_output_candidate_with_commit(
                state,
                &mut lease_registry,
                lease_request,
                managed_authorization.as_ref(),
                final_lease_now_ms,
                "USB-DMX cable start",
                || {
                    let current_device = revalidate()?;
                    if current_device != device {
                        return Err("USB-DMX cable changed before final activation".to_string());
                    }
                    state
                        .engine
                        .enable_usb_dmx_universe_route(
                            universe,
                            current_device,
                            expected_fence.safety_blackout_epoch,
                            expected_fence.safety_blackout_generation,
                            Instant::now() + Duration::from_secs(2),
                        )
                        .map_err(|error| format!("USB-DMX cable start failed: {error}"))?;
                    Ok(true)
                },
            )?;
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}

/// Stop one universe's cable under the exact engaged S0 (one confirmed zero,
/// then a bounded shutdown; a joined faulted cable is taken out of service).
pub(super) fn stop_with_output_control_fence(
    state: &AppState,
    universe: u16,
    expected_fence: &OutputControlFenceV1,
    lease_request: &output_lease::OutputLeaseRequest,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    validate_universe(universe)?;
    let _lifecycle_guard = state.standby_sync_lifecycle.lock().map_err(|_| {
        "Standby synchronization lifecycle lock was poisoned before USB-DMX cable stop".to_string()
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
                return Err("Output control fence changed before USB-DMX cable stop".to_string());
            }
            let safety = state.engine.safety_blackout_authority();
            if !safety.engaged
                || safety.epoch != expected_fence.safety_blackout_epoch
                || safety.generation != expected_fence.safety_blackout_generation
            {
                return Err(
                    "USB-DMX cable stop requires the exact currently-engaged S0 safety blackout authority"
                        .to_string(),
                );
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
                "Output lease registry lock was poisoned before USB-DMX cable stop".to_string()
            })?;
            let final_lease_now_ms = state.output_lease_now_ms()?;
            let (applied, lease_receipt) = super::submit_show_output_candidate_with_commit(
                state,
                &mut lease_registry,
                lease_request,
                managed_authorization.as_ref(),
                final_lease_now_ms,
                "USB-DMX cable stop",
                || {
                    let safety = state.engine.safety_blackout_authority();
                    if !safety.engaged
                        || safety.epoch != expected_fence.safety_blackout_epoch
                        || safety.generation != expected_fence.safety_blackout_generation
                    {
                        return Err(
                            "USB-DMX cable stop was superseded by an S0 safety blackout authority change"
                                .to_string(),
                        );
                    }
                    state
                        .engine
                        .stop_usb_dmx_universe_route(
                            universe,
                            expected_fence.safety_blackout_epoch,
                            expected_fence.safety_blackout_generation,
                            Instant::now() + Duration::from_secs(2),
                        )
                        .map_err(|error| format!("USB-DMX cable stop failed: {error}"))?;
                    Ok(true)
                },
            )?;
            Ok((applied, expected_fence.clone(), lease_receipt))
        },
    )
}

#[cfg(test)]
#[path = "tests/usb_dmx_universes_tests.rs"]
mod tests;
