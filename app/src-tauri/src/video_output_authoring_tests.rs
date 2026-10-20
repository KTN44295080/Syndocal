use super::*;
use crate::tests::MediaAssetA6CommandHarness;
use protocol::control_plane_command::OutputLeaseAuthorityV1;
use std::cell::{Cell, RefCell};

struct Authority {
    query: ControlPlaneQueryState,
    owner: String,
    incarnation: u64,
    lease: OutputLeaseAuthorityV1,
}

fn authority(state: &Arc<AppState>, ttl_ms: u64) -> Authority {
    let owner = "task33-authoring-test-owner".to_string();
    register_project_transaction_owner_for_window_label(state, "main", owner.clone()).unwrap();
    let process = state
        .output_lease_registry
        .lock()
        .unwrap()
        .process_session_incarnation();
    let query = ControlPlaneQueryState::for_process_incarnation(process).unwrap();
    let fence = query
        .issue_output_control_fence_for_window("main", state)
        .unwrap();
    let incarnation = *state
        .project_transaction_owner_incarnations
        .lock()
        .unwrap()
        .get("main")
        .unwrap();
    let acquire = OutputLeaseRequest::from_action(
        &owner,
        "task33-authoring-test-acquire",
        1,
        OutputLeaseRequestAction::Acquire {
            owner: OutputLeaseOwner::new(&owner, "main", process, incarnation).unwrap(),
            resources: OutputLeaseResources::new(&[
                OutputLeaseResource::Lighting,
                OutputLeaseResource::Video,
            ])
            .unwrap(),
            project_identity: output_lease_project_identity(fence.project_epoch),
            ttl_ms,
        },
    )
    .unwrap();
    let receipt = state
        .output_lease_registry
        .lock()
        .unwrap()
        .submit_request(&acquire, state.output_lease_now_ms().unwrap())
        .unwrap();
    assert!(receipt.outcome.is_ok());
    Authority {
        query,
        owner,
        incarnation,
        lease: OutputLeaseAuthorityV1 {
            lease_id: receipt.lease_id.unwrap().encode(),
            generation: receipt.generation_after.unwrap(),
        },
    }
}

fn seed_display(state: &Arc<AppState>) -> VideoOutputSummary {
    let output = VideoOutputSummary {
        id: state.engine.allocate_video_output_id(),
        label: "Original Display".to_string(),
        kind: VideoOutputKind::Display,
        enabled: true,
        composition_id: 1,
        fullscreen: true,
        monitor_id: Some(0),
        monitor_identity: Some("a".repeat(64)),
        width: 1920,
        height: 1080,
        endpoint_name: None,
        opacity: 0.75,
        blackout: false,
        mapping: VideoOutputMapping::default(),
    };
    state
        .engine
        .add_video_output_published(output.clone())
        .unwrap();
    let _admission = lock_project_external_command_admission(state).unwrap();
    let mut coordinator = lock_project_coordinator(state).unwrap();
    reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).unwrap();
    output
}

fn configuration(output: &VideoOutputSummary) -> VideoOutputConfigurationV2 {
    VideoOutputConfigurationV2 {
        kind: output.kind.clone(),
        label: output.label.clone(),
        width: output.width,
        height: output.height,
        fullscreen: output.fullscreen,
        monitor_index: output.monitor_id,
        monitor_identity: output.monitor_identity.clone(),
        endpoint_name: output.endpoint_name.clone(),
    }
}

fn request(
    state: &Arc<AppState>,
    authority: &Authority,
    action: OutputControlActionV2,
    id: u64,
) -> (OutputControlCommandRequestV2, OutputLeaseRequest, u64) {
    let expected_fence = authority
        .query
        .issue_output_control_fence_for_window("main", state)
        .unwrap();
    let operation_id = match &action {
        OutputControlActionV2::AddDisplay { .. } => "syndocal.output.display.add.v2",
        OutputControlActionV2::AddExternalOutput { .. } => "syndocal.output.external.add.v2",
        OutputControlActionV2::ConfigureVideoOutput { .. } => "syndocal.output.video.configure.v2",
        OutputControlActionV2::RemoveVideoOutput { .. } => "syndocal.output.video.remove.v2",
        _ => panic!("authoring test requires a canonical authoring action"),
    }
    .to_string();
    let request = OutputControlCommandRequestV2 {
        operation_id,
        request_id: id,
        expected_fence,
        action,
    };
    request.validate().unwrap();
    let (lease, now) = build_output_lease_authorization_request(
        state,
        &authority.owner,
        "main",
        authority.incarnation,
        id,
        &request.action,
        request.expected_fence.project_epoch,
    )
    .unwrap();
    (request, lease, now)
}

