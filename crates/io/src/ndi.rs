use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use crate::ndi_input_state::{
    exact_source_index, parse_discovered_sources, validate_source_name, NdiInputState,
};
pub use crate::ndi_input_state::NdiSourceDescriptor;
use grafton_ndi::{
    Finder, FinderOptions, LineStrideOrSize, PixelFormat, Receiver, ReceiverColorFormat,
    ReceiverOptions, Sender, SenderOptions, VideoFrame, NDI,
};
use thiserror::Error;

const INPUT_SIGNAL_TIMEOUT: Duration = Duration::from_secs(2);
const INPUT_RETRY_INTERVAL: Duration = Duration::from_millis(500);
const DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(250);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(50);
const OUTPUT_TEARDOWN_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Error)]
pub enum NdiError {
    #[error("NDI endpoint name is required")]
    EmptyEndpoint,
    #[error("NDI runtime not installed: {0}")]
    RuntimeMissing(String),
    #[error("NDI runtime initialization failed: {0}")]
    Runtime(String),
    #[error("NDI worker failed: {0}")]
    Worker(String),
    #[error("NDI frame dimensions must be greater than zero")]
    InvalidDimensions,
    #[error("NDI RGBA frame length {actual} does not match {expected}")]
    InvalidFrameLength { expected: usize, actual: usize },
    #[error("NDI frame dimensions exceed the SDK range")]
    DimensionsOutOfRange,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NdiRgbaFrame {
    pub width: u32,
    pub height: u32,
    pub frame_rate_n: u32,
    pub frame_rate_d: u32,
    pub rgba: Vec<u8>,
}

pub struct NdiInput {
    endpoint_name: String,
    frames: Arc<Mutex<NdiInputState<NdiRgbaFrame>>>,
    running: Arc<AtomicBool>,
    finished: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

pub struct NdiOutput {
    endpoint_name: String,
    ndi: Option<NDI>,
    sender: Option<Sender>,
}

/// Acknowledgement handle for asynchronous NDI SDK/source destruction. The
/// output-ownership teardown lease is owned by the cleanup thread and is
/// released only after the SDK objects have actually been dropped. Keeping
/// this handle in route state makes a timeout retryable without forgetting
/// the old sender.
#[derive(Debug, Clone)]
pub struct NdiOutputTeardown {
    state: Arc<(Mutex<NdiOutputTeardownState>, Condvar)>,
}

#[derive(Debug, Default)]
struct NdiOutputTeardownState {
    result: Option<Result<(), String>>,
    lease_released: bool,
}

impl NdiOutputTeardown {
    pub fn wait(&self, timeout: Duration) -> Result<(), NdiError> {
        let (result_lock, changed) = &*self.state;
        let mut state = result_lock
            .lock()
            .map_err(|_| NdiError::Worker("NDI teardown state lock was poisoned".to_string()))?;
        let deadline = Instant::now() + timeout;
        loop {
            if state.lease_released {
                let Some(result) = state.result.as_ref() else {
                    return Err(NdiError::Worker(
                        "NDI teardown acknowledgement was published without a result".to_string(),
                    ));
                };
                return match result {
                    Ok(()) => Ok(()),
                    Err(error) => Err(NdiError::Worker(error.clone())),
                };
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(NdiError::Worker(
                    "NDI output teardown acknowledgement timed out".to_string(),
                ));
            }
            let (next, wait_result) = changed
                .wait_timeout(state, remaining)
                .map_err(|_| NdiError::Worker("NDI teardown wait was poisoned".to_string()))?;
            state = next;
            if wait_result.timed_out() && !state.lease_released {
                return Err(NdiError::Worker(
                    "NDI output teardown acknowledgement timed out".to_string(),
                ));
            }
        }
    }

