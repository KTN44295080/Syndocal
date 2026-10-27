//! I3 real G/P/I join. No map/success receipts are fabricated.
use super::*;
use crate::gpu_surface::native::{
    AdmittedPhysicalGeometry, NativePresentReceipt, NativePresentTarget, TargetAcquireError,
};
use crate::isf_native_gpu::tests::{gpu, jobs, request as frame_id, shader, Fixture, IDENTITY};
use crate::isf_native_gpu::{
    NativeBatchAdmission, NativeBatchRestrictions, PreparedNativeInputs, PreparedNativeLayer,
};
use crate::isf_runtime::IsfStageInputs;
use crate::native_gpu_composition::tests::layer;
use crate::native_gpu_composition::{NativeCompositionLayer, SelectedTextureKind};
use crate::CompositionPlan;
use protocol::{Transform2D, VideoBlendMode, VideoOutputKind};
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;
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
    let prepared = fixture
        .runtime
        .prepare_native_inputs(request.frame, batch, NativeBatchRestrictions::default())
        .unwrap();
    let mut identities = prepared.prospective_identities(&fixture.runtime).unwrap();
    let mut kinds = [SelectedTextureKind::InThisTransaction; 3];
    if let Some((lease, frame)) = historical {
        identities[0] = *lease.identity();
        kinds[0] = SelectedTextureKind::Validated;
        cpu[0] = frame;
    }
    let plans = plans();
    let selected = plans
        .iter()
        .zip(identities.iter().copied())
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
    // Isolate retained LastValid/Ready/transaction ownership from the new
    // explicitly cached idle ISF storage; keep the exact charge assertions.
    owner.release_resource_pools();
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

type SharedReady = ReadyNativeGpuOutput<Arc<NativeSnapshotGpu>>;
type SharedPresentAttempt =
    NativePresentation<Arc<NativeSnapshotGpu>, Option<SharedReady>, NativePresentReceipt>;

#[derive(Default)]
struct PresentProbe {
    failures: VecDeque<TargetAcquireError>,
    acquires: usize,
    discards: usize,
    presented: Vec<wgpu::Texture>,
}

/// Only the swapchain image is replaced. P's actual mapping, blit, submissions,
/// retirement list and I's real Ready/permit/cache evidence remain in the path.
struct PresentTarget {
    device: wgpu::Device,
    extent: Option<(u32, u32)>,
    probe: Rc<RefCell<PresentProbe>>,
}

impl NativePresentTarget for PresentTarget {
    type Frame = wgpu::Texture;

    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }

    fn configure(&mut self, _: &wgpu::Device, extent: (u32, u32)) {
        self.extent = Some(extent);
    }

    fn acquire(&mut self) -> Result<Self::Frame, TargetAcquireError> {
        let mut probe = self.probe.borrow_mut();
        probe.acquires += 1;
        if let Some(failure) = probe.failures.pop_front() {
            return Err(failure);
        }
        let (width, height) = self.extent.expect("present target must be configured");
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("I4 shared sink offscreen image"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }))
    }

    fn view(&self, frame: &Self::Frame) -> wgpu::TextureView {
        frame.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn present(&mut self, frame: Self::Frame) {
        self.probe.borrow_mut().presented.push(frame);
    }

    fn discard(&mut self, frame: Self::Frame) {
        self.probe.borrow_mut().discards += 1;
        drop(frame);
    }
}

fn present_fixture(
    fixture: &mut Fixture,
) -> (
    SharedNativePresenter<PresentTarget>,
    Rc<RefCell<PresentProbe>>,
) {
    let probe = Rc::new(RefCell::new(PresentProbe::default()));
    let target = PresentTarget {
        device: fixture.owner.context().device().clone(),
        extent: None,
        probe: Rc::clone(&probe),
    };
    let mut presenter = SharedNativePresenter::new(target, &mut fixture.owner).unwrap();
    presenter
        .configure_admitted(
            AdmittedPhysicalGeometry {
                extent: SIZE,
                geometry_generation: frame_id(1).geometry_generation,
            },
            &mut fixture.owner,
        )
        .unwrap();
    (presenter, probe)
}

