use super::*;

fn output_request() -> StartRequestJsonV3 {
    StartRequestJsonV3 {
        schema_version: 3,
        driver_id: "asio:Example".to_owned(),
        mode: StreamModeV3::OutputOnly,
        output: StreamTupleJsonV3 {
            channels: 7,
            sample_format: "f32".to_owned(),
            sample_rate_hz: 48_000,
            fixed_buffer_frames: 256,
        },
        input: None,
        first_output_frame: 0,
        session_generation: 1,
        render_generation: 1,
    }
}

#[test]
fn v3_contract_rejects_duplex_shape_and_unknown_fields() {
    let mut request = output_request();
    request.mode = StreamModeV3::FullDuplex;
    assert!(parse_start_request(request).is_err());

    let text = r#"{"schemaVersion":3,"driverId":"asio:Example","mode":"outputOnly","output":{"channels":2,"sampleFormat":"f32","sampleRateHz":48000,"fixedBufferFrames":256},"firstOutputFrame":0,"sessionGeneration":1,"renderGeneration":1,"physicalCue":3}"#;
    let parsed = serde_json::from_str::<StartRequestJsonV3>(text);
    assert!(parsed.is_err());
}

#[test]
fn v3_full_duplex_rejects_every_shared_clock_mismatch_before_backend_start() {
    unsafe extern "C" fn duplex_callback(
        _context: *mut c_void,
        _input: *const f32,
        _input_channels: u32,
        _output: *mut f32,
        _output_channels: u32,
        _frames: u32,
        _first_frame: u64,
        _session_generation: u64,
        _render_generation: u64,
    ) -> u32 {
        CALLBACK_ACCEPTED_V3
    }

    for (input_rate, input_frames, expected_message) in [
        (44_100, 256, "input.sampleRateHz"),
        (96_000, 256, "input.sampleRateHz"),
        (48_000, 128, "input.fixedBufferFrames"),
        (48_000, 512, "input.fixedBufferFrames"),
    ] {
        let mut request = output_request();
        request.mode = StreamModeV3::FullDuplex;
        request.input = Some(StreamTupleJsonV3 {
            channels: 2,
            sample_format: "f32".to_owned(),
            sample_rate_hz: input_rate,
            fixed_buffer_frames: input_frames,
        });

        let parsed = parse_start_request(request.clone()).unwrap_err();
        assert_eq!(parsed.status, STATUS_INVALID_ARGUMENT_V3);
        assert_eq!(parsed.code, "invalid_argument");
        assert!(parsed.message.contains(expected_message));

        let request = serde_json::to_vec(&request).unwrap();
        let mut handle = ptr::null_mut();
        let mut result = SyndocalAsioStringV3::default();
        let mut error = SyndocalAsioStringV3::default();
        let status = unsafe {
            syndocal_asio_v3_start(
                request.as_ptr(),
                request.len(),
                None,
                Some(duplex_callback),
                None,
                ptr::null_mut(),
                &mut handle,
                &mut result,
                &mut error,
            )
        };
        assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
        assert!(handle.is_null());
        assert!(result.ptr.is_null(), "must not synthesize actualInput");
        let error_bytes = unsafe { slice::from_raw_parts(error.ptr, error.len).to_vec() };
        unsafe { syndocal_asio_v3_string_free(error) };
        let error: serde_json::Value = serde_json::from_slice(&error_bytes).unwrap();
        assert_eq!(error["code"], "invalid_argument");
        assert!(error["message"]
            .as_str()
            .unwrap()
            .contains(expected_message));
    }
}

#[test]
fn v3_callback_event_and_fault_codes_are_frozen() {
    assert_eq!(STATUS_TERMINAL_V3, 6);
    assert_eq!(
        [
            CALLBACK_ACCEPTED_V3,
            CALLBACK_QUEUE_UNDERFLOW_V3,
            CALLBACK_QUEUE_FULL_V3,
            CALLBACK_TERMINAL_V3,
            CALLBACK_INVALID_BLOCK_V3,
            CALLBACK_PANIC_V3,
        ],
        [0, 1, 2, 3, 4, 5]
    );
    assert_eq!(
        [
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
        ],
        [1, 2, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14]
    );
}

