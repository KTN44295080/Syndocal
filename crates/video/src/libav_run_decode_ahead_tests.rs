//! Look-ahead runs (task67, live C4R5 defect D2): when the requests of an
//! input are farther apart than one source frame (a render tick slower than
//! the clip's frame rate), a single next frame never covers the next request
//! and every request decodes on the caller's thread, which keeps the tick
//! slow. A run decodes consecutive frames up to the next expected request.
//! These tests prove the run returns exactly the on-demand frames, never needs
//! a backward seek when the cadence changes, answers slow-cadence requests
//! from the run, and keeps the stop/retirement contract of a single frame.
use super::*;
use protocol::VideoSourceSummary;
use std::sync::atomic::Ordering;

fn fixture(name: &str, seconds: u32) -> PathBuf {
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let path = std::env::temp_dir().join(format!(
        "syndocal-libav-run-{name}-{}.mkv",
        std::process::id()
    ));
    let generated = std::process::Command::new(&ffmpeg)
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            &format!("testsrc2=size=48x27:rate=30:duration={seconds}"),
            "-c:v",
            "ffv1",
            "-g",
            "10",
            "-y",
        ])
        .arg(&path)
        .output()
        .expect("FFmpeg fixture generator must be available");
    assert!(
        generated.status.success(),
        "fixture generation failed: {}",
        String::from_utf8_lossy(&generated.stderr)
    );
    path
}

/// A resident-style keyed input (not the legacy owner).
fn keyed(layer_id: VideoLayerId, path: &std::path::Path, position_ms: u64) -> VideoRenderInput {
    VideoRenderInput {
        key: protocol::VideoRenderInputKey {
            project_render_epoch: 7,
            input_id: protocol::VideoRenderInputId(100 + layer_id),
        },
        request: VideoFrameRequest {
            layer_id,
            label: format!("Layer {layer_id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(path.to_string_lossy().into_owned()),
                name: None,
                codec: Some("ffv1".into()),
                metadata: None,
            },
            position_ms,
            width: 48,
            height: 27,
        },
        isf_effect: None,
        clip_slot_id: None,
    }
}

fn owned(
    frames: Vec<Result<Option<Arc<VideoFrame>>, VideoDecodeError>>,
) -> Vec<Result<Option<VideoFrame>, VideoDecodeError>> {
    frames
        .into_iter()
        .map(|decoded| decoded.map(|frame| frame.map(Arc::unwrap_or_clone)))
        .collect()
}

/// Three layers one by one (the resident route) and as a batch (the legacy
/// route) at the given request positions must equal on-demand decoding.
fn assert_cadence_matches_on_demand(
    path: &std::path::Path,
    positions: &[u64],
    label: &str,
) -> (LibavSessionDiagnostics, LibavSessionDiagnostics) {
    let mut on_demand = LibavFrameDecoder::software_for_tests();
    let mut one_by_one = LibavFrameDecoder::software_for_tests();
    let mut batch = LibavFrameDecoder::software_for_tests();
    for (tick, &position) in positions.iter().enumerate() {
        let inputs = (1..=3)
            .map(|layer| keyed(layer, path, position))
            .collect::<Vec<_>>();
        let expected = inputs
            .iter()
            .map(|input| on_demand.decode_input_frame(input))
            .collect::<Vec<_>>();
        assert!(
            expected.iter().all(|frame| matches!(frame, Ok(Some(_)))),
            "{label} tick {tick} must decode real frames"
        );
        let resident = owned(
            inputs
                .iter()
                .map(|input| {
                    one_by_one.decode_input_frame_shared_with_cancellation(input, &|| false)
                })
                .collect(),
        );
        assert_eq!(
            resident, expected,
            "{label}: one by one, tick {tick} at {position}"
        );
        let legacy = owned(batch.decode_input_frames_shared(&inputs));
        assert_eq!(
            legacy, expected,
            "{label}: batch, tick {tick} at {position}"
        );
    }
    let counters = (
        one_by_one.session_diagnostics(),
        batch.session_diagnostics(),
    );
    assert_eq!(on_demand.session_diagnostics().lookahead_decodes, 0);
    // Changing the cadence never rewinds or reopens a look-ahead session.
    let reference = on_demand.session_diagnostics();
    for counters in [counters.0, counters.1] {
        assert_eq!(counters.opens, reference.opens, "{label}: {counters:?}");
        assert_eq!(counters.resets, reference.resets, "{label}: {counters:?}");
        assert_eq!(counters.errors, 0, "{label}: {counters:?}");
        assert!(
            counters.lookahead_decodes >= counters.lookahead_hits,
            "{label}: {counters:?}"
        );
    }
    one_by_one.retain_inputs(&[]);
    batch.retain_inputs(&[]);
    for decoder in [&one_by_one, &batch] {
        assert!(
            decoder.lookaheads.is_empty(),
            "{label}: retirement reaps runs"
        );
        assert_eq!(decoder.session_diagnostics().active_sessions, 0);
    }
    counters
}

