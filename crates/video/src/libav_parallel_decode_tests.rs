//! Decoding the inputs of one tick together must equal decoding them one by
//! one: the same frames, the same errors, the same session counters and the
//! same working-set order.
use super::*;
use crate::{FfmpegCliFrameDecoder, PreferredVideoFrameDecoder};
use protocol::VideoSourceSummary;

fn fixture(name: &str, seconds: u32) -> PathBuf {
    let ffmpeg = std::env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let path = std::env::temp_dir().join(format!(
        "syndocal-libav-parallel-{name}-{}.mkv",
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

fn input(layer_id: VideoLayerId, path: &std::path::Path, position_ms: u64) -> VideoRenderInput {
    VideoRenderInput::legacy(VideoFrameRequest {
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
    })
}

fn assert_same_state(together: &LibavFrameDecoder, one_by_one: &LibavFrameDecoder, label: &str) {
    assert_eq!(
        together.session_diagnostics(),
        one_by_one.session_diagnostics(),
        "{label}"
    );
    assert_eq!(together.cache_len(), one_by_one.cache_len(), "{label}");
    assert_eq!(
        together.working_set_lru, one_by_one.working_set_lru,
        "{label}: working-set order"
    );
}

#[test]
fn decoding_a_tick_together_equals_decoding_it_one_by_one() {
    let first = fixture("a", 3);
    let second = fixture("b", 2);
    let missing = std::env::temp_dir().join(format!(
        "syndocal-libav-parallel-missing-{}.mkv",
        std::process::id()
    ));
    let mut one_by_one = LibavFrameDecoder::software_for_tests();
    let mut together = LibavFrameDecoder::software_for_tests();
    // Opens, continuations, exact repeats, steps inside one frame, backward
    // and forward seeks, a walk to the end of a clip, positions past the end
    // and a missing file.
    let ticks: [[u64; 4]; 9] = [
        [0, 0, 1_900, 0],
        [33, 16, 1_933, 33],
        [33, 20, 1_966, 66],
        [66, 1_500, 2_000, 100],
        [100, 1_533, 2_033, 133],
        [2_900, 1_566, 2_066, 166],
        [10_000, 0, 2_100, 200],
        [10_033, 16, 50, 233],
        [10_066, 20, 83, 266],
    ];
    for (tick, positions) in ticks.iter().enumerate() {
        let fourth = if tick == 3 { &missing } else { &second };
        let inputs = vec![
            input(1, &first, positions[0]),
            input(2, &first, positions[1]),
            input(3, &second, positions[2]),
            input(4, fourth, positions[3]),
        ];
        let expected = inputs
            .iter()
            .map(|input| one_by_one.decode_input_frame(input))
            .collect::<Vec<_>>();
        let actual = together.decode_input_frames(&inputs);
        assert!(
            expected
                .iter()
                .filter(|result| matches!(result, Ok(Some(_))))
                .count()
                >= 3,
            "tick {tick} must decode real frames"
        );
        assert_eq!(actual, expected, "tick {tick}");
        assert_same_state(&together, &one_by_one, &format!("tick {tick}"));
    }
    let counters = together.session_diagnostics();
    assert!(counters.opens > 4, "{counters:?}");
    assert!(counters.resets > 0, "{counters:?}");
    assert!(counters.sequential_continues > 0, "{counters:?}");
    assert!(counters.frame_reuses > 0, "{counters:?}");
    assert!(counters.errors > 0, "{counters:?}");
}

#[test]
fn a_batch_with_other_owners_in_the_working_set_keeps_its_order() {
    let path = fixture("lru", 1);
    let mut one_by_one = LibavFrameDecoder::software_for_tests();
    let mut together = LibavFrameDecoder::software_for_tests();
    // Fill all but four retained owners, then decode four new ones in parallel.
    // This must exercise the exact retention boundary even when it changes.
    for decoder in [&mut one_by_one, &mut together] {
        for layer in 40..40 + (LIBAV_WORKING_SET_CAPACITY as u64 - 4) {
            decoder.decode_input_frame(&input(layer, &path, 0)).unwrap();
        }
    }
    let batch = (1..5)
        .map(|layer| input(layer, &path, layer * 33))
        .collect::<Vec<_>>();
    let expected = batch
        .iter()
        .map(|input| one_by_one.decode_input_frame(input))
        .collect::<Vec<_>>();
    assert_eq!(together.decode_input_frames(&batch), expected);
    assert_same_state(&together, &one_by_one, "after the batch");
    // A new owner now evicts the least recently used one in both.
    for decoder in [&mut one_by_one, &mut together] {
        decoder.decode_input_frame(&input(100, &path, 0)).unwrap();
        decoder.decode_input_frame(&input(40, &path, 33)).unwrap();
    }
    assert_same_state(&together, &one_by_one, "after eviction");
    assert!(together.session_diagnostics().evictions > 0);
}

#[test]
fn shared_or_oversized_ticks_decode_one_by_one() {
    let path = fixture("c", 1);
    let mut one_by_one = LibavFrameDecoder::software_for_tests();
    let mut together = LibavFrameDecoder::software_for_tests();
    // The same layer twice in one tick, and more owners than the working set.
    let shared = vec![input(7, &path, 0), input(7, &path, 33)];
    let oversized = (0..=LIBAV_WORKING_SET_CAPACITY as u64)
        .map(|layer| input(20 + layer, &path, layer * 33))
        .collect::<Vec<_>>();
    for inputs in [shared, oversized] {
        let expected = inputs
            .iter()
            .map(|input| one_by_one.decode_input_frame(input))
            .collect::<Vec<_>>();
        assert_eq!(together.decode_input_frames(&inputs), expected);
        assert_same_state(&together, &one_by_one, "one by one");
    }
}

#[test]
fn the_preferred_decoder_batches_file_inputs_with_the_same_fallbacks() {
    let first = fixture("p", 1);
    let corrupt = std::env::temp_dir().join(format!(
        "syndocal-libav-parallel-corrupt-{}.mkv",
        std::process::id()
    ));
    std::fs::write(&corrupt, b"not a video").unwrap();
    let missing_cli = || FfmpegCliFrameDecoder::new("missing-parallel-test-ffmpeg.exe");
    let mut one_by_one = PreferredVideoFrameDecoder::with_software_libav_for_tests(missing_cli());
    let mut together = PreferredVideoFrameDecoder::with_software_libav_for_tests(missing_cli());
    for position in [0, 33, 66] {
        // The corrupt file fails in libav and then in the CLI fallback.
        let inputs = vec![
            input(1, &first, position),
            input(2, &corrupt, position),
            input(3, &first, position + 16),
        ];
        let expected = inputs
            .iter()
            .map(|input| one_by_one.decode_input_frame(input))
            .collect::<Vec<_>>();
        assert!(matches!(expected[0], Ok(Some(_))));
        assert!(expected[1].is_err());
        assert_eq!(together.decode_input_frames(&inputs), expected);
        let mut batch = together.diagnostics();
        let mut serial = one_by_one.diagnostics();
        assert_eq!(batch.decode_call_count, position / 33 + 1);
        assert_eq!(serial.decode_call_count, 3 * (position / 33 + 1));
        // Wall latency and call count deliberately differ with batching;
        // every decode/fallback/error/session counter must still agree.
        batch.decode_wall_us = 0;
        serial.decode_wall_us = 0;
        batch.decode_call_count = 0;
        serial.decode_call_count = 0;
        assert_eq!(batch, serial);
    }
    assert!(together.diagnostics().cli_fallback_failures > 0);
    let _ = std::fs::remove_file(&corrupt);
}

#[test]
fn the_preferred_decoder_keeps_libav_frames_shared() {
    let path = fixture("preferred-shared", 1);
    let mut decoder = PreferredVideoFrameDecoder::with_software_libav_for_tests(
        FfmpegCliFrameDecoder::new("missing-shared-test-ffmpeg.exe"),
    );
    let first = decoder
        .decode_input_frame_shared(&input(1, &path, 16))
        .unwrap()
        .unwrap();
    for position in [16, 20] {
        let again = decoder
            .decode_input_frame_shared(&input(1, &path, position))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again), "position {position}");
    }
    let batch = decoder.decode_input_frames_shared(&[input(1, &path, 20), input(2, &path, 0)]);
    assert!(Arc::ptr_eq(
        &first,
        batch[0].as_ref().unwrap().as_ref().unwrap()
    ));
}

#[test]
fn reused_and_repeated_frames_are_shared_not_copied() {
    let path = fixture("shared", 1);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder
        .decode_input_frame_shared(&input(1, &path, 0))
        .unwrap();
    let first = decoder
        .decode_input_frame_shared(&input(1, &path, 16))
        .unwrap()
        .unwrap();
    // An exact repeat, and a later position the decoded frame already
    // covers, reuse it.
    for position in [16, 20] {
        let again = decoder
            .decode_input_frame_shared(&input(1, &path, position))
            .unwrap()
            .unwrap();
        assert!(Arc::ptr_eq(&first, &again), "position {position}");
    }
    assert!(decoder.session_diagnostics().frame_reuses >= 2);
    // The owned form is an equal copy.
    assert_eq!(
        decoder
            .decode_input_frame(&input(1, &path, 20))
            .unwrap()
            .unwrap(),
        *first
    );
}

/// 60 Hz ticks over 30 fps clips: play, pause, a small step back, double
/// speed, a jump, a loop and a run past the end of both clips.
fn playback_ticks() -> Vec<[u64; 2]> {
    let (mut a, mut b) = (0_u64, 0_u64);
    let mut ticks = Vec::new();
    for tick in 0..150 {
        match tick {
            0..=29 => {
                a += 17;
                b += 17;
            }
            30..=39 => {}
            40 => a -= 10,
            41..=59 => {
                a += 33;
                b += 17;
            }
            60 => a = 1_500,
            61..=79 => {
                a += 17;
                b += 50;
            }
            80 => b = 0,
            _ => {
                a += 17;
                b += 17;
            }
        }
        ticks.push([a, b]);
    }
    ticks
}

#[test]
fn decoding_ahead_returns_exactly_the_frames_decoding_on_demand_does() {
    let first = fixture("ahead-a", 2);
    let second = fixture("ahead-b", 1);
    let mut on_demand = LibavFrameDecoder::software_for_tests();
    let mut ahead = LibavFrameDecoder::software_for_tests();
    for (tick, [a, b]) in playback_ticks().into_iter().enumerate() {
        let inputs = vec![input(1, &first, a), input(2, &second, b)];
        let expected = inputs
            .iter()
            .map(|input| on_demand.decode_input_frame(input))
            .collect::<Vec<_>>();
        // Both live entry points, the batch and one input at a time.
        let actual = if tick % 3 == 0 {
            ahead.decode_input_frames_shared(&inputs)
        } else {
            inputs
                .iter()
                .map(|input| ahead.decode_input_frame_shared(input))
                .collect()
        };
        let actual = actual
            .into_iter()
            .map(|decoded| decoded.map(|frame| frame.map(Arc::unwrap_or_clone)))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "tick {tick} at {a} / {b}");
    }
    let counters = ahead.session_diagnostics();
    assert!(counters.lookahead_hits > 20, "{counters:?}");
    assert!(counters.lookahead_decodes >= counters.lookahead_hits);
    // On-demand decoding never decodes ahead.
    assert_eq!(on_demand.session_diagnostics().lookahead_decodes, 0);
}