fn apply_fake(
    state: &Arc<AppState>,
    authority: &Authority,
    request: &OutputControlCommandRequestV2,
    lease_request: &OutputLeaseRequest,
    now: u64,
    native: &FakeNative,
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
        &request.action,
        ControlRequest {
            expected_fence: &request.expected_fence,
            lease_request,
            lease_now_ms: now,
            expected_owner_principal: &authority.owner,
            expected_owner_window_label: "main",
            expected_owner_incarnation: authority.incarnation,
        },
        native,
    )
}

#[derive(Default)]
struct FakeNative {
    retire_calls: Cell<usize>,
    sync_calls: Cell<usize>,
    starts: Cell<usize>,
    retire_delay: Duration,
    startup_delay: Duration,
    fail_sync: bool,
    change_revision_on_retire: bool,
    observed_live: RefCell<Option<VideoOutputSummary>>,
}

fn assert_authority_mutexes_released(state: &AppState) {
    assert!(state.standby_sync_lifecycle.try_lock().is_ok());
    assert!(state.project_transaction_owner_rotation.try_lock().is_ok());
    assert!(state
        .project_external_command_admission
        .gate
        .try_lock()
        .is_ok());
    assert!(state.project_coordinator.try_lock().is_ok());
    assert!(state.output_ownership_transition.try_lock().is_ok());
    assert!(state.output_lease_registry.try_lock().is_ok());
}

impl NativeOperations for FakeNative {
    fn validate(&self, _action: &OutputControlActionV2) -> Result<(), String> {
        Ok(())
    }
    fn retire(
        &self,
        state: &AppState,
        _before: &EngineSnapshot,
        mutation: &Mutation,
        _request: &ControlRequest<'_>,
    ) -> Result<NativeRetirement, String> {
        assert_authority_mutexes_released(state);
        self.retire_calls.set(self.retire_calls.get() + 1);
        if !self.retire_delay.is_zero() {
            std::thread::sleep(self.retire_delay);
        }
        if self.change_revision_on_retire {
            state.project_coordinator.lock().unwrap().revision += 1;
        }
        Ok(NativeRetirement {
            reopen_display: mutation.expected.is_some(),
        })
    }
    fn synchronize(
        &self,
        state: &AppState,
        _candidate: &EngineSnapshot,
        mutation: &Mutation,
        _retired: NativeRetirement,
        request: &ControlRequest<'_>,
    ) -> Result<(), String> {
        assert_authority_mutexes_released(state);
        self.sync_calls.set(self.sync_calls.get() + 1);
        if self.fail_sync {
            return Err("injected native synchronization failure".to_string());
        }
        if !self.startup_delay.is_zero() {
            std::thread::sleep(self.startup_delay);
        }
        // Models the authority gate at the physical SDK publication / first
        // Display present, after a potentially slow native preparation phase.
        PhysicalAuthority::capture(request).validate(state)?;
        let live = state
            .engine
            .video_snapshot()
            .outputs
            .into_iter()
            .find(|output| output.id == mutation.output_id);
        if live.as_ref().is_some_and(|output| output.enabled)
            && state
                .engine
                .output_ownership_status()
                .effective_role
                .video_allowed()
        {
            self.starts.set(self.starts.get() + 1);
        }
        *self.observed_live.borrow_mut() = live;
        Ok(())
    }
}

fn assert_pending_fault(state: &AppState) {
    assert!(state.project_transaction_active.load(Ordering::Acquire));
    assert!(state
        .project_external_command_admission
        .project_transaction_publication_faulted
        .load(Ordering::Acquire));
    assert_eq!(
        state
            .output_lease_durable_receipts
            .lock()
            .unwrap()
            .state
            .pending
            .len(),
        1
    );
    assert!(state
        .project_coordinator
        .lock()
        .unwrap()
        .history
        .undo
        .is_empty());
}

