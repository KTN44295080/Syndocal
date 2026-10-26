//! P lane: native presenter on G's shared context (INTERFACE-01 sections 3.2,
//! 6.3 and 6.4; ADR-01 section 8). C4 prerequisite, test-gated and not routed.
//!
//! A Ready payload is P's premapping composition buffer that the snapshot path
//! already copied, unmapped and published to the CPU LastValid cache. Present
//! is a separate `MappingPresent` transaction on the same owner: one output
//! mapping pass (the existing `gpu_output_mapping.wgsl`, applied exactly once)
//! into a pooled mapped buffer, then the existing buffer blit into the
//! acquired target. The presenter rechecks owner/geometry/authority/Stop right
//! before acquire and again right before present, so a revoked or stopped
//! output gets zero presents. No surface image is held across calls.
//!
//! Ownership: the surface (which owns its window `Arc`) lives in the target;
//! the device and every charge live in G's owner. Submitted resources,
//! including the Ready's premapping source, are retained in G's registry
//! until the present submission completes; a consumed Ready stays in this
//! presenter's retirement list until then. The existing CPU-upload
//! `GpuSurfacePresenter` is unchanged and remains the production path.
//!
//! The same presenter also displays the two non-resident contents of the
//! Artistic Display route on the same owner (P53): a canonical CPU Artistic
//! result (uploaded once through G's broker/ledger into a pooled source, then
//! the same single mapping pass and blit as the legacy MapOnce arm) and a
//! payload-less HardBlackout (a direct exact-black clear of the acquired
//! image; no upload, no mapping, no Ready/LastValid). All three share one
//! outcome/retirement shape, so I drives them through one path.

use std::any::Any;
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::{Duration, Instant};

use protocol::VideoOutputMapping;
use wgpu::util::DeviceExt;

use super::{
    create_buffer_output_bind_group_layout, create_buffer_output_pipeline, dimensions_uniform,
    UnconfiguredNativeSurface, HARD_BLACKOUT_CLEAR_COLOR,
};
use crate::gpu_compositor::{
    create_blank_mapping_mask_texture, create_compositor_bind_group_layout,
    create_output_mapping_pipeline, dispatch_dimensions, mapping_mask_texture_bytes,
    output_mapping_params, MAPPING_MASK_DIMENSION, MAPPING_MASK_PADDED_BYTES_PER_ROW,
};
use crate::native_gpu_budget::{AllocationCharge, ChargeKind, NativeAllocationPlan};
use crate::native_gpu_composition::NativeChargedBuffer;
use crate::native_gpu_context::{
    ContextCreateError, GpuPhaseEncoder, NativeGpuOwner, PendingGpuTransaction, PendingPhaseReceipt,
};
use crate::native_gpu_contract::{
    CacheCommitReceipt, CompletionProgress, GpuAllocationId, GpuContextId, GpuOwnerState, GpuPhase,
    GpuSubmissionId, NativeFrameId, NativeGpuFault, NativeGpuFaultKind, NativePresentSink,
    PresentOutcome, PresentRejection, SnapshotIdentity,
};
use crate::native_gpu_output::{NativeOutputAuthority, NativePresentPermit, ReadyNativeGpuOutput};
use crate::native_gpu_snapshot::NativeSnapshotGpu;
use crate::{VideoFrame, VideoPixelFormat};

const MAPPING_PARAMS_BYTES: u64 = 256;
const DIMENSIONS_BYTES: u64 = 16;
const MASK_TEXTURE_BYTES: u64 = (MAPPING_MASK_DIMENSION * MAPPING_MASK_DIMENSION * 4) as u64;
const MASK_STAGING_BYTES: u64 = (MAPPING_MASK_PADDED_BYTES_PER_ROW * MAPPING_MASK_DIMENSION) as u64;
/// Bounded host waits: first-use validation and present-submission drain.
/// Expiry keeps every owner in G's registry; nothing is detached or faked.
const PRESENT_DEADLINE: Duration = Duration::from_secs(5);

/// What the presenter reads from a Ready. Pixels are only ever P's committed
/// premapping source: the cache receipt (minted by I's real cache publication)
/// must name exactly that snapshot and allocation.
pub(crate) trait FrozenPresentation {
    fn cache_receipt(&self) -> &CacheCommitReceipt;
    fn snapshot_gpu(&self) -> &NativeSnapshotGpu;
    /// The output ticket's frozen mapping, applied here exactly once.
    fn mapping(&self) -> &VideoOutputMapping;
}

/// A canonical CPU post-FX (premapping) Artistic result shown on the shared
/// owner. The presenter uploads `frame()` once, applies `mapping()` exactly
/// once and blits, as the legacy `present_output_artistic_result` MapOnce arm.
pub(crate) trait CpuArtisticPresentation {
    /// This display tick; its geometry generation must be the admitted one.
    fn frame_id(&self) -> NativeFrameId;
    /// Packed RGBA8 at the admitted extent (`data.len() == w * h * 4`).
    fn frame(&self) -> &VideoFrame;
    /// The output ticket's frozen mapping, applied here exactly once.
    fn mapping(&self) -> &VideoOutputMapping;
}

/// A payload-less hard blackout: the acquired image is cleared to opaque
/// exact black. There are no pixels to read, map, cache or retain.
pub(crate) trait HardBlackoutPresentation {
    /// This display tick; its geometry generation must be the admitted one.
    fn frame_id(&self) -> NativeFrameId;
}

/// What one present reads from its input, borrowed for planning and encode.
#[derive(Clone, Copy)]
pub(crate) enum PresentSource<'a> {
    Premapping {
        buffer: &'a Arc<NativeChargedBuffer>,
        mapping: &'a VideoOutputMapping,
    },
    Cpu {
        frame: &'a VideoFrame,
        mapping: &'a VideoOutputMapping,
    },
    ExactBlack,
}

/// Content markers. Each selects one present entry and its input trait;
/// `SharedPresentSink` defaults to `ResidentContent` (`present_ready`).
pub(crate) struct ResidentContent;
pub(crate) struct CpuArtisticContent;
pub(crate) struct HardBlackoutContent;

/// Dispatch from a content marker to the input it presents. The input's
/// shape/identity checks run before any reservation, acquire or GPU work.
pub(crate) trait PresentContentKind<R> {
    fn frame_id(input: &R) -> NativeFrameId;
    /// The extent the input was rendered at; `None` for payload-less input,
    /// which always uses the admitted extent.
    fn input_extent(input: &R) -> Option<(u32, u32)>;
    fn validate(input: &R, context: GpuContextId, extent: (u32, u32)) -> Result<(), &'static str>;
    fn source(input: &R) -> PresentSource<'_>;
    fn content(input: &R) -> NativePresentContent;
}

impl<R: FrozenPresentation> PresentContentKind<R> for ResidentContent {
    fn frame_id(input: &R) -> NativeFrameId {
        input.cache_receipt().descriptor().snapshot.frame
    }

