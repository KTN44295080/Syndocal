//! Canonical, fail-closed control-plane registry version 2.
//!
//! Version 1 inventories implementation sources directly.  This module keeps
//! that inventory intact while separating a small, reviewed canonical
//! operation set from the much larger source inventory.  A source record has
//! no risk or execution authority of its own: only a canonical operation with
//! a validated local adapter policy may expose a local, window-bound read.

use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
};

use serde::{de::Error as _, ser::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::control_plane::{
    OperationAuditRequirement, OperationCapability, OperationClass, OperationIdempotency,
    OperationRisk, OperationSourceFamily as LegacyOperationSourceFamily, SchemaIdentity,
};

/// Schema version for this canonical/source registry contract.
pub const CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION: u16 = 2;
pub const CANONICAL_OPERATION_DESCRIPTOR_SCHEMA_NAME: &str =
    "syndocal.control-plane.canonical-operation-descriptor";
pub const SOURCE_INVENTORY_DESCRIPTOR_SCHEMA_NAME: &str =
    "syndocal.control-plane.source-inventory-descriptor";
pub const CANONICAL_OPERATION_REGISTRY_SCHEMA_NAME: &str =
    "syndocal.control-plane.canonical-operation-registry";

/// Hard bounds keep a malformed local response from becoming an unbounded
/// discovery payload.  The source bound accommodates the exact current
/// inventories while requiring a reviewed increase before a broad expansion.
pub const MAX_CANONICAL_OPERATIONS: usize = 128;
pub const MAX_SOURCE_INVENTORY_DESCRIPTORS: usize = 2_048;
pub const MAX_CANONICAL_REGISTRY_WIRE_BYTES: usize = 4 * 1024 * 1024;
pub const MAX_OPERATION_ID_BYTES: usize = 512;
pub const MAX_SOURCE_ID_BYTES: usize = 512;
pub const MAX_BINDING_ID_BYTES: usize = 576;
pub const MAX_SCHEMA_NAME_BYTES: usize = 768;
pub const MAX_UNCLASSIFIED_REASON_BYTES: usize = 1_024;

/// Version-2-only source namespace. This mirrors legacy v1 families through
/// an explicit conversion so that v2 can inventory additive source families
/// without changing the v1 wire contract or its exact registry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CanonicalSourceFamily {
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
    KeyboardApp,
    KeyboardProjectFile,
}

impl From<LegacyOperationSourceFamily> for CanonicalSourceFamily {
    fn from(family: LegacyOperationSourceFamily) -> Self {
        match family {
            LegacyOperationSourceFamily::TauriCommand => Self::TauriCommand,
            LegacyOperationSourceFamily::EngineCommand => Self::EngineCommand,
            LegacyOperationSourceFamily::RemoteInputEvent => Self::RemoteInputEvent,
            LegacyOperationSourceFamily::RemoteClientRequest => Self::RemoteClientRequest,
            LegacyOperationSourceFamily::RemoteWireOperation => Self::RemoteWireOperation,
            LegacyOperationSourceFamily::MidiControlMessage => Self::MidiControlMessage,
            LegacyOperationSourceFamily::MidiControlAction => Self::MidiControlAction,
            LegacyOperationSourceFamily::MidiClockEvent => Self::MidiClockEvent,
            LegacyOperationSourceFamily::MidiControlEvent => Self::MidiControlEvent,
            LegacyOperationSourceFamily::OscControlAction => Self::OscControlAction,
            LegacyOperationSourceFamily::OscInputEvent => Self::OscInputEvent,
            LegacyOperationSourceFamily::DmxInputProtocol => Self::DmxInputProtocol,
            LegacyOperationSourceFamily::DmxInputEvent => Self::DmxInputEvent,
            LegacyOperationSourceFamily::FrontendInvoke => Self::FrontendInvoke,
        }
    }
}

/// Stable, family-qualified identity for one raw inventory source.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct SourceKey {
    pub family: CanonicalSourceFamily,
    pub source_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceKeyWire {
    family: CanonicalSourceFamily,
    source_id: String,
}

impl SourceKey {
    pub fn new(family: CanonicalSourceFamily, source_id: impl Into<String>) -> Self {
        Self {
            family,
            source_id: source_id.into(),
        }
    }

    /// The binding id is derived from the family-qualified source key.  It is
    /// intentionally not an independently assignable execution identity.
    pub fn binding_id(&self) -> String {
        format!("{}:{}", source_family_name(self.family), self.source_id)
    }

    pub fn validate(&self) -> Result<(), CanonicalRegistryValidationError> {
        if self.source_id.len() > MAX_SOURCE_ID_BYTES
            || !is_valid_source_id(self.family, &self.source_id)
        {
            return Err(CanonicalRegistryValidationError::InvalidSourceKey(
                self.clone(),
            ));
        }
        Ok(())
    }

    fn wire_byte_bound(&self) -> usize {
        96 + json_string_byte_bound(&self.source_id)
    }
}

impl Serialize for SourceKey {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(S::Error::custom)?;
        SourceKeyWire {
            family: self.family,
            source_id: self.source_id.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceKey {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SourceKeyWire::deserialize(deserializer)?;
        let key = Self {
            family: wire.family,
            source_id: wire.source_id,
        };
        key.validate().map_err(D::Error::custom)?;
        Ok(key)
    }
}

/// The structural role assigned to a raw source family.  It is not an
/// execution adapter and is validated against `SourceKey::family`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    LocalWindowCommand,
    EngineCommand,
    RemoteIngress,
    MidiIngress,
    OscIngress,
    DmxIngress,
    FrontendInvocation,
    KeyboardAppShortcut,
    KeyboardProjectFileShortcut,
}

impl SourceRole {
    /// Derive the only valid structural role for a source family.
    pub fn for_family(family: CanonicalSourceFamily) -> Self {
        expected_source_role(family)
    }
}

/// A supported adapter kind.  External variants are representable only so a
/// forged wire value can be rejected explicitly; no valid v2 policy supports
/// them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterKind {
    LocalTauriWindow,
    ExternalMcp,
    ExternalHttp,
    RemoteControl,
}

/// The complete exposure policy for a canonical operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AdapterPolicy {
    /// The only currently supported adapter: an injected local Tauri window
    /// for an R0 read-only operation.
    LocalWindowReadOnly,
    /// A typed, local-window-only authored mutation. It is deliberately
    /// separate from the read policy: it has no Full-Lock allowance and must
    /// retain an exact terminal receipt.
    LocalWindowAuthoritativeMutation,
    /// No adapter is exposed.  This is the only policy available before a
    /// separately reviewed adapter is introduced.
    FailClosed,
}

/// Receipt retention is fail-closed by default. The one reviewed local
/// authoritative mutation uses an exact terminal receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptPolicy {
    FailClosed,
    ExactTerminalReceipt,
}

/// Rate handling deliberately has no enabled implementation in v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RatePolicy {
    FailClosed,
}

/// Payload adaptation deliberately has no enabled implementation in v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PayloadPolicy {
    FailClosed,
}

/// Consent handling deliberately has no enabled implementation in v2.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsentPolicy {
    FailClosed,
}

/// The only reviewed schema projection at this stage is exact identity.
/// Transforming a source request or response is intentionally unsupported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TypedSchemaProjection {
    Exact,
}

/// How a raw source participates in the registry.  Only `Operation` can
/// produce a derived adapter, and registry validation limits that disposition
/// to a reviewed local Tauri source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SourceDisposition {
    Operation {
        canonical_operation_id: String,
        projection: TypedSchemaProjection,
    },
    AliasOfSource {
        target: SourceKey,
    },
    DispatchesToFamily {
        target_family: CanonicalSourceFamily,
    },
    InternalStepOf {
        target: SourceKey,
    },
    PresentationOnly,
    Unclassified {
        reason: String,
    },
}

