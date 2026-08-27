//! Strict, transport-neutral authored control-plane command contracts.
//!
//! These DTOs deliberately describe only the first versioned authored command
//! vertical.  They do not contain a window label, renderer owner, principal,
//! credential, path, project snapshot, or arbitrary JSON payload: those facts
//! are bound by the local server adapter.

use std::fmt;

use serde::{
    de::Error as _, ser::SerializeStruct, Deserialize, Deserializer, Serialize, Serializer,
};

use crate::EffectId;

/// The largest integer which can make a round trip through JavaScript JSON
/// without losing its identity.
pub const MAX_SAFE_JAVASCRIPT_INTEGER: u64 = 9_007_199_254_740_991;
pub const SET_EFFECT_ENABLED_OPERATION_ID: &str = "syndocal.effects.set_enabled.v1";
pub const CUE_LIST_REORDER_OPERATION_ID: &str = "syndocal.cue_lists.reorder.v1";
pub const CUE_LIST_RENAME_OPERATION_ID: &str = "syndocal.cue_lists.rename.v1";
pub const CUE_LIST_DELETE_OPERATION_ID: &str = "syndocal.cue_lists.delete.v1";
/// The one server-authoritative semantic identity for both Empty Scene and
/// Capture-current Scene creation. They are request modes, not distinct
/// operations, so a caller cannot bypass the same admission/receipt path.
pub const SCENE_CREATE_AUTHORITATIVE_V1_OPERATION_ID: &str = "syndocal.scenes.create.v1";
pub const SET_EFFECT_ENABLED_SHAPE_DOMAIN_V1: &[u8] =
    b"syndocal.authored-control-plane.set-effect-enabled.shape.v1\0";

mod sealed {
    pub trait Payload {}
}

/// Closed payload set accepted by [`AuthoredRequestV1`].  Keeping this sealed
/// prevents a downstream adapter from smuggling arbitrary JSON or a secret
/// claim through an otherwise versioned request envelope.
pub trait AuthoredPayloadV1: sealed::Payload + Clone + PartialEq + Eq + Serialize {
    const OPERATION_ID: &'static str;

    fn validate_payload(&self) -> Result<(), AuthoredControlPlaneValidationError>;
    fn append_canonical_payload_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), AuthoredControlPlaneValidationError>;
}

/// Exact project image required before an authored mutation can be admitted.
/// Process/session values come from the local query adapter, never the
/// renderer's own clock or an external identity claim.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ProjectMutationFenceV1 {
    pub process_incarnation: u64,
    pub session_incarnation: u64,
    pub project_epoch: u64,
    pub project_revision: u64,
    pub project_checkpoint_hash: String,
    pub project_publication_generation: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ProjectMutationFenceV1Wire {
    process_incarnation: u64,
    session_incarnation: u64,
    project_epoch: u64,
    project_revision: u64,
    project_checkpoint_hash: String,
    project_publication_generation: u64,
}

impl ProjectMutationFenceV1 {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.process_incarnation == 0 || self.session_incarnation == 0 {
            return Err(AuthoredControlPlaneValidationError::ZeroIncarnation);
        }
        for value in [
            self.process_incarnation,
            self.session_incarnation,
            self.project_epoch,
            self.project_revision,
            self.project_publication_generation,
        ] {
            if value > MAX_SAFE_JAVASCRIPT_INTEGER {
                return Err(AuthoredControlPlaneValidationError::InvalidJavaScriptSafeInteger);
            }
        }
        validate_lower_hex_sha256(&self.project_checkpoint_hash)
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), AuthoredControlPlaneValidationError> {
        self.validate()?;
        append_u64(output, self.process_incarnation);
        append_u64(output, self.session_incarnation);
        append_u64(output, self.project_epoch);
        append_u64(output, self.project_revision);
        append_ascii(output, &self.project_checkpoint_hash)?;
        append_u64(output, self.project_publication_generation);
        Ok(())
    }
}

impl Serialize for ProjectMutationFenceV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("ProjectMutationFenceV1", 6)?;
        state.serialize_field("process_incarnation", &self.process_incarnation)?;
        state.serialize_field("session_incarnation", &self.session_incarnation)?;
        state.serialize_field("project_epoch", &self.project_epoch)?;
        state.serialize_field("project_revision", &self.project_revision)?;
        state.serialize_field("project_checkpoint_hash", &self.project_checkpoint_hash)?;
        state.serialize_field(
            "project_publication_generation",
            &self.project_publication_generation,
        )?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for ProjectMutationFenceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProjectMutationFenceV1Wire::deserialize(deserializer)?;
        let fence = Self {
            process_incarnation: wire.process_incarnation,
            session_incarnation: wire.session_incarnation,
            project_epoch: wire.project_epoch,
            project_revision: wire.project_revision,
            project_checkpoint_hash: wire.project_checkpoint_hash,
            project_publication_generation: wire.project_publication_generation,
        };
        fence.validate().map_err(D::Error::custom)?;
        Ok(fence)
    }
}

/// The complete, typed intent for the first authored command vertical.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEffectEnabledPayload {
    pub effect_id: EffectId,
    pub enabled: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetEffectEnabledPayloadWire {
    effect_id: EffectId,
    enabled: bool,
}

impl SetEffectEnabledPayload {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.effect_id == 0 {
            return Err(AuthoredControlPlaneValidationError::ZeroEffectId);
        }
        if self.effect_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(AuthoredControlPlaneValidationError::InvalidJavaScriptSafeInteger);
        }
        Ok(())
    }
}

impl sealed::Payload for SetEffectEnabledPayload {}

impl AuthoredPayloadV1 for SetEffectEnabledPayload {
    const OPERATION_ID: &'static str = SET_EFFECT_ENABLED_OPERATION_ID;

    fn validate_payload(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        self.validate()
    }

    fn append_canonical_payload_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), AuthoredControlPlaneValidationError> {
        self.validate()?;
        append_u64(output, self.effect_id);
        output.push(u8::from(self.enabled));
        Ok(())
    }
}

impl Serialize for SetEffectEnabledPayload {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SetEffectEnabledPayload", 2)?;
        state.serialize_field("effect_id", &self.effect_id)?;
        state.serialize_field("enabled", &self.enabled)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SetEffectEnabledPayload {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SetEffectEnabledPayloadWire::deserialize(deserializer)?;
        let payload = Self {
            effect_id: wire.effect_id,
            enabled: wire.enabled,
        };
        payload.validate().map_err(D::Error::custom)?;
        Ok(payload)
    }
}

/// A versioned authored request.  The key fields used by server-side receipt
/// retention are deliberately distinct from this semantic shape: retry
/// identity includes caller binding and start fence, while this object records
/// every typed client-visible request field which may not silently change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredRequestV1<P> {
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: ProjectMutationFenceV1,
    pub payload: P,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredRequestV1Wire<P> {
    operation_id: String,
    request_id: u64,
    expected_fence: ProjectMutationFenceV1,
    payload: P,
}

impl<P: AuthoredPayloadV1> AuthoredRequestV1<P> {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.operation_id != P::OPERATION_ID {
            return Err(AuthoredControlPlaneValidationError::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(AuthoredControlPlaneValidationError::InvalidRequestId);
        }
        self.expected_fence.validate()?;
        self.payload.validate_payload()
    }

    /// Deterministic typed bytes for a shape hash.  This intentionally does
    /// not serialize JSON: object-field order, whitespace and unknown JSON
    /// extensions must never define idempotency semantics.
    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, AuthoredControlPlaneValidationError> {
        self.validate()?;
        let mut output = Vec::with_capacity(160);
        append_ascii(&mut output, "authored_request_v1")?;
        append_ascii(&mut output, &self.operation_id)?;
        append_u64(&mut output, self.request_id);
        self.expected_fence.append_canonical_bytes(&mut output)?;
        self.payload.append_canonical_payload_bytes(&mut output)?;
        Ok(output)
    }
}

impl<P: AuthoredPayloadV1> Serialize for AuthoredRequestV1<P> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("AuthoredRequestV1", 4)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("expected_fence", &self.expected_fence)?;
        state.serialize_field("payload", &self.payload)?;
        state.end()
    }
}

impl<'de, P> Deserialize<'de> for AuthoredRequestV1<P>
where
    P: AuthoredPayloadV1 + Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AuthoredRequestV1Wire::<P>::deserialize(deserializer)?;
        let request = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
            payload: wire.payload,
        };
        request.validate().map_err(D::Error::custom)?;
        Ok(request)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoredCommandErrorCodeV1 {
    InvalidRequest,
    Forbidden,
    StaleFence,
    Conflict,
    Busy,
    Overloaded,
    NotFound,
    PublicationFailed,
    Internal,
}

/// Fixed-code failure data.  It intentionally has no free-form message field:
/// machine-local engine details and credential material never cross this DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredCommandErrorV1 {
    pub code: AuthoredCommandErrorCodeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredCommandErrorV1Wire {
    code: AuthoredCommandErrorCodeV1,
}

impl AuthoredCommandErrorV1 {
    pub const fn new(code: AuthoredCommandErrorCodeV1) -> Self {
        Self { code }
    }
}

impl Serialize for AuthoredCommandErrorV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("AuthoredCommandErrorV1", 1)?;
        state.serialize_field("code", &self.code)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AuthoredCommandErrorV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AuthoredCommandErrorV1Wire::deserialize(deserializer)?;
        Ok(Self { code: wire.code })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEffectEnabledAppliedV1 {
    pub effect_id: EffectId,
    pub enabled: bool,
    pub post_fence: ProjectMutationFenceV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetEffectEnabledAppliedV1Wire {
    effect_id: EffectId,
    enabled: bool,
    post_fence: ProjectMutationFenceV1,
}

impl SetEffectEnabledAppliedV1 {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.effect_id == 0 {
            return Err(AuthoredControlPlaneValidationError::ZeroEffectId);
        }
        if self.effect_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(AuthoredControlPlaneValidationError::InvalidJavaScriptSafeInteger);
        }
        self.post_fence.validate()
    }
}

impl Serialize for SetEffectEnabledAppliedV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SetEffectEnabledAppliedV1", 3)?;
        state.serialize_field("effect_id", &self.effect_id)?;
        state.serialize_field("enabled", &self.enabled)?;
        state.serialize_field("post_fence", &self.post_fence)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SetEffectEnabledAppliedV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SetEffectEnabledAppliedV1Wire::deserialize(deserializer)?;
        let applied = Self {
            effect_id: wire.effect_id,
            enabled: wire.enabled,
            post_fence: wire.post_fence,
        };
        applied.validate().map_err(D::Error::custom)?;
        Ok(applied)
    }
}

/// A terminal command outcome.  `NoOp` is still a terminal receipt but its
/// post-fence is exactly the start fence, proving it did not create a revision,
/// publication or Undo/Redo drift.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SetEffectEnabledOutcomeV1 {
    Applied(SetEffectEnabledAppliedV1),
    NoOp(SetEffectEnabledAppliedV1),
    Failed(AuthoredCommandErrorV1),
}

impl SetEffectEnabledOutcomeV1 {
    fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        match self {
            Self::Applied(applied) | Self::NoOp(applied) => applied.validate(),
            Self::Failed(_) => Ok(()),
        }
    }
}

/// Retained terminal receipt returned by an exact reply-loss retry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetEffectEnabledTerminalReceiptV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub start_fence: ProjectMutationFenceV1,
    pub shape_sha256: String,
    pub outcome: SetEffectEnabledOutcomeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetEffectEnabledTerminalReceiptV1Wire {
    operation_id: String,
    request_id: u64,
    start_fence: ProjectMutationFenceV1,
    shape_sha256: String,
    outcome: SetEffectEnabledOutcomeV1,
}

impl SetEffectEnabledTerminalReceiptV1 {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.operation_id != SET_EFFECT_ENABLED_OPERATION_ID {
            return Err(AuthoredControlPlaneValidationError::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(AuthoredControlPlaneValidationError::InvalidRequestId);
        }
        self.start_fence.validate()?;
        validate_lower_hex_sha256(&self.shape_sha256)?;
        self.outcome.validate()?;
        if let SetEffectEnabledOutcomeV1::NoOp(applied) = &self.outcome {
            if applied.post_fence != self.start_fence {
                return Err(AuthoredControlPlaneValidationError::InvalidNoOpFence);
            }
        }
        Ok(())
    }
}

impl Serialize for SetEffectEnabledTerminalReceiptV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SetEffectEnabledTerminalReceiptV1", 5)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("start_fence", &self.start_fence)?;
        state.serialize_field("shape_sha256", &self.shape_sha256)?;
        state.serialize_field("outcome", &self.outcome)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SetEffectEnabledTerminalReceiptV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SetEffectEnabledTerminalReceiptV1Wire::deserialize(deserializer)?;
        let receipt = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            start_fence: wire.start_fence,
            shape_sha256: wire.shape_sha256,
            outcome: wire.outcome,
        };
        receipt.validate().map_err(D::Error::custom)?;
        Ok(receipt)
    }
}

/// A different payload under an already-used receipt key is rejected without
/// exposing the old receipt's payload.  This is deliberately separate from a
/// terminal receipt because it did not execute this request shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthoredCommandRejectionV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub start_fence: ProjectMutationFenceV1,
    pub error: AuthoredCommandErrorV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AuthoredCommandRejectionV1Wire {
    operation_id: String,
    request_id: u64,
    start_fence: ProjectMutationFenceV1,
    error: AuthoredCommandErrorV1,
}

impl AuthoredCommandRejectionV1 {
    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        if self.operation_id != SET_EFFECT_ENABLED_OPERATION_ID {
            return Err(AuthoredControlPlaneValidationError::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(AuthoredControlPlaneValidationError::InvalidRequestId);
        }
        self.start_fence.validate()
    }
}

impl Serialize for AuthoredCommandRejectionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("AuthoredCommandRejectionV1", 4)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("start_fence", &self.start_fence)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for AuthoredCommandRejectionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = AuthoredCommandRejectionV1Wire::deserialize(deserializer)?;
        let rejection = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            start_fence: wire.start_fence,
            error: wire.error,
        };
        rejection.validate().map_err(D::Error::custom)?;
        Ok(rejection)
    }
}

/// Typed command response.  The normal path always has an immutable terminal
/// receipt; only a shape conflict is represented as a non-terminal rejection.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SetEffectEnabledResponseV1 {
    TerminalReceipt(SetEffectEnabledTerminalReceiptV1),
    Rejected(AuthoredCommandRejectionV1),
}

impl SetEffectEnabledResponseV1 {
    pub fn terminal_receipt(&self) -> Option<&SetEffectEnabledTerminalReceiptV1> {
        match self {
            Self::TerminalReceipt(receipt) => Some(receipt),
            Self::Rejected(_) => None,
        }
    }

