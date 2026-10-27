//! I5 real C3 publication into one output-session history ledger.
//! Plans are captured before submission; every seal consumes actual G evidence.
use super::*;
use crate::isf_native_gpu::{CpuMaterializationRequest, NativeGpuMaterializer};
use crate::native_gpu_budget::ChargeKind;
use crate::native_gpu_contract::LayerResultIdentity;
use crate::native_gpu_history::{HistoryMaterializationRequest, HistoryScope};
use crate::VideoEffectStageFault;
use std::sync::Weak;

#[test]
fn native_gpu_output_real_p_fault_survives_late_cancel_and_revocation() {
    struct Revoked;
    impl NativeOutputAuthority for Revoked {
        fn revalidate(
            &self,
            _: &NativeOutputRequest,
            _: OutputBoundary,
        ) -> Result<(), PresentRejection> {
            Err(PresentRejection::Revoked)
        }
    }
    for cancelled in [false, true] {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let mut cache = SharedNativeOutputCache::new(101);
        let request = output_request(&fixture, &mut cache, 1);
        let key = request.key.clone();
        let mut case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
        let mut session = session_for(&case.identities);
        let plan = session.plan(&case.identities, request.pts_ms).unwrap();
        // G really submits its ISF batch. P then rejects the mismatched selected
        // identity; a subsequent live control change must retain that P fault.
        case.composition.layers_bottom_first[0]
            .identity
            .content_sequence += 1;
        let pending = start_history(
            &mut fixture,
            &compositor,
            request,
            case.inputs,
            &case.composition,
            &session,
            plan,
        );
        let mut retained = history_retiring(pending.poll_with_history(
            &mut fixture.owner,
            &mut fixture.runtime,
            &mut cache,
            &mut session,
            &Revoked,
            &|| cancelled,
        ));
        assert!(
            matches!(retained.reason(), Some(OutputRejection::Fault(fault))
            if fault.phase == GpuPhase::CompositionSnapshot
                && fault.kind == NativeGpuFaultKind::IdentityMismatch)
        );
        assert!(cache.last_valid(&key).is_none());
        assert!(fixture.owner.budget_snapshot().allocated_bytes > 0);
        drop(session);
        drop(fixture.runtime);
        drop(compositor);
        cache.blackout().unwrap();
        reap_retirement(&mut retained, &mut fixture.owner);
    }
}

#[test]
fn native_gpu_output_recovered_history_resumes_healthy_owner_but_keeps_p_fault_and_stop() {
    for mode in 0..3 {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let mut cache = SharedNativeOutputCache::new(101);
        let request = output_request(&fixture, &mut cache, 1);
        let key = request.key.clone();
        let mut jobs = batch(1);
        Arc::make_mut(&mut jobs[1].chain.stages[1].shader).wgsl = "not valid WGSL".into();
        let mut case = prepare_history_case(&fixture, &mut reference, &request, jobs);
        let mut session = session_for(&case.identities);
        let plan = session.plan(&case.identities, request.pts_ms).unwrap();
        if mode == 1 {
            // Real P identity rejection while G separately performs its exact
            // fixed-clock recovery. P failure must stay the primary fault.
            case.composition.layers_bottom_first[0]
                .identity
                .content_sequence += 1;
        }
        let pending = start_history(
            &mut fixture,
            &compositor,
            request,
            case.inputs,
            &case.composition,
            &session,
            plan,
        );
        let stopped = Cell::new(false);
        let authority = Authority {
            stopped: &stopped,
            cancel_after_commit: false,
        };
        let retirement = history_retiring(finish_history(
            pending,
            &mut fixture,
            &mut cache,
            &mut session,
            &authority,
            &|| false,
        ));
        assert_eq!(fixture.runtime.native_frame_index(), 3);
        assert!(cache.last_valid(&key).is_none());
        if mode == 2 {
            fixture.owner.begin_stop();
        }
        if mode == 0 {
            let mut history = retirement
                .into_recovered_history(&mut fixture.owner)
                .unwrap_or_else(|(_, fault)| panic!("healthy recovered owner rejected: {fault:?}"));
            assert!(history.is_recovered());
            assert_eq!(history.faults(), case.faults);
            session
                .commit_recovered_with(&mut history, |faults| {
                    assert_eq!(faults, case.faults);
                    Ok(())
                })
                .unwrap();
            assert!(history.is_committed() && !history.output_phases_valid());
            assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
            assert_eq!(fixture.owner.initialize(|_, _| 17u8).unwrap(), 17);
            drop(history);
            drop(session);
            drop(fixture.runtime);
            drop(compositor);
            cache.blackout().unwrap();
            fixture.owner.begin_stop();
            let started = Instant::now();
            while fixture.owner.poll_retirement().unwrap() != RetirementProgress::Reaped {
                assert!(started.elapsed() < Duration::from_secs(5));
                std::thread::yield_now();
            }
            assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
        } else {
            if mode == 1 {
                assert!(
                    matches!(retirement.reason(), Some(OutputRejection::Fault(fault))
                    if fault.phase == GpuPhase::CompositionSnapshot)
                );
            }
            let (mut retained, _) = retirement
                .into_recovered_history(&mut fixture.owner)
                .err()
                .expect("P failure/Stop must return complete recovery ownership");
            let history = retained
                .take_history_outcome()
                .expect("rejected recovery returns all CPU outcomes");
            assert!(history.is_recovered());
            assert_eq!(history.faults(), case.faults);
            drop(history);
            drop(session);
            drop(fixture.runtime);
            drop(compositor);
            cache.blackout().unwrap();
            reap_retirement(&mut retained, &mut fixture.owner);
        }
    }
}

