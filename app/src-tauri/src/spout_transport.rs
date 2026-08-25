#![cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

use engine::{
    EngineHandle, OutputOwnershipActivation, OutputOwnershipCreationLease,
    OutputOwnershipTeardownLease,
};
use protocol::{TimelineFollowSettlementAckResult, VideoLayerId};

#[cfg(test)]
use protocol::{OutputOwnershipState, VideoOutputId};

#[cfg(test)]
use engine::{TimelineFollowVideoRenderSnapshot, VideoPresentationSample};
#[cfg(test)]
use protocol::{
    TimelineFollowSettlementAck, TimelineFollowSettlementConsumerId, TimelineFollowSettlementDomain,
};
#[cfg(test)]
use std::sync::atomic::AtomicUsize;

// One shared FC-28 authority fence, bounded-settlement watchdog, and Follow
// render/settlement source for both physical transports. Both parents include
// the same file without a main.rs declaration; backend differences enter only
// as parameters. See the module docs in physical_output_fence.rs.
#[path = "physical_output_fence.rs"]
mod physical_output_fence;
use physical_output_fence::*;

#[cfg(test)]
static SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS: AtomicUsize = AtomicUsize::new(0);

#[cfg(test)]
static SPOUT_WORKER_SPAWN_FAILURE: AtomicBool = AtomicBool::new(false);

fn spout_output_effect_render_context(
    snapshot: &protocol::EngineSnapshot,
    project_render_epoch: u64,
) -> video::VideoEffectRenderContext<'_> {
    video::VideoEffectRenderContext {
        clip_runtime: &snapshot.video_clip_runtime,
        project_render_epoch,
    }
}
// This transport owns its Follow renderer state locally so the optional Spout
// feature never depends on the optional NDI output path. The FC-28 authority
// fence, bounded settlement watchdog, and Follow render/settlement machinery
// are the shared physical_output_fence module compiled into this parent.

#[cfg(test)]
static SPOUT_TEST_HOOK_LOCK: Mutex<()> = Mutex::new(());

pub type SpoutInputRegistry = Arc<Mutex<HashMap<VideoLayerId, video::VideoFrame>>>;