#[test]
fn retaining_preserves_active_lookaheads_and_releasing_reaps_removed_owners() {
    let path = fixture("ahead-release", 1);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder
        .decode_input_frame_shared(&input(1, &path, 0))
        .unwrap();
    decoder
        .decode_input_frame_shared(&input(2, &path, 0))
        .unwrap();
    assert_eq!(decoder.session_diagnostics().active_sessions, 2);
    decoder.retain_inputs(&[input(2, &path, 0)]);
    assert_eq!(decoder.session_diagnostics().active_sessions, 1);
    // Retention no longer forces an active owner's speculative decode to
    // finish. The removed owner is acknowledged and cannot return later.
    assert_eq!(decoder.lookaheads.len(), 1);
    assert_eq!(decoder.lookaheads[0].layer_id, 2);
    assert!(decoder.sessions.iter().all(|session| session.layer_id == 2));
    decoder.release_layer(2);
    assert_eq!(decoder.session_diagnostics().active_sessions, 0);
}

fn assert_same_frames(
    on_demand: &mut LibavFrameDecoder,
    ahead: &mut LibavFrameDecoder,
    inputs: &[VideoRenderInput],
    label: &str,
) {
    let expected = inputs
        .iter()
        .map(|input| on_demand.decode_input_frame(input))
        .collect::<Vec<_>>();
    let actual = ahead
        .decode_input_frames_shared(inputs)
        .into_iter()
        .map(|decoded| decoded.map(|frame| frame.map(Arc::unwrap_or_clone)))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{label}");
}