    fn input_extent(input: &R) -> Option<(u32, u32)> {
        let snapshot = input.cache_receipt().descriptor().snapshot;
        Some((snapshot.width, snapshot.height))
    }

    fn validate(input: &R, context: GpuContextId, extent: (u32, u32)) -> Result<(), &'static str> {
        let snapshot = input.cache_receipt().descriptor().snapshot;
        let premapping = input.snapshot_gpu().premapping_owner();
        let valid = snapshot.format == VideoPixelFormat::Rgba8
            && snapshot.allocation == premapping.allocation_id()
            && snapshot.allocation.context == context
            && snapshot.completed_submission.context == context
            && packed_bytes(extent).is_some_and(|bytes| premapping.capacity() >= bytes);
        if valid {
            Ok(())
        } else {
            Err("Ready does not carry its committed premapping source")
        }
    }

    fn source(input: &R) -> PresentSource<'_> {
        PresentSource::Premapping {
            buffer: input.snapshot_gpu().premapping_owner(),
            mapping: input.mapping(),
        }
    }

    fn content(input: &R) -> NativePresentContent {
        NativePresentContent::Resident(input.cache_receipt().descriptor().snapshot)
    }
}

impl<R: CpuArtisticPresentation> PresentContentKind<R> for CpuArtisticContent {
    fn frame_id(input: &R) -> NativeFrameId {
        input.frame_id()
    }

    fn input_extent(input: &R) -> Option<(u32, u32)> {
        Some((input.frame().width, input.frame().height))
    }

    fn validate(input: &R, _context: GpuContextId, extent: (u32, u32)) -> Result<(), &'static str> {
        let frame = input.frame();
        if frame.format == VideoPixelFormat::Rgba8
            && packed_bytes(extent) == u64::try_from(frame.data.len()).ok()
        {
            Ok(())
        } else {
            Err("CPU Artistic frame is not packed RGBA8 of its extent")
        }
    }

    fn source(input: &R) -> PresentSource<'_> {
        PresentSource::Cpu {
            frame: input.frame(),
            mapping: input.mapping(),
        }
    }

    fn content(_input: &R) -> NativePresentContent {
        NativePresentContent::CpuArtistic
    }
}

impl<R: HardBlackoutPresentation> PresentContentKind<R> for HardBlackoutContent {
    fn frame_id(input: &R) -> NativeFrameId {
        input.frame_id()
    }

    fn input_extent(_input: &R) -> Option<(u32, u32)> {
        None
    }

    fn validate(
        _input: &R,
        _context: GpuContextId,
        _extent: (u32, u32),
    ) -> Result<(), &'static str> {
        Ok(())
    }

    fn source(_input: &R) -> PresentSource<'_> {
        PresentSource::ExactBlack
    }

    fn content(_input: &R) -> NativePresentContent {
        NativePresentContent::HardBlackout
    }
}

/// The two physical-boundary rechecks of INTERFACE-01 section 6.4.
pub(crate) trait NativePresentGate<R> {
    fn before_acquire(&self, ready: &R) -> Result<(), PresentRejection>;
    fn before_present(&self, ready: &R) -> Result<(), PresentRejection>;
}

impl<A: NativeOutputAuthority, G> NativePresentGate<ReadyNativeGpuOutput<G>>
    for NativePresentPermit<'_, A>
{
    fn before_acquire(&self, ready: &ReadyNativeGpuOutput<G>) -> Result<(), PresentRejection> {
        NativePresentPermit::before_acquire(self, ready)
    }

    fn before_present(&self, ready: &ReadyNativeGpuOutput<G>) -> Result<(), PresentRejection> {
        NativePresentPermit::before_present(self, ready)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TargetAcquireError {
    Timeout,
    Outdated,
    Lost,
    OutOfMemory,
    Other,
}

impl From<wgpu::SurfaceError> for TargetAcquireError {
    fn from(error: wgpu::SurfaceError) -> Self {
        match error {
            wgpu::SurfaceError::Timeout => Self::Timeout,
            wgpu::SurfaceError::Outdated => Self::Outdated,
            wgpu::SurfaceError::Lost => Self::Lost,
            wgpu::SurfaceError::OutOfMemory => Self::OutOfMemory,
            wgpu::SurfaceError::Other => Self::Other,
        }
    }
}

/// The swapchain side, separate from the device: the real surface, or an
/// offscreen texture for headless pixel and failure evidence.
pub(crate) trait NativePresentTarget {
    type Frame;
    fn format(&self) -> wgpu::TextureFormat;
    /// Runs inside G's Initialize phase, so its validation is scope-captured.
    fn configure(&mut self, device: &wgpu::Device, extent: (u32, u32));
    fn acquire(&mut self) -> Result<Self::Frame, TargetAcquireError>;
    fn view(&self, frame: &Self::Frame) -> wgpu::TextureView;
    fn present(&mut self, frame: Self::Frame);
    /// Drops an acquired image without presenting it.
    fn discard(&mut self, frame: Self::Frame);
}

/// The real wgpu surface created by `UnconfiguredNativeSurface` from G's
/// factory instance. Configuration follows the existing presenter: the
/// adapter's default surface configuration with `AutoVsync`. Only
/// `SharedSurfacePresenter::from_shared_unconfigured` constructs it.
pub(crate) struct NativeSurfaceTarget {
    surface: UnconfiguredNativeSurface,
    config: wgpu::SurfaceConfiguration,
}

impl NativeSurfaceTarget {
    /// Re-arm path: the same surface and window for G's next generation
    /// (`create_for_surface`, then `from_shared_unconfigured`, which
    /// configures it on the new device only after admission).
    #[expect(
        dead_code,
        reason = "C4 native presenter route; I enables it with the native first-frame bootstrap"
    )]
    pub(crate) fn into_unconfigured(self) -> UnconfiguredNativeSurface {
        self.surface
    }
}

impl NativePresentTarget for NativeSurfaceTarget {
    type Frame = wgpu::SurfaceTexture;

    fn format(&self) -> wgpu::TextureFormat {
        self.config.format
    }

    fn configure(&mut self, device: &wgpu::Device, extent: (u32, u32)) {
        self.config.width = extent.0;
        self.config.height = extent.1;
        self.surface.surface.configure(device, &self.config);
    }

    fn acquire(&mut self) -> Result<Self::Frame, TargetAcquireError> {
        self.surface
            .surface
            .get_current_texture()
            .map_err(TargetAcquireError::from)
    }

    fn view(&self, frame: &Self::Frame) -> wgpu::TextureView {
        frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn present(&mut self, frame: Self::Frame) {
        frame.present();
    }

    fn discard(&mut self, frame: Self::Frame) {
        drop(frame);
    }
}

/// Physical geometry admitted by I (declared extent on the hidden path,
/// observed extent once visible) and the generation a Ready must carry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct AdmittedPhysicalGeometry {
    pub extent: (u32, u32),
    pub geometry_generation: u64,
}

