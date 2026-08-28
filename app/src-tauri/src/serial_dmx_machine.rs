//! Machine-local authority for the one production USB-DMX interface.
//!
//! A show project deliberately contains only the logical Open-DMX parameters
//! (protocol, baud rate, universe).  A Windows COM alias and PnP instance are
//! host-local facts, so this small independent store is the sole persistence
//! boundary for selecting the physical interface.  Nothing in this module
//! reads or mutates a project file.

use std::{
    ffi::OsStr,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use protocol::SerialPortSummary;
use serde::{Deserialize, Serialize};

pub const SERIAL_DMX_MACHINE_BINDING_FILE: &str = "serial-dmx-machine-binding-v1.json";
const SERIAL_DMX_MACHINE_BINDING_VERSION: u32 = 1;
const MAX_SERIAL_DMX_MACHINE_BINDING_BYTES: u64 = 8 * 1024;
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SerialDmxMachineBindingIdentityV1 {
    pub port_name: String,
    pub port_type: String,
    pub usb_vid: u16,
    pub usb_pid: u16,
    pub serial_number: String,
    pub manufacturer: String,
    pub product: String,
    pub windows_device_instance_id: String,
}

impl SerialDmxMachineBindingIdentityV1 {
    pub fn from_summary(port: &SerialPortSummary) -> Result<Self, String> {
        fn required(value: Option<&str>, label: &str) -> Result<String, String> {
            let value = value.unwrap_or_default().trim();
            if value.is_empty() {
                Err(format!("Selected USB-DMX interface is missing {label}"))
            } else {
                Ok(value.to_string())
            }
        }
        let port_name = required(Some(&port.name), "COM alias")?;
        let port_type = required(Some(&port.port_type), "port type")?;
        let usb_vid = port
            .usb_vid
            .ok_or_else(|| "Selected USB-DMX interface is missing USB VID".to_string())?;
        let usb_pid = port
            .usb_pid
            .ok_or_else(|| "Selected USB-DMX interface is missing USB PID".to_string())?;
        Ok(Self {
            port_name,
            port_type,
            usb_vid,
            usb_pid,
            serial_number: required(port.serial_number.as_deref(), "hardware serial")?,
            manufacturer: required(port.manufacturer.as_deref(), "manufacturer")?,
            product: required(port.product.as_deref(), "product")?,
            windows_device_instance_id: required(
                port.windows_device_instance_id.as_deref(),
                "Windows PnP instance",
            )?,
        })
    }

    pub fn matches_summary(&self, port: &SerialPortSummary) -> bool {
        Self::from_summary(port).as_ref() == Ok(self)
    }

    pub fn label(&self) -> String {
        format!(
            "{} · {} / {} · serial {} · {}",
            self.port_name,
            self.manufacturer,
            self.product,
            self.serial_number,
            self.windows_device_instance_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SerialDmxMachineBindingFileV1 {
    version: u32,
    selected: SerialDmxMachineBindingIdentityV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SerialDmxMachineBindingStateV1 {
    MissingSelection,
    SelectedAndPresent,
    StaleOrMissing,
    Ambiguous,
    BlockedPersistence,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SerialDmxMachineBindingStatusV1 {
    pub state: SerialDmxMachineBindingStateV1,
    pub selected: Option<SerialDmxMachineBindingIdentityV1>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectSerialDmxMachineBindingRequestV1 {
    pub port_name: String,
    pub windows_device_instance_id: String,
}

pub fn serial_dmx_machine_binding_path(local_data_dir: &Path) -> PathBuf {
    local_data_dir.join(SERIAL_DMX_MACHINE_BINDING_FILE)
}

pub fn binding_status_from_path(
    path: &Path,
    ports: &[SerialPortSummary],
) -> SerialDmxMachineBindingStatusV1 {
    let selected = match load_selected_from_path(path) {
        Ok(Some(selected)) => selected,
        Ok(None) => {
            return SerialDmxMachineBindingStatusV1 {
                state: SerialDmxMachineBindingStateV1::MissingSelection,
                selected: None,
                detail: "No machine-local USB-DMX interface is selected. Select and confirm an enumerated interface on this PC.".to_string(),
            }
        }
        Err(detail) => {
            return SerialDmxMachineBindingStatusV1 {
                state: SerialDmxMachineBindingStateV1::BlockedPersistence,
                selected: None,
                detail,
            }
        }
    };
    let matches = ports
        .iter()
        .filter(|port| selected.matches_summary(port))
        .count();
    match matches {
        1 => SerialDmxMachineBindingStatusV1 {
            state: SerialDmxMachineBindingStateV1::SelectedAndPresent,
            selected: Some(selected),
            detail: "The selected machine-local USB-DMX identity is present. Opening revalidates the real Windows handle again.".to_string(),
        },
        0 => SerialDmxMachineBindingStatusV1 {
            state: SerialDmxMachineBindingStateV1::StaleOrMissing,
            selected: Some(selected.clone()),
            detail: format!(
                "Selected USB-DMX identity is not present exactly ({}). COM renumber, restart, replacement, or unplug requires explicit reselection; no interface was substituted.",
                selected.label(),
            ),
        },
        _ => SerialDmxMachineBindingStatusV1 {
            state: SerialDmxMachineBindingStateV1::Ambiguous,
            selected: Some(selected.clone()),
            detail: format!(
                "Selected USB-DMX identity is ambiguous ({}); output remains disabled until one exact interface can be selected.",
                selected.label(),
            ),
        },
    }
}

pub fn select_binding_from_ports(
    path: &Path,
    request: &SelectSerialDmxMachineBindingRequestV1,
    ports: &[SerialPortSummary],
) -> Result<SerialDmxMachineBindingStatusV1, String> {
    let port_name = request.port_name.trim();
    let instance = request.windows_device_instance_id.trim();
    if port_name.is_empty() || instance.is_empty() {
        return Err(
            "USB-DMX selection must include an enumerated COM alias and Windows PnP instance"
                .to_string(),
        );
    }
    let matches = ports
        .iter()
        .filter(|port| {
            port.name == port_name
                && port.windows_device_instance_id.as_deref().map(str::trim) == Some(instance)
        })
        .collect::<Vec<_>>();
    let selected =
        match matches.as_slice() {
            [port] => SerialDmxMachineBindingIdentityV1::from_summary(port)?,
            [] => {
                return Err(
                    "Selected USB-DMX interface is no longer enumerated; refresh and select again"
                        .to_string(),
                )
            }
            _ => return Err(
                "Selected USB-DMX COM alias and PnP instance are ambiguous; no binding was written"
                    .to_string(),
            ),
        };
    persist_selected_to_path(path, &selected)?;
    Ok(binding_status_from_path(path, ports))
}

pub fn resolve_selected_identity_from_path(
    path: &Path,
    ports: &[SerialPortSummary],
) -> Result<SerialDmxMachineBindingIdentityV1, String> {
    let status = binding_status_from_path(path, ports);
    match status.state {
        SerialDmxMachineBindingStateV1::SelectedAndPresent => status
            .selected
            .ok_or_else(|| "USB-DMX selection was lost during validation".to_string()),
        _ => Err(format!(
            "Machine-local USB-DMX selection is not usable: {}",
            status.detail
        )),
    }
}

fn load_selected_from_path(
    path: &Path,
) -> Result<Option<SerialDmxMachineBindingIdentityV1>, String> {
    let file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Unable to read machine-local USB-DMX selection {}: {error}",
                path.display()
            ))
        }
    };
    let declared_len = file
        .metadata()
        .map_err(|error| {
            format!(
                "Unable to inspect machine-local USB-DMX selection {}: {error}",
                path.display()
            )
        })?
        .len();
    if declared_len > MAX_SERIAL_DMX_MACHINE_BINDING_BYTES {
        return Err(format!(
            "Machine-local USB-DMX selection exceeds the {} byte safety limit",
            MAX_SERIAL_DMX_MACHINE_BINDING_BYTES
        ));
    }
    let mut bytes = Vec::with_capacity(declared_len as usize);
    file.take(MAX_SERIAL_DMX_MACHINE_BINDING_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| {
            format!(
                "Unable to read machine-local USB-DMX selection {}: {error}",
                path.display()
            )
        })?;
    if bytes.len() as u64 > MAX_SERIAL_DMX_MACHINE_BINDING_BYTES {
        return Err("Machine-local USB-DMX selection grew during read".to_string());
    }
    let file: SerialDmxMachineBindingFileV1 = serde_json::from_slice(&bytes)
        .map_err(|error| format!("Machine-local USB-DMX selection is invalid: {error}"))?;
    if file.version != SERIAL_DMX_MACHINE_BINDING_VERSION {
        return Err(format!(
            "Machine-local USB-DMX selection version {} is unsupported",
            file.version
        ));
    }
    // Re-run strict nonempty field validation. A manually altered settings file
    // can never turn missing hardware evidence into authority.
    let summary = SerialPortSummary {
        name: file.selected.port_name.clone(),
        port_type: file.selected.port_type.clone(),
        usb_vid: Some(file.selected.usb_vid),
        usb_pid: Some(file.selected.usb_pid),
        serial_number: Some(file.selected.serial_number.clone()),
        manufacturer: Some(file.selected.manufacturer.clone()),
        product: Some(file.selected.product.clone()),
        windows_device_instance_id: Some(file.selected.windows_device_instance_id.clone()),
        recommended_protocol: None,
    };
    Ok(Some(SerialDmxMachineBindingIdentityV1::from_summary(
        &summary,
    )?))
}

fn persist_selected_to_path(
    path: &Path,
    selected: &SerialDmxMachineBindingIdentityV1,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(&SerialDmxMachineBindingFileV1 {
        version: SERIAL_DMX_MACHINE_BINDING_VERSION,
        selected: selected.clone(),
    })
    .map_err(|error| format!("Unable to encode machine-local USB-DMX selection: {error}"))?;
    if bytes.len() as u64 > MAX_SERIAL_DMX_MACHINE_BINDING_BYTES {
        return Err("Machine-local USB-DMX selection exceeds its safety limit".to_string());
    }
    let parent = path.parent().ok_or_else(|| {
        format!(
            "Machine-local USB-DMX selection has no parent: {}",
            path.display()
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        format!(
            "Unable to create machine-local USB-DMX directory {}: {error}",
            parent.display()
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or(SERIAL_DMX_MACHINE_BINDING_FILE);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let temporary = parent.join(format!(
        ".{file_name}.{}.{}.{}.tmp",
        std::process::id(),
        nonce,
        TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|error| {
                format!(
                    "Unable to create temporary machine-local USB-DMX selection {}: {error}",
                    temporary.display()
                )
            })?;
        file.write_all(&bytes).map_err(|error| {
            format!(
                "Unable to write temporary machine-local USB-DMX selection {}: {error}",
                temporary.display()
            )
        })?;
        file.sync_all().map_err(|error| {
            format!(
                "Unable to flush temporary machine-local USB-DMX selection {}: {error}",
                temporary.display()
            )
        })?;
        drop(file);
        super::replace_file_atomically(&temporary, path)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(test)]
#[path = "tests/serial_dmx_machine_tests.rs"]
mod tests;