pub struct SpoutTransportState {
    inputs: SpoutInputRegistry,
    input_workers: HashMap<u64, SpoutRouteWorker>,
    output_workers: HashMap<u64, SpoutRouteWorker>,
    pending_output_startups: HashMap<u64, SpoutPendingOutputStartup>,
    failed_output_routes: HashMap<u64, String>,
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
            pending_output_startups: HashMap::new(),
            failed_output_routes: HashMap::new(),
            #[cfg(feature = "ndi")]
            ndi_inputs,
            capture_inputs,
        }
    }

    pub fn harvest_failed_workers(
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
            .output_workers
            .iter()
            .filter_map(|(route_id, worker)| {
                (worker.failure_snapshot().is_some()
                    || worker
                        .worker
                        .as_ref()
                        .is_some_and(|worker| worker.is_finished()))
                .then_some(*route_id)
            })
            .collect::<Vec<_>>();
        for route_id in failed_ids {
            let Some(worker) = self.output_workers.remove(&route_id) else {
                continue;
            };
            let failure = worker.failure_snapshot();
            let stop_result = worker.stop();
            let failure = failure.or_else(|| stop_result.as_ref().err().cloned());
            if let Some(failure) = failure {
                if engine.output_ownership_status().state != protocol::OutputOwnershipState::Failed
                {
                    drop(engine.begin_output_ownership_failure_fence(format!(
                        "Spout output route {route_id} worker teardown failed: {}",
                        failure.message
                    )));
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
                let worker = SpoutRouteWorker::start_input(
                    route.route_id,
                    route.endpoint_name.clone(),
                    Arc::clone(&self.inputs),
                )?;
                if let Some(previous) = self.input_workers.insert(route.route_id, worker) {
                    let _ = previous.stop();
                }
            }
            video::ExternalVideoTransportDirection::Output => {
                if let Some(pending) = self.pending_output_startups.get(&route.route_id) {
                    return Err(driver_error(format!(
                        "Spout output route {} is still awaiting startup cleanup acknowledgement: {}",
                        route.route_id, pending.message
                    )));
                }
                if self.output_workers.contains_key(&route.route_id) {
                    return Err(driver_error(format!(
                        "Spout output route {} is still awaiting worker teardown acknowledgement",
                        route.route_id
                    )));
                }
                self.failed_output_routes.remove(&route.route_id);
                let worker = match SpoutRouteWorker::start_output(
                    route.route_id,
                    route.endpoint_name.clone(),
                    engine.clone(),
                    Arc::clone(&self.inputs),
                    #[cfg(feature = "ndi")]
                    Arc::clone(&self.ndi_inputs),
                    Arc::clone(&self.capture_inputs),
                    activation.ok_or_else(|| {
                        driver_error("Spout output resource activation was not admitted")
                    })?,
                ) {
                    Ok(worker) => worker,
                    Err(error) => {
                        if let Some(pending) = error.pending_startup {
                            self.pending_output_startups.insert(route.route_id, pending);
                        }
                        return Err(driver_error(error.message));
                    }
                };
                if let Some(previous) = self.output_workers.insert(route.route_id, worker) {
                    previous
                        .stop()
                        .map_err(|error| driver_error(error.message))?;
                }
                let publish_result = self
                    .output_workers
                    .get_mut(&route.route_id)
                    .expect("just-inserted Spout output worker")
                    .publish();
                if let Err(error) = publish_result {
                    let worker = self
                        .output_workers
                        .remove(&route.route_id)
                        .expect("just-inserted Spout output worker");
                    if let Err(stop_error) = worker.stop() {
                        self.failed_output_routes
                            .insert(route.route_id, stop_error.message.clone());
                        return Err(driver_error(format!(
                            "Spout output resource creation was invalidated and teardown failed: {}",
                            stop_error.message
                        )));
                    }
                    return Err(driver_error(format!(
                        "Spout output resource creation was invalidated before publication: {error}"
                    )));
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
                if let Some(worker) = self.input_workers.remove(&route.route_id) {
                    worker.stop().map_err(|error| driver_error(error.message))?;
                }
                self.inputs
                    .lock()
                    .map_err(|_| driver_error("Spout input frame registry lock was poisoned"))?
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
                    return Err(driver_error(
                        "Spout output teardown requires the serialized ownership transition",
                    ));
                }
                if let Some(worker) = self.output_workers.remove(&route.route_id) {
                    match worker.stop() {
                        Ok(()) => {
                            self.failed_output_routes.remove(&route.route_id);
                        }
                        Err(error) => {
                            if engine.output_ownership_status().state
                                != protocol::OutputOwnershipState::Failed
                            {
                                drop(
                                    engine.begin_output_ownership_failure_fence(
                                        error.message.clone(),
                                    ),
                                );
                            }
                            self.failed_output_routes
                                .insert(route.route_id, error.message.clone());
                            return Err(driver_error(error.message));
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
struct SpoutOutputWorkerStopError {
    message: String,
}

impl std::fmt::Display for SpoutOutputWorkerStopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

struct SpoutOutputWorkerStartError {
    message: String,
    pending_startup: Option<SpoutPendingOutputStartup>,
}

impl std::fmt::Debug for SpoutOutputWorkerStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SpoutOutputWorkerStartError")
            .field("message", &self.message)
            .field("has_pending_startup", &self.pending_startup.is_some())
            .finish()
    }
}

struct SpoutPendingOutputStartup {
    worker: Option<std::thread::JoinHandle<Result<(), SpoutOutputWorkerStopError>>>,
    failure_lease: Option<OutputOwnershipTeardownLease>,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    message: String,
}

impl SpoutPendingOutputStartup {
    fn poll(&mut self) -> bool {
        if let Some(worker) = self.worker.as_ref() {
            if !worker.is_finished() {
                return false;
            }
        }
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => self.message = error.message,
                Err(_) => self.message = "Spout output startup worker panicked".to_string(),
            }
        }
        let teardown_lease = self
            .teardown_lease
            .lock()
            .map(|mut slot| slot.take())
            .unwrap_or_else(|poisoned| poisoned.into_inner().take());
        drop(teardown_lease);
        self.failure_lease.take();
        true
    }
}

impl Drop for SpoutPendingOutputStartup {
    fn drop(&mut self) {
        // The transport state normally polls this record to acknowledge the
        // constructor worker and the synchronous sender drop. If the state is
        // destroyed first, keep every accounting token and the join handle in
        // an explicit reaper so the old SDK resource cannot be replaced or
        // become unaccounted.
        let teardown_lease = self
            .teardown_lease
            .lock()
            .map(|mut slot| slot.take())
            .unwrap_or_else(|poisoned| poisoned.into_inner().take());
        let payload = Arc::new(Mutex::new(Some((
            self.worker.take(),
            self.failure_lease.take(),
            teardown_lease,
        ))));
        let worker_payload = Arc::clone(&payload);
        let reaper = std::thread::Builder::new()
            .name("syndocal-spout-startup-reaper".to_string())
            .spawn(move || {
                let Some((worker, failure_lease, teardown_lease)) = worker_payload
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take()
                else {
                    return;
                };
                if let Some(worker) = worker {
                    let _ = worker.join();
                }
                drop(teardown_lease);
                drop(failure_lease);
            });
        if reaper.is_err() {
            if let Some((worker, failure_lease, teardown_lease)) = payload
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
                if let Some(teardown_lease) = teardown_lease {
                    std::mem::forget(teardown_lease);
                }
            }
        }
    }
}

type SpoutCreationLeaseSlot = Arc<Mutex<Option<OutputOwnershipCreationLease>>>;

fn take_spout_creation_lease(
    slot: &SpoutCreationLeaseSlot,
) -> Option<OutputOwnershipCreationLease> {
    slot.lock()
        .map(|mut lease| lease.take())
        .unwrap_or_else(|poisoned| poisoned.into_inner().take())
}

trait SpoutOutputSender {
    fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String>;
}

impl SpoutOutputSender for spout2::dx::Sender {
    fn send_image(&mut self, pixels: &[u8], width: u32, height: u32) -> Result<(), String> {
        self.send_image(pixels, width, height)
            .map_err(|error| error.to_string())
    }
}

struct SpoutRouteWorker {
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<(), SpoutOutputWorkerStopError>>>,
    failure: Arc<Mutex<Option<SpoutOutputWorkerStopError>>>,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    start_signal: Option<mpsc::SyncSender<SpoutOutputStartDecision>>,
    creation_lease: Option<OutputOwnershipCreationLease>,
}

enum SpoutOutputStartDecision {
    Publish,
    Retire(OutputOwnershipCreationLease),
}

fn spawn_spout_output_worker<F>(
    output_id: u64,
    worker: F,
) -> Result<std::thread::JoinHandle<Result<(), SpoutOutputWorkerStopError>>, String>
where
    F: FnOnce() -> Result<(), SpoutOutputWorkerStopError> + Send + 'static,
{
    #[cfg(test)]
    if SPOUT_WORKER_SPAWN_FAILURE.swap(false, Ordering::AcqRel) {
        return Err("injected Spout output worker spawn failure".to_string());
    }
    std::thread::Builder::new()
        .name(format!("syndocal-spout-output-{output_id}"))
        .spawn(worker)
        .map_err(|error| error.to_string())
}

fn record_spout_worker_failure(
    failure: &Arc<Mutex<Option<SpoutOutputWorkerStopError>>>,
    error: SpoutOutputWorkerStopError,
) {
    match failure.lock() {
        Ok(mut slot) => *slot = Some(error),
        Err(poisoned) => *poisoned.into_inner() = Some(error),
    }
}

impl SpoutRouteWorker {
    fn start_input(
        layer_id: VideoLayerId,
        sender_name: String,
        frames: SpoutInputRegistry,
    ) -> Result<Self, video::ExternalVideoTransportDriverError> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let failure = Arc::new(Mutex::new(None));
        let teardown_lease = Arc::new(Mutex::new(None));
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
                        return Ok(());
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
                Ok(())
            })
            .map_err(|error| {
                driver_error(format!("Failed to start Spout input worker: {error}"))
            })?;
        wait_until_ready(
            ready_rx,
            "Spout input",
            &stop,
            worker,
            failure,
            teardown_lease,
        )
    }

    fn start_output(
        output_id: u64,
        sender_name: String,
        engine: EngineHandle,
        spout_inputs: SpoutInputRegistry,
        #[cfg(feature = "ndi")] ndi_inputs: crate::ndi_transport::NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
        activation: OutputOwnershipActivation,
    ) -> Result<Self, SpoutOutputWorkerStartError> {
        let stop = Arc::new(AtomicBool::new(false));
        let worker_stop = Arc::clone(&stop);
        let failure = Arc::new(Mutex::new(None));
        let worker_failure = Arc::clone(&failure);
        let teardown_lease = Arc::new(Mutex::new(None));
        let worker_teardown_lease = Arc::clone(&teardown_lease);
        // A render failure is detected inside the render closure, before the
        // worker can enter its common sender teardown. Transfer that exact
        // fence back to the worker instead of dropping and reacquiring it.
        let render_failure_lease = Arc::new(Mutex::new(None));
        let worker_render_failure_lease = Arc::clone(&render_failure_lease);
        let startup_failure_lease: StartupFailureLeaseSlot =
            Arc::new(Mutex::new(StartupFailureFenceState::default()));
        let worker_startup_failure_lease = Arc::clone(&startup_failure_lease);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_sender, start_receiver) = mpsc::sync_channel::<SpoutOutputStartDecision>(1);
        let creation_lease_slot: SpoutCreationLeaseSlot = Arc::new(Mutex::new(None));
        let worker_creation_lease_slot = Arc::clone(&creation_lease_slot);
        let parent_engine = engine.clone();
        let worker = spawn_spout_output_worker(output_id, move || {
            let mut startup_follow_state = TimelineFollowOutputState::default();
            let creation_lease = match activation.admit_resource_creation() {
                Ok(lease) => lease,
                Err(error) => {
                    if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                        if let Ok(key) = timeline_follow_output_key(&follow, output_id) {
                            publish_timeline_follow_output_result_until_resolved(
                                &engine,
                                &mut startup_follow_state,
                                key,
                                TimelineFollowSettlementAckResult::NotApplicable,
                                || worker_stop.load(Ordering::Acquire),
                            );
                        }
                    }
                    let _ = ready_tx.send(Err(error.clone()));
                    return Err(SpoutOutputWorkerStopError { message: error });
                }
            };
            // The SDK source is constructed only after the linearizable gate
            // admission. A fence that wins before this point therefore never
            // reaches the SDK constructor.
            #[cfg(test)]
            SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS.fetch_add(1, Ordering::AcqRel);
            let mut sender = match spout2::dx::Sender::new(&sender_name) {
                Ok(sender) => sender,
                Err(error) => {
                    let message = format!("Failed to create Spout sender '{sender_name}': {error}");
                    let fault = format!("Spout output route {output_id} open failed: {message}");
                    // The creation lease remains live until below; establish
                    // the hard ownership fence before an ACK retry can wait.
                    if let Some(follow) = engine.timeline_follow_video_render_snapshot() {
                        match timeline_follow_output_key(&follow, output_id) {
                            Ok(key) => {
                                let failed_key = fence_timeline_follow_fault_before_ack_in_startup(
                                    &engine,
                                    &mut startup_follow_state,
                                    key,
                                    fault.clone(),
                                    &worker_startup_failure_lease,
                                );
                                publish_timeline_follow_output_result_until_resolved(
                                    &engine,
                                    &mut startup_follow_state,
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
                    let _ = ready_tx.send(Err(message.clone()));
                    eprintln!("{message}");
                    return Err(SpoutOutputWorkerStopError { message });
                }
            };
            sender.set_format(spout2::dx::format::R8G8B8A8_UNORM);
            match worker_creation_lease_slot.lock() {
                Ok(mut slot) => *slot = Some(creation_lease),
                Err(poisoned) => *poisoned.into_inner() = Some(creation_lease),
            }
            if ready_tx.send(Ok(())).is_err() {
                let Some(lease) = take_spout_creation_lease(&worker_creation_lease_slot) else {
                    return Err(SpoutOutputWorkerStopError {
                        message: "Spout output startup lease was lost after readiness disconnect"
                            .to_string(),
                    });
                };
                drop(sender);
                lease.retire();
                return Ok(());
            }
            let start_decision = loop {
                match start_receiver.recv_timeout(Duration::from_millis(25)) {
                    Ok(decision) => break Some(decision),
                    Err(mpsc::RecvTimeoutError::Timeout) if worker_stop.load(Ordering::Acquire) => {
                        break None;
                    }
                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break None,
                }
            };
            match start_decision {
                Some(SpoutOutputStartDecision::Publish) => {}
                Some(SpoutOutputStartDecision::Retire(lease)) => {
                    drop(sender);
                    lease.retire();
                    return Ok(());
                }
                None => {
                    if let Some(lease) = take_spout_creation_lease(&worker_creation_lease_slot) {
                        drop(sender);
                        lease.retire();
                        return Ok(());
                    }
                    let error =
                        "Spout output resource publication acknowledgement was dropped".to_string();
                    let failure_lease = engine.begin_output_ownership_failure_fence(error.clone());
                    let result = finish_spout_output_worker(
                        &engine,
                        sender,
                        &worker_teardown_lease,
                        Some(failure_lease),
                        Some(error),
                    );
                    if let Err(error) = &result {
                        record_spout_worker_failure(&worker_failure, error.clone());
                    }
                    return result;
                }
            }
            let decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::from_env()
                .with_spout_inputs(spout_inputs)
                .with_capture_inputs(capture_inputs);
            #[cfg(feature = "ndi")]
            let decoder = decoder.with_ndi_inputs(ndi_inputs);
            let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
                video::VideoRuntimeConfig::default(),
                video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
            );
            let render_engine = engine.clone();
            let render_sender_name = sender_name.clone();
            let render_worker_stop = Arc::clone(&worker_stop);
            let render_failure_lease = Arc::clone(&worker_render_failure_lease);
            let result = run_spout_output_worker(
                output_id,
                &sender_name,
                engine,
                worker_stop,
                sender,
                (worker_teardown_lease, worker_render_failure_lease),
                move |follow_output_state| {
                    let presentation_sample = render_engine.video_presentation_sample();
                    let snapshot = &presentation_sample.snapshot;
                    let ownership = render_engine.output_ownership_status();
                    let blackout_authority = render_engine.safety_blackout_authority();
                    renderer
                        .frame_provider_mut()
                        .set_bpm(Some(snapshot.clock.bpm));
                    let follow = render_engine.timeline_follow_video_render_snapshot();
                    let rendered = match capture_output_presentation_authority(
                        "Spout",
                        protocol::VideoOutputKind::SpoutSender,
                        &presentation_sample,
                        ownership,
                        blackout_authority,
                        output_id,
                        &render_sender_name,
                    )
                    .and_then(|authority| {
                        revalidate_output_presentation_authority(&render_engine, "Spout", &authority)?;
                        Ok(authority)
                    }) {
                        Ok(authority) => match follow.as_ref() {
                            Some(follow) if follow.epoch != authority.ownership_epoch() => {
                                let retry_key = timeline_follow_output_key(follow, output_id).ok();
                                Ok((
                                    TimelineFollowOutputRenderDecision::Retry {
                                        reason: "Spout Timeline Follow authority epoch changed before render"
                                            .to_string(),
                                        key: retry_key,
                                    },
                                    None,
                                ))
                            }
                            Some(follow) if authority.project_blackout() => {
                                // A current safety blackout is a canonical
                                // output result, not a property of either
                                // historical Follow snapshot. Do not inspect
                                // or render them.
                                render_timeline_follow_hard_blackout_output(follow, &authority)
                                    .map(|decision| (decision, Some(authority)))
                            }
                            Some(follow) => {
                                match validate_follow_output_authority("Spout", follow, &authority)
                                {
                                    Ok(()) => render_timeline_follow_output(
                                        &mut renderer,
                                        snapshot,
                                        follow,
                                        output_id,
                                        follow_output_state,
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
                                }
                            }
                            None => renderer
                                .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                                    &snapshot.video,
                                    spout_output_effect_render_context(
                                        snapshot,
                                        authority.ownership_epoch(),
                                    ),
                                    &snapshot.video_transition_runtime,
                                    output_id,
                                    authority.output_summary().width,
                                    authority.output_summary().height,
                                )
                                .map_err(|error| format!("Spout output {output_id} artistic render failed: {error:?}"))
                                .and_then(|result| {
                                    materialize_output_artistic_result(result, &authority)
                                })
                                .map(|frame| (
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
                                )),
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
                    if let Err(error) = &rendered {
                        if let Some(follow) = follow.as_ref() {
                            let fault =
                                format!("Spout output route {output_id} render failed: {error}");
                            let failure_fence = match timeline_follow_output_key(follow, output_id) {
                                    Ok(key) => {
                                        let (failure_fence, failed_key) =
                                            fence_timeline_follow_fault_before_ack(
                                                &render_engine,
                                                follow_output_state,
                                                key,
                                                fault.clone(),
                                            );
                                        publish_timeline_follow_output_result_until_resolved(
                                            &render_engine,
                                            follow_output_state,
                                            failed_key,
                                            TimelineFollowSettlementAckResult::Fault { fault },
                                            || render_worker_stop.load(Ordering::Acquire),
                                        );
                                        failure_fence
                                    }
                                    Err(key_error) => render_engine
                                        .begin_output_ownership_failure_fence(format!(
                                            "{fault}; {key_error}"
                                        )),
                                };
                            match render_failure_lease.lock() {
                                Ok(mut slot) => *slot = Some(failure_fence),
                                Err(poisoned) => *poisoned.into_inner() = Some(failure_fence),
                            }
                        } else {
                            let failure_fence = render_engine.begin_output_ownership_failure_fence(
                                format!("Spout output route {output_id} render failed: {error}"),
                            );
                            match render_failure_lease.lock() {
                                Ok(mut slot) => *slot = Some(failure_fence),
                                Err(poisoned) => *poisoned.into_inner() = Some(failure_fence),
                            }
                        }
                    }
                    rendered
                },
            );
            if let Err(error) = &result {
                record_spout_worker_failure(&worker_failure, error.clone());
            }
            result
        })
        .map_err(|error| SpoutOutputWorkerStartError {
            message: format!("Failed to start Spout output worker: {error}"),
            pending_startup: None,
        })?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                let Some(creation_lease) = take_spout_creation_lease(&creation_lease_slot) else {
                    stop.store(true, Ordering::Release);
                    drop(ready_rx);
                    drop(start_sender);
                    return Err(SpoutOutputWorkerStartError {
                        message:
                            "Spout output startup acknowledgement did not retain its creation lease"
                                .to_string(),
                        pending_startup: None,
                    });
                };
                Ok(Self {
                    stop,
                    worker: Some(worker),
                    failure,
                    teardown_lease,
                    start_signal: Some(start_sender),
                    creation_lease: Some(creation_lease),
                })
            }
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                drop(ready_rx);
                drop(start_sender);
                // Preserve a boundary fence established by sender creation or
                // the worker itself. The parent owns it until worker join and
                // physical teardown are acknowledged; do not advance epoch a
                // second time after a dropped/reacquired gap.
                let failure_lease = take_startup_failure_lease(&startup_failure_lease)
                    .or_else(|| match teardown_lease.lock() {
                        Ok(mut slot) => slot.take(),
                        Err(poisoned) => poisoned.into_inner().take(),
                    })
                    .or_else(|| {
                        ensure_startup_failure_fence(
                            &parent_engine,
                            &startup_failure_lease,
                            format!("Spout output startup failed: {error}"),
                        );
                        take_startup_failure_lease(&startup_failure_lease)
                    })
                    .expect("Spout startup failure fence must retain its lease");
                Err(SpoutOutputWorkerStartError {
                    message: error,
                    pending_startup: Some(SpoutPendingOutputStartup {
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown_lease,
                        message: "Spout output startup cleanup is pending".to_string(),
                    }),
                })
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                drop(ready_rx);
                drop(start_sender);
                let message =
                    format!("Spout output worker startup acknowledgement failed: {error}");
                ensure_startup_failure_fence(&parent_engine, &startup_failure_lease, &message);
                let failure_lease = take_startup_failure_lease(&startup_failure_lease)
                    .expect("startup timeout fence must retain its lease");
                Err(SpoutOutputWorkerStartError {
                    message,
                    pending_startup: Some(SpoutPendingOutputStartup {
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown_lease,
                        message: "Spout output startup cleanup is pending".to_string(),
                    }),
                })
            }
        }
    }

    fn publish(&mut self) -> Result<(), String> {
        let lease = self
            .creation_lease
            .take()
            .ok_or_else(|| "Spout output resource was already published".to_string())?;
        match lease.publish() {
            Ok(()) => {
                let Some(start_signal) = self.start_signal.take() else {
                    return Err(
                        "Spout output resource publication signal was already consumed".to_string(),
                    );
                };
                start_signal
                    .send(SpoutOutputStartDecision::Publish)
                    .map_err(|_| {
                        "Spout output worker stopped before resource publication".to_string()
                    })
            }
            Err(lease) => {
                self.creation_lease = Some(lease);
                Err("Spout output resource creation fence was invalidated".to_string())
            }
        }
    }

    fn failure_snapshot(&self) -> Option<SpoutOutputWorkerStopError> {
        self.failure
            .lock()
            .map(|failure| failure.clone())
            .unwrap_or_else(|poisoned| (*poisoned.into_inner()).clone())
    }

    fn release_teardown_lease(&mut self) {
        let lease = self
            .teardown_lease
            .lock()
            .map(|mut slot| slot.take())
            .unwrap_or_else(|poisoned| poisoned.into_inner().take());
        drop(lease);
    }

    fn stop(mut self) -> Result<(), SpoutOutputWorkerStopError> {
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        let join_result = self.worker.take().map(|worker| {
            worker.join().map_err(|_| SpoutOutputWorkerStopError {
                message: "Spout output worker panicked while stopping".to_string(),
            })
        });
        self.release_teardown_lease();
        if let Some(result) = join_result.transpose()? {
            result?;
        }
        drop(undelivered_creation_lease);
        Ok(())
    }

    fn signal_retirement(&mut self) -> Option<OutputOwnershipCreationLease> {
        let lease = self.creation_lease.take()?;
        let Some(start_signal) = self.start_signal.take() else {
            return Some(lease);
        };
        match start_signal.send(SpoutOutputStartDecision::Retire(lease)) {
            Ok(()) => None,
            Err(error) => match error.0 {
                SpoutOutputStartDecision::Retire(lease) => Some(lease),
                SpoutOutputStartDecision::Publish => None,
            },
        }
    }
}

