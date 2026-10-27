//! Default-off native Display orchestration. All rendering and presentation
//! remain behind video's opaque shared-device owner; the legacy path is not
//! entered by this module.

use super::*;

struct StopCancellation<'a>(&'a AtomicBool);

impl video::VideoRenderCancellation for StopCancellation<'_> {
    fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }
}

struct PresentAuthority<'a> {
    engine: &'a EngineHandle,
    captured: &'a NativeDisplayPresentationAuthority,
    follow: Option<NativeTimelineFollowActiveIdentity>,
    stop: &'a AtomicBool,
    window: Option<&'a tauri::Window>,
    extent: (u32, u32),
    first_fence: Option<&'a OutputControlFenceV1>,
    first_admission: Option<&'a (dyn Fn() -> Result<(), String> + Send)>,
}

impl video::NativeResidentPresentationAuthority for PresentAuthority<'_> {
    fn revalidate(
        &self,
        boundary: video::NativeResidentPresentBoundary,
    ) -> video::NativeResidentPresentDecision {
        use video::NativeResidentPresentDecision::{Cancelled, Permit, Revoked};
        if self.stop.load(Ordering::Acquire) {
            return Cancelled;
        }
        if revalidate_native_display_presentation_authority(self.engine, self.captured).is_err() {
            return Revoked;
        }
        if let Some(expected) = self.follow {
            if validate_native_timeline_follow_active_identity(
                expected,
                self.engine.timeline_follow_video_render_snapshot().as_ref(),
            )
            .is_err()
            {
                return Revoked;
            }
        }
        if !matches!(
            boundary,
            video::NativeResidentPresentBoundary::BeforePrepare
        ) {
            if self.window.is_some_and(|window| {
                window
                    .inner_size()
                    .map(|size| (size.width, size.height) != self.extent)
                    .unwrap_or(true)
            }) {
                return Revoked;
            }
            if self.first_fence.is_some_and(|expected| {
                !exact_display_first_frame_fence_matches(self.engine, expected)
            }) {
                return Revoked;
            }
            if self.first_admission.is_some_and(|admit| admit().is_err()) {
                return Revoked;
            }
        }
        if self.stop.load(Ordering::Acquire) {
            Cancelled
        } else if self.engine.video_presentation_config_token()
            != self.captured.presentation_config_token
        {
            Revoked
        } else {
            Permit
        }
    }
}

struct Resources {
    session: video::NativeResidentDisplaySession,
    renderer: AppVideoPreviewRenderer,
    stop_started: bool,
    configured_extent: Option<(u32, u32)>,
}

impl Resources {
    fn poll_stop(&mut self) -> Result<bool, String> {
        if !self.stop_started {
            self.session.begin_stop_with_renderer(&mut self.renderer)?;
            self.stop_started = true;
        }
        self.session.poll_stop()
    }
}

/// Only startup/thread handoff and the existing cold retirement path lock this
/// slot. No rendering iteration locks it, and GPU polling never runs under it.
#[derive(Clone, Default)]
pub(crate) struct ResidentRetirement {
    slot: Arc<Mutex<Option<Box<Resources>>>>,
}

impl ResidentRetirement {
    pub(crate) fn is_empty(&self) -> bool {
        self.slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .is_none()
    }

    fn retain(&self, resources: Box<Resources>) {
        let mut slot = self
            .slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        // The affine lease is the only writer until its thread has joined.
        assert!(slot.is_none(), "resident retirement already owns a session");
        *slot = Some(resources);
    }

    pub(crate) fn poll_with_deadline(&self, deadline: Instant) -> Result<(), String> {
        let resources = self
            .slot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        let Some(resources) = resources else {
            return Ok(());
        };
        // Poll outside the slot mutex, but keep the whole owner guarded during
        // begin_stop/poll_stop. Errors, deadlines and unwind restore the slot.
        let mut lease = ResourceLease {
            resources: Some(resources),
            retirement: self.clone(),
        };
        loop {
            match lease.resources().poll_stop() {
                Ok(true) => {
                    drop(lease.resources.take());
                    return Ok(());
                }
                Ok(false) if Instant::now() < deadline => {
                    std::thread::park_timeout(Duration::from_millis(1));
                }
                Ok(false) => {
                    return Err(
                        "Native resident GPU owner remains pending at retirement deadline".into(),
                    );
                }
                Err(error) => {
                    return Err(format!(
                        "Native resident GPU retirement retained its owner: {error}"
                    ));
                }
            }
        }
    }
}

/// Panic, failed thread creation and every early return preserve the whole
/// session/renderer aggregate. Drop only moves ownership; it does no GPU work.
struct ResourceLease {
    resources: Option<Box<Resources>>,
    retirement: ResidentRetirement,
}

impl ResourceLease {
    fn resources(&mut self) -> &mut Resources {
        self.resources
            .as_mut()
            .expect("active resident lease owns its resources")
    }

    fn finish_stop(&mut self) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_millis(500);
        loop {
            match self.resources().poll_stop() {
                Ok(true) => {
                    self.resources.take();
                    return Ok(());
                }
                Ok(false) if Instant::now() < deadline => {
                    std::thread::park_timeout(Duration::from_millis(1));
                }
                Ok(false) => {
                    return Err("Native resident GPU owner did not retire within 500 ms".into())
                }
                Err(error) => return Err(error),
            }
        }
    }
}

impl Drop for ResourceLease {
    fn drop(&mut self) {
        if let Some(resources) = self.resources.take() {
            self.retirement.retain(resources);
        }
    }
}

/// A failed creation can still own submitted work. Return that owner to the
/// caller so it enters the same bounded cleanup and cold retirement path.
fn retain_creation_owner<T>(
    creation: Result<T, (String, Option<Box<T>>)>,
) -> Result<(T, Option<String>), String> {
    match creation {
        Ok(owner) => Ok((owner, None)),
        Err((error, Some(owner))) => Ok((*owner, Some(error))),
        Err((error, None)) => Err(error),
    }
}

