//! Canonical persisted video-output authoring. Native work runs between short
//! exact-authority boundaries while the project reservation fences other edits.
use super::*;
use protocol::control_plane_command::{
    ExternalVideoOutputSpecV2, OutputControlActionV2, VideoOutputConfigurationV2,
};

pub(super) struct ControlRequest<'a> {
    pub(super) expected_fence: &'a OutputControlFenceV1,
    pub(super) lease_request: &'a OutputLeaseRequest,
    pub(super) lease_now_ms: u64,
    pub(super) expected_owner_principal: &'a str,
    pub(super) expected_owner_window_label: &'a str,
    pub(super) expected_owner_incarnation: u64,
}

#[derive(Clone)]
struct Mutation {
    output_id: VideoOutputId,
    expected: Option<VideoOutputSummary>,
    replacement: Option<VideoOutputSummary>,
    history_label: &'static str,
}

fn endpoint_conflicts(
    outputs: &[VideoOutputSummary],
    kind: &VideoOutputKind,
    endpoint: &str,
    except: Option<VideoOutputId>,
) -> bool {
    outputs.iter().any(|output| {
        Some(output.id) != except
            && &output.kind == kind
            && output
                .endpoint_name
                .as_deref()
                .is_some_and(|current| current.trim().eq_ignore_ascii_case(endpoint.trim()))
    })
}

fn validate_add(
    snapshot: &protocol::VideoSnapshot,
    spec: &ExternalVideoOutputSpecV2,
) -> Result<(), String> {
    spec.validate()
        .map_err(|_| "External video output specification is invalid".to_string())?;
    ensure_video_output_backend_available(&spec.kind)?;
    if !snapshot
        .compositions
        .iter()
        .any(|composition| composition.id == spec.composition_id)
    {
        return Err("The selected video composition no longer exists".to_string());
    }
    if endpoint_conflicts(&snapshot.outputs, &spec.kind, &spec.endpoint_name, None) {
        return Err("An output of this kind already uses that endpoint name".to_string());
    }
    Ok(())
}

fn validate_configure(
    outputs: &[VideoOutputSummary],
    output_id: VideoOutputId,
    spec: &VideoOutputConfigurationV2,
) -> Result<(), String> {
    spec.validate()
        .map_err(|_| "Video output configuration is invalid".to_string())?;
    outputs
        .iter()
        .find(|output| output.id == output_id)
        .ok_or_else(|| format!("Video output {output_id} was not found"))?;
    ensure_video_output_backend_available(&spec.kind)?;
    if let Some(endpoint) = spec.endpoint_name.as_deref() {
        if endpoint_conflicts(outputs, &spec.kind, endpoint, Some(output_id)) {
            return Err("An output of this kind already uses that endpoint name".to_string());
        }
    }
    if spec.kind == VideoOutputKind::Display
        && outputs.iter().any(|other| {
            other.id != output_id
                && other.kind == VideoOutputKind::Display
                && (other.monitor_id == spec.monitor_index
                    || other.monitor_identity == spec.monitor_identity)
        })
    {
        return Err("A Display output already targets this monitor".to_string());
    }
    Ok(())
}

pub(super) fn validate_current(
    state: &AppState,
    action: &OutputControlActionV2,
) -> Result<(), String> {
    let snapshot = state.engine.video_snapshot();
    match action {
        OutputControlActionV2::AddExternalOutput { spec, .. } => validate_add(&snapshot, spec),
        OutputControlActionV2::ConfigureVideoOutput {
            output_id, spec, ..
        } => validate_configure(&snapshot.outputs, *output_id, spec),
        OutputControlActionV2::RemoveVideoOutput { output_id, .. } => {
            if snapshot
                .outputs
                .iter()
                .any(|output| output.id == *output_id)
            {
                Ok(())
            } else {
                Err(format!("Video output {output_id} was not found"))
            }
        }
        _ => Err("Unsupported video output authoring operation".to_string()),
    }
}