#[test]
fn a_transition_on_one_legacy_owner_never_decodes_ahead_from_the_other_clip() {
    // As the app's decoder hands a clip transition over: the outgoing and the
    // incoming clip on the same legacy owner, far apart in time.
    let outgoing = fixture("transition-out", 3);
    let incoming = fixture("transition-in", 3);
    let mut on_demand = LibavFrameDecoder::software_for_tests();
    let mut ahead = LibavFrameDecoder::software_for_tests();
    for tick in 0..40_u64 {
        let inputs = vec![
            input(1, &outgoing, 2_000 + tick * 17),
            input(1, &incoming, tick * 17),
        ];
        assert_same_frames(&mut on_demand, &mut ahead, &inputs, &format!("tick {tick}"));
    }
    assert_eq!(ahead.session_diagnostics().lookahead_decodes, 0);
    // After the transition the incoming clip plays on and decodes ahead.
    for tick in 40..60_u64 {
        let inputs = vec![input(1, &incoming, tick * 17)];
        assert_same_frames(&mut on_demand, &mut ahead, &inputs, &format!("tick {tick}"));
    }
    assert!(ahead.session_diagnostics().lookahead_hits > 0);
}

#[test]
fn an_owner_requested_twice_in_a_tick_does_not_decode_ahead() {
    let path = fixture("twice", 2);
    let mut on_demand = LibavFrameDecoder::software_for_tests();
    let mut ahead = LibavFrameDecoder::software_for_tests();
    assert_same_frames(
        &mut on_demand,
        &mut ahead,
        &[input(1, &path, 1_000), input(1, &path, 500)],
        "backward pair",
    );
    for position in [517, 533, 550] {
        assert_same_frames(
            &mut on_demand,
            &mut ahead,
            &[input(1, &path, position)],
            &format!("at {position}"),
        );
    }
}

