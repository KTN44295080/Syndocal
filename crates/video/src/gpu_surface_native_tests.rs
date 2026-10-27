//! Real-adapter headless evidence for the shared-context presenter: G's real
//! owner, ISF Batch3, P's composition/snapshot and a committed premapping
//! source. The offscreen target stands in for the swapchain image (the same
//! mapping pass and blit run on the shared device); it is not native-window,
//! scanout or timing evidence.
use super::*;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};

use protocol::{
    Transform2D, VideoBlendMode, VideoMaskPoint, VideoOutputAspectMode, VideoOutputKind,
};

use crate::isf_native_gpu::await_transaction;
use crate::isf_native_gpu::tests::{gpu, Fixture};
use crate::native_gpu_composition::tests::{
    identity_batch, layer, submit_shared_frame, validate_isf,
};
use crate::native_gpu_composition::NativeGpuCompositor;
use crate::native_gpu_context::RetirementProgress;
use crate::native_gpu_contract::{
    CacheCommitDescriptor, CachePublicationFence, CpuSnapshotCommit, SnapshotCommitRejection,
};
use crate::native_gpu_output::{NativeOutputRequest, OutputBoundary};
use crate::native_gpu_snapshot::{NativeGpuSnapshotter, SnapshotProgress};
use crate::{apply_video_output_mapping, VideoFrame};

const GENERATION: u64 = 1;

struct Env {
    fixture: Fixture,
    compositor: NativeGpuCompositor,
}

fn shared_env() -> Env {
    let mut fixture = gpu();
    let compositor = NativeGpuCompositor::from_shared(&mut fixture.owner).unwrap();
    Env {
        fixture,
        compositor,
    }
}

/// The CPU LastValid stand-in: it receives the moved packed snapshot.
struct TestCache {
    expected: CacheCommitDescriptor,
    image: Option<VideoFrame>,
}

impl CpuSnapshotCommit for TestCache {
    fn commit(
        &mut self,
        key: &crate::VideoOutputLastValidKey,
        frame: NativeFrameId,
        snapshot: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
        assert_eq!(key, &self.expected.key);
        assert_eq!(frame, self.expected.snapshot.frame);
        assert_eq!(fence, self.expected.fence);
        self.image = Some(snapshot);
        Ok(CacheCommitReceipt::for_test(self.expected.clone()))
    }

    fn invalidate_output(&mut self, _output: u64) {
        self.image = None;
    }
}

/// I's Ready stand-in: P's committed premapping owner, the cache receipt
/// naming it, and the frozen mapping. Drop is observable.
struct TestReady {
    receipt: CacheCommitReceipt,
    gpu: NativeSnapshotGpu,
    mapping: VideoOutputMapping,
    dropped: Arc<AtomicBool>,
}

impl Drop for TestReady {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl FrozenPresentation for TestReady {
    fn cache_receipt(&self) -> &CacheCommitReceipt {
        &self.receipt
    }
    fn snapshot_gpu(&self) -> &NativeSnapshotGpu {
        &self.gpu
    }
    fn mapping(&self) -> &VideoOutputMapping {
        &self.mapping
    }
}

/// One real frame exactly as INTERFACE-01 orders it, up to the cache commit:
/// ISF submit, P composition submit, map, copy/unmap, move into the cache.
/// Returns the Ready stand-in, the committed premapping bytes, a drop flag.
fn real_ready(
    env: &mut Env,
    sequence: u64,
    size: (u32, u32),
    mapping: VideoOutputMapping,
) -> (TestReady, VideoFrame, Arc<AtomicBool>) {
    let plans = [
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
    ];
    let seed = sequence as u8;
    let batch = identity_batch(size, 3, move |layer, index| {
        let value = (index as u8)
            .wrapping_mul(29)
            .wrapping_add(layer as u8 * 71)
            .wrapping_add(seed.wrapping_mul(13));
        [value, value.wrapping_mul(3), 255 - value, 255]
    });
    let shared = submit_shared_frame(
        &mut env.fixture,
        &env.compositor,
        sequence,
        batch,
        &[],
        &plans,
        false,
    );
    let frame = shared.frame;
    let key = crate::VideoOutputLastValidKey {
        project_render_epoch: frame.project_epoch,
        output_id: frame.output_id,
        width: size.0,
        height: size.1,
        composition_id: 1,
        kind: VideoOutputKind::Display,
        fullscreen: true,
        monitor_id: Some(1),
        endpoint_name: None,
    };
    let fence = CachePublicationFence {
        request_sequence: frame.request_sequence,
        blackout_generation: 4,
        configuration_generation: 5,
    };
    let encoded = match shared.encoded {
        Ok(encoded) => encoded,
        Err(fault) => panic!("shared composition failed: {fault:?}"),
    };
    let mut snapshot = match NativeGpuSnapshotter::begin_shared(encoded, key, fence, 1_000, 33) {
        Ok(snapshot) => snapshot,
        Err(failure) => panic!("snapshot begin rejected: {:?}", failure.fault),
    };
    let mut resolved =
        await_transaction(&mut env.fixture.owner, shared.pending, &|| false).unwrap();
    let layers = validate_isf(&mut env.fixture, shared.isf, &mut resolved);
    snapshot.resolve_shared_phase(&mut resolved).unwrap();
    let current = snapshot.descriptor().clone();
    let started = Instant::now();
    let completed = loop {
        match NativeGpuSnapshotter::poll_shared(
            &mut snapshot,
            &mut env.fixture.owner,
            &current,
            &|| false,
        ) {
            SnapshotProgress::Pending => {
                assert!(started.elapsed() < Duration::from_secs(5));
                std::thread::yield_now();
            }
            SnapshotProgress::Complete(completed) => break completed,
            _ => panic!("real snapshot did not complete"),
        }
    };
    let pixels = completed.pixels().clone();
    let mut cache = TestCache {
        expected: current.clone(),
        image: None,
    };
    let bound = match completed.commit(&mut cache, &current, &|| false) {
        Ok(bound) => bound,
        Err(failure) => panic!("cache commit rejected: {:?}", failure.reason),
    };
    assert_eq!(
        cache.image.as_ref().map(|image| &image.data),
        Some(&pixels.data)
    );
    let (gpu, receipt) = bound.into_parts();
    drop(layers);
    drop(resolved);
    drop(snapshot);
    let dropped = Arc::new(AtomicBool::new(false));
    (
        TestReady {
            receipt,
            gpu,
            mapping,
            dropped: Arc::clone(&dropped),
        },
        pixels,
        dropped,
    )
}

/// Offscreen stand-in for the swapchain image: scripted acquire failures,
/// counted configure/acquire/discard/present, readable presented images.
struct OffscreenTarget {
    device: wgpu::Device,
    format: wgpu::TextureFormat,
    image_format: wgpu::TextureFormat,
    extent: Option<(u32, u32)>,
    script: VecDeque<TargetAcquireError>,
    configures: Vec<(u32, u32)>,
    acquires: usize,
    discards: usize,
    presented: Vec<wgpu::Texture>,
}

impl OffscreenTarget {
    fn new(env: &Env) -> Self {
        Self::with_image_format(env, wgpu::TextureFormat::Rgba8Unorm)
    }

    /// A different image format than the pipeline's injects a real
    /// validation error into the present phase.
    fn with_image_format(env: &Env, image_format: wgpu::TextureFormat) -> Self {
        Self::with_formats(env, wgpu::TextureFormat::Rgba8Unorm, image_format)
    }

    /// `format` is the pipeline's target format (the surface configuration);
    /// `image_format` is what acquire hands out.
    fn with_formats(
        env: &Env,
        format: wgpu::TextureFormat,
        image_format: wgpu::TextureFormat,
    ) -> Self {
        Self {
            device: env.fixture.owner.context().device().clone(),
            format,
            image_format,
            extent: None,
            script: VecDeque::new(),
            configures: Vec::new(),
            acquires: 0,
            discards: 0,
            presented: Vec::new(),
        }
    }
}

impl NativePresentTarget for OffscreenTarget {
    type Frame = wgpu::Texture;

    fn format(&self) -> wgpu::TextureFormat {
        self.format
    }

    fn configure(&mut self, _device: &wgpu::Device, extent: (u32, u32)) {
        self.extent = Some(extent);
        self.configures.push(extent);
    }