#[test]
fn configure_live_output_publishes_one_project_step_and_preserves_global_authority() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let original = seed_display(state);
    let authority = authority(state, 60_000);
    let mut spec = configuration(&original);
    spec.label = "  プロジェクター  ".to_string();
    spec.width = 2560;
    spec.height = 1440;
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::ConfigureVideoOutput {
            output_id: original.id,
            spec,
            lease: authority.lease.clone(),
        },
        101,
    );
    let role_before = state.engine.output_ownership_status();
    let native = FakeNative::default();
    let (applied, after, receipt) =
        apply_fake(state, &authority, &request, &lease, now, &native).unwrap();
    assert!(applied);
    assert_eq!(after.project_epoch, request.expected_fence.project_epoch);
    assert_eq!(
        after.project_revision,
        request.expected_fence.project_revision + 1
    );
    assert_eq!(
        after.project_publication_generation,
        request.expected_fence.project_publication_generation + 1
    );
    assert_ne!(
        after.project_checkpoint_hash,
        request.expected_fence.project_checkpoint_hash
    );
    assert_eq!(after.output_epoch, request.expected_fence.output_epoch);
    assert_eq!(
        after.output_generation,
        request.expected_fence.output_generation
    );
    assert_eq!(
        after.safety_blackout_epoch,
        request.expected_fence.safety_blackout_epoch
    );
    assert_eq!(
        after.safety_blackout_generation,
        request.expected_fence.safety_blackout_generation
    );
    let actual = native.observed_live.borrow().clone().unwrap();
    assert_eq!(actual.label, "プロジェクター");
    assert_eq!((actual.width, actual.height), (2560, 1440));
    assert_eq!(actual.enabled, original.enabled);
    assert_eq!(actual.composition_id, original.composition_id);
    assert_eq!(actual.opacity, original.opacity);
    assert_eq!(actual.mapping, original.mapping);
    assert_eq!((native.retire_calls.get(), native.sync_calls.get()), (1, 1));
    assert_eq!(
        state.engine.output_ownership_status().effective_role,
        role_before.effective_role
    );
    assert_eq!(
        state.project_coordinator.lock().unwrap().history.undo.len(),
        1
    );
    assert_eq!(
        state
            .output_lease_durable_receipts
            .lock()
            .unwrap()
            .lookup(&lease)
            .unwrap(),
        Some(receipt)
    );
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
}

#[test]
fn unchanged_configuration_is_noop_without_native_retirement_or_history() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 60_000);
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::ConfigureVideoOutput {
            output_id: output.id,
            spec: configuration(&output),
            lease: authority.lease.clone(),
        },
        102,
    );
    let native = FakeNative::default();
    let (applied, after, _) =
        apply_fake(state, &authority, &request, &lease, now, &native).unwrap();
    assert!(!applied);
    assert_eq!(after, request.expected_fence);
    assert_eq!((native.retire_calls.get(), native.sync_calls.get()), (0, 0));
    assert!(state
        .project_coordinator
        .lock()
        .unwrap()
        .history
        .undo
        .is_empty());
}

