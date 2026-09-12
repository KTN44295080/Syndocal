//! Versioned ShowClock wire contracts and the fail-closed peer admission core.
//!
//! The module is deliberately transport- and output-neutral.  A later adapter
//! may carry these envelopes over the approved wired-LAN transport, but it
//! cannot bypass the session, project/media, generation, replay, or manual
//! fence checks here.  No function in this module grants physical output.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const SHOW_CLOCK_SCHEMA_VERSION: u16 = 1;
pub const SHOW_CLOCK_PROTOCOL_VERSION: u16 = 1;
pub const SHOW_CLOCK_TICKS_PER_SECOND: u64 = 1_000_000;
pub const SHOW_CLOCK_MAX_ID_BYTES: usize = 128;
pub const SHOW_CLOCK_MAX_EXPIRY_US: u64 = 5_000_000;
pub const SHOW_CLOCK_DEFAULT_MAX_SLEW_US_PER_SAMPLE: u64 = 1_000;
/// Maximum number of action identities retained by one receiver session.
/// Entries are never evicted: exhaustion fails closed until the session is
/// replaced by its owner.
pub const SHOW_CLOCK_MAX_ACCEPTED_ACTIONS: usize = 4_096;

const SAMPLE_DOMAIN: &[u8] = b"syndocal.show-clock.sample.v1\0";
const ACTION_DOMAIN: &[u8] = b"syndocal.show-clock.action.v1\0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowClockValidationError {
    EmptyOrInvalidId,
    InvalidSchemaVersion,
    InvalidProtocolVersion,
    ZeroGeneration,
    ZeroSequence,
    InvalidBpm,
    InvalidBeatPhase,
    InvalidExpiry,
    ZeroNonce,
    ZeroHash,
    InvalidActionId,
    InvalidTargetTime,
    ActionPayloadRequired,
    ActionPayloadUnexpected,
    InvalidActionPayload,
    InvalidAuthentication,
    SessionMismatch,
    SenderMismatch,
    ProjectHashMismatch,
    MediaHashMismatch,
    ClockGenerationMismatch,
    FencingGenerationMismatch,
    Replay,
    Reordered,
    ActionConflict,
    DuplicateAction,
    HoldRequired,
    OperatorConfirmationRequired,
    ReArmGenerationNotAdvanced,
    ReArmClockGenerationRewound,
    ActionAdmissionCapacityExceeded,
    InvalidEstimatorConfig,
    ActionOutsideHorizon,
    ActionScheduleCapacityExceeded,
    OutputNotArmed,
    OutputGenerationMismatch,
    OutputOwnerMismatch,
    AlreadyArmed,
}

impl std::fmt::Display for ShowClockValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmptyOrInvalidId => "empty or invalid ShowClock identifier",
            Self::InvalidSchemaVersion => "unsupported ShowClock schema version",
            Self::InvalidProtocolVersion => "unsupported ShowClock protocol version",
            Self::ZeroGeneration => "ShowClock generation must be non-zero",
            Self::ZeroSequence => "ShowClock sequence must be non-zero",
            Self::InvalidBpm => "ShowClock BPM is outside the supported range",
            Self::InvalidBeatPhase => "ShowClock beat phase is outside the supported range",
            Self::InvalidExpiry => "ShowClock expiry is outside the supported range",
            Self::ZeroNonce => "ShowClock nonce must be non-zero",
            Self::ZeroHash => "ShowClock project/media hash must be non-zero",
            Self::InvalidActionId => "ShowClock action id must be non-zero",
            Self::InvalidTargetTime => "ShowClock action target time is invalid",
            Self::ActionPayloadRequired => "ShowClock action requires a typed payload",
            Self::ActionPayloadUnexpected => "ShowClock action does not accept a typed payload",
            Self::InvalidActionPayload => "ShowClock action payload is invalid",
            Self::InvalidAuthentication => "ShowClock authentication tag is invalid",
            Self::SessionMismatch => "ShowClock session does not match the paired session",
            Self::SenderMismatch => "ShowClock sender does not match the paired peer",
            Self::ProjectHashMismatch => "ShowClock project hash does not match",
            Self::MediaHashMismatch => "ShowClock media hash does not match",
            Self::ClockGenerationMismatch => "ShowClock clock generation does not match",
            Self::FencingGenerationMismatch => "ShowClock fencing generation does not match",
            Self::Replay => "ShowClock sequence was already accepted",
            Self::Reordered => "ShowClock sequence is older than the accepted sequence",
            Self::ActionConflict => "ShowClock action id was reused with different content",
            Self::DuplicateAction => "ShowClock action was already accepted",
            Self::HoldRequired => "ShowClock peer must be in manual Hold",
            Self::OperatorConfirmationRequired => "explicit operator confirmation is required",
            Self::ReArmGenerationNotAdvanced => "re-arm fencing generation did not advance",
            Self::ReArmClockGenerationRewound => "re-arm clock generation moved backwards",
            Self::ActionAdmissionCapacityExceeded => {
                "ShowClock action admission capacity is exhausted"
            }
            Self::InvalidEstimatorConfig => "ShowClock estimator configuration is invalid",
            Self::ActionOutsideHorizon => "ShowClock action is outside the scheduling horizon",
            Self::ActionScheduleCapacityExceeded => {
                "ShowClock action schedule capacity is exhausted"
            }
            Self::OutputNotArmed => "ShowClock output owner is not armed",
            Self::OutputGenerationMismatch => "ShowClock output generation context does not match",
            Self::OutputOwnerMismatch => "ShowClock output owner does not match",
            Self::AlreadyArmed => "ShowClock peer is already armed",
        })
    }
}

impl std::error::Error for ShowClockValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ShowClockSessionId(String);