#[test]
fn slow_ticks_are_answered_by_the_run_with_exactly_the_on_demand_frames() {
    // 85 ms between requests at 30 fps: every request is past the single next
    // frame. This was the live 3-layer cadence: 0 hits, all on demand.
    let path = fixture("slow", 3);
    let positions = (0..30).map(|tick| tick * 85).collect::<Vec<_>>();
    let (resident, legacy) = assert_cadence_matches_on_demand(&path, &positions, "85 ms");
    for counters in [resident, legacy] {
        // Three layers, 29 continuations each; only the first request after
        // each open (no stride yet) can miss.
        assert!(
            counters.lookahead_hits >= 3 * 27,
            "slow requests must be answered ahead: {counters:?}"
        );
        assert!(counters.sequential_continues >= counters.lookahead_hits);
    }
}

#[test]
fn changing_cadence_stays_exact_without_backward_seeks() {
    let path = fixture("cadence", 4);
    // Slow to fast (the recovery a run makes possible), fast to slow, jitter,
    // a 250 ms stall, a repeated position, the clip end and past it.
    let mut positions = Vec::new();
    let mut position = 0;
    for stride in [
        100, 100, 100, 17, 16, 17, 17, 50, 120, 33, 33, 0, 16, 250, 17, 85, 90, 80, 85, 17, 17,
        200, 17, 34, 66, 99, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 17, 85, 85, 85, 85,
        85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85, 85,
    ] {
        position += stride;
        positions.push(position);
    }
    assert!(
        *positions.last().unwrap() > 4_000,
        "the walk passes the clip end"
    );
    let (resident, legacy) = assert_cadence_matches_on_demand(&path, &positions, "cadence");
    for counters in [resident, legacy] {
        assert!(counters.lookahead_hits > 30, "{counters:?}");
    }
}

#[test]
fn sixty_hertz_requests_of_a_30_fps_clip_decode_ahead_without_waste() {
    // At 60 Hz a 30 fps clip's run keeps two source frames of slack (C4R7):
    // the frames after the next decode while the next is served, so a frame
    // is ready well before it is first requested. Every decoded frame is
    // still used: at most the last run per owner is unused.
    let path = fixture("sixty", 2);
    let positions = (0..100).map(|tick| tick * 1_000 / 60).collect::<Vec<_>>();
    let (resident, legacy) = assert_cadence_matches_on_demand(&path, &positions, "60 Hz");
    for counters in [resident, legacy] {
        assert!(counters.lookahead_hits >= 3 * 45, "{counters:?}");
        // Unused: at most the frames still queued ahead when the walk ends.
        assert!(
            counters.lookahead_decodes
                <= counters.lookahead_hits + 3 * LIBAV_LOOKAHEAD_MAX_FRAMES as u64,
            "60 Hz decodes ahead without waste: {counters:?}"
        );
    }
}

#[test]
fn run_depth_is_bounded_per_owner_and_by_the_working_set_budget() {
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let output_frame = 2_560 * 1_600 * 4;
    assert_eq!(
        decoder.lookahead_frame_limit(48 * 27 * 4),
        LIBAV_LOOKAHEAD_MAX_FRAMES
    );
    assert_eq!(
        decoder.lookahead_frame_limit(output_frame),
        LIBAV_LOOKAHEAD_MAX_FRAMES
    );
    let key = protocol::VideoRenderInputKey::LEGACY;
    for layer in 1..=LIBAV_WORKING_SET_CAPACITY as u64 {
        decoder.touch_input(key, layer);
    }
    // Sixteen output-sized owners share 256 MiB: the single next frame.
    assert_eq!(decoder.lookahead_frame_limit(output_frame), 1);
    let small = decoder.lookahead_frame_limit(1_920 * 1_080 * 4);
    assert!((1..=2).contains(&small), "{small}");
}

