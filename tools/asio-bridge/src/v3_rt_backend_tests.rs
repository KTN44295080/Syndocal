use super::*;

struct OutputFixture {
    calls: AtomicU64,
    result: u32,
    leave_last_unwritten: bool,
}

unsafe extern "C" fn fill_output(
    context: *mut c_void,
    output: *mut f32,
    channels: u32,
    frames: u32,
    _first_output_frame: u64,
    _session_generation: u64,
    _render_generation: u64,
) -> u32 {
    let fixture = unsafe { &*context.cast::<OutputFixture>() };
    fixture.calls.fetch_add(1, Ordering::Relaxed);
    let len = channels as usize * frames as usize;
    let write_len = if fixture.leave_last_unwritten {
        len.saturating_sub(1)
    } else {
        len
    };
    for index in 0..write_len {
        unsafe { output.add(index).write(0.25) };
    }
    fixture.result
}

struct DuplexFixture {
    saw_expected_input: AtomicBool,
}

unsafe extern "C" fn copy_duplex(
    context: *mut c_void,
    input: *const f32,
    input_channels: u32,
    output: *mut f32,
    output_channels: u32,
    frames: u32,
    _first_output_frame: u64,
    _session_generation: u64,
    _render_generation: u64,
) -> u32 {
    let fixture = unsafe { &*context.cast::<DuplexFixture>() };
    let len = input_channels as usize * frames as usize;
    let input = unsafe { std::slice::from_raw_parts(input, len) };
    fixture.saw_expected_input.store(
        input_channels == 2
            && output_channels == 2
            && input.len() == 4
            && input[0] == 0.0
            && input[1] == -1.0
            && (input[2] - i16::MAX as f32 / 32768.0).abs() < f32::EPSILON
            && input[3] == 0.0,
        Ordering::Release,
    );
    unsafe { std::ptr::copy_nonoverlapping(input.as_ptr(), output, len) };
    CALLBACK_ACCEPTED_V3
}

fn request() -> StartRequestV3 {
    StartRequestV3::for_test(2, 4, 100, 7, 9)
}

fn state(fixture: &OutputFixture, request: &StartRequestV3) -> Box<CallbackState> {
    CallbackState::new(
        request,
        CallbackContractV3 {
            output: Some(fill_output),
            duplex: None,
            event: None,
            context: (fixture as *const OutputFixture)
                .cast_mut()
                .cast::<c_void>(),
        },
    )
    .unwrap()
}

fn output_block(
    output_buffers: *const *mut c_void,
    first_output_frame: u64,
    render_generation: u64,
) -> ProcessBlock {
    ProcessBlock {
        input_buffers: ptr::null(),
        output_buffers,
        input_format_code: 0,
        output_format_code: NativeFormat::F32Le as u32,
        frames: 4,
        first_output_frame,
        session_generation: 7,
        render_generation,
    }
}

#[test]
fn callback_progresses_exact_frames_and_writes_complete_native_blocks() {
    let fixture = OutputFixture {
        calls: AtomicU64::new(0),
        result: CALLBACK_ACCEPTED_V3,
        leave_last_unwritten: false,
    };
    let request = request();
    let mut state = state(&fixture, &request);
    let mut left = [0.0_f32; 4];
    let mut right = [0.0_f32; 4];
    let output = [
        left.as_mut_ptr().cast::<c_void>(),
        right.as_mut_ptr().cast::<c_void>(),
    ];
    for first_frame in [100, 104] {
        assert_eq!(
            unsafe { state.process(output_block(output.as_ptr(), first_frame, 9)) },
            CALLBACK_ACCEPTED_V3
        );
    }
    assert_eq!(left, [0.25; 4]);
    assert_eq!(right, [0.25; 4]);
    assert_eq!(fixture.calls.load(Ordering::Relaxed), 2);
}

#[test]
fn stale_generation_frame_or_partial_write_is_full_silence() {
    for (first_frame, generation, leave_last_unwritten) in
        [(99, 9, false), (100, 8, false), (100, 9, true)]
    {
        let fixture = OutputFixture {
            calls: AtomicU64::new(0),
            result: CALLBACK_ACCEPTED_V3,
            leave_last_unwritten,
        };
        let request = request();
        let mut state = state(&fixture, &request);
        let mut left = [1.0_f32; 4];
        let mut right = [1.0_f32; 4];
        let output = [
            left.as_mut_ptr().cast::<c_void>(),
            right.as_mut_ptr().cast::<c_void>(),
        ];
        assert_eq!(
            unsafe { state.process(output_block(output.as_ptr(), first_frame, generation)) },
            CALLBACK_INVALID_BLOCK_V3
        );
        assert_eq!(left, [0.0; 4]);
        assert_eq!(right, [0.0; 4]);
        let expected_frame = if leave_last_unwritten { 104 } else { 100 };
        assert_eq!(
            state.expected_first_output_frame.load(Ordering::Acquire),
            expected_frame
        );
    }
}

