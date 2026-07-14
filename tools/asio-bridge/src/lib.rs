#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(all(target_os = "windows", feature = "asio")), allow(dead_code))]

use serde::Serialize;
use std::{
    ffi::c_void,
    mem::size_of,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr, slice, str,
};

const ABI_VERSION: u32 = 1;
const STATUS_OK: u32 = 0;
const STATUS_INVALID_ARGUMENT: u32 = 1;
#[cfg(not(all(target_os = "windows", feature = "asio")))]
const STATUS_UNSUPPORTED: u32 = 2;
const STATUS_DRIVER_NOT_FOUND: u32 = 3;
const STATUS_CONFIG_UNSUPPORTED: u32 = 4;
const STATUS_BACKEND_ERROR: u32 = 5;
const STATUS_TERMINAL: u32 = 6;
const STATUS_PANIC: u32 = 255;

const SAMPLE_F32: u32 = 1;
const SAMPLE_I16: u32 = 2;
const SAMPLE_I24: u32 = 3;
const SAMPLE_I32: u32 = 4;
const SAMPLE_F64: u32 = 5;

const MAX_DRIVER_ID_BYTES: usize = 4_096;
const MAX_CHANNEL_MIX_ENTRIES: usize = 256;
const MAX_FIXED_BUFFER_FRAMES: u32 = 1_048_576;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyndocalAsioStringV1 {
    pub ptr: *mut u8,
    pub len: usize,
}

