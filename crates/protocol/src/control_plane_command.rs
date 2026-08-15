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
#[derive(Debug, Clone, PartialEq, Eq)]
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

        let unknown = format!("{encoded}").replacen('}', ",\"secret\":true}", 1);
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
}
