//! Machine-local ASIO PROGRAM/CUE output profile and deterministic mapper.
//!
//! This module has no project-file types on purpose.  Project Timeline audio
//! stores only its logical `PROGRAM | CUE` bus; the driver identity and physical
//! channels below belong to the current machine and must be revalidated before
//! any eventual v3 Start.

use engine::TimelineAudioLiveFence;
use serde::{
    de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor},
    Deserialize, Serialize,
};
use std::collections::{BTreeSet, HashSet};
#[cfg(test)]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION: u32 = 2;
const LEGACY_MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION: u32 = 1;
const MAX_PROFILE_BYTES: usize = 65_536;
const MAX_PROFILE_STRING_BYTES: usize = 512;
const MAX_DEVICE_OUTPUT_CHANNELS: u32 = u16::MAX as u32;
const MAX_FIXED_BUFFER_FRAMES: u32 = 1_048_576;

/// The bounded level used by every operator-facing output preflight test.
/// This is deliberately a single, small constant rather than a caller-
/// supplied amplitude so a test cannot accidentally become a full-scale burst.
pub(crate) const PREFLIGHT_SAFE_AMPLITUDE: f32 = 0.1;
pub(crate) const PREFLIGHT_MAX_DURATION_MS: u64 = 30_000;

/// Typed output targets for the operator preflight.  Physical channel numbers
/// stay exclusively in `MachineAsioOutputProfile`; callers select a logical
/// target and this module resolves it through that validated profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreflightTarget {
    /// Disable the currently armed test.  This target never produces audio.
    Off,
    ProgramLeft,
    ProgramRight,
    ProgramStereo,
    Cue,
    Spare,
}

/// Mutually exclusive logical solo modes for the PROGRAM/CUE preflight.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreflightSolo {
    None,
    ProgramOnly,
    CueOnly,
}

/// Callback-visible classification of the ephemeral selection that produced
/// one render block.  Ordinary PROGRAM/CUE blocks deliberately use zero so
/// they can continue across a preflight-only retirement; Test and Solo blocks
/// must carry a non-zero selection epoch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum PreflightSelectionKind {
    None = 0,
    Test = 1,
    Solo = 2,
}

/// Explicit reasons that retire all ephemeral preflight state.  None of this
/// state is serializable or persisted with the machine output profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreflightClearReason {
    Expired,
    TransportRotation,
    Stop,
    Fault,
    EngineLive,
}

/// The machine-local CUE destination.  The ASIO profile owns only the ASIO
/// PROGRAM channels; an explicit WDM target is metadata for a separate WDM
/// runtime and therefore has no ASIO channel number.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "camelCase", deny_unknown_fields)]
pub(crate) enum CueDelivery {
    SameAsio {
        #[serde(rename = "cueOutput")]
        cue_output: u32,
    },
    ExplicitWdm {
        #[serde(rename = "deviceName")]
        device_name: String,
        #[serde(rename = "topologyFingerprint")]
        topology_fingerprint: String,
    },
}

impl CueDelivery {
    pub(crate) fn is_same_asio(&self) -> bool {
        matches!(self, Self::SameAsio { .. })
    }

    pub(crate) fn is_external_wdm(&self) -> bool {
        matches!(self, Self::ExplicitWdm { .. })
    }
}

/// Strict machine-only schema.  `deny_unknown_fields` deliberately preserves a
/// future/corrupt file as Locked rather than rewriting it into a lossy shape.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct MachineAsioOutputProfile {
    schema_version: u32,
    driver_id: String,
    catalog_generation: u64,
    sample_rate_hz: u32,
    native_format: String,
    fixed_buffer_frames: u32,
    device_output_channels: u32,
    /// One-based UI channel numbers.  Conversion happens only after full
    /// validation so Output 1 always becomes callback channel 0 exactly once.
    program_left_output: u32,
    program_right_output: u32,
    cue_delivery: CueDelivery,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    spare_output: Option<u32>,
}

/// The only accepted legacy shape.  It is parsed separately so the V1
/// `cueOutput` field can never be accepted by the V2 deserializer and so the
/// migration remains an explicit, one-way conversion.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct MachineAsioOutputProfileV1 {
    schema_version: u32,
    driver_id: String,
    catalog_generation: u64,
    sample_rate_hz: u32,
    native_format: String,
    fixed_buffer_frames: u32,
    device_output_channels: u32,
    program_left_output: u32,
    program_right_output: u32,
    cue_output: u32,
    #[serde(default)]
    spare_output: Option<u32>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MachineOutputProfileSchemaProbe {
    schema_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputProfileLock {
    Oversized,
    InvalidJson(String),
    FutureSchema { actual: u32, expected: u32 },
    Invalid(String),
}

/// A live capability revalidation result that may admit exactly one explicit
/// operator reselection.  It is intentionally non-serializable and captures
/// every output tuple dimension whose drift would otherwise make a stored
/// physical mapping unsafe.
#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct FreshOutputCapabilityProof {
    driver_id: String,
    catalog_generation: u64,
    sample_rate_hz: u32,
    native_format: String,
    fixed_buffer_frames: u32,
    device_output_channels: u32,
    token: u64,
    // Clones deliberately share this authority bit.  A caller cannot turn one
    // live revalidation into two admissions by copying the proof before the
    // first explicit reselection consumes it.
    consumed: Arc<AtomicBool>,
}

#[cfg(test)]
impl FreshOutputCapabilityProof {
    pub(crate) fn from_live_revalidation(
        driver_id: String,
        catalog_generation: u64,
        sample_rate_hz: u32,
        native_format: String,
        fixed_buffer_frames: u32,
        device_output_channels: u32,
        token: u64,
    ) -> Result<Self, OutputProfileLock> {
        let profile = MachineAsioOutputProfile {
            schema_version: MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION,
            driver_id: driver_id.clone(),
            catalog_generation,
            sample_rate_hz,
            native_format: native_format.clone(),
            fixed_buffer_frames,
            device_output_channels,
            // The profile validator also validates the capability tuple; use
            // distinct in-range placeholders solely for this local check.
            program_left_output: 1,
            program_right_output: 2,
            cue_delivery: CueDelivery::SameAsio { cue_output: 3 },
            spare_output: None,
        };
        profile.validate_capability_tuple()?;
        if token == 0 {
            return Err(OutputProfileLock::Invalid(
                "fresh ASIO output capability proof token must be non-zero".to_owned(),
            ));
        }
        Ok(Self {
            driver_id,
            catalog_generation,
            sample_rate_hz,
            native_format,
            fixed_buffer_frames,
            device_output_channels,
            token,
            consumed: Arc::new(AtomicBool::new(false)),
        })
    }

    fn admit_once(&mut self, profile: &MachineAsioOutputProfile) -> Result<(), OutputProfileLock> {
        if self.token == 0 || self.consumed.load(Ordering::Acquire) {
            return Err(OutputProfileLock::Invalid(
                "fresh ASIO output capability proof was already consumed".to_owned(),
            ));
        }
        if self.driver_id != profile.driver_id
            || self.catalog_generation != profile.catalog_generation
            || self.sample_rate_hz != profile.sample_rate_hz
            || self.native_format != profile.native_format
            || self.fixed_buffer_frames != profile.fixed_buffer_frames
            || self.device_output_channels != profile.device_output_channels
        {
            return Err(OutputProfileLock::Invalid(
                "fresh ASIO output capability proof no longer matches the explicit machine profile"
                    .to_owned(),
            ));
        }
        if self
            .consumed
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(OutputProfileLock::Invalid(
                "fresh ASIO output capability proof was already consumed".to_owned(),
            ));
        }
        Ok(())
    }
}

struct NoDuplicateJsonObject;

impl<'de> Deserialize<'de> for NoDuplicateJsonObject {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct NoDuplicateVisitor;
        impl<'de> Visitor<'de> for NoDuplicateVisitor {
            type Value = NoDuplicateJsonObject;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a JSON object with no duplicate keys")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut keys = HashSet::new();
                while let Some(key) = map.next_key::<String>()? {
                    if !keys.insert(key.clone()) {
                        return Err(de::Error::custom(format!(
                            "duplicate machine ASIO output profile field {key:?}"
                        )));
                    }
                    map.next_value_seed(NoDuplicateJsonValue)?;
                }
                Ok(NoDuplicateJsonObject)
            }
        }
        deserializer.deserialize_map(NoDuplicateVisitor)
    }
}

/// A recursive JSON pre-scan.  `serde_json::Value` and `IgnoredAny` both lose
/// duplicate object members, so every nested object/array is traversed with a
/// seed before the typed profile deserializer is allowed to run.
struct NoDuplicateJsonValue;

impl<'de> DeserializeSeed<'de> for NoDuplicateJsonValue {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NoDuplicateJsonValueVisitor)
    }
}

struct NoDuplicateJsonValueVisitor;

impl<'de> Visitor<'de> for NoDuplicateJsonValueVisitor {
    type Value = ();

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("any JSON value with no duplicate object keys")
    }

    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: MapAccess<'de>,
    {
        let mut keys = HashSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if !keys.insert(key.clone()) {
                return Err(de::Error::custom(format!(
                    "duplicate machine ASIO output profile field {key:?}"
                )));
            }
            map.next_value_seed(NoDuplicateJsonValue)?;
        }
        Ok(())
    }

    fn visit_seq<A>(self, mut sequence: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        while sequence.next_element_seed(NoDuplicateJsonValue)?.is_some() {}
        Ok(())
    }

    fn visit_bool<E>(self, _value: bool) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_i64<E>(self, _value: i64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_u64<E>(self, _value: u64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_f64<E>(self, _value: f64) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_str<E>(self, _value: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_string<E>(self, _value: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_bytes<E>(self, _value: &[u8]) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_byte_buf<E>(self, _value: Vec<u8>) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_unit<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_none<E>(self) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(())
    }

    fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        NoDuplicateJsonValue.deserialize(deserializer)
    }

    fn visit_newtype_struct<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        NoDuplicateJsonValue.deserialize(deserializer)
    }
}

impl std::fmt::Display for OutputProfileLock {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Oversized => write!(
                f,
                "machine ASIO output profile exceeds {MAX_PROFILE_BYTES} bytes"
            ),
            Self::InvalidJson(error) => {
                write!(f, "machine ASIO output profile is invalid: {error}")
            }
            Self::FutureSchema { actual, expected } => write!(
                f,
                "machine ASIO output profile schema {actual} is unsupported; expected {expected}"
            ),
            Self::Invalid(message) => f.write_str(message),
        }
    }
}