    pub fn validate(&self) -> Result<(), AuthoredControlPlaneValidationError> {
        match self {
            Self::TerminalReceipt(receipt) => receipt.validate(),
            Self::Rejected(rejection) => rejection.validate(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoredControlPlaneValidationError {
    ZeroIncarnation,
    InvalidJavaScriptSafeInteger,
    InvalidCheckpointHash,
    ZeroEffectId,
    UnexpectedOperationId,
    InvalidRequestId,
    InvalidCanonicalString,
    InvalidNoOpFence,
}

impl fmt::Display for AuthoredControlPlaneValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroIncarnation => {
                formatter.write_str("process and session incarnations must be non-zero")
            }
            Self::InvalidJavaScriptSafeInteger => formatter.write_str(
                "client-visible numeric identities and counters must be JavaScript-safe",
            ),
            Self::InvalidCheckpointHash => {
                formatter.write_str("checkpoint hash must be 64 lower-hex bytes")
            }
            Self::ZeroEffectId => formatter.write_str("effect ID must be non-zero"),
            Self::UnexpectedOperationId => formatter.write_str("unexpected authored operation ID"),
            Self::InvalidRequestId => {
                formatter.write_str("request ID must be JavaScript-safe and non-zero")
            }
            Self::InvalidCanonicalString => formatter.write_str("canonical string is invalid"),
            Self::InvalidNoOpFence => formatter.write_str("NoOp receipt changed its project fence"),
        }
    }
}

impl std::error::Error for AuthoredControlPlaneValidationError {}

fn append_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_be_bytes());
}

fn append_ascii(
    output: &mut Vec<u8>,
    value: &str,
) -> Result<(), AuthoredControlPlaneValidationError> {
    if value.len() > u32::MAX as usize || !value.is_ascii() {
        return Err(AuthoredControlPlaneValidationError::InvalidCanonicalString);
    }
    output.extend_from_slice(&(value.len() as u32).to_be_bytes());
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn validate_lower_hex_sha256(value: &str) -> Result<(), AuthoredControlPlaneValidationError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(AuthoredControlPlaneValidationError::InvalidCheckpointHash)
    }
}

// ---------------------------------------------------------------------------
// Runtime-only Timeline transport vertical (AI3).
//
// This is deliberately separate from the authored effect mutation above.  A
// Timeline Play/Pause changes live transport state, never project history, and
// its stale fence includes the engine-owned transport generation in addition
// to the redacted project E/R/H publication fence.

pub const TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID: &str =
    "syndocal.runtime.timeline.transport.set_playing.v1";
/// Discovery identity for the narrow, owner-bound authority bundle. Its
/// response names the target mutation separately, so a caller cannot confuse
/// query permission with execution permission.
pub const TIMELINE_TRANSPORT_AUTHORITY_QUERY_OPERATION_ID: &str =
    "syndocal.query.runtime.timeline.transport.authority.v1";
pub const TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1: &str = "timeline.transport";
pub const TIMELINE_TRANSPORT_SET_PLAYING_SHAPE_DOMAIN_V1: &[u8] =
    b"syndocal.runtime-control-plane.timeline-transport.set-playing.shape.v1\0";
/// An opaque authority is exactly 128 random bits, encoded as canonical
/// unpadded base64url. It deliberately is not an identity claim: the native
/// adapter retains the owner/window binding server-side.
pub const TIMELINE_TRANSPORT_AUTHORITY_ID_BYTES: usize = 16;

/// The complete server-issued capability required by the local Timeline
/// transport command.  It intentionally contains no renderer principal,
/// window label, owner incarnation, path or credential: those are retained by
/// the local adapter and bound into its receipt key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimelineTransportRuntimeFenceV1 {
    pub project: ProjectMutationFenceV1,
    pub domain: String,
    pub source_runtime_epoch: u64,
    pub source_runtime_generation: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineTransportRuntimeFenceV1Wire {
    project: ProjectMutationFenceV1,
    domain: String,
    source_runtime_epoch: u64,
    source_runtime_generation: u64,
}

impl TimelineTransportRuntimeFenceV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        self.project
            .validate()
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        if self.domain != TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1 {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedRuntimeDomain);
        }
        if self.source_runtime_epoch == 0
            || self.source_runtime_epoch > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.source_runtime_generation == 0
            || self.source_runtime_generation > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(RuntimeCommandValidationErrorV1::InvalidJavaScriptSafeInteger);
        }
        Ok(())
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), RuntimeCommandValidationErrorV1> {
        self.validate()?;
        self.project
            .append_canonical_bytes(output)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(output, &self.domain)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_u64(output, self.source_runtime_epoch);
        append_u64(output, self.source_runtime_generation);
        Ok(())
    }
}

impl Serialize for TimelineTransportRuntimeFenceV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineTransportRuntimeFenceV1", 4)?;
        state.serialize_field("project", &self.project)?;
        state.serialize_field("domain", &self.domain)?;
        state.serialize_field("source_runtime_epoch", &self.source_runtime_epoch)?;
        state.serialize_field("source_runtime_generation", &self.source_runtime_generation)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineTransportRuntimeFenceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineTransportRuntimeFenceV1Wire::deserialize(deserializer)?;
        let value = Self {
            project: wire.project,
            domain: wire.domain,
            source_runtime_epoch: wire.source_runtime_epoch,
            source_runtime_generation: wire.source_runtime_generation,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

/// Redacted, server-minted authority bundle for exactly one local runtime
/// command. The owner/window binding and single-use record remain entirely in
/// the native adapter; the renderer sees only the project fence and the exact
/// engine-owned Timeline transport generation it must echo.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandAuthorityBundleV1 {
    pub operation_id: String,
    pub authority_id: String,
    pub fence: TimelineTransportRuntimeFenceV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeCommandAuthorityBundleV1Wire {
    operation_id: String,
    authority_id: String,
    fence: TimelineTransportRuntimeFenceV1,
}

impl RuntimeCommandAuthorityBundleV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        validate_runtime_authority_id(&self.authority_id)?;
        self.fence.validate()
    }
}

impl Serialize for RuntimeCommandAuthorityBundleV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("RuntimeCommandAuthorityBundleV1", 3)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("authority_id", &self.authority_id)?;
        state.serialize_field("fence", &self.fence)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuntimeCommandAuthorityBundleV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeCommandAuthorityBundleV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            authority_id: wire.authority_id,
            fence: wire.fence,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetTimelinePlayingRuntimePayloadV1 {
    pub playing: bool,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SetTimelinePlayingRuntimePayloadV1Wire {
    playing: bool,
}

impl SetTimelinePlayingRuntimePayloadV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        Ok(())
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), RuntimeCommandValidationErrorV1> {
        self.validate()?;
        output.push(u8::from(self.playing));
        Ok(())
    }
}

impl Serialize for SetTimelinePlayingRuntimePayloadV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SetTimelinePlayingRuntimePayloadV1", 1)?;
        state.serialize_field("playing", &self.playing)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SetTimelinePlayingRuntimePayloadV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SetTimelinePlayingRuntimePayloadV1Wire::deserialize(deserializer)?;
        let value = Self {
            playing: wire.playing,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

/// Strict, versioned request for the runtime-only Timeline transport lane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandRequestV1 {
    pub operation_id: String,
    pub authority_id: String,
    pub request_id: u64,
    pub expected_fence: TimelineTransportRuntimeFenceV1,
    pub payload: SetTimelinePlayingRuntimePayloadV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeCommandRequestV1Wire {
    operation_id: String,
    authority_id: String,
    request_id: u64,
    expected_fence: TimelineTransportRuntimeFenceV1,
    payload: SetTimelinePlayingRuntimePayloadV1,
}

impl RuntimeCommandRequestV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        validate_runtime_authority_id(&self.authority_id)?;
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.expected_fence.validate()?;
        self.payload.validate()
    }

    /// Stable typed bytes for server-side shape hashing. JSON object ordering,
    /// whitespace and extensions cannot influence idempotency.
    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, RuntimeCommandValidationErrorV1> {
        self.validate()?;
        let mut output = Vec::with_capacity(192);
        append_ascii(&mut output, "runtime_command_request_v1")
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(&mut output, &self.operation_id)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(&mut output, &self.authority_id)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_u64(&mut output, self.request_id);
        self.expected_fence.append_canonical_bytes(&mut output)?;
        self.payload.append_canonical_bytes(&mut output)?;
        Ok(output)
    }
}

impl Serialize for RuntimeCommandRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("RuntimeCommandRequestV1", 5)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("authority_id", &self.authority_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("expected_fence", &self.expected_fence)?;
        state.serialize_field("payload", &self.payload)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuntimeCommandRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeCommandRequestV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            authority_id: wire.authority_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
            payload: wire.payload,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCommandErrorCodeV1 {
    InvalidRequest,
    Forbidden,
    StaleFence,
    Conflict,
    Busy,
    Overloaded,
    PublicationFailed,
    Internal,
}

/// Fixed-code runtime failure data. No adapter or worker error string crosses
/// this DTO.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandErrorV1 {
    pub code: RuntimeCommandErrorCodeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeCommandErrorV1Wire {
    code: RuntimeCommandErrorCodeV1,
}

impl RuntimeCommandErrorV1 {
    pub const fn new(code: RuntimeCommandErrorCodeV1) -> Self {
        Self { code }
    }
}

impl Serialize for RuntimeCommandErrorV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("RuntimeCommandErrorV1", 1)?;
        state.serialize_field("code", &self.code)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuntimeCommandErrorV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeCommandErrorV1Wire::deserialize(deserializer)?;
        Ok(Self { code: wire.code })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeCommandReceiptOutcomeV1 {
    Applied,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandReceiptV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub fence_before: TimelineTransportRuntimeFenceV1,
    pub requested_playing: bool,
    pub epoch_after: u64,
    pub generation_after: u64,
    pub shape_sha256: String,
    pub outcome: RuntimeCommandReceiptOutcomeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeCommandReceiptV1Wire {
    operation_id: String,
    request_id: u64,
    fence_before: TimelineTransportRuntimeFenceV1,
    requested_playing: bool,
    epoch_after: u64,
    generation_after: u64,
    shape_sha256: String,
    outcome: RuntimeCommandReceiptOutcomeV1,
}

impl RuntimeCommandReceiptV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.fence_before.validate()?;
        if self.epoch_after == 0
            || self.epoch_after > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.generation_after == 0
            || self.generation_after > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(RuntimeCommandValidationErrorV1::InvalidJavaScriptSafeInteger);
        }
        validate_lower_hex_sha256(&self.shape_sha256)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        match self.outcome {
            RuntimeCommandReceiptOutcomeV1::NoOp
                if self.epoch_after != self.fence_before.source_runtime_epoch
                    || self.generation_after != self.fence_before.source_runtime_generation =>
            {
                return Err(RuntimeCommandValidationErrorV1::InvalidNoOpAuthority);
            }
            RuntimeCommandReceiptOutcomeV1::Applied
                if next_timeline_transport_authority(
                    self.fence_before.source_runtime_epoch,
                    self.fence_before.source_runtime_generation,
                ) != Some((self.epoch_after, self.generation_after)) =>
            {
                return Err(RuntimeCommandValidationErrorV1::InvalidAppliedAuthority);
            }
            _ => {}
        }
        Ok(())
    }
}

impl Serialize for RuntimeCommandReceiptV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("RuntimeCommandReceiptV1", 8)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("fence_before", &self.fence_before)?;
        state.serialize_field("requested_playing", &self.requested_playing)?;
        state.serialize_field("epoch_after", &self.epoch_after)?;
        state.serialize_field("generation_after", &self.generation_after)?;
        state.serialize_field("shape_sha256", &self.shape_sha256)?;
        state.serialize_field("outcome", &self.outcome)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuntimeCommandReceiptV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeCommandReceiptV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            fence_before: wire.fence_before,
            requested_playing: wire.requested_playing,
            epoch_after: wire.epoch_after,
            generation_after: wire.generation_after,
            shape_sha256: wire.shape_sha256,
            outcome: wire.outcome,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeCommandRejectionV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub fence_before: TimelineTransportRuntimeFenceV1,
    pub error: RuntimeCommandErrorV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuntimeCommandRejectionV1Wire {
    operation_id: String,
    request_id: u64,
    fence_before: TimelineTransportRuntimeFenceV1,
    error: RuntimeCommandErrorV1,
}

impl RuntimeCommandRejectionV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.fence_before.validate()
    }
}

impl Serialize for RuntimeCommandRejectionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("RuntimeCommandRejectionV1", 4)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("fence_before", &self.fence_before)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for RuntimeCommandRejectionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = RuntimeCommandRejectionV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            fence_before: wire.fence_before,
            error: wire.error,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum RuntimeCommandResponseV1 {
    Receipt(RuntimeCommandReceiptV1),
    Rejected(RuntimeCommandRejectionV1),
}

impl RuntimeCommandResponseV1 {
    pub fn receipt(&self) -> Option<&RuntimeCommandReceiptV1> {
        match self {
            Self::Receipt(receipt) => Some(receipt),
            Self::Rejected(_) => None,
        }
    }

    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        match self {
            Self::Receipt(receipt) => receipt.validate(),
            Self::Rejected(rejection) => rejection.validate(),
        }
    }
}

pub const TIMELINE_FOLLOW_ABORT_OPERATION_ID: &str = "syndocal.runtime.timeline.follow.abort.v1";
pub const TIMELINE_FOLLOW_ABORT_AUTHORITY_QUERY_OPERATION_ID: &str =
    "syndocal.query.runtime.timeline.follow.abort.authority.v1";
pub const TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1: &str = "timeline.follow.abort";
pub const TIMELINE_FOLLOW_ABORT_SHAPE_DOMAIN_V1: &[u8] =
    b"syndocal.runtime-control-plane.timeline-follow.abort.shape.v1\0";

/// Exact project/output/Follow identity captured by one server-issued abort
/// capability. The output epoch and Follow generation form one ABA fence; no
/// window, principal, credential, path or authored Timeline payload crosses
/// this wire contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TimelineFollowAbortRuntimeFenceV1 {
    pub project: ProjectMutationFenceV1,
    pub domain: String,
    pub output_ownership_epoch: u64,
    pub follow_generation: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineFollowAbortRuntimeFenceV1Wire {
    project: ProjectMutationFenceV1,
    domain: String,
    output_ownership_epoch: u64,
    follow_generation: u64,
}

impl TimelineFollowAbortRuntimeFenceV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        self.project
            .validate()
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        if self.domain != TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1 {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedRuntimeDomain);
        }
        if self.output_ownership_epoch == 0
            || self.output_ownership_epoch > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.follow_generation == 0
            || self.follow_generation > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(RuntimeCommandValidationErrorV1::InvalidJavaScriptSafeInteger);
        }
        Ok(())
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), RuntimeCommandValidationErrorV1> {
        self.validate()?;
        self.project
            .append_canonical_bytes(output)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(output, &self.domain)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_u64(output, self.output_ownership_epoch);
        append_u64(output, self.follow_generation);
        Ok(())
    }
}

impl Serialize for TimelineFollowAbortRuntimeFenceV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineFollowAbortRuntimeFenceV1", 4)?;
        state.serialize_field("project", &self.project)?;
        state.serialize_field("domain", &self.domain)?;
        state.serialize_field("output_ownership_epoch", &self.output_ownership_epoch)?;
        state.serialize_field("follow_generation", &self.follow_generation)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineFollowAbortRuntimeFenceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineFollowAbortRuntimeFenceV1Wire::deserialize(deserializer)?;
        let value = Self {
            project: wire.project,
            domain: wire.domain,
            output_ownership_epoch: wire.output_ownership_epoch,
            follow_generation: wire.follow_generation,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineFollowAbortAuthorityBundleV1 {
    pub operation_id: String,
    pub authority_id: String,
    pub fence: TimelineFollowAbortRuntimeFenceV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineFollowAbortAuthorityBundleV1Wire {
    operation_id: String,
    authority_id: String,
    fence: TimelineFollowAbortRuntimeFenceV1,
}

impl TimelineFollowAbortAuthorityBundleV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_FOLLOW_ABORT_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        validate_runtime_authority_id(&self.authority_id)?;
        self.fence.validate()
    }
}