impl Default for SyndocalAsioStringV1 {
    fn default() -> Self {
        Self {
            ptr: ptr::null_mut(),
            len: 0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyndocalAsioChannelMixV1 {
    pub channel_index: u32,
    pub gain: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct SyndocalAsioStartConfigV1 {
    pub struct_size: u32,
    pub abi_version: u32,
    pub driver_id: *const u8,
    pub driver_id_len: usize,
    pub sample_rate_hz: u32,
    pub input_channels: u32,
    pub sample_format: u32,
    pub fixed_buffer_frames: u32,
    pub channel_mix: *const SyndocalAsioChannelMixV1,
    pub channel_mix_len: usize,
    pub flags: u32,
    pub reserved: u32,
}

pub type SyndocalAsioSampleCallbackV1 = Option<
    unsafe extern "C" fn(
        context: *mut c_void,
        mono_samples: *const f32,
        len: usize,
        capture_delay_ns: u64,
        callback_frames: u32,
    ),
>;

pub type SyndocalAsioEventCallbackV1 = Option<
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SampleFormatV1 {
    F32,
    I16,
    I24,
    I32,
    F64,
}

impl SampleFormatV1 {
    fn parse(value: u32) -> Option<Self> {
        match value {
            SAMPLE_F32 => Some(Self::F32),
            SAMPLE_I16 => Some(Self::I16),
            SAMPLE_I24 => Some(Self::I24),
            SAMPLE_I32 => Some(Self::I32),
            SAMPLE_F64 => Some(Self::F64),
            _ => None,
        }
    }

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

#[derive(Clone, Debug)]
struct ChannelMix {
    channel_index: usize,
    gain: f32,
}

#[derive(Clone, Debug)]
struct StartRequest {
    driver_id: String,
    sample_rate_hz: u32,
    input_channels: u16,
    sample_format: SampleFormatV1,
    fixed_buffer_frames: u32,
    channel_mix: Vec<ChannelMix>,
}

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

    fn driver_not_found(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_DRIVER_NOT_FOUND,
            code: "driver_not_found",
            message: message.into(),
        }
    }

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

    fn terminal(message: impl Into<String>) -> Self {
        Self {
            status: STATUS_TERMINAL,
            code: "terminal",
            message: message.into(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ErrorJson<'a> {
    status: u32,
    code: &'a str,
    message: &'a str,
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
    abi_version: u32,
    backend: &'static str,
    built: bool,
    driver: DriverJson,
    input_configs: Vec<InputConfigJson>,
}

pub struct SyndocalAsioHandle {
    inner: backend::StreamHandle,
}

fn owned_bytes(bytes: Vec<u8>) -> SyndocalAsioStringV1 {
    let boxed = bytes.into_boxed_slice();
    let len = boxed.len();
    let ptr = Box::into_raw(boxed).cast::<u8>();
    SyndocalAsioStringV1 { ptr, len }
}

unsafe fn clear_string(output: *mut SyndocalAsioStringV1) {
    if !output.is_null() {
        unsafe { output.write(SyndocalAsioStringV1::default()) };
    }
}

unsafe fn write_string(output: *mut SyndocalAsioStringV1, bytes: Vec<u8>) {
    unsafe { output.write(owned_bytes(bytes)) };
}

fn write_error(output: *mut SyndocalAsioStringV1, error: &BridgeError) {
    if output.is_null() {
        return;
    }
    let payload = ErrorJson {
        status: error.status,
        code: error.code,
        message: &error.message,
    };
    if let Ok(bytes) = serde_json::to_vec(&payload) {
        unsafe { write_string(output, bytes) };
    }
}

fn ffi_guard<F>(out_error: *mut SyndocalAsioStringV1, operation: F) -> u32
where
    F: FnOnce() -> Result<(), BridgeError>,
{
    unsafe { clear_string(out_error) };
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => STATUS_OK,
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

unsafe fn utf8_from_raw<'a>(
    pointer: *const u8,
    len: usize,
    field: &str,
) -> Result<&'a str, BridgeError> {
    if pointer.is_null() {
        return Err(BridgeError::invalid(format!("{field} pointer is null")));
    }
    if len == 0 || len > MAX_DRIVER_ID_BYTES {
        return Err(BridgeError::invalid(format!(
            "{field} length must be between 1 and {MAX_DRIVER_ID_BYTES} bytes"
        )));
    }
    let bytes = unsafe { slice::from_raw_parts(pointer, len) };
    str::from_utf8(bytes).map_err(|_| BridgeError::invalid(format!("{field} is not valid UTF-8")))
}

unsafe fn parse_start_request(
    config: *const SyndocalAsioStartConfigV1,
) -> Result<StartRequest, BridgeError> {
    if config.is_null() {
        return Err(BridgeError::invalid("start config pointer is null"));
    }

    let reported_size = unsafe { ptr::read_unaligned(config.cast::<u32>()) } as usize;
    if reported_size < size_of::<SyndocalAsioStartConfigV1>() {
        return Err(BridgeError::invalid(format!(
            "start config struct_size is {reported_size}, expected at least {}",
            size_of::<SyndocalAsioStartConfigV1>()
        )));
    }
    let config = unsafe { &*config };
    if config.abi_version != ABI_VERSION {
        return Err(BridgeError::invalid(format!(
            "start config abi_version is {}, expected {ABI_VERSION}",
            config.abi_version
        )));
    }
    if config.flags != 0 || config.reserved != 0 {
        return Err(BridgeError::invalid(
            "start config flags and reserved must be zero",
        ));
    }

    let driver_id = unsafe { utf8_from_raw(config.driver_id, config.driver_id_len, "driver_id") }?;
    if !driver_id.starts_with("asio:") || driver_id.len() <= "asio:".len() {
        return Err(BridgeError::invalid(
            "driver_id must be an explicit persistent asio:<driver name> ID",
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
    let sample_format = SampleFormatV1::parse(config.sample_format)
        .ok_or_else(|| BridgeError::invalid("sample_format is not an ABI v1 input format"))?;
    if config.fixed_buffer_frames == 0 || config.fixed_buffer_frames > MAX_FIXED_BUFFER_FRAMES {
        return Err(BridgeError::invalid(format!(
            "fixed_buffer_frames must be between 1 and {MAX_FIXED_BUFFER_FRAMES}"
        )));
    }
    if config.channel_mix.is_null() {
        return Err(BridgeError::invalid("channel_mix pointer is null"));
    }
    if config.channel_mix_len == 0 || config.channel_mix_len > MAX_CHANNEL_MIX_ENTRIES {
        return Err(BridgeError::invalid(format!(
            "channel_mix_len must be between 1 and {MAX_CHANNEL_MIX_ENTRIES}"
        )));
    }

    let raw_mix = unsafe { slice::from_raw_parts(config.channel_mix, config.channel_mix_len) };
    let mut channel_mix = Vec::with_capacity(raw_mix.len());
    let mut has_audible_gain = false;
    for entry in raw_mix {
        if entry.channel_index >= config.input_channels {
            return Err(BridgeError::invalid(format!(
                "channel mix index {} is outside the {} configured input channels",
                entry.channel_index, config.input_channels
            )));
        }
        if !entry.gain.is_finite() {
            return Err(BridgeError::invalid("channel mix gains must be finite"));
        }
        has_audible_gain |= entry.gain != 0.0;
        channel_mix.push(ChannelMix {
            channel_index: entry.channel_index as usize,
            gain: entry.gain,
        });
    }
    if !has_audible_gain {
        return Err(BridgeError::invalid(
            "channel mix must contain at least one non-zero gain",
        ));
    }

    Ok(StartRequest {
        driver_id: driver_id.to_owned(),
        sample_rate_hz: config.sample_rate_hz,
        input_channels: config.input_channels as u16,
        sample_format,
        fixed_buffer_frames: config.fixed_buffer_frames,
        channel_mix,
    })
}

#[no_mangle]
pub extern "C" fn syndocal_asio_abi_version() -> u32 {
    ABI_VERSION
}

#[no_mangle]
pub extern "C" fn syndocal_asio_build_flags() -> u32 {
    if cfg!(all(target_os = "windows", feature = "asio")) {
        1
    } else {
        0
    }
}

#[no_mangle]
/// Returns the current driver catalog as an owned UTF-8 JSON byte string.
///
/// # Safety
///
/// `out_json` must be valid and writable. `out_error_json`, when non-null, must also be valid and
/// writable and must not alias `out_json`. Both outputs must be empty; overwriting a string that
/// has not been released would leak it.
pub unsafe extern "C" fn syndocal_asio_drivers_json(
    out_json: *mut SyndocalAsioStringV1,
    out_error_json: *mut SyndocalAsioStringV1,
) -> u32 {
    ffi_guard(out_error_json, || {
        if out_json.is_null() {
            return Err(BridgeError::invalid("out_json pointer is null"));
        }
        if out_json == out_error_json {
            return Err(BridgeError::invalid(
                "out_json and out_error_json must not alias",
            ));
        }
        unsafe { clear_string(out_json) };
        let json = backend::drivers_json()?;
        let bytes = serde_json::to_vec(&json)
            .map_err(|error| BridgeError::backend(format!("driver JSON failed: {error}")))?;
        unsafe { write_string(out_json, bytes) };
        Ok(())
    })
}

#[no_mangle]
/// Returns exact input capabilities for one explicit persistent ASIO driver ID.
///
/// # Safety
///
/// `driver_id` must reference `driver_id_len` readable bytes for this call. `out_json` must be
/// valid and writable. `out_error_json`, when non-null, must be valid and writable and must not
/// alias `out_json`. Both outputs must be empty.
pub unsafe extern "C" fn syndocal_asio_capabilities_json(
    driver_id: *const u8,
    driver_id_len: usize,
    out_json: *mut SyndocalAsioStringV1,
    out_error_json: *mut SyndocalAsioStringV1,
) -> u32 {
    ffi_guard(out_error_json, || {
        if out_json.is_null() {
            return Err(BridgeError::invalid("out_json pointer is null"));
        }
        if out_json == out_error_json {
            return Err(BridgeError::invalid(
                "out_json and out_error_json must not alias",
            ));
        }
        unsafe { clear_string(out_json) };
        let driver_id = unsafe { utf8_from_raw(driver_id, driver_id_len, "driver_id") }?;
        if !driver_id.starts_with("asio:") || driver_id.len() <= "asio:".len() {
            return Err(BridgeError::invalid(
                "driver_id must be an explicit persistent asio:<driver name> ID",
            ));
        }
        let json = backend::capabilities_json(driver_id)?;
        let bytes = serde_json::to_vec(&json)
            .map_err(|error| BridgeError::backend(format!("capability JSON failed: {error}")))?;
        unsafe { write_string(out_json, bytes) };
        Ok(())
    })
}

#[no_mangle]
/// Releases an owned UTF-8 string returned by this exact loaded bridge DLL.
///
/// # Safety
///
/// A non-empty string must have been returned by this DLL and must be released exactly once. The
/// DLL must remain loaded until the release completes.
pub unsafe extern "C" fn syndocal_asio_string_free(string: SyndocalAsioStringV1) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if string.ptr.is_null() || string.len == 0 {
            return;
        }
        let raw = ptr::slice_from_raw_parts_mut(string.ptr, string.len);
        unsafe { drop(Box::from_raw(raw)) };
    }));
}

#[no_mangle]
/// Revalidates, builds, and starts an exact ASIO input stream.
///
/// # Safety
///
/// `config` and every pointer/length pair it contains must remain readable for this call.
/// `out_handle` must be valid and writable, and `out_error_json`, when non-null, must be empty,
/// valid, and writable. Callbacks and `context` must remain valid until the handle is freed; client
/// callbacks must not unwind, block, or call bridge lifecycle functions.
pub unsafe extern "C" fn syndocal_asio_start(
    config: *const SyndocalAsioStartConfigV1,
    sample_callback: SyndocalAsioSampleCallbackV1,
    event_callback: SyndocalAsioEventCallbackV1,
    context: *mut c_void,
    out_handle: *mut *mut SyndocalAsioHandle,
    out_error_json: *mut SyndocalAsioStringV1,
) -> u32 {
    ffi_guard(out_error_json, || {
        if out_handle.is_null() {
            return Err(BridgeError::invalid("out_handle pointer is null"));
        }
        unsafe { out_handle.write(ptr::null_mut()) };
        let sample_callback = sample_callback
            .ok_or_else(|| BridgeError::invalid("sample_callback must not be null"))?;
        let event_callback = event_callback
            .ok_or_else(|| BridgeError::invalid("event_callback must not be null"))?;
        let request = unsafe { parse_start_request(config) }?;
        let inner = backend::start(request, sample_callback, event_callback, context as usize)?;
        let handle = Box::new(SyndocalAsioHandle { inner });
        unsafe { out_handle.write(Box::into_raw(handle)) };
        Ok(())
    })
}

#[no_mangle]
/// Returns the fixed hardware buffer applied when the stream was created.
///
/// # Safety
///
/// `handle` must be a live handle returned by this DLL and `out_frames` must be valid and writable.
/// Neither may be used concurrently with `syndocal_asio_free`.
pub unsafe extern "C" fn syndocal_asio_actual_buffer_frames(
    handle: *const SyndocalAsioHandle,
    out_frames: *mut u32,
) -> u32 {
    match catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() || out_frames.is_null() {
            return Err(BridgeError::invalid(
                "handle and out_frames must not be null",
            ));
        }
        let frames = unsafe { (*handle).inner.actual_buffer_frames() }?;
        unsafe { out_frames.write(frames) };
        Ok(())
    })) {
        Ok(Ok(())) => STATUS_OK,
        Ok(Err(error)) => error.status,
        Err(_) => STATUS_PANIC,
    }
}

#[no_mangle]
/// Resumes a stopped, non-terminal stream.
///
/// # Safety
///
/// `handle` must be a live handle returned by this DLL and exclusively available for this call.
/// `out_error_json`, when non-null, must be empty, valid, and writable.
pub unsafe extern "C" fn syndocal_asio_play(
    handle: *mut SyndocalAsioHandle,
    out_error_json: *mut SyndocalAsioStringV1,
) -> u32 {
    ffi_guard(out_error_json, || {
        if handle.is_null() {
            return Err(BridgeError::invalid("handle must not be null"));
        }
        unsafe { (*handle).inner.play() }
    })
}

#[no_mangle]
/// Stops delivery and pauses a live stream without selecting any fallback.
///
/// # Safety
///
/// `handle` must be a live handle returned by this DLL and exclusively available for this call.
/// `out_error_json`, when non-null, must be empty, valid, and writable.
pub unsafe extern "C" fn syndocal_asio_stop(
    handle: *mut SyndocalAsioHandle,
    out_error_json: *mut SyndocalAsioStringV1,
) -> u32 {
    ffi_guard(out_error_json, || {
        if handle.is_null() {
            return Err(BridgeError::invalid("handle must not be null"));
        }
        unsafe { (*handle).inner.stop() }
    })
}

#[no_mangle]
/// Returns the number of xruns observed by this stream.
///
/// # Safety
///
/// A non-null `handle` must be a live handle returned by this DLL and must not be used concurrently
/// with `syndocal_asio_free`.
pub unsafe extern "C" fn syndocal_asio_xrun_count(handle: *const SyndocalAsioHandle) -> u64 {
    catch_unwind(AssertUnwindSafe(|| {
        if handle.is_null() {
            0
        } else {
            unsafe { (*handle).inner.xrun_count() }
        }
    }))
    .unwrap_or(0)
}

#[no_mangle]
/// Stops and releases a bridge stream handle.
///
/// # Safety
///
/// A non-null `handle` must have been returned by this exact loaded DLL, must be exclusively owned,
/// and must be freed exactly once. Do not call this function from a bridge callback.
pub unsafe extern "C" fn syndocal_asio_free(handle: *mut SyndocalAsioHandle) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if !handle.is_null() {
            unsafe { drop(Box::from_raw(handle)) };
        }
    }));
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

    #[derive(Clone)]
    struct SharedState {
        active: Arc<AtomicBool>,
        terminal: Arc<AtomicBool>,
        seen_callback: Arc<AtomicBool>,
        last_callback_ns: Arc<AtomicU64>,
        expected_frames: Arc<AtomicU32>,
        xrun_count: Arc<AtomicU64>,
        origin: Instant,
        event_callback: EventCallback,
        context: usize,
    }

    impl SharedState {
        fn new(event_callback: EventCallback, context: usize) -> Self {
            Self {
                active: Arc::new(AtomicBool::new(false)),
                terminal: Arc::new(AtomicBool::new(false)),
                seen_callback: Arc::new(AtomicBool::new(false)),
                last_callback_ns: Arc::new(AtomicU64::new(0)),
                expected_frames: Arc::new(AtomicU32::new(0)),
                xrun_count: Arc::new(AtomicU64::new(0)),
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
            let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
                callback(context, severity, kind, message.as_ptr(), message.len())
            }));
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
                        if !shared.active.load(Ordering::Acquire)
                            || shared.terminal.load(Ordering::Acquire)
                        {
                            thread::sleep(Duration::from_millis(25));
                            continue;
                        }
                        let last = shared.last_callback_ns.load(Ordering::Acquire);
                        if last == 0 {
                            thread::sleep(Duration::from_millis(1));
                            continue;
                        }
                        let elapsed = shared.now_ns().saturating_sub(last);
                        if elapsed >= CALLBACK_GAP_NS {
                            shared.terminal(
                                EVENT_CALLBACK_GAP,
                                b"ASIO input callback gap exceeded 250 ms",
                            );
                        } else {
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
        _watchdog: Watchdog,
    }

    impl StreamHandle {
        pub(super) fn actual_buffer_frames(&self) -> Result<u32, BridgeError> {
            Ok(self.applied_buffer_frames)
        }

        pub(super) fn play(&mut self) -> Result<(), BridgeError> {
            if self.shared.terminal.load(Ordering::Acquire) {
                return Err(BridgeError::terminal(
                    "terminal ASIO stream cannot resume; free it and start explicitly",
                ));
            }
            self.shared.seen_callback.store(false, Ordering::Release);
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

        pub(super) fn stop(&mut self) -> Result<(), BridgeError> {
            self.shared.active.store(false, Ordering::Release);
            self.stream
                .pause()
                .map_err(|error| cpal_error("failed to pause ASIO input stream", error))
        }

        pub(super) fn xrun_count(&self) -> u64 {
            self.shared.xrun_count.load(Ordering::Acquire)
        }
    }

    impl Drop for StreamHandle {
        fn drop(&mut self) {
            self.shared.active.store(false, Ordering::Release);
            let _ = self.stream.pause();
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
    ) -> Result<StreamHandle, BridgeError> {
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
            SampleFormatV1::F32 => build_typed_stream::<f32>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV1::I16 => build_typed_stream::<i16>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV1::I24 => build_typed_stream::<I24>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV1::I32 => build_typed_stream::<i32>(
                &device,
                stream_config,
                request.clone(),
                sample_callback,
                shared.clone(),
            ),
            SampleFormatV1::F64 => build_typed_stream::<f64>(
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
            _watchdog: watchdog,
        };
        handle.play()?;
        Ok(handle)
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

                    data_shared
                        .last_callback_ns
                        .store(data_shared.now_ns(), Ordering::Release);
                    data_shared.seen_callback.store(true, Ordering::Release);

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

                    let timestamp = info.timestamp();
                    let delay_ns = timestamp
                        .callback
                        .checked_duration_since(timestamp.capture)
                        .unwrap_or_default()
                        .as_nanos()
                        .min(u64::MAX as u128) as u64;
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
            ErrorKind::Xrun => {
                shared.xrun_count.fetch_add(1, Ordering::Relaxed);
                shared.terminal(EVENT_XRUN, b"ASIO input xrun");
            }
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
                shared.terminal(EVENT_RESET, b"ASIO driver requested stream reset");
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

    fn to_cpal_sample_format(format: SampleFormatV1) -> SampleFormat {
        match format {
            SampleFormatV1::F32 => SampleFormat::F32,
            SampleFormatV1::I16 => SampleFormat::I16,
            SampleFormatV1::I24 => SampleFormat::I24,
            SampleFormatV1::I32 => SampleFormat::I32,
            SampleFormatV1::F64 => SampleFormat::F64,
        }
    }

    fn cpal_error(context: &str, error: Error) -> BridgeError {
        BridgeError::backend(format!("{context}: {error}"))
    }
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
mod backend {
    use super::*;

    pub(super) struct StreamHandle;

    impl StreamHandle {
        pub(super) fn actual_buffer_frames(&self) -> Result<u32, BridgeError> {
            Err(not_built())
        }

        pub(super) fn play(&mut self) -> Result<(), BridgeError> {
            Err(not_built())
        }

        pub(super) fn stop(&mut self) -> Result<(), BridgeError> {
            Err(not_built())
        }

        pub(super) fn xrun_count(&self) -> u64 {
            0
        }
    }

    pub(super) fn drivers_json() -> Result<CatalogJson, BridgeError> {
        Ok(CatalogJson {
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
    ) -> Result<StreamHandle, BridgeError> {
        Err(not_built())
    }

    fn not_built() -> BridgeError {
        BridgeError::unsupported("this DLL was built without the non-default ASIO feature")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn abi_version_and_struct_size_are_stable() {
        assert_eq!(syndocal_asio_abi_version(), 1);
        assert_eq!(
            syndocal_asio_build_flags(),
            u32::from(cfg!(all(target_os = "windows", feature = "asio")))
        );
        let expected_size = if usize::BITS == 64 { 64 } else { 48 };
        assert_eq!(size_of::<SyndocalAsioStartConfigV1>(), expected_size);
    }

    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    #[test]
    fn sdk_free_catalog_reports_not_built() {
        let mut json = SyndocalAsioStringV1::default();
        let mut error = SyndocalAsioStringV1::default();
        let status = unsafe { syndocal_asio_drivers_json(&mut json, &mut error) };
        assert_eq!(status, STATUS_OK);
        assert!(error.ptr.is_null());
        let bytes = unsafe { slice::from_raw_parts(json.ptr, json.len) };
        let value: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(value["abiVersion"], 1);
        assert_eq!(value["backend"], "asio");
        assert_eq!(value["built"], false);
        assert_eq!(value["drivers"].as_array().unwrap().len(), 0);
        unsafe { syndocal_asio_string_free(json) };
    }

    #[test]
    fn zero_sample_rate_is_rejected_before_backend_access() {
        let driver = b"asio:Unit Test";
        let mix = [SyndocalAsioChannelMixV1 {
            channel_index: 0,
            gain: 1.0,
        }];
        let config = SyndocalAsioStartConfigV1 {
            struct_size: size_of::<SyndocalAsioStartConfigV1>() as u32,
            abi_version: ABI_VERSION,
            driver_id: driver.as_ptr(),
            driver_id_len: driver.len(),
            sample_rate_hz: 0,
            input_channels: 1,
            sample_format: SAMPLE_F32,
            fixed_buffer_frames: 128,
            channel_mix: mix.as_ptr(),
            channel_mix_len: mix.len(),
            flags: 0,
            reserved: 0,
        };
        let error = unsafe { parse_start_request(&config) }.unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("sample_rate_hz"));
    }

    #[test]
    fn raw_driver_name_is_rejected() {
        let driver = b"Unit Test";
        let mix = [SyndocalAsioChannelMixV1 {
            channel_index: 0,
            gain: 1.0,
        }];
        let config = SyndocalAsioStartConfigV1 {
            struct_size: size_of::<SyndocalAsioStartConfigV1>() as u32,
            abi_version: ABI_VERSION,
            driver_id: driver.as_ptr(),
            driver_id_len: driver.len(),
            sample_rate_hz: 48_000,
            input_channels: 1,
            sample_format: SAMPLE_F32,
            fixed_buffer_frames: 128,
            channel_mix: mix.as_ptr(),
            channel_mix_len: mix.len(),
            flags: 0,
            reserved: 0,
        };
        let error = unsafe { parse_start_request(&config) }.unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("explicit persistent"));
    }

    #[test]
    fn mix_index_and_gain_are_validated() {
        let driver = b"asio:Unit Test";
        let mix = [SyndocalAsioChannelMixV1 {
            channel_index: 2,
            gain: f32::NAN,
        }];
        let config = SyndocalAsioStartConfigV1 {
            struct_size: size_of::<SyndocalAsioStartConfigV1>() as u32,
            abi_version: ABI_VERSION,
            driver_id: driver.as_ptr(),
            driver_id_len: driver.len(),
            sample_rate_hz: 48_000,
            input_channels: 2,
            sample_format: SAMPLE_F32,
            fixed_buffer_frames: 128,
            channel_mix: mix.as_ptr(),
            channel_mix_len: mix.len(),
            flags: 0,
            reserved: 0,
        };
        let error = unsafe { parse_start_request(&config) }.unwrap_err();
        assert_eq!(error.status, STATUS_INVALID_ARGUMENT);
        assert!(error.message.contains("outside"));
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

            fn free(mut self) {
                if !self.0.is_null() {
                    unsafe { syndocal_asio_free(self.0) };
                    self.0 = ptr::null_mut();
                }
            }
        }

        impl Drop for OwnedHardwareHandle {
            fn drop(&mut self) {
                if !self.0.is_null() {
                    unsafe { syndocal_asio_free(self.0) };
                    self.0 = ptr::null_mut();
                }
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

        fn take_bridge_string(value: SyndocalAsioStringV1) -> String {
            if value.ptr.is_null() {
                assert_eq!(value.len, 0, "null bridge string had a non-zero length");
                return String::new();
            }
            assert!(value.len != 0, "non-null bridge string had a zero length");
            let bytes = unsafe { slice::from_raw_parts(value.ptr, value.len) };
            let text = String::from_utf8_lossy(bytes).into_owned();
            unsafe { syndocal_asio_string_free(value) };
            text
        }

        fn exact_capability(
            driver_id: &str,
            sample_rate_hz: u32,
            input_channels: u32,
            sample_format: &str,
            buffer_frames: u32,
        ) {
            let mut catalog_json = SyndocalAsioStringV1::default();
            let mut catalog_error = SyndocalAsioStringV1::default();
            let catalog_status =
                unsafe { syndocal_asio_drivers_json(&mut catalog_json, &mut catalog_error) };
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

            let mut capabilities_json = SyndocalAsioStringV1::default();
            let mut capabilities_error = SyndocalAsioStringV1::default();
            let capabilities_status = unsafe {
                syndocal_asio_capabilities_json(
                    driver_id.as_ptr(),
                    driver_id.len(),
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
            let sample_format = match sample_format_label.as_str() {
                "f32" => SAMPLE_F32,
                "i16" => SAMPLE_I16,
                "i24" => SAMPLE_I24,
                "i32" => SAMPLE_I32,
                "f64" => SAMPLE_F64,
                _ => panic!("SYNDOCAL_ASIO_TEST_SAMPLE_FORMAT must be f32, i16, i24, i32, or f64"),
            };
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

            let mix_gain = 1.0 / input_channels as f32;
            let channel_mix = (0..input_channels)
                .map(|channel_index| SyndocalAsioChannelMixV1 {
                    channel_index,
                    gain: mix_gain,
                })
                .collect::<Vec<_>>();
            let started_at = Instant::now();
            let mut total_callbacks = 0_u64;
            let mut total_warnings = 0_u64;
            let mut freed_cycles = 0_u32;

            for cycle in 1..=cycles {
                let counters = Box::new(CallbackCounters::new(buffer_frames));
                let context = (&*counters as *const CallbackCounters)
                    .cast_mut()
                    .cast::<c_void>();
                let config = SyndocalAsioStartConfigV1 {
                    struct_size: size_of::<SyndocalAsioStartConfigV1>() as u32,
                    abi_version: ABI_VERSION,
                    driver_id: driver_id.as_ptr(),
                    driver_id_len: driver_id.len(),
                    sample_rate_hz,
                    input_channels,
                    sample_format,
                    fixed_buffer_frames: buffer_frames,
                    channel_mix: channel_mix.as_ptr(),
                    channel_mix_len: channel_mix.len(),
                    flags: 0,
                    reserved: 0,
                };
                let mut raw_handle = ptr::null_mut();
                let mut start_error = SyndocalAsioStringV1::default();
                let start_status = unsafe {
                    syndocal_asio_start(
                        &config,
                        Some(count_samples),
                        Some(count_events),
                        context,
                        &mut raw_handle,
                        &mut start_error,
                    )
                };
                let start_error = take_bridge_string(start_error);
                let handle = OwnedHardwareHandle(raw_handle);
                assert_eq!(
                    start_status, STATUS_OK,
                    "cycle {cycle}/{cycles} Start failed without fallback: {start_error}"
                );
                assert!(start_error.is_empty());
                assert!(!handle.as_ptr().is_null());

                let mut actual_buffer_frames = 0_u32;
                let actual_status = unsafe {
                    syndocal_asio_actual_buffer_frames(handle.as_ptr(), &mut actual_buffer_frames)
                };

                let deadline = Instant::now() + CALLBACK_TIMEOUT;
                while counters.callbacks.load(Ordering::Relaxed) < CALLBACK_TARGET
                    && counters.terminal_events.load(Ordering::Relaxed) == 0
                    && Instant::now() < deadline
                {
                    thread::sleep(Duration::from_millis(1));
                }

                let mut stop_error = SyndocalAsioStringV1::default();
                let stop_status = unsafe { syndocal_asio_stop(handle.as_ptr(), &mut stop_error) };
                let stop_error = take_bridge_string(stop_error);
                let xrun_count = unsafe { syndocal_asio_xrun_count(handle.as_ptr()) };
                handle.free();
                freed_cycles += 1;

                let callback_count = counters.callbacks.load(Ordering::Relaxed);
                let frame_mismatches = counters.frame_mismatches.load(Ordering::Relaxed);
                let nonfinite_samples = counters.nonfinite_samples.load(Ordering::Relaxed);
                let terminal_events = counters.terminal_events.load(Ordering::Relaxed);
                let xrun_events = counters.xrun_events.load(Ordering::Relaxed);
                let warning_events = counters.warning_events.load(Ordering::Relaxed);
                total_callbacks += callback_count;
                total_warnings += warning_events;

                assert_eq!(
                    actual_status, STATUS_OK,
                    "cycle {cycle}/{cycles} actual-buffer query failed"
                );
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
                    stop_status, STATUS_OK,
                    "cycle {cycle}/{cycles} Stop failed: {stop_error}"
                );
                assert!(stop_error.is_empty());

                if cycle % 10 == 0 || cycle == cycles {
                    eprintln!(
                        "ASIO hardware stability: {cycle}/{cycles} cycles, callbacks={total_callbacks}, warnings={total_warnings}, elapsed={:?}",
                        started_at.elapsed()
                    );
                }
            }

            assert_eq!(freed_cycles, cycles, "not every ASIO handle was freed");
            eprintln!(
                "ASIO hardware stability PASS: driver={driver_id:?}, rate={sample_rate_hz}, channels={input_channels}, format={sample_format_label}, requested_buffer={buffer_frames}, actual_buffer_each_cycle={buffer_frames}, cycles={cycles}, callbacks={total_callbacks}, stops={cycles}, frees={freed_cycles}, warnings={total_warnings}, terminal=0, xruns=0, nonfinite=0, elapsed={:?}",
                started_at.elapsed()
            );
        }
    }
}
