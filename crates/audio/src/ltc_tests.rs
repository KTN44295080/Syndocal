use super::*;

fn tc(hours: u8, minutes: u8, seconds: u8, frames: u8, drop_frame: bool) -> LtcTimecode {
    LtcTimecode {
        hours,
        minutes,
        seconds,
        frames,
        drop_frame,
    }
}

/// Consecutive labels from `start` at `nominal` labels per second (no
/// drop-frame skipping needed for the short runs used here).
fn run(start: LtcTimecode, nominal: u8, count: usize) -> Vec<LtcTimecode> {
    let mut current = start;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        out.push(current);
        current.frames += 1;
        if current.frames == nominal {
            current.frames = 0;
            current.seconds += 1;
            if current.seconds == 60 {
                current.seconds = 0;
                current.minutes += 1;
            }
        }
    }
    out
}

struct Signal {
    sample_rate: u32,
    frames_per_second: f64,
    amplitude: f32,
    invert: bool,
    noise: f32,
}

/// Biphase-mark LTC audio for `frames`, band-limited by a one-pole filter.
fn synthesize(signal: &Signal, frames: &[LtcTimecode]) -> Vec<f32> {
    let samples_per_half = f64::from(signal.sample_rate) / (signal.frames_per_second * 80.0) / 2.0;
    let mut level = if signal.invert { -1.0_f32 } else { 1.0 };
    let mut out = Vec::new();
    let mut position = 0.0_f64;
    let mut filtered = 0.0_f32;
    let mut noise_state = 0x1234_5678_u32;
    let mut emit = |level: f32, until: f64, out: &mut Vec<f32>| {
        while (out.len() as f64) < until {
            filtered += 0.6 * (level - filtered);
            noise_state = noise_state
                .wrapping_mul(1_664_525)
                .wrapping_add(1_013_904_223);
            let noise = ((noise_state >> 8) as f32 / (1 << 24) as f32 - 0.5) * 2.0 * signal.noise;
            out.push((filtered + noise) * signal.amplitude);
        }
    };
    for timecode in frames {
        let bits = ltc_frame_bits(*timecode);
        for bit in 0..80 {
            level = -level;
            position += samples_per_half;
            emit(level, position, &mut out);
            if (bits >> bit) & 1 == 1 {
                level = -level;
            }
            position += samples_per_half;
            emit(level, position, &mut out);
        }
    }
    out
}

/// Decodes `frames` followed by one more frame: a frame completes only at
/// the next frame's first level change. The very first frame can be missed
/// (no level change precedes its first cell).
fn decode(signal: &Signal, frames: &[LtcTimecode]) -> Vec<LtcFrame> {
    let mut stream = frames.to_vec();
    stream.push(LtcTimecode {
        frames: 0,
        ..*frames.last().unwrap()
    });
    let audio = synthesize(signal, &stream);
    let mut decoder = LtcDecoder::new(signal.sample_rate);
    // Feed in device-sized blocks.
    audio
        .chunks(480)
        .flat_map(|block| decoder.push_samples(block))
        .collect()
}

fn signal(sample_rate: u32, frames_per_second: f64) -> Signal {
    Signal {
        sample_rate,
        frames_per_second,
        amplitude: 0.5,
        invert: false,
        noise: 0.0,
    }
}

#[test]
fn frame_bits_round_trip_with_the_sync_word_and_even_parity() {
    for timecode in [
        tc(0, 0, 0, 0, false),
        tc(23, 59, 59, 29, true),
        tc(12, 34, 56, 24, false),
    ] {
        let bits = ltc_frame_bits(timecode);
        assert_eq!((bits >> 64) as u16, SYNC_WORD);
        assert_eq!(bits.count_ones() % 2, 0, "{timecode:?}");
        assert_eq!(decode_frame_bits(bits), Some(timecode));
    }
    // Out-of-range fields are rejected.
    let mut bad = ltc_frame_bits(tc(1, 2, 3, 4, false));
    bad |= 0b1111 << 16;
    assert_eq!(decode_frame_bits(bad), None);
}