struct PreparedHistoryCase {
    inputs: PreparedNativeInputs,
    composition: NativeCompositionPlan,
    identities: StackBatch<LayerResultIdentity>,
    cpu: Vec<Arc<VideoFrame>>,
    faults: Vec<VideoEffectStageFault>,
    input_owners: Vec<Weak<VideoFrame>>,
    input_capacities: Vec<u64>,
}

fn prepare_history_case(
    fixture: &Fixture,
    reference: &mut IsfGpuRuntime,
    request: &NativeOutputRequest,
    jobs: StackBatch<PreparedNativeLayer>,
) -> PreparedHistoryCase {
    let input_owners = jobs.iter().map(|job| Arc::downgrade(&job.input)).collect();
    let input_capacities = jobs
        .iter()
        .map(|job| job.input.data.capacity() as u64)
        .collect();
    let mut faults = Vec::new();
    let cpu = jobs
        .iter()
        .map(|job| {
            let stages = job
                .chain
                .stages
                .iter()
                .map(|stage| IsfStageInputs {
                    shader: &stage.shader,
                    controls: &stage.controls,
                    images: &[],
                })
                .collect::<Vec<_>>();
            let (pixels, failures) = reference.apply_stack_stage_local_with_inputs_and_keys(
                &job.input,
                &stages,
                job.input.pts_ms as f32 / 1000.0,
                job.input.duration_ms as f32 / 1000.0,
                job.date,
                &job.chain.keys,
            );
            faults.extend(job.chain.clone().finish_faults(failures));
            Arc::new(pixels)
        })
        .collect();
    let inputs = fixture
        .runtime
        .prepare_native_inputs(request.frame, jobs, NativeBatchRestrictions::default())
        .unwrap();
    let identities = inputs.prospective_identities(&fixture.runtime).unwrap();
    let layer_plans = plans();
    let selected = layer_plans
        .iter()
        .zip(identities.iter())
        .map(|(plan, identity)| NativeCompositionLayer {
            plan,
            selected_entry: identity.entry,
            identity: *identity,
            kind: SelectedTextureKind::InThisTransaction,
        })
        .collect::<Vec<_>>();
    let limits = &fixture.owner.capabilities().limits;
    let composition = NativeCompositionPlan::preflight(
        request.context,
        request.frame,
        SIZE,
        [0; 4],
        &selected,
        limits.max_texture_dimension_2d,
        u64::from(limits.max_storage_buffer_binding_size),
    )
    .unwrap();
    PreparedHistoryCase {
        inputs,
        composition,
        identities,
        cpu,
        faults,
        input_owners,
        input_capacities,
    }
}

fn scope(identity: &LayerResultIdentity) -> HistoryScope {
    HistoryScope {
        frame_scope: identity.frame,
        input: identity.entry.input,
        layer_id: identity.layer_id,
        queue_epoch: identity.entry.queue_epoch,
        source_generation: identity.source_generation,
        extent: (identity.width, identity.height),
    }
}

fn session_for(identities: &[LayerResultIdentity]) -> NativeOutputHistorySession {
    NativeOutputHistorySession::new(
        identities[0].context,
        identities.iter().map(scope).collect(),
        SIZE,
    )
    .unwrap()
}

