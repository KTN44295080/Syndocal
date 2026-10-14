//! MIDI Time Code output: quarter frames following a transport.
//!
//! `MtcTransmitter` is a pure state machine fed with transport observations
//! and the current time; it decides which messages are due. While the
//! transport plays it sends four quarter frames per frame, each eight-piece
//! sequence carrying the frame at which its first piece was sent (receivers
//! add two frames when the sequence completes). Starting, stopping and any
//! jump send a full-frame SysEx so receivers locate at once. Between
//! transport observations, which arrive only at engine ticks, it runs its
//! own clock and slews it toward the observed position.

use std::time::{Duration, Instant};

use midir::{MidiOutput, MidiOutputConnection};

use crate::midi::{MidiError, MtcFrameRate, MtcTimecode};

/// How far an observed position may drift from the running clock before
/// the clock is re-anchored with a full frame, in frames.
const RELOCATE_AFTER_FRAMES: f64 = 2.0;
/// The fraction of a small drift corrected per observation.
const SLEW: f64 = 0.1;
/// Only the first read of each engine publication is used; with reads every
/// 4 ms it is on average this old.
const OBSERVATION_AGE_MS: f64 = 2.0;
/// A playing transport whose position stops changing for this long (a
/// stalled external source, an exhausted transport) holds the output
/// instead of freewheeling ahead of it.
const FROZEN_AFTER: Duration = Duration::from_millis(100);
/// Quarter frames sent at most per call; a longer stall resynchronizes.
const MAX_QUARTER_FRAMES_PER_POLL: usize = 8;

/// One transport observation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtcTransportObservation {
    pub playing: bool,
    pub position_ms: u64,
    /// Changes when the transport is restarted or relocated.
    pub generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtcMessage {
    QuarterFrame([u8; 2]),
    FullFrame([u8; 10]),
}

impl MtcMessage {
    pub fn bytes(&self) -> &[u8] {
        match self {
            Self::QuarterFrame(bytes) => bytes,
            Self::FullFrame(bytes) => bytes,
        }
    }
}

/// The quarter-frame message carrying `piece` (0..=7) of `timecode`.
pub fn quarter_frame_message(timecode: MtcTimecode, piece: u8) -> [u8; 2] {
    let nibble = match piece & 0x07 {
        0 => timecode.frames & 0x0f,
        1 => (timecode.frames >> 4) & 0x01,
        2 => timecode.seconds & 0x0f,
        3 => (timecode.seconds >> 4) & 0x03,
        4 => timecode.minutes & 0x0f,
        5 => (timecode.minutes >> 4) & 0x03,
        6 => timecode.hours & 0x0f,
        _ => ((timecode.hours >> 4) & 0x01) | (timecode.frame_rate.rate_bits() << 1),
    };
    [0xf1, ((piece & 0x07) << 4) | nibble]
}

/// The full-frame SysEx locating every receiver at `timecode`.
pub fn full_frame_message(timecode: MtcTimecode) -> [u8; 10] {
    [
        0xf0,
        0x7f,
        0x7f,
        0x01,
        0x01,
        (timecode.frame_rate.rate_bits() << 5) | (timecode.hours & 0x1f),
        timecode.minutes,
        timecode.seconds,
        timecode.frames,
        0xf7,
    ]
}

#[derive(Debug, Clone, Copy)]
struct Clock {
    anchor_ms: f64,
    anchor_at: Instant,
}

impl Clock {
    fn position_ms(&self, now: Instant) -> f64 {
        self.anchor_ms + now.saturating_duration_since(self.anchor_at).as_secs_f64() * 1_000.0
    }
}

#[derive(Debug, Clone, Copy)]
struct Cycle {
    /// The frame carried by this eight-piece sequence.
    frame: u64,
    piece: u8,
}

#[derive(Debug)]
pub struct MtcTransmitter {
    frame_rate: MtcFrameRate,
    clock: Option<Clock>,
    cycle: Option<Cycle>,
    last: Option<MtcTransportObservation>,
    /// When the observed position last changed (or playback started).
    last_change_at: Option<Instant>,
}

impl MtcTransmitter {
    pub fn new(frame_rate: MtcFrameRate) -> Self {
        Self {
            frame_rate,
            clock: None,
            cycle: None,
            last: None,
            last_change_at: None,
        }
    }

    pub fn frame_rate(&self) -> MtcFrameRate {
        self.frame_rate
    }

    pub fn is_running(&self) -> bool {
        self.clock.is_some()
    }

    fn frame_ms(&self) -> f64 {
        self.frame_rate.frame_duration_ms()
    }

    fn anchor(&mut self, position_ms: f64, now: Instant) -> MtcMessage {
        let position_ms = position_ms.max(0.0);
        self.clock = Some(Clock {
            anchor_ms: position_ms,
            anchor_at: now,
        });
        let frame = self.frame_rate.frame_at_ms(position_ms as u64);
        // Quarter frames begin with piece 0 at the next frame boundary, so
        // a mid-frame start sends no burst.
        let next_frame = if (position_ms - frame as f64 * self.frame_ms()) > 0.001 {
            frame + 1
        } else {
            frame
        };
        self.cycle = Some(Cycle {
            frame: next_frame,
            piece: 0,
        });
        MtcMessage::FullFrame(full_frame_message(MtcTimecode::from_frame_count(
            frame,
            self.frame_rate,
        )))
    }

