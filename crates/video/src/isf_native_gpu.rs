//! Resident ISF execution through the canonical encoder and G's mutable broker.
//! The C0b module gate leaves production routing unchanged until I's C1 join.
use crate::isf_chain_execution::PreparedChain;
use crate::isf_runtime::{
    IsfGpuRuntime, IsfInstanceKey, IsfStageInputs, NativeStackResources, StackBatch,
};
use crate::native_gpu_budget::{
    checked_upload_staging_bytes, AllocationCharge, BudgetQuote, ChargeKind, NativeAllocationPlan,
};
use crate::native_gpu_context::{
    NativeGpuFrameTxn, NativeGpuOwner, PendingGpuTransaction, ResolvedGpuPhase,
    ResolvedGpuTransaction,
};
use crate::native_gpu_contract::*;
use crate::native_gpu_result::*;
use crate::{IsfRuntimeError, VideoFrame, VideoPixelFormat, VideoRenderCancellation};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

pub(crate) struct PreparedNativeLayer {
    pub input: Arc<VideoFrame>,
    pub chain: PreparedChain<'static>,
    pub entry: HistoryEntryId,
    pub source_generation: u64,
    pub content_sequence: u64,
    pub date: [f32; 4],
}
impl PreparedNativeLayer {
    fn stages(&self) -> Vec<IsfStageInputs<'_>> {
        self.chain
            .stages
            .iter()
            .map(|stage| IsfStageInputs {
                shader: &stage.shader,
                controls: &stage.controls,
                images: &[],
            })
            .collect()
    }
    fn clocks(&self, frame_index: u32) -> FrozenChainClocks {
        FrozenChainClocks {
            frame_index,
            time_seconds_bits: (self.input.pts_ms as f32 / 1000.0).to_bits(),
            time_delta_seconds_bits: (self.input.duration_ms as f32 / 1000.0).to_bits(),
            date_bits: self.date.map(f32::to_bits),
        }
    }
    fn identity(
        &self,
        context: GpuContextId,
        frame: NativeFrameId,
        clocks: FrozenChainClocks,
    ) -> LayerResultIdentity {
        LayerResultIdentity {
            context,
            frame,
            entry: self.entry,
            layer_id: self.input.layer_id,
            source_generation: self.source_generation,
            content_sequence: self.content_sequence,
            pts_ms: self.input.pts_ms,
            duration_ms: self.input.duration_ms,
            width: self.input.width,
            height: self.input.height,
            format: self.input.format,
            origin: LayerOrigin::Executed(clocks),
        }
    }
}
#[derive(Default)]
pub(crate) struct NativeBatchRestrictions {
    pub executing_clip_or_transition: bool,
    pub inter_layer_dependency: bool,
    pub callback: bool,
}
pub(crate) struct PreparedNativeInputs {
    frame: NativeFrameId,
    context: GpuContextId,
    jobs: StackBatch<Arc<PreparedNativeLayer>>,
    plan: Result<NativeAllocationPlan, NativeGpuDecline>,
}
impl PreparedNativeInputs {
    pub(crate) fn allocation_plan(&self) -> Result<&NativeAllocationPlan, NativeGpuDecline> {
        self.plan.as_ref().map_err(|reason| *reason)
    }
    pub(crate) fn into_layers(self) -> StackBatch<Arc<PreparedNativeLayer>> {
        self.jobs
    }
}
pub(crate) struct PreparedNativeBatch {
    inputs: PreparedNativeInputs,
}
pub(crate) enum NativeBatchAdmission {
    Accepted(PreparedNativeBatch),
    Declined {
        reason: NativeGpuDecline,
        prepared: PreparedNativeInputs,
    },
}
struct ScratchOwner {
    _resources: Arc<NativeStackResources>,
    charges: Vec<AllocationCharge>,
    _input: Arc<PreparedNativeLayer>,
    final_image: Arc<NativeTextureAllocation>,
}
impl ScratchOwner {
    fn allocation_ids(&self) -> Vec<GpuAllocationId> {
        let mut ids: Vec<_> = self.charges.iter().map(AllocationCharge::id).collect();
        ids.push(self.final_image.id());
        ids
    }
}
const UNIFORM_BYTES: u64 = 48 + crate::isf_runtime::ISF_MAX_CONTROL_INPUTS as u64 * 16;
fn unsupported(reason: UnsupportedNativeFx) -> NativeGpuDecline {
    NativeGpuDecline::UnsupportedChain { reason }
}
fn fault(
    context: GpuContextId,
    frame: NativeFrameId,
    phase: GpuPhase,
    kind: NativeGpuFaultKind,
    message: impl Into<String>,
) -> NativeGpuFault {
    NativeGpuFault {
        context,
        frame: Some(frame),
        phase,
        kind,
        message: message.into(),
    }
}
fn rgba_bytes(width: u32, height: u32) -> Result<u64, NativeGpuDecline> {
    if width == 0 || height == 0 {
        return Err(NativeGpuDecline::UnsupportedExtent);
    }
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|v| v.checked_mul(4))
        .ok_or(NativeGpuDecline::UnsupportedExtent)
}
fn add_stack_plan(
    plan: &mut NativeAllocationPlan,
    input: &VideoFrame,
    stage_count: usize,
    pitch: Option<u64>,
    readback: bool,
    existing_input: Option<GpuAllocationId>,
) -> Result<(), NativeGpuDecline> {
    let pixels = rgba_bytes(input.width, input.height)?;
    plan.add_new(ChargeKind::IsfScratch, pixels)?;
    plan.add_new(ChargeKind::IsfScratch, pixels)?;
    plan.add_new(ChargeKind::ResidentLayer, pixels)?;
    if let Some(id) = existing_input {
        plan.add_existing(id);
    } else {
        plan.add_new(ChargeKind::ActiveCpuInput, input.data.capacity() as u64)?;
    }
    plan.add_new(
        ChargeKind::QueueUploadStaging,
        checked_upload_staging_bytes(input.width, input.height, pitch)?,
    )?;
    for _ in 0..stage_count {
        plan.add_new(ChargeKind::CachedEffectResource, UNIFORM_BYTES)?;
    }
    if readback {
        plan.add_new(
            ChargeKind::SnapshotReadback,
            checked_upload_staging_bytes(input.width, input.height, Some(256))?,
        )?;
        plan.add_new(ChargeKind::CpuSnapshot, pixels)?;
    }
    Ok(())
}

