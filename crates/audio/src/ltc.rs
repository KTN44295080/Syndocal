//! SMPTE linear timecode (LTC) decoding from audio samples.
//!
//! LTC carries one 80-bit frame per video frame, biphase-mark coded: every
//! bit cell starts with a level change, and a `1` changes level again in
//! the middle of the cell. The decoder finds level changes with a
//! hysteresis that follows the signal's peak (so any input gain and either
//! polarity work), classifies the intervals between them as whole or half
//! bit cells against an adaptive bit period, and decodes a frame whenever
//! the last 16 bits form the sync word. A DC blocker removes input offsets;
//! the bit cell is measured afresh at the start and whenever two frames'
//! worth of cells pass without a frame. The frame rate follows from the
//! frame length and the labels; 29.97 fps is told from 30 fps by the
//! drop-frame flag.
//!
//! Only forward playback is decoded; reversed timecode is ignored.

/// Bits 64..=79 of a frame (`0011 1111 1111 1101` in transmission order),
/// stored least-significant bit first.
const SYNC_WORD: u16 = 0xbffc;
const FRAME_BITS: u32 = 80;
/// Fastest and slowest LTC bit rates accepted (30 fps +20 %, 24 fps -20 %).
const MAX_BITS_PER_SECOND: f64 = 2_400.0 * 1.2;
const MIN_BITS_PER_SECOND: f64 = 1_920.0 * 0.8;
/// Level changes must cross this fraction of the recent peak.
const HYSTERESIS: f32 = 0.2;
/// Quieter signals are treated as silence.
const MIN_PEAK: f32 = 0.002;
/// How fast the peak tracker forgets, per sample.
const PEAK_DECAY: f32 = 0.9995;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LtcFrameRate {
    Fps24,
    Fps25,
    Fps2997Drop,
    Fps30,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtcTimecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub drop_frame: bool,
}

/// A decoded frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LtcFrame {
    pub timecode: LtcTimecode,
    /// None until a few frames have been timed.
    pub frame_rate: Option<LtcFrameRate>,
    /// The sample index (counted from the decoder's start) at which the
    /// frame's last bit ended: the time `timecode` plus one frame.
    pub end_sample: u64,
}

/// The 80 bits of `timecode`'s frame, bit 0 first (least significant).
/// User bits, the colour flag and the binary group flags are zero; the
/// polarity bit makes the frame contain an even number of zeros, so every
/// frame starts on the same level.
pub fn ltc_frame_bits(timecode: LtcTimecode) -> u128 {
    let mut bits = 0_u128;
    let mut put = |value: u8, at: u32, width: u32| {
        bits |= (u128::from(value) & ((1 << width) - 1)) << at;
    };
    put(timecode.frames % 10, 0, 4);
    put(timecode.frames / 10, 8, 2);
    put(u8::from(timecode.drop_frame), 10, 1);
    put(timecode.seconds % 10, 16, 4);
    put(timecode.seconds / 10, 24, 3);
    put(timecode.minutes % 10, 32, 4);
    put(timecode.minutes / 10, 40, 3);
    put(timecode.hours % 10, 48, 4);
    put(timecode.hours / 10, 56, 2);
    bits |= u128::from(SYNC_WORD) << 64;
    // Polarity correction (bit 27): an even number of zeros per frame.
    if (FRAME_BITS - bits.count_ones()) % 2 == 1 {
        bits |= 1 << 27;
    }
    bits
}

fn field(bits: u128, at: u32, width: u32) -> u8 {
    ((bits >> at) & ((1 << width) - 1)) as u8
}

fn decode_frame_bits(bits: u128) -> Option<LtcTimecode> {
    let timecode = LtcTimecode {
        frames: field(bits, 8, 2) * 10 + field(bits, 0, 4),
        seconds: field(bits, 24, 3) * 10 + field(bits, 16, 4),
        minutes: field(bits, 40, 3) * 10 + field(bits, 32, 4),
        hours: field(bits, 56, 2) * 10 + field(bits, 48, 4),
        drop_frame: field(bits, 10, 1) == 1,
    };
    let units_valid = field(bits, 0, 4) <= 9
        && field(bits, 16, 4) <= 9
        && field(bits, 32, 4) <= 9
        && field(bits, 48, 4) <= 9;
    (units_valid
        && timecode.frames < 30
        && timecode.seconds < 60
        && timecode.minutes < 60
        && timecode.hours < 24)
        .then_some(timecode)
}

/// What the labels say about the frame rate. Only labels that follow
/// their predecessor count, so one corrupt frame cannot raise the rate, and
/// the evidence is renewed every second, so a rate switch is followed.
#[derive(Debug, Default, Clone, Copy)]
struct LabelEvidence {
    previous: Option<LtcTimecode>,
    previous_sequential: bool,
    /// The last label of the most recent whole second seen in sequence.
    last_second_end: Option<u8>,
    /// The highest label reached in sequence in the current second.
    current_max: u8,
}

