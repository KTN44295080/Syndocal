//! Local-only control-plane inventory.
//!
//! This module intentionally provides discovery metadata, not an execution
//! transport.  It is compiled from the same `generate_handler!` source that
//! Tauri uses so a newly registered command is fail-closed as R5/Unavailable
//! until an explicit, reviewed R0 classification is added here.

use std::{collections::BTreeSet, fmt, sync::LazyLock};

use engine::control_plane_engine_command_descriptors;
use io::{control_plane_midi_osc_dmx_descriptors, control_plane_remote_descriptors};
use protocol::control_plane::{
    OperationAuditRequirement, OperationAvailability, OperationCapability, OperationClass,
    OperationDescriptor, OperationIdempotency, OperationRegistry, OperationRisk,
    OperationSourceFamily, SchemaIdentity, CONTROL_PLANE_SCHEMA_VERSION,
};

const MAX_REGISTERED_OPERATIONS: usize = 1024;
const MAIN_RS_SOURCE: &str = include_str!("main.rs");
/// The command source is parsed and validated exactly once.  Local discovery
/// calls only clone this immutable, validated value; they never parse source
/// text or make an external request on the invocation path.
static VALIDATED_REGISTRY: LazyLock<Result<OperationRegistry, ControlPlaneRegistryError>> =
    LazyLock::new(build_registry);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneRegistryError {
    HandlerMarkerMissing,
    MultipleHandlerMarkers,
    HandlerClosingBracketMissing,
    InvalidCommandName(String),
    DuplicateCommandName(String),
    TooManyOperations(usize),
    MissingRegistryDescriptor(String),
    UnexpectedRegistryDescriptor(String),
    DescriptorInvalid(String),
    MissingWindowBinding,
}

impl fmt::Display for ControlPlaneRegistryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HandlerMarkerMissing => {
                formatter.write_str("Tauri generate_handler marker is missing")
            }
            Self::MultipleHandlerMarkers => {
                formatter.write_str("exactly one production Tauri generate_handler is required")
            }
            Self::HandlerClosingBracketMissing => {
                formatter.write_str("Tauri generate_handler closing bracket is missing")
            }
            Self::InvalidCommandName(name) => {
                write!(formatter, "invalid Tauri command name: {name}")
            }
            Self::DuplicateCommandName(name) => {
                write!(formatter, "duplicate Tauri command name: {name}")
            }
            Self::TooManyOperations(count) => write!(
                formatter,
                "operation registry exceeds its {MAX_REGISTERED_OPERATIONS} entry bound: {count}"
            ),
            Self::MissingRegistryDescriptor(name) => write!(
                formatter,
                "registered Tauri command has no descriptor: {name}"
            ),
            Self::UnexpectedRegistryDescriptor(name) => {
                write!(formatter, "descriptor is not registered with Tauri: {name}")
            }
            Self::DescriptorInvalid(message) => {
                write!(formatter, "invalid control-plane descriptor: {message}")
            }
            Self::MissingWindowBinding => formatter
                .write_str("control-plane discovery requires a window-bound Tauri invocation"),
        }
    }
}

impl std::error::Error for ControlPlaneRegistryError {}

/// Returns the deterministic, bounded inventory visible to a local Tauri
/// window.  The `window_label` argument makes the security boundary explicit
/// without granting any external or headless adapter.
pub fn registry_for_window(
    window_label: &str,
) -> Result<OperationRegistry, ControlPlaneRegistryError> {
    if window_label.is_empty() {
        return Err(ControlPlaneRegistryError::MissingWindowBinding);
    }
    registry()
}

pub fn registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    VALIDATED_REGISTRY.clone()
}

