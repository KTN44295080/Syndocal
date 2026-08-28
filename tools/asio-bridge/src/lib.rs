#![deny(unsafe_op_in_unsafe_fn)]

mod asio_host_lease;
mod v3_abi;
#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
mod v3_native;
mod v3_rt_backend;

use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::{
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice, str,
};

const ABI_VERSION: u32 = 2;
const JSON_SCHEMA_VERSION: u32 = 2;
const STATUS_OK: u32 = 0;
const STATUS_INVALID_ARGUMENT: u32 = 1;
#[cfg(not(all(target_os = "windows", feature = "asio")))]
const STATUS_UNSUPPORTED: u32 = 2;
#[cfg(all(target_os = "windows", feature = "asio"))]
const STATUS_DRIVER_NOT_FOUND: u32 = 3;
#[cfg(all(target_os = "windows", feature = "asio"))]
const STATUS_CONFIG_UNSUPPORTED: u32 = 4;
const STATUS_BACKEND_ERROR: u32 = 5;
#[cfg(all(target_os = "windows", feature = "asio"))]
const STATUS_TERMINAL: u32 = 6;
const STATUS_PANIC: u32 = 255;

const MAX_CHANNEL_MIX_ENTRIES: usize = 256;
const MAX_CHANNEL_MIX_GAIN: f64 = 1.0;
const MAX_FIXED_BUFFER_FRAMES: u32 = 1_048_576;
const MAX_JSON_BYTES: usize = 65_536;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyndocalAsioStringV2 {
    pub ptr: *mut u8,
    pub len: usize,
}

impl Default for SyndocalAsioStringV2 {
    fn default() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }
}

pub type SyndocalAsioSampleCallbackV2 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        mono_samples: *const f32,
        len: usize,
        capture_delay_ns: u64,
        callback_frames: u32,
    ),
>;

pub type SyndocalAsioEventCallbackV2 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        severity: u32,
        kind: u32,
        message: *const u8,
        message_len: usize,
    ),
>;

type SampleCallback = unsafe extern "C" fn(*mut c_void, *const f32, usize, u64, u32);
type EventCallback = unsafe extern "C" fn(*mut c_void, u32, u32, *const u8, usize);

#[cfg(all(target_os = "windows", feature = "asio"))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SampleFormatV2 {
    F32,
    I16,
    I24,
    I32,
    F64,
}

