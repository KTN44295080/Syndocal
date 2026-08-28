//! Purpose-built ASIO v3 output/full-duplex backend.
//!
//! The SDK callback table is installed by `v3_sdk_backend.cpp`; it never enters
//! CPAL or asio-sys' callback registries. This layer owns the normalized f32
//! staging buffers, native conversion, generation fence, event notifier, and
//! process-wide v2/v3 host lease.

#[cfg(not(all(target_os = "windows", feature = "asio")))]
use crate::v3_abi::STATUS_UNSUPPORTED_V3;
use crate::v3_abi::{
    BridgeErrorV3, CloseResultJsonV3, StartRequestV3, StartResultJsonV3, StopResultJsonV3,
    SyndocalAsioDuplexCallbackV3, SyndocalAsioEventCallbackV3, SyndocalAsioOutputCallbackV3,
    TelemetryJsonV3, JSON_SCHEMA_VERSION_V3,
};
#[cfg(all(target_os = "windows", feature = "asio"))]
use crate::{
    asio_host_lease::{self, Inspection, Owner, StartGuard, Ticket},
    v3_abi::{
        validate_shared_clock_v3, StreamTupleJsonV3, EVENT_BACKEND_V3,
        EVENT_BUFFER_SIZE_CHANGED_V3, EVENT_CALLBACK_GAP_V3, EVENT_CALLBACK_PANIC_V3,
        EVENT_DEVICE_LOST_V3, EVENT_INVALID_OUTPUT_BLOCK_V3, EVENT_MALFORMED_CALLBACK_V3,
        EVENT_OUTPUT_QUEUE_FULL_V3, EVENT_OUTPUT_QUEUE_UNDERFLOW_V3, EVENT_RESET_V3,
        EVENT_RESYNC_V3, EVENT_SAMPLE_RATE_CHANGED_V3, EVENT_TERMINAL_V3, EVENT_XRUN_V3,
        STATUS_TERMINAL_V3,
    },
};
#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
use crate::{
    v3_abi::{
        StreamModeV3, CALLBACK_ACCEPTED_V3, CALLBACK_INVALID_BLOCK_V3, CALLBACK_PANIC_V3,
        STATUS_BACKEND_ERROR_V3,
    },
    v3_native::{
        interleaved_to_native_planar, native_planar_to_interleaved, silence_native_planar,
        NativeFormat,
    },
};
use serde::Serialize;
use std::ffi::c_void;
#[cfg(all(target_os = "windows", feature = "asio"))]
use std::{
    ffi::{c_char, CStr, CString},
    sync::Arc,
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};
#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    ptr,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

#[cfg(all(target_os = "windows", feature = "asio"))]
const CALLBACK_GAP: Duration = Duration::from_millis(250);
#[cfg(all(target_os = "windows", feature = "asio"))]
const WATCHDOG_POLL: Duration = Duration::from_millis(25);
#[cfg(all(target_os = "windows", feature = "asio"))]
const ERROR_BYTES: usize = 512;
#[cfg(all(target_os = "windows", feature = "asio"))]
const DRIVER_SLOTS: usize = 128;
#[cfg(all(target_os = "windows", feature = "asio"))]
const DRIVER_SLOT_BYTES: usize = 128;

