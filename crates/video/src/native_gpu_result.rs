//! Affine ISF candidates and validated single-texture leases.
use crate::isf_native_gpu::PreparedNativeLayer;
use crate::isf_runtime::{IsfGpuImage, IsfGpuPipeline, StackBatch};
use crate::native_gpu_budget::{
    checked_upload_staging_bytes, AllocationCharge, AllocationPin, ChargeKind,
};
use crate::native_gpu_context::{
    NativeGpuFrameTxn, NativeGpuOwner, PendingPhaseReceipt, ResolvedGpuPhase,
};
use crate::native_gpu_contract::{
    FrozenChainClocks, GpuAllocationId, GpuPhase, GpuSubmissionId, LayerResultIdentity,
    MaterializationDescriptor, MaterializationFailure, NativeFrameId, NativeGpuFault,
    NativeGpuFaultKind, RetirementId,
};
use crate::{VideoEffectStageFault, VideoFrame, VideoRenderCancellation};
use std::sync::{mpsc, Arc};
use std::time::{Duration, Instant};

/// A CPU frame and its actual allocation charge share one lifetime. Native
/// canonical invocations alias this owner instead of charging a shared Arc twice.
#[derive(Debug)]
pub(crate) struct NativeCpuFrameOwner {
    pub(crate) frame: Arc<VideoFrame>,
    pub(crate) charge: Option<Arc<AllocationCharge>>,
}

#[derive(Debug)]
pub(crate) struct NativeCpuOutcome {
    owner: Arc<NativeCpuFrameOwner>,
    faults: Vec<VideoEffectStageFault>,
    scope: protocol::VideoEffectScope,
    clocks: Option<FrozenChainClocks>,
}
impl NativeCpuOutcome {
    pub(crate) fn new(
        owner: Arc<NativeCpuFrameOwner>,
        faults: Vec<VideoEffectStageFault>,
        scope: protocol::VideoEffectScope,
        clocks: Option<FrozenChainClocks>,
    ) -> Self {
        Self {
            owner,
            faults,
            scope,
            clocks,
        }
    }
    /// Active output consumers retain this whole owner. A raw frame Arc from
    /// the read-only accessor does not carry allocation accounting by itself.
    pub(crate) fn charged_owner(&self) -> Arc<NativeCpuFrameOwner> {
        Arc::clone(&self.owner)
    }
    pub(crate) fn frame(&self) -> &Arc<VideoFrame> {
        &self.owner.frame
    }
    pub(crate) fn faults(&self) -> &[VideoEffectStageFault] {
        &self.faults
    }
    pub(crate) fn scope(&self) -> &protocol::VideoEffectScope {
        &self.scope
    }
    pub(crate) fn clocks(&self) -> Option<FrozenChainClocks> {
        self.clocks
    }
    pub(crate) fn allocation_ids(&self) -> Vec<GpuAllocationId> {
        self.owner.charge.iter().map(|charge| charge.id()).collect()
    }
}

/// Ordinary canonical results do not mint historical materialization proofs or
/// invent Layer identities for Group, Composition or Output scopes.
#[derive(Debug)]
pub(crate) struct NativeCpuResults {
    outcomes: Vec<NativeCpuOutcome>,
}
impl NativeCpuResults {
    pub(crate) fn new(outcomes: Vec<NativeCpuOutcome>) -> Self {
        Self { outcomes }
    }
    pub(crate) fn outcomes(&self) -> &[NativeCpuOutcome] {
        &self.outcomes
    }
}

/// The charge lives exactly as long as this physical image or any GPU read.
pub(crate) struct NativeTextureAllocation {
    image: IsfGpuImage,
    charge: AllocationCharge,
}
impl NativeTextureAllocation {
    pub(crate) fn new(image: IsfGpuImage, charge: AllocationCharge) -> Self {
        Self { image, charge }
    }
    pub(crate) fn id(&self) -> GpuAllocationId {
        self.charge.id()
    }
}