    fn acquire(&mut self) -> Result<Self::Frame, TargetAcquireError> {
        self.acquires += 1;
        if let Some(error) = self.script.pop_front() {
            return Err(error);
        }
        let (width, height) = self.extent.expect("acquire only after configure");
        Ok(self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Syndocal offscreen swapchain stand-in"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: self.image_format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        }))
    }

    fn view(&self, frame: &Self::Frame) -> wgpu::TextureView {
        frame.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn present(&mut self, frame: Self::Frame) {
        self.presented.push(frame);
    }

    fn discard(&mut self, frame: Self::Frame) {
        self.discards += 1;
        drop(frame);
    }
}

/// I's permit stand-in: scripted rejections at the two physical boundaries
/// and an optional hook that runs inside the pre-present recheck.
#[derive(Default)]
struct ScriptedGate {
    acquire: Option<PresentRejection>,
    present: Option<PresentRejection>,
    before_present_hook: Option<Box<dyn Fn()>>,
    events: RefCell<Vec<&'static str>>,
}

impl<R> NativePresentGate<R> for ScriptedGate {
    fn before_acquire(&self, _ready: &R) -> Result<(), PresentRejection> {
        self.events.borrow_mut().push("before_acquire");
        self.acquire.map_or(Ok(()), Err)
    }

    fn before_present(&self, _ready: &R) -> Result<(), PresentRejection> {
        self.events.borrow_mut().push("before_present");
        if let Some(hook) = &self.before_present_hook {
            hook();
        }
        self.present.map_or(Ok(()), Err)
    }
}

type Presented = Result<PresentOutcome<TestReady, NativePresentReceipt>, NativeGpuFault>;

fn expect_presented(result: Presented) -> NativePresentReceipt {
    match result {
        Ok(PresentOutcome::Presented(receipt)) => receipt,
        Ok(PresentOutcome::NotPresented { reason, .. }) => panic!("not presented: {reason:?}"),
        Err(fault) => panic!("present failed: {fault:?}"),
    }
}

fn expect_not_presented(result: Presented, expected: PresentRejection) -> TestReady {
    match result {
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: Some(ready),
        }) => {
            assert_eq!(reason, expected);
            ready
        }
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: None,
        }) => {
            panic!("Ready was not returned with {reason:?}")
        }
        Ok(PresentOutcome::Presented(_)) => panic!("unexpected present; wanted {expected:?}"),
        Err(fault) => panic!("unexpected fault {fault:?}; wanted {expected:?}"),
    }
}

fn expect_fault(result: Presented, kind: NativeGpuFaultKind) -> NativeGpuFault {
    match result {
        Err(fault) => {
            assert_eq!(fault.kind, kind, "{fault:?}");
            assert_eq!(fault.phase, GpuPhase::MappingPresent);
            fault
        }
        Ok(PresentOutcome::Presented(_)) => panic!("unexpected present; wanted {kind:?}"),
        Ok(PresentOutcome::NotPresented { reason, .. }) => {
            panic!("unexpected {reason:?}; wanted fault {kind:?}")
        }
    }
}

fn geometry(extent: (u32, u32)) -> AdmittedPhysicalGeometry {
    AdmittedPhysicalGeometry {
        extent,
        geometry_generation: GENERATION,
    }
}

fn new_presenter(
    env: &mut Env,
    target: OffscreenTarget,
) -> SharedSurfacePresenter<OffscreenTarget> {
    SharedSurfacePresenter::new(target, &mut env.fixture.owner).unwrap()
}

/// Test oracle readback of a presented stand-in image, after every G
/// transaction has resolved; it is not part of the presenter path.
fn read_image(env: &Env, texture: &wgpu::Texture) -> Vec<u8> {
    let device = env.fixture.owner.context().device();
    let queue = env.fixture.owner.context().queue();
    let (width, height) = (texture.width(), texture.height());
    let row = width * 4;
    let padded = row.div_ceil(256) * 256;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("Syndocal presented stand-in readback"),
        size: u64::from(padded) * u64::from(height),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit(Some(encoder.finish()));
    let (sender, receiver) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });
    device.poll(wgpu::PollType::wait()).unwrap();
    receiver.recv().unwrap().unwrap();
    let view = buffer.slice(..).get_mapped_range();
    let mut pixels = Vec::with_capacity((row * height) as usize);
    for y in 0..height as usize {
        let start = y * padded as usize;
        pixels.extend_from_slice(&view[start..start + row as usize]);
    }
    drop(view);
    buffer.unmap();
    pixels
}

fn wait_for_allocations(owner: &mut NativeGpuOwner, expected: usize) {
    // Measure only the explicitly retained surface/Ready owners; idle ISF
    // resources are independently released through the production bank fence.
    owner.release_resource_pools();
    let started = Instant::now();
    loop {
        match owner.poll_retirement().unwrap() {
            RetirementProgress::Pending {
                transactions: 0,
                allocations,
            } if allocations == expected => return,
            progress => assert!(
                started.elapsed() < Duration::from_secs(5),
                "allocations were not reaped to {expected}: {progress:?}"
            ),
        }
        std::thread::yield_now();
    }
}

fn stop_and_reap<T: NativePresentTarget>(
    presenter: &mut SharedSurfacePresenter<T>,
    owner: &mut NativeGpuOwner,
) {
    presenter.release_for_stop();
    owner.begin_stop();
    let started = Instant::now();
    loop {
        match owner.poll_retirement().unwrap() {
            RetirementProgress::Reaped => break,
            progress => assert!(
                started.elapsed() < Duration::from_secs(5),
                "owner was not reaped: {progress:?}"
            ),
        }
        std::thread::yield_now();
    }
    assert_eq!(owner.budget_snapshot().managed_bytes(), 0);
}

fn projection() -> VideoOutputMapping {
    let mut bitmap_mask_luma_words = [0_u32; 32];
    for index in 0..256 {
        let x = index % 16;
        let y = index / 16;
        let nibble = ((x + y) % 16) as u32;
        bitmap_mask_luma_words[index / 8] |= nibble << ((index % 8) * 4);
    }
    VideoOutputMapping {
        offset_x: 0.04,
        offset_y: -0.03,
        scale_x: 0.92,
        scale_y: 0.88,
        rotation_deg: 4.0,
        aspect_ratio: 4.0 / 3.0,
        aspect_mode: VideoOutputAspectMode::Fit,
        lens_distortion: 0.12,
        edge_blend_left: 0.18,
        edge_blend_right: 0.12,
        edge_blend_top: 0.08,
        edge_blend_bottom: 0.16,
        edge_blend_gamma: 2.0,
        black_level: 0.04,
        mask_point_count: 4,
        mask_softness: 0.04,
        mask_points: [
            VideoMaskPoint { x: 0.08, y: 0.12 },
            VideoMaskPoint { x: 0.92, y: 0.08 },
            VideoMaskPoint { x: 0.85, y: 0.9 },
            VideoMaskPoint { x: 0.15, y: 0.86 },
            VideoMaskPoint { x: 0.0, y: 0.0 },
            VideoMaskPoint { x: 0.0, y: 0.0 },
            VideoMaskPoint { x: 0.0, y: 0.0 },
            VideoMaskPoint { x: 0.0, y: 0.0 },
        ],
        bitmap_mask_width: 16,
        bitmap_mask_height: 16,
        bitmap_mask_luma_words,
        keystone_x: 0.06,
        keystone_y: -0.04,
        corner_top_left_x: -0.02,
        corner_top_left_y: 0.01,
        corner_top_right_x: 0.03,
        corner_top_right_y: -0.02,
        corner_bottom_right_x: 0.02,
        corner_bottom_right_y: 0.03,
        corner_bottom_left_x: -0.01,
        corner_bottom_left_y: -0.03,
        ..VideoOutputMapping::default()
    }
}

fn fit_wide() -> VideoOutputMapping {
    VideoOutputMapping {
        aspect_ratio: 16.0 / 9.0,
        aspect_mode: VideoOutputAspectMode::Fit,
        rotation_deg: -7.0,
        ..VideoOutputMapping::default()
    }
}

struct AllowAll;

impl NativeOutputAuthority for AllowAll {
    fn revalidate(
        &self,
        _request: &NativeOutputRequest,
        _boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        Ok(())
    }
}

