#![cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]

use libloading::Library;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::error::Error;
use std::ffi::c_void;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;

const CANONICAL_DLL_FILE_NAME: &str = "syndocal_asio_bridge.dll";
const REQUIRED_ABI_VERSION: u32 = 2;
const REQUIRED_JSON_SCHEMA_VERSION: u32 = 2;
const BUILD_FLAG_ASIO_COMPILED: u32 = 0x1;
const EXPECTED_BACKEND_LABEL: &str = "asio";
const MAX_JSON_BYTES: usize = 65_536;
const MAX_CHANNEL_MIX_ENTRIES: usize = 256;
const MAX_CHANNEL_MIX_GAIN: f64 = 1.0;
const MAX_FIXED_BUFFER_FRAMES: u32 = 1_048_576;
const MIN_PERSISTED_SAMPLE_RATE_HZ: u32 = 8_000;
const MAX_PERSISTED_SAMPLE_RATE_HZ: u32 = 768_000;
const MAX_INPUT_CHANNELS: u32 = u16::MAX as u32;
const MAX_IDENTITY_TEXT_LEN: usize = 512;
/// Maximum UTF-8 byte length accepted for a driver identity string
/// (`asio:<driver name>`). Enforced uniformly on catalog entries, capability
/// requests, explicit start parameters, and persisted selections so no
/// unbounded identity ever reaches the DLL or the wire.
const MAX_DRIVER_ID_BYTES: usize = 512;
/// Maximum serialized byte length of the capabilities-request wire payload
/// handed across the ABI v2 boundary. The v2 envelope
/// `{"schemaVersion":2,"driverId":"<id>"}` contributes exactly 33 fixed bytes,
/// so any ID within `MAX_DRIVER_ID_BYTES` serializes to at most 545 bytes;
/// this gate keeps documented headroom while staying far below MAX_JSON_BYTES.
const MAX_CAPABILITIES_REQUEST_WIRE_BYTES: usize = 1_024;
/// Upper bound, in characters, for untrusted diagnostic text (rejected bridge
/// error payloads) embedded in a visible failure. Truncation is display-only:
/// it never changes whether a call succeeds or fails.
const MAX_DIAGNOSTIC_TEXT_LEN: usize = 512;

const EVENT_SEVERITY_WARNING: u32 = 1;
#[cfg(test)]
const EVENT_SEVERITY_TERMINAL: u32 = 2;
pub(crate) const EVENT_KIND_XRUN: u32 = 1;

pub(crate) const INTERNAL_FAULT_DROP_WITHOUT_CLOSE: u32 = u32::MAX - 3;
pub(crate) const INTERNAL_FAULT_MALFORMED_EVENT: u32 = u32::MAX - 2;
pub(crate) const INTERNAL_FAULT_FRAME_MISMATCH: u32 = u32::MAX - 1;
pub(crate) const INTERNAL_FAULT_NONFINITE_SAMPLE: u32 = u32::MAX;

const PERSISTED_SELECTION_SCHEMA_VERSION: u32 = 2;

/// Single canonical authority for the exact nine ABI v2 symbols the loader
/// must resolve from the canonical bridge DLL, listed in `BridgeVTable`
/// resolution order. Production `resolve_vtable` destructures this constant
/// directly (the nine-element pattern turns any arity drift into a compile
/// error), so the resolved names and this list cannot drift apart; tests pin
/// the constant itself against the documented external ABI instead of keeping
/// a separate hardcoded list.
const REQUIRED_SYMBOLS: [&str; 9] = [
    "syndocal_asio_v2_abi_version",
    "syndocal_asio_v2_build_flags",
    "syndocal_asio_v2_drivers_json",
    "syndocal_asio_v2_capabilities_json",
    "syndocal_asio_v2_string_free",
    "syndocal_asio_v2_start",
    "syndocal_asio_v2_stop",
    "syndocal_asio_v2_close",
    "syndocal_asio_v2_telemetry_json",
];

#[repr(C)]
#[derive(Clone, Copy)]
struct BridgeString {
    ptr: *mut u8,
    len: usize,
}

impl BridgeString {
    const EMPTY: BridgeString = BridgeString {
        ptr: std::ptr::null_mut(),
        len: 0,
    };

    fn is_empty(self) -> bool {
        self.ptr.is_null() || self.len == 0
    }
}

type AbiVersionFn = unsafe extern "C" fn() -> u32;
type BuildFlagsFn = unsafe extern "C" fn() -> u32;
type DriversJsonFn = unsafe extern "C" fn(*mut BridgeString, *mut BridgeString) -> u32;
type CapabilitiesJsonFn =
    unsafe extern "C" fn(*const u8, usize, *mut BridgeString, *mut BridgeString) -> u32;
type StringFreeFn = unsafe extern "C" fn(BridgeString);
pub(crate) type SampleCallbackFn = unsafe extern "C" fn(*mut c_void, *const f32, usize, u64, u32);
pub(crate) type EventCallbackFn = unsafe extern "C" fn(*mut c_void, u32, u32, *const u8, usize);
type StartFn = unsafe extern "C" fn(
    *const u8,
    usize,
    Option<SampleCallbackFn>,
    Option<EventCallbackFn>,
    *mut c_void,
    *mut *mut c_void,
    *mut BridgeString,
    *mut BridgeString,
) -> u32;
type StopFn = unsafe extern "C" fn(*mut c_void, *mut BridgeString, *mut BridgeString) -> u32;
type CloseFn = unsafe extern "C" fn(*mut *mut c_void, *mut BridgeString, *mut BridgeString) -> u32;
type TelemetryJsonFn =
    unsafe extern "C" fn(*const c_void, *mut BridgeString, *mut BridgeString) -> u32;

#[derive(Clone, Copy)]
struct BridgeVTable {
    abi_version: AbiVersionFn,
    build_flags: BuildFlagsFn,
    drivers_json: DriversJsonFn,
    capabilities_json: CapabilitiesJsonFn,
    string_free: StringFreeFn,
    start: StartFn,
    stop: StopFn,
    close: CloseFn,
    telemetry_json: TelemetryJsonFn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeLoadFault {
    LoadFailed {
        path: String,
        detail: String,
    },
    MissingSymbol {
        symbol: &'static str,
        detail: String,
    },
    AbiMismatch {
        expected: u32,
        actual: u32,
    },
    NotAsioCompiled,
}

impl fmt::Display for BridgeLoadFault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LoadFailed { path, detail } => {
                write!(f, "failed to load ASIO bridge {path}: {detail}")
            }
            Self::MissingSymbol { symbol, detail } => {
                write!(
                    f,
                    "ASIO bridge is missing required v2 symbol {symbol}: {detail}"
                )
            }
            Self::AbiMismatch { expected, actual } => write!(
                f,
                "ASIO bridge ABI mismatch: loader requires ABI {expected}, DLL reports {actual}; ABI v1 and future ABIs are rejected"
            ),
            Self::NotAsioCompiled => write!(
                f,
                "ASIO bridge build flags report the DLL was compiled without the ASIO backend"
            ),
        }
    }
}

impl Error for BridgeLoadFault {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BridgeStatus {
    Ok,
    InvalidArgument,
    Unsupported,
    DriverNotFound,
    ConfigUnsupported,
    BackendError,
    Terminal,
    Panic,
}

fn classify_status(raw: u32) -> Option<BridgeStatus> {
    match raw {
        0 => Some(BridgeStatus::Ok),
        1 => Some(BridgeStatus::InvalidArgument),
        2 => Some(BridgeStatus::Unsupported),
        3 => Some(BridgeStatus::DriverNotFound),
        4 => Some(BridgeStatus::ConfigUnsupported),
        5 => Some(BridgeStatus::BackendError),
        6 => Some(BridgeStatus::Terminal),
        255 => Some(BridgeStatus::Panic),
        _ => None,
    }
}

fn status_label(status: BridgeStatus) -> &'static str {
    match status {
        BridgeStatus::Ok => "ok",
        BridgeStatus::InvalidArgument => "invalid_argument",
        BridgeStatus::Unsupported => "unsupported",
        BridgeStatus::DriverNotFound => "driver_not_found",
        BridgeStatus::ConfigUnsupported => "config_unsupported",
        BridgeStatus::BackendError => "backend_error",
        BridgeStatus::Terminal => "terminal",
        BridgeStatus::Panic => "panic",
    }
}

fn verify_abi_contract(abi_version: u32, build_flags: u32) -> Result<(), BridgeLoadFault> {
    if abi_version != REQUIRED_ABI_VERSION {
        return Err(BridgeLoadFault::AbiMismatch {
            expected: REQUIRED_ABI_VERSION,
            actual: abi_version,
        });
    }
    if build_flags & BUILD_FLAG_ASIO_COMPILED == 0 {
        return Err(BridgeLoadFault::NotAsioCompiled);
    }
    Ok(())
}

unsafe fn resolve_symbol<T: Copy>(
    library: &Library,
    symbol: &'static str,
) -> Result<T, BridgeLoadFault> {
    let mut lookup_bytes = symbol.as_bytes().to_vec();
    lookup_bytes.push(0);
    let resolved = unsafe { library.get::<T>(&lookup_bytes) }.map_err(|error| {
        BridgeLoadFault::MissingSymbol {
            symbol,
            detail: error.to_string(),
        }
    })?;
    Ok(*resolved)
}

struct LoadedModule {
    _library: Library,
    vtable: BridgeVTable,
}

pub(crate) struct AsioBridgeModule {
    inner: Arc<LoadedModule>,
}

impl fmt::Debug for AsioBridgeModule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("AsioBridgeModule(canonical syndocal_asio_bridge.dll)")
    }
}

impl AsioBridgeModule {
    pub(crate) fn load_at(path: &Path) -> Result<Self, BridgeLoadFault> {
        let library =
            unsafe { Library::new(path) }.map_err(|error| BridgeLoadFault::LoadFailed {
                path: path.display().to_string(),
                detail: error.to_string(),
            })?;
        let vtable = Self::resolve_vtable(&library)?;
        let abi_version = unsafe { (vtable.abi_version)() };
        let build_flags = unsafe { (vtable.build_flags)() };
        verify_abi_contract(abi_version, build_flags)?;
        Ok(Self {
            inner: Arc::new(LoadedModule {
                _library: library,
                vtable,
            }),
        })
    }

    fn resolve_vtable(library: &Library) -> Result<BridgeVTable, BridgeLoadFault> {
        // Destructure the single canonical symbol authority: the nine-element
        // pattern makes arity drift a compile error and binds each vtable
        // field to its exact name from REQUIRED_SYMBOLS, so production
        // resolution and the tested list cannot diverge.
        let [abi_version, build_flags, drivers_json, capabilities_json, string_free, start, stop, close, telemetry_json] =
            REQUIRED_SYMBOLS;
        Ok(BridgeVTable {
            abi_version: unsafe { resolve_symbol::<AbiVersionFn>(library, abi_version)? },
            build_flags: unsafe { resolve_symbol::<BuildFlagsFn>(library, build_flags)? },
            drivers_json: unsafe { resolve_symbol::<DriversJsonFn>(library, drivers_json)? },
            capabilities_json: unsafe {
                resolve_symbol::<CapabilitiesJsonFn>(library, capabilities_json)?
            },
            string_free: unsafe { resolve_symbol::<StringFreeFn>(library, string_free)? },
            start: unsafe { resolve_symbol::<StartFn>(library, start)? },
            stop: unsafe { resolve_symbol::<StopFn>(library, stop)? },
            close: unsafe { resolve_symbol::<CloseFn>(library, close)? },
            telemetry_json: unsafe { resolve_symbol::<TelemetryJsonFn>(library, telemetry_json)? },
        })
    }

    fn settle(
        &self,
        operation: &'static str,
        status_raw: u32,
        result: BridgeString,
        error: BridgeString,
    ) -> Result<String, BridgeCallFailure> {
        let sink = VtableStringFreeSink {
            vtable: &self.inner.vtable,
        };
        settle_bridge_strings(&sink, operation, status_raw, result, error)
    }

    pub(crate) fn into_transport(self) -> Arc<VtableTransport> {
        Arc::new(VtableTransport { module: self })
    }

    pub(crate) fn driver_catalog_text(&self) -> Result<String, BridgeCallFailure> {
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe { (self.inner.vtable.drivers_json)(&mut result, &mut error) };
        self.settle("driver enumeration", status, result, error)
    }

    pub(crate) fn capability_text(&self, driver_id: &str) -> Result<String, BridgeCallFailure> {
        let request =
            CapabilitiesRequestJson::new(driver_id).map_err(|detail| BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(BridgeStatus::InvalidArgument),
                error_payload: None,
                detail,
            })?;
        let request_bytes = request.serialize_wire_bytes()?;
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe {
            (self.inner.vtable.capabilities_json)(
                request_bytes.as_ptr(),
                request_bytes.len(),
                &mut result,
                &mut error,
            )
        };
        self.settle("capability query", status, result, error)
    }
}

pub(crate) fn parse_driver_catalog(text: &str) -> Result<DriverCatalogJson, PayloadReject> {
    parse_strict_payload::<DriverCatalogJson>(text)
}

pub(crate) fn parse_capabilities(text: &str) -> Result<DriverCapabilitiesJson, PayloadReject> {
    parse_strict_payload::<DriverCapabilitiesJson>(text)
}

trait StringFreeSink {
    fn release(&self, value: BridgeString);
}

struct VtableStringFreeSink<'a> {
    vtable: &'a BridgeVTable,
}

impl StringFreeSink for VtableStringFreeSink<'_> {
    fn release(&self, value: BridgeString) {
        if value.is_empty() {
            return;
        }
        unsafe { (self.vtable.string_free)(value) };
    }
}

fn decode_bridge_string(value: BridgeString) -> Result<String, String> {
    if value.is_empty() {
        return Ok(String::new());
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr.cast_const(), value.len) };
    String::from_utf8(bytes.to_vec())
        .map_err(|error| format!("output was not valid UTF-8: {error}"))
}

fn settle_bridge_strings<S: StringFreeSink>(
    sink: &S,
    operation: &'static str,
    status_raw: u32,
    result: BridgeString,
    error: BridgeString,
) -> Result<String, BridgeCallFailure> {
    let decoded_result = decode_bridge_string(result);
    let decoded_error = decode_bridge_string(error);
    sink.release(result);
    sink.release(error);
    let result_text = decoded_result.map_err(|detail| BridgeCallFailure {
        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
        error_payload: None,
        detail: format!("{operation}: loader rejected the bridge output: {detail}"),
    })?;
    let error_text = decoded_error.map_err(|detail| BridgeCallFailure {
        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
        error_payload: None,
        detail: format!("{operation}: loader rejected the bridge error output: {detail}"),
    })?;
    let parsed_error_payload = parse_optional_error_payload(&error_text);
    let rejected_payload_context = match &parsed_error_payload {
        Err(reject) => format!(
            "; the bridge's attached error payload was rejected fail-closed and withheld: {}",
            bounded_diagnostic(&reject.to_string())
        ),
        Ok(_) => String::new(),
    };
    let error_payload = parsed_error_payload.ok().flatten();
    match classify_status(status_raw) {
        None => Err(BridgeCallFailure {
            status: BridgeStatusOrUnknown::Unknown(status_raw),
            error_payload,
            detail: format!(
                "{operation}: bridge reported an unrecognized status {status_raw}{rejected_payload_context}"
            ),
        }),
        Some(BridgeStatus::Ok) => {
            if result_text.trim().is_empty() {
                return Err(BridgeCallFailure {
                    status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                    error_payload,
                    detail: format!(
                        "{operation}: bridge returned an empty success response{rejected_payload_context}"
                    ),
                });
            }
            Ok(result_text)
        }
        Some(status) => {
            // The raw bridge text is echoed for diagnosis but stays bounded so
            // a hostile payload cannot balloon the visible failure.
            let detail = if error_text.trim().is_empty() {
                format!("{operation}: bridge reported {}", status_label(status))
            } else {
                format!("{operation}: {}", bounded_diagnostic(&error_text))
            };
            Err(BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(status),
                error_payload,
                detail: format!("{detail}{rejected_payload_context}"),
            })
        }
    }
}

fn parse_optional_error_payload(text: &str) -> Result<Option<BridgeErrorPayload>, PayloadReject> {
    if text.trim().is_empty() {
        return Ok(None);
    }
    // The payload is untrusted bridge output: a rejection is preserved as
    // bounded diagnostic context for the caller's visible failure and the
    // payload itself stays withheld. This never upgrades a failing call.
    parse_strict_payload::<BridgeErrorPayload>(text).map(Some)
}

/// Caps untrusted diagnostic text so hostile payloads cannot balloon a visible
/// failure message; truncation happens on a char boundary and is marked.
fn bounded_diagnostic(text: &str) -> String {
    if text.chars().nth(MAX_DIAGNOSTIC_TEXT_LEN).is_some() {
        let mut bounded: String = text.chars().take(MAX_DIAGNOSTIC_TEXT_LEN).collect();
        bounded.push_str("…");
        return bounded;
    }
    text.to_owned()
}

pub(crate) enum AsioBridgeAvailability {
    NotPackaged { directory: PathBuf },
    Ready(AsioBridgeModule),
    Fault(BridgeLoadFault),
}

impl fmt::Debug for AsioBridgeAvailability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotPackaged { directory } => write!(f, "NotPackaged({})", directory.display()),
            Self::Ready(_) => f.write_str("Ready(AsioBridgeModule)"),
            Self::Fault(fault) => write!(f, "Fault({fault})"),
        }
    }
}

impl AsioBridgeAvailability {
    #[cfg(test)]
    pub(crate) fn label(&self) -> &'static str {
        match self {
            Self::NotPackaged { .. } => "NOT_PACKAGED",
            Self::Ready(_) => "READY",
            Self::Fault(_) => "FAULT",
        }
    }

    pub(crate) fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    pub(crate) fn into_module(self) -> Result<AsioBridgeModule, String> {
        match self {
            Self::Ready(module) => Ok(module),
            availability => Err(availability.to_rejection_message()),
        }
    }

    fn to_rejection_message(&self) -> String {
        match self {
            Self::NotPackaged { directory } => format!(
                "the canonical {CANONICAL_DLL_FILE_NAME} is not packaged beside Syndocal ({}); no other ASIO DLL name, filename alias, or path override is consulted",
                directory.display()
            ),
            Self::Fault(fault) => fault.to_string(),
            Self::Ready(_) => unreachable!("into_module handles Ready above"),
        }
    }
}

fn canonical_bridge_dll_path(executable_directory: &Path) -> PathBuf {
    executable_directory.join(CANONICAL_DLL_FILE_NAME)
}

fn probe_asio_bridge(executable_directory: &Path) -> AsioBridgeAvailability {
    let dll_path = canonical_bridge_dll_path(executable_directory);
    if !dll_path.is_file() {
        return AsioBridgeAvailability::NotPackaged {
            directory: executable_directory.to_path_buf(),
        };
    }
    match AsioBridgeModule::load_at(&dll_path) {
        Ok(module) => AsioBridgeAvailability::Ready(module),
        Err(fault) => AsioBridgeAvailability::Fault(fault),
    }
}

