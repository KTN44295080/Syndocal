//! Bounded, in-memory support packages. There is deliberately no filesystem,
//! process, device, clock, environment lookup, or publication in this module.
//!
//! Integration: register `mod diagnostic_package;` in main.rs, pass the existing
//! four `(name, Vec<u8>)` JSON entries to `build_diagnostic_package`, and publish
//! the returned bytes only after the operator has previewed the included data.
//! Generate before creating/truncating a destination; use a separately reviewed
//! atomic publication path. Do not pass a crash directory or append raw logs.
//!
//! Input unknown JSON fields are discarded, including their keys. Present known
//! fields must have the types/identities below. Missing optional counters stay
//! absent (they are not invented as zero). Invalid JSON, duplicate JSON keys,
//! structural limits, and invalid known values fail closed with data-free errors.
//! The app/version identity is pinned to this build; checking a package from a
//! different app version requires that version's validator. No arbitrary version
//! suffix, backend label, error detail, endpoint, path, or credential is retained.
//!
//! The format is deliberately a canonical, five-file STORED ZIP, with fixed
//! order, timestamp, permissions, and no comments/extra metadata. The validator
//! accepts this format, not arbitrary repackaged ZIPs. It bounds and reads local
//! records itself, then requires a byte-identical ZIP reconstructed with `zip`.
//! This also checks the central directory/CRC/metadata and catches duplicates
//! even if a generic ZIP reader would hide duplicate names. No untrusted ZIP
//! count, decompressor, or central-directory allocation is involved.
//!
//! SHA256 covers each sanitized payload; the generated integrity manifest is
//! verified by exact reconstruction, not a circular self-hash. These unkeyed
//! hashes detect corruption, not authenticity or a malicious author's replacement
//! of both payload and hashes. This module does not close all P2/P observability
//! work: structured safe crash state, rotation, UI preview, atomic publication,
//! and operator/support runbooks remain separate acceptance boundaries.

use serde::de::{self, DeserializeSeed, MapAccess, SeqAccess, Visitor};
use serde::Serialize;
use serde_json::{Map, Number, Value};
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::{self, Cursor, Seek, SeekFrom, Write};

pub(crate) const MAX_INPUT_ENTRY_BYTES: usize = 256 * 1024;
pub(crate) const MAX_TOTAL_INPUT_BYTES: usize = 768 * 1024;
pub(crate) const MAX_ARCHIVE_BYTES: usize = 160 * 1024;
const MAX_JSON_DEPTH: usize = 16;
const MAX_JSON_NODES: usize = 16_384;
const MAX_JSON_CONTAINER_ITEMS: usize = 1024;
const MAX_JSON_STRING_BYTES: usize = 4096;
const MAX_SANITIZED_ENTRY_BYTES: usize = 64 * 1024;
const MAX_TOTAL_SANITIZED_BYTES: usize = 128 * 1024;
const PAYLOAD_NAMES: [&str; 4] = [
    "manifest.json",
    "project-summary.json",
    "engine-telemetry.json",
    "video-runtime.json",
];
const INTEGRITY_NAME: &str = "integrity-manifest.json";
const ZIP_NAMES: [&str; 5] = [
    PAYLOAD_NAMES[0],
    PAYLOAD_NAMES[1],
    PAYLOAD_NAMES[2],
    PAYLOAD_NAMES[3],
    INTEGRITY_NAME,
];

/// Neither Display nor Debug contains caller-supplied strings or parser errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DiagnosticPackageError {
    EntryCount,
    EntryName,
    DuplicateEntry,
    InputLimit,
    InvalidJson,
    InvalidField,
    OutputLimit,
    InvalidArchive,
    IntegrityMismatch,
    ArchiveWrite,
}