#[test]
fn stale_project_epoch_or_revision_never_retires_or_publishes() {
    for change_epoch in [true, false] {
        for operation in 0..3 {
            let harness = MediaAssetA6CommandHarness::new();
            let state = &harness.state;
            let output = seed_display(state);
            let authority = authority(state, 60_000);
            let action = match operation {
                0 => OutputControlActionV2::AddExternalOutput {
                    spec: ExternalVideoOutputSpecV2 {
                        kind: VideoOutputKind::NdiSender,
                        label: "Stale sender".to_string(),
                        width: 1280,
                        height: 720,
                        endpoint_name: "Task33Stale".to_string(),
                        composition_id: 1,
                    },
                    lease: authority.lease.clone(),
                },
                1 => OutputControlActionV2::ConfigureVideoOutput {
                    output_id: output.id,
                    spec: VideoOutputConfigurationV2 {
                        label: "Stale configure".to_string(),
                        ..configuration(&output)
                    },
                    lease: authority.lease.clone(),
                },
                _ => OutputControlActionV2::RemoveVideoOutput {
                    output_id: output.id,
                    lease: authority.lease.clone(),
                },
            };
            let (mut request, lease, now) = request(state, &authority, action, 103);
            if change_epoch {
                request.expected_fence.project_epoch += 1;
            } else {
                request.expected_fence.project_revision += 1;
            }
            let native = FakeNative::default();
            assert!(apply_fake(state, &authority, &request, &lease, now, &native).is_err());
            assert_eq!((native.retire_calls.get(), native.sync_calls.get()), (0, 0));
            assert_eq!(state.engine.video_snapshot().outputs, vec![output]);
            assert!(state
                .output_lease_durable_receipts
                .lock()
                .unwrap()
                .state
                .pending
                .is_empty());
            assert!(!state.project_transaction_active.load(Ordering::Acquire));
        }
    }
}

#[test]
fn authority_change_after_retirement_keeps_pending_and_never_publishes_candidate() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 60_000);
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::RemoveVideoOutput {
            output_id: output.id,
            lease: authority.lease.clone(),
        },
        104,
    );
    let native = FakeNative {
        change_revision_on_retire: true,
        ..FakeNative::default()
    };
    let error = apply_fake(state, &authority, &request, &lease, now, &native).unwrap_err();
    assert!(error.starts_with("Video output authoring is in doubt;"));
    assert_eq!((native.retire_calls.get(), native.sync_calls.get()), (1, 0));
    assert_eq!(state.engine.video_snapshot().outputs, vec![output]);
    assert_pending_fault(state);
}

#[test]
fn lease_expiry_during_retirement_prevents_engine_mutation_and_native_restart() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 1_000);
    let mut spec = configuration(&output);
    spec.label = "After expiry".to_string();
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::ConfigureVideoOutput {
            output_id: output.id,
            spec,
            lease: authority.lease.clone(),
        },
        105,
    );
    let native = FakeNative {
        retire_delay: Duration::from_millis(1_050),
        ..FakeNative::default()
    };
    let error = apply_fake(state, &authority, &request, &lease, now, &native).unwrap_err();
    assert!(error.contains("expired"), "{error}");
    assert_eq!(
        (
            native.retire_calls.get(),
            native.sync_calls.get(),
            native.starts.get()
        ),
        (1, 0, 0)
    );
    assert_eq!(state.engine.video_snapshot().outputs, vec![output]);
    assert_pending_fault(state);
}

#[test]
fn lease_expiry_during_native_startup_blocks_physical_publication_and_retains_pending_b() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 1_000);
    let mut spec = configuration(&output);
    spec.label = "Prepared B".to_string();
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::ConfigureVideoOutput {
            output_id: output.id,
            spec,
            lease: authority.lease.clone(),
        },
        106,
    );
    let native = FakeNative {
        startup_delay: Duration::from_millis(1_050),
        ..FakeNative::default()
    };
    let error = apply_fake(state, &authority, &request, &lease, now, &native).unwrap_err();
    assert!(error.contains("expired"), "{error}");
    assert_eq!(
        (
            native.retire_calls.get(),
            native.sync_calls.get(),
            native.starts.get()
        ),
        (1, 1, 0)
    );
    assert_eq!(state.engine.video_snapshot().outputs[0].label, "Prepared B");
    assert_pending_fault(state);
}

#[test]
fn native_failure_after_ack_never_claims_rollback_or_a_success_receipt() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 60_000);
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::RemoveVideoOutput {
            output_id: output.id,
            lease: authority.lease.clone(),
        },
        107,
    );
    let native = FakeNative {
        fail_sync: true,
        ..FakeNative::default()
    };
    let error = apply_fake(state, &authority, &request, &lease, now, &native).unwrap_err();
    assert!(error.starts_with("Video output authoring is in doubt;"));
    assert!(
        state.engine.video_snapshot().outputs.is_empty(),
        "ACKed B must not be mislabeled a rollback"
    );
    assert_pending_fault(state);
    assert!(state
        .output_lease_durable_receipts
        .lock()
        .unwrap()
        .lookup(&lease)
        .is_err());
    let journal_bytes =
        serde_json::to_vec(&state.output_lease_durable_receipts.lock().unwrap().state).unwrap();
    let mut reopened_journal = OutputLeaseDurableReceiptJournal {
        path: None,
        state: serde_json::from_slice(&journal_bytes).unwrap(),
    };
    assert_eq!(reopened_journal.state.pending.len(), 1);
    assert!(
        reopened_journal.lookup(&lease).is_err(),
        "serialized Pending must still block replay after restart"
    );
    assert!(
        reopened_journal.prepare(&lease).is_err(),
        "restart cannot turn an unresolved physical B into a fresh operation"
    );
}

