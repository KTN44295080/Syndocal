use super::*;
use audio::ltc::LtcTimecode;

fn frame(hours: u8, minutes: u8, seconds: u8, frames: u8, end_sample: u64) -> LtcFrame {
    LtcFrame {
        timecode: LtcTimecode {
            hours,
            minutes,
            seconds,
            frames,
            drop_frame: false,
        },
        frame_rate: Some(LtcFrameRate::Fps25),
        end_sample,
    }
}

#[test]
fn a_frame_chases_to_its_end_plus_the_time_since_it_ended() {
    // 01:00:00:10 at 25 fps ends at 3_600_440 ms; it ended 480 samples
    // (10 ms at 48 kHz) before the end of the block.
    let decoded = frame(1, 0, 0, 10, 10_000);
    let (position_ms, label) =
        chase_position_ms(&decoded, LtcFrameRate::Fps25, 10_480, 48_000).unwrap();
    assert_eq!(position_ms, 3_600_440 + 10);
    assert_eq!(label.label(), "01:00:00:10");
}

#[test]
fn drop_frame_labels_chase_to_real_time() {
    let mut decoded = frame(0, 10, 0, 0, 0);
    decoded.timecode.drop_frame = true;
    // 00:10:00;00 is real frame 17,982; its end is frame 17,983.
    let (position_ms, label) =
        chase_position_ms(&decoded, LtcFrameRate::Fps2997Drop, 0, 48_000).unwrap();
    assert_eq!(position_ms, (17_983 * 1_001 + 29) / 30);
    assert_eq!(label.label(), "00:10:00;00");
}

#[test]
fn a_jump_needs_a_second_frame_to_confirm_it() {
    let mut gate = ContinuityGate::default();
    // The first frame (or the first after a dropout) needs confirming too.
    assert!(!gate.accept(100));
    assert!(gate.accept(101));
    // One corrupt frame far away is held back, and the stream continuing
    // past it (101, X, 103) is still accepted.
    assert!(!gate.accept(9_000));
    assert!(gate.accept(103));
    // A real jump is accepted on its second frame.
    assert!(!gate.accept(50_000));
    assert!(gate.accept(50_001));
    assert!(gate.accept(50_002));
    gate.reset();
    assert!(!gate.accept(7));
    assert!(gate.accept(8));
}

#[test]
fn labels_the_frame_rate_cannot_have_are_ignored() {
    // 00:01:00;00 does not exist in drop-frame; 00:10:00;00 does.
    let mut skipped = frame(0, 1, 0, 0, 0);
    skipped.timecode.drop_frame = true;
    assert!(chase_position_ms(&skipped, LtcFrameRate::Fps2997Drop, 0, 48_000).is_none());
    let mut tenth = frame(0, 10, 0, 1, 0);
    tenth.timecode.drop_frame = true;
    assert!(chase_position_ms(&tenth, LtcFrameRate::Fps2997Drop, 0, 48_000).is_some());
    // Frame 27 does not exist at 25 fps.
    assert!(chase_position_ms(&frame(0, 0, 1, 27, 0), LtcFrameRate::Fps25, 0, 48_000).is_none());
    assert!(chase_position_ms(&frame(0, 0, 1, 27, 0), LtcFrameRate::Fps30, 0, 48_000).is_some());
}