#[test]
fn every_standard_rate_decodes_in_order_and_is_recognised() {
    for (fps, nominal, drop_frame, expected) in [
        (24.0, 24, false, LtcFrameRate::Fps24),
        (25.0, 25, false, LtcFrameRate::Fps25),
        (30_000.0 / 1_001.0, 30, true, LtcFrameRate::Fps2997Drop),
        (30.0, 30, false, LtcFrameRate::Fps30),
    ] {
        for sample_rate in [44_100, 48_000, 96_000] {
            let frames = run(tc(1, 0, 0, 2, drop_frame), nominal, 40);
            let decoded = decode(&signal(sample_rate, fps), &frames);
            let labels = decoded
                .iter()
                .map(|frame| frame.timecode)
                .collect::<Vec<_>>();
            assert!(
                labels.len() >= 39,
                "{fps} @ {sample_rate}: {}",
                labels.len()
            );
            // Everything decoded is in order and exact.
            let offset = frames.len() - labels.len();
            assert_eq!(labels, frames[offset..], "{fps} @ {sample_rate}");
            assert_eq!(
                decoded.last().unwrap().frame_rate,
                Some(expected),
                "{fps} @ {sample_rate}"
            );
            // A frame ends where the next one starts.
            let samples_per_frame = f64::from(sample_rate) / fps;
            let last = decoded.last().unwrap();
            let expected_end = samples_per_frame * frames.len() as f64;
            assert!(
                (last.end_sample as f64 - expected_end).abs() <= samples_per_frame / 40.0 + 2.0,
                "{fps} @ {sample_rate}: end {} vs {expected_end}",
                last.end_sample
            );
        }
    }
}

#[test]
fn quiet_inverted_noisy_and_varispeed_signals_still_decode() {
    let frames = run(tc(10, 20, 30, 0, false), 25, 50);
    for signal in [
        Signal {
            amplitude: 0.01,
            ..signal(48_000, 25.0)
        },
        Signal {
            invert: true,
            ..signal(44_100, 25.0)
        },
        Signal {
            noise: 0.15,
            ..signal(48_000, 25.0)
        },
        // Tape or deck running 8 % fast.
        signal(48_000, 27.0),
    ] {
        let decoded = decode(&signal, &frames);
        assert!(decoded.len() >= 48, "{}", decoded.len());
        let labels = decoded
            .iter()
            .map(|frame| frame.timecode)
            .collect::<Vec<_>>();
        assert_eq!(labels, frames[frames.len() - labels.len()..]);
        assert_eq!(
            decoded.last().unwrap().frame_rate,
            Some(LtcFrameRate::Fps25)
        );
    }
}

#[test]
fn silence_noise_and_non_ltc_tones_decode_nothing() {
    let mut decoder = LtcDecoder::new(48_000);
    assert!(decoder.push_samples(&vec![0.0; 48_000]).is_empty());
    // A 1 kHz tone has the wrong interval pattern for a sync word.
    let tone = (0..48_000)
        .map(|n| (n as f32 * 2.0 * std::f32::consts::PI * 1_000.0 / 48_000.0).sin() * 0.5)
        .collect::<Vec<_>>();
    assert!(decoder.push_samples(&tone).is_empty());
    let mut state = 7_u32;
    let noise = (0..48_000)
        .map(|_| {
            state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            (state >> 8) as f32 / (1 << 24) as f32 - 0.5
        })
        .collect::<Vec<_>>();
    assert!(decoder.push_samples(&noise).is_empty());
    assert!(decoder
        .push_samples(&[f32::NAN, f32::INFINITY, 0.0])
        .is_empty());
    assert_eq!(decoder.sample_index(), 144_003);
}

fn decode_audio(sample_rate: u32, audio: &[f32]) -> Vec<LtcFrame> {
    let mut decoder = LtcDecoder::new(sample_rate);
    audio
        .chunks(480)
        .flat_map(|block| decoder.push_samples(block))
        .collect()
}