#[test]
fn unavailable_external_backend_rejects_before_reservation_or_native_effects() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let authority = authority(state, 60_000);
    let kind = [
        VideoOutputKind::NdiSender,
        VideoOutputKind::SpoutSender,
        VideoOutputKind::SyphonServer,
    ]
    .into_iter()
    .find(|kind| ensure_video_output_backend_available(kind).is_err())
    .expect("at least one platform-specific SDK is unavailable on every host");
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::AddExternalOutput {
            spec: ExternalVideoOutputSpecV2 {
                kind,
                label: "Unavailable".to_string(),
                width: 1280,
                height: 720,
                endpoint_name: "Task33Unavailable".to_string(),
                composition_id: 1,
            },
            lease: authority.lease.clone(),
        },
        108,
    );
    let native = FakeNative::default();
    assert!(apply_fake(state, &authority, &request, &lease, now, &native).is_err());
    assert_eq!((native.retire_calls.get(), native.sync_calls.get()), (0, 0));
    assert!(state.engine.video_snapshot().outputs.is_empty());
    assert!(state
        .output_lease_durable_receipts
        .lock()
        .unwrap()
        .state
        .pending
        .is_empty());
    assert!(!state.project_transaction_active.load(Ordering::Acquire));
}

#[test]
fn endpoint_duplicates_are_case_insensitive_and_exclude_only_the_edited_output() {
    let mut output = VideoOutputSummary {
        id: 8,
        label: "Other sender".to_string(),
        kind: VideoOutputKind::NdiSender,
        enabled: true,
        composition_id: 1,
        fullscreen: false,
        monitor_id: None,
        monitor_identity: None,
        width: 1280,
        height: 720,
        endpoint_name: Some(" Shared Sender ".to_string()),
        opacity: 1.0,
        blackout: false,
        mapping: VideoOutputMapping::default(),
    };
    assert!(endpoint_conflicts(
        &[output.clone()],
        &VideoOutputKind::NdiSender,
        "shared sender",
        None
    ));
    assert!(!endpoint_conflicts(
        &[output.clone()],
        &VideoOutputKind::NdiSender,
        "shared sender",
        Some(8)
    ));
    output.kind = VideoOutputKind::SpoutSender;
    assert!(!endpoint_conflicts(
        &[output],
        &VideoOutputKind::NdiSender,
        "shared sender",
        None
    ));
}

#[test]
fn standby_display_creation_is_enabled_persisted_and_never_arms_or_starts_a_resource() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    state
        .engine
        .begin_output_ownership_transition(MachineOutputRole::Standby)
        .unwrap()
        .complete()
        .unwrap();
    let authority = authority(state, 60_000);
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::AddDisplay {
            spec: DisplayOutputSpecV2 {
                label: "スタンバイ画面".to_string(),
                monitor_identity: "a".repeat(64),
                monitor_index: 0,
                width: 1920,
                height: 1080,
                fullscreen: true,
            },
            lease: authority.lease.clone(),
        },
        110,
    );
    let native = FakeNative::default();
    let (applied, after, _) =
        apply_fake(state, &authority, &request, &lease, now, &native).unwrap();
    assert!(applied);
    let outputs = state.engine.video_snapshot().outputs;
    assert_eq!(outputs.len(), 1);
    assert!(outputs[0].enabled);
    assert_eq!(outputs[0].kind, VideoOutputKind::Display);
    assert_eq!(
        state.engine.output_ownership_status().effective_role,
        MachineOutputRole::Standby
    );
    assert_eq!(native.starts.get(), 0);
    assert_eq!(after.output_epoch, request.expected_fence.output_epoch);
    assert_eq!(
        after.output_generation,
        request.expected_fence.output_generation
    );
    assert_eq!(
        after.project_revision,
        request.expected_fence.project_revision + 1
    );
    assert_eq!(
        state.project_coordinator.lock().unwrap().history.undo.len(),
        1
    );
}