/// Restoration state for strict machine-local output configuration.  A locked
/// value retains exactly the original bytes; no automatic repair/save path may
/// alter them.  Only a validated explicit operator reselection can replace it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MachineAsioOutputProfileState {
    Ready(MachineAsioOutputProfile),
    Locked {
        original_bytes: Vec<u8>,
        reason: OutputProfileLock,
    },
}

impl MachineAsioOutputProfile {
    pub(crate) fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub(crate) fn fixed_buffer_frames(&self) -> u32 {
        self.fixed_buffer_frames
    }

    pub(crate) fn device_output_channels(&self) -> u32 {
        self.device_output_channels
    }

    pub(crate) fn driver_id(&self) -> &str {
        &self.driver_id
    }

    pub(crate) fn catalog_generation(&self) -> u64 {
        self.catalog_generation
    }

    pub(crate) fn native_format(&self) -> &str {
        &self.native_format
    }

    pub(crate) fn cue_delivery(&self) -> &CueDelivery {
        &self.cue_delivery
    }

    /// Return the zero-based ASIO CUE channel, or `None` when CUE belongs to
    /// the separately owned external WDM runtime.
    pub(crate) fn asio_cue_output_index(&self) -> Option<usize> {
        match self.cue_delivery {
            CueDelivery::SameAsio { cue_output } => Some(self.zero_based(cue_output)),
            CueDelivery::ExplicitWdm { .. } => None,
        }
    }

    #[cfg(test)]
    pub(crate) fn for_test(
        device_output_channels: u32,
        fixed_buffer_frames: u32,
        program_left_output: u32,
        program_right_output: u32,
        cue_output: u32,
    ) -> Self {
        Self {
            schema_version: MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION,
            driver_id: "asio:Test Driver".to_owned(),
            catalog_generation: 1,
            sample_rate_hz: 48_000,
            native_format: "f32".to_owned(),
            fixed_buffer_frames,
            device_output_channels,
            program_left_output,
            program_right_output,
            cue_delivery: CueDelivery::SameAsio { cue_output },
            spare_output: None,
        }
    }

    /// Strict parse for machine-local storage.  The caller must keep the input
    /// bytes unchanged whenever this returns `Err`; invalid input never receives
    /// a default device, fallback route, or partial repair. Schema V1 alone has
    /// the explicit one-way migration to the equivalent V2 `SameAsio` route.
    pub(crate) fn parse_storage_text(text: &str) -> Result<Self, OutputProfileLock> {
        if text.len() > MAX_PROFILE_BYTES {
            return Err(OutputProfileLock::Oversized);
        }
        serde_json::from_str::<NoDuplicateJsonObject>(text)
            .map_err(|error| OutputProfileLock::InvalidJson(error.to_string()))?;
        let schema = serde_json::from_str::<MachineOutputProfileSchemaProbe>(text)
            .map_err(|error| OutputProfileLock::InvalidJson(error.to_string()))?;
        let profile = match schema.schema_version {
            LEGACY_MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION => {
                let legacy: MachineAsioOutputProfileV1 = serde_json::from_str(text)
                    .map_err(|error| OutputProfileLock::InvalidJson(error.to_string()))?;
                Self::migrate_v1(legacy)?
            }
            MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION => serde_json::from_str(text)
                .map_err(|error| OutputProfileLock::InvalidJson(error.to_string()))?,
            actual if actual > MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION => {
                return Err(OutputProfileLock::FutureSchema {
                    actual,
                    expected: MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION,
                });
            }
            actual => {
                return Err(OutputProfileLock::Invalid(format!(
                    "machine ASIO output profile schema {actual} is not supported"
                )));
            }
        };
        profile.validate()?;
        Ok(profile)
    }

    fn migrate_v1(legacy: MachineAsioOutputProfileV1) -> Result<Self, OutputProfileLock> {
        if legacy.schema_version != LEGACY_MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION {
            return Err(OutputProfileLock::Invalid(
                "machine ASIO output profile legacy migration received an unexpected schema"
                    .to_owned(),
            ));
        }
        Ok(Self {
            schema_version: MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION,
            driver_id: legacy.driver_id,
            catalog_generation: legacy.catalog_generation,
            sample_rate_hz: legacy.sample_rate_hz,
            native_format: legacy.native_format,
            fixed_buffer_frames: legacy.fixed_buffer_frames,
            device_output_channels: legacy.device_output_channels,
            program_left_output: legacy.program_left_output,
            program_right_output: legacy.program_right_output,
            cue_delivery: CueDelivery::SameAsio {
                cue_output: legacy.cue_output,
            },
            spare_output: legacy.spare_output,
        })
    }

    /// Parses persisted bytes without rewriting them on a malformed, unknown,
    /// future, corrupt, or oversized profile.  The owner must surface Locked
    /// and wait for explicit operator selection rather than choosing a default
    /// audio device or mutating the stored bytes.
    pub(crate) fn restore_storage_bytes(bytes: Vec<u8>) -> MachineAsioOutputProfileState {
        let parsed = if bytes.len() > MAX_PROFILE_BYTES {
            Err(OutputProfileLock::Oversized)
        } else {
            std::str::from_utf8(&bytes)
                .map_err(|error| OutputProfileLock::InvalidJson(error.to_string()))
                .and_then(Self::parse_storage_text)
        };
        match parsed {
            Ok(profile) => MachineAsioOutputProfileState::Ready(profile),
            Err(reason) => MachineAsioOutputProfileState::Locked {
                original_bytes: bytes,
                reason,
            },
        }
    }

    pub(crate) fn validate(&self) -> Result<(), OutputProfileLock> {
        if self.schema_version > MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION {
            return Err(OutputProfileLock::FutureSchema {
                actual: self.schema_version,
                expected: MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION,
            });
        }
        if self.schema_version != MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION {
            return Err(OutputProfileLock::Invalid(format!(
                "machine ASIO output profile schema {} is not supported",
                self.schema_version
            )));
        }
        self.validate_capability_tuple()?;
        let mut selected = BTreeSet::new();
        for (name, channel) in [
            ("programLeftOutput", self.program_left_output),
            ("programRightOutput", self.program_right_output),
        ] {
            if channel == 0 || channel > self.device_output_channels {
                return Err(OutputProfileLock::Invalid(format!(
                    "machine ASIO output profile {name} Output {channel} is outside Output 1..{}",
                    self.device_output_channels
                )));
            }
            if !selected.insert(channel) {
                return Err(OutputProfileLock::Invalid(format!(
                    "machine ASIO output profile duplicates Output {channel}"
                )));
            }
        }
        match &self.cue_delivery {
            CueDelivery::SameAsio { cue_output } => {
                if *cue_output == 0 || *cue_output > self.device_output_channels {
                    return Err(OutputProfileLock::Invalid(format!(
                        "machine ASIO output profile cueDelivery.cueOutput Output {cue_output} is outside Output 1..{}",
                        self.device_output_channels
                    )));
                }
                if !selected.insert(*cue_output) {
                    return Err(OutputProfileLock::Invalid(format!(
                        "machine ASIO output profile duplicates Output {cue_output}"
                    )));
                }
            }
            CueDelivery::ExplicitWdm {
                device_name,
                topology_fingerprint,
            } => {
                validate_profile_text(
                    "cueDelivery.deviceName",
                    device_name,
                    MAX_PROFILE_STRING_BYTES,
                )?;
                validate_profile_text(
                    "cueDelivery.topologyFingerprint",
                    topology_fingerprint,
                    MAX_PROFILE_STRING_BYTES,
                )?;
            }
        }
        if let Some(channel) = self.spare_output {
            if channel == 0 || channel > self.device_output_channels {
                return Err(OutputProfileLock::Invalid(format!(
                    "machine ASIO output profile spareOutput Output {channel} is outside Output 1..{}",
                    self.device_output_channels
                )));
            }
            if !selected.insert(channel) {
                return Err(OutputProfileLock::Invalid(format!(
                    "machine ASIO output profile duplicates Output {channel}"
                )));
            }
        }
        Ok(())
    }

    fn validate_capability_tuple(&self) -> Result<(), OutputProfileLock> {
        if !valid_profile_driver_id(&self.driver_id) {
            return Err(OutputProfileLock::Invalid(
                format!(
                    "machine ASIO output profile driverId must be an explicit asio:<driver name> identity without surrounding whitespace, control characters, or more than {MAX_PROFILE_STRING_BYTES} UTF-8 bytes"
                ),
            ));
        }
        if self.catalog_generation == 0 {
            return Err(OutputProfileLock::Invalid(
                "machine ASIO output profile catalogGeneration must be non-zero".to_owned(),
            ));
        }
        if self.sample_rate_hz == 0 {
            return Err(OutputProfileLock::Invalid(
                "machine ASIO output profile sampleRateHz must be non-zero".to_owned(),
            ));
        }
        if !matches!(
            self.native_format.as_str(),
            "f32" | "i16" | "i24" | "i32" | "f64"
        ) {
            return Err(OutputProfileLock::Invalid(
                "machine ASIO output profile nativeFormat is unsupported".to_owned(),
            ));
        }
        if self.fixed_buffer_frames == 0 || self.fixed_buffer_frames > MAX_FIXED_BUFFER_FRAMES {
            return Err(OutputProfileLock::Invalid(format!(
                "machine ASIO output profile fixedBufferFrames must be between 1 and {MAX_FIXED_BUFFER_FRAMES}"
            )));
        }
        if self.device_output_channels == 0
            || self.device_output_channels > MAX_DEVICE_OUTPUT_CHANNELS
        {
            return Err(OutputProfileLock::Invalid(format!(
                "machine ASIO output profile deviceOutputChannels must be between 1 and {MAX_DEVICE_OUTPUT_CHANNELS}"
            )));
        }
        Ok(())
    }

    fn zero_based(&self, one_based: u32) -> usize {
        debug_assert!(one_based > 0);
        (one_based - 1) as usize
    }

    fn program_left_index(&self) -> usize {
        self.zero_based(self.program_left_output)
    }

    fn program_right_index(&self) -> usize {
        self.zero_based(self.program_right_output)
    }

    fn cue_index(&self) -> usize {
        self.asio_cue_output_index()
            .expect("cue_index is only valid for sameAsio delivery")
    }
}

fn valid_profile_driver_id(value: &str) -> bool {
    value.starts_with("asio:")
        && value.len() > "asio:".len()
        && value.len() <= MAX_PROFILE_STRING_BYTES
        && value.trim() == value
        && !value.chars().any(char::is_control)
}