fn build_registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    let command_names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE)?;
    let engine_descriptors = control_plane_engine_command_descriptors();
    let remote_descriptors = control_plane_remote_descriptors();
    let midi_osc_dmx_descriptors = control_plane_midi_osc_dmx_descriptors();
    let total_operations = command_names.len()
        + engine_descriptors.len()
        + remote_descriptors.len()
        + midi_osc_dmx_descriptors.len();
    if total_operations > MAX_REGISTERED_OPERATIONS {
        return Err(ControlPlaneRegistryError::TooManyOperations(
            total_operations,
        ));
    }
    let mut operations = command_names
        .iter()
        .map(|name| descriptor_for_command(name))
        .collect::<Vec<_>>();
    operations.extend(engine_descriptors);
    operations.extend(remote_descriptors);
    operations.extend(midi_osc_dmx_descriptors);
    operations.sort_by(|left, right| {
        (left.source_family, &left.source_id).cmp(&(right.source_family, &right.source_id))
    });
    let registry = OperationRegistry {
        schema: SchemaIdentity::registry(),
        operations,
    };
    registry
        .validate()
        .map_err(|error| ControlPlaneRegistryError::DescriptorInvalid(error.to_string()))?;
    verify_registry_exact_set(&command_names, &registry)?;
    Ok(registry)
}

fn descriptor_for_command(operation_id: &str) -> OperationDescriptor {
    if operation_id != "get_control_plane_operation_registry" {
        return unavailable_descriptor(operation_id);
    }

    let semantic_operation_id = "syndocal.query.control_plane.registry.v1";
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: semantic_operation_id.to_string(),
        source_family: OperationSourceFamily::TauriCommand,
        source_id: operation_id.to_string(),
        class: OperationClass::Discovery,
        risk: OperationRisk::R0,
        capabilities: vec![
            OperationCapability::ReadOnly,
            OperationCapability::LocalWindowBound,
            // Registry discovery remains observable during Full Lock and has
            // no mutation capability.
            OperationCapability::AllowedDuringFullLock,
            OperationCapability::RegistryDiscovery,
        ],
        availability: OperationAvailability::LocalWindowOnly,
        idempotency: OperationIdempotency::ReadOnly,
        audit: OperationAuditRequirement::NotApplicable,
        request_schema: command_schema(semantic_operation_id, "request"),
        response_schema: command_schema(semantic_operation_id, "response"),
    }
}

fn unavailable_descriptor(operation_id: &str) -> OperationDescriptor {
    let semantic_operation_id = format!("syndocal.inventory.tauri.{operation_id}.v1");
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: semantic_operation_id.clone(),
        source_family: OperationSourceFamily::TauriCommand,
        source_id: operation_id.to_string(),
        class: OperationClass::Mutation,
        risk: OperationRisk::R5,
        capabilities: vec![OperationCapability::LocalWindowBound],
        availability: OperationAvailability::Unavailable,
        idempotency: OperationIdempotency::Mutating,
        audit: OperationAuditRequirement::RequiredBeforeExternalExecution,
        request_schema: command_schema(&semantic_operation_id, "request"),
        response_schema: command_schema(&semantic_operation_id, "response"),
    }
}

fn command_schema(operation_id: &str, direction: &str) -> SchemaIdentity {
    SchemaIdentity {
        name: format!("{operation_id}.{direction}"),
        version: CONTROL_PLANE_SCHEMA_VERSION,
    }
}