/// Probes for the canonical bridge DLL beside this exact application
/// executable. No environment override, alternate filename, or search path is
/// ever consulted.
pub(crate) fn probe_canonical_bridge() -> AsioBridgeAvailability {
    let executable = match std::env::current_exe() {
        Ok(executable) => executable,
        Err(error) => {
            return AsioBridgeAvailability::Fault(BridgeLoadFault::LoadFailed {
                path: "<current executable>".to_owned(),
                detail: format!("failed to locate the Syndocal executable: {error}"),
            })
        }
    };
    let Some(directory) = executable.parent() else {
        return AsioBridgeAvailability::Fault(BridgeLoadFault::LoadFailed {
            path: executable.display().to_string(),
            detail: "the Syndocal executable has no parent directory".to_owned(),
        });
    };
    probe_asio_bridge(directory)
}

/// Probes an explicit directory for the canonical bridge DLL. Used by
/// contract tests proving that no environment override or alternate filename
/// is ever consulted.
#[cfg(test)]
pub(crate) fn probe_canonical_bridge_at(directory: &Path) -> AsioBridgeAvailability {
    probe_asio_bridge(directory)
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum SchemaReject {
    Future { found: u32 },
    Unsupported { found: u32 },
}

impl fmt::Display for SchemaReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Future { found } => write!(
                f,
                "payload schemaVersion {found} was written by a newer Syndocal or bridge and is rejected"
            ),
            Self::Unsupported { found } => write!(
                f,
                "payload schemaVersion {found} is not supported; only schemaVersion {REQUIRED_JSON_SCHEMA_VERSION} is accepted"
            ),
        }
    }
}

impl Error for SchemaReject {}

fn reject_schema_version(found: u32) -> Result<(), SchemaReject> {
    if found == REQUIRED_JSON_SCHEMA_VERSION {
        return Ok(());
    }
    if found > REQUIRED_JSON_SCHEMA_VERSION {
        return Err(SchemaReject::Future { found });
    }
    Err(SchemaReject::Unsupported { found })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PayloadReject {
    DuplicateKey,
    Malformed(String),
    SchemaFuture {
        found: u32,
    },
    SchemaUnsupported {
        found: u32,
    },
    KindMismatch {
        expected: &'static str,
        found: String,
    },
    ContractViolation(String),
}

impl fmt::Display for PayloadReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey => f.write_str("payload contains duplicate JSON object keys"),
            Self::Malformed(detail) => write!(f, "payload is not strict ABI v2 JSON: {detail}"),
            Self::SchemaFuture { found } => {
                write!(f, "payload schemaVersion {found} is a future version and is rejected")
            }
            Self::SchemaUnsupported { found } => write!(
                f,
                "payload schemaVersion {found} is not supported; only {REQUIRED_JSON_SCHEMA_VERSION} is accepted"
            ),
            Self::KindMismatch { expected, found } => write!(
                f,
                "payload kind \"{found}\" does not match the expected kind \"{expected}\""
            ),
            Self::ContractViolation(detail) => {
                write!(f, "payload violates the ABI v2 contract: {detail}")
            }
        }
    }
}

impl Error for PayloadReject {}

impl From<SchemaReject> for PayloadReject {
    fn from(reject: SchemaReject) -> Self {
        match reject {
            SchemaReject::Future { found } => Self::SchemaFuture { found },
            SchemaReject::Unsupported { found } => Self::SchemaUnsupported { found },
        }
    }
}

trait StrictPayload: DeserializeOwned {
    const KIND: &'static str;

    fn schema_version_field(&self) -> u32;

    fn kind_field(&self) -> &str;

    fn validate(&self) -> Result<(), PayloadReject> {
        Ok(())
    }
}

fn parse_strict_payload<T: StrictPayload>(text: &str) -> Result<T, PayloadReject> {
    match json_has_duplicate_keys(text) {
        Ok(true) => return Err(PayloadReject::DuplicateKey),
        Ok(false) => {}
        Err(detail) => {
            return Err(PayloadReject::Malformed(format!(
                "the strict duplicate-key pre-scan rejected this payload because of {detail}"
            )));
        }
    }
    let parsed: T =
        serde_json::from_str(text).map_err(|error| PayloadReject::Malformed(error.to_string()))?;
    reject_schema_version(parsed.schema_version_field())?;
    if parsed.kind_field() != T::KIND {
        return Err(PayloadReject::KindMismatch {
            expected: T::KIND,
            found: parsed.kind_field().to_owned(),
        });
    }
    parsed.validate()?;
    Ok(parsed)
}

fn hex_digit_value(byte: u8) -> Option<u16> {
    match byte {
        b'0'..=b'9' => Some(u16::from(byte - b'0')),
        b'a'..=b'f' => Some(u16::from(byte - b'a' + 10)),
        b'A'..=b'F' => Some(u16::from(byte - b'A' + 10)),
        _ => None,
    }
}

/// Decodes one `\uXXXX` unit whose four hexadecimal digits start at
/// `digits_start`; `content_end` is the exclusive end of the key contents.
fn decode_unicode_escape_unit(
    raw_key: &[u8],
    digits_start: usize,
    content_end: usize,
) -> Result<u16, &'static str> {
    if digits_start + 4 > content_end {
        return Err("a truncated \\u escape at the end of an object key");
    }
    let mut value = 0_u16;
    for offset in 0..4_usize {
        let digit = hex_digit_value(raw_key[digits_start + offset])
            .ok_or("a \\u escape containing a non-hexadecimal digit")?;
        value = (value << 4) | digit;
    }
    Ok(value)
}

/// Decodes a raw JSON object key, including its surrounding quotes, into the
/// exact UTF-8 byte sequence serde_json associates with that spelling, so two
/// object keys compare equal exactly when serde would treat them as the same
/// key. Valid `\uD800-\uDBFF` + `\uDC00-\uDFFF` pairs combine into their
/// scalar and therefore match the literal UTF-8 encoding of that scalar; lone,
/// high-without-low, low-without-high, reversed, truncated, or otherwise
/// malformed surrogate sequences return `Err`, which callers must surface as a
/// Malformed rejection instead of silently reporting "no duplicate".
fn unescape_json_key(raw_key: &[u8]) -> Result<Vec<u8>, &'static str> {
    if raw_key.len() < 2 || raw_key[0] != b'"' || raw_key[raw_key.len() - 1] != b'"' {
        return Err("an object key without exact surrounding quotes");
    }
    let content_end = raw_key.len() - 1;
    let mut out = Vec::with_capacity(raw_key.len());
    let mut cursor = 1;
    while cursor < content_end {
        let byte = raw_key[cursor];
        if byte == b'"' {
            return Err("an unescaped quote inside an object key");
        }
        if byte != b'\\' {
            out.push(byte);
            cursor += 1;
            continue;
        }
        cursor += 1;
        if cursor >= content_end {
            return Err("a backslash terminating an object key");
        }
        match raw_key[cursor] {
            b'"' => out.push(b'"'),
            b'\\' => out.push(b'\\'),
            b'/' => out.push(b'/'),
            b'b' => out.push(0x08),
            b'f' => out.push(0x0C),
            b'n' => out.push(b'\n'),
            b'r' => out.push(b'\r'),
            b't' => out.push(b'\t'),
            b'u' => {
                let first = decode_unicode_escape_unit(raw_key, cursor + 1, content_end)?;
                cursor += 4;
                let scalar = if (0xD800..=0xDBFF).contains(&first) {
                    if cursor + 3 >= content_end
                        || raw_key[cursor + 1] != b'\\'
                        || raw_key[cursor + 2] != b'u'
                    {
                        return Err("a lone high surrogate not followed by another \\u escape");
                    }
                    let second = decode_unicode_escape_unit(raw_key, cursor + 3, content_end)?;
                    if !matches!(second, 0xDC00..=0xDFFF) {
                        return Err(
                            "a high surrogate followed by a \\u escape that is not a low surrogate",
                        );
                    }
                    cursor += 6;
                    0x1_0000 + ((u32::from(first) - 0xD800) << 10) + (u32::from(second) - 0xDC00)
                } else if matches!(first, 0xDC00..=0xDFFF) {
                    return Err("a low surrogate without a preceding high surrogate");
                } else {
                    u32::from(first)
                };
                let ch = char::from_u32(scalar)
                    .ok_or("a \\u escape outside the Unicode scalar range")?;
                let mut encoded = [0_u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut encoded).as_bytes());
            }
            _ => return Err("an unsupported escape sequence in an object key"),
        }
        cursor += 1;
    }
    Ok(out)
}

type DuplicateScanStep = Result<(usize, bool), &'static str>;

/// Strict pre-scan executed before every serde parse: reports whether any JSON
/// object in the document declares the same decoded key twice, and fails
/// closed with a specific reason when an object key contains an escape
/// sequence that cannot be decoded exactly (including malformed surrogate
/// sequences). `Ok(false)` therefore means "no duplicate keys" and never
/// "keys could not be understood", so equivalent-key collisions can never
/// reach serde's silent last-wins merge.
fn json_has_duplicate_keys(text: &str) -> Result<bool, &'static str> {
    let (_, duplicate) = scan_value(text.as_bytes(), 0)?;
    Ok(duplicate)
}

fn skip_whitespace(bytes: &[u8], mut index: usize) -> usize {
    while index < bytes.len() && matches!(bytes[index], b' ' | b'\t' | b'\n' | b'\r') {
        index += 1;
    }
    index
}

fn scan_string(bytes: &[u8], opening_quote: usize) -> usize {
    let mut cursor = opening_quote + 1;
    while cursor < bytes.len() {
        if bytes[cursor] == b'\\' {
            cursor += 2;
        } else if bytes[cursor] == b'"' {
            return cursor + 1;
        } else {
            cursor += 1;
        }
    }
    cursor
}