fn validate_profile_text(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), OutputProfileLock> {
    if value.trim().is_empty() {
        return Err(OutputProfileLock::Invalid(format!(
            "machine ASIO output profile {field} must not be empty"
        )));
    }
    if value.trim() != value {
        return Err(OutputProfileLock::Invalid(format!(
            "machine ASIO output profile {field} must not have surrounding whitespace"
        )));
    }
    if value.chars().any(char::is_control) {
        return Err(OutputProfileLock::Invalid(format!(
            "machine ASIO output profile {field} must not contain control characters"
        )));
    }
    if value.len() > max_bytes {
        return Err(OutputProfileLock::Invalid(format!(
            "machine ASIO output profile {field} exceeds the {max_bytes}-byte UTF-8 limit"
        )));
    }
    Ok(())
}

/// A snapshot of an already validated machine profile used by the pure
/// preflight mapper.  The snapshot owns no device handle and has no runtime or
/// persistence side effects; all physical channel resolution goes through the
/// validated profile captured here.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AsioPreflightMapper {
    profile: MachineAsioOutputProfile,
}

impl AsioPreflightMapper {
    /// Admit only a complete, validated machine profile.  A caller cannot
    /// provide a physical channel to this API; channels are taken solely from
    /// `MachineAsioOutputProfile`.
    pub(crate) fn new(profile: &MachineAsioOutputProfile) -> Result<Self, OutputProfileLock> {
        profile.validate()?;
        Ok(Self {
            profile: profile.clone(),
        })
    }

    /// Validate a logical test selection without touching a destination
    /// buffer.  In particular, Spare is admitted only when the profile has an
    /// explicit Spare mapping; there is no implicit last-channel/default
    /// fallback.
    pub(crate) fn validate_target(&self, target: PreflightTarget) -> Result<(), OutputProfileLock> {
        self.profile.validate()?;
        match target {
            PreflightTarget::Off => Err(OutputProfileLock::Invalid(
                "ASIO preflight Off target does not produce a test frame".to_owned(),
            )),
            PreflightTarget::Cue if self.profile.cue_delivery().is_external_wdm() => {
                Err(OutputProfileLock::Invalid(
                    "ASIO CUE preflight test is unavailable for external WDM delivery; the external WDM runtime owns CUE testing"
                        .to_owned(),
                ))
            }
            PreflightTarget::Spare if self.profile.spare_output.is_none() => {
                Err(OutputProfileLock::Invalid(
                    "ASIO preflight Spare target requires an explicitly selected Spare output"
                        .to_owned(),
                ))
            }
            _ => Ok(()),
        }
    }

    pub(crate) fn validate_solo(&self, mode: PreflightSolo) -> Result<(), OutputProfileLock> {
        if mode == PreflightSolo::CueOnly && self.profile.cue_delivery().is_external_wdm() {
            return Err(OutputProfileLock::Invalid(
                "ASIO CUE solo is unavailable for external WDM delivery; the external WDM runtime owns CUE testing"
                    .to_owned(),
            ));
        }
        Ok(())
    }

    /// Map one bounded operator test target into a complete device-width
    /// frame.  The destination is silenced before every validation and target
    /// lookup, including all unselected channels and an unavailable Spare.
    pub(crate) fn map_target(
        &self,
        target: PreflightTarget,
        destination: &mut [f32],
    ) -> Result<(), OutputProfileLock> {
        destination.fill(0.0);
        self.profile.validate()?;
        if destination.len() != self.profile.device_output_channels as usize {
            return Err(OutputProfileLock::Invalid(format!(
                "device frame has {} channels, expected {}",
                destination.len(),
                self.profile.device_output_channels
            )));
        }
        self.validate_target(target)?;

        match target {
            PreflightTarget::Off => unreachable!("Off was rejected by validate_target"),
            PreflightTarget::ProgramLeft => {
                destination[self.profile.program_left_index()] = PREFLIGHT_SAFE_AMPLITUDE;
            }
            PreflightTarget::ProgramRight => {
                destination[self.profile.program_right_index()] = PREFLIGHT_SAFE_AMPLITUDE;
            }
            PreflightTarget::ProgramStereo => {
                destination[self.profile.program_left_index()] = PREFLIGHT_SAFE_AMPLITUDE;
                destination[self.profile.program_right_index()] = PREFLIGHT_SAFE_AMPLITUDE;
            }
            PreflightTarget::Cue => {
                destination[self.profile.cue_index()] = PREFLIGHT_SAFE_AMPLITUDE;
            }
            PreflightTarget::Spare => {
                let spare = self.profile.spare_output.ok_or_else(|| {
                    OutputProfileLock::Invalid(
                        "ASIO preflight Spare target requires an explicitly selected Spare output"
                            .to_owned(),
                    )
                })?;
                destination[self.profile.zero_based(spare)] = PREFLIGHT_SAFE_AMPLITUDE;
            }
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActivePreflightTest {
    identity: AsioPreflightIdentity,
    target: PreflightTarget,
    expires_at_ms: u64,
}

/// The complete application identity carried by every ephemeral preflight
/// operation.  Transport generation alone is insufficient because a new ASIO
/// session may legitimately begin again at transport generation one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AsioPreflightIdentity {
    session_generation: u64,
    transport_generation: u64,
}

impl AsioPreflightIdentity {
    pub(crate) fn new(
        session_generation: u64,
        transport_generation: u64,
    ) -> Result<Self, PreflightStateError> {
        if session_generation == 0 || transport_generation == 0 {
            return Err(PreflightStateError::GenerationExhausted);
        }
        Ok(Self {
            session_generation,
            transport_generation,
        })
    }

    pub(crate) fn session_generation(self) -> u64 {
        self.session_generation
    }

    pub(crate) fn transport_generation(self) -> u64 {
        self.transport_generation
    }
}

/// Lifecycle admission for ephemeral operator preflight controls.  A test or
/// solo is meaningful only while the exact session/transport pair is Active;
/// Stopped and Faulted are retained as explicit non-admitting phases so a
/// stale UI action cannot accidentally reach a new stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PreflightPhase {
    #[cfg(test)]
    Inactive,
    Active,
    Stopped,
    Faulted,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PreflightStateError {
    IdentityMismatch {
        expected: AsioPreflightIdentity,
        actual: AsioPreflightIdentity,
    },
    GenerationExhausted,
    NotActive {
        phase: PreflightPhase,
        identity: AsioPreflightIdentity,
    },
    Faulted {
        identity: AsioPreflightIdentity,
    },
    #[cfg(test)]
    AlreadyActive,
    #[cfg(test)]
    StaleSessionIdentity {
        previous: AsioPreflightIdentity,
        requested: AsioPreflightIdentity,
    },
    LivePlaybackActive,
    LivePlaybackBlockedByTest(PreflightTarget),
    LivePlaybackBlockedBySolo(PreflightSolo),
    InvalidDuration,
    DurationOverflow,
    #[cfg(test)]
    SpareUnavailable,
    TestAlreadyActive,
    TestBlockedBySolo(PreflightSolo),
    SoloBlockedByTest(PreflightTarget),
    InvalidTransportRotation {
        current: AsioPreflightIdentity,
        next: AsioPreflightIdentity,
    },
    Mapping(OutputProfileLock),
}

impl std::fmt::Display for PreflightStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IdentityMismatch { expected, actual } => write!(
                f,
                "ASIO preflight identity mismatch: expected session {} / transport {}, got session {} / transport {}",
                expected.session_generation,
                expected.transport_generation,
                actual.session_generation,
                actual.transport_generation,
            ),
            Self::GenerationExhausted => {
                f.write_str("ASIO preflight transport generation is exhausted")
            }
            Self::NotActive { phase, identity } => write!(
                f,
                "ASIO preflight requires Active session {} / transport {}; current phase is {phase:?}",
                identity.session_generation,
                identity.transport_generation,
            ),
            Self::Faulted { identity } => write!(
                f,
                "ASIO preflight is blocked by terminal Fault for session {} / transport {}; Stop/Close and a fresh Start are required",
                identity.session_generation,
                identity.transport_generation,
            ),
            #[cfg(test)]
            Self::AlreadyActive => f.write_str("ASIO preflight session is already Active"),
            #[cfg(test)]
            Self::StaleSessionIdentity { previous, requested } => write!(
                f,
                "ASIO preflight activation identity is stale: previous session {} / transport {}, requested session {} / transport {}",
                previous.session_generation,
                previous.transport_generation,
                requested.session_generation,
                requested.transport_generation,
            ),
            Self::LivePlaybackActive => {
                f.write_str("ASIO preflight is blocked while live show playback is Active")
            }
            Self::LivePlaybackBlockedByTest(target) => write!(
                f,
                "live show playback cannot start while ASIO preflight {target:?} test is active"
            ),
            Self::LivePlaybackBlockedBySolo(mode) => write!(
                f,
                "live show playback cannot start while ASIO preflight {mode:?} solo is active"
            ),
            Self::InvalidDuration => write!(
                f,
                "ASIO preflight duration must be between 1 and {PREFLIGHT_MAX_DURATION_MS} ms"
            ),
            Self::DurationOverflow => {
                f.write_str("ASIO preflight expiry exceeded the supported clock range")
            }
            #[cfg(test)]
            Self::SpareUnavailable => {
                f.write_str("ASIO preflight Spare target requires an explicitly selected Spare output")
            }
            Self::TestAlreadyActive => f.write_str("an ASIO preflight test is already active"),
            Self::TestBlockedBySolo(mode) => {
                write!(f, "ASIO preflight test is blocked by {mode:?} solo")
            }
            Self::SoloBlockedByTest(target) => {
                write!(f, "ASIO preflight {target:?} test is already active")
            }
            Self::InvalidTransportRotation { current, next } => write!(
                f,
                "ASIO preflight transport rotation must stay in session {} and advance transport generation (current {}, next {})",
                current.session_generation,
                current.transport_generation,
                next.transport_generation,
            ),
            Self::Mapping(error) => error.fmt(f),
        }
    }
}

impl From<OutputProfileLock> for PreflightStateError {
    fn from(error: OutputProfileLock) -> Self {
        Self::Mapping(error)
    }
}

