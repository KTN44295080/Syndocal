//! Machine-local independent USB-DMX cables for universes 0 through 32767.
//!
//! Each binding maps one universe to one exact machine-local cable identity.
//! Projects never carry COM/PnP identity. Version 2 consumes the legacy U0
//! selection once; an empty unified store is authoritative thereafter.
//! Starting or stopping a cable goes through the OutputControl lease/fence
//! like the Universe 0 cable, and the engine keeps every cable zero-first
//! and S0-bound.

use super::*;
use serial_dmx_machine::{SerialDmxMachineBindingIdentityV2, SerialDmxMachineBindingStateV1};

pub const USB_DMX_UNIVERSE_BINDINGS_FILE: &str = "usb-dmx-universe-bindings-v1.json";
const USB_DMX_UNIVERSE_BINDINGS_VERSION: u32 = 2;
const MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES: u64 = 8 * 1024;
const MAX_USB_DMX_CABLE_UNIVERSE: u16 = 32_767;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsbDmxUniverseBindingV1 {
    pub universe: u16,
    pub identity: SerialDmxMachineBindingIdentityV2,
    /// Preserve the old mirror's channel-500 invariant only for imported U0.
    #[serde(default, skip_serializing_if = "is_false")]
    pub legacy_u0_invariants: bool,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct UsbDmxUniverseBindingsFileV1 {
    version: u32,
    universes: Vec<UsbDmxUniverseBindingV1>,
}

/// Optional project intent fence for host-local table edits; absent for old clients.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DmxTableProjectIdentity {
    pub project_epoch: u64,
    pub project_revision: u64,
    pub checkpoint_hash: String,
}

fn ensure_expected_project(
    expected: Option<&DmxTableProjectIdentity>,
    epoch: u64,
    revision: u64,
    hash: &str,
) -> Result<(), String> {
    if let Some(expected) = expected {
        if expected.checkpoint_hash.trim().is_empty()
            || expected.project_epoch != epoch
            || expected.project_revision != revision
            || expected.checkpoint_hash != hash
        {
            return Err("The project changed before USB-DMX binding persistence; reload output settings before applying".to_string());
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectUsbDmxUniverseBindingRequestV1 {
    #[serde(default)]
    pub expected_project: Option<DmxTableProjectIdentity>,
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
    #[serde(default)]
    pub expected_project: Option<DmxTableProjectIdentity>,
    pub universe: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UsbDmxUniverseBindingStatusV1 {
    universe: u16,
    state: SerialDmxMachineBindingStateV1,
    identity: SerialDmxMachineBindingIdentityV2,
    legacy_u0_invariants: bool,
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
    /// Unified machine-local schema; version 2 includes the one-way U0 migration.
    binding_store_version: u32,
    /// True until the legacy worker is stopped and joined; independent starts are blocked.
    legacy_route_active: bool,
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
    if universe > MAX_USB_DMX_CABLE_UNIVERSE {
        return Err(format!(
            "USB-DMX cable universe must be 0 through {MAX_USB_DMX_CABLE_UNIVERSE}"
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
            "At most {} USB-DMX cables can be bound",
            engine::MAX_INDEPENDENT_USB_DMX_ROUTES
        ));
    }
    for (index, binding) in bindings.iter().enumerate() {
        validate_universe(binding.universe)?;
        if binding.legacy_u0_invariants && binding.universe != 0 {
            return Err("Legacy U0 channel policy is valid only for Universe 0".to_string());
        }
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
fn load_binding_store(path: &Path) -> Result<Vec<UsbDmxUniverseBindingV1>, String> {
    use std::io::Read;
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(format!(
                "Unable to read machine-local USB-DMX cables {}: {error}",
                path.display()
            ))
        }
    };
    let mut bytes = Vec::new();
    file.take(MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "Unable to read machine-local USB-DMX cables {}: {error}",
                path.display()
            )
        })?;
    if bytes.len() as u64 > MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES {
        return Err(format!(
            "Machine-local USB-DMX cables {} exceed their safety limit",
            path.display()
        ));
    }
    let file: UsbDmxUniverseBindingsFileV1 = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "Machine-local USB-DMX cables {} are invalid ({error}); remove the file to start over",
            path.display()
        )
    })?;
    if file.version != 1 && file.version != USB_DMX_UNIVERSE_BINDINGS_VERSION {
        return Err(format!(
            "Machine-local USB-DMX cable file {} has unsupported version {}",
            path.display(),
            file.version
        ));
    }
    let version = file.version;
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
                legacy_u0_invariants: binding.legacy_u0_invariants,
                identity: SerialDmxMachineBindingIdentityV2::from_summary(&summary)?,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    validate_bindings(&universes)?;
    if version == 1 && universes.iter().any(|binding| binding.universe == 0) {
        return Err("Version 1 USB-DMX bindings cannot contain Universe 0".to_string());
    }
    Ok(universes)
}