#[cfg(all(target_os = "windows", feature = "asio"))]
impl SampleFormatV2 {
    fn label(self) -> &'static str {
        match self {
            Self::F32 => "f32",
            Self::I16 => "i16",
            Self::I24 => "i24",
            Self::I32 => "i32",
            Self::F64 => "f64",
        }
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
#[derive(Clone, Debug)]
struct ChannelMix {
    channel_index: usize,
    gain: f32,
}

#[cfg(all(target_os = "windows", feature = "asio"))]
#[derive(Clone, Debug)]
struct StartRequest {
    driver_id: String,
    sample_rate_hz: u32,
    input_channels: u16,
    sample_format: SampleFormatV2,
    fixed_buffer_frames: u32,
    channel_mix: Vec<ChannelMix>,
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
#[derive(Clone, Debug)]
struct StartRequest;

#[derive(Debug)]
struct BridgeError {
    status: u32,
    code: &'static str,
    message: String,
}

impl BridgeError {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_INVALID_ARGUMENT,
            code: "invalid_argument",
            message: message.into(),
        }
    }

    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    fn unsupported(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_UNSUPPORTED,
            code: "unsupported",
            message: message.into(),
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    fn driver_not_found(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_DRIVER_NOT_FOUND,
            code: "driver_not_found",
            message: message.into(),
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    fn config_unsupported(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_CONFIG_UNSUPPORTED,
            code: "config_unsupported",
            message: message.into(),
        }
    }

    fn backend(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_BACKEND_ERROR,
            code: "backend_error",
            message: message.into(),
        }
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    fn terminal(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_TERMINAL,
            code: "terminal",
            message: message.into(),
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorJsonV2<'a> {
    schema_version: u32,
    kind: &'static str,
    status: u32,
    code: &'a str,
    message: &'a str,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CapabilitiesRequestJsonV2 {
    schema_version: u32,
    driver_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ChannelMixJsonV2 {
    channel_index: u32,
    // Validate the JSON number before narrowing to the realtime f32 path. This
    // prevents an out-of-range decimal such as 1.0000000001 from rounding down
    // to 1.0 during deserialization and bypassing the wire contract.
    gain: f64,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct StartRequestJsonV2 {
    schema_version: u32,
    driver_id: String,
    sample_rate_hz: u32,
    input_channels: u32,
    sample_format: String,
    fixed_buffer_frames: u32,
    channel_mix: Vec<ChannelMixJsonV2>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BufferFramesJson {
    min: u32,
    max: u32,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DriverJson {
    id: String,
    name: String,
    input_channels: u32,
    sample_formats: Vec<String>,
    sample_rates_hz: Vec<u32>,
    buffer_frames: BufferFramesJson,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CatalogJson {
    schema_version: u32,
    kind: &'static str,
    abi_version: u32,
    backend: &'static str,
    built: bool,
    drivers: Vec<DriverJson>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct InputConfigJson {
    channels: u32,
    sample_format: String,
    sample_rate_hz: u32,
    buffer_frames: BufferFramesJson,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilitiesJson {
    schema_version: u32,
    kind: &'static str,
    abi_version: u32,
    backend: &'static str,
    built: bool,
    driver: DriverJson,
    input_configs: Vec<InputConfigJson>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StartResultJsonV2 {
    schema_version: u32,
    kind: &'static str,
    actual_buffer_frames: u32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct StopResultJsonV2 {
    schema_version: u32,
    kind: &'static str,
    stopped: bool,
    stream_was_active: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CloseResultJsonV2 {
    schema_version: u32,
    kind: &'static str,
    handle_released: bool,
    stream_was_active: bool,
}

#[derive(Clone, Copy, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
struct PercentilesJsonV2 {
    p50: u64,
    p95: u64,
    p99: u64,
    max: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct TelemetrySnapshot {
    callbacks: u64,
    xruns: u64,
    callback_duration_ns: PercentilesJsonV2,
    capture_delay_ns: PercentilesJsonV2,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct TelemetryJsonV2 {
    schema_version: u32,
    kind: &'static str,
    callbacks: u64,
    xruns: u64,
    callback_duration_ns: PercentilesJsonV2,
    capture_delay_ns: PercentilesJsonV2,
}

trait StreamLifecycle {
    fn actual_buffer_frames(&self) -> Result<u32, BridgeError>;
    fn stop(&mut self) -> Result<bool, BridgeError>;
    fn close(&mut self) -> Result<bool, BridgeError>;
    fn telemetry(&self) -> TelemetrySnapshot;
    fn terminal_faulted(&self) -> bool {
        false
    }
}

fn open_stream_with<F>(
    request: StartRequest,
    sample_callback: SampleCallback,
    event_callback: EventCallback,
    context: usize,
    factory: F,
) -> Result<(SyndocalAsioHandle, u32), BridgeError>
where
    F: FnOnce(
        StartRequest,
        SampleCallback,
        EventCallback,
        usize,
    ) -> Result<Box<dyn StreamLifecycle>, BridgeError>,
{
    let inner = factory(request, sample_callback, event_callback, context)?;
    let actual_buffer_frames = inner.actual_buffer_frames()?;
    Ok((
        SyndocalAsioHandle { inner, lease: None },
        actual_buffer_frames,
    ))
}

pub struct SyndocalAsioHandle {
    inner: Box<dyn StreamLifecycle>,
    lease: Option<asio_host_lease::Ticket>,
}

impl Drop for SyndocalAsioHandle {
    fn drop(&mut self) {
        let Some(ticket) = self.lease.take() else {
            return;
        };
        let _ = asio_host_lease::begin_stop(ticket);
        if self.inner.close().is_ok() {
            asio_host_lease::stop_succeeded(ticket);
        } else {
            asio_host_lease::stop_failed(ticket);
        }
    }
}

fn asio_host_busy(error: asio_host_lease::BusyState) -> BridgeError {
    BridgeError::backend(format!(
        "ASIO host is {}; the current v2/v3 owner must be explicitly stopped and closed before this operation",
        error.label()
    ))
}

fn required_handle_lease(
    lease: Option<asio_host_lease::Ticket>,
) -> Result<Option<asio_host_lease::Ticket>, BridgeError> {
    if lease.is_some() {
        return Ok(lease);
    }
    #[cfg(test)]
    {
        Ok(None)
    }
    #[cfg(not(test))]
    {
        Err(BridgeError::backend(
            "ASIO handle has no process host lease",
        ))
    }
}

fn owned_bytes(bytes: Vec<u8>) -> SyndocalAsioStringV2 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed).cast::<u8>();
    SyndocalAsioStringV2 { ptr, len }
}

unsafe fn require_empty_output(
    output: *mut SyndocalAsioStringV2,
    field: &str,
) -> Result<(), BridgeError> {
    if output.is_null() {
        return Err(BridgeError::invalid(format!("{field} pointer is null")));
    }
    let current = unsafe { output.read() };
    if !current.ptr.is_null() || current.len != 0 {
        return Err(BridgeError::invalid(format!(
            "{field} must be initialized to an empty SyndocalAsioStringV2"
        )));
    }
    Ok(())
}

unsafe fn write_string(output: *mut SyndocalAsioStringV2, bytes: Vec<u8>) {
    unsafe { output.write(owned_bytes(bytes)) };
}

fn write_error(output: *mut SyndocalAsioStringV2, error: &BridgeError) {
    if output.is_null() {
        return;
    }
    let current = unsafe { output.read() };
    if !current.ptr.is_null() || current.len != 0 {
        return;
    }
    let payload = ErrorJsonV2 {
        schema_version: JSON_SCHEMA_VERSION,
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
    out_result: *mut SyndocalAsioStringV2,
    out_error: *mut SyndocalAsioStringV2,
    operation: F,
) -> u32
where
    T: Serialize,
    F: FnOnce() -> Result<T, BridgeError>,
{
    if out_result == out_error {
        let error = BridgeError::invalid("out_result and out_error must not alias");
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
        Ok(Ok(result)) => match serde_json::to_vec(&result) {
            Ok(bytes) => {
                unsafe { write_string(out_result, bytes) };
                STATUS_OK
            }
            Err(error) => {
                let error = BridgeError::backend(format!("result JSON failed: {error}"));
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
            let error = BridgeError {
                status: STATUS_PANIC,
                code: "panic",
                message: "the ASIO bridge caught an internal panic".to_string(),
            };
            write_error(out_error, &error);
            STATUS_PANIC
        }
    }
}

fn ffi_start_guard<F>(
    out_handle: *mut *mut SyndocalAsioHandle,
    out_result: *mut SyndocalAsioStringV2,
    out_error: *mut SyndocalAsioStringV2,
    operation: F,
) -> u32
where
    F: FnOnce() -> Result<(SyndocalAsioHandle, StartResultJsonV2), BridgeError>,
{
    if out_result == out_error {
        let error = BridgeError::invalid("out_result and out_error must not alias");
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
    if out_handle.is_null() {
        let error = BridgeError::invalid("out_handle pointer is null");
        write_error(out_error, &error);
        return error.status;
    }
    if !unsafe { out_handle.read() }.is_null() {
        let error = BridgeError::invalid("out_handle must point to null");
        write_error(out_error, &error);
        return error.status;
    }

    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok((handle, result))) => match serde_json::to_vec(&result) {
            Ok(bytes) => {
                let owned_result = owned_bytes(bytes);
                let owned_handle = Box::into_raw(Box::new(handle));
                unsafe {
                    out_result.write(owned_result);
                    out_handle.write(owned_handle);
                }
                STATUS_OK
            }
            Err(error) => {
                let error = BridgeError::backend(format!("start result JSON failed: {error}"));
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
            let error = BridgeError {
                status: STATUS_PANIC,
                code: "panic",
                message: "the ASIO bridge caught an internal panic while starting".to_string(),
            };
            write_error(out_error, &error);
            STATUS_PANIC
        }
    }
}

unsafe fn parse_json<T: DeserializeOwned>(
    pointer: *const u8,
    len: usize,
    field: &str,
) -> Result<T, BridgeError> {
    if pointer.is_null() {
        return Err(BridgeError::invalid(format!("{field} pointer is null")));
    }
    if len == 0 || len > MAX_JSON_BYTES {
        return Err(BridgeError::invalid(format!(
            "{field} length must be between 1 and {MAX_JSON_BYTES} bytes"
        )));
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, len) };
    let json = str::from_utf8(bytes)
        .map_err(|_| BridgeError::invalid(format!("{field} is not valid UTF-8")))?;
    serde_json::from_str(json).map_err(|error| {
        BridgeError::invalid(format!("{field} is not strict ABI v2 JSON: {error}"))
    })
}

fn validate_schema_version(version: u32, field: &str) -> Result<(), BridgeError> {
    if version != JSON_SCHEMA_VERSION {
        return Err(BridgeError::invalid(format!(
            "{field} schemaVersion is {version}, expected {JSON_SCHEMA_VERSION}"
        )));
    }
    Ok(())
}

fn parse_start_request(config: StartRequestJsonV2) -> Result<StartRequest, BridgeError> {
    validate_schema_version(config.schema_version, "start request")?;
    let driver_id = config.driver_id.trim();
    if !driver_id.starts_with("asio:") || driver_id.len() <= "asio:".len() {
        return Err(BridgeError::invalid(
            "driver_id must be an explicit persistent asio:<driver name> ID",
        ));
    }
    if config.driver_id != driver_id {
        return Err(BridgeError::invalid(
            "driver_id must not contain surrounding whitespace",
        ));
    }
    if config.sample_rate_hz == 0 {
        return Err(BridgeError::invalid("sample_rate_hz must not be zero"));
    }
    if config.input_channels == 0 || config.input_channels > u16::MAX as u32 {
        return Err(BridgeError::invalid(format!(
            "input_channels must be between 1 and {}",
            u16::MAX
        )));
    }
    if !matches!(
        config.sample_format.as_str(),
        "f32" | "i16" | "i24" | "i32" | "f64"
    ) {
        return Err(BridgeError::invalid(
            "sample_format must be exactly f32, i16, i24, i32, or f64",
        ));
    }
    #[cfg(all(target_os = "windows", feature = "asio"))]
    let sample_format = match config.sample_format.as_str() {
        "f32" => SampleFormatV2::F32,
        "i16" => SampleFormatV2::I16,
        "i24" => SampleFormatV2::I24,
        "i32" => SampleFormatV2::I32,
        "f64" => SampleFormatV2::F64,
        _ => unreachable!("sample format was validated above"),
    };
    if config.fixed_buffer_frames == 0 || config.fixed_buffer_frames > MAX_FIXED_BUFFER_FRAMES {
        return Err(BridgeError::invalid(format!(
            "fixed_buffer_frames must be between 1 and {MAX_FIXED_BUFFER_FRAMES}"
        )));
    }
    if config.channel_mix.is_empty() || config.channel_mix.len() > MAX_CHANNEL_MIX_ENTRIES {
        return Err(BridgeError::invalid(format!(
            "channel_mix_len must be between 1 and {MAX_CHANNEL_MIX_ENTRIES}"
        )));
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    let mut channel_mix = Vec::with_capacity(config.channel_mix.len());
    let mut has_audible_gain = false;
    for entry in config.channel_mix {
        if entry.channel_index >= config.input_channels {
            return Err(BridgeError::invalid(format!(
                "channel mix index {} is outside the {} configured input channels",
                entry.channel_index, config.input_channels
            )));
        }
        if !entry.gain.is_finite() {
            return Err(BridgeError::invalid("channel mix gains must be finite"));
        }
        if !(0.0..=MAX_CHANNEL_MIX_GAIN).contains(&entry.gain) {
            return Err(BridgeError::invalid(format!(
                "channel mix gains must be between 0 and {MAX_CHANNEL_MIX_GAIN}"
            )));
        }
        has_audible_gain |= entry.gain != 0.0;
        #[cfg(all(target_os = "windows", feature = "asio"))]
        channel_mix.push(ChannelMix {
            channel_index: entry.channel_index as usize,
            gain: entry.gain as f32,
        });
    }
    if !has_audible_gain {
        return Err(BridgeError::invalid(
            "channel mix must contain at least one non-zero gain",
        ));
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    {
        Ok(StartRequest {
            driver_id: driver_id.to_owned(),
            sample_rate_hz: config.sample_rate_hz,
            input_channels: config.input_channels as u16,
            sample_format,
            fixed_buffer_frames: config.fixed_buffer_frames,
            channel_mix,
        })
    }
    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    {
        Ok(StartRequest)
    }
}

#[no_mangle]
pub extern "C" fn syndocal_asio_v2_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn syndocal_asio_v2_build_flags() -> u32 {
    u32::from(cfg!(all(target_os = "windows", feature = "asio")))
}

#[no_mangle]
/// Returns the current driver catalog as strict, versioned UTF-8 JSON.
///
/// # Safety
///
/// Both output strings must be initialized empty and must not alias. `out_error_json` may be null.
pub unsafe extern "C" fn syndocal_asio_v2_drivers_json(
    out_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_guard(out_json, out_error_json, || {
        let inspection = asio_host_lease::Inspection::begin().map_err(asio_host_busy)?;
        let result = backend::drivers_json();
        inspection.complete();
        result
    })
}

#[no_mangle]
/// Returns capabilities for the exact driver named by strict ABI v2 request JSON.
///
/// # Safety
///
/// `request_json` must reference `request_json_len` readable bytes. Both output strings must be
/// initialized empty and must not alias. `out_error_json` may be null.
pub unsafe extern "C" fn syndocal_asio_v2_capabilities_json(
    request_json: *const u8,
    request_json_len: usize,
    out_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_guard(out_json, out_error_json, || {
        let request: CapabilitiesRequestJsonV2 =
            unsafe { parse_json(request_json, request_json_len, "capabilities request") }?;
        validate_schema_version(request.schema_version, "capabilities request")?;
        if request.driver_id.trim() != request.driver_id
            || !request.driver_id.starts_with("asio:")
            || request.driver_id.len() <= "asio:".len()
        {
            return Err(BridgeError::invalid(
                "driverId must be an exact persistent asio:<driver name> ID without surrounding whitespace",
            ));
        }
        let inspection = asio_host_lease::Inspection::begin().map_err(asio_host_busy)?;
        let result = backend::capabilities_json(&request.driver_id);
        inspection.complete();
        result
    })
}

#[no_mangle]
/// Releases a string returned by this exact loaded v2 bridge DLL.
///
/// # Safety
///
/// A non-empty string must have been returned by this DLL and released exactly once.
pub unsafe extern "C" fn syndocal_asio_v2_string_free(string: SyndocalAsioStringV2) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if string.ptr.is_null() || string.len == 0 {
            return;
        }
        let raw = ptr::slice_from_raw_parts_mut(string.ptr, string.len);
        unsafe { drop(Box::from_raw(raw)) };
    }));
}

#[no_mangle]
/// Revalidates and starts one exact ASIO stream from strict ABI v2 request JSON.
///
/// # Safety
///
/// The request bytes and callbacks must remain valid for the documented durations. `out_handle`
/// must point to a null handle. Both output strings must be initialized empty and must not alias.
pub unsafe extern "C" fn syndocal_asio_v2_start(
    request_json: *const u8,
    request_json_len: usize,
    sample_callback: SyndocalAsioSampleCallbackV2,
    event_callback: SyndocalAsioEventCallbackV2,
    context: *mut c_void,
    out_handle: *mut *mut SyndocalAsioHandle,
    out_result_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_start_guard(out_handle, out_result_json, out_error_json, || {
        let sample_callback = sample_callback
            .ok_or_else(|| BridgeError::invalid("sample_callback must not be null"))?;
        let event_callback = event_callback
            .ok_or_else(|| BridgeError::invalid("event_callback must not be null"))?;
        let config: StartRequestJsonV2 =
            unsafe { parse_json(request_json, request_json_len, "start request") }?;
        let request = parse_start_request(config)?;
        let guard = asio_host_lease::StartGuard::begin(asio_host_lease::Owner::V2)
            .map_err(asio_host_busy)?;
        let (mut handle, actual_buffer_frames) = open_stream_with(
            request,
            sample_callback,
            event_callback,
            context as usize,
            backend::start,
        )?;
        handle.lease = Some(guard.activate());
        Ok((
            handle,
            StartResultJsonV2 {
                schema_version: JSON_SCHEMA_VERSION,
                kind: "start",
                actual_buffer_frames,
            },
        ))
    })
}

#[no_mangle]
/// Stops delivery without releasing the handle. A stopped handle may only be queried or closed.
///
/// # Safety
///
/// `handle` must be live and exclusively owned for this call. Both output strings must be
/// initialized empty and must not alias.
pub unsafe extern "C" fn syndocal_asio_v2_stop(
    handle: *mut SyndocalAsioHandle,
    out_result_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_guard(out_result_json, out_error_json, || {
        if handle.is_null() {
            return Err(BridgeError::invalid("handle must not be null"));
        }
        let handle = unsafe { &mut *handle };
        let ticket = required_handle_lease(handle.lease)?;
        if let Some(ticket) = ticket {
            if handle.inner.terminal_faulted() {
                asio_host_lease::mark_fault(ticket);
            }
            asio_host_lease::begin_stop(ticket).map_err(asio_host_busy)?;
        }
        let stream_was_active = match handle.inner.stop() {
            Ok(active) => active,
            Err(error) => {
                if let Some(ticket) = ticket {
                    asio_host_lease::stop_failed(ticket);
                }
                return Err(error);
            }
        };
        Ok(StopResultJsonV2 {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "stop",
            stopped: true,
            stream_was_active,
        })
    })
}

#[no_mangle]
/// Stops if needed, releases the handle, and reports any close failure.
///
/// The pointer is set to null before teardown begins, including error and panic paths. There is no
/// v2 `free` shortcut that can discard a close failure.
///
/// # Safety
///
/// `handle` must point to a live, exclusively owned handle returned by this DLL. Both output
/// strings must be initialized empty and must not alias. Do not call from a bridge callback.
pub unsafe extern "C" fn syndocal_asio_v2_close(
    handle: *mut *mut SyndocalAsioHandle,
    out_result_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_guard(out_result_json, out_error_json, || {
        if handle.is_null() {
            return Err(BridgeError::invalid("handle pointer is null"));
        }
        let raw = unsafe { handle.replace(ptr::null_mut()) };
        if raw.is_null() {
            return Err(BridgeError::invalid("handle already points to null"));
        }
        let mut owned = unsafe { Box::from_raw(raw) };
        let ticket = required_handle_lease(owned.lease.take())?;
        if let Some(ticket) = ticket {
            if owned.inner.terminal_faulted() {
                asio_host_lease::mark_fault(ticket);
            }
            asio_host_lease::begin_stop(ticket).map_err(asio_host_busy)?;
        }
        let stream_was_active = match owned.inner.close() {
            Ok(active) => {
                if let Some(ticket) = ticket {
                    asio_host_lease::stop_succeeded(ticket);
                }
                active
            }
            Err(error) => {
                if let Some(ticket) = ticket {
                    asio_host_lease::stop_failed(ticket);
                }
                return Err(error);
            }
        };
        drop(owned);
        Ok(CloseResultJsonV2 {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "close",
            handle_released: true,
            stream_was_active,
        })
    })
}

#[no_mangle]
/// Returns an atomic telemetry snapshot as strict versioned UTF-8 JSON.
///
/// # Safety
///
/// `handle` must be live and not used concurrently with Close. Both output strings must be
/// initialized empty and must not alias.
pub unsafe extern "C" fn syndocal_asio_v2_telemetry_json(
    handle: *const SyndocalAsioHandle,
    out_json: *mut SyndocalAsioStringV2,
    out_error_json: *mut SyndocalAsioStringV2,
) -> u32 {
    ffi_guard(out_json, out_error_json, || {
        if handle.is_null() {
            return Err(BridgeError::invalid("handle must not be null"));
        }
        let handle = unsafe { &*handle };
        if handle.inner.terminal_faulted() {
            if let Some(ticket) = handle.lease {
                asio_host_lease::mark_fault(ticket);
            }
        }
        let snapshot = handle.inner.telemetry();
        Ok(TelemetryJsonV2 {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "telemetry",
            callbacks: snapshot.callbacks,
            xruns: snapshot.xruns,
            callback_duration_ns: snapshot.callback_duration_ns,
            capture_delay_ns: snapshot.capture_delay_ns,
        })
    })
}

#[cfg(all(target_os = "windows", feature = "asio"))]
mod backend {
    use super::*;
    use cpal_asio::{
        traits::{DeviceTrait, HostTrait, StreamTrait},
        BufferSize, Device, DeviceId, Error, ErrorKind, FromSample, HostId, Sample, SampleFormat,
        SizedSample, Stream, StreamConfig, SupportedBufferSize, I24,
    };
    use std::{
        collections::BTreeSet,
        sync::{
            atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
            Arc,
        },
        thread::{self, JoinHandle},
        time::{Duration, Instant},
    };

    const EVENT_WARNING: u32 = 1;
    const EVENT_TERMINAL: u32 = 2;
    const EVENT_XRUN: u32 = 1;
    const EVENT_RESET: u32 = 2;
    const EVENT_RESYNC: u32 = 3;
    const EVENT_SAMPLE_RATE_CHANGED: u32 = 4;
    const EVENT_DEVICE_LOST: u32 = 5;
    const EVENT_CALLBACK_GAP: u32 = 6;
    const EVENT_REALTIME_DENIED: u32 = 7;
    const EVENT_BACKEND: u32 = 8;
    const EVENT_MALFORMED_CALLBACK: u32 = 9;
    const EVENT_BUFFER_SIZE_CHANGED: u32 = 10;
    const CALLBACK_GAP_NS: u64 = 250_000_000;
    const HISTOGRAM_BUCKETS: usize = 64;

    struct AtomicHistogram {
        buckets: [AtomicU64; HISTOGRAM_BUCKETS],
        max: AtomicU64,
    }

    impl AtomicHistogram {
        fn new() -> Self {
            Self {
                buckets: std::array::from_fn(|_| AtomicU64::new(0)),
                max: AtomicU64::new(0),
            }
        }

        fn record(&self, value: u64) {
            let bucket = if value == 0 {
                0
            } else {
                (u64::BITS - value.leading_zeros()) as usize
            }
            .min(HISTOGRAM_BUCKETS - 1);
            self.buckets[bucket].fetch_add(1, Ordering::Relaxed);
            self.max.fetch_max(value, Ordering::Relaxed);
        }

        fn snapshot(&self, total: u64) -> PercentilesJsonV2 {
            let percentile = |numerator: u64, denominator: u64| {
                if total == 0 {
                    return 0;
                }
                let target = total
                    .saturating_mul(numerator)
                    .saturating_add(denominator - 1)
                    / denominator;
                let mut observed = 0_u64;
                for (index, bucket) in self.buckets.iter().enumerate() {
                    observed = observed.saturating_add(bucket.load(Ordering::Acquire));
                    if observed >= target {
                        return match index {
                            0 => 0,
                            63 => u64::MAX,
                            _ => (1_u64 << index) - 1,
                        };
                    }
                }
                self.max.load(Ordering::Acquire)
            };
            PercentilesJsonV2 {
                p50: percentile(50, 100),
                p95: percentile(95, 100),
                p99: percentile(99, 100),
                max: self.max.load(Ordering::Acquire),
            }
        }
    }

    struct TelemetryState {
        callbacks: AtomicU64,
        callback_duration_ns: AtomicHistogram,
        capture_delay_ns: AtomicHistogram,
    }

    impl TelemetryState {
        fn new() -> Self {
            Self {
                callbacks: AtomicU64::new(0),
                callback_duration_ns: AtomicHistogram::new(),
                capture_delay_ns: AtomicHistogram::new(),
            }
        }

        fn record(&self, duration_ns: u64, capture_delay_ns: u64) {
            self.callback_duration_ns.record(duration_ns);
            self.capture_delay_ns.record(capture_delay_ns);
            self.callbacks.fetch_add(1, Ordering::Release);
        }
    }

    struct CallbackMeasurement<'a> {
        telemetry: &'a TelemetryState,
        started_at: Instant,
        capture_delay_ns: u64,
    }

    impl Drop for CallbackMeasurement<'_> {
        fn drop(&mut self) {
            let duration_ns = self.started_at.elapsed().as_nanos().min(u64::MAX as u128) as u64;
            self.telemetry.record(duration_ns, self.capture_delay_ns);
        }
    }

    fn callback_gap_due(active: bool, terminal: bool, last_callback_ns: u64, now_ns: u64) -> bool {
        active
            && !terminal
            && last_callback_ns != 0
            && now_ns.saturating_sub(last_callback_ns) >= CALLBACK_GAP_NS
    }

    #[derive(Clone)]
    struct SharedState {
        active: Arc<AtomicBool>,
        terminal: Arc<AtomicBool>,
        last_callback_ns: Arc<AtomicU64>,
        expected_frames: Arc<AtomicU32>,
        xrun_count: Arc<AtomicU64>,
        telemetry: Arc<TelemetryState>,
        origin: Instant,
        event_callback: EventCallback,
        context: usize,
    }

    impl SharedState {
        fn new(event_callback: EventCallback, context: usize) -> Self {
            Self {
                active: Arc::new(AtomicBool::new(false)),
                terminal: Arc::new(AtomicBool::new(false)),
                last_callback_ns: Arc::new(AtomicU64::new(0)),
                expected_frames: Arc::new(AtomicU32::new(0)),
                xrun_count: Arc::new(AtomicU64::new(0)),
                telemetry: Arc::new(TelemetryState::new()),
                origin: Instant::now(),
                event_callback,
                context,
            }
        }

        fn now_ns(&self) -> u64 {
            self.origin.elapsed().as_nanos().min(u64::MAX as u128) as u64
        }

        fn emit(&self, severity: u32, kind: u32, message: &'static [u8]) {
            let callback = self.event_callback;
            let context = self.context as *mut c_void;
            self.run_client_event_callback(|| unsafe {
                callback(context, severity, kind, message.as_ptr(), message.len())
            });
        }

        fn run_client_event_callback(&self, operation: impl FnOnce()) {
            if catch_unwind(AssertUnwindSafe(operation)).is_err() {
                // The consumer violated the no-unwind callback contract. Do
                // not attempt a recursive notification through that same
                // callback; atomically stop delivery and latch the terminal
                // state so every other observer fails closed.
                self.active.store(false, Ordering::Release);
                self.terminal.store(true, Ordering::Release);
            }
        }

        fn terminal(&self, kind: u32, message: &'static [u8]) {
            self.active.store(false, Ordering::Release);
            if !self.terminal.swap(true, Ordering::AcqRel) {
                self.emit(EVENT_TERMINAL, kind, message);
            }
        }
    }

    struct Watchdog {
        stop: Arc<AtomicBool>,
        thread: Option<JoinHandle<()>>,
    }

    impl Watchdog {
        fn spawn(shared: SharedState) -> Result<Self, BridgeError> {
            let stop = Arc::new(AtomicBool::new(false));
            let thread_stop = stop.clone();
            let thread = thread::Builder::new()
                .name("syndocal-asio-watchdog".to_string())
                .spawn(move || {
                    while !thread_stop.load(Ordering::Acquire) {
                        let active = shared.active.load(Ordering::Acquire);
                        let terminal = shared.terminal.load(Ordering::Acquire);
                        if !active || terminal {
                            thread::sleep(Duration::from_millis(25));
                            continue;
                        }
                        let last = shared.last_callback_ns.load(Ordering::Acquire);
                        let now = shared.now_ns();
                        if callback_gap_due(active, terminal, last, now) {
                            shared.terminal(
                                EVENT_CALLBACK_GAP,
                                b"ASIO input callback gap exceeded 250 ms",
                            );
                        } else {
                            let elapsed = now.saturating_sub(last);
                            thread::sleep(Duration::from_nanos(
                                (CALLBACK_GAP_NS - elapsed).min(25_000_000),
                            ));
                        }
                    }
                })
                .map_err(|error| {
                    BridgeError::backend(format!("failed to start ASIO watchdog: {error}"))
                })?;
            Ok(Self {
                stop,
                thread: Some(thread),
            })
        }
    }

    impl Drop for Watchdog {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Release);
            if let Some(thread) = self.thread.take() {
                if thread.thread().id() != std::thread::current().id() {
                    let _ = thread.join();
                }
            }
        }
    }

    pub(super) struct StreamHandle {
        stream: Stream,
        shared: SharedState,
        applied_buffer_frames: u32,
        stopped: bool,
        _watchdog: Watchdog,
    }

    impl StreamLifecycle for StreamHandle {
        fn actual_buffer_frames(&self) -> Result<u32, BridgeError> {
            Ok(self.applied_buffer_frames)
        }

        fn stop(&mut self) -> Result<bool, BridgeError> {
            if self.stopped {
                return Ok(false);
            }
            let was_active = self.shared.active.swap(false, Ordering::AcqRel);
            self.stream
                .pause()
                .map_err(|error| cpal_error("failed to pause ASIO input stream", error))?;
            self.stopped = true;
            Ok(was_active)
        }

        fn close(&mut self) -> Result<bool, BridgeError> {
            self.stop()
        }

        fn telemetry(&self) -> TelemetrySnapshot {
            let callbacks = self.shared.telemetry.callbacks.load(Ordering::Acquire);
            TelemetrySnapshot {
                callbacks,
                xruns: self.shared.xrun_count.load(Ordering::Acquire),
                callback_duration_ns: self
                    .shared
                    .telemetry
                    .callback_duration_ns
                    .snapshot(callbacks),
                capture_delay_ns: self.shared.telemetry.capture_delay_ns.snapshot(callbacks),
            }
        }

        fn terminal_faulted(&self) -> bool {
            self.shared.terminal.load(Ordering::Acquire)
        }
    }

    impl StreamHandle {
        fn start_stream(&mut self) -> Result<(), BridgeError> {
            if self.shared.terminal.load(Ordering::Acquire) {
                return Err(BridgeError::terminal("terminal ASIO stream cannot start"));
            }
            self.shared.active.store(false, Ordering::Release);
            if let Err(error) = self.stream.play() {
                self.shared.active.store(false, Ordering::Release);
                return Err(cpal_error("failed to play ASIO input stream", error));
            }
            self.shared
                .last_callback_ns
                .store(self.shared.now_ns().max(1), Ordering::Release);
            self.shared.active.store(true, Ordering::Release);
            Ok(())
        }
    }

    impl Drop for StreamHandle {
        fn drop(&mut self) {
            self.shared.active.store(false, Ordering::Release);
            if !self.stopped {
                let _ = self.stream.pause();
            }
        }
    }

    pub(super) fn drivers_json() -> Result<CatalogJson, BridgeError> {
        let host = asio_host()?;
        let devices = host
            .input_devices()
            .map_err(|error| cpal_error("failed to enumerate ASIO input drivers", error))?;
        let mut drivers = Vec::new();
        for device in devices {
            drivers.push(driver_details(&device)?.0);
        }
        Ok(CatalogJson {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "drivers",
            abi_version: ABI_VERSION,
            backend: "asio",
            built: true,
            drivers,
        })
    }

    pub(super) fn capabilities_json(driver_id: &str) -> Result<CapabilitiesJson, BridgeError> {
        let device = find_explicit_device(driver_id)?;
        let (driver, input_configs) = driver_details(&device)?;
        Ok(CapabilitiesJson {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "capabilities",
            abi_version: ABI_VERSION,
            backend: "asio",
            built: true,
            driver,
            input_configs,
        })
    }

    pub(super) fn start(
        request: StartRequest,
        sample_callback: SampleCallback,
        event_callback: EventCallback,
        context: usize,
    ) -> Result<Box<dyn StreamLifecycle>, BridgeError> {
        let device = find_explicit_device(&request.driver_id)?;
        validate_exact_config(&device, &request)?;

        let stream_config = StreamConfig {
            channels: request.input_channels,
            sample_rate: request.sample_rate_hz,
            buffer_size: BufferSize::Fixed(request.fixed_buffer_frames),
        };
        let shared = SharedState::new(event_callback, context);
        shared
            .expected_frames
            .store(request.fixed_buffer_frames, Ordering::Release);

        let stream = match request.sample_format {
            SampleFormatV2::F32 => build_typed_stream::<f32>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV2::I16 => build_typed_stream::<i16>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV2::I24 => build_typed_stream::<I24>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV2::I32 => build_typed_stream::<i32>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV2::F64 => build_typed_stream::<f64>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
        }?;

        let applied = stream
            .buffer_size()
            .map_err(|error| cpal_error("failed to query applied ASIO buffer", error))?;
        if applied == 0 || applied != request.fixed_buffer_frames {
            return Err(BridgeError::config_unsupported(format!(
                "ASIO driver applied {applied} frames instead of requested {}",
                request.fixed_buffer_frames
            )));
        }
        shared.expected_frames.store(applied, Ordering::Release);

        let watchdog = Watchdog::spawn(shared.clone())?;
        let mut handle = StreamHandle {
            stream,
            shared,
            applied_buffer_frames: applied,
            stopped: false,
            _watchdog: watchdog,
        };
        handle.start_stream()?;
        Ok(Box::new(handle))
    }

    fn asio_host() -> Result<cpal_asio::Host, BridgeError> {
        cpal_asio::host_from_id(HostId::Asio)
            .map_err(|error| cpal_error("ASIO host is unavailable", error))
    }

    fn find_explicit_device(driver_id: &str) -> Result<Device, BridgeError> {
        let parsed = driver_id
            .parse::<DeviceId>()
            .map_err(|error| cpal_error("invalid ASIO driver ID", error))?;
        if parsed.host() != HostId::Asio {
            return Err(BridgeError::invalid(
                "driver ID does not name the ASIO host",
            ));
        }
        let host = asio_host()?;
        host.device_by_id(&parsed).ok_or_else(|| {
            BridgeError::driver_not_found(format!(
                "explicit ASIO driver is not present in the current catalog: {driver_id}"
            ))
        })
    }

    fn driver_details(device: &Device) -> Result<(DriverJson, Vec<InputConfigJson>), BridgeError> {
        let id = device
            .id()
            .map_err(|error| cpal_error("failed to read ASIO driver ID", error))?
            .to_string();
        let name = device.to_string();
        let configs = device
            .supported_input_configs()
            .map_err(|error| cpal_error("failed to read ASIO input capabilities", error))?;

        let mut tuples = BTreeSet::new();
        for config in configs {
            let Some(format) = sample_format_label(config.sample_format()) else {
                continue;
            };
            let SupportedBufferSize::Range { min, max } = *config.buffer_size() else {
                continue;
            };
            if min == 0 || max < min {
                continue;
            }
            let min_rate = config.min_sample_rate();
            let max_rate = config.max_sample_rate();
            if min_rate == 0 || min_rate != max_rate {
                continue;
            }
            tuples.insert((
                config.channels() as u32,
                format.to_string(),
                min_rate,
                min,
                max,
            ));
        }

        let mut formats = BTreeSet::new();
        let mut rates = BTreeSet::new();
        let mut input_channels = 0;
        let mut buffer_min = u32::MAX;
        let mut buffer_max = 0;
        let mut input_configs = Vec::with_capacity(tuples.len());
        for (channels, format, rate, min, max) in tuples {
            formats.insert(format.clone());
            rates.insert(rate);
            input_channels = input_channels.max(channels);
            buffer_min = buffer_min.min(min);
            buffer_max = buffer_max.max(max);
            input_configs.push(InputConfigJson {
                channels,
                sample_format: format,
                sample_rate_hz: rate,
                buffer_frames: BufferFramesJson { min, max },
            });
        }
        if input_configs.is_empty() {
            return Err(BridgeError::config_unsupported(format!(
                "ASIO driver exposes no exact fixed-buffer input configuration: {id}"
            )));
        }

        Ok((
            DriverJson {
                id,
                name,
                input_channels,
                sample_formats: formats.into_iter().collect(),
                sample_rates_hz: rates.into_iter().collect(),
                buffer_frames: BufferFramesJson {
                    min: buffer_min,
                    max: buffer_max,
                },
            },
            input_configs,
        ))
    }

    fn validate_exact_config(device: &Device, request: &StartRequest) -> Result<(), BridgeError> {
        let expected_format = to_cpal_sample_format(request.sample_format);
        let configs = device
            .supported_input_configs()
            .map_err(|error| cpal_error("failed to revalidate ASIO input config", error))?;
        let matched = configs.into_iter().any(|config| {
            if config.channels() != request.input_channels
                || config.sample_format() != expected_format
                || !config.contains_rate(request.sample_rate_hz)
            {
                return false;
            }
            matches!(
                *config.buffer_size(),
                SupportedBufferSize::Range { min, max }
                    if request.fixed_buffer_frames >= min && request.fixed_buffer_frames <= max
            )
        });
        if !matched {
            return Err(BridgeError::config_unsupported(format!(
                "driver {} does not support exact {} Hz / {} ch / {} / {} frame input",
                request.driver_id,
                request.sample_rate_hz,
                request.input_channels,
                request.sample_format.label(),
                request.fixed_buffer_frames
            )));
        }
        Ok(())
    }

    fn build_typed_stream<T>(
        device: &Device,
        config: StreamConfig,
        request: StartRequest,
        sample_callback: SampleCallback,
        shared: SharedState,
    ) -> Result<Stream, BridgeError>
    where
        T: SizedSample + Copy + Send + 'static,
        f32: FromSample<T>,
    {
        let channels = request.input_channels as usize;
        let mix = request.channel_mix;
        let mut mono = vec![0.0_f32; request.fixed_buffer_frames as usize];
        let data_shared = shared.clone();
        let error_shared = shared;
        device
            .build_input_stream::<T, _, _>(
                config,
                move |data, info| {
                    if !data_shared.active.load(Ordering::Acquire)
                        || data_shared.terminal.load(Ordering::Acquire)
                    {
                        return;
                    }
                    let timestamp = info.timestamp();
                    let Some(capture_delay) =
                        timestamp.callback.checked_duration_since(timestamp.capture)
                    else {
                        data_shared.terminal(
                            EVENT_MALFORMED_CALLBACK,
                            b"ASIO callback timestamp preceded capture timestamp",
                        );
                        return;
                    };
                    let delay_ns = capture_delay.as_nanos().min(u64::MAX as u128) as u64;
                    let _measurement = CallbackMeasurement {
                        telemetry: &data_shared.telemetry,
                        started_at: Instant::now(),
                        capture_delay_ns: delay_ns,
                    };
                    data_shared
                        .last_callback_ns
                        .store(data_shared.now_ns(), Ordering::Release);
                    if channels == 0 || data.len() % channels != 0 {
                        data_shared.terminal(
                            EVENT_MALFORMED_CALLBACK,
                            b"ASIO callback was not aligned to the configured channel count",
                        );
                        return;
                    }
                    let frames = data.len() / channels;
                    let expected = data_shared.expected_frames.load(Ordering::Acquire) as usize;
                    if frames != expected || frames > mono.len() {
                        data_shared.terminal(
                            EVENT_BUFFER_SIZE_CHANGED,
                            b"ASIO callback frame count changed after Start",
                        );
                        return;
                    }

                    for (frame_index, output) in mono[..frames].iter_mut().enumerate() {
                        let frame_offset = frame_index * channels;
                        let mut value = 0.0_f32;
                        for entry in &mix {
                            value += f32::from_sample(data[frame_offset + entry.channel_index])
                                * entry.gain;
                        }
                        if !value.is_finite() {
                            data_shared.terminal(
                                EVENT_MALFORMED_CALLBACK,
                                b"ASIO callback produced a non-finite mono sample",
                            );
                            return;
                        }
                        *output = value.clamp(-1.0, 1.0);
                    }

                    let callback_result = catch_unwind(AssertUnwindSafe(|| unsafe {
                        sample_callback(
                            data_shared.context as *mut c_void,
                            mono.as_ptr(),
                            frames,
                            delay_ns,
                            frames as u32,
                        )
                    }));
                    if callback_result.is_err() {
                        data_shared.terminal(
                            EVENT_BACKEND,
                            b"consumer sample callback attempted to unwind",
                        );
                    }
                },
                move |error| handle_stream_error(&error_shared, error),
                None,
            )
            .map_err(|error| cpal_error("failed to build exact ASIO input stream", error))
    }

    fn handle_stream_error(shared: &SharedState, error: Error) {
        let message = error.message().unwrap_or("ASIO stream error");
        match error.kind() {
            ErrorKind::Xrun => latch_terminal_fault(shared, EVENT_XRUN, b"ASIO input xrun", true),
            ErrorKind::RealtimeDenied => {
                shared.emit(
                    EVENT_WARNING,
                    EVENT_REALTIME_DENIED,
                    b"ASIO realtime scheduling was denied",
                );
            }
            ErrorKind::DeviceNotAvailable | ErrorKind::DeviceChanged => {
                shared.terminal(EVENT_DEVICE_LOST, b"explicit ASIO input device was lost");
            }
            ErrorKind::StreamInvalidated if message.contains("resynchronization") => {
                shared.terminal(EVENT_RESYNC, b"ASIO driver requested resynchronization");
            }
            ErrorKind::StreamInvalidated if message.contains("reset") => {
                latch_terminal_fault(
                    shared,
                    EVENT_RESET,
                    b"ASIO driver requested stream reset",
                    false,
                );
            }
            ErrorKind::StreamInvalidated if message.contains("Sample rate changed") => {
                shared.terminal(
                    EVENT_SAMPLE_RATE_CHANGED,
                    b"ASIO driver changed the configured sample rate",
                );
            }
            _ => shared.terminal(EVENT_BACKEND, b"terminal ASIO backend error"),
        }
    }

    fn latch_terminal_fault(shared: &SharedState, kind: u32, message: &'static [u8], xrun: bool) {
        if xrun {
            shared.xrun_count.fetch_add(1, Ordering::Relaxed);
        }
        shared.terminal(kind, message);
    }

    fn sample_format_label(format: SampleFormat) -> Option<&'static str> {
        match format {
            SampleFormat::F32 => Some("f32"),
            SampleFormat::I16 => Some("i16"),
            SampleFormat::I24 => Some("i24"),
            SampleFormat::I32 => Some("i32"),
            SampleFormat::F64 => Some("f64"),
            _ => None,
        }
    }

    fn to_cpal_sample_format(format: SampleFormatV2) -> SampleFormat {
        match format {
            SampleFormatV2::F32 => SampleFormat::F32,
            SampleFormatV2::I16 => SampleFormat::I16,
            SampleFormatV2::I24 => SampleFormat::I24,
            SampleFormatV2::I32 => SampleFormat::I32,
            SampleFormatV2::F64 => SampleFormat::F64,
        }
    }

    fn cpal_error(context: &str, error: Error) -> BridgeError {
        BridgeError::backend(format!("{context}: {error}"))
    }

    #[cfg(test)]
    mod deterministic_fault_tests {
        use super::*;
        use std::sync::atomic::{AtomicU64, Ordering};

        struct Events {
            terminal: AtomicU64,
            xrun: AtomicU64,
            reset: AtomicU64,
        }

        unsafe extern "C" fn count_event(
            context: *mut c_void,
            severity: u32,
            kind: u32,
            _message: *const u8,
            _message_len: usize,
        ) {
            let events = unsafe { &*context.cast::<Events>() };
            if severity == EVENT_TERMINAL {
                events.terminal.fetch_add(1, Ordering::Relaxed);
            }
            if kind == EVENT_XRUN {
                events.xrun.fetch_add(1, Ordering::Relaxed);
            }
            if kind == EVENT_RESET {
                events.reset.fetch_add(1, Ordering::Relaxed);
            }
        }

        #[test]
        fn event_callback_failure_latches_terminal_without_recursive_emit() {
            let events = Events {
                terminal: AtomicU64::new(0),
                xrun: AtomicU64::new(0),
                reset: AtomicU64::new(0),
            };
            let shared = SharedState::new(count_event, (&events as *const Events) as usize);
            shared.active.store(true, Ordering::Release);

            // A Rust panic may not cross an extern-C boundary. This seam tests
            // the wrapper's catch/latch behavior without invoking undefined
            // FFI unwinding or recursively calling the failed event callback.
            shared.run_client_event_callback(|| panic!("injected client callback failure"));

            assert!(!shared.active.load(Ordering::Acquire));
            assert!(shared.terminal.load(Ordering::Acquire));
            assert_eq!(events.terminal.load(Ordering::Relaxed), 0);
        }

        #[test]
        fn xrun_and_reset_are_terminal_and_latched_once() {
            let events = Events {
                terminal: AtomicU64::new(0),
                xrun: AtomicU64::new(0),
                reset: AtomicU64::new(0),
            };
            let shared = SharedState::new(count_event, (&events as *const Events) as usize);
            shared.active.store(true, Ordering::Release);
            latch_terminal_fault(&shared, EVENT_XRUN, b"injected xrun", true);
            latch_terminal_fault(&shared, EVENT_RESET, b"injected reset", false);
            assert!(shared.terminal.load(Ordering::Acquire));
            assert!(!shared.active.load(Ordering::Acquire));
            assert_eq!(shared.xrun_count.load(Ordering::Acquire), 1);
            assert_eq!(events.terminal.load(Ordering::Relaxed), 1);
            assert_eq!(events.xrun.load(Ordering::Relaxed), 1);
            assert_eq!(events.reset.load(Ordering::Relaxed), 0);

            let reset_events = Events {
                terminal: AtomicU64::new(0),
                xrun: AtomicU64::new(0),
                reset: AtomicU64::new(0),
            };
            let reset = SharedState::new(count_event, (&reset_events as *const Events) as usize);
            latch_terminal_fault(&reset, EVENT_RESET, b"injected reset", false);
            assert_eq!(reset_events.terminal.load(Ordering::Relaxed), 1);
            assert_eq!(reset_events.reset.load(Ordering::Relaxed), 1);
            assert_eq!(reset.xrun_count.load(Ordering::Acquire), 0);
        }

        #[test]
        fn initial_no_callback_gap_fails_closed_at_250_ms() {
            assert!(!callback_gap_due(true, false, 1, CALLBACK_GAP_NS));
            assert!(callback_gap_due(true, false, 1, CALLBACK_GAP_NS + 1));
            assert!(!callback_gap_due(false, false, 1, u64::MAX));
            assert!(!callback_gap_due(true, true, 1, u64::MAX));
            assert!(!callback_gap_due(true, false, 0, u64::MAX));
        }

        #[test]
        fn telemetry_percentiles_are_deterministic_and_bounded() {
            let histogram = AtomicHistogram::new();
            for value in [1, 2, 4, 8, 16, 32, 64, 128, 256, 1_000] {
                histogram.record(value);
            }
            let snapshot = histogram.snapshot(10);
            assert_eq!(snapshot.p50, 31);
            assert_eq!(snapshot.p95, 1_023);
            assert_eq!(snapshot.p99, 1_023);
            assert_eq!(snapshot.max, 1_000);
        }
    }
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
mod backend {
    use super::*;

    pub(super) fn drivers_json() -> Result<CatalogJson, BridgeError> {
        Ok(CatalogJson {
            schema_version: JSON_SCHEMA_VERSION,
            kind: "drivers",
            abi_version: ABI_VERSION,
            backend: "asio",
            built: false,
            drivers: Vec::new(),
        })
    }

    pub(super) fn capabilities_json(_driver_id: &str) -> Result<CapabilitiesJson, BridgeError> {
        Err(not_built())
    }

    pub(super) fn start(
        _request: StartRequest,
        _sample_callback: SampleCallback,
        _event_callback: EventCallback,
        _context: usize,
    ) -> Result<Box<dyn StreamLifecycle>, BridgeError> {
        Err(not_built())
    }

    fn not_built() -> BridgeError {
        BridgeError::unsupported("this DLL was built without the non-default ASIO feature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    };

    unsafe extern "C" fn discard_samples(
        _context: *mut c_void,
        _samples: *const f32,
        _len: usize,
        _capture_delay_ns: u64,
        _callback_frames: u32,
    ) {
    }

    unsafe extern "C" fn discard_event(
        _context: *mut c_void,
        _severity: u32,
        _kind: u32,
        _message: *const u8,
        _message_len: usize,
    ) {
    }

    fn valid_request() -> StartRequestJsonV2 {
        StartRequestJsonV2 {
            schema_version: JSON_SCHEMA_VERSION,
            driver_id: "asio:Unit Test".to_string(),
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: "f32".to_string(),
            fixed_buffer_frames: 128,
            channel_mix: vec![
                ChannelMixJsonV2 {
                    channel_index: 0,
                    gain: 0.5,
                },
                ChannelMixJsonV2 {
                    channel_index: 1,
                    gain: 0.5,
                },
            ],
        }
    }

    fn take_string(value: SyndocalAsioStringV2) -> String {
        if value.ptr.is_null() {
            assert_eq!(value.len, 0);
            return String::new();
        }
        let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
        let text = String::from_utf8(bytes.to_vec()).expect("bridge JSON must be strict UTF-8");
        unsafe { syndocal_asio_v2_string_free(value) };
        text
    }

    fn assert_exported_start_rejects_bytes(request: &[u8], expected_message: &str) {
        let mut handle = ptr::null_mut();
        let mut result = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe {
            syndocal_asio_v2_start(
                request.as_ptr(),
                request.len(),
                Some(discard_samples),
                Some(discard_event),
                ptr::null_mut(),
                &mut handle,
                &mut result,
                &mut error,
            )
        };
        assert_eq!(status, STATUS_INVALID_ARGUMENT);
        assert!(handle.is_null());
        assert!(result.ptr.is_null());
        let error: serde_json::Value = serde_json::from_str(&take_string(error)).unwrap();
        assert_eq!(error["code"], "invalid_argument");
        assert!(
            error["message"]
                .as_str()
                .unwrap()
                .contains(expected_message),
            "unexpected exported Start error: {error}"
        );
    }

    fn assert_exported_start_rejects(request: &StartRequestJsonV2, expected_message: &str) {
        let bytes = serde_json::to_vec(request).unwrap();
        assert_exported_start_rejects_bytes(&bytes, expected_message);
    }

    #[derive(Default)]
    struct FakeCounters {
        stops: AtomicU64,
        closes: AtomicU64,
        drops: AtomicU64,
        fail_stop: AtomicBool,
        fail_close: AtomicBool,
    }

    struct FakeStream {
        counters: Arc<FakeCounters>,
        stopped: bool,
    }

    impl Drop for FakeStream {
        fn drop(&mut self) {
            self.counters.drops.fetch_add(1, Ordering::Relaxed);
        }
    }

    impl StreamLifecycle for FakeStream {
        fn actual_buffer_frames(&self) -> Result<u32, BridgeError> {
            Ok(128)
        }

        fn stop(&mut self) -> Result<bool, BridgeError> {
            self.counters.stops.fetch_add(1, Ordering::Relaxed);
            if self.counters.fail_stop.load(Ordering::Relaxed) {
                return Err(BridgeError::backend("injected stop failure"));
            }
            let was_active = !self.stopped;
            self.stopped = true;
            Ok(was_active)
        }

        fn close(&mut self) -> Result<bool, BridgeError> {
            self.counters.closes.fetch_add(1, Ordering::Relaxed);
            if self.counters.fail_close.load(Ordering::Relaxed) {
                return Err(BridgeError::backend("injected close failure"));
            }
            let was_active = !self.stopped;
            self.stopped = true;
            Ok(was_active)
        }

        fn telemetry(&self) -> TelemetrySnapshot {
            TelemetrySnapshot {
                callbacks: 99,
                xruns: 3,
                callback_duration_ns: PercentilesJsonV2 {
                    p50: 10,
                    p95: 20,
                    p99: 30,
                    max: 40,
                },
                capture_delay_ns: PercentilesJsonV2 {
                    p50: 50,
                    p95: 60,
                    p99: 70,
                    max: 80,
                },
            }
        }
    }

    fn fake_handle(counters: Arc<FakeCounters>) -> *mut SyndocalAsioHandle {
        Box::into_raw(Box::new(SyndocalAsioHandle {
            inner: Box::new(FakeStream {
                counters,
                stopped: false,
            }),
            lease: None,
        }))
    }

    #[test]
    fn abi_version_and_build_identity_are_v2_only() {
        assert_eq!(syndocal_asio_v2_abi_version(), 2);
        assert_eq!(
            syndocal_asio_v2_build_flags(),
            u32::from(cfg!(all(target_os = "windows", feature = "asio")))
        );
        assert_eq!(
            std::mem::size_of::<SyndocalAsioStringV2>(),
            2 * std::mem::size_of::<usize>()
        );
    }

    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    #[test]
    fn sdk_free_catalog_reports_not_built() {
        let mut json = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe { syndocal_asio_v2_drivers_json(&mut json, &mut error) };
        assert_eq!(status, STATUS_OK);
        assert!(error.ptr.is_null());
        let value: serde_json::Value = serde_json::from_str(&take_string(json)).unwrap();
        assert_eq!(value["schemaVersion"], 2);
        assert_eq!(value["kind"], "drivers");
        assert_eq!(value["abiVersion"], 2);
        assert_eq!(value["backend"], "asio");
        assert_eq!(value["built"], false);
        assert_eq!(value["drivers"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn strict_json_rejects_invalid_utf8_unknown_fields_and_future_versions() {
        let invalid_utf8 = [0xff];
        let error = unsafe {
            parse_json::<StartRequestJsonV2>(
                invalid_utf8.as_ptr(),
                invalid_utf8.len(),
                "start request",
            )
        }
        .unwrap_err();
        assert!(error.message.contains("not valid UTF-8"));

        let unknown = br#"{"schemaVersion":2,"driverId":"asio:x","sampleRateHz":48000,"inputChannels":1,"sampleFormat":"f32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1.0}],"legacyFallback":true}"#;
        let error = unsafe {
            parse_json::<StartRequestJsonV2>(unknown.as_ptr(), unknown.len(), "start request")
        }
        .unwrap_err();
        assert!(error.message.contains("unknown field"));

        let mut future = valid_request();
        future.schema_version = 3;
        let error = parse_start_request(future).unwrap_err();
        assert!(error.message.contains("schemaVersion is 3"));

        let unknown_capability =
            br#"{"schemaVersion":2,"driverId":"asio:x","legacyDriverName":"x"}"#;
        let error = unsafe {
            parse_json::<CapabilitiesRequestJsonV2>(
                unknown_capability.as_ptr(),
                unknown_capability.len(),
                "capabilities request",
            )
        }
        .unwrap_err();
        assert!(error.message.contains("unknown field"));
    }

    #[test]
    fn channel_mix_rejects_unbounded_gain_before_start() {
        for gain in [-0.01, 1.000_000_000_1, 1.01, f64::MAX] {
            let mut request = valid_request();
            request.channel_mix[0].gain = gain;
            let error = parse_start_request(request).unwrap_err();
            assert!(error.message.contains("must be between 0 and 1"));
        }

        let mut non_finite = valid_request();
        non_finite.channel_mix[0].gain = f64::INFINITY;
        let error = parse_start_request(non_finite).unwrap_err();
        assert!(error.message.contains("must be finite"));

        for gain in [-0.01, 1.000_000_000_1, f64::MAX] {
            let mut exported = valid_request();
            exported.channel_mix[0].gain = gain;
            assert_exported_start_rejects(&exported, "must be between 0 and 1");
        }
        let non_finite_json = br#"{"schemaVersion":2,"driverId":"asio:Unit Test","sampleRateHz":48000,"inputChannels":2,"sampleFormat":"f32","fixedBufferFrames":128,"channelMix":[{"channelIndex":0,"gain":1e400}]}"#;
        assert_exported_start_rejects_bytes(non_finite_json, "strict ABI v2 JSON");
    }

    #[test]
    fn channel_mix_rejects_empty_and_silent_before_start() {
        let mut empty = valid_request();
        empty.channel_mix.clear();
        let error = parse_start_request(empty).unwrap_err();
        assert!(error.message.contains("channel_mix_len"));

        let mut silent = valid_request();
        for entry in &mut silent.channel_mix {
            entry.gain = 0.0;
        }
        let error = parse_start_request(silent).unwrap_err();
        assert!(error.message.contains("at least one non-zero gain"));

        let mut exported_empty = valid_request();
        exported_empty.channel_mix.clear();
        assert_exported_start_rejects(&exported_empty, "channel_mix_len");

        let mut exported_silent = valid_request();
        for entry in &mut exported_silent.channel_mix {
            entry.gain = 0.0;
        }
        assert_exported_start_rejects(&exported_silent, "at least one non-zero gain");
    }

    #[test]
    fn channel_mix_accepts_supported_single_stereo_and_full_average_shapes() {
        let mut single = valid_request();
        single.channel_mix = vec![ChannelMixJsonV2 {
            channel_index: 1,
            gain: 1.0,
        }];
        parse_start_request(single).expect("single-channel unity mix must be valid");

        parse_start_request(valid_request()).expect("stereo half-gain mix must be valid");

        let mut average = valid_request();
        average.input_channels = MAX_CHANNEL_MIX_ENTRIES as u32;
        average.channel_mix = (0..MAX_CHANNEL_MIX_ENTRIES as u32)
            .map(|channel_index| ChannelMixJsonV2 {
                channel_index,
                gain: 1.0 / MAX_CHANNEL_MIX_ENTRIES as f64,
            })
            .collect();
        parse_start_request(average).expect("256-channel bounded average mix must be valid");
    }

    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    #[test]
    fn sdk_free_start_failure_is_typed_and_never_publishes_a_handle() {
        let request = serde_json::to_vec(&valid_request()).unwrap();
        let mut handle = ptr::null_mut();
        let mut result = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe {
            syndocal_asio_v2_start(
                request.as_ptr(),
                request.len(),
                Some(discard_samples),
                Some(discard_event),
                ptr::null_mut(),
                &mut handle,
                &mut result,
                &mut error,
            )
        };
        assert_eq!(status, STATUS_UNSUPPORTED);
        assert!(handle.is_null());
        assert!(result.ptr.is_null());
        let error: serde_json::Value = serde_json::from_str(&take_string(error)).unwrap();
        assert_eq!(error["schemaVersion"], 2);
        assert_eq!(error["kind"], "error");
        assert_eq!(error["code"], "unsupported");
    }

    #[test]
    fn start_request_values_are_validated_before_backend_access() {
        let mut config = valid_request();
        config.sample_rate_hz = 0;
        let error = parse_start_request(config).unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("sample_rate_hz"));

        let mut config = valid_request();
        config.driver_id = "Unit Test".to_string();
        let error = parse_start_request(config).unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("explicit persistent"));

        let mut config = valid_request();
        config.channel_mix[0].channel_index = 2;
        let error = parse_start_request(config).unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("outside"));
    }

    #[test]
    fn injected_start_failure_returns_no_handle() {
        let request = parse_start_request(valid_request()).unwrap();
        let error = open_stream_with(
            request,
            discard_samples,
            discard_event,
            0,
            |_request, _samples, _events, _context| {
                Err(BridgeError::backend("injected start failure"))
            },
        )
        .err()
        .expect("injected start must fail");
        assert_eq!(error.status, STATUS_BACKEND_ERROR);
        assert!(error.message.contains("injected start failure"));
    }

    #[test]
    fn typed_stop_and_close_report_success_and_release_exactly_once() {
        let counters = Arc::new(FakeCounters::default());
        let mut handle = fake_handle(counters.clone());

        let mut stop_result = SyndocalAsioStringV2::default();
        let mut stop_error = SyndocalAsioStringV2::default();
        let status = unsafe { syndocal_asio_v2_stop(handle, &mut stop_result, &mut stop_error) };
        assert_eq!(status, STATUS_OK);
        assert!(take_string(stop_error).is_empty());
        let result: serde_json::Value = serde_json::from_str(&take_string(stop_result)).unwrap();
        assert_eq!(result["schemaVersion"], 2);
        assert_eq!(result["kind"], "stop");
        assert_eq!(result["stopped"], true);
        assert_eq!(result["streamWasActive"], true);

        let mut close_result = SyndocalAsioStringV2::default();
        let mut close_error = SyndocalAsioStringV2::default();
        let status =
            unsafe { syndocal_asio_v2_close(&mut handle, &mut close_result, &mut close_error) };
        assert_eq!(status, STATUS_OK);
        assert!(handle.is_null());
        assert!(take_string(close_error).is_empty());
        let result: serde_json::Value = serde_json::from_str(&take_string(close_result)).unwrap();
        assert_eq!(result["kind"], "close");
        assert_eq!(result["handleReleased"], true);
        assert_eq!(result["streamWasActive"], false);
        assert_eq!(counters.stops.load(Ordering::Relaxed), 1);
        assert_eq!(counters.closes.load(Ordering::Relaxed), 1);
        assert_eq!(counters.drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn injected_stop_and_close_failures_are_visible_and_close_consumes_handle() {
        let counters = Arc::new(FakeCounters::default());
        counters.fail_stop.store(true, Ordering::Relaxed);
        let mut handle = fake_handle(counters.clone());
        let mut result = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe { syndocal_asio_v2_stop(handle, &mut result, &mut error) };
        assert_eq!(status, STATUS_BACKEND_ERROR);
        assert!(result.ptr.is_null());
        let error: serde_json::Value = serde_json::from_str(&take_string(error)).unwrap();
        assert_eq!(error["kind"], "error");
        assert!(error["message"]
            .as_str()
            .unwrap()
            .contains("injected stop failure"));

        counters.fail_stop.store(false, Ordering::Relaxed);
        counters.fail_close.store(true, Ordering::Relaxed);
        let mut result = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe { syndocal_asio_v2_close(&mut handle, &mut result, &mut error) };
        assert_eq!(status, STATUS_BACKEND_ERROR);
        assert!(handle.is_null());
        assert!(result.ptr.is_null());
        let error: serde_json::Value = serde_json::from_str(&take_string(error)).unwrap();
        assert!(error["message"]
            .as_str()
            .unwrap()
            .contains("injected close failure"));
        assert_eq!(counters.drops.load(Ordering::Relaxed), 1);
    }

    #[test]
    fn telemetry_is_a_typed_v2_snapshot() {
        let counters = Arc::new(FakeCounters::default());
        let mut handle = fake_handle(counters);
        let mut result = SyndocalAsioStringV2::default();
        let mut error = SyndocalAsioStringV2::default();
        let status = unsafe { syndocal_asio_v2_telemetry_json(handle, &mut result, &mut error) };
        assert_eq!(status, STATUS_OK);
        assert!(take_string(error).is_empty());
        let result: serde_json::Value = serde_json::from_str(&take_string(result)).unwrap();
        assert_eq!(result["kind"], "telemetry");
        assert_eq!(result["callbacks"], 99);
        assert_eq!(result["xruns"], 3);
        assert_eq!(result["callbackDurationNs"]["p99"], 30);

        let mut close_result = SyndocalAsioStringV2::default();
        let mut close_error = SyndocalAsioStringV2::default();
        assert_eq!(
            unsafe { syndocal_asio_v2_close(&mut handle, &mut close_result, &mut close_error) },
            STATUS_OK
        );
        take_string(close_result);
        take_string(close_error);
    }

    #[cfg(all(target_os = "windows", feature = "asio"))]
    mod hardware {
        use super::*;
        use std::{
            env, ptr,
            sync::atomic::{AtomicU32, AtomicU64, Ordering},
            thread,
            time::{Duration, Instant},
        };

        const CALLBACK_TARGET: u64 = 2;
        const CALLBACK_TIMEOUT: Duration = Duration::from_secs(2);
        const EVENT_WARNING_SEVERITY: u32 = 1;
        const EVENT_TERMINAL_SEVERITY: u32 = 2;
        const EVENT_XRUN_KIND: u32 = 1;

        struct CallbackCounters {
            expected_frames: AtomicU32,
            callbacks: AtomicU64,
            frame_mismatches: AtomicU64,
            nonfinite_samples: AtomicU64,
            terminal_events: AtomicU64,
            xrun_events: AtomicU64,
            warning_events: AtomicU64,
        }

        impl CallbackCounters {
            fn new(expected_frames: u32) -> Self {
                Self {
                    expected_frames: AtomicU32::new(expected_frames),
                    callbacks: AtomicU64::new(0),
                    frame_mismatches: AtomicU64::new(0),
                    nonfinite_samples: AtomicU64::new(0),
                    terminal_events: AtomicU64::new(0),
                    xrun_events: AtomicU64::new(0),
                    warning_events: AtomicU64::new(0),
                }
            }
        }

        unsafe extern "C" fn count_samples(
            context: *mut c_void,
            mono_samples: *const f32,
            len: usize,
            _capture_delay_ns: u64,
            callback_frames: u32,
        ) {
            if context.is_null() {
                return;
            }
            let counters = unsafe { &*context.cast::<CallbackCounters>() };
            counters.callbacks.fetch_add(1, Ordering::Relaxed);
            let expected_frames = counters.expected_frames.load(Ordering::Relaxed);
            if mono_samples.is_null()
                || len == 0
                || callback_frames != expected_frames
                || len != callback_frames as usize
            {
                counters.frame_mismatches.fetch_add(1, Ordering::Relaxed);
                return;
            }
            let samples = unsafe { slice::from_raw_parts(mono_samples, len) };
            let nonfinite = samples.iter().filter(|sample| !sample.is_finite()).count() as u64;
            if nonfinite != 0 {
                counters
                    .nonfinite_samples
                    .fetch_add(nonfinite, Ordering::Relaxed);
            }
        }

        unsafe extern "C" fn count_events(
            context: *mut c_void,
            severity: u32,
            kind: u32,
            _message: *const u8,
            _message_len: usize,
        ) {
            if context.is_null() {
                return;
            }
            let counters = unsafe { &*context.cast::<CallbackCounters>() };
            if severity == EVENT_TERMINAL_SEVERITY {
                counters.terminal_events.fetch_add(1, Ordering::Relaxed);
            }
            if severity == EVENT_WARNING_SEVERITY {
                counters.warning_events.fetch_add(1, Ordering::Relaxed);
            }
            if kind == EVENT_XRUN_KIND {
                counters.xrun_events.fetch_add(1, Ordering::Relaxed);
            }
        }

        struct OwnedHardwareHandle(*mut SyndocalAsioHandle);

        impl OwnedHardwareHandle {
            fn as_ptr(&self) -> *mut SyndocalAsioHandle {
                self.0
            }

            fn close(mut self) -> (u32, String, String) {
                let mut result = SyndocalAsioStringV2::default();
                let mut error = SyndocalAsioStringV2::default();
                let status =
                    unsafe { syndocal_asio_v2_close(&mut self.0, &mut result, &mut error) };
                (
                    status,
                    take_bridge_string(result),
                    take_bridge_string(error),
                )
            }
        }

        fn required_env(name: &str) -> String {
            let value = env::var(name)
                .unwrap_or_else(|_| panic!("{name} must be set explicitly for this ignored test"));
            assert!(!value.trim().is_empty(), "{name} must not be empty");
            value
        }

        fn required_u32(name: &str) -> u32 {
            let raw = required_env(name);
            let value = raw
                .parse::<u32>()
                .unwrap_or_else(|error| panic!("{name}={raw:?} is not a u32: {error}"));
            assert!(value != 0, "{name} must be greater than zero");
            value
        }

        fn take_bridge_string(value: SyndocalAsioStringV2) -> String {
            if value.ptr.is_null() {
                assert_eq!(value.len, 0, "null bridge string had a non-zero length");
                return String::new();
            }
            assert!(value.len != 0, "non-null bridge string had a zero length");
            let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
            let text = String::from_utf8(bytes.to_vec()).expect("bridge output must be UTF-8");
            unsafe { syndocal_asio_v2_string_free(value) };
            text
        }

        fn exact_capability(
            driver_id: &str,
            sample_rate_hz: u32,
            input_channels: u32,
            sample_format: &str,
            buffer_frames: u32,
        ) {
            let mut catalog_json = SyndocalAsioStringV2::default();
            let mut catalog_error = SyndocalAsioStringV2::default();
            let catalog_status =
                unsafe { syndocal_asio_v2_drivers_json(&mut catalog_json, &mut catalog_error) };
            let catalog_error = take_bridge_string(catalog_error);
            let catalog_text = take_bridge_string(catalog_json);
            assert_eq!(
                catalog_status, STATUS_OK,
                "ASIO catalog failed: {catalog_error}"
            );
            assert!(catalog_error.is_empty());
            let catalog: serde_json::Value = serde_json::from_str(&catalog_text)
                .unwrap_or_else(|error| panic!("ASIO catalog JSON was invalid: {error}"));
            assert_eq!(catalog["built"], true);
            assert!(
                catalog["drivers"]
                    .as_array()
                    .is_some_and(|drivers| drivers.iter().any(|driver| driver["id"] == driver_id)),
                "explicit driver {driver_id:?} was not present; catalog={catalog_text}"
            );

            let request = serde_json::to_vec(&CapabilitiesRequestJsonV2 {
                schema_version: JSON_SCHEMA_VERSION,
                driver_id: driver_id.to_string(),
            })
            .unwrap();
            let mut capabilities_json = SyndocalAsioStringV2::default();
            let mut capabilities_error = SyndocalAsioStringV2::default();
            let capabilities_status = unsafe {
                syndocal_asio_v2_capabilities_json(
                    request.as_ptr(),
                    request.len(),
                    &mut capabilities_json,
                    &mut capabilities_error,
                )
            };
            let capabilities_error = take_bridge_string(capabilities_error);
            let capabilities_text = take_bridge_string(capabilities_json);
            assert_eq!(
                capabilities_status, STATUS_OK,
                "ASIO capabilities failed: {capabilities_error}"
            );
            assert!(capabilities_error.is_empty());
            let capabilities: serde_json::Value = serde_json::from_str(&capabilities_text)
                .unwrap_or_else(|error| panic!("ASIO capabilities JSON was invalid: {error}"));
            assert_eq!(capabilities["driver"]["id"], driver_id);
            let exact = capabilities["inputConfigs"]
                .as_array()
                .is_some_and(|configs| {
                    configs.iter().any(|config| {
                        config["channels"] == input_channels
                            && config["sampleFormat"] == sample_format
                            && config["sampleRateHz"] == sample_rate_hz
                            && config["bufferFrames"]["min"]
                                .as_u64()
                                .is_some_and(|min| min <= u64::from(buffer_frames))
                            && config["bufferFrames"]["max"]
                                .as_u64()
                                .is_some_and(|max| max >= u64::from(buffer_frames))
                    })
                });
            assert!(
                exact,
                "explicit configuration was not advertised; capabilities={capabilities_text}"
            );
        }

        #[test]
        #[ignore = "requires an explicit physical ASIO input driver and local licensed SDK"]
        fn explicit_asio_start_stop_cycles_are_exact_and_clean() {
            let driver_id = required_env("SYNDOCAL_ASIO_TEST_DRIVER_ID");
            assert!(
                driver_id.starts_with("asio:") && driver_id.len() > "asio:".len(),
                "SYNDOCAL_ASIO_TEST_DRIVER_ID must be an explicit asio:<driver name> ID"
            );
            let sample_rate_hz = required_u32("SYNDOCAL_ASIO_TEST_SAMPLE_RATE_HZ");
            let input_channels = required_u32("SYNDOCAL_ASIO_TEST_INPUT_CHANNELS");
            assert!(
                input_channels <= 256,
                "SYNDOCAL_ASIO_TEST_INPUT_CHANNELS must not exceed 256"
            );
            let sample_format_label = required_env("SYNDOCAL_ASIO_TEST_SAMPLE_FORMAT");
            assert!(
                matches!(
                    sample_format_label.as_str(),
                    "f32" | "i16" | "i24" | "i32" | "f64"
                ),
                "SYNDOCAL_ASIO_TEST_SAMPLE_FORMAT must be f32, i16, i24, i32, or f64"
            );
            let buffer_frames = required_u32("SYNDOCAL_ASIO_TEST_BUFFER_FRAMES");
            let cycles = required_u32("SYNDOCAL_ASIO_TEST_CYCLES");
            assert!(
                cycles <= 10_000,
                "SYNDOCAL_ASIO_TEST_CYCLES must not exceed 10000"
            );

            exact_capability(
                &driver_id,
                sample_rate_hz,
                input_channels,
                &sample_format_label,
                buffer_frames,
            );

            let mix_gain = 1.0 / input_channels as f64;
            let channel_mix = (0..input_channels)
                .map(|channel_index| ChannelMixJsonV2 {
                    channel_index,
                    gain: mix_gain,
                })
                .collect::<Vec<_>>();
            let started_at = Instant::now();
            let mut total_callbacks = 0_u64;
            let mut total_warnings = 0_u64;
            let mut closed_cycles = 0_u32;

            for cycle in 1..=cycles {
                let counters = Box::new(CallbackCounters::new(buffer_frames));
                let context = (&*counters as *const CallbackCounters)
                    .cast_mut()
                    .cast::<c_void>();
                let config = serde_json::to_vec(&StartRequestJsonV2 {
                    schema_version: JSON_SCHEMA_VERSION,
                    driver_id: driver_id.clone(),
                    sample_rate_hz,
                    input_channels,
                    sample_format: sample_format_label.clone(),
                    fixed_buffer_frames: buffer_frames,
                    channel_mix: channel_mix.clone(),
                })
                .unwrap();
                let mut raw_handle = ptr::null_mut();
                let mut start_result = SyndocalAsioStringV2::default();
                let mut start_error = SyndocalAsioStringV2::default();
                let start_status = unsafe {
                    syndocal_asio_v2_start(
                        config.as_ptr(),
                        config.len(),
                        Some(count_samples),
                        Some(count_events),
                        context,
                        &mut raw_handle,
                        &mut start_result,
                        &mut start_error,
                    )
                };
                let start_result = take_bridge_string(start_result);
                let start_error = take_bridge_string(start_error);
                let handle = OwnedHardwareHandle(raw_handle);
                assert_eq!(
                    start_status, STATUS_OK,
                    "cycle {cycle}/{cycles} Start failed without fallback: {start_error}"
                );
                assert!(start_error.is_empty());
                assert!(!handle.as_ptr().is_null());
                let start_result: serde_json::Value = serde_json::from_str(&start_result).unwrap();
                let actual_buffer_frames =
                    start_result["actualBufferFrames"].as_u64().unwrap() as u32;

                let deadline = Instant::now() + CALLBACK_TIMEOUT;
                while counters.callbacks.load(Ordering::Relaxed) < CALLBACK_TARGET
                    && counters.terminal_events.load(Ordering::Relaxed) == 0
                    && Instant::now() < deadline
                {
                    thread::sleep(Duration::from_millis(1));
                }

                let mut stop_result = SyndocalAsioStringV2::default();
                let mut stop_error = SyndocalAsioStringV2::default();
                let stop_status = unsafe {
                    syndocal_asio_v2_stop(handle.as_ptr(), &mut stop_result, &mut stop_error)
                };
                let stop_result = take_bridge_string(stop_result);
                let stop_error = take_bridge_string(stop_error);
                let mut telemetry = SyndocalAsioStringV2::default();
                let mut telemetry_error = SyndocalAsioStringV2::default();
                let telemetry_status = unsafe {
                    syndocal_asio_v2_telemetry_json(
                        handle.as_ptr(),
                        &mut telemetry,
                        &mut telemetry_error,
                    )
                };
                let telemetry_error = take_bridge_string(telemetry_error);
                let telemetry: serde_json::Value =
                    serde_json::from_str(&take_bridge_string(telemetry)).unwrap();
                let xrun_count = telemetry["xruns"].as_u64().unwrap();
                let (close_status, close_result, close_error) = handle.close();
                closed_cycles += 1;

                let callback_count = counters.callbacks.load(Ordering::Relaxed);
                let frame_mismatches = counters.frame_mismatches.load(Ordering::Relaxed);
                let nonfinite_samples = counters.nonfinite_samples.load(Ordering::Relaxed);
                let terminal_events = counters.terminal_events.load(Ordering::Relaxed);
                let xrun_events = counters.xrun_events.load(Ordering::Relaxed);
                let warning_events = counters.warning_events.load(Ordering::Relaxed);
                total_callbacks += callback_count;
                total_warnings += warning_events;

                assert_eq!(
                    actual_buffer_frames, buffer_frames,
                    "cycle {cycle}/{cycles} applied a different hardware buffer"
                );
                assert!(
                    callback_count >= CALLBACK_TARGET,
                    "cycle {cycle}/{cycles} delivered {callback_count} callbacks in {CALLBACK_TIMEOUT:?}"
                );
                assert_eq!(
                    frame_mismatches, 0,
                    "cycle {cycle}/{cycles} callback frames differed from {buffer_frames}"
                );
                assert_eq!(
                    nonfinite_samples, 0,
                    "cycle {cycle}/{cycles} delivered non-finite samples"
                );
                assert_eq!(
                    terminal_events, 0,
                    "cycle {cycle}/{cycles} emitted a terminal event"
                );
                assert_eq!(
                    xrun_events, 0,
                    "cycle {cycle}/{cycles} emitted an xrun event"
                );
                assert_eq!(xrun_count, 0, "cycle {cycle}/{cycles} reported xruns");
                assert_eq!(
                    telemetry_status, STATUS_OK,
                    "telemetry failed: {telemetry_error}"
                );
                assert!(telemetry_error.is_empty());
                assert_eq!(
                    stop_status, STATUS_OK,
                    "cycle {cycle}/{cycles} Stop failed: {stop_error}"
                );
                let stop_result: serde_json::Value = serde_json::from_str(&stop_result).unwrap();
                assert_eq!(stop_result["stopped"], true);
                assert!(stop_error.is_empty());
                assert_eq!(
                    close_status, STATUS_OK,
                    "cycle {cycle}/{cycles} Close failed: {close_error}"
                );
                assert!(close_error.is_empty());
                let close_result: serde_json::Value = serde_json::from_str(&close_result).unwrap();
                assert_eq!(close_result["handleReleased"], true);

                if cycle % 10 == 0 || cycle == cycles {
                    eprintln!(
                        "ASIO hardware stability: {cycle}/{cycles} cycles, callbacks={total_callbacks}, warnings={total_warnings}, elapsed={:?}",
                        started_at.elapsed()
                    );
                }
            }

            assert_eq!(closed_cycles, cycles, "not every ASIO handle was closed");
            eprintln!(
                "ASIO hardware stability PASS: driver={driver_id:?}, rate={sample_rate_hz}, channels={input_channels}, format={sample_format_label}, requested_buffer={buffer_frames}, actual_buffer_each_cycle={buffer_frames}, cycles={cycles}, callbacks={total_callbacks}, stops={cycles}, closes={closed_cycles}, warnings={total_warnings}, terminal=0, xruns=0, nonfinite=0, elapsed={:?}",
                started_at.elapsed()
            );
        }
    }
}
