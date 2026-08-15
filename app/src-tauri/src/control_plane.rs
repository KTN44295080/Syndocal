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
use protocol::control_plane_registry_v2::{
    AdapterPolicy, CanonicalControlPlaneRegistry, CanonicalOperationDescriptor,
    CanonicalRegistryValidationError, ConsentPolicy, PayloadPolicy, RatePolicy, ReceiptPolicy,
    SourceDisposition, SourceInventoryDescriptor, SourceKey, SourceRole, TypedSchemaProjection,
};

const MAX_REGISTERED_OPERATIONS: usize = 2048;
const MAIN_RS_SOURCE: &str = include_str!("main.rs");
const FRONTEND_INVOKE_MANIFEST: &str = include_str!("../../src/tauri-invoke-manifest.json");
/// The command source is parsed and validated exactly once.  Local discovery
/// calls only clone this immutable, validated value; they never parse source
/// text or make an external request on the invocation path.
static VALIDATED_REGISTRY: LazyLock<Result<OperationRegistry, ControlPlaneRegistryError>> =
    LazyLock::new(build_registry);
static VALIDATED_CANONICAL_REGISTRY: LazyLock<
    Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError>,
> = LazyLock::new(build_canonical_registry);

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlPlaneRegistryError {
    HandlerMarkerMissing,
    MultipleHandlerMarkers,
    HandlerClosingBracketMissing,
    InvalidCommandName(String),
    DuplicateCommandName(String),
    InvalidFrontendInvokeManifest,
    DuplicateFrontendInvokeCommand(String),
    UnregisteredFrontendInvokeCommand(String),
    TooManyOperations(usize),
    MissingRegistryDescriptor(String),
    UnexpectedRegistryDescriptor(String),
    DescriptorInvalid(String),
    CanonicalRegistryInvalid(String),
    MissingCanonicalSource(String),
    UnexpectedCanonicalSource(String),
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
            Self::InvalidFrontendInvokeManifest => {
                formatter.write_str("frontend invoke manifest is invalid or not byte-sorted")
            }
            Self::DuplicateFrontendInvokeCommand(name) => {
                write!(formatter, "duplicate frontend invoke command: {name}")
            }
            Self::UnregisteredFrontendInvokeCommand(name) => {
                write!(
                    formatter,
                    "frontend invoke command is not registered with Tauri: {name}"
                )
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
            Self::CanonicalRegistryInvalid(message) => {
                write!(
                    formatter,
                    "invalid canonical control-plane registry: {message}"
                )
            }
            Self::MissingCanonicalSource(key) => {
                write!(formatter, "canonical source inventory is missing: {key}")
            }
            Self::UnexpectedCanonicalSource(key) => {
                write!(formatter, "canonical source inventory is not in v1: {key}")
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

/// Returns the version-2 canonical/source inventory for an injected local
/// Tauri window. This is discovery metadata only; it creates no external
/// adapter and carries no caller-owner argument.
pub fn canonical_registry_for_window(
    window_label: &str,
) -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    if window_label.is_empty() {
        return Err(ControlPlaneRegistryError::MissingWindowBinding);
    }
    canonical_registry()
}

pub fn registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    VALIDATED_REGISTRY.clone()
}

pub fn canonical_registry() -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    VALIDATED_CANONICAL_REGISTRY.clone()
}

