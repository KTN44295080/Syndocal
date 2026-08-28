//! Strict ASIO v3 output/full-duplex ABI contract.
//!
//! V3 is deliberately separate from ABI v2.  It describes a single device
//! width/rate/format/buffer stream, fixed callback blocks, generation fences,
//! and distinct output-only/full-duplex callback shapes.  Physical PROGRAM/CUE
//! output selection is application machine state and is therefore absent here.

use crate::v3_rt_backend::{self, SyndocalAsioV3Handle};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice, str,
};

pub(crate) const ABI_VERSION_V3: u32 = 3;
pub(crate) const JSON_SCHEMA_VERSION_V3: u32 = 3;
pub(crate) const STATUS_OK_V3: u32 = 0;
pub(crate) const STATUS_INVALID_ARGUMENT_V3: u32 = 1;
pub(crate) const STATUS_UNSUPPORTED_V3: u32 = 2;
pub(crate) const STATUS_BACKEND_ERROR_V3: u32 = 5;
pub(crate) const STATUS_TERMINAL_V3: u32 = 6;
pub(crate) const STATUS_PANIC_V3: u32 = 255;
const MAX_JSON_BYTES_V3: usize = 65_536;
const MAX_FIXED_BUFFER_FRAMES_V3: u32 = 1_048_576;
const MAX_CHANNELS_V3: u32 = u16::MAX as u32;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyndocalAsioStringV3 {
    pub ptr: *mut u8,
    pub len: usize,
}

impl Default for SyndocalAsioStringV3 {
    fn default() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }
}

/// Exact v3 callback result values, frozen with the public C header.
///
/// A non-accepted result directs the backend to write one full silent block
/// and latch a terminal fault.  Queue full/underflow are both terminal in this
/// contract; they never trigger callback-time retry, fallback, or allocation.
pub(crate) const CALLBACK_ACCEPTED_V3: u32 = 0;
pub(crate) const CALLBACK_QUEUE_UNDERFLOW_V3: u32 = 1;
pub(crate) const CALLBACK_QUEUE_FULL_V3: u32 = 2;
pub(crate) const CALLBACK_TERMINAL_V3: u32 = 3;
pub(crate) const CALLBACK_INVALID_BLOCK_V3: u32 = 4;
pub(crate) const CALLBACK_PANIC_V3: u32 = 5;

pub(crate) const EVENT_WARNING_V3: u32 = 1;
pub(crate) const EVENT_TERMINAL_V3: u32 = 2;
pub(crate) const EVENT_XRUN_V3: u32 = 1;
pub(crate) const EVENT_RESET_V3: u32 = 2;
pub(crate) const EVENT_RESYNC_V3: u32 = 3;
pub(crate) const EVENT_SAMPLE_RATE_CHANGED_V3: u32 = 4;
pub(crate) const EVENT_DEVICE_LOST_V3: u32 = 5;
pub(crate) const EVENT_CALLBACK_GAP_V3: u32 = 6;
pub(crate) const EVENT_REALTIME_DENIED_V3: u32 = 7;
pub(crate) const EVENT_BACKEND_V3: u32 = 8;
pub(crate) const EVENT_MALFORMED_CALLBACK_V3: u32 = 9;
pub(crate) const EVENT_BUFFER_SIZE_CHANGED_V3: u32 = 10;
pub(crate) const EVENT_OUTPUT_QUEUE_UNDERFLOW_V3: u32 = 11;
pub(crate) const EVENT_OUTPUT_QUEUE_FULL_V3: u32 = 12;
pub(crate) const EVENT_CALLBACK_PANIC_V3: u32 = 13;
pub(crate) const EVENT_INVALID_OUTPUT_BLOCK_V3: u32 = 14;