impl Serialize for TimelineFollowAbortAuthorityBundleV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineFollowAbortAuthorityBundleV1", 3)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("authority_id", &self.authority_id)?;
        state.serialize_field("fence", &self.fence)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineFollowAbortAuthorityBundleV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineFollowAbortAuthorityBundleV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            authority_id: wire.authority_id,
            fence: wire.fence,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineFollowAbortRuntimeRequestV1 {
    pub operation_id: String,
    pub authority_id: String,
    pub request_id: u64,
    pub expected_fence: TimelineFollowAbortRuntimeFenceV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineFollowAbortRuntimeRequestV1Wire {
    operation_id: String,
    authority_id: String,
    request_id: u64,
    expected_fence: TimelineFollowAbortRuntimeFenceV1,
}

impl TimelineFollowAbortRuntimeRequestV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_FOLLOW_ABORT_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        validate_runtime_authority_id(&self.authority_id)?;
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.expected_fence.validate()
    }

    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, RuntimeCommandValidationErrorV1> {
        self.validate()?;
        let mut output = Vec::with_capacity(192);
        append_ascii(&mut output, "timeline_follow_abort_runtime_request_v1")
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(&mut output, &self.operation_id)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(&mut output, &self.authority_id)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_u64(&mut output, self.request_id);
        self.expected_fence.append_canonical_bytes(&mut output)?;
        Ok(output)
    }
}

impl Serialize for TimelineFollowAbortRuntimeRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineFollowAbortRuntimeRequestV1", 4)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("authority_id", &self.authority_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("expected_fence", &self.expected_fence)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineFollowAbortRuntimeRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineFollowAbortRuntimeRequestV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            authority_id: wire.authority_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineFollowAbortRuntimeReceiptV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub fence_before: TimelineFollowAbortRuntimeFenceV1,
    pub output_ownership_epoch_after: u64,
    pub follow_generation_after: u64,
    pub shape_sha256: String,
    pub outcome: RuntimeCommandReceiptOutcomeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineFollowAbortRuntimeReceiptV1Wire {
    operation_id: String,
    request_id: u64,
    fence_before: TimelineFollowAbortRuntimeFenceV1,
    output_ownership_epoch_after: u64,
    follow_generation_after: u64,
    shape_sha256: String,
    outcome: RuntimeCommandReceiptOutcomeV1,
}

impl TimelineFollowAbortRuntimeReceiptV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_FOLLOW_ABORT_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.fence_before.validate()?;
        if self.output_ownership_epoch_after == 0
            || self.output_ownership_epoch_after > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.follow_generation_after == 0
            || self.follow_generation_after > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(RuntimeCommandValidationErrorV1::InvalidJavaScriptSafeInteger);
        }
        validate_lower_hex_sha256(&self.shape_sha256)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        let same_authority = self.output_ownership_epoch_after
            == self.fence_before.output_ownership_epoch
            && self.follow_generation_after == self.fence_before.follow_generation;
        match self.outcome {
            RuntimeCommandReceiptOutcomeV1::NoOp if !same_authority => {
                Err(RuntimeCommandValidationErrorV1::InvalidNoOpAuthority)
            }
            RuntimeCommandReceiptOutcomeV1::Applied
                if self.output_ownership_epoch_after
                    != self.fence_before.output_ownership_epoch
                    || self.fence_before.follow_generation == MAX_SAFE_JAVASCRIPT_INTEGER
                    || self.follow_generation_after != self.fence_before.follow_generation + 1 =>
            {
                Err(RuntimeCommandValidationErrorV1::InvalidAppliedAuthority)
            }
            _ => Ok(()),
        }
    }
}

impl Serialize for TimelineFollowAbortRuntimeReceiptV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineFollowAbortRuntimeReceiptV1", 7)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("fence_before", &self.fence_before)?;
        state.serialize_field(
            "output_ownership_epoch_after",
            &self.output_ownership_epoch_after,
        )?;
        state.serialize_field("follow_generation_after", &self.follow_generation_after)?;
        state.serialize_field("shape_sha256", &self.shape_sha256)?;
        state.serialize_field("outcome", &self.outcome)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineFollowAbortRuntimeReceiptV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineFollowAbortRuntimeReceiptV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            fence_before: wire.fence_before,
            output_ownership_epoch_after: wire.output_ownership_epoch_after,
            follow_generation_after: wire.follow_generation_after,
            shape_sha256: wire.shape_sha256,
            outcome: wire.outcome,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimelineFollowAbortRuntimeRejectionV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub fence_before: TimelineFollowAbortRuntimeFenceV1,
    pub error: RuntimeCommandErrorV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TimelineFollowAbortRuntimeRejectionV1Wire {
    operation_id: String,
    request_id: u64,
    fence_before: TimelineFollowAbortRuntimeFenceV1,
    error: RuntimeCommandErrorV1,
}

impl TimelineFollowAbortRuntimeRejectionV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != TIMELINE_FOLLOW_ABORT_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        self.fence_before.validate()
    }
}

impl Serialize for TimelineFollowAbortRuntimeRejectionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("TimelineFollowAbortRuntimeRejectionV1", 4)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("fence_before", &self.fence_before)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for TimelineFollowAbortRuntimeRejectionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = TimelineFollowAbortRuntimeRejectionV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            fence_before: wire.fence_before,
            error: wire.error,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum TimelineFollowAbortRuntimeResponseV1 {
    Receipt(TimelineFollowAbortRuntimeReceiptV1),
    Rejected(TimelineFollowAbortRuntimeRejectionV1),
}

impl TimelineFollowAbortRuntimeResponseV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        match self {
            Self::Receipt(receipt) => receipt.validate(),
            Self::Rejected(rejection) => rejection.validate(),
        }
    }
}

pub const SAFETY_BLACKOUT_ENGAGE_OPERATION_ID: &str = "syndocal.safety.blackout.engage.v1";
pub const SAFETY_BLACKOUT_ENGAGE_SHAPE_DOMAIN_V1: &[u8] =
    b"syndocal.safety.blackout.engage.shape.v1\0";
pub const OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID: &str =
    "syndocal.query.output.control.authority.v1";
/// OutputControl v2 changed the command shape and confirmation boundary. Its
/// request/response schema identity is therefore distinct from the v1
/// inventory schema and the Rust mutation DTO names now match that boundary.
pub const OUTPUT_CONTROL_COMMAND_SCHEMA_VERSION: u16 = 2;
pub const OUTPUT_OWNERSHIP_ARM_OPERATION_ID: &str = "syndocal.output.ownership.arm.v2";
pub const OUTPUT_BLACKOUT_RELEASE_OPERATION_ID: &str = "syndocal.output.blackout.release.v2";
pub const OUTPUT_STANDBY_TAKEOVER_OPERATION_ID: &str = "syndocal.output.standby.takeover.v2";
pub const OUTPUT_LEASE_ACQUIRE_OPERATION_ID: &str = "syndocal.output.lease.acquire.v2";
pub const OUTPUT_LEASE_RENEW_OPERATION_ID: &str = "syndocal.output.lease.renew.v2";
pub const OUTPUT_LEASE_RECOVER_OPERATION_ID: &str = "syndocal.output.lease.recover.v2";
pub const OUTPUT_LEASE_RELINQUISH_OPERATION_ID: &str = "syndocal.output.lease.relinquish.v2";
pub const OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID: &str =
    "syndocal.output.lease.force_transfer.v2";
pub const OUTPUT_DISPLAY_ADD_OPERATION_ID: &str = "syndocal.output.display.add.v2";
/// The sole operator mutation for the physical live Display shell.  This is
/// deliberately separate from AddDisplay: it never changes the persisted
/// output graph, routing, mapping, enable state, or blackout state.
pub const OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID: &str =
    "syndocal.output.display.window.set_open.v2";
/// Normal operator path: one explicit local-renderer enable request. This is
/// deliberately distinct from the public lease lifecycle.
pub const OUTPUT_ENABLE_OPERATION_ID: &str = "syndocal.output.enable.v2";
pub const OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID: &str =
    "syndocal.output.lease.authority.query.v1";
pub const OUTPUT_LEASE_TTL_MS: u64 = 60_000;
pub const MAX_OUTPUT_LEASE_QUERY_STATUSES: usize = 64;
pub const OUTPUT_CONTROL_ARGUMENT_FINGERPRINT_DOMAIN_V2: &[u8] =
    b"syndocal.output-control.argument-fingerprint.v2\0";
pub const OUTPUT_CONTROL_SHAPE_DOMAIN_V2: &[u8] = b"syndocal.output-control.command-shape.v2\0";

/// Minimal, display-only payload for the local R4 output-creation lane.
///
/// The monitor index is resolved against the current native monitor snapshot
/// at both admission and final commit. Width/height stay on the wire so the
/// request fingerprint covers every output-affecting choice, but the UI may
/// simply populate them from the detected display descriptor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DisplayOutputSpecV2 {
    pub label: String,
    pub monitor_identity: String,
    pub monitor_index: u32,
    pub width: u32,
    pub height: u32,
    pub fullscreen: bool,
}

impl DisplayOutputSpecV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        let label = self.label.trim();
        if label.is_empty()
            || label.len() > 128
            || !label.is_ascii()
            || label
                .bytes()
                .any(|byte| byte.is_ascii_control() || matches!(byte, b'/' | b'\\'))
            || validate_lower_hex_sha256(&self.monitor_identity).is_err()
            || self.monitor_index > 255
            || !(1..=16_384).contains(&self.width)
            || !(1..=16_384).contains(&self.height)
        {
            return Err(OutputControlValidationErrorV1::InvalidDisplayOutputSpec);
        }
        Ok(())
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), OutputControlValidationErrorV1> {
        self.validate()?;
        append_ascii(output, self.label.trim())
            .map_err(|_| OutputControlValidationErrorV1::InvalidDisplayOutputSpec)?;
        append_ascii(output, &self.monitor_identity)
            .map_err(|_| OutputControlValidationErrorV1::InvalidDisplayOutputSpec)?;
        append_u64(output, u64::from(self.monitor_index));
        append_u64(output, u64::from(self.width));
        append_u64(output, u64::from(self.height));
        output.push(u8::from(self.fullscreen));
        Ok(())
    }
}

/// Safer-direction-only emergency request. Intentionally no `enabled`,
/// `target`, or other payload field exists on the wire.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyBlackoutEngageRequestV1 {
    pub operation_id: String,
    pub request_id: u64,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SafetyBlackoutEngageRequestV1Wire {
    operation_id: String,
    request_id: u64,
}

impl SafetyBlackoutEngageRequestV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        Ok(())
    }

    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, RuntimeCommandValidationErrorV1> {
        self.validate()?;
        let mut output = Vec::with_capacity(96);
        append_ascii(&mut output, "safety_blackout_engage_request_v1")
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_ascii(&mut output, &self.operation_id)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)?;
        append_u64(&mut output, self.request_id);
        Ok(output)
    }
}

impl Serialize for SafetyBlackoutEngageRequestV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SafetyBlackoutEngageRequestV1", 2)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SafetyBlackoutEngageRequestV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SafetyBlackoutEngageRequestV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyBlackoutEngageOutcomeV1 {
    Applied,
    NoOp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyBlackoutEngageReceiptV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub shape_sha256: String,
    pub audit_sequence: u64,
    pub outcome: SafetyBlackoutEngageOutcomeV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SafetyBlackoutEngageReceiptV1Wire {
    operation_id: String,
    request_id: u64,
    shape_sha256: String,
    audit_sequence: u64,
    outcome: SafetyBlackoutEngageOutcomeV1,
}

impl SafetyBlackoutEngageReceiptV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0
            || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.audit_sequence == 0
            || self.audit_sequence > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err(RuntimeCommandValidationErrorV1::InvalidJavaScriptSafeInteger);
        }
        validate_lower_hex_sha256(&self.shape_sha256)
            .map_err(RuntimeCommandValidationErrorV1::from_authored)
    }
}

impl Serialize for SafetyBlackoutEngageReceiptV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SafetyBlackoutEngageReceiptV1", 5)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("shape_sha256", &self.shape_sha256)?;
        state.serialize_field("audit_sequence", &self.audit_sequence)?;
        state.serialize_field("outcome", &self.outcome)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SafetyBlackoutEngageReceiptV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SafetyBlackoutEngageReceiptV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            shape_sha256: wire.shape_sha256,
            audit_sequence: wire.audit_sequence,
            outcome: wire.outcome,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafetyBlackoutEngageRejectionV1 {
    pub operation_id: String,
    pub request_id: u64,
    pub error: RuntimeCommandErrorV1,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct SafetyBlackoutEngageRejectionV1Wire {
    operation_id: String,
    request_id: u64,
    error: RuntimeCommandErrorV1,
}

impl SafetyBlackoutEngageRejectionV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        if self.operation_id != SAFETY_BLACKOUT_ENGAGE_OPERATION_ID {
            return Err(RuntimeCommandValidationErrorV1::UnexpectedOperationId);
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(RuntimeCommandValidationErrorV1::InvalidRequestId);
        }
        Ok(())
    }
}

impl Serialize for SafetyBlackoutEngageRejectionV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        let mut state = serializer.serialize_struct("SafetyBlackoutEngageRejectionV1", 3)?;
        state.serialize_field("operation_id", &self.operation_id)?;
        state.serialize_field("request_id", &self.request_id)?;
        state.serialize_field("error", &self.error)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for SafetyBlackoutEngageRejectionV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = SafetyBlackoutEngageRejectionV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            error: wire.error,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "result",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum SafetyBlackoutEngageResponseV1 {
    Receipt(SafetyBlackoutEngageReceiptV1),
    Rejected(SafetyBlackoutEngageRejectionV1),
}

