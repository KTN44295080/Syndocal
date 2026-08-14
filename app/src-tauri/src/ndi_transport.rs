use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(feature = "ndi")]
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

#[cfg(feature = "ndi")]
use engine::{
    EngineHandle, OutputOwnershipActivation, OutputOwnershipCreationLease,
    OutputOwnershipTeardownLease,
};
use protocol::{VideoLayerId, VideoSourceKind};

#[cfg(all(test, feature = "ndi"))]
use std::sync::atomic::AtomicUsize;

#[cfg(all(test, feature = "ndi"))]
static NDI_WORKER_SPAWN_FAILURE: AtomicBool = AtomicBool::new(false);

#[cfg(all(test, feature = "ndi"))]
static NDI_OUTPUT_CONSTRUCTION_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(any(feature = "ndi", test))]
fn ndi_output_effect_render_context(
    snapshot: &protocol::EngineSnapshot,
    project_render_epoch: u64,
) -> video::VideoEffectRenderContext<'_> {
    video::VideoEffectRenderContext {
        clip_runtime: &snapshot.video_clip_runtime,
        project_render_epoch,
    }
}

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
    pending_output_startups: HashMap<u64, NdiPendingOutputStartup>,
    pending_output_teardowns: HashMap<u64, io::ndi::NdiOutputTeardown>,
    failed_output_routes: HashMap<u64, String>,
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
            pending_output_startups: HashMap::new(),
            pending_output_teardowns: HashMap::new(),
            failed_output_routes: HashMap::new(),
            capture_inputs,
        }
    }

    fn harvest_failed_workers(
        &mut self,
        engine: &EngineHandle,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        let pending_ids = self
            .pending_output_startups
            .keys()
            .copied()
            .collect::<Vec<_>>();
        for route_id in pending_ids {
            let Some(pending) = self.pending_output_startups.get_mut(&route_id) else {
                continue;
            };
            if pending.poll() {
                let message = pending.message.clone();
                self.pending_output_startups.remove(&route_id);
                self.failed_output_routes.insert(route_id, message);
            } else {
                self.failed_output_routes
                    .insert(route_id, pending.message.clone());
            }
        }
        let failed_ids = self
            .outputs
            .iter()
            .filter_map(|(route_id, worker)| worker.failure_snapshot().map(|_| *route_id))
            .collect::<Vec<_>>();
        for route_id in failed_ids {
            let Some(worker) = self.outputs.remove(&route_id) else {
                continue;
            };
            let failure = worker.failure_snapshot();
            let stop_result = worker.stop(engine);
            let failure = failure.or_else(|| stop_result.as_ref().err().cloned());
            if let Some(failure) = failure {
                if let Some(teardown) = failure.pending_teardown {
                    self.pending_output_teardowns.insert(route_id, teardown);
                }
                self.failed_output_routes.insert(route_id, failure.message);
            }
        }
        Ok(())
    }

    pub fn start_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        activation: Option<OutputOwnershipActivation>,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.harvest_failed_workers(engine)?;
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
                if let Some(pending) = self.pending_output_startups.get(&route.route_id) {
                    return Err(video::ExternalVideoTransportDriverError {
                        message: format!(
                            "NDI output route {} is still awaiting startup cleanup acknowledgement: {}",
                            route.route_id, pending.message
                        ),
                    });
                }
                if let Some(teardown) = self.pending_output_teardowns.get(&route.route_id) {
                    if teardown.is_complete() {
                        if let Err(error) = teardown.wait(Duration::ZERO) {
                            return Err(video::ExternalVideoTransportDriverError {
                                message: format!(
                                    "NDI output route {} teardown acknowledgement failed: {error}",
                                    route.route_id
                                ),
                            });
                        }
                        self.pending_output_teardowns.remove(&route.route_id);
                        self.failed_output_routes.remove(&route.route_id);
                    } else {
                        return Err(video::ExternalVideoTransportDriverError {
                            message: format!(
                                "NDI output route {} is still awaiting teardown acknowledgement",
                                route.route_id
                            ),
                        });
                    }
                }
                self.failed_output_routes.remove(&route.route_id);
                let output = match NdiOutputWorker::start(
                    route,
                    engine.clone(),
                    Arc::clone(&self.inputs),
                    Arc::clone(&self.capture_inputs),
                    activation.ok_or_else(|| video::ExternalVideoTransportDriverError {
                        message: "NDI output resource activation was not admitted".to_string(),
                    })?,
                ) {
                    Ok(output) => output,
                    Err(error) => {
                        if let Some(pending) = error.pending_startup {
                            self.pending_output_startups.insert(route.route_id, pending);
                        }
                        return Err(video::ExternalVideoTransportDriverError {
                            message: error.message,
                        });
                    }
                };
                self.outputs.insert(route.route_id, output);
                let publish_result = self
                    .outputs
                    .get_mut(&route.route_id)
                    .expect("just-inserted NDI output worker")
                    .publish();
                if let Err(_error) = publish_result {
                    let output = self
                        .outputs
                        .remove(&route.route_id)
                        .expect("just-inserted NDI output worker");
                    if let Err(stop_error) = output.stop(engine) {
                        if let Some(teardown) = stop_error.pending_teardown {
                            self.pending_output_teardowns
                                .insert(route.route_id, teardown);
                        }
                        self.failed_output_routes
                            .insert(route.route_id, stop_error.message.clone());
                        return Err(video::ExternalVideoTransportDriverError {
                            message: format!(
                                "NDI output resource creation was invalidated and teardown failed: {}",
                                stop_error.message
                            ),
                        });
                    }
                    return Err(video::ExternalVideoTransportDriverError {
                        message: "NDI output resource creation was invalidated before publication"
                            .to_string(),
                    });
                }
            }
        }
        Ok(())
    }

    pub fn stop_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.harvest_failed_workers(engine)?;
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
                let status = engine.output_ownership_status();
                if !matches!(
                    status.state,
                    protocol::OutputOwnershipState::Transitioning
                        | protocol::OutputOwnershipState::Activating
                        | protocol::OutputOwnershipState::Failed
                ) {
                    return Err(video::ExternalVideoTransportDriverError {
                        message: "NDI output teardown requires the serialized ownership transition"
                            .to_string(),
                    });
                }
                if let Some(teardown) = self.pending_output_teardowns.get(&route.route_id).cloned()
                {
                    let result = teardown.wait(Duration::from_secs(3));
                    if teardown.is_complete() && result.is_ok() {
                        self.pending_output_teardowns.remove(&route.route_id);
                        self.failed_output_routes.remove(&route.route_id);
                    }
                    return result.map_err(|error| video::ExternalVideoTransportDriverError {
                        message: error.to_string(),
                    });
                }
                if let Some(output) = self.outputs.remove(&route.route_id) {
                    match output.stop(engine) {
                        Ok(()) => {
                            self.failed_output_routes.remove(&route.route_id);
                        }
                        Err(error) => {
                            if let Some(teardown) = error.pending_teardown {
                                self.pending_output_teardowns
                                    .insert(route.route_id, teardown);
                            }
                            self.failed_output_routes
                                .insert(route.route_id, error.message.clone());
                            return Err(video::ExternalVideoTransportDriverError {
                                message: error.message,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[cfg(feature = "ndi")]
#[derive(Debug, Clone)]
struct NdiOutputWorkerStopError {
    message: String,
    pending_teardown: Option<io::ndi::NdiOutputTeardown>,
    resource_teardown_pending: bool,
}

#[cfg(feature = "ndi")]
struct NdiOutputWorkerStartError {
    message: String,
    pending_startup: Option<NdiPendingOutputStartup>,
}

#[cfg(feature = "ndi")]
impl std::fmt::Debug for NdiOutputWorkerStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NdiOutputWorkerStartError")
            .field("message", &self.message)
            .field("has_pending_startup", &self.pending_startup.is_some())
            .finish()
    }
}

#[cfg(feature = "ndi")]
struct NdiPendingOutputStartup {
    worker: Option<std::thread::JoinHandle<Result<(), NdiOutputWorkerStopError>>>,
    failure_lease: Option<OutputOwnershipTeardownLease>,
    teardown: Option<io::ndi::NdiOutputTeardown>,
    resource_teardown_pending: bool,
    message: String,
}

#[cfg(feature = "ndi")]
impl NdiPendingOutputStartup {
    fn poll(&mut self) -> bool {
        if let Some(worker) = self.worker.as_ref() {
            if !worker.is_finished() {
                return false;
            }
        }
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => {
                    self.message = error.message.clone();
                    self.resource_teardown_pending |= error.resource_teardown_pending;
                    if let Some(teardown) = error.pending_teardown {
                        self.teardown = Some(teardown);
                        self.resource_teardown_pending = true;
                    }
                }
                Err(_) => {
                    self.message = "NDI output startup worker panicked".to_string();
                    self.resource_teardown_pending = true;
                }
            }
        }
        let Some(teardown) = self.teardown.as_ref() else {
            if self.resource_teardown_pending {
                return false;
            }
            self.failure_lease.take();
            return true;
        };
        if !teardown.is_complete() {
            return false;
        }
        if let Err(error) = teardown.wait(Duration::ZERO) {
            self.message = format!("{}; NDI startup teardown failed: {error}", self.message);
            return false;
        }
        self.teardown.take();
        self.failure_lease.take();
        true
    }
}

#[cfg(feature = "ndi")]
impl Drop for NdiPendingOutputStartup {
    fn drop(&mut self) {
        // A pending startup is normally polled by the owning transport state.
        // If that state itself is dropped while the constructor is still
        // blocked, keep the worker and every ownership token in an explicit
        // reaper until the SDK resource is acknowledged as destroyed.
        let payload = Arc::new(Mutex::new(Some((
            self.worker.take(),
            self.failure_lease.take(),
            self.teardown.take(),
        ))));
        let worker_payload = Arc::clone(&payload);
        let reaper = std::thread::Builder::new()
            .name("syndocal-ndi-startup-reaper".to_string())
            .spawn(move || {
                let Some((worker, failure_lease, teardown)) = worker_payload
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take()
                else {
                    return;
                };
                let (worker_teardown, worker_cleanup_pending) = worker
                    .map(|worker| match worker.join() {
                        Ok(Err(error)) => (error.pending_teardown, error.resource_teardown_pending),
                        _ => (None, false),
                    })
                    .unwrap_or((None, false));
                let teardown = teardown.or(worker_teardown);
                if let Some(teardown) = teardown {
                    while !teardown.is_complete() {
                        let _ = teardown.wait(Duration::from_millis(100));
                    }
                    let _ = teardown.wait(Duration::ZERO);
                } else if worker_cleanup_pending {
                    // There is no safe acknowledgement to wait for. Keep the
                    // failure lease live rather than allowing a replacement
                    // source to overlap an unaccounted SDK resource.
                    std::mem::forget(failure_lease);
                    return;
                }
                drop(failure_lease);
            });
        if reaper.is_err() {
            if let Some((worker, failure_lease, teardown)) = payload
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .take()
            {
                if let Some(worker) = worker {
                    std::mem::forget(worker);
                }
                if let Some(failure_lease) = failure_lease {
                    std::mem::forget(failure_lease);
                }
                if let Some(teardown) = teardown {
                    std::mem::forget(teardown);
                }
            }
        }
    }
}

#[cfg(feature = "ndi")]
type NdiCreationLeaseSlot = Arc<Mutex<Option<OutputOwnershipCreationLease>>>;

#[cfg(feature = "ndi")]
fn take_ndi_creation_lease(slot: &NdiCreationLeaseSlot) -> Option<OutputOwnershipCreationLease> {
    slot.lock()
        .map(|mut lease| lease.take())
        .unwrap_or_else(|poisoned| poisoned.into_inner().take())
}

#[cfg(feature = "ndi")]
struct NdiOutputWorker {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<(), NdiOutputWorkerStopError>>>,
    failure: Arc<Mutex<Option<NdiOutputWorkerStopError>>>,
    start_signal: Option<mpsc::SyncSender<NdiOutputStartDecision>>,
    creation_lease: Option<OutputOwnershipCreationLease>,
}

#[cfg(feature = "ndi")]
enum NdiOutputStartDecision {
    Publish,
    Retire(OutputOwnershipCreationLease),
}

#[cfg(feature = "ndi")]
fn spawn_ndi_output_worker<F>(
    output_id: u64,
    worker: F,
) -> Result<std::thread::JoinHandle<Result<(), NdiOutputWorkerStopError>>, String>
where
    F: FnOnce() -> Result<(), NdiOutputWorkerStopError> + Send + 'static,
{
    #[cfg(all(test, feature = "ndi"))]
    if NDI_WORKER_SPAWN_FAILURE.swap(false, Ordering::AcqRel) {
        return Err("injected NDI output worker spawn failure".to_string());
    }
    std::thread::Builder::new()
        .name(format!("syndocal-ndi-output-{output_id}"))
        .spawn(worker)
        .map_err(|error| error.to_string())
}

#[cfg(feature = "ndi")]
fn retire_ndi_output_after_creation<L>(
    sender: io::ndi::NdiOutput,
    lease: L,
) -> Result<(), NdiOutputWorkerStopError>
where
    L: Send + 'static,
{
    let teardown =
        sender
            .begin_close_with_lease(lease)
            .map_err(|error| NdiOutputWorkerStopError {
                message: format!("NDI sender teardown could not start: {error}"),
                pending_teardown: None,
                resource_teardown_pending: true,
            })?;
    teardown
        .wait(Duration::from_secs(3))
        .map_err(|error| NdiOutputWorkerStopError {
            message: format!("NDI sender teardown pending: {error}"),
            pending_teardown: Some(teardown),
            resource_teardown_pending: true,
        })
}

#[cfg(feature = "ndi")]
impl NdiOutputWorker {
    fn start(
        route: &video::ExternalVideoTransportRoute,
        engine: EngineHandle,
        inputs: NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
        activation: OutputOwnershipActivation,
    ) -> Result<Self, NdiOutputWorkerStartError> {
        let output_id = route.route_id;
        let endpoint_name = route.endpoint_name.clone();
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let failure = Arc::new(Mutex::new(None));
        let worker_failure = Arc::clone(&failure);
        let (startup_sender, startup_receiver) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_sender, start_receiver) = mpsc::sync_channel::<NdiOutputStartDecision>(1);
        let creation_lease_slot: NdiCreationLeaseSlot = Arc::new(Mutex::new(None));
        let worker_creation_lease_slot = Arc::clone(&creation_lease_slot);
        let parent_engine = engine.clone();
        let worker = spawn_ndi_output_worker(output_id, move || {
            let creation_lease = match activation.admit_resource_creation() {
                Ok(lease) => lease,
                Err(error) => {
                    let _ = startup_sender.send(Err(error.clone()));
                    return Err(NdiOutputWorkerStopError {
                        message: error,
                        pending_teardown: None,
                        resource_teardown_pending: false,
                    });
                }
            };
            // The SDK source is constructed only after the linearizable gate
            // admission. A fence that wins before this point therefore never
            // reaches the SDK constructor.
            #[cfg(all(test, feature = "ndi"))]
            NDI_OUTPUT_CONSTRUCTION_ATTEMPTS.fetch_add(1, Ordering::AcqRel);
            let sender = match io::ndi::NdiOutput::new(endpoint_name.clone()) {
                Ok(sender) => sender,
                Err(error) => {
                    let message = error.to_string();
                    creation_lease.retire();
                    let _ = startup_sender.send(Err(message.clone()));
                    return Err(NdiOutputWorkerStopError {
                        message,
                        pending_teardown: None,
                        resource_teardown_pending: false,
                    });
                }
            };
            match worker_creation_lease_slot.lock() {
                Ok(mut slot) => *slot = Some(creation_lease),
                Err(poisoned) => *poisoned.into_inner() = Some(creation_lease),
            }
            if startup_sender.send(Ok(())).is_err() {
                let Some(lease) = take_ndi_creation_lease(&worker_creation_lease_slot) else {
                    return Err(NdiOutputWorkerStopError {
                        message: "NDI output startup lease was lost after readiness disconnect"
                            .to_string(),
                        pending_teardown: None,
                        resource_teardown_pending: true,
                    });
                };
                return retire_ndi_output_after_creation(sender, lease);
            }
            match start_receiver.recv() {
                Ok(NdiOutputStartDecision::Publish) => {}
                Ok(NdiOutputStartDecision::Retire(lease)) => {
                    return retire_ndi_output_after_creation(sender, lease);
                }
                Err(_) => {
                    if let Some(lease) = take_ndi_creation_lease(&worker_creation_lease_slot) {
                        return retire_ndi_output_after_creation(sender, lease);
                    }
                    let error =
                        "NDI output resource publication acknowledgement was dropped".to_string();
                    let failure_lease = engine.begin_output_ownership_failure_fence(error.clone());
                    return retire_ndi_output_after_creation(sender, failure_lease).map_err(
                        |mut stop_error| {
                            stop_error.message = format!("{error}; {}", stop_error.message);
                            stop_error
                        },
                    );
                }
            }
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
            let mut worker_error = None;
            let mut failure_lease = None;
            while !worker_stop.load(Ordering::Acquire) {
                let started = Instant::now();
                let project_render_epoch = engine.output_ownership_status().epoch;
                let snapshot = engine.snapshot();
                renderer
                    .frame_provider_mut()
                    .set_bpm(Some(snapshot.clock.bpm));
                let result = renderer
                    .render_output_with_effects_and_transitions(
                        &snapshot.video,
                        ndi_output_effect_render_context(&snapshot, project_render_epoch),
                        &snapshot.video_transition_runtime,
                        output_id,
                    )
                    .map_err(|error| format!("{error:?}"))
                    .and_then(|frame| {
                        let _permit = loop {
                            match engine.acquire_video_output() {
                                Ok(permit) => break permit,
                                Err(_) if worker_stop.load(Ordering::Acquire) => {
                                    return Err("NDI output worker is stopping".to_string());
                                }
                                Err(_) => {
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                            }
                        };
                        let send_result = sender
                            .send_rgba(&io::ndi::NdiRgbaFrame {
                                width: frame.width,
                                height: frame.height,
                                frame_rate_n: 60,
                                frame_rate_d: 1,
                                rgba: frame.data,
                            })
                            .map_err(|error| error.to_string());
                        if let Err(error) = &send_result {
                            // The frame permit is still held here, so
                            // the failure fence closes the admission
                            // window atomically with the failed physical
                            // send before sender teardown begins.
                            failure_lease = Some(engine.begin_output_ownership_failure_fence(
                                format!("NDI output route {output_id} send failed: {error}"),
                            ));
                        }
                        send_result
                    });
                if let Err(error) = result {
                    if !worker_stop.load(Ordering::Acquire) {
                        worker_error = Some(error.clone());
                    }
                    eprintln!("NDI output '{endpoint_name}' stopped: {error}");
                    break;
                }
                if let Some(remaining) = target_interval.checked_sub(started.elapsed()) {
                    std::thread::sleep(remaining);
                }
            }
            let teardown_lease = if let Some(lease) = failure_lease.take() {
                lease
            } else if let Some(worker_error) = worker_error.as_ref() {
                engine.begin_output_ownership_failure_fence(format!(
                    "NDI output route {output_id} failed before teardown: {worker_error}"
                ))
            } else {
                engine.begin_output_ownership_teardown().map_err(|error| {
                    NdiOutputWorkerStopError {
                        message: format!("NDI output teardown was not admitted: {error}"),
                        pending_teardown: None,
                        resource_teardown_pending: true,
                    }
                })?
            };
            let teardown = sender
                .begin_close_with_lease(teardown_lease)
                .map_err(|error| NdiOutputWorkerStopError {
                    message: format!("NDI sender teardown could not start: {error}"),
                    pending_teardown: None,
                    resource_teardown_pending: true,
                })?;
            let result = match teardown.wait(Duration::from_secs(3)) {
                Ok(()) => match worker_error {
                    Some(worker_error) => Err(NdiOutputWorkerStopError {
                        message: worker_error,
                        pending_teardown: None,
                        resource_teardown_pending: false,
                    }),
                    None => Ok(()),
                },
                Err(teardown_error) => Err(NdiOutputWorkerStopError {
                    message: match worker_error {
                        Some(worker_error) => {
                            format!("{worker_error}; NDI sender teardown pending: {teardown_error}")
                        }
                        None => format!("NDI sender teardown pending: {teardown_error}"),
                    },
                    pending_teardown: Some(teardown.clone()),
                    resource_teardown_pending: true,
                }),
            };
            if let Err(error) = &result {
                engine.mark_output_ownership_transition_failure(error.message.clone());
                if let Ok(mut slot) = worker_failure.lock() {
                    *slot = Some(error.clone());
                } else {
                    let mut slot = worker_failure
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    *slot = Some(error.clone());
                }
            }
            result
        })
        .map_err(|error| NdiOutputWorkerStartError {
            message: format!("failed to start NDI output worker: {error}"),
            pending_startup: None,
        })?;

        let creation_lease = match startup_receiver.recv_timeout(Duration::from_secs(3)) {
            Ok(Ok(())) => match take_ndi_creation_lease(&creation_lease_slot) {
                Some(lease) => lease,
                None => {
                    stop.store(true, Ordering::Release);
                    drop(startup_receiver);
                    drop(start_sender);
                    return Err(NdiOutputWorkerStartError {
                        message:
                            "NDI output startup acknowledgement did not retain its creation lease"
                                .to_string(),
                        pending_startup: None,
                    });
                }
            },
            Ok(Err(message)) => {
                stop.store(true, Ordering::Release);
                drop(startup_receiver);
                drop(start_sender);
                let failure_lease = parent_engine.begin_output_ownership_failure_fence(format!(
                    "NDI output startup failed: {message}"
                ));
                return Err(NdiOutputWorkerStartError {
                    message,
                    pending_startup: Some(NdiPendingOutputStartup {
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown: None,
                        resource_teardown_pending: false,
                        message: "NDI output startup cleanup is pending".to_string(),
                    }),
                });
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                drop(startup_receiver);
                drop(start_sender);
                let message = format!("NDI output worker startup acknowledgement failed: {error}");
                let failure_lease = parent_engine.begin_output_ownership_failure_fence(&message);
                return Err(NdiOutputWorkerStartError {
                    message,
                    pending_startup: Some(NdiPendingOutputStartup {
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown: None,
                        resource_teardown_pending: false,
                        message: "NDI output startup cleanup is pending".to_string(),
                    }),
                });
            }
        };
        Ok(Self {
            stop,
            worker: Some(worker),
            failure,
            start_signal: Some(start_sender),
            creation_lease: Some(creation_lease),
        })
    }

    fn publish(&mut self) -> Result<(), String> {
        let lease = self
            .creation_lease
            .take()
            .ok_or_else(|| "NDI output resource was already published".to_string())?;
        match lease.publish() {
            Ok(()) => {
                let Some(start_signal) = self.start_signal.take() else {
                    return Err(
                        "NDI output resource publication signal was already consumed".to_string(),
                    );
                };
                start_signal
                    .send(NdiOutputStartDecision::Publish)
                    .map_err(|_| {
                        "NDI output worker stopped before resource publication".to_string()
                    })
            }
            Err(lease) => {
                self.creation_lease = Some(lease);
                Err("NDI output resource creation fence was invalidated".to_string())
            }
        }
    }

    fn failure_snapshot(&self) -> Option<NdiOutputWorkerStopError> {
        self.failure
            .lock()
            .map(|failure| failure.clone())
            .unwrap_or_else(|poisoned| (*poisoned.into_inner()).clone())
    }

    fn signal_retirement(&mut self) -> Option<OutputOwnershipCreationLease> {
        let lease = self.creation_lease.take()?;
        let Some(start_signal) = self.start_signal.take() else {
            return Some(lease);
        };
        match start_signal.send(NdiOutputStartDecision::Retire(lease)) {
            Ok(()) => None,
            Err(error) => match error.0 {
                NdiOutputStartDecision::Retire(lease) => Some(lease),
                NdiOutputStartDecision::Publish => None,
            },
        }
    }

    fn stop(mut self, _engine: &EngineHandle) -> Result<(), NdiOutputWorkerStopError> {
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let result = worker.join().map_err(|_| NdiOutputWorkerStopError {
                message: "NDI output worker panicked while stopping".to_string(),
                pending_teardown: None,
                resource_teardown_pending: true,
            })?;
            result?;
        }
        drop(undelivered_creation_lease);
        Ok(())
    }
}