impl ShowClockSessionId {
    pub fn new(value: impl Into<String>) -> Result<Self, ShowClockValidationError> {
        let value = value.into();
        validate_ascii_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ShowClockSessionId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
#[serde(transparent)]
pub struct ShowClockNodeId(String);

impl ShowClockNodeId {
    pub fn new(value: impl Into<String>) -> Result<Self, ShowClockValidationError> {
        let value = value.into();
        validate_ascii_id(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for ShowClockNodeId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShowClockHash(pub [u8; 32]);

impl ShowClockHash {
    pub const ZERO: Self = Self([0; 32]);

    pub fn is_zero(self) -> bool {
        self == Self::ZERO
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ShowClockNonce(pub [u8; 16]);

impl ShowClockNonce {
    fn is_zero(self) -> bool {
        self.0 == [0; 16]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShowClockSource {
    ShowClock = 0,
    MidiClock = 1,
    MidiTimecode = 2,
    Ltc = 3,
    AbletonLink = 4,
    Manual = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShowTransportState {
    Stopped = 0,
    Playing = 1,
    Paused = 2,
    Holding = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShowClockPeerState {
    Locked = 0,
    Acquiring = 1,
    Hold = 2,
    Stale = 3,
    Fault = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShowClockActionKind {
    Go = 0,
    Stop = 1,
    Back = 2,
    Release = 3,
    Blackout = 4,
    Take = 5,
    ClipLaunch = 6,
    Transition = 7,
    TimelineJump = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum ShowClockLatePolicy {
    ExecuteImmediately = 0,
    Drop = 1,
    Hold = 2,
}

/// Typed target data for actions that need an authored/runtime identity.
///
/// The payload is part of the authenticated canonical action bytes.  It is
/// optional at the struct boundary so legacy v1 actions retain their exact
/// canonical bytes; action kinds that require a target reject `None` during
/// shape validation. Unknown payload kinds or malformed fields are rejected
/// by serde and the explicit action-kind binding below.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ShowClockActionPayload {
    CueRelease {
        cue_id: u64,
    },
    VideoTake {
        target_layer_id: u64,
        fade_ms: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preview_position_ms: Option<u64>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        preview_speed_milli: Option<u32>,
    },
    ClipLaunch {
        layer_id: u64,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        slot_id: Option<crate::VideoClipSlotId>,
        #[serde(default)]
        transition_kind: crate::VideoClipTakeKind,
        #[serde(default)]
        transition_duration_ms: u64,
    },
    TimelineJump {
        position_ms: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockSample {
    pub schema_version: u16,
    pub protocol_version: u16,
    pub session_id: ShowClockSessionId,
    pub sender: ShowClockNodeId,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub sequence: u64,
    /// Monotonic timestamp in the sender's local domain.  It is an ordering
    /// fact, not a value that may be directly compared with the receiver's
    /// monotonic clock.
    pub sender_monotonic_us: u64,
    pub show_time_us: u64,
    /// BPM in thousandths avoids floating-point wire ambiguity.
    pub bpm_milli: u32,
    /// Beat phase in millionths, inclusive at 0 and exclusive at 1 beat.
    pub beat_phase_ppm: u32,
    pub transport: ShowTransportState,
    pub source: ShowClockSource,
    pub project_hash: ShowClockHash,
    pub media_hash: ShowClockHash,
    pub expires_after_us: u64,
    pub nonce: ShowClockNonce,
}

impl ShowClockSample {
    pub fn validate_shape(&self) -> Result<(), ShowClockValidationError> {
        if self.schema_version != SHOW_CLOCK_SCHEMA_VERSION {
            return Err(ShowClockValidationError::InvalidSchemaVersion);
        }
        if self.protocol_version != SHOW_CLOCK_PROTOCOL_VERSION {
            return Err(ShowClockValidationError::InvalidProtocolVersion);
        }
        validate_ascii_id(self.session_id.as_str())?;
        validate_ascii_id(self.sender.as_str())?;
        if self.clock_generation == 0 || self.fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        if self.sequence == 0 {
            return Err(ShowClockValidationError::ZeroSequence);
        }
        if !(30_000..=300_000).contains(&self.bpm_milli) {
            return Err(ShowClockValidationError::InvalidBpm);
        }
        if self.beat_phase_ppm >= 1_000_000 {
            return Err(ShowClockValidationError::InvalidBeatPhase);
        }
        if self.expires_after_us == 0 || self.expires_after_us > SHOW_CLOCK_MAX_EXPIRY_US {
            return Err(ShowClockValidationError::InvalidExpiry);
        }
        if self.nonce.is_zero() {
            return Err(ShowClockValidationError::ZeroNonce);
        }
        if self.project_hash.is_zero() || self.media_hash.is_zero() {
            return Err(ShowClockValidationError::ZeroHash);
        }
        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, ShowClockValidationError> {
        self.validate_shape()?;
        let mut output = Vec::with_capacity(256);
        output.extend_from_slice(SAMPLE_DOMAIN);
        append_u16(&mut output, self.schema_version);
        append_u16(&mut output, self.protocol_version);
        append_id(&mut output, self.session_id.as_str())?;
        append_id(&mut output, self.sender.as_str())?;
        append_u64(&mut output, self.clock_generation);
        append_u64(&mut output, self.fencing_generation);
        append_u64(&mut output, self.sequence);
        append_u64(&mut output, self.sender_monotonic_us);
        append_u64(&mut output, self.show_time_us);
        append_u32(&mut output, self.bpm_milli);
        append_u32(&mut output, self.beat_phase_ppm);
        output.push(self.transport as u8);
        output.push(self.source as u8);
        output.extend_from_slice(&self.project_hash.0);
        output.extend_from_slice(&self.media_hash.0);
        append_u64(&mut output, self.expires_after_us);
        output.extend_from_slice(&self.nonce.0);
        Ok(output)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedShowClockSample {
    pub body: ShowClockSample,
    pub authentication_tag: [u8; 32],
}

impl AuthenticatedShowClockSample {
    pub fn sign(body: ShowClockSample, key: &[u8; 32]) -> Result<Self, ShowClockValidationError> {
        let canonical = body.canonical_bytes()?;
        Ok(Self {
            body,
            authentication_tag: hmac_sha256(key, &canonical),
        })
    }

    pub fn verify(&self, key: &[u8; 32]) -> Result<(), ShowClockValidationError> {
        self.verify_and_canonical(key).map(|_| ())
    }

    fn verify_and_canonical(&self, key: &[u8; 32]) -> Result<Vec<u8>, ShowClockValidationError> {
        let canonical = self.body.canonical_bytes()?;
        if constant_time_eq(&self.authentication_tag, &hmac_sha256(key, &canonical)) {
            Ok(canonical)
        } else {
            Err(ShowClockValidationError::InvalidAuthentication)
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct ShowClockAuthenticationKey([u8; 32]);

impl std::fmt::Debug for ShowClockAuthenticationKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("[redacted]")
    }
}

impl ShowClockAuthenticationKey {
    fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ShowClockPeerContext {
    expected_session: ShowClockSessionId,
    expected_sender: ShowClockNodeId,
    expected_project_hash: ShowClockHash,
    expected_media_hash: ShowClockHash,
    expected_clock_generation: u64,
    expected_fencing_generation: u64,
    key: ShowClockAuthenticationKey,
}

impl ShowClockPeerContext {
    fn new(
        expected_session: ShowClockSessionId,
        expected_sender: ShowClockNodeId,
        expected_project_hash: ShowClockHash,
        expected_media_hash: ShowClockHash,
        expected_clock_generation: u64,
        expected_fencing_generation: u64,
        key: [u8; 32],
    ) -> Result<Self, ShowClockValidationError> {
        validate_ascii_id(expected_session.as_str())?;
        validate_ascii_id(expected_sender.as_str())?;
        if expected_clock_generation == 0 || expected_fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        if expected_project_hash.is_zero() || expected_media_hash.is_zero() {
            return Err(ShowClockValidationError::ZeroHash);
        }
        Ok(Self {
            expected_session,
            expected_sender,
            expected_project_hash,
            expected_media_hash,
            expected_clock_generation,
            expected_fencing_generation,
            key: ShowClockAuthenticationKey(key),
        })
    }

    fn validate_sample(&self, sample: &ShowClockSample) -> Result<(), ShowClockValidationError> {
        if sample.session_id != self.expected_session {
            return Err(ShowClockValidationError::SessionMismatch);
        }
        if sample.sender != self.expected_sender {
            return Err(ShowClockValidationError::SenderMismatch);
        }
        if sample.project_hash != self.expected_project_hash {
            return Err(ShowClockValidationError::ProjectHashMismatch);
        }
        if sample.media_hash != self.expected_media_hash {
            return Err(ShowClockValidationError::MediaHashMismatch);
        }
        if sample.clock_generation != self.expected_clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if sample.fencing_generation != self.expected_fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }
        Ok(())
    }

    fn validate_action(&self, action: &ShowClockAction) -> Result<(), ShowClockValidationError> {
        if action.session_id != self.expected_session {
            return Err(ShowClockValidationError::SessionMismatch);
        }
        if action.sender != self.expected_sender {
            return Err(ShowClockValidationError::SenderMismatch);
        }
        if action.project_hash != self.expected_project_hash {
            return Err(ShowClockValidationError::ProjectHashMismatch);
        }
        if action.media_hash != self.expected_media_hash {
            return Err(ShowClockValidationError::MediaHashMismatch);
        }
        if action.clock_generation != self.expected_clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if action.fencing_generation != self.expected_fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockPeerValidator {
    peer: ShowClockPeerContext,
    last_sequence: u64,
    last_show_time_us: Option<u64>,
    last_sender_monotonic_us: Option<u64>,
}

impl ShowClockPeerValidator {
    pub fn new(
        expected_session: ShowClockSessionId,
        expected_sender: ShowClockNodeId,
        expected_project_hash: ShowClockHash,
        expected_media_hash: ShowClockHash,
        expected_clock_generation: u64,
        expected_fencing_generation: u64,
        key: [u8; 32],
    ) -> Result<Self, ShowClockValidationError> {
        let peer = ShowClockPeerContext::new(
            expected_session,
            expected_sender,
            expected_project_hash,
            expected_media_hash,
            expected_clock_generation,
            expected_fencing_generation,
            key,
        )?;
        Ok(Self {
            peer,
            last_sequence: 0,
            last_show_time_us: None,
            last_sender_monotonic_us: None,
        })
    }

    pub fn accept(
        &mut self,
        sample: &AuthenticatedShowClockSample,
    ) -> Result<(), ShowClockValidationError> {
        sample.verify_and_canonical(self.peer.key.as_bytes())?;
        self.peer.validate_sample(&sample.body)?;
        if sample.body.sequence <= self.last_sequence {
            return Err(if sample.body.sequence == self.last_sequence {
                ShowClockValidationError::Replay
            } else {
                ShowClockValidationError::Reordered
            });
        }
        if self
            .last_sender_monotonic_us
            .is_some_and(|last| sample.body.sender_monotonic_us < last)
        {
            return Err(ShowClockValidationError::Reordered);
        }
        if self
            .last_show_time_us
            .is_some_and(|last| sample.body.show_time_us < last)
        {
            return Err(ShowClockValidationError::Reordered);
        }
        self.last_sequence = sample.body.sequence;
        self.last_show_time_us = Some(sample.body.show_time_us);
        self.last_sender_monotonic_us = Some(sample.body.sender_monotonic_us);
        Ok(())
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
    }

    pub fn last_sender_monotonic_us(&self) -> Option<u64> {
        self.last_sender_monotonic_us
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShowClockAction {
    pub schema_version: u16,
    pub protocol_version: u16,
    pub session_id: ShowClockSessionId,
    pub sender: ShowClockNodeId,
    pub action_id: [u8; 16],
    pub sequence: u64,
    pub clock_generation: u64,
    pub fencing_generation: u64,
    pub target_show_time_us: u64,
    pub action: ShowClockActionKind,
    pub late_policy: ShowClockLatePolicy,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<ShowClockActionPayload>,
    pub project_hash: ShowClockHash,
    pub media_hash: ShowClockHash,
}

impl ShowClockAction {
    pub fn validate_shape(&self) -> Result<(), ShowClockValidationError> {
        if self.schema_version != SHOW_CLOCK_SCHEMA_VERSION {
            return Err(ShowClockValidationError::InvalidSchemaVersion);
        }
        if self.protocol_version != SHOW_CLOCK_PROTOCOL_VERSION {
            return Err(ShowClockValidationError::InvalidProtocolVersion);
        }
        validate_ascii_id(self.session_id.as_str())?;
        validate_ascii_id(self.sender.as_str())?;
        if self.sequence == 0 {
            return Err(ShowClockValidationError::ZeroSequence);
        }
        if self.clock_generation == 0 || self.fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        if self.action_id == [0; 16] {
            return Err(ShowClockValidationError::InvalidActionId);
        }
        if self.project_hash.is_zero() || self.media_hash.is_zero() {
            return Err(ShowClockValidationError::ZeroHash);
        }
        if self.target_show_time_us == 0 {
            return Err(ShowClockValidationError::InvalidTargetTime);
        }
        validate_action_payload(self.action, self.payload.as_ref())?;
        Ok(())
    }

    fn canonical_bytes(&self) -> Result<Vec<u8>, ShowClockValidationError> {
        self.validate_shape()?;
        let mut output = Vec::with_capacity(192);
        output.extend_from_slice(ACTION_DOMAIN);
        append_u16(&mut output, self.schema_version);
        append_u16(&mut output, self.protocol_version);
        append_id(&mut output, self.session_id.as_str())?;
        append_id(&mut output, self.sender.as_str())?;
        output.extend_from_slice(&self.action_id);
        append_u64(&mut output, self.sequence);
        append_u64(&mut output, self.clock_generation);
        append_u64(&mut output, self.fencing_generation);
        append_u64(&mut output, self.target_show_time_us);
        output.push(self.action as u8);
        output.push(self.late_policy as u8);
        if let Some(payload) = self.payload.as_ref() {
            append_action_payload(&mut output, payload)?;
        }
        output.extend_from_slice(&self.project_hash.0);
        output.extend_from_slice(&self.media_hash.0);
        Ok(output)
    }
}

fn validate_action_payload(
    action: ShowClockActionKind,
    payload: Option<&ShowClockActionPayload>,
) -> Result<(), ShowClockValidationError> {
    let expected = match action {
        ShowClockActionKind::Release
        | ShowClockActionKind::Take
        | ShowClockActionKind::ClipLaunch
        | ShowClockActionKind::Transition
        | ShowClockActionKind::TimelineJump => true,
        ShowClockActionKind::Go
        | ShowClockActionKind::Stop
        | ShowClockActionKind::Back
        | ShowClockActionKind::Blackout => false,
    };
    if expected && payload.is_none() {
        return Err(ShowClockValidationError::ActionPayloadRequired);
    }
    if !expected && payload.is_some() {
        return Err(ShowClockValidationError::ActionPayloadUnexpected);
    }
    let Some(payload) = payload else {
        return Ok(());
    };
    let compatible = matches!(
        (action, payload),
        (
            ShowClockActionKind::Release,
            ShowClockActionPayload::CueRelease { .. }
        ) | (
            ShowClockActionKind::Take,
            ShowClockActionPayload::VideoTake { .. }
        ) | (
            ShowClockActionKind::ClipLaunch | ShowClockActionKind::Transition,
            ShowClockActionPayload::ClipLaunch { .. }
        ) | (
            ShowClockActionKind::TimelineJump,
            ShowClockActionPayload::TimelineJump { .. }
        )
    );
    if !compatible {
        return Err(ShowClockValidationError::InvalidActionPayload);
    }
    match payload {
        ShowClockActionPayload::CueRelease { cue_id } => {
            if *cue_id == 0 {
                return Err(ShowClockValidationError::InvalidActionPayload);
            }
        }
        ShowClockActionPayload::VideoTake {
            target_layer_id, ..
        } => {
            if *target_layer_id == 0 {
                return Err(ShowClockValidationError::InvalidActionPayload);
            }
        }
        ShowClockActionPayload::ClipLaunch { layer_id, .. } => {
            if *layer_id == 0 {
                return Err(ShowClockValidationError::InvalidActionPayload);
            }
        }
        ShowClockActionPayload::TimelineJump { .. } => {}
    }
    Ok(())
}

fn append_action_payload(
    output: &mut Vec<u8>,
    payload: &ShowClockActionPayload,
) -> Result<(), ShowClockValidationError> {
    match payload {
        ShowClockActionPayload::CueRelease { cue_id } => {
            output.push(1);
            append_u64(output, *cue_id);
        }
        ShowClockActionPayload::VideoTake {
            target_layer_id,
            fade_ms,
            preview_position_ms,
            preview_speed_milli,
        } => {
            output.push(2);
            append_u64(output, *target_layer_id);
            append_u64(output, *fade_ms);
            append_optional_u64(output, *preview_position_ms);
            match preview_speed_milli {
                Some(speed) => {
                    output.push(1);
                    append_u32(output, *speed);
                }
                None => output.push(0),
            }
        }
        ShowClockActionPayload::ClipLaunch {
            layer_id,
            slot_id,
            transition_kind,
            transition_duration_ms,
        } => {
            output.push(3);
            append_u64(output, *layer_id);
            match slot_id {
                Some(slot_id) => {
                    output.push(1);
                    append_u64(output, slot_id.0);
                }
                None => output.push(0),
            }
            output.push(video_clip_take_kind_tag(*transition_kind));
            append_u64(output, *transition_duration_ms);
        }
        ShowClockActionPayload::TimelineJump { position_ms } => {
            output.push(4);
            append_u64(output, *position_ms);
        }
    }
    Ok(())
}

fn append_optional_u64(output: &mut Vec<u8>, value: Option<u64>) {
    match value {
        Some(value) => {
            output.push(1);
            append_u64(output, value);
        }
        None => output.push(0),
    }
}

fn video_clip_take_kind_tag(kind: crate::VideoClipTakeKind) -> u8 {
    match kind {
        crate::VideoClipTakeKind::Cut => 0,
        crate::VideoClipTakeKind::Crossfade => 1,
        crate::VideoClipTakeKind::Dip => 2,
        crate::VideoClipTakeKind::Wipe => 3,
        crate::VideoClipTakeKind::Luma => 4,
        crate::VideoClipTakeKind::Displacement => 5,
        crate::VideoClipTakeKind::Blur => 6,
        crate::VideoClipTakeKind::Glitch => 7,
        crate::VideoClipTakeKind::Custom => 8,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthenticatedShowClockAction {
    pub body: ShowClockAction,
    pub authentication_tag: [u8; 32],
}

impl AuthenticatedShowClockAction {
    pub fn sign(body: ShowClockAction, key: &[u8; 32]) -> Result<Self, ShowClockValidationError> {
        let canonical = body.canonical_bytes()?;
        Ok(Self {
            body,
            authentication_tag: hmac_sha256(key, &canonical),
        })
    }

    fn verify_and_canonical(&self, key: &[u8; 32]) -> Result<Vec<u8>, ShowClockValidationError> {
        let canonical = self.body.canonical_bytes()?;
        if constant_time_eq(&self.authentication_tag, &hmac_sha256(key, &canonical)) {
            Ok(canonical)
        } else {
            Err(ShowClockValidationError::InvalidAuthentication)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowClockActionAdmission {
    Accepted,
    Duplicate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockActionReceiver {
    peer: ShowClockPeerContext,
    last_sequence: u64,
    accepted_actions: BTreeMap<[u8; 16], [u8; 32]>,
}

impl ShowClockActionReceiver {
    pub fn new(
        expected_session: ShowClockSessionId,
        expected_sender: ShowClockNodeId,
        expected_project_hash: ShowClockHash,
        expected_media_hash: ShowClockHash,
        expected_clock_generation: u64,
        expected_fencing_generation: u64,
        key: [u8; 32],
    ) -> Result<Self, ShowClockValidationError> {
        let peer = ShowClockPeerContext::new(
            expected_session,
            expected_sender,
            expected_project_hash,
            expected_media_hash,
            expected_clock_generation,
            expected_fencing_generation,
            key,
        )?;
        Ok(Self {
            peer,
            last_sequence: 0,
            accepted_actions: BTreeMap::new(),
        })
    }

    pub fn admit(
        &mut self,
        action: &AuthenticatedShowClockAction,
    ) -> Result<ShowClockActionAdmission, ShowClockValidationError> {
        let canonical = action.verify_and_canonical(self.peer.key.as_bytes())?;
        self.peer.validate_action(&action.body)?;
        let fingerprint = canonical_fingerprint(&canonical);
        if let Some(previous) = self.accepted_actions.get(&action.body.action_id) {
            return if previous == &fingerprint {
                Ok(ShowClockActionAdmission::Duplicate)
            } else {
                Err(ShowClockValidationError::ActionConflict)
            };
        }
        if action.body.sequence <= self.last_sequence {
            return Err(if action.body.sequence == self.last_sequence {
                ShowClockValidationError::Replay
            } else {
                ShowClockValidationError::Reordered
            });
        }
        if self.accepted_actions.len() >= SHOW_CLOCK_MAX_ACCEPTED_ACTIONS {
            return Err(ShowClockValidationError::ActionAdmissionCapacityExceeded);
        }
        self.last_sequence = action.body.sequence;
        self.accepted_actions
            .insert(action.body.action_id, fingerprint);
        Ok(ShowClockActionAdmission::Accepted)
    }

    pub fn accepted_action_count(&self) -> usize {
        self.accepted_actions.len()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockReArm {
    pub operator_confirmed_primary_stopped: bool,
    pub clock_generation: u64,
    pub fencing_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowClockFenceState {
    Disarmed,
    Hold,
    Armed,
}

/// Manual Hold is the only failover policy in this tranche.  Network loss or
/// process restart cannot arm the standby, and an old primary cannot reclaim
/// output merely by rejoining the session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShowClockManualFence {
    state: ShowClockFenceState,
    /// Monotonic floor observed by this fence. It does not claim to be the
    /// authoritative live clock generation.
    clock_generation: u64,
    fencing_generation: u64,
}

impl ShowClockManualFence {
    pub fn new(
        clock_generation: u64,
        fencing_generation: u64,
    ) -> Result<Self, ShowClockValidationError> {
        if clock_generation == 0 || fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        Ok(Self {
            state: ShowClockFenceState::Disarmed,
            clock_generation,
            fencing_generation,
        })
    }

    pub fn state(self) -> ShowClockFenceState {
        self.state
    }

    pub fn fencing_generation(self) -> u64 {
        self.fencing_generation
    }

    pub fn clock_generation(self) -> u64 {
        self.clock_generation
    }

    pub fn enter_hold(&mut self) {
        self.state = ShowClockFenceState::Hold;
    }

    /// Arm a freshly started Primary without changing its initial generation.
    /// Standby takeover must use [`Self::rearm`] so it always carries Hold,
    /// operator confirmation, and an advanced fencing generation.
    pub fn arm_initial(
        &mut self,
        operator_confirmed: bool,
    ) -> Result<(), ShowClockValidationError> {
        if self.state != ShowClockFenceState::Disarmed {
            return Err(if self.state == ShowClockFenceState::Armed {
                ShowClockValidationError::AlreadyArmed
            } else {
                ShowClockValidationError::HoldRequired
            });
        }
        if !operator_confirmed {
            return Err(ShowClockValidationError::OperatorConfirmationRequired);
        }
        self.state = ShowClockFenceState::Armed;
        Ok(())
    }

    pub fn rearm(&mut self, request: ShowClockReArm) -> Result<(), ShowClockValidationError> {
        if self.state != ShowClockFenceState::Hold {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if !request.operator_confirmed_primary_stopped {
            return Err(ShowClockValidationError::OperatorConfirmationRequired);
        }
        if request.clock_generation < self.clock_generation {
            return Err(ShowClockValidationError::ReArmClockGenerationRewound);
        }
        if request.fencing_generation <= self.fencing_generation {
            return Err(ShowClockValidationError::ReArmGenerationNotAdvanced);
        }
        if request.clock_generation == 0 {
            return Err(ShowClockValidationError::ReArmClockGenerationRewound);
        }
        self.clock_generation = request.clock_generation;
        self.fencing_generation = request.fencing_generation;
        self.state = ShowClockFenceState::Armed;
        Ok(())
    }

    pub fn disarm(&mut self) {
        self.state = ShowClockFenceState::Disarmed;
    }
}

fn validate_ascii_id(value: &str) -> Result<(), ShowClockValidationError> {
    if value.is_empty()
        || value.len() > SHOW_CLOCK_MAX_ID_BYTES
        || value
            .bytes()
            .any(|byte| !byte.is_ascii() || byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(ShowClockValidationError::EmptyOrInvalidId);
    }
    Ok(())
}

fn append_id(output: &mut Vec<u8>, value: &str) -> Result<(), ShowClockValidationError> {
    validate_ascii_id(value)?;
    let len = u16::try_from(value.len()).map_err(|_| ShowClockValidationError::EmptyOrInvalidId)?;
    append_u16(output, len);
    output.extend_from_slice(value.as_bytes());
    Ok(())
}

fn append_u16(output: &mut Vec<u8>, value: u16) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn append_u32(output: &mut Vec<u8>, value: u32) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn append_u64(output: &mut Vec<u8>, value: u64) {
    output.extend_from_slice(&value.to_le_bytes());
}

fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    let mut ipad = [0x36_u8; 64];
    let mut opad = [0x5c_u8; 64];
    for index in 0..32 {
        ipad[index] ^= key[index];
        opad[index] ^= key[index];
    }

    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(message);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    let digest = outer.finalize();
    let mut output = [0_u8; 32];
    output.copy_from_slice(&digest);
    output
}

fn constant_time_eq(left: &[u8; 32], right: &[u8; 32]) -> bool {
    let mut difference = 0_u8;
    for index in 0..32 {
        difference |= left[index] ^ right[index];
    }
    difference == 0
}

fn canonical_fingerprint(canonical: &[u8]) -> [u8; 32] {
    let digest = Sha256::digest(canonical);
    let mut fingerprint = [0_u8; 32];
    fingerprint.copy_from_slice(&digest);
    fingerprint
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: [u8; 32] = [0x42; 32];
    const PROJECT: ShowClockHash = ShowClockHash([0x11; 32]);
    const MEDIA: ShowClockHash = ShowClockHash([0x22; 32]);

    fn session() -> ShowClockSessionId {
        ShowClockSessionId::new("session-1").unwrap()
    }

    fn sender() -> ShowClockNodeId {
        ShowClockNodeId::new("node-primary").unwrap()
    }

    fn sample(sequence: u64, show_time_us: u64) -> ShowClockSample {
        ShowClockSample {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: session(),
            sender: sender(),
            clock_generation: 1,
            fencing_generation: 1,
            sequence,
            sender_monotonic_us: show_time_us,
            show_time_us,
            bpm_milli: 120_000,
            beat_phase_ppm: 250_000,
            transport: ShowTransportState::Playing,
            source: ShowClockSource::ShowClock,
            project_hash: PROJECT,
            media_hash: MEDIA,
            expires_after_us: 250_000,
            nonce: ShowClockNonce([sequence as u8; 16]),
        }
    }

    fn validator() -> ShowClockPeerValidator {
        ShowClockPeerValidator::new(session(), sender(), PROJECT, MEDIA, 1, 1, KEY).unwrap()
    }

    fn action(action_id: u64, sequence: u64) -> ShowClockAction {
        let mut id = [0_u8; 16];
        id[..8].copy_from_slice(&action_id.to_le_bytes());
        ShowClockAction {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: session(),
            sender: sender(),
            action_id: id,
            sequence,
            clock_generation: 1,
            fencing_generation: 1,
            target_show_time_us: 2_000_000 + sequence,
            action: ShowClockActionKind::Go,
            late_policy: ShowClockLatePolicy::Drop,
            payload: None,
            project_hash: PROJECT,
            media_hash: MEDIA,
        }
    }

    #[test]
    fn authenticated_sample_rejects_tamper_replay_and_stale_generation() {
        let first = AuthenticatedShowClockSample::sign(sample(1, 1_000_000), &KEY).unwrap();
        let mut receiver = validator();
        receiver.accept(&first).unwrap();
        assert_eq!(receiver.last_sequence(), 1);
        assert_eq!(
            receiver.accept(&first),
            Err(ShowClockValidationError::Replay)
        );

        let mut tampered = first.clone();
        tampered.body.show_time_us += 1;
        assert_eq!(
            receiver.accept(&tampered),
            Err(ShowClockValidationError::InvalidAuthentication)
        );

        let stale = AuthenticatedShowClockSample::sign(sample(2, 999_000), &KEY).unwrap();
        assert_eq!(
            receiver.accept(&stale),
            Err(ShowClockValidationError::Reordered)
        );

        let mut backwards_sender_time = sample(2, 1_002_000);
        backwards_sender_time.sender_monotonic_us = 999_999;
        let backwards_sender_time =
            AuthenticatedShowClockSample::sign(backwards_sender_time, &KEY).unwrap();
        assert_eq!(
            receiver.accept(&backwards_sender_time),
            Err(ShowClockValidationError::Reordered)
        );
        assert_eq!(receiver.last_sequence(), 1);
        assert_eq!(receiver.last_sender_monotonic_us(), Some(1_000_000));

        let mut equal_sender_time = sample(2, 1_001_000);
        equal_sender_time.sender_monotonic_us = 1_000_000;
        let equal_sender_time =
            AuthenticatedShowClockSample::sign(equal_sender_time, &KEY).unwrap();
        receiver.accept(&equal_sender_time).unwrap();
        assert_eq!(receiver.last_sequence(), 2);
        assert_eq!(receiver.last_sender_monotonic_us(), Some(1_000_000));

        let mut next = sample(3, 1_003_000);
        next.fencing_generation = 2;
        let next = AuthenticatedShowClockSample::sign(next, &KEY).unwrap();
        assert_eq!(
            receiver.accept(&next),
            Err(ShowClockValidationError::FencingGenerationMismatch)
        );
    }

    #[test]
    fn authenticated_action_is_at_most_once_and_rejects_conflicts() {
        let body = action(7, 1);
        let signed = AuthenticatedShowClockAction::sign(body.clone(), &KEY).unwrap();
        let mut receiver =
            ShowClockActionReceiver::new(session(), sender(), PROJECT, MEDIA, 1, 1, KEY).unwrap();
        assert_eq!(
            receiver.admit(&signed),
            Ok(ShowClockActionAdmission::Accepted)
        );
        assert_eq!(
            receiver.admit(&signed),
            Ok(ShowClockActionAdmission::Duplicate)
        );

        let mut conflicting = body;
        conflicting.action = ShowClockActionKind::Take;
        conflicting.payload = Some(ShowClockActionPayload::VideoTake {
            target_layer_id: 1,
            fade_ms: 0,
            preview_position_ms: None,
            preview_speed_milli: None,
        });
        let conflicting = AuthenticatedShowClockAction::sign(conflicting, &KEY).unwrap();
        assert_eq!(
            receiver.admit(&conflicting),
            Err(ShowClockValidationError::ActionConflict)
        );
    }

    #[test]
    fn action_payload_is_required_kind_bound_and_authenticated() {
        let mut missing = action(8, 1);
        missing.action = ShowClockActionKind::Take;
        assert_eq!(
            missing.validate_shape(),
            Err(ShowClockValidationError::ActionPayloadRequired)
        );

        let mut unexpected = action(9, 1);
        unexpected.payload = Some(ShowClockActionPayload::CueRelease { cue_id: 1 });
        assert_eq!(
            unexpected.validate_shape(),
            Err(ShowClockValidationError::ActionPayloadUnexpected)
        );

        let mut invalid = action(10, 1);
        invalid.action = ShowClockActionKind::Release;
        invalid.payload = Some(ShowClockActionPayload::CueRelease { cue_id: 0 });
        assert_eq!(
            invalid.validate_shape(),
            Err(ShowClockValidationError::InvalidActionPayload)
        );

        let mut valid = action(11, 1);
        valid.action = ShowClockActionKind::TimelineJump;
        valid.payload = Some(ShowClockActionPayload::TimelineJump { position_ms: 4_200 });
        let signed = AuthenticatedShowClockAction::sign(valid, &KEY).unwrap();
        signed.verify_and_canonical(&KEY).unwrap();
        let mut receiver =
            ShowClockActionReceiver::new(session(), sender(), PROJECT, MEDIA, 1, 1, KEY).unwrap();
        assert_eq!(
            receiver.admit(&signed),
            Ok(ShowClockActionAdmission::Accepted)
        );
    }

    #[test]
    fn manual_fence_never_arms_without_hold_confirmation_and_advanced_generation() {
        let mut initial = ShowClockManualFence::new(1, 1).unwrap();
        assert_eq!(
            initial.arm_initial(false),
            Err(ShowClockValidationError::OperatorConfirmationRequired)
        );
        initial.arm_initial(true).unwrap();
        assert_eq!(initial.state(), ShowClockFenceState::Armed);
        assert_eq!(
            initial.arm_initial(true),
            Err(ShowClockValidationError::AlreadyArmed)
        );

        let mut fence = ShowClockManualFence::new(1, 1).unwrap();
        assert_eq!(fence.state(), ShowClockFenceState::Disarmed);
        assert_eq!(fence.clock_generation(), 1);
        assert_eq!(
            fence.rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 2,
            }),
            Err(ShowClockValidationError::HoldRequired)
        );
        fence.enter_hold();
        assert_eq!(
            fence.rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: false,
                clock_generation: 2,
                fencing_generation: 2,
            }),
            Err(ShowClockValidationError::OperatorConfirmationRequired)
        );
        assert_eq!(
            fence.rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 2,
                fencing_generation: 1,
            }),
            Err(ShowClockValidationError::ReArmGenerationNotAdvanced)
        );
        fence
            .rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 2,
                fencing_generation: 2,
            })
            .unwrap();
        assert_eq!(fence.state(), ShowClockFenceState::Armed);
        assert_eq!(fence.clock_generation(), 2);
        assert_eq!(fence.fencing_generation(), 2);

        fence.enter_hold();
        assert_eq!(
            fence.rearm(ShowClockReArm {
                operator_confirmed_primary_stopped: true,
                clock_generation: 1,
                fencing_generation: 3,
            }),
            Err(ShowClockValidationError::ReArmClockGenerationRewound)
        );
        assert_eq!(fence.state(), ShowClockFenceState::Hold);
        assert_eq!(fence.fencing_generation(), 2);
    }

    #[test]
    fn admission_capacity_is_bounded_and_fails_closed_without_eviction() {
        let mut receiver =
            ShowClockActionReceiver::new(session(), sender(), PROJECT, MEDIA, 1, 1, KEY).unwrap();
        for sequence in 1..=SHOW_CLOCK_MAX_ACCEPTED_ACTIONS as u64 {
            let signed =
                AuthenticatedShowClockAction::sign(action(sequence, sequence), &KEY).unwrap();
            assert_eq!(
                receiver.admit(&signed),
                Ok(ShowClockActionAdmission::Accepted)
            );
        }
        assert_eq!(
            receiver.accepted_action_count(),
            SHOW_CLOCK_MAX_ACCEPTED_ACTIONS
        );
        let full = AuthenticatedShowClockAction::sign(
            action(
                (SHOW_CLOCK_MAX_ACCEPTED_ACTIONS as u64) + 1,
                (SHOW_CLOCK_MAX_ACCEPTED_ACTIONS as u64) + 1,
            ),
            &KEY,
        )
        .unwrap();
        assert_eq!(
            receiver.admit(&full),
            Err(ShowClockValidationError::ActionAdmissionCapacityExceeded)
        );
        assert_eq!(
            receiver.last_sequence,
            SHOW_CLOCK_MAX_ACCEPTED_ACTIONS as u64
        );
        assert_eq!(
            receiver.admit(&full),
            Err(ShowClockValidationError::ActionAdmissionCapacityExceeded)
        );

        let duplicate = AuthenticatedShowClockAction::sign(action(1, 1), &KEY).unwrap();
        assert_eq!(
            receiver.admit(&duplicate),
            Ok(ShowClockActionAdmission::Duplicate)
        );
        let mut conflicting = action(1, 1);
        conflicting.action = ShowClockActionKind::Take;
        conflicting.payload = Some(ShowClockActionPayload::VideoTake {
            target_layer_id: 1,
            fade_ms: 0,
            preview_position_ms: None,
            preview_speed_milli: None,
        });
        let conflicting = AuthenticatedShowClockAction::sign(conflicting, &KEY).unwrap();
        assert_eq!(
            receiver.admit(&conflicting),
            Err(ShowClockValidationError::ActionConflict)
        );
    }

    #[test]
    fn identifiers_validate_on_construction_and_deserialization() {
        assert!(ShowClockSessionId::new("bad\u{0001}").is_err());
        assert!(ShowClockNodeId::new("bad\t").is_err());
        assert!(serde_json::from_str::<ShowClockSessionId>("\"bad\\u0001\"").is_err());
        assert!(serde_json::from_str::<ShowClockNodeId>("\"bad\\n\"").is_err());

        let mut invalid_action = action(1, 1);
        invalid_action.sequence = 0;
        assert_eq!(
            invalid_action.validate_shape(),
            Err(ShowClockValidationError::ZeroSequence)
        );
        invalid_action.sequence = 1;
        invalid_action.target_show_time_us = 0;
        assert_eq!(
            invalid_action.validate_shape(),
            Err(ShowClockValidationError::InvalidTargetTime)
        );
    }

    #[test]
    fn peer_debug_redacts_authentication_key() {
        let validator = validator();
        let receiver =
            ShowClockActionReceiver::new(session(), sender(), PROJECT, MEDIA, 1, 1, KEY).unwrap();
        let validator_debug = format!("{validator:?}");
        let receiver_debug = format!("{receiver:?}");
        assert!(validator_debug.contains("[redacted]"));
        assert!(receiver_debug.contains("[redacted]"));
        assert!(!validator_debug.contains("66, 66, 66"));
        assert!(!receiver_debug.contains("66, 66, 66"));
    }

    #[test]
    fn authenticated_sample_vector_is_stable() {
        let signed = AuthenticatedShowClockSample::sign(sample(1, 1_000_000), &KEY).unwrap();
        let mut expected_canonical = Vec::new();
        expected_canonical.extend_from_slice(b"syndocal.show-clock.sample.v1\0");
        expected_canonical.extend_from_slice(&[1, 0, 1, 0]);
        expected_canonical.extend_from_slice(&[9, 0]);
        expected_canonical.extend_from_slice(b"session-1");
        expected_canonical.extend_from_slice(&[12, 0]);
        expected_canonical.extend_from_slice(b"node-primary");
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_000_000_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_000_000_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&120_000_u32.to_le_bytes());
        expected_canonical.extend_from_slice(&250_000_u32.to_le_bytes());
        expected_canonical.extend_from_slice(&[1, 0]);
        expected_canonical.extend_from_slice(&[0x11; 32]);
        expected_canonical.extend_from_slice(&[0x22; 32]);
        expected_canonical.extend_from_slice(&250_000_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&[1; 16]);
        assert_eq!(signed.body.canonical_bytes().unwrap(), expected_canonical);
        assert_eq!(
            signed.authentication_tag,
            [
                0xe8, 0x1b, 0x22, 0xfa, 0x02, 0x17, 0xfc, 0x7a, 0x51, 0x08, 0x5b, 0xb2, 0x41, 0x1f,
                0x05, 0x90, 0xa2, 0x5b, 0xed, 0xb9, 0x3d, 0xaf, 0x8c, 0xa4, 0xf4, 0x06, 0xbd, 0xb4,
                0x65, 0x94, 0x0e, 0x0d,
            ]
        );
    }

    #[test]
    fn authenticated_action_vector_is_stable() {
        let signed = AuthenticatedShowClockAction::sign(action(7, 1), &KEY).unwrap();
        let mut expected_canonical = Vec::new();
        expected_canonical.extend_from_slice(b"syndocal.show-clock.action.v1\0");
        expected_canonical.extend_from_slice(&[1, 0, 1, 0]);
        expected_canonical.extend_from_slice(&[9, 0]);
        expected_canonical.extend_from_slice(b"session-1");
        expected_canonical.extend_from_slice(&[12, 0]);
        expected_canonical.extend_from_slice(b"node-primary");
        expected_canonical.extend_from_slice(&[7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&1_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&2_000_001_u64.to_le_bytes());
        expected_canonical.extend_from_slice(&[0, 1]);
        expected_canonical.extend_from_slice(&[0x11; 32]);
        expected_canonical.extend_from_slice(&[0x22; 32]);
        assert_eq!(signed.body.canonical_bytes().unwrap(), expected_canonical);
        assert_eq!(
            signed.authentication_tag,
            [
                0x23, 0x03, 0xcc, 0x7d, 0x52, 0x43, 0x27, 0x01, 0x32, 0xb3, 0x6e, 0x9b, 0x1e, 0x7a,
                0xd8, 0x9f, 0x5d, 0x9d, 0x2e, 0x1c, 0xde, 0xa8, 0xe4, 0x41, 0x71, 0x04, 0x73, 0x8a,
                0x34, 0x1a, 0xc3, 0xaa,
            ]
        );
    }
}
