//! I3 real G/P/I join. No map/success receipts are fabricated.
use super::*;
use crate::isf_native_gpu::tests::{gpu, jobs, request as frame_id, shader, Fixture, IDENTITY};
use crate::isf_native_gpu::{
    NativeBatchAdmission, NativeBatchRestrictions, PreparedNativeInputs, PreparedNativeLayer,
};
use crate::isf_runtime::IsfStageInputs;
use crate::native_gpu_composition::tests::{layer, predicted_identities};
use crate::native_gpu_composition::{NativeCompositionLayer, SelectedTextureKind};
use crate::CompositionPlan;
use protocol::{Transform2D, VideoBlendMode, VideoOutputKind};
use std::cell::Cell;
use std::time::{Duration, Instant};

const INCREMENT: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){vec4 c=IMG_THIS_PIXEL(inputImage);gl_FragColor=vec4(c.r+1.0/255.0,c.gba);}"#;
const CLOCK_GREEN: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){vec4 c=IMG_THIS_PIXEL(inputImage);gl_FragColor=vec4(c.r,float(FRAMEINDEX)/255.0,c.ba);}"#;
const SIZE: (u32, u32) = (41, 29);

struct Authority<'a> {
    stopped: &'a Cell<bool>,
    cancel_after_commit: bool,
}
impl NativeOutputAuthority for Authority<'_> {
    fn revalidate(
        &self,
        _: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        if self.cancel_after_commit && boundary == OutputBoundary::AfterCacheCommit {
            self.stopped.set(true);
        }
        Ok(())
    }
}

fn output_request(
    fixture: &Fixture,
    cache: &mut SharedNativeOutputCache,
    sequence: u64,
) -> NativeOutputRequest {
    let frame = frame_id(sequence);
    let key = VideoOutputLastValidKey {
        project_render_epoch: frame.project_epoch,
        output_id: frame.output_id,
        width: SIZE.0,
        height: SIZE.1,
        composition_id: 1,
        kind: VideoOutputKind::Display,
        fullscreen: false,
        monitor_id: Some(1),
        endpoint_name: None,
    };
    let fence = cache.begin_request(key.clone(), frame).unwrap();
    let mapping = VideoOutputMapping::default();
    NativeOutputRequest {
        context: fixture.owner.context().id(),
        frame,
        key,
        fence,
        pts_ms: 1000,
        duration_ms: 33,
        mapping_identity: VideoOutputMappingIdentity::from_mapping(&mapping),
        mapping,
    }
}

fn plans() -> Vec<crate::CompositionLayerPlan> {
    vec![
        layer(1, VideoBlendMode::Normal, 1.0, Transform2D::default()),
        layer(
            2,
            VideoBlendMode::Add,
            1.0,
            Transform2D {
                x: 0.07,
                y: -0.035,
                scale_x: 0.83,
                scale_y: 0.6,
                crop_left: 0.13,
                crop_bottom: 0.25,
                ..Default::default()
            },
        ),
        layer(
            3,
            VideoBlendMode::Difference,
            1.0,
            Transform2D {
                crop_left: 0.2,
                crop_right: 0.1,
                ..Default::default()
            },
        ),
    ]
}

fn batch(sequence: u64) -> StackBatch<PreparedNativeLayer> {
    let mut batch = jobs(
        SIZE.0,
        SIZE.1,
        &[shader(INCREMENT), shader(CLOCK_GREEN), shader(IDENTITY)],
    );
    for job in batch.iter_mut() {
        job.entry.sequence = sequence * 10 + job.input.layer_id;
        job.content_sequence = sequence;
        let input = Arc::get_mut(&mut job.input).unwrap();
        for (index, pixel) in input.data.chunks_exact_mut(4).enumerate() {
            pixel.copy_from_slice(&[
                ((index * 13 + sequence as usize * 17 + input.layer_id as usize * 21) % 230) as u8,
                31,
                ((index * 3 + sequence as usize * 7) % 251) as u8,
                255,
            ]);
        }
    }
    batch
}

