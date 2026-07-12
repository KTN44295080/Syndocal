use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

use crossbeam_queue::ArrayQueue;
use grafton_ndi::{
    Finder, FinderOptions, LineStrideOrSize, PixelFormat, Receiver, ReceiverColorFormat,
    ReceiverOptions, Sender, SenderOptions, VideoFrame, NDI,
};
use thiserror::Error;

const INPUT_FRAME_QUEUE_CAPACITY: usize = 2;
const DISCOVERY_POLL_INTERVAL: Duration = Duration::from_millis(250);
const CAPTURE_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Error)]
pub enum NdiError {
    #[error("NDI endpoint name is required")]
    EmptyEndpoint,
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
    frames: Arc<ArrayQueue<NdiRgbaFrame>>,
    running: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    error: Arc<Mutex<Option<String>>>,
    finished: mpsc::Receiver<()>,
    worker: Option<JoinHandle<()>>,
}

pub struct NdiOutput {
    endpoint_name: String,
    ndi: Option<NDI>,
    sender: Option<Sender>,
}

impl NdiInput {
    pub fn new(endpoint_name: impl Into<String>) -> Result<Self, NdiError> {
        let endpoint_name = normalize_endpoint(endpoint_name.into())?;
        let ndi = NDI::new().map_err(|error| NdiError::Runtime(error.to_string()))?;
        let finder = Finder::new(
            &ndi,
            &FinderOptions::builder().show_local_sources(true).build(),
        )
        .map_err(|error| NdiError::Runtime(error.to_string()))?;
        let frames = Arc::new(ArrayQueue::new(INPUT_FRAME_QUEUE_CAPACITY));
        let running = Arc::new(AtomicBool::new(true));
        let failed = Arc::new(AtomicBool::new(false));
        let error = Arc::new(Mutex::new(None));
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
                let failed = Arc::clone(&failed);
                let error = Arc::clone(&error);
                move || {
                    if let Err(worker_error) =
                        run_input_worker(ndi, finder, &endpoint_name, &frames, &running)
                    {
                        if let Ok(mut slot) = error.lock() {
                            *slot = Some(worker_error);
                        }
                        failed.store(true, Ordering::Release);
                        running.store(false, Ordering::Release);
                    }
                    let _ = finished_tx.send(());
                }
            })
            .map_err(|error| NdiError::Worker(error.to_string()))?;

        Ok(Self {
            endpoint_name,
            frames,
            running,
            failed,
            error,
            finished,
            worker: Some(worker),
        })
    }

    pub fn endpoint_name(&self) -> &str {
        &self.endpoint_name
    }

    pub fn take_latest(&self) -> Result<Option<NdiRgbaFrame>, NdiError> {
        self.check_worker()?;
        let mut latest = None;
        while let Some(frame) = self.frames.pop() {
            latest = Some(frame);
        }
        Ok(latest)
    }

    fn check_worker(&self) -> Result<(), NdiError> {
        if !self.failed.load(Ordering::Acquire) {
            return Ok(());
        }
        let message = self
            .error
            .lock()
            .map_err(|_| NdiError::Worker("worker error lock was poisoned".to_string()))?
            .clone()
            .unwrap_or_else(|| "worker stopped without an error message".to_string());
        Err(NdiError::Worker(message))
    }
}

impl Drop for NdiInput {
    fn drop(&mut self) {
        self.running.store(false, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            if self
                .finished
                .recv_timeout(Duration::from_millis(250))
                .is_ok()
            {
                let _ = worker.join();
            }
        }
    }
}