#[test]
fn a_streaming_run_is_reaped_on_stop_and_never_returns_success() {
    let path = fixture("run-stop", 3);
    let external = Arc::new(AtomicBool::new(false));
    let mut decoder = LibavFrameDecoder::software_for_tests().with_stop_signal(external.clone());
    for position in [0, 100, 200] {
        decoder
            .decode_input_frame_shared(&keyed(1, &path, position))
            .unwrap()
            .unwrap();
    }
    // A run with a stride of 100 ms is pending with a stream.
    assert_eq!(decoder.lookaheads.len(), 1);
    assert!(decoder.lookaheads[0].frames.is_some());
    external.store(true, Ordering::Release);
    assert_eq!(
        decoder.decode_input_frame_shared(&keyed(1, &path, 300)),
        Err(VideoDecodeError::Cancelled)
    );
    assert!(decoder.lookaheads.is_empty());
    drop(decoder);
    assert_eq!(Arc::strong_count(&external), 1);
}

#[test]
fn a_ready_frame_answers_while_the_rest_of_the_run_is_still_decoding() {
    let path = fixture("run-ready", 3);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let mut oracle = LibavFrameDecoder::software_for_tests();
    for position in [0, 120, 240] {
        let input = keyed(1, &path, position);
        assert_eq!(
            owned(vec![decoder.decode_input_frame_shared(&input)]),
            vec![oracle.decode_input_frame(&input)]
        );
    }
    // The run after 240 ms looks 180 ms ahead. Wait until it has streamed a
    // frame, then ask for that frame: it is answered from the run, which is
    // not joined.
    let pending = &mut decoder.lookaheads[0];
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    while pending.ready.is_empty() {
        pending.receive_streamed();
        assert!(
            std::time::Instant::now() < deadline,
            "the run streams a frame"
        );
        std::thread::yield_now();
    }
    let streamed = Arc::clone(&pending.ready[0]);
    let before = decoder.session_diagnostics();
    let input = keyed(1, &path, streamed.pts_ms);
    let frame = decoder.decode_input_frame_shared(&input).unwrap().unwrap();
    assert!(Arc::ptr_eq(&frame, &streamed));
    assert_eq!(
        Arc::unwrap_or_clone(frame),
        oracle.decode_input_frame(&input).unwrap().unwrap()
    );
    let after = decoder.session_diagnostics();
    assert_eq!(after.lookahead_hits, before.lookahead_hits + 1);
    assert_eq!(after.sequential_continues, before.sequential_continues + 1);
    for position in (streamed.pts_ms + 120..2_900).step_by(120) {
        let input = keyed(1, &path, position);
        assert_eq!(
            owned(vec![decoder.decode_input_frame_shared(&input)]),
            vec![oracle.decode_input_frame(&input)],
            "at {position}"
        );
    }
    assert_eq!(
        decoder.session_diagnostics().resets,
        oracle.session_diagnostics().resets
    );
}

/// Loop playback: positions wrap to the clip start like a looping layer's
/// playhead (C4R7, live 3-layer Display: every 12 s wrap decoded the clip
/// start on the render tick, one layer after another, 70-90 ms).
fn looping_positions(clip_ms: u64, stride_ms: u64, ticks: u64) -> Vec<u64> {
    (0..ticks)
        .map(|tick| (tick * stride_ms) % clip_ms)
        .collect()
}

