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

    #[test]
    #[ignore = "requires an external Spout receiver application and operator confirmation"]
    fn output_worker_publishes_to_an_external_spout_application_and_survives_resize() {
        let sender_name = std::env::var("SYNDOCAL_TEST_SPOUT_OUTPUT")
            .unwrap_or_else(|_| "Syndocal External QA".to_string());
        let confirmation_path = std::env::var("SYNDOCAL_TEST_SPOUT_CONFIRM_FILE")
            .expect("set SYNDOCAL_TEST_SPOUT_CONFIRM_FILE to an operator-controlled file");
        let engine = EngineHandle::start(protocol::DmxOutputConfig::default());
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(engine::EngineCommand::AddVideoLayer {
                layer_id,
                label: "External Spout QA".to_string(),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::Spout,
                    path: None,
                    name: Some("External Spout QA Frame".to_string()),
                    codec: None,
                    metadata: None,
                },
            })
            .unwrap();
        let output_id = engine.allocate_video_output_id();
        engine
            .send(engine::EngineCommand::AddVideoOutput(
                protocol::VideoOutputSummary {
                    id: output_id,
                    label: "External Spout QA".to_string(),
                    kind: protocol::VideoOutputKind::SpoutSender,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    width: 640,
                    height: 360,
                    endpoint_name: Some(sender_name.clone()),
                    opacity: 1.0,
                    blackout: false,
                    mapping: protocol::VideoOutputMapping::default(),
                },
            ))
            .unwrap();
        let spout_inputs = Arc::new(Mutex::new(HashMap::from([(
            layer_id,
            external_qa_spout_frame(layer_id, 640, 360),
        )])));
        let worker = SpoutRouteWorker::start_output(
            output_id,
            sender_name.clone(),
            engine.clone(),
            Arc::clone(&spout_inputs),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
        .unwrap();

        eprintln!("SPOUT_EXTERNAL_OUTPUT_READY {sender_name} 640x360");
        wait_for_spout_operator_confirmation(&confirmation_path, "connected");
        engine
            .send(engine::EngineCommand::SetVideoOutputConfig {
                output_id,
                label: "External Spout QA".to_string(),
                kind: protocol::VideoOutputKind::SpoutSender,
                fullscreen: false,
                monitor_id: None,
                width: 1280,
                height: 720,
                endpoint_name: Some(sender_name),
            })
            .unwrap();
        eprintln!("SPOUT_EXTERNAL_OUTPUT_RESIZED 1280x720");
        wait_for_spout_operator_confirmation(&confirmation_path, "resized");
        worker.stop();
        eprintln!("SPOUT_EXTERNAL_OUTPUT_COMPLETE");
    }

    fn wait_for_spout_operator_confirmation(path: &str, expected: &str) {
        let deadline = Instant::now() + Duration::from_secs(90);
        loop {
            let confirmation = std::fs::read_to_string(path).unwrap_or_default();
            if confirmation.lines().any(|line| line.trim() == expected) {
                return;
            }
            assert!(
                Instant::now() < deadline,
                "external Spout receiver did not confirm '{expected}' within 90 seconds"
            );
            std::thread::sleep(Duration::from_millis(100));
        }
    }

    fn external_qa_spout_frame(
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
    ) -> video::VideoFrame {
        let mut data = vec![0; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let offset = ((y * width + x) * 4) as usize;
                let band = (x * 6 / width).min(5);
                let (red, green, blue) = match band {
                    0 => (255, 32, 32),
                    1 => (255, 220, 32),
                    2 => (32, 255, 64),
                    3 => (32, 220, 255),
                    4 => (64, 64, 255),
                    _ => (220, 32, 255),
                };
                data[offset..offset + 4].copy_from_slice(&[red, green, blue, 255]);
            }
        }
        video::VideoFrame {
            layer_id,
            width,
            height,
            pts_ms: 0,
            duration_ms: 33,
            format: video::VideoPixelFormat::Rgba8,
            data,
        }
    }

    #[test]
    #[ignore = "requires an external Spout sender application and a Windows GPU"]
    fn input_worker_receives_from_an_external_spout_application() {
        let sender_match = std::env::var("SYNDOCAL_TEST_SPOUT_SENDER").unwrap_or_default();
        let probe = spout2::dx::Receiver::new(None).unwrap();
        let senders = probe.sender_list();
        eprintln!("external Spout senders: {senders:?}");
        let sender_name = senders
            .iter()
            .find(|name| {
                sender_match.is_empty()
                    || name.to_lowercase().contains(&sender_match.to_lowercase())
            })
            .cloned()
            .unwrap_or_else(|| panic!("no external Spout sender matched '{sender_match}'"));
        let sender_info = probe.sender_info(&sender_name).unwrap();
        eprintln!("selected external Spout sender '{sender_name}': {sender_info:?}");
        let mut rate_receiver = spout2::dx::Receiver::new(Some(&sender_name)).unwrap();
        let (mut rate_width, mut rate_height) = (sender_info.width, sender_info.height);
        let mut rate_pixels = vec![0_u8; (rate_width * rate_height * 4) as usize];
        let rate_started = Instant::now();
        let rate_deadline = rate_started + Duration::from_secs(2);
        let mut new_frames = 0_u64;
        while Instant::now() < rate_deadline {
            let connected = rate_receiver
                .receive_image(&mut rate_pixels, rate_width, rate_height, false, false)
                .unwrap();
            if rate_receiver.is_updated() {
                (rate_width, rate_height) = rate_receiver.sender_size();
                rate_pixels.resize((rate_width * rate_height * 4) as usize, 0);
                continue;
            }
            if connected && rate_receiver.is_frame_new() {
                new_frames += 1;
            }
            std::thread::sleep(Duration::from_millis(4));
        }
        let observed_fps = new_frames as f64 / rate_started.elapsed().as_secs_f64();
        eprintln!("external Spout observed frame rate: {observed_fps:.1} fps");
        assert!(
            observed_fps >= 30.0,
            "external Spout sender delivered only {observed_fps:.1} fps"
        );
        drop(rate_receiver);
        let frames = Arc::new(Mutex::new(HashMap::new()));
        let worker = SpoutRouteWorker::start_input(91, sender_name, Arc::clone(&frames)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(8);
        let received = loop {
            if let Some(frame) = frames.lock().unwrap().get(&91).cloned() {
                break frame;
            }
            assert!(
                Instant::now() < deadline,
                "Syndocal input worker did not receive an external Spout frame"
            );
            std::thread::sleep(Duration::from_millis(16));
        };
        worker.stop();

        assert_eq!(
            (received.width, received.height),
            (sender_info.width, sender_info.height)
        );
        assert_eq!(
            received.data.len(),
            (received.width * received.height * 4) as usize
        );
    }

    #[test]
    #[ignore = "requires an external Spout sender that the test operator restarts"]
    fn input_worker_recovers_when_external_spout_sender_restarts() {
        let sender_name = std::env::var("SYNDOCAL_TEST_SPOUT_SENDER")
            .expect("set SYNDOCAL_TEST_SPOUT_SENDER to the external sender name");
        let frames = Arc::new(Mutex::new(HashMap::new()));
        let worker =
            SpoutRouteWorker::start_input(92, sender_name.clone(), Arc::clone(&frames)).unwrap();
        let first_deadline = Instant::now() + Duration::from_secs(8);
        while !frames.lock().unwrap().contains_key(&92) {
            assert!(
                Instant::now() < first_deadline,
                "initial external frame timed out"
            );
            std::thread::sleep(Duration::from_millis(16));
        }
        eprintln!("SPOUT_RECONNECT_READY");

        let probe = spout2::dx::Receiver::new(None).unwrap();
        let disconnect_deadline = Instant::now() + Duration::from_secs(30);
        while probe.sender_list().iter().any(|name| name == &sender_name) {
            assert!(
                Instant::now() < disconnect_deadline,
                "external sender was not stopped within 30 seconds"
            );
            std::thread::sleep(Duration::from_millis(50));
        }
        frames.lock().unwrap().remove(&92);
        eprintln!("SPOUT_RECONNECT_DISCONNECTED");

        let reconnect_deadline = Instant::now() + Duration::from_secs(30);
        while !frames.lock().unwrap().contains_key(&92) {
            assert!(
                Instant::now() < reconnect_deadline,
                "Syndocal input worker did not recover after the sender restarted"
            );
            std::thread::sleep(Duration::from_millis(16));
        }
        worker.stop();
        eprintln!("SPOUT_RECONNECT_RECOVERED");
    }
}