#[test]
fn configure_preserves_cue_disabled_live_output_and_does_not_start_from_authored_enabled() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    state
        .engine
        .send(EngineCommand::CreateCue {
            authored_beats: None,
            cue_id: 9,
            label: "Live output".to_string(),
            fade_ms: 0,
            targets: Vec::new(),
            video_targets: Vec::new(),
            video_output_targets: vec![VideoOutputTarget {
                output_id: output.id,
                enabled: false,
                opacity: 0.25,
                blackout: true,
            }],
            node_graph_targets: Vec::new(),
            effect_targets: Vec::new(),
        })
        .unwrap();
    state.engine.send(EngineCommand::TriggerCue(9)).unwrap();
    // Wait until the Cue override reaches the renderer-observable image;
    // persistence replies alone do not acknowledge that separate publication.
    state.engine.await_snapshot_publication().unwrap();
    let persisted = project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert!(persisted.video.outputs[0].enabled);
    assert!(!state.engine.video_snapshot().outputs[0].enabled);
    {
        let _external = lock_project_external_command_admission(state).unwrap();
        let mut coordinator = lock_project_coordinator(state).unwrap();
        reconcile_project_checkpoint_for_coordinator(state, &mut coordinator).unwrap();
    }
    let authority = authority(state, 60_000);
    let mut spec = configuration(&output);
    spec.label = "Renamed while Cue owns levels".to_string();
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::ConfigureVideoOutput {
            output_id: output.id,
            spec,
            lease: authority.lease.clone(),
        },
        111,
    );
    let native = FakeNative::default();
    apply_fake(state, &authority, &request, &lease, now, &native).unwrap();
    let observed = native.observed_live.borrow().clone().unwrap();
    assert!(!observed.enabled);
    assert_eq!(observed.opacity, 0.25);
    assert!(observed.blackout);
    assert_eq!(native.starts.get(), 0);
    let persisted = project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert!(persisted.video.outputs[0].enabled);
    assert_eq!(persisted.video.outputs[0].opacity, output.opacity);
    assert!(!persisted.video.outputs[0].blackout);
}