/// Ephemeral operator preflight state.  This intentionally has no serde
/// derives or storage methods: a test/solo selection expires or is explicitly
/// cleared on transport rotation, Stop, or Fault and can never become project
/// or machine-profile data.  The non-serializable cue availability bit only
/// binds this ephemeral state to the validated mapper's delivery mode.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct AsioPreflightState {
    identity: AsioPreflightIdentity,
    phase: PreflightPhase,
    live_playback_active: bool,
    engine_live_active: bool,
    cue_preflight_available: bool,
    test: Option<ActivePreflightTest>,
    solo: PreflightSolo,
}

impl AsioPreflightState {
    #[cfg(test)]
    pub(crate) fn new(
        session_generation: u64,
        transport_generation: u64,
    ) -> Result<Self, PreflightStateError> {
        let identity = AsioPreflightIdentity::new(session_generation, transport_generation)?;
        Ok(Self::from_identity(identity))
    }

    pub(crate) fn from_identity(identity: AsioPreflightIdentity) -> Self {
        Self::from_identity_in_phase(identity, PreflightPhase::Active)
    }

    /// Construct state bound to the validated machine profile.  External WDM
    /// CUE remains owned by its separate runtime, so this state rejects CUE
    /// Test/Solo admission while preserving the legacy sameAsio constructor.
    #[cfg(test)]
    pub(crate) fn from_identity_with_mapper(
        identity: AsioPreflightIdentity,
        mapper: &AsioPreflightMapper,
    ) -> Self {
        let mut state = Self::from_identity(identity);
        state.cue_preflight_available = mapper.profile.cue_delivery().is_same_asio();
        state
    }

    /// Construct a state holder before the ASIO session has been admitted.
    /// The identity is still retained so every later operation can be checked
    /// against the exact session/transport pair that the caller presents.
    #[cfg(test)]
    pub(crate) fn inactive_from_identity(identity: AsioPreflightIdentity) -> Self {
        Self::from_identity_in_phase(identity, PreflightPhase::Inactive)
    }

    fn from_identity_in_phase(identity: AsioPreflightIdentity, phase: PreflightPhase) -> Self {
        debug_assert!(identity.session_generation > 0);
        debug_assert!(identity.transport_generation > 0);
        Self {
            identity,
            phase,
            live_playback_active: false,
            engine_live_active: false,
            cue_preflight_available: true,
            test: None,
            solo: PreflightSolo::None,
        }
    }

    #[cfg(test)]
    pub(crate) fn identity(&self) -> AsioPreflightIdentity {
        self.identity
    }

    pub(crate) fn phase(&self) -> PreflightPhase {
        self.phase
    }

    pub(crate) fn live_playback_active(&self) -> bool {
        self.live_playback_active || self.engine_live_active
    }

    pub(crate) fn has_selection(&self) -> bool {
        self.test.is_some() || self.solo != PreflightSolo::None
    }

    pub(crate) fn selection_kind(&self) -> PreflightSelectionKind {
        if self.test.is_some() {
            PreflightSelectionKind::Test
        } else if self.solo != PreflightSolo::None {
            PreflightSelectionKind::Solo
        } else {
            PreflightSelectionKind::None
        }
    }

    /// Stable, callback-independent identity for the current ephemeral
    /// selection. It lets the worker fence a Solo mode change even though
    /// both Solo modes share the same Test/Solo classification byte.
    pub(crate) fn selection_signature(&self) -> u8 {
        if let Some(test) = self.test {
            return match test.target {
                PreflightTarget::Off => 0,
                PreflightTarget::ProgramLeft => 1,
                PreflightTarget::ProgramRight => 2,
                PreflightTarget::ProgramStereo => 3,
                PreflightTarget::Cue => 4,
                PreflightTarget::Spare => 5,
            };
        }
        match self.solo {
            PreflightSolo::None => 0,
            PreflightSolo::ProgramOnly => 6,
            PreflightSolo::CueOnly => 7,
        }
    }