impl IsfGpuRuntime {
    pub(crate) fn prepare_native_inputs(
        &self,
        frame: NativeFrameId,
        jobs: StackBatch<PreparedNativeLayer>,
        restrictions: NativeBatchRestrictions,
    ) -> Result<PreparedNativeInputs, NativeGpuFault> {
        let context = self.native_context().ok_or_else(|| {
            fault(
                GpuContextId {
                    owner_id: frame.output_id,
                    device_generation: 0,
                },
                frame,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "native batch requires shared runtime",
            )
        })?;
        let jobs = jobs.map(Arc::new);
        for job in &*jobs {
            if job.entry.input.project_render_epoch != frame.project_epoch
                || job.chain.keys.len() != job.chain.stages.len()
                || job
                    .chain
                    .keys
                    .iter()
                    .any(|key| key.layer_id != job.input.layer_id)
                || matches!(job.chain.source.scope, protocol::VideoEffectScope::Layer { layer_id } if layer_id != job.input.layer_id)
            {
                return Err(fault(
                    context.id(),
                    frame,
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::IdentityMismatch,
                    "prepared layer identity does not match its input/epoch/keys",
                ));
            }
            if job.input.format == VideoPixelFormat::Rgba8
                && rgba_bytes(job.input.width, job.input.height)
                    .is_ok_and(|bytes| bytes != job.input.data.len() as u64)
            {
                return Err(fault(
                    context.id(),
                    frame,
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::Validation,
                    "prepared RGBA byte length is invalid",
                ));
            }
        }
        let plan = (|| {
            if !(2..=3).contains(&jobs.len()) {
                return Err(unsupported(UnsupportedNativeFx::UnsupportedJobCount));
            }
            if restrictions.executing_clip_or_transition {
                return Err(unsupported(UnsupportedNativeFx::ClipOrTransitionExecution));
            }
            if restrictions.inter_layer_dependency {
                return Err(unsupported(UnsupportedNativeFx::InterLayerDependency));
            }
            if restrictions.callback {
                return Err(unsupported(UnsupportedNativeFx::Callback));
            }
            let mut plan = NativeAllocationPlan::new(frame);
            for (index, job) in jobs.iter().enumerate() {
                validate_job(job, context.capabilities().limits.max_texture_dimension_2d)?;
                if jobs[..index].iter().any(|old| {
                    old.input.layer_id == job.input.layer_id
                        || old.entry == job.entry
                        || old
                            .chain
                            .keys
                            .iter()
                            .any(|key| job.chain.keys.contains(key))
                }) {
                    return Err(unsupported(UnsupportedNativeFx::InterLayerDependency));
                }
                add_stack_plan(
                    &mut plan,
                    &job.input,
                    job.chain.stages.len(),
                    context.capabilities().upload_row_pitch,
                    false,
                    None,
                )?;
            }
            Ok(plan)
        })();
        Ok(PreparedNativeInputs {
            frame,
            context: context.id(),
            jobs,
            plan,
        })
    }

    pub(crate) fn admit_native_batch(
        &self,
        inputs: PreparedNativeInputs,
        quote: &BudgetQuote,
    ) -> NativeBatchAdmission {
        let reason = match &inputs.plan {
            Err(reason) => Some(*reason),
            Ok(plan) if quote.context() != inputs.context || !quote.covers_plan(plan) => {
                Some(NativeGpuDecline::QuoteExpired)
            }
            Ok(_) => None,
        };
        match reason {
            Some(reason) => NativeBatchAdmission::Declined {
                reason,
                prepared: inputs,
            },
            None => NativeBatchAdmission::Accepted(PreparedNativeBatch { inputs }),
        }
    }

    pub(crate) fn encode_native_batch(
        &mut self,
        txn: &mut NativeGpuFrameTxn<'_>,
        batch: PreparedNativeBatch,
    ) -> Result<EncodedIsfBatch, NativeGpuFault> {
        let inputs = batch.inputs;
        self.check_native_transaction(txn, inputs.frame)?;
        txn.validate_plan(inputs.plan.as_ref().expect("admitted allocation plan"))?;
        if inputs.context != txn.context().id() {
            return Err(fault(
                inputs.context,
                inputs.frame,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::IdentityMismatch,
                "batch device identity changed",
            ));
        }
        let transaction = txn.id();
        let context = txn.context().id();
        let logical = self.reserve_independent_clocks(inputs.jobs.len());
        let clocks = StackBatch::from_fn(inputs.jobs.len(), |index| {
            inputs.jobs[index].clocks(logical.at(index))
        });
        let mut phase = txn.begin_phase(GpuPhase::IsfBatch)?;
        let mut managed_inputs = StackBatch::Empty;
        for job in &*inputs.jobs {
            let charge = phase
                .bind_allocation(ChargeKind::ActiveCpuInput, job.input.data.capacity() as u64)?;
            let managed = Arc::new(NativeInputAllocation {
                job: Arc::clone(job),
                charge,
            });
            phase.retain_resource(Arc::clone(&managed), &[managed.charge.id()])?;
            managed_inputs.push(managed);
        }
        // Once admitted into the phase, every original chain consumes exactly
        // its canonical index even if encoding fails. Recovery/replay consume 0.
        self.commit_independent_clocks(logical);
        let mut final_images = StackBatch::Empty;
        let mut bindings = StackBatch::Empty;
        let mut commands = Vec::with_capacity(inputs.jobs.len());
        let mut failed = false;
        for (index, job) in inputs.jobs.iter().enumerate() {
            let result = self.encode_resident_job(
                &mut phase,
                job,
                clocks[index],
                inputs.frame,
                Some(&managed_inputs[index]),
                None,
            );
            match result {
                Ok((command, image, pins)) => {
                    commands.push(command);
                    final_images.push(image);
                    bindings.push(pins);
                }
                Err(error) => {
                    phase.record_failure(error.kind, error.message);
                    failed = true;
                    break;
                }
            }
        }
        let encoded = phase.finish(commands);
        let receipt = txn.submit_phase(encoded)?;
        let submission = receipt.submission();
        let candidates = if failed {
            StackBatch::Empty
        } else {
            let mut candidates = StackBatch::Empty;
            for (index, image) in final_images.into_iter().enumerate() {
                candidates.push(EncodedLayerCandidate::from_encoded(
                    inputs.jobs[index].identity(context, inputs.frame, clocks[index]),
                    image,
                    transaction,
                    submission,
                    inputs.frame,
                )?);
            }
            candidates
        };
        Ok(EncodedIsfBatch {
            candidates,
            pending: PendingIsfReceipt::new(receipt),
            jobs: inputs.jobs,
            clocks,
            bindings,
            inputs: managed_inputs,
        })
    }