/// True only for a readable schema-2 store; anything else takes the locked
/// migration path, which reports the exact failure.
fn binding_store_is_current(path: &Path) -> bool {
    #[derive(Deserialize)]
    struct VersionOnly {
        version: u32,
    }
    fs::read(path)
        .ok()
        .filter(|bytes| bytes.len() as u64 <= MAX_USB_DMX_UNIVERSE_BINDINGS_BYTES)
        .and_then(|bytes| serde_json::from_slice::<VersionOnly>(&bytes).ok())
        .is_some_and(|file| file.version == USB_DMX_UNIVERSE_BINDINGS_VERSION)
}

/// Upgrade once, without opening a device or deleting the legacy selection.
/// Version 2 is authoritative even when empty: removals never resurrect U0.
pub(super) fn load_bindings(path: &Path) -> Result<Vec<UsbDmxUniverseBindingV1>, String> {
    static MIGRATION_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _migration_guard = MIGRATION_LOCK
        .lock()
        .map_err(|_| "USB-DMX migration lock poisoned")?;
    let mut bindings = load_binding_store(path)?;
    let snapshot = serial_dmx_machine::capture_binding_file_snapshot(path)?;
    if let serial_dmx_machine::SerialDmxMachineBindingFileSnapshot::Bytes(bytes) = &snapshot {
        let file: UsbDmxUniverseBindingsFileV1 = serde_json::from_slice(bytes)
            .map_err(|error| format!("Invalid USB-DMX binding store: {error}"))?;
        if file.version == USB_DMX_UNIVERSE_BINDINGS_VERSION {
            return Ok(bindings);
        }
    }
    let directory = path
        .parent()
        .ok_or("USB-DMX binding store has no parent directory")?;
    let legacy = serial_dmx_machine::binding_status_from_path(
        &serial_dmx_machine::serial_dmx_machine_binding_path(directory),
        &[],
    );
    if legacy.state == SerialDmxMachineBindingStateV1::BlockedPersistence {
        return Err(format!("USB-DMX migration blocked: {}", legacy.detail));
    }
    if let Some(identity) = legacy.selected {
        bindings.push(UsbDmxUniverseBindingV1 {
            universe: 0,
            legacy_u0_invariants: true,
            identity,
        });
    }
    validate_bindings(&bindings)?;
    // Atomic replacement leaves the exact old file intact if persistence fails.
    persist_bindings(path, &bindings)?;
    Ok(bindings)
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
        legacy_u0_invariants: binding.legacy_u0_invariants,
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

/// Presence/status inspection opens no device. First read atomically upgrades
/// machine-local bindings to schema 2 under lifecycle/admission locks.
pub(super) fn status(
    state: &AppState,
    app: &tauri::AppHandle,
) -> Result<UsbDmxUniversesStatusV1, String> {
    let ports = io::serial_dmx::list_serial_ports()
        .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
    let cables = cable_statuses(state)?;
    let legacy_route_active = legacy_route_active(state)?;
    let path = bindings_path(app)?;
    // An already-upgraded store is read without the mutation locks, so the
    // 2 s poll never contends with Standby/project transactions or reports
    // a transient project-transaction refusal as a persistence error.
    let loaded = if binding_store_is_current(&path) {
        load_binding_store(&path)
    } else {
        with_binding_mutation_locks(state, None, || load_bindings(&path))
    };
    let (bindings, persistence_error) = match loaded {
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
        binding_store_version: USB_DMX_UNIVERSE_BINDINGS_VERSION,
        legacy_route_active,
        bindings,
        cables,
        persistence_error,
    })
}

