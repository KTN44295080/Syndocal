//! SMPTE LTC input: the Timeline chases timecode arriving on one channel of
//! an audio input.
//!
//! A shared-mode capture of its own (independent of Live Audio Input) feeds
//! one channel through `audio::ltc::LtcDecoder`. Every decoded frame becomes
//! a `SyncTimelineTimecode { source: Ltc }` engine command, exactly like the
//! position-injection route: the frame's end time, plus how long ago in the
//! captured block it ended. Commands go through the same project fence as
//! MIDI timecode (held during a project transaction). A frame that does not
//! follow the previous one is taken only when the next frame confirms it,
//! so one corrupt frame never jumps the Timeline.

use std::{
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use audio::ltc::{LtcDecoder, LtcFrame, LtcFrameRate};
use crossbeam_queue::ArrayQueue;
use engine::EngineCommand;
use io::midi::{MtcFrameRate, MtcTimecode};
use serde::Serialize;

/// Samples per block handed from the audio callback to the decoder.
const BLOCK_SAMPLES: usize = 1_024;
const BLOCK_SLOTS: usize = 16;
/// No decoded frame for this long means no LTC signal.
const SIGNAL_TIMEOUT: Duration = Duration::from_millis(500);
const STATUS_INTERVAL: Duration = Duration::from_millis(100);

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct LtcInputDeviceSummary {
    pub(crate) name: String,
    pub(crate) channels: u16,
    pub(crate) sample_rate: u32,
    pub(crate) is_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub(crate) struct LtcInputStatus {
    pub(crate) running: bool,
    pub(crate) device_name: Option<String>,
    pub(crate) channel_index: u16,
    pub(crate) sample_rate: u32,
    /// Frames are being decoded now.
    pub(crate) signal: bool,
    pub(crate) timecode: Option<String>,
    pub(crate) frame_rate: Option<String>,
    pub(crate) frames_decoded: u64,
    /// Samples lost because the decoder fell behind.
    pub(crate) dropped_samples: u64,
    pub(crate) last_error: Option<String>,
}

fn rate_label(rate: LtcFrameRate) -> &'static str {
    match rate {
        LtcFrameRate::Fps24 => "24",
        LtcFrameRate::Fps25 => "25",
        LtcFrameRate::Fps2997Drop => "29.97 drop-frame",
        LtcFrameRate::Fps30 => "30",
    }
}

fn mtc_rate(rate: LtcFrameRate) -> MtcFrameRate {
    match rate {
        LtcFrameRate::Fps24 => MtcFrameRate::Fps24,
        LtcFrameRate::Fps25 => MtcFrameRate::Fps25,
        LtcFrameRate::Fps2997Drop => MtcFrameRate::Fps2997Drop,
        LtcFrameRate::Fps30 => MtcFrameRate::Fps30,
    }
}

/// The Timeline position `frame` stands for once the decoder has read
/// `samples_read` samples: the frame ends where the next one starts, and
/// it ended `samples_read - end_sample` samples ago. None for a label the
/// frame rate cannot have.
pub(crate) fn chase_position_ms(
    frame: &LtcFrame,
    rate: LtcFrameRate,
    samples_read: u64,
    sample_rate: u32,
) -> Option<(u64, MtcTimecode)> {
    let rate = mtc_rate(rate);
    let label = MtcTimecode {
        hours: frame.timecode.hours,
        minutes: frame.timecode.minutes,
        seconds: frame.timecode.seconds,
        frames: frame.timecode.frames,
        frame_rate: rate,
    };
    if u64::from(label.frames) >= rate.nominal_fps() {
        return None;
    }
    // Drop-frame labels skip frames 0 and 1 of every minute but each tenth.
    if rate == MtcFrameRate::Fps2997Drop
        && label.seconds == 0
        && label.frames < 2
        && label.minutes % 10 != 0
    {
        return None;
    }
    let frame_end_ms = rate.frame_start_ms(label.frame_count() + 1);
    let age_ms =
        samples_read.saturating_sub(frame.end_sample) * 1_000 / u64::from(sample_rate.max(1));
    Some((frame_end_ms + age_ms, label))
}

/// Sends one engine command through the project fence; false when it was
/// held back (for example during a project transaction).
pub(crate) type TimecodeSender = Box<dyn Fn(EngineCommand) -> bool + Send>;

/// Frames a decoded frame may skip and still follow the previous one
/// (missed or corrupt frames in between).
const CONTINUITY_SLACK_FRAMES: u64 = 3;

/// Accepts a frame that follows the previous one (allowing a few missed
/// frames); any other frame, including the first after a dropout, is taken
/// only when the next frame follows it.
#[derive(Debug, Default)]
pub(crate) struct ContinuityGate {
    accepted: Option<u64>,
    candidate: Option<u64>,
}

impl ContinuityGate {
    pub(crate) fn accept(&mut self, frame_count: u64) -> bool {
        let follows = |previous: Option<u64>| {
            previous.is_some_and(|previous| {
                (previous..=previous + CONTINUITY_SLACK_FRAMES).contains(&frame_count)
            })
        };
        if follows(self.accepted) || follows(self.candidate) {
            self.accepted = Some(frame_count);
            self.candidate = None;
            true
        } else {
            self.candidate = Some(frame_count);
            false
        }
    }

    /// Starts over (the signal was lost).
    pub(crate) fn reset(&mut self) {
        *self = Self::default();
    }
}

/// The input devices of the shared-mode host.
pub(crate) fn list_devices() -> Result<Vec<LtcInputDeviceSummary>, String> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait};
    let host = rodio::cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .map_err(|error| format!("Failed to list audio input devices: {error}"))?;
    Ok(devices
        .filter_map(|device| {
            let name = device.name().ok()?;
            let config = device.default_input_config().ok()?;
            Some(LtcInputDeviceSummary {
                is_default: default_name.as_deref() == Some(name.as_str()),
                name,
                channels: config.channels(),
                sample_rate: config.sample_rate().0,
            })
        })
        .collect())
}