#[test]
fn the_end_of_a_clip_reopens_no_more_often_than_on_demand() {
    // FFV1 is intra-only, so the last frame comes out before the end of the
    // stream is reached.
    let path = fixture("end", 1);
    let mut on_demand = LibavFrameDecoder::software_for_tests();
    let mut ahead = LibavFrameDecoder::software_for_tests();
    for tick in 0..90_u64 {
        // Play to the end, hold past it, then loop back to the start.
        let position = if tick < 75 {
            tick * 17
        } else {
            (tick - 75) * 17
        };
        assert_same_frames(
            &mut on_demand,
            &mut ahead,
            &[input(1, &path, position)],
            &format!("tick {tick} at {position}"),
        );
    }
    let ahead_counters = ahead.session_diagnostics();
    let on_demand_counters = on_demand.session_diagnostics();
    assert!(
        ahead_counters.opens <= on_demand_counters.opens,
        "{ahead_counters:?} vs {on_demand_counters:?}"
    );
}

#[test]
fn sixteen_playing_inputs_retain_their_sessions_across_ticks() {
    let path = fixture("sixteen-retained", 2);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    for position_ms in [0, 33, 66, 100] {
        let inputs = (1..=16)
            .map(|layer| input(layer, &path, position_ms))
            .collect::<Vec<_>>();
        let frames = decoder.decode_input_frames(&inputs);
        assert_eq!(frames.len(), 16);
        assert!(frames.iter().all(|frame| matches!(frame, Ok(Some(_)))));
        let counters = decoder.session_diagnostics();
        assert_eq!(
            counters.opens, 16,
            "sessions must not churn at {position_ms}ms"
        );
        assert_eq!(counters.evictions, 0);
        assert_eq!(counters.active_sessions, 16);
    }
    for layer in 1..=16 {
        decoder.release_input(&input(layer, &path, 100));
    }
    assert_eq!(decoder.session_diagnostics().active_sessions, 0);
    assert_eq!(decoder.cache_len(), 0);
}

#[test]
fn sixteen_live_shared_inputs_keep_lookahead_owners_until_release() {
    let path = fixture("sixteen-shared-retained", 2);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    for position_ms in [0, 33, 66, 100] {
        let inputs = (1..=16)
            .map(|layer| input(layer, &path, position_ms))
            .collect::<Vec<_>>();
        let frames = decoder.decode_input_frames_shared(&inputs);
        assert_eq!(frames.len(), 16);
        assert!(frames.iter().all(|frame| matches!(frame, Ok(Some(_)))));
        let counters = decoder.session_diagnostics();
        assert_eq!(
            counters.opens, 16,
            "live sessions must not churn at {position_ms}ms"
        );
        assert_eq!(counters.evictions, 0);
        assert_eq!(counters.active_sessions, 16);
    }
    let counters = decoder.session_diagnostics();
    assert!(counters.lookahead_decodes >= 16);
    assert!(counters.lookahead_hits > 0);
    for layer in 1..=16 {
        decoder.release_input(&input(layer, &path, 100));
    }
    assert_eq!(decoder.session_diagnostics().active_sessions, 0);
    assert!(
        decoder.lookaheads.is_empty(),
        "release must reap every live owner"
    );
    assert_eq!(decoder.cache_len(), 0);
}

