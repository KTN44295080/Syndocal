//! Resident ISF execution through the canonical encoder and G's mutable broker.
//! The C0b module gate leaves production routing unchanged until I's C1 join.
use crate::isf_chain_execution::PreparedChain;
use crate::isf_runtime::{
    IndependentClockReservation, IsfGpuRuntime, IsfInstanceKey, IsfStageInputs,
    NativeStackResources, StackBatch,
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

#[path = "isf_native_pool.rs"]
pub(crate) mod pool;

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
    work: PreparedNativeWork,
    audio: Option<protocol::LiveAudioTextureFrame>,
    plan: Result<NativeAllocationPlan, NativeGpuDecline>,
    clocks: Option<IndependentClockReservation>,
    pooled: Vec<pool::StackSelection>,
}
enum PreparedNativeWork {
    Layers(StackBatch<Arc<PreparedNativeLayer>>),
    Cpu(cpu::PreparedCpuWork),
}

/// Read-only preflight metadata is not execution or completion authority.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeProspectiveIdentityError {
    CpuFallback,
    Declined(NativeGpuDecline),
    ContextMismatch,
    StaleClock,
    StalePool,
}

impl PreparedNativeInputs {
    pub(crate) fn allocation_plan(&self) -> Result<&NativeAllocationPlan, NativeGpuDecline> {
        self.plan.as_ref().map_err(|reason| *reason)
    }

    /// Return the exact prospective identities for P's allocation/selection
    /// preflight. Eligible clocks were frozen at prepare_native_inputs, not by
    /// this query. Querying never advances clocks or touches the GPU/history.
    /// Admission and encode revalidate the same runtime epoch and clock range.
    pub(crate) fn prospective_identities(
        &self,
        runtime: &IsfGpuRuntime,
    ) -> Result<StackBatch<LayerResultIdentity>, NativeProspectiveIdentityError> {
        let (jobs, clocks) = self.resident_preview_parts(runtime)?;
        Ok(StackBatch::from_fn(jobs.len(), |index| {
            jobs[index].identity(
                self.context,
                self.frame,
                jobs[index].clocks(clocks.at(index)),
            )
        }))
    }

    fn resident_preview_parts(
        &self,
        runtime: &IsfGpuRuntime,
    ) -> Result<
        (
            &StackBatch<Arc<PreparedNativeLayer>>,
            &IndependentClockReservation,
        ),
        NativeProspectiveIdentityError,
    > {
        let PreparedNativeWork::Layers(jobs) = &self.work else {
            return Err(NativeProspectiveIdentityError::CpuFallback);
        };
        self.allocation_plan()
            .map_err(NativeProspectiveIdentityError::Declined)?;
        if runtime
            .native_context()
            .is_none_or(|context| context.id() != self.context)
        {
            return Err(NativeProspectiveIdentityError::ContextMismatch);
        }
        let epoch = runtime
            .native_context()
            .expect("validated native context")
            .resource_pool_epoch();
        if self.pooled.len() != jobs.len()
            || self
                .pooled
                .iter()
                .zip(jobs.iter())
                .any(|(selection, job)| !selection.current(epoch, self.frame, job))
        {
            return Err(NativeProspectiveIdentityError::StalePool);
        }
        let clocks = self
            .clocks
            .as_ref()
            .filter(|clocks| runtime.native_clocks_current(clocks))
            .ok_or(NativeProspectiveIdentityError::StaleClock)?;
        Ok((jobs, clocks))
    }