    pub fn is_complete(&self) -> bool {
        self.state
            .0
            .lock()
            .map(|state| state.lease_released && state.result.is_some())
            .unwrap_or(false)
    }
}

/// Snapshot discoverable LAN sources using the same guarded SDK as output.
/// Names are exact selectors; ambiguous duplicate names fail closed.
pub fn discover_sources(timeout: Duration) -> Result<Vec<NdiSourceDescriptor>, NdiError> {
    let ndi = crate::ndi_runtime::initialize_ndi()?;
    let finder = Finder::new(
        &ndi,
        &FinderOptions::builder().show_local_sources(true).build(),
    )
    .map_err(|error| NdiError::Runtime(error.to_string()))?;
    finder
        .wait_for_sources(timeout.min(Duration::from_secs(3)))
        .map_err(|error| NdiError::Worker(error.to_string()))?;
    let sources = finder
        .sources(Duration::ZERO)
        .map_err(|error| NdiError::Worker(error.to_string()))?;
    parse_discovered_sources(sources.into_iter().map(|source| source.name))
        .map_err(NdiError::Worker)
}

impl NdiInput {
    pub fn new(endpoint_name: impl Into<String>) -> Result<Self, NdiError> {
        let endpoint_name = endpoint_name.into();
        validate_source_name(&endpoint_name).map_err(NdiError::Worker)?;
        let ndi = crate::ndi_runtime::initialize_ndi()?;
        let finder = Finder::new(
            &ndi,
            &FinderOptions::builder().show_local_sources(true).build(),
        )
        .map_err(|error| NdiError::Runtime(error.to_string()))?;
        let frames = Arc::new(Mutex::new(NdiInputState::new()));
        let running = Arc::new(AtomicBool::new(true));
        let (finished_tx, finished) = mpsc::sync_channel(1);
        let worker = thread::Builder::new()
            .name(format!(
                "syndocal-ndi-input-{}",
                worker_name(&endpoint_name)
            ))
            .spawn({
                let endpoint_name = endpoint_name.clone();
                let frames = Arc::clone(&frames);
                let running = Arc::clone(&running);
                move || {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run_input_worker(ndi, finder, &endpoint_name, &frames, &running)
                    }));
                    let detail = match result {
                        Ok(Ok(())) => "receiver stopped".to_string(),
                        Ok(Err(error)) => error,
                        Err(_) => "NDI receiver worker panicked".to_string(),
                    };
                    frames
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner())
                        .invalidate(detail);
                    running.store(false, Ordering::Release);
                    let _ = finished_tx.send(());
                }
            })
            .map_err(|error| NdiError::Worker(error.to_string()))?;
        Ok(Self {
            endpoint_name,
            frames,
            running,
            finished,
            worker: Some(worker),
        })
    }

    pub fn endpoint_name(&self) -> &str {
        &self.endpoint_name
    }

    /// Reports startup/no-signal and loss until a valid video frame arrives.
    pub fn current_error(&self) -> Option<String> {
        match self.frames.lock() {
            Ok(mut state) => {
                state.expire(Instant::now(), INPUT_SIGNAL_TIMEOUT);
                state.current_error(&self.endpoint_name)
            }
            Err(_) => Some("NDI input state lock was poisoned".to_string()),
        }
    }

    /// Retained broadcast snapshot; Preview and outputs cannot consume each
    /// other's frames. The freshness check also covers a stalled SDK worker.
    pub fn latest_frame_shared(&self) -> Result<Option<Arc<NdiRgbaFrame>>, NdiError> {
        let mut state = self
            .frames
            .lock()
            .map_err(|_| NdiError::Worker("NDI input state lock was poisoned".to_string()))?;
        state.expire(Instant::now(), INPUT_SIGNAL_TIMEOUT);
        if !self.running.load(Ordering::Acquire) {
            return Ok(None);
        }
        Ok(state.latest())
    }

    /// Owned snapshot for existing direct I/O callers; it no longer drains
    /// the sole mailbox and therefore does not starve renderer consumers.
    pub fn take_latest(&self) -> Result<Option<NdiRgbaFrame>, NdiError> {
        self.latest_frame_shared()
            .map(|frame| frame.map(|frame| (*frame).clone()))
    }

    /// On timeout this object still owns the worker. Route state must retain
    /// it and retry, rather than replacing an unreaped receiver.
    pub fn stop(&mut self) -> Result<(), NdiError> {
        self.running.store(false, Ordering::Release);
        if let Ok(mut state) = self.frames.lock() {
            state.invalidate("receiver stopped");
        }
        if self.worker.is_none() {
            return Ok(());
        }
        match self.finished.recv_timeout(Duration::from_millis(250)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                if let Some(worker) = self.worker.take() {
                    worker.join().map_err(|_| {
                        NdiError::Worker("NDI input worker panicked during teardown".to_string())
                    })?;
                }
                Ok(())
            }
            Err(mpsc::RecvTimeoutError::Timeout) => Err(NdiError::Worker(
                "NDI input teardown acknowledgement timed out; receiver remains owned until retry"
                    .to_string(),
            )),
        }
    }
}