fn ready_for_present(
    fixture: &mut Fixture,
    reference: &mut IsfGpuRuntime,
    compositor: &NativeGpuCompositor,
    presenter: &mut SharedNativePresenter<PresentTarget>,
    cache: &mut SharedNativeOutputCache,
    sequence: u64,
    mapping: VideoOutputMapping,
) -> SharedReady {
    let mut request = output_request(fixture, cache, sequence);
    request.mapping_identity = VideoOutputMappingIdentity::from_mapping(&mapping);
    request.mapping = mapping;
    let key = request.key.clone();
    let (inputs, plan, cpu) = prepare(fixture, reference, &request, None);
    let before = fixture.runtime.native_frame_index();
    let (admitted, reservation) = admit(fixture, inputs, &plan, &[]);
    let stopped = Cell::new(false);
    let authority = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let pending = PendingSharedNativeOutput::submit_after_present(
        &mut fixture.owner,
        &mut fixture.runtime,
        compositor,
        presenter,
        request,
        admitted,
        reservation,
        &plan,
        &[],
        &authority,
        &|| stopped.get(),
    )
    .unwrap_or_else(|failure| panic!("present-aware submit failed: {:?}", failure.reason()));
    assert_eq!(fixture.runtime.native_frame_index(), before.wrapping_add(3));
    let ready = match finish(pending, fixture, cache, &authority) {
        SharedNativeOutputProgress::Ready { ready, layers, isf } => {
            assert_eq!(layers.len(), 3);
            assert!(isf.phase().validation().is_ok());
            ready
        }
        _ => panic!("real C3 frame did not become a presentable Ready"),
    };
    assert_cpu(cache, &key, &cpu);
    ready
}

struct PresentAuthority<'a> {
    stopped: &'a Cell<bool>,
    mapping: VideoOutputMappingIdentity,
    rejection: Option<(OutputBoundary, bool)>,
    boundaries: RefCell<Vec<OutputBoundary>>,
}

impl NativeOutputAuthority for PresentAuthority<'_> {
    fn revalidate(
        &self,
        request: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        assert_eq!(request.mapping_identity, self.mapping);
        assert_eq!(
            VideoOutputMappingIdentity::from_mapping(&request.mapping),
            self.mapping,
            "the permit sees the same frozen mapping as the actual Ready",
        );
        self.boundaries.borrow_mut().push(boundary);
        if let Some((expected, stop)) = self.rejection {
            if boundary == expected {
                if stop {
                    self.stopped.set(true);
                } else {
                    return Err(PresentRejection::Revoked);
                }
            }
        }
        Ok(())
    }
}

fn present_actual(
    ready: SharedReady,
    presenter: &mut SharedNativePresenter<PresentTarget>,
    owner: &mut NativeGpuOwner,
    authority: &PresentAuthority<'_>,
    delivery: &mut NativeDeliveryCounter,
) -> SharedPresentAttempt {
    // The callback only moves the whole Ready. P exclusively borrows the broker
    // during the sink call; a retained Ready is polled afterward.
    let mut sink = presenter.sink(owner).unwrap();
    present_native_output_with_retirement(
        ready,
        Some,
        &mut sink,
        authority,
        &|| authority.stopped.get(),
        delivery,
    )
}

fn presented(result: SharedPresentAttempt) -> NativeDelivery<NativePresentReceipt> {
    match result {
        NativePresentation::Presented(delivery) => delivery,
        _ => panic!("real shared sink did not report Presented"),
    }
}