impl SafetyBlackoutEngageResponseV1 {
    pub fn validate(&self) -> Result<(), RuntimeCommandValidationErrorV1> {
        match self {
            Self::Receipt(receipt) => receipt.validate(),
            Self::Rejected(rejection) => rejection.validate(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeCommandValidationErrorV1 {
    UnexpectedOperationId,
    UnexpectedRuntimeDomain,
    InvalidRequestId,
    InvalidAuthorityId,
    InvalidJavaScriptSafeInteger,
    InvalidNoOpAuthority,
    InvalidAppliedAuthority,
    Authored(AuthoredControlPlaneValidationError),
}

impl RuntimeCommandValidationErrorV1 {
    fn from_authored(error: AuthoredControlPlaneValidationError) -> Self {
        Self::Authored(error)
    }
}

impl fmt::Display for RuntimeCommandValidationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnexpectedOperationId => formatter.write_str("unexpected runtime operation ID"),
            Self::UnexpectedRuntimeDomain => formatter.write_str("unexpected runtime domain"),
            Self::InvalidRequestId => {
                formatter.write_str("request ID must be JavaScript-safe and non-zero")
            }
            Self::InvalidAuthorityId => {
                formatter.write_str("authority ID must be canonical base64url")
            }
            Self::InvalidJavaScriptSafeInteger => {
                formatter.write_str("runtime generation must be JavaScript-safe and non-zero")
            }
            Self::InvalidNoOpAuthority => {
                formatter.write_str("NoOp receipt changed its runtime authority")
            }
            Self::InvalidAppliedAuthority => formatter
                .write_str("Applied receipt did not advance runtime authority exactly once"),
            Self::Authored(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for RuntimeCommandValidationErrorV1 {}

fn validate_runtime_authority_id(value: &str) -> Result<(), RuntimeCommandValidationErrorV1> {
    if value.len() != 22 || !value.is_ascii() {
        return Err(RuntimeCommandValidationErrorV1::InvalidAuthorityId);
    }
    let mut last = None;
    for byte in value.bytes() {
        let decoded = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'-' => 62,
            b'_' => 63,
            _ => return Err(RuntimeCommandValidationErrorV1::InvalidAuthorityId),
        };
        last = Some(decoded);
    }
    // 16 bytes occupy 22 unpadded base64url characters; the final sextet has
    // only two payload bits, so canonical encoding requires its low four bits
    // to be zero. That also rules out alternate textual encodings of one ID.
    if last.is_none_or(|decoded| decoded & 0x0f != 0) {
        return Err(RuntimeCommandValidationErrorV1::InvalidAuthorityId);
    }
    Ok(())
}

/// The exact non-saturating successor shared by the wire receipt contract and
/// the engine worker. Once both counters reach JavaScript's safe maximum there
/// is deliberately no next transport authority.
pub fn next_timeline_transport_authority(epoch: u64, generation: u64) -> Option<(u64, u64)> {
    if epoch == 0
        || generation == 0
        || epoch > MAX_SAFE_JAVASCRIPT_INTEGER
        || generation > MAX_SAFE_JAVASCRIPT_INTEGER
    {
        return None;
    }
    if generation < MAX_SAFE_JAVASCRIPT_INTEGER {
        Some((epoch, generation + 1))
    } else if epoch < MAX_SAFE_JAVASCRIPT_INTEGER {
        Some((epoch + 1, 1))
    } else {
        None
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputControlValidationErrorV1 {
    UnexpectedOperationId,
    OperationActionMismatch,
    InvalidRequestId,
    InvalidFence,
    InvalidOpaqueId,
    InvalidFingerprint,
    InvalidStandbyIdentity,
    InvalidReceiptOutcome,
    InvalidLeaseAuthority,
    InvalidDisplayOutputSpec,
}

impl fmt::Display for OutputControlValidationErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::UnexpectedOperationId => "unexpected output-control operation ID",
            Self::OperationActionMismatch => "output-control operation does not match its action",
            Self::InvalidRequestId => "request ID must be JavaScript-safe and non-zero",
            Self::InvalidFence => "output-control fence is invalid",
            Self::InvalidOpaqueId => "opaque output-control ID must be canonical base64url",
            Self::InvalidFingerprint => "output-control fingerprint must be lowercase SHA-256",
            Self::InvalidStandbyIdentity => "standby takeover identity is invalid",
            Self::InvalidReceiptOutcome => "output-control receipt outcome is invalid",
            Self::InvalidLeaseAuthority => "output lease authority is not canonical",
            Self::InvalidDisplayOutputSpec => "display output specification is invalid",
        })
    }
}

impl std::error::Error for OutputControlValidationErrorV1 {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputControlFenceV1 {
    pub process_incarnation: u64,
    pub session_incarnation: u64,
    pub project_epoch: u64,
    pub project_revision: u64,
    pub project_checkpoint_hash: String,
    pub project_publication_generation: u64,
    pub output_epoch: u64,
    pub output_generation: u64,
    pub safety_blackout_epoch: u64,
    pub safety_blackout_generation: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputControlFenceV1Wire {
    process_incarnation: u64,
    session_incarnation: u64,
    project_epoch: u64,
    project_revision: u64,
    project_checkpoint_hash: String,
    project_publication_generation: u64,
    output_epoch: u64,
    output_generation: u64,
    safety_blackout_epoch: u64,
    safety_blackout_generation: u64,
}

impl OutputControlFenceV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.process_incarnation == 0
            || self.session_incarnation == 0
            || self.output_epoch == 0
            || self.output_generation == 0
            || self.safety_blackout_epoch == 0
            || self.safety_blackout_generation == 0
            || [
                self.process_incarnation,
                self.session_incarnation,
                self.project_epoch,
                self.project_revision,
                self.project_publication_generation,
                self.output_epoch,
                self.output_generation,
                self.safety_blackout_epoch,
                self.safety_blackout_generation,
            ]
            .into_iter()
            .any(|value| value > MAX_SAFE_JAVASCRIPT_INTEGER)
            || validate_lower_hex_sha256(&self.project_checkpoint_hash).is_err()
        {
            return Err(OutputControlValidationErrorV1::InvalidFence);
        }
        Ok(())
    }

    fn wire(&self) -> OutputControlFenceV1Wire {
        OutputControlFenceV1Wire {
            process_incarnation: self.process_incarnation,
            session_incarnation: self.session_incarnation,
            project_epoch: self.project_epoch,
            project_revision: self.project_revision,
            project_checkpoint_hash: self.project_checkpoint_hash.clone(),
            project_publication_generation: self.project_publication_generation,
            output_epoch: self.output_epoch,
            output_generation: self.output_generation,
            safety_blackout_epoch: self.safety_blackout_epoch,
            safety_blackout_generation: self.safety_blackout_generation,
        }
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), OutputControlValidationErrorV1> {
        self.validate()?;
        for value in [
            self.process_incarnation,
            self.session_incarnation,
            self.project_epoch,
            self.project_revision,
        ] {
            append_u64(output, value);
        }
        append_ascii(output, &self.project_checkpoint_hash)
            .map_err(|_| OutputControlValidationErrorV1::InvalidFence)?;
        for value in [
            self.project_publication_generation,
            self.output_epoch,
            self.output_generation,
            self.safety_blackout_epoch,
            self.safety_blackout_generation,
        ] {
            append_u64(output, value);
        }
        Ok(())
    }
}

impl Serialize for OutputControlFenceV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        self.wire().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlFenceV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlFenceV1Wire::deserialize(deserializer)?;
        let value = Self {
            process_incarnation: wire.process_incarnation,
            session_incarnation: wire.session_incarnation,
            project_epoch: wire.project_epoch,
            project_revision: wire.project_revision,
            project_checkpoint_hash: wire.project_checkpoint_hash,
            project_publication_generation: wire.project_publication_generation,
            output_epoch: wire.output_epoch,
            output_generation: wire.output_generation,
            safety_blackout_epoch: wire.safety_blackout_epoch,
            safety_blackout_generation: wire.safety_blackout_generation,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputControlTargetRoleV1 {
    Lighting,
    Video,
    Both,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OutputLeaseAuthorityV1 {
    pub lease_id: String,
    pub generation: u64,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputLeaseAuthorityV1Wire {
    lease_id: String,
    generation: u64,
}

impl OutputLeaseAuthorityV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        let token = self.lease_id.as_bytes();
        let valid_id = token.len() == 22
            && self.lease_id.is_ascii()
            && token.starts_with(b"lease-")
            && token[6..]
                .iter()
                .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(byte))
            && token[6..].iter().any(|byte| *byte != b'0');
        if !valid_id || self.generation == 0 || self.generation > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(OutputControlValidationErrorV1::InvalidLeaseAuthority);
        }
        Ok(())
    }

    pub fn canonical_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        let mut bytes = b"output-lease-authority-v1\0".to_vec();
        append_ascii(&mut bytes, &self.lease_id)
            .map_err(|_| OutputControlValidationErrorV1::InvalidLeaseAuthority)?;
        append_u64(&mut bytes, self.generation);
        Ok(bytes)
    }
}