impl fmt::Display for DiagnosticPackageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EntryCount => {
                "Diagnostic package requires exactly the four supported JSON entries."
            }
            Self::EntryName => "Diagnostic package contains an unsupported or unsafe entry name.",
            Self::DuplicateEntry => {
                "Diagnostic package contains a duplicate entry; capture it again."
            }
            Self::InputLimit => {
                "Diagnostic input exceeds the 256 KiB entry or 768 KiB total limit."
            }
            Self::InvalidJson => {
                "Diagnostic JSON is malformed, ambiguous, or exceeds structural limits."
            }
            Self::InvalidField => {
                "Diagnostic data does not match this build's supported field schema."
            }
            Self::OutputLimit => "Diagnostic package exceeds its bounded output size.",
            Self::InvalidArchive => "Diagnostic ZIP is not a complete canonical support package.",
            Self::IntegrityMismatch => {
                "Diagnostic package integrity check failed; capture it again."
            }
            Self::ArchiveWrite => "Diagnostic package could not be encoded in memory.",
        })
    }
}

impl std::error::Error for DiagnosticPackageError {}

type Result<T> = std::result::Result<T, DiagnosticPackageError>;

/// Accept the four existing JSON entries, in any input order. Reject unknown,
/// duplicate, missing, oversize, or structurally invalid inputs. Output is already
/// sanitized and validated; caller owns preview and publication of these bytes.
pub(crate) fn build_diagnostic_package(entries: &[(&str, Vec<u8>)]) -> Result<Vec<u8>> {
    if entries.len() != PAYLOAD_NAMES.len() {
        return Err(DiagnosticPackageError::EntryCount);
    }
    let mut seen = [false; 4];
    let mut total = 0_usize;
    // Complete the cheap name/size checks before parsing any caller JSON.
    for (name, contents) in entries {
        let index = payload_index(name.as_bytes())?;
        if seen[index] {
            return Err(DiagnosticPackageError::DuplicateEntry);
        }
        seen[index] = true;
        total = total
            .checked_add(contents.len())
            .ok_or(DiagnosticPackageError::InputLimit)?;
        if contents.len() > MAX_INPUT_ENTRY_BYTES || total > MAX_TOTAL_INPUT_BYTES {
            return Err(DiagnosticPackageError::InputLimit);
        }
    }
    let mut payloads: [Vec<u8>; 4] = std::array::from_fn(|_| Vec::new());
    for (name, contents) in entries {
        let index = payload_index(name.as_bytes())?;
        payloads[index] = sanitize_payload(index, contents)?;
    }
    let integrity = integrity_bytes(&payloads)?;
    let archive = encode_archive(&payloads, &integrity)?;
    validate_diagnostic_package(&archive)?;
    Ok(archive)
}

/// Check bounds, the exact entry set, the sanitized schema, per-entry sizes and
/// SHA256, and the entire canonical ZIP representation. This is not a signature
/// or authenticity check and deliberately rejects ZIPs changed by repackaging.
pub(crate) fn validate_diagnostic_package(bytes: &[u8]) -> Result<()> {
    let records = bounded_local_records(bytes)?;
    let mut payloads: [Vec<u8>; 4] = std::array::from_fn(|_| Vec::new());
    for index in 0..PAYLOAD_NAMES.len() {
        let sanitized = sanitize_payload(index, records[index])?;
        if sanitized != records[index] {
            // Includes unknown keys/values, noncanonical JSON, and freeform data
            // added by an author who also recalculated the integrity manifest.
            return Err(DiagnosticPackageError::InvalidField);
        }
        payloads[index] = sanitized;
    }
    let integrity = integrity_bytes(&payloads)?;
    if integrity != records[4] {
        return Err(DiagnosticPackageError::IntegrityMismatch);
    }
    if encode_archive(&payloads, &integrity)? != bytes {
        return Err(DiagnosticPackageError::InvalidArchive);
    }
    Ok(())
}

