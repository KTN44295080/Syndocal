//! Exact dynamic-loader and owned lifecycle for the ASIO v3 output ABI.
//!
//! It does not fall back to v2 or WASAPI. A `BridgeV3Session` keeps the DLL
//! alive and must Stop/Close before its application callback context is freed.

#![cfg(all(target_os = "windows", target_arch = "x86_64", feature = "asio"))]

use crate::asio_program_cue::MachineAsioOutputProfile;
use libloading::Library;
use serde::{Deserialize, Serialize};
use std::{ffi::c_void, fmt, path::Path, sync::Arc};

const BUILD_FLAG_ASIO_COMPILED: u32 = 0x1;
const MAX_BRIDGE_JSON_BYTES: usize = 65_536;
pub(crate) const REQUIRED_V3_ABI_VERSION: u32 = 3;
pub(crate) const REQUIRED_V3_SYMBOLS: [&str; 9] = [
    "syndocal_asio_v3_abi_version",
    "syndocal_asio_v3_build_flags",
    "syndocal_asio_v3_drivers_json",
    "syndocal_asio_v3_capabilities_json",
    "syndocal_asio_v3_string_free",
    "syndocal_asio_v3_start",
    "syndocal_asio_v3_stop",
    "syndocal_asio_v3_close",
    "syndocal_asio_v3_telemetry_json",
];

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(crate) struct BridgeStringV3 {
    ptr: *mut u8,
    len: usize,
}
impl BridgeStringV3 {
    fn empty(self) -> bool {
        self.ptr.is_null() && self.len == 0
    }
}

pub(crate) type OutputCallbackV3 =
    unsafe extern "C" fn(*mut c_void, *mut f32, u32, u32, u64, u64, u64) -> u32;
pub(crate) type DuplexCallbackV3 =
    unsafe extern "C" fn(*mut c_void, *const f32, u32, *mut f32, u32, u32, u64, u64, u64) -> u32;