/// Legacy diagnostic API cannot overlap the independent cable owner.
fn legacy_route_active(state: &AppState) -> Result<bool, String> {
    let snapshot = state
        .engine
        .try_show_serial_dmx_safety_blackout_route_status_snapshot()
        .map_err(|error| format!("Legacy USB-DMX worker status unavailable: {error}"))?;
    let route = snapshot.status;
    Ok(route.active
        || route.live_frame_queued
        || !route.worker_shutdown_completed
        || (route.zero_frame_queued
            && !route.zero_frame_physical_write_completed
            && !route.faulted))
}

fn ensure_legacy_worker_retired(state: &AppState) -> Result<(), String> {
    if legacy_route_active(state)? {
        return Err("Stop the legacy Show DMX worker under blackout and wait for shutdown before using independent USB-DMX routes".to_string());
    }
    Ok(())
}

/// A universe's binding may change only while its cable is stopped and
/// joined (a faulted, joined cable only under both S0 latches), exactly as
/// for the Universe 0 cable.
fn ensure_universe_binding_mutation_admissible(
    state: &AppState,
    universe: u16,
) -> Result<(), String> {
    ensure_legacy_worker_retired(state)?;
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
    expected_project: Option<&DmxTableProjectIdentity>,
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
    ensure_expected_project(
        expected_project,
        coordinator.epoch,
        coordinator.revision,
        &coordinator.checkpoint_hash,
    )?;
    let _transition_guard = super::lock_output_ownership_transition(state)?;
    operation()
}

/// Write the new bindings under `admission`, checked twice before the write
/// (enumeration can be slow) and once after it; a late refusal restores the
/// exact previous file.
fn persist_admitted_with(
    path: &Path,
    next: &[UsbDmxUniverseBindingV1],
    admission: &mut dyn FnMut() -> Result<(), String>,
) -> Result<(), String> {
    admission()?;
    admission()?;
    let previous = serial_dmx_machine::capture_binding_file_snapshot(path)?;
    persist_bindings(path, next)?;
    if let Err(admission_error) = admission() {
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

/// The binding rules without app state, so they can be tested: resolve the
/// requested port, keep the cable disjoint from all other universes,
/// then persist under `admission`.
pub(super) fn select_binding_from_ports(
    path: &Path,
    _u0: &serial_dmx_machine::SerialDmxMachineBindingStatusV1,
    request: &SelectUsbDmxUniverseBindingRequestV1,
    ports: &[SerialPortSummary],
    admission: &mut dyn FnMut() -> Result<(), String>,
) -> Result<(), String> {
    validate_universe(request.universe)?;
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
    let identity = match matches.as_slice() {
        [port] => SerialDmxMachineBindingIdentityV2::from_summary(port)?,
        [] => {
            return Err(
                "The selected USB-DMX cable is no longer enumerated; refresh and select again"
                    .to_string(),
            )
        }
        _ => {
            return Err(
                "The selected USB-DMX COM alias and instance are ambiguous; nothing was bound"
                    .to_string(),
            )
        }
    };
    let mut bindings = load_bindings(path)?;
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
        legacy_u0_invariants: false,
        identity,
    });
    persist_admitted_with(path, &bindings, admission)
}

