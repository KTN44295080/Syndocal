use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[cfg(any(feature = "ndi", test))]
use std::time::{Duration, Instant};

#[cfg(feature = "ndi")]
use std::sync::{
    atomic::{AtomicBool, Ordering},
    mpsc,
};

#[cfg(any(feature = "ndi", test))]
use engine::EngineHandle;
#[cfg(feature = "ndi")]
use engine::{
    OutputOwnershipActivation, OutputOwnershipCreationLease, OutputOwnershipTeardownLease,
};
#[cfg(test)]
use engine::{TimelineFollowVideoRenderSnapshot, VideoPresentationSample};
#[cfg(test)]
use protocol::OutputOwnershipState;
#[cfg(any(feature = "ndi", test))]
use protocol::TimelineFollowSettlementAckResult;
#[cfg(test)]
use protocol::VideoOutputId;
#[cfg(test)]
use protocol::{
    TimelineFollowSettlementAck, TimelineFollowSettlementConsumerId,
    TimelineFollowSettlementDomain, VideoEffectScope, VideoTransitionEffectOwner,
};
use protocol::{VideoLayerId, VideoSourceKind};

#[cfg(all(test, feature = "ndi"))]
use std::sync::atomic::AtomicUsize;

#[cfg(all(test, feature = "ndi"))]
static NDI_WORKER_SPAWN_FAILURE: AtomicBool = AtomicBool::new(false);

#[cfg(all(test, feature = "ndi"))]
static NDI_OUTPUT_CONSTRUCTION_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

/// Serializes every test that touches the process-global worker-spawn hook
/// statics; the Spout suite guards the identical pattern with its own lock.
#[cfg(all(test, feature = "ndi"))]
static NDI_TEST_HOOK_LOCK: Mutex<()> = Mutex::new(());