fn mutation_from_snapshot(
    snapshot: &EngineSnapshot,
    action: &OutputControlActionV2,
    new_output_id: VideoOutputId,
) -> Result<Mutation, String> {
    let output = |output_id| {
        snapshot
            .video
            .outputs
            .iter()
            .find(|output| output.id == output_id)
            .cloned()
            .ok_or_else(|| format!("Video output {output_id} was not found"))
    };
    match action {
        OutputControlActionV2::AddDisplay { spec, .. } => {
            spec.validate()
                .map_err(|_| "Display output specification is invalid".to_string())?;
            if snapshot.video.outputs.iter().any(|output| {
                output.kind == VideoOutputKind::Display
                    && (output.monitor_id == Some(spec.monitor_index)
                        || output.monitor_identity.as_deref()
                            == Some(spec.monitor_identity.as_str()))
            }) {
                return Err("A Display output already targets this monitor".to_string());
            }
            Ok(Mutation {
                output_id: new_output_id,
                expected: None,
                replacement: Some(VideoOutputSummary {
                    id: new_output_id,
                    label: spec.label.trim().to_string(),
                    kind: VideoOutputKind::Display,
                    enabled: true,
                    composition_id: 1,
                    fullscreen: spec.fullscreen,
                    monitor_id: Some(spec.monitor_index),
                    monitor_identity: Some(spec.monitor_identity.clone()),
                    width: spec.width,
                    height: spec.height,
                    endpoint_name: None,
                    opacity: 1.0,
                    blackout: false,
                    mapping: VideoOutputMapping::default(),
                }),
                history_label: "Add Display Output",
            })
        }
        OutputControlActionV2::AddExternalOutput { spec, .. } => {
            validate_add(&snapshot.video, spec)?;
            Ok(Mutation {
                output_id: new_output_id,
                expected: None,
                replacement: Some(VideoOutputSummary {
                    id: new_output_id,
                    label: spec.label.trim().to_string(),
                    kind: spec.kind.clone(),
                    enabled: true,
                    composition_id: spec.composition_id,
                    fullscreen: false,
                    monitor_id: None,
                    monitor_identity: None,
                    width: spec.width,
                    height: spec.height,
                    endpoint_name: Some(spec.endpoint_name.trim().to_string()),
                    opacity: 1.0,
                    blackout: false,
                    mapping: VideoOutputMapping::default(),
                }),
                history_label: "Add External Video Output",
            })
        }
        OutputControlActionV2::ConfigureVideoOutput {
            output_id, spec, ..
        } => {
            validate_configure(&snapshot.video.outputs, *output_id, spec)?;
            let before = output(*output_id)?;
            let mut after = before.clone();
            after.kind = spec.kind.clone();
            after.label = spec.label.trim().to_string();
            after.width = spec.width;
            after.height = spec.height;
            after.fullscreen = spec.fullscreen;
            after.monitor_id = spec.monitor_index;
            after.monitor_identity = spec.monitor_identity.clone();
            after.endpoint_name = spec
                .endpoint_name
                .as_ref()
                .map(|name| name.trim().to_string());
            Ok(Mutation {
                output_id: *output_id,
                expected: Some(before),
                replacement: Some(after),
                history_label: "Configure Video Output",
            })
        }
        OutputControlActionV2::RemoveVideoOutput { output_id, .. } => Ok(Mutation {
            output_id: *output_id,
            expected: Some(output(*output_id)?),
            replacement: None,
            history_label: "Remove Video Output",
        }),
        _ => Err("Unsupported video output authoring operation".to_string()),
    }
}

fn exact_authority(
    state: &AppState,
    coordinator: &ProjectCoordinator,
    request: &ControlRequest<'_>,
) -> Result<(), String> {
    if !control_plane_runtime::exact_output_control_owner_matches(
        state,
        request.expected_owner_principal,
        request.expected_owner_window_label,
        request.expected_owner_incarnation,
    ) || !control_plane_runtime::exact_output_control_fence_matches(
        state,
        coordinator,
        request.expected_fence,
    ) {
        return Err("Output control authority changed during video output authoring".to_string());
    }
    ensure_no_pending_project_transaction(coordinator)
}

fn fault_after_admission(state: &AppState, message: String) -> String {
    // A physical delta or ambiguous Engine ACK cannot be represented as a
    // rejected edit with no effect. Keep the durable Pending and project fence
    // until process restart and Recovery establish one authoritative state.
    state
        .project_external_command_admission
        .project_transaction_publication_faulted
        .store(true, Ordering::Release);
    state
        .engine
        .mark_output_ownership_transition_failure(message.clone());
    format!(
        "Video output authoring is in doubt; pending receipt retained: Restart Syndocal before Recovery. {message}"
    )
}

fn validate_current_lease(
    state: &AppState,
    registry: &OutputLeaseRegistry,
    request: &OutputLeaseRequest,
) -> Result<(), String> {
    let mut candidate = registry.clone();
    let receipt = candidate
        .submit_request(request, state.output_lease_now_ms()?)
        .map_err(|error| format!("Video output lease revalidation failed: {error:?}"))?;
    receipt.outcome.map(|_| ()).map_err(|error| {
        format!("Video output lease expired or changed before physical work: {error:?}")
    })
}

#[derive(Clone)]
struct PhysicalAuthority {
    fence: OutputControlFenceV1,
    lease_request: OutputLeaseRequest,
    owner_principal: String,
    window_label: String,
    owner_incarnation: u64,
}

impl PhysicalAuthority {
    fn capture(request: &ControlRequest<'_>) -> Self {
        Self {
            fence: request.expected_fence.clone(),
            lease_request: request.lease_request.clone(),
            owner_principal: request.expected_owner_principal.to_string(),
            window_label: request.expected_owner_window_label.to_string(),
            owner_incarnation: request.expected_owner_incarnation,
        }
    }