/// A data-free inventory for a caller-owned preview/confirmation UI. Validate
/// first so neither filenames nor sizes can be supplied by an untrusted ZIP.
pub(crate) fn diagnostic_package_preview(bytes: &[u8]) -> Result<String> {
    validate_diagnostic_package(bytes)?;
    let records = bounded_local_records(bytes)?;
    let mut preview = format!(
        "Syndocal diagnostic package: {} entries, {} bytes.\n\
         Includes counters, clock timing, fixed build identity and backend states.\n\
         Excludes project/media files, paths, credentials, labels and raw crash logs.\n",
        ZIP_NAMES.len(),
        bytes.len(),
    );
    for (name, contents) in ZIP_NAMES.iter().zip(records) {
        preview.push_str(&format!("{name}: {} bytes\n", contents.len()));
    }
    preview.push_str("SHA256 checks integrity, not authenticity.");
    Ok(preview)
}

fn payload_index(name: &[u8]) -> Result<usize> {
    PAYLOAD_NAMES
        .iter()
        .position(|candidate| candidate.as_bytes() == name)
        .ok_or(DiagnosticPackageError::EntryName)
}

#[derive(Clone, Copy)]
enum Kind {
    Unsigned,
    Signed,
    NonnegativeNumber,
    Bool,
    NullableUnsigned,
    FixedNumber(u64),
    FixedStrings(&'static [&'static str]),
    Object(&'static [Field]),
    Objects {
        fields: &'static [Field],
        max_items: usize,
        unique_key: Option<&'static str>,
    },
}

struct Field {
    name: &'static str,
    kind: Kind,
    required: bool,
}

const fn field(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        kind,
        required: false,
    }
}

const fn required(name: &'static str, kind: Kind) -> Field {
    Field {
        name,
        kind,
        required: true,
    }
}

// These tables are the redaction schema. Never replace them with a traversal
// that copies input keys, "looks numeric", or redacts strings by substring.
const MANIFEST_FIELDS: &[Field] = &[
    required("version", Kind::FixedNumber(1)),
    required("app", Kind::FixedStrings(&["Syndocal"])),
    required(
        "app_version",
        Kind::FixedStrings(&[env!("CARGO_PKG_VERSION")]),
    ),
    field("captured_at_unix_ms", Kind::Unsigned),
    required("os", Kind::FixedStrings(&["windows", "linux", "macos"])),
    required(
        "arch",
        Kind::FixedStrings(&["x86", "x86_64", "arm", "aarch64"]),
    ),
];

const PROJECT_FIELDS: &[Field] = &[
    field("fixtures", Kind::Unsigned),
    field("cues", Kind::Unsigned),
    field("effects", Kind::Unsigned),
    field("node_graphs", Kind::Unsigned),
    field("timeline_events", Kind::Unsigned),
    field("timeline_automations", Kind::Unsigned),
    field("timeline_video_automations", Kind::Unsigned),
    field("video_layers", Kind::Unsigned),
    field("video_outputs", Kind::Unsigned),
    field("dmx_outputs", Kind::Unsigned),
];

const CLOCK_FIELDS: &[Field] = &[
    field("bpm", Kind::NonnegativeNumber),
    field("beat_phase", Kind::NonnegativeNumber),
    field("beat_counter", Kind::Unsigned),
    field("tap_count", Kind::Unsigned),
    field(
        "source",
        Kind::FixedStrings(&[
            "Manual",
            "Tap",
            "MidiClock",
            "MidiTimecode",
            "Ltc",
            "AbletonLink",
            "DjLink",
        ]),
    ),
    field("external_sync_age_ms", Kind::NullableUnsigned),
    field("external_sync_locked", Kind::Bool),
];

const OUTPUT_FIELDS: &[Field] = &[
    field("enabled", Kind::Bool),
    field(
        "protocol",
        Kind::FixedStrings(&[
            "ArtNet",
            "Sacn",
            "EnttecUsbPro",
            "DmxKingUltraDmx",
            "EnttecOpenDmx",
        ]),
    ),
    field("universe", Kind::Unsigned),
    field("serial_baud_rate", Kind::Unsigned),
    // target_ip, port, serial_port, and any future endpoint/label are omitted.
];