impl Drop for NdiInput {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        // Ordinary route teardown calls stop first. This final fallback must
        // retain ownership through SDK destruction, even if it blocks.
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl NdiOutput {
    pub fn new(endpoint_name: impl Into<String>) -> Result<Self, NdiError> {
        let endpoint_name = normalize_endpoint(endpoint_name.into())?;
        let ndi = crate::ndi_runtime::initialize_ndi()?;
        let sender = Sender::new(&ndi, &SenderOptions::builder(endpoint_name.clone()).build())
            .map_err(|error| NdiError::Runtime(error.to_string()))?;
        Ok(Self {
            endpoint_name,
            ndi: Some(ndi),
            sender: Some(sender),
        })
    }

    pub fn endpoint_name(&self) -> &str {
        &self.endpoint_name
    }

    pub fn send_rgba(&self, frame: &NdiRgbaFrame) -> Result<(), NdiError> {
        validate_rgba_frame(frame)?;
        let width = i32::try_from(frame.width).map_err(|_| NdiError::DimensionsOutOfRange)?;
        let height = i32::try_from(frame.height).map_err(|_| NdiError::DimensionsOutOfRange)?;
        let frame_rate_n =
            i32::try_from(frame.frame_rate_n.max(1)).map_err(|_| NdiError::DimensionsOutOfRange)?;
        let frame_rate_d =
            i32::try_from(frame.frame_rate_d.max(1)).map_err(|_| NdiError::DimensionsOutOfRange)?;
        let mut ndi_frame = VideoFrame::builder()
            .resolution(width, height)
            .pixel_format(PixelFormat::RGBA)
            .frame_rate(frame_rate_n, frame_rate_d)
            .aspect_ratio(frame.width as f32 / frame.height as f32)
            .build()
            .map_err(|error| NdiError::Runtime(error.to_string()))?;
        ndi_frame.data.copy_from_slice(&frame.rgba);
        self.sender
            .as_ref()
            .ok_or_else(|| NdiError::Worker("NDI sender is already stopped".to_string()))?
            .send_video(&ndi_frame);
        Ok(())
    }

    /// Begin output destruction on a cleanup thread and keep the supplied
    /// ownership lease there until the SDK/source objects are actually gone.
    /// Callers may retain the returned handle after a timeout and retry the
    /// acknowledgement without creating a replacement sender.
    pub fn begin_close_with_lease<L>(mut self, lease: L) -> Result<NdiOutputTeardown, NdiError>
    where
        L: Send + 'static,
    {
        let Some(sender) = self.sender.take() else {
            drop(self.ndi.take());
            return start_teardown(|| {}, lease);
        };
        let ndi = self.ndi.take();
        start_teardown(
            move || {
                drop(sender);
                drop(ndi);
            },
            lease,
        )
    }

    /// Compatibility helper for direct I/O callers. App-owned NDI output
    /// routes must use `begin_close_with_lease` so transitions retain the
    /// physical teardown fence.
    pub fn close(self) -> Result<(), NdiError> {
        let teardown = self.begin_close_with_lease(())?;
        teardown.wait(OUTPUT_TEARDOWN_TIMEOUT)
    }
}

fn start_teardown<F, L>(cleanup: F, lease: L) -> Result<NdiOutputTeardown, NdiError>
where
    F: FnOnce() + Send + 'static,
    L: Send + 'static,
{
    let state = Arc::new((
        Mutex::new(NdiOutputTeardownState::default()),
        Condvar::new(),
    ));
    let worker_state = Arc::clone(&state);
    thread::Builder::new()
        .name("syndocal-ndi-output-cleanup".to_string())
        .spawn(move || {
            let cleanup_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(cleanup));
            let result = cleanup_result
                .map(|_| ())
                .map_err(|_| "NDI output cleanup thread panicked".to_string());
            let mut slot = worker_state
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.result = Some(result);
            worker_state.1.notify_all();
            drop(slot);
            // The public acknowledgement is complete only after the lease
            // has drained. A recovery transition can therefore not become
            // Ready in the interval between SDK destruction and ownership
            // accounting.
            drop(lease);
            let mut slot = worker_state
                .0
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.lease_released = true;
            worker_state.1.notify_all();
        })
        .map_err(|error| {
            NdiError::Worker(format!("NDI output cleanup failed to start: {error}"))
        })?;
    Ok(NdiOutputTeardown { state })
}