fn start_history(
    fixture: &mut Fixture,
    compositor: &NativeGpuCompositor,
    request: NativeOutputRequest,
    inputs: PreparedNativeInputs,
    composition: &NativeCompositionPlan,
    session: &NativeOutputHistorySession,
    history_plan: NativeOutputHistoryPlan,
) -> PendingSharedNativeOutput {
    let before = fixture.runtime.native_frame_index();
    let (admitted, reservation) = admit(fixture, inputs, composition, &[]);
    let pending = PendingSharedNativeOutput::submit_with_history(
        &mut fixture.owner,
        &mut fixture.runtime,
        compositor,
        request,
        admitted,
        reservation,
        composition,
        &[],
        session,
        history_plan,
    )
    .unwrap_or_else(|(failure, _)| panic!("history submit failed: {:?}", failure.reason()));
    assert_eq!(fixture.runtime.native_frame_index(), before.wrapping_add(3));
    pending
}

fn finish_history(
    mut pending: PendingSharedNativeOutput,
    fixture: &mut Fixture,
    cache: &mut SharedNativeOutputCache,
    session: &mut NativeOutputHistorySession,
    authority: &impl NativeOutputAuthority,
    cancel: &dyn VideoRenderCancellation,
) -> SharedNativeOutputProgress {
    let started = Instant::now();
    loop {
        match pending.poll_with_history(
            &mut fixture.owner,
            &mut fixture.runtime,
            cache,
            session,
            authority,
            cancel,
        ) {
            SharedNativeOutputProgress::Pending(next) => {
                pending = next;
                assert!(started.elapsed() < Duration::from_secs(5));
                std::thread::yield_now();
            }
            result => return result,
        }
    }
}

fn history_ready(result: SharedNativeOutputProgress) -> (SharedReady, NativeOutputHistoryFrame) {
    match result {
        SharedNativeOutputProgress::HistoryReady { ready, history, .. } => (ready, history),
        SharedNativeOutputProgress::Retiring(retirement) => {
            panic!("history did not become Ready: {:?}", retirement.reason())
        }
        _ => panic!("history-bound completion did not return its sealed history owner"),
    }
}

fn history_retiring(result: SharedNativeOutputProgress) -> SharedNativeOutputRetirement {
    match result {
        SharedNativeOutputProgress::Retiring(retirement) => retirement,
        _ => panic!("failed or recovered history became Ready or lost its affine owner"),
    }
}

fn assert_histories_empty(session: &NativeOutputHistorySession, count: usize) {
    for index in 0..count {
        let history = session.history(index).unwrap();
        assert!(history.metadata().is_empty());
        assert_eq!(history.resident_count(), 0);
    }
}

fn assert_entries(
    session: &NativeOutputHistorySession,
    offset: usize,
    identities: &[LayerResultIdentity],
) {
    for (index, identity) in identities.iter().enumerate() {
        let metadata = session.history(offset + index).unwrap().metadata();
        assert_eq!(metadata.len(), 1);
        assert_eq!(metadata[0].id, identity.entry);
        assert_eq!(metadata[0].pts_ms, identity.pts_ms);
        assert_eq!(metadata[0].duration_ms, identity.duration_ms);
    }
}