/// Deliberately evaluated by the v3 build-flags entry point.  The value is not
/// a capability bit; it merely keeps the FFI implementation's frozen numeric
/// contract coupled to the exported surface in non-test builds too.
fn frozen_v3_contract_code_fingerprint() -> u32 {
    [
        STATUS_UNSUPPORTED_V3,
        STATUS_TERMINAL_V3,
        CALLBACK_ACCEPTED_V3,
        CALLBACK_QUEUE_UNDERFLOW_V3,
        CALLBACK_QUEUE_FULL_V3,
        CALLBACK_TERMINAL_V3,
        CALLBACK_INVALID_BLOCK_V3,
        CALLBACK_PANIC_V3,
        EVENT_WARNING_V3,
        EVENT_TERMINAL_V3,
        EVENT_XRUN_V3,
        EVENT_RESET_V3,
        EVENT_RESYNC_V3,
        EVENT_SAMPLE_RATE_CHANGED_V3,
        EVENT_DEVICE_LOST_V3,
        EVENT_CALLBACK_GAP_V3,
        EVENT_REALTIME_DENIED_V3,
        EVENT_BACKEND_V3,
        EVENT_MALFORMED_CALLBACK_V3,
        EVENT_BUFFER_SIZE_CHANGED_V3,
        EVENT_OUTPUT_QUEUE_UNDERFLOW_V3,
        EVENT_OUTPUT_QUEUE_FULL_V3,
        EVENT_CALLBACK_PANIC_V3,
        EVENT_INVALID_OUTPUT_BLOCK_V3,
    ]
    .into_iter()
    .fold(0_u32, |fingerprint, code| {
        fingerprint.wrapping_mul(31).wrapping_add(code)
    })
}

/// Called only by the v3 realtime backend to fill one complete output block.
///
/// The bridge owns native ASIO conversion; the callback is always passed a
/// normalized f32 interleaved frame.  Return [`CALLBACK_ACCEPTED_V3`] only
/// after writing exactly `frames * output_channels` values.  Context remains
/// caller-owned until a successful Stop/Close has unpublished dispatch and
/// drained callback readers.  No callback may run after that return.
pub type SyndocalAsioOutputCallbackV3 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        interleaved_output: *mut f32,
        output_channels: u32,
        frames: u32,
        first_output_frame: u64,
        session_generation: u64,
        render_generation: u64,
    ) -> u32,
>;

/// Full-duplex equivalent.  Input/output formats are independently validated;
/// native conversion is bridge-owned and both callback buffers are normalized
/// f32.  The same accepted/silent-terminal and context lifetime rules apply.
pub type SyndocalAsioDuplexCallbackV3 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        interleaved_input: *const f32,
        input_channels: u32,
        interleaved_output: *mut f32,
        output_channels: u32,
        frames: u32,
        first_output_frame: u64,
        session_generation: u64,
        render_generation: u64,
    ) -> u32,
>;

pub type SyndocalAsioEventCallbackV3 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        severity: u32,
        kind: u32,
        message: *const u8,
        message_len: usize,
    ),
>;