#[cfg(test)]
fn teardown_with_ack<F>(cleanup: F, timeout: Duration) -> Result<(), NdiError>
where
    F: FnOnce() + Send + 'static,
{
    let teardown = start_teardown(cleanup, ())?;
    teardown.wait(timeout)
}

impl Drop for NdiOutput {
    fn drop(&mut self) {
        let Some(sender) = self.sender.take() else {
            return;
        };
        let ndi = self.ndi.take();
        let _ = thread::Builder::new()
            .name("syndocal-ndi-output-cleanup".to_string())
            .spawn(move || {
                drop(sender);
                drop(ndi);
            });
    }
}

fn run_input_worker(
    ndi: NDI,
    finder: Finder,
    endpoint_name: &str,
    frames: &Mutex<NdiInputState<NdiRgbaFrame>>,
    running: &AtomicBool,
) -> Result<(), String> {
    while running.load(Ordering::Acquire) {
        let discovery_started = Instant::now();
        let discovery = finder
            .wait_for_sources(DISCOVERY_POLL_INTERVAL)
            .and_then(|_| finder.sources(Duration::ZERO));
        let mut sources = match discovery {
            Ok(sources) => sources,
            Err(error) => {
                invalidate_input(
                    frames,
                    format!("source discovery failed: {error}; retrying"),
                )?;
                wait_input_retry(running, INPUT_RETRY_INTERVAL);
                continue;
            }
        };
        let source_index = exact_source_index(
            endpoint_name,
            sources.iter().map(|source| source.name.as_str()),
        );
        let source_index = match source_index {
            Ok(Some(index)) => index,
            Ok(None) => {
                invalidate_input(frames, "selected source is not present; refresh NDI sources and verify its full name; retrying discovery".to_string())?;
                wait_input_retry(
                    running,
                    DISCOVERY_POLL_INTERVAL.saturating_sub(discovery_started.elapsed()),
                );
                continue;
            }
            Err(error) => {
                invalidate_input(frames, error)?;
                wait_input_retry(running, INPUT_RETRY_INTERVAL);
                continue;
            }
        };
        if !running.load(Ordering::Acquire) {
            break;
        }
        let receiver = match Receiver::new(
            &ndi,
            &ReceiverOptions::builder(sources.swap_remove(source_index))
                .color(ReceiverColorFormat::RGBX_RGBA)
                .build(),
        ) {
            Ok(receiver) => receiver,
            Err(error) => {
                invalidate_input(
                    frames,
                    format!("receiver connection failed: {error}; reconnecting"),
                )?;
                wait_input_retry(running, INPUT_RETRY_INTERVAL);
                continue;
            }
        };
        let mut last_frame_at = Instant::now();
        while running.load(Ordering::Acquire) {
            let capture_started = Instant::now();
            match receiver.capture_video_timeout(CAPTURE_POLL_INTERVAL) {
                Ok(Some(frame)) => match convert_received_frame(frame) {
                    Ok(frame) => {
                        last_frame_at = Instant::now();
                        frames
                            .lock()
                            .map_err(|_| "NDI input state lock was poisoned".to_string())?
                            .publish(frame, last_frame_at);
                    }
                    Err(error) => {
                        invalidate_input(frames, format!("{error}; reconnecting"))?;
                        break;
                    }
                },
                Ok(None) => {
                    if last_frame_at.elapsed() >= INPUT_SIGNAL_TIMEOUT {
                        invalidate_input(
                            frames,
                            "no video frames received; reconnecting to the selected source"
                                .to_string(),
                        )?;
                        break;
                    }
                    // Some SDK no-video events return immediately. Bound CPU
                    // usage while waiting instead of busy spinning.
                    wait_input_retry(
                        running,
                        Duration::from_millis(10).saturating_sub(capture_started.elapsed()),
                    );
                }
                Err(error) => {
                    invalidate_input(frames, format!("capture failed: {error}; reconnecting"))?;
                    break;
                }
            }
        }
        // Drop the old receiver before discovery can construct a replacement.
        drop(receiver);
        wait_input_retry(running, INPUT_RETRY_INTERVAL);
    }
    Ok(())
}