    fn encode_resident_job(
        &mut self,
        phase: &mut crate::native_gpu_context::GpuPhaseEncoder<'_>,
        job: &Arc<PreparedNativeLayer>,
        clocks: FrozenChainClocks,
        request: NativeFrameId,
        managed_input: Option<&Arc<NativeInputAllocation>>,
        frozen_sampler: Option<&wgpu::Sampler>,
    ) -> Result<
        (
            wgpu::CommandBuffer,
            Arc<NativeTextureAllocation>,
            NativeRecipeBindings,
        ),
        NativeGpuFault,
    > {
        let context = phase.context().id();
        let stages = job.stages();
        let pixels = rgba_bytes(job.input.width, job.input.height).map_err(|_| {
            fault(
                context,
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "admitted extent changed",
            )
        })?;
        let mut charges = Vec::with_capacity(stages.len() + 4);
        charges.push(phase.bind_allocation(ChargeKind::IsfScratch, pixels)?);
        charges.push(phase.bind_allocation(ChargeKind::IsfScratch, pixels)?);
        let final_charge = phase.bind_allocation(ChargeKind::ResidentLayer, pixels)?;
        if let Some(managed) = managed_input {
            if !Arc::ptr_eq(&managed.job.input, &job.input) {
                return Err(fault(
                    context,
                    request,
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::IdentityMismatch,
                    "managed CPU input is not this job's immutable input",
                ));
            }
            phase.retain_resource(Arc::clone(managed), &[managed.charge.id()])?;
        } else {
            charges.push(
                phase.bind_allocation(
                    ChargeKind::ActiveCpuInput,
                    job.input.data.capacity() as u64,
                )?,
            );
        }
        let upload = checked_upload_staging_bytes(
            job.input.width,
            job.input.height,
            phase.context().capabilities().upload_row_pitch,
        )
        .map_err(|_| {
            fault(
                context,
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "admitted pitch changed",
            )
        })?;
        charges.push(phase.bind_allocation(ChargeKind::QueueUploadStaging, upload)?);
        for _ in &stages {
            charges.push(phase.bind_allocation(ChargeKind::CachedEffectResource, UNIFORM_BYTES)?);
        }
        let mut resources = self
            .native_create_stack_resources(&phase, &job.input, &stages, clocks, false)
            .map_err(|error| {
                fault(
                    context,
                    request,
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::Validation,
                    error.message,
                )
            })?;
        if let Some(sampler) = frozen_sampler {
            resources.use_frozen_sampler(sampler);
        }
        let resources = Arc::new(resources);
        let final_image = Arc::new(NativeTextureAllocation::new(
            resources.final_image(stages.len()),
            final_charge,
        ));
        let retained = Arc::new(ScratchOwner {
            _resources: Arc::clone(&resources),
            charges,
            _input: Arc::clone(job),
            final_image: Arc::clone(&final_image),
        });
        phase.retain_resource(Arc::clone(&retained), &retained.allocation_ids())?;
        phase.mark_queue_write();
        let (command, pins) = self
            .native_encode_canonical_stack(
                &phase,
                &resources,
                &job.input,
                &stages,
                &job.chain.keys,
                clocks,
            )
            .map_err(|error| {
                fault(
                    context,
                    request,
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::Validation,
                    error.message,
                )
            })?;
        let bindings = NativeRecipeBindings::new(pins, resources.sampler());
        phase.retain_resource(Arc::new(bindings.clone()), &[])?;
        Ok((command, final_image, bindings))
    }

    fn check_native_transaction(
        &self,
        txn: &NativeGpuFrameTxn<'_>,
        frame: NativeFrameId,
    ) -> Result<(), NativeGpuFault> {
        if self
            .native_context()
            .is_none_or(|context| !Arc::ptr_eq(context, txn.context()))
            || txn.frame() != frame
        {
            return Err(fault(
                txn.context().id(),
                frame,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::IdentityMismatch,
                "native runtime/frame does not own this transaction",
            ));
        }
        Ok(())
    }