/// Audio blocks between the capture callback and the decoder.
struct Blocks {
    free: ArrayQueue<Vec<f32>>,
    ready: ArrayQueue<Vec<f32>>,
    dropped: AtomicU64,
}

impl Blocks {
    fn new() -> Arc<Self> {
        let blocks = Arc::new(Self {
            free: ArrayQueue::new(BLOCK_SLOTS),
            ready: ArrayQueue::new(BLOCK_SLOTS),
            dropped: AtomicU64::new(0),
        });
        for _ in 0..BLOCK_SLOTS {
            let _ = blocks.free.push(Vec::with_capacity(BLOCK_SAMPLES));
        }
        blocks
    }
}

#[cfg(not(target_os = "macos"))]
type CaptureStream = rodio::cpal::Stream;
#[cfg(target_os = "macos")]
type CaptureStream = super::mac_audio_resources::MacAudioInputStream;

fn build_stream<T>(
    device: &rodio::cpal::Device,
    config: &rodio::cpal::StreamConfig,
    channel_index: usize,
    blocks: Arc<Blocks>,
    wake: std::thread::Thread,
    errors: Arc<Mutex<Option<(String, Instant)>>>,
) -> Result<rodio::cpal::Stream, rodio::cpal::BuildStreamError>
where
    T: rodio::cpal::SizedSample + Copy + Send + 'static,
    f32: rodio::cpal::FromSample<T>,
{
    use rodio::cpal::traits::DeviceTrait;
    let channels = usize::from(config.channels.max(1));
    // The block being filled; taken from `free`, handed to `ready` when full.
    let mut current: Option<Vec<f32>> = None;
    device.build_input_stream(
        config,
        move |data: &[T], _| {
            // Never unwind into the audio driver.
            let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                for frame in data.chunks(channels) {
                    let Some(&sample) = frame.get(channel_index) else {
                        continue;
                    };
                    let block = match current.as_mut() {
                        Some(block) => block,
                        None => match blocks.free.pop() {
                            Some(block) => current.insert(block),
                            None => {
                                // The decoder is behind: drop this sample.
                                blocks.dropped.fetch_add(1, Ordering::Relaxed);
                                continue;
                            }
                        },
                    };
                    block.push(<f32 as rodio::cpal::Sample>::from_sample(sample));
                    if block.len() >= BLOCK_SAMPLES {
                        if let Some(full) = current.take() {
                            if let Err(full) = blocks.ready.push(full) {
                                let mut full = full;
                                full.clear();
                                let _ = blocks.free.push(full);
                            }
                        }
                        wake.unpark();
                    }
                }
            }));
        },
        move |error| {
            if let Ok(mut last) = errors.lock() {
                *last = Some((error.to_string(), Instant::now()));
            }
        },
        None,
    )
}