impl SourceDisposition {
    fn kind_name(&self) -> &'static str {
        match self {
            Self::Operation { .. } => "operation",
            Self::AliasOfSource { .. } => "alias_of_source",
            Self::DispatchesToFamily { .. } => "dispatches_to_family",
            Self::InternalStepOf { .. } => "internal_step_of",
            Self::PresentationOnly => "presentation_only",
            Self::Unclassified { .. } => "unclassified",
        }
    }

    fn wire_byte_bound(&self) -> usize {
        match self {
            Self::Operation {
                canonical_operation_id,
                ..
            } => 128 + json_string_byte_bound(canonical_operation_id),
            Self::AliasOfSource { target } | Self::InternalStepOf { target } => {
                96 + target.wire_byte_bound()
            }
            Self::DispatchesToFamily { .. } => 128,
            Self::PresentationOnly => 64,
            Self::Unclassified { reason } => 96 + json_string_byte_bound(reason),
        }
    }
}

/// A deterministic adapter binding derived from a direct source disposition.
/// Callers must not hand-author this list: registry validation recomputes and
/// compares it exactly.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct DerivedAdapterBinding {
    pub adapter: AdapterKind,
    pub source_key: SourceKey,
    pub binding_id: String,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DerivedAdapterBindingWire {
    adapter: AdapterKind,
    source_key: SourceKey,
    binding_id: String,
}

impl DerivedAdapterBinding {
    fn validate_shape(&self) -> Result<(), CanonicalRegistryValidationError> {
        self.source_key.validate()?;
        if self.binding_id.len() > MAX_BINDING_ID_BYTES
            || self.binding_id != self.source_key.binding_id()
        {
            return Err(CanonicalRegistryValidationError::InvalidBindingId(
                self.binding_id.clone(),
            ));
        }
        if self.adapter != AdapterKind::LocalTauriWindow {
            return Err(
                CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(
                    self.binding_id.clone(),
                ),
            );
        }
        if self.source_key.family != CanonicalSourceFamily::TauriCommand {
            return Err(CanonicalRegistryValidationError::FamilyAdapterMismatch(
                self.source_key.clone(),
            ));
        }
        Ok(())
    }

    fn wire_byte_bound(&self) -> usize {
        192 + self.source_key.wire_byte_bound() + json_string_byte_bound(&self.binding_id)
    }
}

impl Serialize for DerivedAdapterBinding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate_shape().map_err(S::Error::custom)?;
        DerivedAdapterBindingWire {
            adapter: self.adapter,
            source_key: self.source_key.clone(),
            binding_id: self.binding_id.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for DerivedAdapterBinding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = DerivedAdapterBindingWire::deserialize(deserializer)?;
        let binding = Self {
            adapter: wire.adapter,
            source_key: wire.source_key,
            binding_id: wire.binding_id,
        };
        binding.validate_shape().map_err(D::Error::custom)?;
        Ok(binding)
    }
}

/// Reviewed, source-independent semantic operation metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalOperationDescriptor {
    pub schema: SchemaIdentity,
    pub operation_id: String,
    pub class: OperationClass,
    pub risk: OperationRisk,
    pub capabilities: Vec<OperationCapability>,
    pub request_schema: SchemaIdentity,
    pub response_schema: SchemaIdentity,
    pub idempotency: OperationIdempotency,
    pub audit: OperationAuditRequirement,
    pub adapter_policy: AdapterPolicy,
    pub receipt_policy: ReceiptPolicy,
    pub rate_policy: RatePolicy,
    pub payload_policy: PayloadPolicy,
    pub consent_policy: ConsentPolicy,
    pub derived_adapters: Vec<DerivedAdapterBinding>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalOperationDescriptorWire {
    schema: SchemaIdentity,
    operation_id: String,
    class: OperationClass,
    risk: OperationRisk,
    capabilities: Vec<OperationCapability>,
    request_schema: SchemaIdentity,
    response_schema: SchemaIdentity,
    idempotency: OperationIdempotency,
    audit: OperationAuditRequirement,
    adapter_policy: AdapterPolicy,
    receipt_policy: ReceiptPolicy,
    rate_policy: RatePolicy,
    payload_policy: PayloadPolicy,
    consent_policy: ConsentPolicy,
    derived_adapters: Vec<DerivedAdapterBinding>,
}

impl CanonicalOperationDescriptor {
    pub fn schema_identity() -> SchemaIdentity {
        SchemaIdentity {
            name: CANONICAL_OPERATION_DESCRIPTOR_SCHEMA_NAME.to_string(),
            version: CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
        }
    }

    /// Validate this descriptor without cross-source reconciliation.  The
    /// enclosing registry additionally verifies the exact derived adapters.
    fn validate_metadata(&self) -> Result<(), CanonicalRegistryValidationError> {
        validate_exact_schema(
            &self.schema,
            CANONICAL_OPERATION_DESCRIPTOR_SCHEMA_NAME,
            CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
            CanonicalRegistryValidationError::InvalidCanonicalDescriptorSchema,
        )?;
        validate_operation_id(&self.operation_id)?;
        validate_operation_schema_identity(&self.request_schema, &self.operation_id, "request")?;
        validate_operation_schema_identity(&self.response_schema, &self.operation_id, "response")?;
        if self.capabilities.is_empty() {
            return Err(CanonicalRegistryValidationError::EmptyCapabilities);
        }
        let unique_capabilities = self.capabilities.iter().copied().collect::<BTreeSet<_>>();
        if unique_capabilities.len() != self.capabilities.len() {
            return Err(CanonicalRegistryValidationError::DuplicateCapability);
        }
        match self.adapter_policy {
            AdapterPolicy::LocalWindowReadOnly => {
                if self.class == OperationClass::Mutation
                    || self.risk != OperationRisk::R0
                    || self.idempotency != OperationIdempotency::ReadOnly
                    || self.audit != OperationAuditRequirement::NotApplicable
                    || self.capabilities
                        != vec![
                            OperationCapability::ReadOnly,
                            OperationCapability::LocalWindowBound,
                            OperationCapability::AllowedDuringFullLock,
                            expected_domain_capability(self.class),
                        ]
                {
                    return Err(
                        CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(
                            self.operation_id.clone(),
                        ),
                    );
                }
                for adapter in &self.derived_adapters {
                    adapter.validate_shape()?;
                    if adapter.adapter != AdapterKind::LocalTauriWindow {
                        return Err(
                            CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(
                                self.operation_id.clone(),
                            ),
                        );
                    }
                }
            }
            AdapterPolicy::LocalWindowAuthoritativeMutation => {
                if self.class != OperationClass::Mutation
                    || self.risk != OperationRisk::R0
                    || self.idempotency != OperationIdempotency::Mutating
                    || self.audit != OperationAuditRequirement::NotApplicable
                    || self.capabilities
                        != vec![
                            OperationCapability::LocalWindowBound,
                            OperationCapability::AuthoritativeProjectMutation,
                        ]
                    || self.receipt_policy != ReceiptPolicy::ExactTerminalReceipt
                    || self.rate_policy != RatePolicy::FailClosed
                    || self.payload_policy != PayloadPolicy::FailClosed
                    || self.consent_policy != ConsentPolicy::FailClosed
                {
                    return Err(
                        CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(
                            self.operation_id.clone(),
                        ),
                    );
                }
                for adapter in &self.derived_adapters {
                    adapter.validate_shape()?;
                    if adapter.adapter != AdapterKind::LocalTauriWindow {
                        return Err(
                            CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(
                                self.operation_id.clone(),
                            ),
                        );
                    }
                }
            }
            AdapterPolicy::FailClosed => {
                if !self.derived_adapters.is_empty() {
                    return Err(
                        CanonicalRegistryValidationError::ExposedFailClosedOperation(
                            self.operation_id.clone(),
                        ),
                    );
                }
            }
        }
        Ok(())
    }