    pub(crate) fn complete_native_batch(
        &mut self,
        encoded: EncodedIsfBatch,
        phase: ResolvedGpuPhase,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> NativeIsfCompletion {
        if phase.context() != owner.context().id()
            || phase.submission() != encoded.pending.submission()
            || phase.phase() != GpuPhase::IsfBatch
        {
            return NativeIsfCompletion::Failed {
                fault: fault(
                    owner.context().id(),
                    phase.frame(),
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::IdentityMismatch,
                    "wrong completed ISF receipt",
                ),
                retirement: None,
            };
        }
        if cancel.is_cancelled() {
            return NativeIsfCompletion::Cancelled { retirement: None };
        }
        if let Err(error) = ensure_publication_active(owner, phase.frame(), GpuPhase::IsfBatch) {
            return match error {
                MaterializationFailure::Cancelled { retirement } => {
                    NativeIsfCompletion::Cancelled { retirement }
                }
                MaterializationFailure::Fault { fault, retirement } => {
                    NativeIsfCompletion::Failed { fault, retirement }
                }
            };
        }
        if phase.validation().is_err() {
            let original = phase.validation().unwrap_err();
            if matches!(
                original.kind,
                NativeGpuFaultKind::DeviceLost | NativeGpuFaultKind::ScopeDrain
            ) || owner.state() != GpuOwnerState::Active
            {
                return NativeIsfCompletion::Failed {
                    fault: copy_fault(original),
                    retirement: None,
                };
            }
            let frame = phase.frame();
            drop(encoded.candidates);
            match self.recover_native_jobs(
                encoded.jobs,
                encoded.clocks,
                encoded.inputs,
                frame,
                owner,
                cancel,
            ) {
                Ok(layers) => {
                    if let Err(error) = check_cancel(cancel)
                        .and_then(|()| ensure_publication_active(owner, frame, GpuPhase::IsfBatch))
                    {
                        return match error {
                            MaterializationFailure::Cancelled { retirement } => {
                                NativeIsfCompletion::Cancelled { retirement }
                            }
                            MaterializationFailure::Fault { fault, retirement } => {
                                NativeIsfCompletion::Failed { fault, retirement }
                            }
                        };
                    }
                    return NativeIsfCompletion::RecoveredCpu { layers };
                }
                Err(MaterializationFailure::Cancelled { retirement }) => {
                    return NativeIsfCompletion::Cancelled { retirement }
                }
                Err(MaterializationFailure::Fault { fault, retirement }) => {
                    return NativeIsfCompletion::Failed { fault, retirement }
                }
            }
        }
        if encoded.candidates.len() != encoded.jobs.len()
            || encoded.bindings.len() != encoded.jobs.len()
        {
            return NativeIsfCompletion::Failed {
                fault: fault(
                    owner.context().id(),
                    phase.frame(),
                    GpuPhase::IsfBatch,
                    NativeGpuFaultKind::StateMismatch,
                    "validated batch is incomplete",
                ),
                retirement: None,
            };
        }
        let mut layers = StackBatch::Empty;
        for ((candidate, job), pins) in encoded
            .candidates
            .into_iter()
            .zip(encoded.jobs)
            .zip(encoded.bindings)
        {
            let lease = match candidate.validate(&phase) {
                Ok(lease) => lease,
                Err(fault) => {
                    return NativeIsfCompletion::Failed {
                        fault,
                        retirement: None,
                    }
                }
            };
            let identity = *lease.identity();
            let recipe = FrozenStatelessRecipe::from_validated(&lease, job, pins);
            layers.push(ValidatedLayerResult {
                identity,
                lease,
                recipe,
            });
        }
        drop(encoded.inputs);
        if let Err(fault) = owner.poll_retirement() {
            return NativeIsfCompletion::Failed {
                fault,
                retirement: None,
            };
        }
        if let Err(error) = check_cancel(cancel)
            .and_then(|()| ensure_publication_active(owner, phase.frame(), GpuPhase::IsfBatch))
        {
            return match error {
                MaterializationFailure::Cancelled { retirement } => {
                    NativeIsfCompletion::Cancelled { retirement }
                }
                MaterializationFailure::Fault { fault, retirement } => {
                    NativeIsfCompletion::Failed { fault, retirement }
                }
            };
        }
        match IsfValidatedReceipt::from_phase(phase) {
            Ok(receipt) => NativeIsfCompletion::Validated { layers, receipt },
            Err(fault) => NativeIsfCompletion::Failed {
                fault,
                retirement: None,
            },
        }
    }
}
fn validate_job(job: &PreparedNativeLayer, max_extent: u32) -> Result<(), NativeGpuDecline> {
    if job.chain.stages.is_empty() {
        return Err(unsupported(UnsupportedNativeFx::EmptyChain));
    }
    if !job.chain.faults.is_empty() {
        return Err(unsupported(UnsupportedNativeFx::AuxiliaryResource));
    }
    if !matches!(job.chain.source.scope, protocol::VideoEffectScope::Layer { layer_id } if layer_id == job.input.layer_id)
    {
        return Err(unsupported(UnsupportedNativeFx::NonLayerScope));
    }
    if job.input.format != VideoPixelFormat::Rgba8 {
        return Err(NativeGpuDecline::UnsupportedFormat);
    }
    let expected = rgba_bytes(job.input.width, job.input.height)?;
    if job.input.width > max_extent
        || job.input.height > max_extent
        || job.input.data.len() as u64 != expected
    {
        return Err(NativeGpuDecline::UnsupportedExtent);
    }
    if job.chain.keys.len() != job.chain.stages.len()
        || job
            .chain
            .keys
            .iter()
            .enumerate()
            .any(|(i, key)| job.chain.keys[..i].contains(key))
    {
        return Err(unsupported(UnsupportedNativeFx::InterLayerDependency));
    }
    if !crate::isf_runtime::native_valid_date(job.date) {
        return Err(unsupported(UnsupportedNativeFx::AuxiliaryResource));
    }
    for stage in &job.chain.stages {
        if !stage.images.is_empty() || !stage.shader.required_layer_images.is_empty() {
            return Err(unsupported(UnsupportedNativeFx::InterLayerDependency));
        }
        if stage.shader.passes.len() != 1 {
            return Err(unsupported(UnsupportedNativeFx::Multipass));
        }
        if !(IsfStageInputs {
            shader: &stage.shader,
            controls: &stage.controls,
            images: &[],
        })
        .can_share_host_buffers()
        {
            return Err(unsupported(UnsupportedNativeFx::AuxiliaryResource));
        }
    }
    Ok(())
}

struct CpuStackOwner {
    _resources: Arc<NativeStackResources>,
    charges: Vec<AllocationCharge>,
    _input: Arc<VideoFrame>,
}
struct ChargedCpuFrame {
    frame: VideoFrame,
    charge: AllocationCharge,
}

fn materialization_fault(fault: NativeGpuFault) -> MaterializationFailure {
    MaterializationFailure::Fault {
        fault,
        retirement: None,
    }
}
fn check_cancel(cancel: &dyn VideoRenderCancellation) -> Result<(), MaterializationFailure> {
    if cancel.is_cancelled() {
        Err(MaterializationFailure::Cancelled { retirement: None })
    } else {
        Ok(())
    }
}
pub(crate) fn ensure_publication_active(
    owner: &NativeGpuOwner,
    frame: NativeFrameId,
    phase: GpuPhase,
) -> Result<(), MaterializationFailure> {
    match owner.state() {
        GpuOwnerState::Active => Ok(()),
        GpuOwnerState::Retiring | GpuOwnerState::Reaped => {
            Err(MaterializationFailure::Cancelled { retirement: None })
        }
        GpuOwnerState::DeviceLostRetained | GpuOwnerState::PoisonedRetained => {
            Err(materialization_fault(fault(
                owner.context().id(),
                frame,
                phase,
                if owner.state() == GpuOwnerState::DeviceLostRetained {
                    NativeGpuFaultKind::DeviceLost
                } else {
                    NativeGpuFaultKind::ScopeDrain
                },
                "GPU owner cannot publish after loss or poisoned retirement",
            )))
        }
    }
}
pub(crate) fn await_transaction(
    owner: &mut NativeGpuOwner,
    mut pending: PendingGpuTransaction,
    cancel: &dyn VideoRenderCancellation,
) -> Result<ResolvedGpuTransaction, MaterializationFailure> {
    let started = Instant::now();
    loop {
        match owner.poll_transaction(&mut pending, cancel) {
            CompletionProgress::Pending if started.elapsed() < Duration::from_secs(5) => {
                std::thread::yield_now()
            }
            CompletionProgress::Pending => {
                let retiring = owner.cancel_and_retire(pending);
                return Err(MaterializationFailure::Fault {
                    fault: NativeGpuFault {
                        context: owner.context().id(),
                        frame: None,
                        phase: GpuPhase::Retirement,
                        kind: NativeGpuFaultKind::ScopeDrain,
                        message: "GPU completion deadline expired; owner retained".into(),
                    },
                    retirement: Some(retiring.id()),
                });
            }
            CompletionProgress::Cancelled => {
                let retiring = owner.cancel_and_retire(pending);
                return Err(MaterializationFailure::Cancelled {
                    retirement: Some(retiring.id()),
                });
            }
            CompletionProgress::Complete(Err(fault)) => {
                let retiring = owner.cancel_and_retire(pending);
                return Err(MaterializationFailure::Fault {
                    fault,
                    retirement: Some(retiring.id()),
                });
            }
            CompletionProgress::Complete(Ok(resolved)) => return Ok(resolved),
        }
    }
}
fn reserve_plan(
    owner: &mut NativeGpuOwner,
    plan: &NativeAllocationPlan,
    phase: GpuPhase,
) -> Result<crate::native_gpu_budget::FrameReservation, MaterializationFailure> {
    owner.poll_retirement().map_err(materialization_fault)?;
    let quote = owner.quote(plan).map_err(|reason| {
        materialization_fault(fault(
            owner.context().id(),
            plan.frame(),
            phase,
            NativeGpuFaultKind::OutOfMemory,
            format!("native recovery/materialization reservation: {reason:?}"),
        ))
    })?;
    owner.reserve(quote).map_err(|reason| {
        materialization_fault(fault(
            owner.context().id(),
            plan.frame(),
            phase,
            NativeGpuFaultKind::StateMismatch,
            format!("native reservation expired: {reason:?}"),
        ))
    })
}

fn receive_map(
    owner: &mut NativeGpuOwner,
    mapping: &mut NativeReadbackMap,
    frame: NativeFrameId,
    cancel: &dyn VideoRenderCancellation,
) -> Result<(), MaterializationFailure> {
    let started = Instant::now();
    loop {
        check_cancel(cancel)?;
        match mapping.try_complete() {
            Ok(Some(())) => return Ok(()),
            Err(error) => {
                return Err(materialization_fault(fault(
                    owner.context().id(),
                    frame,
                    GpuPhase::HistoricalReplay,
                    NativeGpuFaultKind::Map,
                    error,
                )))
            }
            Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                owner.poll_retirement().map_err(materialization_fault)?;
                std::thread::yield_now();
            }
            Ok(None) => {
                return Err(materialization_fault(fault(
                    owner.context().id(),
                    frame,
                    GpuPhase::HistoricalReplay,
                    NativeGpuFaultKind::Map,
                    "native map callback deadline expired",
                )))
            }
        }
    }
}
fn packed_copy(buffer: &wgpu::Buffer, frame: &VideoFrame) -> VideoFrame {
    let mapped = buffer.slice(..).get_mapped_range();
    let row = frame.width as usize * 4;
    let pitch = row.div_ceil(256) * 256;
    let mut data = Vec::with_capacity(row * frame.height as usize);
    if row == pitch {
        data.extend_from_slice(&mapped[..row * frame.height as usize]);
    } else {
        for source in mapped.chunks_exact(pitch).take(frame.height as usize) {
            data.extend_from_slice(&source[..row]);
        }
    }
    drop(mapped);
    VideoFrame {
        layer_id: frame.layer_id,
        width: frame.width,
        height: frame.height,
        pts_ms: frame.pts_ms,
        duration_ms: frame.duration_ms,
        format: VideoPixelFormat::Rgba8,
        data,
    }
}

