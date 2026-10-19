use super::*;

#[test]
fn catch_up_keeps_forward_budget_bounded_and_backward_seeks_exact() {
    for delta in [300, 350, 400, 1_000] {
        assert_eq!(session_decision(Some(100), Some(100), true, false, 100 + delta), LibavSessionDecision::Continue);
    }
    for position in [99, 1_101, 5_000] {
        assert_eq!(session_decision(Some(100), Some(100), true, false, position), LibavSessionDecision::Reopen);
    }
}

#[test]
#[ignore = "requires Windows D3D11VA and SYNDOCAL_GPU_TEST_VIDEO (>4 seconds)"]
fn catch_up_real_gpu_two_outputs_350ms_cadence_preserves_sessions_and_seek_parity() {
    let path = std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set original H264/HEVC video path");
    ffmpeg::init().unwrap();
    assert!(ffmpeg::format::input(&path).unwrap().duration() > 4_000_000);
    let mut gpu = LibavFrameDecoder::new();
    let mut request = VideoFrameRequest {
        layer_id: 901, label: "Bounded two-output catch-up".into(),
        source: protocol::VideoSourceSummary {
            kind: VideoSourceKind::File, path: Some(path), name: None,
            codec: Some("h264".into()), metadata: None,
        }, position_ms: 0, width: 320, height: 128,
    };
    // Reproduce source-time gaps from two serialized 130–200 ms output decodes.
    // No sleeping: these are real compressed frames, not a timing benchmark.
    for position in [0, 350, 700, 1_050, 1_400, 1_800, 3_300, 100] {
        for layer in [901, 902] {
            request.layer_id = layer;
            request.position_ms = position;
            let actual = gpu.decode_frame(&request).unwrap().unwrap();
            // Fresh software seeking is independent of GPU continuation state.
            let expected = LibavFrameDecoder::software_for_tests().decode_frame(&request).unwrap().unwrap();
            assert_eq!((actual.width, actual.height, actual.pts_ms, actual.duration_ms, actual.format),
                (expected.width, expected.height, expected.pts_ms, expected.duration_ms, expected.format));
            assert_eq!(actual.data.len(), expected.data.len());
            let mut total = 0u64;
            let mut maximum = 0u8;
            for (index, (actual, expected)) in actual.data.iter().zip(&expected.data).enumerate() {
                let delta = actual.abs_diff(*expected);
                if index % 4 == 3 { assert_eq!(delta, 0); }
                else { total += u64::from(delta); maximum = maximum.max(delta); }
            }
            let mean = total as f64 / (actual.data.len() / 4 * 3) as f64;
            assert!(maximum <= 3 && mean <= 0.5, "layer {layer} position {position}: max={maximum}, mean={mean}");
        }
        let diagnostics = gpu.session_diagnostics();
        // Far and backward jumps rewind each open session in place: no re-open.
        let expected_resets = if position == 3_300 { 2 } else if position == 100 { 4 } else { 0 };
        assert_eq!(diagnostics.opens, 2, "{diagnostics:?}");
        assert_eq!(diagnostics.resets, expected_resets, "{diagnostics:?}");
        assert_eq!(diagnostics.hardware_sessions, 2);
        assert_eq!(diagnostics.hardware_errors, 0);
    }
    let diagnostics = gpu.session_diagnostics();
    assert_eq!(diagnostics.resets, 4);
    assert!(diagnostics.sequential_continues >= 10);
    assert!(diagnostics.hardware_frames >= 16);
    eprintln!("Real GPU two-output catch-up and seek parity: {diagnostics:?}");
}

#[test]
#[ignore = "requires Windows D3D11VA and SYNDOCAL_GPU_TEST_VIDEO (>4 seconds)"]
fn real_gpu_loop_wraps_rewind_the_hardware_session_in_place_fifty_times() {
    let path = std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set original H264/HEVC video path");
    ffmpeg::init().unwrap();
    assert!(ffmpeg::format::input(&path).unwrap().duration() > 4_000_000);
    let mut gpu = LibavFrameDecoder::new();
    let mut request = VideoFrameRequest {
        layer_id: 903, label: "Hardware loop wrap".into(),
        source: protocol::VideoSourceSummary {
            kind: VideoSourceKind::File, path: Some(path), name: None,
            codec: Some("h264".into()), metadata: None,
        }, position_ms: 0, width: 320, height: 128,
    };
    let expected = LibavFrameDecoder::software_for_tests().decode_frame(&request).unwrap().unwrap();
    for wrap in 0..50_u64 {
        // Alternate wraps before and after the end of the played range.
        let end = if wrap % 2 == 0 { 1_000 } else { 300 };
        for position in (0..=end).step_by(33) {
            request.position_ms = position;
            let frame = gpu.decode_frame(&request).unwrap().unwrap();
            if position == 0 {
                assert_eq!((frame.width, frame.height, frame.pts_ms), (expected.width, expected.height, expected.pts_ms), "wrap {wrap}");
                let maximum = frame.data.iter().zip(&expected.data).map(|(a, b)| a.abs_diff(*b)).max().unwrap();
                assert!(maximum <= 3, "wrap {wrap}: max channel delta {maximum}");
            }
        }
    }
    let diagnostics = gpu.session_diagnostics();
    assert_eq!(diagnostics.opens, 1, "{diagnostics:?}");
    assert_eq!(diagnostics.resets, 49, "{diagnostics:?}");
    assert_eq!((diagnostics.errors, diagnostics.hardware_errors), (0, 0), "{diagnostics:?}");
    assert_eq!(diagnostics.hardware_sessions, 1);
}
