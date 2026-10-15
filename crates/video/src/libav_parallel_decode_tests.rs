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
    // Four owners from earlier ticks, then a batch of four new ones: the
    // working set is exactly full, so the batch decodes in parallel.
    for decoder in [&mut one_by_one, &mut together] {
        for layer in 40..44 {
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
        decoder.decode_input_frame(&input(50, &path, 0)).unwrap();
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
        assert_eq!(together.diagnostics(), one_by_one.diagnostics());
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
fn retaining_or_releasing_joins_the_frames_being_decoded_ahead() {
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
    assert!(decoder.lookaheads.is_empty());
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