fn retire_present_ready(
    retiring: &mut RetiringNativeOutput<Option<SharedReady>>,
    presenter: &mut SharedNativePresenter<PresentTarget>,
    owner: &mut NativeGpuOwner,
) {
    release_shared_presenter_for_stop(presenter, owner).unwrap();
    let started = Instant::now();
    while !retiring
        .poll_with(|ready| {
            match owner.poll_retirement()? {
                RetirementProgress::Reaped => return Ok(true),
                RetirementProgress::Pending {
                    transactions: 0, ..
                } if owner.state() == GpuOwnerState::Retiring => {
                    // Reaped requires zero charges. Release the Ready only after
                    // its real dependent transactions drained; retain this ticket.
                    *ready = None;
                }
                RetirementProgress::Pending { .. } => {}
            }
            Ok(false)
        })
        .unwrap()
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    assert!(retiring.is_retired());
    assert_eq!(owner.state(), GpuOwnerState::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

fn assert_present_retirement_allows_next_frame(
    owner: &mut NativeGpuOwner,
    next_frame: NativeFrameId,
) {
    let plan = crate::native_gpu_budget::NativeAllocationPlan::new(next_frame);
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = match owner.begin_frame(next_frame, reservation) {
        Ok(txn) => txn,
        Err(error) => panic!("a retiring present blocked the next logical frame: {error:?}"),
    };
    match txn.begin_phase(GpuPhase::MappingPresent) {
        Err(error) => assert_eq!(error.kind, NativeGpuFaultKind::StateMismatch),
        Ok(_) => panic!("G admitted a second present while the prior one still retires"),
    }
    // Resolve this empty logical probe only. P still owns the original present
    // token; no presenter poll or drain is needed for the next begin_frame.
    let pending = txn.finish();
    drop(crate::isf_native_gpu::await_transaction(owner, pending, &|| false).unwrap());
}

fn present_two_with_retiring_second(
    fixture: &mut Fixture,
    reference: &mut IsfGpuRuntime,
    compositor: &NativeGpuCompositor,
    presenter: &mut SharedNativePresenter<PresentTarget>,
    cache: &mut SharedNativeOutputCache,
) -> NativeDeliveryCounter {
    let stopped = Cell::new(false);
    let mapping = VideoOutputMapping::default();
    let authority = PresentAuthority {
        stopped: &stopped,
        mapping: VideoOutputMappingIdentity::from_mapping(&mapping),
        rejection: None,
        boundaries: RefCell::new(Vec::new()),
    };
    let mut delivery = NativeDeliveryCounter::default();
    for sequence in 1..=2 {
        let ready = ready_for_present(
            fixture,
            reference,
            compositor,
            presenter,
            cache,
            sequence,
            mapping.clone(),
        );
        presented(present_actual(
            ready,
            presenter,
            &mut fixture.owner,
            &authority,
            &mut delivery,
        ));
    }
    assert_eq!(delivery.delivered(), 2);
    assert_eq!(fixture.runtime.native_frame_index(), 6);
    assert_present_retirement_allows_next_frame(&mut fixture.owner, frame_id(3));
    delivery
}

fn reap_shared_retirement(
    retiring: &mut SharedNativeOutputRetirement,
    presenter: &mut SharedNativePresenter<PresentTarget>,
    owner: &mut NativeGpuOwner,
) {
    let started = Instant::now();
    while !retiring.poll_with_presenter(presenter, owner).unwrap() {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    assert_eq!(owner.state(), GpuOwnerState::Reaped);
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(owner.budget_snapshot().reservation_count, 0);
}

struct SecondSubmitGuard {
    calls: Cell<usize>,
    stopped: Cell<bool>,
    cancel: bool,
}

impl NativeOutputAuthority for SecondSubmitGuard {
    fn revalidate(
        &self,
        _: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        if boundary == OutputBoundary::BeforeSubmit {
            self.calls.set(self.calls.get() + 1);
            if self.calls.get() == 2 {
                if self.cancel {
                    self.stopped.set(true);
                } else {
                    return Err(PresentRejection::Revoked);
                }
            }
        }
        Ok(())
    }
}

/// Oracle-only readback, after P's real submission has drained. It is outside
/// the output adapter and supplies no cache, completion, Ready or delivery proof.
fn presented_pixels(owner: &NativeGpuOwner, image: &wgpu::Texture) -> Vec<u8> {
    let device = owner.context().device();
    let row = image.width() * 4;
    let pitch = row.div_ceil(256) * 256;
    let height = image.height();
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("I4 presented image oracle readback"),
        size: u64::from(pitch) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: image,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(pitch),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width: image.width(),
            height,
            depth_or_array_layers: 1,
        },
    );
    owner.context().queue().submit(Some(encoder.finish()));
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait()).unwrap();
    receiver
        .recv_timeout(Duration::from_secs(5))
        .unwrap()
        .unwrap();
    let view = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((row * height) as usize);
    for y in 0..height as usize {
        let start = y * pitch as usize;
        pixels.extend_from_slice(&view[start..start + row as usize]);
    }
    drop(view);
    buffer.unmap();
    pixels
}

fn frozen_projection() -> VideoOutputMapping {
    VideoOutputMapping {
        offset_x: 0.07,
        offset_y: -0.035,
        scale_x: 0.83,
        scale_y: 0.9,
        edge_blend_left: 0.18,
        edge_blend_right: 0.12,
        edge_blend_gamma: 2.0,
        black_level: 0.04,
        ..VideoOutputMapping::default()
    }
}