#[test]
fn i5_history_real_c3_rejects_foreign_session_before_submit_and_poll_then_commits_exact_pixels() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
    assert!(case.faults.is_empty());
    let mut session = session_for(&case.identities);
    let mut foreign = session_for(&case.identities);
    let host = session.host_budget().clone();
    let plan = session.plan(&case.identities, request.pts_ms).unwrap();
    assert!(!foreign.accepts_plan(&plan));
    let (admitted, reservation) = admit(&mut fixture, case.inputs, &case.composition, &[]);
    let before = fixture.owner.budget_snapshot();
    let (mut rejected, plan) = match PendingSharedNativeOutput::submit_with_history(
        &mut fixture.owner,
        &mut fixture.runtime,
        &compositor,
        request.clone(),
        admitted,
        reservation,
        &case.composition,
        &[],
        &foreign,
        plan,
    ) {
        Err(rejected) => rejected,
        Ok(_) => panic!("foreign session encoded work before rejecting the plan"),
    };
    assert!(
        matches!(rejected.reason(), Some(OutputRejection::Fault(error))
        if error.kind == NativeGpuFaultKind::IdentityMismatch)
    );
    assert_eq!(fixture.runtime.native_frame_index(), 0);
    assert_eq!(fixture.owner.budget_snapshot(), before);
    assert_histories_empty(&foreign, 3);
    let admitted = rejected.admitted.take().unwrap();
    let reservation = rejected.unsubmitted.take().unwrap();
    drop(rejected);
    let pending = PendingSharedNativeOutput::submit_with_history(
        &mut fixture.owner,
        &mut fixture.runtime,
        &compositor,
        request.clone(),
        admitted,
        reservation,
        &case.composition,
        &[],
        &session,
        plan,
    )
    .unwrap_or_else(|(failure, _)| panic!("resumed submit failed: {:?}", failure.reason()));
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    let stopped = Cell::new(false);
    let authority = PresentAuthority {
        stopped: &stopped,
        mapping: request.mapping_identity,
        rejection: None,
        boundaries: RefCell::new(Vec::new()),
    };
    let submitted = fixture.owner.budget_snapshot();
    let transaction = pending.transaction_id;
    let pending = match pending.poll_with_history(
        &mut fixture.owner,
        &mut fixture.runtime,
        &mut cache,
        &mut foreign,
        &authority,
        &|| false,
    ) {
        SharedNativeOutputProgress::Rejected { pending, reason } => {
            assert!(matches!(reason, OutputRejection::Fault(error)
                if error.kind == NativeGpuFaultKind::IdentityMismatch));
            pending
        }
        _ => panic!("foreign session consumed the real pending completion"),
    };
    assert_eq!(pending.transaction_id, transaction);
    assert!(pending.isf.is_some() && pending.snapshot.is_some());
    assert!(authority.boundaries.borrow().is_empty());
    assert_eq!(fixture.owner.budget_snapshot(), submitted);
    assert_histories_empty(&foreign, 3);
    assert_eq!(foreign.host_budget().snapshot().charges, 0);
    let (ready, mut history) = history_ready(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    assert!(history.is_committed() && history.output_phases_valid());
    assert!(!history.is_recovered());
    assert_eq!(history.error(), None);
    assert_eq!(history.receipt().unwrap().phase().frame(), request.frame);
    assert!(history.receipt().unwrap().phase().validation().is_ok());
    assert!(history.take_retired().is_empty());
    assert_entries(&session, 0, &case.identities);
    assert_cpu(&cache, &key, &case.cpu);
    for (index, identity) in case.identities.iter().enumerate() {
        let pin = session.history(index).unwrap().pin(identity.entry).unwrap();
        assert_eq!(pin.resource().identity(), identity);
        let pixels = NativeGpuMaterializer::new(&mut fixture.runtime)
            .materialize_cpu(
                CpuMaterializationRequest::ValidatedGpu {
                    entry: identity.entry,
                    lease: pin.resource(),
                },
                &mut fixture.owner,
                &|| false,
            )
            .unwrap();
        assert_eq!(pixels.frame(), case.cpu[index].as_ref());
    }
    let charged = host.snapshot();
    assert_eq!(
        charged.unique_bytes,
        case.input_capacities.iter().sum::<u64>()
    );
    assert_eq!(charged.charges, 3);
    assert_eq!(charged.managed_alias_bytes, 0);
    drop(ready);
    drop(history);
    drop(session);
    drop(foreign);
    assert_eq!(host.snapshot().unique_bytes, 0);
    cache.blackout().unwrap();
    drop(fixture.runtime);
    reap(fixture.owner);
}

#[test]
fn i5_history_composition_selection_failure_commits_valid_isf_with_false_output_phase() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let mut case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
    let mut session = session_for(&case.identities);
    let host = session.host_budget().clone();
    let plan = session.plan(&case.identities, request.pts_ms).unwrap();
    case.composition.layers_bottom_first[0]
        .identity
        .content_sequence += 100;
    let pending = start_history(
        &mut fixture,
        &compositor,
        request,
        case.inputs,
        &case.composition,
        &session,
        plan,
    );
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retirement = history_retiring(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    assert!(
        matches!(retirement.reason(), Some(OutputRejection::Fault(error))
        if error.phase == GpuPhase::CompositionSnapshot
        && error.kind == NativeGpuFaultKind::IdentityMismatch)
    );
    assert!(retirement.take_isf_outcome().is_none());
    let history = retirement.take_history_outcome().unwrap();
    assert!(history.is_committed());
    assert!(!history.output_phases_valid());
    assert_eq!(history.error(), None);
    assert!(history.receipt().unwrap().phase().validation().is_ok());
    assert_entries(&session, 0, &case.identities);
    assert_eq!(host.snapshot().charges, 3);
    assert!(cache.last_valid(&key).is_none());
    drop(history);
    drop(session);
    assert_eq!(host.snapshot().charges, 0);
    drop(fixture.runtime);
    reap_retirement(&mut retirement, &mut fixture.owner);
}

