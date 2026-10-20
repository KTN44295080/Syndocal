//! macOS Syphon output. Native senders live and retire on their owner thread.
//! Shared authority/Follow helpers and the common pacer preserve the same
//! physical-output admission boundary as Spout and NDI.

use crate::syphon_output_policy::{validate_syphon_config, SyphonOutputLifecycle};
use engine::{
    EngineHandle, OutputOwnershipActivation, OutputOwnershipCreationLease,
    OutputOwnershipTeardownLease,
};
use protocol::TimelineFollowSettlementAckResult;
use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[path = "physical_output_fence.rs"]
mod physical_output_fence;
use physical_output_fence::*;

pub struct SyphonTransportState {
    output_workers: HashMap<u64, SyphonRouteWorker>,
    pending_output_startups: HashMap<u64, SyphonPendingOutputStartup>,
    failed_output_routes: HashMap<u64, String>,
    #[cfg(feature = "ndi")]
    ndi_inputs: crate::ndi_transport::NdiInputRegistry,
    capture_inputs: crate::capture_transport::CaptureInputRegistry,
}

impl SyphonTransportState {
    pub fn new(
        #[cfg(feature = "ndi")] ndi_inputs: crate::ndi_transport::NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
    ) -> Self {
        Self {
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
            let finished = pending.poll();
            self.failed_output_routes
                .insert(route_id, pending.message.clone());
            if finished {
                self.pending_output_startups.remove(&route_id);
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
                        .is_some_and(std::thread::JoinHandle::is_finished))
                .then_some(*route_id)
            })
            .collect::<Vec<_>>();
        for route_id in failed_ids {
            let Some(worker) = self.output_workers.remove(&route_id) else {
                continue;
            };
            let failure = worker.failure_snapshot();
            let stop_result = worker.stop();
            if let Some(failure) = failure.or_else(|| stop_result.err()) {
                if engine.output_ownership_status().state != protocol::OutputOwnershipState::Failed
                {
                    drop(engine.begin_output_ownership_failure_fence(format!(
                        "Syphon output route {route_id} worker teardown failed: {}",
                        failure.message
                    )));
                }
                self.failed_output_routes.insert(route_id, failure.message);
            }
        }
        Ok(())
    }

    #[cfg(all(feature = "syphon", target_os = "macos"))]
    pub fn start_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        activation: Option<OutputOwnershipActivation>,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.start_route_with_factory(
            route,
            engine,
            activation,
            crate::syphon_native::SyphonSender::new,
        )
    }

    #[cfg(all(feature = "syphon", target_os = "macos"))]
    pub fn start_route_for_authoring(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        activation: Option<OutputOwnershipActivation>,
        authority: &dyn Fn() -> Result<(), String>,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.start_route_with_factory_and_authority(
            route, engine, activation, crate::syphon_native::SyphonSender::new,
            Some(authority),
        )
    }

    fn start_route_with_factory<S, F>(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        activation: Option<OutputOwnershipActivation>,
        factory: F,
    ) -> Result<(), video::ExternalVideoTransportDriverError>
    where
        S: SyphonOutputSender + 'static,
        F: FnOnce(&str) -> Result<S, String> + Send + 'static,
    {
        self.start_route_with_factory_and_authority(route, engine, activation, factory, None)
    }

    fn start_route_with_factory_and_authority<S, F>(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        activation: Option<OutputOwnershipActivation>,
        factory: F,
        authority: Option<&dyn Fn() -> Result<(), String>>,
    ) -> Result<(), video::ExternalVideoTransportDriverError>
    where
        S: SyphonOutputSender + 'static,
        F: FnOnce(&str) -> Result<S, String> + Send + 'static,
    {
        if let Some(validate) = authority {
            validate().map_err(driver_error)?;
        }
        self.harvest_failed_workers(engine)?;
        if route.direction != video::ExternalVideoTransportDirection::Output
            || route.backend_id != "syphon"
        {
            return Err(driver_error(
                "Syphon transport accepts Syphon output routes only",
            ));
        }
        if self.output_workers.contains_key(&route.route_id)
            || self.pending_output_startups.contains_key(&route.route_id)
            || self
                .output_workers
                .values()
                .any(|worker| worker.endpoint_name == route.endpoint_name)
            || self
                .pending_output_startups
                .values()
                .any(|pending| pending.sender_name == route.endpoint_name)
        {
            return Err(driver_error(
                "Syphon output id or server name is awaiting prior teardown acknowledgement",
            ));
        }
        let snapshot = engine.snapshot();
        let output = snapshot
            .video
            .outputs
            .iter()
            .find(|output| output.id == route.route_id)
            .ok_or_else(|| driver_error("Syphon output configuration is missing"))?;
        if output.kind != protocol::VideoOutputKind::SyphonServer
            || output.endpoint_name.as_deref() != Some(route.endpoint_name.as_str())
            || !output.enabled
        {
            return Err(driver_error(
                "Syphon output route does not match its enabled configuration",
            ));
        }
        validate_syphon_config(&route.endpoint_name, output.width, output.height)
            .map_err(driver_error)?;
        let activation = activation
            .ok_or_else(|| driver_error("Syphon output resource activation was not admitted"))?;
        self.failed_output_routes.remove(&route.route_id);
        if let Some(validate) = authority {
            validate().map_err(driver_error)?;
        }
        let worker = match SyphonRouteWorker::start_output_with_factory(
            route.route_id,
            route.endpoint_name.clone(),
            engine.clone(),
            #[cfg(feature = "ndi")]
            Arc::clone(&self.ndi_inputs),
            Arc::clone(&self.capture_inputs),
            activation,
            factory,
        ) {
            Ok(worker) => worker,
            Err(error) => {
                if let Some(pending) = error.pending_startup {
                    self.pending_output_startups.insert(route.route_id, pending);
                }
                return Err(driver_error(error.message));
            }
        };
        self.output_workers.insert(route.route_id, worker);
        let publish_result = authority.map_or(Ok(()), |validate| validate()).and_then(|()| {
            self.output_workers
                .get_mut(&route.route_id)
                .expect("inserted Syphon worker")
                .publish()
        });
        if let Err(error) = publish_result {
            let worker = self
                .output_workers
                .remove(&route.route_id)
                .expect("inserted Syphon worker");
            if let Err(stop_error) = worker.stop() {
                self.failed_output_routes
                    .insert(route.route_id, stop_error.message.clone());
                return Err(driver_error(format!(
                    "Syphon output creation was invalidated and teardown failed: {}",
                    stop_error.message
                )));
            }
            return Err(driver_error(format!(
                "Syphon output creation was invalidated before publication: {error}"
            )));
        }
        Ok(())
    }

    pub fn stop_route(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        self.stop_route_with_authority(route, engine, None)
    }

    /// Retire only the authoring target under a counted resource lease. The
    /// ordinary stop API continues to require a global ownership transition.
    pub fn stop_route_for_authoring(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        authority: &dyn Fn() -> Result<(), String>,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        if route.direction != video::ExternalVideoTransportDirection::Output
            || route.backend_id != "syphon"
        {
            return Err(video::ExternalVideoTransportDriverError {
                message: "Syphon authoring accepts only its output routes".to_string(),
            });
        }
        self.stop_route_with_authority(route, engine, Some(authority))
    }

    fn stop_route_with_authority(
        &mut self,
        route: &video::ExternalVideoTransportRoute,
        engine: &EngineHandle,
        authority: Option<&dyn Fn() -> Result<(), String>>,
    ) -> Result<(), video::ExternalVideoTransportDriverError> {
        if let Some(validate) = authority {
            validate().map_err(driver_error)?;
        }
        self.harvest_failed_workers(engine)?;
        if route.direction != video::ExternalVideoTransportDirection::Output
            || route.backend_id != "syphon"
        {
            return Err(driver_error(
                "Syphon transport accepts Syphon output routes only",
            ));
        }
        if authority.is_none() && !matches!(
            engine.output_ownership_status().state,
            protocol::OutputOwnershipState::Transitioning
                | protocol::OutputOwnershipState::Activating
                | protocol::OutputOwnershipState::Failed
        ) {
            return Err(driver_error(
                "Syphon output teardown requires the serialized ownership transition",
            ));
        }
        if let Some(validate) = authority {
            validate().map_err(driver_error)?;
            if let Some(worker) = self.output_workers.get(&route.route_id) {
                worker.admit_authoring_retirement(engine).map_err(driver_error)?;
            }
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
                        drop(engine.begin_output_ownership_failure_fence(error.message.clone()));
                    }
                    self.failed_output_routes
                        .insert(route.route_id, error.message.clone());
                    return Err(driver_error(error.message));
                }
            }
        }
        if let Some(pending) = self.pending_output_startups.get(&route.route_id) {
            return Err(driver_error(format!(
                "Syphon startup cleanup is still pending: {}",
                pending.message
            )));
        }
        Ok(())
    }
}