#[derive(Debug)]
pub(crate) struct BridgeErrorV3 {
    pub(crate) status: u32,
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl BridgeErrorV3 {
    pub(crate) fn new(status: u32, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            status,
            code,
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new(STATUS_INVALID_ARGUMENT_V3, "invalid_argument", message)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorJsonV3<'a> {
    schema_version: u32,
    kind: &'static str,
    status: u32,
    code: &'a str,
    message: &'a str,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum StreamModeV3 {
    OutputOnly,
    FullDuplex,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct StreamTupleJsonV3 {
    pub(crate) channels: u32,
    pub(crate) sample_format: String,
    pub(crate) sample_rate_hz: u32,
    pub(crate) fixed_buffer_frames: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartRequestJsonV3 {
    schema_version: u32,
    driver_id: String,
    mode: StreamModeV3,
    output: StreamTupleJsonV3,
    #[serde(default)]
    input: Option<StreamTupleJsonV3>,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilitiesRequestJsonV3 {
    schema_version: u32,
    driver_id: String,
}

#[derive(Clone, Debug)]
pub(crate) struct StartRequestV3 {
    pub(crate) driver_id: String,
    pub(crate) mode: StreamModeV3,
    #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
    pub(crate) output: StreamTupleJsonV3,
    #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
    pub(crate) input: Option<StreamTupleJsonV3>,
    #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
    pub(crate) first_output_frame: u64,
    #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
    pub(crate) session_generation: u64,
    #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
    pub(crate) render_generation: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartResultJsonV3 {
    schema_version: u32,
    kind: &'static str,
    actual_output: StreamTupleJsonV3,
    actual_input: Option<StreamTupleJsonV3>,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StopResultJsonV3 {
    schema_version: u32,
    kind: &'static str,
    stopped: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CloseResultJsonV3 {
    schema_version: u32,
    kind: &'static str,
    handle_released: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TelemetryJsonV3 {
    schema_version: u32,
    kind: &'static str,
    callbacks: u64,
    xruns: u64,
    terminal_fault: Option<&'static str>,
}

impl StartResultJsonV3 {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn new(
        actual_output: StreamTupleJsonV3,
        actual_input: Option<StreamTupleJsonV3>,
        first_output_frame: u64,
        session_generation: u64,
        render_generation: u64,
    ) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "started",
            actual_output,
            actual_input,
            first_output_frame,
            session_generation,
            render_generation,
        }
    }
}

impl StopResultJsonV3 {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn new(stopped: bool) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "stopped",
            stopped,
        }
    }
}

impl CloseResultJsonV3 {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn new(handle_released: bool) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "closed",
            handle_released,
        }
    }
}

impl TelemetryJsonV3 {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    pub(crate) fn new(callbacks: u64, xruns: u64, terminal_fault: Option<&'static str>) -> Self {
        Self {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "telemetry",
            callbacks,
            xruns,
            terminal_fault,
        }
    }
}

#[cfg(test)]
impl StartRequestV3 {
    pub(crate) fn for_test(
        output_channels: u32,
        frames: u32,
        first_output_frame: u64,
        session_generation: u64,
        render_generation: u64,
    ) -> Self {
        let tuple = |channels| StreamTupleJsonV3 {
            channels,
            sample_format: "f32".to_owned(),
            sample_rate_hz: 48_000,
            fixed_buffer_frames: frames,
        };
        Self {
            driver_id: "asio:Test Driver".to_owned(),
            mode: StreamModeV3::OutputOnly,
            output: tuple(output_channels),
            input: None,
            first_output_frame,
            session_generation,
            render_generation,
        }
    }
}

fn owned_bytes(bytes: Vec<u8>) -> SyndocalAsioStringV3 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed).cast::<u8>();
    SyndocalAsioStringV3 { ptr, len }
}

unsafe fn require_empty_output(
    output: *mut SyndocalAsioStringV3,
    field: &str,
) -> Result<(), BridgeErrorV3> {
    if output.is_null() {
        return Err(BridgeErrorV3::invalid(format!("{field} pointer is null")));
    }
    let current = unsafe { output.read() };
    if !current.ptr.is_null() || current.len != 0 {
        return Err(BridgeErrorV3::invalid(format!(
            "{field} must be initialized to an empty SyndocalAsioStringV3"
        )));
    }
    Ok(())
}

/// Returns true without dereferencing either pointer when their complete typed
/// storage ranges overlap.  FFI callers sometimes pass an interior address of
/// another output slot; pointer equality alone would miss that alias and could
/// make a later output write corrupt the handle/result/error contract.
fn slots_overlap<T, U>(left: *const T, right: *const U) -> bool {
    if left.is_null() || right.is_null() {
        return false;
    }
    let left_start = left.cast::<u8>() as usize;
    let right_start = right.cast::<u8>() as usize;
    let Some(left_end) = left_start.checked_add(std::mem::size_of::<T>()) else {
        return true;
    };
    let Some(right_end) = right_start.checked_add(std::mem::size_of::<U>()) else {
        return true;
    };
    left_start < right_end && right_start < left_end
}

unsafe fn write_string(output: *mut SyndocalAsioStringV3, bytes: Vec<u8>) {
    unsafe { output.write(owned_bytes(bytes)) };
}

fn write_error(output: *mut SyndocalAsioStringV3, error: &BridgeErrorV3) {
    if output.is_null() {
        return;
    }
    let current = unsafe { output.read() };
    if !current.ptr.is_null() || current.len != 0 {
        return;
    }
    let payload = ErrorJsonV3 {
        schema_version: JSON_SCHEMA_VERSION_V3,
        kind: "error",
        status: error.status,
        code: error.code,
        message: &error.message,
    };
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        unsafe { write_string(output, bytes) };
    }
}