fn build_registry() -> Result<OperationRegistry, ControlPlaneRegistryError> {
    let command_names = registered_tauri_command_names_from_source(MAIN_RS_SOURCE)?;
    let frontend_invoke_names = frontend_invoke_names_from_manifest(FRONTEND_INVOKE_MANIFEST)?;
    for name in &frontend_invoke_names {
        if command_names.binary_search(name).is_err() {
            return Err(ControlPlaneRegistryError::UnregisteredFrontendInvokeCommand(name.clone()));
        }
    }
    let engine_descriptors = control_plane_engine_command_descriptors();
    let remote_descriptors = control_plane_remote_descriptors();
    let midi_osc_dmx_descriptors = control_plane_midi_osc_dmx_descriptors();
    let total_operations = command_names.len()
        + engine_descriptors.len()
        + remote_descriptors.len()
        + midi_osc_dmx_descriptors.len()
        + frontend_invoke_names.len();
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
    operations.extend(
        frontend_invoke_names
            .iter()
            .map(|name| unavailable_frontend_invoke_descriptor(name)),
    );
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

/// Builds v2 solely from the already validated exact v1 source inventory.
/// In particular, the v1 descriptor's source risk is deliberately not read as
/// canonical authority: only the explicit reviewed query table below creates
/// a canonical operation.
fn build_canonical_registry() -> Result<CanonicalControlPlaneRegistry, ControlPlaneRegistryError> {
    let legacy = registry()?;
    let mut canonical_operations = legacy
        .operations
        .iter()
        .filter(|descriptor| descriptor.source_family == OperationSourceFamily::TauriCommand)
        .filter_map(|descriptor| {
            let reviewed = reviewed_query_operation(&descriptor.source_id)?;
            Some(CanonicalOperationDescriptor {
                schema: CanonicalOperationDescriptor::schema_identity(),
                operation_id: reviewed.operation_id.to_string(),
                class: reviewed.class,
                risk: OperationRisk::R0,
                capabilities: vec![
                    OperationCapability::ReadOnly,
                    OperationCapability::LocalWindowBound,
                    OperationCapability::AllowedDuringFullLock,
                    reviewed.domain_capability,
                ],
                request_schema: descriptor.request_schema.clone(),
                response_schema: descriptor.response_schema.clone(),
                idempotency: OperationIdempotency::ReadOnly,
                audit: OperationAuditRequirement::NotApplicable,
                adapter_policy: AdapterPolicy::LocalWindowReadOnly,
                receipt_policy: ReceiptPolicy::FailClosed,
                rate_policy: RatePolicy::FailClosed,
                payload_policy: PayloadPolicy::FailClosed,
                consent_policy: ConsentPolicy::FailClosed,
                derived_adapters: Vec::new(),
            })
        })
        .collect::<Vec<_>>();
    canonical_operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));

    let mut source_inventory = legacy
        .operations
        .iter()
        .map(canonical_source_inventory_descriptor)
        .collect::<Vec<_>>();
    source_inventory.sort_by(|left, right| left.source_key.cmp(&right.source_key));

    let mut canonical = CanonicalControlPlaneRegistry {
        schema: CanonicalControlPlaneRegistry::schema_identity(),
        canonical_operations,
        source_inventory,
    };
    canonical
        .populate_derived_adapters()
        .map_err(canonical_registry_error)?;
    verify_canonical_registry_exact_sources(&legacy, &canonical)?;
    Ok(canonical)
}

fn canonical_source_inventory_descriptor(
    descriptor: &OperationDescriptor,
) -> SourceInventoryDescriptor {
    let source_key = SourceKey::new(descriptor.source_family, descriptor.source_id.clone());
    let disposition = match descriptor.source_family {
        OperationSourceFamily::TauriCommand => {
            if let Some(reviewed) = reviewed_query_operation(&descriptor.source_id) {
                SourceDisposition::Operation {
                    canonical_operation_id: reviewed.operation_id.to_string(),
                    projection: TypedSchemaProjection::Exact,
                }
            } else {
                SourceDisposition::Unclassified {
                    reason: unclassified_reason(descriptor.source_family).to_string(),
                }
            }
        }
        OperationSourceFamily::FrontendInvoke => SourceDisposition::AliasOfSource {
            target: SourceKey::new(
                OperationSourceFamily::TauriCommand,
                descriptor.source_id.clone(),
            ),
        },
        _ => SourceDisposition::Unclassified {
            reason: unclassified_reason(descriptor.source_family).to_string(),
        },
    };
    SourceInventoryDescriptor {
        schema: SourceInventoryDescriptor::schema_identity(),
        binding_id: source_key.binding_id(),
        source_key,
        role: SourceRole::for_family(descriptor.source_family),
        raw_request_schema: descriptor.request_schema.clone(),
        raw_response_schema: descriptor.response_schema.clone(),
        disposition,
    }
}