impl IsfGpuRuntime {
    fn render_native_fixed_cpu_stack(
        &mut self,
        owner: &mut NativeGpuOwner,
        request: NativeFrameId,
        frame: &VideoFrame,
        original: &Arc<VideoFrame>,
        stages: &[IsfStageInputs<'_>],
        keys: &[IsfInstanceKey],
        managed_input: &Arc<NativeInputAllocation>,
        clocks: FrozenChainClocks,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ChargedCpuFrame, MaterializationFailure> {
        check_cancel(cancel)?;
        let context = owner.context().id();
        let mut plan = NativeAllocationPlan::new(request);
        let original_is_current = std::ptr::eq(frame, original.as_ref());
        add_stack_plan(
            &mut plan,
            frame,
            stages.len(),
            owner.capabilities().upload_row_pitch,
            true,
            original_is_current.then_some(managed_input.charge.id()),
        )
        .map_err(|reason| {
            materialization_fault(fault(
                context,
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::OutOfMemory,
                format!("{reason:?}"),
            ))
        })?;
        let reservation = reserve_plan(owner, &plan, GpuPhase::IsfBatch)?;
        let mut txn = owner
            .begin_frame(request, reservation)
            .map_err(materialization_fault)?;
        let retirement_id = txn.id();
        let pending_fault = |fault| MaterializationFailure::Fault {
            fault,
            retirement: Some(retirement_id),
        };
        self.check_native_transaction(&txn, request)
            .map_err(pending_fault)?;
        let mut phase = txn.begin_phase(GpuPhase::IsfBatch).map_err(pending_fault)?;
        let pixels = rgba_bytes(frame.width, frame.height).expect("validated CPU stack extent");
        let mut charges = Vec::new();
        for kind in [
            ChargeKind::IsfScratch,
            ChargeKind::IsfScratch,
            ChargeKind::ResidentLayer,
        ] {
            charges.push(phase.bind_allocation(kind, pixels).map_err(pending_fault)?);
        }
        if original_is_current {
            phase
                .retain_resource(Arc::clone(managed_input), &[managed_input.charge.id()])
                .map_err(pending_fault)?;
        } else {
            charges.push(
                phase
                    .bind_allocation(ChargeKind::ActiveCpuInput, frame.data.capacity() as u64)
                    .map_err(pending_fault)?,
            );
        }
        let upload = checked_upload_staging_bytes(
            frame.width,
            frame.height,
            phase.context().capabilities().upload_row_pitch,
        )
        .expect("reserved upload pitch");
        charges.push(
            phase
                .bind_allocation(ChargeKind::QueueUploadStaging, upload)
                .map_err(pending_fault)?,
        );
        for _ in stages {
            charges.push(
                phase
                    .bind_allocation(ChargeKind::CachedEffectResource, UNIFORM_BYTES)
                    .map_err(pending_fault)?,
            );
        }
        let padded = checked_upload_staging_bytes(frame.width, frame.height, Some(256))
            .expect("reserved readback pitch");
        charges.push(
            phase
                .bind_allocation(ChargeKind::SnapshotReadback, padded)
                .map_err(pending_fault)?,
        );
        let output_charge = phase
            .bind_allocation(ChargeKind::CpuSnapshot, pixels)
            .map_err(pending_fault)?;
        let input = if std::ptr::eq(frame, original.as_ref()) {
            Arc::clone(original)
        } else {
            Arc::new(frame.clone())
        };
        let resources = Arc::new(
            self.native_create_stack_resources(&phase, &input, stages, clocks, true)
                .map_err(|error| {
                    pending_fault(fault(
                        context,
                        request,
                        GpuPhase::IsfBatch,
                        NativeGpuFaultKind::Validation,
                        error.message,
                    ))
                })?,
        );
        let retained = Arc::new(CpuStackOwner {
            _resources: Arc::clone(&resources),
            charges,
            _input: Arc::clone(&input),
        });
        let ids: Vec<_> = retained.charges.iter().map(AllocationCharge::id).collect();
        phase
            .retain_resource(Arc::clone(&retained), &ids)
            .map_err(pending_fault)?;
        phase.mark_queue_write();
        let encoded =
            self.native_encode_canonical_stack(&phase, &resources, &input, stages, keys, clocks);
        let commands = match encoded {
            Ok((command, pins)) => {
                phase
                    .retain_resource(Arc::new(pins), &[])
                    .map_err(pending_fault)?;
                vec![command]
            }
            Err(error) => {
                phase.record_failure(NativeGpuFaultKind::Validation, error.message);
                Vec::new()
            }
        };
        let phase_encoded = phase.finish(commands);
        let receipt = txn.submit_phase(phase_encoded).map_err(pending_fault)?;
        let pending = txn.finish();
        let buffer = resources
            .readback()
            .expect("CPU continuation has a map buffer");
        let mut mapping = NativeReadbackMap::start(buffer);
        let result = (|| {
            let mut resolved = await_transaction(owner, pending, cancel)?;
            resolved
                .take_phase(receipt)
                .map_err(materialization_fault)?
                .into_validation()
                .map_err(materialization_fault)?;
            receive_map(owner, &mut mapping, request, cancel)?;
            let frame = packed_copy(buffer, frame);
            check_cancel(cancel)?;
            ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
            Ok(ChargedCpuFrame {
                frame,
                charge: output_charge,
            })
        })();
        // Also cancels a pending map when cancellation/drain failure returned.
        mapping.unmap();
        drop(retained);
        result
    }

    fn charge_original_clone(
        &self,
        owner: &mut NativeGpuOwner,
        request: NativeFrameId,
        input: &VideoFrame,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ChargedCpuFrame, MaterializationFailure> {
        check_cancel(cancel)?;
        let mut plan = NativeAllocationPlan::new(request);
        let bytes = input.data.len() as u64;
        plan.add_new(ChargeKind::CpuSnapshot, bytes).map_err(|_| {
            materialization_fault(fault(
                owner.context().id(),
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::OutOfMemory,
                "CPU failed-chain result capacity",
            ))
        })?;
        let reservation = reserve_plan(owner, &plan, GpuPhase::IsfBatch)?;
        let mut txn = owner
            .begin_frame(request, reservation)
            .map_err(materialization_fault)?;
        let retirement_id = txn.id();
        let pending_fault = |fault| MaterializationFailure::Fault {
            fault,
            retirement: Some(retirement_id),
        };
        let charge = txn
            .bind_allocation(ChargeKind::CpuSnapshot, bytes)
            .map_err(pending_fault)?;
        let retained = Arc::new(ChargedCpuFrame {
            frame: input.clone(),
            charge,
        });
        txn.retain_resource(Arc::clone(&retained), &[retained.charge.id()])
            .map_err(pending_fault)?;
        let pending = txn.finish();
        await_transaction(owner, pending, cancel)?;
        check_cancel(cancel)?;
        ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
        Arc::try_unwrap(retained).map_err(|_| {
            materialization_fault(fault(
                owner.context().id(),
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "CPU result remains pending",
            ))
        })
    }

    fn recover_native_jobs(
        &mut self,
        jobs: StackBatch<Arc<PreparedNativeLayer>>,
        clocks: StackBatch<FrozenChainClocks>,
        managed_inputs: StackBatch<Arc<NativeInputAllocation>>,
        request: NativeFrameId,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<StackBatch<CanonicalCpuLayerOutcome>, MaterializationFailure> {
        let mut layers = StackBatch::Empty;
        for ((job, clocks), managed_input) in jobs.into_iter().zip(clocks).zip(managed_inputs) {
            check_cancel(cancel)?;
            let stages = job.stages();
            let mut retained_charge = None;
            let mut terminal = None;
            let (rendered, failures) = crate::isf_runtime::run_stage_local_chain(
                &job.input,
                &stages,
                &job.chain.keys,
                |frame, stages, keys| {
                    if terminal.is_some() {
                        return Err(IsfRuntimeError {
                            message: "native recovery stopped".into(),
                        });
                    }
                    match self.render_native_fixed_cpu_stack(
                        owner,
                        request,
                        frame,
                        &job.input,
                        stages,
                        keys,
                        &managed_input,
                        clocks,
                        cancel,
                    ) {
                        Ok(output) => {
                            retained_charge = Some(output.charge);
                            Ok(output.frame)
                        }
                        Err(MaterializationFailure::Fault { fault, retirement })
                            if !matches!(
                                fault.kind,
                                NativeGpuFaultKind::DeviceLost | NativeGpuFaultKind::ScopeDrain
                            ) && owner.state() == GpuOwnerState::Active
                                && retirement.is_none() =>
                        {
                            Err(IsfRuntimeError {
                                message: fault.message,
                            })
                        }
                        Err(error) => {
                            terminal = Some(error);
                            Err(IsfRuntimeError {
                                message: "native recovery cancelled or owner retained".into(),
                            })
                        }
                    }
                },
            );
            if let Some(error) = terminal {
                return Err(error);
            }
            check_cancel(cancel)?;
            let output = match rendered {
                std::borrow::Cow::Owned(frame) => ChargedCpuFrame {
                    frame,
                    charge: retained_charge.expect("successful CPU output owns its charge"),
                },
                std::borrow::Cow::Borrowed(frame) => {
                    self.charge_original_clone(owner, request, frame, cancel)?
                }
            };
            let identity = job.identity(owner.context().id(), request, clocks);
            let faults = job.chain.clone().finish_faults(failures);
            layers.push(CanonicalCpuLayerOutcome {
                identity,
                frame: output.frame,
                faults,
                provenance: CpuOutcomeProvenance::FixedClockRecovery,
                charge: output.charge,
            });
        }
        Ok(layers)
    }
}

pub(crate) enum CpuMaterializationRequest<'a> {
    ValidatedGpu {
        entry: HistoryEntryId,
        lease: &'a GpuLayerLease,
    },
    FrozenReplay {
        entry: HistoryEntryId,
        recipe: &'a FrozenStatelessRecipe,
    },
}
pub(crate) struct NativeGpuMaterializer<'a> {
    runtime: &'a mut IsfGpuRuntime,
}
impl<'a> NativeGpuMaterializer<'a> {
    pub(crate) fn new(runtime: &'a mut IsfGpuRuntime) -> Self {
        Self { runtime }
    }
    pub(crate) fn materialize_cpu(
        &mut self,
        request: CpuMaterializationRequest<'_>,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ValidatedCpuMaterialization, MaterializationFailure> {
        match request {
            CpuMaterializationRequest::ValidatedGpu { entry, lease } => self
                .runtime
                .materialize_validated_layer(entry, lease, owner, cancel),
            CpuMaterializationRequest::FrozenReplay { entry, recipe } => self
                .runtime
                .replay_recipe_to_cpu(entry, recipe, owner, cancel),
        }
    }
}
pub(crate) struct MaterializedGpuLayer {
    pub lease: GpuLayerLease,
    // I's C1 adapter can mint the common proof only from this affine completion.
    phase: ResolvedGpuPhase,
}
impl MaterializedGpuLayer {
    pub(crate) fn descriptor(&self) -> MaterializationDescriptor {
        MaterializationDescriptor {
            layer: *self.lease.identity(),
            allocation: self.lease.allocation_id(),
            completed_submission: self.phase.submission(),
        }
    }
    pub(crate) fn into_parts(self) -> (GpuLayerLease, ResolvedGpuPhase) {
        (self.lease, self.phase)
    }
}
impl IsfGpuRuntime {
    pub(crate) fn replay_allocation_plan(
        &self,
        frame: NativeFrameId,
        recipe: &FrozenStatelessRecipe,
    ) -> Result<NativeAllocationPlan, NativeGpuDecline> {
        let mut plan = NativeAllocationPlan::new(frame);
        let context = self
            .native_context()
            .ok_or(NativeGpuDecline::QuoteExpired)?;
        add_stack_plan(
            &mut plan,
            &recipe.job().input,
            recipe.job().chain.stages.len(),
            context.capabilities().upload_row_pitch,
            false,
            None,
        )?;
        Ok(plan)
    }
    pub(crate) fn encode_historical_replay(
        &mut self,
        txn: &mut NativeGpuFrameTxn<'_>,
        entry: HistoryEntryId,
        recipe: &FrozenStatelessRecipe,
    ) -> Result<EncodedLayerCandidate, NativeGpuFault> {
        let frame = txn.frame();
        self.check_native_transaction(txn, frame)?;
        if recipe.identity().context != txn.context().id() || recipe.identity().entry != entry {
            return Err(fault(
                txn.context().id(),
                frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::IdentityMismatch,
                "historical replay entry/generation mismatch",
            ));
        }
        let transaction = txn.id();
        let mut phase = txn.begin_phase(GpuPhase::HistoricalReplay)?;
        let result = self.encode_resident_job(
            &mut phase,
            recipe.job(),
            recipe.clocks(),
            frame,
            None,
            Some(recipe.sampler()),
        );
        let (commands, image) = match result {
            Ok((command, image, _)) => (vec![command], Some(image)),
            Err(error) => {
                phase.record_failure(error.kind, error.message);
                (Vec::new(), None)
            }
        };
        let encoded = phase.finish(commands);
        let receipt = txn.submit_phase(encoded)?;
        let Some(image) = image else {
            // The txn still owns all queued uploads/scope resources. Caller must
            // retire it; replay never becomes a successful history placeholder.
            return Err(fault(
                txn.context().id(),
                frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::HistoricalMaterialization { entry },
                "historical replay encoding failed; transaction retains resources",
            ));
        };
        let mut candidate = EncodedLayerCandidate::from_encoded(
            *recipe.identity(),
            image,
            transaction,
            receipt.submission(),
            frame,
        )?;
        candidate.attach_replay_receipt(receipt);
        Ok(candidate)
    }
    pub(crate) fn complete_historical_replay(
        &mut self,
        candidate: EncodedLayerCandidate,
        phase: ResolvedGpuPhase,
        owner: &mut NativeGpuOwner,
    ) -> Result<MaterializedGpuLayer, MaterializationFailure> {
        if phase.context() != owner.context().id() || phase.phase() != GpuPhase::HistoricalReplay {
            return Err(materialization_fault(fault(
                owner.context().id(),
                phase.frame(),
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::IdentityMismatch,
                "wrong historical completion",
            )));
        }
        ensure_publication_active(owner, phase.frame(), GpuPhase::HistoricalReplay)?;
        let lease = candidate.validate(&phase).map_err(materialization_fault)?;
        owner.poll_retirement().map_err(materialization_fault)?;
        ensure_publication_active(owner, phase.frame(), GpuPhase::HistoricalReplay)?;
        Ok(MaterializedGpuLayer { lease, phase })
    }
    pub(crate) fn replay_recipe_to_cpu(
        &mut self,
        entry: HistoryEntryId,
        recipe: &FrozenStatelessRecipe,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ValidatedCpuMaterialization, MaterializationFailure> {
        check_cancel(cancel)?;
        let frame = recipe.identity().frame;
        let plan = self
            .replay_allocation_plan(frame, recipe)
            .map_err(|reason| {
                materialization_fault(fault(
                    owner.context().id(),
                    frame,
                    GpuPhase::HistoricalReplay,
                    NativeGpuFaultKind::OutOfMemory,
                    format!("{reason:?}"),
                ))
            })?;
        let reservation = reserve_plan(owner, &plan, GpuPhase::HistoricalReplay)?;
        let mut txn = owner
            .begin_frame(frame, reservation)
            .map_err(materialization_fault)?;
        let retirement_id = txn.id();
        let mut candidate = match self.encode_historical_replay(&mut txn, entry, recipe) {
            Ok(candidate) => candidate,
            Err(fault) => {
                drop(txn);
                return Err(MaterializationFailure::Fault {
                    fault,
                    retirement: Some(retirement_id),
                });
            }
        };
        let receipt =
            candidate
                .take_replay_receipt()
                .map_err(|fault| MaterializationFailure::Fault {
                    fault,
                    retirement: Some(retirement_id),
                })?;
        let pending = txn.finish();
        let mut resolved = await_transaction(owner, pending, cancel)?;
        let phase = resolved
            .take_phase(receipt)
            .map_err(materialization_fault)?;
        let replay = self.complete_historical_replay(candidate, phase, owner)?;
        check_cancel(cancel)?;
        self.materialize_validated_layer(entry, &replay.lease, owner, cancel)
    }
    pub(crate) fn materialize_validated_layer(
        &mut self,
        entry: HistoryEntryId,
        lease: &GpuLayerLease,
        owner: &mut NativeGpuOwner,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<ValidatedCpuMaterialization, MaterializationFailure> {
        check_cancel(cancel)?;
        let identity = *lease.identity();
        if identity.entry != entry
            || identity.context != owner.context().id()
            || self
                .native_context()
                .is_none_or(|context| !Arc::ptr_eq(context, owner.context()))
        {
            return Err(materialization_fault(fault(
                owner.context().id(),
                identity.frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::IdentityMismatch,
                "CPU materialization requires the exact entry and generation",
            )));
        }
        let frame = identity.frame;
        let bytes = rgba_bytes(identity.width, identity.height).map_err(|_| {
            materialization_fault(fault(
                identity.context,
                frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::Validation,
                "invalid materialization extent",
            ))
        })?;
        let padded = checked_upload_staging_bytes(identity.width, identity.height, Some(256))
            .expect("validated readback extent");
        let mut plan = NativeAllocationPlan::new(frame);
        plan.add_existing(lease.allocation_id());
        plan.add_new(ChargeKind::SnapshotReadback, padded)
            .expect("bounded map allocation");
        plan.add_new(ChargeKind::CpuSnapshot, bytes)
            .expect("bounded CPU copy allocation");
        let reservation = reserve_plan(owner, &plan, GpuPhase::HistoricalReplay)?;
        let mut txn = owner
            .begin_frame(frame, reservation)
            .map_err(materialization_fault)?;
        let retirement_id = txn.id();
        let (copy, receipt) =
            PendingCpuMaterialization::encode(&mut txn, lease).map_err(|fault| {
                MaterializationFailure::Fault {
                    fault,
                    retirement: Some(retirement_id),
                }
            })?;
        let pending = txn.finish();
        let mut resolved = await_transaction(owner, pending, cancel)?;
        let phase = resolved
            .take_phase(receipt)
            .map_err(materialization_fault)?;
        copy.complete(phase, owner, cancel)
    }
}

#[cfg(test)]
#[path = "isf_native_gpu_tests.rs"]
pub(crate) mod tests;