/// Parse the single production `generate_handler!` declaration.  This is a
/// compile-time embedded source snapshot, so release inventory never depends
/// on disk I/O.  It is deliberately strict: comments, qualified paths, and
/// duplicate names cause startup discovery to fail instead of inventing an
/// ambiguous operation identity.
pub fn registered_tauri_command_names_from_source(
    source: &str,
) -> Result<Vec<String>, ControlPlaneRegistryError> {
    const MARKER: &str = ".invoke_handler(tauri::generate_handler![";
    if source.match_indices(MARKER).count() > 1 {
        return Err(ControlPlaneRegistryError::MultipleHandlerMarkers);
    }
    let marker_offset = source
        .find(MARKER)
        .ok_or(ControlPlaneRegistryError::HandlerMarkerMissing)?;
    let body_start = marker_offset + MARKER.len();
    let remainder = &source[body_start..];
    let body_end = remainder
        .find("]) ")
        .or_else(|| remainder.find("])\r\n"))
        .or_else(|| remainder.find("])\n"))
        .or_else(|| remainder.find("])"))
        .ok_or(ControlPlaneRegistryError::HandlerClosingBracketMissing)?;
    let mut distinct = BTreeSet::new();
    let mut names = Vec::new();
    for raw_line in remainder[..body_end].lines() {
        let name = raw_line.trim().trim_end_matches(',').trim();
        if name.is_empty() {
            continue;
        }
        if !is_lower_snake_case(name) {
            return Err(ControlPlaneRegistryError::InvalidCommandName(
                name.to_string(),
            ));
        }
        if !distinct.insert(name.to_string()) {
            return Err(ControlPlaneRegistryError::DuplicateCommandName(
                name.to_string(),
            ));
        }
        names.push(name.to_string());
    }
    if names.is_empty() {
        return Err(ControlPlaneRegistryError::HandlerMarkerMissing);
    }
    names.sort();
    Ok(names)
}

pub fn verify_registry_exact_set(
    command_names: &[String],
    registry: &OperationRegistry,
) -> Result<(), ControlPlaneRegistryError> {
    let command_names = command_names.iter().collect::<BTreeSet<_>>();
    let descriptor_source_ids = registry
        .operations
        .iter()
        .filter(|descriptor| descriptor.source_family == OperationSourceFamily::TauriCommand)
        .map(|descriptor| &descriptor.source_id)
        .collect::<BTreeSet<_>>();
    if let Some(missing) = command_names.difference(&descriptor_source_ids).next() {
        return Err(ControlPlaneRegistryError::MissingRegistryDescriptor(
            (*missing).to_string(),
        ));
    }
    if let Some(unexpected) = descriptor_source_ids.difference(&command_names).next() {
        return Err(ControlPlaneRegistryError::UnexpectedRegistryDescriptor(
            (*unexpected).to_string(),
        ));
    }
    Ok(())
}