#[test]
fn shared_present_contract_types_join_i_present_policy() {
    fn assert_send<T: Send + 'static>() {}
    assert_send::<SharedSurfacePresenter<NativeSurfaceTarget>>();
    assert_send::<SharedSurfacePresenter<OffscreenTarget>>();
    fn assert_gate<A: NativeOutputAuthority, G>()
    where
        for<'a> NativePresentPermit<'a, A>: NativePresentGate<ReadyNativeGpuOutput<G>>,
    {
    }
    assert_gate::<AllowAll, NativeSnapshotGpu>();
    fn assert_sink<S>()
    where
        S: NativePresentSink<
            Ready = TestReady,
            Permit = ScriptedGate,
            Receipt = NativePresentReceipt,
        >,
    {
    }
    assert_sink::<SharedPresentSink<'static, OffscreenTarget, TestReady, ScriptedGate>>();
}

#[test]
fn shared_present_maps_committed_premapping_once_and_matches_cpu_oracle() {
    let mut env = shared_env();
    eprintln!(
        "REAL_ADAPTER shared present: {:?}",
        env.fixture.owner.capabilities().adapter_info
    );
    let size = (41, 29);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    let (ready, pixels, dropped) = real_ready(&mut env, 1, size, projection());
    let snapshot = ready.receipt.descriptor().snapshot;

    // Never configured: Suspended, zero acquires, the Ready comes back intact.
    let gate = ScriptedGate::default();
    let ready = expect_not_presented(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        PresentRejection::Suspended,
    );
    assert_eq!(presenter.target.acquires, 0);
    assert!(
        gate.events.borrow().is_empty(),
        "no physical boundary was reached"
    );

    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    assert_eq!(presenter.configured_extent(), Some(size));
    assert_eq!(presenter.target.configures, vec![size]);
    let receipt = {
        let mut sink = presenter.sink::<TestReady, ScriptedGate>(&mut env.fixture.owner);
        expect_presented(sink.present_once(ready, ScriptedGate::default()))
    };
    assert_eq!(
        (receipt.sequence(), receipt.extent(), receipt.frame()),
        (1, size, snapshot.frame)
    );
    assert_eq!(receipt.snapshot(), Some(snapshot));
    assert_eq!(receipt.content(), NativePresentContent::Resident(snapshot));
    assert_eq!(
        receipt.submission().context,
        env.fixture.owner.context().id()
    );
    assert!(
        receipt.submission().sequence > snapshot.completed_submission.sequence,
        "present is its own later submission, after the snapshot/cache commit"
    );
    assert_eq!(presenter.presents(), 1);
    assert_eq!(
        (presenter.target.acquires, presenter.target.discards),
        (1, 0)
    );
    // The consumed Ready is retained with its submission, then released.
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    assert!(dropped.load(Ordering::SeqCst));

    let mapped = apply_video_output_mapping(pixels.clone(), &projection());
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(
        image, mapped.data,
        "one output mapping over the committed premapping bytes, exactly"
    );
    assert_ne!(image, pixels.data, "the mapping is visible");
    let twice = apply_video_output_mapping(mapped.clone(), &projection());
    assert_ne!(image, twice.data, "the mapping is applied exactly once");

    // Pooled: mapped buffer + dimensions, params + mask. Staging, the Ready's
    // premapping and CPU charge are gone once the submission completed.
    wait_for_allocations(&mut env.fixture.owner, 4);
    let pooled = env
        .fixture
        .owner
        .budget_snapshot()
        .for_kind(ChargeKind::MappingBuffer)
        .active_bytes;
    let packed = u64::from(size.0 * size.1 * 4);
    assert_eq!(
        pooled,
        packed + DIMENSIONS_BYTES + MAPPING_PARAMS_BYTES + MASK_TEXTURE_BYTES
    );

    // Steady state, same mapping: no new present allocation.
    let (ready, pixels, _) = real_ready(&mut env, 2, size, projection());
    let gate = ScriptedGate::default();
    let receipt = expect_presented(presenter.present_ready(ready, &gate, &mut env.fixture.owner));
    assert_eq!(receipt.sequence(), 2);
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    // Steady state presents without a host wait: the submission is still the
    // presenter's. G3: that retiring present does not block the next logical
    // frame, but a second present is refused while it retires (before any
    // scope, allocation or write of the next mapping phase).
    assert_eq!(presenter.retirements.len(), 1);
    assert!(presenter.retirements[0].pending.is_some());
    let next_frame = NativeFrameId {
        request_sequence: receipt.frame().request_sequence + 1,
        ..receipt.frame()
    };
    let next = NativeAllocationPlan::new(next_frame);
    let quote = env.fixture.owner.quote(&next).unwrap();
    let reservation = env.fixture.owner.reserve(quote).unwrap();
    let mut txn = match env.fixture.owner.begin_frame(next_frame, reservation) {
        Ok(txn) => txn,
        Err(fault) => panic!("a retiring present blocked the next logical frame: {fault:?}"),
    };
    match txn.begin_phase(GpuPhase::MappingPresent) {
        Err(fault) => assert_eq!(fault.kind, NativeGpuFaultKind::StateMismatch),
        Ok(_) => panic!("G admitted a second present while one was retiring"),
    }
    let pending = txn.finish();
    assert!(
        presenter.retirements[0].pending.is_some(),
        "admitting the next logical frame needed no present drain"
    );
    drop(await_transaction(&mut env.fixture.owner, pending, &|| false).unwrap());
    presenter.drain_retirements(&mut env.fixture.owner).unwrap();
    assert!(presenter.retirements.is_empty());
    wait_for_allocations(&mut env.fixture.owner, 4);
    let image = read_image(&env, &presenter.target.presented[1]);
    assert_eq!(
        image,
        apply_video_output_mapping(pixels, &projection()).data
    );

    // A different frozen mapping replaces only the pooled mapping inputs.
    let (ready, pixels, _) = real_ready(&mut env, 3, size, fit_wide());
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    let started = Instant::now();
    while presenter.poll_retirements(&mut env.fixture.owner).unwrap() != 0 {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    wait_for_allocations(&mut env.fixture.owner, 4);
    let image = read_image(&env, &presenter.target.presented[2]);
    assert_eq!(image, apply_video_output_mapping(pixels, &fit_wide()).data);
    assert_eq!(presenter.target.configures, vec![size], "no reconfigure");
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_present_resize_rejects_old_geometry_and_reconfigures_once() {
    let mut env = shared_env();
    let first = (41, 29);
    let second = (33, 21);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(first), &mut env.fixture.owner)
        .unwrap();
    let (ready, _, _) = real_ready(&mut env, 1, first, projection());
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    let (stale, _, stale_dropped) = real_ready(&mut env, 2, first, projection());

    presenter
        .configure_admitted(geometry(second), &mut env.fixture.owner)
        .unwrap();
    assert_eq!(presenter.target.configures, vec![first, second]);
    let gate = ScriptedGate::default();
    let stale = expect_not_presented(
        presenter.present_ready(stale, &gate, &mut env.fixture.owner),
        PresentRejection::StaleGeometry,
    );
    assert!(gate.events.borrow().is_empty());
    assert_eq!(presenter.target.acquires, 1);

    // A new geometry generation at the same extent: no reconfigure, but a
    // Ready of the old generation is not proof for it.
    presenter
        .configure_admitted(
            AdmittedPhysicalGeometry {
                extent: second,
                geometry_generation: GENERATION + 1,
            },
            &mut env.fixture.owner,
        )
        .unwrap();
    assert_eq!(presenter.target.configures.len(), 2);
    let (ready, pixels, _) = real_ready(&mut env, 3, second, projection());
    let ready = expect_not_presented(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::StaleGeometry,
    );
    presenter
        .configure_admitted(geometry(second), &mut env.fixture.owner)
        .unwrap();
    assert_eq!(presenter.target.configures.len(), 2);
    let receipt = expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert_eq!(receipt.extent(), second);
    assert_eq!(presenter.target.acquires, 2);
    let started = Instant::now();
    while presenter.poll_retirements(&mut env.fixture.owner).unwrap() != 0 {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    let image = read_image(&env, &presenter.target.presented[1]);
    assert_eq!(
        image,
        apply_video_output_mapping(pixels, &projection()).data
    );
    // The old-extent mapped target was released after its last user.
    drop(stale);
    assert!(stale_dropped.load(Ordering::SeqCst));
    wait_for_allocations(&mut env.fixture.owner, 4);
    let pooled = env
        .fixture
        .owner
        .budget_snapshot()
        .for_kind(ChargeKind::MappingBuffer)
        .active_bytes;
    assert_eq!(
        pooled,
        u64::from(second.0 * second.1 * 4)
            + DIMENSIONS_BYTES
            + MAPPING_PARAMS_BYTES
            + MASK_TEXTURE_BYTES
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_present_surface_acquire_failures_never_present_and_keep_ready() {
    let mut env = shared_env();
    let size = (29, 19);
    let mut target = OffscreenTarget::new(&env);
    target.script.extend([
        TargetAcquireError::Timeout,
        TargetAcquireError::Outdated,
        TargetAcquireError::Lost,
    ]);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, pixels, dropped) = real_ready(&mut env, 1, size, projection());
    let ready = expect_not_presented(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::SurfaceTimeout,
    );
    let budget = env.fixture.owner.budget_snapshot();
    assert_eq!(
        budget.reservation_count, 0,
        "the present reservation was returned"
    );
    assert_eq!(presenter.target.configures.len(), 1);
    // Lost/Outdated reconfigure the same admitted extent; the Ready comes
    // back for I's single retry allowance.
    let ready = expect_not_presented(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::SurfaceLostOrOutdated,
    );
    let ready = expect_not_presented(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::SurfaceLostOrOutdated,
    );
    assert_eq!(presenter.target.configures, vec![size, size, size]);
    assert_eq!(presenter.presents(), 0);
    assert!(!dropped.load(Ordering::SeqCst));
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert_eq!(
        (presenter.target.acquires, presenter.target.discards),
        (4, 0)
    );
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(
        image,
        apply_video_output_mapping(pixels, &projection()).data
    );

    // Other/OutOfMemory are faults: no present, the consumed Ready stays
    // with the presenter until its next poll.
    for (error, kind) in [
        (TargetAcquireError::Other, NativeGpuFaultKind::Submission),
        (
            TargetAcquireError::OutOfMemory,
            NativeGpuFaultKind::OutOfMemory,
        ),
    ] {
        presenter.target.script.push_back(error);
        let (ready, _, dropped) = real_ready(&mut env, 2, size, projection());
        expect_fault(
            presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
            kind,
        );
        assert!(
            !dropped.load(Ordering::SeqCst),
            "Ready is retained, not dropped"
        );
        assert_eq!(
            presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
            0
        );
        assert!(dropped.load(Ordering::SeqCst));
    }
    assert_eq!(presenter.presents(), 1);
    assert_eq!(presenter.target.presented.len(), 1);
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_present_stop_or_revocation_at_each_boundary_yields_zero_presents() {
    let mut env = shared_env();
    let size = (23, 17);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, _, _) = real_ready(&mut env, 1, size, projection());

    // Stop/revocation between the snapshot/cache commit and acquire.
    for reason in [PresentRejection::Cancelled, PresentRejection::Revoked] {
        let gate = ScriptedGate {
            acquire: Some(reason),
            ..Default::default()
        };
        let returned = expect_not_presented(
            presenter.present_ready(ready_ref(&mut env, size), &gate, &mut env.fixture.owner),
            reason,
        );
        drop(returned);
        assert_eq!(*gate.events.borrow(), vec!["before_acquire"]);
        assert_eq!(
            presenter.target.acquires, 0,
            "no acquire after a revoked permit"
        );
        assert_eq!(env.fixture.owner.budget_snapshot().reservation_count, 0);
    }

    // Validate first use, so the next present leaves a real pending submission.
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );

    // Stop between acquire and present: the mapping submission stays owned by
    // the presenter/G registry; the image is discarded, never presented.
    let (ready, _, dropped) = real_ready(&mut env, 3, size, projection());
    let gate = ScriptedGate {
        present: Some(PresentRejection::Cancelled),
        ..Default::default()
    };
    let ready = expect_not_presented(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        PresentRejection::Cancelled,
    );
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents()
        ),
        (2, 1, 1)
    );
    assert_eq!(presenter.target.presented.len(), 1);
    assert_eq!(presenter.retirements.len(), 1, "submitted owners retained");
    drop(ready);
    assert!(dropped.load(Ordering::SeqCst));
    let started = Instant::now();
    while presenter.poll_retirements(&mut env.fixture.owner).unwrap() != 0 {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    // G released the Ready's premapping only after the submission completed.
    wait_for_allocations(&mut env.fixture.owner, 4);

    // Revocation (not Stop) before present is the same zero-present path.
    let (ready, _, _) = real_ready(&mut env, 4, size, projection());
    let gate = ScriptedGate {
        present: Some(PresentRejection::Revoked),
        ..Default::default()
    };
    let ready = expect_not_presented(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        PresentRejection::Revoked,
    );
    assert_eq!(presenter.presents(), 1);

    // After Stop the owner is not Active: zero acquires, Ready returned.
    presenter.release_for_stop();
    env.fixture.owner.begin_stop();
    let acquires = presenter.target.acquires;
    let ready = expect_not_presented(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::Cancelled,
    );
    assert_eq!(presenter.target.acquires, acquires);
    assert_eq!(presenter.presents(), 1);
    drop(ready);
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

/// A fresh committed Ready for boundary checks that never reach the GPU.
fn ready_ref(env: &mut Env, size: (u32, u32)) -> TestReady {
    real_ready(env, 2, size, projection()).0
}

#[test]
fn shared_present_device_loss_never_presents_and_target_rearms_on_new_generation() {
    // Loss observed before the physical boundary.
    let mut env = shared_env();
    let size = (19, 13);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, _, dropped) = real_ready(&mut env, 1, size, projection());
    env.fixture.owner.context().device().destroy();
    let started = Instant::now();
    while env.fixture.owner.state() != GpuOwnerState::DeviceLostRetained {
        let _ = env.fixture.owner.poll_retirement();
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "loss not observed"
        );
        std::thread::yield_now();
    }
    let gate = ScriptedGate::default();
    expect_fault(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        NativeGpuFaultKind::DeviceLost,
    );
    assert!(gate.events.borrow().is_empty());
    assert_eq!((presenter.target.acquires, presenter.presents()), (0, 0));
    assert!(
        !dropped.load(Ordering::SeqCst),
        "owners are retained, not dropped"
    );
    // Re-arm: the same target (surface stand-in) leaves the lost generation
    // and serves a new owner; it is configured again only after admission.
    let mut target = presenter.into_target();
    assert!(dropped.load(Ordering::SeqCst));
    let lost_generation = env.fixture.owner.context().id();
    drop(env);
    let mut env = shared_env();
    assert_ne!(env.fixture.owner.context().id(), lost_generation);
    target.device = env.fixture.owner.context().device().clone();
    let mut presenter = new_presenter(&mut env, target);
    assert_eq!(presenter.configured_extent(), None);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    assert_eq!(presenter.target.configures, vec![size, size]);
    let (ready, pixels, _) = real_ready(&mut env, 1, size, projection());
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(
        image,
        apply_video_output_mapping(pixels, &projection()).data
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
    drop(presenter);
    drop(env);

    // Loss between the present submission and present.
    let mut env = shared_env();
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, _, _) = real_ready(&mut env, 1, size, projection());
    expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    let (ready, _, dropped) = real_ready(&mut env, 2, size, projection());
    let device = env.fixture.owner.context().device().clone();
    let gate = ScriptedGate {
        before_present_hook: Some(Box::new(move || {
            device.destroy();
            let _ = device.poll(wgpu::PollType::wait());
        })),
        ..Default::default()
    };
    expect_fault(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        NativeGpuFaultKind::DeviceLost,
    );
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents()
        ),
        (2, 1, 1)
    );
    assert_eq!(presenter.target.presented.len(), 1);
    assert_eq!(env.fixture.owner.state(), GpuOwnerState::DeviceLostRetained);
    assert!(!dropped.load(Ordering::SeqCst));
    // Loss is the owner's quarantined state, not a sticky present fault.
    assert!(presenter.fault.is_none());
    presenter.release_for_stop();
    assert!(dropped.load(Ordering::SeqCst));
}