fn syphon_output_effect_render_context(
    snapshot: &protocol::EngineSnapshot,
    project_render_epoch: u64,
) -> video::VideoEffectRenderContext<'_> {
    video::VideoEffectRenderContext {
        clip_runtime: &snapshot.video_clip_runtime,
        audio_frame: snapshot.live_audio_textures.as_deref(),
        project_render_epoch,
    }
}

#[derive(Clone, Debug)]
pub(crate) struct SyphonOutputWorkerStopError {
    pub(crate) message: String,
}

impl std::fmt::Display for SyphonOutputWorkerStopError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

struct SyphonOutputWorkerStartError {
    message: String,
    pending_startup: Option<SyphonPendingOutputStartup>,
}

impl std::fmt::Debug for SyphonOutputWorkerStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("SyphonOutputWorkerStartError")
            .field("message", &self.message)
            .field("has_pending_startup", &self.pending_startup.is_some())
            .finish()
    }
}

struct SyphonPendingOutputStartup {
    sender_name: String,
    worker: Option<std::thread::JoinHandle<Result<(), SyphonOutputWorkerStopError>>>,
    failure_lease: Option<OutputOwnershipTeardownLease>,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    message: String,
}

impl SyphonPendingOutputStartup {
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
                Err(_) => self.message = "Syphon output startup worker panicked".to_string(),
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

impl Drop for SyphonPendingOutputStartup {
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
        if self.worker.is_none() && self.failure_lease.is_none() && teardown_lease.is_none() {
            return;
        }
        let payload = Arc::new(Mutex::new(Some((
            self.worker.take(),
            self.failure_lease.take(),
            teardown_lease,
        ))));
        let worker_payload = Arc::clone(&payload);
        let reaper = std::thread::Builder::new()
            .name("syndocal-syphon-startup-reaper".to_string())
            .spawn(move || {
                let Some((worker, failure_lease, teardown_lease)) = worker_payload
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner())
                    .take()
                else {
                    return;
                };
                if let Some(worker) = worker {
                    match worker.join() {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => eprintln!(
                            "Syphon pending startup reaper observed sender cleanup failure: {error}"
                        ),
                        Err(_) => eprintln!(
                            "Syphon pending startup reaper observed a sender worker panic during cleanup"
                        ),
                    }
                }
                drop(teardown_lease);
                drop(failure_lease);
            });
        if reaper.is_err() {
            eprintln!(
                "Syphon pending startup cleanup reaper could not start; retaining its sender worker and ownership leases"
            );
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

type SyphonCreationLeaseSlot = Arc<Mutex<Option<OutputOwnershipCreationLease>>>;

fn take_syphon_creation_lease(
    slot: &SyphonCreationLeaseSlot,
) -> Option<OutputOwnershipCreationLease> {
    slot.lock()
        .map(|mut lease| lease.take())
        .unwrap_or_else(|poisoned| poisoned.into_inner().take())
}

trait SyphonOutputSender {
    fn send_rgba(&mut self, width: u32, height: u32, pixels: &[u8]) -> Result<(), String>;
    fn close(&mut self) -> Result<(), String>;
}

#[cfg(all(feature = "syphon", target_os = "macos"))]
impl SyphonOutputSender for crate::syphon_native::SyphonSender {
    fn send_rgba(&mut self, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
        crate::syphon_native::SyphonSender::send_rgba(self, width, height, pixels)
    }
    fn close(&mut self) -> Result<(), String> {
        crate::syphon_native::SyphonSender::close(self)
    }
}

struct SyphonOwnedSender<S: SyphonOutputSender> {
    sender: Option<S>,
    engine: EngineHandle,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    retired: bool,
}

impl<S: SyphonOutputSender> SyphonOwnedSender<S> {
    fn new(
        sender: S,
        engine: EngineHandle,
        teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    ) -> Self {
        Self {
            sender: Some(sender),
            engine,
            teardown_lease,
            retired: false,
        }
    }
}

impl<S: SyphonOutputSender> SyphonOutputSender for SyphonOwnedSender<S> {
    fn send_rgba(&mut self, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
        self.sender
            .as_mut()
            .ok_or_else(|| "Syphon sender has already retired".to_string())?
            .send_rgba(width, height, pixels)
    }

    fn close(&mut self) -> Result<(), String> {
        if self.retired {
            return Ok(());
        }
        self.sender
            .as_mut()
            .ok_or_else(|| "Syphon sender has already retired".to_string())?
            .close()?;
        self.retired = true;
        Ok(())
    }
}

impl<S: SyphonOutputSender> Drop for SyphonOwnedSender<S> {
    fn drop(&mut self) {
        let Some(mut sender) = self.sender.take() else {
            return;
        };
        if self.retired {
            drop(sender);
            return;
        }
        // A Rust panic can leave the render/send path before its explicit
        // finish function. Fence before native destruction and retain the
        // acknowledgement until the parent joins this exact owner thread.
        let lease = self.engine.begin_output_ownership_failure_fence(
            "Syphon sender owner exited before acknowledging native teardown",
        );
        match sender.close() {
            Ok(()) => {
                drop(sender);
                let mut slot = self
                    .teardown_lease
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if slot.is_none() {
                    *slot = Some(lease);
                }
            }
            Err(error) => {
                let message = format!("Syphon owner cleanup failed: {error}. Restart Syndocal before re-arming outputs");
                drop(
                    self.engine
                        .begin_output_ownership_failure_fence(message.clone()),
                );
                eprintln!("{message}");
                std::mem::forget(sender);
                std::mem::forget(lease);
            }
        }
    }
}

struct SyphonRouteWorker {
    endpoint_name: String,
    lifecycle: SyphonOutputLifecycle,
    stop: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<Result<(), SyphonOutputWorkerStopError>>>,
    failure: Arc<Mutex<Option<SyphonOutputWorkerStopError>>>,
    teardown_lease: Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    start_signal: Option<mpsc::SyncSender<SyphonOutputStartDecision>>,
    creation_lease: Option<OutputOwnershipCreationLease>,
}

enum SyphonOutputStartDecision {
    Publish,
    Retire(OutputOwnershipCreationLease),
}

fn spawn_syphon_output_worker<F>(
    output_id: u64,
    worker: F,
) -> Result<std::thread::JoinHandle<Result<(), SyphonOutputWorkerStopError>>, String>
where
    F: FnOnce() -> Result<(), SyphonOutputWorkerStopError> + Send + 'static,
{
    std::thread::Builder::new()
        .name(format!("syndocal-syphon-output-{output_id}"))
        .spawn(worker)
        .map_err(|error| error.to_string())
}

fn record_syphon_worker_failure(
    failure: &Arc<Mutex<Option<SyphonOutputWorkerStopError>>>,
    error: SyphonOutputWorkerStopError,
) {
    match failure.lock() {
        Ok(mut slot) => *slot = Some(error),
        Err(poisoned) => *poisoned.into_inner() = Some(error),
    }
}

impl SyphonRouteWorker {
    fn start_output_with_factory<S, F>(
        output_id: u64,
        sender_name: String,
        engine: EngineHandle,
        #[cfg(feature = "ndi")] ndi_inputs: crate::ndi_transport::NdiInputRegistry,
        capture_inputs: crate::capture_transport::CaptureInputRegistry,
        activation: OutputOwnershipActivation,
        factory: F,
    ) -> Result<Self, SyphonOutputWorkerStartError>
    where
        S: SyphonOutputSender + 'static,
        F: FnOnce(&str) -> Result<S, String> + Send + 'static,
    {
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
        let (start_sender, start_receiver) = mpsc::sync_channel::<SyphonOutputStartDecision>(1);
        let creation_lease_slot: SyphonCreationLeaseSlot = Arc::new(Mutex::new(None));
        let worker_creation_lease_slot = Arc::clone(&creation_lease_slot);
        let parent_engine = engine.clone();
        let pending_sender_name = sender_name.clone();
        let worker = spawn_syphon_output_worker(output_id, move || {
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
                    return Err(SyphonOutputWorkerStopError { message: error });
                }
            };
            // The SDK source is constructed only after the linearizable gate
            // admission. A fence that wins before this point therefore never
            // reaches the SDK constructor.
            let sender = match factory(&sender_name) {
                Ok(sender) => sender,
                Err(error) => {
                    let message = format!("Failed to create Syphon sender '{sender_name}': {error}");
                    let fault = format!("Syphon output route {output_id} open failed: {message}");
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
                    return Err(SyphonOutputWorkerStopError { message });
                }
            };
            let sender = SyphonOwnedSender::new(
                sender,
                engine.clone(),
                Arc::clone(&worker_teardown_lease),
            );
            match worker_creation_lease_slot.lock() {
                Ok(mut slot) => *slot = Some(creation_lease),
                Err(poisoned) => *poisoned.into_inner() = Some(creation_lease),
            }
            if ready_tx.send(Ok(())).is_err() {
                let Some(lease) = take_syphon_creation_lease(&worker_creation_lease_slot) else {
                    return Err(SyphonOutputWorkerStopError {
                        message: "Syphon output startup lease was lost after readiness disconnect"
                            .to_string(),
                    });
                };
                return retire_unpublished_syphon_sender(&engine, sender, lease);
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
                Some(SyphonOutputStartDecision::Publish) => {}
                Some(SyphonOutputStartDecision::Retire(lease)) => {
                    return retire_unpublished_syphon_sender(&engine, sender, lease);
                }
                None => {
                    if let Some(lease) = take_syphon_creation_lease(&worker_creation_lease_slot) {
                        return retire_unpublished_syphon_sender(&engine, sender, lease);
                    }
                    let error =
                        "Syphon output resource publication acknowledgement was dropped".to_string();
                    let failure_lease = engine.begin_output_ownership_failure_fence(error.clone());
                    let result = finish_syphon_output_worker(
                        &engine,
                        sender,
                        &worker_teardown_lease,
                        Some(failure_lease),
                        Some(error),
                    );
                    if let Err(error) = &result {
                        record_syphon_worker_failure(&worker_failure, error.clone());
                    }
                    return result;
                }
            }
            // Publication precedes Arm's Ready transition. Rendering before
            // that transition would mistake its own startup fence for a fault.
            let readiness_error = match activation.wait_until_ready(&worker_stop, Duration::from_secs(30)) {
                Ok(true) => None,
                Ok(false) => Some(None),
                Err(error) => Some(Some(error)),
            };
            if let Some(error) = readiness_error {
                if let Some(error) = error.as_ref() {
                    eprintln!("Syphon output startup stopped: {error}");
                }
                let result = finish_syphon_output_worker(
                    &engine, sender, &worker_teardown_lease, None, error,
                );
                if let Err(error) = &result {
                    record_syphon_worker_failure(&worker_failure, error.clone());
                }
                return result;
            }
            let decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::from_env()
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
            let result = run_syphon_output_worker(
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
                        "Syphon",
                        protocol::VideoOutputKind::SyphonServer,
                        &presentation_sample,
                        ownership,
                        blackout_authority,
                        output_id,
                        &render_sender_name,
                    )
                    .map_err(OutputPresentationRevalidationError::Other)
                    .and_then(|authority| {
                        revalidate_output_presentation_authority_classified(&render_engine, "Syphon", &authority)?;
                        Ok(authority)
                    }) {
                        Ok(authority) => match follow.as_ref() {
                            Some(follow) if follow.epoch != authority.ownership_epoch() => {
                                let retry_key = timeline_follow_output_key(follow, output_id).ok();
                                Ok((
                                    TimelineFollowOutputRenderDecision::Retry {
                                        reason: "Syphon Timeline Follow authority epoch changed before render"
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
                                match validate_follow_output_authority("Syphon", follow, &authority)
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
                            None if authority.project_blackout() =>
                                materialize_current_hard_blackout(&authority).map(|frame| (
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
                            None => renderer
                                .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                                    &snapshot.video,
                                    syphon_output_effect_render_context(
                                        snapshot,
                                        authority.ownership_epoch(),
                                    ),
                                    &snapshot.video_transition_runtime,
                                    output_id,
                                    authority.output_summary().width,
                                    authority.output_summary().height,
                                )
                                .map_err(|error| format!("Syphon output {output_id} artistic render failed: {error:?}"))
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
                        Err(error) => {
                            syphon_render_error_decision(error, follow.as_ref(), output_id)
                        }
                    };
                    if let Err(error) = &rendered {
                        if let Some(follow) = follow.as_ref() {
                            let fault =
                                format!("Syphon output route {output_id} render failed: {error}");
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
                                format!("Syphon output route {output_id} render failed: {error}"),
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
                record_syphon_worker_failure(&worker_failure, error.clone());
            }
            result
        })
        .map_err(|error| SyphonOutputWorkerStartError {
            message: format!("Failed to start Syphon output worker: {error}"),
            pending_startup: None,
        })?;
        match ready_rx.recv_timeout(Duration::from_secs(5)) {
            Ok(Ok(())) => {
                let Some(creation_lease) = take_syphon_creation_lease(&creation_lease_slot) else {
                    stop.store(true, Ordering::Release);
                    drop(ready_rx);
                    drop(start_sender);
                    let message =
                        "Syphon output startup acknowledgement did not retain its creation lease"
                            .to_string();
                    ensure_startup_failure_fence(&parent_engine, &startup_failure_lease, &message);
                    return Err(SyphonOutputWorkerStartError {
                        message: message.clone(),
                        pending_startup: Some(SyphonPendingOutputStartup {
                            sender_name: pending_sender_name,
                            worker: Some(worker),
                            failure_lease: take_startup_failure_lease(&startup_failure_lease),
                            teardown_lease,
                            message,
                        }),
                    });
                };
                Ok(Self {
                    endpoint_name: pending_sender_name,
                    lifecycle: {
                        let mut state = SyphonOutputLifecycle::new();
                        state.ready().expect("new sender lifecycle");
                        state
                    },
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
                            format!("Syphon output startup failed: {error}"),
                        );
                        take_startup_failure_lease(&startup_failure_lease)
                    })
                    .expect("Syphon startup failure fence must retain its lease");
                Err(SyphonOutputWorkerStartError {
                    message: error,
                    pending_startup: Some(SyphonPendingOutputStartup {
                        sender_name: pending_sender_name.clone(),
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown_lease,
                        message: "Syphon output startup cleanup is pending".to_string(),
                    }),
                })
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                drop(ready_rx);
                drop(start_sender);
                let message =
                    format!("Syphon output worker startup acknowledgement failed: {error}");
                ensure_startup_failure_fence(&parent_engine, &startup_failure_lease, &message);
                let failure_lease = take_startup_failure_lease(&startup_failure_lease)
                    .expect("startup timeout fence must retain its lease");
                Err(SyphonOutputWorkerStartError {
                    message,
                    pending_startup: Some(SyphonPendingOutputStartup {
                        sender_name: pending_sender_name,
                        worker: Some(worker),
                        failure_lease: Some(failure_lease),
                        teardown_lease,
                        message: "Syphon output startup cleanup is pending".to_string(),
                    }),
                })
            }
        }
    }

    fn publish(&mut self) -> Result<(), String> {
        let lease = self
            .creation_lease
            .take()
            .ok_or_else(|| "Syphon resource was already published".to_string())?;
        match lease.publish() {
            Ok(()) => {
                self.lifecycle.publish()?;
                let signal = self
                    .start_signal
                    .take()
                    .ok_or_else(|| "Syphon publication signal was already consumed".to_string())?;
                signal
                    .send(SyphonOutputStartDecision::Publish)
                    .map_err(|_| "Syphon worker stopped before resource publication".to_string())
            }
            Err(lease) => {
                self.creation_lease = Some(lease);
                Err("Syphon output resource creation fence was invalidated".to_string())
            }
        }
    }

    fn failure_snapshot(&self) -> Option<SyphonOutputWorkerStopError> {
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

    fn admit_authoring_retirement(&self, engine: &EngineHandle) -> Result<(), String> {
        let lease = engine.begin_video_output_resource_teardown()?;
        let mut slot = self.teardown_lease.lock()
            .map_err(|_| "Video authoring retirement lease slot was poisoned".to_string())?;
        if slot.is_none() {
            *slot = Some(lease);
        }
        Ok(())
    }

    fn stop(mut self) -> Result<(), SyphonOutputWorkerStopError> {
        self.lifecycle.stopping();
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        let result = self
            .worker
            .take()
            .map(|worker| {
                worker.join().unwrap_or_else(|_| {
                    Err(SyphonOutputWorkerStopError {
                        message: "Syphon output worker panicked while stopping".to_string(),
                    })
                })
            })
            .unwrap_or(Ok(()));
        self.release_teardown_lease();
        drop(undelivered_creation_lease);
        match result {
            Ok(()) => self
                .lifecycle
                .stopped()
                .map_err(|message| SyphonOutputWorkerStopError { message }),
            Err(error) => {
                self.lifecycle.failed();
                Err(error)
            }
        }
    }

    fn signal_retirement(&mut self) -> Option<OutputOwnershipCreationLease> {
        let lease = self.creation_lease.take()?;
        let Some(start_signal) = self.start_signal.take() else {
            return Some(lease);
        };
        match start_signal.send(SyphonOutputStartDecision::Retire(lease)) {
            Ok(()) => None,
            Err(error) => match error.0 {
                SyphonOutputStartDecision::Retire(lease) => Some(lease),
                SyphonOutputStartDecision::Publish => None,
            },
        }
    }
}

fn driver_error(message: impl Into<String>) -> video::ExternalVideoTransportDriverError {
    video::ExternalVideoTransportDriverError {
        message: message.into(),
    }
}

impl Drop for SyphonRouteWorker {
    fn drop(&mut self) {
        let undelivered_creation_lease = self.signal_retirement();
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            match worker.join() {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!(
                    "Syphon output worker '{}' Drop cleanup failed: {error}",
                    self.endpoint_name
                ),
                Err(_) => eprintln!(
                    "Syphon output worker '{}' panicked during Drop cleanup",
                    self.endpoint_name
                ),
            }
        }
        self.release_teardown_lease();
        drop(undelivered_creation_lease);
    }
}

fn retire_unpublished_syphon_sender<S: SyphonOutputSender>(
    engine: &EngineHandle,
    mut sender: S,
    creation_lease: OutputOwnershipCreationLease,
) -> Result<(), SyphonOutputWorkerStopError> {
    if let Err(error) = sender.close() {
        let message = format!("Unpublished Syphon server could not stop: {error}. Restart Syndocal before re-arming outputs");
        drop(engine.begin_output_ownership_failure_fence(message.clone()));
        std::mem::forget(sender);
        std::mem::forget(creation_lease);
        return Err(SyphonOutputWorkerStopError { message });
    }
    drop(sender);
    creation_lease.retire();
    Ok(())
}

fn finish_syphon_output_worker<S: SyphonOutputSender>(
    engine: &EngineHandle,
    mut sender: S,
    teardown_lease_slot: &Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    failure_lease: Option<engine::OutputOwnershipTeardownLease>,
    mut worker_error: Option<String>,
) -> Result<(), SyphonOutputWorkerStopError> {
    let authored_teardown = teardown_lease_slot.lock()
        .map(|mut slot| slot.take())
        .unwrap_or_else(|poisoned| poisoned.into_inner().take());
    let teardown_lease = if let Some(lease) = failure_lease {
        lease
    } else if let Some(error) = worker_error.as_ref() {
        engine.begin_output_ownership_failure_fence(error.clone())
    } else if let Some(lease) = authored_teardown {
        lease
    } else {
        match engine.begin_output_ownership_teardown() {
            Ok(lease) => lease,
            Err(error) => {
                let message = format!("Syphon output teardown was not admitted: {error}");
                worker_error = Some(message.clone());
                engine.begin_output_ownership_failure_fence(message)
            }
        }
    };

    // Native close waits for accepted GPU work and unregisters the server.
    // Never release ownership accounting for a framework object whose stop
    // failed: its worker is retired, but the process must be restarted before
    // any successor can acquire physical output ownership.
    if let Err(error) = sender.close() {
        let message = format!(
            "Syphon server could not stop: {error}. Restart Syndocal before re-arming outputs"
        );
        drop(engine.begin_output_ownership_failure_fence(message.clone()));
        std::mem::forget(sender);
        std::mem::forget(teardown_lease);
        return Err(SyphonOutputWorkerStopError { message });
    }
    drop(sender);
    match teardown_lease_slot.lock() {
        Ok(mut slot) => *slot = Some(teardown_lease),
        Err(poisoned) => *poisoned.into_inner() = Some(teardown_lease),
    }

    match worker_error {
        Some(message) => Err(SyphonOutputWorkerStopError { message }),
        None => Ok(()),
    }
}

type SyphonLeaseSlots = (
    Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
    Arc<Mutex<Option<OutputOwnershipTeardownLease>>>,
);

fn run_syphon_output_worker<S, R>(
    output_id: u64,
    sender_name: &str,
    engine: EngineHandle,
    worker_stop: Arc<AtomicBool>,
    mut sender: S,
    lease_slots: SyphonLeaseSlots,
    mut render: R,
) -> Result<(), SyphonOutputWorkerStopError>
where
    S: SyphonOutputSender,
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
    let mut output_pacer =
        crate::video_output_pacer::VideoOutputPacer::new(Duration::from_nanos(1_000_000_000 / 60));
    let mut worker_error = None;
    let mut failure_lease = None;
    let mut follow_output_state = TimelineFollowOutputState::default();
    let mut output_health =
        crate::live_video_output_health::OutputHealthWriter::register(output_id, "syphon");
    while !worker_stop.load(Ordering::Acquire) {
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
                    let (lease, failed_key, fault) = fence_timeline_follow_settlement_deadline(
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
                        let (lease, failed_key, fault) = fence_timeline_follow_settlement_deadline(
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
                            break Err(SyphonOutputWorkerStopError {
                                message: "Syphon output worker is stopping".to_string(),
                            });
                        }
                        Err(_)
                            if admission_started.elapsed() >= VIDEO_OUTPUT_ADMISSION_DEADLINE =>
                        {
                            break Err(SyphonOutputWorkerStopError {
                                message: format!(
                                    "Syphon output route {output_id} video ownership admission deadline ({VIDEO_OUTPUT_ADMISSION_DEADLINE:?}) exceeded"
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
                            "Syphon",
                            &engine,
                            &authority,
                            follow_identity,
                            || {
                                sender.send_rgba(frame.width, frame.height, &frame.data)?;
                                output_health.delivered();
                                Ok(())
                            },
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
                                        "Syphon",
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
                                format!("Syphon output route {output_id} send failed: {error}"),
                            );
                                    failure_lease = Some(lease);
                                    Some(failed_key)
                                } else {
                                    failure_lease =
                                        Some(engine.begin_output_ownership_failure_fence(format!(
                                            "Syphon output route {output_id} send failed: {error}"
                                        )));
                                    None
                                };
                                if follow_identity.is_some() {
                                    let settlement = timeline_follow_result_after_physical_send(
                                        follow_result,
                                        Some(error.as_str()),
                                        "Syphon",
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
                Err("Syphon output frame was produced without an authority contract".to_string())
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
                            "Syphon output route {output_id} render failed: {error}"
                        ))
                    }));
                }
                worker_error = Some(error.clone());
            }
            eprintln!("Syphon output '{sender_name}' stopped: {error}");
            break;
        }
        output_pacer.wait_after_frame();
    }

    finish_syphon_output_worker(
        &engine,
        sender,
        &teardown_lease_slot,
        failure_lease,
        worker_error,
    )
}

fn syphon_render_error_decision(
    error: OutputPresentationRevalidationError,
    follow: Option<&engine::TimelineFollowVideoRenderSnapshot>,
    output_id: u64,
) -> Result<
    (
        TimelineFollowOutputRenderDecision,
        Option<OutputPresentationAuthority>,
    ),
    String,
> {
    match error {
        OutputPresentationRevalidationError::SafetyChanged(reason)
        | OutputPresentationRevalidationError::PresentationChanged(reason) => Ok((
            TimelineFollowOutputRenderDecision::Retry {
                reason,
                key: follow.and_then(|follow| timeline_follow_output_key(follow, output_id).ok()),
            },
            None,
        )),
        OutputPresentationRevalidationError::Other(error) => match follow {
            Some(follow) => Ok((
                TimelineFollowOutputRenderDecision::Retry {
                    reason: error,
                    key: timeline_follow_output_key(follow, output_id).ok(),
                },
                None,
            )),
            None => Err(error),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn syphon_effect_context_preserves_live_audio_frame_identity_and_silence() {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        let mut snapshot = engine.snapshot();
        assert!(syphon_output_effect_render_context(&snapshot, 27)
            .audio_frame
            .is_none());
        let audio = Arc::new(protocol::LiveAudioTextureFrame {
            generation: 27,
            feature_sequence: 1,
            sample_rate: 48_000,
            waveform: [0.25; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
            fft: [0.5; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        });
        snapshot.live_audio_textures = Some(audio.clone());
        let context = syphon_output_effect_render_context(&snapshot, 27);
        assert!(std::ptr::eq(context.audio_frame.unwrap(), audio.as_ref()));
        assert_eq!(context.project_render_epoch, 27);
        snapshot.live_audio_textures = None;
        assert!(syphon_output_effect_render_context(&snapshot, 28)
            .audio_frame
            .is_none());
    }

    struct FakeSender {
        owner: std::thread::ThreadId,
        sends: Arc<AtomicUsize>,
        closed: Arc<AtomicBool>,
        close_error: bool,
        panic_send: bool,
    }

    impl FakeSender {
        fn new(sends: Arc<AtomicUsize>, closed: Arc<AtomicBool>) -> Self {
            Self {
                owner: std::thread::current().id(),
                sends,
                closed,
                close_error: false,
                panic_send: false,
            }
        }
    }

    impl SyphonOutputSender for FakeSender {
        fn send_rgba(&mut self, width: u32, height: u32, pixels: &[u8]) -> Result<(), String> {
            assert_eq!(self.owner, std::thread::current().id());
            if self.panic_send {
                panic!("injected Syphon send panic");
            }
            assert_eq!(pixels.len(), width as usize * height as usize * 4);
            self.sends.fetch_add(1, Ordering::AcqRel);
            Ok(())
        }
        fn close(&mut self) -> Result<(), String> {
            assert_eq!(self.owner, std::thread::current().id());
            if self.close_error {
                return Err("injected native stop failure".to_string());
            }
            self.closed.store(true, Ordering::Release);
            Ok(())
        }
    }

    fn test_engine(output_id: u64) -> EngineHandle {
        let engine = EngineHandle::start_for_tests(protocol::DmxOutputConfig {
            enabled: false,
            ..protocol::DmxOutputConfig::default()
        });
        engine
            .send(engine::EngineCommand::AddVideoOutput(
                protocol::VideoOutputSummary {
                    id: output_id,
                    label: format!("Test Syphon {output_id}"),
                    kind: protocol::VideoOutputKind::SyphonServer,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width: 1,
                    height: 1,
                    endpoint_name: Some(format!("Test Syphon {output_id}")),
                    opacity: 1.0,
                    blackout: false,
                    mapping: protocol::VideoOutputMapping::default(),
                },
            ))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !engine
            .snapshot()
            .video
            .outputs
            .iter()
            .any(|output| output.id == output_id)
        {
            assert!(Instant::now() < deadline, "test output must publish");
            std::thread::sleep(Duration::from_millis(2));
        }
        engine
    }

    fn test_route(output_id: u64) -> video::ExternalVideoTransportRoute {
        video::ExternalVideoTransportRoute {
            direction: video::ExternalVideoTransportDirection::Output,
            route_id: output_id,
            label: format!("Test Syphon {output_id}"),
            backend_id: "syphon".to_string(),
            endpoint_name: format!("Test Syphon {output_id}"),
        }
    }

    fn test_transport() -> SyphonTransportState {
        SyphonTransportState::new(
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
        )
    }

    fn test_authority(engine: &EngineHandle, output_id: u64) -> OutputPresentationAuthority {
        capture_output_presentation_authority(
            "Syphon",
            protocol::VideoOutputKind::SyphonServer,
            &engine.video_presentation_sample(),
            engine.output_ownership_status(),
            engine.safety_blackout_authority(),
            output_id,
            &format!("Test Syphon {output_id}"),
        )
        .unwrap()
    }

    #[test]
    fn syphon_authoring_rejects_expired_authority_after_readiness_without_sending() {
        let engine = test_engine(930_020);
        let before = engine.output_ownership_status();
        let activation = engine.admit_output_activation(before.effective_role).unwrap();
        let expired = Arc::new(AtomicBool::new(false));
        let expiry_after_constructor = Arc::clone(&expired);
        let sends = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let sender_sends = Arc::clone(&sends);
        let sender_closed = Arc::clone(&closed);
        let authority = || {
            if expired.load(Ordering::Acquire) {
                Err("authoring lease expired during sender startup".to_string())
            } else {
                Ok(())
            }
        };
        let mut transport = test_transport();
        let error = transport.start_route_with_factory_and_authority(
            &test_route(930_020), &engine, Some(activation),
            move |_| {
                expiry_after_constructor.store(true, Ordering::Release);
                Ok(FakeSender::new(sender_sends, sender_closed))
            },
            Some(&authority),
        ).unwrap_err();
        assert!(error.message.contains("authoring lease expired"));
        assert_eq!(sends.load(Ordering::Acquire), 0);
        assert!(closed.load(Ordering::Acquire));
        assert!(transport.output_workers.is_empty());
        assert!(transport.pending_output_startups.is_empty());
        assert_eq!(engine.output_ownership_status(), before);
        engine.begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap().complete().unwrap();
    }

    #[test]
    fn syphon_authoring_retires_one_live_route_without_rearming_or_stopping_another() {
        let engine = test_engine(930_021);
        let mut second = engine.video_output_snapshot(930_021).unwrap();
        second.id = 930_022;
        second.label = "Test Syphon 930022".to_string();
        second.endpoint_name = Some("Test Syphon 930022".to_string());
        // A persistence projection response can precede visible publication.
        // Wait for the second output's acknowledged publication before either
        // fake sender captures its output configuration.
        engine
            .send(engine::EngineCommand::AddVideoOutput(second))
            .unwrap();
        engine.await_snapshot_publication().unwrap();
        assert!(engine.video_output_snapshot(930_021).is_some());
        assert!(engine.video_output_snapshot(930_022).is_some());
        let before = engine.output_ownership_status();
        let closed_a = Arc::new(AtomicBool::new(false));
        let closed_b = Arc::new(AtomicBool::new(false));
        let mut transport = test_transport();
        for (id, closed) in [(930_021, Arc::clone(&closed_a)), (930_022, Arc::clone(&closed_b))] {
            let activation = engine.admit_output_activation(before.effective_role).unwrap();
            transport.start_route_with_factory_and_authority(
                &test_route(id), &engine, Some(activation),
                move |_| Ok(FakeSender::new(Arc::new(AtomicUsize::new(0)), closed)),
                Some(&|| Ok(())),
            ).unwrap();
        }
        assert!(transport.stop_route(&test_route(930_021), &engine).is_err(),
            "the ordinary API still requires a global transition");
        assert!(transport.stop_route_for_authoring(&test_route(930_021), &engine,
            &|| Err("stale authoring owner".to_string())).is_err());
        assert_eq!(transport.output_workers.len(), 2);
        assert!(!closed_a.load(Ordering::Acquire));
        transport.stop_route_for_authoring(&test_route(930_021), &engine, &|| Ok(())).unwrap();
        assert!(closed_a.load(Ordering::Acquire));
        assert!(!closed_b.load(Ordering::Acquire));
        assert!(!transport.output_workers.contains_key(&930_021));
        assert!(transport.output_workers.contains_key(&930_022));
        assert_eq!(engine.output_ownership_status(), before);
        transport.stop_route_for_authoring(&test_route(930_022), &engine, &|| Ok(())).unwrap();
        assert!(closed_b.load(Ordering::Acquire));
        assert!(transport.output_workers.is_empty());
        engine.begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap().complete().unwrap();
    }

    #[test]
    fn syphon_authoring_failed_native_close_retains_accounting_and_fences_successors() {
        let engine = test_engine(930_023);
        let activation = engine.admit_output_activation(protocol::MachineOutputRole::Both).unwrap();
        let closed = Arc::new(AtomicBool::new(false));
        let sender_closed = Arc::clone(&closed);
        let mut transport = test_transport();
        transport.start_route_with_factory_and_authority(
            &test_route(930_023), &engine, Some(activation),
            move |_| {
                let mut sender = FakeSender::new(Arc::new(AtomicUsize::new(0)), sender_closed);
                sender.close_error = true;
                Ok(sender)
            },
            Some(&|| Ok(())),
        ).unwrap();
        let error = transport.stop_route_for_authoring(&test_route(930_023), &engine, &|| Ok(())).unwrap_err();
        assert!(error.message.contains("native stop failure"));
        assert!(!closed.load(Ordering::Acquire));
        assert!(transport.failed_output_routes.contains_key(&930_023));
        assert_eq!(engine.output_ownership_status().state, protocol::OutputOwnershipState::Failed);
        assert!(engine.begin_output_ownership_transition(protocol::MachineOutputRole::Both).is_err());
    }

    #[test]
    fn syphon_published_startup_waits_for_ready_and_cancel_retires_the_owner() {
        let engine = test_engine(930_001);
        let mut transition = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Video)
            .unwrap();
        transition.begin_activation().unwrap();
        let activation = transition.admit_output_activation().unwrap();
        let sends = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let worker_sends = Arc::clone(&sends);
        let worker_closed = Arc::clone(&closed);
        let mut transport = test_transport();
        let route = test_route(930_001);
        transport
            .start_route_with_factory(&route, &engine, Some(activation), move |_| {
                Ok(FakeSender::new(worker_sends, worker_closed))
            })
            .unwrap();
        assert_eq!(
            engine.output_ownership_status().state,
            protocol::OutputOwnershipState::Activating
        );
        assert!(engine.acquire_video_output().is_err());
        std::thread::sleep(Duration::from_millis(40));
        assert_eq!(
            sends.load(Ordering::Acquire),
            0,
            "publication must not bypass the READY barrier"
        );
        assert!(!closed.load(Ordering::Acquire));
        transport.stop_route(&route, &engine).unwrap();
        assert!(closed.load(Ordering::Acquire));
        assert!(transport.output_workers.is_empty());
        assert_eq!(sends.load(Ordering::Acquire), 0);
        transition.fail("test cancels Arm before READY");
        let standby = engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap()
            .complete()
            .unwrap();
        assert!(!standby.video_allowed);
    }

    #[test]
    fn syphon_unpublished_sender_retires_under_its_creation_lease() {
        let engine = test_engine(930_002);
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let sends = Arc::new(AtomicUsize::new(0));
        let closed = Arc::new(AtomicBool::new(false));
        let worker_sends = Arc::clone(&sends);
        let worker_closed = Arc::clone(&closed);
        let worker = SyphonRouteWorker::start_output_with_factory(
            930_002,
            "Test Syphon 930002".to_string(),
            engine.clone(),
            #[cfg(feature = "ndi")]
            Arc::new(Mutex::new(HashMap::new())),
            Arc::new(Mutex::new(HashMap::new())),
            activation,
            move |_| Ok(FakeSender::new(worker_sends, worker_closed)),
        )
        .unwrap();
        worker.stop().unwrap();
        assert!(closed.load(Ordering::Acquire));
        assert_eq!(sends.load(Ordering::Acquire), 0);
        engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap()
            .complete()
            .unwrap();
    }

    #[test]
    fn syphon_constructor_failure_retains_fence_until_worker_is_harvested() {
        let engine = test_engine(930_003);
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        let mut transport = test_transport();
        let error = transport
            .start_route_with_factory(&test_route(930_003), &engine, Some(activation), |_| {
                Err::<FakeSender, String>("injected constructor failure".to_string())
            })
            .unwrap_err();
        assert!(error.message.contains("injected constructor failure"));
        assert_eq!(
            engine.output_ownership_status().state,
            protocol::OutputOwnershipState::Failed
        );
        assert!(engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
            .is_err());
        let deadline = Instant::now() + Duration::from_secs(2);
        while !transport.pending_output_startups.is_empty() {
            transport.harvest_failed_workers(&engine).unwrap();
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert!(transport.failed_output_routes.contains_key(&930_003));
        engine
            .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
            .unwrap()
            .complete()
            .unwrap();
    }

    #[test]
    fn syphon_stale_activation_and_input_routes_never_construct_a_sender() {
        let engine = test_engine(930_004);
        let mut transport = test_transport();
        let calls = Arc::new(AtomicUsize::new(0));
        let mut route = test_route(930_004);
        route.direction = video::ExternalVideoTransportDirection::Input;
        let attempted = Arc::clone(&calls);
        let result = transport.start_route_with_factory(&route, &engine, None, move |_| {
            attempted.fetch_add(1, Ordering::AcqRel);
            Ok(FakeSender::new(
                Arc::new(AtomicUsize::new(0)),
                Arc::new(AtomicBool::new(false)),
            ))
        });
        assert!(result.is_err());
        assert_eq!(calls.load(Ordering::Acquire), 0);
        route.direction = video::ExternalVideoTransportDirection::Output;
        let activation = engine
            .admit_output_activation(protocol::MachineOutputRole::Both)
            .unwrap();
        drop(engine.begin_output_ownership_failure_fence("invalidate old Syphon activation"));
        let attempted = Arc::clone(&calls);
        let result =
            transport.start_route_with_factory(&route, &engine, Some(activation), move |_| {
                attempted.fetch_add(1, Ordering::AcqRel);
                Ok(FakeSender::new(
                    Arc::new(AtomicUsize::new(0)),
                    Arc::new(AtomicBool::new(false)),
                ))
            });
        assert!(result.is_err());
        assert_eq!(
            calls.load(Ordering::Acquire),
            0,
            "creation admission precedes the native constructor"
        );
    }

    #[test]
    fn syphon_authority_allows_one_send_and_revokes_changed_output_mapping() {
        let engine = test_engine(930_005);
        let mut authority = test_authority(&engine, 930_005);
        assert_eq!(
            authority.ownership_status(),
            engine.output_ownership_status()
        );
        assert_eq!(
            authority.presentation_config_token(),
            engine.video_presentation_config_token()
        );
        let key = TimelineFollowOutputFrameKey {
            epoch: authority.ownership_epoch(),
            generation: 0,
            output_id: 930_005,
            width: 1,
            height: 1,
        };
        assert!(timeline_follow_epoch_matches_ready_owner(&engine, key));
        let mut sends = 0;
        let permit = engine.acquire_video_output().unwrap();
        send_frame_if_authorized("Syphon", &engine, &authority, None, || {
            sends += 1;
            Ok(())
        })
        .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(sends, 1);
        let mut mapping = authority.output_summary().mapping.clone();
        mapping.black_level = 0.75;
        authority.override_output_mapping_for_test(mapping);
        let rejected = send_frame_if_authorized("Syphon", &engine, &authority, None, || {
            sends += 1;
            Ok(())
        });
        assert!(matches!(rejected, Err(PhysicalOutputSendError::Revoked(_))));
        assert_eq!(sends, 1, "revalidation rejects before entering the SDK");
        let black = materialize_current_hard_blackout(&authority).unwrap();
        assert_eq!(
            black.data,
            vec![0, 0, 0, 255],
            "hard blackout bypasses authored black level"
        );
        drop(permit);
        let mut follow_state = TimelineFollowOutputState::default();
        follow_state.queue_ack(
            key,
            TimelineFollowSettlementAckResult::Fault {
                fault: "test failure".to_string(),
            },
            Instant::now(),
        );
        assert!(matches!(
            follow_state.pending_ack_result(),
            Some(TimelineFollowSettlementAckResult::Fault { .. })
        ));
    }

    #[test]
    fn syphon_close_failure_cannot_acknowledge_retirement_or_rearm() {
        let engine = test_engine(930_006);
        let closed = Arc::new(AtomicBool::new(false));
        let mut sender = FakeSender::new(Arc::new(AtomicUsize::new(0)), Arc::clone(&closed));
        sender.close_error = true;
        let lease = engine.begin_output_ownership_failure_fence("test native retirement failure");
        let slot = Arc::new(Mutex::new(None));
        let error =
            finish_syphon_output_worker(&engine, sender, &slot, Some(lease), None).unwrap_err();
        assert!(error.message.contains("Restart Syndocal"));
        assert!(!closed.load(Ordering::Acquire));
        assert!(
            slot.lock().unwrap().is_none(),
            "unretired native resources have no teardown ACK"
        );
        assert_eq!(
            engine.output_ownership_status().state,
            protocol::OutputOwnershipState::Failed
        );
        assert!(
            engine
                .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
                .is_err(),
            "retained native-resource lease fences every successor"
        );
    }
    #[test]
    fn syphon_sender_panic_preserves_retirement_accounting_until_join() {
        for close_error in [false, true] {
            let engine = test_engine(if close_error { 930_008 } else { 930_007 });
            let activation = engine
                .admit_output_activation(protocol::MachineOutputRole::Both)
                .unwrap();
            let closed = Arc::new(AtomicBool::new(false));
            let slot = Arc::new(Mutex::new(None));
            let worker_closed = Arc::clone(&closed);
            let worker_slot = Arc::clone(&slot);
            let worker_engine = engine.clone();
            let owner = std::thread::spawn(move || {
                let creation = activation.admit_resource_creation().unwrap();
                let mut raw = FakeSender::new(Arc::new(AtomicUsize::new(0)), worker_closed);
                raw.close_error = close_error;
                raw.panic_send = true;
                let mut sender = SyphonOwnedSender::new(raw, worker_engine, worker_slot);
                creation.publish().unwrap();
                sender.send_rgba(1, 1, &[0, 0, 0, 255]).unwrap();
            });
            assert!(
                owner.join().is_err(),
                "the injected send must unwind the real sender guard"
            );
            assert_eq!(
                engine.output_ownership_status().state,
                protocol::OutputOwnershipState::Failed
            );
            assert!(engine
                .begin_output_ownership_transition(protocol::MachineOutputRole::Both)
                .is_err());
            if close_error {
                assert!(!closed.load(Ordering::Acquire));
                assert!(
                    slot.lock().unwrap().is_none(),
                    "failed native cleanup never acknowledges retirement"
                );
            } else {
                assert!(closed.load(Ordering::Acquire));
                assert!(
                    slot.lock().unwrap().is_some(),
                    "native cleanup retains its lease for the join owner"
                );
                drop(slot.lock().unwrap().take());
                engine
                    .begin_output_ownership_transition(protocol::MachineOutputRole::Standby)
                    .unwrap()
                    .complete()
                    .unwrap();
            }
        }
    }
}