#[test]
fn i4_shared_sink_presents_real_ready_with_frozen_mapping_once_and_delivery() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let (mut presenter, probe) = present_fixture(&mut fixture);
    let mut cache = SharedNativeOutputCache::new(101);
    let frozen = frozen_projection();
    let stopped = Cell::new(false);
    let authority = PresentAuthority {
        stopped: &stopped,
        mapping: VideoOutputMappingIdentity::from_mapping(&frozen),
        rejection: None,
        boundaries: RefCell::new(Vec::new()),
    };
    let mut delivery = NativeDeliveryCounter::default();
    let mut frames = Vec::new();
    let mut last_key = None;
    for sequence in 1..=3 {
        let ready = ready_for_present(
            &mut fixture,
            &mut reference,
            &compositor,
            &mut presenter,
            &mut cache,
            sequence,
            frozen.clone(),
        );
        let snapshot = ready.snapshot_identity();
        let key = ready.cache_receipt().descriptor().key.clone();
        let premapping = cache.last_valid(&key).unwrap().clone();
        assert_eq!(
            VideoOutputMappingIdentity::from_mapping(ready.mapping()),
            VideoOutputMappingIdentity::from_mapping(&frozen),
        );
        let retained = Arc::downgrade(ready.payload());
        let receipt = presented(present_actual(
            ready,
            &mut presenter,
            &mut fixture.owner,
            &authority,
            &mut delivery,
        ));
        assert_eq!(delivery.delivered(), sequence);
        assert_eq!(receipt.frame(), snapshot.frame);
        assert_eq!(receipt.receipt().snapshot(), snapshot);
        assert_eq!(
            receipt.receipt().submission().sequence,
            snapshot.completed_submission.sequence + 1,
        );
        assert_eq!(presenter.presents(), sequence);
        assert_eq!(probe.borrow().acquires, sequence as usize);
        assert_eq!(probe.borrow().discards, 0);
        assert_eq!(
            &authority.boundaries.borrow()[(sequence as usize - 1) * 3..],
            &[
                OutputBoundary::BeforePresentCall,
                OutputBoundary::BeforeAcquire,
                OutputBoundary::BeforePhysicalPresent,
            ],
        );
        assert_eq!(cache.last_valid(&key).unwrap().data, premapping.data);
        assert_eq!(fixture.runtime.native_frame_index(), sequence as u32 * 3);
        frames.push((premapping, retained));
        last_key = Some(key);
        if sequence == 2 {
            // G3 leaves the steady present in retirement while frame 3 can
            // begin. Only the next MappingPresent must wait for that retirement.
            assert_present_retirement_allows_next_frame(&mut fixture.owner, frame_id(3));
        }
    }
    presenter.drain_retirements(&mut fixture.owner).unwrap();
    for (index, (premapping, retained)) in frames.iter().enumerate() {
        assert_eq!(
            retained.upgrade().is_some(),
            index == 2,
            "only the latest LastValid retains its snapshot charge",
        );
        let pixels = presented_pixels(&fixture.owner, &probe.borrow().presented[index]);
        let once = crate::apply_video_output_mapping(premapping.clone(), &frozen);
        let twice = crate::apply_video_output_mapping(once.clone(), &frozen);
        assert_eq!(
            pixels, once.data,
            "actual I Ready feeds exactly one P mapping"
        );
        assert_ne!(pixels, premapping.data, "the frozen mapping changes pixels");
        assert_ne!(
            pixels, twice.data,
            "a second mapping would change the output"
        );
    }
    assert_eq!(
        cache.last_valid(&last_key.unwrap()).unwrap().data,
        frames.last().unwrap().0.data,
    );
    cache.blackout().unwrap();
    assert!(frames.last().unwrap().1.upgrade().is_none());
    release_shared_presenter_for_stop(&mut presenter, &mut fixture.owner).unwrap();
    drop(fixture.runtime);
    reap(fixture.owner);
}

#[test]
fn i4_shared_sink_stop_and_revocation_preserve_real_ready_through_drain() {
    for boundary in [
        OutputBoundary::BeforeAcquire,
        OutputBoundary::BeforePhysicalPresent,
    ] {
        for stop in [false, true] {
            let mut fixture = gpu();
            let mut reference = IsfGpuRuntime::new().unwrap();
            let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
            let (mut presenter, probe) = present_fixture(&mut fixture);
            let mut cache = SharedNativeOutputCache::new(101);
            let mapping = VideoOutputMapping::default();
            let stopped = Cell::new(false);
            let mut delivery = NativeDeliveryCounter::default();
            let mut sequence = 1;
            if boundary == OutputBoundary::BeforePhysicalPresent {
                // Warm first-use validation so the rejected frame leaves a real
                // submitted MappingPresent transaction for retirement to drain.
                let warm = ready_for_present(
                    &mut fixture,
                    &mut reference,
                    &compositor,
                    &mut presenter,
                    &mut cache,
                    sequence,
                    mapping.clone(),
                );
                let allow = PresentAuthority {
                    stopped: &stopped,
                    mapping: VideoOutputMappingIdentity::from_mapping(&mapping),
                    rejection: None,
                    boundaries: RefCell::new(Vec::new()),
                };
                presented(present_actual(
                    warm,
                    &mut presenter,
                    &mut fixture.owner,
                    &allow,
                    &mut delivery,
                ));
                sequence += 1;
            }
            let ready = ready_for_present(
                &mut fixture,
                &mut reference,
                &compositor,
                &mut presenter,
                &mut cache,
                sequence,
                mapping.clone(),
            );
            let retained = Arc::downgrade(ready.payload());
            let before_delivery = delivery.delivered();
            let before_acquires = probe.borrow().acquires;
            let before_presents = probe.borrow().presented.len();
            let authority = PresentAuthority {
                stopped: &stopped,
                mapping: VideoOutputMappingIdentity::from_mapping(&mapping),
                rejection: Some((boundary, stop)),
                boundaries: RefCell::new(Vec::new()),
            };
            let mut retiring = match present_actual(
                ready,
                &mut presenter,
                &mut fixture.owner,
                &authority,
                &mut delivery,
            ) {
                NativePresentation::Retiring(retiring) => retiring,
                _ => panic!("rejected physical boundary did not retain the whole Ready"),
            };
            assert_eq!(
                retiring.reason(),
                &if stop {
                    OutputRejection::Cancelled
                } else {
                    OutputRejection::Authority(PresentRejection::Revoked)
                },
            );
            assert_eq!(delivery.delivered(), before_delivery);
            assert_eq!(probe.borrow().presented.len(), before_presents);
            assert_eq!(
                probe.borrow().acquires - before_acquires,
                usize::from(boundary == OutputBoundary::BeforePhysicalPresent),
            );
            assert_eq!(
                probe.borrow().discards,
                usize::from(boundary == OutputBoundary::BeforePhysicalPresent),
            );
            assert!(authority.boundaries.borrow().contains(&boundary));
            cache.blackout().unwrap();
            assert!(
                retained.upgrade().is_some(),
                "retirement owns Ready after cache release"
            );
            drop(fixture.runtime);
            retire_present_ready(&mut retiring, &mut presenter, &mut fixture.owner);
            assert!(retained.upgrade().is_none());
        }
    }
}