pub(crate) struct EncodedLayerCandidate {
    identity: LayerResultIdentity,
    allocation: Arc<NativeTextureAllocation>,
    transaction: RetirementId,
    submission: GpuSubmissionId,
    encoded_frame: crate::native_gpu_contract::NativeFrameId,
    replay_receipt: Option<PendingPhaseReceipt>,
}
impl EncodedLayerCandidate {
    pub(crate) fn from_encoded(
        identity: LayerResultIdentity,
        allocation: Arc<NativeTextureAllocation>,
        transaction: RetirementId,
        submission: GpuSubmissionId,
        encoded_frame: crate::native_gpu_contract::NativeFrameId,
    ) -> Result<Self, NativeGpuFault> {
        if identity.context != allocation.id().context
            || identity.context != submission.context
            || transaction.context != identity.context
            || identity.width != allocation.image.width
            || identity.height != allocation.image.height
            || allocation.image.float
            || allocation.image.native_origin() != Some(identity.context)
            || allocation.image.texture.format() != wgpu::TextureFormat::Rgba8Unorm
            || identity.format != crate::VideoPixelFormat::Rgba8
        {
            return Err(identity_fault(
                identity,
                "encoded layer allocation identity mismatch",
            ));
        }
        Ok(Self {
            identity,
            allocation,
            transaction,
            submission,
            encoded_frame,
            replay_receipt: None,
        })
    }
    pub(crate) fn identity(&self) -> &LayerResultIdentity {
        &self.identity
    }
    pub(crate) fn pin_for_read(
        &self,
        txn: &mut NativeGpuFrameTxn<'_>,
    ) -> Result<GpuAllocationReadUse, NativeGpuFault> {
        if self.transaction != txn.id() || self.encoded_frame != txn.frame() {
            return Err(identity_fault(
                self.identity,
                "provisional candidate is private to its transaction",
            ));
        }
        pin_read(self.identity, &self.allocation, txn)
    }
    pub(crate) fn validate(
        self,
        phase: &ResolvedGpuPhase,
    ) -> Result<GpuLayerLease, NativeGpuFault> {
        if phase.context() != self.identity.context
            || phase.frame() != self.encoded_frame
            || phase.transaction() != self.transaction
            || phase.submission() != self.submission
            || !matches!(
                phase.phase(),
                GpuPhase::IsfBatch | GpuPhase::HistoricalReplay
            )
        {
            return Err(identity_fault(
                self.identity,
                "candidate completion does not match its ISF submission",
            ));
        }
        if let Err(fault) = phase.validation() {
            return Err(copy_fault(fault));
        }
        Ok(GpuLayerLease {
            identity: self.identity,
            allocation: self.allocation,
        })
    }
    pub(crate) fn attach_replay_receipt(&mut self, receipt: PendingPhaseReceipt) {
        self.replay_receipt = Some(receipt);
    }
    pub(crate) fn take_replay_receipt(&mut self) -> Result<PendingPhaseReceipt, NativeGpuFault> {
        self.replay_receipt.take().ok_or_else(|| {
            identity_fault(self.identity, "candidate has no unconsumed replay receipt")
        })
    }
}

/// Non-Clone. No workspace/input/ping-pong/snapshot is reachable from a lease.
pub(crate) struct GpuLayerLease {
    identity: LayerResultIdentity,
    allocation: Arc<NativeTextureAllocation>,
}
impl GpuLayerLease {
    pub(crate) fn identity(&self) -> &LayerResultIdentity {
        &self.identity
    }
    pub(crate) fn allocation_id(&self) -> GpuAllocationId {
        self.allocation.id()
    }
    pub(crate) fn pin_for_read(
        &self,
        txn: &mut NativeGpuFrameTxn<'_>,
    ) -> Result<GpuAllocationReadUse, NativeGpuFault> {
        pin_read(self.identity, &self.allocation, txn)
    }
}
pub(crate) struct GpuAllocationReadUse {
    allocation: Arc<NativeTextureAllocation>,
    pin: AllocationPin,
}
impl GpuAllocationReadUse {
    pub(crate) fn id(&self) -> GpuAllocationId {
        self.pin.id()
    }
    pub(crate) fn view(&self) -> &wgpu::TextureView {
        &self.allocation.image.view
    }
    pub(crate) fn texture(&self) -> &wgpu::Texture {
        &self.allocation.image.texture
    }
}
fn pin_read(
    identity: LayerResultIdentity,
    allocation: &Arc<NativeTextureAllocation>,
    txn: &mut NativeGpuFrameTxn<'_>,
) -> Result<GpuAllocationReadUse, NativeGpuFault> {
    if identity.context != txn.context().id() {
        return Err(identity_fault(
            identity,
            "layer read belongs to a different device generation",
        ));
    }
    txn.retain_resource(Arc::clone(allocation), &[allocation.id()])?;
    Ok(GpuAllocationReadUse {
        pin: allocation.charge.pin(),
        allocation: Arc::clone(allocation),
    })
}