/// Existing standalone CPU-result ISF executor followed by the existing CPU compositor.
fn canonical_cpu_layers(
    reference: &mut IsfGpuRuntime,
    batch: &[PreparedNativeLayer],
) -> Vec<Arc<VideoFrame>> {
    batch
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
            let (frame, failures) = reference.apply_stack_stage_local_with_inputs_and_keys(
                &job.input,
                &stages,
                job.input.pts_ms as f32 / 1000.0,
                job.input.duration_ms as f32 / 1000.0,
                job.date,
                &job.chain.keys,
            );
            assert!(
                failures.is_empty(),
                "canonical CPU-result ISF oracle faulted"
            );
            Arc::new(frame)
        })
        .collect()
}

fn prepare(
    fixture: &Fixture,
    reference: &mut IsfGpuRuntime,
    request: &NativeOutputRequest,
    historical: Option<(&GpuLayerLease, Arc<VideoFrame>)>,
) -> (
    PreparedNativeInputs,
    NativeCompositionPlan,
    Vec<Arc<VideoFrame>>,
) {
    let batch = batch(request.frame.request_sequence);
    let mut cpu = canonical_cpu_layers(reference, &batch);
    // Existing P test-only prediction helper, never a production identity API.
    // Its predicted values are checked by G/P's actual candidate identity checks.
    let mut identities = predicted_identities(fixture, request.frame, &batch);
    let mut kinds = [SelectedTextureKind::InThisTransaction; 3];
    if let Some((lease, frame)) = historical {
        identities[0] = *lease.identity();
        kinds[0] = SelectedTextureKind::Validated;
        cpu[0] = frame;
    }
    let plans = plans();
    let selected = plans
        .iter()
        .zip(identities)
        .zip(kinds)
        .map(|((plan, identity), kind)| NativeCompositionLayer {
            plan,
            selected_entry: identity.entry,
            identity,
            kind,
        })
        .collect::<Vec<_>>();
    let limits = &fixture.owner.capabilities().limits;
    let plan = NativeCompositionPlan::preflight(
        request.context,
        request.frame,
        SIZE,
        [0; 4],
        &selected,
        limits.max_texture_dimension_2d,
        u64::from(limits.max_storage_buffer_binding_size),
    )
    .unwrap();
    let prepared = fixture
        .runtime
        .prepare_native_inputs(request.frame, batch, NativeBatchRestrictions::default())
        .unwrap();
    (prepared, plan, cpu)
}

fn admit(
    fixture: &mut Fixture,
    inputs: PreparedNativeInputs,
    plan: &NativeCompositionPlan,
    history: &[&GpuLayerLease],
) -> (PreparedNativeBatch, FrameReservation) {
    let mut merged = inputs.allocation_plan().unwrap().clone();
    let g_bytes = merged.requested_bytes();
    merged.merge(plan.allocation_plan()).unwrap();
    assert_eq!(
        merged.requested_bytes(),
        g_bytes + plan.allocation_plan().requested_bytes()
    );
    for lease in history {
        merged.add_existing(lease.allocation_id());
    }
    let quote = fixture.owner.quote(&merged).unwrap();
    let batch = match fixture.runtime.admit_native_batch(inputs, &quote) {
        NativeBatchAdmission::Accepted(batch) => batch,
        NativeBatchAdmission::Declined { reason, .. } => panic!("C3 declined: {reason:?}"),
    };
    (batch, fixture.owner.reserve(quote).unwrap())
}

fn start(
    fixture: &mut Fixture,
    compositor: &NativeGpuCompositor,
    request: NativeOutputRequest,
    inputs: PreparedNativeInputs,
    plan: &NativeCompositionPlan,
    history: &[&GpuLayerLease],
) -> PendingSharedNativeOutput {
    let before = fixture.runtime.native_frame_index();
    let (admitted, reservation) = admit(fixture, inputs, plan, history);
    let pending = PendingSharedNativeOutput::submit(
        &mut fixture.owner,
        &mut fixture.runtime,
        compositor,
        request,
        admitted,
        reservation,
        plan,
        history,
    )
    .unwrap_or_else(|failure| panic!("shared submit failed: {:?}", failure.reason()));
    assert_eq!(fixture.runtime.native_frame_index(), before.wrapping_add(3));
    pending
}