fn open_stream(
    device_name: Option<&str>,
    channel_index: u16,
    blocks: Arc<Blocks>,
    wake: std::thread::Thread,
    errors: Arc<Mutex<Option<(String, Instant)>>>,
) -> Result<(CaptureStream, String, u32), String> {
    use rodio::cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
    let host = rodio::cpal::default_host();
    let device = match device_name {
        Some(name) => host
            .input_devices()
            .map_err(|error| format!("Failed to list audio input devices: {error}"))?
            .find(|device| device.name().ok().as_deref() == Some(name))
            .ok_or_else(|| format!("Audio input \"{name}\" was not found; refresh the list"))?,
        None => host
            .default_input_device()
            .ok_or_else(|| "No default audio input device is available".to_string())?,
    };
    let name = device.name().unwrap_or_else(|_| "Audio input".to_string());
    let supported = device
        .default_input_config()
        .map_err(|error| format!("Failed to read audio input configuration: {error}"))?;
    if channel_index >= supported.channels() {
        return Err(format!(
            "{name} has {} channel(s); channel {} does not exist",
            supported.channels(),
            u32::from(channel_index) + 1
        ));
    }
    let sample_rate = supported.sample_rate().0;
    let sample_format = supported.sample_format();
    let config: rodio::cpal::StreamConfig = supported.into();
    let channel = usize::from(channel_index);
    let build = move || -> Result<rodio::cpal::Stream, String> {
        use rodio::cpal::SampleFormat;
        let stream = match sample_format {
            SampleFormat::I16 => {
                build_stream::<i16>(&device, &config, channel, blocks, wake, errors)
            }
            SampleFormat::I32 => {
                build_stream::<i32>(&device, &config, channel, blocks, wake, errors)
            }
            SampleFormat::U16 => {
                build_stream::<u16>(&device, &config, channel, blocks, wake, errors)
            }
            SampleFormat::F32 => {
                build_stream::<f32>(&device, &config, channel, blocks, wake, errors)
            }
            SampleFormat::F64 => {
                build_stream::<f64>(&device, &config, channel, blocks, wake, errors)
            }
            other => return Err(format!("Unsupported audio input sample format {other:?}")),
        }
        .map_err(|error| format!("Failed to open the LTC input: {error}"))?;
        stream
            .play()
            .map_err(|error| format!("Failed to start the LTC input: {error}"))?;
        Ok(stream)
    };
    #[cfg(not(target_os = "macos"))]
    let stream = build()?;
    #[cfg(target_os = "macos")]
    let stream = super::mac_audio_resources::own_input_stream(build)?;
    Ok((stream, name, sample_rate))
}

pub(crate) struct LtcInputRuntime {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    stream: Option<CaptureStream>,
    status: Arc<Mutex<LtcInputStatus>>,
}

impl LtcInputRuntime {
    pub(crate) fn start(
        sender: TimecodeSender,
        device_name: Option<String>,
        channel_index: u16,
    ) -> Result<Self, String> {
        let blocks = Blocks::new();
        let errors = Arc::new(Mutex::new(None));
        let status = Arc::new(Mutex::new(LtcInputStatus {
            running: true,
            channel_index,
            ..LtcInputStatus::default()
        }));
        let stop = Arc::new(AtomicBool::new(false));
        let sample_rate = Arc::new(AtomicU64::new(0));
        let worker = {
            let (stop, status, blocks, errors, sample_rate) = (
                Arc::clone(&stop),
                Arc::clone(&status),
                Arc::clone(&blocks),
                Arc::clone(&errors),
                Arc::clone(&sample_rate),
            );
            std::thread::Builder::new()
                .name("syndocal-ltc-input".to_string())
                .spawn(move || run(sender, &stop, &status, &blocks, &errors, &sample_rate))
                .map_err(|error| format!("Cannot start the LTC decoder: {error}"))?
        };
        let opened = open_stream(
            device_name.as_deref(),
            channel_index,
            Arc::clone(&blocks),
            worker.thread().clone(),
            errors,
        );
        let (stream, name, rate) = match opened {
            Ok(opened) => opened,
            Err(error) => {
                stop.store(true, Ordering::Release);
                worker.thread().unpark();
                let _ = worker.join();
                return Err(error);
            }
        };
        if let Ok(mut current) = status.lock() {
            current.device_name = Some(name);
            current.sample_rate = rate;
        }
        // The decoder starts only once it knows the sample rate.
        sample_rate.store(u64::from(rate), Ordering::Release);
        worker.thread().unpark();
        Ok(Self {
            stop,
            worker: Some(worker),
            stream: Some(stream),
            status,
        })
    }