#[test]
fn i4_shared_sink_retry_preserves_real_snapshot_and_allows_one_retry_only() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let (mut presenter, probe) = present_fixture(&mut fixture);
    let mut cache = SharedNativeOutputCache::new(101);
    let stopped = Cell::new(false);
    let authority = PresentAuthority {
        stopped: &stopped,
        mapping: VideoOutputMappingIdentity::from_mapping(&VideoOutputMapping::default()),
        rejection: None,
        boundaries: RefCell::new(Vec::new()),
    };
    let mut delivery = NativeDeliveryCounter::default();

    for sequence in 1..=2 {
        // The actual I admission wrapper polls the preceding present once
        // without draining it before the next C3 frame begins.
        let ready = ready_for_present(
            &mut fixture,
            &mut reference,
            &compositor,
            &mut presenter,
            &mut cache,
            sequence,
            VideoOutputMapping::default(),
        );
        let snapshot = ready.snapshot_identity();
        let retained = Arc::downgrade(ready.payload());
        probe
            .borrow_mut()
            .failures
            .push_back(TargetAcquireError::Lost);
        if sequence == 2 {
            probe
                .borrow_mut()
                .failures
                .push_back(TargetAcquireError::Outdated);
        }
        let before = delivery.delivered();
        let retry = match present_actual(
            ready,
            &mut presenter,
            &mut fixture.owner,
            &authority,
            &mut delivery,
        ) {
            NativePresentation::Retry(ready) => ready,
            _ => panic!("first surface loss did not return the original Ready"),
        };
        assert_eq!(retry.snapshot_identity(), snapshot);
        assert!(Arc::ptr_eq(retry.payload(), &retained.upgrade().unwrap()));
        assert_eq!(fixture.runtime.native_frame_index(), sequence as u32 * 3);
        assert_eq!(delivery.delivered(), before);
        let outcome = present_actual(
            retry,
            &mut presenter,
            &mut fixture.owner,
            &authority,
            &mut delivery,
        );
        if sequence == 1 {
            let receipt = presented(outcome);
            assert_eq!(receipt.receipt().snapshot(), snapshot);
            assert_eq!(delivery.delivered(), 1);
            assert_eq!(probe.borrow().presented.len(), 1);
        } else {
            let mut retiring = match outcome {
                NativePresentation::Retiring(retiring) => retiring,
                _ => panic!("a second surface loss exceeded the one-retry allowance"),
            };
            assert_eq!(
                retiring.reason(),
                &OutputRejection::Authority(PresentRejection::SurfaceLostOrOutdated),
            );
            assert_eq!(
                delivery.delivered(),
                1,
                "only the first actual Presented is delivery"
            );
            assert_eq!(probe.borrow().presented.len(), 1);
            assert_eq!(
                fixture.runtime.native_frame_index(),
                6,
                "retry never reruns FX"
            );
            assert_eq!(probe.borrow().acquires, 4);
            cache.blackout().unwrap();
            assert!(retained.upgrade().is_some());
            drop(fixture.runtime);
            retire_present_ready(&mut retiring, &mut presenter, &mut fixture.owner);
            assert!(retained.upgrade().is_none());
            return;
        }
    }
    unreachable!("second sequence must exhaust its retry");
}