fn finish(
    mut pending: PendingSharedNativeOutput,
    fixture: &mut Fixture,
    cache: &mut SharedNativeOutputCache,
    authority: &Authority<'_>,
) -> SharedNativeOutputProgress {
    let started = Instant::now();
    loop {
        match pending.poll(
            &mut fixture.owner,
            &mut fixture.runtime,
            cache,
            authority,
            &|| authority.stopped.get(),
        ) {
            SharedNativeOutputProgress::Pending(next) => {
                assert!(
                    started.elapsed() < Duration::from_secs(5),
                    "C3 completion timed out"
                );
                pending = next;
                std::thread::yield_now();
            }
            other => return other,
        }
    }
}

fn assert_cpu(
    cache: &SharedNativeOutputCache,
    key: &VideoOutputLastValidKey,
    frames: &[Arc<VideoFrame>],
) {
    let oracle = crate::composite_frames_rgba8(
        &CompositionPlan {
            composition_id: 1,
            label: "I3 canonical CPU oracle".into(),
            output_ids: vec![],
            master_opacity: 1.0,
            blackout: false,
            layers: plans(),
        },
        frames,
        SIZE.0,
        SIZE.1,
    )
    .unwrap();
    assert_eq!(
        cache.last_valid(key).unwrap().data,
        oracle.data,
        "real G Batch3 + P snapshot + I LastValid must be byte-equal to CPU path"
    );
}

fn wait_allocations(owner: &mut NativeGpuOwner, count: usize) {
    let started = Instant::now();
    loop {
        let progress = owner.poll_retirement().unwrap();
        if matches!(progress, RetirementProgress::Pending { transactions: 0, allocations } if allocations == count)
        {
            return;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "allocation drain: {progress:?}"
        );
        std::thread::yield_now();
    }
}
fn reap(mut owner: NativeGpuOwner) {
    owner.begin_stop();
    let started = Instant::now();
    while owner.poll_retirement().unwrap() != RetirementProgress::Reaped {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "owner not reaped"
        );
        std::thread::yield_now();
    }
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}
fn reap_retirement(retiring: &mut SharedNativeOutputRetirement, owner: &mut NativeGpuOwner) {
    let started = Instant::now();
    while !retiring.poll(owner).unwrap() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "retirement not reaped"
        );
        std::thread::yield_now();
    }
    assert_eq!(owner.state(), GpuOwnerState::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

#[test]
fn i3_c3_real_batch3_snapshot_cache_ready_and_history_alias_match_cpu_exactly() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    eprintln!(
        "I3_REAL_ADAPTER {:?}",
        fixture.owner.capabilities().adapter_info
    );
    let request1 = output_request(&fixture, &mut cache, 1);
    let key = request1.key.clone();
    let (inputs1, plan1, cpu1) = prepare(&fixture, &mut reference, &request1, None);
    let pending1 = start(&mut fixture, &compositor, request1, inputs1, &plan1, &[]);
    assert!(
        cache.last_valid(&key).is_none(),
        "submitted is not LastValid/Fresh"
    );
    let (ready1, layers1) = match finish(pending1, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Ready { ready, layers, isf } => {
            assert_eq!(
                ready.snapshot_identity().completed_submission.sequence,
                isf.phase().submission().sequence + 1
            );
            assert_eq!(
                ready.evidence().freshness,
                VideoOutputRenderFreshness::Fresh
            );
            (ready, layers)
        }
        _ => panic!("first C3 frame did not become Ready"),
    };
    assert_cpu(&cache, &key, &cpu1);
    assert_eq!(
        ready1.payload().premapping().allocation_id(),
        ready1.snapshot_identity().allocation
    );
    assert_eq!(
        ready1.cache_receipt().descriptor().snapshot,
        ready1.snapshot_identity()
    );

    let old_layer = &layers1[0].lease;
    let request2 = output_request(&fixture, &mut cache, 2);
    let (inputs2, plan2, cpu2) = prepare(
        &fixture,
        &mut reference,
        &request2,
        Some((old_layer, Arc::clone(&cpu1[0]))),
    );
    assert_ne!(plan2.layers_bottom_first[0].identity.frame, request2.frame);
    let pending2 = start(
        &mut fixture,
        &compositor,
        request2,
        inputs2,
        &plan2,
        &[old_layer],
    );
    let (ready2, layers2) = match finish(pending2, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Ready { ready, layers, isf } => {
            assert_eq!(
                layers.len(),
                3,
                "all three current FX chains still executed"
            );
            assert_eq!(isf.phase().frame(), frame_id(2));
            (ready, layers)
        }
        _ => panic!("history-selected C3 frame did not become Ready"),
    };
    assert_eq!(fixture.runtime.native_frame_index(), 6);
    assert_cpu(&cache, &key, &cpu2);
    assert_ne!(
        ready1.snapshot_identity().allocation,
        ready2.snapshot_identity().allocation
    );
    let old_gpu = Arc::downgrade(ready1.payload());
    let new_gpu = Arc::downgrade(ready2.payload());
    drop(layers1);
    drop(layers2);
    drop(ready1);
    assert!(
        old_gpu.upgrade().is_none(),
        "replacement does not retain the old payload"
    );
    drop(ready2);
    assert!(
        new_gpu.upgrade().is_some(),
        "LastValid must retain CPU charge after Ready drops"
    );
    wait_allocations(&mut fixture.owner, 2);
    assert_eq!(
        fixture.owner.budget_snapshot().allocated_bytes,
        2 * u64::from(SIZE.0 * SIZE.1 * 4)
    );
    cache.blackout().unwrap();
    assert!(cache.last_valid(&key).is_none());
    assert!(new_gpu.upgrade().is_none());
    wait_allocations(&mut fixture.owner, 0);
    drop(reference);
    drop(fixture.runtime);
    reap(fixture.owner);
}