/// What one present put on the image.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativePresentContent {
    /// P's committed premapping snapshot (`present_ready`).
    Resident(SnapshotIdentity),
    /// A canonical CPU Artistic result (`present_cpu_artistic`).
    CpuArtistic,
    /// Payload-less exact black (`present_hard_blackout`).
    HardBlackout,
}

/// `receipt.snapshot() == identity` holds only for the Resident present of
/// exactly that snapshot; a CPU Artistic or HardBlackout receipt (`None`)
/// never equals any snapshot.
impl PartialEq<SnapshotIdentity> for Option<SnapshotIdentity> {
    fn eq(&self, other: &SnapshotIdentity) -> bool {
        self.as_ref() == Some(other)
    }
}

/// Evidence that `present` was called for one input; not GPU completion.
#[derive(Debug)]
pub(crate) struct NativePresentReceipt {
    frame: NativeFrameId,
    content: NativePresentContent,
    submission: GpuSubmissionId,
    extent: (u32, u32),
    sequence: u64,
}

impl NativePresentReceipt {
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    pub(crate) fn content(&self) -> NativePresentContent {
        self.content
    }
    /// The presented snapshot; only Resident presents have one. Existing
    /// callers that compare it with a `SnapshotIdentity` keep compiling (see
    /// the `PartialEq` impl below): only a Resident receipt can be equal.
    pub(crate) fn snapshot(&self) -> Option<SnapshotIdentity> {
        match self.content {
            NativePresentContent::Resident(snapshot) => Some(snapshot),
            NativePresentContent::CpuArtistic | NativePresentContent::HardBlackout => None,
        }
    }
    pub(crate) fn submission(&self) -> GpuSubmissionId {
        self.submission
    }
    pub(crate) fn extent(&self) -> (u32, u32) {
        self.extent
    }
    pub(crate) fn sequence(&self) -> u64 {
        self.sequence
    }
}

struct NativePresentPipelines {
    mapping_layout: wgpu::BindGroupLayout,
    mapping: wgpu::ComputePipeline,
    output_layout: wgpu::BindGroupLayout,
    output: wgpu::RenderPipeline,
}

impl NativePresentPipelines {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let mapping_layout = create_compositor_bind_group_layout(device);
        let mapping_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Syndocal shared output mapping pipeline layout"),
                bind_group_layouts: &[&mapping_layout],
                push_constant_ranges: &[],
            });
        let mapping = create_output_mapping_pipeline(device, &mapping_pipeline_layout);
        let output_layout = create_buffer_output_bind_group_layout(device);
        let output = create_buffer_output_pipeline(device, &output_layout, format);
        Self {
            mapping_layout,
            mapping,
            output_layout,
            output,
        }
    }
}

/// A physical resource and the charge that lives exactly as long as it.
struct Charged<T> {
    resource: T,
    charge: AllocationCharge,
}

/// Pooled per admitted extent: the mapped buffer, its dimensions uniform and
/// the blit bind group. Replaced on resize; in-flight users keep the old one.
struct MappedTarget {
    extent: (u32, u32),
    mapped: Charged<wgpu::Buffer>,
    dimensions: Charged<wgpu::Buffer>,
    output_bind_group: wgpu::BindGroup,
}

impl MappedTarget {
    fn ids(&self) -> [GpuAllocationId; 2] {
        [self.mapped.charge.id(), self.dimensions.charge.id()]
    }
}

/// Pooled per exact mapping bytes: the params uniform and bitmap mask.
struct MappingInputs {
    params_bytes: [u8; 256],
    mask_bytes: Vec<u8>,
    params: Charged<wgpu::Buffer>,
    mask: Charged<wgpu::Texture>,
    mask_view: wgpu::TextureView,
}

impl MappingInputs {
    fn ids(&self) -> [GpuAllocationId; 2] {
        [self.params.charge.id(), self.mask.charge.id()]
    }
}

/// Pooled per admitted extent: the storage buffer a CPU Artistic frame is
/// uploaded into; the single mapping pass reads it like a premapping source.
struct CpuArtisticSource {
    extent: (u32, u32),
    source: Charged<wgpu::Buffer>,
}

impl CpuArtisticSource {
    fn id(&self) -> GpuAllocationId {
        self.source.charge.id()
    }
}

/// Per-present owners retained in G's registry with the submission: the
/// mapping bind group, a new mask staging buffer and the queue-upload charge
/// of a CPU Artistic frame (wgpu's staging copy of it).
struct PresentFrameResources {
    _bind_group: wgpu::BindGroup,
    _staging: Option<Charged<wgpu::Buffer>>,
    _upload: Option<AllocationCharge>,
}

/// A consumed input and its (possibly) still pending present submission.
struct PresentRetirement {
    pending: Option<PendingGpuTransaction>,
    receipt: Option<PendingPhaseReceipt>,
    _ready: Option<Box<dyn Any + Send>>,
}

/// What this present must allocate: a mapped target for a new extent,
/// mapping inputs (exact params and mask bytes) for a new frozen mapping, and
/// a CPU Artistic source for a new extent.
struct PresentNeeds {
    mapped: bool,
    inputs: Option<([u8; 256], Vec<u8>)>,
    cpu_source: bool,
}

/// First-use validation-before-present, per content: each has its own cold
/// resources (premapping source, CPU upload, bare clear). Reset on configure.
#[derive(Clone, Copy, Debug, Default)]
struct FirstUseValidated {
    resident: bool,
    cpu_artistic: bool,
    hard_blackout: bool,
}

impl FirstUseValidated {
    fn slot(&mut self, content: NativePresentContent) -> &mut bool {
        match content {
            NativePresentContent::Resident(_) => &mut self.resident,
            NativePresentContent::CpuArtistic => &mut self.cpu_artistic,
            NativePresentContent::HardBlackout => &mut self.hard_blackout,
        }
    }
}

enum FirstUse {
    Validated,
    Failed(NativeGpuFault),
    Cancelled,
    Expired,
}

/// The shared-context presenter. Field order is drop order: retirements and
/// pooled resources before the pipelines and the target (surface + window).
pub(crate) struct SharedSurfacePresenter<T: NativePresentTarget> {
    retirements: Vec<PresentRetirement>,
    mapped: Option<Arc<MappedTarget>>,
    inputs: Option<Arc<MappingInputs>>,
    cpu_source: Option<Arc<CpuArtisticSource>>,
    pipelines: Arc<NativePresentPipelines>,
    target: T,
    context: GpuContextId,
    configured: Option<(u32, u32)>,
    geometry: Option<AdmittedPhysicalGeometry>,
    validated: FirstUseValidated,
    fault: Option<(NativeGpuFaultKind, String)>,
    presents: u64,
    hard_blackouts: u64,
}