    /// Admit a fresh ASIO session identity.  Reusing an identity from a
    /// stopped/faulted session is rejected so a stale UI/control message
    /// cannot arm preflight on a later stream.
    #[cfg(test)]
    pub(crate) fn activate(
        &mut self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        if self.phase == PreflightPhase::Active {
            if identity == self.identity {
                return Err(PreflightStateError::AlreadyActive);
            }
            return Err(PreflightStateError::IdentityMismatch {
                expected: self.identity,
                actual: identity,
            });
        }
        if self.phase == PreflightPhase::Faulted {
            return Err(PreflightStateError::Faulted {
                identity: self.identity,
            });
        }
        if self.phase != PreflightPhase::Inactive
            && identity.session_generation <= self.identity.session_generation
        {
            return Err(PreflightStateError::StaleSessionIdentity {
                previous: self.identity,
                requested: identity,
            });
        }
        self.identity = identity;
        self.phase = PreflightPhase::Active;
        self.live_playback_active = false;
        self.engine_live_active = false;
        self.clear_with_reason(PreflightClearReason::TransportRotation);
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn transport_generation(&self) -> u64 {
        self.identity.transport_generation
    }

    pub(crate) fn test_target(
        &mut self,
        identity: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<Option<PreflightTarget>, PreflightStateError> {
        self.require_active(identity)?;
        self.clear_if_expired_at(now_ms);
        Ok(self.test.map(|test| test.target))
    }

    pub(crate) fn solo(&self) -> PreflightSolo {
        self.solo
    }

    /// Arm one typed test for a bounded duration.  The caller supplies no
    /// physical channel, and an active non-Off solo blocks the test.
    #[cfg(test)]
    pub(crate) fn begin_test(
        &mut self,
        target: PreflightTarget,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        duration_ms: u64,
    ) -> Result<(), PreflightStateError> {
        self.require_active_and_quiet(identity)?;
        if target == PreflightTarget::Off {
            self.test = None;
            return Ok(());
        }
        if target == PreflightTarget::Cue && !self.cue_preflight_available {
            return Err(external_wdm_cue_test_rejected());
        }
        if target == PreflightTarget::Spare {
            return Err(PreflightStateError::SpareUnavailable);
        }
        self.begin_test_after_admission(target, identity, now_ms, duration_ms)
    }

    /// Profile-bound test admission.  This is the only path that can arm the
    /// optional Spare target; a state created without a validated profile
    /// fails closed instead of guessing a physical channel.
    pub(crate) fn begin_test_with_mapper(
        &mut self,
        mapper: &AsioPreflightMapper,
        target: PreflightTarget,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        duration_ms: u64,
    ) -> Result<(), PreflightStateError> {
        self.begin_test_with_mapper_and_live_fence(
            mapper,
            target,
            identity,
            now_ms,
            duration_ms,
            None,
            false,
        )
    }

    /// Profile-bound test admission with the authoritative Timeline live
    /// fence checked while this state is held.  A live owner permanently
    /// clears any pending Test/Solo selection; callers fence that transition
    /// before returning the actionable rejection to the UI.
    pub(crate) fn begin_test_with_mapper_and_live_fence(
        &mut self,
        mapper: &AsioPreflightMapper,
        target: PreflightTarget,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        duration_ms: u64,
        live_fence: Option<&TimelineAudioLiveFence>,
        drain_active: bool,
    ) -> Result<(), PreflightStateError> {
        self.require_active_and_quiet(identity)?;
        self.cue_preflight_available = mapper.profile.cue_delivery().is_same_asio();
        if target == PreflightTarget::Off {
            self.test = None;
            return Ok(());
        }
        if drain_active || live_fence.is_some_and(TimelineAudioLiveFence::active) {
            self.clear_with_reason(PreflightClearReason::EngineLive);
            return Err(PreflightStateError::LivePlaybackActive);
        }
        mapper
            .validate_target(target)
            .map_err(PreflightStateError::Mapping)?;
        self.begin_test_after_admission(target, identity, now_ms, duration_ms)
    }

    fn begin_test_after_admission(
        &mut self,
        target: PreflightTarget,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        duration_ms: u64,
    ) -> Result<(), PreflightStateError> {
        let expires_at_ms = checked_expiry(now_ms, duration_ms)?;
        self.clear_if_expired_at(now_ms);
        if let Some(mode) = non_none_solo(self.solo) {
            return Err(PreflightStateError::TestBlockedBySolo(mode));
        }
        if self.test.is_some() {
            return Err(PreflightStateError::TestAlreadyActive);
        }
        self.test = Some(ActivePreflightTest {
            identity,
            target,
            expires_at_ms,
        });
        Ok(())
    }

    /// Change solo mode only when the caller's transport generation is still
    /// current.  A non-Off solo cannot coexist with an active test; Off may be
    /// selected to release the solo gate without touching the test state.
    pub(crate) fn set_solo(
        &mut self,
        mode: PreflightSolo,
        identity: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<(), PreflightStateError> {
        self.set_solo_with_live_fence(mode, identity, now_ms, None, false)
    }

    #[cfg(test)]
    pub(crate) fn set_solo_with_mapper(
        &mut self,
        mapper: &AsioPreflightMapper,
        mode: PreflightSolo,
        identity: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<(), PreflightStateError> {
        self.set_solo_with_mapper_and_live_fence(mapper, mode, identity, now_ms, None, false)
    }

    #[cfg(test)]
    pub(crate) fn set_solo_with_mapper_and_live_fence(
        &mut self,
        mapper: &AsioPreflightMapper,
        mode: PreflightSolo,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        live_fence: Option<&TimelineAudioLiveFence>,
        drain_active: bool,
    ) -> Result<(), PreflightStateError> {
        self.set_solo_with_live_fence_inner(
            mode,
            identity,
            now_ms,
            live_fence,
            drain_active,
            Some(mapper),
        )
    }

    /// Solo admission with the authoritative Timeline live fence checked
    /// while this state is held.  Rejected live admission also clears a stale
    /// selection so it cannot resume when live playback exits.
    pub(crate) fn set_solo_with_live_fence(
        &mut self,
        mode: PreflightSolo,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        live_fence: Option<&TimelineAudioLiveFence>,
        drain_active: bool,
    ) -> Result<(), PreflightStateError> {
        self.set_solo_with_live_fence_inner(mode, identity, now_ms, live_fence, drain_active, None)
    }

    fn set_solo_with_live_fence_inner(
        &mut self,
        mode: PreflightSolo,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        live_fence: Option<&TimelineAudioLiveFence>,
        drain_active: bool,
        mapper: Option<&AsioPreflightMapper>,
    ) -> Result<(), PreflightStateError> {
        self.require_active_and_quiet(identity)?;
        if let Some(mapper) = mapper {
            self.cue_preflight_available = mapper.profile.cue_delivery().is_same_asio();
        }
        self.clear_if_expired_at(now_ms);
        if mode != PreflightSolo::None
            && (drain_active || live_fence.is_some_and(TimelineAudioLiveFence::active))
        {
            self.clear_with_reason(PreflightClearReason::EngineLive);
            return Err(PreflightStateError::LivePlaybackActive);
        }
        if let Some(mapper) = mapper {
            mapper
                .validate_solo(mode)
                .map_err(PreflightStateError::Mapping)?;
        } else if mode == PreflightSolo::CueOnly && !self.cue_preflight_available {
            return Err(external_wdm_cue_solo_rejected());
        }
        if mode != PreflightSolo::None {
            if let Some(test) = self.test {
                return Err(PreflightStateError::SoloBlockedByTest(test.target));
            }
        }
        self.solo = mode;
        Ok(())
    }

    /// Render the currently armed test, if any, through the profile-bound
    /// mapper.  A stale identity silences the destination and returns an error
    /// without mutating the state; expiry silences and returns `Ok(false)`.
    #[cfg(test)]
    pub(crate) fn render_test(
        &mut self,
        mapper: &AsioPreflightMapper,
        identity: AsioPreflightIdentity,
        now_ms: u64,
        destination: &mut [f32],
    ) -> Result<bool, PreflightStateError> {
        destination.fill(0.0);
        self.require_active_and_quiet(identity)?;
        self.clear_if_expired_at(now_ms);
        let Some(test) = self.test else {
            return Ok(false);
        };
        if test.identity != identity {
            destination.fill(0.0);
            return Err(PreflightStateError::IdentityMismatch {
                expected: self.identity,
                actual: test.identity,
            });
        }
        if self.solo != PreflightSolo::None {
            destination.fill(0.0);
            return Err(PreflightStateError::TestBlockedBySolo(self.solo));
        }
        mapper
            .map_target(test.target, destination)
            .map(|()| true)
            .map_err(PreflightStateError::Mapping)
    }

    pub(crate) fn clear_if_expired(
        &mut self,
        identity: AsioPreflightIdentity,
        now_ms: u64,
    ) -> Result<bool, PreflightStateError> {
        self.require_active(identity)?;
        Ok(self.clear_if_expired_at(now_ms))
    }

    /// Mark live show playback as owning the output.  Starting live playback
    /// is rejected while a preflight selection exists; stopping it is always
    /// allowed for the exact current identity.
    pub(crate) fn set_live_playback_active(
        &mut self,
        active: bool,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.require_identity(identity)?;
        if active {
            self.require_active(identity)?;
            if let Some(test) = self.test {
                return Err(PreflightStateError::LivePlaybackBlockedByTest(test.target));
            }
            if let Some(mode) = non_none_solo(self.solo) {
                return Err(PreflightStateError::LivePlaybackBlockedBySolo(mode));
            }
        }
        self.live_playback_active = active;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn begin_live_playback(
        &mut self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.set_live_playback_active(true, identity)
    }

    #[cfg(test)]
    pub(crate) fn end_live_playback(
        &mut self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.set_live_playback_active(false, identity)
    }

    /// Synchronize the worker-local view of Timeline live ownership.  The
    /// fence itself remains the authoritative cross-thread state; this bit is
    /// only a mutex-protected diagnostic/arming view and never restores a
    /// selection cleared while live was active. A changed monotonic fence word
    /// also retires a selection when an enter/exit pair completed between
    /// worker blocks, so the selection cannot resume after an unseen live
    /// interval.
    pub(crate) fn sync_engine_live(
        &mut self,
        identity: AsioPreflightIdentity,
        active: bool,
        fence_changed: bool,
    ) -> Result<bool, PreflightStateError> {
        self.require_identity(identity)?;
        let changed = (active || fence_changed) && self.has_selection();
        self.engine_live_active = active;
        if active || fence_changed {
            self.clear_with_reason(PreflightClearReason::EngineLive);
        }
        Ok(changed)
    }

    pub(crate) fn clear_for_stop(
        &mut self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.require_identity(identity)?;
        self.clear_with_reason(PreflightClearReason::Stop);
        self.live_playback_active = false;
        self.engine_live_active = false;
        self.phase = PreflightPhase::Stopped;
        Ok(())
    }

    pub(crate) fn clear_for_fault(
        &mut self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.require_identity(identity)?;
        self.clear_with_reason(PreflightClearReason::Fault);
        self.live_playback_active = false;
        self.engine_live_active = false;
        self.phase = PreflightPhase::Faulted;
        Ok(())
    }

    /// Install the explicitly rotated transport identity and clear all
    /// selections.  Both the current and successor identities are checked so
    /// a stale session cannot rotate or clear a newer session's state.
    pub(crate) fn clear_for_transport_rotation(
        &mut self,
        current: AsioPreflightIdentity,
        next: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.require_active(current)?;
        if next.session_generation == 0 || next.transport_generation == 0 {
            return Err(PreflightStateError::GenerationExhausted);
        }
        if next.session_generation != current.session_generation
            || next.transport_generation <= current.transport_generation
        {
            return Err(PreflightStateError::InvalidTransportRotation { current, next });
        }
        self.clear_with_reason(PreflightClearReason::TransportRotation);
        self.engine_live_active = false;
        self.identity = next;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn rotate_transport(
        &mut self,
        current: AsioPreflightIdentity,
    ) -> Result<AsioPreflightIdentity, PreflightStateError> {
        self.require_identity(current)?;
        let next_transport_generation = current
            .transport_generation
            .checked_add(1)
            .ok_or(PreflightStateError::GenerationExhausted)?;
        let next = AsioPreflightIdentity {
            session_generation: current.session_generation,
            transport_generation: next_transport_generation,
        };
        self.clear_for_transport_rotation(current, next)?;
        Ok(next)
    }

    fn clear_if_expired_at(&mut self, now_ms: u64) -> bool {
        if self.test.is_some_and(|test| now_ms >= test.expires_at_ms) {
            self.clear_with_reason(PreflightClearReason::Expired);
            true
        } else {
            false
        }
    }

    fn require_identity(&self, identity: AsioPreflightIdentity) -> Result<(), PreflightStateError> {
        if identity == self.identity {
            Ok(())
        } else {
            Err(PreflightStateError::IdentityMismatch {
                expected: self.identity,
                actual: identity,
            })
        }
    }

    fn require_active(&self, identity: AsioPreflightIdentity) -> Result<(), PreflightStateError> {
        self.require_identity(identity)?;
        match self.phase {
            PreflightPhase::Active => Ok(()),
            PreflightPhase::Faulted => Err(PreflightStateError::Faulted { identity }),
            phase => Err(PreflightStateError::NotActive { phase, identity }),
        }
    }

    fn require_active_and_quiet(
        &self,
        identity: AsioPreflightIdentity,
    ) -> Result<(), PreflightStateError> {
        self.require_active(identity)?;
        if self.live_playback_active {
            return Err(PreflightStateError::LivePlaybackActive);
        }
        Ok(())
    }

    fn clear_with_reason(&mut self, _reason: PreflightClearReason) {
        self.test = None;
        self.solo = PreflightSolo::None;
    }
}

fn non_none_solo(mode: PreflightSolo) -> Option<PreflightSolo> {
    match mode {
        PreflightSolo::None => None,
        other => Some(other),
    }
}

#[cfg(test)]
fn external_wdm_cue_test_rejected() -> PreflightStateError {
    PreflightStateError::Mapping(OutputProfileLock::Invalid(
        "ASIO CUE preflight test is unavailable for external WDM delivery; the external WDM runtime owns CUE testing"
            .to_owned(),
    ))
}

fn external_wdm_cue_solo_rejected() -> PreflightStateError {
    PreflightStateError::Mapping(OutputProfileLock::Invalid(
        "ASIO CUE solo is unavailable for external WDM delivery; the external WDM runtime owns CUE testing"
            .to_owned(),
    ))
}

fn checked_expiry(now_ms: u64, duration_ms: u64) -> Result<u64, PreflightStateError> {
    if duration_ms == 0 || duration_ms > PREFLIGHT_MAX_DURATION_MS {
        return Err(PreflightStateError::InvalidDuration);
    }
    now_ms
        .checked_add(duration_ms)
        .ok_or(PreflightStateError::DurationOverflow)
}

impl MachineAsioOutputProfileState {
    #[cfg(test)]
    pub(crate) fn locked_bytes(&self) -> Option<&[u8]> {
        match self {
            Self::Ready(_) => None,
            Self::Locked { original_bytes, .. } => Some(original_bytes),
        }
    }

    /// Lock is intentionally non-serializable.  The caller must collect a
    /// fresh selection and invoke `replace_from_explicit_reselection` only
    /// after live capability revalidation has succeeded.
    pub(crate) fn serialize_ready(&self) -> Result<Vec<u8>, OutputProfileLock> {
        match self {
            Self::Ready(profile) => {
                profile.validate()?;
                if profile.schema_version != MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION {
                    return Err(OutputProfileLock::Invalid(
                        "machine ASIO output profile serialization requires schema 2".to_owned(),
                    ));
                }
                let bytes = serde_json::to_vec(profile).map_err(|error| {
                    OutputProfileLock::Invalid(format!(
                        "machine ASIO output profile serialization failed: {error}"
                    ))
                })?;
                if bytes.len() > MAX_PROFILE_BYTES {
                    return Err(OutputProfileLock::Oversized);
                }
                Ok(bytes)
            }
            Self::Locked { reason, .. } => Err(OutputProfileLock::Invalid(format!(
                "machine ASIO output profile is locked and cannot be rewritten: {reason}"
            ))),
        }
    }

    #[cfg(test)]
    pub(crate) fn replace_from_explicit_reselection(
        &mut self,
        profile: MachineAsioOutputProfile,
        proof: &mut FreshOutputCapabilityProof,
    ) -> Result<(), OutputProfileLock> {
        profile.validate()?;
        proof.admit_once(&profile)?;
        *self = Self::Ready(profile);
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum CueFrame {
    #[cfg(test)]
    Mono(f32),
    Stereo {
        left: f32,
        right: f32,
    },
}

impl CueFrame {
    fn mono_gain_safe(self) -> Result<f32, OutputProfileLock> {
        let mono = match self {
            #[cfg(test)]
            Self::Mono(value) => value,
            Self::Stereo { left, right } => left * 0.5 + right * 0.5,
        };
        if !mono.is_finite() {
            return Err(OutputProfileLock::Invalid(
                "CUE frame contains non-finite audio".to_owned(),
            ));
        }
        Ok(mono)
    }
}

/// Maps one logical PROGRAM/CUE frame into a full device-width interleaved
/// frame.  The destination is cleared first, including Spare and all other
/// unselected channels; non-contiguous and reordered physical selection is
/// intentional and fully supported.
pub(crate) fn map_frame(
    profile: &MachineAsioOutputProfile,
    program_left: f32,
    program_right: f32,
    cue: CueFrame,
    destination: &mut [f32],
) -> Result<(), OutputProfileLock> {
    // Failures always silence the full caller-supplied frame, even when its
    // width is wrong.  Validate every source before assigning any mapping.
    destination.fill(0.0);
    profile.validate()?;
    if destination.len() != profile.device_output_channels as usize {
        return Err(OutputProfileLock::Invalid(format!(
            "device frame has {} channels, expected {}",
            destination.len(),
            profile.device_output_channels
        )));
    }
    if !program_left.is_finite() || !program_right.is_finite() {
        return Err(OutputProfileLock::Invalid(
            "PROGRAM frame contains non-finite audio".to_owned(),
        ));
    }
    let asio_cue = profile
        .cue_delivery()
        .is_same_asio()
        .then(|| cue.mono_gain_safe())
        .transpose()?;
    destination[profile.program_left_index()] = program_left;
    destination[profile.program_right_index()] = program_right;
    if let Some(cue) = asio_cue {
        destination[profile
            .asio_cue_output_index()
            .expect("sameAsio delivery must have an ASIO CUE output")] = cue;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> MachineAsioOutputProfile {
        MachineAsioOutputProfile {
            schema_version: 2,
            driver_id: "asio:Generic multichannel".to_owned(),
            catalog_generation: 9,
            sample_rate_hz: 48_000,
            native_format: "f32".to_owned(),
            fixed_buffer_frames: 256,
            device_output_channels: 7,
            program_left_output: 5,
            program_right_output: 1,
            cue_delivery: CueDelivery::SameAsio { cue_output: 7 },
            spare_output: Some(3),
        }
    }

    fn fresh_proof(profile: &MachineAsioOutputProfile, token: u64) -> FreshOutputCapabilityProof {
        FreshOutputCapabilityProof::from_live_revalidation(
            profile.driver_id.clone(),
            profile.catalog_generation,
            profile.sample_rate_hz,
            profile.native_format.clone(),
            profile.fixed_buffer_frames,
            profile.device_output_channels,
            token,
        )
        .unwrap()
    }

    fn preflight_mapper() -> AsioPreflightMapper {
        AsioPreflightMapper::new(&profile()).unwrap()
    }

    fn identity(session_generation: u64, transport_generation: u64) -> AsioPreflightIdentity {
        AsioPreflightIdentity::new(session_generation, transport_generation).unwrap()
    }

    #[test]
    fn noncontiguous_one_based_program_and_cue_mapping_zeroes_every_other_channel() {
        let mut frame = [99.0; 7];
        map_frame(
            &profile(),
            0.25,
            -0.5,
            CueFrame::Stereo {
                left: 0.8,
                right: -0.2,
            },
            &mut frame,
        )
        .unwrap();
        assert_eq!(frame, [-0.5, 0.0, 0.0, 0.0, 0.25, 0.0, 0.3]);
    }

    #[test]
    fn duplicate_or_out_of_range_machine_outputs_lock_without_default_fallback() {
        let mut duplicate = profile();
        duplicate.cue_delivery = CueDelivery::SameAsio {
            cue_output: duplicate.program_left_output,
        };
        assert!(matches!(
            duplicate.validate(),
            Err(OutputProfileLock::Invalid(_))
        ));
        let mut out_of_range = profile();
        out_of_range.cue_delivery = CueDelivery::SameAsio { cue_output: 8 };
        assert!(matches!(
            out_of_range.validate(),
            Err(OutputProfileLock::Invalid(_))
        ));
    }

    #[test]
    fn future_and_unknown_machine_profiles_fail_closed() {
        let future = serde_json::json!({
            "schemaVersion": 3,
            "driverId": "asio:Generic multichannel",
            "catalogGeneration": 9,
            "sampleRateHz": 48000,
            "nativeFormat": "f32",
            "fixedBufferFrames": 256,
            "deviceOutputChannels": 7,
            "programLeftOutput": 5,
            "programRightOutput": 1,
            "cueDelivery": {"mode": "sameAsio", "cueOutput": 7}
        });
        assert!(matches!(
            MachineAsioOutputProfile::parse_storage_text(&future.to_string()),
            Err(OutputProfileLock::FutureSchema { .. })
        ));
        let unknown = future
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .chain(std::iter::once((
                "unknown".to_owned(),
                serde_json::json!(true),
            )))
            .collect::<serde_json::Map<_, _>>();
        assert!(matches!(
            MachineAsioOutputProfile::parse_storage_text(
                &serde_json::Value::Object(unknown).to_string()
            ),
            Err(OutputProfileLock::FutureSchema { .. })
        ));
        let duplicate = r#"{"schemaVersion":1,"driverId":"asio:Generic multichannel","catalogGeneration":9,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":7,"programLeftOutput":5,"programRightOutput":1,"cueOutput":7,"cueOutput":7}"#;
        assert!(matches!(
            MachineAsioOutputProfile::parse_storage_text(duplicate),
            Err(OutputProfileLock::InvalidJson(_))
        ));
    }

    fn external_profile() -> MachineAsioOutputProfile {
        let mut profile = profile();
        profile.cue_delivery = CueDelivery::ExplicitWdm {
            device_name: "WDM Cue Device".to_owned(),
            topology_fingerprint: "wdm-topology-v1".to_owned(),
        };
        profile
    }

    #[test]
    fn v1_storage_migrates_once_to_same_asio_and_serializes_only_v2() {
        let legacy = r#"{
            "schemaVersion":1,
            "driverId":"asio:Generic multichannel",
            "catalogGeneration":9,
            "sampleRateHz":48000,
            "nativeFormat":"f32",
            "fixedBufferFrames":256,
            "deviceOutputChannels":7,
            "programLeftOutput":5,
            "programRightOutput":1,
            "cueOutput":7,
            "spareOutput":3
        }"#;
        let migrated = MachineAsioOutputProfile::parse_storage_text(legacy).unwrap();
        assert_eq!(
            migrated.schema_version,
            MACHINE_OUTPUT_PROFILE_SCHEMA_VERSION
        );
        assert!(matches!(
            migrated.cue_delivery(),
            CueDelivery::SameAsio { cue_output: 7 }
        ));
        assert_eq!(migrated.asio_cue_output_index(), Some(6));

        let state = MachineAsioOutputProfileState::Ready(migrated);
        let serialized = state.serialize_ready().unwrap();
        let value: serde_json::Value = serde_json::from_slice(&serialized).unwrap();
        assert_eq!(value["schemaVersion"], serde_json::json!(2));
        assert_eq!(
            value["cueDelivery"],
            serde_json::json!({"mode":"sameAsio", "cueOutput":7})
        );
        assert!(value.get("cueOutput").is_none());
    }

    #[test]
    fn v2_same_asio_roundtrip_preserves_exact_cue_delivery_shape() {
        let bytes = serde_json::to_vec(&profile()).unwrap();
        let parsed =
            MachineAsioOutputProfile::parse_storage_text(std::str::from_utf8(&bytes).unwrap())
                .unwrap();
        assert_eq!(parsed, profile());
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&parsed_state_bytes(&parsed)).unwrap()
                ["cueDelivery"],
            serde_json::json!({"mode":"sameAsio", "cueOutput":7})
        );
    }

    fn parsed_state_bytes(profile: &MachineAsioOutputProfile) -> Vec<u8> {
        MachineAsioOutputProfileState::Ready(profile.clone())
            .serialize_ready()
            .unwrap()
    }

    #[test]
    fn external_wdm_validation_has_no_asio_cue_channel_and_rejects_invalid_strings() {
        let external = external_profile();
        assert!(external.validate().is_ok());
        assert!(external.cue_delivery().is_external_wdm());
        assert_eq!(external.asio_cue_output_index(), None);

        let mut duplicate = external.clone();
        duplicate.spare_output = Some(duplicate.program_left_output);
        assert!(matches!(
            duplicate.validate(),
            Err(OutputProfileLock::Invalid(_))
        ));

        for (device_name, topology_fingerprint) in [
            ("", "wdm-topology-v1"),
            (" WDM Cue Device", "wdm-topology-v1"),
            ("WDM\u{0007}Cue Device", "wdm-topology-v1"),
            ("WDM Cue Device", ""),
            ("WDM Cue Device", " wdm-topology-v1"),
            ("WDM Cue Device", "wdm\u{0007}topology-v1"),
        ] {
            let mut invalid = external.clone();
            invalid.cue_delivery = CueDelivery::ExplicitWdm {
                device_name: device_name.to_owned(),
                topology_fingerprint: topology_fingerprint.to_owned(),
            };
            assert!(matches!(
                invalid.validate(),
                Err(OutputProfileLock::Invalid(_))
            ));
        }

        let mut oversize = external;
        oversize.cue_delivery = CueDelivery::ExplicitWdm {
            device_name: "x".repeat(MAX_PROFILE_STRING_BYTES + 1),
            topology_fingerprint: "wdm-topology-v1".to_owned(),
        };
        assert!(matches!(
            oversize.validate(),
            Err(OutputProfileLock::Invalid(_))
        ));
    }

    #[test]
    fn v2_explicit_wdm_roundtrip_is_camel_case_and_invalid_variants_preserve_bytes_locked() {
        let external = external_profile();
        let bytes = parsed_state_bytes(&external);
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["cueDelivery"],
            serde_json::json!({
                "mode": "explicitWdm",
                "deviceName": "WDM Cue Device",
                "topologyFingerprint": "wdm-topology-v1"
            })
        );
        assert_eq!(
            MachineAsioOutputProfile::parse_storage_text(std::str::from_utf8(&bytes).unwrap())
                .unwrap(),
            external
        );

        let invalid_cases = [
            r#"{"schemaVersion":2,"driverId":"asio:Generic multichannel","catalogGeneration":9,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":7,"programLeftOutput":5,"programRightOutput":1,"cueDelivery":{"mode":"explicitWdm","deviceName":"WDM Cue Device","topologyFingerprint":"wdm-topology-v1","unknown":true},"spareOutput":3}"#,
            r#"{"schemaVersion":2,"driverId":"asio:Generic multichannel","catalogGeneration":9,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":7,"programLeftOutput":5,"programRightOutput":1,"cueDelivery":{"mode":"futureRoute","deviceName":"WDM Cue Device","topologyFingerprint":"wdm-topology-v1"},"spareOutput":3}"#,
            r#"{"schemaVersion":3,"driverId":"asio:Generic multichannel","catalogGeneration":9,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":7,"programLeftOutput":5,"programRightOutput":1,"cueDelivery":{"mode":"explicitWdm","deviceName":"WDM Cue Device","topologyFingerprint":"wdm-topology-v1"},"spareOutput":3}"#,
        ];
        for text in invalid_cases {
            let original = text.as_bytes().to_vec();
            let state = MachineAsioOutputProfile::restore_storage_bytes(original.clone());
            assert_eq!(state.locked_bytes(), Some(original.as_slice()));
            assert!(state.serialize_ready().is_err());
        }
    }

    #[test]
    fn external_wdm_keeps_asio_cue_silent_and_rejects_cue_test_and_solo() {
        let external = external_profile();
        let mapper = AsioPreflightMapper::new(&external).unwrap();
        assert!(matches!(
            mapper.validate_target(PreflightTarget::Cue),
            Err(OutputProfileLock::Invalid(message)) if message.contains("external WDM")
        ));

        let mut frame = [99.0; 7];
        map_frame(
            &external,
            0.25,
            -0.5,
            CueFrame::Stereo {
                left: 0.8,
                right: -0.2,
            },
            &mut frame,
        )
        .unwrap();
        assert_eq!(frame, [-0.5, 0.0, 0.0, 0.0, 0.25, 0.0, 0.0]);

        frame.fill(99.0);
        assert!(mapper.map_target(PreflightTarget::Cue, &mut frame).is_err());
        assert_eq!(frame, [0.0; 7]);

        let identity = identity(40, 1);
        let mut state = AsioPreflightState::from_identity_with_mapper(identity, &mapper);
        assert!(matches!(
            state.begin_test_with_mapper(&mapper, PreflightTarget::Cue, identity, 0, 100),
            Err(PreflightStateError::Mapping(OutputProfileLock::Invalid(message)))
                if message.contains("external WDM")
        ));
        assert!(matches!(
            state.set_solo(PreflightSolo::CueOnly, identity, 0),
            Err(PreflightStateError::Mapping(OutputProfileLock::Invalid(message)))
                if message.contains("external WDM")
        ));
        assert!(matches!(
            state.set_solo_with_mapper(&mapper, PreflightSolo::CueOnly, identity, 0),
            Err(PreflightStateError::Mapping(OutputProfileLock::Invalid(message)))
                if message.contains("external WDM")
        ));
        assert_eq!(state.solo(), PreflightSolo::None);
    }

    #[test]
    fn nested_duplicate_keys_are_rejected_before_v2_deserialization() {
        let duplicate = r#"{"schemaVersion":2,"driverId":"asio:Generic multichannel","catalogGeneration":9,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":7,"programLeftOutput":5,"programRightOutput":1,"cueDelivery":{"mode":"sameAsio","cueOutput":7,"cueOutput":7}}"#;
        assert!(matches!(
            MachineAsioOutputProfile::parse_storage_text(duplicate),
            Err(OutputProfileLock::InvalidJson(_))
        ));
    }

    #[test]
    fn locked_machine_profile_preserves_all_invalid_bytes_until_explicit_reselection() {
        let valid = serde_json::to_vec(&profile()).unwrap();
        assert!(matches!(
            MachineAsioOutputProfile::restore_storage_bytes(valid),
            MachineAsioOutputProfileState::Ready(_)
        ));

        let cases = vec![
            b"{not JSON".to_vec(),
            vec![0xff, 0xfe, 0xfd],
            vec![b'x'; MAX_PROFILE_BYTES + 1],
            br#"{"schemaVersion":3,"driverId":"asio:Future","catalogGeneration":1,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":3,"programLeftOutput":1,"programRightOutput":2,"cueDelivery":{"mode":"sameAsio","cueOutput":3}}"#.to_vec(),
            br#"{"schemaVersion":1,"driverId":"asio:Unknown","catalogGeneration":1,"sampleRateHz":48000,"nativeFormat":"f32","fixedBufferFrames":256,"deviceOutputChannels":3,"programLeftOutput":1,"programRightOutput":2,"cueOutput":3,"unknown":true}"#.to_vec(),
        ];
        for bytes in cases {
            let original = bytes.clone();
            let mut state = MachineAsioOutputProfile::restore_storage_bytes(bytes);
            assert_eq!(state.locked_bytes(), Some(original.as_slice()));
            assert!(state.serialize_ready().is_err());
            let replacement = profile();
            let mut proof = fresh_proof(&replacement, 1);
            state
                .replace_from_explicit_reselection(replacement, &mut proof)
                .unwrap();
            assert_eq!(state.locked_bytes(), None);
            assert!(state.serialize_ready().is_ok());
        }
    }

    #[test]
    fn reselection_requires_matching_fresh_capability_proof_and_rejects_cloned_replay() {
        let replacement = profile();
        let mut state = MachineAsioOutputProfileState::Locked {
            original_bytes: b"bad".to_vec(),
            reason: OutputProfileLock::InvalidJson("bad".to_owned()),
        };
        let mut stale = fresh_proof(&replacement, 11);
        let mut changed = replacement.clone();
        changed.catalog_generation += 1;
        assert!(state
            .replace_from_explicit_reselection(changed, &mut stale)
            .is_err());
        assert!(matches!(
            state,
            MachineAsioOutputProfileState::Locked { .. }
        ));

        let mut fresh = fresh_proof(&replacement, 12);
        // This is the concrete replay proof: Clone is allowed only because
        // both values retain the same one-shot authority bit.
        let mut cloned_replay = fresh.clone();
        state
            .replace_from_explicit_reselection(replacement.clone(), &mut fresh)
            .unwrap();
        let mut another_locked = MachineAsioOutputProfileState::Locked {
            original_bytes: b"bad-again".to_vec(),
            reason: OutputProfileLock::InvalidJson("bad-again".to_owned()),
        };
        assert!(another_locked
            .replace_from_explicit_reselection(replacement, &mut cloned_replay)
            .is_err());
        assert!(matches!(
            another_locked,
            MachineAsioOutputProfileState::Locked { .. }
        ));
    }

    #[test]
    fn mapper_failure_silences_the_entire_destination_before_any_channel_write() {
        let mut frame = [0.75; 7];
        assert!(map_frame(
            &profile(),
            0.25,
            -0.5,
            CueFrame::Stereo {
                left: f32::NAN,
                right: 0.0,
            },
            &mut frame,
        )
        .is_err());
        assert_eq!(frame, [0.0; 7]);

        frame.fill(0.75);
        assert!(map_frame(&profile(), f32::NAN, -0.5, CueFrame::Mono(0.25), &mut frame,).is_err());
        assert_eq!(frame, [0.0; 7]);
    }

    #[test]
    fn logical_buses_remain_project_level_and_mono_cue_is_unchanged() {
        assert_eq!(CueFrame::Mono(0.25).mono_gain_safe().unwrap(), 0.25);
    }

    #[test]
    fn preflight_targets_use_exact_reordered_profile_channels_and_safe_level() {
        let mapper = preflight_mapper();
        let cases = [
            (
                PreflightTarget::ProgramLeft,
                [0.0, 0.0, 0.0, 0.0, PREFLIGHT_SAFE_AMPLITUDE, 0.0, 0.0],
            ),
            (
                PreflightTarget::ProgramRight,
                [PREFLIGHT_SAFE_AMPLITUDE, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
            ),
            (
                PreflightTarget::ProgramStereo,
                [
                    PREFLIGHT_SAFE_AMPLITUDE,
                    0.0,
                    0.0,
                    0.0,
                    PREFLIGHT_SAFE_AMPLITUDE,
                    0.0,
                    0.0,
                ],
            ),
            (
                PreflightTarget::Cue,
                [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, PREFLIGHT_SAFE_AMPLITUDE],
            ),
            (
                PreflightTarget::Spare,
                [0.0, 0.0, PREFLIGHT_SAFE_AMPLITUDE, 0.0, 0.0, 0.0, 0.0],
            ),
        ];
        for (target, expected) in cases {
            let mut destination = [99.0; 7];
            mapper.map_target(target, &mut destination).unwrap();
            assert_eq!(destination, expected);
            assert!(destination
                .iter()
                .all(|sample| sample.abs() <= PREFLIGHT_SAFE_AMPLITUDE));
        }
    }

    #[test]
    fn preflight_rejects_invalid_profiles_and_unavailable_spare_without_output() {
        let mut duplicate = profile();
        duplicate.spare_output = Some(duplicate.program_left_output);
        assert!(AsioPreflightMapper::new(&duplicate).is_err());

        let mut out_of_range = profile();
        out_of_range.spare_output = Some(out_of_range.device_output_channels + 1);
        assert!(AsioPreflightMapper::new(&out_of_range).is_err());

        let mut no_spare = profile();
        no_spare.spare_output = None;
        let mapper = AsioPreflightMapper::new(&no_spare).unwrap();
        let mut destination = [99.0; 7];
        assert!(mapper
            .map_target(PreflightTarget::Spare, &mut destination)
            .is_err());
        assert_eq!(destination, [0.0; 7]);
    }

    #[test]
    fn preflight_requires_active_phase_and_rejects_reused_session_identity() {
        let active = identity(30, 1);
        let mut state = AsioPreflightState::inactive_from_identity(active);
        assert_eq!(state.phase(), PreflightPhase::Inactive);
        assert!(matches!(
            state.begin_test(PreflightTarget::ProgramLeft, active, 0, 1),
            Err(PreflightStateError::NotActive {
                phase: PreflightPhase::Inactive,
                ..
            })
        ));

        state.activate(active).unwrap();
        state
            .begin_test(PreflightTarget::ProgramLeft, active, 0, 1)
            .unwrap();
        state.clear_for_stop(active).unwrap();
        assert_eq!(state.phase(), PreflightPhase::Stopped);
        assert!(matches!(
            state.activate(active),
            Err(PreflightStateError::StaleSessionIdentity { .. })
        ));

        let next_session = identity(31, 1);
        state.activate(next_session).unwrap();
        assert_eq!(state.phase(), PreflightPhase::Active);
        assert_eq!(state.solo(), PreflightSolo::None);
        assert!(state
            .test_target(next_session, 1)
            .expect("active identity query must be admitted")
            .is_none());
    }

    #[test]
    fn preflight_rejects_live_playback_and_prevents_live_start_with_selection() {
        let active = identity(32, 1);
        let mut state = AsioPreflightState::from_identity(active);

        state.begin_live_playback(active).unwrap();
        assert!(state.live_playback_active());
        assert!(matches!(
            state.begin_test(PreflightTarget::Cue, active, 0, 100),
            Err(PreflightStateError::LivePlaybackActive)
        ));
        assert!(matches!(
            state.set_solo(PreflightSolo::ProgramOnly, active, 0),
            Err(PreflightStateError::LivePlaybackActive)
        ));
        state.end_live_playback(active).unwrap();

        state
            .begin_test(PreflightTarget::Cue, active, 0, 100)
            .unwrap();
        assert!(matches!(
            state.begin_live_playback(active),
            Err(PreflightStateError::LivePlaybackBlockedByTest(
                PreflightTarget::Cue
            ))
        ));
        state
            .begin_test(PreflightTarget::Off, active, 1, 1)
            .unwrap();
        state.set_solo(PreflightSolo::CueOnly, active, 1).unwrap();
        assert!(matches!(
            state.begin_live_playback(active),
            Err(PreflightStateError::LivePlaybackBlockedBySolo(
                PreflightSolo::CueOnly
            ))
        ));
        state.set_solo(PreflightSolo::None, active, 1).unwrap();
        state.begin_live_playback(active).unwrap();
        assert!(matches!(
            state.set_solo(PreflightSolo::ProgramOnly, active, 2),
            Err(PreflightStateError::LivePlaybackActive)
        ));
    }

    #[test]
    fn preflight_spare_requires_profile_bound_admission_and_off_is_silent() {
        let active = identity(33, 1);
        let mut state = AsioPreflightState::from_identity(active);
        assert!(matches!(
            state.begin_test(PreflightTarget::Spare, active, 0, 100),
            Err(PreflightStateError::SpareUnavailable)
        ));

        let mut no_spare = profile();
        no_spare.spare_output = None;
        let no_spare_mapper = AsioPreflightMapper::new(&no_spare).unwrap();
        assert!(matches!(
            state.begin_test_with_mapper(&no_spare_mapper, PreflightTarget::Spare, active, 0, 100,),
            Err(PreflightStateError::Mapping(OutputProfileLock::Invalid(_)))
        ));

        let mapper = preflight_mapper();
        state
            .begin_test_with_mapper(&mapper, PreflightTarget::Spare, active, 0, 100)
            .unwrap();
        assert_eq!(
            state.test_target(active, 1).unwrap(),
            Some(PreflightTarget::Spare)
        );
        state
            .begin_test(PreflightTarget::Off, active, 1, 1)
            .unwrap();
        assert_eq!(state.test_target(active, 1).unwrap(), None);
    }

    #[test]
    fn preflight_duration_is_bounded_and_overflow_safe() {
        let active = identity(34, 1);
        let mut state = AsioPreflightState::from_identity(active);
        assert!(matches!(
            state.begin_test(PreflightTarget::Cue, active, 0, 0),
            Err(PreflightStateError::InvalidDuration)
        ));
        assert!(matches!(
            state.begin_test(
                PreflightTarget::Cue,
                active,
                0,
                PREFLIGHT_MAX_DURATION_MS + 1,
            ),
            Err(PreflightStateError::InvalidDuration)
        ));
        assert!(matches!(
            state.begin_test(PreflightTarget::Cue, active, u64::MAX - 1, 2),
            Err(PreflightStateError::DurationOverflow)
        ));
        state
            .begin_test(PreflightTarget::Cue, active, 0, PREFLIGHT_MAX_DURATION_MS)
            .unwrap();
        assert!(state
            .test_target(active, PREFLIGHT_MAX_DURATION_MS - 1)
            .unwrap()
            .is_some());
        assert!(state
            .test_target(active, PREFLIGHT_MAX_DURATION_MS)
            .unwrap()
            .is_none());
    }

    #[test]
    fn preflight_test_and_solo_are_mutually_exclusive() {
        let active = identity(3, 9);
        let mut state = AsioPreflightState::from_identity(active);
        state
            .begin_test(PreflightTarget::ProgramLeft, active, 100, 1000)
            .unwrap();
        assert!(matches!(
            state.set_solo(PreflightSolo::ProgramOnly, active, 100),
            Err(PreflightStateError::SoloBlockedByTest(
                PreflightTarget::ProgramLeft
            ))
        ));

        let next = state.rotate_transport(active).unwrap();
        state.set_solo(PreflightSolo::CueOnly, next, 100).unwrap();
        assert!(matches!(
            state.begin_test(PreflightTarget::Cue, next, 100, 1000),
            Err(PreflightStateError::TestBlockedBySolo(
                PreflightSolo::CueOnly
            ))
        ));
        state.set_solo(PreflightSolo::None, next, 100).unwrap();
        state
            .begin_test(PreflightTarget::Cue, next, 100, 1000)
            .unwrap();
    }

    #[test]
    fn preflight_expiry_and_explicit_stop_fault_clear_silence_state() {
        let mapper = preflight_mapper();
        let active = identity(7, 4);
        let mut state = AsioPreflightState::from_identity(active);
        state
            .begin_test(PreflightTarget::ProgramStereo, active, 100, 100)
            .unwrap();

        let mut destination = [99.0; 7];
        assert!(state
            .render_test(&mapper, active, 199, &mut destination)
            .unwrap());
        assert_eq!(destination[0], PREFLIGHT_SAFE_AMPLITUDE);
        assert_eq!(destination[4], PREFLIGHT_SAFE_AMPLITUDE);

        destination.fill(99.0);
        assert!(!state
            .render_test(&mapper, active, 200, &mut destination)
            .unwrap());
        assert_eq!(destination, [0.0; 7]);
        assert_eq!(state.test_target(active, 200).unwrap(), None);

        state
            .begin_test(PreflightTarget::Cue, active, 300, 100)
            .unwrap();
        state.clear_for_stop(active).unwrap();
        assert_eq!(state.phase(), PreflightPhase::Stopped);
        assert!(matches!(
            state.test_target(active, 300),
            Err(PreflightStateError::NotActive {
                phase: PreflightPhase::Stopped,
                ..
            })
        ));
        let faulted = identity(8, 1);
        state.activate(faulted).unwrap();
        state
            .begin_test_with_mapper(&mapper, PreflightTarget::Spare, faulted, 400, 100)
            .unwrap();
        state.clear_for_fault(faulted).unwrap();
        assert_eq!(state.phase(), PreflightPhase::Faulted);
        assert!(matches!(
            state.test_target(faulted, 400),
            Err(PreflightStateError::Faulted { .. })
        ));
        assert!(matches!(
            state.activate(identity(9, 1)),
            Err(PreflightStateError::Faulted { .. })
        ));
        state.clear_for_stop(faulted).unwrap();
        state.activate(identity(9, 1)).unwrap();
        assert_eq!(state.phase(), PreflightPhase::Active);
        assert_eq!(state.solo(), PreflightSolo::None);
    }

    #[test]
    fn preflight_transport_rotation_clears_and_old_generation_is_rejected() {
        let mapper = preflight_mapper();
        let active = identity(5, 11);
        let mut state = AsioPreflightState::from_identity(active);
        assert_eq!(state.transport_generation(), 11);
        state
            .begin_test(PreflightTarget::Cue, active, 10, 1000)
            .unwrap();
        let mut destination = [99.0; 7];
        assert!(matches!(
            state.render_test(&mapper, identity(5, 12), 10, &mut destination),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            })
            if expected == active && actual == identity(5, 12)
        ));
        assert_eq!(destination, [0.0; 7]);
        // A stale render is rejected without clearing the current identity's
        // test; only an explicit rotation/stop/fault clear may retire it.
        assert_eq!(
            state.test_target(active, 10).unwrap(),
            Some(PreflightTarget::Cue)
        );

        state
            .set_solo(PreflightSolo::ProgramOnly, active, 10)
            .unwrap_err();
        let next = state.rotate_transport(active).unwrap();
        assert_eq!(next, identity(5, 12));
        assert_eq!(state.identity(), next);
        assert_eq!(state.solo(), PreflightSolo::None);
        assert_eq!(state.test_target(next, 10).unwrap(), None);
    }

    #[test]
    fn preflight_full_identity_rejects_zero_stale_and_session_rollover_collision() {
        assert!(AsioPreflightIdentity::new(0, 1).is_err());
        assert!(AsioPreflightIdentity::new(1, 0).is_err());

        let old = identity(21, 1);
        let current = identity(22, 1);
        assert_eq!(current.session_generation(), 22);
        assert_eq!(current.transport_generation(), 1);
        let mapper = preflight_mapper();
        let mut state = AsioPreflightState::new(22, 1).unwrap();
        let mut destination = [99.0; 7];

        assert!(matches!(
            state.begin_test(PreflightTarget::ProgramLeft, old, 100, 1000),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert!(matches!(
            state.set_solo(PreflightSolo::ProgramOnly, old, 100),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert!(matches!(
            state.render_test(&mapper, old, 100, &mut destination),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert_eq!(destination, [0.0; 7]);
        assert!(matches!(
            state.clear_if_expired(old, 100),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert!(matches!(
            state.clear_for_stop(old),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert!(matches!(
            state.clear_for_fault(old),
            Err(PreflightStateError::IdentityMismatch {
                expected,
                actual
            }) if expected == current && actual == old
        ));
        assert_eq!(state.identity(), current);
        assert_eq!(state.test_target(current, 100).unwrap(), None);

        state
            .begin_test(PreflightTarget::Cue, current, 100, 1000)
            .unwrap();
        destination.fill(99.0);
        assert!(state
            .render_test(&mapper, old, 100, &mut destination)
            .is_err());
        assert_eq!(destination, [0.0; 7]);
        // A session rollover collision cannot clear or render the new
        // session's selection merely because both sessions use transport 1.
        assert_eq!(
            state.test_target(current, 100).unwrap(),
            Some(PreflightTarget::Cue)
        );
        assert!(state.clear_for_stop(old).is_err());
        assert_eq!(
            state.test_target(current, 100).unwrap(),
            Some(PreflightTarget::Cue)
        );
        state.clear_for_stop(current).unwrap();
        assert!(matches!(
            state.test_target(current, 100),
            Err(PreflightStateError::NotActive {
                phase: PreflightPhase::Stopped,
                ..
            })
        ));
    }
}