pub(crate) type EventCallbackV3 = unsafe extern "C" fn(*mut c_void, u32, u32, *const u8, usize);
type AbiVersionFn = unsafe extern "C" fn() -> u32;
type BuildFlagsFn = unsafe extern "C" fn() -> u32;
type DriversJsonFn = unsafe extern "C" fn(*mut BridgeStringV3, *mut BridgeStringV3) -> u32;
type CapabilitiesJsonFn =
    unsafe extern "C" fn(*const u8, usize, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;
type StringFreeFn = unsafe extern "C" fn(BridgeStringV3);
type StartFn = unsafe extern "C" fn(
    *const u8,
    usize,
    Option<OutputCallbackV3>,
    Option<DuplexCallbackV3>,
    Option<EventCallbackV3>,
    *mut c_void,
    *mut *mut c_void,
    *mut BridgeStringV3,
    *mut BridgeStringV3,
) -> u32;
type StopFn = unsafe extern "C" fn(*mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;
type CloseFn =
    unsafe extern "C" fn(*mut *mut c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;
type TelemetryJsonFn =
    unsafe extern "C" fn(*const c_void, *mut BridgeStringV3, *mut BridgeStringV3) -> u32;

#[derive(Clone, Copy)]
struct Table {
    abi: AbiVersionFn,
    flags: BuildFlagsFn,
    drivers: DriversJsonFn,
    capabilities: CapabilitiesJsonFn,
    free: StringFreeFn,
    start: StartFn,
    stop: StopFn,
    close: CloseFn,
    telemetry: TelemetryJsonFn,
}
struct Inner {
    // `None` is used only by the in-module deterministic tests below. The
    // production loader always stores the loaded library here so every
    // session keeps its function table alive until the opaque handle closes.
    _library: Option<Library>,
    table: Table,
}
#[derive(Clone)]
pub(crate) struct BridgeV3Module {
    inner: Arc<Inner>,
}
impl fmt::Debug for BridgeV3Module {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BridgeV3Module")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BridgeV3Fault {
    Load {
        path: String,
        detail: String,
    },
    MissingSymbol(&'static str),
    AbiMismatch {
        expected: u32,
        actual: u32,
    },
    NotAsioCompiled,
    InvalidRequest(String),
    Call {
        operation: &'static str,
        status: u32,
        detail: String,
    },
    InvalidResponse(String),
    AlreadyClosed,
}
impl fmt::Display for BridgeV3Fault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Load { path, detail } => {
                write!(f, "failed to load ASIO v3 bridge {path}: {detail}")
            }
            Self::MissingSymbol(s) => write!(f, "ASIO v3 bridge is missing {s}"),
            Self::AbiMismatch { expected, actual } => {
                write!(f, "ASIO v3 ABI mismatch: expected {expected}, got {actual}")
            }
            Self::NotAsioCompiled => f.write_str("ASIO v3 bridge is not ASIO compiled"),
            Self::InvalidRequest(s) => write!(f, "invalid ASIO v3 request: {s}"),
            Self::Call {
                operation,
                status,
                detail,
            } => write!(f, "ASIO v3 {operation} failed ({status}): {detail}"),
            Self::InvalidResponse(s) => write!(f, "invalid ASIO v3 response: {s}"),
            Self::AlreadyClosed => f.write_str("ASIO v3 session is already closed"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutputStartRequestV3 {
    schema_version: u32,
    driver_id: String,
    mode: &'static str,
    output: StreamTupleV3,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StreamTupleV3 {
    channels: u32,
    sample_format: String,
    sample_rate_hz: u32,
    fixed_buffer_frames: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartResponse {
    schema_version: u32,
    kind: String,
    actual_output: StreamTupleV3,
    actual_input: Option<StreamTupleV3>,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BridgeV3SessionPhase {
    Started,
    StoppedPendingClose,
    Closed,
}

impl OutputStartRequestV3 {
    pub(crate) fn from_profile(
        profile: &MachineAsioOutputProfile,
        first_output_frame: u64,
        session_generation: u64,
    ) -> Result<Self, BridgeV3Fault> {
        profile
            .validate()
            .map_err(|e| BridgeV3Fault::InvalidRequest(e.to_string()))?;
        let value = Self {
            schema_version: 3,
            driver_id: profile.driver_id().to_owned(),
            mode: "outputOnly",
            output: StreamTupleV3 {
                channels: profile.device_output_channels(),
                sample_format: profile.native_format().to_owned(),
                sample_rate_hz: profile.sample_rate_hz(),
                fixed_buffer_frames: profile.fixed_buffer_frames(),
            },
            first_output_frame,
            session_generation,
            render_generation: session_generation,
        };
        value.validate()?;
        Ok(value)
    }
    fn validate(&self) -> Result<(), BridgeV3Fault> {
        if self.schema_version != 3
            || self.mode != "outputOnly"
            || self.driver_id.trim() != self.driver_id
            || !self.driver_id.starts_with("asio:")
            || self.driver_id.len() <= 5
            || self.output.channels == 0
            || self.output.sample_rate_hz != 48_000
            || self.output.fixed_buffer_frames == 0
            || self.session_generation == 0
            || self.render_generation != self.session_generation
            || !matches!(
                self.output.sample_format.as_str(),
                "f32" | "i16" | "i24" | "i32" | "f64"
            )
        {
            return Err(BridgeV3Fault::InvalidRequest("output-only v3 request is not an exact 48 kHz tuple with immutable session root generation".to_owned()));
        }
        Ok(())
    }
}

pub(crate) struct BridgeV3Session {
    inner: Arc<Inner>,
    // The FFI handle is used only under the owning runtime mutex. Keeping an
    // address (rather than a raw pointer) lets `AppState` remain Send + Sync.
    raw: usize,
    phase: BridgeV3SessionPhase,
}

impl fmt::Debug for BridgeV3Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BridgeV3Session")
            .field("raw", &self.raw)
            .field("phase", &self.phase)
            .finish()
    }
}

#[derive(Debug)]
pub(crate) enum BridgeV3StartError {
    Bridge(BridgeV3Fault),
    Cleanup {
        start: BridgeV3Fault,
        cleanup: BridgeV3Fault,
        session: Box<BridgeV3Session>,
    },
}

impl fmt::Display for BridgeV3StartError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bridge(error) => error.fmt(f),
            Self::Cleanup { start, cleanup, .. } => write!(
                f,
                "{start}; bridge session cleanup failed and remains retryable: {cleanup}"
            ),
        }
    }
}

fn failed_start_with_cleanup(
    inner: &Arc<Inner>,
    raw: *mut c_void,
    start: BridgeV3Fault,
) -> BridgeV3StartError {
    let mut session = BridgeV3Session {
        inner: Arc::clone(inner),
        raw: raw as usize,
        phase: BridgeV3SessionPhase::Started,
    };
    match session.stop_and_close() {
        Ok(()) => BridgeV3StartError::Bridge(start),
        Err(cleanup) => BridgeV3StartError::Cleanup {
            start,
            cleanup,
            session: Box::new(session),
        },
    }
}

impl BridgeV3Module {
    pub(crate) fn load_at(path: &Path) -> Result<Self, BridgeV3Fault> {
        let library = unsafe { Library::new(path) }.map_err(|e| BridgeV3Fault::Load {
            path: path.display().to_string(),
            detail: e.to_string(),
        })?;
        unsafe fn symbol<T: Copy>(
            library: &Library,
            name: &'static str,
        ) -> Result<T, BridgeV3Fault> {
            let mut n = Vec::with_capacity(name.len() + 1);
            n.extend_from_slice(name.as_bytes());
            n.push(0);
            Ok(*unsafe { library.get::<T>(&n) }.map_err(|_| BridgeV3Fault::MissingSymbol(name))?)
        }
        let table = unsafe {
            Table {
                abi: symbol(&library, REQUIRED_V3_SYMBOLS[0])?,
                flags: symbol(&library, REQUIRED_V3_SYMBOLS[1])?,
                drivers: symbol(&library, REQUIRED_V3_SYMBOLS[2])?,
                capabilities: symbol(&library, REQUIRED_V3_SYMBOLS[3])?,
                free: symbol(&library, REQUIRED_V3_SYMBOLS[4])?,
                start: symbol(&library, REQUIRED_V3_SYMBOLS[5])?,
                stop: symbol(&library, REQUIRED_V3_SYMBOLS[6])?,
                close: symbol(&library, REQUIRED_V3_SYMBOLS[7])?,
                telemetry: symbol(&library, REQUIRED_V3_SYMBOLS[8])?,
            }
        };
        let actual = unsafe { (table.abi)() };
        if actual != REQUIRED_V3_ABI_VERSION {
            return Err(BridgeV3Fault::AbiMismatch {
                expected: REQUIRED_V3_ABI_VERSION,
                actual,
            });
        }
        if unsafe { (table.flags)() } & BUILD_FLAG_ASIO_COMPILED == 0 {
            return Err(BridgeV3Fault::NotAsioCompiled);
        }
        Ok(Self {
            inner: Arc::new(Inner {
                _library: Some(library),
                table,
            }),
        })
    }
    pub(crate) fn start_output(
        &self,
        request: &OutputStartRequestV3,
        callback: OutputCallbackV3,
        event: EventCallbackV3,
        context: *mut c_void,
    ) -> Result<BridgeV3Session, BridgeV3StartError> {
        request.validate().map_err(BridgeV3StartError::Bridge)?;
        if context.is_null() {
            return Err(BridgeV3StartError::Bridge(BridgeV3Fault::InvalidRequest(
                "callback context is null".to_owned(),
            )));
        }
        let bytes = serde_json::to_vec(request).map_err(|e| {
            BridgeV3StartError::Bridge(BridgeV3Fault::InvalidRequest(e.to_string()))
        })?;
        if bytes.len() > MAX_BRIDGE_JSON_BYTES {
            return Err(BridgeV3StartError::Bridge(BridgeV3Fault::InvalidRequest(
                "Start JSON exceeds bound".to_owned(),
            )));
        }
        let mut raw = std::ptr::null_mut();
        let mut result = BridgeStringV3::default();
        let mut error = BridgeStringV3::default();
        let status = unsafe {
            (self.inner.table.start)(
                bytes.as_ptr(),
                bytes.len(),
                Some(callback),
                None,
                Some(event),
                context,
                &mut raw,
                &mut result,
                &mut error,
            )
        };
        let text = match settle(&self.inner.table, "start", status, result, error) {
            Ok(text) => text,
            Err(start_error) if !raw.is_null() => {
                return Err(failed_start_with_cleanup(&self.inner, raw, start_error));
            }
            Err(start_error) => return Err(BridgeV3StartError::Bridge(start_error)),
        };
        if raw.is_null() {
            return Err(BridgeV3StartError::Bridge(BridgeV3Fault::InvalidResponse(
                "start returned null handle".to_owned(),
            )));
        }
        let response: StartResponse = match serde_json::from_str(&text) {
            Ok(response) => response,
            Err(error) => {
                return Err(failed_start_with_cleanup(
                    &self.inner,
                    raw,
                    BridgeV3Fault::InvalidResponse(error.to_string()),
                ));
            }
        };
        if response.schema_version != 3
            || response.kind != "started"
            || response.actual_input.is_some()
            || response.actual_output != request.output
            || response.first_output_frame != request.first_output_frame
            || response.session_generation != request.session_generation
            || response.render_generation != request.render_generation
        {
            return Err(failed_start_with_cleanup(
                &self.inner,
                raw,
                BridgeV3Fault::InvalidResponse(
                    "Start response did not exactly match admitted output tuple".to_owned(),
                ),
            ));
        }
        Ok(BridgeV3Session {
            inner: Arc::clone(&self.inner),
            raw: raw as usize,
            phase: BridgeV3SessionPhase::Started,
        })
    }
    pub(crate) fn driver_catalog_text(&self) -> Result<String, BridgeV3Fault> {
        self.call_json("drivers", |r, e| unsafe {
            (self.inner.table.drivers)(r, e)
        })
    }
    pub(crate) fn capabilities_text(&self, id: &str) -> Result<String, BridgeV3Fault> {
        if id.trim() != id || !id.starts_with("asio:") {
            return Err(BridgeV3Fault::InvalidRequest(
                "capabilities requires exact ASIO identity".to_owned(),
            ));
        };
        let bytes = serde_json::to_vec(&serde_json::json!({"schemaVersion":3,"driverId":id}))
            .map_err(|e| BridgeV3Fault::InvalidRequest(e.to_string()))?;
        self.call_json("capabilities", |r, e| unsafe {
            (self.inner.table.capabilities)(bytes.as_ptr(), bytes.len(), r, e)
        })
    }
    fn call_json(
        &self,
        operation: &'static str,
        call: impl FnOnce(*mut BridgeStringV3, *mut BridgeStringV3) -> u32,
    ) -> Result<String, BridgeV3Fault> {
        let mut r = BridgeStringV3::default();
        let mut e = BridgeStringV3::default();
        settle(&self.inner.table, operation, call(&mut r, &mut e), r, e)
    }
}
impl BridgeV3Session {
    pub(crate) fn phase(&self) -> BridgeV3SessionPhase {
        self.phase
    }

    pub(crate) fn is_closed(&self) -> bool {
        matches!(self.phase, BridgeV3SessionPhase::Closed) || self.raw == 0
    }

    pub(crate) fn telemetry_text(&self) -> Result<String, BridgeV3Fault> {
        if self.is_closed() {
            return Err(BridgeV3Fault::AlreadyClosed);
        };
        let mut r = BridgeStringV3::default();
        let mut e = BridgeStringV3::default();
        let status =
            unsafe { (self.inner.table.telemetry)(self.raw as *const c_void, &mut r, &mut e) };
        settle(&self.inner.table, "telemetry", status, r, e)
    }

    pub(crate) fn stop_and_close(&mut self) -> Result<(), BridgeV3Fault> {
        if self.is_closed() {
            return Err(BridgeV3Fault::AlreadyClosed);
        };

        // Stop is intentionally a one-way phase transition. If Close fails,
        // the next call must retry Close only; repeating Stop can race the
        // backend's already-drained callback/driver state.
        if matches!(self.phase, BridgeV3SessionPhase::Started) {
            let mut r = BridgeStringV3::default();
            let mut e = BridgeStringV3::default();
            let status =
                unsafe { (self.inner.table.stop)(self.raw as *mut c_void, &mut r, &mut e) };
            let _ = settle(&self.inner.table, "stop", status, r, e)?;
            self.phase = BridgeV3SessionPhase::StoppedPendingClose;
        }

        let mut r = BridgeStringV3::default();
        let mut e = BridgeStringV3::default();
        let mut raw = self.raw as *mut c_void;
        let status = unsafe { (self.inner.table.close)(&mut raw, &mut r, &mut e) };
        let close_result = settle(&self.inner.table, "close", status, r, e);
        if raw.is_null() {
            // A null handle is authoritative even if the bridge returned a
            // malformed diagnostic. Never retain a pointer that the bridge
            // has already consumed; report the diagnostic but make retries
            // impossible because the session is in fact closed.
            self.raw = 0;
            self.phase = BridgeV3SessionPhase::Closed;
            return close_result.map(|_| ());
        }
        match close_result {
            Ok(_) => {
                // Success with a retained in/out handle violates the v3 ABI and
                // leaves ownership ambiguous: the bridge may already have
                // destroyed the native object without clearing the caller's
                // stale address. Never retry or dereference that address. Stop
                // has already drained callbacks, so terminally quarantine the
                // handle value and report the contract failure. This may leak a
                // broken backend object, but cannot double-Close or use it after
                // free.
                self.raw = 0;
                self.phase = BridgeV3SessionPhase::Closed;
                Err(BridgeV3Fault::InvalidResponse(
                    "close reported success without clearing handle; stale handle was quarantined and will not be retried"
                        .to_owned(),
                ))
            }
            Err(error) => Err(error),
        }
    }
}
fn text(value: BridgeStringV3) -> Result<String, BridgeV3Fault> {
    if value.empty() {
        return Ok(String::new());
    }
    if value.ptr.is_null() || value.len > MAX_BRIDGE_JSON_BYTES {
        return Err(BridgeV3Fault::InvalidResponse(
            "invalid bridge string range".to_owned(),
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(value.ptr.cast_const(), value.len) };
    String::from_utf8(bytes.to_vec()).map_err(|e| BridgeV3Fault::InvalidResponse(e.to_string()))
}
fn settle(
    table: &Table,
    operation: &'static str,
    status: u32,
    result: BridgeStringV3,
    error: BridgeStringV3,
) -> Result<String, BridgeV3Fault> {
    let result_text = text(result);
    let error_text = text(error);
    if !result.empty() {
        unsafe { (table.free)(result) }
    }
    if !error.empty() {
        unsafe { (table.free)(error) }
    }
    let result_text = result_text?;
    let error_text = error_text?;
    if status != 0 {
        return Err(BridgeV3Fault::Call {
            operation,
            status,
            detail: if error_text.trim().is_empty() {
                "bridge returned no diagnostic".to_owned()
            } else {
                error_text.chars().take(512).collect()
            },
        });
    }
    if result_text.trim().is_empty() {
        return Err(BridgeV3Fault::InvalidResponse(format!(
            "{operation} returned empty success JSON"
        )));
    }
    Ok(result_text)
}

#[cfg(test)]
pub(crate) struct BridgeV3TestSessionControl {
    pub(crate) stop_calls: std::sync::atomic::AtomicUsize,
    pub(crate) close_calls: std::sync::atomic::AtomicUsize,
    stop_failures: std::sync::atomic::AtomicUsize,
    close_failures: std::sync::atomic::AtomicUsize,
    close_success_keeps_handle: std::sync::atomic::AtomicBool,
}

#[cfg(test)]
pub(crate) fn test_session_with_failures(
    stop_failures: usize,
    close_failures: usize,
) -> (BridgeV3Session, Arc<BridgeV3TestSessionControl>) {
    use std::sync::atomic::AtomicUsize;

    let control = Arc::new(BridgeV3TestSessionControl {
        stop_calls: AtomicUsize::new(0),
        close_calls: AtomicUsize::new(0),
        stop_failures: AtomicUsize::new(stop_failures),
        close_failures: AtomicUsize::new(close_failures),
        close_success_keeps_handle: std::sync::atomic::AtomicBool::new(false),
    });
    let native = Box::new(BridgeV3TestNativeSession {
        control: Arc::clone(&control),
    });
    let raw = Box::into_raw(native) as usize;
    let table = Table {
        abi: test_abi,
        flags: test_flags,
        drivers: test_json,
        capabilities: test_capabilities,
        free: test_string_free,
        start: test_start,
        stop: test_stop,
        close: test_close,
        telemetry: test_telemetry,
    };
    let inner = Arc::new(Inner {
        _library: None,
        table,
    });
    (
        BridgeV3Session {
            inner,
            raw,
            phase: BridgeV3SessionPhase::Started,
        },
        control,
    )
}

#[cfg(test)]
pub(crate) fn test_session_with_nonclearing_successful_close(
) -> (BridgeV3Session, Arc<BridgeV3TestSessionControl>) {
    let (session, control) = test_session_with_failures(0, 0);
    control
        .close_success_keeps_handle
        .store(true, std::sync::atomic::Ordering::Release);
    (session, control)
}

#[cfg(test)]
struct BridgeV3TestNativeSession {
    control: Arc<BridgeV3TestSessionControl>,
}

#[cfg(test)]
fn consume_test_failure(counter: &std::sync::atomic::AtomicUsize) -> bool {
    counter
        .fetch_update(
            std::sync::atomic::Ordering::AcqRel,
            std::sync::atomic::Ordering::Acquire,
            |remaining| remaining.checked_sub(1),
        )
        .is_ok()
}

#[cfg(test)]
fn test_owned_json(value: &'static [u8]) -> BridgeStringV3 {
    let bytes = value.to_vec().into_boxed_slice();
    let len = bytes.len();
    let ptr = Box::into_raw(bytes).cast::<u8>();
    BridgeStringV3 { ptr, len }
}

#[cfg(test)]
unsafe fn test_write_json(out: *mut BridgeStringV3, value: &'static [u8]) {
    if !out.is_null() {
        unsafe { out.write(test_owned_json(value)) };
    }
}

#[cfg(test)]
unsafe extern "C" fn test_abi() -> u32 {
    3
}

#[cfg(test)]
unsafe extern "C" fn test_flags() -> u32 {
    BUILD_FLAG_ASIO_COMPILED
}

#[cfg(test)]
unsafe extern "C" fn test_json(result: *mut BridgeStringV3, _error: *mut BridgeStringV3) -> u32 {
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
unsafe extern "C" fn test_capabilities(
    _request: *const u8,
    _request_len: usize,
    result: *mut BridgeStringV3,
    _error: *mut BridgeStringV3,
) -> u32 {
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
unsafe extern "C" fn test_string_free(value: BridgeStringV3) {
    if !value.empty() {
        let slice = std::ptr::slice_from_raw_parts_mut(value.ptr, value.len);
        unsafe { drop(Box::from_raw(slice)) };
    }
}

#[cfg(test)]
unsafe extern "C" fn test_start(
    _request: *const u8,
    _request_len: usize,
    _output: Option<OutputCallbackV3>,
    _duplex: Option<DuplexCallbackV3>,
    _event: Option<EventCallbackV3>,
    _context: *mut c_void,
    out_handle: *mut *mut c_void,
    result: *mut BridgeStringV3,
    _error: *mut BridgeStringV3,
) -> u32 {
    if !out_handle.is_null() {
        unsafe { out_handle.write(std::ptr::null_mut()) };
    }
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
unsafe extern "C" fn test_stop(
    handle: *mut c_void,
    result: *mut BridgeStringV3,
    error: *mut BridgeStringV3,
) -> u32 {
    if handle.is_null() {
        unsafe { test_write_json(error, br#"{"error":"null handle"}"#) };
        return 1;
    }
    let native = unsafe { &*(handle.cast::<BridgeV3TestNativeSession>()) };
    native
        .control
        .stop_calls
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    if consume_test_failure(&native.control.stop_failures) {
        unsafe { test_write_json(error, br#"{"error":"injected stop failure"}"#) };
        return 1;
    }
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
unsafe extern "C" fn test_close(
    handle: *mut *mut c_void,
    result: *mut BridgeStringV3,
    error: *mut BridgeStringV3,
) -> u32 {
    if handle.is_null() || unsafe { (*handle).is_null() } {
        unsafe { test_write_json(error, br#"{"error":"null handle"}"#) };
        return 1;
    }
    let native = unsafe { &*(*handle).cast::<BridgeV3TestNativeSession>() };
    native
        .control
        .close_calls
        .fetch_add(1, std::sync::atomic::Ordering::AcqRel);
    if consume_test_failure(&native.control.close_failures) {
        unsafe { test_write_json(error, br#"{"error":"injected close failure"}"#) };
        return 1;
    }
    if native
        .control
        .close_success_keeps_handle
        .load(std::sync::atomic::Ordering::Acquire)
    {
        unsafe { test_write_json(result, br#"{}"#) };
        return 0;
    }
    let raw = unsafe { *handle };
    unsafe { handle.write(std::ptr::null_mut()) };
    unsafe { drop(Box::from_raw(raw.cast::<BridgeV3TestNativeSession>())) };
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
unsafe extern "C" fn test_telemetry(
    _handle: *const c_void,
    result: *mut BridgeStringV3,
    _error: *mut BridgeStringV3,
) -> u32 {
    unsafe { test_write_json(result, br#"{}"#) };
    0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asio_program_cue::MachineAsioOutputProfile;
    #[test]
    fn exact_output_only_request_has_frozen_session_root_generation() {
        let p = MachineAsioOutputProfile::for_test(4, 256, 1, 2, 3);
        let r = OutputStartRequestV3::from_profile(&p, 99, 42).unwrap();
        r.validate().unwrap();
        let v = serde_json::to_value(r).unwrap();
        assert_eq!(v["mode"], "outputOnly");
        assert_eq!(v["renderGeneration"], 42);
        assert!(v.get("input").is_none());
    }
    #[test]
    fn non_48k_and_mutated_root_are_rejected() {
        let p = MachineAsioOutputProfile::for_test(4, 256, 1, 2, 3);
        let mut r = OutputStartRequestV3::from_profile(&p, 0, 1).unwrap();
        r.output.sample_rate_hz = 44_100;
        assert!(r.validate().is_err());
        r.output.sample_rate_hz = 48_000;
        r.render_generation = 2;
        assert!(r.validate().is_err());
    }

    #[test]
    fn close_failure_enters_pending_phase_and_retries_close_without_stop() {
        let (mut session, control) = test_session_with_failures(0, 1);

        let error = session.stop_and_close().unwrap_err();
        assert!(error.to_string().contains("close"));
        assert_eq!(session.phase(), BridgeV3SessionPhase::StoppedPendingClose);
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );

        session.stop_and_close().unwrap();
        assert_eq!(session.phase(), BridgeV3SessionPhase::Closed);
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
            "a pending close retry must not repeat Stop",
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );
    }

    #[test]
    fn stop_failure_remains_started_and_retries_stop_before_close() {
        let (mut session, control) = test_session_with_failures(1, 0);

        assert!(session.stop_and_close().is_err());
        assert_eq!(session.phase(), BridgeV3SessionPhase::Started);
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            0
        );

        session.stop_and_close().unwrap();
        assert_eq!(session.phase(), BridgeV3SessionPhase::Closed);
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            2
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
    }

    #[test]
    fn successful_close_that_retains_handle_is_terminally_quarantined() {
        let (mut session, control) = test_session_with_nonclearing_successful_close();

        let error = session.stop_and_close().unwrap_err();
        assert!(error.to_string().contains("stale handle was quarantined"));
        assert!(session.is_closed());
        assert_eq!(session.phase(), BridgeV3SessionPhase::Closed);
        assert_eq!(
            control
                .stop_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1
        );
        assert!(matches!(
            session.stop_and_close(),
            Err(BridgeV3Fault::AlreadyClosed)
        ));
        assert_eq!(
            control
                .close_calls
                .load(std::sync::atomic::Ordering::Acquire),
            1,
            "an ambiguous successful Close must never be retried",
        );
    }
}
