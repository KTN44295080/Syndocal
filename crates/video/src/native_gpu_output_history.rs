//! I-owned output-session history: one host ledger and affine per-input publication.
//! P owns sealing, history policy and CPU backing. This adapter joins their real
//! owners without exposing a pixel Arc, repeating FX, or changing production gates.
use std::sync::Arc;

use crate::frame_history_policy::{HistoryMeta, HistoryPlan};
use crate::isf_runtime::StackBatch;
use crate::native_gpu_contract::{GpuContextId, LayerResultIdentity, NativeFrameId};
use crate::native_gpu_history::adapter::{NativeGpuHistory, NativeGpuValidatedEntries};
use crate::native_gpu_history::{
    HistoryCommit, HistoryError, HistoryHostBudget, HistoryScope, RetiredResident,
};
use crate::native_gpu_result::{
    CanonicalCpuLayerOutcome, GpuLayerLease, IsfValidatedReceipt, ValidatedLayerResult,
};
use crate::{VideoEffectStageFault, VideoPixelFormat};

/// The ledger is created once for the output session and shared with every input.
pub(crate) struct NativeOutputHistorySession {
    identity: Arc<()>,
    context: GpuContextId,
    host: HistoryHostBudget,
    histories: Vec<NativeGpuHistory>,
}

struct PlannedInput {
    history: usize,
    identity: LayerResultIdentity,
    plan: Option<HistoryPlan>,
}

/// Prospective policy decisions made before encoding, not recomputed on completion.
pub(crate) struct NativeOutputHistoryPlan {
    session: Arc<()>,
    context: GpuContextId,
    frame: NativeFrameId,
    inputs: Vec<PlannedInput>,
}

impl NativeOutputHistoryPlan {
    pub(crate) fn context(&self) -> GpuContextId {
        self.context
    }

    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
}

enum HistoryLayerState {
    Validated(ValidatedLayerResult),
    Recovered(CanonicalCpuLayerOutcome),
    Sealed(NativeGpuValidatedEntries),
    Committed(HistoryCommit<GpuLayerLease>),
}

/// Retains every layer on seal/commit rejection, including already committed
/// inputs' retired residents. A partial commit is never reported as complete.
pub(crate) struct NativeOutputHistoryFrame {
    plan: NativeOutputHistoryPlan,
    receipt: Option<IsfValidatedReceipt>,
    layers: StackBatch<Option<HistoryLayerState>>,
    failure: Option<HistoryError>,
    composition: Option<bool>,
    recovered: bool,
    faults: Vec<VideoEffectStageFault>,
}

impl NativeOutputHistoryFrame {
    pub(crate) fn receipt(&self) -> Option<&IsfValidatedReceipt> {
        self.receipt.as_ref()
    }

    pub(crate) fn error(&self) -> Option<HistoryError> {
        self.failure
    }

    pub(crate) fn is_recovered(&self) -> bool {
        self.recovered
    }

    pub(crate) fn faults(&self) -> &[VideoEffectStageFault] {
        &self.faults
    }

    pub(crate) fn is_committed(&self) -> bool {
        self.failure.is_none()
            && self
                .layers
                .iter()
                .all(|layer| matches!(layer, Some(HistoryLayerState::Committed(_))))
    }

    pub(crate) fn output_phases_valid(&self) -> bool {
        !self.recovered
            && self.composition == Some(true)
            && self.is_committed()
            && self.layers.iter().all(|layer| {
                matches!(layer, Some(HistoryLayerState::Committed(commit)) if commit.output_phases_valid)
            })
    }

    pub(crate) fn take_retired(&mut self) -> Vec<RetiredResident<GpuLayerLease>> {
        let mut retired = Vec::new();
        for layer in self.layers.iter_mut() {
            if let Some(HistoryLayerState::Committed(commit)) = layer {
                retired.append(&mut commit.retired);
            }
        }
        retired
    }

    fn fail(&mut self, error: HistoryError) {
        self.failure.get_or_insert(error);
    }
}

impl NativeOutputHistorySession {
    pub(crate) fn new(
        context: GpuContextId,
        scopes: Vec<HistoryScope>,
        extent: (u32, u32),
    ) -> Result<Self, HistoryError> {
        let first = scopes.first().ok_or(HistoryError::InvalidPolicy)?;
        if extent.0 == 0 || extent.1 == 0 {
            return Err(HistoryError::InvalidPixels);
        }
        for (index, scope) in scopes.iter().enumerate() {
            if scope.extent != extent
                || scope.frame_scope.output_id != first.frame_scope.output_id
                || scope.frame_scope.project_epoch != first.frame_scope.project_epoch
                || scope.frame_scope.geometry_generation != first.frame_scope.geometry_generation
                || scope.input.project_render_epoch != scope.frame_scope.project_epoch
                || scopes[..index].iter().any(|old| old.input == scope.input)
            {
                return Err(HistoryError::IdentityMismatch);
            }
        }
        let limit = HistoryHostBudget::planning_limit(scopes.len(), extent)
            .ok_or(HistoryError::HostBudgetExceeded)?;
        let host = HistoryHostBudget::new(limit);
        let histories = scopes
            .into_iter()
            .map(|scope| NativeGpuHistory::new(context, scope, host.clone()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            identity: Arc::new(()),
            context,
            host,
            histories,
        })
    }