const ROUTE_FIELDS: &[Field] = &[
    field("index", Kind::Unsigned),
    field("universe", Kind::Unsigned),
    field("attempted", Kind::Bool),
    field("success", Kind::Bool),
    field("bytes", Kind::Unsigned),
    field("consecutive_failures", Kind::Unsigned),
    field("reconnect_attempts", Kind::Unsigned),
    field("reconnecting", Kind::Bool),
    field("retry_in_ms", Kind::NullableUnsigned),
    field("last_success_unix_ms", Kind::NullableUnsigned),
];

const TELEMETRY_FIELDS: &[Field] = &[
    field("frame_counter", Kind::Unsigned),
    field("enabled_effect_count", Kind::Unsigned),
    field("supported_effect_count", Kind::Unsigned),
    field("effects_over_supported_envelope", Kind::Bool),
    field("queue_depth", Kind::Unsigned),
    field("queue_depth_abs_max", Kind::Unsigned),
    field("queue_push_failure_count", Kind::Unsigned),
    field("last_tick_interval_us", Kind::Unsigned),
    field("tick_jitter_last_us", Kind::Signed),
    field("tick_jitter_abs_max_us", Kind::Unsigned),
    field("tick_jitter_stddev_us", Kind::NonnegativeNumber),
    field("tick_jitter_p95_us", Kind::Unsigned),
    field("tick_jitter_p99_us", Kind::Unsigned),
    field("tick_jitter_samples", Kind::Unsigned),
    field("last_command_queue_latency_us", Kind::Unsigned),
    field("command_queue_latency_abs_max_us", Kind::Unsigned),
    field("command_queue_latency_p95_us", Kind::Unsigned),
    field("command_queue_latency_p99_us", Kind::Unsigned),
    field("command_queue_latency_samples", Kind::Unsigned),
    field("last_command_drain_count", Kind::Unsigned),
    field("command_drain_abs_max", Kind::Unsigned),
    field("command_drain_limit_hit_count", Kind::Unsigned),
    field("last_command_to_dmx_tick_latency_us", Kind::Unsigned),
    field("command_to_dmx_tick_latency_abs_max_us", Kind::Unsigned),
    field("command_to_dmx_tick_latency_p95_us", Kind::Unsigned),
    field("command_to_dmx_tick_latency_p99_us", Kind::Unsigned),
    field("command_to_dmx_tick_latency_samples", Kind::Unsigned),
    field("last_dmx_send_interval_us", Kind::Unsigned),
    field("dmx_send_interval_min_us", Kind::Unsigned),
    field("dmx_send_interval_max_us", Kind::Unsigned),
    field("dmx_send_interval_samples", Kind::Unsigned),
    field("low_latency_dmx_tick_request_count", Kind::Unsigned),
    field("low_latency_dmx_tick_advance_count", Kind::Unsigned),
    field("low_latency_dmx_tick_defer_count", Kind::Unsigned),
    field("last_packet_bytes", Kind::Unsigned),
    field("last_dmx_output_count", Kind::Unsigned),
    field("last_dmx_send_success_count", Kind::Unsigned),
    field("last_dmx_send_failure_count", Kind::Unsigned),
    field("total_dmx_send_success_count", Kind::Unsigned),
    field("total_dmx_send_failure_count", Kind::Unsigned),
    field(
        "last_dmx_route_results",
        Kind::Objects {
            fields: ROUTE_FIELDS,
            max_items: 128,
            unique_key: None,
        },
    ),
];

