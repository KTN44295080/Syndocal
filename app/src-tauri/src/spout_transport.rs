#![cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use engine::EngineHandle;
use protocol::VideoLayerId;

pub type SpoutInputRegistry = Arc<Mutex<HashMap<VideoLayerId, video::VideoFrame>>>;

pub struct SpoutTransportState {
    inputs: SpoutInputRegistry,
    input_workers: HashMap<u64, SpoutRouteWorker>,
    output_workers: HashMap<u64, SpoutRouteWorker>,
    #[cfg(feature = "ndi")]
    ndi_inputs: crate::ndi_transport::NdiInputRegistry,
    capture_inputs: crate::capture_transport::CaptureInputRegistry,
}

impl SpoutTransportState {
    pub fn new(
        inputs: SpoutInputRegistry,
        #[cfg(feature = "ndi")] ndi_inputs: crate::ndi_transport::NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Self {
        Self {
            inputs,
            input_workers: HashMap::new(),
            output_workers: HashMap::new(),
            #[cfg(feature = "ndi")]
            ndi_inputs,
            capture_inputs,
        }
    }

    pub fn start_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        match route.direction {
            video::ExternalVideoTransportDirection::Input => {
                let worker = SpoutRouteWorker::start_input(
                    route.route_id,
                    route.endpoint_name.clone(),
                    Arc::clone(&self.inputs),
                )?;
                if let Some(previous) = self.input_workers.insert(route.route_id, worker) {
                    previous.stop();
                }
            }
            video::ExternalVideoTransportDirection::Output => {
                let worker = SpoutRouteWorker::start_output(
                    route.route_id,
                    route.endpoint_name.clone(),
                    engine.clone(),
                    Arc::clone(&self.inputs),
                    #[cfg(feature = "ndi")]
                    Arc::clone(&self.ndi_inputs),
                    Arc::clone(&self.capture_inputs),
                )?;
                if let Some(previous) = self.output_workers.insert(route.route_id, worker) {
                    previous.stop();
                }
            }
        }
        Ok(())
    }

    pub fn stop_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        match route.direction {
            video::ExternalVideoTransportDirection::Input => {
                if let Some(worker) = self.input_workers.remove(&route.route_id) {
                    worker.stop();
                }
                self.inputs
                    .lock()
                    .map_err(|_| driver_error("Spout input frame registry lock was poisoned"))?
                    .remove(&route.route_id);
            }
            video::ExternalVideoTransportDirection::Output => {
                if let Some(worker) = self.output_workers.remove(&route.route_id) {
                    worker.stop();
                }
            }
        }
        Ok(())
    }
}