impl NdiOutput {
    pub fn new(endpoint_name: impl Into<String>) -> Result<Self, NdiError> {
        let endpoint_name = normalize_endpoint(endpoint_name.into())?;
        let ndi = NDI::new().map_err(|error| NdiError::Runtime(error.to_string()))?;
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
    frames: &ArrayQueue<NdiRgbaFrame>,
    running: &AtomicBool,
) -> Result<(), String> {
    while running.load(Ordering::Acquire) {
        finder
            .wait_for_sources(DISCOVERY_POLL_INTERVAL)
            .map_err(|error| error.to_string())?;
        let sources = finder
            .sources(Duration::ZERO)
            .map_err(|error| error.to_string())?;
        let Some(source) = sources
            .into_iter()
            .find(|source| source_name_matches(&source.name, endpoint_name))
        else {
            continue;
        };
        let receiver = Receiver::new(
            &ndi,
            &ReceiverOptions::builder(source)
                .color(ReceiverColorFormat::RGBX_RGBA)
                .build(),
        )
        .map_err(|error| error.to_string())?;
        while running.load(Ordering::Acquire) {
            let Some(frame) = receiver
                .capture_video_timeout(CAPTURE_POLL_INTERVAL)
                .map_err(|error| error.to_string())?
            else {
                continue;
            };
            enqueue_latest(frames, convert_received_frame(frame)?);
        }
    }
    Ok(())
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
    if stride < row_bytes || frame.data.len() < stride.saturating_mul(height) {
        return Err("received NDI frame buffer is shorter than its declared stride".to_string());
    }
    let mut rgba = vec![0; row_bytes.saturating_mul(height)];
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

fn enqueue_latest(queue: &ArrayQueue<NdiRgbaFrame>, frame: NdiRgbaFrame) {
    if let Err(frame) = queue.push(frame) {
        let _ = queue.pop();
        let _ = queue.push(frame);
    }
}

fn normalize_endpoint(endpoint_name: String) -> Result<String, NdiError> {
    let endpoint_name = endpoint_name.trim().to_string();
    if endpoint_name.is_empty() {
        Err(NdiError::EmptyEndpoint)
    } else {
        Ok(endpoint_name)
    }
}

fn source_name_matches(advertised_name: &str, endpoint_name: &str) -> bool {
    if advertised_name.eq_ignore_ascii_case(endpoint_name) {
        return true;
    }
    let advertised_name = advertised_name.trim();
    advertised_name
        .strip_suffix(')')
        .and_then(|name| name.rsplit_once('(').map(|(_, endpoint)| endpoint.trim()))
        .is_some_and(|endpoint| endpoint.eq_ignore_ascii_case(endpoint_name))
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
    fn latest_frame_queue_drops_stale_frame() {
        let queue = ArrayQueue::new(2);
        for marker in 1..=3 {
            enqueue_latest(
                &queue,
                NdiRgbaFrame {
                    width: 1,
                    height: 1,
                    frame_rate_n: 60,
                    frame_rate_d: 1,
                    rgba: vec![marker; 4],
                },
            );
        }
        assert_eq!(queue.pop().unwrap().rgba[0], 2);
        assert_eq!(queue.pop().unwrap().rgba[0], 3);
    }

    #[test]
    fn matches_plain_and_machine_qualified_source_names() {
        assert!(source_name_matches("Program", "program"));
        assert!(source_name_matches("STAGE-PC (Program)", "Program"));
        assert!(!source_name_matches("STAGE-PC (Preview)", "Program"));
    }

    #[test]
    #[ignore = "requires the NDI runtime and local network discovery"]
    fn sends_and_receives_rgba_over_local_ndi() {
        let endpoint = format!("Syndocal Loopback {}", std::process::id());
        eprintln!("ndi-loopback: create output");
        let output = NdiOutput::new(endpoint.clone()).unwrap();
        eprintln!("ndi-loopback: create input");
        let input = NdiInput::new(endpoint).unwrap();
        let expected = NdiRgbaFrame {
            width: 4,
            height: 2,
            frame_rate_n: 60,
            frame_rate_d: 1,
            rgba: [17, 83, 201, 255].repeat(8),
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
        let ndi = NDI::new().unwrap();
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