const BUDGET_STATES: &[&str] = &["Pass", "Warn", "Fail", "InsufficientSamples", "Idle"];
const BUDGET_CHECK_FIELDS: &[Field] = &[
    required(
        "name",
        Kind::FixedStrings(&[
            "tick_jitter_p99",
            "command_queue_p99",
            "command_to_dmx_tick_p99",
            "dmx_send_success",
            "dmx_send_interval",
        ]),
    ),
    required("status", Kind::FixedStrings(BUDGET_STATES)),
    field("measured_us", Kind::NullableUnsigned),
    field("target_us", Kind::NullableUnsigned),
    field("samples", Kind::Unsigned),
];
const BUDGET_FIELDS: &[Field] = &[
    field("overall", Kind::FixedStrings(BUDGET_STATES)),
    field("target_dmx_frame_rate_hz", Kind::Unsigned),
    field("target_tick_interval_us", Kind::Unsigned),
    field("tick_jitter_p99_target_us", Kind::Unsigned),
    field("command_queue_p99_target_us", Kind::Unsigned),
    field("command_to_dmx_p99_target_us", Kind::Unsigned),
    field("dmx_send_interval_tolerance_us", Kind::Unsigned),
    field(
        "checks",
        Kind::Objects {
            fields: BUDGET_CHECK_FIELDS,
            max_items: 5,
            unique_key: Some("name"),
        },
    ),
];

const ENGINE_FIELDS: &[Field] = &[
    required("version", Kind::FixedNumber(1)),
    field("captured_at_unix_ms", Kind::Unsigned),
    field("fixture_count", Kind::Unsigned),
    field("cue_count", Kind::Unsigned),
    field("effect_count", Kind::Unsigned),
    field("node_graph_count", Kind::Unsigned),
    field("video_layer_count", Kind::Unsigned),
    field("video_output_count", Kind::Unsigned),
    field("dmx_output_count", Kind::Unsigned),
    field("enabled_dmx_output_count", Kind::Unsigned),
    field("dmx_preview_universe_count", Kind::Unsigned),
    field("clock", Kind::Object(CLOCK_FIELDS)),
    field("primary_output", Kind::Object(OUTPUT_FIELDS)),
    field(
        "dmx_outputs",
        Kind::Objects {
            fields: OUTPUT_FIELDS,
            max_items: 128,
            unique_key: None,
        },
    ),
    field("budget", Kind::Object(BUDGET_FIELDS)),
    field("telemetry", Kind::Object(TELEMETRY_FIELDS)),
];

const VIDEO_BACKEND_FIELDS: &[Field] = &[
    required(
        "id",
        Kind::FixedStrings(&[
            "still_image",
            "libav",
            "ffmpeg",
            "camera",
            "screen_capture",
            "ffprobe",
            "hap_ffmpeg",
            "dxt_cpu_reference",
            "hap_gpu",
            "ndi",
            "spout",
            "syphon",
        ]),
    ),
    required(
        "state",
        Kind::FixedStrings(&["Available", "Missing", "NotBuilt"]),
    ),
];
const VIDEO_FIELDS: &[Field] = &[required(
    "backends",
    Kind::Objects {
        fields: VIDEO_BACKEND_FIELDS,
        max_items: 12,
        unique_key: Some("id"),
    },
)];

fn sanitize_payload(index: usize, contents: &[u8]) -> Result<Vec<u8>> {
    let schema = match index {
        0 => MANIFEST_FIELDS,
        1 => PROJECT_FIELDS,
        2 => ENGINE_FIELDS,
        3 => VIDEO_FIELDS,
        _ => return Err(DiagnosticPackageError::EntryName),
    };
    let value = parse_bounded_json(contents)?;
    serialize_bounded(&project_object(&value, schema)?)
}