struct SpoutRouteWorker {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

impl SpoutRouteWorker {
    fn start_input(
        layer_id: VideoLayerId,
        sender_name: String,
        frames: SpoutInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name(format!("syndocal-spout-input-{layer_id}"))
            .spawn(move || {
                let mut receiver = match spout2::dx::Receiver::new(Some(&sender_name)) {
                    Ok(receiver) => {
                        let _ = ready_tx.send(Ok(()));
                        receiver
                    }
                    Err(error) => {
                        let message =
                            format!("Failed to create Spout receiver '{sender_name}': {error}");
                        let _ = ready_tx.send(Err(message.clone()));
                        eprintln!("{message}");
                        return;
                    }
                };
                let (mut width, mut height) = (256_u32, 256_u32);
                let mut pixels = vec![0_u8; (width * height * 4) as usize];
                while !worker_stop.load(Ordering::Acquire) {
                    match receiver.receive_image(&mut pixels, width, height, false, false) {
                        Ok(connected) => {
                            if receiver.is_updated() {
                                (width, height) = receiver.sender_size();
                                if width == 0 || height == 0 {
                                    width = 256;
                                    height = 256;
                                }
                                pixels.resize((width * height * 4) as usize, 0);
                                continue;
                            }
                            if connected && receiver.is_frame_new() {
                                let format = match receiver.sender_format() {
                                    spout2::dx::format::B8G8R8A8_UNORM => {
                                        video::VideoPixelFormat::Bgra8
                                    }
                                    _ => video::VideoPixelFormat::Rgba8,
                                };
                                if let Ok(mut frames) = frames.lock() {
                                    frames.insert(
                                        layer_id,
                                        video::VideoFrame {
                                            layer_id,
                                            width,
                                            height,
                                            pts_ms: 0,
                                            duration_ms: 0,
                                            format,
                                            data: pixels.clone(),
                                        },
                                    );
                                }
                            }
                        }
                        Err(error) => {
                            eprintln!("Spout input '{sender_name}' stopped: {error}");
                            break;
                        }
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
            })
            .map_err(|error| {
                driver_error(format!("Failed to start Spout input worker: {error}"))
            })?;
        wait_until_ready(ready_rx, "Spout input", &stop, worker)
    }

    fn start_output(
        output_id: u64,
        sender_name: String,
        engine: EngineHandle,
        spout_inputs: SpoutInputRegistry,
        #[cfg(feature = "ndi")] ndi_inputs: crate::ndi_transport::NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let worker = std::thread::Builder::new()
            .name(format!("syndocal-spout-output-{output_id}"))
            .spawn(move || {
                let mut sender = match spout2::dx::Sender::new(&sender_name) {
                    Ok(mut sender) => {
                        sender.set_format(spout2::dx::format::R8G8B8A8_UNORM);
                        let _ = ready_tx.send(Ok(()));
                        sender
                    }
                    Err(error) => {
                        let message =
                            format!("Failed to create Spout sender '{sender_name}': {error}");
                        let _ = ready_tx.send(Err(message.clone()));
                        eprintln!("{message}");
                        return;
                    }
                };
                let decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::from_env()
                    .with_spout_inputs(spout_inputs)
                    .with_capture_inputs(capture_inputs);
                #[cfg(feature = "ndi")]
                let decoder = decoder.with_ndi_inputs(ndi_inputs);
                let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
                    video::VideoRuntimeConfig::default(),
                    video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
                );
                let target_interval = Duration::from_nanos(1_000_000_000 / 60);
                while !worker_stop.load(Ordering::Acquire) {
                    let started = Instant::now();
                    let snapshot = engine.snapshot();
                    renderer
                        .frame_provider_mut()
                        .set_bpm(Some(snapshot.clock.bpm));
                    let result = renderer
                        .render_output(&snapshot.video, output_id)
                        .map_err(|error| format!("{error:?}"))
                        .and_then(|frame| {
                            sender
                                .send_image(&frame.data, frame.width, frame.height)
                                .map_err(|error| error.to_string())
                        });
                    if let Err(error) = result {
                        eprintln!("Spout output '{sender_name}' stopped: {error}");
                        break;
                    }
                    if let Some(remaining) = target_interval.checked_sub(started.elapsed()) {
                        std::thread::sleep(remaining);
                    }
                }
            })
            .map_err(|error| {
                driver_error(format!("Failed to start Spout output worker: {error}"))
            })?;
        wait_until_ready(ready_rx, "Spout output", &stop, worker)
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn wait_until_ready(
    ready: mpsc::Receiver<Result<(), String>>,
    label: &str,
    stop: &Arc<AtomicBool>,
    worker: std::thread::JoinHandle<()>,
) -> Result<SpoutRouteWorker, video::ExternalVideoTransportDriverError> {
    match ready.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(SpoutRouteWorker {
            stop: Arc::clone(stop),
            worker: Some(worker),
        }),
        Ok(Err(error)) => {
            stop.store(true, Ordering::Release);
            let _ = worker.join();
            Err(driver_error(error))
        }
        Err(error) => {
            stop.store(true, Ordering::Release);
            let _ = worker.join();
            Err(driver_error(format!(
                "{label} initialization timed out: {error}"
            )))
        }
    }
}

fn driver_error(message: impl Into<String>) -> video::ExternalVideoTransportDriverError {
    video::ExternalVideoTransportDriverError {
        message: message.into(),
    }
}

impl Drop for SpoutRouteWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    #[ignore = "requires a Windows GPU and the real Spout2 DirectX transport"]
    fn input_worker_receives_a_real_spout_frame() {
        let name = format!(
            "syndocal-spout-loopback-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_nanos()
        );
        let mut sender = spout2::dx::Sender::new(&name).unwrap();
        sender.set_format(spout2::dx::format::R8G8B8A8_UNORM);
        let pixels = vec![0x7f_u8; 16 * 16 * 4];
        sender.send_image(&pixels, 16, 16).unwrap();
        let frames = Arc::new(Mutex::new(HashMap::new()));
        let worker = SpoutRouteWorker::start_input(7, name, Arc::clone(&frames)).unwrap();

        let mut received = None;
        for _ in 0..120 {
            sender.send_image(&pixels, 16, 16).unwrap();
            received = frames.lock().unwrap().get(&7).cloned();
            if received.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        worker.stop();

        let received = received.expect("Spout input worker did not receive a frame");
        assert_eq!((received.width, received.height), (16, 16));
        assert_eq!(received.format, video::VideoPixelFormat::Rgba8);
        assert_eq!(received.data, pixels);
    }

    #[test]
    #[ignore = "requires a Windows GPU and the real Spout2 DirectX transport"]
    fn output_worker_publishes_the_engine_composition() {
        let name = format!(
            "syndocal-spout-output-loopback-{}-{}",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or(Duration::ZERO)
                .as_nanos()
        );
        let engine = EngineHandle::start(protocol::DmxOutputConfig::default());
        engine
            .send(engine::EngineCommand::AddVideoOutput(
                protocol::VideoOutputSummary {
                    id: 9,
                    label: "Spout Test".to_string(),
                    kind: protocol::VideoOutputKind::SpoutSender,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    width: 16,
                    height: 16,
                    endpoint_name: Some(name.clone()),
                    opacity: 1.0,
                    blackout: false,
                    mapping: protocol::VideoOutputMapping::default(),
                },
            ))
            .unwrap();
        for _ in 0..40 {
            if engine
                .snapshot()
                .video
                .outputs
                .iter()
                .any(|output| output.id == 9)
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let worker = SpoutRouteWorker::start_output(
            9,
            name.clone(),
            engine,
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
        .unwrap();
        let mut receiver = spout2::dx::Receiver::new(Some(&name)).unwrap();
        let (mut width, mut height) = (16_u32, 16_u32);
        let mut pixels = vec![0_u8; (width * height * 4) as usize];
        let mut received = false;
        for _ in 0..120 {
            let connected = receiver
                .receive_image(&mut pixels, width, height, false, false)
                .unwrap();
            if receiver.is_updated() {
                (width, height) = receiver.sender_size();
                pixels.resize((width * height * 4) as usize, 0);
                continue;
            }
            if connected && receiver.is_frame_new() {
                received = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(8));
        }
        worker.stop();

        assert!(received, "Spout output worker did not publish a frame");
        assert_eq!((width, height), (16, 16));
        assert_eq!(pixels.len(), 16 * 16 * 4);
    }
}