    pub(crate) fn host_budget(&self) -> &HistoryHostBudget {
        &self.host
    }

    pub(crate) fn history(&self, index: usize) -> Option<&NativeGpuHistory> {
        self.histories.get(index)
    }

    pub(crate) fn into_histories(self) -> Vec<NativeGpuHistory> {
        self.histories
    }

    pub(crate) fn accepts_plan(&self, plan: &NativeOutputHistoryPlan) -> bool {
        plan.context == self.context && Arc::ptr_eq(&plan.session, &self.identity)
    }

    pub(crate) fn accepts_frame(&self, frame: &NativeOutputHistoryFrame) -> bool {
        self.accepts_plan(&frame.plan)
    }

    pub(crate) fn plan(
        &self,
        identities: &[LayerResultIdentity],
        target_ms: u64,
    ) -> Result<NativeOutputHistoryPlan, HistoryError> {
        let frame = identities.first().ok_or(HistoryError::InvalidPolicy)?.frame;
        if identities.len() > 3 {
            return Err(HistoryError::InvalidPolicy);
        }
        let mut inputs: Vec<PlannedInput> = Vec::with_capacity(identities.len());
        for identity in identities {
            let index = self
                .histories
                .iter()
                .position(|history| history.scope().input == identity.entry.input)
                .ok_or(HistoryError::IdentityMismatch)?;
            let history = &self.histories[index];
            let scope = history.scope();
            if identity.frame != frame
                || identity.context != self.context
                || identity.frame.output_id != scope.frame_scope.output_id
                || identity.frame.project_epoch != scope.frame_scope.project_epoch
                || identity.frame.geometry_generation != scope.frame_scope.geometry_generation
                || identity.entry.queue_epoch != scope.queue_epoch
                || identity.layer_id != scope.layer_id
                || identity.source_generation != scope.source_generation
                || (identity.width, identity.height) != scope.extent
                || identity.format != VideoPixelFormat::Rgba8
                || inputs.iter().any(|input| input.history == index)
            {
                return Err(HistoryError::IdentityMismatch);
            }
            let plan = history.plan(
                &[HistoryMeta {
                    id: identity.entry,
                    pts_ms: identity.pts_ms,
                    duration_ms: identity.duration_ms,
                }],
                target_ms,
            )?;
            inputs.push(PlannedInput {
                history: index,
                identity: *identity,
                plan: Some(plan),
            });
        }
        Ok(NativeOutputHistoryPlan {
            session: Arc::clone(&self.identity),
            context: self.context,
            frame,
            inputs,
        })
    }

    /// Called immediately after G validates ISF, before resolving P composition.
    /// Even a partial seal failure returns one aggregate owning every raw/sealed layer.
    pub(crate) fn seal_validated(
        &self,
        plan: NativeOutputHistoryPlan,
        layers: StackBatch<ValidatedLayerResult>,
        receipt: IsfValidatedReceipt,
    ) -> NativeOutputHistoryFrame {
        let mut frame = NativeOutputHistoryFrame {
            plan,
            receipt: Some(receipt),
            layers: layers.map(|layer| Some(HistoryLayerState::Validated(layer))),
            failure: None,
            composition: None,
            recovered: false,
            faults: Vec::new(),
        };
        if let Err(error) = self.check_unsealed(&frame) {
            frame.fail(error);
            return frame;
        }
        for index in 0..frame.layers.len() {
            let Some(HistoryLayerState::Validated(layer)) = frame.layers[index].take() else {
                unreachable!("validated frame contains raw validated layers");
            };
            match NativeGpuValidatedEntries::seal_isf_layer(
                frame.receipt.as_ref().expect("real ISF receipt retained"),
                layer,
                &self.host,
            ) {
                Ok(sealed) => frame.layers[index] = Some(HistoryLayerState::Sealed(sealed)),
                Err(failure) => {
                    frame.layers[index] = Some(HistoryLayerState::Validated(failure.outcome));
                    frame.fail(failure.error);
                }
            }
        }
        frame
    }