fn project_object(value: &Value, schema: &[Field]) -> Result<Value> {
    let input = value
        .as_object()
        .ok_or(DiagnosticPackageError::InvalidField)?;
    let mut output = Map::new();
    for field in schema {
        let Some(value) = input.get(field.name) else {
            if field.required {
                return Err(DiagnosticPackageError::InvalidField);
            }
            continue;
        };
        let safe = match field.kind {
            Kind::Unsigned => value.as_u64().map(Value::from),
            Kind::Signed => value.as_i64().map(Value::from),
            Kind::NonnegativeNumber => match value {
                Value::Number(number)
                    if number
                        .as_f64()
                        .is_some_and(|value| value.is_finite() && value >= 0.0) =>
                {
                    Some(Value::Number(number.clone()))
                }
                _ => None,
            },
            Kind::Bool => value.as_bool().map(Value::Bool),
            Kind::NullableUnsigned if value.is_null() => Some(Value::Null),
            Kind::NullableUnsigned => value.as_u64().map(Value::from),
            Kind::FixedNumber(expected) => {
                (value.as_u64() == Some(expected)).then_some(Value::from(expected))
            }
            Kind::FixedStrings(allowed) => allowed
                .iter()
                .find(|candidate| Some(**candidate) == value.as_str())
                .map(|constant| Value::String((*constant).to_string())),
            Kind::Object(fields) => Some(project_object(value, fields)?),
            Kind::Objects {
                fields,
                max_items,
                unique_key,
            } => {
                let items = value
                    .as_array()
                    .ok_or(DiagnosticPackageError::InvalidField)?;
                if items.len() > max_items {
                    return Err(DiagnosticPackageError::InvalidField);
                }
                let mut projected = Vec::<Value>::with_capacity(items.len());
                for item in items {
                    let safe = project_object(item, fields)?;
                    if let Some(key) = unique_key {
                        if projected
                            .iter()
                            .any(|prior| prior.get(key) == safe.get(key))
                        {
                            return Err(DiagnosticPackageError::InvalidField);
                        }
                    }
                    projected.push(safe);
                }
                Some(Value::Array(projected))
            }
        }
        .ok_or(DiagnosticPackageError::InvalidField)?;
        output.insert(field.name.to_string(), safe);
    }
    Ok(Value::Object(output))
}

// A bounded visitor rejects depth and duplicate decoded keys before building a
// full tree. Even discarded fields pass through it, so an unknown subtree cannot
// evade limits. serde errors are never exposed, since they may echo input keys.
struct JsonSeed<'a> {
    depth: usize,
    nodes: &'a mut usize,
}

impl<'de> DeserializeSeed<'de> for JsonSeed<'_> {
    type Value = Value;

    fn deserialize<D: de::Deserializer<'de>>(
        self,
        deserializer: D,
    ) -> std::result::Result<Value, D::Error> {
        if self.depth > MAX_JSON_DEPTH || *self.nodes >= MAX_JSON_NODES {
            return Err(de::Error::custom("diagnostic JSON structural limit"));
        }
        *self.nodes += 1;
        deserializer.deserialize_any(self)
    }
}

