use super::*;
use crate::midi::{mtc_full_frame_timecode, MtcQuarterFrameDecoder};

const RATES: [MtcFrameRate; 4] = [
    MtcFrameRate::Fps24,
    MtcFrameRate::Fps25,
    MtcFrameRate::Fps2997Drop,
    MtcFrameRate::Fps30,
];

fn tc(hours: u8, minutes: u8, seconds: u8, frames: u8, frame_rate: MtcFrameRate) -> MtcTimecode {
    MtcTimecode {
        hours,
        minutes,
        seconds,
        frames,
        frame_rate,
    }
}

fn playing(position_ms: u64, generation: u64) -> MtcTransportObservation {
    MtcTransportObservation {
        playing: true,
        position_ms,
        generation,
    }
}

#[test]
fn drop_frame_labels_skip_two_numbers_each_minute_except_every_tenth() {
    let df = MtcFrameRate::Fps2997Drop;
    assert_eq!(
        MtcTimecode::from_frame_count(1_799, df),
        tc(0, 0, 59, 29, df)
    );
    assert_eq!(MtcTimecode::from_frame_count(1_800, df), tc(0, 1, 0, 2, df));
    assert_eq!(
        MtcTimecode::from_frame_count(17_982, df),
        tc(0, 10, 0, 0, df)
    );
    assert_eq!(tc(0, 1, 0, 2, df).frame_count(), 1_800);
    assert_eq!(tc(0, 10, 0, 0, df).frame_count(), 17_982);
    // Ten drop-frame minutes are 17,982 real frames: 599.9994 s, so the
    // first whole millisecond of that frame is 600,000.
    assert_eq!(tc(0, 10, 0, 0, df).position_ms(), 600_000);
    assert_eq!(tc(1, 2, 3, 4, df).label(), "01:02:03;04");
    assert_eq!(tc(1, 2, 3, 4, MtcFrameRate::Fps25).label(), "01:02:03:04");
}

#[test]
fn every_frame_round_trips_through_its_label_and_position() {
    for rate in RATES {
        for frame in (0..rate.frames_per_day()).step_by(997).chain([
            rate.frames_per_day() - 1,
            1_799,
            1_800,
            17_981,
            17_982,
        ]) {
            let timecode = MtcTimecode::from_frame_count(frame, rate);
            assert!(
                timecode.frames < rate.nominal_fps() as u8,
                "{rate:?} {frame}"
            );
            assert_eq!(timecode.frame_count(), frame, "{rate:?} {timecode:?}");
            let start = rate.frame_start_ms(frame);
            assert_eq!(rate.frame_at_ms(start), frame, "{rate:?} {frame}");
            assert_eq!(MtcTimecode::from_position_ms(start, rate), timecode);
            assert_eq!(
                MtcTimecode::from_position_ms(timecode.position_ms(), rate),
                timecode
            );
        }
        // Wraps at 24 hours.
        assert_eq!(
            MtcTimecode::from_frame_count(rate.frames_per_day(), rate),
            tc(0, 0, 0, 0, rate)
        );
    }
}

#[test]
fn quarter_and_full_frames_decode_to_the_sent_timecode() {
    for rate in RATES {
        let timecode = MtcTimecode::from_frame_count(1_234_567 % rate.frames_per_day(), rate);
        let mut decoder = MtcQuarterFrameDecoder::default();
        let mut decoded = None;
        for piece in 0..8 {
            decoded = decoder.push_message(&quarter_frame_message(timecode, piece));
        }
        // Receivers add the two frames the sequence took to send.
        assert_eq!(decoded, Some(timecode.advanced(2)), "{rate:?}");
        assert_eq!(
            mtc_full_frame_timecode(&full_frame_message(timecode)),
            Some(timecode),
            "{rate:?}"
        );
    }
}

/// Plays the transport from `start_ms` for `duration_ms`, observed at 22.7 ms
/// engine ticks and polled every millisecond. Returns the quarter frames with
/// their send times in milliseconds since the start.
fn simulate(rate: MtcFrameRate, start_ms: u64, duration_ms: u64) -> Vec<(f64, [u8; 2])> {
    let t0 = Instant::now();
    let mut transmitter = MtcTransmitter::new(rate);
    let first = transmitter.observe(playing(start_ms, 1), t0);
    assert!(matches!(first, Some(MtcMessage::FullFrame(_))));
    let mut sent = Vec::new();
    let mut next_tick = 0.0_f64;
    for millisecond in 0..duration_ms {
        let elapsed = millisecond as f64;
        let now = t0 + Duration::from_millis(millisecond);
        if elapsed >= next_tick {
            // The engine publishes the position at its tick.
            let observed = transmitter.observe(playing(start_ms + millisecond, 1), now);
            assert_eq!(observed, None, "steady playback never relocates");
            next_tick += 22.7;
        }
        for message in transmitter.due(now) {
            let MtcMessage::QuarterFrame(bytes) = message else {
                panic!("quarter frames only while playing");
            };
            sent.push((elapsed, bytes));
        }
    }
    sent
}

#[test]
fn playback_sends_four_evenly_spaced_quarter_frames_per_frame() {
    for rate in RATES {
        let sent = simulate(rate, 3_600_000, 2_000);
        let quarter_ms = rate.frame_duration_ms() / 4.0;
        let expected = 2_000.0 / quarter_ms;
        // Up to one frame is lost waiting for the first frame boundary.
        assert!(
            (sent.len() as f64 - expected).abs() <= 5.0,
            "{rate:?}: {} quarter frames, expected about {expected}",
            sent.len()
        );
        for pair in sent.windows(2) {
            let gap = pair[1].0 - pair[0].0;
            assert!(
                (gap - quarter_ms).abs() <= 1.5,
                "{rate:?}: gap {gap} ms, expected {quarter_ms}"
            );
            // Pieces go 0..7 in order.
            assert_eq!(
                (pair[0].1[1] >> 4).wrapping_add(1) & 0x07,
                pair[1].1[1] >> 4,
                "{rate:?}"
            );
        }
    }
}

