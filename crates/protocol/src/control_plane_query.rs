//! Canonical, transport-neutral contracts for bounded control-plane queries.
//!
//! This module contains no adapter, authentication secret, filesystem address,
//! project payload, or execution entry point. It only defines strict wire DTOs
//! and validation helpers used by later authenticated query adapters.

use std::{collections::BTreeSet, fmt};

use serde::{
    de::Error as _, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer,
};

macro_rules! impl_validated_struct_serialize {
    ($type:ty, $name:literal, $validator:ident, [$($field:ident),+ $(,)?]) => {
        impl Serialize for $type {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: Serializer,
            {
                self.$validator().map_err(serde::ser::Error::custom)?;
                let mut state = serializer.serialize_struct($name, 0 $(+ { let _ = stringify!($field); 1 })+)?;
                $(state.serialize_field(stringify!($field), &self.$field)?;)+
                state.end()
            }
        }
    };
}

pub const QUERY_PROTOCOL_MAJOR: u16 = 1;
pub const QUERY_PROTOCOL_MINOR: u16 = 0;
pub const PAGE_DEFAULT_LIMIT: u16 = 50;
pub const PAGE_MAX_LIMIT: u16 = 100;
pub const MAX_RUNTIME_DOMAIN_GENERATIONS: usize = 32;
pub const MAX_SCHEMA_CATALOG_ENTRIES: usize = 512;
pub const MAX_CAPABILITY_DISCOVERY_ENTRIES: usize = 512;
pub const MAX_OPERATION_IDS_PER_CAPABILITY: usize = 512;
pub const MAX_EVENT_PAGE_EVENTS: usize = 100;
pub const MIN_CURSOR_TOKEN_BYTES: usize = 16;
pub const MAX_CURSOR_TOKEN_BYTES: usize = 1024;
pub const MAX_RESOURCE_ID_BYTES: usize = 128;
pub const MAX_SCHEMA_ID_BYTES: usize = 192;
pub const MAX_DOMAIN_ID_BYTES: usize = 64;
pub const MAX_CAPABILITY_ID_BYTES: usize = 128;

mod query_payload_seal {
    pub trait Sealed {}
}

/// Marker for the closed set of DTOs allowed inside query pages and events.
///
/// The private supertrait prevents adapters and downstream crates from placing
/// arbitrary JSON, credentials, or application-internal snapshots on the wire.
pub trait ControlPlaneQueryPayload: query_payload_seal::Sealed + Serialize {
    fn validate_payload(&self) -> Result<(), QueryContractValidationError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct QueryProtocolVersion {
    pub major: u16,
    pub minor: u16,
}

impl Serialize for QueryProtocolVersion {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.ensure_compatible()
            .map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("QueryProtocolVersion", 2)?;
        state.serialize_field("major", &self.major)?;
        state.serialize_field("minor", &self.minor)?;
        state.end()
    }
}

impl QueryProtocolVersion {
    pub const CURRENT: Self = Self {
        major: QUERY_PROTOCOL_MAJOR,
        minor: QUERY_PROTOCOL_MINOR,
    };

    pub fn ensure_compatible(self) -> Result<(), QueryContractValidationError> {
        if self == Self::CURRENT {
            Ok(())
        } else {
            Err(QueryContractValidationError::UnsupportedProtocolVersion)
        }
    }
}