impl<'de> Visitor<'de> for JsonSeed<'_> {
    type Value = Value;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("bounded diagnostic JSON")
    }

    fn visit_bool<E: de::Error>(self, value: bool) -> std::result::Result<Value, E> {
        Ok(Value::Bool(value))
    }

    fn visit_i64<E: de::Error>(self, value: i64) -> std::result::Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_u64<E: de::Error>(self, value: u64) -> std::result::Result<Value, E> {
        Ok(Value::from(value))
    }

    fn visit_f64<E: de::Error>(self, value: f64) -> std::result::Result<Value, E> {
        Number::from_f64(value)
            .map(Value::Number)
            .ok_or_else(|| de::Error::custom("diagnostic JSON invalid number"))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> std::result::Result<Value, E> {
        if value.len() > MAX_JSON_STRING_BYTES {
            return Err(de::Error::custom("diagnostic JSON string limit"));
        }
        Ok(Value::String(value.to_owned()))
    }

    fn visit_unit<E: de::Error>(self) -> std::result::Result<Value, E> {
        Ok(Value::Null)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> std::result::Result<Value, A::Error> {
        let mut values = Vec::new();
        while let Some(value) = sequence.next_element_seed(JsonSeed {
            depth: self.depth + 1,
            nodes: &mut *self.nodes,
        })? {
            if values.len() >= MAX_JSON_CONTAINER_ITEMS {
                return Err(de::Error::custom("diagnostic JSON array limit"));
            }
            values.push(value);
        }
        Ok(Value::Array(values))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> std::result::Result<Value, A::Error> {
        let mut values = Map::new();
        while let Some(key) = object.next_key::<String>()? {
            if values.len() >= MAX_JSON_CONTAINER_ITEMS
                || key.len() > MAX_JSON_STRING_BYTES
                || *self.nodes >= MAX_JSON_NODES
                || values.contains_key(&key)
            {
                return Err(de::Error::custom(
                    "diagnostic JSON object limit or duplicate key",
                ));
            }
            *self.nodes += 1;
            let value = object.next_value_seed(JsonSeed {
                depth: self.depth + 1,
                nodes: &mut *self.nodes,
            })?;
            values.insert(key, value);
        }
        Ok(Value::Object(values))
    }
}

fn parse_bounded_json(bytes: &[u8]) -> Result<Value> {
    if bytes.len() > MAX_INPUT_ENTRY_BYTES {
        return Err(DiagnosticPackageError::InputLimit);
    }
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    let mut nodes = 0;
    let value = JsonSeed {
        depth: 1,
        nodes: &mut nodes,
    }
    .deserialize(&mut deserializer)
    .map_err(|_| DiagnosticPackageError::InvalidJson)?;
    deserializer
        .end()
        .map_err(|_| DiagnosticPackageError::InvalidJson)?;
    Ok(value)
}

#[derive(Serialize)]
struct IntegrityManifest {
    format: &'static str,
    schema_version: u32,
    redaction_schema_version: u32,
    entries: Vec<IntegrityEntry>,
}

#[derive(Serialize)]
struct IntegrityEntry {
    name: &'static str,
    size_bytes: u64,
    sha256: String,
}

fn integrity_bytes(payloads: &[Vec<u8>; 4]) -> Result<Vec<u8>> {
    let entries = PAYLOAD_NAMES
        .iter()
        .zip(payloads)
        .map(|(&name, contents)| IntegrityEntry {
            name,
            size_bytes: contents.len() as u64,
            sha256: format!("{:x}", Sha256::digest(contents)),
        })
        .collect();
    serialize_bounded(&IntegrityManifest {
        format: "syndocal-diagnostic-package",
        schema_version: 1,
        redaction_schema_version: 1,
        entries,
    })
}

fn serialize_bounded(value: &impl Serialize) -> Result<Vec<u8>> {
    let mut writer = LimitedCursor::new(MAX_SANITIZED_ENTRY_BYTES);
    serde_json::to_writer(&mut writer, value).map_err(|_| DiagnosticPackageError::OutputLimit)?;
    Ok(writer.into_inner())
}

/// Serialize a trusted runtime value through the same bounded writer used by
/// the package sanitizer. The package builder still performs the authoritative
/// schema projection; this helper only prevents the caller-side capture from
/// allocating an unbounded entry before that boundary.
pub(crate) fn diagnostic_json_bytes(value: &impl Serialize) -> Result<Vec<u8>> {
    serialize_bounded(value)
}

fn encode_archive(payloads: &[Vec<u8>; 4], integrity: &[u8]) -> Result<Vec<u8>> {
    let contents: [&[u8]; 5] = [
        &payloads[0],
        &payloads[1],
        &payloads[2],
        &payloads[3],
        integrity,
    ];
    let mut total = 0_usize;
    for entry in &contents {
        total = total
            .checked_add(entry.len())
            .ok_or(DiagnosticPackageError::OutputLimit)?;
        if entry.len() > MAX_SANITIZED_ENTRY_BYTES || total > MAX_TOTAL_SANITIZED_BYTES {
            return Err(DiagnosticPackageError::OutputLimit);
        }
    }
    let mut archive = zip::ZipWriter::new(LimitedCursor::new(MAX_ARCHIVE_BYTES));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o600);
    for (name, contents) in ZIP_NAMES.iter().zip(contents) {
        archive
            .start_file(*name, options)
            .map_err(|_| DiagnosticPackageError::ArchiveWrite)?;
        archive
            .write_all(contents)
            .map_err(|_| DiagnosticPackageError::OutputLimit)?;
    }
    archive
        .finish()
        .map(|writer| writer.into_inner())
        .map_err(|_| DiagnosticPackageError::ArchiveWrite)
}

/// Never grows or seeks beyond its fixed cap, including ZIP finalization writes.
struct LimitedCursor {
    cursor: Cursor<Vec<u8>>,
    limit: usize,
}

impl LimitedCursor {
    fn new(limit: usize) -> Self {
        Self {
            cursor: Cursor::new(Vec::with_capacity(limit)),
            limit,
        }
    }

    fn into_inner(self) -> Vec<u8> {
        self.cursor.into_inner()
    }
}

impl Write for LimitedCursor {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self.cursor.position().checked_add(bytes.len() as u64);
        if end.is_none_or(|end| end > self.limit as u64) {
            return Err(io::Error::other("diagnostic output limit"));
        }
        self.cursor.write(bytes)
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Seek for LimitedCursor {
    fn seek(&mut self, position: SeekFrom) -> io::Result<u64> {
        let target = match position {
            SeekFrom::Start(value) => value as i128,
            SeekFrom::Current(delta) => self.cursor.position() as i128 + delta as i128,
            SeekFrom::End(delta) => self.cursor.get_ref().len() as i128 + delta as i128,
        };
        if target < 0 || target > self.limit as i128 {
            return Err(io::Error::other("diagnostic output limit"));
        }
        self.cursor.seek(SeekFrom::Start(target as u64))
    }
}

fn bounded_local_records(bytes: &[u8]) -> Result<[&[u8]; 5]> {
    if bytes.len() > MAX_ARCHIVE_BYTES {
        return Err(DiagnosticPackageError::OutputLimit);
    }
    let mut records: [&[u8]; 5] = [&[]; 5];
    let mut seen = [false; 5];
    let mut offset = 0_usize;
    let mut total = 0_usize;
    for _ in 0..ZIP_NAMES.len() {
        let header_end = offset
            .checked_add(30)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        let header = bytes
            .get(offset..header_end)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        // No encryption, compression, data descriptors, UTF-8 ambiguity or extras.
        if &header[..4] != b"PK\x03\x04"
            || header[6..10] != [0, 0, 0, 0]
            || header[28..30] != [0, 0]
        {
            return Err(DiagnosticPackageError::InvalidArchive);
        }
        let compressed =
            u32::from_le_bytes([header[18], header[19], header[20], header[21]]) as usize;
        let size = u32::from_le_bytes([header[22], header[23], header[24], header[25]]) as usize;
        if compressed != size {
            return Err(DiagnosticPackageError::InvalidArchive);
        }
        total = total
            .checked_add(size)
            .ok_or(DiagnosticPackageError::OutputLimit)?;
        if size > MAX_SANITIZED_ENTRY_BYTES || total > MAX_TOTAL_SANITIZED_BYTES {
            return Err(DiagnosticPackageError::OutputLimit);
        }
        let name_len = u16::from_le_bytes([header[26], header[27]]) as usize;
        let name_end = header_end
            .checked_add(name_len)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        let name = bytes
            .get(header_end..name_end)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        let index = ZIP_NAMES
            .iter()
            .position(|candidate| candidate.as_bytes() == name)
            .ok_or(DiagnosticPackageError::EntryName)?;
        if seen[index] {
            return Err(DiagnosticPackageError::DuplicateEntry);
        }
        seen[index] = true;
        let end = name_end
            .checked_add(size)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        records[index] = bytes
            .get(name_end..end)
            .ok_or(DiagnosticPackageError::InvalidArchive)?;
        offset = end;
    }
    // No generic ZIP parser sees attacker-controlled central-directory counts.
    // The final whole-ZIP equality check verifies every remaining byte, including
    // extra/missing/duplicate central entries, offsets, comments and trailing data.
    Ok(records)
}

#[cfg(test)]
#[path = "diagnostic_package_tests.rs"]
mod tests;