    /// Recovery is CPU history only. Exact stage faults stay with this aggregate
    /// until the caller applies its canonical Error/LastValid publication policy.
    pub(crate) fn seal_recovered(
        &self,
        plan: NativeOutputHistoryPlan,
        layers: StackBatch<CanonicalCpuLayerOutcome>,
    ) -> NativeOutputHistoryFrame {
        // Fault metadata is small and belongs to the whole canonical batch.
        // Preserve its exact order even if identity/budget checks leave some
        // CPU outcomes unsealed and therefore inaccessible through P backing.
        let faults = layers
            .iter()
            .flat_map(|layer| layer.faults.iter().cloned())
            .collect();
        let mut frame = NativeOutputHistoryFrame {
            plan,
            receipt: None,
            layers: layers.map(|layer| Some(HistoryLayerState::Recovered(layer))),
            failure: None,
            composition: None,
            recovered: true,
            faults,
        };
        if let Err(error) = self.check_unsealed(&frame) {
            frame.fail(error);
            return frame;
        }
        for index in 0..frame.layers.len() {
            let Some(HistoryLayerState::Recovered(layer)) = frame.layers[index].take() else {
                unreachable!("recovery frame contains raw CPU outcomes");
            };
            match NativeGpuValidatedEntries::seal_recovered_cpu(frame.plan.frame, layer, &self.host)
            {
                Ok((sealed, _faults)) => {
                    frame.layers[index] = Some(HistoryLayerState::Sealed(sealed));
                }
                Err(failure) => {
                    frame.layers[index] = Some(HistoryLayerState::Recovered(failure.outcome));
                    frame.fail(failure.error);
                }
            }
        }
        frame
    }

    fn check_unsealed(&self, frame: &NativeOutputHistoryFrame) -> Result<(), HistoryError> {
        if !self.accepts_frame(frame) || frame.layers.len() != frame.plan.inputs.len() {
            return Err(HistoryError::IdentityMismatch);
        }
        for (layer, input) in frame.layers.iter().zip(&frame.plan.inputs) {
            let identity = match layer {
                Some(HistoryLayerState::Validated(layer)) => layer.identity,
                Some(HistoryLayerState::Recovered(layer)) => layer.identity,
                _ => return Err(HistoryError::StateMismatch),
            };
            if identity != input.identity {
                return Err(HistoryError::IdentityMismatch);
            }
        }
        Ok(())
    }

    /// Record the real P phase resolution exactly once, then retain ISF history
    /// even if composition failed. Invalid output phases can never produce Ready.
    pub(crate) fn resolve_composition(
        &mut self,
        frame: &mut NativeOutputHistoryFrame,
        valid: bool,
    ) -> Result<(), HistoryError> {
        self.check_publication(frame)?;
        if frame.recovered || frame.composition.is_some() {
            return Err(HistoryError::StateMismatch);
        }
        for layer in frame.layers.iter_mut() {
            let Some(HistoryLayerState::Sealed(sealed)) = layer else {
                return Err(HistoryError::StateMismatch);
            };
            sealed.record_composition(valid)?;
        }
        frame.composition = Some(valid);
        self.commit(frame)
    }

    /// The caller must apply the exact recovered faults before any history changes.
    /// A refused policy leaves the complete sealed CPU owner available to the caller.
    pub(crate) fn commit_recovered_with(
        &mut self,
        frame: &mut NativeOutputHistoryFrame,
        apply_canonical_faults: impl FnOnce(&[VideoEffectStageFault]) -> Result<(), HistoryError>,
    ) -> Result<(), HistoryError> {
        self.check_publication(frame)?;
        if !frame.recovered || frame.composition.is_some() {
            return Err(HistoryError::StateMismatch);
        }
        apply_canonical_faults(&frame.faults)?;
        frame.composition = Some(false);
        self.commit(frame)
    }

    fn check_publication(&self, frame: &NativeOutputHistoryFrame) -> Result<(), HistoryError> {
        if !self.accepts_frame(frame) {
            return Err(HistoryError::IdentityMismatch);
        }
        if let Some(error) = frame.failure {
            return Err(error);
        }
        Ok(())
    }

    fn commit(&mut self, frame: &mut NativeOutputHistoryFrame) -> Result<(), HistoryError> {
        for index in 0..frame.layers.len() {
            let sealed = match frame.layers[index].take() {
                Some(HistoryLayerState::Sealed(sealed)) => sealed,
                state => {
                    frame.layers[index] = state;
                    frame.fail(HistoryError::StateMismatch);
                    return Err(HistoryError::StateMismatch);
                }
            };
            let input = &mut frame.plan.inputs[index];
            let plan = input
                .plan
                .take()
                .expect("uncommitted input retains its plan");
            match self.histories[input.history].commit_validated(plan, sealed) {
                Ok(commit) => {
                    frame.layers[index] = Some(HistoryLayerState::Committed(commit));
                }
                Err(failure) => {
                    input.plan = Some(failure.plan);
                    frame.layers[index] = Some(HistoryLayerState::Sealed(failure.entries));
                    frame.fail(failure.error);
                    return Err(failure.error);
                }
            }
        }
        Ok(())
    }
}