#[cfg(not(all(target_os = "windows", feature = "asio")))]
#[test]
fn v3_output_start_is_visible_unsupported_without_fallback() {
    unsafe extern "C" fn output_callback(
        _context: *mut c_void,
        _output: *mut f32,
        _channels: u32,
        _frames: u32,
        _first_frame: u64,
        _session_generation: u64,
        _render_generation: u64,
    ) -> u32 {
        0
    }
    let request = serde_json::to_vec(&output_request()).unwrap();
    let mut handle = ptr::null_mut();
    let mut result = SyndocalAsioStringV3::default();
    let mut error = SyndocalAsioStringV3::default();
    let status = unsafe {
        syndocal_asio_v3_start(
            request.as_ptr(),
            request.len(),
            Some(output_callback),
            None,
            None,
            ptr::null_mut(),
            &mut handle,
            &mut result,
            &mut error,
        )
    };
    assert_eq!(status, STATUS_UNSUPPORTED_V3);
    assert!(handle.is_null());
    let error_bytes = unsafe { slice::from_raw_parts(error.ptr, error.len).to_vec() };
    unsafe { syndocal_asio_v3_string_free(error) };
    let error: serde_json::Value = serde_json::from_slice(&error_bytes).unwrap();
    assert_eq!(error["code"], "v3_rt_backend_unavailable");
    assert!(result.ptr.is_null());
}

#[test]
fn v3_cross_aliases_are_rejected_before_any_output_write() {
    unsafe extern "C" fn output_callback(
        _context: *mut c_void,
        _output: *mut f32,
        _channels: u32,
        _frames: u32,
        _first_frame: u64,
        _session_generation: u64,
        _render_generation: u64,
    ) -> u32 {
        CALLBACK_ACCEPTED_V3
    }
    let request = serde_json::to_vec(&output_request()).unwrap();

    // The handle slot overlaps the first word of the result struct.
    let mut start_slots = [0_usize; 4];
    let status = unsafe {
        syndocal_asio_v3_start(
            request.as_ptr(),
            request.len(),
            Some(output_callback),
            None,
            None,
            ptr::null_mut(),
            start_slots.as_mut_ptr().cast::<*mut SyndocalAsioV3Handle>(),
            start_slots.as_mut_ptr().cast::<SyndocalAsioStringV3>(),
            start_slots
                .as_mut_ptr()
                .wrapping_add(2)
                .cast::<SyndocalAsioStringV3>(),
        )
    };
    assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
    assert_eq!(start_slots, [0; 4]);

    // Result/error slots overlap by one usize but are not equal pointers.
    let mut result_slots = [0_usize; 4];
    let mut handle = ptr::null_mut();
    let status = unsafe {
        syndocal_asio_v3_start(
            request.as_ptr(),
            request.len(),
            Some(output_callback),
            None,
            None,
            ptr::null_mut(),
            &mut handle,
            result_slots.as_mut_ptr().cast::<SyndocalAsioStringV3>(),
            result_slots
                .as_mut_ptr()
                .wrapping_add(1)
                .cast::<SyndocalAsioStringV3>(),
        )
    };
    assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
    assert!(handle.is_null());
    assert_eq!(result_slots, [0; 4]);

    // A null out_handle must not short-circuit the result/error alias
    // gate into writing an error payload.  Use non-zero sentinels so this
    // proves the entire affected byte range remains untouched.
    let sentinel = 0x9d91_6cf5_1234_5678_usize;
    let mut null_handle_slots = [sentinel; 4];
    let status = unsafe {
        syndocal_asio_v3_start(
            request.as_ptr(),
            request.len(),
            Some(output_callback),
            None,
            None,
            ptr::null_mut(),
            ptr::null_mut(),
            null_handle_slots
                .as_mut_ptr()
                .cast::<SyndocalAsioStringV3>(),
            null_handle_slots
                .as_mut_ptr()
                .wrapping_add(1)
                .cast::<SyndocalAsioStringV3>(),
        )
    };
    assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
    assert_eq!(null_handle_slots, [sentinel; 4]);

    // Close must reject the handle slot crossing the result range before
    // reading the slot (so even a fabricated/invalid handle is harmless).
    let mut close_slots = [0_usize; 3];
    let status = unsafe {
        syndocal_asio_v3_close(
            close_slots.as_mut_ptr().cast::<*mut SyndocalAsioV3Handle>(),
            close_slots.as_mut_ptr().cast::<SyndocalAsioStringV3>(),
            close_slots
                .as_mut_ptr()
                .wrapping_add(2)
                .cast::<SyndocalAsioStringV3>(),
        )
    };
    assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
    assert_eq!(close_slots, [0; 3]);

    let mut driver_slots = [0_usize; 3];
    let status = unsafe {
        syndocal_asio_v3_drivers_json(
            driver_slots.as_mut_ptr().cast::<SyndocalAsioStringV3>(),
            driver_slots
                .as_mut_ptr()
                .wrapping_add(1)
                .cast::<SyndocalAsioStringV3>(),
        )
    };
    assert_eq!(status, STATUS_INVALID_ARGUMENT_V3);
    assert_eq!(driver_slots, [0; 3]);
}
