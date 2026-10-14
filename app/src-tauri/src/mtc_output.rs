//! MIDI Time Code output following the Timeline playhead.
//!
//! A worker sends quarter frames while the Timeline plays and a full frame
//! when it starts, stops or jumps (`io::mtc_output::MtcTransmitter`). It
//! sends only while this PC owns lighting or video output: MTC drives
//! external show equipment, so a Standby or unarmed PC stays silent, and a
//! newly armed PC begins with a full frame.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use engine::EngineHandle;
use io::midi::{MtcFrameRate, MtcTimecode};
use io::mtc_output::{MtcMessage, MtcOutput, MtcTransmitter, MtcTransportObservation};
use serde::{Deserialize, Serialize};

/// How often the Timeline transport is read.
const TRANSPORT_POLL: Duration = Duration::from_millis(4);
/// How often output ownership and the visible status are refreshed.
const OWNERSHIP_POLL: Duration = Duration::from_millis(100);
/// Consecutive send failures after which the output is closed (about five
/// seconds of quarter frames), so a brief USB glitch survives.
const MAX_CONSECUTIVE_SEND_FAILURES: u32 = 600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MtcOutputFrameRate {
    Fps24,
    Fps25,
    Fps2997Drop,
    Fps30,
}

impl From<MtcOutputFrameRate> for MtcFrameRate {
    fn from(rate: MtcOutputFrameRate) -> Self {
        match rate {
            MtcOutputFrameRate::Fps24 => Self::Fps24,
            MtcOutputFrameRate::Fps25 => Self::Fps25,
            MtcOutputFrameRate::Fps2997Drop => Self::Fps2997Drop,
            MtcOutputFrameRate::Fps30 => Self::Fps30,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct MtcOutputStatus {
    pub(crate) running: bool,
    pub(crate) port_name: Option<String>,
    pub(crate) frame_rate: Option<MtcOutputFrameRate>,
    /// Quarter frames are going out now (the Timeline plays and this PC
    /// owns output).
    pub(crate) transmitting: bool,
    /// Why nothing is sent although the output is open.
    pub(crate) held_reason: Option<String>,
    pub(crate) timecode: Option<String>,
    pub(crate) quarter_frames_sent: u64,
    pub(crate) full_frames_sent: u64,
    pub(crate) last_error: Option<String>,
}

/// Whether MTC may drive external equipment from this PC now.
fn output_owned(status: &protocol::OutputOwnershipStatus) -> Result<(), String> {
    if status.state == protocol::OutputOwnershipState::Ready
        && (status.lighting_allowed || status.video_allowed)
    {
        Ok(())
    } else {
        Err("This PC does not own output; MTC waits until output is armed".to_string())
    }
}

pub(crate) struct MtcOutputRuntime {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    status: Arc<Mutex<MtcOutputStatus>>,
}

impl MtcOutputRuntime {
    pub(crate) fn start(
        engine: EngineHandle,
        output: MtcOutput,
        frame_rate: MtcOutputFrameRate,
    ) -> Result<Self, String> {
        let status = Arc::new(Mutex::new(MtcOutputStatus {
            running: true,
            port_name: Some(output.port_name().to_string()),
            frame_rate: Some(frame_rate),
            ..MtcOutputStatus::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker_status = Arc::clone(&status);
        let worker = std::thread::Builder::new()
            .name("syndocal-mtc-output".to_string())
            .spawn(move || {
                run(
                    engine,
                    output,
                    frame_rate.into(),
                    &worker_stop,
                    &worker_status,
                )
            })
            .map_err(|error| format!("Cannot start the MTC output worker: {error}"))?;
        Ok(Self {
            stop,
            worker: Some(worker),
            status,
        })
    }

    pub(crate) fn status(&self) -> MtcOutputStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }
}

impl MtcOutputRuntime {
    pub(crate) fn port_name(&self) -> Option<String> {
        self.status().port_name
    }
}

impl Drop for MtcOutputRuntime {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            // The worker parks between messages; wake it so it sees Stop.
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

fn run(
    engine: EngineHandle,
    mut output: MtcOutput,
    frame_rate: MtcFrameRate,
    stop: &AtomicBool,
    status: &Mutex<MtcOutputStatus>,
) {
    let _realtime = engine::configure_realtime_thread();
    let mut transmitter = MtcTransmitter::new(frame_rate);
    let mut owned: Result<(), String> = Err(String::new());
    let mut next_transport = Instant::now();
    let mut next_ownership = Instant::now();
    let mut quarter_frames = 0_u64;
    let mut full_frames = 0_u64;
    let mut failures = 0_u32;
    let mut last_error: Option<String> = None;
    let mut send = |message: &MtcMessage,
                    quarter_frames: &mut u64,
                    full_frames: &mut u64,
                    last_error: &mut Option<String>|
     -> bool {
        match output.send(message) {
            Ok(()) => {
                failures = 0;
                match message {
                    MtcMessage::QuarterFrame(_) => *quarter_frames += 1,
                    MtcMessage::FullFrame(_) => *full_frames += 1,
                }
                true
            }
            Err(error) => {
                failures += 1;
                *last_error = Some(error.to_string());
                failures < MAX_CONSECUTIVE_SEND_FAILURES
            }
        }
    };
    while !stop.load(Ordering::Acquire) {
        let now = Instant::now();
        if now >= next_ownership {
            let current = output_owned(&engine.output_ownership_status());
            if current.is_err() {
                // Not owned: forget the transport, so arming starts with a
                // full frame.
                transmitter = MtcTransmitter::new(frame_rate);
            }
            owned = current;
            next_ownership = now + OWNERSHIP_POLL;
            if let Ok(mut current) = status.lock() {
                current.transmitting = owned.is_ok() && transmitter.is_running();
                current.held_reason = owned.as_ref().err().cloned();
                current.timecode = transmitter.timecode(now).map(MtcTimecode::label);
                current.quarter_frames_sent = quarter_frames;
                current.full_frames_sent = full_frames;
                current.last_error = last_error.clone();
            }
        }
        let mut alive = true;
        if owned.is_ok() {
            if now >= next_transport {
                let observation = engine.inspect_snapshot(|snapshot| MtcTransportObservation {
                    // A count-in plays without moving the playhead; MTC
                    // starts with the Timeline itself.
                    playing: snapshot.timeline.playing
                        && snapshot.timeline.count_in_remaining_ms == 0,
                    position_ms: snapshot.timeline.position_ms,
                    generation: snapshot.timeline.transport_generation,
                });
                match observation {
                    Ok(observation) => {
                        if let Some(message) = transmitter.observe(observation, now) {
                            alive &= send(
                                &message,
                                &mut quarter_frames,
                                &mut full_frames,
                                &mut last_error,
                            );
                        }
                    }
                    // No transport to follow: never freewheel.
                    Err(_) => transmitter = MtcTransmitter::new(frame_rate),
                }
                next_transport = now + TRANSPORT_POLL;
            }
            for message in transmitter.due(Instant::now()) {
                alive &= send(
                    &message,
                    &mut quarter_frames,
                    &mut full_frames,
                    &mut last_error,
                );
            }
        }
        if !alive {
            break;
        }
        let mut wake = next_ownership;
        if owned.is_ok() {
            wake = wake.min(next_transport);
            if let Some(due) = transmitter.next_due() {
                wake = wake.min(due);
            }
        }
        let now = Instant::now();
        if wake > now {
            std::thread::park_timeout(wake - now);
        }
    }
    if let Ok(mut current) = status.lock() {
        current.running = false;
        current.transmitting = false;
        current.quarter_frames_sent = quarter_frames;
        current.full_frames_sent = full_frames;
        current.last_error = last_error;
    }
}