fn is_lower_snake_case(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_handler_and_registry_have_the_exact_same_set() {
        let names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE).unwrap();
        let registry = registry().unwrap();
        const R0_ALLOWLIST: [&str; 1] = ["syndocal.query.control_plane.registry.v1"];
        assert_eq!(names.len(), 438);
        const ENGINE_COMMAND_COUNT: usize = 247;
        const REMOTE_INPUT_EVENT_COUNT: usize = 51;
        const REMOTE_CLIENT_REQUEST_COUNT: usize = 7;
        const REMOTE_WIRE_OPERATION_COUNT: usize = 58;
        const REMOTE_OPERATION_COUNT: usize =
            REMOTE_INPUT_EVENT_COUNT + REMOTE_CLIENT_REQUEST_COUNT + REMOTE_WIRE_OPERATION_COUNT;
        const MIDI_CONTROL_MESSAGE_COUNT: usize = 4;
        const MIDI_CONTROL_ACTION_COUNT: usize = 51;
        const MIDI_CLOCK_EVENT_COUNT: usize = 6;
        const MIDI_CONTROL_EVENT_COUNT: usize = 47;
        const OSC_CONTROL_ACTION_COUNT: usize = 47;
        const OSC_INPUT_EVENT_COUNT: usize = 47;
        const DMX_INPUT_PROTOCOL_COUNT: usize = 2;
        const DMX_INPUT_EVENT_COUNT: usize = 2;
        const MIDI_OSC_DMX_OPERATION_COUNT: usize = MIDI_CONTROL_MESSAGE_COUNT
            + MIDI_CONTROL_ACTION_COUNT
            + MIDI_CLOCK_EVENT_COUNT
            + MIDI_CONTROL_EVENT_COUNT
            + OSC_CONTROL_ACTION_COUNT
            + OSC_INPUT_EVENT_COUNT
            + DMX_INPUT_PROTOCOL_COUNT
            + DMX_INPUT_EVENT_COUNT;
        assert_eq!(MIDI_OSC_DMX_OPERATION_COUNT, 206);
        assert_eq!(
            registry.operations.len(),
            438 + ENGINE_COMMAND_COUNT + REMOTE_OPERATION_COUNT + MIDI_OSC_DMX_OPERATION_COUNT
        );
        assert_eq!(registry.operations.len(), 1007);
        verify_registry_exact_set(&names, &registry).unwrap();
        let r0 = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.risk == OperationRisk::R0)
            .collect::<Vec<_>>();
        assert_eq!(r0.len(), 1);
        assert_eq!(
            registry.operations.len() - r0.len(),
            437 + ENGINE_COMMAND_COUNT + REMOTE_OPERATION_COUNT + MIDI_OSC_DMX_OPERATION_COUNT
        );
        assert_eq!(r0[0].operation_id, R0_ALLOWLIST[0]);
        assert_eq!(r0[0].source_family, OperationSourceFamily::TauriCommand);
        assert_eq!(r0[0].source_id, "get_control_plane_operation_registry");
        assert_eq!(r0[0].class, OperationClass::Discovery);
        assert_eq!(
            r0[0].capabilities,
            vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                OperationCapability::AllowedDuringFullLock,
                OperationCapability::RegistryDiscovery,
            ]
        );
        let tauri_unavailable = registry.operations.iter().filter(|descriptor| {
            descriptor.source_family == OperationSourceFamily::TauriCommand
                && !R0_ALLOWLIST.contains(&descriptor.operation_id.as_str())
        });
        assert_eq!(tauri_unavailable.clone().count(), 437);
        for descriptor in tauri_unavailable {
            assert_eq!(
                descriptor.risk,
                OperationRisk::R5,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.availability,
                OperationAvailability::Unavailable,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.idempotency,
                OperationIdempotency::Mutating,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution,
                "{}",
                descriptor.operation_id
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::LocalWindowBound],
                "{}",
                descriptor.operation_id
            );
        }
        let engine_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.source_family == OperationSourceFamily::EngineCommand)
            .collect::<Vec<_>>();
        assert_eq!(engine_unavailable.len(), ENGINE_COMMAND_COUNT);
        for descriptor in engine_unavailable {
            assert_eq!(descriptor.risk, OperationRisk::R5);
            assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
            assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::InternalInventory]
            );
        }
        let remote_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| {
                matches!(
                    descriptor.source_family,
                    OperationSourceFamily::RemoteInputEvent
                        | OperationSourceFamily::RemoteClientRequest
                        | OperationSourceFamily::RemoteWireOperation
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(remote_unavailable.len(), REMOTE_OPERATION_COUNT);
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteInputEvent
                })
                .count(),
            REMOTE_INPUT_EVENT_COUNT
        );
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteClientRequest
                })
                .count(),
            REMOTE_CLIENT_REQUEST_COUNT
        );
        assert_eq!(
            remote_unavailable
                .iter()
                .filter(|descriptor| {
                    descriptor.source_family == OperationSourceFamily::RemoteWireOperation
                })
                .count(),
            REMOTE_WIRE_OPERATION_COUNT
        );
        for descriptor in remote_unavailable {
            assert_eq!(descriptor.risk, OperationRisk::R5);
            assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
            assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::InternalInventory]
            );
        }
        let midi_osc_dmx_families = [
            (
                OperationSourceFamily::MidiControlMessage,
                MIDI_CONTROL_MESSAGE_COUNT,
            ),
            (
                OperationSourceFamily::MidiControlAction,
                MIDI_CONTROL_ACTION_COUNT,
            ),
            (
                OperationSourceFamily::MidiClockEvent,
                MIDI_CLOCK_EVENT_COUNT,
            ),
            (
                OperationSourceFamily::MidiControlEvent,
                MIDI_CONTROL_EVENT_COUNT,
            ),
            (
                OperationSourceFamily::OscControlAction,
                OSC_CONTROL_ACTION_COUNT,
            ),
            (OperationSourceFamily::OscInputEvent, OSC_INPUT_EVENT_COUNT),
            (
                OperationSourceFamily::DmxInputProtocol,
                DMX_INPUT_PROTOCOL_COUNT,
            ),
            (OperationSourceFamily::DmxInputEvent, DMX_INPUT_EVENT_COUNT),
        ];
        let midi_osc_dmx_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| {
                midi_osc_dmx_families
                    .iter()
                    .any(|(family, _)| descriptor.source_family == *family)
            })
            .collect::<Vec<_>>();
        assert_eq!(midi_osc_dmx_unavailable.len(), MIDI_OSC_DMX_OPERATION_COUNT);
        for (family, expected_count) in midi_osc_dmx_families {
            assert_eq!(
                midi_osc_dmx_unavailable
                    .iter()
                    .filter(|descriptor| descriptor.source_family == family)
                    .count(),
                expected_count,
                "{family:?}"
            );
        }
        for descriptor in midi_osc_dmx_unavailable {
            assert_eq!(descriptor.class, OperationClass::Mutation);
            assert_eq!(descriptor.risk, OperationRisk::R5);
            assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
            assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
            assert_eq!(
                descriptor.audit,
                OperationAuditRequirement::RequiredBeforeExternalExecution
            );
            assert_eq!(
                descriptor.capabilities,
                vec![OperationCapability::InternalInventory]
            );
        }
    }

    #[test]
    fn duplicate_handler_command_is_rejected() {
        let source =
            ".invoke_handler(tauri::generate_handler![\n  get_snapshot,\n  get_snapshot,\n])";
        assert_eq!(
            registered_tauri_command_names_from_source(source),
            Err(ControlPlaneRegistryError::DuplicateCommandName(
                "get_snapshot".to_string()
            ))
        );
    }

    #[test]
    fn multiple_production_handler_blocks_are_rejected() {
        let source = ".invoke_handler(tauri::generate_handler![\n  get_snapshot,\n])\n.invoke_handler(tauri::generate_handler![\n  get_snapshot_delta,\n])";
        assert_eq!(
            registered_tauri_command_names_from_source(source),
            Err(ControlPlaneRegistryError::MultipleHandlerMarkers)
        );
    }

    #[test]
    fn missing_or_new_registry_descriptor_is_rejected() {
        let names = vec!["get_snapshot".to_string()];
        let mut registry = OperationRegistry {
            schema: SchemaIdentity::registry(),
            operations: Vec::new(),
        };
        assert_eq!(
            verify_registry_exact_set(&names, &registry),
            Err(ControlPlaneRegistryError::MissingRegistryDescriptor(
                "get_snapshot".to_string()
            ))
        );
        registry
            .operations
            .push(descriptor_for_command("get_snapshot"));
        registry
            .operations
            .push(unavailable_descriptor("new_command"));
        assert_eq!(
            verify_registry_exact_set(&names, &registry),
            Err(ControlPlaneRegistryError::UnexpectedRegistryDescriptor(
                "new_command".to_string()
            ))
        );
    }

    #[test]
    fn conservative_default_never_makes_a_mutation_externally_available() {
        let descriptor = unavailable_descriptor("set_blackout");
        assert_eq!(descriptor.risk, OperationRisk::R5);
        assert_eq!(descriptor.availability, OperationAvailability::Unavailable);
        assert_eq!(descriptor.idempotency, OperationIdempotency::Mutating);
        assert!(descriptor.validate().is_ok());
    }

    #[test]
    fn registry_requires_a_window_binding() {
        assert_eq!(
            registry_for_window(""),
            Err(ControlPlaneRegistryError::MissingWindowBinding)
        );
    }
}
