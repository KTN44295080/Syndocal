//! Loop wraps rewind the open session in place. These tests pin the real wrap
//! (including after EOF), repeated wraps, start-time offsets and the explicit
//! re-open fallback when the rewind seek cannot work.
use super::*;
use std::process::Command;

fn ffmpeg() -> std::ffi::OsString {
    std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into())
}

fn fixture_request(path: &std::path::Path, codec: &str, frame_rate: f32, duration_ms: u64) -> VideoFrameRequest {
    VideoFrameRequest {
        layer_id: 7101,
        label: "Loop wrap fixture".into(),
        source: protocol::VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(path.to_string_lossy().into_owned()),
            name: None,
            codec: Some(codec.into()),
            metadata: Some(protocol::VideoMediaMetadata {
                duration_ms: Some(duration_ms),
                width: Some(32),
                height: Some(18),
                frame_rate: Some(frame_rate),
                has_audio: false,
            }),
        },
        position_ms: 0,
        width: 32,
        height: 18,
    }
}

fn generate(name: &str, args: &[&str]) -> std::path::PathBuf {
    let (stem, extension) = name.rsplit_once('.').expect("fixture name needs an extension");
    let path = std::env::temp_dir().join(format!("syndocal-loop-wrap-{stem}-{}.{extension}", std::process::id()));
    let output = Command::new(ffmpeg())
        .args(["-v", "error", "-f", "lavfi", "-i"])
        .args(args)
        .args(["-y"])
        .arg(&path)
        .output()
        .expect("FFmpeg fixture generator must be available");
    assert!(output.status.success(), "{name} fixture: {}", String::from_utf8_lossy(&output.stderr));
    path
}

fn bframe_mp4(test: &str) -> std::path::PathBuf {
    generate(
        &format!("bframes-{test}.mp4"),
        &["testsrc2=size=32x18:rate=30:duration=3", "-c:v", "mpeg4", "-pix_fmt", "yuv420p", "-bf", "2", "-g", "30", "-q:v", "2"],
    )
}

fn ts_fixture(label: &str, offset: &str) -> std::path::PathBuf {
    generate(
        &format!("{label}.ts"),
        &[
            "testsrc2=size=32x18:rate=10:duration=2", "-c:v", "mpeg2video", "-bf", "2", "-g", "10",
            "-output_ts_offset", offset, "-avoid_negative_ts", "disabled", "-muxdelay", "0", "-muxpreload", "0",
        ],
    )
}

fn fresh_frame(request: &VideoFrameRequest) -> VideoFrame {
    LibavFrameDecoder::software_for_tests().decode_frame(request).unwrap().unwrap()
}

fn assert_same_frame(actual: &VideoFrame, expected: &VideoFrame, context: &str) {
    assert_eq!(actual.pts_ms, expected.pts_ms, "{context}: PTS");
    assert_eq!((actual.width, actual.height, actual.format), (expected.width, expected.height, expected.format), "{context}");
    assert_eq!(actual.data, expected.data, "{context}: pixels");
}

fn decode_at(decoder: &mut LibavFrameDecoder, request: &mut VideoFrameRequest, position_ms: u64) -> VideoFrame {
    request.position_ms = position_ms;
    decoder.decode_frame(request).unwrap().unwrap()
}

#[test]
fn real_loop_wraps_rewind_in_place_before_and_after_eof_fifty_times() {
    let path = bframe_mp4("fifty");
    let mut request = fixture_request(&path, "mpeg4", 30.0, 3_000);
    let first = fresh_frame(&request);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    request.layer_id = 7101;
    for wrap in 0..50_u64 {
        // Odd wraps leave from the middle of the clip; even wraps play through EOF.
        let end = if wrap % 2 == 0 { 3_500 } else { 1_500 };
        for position in (0..=end).step_by(33) {
            let frame = decode_at(&mut decoder, &mut request, position);
            if position == 0 {
                assert_same_frame(&frame, &first, &format!("wrap {wrap} position 0"));
                assert!(!decoder.sessions[0].eof_sent && !decoder.sessions[0].decoder_drained, "wrap {wrap}: rewound session must not stay drained");
            }
        }
    }
    assert_eq!(decoder.session_counters.opens, 1, "every wrap reuses the open session");
    assert_eq!(decoder.session_counters.errors, 0);
    assert_eq!(decoder.session_counters.resets, 49, "one in-place rewind per wrap after the first pass");
    let _ = std::fs::remove_file(path);
}

#[test]
fn start_time_offset_fixtures_wrap_to_position_zero_with_reference_parity() {
    for (label, offset) in [("positive", "1"), ("negative", "-1")] {
        let path = ts_fixture(label, offset);
        let mut request = fixture_request(&path, "mpeg2video", 10.0, 2_000);
        let first = fresh_frame(&request);
        let mut decoder = LibavFrameDecoder::software_for_tests();
        for wrap in 0..20_u64 {
            for position in (0..=1_900).step_by(50) {
                let frame = decode_at(&mut decoder, &mut request, position);
                if position == 0 {
                    assert_same_frame(&frame, &first, &format!("{label} wrap {wrap}"));
                }
            }
        }
        assert_eq!(decoder.session_counters.errors, 0, "{label}");
        if label == "positive" {
            assert_eq!(decoder.session_counters.opens, 1, "{label}: in-place rewind");
        } else {
            // A negative start time cannot seek to 0; every wrap then takes the
            // explicit re-open fallback and must still return the right frame.
            assert!(decoder.session_counters.opens <= 20, "{label}: at most one re-open per wrap");
        }
        let _ = std::fs::remove_file(path);
    }
}

#[test]
fn failed_rewind_seek_reopens_and_returns_the_correct_frame_before_and_after_eof() {
    let path = bframe_mp4("failed-seek");
    let mut request = fixture_request(&path, "mpeg4", 30.0, 3_000);
    let first = fresh_frame(&request);
    for (label, end) in [("before EOF", 1_500_u64), ("after EOF", 3_500_u64)] {
        let mut decoder = LibavFrameDecoder::software_for_tests();
        for position in (0..=end).step_by(33) {
            decode_at(&mut decoder, &mut request, position);
        }
        let opens = decoder.session_counters.opens;
        FORCE_REWIND_SEEK_FAILURE.with(|flag| flag.set(true));
        let wrapped = decode_at(&mut decoder, &mut request, 0);
        FORCE_REWIND_SEEK_FAILURE.with(|flag| flag.set(false));
        assert_same_frame(&wrapped, &first, &format!("{label}: wrap with a failing seek"));
        assert_eq!(decoder.session_counters.opens, opens + 1, "{label}: the failed rewind re-opens the media");
        assert_eq!(decoder.session_counters.errors, 0, "{label}: recovered, not an error");
        assert_eq!(decoder.sessions.len(), 1, "{label}");
        assert!(!decoder.sessions[0].decoder_drained && !decoder.sessions[0].rewind_pending, "{label}");
        // The replacement session keeps serving the loop normally.
        let next = decode_at(&mut decoder, &mut request, 33);
        request.position_ms = 33;
        assert_same_frame(&next, &fresh_frame(&request), &format!("{label}: frame after the re-open"));
        assert_eq!(decoder.session_counters.opens, opens + 1, "{label}: no further re-open");
    }
    let _ = std::fs::remove_file(path);
}