fn ffi_guard<T, F>(
    out_result: *mut SyndocalAsioStringV3,
    out_error: *mut SyndocalAsioStringV3,
    operation: F,
) -> u32
where
    T: Serialize,
    F: FnOnce() -> Result<T, BridgeErrorV3>,
{
    if slots_overlap(out_result, out_error) {
        // Do not write either slot: a cross-alias means even the error payload
        // would be an unauthorized write into the caller's result storage.
        return STATUS_INVALID_ARGUMENT_V3;
    }
    if let Err(error) = unsafe { require_empty_output(out_result, "out_result") } {
        write_error(out_error, &error);
        return error.status;
    }
    if !out_error.is_null() {
        if let Err(error) = unsafe { require_empty_output(out_error, "out_error") } {
            return error.status;
        }
    }
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => match serde_json::to_vec(&result) {
            Ok(bytes) => {
                unsafe { write_string(out_result, bytes) };
                STATUS_OK_V3
            }
            Err(error) => {
                let error = BridgeErrorV3::new(
                    STATUS_BACKEND_ERROR_V3,
                    "result_json_failed",
                    format!("v3 result JSON failed: {error}"),
                );
                write_error(out_error, &error);
                error.status
            }
        },
        Ok(Err(error)) => {
            let status = error.status;
            write_error(out_error, &error);
            status
        }
        Err(_) => {
            let error = BridgeErrorV3::new(
                STATUS_PANIC_V3,
                "panic",
                "the ASIO v3 bridge caught an internal panic before crossing FFI",
            );
            write_error(out_error, &error);
            error.status
        }
    }
}

fn ffi_start_guard<F>(
    out_handle: *mut *mut SyndocalAsioV3Handle,
    out_result: *mut SyndocalAsioStringV3,
    out_error: *mut SyndocalAsioStringV3,
    operation: F,
) -> u32
where
    F: FnOnce() -> Result<(SyndocalAsioV3Handle, StartResultJsonV3), BridgeErrorV3>,
{
    // Validate every output-slot range before any branch is allowed to write
    // the error JSON.  In particular, a null handle is still not permission
    // to clobber an aliased result/error pair.
    if slots_overlap(out_handle, out_result)
        || slots_overlap(out_handle, out_error)
        || slots_overlap(out_result, out_error)
    {
        // Every affected output is left byte-for-byte untouched.
        return STATUS_INVALID_ARGUMENT_V3;
    }
    if out_handle.is_null() {
        let error = BridgeErrorV3::invalid("out_handle pointer is null");
        write_error(out_error, &error);
        return error.status;
    }
    if !unsafe { out_handle.read() }.is_null() {
        let error = BridgeErrorV3::invalid("out_handle must point to null");
        write_error(out_error, &error);
        return error.status;
    }
    if let Err(error) = unsafe { require_empty_output(out_result, "out_result") } {
        write_error(out_error, &error);
        return error.status;
    }
    if !out_error.is_null() {
        if let Err(error) = unsafe { require_empty_output(out_error, "out_error") } {
            return error.status;
        }
    }
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok((handle, result))) => match serde_json::to_vec(&result) {
            Ok(bytes) => {
                unsafe {
                    out_result.write(owned_bytes(bytes));
                    out_handle.write(Box::into_raw(Box::new(handle)));
                }
                STATUS_OK_V3
            }
            Err(error) => {
                let error = BridgeErrorV3::new(
                    STATUS_BACKEND_ERROR_V3,
                    "result_json_failed",
                    format!("v3 start result JSON failed: {error}"),
                );
                write_error(out_error, &error);
                error.status
            }
        },
        Ok(Err(error)) => {
            let status = error.status;
            write_error(out_error, &error);
            status
        }
        Err(_) => {
            let error = BridgeErrorV3::new(
                STATUS_PANIC_V3,
                "panic",
                "the ASIO v3 bridge caught an internal panic while starting",
            );
            write_error(out_error, &error);
            error.status
        }
    }
}

unsafe fn parse_json<T: DeserializeOwned>(
    pointer: *const u8,
    len: usize,
    field: &str,
) -> Result<T, BridgeErrorV3> {
    if pointer.is_null() {
        return Err(BridgeErrorV3::invalid(format!("{field} pointer is null")));
    }
    if len == 0 || len > MAX_JSON_BYTES_V3 {
        return Err(BridgeErrorV3::invalid(format!(
            "{field} length must be between 1 and {MAX_JSON_BYTES_V3} bytes"
        )));
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, len) };
    let json = str::from_utf8(bytes)
        .map_err(|_| BridgeErrorV3::invalid(format!("{field} is not valid UTF-8")))?;
    serde_json::from_str(json).map_err(|error| {
        BridgeErrorV3::invalid(format!("{field} is not strict ABI v3 JSON: {error}"))
    })
}

