//! Versioned, fail-closed descriptions of the local control-plane surface.
//!
//! These types describe an operation; they do not grant authority to execute
//! one.  In particular, an `OperationAvailability::LocalWindowOnly` descriptor
//! is still not an MCP, HTTP, or remote-control adapter.

use std::{collections::BTreeSet, fmt};

use serde::{de::Error as _, Deserialize, Deserializer, Serialize};

use crate::control_plane_command::{
    OUTPUT_BLACKOUT_RELEASE_OPERATION_ID, OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION,
    OUTPUT_DISPLAY_ADD_OPERATION_ID, OUTPUT_ENABLE_OPERATION_ID, OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
    OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID, OUTPUT_LEASE_RECOVER_OPERATION_ID,
    OUTPUT_LEASE_RELINQUISH_OPERATION_ID, OUTPUT_LEASE_RENEW_OPERATION_ID,
    OUTPUT_OWNERSHIP_ARM_OPERATION_ID, OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
};

/// Wire format version for the control-plane inventory.
pub const CONTROL_PLANE_SCHEMA_VERSION: u16 = 1;
pub const OPERATION_DESCRIPTOR_SCHEMA_NAME: &str = "syndocal.control-plane.operation-descriptor";
pub const OPERATION_REGISTRY_SCHEMA_NAME: &str = "syndocal.control-plane.operation-registry";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SchemaIdentity {
    pub name: String,
    pub version: u16,
}

impl SchemaIdentity {
    pub fn descriptor() -> Self {
        Self {
            name: OPERATION_DESCRIPTOR_SCHEMA_NAME.to_string(),
            version: CONTROL_PLANE_SCHEMA_VERSION,
        }
    }