// One shared FC-28 authority fence, bounded-settlement watchdog, and Follow
// render/settlement source for both physical transports. Both parents include
// the same file without a main.rs declaration; backend differences enter only
// as parameters. See the module docs in physical_output_fence.rs.
#[cfg(any(feature = "ndi", test))]
#[path = "physical_output_fence.rs"]
mod physical_output_fence;
#[cfg(any(feature = "ndi", test))]
use physical_output_fence::*;

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
        let startup_failure_lease: StartupFailureLeaseSlot =
            Arc::new(Mutex::new(StartupFailureFenceState::default()));
        let worker_startup_failure_lease = Arc::clone(&startup_failure_lease);
        let parent_engine = engine.clone();
        let worker = spawn_ndi_output_worker(output_id, move || {
            let mut follow_output_state = TimelineFollowOutputState::default();
            let creation_lease = match activation.admit_resource_creation() {
                Ok(lease) => lease,
                Err(error) => {
                    if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                        if let Ok(key) = timeline_follow_output_key(&follow, output_id) {
                            publish_timeline_follow_output_result_until_resolved(
                                &engine,
                                &mut follow_output_state,
                                key,
                                TimelineFollowSettlementAckResult::NotApplicable,
                                || worker_stop.load(Ordering::Acquire),
                            );
                        }
                    }
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
                    let fault = format!("NDI output route {output_id} open failed: {message}");
                    // Creation is still admitted here. Fence it before an ACK
                    // retry can wait so no competing output can enter.
                    if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                        match timeline_follow_output_key(&follow, output_id) {
                            Ok(key) => {
                                let failed_key = fence_timeline_follow_fault_before_ack_in_startup(
                                    &engine,
                                    &mut follow_output_state,
                                    key,
                                    fault.clone(),
                                    &worker_startup_failure_lease,
                                );
                                publish_timeline_follow_output_result_until_resolved(
                                    &engine,
                                    &mut follow_output_state,
                                    failed_key,
                                    TimelineFollowSettlementAckResult::Fault {
                                        fault: fault.clone(),
                                    },
                                    || worker_stop.load(Ordering::Acquire),
                                );
                            }
                            Err(key_error) => {
                                let _ = ensure_startup_failure_fence(
                                    &engine,
                                    &worker_startup_failure_lease,
                                    format!("{fault}; {key_error}"),
                                );
                            }
                        }
                    } else {
                        ensure_startup_failure_fence(
                            &engine,
                            &worker_startup_failure_lease,
                            fault.clone(),
                        );
                    }
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
                let presentation_sample = engine.video_presentation_sample();
                let snapshot = &presentation_sample.snapshot;
                let ownership = engine.output_ownership_status();
                let blackout_authority = engine.safety_blackout_authority();
                renderer
                    .frame_provider_mut()
                    .set_bpm(Some(snapshot.clock.bpm));
                let follow = engine.timeline_follow_video_render_snapshot();
                let rendered = match capture_output_presentation_authority(
                    "NDI",
                    protocol::VideoOutputKind::NdiSender,
                    &presentation_sample,
                    ownership,
                    blackout_authority,
                    output_id,
                    &endpoint_name,
                )
                .and_then(|authority| {
                    revalidate_output_presentation_authority(&engine, "NDI", &authority)?;
                    Ok(authority)
                }) {
                    Ok(authority) => match follow.as_ref() {
                        Some(follow) if follow.epoch != authority.ownership_epoch() => {
                            let retry_key = timeline_follow_output_key(follow, output_id).ok();
                            Ok((
                                TimelineFollowOutputRenderDecision::Retry {
                                    reason: "NDI Timeline Follow authority epoch changed before render"
                                        .to_string(),
                                    key: retry_key,
                                },
                                None,
                            ))
                        }
                        Some(follow) if authority.project_blackout() => {
                            // A current safety blackout is a canonical output
                            // result, not a property of either historical
                            // Follow snapshot. Do not inspect or render them.
                            render_timeline_follow_hard_blackout_output(follow, &authority)
                                .map(|decision| (decision, Some(authority)))
                        }
                        Some(follow) => match validate_follow_output_authority("NDI", follow, &authority)
                        {
                            Ok(()) => render_timeline_follow_output(
                                &mut renderer,
                                snapshot,
                                follow,
                                output_id,
                                &mut follow_output_state,
                            )
                            .map(|decision| (decision, Some(authority))),
                            Err(reason) => {
                                let retry_key =
                                    timeline_follow_output_key(follow, output_id).ok();
                                Ok((
                                    TimelineFollowOutputRenderDecision::Retry {
                                        reason,
                                        key: retry_key,
                                    },
                                    None,
                                ))
                            }
                        },
                        None => renderer
                            .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                                &snapshot.video,
                                ndi_output_effect_render_context(
                                    snapshot,
                                    authority.ownership_epoch(),
                                ),
                                &snapshot.video_transition_runtime,
                                output_id,
                                authority.output_summary().width,
                                authority.output_summary().height,
                            )
                            .map_err(|error| {
                                format!("NDI output {output_id} artistic render failed: {error:?}")
                            })
                            .and_then(|result| {
                                materialize_output_artistic_result(result, &authority)
                            })
                            .map(|frame| {
                                (
                                    TimelineFollowOutputRenderDecision::Frame {
                                        key: TimelineFollowOutputFrameKey {
                                            epoch: authority.ownership_epoch(),
                                            generation: 0,
                                            output_id,
                                            width: frame.width,
                                            height: frame.height,
                                        },
                                        frame,
                                        result: TimelineFollowSettlementAckResult::Applied,
                                        follow_identity: None,
                                    },
                                    Some(authority),
                                )
                            }),
                    },
                    Err(error) => match follow.as_ref() {
                        Some(follow) => {
                            let retry_key = timeline_follow_output_key(follow, output_id).ok();
                            Ok((
                                TimelineFollowOutputRenderDecision::Retry {
                                    reason: error,
                                    key: retry_key,
                                },
                                None,
                            ))
                        }
                        None => Err(error),
                    },
                };
                let result = match rendered {
                    Ok((TimelineFollowOutputRenderDecision::NotApplicable { key, reason }, _)) => {
                        // This transport cannot prove an atomic engine video
                        // quorum, so it may not turn a locally observed
                        // NotApplicable sample into a terminal Follow ACK.
                        // Discard it and reacquire on the next iteration, but
                        // only within the bounded-settlement deadline: a hold
                        // that never admits must fault visibly.
                        if follow_output_state.tick_follow_unresolved(
                            key,
                            Instant::now(),
                            &reason,
                        ) {
                            let (_, failed_key, fault) = fence_timeline_follow_settlement_deadline(
                                &engine,
                                &mut follow_output_state,
                                key,
                                &reason,
                            );
                            publish_timeline_follow_output_result_until_resolved(
                                &engine,
                                &mut follow_output_state,
                                failed_key,
                                TimelineFollowSettlementAckResult::Fault { fault: fault.clone() },
                                || worker_stop.load(Ordering::Acquire),
                            );
                            Err(fault)
                        } else {
                            Ok(())
                        }
                    }
                    Ok((TimelineFollowOutputRenderDecision::Retry { reason, key }, _)) => {
                        // A route/ownership/Follow generation changed while
                        // sampling. It is neither a physical-send fault nor a
                        // NotApplicable acknowledgement, but the same
                        // generation may not stay unadmitted forever.
                        let mut outcome = Ok(());
                        if let Some(key) = key {
                            if follow_output_state.tick_follow_unresolved(
                                key,
                                Instant::now(),
                                &reason,
                            ) {
                                let (_, failed_key, fault) =
                                    fence_timeline_follow_settlement_deadline(
                                        &engine,
                                        &mut follow_output_state,
                                        key,
                                        &reason,
                                    );
                                publish_timeline_follow_output_result_until_resolved(
                                    &engine,
                                    &mut follow_output_state,
                                    failed_key,
                                    TimelineFollowSettlementAckResult::Fault {
                                        fault: fault.clone(),
                                    },
                                    || worker_stop.load(Ordering::Acquire),
                                );
                                outcome = Err(fault);
                            }
                        }
                        outcome
                    }
                    Ok((
                        TimelineFollowOutputRenderDecision::Frame {
                            key,
                            frame,
                            result: follow_result,
                            follow_identity,
                        },
                        Some(authority),
                    )) => {
                        let admission_started = Instant::now();
                        let permit_outcome = loop {
                            match engine.acquire_video_output() {
                                Ok(permit) => break Ok(permit),
                                Err(_) if worker_stop.load(Ordering::Acquire) => {
                                    break Err(NdiOutputWorkerStopError {
                                        message: "NDI output worker is stopping".to_string(),
                                        pending_teardown: None,
                                        resource_teardown_pending: false,
                                    });
                                }
                                Err(_) if admission_started.elapsed()
                                    >= VIDEO_OUTPUT_ADMISSION_DEADLINE =>
                                {
                                    break Err(NdiOutputWorkerStopError {
                                        message: format!(
                                            "NDI output route {output_id} video ownership admission deadline ({VIDEO_OUTPUT_ADMISSION_DEADLINE:?}) exceeded"
                                        ),
                                        pending_teardown: None,
                                        resource_teardown_pending: false,
                                    });
                                }
                                Err(_) => {
                                    // The ownership gate did not admit this
                                    // frame. Without an engine-issued atomic
                                    // quorum proof, do not acknowledge it.
                                    // The bounded wait above turns a stuck
                                    // gate into a visible fault instead of an
                                    // indefinite silent hold.
                                    std::thread::sleep(Duration::from_millis(5));
                                }
                            }
                        };
                        match permit_outcome {
                            Ok(_permit) => {
                                let send_result = send_frame_if_authorized(
                                    "NDI",
                                    &engine,
                                    &authority,
                                    follow_identity,
                                    || {
                                        sender
                                            .send_rgba(&io::ndi::NdiRgbaFrame {
                                                width: frame.width,
                                                height: frame.height,
                                                frame_rate_n: 60,
                                                frame_rate_d: 1,
                                                rgba: frame.data,
                                            })
                                            .map_err(|error| error.to_string())
                                    },
                                );
                                match send_result {
                            Ok(()) => {
                                if follow_identity.is_some() {
                                    // The settled physical outcome for this
                                    // generation was just achieved; clear its
                                    // unresolved clock.
                                    follow_output_state.note_follow_admitted();
                                    let settlement = timeline_follow_result_after_physical_send(
                                        follow_result,
                                        None,
                                        "NDI",
                                        output_id,
                                    );
                                    if matches!(
                                        &settlement,
                                        TimelineFollowSettlementAckResult::Applied
                                    ) {
                                        // The physical send above succeeded.
                                        // Keep its exact Applied ACK retryable
                                        // through terminal presenter
                                        // retirement/reply loss.
                                        publish_timeline_follow_output_result_until_resolved(
                                            &engine,
                                            &mut follow_output_state,
                                            key,
                                            settlement,
                                            || worker_stop.load(Ordering::Acquire),
                                        );
                                    } else {
                                        let _ = publish_timeline_follow_output_result(
                                            &engine,
                                            &mut follow_output_state,
                                            key,
                                            settlement,
                                        );
                                    }
                                }
                                Ok(())
                            }
                            Err(PhysicalOutputSendError::Revoked(reason)) => {
                                // The physical closure was not entered. A
                                // fresh loop captures current authority (and,
                                // for blackout, produces canonical hard
                                // black) without a stale ACK or fault fence —
                                // but the same generation may not be revoked
                                // on every attempt forever.
                                let mut outcome = Ok(());
                                if follow_identity.is_some()
                                    && follow_output_state.tick_follow_unresolved(
                                        key,
                                        Instant::now(),
                                        &reason,
                                    )
                                {
                                    let (lease, failed_key, fault) =
                                        fence_timeline_follow_settlement_deadline(
                                            &engine,
                                            &mut follow_output_state,
                                            key,
                                            &reason,
                                        );
                                    failure_lease = Some(lease);
                                    publish_timeline_follow_output_result_until_resolved(
                                        &engine,
                                        &mut follow_output_state,
                                        failed_key,
                                        TimelineFollowSettlementAckResult::Fault {
                                            fault: fault.clone(),
                                        },
                                        || worker_stop.load(Ordering::Acquire),
                                    );
                                    outcome = Err(fault);
                                }
                                outcome
                            }
                            Err(PhysicalOutputSendError::Sdk(error)) => {
                                // Hold the physical frame permit while
                                // fencing; a Follow Fault acknowledgement may
                                // retry below.
                                let failed_follow_key = if follow_identity.is_some() {
                                    let (lease, failed_key) =
                                        fence_timeline_follow_fault_before_ack(
                                            &engine,
                                            &mut follow_output_state,
                                            key,
                                            format!(
                                                "NDI output route {output_id} send failed: {error}"
                                            ),
                                        );
                                    failure_lease = Some(lease);
                                    Some(failed_key)
                                } else {
                                    failure_lease = Some(
                                        engine.begin_output_ownership_failure_fence(format!(
                                            "NDI output route {output_id} send failed: {error}"
                                        )),
                                    );
                                    None
                                };
                                if follow_identity.is_some() {
                                    let settlement = timeline_follow_result_after_physical_send(
                                        follow_result,
                                        Some(error.as_str()),
                                        "NDI",
                                        output_id,
                                    );
                                    publish_timeline_follow_output_result_until_resolved(
                                        &engine,
                                        &mut follow_output_state,
                                        failed_follow_key.unwrap_or(key),
                                        settlement,
                                        || worker_stop.load(Ordering::Acquire),
                                    );
                                }
                                Err(error)
                            }
                                }
                            }
                            Err(admission_error) => {
                                // A lost-ownership wait is still a settlement
                                // path: a pending Follow generation must
                                // resolve visibly, and the frame itself must
                                // never be silently dropped. Keep the failure
                                // lease alive through shared teardown.
                                if follow_identity.is_some() {
                                    let (lease, failed_key, fault) =
                                        fence_timeline_follow_settlement_deadline(
                                            &engine,
                                            &mut follow_output_state,
                                            key,
                                            &admission_error.message,
                                        );
                                    failure_lease = Some(lease);
                                    publish_timeline_follow_output_result_until_resolved(
                                        &engine,
                                        &mut follow_output_state,
                                        failed_key,
                                        TimelineFollowSettlementAckResult::Fault {
                                            fault: fault.clone(),
                                        },
                                        || worker_stop.load(Ordering::Acquire),
                                    );
                                    Err(fault)
                                } else {
                                    Err(admission_error.message)
                                }
                            }
                        }
                    }
                    Ok((TimelineFollowOutputRenderDecision::Frame { .. }, None)) => Err(
                        "NDI output frame was produced without an authority contract".to_string(),
                    ),
                    Err(error) => {
                        if let Some(follow) = follow.as_ref() {
                            let fault =
                                format!("NDI output route {output_id} render failed: {error}");
                            match timeline_follow_output_key(follow, output_id) {
                                Ok(key) => {
                                    let (lease, failed_key) = fence_timeline_follow_fault_before_ack(
                                        &engine,
                                        &mut follow_output_state,
                                        key,
                                        fault.clone(),
                                    );
                                    failure_lease = Some(lease);
                                    publish_timeline_follow_output_result_until_resolved(
                                        &engine,
                                        &mut follow_output_state,
                                        failed_key,
                                        TimelineFollowSettlementAckResult::Fault { fault },
                                        || worker_stop.load(Ordering::Acquire),
                                    );
                                }
                                Err(key_error) => {
                                    failure_lease = Some(engine.begin_output_ownership_failure_fence(
                                        format!("{fault}; {key_error}"),
                                    ));
                                }
                            }
                        } else if !worker_stop.load(Ordering::Acquire) && failure_lease.is_none() {
                            failure_lease = Some(engine.begin_output_ownership_failure_fence(
                                format!("NDI output route {output_id} render failed: {error}"),
                            ));
                        }
                        Err(error)
                    }
                };
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
                match engine.begin_output_ownership_teardown() {
                    Ok(lease) => lease,
                    Err(error) => {
                        let message = format!("NDI output teardown was not admitted: {error}");
                        worker_error = Some(message.clone());
                        engine.begin_output_ownership_failure_fence(message)
                    }
                }
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
                let failure_lease = take_startup_failure_lease(&startup_failure_lease)
                    .or_else(|| {
                        ensure_startup_failure_fence(
                            &parent_engine,
                            &startup_failure_lease,
                            format!("NDI output startup failed: {message}"),
                        );
                        take_startup_failure_lease(&startup_failure_lease)
                    })
                    .expect("NDI startup failure fence must retain its lease");
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
                ensure_startup_failure_fence(&parent_engine, &startup_failure_lease, &message);
                let failure_lease = take_startup_failure_lease(&startup_failure_lease)
                    .expect("startup timeout fence must retain its lease");
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

    fn follow_video_snapshot(
        output_id: VideoOutputId,
        width: u32,
        height: u32,
        enabled: bool,
    ) -> protocol::VideoSnapshot {
        protocol::VideoSnapshot {
            compositions: vec![protocol::CompositionSummary {
                id: 1,
                label: "Follow".to_string(),
                layer_ids: Vec::new(),
                timeline_layer_ids: Vec::new(),
                output_ids: vec![output_id],
            }],
            outputs: vec![protocol::VideoOutputSummary {
                id: output_id,
                label: "Program".to_string(),
                kind: protocol::VideoOutputKind::Display,
                enabled,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width,
                height,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: protocol::VideoOutputMapping::default(),
            }],
            ..protocol::VideoSnapshot::default()
        }
    }

    fn follow_render_snapshot(
        kind: protocol::VideoClipTakeKind,
    ) -> TimelineFollowVideoRenderSnapshot {
        TimelineFollowVideoRenderSnapshot {
            epoch: 9,
            generation: 12,
            source_timeline_id: protocol::TimelineId(41),
            target_timeline_id: protocol::TimelineId(42),
            kind,
            curve: protocol::VideoLayerTransitionCurve::Linear,
            progress_millis: 500,
            outgoing_video: follow_video_snapshot(7, 4, 2, true),
            incoming_video: follow_video_snapshot(7, 4, 2, true),
            transition_effect_chain: None,
        }
    }

    fn invalid_follow_transition_chain(
        source_timeline_id: protocol::TimelineId,
    ) -> protocol::VideoEffectChainSummary {
        protocol::VideoEffectChainSummary {
            id: protocol::VideoEffectChainId(70),
            scope: VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::TimelineFollow { source_timeline_id },
            },
            bypassed: false,
            stages: vec![protocol::VideoEffectStageSummary {
                id: protocol::VideoEffectStageId(71),
                enabled: true,
                label: "Broken".to_string(),
                effect: protocol::VideoEffectSummary {
                    id: protocol::VideoEffectId(72),
                    kind: protocol::VideoEffectKind::Isf {
                        effect: protocol::VideoIsfEffectSummary {
                            enabled: true,
                            label: "Broken".to_string(),
                            source: Arc::from("this is not valid ISF source"),
                            source_path: None,
                            description: None,
                            categories: Vec::new(),
                            controls: Vec::new(),
                            stack: Vec::new(),
                        },
                    },
                },
            }],
        }
    }

    fn ndi_test_endpoint(output_id: VideoOutputId) -> String {
        format!("Injected NDI {output_id}")
    }

    fn engine_with_ndi_output_extent(
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) -> EngineHandle {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        transition.complete().unwrap();
        let mut output = follow_video_snapshot(output_id, width, height, true)
            .outputs
            .into_iter()
            .next()
            .unwrap();
        output.kind = protocol::VideoOutputKind::NdiSender;
        output.endpoint_name = Some(ndi_test_endpoint(output_id));
        engine
            .send(engine::EngineCommand::AddVideoOutput(output))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !engine
            .snapshot()
            .video
            .outputs
            .iter()
            .any(|output| output.id == output_id)
        {
            assert!(
                Instant::now() < deadline,
                "NDI test output {output_id} did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        engine
    }

    fn engine_with_ndi_output(output_id: VideoOutputId) -> EngineHandle {
        engine_with_ndi_output_extent(output_id, 2, 1)
    }

    fn capture_ndi_test_authority(
        engine: &EngineHandle,
        output_id: VideoOutputId,
    ) -> OutputPresentationAuthority {
        let sample = engine.video_presentation_sample();
        capture_output_presentation_authority(
            "NDI",
            protocol::VideoOutputKind::NdiSender,
            &sample,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &ndi_test_endpoint(output_id),
        )
        .expect("NDI test authority must be present and owned")
    }

    fn assert_ndi_authority_rejects_without_send(
        engine: &EngineHandle,
        authority: &OutputPresentationAuthority,
        expected_error: &str,
    ) {
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sends_for_attempt = Arc::clone(&sends);
        let error = send_frame_if_authorized("NDI", engine, authority, None, move || {
            sends_for_attempt.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(())
        })
        .expect_err("changed authority must reject before the NDI SDK send");
        assert!(
            matches!(&error, PhysicalOutputSendError::Revoked(_)),
            "an authority mismatch is a typed revoke, never an SDK fault or success: {error}"
        );
        assert!(error.to_string().contains(expected_error), "{error}");
        assert_eq!(
            sends.load(std::sync::atomic::Ordering::Acquire),
            0,
            "a rejected NDI authority must not enter the physical send closure"
        );
    }

    fn ndi_hard_blackout_artistic_result(
        authority: &OutputPresentationAuthority,
        freshness: video::VideoOutputRenderFreshness,
    ) -> video::VideoOutputArtisticRenderResult {
        video::VideoOutputArtisticRenderResult {
            payload: video::VideoOutputArtisticPayload::HardBlackout,
            output_mapping: authority.output_summary().mapping.clone(),
            output_mapping_identity: video::VideoOutputMappingIdentity::from_mapping(
                &authority.output_summary().mapping,
            ),
            evidence: video::VideoOutputRenderEvidence {
                project_render_epoch: authority.ownership_epoch(),
                output_id: authority.output_summary().id,
                freshness,
                error: None,
            },
        }
    }

    fn bind_follow_to_ndi_route(
        follow: &mut TimelineFollowVideoRenderSnapshot,
        output_id: VideoOutputId,
    ) {
        for video in [&mut follow.outgoing_video, &mut follow.incoming_video] {
            let output = video
                .outputs
                .iter_mut()
                .find(|output| output.id == output_id)
                .expect("historical NDI Follow output exists");
            output.kind = protocol::VideoOutputKind::NdiSender;
            output.endpoint_name = Some(ndi_test_endpoint(output_id));
        }
    }

    #[test]
    fn ndi_route_identity_mismatch_rejects_before_any_send() {
        let output_id = 98_099;
        let engine = engine_with_ndi_output(output_id);
        let sample = engine.video_presentation_sample();
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let error = capture_output_presentation_authority(
            "NDI",
            protocol::VideoOutputKind::NdiSender,
            &sample,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            "different NDI sender",
        )
        .expect_err("endpoint mismatch must reject the bound NDI route");
        assert!(error.contains("endpoint does not match"), "{error}");
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 0);

        let mut wrong_kind = VideoPresentationSample {
            config_token: sample.config_token,
            snapshot: sample.snapshot.clone(),
        };
        wrong_kind
            .snapshot
            .video
            .outputs
            .iter_mut()
            .find(|output| output.id == output_id)
            .expect("NDI test output exists")
            .kind = protocol::VideoOutputKind::Display;
        assert!(capture_output_presentation_authority(
            "NDI",
            protocol::VideoOutputKind::NdiSender,
            &wrong_kind,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &ndi_test_endpoint(output_id),
        )
        .expect_err("wrong output kind must reject the bound NDI route")
        .contains("not NdiSender"));
    }

    #[test]
    fn ndi_current_blackout_overrides_visible_historical_follow_snapshots() {
        let output_id = 7;
        let engine = engine_with_ndi_output_extent(output_id, 4, 2);
        let mut follow = follow_render_snapshot(protocol::VideoClipTakeKind::Custom);
        bind_follow_to_ndi_route(&mut follow, output_id);
        assert!(!follow.outgoing_video.blackout);
        assert!(!follow.incoming_video.blackout);
        assert!(!follow.outgoing_video.outputs[0].blackout);
        assert!(!follow.incoming_video.outputs[0].blackout);

        engine
            .send(engine::EngineCommand::SetAllBlackout(true))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !engine.snapshot().blackout {
            assert!(
                Instant::now() < deadline,
                "NDI current project blackout did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let authority = capture_ndi_test_authority(&engine, output_id);
        assert!(authority.project_blackout());
        follow.epoch = authority.ownership_epoch();
        // Safety blackout deliberately does not depend on either historical
        // route: an old visible Follow snapshot may already be structurally
        // obsolete, yet it must still yield current opaque black.
        follow.outgoing_video.outputs[0].kind = protocol::VideoOutputKind::Display;
        follow.outgoing_video.outputs[0].endpoint_name = None;
        let TimelineFollowOutputRenderDecision::Frame {
            frame,
            follow_identity,
            ..
        } = render_timeline_follow_hard_blackout_output(&follow, &authority)
            .expect("current blackout must bypass historical Follow rendering")
        else {
            panic!("current blackout must produce a physical Follow frame");
        };
        assert_eq!(
            follow_identity,
            Some(timeline_follow_active_identity(&follow))
        );
        assert_eq!(frame.data, vec![0, 0, 0, 255].repeat(8));
    }

    #[test]
    fn ndi_follow_retirement_or_generation_rollover_rejects_before_any_send() {
        let mut follow = follow_render_snapshot(protocol::VideoClipTakeKind::Crossfade);
        let output_id = follow.outgoing_video.outputs[0].id;
        let engine = engine_with_ndi_output_extent(output_id, 4, 2);
        let authority = capture_ndi_test_authority(&engine, output_id);
        bind_follow_to_ndi_route(&mut follow, output_id);
        follow.epoch = authority.ownership_epoch();
        let identity = timeline_follow_active_identity(&follow);
        let mut rollover = follow.clone();
        rollover.generation = rollover.generation.saturating_add(1);
        assert!(
            validate_timeline_follow_active_identity("NDI", identity, Some(&rollover))
                .expect_err("Follow generation rollover must reject the old render")
                .contains("identity changed")
        );

        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sends_for_attempt = Arc::clone(&sends);
        let error =
            send_frame_if_authorized("NDI", &engine, &authority, Some(identity), move || {
                sends_for_attempt.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                Ok(())
            })
            .expect_err("retired Follow must reject before entering the NDI SDK closure");
        assert!(
            matches!(&error, PhysicalOutputSendError::Revoked(_)),
            "a pre-send Follow retirement is a discard/re-render, not an SDK fault: {error}"
        );
        assert!(error.to_string().contains("was retired"), "{error}");
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 0);
    }

    #[test]
    fn ndi_transport_rejects_nonfresh_or_zero_area_and_materializes_hard_blackout_exactly() {
        let output_id = 98_100;
        let engine = engine_with_ndi_output(output_id);
        let mut authority = capture_ndi_test_authority(&engine, output_id);
        let mut overridden_mapping = authority.output_summary().mapping.clone();
        overridden_mapping.black_level = 0.9;
        authority.override_output_mapping_for_test(overridden_mapping);
        let frame = materialize_output_artistic_result(
            ndi_hard_blackout_artistic_result(&authority, video::VideoOutputRenderFreshness::Fresh),
            &authority,
        )
        .expect("hard blackout must reach the NDI boundary as an admitted frame");
        assert_eq!(frame.data, vec![0, 0, 0, 255, 0, 0, 0, 255]);

        for freshness in [
            video::VideoOutputRenderFreshness::Error,
            video::VideoOutputRenderFreshness::LastValid,
        ] {
            let error = materialize_output_artistic_result(
                ndi_hard_blackout_artistic_result(&authority, freshness),
                &authority,
            )
            .expect_err("non-fresh render evidence must not create NDI send bytes");
            assert!(error.contains("NotFresh"), "{error}");
        }

        let mut zero_area = VideoPresentationSample {
            config_token: 1,
            snapshot: protocol::EngineSnapshot::default(),
        };
        zero_area
            .snapshot
            .video
            .outputs
            .push(protocol::VideoOutputSummary {
                id: output_id,
                label: "Zero Area".to_string(),
                kind: protocol::VideoOutputKind::NdiSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 0,
                height: 2,
                endpoint_name: Some(ndi_test_endpoint(output_id)),
                opacity: 1.0,
                blackout: false,
                mapping: protocol::VideoOutputMapping::default(),
            });
        let error = capture_output_presentation_authority(
            "NDI",
            protocol::VideoOutputKind::NdiSender,
            &zero_area,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &ndi_test_endpoint(output_id),
        )
        .expect_err("zero-area NDI output must reject before render or send");
        assert!(error.contains("zero-area"), "{error}");
    }

    #[test]
    fn ndi_output_authority_races_fail_closed_before_any_send() {
        let output_id = 98_101;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        let mut mapping = authority.output_summary().mapping.clone();
        mapping.black_level = 0.25;
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping { output_id, mapping })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].mapping.black_level != 0.25 {
            assert!(
                Instant::now() < deadline,
                "NDI mapping mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "route, mapping, or enablement changed",
        );

        let authority = capture_ndi_test_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::SetVideoOutputEnabled {
                output_id,
                enabled: false,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].enabled {
            assert!(
                Instant::now() < deadline,
                "NDI disable mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "route, mapping, or enablement changed",
        );

        let output_id = 98_103;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::RemoveVideoOutput(output_id))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine
            .snapshot()
            .video
            .outputs
            .iter()
            .any(|output| output.id == output_id)
        {
            assert!(
                Instant::now() < deadline,
                "NDI output removal did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_ndi_authority_rejects_without_send(&engine, &authority, "was removed before send");

        let output_id = 98_104;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::SetAllBlackout(true))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !engine.snapshot().blackout {
            assert!(
                Instant::now() < deadline,
                "NDI safety blackout did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "project safety blackout changed",
        );

        let output_id = 98_102;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap();
        assert_ndi_authority_rejects_without_send(&engine, &authority, "authority changed");
        transition.complete().unwrap();
    }

    #[test]
    fn ndi_presentation_token_change_after_preparation_revokes_restored_content() {
        let output_id = 98_105;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        let captured_video = engine.snapshot().video;
        let original_mapping = authority.output_summary().mapping.clone();
        let mut mutated = original_mapping.clone();
        mutated.black_level = 0.25;
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping {
                output_id,
                mapping: mutated,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].mapping.black_level != 0.25 {
            assert!(
                Instant::now() < deadline,
                "NDI token-hostile mapping mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping {
                output_id,
                mapping: original_mapping,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].mapping.black_level
            != authority.output_summary().mapping.black_level
        {
            assert!(
                Instant::now() < deadline,
                "NDI token-hostile mapping revert did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // Every deep-equality observation is restored exactly: only the
        // engine-issued presentation token can see the intervening mutation.
        assert_eq!(engine.snapshot().video, captured_video);
        assert_eq!(
            engine.output_ownership_status(),
            authority.ownership_status(),
            "the ownership epoch/generation must be unchanged for this hostile case"
        );
        assert_eq!(
            engine.safety_blackout_authority(),
            authority.blackout_authority()
        );
        assert_ne!(
            engine.video_presentation_config_token(),
            authority.presentation_config_token(),
            "the presentation config token must have advanced across mutate/revert"
        );

        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "presentation authority token advanced before send",
        );
    }

    #[test]
    fn ndi_safety_blackout_latch_cycle_after_preparation_revokes_before_send() {
        let output_id = 98_106;
        let engine = engine_with_ndi_output_extent(output_id, 4, 2);
        let authority = capture_ndi_test_authority(&engine, output_id);
        assert!(!authority.project_blackout());

        let engaged = engine
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(30))
            .unwrap();
        assert_eq!(
            engaged,
            engine::SafetyBlackoutEngageDisposition::Applied,
            "a successful emergency engage must follow its publication"
        );
        let engaged_authority = engine.safety_blackout_authority();
        assert!(engaged_authority.engaged);
        let released = engine
            .safety_blackout_release_published(
                engaged_authority.epoch,
                engaged_authority.generation,
                Instant::now() + Duration::from_secs(30),
            )
            .unwrap();
        assert_eq!(released, engine::SafetyBlackoutReleaseDisposition::Applied);
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().blackout {
            assert!(
                Instant::now() < deadline,
                "NDI safety blackout latch release did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // The visible blackout bit returned to its captured value; only the
        // runtime safety-blackout authority (engaged/epoch/generation) still
        // proves the intervening emergency cycle.
        assert_eq!(engine.snapshot().blackout, authority.project_blackout());
        assert_ne!(
            engine.safety_blackout_authority(),
            authority.blackout_authority()
        );

        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "blackout authority changed before send",
        );
    }

    #[test]
    fn ndi_output_identity_substitution_after_preparation_revokes_before_send() {
        let output_id = 98_107;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::RemoveVideoOutput(output_id))
            .unwrap();
        let substituted_endpoint = format!("Substituted NDI {output_id}");
        let mut substituted = authority.output_summary().clone();
        substituted.endpoint_name = Some(substituted_endpoint.clone());
        engine
            .send(engine::EngineCommand::AddVideoOutput(substituted))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !engine.snapshot().video.outputs.iter().any(|output| {
            output.id == output_id
                && output.endpoint_name.as_deref() == Some(substituted_endpoint.as_str())
        }) {
            assert!(
                Instant::now() < deadline,
                "substituted NDI output identity did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // The output ID resolves again, so removal alone cannot fence it:
        // exact route-resource identity must.
        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "bound route resource identity changed",
        );
    }

    #[test]
    fn ndi_stable_authority_commits_exactly_one_sdk_send_per_admission() {
        let output_id = 98_108;
        let engine = engine_with_ndi_output(output_id);
        let authority = capture_ndi_test_authority(&engine, output_id);
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let first_commit = send_frame_if_authorized("NDI", &engine, &authority, None, || {
            sends.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(41_u8)
        })
        .expect("stable authority must commit the prepared frame");
        assert_eq!(first_commit, 41);
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 1);

        let second_commit = send_frame_if_authorized("NDI", &engine, &authority, None, || {
            sends.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(())
        })
        .expect("unchanged authority must authorize an independent second commit");
        assert_eq!(second_commit, ());
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 2);
        assert_eq!(
            engine.video_presentation_config_token(),
            authority.presentation_config_token(),
            "a stable authority must never advance or consume the presentation token"
        );
    }

    #[test]
    fn ndi_playing_timeline_ticks_admit_frames_but_token_mutations_revoke() {
        let output_id = 98_110;
        let engine = engine_with_ndi_output(output_id);
        let source = protocol::TimelineSnapshot {
            id: protocol::TimelineId(98_111),
            label: "NDI playing admission source".to_string(),
            layers: vec![protocol::TimelineLayerSummary {
                id: 2,
                label: "Audio".to_string(),
                order: 0,
                muted: false,
                locked: false,
                solo: false,
                expanded: false,
                kind: protocol::TimelineLayerKind::Audio,
            }],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 98_112,
                layer_id: 2,
                path: "memory://ndi-playing-admission.wav".to_string(),
                duration_ms: 60_000,
                ..protocol::TimelineAudioClipSummary::default()
            }],
            duration_ms: 60_000,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(vec![source], protocol::TimelineId(98_111), false)
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekTimeline(1_000))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetTimelinePlaying(true))
            .unwrap();
        let advanced_deadline = Instant::now() + Duration::from_secs(2);
        while engine.snapshot().timeline.position_ms <= 1_000 {
            assert!(
                Instant::now() < advanced_deadline,
                "the playing timeline never advanced past its seek point"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // Capture an authority mid-playback, then let the transport cadence
        // elapse. Natural per-tick position/transition progress must never
        // revoke the frame; only the engine-issued presentation token can.
        let authority = capture_ndi_test_authority(&engine, output_id);
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(
            engine.output_ownership_status(),
            authority.ownership_status(),
            "natural playback must not rotate output ownership"
        );
        assert_eq!(
            engine.safety_blackout_authority(),
            authority.blackout_authority()
        );
        assert_eq!(
            engine.video_presentation_config_token(),
            authority.presentation_config_token(),
            "per-tick playback must never advance the presentation token"
        );

        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sends_for_attempt = Arc::clone(&sends);
        send_frame_if_authorized("NDI", &engine, &authority, None, move || {
            sends_for_attempt.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(())
        })
        .expect("normal per-tick playback must admit the prepared frame instead of mass-revoking");
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 1);

        // A real presentation mutation that reverts its visible content must
        // still revoke through the advanced token: no stale physical send.
        let original_mapping = authority.output_summary().mapping.clone();
        let mut mutated = original_mapping.clone();
        mutated.black_level = 0.4;
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping {
                output_id,
                mapping: mutated,
            })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].mapping.black_level != 0.4 {
            assert!(
                Instant::now() < deadline,
                "NDI admission mapping mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping {
                output_id,
                mapping: original_mapping,
            })
            .unwrap();
        loop {
            assert!(
                Instant::now() < deadline,
                "NDI admission mapping revert did not publish"
            );
            if engine.snapshot().video.outputs[0].mapping.black_level
                == authority.output_summary().mapping.black_level
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_ndi_authority_rejects_without_send(
            &engine,
            &authority,
            "presentation authority token advanced before send",
        );
    }

    #[test]
    fn ndi_persistent_supersession_faults_at_bounded_settlement_deadline() {
        // One live settling Hold generation whose historical Follow route is
        // permanently superseded by a renamed current output. Every worker
        // iteration observes exactly this supersession, so the bounded
        // settlement watchdog must convert it into a visible Fault instead of
        // extending the hold indefinitely.
        let output_id = 98_112;
        let engine = engine_with_ndi_output_extent(output_id, 4, 2);
        let audio_layer = protocol::TimelineLayerSummary {
            id: 2,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: false,
            kind: protocol::TimelineLayerKind::Audio,
        };
        let mut source = protocol::TimelineSnapshot {
            id: protocol::TimelineId(98_113),
            label: "NDI deadline Follow source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 98_114,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://ndi-deadline-follow.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(98_115),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(98_115),
            label: "NDI deadline Follow target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(98_113),
                false,
            )
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekTimeline(90))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetTimelinePlaying(true))
            .unwrap();
        let settle_deadline = Instant::now() + Duration::from_secs(2);
        let follow = loop {
            if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                if matches!(
                    engine.timeline_follow_runtime_status(follow.epoch).status,
                    protocol::TimelineFollowRuntimeStatus::Settling
                ) {
                    break follow;
                }
            }
            assert!(
                Instant::now() < settle_deadline,
                "NDI deadline Follow never entered settlement"
            );
            std::thread::sleep(Duration::from_millis(10));
        };

        // Supersede the bound route for the rest of the hold.
        let mut superseded = follow_video_snapshot(output_id, 4, 2, true)
            .outputs
            .into_iter()
            .next()
            .unwrap();
        superseded.endpoint_name = Some("Superseded NDI deadline sender".to_string());
        engine
            .send(engine::EngineCommand::SetVideoOutputConfig {
                output_id,
                label: "Superseded NDI deadline".to_string(),
                kind: protocol::VideoOutputKind::NdiSender,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 4,
                height: 2,
                endpoint_name: Some("Superseded NDI deadline sender".to_string()),
            })
            .unwrap();
        let route_publication_deadline = Instant::now() + Duration::from_secs(1);
        while engine
            .snapshot()
            .video
            .outputs
            .iter()
            .find(|output| output.id == output_id)
            .and_then(|output| output.endpoint_name.as_deref())
            != Some("Superseded NDI deadline sender")
        {
            assert!(
                Instant::now() < route_publication_deadline,
                "the superseded NDI route identity did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // Reproduce the production render decision for every iteration: the
        // historical Follow route no longer matches current authority, so
        // each observation is a Retry carrying the same frame key.
        let retry_key = timeline_follow_output_key(&follow, output_id).unwrap();
        let superseded_sample = engine.video_presentation_sample();
        let superseded_authority = capture_output_presentation_authority(
            "NDI",
            protocol::VideoOutputKind::NdiSender,
            &superseded_sample,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            "Superseded NDI deadline sender",
        )
        .expect("the renamed route must still be capturable");
        let reason = validate_follow_output_authority("NDI", &follow, &superseded_authority)
            .expect_err("the superseded route must no longer match");
        assert!(reason.contains("does not match the current project authority"));

        let mut state = TimelineFollowOutputState::default();
        state.set_settlement_deadline(Duration::from_millis(80));
        assert!(!state.tick_follow_unresolved(retry_key, Instant::now(), &reason));
        std::thread::sleep(Duration::from_millis(90));
        assert!(
            state.tick_follow_unresolved(retry_key, Instant::now(), &reason),
            "continuous supersession beyond the deadline must expire visibly"
        );

        let (failure_lease, failed_key, fault) =
            fence_timeline_follow_settlement_deadline(&engine, &mut state, retry_key, &reason);
        publish_timeline_follow_output_result_until_resolved(
            &engine,
            &mut state,
            failed_key,
            TimelineFollowSettlementAckResult::Fault { fault },
            || false,
        );
        assert!(state.published_for(failed_key));
        let runtime = engine.snapshot().timeline.follow_runtime;
        assert_eq!(runtime.status, protocol::TimelineFollowRuntimeStatus::Held);
        assert!(matches!(
            runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Held)
        ));
        drop(failure_lease);
    }

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

    #[test]
    fn follow_output_renders_every_kind_fresh_and_rejects_wrong_custom_owner() {
        let engine_snapshot = protocol::EngineSnapshot::default();
        for kind in [
            protocol::VideoClipTakeKind::Cut,
            protocol::VideoClipTakeKind::Crossfade,
            protocol::VideoClipTakeKind::Dip,
            protocol::VideoClipTakeKind::Wipe,
            protocol::VideoClipTakeKind::Custom,
        ] {
            let mut renderer =
                video::VideoPreviewRenderer::new(video::VideoRuntimeConfig::default());
            let mut state = TimelineFollowOutputState::default();
            let follow = follow_render_snapshot(kind);
            let decision = render_timeline_follow_output(
                &mut renderer,
                &engine_snapshot,
                &follow,
                7,
                &mut state,
            )
            .expect("empty compositions render a real transparent program frame");
            let TimelineFollowOutputRenderDecision::Frame {
                key, frame, result, ..
            } = decision
            else {
                panic!("enabled Follow output must render");
            };
            assert_eq!(
                (key.epoch, key.generation, key.width, key.height),
                (9, 12, 4, 2)
            );
            assert_eq!((frame.width, frame.height, frame.data.len()), (4, 2, 32));
            assert_eq!(result, TimelineFollowSettlementAckResult::Applied);
        }

        let mut renderer = video::VideoPreviewRenderer::new(video::VideoRuntimeConfig::default());
        let mut state = TimelineFollowOutputState::default();
        let mut wrong_owner = follow_render_snapshot(protocol::VideoClipTakeKind::Custom);
        wrong_owner.transition_effect_chain =
            Some(invalid_follow_transition_chain(protocol::TimelineId(999)));
        let error = render_timeline_follow_output(
            &mut renderer,
            &engine_snapshot,
            &wrong_owner,
            7,
            &mut state,
        )
        .expect_err("Custom chain owner must be exact");
        assert!(error.contains("wrong owner"), "{error}");
    }

    #[test]
    fn follow_output_last_valid_is_size_and_generation_fenced_and_fault_is_not_applied() {
        let engine_snapshot = protocol::EngineSnapshot::default();
        let mut renderer = video::VideoPreviewRenderer::new(video::VideoRuntimeConfig::default());
        let mut state = TimelineFollowOutputState::default();
        let mut follow = follow_render_snapshot(protocol::VideoClipTakeKind::Custom);
        let first =
            render_timeline_follow_output(&mut renderer, &engine_snapshot, &follow, 7, &mut state)
                .unwrap();
        let TimelineFollowOutputRenderDecision::Frame {
            key, frame, result, ..
        } = first
        else {
            panic!("enabled Follow output must render");
        };
        assert_eq!(result, TimelineFollowSettlementAckResult::Applied);
        assert_eq!(state.last_valid(key), Some(&frame));

        follow.transition_effect_chain =
            Some(invalid_follow_transition_chain(protocol::TimelineId(41)));
        let error =
            render_timeline_follow_output(&mut renderer, &engine_snapshot, &follow, 7, &mut state)
                .expect_err("a LastValid Follow result must fail before the send boundary");
        assert!(error.contains("ISF source"), "{error}");
        assert_eq!(
            state.last_valid(key),
            Some(&frame),
            "the rejected LastValid result must not mutate the transport cache"
        );

        let resized_key = TimelineFollowOutputFrameKey { width: 8, ..key };
        let newer_key = TimelineFollowOutputFrameKey {
            generation: 13,
            ..key
        };
        assert!(state.last_valid(resized_key).is_none());
        assert!(state.last_valid(newer_key).is_none());
    }

    #[test]
    fn follow_output_disabled_and_physical_send_faults_are_explicit() {
        let engine_snapshot = protocol::EngineSnapshot::default();
        let mut renderer = video::VideoPreviewRenderer::new(video::VideoRuntimeConfig::default());
        let mut state = TimelineFollowOutputState::default();
        let mut follow = follow_render_snapshot(protocol::VideoClipTakeKind::Crossfade);
        follow.outgoing_video.outputs[0].enabled = false;
        let disabled =
            render_timeline_follow_output(&mut renderer, &engine_snapshot, &follow, 7, &mut state)
                .unwrap();
        let TimelineFollowOutputRenderDecision::NotApplicable { key, reason } = disabled else {
            panic!("disabled Follow output must be explicitly not applicable");
        };
        assert_eq!(key.output_id, 7);
        assert!(reason.contains("disabled"));

        assert_eq!(
            timeline_follow_result_after_physical_send(
                TimelineFollowSettlementAckResult::Applied,
                None,
                "Fake",
                7,
            ),
            TimelineFollowSettlementAckResult::Applied
        );
        let fault = timeline_follow_result_after_physical_send(
            TimelineFollowSettlementAckResult::Applied,
            Some("injected send fault"),
            "Fake",
            7,
        );
        assert_eq!(
            fault,
            TimelineFollowSettlementAckResult::Fault {
                fault: "Fake output route 7 send failed: injected send fault".to_string(),
            }
        );
    }

    #[test]
    fn follow_ack_retries_the_exact_result_then_deduplicates_publication() {
        let key = TimelineFollowOutputFrameKey {
            epoch: 4,
            generation: 8,
            output_id: 12,
            width: 1920,
            height: 1080,
        };
        let now = Instant::now();
        let mut state = TimelineFollowOutputState::default();
        state.queue_ack(key, TimelineFollowSettlementAckResult::Applied, now);
        let mut attempts = Vec::new();
        assert!(state
            .try_publish_ack(now, |ack| {
                attempts.push(ack.clone());
                Err("injected reply loss".to_string())
            })
            .is_err());
        state.queue_ack(
            key,
            TimelineFollowSettlementAckResult::Fault {
                fault: "later frame must not replace pending result".to_string(),
            },
            now + TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY,
        );
        assert!(state
            .try_publish_ack(now + TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY, |ack| {
                attempts.push(ack.clone());
                Ok(())
            })
            .unwrap());
        assert_eq!(attempts.len(), 2);
        assert_eq!(
            attempts[0], attempts[1],
            "reply loss retries the identical ACK"
        );
        assert_eq!(
            attempts[1].result,
            TimelineFollowSettlementAckResult::Applied
        );

        let mut post_publish_calls = 0;
        state.queue_ack(
            key,
            TimelineFollowSettlementAckResult::Fault {
                fault: "published physical result must not be replaced".to_string(),
            },
            Instant::now(),
        );
        assert!(!state
            .try_publish_ack(Instant::now(), |_| {
                post_publish_calls += 1;
                Ok(())
            })
            .unwrap());
        assert_eq!(post_publish_calls, 0, "published result is not stormed");

        let next = TimelineFollowOutputFrameKey {
            generation: 9,
            ..key
        };
        state.queue_ack(
            next,
            TimelineFollowSettlementAckResult::NotApplicable,
            Instant::now(),
        );
        assert!(state.try_publish_ack(Instant::now(), |_| Ok(())).unwrap());
        assert!(state.published_for(next));
    }

    #[test]
    fn follow_ack_reply_loss_rollover_retires_old_generation_for_new_send() {
        let old = TimelineFollowOutputFrameKey {
            epoch: 4,
            generation: 8,
            output_id: 12,
            width: 1920,
            height: 1080,
        };
        let new = TimelineFollowOutputFrameKey {
            generation: 9,
            ..old
        };
        let now = Instant::now();
        let mut state = TimelineFollowOutputState::default();

        // The old physical send succeeded, but its engine reply was lost.
        state.queue_ack(old, TimelineFollowSettlementAckResult::Applied, now);
        assert!(state
            .try_publish_ack(now, |_| Err("injected old reply loss".to_string()))
            .is_err());
        assert_eq!(
            state.pending_ack_result(),
            Some(&TimelineFollowSettlementAckResult::Applied)
        );

        // No Follow snapshot is observed while the presenter is absent. When
        // the new generation reaches Fresh+physical-send, its key retires the
        // old retry instead of allowing it to starve this consumer.
        state.queue_ack(new, TimelineFollowSettlementAckResult::Applied, now);
        let mut published = Vec::new();
        assert!(state
            .try_publish_ack(now, |ack| {
                published.push(ack.clone());
                Ok(())
            })
            .unwrap());
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].generation, new.generation);
        assert_eq!(
            published[0].result,
            TimelineFollowSettlementAckResult::Applied
        );
        assert!(state.published_for(new));
        assert!(!state.published_for(old));
    }

    #[test]
    fn follow_ack_reply_loss_resize_preserves_ack_but_reseeds_last_valid() {
        let old = TimelineFollowOutputFrameKey {
            epoch: 4,
            generation: 8,
            output_id: 12,
            width: 4,
            height: 2,
        };
        let resized = TimelineFollowOutputFrameKey { width: 8, ..old };
        let now = Instant::now();
        let mut state = TimelineFollowOutputState::default();
        let old_frame = video::VideoFrame {
            layer_id: 0,
            width: old.width,
            height: old.height,
            pts_ms: 0,
            duration_ms: 33,
            format: video::VideoPixelFormat::Rgba8,
            data: vec![0; (old.width * old.height * 4) as usize],
        };

        // Fresh old-size frame physically sent, followed by reply loss.
        state.update_last_valid(old, old_frame);
        state.queue_ack(old, TimelineFollowSettlementAckResult::Applied, now);
        assert!(state
            .try_publish_ack(now, |_| Err("injected old reply loss".to_string()))
            .is_err());

        // The engine consumer is unchanged by resize, so only last-valid is
        // invalidated. Fresh resized output reseeds continuity without
        // replacing the pending physical Applied acknowledgement.
        state.observe_key(resized);
        assert!(state.last_valid(old).is_none());
        assert!(state.last_valid(resized).is_none());
        let resized_frame = video::VideoFrame {
            layer_id: 0,
            width: resized.width,
            height: resized.height,
            pts_ms: 0,
            duration_ms: 33,
            format: video::VideoPixelFormat::Rgba8,
            data: vec![0; (resized.width * resized.height * 4) as usize],
        };
        state.update_last_valid(resized, resized_frame.clone());
        state.queue_ack(resized, TimelineFollowSettlementAckResult::Applied, now);
        assert_eq!(
            state.pending_ack_result(),
            Some(&TimelineFollowSettlementAckResult::Applied)
        );
        let mut published = Vec::new();
        assert!(state
            .try_publish_ack(now + TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY, |ack| {
                published.push(ack.clone());
                Ok(())
            })
            .unwrap());
        assert_eq!(published.len(), 1);
        assert_eq!(
            published[0].result,
            TimelineFollowSettlementAckResult::Applied
        );
        assert!(state.published_for(resized));
        assert_eq!(state.last_valid(resized), Some(&resized_frame));
    }

    #[test]
    fn follow_epoch_fence_rejects_rotated_owner_before_physical_send() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let first = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        first.complete().unwrap();
        let stale_epoch = engine.output_ownership_status().epoch;
        let second = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        second.complete().unwrap();
        let current_epoch = engine.output_ownership_status().epoch;
        assert_ne!(stale_epoch, current_epoch);

        let stale = TimelineFollowOutputFrameKey {
            epoch: stale_epoch,
            generation: 8,
            output_id: 12,
            width: 4,
            height: 2,
        };
        let current = TimelineFollowOutputFrameKey {
            epoch: current_epoch,
            ..stale
        };
        assert!(
            !timeline_follow_epoch_matches_ready_owner(&engine, stale),
            "stale Follow frame must be rejected before NDI send or Applied ACK"
        );
        assert!(timeline_follow_epoch_matches_ready_owner(&engine, current));
    }

    #[test]
    fn follow_fault_fences_before_rejected_ack_then_retries_exactly() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let key = TimelineFollowOutputFrameKey {
            epoch: engine.output_ownership_status().epoch,
            generation: 8,
            output_id: 12,
            width: 4,
            height: 2,
        };
        let mut state = TimelineFollowOutputState::default();
        let (failure_lease, failed_key) = fence_timeline_follow_fault_before_ack(
            &engine,
            &mut state,
            key,
            "injected NDI send failure".to_string(),
        );
        assert_eq!(
            engine.output_ownership_status().state,
            OutputOwnershipState::Failed
        );
        assert!(engine.acquire_video_output().is_err());
        let now = Instant::now();
        assert!(state
            .try_publish_ack(now, |_| Err("injected ACK timeout".to_string()))
            .is_err());
        assert_eq!(
            engine.output_ownership_status().state,
            OutputOwnershipState::Failed
        );
        assert!(engine.acquire_video_output().is_err());
        assert!(state
            .try_publish_ack(now + TIMELINE_FOLLOW_SETTLEMENT_ACK_RETRY, |_| Ok(()))
            .unwrap());
        assert_ne!(failed_key.epoch, key.epoch);
        assert!(state.published_for(failed_key));
        drop(failure_lease);
    }

    #[test]
    fn follow_failure_fence_acknowledges_fault_at_failed_epoch_and_resolves_hold() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let output_id = 91_201;
        let output = follow_video_snapshot(output_id, 4, 2, true)
            .outputs
            .into_iter()
            .next()
            .unwrap();
        engine
            .send(engine::EngineCommand::AddVideoOutput(output))
            .unwrap();

        let audio_layer = protocol::TimelineLayerSummary {
            id: 2,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: false,
            kind: protocol::TimelineLayerKind::Audio,
        };
        let mut source = protocol::TimelineSnapshot {
            id: protocol::TimelineId(91_202),
            label: "NDI failed-epoch Follow source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 91_204,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://ndi-failed-epoch-follow.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(91_203),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(91_203),
            label: "NDI failed-epoch Follow target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(91_202),
                false,
            )
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekTimeline(90))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetTimelinePlaying(true))
            .unwrap();

        let deadline = Instant::now() + Duration::from_secs(2);
        let follow = loop {
            if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                if matches!(
                    engine.timeline_follow_runtime_status(follow.epoch).status,
                    protocol::TimelineFollowRuntimeStatus::Settling
                ) {
                    break follow;
                }
            }
            assert!(
                Instant::now() < deadline,
                "NDI Follow never entered settlement"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        let old_key = TimelineFollowOutputFrameKey {
            epoch: follow.epoch,
            generation: follow.generation,
            output_id,
            width: 4,
            height: 2,
        };
        assert_eq!(old_key.epoch, engine.output_ownership_status().epoch);

        let mut state = TimelineFollowOutputState::default();
        let (failure_lease, failed_key) = fence_timeline_follow_fault_before_ack(
            &engine,
            &mut state,
            old_key,
            "injected NDI renderer failure".to_string(),
        );
        assert_ne!(failed_key.epoch, old_key.epoch);
        assert!(engine
            .acknowledge_timeline_follow_settlement_published(
                old_key.settlement_ack(TimelineFollowSettlementAckResult::Fault {
                    fault: "stale pre-fence NDI fault".to_string(),
                }),
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap_err()
            .contains("epoch"));
        let mut publish_attempts = 0;
        publish_timeline_follow_output_result_until_resolved_with(
            &engine,
            &mut state,
            failed_key,
            TimelineFollowSettlementAckResult::Fault {
                fault: "injected NDI renderer failure".to_string(),
            },
            || false,
            |ack| {
                publish_attempts += 1;
                engine.acknowledge_timeline_follow_settlement_published(
                    ack.clone(),
                    Instant::now() + Duration::from_secs(1),
                )?;
                if publish_attempts == 1 {
                    assert!(
                        engine.timeline_follow_video_render_snapshot().is_none(),
                        "the first physical Fault ACK must retire the presenter"
                    );
                    Err("injected terminal ACK reply loss".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(
            publish_attempts, 2,
            "the exact Fault ACK must replay through the engine terminal receipt"
        );
        assert!(state.published_for(failed_key));
        let runtime = engine.snapshot().timeline.follow_runtime;
        assert_eq!(runtime.status, protocol::TimelineFollowRuntimeStatus::Held);
        assert!(matches!(
            runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Held)
        ));
        drop(failure_lease);
    }

    #[test]
    fn follow_applied_final_ack_retries_terminal_receipt_after_reply_loss() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let output_id = 93_201;
        let output = follow_video_snapshot(output_id, 4, 2, true)
            .outputs
            .into_iter()
            .next()
            .unwrap();
        engine
            .send(engine::EngineCommand::AddVideoOutput(output))
            .unwrap();
        let audio_layer = protocol::TimelineLayerSummary {
            id: 2,
            label: "Audio".to_string(),
            order: 0,
            muted: false,
            locked: false,
            solo: false,
            expanded: false,
            kind: protocol::TimelineLayerKind::Audio,
        };
        let mut source = protocol::TimelineSnapshot {
            id: protocol::TimelineId(93_202),
            label: "NDI Applied receipt source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 93_204,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://ndi-applied-receipt.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
                output_bus: protocol::TimelineAudioOutputBus::Program,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(93_203),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            hold_first_destination_measure: false,
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(93_203),
            label: "NDI Applied receipt target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(93_202),
                false,
            )
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekTimeline(90))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetTimelinePlaying(true))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        let follow = loop {
            if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                if matches!(
                    engine.timeline_follow_runtime_status(follow.epoch).status,
                    protocol::TimelineFollowRuntimeStatus::Settling
                ) {
                    break follow;
                }
            }
            assert!(
                Instant::now() < deadline,
                "NDI Applied Follow never entered settlement"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        engine
            .acknowledge_timeline_follow_settlement_published(
                TimelineFollowSettlementAck {
                    epoch: follow.epoch,
                    generation: follow.generation,
                    domain: TimelineFollowSettlementDomain::Audio,
                    consumer_id: TimelineFollowSettlementConsumerId::Audio,
                    result: TimelineFollowSettlementAckResult::Applied,
                },
                Instant::now() + Duration::from_secs(1),
            )
            .unwrap();
        let key = TimelineFollowOutputFrameKey {
            epoch: follow.epoch,
            generation: follow.generation,
            output_id,
            width: 4,
            height: 2,
        };
        let mut state = TimelineFollowOutputState::default();
        let mut publish_attempts = 0;
        publish_timeline_follow_output_result_until_resolved_with(
            &engine,
            &mut state,
            key,
            TimelineFollowSettlementAckResult::Applied,
            || false,
            |ack| {
                publish_attempts += 1;
                engine.acknowledge_timeline_follow_settlement_published(
                    ack.clone(),
                    Instant::now() + Duration::from_secs(1),
                )?;
                if publish_attempts == 1 {
                    assert!(engine.timeline_follow_video_render_snapshot().is_none());
                    Err("injected NDI Applied ACK reply loss".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(publish_attempts, 2);
        assert!(state.published_for(key));
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.timeline.id, protocol::TimelineId(93_203));
        assert!(matches!(
            snapshot.timeline.follow_runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Completed)
        ));
    }

    #[test]
    fn ndi_open_failure_hands_one_fence_to_parent_until_cleanup_ack() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let initial_epoch = engine.output_ownership_status().epoch;
        let startup_failure_lease: StartupFailureLeaseSlot =
            Arc::new(Mutex::new(StartupFailureFenceState::default()));
        let worker_slot = Arc::clone(&startup_failure_lease);
        let worker_engine = engine.clone();
        let (failed_tx, failed_rx) = std::sync::mpsc::sync_channel(1);
        let (cleanup_tx, cleanup_rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            ensure_startup_failure_fence(&worker_engine, &worker_slot, "injected NDI open failure");
            creation_lease.retire();
            failed_tx.send(()).unwrap();
            cleanup_rx.recv().unwrap();
        });
        failed_rx.recv().unwrap();

        let failure_lease = take_startup_failure_lease(&startup_failure_lease)
            .expect("the parent must receive the exact inner open-failure fence");
        let failed = engine.output_ownership_status();
        assert_eq!(failed.epoch, initial_epoch + 1);
        assert_eq!(failed.state, OutputOwnershipState::Failed);
        assert!(engine.acquire_video_output().is_err());
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());

        cleanup_tx.send(()).unwrap();
        worker.join().unwrap();
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());
        drop(failure_lease);

        let rearmed = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        rearmed.complete().unwrap();
    }

    #[test]
    fn ndi_startup_timeout_and_late_constructor_error_share_one_fence() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let initial_epoch = engine.output_ownership_status().epoch;
        let startup_failure_lease: StartupFailureLeaseSlot =
            Arc::new(Mutex::new(StartupFailureFenceState::default()));
        let worker_slot = Arc::clone(&startup_failure_lease);
        let worker_engine = engine.clone();
        let (entered_tx, entered_rx) = std::sync::mpsc::sync_channel(1);
        let (return_error_tx, return_error_rx) = std::sync::mpsc::sync_channel(1);
        let (failed_epoch_tx, failed_epoch_rx) = std::sync::mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            entered_tx.send(()).unwrap();
            return_error_rx.recv().unwrap();
            let mut state = TimelineFollowOutputState::default();
            let failed_key = fence_timeline_follow_fault_before_ack_in_startup(
                &worker_engine,
                &mut state,
                TimelineFollowOutputFrameKey {
                    epoch: initial_epoch,
                    generation: 66,
                    output_id: 706,
                    width: 1,
                    height: 1,
                },
                "late NDI constructor error".to_string(),
                &worker_slot,
            );
            creation_lease.retire();
            failed_epoch_tx.send(failed_key.epoch).unwrap();
            Err::<(), String>("late NDI constructor error".to_string())
        });
        entered_rx.recv().unwrap();

        let timeout_epoch = ensure_startup_failure_fence(
            &engine,
            &startup_failure_lease,
            "NDI startup constructor timeout",
        );
        let failure_lease = take_startup_failure_lease(&startup_failure_lease)
            .expect("timeout must transfer its single fence to parent cleanup");
        assert_eq!(timeout_epoch, initial_epoch + 1);
        return_error_tx.send(()).unwrap();
        assert_eq!(failed_epoch_rx.recv().unwrap(), timeout_epoch);
        assert!(worker.join().unwrap().is_err());
        assert_eq!(engine.output_ownership_status().epoch, initial_epoch + 1);
        assert!(engine.acquire_video_output().is_err());
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());
        drop(failure_lease);

        let rearmed = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        rearmed.complete().unwrap();
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
        let _hook_guard = NDI_TEST_HOOK_LOCK.lock().unwrap();
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
        let _hook_guard = NDI_TEST_HOOK_LOCK.lock().unwrap();
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
        // Synchronize on the delayed constructor actually entering before the
        // startup-timeout window is measured, mirroring the Spout twin's
        // entered-channel handshake; otherwise the attempt count below races
        // the scheduler.
        let entered_deadline = Instant::now() + Duration::from_secs(2);
        while constructor_attempts.load(Ordering::Acquire) == 0 {
            assert!(
                Instant::now() < entered_deadline,
                "delayed NDI constructor never entered"
            );
            std::thread::yield_now();
        }

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
        drop(pending);
        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
    }
}