#[test]
fn shared_present_rejects_foreign_generation_and_faults_on_first_use_validation() {
    let mut env_a = shared_env();
    let mut env_b = shared_env();
    let size = (17, 11);
    let target = OffscreenTarget::new(&env_a);
    let mut presenter = new_presenter(&mut env_a, target);
    presenter
        .configure_admitted(geometry(size), &mut env_a.fixture.owner)
        .unwrap();
    let (ready, _, dropped) = real_ready(&mut env_a, 1, size, projection());
    let gate = ScriptedGate::default();
    expect_fault(
        presenter.present_ready(ready, &gate, &mut env_b.fixture.owner),
        NativeGpuFaultKind::IdentityMismatch,
    );
    assert!(gate.events.borrow().is_empty());
    assert_eq!(presenter.target.acquires, 0);
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(
        presenter
            .poll_retirements(&mut env_a.fixture.owner)
            .unwrap(),
        0
    );
    assert!(dropped.load(Ordering::SeqCst));
    stop_and_reap(&mut presenter, &mut env_a.fixture.owner);
    crate::isf_native_gpu::tests::stop(&mut env_b.fixture.owner);

    // A real validation error in the first present phase: the image is
    // discarded before present and the presenter stays faulted.
    let mut env = shared_env();
    let target = OffscreenTarget::with_image_format(&env, wgpu::TextureFormat::Bgra8Unorm);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, _, _) = real_ready(&mut env, 1, size, projection());
    expect_fault(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        NativeGpuFaultKind::Validation,
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents()
        ),
        (1, 1, 0)
    );
    assert_eq!(
        env.fixture.owner.state(),
        GpuOwnerState::Active,
        "phase fault only"
    );
    let (ready, _, _) = real_ready(&mut env, 2, size, projection());
    let fault = expect_fault(
        presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner),
        NativeGpuFaultKind::Validation,
    );
    assert!(fault.message.contains("faulted"), "{fault:?}");
    assert_eq!(
        presenter.target.acquires, 1,
        "a faulted presenter never acquires"
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

// P53: CPU Artistic display and payload-less HardBlackout on the same shared
// owner, presenter and target as `present_ready`.

fn display_frame_id(sequence: u64) -> NativeFrameId {
    NativeFrameId {
        output_id: 9,
        project_epoch: 7,
        request_sequence: sequence,
        geometry_generation: GENERATION,
    }
}

/// I's CPU Artistic stand-in: the canonical post-FX frame, its frozen
/// mapping and (in production) its CPU charge owner. Drop is observable.
struct TestArtistic {
    frame_id: NativeFrameId,
    frame: VideoFrame,
    mapping: VideoOutputMapping,
    dropped: Arc<AtomicBool>,
}

impl Drop for TestArtistic {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl CpuArtisticPresentation for TestArtistic {
    fn frame_id(&self) -> NativeFrameId {
        self.frame_id
    }
    fn frame(&self) -> &VideoFrame {
        &self.frame
    }
    fn mapping(&self) -> &VideoOutputMapping {
        &self.mapping
    }
}

/// I's payload-less blackout stand-in: identity only.
struct TestBlackout {
    frame_id: NativeFrameId,
    dropped: Arc<AtomicBool>,
}

impl Drop for TestBlackout {
    fn drop(&mut self) {
        self.dropped.store(true, Ordering::SeqCst);
    }
}

impl HardBlackoutPresentation for TestBlackout {
    fn frame_id(&self) -> NativeFrameId {
        self.frame_id
    }
}

fn blackout(sequence: u64) -> (TestBlackout, Arc<AtomicBool>) {
    let dropped = Arc::new(AtomicBool::new(false));
    (
        TestBlackout {
            frame_id: display_frame_id(sequence),
            dropped: Arc::clone(&dropped),
        },
        dropped,
    )
}

fn artistic_frame((width, height): (u32, u32), seed: u8) -> VideoFrame {
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let value = (x as u8)
                .wrapping_mul(37)
                .wrapping_add((y as u8).wrapping_mul(53))
                .wrapping_add(seed.wrapping_mul(17));
            data.extend_from_slice(&[value, value.wrapping_mul(3), 255 - value, 255]);
        }
    }
    VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms: 1_000,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

fn with_frame(
    sequence: u64,
    frame: VideoFrame,
    mapping: VideoOutputMapping,
) -> (TestArtistic, VideoFrame, Arc<AtomicBool>) {
    let dropped = Arc::new(AtomicBool::new(false));
    (
        TestArtistic {
            frame_id: display_frame_id(sequence),
            frame: frame.clone(),
            mapping,
            dropped: Arc::clone(&dropped),
        },
        frame,
        dropped,
    )
}

fn artistic(
    sequence: u64,
    size: (u32, u32),
    mapping: VideoOutputMapping,
) -> (TestArtistic, VideoFrame, Arc<AtomicBool>) {
    with_frame(sequence, artistic_frame(size, sequence as u8), mapping)
}

type Outcome<R> = Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault>;

fn presented<R>(result: Outcome<R>) -> NativePresentReceipt {
    match result {
        Ok(PresentOutcome::Presented(receipt)) => receipt,
        Ok(PresentOutcome::NotPresented { reason, .. }) => panic!("not presented: {reason:?}"),
        Err(fault) => panic!("present failed: {fault:?}"),
    }
}

fn returned<R>(result: Outcome<R>, expected: PresentRejection) -> R {
    match result {
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: Some(input),
        }) => {
            assert_eq!(reason, expected);
            input
        }
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: None,
        }) => {
            panic!("input was not returned with {reason:?}")
        }
        Ok(PresentOutcome::Presented(_)) => panic!("unexpected present; wanted {expected:?}"),
        Err(fault) => panic!("unexpected fault {fault:?}; wanted {expected:?}"),
    }
}