impl SharedSurfacePresenter<NativeSurfaceTarget> {
    /// INTERFACE-01 section 3.1: the unconfigured surface whose adapter G
    /// selected. Nothing is configured, acquired or presented here.
    #[expect(
        dead_code,
        reason = "C4 native presenter route; I enables it with the native first-frame bootstrap"
    )]
    pub(crate) fn from_shared_unconfigured(
        surface: UnconfiguredNativeSurface,
        owner: &mut NativeGpuOwner,
    ) -> Result<Self, ContextCreateError> {
        if !Arc::ptr_eq(&surface.instance, owner.context().instance()) {
            return Err(ContextCreateError::Instance(
                "surface belongs to another factory".into(),
            ));
        }
        let mut config = surface
            .surface
            .get_default_config(owner.context().adapter(), 1, 1)
            .ok_or_else(|| {
                ContextCreateError::Surface("no supported surface configuration".into())
            })?;
        config.present_mode = wgpu::PresentMode::AutoVsync;
        Self::new(NativeSurfaceTarget { surface, config }, owner)
            .map_err(|fault| ContextCreateError::Device(fault.message))
    }
}

impl<T: NativePresentTarget> SharedSurfacePresenter<T> {
    /// Pipelines are created in G's scoped Initialize phase, never between a
    /// frame's phases. The target stays unconfigured until admission.
    pub(crate) fn new(target: T, owner: &mut NativeGpuOwner) -> Result<Self, NativeGpuFault> {
        let format = target.format();
        let pipelines =
            owner.initialize(|device, _| NativePresentPipelines::new(device, format))?;
        Ok(Self {
            retirements: Vec::new(),
            mapped: None,
            inputs: None,
            cpu_source: None,
            pipelines: Arc::new(pipelines),
            target,
            context: owner.context().id(),
            configured: None,
            geometry: None,
            validated: FirstUseValidated::default(),
            fault: None,
            presents: 0,
            hard_blackouts: 0,
        })
    }

    pub(crate) fn configured_extent(&self) -> Option<(u32, u32)> {
        self.configured
    }

    /// Every physical present: Resident, CPU Artistic and HardBlackout.
    pub(crate) fn presents(&self) -> u64 {
        self.presents
    }

    /// HardBlackout presents only (also counted in `presents`).
    pub(crate) fn hard_blackout_presents(&self) -> u64 {
        self.hard_blackouts
    }