#[test]
fn i3_c3_cancelled_submitted_snapshot_retains_owner_until_drain_and_reaped() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
    let pending = start(&mut fixture, &compositor, request, inputs, &plan, &[]);
    let before = fixture.owner.budget_snapshot().allocated_bytes;
    assert!(before > 0);
    let stopped = Cell::new(true);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("cancelled output escaped retirement"),
    };
    assert_eq!(retiring.reason(), Some(&OutputRejection::Cancelled));
    assert!(cache.last_valid(&key).is_none());
    assert!(retiring.snapshot.is_some());
    assert!(fixture.owner.budget_snapshot().allocated_bytes > 0);
    assert!(retiring.take_isf_outcome().is_none());
    drop(fixture.runtime);
    reap_retirement(&mut retiring, &mut fixture.owner);
}

#[test]
fn i3_c3_post_commit_stop_keeps_last_valid_charge_until_cache_release() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let (inputs, plan, cpu) = prepare(&fixture, &mut reference, &request, None);
    let pending = start(&mut fixture, &compositor, request, inputs, &plan, &[]);
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: true,
    };
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("post-commit Stop became Ready"),
    };
    assert_eq!(retiring.reason(), Some(&OutputRejection::Cancelled));
    assert_cpu(&cache, &key, &cpu);
    assert!(matches!(
        retiring.take_isf_outcome(),
        Some(NativeIsfCompletion::Validated { .. })
    ));
    drop(fixture.runtime);
    for _ in 0..3 {
        assert!(!retiring.poll(&mut fixture.owner).unwrap());
    }
    assert!(fixture.owner.budget_snapshot().allocated_bytes >= 2 * u64::from(SIZE.0 * SIZE.1 * 4));
    cache.blackout().unwrap();
    reap_retirement(&mut retiring, &mut fixture.owner);
}

#[test]
fn i3_c3_composition_selection_failure_keeps_valid_isf_history_and_no_cache() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let (inputs, mut plan, _) = prepare(&fixture, &mut reference, &request, None);
    plan.layers_bottom_first[0].identity.content_sequence += 100;
    let pending = start(&mut fixture, &compositor, request, inputs, &plan, &[]);
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("mismatched selection became Ready"),
    };
    assert!(
        matches!(retiring.reason(), Some(OutputRejection::Fault(error))
        if error.phase == GpuPhase::CompositionSnapshot && error.kind == NativeGpuFaultKind::IdentityMismatch)
    );
    assert!(cache.last_valid(&key).is_none());
    match retiring.take_isf_outcome().unwrap() {
        NativeIsfCompletion::Validated { layers, receipt } => {
            assert_eq!(layers.len(), 3);
            assert_eq!(receipt.phase().phase(), GpuPhase::IsfBatch);
        }
        _ => panic!("P failure lost successful ISF history"),
    }
    drop(fixture.runtime);
    reap_retirement(&mut retiring, &mut fixture.owner);
}

