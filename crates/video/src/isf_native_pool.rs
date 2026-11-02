//! Resident ISF scratch and final-image pools. Strong cache ownership belongs
//! to G; runtime lookups are weak so owner invalidation releases idle storage.
use super::*;
use std::sync::Weak;

#[derive(Clone, Debug, PartialEq, Eq)]
struct StackKey {
    width: u32,
    height: u32,
    format: VideoPixelFormat,
    stages: usize,
    output: u64,
    geometry: u64,
    epoch: u64,
    resources: Option<(u64, u64, Vec<IsfInstanceKey>, Vec<u64>)>,
}
impl StackKey {
    fn new(epoch: u64, frame: NativeFrameId, job: &PreparedNativeLayer) -> Self {
        Self {
            width: job.input.width,
            height: job.input.height,
            format: job.input.format,
            stages: job.chain.stages.len(),
            output: frame.output_id,
            geometry: frame.geometry_generation,
            epoch,
            resources: job.requires_retained_history().then(|| {
                (
                    frame.project_epoch,
                    job.input.layer_id,
                    job.chain.keys.clone(),
                    job.chain
                        .stages
                        .iter()
                        .map(|stage| stage.shader.fingerprint)
                        .collect(),
                )
            }),
        }
    }
}
struct ScratchAllocation {
    resources: NativeStackResources,
    charges: Vec<AllocationCharge>,
}
impl ScratchAllocation {
    fn ids(&self) -> Vec<GpuAllocationId> {
        self.charges.iter().map(AllocationCharge::id).collect()
    }
}
#[derive(Default)]
pub(crate) struct NativeStackPool {
    scratch: Vec<(StackKey, Weak<ScratchAllocation>)>,
    finals: Vec<(StackKey, Weak<NativeTextureAllocation>)>,
}
/// These strong checkout pins are captured during preflight, so neither another
/// prepare nor encoding can substitute an allocation after budget admission.
pub(super) struct StackSelection {
    key: StackKey,
    scratch: Option<Arc<ScratchAllocation>>,
    final_image: Option<Arc<NativeTextureAllocation>>,
}
fn checkout<T>(entries: &[(StackKey, Weak<T>)], key: &StackKey) -> Option<Arc<T>> {
    entries.iter().find_map(|(old, weak)| {
        if old != key {
            return None;
        }
        // Upgrade before inspecting: two concurrent read-only preflights must
        // never both observe a sole owner and then acquire the same resource.
        let resource = weak.upgrade()?;
        (Arc::strong_count(&resource) == 2).then_some(resource)
    })
}
impl NativeStackPool {
    pub(super) fn select(
        &self,
        epoch: u64,
        frame: NativeFrameId,
        job: &PreparedNativeLayer,
        pending: &[StackSelection],
    ) -> Result<StackSelection, NativeGpuDecline> {
        let key = StackKey::new(epoch, frame, job);
        let selection = StackSelection {
            key: key.clone(),
            scratch: checkout(&self.scratch, &key),
            final_image: checkout(&self.finals, &key),
        };
        fn capacity(key: &StackKey, scratch: bool) -> u64 {
            let pixels = u64::from(key.width) * u64::from(key.height) * 4;
            if scratch {
                pixels * 2 + key.stages as u64 * UNIFORM_BYTES
            } else {
                pixels
            }
        }
        // Ordinary stacks need three final generations. Extended/stateful
        // stacks retain all eight history generations plus the incoming tick;
        // a mixed frame shares that pool capacity, independent of job order.
        // A full pool declines before clock/upload/allocation. It never grows
        // a second workspace set to hide pending retirement or history pins.
        for (scratch, limit) in [
            (true, 3usize),
            (
                false,
                if key.resources.is_some()
                    || self.finals.iter().any(|(old, weak)| {
                        old.epoch == epoch && old.resources.is_some() && weak.strong_count() != 0
                    })
                    || pending.iter().any(|entry| entry.key.resources.is_some())
                {
                    27usize
                } else {
                    9usize
                },
            ),
        ] {
            if (scratch && selection.scratch.is_some())
                || (!scratch && selection.final_image.is_some())
            {
                continue;
            }
            let accumulate = |(count, bytes, largest): (usize, u64, u64), size: u64| {
                (count + 1, bytes + size, largest.max(size))
            };
            let initial = if scratch {
                self.scratch
                    .iter()
                    .filter(|(old, weak)| old.epoch == epoch && weak.strong_count() != 0)
                    .map(|(old, _)| capacity(old, true))
                    .fold((0, 0, 0), accumulate)
            } else {
                self.finals
                    .iter()
                    .filter(|(old, weak)| old.epoch == epoch && weak.strong_count() != 0)
                    .map(|(old, _)| capacity(old, false))
                    .fold((0, 0, 0), accumulate)
            };
            let (count, live, largest) = pending
                .iter()
                .filter(|entry| {
                    if scratch {
                        entry.scratch.is_none()
                    } else {
                        entry.final_image.is_none()
                    }
                })
                .map(|entry| capacity(&entry.key, scratch))
                .fold(initial, accumulate);
            if count >= limit {
                let requested = capacity(&key, scratch);
                return Err(NativeGpuDecline::BudgetExceeded {
                    live,
                    requested,
                    limit: largest.max(requested) * limit as u64,
                });
            }
        }
        Ok(selection)
    }
    fn prune(&mut self) {
        self.scratch.retain(|(_, entry)| entry.strong_count() != 0);
        self.finals.retain(|(_, entry)| entry.strong_count() != 0);
    }
}
impl StackSelection {
    pub(super) fn current(
        &self,
        epoch: u64,
        frame: NativeFrameId,
        job: &PreparedNativeLayer,
    ) -> bool {
        self.key == StackKey::new(epoch, frame, job)
    }
    fn uniform_upload(&self) -> u64 {
        // Each queue.write_buffer may retain an aligned backend staging block.
        UNIFORM_BYTES.div_ceil(wgpu::COPY_BUFFER_ALIGNMENT)
            * wgpu::COPY_BUFFER_ALIGNMENT
            * self.key.stages as u64
    }
    pub(super) fn add_plan(
        &self,
        plan: &mut NativeAllocationPlan,
        job: &PreparedNativeLayer,
        pitch: Option<u64>,
        runtime: &IsfGpuRuntime,
    ) -> Result<(), NativeGpuDecline> {
        let input = &job.input;
        let pixels = rgba_bytes(input.width, input.height)?;
        if let Some(scratch) = &self.scratch {
            for id in scratch.ids() {
                plan.add_existing(id);
            }
        } else {
            plan.add_new(ChargeKind::IsfScratch, pixels)?;
            plan.add_new(ChargeKind::IsfScratch, pixels)?;
            for _ in 0..self.key.stages {
                plan.add_new(ChargeKind::CachedEffectResource, UNIFORM_BYTES)?;
            }
        }
        if let Some(image) = &self.final_image {
            plan.add_existing(image.id());
        } else {
            plan.add_new(ChargeKind::ResidentLayer, pixels)?;
        }
        plan.add_new(ChargeKind::ActiveCpuInput, input.data.capacity() as u64)?;
        plan.add_new(
            ChargeKind::QueueUploadStaging,
            checked_upload_staging_bytes(input.width, input.height, pitch)?,
        )?;
        plan.add_new(ChargeKind::QueueUploadStaging, self.uniform_upload())?;
        if job.requires_retained_history() {
            let resources = self.scratch.as_ref().map(|scratch| &scratch.resources);
            let auxiliary = runtime
                .native_resident_resource_plan(
                    plan.frame(),
                    input,
                    &job.stages(),
                    &job.chain.keys,
                    runtime.native_audio_snapshot().as_ref(),
                    resources,
                )
                .map_err(|_| unsupported(UnsupportedNativeFx::AuxiliaryResource))?;
            plan.merge(&auxiliary)?;
        }
        Ok(())
    }
}
impl IsfGpuRuntime {
    pub(super) fn finish_native_resident_resources(&mut self, frame: NativeFrameId, success: bool) {
        for (_, weak) in &self.native_stack_pool.scratch {
            if let Some(scratch) = weak.upgrade() {
                scratch.resources.finish_resident(frame, success);
            }
        }
    }
    #[cfg(test)]
    pub(crate) fn clear_native_stack_pool(&mut self) {
        // G's release_resource_pools removes the owning cache. Clearing lookup
        // metadata cannot invalidate outstanding transaction or history owners.
        self.native_stack_pool = NativeStackPool::default();
    }
    pub(super) fn acquire_native_stack(
        &mut self,
        phase: &mut crate::native_gpu_context::GpuPhaseEncoder<'_>,
        job: &Arc<PreparedNativeLayer>,
        clocks: FrozenChainClocks,
        request: NativeFrameId,
        selected: &StackSelection,
    ) -> Result<(Arc<NativeStackResources>, Arc<NativeTextureAllocation>), NativeGpuFault> {
        let context = phase.context().id();
        let invalid = || {
            fault(
                context,
                request,
                GpuPhase::IsfBatch,
                NativeGpuFaultKind::StateMismatch,
                "pooled stack preflight is no longer current",
            )
        };
        if selected.key != StackKey::new(phase.context().resource_pool_epoch(), request, job) {
            return Err(invalid());
        }
        self.native_stack_pool.prune();
        let pixels = rgba_bytes(job.input.width, job.input.height).map_err(|_| invalid())?;
        let scratch = if let Some(scratch) = &selected.scratch {
            Arc::clone(scratch)
        } else {
            let mut charges = vec![
                phase.bind_allocation(ChargeKind::IsfScratch, pixels)?,
                phase.bind_allocation(ChargeKind::IsfScratch, pixels)?,
            ];
            for _ in 0..selected.key.stages {
                charges
                    .push(phase.bind_allocation(ChargeKind::CachedEffectResource, UNIFORM_BYTES)?);
            }
            let resources = self
                .native_create_pool_scratch(phase, &job.input, selected.key.stages)
                .map_err(|error| {
                    fault(
                        context,
                        request,
                        GpuPhase::IsfBatch,
                        NativeGpuFaultKind::Validation,
                        error.message,
                    )
                })?;
            let scratch = Arc::new(ScratchAllocation { resources, charges });
            phase.retain_pooled_resource(Arc::clone(&scratch))?;
            self.native_stack_pool
                .scratch
                .push((selected.key.clone(), Arc::downgrade(&scratch)));
            scratch
        };
        phase.retain_resource(Arc::clone(&scratch), &scratch.ids())?;
        let final_image = if let Some(image) = &selected.final_image {
            Arc::clone(image)
        } else {
            let charge = phase.bind_allocation(ChargeKind::ResidentLayer, pixels)?;
            let image = self
                .native_pool_final_image(phase, job.input.width, job.input.height)
                .map_err(|error| {
                    fault(
                        context,
                        request,
                        GpuPhase::IsfBatch,
                        NativeGpuFaultKind::Validation,
                        error.message,
                    )
                })?;
            let image = Arc::new(NativeTextureAllocation::new(image, charge));
            phase.retain_pooled_resource(Arc::clone(&image))?;
            self.native_stack_pool
                .finals
                .push((selected.key.clone(), Arc::downgrade(&image)));
            image
        };
        // Retain before any queue write; failure/cancellation keeps both GPU
        // allocations and staging charges in G until verified queue retirement.
        phase.retain_resource(Arc::clone(&final_image), &[final_image.id()])?;
        let uniform_upload = Arc::new(
            phase.bind_allocation(ChargeKind::QueueUploadStaging, selected.uniform_upload())?,
        );
        phase.retain_resource(Arc::clone(&uniform_upload), &[uniform_upload.id()])?;
        phase.mark_queue_write();
        let resources = self.native_assemble_pool_stack(
            &scratch.resources,
            final_image.pool_image(),
            &job.input,
            &job.stages(),
            clocks,
        );
        Ok((Arc::new(resources), final_image))
    }
}

#[cfg(test)]
#[path = "isf_native_pool_tests.rs"]
mod tests;