#[test]
fn i5_shared_submit_rechecks_stop_and_authority_after_nonblocking_present_poll() {
    for cancel in [false, true] {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let (mut presenter, probe) = present_fixture(&mut fixture);
        let mut cache = SharedNativeOutputCache::new(101);
        let delivery = present_two_with_retiring_second(
            &mut fixture,
            &mut reference,
            &compositor,
            &mut presenter,
            &mut cache,
        );
        let request = output_request(&fixture, &mut cache, 3);
        let key = request.key.clone();
        let previous_pixels = cache.last_valid(&key).unwrap().data.as_ptr();
        let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
        let (admitted, reservation) = admit(&mut fixture, inputs, &plan, &[]);
        let authority = SecondSubmitGuard {
            calls: Cell::new(0),
            stopped: Cell::new(false),
            cancel,
        };
        let mut retiring = match PendingSharedNativeOutput::submit_after_present(
            &mut fixture.owner,
            &mut fixture.runtime,
            &compositor,
            &mut presenter,
            request,
            admitted,
            reservation,
            &plan,
            &[],
            &authority,
            &|| authority.stopped.get(),
        ) {
            Err(retiring) => retiring,
            Ok(_) => panic!("the second submit guard must reject before FX or begin_frame"),
        };
        assert_eq!(authority.calls.get(), 2);
        assert_eq!(
            retiring.reason(),
            Some(&if cancel {
                OutputRejection::Cancelled
            } else {
                OutputRejection::Authority(PresentRejection::Revoked)
            }),
        );
        assert_eq!(fixture.runtime.native_frame_index(), 6);
        assert_eq!(delivery.delivered(), 2);
        assert_eq!(presenter.presents(), 2);
        assert_eq!(probe.borrow().presented.len(), 2);
        assert_eq!(probe.borrow().acquires, 2);
        assert_eq!(
            cache.last_valid(&key).unwrap().data.as_ptr(),
            previous_pixels
        );
        assert!(retiring.unsubmitted.is_some());
        assert!(retiring.admitted.is_some());
        assert!(retiring.transaction.is_none());
        assert!(retiring.snapshot.is_none());
        assert!(retiring.encoded_isf.is_none());
        assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
        cache.blackout().unwrap();
        drop(fixture.runtime);
        reap_shared_retirement(&mut retiring, &mut presenter, &mut fixture.owner);
        assert!(retiring.unsubmitted.is_none());
        assert!(retiring.admitted.is_none());
    }
}