fn validate_tuple(tuple: &StreamTupleJsonV3, field: &str) -> Result<(), BridgeErrorV3> {
    if tuple.channels == 0 || tuple.channels > MAX_CHANNELS_V3 {
        return Err(BridgeErrorV3::invalid(format!(
            "{field}.channels must be between 1 and {MAX_CHANNELS_V3}"
        )));
    }
    if !matches!(
        tuple.sample_format.as_str(),
        "f32" | "i16" | "i24" | "i32" | "f64"
    ) {
        return Err(BridgeErrorV3::invalid(format!(
            "{field}.sampleFormat must be exactly f32, i16, i24, i32, or f64"
        )));
    }
    if tuple.sample_rate_hz == 0 {
        return Err(BridgeErrorV3::invalid(format!(
            "{field}.sampleRateHz must not be zero"
        )));
    }
    if tuple.fixed_buffer_frames == 0 || tuple.fixed_buffer_frames > MAX_FIXED_BUFFER_FRAMES_V3 {
        return Err(BridgeErrorV3::invalid(format!(
            "{field}.fixedBufferFrames must be between 1 and {MAX_FIXED_BUFFER_FRAMES_V3}"
        )));
    }
    Ok(())
}

pub(crate) fn validate_shared_clock_v3(
    mode: StreamModeV3,
    output: &StreamTupleJsonV3,
    input: Option<&StreamTupleJsonV3>,
) -> Result<(), BridgeErrorV3> {
    match (mode, input) {
        (StreamModeV3::OutputOnly, None) => Ok(()),
        (StreamModeV3::OutputOnly, Some(_)) => Err(BridgeErrorV3::invalid(
            "outputOnly mode must not contain input",
        )),
        (StreamModeV3::FullDuplex, None) => {
            Err(BridgeErrorV3::invalid("fullDuplex mode requires input"))
        }
        (StreamModeV3::FullDuplex, Some(input)) => {
            if input.sample_rate_hz != output.sample_rate_hz {
                return Err(BridgeErrorV3::invalid(
                    "fullDuplex input.sampleRateHz must equal output.sampleRateHz because one ASIO stream has one device clock",
                ));
            }
            if input.fixed_buffer_frames != output.fixed_buffer_frames {
                return Err(BridgeErrorV3::invalid(
                    "fullDuplex input.fixedBufferFrames must equal output.fixedBufferFrames because one ASIO stream has one callback block size",
                ));
            }
            Ok(())
        }
    }
}

fn parse_start_request(config: StartRequestJsonV3) -> Result<StartRequestV3, BridgeErrorV3> {
    if config.schema_version != JSON_SCHEMA_VERSION_V3 {
        return Err(BridgeErrorV3::invalid(format!(
            "start request schemaVersion is {}, expected {JSON_SCHEMA_VERSION_V3}",
            config.schema_version
        )));
    }
    let driver_id = config.driver_id.trim();
    if !driver_id.starts_with("asio:")
        || driver_id.len() <= "asio:".len()
        || driver_id != config.driver_id
    {
        return Err(BridgeErrorV3::invalid(
            "driverId must be a non-whitespace explicit persistent asio:<driver name> ID",
        ));
    }
    validate_tuple(&config.output, "output")?;
    if let Some(input) = config.input.as_ref() {
        validate_tuple(input, "input")?;
    }
    validate_shared_clock_v3(config.mode, &config.output, config.input.as_ref())?;
    if config.session_generation == 0 || config.render_generation == 0 {
        return Err(BridgeErrorV3::invalid(
            "sessionGeneration and renderGeneration must both be non-zero",
        ));
    }
    Ok(StartRequestV3 {
        driver_id: config.driver_id,
        mode: config.mode,
        #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
        output: config.output,
        #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
        input: config.input,
        #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
        first_output_frame: config.first_output_frame,
        #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
        session_generation: config.session_generation,
        #[cfg(any(test, all(target_os = "windows", feature = "asio")))]
        render_generation: config.render_generation,
    })
}

#[no_mangle]
pub extern "C" fn syndocal_asio_v3_abi_version() -> u32 {
    ABI_VERSION_V3
}