    fn validate(&self, state: &AppState) -> Result<(), String> {
        if !state.project_transaction_active.load(Ordering::Acquire)
            || state
                .project_external_command_admission
                .project_transaction_publication_faulted
                .load(Ordering::Acquire)
        {
            return Err("Video output authoring reservation is not active".to_string());
        }
        // Start admission can execute while transport locks are held. Never
        // wait for authority locks in the opposite order to a safety teardown.
        let _lifecycle = state
            .standby_sync_lifecycle
            .try_lock()
            .map_err(|_| "Video output lifecycle is busy before native start".to_string())?;
        let _owner_rotation = state
            .project_transaction_owner_rotation
            .try_lock()
            .map_err(|_| "Video output owner authority is busy before native start".to_string())?;
        let _external = state
            .project_external_command_admission
            .gate
            .try_lock()
            .map_err(|_| {
                "Video output project admission is busy before native start".to_string()
            })?;
        let coordinator = state.project_coordinator.try_lock().map_err(|_| {
            "Video output project authority is busy before native start".to_string()
        })?;
        let _transition = state.output_ownership_transition.try_lock().map_err(|_| {
            "Video output ownership transition is busy before native start".to_string()
        })?;
        let request = ControlRequest {
            expected_fence: &self.fence,
            lease_request: &self.lease_request,
            lease_now_ms: 0,
            expected_owner_principal: &self.owner_principal,
            expected_owner_window_label: &self.window_label,
            expected_owner_incarnation: self.owner_incarnation,
        };
        exact_authority(state, &coordinator, &request)?;
        let registry = state
            .output_lease_registry
            .try_lock()
            .map_err(|_| "Video output lease authority is busy before native start".to_string())?;
        validate_current_lease(state, &registry, &self.lease_request)
    }
}

#[derive(Default)]
struct NativeRetirement {
    reopen_display: bool,
}

trait NativeOperations {
    fn validate(&self, action: &OutputControlActionV2) -> Result<(), String>;
    fn retire(
        &self,
        state: &AppState,
        before: &EngineSnapshot,
        mutation: &Mutation,
        request: &ControlRequest<'_>,
    ) -> Result<NativeRetirement, String>;
    fn synchronize(
        &self,
        state: &AppState,
        candidate: &EngineSnapshot,
        mutation: &Mutation,
        retired: NativeRetirement,
        request: &ControlRequest<'_>,
    ) -> Result<(), String>;
}

struct TauriNativeOperations<'a> {
    app: &'a tauri::AppHandle,
    editor_window: &'a WebviewWindow,
}

impl NativeOperations for TauriNativeOperations<'_> {
    fn validate(&self, action: &OutputControlActionV2) -> Result<(), String> {
        let display = match action {
            OutputControlActionV2::AddDisplay { spec, .. } => Some(spec.clone()),
            OutputControlActionV2::ConfigureVideoOutput { spec, .. } => spec.display_spec(),
            _ => None,
        };
        if let Some(display) = display {
            let monitor = authoritative_video_display_monitor(
                self.app,
                display.monitor_index,
                &display.monitor_identity,
            )?;
            validate_display_monitor_dimensions(
                display.width,
                display.height,
                monitor.physical_width,
                monitor.physical_height,
            )?;
        }
        Ok(())
    }

    fn retire(
        &self,
        state: &AppState,
        _before: &EngineSnapshot,
        mutation: &Mutation,
        request: &ControlRequest<'_>,
    ) -> Result<NativeRetirement, String> {
        let Some(output) = mutation.expected.as_ref() else {
            return Ok(NativeRetirement::default());
        };
        if output.kind != VideoOutputKind::Display {
            // Route identity does not contain dimensions. Removing only this
            // route first guarantees an acknowledged sender recreation even
            // when an endpoint name stays unchanged across a resize.
            let mut without_output = state.engine.video_snapshot();
            without_output
                .outputs
                .retain(|output| output.id != mutation.output_id);
            synchronize_external(state, &without_output, mutation.output_id, request)?;
            return Ok(NativeRetirement {
                reopen_display: output.enabled
                    && state.engine.output_ownership_status().video_allowed,
            });
        }
        let label = video_output_window_label(output.id, false);
        let reopen_display = self.app.windows().contains_key(&label);
        let test_label = video_output_window_label(output.id, true);
        let live_quarantine = format!("{label}::quarantine-");
        let test_quarantine = format!("{test_label}::quarantine-");
        let labels =
            native_video_output_retirement_labels(self.app, &state.native_video_output_workers)?
                .into_iter()
                .filter(|candidate| {
                    candidate == &label
                        || candidate == &test_label
                        || candidate.starts_with(&live_quarantine)
                        || candidate.starts_with(&test_quarantine)
                });
        let authority = PhysicalAuthority::capture(request);
        let errors = attempt_native_video_output_retirements(labels, |label| {
            retire_display_for_authoring(self.app, state, label, &authority)
        });
        if !errors.is_empty() {
            return Err(errors.join("; "));
        }
        state
            .native_video_output_metrics
            .lock()
            .map_err(|_| "Native output metrics lock was poisoned".to_string())?
            .remove(&output.id);
        Ok(NativeRetirement { reopen_display })
    }

    fn synchronize(
        &self,
        state: &AppState,
        _candidate: &EngineSnapshot,
        mutation: &Mutation,
        retired: NativeRetirement,
        request: &ControlRequest<'_>,
    ) -> Result<(), String> {
        if mutation
            .replacement
            .as_ref()
            .is_some_and(|output| output.kind == VideoOutputKind::Display)
        {
            if retired.reopen_display && state.engine.output_ownership_status().video_allowed {
                let output = state
                    .engine
                    .video_output_snapshot(mutation.output_id)
                    .ok_or_else(|| "Display replacement is missing".to_string())?;
                if !output.enabled {
                    return Ok(());
                }
                let monitor = authoritative_video_display_monitor_for_output(self.app, &output)?;
                validate_display_monitor_dimensions(
                    output.width,
                    output.height,
                    monitor.physical_width,
                    monitor.physical_height,
                )?;
                reopen_published_display(
                    self.app,
                    self.editor_window,
                    state,
                    &output,
                    &monitor,
                    request,
                )?;
            }
            return Ok(());
        }
        if mutation.replacement.is_none()
            && mutation
                .expected
                .as_ref()
                .is_some_and(|output| output.kind == VideoOutputKind::Display)
        {
            return Ok(());
        }
        synchronize_external(
            state,
            &state.engine.video_snapshot(),
            mutation.output_id,
            request,
        )
    }
}