fn unclassified_reason(family: OperationSourceFamily) -> &'static str {
    match family {
        OperationSourceFamily::TauriCommand => "unreviewed tauri command",
        OperationSourceFamily::EngineCommand => "unreviewed engine command",
        OperationSourceFamily::RemoteInputEvent
        | OperationSourceFamily::RemoteClientRequest
        | OperationSourceFamily::RemoteWireOperation => "unreviewed remote ingress",
        OperationSourceFamily::MidiControlMessage
        | OperationSourceFamily::MidiControlAction
        | OperationSourceFamily::MidiClockEvent
        | OperationSourceFamily::MidiControlEvent => "unreviewed midi ingress",
        OperationSourceFamily::OscControlAction | OperationSourceFamily::OscInputEvent => {
            "unreviewed osc ingress"
        }
        OperationSourceFamily::DmxInputProtocol | OperationSourceFamily::DmxInputEvent => {
            "unreviewed dmx ingress"
        }
        OperationSourceFamily::FrontendInvoke => "unreviewed frontend invocation",
    }
}

fn canonical_registry_error(error: CanonicalRegistryValidationError) -> ControlPlaneRegistryError {
    ControlPlaneRegistryError::CanonicalRegistryInvalid(error.to_string())
}

pub fn verify_canonical_registry_exact_sources(
    legacy: &OperationRegistry,
    canonical: &CanonicalControlPlaneRegistry,
) -> Result<(), ControlPlaneRegistryError> {
    let expected = legacy
        .operations
        .iter()
        .map(|descriptor| SourceKey::new(descriptor.source_family, descriptor.source_id.clone()))
        .collect::<BTreeSet<_>>();
    let actual = canonical
        .source_inventory
        .iter()
        .map(|source| source.source_key.clone())
        .collect::<BTreeSet<_>>();
    if let Some(missing) = expected.difference(&actual).next() {
        return Err(ControlPlaneRegistryError::MissingCanonicalSource(
            missing.binding_id(),
        ));
    }
    if let Some(unexpected) = actual.difference(&expected).next() {
        return Err(ControlPlaneRegistryError::UnexpectedCanonicalSource(
            unexpected.binding_id(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct ReviewedQueryOperation {
    operation_id: &'static str,
    class: OperationClass,
    domain_capability: OperationCapability,
}

fn reviewed_query_operation(command: &str) -> Option<ReviewedQueryOperation> {
    let (operation_id, class, domain_capability) = match command {
        "get_control_plane_operation_registry" => (
            "syndocal.query.control_plane.registry.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_canonical_registry" => (
            "syndocal.query.control_plane.canonical_registry.v2",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_query_schema_catalog" => (
            "syndocal.query.control_plane.schemas.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "get_control_plane_query_capabilities" => (
            "syndocal.query.control_plane.capabilities.v1",
            OperationClass::Discovery,
            OperationCapability::RegistryDiscovery,
        ),
        "query_control_plane_project_authority" => (
            "syndocal.query.project.authority.v1",
            OperationClass::ProjectAuthority,
            OperationCapability::ProjectAuthorityRead,
        ),
        "query_control_plane_runtime_generations" => (
            "syndocal.query.runtime.generations.v1",
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        "query_control_plane_output_ownership" => (
            "syndocal.query.output.ownership.v1",
            OperationClass::OutputOwnership,
            OperationCapability::OutputOwnershipRead,
        ),
        "poll_control_plane_observation_events" => (
            "syndocal.query.events.observations.v1",
            OperationClass::RuntimeObservation,
            OperationCapability::RuntimeRead,
        ),
        _ => return None,
    };
    Some(ReviewedQueryOperation {
        operation_id,
        class,
        domain_capability,
    })
}

fn descriptor_for_command(operation_id: &str) -> OperationDescriptor {
    let Some(reviewed) = reviewed_query_operation(operation_id) else {
        return unavailable_descriptor(operation_id);
    };
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: reviewed.operation_id.to_string(),
        source_family: OperationSourceFamily::TauriCommand,
        source_id: operation_id.to_string(),
        class: reviewed.class,
        risk: OperationRisk::R0,
        capabilities: vec![
            OperationCapability::ReadOnly,
            OperationCapability::LocalWindowBound,
            // Registry discovery remains observable during Full Lock and has
            // no mutation capability.
            OperationCapability::AllowedDuringFullLock,
            reviewed.domain_capability,
        ],
        availability: OperationAvailability::LocalWindowOnly,
        idempotency: OperationIdempotency::ReadOnly,
        audit: OperationAuditRequirement::NotApplicable,
        request_schema: command_schema(reviewed.operation_id, "request"),
        response_schema: command_schema(reviewed.operation_id, "response"),
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

fn unavailable_frontend_invoke_descriptor(source_id: &str) -> OperationDescriptor {
    let operation_id = format!("syndocal.inventory.frontend.invoke.{source_id}.v1");
    OperationDescriptor {
        schema: SchemaIdentity::descriptor(),
        operation_id: operation_id.clone(),
        source_family: OperationSourceFamily::FrontendInvoke,
        source_id: source_id.to_string(),
        class: OperationClass::Mutation,
        risk: OperationRisk::R5,
        capabilities: vec![OperationCapability::InternalInventory],
        availability: OperationAvailability::Unavailable,
        idempotency: OperationIdempotency::Mutating,
        audit: OperationAuditRequirement::RequiredBeforeExternalExecution,
        request_schema: command_schema(&operation_id, "request"),
        response_schema: command_schema(&operation_id, "response"),
    }
}

fn frontend_invoke_names_from_manifest(
    source: &str,
) -> Result<Vec<String>, ControlPlaneRegistryError> {
    let names = serde_json::from_str::<Vec<String>>(source)
        .map_err(|_| ControlPlaneRegistryError::InvalidFrontendInvokeManifest)?;
    let mut prior: Option<&str> = None;
    for name in &names {
        if !is_lower_snake_case(name) {
            return Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest);
        }
        if let Some(prior) = prior {
            if name == prior {
                return Err(ControlPlaneRegistryError::DuplicateFrontendInvokeCommand(
                    name.clone(),
                ));
            }
            if name.as_str() < prior {
                return Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest);
            }
        }
        prior = Some(name);
    }
    Ok(names)
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
        const R0_ALLOWLIST: [&str; 8] = [
            "syndocal.query.control_plane.registry.v1",
            "syndocal.query.control_plane.canonical_registry.v2",
            "syndocal.query.control_plane.capabilities.v1",
            "syndocal.query.control_plane.schemas.v1",
            "syndocal.query.events.observations.v1",
            "syndocal.query.output.ownership.v1",
            "syndocal.query.project.authority.v1",
            "syndocal.query.runtime.generations.v1",
        ];
        assert_eq!(names.len(), 445);
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
        const FRONTEND_INVOKE_COUNT: usize = 386;
        assert_eq!(MIDI_OSC_DMX_OPERATION_COUNT, 206);
        assert_eq!(
            registry.operations.len(),
            445 + ENGINE_COMMAND_COUNT
                + REMOTE_OPERATION_COUNT
                + MIDI_OSC_DMX_OPERATION_COUNT
                + FRONTEND_INVOKE_COUNT
        );
        assert_eq!(registry.operations.len(), 1400);
        verify_registry_exact_set(&names, &registry).unwrap();
        let r0 = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.risk == OperationRisk::R0)
            .collect::<Vec<_>>();
        assert_eq!(r0.len(), R0_ALLOWLIST.len());
        assert_eq!(
            registry.operations.len() - r0.len(),
            437 + ENGINE_COMMAND_COUNT
                + REMOTE_OPERATION_COUNT
                + MIDI_OSC_DMX_OPERATION_COUNT
                + FRONTEND_INVOKE_COUNT
        );
        assert_eq!(
            r0.iter()
                .map(|descriptor| descriptor.operation_id.as_str())
                .collect::<BTreeSet<_>>(),
            R0_ALLOWLIST.into_iter().collect::<BTreeSet<_>>()
        );
        for descriptor in &r0 {
            assert_eq!(
                descriptor.source_family,
                OperationSourceFamily::TauriCommand
            );
            assert_eq!(descriptor.risk, OperationRisk::R0);
            assert_eq!(
                descriptor.availability,
                OperationAvailability::LocalWindowOnly
            );
            assert_eq!(descriptor.idempotency, OperationIdempotency::ReadOnly);
            assert_eq!(descriptor.audit, OperationAuditRequirement::NotApplicable);
            assert!(descriptor
                .capabilities
                .contains(&OperationCapability::AllowedDuringFullLock));
        }
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
        let frontend_manifest =
            frontend_invoke_names_from_manifest(FRONTEND_INVOKE_MANIFEST).unwrap();
        assert_eq!(frontend_manifest.len(), FRONTEND_INVOKE_COUNT);
        let frontend_unavailable = registry
            .operations
            .iter()
            .filter(|descriptor| descriptor.source_family == OperationSourceFamily::FrontendInvoke)
            .collect::<Vec<_>>();
        assert_eq!(frontend_unavailable.len(), FRONTEND_INVOKE_COUNT);
        assert_eq!(
            frontend_unavailable
                .iter()
                .map(|descriptor| descriptor.source_id.as_str())
                .collect::<BTreeSet<_>>(),
            frontend_manifest
                .iter()
                .map(String::as_str)
                .collect::<BTreeSet<_>>()
        );
        for descriptor in frontend_unavailable {
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
            assert_eq!(
                descriptor.operation_id,
                format!(
                    "syndocal.inventory.frontend.invoke.{}.v1",
                    descriptor.source_id
                )
            );
        }
    }

    #[test]
    fn canonical_registry_derives_the_exact_current_source_inventory() {
        let legacy = registry().unwrap();
        let canonical = canonical_registry().unwrap();
        canonical.validate().unwrap();
        verify_canonical_registry_exact_sources(&legacy, &canonical).unwrap();

        const TAURI_COUNT: usize = 445;
        const ENGINE_COUNT: usize = 247;
        const REMOTE_COUNT: usize = 116;
        const MIDI_OSC_DMX_COUNT: usize = 206;
        const FRONTEND_COUNT: usize = 386;
        const SOURCE_TOTAL: usize =
            TAURI_COUNT + ENGINE_COUNT + REMOTE_COUNT + MIDI_OSC_DMX_COUNT + FRONTEND_COUNT;
        assert_eq!(SOURCE_TOTAL, 1400);
        assert_eq!(canonical.source_inventory.len(), SOURCE_TOTAL);
        assert_eq!(canonical.canonical_operations.len(), 8);

        let count_family = |family| {
            canonical
                .source_inventory
                .iter()
                .filter(|source| source.source_key.family == family)
                .count()
        };
        assert_eq!(
            count_family(OperationSourceFamily::TauriCommand),
            TAURI_COUNT
        );
        assert_eq!(
            count_family(OperationSourceFamily::EngineCommand),
            ENGINE_COUNT
        );
        assert_eq!(
            count_family(OperationSourceFamily::RemoteInputEvent)
                + count_family(OperationSourceFamily::RemoteClientRequest)
                + count_family(OperationSourceFamily::RemoteWireOperation),
            REMOTE_COUNT
        );
        assert_eq!(
            count_family(OperationSourceFamily::MidiControlMessage)
                + count_family(OperationSourceFamily::MidiControlAction)
                + count_family(OperationSourceFamily::MidiClockEvent)
                + count_family(OperationSourceFamily::MidiControlEvent)
                + count_family(OperationSourceFamily::OscControlAction)
                + count_family(OperationSourceFamily::OscInputEvent)
                + count_family(OperationSourceFamily::DmxInputProtocol)
                + count_family(OperationSourceFamily::DmxInputEvent),
            MIDI_OSC_DMX_COUNT
        );
        assert_eq!(
            count_family(OperationSourceFamily::FrontendInvoke),
            FRONTEND_COUNT
        );

        let direct = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::Operation { .. }))
            .collect::<Vec<_>>();
        let aliases = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::AliasOfSource { .. }))
            .collect::<Vec<_>>();
        let unclassified = canonical
            .source_inventory
            .iter()
            .filter(|source| matches!(&source.disposition, SourceDisposition::Unclassified { .. }))
            .collect::<Vec<_>>();
        assert_eq!(direct.len(), 8);
        assert_eq!(aliases.len(), FRONTEND_COUNT);
        assert_eq!(unclassified.len(), 1006);
        assert_eq!(
            direct.len() + aliases.len() + unclassified.len(),
            SOURCE_TOTAL
        );

        for source in &direct {
            assert_eq!(
                source.source_key.family,
                OperationSourceFamily::TauriCommand
            );
            assert_eq!(source.role, SourceRole::LocalWindowCommand);
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_some());
        }
        for source in &aliases {
            assert_eq!(
                source.source_key.family,
                OperationSourceFamily::FrontendInvoke
            );
            assert_eq!(source.role, SourceRole::FrontendInvocation);
            let SourceDisposition::AliasOfSource { target } = &source.disposition else {
                unreachable!("filtered aliases must retain their disposition");
            };
            assert_eq!(target.family, OperationSourceFamily::TauriCommand);
            assert_eq!(target.source_id, source.source_key.source_id);
            assert_eq!(
                canonical
                    .canonical_operation_for_source(&source.source_key)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str()),
                canonical
                    .canonical_operation_for_source(target)
                    .unwrap()
                    .map(|operation| operation.operation_id.as_str())
            );
        }
        for source in &unclassified {
            assert!(canonical
                .canonical_operation_for_source(&source.source_key)
                .unwrap()
                .is_none());
        }
        for operation in &canonical.canonical_operations {
            assert_eq!(operation.risk, OperationRisk::R0);
            assert_eq!(operation.idempotency, OperationIdempotency::ReadOnly);
            assert_eq!(operation.audit, OperationAuditRequirement::NotApplicable);
            assert_eq!(operation.adapter_policy, AdapterPolicy::LocalWindowReadOnly);
            assert_eq!(operation.derived_adapters.len(), 1);
        }

        let canonical_json = serde_json::to_value(&canonical).unwrap();
        for source in canonical_json["source_inventory"].as_array().unwrap() {
            let source = source.as_object().unwrap();
            assert!(!source.contains_key("risk"));
            assert!(!source.contains_key("availability"));
            assert!(!source.contains_key("adapter_policy"));
        }
    }

    #[test]
    fn canonical_registry_rejects_a_dropped_v1_source() {
        let legacy = registry().unwrap();
        let mut canonical = canonical_registry().unwrap();
        canonical.source_inventory.pop();
        assert!(matches!(
            verify_canonical_registry_exact_sources(&legacy, &canonical),
            Err(ControlPlaneRegistryError::MissingCanonicalSource(_))
        ));
    }

    #[test]
    fn legacy_registry_only_adds_the_canonical_query_endpoint_row() {
        let legacy = registry().unwrap();
        let encoded = serde_json::to_value(&legacy).unwrap();
        let operations = encoded["operations"].as_array().unwrap();
        let endpoint_rows = operations
            .iter()
            .filter(|operation| {
                operation["source_family"] == "tauri_command"
                    && operation["source_id"] == "get_control_plane_canonical_registry"
            })
            .collect::<Vec<_>>();
        assert_eq!(endpoint_rows.len(), 1);
        let endpoint = endpoint_rows[0];
        assert_eq!(
            endpoint["operation_id"],
            "syndocal.query.control_plane.canonical_registry.v2"
        );
        assert_eq!(endpoint["risk"], "r0");
        assert_eq!(endpoint["availability"], "local_window_only");
        assert_eq!(endpoint["idempotency"], "read_only");
        assert_eq!(endpoint["audit"], "not_applicable");
        assert_eq!(
            endpoint["capabilities"],
            serde_json::json!([
                "read_only",
                "local_window_bound",
                "allowed_during_full_lock",
                "registry_discovery"
            ])
        );
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
    fn frontend_manifest_rejects_duplicate_unsorted_and_invalid_commands() {
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"get_snapshot\",\"get_snapshot\"]"),
            Err(ControlPlaneRegistryError::DuplicateFrontendInvokeCommand(
                "get_snapshot".to_string()
            ))
        );
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"set_bpm\",\"get_snapshot\"]"),
            Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest)
        );
        assert_eq!(
            frontend_invoke_names_from_manifest("[\"GetSnapshot\"]"),
            Err(ControlPlaneRegistryError::InvalidFrontendInvokeManifest)
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