#[derive(Clone)]
pub(crate) struct NativeRecipeBindings {
    pipelines: Vec<Arc<IsfGpuPipeline>>,
    sampler: wgpu::Sampler,
}
impl NativeRecipeBindings {
    pub(crate) fn new(pipelines: Vec<Arc<IsfGpuPipeline>>, sampler: wgpu::Sampler) -> Self {
        Self { pipelines, sampler }
    }
}

pub(crate) struct FrozenStatelessRecipe {
    identity: LayerResultIdentity,
    job: Arc<PreparedNativeLayer>,
    bindings: NativeRecipeBindings,
}
impl FrozenStatelessRecipe {
    pub(crate) fn from_validated(
        lease: &GpuLayerLease,
        job: Arc<PreparedNativeLayer>,
        bindings: NativeRecipeBindings,
    ) -> Self {
        Self {
            identity: *lease.identity(),
            job,
            bindings,
        }
    }
    pub(crate) fn identity(&self) -> &LayerResultIdentity {
        &self.identity
    }
    pub(crate) fn clocks(&self) -> FrozenChainClocks {
        match self.identity.origin {
            crate::native_gpu_contract::LayerOrigin::Executed(clocks) => clocks,
            crate::native_gpu_contract::LayerOrigin::ImportedCpu => {
                unreachable!("recipes describe executed chains")
            }
        }
    }
    pub(crate) fn job(&self) -> &Arc<PreparedNativeLayer> {
        &self.job
    }
    pub(crate) fn pinned_pipeline_count(&self) -> usize {
        self.bindings.pipelines.len()
    }
    pub(crate) fn sampler(&self) -> &wgpu::Sampler {
        &self.bindings.sampler
    }
}

pub(crate) struct PendingIsfReceipt {
    receipt: Option<PendingPhaseReceipt>,
    submission: GpuSubmissionId,
}
impl PendingIsfReceipt {
    pub(crate) fn new(receipt: PendingPhaseReceipt) -> Self {
        Self {
            submission: receipt.submission(),
            receipt: Some(receipt),
        }
    }
    pub(crate) fn take(&mut self) -> Result<PendingPhaseReceipt, NativeGpuFault> {
        self.receipt.take().ok_or_else(|| NativeGpuFault {
            context: self.submission.context,
            frame: None,
            phase: GpuPhase::IsfBatch,
            kind: NativeGpuFaultKind::StateMismatch,
            message: "ISF receipt already consumed".into(),
        })
    }
    pub(crate) fn submission(&self) -> GpuSubmissionId {
        self.submission
    }
}
pub(crate) struct IsfValidatedReceipt {
    phase: ResolvedGpuPhase,
}
impl IsfValidatedReceipt {
    pub(crate) fn from_phase(phase: ResolvedGpuPhase) -> Result<Self, NativeGpuFault> {
        if let Err(fault) = phase.validation() {
            return Err(copy_fault(fault));
        }
        if phase.phase() != GpuPhase::IsfBatch {
            return Err(NativeGpuFault {
                context: phase.context(),
                frame: Some(phase.frame()),
                phase: phase.phase(),
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "ISF history requires an ISF phase receipt".into(),
            });
        }
        Ok(Self { phase })
    }
    pub(crate) fn phase(&self) -> &ResolvedGpuPhase {
        &self.phase
    }
}
pub(crate) struct ValidatedLayerResult {
    pub identity: LayerResultIdentity,
    pub lease: GpuLayerLease,
    pub recipe: FrozenStatelessRecipe,
}
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CpuOutcomeProvenance {
    FixedClockRecovery,
}
pub(crate) struct CanonicalCpuLayerOutcome {
    pub identity: LayerResultIdentity,
    pub frame: VideoFrame,
    pub faults: Vec<VideoEffectStageFault>,
    pub provenance: CpuOutcomeProvenance,
    pub(crate) charge: AllocationCharge,
}
pub(crate) struct EncodedIsfBatch {
    pub candidates: StackBatch<EncodedLayerCandidate>,
    pub pending: PendingIsfReceipt,
    pub(crate) jobs: StackBatch<Arc<PreparedNativeLayer>>,
    pub(crate) clocks: StackBatch<FrozenChainClocks>,
    pub(crate) bindings: StackBatch<NativeRecipeBindings>,
    pub(crate) inputs: StackBatch<Arc<NativeInputAllocation>>,
}
pub(crate) struct NativeInputAllocation {
    pub(crate) job: Arc<PreparedNativeLayer>,
    pub(crate) charge: AllocationCharge,
}
pub(crate) enum NativeIsfCompletion {
    Validated {
        layers: StackBatch<ValidatedLayerResult>,
        receipt: IsfValidatedReceipt,
    },
    RecoveredCpu {
        layers: StackBatch<CanonicalCpuLayerOutcome>,
    },
    Cancelled {
        retirement: Option<RetirementId>,
    },
    Failed {
        fault: NativeGpuFault,
        retirement: Option<RetirementId>,
    },
}