    /// A borrowed `NativePresentSink` for I's present policy.
    pub(crate) fn sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> SharedPresentSink<'p, T, R, P> {
        SharedPresentSink {
            presenter: self,
            owner,
            _types: PhantomData,
        }
    }

    /// The `NativePresentSink` over `present_cpu_artistic`.
    pub(crate) fn cpu_artistic_sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> SharedPresentSink<'p, T, R, P, CpuArtisticContent> {
        SharedPresentSink {
            presenter: self,
            owner,
            _types: PhantomData,
        }
    }

    /// The `NativePresentSink` over `present_hard_blackout`.
    pub(crate) fn hard_blackout_sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> SharedPresentSink<'p, T, R, P, HardBlackoutContent> {
        SharedPresentSink {
            presenter: self,
            owner,
            _types: PhantomData,
        }
    }

    fn fault_at(
        &self,
        frame: Option<NativeFrameId>,
        kind: NativeGpuFaultKind,
        message: &str,
    ) -> NativeGpuFault {
        NativeGpuFault {
            context: self.context,
            frame,
            phase: GpuPhase::MappingPresent,
            kind,
            message: message.into(),
        }
    }

    /// Owner generation, liveness and sticky faults. `Ok(false)` means the
    /// owner is stopping: zero presents, the caller retires the Ready.
    fn check_owner(
        &self,
        owner: &NativeGpuOwner,
        frame: Option<NativeFrameId>,
    ) -> Result<bool, NativeGpuFault> {
        if let Some((kind, message)) = &self.fault {
            return Err(self.fault_at(
                frame,
                *kind,
                &format!("native presenter is faulted: {message}"),
            ));
        }
        if owner.context().id() != self.context {
            return Err(self.fault_at(
                frame,
                NativeGpuFaultKind::IdentityMismatch,
                "presenter belongs to another device generation",
            ));
        }
        match owner.state() {
            GpuOwnerState::Active => Ok(true),
            GpuOwnerState::Retiring | GpuOwnerState::Reaped => Ok(false),
            GpuOwnerState::DeviceLostRetained => Err(self.fault_at(
                frame,
                NativeGpuFaultKind::DeviceLost,
                "shared device lost; no present",
            )),
            GpuOwnerState::PoisonedRetained => Err(self.fault_at(
                frame,
                NativeGpuFaultKind::ScopeDrain,
                "shared generation is quarantined; no present",
            )),
        }
    }

    /// Configure (or reconfigure on resize) to I's admitted physical extent.
    /// A zero extent is Suspended: zero presents, the surface is untouched.
    /// Old-geometry resources are released here; in-flight users keep theirs.
    pub(crate) fn configure_admitted(
        &mut self,
        geometry: AdmittedPhysicalGeometry,
        owner: &mut NativeGpuOwner,
    ) -> Result<(), NativeGpuFault> {
        if !self.check_owner(owner, None)? {
            return Err(self.fault_at(
                None,
                NativeGpuFaultKind::StateMismatch,
                "owner is stopping; no configure",
            ));
        }
        let (width, height) = geometry.extent;
        if width == 0 || height == 0 {
            self.geometry = Some(geometry);
            return Ok(());
        }
        let limits = &owner.capabilities().limits;
        let fits = width <= limits.max_texture_dimension_2d
            && height <= limits.max_texture_dimension_2d
            && packed_bytes(geometry.extent)
                .is_some_and(|bytes| bytes <= u64::from(limits.max_storage_buffer_binding_size));
        if !fits {
            return Err(self.fault_at(
                None,
                NativeGpuFaultKind::StateMismatch,
                "admitted extent exceeds the shared device limits",
            ));
        }
        if self.configured != Some(geometry.extent) {
            self.reconfigure(owner, geometry.extent)?;
        }
        self.geometry = Some(geometry);
        Ok(())
    }

    fn reconfigure(
        &mut self,
        owner: &mut NativeGpuOwner,
        extent: (u32, u32),
    ) -> Result<(), NativeGpuFault> {
        self.drain_retirements(owner)?;
        self.configured = None;
        self.validated = FirstUseValidated::default();
        if self
            .mapped
            .as_ref()
            .is_some_and(|mapped| mapped.extent != extent)
        {
            self.mapped = None;
        }
        if self
            .cpu_source
            .as_ref()
            .is_some_and(|source| source.extent != extent)
        {
            self.cpu_source = None;
        }
        let target = &mut self.target;
        owner.initialize(|device, _| target.configure(device, extent))?;
        self.configured = Some(extent);
        Ok(())
    }

    /// Nonblocking: reap present submissions that G resolved. A resolved
    /// validation failure faults the presenter. Returns the pending count.
    /// I calls this each tick; since G3 a retiring present no longer blocks
    /// the next logical frame's `begin_frame`, so no drain is needed there.
    pub(crate) fn poll_retirements(
        &mut self,
        owner: &mut NativeGpuOwner,
    ) -> Result<usize, NativeGpuFault> {
        let mut first_fault = None;
        let mut index = 0;
        while index < self.retirements.len() {
            let record = &mut self.retirements[index];
            let outcome = match record.pending.as_mut() {
                None => Some(Ok(())),
                Some(pending) => match owner.poll_transaction(pending, &|| false) {
                    CompletionProgress::Pending => None,
                    CompletionProgress::Cancelled => Some(Ok(())),
                    CompletionProgress::Complete(Ok(mut resolved)) => {
                        Some(match record.receipt.take() {
                            Some(receipt) => resolved
                                .take_phase(receipt)
                                .and_then(|phase| phase.into_validation()),
                            None => Ok(()),
                        })
                    }
                    CompletionProgress::Complete(Err(fault)) => Some(Err(fault)),
                },
            };
            match outcome {
                None => index += 1,
                Some(result) => {
                    self.retirements.remove(index);
                    if let Err(fault) = result {
                        self.fault
                            .get_or_insert((fault.kind, fault.message.clone()));
                        first_fault.get_or_insert(fault);
                    }
                }
            }
        }
        // G finishes cancelled/abandoned records only through its retirement
        // poll; it is nonblocking and also reaps released allocations.
        owner.poll_retirement()?;
        match first_fault {
            Some(fault) => Err(fault),
            None => Ok(self.retirements.len()),
        }
    }

    /// Bounded backpressure: G refuses a second MappingPresent while one
    /// retires, so the previous present submission must resolve before this
    /// presenter's next present (and before Stop/re-arm). The next logical
    /// frame is admitted without it (G3).
    pub(crate) fn drain_retirements(
        &mut self,
        owner: &mut NativeGpuOwner,
    ) -> Result<(), NativeGpuFault> {
        let started = Instant::now();
        loop {
            if self.poll_retirements(owner)? == 0 {
                return Ok(());
            }
            if started.elapsed() >= PRESENT_DEADLINE {
                return Err(self.fault_at(
                    None,
                    NativeGpuFaultKind::ScopeDrain,
                    "previous present submission did not complete; owners retained",
                ));
            }
            std::thread::yield_now();
        }
    }

    /// Stop: release this presenter's pooled resources and consumed Readys.
    /// G's registry keeps every submitted owner until it reaps them, and no
    /// later present is possible because the owner is no longer Active.
    pub(crate) fn release_for_stop(&mut self) {
        self.retirements.clear();
        self.mapped = None;
        self.inputs = None;
        self.cpu_source = None;
    }

    /// Surface lifetime is separate from the device's (ADR-01 section 8.2):
    /// after Stop or device loss the target (surface and its window) leaves
    /// this generation's presenter for a new owner/presenter to re-arm. This
    /// generation's submitted owners stay in its G registry until reaped.
    pub(crate) fn into_target(mut self) -> T {
        self.release_for_stop();
        self.target
    }

    fn retain_failure<R: Send + 'static>(
        &mut self,
        ready: R,
        pending: Option<PendingGpuTransaction>,
        receipt: Option<PendingPhaseReceipt>,
        fault: NativeGpuFault,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault> {
        self.retirements.push(PresentRetirement {
            pending,
            receipt,
            _ready: Some(Box::new(ready)),
        });
        Err(fault)
    }

    /// Present one Ready exactly once (INTERFACE-01 section 6.4). The Ready is
    /// consumed: `Presented` keeps it until the submission completes,
    /// `NotPresented` returns it intact for I to retry or retire, and `Err`
    /// keeps it in the presenter's retirement list.
    pub(crate) fn present_ready<R, P>(
        &mut self,
        ready: R,
        gate: &P,
        owner: &mut NativeGpuOwner,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault>
    where
        R: FrozenPresentation + Send + 'static,
        P: NativePresentGate<R>,
    {
        self.present_content::<ResidentContent, R, P>(ready, gate, owner)
    }

    /// Display one canonical CPU Artistic result on the shared owner: one
    /// upload of `frame()` through G's broker/ledger into the pooled source,
    /// the single output mapping pass and the existing blit (the legacy
    /// MapOnce arm's pixels). Outcome and retirement are `present_ready`'s:
    /// `Presented` retains the whole input (its CPU charge owner) with the
    /// submission until G resolves it; `NotPresented` returns it intact;
    /// `Err` keeps it in the retirement list.
    pub(crate) fn present_cpu_artistic<R, P>(
        &mut self,
        artistic: R,
        gate: &P,
        owner: &mut NativeGpuOwner,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault>
    where
        R: CpuArtisticPresentation + Send + 'static,
        P: NativePresentGate<R>,
    {
        self.present_content::<CpuArtisticContent, R, P>(artistic, gate, owner)
    }

    /// Payload-less HardBlackout: clear the acquired image to opaque exact
    /// black in a G MappingPresent phase on the shared device. No upload, no
    /// mapping pass, no pooled resource, no Ready or LastValid. Same gate,
    /// first-use validation, outcome and retirement as `present_ready`.
    pub(crate) fn present_hard_blackout<R, P>(
        &mut self,
        blackout: R,
        gate: &P,
        owner: &mut NativeGpuOwner,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault>
    where
        R: HardBlackoutPresentation + Send + 'static,
        P: NativePresentGate<R>,
    {
        self.present_content::<HardBlackoutContent, R, P>(blackout, gate, owner)
    }

    /// The one present protocol for every content kind.
    fn present_content<K, R, P>(
        &mut self,
        input: R,
        gate: &P,
        owner: &mut NativeGpuOwner,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault>
    where
        K: PresentContentKind<R>,
        R: Send + 'static,
        P: NativePresentGate<R>,
    {
        let not_presented = |reason, input| {
            Ok(PresentOutcome::NotPresented {
                reason,
                retry: Some(input),
            })
        };
        let frame = K::frame_id(&input);
        let content = K::content(&input);
        match self.check_owner(owner, Some(frame)) {
            Ok(true) => {}
            Ok(false) => return not_presented(PresentRejection::Cancelled, input),
            Err(fault) => return self.retain_failure(input, None, None, fault),
        }
        if let Err(fault) = self.drain_retirements(owner) {
            return self.retain_failure(input, None, None, fault);
        }
        let Some(geometry) = self.geometry else {
            return not_presented(PresentRejection::Suspended, input);
        };
        if geometry.extent.0 == 0 || geometry.extent.1 == 0 {
            return not_presented(PresentRejection::Suspended, input);
        }
        let extent = K::input_extent(&input).unwrap_or(geometry.extent);
        if extent != geometry.extent
            || self.configured != Some(extent)
            || frame.geometry_generation != geometry.geometry_generation
        {
            return not_presented(PresentRejection::StaleGeometry, input);
        }
        if let Err(message) = K::validate(&input, self.context, extent) {
            let fault = self.fault_at(Some(frame), NativeGpuFaultKind::IdentityMismatch, message);
            return self.retain_failure(input, None, None, fault);
        }
        let Some(sequence) = self.presents.checked_add(1) else {
            let fault = self.fault_at(
                Some(frame),
                NativeGpuFaultKind::SequenceExhausted,
                "present sequence exhausted",
            );
            return self.retain_failure(input, None, None, fault);
        };
        let (plan, needs) = match self.present_plan(frame, extent, K::source(&input)) {
            Ok(planned) => planned,
            Err(fault) => return self.retain_failure(input, None, None, fault),
        };
        let reservation = match owner.quote(&plan).and_then(|quote| owner.reserve(quote)) {
            Ok(reservation) => reservation,
            Err(decline) => {
                let fault = self.fault_at(
                    Some(frame),
                    NativeGpuFaultKind::StateMismatch,
                    &format!("mapping/present reservation declined: {decline:?}"),
                );
                return self.retain_failure(input, None, None, fault);
            }
        };

        // Physical boundary 1: authority/Stop/epoch immediately before acquire.
        if let Err(reason) = gate.before_acquire(&input) {
            return match owner.discard_unsubmitted_reservation(reservation) {
                Ok(()) => not_presented(reason, input),
                Err((fault, _reservation)) => self.retain_failure(input, None, None, fault),
            };
        }
        let acquired = match self.target.acquire() {
            Ok(acquired) => acquired,
            Err(error) => {
                if let Err((fault, _reservation)) =
                    owner.discard_unsubmitted_reservation(reservation)
                {
                    return self.retain_failure(input, None, None, fault);
                }
                return match error {
                    TargetAcquireError::Timeout => {
                        not_presented(PresentRejection::SurfaceTimeout, input)
                    }
                    TargetAcquireError::Outdated | TargetAcquireError::Lost => {
                        // Same admitted extent and device generation; I owns the
                        // single retry allowance of this input.
                        match self.reconfigure(owner, extent) {
                            Ok(()) => not_presented(PresentRejection::SurfaceLostOrOutdated, input),
                            Err(fault) => self.retain_failure(input, None, None, fault),
                        }
                    }
                    TargetAcquireError::OutOfMemory | TargetAcquireError::Other => {
                        let kind = if error == TargetAcquireError::OutOfMemory {
                            NativeGpuFaultKind::OutOfMemory
                        } else {
                            NativeGpuFaultKind::Submission
                        };
                        let fault = self.fault_at(Some(frame), kind, "surface acquire failed");
                        self.retain_failure(input, None, None, fault)
                    }
                };
            }
        };
        let view = self.target.view(&acquired);
        let mut txn = match owner.begin_frame(frame, reservation) {
            Ok(txn) => txn,
            Err(fault) => {
                self.target.discard(acquired);
                return self.retain_failure(input, None, None, fault);
            }
        };
        let mut phase = match txn.begin_phase(GpuPhase::MappingPresent) {
            Ok(phase) => phase,
            Err(fault) => {
                // No phase opened. Dropping the unfinished transaction retires
                // it in G's registry; poll_retirements finishes that record.
                self.target.discard(acquired);
                return self.retain_failure(input, None, None, fault);
            }
        };
        let encoded = match self.encode(&mut phase, K::source(&input), extent, needs, &view) {
            Ok(commands) => phase.finish(commands),
            Err(fault) => {
                self.mapped = None;
                self.inputs = None;
                self.cpu_source = None;
                phase.record_failure(fault.kind, fault.message.clone());
                let sealed = phase.finish(Vec::new());
                // If even the empty seal is refused, finish abandons the phase.
                let _ = txn.submit_phase(sealed);
                let pending = txn.finish();
                self.target.discard(acquired);
                return self.retain_failure(input, Some(pending), None, fault);
            }
        };
        drop(view);
        let receipt = match txn.submit_phase(encoded) {
            Ok(receipt) => receipt,
            Err(fault) => {
                let pending = txn.finish();
                self.target.discard(acquired);
                return self.retain_failure(input, Some(pending), None, fault);
            }
        };
        let submission = receipt.submission();
        let mut pending = Some(txn.finish());
        let mut receipt = Some(receipt);

        // Existing first-use validation-before-present (cold resources/format),
        // kept separately for each content kind.
        if !*self.validated.slot(content) {
            let first = match pending.as_mut() {
                Some(transaction) => await_first_use(owner, transaction, &mut receipt),
                None => FirstUse::Expired,
            };
            match first {
                FirstUse::Validated => {
                    pending = None;
                    *self.validated.slot(content) = true;
                }
                FirstUse::Failed(fault) => {
                    self.fault
                        .get_or_insert((fault.kind, fault.message.clone()));
                    self.target.discard(acquired);
                    return self.retain_failure(input, None, None, fault);
                }
                FirstUse::Cancelled => {
                    self.target.discard(acquired);
                    return not_presented(PresentRejection::Cancelled, input);
                }
                FirstUse::Expired => {
                    self.target.discard(acquired);
                    let fault = self.fault_at(
                        Some(frame),
                        NativeGpuFaultKind::ScopeDrain,
                        "first-use present validation did not complete; owners retained",
                    );
                    return self.retain_failure(input, pending, receipt, fault);
                }
            }
        }

        // Physical boundary 2: authority/Stop, then the device, before present.
        if let Err(reason) = gate.before_present(&input) {
            self.target.discard(acquired);
            self.retirements.push(PresentRetirement {
                pending,
                receipt,
                _ready: None,
            });
            return not_presented(reason, input);
        }
        match self.check_owner(owner, Some(frame)) {
            Ok(true) => {}
            Ok(false) => {
                self.target.discard(acquired);
                self.retirements.push(PresentRetirement {
                    pending,
                    receipt,
                    _ready: None,
                });
                return not_presented(PresentRejection::Cancelled, input);
            }
            Err(fault) => {
                self.target.discard(acquired);
                return self.retain_failure(input, pending, receipt, fault);
            }
        }
        self.target.present(acquired);
        self.presents = sequence;
        if content == NativePresentContent::HardBlackout {
            self.hard_blackouts = self.hard_blackouts.saturating_add(1);
        }
        self.retirements.push(PresentRetirement {
            pending,
            receipt,
            _ready: Some(Box::new(input)),
        });
        Ok(PresentOutcome::Presented(NativePresentReceipt {
            frame,
            content,
            submission,
            extent,
            sequence,
        }))
    }

    /// The exact new allocations of this present plus aliases of the pooled
    /// resources and a Ready's premapping source. Steady state is all alias,
    /// except one queue-upload charge per CPU Artistic present. A blackout
    /// plans nothing.
    fn present_plan(
        &self,
        frame: NativeFrameId,
        extent: (u32, u32),
        source: PresentSource<'_>,
    ) -> Result<(NativeAllocationPlan, PresentNeeds), NativeGpuFault> {
        let decline = |reason| {
            self.fault_at(
                Some(frame),
                NativeGpuFaultKind::StateMismatch,
                &format!("mapping/present plan declined: {reason:?}"),
            )
        };
        let mut plan = NativeAllocationPlan::new(frame);
        let mapping = match source {
            PresentSource::ExactBlack => {
                return Ok((
                    plan,
                    PresentNeeds {
                        mapped: false,
                        inputs: None,
                        cpu_source: false,
                    },
                ));
            }
            PresentSource::Premapping { mapping, .. } | PresentSource::Cpu { mapping, .. } => {
                mapping
            }
        };
        let bytes = packed_bytes(extent).ok_or_else(|| {
            decline(crate::native_gpu_contract::NativeGpuDecline::UnsupportedExtent)
        })?;
        let params_bytes = output_mapping_params(extent.0, extent.1, mapping);
        let mask_bytes = mapping_mask_texture_bytes(mapping);
        let mapped = self
            .mapped
            .as_ref()
            .filter(|mapped| mapped.extent == extent);
        let inputs = self.inputs.as_ref().filter(|inputs| {
            inputs.params_bytes == params_bytes && inputs.mask_bytes == mask_bytes
        });
        match mapped {
            Some(mapped) => mapped
                .ids()
                .into_iter()
                .for_each(|id| plan.add_existing(id)),
            None => {
                plan.add_new(ChargeKind::MappingBuffer, bytes)
                    .map_err(decline)?;
                plan.add_new(ChargeKind::MappingBuffer, DIMENSIONS_BYTES)
                    .map_err(decline)?;
            }
        }
        match inputs {
            Some(inputs) => inputs
                .ids()
                .into_iter()
                .for_each(|id| plan.add_existing(id)),
            None => {
                for bytes in [MAPPING_PARAMS_BYTES, MASK_TEXTURE_BYTES, MASK_STAGING_BYTES] {
                    plan.add_new(ChargeKind::MappingBuffer, bytes)
                        .map_err(decline)?;
                }
            }
        }
        let mut cpu_source = false;
        match source {
            PresentSource::Premapping { buffer, .. } => plan.add_existing(buffer.allocation_id()),
            PresentSource::Cpu { .. } => {
                match self
                    .cpu_source
                    .as_ref()
                    .filter(|source| source.extent == extent)
                {
                    Some(source) => plan.add_existing(source.id()),
                    None => {
                        plan.add_new(ChargeKind::CompositionBuffer, bytes)
                            .map_err(decline)?;
                        cpu_source = true;
                    }
                }
                plan.add_new(ChargeKind::QueueUploadStaging, bytes)
                    .map_err(decline)?;
            }
            PresentSource::ExactBlack => {}
        }
        Ok((
            plan,
            PresentNeeds {
                mapped: mapped.is_none(),
                inputs: inputs.is_none().then_some((params_bytes, mask_bytes)),
                cpu_source,
            },
        ))
    }

    /// One command buffer. Mapped content: (mask staging copy), clear, the
    /// single output mapping pass, then the existing buffer blit into the
    /// acquired image; a CPU Artistic frame is first written once into the
    /// pooled source. HardBlackout: only an exact-black clear of the image.
    /// Every charge is bound before its physical allocation or queue write;
    /// every read resource is retained with this submission.
    fn encode(
        &mut self,
        phase: &mut GpuPhaseEncoder<'_>,
        source: PresentSource<'_>,
        extent: (u32, u32),
        needs: PresentNeeds,
        view: &wgpu::TextureView,
    ) -> Result<Vec<wgpu::CommandBuffer>, NativeGpuFault> {
        let device = phase.device().clone();
        if let PresentSource::ExactBlack = source {
            return Ok(vec![encode_exact_black(&device, view)]);
        }
        let bytes = packed_bytes(extent).ok_or_else(|| {
            self.fault_at(
                None,
                NativeGpuFaultKind::StateMismatch,
                "present extent overflow",
            )
        })?;
        if needs.mapped {
            let mapped_charge = phase.bind_allocation(ChargeKind::MappingBuffer, bytes)?;
            let dimensions_charge =
                phase.bind_allocation(ChargeKind::MappingBuffer, DIMENSIONS_BYTES)?;
            let mapped = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Syndocal shared mapped output"),
                size: bytes,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            let dimensions = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal shared mapped output dimensions"),
                contents: &dimensions_uniform(extent.0, extent.1),
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let output_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Syndocal shared mapped output blit"),
                layout: &self.pipelines.output_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: mapped.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: dimensions.as_entire_binding(),
                    },
                ],
            });
            self.mapped = Some(Arc::new(MappedTarget {
                extent,
                mapped: Charged {
                    resource: mapped,
                    charge: mapped_charge,
                },
                dimensions: Charged {
                    resource: dimensions,
                    charge: dimensions_charge,
                },
                output_bind_group,
            }));
        }
        let mut staging = None;
        if let Some((params_bytes, mask_bytes)) = needs.inputs {
            let params_charge =
                phase.bind_allocation(ChargeKind::MappingBuffer, MAPPING_PARAMS_BYTES)?;
            let mask_charge =
                phase.bind_allocation(ChargeKind::MappingBuffer, MASK_TEXTURE_BYTES)?;
            let staging_charge =
                phase.bind_allocation(ChargeKind::MappingBuffer, MASK_STAGING_BYTES)?;
            let params = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal shared output mapping params"),
                contents: &params_bytes,
                usage: wgpu::BufferUsages::UNIFORM,
            });
            let mask = create_blank_mapping_mask_texture(
                &device,
                "Syndocal shared output mapping bitmap mask",
            );
            let staging_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Syndocal shared output mapping mask staging"),
                contents: &mask_bytes,
                usage: wgpu::BufferUsages::COPY_SRC,
            });
            let mask_view = mask.create_view(&wgpu::TextureViewDescriptor::default());
            self.inputs = Some(Arc::new(MappingInputs {
                params_bytes,
                mask_bytes,
                params: Charged {
                    resource: params,
                    charge: params_charge,
                },
                mask: Charged {
                    resource: mask,
                    charge: mask_charge,
                },
                mask_view,
            }));
            staging = Some(Charged {
                resource: staging_buffer,
                charge: staging_charge,
            });
        }
        let (Some(mapped), Some(inputs)) = (self.mapped.clone(), self.inputs.clone()) else {
            return Err(self.fault_at(
                None,
                NativeGpuFaultKind::StateMismatch,
                "pooled present resources are missing",
            ));
        };
        // The storage buffer the mapping pass reads, and its owner.
        let mut upload = None;
        let (premapping, cpu_source) = match source {
            PresentSource::Premapping { buffer, .. } => (Some(Arc::clone(buffer)), None),
            PresentSource::Cpu { frame, .. } => {
                if needs.cpu_source {
                    let charge = phase.bind_allocation(ChargeKind::CompositionBuffer, bytes)?;
                    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Syndocal shared CPU Artistic source"),
                        size: bytes,
                        usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    self.cpu_source = Some(Arc::new(CpuArtisticSource {
                        extent,
                        source: Charged {
                            resource: buffer,
                            charge,
                        },
                    }));
                }
                let Some(source) = self
                    .cpu_source
                    .clone()
                    .filter(|source| source.extent == extent)
                else {
                    return Err(self.fault_at(
                        None,
                        NativeGpuFaultKind::StateMismatch,
                        "pooled CPU Artistic source is missing",
                    ));
                };
                // One upload of the canonical frame. G accounts wgpu's staging
                // copy until this submission completes; the previous present
                // that read this pooled source was drained before planning.
                upload = Some(phase.bind_allocation(ChargeKind::QueueUploadStaging, bytes)?);
                phase
                    .queue()
                    .write_buffer(&source.source.resource, 0, &frame.data);
                phase.mark_queue_write();
                (None, Some(source))
            }
            PresentSource::ExactBlack => unreachable!("handled above"),
        };
        let source_buffer = match (&premapping, &cpu_source) {
            (Some(premapping), _) => premapping.buffer(),
            (None, Some(source)) => &source.source.resource,
            (None, None) => unreachable!("one mapping source"),
        };
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Syndocal shared output mapping"),
            layout: &self.pipelines.mapping_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: source_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: mapped.mapped.resource.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: inputs.params.resource.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&inputs.mask_view),
                },
            ],
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Syndocal shared mapping and present"),
        });
        if let Some(staging) = &staging {
            encoder.copy_buffer_to_texture(
                wgpu::TexelCopyBufferInfo {
                    buffer: &staging.resource,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(MAPPING_MASK_PADDED_BYTES_PER_ROW),
                        rows_per_image: Some(MAPPING_MASK_DIMENSION),
                    },
                },
                wgpu::TexelCopyTextureInfo {
                    texture: &inputs.mask.resource,
                    mip_level: 0,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                wgpu::Extent3d {
                    width: MAPPING_MASK_DIMENSION,
                    height: MAPPING_MASK_DIMENSION,
                    depth_or_array_layers: 1,
                },
            );
        }
        // Uncovered mapped pixels stay transparent black, as in the existing
        // presenter, whatever an earlier present left in the pooled buffer.
        encoder.clear_buffer(&mapped.mapped.resource, 0, Some(bytes));
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Syndocal shared output mapping pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&self.pipelines.mapping);
            pass.set_bind_group(0, &bind_group, &[]);
            let (workgroups_x, workgroups_y) = dispatch_dimensions(extent.0, extent.1);
            pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Syndocal shared mapped output blit"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipelines.output);
            pass.set_bind_group(0, &mapped.output_bind_group, &[]);
            pass.draw(0..3, 0..1);
        }
        let command = encoder.finish();
        if let Some(premapping) = premapping {
            let id = premapping.allocation_id();
            phase.retain_resource(premapping, &[id])?;
        }
        if let Some(source) = cpu_source {
            let id = source.id();
            phase.retain_resource(source, &[id])?;
        }
        let mapped_ids = mapped.ids();
        phase.retain_resource(mapped, &mapped_ids)?;
        let input_ids = inputs.ids();
        phase.retain_resource(inputs, &input_ids)?;
        let frame_ids: Vec<_> = staging
            .iter()
            .map(|staging| staging.charge.id())
            .chain(upload.iter().map(AllocationCharge::id))
            .collect();
        phase.retain_resource(
            Arc::new(PresentFrameResources {
                _bind_group: bind_group,
                _staging: staging,
                _upload: upload,
            }),
            &frame_ids,
        )?;
        phase.retain_resource(Arc::clone(&self.pipelines), &[])?;
        Ok(vec![command])
    }
}