impl LabelEvidence {
    fn observe(&mut self, timecode: LtcTimecode) {
        let same_second = |a: &LtcTimecode| {
            (a.hours, a.minutes, a.seconds) == (timecode.hours, timecode.minutes, timecode.seconds)
        };
        let sequential = self.previous.is_some_and(|previous| {
            same_second(&previous) && timecode.frames == previous.frames + 1
        });
        // A new second starts at frame 0 (frame 2 for drop-frame minutes).
        let wrapped = self
            .previous
            .is_some_and(|previous| !same_second(&previous) && timecode.frames <= 2);
        if wrapped {
            if self.previous_sequential {
                self.last_second_end = self.previous.map(|previous| previous.frames);
            }
            self.current_max = 0;
        } else if sequential {
            self.current_max = self.current_max.max(timecode.frames);
        }
        self.previous_sequential = sequential;
        self.previous = Some(timecode);
    }

    fn ceiling(&self) -> u8 {
        self.last_second_end.unwrap_or(0).max(self.current_max)
    }
}

/// The frame rate, from the measured frame length and the labels. 24 and 25
/// fps are only 4 % apart, so a fast 24 or slow 25 fps source is told apart
/// by its labels: frame 24 exists only at 25 fps and up, and a whole second
/// ending at frame 23 is 24 fps.
fn classify_rate(frames_per_second: f64, drop_frame: bool, labels: &LabelEvidence) -> LtcFrameRate {
    let thirty = if drop_frame {
        LtcFrameRate::Fps2997Drop
    } else {
        LtcFrameRate::Fps30
    };
    let ceiling = labels.ceiling();
    if ceiling >= 25 {
        thirty
    } else if ceiling == 24 && frames_per_second < 27.5 {
        LtcFrameRate::Fps25
    } else if labels.last_second_end == Some(23) && frames_per_second < 27.5 {
        LtcFrameRate::Fps24
    } else if frames_per_second < 24.5 {
        LtcFrameRate::Fps24
    } else if frames_per_second < 27.5 {
        LtcFrameRate::Fps25
    } else {
        thirty
    }
}

/// Intervals collected to estimate the bit cell after (re)starting.
const BOOTSTRAP_INTERVALS: usize = 24;
/// Bits without a valid frame after which the bit cell is re-estimated (two
/// frames' worth): the estimate may have locked onto the wrong cell length.
const RESYNC_AFTER_BITS: u32 = 2 * FRAME_BITS;
/// The DC blocker's pole: removes offsets, keeps the LTC band.
const DC_BLOCKER_POLE: f32 = 0.995;

#[derive(Debug)]
pub struct LtcDecoder {
    sample_rate: f64,
    peak: f32,
    high: bool,
    dc_input: f32,
    dc_output: f32,
    sample_index: u64,
    last_edge: Option<u64>,
    /// Samples per whole bit cell; None while bootstrapping.
    bit_period: Option<f64>,
    bootstrap: Vec<f64>,
    half_pending: bool,
    bits: u128,
    bits_valid: u32,
    bits_since_frame: u32,
    last_frame_end: Option<u64>,
    /// Smoothed samples per frame.
    frame_period: Option<f64>,
    timed_frames: u32,
    labels: LabelEvidence,
}