#[test]
fn a_tone_or_slow_timecode_before_normal_ltc_does_not_lock_the_decoder() {
    let frames = run(tc(2, 0, 0, 0, false), 30, 60);
    let ltc = synthesize(&signal(48_000, 30.0), &frames);
    // A 700 Hz tone (just under the slowest LTC half cell) first.
    let mut audio = (0..48_000)
        .map(|n| (n as f32 * 2.0 * std::f32::consts::PI * 700.0 / 48_000.0).sin() * 0.5)
        .collect::<Vec<_>>();
    audio.extend_from_slice(&ltc);
    let decoded = decode_audio(48_000, &audio);
    assert!(decoded.len() >= 50, "after a tone: {}", decoded.len());
    // Wind-down: 30 fps labels running at 21 fps, then normal speed.
    let slow = synthesize(&signal(48_000, 21.0), &run(tc(3, 0, 0, 0, false), 30, 30));
    let mut audio = slow;
    audio.extend_from_slice(&ltc);
    let decoded = decode_audio(48_000, &audio);
    let recovered = decoded
        .iter()
        .filter(|frame| frame.timecode.hours == 2)
        .count();
    assert!(recovered >= 50, "after slow timecode: {recovered}");
}

#[test]
fn a_dc_offset_still_decodes() {
    let frames = run(tc(4, 0, 0, 0, false), 25, 50);
    let audio = synthesize(
        &Signal {
            amplitude: 0.05,
            ..signal(48_000, 25.0)
        },
        &frames,
    )
    .into_iter()
    .map(|sample| sample + 0.2)
    .collect::<Vec<_>>();
    let decoded = decode_audio(48_000, &audio);
    assert!(decoded.len() >= 45, "{}", decoded.len());
}

#[test]
fn off_speed_24_and_25_fps_are_told_apart_by_their_labels() {
    // 24 fps running 3 % fast measures like 25 fps; after a second of
    // labels that never reach 24 it is 24 fps.
    let frames = run(tc(5, 0, 0, 0, false), 24, 60);
    let decoded = decode(&signal(48_000, 24.0 * 1.03), &frames);
    assert_eq!(
        decoded.last().unwrap().frame_rate,
        Some(LtcFrameRate::Fps24)
    );
    // 25 fps running 2.5 % slow measures like 24 fps; label 24 settles it.
    let frames = run(tc(5, 0, 0, 0, false), 25, 60);
    let decoded = decode(&signal(48_000, 25.0 * 0.975), &frames);
    assert_eq!(
        decoded.last().unwrap().frame_rate,
        Some(LtcFrameRate::Fps25)
    );
}

#[test]
fn one_corrupt_label_never_raises_the_frame_rate_and_rate_switches_follow() {
    let mut labels = LabelEvidence::default();
    for timecode in run(tc(7, 0, 0, 0, false), 25, 30) {
        labels.observe(timecode);
    }
    // A corrupt frame labelled 27 in the middle of a 25 fps second.
    labels.observe(tc(7, 0, 1, 27, false));
    for timecode in run(tc(7, 0, 1, 6, false), 25, 10) {
        labels.observe(timecode);
    }
    assert_eq!(classify_rate(25.0, false, &labels), LtcFrameRate::Fps25);
    // A switch from 30 to 25 fps is followed after one whole second.
    let mut labels = LabelEvidence::default();
    for timecode in run(tc(8, 0, 0, 0, false), 30, 40) {
        labels.observe(timecode);
    }
    assert_eq!(classify_rate(30.0, false, &labels), LtcFrameRate::Fps30);
    for timecode in run(tc(8, 0, 2, 0, false), 25, 30) {
        labels.observe(timecode);
    }
    assert_eq!(classify_rate(25.0, false, &labels), LtcFrameRate::Fps25);
}

#[test]
fn a_missed_frame_keeps_the_frame_rate() {
    let frames = run(tc(6, 0, 0, 0, false), 25, 30);
    let signal = signal(48_000, 25.0);
    let mut audio = synthesize(&signal, &frames[..15]);
    // One frame of silence (a dropout), then the stream continues.
    let samples_per_frame = (48_000.0 / 25.0) as usize;
    audio.extend(std::iter::repeat_n(0.0, samples_per_frame));
    let mut rest = frames[16..].to_vec();
    rest.push(LtcTimecode {
        frames: 0,
        ..*rest.last().unwrap()
    });
    audio.extend(synthesize(&signal, &rest));
    let decoded = decode_audio(48_000, &audio);
    let after_gap = decoded
        .iter()
        .find(|frame| frame.timecode.frames >= 18)
        .expect("frames after the dropout decode");
    assert_eq!(after_gap.frame_rate, Some(LtcFrameRate::Fps25));
}