#[test]
fn i5_history_snapshot_identity_failure_keeps_committed_history_and_real_retirement() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
    let mut session = session_for(&case.identities);
    let host = session.host_budget().clone();
    let plan = session.plan(&case.identities, request.pts_ms).unwrap();
    let pending = start(
        &mut fixture,
        &compositor,
        request,
        case.inputs,
        &case.composition,
        &[],
    );
    let mut pending = pending
        .bind_history(plan)
        .unwrap_or_else(|(error, _, _)| panic!("binding failed: {error:?}"));
    let actual_pts = pending
        .snapshot
        .as_ref()
        .unwrap()
        .descriptor()
        .snapshot
        .pts_ms;
    // Only the caller's expected binding changes; G/P's actual evidence stays intact.
    pending.request.pts_ms = actual_pts + 1;
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retirement = history_retiring(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    assert!(
        matches!(retirement.reason(), Some(OutputRejection::Fault(error))
        if error.phase == GpuPhase::CompositionSnapshot
        && error.kind == NativeGpuFaultKind::IdentityMismatch)
    );
    let history = retirement.take_history_outcome().unwrap();
    assert!(history.is_committed() && history.output_phases_valid());
    assert_entries(&session, 0, &case.identities);
    assert!(retirement.snapshot.is_some());
    assert!(cache.last_valid(&key).is_none() && cache.snapshot_owner.is_none());
    assert!(fixture.owner.budget_snapshot().managed_bytes() > 0);
    drop(history);
    drop(session);
    assert_eq!(host.snapshot().charges, 0);
    drop(fixture.runtime);
    reap_retirement(&mut retirement, &mut fixture.owner);
}