#[derive(Clone, Copy)]
pub(crate) struct CallbackContractV3 {
    pub(crate) output: SyndocalAsioOutputCallbackV3,
    pub(crate) duplex: SyndocalAsioDuplexCallbackV3,
    pub(crate) event: SyndocalAsioEventCallbackV3,
    pub(crate) context: *mut c_void,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DriverJsonV3 {
    id: String,
    name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogJsonV3 {
    schema_version: u32,
    kind: &'static str,
    abi_version: u32,
    backend: &'static str,
    built: bool,
    drivers: Vec<DriverJsonV3>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BufferConstraintsJsonV3 {
    min: u32,
    max: u32,
    preferred: u32,
    granularity: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CapabilityTupleJsonV3 {
    channels: u32,
    native_format: &'static str,
    sample_rates_hz: Vec<u32>,
    fixed_buffer_frames: BufferConstraintsJsonV3,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilitiesJsonV3 {
    schema_version: u32,
    kind: &'static str,
    abi_version: u32,
    backend: &'static str,
    built: bool,
    driver: DriverJsonV3,
    output: CapabilityTupleJsonV3,
    input: Option<CapabilityTupleJsonV3>,
}

#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
struct CallbackState {
    mode: StreamModeV3,
    output_callback: SyndocalAsioOutputCallbackV3,
    duplex_callback: SyndocalAsioDuplexCallbackV3,
    caller_context: usize,
    output_channels: usize,
    input_channels: usize,
    frames: usize,
    session_generation: u64,
    render_generation: u64,
    expected_first_output_frame: AtomicU64,
    render_enabled: AtomicBool,
    output_staging: Vec<f32>,
    input_staging: Vec<f32>,
}

#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
#[derive(Clone, Copy)]
struct ProcessBlock {
    input_buffers: *const *const c_void,
    output_buffers: *const *mut c_void,
    input_format_code: u32,
    output_format_code: u32,
    frames: u32,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
}

#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
impl CallbackState {
    fn new(
        request: &StartRequestV3,
        callbacks: CallbackContractV3,
    ) -> Result<Box<Self>, BridgeErrorV3> {
        let output_samples = (request.output.channels as usize)
            .checked_mul(request.output.fixed_buffer_frames as usize)
            .ok_or_else(|| backend_error("output callback block size overflowed"))?;
        let input_channels = request
            .input
            .as_ref()
            .map(|input| input.channels as usize)
            .unwrap_or(0);
        let input_samples = input_channels
            .checked_mul(request.output.fixed_buffer_frames as usize)
            .ok_or_else(|| backend_error("input callback block size overflowed"))?;
        let mut output_staging = Vec::new();
        output_staging
            .try_reserve_exact(output_samples)
            .map_err(|_| backend_error("output callback staging allocation failed"))?;
        output_staging.resize(output_samples, 0.0);
        let mut input_staging = Vec::new();
        input_staging
            .try_reserve_exact(input_samples)
            .map_err(|_| backend_error("input callback staging allocation failed"))?;
        input_staging.resize(input_samples, 0.0);
        Ok(Box::new(Self {
            mode: request.mode,
            output_callback: callbacks.output,
            duplex_callback: callbacks.duplex,
            caller_context: callbacks.context as usize,
            output_channels: request.output.channels as usize,
            input_channels,
            frames: request.output.fixed_buffer_frames as usize,
            session_generation: request.session_generation,
            render_generation: request.render_generation,
            expected_first_output_frame: AtomicU64::new(request.first_output_frame),
            render_enabled: AtomicBool::new(true),
            output_staging,
            input_staging,
        }))
    }

    unsafe fn process(&mut self, block: ProcessBlock) -> u32 {
        let Some(output_format) = NativeFormat::from_code(block.output_format_code) else {
            return CALLBACK_INVALID_BLOCK_V3;
        };
        if !self.render_enabled.load(Ordering::Acquire)
            || block.frames as usize != self.frames
            || block.session_generation != self.session_generation
            || block.render_generation != self.render_generation
            || self
                .expected_first_output_frame
                .compare_exchange(
                    block.first_output_frame,
                    block.first_output_frame.saturating_add(block.frames as u64),
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_err()
        {
            let _ = unsafe {
                silence_native_planar(
                    block.output_buffers,
                    self.output_channels,
                    self.frames,
                    output_format,
                )
            };
            return CALLBACK_INVALID_BLOCK_V3;
        }

        self.output_staging.fill(f32::NAN);
        if self.mode == StreamModeV3::FullDuplex {
            let Some(input_format) = NativeFormat::from_code(block.input_format_code) else {
                let _ = unsafe {
                    silence_native_planar(
                        block.output_buffers,
                        self.output_channels,
                        self.frames,
                        output_format,
                    )
                };
                return CALLBACK_INVALID_BLOCK_V3;
            };
            if !unsafe {
                native_planar_to_interleaved(
                    block.input_buffers,
                    self.input_channels,
                    self.frames,
                    input_format,
                    &mut self.input_staging,
                )
            } {
                let _ = unsafe {
                    silence_native_planar(
                        block.output_buffers,
                        self.output_channels,
                        self.frames,
                        output_format,
                    )
                };
                return CALLBACK_INVALID_BLOCK_V3;
            }
        }

        let callback_result = catch_unwind(AssertUnwindSafe(|| unsafe {
            match self.mode {
                StreamModeV3::OutputOnly => {
                    self.output_callback
                        .map_or(CALLBACK_INVALID_BLOCK_V3, |callback| {
                            callback(
                                self.caller_context as *mut c_void,
                                self.output_staging.as_mut_ptr(),
                                self.output_channels as u32,
                                block.frames,
                                block.first_output_frame,
                                block.session_generation,
                                block.render_generation,
                            )
                        })
                }
                StreamModeV3::FullDuplex => {
                    self.duplex_callback
                        .map_or(CALLBACK_INVALID_BLOCK_V3, |callback| {
                            callback(
                                self.caller_context as *mut c_void,
                                self.input_staging.as_ptr(),
                                self.input_channels as u32,
                                self.output_staging.as_mut_ptr(),
                                self.output_channels as u32,
                                block.frames,
                                block.first_output_frame,
                                block.session_generation,
                                block.render_generation,
                            )
                        })
                }
            }
        }));
        let result = match callback_result {
            Ok(result @ 0..=5) => result,
            Ok(_) => CALLBACK_INVALID_BLOCK_V3,
            Err(_) => CALLBACK_PANIC_V3,
        };
        if result != CALLBACK_ACCEPTED_V3
            || self.output_staging.iter().any(|sample| !sample.is_finite())
            || !unsafe {
                interleaved_to_native_planar(
                    &self.output_staging,
                    block.output_buffers,
                    self.output_channels,
                    self.frames,
                    output_format,
                )
            }
        {
            let _ = unsafe {
                silence_native_planar(
                    block.output_buffers,
                    self.output_channels,
                    self.frames,
                    output_format,
                )
            };
            return if result == CALLBACK_ACCEPTED_V3 {
                CALLBACK_INVALID_BLOCK_V3
            } else {
                result
            };
        }
        CALLBACK_ACCEPTED_V3
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
unsafe extern "C" fn process_callback(
    context: *mut c_void,
    input_buffers: *const *const c_void,
    output_buffers: *const *mut c_void,
    input_format: u32,
    output_format: u32,
    frames: u32,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
) -> u32 {
    if context.is_null() {
        return CALLBACK_INVALID_BLOCK_V3;
    }
    match catch_unwind(AssertUnwindSafe(|| unsafe {
        (&mut *context.cast::<CallbackState>()).process(ProcessBlock {
            input_buffers,
            output_buffers,
            input_format_code: input_format,
            output_format_code: output_format,
            frames,
            first_output_frame,
            session_generation,
            render_generation,
        })
    })) {
        Ok(result) => result,
        Err(_) => CALLBACK_PANIC_V3,
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
struct Watchdog {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

#[cfg(all(target_os = "windows", feature = "asio"))]
impl Watchdog {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    fn spawn(
        session: *mut c_void,
        ticket: Ticket,
        event_callback: SyndocalAsioEventCallbackV3,
        caller_context: usize,
    ) -> Result<Self, BridgeErrorV3> {
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let session_address = session as usize;
        let thread = thread::Builder::new()
            .name("syndocal-asio-v3-watchdog".to_string())
            .spawn(move || {
                let session = session_address as *mut c_void;
                let mut last_callbacks = 0_u64;
                let mut last_change = Instant::now();
                let mut event_sent = false;
                while !thread_stop.load(Ordering::Acquire) {
                    let telemetry = sdk::telemetry(session);
                    if telemetry.callbacks != last_callbacks {
                        last_callbacks = telemetry.callbacks;
                        last_change = Instant::now();
                    } else if telemetry.running != 0
                        && telemetry.terminal_kind == 0
                        && last_change.elapsed() >= CALLBACK_GAP
                    {
                        unsafe { sdk::latch_terminal(session, EVENT_CALLBACK_GAP_V3) };
                    }
                    let telemetry = sdk::telemetry(session);
                    if telemetry.terminal_kind != 0 && !event_sent {
                        event_sent = true;
                        asio_host_lease::mark_fault(ticket);
                        if let Some(callback) = event_callback {
                            let message = terminal_message(telemetry.terminal_kind);
                            let _ = catch_unwind(AssertUnwindSafe(|| unsafe {
                                callback(
                                    caller_context as *mut c_void,
                                    EVENT_TERMINAL_V3,
                                    telemetry.terminal_kind,
                                    message.as_ptr(),
                                    message.len(),
                                )
                            }));
                        }
                    }
                    thread::sleep(WATCHDOG_POLL);
                }
            })
            .map_err(|error| backend_error(format!("failed to start ASIO v3 watchdog: {error}")))?;
        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }

    fn stop_and_join(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(thread) = self.thread.take() {
            if thread.thread().id() != thread::current().id() {
                let _ = thread.join();
            }
        }
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
impl Drop for Watchdog {
    fn drop(&mut self) {
        self.stop_and_join();
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub struct SyndocalAsioV3Handle {
    native_session: *mut c_void,
    callback_state: Box<CallbackState>,
    ticket: Ticket,
    watchdog: Watchdog,
    stopped: bool,
    lease_released: bool,
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
pub struct SyndocalAsioV3Handle {
    _unavailable: (),
}

#[cfg(all(target_os = "windows", feature = "asio"))]
unsafe impl Send for SyndocalAsioV3Handle {}

#[cfg(all(target_os = "windows", feature = "asio"))]
impl Drop for SyndocalAsioV3Handle {
    fn drop(&mut self) {
        self.callback_state
            .render_enabled
            .store(false, Ordering::Release);
        self.watchdog.stop_and_join();
        #[cfg(all(target_os = "windows", feature = "asio"))]
        let close_succeeded = self.native_session.is_null()
            || unsafe { sdk::close(&mut self.native_session) }.is_ok();
        #[cfg(not(all(target_os = "windows", feature = "asio")))]
        let close_succeeded = true;
        if !self.lease_released {
            let _ = asio_host_lease::begin_stop(self.ticket);
            if close_succeeded {
                asio_host_lease::stop_succeeded(self.ticket);
            } else {
                asio_host_lease::stop_failed(self.ticket);
            }
            self.lease_released = true;
        }
        self.stopped = true;
    }
}

pub(crate) fn built() -> bool {
    cfg!(all(target_os = "windows", feature = "asio"))
}

pub(crate) fn build_flags() -> u32 {
    u32::from(built())
}

pub(crate) fn drivers_json() -> Result<CatalogJsonV3, BridgeErrorV3> {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    {
        let inspection = Inspection::begin().map_err(busy_error)?;
        let result = sdk::driver_names().map(|names| CatalogJsonV3 {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "drivers",
            abi_version: 3,
            backend: "asio-sdk-v3-rt",
            built: true,
            drivers: names
                .into_iter()
                .map(|name| DriverJsonV3 {
                    id: format!("asio:{name}"),
                    name,
                })
                .collect(),
        });
        inspection.complete();
        result
    }
    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    {
        Ok(CatalogJsonV3 {
            schema_version: JSON_SCHEMA_VERSION_V3,
            kind: "drivers",
            abi_version: 3,
            backend: "asio-sdk-v3-rt",
            built: false,
            drivers: Vec::new(),
        })
    }
}

pub(crate) fn capabilities_json(driver_id: &str) -> Result<CapabilitiesJsonV3, BridgeErrorV3> {
    #[cfg(all(target_os = "windows", feature = "asio"))]
    {
        let inspection = Inspection::begin().map_err(busy_error)?;
        let name = driver_id
            .strip_prefix("asio:")
            .ok_or_else(|| backend_error("driverId is not an explicit ASIO driver"))?;
        let result = sdk::capabilities(name).and_then(|capabilities| {
            let output_format = NativeFormat::from_code(capabilities.output_format)
                .ok_or_else(|| backend_error("driver output native format is unsupported"))?;
            let input_format = if capabilities.input_channels == 0 {
                None
            } else {
                Some(
                    NativeFormat::from_code(capabilities.input_format).ok_or_else(|| {
                        backend_error("driver input native format is unsupported")
                    })?,
                )
            };
            let rates = capabilities.supported_rates();
            if rates.is_empty() {
                return Err(backend_error(
                    "driver exposes no supported exact sample rate",
                ));
            }
            let buffers = || BufferConstraintsJsonV3 {
                min: capabilities.buffer_min_frames,
                max: capabilities.buffer_max_frames,
                preferred: capabilities.buffer_preferred_frames,
                granularity: capabilities.buffer_granularity,
            };
            Ok(CapabilitiesJsonV3 {
                schema_version: JSON_SCHEMA_VERSION_V3,
                kind: "capabilities",
                abi_version: 3,
                backend: "asio-sdk-v3-rt",
                built: true,
                driver: DriverJsonV3 {
                    id: driver_id.to_string(),
                    name: name.to_string(),
                },
                output: CapabilityTupleJsonV3 {
                    channels: capabilities.output_channels,
                    native_format: output_format.label(),
                    sample_rates_hz: rates.clone(),
                    fixed_buffer_frames: buffers(),
                },
                input: input_format.map(|format| CapabilityTupleJsonV3 {
                    channels: capabilities.input_channels,
                    native_format: format.label(),
                    sample_rates_hz: rates,
                    fixed_buffer_frames: buffers(),
                }),
            })
        });
        inspection.complete();
        result
    }
    #[cfg(not(all(target_os = "windows", feature = "asio")))]
    {
        let _ = driver_id;
        Err(BridgeErrorV3::new(
            STATUS_UNSUPPORTED_V3,
            "v3_rt_backend_unavailable",
            "ASIO v3 playback is unavailable because this DLL was built without the non-default ASIO feature",
        ))
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub(crate) fn start(
    request: StartRequestV3,
    callbacks: CallbackContractV3,
) -> Result<(SyndocalAsioV3Handle, StartResultJsonV3), BridgeErrorV3> {
    validate_shared_clock_v3(request.mode, &request.output, request.input.as_ref())?;
    let guard = StartGuard::begin(Owner::V3).map_err(busy_error)?;
    let mut callback_state = CallbackState::new(&request, callbacks)?;
    let name = request
        .driver_id
        .strip_prefix("asio:")
        .ok_or_else(|| backend_error("driverId is not an explicit ASIO driver"))?;
    let native_request = sdk::StartConfig {
        driver_name: CString::new(name)
            .map_err(|_| backend_error("ASIO driver name contains a null byte"))?,
        output_channels: request.output.channels,
        input_channels: request
            .input
            .as_ref()
            .map(|input| input.channels)
            .unwrap_or(0),
        output_format: NativeFormat::request_code(&request.output.sample_format)
            .ok_or_else(|| backend_error("output native format is unsupported"))?,
        input_format: request
            .input
            .as_ref()
            .and_then(|input| NativeFormat::request_code(&input.sample_format))
            .unwrap_or(0),
        input_sample_rate_hz: request
            .input
            .as_ref()
            .map(|input| input.sample_rate_hz)
            .unwrap_or(0),
        input_fixed_buffer_frames: request
            .input
            .as_ref()
            .map(|input| input.fixed_buffer_frames)
            .unwrap_or(0),
        sample_rate_hz: request.output.sample_rate_hz,
        fixed_buffer_frames: request.output.fixed_buffer_frames,
        first_output_frame: request.first_output_frame,
        session_generation: request.session_generation,
        render_generation: request.render_generation,
    };
    let (mut native_session, actual) = match unsafe {
        sdk::start(
            &native_request,
            callback_state.as_mut() as *mut CallbackState,
        )
    } {
        Ok(value) => value,
        Err(error) => {
            return Err(error);
        }
    };
    let exact = actual.output_channels == request.output.channels
        && actual.input_channels
            == request
                .input
                .as_ref()
                .map(|input| input.channels)
                .unwrap_or(0)
        && actual.sample_rate_hz == request.output.sample_rate_hz
        && actual.fixed_buffer_frames == request.output.fixed_buffer_frames
        && NativeFormat::from_code(actual.output_format).map(NativeFormat::label)
            == Some(request.output.sample_format.as_str())
        && match request.input.as_ref() {
            None => actual.input_format == 0,
            Some(input) => {
                NativeFormat::from_code(actual.input_format).map(NativeFormat::label)
                    == Some(input.sample_format.as_str())
            }
        };
    if !exact {
        let _ = unsafe { sdk::close(&mut native_session) };
        return Err(backend_error(
            "ASIO opened tuple differed from the exact v3 Start request",
        ));
    }
    let ticket = guard.activate();
    let watchdog = match Watchdog::spawn(
        native_session,
        ticket,
        callbacks.event,
        callbacks.context as usize,
    ) {
        Ok(watchdog) => watchdog,
        Err(error) => {
            let _ = asio_host_lease::begin_stop(ticket);
            callback_state
                .render_enabled
                .store(false, Ordering::Release);
            let close_result = unsafe { sdk::close(&mut native_session) };
            if close_result.is_ok() {
                asio_host_lease::stop_succeeded(ticket);
            } else {
                asio_host_lease::stop_failed(ticket);
            }
            return Err(error);
        }
    };
    let actual_output = StreamTupleJsonV3 {
        channels: actual.output_channels,
        sample_format: NativeFormat::from_code(actual.output_format)
            .expect("exact output native format was revalidated")
            .label()
            .to_owned(),
        sample_rate_hz: actual.sample_rate_hz,
        fixed_buffer_frames: actual.fixed_buffer_frames,
    };
    let actual_input = if actual.input_channels == 0 {
        None
    } else {
        Some(StreamTupleJsonV3 {
            channels: actual.input_channels,
            sample_format: NativeFormat::from_code(actual.input_format)
                .expect("exact input native format was revalidated")
                .label()
                .to_owned(),
            sample_rate_hz: actual.sample_rate_hz,
            fixed_buffer_frames: actual.fixed_buffer_frames,
        })
    };
    let result = StartResultJsonV3::new(
        actual_output,
        actual_input,
        request.first_output_frame,
        request.session_generation,
        request.render_generation,
    );
    Ok((
        SyndocalAsioV3Handle {
            native_session,
            callback_state,
            ticket,
            watchdog,
            stopped: false,
            lease_released: false,
        },
        result,
    ))
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
pub(crate) fn start(
    request: StartRequestV3,
    callbacks: CallbackContractV3,
) -> Result<(SyndocalAsioV3Handle, StartResultJsonV3), BridgeErrorV3> {
    let _unavailable_request = (request.driver_id, request.mode);
    let _unavailable_callbacks = (
        callbacks.output,
        callbacks.duplex,
        callbacks.event,
        callbacks.context,
    );
    Err(BridgeErrorV3::new(
        STATUS_UNSUPPORTED_V3,
        "v3_rt_backend_unavailable",
        "ASIO v3 playback is unavailable because this DLL was built without the non-default ASIO feature; no driver was opened and no fallback was selected",
    ))
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub(crate) fn stop(handle: &mut SyndocalAsioV3Handle) -> Result<StopResultJsonV3, BridgeErrorV3> {
    if handle.stopped {
        return Ok(StopResultJsonV3::new(true));
    }
    handle
        .callback_state
        .render_enabled
        .store(false, Ordering::Release);
    handle.watchdog.stop_and_join();
    let faulted_before_stop = asio_host_lease::is_fault(handle.ticket);
    asio_host_lease::begin_stop(handle.ticket).map_err(busy_error)?;
    let terminal_before_close = {
        if let Err(error) = unsafe { sdk::stop(handle.native_session) } {
            asio_host_lease::stop_failed(handle.ticket);
            return Err(error);
        }
        sdk::telemetry(handle.native_session).terminal_kind != 0
    };
    if !faulted_before_stop && !terminal_before_close {
        asio_host_lease::stop_succeeded(handle.ticket);
        handle.lease_released = true;
    }
    handle.stopped = true;
    Ok(StopResultJsonV3::new(true))
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
pub(crate) fn stop(_handle: &mut SyndocalAsioV3Handle) -> Result<StopResultJsonV3, BridgeErrorV3> {
    Err(unavailable_lifecycle_error("stop"))
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub(crate) fn close(handle: &mut SyndocalAsioV3Handle) -> Result<CloseResultJsonV3, BridgeErrorV3> {
    if !handle.stopped {
        stop(handle)?;
    }
    if let Err(error) = unsafe { sdk::close(&mut handle.native_session) } {
        if !handle.lease_released {
            asio_host_lease::stop_failed(handle.ticket);
        }
        return Err(error);
    }
    if !handle.lease_released {
        asio_host_lease::stop_succeeded(handle.ticket);
        handle.lease_released = true;
    }
    Ok(CloseResultJsonV3::new(true))
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
pub(crate) fn close(
    _handle: &mut SyndocalAsioV3Handle,
) -> Result<CloseResultJsonV3, BridgeErrorV3> {
    Err(unavailable_lifecycle_error("close"))
}

#[cfg(all(target_os = "windows", feature = "asio"))]
pub(crate) fn telemetry(handle: &SyndocalAsioV3Handle) -> Result<TelemetryJsonV3, BridgeErrorV3> {
    let telemetry = sdk::telemetry(handle.native_session);
    if telemetry.terminal_kind != 0 {
        asio_host_lease::mark_fault(handle.ticket);
    }
    Ok(TelemetryJsonV3::new(
        telemetry.callbacks,
        telemetry.xruns,
        terminal_label(telemetry.terminal_kind),
    ))
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
pub(crate) fn telemetry(_handle: &SyndocalAsioV3Handle) -> Result<TelemetryJsonV3, BridgeErrorV3> {
    Err(unavailable_lifecycle_error("telemetry"))
}

#[cfg(any(test, all(target_os = "windows", feature = "asio")))]
fn backend_error(message: impl Into<String>) -> BridgeErrorV3 {
    BridgeErrorV3::new(STATUS_BACKEND_ERROR_V3, "asio_v3_backend_error", message)
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn terminal_error(message: impl Into<String>) -> BridgeErrorV3 {
    BridgeErrorV3::new(STATUS_TERMINAL_V3, "asio_v3_terminal", message)
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
fn unavailable_lifecycle_error(operation: &'static str) -> BridgeErrorV3 {
    BridgeErrorV3::new(
        STATUS_UNSUPPORTED_V3,
        "v3_rt_backend_unavailable",
        format!("ASIO v3 {operation} is unavailable because this DLL was built without the non-default ASIO feature"),
    )
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn busy_error(state: asio_host_lease::BusyState) -> BridgeErrorV3 {
    BridgeErrorV3::new(
        STATUS_BACKEND_ERROR_V3,
        "asio_host_busy",
        format!(
            "ASIO host is {}; current owner was not stopped, migrated, or probed",
            state.label()
        ),
    )
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn terminal_label(kind: u32) -> Option<&'static str> {
    match kind {
        0 => None,
        EVENT_XRUN_V3 => Some("xrun"),
        EVENT_RESET_V3 => Some("reset"),
        EVENT_RESYNC_V3 => Some("resync"),
        EVENT_SAMPLE_RATE_CHANGED_V3 => Some("sample-rate-changed"),
        EVENT_DEVICE_LOST_V3 => Some("device-lost"),
        EVENT_CALLBACK_GAP_V3 => Some("callback-gap"),
        EVENT_BACKEND_V3 => Some("backend"),
        EVENT_MALFORMED_CALLBACK_V3 => Some("malformed-callback"),
        EVENT_BUFFER_SIZE_CHANGED_V3 => Some("buffer-size-changed"),
        EVENT_OUTPUT_QUEUE_UNDERFLOW_V3 => Some("output-queue-underflow"),
        EVENT_OUTPUT_QUEUE_FULL_V3 => Some("output-queue-full"),
        EVENT_CALLBACK_PANIC_V3 => Some("callback-panic"),
        EVENT_INVALID_OUTPUT_BLOCK_V3 => Some("invalid-output-block"),
        _ => Some("unknown-terminal"),
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn terminal_message(kind: u32) -> &'static [u8] {
    match terminal_label(kind) {
        Some("xrun") => b"ASIO v3 output xrun",
        Some("reset") => b"ASIO v3 driver requested reset",
        Some("resync") => b"ASIO v3 driver requested resynchronization",
        Some("sample-rate-changed") => b"ASIO v3 sample rate changed",
        Some("device-lost") => b"ASIO v3 explicit device was lost",
        Some("callback-gap") => b"ASIO v3 callback gap exceeded 250 ms",
        Some("buffer-size-changed") => b"ASIO v3 fixed buffer changed",
        Some("output-queue-underflow") => b"ASIO v3 output queue underflow",
        Some("output-queue-full") => b"ASIO v3 output queue full",
        Some("callback-panic") => b"ASIO v3 callback panic was contained",
        Some("invalid-output-block") => b"ASIO v3 callback returned an invalid output block",
        Some("malformed-callback") => b"ASIO v3 driver callback was malformed",
        _ => b"ASIO v3 terminal backend fault",
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
#[path = "v3_sdk_ffi.rs"]
mod sdk;

#[cfg(test)]
#[path = "v3_rt_backend_tests.rs"]
mod tests;