#[test]
fn i5_shared_stop_retains_unsubmitted_or_overlapping_snapshot_and_latches_reaped() {
    for submitted in [false, true] {
        let mut fixture = gpu();
        let mut reference = IsfGpuRuntime::new().unwrap();
        let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
        let (mut presenter, probe) = present_fixture(&mut fixture);
        let mut cache = SharedNativeOutputCache::new(101);
        let delivery = present_two_with_retiring_second(
            &mut fixture,
            &mut reference,
            &compositor,
            &mut presenter,
            &mut cache,
        );
        let request = output_request(&fixture, &mut cache, 3);
        let key = request.key.clone();
        let previous_pixels = cache.last_valid(&key).unwrap().data.as_ptr();
        let previous_present = Arc::downgrade(cache.snapshot_owner.as_ref().unwrap());
        let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
        let (admitted, reservation) = admit(&mut fixture, inputs, &plan, &[]);
        let stopped = Cell::new(false);
        let mut retiring = if submitted {
            // Deliberately do not poll P here: exercise actual Batch3 plus
            // snapshot submission while its preceding steady present is still
            // owned. Ordinary ticks use submit_after_present's nonblocking poll.
            let pending = PendingSharedNativeOutput::submit(
                &mut fixture.owner,
                &mut fixture.runtime,
                &compositor,
                request,
                admitted,
                reservation,
                &plan,
                &[],
            )
            .unwrap_or_else(|failure| {
                panic!("overlapping snapshot submit failed: {:?}", failure.reason())
            });
            assert!(pending.snapshot.is_some());
            assert_eq!(fixture.runtime.native_frame_index(), 9);
            stopped.set(true);
            let authority = Authority {
                stopped: &stopped,
                cancel_after_commit: false,
            };
            let retiring = match finish(pending, &mut fixture, &mut cache, &authority) {
                SharedNativeOutputProgress::Retiring(retiring) => retiring,
                _ => panic!("submitted overlap did not retain its cancelled snapshot"),
            };
            assert_eq!(retiring.reason(), Some(&OutputRejection::Cancelled));
            assert!(retiring.snapshot.is_some());
            assert!(retiring.transaction.is_some());
            assert!(retiring.encoded_isf.is_some());
            assert!(retiring.unsubmitted.is_none());
            retiring
        } else {
            // A real authority rejection before the tick's present poll keeps
            // its unused affine admission and the preceding present intact.
            let authority = PresentAuthority {
                stopped: &stopped,
                mapping: request.mapping_identity,
                rejection: Some((OutputBoundary::BeforeSubmit, false)),
                boundaries: RefCell::new(Vec::new()),
            };
            let retiring = match PendingSharedNativeOutput::submit_after_present(
                &mut fixture.owner,
                &mut fixture.runtime,
                &compositor,
                &mut presenter,
                request,
                admitted,
                reservation,
                &plan,
                &[],
                &authority,
                &|| stopped.get(),
            ) {
                Err(retiring) => retiring,
                Ok(_) => panic!("revoked admission submitted the next frame"),
            };
            assert_eq!(
                *authority.boundaries.borrow(),
                vec![OutputBoundary::BeforeSubmit],
            );
            assert_eq!(
                retiring.reason(),
                Some(&OutputRejection::Authority(PresentRejection::Revoked)),
            );
            assert_eq!(fixture.runtime.native_frame_index(), 6);
            assert!(retiring.unsubmitted.is_some());
            assert!(retiring.admitted.is_some());
            assert!(retiring.transaction.is_none());
            assert!(retiring.snapshot.is_none());
            assert!(retiring.encoded_isf.is_none());
            retiring
        };
        assert!(fixture.owner.budget_snapshot().managed_bytes() > 0);
        assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
        assert_eq!(delivery.delivered(), 2);
        assert_eq!(presenter.presents(), 2);
        assert_eq!(probe.borrow().presented.len(), 2);
        assert_eq!(
            cache.last_valid(&key).unwrap().data.as_ptr(),
            previous_pixels
        );
        cache.blackout().unwrap();
        assert!(
            previous_present.upgrade().is_some(),
            "the prior present retains its whole Ready after LastValid is released",
        );
        drop(fixture.runtime);
        // The aggregate conservatively retains either the unused root or the
        // submitted snapshot until all dependent transactions have drained.
        let started = Instant::now();
        while retiring.unsubmitted.is_some() || retiring.snapshot.is_some() {
            assert!(
                !retiring
                    .poll_with_presenter(&mut presenter, &mut fixture.owner)
                    .unwrap(),
                "the owned root or snapshot charge prevents Reaped before release",
            );
            assert!(started.elapsed() < Duration::from_secs(5));
            std::thread::yield_now();
        }
        assert!(retiring.unsubmitted.is_none());
        assert!(retiring.admitted.is_none());
        assert!(retiring.snapshot.is_none());
        assert!(retiring.encoded_isf.is_none());
        assert!(!retiring.reaped);
        assert_eq!(fixture.owner.state(), GpuOwnerState::Retiring);
        // An outer drain observes G's real one-shot Reaped result first. Its
        // persistent owner state must complete this aggregate on the next poll.
        loop {
            match fixture.owner.poll_retirement().unwrap() {
                RetirementProgress::Reaped => break,
                RetirementProgress::Pending { .. } => {
                    assert!(started.elapsed() < Duration::from_secs(5));
                    std::thread::yield_now();
                }
            }
        }
        assert_eq!(fixture.owner.state(), GpuOwnerState::Reaped);
        assert!(!retiring.reaped, "only the outer owner observed completion");
        assert!(retiring
            .poll_with_presenter(&mut presenter, &mut fixture.owner)
            .unwrap());
        assert!(retiring.reaped);
        assert!(retiring
            .poll_with_presenter(&mut presenter, &mut fixture.owner)
            .unwrap());
        assert!(previous_present.upgrade().is_none());
        assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
        assert_eq!(fixture.owner.budget_snapshot().reservation_count, 0);
    }
}