#[test]
fn decode_timing_counts_one_demand_call_for_a_mixed_batch() {
    let path = fixture("timed-mixed", 1);
    let mut decoder = PreferredVideoFrameDecoder::with_software_libav_for_tests(
        FfmpegCliFrameDecoder::new("missing-timing-test-ffmpeg.exe"),
    );
    let mut deferred = input(2, &path, 0);
    deferred.request.source.kind = VideoSourceKind::Camera;
    let frames = decoder.decode_input_frames(&[input(1, &path, 0), deferred]);
    assert!(matches!(frames[0], Ok(Some(_))));
    assert!(matches!(frames[1], Ok(None)));
    let diagnostics = decoder.diagnostics();
    assert_eq!(
        diagnostics.decode_call_count, 1,
        "mixed batches must not double count nested calls"
    );
    assert_eq!(diagnostics.total_requests, 2);
    assert!(diagnostics.decode_wall_us > 0);
    let _ = decoder.decode_input_frame(&input(1, &path, 33)).unwrap();
    assert_eq!(decoder.diagnostics().decode_call_count, 2);
    assert!(decoder.diagnostics().decode_wall_us >= diagnostics.decode_wall_us);
}

/// Component demand latency only: no output, compositor, monitor, or real-time
/// pacing. Run explicitly in release; never interpret these numbers as fps.
#[test]
#[ignore = "explicit release QA benchmark; requires SYNDOCAL_PERF_MEDIA_DIR and SYNDOCAL_PERF_RESULT"]
fn task20_decode_demand_benchmark() {
    let media =
        PathBuf::from(std::env::var_os("SYNDOCAL_PERF_MEDIA_DIR").expect("media directory"));
    let result = PathBuf::from(std::env::var_os("SYNDOCAL_PERF_RESULT").expect("result JSON path"));
    let expected_capacity: usize = std::env::var("SYNDOCAL_PERF_EXPECT_CAPACITY")
        .expect("expected source capacity")
        .parse()
        .expect("integer source capacity");
    assert_eq!(
        LIBAV_WORKING_SET_CAPACITY, expected_capacity,
        "stale benchmark executable"
    );
    // The supervising QA process opens this gate after builds and browser
    // checks settle, keeping their resource contention out of the samples.
    let gate = result.with_extension("ready");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    while !gate.is_file() && std::time::Instant::now() < deadline {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    assert!(
        gate.is_file(),
        "QA start gate was not opened: {}",
        gate.display()
    );
    let mut rows = Vec::new();
    for source in ["h264_1080p30.mp4", "h264_2160p30.mp4"] {
        let path = media.join(source);
        assert!(path.is_file(), "missing QA media: {}", path.display());
        for layers in [1_u64, 3, 6, 8, 12, 16] {
            let mut decoder = PreferredVideoFrameDecoder::new(FfmpegCliFrameDecoder::default());
            let mut times_ms = Vec::new();
            let mut errors = Vec::new();
            let mut baseline = None;
            for tick in 0..20 {
                let inputs = (1..=layers)
                    .map(|layer| {
                        let mut request = input(layer, &path, tick * 17);
                        request.request.width = 1920;
                        request.request.height = 1080;
                        request.request.source.codec = Some("h264".into());
                        request
                    })
                    .collect::<Vec<_>>();
                let started = std::time::Instant::now();
                let frames = decoder.decode_input_frames_shared(&inputs);
                let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                for frame in frames {
                    if !matches!(frame, Ok(Some(_))) && errors.len() < 3 {
                        errors.push(format!("{frame:?}"));
                    }
                }
                if tick == 3 {
                    baseline = Some(decoder.diagnostics());
                }
                if tick >= 4 {
                    times_ms.push(elapsed);
                }
            }
            times_ms.sort_by(f64::total_cmp);
            let percentile = |p: usize| times_ms[(times_ms.len() * p).div_ceil(100) - 1];
            let diagnostics = decoder.diagnostics();
            let baseline = baseline.unwrap();
            let row = serde_json::json!({
                "source": source, "layers": layers, "samples": times_ms.len(),
                "workingSetCapacity": LIBAV_WORKING_SET_CAPACITY,
                "decodeDemandP50Ms": percentile(50), "decodeDemandP95Ms": percentile(95),
                "decodeDemandP99Ms": percentile(99), "errors": errors,
                "measuredOpens": diagnostics.libav_session_open_count - baseline.libav_session_open_count,
                "measuredEvictions": diagnostics.libav_working_set_eviction_count - baseline.libav_working_set_eviction_count,
                "diagnostics": diagnostics,
            });
            println!("TASK20_DECODE {row}");
            rows.push(row);
            // Explicit retirement joins every lookahead before the next case;
            // dropping receivers alone does not wait for queued pool work.
            decoder.retain_layers(&[]);
            assert_eq!(decoder.diagnostics().libav_session_count, 0);
        }
    }
    std::fs::write(result, serde_json::to_vec_pretty(&rows).unwrap()).unwrap();
}

/// A real native session behind a deterministic, test-only worker admission
/// gate. Drop always releases and reaps it, including assertion failures.
struct HeldLookahead {
    release: Option<std::sync::mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl HeldLookahead {
    fn release(&mut self) {
        if let Some(release) = self.release.take() {
            let _ = release.send(());
        }
        if let Some(worker) = self.worker.take() {
            worker.join().unwrap();
        }
    }
}

impl Drop for HeldLookahead {
    fn drop(&mut self) {
        self.release();
    }
}

fn hold_next_frame(
    decoder: &mut LibavFrameDecoder,
    input: &VideoRenderInput,
    served: &Arc<VideoFrame>,
) -> HeldLookahead {
    assert!(decoder.lookaheads.is_empty());
    let index = decoder.session_index(input.key, input.layer_id()).unwrap();
    let mut session = decoder.sessions.swap_remove(index);
    let last_request_ms = session.last_request_ms;
    let mut request = input.request.clone();
    request.position_ms = served.pts_ms.saturating_add(1);
    let terminal = Arc::clone(served);
    let (release, gate) = std::sync::mpsc::channel();
    let (sender, done) = std::sync::mpsc::sync_channel(1);
    let worker = std::thread::spawn(move || {
        let _ = gate.recv();
        let mut hardware_frames = 0;
        let result = decode_libav_session_frame(
            &request,
            &mut session,
            false,
            Some(&terminal),
            &mut hardware_frames,
        );
        let _ = sender.send(LibavLookaheadDone {
            session,
            result,
            hardware_frames,
        });
    });
    decoder.lookaheads.push(LibavLookahead {
        key: input.key,
        layer_id: input.layer_id(),
        served_pts_ms: served.pts_ms,
        last_request_ms,
        hardware: false,
        done,
    });
    HeldLookahead {
        release: Some(release),
        worker: Some(worker),
    }
}

#[test]
fn covered_60hz_tail_ticks_do_not_join_or_synchronously_decode_pending_lookahead() {
    let path = fixture("pending-covered", 2);
    let first = input(1, &path, 16);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    let served = Arc::new(decoder.decode_input_frame(&first).unwrap().unwrap());
    // The real cache owns a separate Arc in the owned entry point.
    let cached = Arc::clone(&decoder.entries[0].frame);
    assert_eq!(served.pts_ms, 33);
    let mut gate = hold_next_frame(&mut decoder, &first, &cached);
    let before = decoder.session_diagnostics();
    let (finished, ready) = std::sync::mpsc::sync_channel(1);
    let path_worker = path.clone();
    let checking = std::thread::spawn(move || {
        decoder.retain_inputs(std::slice::from_ref(&first));
        decoder.retain_layers(&[1]);
        let frames = [16, 20, 33].map(|position| {
            decoder
                .decode_input_frame_shared(&input(1, &path_worker, position))
                .unwrap()
                .unwrap()
        });
        let saved = decoder.lookaheads[0].last_request_ms;
        let counters = decoder.session_diagnostics();
        finished.send((frames, saved, counters)).unwrap();
        decoder
    });
    // Success must arrive while no worker is allowed to decode. The timeout is
    // only a regression watchdog; the gate, not elapsed speed, proves reuse.
    let result = ready.recv_timeout(std::time::Duration::from_secs(2));
    gate.release();
    let mut decoder = checking.join().unwrap();
    let (frames, saved, after) = result.expect("covered ticks must finish before worker admission");
    assert!(frames.iter().all(|frame| Arc::ptr_eq(&cached, frame)));
    assert_eq!(
        saved,
        Some(33),
        "join must restore the latest covered request"
    );
    assert_eq!(after.sequential_continues, before.sequential_continues);
    assert_eq!(after.opens, before.opens);
    assert_eq!(after.lookahead_decodes, before.lookahead_decodes);
    assert_eq!(after.frame_reuses - before.frame_reuses, 3);
    assert_eq!(decoder.lookaheads.len(), 1);
    assert_eq!(decoder.entries[0].position_ms, 33);

    let mut oracle = LibavFrameDecoder::software_for_tests();
    for position in [16, 20, 33] {
        oracle
            .decode_input_frame(&input(1, &path, position))
            .unwrap();
    }
    // Passed-next catch-up, exact PTS ties, a seek, loop reversal and EOF all
    // retain the on-demand oracle. Integer rational targets cover 60/30 Hz.
    // Reversal inside the cached PTS must reset from the saved 33ms request,
    // rather than reuse as if the worker's original saved 16ms still applied.
    let resets = decoder.session_diagnostics().resets;
    let positions = [
        20, 34, 50, 66, 67, 100, 1_500, 0, 16, 33, 50, 1_966, 2_100, 0,
    ];
    for position in positions
        .into_iter()
        .chain((0..120).map(|tick| tick * 1_000 / 60))
    {
        assert_same_frames(
            &mut oracle,
            &mut decoder,
            &[input(1, &path, position)],
            &format!("after delayed worker at {position}"),
        );
    }
    assert!(decoder.session_diagnostics().resets > resets);
    decoder.retain_inputs(&[]);
    assert_eq!(decoder.session_diagnostics().active_sessions, 0);
}

#[test]
fn pending_cached_reuse_rejects_changed_identity_extent_signature_and_backward_time() {
    let path = fixture("pending-invalid", 2);
    let other_path = fixture("pending-other", 2);
    let first = input(1, &path, 16);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder.decode_input_frame(&first).unwrap().unwrap();
    let served = Arc::clone(&decoder.entries[0].frame);
    let mut gate = hold_next_frame(&mut decoder, &first, &served);
    let signature = StillImageSignature::from_path(&path);
    let target = input(1, &path, 20);
    let mut extent = target.clone();
    extent.request.width += 1;
    let mut owner = target.clone();
    owner.key = protocol::VideoRenderInputKey {
        project_render_epoch: 1,
        input_id: protocol::VideoRenderInputId(999),
    };
    let mut layer = target.clone();
    layer.request.layer_id = 2;
    for invalid in [
        extent,
        owner,
        layer,
        input(1, &path, 0),
        input(1, &path, 67),
    ] {
        assert!(decoder
            .reuse_pending_cached_frame(&invalid, &path, &signature)
            .is_none());
    }
    assert!(decoder
        .reuse_pending_cached_frame(&target, &other_path, &signature)
        .is_none());
    let mut changed_signature = signature.clone();
    changed_signature.len = Some(changed_signature.len.unwrap_or(0).wrapping_add(1));
    assert!(decoder
        .reuse_pending_cached_frame(&target, &path, &changed_signature)
        .is_none());
    assert_eq!(decoder.lookaheads[0].last_request_ms, Some(16));
    assert_eq!(decoder.entries[0].position_ms, 16);
    gate.release();
    let mut oracle = LibavFrameDecoder::software_for_tests();
    oracle.decode_input_frame(&first).unwrap();
    let mut resized = target;
    resized.request.width += 1;
    assert_same_frames(&mut oracle, &mut decoder, &[resized], "extent invalidation");
    assert_same_frames(
        &mut oracle,
        &mut decoder,
        &[input(1, &other_path, 20)],
        "path replacement",
    );
}

#[test]
fn pending_cached_stop_reaps_worker_and_never_returns_success() {
    let path = fixture("pending-stop", 1);
    let external = Arc::new(AtomicBool::new(false));
    let mut decoder = LibavFrameDecoder::software_for_tests().with_stop_signal(external.clone());
    let first = input(1, &path, 16);
    decoder.decode_input_frame(&first).unwrap().unwrap();
    let served = Arc::clone(&decoder.entries[0].frame);
    let mut gate = hold_next_frame(&mut decoder, &first, &served);
    external.store(true, std::sync::atomic::Ordering::Release);
    assert!(decoder
        .reuse_pending_cached_frame(
            &input(1, &path, 20),
            &path,
            &StillImageSignature::from_path(&path)
        )
        .is_none());
    let (finished, ready) = std::sync::mpsc::sync_channel(1);
    let checking = std::thread::spawn(move || {
        let result = decoder.decode_input_frame_shared(&first);
        finished.send(result).unwrap();
        decoder
    });
    assert!(matches!(
        ready.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    gate.release();
    let decoder = checking.join().unwrap();
    assert_eq!(ready.recv().unwrap(), Err(VideoDecodeError::Cancelled));
    assert!(decoder.lookaheads.is_empty());
    drop(decoder);
    assert_eq!(Arc::strong_count(&external), 1);
}

#[test]
fn removing_a_pending_owner_waits_for_acknowledgement_and_does_not_restore_it() {
    let path = fixture("pending-retire", 1);
    let first = input(1, &path, 16);
    let mut decoder = LibavFrameDecoder::software_for_tests();
    decoder.decode_input_frame(&first).unwrap().unwrap();
    let served = Arc::clone(&decoder.entries[0].frame);
    let mut gate = hold_next_frame(&mut decoder, &first, &served);
    let (started, starting) = std::sync::mpsc::sync_channel(1);
    let (finished, ready) = std::sync::mpsc::sync_channel(1);
    let checking = std::thread::spawn(move || {
        started.send(()).unwrap();
        decoder.retain_inputs(&[]);
        finished.send(decoder.session_diagnostics()).unwrap();
        decoder
    });
    starting.recv().unwrap();
    let before_release = ready.recv_timeout(std::time::Duration::from_millis(100));
    gate.release();
    let decoder = checking.join().unwrap();
    assert!(matches!(
        before_release,
        Err(std::sync::mpsc::RecvTimeoutError::Timeout)
    ));
    let counters = ready.recv().unwrap();
    assert_eq!(counters.active_sessions, 0);
    assert_eq!(counters.lookahead_discards, 1);
    assert_eq!(decoder.cache_len(), 0);
    assert!(decoder.lookaheads.is_empty());
    assert!(decoder.sessions.is_empty());
    assert!(decoder.working_set_lru.is_empty());
}
#[cfg(feature = "qa-native-frame-timing")]
#[test]
fn native_frame_timing_links_real_parallel_decoder_and_provider_cache_requests() {
    use crate::native_frame_timing as timing;
    use crate::VideoFrameProvider;
    let _serial = timing::TIMING_TEST_SERIAL.lock().unwrap();
    let path = fixture("request-timing", 2);
    let mut provider =
        crate::DecoderBackedFrameProvider::new(LibavFrameDecoder::software_for_tests());
    timing::enable_for_tests();
    timing::start().unwrap();
    timing::prepare_thread().unwrap();
    for position in [0, 16, 20, 100, 0] {
        let inputs = [input(1, &path, position), input(2, &path, position)];
        let _parent = timing::begin_parent(10, 1, 48, 27);
        {
            let _retention = timing::span(timing::Phase::Retention);
            provider.retain_inputs(&inputs);
        }
        {
            let _demand = timing::span(timing::Phase::Demand);
            provider.decode_inputs_ahead(&inputs);
        }
        for input in &inputs {
            let _demand = timing::span(timing::Phase::Demand);
            provider
                .decode_or_placeholder_for_input_shared(input)
                .unwrap();
        }
        timing::set_outcome(timing::Outcome::Presented);
    }
    provider.retain_inputs(&[]);
    timing::stop_capture().unwrap();
    timing::retire_thread();
    timing::acknowledge_stop().unwrap();
    let snapshot = timing::snapshot().unwrap();
    assert_eq!(snapshot.parents.len(), 5);
    for parent in &snapshot.parents {
        assert_eq!(parent.flags, 0, "{parent:?}");
        assert_eq!(parent.decoder_requests.len(), 4);
        for index in 0..2 {
            let request = &parent.decoder_requests[index];
            let access = &parent.decoder_requests[index + 2];
            assert_eq!(request.layer_id, index as u64 + 1);
            assert_eq!(access.paths, timing::DECODER_PROVIDER);
            assert_eq!(access.source_request_id, Some(index as u8));
            assert_eq!(access.selected_pts_ms, request.selected_pts_ms);
            assert_eq!(access.work_count, 0);
            assert_ne!(access.demand_span_id, request.demand_span_id);
            assert!(request.complete);
            if request.paths & timing::DECODER_SYNC != 0 {
                assert_eq!(request.work_count, 1);
                assert_ne!(request.work.worker_id, 0);
                assert_eq!(
                    u64::from(request.work.decode_io_us)
                        + u64::from(request.work.hardware_download_us)
                        + u64::from(request.work.swscale_us)
                        + u64::from(request.work.alloc_copy_us),
                    request.work.end_us - request.work.start_us
                );
            } else {
                assert_eq!(request.work_count, 0);
            }
        }
    }
    for (parent, expected) in snapshot.parents.iter().zip([
        timing::DECODER_SYNC | timing::DECODER_REOPEN,
        timing::DECODER_PREFETCH,
        timing::DECODER_CACHED,
        timing::DECODER_SYNC,
        timing::DECODER_SYNC | timing::DECODER_RESET,
    ]) {
        assert!(
            parent.decoder_requests[..2]
                .iter()
                .all(|request| request.paths == expected),
            "{parent:?}"
        );
    }
}