#[test]
fn i5_history_shared_host_ledger_counts_input_aliases_once_across_distinct_input_ids() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let first_jobs = batch(1);
    let mut second_jobs = batch(2);
    for (first, second) in first_jobs.iter().zip(second_jobs.iter_mut()) {
        second.input = Arc::clone(&first.input);
        second.entry.input.input_id = protocol::VideoRenderInputId(2000 + second.input.layer_id);
        for key in &mut second.chain.keys {
            key.input = Some(second.entry.input);
        }
        for stage in &mut second.chain.stages {
            stage.fault_context.render_input_key = Some(second.entry.input);
        }
    }
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let first = prepare_history_case(&fixture, &mut reference, &request, first_jobs);
    let mut scopes = first.identities.iter().map(scope).collect::<Vec<_>>();
    for (identity, job) in first.identities.iter().zip(second_jobs.iter()) {
        scopes.push(HistoryScope {
            input: job.entry.input,
            ..scope(identity)
        });
    }
    let mut session = NativeOutputHistorySession::new(request.context, scopes, SIZE).unwrap();
    let host = session.host_budget().clone();
    let plan = session.plan(&first.identities, request.pts_ms).unwrap();
    let pending = start_history(
        &mut fixture,
        &compositor,
        request,
        first.inputs,
        &first.composition,
        &session,
        plan,
    );
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let (ready, history) = history_ready(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    assert_cpu(&cache, &key, &first.cpu);
    assert!(history.is_committed());
    drop(ready);
    drop(history);
    let bytes = first.input_capacities.iter().sum::<u64>();
    assert_eq!(host.snapshot().unique_bytes, bytes);
    assert_eq!(host.snapshot().charges, 3);
    let request = output_request(&fixture, &mut cache, 2);
    let second = prepare_history_case(&fixture, &mut reference, &request, second_jobs);
    for (a, b) in first.identities.iter().zip(second.identities.iter()) {
        assert_eq!(a.layer_id, b.layer_id);
        assert_ne!(a.entry.input, b.entry.input);
    }
    let plan = session.plan(&second.identities, request.pts_ms).unwrap();
    let pending = start_history(
        &mut fixture,
        &compositor,
        request,
        second.inputs,
        &second.composition,
        &session,
        plan,
    );
    let (ready, history) = history_ready(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    assert!(history.is_committed() && history.output_phases_valid());
    assert_cpu(&cache, &key, &second.cpu);
    assert_entries(&session, 0, &first.identities);
    assert_entries(&session, 3, &second.identities);
    let charged = host.snapshot();
    assert_eq!(charged.unique_bytes, bytes);
    assert_eq!(charged.allocations, 3);
    assert_eq!(charged.charges, 6);
    assert_eq!(charged.managed_alias_bytes, 0);
    assert_eq!(
        charged.limit,
        u64::from(SIZE.0) * u64::from(SIZE.1) * 4 * 6 * 9
    );
    drop(ready);
    drop(history);
    drop(session);
    assert_eq!(host.snapshot().unique_bytes, 0);
    assert_eq!(host.snapshot().charges, 0);
    cache.blackout().unwrap();
    drop(fixture.runtime);
    reap(fixture.owner);
}

#[test]
fn i5_history_partial_host_budget_refusal_retains_sealed_and_raw_real_owners() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
    let mut session = session_for(&case.identities);
    let host = session.host_budget().clone();
    let room = case.input_capacities[0];
    let reserved = host.snapshot().limit - room;
    let pressure = host.reserve(reserved).unwrap();
    let plan = session.plan(&case.identities, request.pts_ms).unwrap();
    let pending = start_history(
        &mut fixture,
        &compositor,
        request,
        case.inputs,
        &case.composition,
        &session,
        plan,
    );
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retirement = history_retiring(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    let history = retirement.take_history_outcome().unwrap();
    assert_eq!(history.error(), Some(HistoryError::HostBudgetExceeded));
    assert!(!history.is_committed() && !history.output_phases_valid());
    assert!(history.receipt().unwrap().phase().validation().is_ok());
    assert_histories_empty(&session, 3);
    assert_eq!(host.snapshot().unique_bytes, room);
    assert_eq!(
        host.snapshot().charges,
        1,
        "the first seal succeeds; later real owners stay raw"
    );
    assert_eq!(host.snapshot().reserved_bytes, reserved);
    assert!(case
        .input_owners
        .iter()
        .all(|owner| owner.upgrade().is_some()));
    assert!(cache.last_valid(&key).is_none());
    drop(session);
    assert_eq!(
        host.snapshot().charges,
        1,
        "the failed aggregate owns the sealed entry"
    );
    drop(history);
    assert!(case
        .input_owners
        .iter()
        .all(|owner| owner.upgrade().is_none()));
    assert_eq!(host.snapshot().unique_bytes, 0);
    drop(pressure);
    assert_eq!(host.snapshot().reserved_bytes, 0);
    drop(fixture.runtime);
    reap_retirement(&mut retirement, &mut fixture.owner);
}

#[test]
fn i5_history_stale_prospective_input_plan_reports_partial_commit_and_retains_later_owners() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let first = prepare_history_case(&fixture, &mut reference, &request, batch(1));
    let mut session = session_for(&first.identities);
    let host = session.host_budget().clone();
    let side_plan = session
        .plan(&first.identities[1..2], request.pts_ms)
        .unwrap();
    let pending = start(
        &mut fixture,
        &compositor,
        request,
        first.inputs,
        &first.composition,
        &[],
    );
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let (ready, layers, receipt) = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Ready { ready, layers, isf } => (ready, layers, isf),
        _ => panic!("the first actual C3 did not validate"),
    };
    let previous_pixels = cache.last_valid(&key).unwrap().data.clone();
    drop(ready);
    let second_layer = layers.into_iter().nth(1).unwrap();
    let request = output_request(&fixture, &mut cache, 2);
    let second = prepare_history_case(&fixture, &mut reference, &request, batch(2));
    let stale = session.plan(&second.identities, request.pts_ms).unwrap();
    // The future plan is already frozen. Publish only input 2 from the earlier
    // real C3 using its own pre-submit plan and actual ISF receipt.
    let mut side = session.seal_validated(side_plan, StackBatch::One([second_layer]), receipt);
    session.resolve_composition(&mut side, true).unwrap();
    assert!(side.is_committed());
    assert!(session.history(0).unwrap().metadata().is_empty());
    assert_eq!(
        session.history(1).unwrap().metadata()[0].id,
        first.identities[1].entry
    );
    assert!(session.history(2).unwrap().metadata().is_empty());
    let pending = start_history(
        &mut fixture,
        &compositor,
        request,
        second.inputs,
        &second.composition,
        &session,
        stale,
    );
    let mut retirement = history_retiring(finish_history(
        pending,
        &mut fixture,
        &mut cache,
        &mut session,
        &authority,
        &|| false,
    ));
    let history = retirement.take_history_outcome().unwrap();
    assert_eq!(history.error(), Some(HistoryError::StateMismatch));
    assert!(!history.is_committed() && !history.output_phases_valid());
    assert_eq!(
        session.history(0).unwrap().metadata()[0].id,
        second.identities[0].entry
    );
    assert_eq!(
        session.history(1).unwrap().metadata()[0].id,
        first.identities[1].entry
    );
    assert!(session.history(2).unwrap().metadata().is_empty());
    assert_eq!(host.snapshot().charges, 4);
    assert!(second
        .input_owners
        .iter()
        .all(|owner| owner.upgrade().is_some()));
    assert_eq!(cache.last_valid(&key).unwrap().data, previous_pixels);
    assert_eq!(fixture.runtime.native_frame_index(), 6);
    drop(history);
    assert_eq!(host.snapshot().charges, 2);
    assert!(second.input_owners[0].upgrade().is_some());
    assert!(second.input_owners[1].upgrade().is_none());
    assert!(second.input_owners[2].upgrade().is_none());
    drop(side);
    drop(session);
    assert_eq!(host.snapshot().charges, 0);
    cache.blackout().unwrap();
    drop(fixture.runtime);
    reap_retirement(&mut retirement, &mut fixture.owner);
}