fn packed_bytes((width, height): (u32, u32)) -> Option<u64> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .filter(|pixels| *pixels <= u64::from(u32::MAX))
        .and_then(|pixels| pixels.checked_mul(4))
}

/// The legacy hard-blackout bypass on the shared device: one render-pass
/// clear of the acquired image to opaque exact black `[0, 0, 0, 255]`. No
/// upload, no mapping pass, no stored content, so mapping values such as
/// `black_level` can never lift it.
fn encode_exact_black(device: &wgpu::Device, view: &wgpu::TextureView) -> wgpu::CommandBuffer {
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("Syndocal shared hard blackout"),
    });
    {
        let _pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Syndocal shared hard blackout clear"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(HARD_BLACKOUT_CLEAR_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    encoder.finish()
}

fn await_first_use(
    owner: &mut NativeGpuOwner,
    pending: &mut PendingGpuTransaction,
    receipt: &mut Option<PendingPhaseReceipt>,
) -> FirstUse {
    let started = Instant::now();
    loop {
        match owner.poll_transaction(pending, &|| false) {
            CompletionProgress::Pending if started.elapsed() < PRESENT_DEADLINE => {
                std::thread::yield_now()
            }
            CompletionProgress::Pending => return FirstUse::Expired,
            CompletionProgress::Cancelled => return FirstUse::Cancelled,
            CompletionProgress::Complete(Err(fault)) => return FirstUse::Failed(fault),
            CompletionProgress::Complete(Ok(mut resolved)) => {
                let Some(receipt) = receipt.take() else {
                    return FirstUse::Failed(NativeGpuFault {
                        context: owner.context().id(),
                        frame: None,
                        phase: GpuPhase::MappingPresent,
                        kind: NativeGpuFaultKind::StateMismatch,
                        message: "present phase receipt missing".into(),
                    });
                };
                return match resolved
                    .take_phase(receipt)
                    .and_then(|phase| phase.into_validation())
                {
                    Ok(()) => FirstUse::Validated,
                    Err(fault) => FirstUse::Failed(fault),
                };
            }
        }
    }
}

/// `NativePresentSink` over the shared presenter for one I present call.
/// `K` selects the entry: `ResidentContent` (default, `present_ready`),
/// `CpuArtisticContent` or `HardBlackoutContent`.
pub(crate) struct SharedPresentSink<'p, T: NativePresentTarget, R, P, K = ResidentContent> {
    presenter: &'p mut SharedSurfacePresenter<T>,
    owner: &'p mut NativeGpuOwner,
    _types: PhantomData<fn(R, P, K)>,
}

impl<T, R, P, K> NativePresentSink for SharedPresentSink<'_, T, R, P, K>
where
    T: NativePresentTarget,
    R: Send + 'static,
    P: NativePresentGate<R>,
    K: PresentContentKind<R>,
{
    type Ready = R;
    type Permit = P;
    type Receipt = NativePresentReceipt;

    fn present_once(
        &mut self,
        ready: R,
        permit: P,
    ) -> Result<PresentOutcome<R, NativePresentReceipt>, NativeGpuFault> {
        self.presenter
            .present_content::<K, R, P>(ready, &permit, self.owner)
    }
}

#[cfg(test)]
#[path = "gpu_surface_native_tests.rs"]
mod tests;