#[test]
fn loop_wraps_are_answered_from_the_clip_start_decoded_ahead_exactly() {
    // 2 s clip at 30 fps, key frames every 10 frames; 60 Hz-like and 85 ms
    // strides over three loops. Each wrap equals on-demand decoding, keeps
    // the same rewinds/opens (assert_cadence_matches_on_demand), and is a
    // look-ahead hit rather than a decode on the caller.
    let path = fixture("wrap", 2);
    for (stride, ticks) in [(17_u64, 360_u64), (85, 75)] {
        let positions = looping_positions(2_000, stride, ticks);
        let wraps = positions
            .windows(2)
            .filter(|pair| pair[1] < pair[0])
            .count() as u64;
        assert!(wraps >= 2, "{stride} ms walks past at least two wraps");
        let label = format!("{stride} ms loop");
        assert_cadence_matches_on_demand(&path, &positions, &label);
    }
    // Wrap by wrap, one owner: the clip start is decoded ahead before the
    // wrap and answers it without a decode on the caller.
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let mut oracle = LibavFrameDecoder::software_for_tests();
    let positions = looping_positions(2_000, 17, 360);
    let mut answered = 0;
    for pair in positions.windows(2) {
        let wrapped = pair[1] < pair[0];
        if wrapped {
            // The run that read to the clip end has decoded the clip start.
            decoder.join_lookaheads();
            assert!(
                decoder
                    .sessions
                    .iter()
                    .all(|session| session.wrap.is_some()),
                "clip start decoded ahead before the wrap at {}",
                pair[0]
            );
        }
        let input = keyed(1, &path, pair[1]);
        let before = decoder.session_diagnostics();
        assert_eq!(
            owned(vec![decoder.decode_input_frame_shared(&input)]),
            vec![oracle.decode_input_frame(&input)],
            "at {}",
            pair[1]
        );
        if wrapped {
            let after = decoder.session_diagnostics();
            assert_eq!(after.resets, before.resets + 1, "a wrap is still a rewind");
            assert_eq!(
                after.lookahead_hits,
                before.lookahead_hits + 1,
                "the wrap is answered from the clip start decoded ahead"
            );
            assert!(decoder
                .sessions
                .iter()
                .all(|session| session.wrap.is_none()));
            answered += 1;
        }
    }
    assert!(answered >= 2);
    assert_eq!(
        decoder.session_diagnostics().opens,
        oracle.session_diagnostics().opens
    );
}

#[test]
fn a_run_reaching_the_declared_end_decodes_the_clip_start_before_any_request_passes_it() {
    // 60 Hz requests that stop before the clip's last frame time: no request
    // ever finds the clip end itself, so only the run's declared-end read can
    // have decoded the clip start (C4R7: the wrap then never waits for it).
    let path = fixture("wrap-early", 2);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let mut oracle = LibavFrameDecoder::software_for_tests();
    for position in (0..=1_890).step_by(17) {
        let input = keyed(1, &path, position);
        assert_eq!(
            owned(vec![decoder.decode_input_frame_shared(&input)]),
            vec![oracle.decode_input_frame(&input)],
            "at {position}"
        );
    }
    decoder.join_lookaheads();
    assert!(
        decoder
            .sessions
            .iter()
            .all(|session| session.wrap.is_some()),
        "the run read on to the clip end and decoded the clip start"
    );
    // The loop wrap is then answered from it, exactly.
    let before = decoder.session_diagnostics();
    let input = keyed(1, &path, 4);
    assert_eq!(
        owned(vec![decoder.decode_input_frame_shared(&input)]),
        vec![oracle.decode_input_frame(&input)]
    );
    assert_eq!(
        decoder.session_diagnostics().lookahead_hits,
        before.lookahead_hits + 1
    );
}

#[test]
fn a_wrap_past_another_key_frame_is_refused_and_rewinds_on_demand() {
    // The clip start answers a wrap only when the on-demand rewind would
    // start from the same key frame: no queued key frame other than the
    // front at or before the target.
    let path = fixture("wrap-keys", 2);
    let input = keyed(1, &path, 0);
    let mut session = open_libav_session(
        input.key,
        &input.request,
        path.clone(),
        StillImageSignature::from_path(&path),
        false,
    )
    .unwrap();
    let frame = |pts_ms| {
        Arc::new(VideoFrame {
            layer_id: 1,
            width: 1,
            height: 1,
            pts_ms,
            duration_ms: 33,
            format: crate::VideoPixelFormat::Rgba8,
            data: vec![0; 4],
        })
    };
    let queue = |keys: [bool; 3]| LibavWrapAhead {
        frames: [0, 33, 67]
            .into_iter()
            .zip(keys)
            .map(|(pts, key)| (frame(pts), key))
            .collect(),
        eof_sent: false,
        decoder_drained: false,
    };
    // Front key frame, none after: any covered target is answered with the
    // first frame at or after it, and the rest stays queued.
    session.wrap = Some(queue([true, false, false]));
    let answered = session.take_wrapped(20).unwrap();
    assert_eq!(answered.pts_ms, 33);
    assert_eq!(session.next_frames.len(), 1);
    assert_eq!(session.last_request_ms, Some(20));
    assert!(session.wrap.is_none() && !session.rewind_pending);
    // A key frame at or before the target: refused, consumed.
    session.wrap = Some(queue([true, true, false]));
    assert!(session.take_wrapped(40).is_none());
    assert!(session.wrap.is_none());
    // A key frame after the target does not matter.
    session.wrap = Some(queue([true, false, true]));
    assert_eq!(session.take_wrapped(10).unwrap().pts_ms, 33);
    // Not from a key frame, or past the queue: refused.
    session.wrap = Some(queue([false, false, false]));
    assert!(session.take_wrapped(0).is_none());
    session.wrap = Some(queue([true, false, false]));
    assert!(session.take_wrapped(68).is_none());
}