struct Presentable {
    payload: video::NativeResidentDisplayFrame,
    authority: NativeDisplayPresentationAuthority,
    validation_identity: NativeVideoOutputValidationIdentity,
    follow_identity: Option<NativeTimelineFollowActiveIdentity>,
    settlement: Option<TimelineFollowSettlementAck>,
}

enum PreparedOutput {
    Presentable(Presentable),
    SettlementOnly(TimelineFollowSettlementAck),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrepareInterruption {
    Cancelled { retirement_required: bool },
    Revoked { retirement_required: bool },
}

impl PrepareInterruption {
    fn ends_session(self) -> bool {
        match self {
            Self::Cancelled { .. } => true,
            Self::Revoked {
                retirement_required,
            } => retirement_required,
        }
    }

    fn reason(self) -> &'static str {
        match self {
            Self::Cancelled {
                retirement_required: false,
            } => "Native resident preparation cancelled before admission",
            Self::Cancelled {
                retirement_required: true,
            } => "Native resident preparation cancelled; owner retirement required",
            Self::Revoked {
                retirement_required: false,
            } => "Native resident preparation authority revoked; await a fresh tick",
            Self::Revoked {
                retirement_required: true,
            } => "Native resident preparation authority revoked; owner retirement required",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FrameProgress {
    Complete,
    Interrupted(PrepareInterruption),
    StartupRejected,
}

#[derive(Debug)]
enum FrameFault {
    Preparation(String),
    Operation(String),
}

impl From<String> for FrameFault {
    fn from(error: String) -> Self {
        Self::Operation(error)
    }
}

impl FrameFault {
    fn message(&self) -> &str {
        match self {
            Self::Preparation(error) | Self::Operation(error) => error,
        }
    }

    fn is_preparation_fault(&self) -> bool {
        matches!(self, Self::Preparation(_))
    }

    fn into_worker_result(
        self,
        stop: &AtomicBool,
        ownership: protocol::OutputOwnershipState,
    ) -> Result<(), String> {
        match self {
            Self::Preparation(error) => Err(error),
            Self::Operation(error) => {
                native_video_output_worker_result(stop, ownership, Err(error))
            }
        }
    }
}

/// Preparation control outcomes never enter the physical callback. The typed
/// observation stays latched even if authority has already changed again.
fn with_prepared_output<T>(
    prepared: Result<T, video::NativeResidentPrepareError>,
    physical: impl FnOnce(T) -> Result<FrameProgress, FrameFault>,
) -> Result<FrameProgress, FrameFault> {
    match prepared {
        Ok(output) => physical(output),
        Err(video::NativeResidentPrepareError::Cancelled {
            retirement_required,
        }) => Ok(FrameProgress::Interrupted(PrepareInterruption::Cancelled {
            retirement_required,
        })),
        Err(video::NativeResidentPrepareError::Revoked {
            retirement_required,
        }) => Ok(FrameProgress::Interrupted(PrepareInterruption::Revoked {
            retirement_required,
        })),
        Err(video::NativeResidentPrepareError::Fault(error)) => Err(FrameFault::Preparation(error)),
    }
}

fn startup_rejected(boundary: &mut Option<String>, reason: impl Into<String>) -> FrameProgress {
    *boundary = Some(reason.into());
    FrameProgress::StartupRejected
}

fn close_for_orderly_retirement(
    stop: &AtomicBool,
    close: impl FnOnce() -> Result<(), String>,
) -> Result<(), String> {
    stop.store(true, Ordering::Release);
    close()
}

fn prepare(
    resources: &mut Resources,
    engine: &EngineHandle,
    output_id: VideoOutputId,
    extent: (u32, u32),
    label: &str,
    unpublished: Option<&EngineSnapshot>,
    stop: &AtomicBool,
    observed_follow: &mut Option<NativeTimelineFollowActiveIdentity>,
) -> Result<PreparedOutput, video::NativeResidentPrepareError> {
    *observed_follow = None;
    let cancel = StopCancellation(stop);
    if unpublished.is_none() {
        if let Some((context, follow)) =
            capture_native_timeline_follow_video_render_snapshot(engine)
        {
            let follow_identity = native_timeline_follow_active_identity(&follow);
            *observed_follow = Some(follow_identity);
            let sample = engine.video_presentation_sample();
            let authority = capture_native_display_presentation_authority(
                &sample,
                engine.output_ownership_status(),
                engine.safety_blackout_authority(),
                output_id,
                label,
            )
            .map_err(|_| video::NativeResidentPrepareError::Revoked {
                retirement_required: false,
            })?;
            if follow.epoch != authority.ownership.epoch
                || follow.epoch != context.project_render_epoch
            {
                return Err(video::NativeResidentPrepareError::Revoked {
                    retirement_required: false,
                });
            }
            for snapshot in [&follow.outgoing_video, &follow.incoming_video] {
                let output = snapshot
                    .outputs
                    .iter()
                    .find(|output| output.id == output_id)
                    .ok_or(video::NativeResidentPrepareError::Revoked {
                        retirement_required: false,
                    })?;
                if output != &authority.output {
                    return Err(video::NativeResidentPrepareError::Revoked {
                        retirement_required: false,
                    });
                }
                if !output.enabled {
                    return Ok(PreparedOutput::SettlementOnly(
                        native_timeline_follow_video_ack(
                            &follow,
                            output_id,
                            TimelineFollowSettlementAckResult::NotApplicable,
                        ),
                    ));
                }
            }
            let transition_chain = follow
                .transition_effect_chain
                .as_ref()
                .map(|expected| {
                    let resolved =
                        video::resolve_video_effect_chain(&follow.outgoing_video, &expected.scope)
                            .map_err(|fault| {
                                format!(
                                    "Timeline Follow Transition chain is invalid: {}",
                                    fault.message
                                )
                            })?
                            .ok_or_else(|| {
                                "Timeline Follow Transition chain disappeared".to_string()
                            })?;
                    if resolved.chain_id != Some(expected.id) {
                        return Err("Timeline Follow Transition chain identity changed".to_string());
                    }
                    Ok(resolved)
                })
                .transpose()
                .map_err(video::NativeResidentPrepareError::Fault)?;
            resources
                .renderer
                .frame_provider_mut()
                .set_bpm(Some(context.runtime.clock_bpm));
            let live = PresentAuthority {
                engine,
                captured: &authority,
                follow: Some(follow_identity),
                stop,
                window: None,
                extent,
                first_fence: None,
                first_admission: None,
            };
            let payload = resources.session.prepare_follow_output(
                &mut resources.renderer,
                &follow.outgoing_video,
                &follow.incoming_video,
                video::VideoEffectRenderContext {
                    clip_runtime: &context.runtime.clip_runtime,
                    audio_frame: context.runtime.live_audio_textures.as_deref(),
                    project_render_epoch: follow.epoch,
                },
                &context.runtime.transition_runtime,
                follow.kind,
                follow.curve,
                follow.progress_millis,
                follow.source_timeline_id,
                follow.generation,
                transition_chain.as_ref(),
                extent,
                &cancel,
                &live,
            )?;
            if payload.evidence().freshness != video::VideoOutputRenderFreshness::Fresh {
                return Ok(PreparedOutput::SettlementOnly(
                    native_timeline_follow_video_ack(
                        &follow,
                        output_id,
                        TimelineFollowSettlementAckResult::Fault {
                            fault: payload.evidence().error.clone().unwrap_or_else(|| {
                                "Timeline Follow produced non-fresh evidence".into()
                            }),
                        },
                    ),
                ));
            }
            return Ok(PreparedOutput::Presentable(Presentable {
                validation_identity: native_video_output_validation_identity_from_runtime(
                    &authority,
                    &context.runtime,
                    Some(follow_identity),
                    extent.0,
                    extent.1,
                ),
                payload,
                authority,
                follow_identity: Some(follow_identity),
                settlement: Some(native_timeline_follow_video_ack(
                    &follow,
                    output_id,
                    TimelineFollowSettlementAckResult::Applied,
                )),
            }));
        }
    }
    let sample;
    let (snapshot, authority): (&EngineSnapshot, NativeDisplayPresentationAuthority) =
        if let Some(snapshot) = unpublished {
            let authority = capture_unpublished_native_display_presentation_authority(
                snapshot,
                &engine.output_ownership_status(),
                engine.safety_blackout_authority(),
                engine.video_presentation_config_token(),
                output_id,
                label,
            )
            .map_err(video::NativeResidentPrepareError::Fault)?;
            (snapshot, authority)
        } else {
            sample = engine.video_presentation_sample();
            let authority = capture_native_display_presentation_authority(
                &sample,
                engine.output_ownership_status(),
                engine.safety_blackout_authority(),
                output_id,
                label,
            )
            .map_err(|_| video::NativeResidentPrepareError::Revoked {
                retirement_required: false,
            })?;
            (&sample.snapshot, authority)
        };
    resources
        .renderer
        .frame_provider_mut()
        .set_bpm(Some(snapshot.clock.bpm));
    let live = PresentAuthority {
        engine,
        captured: &authority,
        follow: None,
        stop,
        window: None,
        extent,
        first_fence: None,
        first_admission: None,
    };
    let payload = resources.session.prepare_output(
        &mut resources.renderer,
        &snapshot.video,
        video::VideoEffectRenderContext {
            clip_runtime: &snapshot.video_clip_runtime,
            audio_frame: snapshot.live_audio_textures.as_deref(),
            project_render_epoch: authority.ownership.epoch,
        },
        &snapshot.video_transition_runtime,
        extent,
        &cancel,
        &live,
    )?;
    Ok(PreparedOutput::Presentable(Presentable {
        validation_identity: native_video_output_validation_identity(
            &authority, snapshot, None, extent.0, extent.1,
        ),
        payload,
        authority,
        follow_identity: None,
        settlement: None,
    }))
}

/// Successful delivery and Follow Applied originate only from the shared
/// presenter's Presented receipt. NotPresented is an observation, never a
/// successful delivery or an instruction to prepare the same tick again.
fn present(
    resources: &mut Resources,
    output: PreparedOutput,
    engine: &EngineHandle,
    stop: &Arc<AtomicBool>,
    window: &tauri::Window,
    first_fence: Option<&OutputControlFenceV1>,
    first_admission: Option<&(dyn Fn() -> Result<(), String> + Send)>,
    health: &mut live_video_output_health::OutputHealthWriter,
    render_time: Duration,
    validation: &mut Option<NativeVideoOutputValidationIdentity>,
    boundary: &mut Option<String>,
) -> Result<bool, String> {
    if stop.load(Ordering::Acquire) {
        *boundary = Some("Native resident output cancelled before present".into());
        return Ok(false);
    }
    match output {
        PreparedOutput::SettlementOnly(ack) => {
            *boundary = Some(native_video_output_settlement_validation_reason(&ack));
            acknowledge_native_timeline_follow_video_settlement(engine, ack, stop);
            Ok(false)
        }
        PreparedOutput::Presentable(frame) => {
            *validation = Some(frame.validation_identity);
            let live = PresentAuthority {
                engine,
                captured: &frame.authority,
                follow: frame.follow_identity,
                stop,
                window: Some(window),
                extent: frame.payload.extent(),
                first_fence,
                first_admission,
            };
            let outcome =
                resources
                    .session
                    .present(frame.payload, &live, &StopCancellation(stop))?;
            Ok(complete_present(
                outcome,
                stop,
                || {
                    #[cfg(feature = "qa-native-frame-timing")]
                    video::native_frame_timing::set_outcome(
                        video::native_frame_timing::Outcome::Presented,
                    );
                    health.delivered_with_render_time(render_time);
                },
                || {
                    if let Some(ack) = frame.settlement {
                        acknowledge_native_timeline_follow_video_settlement(engine, ack, stop);
                    }
                },
                boundary,
            ))
        }
    }
}

/// Receipt classification is shared by the production callback and focused
/// Stop/ACK tests. It never derives successful delivery from a prepared frame.
fn complete_present(
    outcome: video::NativeResidentPresentResult,
    stop: &AtomicBool,
    delivered: impl FnOnce(),
    acknowledge: impl FnOnce(),
    boundary: &mut Option<String>,
) -> bool {
    match outcome {
        video::NativeResidentPresentResult::Presented { .. } => {
            delivered();
            if !stop.load(Ordering::Acquire) {
                acknowledge();
            }
            true
        }
        video::NativeResidentPresentResult::NotPresented { reason } => {
            *boundary = Some(reason);
            false
        }
    }
}

fn record_metrics(
    metrics: &Arc<Mutex<NativeVideoOutputMetrics>>,
    resources: &Resources,
    extent: (u32, u32),
    started: Instant,
    error: Option<String>,
    validation: Option<NativeVideoOutputValidationIdentity>,
    boundary: Option<String>,
) {
    if let Ok(mut metrics) = metrics.lock() {
        metrics.record_resident(
            extent.0,
            extent.1,
            started.elapsed(),
            resources.session.presented_frames(),
            resources.session.hard_blackout_frames_presented(),
            resources.renderer.frame_provider().decoder().diagnostics(),
            error,
            validation,
            boundary,
        );
    }
}

fn retain_failed_start(
    window: &tauri::Window,
    label: &str,
    stop: Arc<AtomicBool>,
    teardown_lease: Arc<Mutex<Option<engine::OutputOwnershipTeardownLease>>>,
    retirement: ResidentRetirement,
) {
    if retirement.is_empty() {
        return;
    }
    let app = window.app_handle();
    let state = app.state::<AppState>();
    let mut registry = state
        .native_video_output_workers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let key = if registry.contains_key(label) {
        native_video_output_quarantine_label(label)
    } else {
        label.to_string()
    };
    let mut worker = NativeVideoOutputWorker::for_retirement(None);
    worker.stop = stop;
    worker.teardown_lease = teardown_lease;
    worker.resident_retirements.push(retirement);
    registry.insert(key, worker);
}

pub(crate) fn start(
    request: NativeVideoLiveOutputRequest,
) -> Result<NativeVideoOutputWorker, String> {
    let NativeVideoLiveOutputRequest {
        window,
        engine,
        output_id,
        metrics,
        #[cfg(feature = "ndi")]
        ndi_inputs,
        #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
        spout_inputs,
        capture_inputs,
        initial_size_override,
        fullscreen,
        unpublished_snapshot,
        gate_render_loop_until_publication,
        first_frame_fence,
        first_frame_admission,
        phase_cancelled,
        show_before_first_frame,
        label,
        teardown_lease,
    } = request;
    let stop = phase_cancelled.unwrap_or_else(|| Arc::new(AtomicBool::new(false)));
    if stop.load(Ordering::Acquire) {
        return Err("Display output native phase expired before resident initialization".into());
    }
    ensure_video_output_allowed(&engine)?;
    let extent = match initial_size_override {
        Some((width, height)) => (width.max(1), height.max(1)),
        None => {
            let size = window.inner_size().map_err(|error| error.to_string())?;
            (size.width.max(1), size.height.max(1))
        }
    };
    #[cfg(feature = "qa-native-frame-timing")]
    video::native_frame_timing::prepare_automatic_thread()?;
    #[cfg(feature = "qa-native-frame-timing")]
    let _trace_retirement = video::native_frame_timing::ThreadRetirementGuard::new();
    let window = Arc::new(window);
    let decoder = ndi_transport::NdiAwareVideoFrameDecoder::from_env()
        .with_stop_signal(Arc::clone(&stop))
        .with_capture_inputs(capture_inputs);
    #[cfg(feature = "ndi")]
    let decoder = decoder.with_ndi_inputs(ndi_inputs);
    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    let decoder = decoder.with_spout_inputs(spout_inputs);
    let renderer = video::VideoPreviewRenderer::with_frame_provider(
        video::VideoRuntimeConfig::default(),
        video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
    );
    let (session, creation_error) = retain_creation_owner(
        video::NativeResidentDisplaySession::new(Arc::clone(&window), output_id)
            .map_err(|failure| failure.into_parts()),
    )?;
    let retirement = ResidentRetirement::default();
    let mut lease = ResourceLease {
        resources: Some(Box::new(Resources {
            session,
            renderer,
            stop_started: false,
            configured_extent: None,
        })),
        retirement: retirement.clone(),
    };
    let mut health = live_video_output_health::OutputHealthWriter::register(output_id, "display");
    let mut observed_follow = None;
    let mut validation = None;
    let mut boundary = None;
    let started = Instant::now();
    #[cfg(feature = "qa-native-frame-timing")]
    let trace_epoch = engine.output_ownership_status().epoch;
    #[cfg(feature = "qa-native-frame-timing")]
    let first_trace =
        video::native_frame_timing::begin_parent(output_id, trace_epoch, extent.0, extent.1);
    let first = (|| -> Result<FrameProgress, FrameFault> {
        if let Some(error) = creation_error {
            return Err(error.into());
        }
        let resources = lease.resources();
        resources
            .session
            .install_renderer(&mut resources.renderer)?;
        let generation = resources.session.reserve_geometry(extent.0, extent.1)?;
        let render_started = Instant::now();
        #[cfg(feature = "qa-native-frame-timing")]
        let prepare_trace =
            video::native_frame_timing::span(video::native_frame_timing::Phase::Prepare);
        #[cfg(feature = "qa-native-frame-timing")]
        let demand_before = native_frame_demand_counters(&resources.renderer);
        let prepared = prepare(
            resources,
            &engine,
            output_id,
            extent,
            &label,
            unpublished_snapshot.as_ref(),
            &stop,
            &mut observed_follow,
        );
        #[cfg(feature = "qa-native-frame-timing")]
        if let (Some(before), Some(after)) = (
            demand_before,
            native_frame_demand_counters(&resources.renderer),
        ) {
            video::native_frame_timing::set_demand_counters(before, after);
        }
        #[cfg(feature = "qa-native-frame-timing")]
        drop(prepare_trace);
        let render_time = render_started.elapsed();
        with_prepared_output(prepared, |prepared| {
            if stop.load(Ordering::Acquire) {
                return Ok(startup_rejected(
                    &mut boundary,
                    "Display output native phase expired before first physical frame",
                ));
            }
            if first_frame_fence
                .as_ref()
                .is_some_and(|expected| !exact_display_first_frame_fence_matches(&engine, expected))
            {
                return Ok(startup_rejected(
                    &mut boundary,
                    "Display output authority changed before first physical frame",
                ));
            }
            if let Some(admit) = first_frame_admission.as_ref() {
                if let Err(error) = admit() {
                    return Ok(startup_rejected(&mut boundary, error));
                }
            }
            let _permit = match engine.acquire_video_output() {
                Ok(permit) => permit,
                Err(error) => return Ok(startup_rejected(&mut boundary, error)),
            };
            let physical_result: Result<FrameProgress, FrameFault> = (|| {
                let actual = if show_before_first_frame {
                    window.show().map_err(|error| {
                        format!("Native resident first-frame show failed: {error}")
                    })?;
                    let size = activate_native_video_output_window_after_show(
                        &window, extent.0, extent.1, fullscreen,
                    )?;
                    (size.width, size.height)
                } else {
                    let size = window.inner_size().map_err(|error| error.to_string())?;
                    (size.width, size.height)
                };
                if actual != extent {
                    return Err(format!("Native resident first geometry changed: prepared {extent:?}, observed {actual:?}").into());
                }
                if stop.load(Ordering::Acquire) {
                    return Ok(startup_rejected(
                        &mut boundary,
                        "Native resident first configure cancelled",
                    ));
                }
                if let Some(admit) = first_frame_admission.as_ref() {
                    if let Err(error) = admit() {
                        return Ok(startup_rejected(&mut boundary, error));
                    }
                }
                if let PreparedOutput::Presentable(frame) = &prepared {
                    let gate = PresentAuthority {
                        engine: &engine,
                        captured: &frame.authority,
                        follow: frame.follow_identity,
                        stop: &stop,
                        window: Some(&window),
                        extent,
                        first_fence: first_frame_fence.as_ref(),
                        first_admission: first_frame_admission.as_deref(),
                    };
                    if video::NativeResidentPresentationAuthority::revalidate(
                        &gate,
                        video::NativeResidentPresentBoundary::BeforeAcquire,
                    ) != video::NativeResidentPresentDecision::Permit
                    {
                        return Ok(startup_rejected(
                            &mut boundary,
                            "Native resident first configuration authority was revoked",
                        ));
                    }
                }
                resources
                    .session
                    .configure_reserved(extent.0, extent.1, generation)?;
                resources.configured_extent = Some(extent);
                let settlement_only = matches!(&prepared, PreparedOutput::SettlementOnly(_));
                let delivered = present(
                    resources,
                    prepared,
                    &engine,
                    &stop,
                    &window,
                    first_frame_fence.as_ref(),
                    first_frame_admission.as_deref(),
                    &mut health,
                    render_time,
                    &mut validation,
                    &mut boundary,
                )?;
                if !delivered && !settlement_only {
                    return Ok(FrameProgress::StartupRejected);
                }
                if stop.load(Ordering::Acquire) {
                    return Ok(startup_rejected(
                        &mut boundary,
                        "Native resident first frame cancelled before handoff",
                    ));
                }
                Ok(FrameProgress::Complete)
            })();
            if let Err(error) = &physical_result {
                if !stop.load(Ordering::Acquire) {
                    arm_native_video_output_failure_fence(
                        &engine,
                        &teardown_lease,
                        &label,
                        error.message().to_string(),
                    );
                }
            }
            physical_result
        })
    })();
    if let Ok(FrameProgress::Interrupted(interruption)) = &first {
        boundary = Some(interruption.reason().into());
    }
    let startup_error = match &first {
        Ok(FrameProgress::Complete) => None,
        Ok(FrameProgress::Interrupted(interruption)) => Some(interruption.reason().to_string()),
        Ok(FrameProgress::StartupRejected) => Some(
            boundary
                .clone()
                .unwrap_or_else(|| "Native resident first frame was not presented".into()),
        ),
        Err(error) => Some(error.message().to_string()),
    };
    record_metrics(
        &metrics,
        lease.resources(),
        extent,
        started,
        first
            .as_ref()
            .err()
            .map(|error| error.message().to_string()),
        validation,
        boundary,
    );
    #[cfg(feature = "qa-native-frame-timing")]
    {
        if first.is_err() {
            video::native_frame_timing::set_outcome(
                if stop.load(Ordering::Acquire)
                    && !first
                        .as_ref()
                        .err()
                        .is_some_and(FrameFault::is_preparation_fault)
                {
                    video::native_frame_timing::Outcome::Cancelled
                } else {
                    video::native_frame_timing::Outcome::Failed
                },
            );
        } else if startup_error.is_some() {
            video::native_frame_timing::set_outcome(
                if stop.load(Ordering::Acquire)
                    || matches!(
                        &first,
                        Ok(FrameProgress::Interrupted(
                            PrepareInterruption::Cancelled { .. }
                        ))
                    )
                {
                    video::native_frame_timing::Outcome::Cancelled
                } else {
                    video::native_frame_timing::Outcome::ZeroPresent
                },
            );
        }
        drop(first_trace);
        video::native_frame_timing::retire_thread();
    }
    if let Some(error) = startup_error {
        if first
            .as_ref()
            .err()
            .is_some_and(|fault| fault.is_preparation_fault() || !stop.load(Ordering::Acquire))
        {
            let ack = observed_follow.map(|follow| {
                native_timeline_follow_video_ack_for_identity(
                    follow,
                    output_id,
                    TimelineFollowSettlementAckResult::Fault {
                        fault: error.clone(),
                    },
                )
            });
            retain_native_video_output_failure_before_ack(
                &engine,
                &teardown_lease,
                &label,
                error.clone(),
                ack,
                |ack| acknowledge_native_timeline_follow_video_settlement(&engine, ack, &stop),
            );
        } else if first.is_ok() {
            // A fenced first frame cannot publish a worker after losing its
            // authority. Roll it back without re-preparing the same candidate.
            stop.store(true, Ordering::Release);
        }
        let cleanup = lease.finish_stop();
        if let Err(cleanup_error) = &cleanup {
            arm_native_video_output_failure_fence(
                &engine,
                &teardown_lease,
                &label,
                cleanup_error.clone(),
            );
        }
        drop(lease);
        retain_failed_start(
            &window,
            &label,
            Arc::clone(&stop),
            Arc::clone(&teardown_lease),
            retirement,
        );
        return Err(match cleanup {
            Ok(()) => error,
            Err(cleanup) => format!("{error}; {cleanup}"),
        });
    }
    let event_stop = Arc::clone(&stop);
    window.on_window_event(move |event| {
        if matches!(
            event,
            tauri::WindowEvent::CloseRequested { .. } | tauri::WindowEvent::Destroyed
        ) {
            event_stop.store(true, Ordering::Release);
        }
    });
    let start_gate = gate_render_loop_until_publication.then(|| Arc::new(AtomicBool::new(false)));
    let worker_gate = start_gate.clone();
    let worker_stop = Arc::clone(&stop);
    let worker_window = Arc::clone(&window);
    let worker_lease = Arc::clone(&teardown_lease);
    let worker_label = label.clone();
    let join = std::thread::Builder::new()
        .name(format!("syndocal-video-output-{output_id}"))
        .spawn(move || {
            let result = run(
                &mut lease,
                &worker_window,
                &engine,
                output_id,
                &metrics,
                &worker_stop,
                worker_gate.as_ref(),
                &worker_label,
                &worker_lease,
                &mut health,
            );
            if let Err(error) = &result {
                if !worker_stop.load(Ordering::Acquire) {
                    arm_native_video_output_failure_fence(
                        &engine,
                        &worker_lease,
                        &worker_label,
                        error.clone(),
                    );
                }
            }
            let cleanup = lease.finish_stop();
            if let Err(error) = &cleanup {
                arm_native_video_output_failure_fence(
                    &engine,
                    &worker_lease,
                    &worker_label,
                    error.clone(),
                );
            }
            #[cfg(feature = "qa-native-frame-timing")]
            video::native_frame_timing::retire_thread();
            match (result, cleanup) {
                (Ok(()), Ok(())) => Ok(()),
                (Err(error), Ok(())) | (Ok(()), Err(error)) => Err(error),
                (Err(error), Err(cleanup)) => Err(format!("{error}; {cleanup}")),
            }
        });
    let join = match join {
        Ok(join) => join,
        Err(error) => {
            let cleanup =
                retirement.poll_with_deadline(Instant::now() + Duration::from_millis(500));
            retain_failed_start(
                &window,
                &label,
                Arc::clone(&stop),
                Arc::clone(&teardown_lease),
                retirement,
            );
            return Err(format!(
                "Native resident output thread failed: {error}; cleanup={cleanup:?}"
            ));
        }
    };
    let mut worker = NativeVideoOutputWorker::with_teardown_lease_and_start_gate(
        stop,
        join,
        teardown_lease,
        start_gate,
    );
    worker.resident_retirements.push(retirement);
    Ok(worker)
}

fn run(
    lease: &mut ResourceLease,
    window: &tauri::Window,
    engine: &EngineHandle,
    output_id: VideoOutputId,
    metrics: &Arc<Mutex<NativeVideoOutputMetrics>>,
    stop: &Arc<AtomicBool>,
    start_gate: Option<&Arc<AtomicBool>>,
    label: &str,
    teardown: &Arc<Mutex<Option<engine::OutputOwnershipTeardownLease>>>,
    health: &mut live_video_output_health::OutputHealthWriter,
) -> Result<(), String> {
    #[cfg(feature = "qa-native-frame-timing")]
    video::native_frame_timing::prepare_thread().map_err(str::to_string)?;
    #[cfg(feature = "qa-native-frame-timing")]
    let _thread_retirement = video::native_frame_timing::ThreadRetirementGuard::new();
    if let Some(gate) = start_gate {
        while !gate.load(Ordering::Acquire) && !stop.load(Ordering::Acquire) {
            std::thread::park_timeout(Duration::from_millis(1));
        }
    }
    health.restart_cadence();
    let mut pacer =
        video_output_pacer::VideoOutputPacer::new(Duration::from_nanos(1_000_000_000 / 60));
    while !stop.load(Ordering::Acquire) {
        let started = Instant::now();
        let size = window.inner_size().map_err(|error| error.to_string())?;
        let extent = (size.width, size.height);
        let mut validation = None;
        let mut boundary = None;
        let mut observed_follow = None;
        #[cfg(feature = "qa-native-frame-timing")]
        let frame_trace = video::native_frame_timing::begin_parent(
            output_id,
            engine.output_ownership_status().epoch,
            extent.0,
            extent.1,
        );
        #[cfg(feature = "qa-native-frame-timing")]
        video::native_frame_timing::set_outcome(video::native_frame_timing::Outcome::ZeroPresent);
        if native_video_output_render_extent(extent.0, extent.1).is_none() {
            record_metrics(
                metrics,
                lease.resources(),
                extent,
                started,
                None,
                None,
                Some(format!(
                    "Native resident physical extent is non-presentable ({extent:?})"
                )),
            );
            #[cfg(feature = "qa-native-frame-timing")]
            drop(frame_trace);
            pacer.wait_after_frame();
            continue;
        }
        let _permit = match engine.acquire_video_output() {
            Ok(permit) => permit,
            Err(error) => {
                record_metrics(
                    metrics,
                    lease.resources(),
                    extent,
                    started,
                    None,
                    None,
                    Some(error),
                );
                if stop.load(Ordering::Acquire) {
                    break;
                }
                if engine.output_ownership_status().state
                    == protocol::OutputOwnershipState::Transitioning
                {
                    if wait_for_native_video_output_transition_retry(stop) {
                        continue;
                    }
                }
                report_native_timeline_follow_video_result(
                    engine,
                    output_id,
                    TimelineFollowSettlementAckResult::NotApplicable,
                    stop,
                );
                break;
            }
        };
        let result = (|| -> Result<FrameProgress, FrameFault> {
            let resources = lease.resources();
            resources.session.poll_present_retirements()?;
            if resources.configured_extent != Some(extent) {
                observed_follow = engine
                    .timeline_follow_video_render_snapshot()
                    .as_ref()
                    .map(native_timeline_follow_active_identity);
                resources.session.configure(extent.0, extent.1)?;
                resources.configured_extent = Some(extent);
            }
            let render_started = Instant::now();
            #[cfg(feature = "qa-native-frame-timing")]
            let prepare_trace =
                video::native_frame_timing::span(video::native_frame_timing::Phase::Prepare);
            #[cfg(feature = "qa-native-frame-timing")]
            let demand_before = native_frame_demand_counters(&resources.renderer);
            let prepared = prepare(
                resources,
                engine,
                output_id,
                extent,
                label,
                None,
                stop,
                &mut observed_follow,
            );
            #[cfg(feature = "qa-native-frame-timing")]
            if let (Some(before), Some(after)) = (
                demand_before,
                native_frame_demand_counters(&resources.renderer),
            ) {
                video::native_frame_timing::set_demand_counters(before, after);
            }
            #[cfg(feature = "qa-native-frame-timing")]
            drop(prepare_trace);
            with_prepared_output(prepared, |prepared| {
                present(
                    resources,
                    prepared,
                    engine,
                    stop,
                    window,
                    None,
                    None,
                    health,
                    render_started.elapsed(),
                    &mut validation,
                    &mut boundary,
                )
                .map(|_| FrameProgress::Complete)
                .map_err(FrameFault::Operation)
            })
        })();
        if let Ok(FrameProgress::Interrupted(interruption)) = &result {
            boundary = Some(interruption.reason().into());
        }
        record_metrics(
            metrics,
            lease.resources(),
            extent,
            started,
            result
                .as_ref()
                .err()
                .map(|error| error.message().to_string()),
            validation,
            boundary,
        );
        if let Ok(FrameProgress::Interrupted(interruption)) = &result {
            if interruption.ends_session() {
                #[cfg(feature = "qa-native-frame-timing")]
                if matches!(interruption, PrepareInterruption::Cancelled { .. }) {
                    video::native_frame_timing::set_outcome(
                        video::native_frame_timing::Outcome::Cancelled,
                    );
                }
                // Leave the visible output lifecycle through its normal close
                // observer. The caller still retires/retains the whole GPU owner.
                if let Err(error) = close_for_orderly_retirement(stop, || {
                    window.close().map_err(|error| error.to_string())
                }) {
                    let error = format!(
                        "Native resident orderly retirement could not close its window: {error}"
                    );
                    arm_native_video_output_failure_fence(engine, teardown, label, error.clone());
                    return Err(error);
                }
                return Ok(());
            }
            // Healthy revocation consumes no physical callback or Follow ACK;
            // pacing below starts a new tick with a new authority/snapshot read.
        }
        if let Err(error) = result {
            #[cfg(feature = "qa-native-frame-timing")]
            video::native_frame_timing::set_outcome(
                if stop.load(Ordering::Acquire) && !error.is_preparation_fault() {
                    video::native_frame_timing::Outcome::Cancelled
                } else {
                    video::native_frame_timing::Outcome::Failed
                },
            );
            // The facade has already distinguished a real GPU fault from a
            // control interruption. A later ownership change cannot hide it.
            let result = error.into_worker_result(stop, engine.output_ownership_status().state);
            if let Err(error) = result {
                let ack = observed_follow.map(|follow| {
                    native_timeline_follow_video_ack_for_identity(
                        follow,
                        output_id,
                        TimelineFollowSettlementAckResult::Fault {
                            fault: error.clone(),
                        },
                    )
                });
                let error = retain_native_video_output_failure_before_ack(
                    engine,
                    teardown,
                    label,
                    error,
                    ack,
                    |ack| acknowledge_native_timeline_follow_video_settlement(engine, ack, stop),
                );
                let _ = window.close();
                return Err(error);
            }
            break;
        }
        #[cfg(feature = "qa-native-frame-timing")]
        drop(frame_trace);
        pacer.wait_after_frame();
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn native_display_resident_prepare_revocation_waits_for_a_fresh_tick() {
        let authority = AtomicBool::new(true);
        let physical_calls = Cell::new(0);
        let deliveries = Cell::new(0);
        let applied = Cell::new(0);
        let faults = Cell::new(0);
        let physical = |_: u64| {
            physical_calls.set(physical_calls.get() + 1);
            deliveries.set(deliveries.get() + 1);
            applied.set(applied.get() + 1);
            Ok(FrameProgress::Complete)
        };

        // The first tick observes a live revocation during preparation. A
        // subsequent Permit cannot resurrect that rejected affine attempt.
        authority.store(false, Ordering::Release);
        let first: Result<u64, video::NativeResidentPrepareError> =
            if authority.load(Ordering::Acquire) {
                Ok(1)
            } else {
                Err(video::NativeResidentPrepareError::Revoked {
                    retirement_required: false,
                })
            };
        authority.store(true, Ordering::Release);
        let progress = with_prepared_output(first, &physical).unwrap_or_else(|_| {
            faults.set(faults.get() + 1);
            panic!("authority revocation is not a physical fault")
        });
        assert_eq!(
            progress,
            FrameProgress::Interrupted(PrepareInterruption::Revoked {
                retirement_required: false
            })
        );
        assert_eq!(
            (
                physical_calls.get(),
                deliveries.get(),
                applied.get(),
                faults.get()
            ),
            (0, 0, 0, 0)
        );

        // Only a newly captured next tick reaches the physical callback.
        let next = authority.load(Ordering::Acquire).then_some(2).unwrap();
        assert_eq!(
            with_prepared_output(Ok(next), physical).unwrap(),
            FrameProgress::Complete
        );
        assert_eq!(
            (
                physical_calls.get(),
                deliveries.get(),
                applied.get(),
                faults.get()
            ),
            (1, 1, 1, 0)
        );
    }

    #[test]
    fn native_display_resident_prepare_terminal_controls_close_without_fault_ack() {
        for error in [
            video::NativeResidentPrepareError::Cancelled {
                retirement_required: false,
            },
            video::NativeResidentPrepareError::Cancelled {
                retirement_required: true,
            },
            video::NativeResidentPrepareError::Revoked {
                retirement_required: true,
            },
        ] {
            let stop = AtomicBool::new(false);
            let closes = Cell::new(0);
            let progress = with_prepared_output::<()>(Err(error), |_| {
                panic!("cancelled/revoked preparation cannot present or settle Follow")
            })
            .unwrap();
            let FrameProgress::Interrupted(interruption) = progress else {
                panic!("terminal preparation control must remain typed")
            };
            assert!(interruption.ends_session());
            close_for_orderly_retirement(&stop, || {
                assert!(
                    stop.load(Ordering::Acquire),
                    "Stop precedes visible-window retirement"
                );
                closes.set(closes.get() + 1);
                Ok(())
            })
            .unwrap();
            assert_eq!(closes.get(), 1);
        }
    }

    #[test]
    fn native_display_resident_prepare_fault_survives_later_stop_or_authority_change() {
        let error = with_prepared_output::<()>(
            Err(video::NativeResidentPrepareError::Fault(
                "real GPU validation failure".into(),
            )),
            |_| panic!("failed preparation cannot present"),
        )
        .unwrap_err();
        assert!(error.is_preparation_fault());
        assert_eq!(
            error.into_worker_result(
                &AtomicBool::new(true),
                protocol::OutputOwnershipState::Transitioning,
            ),
            Err("real GPU validation failure".into())
        );
    }

    #[test]
    fn native_display_resident_creation_failure_retains_partial_owner() {
        struct TrackedOwner<'a>(&'a Cell<usize>);
        impl Drop for TrackedOwner<'_> {
            fn drop(&mut self) {
                self.0.set(self.0.get() + 1);
            }
        }

        let drops = Cell::new(0);
        let (owner, failure) = retain_creation_owner(Err((
            "presenter initialization failed".into(),
            Some(Box::new(TrackedOwner(&drops))),
        )))
        .unwrap();
        assert_eq!(failure.as_deref(), Some("presenter initialization failed"));
        assert_eq!(drops.get(), 0);
        drop(owner);
        assert_eq!(drops.get(), 1);

        let (owner, failure) = retain_creation_owner(Ok(TrackedOwner(&drops))).unwrap();
        assert_eq!(failure, None);
        assert_eq!(drops.get(), 1);
        drop(owner);
        assert_eq!(drops.get(), 2);
        assert!(matches!(
            retain_creation_owner::<()>(Err(("adapter unavailable".into(), None))),
            Err(error) if error == "adapter unavailable"
        ));
    }

    #[test]
    fn native_display_resident_not_presented_never_delivers_or_acks() {
        for reason in [
            "Cancelled",
            "Revoked",
            "StaleGeometry",
            "SurfaceTimeout",
            "SurfaceLostOrOutdated",
        ] {
            let stop = AtomicBool::new(false);
            let deliveries = Cell::new(0);
            let acknowledgements = Cell::new(0);
            let mut boundary = None;
            assert!(!complete_present(
                video::NativeResidentPresentResult::NotPresented {
                    reason: reason.into()
                },
                &stop,
                || deliveries.set(deliveries.get() + 1),
                || acknowledgements.set(acknowledgements.get() + 1),
                &mut boundary,
            ));
            assert_eq!(deliveries.get(), 0);
            assert_eq!(acknowledgements.get(), 0);
            assert_eq!(boundary.as_deref(), Some(reason));
        }
    }

    #[test]
    fn native_display_resident_presented_is_the_only_delivery_and_follow_success() {
        for hard_blackout in [false, true] {
            let stop = AtomicBool::new(false);
            let deliveries = Cell::new(0);
            let acknowledgements = Cell::new(0);
            let mut boundary = None;
            assert!(complete_present(
                video::NativeResidentPresentResult::Presented { hard_blackout },
                &stop,
                || deliveries.set(deliveries.get() + 1),
                || acknowledgements.set(acknowledgements.get() + 1),
                &mut boundary,
            ));
            assert_eq!(deliveries.get(), 1);
            assert_eq!(acknowledgements.get(), 1);
            assert_eq!(boundary, None);
        }
    }

    #[test]
    fn native_display_resident_stop_after_present_cannot_ack_follow() {
        let stop = AtomicBool::new(false);
        let deliveries = Cell::new(0);
        let acknowledgements = Cell::new(0);
        let mut boundary = None;
        assert!(complete_present(
            video::NativeResidentPresentResult::Presented {
                hard_blackout: false
            },
            &stop,
            || {
                deliveries.set(deliveries.get() + 1);
                stop.store(true, Ordering::Release);
            },
            || acknowledgements.set(acknowledgements.get() + 1),
            &mut boundary,
        ));
        assert_eq!(deliveries.get(), 1);
        assert_eq!(acknowledgements.get(), 0);
    }

    #[test]
    fn native_display_resident_metrics_mark_unmeasured_legacy_allocations() {
        let mut metrics = NativeVideoOutputMetrics::default();
        let legacy = serde_json::to_value(metrics.snapshot()).unwrap();
        assert!(legacy.get("legacy_buffer_stats_available").is_none());
        assert!(legacy.get("resident_presentation").is_none());
        metrics.record_resident(
            1920,
            1080,
            Duration::from_millis(10),
            7,
            2,
            video::VideoDecoderDiagnostics::default(),
            None,
            None,
            Some("test: no captured validation identity".into()),
        );
        let resident = serde_json::to_value(metrics.snapshot()).unwrap();
        assert_eq!(resident["legacy_buffer_stats_available"], false);
        assert_eq!(resident["resident_presentation"]["frames_presented"], 7);
        assert_eq!(
            resident["resident_presentation"]["hard_blackout_frames_presented"],
            2
        );
        assert_eq!(metrics.lifetime_frame_count, 0);
    }
}
