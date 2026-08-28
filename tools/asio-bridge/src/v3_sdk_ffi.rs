use super::*;

#[repr(C)]
struct StartConfigRaw {
    driver_name: *const c_char,
    output_channels: u32,
    input_channels: u32,
    output_format: u32,
    input_format: u32,
    input_sample_rate_hz: u32,
    input_fixed_buffer_frames: u32,
    sample_rate_hz: u32,
    fixed_buffer_frames: u32,
    first_output_frame: u64,
    session_generation: u64,
    render_generation: u64,
    process_callback: Option<
        unsafe extern "C" fn(
            *mut c_void,
            *const *const c_void,
            *const *mut c_void,
            u32,
            u32,
            u32,
            u64,
            u64,
            u64,
        ) -> u32,
    >,
    process_context: *mut c_void,
}

pub(super) struct StartConfig {
    pub(super) driver_name: CString,
    pub(super) output_channels: u32,
    pub(super) input_channels: u32,
    pub(super) output_format: u32,
    pub(super) input_format: u32,
    pub(super) input_sample_rate_hz: u32,
    pub(super) input_fixed_buffer_frames: u32,
    pub(super) sample_rate_hz: u32,
    pub(super) fixed_buffer_frames: u32,
    pub(super) first_output_frame: u64,
    pub(super) session_generation: u64,
    pub(super) render_generation: u64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct Started {
    pub(super) output_format: u32,
    pub(super) input_format: u32,
    pub(super) output_channels: u32,
    pub(super) input_channels: u32,
    pub(super) sample_rate_hz: u32,
    pub(super) fixed_buffer_frames: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Telemetry {
    pub(super) callbacks: u64,
    pub(super) xruns: u64,
    pub(super) last_callback_tick_ms: u64,
    pub(super) terminal_kind: u32,
    pub(super) running: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub(super) struct Capabilities {
    pub(super) input_channels: u32,
    pub(super) output_channels: u32,
    pub(super) input_format: u32,
    pub(super) output_format: u32,
    pub(super) current_sample_rate_hz: u32,
    pub(super) buffer_min_frames: u32,
    pub(super) buffer_max_frames: u32,
    pub(super) buffer_preferred_frames: u32,
    pub(super) buffer_granularity: i32,
    supported_rate_count: u32,
    supported_rates_hz: [u32; 24],
}

impl Capabilities {
    pub(super) fn supported_rates(&self) -> Vec<u32> {
        let len = (self.supported_rate_count as usize).min(self.supported_rates_hz.len());
        self.supported_rates_hz[..len].to_vec()
    }
}

extern "C" {
    fn sd_asio_v3_driver_names(
        slots: *mut c_char,
        slot_size: usize,
        max_drivers: usize,
        out_count: *mut usize,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn sd_asio_v3_capabilities(
        driver_name: *const c_char,
        out_capabilities: *mut Capabilities,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn sd_asio_v3_start(
        config: *const StartConfigRaw,
        out_session: *mut *mut c_void,
        out_started: *mut Started,
        error: *mut c_char,
        error_len: usize,
    ) -> i32;
    fn sd_asio_v3_stop(session: *mut c_void, error: *mut c_char, error_len: usize) -> i32;
    fn sd_asio_v3_close(session: *mut *mut c_void, error: *mut c_char, error_len: usize) -> i32;
    fn sd_asio_v3_telemetry(session: *const c_void, out_telemetry: *mut Telemetry);
    fn sd_asio_v3_latch_terminal(session: *mut c_void, terminal_kind: u32);
    #[cfg(test)]
    fn sd_asio_v3_driver_open_attempts() -> u64;
    #[cfg(test)]
    fn sd_asio_v3_has_published_session() -> u32;
}

fn error_text(bytes: &[c_char; ERROR_BYTES], fallback: &str) -> String {
    let value = unsafe { CStr::from_ptr(bytes.as_ptr()) }.to_string_lossy();
    if value.is_empty() {
        fallback.to_string()
    } else {
        value.into_owned()
    }
}

pub(super) fn driver_names() -> Result<Vec<String>, BridgeErrorV3> {
    let mut slots = vec![0_i8; DRIVER_SLOTS * DRIVER_SLOT_BYTES];
    let mut count = 0_usize;
    let mut error = [0_i8; ERROR_BYTES];
    let status = unsafe {
        sd_asio_v3_driver_names(
            slots.as_mut_ptr(),
            DRIVER_SLOT_BYTES,
            DRIVER_SLOTS,
            &mut count,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 || count > DRIVER_SLOTS {
        return Err(backend_error(error_text(
            &error,
            "ASIO driver enumeration failed",
        )));
    }
    let mut names = Vec::with_capacity(count);
    for index in 0..count {
        let pointer = unsafe { slots.as_ptr().add(index * DRIVER_SLOT_BYTES) };
        let name = unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .into_owned();
        if name.is_empty() {
            return Err(backend_error(
                "ASIO driver catalog contained an invalid name",
            ));
        }
        names.push(name);
    }
    Ok(names)
}

pub(super) fn capabilities(name: &str) -> Result<Capabilities, BridgeErrorV3> {
    let name =
        CString::new(name).map_err(|_| backend_error("ASIO driver name contains a null byte"))?;
    let mut capabilities = Capabilities::default();
    let mut error = [0_i8; ERROR_BYTES];
    let status = unsafe {
        sd_asio_v3_capabilities(
            name.as_ptr(),
            &mut capabilities,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 {
        return Err(backend_error(error_text(
            &error,
            "ASIO capability inspection failed",
        )));
    }
    Ok(capabilities)
}

pub(super) unsafe fn start(
    config: &StartConfig,
    callback_state: *mut CallbackState,
) -> Result<(*mut c_void, Started), BridgeErrorV3> {
    let raw = StartConfigRaw {
        driver_name: config.driver_name.as_ptr(),
        output_channels: config.output_channels,
        input_channels: config.input_channels,
        output_format: config.output_format,
        input_format: config.input_format,
        input_sample_rate_hz: config.input_sample_rate_hz,
        input_fixed_buffer_frames: config.input_fixed_buffer_frames,
        sample_rate_hz: config.sample_rate_hz,
        fixed_buffer_frames: config.fixed_buffer_frames,
        first_output_frame: config.first_output_frame,
        session_generation: config.session_generation,
        render_generation: config.render_generation,
        process_callback: Some(process_callback),
        process_context: callback_state.cast::<c_void>(),
    };
    let mut session = ptr::null_mut();
    let mut started = Started::default();
    let mut error = [0_i8; ERROR_BYTES];
    let status = unsafe {
        sd_asio_v3_start(
            &raw,
            &mut session,
            &mut started,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    if status != 0 || session.is_null() {
        return Err(backend_error(error_text(&error, "ASIO v3 Start failed")));
    }
    Ok((session, started))
}

pub(super) unsafe fn stop(session: *mut c_void) -> Result<(), BridgeErrorV3> {
    let mut error = [0_i8; ERROR_BYTES];
    let status = unsafe { sd_asio_v3_stop(session, error.as_mut_ptr(), error.len()) };
    if status == 0 {
        Ok(())
    } else {
        Err(terminal_error(error_text(&error, "ASIO v3 Stop failed")))
    }
}

pub(super) unsafe fn close(session: &mut *mut c_void) -> Result<(), BridgeErrorV3> {
    if session.is_null() {
        return Ok(());
    }
    let mut error = [0_i8; ERROR_BYTES];
    let status = unsafe { sd_asio_v3_close(session, error.as_mut_ptr(), error.len()) };
    if status == 0 {
        Ok(())
    } else {
        Err(terminal_error(error_text(&error, "ASIO v3 Close failed")))
    }
}

pub(super) fn telemetry(session: *mut c_void) -> Telemetry {
    let mut telemetry = Telemetry::default();
    unsafe { sd_asio_v3_telemetry(session, &mut telemetry) };
    telemetry
}

pub(super) unsafe fn latch_terminal(session: *mut c_void, kind: u32) {
    unsafe { sd_asio_v3_latch_terminal(session, kind) };
}

#[cfg(test)]
pub(super) struct RejectedStartProbe {
    pub(super) status: i32,
    pub(super) error: String,
    pub(super) session_is_null: bool,
    pub(super) started_before: Started,
    pub(super) started_after: Started,
    pub(super) driver_attempts_before: u64,
    pub(super) driver_attempts_after: u64,
    pub(super) session_published_after: bool,
}

#[cfg(test)]
pub(super) unsafe fn rejected_start_probe(config: &StartConfig) -> RejectedStartProbe {
    let raw = StartConfigRaw {
        driver_name: config.driver_name.as_ptr(),
        output_channels: config.output_channels,
        input_channels: config.input_channels,
        output_format: config.output_format,
        input_format: config.input_format,
        input_sample_rate_hz: config.input_sample_rate_hz,
        input_fixed_buffer_frames: config.input_fixed_buffer_frames,
        sample_rate_hz: config.sample_rate_hz,
        fixed_buffer_frames: config.fixed_buffer_frames,
        first_output_frame: config.first_output_frame,
        session_generation: config.session_generation,
        render_generation: config.render_generation,
        process_callback: Some(process_callback),
        process_context: ptr::null_mut(),
    };
    let started_before = Started {
        output_format: 0xa1,
        input_format: 0xb2,
        output_channels: 0xc3,
        input_channels: 0xd4,
        sample_rate_hz: 0xe5,
        fixed_buffer_frames: 0xf6,
    };
    let mut started_after = started_before;
    let mut session = ptr::null_mut();
    let mut error = [0_i8; ERROR_BYTES];
    let driver_attempts_before = unsafe { sd_asio_v3_driver_open_attempts() };
    let status = unsafe {
        sd_asio_v3_start(
            &raw,
            &mut session,
            &mut started_after,
            error.as_mut_ptr(),
            error.len(),
        )
    };
    RejectedStartProbe {
        status,
        error: error_text(&error, "ASIO v3 probe failed without an error"),
        session_is_null: session.is_null(),
        started_before,
        started_after,
        driver_attempts_before,
        driver_attempts_after: unsafe { sd_asio_v3_driver_open_attempts() },
        session_published_after: unsafe { sd_asio_v3_has_published_session() } != 0,
    }
}

#[cfg(test)]
pub(super) fn driver_open_attempts_for_test() -> u64 {
    unsafe { sd_asio_v3_driver_open_attempts() }
}

#[cfg(test)]
pub(super) fn session_published_for_test() -> bool {
    unsafe { sd_asio_v3_has_published_session() != 0 }
}