fn wait_until_ready(
    ready: mpsc::Receiver<Result<(), String>>,
    label: &str,
    stop: &Arc<AtomicBool>,
    worker: std::thread::JoinHandle<Result<(), SpoutOutputWorkerStopError>>,
    failure: Arc<Mutex<Option<SpoutOutputWorkerStopError>>>,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
) -> Result<SpoutRouteWorker, video::ExternalVideoTransportDriverError> {
    match ready.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(SpoutRouteWorker {
            stop: Arc::clone(stop),
            worker: Some(worker),
            failure,
            teardown_lease,
            start_signal: None,
            creation_lease: None,
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
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        self.release_teardown_lease();
        drop(undelivered_creation_lease);
    }
}

fn finish_spout_output_worker<S: SpoutOutputSender>(
    engine: &EngineHandle,
    sender: S,
    teardown_lease_slot: &Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    failure_lease: Option<engine::OutputOwnershipTeardownLease>,
    mut worker_error: Option<String>,
) -> Result<(), SpoutOutputWorkerStopError> {
    let teardown_lease = if let Some(lease) = failure_lease {
        lease
    } else if let Some(error) = worker_error.as_ref() {
        engine.begin_output_ownership_failure_fence(error.clone())
    } else {
        match engine.begin_output_ownership_teardown() {
            Ok(lease) => lease,
            Err(error) => {
                let message = format!("Spout output teardown was not admitted: {error}");
                worker_error = Some(message.clone());
                engine.begin_output_ownership_failure_fence(message)
            }
        }
    };

    // Spout has no asynchronous close acknowledgement. Dropping the sender is
    // the physical teardown, and the worker's join is the acknowledgement that
    // this synchronous destruction has completed. Keep the ownership lease live
    // across the drop so a retry cannot overlap the old sender.
    drop(sender);
    match teardown_lease_slot.lock() {
        Ok(mut slot) => *slot = Some(teardown_lease),
        Err(poisoned) => *poisoned.into_inner() = Some(teardown_lease),
    }

    match worker_error {
        Some(message) => Err(SpoutOutputWorkerStopError { message }),
        None => Ok(()),
    }
}

type SpoutLeaseSlots = (
    Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
);

fn run_spout_output_worker<S, R>(
    output_id: u64,
    sender_name: &str,
    engine: EngineHandle,
    worker_stop: Arc<AtomicBool>,
    mut sender: S,
    lease_slots: SpoutLeaseSlots,
    mut render: R,
) -> Result<(), SpoutOutputWorkerStopError>
where
    S: SpoutOutputSender,
    R: FnMut(
        &mut TimelineFollowOutputState,
    ) -> Result<
        (
            TimelineFollowOutputRenderDecision,
            Option<OutputPresentationAuthority>,
        ),
        String,
    >,
{
    let (teardown_lease_slot, render_failure_lease) = lease_slots;
    let target_interval = Duration::from_nanos(1_000_000_000 / 60);
    let mut worker_error = None;
    let mut failure_lease = None;
    let mut follow_output_state = TimelineFollowOutputState::default();
    while !worker_stop.load(Ordering::Acquire) {
        let started = Instant::now();
        let rendered = render(&mut follow_output_state);
        let handed_off_failure_lease =
            rendered
                .as_ref()
                .err()
                .and_then(|_| match render_failure_lease.lock() {
                    Ok(mut slot) => slot.take(),
                    Err(poisoned) => poisoned.into_inner().take(),
                });
        let result = match rendered {
            Ok((TimelineFollowOutputRenderDecision::NotApplicable { key, reason }, _)) => {
                // This transport cannot prove an atomic engine video quorum,
                // so it may not turn a locally observed NotApplicable sample
                // into a terminal Follow ACK. Discard and reacquire instead,
                // but only within the bounded-settlement deadline: a hold
                // that never admits must fault visibly.
                if follow_output_state.tick_follow_unresolved(key, Instant::now(), &reason) {
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
                        TimelineFollowSettlementAckResult::Fault {
                            fault: fault.clone(),
                        },
                        || worker_stop.load(Ordering::Acquire),
                    );
                    Err(fault)
                } else {
                    Ok(())
                }
            }
            Ok((TimelineFollowOutputRenderDecision::Retry { reason, key }, _)) => {
                // A route/ownership/Follow generation changed while sampling.
                // It is neither a physical-send fault nor a NotApplicable ACK,
                // but the same generation may not stay unadmitted forever.
                let mut outcome = Ok(());
                if let Some(key) = key {
                    if follow_output_state.tick_follow_unresolved(key, Instant::now(), &reason) {
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
                            break Err(SpoutOutputWorkerStopError {
                                message: "Spout output worker is stopping".to_string(),
                            });
                        }
                        Err(_)
                            if admission_started.elapsed() >= VIDEO_OUTPUT_ADMISSION_DEADLINE =>
                        {
                            break Err(SpoutOutputWorkerStopError {
                                message: format!(
                                    "Spout output route {output_id} video ownership admission deadline ({VIDEO_OUTPUT_ADMISSION_DEADLINE:?}) exceeded"
                                ),
                            });
                        }
                        Err(_) => {
                            // The ownership gate did not admit this frame. Without
                            // an engine-issued atomic quorum proof, do not
                            // acknowledge it. The bounded wait above turns a
                            // stuck gate into a visible fault instead of an
                            // indefinite silent hold.
                            std::thread::sleep(Duration::from_millis(5));
                        }
                    }
                };
                match permit_outcome {
                    Ok(_permit) => {
                        let send_result = send_frame_if_authorized(
                            "Spout",
                            &engine,
                            &authority,
                            follow_identity,
                            || sender.send_image(&frame.data, frame.width, frame.height),
                        );
                        match send_result {
                            Ok(()) => {
                                if follow_identity.is_some() {
                                    // The settled physical outcome for this generation
                                    // was just achieved; clear its unresolved clock.
                                    follow_output_state.note_follow_admitted();
                                    let settlement = timeline_follow_result_after_physical_send(
                                        follow_result,
                                        None,
                                        "Spout",
                                        output_id,
                                    );
                                    if matches!(
                                        &settlement,
                                        TimelineFollowSettlementAckResult::Applied
                                    ) {
                                        // The physical send above succeeded. Keep its
                                        // exact Applied ACK retryable through terminal
                                        // presenter retirement/reply loss.
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
                                // The physical closure was not entered. A fresh loop
                                // captures current authority (and, for blackout,
                                // produces canonical hard black) without a stale ACK
                                // or fault fence — but the same generation may not be
                                // revoked on every attempt forever.
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
                                // The frame permit is live here. Fence before a Follow
                                // Fault acknowledgement retry can block this worker.
                                let failed_follow_key = if follow_identity.is_some() {
                                    let (lease, failed_key) = fence_timeline_follow_fault_before_ack(
                                &engine,
                                &mut follow_output_state,
                                key,
                                format!("Spout output route {output_id} send failed: {error}"),
                            );
                                    failure_lease = Some(lease);
                                    Some(failed_key)
                                } else {
                                    failure_lease =
                                        Some(engine.begin_output_ownership_failure_fence(format!(
                                            "Spout output route {output_id} send failed: {error}"
                                        )));
                                    None
                                };
                                if follow_identity.is_some() {
                                    let settlement = timeline_follow_result_after_physical_send(
                                        follow_result,
                                        Some(error.as_str()),
                                        "Spout",
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
                        // A lost-ownership wait is still a settlement path: a
                        // pending Follow generation must resolve visibly, and
                        // the frame itself must never be silently dropped. Keep
                        // the failure lease alive through shared teardown.
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
            Ok((TimelineFollowOutputRenderDecision::Frame { .. }, None)) => {
                Err("Spout output frame was produced without an authority contract".to_string())
            }
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            if !worker_stop.load(Ordering::Acquire) {
                if failure_lease.is_none() {
                    // Render failures happen before a frame permit can be
                    // acquired. The inner render path may already have
                    // fenced and handed off its lease; otherwise fence here.
                    failure_lease = Some(handed_off_failure_lease.unwrap_or_else(|| {
                        engine.begin_output_ownership_failure_fence(format!(
                            "Spout output route {output_id} render failed: {error}"
                        ))
                    }));
                }
                worker_error = Some(error.clone());
            }
            eprintln!("Spout output '{sender_name}' stopped: {error}");
            break;
        }
        if let Some(remaining) = target_interval.checked_sub(started.elapsed()) {
            std::thread::sleep(remaining);
        }
    }

    finish_spout_output_worker(
        &engine,
        sender,
        &teardown_lease_slot,
        failure_lease,
        worker_error,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

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

    fn follow_render_snapshot(enabled: bool) -> TimelineFollowVideoRenderSnapshot {
        TimelineFollowVideoRenderSnapshot {
            epoch: 21,
            generation: 34,
            source_timeline_id: protocol::TimelineId(55),
            target_timeline_id: protocol::TimelineId(56),
            kind: protocol::VideoClipTakeKind::Wipe,
            curve: protocol::VideoLayerTransitionCurve::Linear,
            progress_millis: 500,
            outgoing_video: follow_video_snapshot(17, 4, 2, enabled),
            incoming_video: follow_video_snapshot(17, 4, 2, enabled),
            transition_effect_chain: None,
        }
    }

    #[test]
    fn spout_follow_render_is_local_evidenced_and_send_gated() {
        let mut renderer = video::VideoPreviewRenderer::new(video::VideoRuntimeConfig::default());
        let engine_snapshot = protocol::EngineSnapshot::default();
        let mut state = TimelineFollowOutputState::default();
        let follow = follow_render_snapshot(true);

        let TimelineFollowOutputRenderDecision::Frame {
            key, frame, result, ..
        } = render_timeline_follow_output(&mut renderer, &engine_snapshot, &follow, 17, &mut state)
            .expect("transparent Follow compositions have a presentable output")
        else {
            panic!("enabled Spout Follow output must render");
        };
        assert_eq!(
            (key.epoch, key.generation, key.width, key.height),
            (21, 34, 4, 2)
        );
        assert_eq!((frame.width, frame.height, frame.data.len()), (4, 2, 32));
        assert_eq!(result, TimelineFollowSettlementAckResult::Applied);
        assert_eq!(state.last_valid(key), Some(&frame));
        assert!(matches!(
            timeline_follow_result_after_physical_send(result, Some("injected"), "Spout", 17),
            TimelineFollowSettlementAckResult::Fault { .. }
        ));

        let disabled = follow_render_snapshot(false);
        assert!(matches!(
            render_timeline_follow_output(
                &mut renderer,
                &engine_snapshot,
                &disabled,
                17,
                &mut state
            )
            .unwrap(),
            TimelineFollowOutputRenderDecision::NotApplicable { .. }
        ));
    }

    #[test]
    fn spout_follow_ack_reply_loss_rollover_retires_old_generation_for_new_send() {
        let old = TimelineFollowOutputFrameKey {
            epoch: 21,
            generation: 34,
            output_id: 17,
            width: 4,
            height: 2,
        };
        let new = TimelineFollowOutputFrameKey {
            generation: 35,
            ..old
        };
        let now = Instant::now();
        let mut state = TimelineFollowOutputState::default();

        // A successful old send with a lost ACK remains exactly retryable
        // while old is current.
        state.queue_ack(old, TimelineFollowSettlementAckResult::Applied, now);
        assert!(state
            .try_publish_ack(now, |_| Err("injected old reply loss".to_string()))
            .is_err());

        // The old presenter then disappears; the next Fresh+sent frame owns
        // a new key and must not be blocked by that stale retry.
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
    fn spout_follow_ack_reply_loss_resize_preserves_ack_but_reseeds_last_valid() {
        let old = TimelineFollowOutputFrameKey {
            epoch: 21,
            generation: 34,
            output_id: 17,
            width: 4,
            height: 2,
        };
        let resized = TimelineFollowOutputFrameKey { height: 4, ..old };
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

        state.update_last_valid(old, old_frame);
        state.queue_ack(old, TimelineFollowSettlementAckResult::Applied, now);
        assert!(state
            .try_publish_ack(now, |_| Err("injected old reply loss".to_string()))
            .is_err());

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
    fn spout_follow_epoch_fence_rejects_rotated_owner_before_physical_send() {
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
            generation: 34,
            output_id: 17,
            width: 4,
            height: 2,
        };
        let current = TimelineFollowOutputFrameKey {
            epoch: current_epoch,
            ..stale
        };
        assert!(
            !timeline_follow_epoch_matches_ready_owner(&engine, stale),
            "stale Follow frame must be rejected before Spout send or Applied ACK"
        );
        assert!(timeline_follow_epoch_matches_ready_owner(&engine, current));
    }

    #[test]
    fn spout_follow_fault_fences_before_rejected_ack_then_retries_exactly() {
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
            generation: 34,
            output_id: 17,
            width: 4,
            height: 2,
        };
        let mut state = TimelineFollowOutputState::default();
        let (failure_lease, failed_key) = fence_timeline_follow_fault_before_ack(
            &engine,
            &mut state,
            key,
            "injected Spout send failure".to_string(),
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
    fn spout_follow_failure_fence_acknowledges_fault_at_failed_epoch_and_resolves_hold() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let output_id = 92_201;
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
            id: protocol::TimelineId(92_202),
            label: "Spout failed-epoch Follow source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 92_204,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://spout-failed-epoch-follow.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(92_203),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(92_203),
            label: "Spout failed-epoch Follow target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(92_202),
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
                "Spout Follow never entered settlement"
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
            "injected Spout renderer failure".to_string(),
        );
        assert_ne!(failed_key.epoch, old_key.epoch);
        assert!(engine
            .acknowledge_timeline_follow_settlement_published(
                old_key.settlement_ack(TimelineFollowSettlementAckResult::Fault {
                    fault: "stale pre-fence Spout fault".to_string(),
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
                fault: "injected Spout renderer failure".to_string(),
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
    fn spout_follow_applied_final_ack_retries_terminal_receipt_after_reply_loss() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let output_id = 94_201;
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
            id: protocol::TimelineId(94_202),
            label: "Spout Applied receipt source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 94_204,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://spout-applied-receipt.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(94_203),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(94_203),
            label: "Spout Applied receipt target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(94_202),
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
                "Spout Applied Follow never entered settlement"
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
                    Err("injected Spout Applied ACK reply loss".to_string())
                } else {
                    Ok(())
                }
            },
        );
        assert_eq!(publish_attempts, 2);
        assert!(state.published_for(key));
        let snapshot = engine.snapshot();
        assert_eq!(snapshot.timeline.id, protocol::TimelineId(94_203));
        assert!(matches!(
            snapshot.timeline.follow_runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Completed)
        ));
    }

    #[test]
    fn spout_output_effect_context_uses_runtime_clip_truth_and_ownership_epoch() {
        let mut snapshot = protocol::EngineSnapshot::default();
        snapshot
            .video_clip_runtime
            .layers
            .push(protocol::VideoClipLayerRuntimeSummary {
                layer_id: 11,
                active_slot_id: Some(protocol::VideoClipSlotId(14)),
                ..Default::default()
            });

        let context = spout_output_effect_render_context(&snapshot, 79);

        assert_eq!(context.project_render_epoch, 79);
        assert_eq!(context.clip_runtime, &snapshot.video_clip_runtime);
        assert_eq!(context.clip_runtime.layers[0].layer_id, 11);
        assert_eq!(
            context.clip_runtime.layers[0].active_slot_id,
            Some(protocol::VideoClipSlotId(14))
        );
    }

    #[test]
    fn failure_fence_before_spout_creation_admission_never_constructs_sender() {
        let _hook_guard = SPOUT_TEST_HOOK_LOCK.lock().unwrap();
        SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS.store(0, Ordering::Release);
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
            .begin_output_ownership_failure_fence("injected fence before Spout creation admission");

        let result = SpoutRouteWorker::start_output(
            19,
            "Fenced Spout Sender".to_string(),
            engine,
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
        );

        assert!(result.is_err());
        assert_eq!(
            SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS.load(Ordering::Acquire),
            0,
            "Spout constructor must not run after the failure fence wins admission"
        );
        drop(failure);
    }

    struct InjectedSpoutSender {
        send_error: Option<String>,
        drop_started: Arc<AtomicBool>,
        allow_drop: Arc<AtomicBool>,
        dropped: Arc<AtomicBool>,
    }

    impl SpoutOutputSender for InjectedSpoutSender {
        fn send_image(&mut self, _pixels: &[u8], _width: u32, _height: u32) -> Result<(), String> {
            self.send_error.take().map_or(Ok(()), Err)
        }
    }

    impl Drop for InjectedSpoutSender {
        fn drop(&mut self) {
            self.drop_started.store(true, Ordering::Release);
            while !self.allow_drop.load(Ordering::Acquire) {
                std::thread::yield_now();
            }
            self.dropped.store(true, Ordering::Release);
        }
    }

    fn injected_spout_frame() -> video::VideoFrame {
        video::VideoFrame {
            layer_id: 0,
            width: 1,
            height: 1,
            pts_ms: 0,
            duration_ms: 0,
            format: video::VideoPixelFormat::Rgba8,
            data: vec![0, 0, 0, 255],
        }
    }

    fn injected_spout_render_decision(
        output_id: VideoOutputId,
        ownership_epoch: u64,
    ) -> TimelineFollowOutputRenderDecision {
        let frame = injected_spout_frame();
        TimelineFollowOutputRenderDecision::Frame {
            key: TimelineFollowOutputFrameKey {
                epoch: ownership_epoch,
                generation: 0,
                output_id,
                width: frame.width,
                height: frame.height,
            },
            frame,
            result: TimelineFollowSettlementAckResult::Applied,
            follow_identity: None,
        }
    }

    fn install_injected_spout_output_extent(
        engine: &EngineHandle,
        output_id: VideoOutputId,
        width: u32,
        height: u32,
    ) {
        engine
            .send(engine::EngineCommand::AddVideoOutput(
                protocol::VideoOutputSummary {
                    id: output_id,
                    label: format!("Injected Spout {output_id}"),
                    kind: protocol::VideoOutputKind::SpoutSender,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width,
                    height,
                    endpoint_name: Some(injected_spout_endpoint(output_id)),
                    opacity: 1.0,
                    blackout: false,
                    mapping: protocol::VideoOutputMapping::default(),
                },
            ))
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
                "injected Spout output {output_id} did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn install_injected_spout_output(engine: &EngineHandle, output_id: VideoOutputId) {
        install_injected_spout_output_extent(engine, output_id, 1, 1);
    }

    fn injected_spout_endpoint(output_id: VideoOutputId) -> String {
        format!("Injected Spout {output_id}")
    }

    fn injected_spout_authority(
        engine: &EngineHandle,
        output_id: VideoOutputId,
    ) -> OutputPresentationAuthority {
        let sample = engine.video_presentation_sample();
        capture_output_presentation_authority(
            "Spout",
            protocol::VideoOutputKind::SpoutSender,
            &sample,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &injected_spout_endpoint(output_id),
        )
        .expect("injected Spout authority must be present and owned")
    }

    fn assert_spout_authority_rejects_without_send(
        engine: &EngineHandle,
        authority: &OutputPresentationAuthority,
        expected_error: &str,
    ) {
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sends_for_attempt = Arc::clone(&sends);
        let error = send_frame_if_authorized("Spout", engine, authority, None, move || {
            sends_for_attempt.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(())
        })
        .expect_err("changed authority must reject before the Spout SDK send");
        assert!(
            matches!(&error, PhysicalOutputSendError::Revoked(_)),
            "an authority mismatch is a typed revoke, never an SDK fault or success: {error}"
        );
        assert!(error.to_string().contains(expected_error), "{error}");
        assert_eq!(
            sends.load(std::sync::atomic::Ordering::Acquire),
            0,
            "a rejected Spout authority must not enter the physical send closure"
        );
    }

    fn spout_hard_blackout_artistic_result(
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

    fn bind_follow_to_spout_route(
        follow: &mut TimelineFollowVideoRenderSnapshot,
        output_id: VideoOutputId,
    ) {
        for video in [&mut follow.outgoing_video, &mut follow.incoming_video] {
            let output = video
                .outputs
                .iter_mut()
                .find(|output| output.id == output_id)
                .expect("historical Spout Follow output exists");
            output.label = format!("Injected Spout {output_id}");
            output.kind = protocol::VideoOutputKind::SpoutSender;
            output.endpoint_name = Some(injected_spout_endpoint(output_id));
        }
    }

    #[test]
    fn spout_route_identity_mismatch_rejects_before_any_send() {
        let output_id = 98_199;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let sample = engine.video_presentation_sample();
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let error = capture_output_presentation_authority(
            "Spout",
            protocol::VideoOutputKind::SpoutSender,
            &sample,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            "different Spout sender",
        )
        .expect_err("endpoint mismatch must reject the bound Spout route");
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
            .expect("Spout test output exists")
            .kind = protocol::VideoOutputKind::Display;
        assert!(capture_output_presentation_authority(
            "Spout",
            protocol::VideoOutputKind::SpoutSender,
            &wrong_kind,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &injected_spout_endpoint(output_id),
        )
        .expect_err("wrong output kind must reject the bound Spout route")
        .contains("not SpoutSender"));
    }

    #[test]
    fn spout_current_blackout_overrides_visible_historical_follow_snapshots() {
        let output_id = 17;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output_extent(&engine, output_id, 4, 2);
        let mut follow = follow_render_snapshot(true);
        bind_follow_to_spout_route(&mut follow, output_id);
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
                "Spout current project blackout did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        let authority = injected_spout_authority(&engine, output_id);
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
    fn spout_follow_retirement_or_generation_rollover_rejects_before_any_send() {
        let mut follow = follow_render_snapshot(true);
        let output_id = follow.outgoing_video.outputs[0].id;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output_extent(&engine, output_id, 4, 2);
        let authority = injected_spout_authority(&engine, output_id);
        bind_follow_to_spout_route(&mut follow, output_id);
        follow.epoch = authority.ownership_epoch();
        let identity = timeline_follow_active_identity(&follow);
        let mut rollover = follow.clone();
        rollover.generation = rollover.generation.saturating_add(1);
        assert!(
            validate_timeline_follow_active_identity("Spout", identity, Some(&rollover))
                .expect_err("Follow generation rollover must reject the old render")
                .contains("identity changed")
        );

        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let sends_for_attempt = Arc::clone(&sends);
        let error =
            send_frame_if_authorized("Spout", &engine, &authority, Some(identity), move || {
                sends_for_attempt.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
                Ok(())
            })
            .expect_err("retired Follow must reject before entering the Spout SDK closure");
        assert!(
            matches!(&error, PhysicalOutputSendError::Revoked(_)),
            "a pre-send Follow retirement is a discard/re-render, not an SDK fault: {error}"
        );
        assert!(error.to_string().contains("was retired"), "{error}");
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 0);
    }

    #[test]
    fn spout_transport_rejects_nonfresh_or_zero_area_and_materializes_hard_blackout_exactly() {
        let output_id = 98_200;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let mut authority = injected_spout_authority(&engine, output_id);
        let mut overridden_mapping = authority.output_summary().mapping.clone();
        overridden_mapping.black_level = 0.9;
        authority.override_output_mapping_for_test(overridden_mapping);
        let frame = materialize_output_artistic_result(
            spout_hard_blackout_artistic_result(
                &authority,
                video::VideoOutputRenderFreshness::Fresh,
            ),
            &authority,
        )
        .expect("hard blackout must reach the Spout boundary as an admitted frame");
        assert_eq!(frame.data, vec![0, 0, 0, 255]);

        for freshness in [
            video::VideoOutputRenderFreshness::Error,
            video::VideoOutputRenderFreshness::LastValid,
        ] {
            let error = materialize_output_artistic_result(
                spout_hard_blackout_artistic_result(&authority, freshness),
                &authority,
            )
            .expect_err("non-fresh render evidence must not create Spout send bytes");
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
                kind: protocol::VideoOutputKind::SpoutSender,
                enabled: true,
                composition_id: 1,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 2,
                height: 0,
                endpoint_name: Some(injected_spout_endpoint(output_id)),
                opacity: 1.0,
                blackout: false,
                mapping: protocol::VideoOutputMapping::default(),
            });
        let error = capture_output_presentation_authority(
            "Spout",
            protocol::VideoOutputKind::SpoutSender,
            &zero_area,
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &injected_spout_endpoint(output_id),
        )
        .expect_err("zero-area Spout output must reject before render or send");
        assert!(error.contains("zero-area"), "{error}");
    }

    #[test]
    fn spout_output_authority_races_fail_closed_before_any_send() {
        let output_id = 98_201;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        let mut mapping = authority.output_summary().mapping.clone();
        mapping.black_level = 0.25;
        engine
            .send(engine::EngineCommand::SetVideoOutputMapping { output_id, mapping })
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while engine.snapshot().video.outputs[0].mapping.black_level != 0.25 {
            assert!(
                Instant::now() < deadline,
                "Spout mapping mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "route, mapping, or enablement changed",
        );

        let authority = injected_spout_authority(&engine, output_id);
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
                "Spout disable mutation did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "route, mapping, or enablement changed",
        );

        let output_id = 98_203;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
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
                "Spout output removal did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_spout_authority_rejects_without_send(&engine, &authority, "was removed before send");

        let output_id = 98_204;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::SetAllBlackout(true))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !engine.snapshot().blackout {
            assert!(
                Instant::now() < deadline,
                "Spout safety blackout did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "project safety blackout changed",
        );

        let output_id = 98_202;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap();
        assert_spout_authority_rejects_without_send(&engine, &authority, "authority changed");
        transition.complete().unwrap();
    }

    #[test]
    fn spout_presentation_token_change_after_preparation_revokes_restored_content() {
        let output_id = 98_205;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
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
                "Spout token-hostile mapping mutation did not publish"
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
                "Spout token-hostile mapping revert did not publish"
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

        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "presentation authority token advanced before send",
        );
    }

    #[test]
    fn spout_safety_blackout_latch_cycle_after_preparation_revokes_before_send() {
        let output_id = 98_206;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
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
                "Spout safety blackout latch release did not publish"
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

        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "blackout authority changed before send",
        );
    }

    #[test]
    fn spout_output_identity_substitution_after_preparation_revokes_before_send() {
        let output_id = 98_207;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        engine
            .send(engine::EngineCommand::RemoveVideoOutput(output_id))
            .unwrap();
        let substituted_endpoint = format!("Substituted Spout {output_id}");
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
                "substituted Spout output identity did not publish"
            );
            std::thread::sleep(Duration::from_millis(2));
        }

        // The output ID resolves again, so removal alone cannot fence it:
        // exact route-resource identity must.
        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "bound route resource identity changed",
        );
    }

    #[test]
    fn spout_stable_authority_commits_exactly_one_sdk_send_per_admission() {
        let output_id = 98_208;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let authority = injected_spout_authority(&engine, output_id);
        let sends = Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let first_commit = send_frame_if_authorized("Spout", &engine, &authority, None, || {
            sends.fetch_add(1, std::sync::atomic::Ordering::AcqRel);
            Ok(41_u8)
        })
        .expect("stable authority must commit the prepared frame");
        assert_eq!(first_commit, 41);
        assert_eq!(sends.load(std::sync::atomic::Ordering::Acquire), 1);

        let second_commit = send_frame_if_authorized("Spout", &engine, &authority, None, || {
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

    /// A published, playing timeline plus one owned Spout route. Playback
    /// advances the engine clock naturally on every tick.
    fn playing_timeline_engine(output_id: VideoOutputId) -> EngineHandle {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        transition.complete().unwrap();

        let source = protocol::TimelineSnapshot {
            id: protocol::TimelineId(98_301),
            label: "Spout playing admission source".to_string(),
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
                id: 98_302,
                layer_id: 2,
                path: "memory://spout-playing-admission.wav".to_string(),
                duration_ms: 60_000,
                ..protocol::TimelineAudioClipSummary::default()
            }],
            duration_ms: 60_000,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(vec![source], protocol::TimelineId(98_301), false)
            .unwrap();
        engine
            .send(engine::EngineCommand::SeekTimeline(1_000))
            .unwrap();
        engine
            .send(engine::EngineCommand::SetTimelinePlaying(true))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while engine.snapshot().timeline.position_ms <= 1_000 {
            let snapshot = engine.snapshot();
            assert!(
                Instant::now() < deadline,
                "the playing timeline never advanced past its seek point: playing={}, position_ms={}, duration_ms={}, last_error={:?}",
                snapshot.timeline.playing,
                snapshot.timeline.position_ms,
                snapshot.timeline.duration_ms,
                snapshot.telemetry.last_error,
            );
            std::thread::sleep(Duration::from_millis(2));
        }
        engine
    }

    #[test]
    fn spout_playing_timeline_ticks_admit_frames_but_token_mutations_revoke() {
        let output_id = 98_300;
        let engine = playing_timeline_engine(output_id);

        // Capture an authority mid-playback, then let the transport cadence
        // elapse. Natural per-tick position/transition progress must never
        // revoke the frame; only the engine-issued presentation token can.
        let authority = injected_spout_authority(&engine, output_id);
        std::thread::sleep(Duration::from_millis(120));
        assert_eq!(
            engine.snapshot().timeline.follow_runtime.status,
            protocol::TimelineFollowRuntimeStatus::Idle,
            "this scenario is plain playback with no Follow hold"
        );
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
        send_frame_if_authorized("Spout", &engine, &authority, None, move || {
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
                "Spout admission mapping mutation did not publish"
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
                "Spout admission mapping revert did not publish"
            );
            if engine.snapshot().video.outputs[0].mapping.black_level
                == authority.output_summary().mapping.black_level
            {
                break;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_spout_authority_rejects_without_send(
            &engine,
            &authority,
            "presentation authority token advanced before send",
        );
    }

    fn run_injected_spout_failure(render_failure: bool) {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, 701);
        let authority = injected_spout_authority(&engine, 701);
        let drop_started = Arc::new(AtomicBool::new(false));
        let allow_drop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let sender = InjectedSpoutSender {
            send_error: (!render_failure).then(|| "injected Spout send failure".to_string()),
            drop_started: Arc::clone(&drop_started),
            allow_drop: Arc::clone(&allow_drop),
            dropped: Arc::clone(&dropped),
        };
        let worker_stop = Arc::new(AtomicBool::new(false));
        let teardown_lease_slot = Arc::new(Mutex::new(None));
        let render_failure_lease = Arc::new(Mutex::new(None));
        let render_error = render_failure.then(|| "injected Spout render failure".to_string());
        let worker_engine = engine.clone();
        let worker_stop_for_thread = Arc::clone(&worker_stop);
        let worker_teardown_lease_slot = Arc::clone(&teardown_lease_slot);
        let worker = std::thread::spawn(move || {
            run_spout_output_worker(
                701,
                "Injected Spout",
                worker_engine,
                worker_stop_for_thread,
                sender,
                (worker_teardown_lease_slot, render_failure_lease),
                move |_| match &render_error {
                    Some(error) => Err(error.clone()),
                    None => Ok((
                        injected_spout_render_decision(
                            authority.output_summary().id,
                            authority.ownership_epoch(),
                        ),
                        Some(authority.clone()),
                    )),
                },
            )
        });

        while !drop_started.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        let status_while_teardown = engine.output_ownership_status();
        let retry_blocked_while_teardown = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err();
        allow_drop.store(true, Ordering::Release);
        let worker_result = worker.join().unwrap();
        let retry_blocked_until_join_ack = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err();
        let teardown_lease = teardown_lease_slot.lock().unwrap().take();
        drop(teardown_lease);

        assert!(worker_result.is_err());
        assert_eq!(
            status_while_teardown.state,
            protocol::OutputOwnershipState::Failed
        );
        assert!(!status_while_teardown.video_allowed);
        assert!(status_while_teardown.error.as_deref().is_some_and(|error| {
            error.contains(if render_failure {
                "render failed"
            } else {
                "send failed"
            })
        }));
        assert!(retry_blocked_while_teardown);
        assert!(retry_blocked_until_join_ack);
        assert!(
            dropped.load(Ordering::Acquire),
            "Spout sender was not dropped"
        );

        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
        assert!(engine.acquire_video_output().is_ok());
    }

    #[test]
    fn injected_spout_render_failure_fences_until_sender_teardown_acknowledged() {
        run_injected_spout_failure(true);
    }

    #[test]
    fn inner_render_fault_ack_retry_transfers_one_fence_through_sender_teardown() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let initial_epoch = engine.output_ownership_status().epoch;
        let drop_started = Arc::new(AtomicBool::new(false));
        let allow_drop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let sender = InjectedSpoutSender {
            send_error: None,
            drop_started: Arc::clone(&drop_started),
            allow_drop: Arc::clone(&allow_drop),
            dropped: Arc::clone(&dropped),
        };
        let worker_stop = Arc::new(AtomicBool::new(false));
        let teardown_lease_slot = Arc::new(Mutex::new(None));
        let render_failure_lease = Arc::new(Mutex::new(None));
        let worker_engine = engine.clone();
        let render_engine = engine.clone();
        let worker = std::thread::spawn({
            let worker_stop = Arc::clone(&worker_stop);
            let teardown_lease_slot = Arc::clone(&teardown_lease_slot);
            let render_failure_lease = Arc::clone(&render_failure_lease);
            move || {
                run_spout_output_worker(
                    705,
                    "Inner render-failure Spout",
                    worker_engine,
                    worker_stop,
                    sender,
                    (teardown_lease_slot, render_failure_lease.clone()),
                    move |follow_output_state| {
                        let fault = "Spout output route 705 render failed: injected inner failure"
                            .to_string();
                        let key = TimelineFollowOutputFrameKey {
                            epoch: render_engine.output_ownership_status().epoch,
                            generation: 55,
                            output_id: 705,
                            width: 1,
                            height: 1,
                        };
                        let (failure_lease, failed_key) = fence_timeline_follow_fault_before_ack(
                            &render_engine,
                            follow_output_state,
                            key,
                            fault.clone(),
                        );
                        // Exercise the actual inner Fault ACK retry before
                        // handing its already-active fence to the worker.
                        publish_timeline_follow_output_result_until_resolved(
                            &render_engine,
                            follow_output_state,
                            failed_key,
                            TimelineFollowSettlementAckResult::Fault {
                                fault: fault.clone(),
                            },
                            || false,
                        );
                        match render_failure_lease.lock() {
                            Ok(mut slot) => *slot = Some(failure_lease),
                            Err(poisoned) => *poisoned.into_inner() = Some(failure_lease),
                        }
                        Err(fault)
                    },
                )
            }
        });

        while !drop_started.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        let status_during_sender_drop = engine.output_ownership_status();
        assert_eq!(
            status_during_sender_drop.epoch,
            initial_epoch + 1,
            "inner render fence must transfer instead of being dropped and reacquired"
        );
        assert_eq!(
            status_during_sender_drop.state,
            protocol::OutputOwnershipState::Failed
        );
        assert!(engine.acquire_video_output().is_err());
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());

        allow_drop.store(true, Ordering::Release);
        assert!(worker.join().unwrap().is_err());
        assert!(dropped.load(Ordering::Acquire));
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());
        drop(teardown_lease_slot.lock().unwrap().take());

        let rearmed = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        rearmed.complete().unwrap();
    }

    #[test]
    fn spout_startup_timeout_and_late_constructor_error_share_one_fence() {
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
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (return_error_tx, return_error_rx) = mpsc::sync_channel(1);
        let (failed_epoch_tx, failed_epoch_rx) = mpsc::sync_channel(1);
        let worker = std::thread::spawn(move || {
            entered_tx.send(()).unwrap();
            return_error_rx.recv().unwrap();
            let mut state = TimelineFollowOutputState::default();
            let failed_key = fence_timeline_follow_fault_before_ack_in_startup(
                &worker_engine,
                &mut state,
                TimelineFollowOutputFrameKey {
                    epoch: initial_epoch,
                    generation: 67,
                    output_id: 707,
                    width: 1,
                    height: 1,
                },
                "late Spout constructor error".to_string(),
                &worker_slot,
            );
            creation_lease.retire();
            failed_epoch_tx.send(failed_key.epoch).unwrap();
            Err::<(), String>("late Spout constructor error".to_string())
        });
        entered_rx.recv().unwrap();

        let timeout_epoch = ensure_startup_failure_fence(
            &engine,
            &startup_failure_lease,
            "Spout startup constructor timeout",
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

    #[test]
    fn injected_spout_send_failure_fences_while_frame_permit_is_held() {
        run_injected_spout_failure(false);
    }

    #[test]
    fn spout_persistent_retry_faults_at_bounded_settlement_deadline() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        activation.complete().unwrap();
        let output_id = 98_310;
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
            id: protocol::TimelineId(98_311),
            label: "Spout deadline Follow source".to_string(),
            layers: vec![audio_layer.clone()],
            audio_clips: vec![protocol::TimelineAudioClipSummary {
                id: 98_312,
                layer_id: 2,
                media_asset_id: None,
                path: "memory://spout-deadline-follow.wav".to_string(),
                start_ms: 0,
                offset_ms: 0,
                duration_ms: 100,
                gain: 1.0,
                fade_in_ms: 0,
                fade_out_ms: 0,
            }],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        source.follow = Some(protocol::TimelineFollowSummary {
            enabled: true,
            next_timeline_id: protocol::TimelineId(98_313),
            duration: protocol::VideoClipTakeDuration::milliseconds(10),
            curve: protocol::VideoLayerTransitionCurve::Linear,
            video_kind: protocol::VideoClipTakeKind::Crossfade,
            lighting_policy: protocol::TimelineFollowLightingPolicy::LinearMerge,
            destination_bpm: None,
            preroll_ms: 0,
            trans_cadence_bars: 4,
            trans_target_measures: Vec::new(),
            fault_policy: protocol::TimelineFollowFaultPolicy::Hold,
        });
        let target = protocol::TimelineSnapshot {
            id: protocol::TimelineId(98_313),
            label: "Spout deadline Follow target".to_string(),
            layers: vec![audio_layer],
            duration_ms: 100,
            ..protocol::TimelineSnapshot::default()
        };
        engine
            .apply_timeline_bank_published(
                vec![source, target],
                protocol::TimelineId(98_311),
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
                "Spout deadline Follow never entered settlement"
            );
            std::thread::sleep(Duration::from_millis(10));
        };
        let retry_key = timeline_follow_output_key(&follow, output_id).unwrap();

        let drop_started = Arc::new(AtomicBool::new(false));
        let allow_drop = Arc::new(AtomicBool::new(true));
        let dropped = Arc::new(AtomicBool::new(false));
        let sender = InjectedSpoutSender {
            send_error: None,
            drop_started: Arc::clone(&drop_started),
            allow_drop: Arc::clone(&allow_drop),
            dropped: Arc::clone(&dropped),
        };
        let worker_engine = engine.clone();
        let worker = std::thread::spawn(move || {
            run_spout_output_worker(
                output_id,
                "Deadline Spout",
                worker_engine,
                Arc::new(AtomicBool::new(false)),
                sender,
                (Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None))),
                move |follow_output_state| {
                    // Every observation of this generation is superseded
                    // before it could admit: the bounded-settlement watchdog
                    // must convert the endless retry into a visible Fault.
                    follow_output_state.set_settlement_deadline(Duration::from_millis(80));
                    Ok((
                        TimelineFollowOutputRenderDecision::Retry {
                            reason: "injected persistent supersession".to_string(),
                            key: Some(retry_key),
                        },
                        None,
                    ))
                },
            )
        });

        let worker_result = worker.join().unwrap();
        let error = worker_result.expect_err("endless retries must fault at the deadline");
        assert!(
            error.message.contains("bounded settlement deadline"),
            "the terminal error must name the bounded settlement deadline: {error}"
        );
        assert!(dropped.load(Ordering::Acquire), "the sender must tear down");
        assert_eq!(
            engine.output_ownership_status().state,
            OutputOwnershipState::Failed,
            "deadline expiry must fail ownership visibly"
        );
        let runtime = engine.snapshot().timeline.follow_runtime;
        assert_eq!(runtime.status, protocol::TimelineFollowRuntimeStatus::Held);
        assert!(matches!(
            runtime.outcome,
            Some(protocol::TimelineFollowOutcome::Held)
        ));
    }

    #[test]
    fn spout_stuck_video_admission_faults_visibly_instead_of_spinning() {
        let output_id = 98_320;
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, output_id);
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        transition.complete().unwrap();
        let authority = injected_spout_authority(&engine, output_id);
        let stuck_frame_fence =
            engine.begin_output_ownership_failure_fence("injected stuck video admission");

        let drop_started = Arc::new(AtomicBool::new(false));
        let allow_drop = Arc::new(AtomicBool::new(true));
        let dropped = Arc::new(AtomicBool::new(false));
        let sender = InjectedSpoutSender {
            send_error: None,
            drop_started: Arc::clone(&drop_started),
            allow_drop: Arc::clone(&allow_drop),
            dropped: Arc::clone(&dropped),
        };
        let worker_engine = engine.clone();
        let worker = std::thread::spawn(move || {
            run_spout_output_worker(
                output_id,
                "Stuck Admission Spout",
                worker_engine,
                Arc::new(AtomicBool::new(false)),
                sender,
                (Arc::new(Mutex::new(None)), Arc::new(Mutex::new(None))),
                move |_| {
                    Ok((
                        injected_spout_render_decision(
                            authority.output_summary().id,
                            authority.ownership_epoch(),
                        ),
                        Some(authority.clone()),
                    ))
                },
            )
        });

        let error = worker
            .join()
            .unwrap()
            .expect_err("stuck ownership admission must fault within its deadline");
        assert!(
            error.message.contains("admission deadline"),
            "the terminal error must name the bounded admission deadline: {error}"
        );
        drop(stuck_frame_fence);

        let teardown_lease = match engine.begin_output_ownership_teardown() {
            Ok(lease) => Some(lease),
            Err(_) => None,
        };
        drop(teardown_lease);
        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
        assert!(engine.acquire_video_output().is_ok());
    }

    #[test]
    fn normal_spout_stop_during_all_deny_transition_completes_successfully() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap();
        let allow_drop = Arc::new(AtomicBool::new(true));
        let dropped = Arc::new(AtomicBool::new(false));
        let sender = InjectedSpoutSender {
            send_error: None,
            drop_started: Arc::new(AtomicBool::new(false)),
            allow_drop,
            dropped: Arc::clone(&dropped),
        };
        let worker_stop = Arc::new(AtomicBool::new(true));
        let teardown_lease_slot = Arc::new(Mutex::new(None));
        let result = run_spout_output_worker(
            703,
            "Normal stop Spout",
            engine.clone(),
            worker_stop,
            sender,
            (Arc::clone(&teardown_lease_slot), Arc::new(Mutex::new(None))),
            |_| {
                Ok((
                    injected_spout_render_decision(703, engine.output_ownership_status().epoch),
                    None,
                ))
            },
        );

        assert!(result.is_ok());
        assert!(dropped.load(Ordering::Acquire));
        drop(teardown_lease_slot.lock().unwrap().take());
        let status = transition.complete().unwrap();
        assert_eq!(status.state, protocol::OutputOwnershipState::Ready);
        assert_eq!(status.effective_role, protocol::MachineOutputRole::Standby);
    }

    #[test]
    fn spout_state_harvests_failed_worker_before_route_can_be_rearmed() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        install_injected_spout_output(&engine, 704);
        let authority = injected_spout_authority(&engine, 704);
        let drop_started = Arc::new(AtomicBool::new(false));
        let allow_drop = Arc::new(AtomicBool::new(false));
        let dropped = Arc::new(AtomicBool::new(false));
        let worker_failure = Arc::new(Mutex::new(None));
        let teardown_lease = Arc::new(Mutex::new(None));
        let worker_stop = Arc::new(AtomicBool::new(false));
        let worker_stop_for_thread = Arc::clone(&worker_stop);
        let worker_failure_for_thread = Arc::clone(&worker_failure);
        let worker_teardown_lease = Arc::clone(&teardown_lease);
        let worker_engine = engine.clone();
        let drop_started_for_thread = Arc::clone(&drop_started);
        let allow_drop_for_thread = Arc::clone(&allow_drop);
        let dropped_for_thread = Arc::clone(&dropped);
        let worker = std::thread::spawn(move || {
            let sender = InjectedSpoutSender {
                send_error: Some("injected Spout state send failure".to_string()),
                drop_started: drop_started_for_thread,
                allow_drop: allow_drop_for_thread,
                dropped: dropped_for_thread,
            };
            let result = run_spout_output_worker(
                704,
                "State Spout",
                worker_engine,
                worker_stop_for_thread,
                sender,
                (worker_teardown_lease, Arc::new(Mutex::new(None))),
                |_| {
                    Ok((
                        injected_spout_render_decision(
                            authority.output_summary().id,
                            authority.ownership_epoch(),
                        ),
                        Some(authority.clone()),
                    ))
                },
            );
            if let Err(error) = &result {
                record_spout_worker_failure(&worker_failure_for_thread, error.clone());
            }
            result
        });
        while !drop_started.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        assert_eq!(
            engine.output_ownership_status().state,
            protocol::OutputOwnershipState::Failed
        );
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());
        allow_drop.store(true, Ordering::Release);
        worker.join().unwrap().unwrap_err();

        let route = video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: 704,
            label: "State Spout".to_string(),
            backend_id: "spout".to_string(),
            endpoint_name: "State Spout".to_string(),
        };
        let mut transport = SpoutTransportState::new(
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        );
        transport.output_workers.insert(
            route.route_id,
            SpoutRouteWorker {
                stop: worker_stop,
                worker: None,
                failure: worker_failure,
                teardown_lease,
                start_signal: None,
                creation_lease: None,
            },
        );
        transport.stop_route(&route, &engine).unwrap();
        assert!(transport.failed_output_routes.contains_key(&route.route_id));
        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
        assert!(engine.acquire_video_output().is_ok());
    }

    #[test]
    fn injected_spout_worker_spawn_failure_constructs_no_sender() {
        let _hook_guard = SPOUT_TEST_HOOK_LOCK.lock().unwrap();
        SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS.store(0, Ordering::Release);
        SPOUT_WORKER_SPAWN_FAILURE.store(true, Ordering::Release);
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let result = SpoutRouteWorker::start_output(
            702,
            "Spawn failure Spout".to_string(),
            engine,
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
        );

        let error = match result {
            Ok(_) => panic!("injected worker spawn failure must reject the route"),
            Err(error) => error,
        };
        assert!(error
            .message
            .contains("injected Spout output worker spawn failure"));
        assert_eq!(
            SPOUT_OUTPUT_CONSTRUCTION_ATTEMPTS.load(Ordering::Acquire),
            0,
            "sender creation must remain inside the worker and never run after spawn failure"
        );
    }

    #[test]
    fn startup_timeout_is_bounded_and_delayed_constructor_drop_blocks_duplicate_sender_creation() {
        struct StartupResource(Arc<AtomicBool>);

        impl Drop for StartupResource {
            fn drop(&mut self) {
                self.0.store(true, Ordering::Release);
            }
        }

        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let (allow_constructor_tx, allow_constructor_rx) = mpsc::sync_channel(1);
        let (constructor_entered_tx, constructor_entered_rx) = mpsc::sync_channel(1);
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_tx, start_rx) = mpsc::sync_channel::<SpoutOutputStartDecision>(1);
        let constructor_attempts = Arc::new(AtomicUsize::new(0));
        let resource_destroyed = Arc::new(AtomicBool::new(false));
        let attempts_for_thread = Arc::clone(&constructor_attempts);
        let destroyed_for_thread = Arc::clone(&resource_destroyed);
        let worker = std::thread::spawn(move || {
            attempts_for_thread.fetch_add(1, Ordering::AcqRel);
            constructor_entered_tx.send(()).unwrap();
            allow_constructor_rx.recv().unwrap();
            let resource = StartupResource(destroyed_for_thread);
            let _ = ready_tx.send(Ok(()));
            if start_rx.recv().is_err() {
                drop(resource);
                creation_lease.retire();
            }
            Ok::<(), SpoutOutputWorkerStopError>(())
        });

        constructor_entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("delayed constructor worker must enter before timeout is measured");

        assert!(matches!(
            ready_rx.recv_timeout(Duration::from_millis(1)),
            Err(mpsc::RecvTimeoutError::Timeout)
        ));
        drop(ready_rx);
        drop(start_tx);
        let failure_lease = engine.begin_output_ownership_failure_fence(
            "Spout startup constructor timed out; sender cleanup remains pending",
        );
        let teardown_lease = Arc::new(Mutex::new(None));
        let pending = SpoutPendingOutputStartup {
            worker: Some(worker),
            failure_lease: Some(failure_lease),
            teardown_lease: Arc::clone(&teardown_lease),
            message: "Spout startup constructor is still running".to_string(),
        };
        let mut transport = SpoutTransportState::new(
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        );
        transport.pending_output_startups.insert(77, pending);
        let route = video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: 77,
            label: "Delayed Spout".to_string(),
            backend_id: "spout".to_string(),
            endpoint_name: "Delayed Spout".to_string(),
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
    fn startup_publication_disconnect_retires_synchronous_sender_drop_before_retry() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        // Synchronize this test's private EngineHandle with its first
        // published persistence image.  Without the command acknowledgement,
        // a full-suite run can race the startup ownership image while this
        // test is fencing the disconnected sender.
        engine
            .persistence_snapshot()
            .expect("Spout test engine must acknowledge startup publication");
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let creation_lease = activation.admit_resource_creation().unwrap();
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<(), String>>(1);
        let (start_tx, start_rx) = mpsc::sync_channel::<SpoutOutputStartDecision>(1);
        let worker = std::thread::spawn(move || {
            ready_tx.send(Ok(())).unwrap();
            if start_rx.recv().is_err() {
                creation_lease.retire();
            }
            Ok::<(), SpoutOutputWorkerStopError>(())
        });
        ready_rx.recv().unwrap().unwrap();
        drop(ready_rx);
        drop(start_tx);
        let failure_lease = engine.begin_output_ownership_failure_fence(
            "Spout publication handshake disconnected during startup",
        );
        let mut pending = SpoutPendingOutputStartup {
            worker: Some(worker),
            failure_lease: Some(failure_lease),
            teardown_lease: Arc::new(Mutex::new(None)),
            message: "Spout publication handshake is pending".to_string(),
        };
        while !pending.poll() {
            std::thread::yield_now();
        }
        // Make the worker/failure lease cleanup explicit before attempting
        // the next ownership transition; the retry must never rely on a
        // scope drop that can be delayed by the test harness.
        drop(pending);
        let retry = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .unwrap();
        retry.complete().unwrap();
    }

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
        let _ = worker.stop();

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
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig::default());
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
                    monitor_identity: None,
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
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let mut worker = SpoutRouteWorker::start_output(
            9,
            name.clone(),
            engine,
            Arc::new(Mutex::new(HashMap::new())),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
        )
        .unwrap();
        worker.publish().unwrap();
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
        let _ = worker.stop();

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
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig::default());
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
                    monitor_identity: None,
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
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let mut worker = SpoutRouteWorker::start_output(
            output_id,
            sender_name.clone(),
            engine.clone(),
            Arc::clone(&spout_inputs),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
        )
        .unwrap();
        worker.publish().unwrap();

        eprintln!("SPOUT_EXTERNAL_OUTPUT_READY {sender_name} 640x360");
        wait_for_spout_operator_confirmation(&confirmation_path, "connected");
        engine
            .send(engine::EngineCommand::SetVideoOutputConfig {
                output_id,
                label: "External Spout QA".to_string(),
                kind: protocol::VideoOutputKind::SpoutSender,
                fullscreen: false,
                monitor_id: None,
                monitor_identity: None,
                width: 1280,
                height: 720,
                endpoint_name: Some(sender_name),
            })
            .unwrap();
        eprintln!("SPOUT_EXTERNAL_OUTPUT_RESIZED 1280x720");
        wait_for_spout_operator_confirmation(&confirmation_path, "resized");
        let _ = worker.stop();
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
        let _ = worker.stop();

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
        let _ = worker.stop();
        eprintln!("SPOUT_RECONNECT_RECOVERED");
    }
}