#[cfg(feature = "ndi")]
impl Drop for NdiOutputWorker {
    fn drop(&mut self) {
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        drop(undelivered_creation_lease);
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

    #[test]
    fn ndi_output_effect_context_uses_runtime_clip_truth_and_ownership_epoch() {
        let mut snapshot = protocol::EngineSnapshot::default();
        snapshot
            .video_clip_runtime
            .layers
            .push(protocol::VideoClipLayerRuntimeSummary {
                layer_id: 9,
                active_slot_id: Some(protocol::VideoClipSlotId(12)),
                ..Default::default()
            });

        let context = ndi_output_effect_render_context(&snapshot, 73);

        assert_eq!(context.project_render_epoch, 73);
        assert_eq!(context.clip_runtime, &snapshot.video_clip_runtime);
        assert_eq!(context.clip_runtime.layers[0].layer_id, 9);
        assert_eq!(
            context.clip_runtime.layers[0].active_slot_id,
            Some(protocol::VideoClipSlotId(12))
        );
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

#[cfg(all(test, feature = "ndi"))]
mod ndi_output_worker_tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    };

    #[test]
    fn injected_worker_spawn_failure_constructs_no_sender() {
        NDI_OUTPUT_CONSTRUCTION_ATTEMPTS.store(0, Ordering::Release);
        NDI_WORKER_SPAWN_FAILURE.store(true, Ordering::Release);
        let route = video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: 17,
            label: "Injected NDI".to_string(),
            backend_id: "ndi".to_string(),
            endpoint_name: "Injected NDI Sender".to_string(),
        };
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let inputs = Arc::new(Mutex::new(HashMap::new()));
        let capture_inputs = Arc::new(Mutex::new(HashMap::new()));

        let error = match NdiOutputWorker::start(&route, engine, inputs, capture_inputs, activation)
        {
            Ok(_) => panic!("injected worker spawn failure must reject the route"),
            Err(error) => error,
        };
        assert!(error
            .message
            .contains("injected NDI output worker spawn failure"));
        assert_eq!(
            NDI_OUTPUT_CONSTRUCTION_ATTEMPTS.load(Ordering::Acquire),
            0,
            "sender creation must remain inside the worker and never run after spawn failure"
        );
    }