    #[cfg(test)]
    pub(crate) fn into_layers(self) -> StackBatch<Arc<PreparedNativeLayer>> {
        match self.work {
            PreparedNativeWork::Layers(jobs) => jobs,
            PreparedNativeWork::Cpu(_) => panic!("canonical CPU inputs are not Layer identities"),
        }
    }
}
pub(crate) struct PreparedNativeBatch {
    inputs: PreparedNativeInputs,
}
impl PreparedNativeBatch {
    /// Read-only P selection preflight against the original frozen G inputs.
    /// This neither reserves new clocks nor supplies execution/completion proof.
    pub(crate) fn prospective_identities(
        &self,
        runtime: &IsfGpuRuntime,
    ) -> Result<StackBatch<LayerResultIdentity>, NativeProspectiveIdentityError> {
        self.inputs.prospective_identities(runtime)
    }
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
        let mut pooled = Vec::new();
        let plan = (|| {
            // One to three independent layer jobs share the same resident
            // batch executor; a single-layer Display is the ordinary case.
            if !(1..=3).contains(&jobs.len()) {
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
                let selection = self.native_stack_pool.select(
                    context.resource_pool_epoch(),
                    frame,
                    job,
                    &pooled,
                )?;
                selection.add_plan(
                    &mut plan,
                    &job.input,
                    context.capabilities().upload_row_pitch,
                )?;
                pooled.push(selection);
            }
            Ok(plan)
        })();
        let clocks = plan
            .as_ref()
            .ok()
            .map(|_| self.reserve_independent_clocks(jobs.len()));
        Ok(PreparedNativeInputs {
            frame,
            context: context.id(),
            work: PreparedNativeWork::Layers(jobs),
            audio: self.native_audio_snapshot(),
            plan,
            clocks,
            pooled,
        })
    }

    pub(crate) fn admit_native_batch(
        &self,
        inputs: PreparedNativeInputs,
        quote: &BudgetQuote,
    ) -> NativeBatchAdmission {
        let reason = match inputs.resident_preview_parts(self) {
            Err(NativeProspectiveIdentityError::CpuFallback) => {
                Some(unsupported(UnsupportedNativeFx::UnsupportedJobCount))
            }
            Err(NativeProspectiveIdentityError::Declined(reason)) => Some(reason),
            Err(
                NativeProspectiveIdentityError::ContextMismatch
                | NativeProspectiveIdentityError::StaleClock
                | NativeProspectiveIdentityError::StalePool,
            ) => Some(NativeGpuDecline::QuoteExpired),
            Ok(_)
                if !inputs.plan.as_ref().is_ok_and(|plan| {
                    quote.context() == inputs.context && quote.covers_plan(plan)
                }) =>
            {
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
        inputs.resident_preview_parts(self).map_err(|error| {
            fault(
                inputs.context,
                inputs.frame,
                GpuPhase::IsfBatch,
                if error == NativeProspectiveIdentityError::ContextMismatch {
                    NativeGpuFaultKind::IdentityMismatch
                } else {
                    NativeGpuFaultKind::StateMismatch
                },
                format!("prospective native identity is no longer current: {error:?}"),
            )
        })?;
        // P must split its composition/snapshot allocation slots before G can
        // bind by kind/capacity. Extra validated-history aliases are permitted.
        txn.validate_root_plan(inputs.plan.as_ref().expect("admitted allocation plan"))?;
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
        let PreparedNativeWork::Layers(jobs) = inputs.work else {
            return Err(fault(
                context,
                inputs.frame,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "CPU invocation cannot enter resident Batch3",
            ));
        };
        let logical = inputs.clocks.expect("validated prospective clock range");
        let clocks = StackBatch::from_fn(jobs.len(), |index| jobs[index].clocks(logical.at(index)));
        let mut phase = txn.begin_phase(GpuPhase::IsfBatch)?;
        let mut managed_inputs = StackBatch::Empty;
        for job in &*jobs {
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
        let mut commands = Vec::with_capacity(jobs.len());
        let mut failed = false;
        for (index, job) in jobs.iter().enumerate() {
            let result = self.encode_resident_job(
                &mut phase,
                job,
                clocks[index],
                inputs.frame,
                Some(&managed_inputs[index]),
                None,
                Some(&inputs.pooled[index]),
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
                    jobs[index].identity(context, inputs.frame, clocks[index]),
                    image,
                    transaction,
                    submission,
                    inputs.frame,
                )?);
            }
            candidates
        };
        Ok(EncodedIsfBatch::from_encoding(
            candidates,
            PendingIsfReceipt::new(receipt),
            jobs,
            clocks,
            bindings,
            managed_inputs,
            failed,
        ))
    }

    fn encode_resident_job(
        &mut self,
        phase: &mut crate::native_gpu_context::GpuPhaseEncoder<'_>,
        job: &Arc<PreparedNativeLayer>,
        clocks: FrozenChainClocks,
        request: NativeFrameId,
        managed_input: Option<&Arc<NativeInputAllocation>>,
        frozen_sampler: Option<&wgpu::Sampler>,
        pooled: Option<&pool::StackSelection>,
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
        // Pool charges belong to persistent owners; per-tick input/upload
        // charges remain transaction scoped.
        let legacy_final_charge = if pooled.is_none() {
            charges.push(phase.bind_allocation(ChargeKind::IsfScratch, pixels)?);
            charges.push(phase.bind_allocation(ChargeKind::IsfScratch, pixels)?);
            Some(phase.bind_allocation(ChargeKind::ResidentLayer, pixels)?)
        } else {
            None
        };
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
        let (resources, final_image) = if let Some(selection) = pooled {
            self.acquire_native_stack(phase, job, clocks, request, selection)?
        } else {
            for _ in &stages {
                charges
                    .push(phase.bind_allocation(ChargeKind::CachedEffectResource, UNIFORM_BYTES)?);
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
            let final_image = Arc::new(NativeTextureAllocation::new(
                resources.final_image(stages.len()),
                legacy_final_charge.expect("legacy final charge"),
            ));
            (Arc::new(resources), final_image)
        };
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
        if let Some(reason) = (IsfStageInputs {
            shader: &stage.shader,
            controls: &stage.controls,
            images: &[],
        })
        .native_target_restriction()
        {
            return Err(unsupported(reason));
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
    phase: GpuPhase,
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
                    phase,
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
                    phase,
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
            receive_map(owner, &mut mapping, request, GpuPhase::IsfBatch, cancel)?;
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
#[cfg(test)]
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
        if recipe.identity().context != txn.context().id()
            || recipe.identity().entry != entry
            || recipe.pinned_pipeline_count() != recipe.job().chain.stages.len()
        {
            return Err(fault(
                txn.context().id(),
                frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::IdentityMismatch,
                "historical replay entry/generation or frozen pipeline binding mismatch",
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
            None,
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
        replay
            .phase
            .validation()
            .map_err(|fault| materialization_fault(copy_fault(fault)))?;
        if replay.phase.context() != owner.context().id()
            || replay.phase.frame() != recipe.identity().frame
            || replay.phase.phase() != GpuPhase::HistoricalReplay
        {
            return Err(materialization_fault(fault(
                owner.context().id(),
                frame,
                GpuPhase::HistoricalReplay,
                NativeGpuFaultKind::IdentityMismatch,
                "CPU replay readback does not match its resolved replay phase",
            )));
        }
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

pub(crate) use cpu::{NativeCpuRenderFailure, PreparedNativeCpuInvocation};

#[path = "isf_native_continuation.rs"]
pub(crate) mod continuation;

// The shared-context CPU bridge is a separate policy/lifecycle section. It
// delegates every shader pass and stage-local decision to the canonical code.
mod cpu {
    use super::*;
    use crate::native_gpu_budget::FrameReservation;
    use std::collections::HashSet;

    pub(crate) struct PreparedNativeCpuInvocation {
        input: Arc<VideoFrame>,
        chain: PreparedChain<'static>,
        date: [f32; 4],
        audio: Option<Arc<protocol::LiveAudioTextureFrame>>,
    }
    impl PreparedNativeCpuInvocation {
        pub(crate) fn new(
            input: Arc<VideoFrame>,
            chain: PreparedChain<'static>,
            date: [f32; 4],
            audio: Option<protocol::LiveAudioTextureFrame>,
        ) -> Self {
            Self {
                input,
                chain,
                date,
                audio: audio
                    .filter(protocol::LiveAudioTextureFrame::is_valid)
                    .map(Arc::new),
            }
        }
    }

    pub(crate) enum NativeCpuRenderFailure {
        Admission {
            fault: NativeGpuFault,
            prepared: PreparedNativeInputs,
            reservation: FrameReservation,
        },
        Declined {
            reason: NativeGpuDecline,
            prepared: PreparedNativeInputs,
        },
        Cancelled {
            retirement: Option<RetirementId>,
        },
        Fault {
            fault: NativeGpuFault,
            retirement: Option<RetirementId>,
        },
    }
    impl std::fmt::Debug for NativeCpuRenderFailure {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Self::Admission {
                    fault, reservation, ..
                } => f
                    .debug_struct("Admission")
                    .field("fault", fault)
                    .field("reservation", reservation)
                    .finish_non_exhaustive(),
                Self::Declined { reason, .. } => f
                    .debug_struct("Declined")
                    .field("reason", reason)
                    .finish_non_exhaustive(),
                Self::Cancelled { retirement } => f
                    .debug_struct("Cancelled")
                    .field("retirement", retirement)
                    .finish(),
                Self::Fault { fault, retirement } => f
                    .debug_struct("Fault")
                    .field("fault", fault)
                    .field("retirement", retirement)
                    .finish(),
            }
        }
    }
    impl From<MaterializationFailure> for NativeCpuRenderFailure {
        fn from(value: MaterializationFailure) -> Self {
            match value {
                MaterializationFailure::Cancelled { retirement } => Self::Cancelled { retirement },
                MaterializationFailure::Fault { fault, retirement } => {
                    Self::Fault { fault, retirement }
                }
            }
        }
    }

    enum CpuInput {
        Frame(Arc<VideoFrame>),
        Existing(Arc<NativeCpuFrameOwner>),
        Bytes(Arc<[u8]>),
        Audio(Arc<protocol::LiveAudioTextureFrame>),
    }
    impl CpuInput {
        fn identity(&self) -> (u8, usize) {
            match self {
                Self::Frame(frame) => (0, Arc::as_ptr(frame) as usize),
                Self::Existing(owner) => (0, Arc::as_ptr(&owner.frame) as usize),
                Self::Bytes(bytes) => (1, Arc::as_ptr(bytes) as *const u8 as usize),
                Self::Audio(audio) => (2, Arc::as_ptr(audio) as usize),
            }
        }
        fn new_bytes(&self) -> u64 {
            match self {
                Self::Frame(frame) => frame.data.capacity() as u64,
                Self::Existing(owner) => {
                    if owner.charge.is_none() {
                        owner.frame.data.capacity() as u64
                    } else {
                        0
                    }
                }
                Self::Bytes(bytes) => bytes.len() as u64,
                Self::Audio(_) => std::mem::size_of::<protocol::LiveAudioTextureFrame>() as u64,
            }
        }
    }
    pub(super) struct PreparedCpuWork {
        invocations: Vec<Arc<PreparedNativeCpuInvocation>>,
        inputs: Vec<CpuInput>,
        cache_stamp: (u64, Vec<(GpuAllocationId, usize)>),
    }

    struct AuxiliaryCpuOwner {
        _input: CpuInput,
        charge: Option<Arc<AllocationCharge>>,
    }
    struct HeldCpuInputs {
        frames: Vec<Arc<NativeCpuFrameOwner>>,
        auxiliary: Vec<Arc<AuxiliaryCpuOwner>>,
    }
    impl HeldCpuInputs {
        fn ids(&self) -> Vec<GpuAllocationId> {
            self.frames
                .iter()
                .flat_map(|owner| &owner.charge)
                .map(|charge| charge.id())
                .chain(
                    self.auxiliary
                        .iter()
                        .flat_map(|owner| &owner.charge)
                        .map(|charge| charge.id()),
                )
                .collect()
        }
        fn frame(&self, input: &Arc<VideoFrame>) -> Arc<NativeCpuFrameOwner> {
            Arc::clone(
                self.frames
                    .iter()
                    .find(|owner| Arc::ptr_eq(&owner.frame, input))
                    .expect("admitted input owner"),
            )
        }
    }
    enum CpuChunkFailure {
        Canonical(IsfRuntimeError),
        Terminal(MaterializationFailure),
    }
    impl From<MaterializationFailure> for CpuChunkFailure {
        fn from(value: MaterializationFailure) -> Self {
            Self::Terminal(value)
        }
    }

    fn retain_inputs(
        owner: &mut NativeGpuOwner,
        frame: NativeFrameId,
        inputs: Vec<CpuInput>,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<Arc<HeldCpuInputs>, MaterializationFailure> {
        let mut plan = NativeAllocationPlan::new(frame);
        for input in &inputs {
            if let CpuInput::Existing(prior) = input {
                if let Some(charge) = &prior.charge {
                    plan.add_existing(charge.id());
                }
            }
            if input.new_bytes() != 0 {
                plan.add_new(ChargeKind::ActiveCpuInput, input.new_bytes())
                    .map_err(|reason| {
                        materialization_fault(fault(
                            owner.context().id(),
                            frame,
                            GpuPhase::IsfBatch,
                            NativeGpuFaultKind::OutOfMemory,
                            format!("CPU source plan: {reason:?}"),
                        ))
                    })?;
            }
        }
        let reservation = reserve_plan(owner, &plan, GpuPhase::IsfBatch)?;
        let mut txn = owner
            .begin_frame(frame, reservation)
            .map_err(materialization_fault)?;
        let retirement = txn.id();
        let pending_fault = |fault| MaterializationFailure::Fault {
            fault,
            retirement: Some(retirement),
        };
        let mut held = HeldCpuInputs {
            frames: Vec::new(),
            auxiliary: Vec::new(),
        };
        for input in inputs {
            let bytes = input.new_bytes();
            let charge = if bytes == 0 {
                None
            } else {
                Some(Arc::new(
                    txn.bind_allocation(ChargeKind::ActiveCpuInput, bytes)
                        .map_err(pending_fault)?,
                ))
            };
            match input {
                CpuInput::Existing(prior) => {
                    let prior = match charge {
                        Some(charge) => {
                            crate::native_gpu_history::adapter::activate_native_cpu_owner(
                                prior, charge,
                            )
                        }
                        None => prior,
                    };
                    txn.retain_resource(
                        Arc::clone(&prior),
                        &prior
                            .charge
                            .iter()
                            .map(|charge| charge.id())
                            .collect::<Vec<_>>(),
                    )
                    .map_err(pending_fault)?;
                    held.frames.push(prior);
                }
                CpuInput::Frame(frame) => {
                    let retained = Arc::new(NativeCpuFrameOwner::new(frame, charge));
                    txn.retain_resource(
                        Arc::clone(&retained),
                        &retained
                            .charge
                            .iter()
                            .map(|charge| charge.id())
                            .collect::<Vec<_>>(),
                    )
                    .map_err(pending_fault)?;
                    held.frames.push(retained);
                }
                input => {
                    let retained = Arc::new(AuxiliaryCpuOwner {
                        _input: input,
                        charge,
                    });
                    txn.retain_resource(
                        Arc::clone(&retained),
                        &retained
                            .charge
                            .iter()
                            .map(|charge| charge.id())
                            .collect::<Vec<_>>(),
                    )
                    .map_err(pending_fault)?;
                    held.auxiliary.push(retained);
                }
            }
        }
        let held = Arc::new(held);
        let pending = txn.finish();
        await_transaction(owner, pending, cancel)?;
        Ok(held)
    }

    fn with_stages<T>(
        invocation: &PreparedNativeCpuInvocation,
        run: impl FnOnce(&[IsfStageInputs<'_>]) -> T,
    ) -> T {
        let images = invocation
            .chain
            .stages
            .iter()
            .map(|stage| {
                stage
                    .images
                    .iter()
                    .map(|(name, frame)| crate::isf_runtime::IsfImageInput { name, frame })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let stages = invocation
            .chain
            .stages
            .iter()
            .zip(&images)
            .map(|(stage, images)| IsfStageInputs {
                shader: &stage.shader,
                controls: &stage.controls,
                images,
            })
            .collect::<Vec<_>>();
        run(&stages)
    }

    fn checked_sum(left: u64, right: u64) -> Result<u64, NativeGpuDecline> {
        left.checked_add(right)
            .ok_or(NativeGpuDecline::BudgetExceeded {
                live: 0,
                requested: u64::MAX,
                limit: crate::native_gpu_budget::NATIVE_MANAGED_LIMIT,
            })
    }

    impl IsfGpuRuntime {
        pub(crate) fn render_native_cpu_fallback(
            &mut self,
            owner: &mut NativeGpuOwner,
            inputs: PreparedNativeInputs,
            reservation: FrameReservation,
            cancel: &dyn VideoRenderCancellation,
        ) -> Result<NativeCpuResults, NativeCpuRenderFailure> {
            let plan = match &inputs.plan {
                Ok(plan) => plan,
                Err(reason) => {
                    let reason = *reason;
                    if let Err((fault, reservation)) =
                        owner.discard_unsubmitted_reservation(reservation)
                    {
                        return Err(NativeCpuRenderFailure::Admission {
                            fault,
                            prepared: inputs,
                            reservation,
                        });
                    }
                    return Err(NativeCpuRenderFailure::Declined {
                        reason,
                        prepared: inputs,
                    });
                }
            };
            if let Err((fault, reservation)) = owner.admit_serial_cpu_plan(reservation, plan) {
                return Err(NativeCpuRenderFailure::Admission {
                    fault,
                    prepared: inputs,
                    reservation,
                });
            }
            if inputs.context != owner.context().id()
                || self
                    .native_context()
                    .is_none_or(|context| !Arc::ptr_eq(context, owner.context()))
            {
                return Err(NativeCpuRenderFailure::Fault {
                    fault: fault(
                        owner.context().id(),
                        inputs.frame,
                        GpuPhase::IsfBatch,
                        NativeGpuFaultKind::IdentityMismatch,
                        "CPU invocation belongs to another shared GPU context",
                    ),
                    retirement: None,
                });
            }
            check_cancel(cancel)?;
            ensure_publication_active(owner, inputs.frame, GpuPhase::IsfBatch)?;
            let PreparedNativeWork::Cpu(work) = &inputs.work else {
                return Err(NativeCpuRenderFailure::Declined {
                    reason: NativeGpuDecline::QuoteExpired,
                    prepared: inputs,
                });
            };
            if work.cache_stamp != self.native_cpu_cache_stamp() {
                return Err(NativeCpuRenderFailure::Declined {
                    reason: NativeGpuDecline::QuoteExpired,
                    prepared: inputs,
                });
            }
            let request = inputs.frame;
            let PreparedNativeWork::Cpu(work) = inputs.work else {
                unreachable!()
            };
            if let Some(first) = work.invocations.iter().find(|invocation| {
                crate::resolved_chain_executes(&invocation.chain.source)
                    && !invocation.chain.stages.is_empty()
                    && !invocation.chain.keys.is_empty()
                    && crate::isf_runtime::native_valid_date(invocation.date)
            }) {
                let _ = self.native_cpu_apply_baseline_evictions(&first.input);
                owner.poll_retirement().map_err(materialization_fault)?;
            }
            let held = retain_inputs(owner, request, work.inputs, cancel)?;
            let mut outcomes = Vec::with_capacity(work.invocations.len());
            for invocation in work.invocations {
                check_cancel(cancel)?;
                ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
                let input = held.frame(&invocation.input);
                let executes = crate::resolved_chain_executes(&invocation.chain.source);
                let clocks = if executes {
                    let logical = self.reserve_independent_clocks(1);
                    let clocks = FrozenChainClocks {
                        frame_index: logical.at(0),
                        time_seconds_bits: (invocation.input.pts_ms as f32 / 1000.0).to_bits(),
                        time_delta_seconds_bits: (invocation.input.duration_ms as f32 / 1000.0)
                            .to_bits(),
                        date_bits: invocation.date.map(f32::to_bits),
                    };
                    self.commit_independent_clocks(logical);
                    Some(clocks)
                } else {
                    None
                };
                self.set_audio_frame(invocation.audio.as_deref());
                let started = Instant::now();
                let (rendered, failures) = if let Some(clocks) = clocks {
                    let mut terminal = None;
                    let result = with_stages(&invocation, |stages| {
                        crate::isf_runtime::run_stage_local_values(
                            input,
                            stages,
                            &invocation.chain.keys,
                            |current, stages, keys| {
                                if terminal.is_some() {
                                    return Err(IsfRuntimeError {
                                        message: "native CPU chain cancelled or owner retained"
                                            .into(),
                                    });
                                }
                                match self.render_native_cpu_chunk(
                                    owner,
                                    request,
                                    Arc::clone(current),
                                    Arc::clone(&held),
                                    stages,
                                    keys,
                                    clocks,
                                    cancel,
                                ) {
                                    Ok(output) => Ok(output),
                                    Err(CpuChunkFailure::Canonical(error)) => Err(error),
                                    Err(CpuChunkFailure::Terminal(error)) => {
                                        terminal = Some(error);
                                        Err(IsfRuntimeError {
                                            message: "native CPU chain cancelled or owner retained"
                                                .into(),
                                        })
                                    }
                                }
                            },
                        )
                    });
                    if let Some(error) = terminal {
                        return Err(error.into());
                    }
                    result
                } else {
                    (input, Vec::new())
                };
                if executes {
                    self.native_record_cpu_metrics(
                        invocation.chain.stages.len(),
                        if invocation.chain.stages.is_empty() {
                            0
                        } else {
                            started.elapsed().as_micros().min(u128::from(u64::MAX)) as u64
                        },
                    );
                }
                check_cancel(cancel)?;
                ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
                self.remember_native_cpu_frame(&rendered);
                let faults = invocation.chain.clone().finish_faults(failures);
                outcomes.push(NativeCpuOutcome::new(
                    rendered,
                    faults,
                    invocation.chain.source.scope.clone(),
                    clocks,
                ));
                owner.poll_retirement().map_err(materialization_fault)?;
            }
            drop(held);
            owner.poll_retirement().map_err(materialization_fault)?;
            Ok(NativeCpuResults::new(outcomes))
        }

        fn render_native_cpu_chunk(
            &mut self,
            owner: &mut NativeGpuOwner,
            request: NativeFrameId,
            current: Arc<NativeCpuFrameOwner>,
            held: Arc<HeldCpuInputs>,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            clocks: FrozenChainClocks,
            cancel: &dyn VideoRenderCancellation,
        ) -> Result<Arc<NativeCpuFrameOwner>, CpuChunkFailure> {
            check_cancel(cancel)?;
            ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
            if !crate::isf_runtime::native_valid_date(clocks.date_bits.map(f32::from_bits)) {
                return Err(CpuChunkFailure::Canonical(IsfRuntimeError {
                    message: "ISF DATE contains an invalid date or time".into(),
                }));
            }
            self.native_cpu_apply_baseline_evictions(&current.frame)
                .map_err(CpuChunkFailure::Canonical)?;
            owner.poll_retirement().map_err(materialization_fault)?;
            self.native_cpu_validate_input(&current.frame)
                .map_err(CpuChunkFailure::Canonical)?;
            let pipeline_pins = self.prepare_cpu_pipeline_chunk(
                owner, request, &current, &held, stages, keys, cancel,
            )?;
            check_cancel(cancel)?;
            self.native_cpu_apply_take_evictions(&current.frame, stages, keys)
                .map_err(CpuChunkFailure::Canonical)?;
            owner.poll_retirement().map_err(materialization_fault)?;
            let audio = self.native_audio_snapshot();
            let mut chunk_plan = self
                .native_cpu_chunk_plan(request, &current.frame, stages, keys, audio.as_ref())
                .map_err(CpuChunkFailure::Canonical)?;
            let held_ids = held.ids();
            for id in &held_ids {
                chunk_plan.plan.add_existing(*id);
            }
            let current_ids: Vec<_> = current.charge.iter().map(|charge| charge.id()).collect();
            for id in &current_ids {
                chunk_plan.plan.add_existing(*id);
            }
            // The admission envelope was validated without intervening owners.
            // A late mismatch/OOM is terminal, never an unexecuted decline.
            let reservation = reserve_plan(owner, &chunk_plan.plan, GpuPhase::IsfBatch)?;
            let mut txn = owner
                .begin_frame(request, reservation)
                .map_err(materialization_fault)?;
            let retirement = txn.id();
            let pending_fault = |fault| MaterializationFailure::Fault {
                fault,
                retirement: Some(retirement),
            };
            txn.retain_resource(Arc::clone(&held), &held_ids)
                .map_err(pending_fault)?;
            txn.retain_resource(Arc::clone(&current), &current_ids)
                .map_err(pending_fault)?;
            let mut phase = txn.begin_phase(GpuPhase::IsfBatch).map_err(pending_fault)?;
            phase
                .retain_resource(pipeline_pins, &[])
                .map_err(pending_fault)?;
            let (mut encoded, encode_failure) = match self.native_encode_cpu_chunk(
                &mut phase,
                &current.frame,
                stages,
                keys,
                clocks,
            ) {
                Ok(chunk) => (Some(chunk), None),
                Err(fault) => {
                    phase.record_failure(fault.kind, fault.message.clone());
                    (None, Some(fault))
                }
            };
            let commands = encoded
                .as_mut()
                .map(|chunk| chunk.take_commands())
                .unwrap_or_default();
            let phase = phase.finish(commands);
            let receipt = txn.submit_phase(phase).map_err(pending_fault)?;
            let pending = txn.finish();
            let mut mapping = encoded
                .as_ref()
                .filter(|chunk| chunk.host_error().is_none())
                .map(|chunk| NativeReadbackMap::start(chunk.readback()));
            let completed = await_transaction(owner, pending, cancel);
            let mut completed = match completed {
                Ok(completed) => completed,
                Err(error) => {
                    if let Some(mapping) = &mut mapping {
                        mapping.unmap();
                    }
                    drop(mapping);
                    if let Some(chunk) = encoded {
                        self.native_finish_cpu_chunk(chunk, false, true);
                    }
                    return Err(CpuChunkFailure::Terminal(error));
                }
            };
            let validation = completed
                .take_phase(receipt)
                .and_then(|phase| phase.into_validation());
            if let Some(fault) = encode_failure {
                if let Some(mapping) = &mut mapping {
                    mapping.unmap();
                }
                drop(mapping);
                if let Some(chunk) = encoded {
                    self.native_finish_cpu_chunk(chunk, false, true);
                }
                return Err(CpuChunkFailure::Terminal(materialization_fault(fault)));
            }
            let chunk = encoded.expect("canonical encoder owns its completed CPU resources");
            if let Err(fault) = validation {
                if let Some(mapping) = &mut mapping {
                    mapping.unmap();
                }
                drop(mapping);
                self.native_finish_cpu_chunk(chunk, false, true);
                owner.poll_retirement().map_err(materialization_fault)?;
                return Err(CpuChunkFailure::Canonical(IsfRuntimeError {
                    message: format!("ISF GPU pass failed: {}", fault.message),
                }));
            }
            if let Some(error) = chunk.host_error().cloned() {
                self.native_finish_cpu_chunk(chunk, false, false);
                owner.poll_retirement().map_err(materialization_fault)?;
                return Err(CpuChunkFailure::Canonical(error));
            }
            let mut mapping = mapping.expect("successful CPU encoder has a readback map");
            if let Err(error) =
                receive_map(owner, &mut mapping, request, GpuPhase::IsfBatch, cancel)
            {
                mapping.unmap();
                drop(mapping);
                self.native_finish_cpu_chunk(chunk, false, false);
                return match error {
                    MaterializationFailure::Fault {
                        fault,
                        retirement: None,
                    } if fault.kind == NativeGpuFaultKind::Map => {
                        Err(CpuChunkFailure::Canonical(IsfRuntimeError {
                            message: format!("ISF readback failed: {}", fault.message),
                        }))
                    }
                    other => Err(CpuChunkFailure::Terminal(other)),
                };
            }
            let output = chunk.copy_mapped();
            mapping.unmap();
            drop(mapping);
            self.native_finish_cpu_chunk(chunk, true, false);
            // Canonical feedback is committed after successful readback even if
            // the publication checkpoint immediately observes cancellation.
            check_cancel(cancel)?;
            ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
            owner.poll_retirement().map_err(materialization_fault)?;
            Ok(output)
        }

        fn prepare_cpu_pipeline_chunk(
            &mut self,
            owner: &mut NativeGpuOwner,
            request: NativeFrameId,
            current: &Arc<NativeCpuFrameOwner>,
            held: &Arc<HeldCpuInputs>,
            stages: &[IsfStageInputs<'_>],
            keys: &[IsfInstanceKey],
            cancel: &dyn VideoRenderCancellation,
        ) -> Result<Arc<Vec<Arc<crate::isf_runtime::IsfGpuPipeline>>>, CpuChunkFailure> {
            check_cancel(cancel)?;
            let mut plan = NativeAllocationPlan::new(request);
            let held_ids = held.ids();
            let current_ids: Vec<_> = current.charge.iter().map(|charge| charge.id()).collect();
            for id in held_ids.iter().chain(&current_ids) {
                plan.add_existing(*id);
            }
            let reservation = reserve_plan(owner, &plan, GpuPhase::IsfBatch)?;
            let mut txn = owner
                .begin_frame(request, reservation)
                .map_err(materialization_fault)?;
            let retirement = txn.id();
            let pending_fault = |fault| MaterializationFailure::Fault {
                fault,
                retirement: Some(retirement),
            };
            txn.retain_resource(Arc::clone(held), &held_ids)
                .map_err(pending_fault)?;
            txn.retain_resource(Arc::clone(current), &current_ids)
                .map_err(pending_fault)?;
            let mut phase = txn.begin_phase(GpuPhase::IsfBatch).map_err(pending_fault)?;
            let (pins, prepared) = self.native_prepare_cpu_pipelines(&phase, stages);
            let pins = Arc::new(pins);
            let host_error = prepared.err();
            phase
                .retain_resource(Arc::clone(&pins), &[])
                .map_err(pending_fault)?;
            let phase = phase.finish(Vec::new());
            let receipt = txn.submit_phase(phase).map_err(pending_fault)?;
            let pending = txn.finish();
            let mut resolved = await_transaction(owner, pending, cancel)?;
            if let Err(fault) = resolved
                .take_phase(receipt)
                .and_then(|phase| phase.into_validation())
            {
                self.native_cpu_invalidate_keys(keys);
                owner.poll_retirement().map_err(materialization_fault)?;
                return Err(CpuChunkFailure::Canonical(IsfRuntimeError {
                    message: format!("ISF GPU pass failed: {}", fault.message),
                }));
            }
            if let Some(error) = host_error {
                return Err(CpuChunkFailure::Canonical(error));
            }
            check_cancel(cancel)?;
            ensure_publication_active(owner, request, GpuPhase::IsfBatch)?;
            Ok(pins)
        }

        pub(crate) fn prepare_native_cpu_inputs(
            &self,
            frame: NativeFrameId,
            invocation: PreparedNativeCpuInvocation,
        ) -> Result<PreparedNativeInputs, NativeGpuFault> {
            self.prepare_cpu_invocations(frame, vec![Arc::new(invocation)])
        }

        pub(crate) fn prepare_native_cpu_fallback(
            &self,
            inputs: PreparedNativeInputs,
        ) -> Result<PreparedNativeInputs, NativeGpuFault> {
            match inputs.work {
                PreparedNativeWork::Cpu(work) => {
                    self.prepare_cpu_invocations(inputs.frame, work.invocations)
                }
                PreparedNativeWork::Layers(jobs) => {
                    let invocations = jobs
                        .into_iter()
                        .map(|job| {
                            let (input, chain, date) = match Arc::try_unwrap(job) {
                                Ok(job) => (job.input, job.chain, job.date),
                                Err(job) => (Arc::clone(&job.input), job.chain.clone(), job.date),
                            };
                            Arc::new(PreparedNativeCpuInvocation::new(
                                input,
                                chain,
                                date,
                                inputs.audio.clone(),
                            ))
                        })
                        .collect();
                    self.prepare_cpu_invocations(inputs.frame, invocations)
                }
            }
        }

        fn prepare_cpu_invocations(
            &self,
            frame: NativeFrameId,
            invocations: Vec<Arc<PreparedNativeCpuInvocation>>,
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
                    "native CPU preparation requires its shared runtime",
                )
            })?;
            let known = self.native_cpu_cached_inputs();
            let mut inputs = Vec::new();
            let mut seen = HashSet::new();
            let mut collect = |input: CpuInput| {
                if seen.insert(input.identity()) {
                    let input = match input {
                        CpuInput::Frame(frame) => known
                            .iter()
                            .find(|prior| Arc::ptr_eq(&prior.frame, &frame))
                            .map(|prior| CpuInput::Existing(Arc::clone(prior)))
                            .unwrap_or(CpuInput::Frame(frame)),
                        other => other,
                    };
                    inputs.push(input);
                }
            };
            for invocation in &invocations {
                collect(CpuInput::Frame(Arc::clone(&invocation.input)));
                for stage in &invocation.chain.stages {
                    for (_, image) in &stage.images {
                        collect(CpuInput::Frame(Arc::clone(image)));
                    }
                    for image in stage
                        .shader
                        .imported_images
                        .iter()
                        .chain(&stage.shader.bound_images)
                    {
                        collect(CpuInput::Bytes(Arc::clone(&image.rgba)));
                    }
                }
                if let Some(audio) = &invocation.audio {
                    collect(CpuInput::Audio(Arc::clone(audio)));
                }
            }
            let cache_stamp = self.native_cpu_cache_stamp();
            let plan = (|| {
                let mut plan = NativeAllocationPlan::new(frame);
                let mut active_inputs = 0;
                for input in &inputs {
                    active_inputs = checked_sum(active_inputs, input.new_bytes())?;
                    if let CpuInput::Existing(owner) = input {
                        if let Some(charge) = &owner.charge {
                            plan.add_existing(charge.id());
                        }
                    }
                }
                for (id, _) in &cache_stamp.1 {
                    plan.add_existing(*id);
                }
                let mut cache = self.native_cpu_cache_model();
                let initial_credit = cache.reclaimable_bytes();
                if let Some(first) = invocations.iter().find(|invocation| {
                    crate::resolved_chain_executes(&invocation.chain.source)
                        && !invocation.chain.stages.is_empty()
                        && !invocation.chain.keys.is_empty()
                        && crate::isf_runtime::native_valid_date(invocation.date)
                }) {
                    // The matching baseline eviction runs before importing the
                    // newly admitted CPU owners. No GPU resources are created.
                    let _ = self.native_cpu_simulate_baseline(&mut cache, &first.input);
                }
                let mut serial_peak = checked_sum(active_inputs, cache.reclaimable_bytes())?;
                let mut completed_outputs = 0;
                for invocation in &invocations {
                    if !crate::resolved_chain_executes(&invocation.chain.source)
                        || !crate::isf_runtime::native_valid_date(invocation.date)
                    {
                        continue;
                    }
                    let pixels =
                        rgba_bytes(invocation.input.width, invocation.input.height).unwrap_or(0);
                    let mut has_output = false;
                    with_stages(invocation, |stages| -> Result<(), NativeGpuDecline> {
                        let mut index = 0;
                        while index < stages.len() {
                            if index >= invocation.chain.keys.len() {
                                break;
                            }
                            let mut end = index + 1;
                            if stages[index].can_share_host_buffers() {
                                while end < stages.len()
                                    && end < invocation.chain.keys.len()
                                    && stages[end].can_share_host_buffers()
                                {
                                    end += 1;
                                }
                            }
                            let before = cache.clone();
                            let prior_output = has_output;
                            let live_cpu = checked_sum(
                                checked_sum(active_inputs, completed_outputs)?,
                                if has_output { pixels } else { 0 },
                            )?;
                            let success = match self.native_cpu_simulate_chunk(
                                &mut cache,
                                frame,
                                &invocation.input,
                                !has_output,
                                &stages[index..end],
                                &invocation.chain.keys[index..end],
                                invocation.audio.as_deref(),
                            ) {
                                Ok(peak) => {
                                    serial_peak = serial_peak.max(checked_sum(live_cpu, peak)?);
                                    true
                                }
                                Err(_) => {
                                    serial_peak = serial_peak
                                        .max(checked_sum(live_cpu, cache.reclaimable_bytes())?);
                                    false
                                }
                            };
                            has_output |= success;
                            if end > index + 1 {
                                let mut recovery_cache = before;
                                let mut recovery_output = prior_output;
                                for stage_index in index..end {
                                    let live_cpu = checked_sum(
                                        checked_sum(active_inputs, completed_outputs)?,
                                        if recovery_output { pixels } else { 0 },
                                    )?;
                                    if let Ok(peak) = self.native_cpu_simulate_chunk(
                                        &mut recovery_cache,
                                        frame,
                                        &invocation.input,
                                        !recovery_output,
                                        std::slice::from_ref(&stages[stage_index]),
                                        std::slice::from_ref(&invocation.chain.keys[stage_index]),
                                        invocation.audio.as_deref(),
                                    ) {
                                        serial_peak = serial_peak.max(checked_sum(live_cpu, peak)?);
                                        recovery_output = true;
                                    }
                                }
                                if !success {
                                    cache = recovery_cache;
                                    has_output = recovery_output;
                                }
                            }
                            index = end;
                        }
                        Ok(())
                    })?;
                    if has_output {
                        completed_outputs = checked_sum(completed_outputs, pixels)?;
                    }
                }
                // Only capacity whose sole physical cache owner can retire is
                // credited. Aliases/retired/external owners remain in the live
                // ledger and are never counted as free by this envelope.
                let peak = crate::isf_runtime::NativeCpuCacheModel::additional_peak(
                    initial_credit,
                    serial_peak,
                );
                if peak != 0 {
                    plan.add_new(ChargeKind::SerialCpuAdmission, peak)?;
                }
                Ok(plan)
            })();
            Ok(PreparedNativeInputs {
                frame,
                context: context.id(),
                audio: None,
                work: PreparedNativeWork::Cpu(PreparedCpuWork {
                    invocations,
                    inputs,
                    cache_stamp,
                }),
                plan,
                clocks: None,
                pooled: Vec::new(),
            })
        }
    }
}

#[cfg(test)]
#[path = "isf_native_gpu_tests.rs"]
pub(crate) mod tests;