#[test]
fn every_nonaccepted_callback_result_is_full_silence() {
    for callback_result in 1..=5 {
        let fixture = OutputFixture {
            calls: AtomicU64::new(0),
            result: callback_result,
            leave_last_unwritten: false,
        };
        let request = request();
        let mut state = state(&fixture, &request);
        let mut left = [1.0_f32; 4];
        let mut right = [1.0_f32; 4];
        let output = [
            left.as_mut_ptr().cast::<c_void>(),
            right.as_mut_ptr().cast::<c_void>(),
        ];
        assert_eq!(
            unsafe { state.process(output_block(output.as_ptr(), 100, 9)) },
            callback_result
        );
        assert_eq!(left, [0.0; 4]);
        assert_eq!(right, [0.0; 4]);
        assert_eq!(
            state.expected_first_output_frame.load(Ordering::Acquire),
            104
        );
    }
}

#[test]
fn full_duplex_interleaves_native_input_and_converts_complete_output() {
    let fixture = DuplexFixture {
        saw_expected_input: AtomicBool::new(false),
    };
    let mut request = request();
    request.mode = StreamModeV3::FullDuplex;
    request.output.fixed_buffer_frames = 2;
    request.input = Some(crate::v3_abi::StreamTupleJsonV3 {
        channels: 2,
        sample_format: "i16".to_owned(),
        sample_rate_hz: 48_000,
        fixed_buffer_frames: 2,
    });
    let mut state = CallbackState::new(
        &request,
        CallbackContractV3 {
            output: None,
            duplex: Some(copy_duplex),
            event: None,
            context: (&fixture as *const DuplexFixture)
                .cast_mut()
                .cast::<c_void>(),
        },
    )
    .unwrap();
    let left_input = [0_i16, i16::MAX];
    let right_input = [i16::MIN, 0];
    let input = [
        left_input.as_ptr().cast::<c_void>(),
        right_input.as_ptr().cast::<c_void>(),
    ];
    let mut left_output = [1.0_f32; 2];
    let mut right_output = [1.0_f32; 2];
    let output = [
        left_output.as_mut_ptr().cast::<c_void>(),
        right_output.as_mut_ptr().cast::<c_void>(),
    ];
    assert_eq!(
        unsafe {
            state.process(ProcessBlock {
                input_buffers: input.as_ptr(),
                output_buffers: output.as_ptr(),
                input_format_code: NativeFormat::I16Le as u32,
                output_format_code: NativeFormat::F32Le as u32,
                frames: 2,
                first_output_frame: 100,
                session_generation: 7,
                render_generation: 9,
            })
        },
        CALLBACK_ACCEPTED_V3
    );
    assert!(fixture.saw_expected_input.load(Ordering::Acquire));
    assert_eq!(left_output[0], 0.0);
    assert_eq!(right_output[0], -1.0);
    assert_eq!(left_output[1], i16::MAX as f32 / 32768.0);
    assert_eq!(right_output[1], 0.0);
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn mismatched_duplex_request(input_rate: u32, input_frames: u32) -> StartRequestV3 {
    let mut request = request();
    request.mode = StreamModeV3::FullDuplex;
    request.output.sample_rate_hz = 48_000;
    request.output.fixed_buffer_frames = 256;
    request.input = Some(crate::v3_abi::StreamTupleJsonV3 {
        channels: 2,
        sample_format: "f32".to_owned(),
        sample_rate_hz: input_rate,
        fixed_buffer_frames: input_frames,
    });
    request
}

#[cfg(all(target_os = "windows", feature = "asio"))]
fn mismatched_sdk_config(input_rate: u32, input_frames: u32) -> sdk::StartConfig {
    sdk::StartConfig {
        driver_name: CString::new("This driver must never be opened").unwrap(),
        output_channels: 2,
        input_channels: 2,
        output_format: NativeFormat::F32Le as u32,
        input_format: NativeFormat::F32Le as u32,
        input_sample_rate_hz: input_rate,
        input_fixed_buffer_frames: input_frames,
        sample_rate_hz: 48_000,
        fixed_buffer_frames: 256,
        first_output_frame: 100,
        session_generation: 7,
        render_generation: 9,
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
#[test]
fn backend_rejects_shared_clock_mismatches_before_lease_or_driver_start() {
    let fixture = DuplexFixture {
        saw_expected_input: AtomicBool::new(false),
    };
    for (input_rate, input_frames, expected_message) in [
        (44_100, 256, "input.sampleRateHz"),
        (96_000, 256, "input.sampleRateHz"),
        (48_000, 128, "input.fixedBufferFrames"),
        (48_000, 512, "input.fixedBufferFrames"),
    ] {
        let attempts_before = sdk::driver_open_attempts_for_test();
        let outcome = start(
            mismatched_duplex_request(input_rate, input_frames),
            CallbackContractV3 {
                output: None,
                duplex: Some(copy_duplex),
                event: None,
                context: (&fixture as *const DuplexFixture)
                    .cast_mut()
                    .cast::<c_void>(),
            },
        );
        let error = match outcome {
            Ok(_) => panic!("mismatched shared-clock tuple started"),
            Err(error) => error,
        };
        assert_eq!(error.status, crate::v3_abi::STATUS_INVALID_ARGUMENT_V3);
        assert_eq!(error.code, "invalid_argument");
        assert!(error.message.contains(expected_message));
        assert_eq!(sdk::driver_open_attempts_for_test(), attempts_before);
        assert!(!sdk::session_published_for_test());
        assert!(!fixture.saw_expected_input.load(Ordering::Acquire));
    }
}

#[cfg(all(target_os = "windows", feature = "asio"))]
#[test]
fn cpp_boundary_rejects_shared_clock_mismatches_without_open_or_actual_input() {
    for (input_rate, input_frames, expected_message) in [
        (44_100, 256, "sample rates must match"),
        (96_000, 256, "sample rates must match"),
        (48_000, 128, "buffer sizes must match"),
        (48_000, 512, "buffer sizes must match"),
    ] {
        let probe =
            unsafe { sdk::rejected_start_probe(&mismatched_sdk_config(input_rate, input_frames)) };
        assert_eq!(probe.status, 1);
        assert!(probe.error.contains(expected_message));
        assert!(probe.session_is_null);
        assert_eq!(probe.started_after, probe.started_before);
        assert_eq!(probe.driver_attempts_after, probe.driver_attempts_before);
        assert!(!probe.session_published_after);
    }
}

#[test]
fn sdk_callback_source_is_allocation_free_and_unpublishes_before_drain() {
    let source = include_str!("v3_sdk_backend.cpp");
    let callback_start = source.find("void process_buffer(").unwrap();
    let callback_end = source[callback_start..]
        .find("\nvoid buffer_switch(")
        .map(|offset| callback_start + offset)
        .unwrap();
    let callback = &source[callback_start..callback_end];
    for forbidden in [
        "std::mutex",
        "lock_guard",
        "unique_lock",
        "new ",
        ".resize(",
        ".reserve(",
        "filesystem",
        "Sleep(",
    ] {
        assert!(
            !callback.contains(forbidden),
            "callback contains {forbidden}"
        );
    }
    assert!(callback.contains("g_readers.fetch_add"));
    assert!(callback.contains("g_readers.fetch_sub"));
    assert!(source.contains("terminal_kind.compare_exchange_strong"));

    let cleanup_start = source.find("bool cleanup_session(").unwrap();
    let cleanup_end = source[cleanup_start..]
        .find("extern \"C\" int32_t sd_asio_v3_driver_names")
        .map(|offset| cleanup_start + offset)
        .unwrap();
    let cleanup = &source[cleanup_start..cleanup_end];
    let disable = cleanup.find("accept_client.store(false").unwrap();
    let unpublish = cleanup.find("g_published.compare_exchange_strong").unwrap();
    let drain = cleanup.find("g_readers.load").unwrap();
    let dispose = cleanup.find("ASIODisposeBuffers").unwrap();
    assert!(disable < unpublish && unpublish < drain && drain < dispose);
}

#[test]
fn default_manifest_graph_keeps_asio_and_cpp_dependencies_optional() {
    let manifest = include_str!("../Cargo.toml");
    let build = include_str!("../build.rs");
    assert!(manifest.contains("asio = [\"dep:cc\", \"dep:cpal-asio\"]"));
    assert!(manifest.contains("cc = { version = \"1\", optional = true }"));
    assert!(manifest.contains("cpal-asio = { package = \"cpal\""));
    assert!(manifest.contains("optional = true"));
    let feature_gate = build.find("CARGO_FEATURE_ASIO").unwrap();
    let early_return = build[feature_gate..].find("return;").unwrap() + feature_gate;
    let cpp_compile = build.find("cc::Build::new()").unwrap();
    assert!(feature_gate < early_return && early_return < cpp_compile);
    assert!(!build.contains("Invoke-WebRequest"));
    assert!(!build.contains("curl"));
    assert!(!build.contains("http://"));
    assert!(!build.contains("https://"));
}