    pub(crate) fn status(&self) -> LtcInputStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_else(|poisoned| poisoned.into_inner().clone())
    }
}

impl Drop for LtcInputRuntime {
    fn drop(&mut self) {
        // Close the capture first, so no callback runs after the worker.
        drop(self.stream.take());
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            worker.thread().unpark();
            let _ = worker.join();
        }
    }
}

fn run(
    sender: TimecodeSender,
    stop: &AtomicBool,
    status: &Mutex<LtcInputStatus>,
    blocks: &Blocks,
    errors: &Mutex<Option<(String, Instant)>>,
    sample_rate: &AtomicU64,
) {
    let sample_rate_hz = loop {
        if stop.load(Ordering::Acquire) {
            return;
        }
        match sample_rate.load(Ordering::Acquire) {
            0 => std::thread::park_timeout(Duration::from_millis(50)),
            hz => break hz as u32,
        }
    };
    let mut decoder = LtcDecoder::new(sample_rate_hz);
    let mut last_frame_at: Option<Instant> = None;
    let mut last_label: Option<(MtcTimecode, LtcFrameRate)> = None;
    let mut frames_decoded = 0_u64;
    let mut continuity = ContinuityGate::default();
    let mut next_status = Instant::now();
    while !stop.load(Ordering::Acquire) {
        let mut decoded_any = false;
        while let Some(mut block) = blocks.ready.pop() {
            let frames = decoder.push_samples(&block);
            block.clear();
            let _ = blocks.free.push(block);
            for frame in frames {
                let Some(rate) = frame.frame_rate else {
                    continue;
                };
                let Some((position_ms, label)) =
                    chase_position_ms(&frame, rate, decoder.sample_index(), sample_rate_hz)
                else {
                    continue;
                };
                frames_decoded += 1;
                decoded_any = true;
                if !continuity.accept(label.frame_count()) {
                    continue;
                }
                last_label = Some((label, rate));
                sender(EngineCommand::SyncTimelineTimecode {
                    position_ms,
                    source: protocol::ClockSource::Ltc,
                });
            }
        }
        let now = Instant::now();
        if decoded_any {
            last_frame_at = Some(now);
        } else if last_frame_at.is_some_and(|at| now - at >= SIGNAL_TIMEOUT) {
            continuity.reset();
        }
        if now >= next_status {
            next_status = now + STATUS_INTERVAL;
            if let Ok(mut current) = status.lock() {
                current.signal = last_frame_at.is_some_and(|at| now - at < SIGNAL_TIMEOUT);
                current.timecode = last_label.map(|(label, _)| label.label());
                current.frame_rate = last_label.map(|(_, rate)| rate_label(rate).to_string());
                current.frames_decoded = frames_decoded;
                current.dropped_samples = blocks.dropped.load(Ordering::Relaxed);
                // A stream error (a lost device) stays until restart; the
                // panel shows it whenever no LTC is being chased.
                current.last_error = errors
                    .lock()
                    .ok()
                    .and_then(|error| error.clone().map(|(error, _)| error));
            }
        }
        std::thread::park_timeout(Duration::from_millis(20));
    }
    if let Ok(mut current) = status.lock() {
        current.running = false;
        current.signal = false;
    }
}

#[cfg(test)]
#[path = "tests/ltc_input_tests.rs"]
mod tests;