impl LtcDecoder {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate: f64::from(sample_rate.max(1)),
            peak: 0.0,
            high: false,
            dc_input: 0.0,
            dc_output: 0.0,
            sample_index: 0,
            last_edge: None,
            bit_period: None,
            bootstrap: Vec::with_capacity(BOOTSTRAP_INTERVALS),
            half_pending: false,
            bits: 0,
            bits_valid: 0,
            bits_since_frame: 0,
            last_frame_end: None,
            frame_period: None,
            timed_frames: 0,
            labels: LabelEvidence::default(),
        }
    }

    /// Samples processed so far.
    pub fn sample_index(&self) -> u64 {
        self.sample_index
    }

    fn shortest_whole_cell(&self) -> f64 {
        self.sample_rate / MAX_BITS_PER_SECOND
    }

    fn longest_whole_cell(&self) -> f64 {
        self.sample_rate / MIN_BITS_PER_SECOND
    }

    fn lose_sync(&mut self) {
        self.half_pending = false;
        self.bits_valid = 0;
    }

    /// Forget the bit cell estimate and measure it again.
    fn rebootstrap(&mut self) {
        self.lose_sync();
        self.bit_period = None;
        self.bootstrap.clear();
        self.bits_since_frame = 0;
    }

    /// Decodes mono samples, returning every complete frame in them.
    pub fn push_samples(&mut self, samples: &[f32]) -> Vec<LtcFrame> {
        let mut frames = Vec::new();
        for &sample in samples {
            let sample = if sample.is_finite() { sample } else { 0.0 };
            // DC blocker: an offset must not hide level changes.
            let blocked = sample - self.dc_input + DC_BLOCKER_POLE * self.dc_output;
            self.dc_input = sample;
            self.dc_output = blocked;
            let sample = blocked;
            self.peak = (self.peak * PEAK_DECAY).max(sample.abs());
            let threshold = (self.peak * HYSTERESIS).max(MIN_PEAK);
            let edge = if self.high {
                sample < -threshold
            } else {
                sample > threshold
            };
            if edge && self.peak >= MIN_PEAK {
                self.high = !self.high;
                if let Some(last) = self.last_edge {
                    let interval = (self.sample_index - last) as f64;
                    if let Some(frame) = self.interval(interval) {
                        frames.push(frame);
                    }
                }
                self.last_edge = Some(self.sample_index);
            }
            self.sample_index += 1;
        }
        frames
    }

    /// Estimates the whole cell from the first intervals: whole and half
    /// cells differ by two, so with both present the longer group is the
    /// whole cell. A window of one kind is a run of ones (half cells, short)
    /// or of zeros (whole cells, too long to be halves).
    fn bootstrap_period(&mut self, interval: f64) -> Option<f64> {
        self.bootstrap.push(interval);
        if self.bootstrap.len() < BOOTSTRAP_INTERVALS {
            return None;
        }
        let samples = std::mem::take(&mut self.bootstrap);
        let longest = samples.iter().copied().fold(0.0, f64::max);
        let shortest = samples.iter().copied().fold(f64::MAX, f64::min);
        let whole = if longest >= shortest * 1.5 {
            let split = (longest + shortest) / 2.0;
            let long = samples
                .iter()
                .copied()
                .filter(|interval| *interval >= split)
                .collect::<Vec<_>>();
            long.iter().sum::<f64>() / long.len() as f64
        } else if longest * 2.0 <= self.longest_whole_cell() * 1.1 {
            longest * 2.0
        } else {
            longest
        };
        (self.shortest_whole_cell() * 0.9..=self.longest_whole_cell() * 1.1)
            .contains(&whole)
            .then_some(whole)
    }

    fn interval(&mut self, interval: f64) -> Option<LtcFrame> {
        let shortest = self.shortest_whole_cell() / 2.0;
        let longest = self.longest_whole_cell();
        if !(shortest * 0.8..=longest * 1.25).contains(&interval) {
            self.lose_sync();
            return None;
        }
        let period = match self.bit_period {
            Some(period) => period,
            None => {
                self.bit_period = self.bootstrap_period(interval);
                return None;
            }
        };
        let (min_period, max_period) = (
            self.shortest_whole_cell() * 0.9,
            self.longest_whole_cell() * 1.1,
        );
        let clamp = |period: f64| period.clamp(min_period, max_period);
        let frame = if interval >= period * 0.75 {
            // A whole cell: a 0. A pending half cell means we were out of
            // step; this realigns.
            if self.half_pending {
                self.lose_sync();
            }
            self.bit_period = Some(clamp(period * 0.9 + interval * 0.1));
            self.push_bit(false)
        } else if self.half_pending {
            self.half_pending = false;
            self.push_bit(true)
        } else {
            self.half_pending = true;
            self.bit_period = Some(clamp(period * 0.95 + interval * 2.0 * 0.05));
            None
        };
        if frame.is_some() {
            self.bits_since_frame = 0;
        } else if self.bits_since_frame > RESYNC_AFTER_BITS {
            // Two frames' worth of cells and no frame: the cell estimate is
            // wrong (a tone, a wind-down); measure it again.
            self.rebootstrap();
        }
        frame
    }

    fn push_bit(&mut self, bit: bool) -> Option<LtcFrame> {
        self.bits_since_frame = self.bits_since_frame.saturating_add(1);
        self.bits = (self.bits >> 1) | (u128::from(bit) << (FRAME_BITS - 1));
        self.bits_valid = (self.bits_valid + 1).min(FRAME_BITS);
        if self.bits_valid < FRAME_BITS || (self.bits >> 64) as u16 != SYNC_WORD {
            return None;
        }
        let timecode = decode_frame_bits(self.bits)?;
        let end_sample = self.sample_index;
        if let Some(last) = self.last_frame_end {
            let length = (end_sample - last) as f64;
            let expected = self.frame_period.unwrap_or(length);
            // A missed frame makes one gap two or three frames long; keep
            // the timing instead of starting over.
            let frames_spanned = (length / expected).round().clamp(1.0, 3.0);
            let per_frame = length / frames_spanned;
            if (per_frame - expected).abs() <= expected * 0.25 {
                self.frame_period = Some(expected * 0.8 + per_frame * 0.2);
                self.timed_frames = self.timed_frames.saturating_add(1);
            } else {
                self.frame_period = Some(length);
                self.timed_frames = 0;
                self.labels = LabelEvidence::default();
            }
        }
        self.last_frame_end = Some(end_sample);
        self.labels.observe(timecode);
        let frame_rate = self
            .frame_period
            .filter(|_| self.timed_frames >= 2)
            .map(|period| {
                classify_rate(self.sample_rate / period, timecode.drop_frame, &self.labels)
            });
        Some(LtcFrame {
            timecode,
            frame_rate,
            end_sample,
        })
    }
}

#[cfg(test)]
#[path = "ltc_tests.rs"]
mod tests;