/// Bind one exact, currently enumerated cable to a universe. The cable must
/// not already serve another universe.
pub(super) fn select_binding(
    state: &AppState,
    app: &tauri::AppHandle,
    request: SelectUsbDmxUniverseBindingRequestV1,
) -> Result<UsbDmxUniversesStatusV1, String> {
    validate_universe(request.universe)?;
    with_binding_mutation_locks(state, request.expected_project.as_ref(), || {
        let ports = io::serial_dmx::list_serial_ports()
            .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
        let u0 = serial_dmx_machine::binding_status_from_path(
            &show_serial_dmx_route::serial_dmx_machine_binding_path(app)?,
            &ports,
        );
        let path = bindings_path(app)?;
        select_binding_from_ports(&path, &u0, &request, &ports, &mut || {
            ensure_universe_binding_mutation_admissible(state, request.universe)
        })?;
        Ok(status_after_commit(state, &path, &ports))
    })
}

/// Remove a universe's cable binding (its cable must be stopped).
pub(super) fn clear_binding(
    state: &AppState,
    app: &tauri::AppHandle,
    request: ClearUsbDmxUniverseBindingRequestV1,
) -> Result<UsbDmxUniversesStatusV1, String> {
    validate_universe(request.universe)?;
    with_binding_mutation_locks(state, request.expected_project.as_ref(), || {
        let ports = io::serial_dmx::list_serial_ports()
            .map_err(|error| format!("USB-DMX device enumeration failed: {error}"))?;
        let path = bindings_path(app)?;
        let mut bindings = load_bindings(&path)?;
        let before = bindings.len();
        bindings.retain(|binding| binding.universe != request.universe);
        if bindings.len() != before {
            persist_admitted_with(&path, &bindings, &mut || {
                ensure_universe_binding_mutation_admissible(state, request.universe)
            })?;
        }
        Ok(status_after_commit(state, &path, &ports))
    })
}

/// The reply after a committed change, built from the ports already listed.
/// A cable status that cannot be read is reported in the reply, never as a
/// failure of the change that was already written.
fn status_after_commit(
    state: &AppState,
    path: &Path,
    ports: &[SerialPortSummary],
) -> UsbDmxUniversesStatusV1 {
    let (bindings, mut persistence_error) = match load_bindings(path) {
        Ok(bindings) => (
            bindings
                .iter()
                .map(|binding| binding_status(binding, ports))
                .collect(),
            None,
        ),
        Err(error) => (Vec::new(), Some(error)),
    };
    let cables = match cable_statuses(state) {
        Ok(cables) => cables,
        Err(error) => {
            persistence_error.get_or_insert(format!("The change was saved; {error}"));
            Vec::new()
        }
    };
    let legacy_route_active = match legacy_route_active(state) {
        Ok(active) => active,
        Err(error) => {
            persistence_error.get_or_insert(format!("The change was saved; {error}"));
            true
        }
    };
    UsbDmxUniversesStatusV1 {
        binding_store_version: USB_DMX_UNIVERSE_BINDINGS_VERSION,
        legacy_route_active,
        bindings,
        cables,
        persistence_error,
    }
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
) -> Result<(io::serial_dmx::VerifiedUsbSerialPortIdentity, bool), String> {
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
    let device = show_serial_dmx_route::validate_exact_show_serial_dmx_device_matches(
        &ports,
        &binding.identity,
    )?;
    Ok((device, binding.legacy_u0_invariants))
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
    let revalidate = || -> Result<(io::serial_dmx::VerifiedUsbSerialPortIdentity, bool), String> {
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
        ensure_legacy_worker_retired(state)?;
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
                        .enable_independent_usb_dmx_route_with_legacy_u0_policy(
                            universe,
                            current_device.0,
                            expected_fence.safety_blackout_epoch,
                            expected_fence.safety_blackout_generation,
                            Instant::now() + Duration::from_secs(2),
                            current_device.1,
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