    /// Feeds the latest transport state. Returns the full frame to send
    /// when the transport started, stopped, froze or jumped.
    pub fn observe(
        &mut self,
        observation: MtcTransportObservation,
        now: Instant,
    ) -> Option<MtcMessage> {
        let previous = self.last.replace(observation);
        let moved = previous.is_none_or(|previous| previous.position_ms != observation.position_ms);
        let started = previous.is_none_or(|previous| !previous.playing) && observation.playing;
        if moved || started {
            self.last_change_at = Some(now);
        }
        let frozen = observation.playing
            && self
                .last_change_at
                .is_some_and(|at| now.saturating_duration_since(at) >= FROZEN_AFTER);
        let full_frame = || {
            MtcMessage::FullFrame(full_frame_message(MtcTimecode::from_position_ms(
                observation.position_ms,
                self.frame_rate,
            )))
        };
        if !observation.playing || frozen {
            // Stopped (or the playhead no longer moves): no quarter frames;
            // one full frame whenever the held position changes.
            let was_running = self.clock.take().is_some();
            self.cycle = None;
            let changed = was_running
                || previous.is_none_or(|previous| {
                    previous.position_ms != observation.position_ms
                        || previous.generation != observation.generation
                });
            return changed.then(full_frame);
        }
        let observed_ms = observation.position_ms as f64 + OBSERVATION_AGE_MS;
        let Some(clock) = self.clock.as_mut() else {
            return Some(self.anchor(observed_ms, now));
        };
        if !moved {
            // The same engine publication read again: nothing new to correct.
            return None;
        }
        // A new transport generation alone (tempo, tap, sync) is no jump;
        // only real drift relocates.
        let drift = observed_ms - clock.position_ms(now);
        if drift.abs() > RELOCATE_AFTER_FRAMES * self.frame_rate.frame_duration_ms() {
            return Some(self.anchor(observed_ms, now));
        }
        clock.anchor_ms += drift * SLEW;
        None
    }

    fn due_position_ms(&self, cycle: Cycle) -> f64 {
        (cycle.frame as f64 + f64::from(cycle.piece) / 4.0) * self.frame_ms()
    }

    /// The quarter frames due by `now`.
    pub fn due(&mut self, now: Instant) -> Vec<MtcMessage> {
        let (Some(clock), Some(mut cycle)) = (self.clock, self.cycle) else {
            return Vec::new();
        };
        let position = clock.position_ms(now);
        // Fell more than a frame behind (a stall): restart the sequence at
        // the current frame instead of bursting stale pieces.
        if position - self.due_position_ms(cycle) > self.frame_ms() {
            cycle = Cycle {
                frame: self.frame_rate.frame_at_ms(position.max(0.0) as u64),
                piece: 0,
            };
        }
        let mut messages = Vec::new();
        while messages.len() < MAX_QUARTER_FRAMES_PER_POLL
            && position >= self.due_position_ms(cycle)
        {
            let timecode = MtcTimecode::from_frame_count(cycle.frame, self.frame_rate);
            messages.push(MtcMessage::QuarterFrame(quarter_frame_message(
                timecode,
                cycle.piece,
            )));
            cycle.piece += 1;
            if cycle.piece == 8 {
                cycle = Cycle {
                    frame: cycle.frame + 2,
                    piece: 0,
                };
            }
        }
        self.cycle = Some(cycle);
        messages
    }

    /// When the next quarter frame is due; None while stopped.
    pub fn next_due(&self) -> Option<Instant> {
        let (clock, cycle) = (self.clock?, self.cycle?);
        let ahead_ms = self.due_position_ms(cycle) - clock.anchor_ms;
        Some(if ahead_ms <= 0.0 {
            clock.anchor_at
        } else {
            clock.anchor_at + Duration::from_secs_f64(ahead_ms / 1_000.0)
        })
    }

    /// The timecode the transmitter is at now.
    pub fn timecode(&self, now: Instant) -> Option<MtcTimecode> {
        match self.clock {
            Some(clock) => Some(MtcTimecode::from_position_ms(
                clock.position_ms(now).max(0.0) as u64,
                self.frame_rate,
            )),
            None => self
                .last
                .map(|last| MtcTimecode::from_position_ms(last.position_ms, self.frame_rate)),
        }
    }
}

/// An open MIDI output used only for timecode.
pub struct MtcOutput {
    connection: MidiOutputConnection,
    port_name: String,
}

impl MtcOutput {
    pub fn port_name(&self) -> &str {
        &self.port_name
    }

    pub fn send(&mut self, message: &MtcMessage) -> Result<(), MidiError> {
        self.connection
            .send(message.bytes())
            .map_err(|error| MidiError::Send(error.to_string()))
    }
}

/// Opens output `port_index`, refusing it when its name is no longer
/// `expected_name` (the device list changed since it was chosen).
pub fn connect_mtc_output_named(
    port_index: usize,
    expected_name: &str,
) -> Result<MtcOutput, MidiError> {
    let output = MidiOutput::new("syndocal-mtc")?;
    let ports = output.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let port_name = output.port_name(port).unwrap_or_default();
    if port_name != expected_name {
        return Err(MidiError::Connect(
            "MIDI output device changed; scan and select it again".into(),
        ));
    }
    let connection = output
        .connect(port, "syndocal-mtc-output")
        .map_err(|error| MidiError::Connect(error.to_string()))?;
    Ok(MtcOutput {
        connection,
        port_name,
    })
}

#[cfg(test)]
#[path = "mtc_output_tests.rs"]
mod tests;
