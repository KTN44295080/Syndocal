#[cfg(feature = "ndi")]
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[cfg(feature = "ndi")]
use engine::EngineHandle;
use protocol::VideoLayerId;
#[cfg(feature = "ndi")]
use protocol::VideoSourceKind;

pub struct NdiAwareVideoFrameDecoder {
    files: video::PreferredVideoFrameDecoder,
    #[cfg(feature = "ndi")]
    ndi_inputs: NdiInputRegistry,
    #[cfg(feature = "ndi")]
    ndi_frames: HashMap<u64, video::VideoFrame>,
}

impl NdiAwareVideoFrameDecoder {
    #[cfg(any(not(feature = "ndi"), test))]
    pub fn from_env() -> Self {
        Self {
            files: video::PreferredVideoFrameDecoder::from_env(),
            #[cfg(feature = "ndi")]
            ndi_inputs: Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            ndi_frames: HashMap::new(),
        }
    }

    #[cfg(feature = "ndi")]
    pub fn with_ndi_inputs(ndi_inputs: NdiInputRegistry) -> Self {
        Self {
            files: video::PreferredVideoFrameDecoder::from_env(),
            ndi_inputs,
            ndi_frames: HashMap::new(),
        }
    }

    pub fn diagnostics(&self) -> video::VideoDecoderDiagnostics {
        self.files.diagnostics()
    }

    pub fn cache_len(&self) -> usize {
        let file_frames = self.files.cache_len();
        #[cfg(feature = "ndi")]
        {
            return file_frames.saturating_add(self.ndi_frames.len());
        }
        #[cfg(not(feature = "ndi"))]
        file_frames
    }
}

impl video::VideoFrameDecoder for NdiAwareVideoFrameDecoder {
    fn retain_layers(&mut self, layer_ids: &[VideoLayerId]) {
        video::VideoFrameDecoder::retain_layers(&mut self.files, layer_ids);
        #[cfg(feature = "ndi")]
        self.ndi_frames
            .retain(|layer_id, _| layer_ids.contains(layer_id));
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
        video::VideoFrameDecoder::decode_frame(&mut self.files, request)
    }
}

#[cfg(feature = "ndi")]
pub type NdiInputRegistry = Arc<Mutex<HashMap<u64, io::ndi::NdiInput>>>;

#[cfg(feature = "ndi")]
pub struct NdiTransportState {
    inputs: NdiInputRegistry,
    outputs: HashMap<u64, NdiOutputWorker>,
}

#[cfg(feature = "ndi")]
impl NdiTransportState {
    pub fn new(inputs: NdiInputRegistry) -> Self {
        Self {
            inputs,
            outputs: HashMap::new(),
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
                let output =
                    NdiOutputWorker::start(route, engine.clone(), Arc::clone(&self.inputs))?;
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
                        NdiAwareVideoFrameDecoder::with_ndi_inputs(inputs),
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