impl Serialize for OutputLeaseAuthorityV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputLeaseAuthorityV1Wire {
            lease_id: self.lease_id.clone(),
            generation: self.generation,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputLeaseAuthorityV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputLeaseAuthorityV1Wire::deserialize(deserializer)?;
        let value = Self {
            lease_id: wire.lease_id,
            generation: wire.generation,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputControlActionV2 {
    EnableOutput,
    Arm {
        role: OutputControlTargetRoleV1,
        lease: OutputLeaseAuthorityV1,
    },
    ReleaseBlackout {
        lease: OutputLeaseAuthorityV1,
    },
    TakeOverStandby {
        force: bool,
        standby_session_id: String,
        standby_generation: u64,
        lease: OutputLeaseAuthorityV1,
    },
    AcquireLease {
        role: OutputControlTargetRoleV1,
    },
    RenewLease {
        lease: OutputLeaseAuthorityV1,
    },
    RecoverLease {
        lease: OutputLeaseAuthorityV1,
    },
    RelinquishOutputLease {
        lease: OutputLeaseAuthorityV1,
    },
    ForceTransferLease {
        lease: OutputLeaseAuthorityV1,
    },
    AddDisplay {
        spec: DisplayOutputSpecV2,
        lease: OutputLeaseAuthorityV1,
    },
    SetDisplayWindowOpen {
        output_id: u64,
        open: bool,
        lease: OutputLeaseAuthorityV1,
    },
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
enum OutputControlActionV2Wire {
    EnableOutput {},
    Arm {
        role: OutputControlTargetRoleV1,
        lease: OutputLeaseAuthorityV1,
    },
    ReleaseBlackout {
        lease: OutputLeaseAuthorityV1,
    },
    TakeOverStandby {
        force: bool,
        standby_session_id: String,
        standby_generation: u64,
        lease: OutputLeaseAuthorityV1,
    },
    AcquireLease {
        role: OutputControlTargetRoleV1,
    },
    RenewLease {
        lease: OutputLeaseAuthorityV1,
    },
    RecoverLease {
        lease: OutputLeaseAuthorityV1,
    },
    RelinquishOutputLease {
        lease: OutputLeaseAuthorityV1,
    },
    ForceTransferLease {
        lease: OutputLeaseAuthorityV1,
    },
    AddDisplay {
        spec: DisplayOutputSpecV2,
        lease: OutputLeaseAuthorityV1,
    },
    SetDisplayWindowOpen {
        output_id: u64,
        open: bool,
        lease: OutputLeaseAuthorityV1,
    },
}

impl OutputControlActionV2 {
    pub const fn operation_id(&self) -> &'static str {
        match self {
            Self::EnableOutput => OUTPUT_ENABLE_OPERATION_ID,
            Self::Arm { .. } => OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
            Self::ReleaseBlackout { .. } => OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
            Self::TakeOverStandby { .. } => OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
            Self::AcquireLease { .. } => OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
            Self::RenewLease { .. } => OUTPUT_LEASE_RENEW_OPERATION_ID,
            Self::RecoverLease { .. } => OUTPUT_LEASE_RECOVER_OPERATION_ID,
            Self::RelinquishOutputLease { .. } => OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
            Self::ForceTransferLease { .. } => OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
            Self::AddDisplay { .. } => OUTPUT_DISPLAY_ADD_OPERATION_ID,
            Self::SetDisplayWindowOpen { .. } => OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID,
        }
    }

    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if let Self::TakeOverStandby {
            standby_session_id,
            standby_generation,
            lease,
            ..
        } = self
        {
            if standby_session_id.is_empty()
                || standby_session_id.len() > 128
                || !standby_session_id.is_ascii()
                || standby_session_id
                    .bytes()
                    .any(|byte| byte.is_ascii_control() || matches!(byte, b'/' | b'\\'))
                || *standby_generation == 0
                || *standby_generation > MAX_SAFE_JAVASCRIPT_INTEGER
            {
                return Err(OutputControlValidationErrorV1::InvalidStandbyIdentity);
            }
            lease.validate()?;
        }
        match self {
            Self::EnableOutput => {}
            Self::Arm { lease, .. }
            | Self::ReleaseBlackout { lease }
            | Self::RenewLease { lease }
            | Self::RecoverLease { lease }
            | Self::RelinquishOutputLease { lease }
            | Self::ForceTransferLease { lease } => lease.validate()?,
            Self::AddDisplay { spec, lease } => {
                spec.validate()?;
                lease.validate()?;
            }
            Self::SetDisplayWindowOpen {
                output_id, lease, ..
            } => {
                if *output_id == 0 || *output_id > MAX_SAFE_JAVASCRIPT_INTEGER {
                    return Err(OutputControlValidationErrorV1::InvalidDisplayOutputSpec);
                }
                lease.validate()?;
            }
            Self::TakeOverStandby { .. } | Self::AcquireLease { .. } => {}
        }
        Ok(())
    }

    fn wire(&self) -> OutputControlActionV2Wire {
        match self {
            Self::EnableOutput => OutputControlActionV2Wire::EnableOutput {},
            Self::Arm { role, lease } => OutputControlActionV2Wire::Arm {
                role: *role,
                lease: lease.clone(),
            },
            Self::ReleaseBlackout { lease } => OutputControlActionV2Wire::ReleaseBlackout {
                lease: lease.clone(),
            },
            Self::TakeOverStandby {
                force,
                standby_session_id,
                standby_generation,
                lease,
            } => OutputControlActionV2Wire::TakeOverStandby {
                force: *force,
                standby_session_id: standby_session_id.clone(),
                standby_generation: *standby_generation,
                lease: lease.clone(),
            },
            Self::AcquireLease { role } => OutputControlActionV2Wire::AcquireLease { role: *role },
            Self::RenewLease { lease } => OutputControlActionV2Wire::RenewLease {
                lease: lease.clone(),
            },
            Self::RecoverLease { lease } => OutputControlActionV2Wire::RecoverLease {
                lease: lease.clone(),
            },
            Self::RelinquishOutputLease { lease } => {
                OutputControlActionV2Wire::RelinquishOutputLease {
                    lease: lease.clone(),
                }
            }
            Self::ForceTransferLease { lease } => OutputControlActionV2Wire::ForceTransferLease {
                lease: lease.clone(),
            },
            Self::AddDisplay { spec, lease } => OutputControlActionV2Wire::AddDisplay {
                spec: spec.clone(),
                lease: lease.clone(),
            },
            Self::SetDisplayWindowOpen {
                output_id,
                open,
                lease,
            } => OutputControlActionV2Wire::SetDisplayWindowOpen {
                output_id: *output_id,
                open: *open,
                lease: lease.clone(),
            },
        }
    }

    fn append_canonical_bytes(
        &self,
        output: &mut Vec<u8>,
    ) -> Result<(), OutputControlValidationErrorV1> {
        self.validate()?;
        match self {
            Self::EnableOutput => {
                output.push(9);
            }
            Self::Arm { role, lease } => {
                output.push(0);
                output.push(match role {
                    OutputControlTargetRoleV1::Lighting => 0,
                    OutputControlTargetRoleV1::Video => 1,
                    OutputControlTargetRoleV1::Both => 2,
                });
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::ReleaseBlackout { lease } => {
                output.push(1);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::TakeOverStandby {
                force,
                standby_session_id,
                standby_generation,
                lease,
            } => {
                output.push(2);
                output.push(u8::from(*force));
                append_ascii(output, standby_session_id)
                    .map_err(|_| OutputControlValidationErrorV1::InvalidStandbyIdentity)?;
                append_u64(output, *standby_generation);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::AcquireLease { role } => {
                output.push(3);
                output.push(match role {
                    OutputControlTargetRoleV1::Lighting => 0,
                    OutputControlTargetRoleV1::Video => 1,
                    OutputControlTargetRoleV1::Both => 2,
                });
            }
            Self::RenewLease { lease } => {
                output.push(4);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::RecoverLease { lease } => {
                output.push(5);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::RelinquishOutputLease { lease } => {
                output.push(6);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::ForceTransferLease { lease } => {
                output.push(7);
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::AddDisplay { spec, lease } => {
                output.push(8);
                spec.append_canonical_bytes(output)?;
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
            Self::SetDisplayWindowOpen {
                output_id,
                open,
                lease,
            } => {
                // 0..=9 are frozen wire discriminants.  Appending 10 keeps
                // old golden bytes byte-for-byte stable.
                output.push(10);
                append_u64(output, *output_id);
                output.push(u8::from(*open));
                output.extend_from_slice(lease.lease_id.as_bytes());
                append_u64(output, lease.generation);
            }
        }
        Ok(())
    }
}

impl Serialize for OutputControlActionV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        self.wire().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlActionV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlActionV2Wire::deserialize(deserializer)?;
        let value = match wire {
            OutputControlActionV2Wire::EnableOutput {} => Self::EnableOutput,
            OutputControlActionV2Wire::Arm { role, lease } => Self::Arm { role, lease },
            OutputControlActionV2Wire::ReleaseBlackout { lease } => Self::ReleaseBlackout { lease },
            OutputControlActionV2Wire::TakeOverStandby {
                force,
                standby_session_id,
                standby_generation,
                lease,
            } => Self::TakeOverStandby {
                force,
                standby_session_id,
                standby_generation,
                lease,
            },
            OutputControlActionV2Wire::AcquireLease { role } => Self::AcquireLease { role },
            OutputControlActionV2Wire::RenewLease { lease } => Self::RenewLease { lease },
            OutputControlActionV2Wire::RecoverLease { lease } => Self::RecoverLease { lease },
            OutputControlActionV2Wire::RelinquishOutputLease { lease } => {
                Self::RelinquishOutputLease { lease }
            }
            OutputControlActionV2Wire::ForceTransferLease { lease } => {
                Self::ForceTransferLease { lease }
            }
            OutputControlActionV2Wire::AddDisplay { spec, lease } => {
                Self::AddDisplay { spec, lease }
            }
            OutputControlActionV2Wire::SetDisplayWindowOpen {
                output_id,
                open,
                lease,
            } => Self::SetDisplayWindowOpen {
                output_id,
                open,
                lease,
            },
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlAuthorityBundleV1 {
    pub operation_id: String,
    pub fence: OutputControlFenceV1,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case", deny_unknown_fields)]
pub enum OutputLeaseAuthorityQueryStatusV1 {
    Unavailable,
    HeldActive {
        authority: OutputLeaseAuthorityV1,
        resources: Vec<OutputControlTargetRoleV1>,
    },
    HeldOrphaned {
        authority: OutputLeaseAuthorityV1,
        resources: Vec<OutputControlTargetRoleV1>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputLeaseAuthorityQueryV1 {
    pub operation_id: String,
    pub statuses: Vec<OutputLeaseAuthorityQueryStatusV1>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputLeaseAuthorityQueryV1Wire {
    operation_id: String,
    statuses: Vec<OutputLeaseAuthorityQueryStatusV1>,
}

impl OutputLeaseAuthorityQueryV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID
            || self.statuses.is_empty()
            || self.statuses.len() > MAX_OUTPUT_LEASE_QUERY_STATUSES
        {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        if self
            .statuses
            .iter()
            .any(|status| matches!(status, OutputLeaseAuthorityQueryStatusV1::Unavailable))
        {
            if self.statuses.len() != 1
                || !matches!(
                    self.statuses.first(),
                    Some(OutputLeaseAuthorityQueryStatusV1::Unavailable)
                )
            {
                return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
            }
            return Ok(());
        }
        let mut previous_lease_id: Option<&str> = None;
        for status in &self.statuses {
            let (authority, resources) = match status {
                OutputLeaseAuthorityQueryStatusV1::HeldActive {
                    authority,
                    resources,
                }
                | OutputLeaseAuthorityQueryStatusV1::HeldOrphaned {
                    authority,
                    resources,
                } => (authority, resources),
                OutputLeaseAuthorityQueryStatusV1::Unavailable => {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
                }
            };
            authority.validate()?;
            if previous_lease_id.is_some_and(|previous| previous >= authority.lease_id.as_str()) {
                return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
            }
            previous_lease_id = Some(authority.lease_id.as_str());
            if resources.is_empty()
                || resources.windows(2).any(|pair| pair[0] >= pair[1])
                || resources.contains(&OutputControlTargetRoleV1::Both)
            {
                return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
            }
        }
        Ok(())
    }
}

impl Serialize for OutputLeaseAuthorityQueryV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputLeaseAuthorityQueryV1Wire {
            operation_id: self.operation_id.clone(),
            statuses: self.statuses.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputLeaseAuthorityQueryV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputLeaseAuthorityQueryV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            statuses: wire.statuses,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputControlAuthorityBundleV1Wire {
    operation_id: String,
    fence: OutputControlFenceV1,
}

impl OutputControlAuthorityBundleV1 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        self.fence.validate()
    }
}

impl Serialize for OutputControlAuthorityBundleV1 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputControlAuthorityBundleV1Wire {
            operation_id: self.operation_id.clone(),
            fence: self.fence.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlAuthorityBundleV1 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlAuthorityBundleV1Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            fence: wire.fence,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlCommandRequestV2 {
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: OutputControlFenceV1,
    pub action: OutputControlActionV2,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputControlCommandRequestV2Wire {
    operation_id: String,
    request_id: u64,
    expected_fence: OutputControlFenceV1,
    action: OutputControlActionV2,
}

impl OutputControlCommandRequestV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if self.operation_id != self.action.operation_id() {
            return Err(OutputControlValidationErrorV1::OperationActionMismatch);
        }
        validate_output_request_id(self.request_id)?;
        self.expected_fence.validate()?;
        self.action.validate()
    }

    pub fn canonical_shape_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        let mut bytes = OUTPUT_CONTROL_SHAPE_DOMAIN_V2.to_vec();
        append_ascii(&mut bytes, &self.operation_id)
            .map_err(|_| OutputControlValidationErrorV1::UnexpectedOperationId)?;
        self.expected_fence.append_canonical_bytes(&mut bytes)?;
        self.action.append_canonical_bytes(&mut bytes)?;
        Ok(bytes)
    }

    pub fn argument_fingerprint_bytes(&self) -> Result<Vec<u8>, OutputControlValidationErrorV1> {
        self.validate()?;
        let mut bytes = OUTPUT_CONTROL_ARGUMENT_FINGERPRINT_DOMAIN_V2.to_vec();
        append_ascii(&mut bytes, &self.operation_id)
            .map_err(|_| OutputControlValidationErrorV1::UnexpectedOperationId)?;
        self.action.append_canonical_bytes(&mut bytes)?;
        Ok(bytes)
    }
}

impl Serialize for OutputControlCommandRequestV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputControlCommandRequestV2Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            expected_fence: self.expected_fence.clone(),
            action: self.action.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlCommandRequestV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlCommandRequestV2Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
            action: wire.action,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputControlReceiptOutcomeV2 {
    Applied,
    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputControlErrorCodeV2 {
    InvalidRequest,
    Forbidden,
    StaleFence,
    Busy,
    Overloaded,
    PublicationFailed,
    Internal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLeaseReceiptPhaseV2 {
    Unclaimed,
    HeldActive,
    HeldOrphaned,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputLeaseReceiptOutcomeV2 {
    Acquired,
    Renewed,
    ExpiryObserved,
    Recovered,
    Relinquished,
    Transferred,
    OwnerRetired,
    ProjectOrphaned,
    Authorized,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputLeaseReceiptChangeV2 {
    pub lease_id: String,
    pub before_generation: Option<u64>,
    pub after_generation: Option<u64>,
    pub before_resources: Vec<OutputControlTargetRoleV1>,
    pub after_resources: Vec<OutputControlTargetRoleV1>,
    pub before_phase: Option<OutputLeaseReceiptPhaseV2>,
    pub after_phase: Option<OutputLeaseReceiptPhaseV2>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OutputControlLeaseResultV2 {
    pub authority: OutputLeaseAuthorityV1,
    pub resources: Vec<OutputControlTargetRoleV1>,
    pub phase: OutputLeaseReceiptPhaseV2,
    pub outcome: OutputLeaseReceiptOutcomeV2,
    pub audit_sequence: u64,
    pub changes: Vec<OutputLeaseReceiptChangeV2>,
}

impl OutputControlLeaseResultV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        self.authority.validate()?;
        if self.audit_sequence == 0 || self.audit_sequence > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        if self.resources.is_empty()
            || self.resources.windows(2).any(|pair| pair[0] >= pair[1])
            || self.resources.contains(&OutputControlTargetRoleV1::Both)
        {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        if self.changes.len() != 1 {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        let change = &self.changes[0];
        if change.lease_id != self.authority.lease_id {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        let change_generation = change
            .after_generation
            .or(change.before_generation)
            .ok_or(OutputControlValidationErrorV1::InvalidReceiptOutcome)?;
        if change_generation != self.authority.generation {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        let terminal_phase = change
            .after_phase
            .or(change.before_phase)
            .ok_or(OutputControlValidationErrorV1::InvalidReceiptOutcome)?;
        if terminal_phase != self.phase {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        let terminal_resources = if self.outcome == OutputLeaseReceiptOutcomeV2::Relinquished {
            &change.before_resources
        } else {
            &change.after_resources
        };
        if terminal_resources != &self.resources {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        {
            let authority = OutputLeaseAuthorityV1 {
                lease_id: change.lease_id.clone(),
                generation: change
                    .before_generation
                    .or(change.after_generation)
                    .ok_or(OutputControlValidationErrorV1::InvalidReceiptOutcome)?,
            };
            authority.validate()?;
            for generation in [change.before_generation, change.after_generation]
                .into_iter()
                .flatten()
            {
                if generation == 0 || generation > MAX_SAFE_JAVASCRIPT_INTEGER {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            if change
                .before_resources
                .windows(2)
                .any(|pair| pair[0] >= pair[1])
                || change
                    .after_resources
                    .windows(2)
                    .any(|pair| pair[0] >= pair[1])
                || change
                    .before_resources
                    .contains(&OutputControlTargetRoleV1::Both)
                || change
                    .after_resources
                    .contains(&OutputControlTargetRoleV1::Both)
            {
                return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
            }
        }
        Ok(())
    }

    fn validate_for_operation(
        &self,
        operation_id: &str,
    ) -> Result<(), OutputControlValidationErrorV1> {
        self.validate()?;
        let expected_outcome = match operation_id {
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID
            | OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
            | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Authorized,
            OUTPUT_LEASE_ACQUIRE_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Acquired,
            OUTPUT_LEASE_RENEW_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Renewed,
            OUTPUT_LEASE_RECOVER_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Recovered,
            OUTPUT_LEASE_RELINQUISH_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Relinquished,
            OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Transferred,
            OUTPUT_DISPLAY_ADD_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Authorized,
            OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Authorized,
            OUTPUT_ENABLE_OPERATION_ID => OutputLeaseReceiptOutcomeV2::Acquired,
            _ => return Err(OutputControlValidationErrorV1::UnexpectedOperationId),
        };
        let enable_outcome = operation_id == OUTPUT_ENABLE_OPERATION_ID
            && matches!(
                self.outcome,
                OutputLeaseReceiptOutcomeV2::Acquired | OutputLeaseReceiptOutcomeV2::Recovered
            );
        let takeover_project_orphan = operation_id == OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
            && self.outcome == OutputLeaseReceiptOutcomeV2::ProjectOrphaned;
        if (!takeover_project_orphan && !enable_outcome && self.outcome != expected_outcome)
            || self.changes.len() != 1
        {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        let change = self
            .changes
            .first()
            .ok_or(OutputControlValidationErrorV1::InvalidReceiptOutcome)?;
        let before_generation = change.before_generation;
        let after_generation = change.after_generation;
        let transition_outcome = if takeover_project_orphan {
            OutputLeaseReceiptOutcomeV2::ProjectOrphaned
        } else if enable_outcome {
            self.outcome
        } else {
            expected_outcome
        };
        match transition_outcome {
            OutputLeaseReceiptOutcomeV2::Authorized => {
                if before_generation != after_generation
                    || change.before_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
                    || change.after_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
                    || change.before_resources != self.resources
                    || change.after_resources != self.resources
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            OutputLeaseReceiptOutcomeV2::Acquired => {
                if change.before_generation.is_some()
                    || change.before_phase.is_some()
                    || !change.before_resources.is_empty()
                    || change.after_generation != Some(self.authority.generation)
                    || change.after_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
                    || change.after_resources != self.resources
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            OutputLeaseReceiptOutcomeV2::Renewed
            | OutputLeaseReceiptOutcomeV2::Recovered
            | OutputLeaseReceiptOutcomeV2::Transferred => {
                if before_generation.is_none()
                    || after_generation.is_none()
                    || after_generation <= before_generation
                    || change.after_generation != Some(self.authority.generation)
                    || change.after_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
                    || change.after_resources != self.resources
                    || change.before_resources.is_empty()
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
                let expected_before_phase = match transition_outcome {
                    OutputLeaseReceiptOutcomeV2::Recovered => {
                        OutputLeaseReceiptPhaseV2::HeldOrphaned
                    }
                    _ => OutputLeaseReceiptPhaseV2::HeldActive,
                };
                if change.before_phase != Some(expected_before_phase)
                    || change.before_resources != self.resources
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            OutputLeaseReceiptOutcomeV2::Relinquished => {
                if before_generation.is_none()
                    || after_generation.is_none()
                    || after_generation <= before_generation
                    || change.after_generation != Some(self.authority.generation)
                    || change.after_phase != Some(OutputLeaseReceiptPhaseV2::Unclaimed)
                    || !change.after_resources.is_empty()
                    || change.before_resources != self.resources
                    || !matches!(
                        change.before_phase,
                        Some(OutputLeaseReceiptPhaseV2::HeldActive)
                            | Some(OutputLeaseReceiptPhaseV2::HeldOrphaned)
                    )
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            OutputLeaseReceiptOutcomeV2::ProjectOrphaned if takeover_project_orphan => {
                if before_generation.is_none()
                    || after_generation.is_none()
                    || after_generation <= before_generation
                    || change.after_generation != Some(self.authority.generation)
                    || change.before_phase != Some(OutputLeaseReceiptPhaseV2::HeldActive)
                    || change.after_phase != Some(OutputLeaseReceiptPhaseV2::HeldOrphaned)
                    || change.before_resources != self.resources
                    || change.after_resources != self.resources
                {
                    return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
                }
            }
            OutputLeaseReceiptOutcomeV2::ExpiryObserved
            | OutputLeaseReceiptOutcomeV2::OwnerRetired
            | OutputLeaseReceiptOutcomeV2::ProjectOrphaned => {
                return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlReceiptV2 {
    pub operation_id: String,
    pub request_id: u64,
    pub shape_sha256: String,
    pub argument_fingerprint: String,
    pub audit_sequence: u64,
    pub fence_before: OutputControlFenceV1,
    pub fence_after: OutputControlFenceV1,
    pub outcome: OutputControlReceiptOutcomeV2,
    pub lease_result: Option<OutputControlLeaseResultV2>,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputControlReceiptV2Wire {
    operation_id: String,
    request_id: u64,
    shape_sha256: String,
    argument_fingerprint: String,
    audit_sequence: u64,
    fence_before: OutputControlFenceV1,
    fence_after: OutputControlFenceV1,
    outcome: OutputControlReceiptOutcomeV2,
    lease_result: Option<OutputControlLeaseResultV2>,
}

impl OutputControlReceiptV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if !matches!(
            self.operation_id.as_str(),
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                | OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                | OUTPUT_LEASE_RENEW_OPERATION_ID
                | OUTPUT_LEASE_RECOVER_OPERATION_ID
                | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
                | OUTPUT_DISPLAY_ADD_OPERATION_ID
                | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                | OUTPUT_ENABLE_OPERATION_ID
        ) {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        validate_output_request_id(self.request_id)?;
        validate_lower_hex_fingerprint(&self.shape_sha256)?;
        validate_lower_hex_fingerprint(&self.argument_fingerprint)?;
        if self.audit_sequence == 0 || self.audit_sequence > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        self.fence_before.validate()?;
        self.fence_after.validate()?;
        if let Some(lease_result) = &self.lease_result {
            lease_result.validate_for_operation(&self.operation_id)?;
        } else {
            return Err(OutputControlValidationErrorV1::InvalidReceiptOutcome);
        }
        match self.outcome {
            OutputControlReceiptOutcomeV2::NoOp if self.fence_before != self.fence_after => {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            // Physical live-window mutations intentionally leave the
            // persisted/project/output-ownership fence unchanged.  Their
            // durable receipt/audit identity is still authoritative.
            OutputControlReceiptOutcomeV2::Applied
                if self.fence_before == self.fence_after
                    && self.operation_id != OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID =>
            {
                Err(OutputControlValidationErrorV1::InvalidReceiptOutcome)
            }
            _ => Ok(()),
        }
    }
}

impl Serialize for OutputControlReceiptV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputControlReceiptV2Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            shape_sha256: self.shape_sha256.clone(),
            argument_fingerprint: self.argument_fingerprint.clone(),
            audit_sequence: self.audit_sequence,
            fence_before: self.fence_before.clone(),
            fence_after: self.fence_after.clone(),
            outcome: self.outcome,
            lease_result: self.lease_result.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlReceiptV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlReceiptV2Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            shape_sha256: wire.shape_sha256,
            argument_fingerprint: wire.argument_fingerprint,
            audit_sequence: wire.audit_sequence,
            fence_before: wire.fence_before,
            fence_after: wire.fence_after,
            outcome: wire.outcome,
            lease_result: wire.lease_result,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputControlRejectionV2 {
    pub operation_id: String,
    pub request_id: u64,
    pub error: OutputControlErrorCodeV2,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct OutputControlRejectionV2Wire {
    operation_id: String,
    request_id: u64,
    error: OutputControlErrorCodeV2,
}

impl OutputControlRejectionV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        if !matches!(
            self.operation_id.as_str(),
            OUTPUT_OWNERSHIP_ARM_OPERATION_ID
                | OUTPUT_BLACKOUT_RELEASE_OPERATION_ID
                | OUTPUT_STANDBY_TAKEOVER_OPERATION_ID
                | OUTPUT_LEASE_ACQUIRE_OPERATION_ID
                | OUTPUT_LEASE_RENEW_OPERATION_ID
                | OUTPUT_LEASE_RECOVER_OPERATION_ID
                | OUTPUT_LEASE_RELINQUISH_OPERATION_ID
                | OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID
                | OUTPUT_DISPLAY_ADD_OPERATION_ID
                | OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
                | OUTPUT_ENABLE_OPERATION_ID
        ) {
            return Err(OutputControlValidationErrorV1::UnexpectedOperationId);
        }
        validate_output_request_id(self.request_id)
    }
}

impl Serialize for OutputControlRejectionV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        OutputControlRejectionV2Wire {
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            error: self.error,
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlRejectionV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlRejectionV2Wire::deserialize(deserializer)?;
        let value = Self {
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            error: wire.error,
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutputControlResponseV2 {
    /// A terminal receipt is boxed so the response enum stays a small
    /// discriminated handle while preserving the exact JSON wire shape.
    Receipt(Box<OutputControlReceiptV2>),
    Rejected(OutputControlRejectionV2),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
enum OutputControlResponseV2Wire {
    Receipt {
        receipt: Box<OutputControlReceiptV2>,
    },
    Rejected {
        rejection: OutputControlRejectionV2,
    },
}

impl OutputControlResponseV2 {
    pub fn validate(&self) -> Result<(), OutputControlValidationErrorV1> {
        match self {
            Self::Receipt(receipt) => receipt.validate(),
            Self::Rejected(rejection) => rejection.validate(),
        }
    }
}

impl Serialize for OutputControlResponseV2 {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.validate().map_err(serde::ser::Error::custom)?;
        match self {
            Self::Receipt(receipt) => OutputControlResponseV2Wire::Receipt {
                receipt: receipt.clone(),
            },
            Self::Rejected(rejection) => OutputControlResponseV2Wire::Rejected {
                rejection: rejection.clone(),
            },
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for OutputControlResponseV2 {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = OutputControlResponseV2Wire::deserialize(deserializer)?;
        let value = match wire {
            OutputControlResponseV2Wire::Receipt { receipt } => Self::Receipt(receipt),
            OutputControlResponseV2Wire::Rejected { rejection } => Self::Rejected(rejection),
        };
        value.validate().map_err(D::Error::custom)?;
        Ok(value)
    }
}

fn validate_output_request_id(request_id: u64) -> Result<(), OutputControlValidationErrorV1> {
    if request_id == 0 || request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
        Err(OutputControlValidationErrorV1::InvalidRequestId)
    } else {
        Ok(())
    }
}

fn validate_lower_hex_fingerprint(value: &str) -> Result<(), OutputControlValidationErrorV1> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        Ok(())
    } else {
        Err(OutputControlValidationErrorV1::InvalidFingerprint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hash(seed: char) -> String {
        std::iter::repeat_n(seed, 64).collect()
    }

    fn fence() -> ProjectMutationFenceV1 {
        ProjectMutationFenceV1 {
            process_incarnation: 11,
            session_incarnation: 12,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: hash('a'),
            project_publication_generation: 5,
        }
    }

    fn request() -> AuthoredRequestV1<SetEffectEnabledPayload> {
        AuthoredRequestV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: 9,
            expected_fence: fence(),
            payload: SetEffectEnabledPayload {
                effect_id: 7,
                enabled: true,
            },
        }
    }

    fn applied_response() -> SetEffectEnabledResponseV1 {
        let request = request();
        SetEffectEnabledResponseV1::TerminalReceipt(SetEffectEnabledTerminalReceiptV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: request.request_id,
            start_fence: request.expected_fence.clone(),
            shape_sha256: hash('b'),
            outcome: SetEffectEnabledOutcomeV1::Applied(SetEffectEnabledAppliedV1 {
                effect_id: request.payload.effect_id,
                enabled: request.payload.enabled,
                post_fence: request.expected_fence,
            }),
        })
    }

    fn rejected_response() -> SetEffectEnabledResponseV1 {
        let request = request();
        SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: request.request_id,
            start_fence: request.expected_fence,
            error: AuthoredCommandErrorV1::new(AuthoredCommandErrorCodeV1::Conflict),
        })
    }

    fn runtime_fence() -> TimelineTransportRuntimeFenceV1 {
        TimelineTransportRuntimeFenceV1 {
            project: fence(),
            domain: TIMELINE_TRANSPORT_RUNTIME_DOMAIN_V1.to_string(),
            source_runtime_epoch: 6,
            source_runtime_generation: 7,
        }
    }

    fn runtime_request() -> RuntimeCommandRequestV1 {
        RuntimeCommandRequestV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            authority_id: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
            request_id: 10,
            expected_fence: runtime_fence(),
            payload: SetTimelinePlayingRuntimePayloadV1 { playing: true },
        }
    }

    fn follow_abort_fence() -> TimelineFollowAbortRuntimeFenceV1 {
        TimelineFollowAbortRuntimeFenceV1 {
            project: fence(),
            domain: TIMELINE_FOLLOW_ABORT_RUNTIME_DOMAIN_V1.to_string(),
            output_ownership_epoch: 11,
            follow_generation: 23,
        }
    }

    fn follow_abort_request() -> TimelineFollowAbortRuntimeRequestV1 {
        TimelineFollowAbortRuntimeRequestV1 {
            operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
            authority_id: "AAAAAAAAAAAAAAAAAAAAAA".to_string(),
            request_id: 12,
            expected_fence: follow_abort_fence(),
        }
    }

    fn set_fence_numeric_field(fence: &mut ProjectMutationFenceV1, field: &str, value: u64) {
        match field {
            "process_incarnation" => fence.process_incarnation = value,
            "session_incarnation" => fence.session_incarnation = value,
            "project_epoch" => fence.project_epoch = value,
            "project_revision" => fence.project_revision = value,
            "project_publication_generation" => fence.project_publication_generation = value,
            _ => panic!("unknown fence numeric field {field}"),
        }
    }

    #[test]
    fn request_is_strict_validated_and_uses_typed_canonical_bytes() {
        let request = request();
        let encoded = serde_json::to_string(&request).unwrap();
        let decoded: AuthoredRequestV1<SetEffectEnabledPayload> =
            serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, request);
        assert_eq!(
            decoded.canonical_shape_bytes().unwrap(),
            request.canonical_shape_bytes().unwrap()
        );

        let unknown = encoded.to_string().replacen('}', ",\"secret\":true}", 1);
        assert!(
            serde_json::from_str::<AuthoredRequestV1<SetEffectEnabledPayload>>(&unknown).is_err()
        );
        let mut invalid = request.clone();
        invalid.request_id = MAX_SAFE_JAVASCRIPT_INTEGER + 1;
        assert!(invalid.validate().is_err());
        invalid = request.clone();
        invalid.expected_fence.project_checkpoint_hash = hash('A');
        assert!(invalid.validate().is_err());
        invalid = request.clone();
        invalid.operation_id = "syndocal.effects.set_enabled.v2".to_string();
        assert!(invalid.validate().is_err());
    }

    #[test]
    fn terminal_noop_receipt_cannot_drift_the_start_fence() {
        let request = request();
        let receipt = SetEffectEnabledTerminalReceiptV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: request.request_id,
            start_fence: request.expected_fence.clone(),
            shape_sha256: hash('b'),
            outcome: SetEffectEnabledOutcomeV1::NoOp(SetEffectEnabledAppliedV1 {
                effect_id: request.payload.effect_id,
                enabled: request.payload.enabled,
                post_fence: request.expected_fence.clone(),
            }),
        };
        let response = SetEffectEnabledResponseV1::TerminalReceipt(receipt.clone());
        let bytes = serde_json::to_vec(&response).unwrap();
        assert_eq!(serde_json::to_vec(&response).unwrap(), bytes);
        let decoded: SetEffectEnabledResponseV1 = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(decoded, response);

        let mut drifted = receipt;
        let SetEffectEnabledOutcomeV1::NoOp(result) = &mut drifted.outcome else {
            unreachable!();
        };
        result.post_fence.project_revision += 1;
        assert!(drifted.validate().is_err());
    }

    #[test]
    fn rejection_has_only_fixed_error_data() {
        let request = request();
        let response = SetEffectEnabledResponseV1::Rejected(AuthoredCommandRejectionV1 {
            operation_id: SET_EFFECT_ENABLED_OPERATION_ID.to_string(),
            request_id: request.request_id,
            start_fence: request.expected_fence,
            error: AuthoredCommandErrorV1::new(AuthoredCommandErrorCodeV1::Conflict),
        });
        let json = serde_json::to_value(response).unwrap();
        assert_eq!(json["kind"], "rejected");
        assert_eq!(
            json["result"]["error"],
            serde_json::json!({ "code": "conflict" })
        );

        let mut forged = json;
        forged
            .as_object_mut()
            .unwrap()
            .insert("secret".to_string(), serde_json::json!("not-accepted"));
        assert!(serde_json::from_value::<SetEffectEnabledResponseV1>(forged).is_err());
    }

    #[test]
    fn every_client_visible_numeric_value_rejects_lossy_javascript_values_on_both_wire_directions()
    {
        const FENCE_NUMERIC_FIELDS: [&str; 5] = [
            "process_incarnation",
            "session_incarnation",
            "project_epoch",
            "project_revision",
            "project_publication_generation",
        ];
        let lossy = MAX_SAFE_JAVASCRIPT_INTEGER + 1;

        // The request covers its request identity, every start-fence counter
        // and the payload effect identity on outbound serialization.
        let mut invalid_request_id = request();
        invalid_request_id.request_id = lossy;
        assert!(serde_json::to_vec(&invalid_request_id).is_err());
        let mut invalid_effect_id = request();
        invalid_effect_id.payload.effect_id = lossy;
        assert!(serde_json::to_vec(&invalid_effect_id).is_err());
        for field in FENCE_NUMERIC_FIELDS {
            let mut invalid = request();
            set_fence_numeric_field(&mut invalid.expected_fence, field, lossy);
            assert!(
                serde_json::to_vec(&invalid).is_err(),
                "outbound request field {field}"
            );
        }

        // Receipts additionally expose a post-fence and applied effect ID.
        let mut invalid_receipt_request_id = applied_response();
        let SetEffectEnabledResponseV1::TerminalReceipt(receipt) = &mut invalid_receipt_request_id
        else {
            unreachable!();
        };
        receipt.request_id = lossy;
        assert!(serde_json::to_vec(&invalid_receipt_request_id).is_err());
        let mut invalid_receipt_effect_id = applied_response();
        let SetEffectEnabledResponseV1::TerminalReceipt(receipt) = &mut invalid_receipt_effect_id
        else {
            unreachable!();
        };
        let SetEffectEnabledOutcomeV1::Applied(applied) = &mut receipt.outcome else {
            unreachable!();
        };
        applied.effect_id = lossy;
        assert!(serde_json::to_vec(&invalid_receipt_effect_id).is_err());
        for field in FENCE_NUMERIC_FIELDS {
            let mut invalid_start = applied_response();
            let SetEffectEnabledResponseV1::TerminalReceipt(receipt) = &mut invalid_start else {
                unreachable!();
            };
            set_fence_numeric_field(&mut receipt.start_fence, field, lossy);
            assert!(
                serde_json::to_vec(&invalid_start).is_err(),
                "outbound receipt start field {field}"
            );

            let mut invalid_post = applied_response();
            let SetEffectEnabledResponseV1::TerminalReceipt(receipt) = &mut invalid_post else {
                unreachable!();
            };
            let SetEffectEnabledOutcomeV1::Applied(applied) = &mut receipt.outcome else {
                unreachable!();
            };
            set_fence_numeric_field(&mut applied.post_fence, field, lossy);
            assert!(
                serde_json::to_vec(&invalid_post).is_err(),
                "outbound receipt post field {field}"
            );
        }

        let request_json = serde_json::to_value(request()).unwrap();
        for pointer in [
            "/request_id",
            "/expected_fence/process_incarnation",
            "/expected_fence/session_incarnation",
            "/expected_fence/project_epoch",
            "/expected_fence/project_revision",
            "/expected_fence/project_publication_generation",
            "/payload/effect_id",
        ] {
            let mut forged = request_json.clone();
            *forged.pointer_mut(pointer).unwrap() = serde_json::json!(lossy);
            assert!(
                serde_json::from_value::<AuthoredRequestV1<SetEffectEnabledPayload>>(forged)
                    .is_err(),
                "inbound request field {pointer}"
            );
        }

        let response_json = serde_json::to_value(applied_response()).unwrap();
        for pointer in [
            "/result/request_id",
            "/result/start_fence/process_incarnation",
            "/result/start_fence/session_incarnation",
            "/result/start_fence/project_epoch",
            "/result/start_fence/project_revision",
            "/result/start_fence/project_publication_generation",
            "/result/outcome/result/effect_id",
            "/result/outcome/result/post_fence/process_incarnation",
            "/result/outcome/result/post_fence/session_incarnation",
            "/result/outcome/result/post_fence/project_epoch",
            "/result/outcome/result/post_fence/project_revision",
            "/result/outcome/result/post_fence/project_publication_generation",
        ] {
            let mut forged = response_json.clone();
            *forged.pointer_mut(pointer).unwrap() = serde_json::json!(lossy);
            assert!(
                serde_json::from_value::<SetEffectEnabledResponseV1>(forged).is_err(),
                "inbound response field {pointer}"
            );
        }

        let mut invalid_rejection_request_id = rejected_response();
        let SetEffectEnabledResponseV1::Rejected(rejection) = &mut invalid_rejection_request_id
        else {
            unreachable!();
        };
        rejection.request_id = lossy;
        assert!(serde_json::to_vec(&invalid_rejection_request_id).is_err());
        for field in FENCE_NUMERIC_FIELDS {
            let mut invalid_rejection = rejected_response();
            let SetEffectEnabledResponseV1::Rejected(rejection) = &mut invalid_rejection else {
                unreachable!();
            };
            set_fence_numeric_field(&mut rejection.start_fence, field, lossy);
            assert!(
                serde_json::to_vec(&invalid_rejection).is_err(),
                "outbound rejection field {field}"
            );
        }
        let rejected_json = serde_json::to_value(rejected_response()).unwrap();
        for pointer in [
            "/result/request_id",
            "/result/start_fence/process_incarnation",
            "/result/start_fence/session_incarnation",
            "/result/start_fence/project_epoch",
            "/result/start_fence/project_revision",
            "/result/start_fence/project_publication_generation",
        ] {
            let mut forged = rejected_json.clone();
            *forged.pointer_mut(pointer).unwrap() = serde_json::json!(lossy);
            assert!(
                serde_json::from_value::<SetEffectEnabledResponseV1>(forged).is_err(),
                "inbound rejection field {pointer}"
            );
        }
    }

    #[test]
    fn unknown_response_or_terminal_outcome_discriminators_fail_closed() {
        let response = serde_json::to_value(applied_response()).unwrap();
        let mut unknown_response = response.clone();
        unknown_response["kind"] = serde_json::json!("future_terminal_receipt");
        assert!(serde_json::from_value::<SetEffectEnabledResponseV1>(unknown_response).is_err());

        let mut unknown_outcome = response;
        unknown_outcome["result"]["outcome"]["kind"] = serde_json::json!("future_outcome");
        assert!(serde_json::from_value::<SetEffectEnabledResponseV1>(unknown_outcome).is_err());
    }

    #[test]
    fn runtime_transport_wire_is_strict_typed_and_noop_generation_cannot_drift() {
        let request = runtime_request();
        let encoded = serde_json::to_value(&request).unwrap();
        assert_eq!(
            serde_json::from_value::<RuntimeCommandRequestV1>(encoded.clone()).unwrap(),
            request
        );
        let mut unknown = encoded;
        unknown
            .as_object_mut()
            .unwrap()
            .insert("principal".to_string(), serde_json::json!("forged"));
        assert!(serde_json::from_value::<RuntimeCommandRequestV1>(unknown).is_err());

        let mut wrong_domain = runtime_request();
        wrong_domain.expected_fence.domain = "timeline.follow".to_string();
        assert!(wrong_domain.validate().is_err());
        let mut lossy_generation = runtime_request();
        lossy_generation.expected_fence.source_runtime_generation = MAX_SAFE_JAVASCRIPT_INTEGER + 1;
        assert!(serde_json::to_value(lossy_generation).is_err());
        let mut forged_authority = runtime_request();
        forged_authority.authority_id = "not-a-canonical-authority".to_string();
        assert!(forged_authority.validate().is_err());

        let receipt = RuntimeCommandReceiptV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            requested_playing: request.payload.playing,
            epoch_after: request.expected_fence.source_runtime_epoch,
            generation_after: request.expected_fence.source_runtime_generation,
            shape_sha256: hash('c'),
            outcome: RuntimeCommandReceiptOutcomeV1::NoOp,
        };
        let response = RuntimeCommandResponseV1::Receipt(receipt.clone());
        assert_eq!(
            serde_json::from_value::<RuntimeCommandResponseV1>(
                serde_json::to_value(&response).unwrap()
            )
            .unwrap(),
            response
        );
        let mut drifted = receipt;
        drifted.generation_after += 1;
        assert!(drifted.validate().is_err());

        let mut applied = RuntimeCommandReceiptV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            requested_playing: request.payload.playing,
            epoch_after: request.expected_fence.source_runtime_epoch,
            generation_after: request.expected_fence.source_runtime_generation + 1,
            shape_sha256: hash('d'),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        };
        applied.validate().unwrap();
        applied.generation_after += 1;
        assert!(
            applied.validate().is_err(),
            "Applied must advance exactly once"
        );

        let mut rollover = runtime_request();
        rollover.expected_fence.source_runtime_epoch = 9;
        rollover.expected_fence.source_runtime_generation = MAX_SAFE_JAVASCRIPT_INTEGER;
        let rollover_receipt = RuntimeCommandReceiptV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: rollover.request_id,
            fence_before: rollover.expected_fence.clone(),
            requested_playing: rollover.payload.playing,
            epoch_after: 10,
            generation_after: 1,
            shape_sha256: hash('e'),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        };
        rollover_receipt.validate().unwrap();
        let terminal_receipt = RuntimeCommandReceiptV1 {
            epoch_after: MAX_SAFE_JAVASCRIPT_INTEGER,
            generation_after: MAX_SAFE_JAVASCRIPT_INTEGER,
            fence_before: TimelineTransportRuntimeFenceV1 {
                source_runtime_epoch: MAX_SAFE_JAVASCRIPT_INTEGER,
                source_runtime_generation: MAX_SAFE_JAVASCRIPT_INTEGER,
                ..rollover.expected_fence
            },
            ..rollover_receipt
        };
        assert!(
            terminal_receipt.validate().is_err(),
            "terminal pair cannot be Applied"
        );

        let rejected = RuntimeCommandResponseV1::Rejected(RuntimeCommandRejectionV1 {
            operation_id: TIMELINE_TRANSPORT_SET_PLAYING_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence,
            error: RuntimeCommandErrorV1::new(RuntimeCommandErrorCodeV1::Conflict),
        });
        let json = serde_json::to_value(rejected).unwrap();
        assert_eq!(
            json["result"]["error"],
            serde_json::json!({ "code": "conflict" })
        );
    }

    #[test]
    fn timeline_follow_abort_wire_is_strict_and_applied_advances_exactly_once() {
        let request = follow_abort_request();
        let encoded = serde_json::to_value(&request).unwrap();
        assert_eq!(
            serde_json::from_value::<TimelineFollowAbortRuntimeRequestV1>(encoded.clone()).unwrap(),
            request
        );
        let mut unknown = encoded;
        unknown
            .as_object_mut()
            .unwrap()
            .insert("owner_id".to_string(), serde_json::json!("forged"));
        assert!(serde_json::from_value::<TimelineFollowAbortRuntimeRequestV1>(unknown).is_err());

        let receipt = TimelineFollowAbortRuntimeReceiptV1 {
            operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
            request_id: request.request_id,
            fence_before: request.expected_fence.clone(),
            output_ownership_epoch_after: request.expected_fence.output_ownership_epoch,
            follow_generation_after: request.expected_fence.follow_generation + 1,
            shape_sha256: hash('f'),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        };
        let response = TimelineFollowAbortRuntimeResponseV1::Receipt(receipt.clone());
        assert_eq!(
            serde_json::from_value::<TimelineFollowAbortRuntimeResponseV1>(
                serde_json::to_value(&response).unwrap()
            )
            .unwrap(),
            response
        );
        let mut skipped = receipt.clone();
        skipped.follow_generation_after += 1;
        assert!(skipped.validate().is_err());
        let mut wrong_epoch = receipt;
        wrong_epoch.output_ownership_epoch_after += 1;
        assert!(wrong_epoch.validate().is_err());

        let mut max_request = follow_abort_request();
        max_request.expected_fence.follow_generation = MAX_SAFE_JAVASCRIPT_INTEGER;
        let terminal = TimelineFollowAbortRuntimeReceiptV1 {
            operation_id: TIMELINE_FOLLOW_ABORT_OPERATION_ID.to_string(),
            request_id: max_request.request_id,
            fence_before: max_request.expected_fence.clone(),
            output_ownership_epoch_after: max_request.expected_fence.output_ownership_epoch,
            follow_generation_after: MAX_SAFE_JAVASCRIPT_INTEGER,
            shape_sha256: hash('a'),
            outcome: RuntimeCommandReceiptOutcomeV1::Applied,
        };
        assert!(
            terminal.validate().is_err(),
            "Follow generation MAX cannot report an ABA-unsafe Applied receipt"
        );
        let noop = TimelineFollowAbortRuntimeReceiptV1 {
            outcome: RuntimeCommandReceiptOutcomeV1::NoOp,
            ..terminal
        };
        noop.validate().unwrap();

        let mut unknown_kind = serde_json::to_value(response).unwrap();
        unknown_kind["kind"] = serde_json::json!("future_terminal");
        assert!(
            serde_json::from_value::<TimelineFollowAbortRuntimeResponseV1>(unknown_kind).is_err()
        );
    }

    #[test]
    fn safety_blackout_wire_has_no_release_payload_and_is_strict() {
        let request = SafetyBlackoutEngageRequestV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: 71,
        };
        let encoded = serde_json::to_value(&request).unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "operation_id": SAFETY_BLACKOUT_ENGAGE_OPERATION_ID,
                "request_id": 71
            })
        );
        assert_eq!(
            serde_json::from_value::<SafetyBlackoutEngageRequestV1>(encoded.clone()).unwrap(),
            request
        );
        for (field, value) in [
            ("enabled", serde_json::json!(false)),
            ("target", serde_json::json!("release")),
            ("toggle", serde_json::json!(true)),
        ] {
            let mut forged = encoded.clone();
            forged
                .as_object_mut()
                .unwrap()
                .insert(field.to_string(), value);
            assert!(
                serde_json::from_value::<SafetyBlackoutEngageRequestV1>(forged).is_err(),
                "unexpected safety field {field}"
            );
        }

        let response = SafetyBlackoutEngageResponseV1::Receipt(SafetyBlackoutEngageReceiptV1 {
            operation_id: SAFETY_BLACKOUT_ENGAGE_OPERATION_ID.to_string(),
            request_id: request.request_id,
            shape_sha256: hash('b'),
            audit_sequence: 1,
            outcome: SafetyBlackoutEngageOutcomeV1::Applied,
        });
        let response_json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            serde_json::from_value::<SafetyBlackoutEngageResponseV1>(response_json.clone())
                .unwrap(),
            response
        );
        let mut unknown_outcome = response_json.clone();
        unknown_outcome["result"]["outcome"] = serde_json::json!("released");
        assert!(serde_json::from_value::<SafetyBlackoutEngageResponseV1>(unknown_outcome).is_err());
        let mut unknown_terminal = response_json;
        unknown_terminal["kind"] = serde_json::json!("future_receipt");
        assert!(
            serde_json::from_value::<SafetyBlackoutEngageResponseV1>(unknown_terminal).is_err()
        );

        let mut zero_audit = match response {
            SafetyBlackoutEngageResponseV1::Receipt(receipt) => receipt,
            SafetyBlackoutEngageResponseV1::Rejected(_) => unreachable!(),
        };
        zero_audit.audit_sequence = 0;
        assert!(serde_json::to_value(zero_audit).is_err());
        let mut lossy = request;
        lossy.request_id = MAX_SAFE_JAVASCRIPT_INTEGER + 1;
        assert!(serde_json::to_value(lossy).is_err());
    }

    fn output_fence() -> OutputControlFenceV1 {
        OutputControlFenceV1 {
            process_incarnation: 11,
            session_incarnation: 12,
            project_epoch: 3,
            project_revision: 4,
            project_checkpoint_hash: hash('a'),
            project_publication_generation: 5,
            output_epoch: 6,
            output_generation: 7,
            safety_blackout_epoch: 8,
            safety_blackout_generation: 9,
        }
    }

    fn lease_authority() -> OutputLeaseAuthorityV1 {
        OutputLeaseAuthorityV1 {
            lease_id: "lease-0000000000000001".to_string(),
            generation: 1,
        }
    }

    #[test]
    fn enable_output_lease_result_accepts_only_exact_acquire_or_recover_transitions() {
        let resources = vec![
            OutputControlTargetRoleV1::Lighting,
            OutputControlTargetRoleV1::Video,
        ];
        let acquired = OutputControlLeaseResultV2 {
            authority: lease_authority(),
            resources: resources.clone(),
            phase: OutputLeaseReceiptPhaseV2::HeldActive,
            outcome: OutputLeaseReceiptOutcomeV2::Acquired,
            audit_sequence: 1,
            changes: vec![OutputLeaseReceiptChangeV2 {
                lease_id: lease_authority().lease_id,
                before_generation: None,
                after_generation: Some(1),
                before_resources: Vec::new(),
                after_resources: resources.clone(),
                before_phase: None,
                after_phase: Some(OutputLeaseReceiptPhaseV2::HeldActive),
            }],
        };
        acquired
            .validate_for_operation(OUTPUT_ENABLE_OPERATION_ID)
            .unwrap();

        let mut recovered = acquired.clone();
        recovered.authority.generation = 3;
        recovered.outcome = OutputLeaseReceiptOutcomeV2::Recovered;
        recovered.changes[0].before_generation = Some(2);
        recovered.changes[0].after_generation = Some(3);
        recovered.changes[0].before_resources = resources.clone();
        recovered.changes[0].before_phase = Some(OutputLeaseReceiptPhaseV2::HeldOrphaned);
        recovered
            .validate_for_operation(OUTPUT_ENABLE_OPERATION_ID)
            .unwrap();

        assert!(recovered
            .validate_for_operation(OUTPUT_LEASE_ACQUIRE_OPERATION_ID)
            .is_err());
        assert!(acquired
            .validate_for_operation(OUTPUT_LEASE_RECOVER_OPERATION_ID)
            .is_err());

        let mut forged_recovery = recovered.clone();
        forged_recovery.changes[0].before_phase = Some(OutputLeaseReceiptPhaseV2::HeldActive);
        assert!(forged_recovery
            .validate_for_operation(OUTPUT_ENABLE_OPERATION_ID)
            .is_err());
        let mut forged_acquire = acquired;
        forged_acquire.changes[0].before_generation = Some(1);
        forged_acquire.changes[0].before_resources = resources;
        forged_acquire.changes[0].before_phase = Some(OutputLeaseReceiptPhaseV2::HeldOrphaned);
        assert!(forged_acquire
            .validate_for_operation(OUTPUT_ENABLE_OPERATION_ID)
            .is_err());
    }

    #[test]
    fn r4_command_wire_is_strict_and_exactly_bound() {
        let action = OutputControlActionV2::TakeOverStandby {
            force: true,
            standby_session_id: "primary-session-1".to_string(),
            standby_generation: 91,
            lease: lease_authority(),
        };
        let command = OutputControlCommandRequestV2 {
            operation_id: OUTPUT_STANDBY_TAKEOVER_OPERATION_ID.to_string(),
            request_id: 23,
            expected_fence: output_fence(),
            action,
        };
        command.validate().unwrap();
        let command_json = serde_json::to_value(&command).unwrap();
        assert_eq!(
            serde_json::from_value::<OutputControlCommandRequestV2>(command_json.clone()).unwrap(),
            command
        );
        let mut wrong_operation = command.clone();
        wrong_operation.operation_id = OUTPUT_OWNERSHIP_ARM_OPERATION_ID.to_string();
        assert!(serde_json::to_value(wrong_operation).is_err());
        let mut wrong_generation = command.clone();
        wrong_generation.expected_fence.output_generation = 0;
        assert!(serde_json::to_value(wrong_generation).is_err());
        let mut legacy_shape = command_json.clone();
        legacy_shape["consent_token"] = serde_json::json!("AAAAAAAAAAAAAAAAAAAAAA");
        assert!(serde_json::from_value::<OutputControlCommandRequestV2>(legacy_shape).is_err());

        let fence_before = output_fence();
        let mut fence_after = fence_before.clone();
        fence_after.safety_blackout_generation += 1;
        let response = OutputControlResponseV2::Receipt(Box::new(OutputControlReceiptV2 {
            operation_id: OUTPUT_BLACKOUT_RELEASE_OPERATION_ID.to_string(),
            request_id: 24,
            shape_sha256: hash('d'),
            argument_fingerprint: hash('e'),
            audit_sequence: 1,
            fence_before,
            fence_after,
            outcome: OutputControlReceiptOutcomeV2::Applied,
            lease_result: Some(OutputControlLeaseResultV2 {
                authority: lease_authority(),
                resources: vec![
                    OutputControlTargetRoleV1::Lighting,
                    OutputControlTargetRoleV1::Video,
                ],
                phase: OutputLeaseReceiptPhaseV2::HeldActive,
                outcome: OutputLeaseReceiptOutcomeV2::Authorized,
                audit_sequence: 1,
                changes: vec![OutputLeaseReceiptChangeV2 {
                    lease_id: lease_authority().lease_id,
                    before_generation: Some(1),
                    after_generation: Some(1),
                    before_resources: vec![
                        OutputControlTargetRoleV1::Lighting,
                        OutputControlTargetRoleV1::Video,
                    ],
                    after_resources: vec![
                        OutputControlTargetRoleV1::Lighting,
                        OutputControlTargetRoleV1::Video,
                    ],
                    before_phase: Some(OutputLeaseReceiptPhaseV2::HeldActive),
                    after_phase: Some(OutputLeaseReceiptPhaseV2::HeldActive),
                }],
            }),
        }));
        let response_json = serde_json::to_value(&response).unwrap();
        assert_eq!(
            serde_json::from_value::<OutputControlResponseV2>(response_json.clone()).unwrap(),
            response
        );
        let mut takeover_orphan = match response.clone() {
            OutputControlResponseV2::Receipt(receipt) => receipt,
            OutputControlResponseV2::Rejected(_) => unreachable!(),
        };
        takeover_orphan.operation_id = OUTPUT_STANDBY_TAKEOVER_OPERATION_ID.to_string();
        let takeover_lease = takeover_orphan.lease_result.as_mut().unwrap();
        takeover_lease.authority.generation = 2;
        takeover_lease.phase = OutputLeaseReceiptPhaseV2::HeldOrphaned;
        takeover_lease.outcome = OutputLeaseReceiptOutcomeV2::ProjectOrphaned;
        takeover_lease.changes[0].after_generation = Some(2);
        takeover_lease.changes[0].after_phase = Some(OutputLeaseReceiptPhaseV2::HeldOrphaned);
        assert!(serde_json::to_value(takeover_orphan).is_ok());
        let mut mismatched_authority = response_json.clone();
        mismatched_authority["receipt"]["lease_result"]["authority"]["generation"] =
            serde_json::json!(2);
        assert!(serde_json::from_value::<OutputControlResponseV2>(mismatched_authority).is_err());
        let mut mismatched_change = response_json.clone();
        mismatched_change["receipt"]["lease_result"]["changes"][0]["lease_id"] =
            serde_json::json!("lease-0000000000000002");
        assert!(serde_json::from_value::<OutputControlResponseV2>(mismatched_change).is_err());
        let mut mismatched_resources = response_json.clone();
        mismatched_resources["receipt"]["lease_result"]["resources"] =
            serde_json::json!(["lighting"]);
        assert!(serde_json::from_value::<OutputControlResponseV2>(mismatched_resources).is_err());
        let mut extra_change = response_json.clone();
        extra_change["receipt"]["lease_result"]["changes"] = serde_json::json!([
            {
                "lease_id": "lease-0000000000000001",
                "before_generation": 1,
                "after_generation": 1,
                "before_resources": ["lighting", "video"],
                "after_resources": ["lighting", "video"],
                "before_phase": "held_active",
                "after_phase": "held_active"
            },
            {
                "lease_id": "lease-0000000000000001",
                "before_generation": 1,
                "after_generation": 1,
                "before_resources": ["lighting", "video"],
                "after_resources": ["lighting", "video"],
                "before_phase": "held_active",
                "after_phase": "held_active"
            }
        ]);
        assert!(serde_json::from_value::<OutputControlResponseV2>(extra_change).is_err());
        let mut unknown_terminal = response_json;
        unknown_terminal["type"] = serde_json::json!("future_terminal");
        assert!(serde_json::from_value::<OutputControlResponseV2>(unknown_terminal).is_err());

        let mut invalid_noop = match response {
            OutputControlResponseV2::Receipt(receipt) => receipt,
            OutputControlResponseV2::Rejected(_) => unreachable!(),
        };
        invalid_noop.outcome = OutputControlReceiptOutcomeV2::NoOp;
        assert!(serde_json::to_value(invalid_noop).is_err());

        let rejection = OutputControlResponseV2::Rejected(OutputControlRejectionV2 {
            operation_id: OUTPUT_OWNERSHIP_ARM_OPERATION_ID.to_string(),
            request_id: 25,
            error: OutputControlErrorCodeV2::Busy,
        });
        let rejection_json = serde_json::to_value(&rejection).unwrap();
        assert_eq!(
            serde_json::from_value::<OutputControlResponseV2>(rejection_json.clone()).unwrap(),
            rejection
        );
        let mut unknown_error = rejection_json;
        unknown_error["rejection"]["error"] = serde_json::json!("future_error");
        assert!(serde_json::from_value::<OutputControlResponseV2>(unknown_error).is_err());
    }

    #[test]
    fn output_lease_authority_is_strict_opaque_and_operation_shapes_are_distinct() {
        let authority = lease_authority();
        assert!(authority.validate().is_ok());
        assert_eq!(authority.lease_id.len(), 22);
        assert!(
            serde_json::from_value::<OutputLeaseAuthorityV1>(serde_json::json!({
                "lease_id": "lease-0000000000000001",
                "generation": 1,
                "numeric": 1
            }))
            .is_err()
        );
        for lease_id in [
            "lease-000000000000001",
            "Lease-0000000000000001",
            "lease-000000000000000g",
            "lease-0000000000000000",
        ] {
            assert!(
                serde_json::from_value::<OutputLeaseAuthorityV1>(serde_json::json!({
                    "lease_id": lease_id,
                    "generation": 1
                }))
                .is_err()
            );
        }
        assert!(
            serde_json::from_value::<OutputLeaseAuthorityV1>(serde_json::json!({
                "lease_id": "lease-0000000000000001",
                "generation": 0
            }))
            .is_err()
        );
        let acquire = OutputControlActionV2::AcquireLease {
            role: OutputControlTargetRoleV1::Lighting,
        };
        let renew = OutputControlActionV2::RenewLease { lease: authority };
        let mut acquire_shape = Vec::new();
        acquire.append_canonical_bytes(&mut acquire_shape).unwrap();
        let mut renew_shape = Vec::new();
        renew.append_canonical_bytes(&mut renew_shape).unwrap();
        assert_ne!(acquire_shape, renew_shape);
        assert_ne!(acquire.operation_id(), renew.operation_id());
        assert_ne!(
            acquire.operation_id().as_bytes(),
            renew.operation_id().as_bytes()
        );

        let enable = OutputControlActionV2::EnableOutput;
        assert_eq!(enable.operation_id(), OUTPUT_ENABLE_OPERATION_ID);
        let mut enable_shape = Vec::new();
        enable.append_canonical_bytes(&mut enable_shape).unwrap();
        assert_eq!(enable_shape, vec![9]);
        let authority = lease_authority();
        let windows_device_label = DisplayOutputSpecV2 {
            label: r"\\.\DISPLAY2".to_string(),
            monitor_identity: "a".repeat(64),
            monitor_index: 1,
            width: 1920,
            height: 1080,
            fullscreen: true,
        };
        assert_eq!(
            windows_device_label.validate(),
            Err(OutputControlValidationErrorV1::InvalidDisplayOutputSpec),
            "Windows device paths stay rejected on the wire"
        );
        let safe_display_label = DisplayOutputSpecV2 {
            label: "Display 2".to_string(),
            ..windows_device_label
        };
        assert!(safe_display_label.validate().is_ok());
        let legacy_shapes = [
            (
                0,
                OutputControlActionV2::Arm {
                    role: OutputControlTargetRoleV1::Lighting,
                    lease: authority.clone(),
                },
            ),
            (
                1,
                OutputControlActionV2::ReleaseBlackout {
                    lease: authority.clone(),
                },
            ),
            (
                2,
                OutputControlActionV2::TakeOverStandby {
                    force: false,
                    standby_session_id: "primary-session-1".to_string(),
                    standby_generation: 1,
                    lease: authority.clone(),
                },
            ),
            (
                3,
                OutputControlActionV2::AcquireLease {
                    role: OutputControlTargetRoleV1::Both,
                },
            ),
            (
                4,
                OutputControlActionV2::RenewLease {
                    lease: authority.clone(),
                },
            ),
            (
                5,
                OutputControlActionV2::RecoverLease {
                    lease: authority.clone(),
                },
            ),
            (
                6,
                OutputControlActionV2::RelinquishOutputLease {
                    lease: authority.clone(),
                },
            ),
            (
                7,
                OutputControlActionV2::ForceTransferLease {
                    lease: authority.clone(),
                },
            ),
            (
                8,
                OutputControlActionV2::AddDisplay {
                    spec: DisplayOutputSpecV2 {
                        label: "display".to_string(),
                        monitor_identity:
                            "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
                                .to_string(),
                        monitor_index: 0,
                        width: 1920,
                        height: 1080,
                        fullscreen: true,
                    },
                    lease: authority,
                },
            ),
        ];
        for (expected, action) in &legacy_shapes {
            let mut shape = Vec::new();
            action.append_canonical_bytes(&mut shape).unwrap();
            assert_eq!(shape.first(), Some(expected));
        }
        // The new physical-window operation is append-only: its discriminant
        // is 10 and the frozen 0..=9 golden bytes above never move.
        let set_window = OutputControlActionV2::SetDisplayWindowOpen {
            output_id: 42,
            open: true,
            lease: lease_authority(),
        };
        let mut set_window_shape = Vec::new();
        set_window
            .append_canonical_bytes(&mut set_window_shape)
            .unwrap();
        assert_eq!(
            set_window.operation_id(),
            OUTPUT_DISPLAY_WINDOW_SET_OPEN_OPERATION_ID
        );
        assert_eq!(set_window_shape.first(), Some(&10));
        assert_eq!(set_window_shape[1..9], 42_u64.to_be_bytes());
        assert!(
            serde_json::from_value::<OutputControlActionV2>(serde_json::json!({
                "kind": "set_display_window_open",
                "output_id": 0,
                "open": true,
                "lease": serde_json::to_value(lease_authority()).unwrap(),
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<OutputControlActionV2>(serde_json::json!({
                "kind": "set_display_window_open",
                "output_id": 42,
                "open": true,
                "lease": serde_json::to_value(lease_authority()).unwrap(),
                "unexpected": true,
            }))
            .is_err()
        );
        let enable_json = serde_json::to_value(&enable).unwrap();
        assert_eq!(enable_json, serde_json::json!({ "kind": "enable_output" }));
        assert_eq!(
            serde_json::from_value::<OutputControlActionV2>(enable_json.clone()).unwrap(),
            enable
        );
        let mut forged_enable = enable_json;
        forged_enable["lease"] = serde_json::json!({
            "lease_id": "lease-0000000000000001",
            "generation": 1
        });
        assert!(serde_json::from_value::<OutputControlActionV2>(forged_enable).is_err());
        let enable_request = OutputControlCommandRequestV2 {
            operation_id: OUTPUT_ENABLE_OPERATION_ID.to_string(),
            request_id: 77,
            expected_fence: output_fence(),
            action: enable,
        };
        assert_eq!(
            serde_json::from_value::<OutputControlCommandRequestV2>(
                serde_json::to_value(&enable_request).unwrap()
            )
            .unwrap(),
            enable_request
        );

        // The v2 operation/schema boundary is explicit: every retired v1 and
        // unknown future v3 mutating operation is rejected even when its
        // payload otherwise has a valid action shape.
        for (_, action) in &legacy_shapes {
            for suffix in ["v1", "v3"] {
                let retired_operation = action.operation_id().replace(".v2", &format!(".{suffix}"));
                let retired_request = OutputControlCommandRequestV2 {
                    operation_id: action.operation_id().to_string(),
                    request_id: 78,
                    expected_fence: output_fence(),
                    action: action.clone(),
                };
                let mut retired_json = serde_json::to_value(retired_request).unwrap();
                retired_json["operation_id"] = serde_json::json!(retired_operation);
                assert!(
                    serde_json::from_value::<OutputControlCommandRequestV2>(retired_json).is_err()
                );
            }
        }
        for suffix in ["v1", "v3"] {
            let retired_operation =
                OUTPUT_ENABLE_OPERATION_ID.replace(".v2", &format!(".{suffix}"));
            let retired_request = OutputControlCommandRequestV2 {
                operation_id: OUTPUT_ENABLE_OPERATION_ID.to_string(),
                request_id: 79,
                expected_fence: output_fence(),
                action: OutputControlActionV2::EnableOutput,
            };
            let mut retired_json = serde_json::to_value(retired_request).unwrap();
            retired_json["operation_id"] = serde_json::json!(retired_operation);
            assert!(serde_json::from_value::<OutputControlCommandRequestV2>(retired_json).is_err());
        }
    }
}
