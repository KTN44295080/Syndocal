use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(feature = "ndi")]
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

#[cfg(feature = "ndi")]
use engine::EngineHandle;
use protocol::{VideoLayerId, VideoSourceKind};

pub struct NdiAwareVideoFrameDecoder {
    files: video::PreferredVideoFrameDecoder,
    #[cfg(feature = "ndi")]
    ndi_inputs: NdiInputRegistry,
    #[cfg(feature = "ndi")]
    ndi_frames: HashMap<u64, video::VideoFrame>,
    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    spout_inputs: crate::spout_transport::SpoutInputRegistry,
    capture_inputs: crate::capture_transport::CaptureInputRegistry,
}

impl NdiAwareVideoFrameDecoder {
    pub fn from_env() -> Self {
        Self {
            files: video::PreferredVideoFrameDecoder::from_env(),
            #[cfg(feature = "ndi")]
            ndi_inputs: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            ndi_frames: HashMap::new(),
            #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
            spout_inputs: Arc::new(Mutex::new(HashMap::new())),
            capture_inputs: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    #[cfg(feature = "ndi")]
    pub fn with_ndi_inputs(mut self, ndi_inputs: NdiInputRegistry) -> Self {
        self.ndi_inputs = ndi_inputs;
        self
    }

    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    pub fn with_spout_inputs(
        mut self,
        spout_inputs: crate::spout_transport::SpoutInputRegistry,
    ) -> Self {
        self.spout_inputs = spout_inputs;
        self
    }

    pub fn with_capture_inputs(
        mut self,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Self {
        self.capture_inputs = capture_inputs;
        self
    }

    pub fn diagnostics(&self) -> video::VideoDecoderDiagnostics {
        self.files.diagnostics()
    }

    pub fn cache_len(&self) -> usize {
        let file_frames = self.files.cache_len();
        let capture_frames = self
            .capture_inputs
            .lock()
            .map(|frames| frames.len())
            .unwrap_or(0);
        #[cfg(feature = "ndi")]
        return file_frames
            .saturating_add(self.ndi_frames.len())
            .saturating_add(capture_frames);
        #[cfg(not(feature = "ndi"))]
        file_frames.saturating_add(capture_frames)
    }
}

impl video::VideoFrameDecoder for NdiAwareVideoFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        video::VideoFrameDecoder::retain_layers(&mut self.files, layer_ids);
        #[cfg(feature = "ndi")]
        self.ndi_frames
            .retain(|layer_id, _| layer_ids.contains(layer_id));
        if let Ok(mut frames) = self.capture_inputs.lock() {
            frames.retain(|layer_id, _| layer_ids.contains(layer_id));
        }
    }

    fn decode_frame(
        &mut self,
        request: &video::VideoFrameRequest,
    ) -> Result<Option<video::VideoFrame>, video::VideoDecodeError> {
        #[cfg(feature = "ndi")]
        if request.source.kind == VideoSourceKind::Ndi {
            let inputs = self
                .ndi_inputs
                .lock()
                .map_err(|_| video::VideoDecodeError::Decode {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    message: "NDI input state lock was poisoned".to_string(),
                })?;
            let Some(input) = inputs.get(&request.layer_id) else {
                return Ok(self.ndi_frames.get(&request.layer_id).cloned());
            };
            if let Some(frame) =
                input
                    .take_latest()
                    .map_err(|error| video::VideoDecodeError::Decode {
                        layer_id: request.layer_id,
                        label: request.label.clone(),
                        message: error.to_string(),
                    })?
            {
                self.ndi_frames.insert(
                    request.layer_id,
                    video::VideoFrame {
                        layer_id: request.layer_id,
                        width: frame.width,
                        height: frame.height,
                        pts_ms: request.position_ms,
                        duration_ms: 0,
                        format: video::VideoPixelFormat::Rgba8,
                        data: frame.rgba,
                    },
                );
            }
            return Ok(self.ndi_frames.get(&request.layer_id).cloned());
        }
        #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
        if request.source.kind == VideoSourceKind::Spout {
            let mut frame = self
                .spout_inputs
                .lock()
                .map_err(|_| video::VideoDecodeError::Decode {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    message: "Spout input frame registry lock was poisoned".to_string(),
                })?
                .get(&request.layer_id)
                .cloned();
            if let Some(frame) = &mut frame {
                frame.layer_id = request.layer_id;
                frame.pts_ms = request.position_ms;
            }
            return Ok(frame);
        }
        if matches!(
            request.source.kind,
            VideoSourceKind::Camera | VideoSourceKind::ScreenCapture
        ) {
            let mut frame = self
                .capture_inputs
                .lock()
                .map_err(|_| video::VideoDecodeError::Decode {
                    layer_id: request.layer_id,
                    label: request.label.clone(),
                    message: "Capture input frame registry lock was poisoned".to_string(),
                })?
                .get(&request.layer_id)
                .cloned();
            if let Some(frame) = &mut frame {
                frame.layer_id = request.layer_id;
                frame.pts_ms = request.position_ms;
            }
            return Ok(frame);
        }
        video::VideoFrameDecoder::decode_frame(&mut self.files, request)
    }
}

#[cfg(feature = "ndi")]
pub type NdiInputRegistry = Arc<Mutex<HashMap<u64, io::ndi::NdiInput>>>;

#[cfg(feature = "ndi")]
pub struct NdiTransportState {
    inputs: NdiInputRegistry,
    outputs: HashMap<u64, NdiOutputWorker>,
    capture_inputs: crate::capture_transport::CaptureInputRegistry,
}

#[cfg(feature = "ndi")]
impl NdiTransportState {
    pub fn new(
        inputs: NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Self {
        Self {
            inputs,
            outputs: HashMap::new(),
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
                let input =
                    io::ndi::NdiInput::new(route.endpoint_name.clone()).map_err(|error| {
                        video::ExternalVideoTransportDriverError {
                            message: error.to_string(),
                        }
                    })?;
                self.inputs
                    .lock()
                    .map_err(|_| video::ExternalVideoTransportDriverError {
                        message: "NDI input state lock was poisoned".to_string(),
                    })?
                    .insert(route.route_id, input);
            }
            video::ExternalVideoTransportDirection::Output => {
                let output = NdiOutputWorker::start(
                    route,
                    engine.clone(),
                    Arc::clone(&self.inputs),
                    Arc::clone(&self.capture_inputs),
                )?;
                self.outputs.insert(route.route_id, output);
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
                self.inputs
                    .lock()
                    .map_err(|_| video::ExternalVideoTransportDriverError {
                        message: "NDI input state lock was poisoned".to_string(),
                    })?
                    .remove(&route.route_id);
            }
            video::ExternalVideoTransportDirection::Output => {
                if let Some(output) = self.outputs.remove(&route.route_id) {
                    output.stop();
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "ndi")]
struct NdiOutputWorker {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "ndi")]
impl NdiOutputWorker {
    fn start(
        route: &video::ExternalVideoTransportRoute,
        engine: EngineHandle,
        inputs: NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let output_id = route.route_id;
        let endpoint_name = route.endpoint_name.clone();
        let sender = io::ndi::NdiOutput::new(endpoint_name.clone()).map_err(|error| {
            video::ExternalVideoTransportDriverError {
                message: error.to_string(),
            }
        })?;
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let worker = std::thread::Builder::new()
            .name(format!("syndocal-ndi-output-{output_id}"))
            .spawn(move || {
                let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
                    video::VideoRuntimeConfig::default(),
                    video::DecoderBackedFrameProvider::new(
                        NdiAwareVideoFrameDecoder::from_env()
                            .with_ndi_inputs(inputs)
                            .with_capture_inputs(capture_inputs),
                    )
                    .with_prefetch(0, 33),
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
                                .send_rgba(&io::ndi::NdiRgbaFrame {
                                    width: frame.width,
                                    height: frame.height,
                                    frame_rate_n: 60,
                                    frame_rate_d: 1,
                                    rgba: frame.data,
                                })
                                .map_err(|error| error.to_string())
                        });
                    if let Err(error) = result {
                        eprintln!("NDI output '{endpoint_name}' stopped: {error}");
                        break;
                    }
                    if let Some(remaining) = target_interval.checked_sub(started.elapsed()) {
                        std::thread::sleep(remaining);
                    }
                }
            })
            .map_err(|error| video::ExternalVideoTransportDriverError {
                message: format!("failed to start NDI output worker: {error}"),
            })?;
        Ok(Self {
            stop,
            worker: Some(worker),
        })
    }