#[test]
fn i5_history_real_recovered_cpu_requires_fault_policy_and_keeps_faults_on_budget_refusal() {
    for exhausted in [false, true] {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let mut cache = SharedNativeOutputCache::new(101);
        let request = output_request(&fixture, &mut cache, 1);
        let key = request.key.clone();
        let mut jobs = batch(1);
        Arc::make_mut(&mut jobs[1].chain.stages[1].shader).wgsl = "not valid WGSL".into();
        let case = prepare_history_case(&fixture, &mut reference, &request, jobs);
        assert_eq!(case.faults.len(), 1);
        assert_eq!(case.faults[0].stage_index, Some(1));
        assert_eq!(
            case.faults[0].stage_id,
            Some(protocol::VideoEffectStageId(21))
        );
        assert_eq!(case.faults[0].effect_id, Some(protocol::VideoEffectId(201)));
        let mut session = session_for(&case.identities);
        let host = session.host_budget().clone();
        let pressure = if exhausted {
            Some(host.reserve(host.snapshot().limit).unwrap())
        } else {
            None
        };
        let plan = session.plan(&case.identities, request.pts_ms).unwrap();
        let pending = start_history(
            &mut fixture,
            &compositor,
            request,
            case.inputs,
            &case.composition,
            &session,
            plan,
        );
        let stopped = Cell::new(false);
        let authority = Authority {
            stopped: &stopped,
            cancel_after_commit: false,
        };
        let mut retirement = history_retiring(finish_history(
            pending,
            &mut fixture,
            &mut cache,
            &mut session,
            &authority,
            &|| false,
        ));
        let mut history = retirement.take_history_outcome().unwrap();
        assert!(history.is_recovered());
        assert!(history.receipt().is_none());
        assert!(!history.is_committed() && !history.output_phases_valid());
        assert_eq!(history.faults(), case.faults.as_slice());
        assert_histories_empty(&session, 3);
        assert!(cache.last_valid(&key).is_none());
        assert_eq!(
            fixture.runtime.native_frame_index(),
            3,
            "recovery does not consume new clocks"
        );
        let calls = Cell::new(0);
        if exhausted {
            assert_eq!(history.error(), Some(HistoryError::HostBudgetExceeded));
            assert_eq!(host.snapshot().charges, 0);
            assert_eq!(
                session.commit_recovered_with(&mut history, |_| {
                    calls.set(calls.get() + 1);
                    Ok(())
                }),
                Err(HistoryError::HostBudgetExceeded)
            );
            assert_eq!(calls.get(), 0);
            assert_eq!(history.faults(), case.faults.as_slice());
        } else {
            assert_eq!(retirement.reason(), None);
            assert_eq!(history.error(), None);
            let charged = host.snapshot();
            assert_eq!(charged.charges, 3);
            assert_eq!(charged.unique_bytes, charged.managed_alias_bytes);
            assert_eq!(charged.unmanaged_bytes(), 0);
            assert_eq!(
                session.commit_recovered_with(&mut history, |faults| {
                    assert_eq!(faults, case.faults.as_slice());
                    calls.set(calls.get() + 1);
                    Err(HistoryError::Cancelled)
                }),
                Err(HistoryError::Cancelled)
            );
            assert_eq!(calls.get(), 1);
            assert_histories_empty(&session, 3);
            assert!(!history.is_committed());
            assert_eq!(
                host.snapshot(),
                charged,
                "refused fault policy keeps every CPU owner"
            );
            session
                .commit_recovered_with(&mut history, |faults| {
                    assert_eq!(faults, case.faults.as_slice());
                    calls.set(calls.get() + 1);
                    Ok(())
                })
                .unwrap();
            assert_eq!(calls.get(), 2);
            assert!(history.is_committed());
            assert!(!history.output_phases_valid());
            assert_entries(&session, 0, &case.identities);
            for (index, identity) in case.identities.iter().enumerate() {
                let input_history = session.history(index).unwrap();
                assert_eq!(input_history.resident_count(), 0);
                let selection_plan = input_history.plan(&[], identity.pts_ms).unwrap();
                let selected = input_history
                    .select_materialization_requests(&selection_plan)
                    .unwrap();
                match selected.selected.unwrap() {
                    HistoryMaterializationRequest::UploadCpuPostFx { entry, frame } => {
                        assert_eq!(entry, identity.entry);
                        assert_eq!(frame, case.cpu[index].as_ref());
                    }
                    _ => panic!("recovered CPU history became a resident or replay recipe"),
                }
            }
        }
        assert!(
            fixture
                .owner
                .budget_snapshot()
                .for_kind(ChargeKind::CpuSnapshot)
                .active_bytes
                > 0
        );
        assert!(!retirement.poll(&mut fixture.owner).unwrap());
        assert_eq!(fixture.owner.state(), GpuOwnerState::Retiring);
        drop(history);
        drop(session);
        assert_eq!(host.snapshot().charges, 0);
        drop(pressure);
        assert_eq!(host.snapshot().reserved_bytes, 0);
        drop(fixture.runtime);
        reap_retirement(&mut retirement, &mut fixture.owner);
    }
}