struct CpuReadbackAllocation {
    buffer: wgpu::Buffer,
    charge: AllocationCharge,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CpuReadbackMapState {
    Pending,
    Mapped,
    Unmapped,
}

/// Shared native readback lifecycle. The caller must retain its charged buffer
/// owner through GPU completion; this guard owns mapping cleanup, not retirement.
/// Device polling remains serialized by NativeGpuOwner and stays outside here.
pub(crate) struct NativeReadbackMap {
    buffer: wgpu::Buffer,
    receiver: mpsc::Receiver<Result<(), wgpu::BufferAsyncError>>,
    state: CpuReadbackMapState,
}

impl NativeReadbackMap {
    pub(crate) fn start(buffer: &wgpu::Buffer) -> Self {
        let (sender, receiver) = mpsc::sync_channel(1);
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                let _ = sender.send(result);
            });
        Self {
            buffer: buffer.clone(),
            receiver,
            state: CpuReadbackMapState::Pending,
        }
    }

    pub(crate) fn try_complete(&mut self) -> Result<Option<()>, String> {
        match self.state {
            CpuReadbackMapState::Mapped => return Ok(Some(())),
            CpuReadbackMapState::Unmapped => return Err("native map has already ended".into()),
            CpuReadbackMapState::Pending => {}
        }
        match self.receiver.try_recv() {
            Ok(Ok(())) => {
                self.state = CpuReadbackMapState::Mapped;
                Ok(Some(()))
            }
            Ok(Err(error)) => {
                // Failed map requests are already Idle. Calling unmap here
                // would create a second, uncaptured NotMapped device error.
                self.state = CpuReadbackMapState::Unmapped;
                Err(format!("native map failed: {error}"))
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                self.state = CpuReadbackMapState::Unmapped;
                Err("native map callback disconnected".into())
            }
            Err(mpsc::TryRecvError::Empty) => Ok(None),
        }
    }

    pub(crate) fn unmap(&mut self) {
        if self.state == CpuReadbackMapState::Pending {
            // Cancellation/validation rejection can leave a delivered callback
            // unread; observe it before deciding whether cleanup owns a map.
            let _ = self.try_complete();
        }
        if self.state != CpuReadbackMapState::Unmapped {
            self.state = CpuReadbackMapState::Unmapped;
            self.buffer.unmap();
        }
    }
}

impl Drop for NativeReadbackMap {
    fn drop(&mut self) {
        self.unmap();
    }
}

/// A private readback is created, copied and submitted here, so metadata and a
/// successful unrelated phase cannot mint a CPU copy proof. The broker owns the
/// source and destination until GPU completion; this affine token also owns them
/// through map/copy/unmap. Dropping it cancels mapping, never GPU retirement.
pub(crate) struct PendingCpuMaterialization {
    identity: LayerResultIdentity,
    frame: NativeFrameId,
    transaction: RetirementId,
    submission: GpuSubmissionId,
    source: GpuAllocationReadUse,
    map: NativeReadbackMap,
    readback: Arc<CpuReadbackAllocation>,
    cpu_charge: Option<AllocationCharge>,
}