    fn stop(mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(feature = "ndi")]
impl Drop for NdiOutputWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

#[cfg(test)]
mod capture_decoder_tests {
    use super::*;
    use video::VideoFrameDecoder;

    #[test]
    fn capture_registry_frame_is_exposed_at_requested_playhead() {
        let inputs = Arc::new(Mutex::new(HashMap::from([(
            8,
            video::VideoFrame {
                layer_id: 0,
                width: 2,
                height: 1,
                pts_ms: 0,
                duration_ms: 33,
                format: video::VideoPixelFormat::Rgba8,
                data: vec![1, 2, 3, 255, 4, 5, 6, 255],
            },
        )])));
        let mut decoder = NdiAwareVideoFrameDecoder::from_env().with_capture_inputs(inputs);
        for kind in [
            protocol::VideoSourceKind::Camera,
            protocol::VideoSourceKind::ScreenCapture,
        ] {
            let frame = decoder
                .decode_frame(&video::VideoFrameRequest {
                    layer_id: 8,
                    label: "Live Capture".to_string(),
                    source: protocol::VideoSourceSummary {
                        kind,
                        path: None,
                        name: Some("source".to_string()),
                        codec: None,
                        metadata: None,
                    },
                    position_ms: 2_345,
                    width: 2,
                    height: 1,
                })
                .unwrap()
                .unwrap();
            assert_eq!(frame.layer_id, 8);
            assert_eq!(frame.pts_ms, 2_345);
            assert_eq!(frame.duration_ms, 33);
            assert_eq!(frame.data, vec![1, 2, 3, 255, 4, 5, 6, 255]);
        }
    }
}

#[cfg(all(test, feature = "spout", target_os = "windows", target_arch = "x86_64"))]
mod spout_decoder_tests {
    use super::*;
    use video::VideoFrameDecoder;

    #[test]
    fn spout_registry_frame_is_exposed_at_requested_playhead() {
        let inputs = Arc::new(Mutex::new(HashMap::from([(
            7,
            video::VideoFrame {
                layer_id: 99,
                width: 2,
                height: 1,
                pts_ms: 0,
                duration_ms: 0,
                format: video::VideoPixelFormat::Bgra8,
                data: vec![1, 2, 3, 255, 4, 5, 6, 255],
            },
        )])));
        let mut decoder = NdiAwareVideoFrameDecoder::from_env().with_spout_inputs(inputs);
        let frame = decoder
            .decode_frame(&video::VideoFrameRequest {
                layer_id: 7,
                label: "Spout Camera".to_string(),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::Spout,
                    path: None,
                    name: Some("Camera A".to_string()),
                    codec: None,
                    metadata: None,
                },
                position_ms: 1_234,
                width: 2,
                height: 1,
            })
            .unwrap()
            .unwrap();

        assert_eq!(frame.layer_id, 7);
        assert_eq!(frame.pts_ms, 1_234);
        assert_eq!(frame.format, video::VideoPixelFormat::Bgra8);
        assert_eq!(frame.data, vec![1, 2, 3, 255, 4, 5, 6, 255]);
    }
}