#[test]
fn a_receiver_tracks_the_transport_within_one_frame() {
    for rate in RATES {
        let start_ms = 3_600_000;
        let sent = simulate(rate, start_ms, 3_000);
        let mut decoder = MtcQuarterFrameDecoder::default();
        let mut checked = 0;
        for (elapsed, bytes) in sent {
            if let Some(decoded) = decoder.push_message(&bytes) {
                let transport_ms = start_ms as f64 + elapsed;
                let error = decoded.position_ms() as f64 - transport_ms;
                assert!(
                    error.abs() <= rate.frame_duration_ms() + 1.0,
                    "{rate:?}: decoded {} vs transport {transport_ms} ({error} ms)",
                    decoded.position_ms()
                );
                checked += 1;
            }
        }
        assert!(checked > 20, "{rate:?}: {checked}");
    }
}

#[test]
fn stop_and_jumps_send_one_full_frame_and_stop_quarter_frames() {
    let rate = MtcFrameRate::Fps25;
    let t0 = Instant::now();
    let mut transmitter = MtcTransmitter::new(rate);
    assert!(transmitter.observe(playing(10_000, 1), t0).is_some());
    assert!(!transmitter.due(t0 + Duration::from_millis(40)).is_empty());
    // Re-reading the same tick does nothing.
    assert_eq!(transmitter.observe(playing(10_000, 1), t0), None);
    // A new transport generation alone (a tempo change) does not relocate.
    assert_eq!(
        transmitter.observe(playing(10_005, 2), t0 + Duration::from_millis(5)),
        None
    );
    // A seek relocates at once.
    let seek = transmitter.observe(playing(70_000, 2), t0 + Duration::from_millis(10));
    assert_eq!(
        seek,
        Some(MtcMessage::FullFrame(full_frame_message(
            MtcTimecode::from_position_ms(70_002, rate)
        )))
    );
    // A large jump without a new generation relocates too.
    let jump = transmitter.observe(playing(90_000, 2), t0 + Duration::from_millis(30));
    assert!(matches!(jump, Some(MtcMessage::FullFrame(_))));
    // Stopping sends the stop position once and nothing more.
    let stop = MtcTransportObservation {
        playing: false,
        position_ms: 90_040,
        generation: 2,
    };
    let at = t0 + Duration::from_millis(60);
    assert!(matches!(
        transmitter.observe(stop, at),
        Some(MtcMessage::FullFrame(_))
    ));
    assert_eq!(transmitter.observe(stop, at), None);
    assert!(transmitter.due(at + Duration::from_secs(1)).is_empty());
    assert_eq!(transmitter.next_due(), None);
    assert!(!transmitter.is_running());
}

#[test]
fn a_frozen_playhead_holds_quarter_frames_until_it_moves_again() {
    // A count-in or a stalled external source: playing, but not moving.
    let rate = MtcFrameRate::Fps25;
    let t0 = Instant::now();
    let mut transmitter = MtcTransmitter::new(rate);
    assert!(transmitter.observe(playing(5_000, 1), t0).is_some());
    for millisecond in (4..100).step_by(4) {
        let now = t0 + Duration::from_millis(millisecond);
        assert_eq!(transmitter.observe(playing(5_000, 1), now), None);
    }
    let frozen_at = t0 + Duration::from_millis(100);
    assert!(matches!(
        transmitter.observe(playing(5_000, 1), frozen_at),
        Some(MtcMessage::FullFrame(_))
    ));
    assert!(!transmitter.is_running());
    assert!(transmitter
        .due(frozen_at + Duration::from_secs(2))
        .is_empty());
    // Moving again re-anchors with a full frame and resumes.
    let resumed = frozen_at + Duration::from_secs(2);
    assert!(matches!(
        transmitter.observe(playing(5_023, 1), resumed),
        Some(MtcMessage::FullFrame(_))
    ));
    assert!(transmitter.is_running());
    // A fresh start is not frozen, however long the position stood still.
    let mut fresh = MtcTransmitter::new(rate);
    let stopped = MtcTransportObservation {
        playing: false,
        position_ms: 0,
        generation: 1,
    };
    fresh.observe(stopped, t0);
    let start = t0 + Duration::from_secs(60);
    assert!(fresh.observe(playing(0, 2), start).is_some());
    assert!(fresh.is_running());
}

#[test]
fn a_stall_resumes_at_the_current_frame_without_a_burst() {
    let rate = MtcFrameRate::Fps30;
    let t0 = Instant::now();
    let mut transmitter = MtcTransmitter::new(rate);
    transmitter.observe(playing(0, 1), t0);
    // The first sequence starts at the next frame boundary (frame 1).
    assert!(transmitter.due(t0).is_empty());
    assert_eq!(transmitter.due(t0 + Duration::from_millis(36)).len(), 1);
    let late = t0 + Duration::from_millis(500);
    let messages = transmitter.due(late);
    assert!(messages.len() <= 2, "{}", messages.len());
    let MtcMessage::QuarterFrame(first) = messages[0] else {
        panic!("quarter frame expected");
    };
    // Piece 0 of the frame showing at 500 ms (frame 15).
    assert_eq!(
        first,
        quarter_frame_message(MtcTimecode::from_frame_count(15, rate), 0)
    );
}