fn invalidate_input(
    frames: &Mutex<NdiInputState<NdiRgbaFrame>>,
    detail: String,
) -> Result<(), String> {
    frames
        .lock()
        .map_err(|_| "NDI input state lock was poisoned".to_string())?
        .invalidate(detail);
    Ok(())
}

fn wait_input_retry(running: &AtomicBool, duration: Duration) {
    let deadline = Instant::now() + duration;
    while running.load(Ordering::Acquire) {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            break;
        }
        thread::sleep(remaining.min(Duration::from_millis(10)));
    }
}

fn convert_received_frame(frame: VideoFrame) -> Result<NdiRgbaFrame, String> {
    if frame.width <= 0 || frame.height <= 0 {
        return Err("received an NDI frame with invalid dimensions".to_string());
    }
    let stride = match frame.line_stride_or_size {
        LineStrideOrSize::LineStrideBytes(stride) if stride > 0 => stride as usize,
        _ => return Err("received an NDI frame without a valid line stride".to_string()),
    };
    let width = frame.width as usize;
    let height = frame.height as usize;
    let row_bytes = width
        .checked_mul(4)
        .ok_or_else(|| "received NDI frame dimensions overflow".to_string())?;
    let required_length = stride
        .checked_mul(height)
        .ok_or_else(|| "received NDI frame stride overflows".to_string())?;
    let packed_length = row_bytes
        .checked_mul(height)
        .ok_or_else(|| "received NDI frame dimensions overflow".to_string())?;
    if stride < row_bytes || frame.data.len() < required_length {
        return Err("received NDI frame buffer is shorter than its declared stride".to_string());
    }
    let mut rgba = Vec::new();
    rgba
        .try_reserve_exact(packed_length)
        .map_err(|error| format!("cannot allocate the received NDI frame: {error}"))?;
    rgba.resize(packed_length, 0);
    for (source, target) in frame
        .data
        .chunks_exact(stride)
        .zip(rgba.chunks_exact_mut(row_bytes))
        .take(height)
    {
        target.copy_from_slice(&source[..row_bytes]);
    }
    match frame.pixel_format {
        PixelFormat::RGBA => {}
        PixelFormat::RGBX => {
            for alpha in rgba.iter_mut().skip(3).step_by(4) {
                *alpha = u8::MAX;
            }
        }
        PixelFormat::BGRA | PixelFormat::BGRX => {
            for pixel in rgba.chunks_exact_mut(4) {
                pixel.swap(0, 2);
                if frame.pixel_format == PixelFormat::BGRX {
                    pixel[3] = u8::MAX;
                }
            }
        }
        format => return Err(format!("unsupported NDI receive pixel format {format:?}")),
    }
    Ok(NdiRgbaFrame {
        width: frame.width as u32,
        height: frame.height as u32,
        frame_rate_n: frame.frame_rate_n.max(1) as u32,
        frame_rate_d: frame.frame_rate_d.max(1) as u32,
        rgba,
    })
}