impl PendingCpuMaterialization {
    /// The caller's existing reservation includes this lease alias, padded map
    /// buffer and exact CPU capacity. Any error after admission must retire the
    /// caller's transaction; no successful token is minted on an encoding error.
    pub(crate) fn encode(
        txn: &mut NativeGpuFrameTxn<'_>,
        lease: &GpuLayerLease,
    ) -> Result<(Self, PendingPhaseReceipt), NativeGpuFault> {
        let identity = *lease.identity();
        let frame = txn.frame();
        let transaction = txn.id();
        let bytes = u64::from(identity.width)
            .checked_mul(u64::from(identity.height))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes != 0 && usize::try_from(*bytes).is_ok())
            .ok_or_else(|| {
                cpu_copy_fault(
                    identity,
                    frame,
                    NativeGpuFaultKind::IdentityMismatch,
                    "invalid CPU copy extent",
                )
            })?;
        let padded = checked_upload_staging_bytes(identity.width, identity.height, Some(256))
            .map_err(|_| {
                cpu_copy_fault(
                    identity,
                    frame,
                    NativeGpuFaultKind::IdentityMismatch,
                    "invalid CPU copy pitch",
                )
            })?;
        if identity.format != crate::VideoPixelFormat::Rgba8
            || lease.allocation.charge.kind() != ChargeKind::ResidentLayer
            || lease.allocation.charge.capacity() < bytes
        {
            return Err(cpu_copy_fault(
                identity,
                frame,
                NativeGpuFaultKind::IdentityMismatch,
                "CPU copy source is not a charged RGBA resident layer",
            ));
        }
        let source = lease.pin_for_read(txn)?;
        let mut phase = txn.begin_phase(GpuPhase::HistoricalReplay)?;
        let map_charge = phase.bind_allocation(ChargeKind::SnapshotReadback, padded)?;
        let cpu_charge = phase.bind_allocation(ChargeKind::CpuSnapshot, bytes)?;
        let buffer = phase.device().create_buffer(&wgpu::BufferDescriptor {
            label: Some("Syndocal exact historical layer readback"),
            size: padded,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let readback = Arc::new(CpuReadbackAllocation {
            buffer,
            charge: map_charge,
        });
        phase.retain_resource(Arc::clone(&readback), &[readback.charge.id()])?;
        let mut encoder = phase
            .device()
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Syndocal exact historical layer copy"),
            });
        encoder.copy_texture_to_buffer(
            wgpu::TexelCopyTextureInfo {
                texture: source.texture(),
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            wgpu::TexelCopyBufferInfo {
                buffer: &readback.buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some((identity.width * 4).div_ceil(256) * 256),
                    rows_per_image: Some(identity.height),
                },
            },
            wgpu::Extent3d {
                width: identity.width,
                height: identity.height,
                depth_or_array_layers: 1,
            },
        );
        let encoded = phase.finish(vec![encoder.finish()]);
        let receipt = txn.submit_phase(encoded)?;
        let submission = receipt.submission();
        let map = NativeReadbackMap::start(&readback.buffer);
        Ok((
            Self {
                identity,
                frame,
                transaction,
                submission,
                source,
                map,
                readback,
                cpu_charge: Some(cpu_charge),
            },
            receipt,
        ))
    }

    /// Consumes the exact submitted copy and its successful scope completion.
    /// Callers cannot supply replacement pixels, metadata, buffer or charge.
    pub(crate) fn complete(
        mut self,
        phase: ResolvedGpuPhase,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ValidatedCpuMaterialization, MaterializationFailure> {
        self.check_publication(owner, cancel)?;
        let charge = self
            .cpu_charge
            .as_ref()
            .expect("affine CPU copy owns its charge");
        let cpu_capacity = charge.capacity();
        let pixels = u64::from(self.identity.width) * u64::from(self.identity.height) * 4;
        if phase.phase() != GpuPhase::HistoricalReplay
            || phase.context() != self.identity.context
            || phase.frame() != self.frame
            || phase.transaction() != self.transaction
            || phase.submission() != self.submission
            || owner.context().id() != self.identity.context
            || self.source.id().context != self.identity.context
            || charge.id().context != self.identity.context
            || charge.kind() != ChargeKind::CpuSnapshot
            || cpu_capacity < pixels
        {
            return Err(cpu_copy_failure(cpu_copy_fault(
                self.identity,
                self.frame,
                NativeGpuFaultKind::IdentityMismatch,
                "CPU copy completion/charge is not the submitted copy",
            )));
        }
        phase.into_validation().map_err(cpu_copy_failure)?;
        let started = Instant::now();
        loop {
            self.check_publication(owner, cancel)?;
            match self.map.try_complete() {
                Ok(Some(())) => break,
                Err(error) => {
                    return Err(cpu_copy_failure(cpu_copy_fault(
                        self.identity,
                        self.frame,
                        NativeGpuFaultKind::Map,
                        error,
                    )))
                }
                Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                    owner.poll_retirement().map_err(cpu_copy_failure)?;
                    std::thread::yield_now();
                }
                Ok(None) => {
                    return Err(cpu_copy_failure(cpu_copy_fault(
                        self.identity,
                        self.frame,
                        NativeGpuFaultKind::Map,
                        "native map callback deadline expired",
                    )))
                }
            }
        }
        self.check_publication(owner, cancel)?;
        let row = self.identity.width as usize * 4;
        let pitch = row.div_ceil(256) * 256;
        let mapped = self.readback.buffer.slice(..).get_mapped_range();
        if mapped.len() < pitch * self.identity.height as usize {
            return Err(cpu_copy_failure(cpu_copy_fault(
                self.identity,
                self.frame,
                NativeGpuFaultKind::SnapshotCopy,
                "mapped CPU copy is shorter than its extent",
            )));
        }
        let mut data = Vec::with_capacity(pixels as usize);
        if row == pitch {
            data.extend_from_slice(&mapped[..pixels as usize]);
        } else {
            for source in mapped
                .chunks_exact(pitch)
                .take(self.identity.height as usize)
            {
                data.extend_from_slice(&source[..row]);
            }
        }
        drop(mapped);
        self.map.unmap();
        if data.len() as u64 != pixels || data.capacity() as u64 > cpu_capacity {
            return Err(cpu_copy_failure(cpu_copy_fault(
                self.identity,
                self.frame,
                NativeGpuFaultKind::SnapshotCopy,
                "retained CPU copy exceeds its charged capacity",
            )));
        }
        self.check_publication(owner, cancel)?;
        let frame = VideoFrame {
            layer_id: self.identity.layer_id,
            width: self.identity.width,
            height: self.identity.height,
            pts_ms: self.identity.pts_ms,
            duration_ms: self.identity.duration_ms,
            format: self.identity.format,
            data,
        };
        Ok(ValidatedCpuMaterialization {
            frame,
            descriptor: MaterializationDescriptor {
                layer: self.identity,
                allocation: self.source.id(),
                completed_submission: self.submission,
            },
            charge: self
                .cpu_charge
                .take()
                .expect("validated CPU copy owns its charge"),
        })
    }

    fn check_publication(
        &self,
        owner: &NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<(), MaterializationFailure> {
        if cancel.is_cancelled() {
            return Err(MaterializationFailure::Cancelled { retirement: None });
        }
        crate::isf_native_gpu::ensure_publication_active(
            owner,
            self.frame,
            GpuPhase::HistoricalReplay,
        )
    }
}

