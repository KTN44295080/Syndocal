//! Extended resident stages reuse the canonical pass scheduler. The pooled
//! scratch owner, held strongly by G and weakly by the runtime, owns feedback.
//! An encoded tick cannot publish its new feedback until its real scopes pass.
use super::*;
use crate::native_gpu_budget::NativeAllocationPlan;
use crate::native_gpu_context::GpuPhaseEncoder;
use crate::native_gpu_contract::{FrozenChainClocks, NativeFrameId};
use std::sync::Arc;

#[derive(Default)]
pub(super) struct ResidentStates {
    committed: BTreeMap<IsfInstanceKey, StageResources>,
    pending: Option<(NativeFrameId, Vec<(IsfInstanceKey, StageResources)>)>,
}

impl NativeStackResources {
    pub(crate) fn finish_resident(&self, frame: NativeFrameId, success: bool) {
        let Ok(mut state) = self.resident.lock() else {
            return;
        };
        if state
            .pending
            .as_ref()
            .is_some_and(|(pending, _)| *pending == frame)
        {
            let (_, pending) = state.pending.take().unwrap();
            if success {
                state.committed.extend(pending);
            } else {
                state.committed.clear();
            }
        }
    }
}

fn cache_model(entries: &BTreeMap<IsfInstanceKey, StageResources>) -> NativeCpuCacheModel {
    let entries = entries
        .iter()
        .map(|(key, state)| {
            let allocations = state
                .targets
                .iter()
                .flat_map(|target| &target.images)
                .chain(&state.imports)
                .chain(&state.bound_images)
                .chain(&state.audio)
                .filter_map(|image| image.native_charge.as_ref().map(|charge| charge.id()))
                .collect();
            (
                key.clone(),
                cpu_bridge::NativeCpuCacheEntry {
                    fingerprint: state.fingerprint,
                    sizes: state.sizes.clone(),
                    bytes: state.bytes,
                    last_used: state.last_used,
                    audio_identity: state.audio_identity,
                    audio_initialized: state.audio_initialized,
                    allocations,
                    reclaimable: false,
                },
            )
        })
        .collect();
    NativeCpuCacheModel::new(entries, 0)
}

impl IsfGpuRuntime {
    /// Auxiliary allocations only: primary upload/scratch/final/uniform pool
    /// slots are already described by the selected resident stack plan.
    pub(crate) fn native_resident_resource_plan(
        &self,
        request: NativeFrameId,
        frame: &VideoFrame,
        stages: &[IsfStageInputs<'_>],
        keys: &[IsfInstanceKey],
        audio: Option<&protocol::LiveAudioTextureFrame>,
        resources: Option<&NativeStackResources>,
    ) -> Result<NativeAllocationPlan, IsfRuntimeError> {
        let state = resources
            .map(|resources| {
                resources
                    .resident
                    .lock()
                    .map_err(|_| runtime_error("resident ISF feedback state is poisoned"))
            })
            .transpose()?;
        if state.as_ref().is_some_and(|state| state.pending.is_some()) {
            return Err(runtime_error(
                "resident ISF feedback is awaiting GPU validation",
            ));
        }
        let empty = BTreeMap::new();
        let model = cache_model(
            state
                .as_ref()
                .map(|state| &state.committed)
                .unwrap_or(&empty),
        );
        let mut plan = NativeAllocationPlan::new(request);
        self.native_stage_resource_plan(&mut plan, frame, stages, keys, audio, &model)?;
        Ok(plan)
    }

    pub(crate) fn native_encode_resident_resources(
        &mut self,
        phase: &mut GpuPhaseEncoder<'_>,
        resources: &NativeStackResources,
        frame: &Arc<VideoFrame>,
        stages: &[IsfStageInputs<'_>],
        keys: &[IsfInstanceKey],
        clocks: FrozenChainClocks,
        request: NativeFrameId,
    ) -> Result<(wgpu::CommandBuffer, Vec<Arc<IsfGpuPipeline>>), IsfRuntimeError> {
        if self
            .native_context()
            .is_none_or(|context| !Arc::ptr_eq(context, phase.context()))
        {
            return Err(runtime_error(
                "resident ISF resources require their shared device phase",
            ));
        }
        let mut state = resources
            .resident
            .lock()
            .map_err(|_| runtime_error("resident ISF feedback state is poisoned"))?;
        if state.pending.is_some() {
            return Err(runtime_error(
                "resident ISF feedback is awaiting GPU validation",
            ));
        }
        // Keep canonical CPU-preview/fallback feedback separate from resident
        // Display ownership, including when the same authored stage is present.
        let previous = std::mem::replace(
            &mut self.resources.entries,
            std::mem::take(&mut state.committed),
        );
        let mut pending = PendingStack::new(
            frame,
            Some(Arc::clone(frame)),
            Cow::Owned(keys.to_vec()),
            stack_transaction::StackClocks {
                time_seconds: f32::from_bits(clocks.time_seconds_bits),
                time_delta_seconds: f32::from_bits(clocks.time_delta_seconds_bits),
                frame_index: clocks.frame_index,
                date: clocks.date_bits.map(f32::from_bits),
            },
            Some(resources.workspace.clone()),
        );
        let mut native = NativeCpuAllocationScope {
            phase,
            buffers: Vec::new(),
            failure: None,
            primary_upload_precharged: true,
        };
        let result =
            self.encode_pass_stack(&mut pending, frame, stages, Some(0), Some(&mut native));
        let unused = std::mem::replace(&mut self.resources.entries, previous);
        // Ownership of every used image was already retained by the broker,
        // including partial state on a host error or a failed shader scope.
        if result.is_ok() {
            state.committed = unused;
            state.pending = Some((request, std::mem::take(&mut pending.states)));
        } else {
            state.committed.clear();
        }
        let pins = std::mem::take(&mut pending.native_pipeline_pins);
        native
            .phase
            .retain_resource(Arc::new(pins.clone()), &[])
            .map_err(|fault| runtime_error(fault.message))?;
        if let Some(fault) = native.failure {
            return Err(runtime_error(fault.message));
        }
        result?;
        let command = pending
            .encoder
            .take()
            .expect("canonical resident encoder")
            .finish();
        Ok((command, pins))
    }
}