#[test]
fn i3_c3_snapshot_identity_failure_retains_real_mapper_until_reaped() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let key = request.key.clone();
    let context = request.context;
    let frame = request.frame;
    let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
    let mut pending = start(&mut fixture, &compositor, request, inputs, &plan, &[]);
    let actual_pts = pending
        .snapshot
        .as_ref()
        .unwrap()
        .descriptor()
        .snapshot
        .pts_ms;
    // Corrupt only I's expected binding, never the actual G/P receipts or mapper.
    pending.request.pts_ms = actual_pts + 1;
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("mismatched snapshot identity escaped retirement"),
    };
    assert!(
        matches!(retiring.reason(), Some(OutputRejection::Fault(error))
        if error.phase == GpuPhase::CompositionSnapshot
            && error.kind == NativeGpuFaultKind::IdentityMismatch
            && error.context == context && error.frame == Some(frame))
    );
    assert!(cache.last_valid(&key).is_none());
    assert!(cache.snapshot_owner.is_none());
    assert!(
        retiring.snapshot.is_some(),
        "failure retains the actual P retirement owner"
    );
    assert!(fixture.owner.budget_snapshot().managed_bytes() > 0);
    match retiring.take_isf_outcome().unwrap() {
        NativeIsfCompletion::Validated { layers, receipt } => {
            assert_eq!(layers.len(), 3);
            assert_eq!(receipt.phase().frame(), frame);
            assert!(receipt.phase().validation().is_ok());
        }
        _ => panic!("snapshot identity failure lost valid real ISF history"),
    }
    drop(fixture.runtime);
    reap_retirement(&mut retiring, &mut fixture.owner);
}

#[test]
fn i3_c3_foreign_poll_and_invalid_request_preserve_owners_without_work() {
    let mut fixture = gpu();
    let mut foreign = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let request = output_request(&fixture, &mut cache, 1);
    let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
    let pending = start(&mut fixture, &compositor, request, inputs, &plan, &[]);
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let pending = match pending.poll(
        &mut foreign.owner,
        &mut fixture.runtime,
        &mut cache,
        &authority,
        &|| false,
    ) {
        SharedNativeOutputProgress::Rejected { pending, reason } => {
            assert!(
                matches!(reason, OutputRejection::Fault(error) if error.kind == NativeGpuFaultKind::IdentityMismatch)
            );
            pending
        }
        _ => panic!("foreign owner consumed a submitted frame"),
    };
    assert_eq!(foreign.owner.state(), GpuOwnerState::Active);
    stopped.set(true);
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("cancel after foreign rejection failed"),
    };
    drop(fixture.runtime);
    reap_retirement(&mut retiring, &mut fixture.owner);
    drop(foreign.runtime);
    reap(foreign.owner);

    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let mut cache = SharedNativeOutputCache::new(101);
    let mut request = output_request(&fixture, &mut cache, 1);
    let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
    let (admitted, reservation) = admit(&mut fixture, inputs, &plan, &[]);
    request.context.device_generation += 100;
    let before = fixture.runtime.native_frame_index();
    let mut retiring = match PendingSharedNativeOutput::submit(
        &mut fixture.owner,
        &mut fixture.runtime,
        &compositor,
        request,
        admitted,
        reservation,
        &plan,
        &[],
    ) {
        Err(retiring) => retiring,
        Ok(_) => panic!("wrong request context submitted work"),
    };
    assert_eq!(fixture.runtime.native_frame_index(), before);
    assert!(retiring.unsubmitted.is_some());
    assert!(retiring.admitted.is_some());
    drop(fixture.runtime);
    reap_retirement(&mut retiring, &mut fixture.owner);
}
