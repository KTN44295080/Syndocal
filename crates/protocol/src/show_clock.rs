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
            Self::AlreadyArmed => "ShowClock peer is already armed",
        })
    }
}

impl std::error::Error for ShowClockValidationError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
pub enum ShowClockSource {
    ShowClock,
    MidiClock,
    MidiTimecode,
    Ltc,
    AbletonLink,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowTransportState {
    Stopped,
    Playing,
    Paused,
    Holding,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowClockPeerState {
    Locked,
    Acquiring,
    Hold,
    Stale,
    Fault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowClockActionKind {
    Go,
    Stop,
    Back,
    Release,
    Blackout,
    Take,
    ClipLaunch,
    Transition,
    TimelineJump,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShowClockLatePolicy {
    ExecuteImmediately,
    Drop,
    Hold,
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
        let canonical = self.body.canonical_bytes()?;
        if constant_time_eq(&self.authentication_tag, &hmac_sha256(key, &canonical)) {
            Ok(())
        } else {
            Err(ShowClockValidationError::InvalidAuthentication)
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowClockPeerValidator {
    expected_session: ShowClockSessionId,
    expected_sender: ShowClockNodeId,
    expected_project_hash: ShowClockHash,
    expected_media_hash: ShowClockHash,
    expected_clock_generation: u64,
    expected_fencing_generation: u64,
    key: [u8; 32],
    last_sequence: u64,
    last_show_time_us: Option<u64>,
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
            key,
            last_sequence: 0,
            last_show_time_us: None,
        })
    }

    pub fn accept(
        &mut self,
        sample: &AuthenticatedShowClockSample,
    ) -> Result<(), ShowClockValidationError> {
        sample.body.validate_shape()?;
        sample.verify(&self.key)?;
        if sample.body.session_id != self.expected_session {
            return Err(ShowClockValidationError::SessionMismatch);
        }
        if sample.body.sender != self.expected_sender {
            return Err(ShowClockValidationError::SenderMismatch);
        }
        if sample.body.project_hash != self.expected_project_hash {
            return Err(ShowClockValidationError::ProjectHashMismatch);
        }
        if sample.body.media_hash != self.expected_media_hash {
            return Err(ShowClockValidationError::MediaHashMismatch);
        }
        if sample.body.clock_generation != self.expected_clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if sample.body.fencing_generation != self.expected_fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }
        if sample.body.sequence <= self.last_sequence {
            return Err(if sample.body.sequence == self.last_sequence {
                ShowClockValidationError::Replay
            } else {
                ShowClockValidationError::Reordered
            });
        }
        if self
            .last_show_time_us
            .is_some_and(|last| sample.body.show_time_us < last)
        {
            return Err(ShowClockValidationError::Reordered);
        }
        self.last_sequence = sample.body.sequence;
        self.last_show_time_us = Some(sample.body.show_time_us);
        Ok(())
    }

    pub fn last_sequence(&self) -> u64 {
        self.last_sequence
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
        if self.sequence == 0 || self.clock_generation == 0 || self.fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        if self.action_id == [0; 16] {
            return Err(ShowClockValidationError::InvalidActionId);
        }
        if self.project_hash.is_zero() || self.media_hash.is_zero() {
            return Err(ShowClockValidationError::ZeroHash);
        }
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
        output.extend_from_slice(&self.project_hash.0);
        output.extend_from_slice(&self.media_hash.0);
        Ok(output)
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

    fn verify(&self, key: &[u8; 32]) -> Result<(), ShowClockValidationError> {
        let canonical = self.body.canonical_bytes()?;
        if constant_time_eq(&self.authentication_tag, &hmac_sha256(key, &canonical)) {
            Ok(())
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
    expected_session: ShowClockSessionId,
    expected_sender: ShowClockNodeId,
    expected_project_hash: ShowClockHash,
    expected_media_hash: ShowClockHash,
    expected_clock_generation: u64,
    expected_fencing_generation: u64,
    key: [u8; 32],
    last_sequence: u64,
    accepted_actions: BTreeMap<[u8; 16], Vec<u8>>,
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
            key,
            last_sequence: 0,
            accepted_actions: BTreeMap::new(),
        })
    }

    pub fn admit(
        &mut self,
        action: &AuthenticatedShowClockAction,
    ) -> Result<ShowClockActionAdmission, ShowClockValidationError> {
        action.body.validate_shape()?;
        action.verify(&self.key)?;
        if action.body.session_id != self.expected_session {
            return Err(ShowClockValidationError::SessionMismatch);
        }
        if action.body.sender != self.expected_sender {
            return Err(ShowClockValidationError::SenderMismatch);
        }
        if action.body.project_hash != self.expected_project_hash {
            return Err(ShowClockValidationError::ProjectHashMismatch);
        }
        if action.body.media_hash != self.expected_media_hash {
            return Err(ShowClockValidationError::MediaHashMismatch);
        }
        if action.body.clock_generation != self.expected_clock_generation {
            return Err(ShowClockValidationError::ClockGenerationMismatch);
        }
        if action.body.fencing_generation != self.expected_fencing_generation {
            return Err(ShowClockValidationError::FencingGenerationMismatch);
        }

        let canonical = action.body.canonical_bytes()?;
        if let Some(previous) = self.accepted_actions.get(&action.body.action_id) {
            return if previous == &canonical {
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
        self.last_sequence = action.body.sequence;
        self.accepted_actions
            .insert(action.body.action_id, canonical);
        Ok(ShowClockActionAdmission::Accepted)
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
    fencing_generation: u64,
}

impl ShowClockManualFence {
    pub fn new(fencing_generation: u64) -> Result<Self, ShowClockValidationError> {
        if fencing_generation == 0 {
            return Err(ShowClockValidationError::ZeroGeneration);
        }
        Ok(Self {
            state: ShowClockFenceState::Disarmed,
            fencing_generation,
        })
    }

    pub fn state(self) -> ShowClockFenceState {
        self.state
    }

    pub fn fencing_generation(self) -> u64 {
        self.fencing_generation
    }

    pub fn enter_hold(&mut self) {
        self.state = ShowClockFenceState::Hold;
    }

    pub fn rearm(&mut self, request: ShowClockReArm) -> Result<(), ShowClockValidationError> {
        if self.state != ShowClockFenceState::Hold {
            return Err(ShowClockValidationError::HoldRequired);
        }
        if !request.operator_confirmed_primary_stopped {
            return Err(ShowClockValidationError::OperatorConfirmationRequired);
        }
        if request.clock_generation == 0 || request.fencing_generation <= self.fencing_generation {
            return Err(ShowClockValidationError::ReArmGenerationNotAdvanced);
        }
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
            .any(|byte| !byte.is_ascii() || byte.is_ascii_whitespace())
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

        let mut next = sample(2, 1_001_000);
        next.fencing_generation = 2;
        let next = AuthenticatedShowClockSample::sign(next, &KEY).unwrap();
        assert_eq!(
            receiver.accept(&next),
            Err(ShowClockValidationError::FencingGenerationMismatch)
        );
    }

    #[test]
    fn authenticated_action_is_exactly_once_and_rejects_conflicts() {
        let body = ShowClockAction {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: session(),
            sender: sender(),
            action_id: [7; 16],
            sequence: 1,
            clock_generation: 1,
            fencing_generation: 1,
            target_show_time_us: 2_000_000,
            action: ShowClockActionKind::Go,
            late_policy: ShowClockLatePolicy::Drop,
            project_hash: PROJECT,
            media_hash: MEDIA,
        };
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
        let conflicting = AuthenticatedShowClockAction::sign(conflicting, &KEY).unwrap();
        assert_eq!(
            receiver.admit(&conflicting),
            Err(ShowClockValidationError::ActionConflict)
        );
    }

    #[test]
    fn manual_fence_never_arms_without_hold_confirmation_and_advanced_generation() {
        let mut fence = ShowClockManualFence::new(1).unwrap();
        assert_eq!(fence.state(), ShowClockFenceState::Disarmed);
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
        assert_eq!(fence.fencing_generation(), 2);
    }
}