#[cfg(any(
    feature = "ndi",
    all(feature = "spout", target_os = "windows", target_arch = "x86_64"),
    all(feature = "syphon", target_os = "macos")
))]
#[test]
fn external_add_and_kind_conversion_are_enabled_and_follow_current_global_role() {
    let kind = [
        VideoOutputKind::SpoutSender,
        VideoOutputKind::NdiSender,
        VideoOutputKind::SyphonServer,
    ]
    .into_iter()
    .find(|kind| ensure_video_output_backend_available(kind).is_ok())
    .expect("feature-enabled native test build must provide one available external backend");
    for role in [MachineOutputRole::Standby, MachineOutputRole::Both] {
        let harness = MediaAssetA6CommandHarness::new();
        let state = &harness.state;
        state
            .engine
            .begin_output_ownership_transition(role)
            .unwrap()
            .complete()
            .unwrap();
        let authority = authority(state, 60_000);
        let action = OutputControlActionV2::AddExternalOutput {
            spec: ExternalVideoOutputSpecV2 {
                kind: kind.clone(),
                label: "  外部出力  ".to_string(),
                width: 1920,
                height: 1080,
                endpoint_name: "  Task33 Sender  ".to_string(),
                composition_id: 1,
            },
            lease: authority.lease.clone(),
        };
        let (command, lease, now) = request(state, &authority, action, 112);
        let native = FakeNative::default();
        let (applied, fence, _) =
            apply_fake(state, &authority, &command, &lease, now, &native).unwrap();
        assert!(applied);
        let output = state.engine.video_snapshot().outputs[0].clone();
        assert!(output.enabled);
        assert_eq!(output.label, "外部出力");
        assert_eq!(output.endpoint_name.as_deref(), Some("Task33 Sender"));
        assert_eq!(output.kind, kind);
        assert_eq!(native.starts.get(), usize::from(role.video_allowed()));
        assert_eq!(state.engine.output_ownership_status().effective_role, role);
        assert_eq!(fence.output_epoch, command.expected_fence.output_epoch);
        assert_eq!(
            fence.output_generation,
            command.expected_fence.output_generation
        );
        assert_eq!(
            fence.project_revision,
            command.expected_fence.project_revision + 1
        );
        let spec = VideoOutputConfigurationV2 {
            kind: VideoOutputKind::Display,
            label: "Converted display".to_string(),
            width: 1920,
            height: 1080,
            fullscreen: true,
            monitor_index: Some(0),
            monitor_identity: Some("a".repeat(64)),
            endpoint_name: None,
        };
        let (command, lease, now) = request(
            state,
            &authority,
            OutputControlActionV2::ConfigureVideoOutput {
                output_id: output.id,
                spec,
                lease: authority.lease.clone(),
            },
            113,
        );
        let converted = FakeNative::default();
        apply_fake(state, &authority, &command, &lease, now, &converted).unwrap();
        let actual = state.engine.video_snapshot().outputs[0].clone();
        assert_eq!(actual.kind, VideoOutputKind::Display);
        assert!(actual.enabled);
        assert_eq!(actual.id, output.id);
        assert_eq!(converted.retire_calls.get(), 1);
        assert_eq!(converted.starts.get(), usize::from(role.video_allowed()));
        assert_eq!(state.engine.output_ownership_status().effective_role, role);
        assert_eq!(
            state.project_coordinator.lock().unwrap().history.undo.len(),
            2
        );
    }
}

struct HistoryPlatform;
impl ProjectReplacementPlatform for HistoryPlatform {
    fn advance_recovery_authority(
        &self,
        _state: &AppState,
        coordinator: &mut ProjectCoordinator,
        transition: ProjectRecoveryAuthorityTransition,
    ) -> Result<u64, String> {
        advance_project_recovery_authority_serial_with_persist(coordinator, transition, |_, _| {
            Ok(())
        })
    }
    fn fence_and_retire_outputs(&self, _state: &AppState) -> Result<(), String> {
        Ok(())
    }
    fn emit_authority_event(
        &self,
        _effect: ProjectReplacementCoordinatorEffect,
        _result: &ProjectLoadResult,
    ) {
    }
}

fn navigate(state: &AppState, undo: bool) {
    let _external = lock_project_external_command_admission(state).unwrap();
    let mut coordinator = lock_project_coordinator(state).unwrap();
    let status = project_history_status_for_coordinator(&coordinator);
    navigate_project_history_with_coordinator(
        state,
        &mut coordinator,
        undo,
        Some(status.project_epoch),
        if undo {
            status.undo_entry_id
        } else {
            status.redo_entry_id
        },
        Some(status.checkpoint_hash),
        |state, prepared, coordinator| {
            replace_prepared_project_snapshot_with_coordinator_and_platform(
                state,
                prepared,
                coordinator,
                ProjectReplacementCoordinatorPlan {
                    coordinator_effect: ProjectReplacementCoordinatorEffect::RevisionMutation,
                    emit_authority_event: false,
                    validate_before_publication: no_project_replacement_validation,
                    capture_after_commit: no_project_replacement_capture,
                    lease_authorization: None,
                },
                &HistoryPlatform,
            )
            .map(|(result, (), _)| result)
        },
    )
    .unwrap();
}