#[no_mangle]
pub extern "C" fn syndocal_asio_v3_build_flags() -> u32 {
    let _frozen_contract_code_fingerprint = frozen_v3_contract_code_fingerprint();
    v3_rt_backend::build_flags()
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_drivers_json(
    out_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    ffi_guard(out_json, out_error_json, v3_rt_backend::drivers_json)
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_capabilities_json(
    request_json: *const u8,
    request_json_len: usize,
    out_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    ffi_guard(out_json, out_error_json, || {
        let request: CapabilitiesRequestJsonV3 =
            unsafe { parse_json(request_json, request_json_len, "capabilities request")? };
        if request.schema_version != JSON_SCHEMA_VERSION_V3
            || request.driver_id.trim() != request.driver_id
            || !request.driver_id.starts_with("asio:")
        {
            return Err(BridgeErrorV3::invalid(
                "capabilities request must carry strict ABI v3 schemaVersion and driverId",
            ));
        }
        v3_rt_backend::capabilities_json(&request.driver_id)
    })
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_string_free(string: SyndocalAsioStringV3) {
    if string.ptr.is_null() {
        return;
    }
    if catch_unwind(AssertUnwindSafe(|| unsafe {
        // `owned_bytes` allocates `Box<[u8]>`; reconstruct that exact owning
        // allocation rather than a Vec with coincidentally matching capacity.
        let slice = ptr::slice_from_raw_parts_mut(string.ptr, string.len);
        drop(Box::<[u8]>::from_raw(slice));
    }))
    .is_err()
    {
        // FFI teardown must never unwind into a foreign caller.
    }
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_start(
    request_json: *const u8,
    request_json_len: usize,
    output_callback: SyndocalAsioOutputCallbackV3,
    duplex_callback: SyndocalAsioDuplexCallbackV3,
    event_callback: SyndocalAsioEventCallbackV3,
    context: *mut c_void,
    out_handle: *mut *mut SyndocalAsioV3Handle,
    out_result_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    ffi_start_guard(out_handle, out_result_json, out_error_json, || {
        let request: StartRequestJsonV3 =
            unsafe { parse_json(request_json, request_json_len, "start request")? };
        let request = parse_start_request(request)?;
        match request.mode {
            StreamModeV3::OutputOnly if output_callback.is_none() || duplex_callback.is_some() => {
                return Err(BridgeErrorV3::invalid(
                    "outputOnly mode requires exactly output_callback and no duplex_callback",
                ));
            }
            StreamModeV3::FullDuplex if duplex_callback.is_none() || output_callback.is_some() => {
                return Err(BridgeErrorV3::invalid(
                    "fullDuplex mode requires exactly duplex_callback and no output_callback",
                ));
            }
            _ => {}
        }
        v3_rt_backend::start(
            request,
            v3_rt_backend::CallbackContractV3 {
                output: output_callback,
                duplex: duplex_callback,
                event: event_callback,
                context,
            },
        )
    })
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_stop(
    handle: *mut SyndocalAsioV3Handle,
    out_result_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    ffi_guard(out_result_json, out_error_json, || {
        if handle.is_null() {
            return Err(BridgeErrorV3::invalid("handle pointer is null"));
        }
        v3_rt_backend::stop(unsafe { &mut *handle })
    })
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_close(
    handle: *mut *mut SyndocalAsioV3Handle,
    out_result_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    if handle.is_null()
        || slots_overlap(handle, out_result_json)
        || slots_overlap(handle, out_error_json)
    {
        return STATUS_INVALID_ARGUMENT_V3;
    }
    ffi_guard(out_result_json, out_error_json, || {
        if unsafe { handle.read() }.is_null() {
            return Err(BridgeErrorV3::invalid(
                "handle must point to a non-null v3 handle",
            ));
        }
        let raw = unsafe { handle.read() };
        let result = v3_rt_backend::close(unsafe { &mut *raw })?;
        unsafe {
            drop(Box::from_raw(raw));
            handle.write(ptr::null_mut());
        }
        Ok(result)
    })
}

#[no_mangle]
pub unsafe extern "C" fn syndocal_asio_v3_telemetry_json(
    handle: *const SyndocalAsioV3Handle,
    out_json: *mut SyndocalAsioStringV3,
    out_error_json: *mut SyndocalAsioStringV3,
) -> u32 {
    ffi_guard(out_json, out_error_json, || {
        if handle.is_null() {
            return Err(BridgeErrorV3::invalid("handle pointer is null"));
        }
        v3_rt_backend::telemetry(unsafe { &*handle })
    })
}

#[cfg(test)]
#[path = "v3_abi_tests.rs"]
mod tests;