#[test]
fn i4_shared_presenter_wrong_owner_operations_preserve_correct_generation() {
    let mut fixture = gpu();
    let mut foreign = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let (mut presenter, probe) = present_fixture(&mut fixture);
    let mut cache = SharedNativeOutputCache::new(101);
    let delivery = present_two_with_retiring_second(
        &mut fixture,
        &mut reference,
        &compositor,
        &mut presenter,
        &mut cache,
    );
    assert_ne!(fixture.owner.context().id(), foreign.owner.context().id());
    let correct_budget = fixture.owner.budget_snapshot();
    let foreign_budget = foreign.owner.budget_snapshot();
    assert_eq!(
        presenter
            .poll_retirements(&mut foreign.owner)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch,
    );
    assert_eq!(
        presenter
            .drain_retirements(&mut foreign.owner)
            .unwrap_err()
            .kind,
        NativeGpuFaultKind::IdentityMismatch,
    );
    match release_shared_presenter_for_stop(&mut presenter, &mut foreign.owner) {
        Err(error) => assert_eq!(error.kind, NativeGpuFaultKind::IdentityMismatch),
        Ok(_) => panic!("a foreign broker must not release or stop this presenter"),
    }
    assert_eq!(fixture.owner.state(), GpuOwnerState::Active);
    assert_eq!(foreign.owner.state(), GpuOwnerState::Active);
    assert_eq!(fixture.owner.budget_snapshot(), correct_budget);
    assert_eq!(foreign.owner.budget_snapshot(), foreign_budget);
    assert_eq!(delivery.delivered(), 2);
    assert_eq!(presenter.presents(), 2);
    assert_eq!(probe.borrow().acquires, 2);
    assert_eq!(probe.borrow().discards, 0);
    assert_eq!(probe.borrow().presented.len(), 2);
    assert_present_retirement_allows_next_frame(&mut fixture.owner, frame_id(3));
    presenter.drain_retirements(&mut fixture.owner).unwrap();
    cache.blackout().unwrap();
    release_shared_presenter_for_stop(&mut presenter, &mut fixture.owner).unwrap();
    drop(fixture.runtime);
    reap(fixture.owner);
    drop(foreign.runtime);
    reap(foreign.owner);
}

#[test]
fn i4_shared_sink_submitted_snapshot_cancel_releases_presenter_before_stop_and_reaps() {
    let mut fixture = gpu();
    let mut reference = IsfGpuRuntime::new().unwrap();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    let (mut presenter, probe) = present_fixture(&mut fixture);
    let mut cache = SharedNativeOutputCache::new(101);
    let stopped = Cell::new(false);
    let mapping = VideoOutputMapping::default();
    let ready = ready_for_present(
        &mut fixture,
        &mut reference,
        &compositor,
        &mut presenter,
        &mut cache,
        1,
        mapping.clone(),
    );
    let key = ready.cache_receipt().descriptor().key.clone();
    let authority = PresentAuthority {
        stopped: &stopped,
        mapping: VideoOutputMappingIdentity::from_mapping(&mapping),
        rejection: None,
        boundaries: RefCell::new(Vec::new()),
    };
    let mut delivery = NativeDeliveryCounter::default();
    presented(present_actual(
        ready,
        &mut presenter,
        &mut fixture.owner,
        &authority,
        &mut delivery,
    ));
    let request = output_request(&fixture, &mut cache, 2);
    let (inputs, plan, _) = prepare(&fixture, &mut reference, &request, None);
    let (admitted, reservation) = admit(&mut fixture, inputs, &plan, &[]);
    let pending = PendingSharedNativeOutput::submit_after_present(
        &mut fixture.owner,
        &mut fixture.runtime,
        &compositor,
        &mut presenter,
        request,
        admitted,
        reservation,
        &plan,
        &[],
        &authority,
        &|| stopped.get(),
    )
    .unwrap_or_else(|failure| panic!("next snapshot submit failed: {:?}", failure.reason()));
    assert!(
        pending.snapshot.is_some(),
        "P owns a real submitted readback/map"
    );
    stopped.set(true);
    let cancelled = Authority {
        stopped: &stopped,
        cancel_after_commit: false,
    };
    let mut retiring = match finish(pending, &mut fixture, &mut cache, &cancelled) {
        SharedNativeOutputProgress::Retiring(retiring) => retiring,
        _ => panic!("submitted snapshot cancellation escaped retirement"),
    };
    assert_eq!(retiring.reason(), Some(&OutputRejection::Cancelled));
    assert!(retiring.snapshot.is_some());
    assert_eq!(
        fixture.owner.state(),
        GpuOwnerState::Active,
        "pending retirement leaves Stop ordering to the presenter-aware owner",
    );
    assert_eq!(delivery.delivered(), 1);
    assert_eq!(probe.borrow().presented.len(), 1);
    assert!(
        cache.last_valid(&key).is_some(),
        "the previous LastValid remains valid"
    );
    cache.blackout().unwrap();
    drop(fixture.runtime);
    let started = Instant::now();
    while !retiring
        .poll_with_presenter(&mut presenter, &mut fixture.owner)
        .unwrap()
    {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    assert_eq!(fixture.owner.state(), GpuOwnerState::Reaped);
    assert_eq!(fixture.owner.budget_snapshot().managed_bytes(), 0);
}

#[path = "native_gpu_output_history_tests.rs"]
mod history_tests;