/// QA cadence bench (task67): the real C4/C5 media at the output extent,
/// three keyed layers one by one (the resident route) or as a batch (the
/// legacy route), request positions from the wall clock, a simulated
/// non-decode tick cost, an optional cold-start stall. Reports ms per decode
/// call and look-ahead hits; not a present-FPS measurement.
#[test]
#[ignore = "QA bench: real media in SYNDOCAL_QA_C4_MEDIA, D3D11VA, 2560x1600 frames"]
fn qa_wall_clock_cadence_bench() {
    use std::time::{Duration, Instant};
    let media = PathBuf::from(std::env::var_os("SYNDOCAL_QA_C4_MEDIA").expect("media path"));
    let seconds = std::env::var("SYNDOCAL_QA_CADENCE_SECONDS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(8_u64);
    for (layers, tick_work_ms, stall_ms, batch) in [
        (1_u64, 10_u64, 0_u64, false),
        (3, 12, 0, false),
        (3, 12, 300, false),
        (3, 60, 0, false),
        (3, 30, 0, true),
        (3, 30, 300, true),
    ] {
        let mut decoder = LibavFrameDecoder::new();
        let started = Instant::now();
        let (mut ticks, mut call_ms, mut tick_ms) = (0_u64, Vec::new(), Vec::new());
        while started.elapsed() < Duration::from_secs(seconds) {
            let tick = Instant::now();
            let position = started.elapsed().as_millis() as u64;
            let inputs = (1..=layers)
                .map(|layer| {
                    let mut input = keyed(layer, &media, position);
                    input.request.width = 2_560;
                    input.request.height = 1_600;
                    input.request.source.codec = Some("h264".into());
                    input
                })
                .collect::<Vec<_>>();
            if batch {
                let call = Instant::now();
                assert!(decoder
                    .decode_input_frames_shared(&inputs)
                    .iter()
                    .all(|frame| matches!(frame, Ok(Some(_)))));
                call_ms.push(call.elapsed().as_secs_f64() * 1_000.0);
            } else {
                for input in &inputs {
                    let call = Instant::now();
                    decoder.decode_input_frame_shared(input).unwrap().unwrap();
                    call_ms.push(call.elapsed().as_secs_f64() * 1_000.0);
                }
            }
            if ticks == 0 && stall_ms > 0 {
                std::thread::sleep(Duration::from_millis(stall_ms));
            }
            std::thread::sleep(Duration::from_millis(tick_work_ms));
            if let Some(rest) = Duration::from_micros(16_667).checked_sub(tick.elapsed()) {
                std::thread::sleep(rest);
            }
            tick_ms.push(tick.elapsed().as_secs_f64() * 1_000.0);
            ticks += 1;
        }
        let mean = |values: &[f64]| {
            let tail = &values[values.len() / 5..];
            tail.iter().sum::<f64>() / tail.len() as f64
        };
        let counters = decoder.session_diagnostics();
        println!(
            "D2 cadence layers={layers} work={tick_work_ms}ms stall={stall_ms}ms batch={batch}: \
             {:.1} ticks/s, tick {:.1} ms, call {:.2} ms, look-ahead hits {}/{} decodes, \
             reuse {}, continues {}, resets {}",
            ticks as f64 / seconds as f64,
            mean(&tick_ms),
            mean(&call_ms),
            counters.lookahead_hits,
            counters.lookahead_decodes,
            counters.frame_reuses,
            counters.sequential_continues,
            counters.resets,
        );
    }
}