    #[test]
    fn failure_fence_before_ndi_creation_admission_never_constructs_sender() {
        NDI_OUTPUT_CONSTRUCTION_ATTEMPTS.store(0, Ordering::Release);
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        engine
            .validate_output_activation(&activation, protocol::MachineOutputRole::Both)
            .unwrap();
        let failure = engine
            .begin_output_ownership_failure_fence("injected fence before NDI creation admission");
        let route = video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: 18,
            label: "Fenced NDI".to_string(),
            backend_id: "ndi".to_string(),
            endpoint_name: "Fenced NDI Sender".to_string(),
        };

        let error = match NdiOutputWorker::start(
            &route,
            engine,
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
        ) {
            Ok(_) => panic!("fenced NDI creation must reject the route"),
            Err(error) => error,
        };

        assert!(error.message.contains("creation"));
        assert_eq!(
            NDI_OUTPUT_CONSTRUCTION_ATTEMPTS.load(Ordering::Acquire),
            0,
            "NDI constructor must not run after the failure fence wins admission"
        );
        drop(failure);
    }

    #[test]
    fn startup_timeout_is_bounded_and_delayed_constructor_retires_without_duplicate_creation() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let (allow_constructor_tx, allow_constructor_rx) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_tx, start_rx) = mpsc::sync_channel::<()>(1);
        let constructor_attempts = Arc::new(AtomicUsize::new(0));
        let resource_destroyed = Arc::new(AtomicBool::new(false));
        let worker_slot: NdiCreationLeaseSlot = Arc::new(Mutex::new(None));
        let worker_slot_for_thread = Arc::clone(&worker_slot);
        let attempts_for_thread = Arc::clone(&constructor_attempts);
        let destroyed_for_thread = Arc::clone(&resource_destroyed);
        let worker = std::thread::spawn(move || {
            attempts_for_thread.fetch_add(1, Ordering::AcqRel);
            allow_constructor_rx.recv().unwrap();
            match worker_slot_for_thread.lock() {
                Ok(mut slot) => *slot = Some(creation_lease),
                Err(poisoned) => *poisoned.into_inner() = Some(creation_lease),
            }
            if ready_tx.send(Ok(())).is_err() || start_rx.recv().is_err() {
                let lease = take_ndi_creation_lease(&worker_slot_for_thread)
                    .expect("cancelled startup must retain its creation lease");
                destroyed_for_thread.store(true, Ordering::Release);
                lease.retire();
            }
            Ok::<(), NdiOutputWorkerStopError>(())
        });

        assert!(matches!(
            ready_rx.recv_timeout(Duration::from_millis(1)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(ready_rx);
        drop(start_tx);
        let failure_lease = engine.begin_output_ownership_failure_fence(
            "NDI startup constructor timed out; cleanup remains pending",
        );
        let pending = NdiPendingOutputStartup {
            worker: Some(worker),
            failure_lease: Some(failure_lease),
            teardown: None,
            resource_teardown_pending: false,
            message: "NDI startup constructor is still running".to_string(),
        };
        let mut transport = NdiTransportState::new(
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        );
        transport.pending_output_startups.insert(77, pending);
        let route = video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: 77,
            label: "Delayed NDI".to_string(),
            backend_id: "ndi".to_string(),
            endpoint_name: "Delayed NDI".to_string(),
        };

        let blocked = transport.start_route(&route, &engine, Some(activation));
        assert!(blocked
            .as_ref()
            .is_err_and(|error| error.message.contains("startup cleanup")));
        assert_eq!(constructor_attempts.load(Ordering::Acquire), 1);
        assert!(engine.acquire_video_output().is_err());

        allow_constructor_tx.send(()).unwrap();
        while !transport.pending_output_startups.is_empty() {
            transport.harvest_failed_workers(&engine).unwrap();
            std::thread::yield_now();
        }
        assert!(resource_destroyed.load(Ordering::Acquire));
        assert_eq!(constructor_attempts.load(Ordering::Acquire), 1);

        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
        assert!(engine.acquire_video_output().is_ok());
    }

    #[test]
    fn startup_handshake_disconnect_retires_worker_owned_creation_lease() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_tx, start_rx) = mpsc::sync_channel::<()>(1);
        let worker_slot: NdiCreationLeaseSlot = Arc::new(Mutex::new(None));
        let worker_slot_for_thread = Arc::clone(&worker_slot);
        let worker = std::thread::spawn(move || {
            match worker_slot_for_thread.lock() {
                Ok(mut slot) => *slot = Some(creation_lease),
                Err(poisoned) => *poisoned.into_inner() = Some(creation_lease),
            }
            ready_tx.send(Ok(())).unwrap();
            if start_rx.recv().is_err() {
                let lease = take_ndi_creation_lease(&worker_slot_for_thread)
                    .expect("handshake disconnect must return the worker-owned lease");
                lease.retire();
            }
            Ok::<(), NdiOutputWorkerStopError>(())
        });

        ready_rx.recv().unwrap().unwrap();
        drop(ready_rx);
        drop(start_tx);
        let failure_lease = engine.begin_output_ownership_failure_fence(
            "NDI publication handshake disconnected during startup",
        );
        let mut pending = NdiPendingOutputStartup {
            worker: Some(worker),
            failure_lease: Some(failure_lease),
            teardown: None,
            resource_teardown_pending: false,
            message: "NDI publication handshake is pending".to_string(),
        };
        while !pending.poll() {
            std::thread::yield_now();
        }
        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
    }
}