    pub fn validate(&self) -> Result<(), CanonicalRegistryValidationError> {
        self.validate_metadata()?;
        if matches!(
            self.adapter_policy,
            AdapterPolicy::LocalWindowReadOnly | AdapterPolicy::LocalWindowAuthoritativeMutation
        ) {
            if self.derived_adapters.is_empty() {
                return Err(CanonicalRegistryValidationError::MissingDerivedAdapter(
                    self.operation_id.clone(),
                ));
            }
            if !is_strictly_sorted(&self.derived_adapters) {
                return Err(CanonicalRegistryValidationError::UnsortedDerivedAdapters(
                    self.operation_id.clone(),
                ));
            }
            let unique = self.derived_adapters.iter().collect::<BTreeSet<_>>();
            if unique.len() != self.derived_adapters.len() {
                return Err(CanonicalRegistryValidationError::DuplicateDerivedAdapter(
                    self.operation_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn wire_byte_bound(&self) -> usize {
        1_024
            + schema_wire_byte_bound(&self.schema)
            + json_string_byte_bound(&self.operation_id)
            + self.capabilities.len() * 64
            + schema_wire_byte_bound(&self.request_schema)
            + schema_wire_byte_bound(&self.response_schema)
            + self
                .derived_adapters
                .iter()
                .map(DerivedAdapterBinding::wire_byte_bound)
                .sum::<usize>()
    }
}

impl Serialize for CanonicalOperationDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(S::Error::custom)?;
        CanonicalOperationDescriptorWire {
            schema: self.schema.clone(),
            operation_id: self.operation_id.clone(),
            class: self.class,
            risk: self.risk,
            capabilities: self.capabilities.clone(),
            request_schema: self.request_schema.clone(),
            response_schema: self.response_schema.clone(),
            idempotency: self.idempotency,
            audit: self.audit,
            adapter_policy: self.adapter_policy,
            receipt_policy: self.receipt_policy,
            rate_policy: self.rate_policy,
            payload_policy: self.payload_policy,
            consent_policy: self.consent_policy,
            derived_adapters: self.derived_adapters.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CanonicalOperationDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CanonicalOperationDescriptorWire::deserialize(deserializer)?;
        let descriptor = Self {
            schema: wire.schema,
            operation_id: wire.operation_id,
            class: wire.class,
            risk: wire.risk,
            capabilities: wire.capabilities,
            request_schema: wire.request_schema,
            response_schema: wire.response_schema,
            idempotency: wire.idempotency,
            audit: wire.audit,
            adapter_policy: wire.adapter_policy,
            receipt_policy: wire.receipt_policy,
            rate_policy: wire.rate_policy,
            payload_policy: wire.payload_policy,
            consent_policy: wire.consent_policy,
            derived_adapters: wire.derived_adapters,
        };
        descriptor.validate().map_err(D::Error::custom)?;
        Ok(descriptor)
    }
}

/// Raw source metadata.  Deliberately absent are risk, availability, or any
/// external-adapter field: source inventory is not canonical authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceInventoryDescriptor {
    pub schema: SchemaIdentity,
    pub source_key: SourceKey,
    pub binding_id: String,
    pub role: SourceRole,
    pub raw_request_schema: SchemaIdentity,
    pub raw_response_schema: SchemaIdentity,
    pub disposition: SourceDisposition,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SourceInventoryDescriptorWire {
    schema: SchemaIdentity,
    source_key: SourceKey,
    binding_id: String,
    role: SourceRole,
    raw_request_schema: SchemaIdentity,
    raw_response_schema: SchemaIdentity,
    disposition: SourceDisposition,
}

impl SourceInventoryDescriptor {
    pub fn schema_identity() -> SchemaIdentity {
        SchemaIdentity {
            name: SOURCE_INVENTORY_DESCRIPTOR_SCHEMA_NAME.to_string(),
            version: CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
        }
    }

    pub fn validate(&self) -> Result<(), CanonicalRegistryValidationError> {
        validate_exact_schema(
            &self.schema,
            SOURCE_INVENTORY_DESCRIPTOR_SCHEMA_NAME,
            CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
            CanonicalRegistryValidationError::InvalidSourceDescriptorSchema,
        )?;
        self.source_key.validate()?;
        if self.binding_id.len() > MAX_BINDING_ID_BYTES
            || self.binding_id != self.source_key.binding_id()
        {
            return Err(CanonicalRegistryValidationError::InvalidBindingId(
                self.binding_id.clone(),
            ));
        }
        if self.role != expected_source_role(self.source_key.family) {
            return Err(CanonicalRegistryValidationError::InvalidSourceRole(
                self.source_key.clone(),
            ));
        }
        validate_raw_schema_identity(&self.raw_request_schema, "request")?;
        validate_raw_schema_identity(&self.raw_response_schema, "response")?;
        self.validate_disposition_matrix()
    }

    fn validate_disposition_matrix(&self) -> Result<(), CanonicalRegistryValidationError> {
        match &self.disposition {
            SourceDisposition::Operation {
                canonical_operation_id,
                projection,
            } => {
                validate_operation_id(canonical_operation_id)?;
                if *projection != TypedSchemaProjection::Exact
                    || self.source_key.family != CanonicalSourceFamily::TauriCommand
                    || self.role != SourceRole::LocalWindowCommand
                {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::AliasOfSource { target } => {
                target.validate()?;
                let allowed = match self.source_key.family {
                    CanonicalSourceFamily::FrontendInvoke => {
                        target.family == CanonicalSourceFamily::TauriCommand
                            && target.source_id == self.source_key.source_id
                    }
                    CanonicalSourceFamily::TauriCommand => {
                        target.family == CanonicalSourceFamily::TauriCommand
                    }
                    _ => false,
                };
                if !allowed {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::DispatchesToFamily { .. } => {
                if !matches!(
                    self.role,
                    SourceRole::RemoteIngress
                        | SourceRole::MidiIngress
                        | SourceRole::OscIngress
                        | SourceRole::DmxIngress
                ) {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::InternalStepOf { target } => {
                target.validate()?;
                if self.role != SourceRole::EngineCommand {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::PresentationOnly => {
                if self.role != SourceRole::FrontendInvocation {
                    return Err(
                        CanonicalRegistryValidationError::InvalidDispositionForFamily(
                            self.source_key.clone(),
                            self.disposition.kind_name(),
                        ),
                    );
                }
            }
            SourceDisposition::Unclassified { reason } => {
                if reason.is_empty()
                    || reason.len() > MAX_UNCLASSIFIED_REASON_BYTES
                    || !reason
                        .bytes()
                        .all(|byte| byte.is_ascii_graphic() || byte == b' ')
                {
                    return Err(CanonicalRegistryValidationError::InvalidUnclassifiedReason(
                        self.source_key.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    fn wire_byte_bound(&self) -> usize {
        1_024
            + schema_wire_byte_bound(&self.schema)
            + self.source_key.wire_byte_bound()
            + json_string_byte_bound(&self.binding_id)
            + schema_wire_byte_bound(&self.raw_request_schema)
            + schema_wire_byte_bound(&self.raw_response_schema)
            + self.disposition.wire_byte_bound()
    }
}

impl Serialize for SourceInventoryDescriptor {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(S::Error::custom)?;
        SourceInventoryDescriptorWire {
            schema: self.schema.clone(),
            source_key: self.source_key.clone(),
            binding_id: self.binding_id.clone(),
            role: self.role,
            raw_request_schema: self.raw_request_schema.clone(),
            raw_response_schema: self.raw_response_schema.clone(),
            disposition: self.disposition.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SourceInventoryDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SourceInventoryDescriptorWire::deserialize(deserializer)?;
        let descriptor = Self {
            schema: wire.schema,
            source_key: wire.source_key,
            binding_id: wire.binding_id,
            role: wire.role,
            raw_request_schema: wire.raw_request_schema,
            raw_response_schema: wire.raw_response_schema,
            disposition: wire.disposition,
        };
        descriptor.validate().map_err(D::Error::custom)?;
        Ok(descriptor)
    }
}

/// Complete canonical operation/source inventory.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalControlPlaneRegistry {
    pub schema: SchemaIdentity,
    pub canonical_operations: Vec<CanonicalOperationDescriptor>,
    pub source_inventory: Vec<SourceInventoryDescriptor>,
}

/// Short alias for callers that do not need the control-plane prefix.
pub type CanonicalOperationRegistry = CanonicalControlPlaneRegistry;

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CanonicalControlPlaneRegistryWire {
    schema: SchemaIdentity,
    canonical_operations: Vec<CanonicalOperationDescriptor>,
    source_inventory: Vec<SourceInventoryDescriptor>,
}

impl CanonicalControlPlaneRegistry {
    pub fn schema_identity() -> SchemaIdentity {
        SchemaIdentity {
            name: CANONICAL_OPERATION_REGISTRY_SCHEMA_NAME.to_string(),
            version: CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
        }
    }

    /// Recompute the only valid adapter list from direct source operation
    /// bindings.  This is intended for trusted builders; serialization still
    /// revalidates the resulting registry independently.
    pub fn populate_derived_adapters(&mut self) -> Result<(), CanonicalRegistryValidationError> {
        let derived = self.derive_adapters_from_sources()?;
        for operation in &mut self.canonical_operations {
            operation.derived_adapters = derived
                .get(&operation.operation_id)
                .cloned()
                .unwrap_or_default();
        }
        self.validate()
    }

    /// Resolve a source through aliases/internal steps to its canonical
    /// operation.  Structural and unclassified sources intentionally resolve
    /// to `None`; they never gain an adapter through this query.
    pub fn canonical_operation_for_source(
        &self,
        source_key: &SourceKey,
    ) -> Result<Option<&CanonicalOperationDescriptor>, CanonicalRegistryValidationError> {
        self.validate()?;
        let canonical_ids = self
            .canonical_operations
            .iter()
            .map(|operation| operation.operation_id.clone())
            .collect::<BTreeSet<_>>();
        let sources = self
            .source_inventory
            .iter()
            .map(|source| (source.source_key.clone(), source))
            .collect::<BTreeMap<_, _>>();
        let operation_id = resolve_canonical_operation_id(
            source_key,
            &sources,
            &canonical_ids,
            &mut BTreeSet::new(),
        )?;
        Ok(operation_id.and_then(|operation_id| {
            self.canonical_operations
                .iter()
                .find(|operation| operation.operation_id == operation_id)
        }))
    }

    /// A conservative upper bound for this ASCII-only wire representation.
    /// Validation rejects a registry that exceeds this bound before it can be
    /// serialized or returned by a local discovery command.
    pub fn estimated_wire_bytes(&self) -> usize {
        1_024
            + schema_wire_byte_bound(&self.schema)
            + self
                .canonical_operations
                .iter()
                .map(CanonicalOperationDescriptor::wire_byte_bound)
                .sum::<usize>()
            + self
                .source_inventory
                .iter()
                .map(SourceInventoryDescriptor::wire_byte_bound)
                .sum::<usize>()
    }

    pub fn validate(&self) -> Result<(), CanonicalRegistryValidationError> {
        let derived = self.derive_adapters_from_sources()?;
        for operation in &self.canonical_operations {
            operation.validate()?;
            let expected = derived
                .get(&operation.operation_id)
                .cloned()
                .unwrap_or_default();
            if matches!(
                operation.adapter_policy,
                AdapterPolicy::LocalWindowReadOnly
                    | AdapterPolicy::LocalWindowAuthoritativeMutation
            ) && expected.is_empty()
            {
                return Err(
                    CanonicalRegistryValidationError::CanonicalWithoutDirectSource(
                        operation.operation_id.clone(),
                    ),
                );
            }
            if operation.derived_adapters != expected {
                return Err(CanonicalRegistryValidationError::DerivedAdaptersMismatch(
                    operation.operation_id.clone(),
                ));
            }
        }
        Ok(())
    }

    fn derive_adapters_from_sources(
        &self,
    ) -> Result<BTreeMap<String, Vec<DerivedAdapterBinding>>, CanonicalRegistryValidationError>
    {
        validate_exact_schema(
            &self.schema,
            CANONICAL_OPERATION_REGISTRY_SCHEMA_NAME,
            CONTROL_PLANE_REGISTRY_V2_SCHEMA_VERSION,
            CanonicalRegistryValidationError::InvalidRegistrySchema,
        )?;
        if self.canonical_operations.is_empty() {
            return Err(CanonicalRegistryValidationError::EmptyCanonicalOperations);
        }
        if self.source_inventory.is_empty() {
            return Err(CanonicalRegistryValidationError::EmptySourceInventory);
        }
        if self.canonical_operations.len() > MAX_CANONICAL_OPERATIONS {
            return Err(
                CanonicalRegistryValidationError::TooManyCanonicalOperations(
                    self.canonical_operations.len(),
                ),
            );
        }
        if self.source_inventory.len() > MAX_SOURCE_INVENTORY_DESCRIPTORS {
            return Err(
                CanonicalRegistryValidationError::TooManySourceInventoryDescriptors(
                    self.source_inventory.len(),
                ),
            );
        }
        let estimated_wire_bytes = self.estimated_wire_bytes();
        if estimated_wire_bytes > MAX_CANONICAL_REGISTRY_WIRE_BYTES {
            return Err(CanonicalRegistryValidationError::RegistryByteLimitExceeded(
                estimated_wire_bytes,
            ));
        }

        let mut canonical_by_id = BTreeMap::new();
        for operation in &self.canonical_operations {
            operation.validate_metadata()?;
            if canonical_by_id
                .insert(operation.operation_id.clone(), operation)
                .is_some()
            {
                return Err(
                    CanonicalRegistryValidationError::DuplicateCanonicalOperationId(
                        operation.operation_id.clone(),
                    ),
                );
            }
        }
        if !self
            .canonical_operations
            .windows(2)
            .all(|pair| pair[0].operation_id < pair[1].operation_id)
        {
            return Err(CanonicalRegistryValidationError::UnsortedCanonicalOperations);
        }

        let mut sources = BTreeMap::new();
        let mut binding_ids = BTreeSet::new();
        for source in &self.source_inventory {
            source.validate()?;
            if sources.insert(source.source_key.clone(), source).is_some() {
                return Err(CanonicalRegistryValidationError::DuplicateSourceKey(
                    source.source_key.clone(),
                ));
            }
            if !binding_ids.insert(source.binding_id.as_str()) {
                return Err(CanonicalRegistryValidationError::DuplicateBindingId(
                    source.binding_id.clone(),
                ));
            }
        }
        if !self
            .source_inventory
            .windows(2)
            .all(|pair| pair[0].source_key < pair[1].source_key)
        {
            return Err(CanonicalRegistryValidationError::UnsortedSourceInventory);
        }

        let canonical_ids = canonical_by_id.keys().cloned().collect::<BTreeSet<_>>();
        let mut derived = BTreeMap::<String, Vec<DerivedAdapterBinding>>::new();
        for source in &self.source_inventory {
            match &source.disposition {
                SourceDisposition::Operation {
                    canonical_operation_id,
                    projection,
                } => {
                    let canonical =
                        canonical_by_id.get(canonical_operation_id).ok_or_else(|| {
                            CanonicalRegistryValidationError::MissingCanonicalOperation(
                                canonical_operation_id.clone(),
                            )
                        })?;
                    if source.source_key.family != CanonicalSourceFamily::TauriCommand
                        || source.role != SourceRole::LocalWindowCommand
                        || !matches!(
                            canonical.adapter_policy,
                            AdapterPolicy::LocalWindowReadOnly
                                | AdapterPolicy::LocalWindowAuthoritativeMutation
                        )
                    {
                        return Err(CanonicalRegistryValidationError::FamilyAdapterMismatch(
                            source.source_key.clone(),
                        ));
                    }
                    if *projection != TypedSchemaProjection::Exact
                        || source.raw_request_schema != canonical.request_schema
                        || source.raw_response_schema != canonical.response_schema
                    {
                        return Err(CanonicalRegistryValidationError::SchemaProjectionMismatch(
                            source.source_key.clone(),
                        ));
                    }
                    derived
                        .entry(canonical_operation_id.clone())
                        .or_default()
                        .push(DerivedAdapterBinding {
                            adapter: AdapterKind::LocalTauriWindow,
                            source_key: source.source_key.clone(),
                            binding_id: source.binding_id.clone(),
                        });
                }
                SourceDisposition::AliasOfSource { target } => {
                    let _target_source = sources.get(target).ok_or_else(|| {
                        CanonicalRegistryValidationError::MissingSourceTarget(target.clone())
                    })?;
                }
                SourceDisposition::DispatchesToFamily { target_family } => {
                    if !self
                        .source_inventory
                        .iter()
                        .any(|candidate| candidate.source_key.family == *target_family)
                    {
                        return Err(CanonicalRegistryValidationError::MissingDispatchFamily(
                            *target_family,
                        ));
                    }
                }
                SourceDisposition::InternalStepOf { target } => {
                    if !sources.contains_key(target) {
                        return Err(CanonicalRegistryValidationError::MissingSourceTarget(
                            target.clone(),
                        ));
                    }
                }
                SourceDisposition::PresentationOnly | SourceDisposition::Unclassified { .. } => {}
            }
        }

        for source_key in sources.keys() {
            let _ = resolve_canonical_operation_id(
                source_key,
                &sources,
                &canonical_ids,
                &mut BTreeSet::new(),
            )?;
        }

        for bindings in derived.values_mut() {
            bindings.sort();
            if bindings.windows(2).any(|pair| pair[0] == pair[1]) {
                return Err(CanonicalRegistryValidationError::DuplicateDerivedAdapter(
                    "derived source adapter".to_string(),
                ));
            }
        }
        Ok(derived)
    }
}

impl Serialize for CanonicalControlPlaneRegistry {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(S::Error::custom)?;
        CanonicalControlPlaneRegistryWire {
            schema: self.schema.clone(),
            canonical_operations: self.canonical_operations.clone(),
            source_inventory: self.source_inventory.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for CanonicalControlPlaneRegistry {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CanonicalControlPlaneRegistryWire::deserialize(deserializer)?;
        let registry = Self {
            schema: wire.schema,
            canonical_operations: wire.canonical_operations,
            source_inventory: wire.source_inventory,
        };
        registry.validate().map_err(D::Error::custom)?;
        Ok(registry)
    }
}

/// Errors are intentionally specific enough for a builder to fail closed
/// rather than silently retaining an ambiguous source mapping.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalRegistryValidationError {
    InvalidRegistrySchema,
    InvalidCanonicalDescriptorSchema,
    InvalidSourceDescriptorSchema,
    InvalidOperationId(String),
    InvalidOperationSchema(String),
    InvalidRawSchema(String),
    InvalidSourceKey(SourceKey),
    InvalidBindingId(String),
    InvalidSourceRole(SourceKey),
    InvalidUnclassifiedReason(SourceKey),
    EmptyCapabilities,
    DuplicateCapability,
    UnsafeCanonicalLocalOperation(String),
    UnsupportedAdapterForPolicy(String),
    ExposedFailClosedOperation(String),
    MissingDerivedAdapter(String),
    UnsortedDerivedAdapters(String),
    DuplicateDerivedAdapter(String),
    EmptyCanonicalOperations,
    EmptySourceInventory,
    TooManyCanonicalOperations(usize),
    TooManySourceInventoryDescriptors(usize),
    RegistryByteLimitExceeded(usize),
    DuplicateCanonicalOperationId(String),
    DuplicateSourceKey(SourceKey),
    DuplicateBindingId(String),
    UnsortedCanonicalOperations,
    UnsortedSourceInventory,
    InvalidDispositionForFamily(SourceKey, &'static str),
    MissingCanonicalOperation(String),
    MissingSourceTarget(SourceKey),
    MissingDispatchFamily(CanonicalSourceFamily),
    SourceReferenceCycle(SourceKey),
    AliasDoesNotResolveToOperation(SourceKey),
    InternalStepDoesNotResolveToOperation(SourceKey),
    SchemaProjectionMismatch(SourceKey),
    FamilyAdapterMismatch(SourceKey),
    CanonicalWithoutDirectSource(String),
    DerivedAdaptersMismatch(String),
}

impl fmt::Display for CanonicalRegistryValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRegistrySchema => formatter.write_str("invalid canonical registry schema"),
            Self::InvalidCanonicalDescriptorSchema => {
                formatter.write_str("invalid canonical operation descriptor schema")
            }
            Self::InvalidSourceDescriptorSchema => {
                formatter.write_str("invalid source inventory descriptor schema")
            }
            Self::InvalidOperationId(value) => write!(formatter, "invalid operation id: {value}"),
            Self::InvalidOperationSchema(value) => {
                write!(formatter, "invalid canonical operation schema: {value}")
            }
            Self::InvalidRawSchema(value) => write!(formatter, "invalid raw source schema: {value}"),
            Self::InvalidSourceKey(key) => write!(formatter, "invalid source key: {key:?}"),
            Self::InvalidBindingId(value) => write!(formatter, "invalid source binding id: {value}"),
            Self::InvalidSourceRole(key) => write!(formatter, "invalid source role for {key:?}"),
            Self::InvalidUnclassifiedReason(key) => {
                write!(formatter, "invalid bounded unclassified reason for {key:?}")
            }
            Self::EmptyCapabilities => formatter.write_str("canonical capabilities must not be empty"),
            Self::DuplicateCapability => formatter.write_str("canonical capabilities must be unique"),
            Self::UnsafeCanonicalLocalOperation(id) => {
                write!(formatter, "unsafe local canonical operation: {id}")
            }
            Self::UnsupportedAdapterForPolicy(id) => {
                write!(formatter, "unsupported adapter for canonical policy: {id}")
            }
            Self::ExposedFailClosedOperation(id) => {
                write!(formatter, "fail-closed canonical operation exposes an adapter: {id}")
            }
            Self::MissingDerivedAdapter(id) => {
                write!(formatter, "local canonical operation lacks a derived adapter: {id}")
            }
            Self::UnsortedDerivedAdapters(id) => {
                write!(formatter, "derived adapters are not sorted for: {id}")
            }
            Self::DuplicateDerivedAdapter(id) => {
                write!(formatter, "duplicate derived adapter for: {id}")
            }
            Self::EmptyCanonicalOperations => formatter.write_str("canonical operation inventory is empty"),
            Self::EmptySourceInventory => formatter.write_str("source inventory is empty"),
            Self::TooManyCanonicalOperations(count) => write!(
                formatter,
                "canonical operation inventory exceeds {MAX_CANONICAL_OPERATIONS}: {count}"
            ),
            Self::TooManySourceInventoryDescriptors(count) => write!(
                formatter,
                "source inventory exceeds {MAX_SOURCE_INVENTORY_DESCRIPTORS}: {count}"
            ),
            Self::RegistryByteLimitExceeded(bytes) => write!(
                formatter,
                "canonical registry exceeds {MAX_CANONICAL_REGISTRY_WIRE_BYTES} bounded bytes: {bytes}"
            ),
            Self::DuplicateCanonicalOperationId(id) => {
                write!(formatter, "duplicate canonical operation id: {id}")
            }
            Self::DuplicateSourceKey(key) => write!(formatter, "duplicate source key: {key:?}"),
            Self::DuplicateBindingId(id) => write!(formatter, "duplicate source binding id: {id}"),
            Self::UnsortedCanonicalOperations => {
                formatter.write_str("canonical operation inventory is not strictly sorted")
            }
            Self::UnsortedSourceInventory => {
                formatter.write_str("source inventory is not strictly sorted")
            }
            Self::InvalidDispositionForFamily(key, disposition) => write!(
                formatter,
                "source disposition {disposition} is not allowed for {key:?}"
            ),
            Self::MissingCanonicalOperation(id) => {
                write!(formatter, "source points to missing canonical operation: {id}")
            }
            Self::MissingSourceTarget(key) => write!(formatter, "source target is missing: {key:?}"),
            Self::MissingDispatchFamily(family) => {
                write!(formatter, "dispatch target family has no source: {family:?}")
            }
            Self::SourceReferenceCycle(key) => {
                write!(formatter, "source alias/internal chain cycles at: {key:?}")
            }
            Self::AliasDoesNotResolveToOperation(key) => {
                write!(formatter, "source alias does not terminate at an operation: {key:?}")
            }
            Self::InternalStepDoesNotResolveToOperation(key) => write!(
                formatter,
                "internal source step does not terminate at an operation: {key:?}"
            ),
            Self::SchemaProjectionMismatch(key) => {
                write!(formatter, "source schemas do not match exact projection: {key:?}")
            }
            Self::FamilyAdapterMismatch(key) => {
                write!(formatter, "source family cannot use canonical adapter: {key:?}")
            }
            Self::CanonicalWithoutDirectSource(id) => {
                write!(formatter, "canonical operation lacks a direct source: {id}")
            }
            Self::DerivedAdaptersMismatch(id) => {
                write!(formatter, "derived adapters are not exact for: {id}")
            }
        }
    }
}

impl std::error::Error for CanonicalRegistryValidationError {}

fn resolve_canonical_operation_id(
    source_key: &SourceKey,
    sources: &BTreeMap<SourceKey, &SourceInventoryDescriptor>,
    canonical_ids: &BTreeSet<String>,
    visiting: &mut BTreeSet<SourceKey>,
) -> Result<Option<String>, CanonicalRegistryValidationError> {
    if !visiting.insert(source_key.clone()) {
        return Err(CanonicalRegistryValidationError::SourceReferenceCycle(
            source_key.clone(),
        ));
    }
    let result = (|| {
        let source = sources.get(source_key).ok_or_else(|| {
            CanonicalRegistryValidationError::MissingSourceTarget(source_key.clone())
        })?;
        match &source.disposition {
            SourceDisposition::Operation {
                canonical_operation_id,
                ..
            } => {
                if !canonical_ids.contains(canonical_operation_id) {
                    return Err(CanonicalRegistryValidationError::MissingCanonicalOperation(
                        canonical_operation_id.clone(),
                    ));
                }
                Ok(Some(canonical_operation_id.clone()))
            }
            SourceDisposition::AliasOfSource { target } => {
                resolve_canonical_operation_id(target, sources, canonical_ids, visiting)
            }
            SourceDisposition::InternalStepOf { target } => {
                resolve_canonical_operation_id(target, sources, canonical_ids, visiting)
            }
            SourceDisposition::DispatchesToFamily { .. }
            | SourceDisposition::PresentationOnly
            | SourceDisposition::Unclassified { .. } => Ok(None),
        }
    })();
    visiting.remove(source_key);
    result
}

fn validate_exact_schema(
    schema: &SchemaIdentity,
    expected_name: &'static str,
    expected_version: u16,
    error: CanonicalRegistryValidationError,
) -> Result<(), CanonicalRegistryValidationError> {
    if schema.name != expected_name || schema.version != expected_version {
        return Err(error);
    }
    Ok(())
}

fn validate_operation_id(operation_id: &str) -> Result<(), CanonicalRegistryValidationError> {
    if operation_id.len() > MAX_OPERATION_ID_BYTES {
        return Err(CanonicalRegistryValidationError::InvalidOperationId(
            operation_id.to_string(),
        ));
    }
    let mut segments = operation_id.split('.');
    let Some(first) = segments.next() else {
        return Err(CanonicalRegistryValidationError::InvalidOperationId(
            operation_id.to_string(),
        ));
    };
    let remaining = segments.collect::<Vec<_>>();
    if !is_lower_snake_case(first)
        || remaining.is_empty()
        || !remaining[..remaining.len() - 1]
            .iter()
            .all(|segment| is_lower_snake_case(segment))
    {
        return Err(CanonicalRegistryValidationError::InvalidOperationId(
            operation_id.to_string(),
        ));
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
        return Err(CanonicalRegistryValidationError::InvalidOperationId(
            operation_id.to_string(),
        ));
    }
    Ok(())
}

fn validate_operation_schema_identity(
    schema: &SchemaIdentity,
    operation_id: &str,
    direction: &str,
) -> Result<(), CanonicalRegistryValidationError> {
    if schema.version == 0
        || schema.name.len() > MAX_SCHEMA_NAME_BYTES
        || schema.name != format!("{operation_id}.{direction}")
        || !is_valid_schema_name(&schema.name)
    {
        return Err(CanonicalRegistryValidationError::InvalidOperationSchema(
            schema.name.clone(),
        ));
    }
    Ok(())
}

fn validate_raw_schema_identity(
    schema: &SchemaIdentity,
    direction: &str,
) -> Result<(), CanonicalRegistryValidationError> {
    if schema.version == 0
        || schema.name.len() > MAX_SCHEMA_NAME_BYTES
        || !schema.name.ends_with(&format!(".{direction}"))
        || !is_valid_schema_name(&schema.name)
    {
        return Err(CanonicalRegistryValidationError::InvalidRawSchema(
            schema.name.clone(),
        ));
    }
    Ok(())
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

fn expected_source_role(family: CanonicalSourceFamily) -> SourceRole {
    match family {
        CanonicalSourceFamily::TauriCommand => SourceRole::LocalWindowCommand,
        CanonicalSourceFamily::EngineCommand => SourceRole::EngineCommand,
        CanonicalSourceFamily::RemoteInputEvent
        | CanonicalSourceFamily::RemoteClientRequest
        | CanonicalSourceFamily::RemoteWireOperation => SourceRole::RemoteIngress,
        CanonicalSourceFamily::MidiControlMessage
        | CanonicalSourceFamily::MidiControlAction
        | CanonicalSourceFamily::MidiClockEvent
        | CanonicalSourceFamily::MidiControlEvent => SourceRole::MidiIngress,
        CanonicalSourceFamily::OscControlAction | CanonicalSourceFamily::OscInputEvent => {
            SourceRole::OscIngress
        }
        CanonicalSourceFamily::DmxInputProtocol | CanonicalSourceFamily::DmxInputEvent => {
            SourceRole::DmxIngress
        }
        CanonicalSourceFamily::FrontendInvoke => SourceRole::FrontendInvocation,
        CanonicalSourceFamily::KeyboardApp => SourceRole::KeyboardAppShortcut,
        CanonicalSourceFamily::KeyboardProjectFile => SourceRole::KeyboardProjectFileShortcut,
    }
}

fn source_family_name(family: CanonicalSourceFamily) -> &'static str {
    match family {
        CanonicalSourceFamily::TauriCommand => "tauri_command",
        CanonicalSourceFamily::EngineCommand => "engine_command",
        CanonicalSourceFamily::RemoteInputEvent => "remote_input_event",
        CanonicalSourceFamily::RemoteClientRequest => "remote_client_request",
        CanonicalSourceFamily::RemoteWireOperation => "remote_wire_operation",
        CanonicalSourceFamily::MidiControlMessage => "midi_control_message",
        CanonicalSourceFamily::MidiControlAction => "midi_control_action",
        CanonicalSourceFamily::MidiClockEvent => "midi_clock_event",
        CanonicalSourceFamily::MidiControlEvent => "midi_control_event",
        CanonicalSourceFamily::OscControlAction => "osc_control_action",
        CanonicalSourceFamily::OscInputEvent => "osc_input_event",
        CanonicalSourceFamily::DmxInputProtocol => "dmx_input_protocol",
        CanonicalSourceFamily::DmxInputEvent => "dmx_input_event",
        CanonicalSourceFamily::FrontendInvoke => "frontend_invoke",
        CanonicalSourceFamily::KeyboardApp => "keyboard_app",
        CanonicalSourceFamily::KeyboardProjectFile => "keyboard_project_file",
    }
}

fn is_valid_source_id(family: CanonicalSourceFamily, source_id: &str) -> bool {
    match family {
        CanonicalSourceFamily::RemoteWireOperation => is_bounded_lower_camel_ascii(source_id),
        CanonicalSourceFamily::KeyboardApp | CanonicalSourceFamily::KeyboardProjectFile => {
            is_versioned_lower_snake_case(source_id)
        }
        CanonicalSourceFamily::TauriCommand
        | CanonicalSourceFamily::EngineCommand
        | CanonicalSourceFamily::RemoteInputEvent
        | CanonicalSourceFamily::RemoteClientRequest
        | CanonicalSourceFamily::MidiControlMessage
        | CanonicalSourceFamily::MidiControlAction
        | CanonicalSourceFamily::MidiClockEvent
        | CanonicalSourceFamily::MidiControlEvent
        | CanonicalSourceFamily::OscControlAction
        | CanonicalSourceFamily::OscInputEvent
        | CanonicalSourceFamily::DmxInputProtocol
        | CanonicalSourceFamily::DmxInputEvent
        | CanonicalSourceFamily::FrontendInvoke => is_lower_snake_case(source_id),
    }
}

fn is_bounded_lower_camel_ascii(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase() && bytes.all(|byte| byte.is_ascii_alphanumeric())
}

fn is_lower_snake_case(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

fn is_versioned_lower_snake_case(value: &str) -> bool {
    let Some((prefix, version)) = value.rsplit_once("_v") else {
        return false;
    };
    is_lower_snake_case(prefix)
        && !version.is_empty()
        && !version.starts_with('0')
        && version.bytes().all(|byte| byte.is_ascii_digit())
}

fn is_valid_schema_name(value: &str) -> bool {
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'.' | b'_' | b'-')
        })
}

fn schema_wire_byte_bound(schema: &SchemaIdentity) -> usize {
    96 + json_string_byte_bound(&schema.name)
}

fn json_string_byte_bound(value: &str) -> usize {
    // Every accepted dynamic field is ASCII.  Doubling reserves enough space
    // for a JSON escape while keeping the calculation independent of a JSON
    // implementation in this production crate.
    2 + value.len().saturating_mul(2)
}

fn is_strictly_sorted<T: Ord>(values: &[T]) -> bool {
    values.windows(2).all(|pair| pair[0] < pair[1])
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const OPERATION_ID: &str = "syndocal.query.demo.v1";
    const DIRECT_SOURCE_ID: &str = "get_demo";

    fn operation_schema(direction: &str) -> SchemaIdentity {
        SchemaIdentity {
            name: format!("{OPERATION_ID}.{direction}"),
            version: 1,
        }
    }

    fn canonical_operation() -> CanonicalOperationDescriptor {
        CanonicalOperationDescriptor {
            schema: CanonicalOperationDescriptor::schema_identity(),
            operation_id: OPERATION_ID.to_string(),
            class: OperationClass::Discovery,
            risk: OperationRisk::R0,
            capabilities: vec![
                OperationCapability::ReadOnly,
                OperationCapability::LocalWindowBound,
                OperationCapability::AllowedDuringFullLock,
                OperationCapability::RegistryDiscovery,
            ],
            request_schema: operation_schema("request"),
            response_schema: operation_schema("response"),
            idempotency: OperationIdempotency::ReadOnly,
            audit: OperationAuditRequirement::NotApplicable,
            adapter_policy: AdapterPolicy::LocalWindowReadOnly,
            receipt_policy: ReceiptPolicy::FailClosed,
            rate_policy: RatePolicy::FailClosed,
            payload_policy: PayloadPolicy::FailClosed,
            consent_policy: ConsentPolicy::FailClosed,
            derived_adapters: Vec::new(),
        }
    }

    fn authoritative_mutation_operation() -> CanonicalOperationDescriptor {
        let operation_id = "syndocal.effects.set_enabled.v1";
        CanonicalOperationDescriptor {
            schema: CanonicalOperationDescriptor::schema_identity(),
            operation_id: operation_id.to_string(),
            class: OperationClass::Mutation,
            risk: OperationRisk::R0,
            capabilities: vec![
                OperationCapability::LocalWindowBound,
                OperationCapability::AuthoritativeProjectMutation,
            ],
            request_schema: SchemaIdentity {
                name: format!("{operation_id}.request"),
                version: 1,
            },
            response_schema: SchemaIdentity {
                name: format!("{operation_id}.response"),
                version: 1,
            },
            idempotency: OperationIdempotency::Mutating,
            audit: OperationAuditRequirement::NotApplicable,
            adapter_policy: AdapterPolicy::LocalWindowAuthoritativeMutation,
            receipt_policy: ReceiptPolicy::ExactTerminalReceipt,
            rate_policy: RatePolicy::FailClosed,
            payload_policy: PayloadPolicy::FailClosed,
            consent_policy: ConsentPolicy::FailClosed,
            derived_adapters: Vec::new(),
        }
    }

    fn source(
        family: CanonicalSourceFamily,
        source_id: &str,
        disposition: SourceDisposition,
    ) -> SourceInventoryDescriptor {
        let source_key = SourceKey::new(family, source_id);
        SourceInventoryDescriptor {
            schema: SourceInventoryDescriptor::schema_identity(),
            binding_id: source_key.binding_id(),
            source_key,
            role: expected_source_role(family),
            raw_request_schema: operation_schema("request"),
            raw_response_schema: operation_schema("response"),
            disposition,
        }
    }

    fn direct_source() -> SourceInventoryDescriptor {
        source(
            CanonicalSourceFamily::TauriCommand,
            DIRECT_SOURCE_ID,
            SourceDisposition::Operation {
                canonical_operation_id: OPERATION_ID.to_string(),
                projection: TypedSchemaProjection::Exact,
            },
        )
    }

    fn valid_registry() -> CanonicalControlPlaneRegistry {
        let direct = direct_source();
        let frontend = source(
            CanonicalSourceFamily::FrontendInvoke,
            DIRECT_SOURCE_ID,
            SourceDisposition::AliasOfSource {
                target: direct.source_key.clone(),
            },
        );
        let mut registry = CanonicalControlPlaneRegistry {
            schema: CanonicalControlPlaneRegistry::schema_identity(),
            canonical_operations: vec![canonical_operation()],
            source_inventory: vec![direct, frontend],
        };
        registry
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        registry.populate_derived_adapters().unwrap();
        registry
    }

    #[test]
    fn valid_multiple_sources_resolve_to_one_canonical_operation() {
        let registry = valid_registry();
        registry.validate().unwrap();
        assert_eq!(registry.canonical_operations.len(), 1);
        assert_eq!(registry.source_inventory.len(), 2);
        assert_eq!(
            registry
                .canonical_operation_for_source(&SourceKey::new(
                    CanonicalSourceFamily::FrontendInvoke,
                    DIRECT_SOURCE_ID,
                ))
                .unwrap()
                .map(|operation| operation.operation_id.as_str()),
            Some(OPERATION_ID)
        );
        let encoded = serde_json::to_string(&registry).unwrap();
        let round_trip: CanonicalControlPlaneRegistry = serde_json::from_str(&encoded).unwrap();
        assert_eq!(round_trip, registry);
    }

    #[test]
    fn duplicate_orphan_and_alias_cycle_are_rejected() {
        let mut duplicate = valid_registry();
        duplicate
            .source_inventory
            .push(duplicate.source_inventory[0].clone());
        duplicate
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            duplicate.validate(),
            Err(CanonicalRegistryValidationError::DuplicateSourceKey(_))
        ));

        let mut orphan = valid_registry();
        orphan.source_inventory.push(source(
            CanonicalSourceFamily::TauriCommand,
            "orphan_alias",
            SourceDisposition::AliasOfSource {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, "missing_source"),
            },
        ));
        orphan
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            orphan.validate(),
            Err(CanonicalRegistryValidationError::MissingSourceTarget(_))
        ));

        let mut cycle = valid_registry();
        let first = source(
            CanonicalSourceFamily::TauriCommand,
            "alias_a",
            SourceDisposition::AliasOfSource {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, "alias_b"),
            },
        );
        let second = source(
            CanonicalSourceFamily::TauriCommand,
            "alias_b",
            SourceDisposition::AliasOfSource {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, "alias_a"),
            },
        );
        cycle.source_inventory.extend([first, second]);
        cycle
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            cycle.validate(),
            Err(CanonicalRegistryValidationError::SourceReferenceCycle(_))
        ));
    }

    #[test]
    fn unclassified_and_internal_sources_cannot_be_exposed() {
        let mut unclassified = valid_registry();
        let unclassified_source = source(
            CanonicalSourceFamily::TauriCommand,
            "unclassified_query",
            SourceDisposition::Unclassified {
                reason: "not reviewed".to_string(),
            },
        );
        let forged_adapter = DerivedAdapterBinding {
            adapter: AdapterKind::LocalTauriWindow,
            source_key: unclassified_source.source_key.clone(),
            binding_id: unclassified_source.binding_id.clone(),
        };
        unclassified.source_inventory.push(unclassified_source);
        unclassified
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        unclassified.canonical_operations[0]
            .derived_adapters
            .push(forged_adapter);
        unclassified.canonical_operations[0].derived_adapters.sort();
        assert!(matches!(
            unclassified.validate(),
            Err(CanonicalRegistryValidationError::DerivedAdaptersMismatch(_))
        ));

        let mut internal = valid_registry();
        let internal_source = source(
            CanonicalSourceFamily::EngineCommand,
            "internal_step",
            SourceDisposition::InternalStepOf {
                target: SourceKey::new(CanonicalSourceFamily::TauriCommand, DIRECT_SOURCE_ID),
            },
        );
        internal.source_inventory.push(internal_source.clone());
        internal
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        internal.canonical_operations[0]
            .derived_adapters
            .push(DerivedAdapterBinding {
                adapter: AdapterKind::LocalTauriWindow,
                source_key: internal_source.source_key,
                binding_id: internal_source.binding_id,
            });
        internal.canonical_operations[0].derived_adapters.sort();
        assert!(matches!(
            internal.validate(),
            Err(CanonicalRegistryValidationError::FamilyAdapterMismatch(_))
        ));
    }

    #[test]
    fn v2_keyboard_sources_are_versioned_unclassified_and_cannot_be_exposed() {
        assert_eq!(
            CanonicalSourceFamily::from(LegacyOperationSourceFamily::TauriCommand),
            CanonicalSourceFamily::TauriCommand
        );
        assert_eq!(
            CanonicalSourceFamily::from(LegacyOperationSourceFamily::FrontendInvoke),
            CanonicalSourceFamily::FrontendInvoke
        );
        assert!(
            SourceKey::new(CanonicalSourceFamily::KeyboardApp, "new_project_v1")
                .validate()
                .is_ok()
        );
        assert!(
            SourceKey::new(CanonicalSourceFamily::KeyboardApp, "new_project")
                .validate()
                .is_err()
        );
        for source_id in ["new_project_v0", "new_project_v01", "new_project_v"] {
            assert!(
                serde_json::from_value::<SourceKey>(json!({
                    "family": "keyboard_app",
                    "source_id": source_id,
                }))
                .is_err(),
                "wire input must reject {source_id}"
            );
            assert!(
                serde_json::to_value(SourceKey::new(
                    CanonicalSourceFamily::KeyboardApp,
                    source_id,
                ))
                .is_err(),
                "outbound serialization must reject {source_id}"
            );
        }

        let keyboard_source = source(
            CanonicalSourceFamily::KeyboardApp,
            "new_project_v1",
            SourceDisposition::Unclassified {
                reason: "unreviewed keyboard app shortcut".to_string(),
            },
        );
        let mut registry = valid_registry();
        registry.source_inventory.push(keyboard_source.clone());
        registry
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        registry.validate().unwrap();
        assert!(registry
            .canonical_operation_for_source(&keyboard_source.source_key)
            .unwrap()
            .is_none());

        let mut forged_operation = registry.clone();
        let source = forged_operation
            .source_inventory
            .iter_mut()
            .find(|source| source.source_key == keyboard_source.source_key)
            .unwrap();
        source.disposition = SourceDisposition::Operation {
            canonical_operation_id: OPERATION_ID.to_string(),
            projection: TypedSchemaProjection::Exact,
        };
        assert!(matches!(
            forged_operation.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, _))
        ));

        let mut forged_adapter = registry;
        forged_adapter.canonical_operations[0]
            .derived_adapters
            .push(DerivedAdapterBinding {
                adapter: AdapterKind::LocalTauriWindow,
                source_key: keyboard_source.source_key.clone(),
                binding_id: keyboard_source.binding_id.clone(),
            });
        forged_adapter.canonical_operations[0]
            .derived_adapters
            .sort();
        assert!(matches!(
            forged_adapter.validate(),
            Err(CanonicalRegistryValidationError::FamilyAdapterMismatch(_))
        ));

        let mut forged_presentation = valid_registry();
        let mut presentation_source = keyboard_source;
        presentation_source.disposition = SourceDisposition::PresentationOnly;
        forged_presentation
            .source_inventory
            .push(presentation_source);
        forged_presentation
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            forged_presentation.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, _))
        ));
    }

    #[test]
    fn family_projection_and_supported_adapter_mismatches_are_rejected() {
        let mut family = valid_registry();
        let engine_operation = source(
            CanonicalSourceFamily::EngineCommand,
            "engine_query",
            SourceDisposition::Operation {
                canonical_operation_id: OPERATION_ID.to_string(),
                projection: TypedSchemaProjection::Exact,
            },
        );
        family.source_inventory.push(engine_operation);
        family
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            family.validate(),
            Err(CanonicalRegistryValidationError::InvalidDispositionForFamily(_, _))
        ));

        let mut projection = valid_registry();
        projection.source_inventory[0].raw_response_schema = SchemaIdentity {
            name: "syndocal.query.other.v1.response".to_string(),
            version: 1,
        };
        assert!(matches!(
            projection.validate(),
            Err(CanonicalRegistryValidationError::SchemaProjectionMismatch(
                _
            ))
        ));

        let mut adapter = valid_registry();
        adapter.canonical_operations[0].derived_adapters[0].adapter = AdapterKind::ExternalMcp;
        assert!(matches!(
            adapter.validate(),
            Err(CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(_))
        ));
    }

    #[test]
    fn authoritative_local_mutation_policy_rejects_forged_read_full_lock_and_external_shapes() {
        let mut mutation = authoritative_mutation_operation();
        mutation.derived_adapters.push(DerivedAdapterBinding {
            adapter: AdapterKind::LocalTauriWindow,
            source_key: SourceKey::new(CanonicalSourceFamily::TauriCommand, "set_effect_enabled"),
            binding_id: SourceKey::new(CanonicalSourceFamily::TauriCommand, "set_effect_enabled")
                .binding_id(),
        });
        mutation.validate().unwrap();

        let mut forged_read = mutation.clone();
        forged_read
            .capabilities
            .insert(0, OperationCapability::ReadOnly);
        assert!(matches!(
            forged_read.validate(),
            Err(CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(_))
        ));

        let mut forged_full_lock = mutation.clone();
        forged_full_lock
            .capabilities
            .insert(1, OperationCapability::AllowedDuringFullLock);
        assert!(matches!(
            forged_full_lock.validate(),
            Err(CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(_))
        ));

        let mut forged_read_policy = mutation.clone();
        forged_read_policy.adapter_policy = AdapterPolicy::LocalWindowReadOnly;
        assert!(matches!(
            forged_read_policy.validate(),
            Err(CanonicalRegistryValidationError::UnsafeCanonicalLocalOperation(_))
        ));

        let mut external = mutation;
        external.derived_adapters[0].adapter = AdapterKind::ExternalMcp;
        assert!(matches!(
            external.validate(),
            Err(CanonicalRegistryValidationError::UnsupportedAdapterForPolicy(_))
        ));
    }

    #[test]
    fn source_risk_forgery_and_invalid_outbound_serialization_are_rejected() {
        let registry = valid_registry();
        let mut forged: Value = serde_json::to_value(&registry).unwrap();
        forged["source_inventory"][0]
            .as_object_mut()
            .unwrap()
            .insert("risk".to_string(), json!("r0"));
        assert!(serde_json::from_value::<CanonicalControlPlaneRegistry>(forged).is_err());

        let mut outbound = registry;
        outbound.canonical_operations[0].derived_adapters[0].binding_id = "forged".to_string();
        assert!(serde_json::to_string(&outbound).is_err());
    }

    #[test]
    fn oversized_registry_wire_bound_is_rejected() {
        let mut registry = valid_registry();
        let large_reason = "x".repeat(MAX_UNCLASSIFIED_REASON_BYTES);
        for index in 0..(MAX_SOURCE_INVENTORY_DESCRIPTORS - registry.source_inventory.len()) {
            let source_id = format!("oversized_source_{index}");
            registry.source_inventory.push(source(
                CanonicalSourceFamily::TauriCommand,
                &source_id,
                SourceDisposition::Unclassified {
                    reason: large_reason.clone(),
                },
            ));
        }
        registry
            .source_inventory
            .sort_by(|left, right| left.source_key.cmp(&right.source_key));
        assert!(matches!(
            registry.validate(),
            Err(CanonicalRegistryValidationError::RegistryByteLimitExceeded(
                _
            ))
        ));
    }
}