fn cpu_copy_fault(
    identity: LayerResultIdentity,
    frame: NativeFrameId,
    kind: NativeGpuFaultKind,
    message: impl Into<String>,
) -> NativeGpuFault {
    NativeGpuFault {
        context: identity.context,
        frame: Some(frame),
        phase: GpuPhase::HistoricalReplay,
        kind,
        message: message.into(),
    }
}
fn cpu_copy_failure(fault: NativeGpuFault) -> MaterializationFailure {
    MaterializationFailure::Fault {
        fault,
        retirement: None,
    }
}

/// Real private-buffer copy/unmap proof awaiting I's C1 ExactCpuLayer mint
/// adapter. Only PendingCpuMaterialization::complete constructs this value.
pub(crate) struct ValidatedCpuMaterialization {
    frame: VideoFrame,
    descriptor: MaterializationDescriptor,
    charge: AllocationCharge,
}
impl ValidatedCpuMaterialization {
    pub(crate) fn frame(&self) -> &VideoFrame {
        &self.frame
    }
    pub(crate) fn descriptor(&self) -> &MaterializationDescriptor {
        &self.descriptor
    }
    pub(crate) fn into_parts(self) -> (VideoFrame, MaterializationDescriptor, AllocationCharge) {
        (self.frame, self.descriptor, self.charge)
    }
}
pub(crate) fn copy_fault(fault: &NativeGpuFault) -> NativeGpuFault {
    NativeGpuFault {
        context: fault.context,
        frame: fault.frame,
        phase: fault.phase,
        kind: fault.kind,
        message: fault.message.clone(),
    }
}
fn identity_fault(identity: LayerResultIdentity, message: &str) -> NativeGpuFault {
    NativeGpuFault {
        context: identity.context,
        frame: Some(identity.frame),
        phase: GpuPhase::IsfBatch,
        kind: NativeGpuFaultKind::IdentityMismatch,
        message: message.into(),
    }
}
#[cfg(test)]
#[path = "native_gpu_result_tests.rs"]
mod tests;