fn scan_value(bytes: &[u8], start: usize) -> DuplicateScanStep {
    let index = skip_whitespace(bytes, start);
    if index >= bytes.len() {
        return Ok((bytes.len(), false));
    }
    match bytes[index] {
        b'{' => {
            let mut cursor = skip_whitespace(bytes, index + 1);
            let mut keys: HashSet<Vec<u8>> = HashSet::new();
            if cursor < bytes.len() && bytes[cursor] == b'}' {
                return Ok((cursor + 1, false));
            }
            while cursor < bytes.len() {
                if bytes[cursor] != b'"' {
                    return Ok((bytes.len(), false));
                }
                let key_end = scan_string(bytes, cursor);
                let key = unescape_json_key(&bytes[cursor..key_end])?;
                if keys.contains(&key) {
                    return Ok((key_end, true));
                }
                keys.insert(key);
                cursor = skip_whitespace(bytes, key_end);
                if cursor >= bytes.len() || bytes[cursor] != b':' {
                    return Ok((bytes.len(), false));
                }
                let (value_end, duplicate) = scan_value(bytes, cursor + 1)?;
                if duplicate {
                    return Ok((value_end, true));
                }
                cursor = skip_whitespace(bytes, value_end);
                if cursor < bytes.len() && bytes[cursor] == b'}' {
                    return Ok((cursor + 1, false));
                }
                if cursor >= bytes.len() || bytes[cursor] != b',' {
                    return Ok((bytes.len(), false));
                }
                cursor = skip_whitespace(bytes, cursor + 1);
            }
            Ok((cursor, false))
        }
        b'[' => {
            let mut cursor = skip_whitespace(bytes, index + 1);
            if cursor < bytes.len() && bytes[cursor] == b']' {
                return Ok((cursor + 1, false));
            }
            while cursor < bytes.len() {
                let (value_end, duplicate) = scan_value(bytes, cursor)?;
                if duplicate {
                    return Ok((value_end, true));
                }
                cursor = skip_whitespace(bytes, value_end);
                if cursor < bytes.len() && bytes[cursor] == b']' {
                    return Ok((cursor + 1, false));
                }
                if cursor >= bytes.len() || bytes[cursor] != b',' {
                    return Ok((bytes.len(), false));
                }
                cursor += 1;
            }
            Ok((cursor, false))
        }
        b'"' => Ok((scan_string(bytes, index), false)),
        _ => {
            let mut cursor = index;
            while cursor < bytes.len()
                && !matches!(
                    bytes[cursor],
                    b' ' | b'\t' | b'\n' | b'\r' | b',' | b'}' | b']'
                )
            {
                cursor += 1;
            }
            Ok((cursor, false))
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BufferFramesRangeJson {
    pub(crate) min: u32,
    pub(crate) max: u32,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DriverEntryJson {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) input_channels: u32,
    pub(crate) sample_formats: Vec<String>,
    pub(crate) sample_rates_hz: Vec<u32>,
    pub(crate) buffer_frames: BufferFramesRangeJson,
}

fn is_supported_native_sample_format(label: &str) -> bool {
    matches!(label, "f32" | "i16" | "i24" | "i32" | "f64")
}

fn valid_persistent_driver_id(id: &str) -> bool {
    if !(id.starts_with("asio:") && id.len() > "asio:".len() && id.trim() == id) {
        return false;
    }
    if id.len() > MAX_DRIVER_ID_BYTES {
        return false;
    }
    // Reject control-hostile identities: ASCII C0 controls and DEL anywhere in
    // the id can corrupt wire payloads, logs, and terminal displays. In valid
    // UTF-8 every byte >= 0x80 is a lead/continuation byte of a multi-byte
    // scalar, never a standalone C1 control, so an ASCII-range scan is exact.
    id.bytes().all(|byte| !(byte < 0x20 || byte == 0x7F))
}

fn validate_buffer_range(range: &BufferFramesRangeJson) -> Result<(), PayloadReject> {
    if range.min == 0 || range.min > range.max || range.max > MAX_FIXED_BUFFER_FRAMES {
        return Err(PayloadReject::ContractViolation(format!(
            "buffer frame range {}..{} must satisfy 1 <= min <= max <= {MAX_FIXED_BUFFER_FRAMES}",
            range.min, range.max
        )));
    }
    Ok(())
}

fn validate_driver_entry(entry: &DriverEntryJson) -> Result<(), PayloadReject> {
    if !valid_persistent_driver_id(&entry.id) {
        return Err(PayloadReject::ContractViolation(format!(
            "catalog driver id {:?} is not an explicit persistent asio:<driver name> identity without surrounding whitespace, ASCII control characters, or within {MAX_DRIVER_ID_BYTES} UTF-8 bytes",
            entry.id
        )));
    }
    if entry.name.trim().is_empty() || entry.name.len() > MAX_IDENTITY_TEXT_LEN {
        return Err(PayloadReject::ContractViolation(
            "catalog driver name must be non-empty and bounded".to_owned(),
        ));
    }
    if entry.input_channels == 0 || entry.input_channels > MAX_INPUT_CHANNELS {
        return Err(PayloadReject::ContractViolation(format!(
            "catalog driver input_channels {} must be between 1 and {MAX_INPUT_CHANNELS}",
            entry.input_channels
        )));
    }
    if entry.sample_formats.is_empty()
        || !entry
            .sample_formats
            .iter()
            .all(|format| is_supported_native_sample_format(format))
    {
        return Err(PayloadReject::ContractViolation(
            "catalog driver native sample formats must be exactly f32, i16, i24, i32, or f64"
                .to_owned(),
        ));
    }
    if entry.sample_rates_hz.is_empty() || entry.sample_rates_hz.iter().any(|rate| *rate == 0) {
        return Err(PayloadReject::ContractViolation(
            "catalog driver must advertise at least one non-zero sample rate".to_owned(),
        ));
    }
    validate_buffer_range(&entry.buffer_frames)?;
    Ok(())
}

fn validate_envelope_identity(
    abi_version: u32,
    backend: &str,
    built: bool,
) -> Result<(), PayloadReject> {
    if abi_version != REQUIRED_ABI_VERSION {
        return Err(PayloadReject::ContractViolation(format!(
            "payload reports abi_version {abi_version}, expected {REQUIRED_ABI_VERSION}"
        )));
    }
    if backend != EXPECTED_BACKEND_LABEL {
        return Err(PayloadReject::ContractViolation(format!(
            "payload reports backend {backend:?}, expected {EXPECTED_BACKEND_LABEL:?}"
        )));
    }
    if !built {
        return Err(PayloadReject::ContractViolation(
            "payload reports built=false: this DLL was compiled without the ASIO backend"
                .to_owned(),
        ));
    }
    Ok(())
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DriverCatalogJson {
    schema_version: u32,
    kind: String,
    abi_version: u32,
    backend: String,
    built: bool,
    pub(crate) drivers: Vec<DriverEntryJson>,
}

impl StrictPayload for DriverCatalogJson {
    const KIND: &'static str = "drivers";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }

    fn validate(&self) -> Result<(), PayloadReject> {
        validate_envelope_identity(self.abi_version, &self.backend, self.built)?;
        for driver in &self.drivers {
            validate_driver_entry(driver)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilitiesRequestJson {
    schema_version: u32,
    driver_id: String,
}

impl CapabilitiesRequestJson {
    fn new(driver_id: &str) -> Result<Self, String> {
        if !valid_persistent_driver_id(driver_id) {
            return Err(format!(
                "capabilities request requires an exact persistent asio:<driver name> id without surrounding whitespace, ASCII control characters, or within {MAX_DRIVER_ID_BYTES} UTF-8 bytes, got {driver_id:?}"
            ));
        }
        Ok(Self {
            schema_version: REQUIRED_JSON_SCHEMA_VERSION,
            driver_id: driver_id.to_owned(),
        })
    }

    fn serialize_wire_bytes(&self) -> Result<Vec<u8>, BridgeCallFailure> {
        let bytes = serde_json::to_vec(self).map_err(|error| BridgeCallFailure {
            status: BridgeStatusOrUnknown::Known(BridgeStatus::InvalidArgument),
            error_payload: None,
            detail: format!("capabilities request serialization failed: {error}"),
        })?;
        // Defense-in-depth wire gate: the ID cap above already bounds this
        // payload, but the byte cap is enforced at the DLL boundary itself so
        // envelope growth can never silently widen the FFI input.
        if bytes.len() > MAX_CAPABILITIES_REQUEST_WIRE_BYTES {
            return Err(BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(BridgeStatus::InvalidArgument),
                error_payload: None,
                detail: format!(
                    "serialized capabilities request spans {} bytes and exceeds the {MAX_CAPABILITIES_REQUEST_WIRE_BYTES}-byte wire limit",
                    bytes.len()
                ),
            });
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InputConfigJson {
    pub(crate) channels: u32,
    pub(crate) sample_format: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) buffer_frames: BufferFramesRangeJson,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DriverCapabilitiesJson {
    schema_version: u32,
    kind: String,
    abi_version: u32,
    backend: String,
    built: bool,
    pub(crate) driver: DriverEntryJson,
    pub(crate) input_configs: Vec<InputConfigJson>,
}

impl StrictPayload for DriverCapabilitiesJson {
    const KIND: &'static str = "capabilities";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }

    fn validate(&self) -> Result<(), PayloadReject> {
        validate_envelope_identity(self.abi_version, &self.backend, self.built)?;
        validate_driver_entry(&self.driver)?;
        for config in &self.input_configs {
            if config.channels == 0 || config.channels > self.driver.input_channels {
                return Err(PayloadReject::ContractViolation(format!(
                    "capability config channels {} must be between 1 and the driver's {} input channels",
                    config.channels, self.driver.input_channels
                )));
            }
            if !is_supported_native_sample_format(&config.sample_format) {
                return Err(PayloadReject::ContractViolation(
                    "capability config native sample formats must be exactly f32, i16, i24, i32, or f64"
                        .to_owned(),
                ));
            }
            if config.sample_rate_hz == 0 {
                return Err(PayloadReject::ContractViolation(
                    "capability config sample rate must not be zero".to_owned(),
                ));
            }
            validate_buffer_range(&config.buffer_frames)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartResultJson {
    schema_version: u32,
    kind: String,
    actual_buffer_frames: u32,
}

impl StrictPayload for StartResultJson {
    const KIND: &'static str = "start";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StopResultJson {
    schema_version: u32,
    kind: String,
    stopped: bool,
    stream_was_active: bool,
}

impl StrictPayload for StopResultJson {
    const KIND: &'static str = "stop";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }

    fn validate(&self) -> Result<(), PayloadReject> {
        if !self.stopped {
            return Err(PayloadReject::ContractViolation(
                "stop success response must confirm stopped=true".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CloseResultJson {
    schema_version: u32,
    kind: String,
    handle_released: bool,
    stream_was_active: bool,
}

impl StrictPayload for CloseResultJson {
    const KIND: &'static str = "close";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }

    fn validate(&self) -> Result<(), PayloadReject> {
        if !self.handle_released {
            return Err(PayloadReject::ContractViolation(
                "close success response must confirm handleReleased=true".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PercentilesJson {
    p50: u64,
    p95: u64,
    p99: u64,
    max: u64,
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TelemetryJson {
    schema_version: u32,
    kind: String,
    pub(crate) callbacks: u64,
    pub(crate) xruns: u64,
    callback_duration_ns: PercentilesJson,
    capture_delay_ns: PercentilesJson,
}

impl StrictPayload for TelemetryJson {
    const KIND: &'static str = "telemetry";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BridgeErrorPayload {
    schema_version: u32,
    kind: String,
    status: u32,
    code: String,
    message: String,
}

impl StrictPayload for BridgeErrorPayload {
    const KIND: &'static str = "error";

    fn schema_version_field(&self) -> u32 {
        self.schema_version
    }

    fn kind_field(&self) -> &str {
        &self.kind
    }

    fn validate(&self) -> Result<(), PayloadReject> {
        if self.code.trim().is_empty() {
            return Err(PayloadReject::ContractViolation(
                "bridge error payload code must not be empty".to_owned(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ChannelMixGainJson {
    pub(crate) channel_index: u32,
    pub(crate) gain: f64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StartRequestJson {
    schema_version: u32,
    pub(crate) driver_id: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) input_channels: u32,
    pub(crate) sample_format: String,
    pub(crate) fixed_buffer_frames: u32,
    pub(crate) channel_mix: Vec<ChannelMixGainJson>,
}

pub(crate) struct ExplicitStreamParameters<'a> {
    pub(crate) driver_id: &'a str,
    pub(crate) sample_rate_hz: u32,
    pub(crate) input_channels: u32,
    pub(crate) sample_format: &'a str,
    pub(crate) fixed_buffer_frames: u32,
    pub(crate) channel_mix: &'a [ChannelMixGainJson],
}

fn validate_explicit_stream_parameters(
    parameters: &ExplicitStreamParameters<'_>,
) -> Result<(), String> {
    if !valid_persistent_driver_id(parameters.driver_id) {
        return Err(format!(
            "driver_id {:?} must be an explicit persistent asio:<driver name> ID without surrounding whitespace, ASCII control characters, or more than {MAX_DRIVER_ID_BYTES} UTF-8 bytes; default, first-driver, hyphenated, and WASAPI substitutions are unsupported",
            parameters.driver_id
        ));
    }
    if parameters.sample_rate_hz == 0 {
        return Err("sample_rate_hz must not be zero".to_owned());
    }
    if parameters.input_channels == 0 || parameters.input_channels > MAX_INPUT_CHANNELS {
        return Err(format!(
            "input_channels must be between 1 and {MAX_INPUT_CHANNELS}"
        ));
    }
    if !is_supported_native_sample_format(parameters.sample_format) {
        return Err(format!(
            "sample_format {:?} must be exactly f32, i16, i24, i32, or f64",
            parameters.sample_format
        ));
    }
    if parameters.fixed_buffer_frames == 0
        || parameters.fixed_buffer_frames > MAX_FIXED_BUFFER_FRAMES
    {
        return Err(format!(
            "fixed_buffer_frames must be between 1 and {MAX_FIXED_BUFFER_FRAMES}"
        ));
    }
    if parameters.channel_mix.is_empty() || parameters.channel_mix.len() > MAX_CHANNEL_MIX_ENTRIES {
        return Err(format!(
            "channel_mix must contain between 1 and {MAX_CHANNEL_MIX_ENTRIES} entries"
        ));
    }
    let mut has_audible_gain = false;
    // Duplicate input indices would map one physical channel to several gains,
    // an ambiguity the bridge cannot resolve; reject before start or publish.
    let mut seen_mix_indices: HashSet<u32> = HashSet::with_capacity(parameters.channel_mix.len());
    for entry in parameters.channel_mix {
        if entry.channel_index >= parameters.input_channels {
            return Err(format!(
                "channel mix index {} is outside the {} configured input channels",
                entry.channel_index, parameters.input_channels
            ));
        }
        if !seen_mix_indices.insert(entry.channel_index) {
            return Err(format!(
                "channel mix declares input channel index {} more than once; every input mapping must be unambiguous",
                entry.channel_index
            ));
        }
        if !entry.gain.is_finite() {
            return Err("channel mix gains must be finite".to_owned());
        }
        if !(0.0..=MAX_CHANNEL_MIX_GAIN).contains(&entry.gain) {
            return Err(format!(
                "channel mix gain {} must be within 0.0..={MAX_CHANNEL_MIX_GAIN}",
                entry.gain
            ));
        }
        has_audible_gain |= entry.gain != 0.0;
    }
    if !has_audible_gain {
        return Err("channel mix must contain at least one non-zero gain".to_owned());
    }
    Ok(())
}

impl StartRequestJson {
    fn from_explicit(parameters: &ExplicitStreamParameters<'_>) -> Result<Self, PayloadReject> {
        validate_explicit_stream_parameters(parameters)
            .map_err(PayloadReject::ContractViolation)?;
        Ok(Self {
            schema_version: REQUIRED_JSON_SCHEMA_VERSION,
            driver_id: parameters.driver_id.to_owned(),
            sample_rate_hz: parameters.sample_rate_hz,
            input_channels: parameters.input_channels,
            sample_format: parameters.sample_format.to_owned(),
            fixed_buffer_frames: parameters.fixed_buffer_frames,
            channel_mix: parameters.channel_mix.to_vec(),
        })
    }

    fn serialize_wire_bytes(&self) -> Result<Vec<u8>, String> {
        let bytes = serde_json::to_vec(self)
            .map_err(|error| format!("start request serialization failed: {error}"))?;
        if bytes.len() > MAX_JSON_BYTES {
            return Err(format!(
                "serialized start request exceeds the {MAX_JSON_BYTES}-byte bridge limit"
            ));
        }
        Ok(bytes)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SelectionReject {
    DuplicateKey,
    Malformed(String),
    SchemaFuture { found: u32 },
    SchemaUnsupported { found: u32 },
    InvalidField(String),
    SerializationFailed(String),
}

impl fmt::Display for SelectionReject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateKey => {
                f.write_str("persisted selection contains duplicate JSON object keys")
            }
            Self::Malformed(detail) => {
                write!(f, "persisted selection is not strict JSON: {detail}")
            }
            Self::SchemaFuture { found } => write!(
                f,
                "persisted selection schemaVersion {found} was written by a newer Syndocal and stays locked"
            ),
            Self::SchemaUnsupported { found } => write!(
                f,
                "persisted selection schemaVersion {found} is unsupported and stays locked"
            ),
            Self::InvalidField(detail) => {
                write!(f, "persisted selection violates its contract: {detail}")
            }
            Self::SerializationFailed(detail) => {
                write!(f, "persisted selection serialization failed: {detail}")
            }
        }
    }
}

impl Error for SelectionReject {}

impl From<SchemaReject> for SelectionReject {
    fn from(reject: SchemaReject) -> Self {
        match reject {
            SchemaReject::Future { found } => Self::SchemaFuture { found },
            SchemaReject::Unsupported { found } => Self::SchemaUnsupported { found },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PersistedAsioSelection {
    schema_version: u32,
    driver_id: String,
    driver_name: String,
    sample_rate_hz: u32,
    input_channels: u32,
    sample_format: String,
    fixed_buffer_frames: u32,
    channel_mix: Vec<ChannelMixGainJson>,
}

impl PersistedAsioSelection {
    pub(crate) fn new(
        driver_id: String,
        driver_name: String,
        sample_rate_hz: u32,
        input_channels: u32,
        sample_format: String,
        fixed_buffer_frames: u32,
        channel_mix: Vec<ChannelMixGainJson>,
    ) -> Result<Self, SelectionReject> {
        let candidate = Self {
            schema_version: PERSISTED_SELECTION_SCHEMA_VERSION,
            driver_id,
            driver_name,
            sample_rate_hz,
            input_channels,
            sample_format,
            fixed_buffer_frames,
            channel_mix,
        };
        candidate.validate()?;
        Ok(candidate)
    }

    pub(crate) fn parse_storage_text(text: &str) -> Result<Self, SelectionReject> {
        match json_has_duplicate_keys(text) {
            Ok(true) => return Err(SelectionReject::DuplicateKey),
            Ok(false) => {}
            Err(detail) => {
                return Err(SelectionReject::Malformed(format!(
                    "the strict duplicate-key pre-scan rejected this persisted selection because of {detail}"
                )));
            }
        }
        let parsed: Self = serde_json::from_str(text)
            .map_err(|error| SelectionReject::Malformed(error.to_string()))?;
        reject_schema_version(parsed.schema_version).map_err(SelectionReject::from)?;
        parsed.validate()?;
        Ok(parsed)
    }

    pub(crate) fn storage_text(&self) -> Result<String, SelectionReject> {
        serde_json::to_string(self)
            .map_err(|error| SelectionReject::SerializationFailed(error.to_string()))
    }

    fn validate(&self) -> Result<(), SelectionReject> {
        if self.driver_name.trim().is_empty() || self.driver_name.len() > MAX_IDENTITY_TEXT_LEN {
            return Err(SelectionReject::InvalidField(
                "persisted driver_name must be non-empty and bounded".to_owned(),
            ));
        }
        if !(MIN_PERSISTED_SAMPLE_RATE_HZ..=MAX_PERSISTED_SAMPLE_RATE_HZ)
            .contains(&self.sample_rate_hz)
        {
            return Err(SelectionReject::InvalidField(format!(
                "persisted sample_rate_hz must stay within {}..{MAX_PERSISTED_SAMPLE_RATE_HZ}",
                MIN_PERSISTED_SAMPLE_RATE_HZ
            )));
        }
        validate_explicit_stream_parameters(&ExplicitStreamParameters {
            driver_id: &self.driver_id,
            sample_rate_hz: self.sample_rate_hz,
            input_channels: self.input_channels,
            sample_format: &self.sample_format,
            fixed_buffer_frames: self.fixed_buffer_frames,
            channel_mix: &self.channel_mix,
        })
        .map_err(SelectionReject::InvalidField)
    }

    pub(crate) fn driver_id(&self) -> &str {
        &self.driver_id
    }

    pub(crate) fn driver_name(&self) -> &str {
        &self.driver_name
    }

    pub(crate) fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub(crate) fn input_channels(&self) -> u32 {
        self.input_channels
    }

    pub(crate) fn sample_format(&self) -> &str {
        &self.sample_format
    }

    pub(crate) fn fixed_buffer_frames(&self) -> u32 {
        self.fixed_buffer_frames
    }

    pub(crate) fn start_request(&self) -> Result<StartRequestJson, SelectionReject> {
        StartRequestJson::from_explicit(&self.explicit_parameters())
            .map_err(|reject| SelectionReject::InvalidField(reject.to_string()))
    }

    fn explicit_parameters(&self) -> ExplicitStreamParameters<'_> {
        ExplicitStreamParameters {
            driver_id: &self.driver_id,
            sample_rate_hz: self.sample_rate_hz,
            input_channels: self.input_channels,
            sample_format: &self.sample_format,
            fixed_buffer_frames: self.fixed_buffer_frames,
            channel_mix: &self.channel_mix,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaleLockReason {
    CatalogInvalid,
    DriverMissing,
    DriverAmbiguous,
    ConfigurationDrift,
    BackendMismatch,
    NotAsioBuilt,
}

impl fmt::Display for StaleLockReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::CatalogInvalid => "CATALOG_INVALID",
            Self::DriverMissing => "DRIVER_MISSING",
            Self::DriverAmbiguous => "DRIVER_AMBIGUOUS",
            Self::ConfigurationDrift => "CONFIGURATION_DRIFT",
            Self::BackendMismatch => "BACKEND_MISMATCH",
            Self::NotAsioBuilt => "NOT_ASIO_BUILT",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SelectionRevalidationOutcome {
    Revalidated {
        current_driver_id: String,
        start_request: StartRequestJson,
    },
    StillLocked {
        reason: StaleLockReason,
        message: String,
    },
}

fn still_locked(reason: StaleLockReason, message: String) -> SelectionRevalidationOutcome {
    SelectionRevalidationOutcome::StillLocked { reason, message }
}

fn configuration_drift_message(
    driver: &DriverEntryJson,
    selection: &PersistedAsioSelection,
) -> Option<String> {
    if driver.input_channels != selection.input_channels {
        return Some(format!(
            "driver now exposes {} input channels but the persisted selection names {}",
            driver.input_channels, selection.input_channels
        ));
    }
    if !driver.sample_rates_hz.contains(&selection.sample_rate_hz) {
        return Some(format!(
            "driver no longer advertises the persisted sample rate {}",
            selection.sample_rate_hz
        ));
    }
    if !driver.sample_formats.contains(&selection.sample_format) {
        return Some(format!(
            "driver no longer advertises the persisted native format {:?}",
            selection.sample_format
        ));
    }
    if driver.buffer_frames.min > selection.fixed_buffer_frames
        || selection.fixed_buffer_frames > driver.buffer_frames.max
    {
        return Some(format!(
            "driver buffer range {}..{} excludes the persisted fixed buffer {}",
            driver.buffer_frames.min, driver.buffer_frames.max, selection.fixed_buffer_frames
        ));
    }
    None
}

fn revalidated_selection(
    current_driver_id: String,
    selection: &PersistedAsioSelection,
) -> SelectionRevalidationOutcome {
    match selection.start_request() {
        Ok(start_request) => SelectionRevalidationOutcome::Revalidated {
            current_driver_id,
            start_request,
        },
        Err(reject) => still_locked(
            StaleLockReason::CatalogInvalid,
            format!("persisted ASIO selection could not produce a start request: {reject}"),
        ),
    }
}

pub(crate) fn revalidate_selection_with_catalog(
    selection: &PersistedAsioSelection,
    catalog_text: &str,
) -> SelectionRevalidationOutcome {
    if selection.validate().is_err() {
        return still_locked(
            StaleLockReason::CatalogInvalid,
            "the persisted ASIO selection no longer satisfies its own contract".to_owned(),
        );
    }
    let catalog = match parse_strict_payload::<DriverCatalogJson>(catalog_text) {
        Ok(catalog) => catalog,
        Err(reject) => {
            return still_locked(
                StaleLockReason::CatalogInvalid,
                format!("refreshed ASIO catalog was rejected: {reject}"),
            )
        }
    };
    let exact_matches: Vec<&DriverEntryJson> = catalog
        .drivers
        .iter()
        .filter(|driver| driver.id == selection.driver_id)
        .collect();
    if exact_matches.is_empty() {
        return still_locked(
            StaleLockReason::DriverMissing,
            format!(
                "persisted ASIO driver {:?} is absent from the refreshed catalog of {} drivers; the selection stays locked and preserved",
                selection.driver_id,
                catalog.drivers.len()
            ),
        );
    }
    if exact_matches.len() > 1 {
        return still_locked(
            StaleLockReason::DriverAmbiguous,
            format!(
                "persisted ASIO driver {:?} matched {} catalog entries; refusing to guess",
                selection.driver_id,
                exact_matches.len()
            ),
        );
    }
    let driver = exact_matches[0];
    if let Some(message) = configuration_drift_message(driver, selection) {
        return still_locked(StaleLockReason::ConfigurationDrift, message);
    }
    revalidated_selection(driver.id.clone(), selection)
}

pub(crate) fn revalidate_selection_with_capabilities(
    selection: &PersistedAsioSelection,
    capabilities_text: &str,
) -> SelectionRevalidationOutcome {
    if selection.validate().is_err() {
        return still_locked(
            StaleLockReason::CatalogInvalid,
            "the persisted ASIO selection no longer satisfies its own contract".to_owned(),
        );
    }
    let capabilities = match parse_strict_payload::<DriverCapabilitiesJson>(capabilities_text) {
        Ok(capabilities) => capabilities,
        Err(reject) => {
            return still_locked(
                StaleLockReason::CatalogInvalid,
                format!("refreshed ASIO capabilities were rejected: {reject}"),
            )
        }
    };
    if !capabilities.built {
        return still_locked(
            StaleLockReason::NotAsioBuilt,
            "the loaded bridge reports built=false so ASIO streaming is unavailable".to_owned(),
        );
    }
    if capabilities.backend != EXPECTED_BACKEND_LABEL {
        return still_locked(
            StaleLockReason::BackendMismatch,
            format!(
                "capabilities report backend {:?}, expected {EXPECTED_BACKEND_LABEL:?}",
                capabilities.backend
            ),
        );
    }
    if capabilities.driver.id != selection.driver_id {
        return still_locked(
            StaleLockReason::DriverMissing,
            format!(
                "fresh capabilities belong to driver {:?}, not the persisted {:?}; the selection stays locked",
                capabilities.driver.id, selection.driver_id
            ),
        );
    }
    if let Some(message) = configuration_drift_message(&capabilities.driver, selection) {
        return still_locked(StaleLockReason::ConfigurationDrift, message);
    }
    let exact_config_present = capabilities.input_configs.iter().any(|config| {
        config.channels == selection.input_channels
            && config.sample_format == selection.sample_format
            && config.sample_rate_hz == selection.sample_rate_hz
            && config.buffer_frames.min <= selection.fixed_buffer_frames
            && selection.fixed_buffer_frames <= config.buffer_frames.max
    });
    if !exact_config_present {
        return still_locked(
            StaleLockReason::ConfigurationDrift,
            "no refreshed input capability config matches the persisted rate, channels, native format, and buffer exactly".to_owned(),
        );
    }
    revalidated_selection(selection.driver_id.clone(), selection)
}

fn atomic_max_u64(cell: &AtomicU64, value: u64) {
    let mut observed = cell.load(Ordering::Relaxed);
    loop {
        if value <= observed {
            return;
        }
        match cell.compare_exchange_weak(observed, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(seen) => observed = seen,
        }
    }
}

fn atomic_min_u32(cell: &AtomicU32, value: u32) {
    let mut observed = cell.load(Ordering::Relaxed);
    loop {
        if value >= observed {
            return;
        }
        match cell.compare_exchange_weak(observed, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(seen) => observed = seen,
        }
    }
}

fn atomic_max_u32(cell: &AtomicU32, value: u32) {
    let mut observed = cell.load(Ordering::Relaxed);
    loop {
        if value <= observed {
            return;
        }
        match cell.compare_exchange_weak(observed, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => return,
            Err(seen) => observed = seen,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CallbackContextTelemetry {
    pub callbacks_total: u64,
    pub stale_callbacks_total: u64,
    pub terminal_events_total: u64,
    pub warnings_total: u64,
    pub xruns_total: u64,
    pub nonfinite_samples_total: u64,
    pub last_callback_frames: u32,
    pub min_callback_frames: Option<u32>,
    pub max_callback_frames: u32,
    pub max_capture_delay_ns: u64,
    pub pinned_frames: Option<u32>,
    pub latched_terminal_kind: Option<u32>,
    pub safety_zero_requested: bool,
}

/// Application-side realtime sink invoked by the bridge trampolines while the
/// owning stream generation is current and no terminal fault is latched.
///
/// Implementations run inside the ASIO realtime callback: the normal path must
/// not allocate heap memory and must not acquire locks. The trampolines stop
/// invoking `on_samples` permanently once a terminal fault is latched; the
/// application is expected to publish safety zero and require an explicit
/// restart after that point.
pub(crate) trait StreamCallbackHooks: Send + Sync {
    /// Receives the validated mono payload for the current generation.
    fn on_samples(&self, mono_samples: &[f32], capture_delay_ns: u64);

    /// Called exactly once per stream when this callback first latches a
    /// terminal fault. `kind` is the effective terminal kind, including the
    /// loader-internal fault identifiers.
    fn on_terminal_latch(&self, kind: u32);
}

pub(crate) struct CallbackContext {
    generation: u64,
    live_generation: Arc<AtomicU64>,
    hooks: Option<Arc<dyn StreamCallbackHooks>>,
    /// Single-word terminal authority: 0 means streaming (no fault latched);
    /// any nonzero value means the stream is terminally latched with exactly
    /// that effective fault kind (effective kinds are never 0: kind 0 maps to
    /// INTERNAL_FAULT_MALFORMED_EVENT, and the loader-internal faults occupy
    /// u32::MAX-3..=u32::MAX). Publishing latch flag and kind in one word makes
    /// a mixed observation — latched-without-kind or kind-without-latch —
    /// structurally impossible instead of merely unlikely.
    latched_terminal_kind: AtomicU32,
    safety_zero_requested: AtomicBool,
    pinned_frames: AtomicU32,
    callbacks_total: AtomicU64,
    stale_callbacks_total: AtomicU64,
    terminal_events_total: AtomicU64,
    warnings_total: AtomicU64,
    xruns_total: AtomicU64,
    nonfinite_samples_total: AtomicU64,
    last_callback_frames: AtomicU32,
    min_callback_frames: AtomicU32,
    max_callback_frames: AtomicU32,
    max_capture_delay_ns: AtomicU64,
}

impl CallbackContext {
    #[cfg(test)]
    pub(crate) fn new(generation: u64, live_generation: Arc<AtomicU64>) -> Arc<Self> {
        Self::with_hooks(generation, live_generation, None)
    }

    pub(crate) fn with_hooks(
        generation: u64,
        live_generation: Arc<AtomicU64>,
        hooks: Option<Arc<dyn StreamCallbackHooks>>,
    ) -> Arc<Self> {
        Arc::new(Self {
            generation,
            live_generation,
            hooks,
            latched_terminal_kind: AtomicU32::new(0),
            safety_zero_requested: AtomicBool::new(false),
            pinned_frames: AtomicU32::new(0),
            callbacks_total: AtomicU64::new(0),
            stale_callbacks_total: AtomicU64::new(0),
            terminal_events_total: AtomicU64::new(0),
            warnings_total: AtomicU64::new(0),
            xruns_total: AtomicU64::new(0),
            nonfinite_samples_total: AtomicU64::new(0),
            last_callback_frames: AtomicU32::new(0),
            min_callback_frames: AtomicU32::new(u32::MAX),
            max_callback_frames: AtomicU32::new(0),
            max_capture_delay_ns: AtomicU64::new(0),
        })
    }

    fn is_current_generation(&self) -> bool {
        self.live_generation.load(Ordering::Acquire) == self.generation
    }

    fn is_terminal_latched(&self) -> bool {
        self.latched_terminal_kind.load(Ordering::Acquire) != 0
    }

    fn effective_terminal_kind(&self) -> u32 {
        let latched = self.latched_terminal_kind.load(Ordering::Acquire);
        if latched == 0 {
            INTERNAL_FAULT_MALFORMED_EVENT
        } else {
            latched
        }
    }

    /// Delivery gate for the sample trampoline: samples flow only while the
    /// stream generation is current and no terminal fault has been latched.
    fn can_deliver_samples(&self) -> bool {
        !self.is_terminal_latched() && self.is_current_generation()
    }

    fn notify_terminal_latch(&self) {
        if let Some(hooks) = &self.hooks {
            hooks.on_terminal_latch(self.effective_terminal_kind());
        }
    }

    pub(crate) fn request_safety_zero(&self) {
        self.safety_zero_requested.store(true, Ordering::Release);
    }

    pub(crate) fn safety_zero_requested(&self) -> bool {
        self.safety_zero_requested.load(Ordering::Acquire)
    }

    pub(crate) fn latch_terminal(&self, kind: u32) -> bool {
        self.request_safety_zero();
        self.terminal_events_total.fetch_add(1, Ordering::Relaxed);
        let effective_kind = if kind == 0 {
            INTERNAL_FAULT_MALFORMED_EVENT
        } else {
            kind
        };
        // One compare_exchange publishes latched-flag and kind coherently: a
        // reader observes either both (nonzero kind) or neither (0). The first
        // terminal fault wins; later faults only bump terminal_events_total.
        self.latched_terminal_kind
            .compare_exchange(0, effective_kind, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    pub(crate) fn on_event(&self, severity: u32, kind: u32) -> bool {
        if !self.is_current_generation() {
            self.stale_callbacks_total.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        if kind == EVENT_KIND_XRUN {
            // Every backend overrun latches the terminal safety zero no matter
            // which severity the bridge attached: a warning-severity XRUN is
            // still audible corruption under the fail-closed contract.
            self.xruns_total.fetch_add(1, Ordering::Relaxed);
            return self.latch_terminal(kind);
        }
        let terminal = !matches!(severity, EVENT_SEVERITY_WARNING);
        if terminal {
            return self.latch_terminal(kind);
        }
        self.warnings_total.fetch_add(1, Ordering::Relaxed);
        false
    }

    pub(crate) fn on_sample(
        &self,
        mono_samples: *const f32,
        len: usize,
        capture_delay_ns: u64,
        callback_frames: u32,
    ) -> bool {
        if !self.is_current_generation() {
            self.stale_callbacks_total.fetch_add(1, Ordering::Relaxed);
            return false;
        }
        self.callbacks_total.fetch_add(1, Ordering::Relaxed);
        atomic_max_u64(&self.max_capture_delay_ns, capture_delay_ns);
        self.last_callback_frames
            .store(callback_frames, Ordering::Relaxed);
        if callback_frames == 0 || len != callback_frames as usize {
            return self.latch_terminal(INTERNAL_FAULT_FRAME_MISMATCH);
        }
        if let Err(pinned) = self.pinned_frames.compare_exchange(
            0,
            callback_frames,
            Ordering::AcqRel,
            Ordering::Acquire,
        ) {
            if pinned != callback_frames {
                return self.latch_terminal(INTERNAL_FAULT_FRAME_MISMATCH);
            }
        }
        atomic_min_u32(&self.min_callback_frames, callback_frames);
        atomic_max_u32(&self.max_callback_frames, callback_frames);
        let samples = unsafe { std::slice::from_raw_parts(mono_samples, len) };
        let mut latched_nonfinite_this_callback = false;
        for &sample in samples {
            if sample.is_finite() {
                continue;
            }
            self.nonfinite_samples_total.fetch_add(1, Ordering::Relaxed);
            if !latched_nonfinite_this_callback {
                latched_nonfinite_this_callback =
                    self.latch_terminal(INTERNAL_FAULT_NONFINITE_SAMPLE);
            }
        }
        latched_nonfinite_this_callback
    }

    pub(crate) fn snapshot(&self) -> CallbackContextTelemetry {
        CallbackContextTelemetry {
            callbacks_total: self.callbacks_total.load(Ordering::Acquire),
            stale_callbacks_total: self.stale_callbacks_total.load(Ordering::Acquire),
            terminal_events_total: self.terminal_events_total.load(Ordering::Acquire),
            warnings_total: self.warnings_total.load(Ordering::Acquire),
            xruns_total: self.xruns_total.load(Ordering::Acquire),
            nonfinite_samples_total: self.nonfinite_samples_total.load(Ordering::Acquire),
            last_callback_frames: self.last_callback_frames.load(Ordering::Acquire),
            min_callback_frames: match self.min_callback_frames.load(Ordering::Acquire) {
                u32::MAX => None,
                frames => Some(frames),
            },
            max_callback_frames: self.max_callback_frames.load(Ordering::Acquire),
            max_capture_delay_ns: self.max_capture_delay_ns.load(Ordering::Acquire),
            pinned_frames: match self.pinned_frames.load(Ordering::Acquire) {
                0 => None,
                frames => Some(frames),
            },
            latched_terminal_kind: match self.latched_terminal_kind.load(Ordering::Acquire) {
                0 => None,
                kind => Some(kind),
            },
            safety_zero_requested: self.safety_zero_requested(),
        }
    }
}

unsafe extern "C" fn trampoline_sample(
    context: *mut c_void,
    mono_samples: *const f32,
    len: usize,
    capture_delay_ns: u64,
    callback_frames: u32,
) {
    let context = unsafe { &*(context.cast::<CallbackContext>()) };
    if mono_samples.is_null() {
        // A null payload can never be dereferenced safely; fail closed instead
        // of forming a slice from it. Do not call application hooks here:
        // even the terminal hook is an application callback, and this FFI
        // payload is not safe to deliver across that boundary. The context
        // latch is the immediate local safety-zero signal.
        context.latch_terminal(INTERNAL_FAULT_FRAME_MISMATCH);
        return;
    }
    let latched_this_callback =
        context.on_sample(mono_samples, len, capture_delay_ns, callback_frames);
    if latched_this_callback {
        context.notify_terminal_latch();
        return;
    }
    if !context.can_deliver_samples() {
        return;
    }
    if let Some(hooks) = &context.hooks {
        let samples = unsafe { std::slice::from_raw_parts(mono_samples, len) };
        hooks.on_samples(samples, capture_delay_ns);
    }
}

unsafe extern "C" fn trampoline_event(
    context: *mut c_void,
    severity: u32,
    kind: u32,
    _message: *const u8,
    _message_len: usize,
) {
    let context = unsafe { &*(context.cast::<CallbackContext>()) };
    if context.on_event(severity, kind) {
        context.notify_terminal_latch();
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeStatusOrUnknown {
    Known(BridgeStatus),
    Unknown(u32),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BridgeCallFailure {
    pub(crate) status: BridgeStatusOrUnknown,
    pub(crate) error_payload: Option<BridgeErrorPayload>,
    pub(crate) detail: String,
}

impl fmt::Display for BridgeCallFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let status = match &self.status {
            BridgeStatusOrUnknown::Known(status) => status_label(*status).to_owned(),
            BridgeStatusOrUnknown::Unknown(raw) => format!("unknown({raw})"),
        };
        write!(f, "ASIO bridge failure [{status}] {}", self.detail)?;
        if let Some(payload) = &self.error_payload {
            write!(
                f,
                " (bridge code {:?}, message: {})",
                payload.code, payload.message
            )?;
        }
        Ok(())
    }
}

impl Error for BridgeCallFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RawStreamHandle(pub(crate) *mut c_void);

// The pointer is an opaque bridge token: Rust never dereferences it, and one
// AsioStreamSession owns it until stop/close consumes it. All access crosses
// the bridge vtable, whose implementation owns the driver/control thread.
// Moving that sole owner between application threads is therefore permitted;
// sharing the raw token without session serialization remains forbidden.
unsafe impl Send for RawStreamHandle {}

impl RawStreamHandle {
    pub(crate) fn is_null(self) -> bool {
        self.0.is_null()
    }
}

pub(crate) struct RawCloseSlot(pub(crate) *mut c_void);

pub(crate) struct RawStart {
    pub(crate) handle: RawStreamHandle,
    pub(crate) result_json: String,
}

pub(crate) trait BridgeTransport {
    fn start(
        &self,
        request_json: &[u8],
        sample: SampleCallbackFn,
        event: EventCallbackFn,
        context: *mut c_void,
    ) -> Result<RawStart, BridgeCallFailure>;
    fn stop(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure>;
    fn telemetry_json(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure>;
    fn close(&self, slot: &mut RawCloseSlot) -> Result<String, BridgeCallFailure>;
}

pub(crate) struct VtableTransport {
    module: AsioBridgeModule,
}

impl BridgeTransport for VtableTransport {
    fn start(
        &self,
        request_json: &[u8],
        sample: SampleCallbackFn,
        event: EventCallbackFn,
        context: *mut c_void,
    ) -> Result<RawStart, BridgeCallFailure> {
        let mut handle_slot: *mut c_void = std::ptr::null_mut();
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe {
            (self.module.inner.vtable.start)(
                request_json.as_ptr(),
                request_json.len(),
                Some(sample),
                Some(event),
                context,
                &mut handle_slot,
                &mut result,
                &mut error,
            )
        };
        let settled = self.module.settle("start", status, result, error);
        match settled {
            Ok(result_json) => {
                if handle_slot.is_null() {
                    return Err(BridgeCallFailure {
                        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                        error_payload: None,
                        detail: "start succeeded without producing a stream handle".to_owned(),
                    });
                }
                Ok(RawStart {
                    handle: RawStreamHandle(handle_slot),
                    result_json,
                })
            }
            Err(mut failure) => {
                if !handle_slot.is_null() {
                    failure.detail.push_str(
                        "; the bridge leaked a stream handle on a failed start and it was deliberately not freed",
                    );
                }
                Err(failure)
            }
        }
    }

    fn stop(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure> {
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe { (self.module.inner.vtable.stop)(handle.0, &mut result, &mut error) };
        self.module.settle("stop", status, result, error)
    }

    fn telemetry_json(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure> {
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe {
            (self.module.inner.vtable.telemetry_json)(
                handle.0.cast_const(),
                &mut result,
                &mut error,
            )
        };
        self.module.settle("telemetry", status, result, error)
    }

    fn close(&self, slot: &mut RawCloseSlot) -> Result<String, BridgeCallFailure> {
        let mut handle_pointer = slot.0;
        let mut result = BridgeString::EMPTY;
        let mut error = BridgeString::EMPTY;
        let status = unsafe {
            (self.module.inner.vtable.close)(&mut handle_pointer, &mut result, &mut error)
        };
        slot.0 = handle_pointer;
        self.module.settle("close", status, result, error)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SessionStartError {
    GenerationExhausted,
    RequestSerialization(String),
    Call(BridgeCallFailure),
    ResultPayload {
        reject: PayloadReject,
        cleanup: Result<(), BridgeCallFailure>,
    },
    ZeroActualBufferFrames {
        cleanup: Result<(), BridgeCallFailure>,
    },
}

impl fmt::Display for SessionStartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::GenerationExhausted => f.write_str(
                "the ASIO stream generation counter is exhausted; restart Syndocal before starting ASIO input again",
            ),
            Self::RequestSerialization(detail) => write!(f, "{detail}"),
            Self::Call(failure) => write!(f, "{failure}"),
            Self::ResultPayload { reject, cleanup } => {
                write!(f, "start result was rejected: {reject}")?;
                report_started_stream_cleanup(f, cleanup)
            }
            Self::ZeroActualBufferFrames { cleanup } => {
                write!(f, "start reported actualBufferFrames=0")?;
                report_started_stream_cleanup(f, cleanup)
            }
        }
    }
}

fn report_started_stream_cleanup(
    f: &mut fmt::Formatter<'_>,
    cleanup: &Result<(), BridgeCallFailure>,
) -> fmt::Result {
    match cleanup {
        Ok(()) => write!(
            f,
            "; the opened stream was stopped and closed exactly once and no session was created"
        ),
        Err(failure) => write!(
            f,
            "; the opened stream was stopped and closed exactly once but the teardown reported: {failure}"
        ),
    }
}

/// Stops and closes one successfully opened native stream exactly once. Stop
/// runs first so the realtime callbacks retire before the handle closes; Close
/// then consumes the raw handle even when Stop failed. The callback context
/// behind `handle` must stay alive until this returns.
fn stop_and_close_started_stream<T: BridgeTransport>(
    transport: &T,
    handle: RawStreamHandle,
) -> Result<(), BridgeCallFailure> {
    let stop_result = transport.stop(handle);
    let mut slot = RawCloseSlot(handle.0);
    let close_result = transport.close(&mut slot);
    debug_assert!(
        slot.0.is_null() || close_result.is_err(),
        "a successful close must consume the raw stream handle"
    );
    match (stop_result, close_result) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(stop_failure), _) => Err(stop_failure),
        (_, Err(close_failure)) => Err(close_failure),
    }
}

impl Error for SessionStartError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SessionStopError {
    AlreadyStopped,
    Call(BridgeCallFailure),
    ResultPayload(PayloadReject),
}

impl fmt::Display for SessionStopError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyStopped => f.write_str(
                "this ASIO stream was already stopped; restart requires Close followed by a new explicit Start",
            ),
            Self::Call(failure) => write!(f, "{failure}"),
            Self::ResultPayload(reject) => {
                write!(f, "stop result was rejected: {reject}")
            }
        }
    }
}

impl Error for SessionStopError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SessionQueryError {
    Call(BridgeCallFailure),
    ResultPayload(PayloadReject),
}

impl fmt::Display for SessionQueryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Call(failure) => write!(f, "{failure}"),
            Self::ResultPayload(reject) => {
                write!(f, "telemetry result was rejected: {reject}")
            }
        }
    }
}

impl Error for SessionQueryError {}

#[derive(Debug, Clone, PartialEq)]
enum CloseConsumedCause {
    Call(BridgeCallFailure),
    ResultPayload(PayloadReject),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloseConsumedError {
    cause: CloseConsumedCause,
    context_snapshot: CallbackContextTelemetry,
}

impl fmt::Display for CloseConsumedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "ASIO stream close failed and the handle was consumed anyway; restart requires a fresh explicit Start. Cause: "
        )?;
        match &self.cause {
            CloseConsumedCause::Call(failure) => write!(f, "{failure}"),
            CloseConsumedCause::ResultPayload(reject) => {
                write!(f, "close result was rejected: {reject}")
            }
        }
    }
}

impl Error for CloseConsumedError {}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct CloseOutcome {
    report: CloseResultJson,
    generation: u64,
    context_snapshot: CallbackContextTelemetry,
}

impl CloseOutcome {
    #[cfg(test)]
    pub(crate) fn handle_released(&self) -> bool {
        self.report.handle_released
    }

    #[cfg(test)]
    pub(crate) fn stream_was_active(&self) -> bool {
        self.report.stream_was_active
    }

    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    #[cfg(test)]
    pub(crate) fn context_snapshot(&self) -> &CallbackContextTelemetry {
        &self.context_snapshot
    }
}

pub(crate) struct AsioStreamSession<T: BridgeTransport> {
    transport: Arc<T>,
    handle: Option<RawStreamHandle>,
    context: Arc<CallbackContext>,
    generation: u64,
    actual_buffer_frames: u32,
    stopped: bool,
    #[cfg(test)]
    start_report: StartResultJson,
}

impl<T: BridgeTransport> AsioStreamSession<T> {
    #[cfg(test)]
    pub(crate) fn generation(&self) -> u64 {
        self.generation
    }

    pub(crate) fn actual_buffer_frames(&self) -> u32 {
        self.actual_buffer_frames
    }

    pub(crate) fn is_stopped(&self) -> bool {
        self.stopped
    }

    #[cfg(test)]
    pub(crate) fn start_report(&self) -> &StartResultJson {
        &self.start_report
    }

    #[cfg(test)]
    pub(crate) fn context(&self) -> &Arc<CallbackContext> {
        &self.context
    }

    pub(crate) fn stop(&mut self) -> Result<StopResultJson, SessionStopError> {
        if self.stopped {
            return Err(SessionStopError::AlreadyStopped);
        }
        let handle = self.handle.ok_or(SessionStopError::AlreadyStopped)?;
        let text = self
            .transport
            .stop(handle)
            .map_err(SessionStopError::Call)?;
        let parsed = parse_strict_payload::<StopResultJson>(&text)
            .map_err(SessionStopError::ResultPayload)?;
        self.stopped = true;
        Ok(parsed)
    }

    pub(crate) fn telemetry_json(&self) -> Result<TelemetryJson, SessionQueryError> {
        let handle = self.handle.ok_or_else(|| {
            SessionQueryError::Call(BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                error_payload: None,
                detail: "telemetry requires a live handle".to_owned(),
            })
        })?;
        let text = self
            .transport
            .telemetry_json(handle)
            .map_err(SessionQueryError::Call)?;
        parse_strict_payload::<TelemetryJson>(&text).map_err(SessionQueryError::ResultPayload)
    }

    pub(crate) fn close(mut self) -> Result<CloseOutcome, CloseConsumedError> {
        let taken_handle = self.handle.take();
        let mut slot = RawCloseSlot(
            taken_handle
                .map(|handle| handle.0)
                .unwrap_or(std::ptr::null_mut()),
        );
        let snapshot_before_close = self.context.snapshot();
        let outcome = match self.transport.close(&mut slot) {
            Ok(text) => match parse_strict_payload::<CloseResultJson>(&text) {
                Ok(report) => Ok(CloseOutcome {
                    report,
                    generation: self.generation,
                    context_snapshot: self.context.snapshot(),
                }),
                Err(reject) => Err(CloseConsumedError {
                    cause: CloseConsumedCause::ResultPayload(reject),
                    context_snapshot: snapshot_before_close,
                }),
            },
            Err(failure) => Err(CloseConsumedError {
                cause: CloseConsumedCause::Call(failure),
                context_snapshot: snapshot_before_close,
            }),
        };
        drop(self);
        outcome
    }
}

impl<T: BridgeTransport> Drop for AsioStreamSession<T> {
    fn drop(&mut self) {
        if self
            .handle
            .as_ref()
            .map(|handle| !handle.is_null())
            .unwrap_or(false)
        {
            self.context.request_safety_zero();
            self.context
                .latch_terminal(INTERNAL_FAULT_DROP_WITHOUT_CLOSE);
            // The native DLL owns only a raw pointer to this context. If a
            // caller violates the mandatory stop/close lifecycle, retain one
            // strong reference for the process lifetime so a late callback
            // cannot dereference freed memory. The terminal latch above keeps
            // that retained context fail-closed and prevents sample delivery.
            std::mem::forget(Arc::clone(&self.context));
        }
    }
}

pub(crate) fn publish_generation_and_start<T: BridgeTransport>(
    transport: Arc<T>,
    request: &StartRequestJson,
    live_generation: Arc<AtomicU64>,
    hooks: Option<Arc<dyn StreamCallbackHooks>>,
) -> Result<AsioStreamSession<T>, SessionStartError> {
    let request_bytes = request
        .serialize_wire_bytes()
        .map_err(SessionStartError::RequestSerialization)?;
    // checked_add is mandatory: a wrapped counter would publish generation 0
    // and could collide with an identity a retired stream still holds, so
    // overflow must reject before anything is published or opened.
    let generation = live_generation
        .load(Ordering::Acquire)
        .checked_add(1)
        .ok_or(SessionStartError::GenerationExhausted)?;
    let context = CallbackContext::with_hooks(generation, Arc::clone(&live_generation), hooks);
    live_generation.store(generation, Ordering::Release);
    let raw_start = transport
        .start(
            &request_bytes,
            trampoline_sample,
            trampoline_event,
            Arc::as_ptr(&context).cast::<c_void>().cast_mut(),
        )
        .map_err(SessionStartError::Call)?;
    // The native stream is open from here on: every rejection below must stop
    // and close it exactly once while the callback context is still alive.
    let start_report = match parse_strict_payload::<StartResultJson>(&raw_start.result_json) {
        Ok(report) => report,
        Err(reject) => {
            let cleanup = stop_and_close_started_stream(transport.as_ref(), raw_start.handle);
            return Err(SessionStartError::ResultPayload { reject, cleanup });
        }
    };
    if start_report.actual_buffer_frames == 0 {
        let cleanup = stop_and_close_started_stream(transport.as_ref(), raw_start.handle);
        return Err(SessionStartError::ZeroActualBufferFrames { cleanup });
    }
    Ok(AsioStreamSession {
        transport,
        handle: Some(raw_start.handle),
        context,
        generation,
        actual_buffer_frames: start_report.actual_buffer_frames,
        stopped: false,
        #[cfg(test)]
        start_report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::env;
    use std::fs;
    use std::process;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Mutex;

    const DRIVER_A: &str = "asio:TOPPING Pro USB Audio Device";
    const DRIVER_B: &str = "asio:HOTONE AUDIO USB Audio Device";

    struct CountingSink {
        releases: Mutex<Vec<(usize, usize)>>,
    }

    impl CountingSink {
        fn new() -> Self {
            Self {
                releases: Mutex::new(Vec::new()),
            }
        }

        fn released(&self) -> Vec<(usize, usize)> {
            self.releases.lock().unwrap().clone()
        }
    }

    impl StringFreeSink for CountingSink {
        fn release(&self, value: BridgeString) {
            if value.is_empty() {
                return;
            }
            self.releases
                .lock()
                .unwrap()
                .push((value.ptr as usize, value.len));
        }
    }

    fn owned_string(text: &str) -> BridgeString {
        let mut bytes = text.as_bytes().to_vec();
        let ptr = bytes.as_mut_ptr();
        let len = bytes.len();
        std::mem::forget(bytes);
        BridgeString { ptr, len }
    }

    fn owned_bytes(raw: &[u8]) -> BridgeString {
        let mut bytes = raw.to_vec();
        let ptr = bytes.as_mut_ptr();
        let len = bytes.len();
        std::mem::forget(bytes);
        BridgeString { ptr, len }
    }

    fn call_failure(status: u32) -> BridgeCallFailure {
        settle_bridge_strings(
            &CountingSink::new(),
            "probe",
            status,
            BridgeString::EMPTY,
            BridgeString::EMPTY,
        )
        .unwrap_err()
    }

    fn catalog_json() -> String {
        serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [{
                "id": DRIVER_A,
                "name": "TOPPING Pro USB Audio Device",
                "inputChannels": 2,
                "sampleFormats": ["f32", "i24", "i32"],
                "sampleRatesHz": [44100, 48000, 96000],
                "bufferFrames": {"min": 8, "max": 2048}
            }]
        }))
        .unwrap()
    }

    fn capabilities_json() -> String {
        capabilities_json_with_buffer_range(8, 2048)
    }

    fn capabilities_json_with_buffer_range(min_frames: u32, max_frames: u32) -> String {
        serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "capabilities",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "driver": {
                "id": DRIVER_A,
                "name": "TOPPING Pro USB Audio Device",
                "inputChannels": 2,
                "sampleFormats": ["f32", "i24", "i32"],
                "sampleRatesHz": [44100, 48000, 96000],
                "bufferFrames": {"min": min_frames, "max": max_frames}
            },
            "inputConfigs": [{
                "channels": 2,
                "sampleFormat": "i32",
                "sampleRateHz": 48000,
                "bufferFrames": {"min": min_frames, "max": max_frames}
            }]
        }))
        .unwrap()
    }

    fn persisted_selection() -> PersistedAsioSelection {
        PersistedAsioSelection::new(
            DRIVER_A.to_owned(),
            "TOPPING Pro USB Audio Device".to_owned(),
            48_000,
            2,
            "i32".to_owned(),
            128,
            vec![
                ChannelMixGainJson {
                    channel_index: 0,
                    gain: 0.5,
                },
                ChannelMixGainJson {
                    channel_index: 1,
                    gain: 0.5,
                },
            ],
        )
        .unwrap()
    }

    #[test]
    fn required_symbols_are_exactly_nine_canonical_v2_names() {
        // REQUIRED_SYMBOLS is the single authority consumed directly by
        // production resolve_vtable; this test pins that one constant against
        // the documented external ABI v2 export names in resolution order.
        // There is deliberately no second symbol list anywhere.
        let [abi_version, build_flags, drivers_json, capabilities_json, string_free, start, stop, close, telemetry_json] =
            REQUIRED_SYMBOLS;
        assert_eq!(
            [
                abi_version,
                build_flags,
                drivers_json,
                capabilities_json,
                string_free,
                start,
                stop,
                close,
                telemetry_json,
            ],
            [
                "syndocal_asio_v2_abi_version",
                "syndocal_asio_v2_build_flags",
                "syndocal_asio_v2_drivers_json",
                "syndocal_asio_v2_capabilities_json",
                "syndocal_asio_v2_string_free",
                "syndocal_asio_v2_start",
                "syndocal_asio_v2_stop",
                "syndocal_asio_v2_close",
                "syndocal_asio_v2_telemetry_json",
            ]
        );
        assert_eq!(REQUIRED_SYMBOLS.len(), 9);
        let mut sorted = REQUIRED_SYMBOLS.to_vec();
        sorted.sort_unstable();
        let mut unique = sorted.clone();
        unique.dedup();
        assert_eq!(sorted, unique);
        for symbol in REQUIRED_SYMBOLS {
            assert!(
                symbol.starts_with("syndocal_asio_v2_"),
                "symbol {symbol} must carry the v2 prefix"
            );
        }
        for retired in [
            "syndocal_asio_abi_version",
            "syndocal_asio_build_flags",
            "syndocal_asio_drivers_json",
            "syndocal_asio_capabilities_json",
            "syndocal_asio_string_free",
            "syndocal_asio_start",
            "syndocal_asio_play",
            "syndocal_asio_actual_buffer_frames",
            "syndocal_asio_stop",
            "syndocal_asio_xrun_count",
            "syndocal_asio_free",
        ] {
            assert!(
                !REQUIRED_SYMBOLS.contains(&retired),
                "retired ABI v1 symbol {retired} must never be resolved"
            );
        }
    }

    #[test]
    fn canonical_dll_identity_rejects_hyphenated_alias() {
        assert_eq!(CANONICAL_DLL_FILE_NAME, "syndocal_asio_bridge.dll");
        assert!(!CANONICAL_DLL_FILE_NAME.contains('-'));
    }

    #[test]
    fn abi_contract_verification_is_exact() {
        assert_eq!(verify_abi_contract(2, 0x1), Ok(()));
        assert_eq!(verify_abi_contract(2, 0x3), Ok(()));
        assert_eq!(
            verify_abi_contract(1, 0x1),
            Err(BridgeLoadFault::AbiMismatch {
                expected: 2,
                actual: 1
            })
        );
        assert_eq!(
            verify_abi_contract(3, 0x1),
            Err(BridgeLoadFault::AbiMismatch {
                expected: 2,
                actual: 3
            })
        );
        assert_eq!(
            verify_abi_contract(2, 0x0),
            Err(BridgeLoadFault::NotAsioCompiled)
        );
        assert_eq!(
            verify_abi_contract(2, 0x2),
            Err(BridgeLoadFault::NotAsioCompiled)
        );
    }

    #[test]
    fn unknown_bridge_status_fails_closed() {
        for raw in [0u32, 1, 2, 3, 4, 5, 6, 255] {
            assert!(classify_status(raw).is_some());
        }
        for raw in [7u32, 42, 100, 254, 256] {
            assert!(classify_status(raw).is_none());
        }
        let failure = call_failure(97);
        assert_eq!(failure.status, BridgeStatusOrUnknown::Unknown(97));
        assert!(failure.detail.contains("unrecognized status 97"));
    }

    #[test]
    fn probe_reports_not_packaged_and_never_consults_the_hyphen_alias() {
        let counter = NEXT_TEMP_DIR.fetch_add(1, Ordering::Relaxed);
        let directory = env::temp_dir().join(format!(
            "syndocal_asio_bridge_v2_probe_{}_{}",
            process::id(),
            counter
        ));
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("syndocal-asio-bridge.dll"), b"decoy").unwrap();
        let availability = probe_asio_bridge(&directory);
        match &availability {
            AsioBridgeAvailability::NotPackaged { .. } => {}
            other => panic!("expected NotPackaged, got {other:?}"),
        }
        assert_eq!(availability.label(), "NOT_PACKAGED");
        let _ = fs::remove_dir_all(&directory);
    }

    #[test]
    fn duplicate_key_scanner_matches_strict_storage_rules() {
        assert_eq!(json_has_duplicate_keys("{}"), Ok(false));
        assert_eq!(
            json_has_duplicate_keys(r#"{"a":1,"b":{"c":2,"d":[1,2]}}"#),
            Ok(false)
        );
        assert_eq!(
            json_has_duplicate_keys(r#"{"schemaVersion":2,"schemaVersion":3}"#),
            Ok(true)
        );
        assert_eq!(json_has_duplicate_keys(r#"{"a":{"x":1,"x":2}}"#), Ok(true));
        assert_eq!(
            json_has_duplicate_keys(r#"[{"k":1},{"j":2,"j":3}]"#),
            Ok(true)
        );
        assert_eq!(json_has_duplicate_keys(r#"{"\u0061":1,"a":2}"#), Ok(true));
        assert_eq!(json_has_duplicate_keys("{\"a\":1,\"\\u0061\":2}"), Ok(true));
        assert_eq!(
            json_has_duplicate_keys(
                r#"{"text":"brace } quote \" comma , key:","other":[true,false,null]}"#
            ),
            Ok(false)
        );
    }

    #[test]
    fn surrogate_escapes_match_literals_and_malformed_sequences_fail_closed() {
        // A valid UTF-16 surrogate pair combines into exactly the scalar serde
        // uses for the literal UTF-8 encoding: literal vs escaped is a
        // duplicate-key collision, never two different keys.
        assert_eq!(
            json_has_duplicate_keys(r#"{"😀":1,"\uD83D\uDE00":2}"#),
            Ok(true)
        );
        // Both escaped spellings of the same scalar collide with each other,
        // regardless of hexadecimal case.
        assert_eq!(
            json_has_duplicate_keys(r#"{"\uD83D\uDE00":1,"\ud83d\ude00":2}"#),
            Ok(true)
        );
        // Distinct scalars stay distinct.
        assert_eq!(
            json_has_duplicate_keys(r#"{"\uD83D\uDE00":1,"😀x":2}"#),
            Ok(false)
        );
        // Escaped slash and other short escapes compare equal to literals.
        assert_eq!(json_has_duplicate_keys(r#"{"a/b":1,"a\/b":2}"#), Ok(true));
        assert_eq!(
            json_has_duplicate_keys(r#"{"a\\b":1,"a\u005Cb":2}"#),
            Ok(true)
        );

        let malformed_fixtures: [(&str, &str); 8] = [
            ("lone high surrogate", r#"{"\uD83D":1,"other":2}"#),
            (
                "high without low before a plain char",
                r#"{"\uD83DX":1,"other":2}"#,
            ),
            (
                "high followed by a non-low escape",
                r#"{"\uD83D\u0041":1,"other":2}"#,
            ),
            ("lone low surrogate", r#"{"\uDE00":1,"other":2}"#),
            ("reversed surrogate pair", r#"{"\uDE00\uD83D":1,"other":2}"#),
            ("truncated second unit", r#"{"\uD83D\uDE0":1,"other":2}"#),
            ("non-hexadecimal escape digit", r#"{"\uZZZZ":1,"other":2}"#),
            (
                "second unit truncated after \\u",
                r#"{"\uD83D\uD":1,"other":2}"#,
            ),
        ];
        for (label, text) in malformed_fixtures {
            let verdict = json_has_duplicate_keys(text);
            assert!(verdict.is_err(), "{label} must fail closed");
            if label.contains("high") {
                assert!(
                    verdict.unwrap_err().contains("high surrogate"),
                    "{label} must name the high surrogate defect"
                );
            }
        }

        // End-to-end: equivalent spellings reach serde's silent last-wins
        // merge only if the pre-scan misses them, so the strict parser must
        // report DuplicateKey for the literal/escaped collision...
        let duplicated_catalog = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":2,"backend":"asio","built":true,"drivers":[],"driv\u0065rs":[],"😀":1,"\uD83D\uDE00":2}"#;
        assert_eq!(
            parse_strict_payload::<DriverCatalogJson>(duplicated_catalog)
                .err()
                .expect("equivalent escaped/literal keys must be rejected"),
            PayloadReject::DuplicateKey
        );
        // ...and Malformed (never "no duplicate") for a malformed surrogate.
        let lone_surrogate_catalog = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":2,"backend":"asio","built":true,"drivers":[],"😀":1,"\uD83D":2}"#;
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(lone_surrogate_catalog),
            Err(PayloadReject::Malformed(detail)) if detail.contains("high surrogate")
        ));

        // The persisted-selection entry point enforces the same rules.
        let duplicated_selection = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0}],"sampl\u0065Format":"f32"}"#;
        assert_eq!(
            PersistedAsioSelection::parse_storage_text(duplicated_selection),
            Err(SelectionReject::DuplicateKey)
        );
        let lone_surrogate_selection = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0}],"\uDE00":9}"#;
        assert!(matches!(
            PersistedAsioSelection::parse_storage_text(lone_surrogate_selection),
            Err(SelectionReject::Malformed(detail)) if detail.contains("low surrogate")
        ));
    }

    #[test]
    fn catalog_accepts_valid_payload_and_rejects_contract_breaks() {
        let parsed: DriverCatalogJson =
            parse_strict_payload(&catalog_json()).expect("valid catalog must parse");
        assert_eq!(parsed.drivers.len(), 1);
        assert_eq!(parsed.drivers[0].id, DRIVER_A);

        let unknown_field = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":2,"backend":"asio","built":true,"extra":1,"drivers":[]}"#;
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(unknown_field),
            Err(PayloadReject::Malformed(_))
        ));

        let future = r#"{"schemaVersion":3,"kind":"drivers","abiVersion":2,"backend":"asio","built":true,"drivers":[]}"#;
        assert_eq!(
            parse_strict_payload::<DriverCatalogJson>(future),
            Err(PayloadReject::SchemaFuture { found: 3 })
        );

        let legacy = r#"{"schemaVersion":1,"kind":"drivers","abiVersion":2,"backend":"asio","built":true,"drivers":[]}"#;
        assert_eq!(
            parse_strict_payload::<DriverCatalogJson>(legacy),
            Err(PayloadReject::SchemaUnsupported { found: 1 })
        );

        let wrong_kind = r#"{"schemaVersion":2,"kind":"capabilities","abiVersion":2,"backend":"asio","built":true,"drivers":[]}"#;
        assert_eq!(
            parse_strict_payload::<DriverCatalogJson>(wrong_kind),
            Err(PayloadReject::KindMismatch {
                expected: "drivers",
                found: "capabilities".to_owned()
            })
        );

        let duplicated = r#"{"schemaVersion":2,"kind":"drivers","schemaVersion":2,"abiVersion":2,"backend":"asio","built":true,"drivers":[]}"#;
        assert_eq!(
            parse_strict_payload::<DriverCatalogJson>(duplicated),
            Err(PayloadReject::DuplicateKey)
        );

        let not_built = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":2,"backend":"asio","built":false,"drivers":[]}"#;
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(not_built),
            Err(PayloadReject::ContractViolation(message)) if message.contains("built=false")
        ));

        let wrong_backend = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":2,"backend":"wasapi","built":true,"drivers":[]}"#;
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(wrong_backend),
            Err(PayloadReject::ContractViolation(_))
        ));

        let wrong_abi = r#"{"schemaVersion":2,"kind":"drivers","abiVersion":1,"backend":"asio","built":true,"drivers":[]}"#;
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(wrong_abi),
            Err(PayloadReject::ContractViolation(message)) if message.contains("abi_version 1")
        ));

        let non_persistent_id = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [
                {"id": "TOPPING Pro USB Audio Device", "name": "T", "inputChannels": 2, "sampleFormats": ["i32"], "sampleRatesHz": [48000], "bufferFrames": {"min": 8, "max": 64}}
            ]
        }))
        .unwrap();
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(&non_persistent_id),
            Err(PayloadReject::ContractViolation(message)) if message.contains("persistent asio:")
        ));

        let inverted_range = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [
                {"id": DRIVER_A, "name": "T", "inputChannels": 2, "sampleFormats": ["i32"], "sampleRatesHz": [48000], "bufferFrames": {"min": 64, "max": 8}}
            ]
        }))
        .unwrap();
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(&inverted_range),
            Err(PayloadReject::ContractViolation(_))
        ));

        let unknown_format = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [
                {"id": DRIVER_A, "name": "T", "inputChannels": 2, "sampleFormats": ["u16"], "sampleRatesHz": [48000], "bufferFrames": {"min": 8, "max": 64}}
            ]
        }))
        .unwrap();
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(&unknown_format),
            Err(PayloadReject::ContractViolation(_))
        ));
    }

    #[test]
    fn capabilities_lock_the_selection_and_never_substitute() {
        let selection = persisted_selection();
        let outcome = revalidate_selection_with_capabilities(&selection, &capabilities_json());
        let expected_request = selection.start_request().unwrap();
        match outcome {
            SelectionRevalidationOutcome::Revalidated {
                current_driver_id,
                start_request,
            } => {
                assert_eq!(current_driver_id, DRIVER_A);
                assert_eq!(start_request, expected_request);
                assert_eq!(start_request.driver_id, DRIVER_A);
                assert_eq!(start_request.sample_rate_hz, 48_000);
                assert_eq!(start_request.input_channels, 2);
                assert_eq!(start_request.sample_format, "i32");
                assert_eq!(start_request.fixed_buffer_frames, 128);
                assert_eq!(start_request.channel_mix.len(), 2);
            }
            other => panic!("expected Revalidated, got {other:?}"),
        }

        let other_driver = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "capabilities",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "driver": {
                "id": DRIVER_B,
                "name": "HOTONE AUDIO USB Audio Device",
                "inputChannels": 2,
                "sampleFormats": ["i32"],
                "sampleRatesHz": [44100],
                "bufferFrames": {"min": 8, "max": 2048}
            },
            "inputConfigs": []
        }))
        .unwrap();
        match revalidate_selection_with_capabilities(&selection, &other_driver) {
            SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, StaleLockReason::DriverMissing)
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let drifted_rate = capabilities_json().replace("48000", "44100");
        match revalidate_selection_with_capabilities(&selection, &drifted_rate) {
            SelectionRevalidationOutcome::StillLocked { reason, message } => {
                assert_eq!(reason, StaleLockReason::ConfigurationDrift);
                assert!(message.contains("rate") || message.contains("config"));
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let drifted_format = capabilities_json().replace("\"i32\"", "\"f32\"");
        match revalidate_selection_with_capabilities(&selection, &drifted_format) {
            SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, StaleLockReason::ConfigurationDrift)
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let shrunk_buffers = capabilities_json_with_buffer_range(512, 2048);
        match revalidate_selection_with_capabilities(&selection, &shrunk_buffers) {
            SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, StaleLockReason::ConfigurationDrift)
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let malformed = "{\"schemaVersion\":2,";
        match revalidate_selection_with_capabilities(&selection, malformed) {
            SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, StaleLockReason::CatalogInvalid)
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }
    }

    #[test]
    fn catalog_revalidation_stays_locked_when_missing_or_ambiguous() {
        let selection = persisted_selection();

        let others_only = catalog_json().replace(DRIVER_A, DRIVER_B);
        match revalidate_selection_with_catalog(&selection, &others_only) {
            SelectionRevalidationOutcome::StillLocked { reason, message } => {
                assert_eq!(reason, StaleLockReason::DriverMissing);
                assert!(message.contains(DRIVER_A));
                assert!(message.contains("stays locked"));
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let ambiguous = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [
                {"id": DRIVER_A, "name": "One", "inputChannels": 2, "sampleFormats": ["i32"], "sampleRatesHz": [48000], "bufferFrames": {"min": 8, "max": 64}},
                {"id": DRIVER_A, "name": "Two", "inputChannels": 2, "sampleFormats": ["i32"], "sampleRatesHz": [48000], "bufferFrames": {"min": 8, "max": 64}}
            ]
        }))
        .unwrap();
        match revalidate_selection_with_catalog(&selection, &ambiguous) {
            SelectionRevalidationOutcome::StillLocked { reason, .. } => {
                assert_eq!(reason, StaleLockReason::DriverAmbiguous)
            }
            other => panic!("expected StillLocked, got {other:?}"),
        }

        let happy = revalidate_selection_with_catalog(&selection, &catalog_json());
        match happy {
            SelectionRevalidationOutcome::Revalidated {
                current_driver_id, ..
            } => assert_eq!(current_driver_id, DRIVER_A),
            other => panic!("expected Revalidated, got {other:?}"),
        }
    }

    #[test]
    fn start_request_builder_enforces_every_explicit_bound() {
        let ok = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[ChannelMixGainJson {
                channel_index: 0,
                gain: 0.5,
            }],
        })
        .unwrap();
        assert_eq!(ok.schema_version, REQUIRED_JSON_SCHEMA_VERSION);

        let subtle_over_one = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[ChannelMixGainJson {
                channel_index: 0,
                gain: 1.0000000001,
            }],
        });
        assert!(subtle_over_one.is_err());

        for gain in [-0.5_f64, 1.5, f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
            let rejected = StartRequestJson::from_explicit(&ExplicitStreamParameters {
                driver_id: DRIVER_A,
                sample_rate_hz: 48_000,
                input_channels: 2,
                sample_format: "i32",
                fixed_buffer_frames: 128,
                channel_mix: &[ChannelMixGainJson {
                    channel_index: 0,
                    gain,
                }],
            });
            assert!(rejected.is_err(), "gain {gain} must be rejected");
        }

        let all_silent = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[ChannelMixGainJson {
                channel_index: 0,
                gain: 0.0,
            }],
        });
        assert!(all_silent
            .unwrap_err()
            .to_string()
            .contains("at least one non-zero gain"));

        let index_overflow = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[ChannelMixGainJson {
                channel_index: 2,
                gain: 1.0,
            }],
        });
        assert!(index_overflow.is_err());

        let empty_mix = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[],
        });
        assert!(empty_mix.is_err());

        let oversized_mix: Vec<ChannelMixGainJson> = (0..=MAX_CHANNEL_MIX_ENTRIES)
            .map(|index| ChannelMixGainJson {
                channel_index: index as u32 % 2,
                gain: if index == 0 { 1.0 } else { 0.0 },
            })
            .collect();
        let oversized = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &oversized_mix,
        });
        assert!(oversized.is_err());

        let max_mix: Vec<ChannelMixGainJson> = (0..MAX_CHANNEL_MIX_ENTRIES)
            .map(|index| ChannelMixGainJson {
                channel_index: index as u32,
                gain: if index == 0 { 1.0 } else { 0.0 },
            })
            .collect();
        let max_ok = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            // Unique indices require one channel per entry.
            input_channels: MAX_CHANNEL_MIX_ENTRIES as u32,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &max_mix,
        });
        assert!(max_ok.is_ok());

        for driver_id in [
            "TOPPING",
            "asio:",
            " asio:Topping",
            "asio:Topping ",
            "wasapi:x",
        ] {
            let rejected = StartRequestJson::from_explicit(&ExplicitStreamParameters {
                driver_id,
                sample_rate_hz: 48_000,
                input_channels: 2,
                sample_format: "i32",
                fixed_buffer_frames: 128,
                channel_mix: &[ChannelMixGainJson {
                    channel_index: 0,
                    gain: 1.0,
                }],
            });
            assert!(
                rejected.is_err(),
                "driver id {driver_id:?} must be rejected"
            );
        }

        for buffer_frames in [0u32, MAX_FIXED_BUFFER_FRAMES + 1] {
            let rejected = StartRequestJson::from_explicit(&ExplicitStreamParameters {
                driver_id: DRIVER_A,
                sample_rate_hz: 48_000,
                input_channels: 2,
                sample_format: "i32",
                fixed_buffer_frames: buffer_frames,
                channel_mix: &[ChannelMixGainJson {
                    channel_index: 0,
                    gain: 1.0,
                }],
            });
            assert!(rejected.is_err());
        }

        for channels in [0u32, u16::MAX as u32 + 1] {
            let rejected = StartRequestJson::from_explicit(&ExplicitStreamParameters {
                driver_id: DRIVER_A,
                sample_rate_hz: 48_000,
                input_channels: channels,
                sample_format: "i32",
                fixed_buffer_frames: 128,
                channel_mix: &[ChannelMixGainJson {
                    channel_index: 0,
                    gain: 1.0,
                }],
            });
            assert!(rejected.is_err());
        }

        for format in ["I32", "f32 ", "", "s16"] {
            let rejected = StartRequestJson::from_explicit(&ExplicitStreamParameters {
                driver_id: DRIVER_A,
                sample_rate_hz: 48_000,
                input_channels: 2,
                sample_format: format,
                fixed_buffer_frames: 128,
                channel_mix: &[ChannelMixGainJson {
                    channel_index: 0,
                    gain: 1.0,
                }],
            });
            assert!(rejected.is_err(), "format {format:?} must be rejected");
        }

        let zero_rate = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 0,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &[ChannelMixGainJson {
                channel_index: 0,
                gain: 1.0,
            }],
        });
        assert!(zero_rate.is_err());
    }

    #[test]
    fn duplicate_channel_mix_indices_are_rejected_before_start_or_publish() {
        let duplicated_mix = [
            ChannelMixGainJson {
                channel_index: 1,
                gain: 1.0,
            },
            ChannelMixGainJson {
                channel_index: 0,
                gain: 0.25,
            },
            ChannelMixGainJson {
                channel_index: 1,
                gain: 0.5,
            },
        ];
        let start_reject = StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &duplicated_mix,
        })
        .expect_err("a duplicated input index must never produce a start request");
        assert!(
            start_reject.to_string().contains("more than once"),
            "rejection must name the ambiguity: {start_reject}"
        );

        let triple_duplicate = vec![
            ChannelMixGainJson {
                channel_index: 0,
                gain: 1.0,
            },
            ChannelMixGainJson {
                channel_index: 0,
                gain: 0.5,
            },
            ChannelMixGainJson {
                channel_index: 0,
                gain: 0.25,
            },
        ];
        assert!(StartRequestJson::from_explicit(&ExplicitStreamParameters {
            driver_id: DRIVER_A,
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "i32",
            fixed_buffer_frames: 128,
            channel_mix: &triple_duplicate,
        })
        .is_err());

        let persisted = PersistedAsioSelection::new(
            DRIVER_A.to_owned(),
            "TOPPING Pro USB Audio Device".to_owned(),
            48_000,
            2,
            "i32".to_owned(),
            128,
            vec![
                ChannelMixGainJson {
                    channel_index: 0,
                    gain: 1.0,
                },
                ChannelMixGainJson {
                    channel_index: 0,
                    gain: 0.5,
                },
            ],
        );
        assert!(matches!(
            persisted,
            Err(SelectionReject::InvalidField(message)) if message.contains("more than once")
        ));

        let stored_duplicate = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":1,"gain":1.0},{"channelIndex":1,"gain":0.5}]}"#;
        assert!(matches!(
            PersistedAsioSelection::parse_storage_text(stored_duplicate),
            Err(SelectionReject::InvalidField(message)) if message.contains("more than once")
        ));
    }

    #[test]
    fn persisted_selection_roundtrips_and_future_versions_stay_rejected() {
        let selection = persisted_selection();
        let text = selection.storage_text().unwrap();
        let reparsed = PersistedAsioSelection::parse_storage_text(&text).unwrap();
        assert_eq!(reparsed, selection);
        assert_eq!(reparsed.driver_id(), DRIVER_A);

        let future = text.replace("\"schemaVersion\":2", "\"schemaVersion\":3");
        assert_eq!(
            PersistedAsioSelection::parse_storage_text(&future),
            Err(SelectionReject::SchemaFuture { found: 3 })
        );

        let legacy = text.replace("\"schemaVersion\":2", "\"schemaVersion\":1");
        assert_eq!(
            PersistedAsioSelection::parse_storage_text(&legacy),
            Err(SelectionReject::SchemaUnsupported { found: 1 })
        );

        let duplicated = format!(
            "{{{},{}}}",
            text.trim_start_matches('{').trim_end_matches('}'),
            text.trim_start_matches('{').trim_end_matches('}')
        );
        assert_eq!(
            PersistedAsioSelection::parse_storage_text(&duplicated),
            Err(SelectionReject::DuplicateKey)
        );

        let unknown_field = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0}],"legacyKey":1}"#;
        assert!(matches!(
            PersistedAsioSelection::parse_storage_text(unknown_field),
            Err(SelectionReject::Malformed(_))
        ));

        let bad_rate = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":7999,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0}]}"#;
        assert!(matches!(
            PersistedAsioSelection::parse_storage_text(bad_rate),
            Err(SelectionReject::InvalidField(detail)) if detail.contains("sample_rate_hz")
        ));

        let bad_gain = r#"{"schemaVersion":2,"driverId":"asio:X","driverName":"X","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"i32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0000000001}]}"#;
        assert!(matches!(
            PersistedAsioSelection::parse_storage_text(bad_gain),
            Err(SelectionReject::InvalidField(_))
        ));
    }

    #[test]
    fn error_payload_parses_and_display_carries_details() {
        let payload = r#"{"schemaVersion":2,"kind":"error","status":3,"code":"driver_not_found","message":"explicit driver was absent"}"#;
        let parsed: BridgeErrorPayload = parse_strict_payload(payload).unwrap();
        assert_eq!(parsed.status, 3);
        assert_eq!(parsed.code, "driver_not_found");

        let failure = BridgeCallFailure {
            status: BridgeStatusOrUnknown::Known(BridgeStatus::DriverNotFound),
            error_payload: Some(parsed),
            detail: "capability query: explicit driver was absent".to_owned(),
        };
        let display = failure.to_string();
        assert!(display.contains("driver_not_found"));
        assert!(display.contains("capability query"));
        assert!(display.contains("explicit driver was absent"));

        let empty_code = r#"{"schemaVersion":2,"kind":"error","status":1,"code":"","message":"x"}"#;
        assert!(matches!(
            parse_strict_payload::<BridgeErrorPayload>(empty_code),
            Err(PayloadReject::ContractViolation(_))
        ));
    }

    #[test]
    fn telemetry_payload_requires_exact_shape() {
        let valid = r#"{"schemaVersion":2,"kind":"telemetry","callbacks":10,"xruns":0,"callbackDurationNs":{"p50":100,"p95":150,"p99":180,"max":200},"captureDelayNs":{"p50":1,"p95":2,"p99":3,"max":4}}"#;
        let parsed: TelemetryJson = parse_strict_payload(valid).unwrap();
        assert_eq!(parsed.callbacks, 10);
        assert_eq!(parsed.callback_duration_ns.max, 200);

        let extra_field = r#"{"schemaVersion":2,"kind":"telemetry","callbacks":10,"xruns":0,"callbackDurationNs":{"p50":1,"p95":1,"p99":1,"max":1,"p100":9},"captureDelayNs":{"p50":1,"p95":1,"p99":1,"max":1}}"#;
        assert!(matches!(
            parse_strict_payload::<TelemetryJson>(extra_field),
            Err(PayloadReject::Malformed(_))
        ));

        let wrong_kind = r#"{"schemaVersion":2,"kind":"stop","callbacks":10,"xruns":0,"callbackDurationNs":{"p50":1,"p95":1,"p99":1,"max":1},"captureDelayNs":{"p50":1,"p95":1,"p99":1,"max":1}}"#;
        assert!(matches!(
            parse_strict_payload::<TelemetryJson>(wrong_kind),
            Err(PayloadReject::KindMismatch { .. })
        ));
    }

    #[test]
    fn generation_fence_ignores_callbacks_from_retired_streams() {
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::new(1, Arc::clone(&live));
        let samples = [0.0_f32; 8];
        live.store(2, Ordering::Release);
        assert!(!context.on_sample(samples.as_ptr(), samples.len(), 5, 8));
        assert!(!context.on_event(EVENT_SEVERITY_TERMINAL, EVENT_KIND_XRUN));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.callbacks_total, 0);
        assert_eq!(snapshot.stale_callbacks_total, 2);
        assert_eq!(snapshot.latched_terminal_kind, None);
        assert!(!snapshot.safety_zero_requested);
    }

    #[test]
    fn terminal_fault_latch_is_one_shot_with_immediate_safety_zero() {
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::new(1, Arc::clone(&live));
        assert!(!context.safety_zero_requested());
        assert!(context.on_event(EVENT_SEVERITY_TERMINAL, 5));
        assert!(context.safety_zero_requested());
        assert!(!context.on_event(EVENT_SEVERITY_TERMINAL, 6));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.latched_terminal_kind, Some(5));
        assert_eq!(snapshot.terminal_events_total, 2);

        assert!(!context.latch_terminal(INTERNAL_FAULT_FRAME_MISMATCH));
        assert!(!context.latch_terminal(INTERNAL_FAULT_NONFINITE_SAMPLE));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.latched_terminal_kind, Some(5));
        assert_eq!(snapshot.terminal_events_total, 4);
    }

    #[test]
    fn terminal_latch_publishes_flag_and_kind_as_one_coherent_word_under_race() {
        const PUBLISHERS: u32 = 8;
        for round in 0..64_u64 {
            let live = Arc::new(AtomicU64::new(round + 1));
            let context = CallbackContext::new(round + 1, Arc::clone(&live));
            let barrier = Arc::new(std::sync::Barrier::new(PUBLISHERS as usize));
            let winners = Arc::new(AtomicU64::new(0));
            let handles: Vec<_> = (0..PUBLISHERS)
                .map(|publisher| {
                    let context = Arc::clone(&context);
                    let barrier = Arc::clone(&barrier);
                    let winners = Arc::clone(&winners);
                    std::thread::spawn(move || {
                        barrier.wait();
                        if context.latch_terminal(100 + publisher) {
                            winners.fetch_add(1, Ordering::Relaxed);
                        }
                    })
                })
                .collect();
            for handle in handles {
                handle.join().unwrap();
            }
            assert_eq!(
                winners.load(Ordering::Relaxed),
                1,
                "exactly one publisher may win the one-shot latch"
            );
            // The winning kind and the latched state are one word: a snapshot
            // taken after any successful latch can never show a latched stream
            // without its kind or a kind without the latch.
            let snapshot = context.snapshot();
            let kind = snapshot
                .latched_terminal_kind
                .expect("a won latch must publish its kind in the same word");
            assert!((100..100 + PUBLISHERS).contains(&kind));
            assert_eq!(snapshot.terminal_events_total, u64::from(PUBLISHERS));
            assert!(snapshot.safety_zero_requested);
            assert!(!context.can_deliver_samples());
            assert_eq!(context.effective_terminal_kind(), kind);
        }
    }

    #[test]
    fn a_frozen_stream_is_never_observed_without_its_latched_kind() {
        for round in 0..64_u32 {
            let live = Arc::new(AtomicU64::new(u64::from(round) + 1));
            let context = CallbackContext::new(u64::from(round) + 1, Arc::clone(&live));
            let publisher = {
                let context = Arc::clone(&context);
                std::thread::spawn(move || {
                    context.latch_terminal(EVENT_KIND_XRUN);
                })
            };
            // Poll the telemetry word while the latch races in. The latch is
            // monotonic single-word state, so two properties must hold at every
            // instant: an observed latch always carries the exact kind, and
            // once any observation saw the latch no later observation can see
            // it gone; observing the kind then re-reading the delivery gate
            // (a later load) must therefore report the stream frozen.
            let mut ever_latched = false;
            for _ in 0..20_000 {
                match context.snapshot().latched_terminal_kind {
                    None => {
                        assert!(
                            !ever_latched,
                            "impossible mixed state: the latched word regressed to zero"
                        );
                    }
                    Some(kind) => {
                        assert_eq!(kind, EVENT_KIND_XRUN);
                        ever_latched = true;
                        assert!(
                            !context.can_deliver_samples(),
                            "a latched stream must be frozen for every later load"
                        );
                    }
                }
            }
            publisher.join().unwrap();
            let final_snapshot = context.snapshot();
            if ever_latched || final_snapshot.latched_terminal_kind.is_some() {
                assert_eq!(
                    final_snapshot.latched_terminal_kind,
                    Some(EVENT_KIND_XRUN),
                    "round {round}: the coherent latch must be visible after the publisher joined"
                );
            } else {
                // The publisher never got scheduled during polling; after join
                // its latch is globally visible within a finite number of
                // Acquire loads.
                let mut saw_coherent_latch = false;
                for _ in 0..1_000_000 {
                    if let Some(kind) = context.snapshot().latched_terminal_kind {
                        assert_eq!(kind, EVENT_KIND_XRUN);
                        assert!(!context.can_deliver_samples());
                        saw_coherent_latch = true;
                        break;
                    }
                }
                assert!(
                    saw_coherent_latch,
                    "round {round}: the coherent latch must become visible"
                );
            }
        }
    }

    #[test]
    fn every_xrun_latches_terminal_safety_zero_regardless_of_bridge_severity() {
        for severity in [
            0_u32,
            EVENT_SEVERITY_WARNING,
            EVENT_SEVERITY_TERMINAL,
            7,
            99,
        ] {
            let live = Arc::new(AtomicU64::new(1));
            let context = CallbackContext::new(1, Arc::clone(&live));
            assert!(
                context.on_event(severity, EVENT_KIND_XRUN),
                "XRUN at severity {severity} must latch the terminal fault"
            );
            let snapshot = context.snapshot();
            assert_eq!(snapshot.xruns_total, 1);
            assert_eq!(snapshot.latched_terminal_kind, Some(EVENT_KIND_XRUN));
            assert!(snapshot.safety_zero_requested);
        }
        // Non-XRUN severities keep their existing classification: unknown
        // severities fail closed as terminal, only exact warnings stay soft.
        let live = Arc::new(AtomicU64::new(1));
        let warning_context = CallbackContext::new(1, Arc::clone(&live));
        assert!(!warning_context.on_event(EVENT_SEVERITY_WARNING, 99));
        let snapshot = warning_context.snapshot();
        assert_eq!(snapshot.warnings_total, 1);
        assert_eq!(snapshot.xruns_total, 0);
        assert_eq!(snapshot.latched_terminal_kind, None);
        assert!(!snapshot.safety_zero_requested);

        let terminal_context = CallbackContext::new(1, Arc::clone(&live));
        assert!(terminal_context.on_event(EVENT_SEVERITY_TERMINAL, 2));
        assert!(
            !terminal_context.on_event(7, 3),
            "the terminal latch is one-shot even when a later event is terminal"
        );
        assert!(!terminal_context.on_event(0, 4));
        let snapshot = terminal_context.snapshot();
        assert_eq!(snapshot.xruns_total, 0);
        assert_eq!(snapshot.terminal_events_total, 3);
        assert_eq!(snapshot.latched_terminal_kind, Some(2));
    }

    #[test]
    fn sample_callback_pins_frames_and_latches_mismatches_once() {
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::new(1, Arc::clone(&live));
        let first = [0.25_f32; 128];
        assert!(!context.on_sample(first.as_ptr(), 128, 500, 128));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.pinned_frames, Some(128));
        assert_eq!(snapshot.min_callback_frames, Some(128));
        assert_eq!(snapshot.max_callback_frames, 128);
        assert_eq!(snapshot.last_callback_frames, 128);
        assert_eq!(snapshot.max_capture_delay_ns, 500);

        let second = [0.5_f32; 64];
        assert!(context.on_sample(second.as_ptr(), 64, 900, 64));
        assert!(!context.on_sample(second.as_ptr(), 64, 900, 64));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.callbacks_total, 3);
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(INTERNAL_FAULT_FRAME_MISMATCH)
        );
        assert!(snapshot.safety_zero_requested);
        assert_eq!(snapshot.max_capture_delay_ns, 900);
        assert_eq!(snapshot.pinned_frames, Some(128));

        let zero_frames: [f32; 0] = [];
        let fresh_context = CallbackContext::new(1, Arc::clone(&live));
        assert!(fresh_context.on_sample(zero_frames.as_ptr(), 0, 0, 0));
        assert_eq!(
            fresh_context.snapshot().latched_terminal_kind,
            Some(INTERNAL_FAULT_FRAME_MISMATCH)
        );
    }

    #[test]
    fn nonfinite_samples_latch_terminal_once_per_callback() {
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::new(1, Arc::clone(&live));
        let clean = [0.0_f32; 16];
        assert!(!context.on_sample(clean.as_ptr(), clean.len(), 0, clean.len() as u32));
        let mut dirty = [0.0_f32; 16];
        dirty[1] = f32::NAN;
        dirty[2] = f32::NAN;
        dirty[3] = f32::INFINITY;
        dirty[4] = f32::NEG_INFINITY;
        assert!(context.on_sample(dirty.as_ptr(), dirty.len(), 0, dirty.len() as u32));
        let snapshot = context.snapshot();
        assert_eq!(snapshot.nonfinite_samples_total, 4);
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(INTERNAL_FAULT_NONFINITE_SAMPLE)
        );
        assert_eq!(snapshot.terminal_events_total, 1);
        assert!(snapshot.safety_zero_requested);
    }

    struct RecordingHooks {
        events: Mutex<Vec<String>>,
    }

    impl RecordingHooks {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                events: Mutex::new(Vec::new()),
            })
        }

        fn recorded(&self) -> Vec<String> {
            self.events.lock().unwrap().clone()
        }
    }

    impl StreamCallbackHooks for RecordingHooks {
        fn on_samples(&self, mono_samples: &[f32], capture_delay_ns: u64) {
            self.events
                .lock()
                .unwrap()
                .push(format!("samples:{}:{capture_delay_ns}", mono_samples.len()));
        }

        fn on_terminal_latch(&self, kind: u32) {
            self.events.lock().unwrap().push(format!("terminal:{kind}"));
        }
    }

    fn invoke_sample_trampoline(
        context: &Arc<CallbackContext>,
        samples: &[f32],
        capture_delay_ns: u64,
    ) {
        unsafe {
            trampoline_sample(
                Arc::as_ptr(context).cast::<c_void>().cast_mut(),
                samples.as_ptr(),
                samples.len(),
                capture_delay_ns,
                samples.len() as u32,
            );
        }
    }

    fn invoke_event_trampoline(context: &Arc<CallbackContext>, severity: u32, kind: u32) {
        unsafe {
            trampoline_event(
                Arc::as_ptr(context).cast::<c_void>().cast_mut(),
                severity,
                kind,
                std::ptr::null(),
                0,
            );
        }
    }

    #[test]
    fn hooks_receive_samples_until_a_terminal_fault_freezes_delivery() {
        let hooks = RecordingHooks::new();
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::with_hooks(
            1,
            Arc::clone(&live),
            Some(Arc::clone(&hooks) as Arc<dyn StreamCallbackHooks>),
        );
        let clean = [0.25_f32; 8];
        invoke_sample_trampoline(&context, &clean, 250);
        assert_eq!(hooks.recorded(), vec!["samples:8:250".to_owned()]);

        let mut dirty = [0.0_f32; 8];
        dirty[3] = f32::NAN;
        invoke_sample_trampoline(&context, &dirty, 300);
        invoke_sample_trampoline(&context, &dirty, 325);
        assert_eq!(
            hooks.recorded(),
            vec![
                "samples:8:250".to_owned(),
                format!("terminal:{INTERNAL_FAULT_NONFINITE_SAMPLE}")
            ]
        );

        invoke_sample_trampoline(&context, &clean, 350);
        invoke_event_trampoline(&context, EVENT_SEVERITY_TERMINAL, EVENT_KIND_XRUN);
        assert_eq!(
            hooks.recorded(),
            vec![
                "samples:8:250".to_owned(),
                format!("terminal:{INTERNAL_FAULT_NONFINITE_SAMPLE}")
            ],
            "delivery must stay frozen after the one-shot terminal latch"
        );
        let snapshot = context.snapshot();
        assert!(snapshot.safety_zero_requested);
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(INTERNAL_FAULT_NONFINITE_SAMPLE)
        );
    }

    #[test]
    fn hooks_never_observe_stale_generation_callbacks() {
        let hooks = RecordingHooks::new();
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::with_hooks(
            1,
            Arc::clone(&live),
            Some(Arc::clone(&hooks) as Arc<dyn StreamCallbackHooks>),
        );
        live.store(2, Ordering::Release);
        let samples = [0.5_f32; 4];
        invoke_sample_trampoline(&context, &samples, 10);
        invoke_event_trampoline(&context, EVENT_SEVERITY_TERMINAL, EVENT_KIND_XRUN);
        assert!(
            hooks.recorded().is_empty(),
            "stale-generation callbacks must be fenced before the application sink"
        );
        let snapshot = context.snapshot();
        assert_eq!(
            snapshot.stale_callbacks_total, 2,
            "both the stale sample and the stale event must be fenced before hooks"
        );
        assert_eq!(snapshot.latched_terminal_kind, None);
        assert!(!snapshot.safety_zero_requested);
    }

    #[test]
    fn null_sample_payload_fails_closed_without_reaching_the_application_sink() {
        let hooks = RecordingHooks::new();
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::with_hooks(
            1,
            Arc::clone(&live),
            Some(Arc::clone(&hooks) as Arc<dyn StreamCallbackHooks>),
        );
        unsafe {
            trampoline_sample(
                Arc::as_ptr(&context).cast::<c_void>().cast_mut(),
                std::ptr::null(),
                8,
                0,
                8,
            );
        }
        assert!(hooks.recorded().is_empty());
        let snapshot = context.snapshot();
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(INTERNAL_FAULT_FRAME_MISMATCH)
        );
        assert!(snapshot.safety_zero_requested);
    }

    #[test]
    fn terminal_bridge_events_notify_the_hook_exactly_once_with_effective_kinds() {
        let hooks = RecordingHooks::new();
        let live = Arc::new(AtomicU64::new(1));
        let context = CallbackContext::with_hooks(
            1,
            Arc::clone(&live),
            Some(Arc::clone(&hooks) as Arc<dyn StreamCallbackHooks>),
        );
        invoke_event_trampoline(&context, EVENT_SEVERITY_TERMINAL, 0);
        invoke_event_trampoline(&context, EVENT_SEVERITY_TERMINAL, 5);
        invoke_event_trampoline(&context, EVENT_SEVERITY_WARNING, EVENT_KIND_XRUN);
        assert_eq!(
            hooks.recorded(),
            vec![format!("terminal:{INTERNAL_FAULT_MALFORMED_EVENT}")],
            "kind zero must map to the malformed-event internal fault and later terminals stay silent"
        );
    }

    #[test]
    fn bridge_strings_are_released_exactly_once_through_the_same_dll_sink() {
        let sink = CountingSink::new();
        let result_text = "{\"ok\":true}";
        let error_text = "{\"schemaVersion\":2,\"kind\":\"error\",\"status\":5,\"code\":\"backend\",\"message\":\"boom\"}";

        let settled = settle_bridge_strings(
            &sink,
            "driver enumeration",
            0,
            owned_string(result_text),
            BridgeString::EMPTY,
        )
        .unwrap();
        assert_eq!(settled, result_text);
        assert_eq!(sink.released().len(), 1);

        let sink = CountingSink::new();
        let failure = settle_bridge_strings(
            &sink,
            "stop",
            5,
            BridgeString::EMPTY,
            owned_string(error_text),
        )
        .unwrap_err();
        assert_eq!(
            failure.status,
            BridgeStatusOrUnknown::Known(BridgeStatus::BackendError)
        );
        assert_eq!(
            failure
                .error_payload
                .as_ref()
                .map(|payload| payload.code.as_str()),
            Some("backend")
        );
        let releases = sink.released();
        assert_eq!(releases.len(), 1);

        let sink = CountingSink::new();
        let invalid_utf8 = settle_bridge_strings(
            &sink,
            "telemetry",
            0,
            owned_bytes(&[0xFF, 0xFE, 0x00]),
            BridgeString::EMPTY,
        );
        assert!(invalid_utf8.is_err());
        assert_eq!(sink.released().len(), 1);

        let sink = CountingSink::new();
        settle_bridge_strings(&sink, "close", 6, BridgeString::EMPTY, BridgeString::EMPTY)
            .unwrap_err();
        assert!(sink.released().is_empty());
    }

    #[test]
    fn rejected_error_payloads_stay_failed_and_preserve_bounded_context() {
        let duplicated = owned_string(
            r#"{"schemaVersion":2,"kind":"error","status":5,"code":"backend","message":"boom","code":"again"}"#,
        );
        let failure = settle_bridge_strings(
            &CountingSink::new(),
            "capability query",
            5,
            BridgeString::EMPTY,
            duplicated,
        )
        .unwrap_err();
        assert_eq!(
            failure.status,
            BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
            "a rejected payload must never upgrade the call outcome"
        );
        assert!(failure.error_payload.is_none());
        assert!(failure.detail.contains("error payload was rejected"));
        assert!(failure.detail.contains("duplicate JSON object keys"));

        let malformed = owned_string("{\"kind\":\"error\",");
        let failure = settle_bridge_strings(
            &CountingSink::new(),
            "stop",
            3,
            BridgeString::EMPTY,
            malformed,
        )
        .unwrap_err();
        assert_eq!(
            failure.status,
            BridgeStatusOrUnknown::Known(BridgeStatus::DriverNotFound)
        );
        assert!(failure.error_payload.is_none());
        assert!(failure.detail.contains("error payload was rejected"));
        assert!(failure.detail.contains("not strict ABI v2 JSON"));

        let future_schema = owned_string(
            r#"{"schemaVersion":99,"kind":"error","status":6,"code":"terminal","message":"x"}"#,
        );
        let failure = settle_bridge_strings(
            &CountingSink::new(),
            "telemetry",
            6,
            BridgeString::EMPTY,
            future_schema,
        )
        .unwrap_err();
        assert!(failure.detail.contains("future version"));

        let unknown_status = settle_bridge_strings(
            &CountingSink::new(),
            "probe",
            97,
            BridgeString::EMPTY,
            owned_string("{not json"),
        )
        .unwrap_err();
        assert_eq!(unknown_status.status, BridgeStatusOrUnknown::Unknown(97));
        assert!(unknown_status.detail.contains("unrecognized status 97"));
        assert!(unknown_status.detail.contains("error payload was rejected"));

        // A success status with an empty result still fails, and the rejected
        // payload context travels with that visible failure too.
        let empty_success = settle_bridge_strings(
            &CountingSink::new(),
            "driver enumeration",
            0,
            BridgeString::EMPTY,
            owned_string(
                r#"{"schemaVersion":2,"kind":"error","status":1,"code":"","message":"m"}"#,
            ),
        )
        .unwrap_err();
        assert_eq!(
            empty_success.status,
            BridgeStatusOrUnknown::Known(BridgeStatus::BackendError)
        );
        assert!(empty_success.detail.contains("empty success response"));
        assert!(empty_success.detail.contains("must not be empty"));

        // Hostile payloads cannot balloon the visible failure: the context is
        // truncated on a char boundary.
        let hostile_kind = format!(
            r#"{{"schemaVersion":2,"kind":"{}","status":5,"code":"backend","message":"boom"}}"#,
            "x".repeat(10_000)
        );
        let failure = settle_bridge_strings(
            &CountingSink::new(),
            "capability query",
            5,
            BridgeString::EMPTY,
            owned_string(&hostile_kind),
        )
        .unwrap_err();
        assert_eq!(failure.error_payload, None);
        assert!(failure.detail.contains('…'), "truncation must be marked");
        // Both the echoed bridge text and the rejection context are bounded
        // independently, so the total stays under twice the cap plus framing.
        assert!(
            failure.detail.chars().count() <= MAX_DIAGNOSTIC_TEXT_LEN * 2 + 200,
            "diagnostic context must stay bounded, got {} chars",
            failure.detail.chars().count()
        );

        // A valid payload attaches exactly as before, without rejection text.
        let valid = settle_bridge_strings(
            &CountingSink::new(),
            "stop",
            5,
            BridgeString::EMPTY,
            owned_string(
                r#"{"schemaVersion":2,"kind":"error","status":5,"code":"backend","message":"boom"}"#,
            ),
        )
        .unwrap_err();
        assert_eq!(
            valid
                .error_payload
                .as_ref()
                .map(|payload| payload.code.as_str()),
            Some("backend")
        );
        assert!(!valid.detail.contains("rejected"));
    }

    struct FakeTransport {
        calls: Mutex<Vec<&'static str>>,
        start_results: Mutex<VecDeque<Result<RawStart, BridgeCallFailure>>>,
        stop_results: Mutex<VecDeque<Result<String, BridgeCallFailure>>>,
        telemetry_results: Mutex<VecDeque<Result<String, BridgeCallFailure>>>,
        close_results: Mutex<VecDeque<Result<String, BridgeCallFailure>>>,
        close_slots_seen: Mutex<Vec<usize>>,
    }

    impl FakeTransport {
        fn new() -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                start_results: Mutex::new(VecDeque::new()),
                stop_results: Mutex::new(VecDeque::new()),
                telemetry_results: Mutex::new(VecDeque::new()),
                close_results: Mutex::new(VecDeque::new()),
                close_slots_seen: Mutex::new(Vec::new()),
            }
        }

        fn push_start_success(&self, handle_index: usize, result_json: String) {
            self.start_results.lock().unwrap().push_back(Ok(RawStart {
                handle: RawStreamHandle(handle_index as *mut c_void),
                result_json,
            }));
        }

        fn record_call(&self, name: &'static str) {
            self.calls.lock().unwrap().push(name);
        }

        fn call_count(&self, name: &str) -> usize {
            self.calls
                .lock()
                .unwrap()
                .iter()
                .filter(|call| **call == name)
                .count()
        }
    }

    impl BridgeTransport for FakeTransport {
        fn start(
            &self,
            request_json: &[u8],
            _sample: SampleCallbackFn,
            _event: EventCallbackFn,
            _context: *mut c_void,
        ) -> Result<RawStart, BridgeCallFailure> {
            self.record_call("start");
            assert!(!request_json.is_empty());
            self.start_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(BridgeCallFailure {
                        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                        error_payload: None,
                        detail: "fake transport was not scripted for start".to_owned(),
                    })
                })
        }

        fn stop(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure> {
            self.record_call("stop");
            assert!(!handle.is_null());
            self.stop_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(BridgeCallFailure {
                        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                        error_payload: None,
                        detail: "fake transport was not scripted for stop".to_owned(),
                    })
                })
        }

        fn telemetry_json(&self, handle: RawStreamHandle) -> Result<String, BridgeCallFailure> {
            self.record_call("telemetry");
            assert!(!handle.is_null());
            self.telemetry_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(BridgeCallFailure {
                        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                        error_payload: None,
                        detail: "fake transport was not scripted for telemetry".to_owned(),
                    })
                })
        }

        fn close(&self, slot: &mut RawCloseSlot) -> Result<String, BridgeCallFailure> {
            self.record_call("close");
            self.close_slots_seen.lock().unwrap().push(slot.0 as usize);
            slot.0 = std::ptr::null_mut();
            self.close_results
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    Err(BridgeCallFailure {
                        status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                        error_payload: None,
                        detail: "fake transport was not scripted for close".to_owned(),
                    })
                })
        }
    }

    fn fake_start_result(actual_buffer_frames: u32) -> String {
        serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "start",
            "actualBufferFrames": actual_buffer_frames
        }))
        .unwrap()
    }

    fn scripted_session(transport: &Arc<FakeTransport>) -> AsioStreamSession<FakeTransport> {
        publish_generation_and_start(
            Arc::clone(transport),
            &persisted_selection().start_request().unwrap(),
            Arc::new(AtomicU64::new(5)),
            None,
        )
        .unwrap()
    }

    #[test]
    fn start_publishes_a_new_generation_and_records_the_applied_buffer() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(0xDEAD, fake_start_result(128));
        let session = scripted_session(&transport);
        assert_eq!(session.generation(), 6);
        assert_eq!(session.actual_buffer_frames(), 128);
        assert_eq!(session.start_report().actual_buffer_frames, 128);
        assert!(!session.is_stopped());
        drop(session);
    }

    #[test]
    fn zero_actual_buffer_stops_and_closes_exactly_once() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(0xBEEF, fake_start_result(0));
        transport
            .stop_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "stop",
                "stopped": true,
                "streamWasActive": true
            }))
            .unwrap()));
        transport
            .close_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "close",
                "handleReleased": true,
                "streamWasActive": true
            }))
            .unwrap()));
        let outcome = publish_generation_and_start(
            Arc::clone(&transport),
            &persisted_selection().start_request().unwrap(),
            Arc::new(AtomicU64::new(0)),
            None,
        );
        let failure_text = match &outcome {
            Err(SessionStartError::ZeroActualBufferFrames { cleanup }) => {
                assert!(cleanup.is_ok());
                outcome
                    .as_ref()
                    .err()
                    .expect("the matched outcome must contain an error")
                    .to_string()
            }
            Err(_) => panic!("expected ZeroActualBufferFrames"),
            Ok(_) => panic!("expected a rejected start result"),
        };
        assert!(failure_text.contains("actualBufferFrames=0"));
        assert!(failure_text.contains("stopped and closed exactly once"));
        assert_eq!(transport.call_count("start"), 1);
        assert_eq!(transport.call_count("stop"), 1);
        assert_eq!(transport.call_count("close"), 1);
        assert_eq!(
            transport.close_slots_seen.lock().unwrap().as_slice(),
            &[0xBEEF]
        );
    }

    #[test]
    fn malformed_start_result_stops_and_closes_exactly_once_before_context_drop() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(0xCAFE, "{\"schemaVersion\":2,\"kind\":\"start\"".to_owned());
        transport
            .stop_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "stop",
                "stopped": true,
                "streamWasActive": true
            }))
            .unwrap()));
        transport
            .close_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "close",
                "handleReleased": true,
                "streamWasActive": true
            }))
            .unwrap()));
        let outcome = publish_generation_and_start(
            Arc::clone(&transport),
            &persisted_selection().start_request().unwrap(),
            Arc::new(AtomicU64::new(0)),
            None,
        );
        match outcome {
            Err(SessionStartError::ResultPayload { cleanup, .. }) => {
                assert_eq!(cleanup, Ok(()));
            }
            Err(other) => panic!("expected ResultPayload, got {other:?}"),
            Ok(_) => panic!("expected ResultPayload but a live session was created"),
        }
        assert_eq!(transport.call_count("start"), 1);
        assert_eq!(transport.call_count("stop"), 1);
        assert_eq!(transport.call_count("close"), 1);
        assert_eq!(
            transport.close_slots_seen.lock().unwrap().as_slice(),
            &[0xCAFE],
            "the opened stream must be torn down through the same raw handle"
        );
    }

    #[test]
    fn failed_stop_is_visible_while_the_opened_stream_still_closes_exactly_once() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(0xF00D, fake_start_result(0));
        transport
            .stop_results
            .lock()
            .unwrap()
            .push_back(Err(BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(BridgeStatus::BackendError),
                error_payload: None,
                detail: "stop exploded during start cleanup".to_owned(),
            }));
        let outcome = publish_generation_and_start(
            Arc::clone(&transport),
            &persisted_selection().start_request().unwrap(),
            Arc::new(AtomicU64::new(0)),
            None,
        );
        match &outcome {
            Err(SessionStartError::ZeroActualBufferFrames { cleanup }) => {
                let failure = cleanup
                    .as_ref()
                    .expect_err("the scripted Stop failure must be visible");
                assert!(failure
                    .detail
                    .contains("stop exploded during start cleanup"));
                assert!(outcome
                    .as_ref()
                    .err()
                    .expect("the matched outcome must contain an error")
                    .to_string()
                    .contains("teardown reported"));
            }
            Err(_) => panic!("expected ZeroActualBufferFrames"),
            Ok(_) => panic!("expected a rejected start result"),
        }
        assert_eq!(transport.call_count("stop"), 1);
        assert_eq!(transport.call_count("close"), 1);
    }

    #[test]
    fn generation_overflow_rejects_without_publishing_or_identity_reuse() {
        let transport = Arc::new(FakeTransport::new());
        let live_generation = Arc::new(AtomicU64::new(u64::MAX));
        let outcome = publish_generation_and_start(
            Arc::clone(&transport),
            &persisted_selection().start_request().unwrap(),
            Arc::clone(&live_generation),
            None,
        );
        assert!(
            matches!(outcome, Err(SessionStartError::GenerationExhausted)),
            "a saturated counter must reject instead of wrapping to identity 0"
        );
        assert_eq!(
            transport.call_count("start"),
            0,
            "overflow must reject before the native stream is opened"
        );
        assert_eq!(
            transport.call_count("stop") + transport.call_count("close"),
            0
        );
        assert_eq!(
            live_generation.load(Ordering::Acquire),
            u64::MAX,
            "identity 0 must never be published by an overflow"
        );
    }

    #[test]
    fn stop_is_single_shot_and_second_stop_never_reaches_the_dll() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(1, fake_start_result(128));
        transport
            .stop_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "stop",
                "stopped": true,
                "streamWasActive": true
            }))
            .unwrap()));
        let mut session = scripted_session(&transport);
        let report = session.stop().unwrap();
        assert!(report.stopped);
        assert!(report.stream_was_active);
        assert!(session.is_stopped());
        let second = session.stop();
        assert_eq!(second, Err(SessionStopError::AlreadyStopped));
        assert_eq!(transport.call_count("stop"), 1);
        drop(session);
    }

    #[test]
    fn telemetry_roundtrip_uses_strict_payload_parsing() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(2, fake_start_result(256));
        transport
            .telemetry_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "telemetry",
                "callbacks": 7,
                "xruns": 0,
                "callbackDurationNs": {"p50": 10, "p95": 20, "p99": 30, "max": 40},
                "captureDelayNs": {"p50": 1, "p95": 2, "p99": 3, "max": 4}
            }))
            .unwrap()));
        let session = scripted_session(&transport);
        let telemetry = session.telemetry_json().unwrap();
        assert_eq!(telemetry.callbacks, 7);
        assert_eq!(telemetry.xruns, 0);
        assert_eq!(telemetry.capture_delay_ns.max, 4);
        drop(session);
    }

    #[test]
    fn close_success_consumes_the_session_without_drop_side_effects() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(3, fake_start_result(128));
        transport
            .close_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "close",
                "handleReleased": true,
                "streamWasActive": true
            }))
            .unwrap()));
        let session = scripted_session(&transport);
        let generation = session.generation();
        let outcome = session.close().unwrap();
        assert!(outcome.handle_released());
        assert!(outcome.stream_was_active());
        assert_eq!(outcome.generation(), generation);
        assert_eq!(
            outcome.context_snapshot().latched_terminal_kind,
            None,
            "an orderly close must not latch a fault"
        );
        assert_eq!(transport.call_count("close"), 1);
        assert_eq!(transport.close_slots_seen.lock().unwrap().as_slice(), &[3]);
    }

    #[test]
    fn close_failure_remains_visible_and_the_handle_is_consumed_anyway() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(4, fake_start_result(128));
        transport
            .close_results
            .lock()
            .unwrap()
            .push_back(Err(BridgeCallFailure {
                status: BridgeStatusOrUnknown::Known(BridgeStatus::Terminal),
                error_payload: None,
                detail: "teardown exploded".to_owned(),
            }));
        let session = scripted_session(&transport);
        let consumed = session.close().unwrap_err();
        let display = consumed.to_string();
        assert!(display.contains("handle was consumed anyway"));
        assert!(display.contains("teardown exploded"));
        assert_eq!(transport.call_count("close"), 1);
        assert_eq!(transport.close_slots_seen.lock().unwrap().as_slice(), &[4]);
    }

    #[test]
    fn close_with_malformed_result_is_reported_while_consuming_the_handle() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(5, fake_start_result(128));
        transport
            .close_results
            .lock()
            .unwrap()
            .push_back(Ok(serde_json::to_string(&serde_json::json!({
                "schemaVersion": 2,
                "kind": "stop",
                "handleReleased": true,
                "streamWasActive": true
            }))
            .unwrap()));
        let session = scripted_session(&transport);
        let consumed = session.close().unwrap_err();
        match &consumed.cause {
            CloseConsumedCause::ResultPayload(PayloadReject::KindMismatch { expected, found }) => {
                assert_eq!(*expected, "close");
                assert_eq!(found, "stop");
            }
            other => panic!("expected ResultPayload rejection, got {other:?}"),
        }
    }

    #[test]
    fn dropping_an_unclosed_stream_latches_a_visible_terminal_fault() {
        let transport = Arc::new(FakeTransport::new());
        transport.push_start_success(6, fake_start_result(128));
        let session = scripted_session(&transport);
        let context = Arc::clone(session.context());
        drop(session);
        let snapshot = context.snapshot();
        assert!(snapshot.safety_zero_requested);
        assert_eq!(
            snapshot.latched_terminal_kind,
            Some(INTERNAL_FAULT_DROP_WITHOUT_CLOSE)
        );
        assert_eq!(transport.call_count("close"), 0);
    }

    #[test]
    fn capabilities_request_serialization_is_strict() {
        let request = CapabilitiesRequestJson::new(DRIVER_A).unwrap();
        let bytes = request.serialize_wire_bytes().unwrap();
        let text = std::str::from_utf8(&bytes).unwrap();
        assert!(text.contains("\"schemaVersion\":2"));
        assert!(text.contains("\"driverId\""));

        assert!(CapabilitiesRequestJson::new("TOPPING").is_err());
        assert!(CapabilitiesRequestJson::new("asio:").is_err());
        assert!(CapabilitiesRequestJson::new(" asio:Topping").is_err());
    }

    #[test]
    fn driver_ids_are_bounded_in_utf8_bytes_and_free_of_control_characters() {
        let max_name = "a".repeat(MAX_DRIVER_ID_BYTES - "asio:".len());
        let max_id = format!("asio:{max_name}");
        assert_eq!(max_id.len(), MAX_DRIVER_ID_BYTES);
        assert!(valid_persistent_driver_id(&max_id));

        let oversize = format!("asio:{max_name}a");
        assert_eq!(oversize.len(), MAX_DRIVER_ID_BYTES + 1);
        assert!(!valid_persistent_driver_id(&oversize));
        assert!(CapabilitiesRequestJson::new(&oversize)
            .unwrap_err()
            .contains("UTF-8 bytes"));

        // The budget counts UTF-8 bytes, not characters: multi-byte scalars
        // consume it by their encoded width.
        let multibyte_name = "é".repeat(253) + "a";
        assert_eq!(multibyte_name.len(), 507);
        let multibyte_max = format!("asio:{multibyte_name}");
        assert_eq!(multibyte_max.len(), MAX_DRIVER_ID_BYTES);
        assert!(valid_persistent_driver_id(&multibyte_max));
        assert!(!valid_persistent_driver_id(&format!(
            "asio:{multibyte_name}a"
        )));

        for hostile in [
            "asio:a\u{0000}b",
            "asio:a\u{0001}b",
            "asio:a\u{0007}b",
            "asio:a\u{001F}",
            "asio:a\u{007F}",
            "asio:a\tb",
            "asio:a\nb",
            "asio:a\u{000B}b",
        ] {
            assert!(
                !valid_persistent_driver_id(hostile),
                "control-hostile id {hostile:?} must be rejected"
            );
        }
        // Interior non-control Unicode and punctuation stay acceptable.
        assert!(valid_persistent_driver_id("asio:Töpping ✓ (USB)"));
        assert!(valid_persistent_driver_id("asio:a\u{00E9}b"));

        let catalog_entry_with_oversize_id = serde_json::to_string(&serde_json::json!({
            "schemaVersion": 2,
            "kind": "drivers",
            "abiVersion": 2,
            "backend": "asio",
            "built": true,
            "drivers": [{
                "id": oversize,
                "name": "T",
                "inputChannels": 2,
                "sampleFormats": ["i32"],
                "sampleRatesHz": [48000],
                "bufferFrames": {"min": 8, "max": 64}
            }]
        }))
        .unwrap();
        assert!(matches!(
            parse_strict_payload::<DriverCatalogJson>(&catalog_entry_with_oversize_id),
            Err(PayloadReject::ContractViolation(message))
                if message.contains("UTF-8 bytes") && message.contains("persistent asio:")
        ));
    }

    #[test]
    fn capabilities_request_wire_payload_stays_within_its_byte_cap() {
        let max_id = format!("asio:{}", "a".repeat(MAX_DRIVER_ID_BYTES - "asio:".len()));
        let bytes = CapabilitiesRequestJson::new(&max_id)
            .unwrap()
            .serialize_wire_bytes()
            .unwrap();
        // Pins the documented envelope arithmetic: 33 fixed JSON bytes plus
        // the capped identity, comfortably under the wire limit.
        assert_eq!(bytes.len(), 33 + MAX_DRIVER_ID_BYTES);
        assert!(bytes.len() <= MAX_CAPABILITIES_REQUEST_WIRE_BYTES);
    }

    static NEXT_TEMP_DIR: AtomicUsize = AtomicUsize::new(0);
}