#[test]
fn remove_is_one_undo_step_and_project_json_reopens_with_exact_output_configuration() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    let output = seed_display(state);
    let authority = authority(state, 60_000);
    let (request, lease, now) = request(
        state,
        &authority,
        OutputControlActionV2::RemoveVideoOutput {
            output_id: output.id,
            lease: authority.lease.clone(),
        },
        109,
    );
    let native = FakeNative::default();
    apply_fake(state, &authority, &request, &lease, now, &native).unwrap();
    assert!(state.engine.video_snapshot().outputs.is_empty());
    assert_eq!(native.retire_calls.get(), 1);
    assert_eq!(native.starts.get(), 0);
    navigate(state, true);
    assert_eq!(
        state.engine.output_ownership_status().effective_role,
        MachineOutputRole::Standby
    );
    assert!(!state.engine.output_ownership_status().video_allowed);
    let restored = project_snapshot_for_save(state.engine.persistence_snapshot().unwrap());
    assert_eq!(restored.video.outputs, vec![output.clone()]);
    let coordinator = lock_project_coordinator(state).unwrap();
    let project = project_file_for_save_from_parts(restored, &coordinator.ancillary);
    let bytes = serde_json::to_vec(&project).unwrap();
    let reopened: ProjectFile = serde_json::from_slice(&bytes).unwrap();
    let prepared = prepare_project_load(
        reopened,
        coordinator.mappings.clone(),
        "Task33 reopen".to_string(),
        None,
    )
    .unwrap();
    assert_eq!(
        project_file_from_prepared_load(&prepared)
            .snapshot
            .video
            .outputs,
        vec![output.clone()]
    );
    let recovery = prepare_project_recovery_load(
        serde_json::to_value(&project).unwrap(),
        "Task33 recovery".to_string(),
        None,
        0,
        "task33-recovery".to_string(),
    )
    .unwrap();
    assert_eq!(
        recovery.authority_disposition,
        ProjectAuthorityDisposition::RecoveryPendingAck
    );
    assert_eq!(
        project_file_from_prepared_load(&recovery)
            .snapshot
            .video
            .outputs,
        vec![output]
    );
    drop(coordinator);
    navigate(state, false);
    assert!(
        project_snapshot_for_save(state.engine.persistence_snapshot().unwrap())
            .video
            .outputs
            .is_empty()
    );
}

#[test]
fn retained_display_creation_blocks_global_transition_until_exact_retirement_ack() {
    let harness = MediaAssetA6CommandHarness::new();
    let state = &harness.state;
    state
        .engine
        .begin_output_ownership_transition(MachineOutputRole::Both)
        .unwrap()
        .complete()
        .unwrap();
    let activation = state
        .engine
        .admit_output_activation(MachineOutputRole::Both)
        .unwrap();
    let first = activation.admit_resource_creation().unwrap();
    let second = activation.admit_resource_creation().unwrap();
    let label = video_output_window_label(42, false);
    retain_failed_display_creation(state, &label, first);
    retain_failed_display_creation(state, &label, second);
    {
        let workers = state.native_video_output_workers.lock().unwrap();
        let retained = workers.get(&label).unwrap();
        assert!(
            retained.join.is_none(),
            "a shell-only record owns no presenter thread"
        );
        assert_eq!(
            retained.retained_creation_leases.len(),
            2,
            "same-label ownership is accumulated, never overwritten"
        );
    }
    let (tx, rx) = std::sync::mpsc::channel();
    let engine = state.engine.clone();
    let transition_worker = std::thread::spawn(move || {
        tx.send(
            engine
                .begin_output_ownership_transition(MachineOutputRole::Standby)
                .and_then(|transition| transition.complete()),
        )
        .unwrap();
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    while state.engine.output_ownership_status().state
        != protocol::OutputOwnershipState::Transitioning
    {
        assert!(
            Instant::now() < deadline,
            "global transition must enter its counted-resource wait"
        );
        std::thread::yield_now();
    }
    assert!(matches!(
        rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    let mut retained = take_native_video_output_worker(&state.native_video_output_workers, &label)
        .unwrap()
        .unwrap();
    assert_eq!(retained.retained_creation_leases.len(), 2);
    // Models the existing retirement helper's exact joined + HWND-ACK branch;
    // merely taking/removing the quarantine entry did not release ownership.
    assert!(matches!(
        rx.try_recv(),
        Err(std::sync::mpsc::TryRecvError::Empty)
    ));
    retained.release_teardown_lease_after_retirement_ack();
    assert!(retained.retained_creation_leases.is_empty());
    let status = rx.recv_timeout(Duration::from_secs(2)).unwrap().unwrap();
    assert_eq!(status.effective_role, MachineOutputRole::Standby);
    transition_worker.join().unwrap();
}