fn validate_rgba_frame(frame: &NdiRgbaFrame) -> Result<(), NdiError> {
    if frame.width == 0 || frame.height == 0 {
        return Err(NdiError::InvalidDimensions);
    }
    let expected = frame
        .width
        .checked_mul(frame.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .map(|bytes| bytes as usize)
        .ok_or(NdiError::DimensionsOutOfRange)?;
    if frame.rgba.len() != expected {
        return Err(NdiError::InvalidFrameLength {
            expected,
            actual: frame.rgba.len(),
        });
    }
    Ok(())
}

fn normalize_endpoint(endpoint_name: String) -> Result<String, NdiError> {
    let endpoint_name = endpoint_name.trim().to_string();
    if endpoint_name.is_empty() {
        Err(NdiError::EmptyEndpoint)
    } else {
        Ok(endpoint_name)
    }
}

fn worker_name(endpoint_name: &str) -> String {
    endpoint_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .take(48)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_rgba_frame_length() {
        let frame = NdiRgbaFrame {
            width: 2,
            height: 2,
            frame_rate_n: 60,
            frame_rate_d: 1,
            rgba: vec![0; 15],
        };
        assert!(matches!(
            validate_rgba_frame(&frame),
            Err(NdiError::InvalidFrameLength {
                expected: 16,
                actual: 15
            })
        ));
    }

    #[test]
    fn teardown_ack_waits_for_injected_cleanup() {
        let (started_tx, started_rx) = mpsc::sync_channel(1);
        let result = teardown_with_ack(
            move || {
                started_tx.send(()).unwrap();
                thread::sleep(Duration::from_millis(20));
            },
            Duration::from_secs(1),
        );
        started_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(result.is_ok());
    }

    #[test]
    fn teardown_ack_timeout_is_a_failure() {
        let result = teardown_with_ack(
            || thread::sleep(Duration::from_millis(100)),
            Duration::from_millis(1),
        );
        assert!(
            matches!(result, Err(NdiError::Worker(message)) if message.contains("acknowledgement timed out"))
        );
    }

    #[test]
    fn injected_slow_cleanup_remains_retryable_until_acknowledged() {
        let teardown = start_teardown(|| thread::sleep(Duration::from_millis(80)), ()).unwrap();
        assert!(teardown.wait(Duration::from_millis(1)).is_err());
        assert!(!teardown.is_complete());
        teardown.wait(Duration::from_secs(1)).unwrap();
        assert!(teardown.is_complete());
    }

    #[test]
    #[ignore = "requires the NDI runtime and local network discovery"]
    fn sends_and_receives_rgba_over_local_ndi() {
        let endpoint = format!("Syndocal Loopback {}", std::process::id());
        eprintln!("ndi-loopback: create output");
        let output = NdiOutput::new(endpoint.clone()).unwrap();
        let expected = NdiRgbaFrame {
            width: 4,
            height: 2,
            frame_rate_n: 60,
            frame_rate_d: 1,
            rgba: [17, 83, 201, 255].repeat(8),
        };
        output.send_rgba(&expected).unwrap();
        let discovery_deadline = Instant::now() + Duration::from_secs(8);
        let input = loop {
            let mut sources: Vec<_> = discover_sources(Duration::from_millis(250)).unwrap()
                .into_iter()
                .filter(|source| source.name == endpoint || source.name.ends_with(&format!(" ({endpoint})")))
                .collect();
            assert!(sources.len() <= 1, "loopback sender advertisement is ambiguous");
            if let Some(source) = sources.pop() {
                break NdiInput::new(source.endpoint_name).unwrap();
            }
            assert!(Instant::now() < discovery_deadline, "loopback sender was not discovered");
            output.send_rgba(&expected).unwrap();
        };
        let deadline = std::time::Instant::now() + Duration::from_secs(8);
        eprintln!("ndi-loopback: send and discover");
        let received = loop {
            output.send_rgba(&expected).unwrap();
            if let Some(frame) = input.take_latest().unwrap() {
                break frame;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "local NDI source was not discovered within 8 seconds"
            );
            thread::sleep(Duration::from_millis(16));
        };
        assert_eq!((received.width, received.height), (4, 2));
        for (actual, expected) in received.rgba[..3].iter().zip([17u8, 83, 201]) {
            assert!(actual.abs_diff(expected) <= 5);
        }
        eprintln!("ndi-loopback: received; dropping input");
        drop(input);
        eprintln!("ndi-loopback: input dropped; dropping output");
        drop(output);
        eprintln!("ndi-loopback: output dropped");
    }

    #[test]
    #[ignore = "requires an external NDI sender and the NDI runtime"]
    fn receives_rgba_from_an_external_ndi_application() {
        let source_match = std::env::var("SYNDOCAL_TEST_NDI_SOURCE").unwrap_or_default();
        let ndi = crate::ndi_runtime::initialize_ndi().unwrap();
        let finder = Finder::new(
            &ndi,
            &FinderOptions::builder().show_local_sources(true).build(),
        )
        .unwrap();
        let discovery_deadline = std::time::Instant::now() + Duration::from_secs(8);
        let source_name = loop {
            finder.wait_for_sources(DISCOVERY_POLL_INTERVAL).unwrap();
            let sources = finder.sources(Duration::ZERO).unwrap();
            eprintln!(
                "external NDI sources: {:?}",
                sources
                    .iter()
                    .map(|source| &source.name)
                    .collect::<Vec<_>>()
            );
            if let Some(source) = sources.into_iter().find(|source| {
                source_match.is_empty()
                    || source
                        .name
                        .to_lowercase()
                        .contains(&source_match.to_lowercase())
            }) {
                break source.name;
            }
            assert!(
                std::time::Instant::now() < discovery_deadline,
                "no external NDI source matched '{source_match}'"
            );
        };
        let input = NdiInput::new(source_name.clone()).unwrap();
        let frame_deadline = std::time::Instant::now() + Duration::from_secs(8);
        let received = loop {
            if let Some(frame) = input.take_latest().unwrap() {
                break frame;
            }
            assert!(
                std::time::Instant::now() < frame_deadline,
                "external NDI source '{source_name}' produced no RGBA frame"
            );
            thread::sleep(Duration::from_millis(16));
        };
        eprintln!(
            "received external NDI source '{source_name}': {}x{} @ {}/{}",
            received.width, received.height, received.frame_rate_n, received.frame_rate_d
        );
        assert!(received.width > 0 && received.height > 0);
        assert_eq!(
            received.rgba.len(),
            (received.width * received.height * 4) as usize
        );
    }

    #[test]
    #[ignore = "requires an external NDI receiver and the NDI runtime"]
    fn publishes_rgba_to_an_external_ndi_application_and_survives_resize() {
        let endpoint = std::env::var("SYNDOCAL_TEST_NDI_OUTPUT")
            .unwrap_or_else(|_| "Syndocal External QA".to_string());
        let output = NdiOutput::new(endpoint.clone()).unwrap();
        let started = std::time::Instant::now();
        let deadline = started + Duration::from_secs(90);
        let mut connected_at = None;
        let mut resized_at = None;
        let mut frame_index = 0u64;

        eprintln!("NDI_EXTERNAL_OUTPUT_READY {endpoint}");
        loop {
            let now = std::time::Instant::now();
            assert!(
                now < deadline,
                "external NDI receiver did not connect and remain connected through resize"
            );
            let connected = output
                .sender
                .as_ref()
                .unwrap()
                .connection_count(Duration::from_millis(10))
                .unwrap_or(0);
            if connected > 0 && connected_at.is_none() {
                connected_at = Some(now);
                eprintln!("NDI_EXTERNAL_OUTPUT_CONNECTED clients={connected}");
            }

            let should_resize = connected_at.is_some_and(|connected| {
                now.saturating_duration_since(connected) >= Duration::from_secs(8)
            });
            let (width, height) = if should_resize {
                if resized_at.is_none() {
                    resized_at = Some(now);
                    eprintln!("NDI_EXTERNAL_OUTPUT_RESIZED 1280x720");
                }
                (1280, 720)
            } else {
                (640, 360)
            };
            output
                .send_rgba(&external_qa_frame(width, height, frame_index))
                .unwrap();
            frame_index += 1;

            if resized_at.is_some_and(|resized| {
                connected > 0 && now.saturating_duration_since(resized) >= Duration::from_secs(8)
            }) {
                eprintln!("NDI_EXTERNAL_OUTPUT_COMPLETE frames={frame_index} clients={connected}");
                break;
            }
            thread::sleep(Duration::from_millis(33));
        }
    }

    fn external_qa_frame(width: u32, height: u32, frame_index: u64) -> NdiRgbaFrame {
        let mut rgba = vec![0; (width * height * 4) as usize];
        let moving_bar = (frame_index as u32 * 7) % width;
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let band = (x * 6 / width).min(5);
                let (mut red, mut green, mut blue) = match band {
                    0 => (255, 32, 32),
                    1 => (255, 220, 32),
                    2 => (32, 255, 64),
                    3 => (32, 220, 255),
                    4 => (64, 64, 255),
                    _ => (220, 32, 255),
                };
                if x.abs_diff(moving_bar) < 12 || y == height / 2 {
                    red = 255;
                    green = 255;
                    blue = 255;
                }
                rgba[offset..offset + 4].copy_from_slice(&[red, green, blue, 255]);
            }
        }
        NdiRgbaFrame {
            width,
            height,
            frame_rate_n: 30,
            frame_rate_d: 1,
            rgba,
        }
    }
}