/// Count only the targeted presenter's retirement. A shell without a worker
/// gets a threadless record, so quarantine/Recovery can release its counted
/// lease after a later exact HWND acknowledgement.
fn retire_display_for_authoring(
    app: &tauri::AppHandle,
    state: &AppState,
    label: &str,
    authority: &PhysicalAuthority,
) -> Result<(), String> {
    authority.validate(state)?;
    {
        let mut workers = state
            .native_video_output_workers
            .lock()
            .map_err(|_| "Native video output worker registry lock was poisoned".to_string())?;
        if let Some(worker) = workers.get(label) {
            let mut slot = worker
                .teardown_lease
                .lock()
                .map_err(|_| "Native video output teardown lease lock was poisoned".to_string())?;
            if slot.is_none() {
                *slot = Some(state.engine.begin_video_output_resource_teardown()?);
            }
        } else if app.windows().contains_key(label) {
            let lease = state.engine.begin_video_output_resource_teardown()?;
            workers.insert(
                label.to_string(),
                NativeVideoOutputWorker::for_retirement(Some(lease)),
            );
        }
    }
    retire_native_video_output_window(app, &state.native_video_output_workers, label).map(|_| ())
}

/// Constructor failure has finished its native phase but shell cleanup was
/// not acknowledged. Preserve its lease under the exact label; even poisoned
/// registry state must retain ownership instead of releasing a live resource.
fn retain_failed_display_creation(
    state: &AppState,
    label: &str,
    lease: engine::OutputOwnershipCreationLease,
) {
    let mut workers = state
        .native_video_output_workers
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let worker = workers
        .entry(label.to_string())
        .or_insert_with(|| NativeVideoOutputWorker::for_retirement(None));
    worker.retained_creation_leases.push(lease);
}

/// Attach ownership before cleanup begins, so a timed-out native worker
/// carries its own creation lease into the existing quarantine registry.
fn cleanup_prepared_display_for_authoring(
    native: &TauriDisplayOutputNativeOperations,
    mut prepared: PreparedNativeVideoOutputCandidate,
    lease: Option<engine::OutputOwnershipCreationLease>,
) -> Result<(), String> {
    if let Some(lease) = lease {
        prepared.worker.retained_creation_leases.push(lease);
    }
    native.cleanup_native_candidate(prepared)
}