impl Default for QueryProtocolVersion {
    fn default() -> Self {
        Self::CURRENT
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryProtocolVersionWire {
    major: u16,
    minor: u16,
}

impl<'de> Deserialize<'de> for QueryProtocolVersion {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QueryProtocolVersionWire::deserialize(deserializer)?;
        let version = Self {
            major: wire.major,
            minor: wire.minor,
        };
        version.ensure_compatible().map_err(D::Error::custom)?;
        Ok(version)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryErrorCode {
    InvalidRequest,
    UnsupportedProtocolVersion,
    UnknownOperation,
    Unauthorized,
    Forbidden,
    NotFound,
    Unavailable,
    SchemaMismatch,
    CursorInvalid,
    CursorStale,
    SnapshotRequired,
    EventGap,
    RateLimited,
    Overloaded,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct QueryError {
    code: QueryErrorCode,
    message: String,
    retryable: bool,
    resnapshot_required: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryErrorWire {
    code: QueryErrorCode,
    message: String,
    retryable: bool,
    resnapshot_required: bool,
}

impl QueryError {
    pub fn from_code(code: QueryErrorCode) -> Self {
        let (message, retryable, resnapshot_required) = query_error_contract(code);
        Self {
            code,
            message: message.to_string(),
            retryable,
            resnapshot_required,
        }
    }

    pub fn code(&self) -> QueryErrorCode {
        self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn retryable(&self) -> bool {
        self.retryable
    }

    pub fn resnapshot_required(&self) -> bool {
        self.resnapshot_required
    }

    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        let (message, retryable, resnapshot_required) = query_error_contract(self.code);
        if self.message != message
            || self.retryable != retryable
            || self.resnapshot_required != resnapshot_required
        {
            return Err(QueryContractValidationError::InconsistentQueryError);
        }
        Ok(())
    }
}

fn query_error_contract(code: QueryErrorCode) -> (&'static str, bool, bool) {
    match code {
        QueryErrorCode::InvalidRequest => ("invalid request", false, false),
        QueryErrorCode::UnsupportedProtocolVersion => {
            ("unsupported protocol version", false, false)
        }
        QueryErrorCode::UnknownOperation => ("unknown operation", false, false),
        QueryErrorCode::Unauthorized => ("authentication required", false, false),
        QueryErrorCode::Forbidden => ("operation forbidden", false, false),
        QueryErrorCode::NotFound => ("resource not found", false, false),
        QueryErrorCode::SchemaMismatch => ("schema mismatch", false, false),
        QueryErrorCode::CursorInvalid => ("cursor invalid", false, false),
        QueryErrorCode::CursorStale => ("cursor stale", false, true),
        QueryErrorCode::SnapshotRequired => ("snapshot required", false, true),
        QueryErrorCode::EventGap => ("event gap detected", false, true),
        QueryErrorCode::RateLimited => ("rate limited", true, false),
        QueryErrorCode::Overloaded => ("query service overloaded", true, false),
        QueryErrorCode::Unavailable => ("query service unavailable", true, false),
        QueryErrorCode::Internal => ("internal query failure", true, false),
    }
}

impl<'de> Deserialize<'de> for QueryError {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QueryErrorWire::deserialize(deserializer)?;
        let error = Self {
            code: wire.code,
            message: wire.message,
            retryable: wire.retryable,
            resnapshot_required: wire.resnapshot_required,
        };
        error.validate().map_err(D::Error::custom)?;
        Ok(error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeDomainGeneration {
    pub domain: String,
    pub generation: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeDomainGenerationWire {
    domain: String,
    generation: u64,
}

impl RuntimeDomainGeneration {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        validate_dotted_id(&self.domain, MAX_DOMAIN_ID_BYTES)
    }
}

impl_validated_struct_serialize!(
    RuntimeDomainGeneration,
    "RuntimeDomainGeneration",
    validate,
    [domain, generation]
);

impl<'de> Deserialize<'de> for RuntimeDomainGeneration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeDomainGenerationWire::deserialize(deserializer)?;
        let generation = Self {
            domain: wire.domain,
            generation: wire.generation,
        };
        generation.validate().map_err(D::Error::custom)?;
        Ok(generation)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryFence {
    pub process_incarnation: u64,
    pub session_incarnation: u64,
    pub project_epoch: u64,
    pub project_revision: u64,
    pub project_history_generation: u64,
    pub project_checkpoint_hash: String,
    pub project_publication_generation: u64,
    pub output_epoch: u64,
    pub output_generation: u64,
    pub event_stream_epoch: u64,
    pub event_stream_generation: u64,
    pub runtime_domains: Vec<RuntimeDomainGeneration>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QueryFenceWire {
    process_incarnation: u64,
    session_incarnation: u64,
    project_epoch: u64,
    project_revision: u64,
    project_history_generation: u64,
    project_checkpoint_hash: String,
    project_publication_generation: u64,
    output_epoch: u64,
    output_generation: u64,
    event_stream_epoch: u64,
    event_stream_generation: u64,
    runtime_domains: Vec<RuntimeDomainGeneration>,
}

impl QueryFence {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        if self.process_incarnation == 0 || self.session_incarnation == 0 {
            return Err(QueryContractValidationError::ZeroIncarnation);
        }
        validate_fingerprint(&self.project_checkpoint_hash)?;
        if self.event_stream_epoch == 0 {
            return Err(QueryContractValidationError::ZeroEventStreamEpoch);
        }
        if self.runtime_domains.len() > MAX_RUNTIME_DOMAIN_GENERATIONS {
            return Err(QueryContractValidationError::TooManyRuntimeDomains);
        }
        let mut previous: Option<&str> = None;
        for domain in &self.runtime_domains {
            domain.validate()?;
            if let Some(previous) = previous {
                if domain.domain.as_str() == previous {
                    return Err(QueryContractValidationError::DuplicateRuntimeDomain);
                }
                if domain.domain.as_str() < previous {
                    return Err(QueryContractValidationError::UnsortedRuntimeDomains);
                }
            }
            previous = Some(domain.domain.as_str());
        }
        Ok(())
    }

    pub fn same_snapshot_as(&self, other: &Self) -> bool {
        self == other
    }
}

impl_validated_struct_serialize!(
    QueryFence,
    "QueryFence",
    validate,
    [
        process_incarnation,
        session_incarnation,
        project_epoch,
        project_revision,
        project_history_generation,
        project_checkpoint_hash,
        project_publication_generation,
        output_epoch,
        output_generation,
        event_stream_epoch,
        event_stream_generation,
        runtime_domains,
    ]
);

impl<'de> Deserialize<'de> for QueryFence {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QueryFenceWire::deserialize(deserializer)?;
        let fence = Self {
            process_incarnation: wire.process_incarnation,
            session_incarnation: wire.session_incarnation,
            project_epoch: wire.project_epoch,
            project_revision: wire.project_revision,
            project_history_generation: wire.project_history_generation,
            project_checkpoint_hash: wire.project_checkpoint_hash,
            project_publication_generation: wire.project_publication_generation,
            output_epoch: wire.output_epoch,
            output_generation: wire.output_generation,
            event_stream_epoch: wire.event_stream_epoch,
            event_stream_generation: wire.event_stream_generation,
            runtime_domains: wire.runtime_domains,
        };
        fence.validate().map_err(D::Error::custom)?;
        Ok(fence)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(transparent)]
pub struct OpaqueCursorToken(String);

impl OpaqueCursorToken {
    pub fn try_new(value: impl Into<String>) -> Result<Self, QueryContractValidationError> {
        let value = value.into();
        validate_cursor_token(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OpaqueCursorToken {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Self::try_new(String::deserialize(deserializer)?).map_err(D::Error::custom)
    }
}

/// Typed server-side state represented by an opaque, integrity-protected token.
///
/// It deliberately has no serde implementation: adapters must use a dedicated
/// authenticated codec rather than exposing these claims as client JSON.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CursorClaims {
    pub resource: String,
    pub schema: String,
    pub sort_fingerprint: String,
    pub filter_fingerprint: String,
    pub snapshot_fence: QueryFence,
    pub next_index: u64,
}

impl CursorClaims {
    pub fn validate(&self) -> Result<(), CursorValidationError> {
        validate_dotted_id(&self.resource, MAX_RESOURCE_ID_BYTES)
            .map_err(|_| CursorValidationError::InvalidClaims)?;
        validate_versioned_id(&self.schema, MAX_SCHEMA_ID_BYTES)
            .map_err(|_| CursorValidationError::InvalidClaims)?;
        validate_fingerprint(&self.sort_fingerprint)
            .map_err(|_| CursorValidationError::InvalidClaims)?;
        validate_fingerprint(&self.filter_fingerprint)
            .map_err(|_| CursorValidationError::InvalidClaims)?;
        self.snapshot_fence
            .validate()
            .map_err(|_| CursorValidationError::InvalidClaims)
    }

    pub fn validate_for(
        &self,
        resource: &str,
        schema: &str,
        sort_fingerprint: &str,
        filter_fingerprint: &str,
        current_fence: &QueryFence,
    ) -> Result<(), CursorValidationError> {
        self.validate()?;
        if self.resource != resource {
            return Err(CursorValidationError::ResourceMismatch);
        }
        if self.schema != schema {
            return Err(CursorValidationError::SchemaMismatch);
        }
        if self.sort_fingerprint != sort_fingerprint {
            return Err(CursorValidationError::SortMismatch);
        }
        if self.filter_fingerprint != filter_fingerprint {
            return Err(CursorValidationError::FilterMismatch);
        }
        current_fence
            .validate()
            .map_err(|_| CursorValidationError::InvalidCurrentFence)?;
        if self.is_stale_against(current_fence) {
            return Err(CursorValidationError::StaleSnapshot);
        }
        Ok(())
    }

    pub fn is_stale_against(&self, current_fence: &QueryFence) -> bool {
        !self.snapshot_fence.same_snapshot_as(current_fence)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorValidationError {
    InvalidClaims,
    InvalidCurrentFence,
    ResourceMismatch,
    SchemaMismatch,
    SortMismatch,
    FilterMismatch,
    StaleSnapshot,
}

impl CursorValidationError {
    pub fn query_error_code(self) -> QueryErrorCode {
        match self {
            Self::SchemaMismatch => QueryErrorCode::SchemaMismatch,
            Self::StaleSnapshot => QueryErrorCode::CursorStale,
            Self::InvalidClaims
            | Self::InvalidCurrentFence
            | Self::ResourceMismatch
            | Self::SortMismatch
            | Self::FilterMismatch => QueryErrorCode::CursorInvalid,
        }
    }

    pub fn resnapshot_required(self) -> bool {
        matches!(self, Self::StaleSnapshot)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageRequest {
    pub limit: u16,
    pub cursor: Option<OpaqueCursorToken>,
}

impl Serialize for PageRequest {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer
            .serialize_struct("PageRequest", if self.cursor.is_some() { 2 } else { 1 })?;
        state.serialize_field("limit", &self.limit)?;
        if let Some(cursor) = &self.cursor {
            state.serialize_field("cursor", cursor)?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageRequestWire {
    #[serde(default = "default_page_limit")]
    limit: u16,
    #[serde(default)]
    cursor: Option<OpaqueCursorToken>,
}

const fn default_page_limit() -> u16 {
    PAGE_DEFAULT_LIMIT
}

impl Default for PageRequest {
    fn default() -> Self {
        Self {
            limit: PAGE_DEFAULT_LIMIT,
            cursor: None,
        }
    }
}

impl PageRequest {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        if self.limit == 0 || self.limit > PAGE_MAX_LIMIT {
            return Err(QueryContractValidationError::InvalidPageLimit);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for PageRequest {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PageRequestWire::deserialize(deserializer)?;
        let request = Self {
            limit: wire.limit,
            cursor: wire.cursor,
        };
        request.validate().map_err(D::Error::custom)?;
        Ok(request)
    }
}

/// A bounded snapshot page containing only protocol-approved payload DTOs.
///
/// Arbitrary downstream payloads are rejected at compile time:
///
/// ```
/// use protocol::control_plane_query::{Page, QuerySchemaDescriptor};
/// fn accepts_approved(_: Option<Page<QuerySchemaDescriptor>>) {}
/// accepts_approved(None);
/// ```
///
/// ```compile_fail
/// use protocol::control_plane_query::Page;
/// let _: Option<Page<String>> = None;
/// ```
///
/// ```compile_fail
/// use protocol::control_plane_query::Page;
/// let _: Option<Page<serde_json::Value>> = None;
/// ```
///
/// ```compile_fail
/// use protocol::control_plane_query::{
///     ControlPlaneQueryPayload, Page, QueryContractValidationError,
/// };
/// #[derive(serde::Serialize)]
/// struct DownstreamPayload;
/// impl ControlPlaneQueryPayload for DownstreamPayload {
///     fn validate_payload(&self) -> Result<(), QueryContractValidationError> {
///         Ok(())
///     }
/// }
/// let _: Option<Page<DownstreamPayload>> = None;
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Page<T: ControlPlaneQueryPayload> {
    pub protocol_version: QueryProtocolVersion,
    pub resource: String,
    pub snapshot_fence: QueryFence,
    pub items: Vec<T>,
    pub next_cursor: Option<OpaqueCursorToken>,
}

impl<T: ControlPlaneQueryPayload> Serialize for Page<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state =
            serializer.serialize_struct("Page", if self.next_cursor.is_some() { 5 } else { 4 })?;
        state.serialize_field("protocol_version", &self.protocol_version)?;
        state.serialize_field("resource", &self.resource)?;
        state.serialize_field("snapshot_fence", &self.snapshot_fence)?;
        state.serialize_field("items", &self.items)?;
        if let Some(cursor) = &self.next_cursor {
            state.serialize_field("next_cursor", cursor)?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PageWire<T> {
    protocol_version: QueryProtocolVersion,
    resource: String,
    snapshot_fence: QueryFence,
    items: Vec<T>,
    #[serde(default)]
    next_cursor: Option<OpaqueCursorToken>,
}

impl<T: ControlPlaneQueryPayload> Page<T> {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        self.protocol_version.ensure_compatible()?;
        validate_dotted_id(&self.resource, MAX_RESOURCE_ID_BYTES)?;
        self.snapshot_fence.validate()?;
        if self.items.len() > PAGE_MAX_LIMIT as usize {
            return Err(QueryContractValidationError::TooManyPageItems);
        }
        for item in &self.items {
            item.validate_payload()?;
        }
        Ok(())
    }
}

impl<'de, T> Deserialize<'de> for Page<T>
where
    T: ControlPlaneQueryPayload + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PageWire::<T>::deserialize(deserializer)?;
        let page = Self {
            protocol_version: wire.protocol_version,
            resource: wire.resource,
            snapshot_fence: wire.snapshot_fence,
            items: wire.items,
            next_cursor: wire.next_cursor,
        };
        page.validate().map_err(D::Error::custom)?;
        Ok(page)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuerySchemaKind {
    Request,
    Response,
    Resource,
    Event,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QuerySchemaDescriptor {
    pub schema_id: String,
    pub version: QueryProtocolVersion,
    pub kind: QuerySchemaKind,
}

impl_validated_struct_serialize!(
    QuerySchemaDescriptor,
    "QuerySchemaDescriptor",
    validate,
    [schema_id, version, kind]
);

impl QuerySchemaDescriptor {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        validate_versioned_id(&self.schema_id, MAX_SCHEMA_ID_BYTES)?;
        self.version.ensure_compatible()
    }
}

impl query_payload_seal::Sealed for QuerySchemaDescriptor {}
impl ControlPlaneQueryPayload for QuerySchemaDescriptor {
    fn validate_payload(&self) -> Result<(), QueryContractValidationError> {
        self.validate()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct QuerySchemaDescriptorWire {
    schema_id: String,
    version: QueryProtocolVersion,
    kind: QuerySchemaKind,
}

impl<'de> Deserialize<'de> for QuerySchemaDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = QuerySchemaDescriptorWire::deserialize(deserializer)?;
        let descriptor = Self {
            schema_id: wire.schema_id,
            version: wire.version,
            kind: wire.kind,
        };
        descriptor.validate().map_err(D::Error::custom)?;
        Ok(descriptor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaCatalog {
    pub protocol_version: QueryProtocolVersion,
    pub catalog_generation: u64,
    pub schemas: Vec<QuerySchemaDescriptor>,
}

impl_validated_struct_serialize!(
    SchemaCatalog,
    "SchemaCatalog",
    validate,
    [protocol_version, catalog_generation, schemas]
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SchemaCatalogWire {
    protocol_version: QueryProtocolVersion,
    catalog_generation: u64,
    schemas: Vec<QuerySchemaDescriptor>,
}

impl SchemaCatalog {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        self.protocol_version.ensure_compatible()?;
        if self.schemas.len() > MAX_SCHEMA_CATALOG_ENTRIES {
            return Err(QueryContractValidationError::TooManySchemaEntries);
        }
        let mut previous: Option<&str> = None;
        for schema in &self.schemas {
            schema.validate()?;
            if let Some(previous) = previous {
                if schema.schema_id.as_str() == previous {
                    return Err(QueryContractValidationError::DuplicateSchemaId);
                }
                if schema.schema_id.as_str() < previous {
                    return Err(QueryContractValidationError::UnsortedSchemaIds);
                }
            }
            previous = Some(schema.schema_id.as_str());
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for SchemaCatalog {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SchemaCatalogWire::deserialize(deserializer)?;
        let catalog = Self {
            protocol_version: wire.protocol_version,
            catalog_generation: wire.catalog_generation,
            schemas: wire.schemas,
        };
        catalog.validate().map_err(D::Error::custom)?;
        Ok(catalog)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDescriptor {
    pub capability_id: String,
    pub operation_ids: Vec<String>,
    pub allowed_during_full_lock: bool,
}

impl_validated_struct_serialize!(
    CapabilityDescriptor,
    "CapabilityDescriptor",
    validate,
    [capability_id, operation_ids, allowed_during_full_lock,]
);

impl CapabilityDescriptor {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        validate_dotted_id(&self.capability_id, MAX_CAPABILITY_ID_BYTES)?;
        if self.operation_ids.len() > MAX_OPERATION_IDS_PER_CAPABILITY {
            return Err(QueryContractValidationError::TooManyCapabilityOperations);
        }
        let mut operations = BTreeSet::new();
        for operation_id in &self.operation_ids {
            validate_versioned_id(operation_id, MAX_SCHEMA_ID_BYTES)?;
            if !operations.insert(operation_id) {
                return Err(QueryContractValidationError::DuplicateCapabilityOperation);
            }
        }
        if self.operation_ids.windows(2).any(|pair| pair[0] > pair[1]) {
            return Err(QueryContractValidationError::UnsortedCapabilityOperations);
        }
        Ok(())
    }
}

impl query_payload_seal::Sealed for CapabilityDescriptor {}
impl ControlPlaneQueryPayload for CapabilityDescriptor {
    fn validate_payload(&self) -> Result<(), QueryContractValidationError> {
        self.validate()
    }
}

/// Closed, bounded event payload for a runtime-domain generation observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeGenerationPayload {
    pub domain: String,
    pub generation: u64,
    pub active: bool,
}

impl_validated_struct_serialize!(
    RuntimeGenerationPayload,
    "RuntimeGenerationPayload",
    validate,
    [domain, generation, active]
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeGenerationPayloadWire {
    domain: String,
    generation: u64,
    active: bool,
}

impl RuntimeGenerationPayload {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        validate_dotted_id(&self.domain, MAX_DOMAIN_ID_BYTES)?;
        if self.generation == 0 {
            return Err(QueryContractValidationError::ZeroEventGeneration);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for RuntimeGenerationPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeGenerationPayloadWire::deserialize(deserializer)?;
        let payload = Self {
            domain: wire.domain,
            generation: wire.generation,
            active: wire.active,
        };
        payload.validate().map_err(D::Error::custom)?;
        Ok(payload)
    }
}

impl query_payload_seal::Sealed for RuntimeGenerationPayload {}
impl ControlPlaneQueryPayload for RuntimeGenerationPayload {
    fn validate_payload(&self) -> Result<(), QueryContractValidationError> {
        self.validate()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityDescriptorWire {
    capability_id: String,
    operation_ids: Vec<String>,
    allowed_during_full_lock: bool,
}

impl<'de> Deserialize<'de> for CapabilityDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityDescriptorWire::deserialize(deserializer)?;
        let descriptor = Self {
            capability_id: wire.capability_id,
            operation_ids: wire.operation_ids,
            allowed_during_full_lock: wire.allowed_during_full_lock,
        };
        descriptor.validate().map_err(D::Error::custom)?;
        Ok(descriptor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapabilityDiscovery {
    pub protocol_version: QueryProtocolVersion,
    pub discovery_generation: u64,
    pub snapshot_fence: QueryFence,
    pub capabilities: Vec<CapabilityDescriptor>,
}

impl_validated_struct_serialize!(
    CapabilityDiscovery,
    "CapabilityDiscovery",
    validate,
    [
        protocol_version,
        discovery_generation,
        snapshot_fence,
        capabilities,
    ]
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CapabilityDiscoveryWire {
    protocol_version: QueryProtocolVersion,
    discovery_generation: u64,
    snapshot_fence: QueryFence,
    capabilities: Vec<CapabilityDescriptor>,
}

impl CapabilityDiscovery {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        self.protocol_version.ensure_compatible()?;
        self.snapshot_fence.validate()?;
        if self.capabilities.len() > MAX_CAPABILITY_DISCOVERY_ENTRIES {
            return Err(QueryContractValidationError::TooManyCapabilities);
        }
        let mut previous: Option<&str> = None;
        for capability in &self.capabilities {
            capability.validate()?;
            if let Some(previous) = previous {
                if capability.capability_id.as_str() == previous {
                    return Err(QueryContractValidationError::DuplicateCapabilityId);
                }
                if capability.capability_id.as_str() < previous {
                    return Err(QueryContractValidationError::UnsortedCapabilityIds);
                }
            }
            previous = Some(capability.capability_id.as_str());
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for CapabilityDiscovery {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityDiscoveryWire::deserialize(deserializer)?;
        let discovery = Self {
            protocol_version: wire.protocol_version,
            discovery_generation: wire.discovery_generation,
            snapshot_fence: wire.snapshot_fence,
            capabilities: wire.capabilities,
        };
        discovery.validate().map_err(D::Error::custom)?;
        Ok(discovery)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlPlaneEvent<T: ControlPlaneQueryPayload> {
    pub protocol_version: QueryProtocolVersion,
    pub resource: String,
    pub schema: String,
    pub stream_epoch: u64,
    pub generation: u64,
    pub payload: T,
}

impl<T: ControlPlaneQueryPayload> Serialize for ControlPlaneEvent<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("ControlPlaneEvent", 6)?;
        state.serialize_field("protocol_version", &self.protocol_version)?;
        state.serialize_field("resource", &self.resource)?;
        state.serialize_field("schema", &self.schema)?;
        state.serialize_field("stream_epoch", &self.stream_epoch)?;
        state.serialize_field("generation", &self.generation)?;
        state.serialize_field("payload", &self.payload)?;
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ControlPlaneEventWire<T> {
    protocol_version: QueryProtocolVersion,
    resource: String,
    schema: String,
    stream_epoch: u64,
    generation: u64,
    payload: T,
}

impl<T: ControlPlaneQueryPayload> ControlPlaneEvent<T> {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        self.protocol_version.ensure_compatible()?;
        validate_dotted_id(&self.resource, MAX_RESOURCE_ID_BYTES)?;
        if self.schema != format!("{}.event.v1", self.resource) {
            return Err(QueryContractValidationError::EventSchemaMismatch);
        }
        if self.stream_epoch == 0 {
            return Err(QueryContractValidationError::ZeroEventStreamEpoch);
        }
        if self.generation == 0 {
            return Err(QueryContractValidationError::ZeroEventGeneration);
        }
        self.payload.validate_payload()?;
        Ok(())
    }
}

impl<'de, T> Deserialize<'de> for ControlPlaneEvent<T>
where
    T: ControlPlaneQueryPayload + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ControlPlaneEventWire::<T>::deserialize(deserializer)?;
        let event = Self {
            protocol_version: wire.protocol_version,
            resource: wire.resource,
            schema: wire.schema,
            stream_epoch: wire.stream_epoch,
            generation: wire.generation,
            payload: wire.payload,
        };
        event.validate().map_err(D::Error::custom)?;
        Ok(event)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GapReason {
    SubscriberOverflow,
    RetentionExpired,
    EpochChanged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GapMarker {
    pub previous_stream_epoch: u64,
    pub previous_generation: u64,
    pub next_stream_epoch: u64,
    pub next_available_generation: u64,
    pub reason: GapReason,
    pub resnapshot_required: bool,
}

impl_validated_struct_serialize!(
    GapMarker,
    "GapMarker",
    validate,
    [
        previous_stream_epoch,
        previous_generation,
        next_stream_epoch,
        next_available_generation,
        reason,
        resnapshot_required,
    ]
);

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct GapMarkerWire {
    previous_stream_epoch: u64,
    previous_generation: u64,
    next_stream_epoch: u64,
    next_available_generation: u64,
    reason: GapReason,
    resnapshot_required: bool,
}

impl GapMarker {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        if !self.resnapshot_required {
            return Err(QueryContractValidationError::InconsistentResnapshotFlag);
        }
        if self.previous_stream_epoch == 0 || self.next_stream_epoch == 0 {
            return Err(QueryContractValidationError::ZeroEventStreamEpoch);
        }
        if self.previous_generation == 0 || self.next_available_generation == 0 {
            return Err(QueryContractValidationError::ZeroEventGeneration);
        }
        match self.reason {
            GapReason::EpochChanged
                if self.next_stream_epoch <= self.previous_stream_epoch
                    || self.next_available_generation != 1 =>
            {
                Err(QueryContractValidationError::InconsistentGapMarker)
            }
            GapReason::SubscriberOverflow | GapReason::RetentionExpired
                if self.previous_stream_epoch != self.next_stream_epoch =>
            {
                Err(QueryContractValidationError::InconsistentGapMarker)
            }
            GapReason::SubscriberOverflow | GapReason::RetentionExpired
                if self
                    .previous_generation
                    .checked_add(1)
                    .map_or(true, |next| self.next_available_generation <= next) =>
            {
                Err(QueryContractValidationError::InconsistentGapMarker)
            }
            _ => Ok(()),
        }
    }
}

impl<'de> Deserialize<'de> for GapMarker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = GapMarkerWire::deserialize(deserializer)?;
        let gap = Self {
            previous_stream_epoch: wire.previous_stream_epoch,
            previous_generation: wire.previous_generation,
            next_stream_epoch: wire.next_stream_epoch,
            next_available_generation: wire.next_available_generation,
            reason: wire.reason,
            resnapshot_required: wire.resnapshot_required,
        };
        gap.validate().map_err(D::Error::custom)?;
        Ok(gap)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EventPage<T: ControlPlaneQueryPayload> {
    pub protocol_version: QueryProtocolVersion,
    pub resource: String,
    pub snapshot_fence: QueryFence,
    pub events: Vec<ControlPlaneEvent<T>>,
    pub gap: Option<GapMarker>,
    pub next_cursor: Option<OpaqueCursorToken>,
}

impl<T: ControlPlaneQueryPayload> Serialize for EventPage<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let optional_fields =
            usize::from(self.gap.is_some()) + usize::from(self.next_cursor.is_some());
        let mut state = serializer.serialize_struct("EventPage", 4 + optional_fields)?;
        state.serialize_field("protocol_version", &self.protocol_version)?;
        state.serialize_field("resource", &self.resource)?;
        state.serialize_field("snapshot_fence", &self.snapshot_fence)?;
        state.serialize_field("events", &self.events)?;
        if let Some(gap) = &self.gap {
            state.serialize_field("gap", gap)?;
        }
        if let Some(cursor) = &self.next_cursor {
            state.serialize_field("next_cursor", cursor)?;
        }
        state.end()
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EventPageWire<T: ControlPlaneQueryPayload> {
    protocol_version: QueryProtocolVersion,
    resource: String,
    snapshot_fence: QueryFence,
    events: Vec<ControlPlaneEvent<T>>,
    #[serde(default)]
    gap: Option<GapMarker>,
    #[serde(default)]
    next_cursor: Option<OpaqueCursorToken>,
}

impl<T: ControlPlaneQueryPayload> EventPage<T> {
    pub fn validate(&self) -> Result<(), QueryContractValidationError> {
        self.protocol_version.ensure_compatible()?;
        validate_dotted_id(&self.resource, MAX_RESOURCE_ID_BYTES)?;
        self.snapshot_fence.validate()?;
        if self.events.len() > MAX_EVENT_PAGE_EVENTS {
            return Err(QueryContractValidationError::TooManyEvents);
        }
        if let Some(gap) = &self.gap {
            gap.validate()?;
            if !self.events.is_empty() || self.next_cursor.is_some() {
                return Err(QueryContractValidationError::AmbiguousGapPage);
            }
            if gap.previous_stream_epoch != self.snapshot_fence.event_stream_epoch
                || gap.previous_generation != self.snapshot_fence.event_stream_generation
            {
                return Err(QueryContractValidationError::InconsistentGapMarker);
            }
            return Ok(());
        }
        let mut previous_generation = self.snapshot_fence.event_stream_generation;
        for event in &self.events {
            event.validate()?;
            let expected = previous_generation
                .checked_add(1)
                .ok_or(QueryContractValidationError::GenerationOverflow)?;
            if event.resource != self.resource
                || event.stream_epoch != self.snapshot_fence.event_stream_epoch
                || event.generation != expected
            {
                return Err(QueryContractValidationError::NonContiguousEventSequence);
            }
            previous_generation = event.generation;
        }
        Ok(())
    }
}

impl<'de, T> Deserialize<'de> for EventPage<T>
where
    T: ControlPlaneQueryPayload + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = EventPageWire::<T>::deserialize(deserializer)?;
        let page = Self {
            protocol_version: wire.protocol_version,
            resource: wire.resource,
            snapshot_fence: wire.snapshot_fence,
            events: wire.events,
            gap: wire.gap,
            next_cursor: wire.next_cursor,
        };
        page.validate().map_err(D::Error::custom)?;
        Ok(page)
    }
}

fn validate_cursor_token(token: &str) -> Result<(), QueryContractValidationError> {
    if token.len() < MIN_CURSOR_TOKEN_BYTES
        || token.len() > MAX_CURSOR_TOKEN_BYTES
        || !token
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(QueryContractValidationError::InvalidCursorToken);
    }
    Ok(())
}

fn validate_fingerprint(value: &str) -> Result<(), QueryContractValidationError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(QueryContractValidationError::InvalidFingerprint);
    }
    Ok(())
}

fn validate_dotted_id(value: &str, max_bytes: usize) -> Result<(), QueryContractValidationError> {
    if value.is_empty()
        || value.len() > max_bytes
        || !value
            .split('.')
            .all(|segment| validate_lower_id_segment(segment))
    {
        return Err(QueryContractValidationError::InvalidIdentifier);
    }
    Ok(())
}

fn validate_versioned_id(
    value: &str,
    max_bytes: usize,
) -> Result<(), QueryContractValidationError> {
    validate_dotted_id(value, max_bytes)?;
    let Some(version) = value.rsplit('.').next() else {
        return Err(QueryContractValidationError::UnversionedIdentifier);
    };
    let Some(digits) = version.strip_prefix('v') else {
        return Err(QueryContractValidationError::UnversionedIdentifier);
    };
    let Ok(number) = digits.parse::<u16>() else {
        return Err(QueryContractValidationError::UnversionedIdentifier);
    };
    if number == 0 || digits != number.to_string() {
        return Err(QueryContractValidationError::UnversionedIdentifier);
    }
    Ok(())
}

fn validate_lower_id_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && bytes.all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'_' | b'-')
        })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryContractValidationError {
    UnsupportedProtocolVersion,
    InconsistentQueryError,
    InconsistentResnapshotFlag,
    ZeroIncarnation,
    ZeroEventStreamEpoch,
    ZeroEventGeneration,
    TooManyRuntimeDomains,
    DuplicateRuntimeDomain,
    UnsortedRuntimeDomains,
    InvalidCursorToken,
    InvalidFingerprint,
    InvalidIdentifier,
    UnversionedIdentifier,
    InvalidPageLimit,
    TooManyPageItems,
    TooManySchemaEntries,
    DuplicateSchemaId,
    UnsortedSchemaIds,
    TooManyCapabilities,
    DuplicateCapabilityId,
    UnsortedCapabilityIds,
    TooManyCapabilityOperations,
    DuplicateCapabilityOperation,
    UnsortedCapabilityOperations,
    TooManyEvents,
    AmbiguousGapPage,
    InconsistentGapMarker,
    EventSchemaMismatch,
    NonContiguousEventSequence,
    GenerationOverflow,
}

impl fmt::Display for QueryContractValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnsupportedProtocolVersion => "query protocol major and minor must match exactly",
            Self::InconsistentQueryError => {
                "query error message and flags must be derived exactly from its code"
            }
            Self::InconsistentResnapshotFlag => "query result requires an explicit resnapshot flag",
            Self::ZeroIncarnation => "process and session incarnations must be non-zero",
            Self::ZeroEventStreamEpoch => "event stream epoch must be non-zero",
            Self::ZeroEventGeneration => "event generation must be non-zero",
            Self::TooManyRuntimeDomains => "runtime-domain fence exceeds its bound",
            Self::DuplicateRuntimeDomain => "runtime-domain fence contains a duplicate",
            Self::UnsortedRuntimeDomains => "runtime-domain fence is not sorted",
            Self::InvalidCursorToken => {
                "cursor token is undersized, oversized, or not opaque ASCII"
            }
            Self::InvalidFingerprint => "query fingerprint must be lowercase hexadecimal sha256",
            Self::InvalidIdentifier => "query identifier is invalid or oversized",
            Self::UnversionedIdentifier => "schema and operation ids must end in canonical vN",
            Self::InvalidPageLimit => "page limit must be between one and the protocol maximum",
            Self::TooManyPageItems => "page exceeds the protocol item bound",
            Self::TooManySchemaEntries => "schema catalog exceeds its bound",
            Self::DuplicateSchemaId => "schema catalog contains a duplicate id",
            Self::UnsortedSchemaIds => "schema catalog is not sorted",
            Self::TooManyCapabilities => "capability discovery exceeds its bound",
            Self::DuplicateCapabilityId => "capability discovery contains a duplicate id",
            Self::UnsortedCapabilityIds => "capability discovery is not sorted",
            Self::TooManyCapabilityOperations => "capability operation list exceeds its bound",
            Self::DuplicateCapabilityOperation => "capability operation list contains a duplicate",
            Self::UnsortedCapabilityOperations => "capability operation list is not sorted",
            Self::TooManyEvents => "event page exceeds its bound",
            Self::AmbiguousGapPage => "a gap page cannot also contain events or a cursor",
            Self::InconsistentGapMarker => "gap marker does not match its fence or reason",
            Self::EventSchemaMismatch => "event schema must exactly match its resource event v1",
            Self::NonContiguousEventSequence => "events must be an exact N plus one sequence",
            Self::GenerationOverflow => "event generation overflowed",
        })
    }
}

impl std::error::Error for QueryContractValidationError {}

impl fmt::Display for CursorValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidClaims => "cursor claims are invalid",
            Self::InvalidCurrentFence => "current query fence is invalid",
            Self::ResourceMismatch => "cursor belongs to another resource",
            Self::SchemaMismatch => "cursor belongs to another schema",
            Self::SortMismatch => "cursor sort fingerprint does not match",
            Self::FilterMismatch => "cursor filter fingerprint does not match",
            Self::StaleSnapshot => "cursor snapshot fence is stale",
        })
    }
}

impl std::error::Error for CursorValidationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fence(generation: u64) -> QueryFence {
        QueryFence {
            process_incarnation: 11,
            session_incarnation: 12,
            project_epoch: 2,
            project_revision: 3,
            project_history_generation: 4,
            project_checkpoint_hash: fingerprint('c'),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            event_stream_epoch: 8,
            event_stream_generation: generation,
            runtime_domains: vec![
                RuntimeDomainGeneration {
                    domain: "audio.transport".to_string(),
                    generation: 9,
                },
                RuntimeDomainGeneration {
                    domain: "video.output".to_string(),
                    generation: 10,
                },
            ],
        }
    }

    fn fingerprint(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn claims() -> CursorClaims {
        CursorClaims {
            resource: "syndocal.query.timeline".to_string(),
            schema: "syndocal.query.timeline.response.v1".to_string(),
            sort_fingerprint: fingerprint('a'),
            filter_fingerprint: fingerprint('b'),
            snapshot_fence: fence(20),
            next_index: 50,
        }
    }

    fn runtime_payload(generation: u64, active: bool) -> RuntimeGenerationPayload {
        RuntimeGenerationPayload {
            domain: "audio.transport".to_string(),
            generation,
            active,
        }
    }

    #[test]
    fn protocol_version_requires_exact_major_and_minor() {
        QueryProtocolVersion::CURRENT.ensure_compatible().unwrap();
        assert_eq!(
            QueryProtocolVersion { major: 2, minor: 0 }.ensure_compatible(),
            Err(QueryContractValidationError::UnsupportedProtocolVersion)
        );
        assert_eq!(
            QueryProtocolVersion { major: 1, minor: 1 }.ensure_compatible(),
            Err(QueryContractValidationError::UnsupportedProtocolVersion)
        );
        assert!(serde_json::from_value::<QueryProtocolVersion>(json!({
            "major": 1,
            "minor": 1
        }))
        .is_err());
        assert!(serde_json::from_value::<QueryProtocolVersion>(json!({
            "major": 1,
            "minor": 0,
            "patch": 1
        }))
        .is_err());
    }

    #[test]
    fn query_error_is_exactly_code_derived_and_rejects_secret_like_text() {
        let error = QueryError::from_code(QueryErrorCode::CursorStale);
        assert_eq!(
            serde_json::to_string(&error).unwrap(),
            r#"{"code":"cursor_stale","message":"cursor stale","retryable":false,"resnapshot_required":true}"#
        );
        for invalid in [
            json!({
                "code": "cursor_stale",
                "message": "password token",
                "retryable": false,
                "resnapshot_required": true
            }),
            json!({
                "code": "internal",
                "message": "C:\\secret",
                "retryable": true,
                "resnapshot_required": false
            }),
            json!({
                "code": "cursor_stale",
                "message": "cursor stale",
                "retryable": true,
                "resnapshot_required": true
            }),
            json!({
                "code": "cursor_stale",
                "message": "cursor stale",
                "retryable": false,
                "resnapshot_required": false
            }),
        ] {
            assert!(serde_json::from_value::<QueryError>(invalid).is_err());
        }

        let exact = [
            (QueryErrorCode::InvalidRequest, false, false),
            (QueryErrorCode::UnsupportedProtocolVersion, false, false),
            (QueryErrorCode::UnknownOperation, false, false),
            (QueryErrorCode::Unauthorized, false, false),
            (QueryErrorCode::Forbidden, false, false),
            (QueryErrorCode::NotFound, false, false),
            (QueryErrorCode::SchemaMismatch, false, false),
            (QueryErrorCode::CursorInvalid, false, false),
            (QueryErrorCode::CursorStale, false, true),
            (QueryErrorCode::SnapshotRequired, false, true),
            (QueryErrorCode::EventGap, false, true),
            (QueryErrorCode::RateLimited, true, false),
            (QueryErrorCode::Overloaded, true, false),
            (QueryErrorCode::Unavailable, true, false),
            (QueryErrorCode::Internal, true, false),
        ];
        for (code, retryable, resnapshot_required) in exact {
            let error = QueryError::from_code(code);
            assert_eq!(error.retryable(), retryable);
            assert_eq!(error.resnapshot_required(), resnapshot_required);
            let roundtrip: QueryError =
                serde_json::from_slice(&serde_json::to_vec(&error).unwrap()).unwrap();
            assert_eq!(roundtrip, error);
        }
    }

    #[test]
    fn cursor_claims_reject_cross_resource_filter_and_stale_fence() {
        let claims = claims();
        claims
            .validate_for(
                "syndocal.query.timeline",
                "syndocal.query.timeline.response.v1",
                &fingerprint('a'),
                &fingerprint('b'),
                &fence(20),
            )
            .unwrap();
        assert_eq!(
            claims.validate_for(
                "syndocal.query.cue",
                "syndocal.query.timeline.response.v1",
                &fingerprint('a'),
                &fingerprint('b'),
                &fence(20),
            ),
            Err(CursorValidationError::ResourceMismatch)
        );
        assert_eq!(
            claims.validate_for(
                "syndocal.query.timeline",
                "syndocal.query.timeline.response.v2",
                &fingerprint('a'),
                &fingerprint('b'),
                &fence(20),
            ),
            Err(CursorValidationError::SchemaMismatch)
        );
        assert_eq!(
            claims.validate_for(
                "syndocal.query.timeline",
                "syndocal.query.timeline.response.v1",
                &fingerprint('a'),
                &fingerprint('c'),
                &fence(20),
            ),
            Err(CursorValidationError::FilterMismatch)
        );
        assert_eq!(
            claims.validate_for(
                "syndocal.query.timeline",
                "syndocal.query.timeline.response.v1",
                &fingerprint('a'),
                &fingerprint('b'),
                &fence(21),
            ),
            Err(CursorValidationError::StaleSnapshot)
        );
        assert!(serde_json::to_value(&claims.resource).is_ok());
    }

    #[test]
    fn page_request_defaults_to_fifty_and_rejects_out_of_bounds() {
        let request: PageRequest = serde_json::from_str("{}").unwrap();
        assert_eq!(request, PageRequest::default());
        assert_eq!(request.limit, 50);
        assert!(serde_json::from_str::<PageRequest>(r#"{"limit":0}"#).is_err());
        assert!(serde_json::from_str::<PageRequest>(r#"{"limit":101}"#).is_err());
        assert!(serde_json::from_str::<PageRequest>(r#"{"limit":50,"extra":1}"#).is_err());
        assert!(OpaqueCursorToken::try_new("bad token").is_err());
    }

    #[test]
    fn invalid_outbound_dtos_cannot_be_serialized() {
        fn rejected(value: &impl Serialize) {
            assert!(
                serde_json::to_vec(value).is_err(),
                "invalid outbound DTO must fail before bytes are produced"
            );
        }

        rejected(&QueryProtocolVersion { major: 1, minor: 1 });
        rejected(&RuntimeDomainGeneration {
            domain: "C:\\secret".to_string(),
            generation: 1,
        });

        let mut invalid_fence = fence(1);
        invalid_fence.event_stream_epoch = 0;
        rejected(&invalid_fence);

        rejected(&PageRequest {
            limit: PAGE_MAX_LIMIT + 1,
            cursor: None,
        });

        let invalid_schema = QuerySchemaDescriptor {
            schema_id: "syndocal.query.unversioned".to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        };
        rejected(&invalid_schema);
        rejected(&Page {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.query.project".to_string(),
            snapshot_fence: fence(1),
            items: vec![invalid_schema.clone()],
            next_cursor: None,
        });
        rejected(&SchemaCatalog {
            protocol_version: QueryProtocolVersion::CURRENT,
            catalog_generation: 1,
            schemas: vec![invalid_schema],
        });

        let invalid_capability = CapabilityDescriptor {
            capability_id: "project.read".to_string(),
            operation_ids: vec!["syndocal.query.project".to_string()],
            allowed_during_full_lock: true,
        };
        rejected(&invalid_capability);
        rejected(&CapabilityDiscovery {
            protocol_version: QueryProtocolVersion::CURRENT,
            discovery_generation: 1,
            snapshot_fence: fence(1),
            capabilities: vec![invalid_capability],
        });

        let invalid_payload = RuntimeGenerationPayload {
            domain: "C:\\secret".to_string(),
            generation: 0,
            active: true,
        };
        rejected(&invalid_payload);
        rejected(&ControlPlaneEvent {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            schema: "syndocal.runtime.transport.event.v1".to_string(),
            stream_epoch: 8,
            generation: 2,
            payload: invalid_payload,
        });

        let invalid_gap = GapMarker {
            previous_stream_epoch: 8,
            previous_generation: 1,
            next_stream_epoch: 8,
            next_available_generation: 2,
            reason: GapReason::SubscriberOverflow,
            resnapshot_required: true,
        };
        rejected(&invalid_gap);
        rejected(&EventPage::<RuntimeGenerationPayload> {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            snapshot_fence: fence(1),
            events: Vec::new(),
            gap: Some(invalid_gap),
            next_cursor: None,
        });
    }

    #[test]
    fn fence_requires_sorted_unique_bounded_domains() {
        fence(1).validate().unwrap();
        let mut bad_hash = fence(1);
        bad_hash.project_checkpoint_hash = fingerprint('A');
        assert_eq!(
            bad_hash.validate(),
            Err(QueryContractValidationError::InvalidFingerprint)
        );
        let mut zero_epoch = fence(1);
        zero_epoch.event_stream_epoch = 0;
        assert_eq!(
            zero_epoch.validate(),
            Err(QueryContractValidationError::ZeroEventStreamEpoch)
        );
        let mut duplicate = fence(1);
        duplicate.runtime_domains[1].domain = "audio.transport".to_string();
        assert_eq!(
            duplicate.validate(),
            Err(QueryContractValidationError::DuplicateRuntimeDomain)
        );
        let mut unsorted = fence(1);
        unsorted.runtime_domains.reverse();
        assert_eq!(
            unsorted.validate(),
            Err(QueryContractValidationError::UnsortedRuntimeDomains)
        );
        let mut oversized = fence(1);
        oversized.runtime_domains = (0..=MAX_RUNTIME_DOMAIN_GENERATIONS)
            .map(|index| RuntimeDomainGeneration {
                domain: format!("domain.d{index:02}"),
                generation: index as u64,
            })
            .collect();
        assert_eq!(
            oversized.validate(),
            Err(QueryContractValidationError::TooManyRuntimeDomains)
        );
        let mut unknown = serde_json::to_value(fence(1)).unwrap();
        unknown["secret"] = json!(true);
        assert!(serde_json::from_value::<QueryFence>(unknown).is_err());
    }

    #[test]
    fn event_page_requires_n_plus_one_and_marks_epoch_gaps() {
        let page = EventPage {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            snapshot_fence: fence(40),
            events: vec![
                ControlPlaneEvent {
                    protocol_version: QueryProtocolVersion::CURRENT,
                    resource: "syndocal.runtime.transport".to_string(),
                    schema: "syndocal.runtime.transport.event.v1".to_string(),
                    stream_epoch: 8,
                    generation: 41,
                    payload: runtime_payload(41, true),
                },
                ControlPlaneEvent {
                    protocol_version: QueryProtocolVersion::CURRENT,
                    resource: "syndocal.runtime.transport".to_string(),
                    schema: "syndocal.runtime.transport.event.v1".to_string(),
                    stream_epoch: 8,
                    generation: 42,
                    payload: runtime_payload(42, false),
                },
            ],
            gap: None,
            next_cursor: None,
        };
        page.validate().unwrap();
        let mut skipped = page.clone();
        skipped.events[1].generation = 43;
        assert_eq!(
            skipped.validate(),
            Err(QueryContractValidationError::NonContiguousEventSequence)
        );

        let gap_page: EventPage<RuntimeGenerationPayload> = EventPage {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            snapshot_fence: fence(40),
            events: Vec::new(),
            gap: Some(GapMarker {
                previous_stream_epoch: 8,
                previous_generation: 40,
                next_stream_epoch: 9,
                next_available_generation: 1,
                reason: GapReason::EpochChanged,
                resnapshot_required: true,
            }),
            next_cursor: None,
        };
        gap_page.validate().unwrap();
        assert!(serde_json::to_string(&gap_page)
            .unwrap()
            .contains(r#""reason":"epoch_changed""#));

        let encoded = serde_json::to_vec(&page).unwrap();
        let decoded: EventPage<RuntimeGenerationPayload> =
            serde_json::from_slice(&encoded).unwrap();
        assert_eq!(decoded, page);
    }

    #[test]
    fn event_and_gap_epochs_generations_and_schema_are_exact() {
        let event = ControlPlaneEvent {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            schema: "syndocal.runtime.transport.event.v1".to_string(),
            stream_epoch: 8,
            generation: 41,
            payload: runtime_payload(41, true),
        };
        event.validate().unwrap();
        for invalid in [
            ControlPlaneEvent {
                stream_epoch: 0,
                ..event.clone()
            },
            ControlPlaneEvent {
                generation: 0,
                ..event.clone()
            },
            ControlPlaneEvent {
                schema: "syndocal.runtime.other.event.v1".to_string(),
                ..event.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }

        let epoch_gap = GapMarker {
            previous_stream_epoch: 8,
            previous_generation: 40,
            next_stream_epoch: 9,
            next_available_generation: 1,
            reason: GapReason::EpochChanged,
            resnapshot_required: true,
        };
        epoch_gap.validate().unwrap();
        let same_epoch_gap = GapMarker {
            previous_stream_epoch: 8,
            previous_generation: 40,
            next_stream_epoch: 8,
            next_available_generation: 42,
            reason: GapReason::SubscriberOverflow,
            resnapshot_required: true,
        };
        same_epoch_gap.validate().unwrap();

        for invalid in [
            GapMarker {
                next_stream_epoch: 8,
                ..epoch_gap.clone()
            },
            GapMarker {
                next_stream_epoch: 7,
                ..epoch_gap.clone()
            },
            GapMarker {
                next_available_generation: 2,
                ..epoch_gap.clone()
            },
            GapMarker {
                next_available_generation: 41,
                ..same_epoch_gap.clone()
            },
            GapMarker {
                next_stream_epoch: 9,
                ..same_epoch_gap.clone()
            },
            GapMarker {
                previous_generation: 0,
                ..same_epoch_gap.clone()
            },
        ] {
            assert!(invalid.validate().is_err());
        }

        let mut wrong_side: EventPage<RuntimeGenerationPayload> = EventPage {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            snapshot_fence: fence(40),
            events: Vec::new(),
            gap: Some(epoch_gap),
            next_cursor: None,
        };
        wrong_side.gap.as_mut().unwrap().previous_generation = 39;
        assert_eq!(
            wrong_side.validate(),
            Err(QueryContractValidationError::InconsistentGapMarker)
        );
    }

    #[test]
    fn catalogs_are_bounded_sorted_and_canonical() {
        let catalog = SchemaCatalog {
            protocol_version: QueryProtocolVersion::CURRENT,
            catalog_generation: 3,
            schemas: vec![QuerySchemaDescriptor {
                schema_id: "syndocal.query.timeline.response.v1".to_string(),
                version: QueryProtocolVersion::CURRENT,
                kind: QuerySchemaKind::Response,
            }],
        };
        catalog.validate().unwrap();
        let encoded = serde_json::to_vec(&catalog).unwrap();
        let decoded: SchemaCatalog = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), encoded);

        let discovery = CapabilityDiscovery {
            protocol_version: QueryProtocolVersion::CURRENT,
            discovery_generation: 4,
            snapshot_fence: fence(5),
            capabilities: vec![CapabilityDescriptor {
                capability_id: "project.read".to_string(),
                operation_ids: vec!["syndocal.query.project.v1".to_string()],
                allowed_during_full_lock: true,
            }],
        };
        discovery.validate().unwrap();
        let encoded = serde_json::to_vec(&discovery).unwrap();
        let decoded: CapabilityDiscovery = serde_json::from_slice(&encoded).unwrap();
        decoded.validate().unwrap();
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), encoded);
    }

    #[test]
    fn every_collection_bound_rejects_one_over_the_limit() {
        let page = Page {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.query.project".to_string(),
            snapshot_fence: fence(1),
            items: vec![
                QuerySchemaDescriptor {
                    schema_id: "syndocal.query.project.response.v1".to_string(),
                    version: QueryProtocolVersion::CURRENT,
                    kind: QuerySchemaKind::Response,
                };
                PAGE_MAX_LIMIT as usize + 1
            ],
            next_cursor: None,
        };
        assert_eq!(
            page.validate(),
            Err(QueryContractValidationError::TooManyPageItems)
        );

        let catalog = SchemaCatalog {
            protocol_version: QueryProtocolVersion::CURRENT,
            catalog_generation: 1,
            schemas: (0..=MAX_SCHEMA_CATALOG_ENTRIES)
                .map(|index| QuerySchemaDescriptor {
                    schema_id: format!("syndocal.schema.s{index:03}.v1"),
                    version: QueryProtocolVersion::CURRENT,
                    kind: QuerySchemaKind::Resource,
                })
                .collect(),
        };
        assert_eq!(
            catalog.validate(),
            Err(QueryContractValidationError::TooManySchemaEntries)
        );

        let descriptor = CapabilityDescriptor {
            capability_id: "project.read".to_string(),
            operation_ids: (0..=MAX_OPERATION_IDS_PER_CAPABILITY)
                .map(|index| format!("syndocal.query.operation.s{index:03}.v1"))
                .collect(),
            allowed_during_full_lock: true,
        };
        assert_eq!(
            descriptor.validate(),
            Err(QueryContractValidationError::TooManyCapabilityOperations)
        );

        let event_page = EventPage {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.runtime.transport".to_string(),
            snapshot_fence: fence(0),
            events: (1..=MAX_EVENT_PAGE_EVENTS + 1)
                .map(|generation| ControlPlaneEvent {
                    protocol_version: QueryProtocolVersion::CURRENT,
                    resource: "syndocal.runtime.transport".to_string(),
                    schema: "syndocal.runtime.transport.event.v1".to_string(),
                    stream_epoch: 8,
                    generation: generation as u64,
                    payload: runtime_payload(generation as u64, false),
                })
                .collect(),
            gap: None,
            next_cursor: None,
        };
        assert_eq!(
            event_page.validate(),
            Err(QueryContractValidationError::TooManyEvents)
        );
    }

    #[test]
    fn event_and_gap_wire_shapes_reject_unknowns_and_unknown_enums() {
        assert!(
            serde_json::from_value::<ControlPlaneEvent<RuntimeGenerationPayload>>(json!({
                "protocol_version": {"major": 1, "minor": 0},
                "resource": "syndocal.runtime.transport",
                "schema": "syndocal.runtime.transport.event.v1",
                "stream_epoch": 8,
                "generation": 1,
                "payload": {"domain": "audio.transport", "generation": 1, "active": true},
                "unexpected": true
            }))
            .is_err()
        );
        assert!(serde_json::from_value::<GapMarker>(json!({
            "previous_stream_epoch": 8,
            "previous_generation": 40,
            "next_stream_epoch": 9,
            "next_available_generation": 1,
            "reason": "invented",
            "resnapshot_required": true
        }))
        .is_err());
    }

    #[test]
    fn closed_page_payload_roundtrips_without_an_open_json_escape_hatch() {
        let page = Page {
            protocol_version: QueryProtocolVersion::CURRENT,
            resource: "syndocal.query.project".to_string(),
            snapshot_fence: fence(1),
            items: vec![QuerySchemaDescriptor {
                schema_id: "syndocal.query.project.response.v1".to_string(),
                version: QueryProtocolVersion::CURRENT,
                kind: QuerySchemaKind::Response,
            }],
            next_cursor: Some(OpaqueCursorToken::try_new("opaque.v1.ABC_123").unwrap()),
        };
        page.validate().unwrap();
        let bytes = serde_json::to_vec(&page).unwrap();
        let restored: Page<QuerySchemaDescriptor> = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(restored, page);
        assert_eq!(serde_json::to_vec(&restored).unwrap(), bytes);
    }

    #[test]
    fn semantic_operation_and_schema_ids_require_canonical_versions() {
        let unversioned_schema = QuerySchemaDescriptor {
            schema_id: "syndocal.query.project.response".to_string(),
            version: QueryProtocolVersion::CURRENT,
            kind: QuerySchemaKind::Response,
        };
        assert_eq!(
            unversioned_schema.validate(),
            Err(QueryContractValidationError::UnversionedIdentifier)
        );
        let unversioned_operation = CapabilityDescriptor {
            capability_id: "project.read".to_string(),
            operation_ids: vec!["syndocal.query.project".to_string()],
            allowed_during_full_lock: true,
        };
        assert_eq!(
            unversioned_operation.validate(),
            Err(QueryContractValidationError::UnversionedIdentifier)
        );
        let noncanonical_operation = CapabilityDescriptor {
            capability_id: "project.read".to_string(),
            operation_ids: vec!["syndocal.query.project.v01".to_string()],
            allowed_during_full_lock: true,
        };
        assert_eq!(
            noncanonical_operation.validate(),
            Err(QueryContractValidationError::UnversionedIdentifier)
        );
    }
}