fn faulted<R>(result: Outcome<R>, kind: NativeGpuFaultKind) -> NativeGpuFault {
    match result {
        Err(fault) => {
            assert_eq!(fault.kind, kind, "{fault:?}");
            assert_eq!(fault.phase, GpuPhase::MappingPresent);
            fault
        }
        Ok(PresentOutcome::Presented(_)) => panic!("unexpected present; wanted {kind:?}"),
        Ok(PresentOutcome::NotPresented { reason, .. }) => {
            panic!("unexpected {reason:?}; wanted fault {kind:?}")
        }
    }
}

fn reap<T: NativePresentTarget>(
    presenter: &mut SharedSurfacePresenter<T>,
    owner: &mut NativeGpuOwner,
) {
    let started = Instant::now();
    while presenter.poll_retirements(owner).unwrap() != 0 {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
}

/// The legacy presenter's own CPU Artistic MapOnce encoding
/// (`NativeGpuFrameBuffers::encode_mapped_artistic`, used by
/// `GpuSurfacePresenter::present_output_artistic_result`) into an offscreen
/// image of `format`: the pixel-parity oracle. Test-only, on the same
/// adapter, outside the presenter path.
fn legacy_artistic_image(
    env: &Env,
    format: wgpu::TextureFormat,
    frame: &VideoFrame,
    mapping: &VideoOutputMapping,
) -> Vec<u8> {
    let device = env.fixture.owner.context().device();
    let queue = env.fixture.owner.context().queue();
    let composite = crate::gpu_compositor::create_gpu_composite_pipelines(device);
    let output_layout = create_buffer_output_bind_group_layout(device);
    let blit = create_buffer_output_pipeline(device, &output_layout, format);
    let buffers = super::super::NativeGpuFrameBuffers::new(
        device,
        &composite.bind_group_layout,
        &output_layout,
        frame.data.len() as u64,
    );
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Syndocal legacy artistic parity image"),
        size: wgpu::Extent3d {
            width: frame.width,
            height: frame.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let encoder = buffers.encode_mapped_artistic(
        device,
        queue,
        (&composite.output_mapping, &blit),
        (frame, mapping),
        (frame.width, frame.height),
        &view,
    );
    queue.submit(Some(encoder.finish()));
    read_image(env, &texture)
}

fn exact_black(image: &[u8]) -> bool {
    image.chunks_exact(4).all(|pixel| pixel == [0, 0, 0, 255])
}

#[test]
fn shared_cpu_artistic_and_blackout_contract_types_join_i_present_policy() {
    fn assert_sink<S, R>()
    where
        S: NativePresentSink<Ready = R, Permit = ScriptedGate, Receipt = NativePresentReceipt>,
    {
    }
    assert_sink::<
        SharedPresentSink<'static, OffscreenTarget, TestArtistic, ScriptedGate, CpuArtisticContent>,
        TestArtistic,
    >();
    assert_sink::<
        SharedPresentSink<
            'static,
            OffscreenTarget,
            TestBlackout,
            ScriptedGate,
            HardBlackoutContent,
        >,
        TestBlackout,
    >();
    // The default content is unchanged for the existing Resident sink.
    assert_sink::<SharedPresentSink<'static, OffscreenTarget, TestReady, ScriptedGate>, TestReady>(
    );
}

#[test]
fn shared_cpu_artistic_uploads_once_maps_once_and_matches_legacy_presenter_pixels() {
    let mut env = shared_env();
    eprintln!(
        "REAL_ADAPTER shared CPU Artistic: {:?}",
        env.fixture.owner.capabilities().adapter_info
    );
    let size = (41, 29);
    let packed = u64::from(size.0 * size.1 * 4);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    let (input, frame, dropped) = artistic(1, size, projection());

    // Never configured: Suspended, zero acquires, the input comes back intact.
    let gate = ScriptedGate::default();
    let input = returned(
        presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner),
        PresentRejection::Suspended,
    );
    assert_eq!(presenter.target.acquires, 0);
    assert!(gate.events.borrow().is_empty());

    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let receipt = {
        let mut sink =
            presenter.cpu_artistic_sink::<TestArtistic, ScriptedGate>(&mut env.fixture.owner);
        presented(sink.present_once(input, ScriptedGate::default()))
    };
    assert_eq!(
        (receipt.sequence(), receipt.extent(), receipt.frame()),
        (1, size, display_frame_id(1))
    );
    assert_eq!(receipt.content(), NativePresentContent::CpuArtistic);
    assert_eq!(receipt.snapshot(), None);
    assert_eq!(
        receipt.submission().context,
        env.fixture.owner.context().id()
    );
    assert_eq!(
        (presenter.presents(), presenter.hard_blackout_presents()),
        (1, 0)
    );
    assert_eq!(
        (presenter.target.acquires, presenter.target.discards),
        (1, 0)
    );
    // The consumed input (its CPU charge owner) stays with the presenter.
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    assert!(dropped.load(Ordering::SeqCst));

    let image = read_image(&env, &presenter.target.presented[0]);
    let mapped = apply_video_output_mapping(frame.clone(), &projection());
    assert_eq!(
        image, mapped.data,
        "one output mapping over the uploaded canonical CPU bytes, exactly"
    );
    assert_ne!(image, frame.data, "the mapping is visible");
    let twice = apply_video_output_mapping(mapped, &projection());
    assert_ne!(image, twice.data, "the mapping is applied exactly once");
    assert_eq!(
        image,
        legacy_artistic_image(&env, wgpu::TextureFormat::Rgba8Unorm, &frame, &projection()),
        "byte parity with the legacy presenter's CPU Artistic arm"
    );

    // Pooled: mapped buffer + dimensions, params + mask, CPU source. The
    // upload staging and the mask staging were released after completion.
    wait_for_allocations(&mut env.fixture.owner, 5);
    let budget = env.fixture.owner.budget_snapshot();
    assert_eq!(
        budget.for_kind(ChargeKind::CompositionBuffer).active_bytes,
        packed
    );
    assert_eq!(
        budget.for_kind(ChargeKind::QueueUploadStaging).active_bytes,
        0
    );
    assert_eq!(
        budget.for_kind(ChargeKind::MappingBuffer).active_bytes,
        packed + DIMENSIONS_BYTES + MAPPING_PARAMS_BYTES + MASK_TEXTURE_BYTES
    );

    // Steady state: no host wait, the submission and the input (with its
    // upload charge) are held until G resolves the present submission.
    let (input, frame, dropped) = artistic(2, size, projection());
    let gate = ScriptedGate::default();
    let receipt = presented(presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner));
    assert_eq!(receipt.sequence(), 2);
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    assert_eq!(presenter.retirements.len(), 1);
    assert!(presenter.retirements[0].pending.is_some());
    assert!(!dropped.load(Ordering::SeqCst), "retained until completion");
    let budget = env.fixture.owner.budget_snapshot();
    assert_eq!(
        budget.for_kind(ChargeKind::QueueUploadStaging).active_bytes,
        packed,
        "the upload charge lives with its submission"
    );
    assert_eq!(
        budget.for_kind(ChargeKind::CompositionBuffer).active_bytes,
        packed,
        "the CPU source is pooled, not reallocated"
    );
    reap(&mut presenter, &mut env.fixture.owner);
    assert!(dropped.load(Ordering::SeqCst));
    wait_for_allocations(&mut env.fixture.owner, 5);
    assert_eq!(
        env.fixture
            .owner
            .budget_snapshot()
            .for_kind(ChargeKind::QueueUploadStaging)
            .active_bytes,
        0
    );
    let image = read_image(&env, &presenter.target.presented[1]);
    assert_eq!(image, apply_video_output_mapping(frame, &projection()).data);
    assert_eq!(presenter.target.configures, vec![size], "no reconfigure");
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_cpu_artistic_on_default_srgb_surface_format_keeps_legacy_byte_parity() {
    let mut env = shared_env();
    let size = (33, 21);
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let target = OffscreenTarget::with_formats(&env, format, format);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (input, frame, _) = artistic(1, size, fit_wide());
    presented(presenter.present_cpu_artistic(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    reap(&mut presenter, &mut env.fixture.owner);
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(
        image,
        legacy_artistic_image(&env, format, &frame, &fit_wide()),
        "same byte/255 blit into the sRGB surface format as the legacy presenter"
    );
    assert_ne!(
        image,
        apply_video_output_mapping(frame, &fit_wide()).data,
        "the sRGB target encodes the blit output, as the legacy presenter does"
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_hard_blackout_clears_exact_black_without_payload_mapping_or_allocation() {
    let mut env = shared_env();
    eprintln!(
        "REAL_ADAPTER shared HardBlackout: {:?}",
        env.fixture.owner.capabilities().adapter_info
    );
    let size = (37, 23);
    // The production default surface format.
    let format = wgpu::TextureFormat::Rgba8UnormSrgb;
    let target = OffscreenTarget::with_formats(&env, format, format);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();

    // A cold blackout: no pooled resource exists or is created.
    let (input, dropped) = blackout(1);
    let receipt = {
        let mut sink =
            presenter.hard_blackout_sink::<TestBlackout, ScriptedGate>(&mut env.fixture.owner);
        presented(sink.present_once(input, ScriptedGate::default()))
    };
    assert_eq!(
        (
            receipt.sequence(),
            receipt.extent(),
            receipt.frame(),
            receipt.content(),
            receipt.snapshot()
        ),
        (
            1,
            size,
            display_frame_id(1),
            NativePresentContent::HardBlackout,
            None
        )
    );
    assert_eq!(
        (presenter.presents(), presenter.hard_blackout_presents()),
        (1, 1)
    );
    assert!(presenter.mapped.is_none() && presenter.inputs.is_none());
    assert!(presenter.cpu_source.is_none());
    assert!(!dropped.load(Ordering::SeqCst));
    reap(&mut presenter, &mut env.fixture.owner);
    assert!(dropped.load(Ordering::SeqCst));
    wait_for_allocations(&mut env.fixture.owner, 0);
    assert_eq!(env.fixture.owner.budget_snapshot().managed_bytes(), 0);
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(image.len(), (size.0 * size.1 * 4) as usize);
    assert!(exact_black(&image), "opaque exact black [0, 0, 0, 255]");

    // A hostile mapping lifts a black CPU frame (the reason for the bypass).
    let mut hostile = projection();
    hostile.black_level = 0.9;
    let (input, _, _) = with_frame(2, crate::video_output_black_frame(size.0, size.1), hostile);
    presented(presenter.present_cpu_artistic(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    reap(&mut presenter, &mut env.fixture.owner);
    assert!(!exact_black(&read_image(
        &env,
        &presenter.target.presented[1]
    )));
    wait_for_allocations(&mut env.fixture.owner, 5);
    let before = env.fixture.owner.budget_snapshot();

    // Blackout after it: same gate boundaries, nothing uploaded, mapped or
    // allocated, and still exact black.
    let (input, _) = blackout(3);
    let gate = ScriptedGate::default();
    let receipt = presented(presenter.present_hard_blackout(input, &gate, &mut env.fixture.owner));
    assert_eq!(receipt.sequence(), 3);
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    let during = env.fixture.owner.budget_snapshot();
    assert_eq!(during.managed_bytes(), before.managed_bytes());
    assert_eq!(
        during.for_kind(ChargeKind::QueueUploadStaging).active_bytes,
        0
    );
    reap(&mut presenter, &mut env.fixture.owner);
    wait_for_allocations(&mut env.fixture.owner, 5);
    assert_eq!(
        env.fixture.owner.budget_snapshot().managed_bytes(),
        before.managed_bytes()
    );
    assert!(exact_black(&read_image(
        &env,
        &presenter.target.presented[2]
    )));
    assert_eq!(
        (presenter.presents(), presenter.hard_blackout_presents()),
        (3, 2)
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_cpu_artistic_and_blackout_stop_or_revocation_before_present_never_present() {
    let mut env = shared_env();
    let size = (23, 17);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    // Validate first use of both contents, so later presents leave a real
    // pending submission.
    let (input, _, _) = artistic(1, size, projection());
    presented(presenter.present_cpu_artistic(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    presented(presenter.present_hard_blackout(
        blackout(2).0,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    reap(&mut presenter, &mut env.fixture.owner);
    assert_eq!(presenter.target.acquires, 2);

    // Boundary 1: Stop/revocation before acquire. Zero acquires, the
    // reservation is returned, the input comes back intact.
    for reason in [PresentRejection::Cancelled, PresentRejection::Revoked] {
        let gate = ScriptedGate {
            acquire: Some(reason),
            ..Default::default()
        };
        let (input, _, dropped) = artistic(3, size, projection());
        let input = returned(
            presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner),
            reason,
        );
        assert!(!dropped.load(Ordering::SeqCst));
        drop(input);
        let gate_blackout = ScriptedGate {
            acquire: Some(reason),
            ..Default::default()
        };
        let input = returned(
            presenter.present_hard_blackout(blackout(4).0, &gate_blackout, &mut env.fixture.owner),
            reason,
        );
        drop(input);
        assert_eq!(*gate.events.borrow(), vec!["before_acquire"]);
        assert_eq!(*gate_blackout.events.borrow(), vec!["before_acquire"]);
        assert_eq!(
            presenter.target.acquires, 2,
            "no acquire after a revoked permit"
        );
        assert_eq!(env.fixture.owner.budget_snapshot().reservation_count, 0);
    }

    // Boundary 2: Stop after acquire and submit, before present. The image is
    // discarded, never presented; the submitted work stays in the presenter's
    // retirement list until G resolves it; the input comes back intact.
    let gate = ScriptedGate {
        present: Some(PresentRejection::Cancelled),
        ..Default::default()
    };
    let (input, _, dropped) = artistic(5, size, projection());
    let input = returned(
        presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner),
        PresentRejection::Cancelled,
    );
    assert_eq!(
        *gate.events.borrow(),
        vec!["before_acquire", "before_present"]
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents()
        ),
        (3, 1, 2)
    );
    assert_eq!(presenter.retirements.len(), 1, "submitted owners retained");
    assert!(presenter.retirements[0].pending.is_some());
    assert!(presenter.retirements[0]._ready.is_none());
    drop(input);
    assert!(dropped.load(Ordering::SeqCst));

    let gate = ScriptedGate {
        present: Some(PresentRejection::Revoked),
        ..Default::default()
    };
    let (input, dropped) = blackout(6);
    let input = returned(
        presenter.present_hard_blackout(input, &gate, &mut env.fixture.owner),
        PresentRejection::Revoked,
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents(),
            presenter.hard_blackout_presents()
        ),
        (4, 2, 2, 1)
    );
    assert_eq!(presenter.retirements.len(), 1);
    assert!(presenter.retirements[0].pending.is_some());
    assert!(!dropped.load(Ordering::SeqCst));
    drop(input);
    reap(&mut presenter, &mut env.fixture.owner);
    assert_eq!(presenter.target.presented.len(), 2);

    // After Stop the owner is not Active: zero acquires, inputs returned.
    presenter.release_for_stop();
    env.fixture.owner.begin_stop();
    let (input, _, _) = artistic(7, size, projection());
    drop(returned(
        presenter.present_cpu_artistic(input, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::Cancelled,
    ));
    drop(returned(
        presenter.present_hard_blackout(
            blackout(8).0,
            &ScriptedGate::default(),
            &mut env.fixture.owner,
        ),
        PresentRejection::Cancelled,
    ));
    assert_eq!(presenter.target.acquires, 4);
    assert_eq!(presenter.presents(), 2);
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_cpu_artistic_and_blackout_reject_stale_or_malformed_input_and_validate_first_use() {
    let mut env = shared_env();
    let size = (19, 13);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();

    // Another extent, or another geometry generation: StaleGeometry before
    // any reservation or acquire; inputs come back intact.
    let (input, _, _) = artistic(1, (size.0 + 1, size.1), projection());
    drop(returned(
        presenter.present_cpu_artistic(input, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::StaleGeometry,
    ));
    let (mut input, _, _) = artistic(2, size, projection());
    input.frame_id.geometry_generation = GENERATION + 1;
    drop(returned(
        presenter.present_cpu_artistic(input, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::StaleGeometry,
    ));
    let (mut input, _) = blackout(3);
    input.frame_id.geometry_generation = GENERATION + 1;
    drop(returned(
        presenter.present_hard_blackout(input, &ScriptedGate::default(), &mut env.fixture.owner),
        PresentRejection::StaleGeometry,
    ));
    assert_eq!(presenter.target.acquires, 0);
    assert_eq!(env.fixture.owner.budget_snapshot().reservation_count, 0);

    // A partial CPU payload is a fault; the input is retained, not dropped.
    let (mut input, _, dropped) = artistic(4, size, projection());
    let truncated = input.frame.data.len() - 4;
    input.frame.data.truncate(truncated);
    let gate = ScriptedGate::default();
    faulted(
        presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner),
        NativeGpuFaultKind::IdentityMismatch,
    );
    assert!(gate.events.borrow().is_empty());
    assert_eq!(presenter.target.acquires, 0);
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(
        presenter.poll_retirements(&mut env.fixture.owner).unwrap(),
        0
    );
    assert!(dropped.load(Ordering::SeqCst));
    // Not a sticky fault: the next well-formed frame presents.
    let (input, frame, _) = artistic(5, size, projection());
    presented(presenter.present_cpu_artistic(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    reap(&mut presenter, &mut env.fixture.owner);
    assert_eq!(
        read_image(&env, &presenter.target.presented[0]),
        apply_video_output_mapping(frame, &projection()).data
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
    drop(presenter);
    drop(env);

    // A real validation error in the first CPU Artistic present phase: the
    // image is discarded before present and the presenter stays faulted, so
    // a later blackout never acquires either.
    let mut env = shared_env();
    let target = OffscreenTarget::with_image_format(&env, wgpu::TextureFormat::Bgra8Unorm);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (input, _, dropped) = artistic(1, size, projection());
    faulted(
        presenter.present_cpu_artistic(input, &ScriptedGate::default(), &mut env.fixture.owner),
        NativeGpuFaultKind::Validation,
    );
    assert_eq!(
        (
            presenter.target.acquires,
            presenter.target.discards,
            presenter.presents()
        ),
        (1, 1, 0)
    );
    assert!(!dropped.load(Ordering::SeqCst));
    assert_eq!(env.fixture.owner.state(), GpuOwnerState::Active);
    let fault = faulted(
        presenter.present_hard_blackout(
            blackout(2).0,
            &ScriptedGate::default(),
            &mut env.fixture.owner,
        ),
        NativeGpuFaultKind::Validation,
    );
    assert!(fault.message.contains("faulted"), "{fault:?}");
    assert_eq!(presenter.target.acquires, 1);
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

#[test]
fn shared_cpu_artistic_and_blackout_device_loss_never_present_and_retain_inputs() {
    let mut env = shared_env();
    let size = (17, 11);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    env.fixture.owner.context().device().destroy();
    let started = Instant::now();
    while env.fixture.owner.state() != GpuOwnerState::DeviceLostRetained {
        let _ = env.fixture.owner.poll_retirement();
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "loss not observed"
        );
        std::thread::yield_now();
    }
    let (input, _, artistic_dropped) = artistic(1, size, projection());
    let gate = ScriptedGate::default();
    faulted(
        presenter.present_cpu_artistic(input, &gate, &mut env.fixture.owner),
        NativeGpuFaultKind::DeviceLost,
    );
    let (input, blackout_dropped) = blackout(2);
    faulted(
        presenter.present_hard_blackout(input, &gate, &mut env.fixture.owner),
        NativeGpuFaultKind::DeviceLost,
    );
    assert!(gate.events.borrow().is_empty());
    assert_eq!((presenter.target.acquires, presenter.presents()), (0, 0));
    assert!(!artistic_dropped.load(Ordering::SeqCst));
    assert!(!blackout_dropped.load(Ordering::SeqCst));
    presenter.release_for_stop();
    assert!(artistic_dropped.load(Ordering::SeqCst));
    assert!(blackout_dropped.load(Ordering::SeqCst));
}

#[test]
fn shared_present_drives_resident_cpu_artistic_and_blackout_through_one_retirement_path() {
    let mut env = shared_env();
    let size = (41, 29);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let (ready, resident_pixels, ready_dropped) = real_ready(&mut env, 1, size, projection());
    let resident =
        presented(presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner));
    let (input, frame, cpu_dropped) = artistic(2, size, projection());
    let cpu = presented(presenter.present_cpu_artistic(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    let (input, blackout_dropped) = blackout(3);
    let black = presented(presenter.present_hard_blackout(
        input,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    let (ready, last_pixels, _) = real_ready(&mut env, 4, size, projection());
    let last =
        presented(presenter.present_ready(ready, &ScriptedGate::default(), &mut env.fixture.owner));
    assert_eq!(
        [
            resident.sequence(),
            cpu.sequence(),
            black.sequence(),
            last.sequence()
        ],
        [1, 2, 3, 4]
    );
    assert!(matches!(
        resident.content(),
        NativePresentContent::Resident(_)
    ));
    assert_eq!(cpu.content(), NativePresentContent::CpuArtistic);
    assert_eq!(black.content(), NativePresentContent::HardBlackout);
    let snapshot = resident
        .snapshot()
        .expect("a Resident receipt names its snapshot");
    assert_eq!(resident.snapshot(), snapshot);
    assert_ne!(
        cpu.snapshot(),
        snapshot,
        "no snapshot is claimed for CPU pixels"
    );
    assert_ne!(
        black.snapshot(),
        snapshot,
        "no snapshot is claimed for blackout"
    );
    assert!(matches!(last.content(), NativePresentContent::Resident(_)));
    let submissions = [
        resident.submission().sequence,
        cpu.submission().sequence,
        black.submission().sequence,
        last.submission().sequence,
    ];
    assert!(
        submissions.windows(2).all(|pair| pair[0] < pair[1]),
        "one owner, ordered submissions: {submissions:?}"
    );
    // Each present drained its predecessor; earlier inputs were released
    // once their submissions resolved, the last one is still held.
    assert!(ready_dropped.load(Ordering::SeqCst));
    assert!(cpu_dropped.load(Ordering::SeqCst));
    assert!(blackout_dropped.load(Ordering::SeqCst));
    assert_eq!(presenter.retirements.len(), 1);
    reap(&mut presenter, &mut env.fixture.owner);
    assert_eq!(
        (presenter.presents(), presenter.hard_blackout_presents()),
        (4, 1)
    );
    assert_eq!(
        read_image(&env, &presenter.target.presented[0]),
        apply_video_output_mapping(resident_pixels, &projection()).data
    );
    assert_eq!(
        read_image(&env, &presenter.target.presented[1]),
        apply_video_output_mapping(frame, &projection()).data
    );
    assert!(exact_black(&read_image(
        &env,
        &presenter.target.presented[2]
    )));
    assert_eq!(
        read_image(&env, &presenter.target.presented[3]),
        apply_video_output_mapping(last_pixels, &projection()).data
    );
    // Pools: mapped target, mapping inputs and the CPU source; every Ready
    // premapping, upload and staging charge was released.
    wait_for_allocations(&mut env.fixture.owner, 5);
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}

struct InFlightBuffer {
    buffer: wgpu::Buffer,
    _charge: AllocationCharge,
}

/// A real submitted, unpolled logical output (the resident ring's output N+1).
fn in_flight_logical_output(
    owner: &mut NativeGpuOwner,
    frame: NativeFrameId,
) -> (PendingPhaseReceipt, PendingGpuTransaction) {
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_new(ChargeKind::IsfScratch, 16).unwrap();
    let quote = owner.quote(&plan).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let mut txn = owner.begin_frame(frame, reservation).unwrap();
    let mut phase = txn.begin_phase(GpuPhase::IsfBatch).unwrap();
    let charge = phase.bind_allocation(ChargeKind::IsfScratch, 16).unwrap();
    let id = charge.id();
    let resource = Arc::new(InFlightBuffer {
        buffer: phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("ring next logical output"),
            size: 16,
            usage: wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        }),
        _charge: charge,
    });
    phase.retain_resource(Arc::clone(&resource), &[id]).unwrap();
    let mut encoder = phase
        .device()
        .create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("ring next logical output clear"),
        });
    encoder.clear_buffer(&resource.buffer, 0, None);
    let encoded = phase.finish(vec![encoder.finish()]);
    let receipt = txn.submit_phase(encoded).unwrap();
    (receipt, txn.finish())
}

/// ADR-01 section 7.2 snapshot ring, P side: output N presents while output
/// N+1 already owns G's logical slot. The present is an auxiliary
/// transaction queued after N+1; a revoked acquire refunds only its own unused
/// root, N+1 resolves independently, and the presented pixels are N's.
#[test]
fn shared_present_runs_beside_the_next_in_flight_logical_output() {
    let mut env = shared_env();
    let size = (23, 17);
    let target = OffscreenTarget::new(&env);
    let mut presenter = new_presenter(&mut env, target);
    presenter
        .configure_admitted(geometry(size), &mut env.fixture.owner)
        .unwrap();
    let mapping = projection();
    let (ready, pixels, _) = real_ready(&mut env, 1, size, mapping.clone());
    let (next_receipt, next) =
        in_flight_logical_output(&mut env.fixture.owner, display_frame_id(2));
    assert!(env.fixture.owner.has_unfinished_logical_frame());
    let reservations = env.fixture.owner.budget_snapshot().reservation_count;

    let gate = ScriptedGate {
        acquire: Some(PresentRejection::Revoked),
        ..Default::default()
    };
    let ready = expect_not_presented(
        presenter.present_ready(ready, &gate, &mut env.fixture.owner),
        PresentRejection::Revoked,
    );
    assert_eq!(presenter.target.acquires, 0);
    assert_eq!(
        env.fixture.owner.budget_snapshot().reservation_count,
        reservations,
        "only the present's unused root was refunded"
    );

    let receipt = expect_presented(presenter.present_ready(
        ready,
        &ScriptedGate::default(),
        &mut env.fixture.owner,
    ));
    assert!(receipt.submission().sequence > next_receipt.submission().sequence);
    assert!(
        env.fixture.owner.has_unfinished_logical_frame(),
        "output N+1 keeps the logical slot through N's present"
    );
    let mut resolved = await_transaction(&mut env.fixture.owner, next, &|| false)
        .unwrap_or_else(|_| panic!("output N+1 did not resolve"));
    resolved
        .take_phase(next_receipt)
        .unwrap()
        .into_validation()
        .unwrap();
    let started = Instant::now();
    while presenter.poll_retirements(&mut env.fixture.owner).unwrap() != 0 {
        assert!(started.elapsed() < Duration::from_secs(5));
        std::thread::yield_now();
    }
    assert_eq!(presenter.presents(), 1);
    let image = read_image(&env, &presenter.target.presented[0]);
    assert_eq!(
        image,
        apply_video_output_mapping(pixels, &mapping).data,
        "the presented image is output N's premapping snapshot, mapped once"
    );
    stop_and_reap(&mut presenter, &mut env.fixture.owner);
}