/// Recreate an already-open Display from acknowledged Engine B. This must use
/// the published-frame path: AddDisplay's unpublished-frame authority rejects
/// an output id which is already present in Engine.
fn reopen_published_display(
    app: &tauri::AppHandle,
    editor_window: &WebviewWindow,
    state: &AppState,
    output: &VideoOutputSummary,
    monitor: &VideoDisplayMonitor,
    request: &ControlRequest<'_>,
) -> Result<(), String> {
    PhysicalAuthority::capture(request).validate(state)?;
    let native = TauriDisplayOutputNativeOperations {
        app: app.clone(),
        editor_window: editor_window.clone(),
    };
    let activation = state
        .engine
        .admit_output_activation(state.engine.output_ownership_status().effective_role)?;
    let creation_lease = activation.admit_resource_creation()?;
    let hidden_authority = PhysicalAuthority::capture(request);
    let hidden_ops = native.clone();
    let hidden_output = output.clone();
    let hidden_monitor = monitor.clone();
    let late_hidden_ops = native.clone();
    let (hidden, creation_lease) = run_bounded_display_native_phase(
        "syndocal-output-authoring-hidden-shell",
        native.hidden_phase_timeout(),
        move |_| {
            if let Err(error) = hidden_authority.validate(&hidden_ops.app.state::<AppState>()) {
                creation_lease.retire();
                return Err(OutputLeaseCandidateCommitFailure::safe(error));
            }
            match hidden_ops.prepare_hidden_candidate(&hidden_output, &hidden_monitor) {
                Ok(hidden) => Ok((hidden, creation_lease)),
                Err(error) => {
                    if matches!(error, OutputLeaseCandidateCommitFailure::SafeAbort(_)) {
                        creation_lease.retire();
                    } else {
                        retain_failed_display_creation(
                            &hidden_ops.app.state::<AppState>(),
                            &video_output_window_label(hidden_output.id, false),
                            creation_lease,
                        );
                    }
                    Err(error)
                }
            }
        },
        move |(hidden, lease)| {
            let label = hidden.clone();
            if late_hidden_ops.cleanup_hidden_candidate(hidden).is_ok() {
                lease.retire();
            } else {
                retain_failed_display_creation(
                    &late_hidden_ops.app.state::<AppState>(),
                    &label,
                    lease,
                );
            }
        },
    )
    .map_err(OutputLeaseCandidateCommitFailure::into_message)?;
    let gpu_ops = native.clone();
    let gpu_output = output.clone();
    let gpu_monitor = monitor.clone();
    let gpu_fence = request.expected_fence.clone();
    let gpu_authority = PhysicalAuthority::capture(request);
    let late_gpu_ops = native.clone();
    let (prepared, creation_lease) = run_bounded_display_native_phase(
        "syndocal-output-authoring-display-first-frame",
        native.gpu_phase_timeout(),
        move |cancelled| {
            let app = &gpu_ops.app;
            let state = app.state::<AppState>();
            let result = (|| -> Result<PreparedNativeVideoOutputCandidate, String> {
                gpu_authority.validate(&state)?;
                let window = app
                    .windows()
                    .get(&hidden)
                    .cloned()
                    .ok_or_else(|| "Display authoring shell disappeared".to_string())?;
                let current_monitor =
                    authoritative_video_display_monitor_for_output(app, &gpu_output)?;
                if !exact_video_display_monitor_matches(&current_monitor, &gpu_monitor) {
                    return Err("Display monitor topology changed during authoring".to_string());
                }
                let metrics = Arc::new(Mutex::new(NativeVideoOutputMetrics::default()));
                let teardown_lease = NativeVideoOutputWorker::new_teardown_lease_slot();
                let admission_app = app.clone();
                let first_frame_authority = gpu_authority.clone();
                let worker = start_native_video_live_output(NativeVideoLiveOutputRequest {
                    window,
                    engine: state.engine.clone(),
                    output_id: gpu_output.id,
                    metrics: Arc::clone(&metrics),
                    #[cfg(feature = "ndi")]
                    ndi_inputs: Arc::clone(&state.ndi_inputs),
                    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
                    spout_inputs: Arc::clone(&state.spout_inputs),
                    capture_inputs: Arc::clone(&state.capture_inputs),
                    initial_size_override: Some((gpu_output.width, gpu_output.height)),
                    fullscreen: gpu_output.fullscreen,
                    unpublished_snapshot: None,
                    gate_render_loop_until_publication: true,
                    first_frame_fence: Some(gpu_fence),
                    first_frame_admission: Some(Box::new(move || {
                        first_frame_authority.validate(&admission_app.state::<AppState>())
                    })),
                    phase_cancelled: Some(cancelled),
                    show_before_first_frame: true,
                    label: hidden.clone(),
                    teardown_lease,
                })?;
                Ok(PreparedNativeVideoOutputCandidate {
                    label: hidden.clone(),
                    output_id: gpu_output.id,
                    metrics,
                    worker,
                })
            })();
            match result {
                Ok(prepared) => Ok((prepared, creation_lease)),
                Err(error) => {
                    let label = hidden.clone();
                    let cleanup = gpu_ops.cleanup_hidden_candidate(hidden);
                    if cleanup.is_ok() {
                        creation_lease.retire();
                    } else {
                        retain_failed_display_creation(&state, &label, creation_lease);
                    }
                    Err(OutputLeaseCandidateCommitFailure::safe(error)
                        .with_cleanup("Display authoring first-frame cleanup", cleanup))
                }
            }
        },
        move |(prepared, lease)| {
            let _ = cleanup_prepared_display_for_authoring(&late_gpu_ops, prepared, Some(lease));
        },
    )
    .map_err(OutputLeaseCandidateCommitFailure::into_message)?;
    let mut creation_lease = Some(creation_lease);
    if let Err((prepared, failure)) =
        native.publish_native_candidate_after_preflight(prepared, || {
            PhysicalAuthority::capture(request)
                .validate(state)
                .map_err(OutputLeaseCandidateCommitFailure::in_doubt)?;
            let lease = creation_lease.take().ok_or_else(|| {
                OutputLeaseCandidateCommitFailure::in_doubt("Display creation lease missing")
            })?;
            match lease.publish() {
                Ok(()) => Ok(()),
                Err(lease) => {
                    creation_lease = Some(lease);
                    Err(OutputLeaseCandidateCommitFailure::safe(
                        "Display authority changed before resource publication",
                    ))
                }
            }
        })
    {
        let cleanup = cleanup_prepared_display_for_authoring(&native, prepared, creation_lease);
        return Err(failure
            .with_cleanup("Display authoring publication cleanup", cleanup)
            .into_message());
    }
    Ok(())
}