    pub fn registry() -> Self {
        Self {
            name: OPERATION_REGISTRY_SCHEMA_NAME.to_string(),
            version: CONTROL_PLANE_SCHEMA_VERSION,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    Discovery,
    ProjectAuthority,
    RuntimeObservation,
    OutputOwnership,
    DeviceHealth,
    TerminalReceipt,
    Mutation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationRisk {
    R0,
    R1,
    R2,
    R3,
    R4,
    R5,
    /// Safer-direction-only emergency operations. This is not ordered above
    /// R5 and never implies a general administrator capability.
    S0,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationCapability {
    ReadOnly,
    LocalWindowBound,
    /// A bounded, server-authoritative project mutation. This does not imply
    /// a remote, HTTP, MCP, or Full-Lock adapter.
    AuthoritativeProjectMutation,
    /// A typed, local-window-only runtime mutation. Unlike project mutation,
    /// it never writes history or persistence; it remains separately fenced
    /// by an engine-owned runtime generation.
    AuthoritativeRuntimeMutation,
    /// Narrow safer-direction capability for emergency lighting blackout
    /// engagement. It cannot authorize release or any other S0 operation.
    SafetyBlackoutEngage,
    /// Local-only output control. This capability is always paired with the
    /// reviewed R4 output-control adapter and a local-window policy; it never
    /// implies an external principal or grant.
    OutputControl,
    /// Marks an internal source-family inventory entry.  It grants neither a
    /// local-window invocation nor an external execution adapter.
    InternalInventory,
    AllowedDuringFullLock,
    RegistryDiscovery,
    ProjectAuthorityRead,
    RuntimeRead,
    OutputOwnershipRead,
    DeviceHealthRead,
    TerminalReceiptRead,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAvailability {
    /// The existing local Tauri invocation may observe this operation.
    /// No remote transport or agent adapter is implied.
    LocalWindowOnly,
    /// This command is inventoried but deliberately has no control-plane
    /// execution adapter.
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationIdempotency {
    ReadOnly,
    Mutating,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationAuditRequirement {
    NotApplicable,
    RequiredBeforeExternalExecution,
    /// Every accepted attempt is appended to the immutable safety audit
    /// before a terminal receipt is returned.
    Immutable,
}

/// Namespace that owns the raw operation source identifier. Additional source
/// families are additive and never change an existing semantic operation ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationSourceFamily {
    TauriCommand,
    EngineCommand,
    RemoteInputEvent,
    RemoteClientRequest,
    RemoteWireOperation,
    MidiControlMessage,
    MidiControlAction,
    MidiClockEvent,
    MidiControlEvent,
    OscControlAction,
    OscInputEvent,
    DmxInputProtocol,
    DmxInputEvent,
    FrontendInvoke,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationDescriptor {
    pub schema: SchemaIdentity,
    /// Versioned semantic contract, independent of the implementation name.
    pub operation_id: String,
    pub source_family: OperationSourceFamily,
    /// Raw identifier in `source_family` (a Tauri function name for AI0).
    pub source_id: String,
    pub class: OperationClass,
    pub risk: OperationRisk,
    pub capabilities: Vec<OperationCapability>,
    pub availability: OperationAvailability,
    pub idempotency: OperationIdempotency,
    pub audit: OperationAuditRequirement,
    pub request_schema: SchemaIdentity,
    pub response_schema: SchemaIdentity,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationDescriptorWire {
    schema: SchemaIdentity,
    operation_id: String,
    source_family: OperationSourceFamily,
    source_id: String,
    class: OperationClass,
    risk: OperationRisk,
    capabilities: Vec<OperationCapability>,
    availability: OperationAvailability,
    idempotency: OperationIdempotency,
    audit: OperationAuditRequirement,
    request_schema: SchemaIdentity,
    response_schema: SchemaIdentity,
}

impl From<OperationDescriptorWire> for OperationDescriptor {
    fn from(wire: OperationDescriptorWire) -> Self {
        Self {
            schema: wire.schema,
            operation_id: wire.operation_id,
            source_family: wire.source_family,
            source_id: wire.source_id,
            class: wire.class,
            risk: wire.risk,
            capabilities: wire.capabilities,
            availability: wire.availability,
            idempotency: wire.idempotency,
            audit: wire.audit,
            request_schema: wire.request_schema,
            response_schema: wire.response_schema,
        }
    }
}

impl<'de> Deserialize<'de> for OperationDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let descriptor: OperationDescriptor =
            OperationDescriptorWire::deserialize(deserializer)?.into();
        descriptor.validate().map_err(D::Error::custom)?;
        Ok(descriptor)
    }
}

impl OperationDescriptor {
    pub fn validate(&self) -> Result<(), OperationDescriptorValidationError> {
        validate_schema(&self.schema, OPERATION_DESCRIPTOR_SCHEMA_NAME)?;
        validate_operation_id(&self.operation_id)?;
        validate_source_id(self.source_family, &self.source_id)?;
        validate_operation_schema_identity(&self.request_schema, &self.operation_id, "request")?;
        validate_operation_schema_identity(&self.response_schema, &self.operation_id, "response")?;
        if self.capabilities.is_empty() {
            return Err(OperationDescriptorValidationError::EmptyCapabilities);
        }
        let distinct_capabilities = self.capabilities.iter().copied().collect::<BTreeSet<_>>();
        if distinct_capabilities.len() != self.capabilities.len() {
            return Err(OperationDescriptorValidationError::DuplicateCapability);
        }
        if self.availability == OperationAvailability::LocalWindowOnly {
            validate_local_window_only_descriptor(self, &distinct_capabilities)?;
        }
        if self.availability == OperationAvailability::Unavailable && self.risk != OperationRisk::R5
        {
            return Err(OperationDescriptorValidationError::UnavailableOperationMustBeR5);
        }
        if self.availability == OperationAvailability::Unavailable {
            validate_unavailable_descriptor(self)?;
        }
        Ok(())
    }
}

fn validate_unavailable_descriptor(
    descriptor: &OperationDescriptor,
) -> Result<(), OperationDescriptorValidationError> {
    if descriptor.idempotency != OperationIdempotency::Mutating
        || descriptor.audit != OperationAuditRequirement::RequiredBeforeExternalExecution
    {
        return Err(OperationDescriptorValidationError::UnsafeUnavailableOperation);
    }
    let expected_capability = match descriptor.source_family {
        OperationSourceFamily::TauriCommand => OperationCapability::LocalWindowBound,
        OperationSourceFamily::EngineCommand
        | OperationSourceFamily::RemoteInputEvent
        | OperationSourceFamily::RemoteClientRequest
        | OperationSourceFamily::RemoteWireOperation
        | OperationSourceFamily::MidiControlMessage
        | OperationSourceFamily::MidiControlAction
        | OperationSourceFamily::MidiClockEvent
        | OperationSourceFamily::MidiControlEvent
        | OperationSourceFamily::OscControlAction
        | OperationSourceFamily::OscInputEvent
        | OperationSourceFamily::DmxInputProtocol
        | OperationSourceFamily::DmxInputEvent
        | OperationSourceFamily::FrontendInvoke => OperationCapability::InternalInventory,
    };
    if descriptor.capabilities != vec![expected_capability] {
        return Err(OperationDescriptorValidationError::UnsafeUnavailableOperation);
    }
    Ok(())
}

fn validate_local_window_only_descriptor(
    descriptor: &OperationDescriptor,
    capabilities: &BTreeSet<OperationCapability>,
) -> Result<(), OperationDescriptorValidationError> {
    if descriptor.source_family != OperationSourceFamily::TauriCommand
        || descriptor.risk != OperationRisk::R0
        || descriptor.idempotency != OperationIdempotency::ReadOnly
        || descriptor.audit != OperationAuditRequirement::NotApplicable
        || !capabilities.contains(&OperationCapability::ReadOnly)
        || !capabilities.contains(&OperationCapability::LocalWindowBound)
    {
        return Err(OperationDescriptorValidationError::UnsafeLocalAvailability);
    }
    if !capabilities.contains(&OperationCapability::AllowedDuringFullLock) {
        return Err(OperationDescriptorValidationError::MissingFullLockAllowance);
    }
    let domain_capabilities = capabilities
        .iter()
        .copied()
        .filter(|capability| is_domain_capability(*capability))
        .collect::<Vec<_>>();
    match domain_capabilities.len() {
        0 => return Err(OperationDescriptorValidationError::MissingReviewedDomainCapability),
        1 => {}
        _ => return Err(OperationDescriptorValidationError::MultipleReviewedDomainCapabilities),
    }
    let expected_domain = expected_domain_capability(descriptor.class);
    if descriptor.class != OperationClass::Discovery
        && capabilities.contains(&OperationCapability::RegistryDiscovery)
    {
        return Err(OperationDescriptorValidationError::RegistryDiscoveryOutsideDiscovery);
    }
    if domain_capabilities[0] != expected_domain {
        return Err(OperationDescriptorValidationError::IncompatibleDomainCapability);
    }
    if capabilities.len() != 4 {
        return Err(OperationDescriptorValidationError::UnexpectedLocalCapability);
    }
    Ok(())
}

fn is_domain_capability(capability: OperationCapability) -> bool {
    matches!(
        capability,
        OperationCapability::RegistryDiscovery
            | OperationCapability::ProjectAuthorityRead
            | OperationCapability::RuntimeRead
            | OperationCapability::OutputOwnershipRead
            | OperationCapability::DeviceHealthRead
            | OperationCapability::TerminalReceiptRead
    )
}

fn expected_domain_capability(class: OperationClass) -> OperationCapability {
    match class {
        OperationClass::Discovery => OperationCapability::RegistryDiscovery,
        OperationClass::ProjectAuthority => OperationCapability::ProjectAuthorityRead,
        OperationClass::RuntimeObservation => OperationCapability::RuntimeRead,
        OperationClass::OutputOwnership => OperationCapability::OutputOwnershipRead,
        OperationClass::DeviceHealth => OperationCapability::DeviceHealthRead,
        OperationClass::TerminalReceipt => OperationCapability::TerminalReceiptRead,
        OperationClass::Mutation => OperationCapability::ReadOnly,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationRegistry {
    pub schema: SchemaIdentity,
    pub operations: Vec<OperationDescriptor>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct OperationRegistryWire {
    schema: SchemaIdentity,
    operations: Vec<OperationDescriptor>,
}

impl<'de> Deserialize<'de> for OperationRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OperationRegistryWire::deserialize(deserializer)?;
        let registry = Self {
            schema: wire.schema,
            operations: wire.operations,
        };
        registry.validate().map_err(D::Error::custom)?;
        Ok(registry)
    }
}

impl OperationRegistry {
    pub fn validate(&self) -> Result<(), OperationDescriptorValidationError> {
        validate_schema(&self.schema, OPERATION_REGISTRY_SCHEMA_NAME)?;
        let mut ids = BTreeSet::new();
        let mut sources = BTreeSet::new();
        for operation in &self.operations {
            operation.validate()?;
            if !ids.insert(operation.operation_id.as_str()) {
                return Err(OperationDescriptorValidationError::DuplicateOperationId);
            }
            if !sources.insert((operation.source_family, operation.source_id.as_str())) {
                return Err(OperationDescriptorValidationError::DuplicateOperationSource);
            }
        }
        Ok(())
    }
}

fn validate_schema(
    schema: &SchemaIdentity,
    expected_name: &'static str,
) -> Result<(), OperationDescriptorValidationError> {
    if schema.name != expected_name {
        return Err(OperationDescriptorValidationError::UnexpectedSchemaName);
    }
    if schema.version != CONTROL_PLANE_SCHEMA_VERSION {
        return Err(OperationDescriptorValidationError::UnsupportedSchemaVersion);
    }
    Ok(())
}

fn validate_operation_id(operation_id: &str) -> Result<(), OperationDescriptorValidationError> {
    let mut segments = operation_id.split('.');
    let Some(first) = segments.next() else {
        return Err(OperationDescriptorValidationError::InvalidOperationId);
    };
    if !is_lower_snake_case(first) {
        return Err(OperationDescriptorValidationError::InvalidOperationId);
    }
    let remaining = segments.collect::<Vec<_>>();
    if remaining.is_empty()
        || !remaining[..remaining.len() - 1]
            .iter()
            .all(|segment| is_lower_snake_case(segment))
    {
        return Err(OperationDescriptorValidationError::InvalidOperationId);
    }
    let version = remaining.last().copied().unwrap_or_default();
    let numeric = version.strip_prefix('v').unwrap_or_default();
    if numeric.is_empty()
        || (numeric.len() > 1 && numeric.starts_with('0'))
        || !numeric.bytes().all(|byte| byte.is_ascii_digit())
        || numeric
            .parse::<u32>()
            .ok()
            .filter(|value| *value > 0)
            .is_none()
    {
        return Err(OperationDescriptorValidationError::InvalidOperationId);
    }
    Ok(())
}

fn validate_source_id(
    source_family: OperationSourceFamily,
    source_id: &str,
) -> Result<(), OperationDescriptorValidationError> {
    let valid = match source_family {
        OperationSourceFamily::RemoteWireOperation => is_bounded_lower_camel_ascii(source_id),
        OperationSourceFamily::TauriCommand
        | OperationSourceFamily::EngineCommand
        | OperationSourceFamily::RemoteInputEvent
        | OperationSourceFamily::RemoteClientRequest
        | OperationSourceFamily::MidiControlMessage
        | OperationSourceFamily::MidiControlAction
        | OperationSourceFamily::MidiClockEvent
        | OperationSourceFamily::MidiControlEvent
        | OperationSourceFamily::OscControlAction
        | OperationSourceFamily::OscInputEvent
        | OperationSourceFamily::DmxInputProtocol
        | OperationSourceFamily::DmxInputEvent
        | OperationSourceFamily::FrontendInvoke => is_lower_snake_case(source_id),
    };
    if valid {
        Ok(())
    } else {
        Err(OperationDescriptorValidationError::InvalidSourceId)
    }
}

fn is_bounded_lower_camel_ascii(value: &str) -> bool {
    const MAX_REMOTE_WIRE_SOURCE_ID_BYTES: usize = 64;
    if value.len() > MAX_REMOTE_WIRE_SOURCE_ID_BYTES {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase() && bytes.all(|byte| byte.is_ascii_alphanumeric())
}

fn is_lower_snake_case(value: &str) -> bool {
    let mut characters = value.bytes();
    let Some(first) = characters.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && characters.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn validate_operation_schema_identity(
    schema: &SchemaIdentity,
    operation_id: &str,
    direction: &str,
) -> Result<(), OperationDescriptorValidationError> {
    if schema.name.is_empty() || schema.name != format!("{operation_id}.{direction}") {
        return Err(OperationDescriptorValidationError::InvalidOperationSchemaName);
    }
    let expected_version = match operation_id {
        OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
        | OUTPUT_OWNERSHIP_ARM_OPERATION_ID
        | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
        | OUTPUT_DISPLAY_ADD_OPERATION_ID
        | OUTPUT_ENABLE_OPERATION_ID
        | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
        | OUTPUT_LEASE_RENEW_OPERATION_ID
        | OUTPUT_LEASE_RECOVER_OPERATION_ID
        | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
        | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID => OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION,
        _ => CONTROL_PLANE_SCHEMA_VERSION,
    };
    if schema.version != expected_version {
        return Err(OperationDescriptorValidationError::UnsupportedOperationSchemaVersion);
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperationDescriptorValidationError {
    UnexpectedSchemaName,
    UnsupportedSchemaVersion,
    UnsupportedOperationSchemaVersion,
    InvalidOperationSchemaName,
    InvalidOperationId,
    InvalidSourceId,
    EmptyCapabilities,
    DuplicateCapability,
    DuplicateOperationId,
    DuplicateOperationSource,
    UnsafeLocalAvailability,
    MissingFullLockAllowance,
    MissingReviewedDomainCapability,
    MultipleReviewedDomainCapabilities,
    IncompatibleDomainCapability,
    RegistryDiscoveryOutsideDiscovery,
    UnexpectedLocalCapability,
    UnavailableOperationMustBeR5,
    UnsafeUnavailableOperation,
}

impl fmt::Display for OperationDescriptorValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnexpectedSchemaName => "control-plane schema name is not recognized",
            Self::UnsupportedSchemaVersion => "control-plane schema version is not supported",
            Self::UnsupportedOperationSchemaVersion => {
                "operation request or response schema version is not supported"
            }
            Self::InvalidOperationSchemaName => {
                "operation request and response schema names must match the operation id"
            }
            Self::InvalidOperationId => {
                "operation id must use lowercase dotted segments and end in vN"
            }
            Self::InvalidSourceId => {
                "operation source id does not match its source family's bounded identifier syntax"
            }
            Self::EmptyCapabilities => "operation capabilities must not be empty",
            Self::DuplicateCapability => "operation capabilities must be unique",
            Self::DuplicateOperationId => "operation ids must be unique",
            Self::DuplicateOperationSource => "operation source family and id must be unique",
            Self::UnsafeLocalAvailability => {
                "local availability is limited to R0 read-only window-bound operations"
            }
            Self::MissingFullLockAllowance => {
                "local availability must explicitly allow observation during Full Lock"
            }
            Self::MissingReviewedDomainCapability => {
                "local availability must declare exactly one reviewed domain capability"
            }
            Self::MultipleReviewedDomainCapabilities => {
                "local availability cannot declare more than one reviewed domain capability"
            }
            Self::IncompatibleDomainCapability => {
                "local availability domain capability is incompatible with its operation class"
            }
            Self::RegistryDiscoveryOutsideDiscovery => {
                "registry discovery capability is limited to discovery operations"
            }
            Self::UnexpectedLocalCapability => "local availability has an unreviewed capability",
            Self::UnavailableOperationMustBeR5 => "unavailable operations must remain R5",
            Self::UnsafeUnavailableOperation => {
                "unavailable operations must remain mutating, audited, and carry only their family inventory capability"
            }
        })
    }
}

impl std::error::Error for OperationDescriptorValidationError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn local_read_descriptor() -> OperationDescriptor {
        OperationDescriptor {
            schema: SchemaIdentity::descriptor(),
            operation_id: "syndocal.query.snapshot.v1".to_string(),
            source_family: OperationSourceFamily::TauriCommand,
            source_id: "get_snapshot".to_string(),
            class: OperationClass::RuntimeObservation,
            risk: OperationRisk::R0,
            capabilities: vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                OperationCapability::AllowedDuringFullLock,
                OperationCapability::RuntimeRead,
            ],
            availability: OperationAvailability::LocalWindowOnly,
            idempotency: OperationIdempotency::ReadOnly,
            audit: OperationAuditRequirement::NotApplicable,
            request_schema: SchemaIdentity {
                name: "syndocal.query.snapshot.v1.request".to_string(),
                version: 1,
            },
            response_schema: SchemaIdentity {
                name: "syndocal.query.snapshot.v1.response".to_string(),
                version: 1,
            },
        }
    }

    #[test]
    fn descriptor_json_round_trip_is_strict_and_validated() {
        let descriptor = local_read_descriptor();
        descriptor.validate().unwrap();
        let json = serde_json::to_string(&descriptor).unwrap();
        let restored: OperationDescriptor = serde_json::from_str(&json).unwrap();
        assert_eq!(restored, descriptor);
        let unknown = json.trim_end_matches('}').to_string() + ",\"unexpected\":true}";
        assert!(serde_json::from_str::<OperationDescriptor>(&unknown).is_err());
        let unsafe_json = json.replace("\"r0\"", "\"r5\"");
        assert!(serde_json::from_str::<OperationDescriptor>(&unsafe_json).is_err());
    }

    #[test]
    fn conservative_availability_rejects_mutation_or_lower_risk_unavailable() {
        let mut descriptor = local_read_descriptor();
        descriptor.idempotency = OperationIdempotency::Mutating;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeLocalAvailability)
        );
        descriptor.availability = OperationAvailability::Unavailable;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnavailableOperationMustBeR5)
        );
    }

    #[test]
    fn engine_inventory_is_unavailable_and_cannot_be_forged_as_read_or_discovery() {
        let mut descriptor = local_read_descriptor();
        descriptor.source_family = OperationSourceFamily::EngineCommand;
        descriptor.operation_id = "syndocal.inventory.engine.set_blackout.v1".to_string();
        descriptor.source_id = "set_blackout".to_string();
        descriptor.class = OperationClass::Mutation;
        descriptor.risk = OperationRisk::R5;
        descriptor.capabilities = vec![OperationCapability::InternalInventory];
        descriptor.availability = OperationAvailability::Unavailable;
        descriptor.idempotency = OperationIdempotency::Mutating;
        descriptor.audit = OperationAuditRequirement::RequiredBeforeExternalExecution;
        descriptor.request_schema = SchemaIdentity {
            name: "syndocal.inventory.engine.set_blackout.v1.request".to_string(),
            version: CONTROL_PLANE_SCHEMA_VERSION,
        };
        descriptor.response_schema = SchemaIdentity {
            name: "syndocal.inventory.engine.set_blackout.v1.response".to_string(),
            version: CONTROL_PLANE_SCHEMA_VERSION,
        };
        descriptor.validate().unwrap();

        descriptor.capabilities = vec![OperationCapability::ReadOnly];
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeUnavailableOperation)
        );
        descriptor.capabilities = vec![OperationCapability::RegistryDiscovery];
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeUnavailableOperation)
        );
        descriptor.capabilities = vec![
            OperationCapability::ReadOnly,
            OperationCapability::LocalWindowBound,
            OperationCapability::AllowedDuringFullLock,
            OperationCapability::RuntimeRead,
        ];
        descriptor.availability = OperationAvailability::LocalWindowOnly;
        descriptor.risk = OperationRisk::R0;
        descriptor.idempotency = OperationIdempotency::ReadOnly;
        descriptor.audit = OperationAuditRequirement::NotApplicable;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeLocalAvailability)
        );
        descriptor.source_family = OperationSourceFamily::RemoteWireOperation;
        descriptor.source_id = "setBlackout".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeLocalAvailability)
        );
    }

    #[test]
    fn internal_inventory_families_cannot_be_forged_as_read_local_or_full_lock() {
        let families = [
            OperationSourceFamily::MidiControlMessage,
            OperationSourceFamily::MidiControlAction,
            OperationSourceFamily::MidiClockEvent,
            OperationSourceFamily::MidiControlEvent,
            OperationSourceFamily::OscControlAction,
            OperationSourceFamily::OscInputEvent,
            OperationSourceFamily::DmxInputProtocol,
            OperationSourceFamily::DmxInputEvent,
            OperationSourceFamily::FrontendInvoke,
        ];
        for (index, source_family) in families.into_iter().enumerate() {
            let operation_id = format!("syndocal.inventory.io.family_{index}.v1");
            let mut descriptor = OperationDescriptor {
                schema: SchemaIdentity::descriptor(),
                operation_id: operation_id.clone(),
                source_family,
                source_id: "sample_variant".to_string(),
                class: OperationClass::Mutation,
                risk: OperationRisk::R5,
                capabilities: vec![OperationCapability::InternalInventory],
                availability: OperationAvailability::Unavailable,
                idempotency: OperationIdempotency::Mutating,
                audit: OperationAuditRequirement::RequiredBeforeExternalExecution,
                request_schema: SchemaIdentity {
                    name: format!("{operation_id}.request"),
                    version: CONTROL_PLANE_SCHEMA_VERSION,
                },
                response_schema: SchemaIdentity {
                    name: format!("{operation_id}.response"),
                    version: CONTROL_PLANE_SCHEMA_VERSION,
                },
            };
            descriptor.validate().unwrap();

            descriptor.capabilities = vec![OperationCapability::ReadOnly];
            assert_eq!(
                descriptor.validate(),
                Err(OperationDescriptorValidationError::UnsafeUnavailableOperation),
                "{source_family:?}"
            );
            descriptor.capabilities = vec![
                OperationCapability::InternalInventory,
                OperationCapability::AllowedDuringFullLock,
            ];
            assert_eq!(
                descriptor.validate(),
                Err(OperationDescriptorValidationError::UnsafeUnavailableOperation),
                "{source_family:?}"
            );
            descriptor.capabilities = vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                OperationCapability::AllowedDuringFullLock,
                OperationCapability::RuntimeRead,
            ];
            descriptor.availability = OperationAvailability::LocalWindowOnly;
            descriptor.risk = OperationRisk::R0;
            descriptor.idempotency = OperationIdempotency::ReadOnly;
            descriptor.audit = OperationAuditRequirement::NotApplicable;
            assert_eq!(
                descriptor.validate(),
                Err(OperationDescriptorValidationError::UnsafeLocalAvailability),
                "{source_family:?}"
            );
        }
    }

    #[test]
    fn schema_versions_are_exactly_compatible() {
        let mut descriptor = local_read_descriptor();
        descriptor.schema.version = CONTROL_PLANE_SCHEMA_VERSION + 1;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsupportedSchemaVersion)
        );
        descriptor.schema = SchemaIdentity::descriptor();
        descriptor.request_schema.version = 0;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsupportedOperationSchemaVersion)
        );
    }

    #[test]
    fn local_window_only_requires_exactly_one_matching_domain_and_full_lock_allowance() {
        let mut descriptor = local_read_descriptor();
        descriptor
            .capabilities
            .retain(|capability| *capability != OperationCapability::AllowedDuringFullLock);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::MissingFullLockAllowance)
        );
        descriptor = local_read_descriptor();
        descriptor
            .capabilities
            .retain(|capability| *capability != OperationCapability::RuntimeRead);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::MissingReviewedDomainCapability)
        );
        descriptor = local_read_descriptor();
        descriptor
            .capabilities
            .push(OperationCapability::DeviceHealthRead);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::MultipleReviewedDomainCapabilities)
        );
        descriptor = local_read_descriptor();
        descriptor
            .capabilities
            .push(OperationCapability::RuntimeRead);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::DuplicateCapability)
        );
        descriptor = local_read_descriptor();
        descriptor
            .capabilities
            .retain(|capability| *capability != OperationCapability::RuntimeRead);
        descriptor
            .capabilities
            .push(OperationCapability::RegistryDiscovery);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::RegistryDiscoveryOutsideDiscovery)
        );
    }

    #[test]
    fn operation_schema_identity_is_exact_and_nonempty() {
        let mut descriptor = local_read_descriptor();
        descriptor.request_schema.name.clear();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidOperationSchemaName)
        );
        descriptor = local_read_descriptor();
        descriptor.response_schema.name = "syndocal.tauri.other.response".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidOperationSchemaName)
        );
        descriptor = local_read_descriptor();
        descriptor.response_schema.version = CONTROL_PLANE_SCHEMA_VERSION + 1;
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsupportedOperationSchemaVersion)
        );
    }

    #[test]
    fn semantic_operation_id_and_raw_source_id_are_independently_strict() {
        let mut descriptor = local_read_descriptor();
        descriptor.operation_id = "get_snapshot".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidOperationId)
        );
        descriptor = local_read_descriptor();
        descriptor.operation_id = "syndocal.query.snapshot.v01".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidOperationId)
        );
        descriptor = local_read_descriptor();
        descriptor.source_id = "get.snapshot".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidSourceId)
        );

        descriptor = local_read_descriptor();
        descriptor.source_family = OperationSourceFamily::RemoteWireOperation;
        descriptor.source_id = "get_snapshot".to_string();
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::InvalidSourceId)
        );
        for invalid in [
            "GetSnapshot".to_string(),
            "get-snapshot".to_string(),
            "get.snapshot".to_string(),
            "g".repeat(65),
        ] {
            descriptor.source_id = invalid;
            assert_eq!(
                descriptor.validate(),
                Err(OperationDescriptorValidationError::InvalidSourceId)
            );
        }
    }

    #[test]
    fn unavailable_descriptors_are_completely_fail_closed_on_validate_and_deserialize() {
        let mut descriptor = local_read_descriptor();
        descriptor.risk = OperationRisk::R5;
        descriptor.availability = OperationAvailability::Unavailable;
        descriptor.idempotency = OperationIdempotency::Mutating;
        descriptor.audit = OperationAuditRequirement::RequiredBeforeExternalExecution;
        descriptor.capabilities = vec![OperationCapability::LocalWindowBound];
        assert!(descriptor.validate().is_ok());
        descriptor.capabilities.push(OperationCapability::ReadOnly);
        assert_eq!(
            descriptor.validate(),
            Err(OperationDescriptorValidationError::UnsafeUnavailableOperation)
        );
        let json = serde_json::to_string(&descriptor).unwrap();
        assert!(serde_json::from_str::<OperationDescriptor>(&json).is_err());
    }
}