#[test]
fn i5_history_phases_resolved_stop_or_revocation_retains_seals_without_committing_metadata() {
    for stop in [false, true] {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let mut cache = SharedNativeOutputCache::new(101);
        let request = output_request(&fixture, &mut cache, 1);
        let key = request.key.clone();
        let case = prepare_history_case(&fixture, &mut reference, &request, batch(1));
        let mut session = session_for(&case.identities);
        let host = session.host_budget().clone();
        let plan = session.plan(&case.identities, request.pts_ms).unwrap();
        let stopped = Cell::new(false);
        let authority = PresentAuthority {
            stopped: &stopped,
            mapping: request.mapping_identity,
            rejection: Some((OutputBoundary::PhasesResolved, stop)),
            boundaries: RefCell::new(Vec::new()),
        };
        let pending = start_history(
            &mut fixture,
            &compositor,
            request,
            case.inputs,
            &case.composition,
            &session,
            plan,
        );
        let mut retirement = history_retiring(finish_history(
            pending,
            &mut fixture,
            &mut cache,
            &mut session,
            &authority,
            &|| stopped.get(),
        ));
        assert_eq!(
            retirement.reason(),
            Some(&if stop {
                OutputRejection::Cancelled
            } else {
                OutputRejection::Authority(PresentRejection::Revoked)
            })
        );
        assert!(authority
            .boundaries
            .borrow()
            .contains(&OutputBoundary::PhasesResolved));
        let history = retirement.take_history_outcome().unwrap();
        assert_eq!(history.error(), None);
        assert!(!history.is_committed() && !history.output_phases_valid());
        assert!(history.receipt().unwrap().phase().validation().is_ok());
        assert_histories_empty(&session, 3);
        assert_eq!(
            host.snapshot().charges,
            3,
            "sealing keeps owners without publishing history"
        );
        assert!(case
            .input_owners
            .iter()
            .all(|owner| owner.upgrade().is_some()));
        assert!(cache.last_valid(&key).is_none());
        assert!(!retirement.poll(&mut fixture.owner).unwrap());
        drop(session);
        assert_eq!(host.snapshot().charges, 3);
        drop(history);
        assert_eq!(host.snapshot().charges, 0);
        assert!(case
            .input_owners
            .iter()
            .all(|owner| owner.upgrade().is_none()));
        drop(fixture.runtime);
        reap_retirement(&mut retirement, &mut fixture.owner);
    }
}