fn synchronize_external(
    state: &AppState,
    snapshot: &protocol::VideoSnapshot,
    output_id: VideoOutputId,
    request: &ControlRequest<'_>,
) -> Result<(), String> {
    let authority = PhysicalAuthority::capture(request);
    authority.validate(state)?;
    let ownership = state.engine.output_ownership_status();
    let plans =
        video::build_external_video_io_route_plans(snapshot, &video::video_runtime_status());
    let mut transport = state
        .external_video_transport
        .lock()
        .map_err(|_| "External video transport runtime lock was poisoned".to_string())?;
    let mut events = state
        .external_video_transport_events
        .lock()
        .map_err(|_| "External video transport event log lock was poisoned".to_string())?;
    let mut capture = state
        .capture_transport
        .lock()
        .map_err(|_| "Capture transport state lock was poisoned".to_string())?;
    #[cfg(feature = "ndi")]
    let mut ndi = state
        .ndi_transport
        .lock()
        .map_err(|_| "NDI transport state lock was poisoned".to_string())?;
    #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
    let mut spout = state
        .spout_transport
        .lock()
        .map_err(|_| "Spout transport state lock was poisoned".to_string())?;
    #[cfg(all(feature = "syphon", target_os = "macos"))]
    let mut syphon = state
        .syphon_transport
        .lock()
        .map_err(|_| "Syphon transport state lock was poisoned".to_string())?;
    let physical_admission = || authority.validate(state);
    let mut driver = AppExternalVideoTransportDriver {
        authoring_admission: Some(VideoOutputAuthoringAdmission {
            output_id,
            validate: &physical_admission,
        }),
        recording: RecordingExternalVideoTransportDriver {
            events: &mut events,
        },
        output_ownership_role: ownership.effective_role,
        capture: &mut capture,
        #[cfg(feature = "ndi")]
        ndi: &mut ndi,
        #[cfg(all(feature = "spout", target_os = "windows", target_arch = "x86_64"))]
        spout: &mut spout,
        #[cfg(all(feature = "syphon", target_os = "macos"))]
        syphon: &mut syphon,
        #[cfg(any(
            feature = "ndi",
            all(feature = "spout", target_os = "windows", target_arch = "x86_64"),
            all(feature = "syphon", target_os = "macos")
        ))]
        engine: state.engine.clone(),
        #[cfg(any(
            feature = "ndi",
            all(feature = "spout", target_os = "windows", target_arch = "x86_64"),
            all(feature = "syphon", target_os = "macos")
        ))]
        activation: None,
    };
    let report = transport.sync_output_route_with_driver_and_role_with_start_admission(
        output_id,
        &plans,
        &mut driver,
        ownership.effective_role,
        |_driver| {
            authority.validate(state)?;
            #[cfg(any(
                feature = "ndi",
                all(feature = "spout", target_os = "windows", target_arch = "x86_64"),
                all(feature = "syphon", target_os = "macos")
            ))]
            if ownership.effective_role.video_allowed() {
                _driver.activation = Some(
                    state
                        .engine
                        .admit_output_activation(ownership.effective_role)?,
                );
            }
            Ok(())
        },
    );
    if external_video_transport_has_output_failure(&report) {
        return Err(format!(
            "Video output synchronization failed (stops: {}; starts: {})",
            report.stop_failed.len(),
            report.start_failed.len()
        ));
    }
    Ok(())
}

pub(super) fn apply(
    app: &tauri::AppHandle,
    window: &WebviewWindow,
    state: &AppState,
    action: &OutputControlActionV2,
    request: ControlRequest<'_>,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    apply_with_native(
        state,
        action,
        request,
        &TauriNativeOperations {
            app,
            editor_window: window,
        },
    )
}

struct OutputIdReservation<'a> {
    engine: &'a EngineHandle,
    id: Option<VideoOutputId>,
}

impl Drop for OutputIdReservation<'_> {
    fn drop(&mut self) {
        if let Some(id) = self.id {
            let _ = self.engine.release_video_output_id_if_last(id);
        }
    }
}

fn apply_with_native(
    state: &AppState,
    action: &OutputControlActionV2,
    request: ControlRequest<'_>,
    native: &impl NativeOperations,
) -> Result<
    (
        bool,
        OutputControlFenceV1,
        output_lease::OutputLeaseRequestReceipt,
    ),
    String,
> {
    if state.output_lease_now_ms()? < request.lease_now_ms {
        return Err(
            "Video output lease monotonic clock moved backwards before admission".to_string(),
        );
    }
    native.validate(action)?;
    let mut allocation = OutputIdReservation {
        engine: &state.engine,
        id: None,
    };
    let (
        submission,
        mutation,
        before_snapshot,
        candidate_snapshot,
        plan,
        prepared_lease,
        ancillary,
        mappings,
    ) = {
        let _lifecycle = state
            .standby_sync_lifecycle
            .lock()
            .map_err(|_| "Standby synchronization lifecycle lock was poisoned".to_string())?;
        let _owner = state
            .project_transaction_owner_rotation
            .lock()
            .map_err(|_| "Project transaction owner rotation lock was poisoned".to_string())?;
        let _admission = lock_project_external_command_admission(state)?;
        let mut coordinator = lock_project_coordinator(state)?;
        let _transition = lock_output_ownership_transition(state)?;
        let mut registry = state
            .output_lease_registry
            .lock()
            .map_err(|_| "Output lease registry lock was poisoned".to_string())?;
        let submission = state
            .engine
            .begin_persistence_mutation_submission()
            .map_err(|error| error.to_string())?;
        reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
        exact_authority(state, &coordinator, &request)?;
        let mut before_snapshot = submission.persistence_snapshot()?;
        use_authored_video_snapshot(&mut before_snapshot);
        let new_id = if matches!(
            action,
            OutputControlActionV2::AddDisplay { .. }
                | OutputControlActionV2::AddExternalOutput { .. }
        ) {
            let id = state.engine.allocate_video_output_id();
            allocation.id = Some(id);
            id
        } else {
            0
        };
        let mutation = mutation_from_snapshot(&before_snapshot, action, new_id)?;
        let candidate_snapshot = engine::video_output_authoring::candidate_snapshot(
            &before_snapshot,
            mutation.output_id,
            &mutation.expected,
            &mutation.replacement,
        )?;
        if mutation.expected == mutation.replacement {
            let ((applied, fence), receipt) = submit_output_lease_candidate_with_classified_commit(
                state,
                &mut registry,
                request.lease_request,
                state.output_lease_now_ms()?,
                "video output authoring",
                || Ok((false, request.expected_fence.clone())),
            )?;
            return Ok((applied, fence, receipt));
        }
        let before_project =
            project_file_for_save_from_parts(before_snapshot.clone(), &coordinator.ancillary);
        let before = ProjectCheckpoint {
            hash: project_checkpoint_hash(&before_project, &coordinator.mappings)?,
            project: before_project,
            mappings: coordinator.mappings.clone(),
            epoch: coordinator.epoch,
            revision: coordinator.revision,
        };
        let after = ProjectCheckpoint {
            project: project_file_for_save_from_parts(
                candidate_snapshot.clone(),
                &coordinator.ancillary,
            ),
            mappings: coordinator.mappings.clone(),
            epoch: coordinator.epoch,
            revision: coordinator.revision,
            hash: String::new(),
        };
        let plan = prepare_internal_media_asset_commit(
            &coordinator,
            mutation.history_label,
            "",
            before,
            after,
            current_unix_ms().min(u64::MAX as u128) as u64,
        )?;
        if plan.next_checkpoint_hash.is_none() {
            return Err("Video output candidate did not change the project".to_string());
        }
        let prepared = prepare_display_output_lease_candidate(
            state,
            &registry,
            request.lease_request,
            state.output_lease_now_ms()?,
        )?;
        if state
            .project_transaction_active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return Err(abort_prepared_display_output_lease_candidate(
                state,
                request.lease_request,
                OutputLeaseCandidateCommitFailure::safe("Another project transaction is active"),
            ));
        }
        (
            submission,
            mutation,
            before_snapshot,
            candidate_snapshot,
            plan,
            prepared,
            coordinator.ancillary.clone(),
            coordinator.mappings.clone(),
        )
    };

    // These callbacks may join workers, close HWNDs, initialize a GPU or wait
    // on Engine. No coordinator, owner, lease or transition mutex is retained.
    let retired = native
        .retire(state, &before_snapshot, &mutation, &request)
        .map_err(|error| fault_after_admission(state, error))?;
    let revalidate = || -> Result<(), String> {
        let _lifecycle = state
            .standby_sync_lifecycle
            .lock()
            .map_err(|_| "Standby synchronization lifecycle lock was poisoned".to_string())?;
        let _owner = state
            .project_transaction_owner_rotation
            .lock()
            .map_err(|_| "Project transaction owner rotation lock was poisoned".to_string())?;
        let _admission = lock_project_external_command_admission_for_display_finalize(state)?;
        let coordinator = lock_project_coordinator(state)?;
        let _transition = lock_output_ownership_transition(state)?;
        let registry = state
            .output_lease_registry
            .lock()
            .map_err(|_| "Output lease registry lock was poisoned".to_string())?;
        if !state.project_transaction_active.load(Ordering::Acquire)
            || *registry != prepared_lease.baseline
        {
            return Err("Video output authoring reservation or lease changed".to_string());
        }
        exact_authority(state, &coordinator, &request)?;
        validate_current_lease(state, &registry, request.lease_request)
    };
    revalidate().map_err(|error| fault_after_admission(state, error))?;
    // After admission to Engine an unknown ACK must never permit id reuse.
    allocation.id = None;
    submission
        .author_video_output_published(
            mutation.output_id,
            mutation.expected.clone(),
            mutation.replacement.clone(),
        )
        .map_err(|failure| {
            fault_after_admission(
                state,
                match failure {
                    SnapshotPublicationFailure::Definitive(message)
                    | SnapshotPublicationFailure::Indeterminate(message) => message,
                },
            )
        })?;
    revalidate().map_err(|error| fault_after_admission(state, error))?;
    native
        .validate(action)
        .map_err(|error| fault_after_admission(state, error))?;
    native
        .synchronize(state, &candidate_snapshot, &mutation, retired, &request)
        .map_err(|error| fault_after_admission(state, error))?;
    let actual_snapshot = submission
        .persistence_snapshot()
        .map_err(|error| fault_after_admission(state, error))?;
    let actual_project = project_file_for_save_from_parts(actual_snapshot, &ancillary);
    let actual_hash = project_checkpoint_hash(&actual_project, &mappings)
        .map_err(|error| fault_after_admission(state, error))?;
    if Some(&actual_hash) != plan.next_checkpoint_hash.as_ref() {
        return Err(fault_after_admission(
            state,
            "Published video output does not match the prepared project candidate".to_string(),
        ));
    }

    let final_commit = (|| -> Result<_, String> {
        let _lifecycle = state
            .standby_sync_lifecycle
            .lock()
            .map_err(|_| "Standby synchronization lifecycle lock was poisoned".to_string())?;
        let _owner = state
            .project_transaction_owner_rotation
            .lock()
            .map_err(|_| "Project transaction owner rotation lock was poisoned".to_string())?;
        let _admission = lock_project_external_command_admission_for_display_finalize(state)?;
        let mut coordinator = lock_project_coordinator(state)?;
        let _transition = lock_output_ownership_transition(state)?;
        let mut registry = state
            .output_lease_registry
            .lock()
            .map_err(|_| "Output lease registry lock was poisoned".to_string())?;
        exact_authority(state, &coordinator, &request)?;
        if !state.project_transaction_active.load(Ordering::Acquire)
            || *registry != prepared_lease.baseline
        {
            return Err(
                "Video output authoring reservation or lease changed before commit".to_string(),
            );
        }
        // Refresh wall-clock lease validity at the terminal boundary, not at
        // the time native preparation began.
        let mut candidate = prepared_lease.baseline.clone();
        let receipt = candidate
            .submit_request(request.lease_request, state.output_lease_now_ms()?)
            .map_err(|error| format!("Video output lease revalidation failed: {error:?}"))?;
        if let Err(error) = &receipt.outcome {
            return Err(format!(
                "Video output lease expired or changed before commit: {error:?}"
            ));
        }
        apply_internal_media_asset_transaction_after_preflight(&mut coordinator, plan);
        *registry = candidate;
        let fence = control_plane_runtime::committed_output_control_fence(
            request.expected_fence,
            control_plane_runtime::CommittedOutputControlFenceValues {
                project_epoch: coordinator.epoch,
                project_revision: coordinator.revision,
                project_checkpoint_hash: &coordinator.checkpoint_hash,
                project_publication_generation: coordinator.publication_generation,
                output_epoch: request.expected_fence.output_epoch,
                output_generation: request.expected_fence.output_generation,
                safety_blackout_epoch: request.expected_fence.safety_blackout_epoch,
                safety_blackout_generation: request.expected_fence.safety_blackout_generation,
            },
        );
        Ok((fence, receipt))
    })();
    let (fence, receipt) = final_commit.map_err(|error| fault_after_admission(state, error))?;
    record_published_display_output_terminal(
        &state.output_lease_durable_receipts,
        &state.project_transaction_active,
        &receipt,
    )
    .map_err(|error| fault_after_admission(state, error.into_message()))?;
    Ok((true, fence, receipt))
}

#[cfg(test)]
#[path = "video_output_authoring_tests.rs"]
mod tests;
