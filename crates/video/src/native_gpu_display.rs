//! Opaque app-facing owner for one resident Display output session.
//!
//! This is I-owned wiring: G/P identities, budgets, transactions and surface
//! tokens stay crate-private.  The app may create/configure/re-arm one session
//! and bind its renderer to the one shared ISF runtime, but cannot create a
//! second device or bypass the session ledger.

use std::sync::Arc;

#[path = "native_gpu_display_prepare.rs"]
mod prepare;
#[path = "native_gpu_display_present.rs"]
mod present;
#[path = "native_gpu_display_ring.rs"]
mod ring;

#[cfg(all(test, feature = "qa-native-frame-timing"))]
#[path = "native_gpu_display_bench_tests.rs"]
mod bench_tests;
#[cfg(test)]
#[path = "native_gpu_display_control_tests.rs"]
mod control_tests;
#[cfg(test)]
#[path = "native_gpu_display_steady_tests.rs"]
mod steady_tests;
pub use present::{
    NativeResidentDisplayFrame, NativeResidentPresentBoundary, NativeResidentPresentDecision,
    NativeResidentPresentResult, NativeResidentPresentationAuthority,
};

use raw_window_handle::{HasDisplayHandle, HasWindowHandle};

use crate::gpu_surface::native::{AdmittedPhysicalGeometry, NativeSurfaceTarget};
use crate::gpu_surface::UnconfiguredNativeSurface;
use crate::isf_native_gpu::continuation::{
    NativeContinuationAdmission, NativeCpuContinuationFailure, NativeLayerPreparation,
};
use crate::isf_native_gpu::{
    CpuMaterializationRequest, NativeBatchRestrictions, NativeGpuMaterializer,
};
use crate::isf_runtime::StackBatch;
use crate::native_gpu_budget::AllocationCharge;
use crate::native_gpu_composition::{
    CompositionPlanFailure, NativeCompositionLayer, NativeCompositionPlan, NativeGpuCompositor,
    SelectedTextureKind,
};
use crate::native_gpu_context::{
    NativeGpuFactory, NativeGpuOwner, NativeGpuRequirements, RetirementProgress,
};
use crate::native_gpu_contract::NativeFrameId;
use crate::native_gpu_history::{
    CpuHistoryHandover, CpuHistoryMaterializationRequest, CpuHistorySnapshot, HistoryHostBudget,
    HistoryScope,
};
use crate::native_gpu_output::cpu::history::{
    accept_cpu_history_materialization, CpuHistoryEntrySequence, NativeCpuHistoryQueue,
};
use crate::native_gpu_output::history::{NativeHistorySelected, NativeOutputHistorySession};
use crate::native_gpu_output::shared::{
    release_shared_presenter_for_stop, PendingSharedNativeOutput, SharedNativeOutputCache,
    SharedNativeOutputProgress, SharedNativeOutputRetirement, SharedNativePresenter,
};
use crate::native_gpu_output::{NativeOutputAuthority, NativeOutputRequest};
use crate::native_gpu_result::{
    FrozenStatelessRecipe, GpuLayerLease, NativeCpuResults, ValidatedCpuMaterialization,
};
use crate::{
    CompositionLayerPlan, VideoFrameProvider, VideoPreviewRenderer, VideoRenderCancellation,
};

pub struct NativeResidentHistoryInput {
    pub input: protocol::VideoRenderInputKey,
    pub layer_id: protocol::VideoLayerId,
    pub queue_epoch: u64,
    pub source_generation: u64,
}

type ResidentCpuHandover =
    CpuHistoryHandover<FrozenStatelessRecipe, GpuLayerLease, AllocationCharge>;

struct ResidentHistoryActive {
    session: NativeOutputHistorySession,
    sequences: Vec<CpuHistoryEntrySequence>,
    inputs: Vec<protocol::VideoRenderInputKey>,
}

struct ResidentHistoryHandover {
    handovers: Vec<Option<ResidentCpuHandover>>,
    snapshots: Vec<Option<CpuHistorySnapshot<AllocationCharge>>>,
    completed: Vec<Option<NativeCpuHistoryQueue>>,
    inputs: Vec<protocol::VideoRenderInputKey>,
    sequences: Vec<CpuHistoryEntrySequence>,
    retained_materializations: Vec<ValidatedCpuMaterialization>,
}

enum ResidentHistoryState {
    Active(ResidentHistoryActive),
    Handover(ResidentHistoryHandover),
}

pub(crate) enum NativeResidentExecution {
    Gpu(SharedNativeOutputProgress),
    Cpu {
        reason: crate::native_gpu_contract::NativeGpuDecline,
        results: NativeCpuResults,
    },
    Retiring(SharedNativeOutputRetirement),
}

/// The broker has one owner. Canonical CPU execution temporarily moves it
/// into the renderer and returns it before any surface operation can run.
struct ResidentOwner(Option<NativeGpuOwner>);

impl ResidentOwner {
    fn take(&mut self) -> NativeGpuOwner {
        self.0
            .take()
            .expect("native broker is not already in a CPU invocation")
    }

    fn restore(&mut self, owner: NativeGpuOwner) {
        assert!(self.0.is_none(), "native broker returned exactly once");
        self.0 = Some(owner);
    }
}

impl std::ops::Deref for ResidentOwner {
    type Target = NativeGpuOwner;

    fn deref(&self) -> &Self::Target {
        self.0
            .as_ref()
            .expect("native CPU invocation has returned its broker")
    }
}

impl std::ops::DerefMut for ResidentOwner {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.0
            .as_mut()
            .expect("native CPU invocation has returned its broker")
    }
}

pub struct NativeResidentDisplaySession {
    output_id: u64,
    factory: NativeGpuFactory,
    owner: ResidentOwner,
    presenter: Option<SharedNativePresenter<NativeSurfaceTarget>>,
    compositor: Option<NativeGpuCompositor>,
    cache: SharedNativeOutputCache,
    geometry_generation: u64,
    reserved_geometry: Option<(u32, u32, u64)>,
    history: Option<ResidentHistoryState>,
    terminal_fault: Option<String>,
    delivery: super::NativeDeliveryCounter,
    retired_ready: Vec<
        super::RetiringNativeOutput<
            super::ReadyNativeGpuOutput<Arc<crate::native_gpu_snapshot::NativeSnapshotGpu>>,
        >,
    >,
    retirements: Vec<SharedNativeOutputRetirement>,
    unsubmitted: Vec<crate::native_gpu_budget::FrameReservation>,
    recovered_history: Option<crate::native_gpu_output::history::NativeOutputHistoryFrame>,
    request_sequence: u64,
    history_configuration: Option<prepare::ResidentHistoryConfiguration>,
    input_bindings: Vec<prepare::ResidentInputBinding>,
    next_input_id: u64,
    history_host: Option<HistoryHostBudget>,
    follow_last_valid: Option<prepare::ResidentFollowLastValid>,
    ring: ring::ResidentSnapshotRing,
}

/// Initialization can fail after a broker has retained real GPU work. The app
/// must install this complete session in its ordinary retirement owner before
/// reporting the error; string conversion cannot discard that ownership.
pub struct NativeResidentDisplayCreationFailure {
    message: String,
    retained: Option<Box<NativeResidentDisplaySession>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeResidentPrepareError {
    Cancelled { retirement_required: bool },
    Revoked { retirement_required: bool },
    Fault(String),
}

impl std::fmt::Display for NativeResidentPrepareError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled {
                retirement_required,
            } => write!(
                f,
                "native preparation cancelled; retirement required={retirement_required}"
            ),
            Self::Revoked {
                retirement_required,
            } => write!(
                f,
                "native preparation authority revoked; retirement required={retirement_required}"
            ),
            Self::Fault(message) => f.write_str(message),
        }
    }
}

impl NativeResidentDisplayCreationFailure {
    pub fn into_parts(self) -> (String, Option<Box<NativeResidentDisplaySession>>) {
        (self.message, self.retained)
    }
}

impl std::fmt::Debug for NativeResidentDisplayCreationFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeResidentDisplayCreationFailure")
            .field("message", &self.message)
            .field("retained_session", &self.retained.is_some())
            .finish()
    }
}

impl NativeResidentDisplaySession {
    pub fn new<W>(
        window: Arc<W>,
        output_id: u64,
    ) -> Result<Self, NativeResidentDisplayCreationFailure>
    where
        W: HasWindowHandle + HasDisplayHandle + Send + Sync + 'static,
    {
        let factory = NativeGpuFactory::new();
        let (surface, owner) = UnconfiguredNativeSurface::create_with_owner(
            &factory,
            window,
            output_id,
            NativeGpuRequirements::default(),
        )
        .map_err(|error| NativeResidentDisplayCreationFailure {
            message: format!("shared native GPU creation failed: {error:?}"),
            retained: None,
        })?;
        let cache = SharedNativeOutputCache::new(output_id);
        let mut session = Self {
            output_id,
            factory,
            owner: ResidentOwner(Some(owner)),
            presenter: None,
            compositor: None,
            cache,
            geometry_generation: 0,
            reserved_geometry: None,
            history: None,
            terminal_fault: None,
            delivery: super::NativeDeliveryCounter::default(),
            retired_ready: Vec::new(),
            retirements: Vec::new(),
            unsubmitted: Vec::new(),
            recovered_history: None,
            request_sequence: 0,
            history_configuration: None,
            input_bindings: Vec::new(),
            next_input_id: 1,
            history_host: None,
            follow_last_valid: None,
            ring: ring::ResidentSnapshotRing::default(),
        };
        session.presenter =
            match SharedNativePresenter::from_shared_unconfigured(surface, &mut session.owner) {
                Ok(presenter) => Some(presenter),
                Err(error) => {
                    return Err(session.retain_creation_failure(format!(
                        "shared native presenter creation failed: {error:?}"
                    )))
                }
            };
        session.compositor = match NativeGpuCompositor::from_shared(&mut session.owner) {
            Ok(compositor) => Some(compositor),
            Err(error) => {
                return Err(session.retain_creation_failure(format!(
                    "shared native compositor creation failed: {error:?}"
                )))
            }
        };
        Ok(session)
    }

    fn retain_creation_failure(mut self, message: String) -> NativeResidentDisplayCreationFailure {
        self.terminal_fault = Some(message.clone());
        NativeResidentDisplayCreationFailure {
            message,
            retained: Some(Box::new(self)),
        }
    }

    fn ensure_usable(&self) -> Result<(), String> {
        match self.terminal_fault.as_ref() {
            Some(fault) => Err(format!(
                "shared native Display session is terminal: {fault}"
            )),
            None => Ok(()),
        }
    }

    pub fn output_id(&self) -> u64 {
        self.output_id
    }

    pub fn configure(&mut self, width: u32, height: u32) -> Result<(), String> {
        let generation = self.reserve_geometry(width, height)?;
        self.configure_reserved(width, height, generation)
    }

    /// Reserve the exact geometry identity used by preparation without
    /// configuring/acquiring the hidden surface. The app confirms this same
    /// reservation only after the window reports its physical client extent.
    pub fn reserve_geometry(&mut self, width: u32, height: u32) -> Result<u64, String> {
        self.ensure_usable()?;
        if width == 0 || height == 0 {
            return Err("shared native Display cannot reserve a zero extent".into());
        }
        if let Some((reserved_width, reserved_height, generation)) = self.reserved_geometry {
            if (reserved_width, reserved_height) == (width, height) {
                return Ok(generation);
            }
            return Err(format!(
                "shared native Display already reserved geometry {reserved_width}x{reserved_height} generation {generation}"
            ));
        }
        self.geometry_generation = self
            .geometry_generation
            .checked_add(1)
            .ok_or_else(|| "shared native Display geometry generation exhausted".to_string())?;
        self.owner.release_resource_pools();
        if let Some(compositor) = &self.compositor {
            compositor.release_pooled_outputs();
        }
        self.reserved_geometry = Some((width, height, self.geometry_generation));
        Ok(self.geometry_generation)
    }

    pub fn configure_reserved(
        &mut self,
        width: u32,
        height: u32,
        generation: u64,
    ) -> Result<(), String> {
        self.ensure_usable()?;
        if self.reserved_geometry != Some((width, height, generation)) {
            return Err(
                "shared native Display geometry confirmation does not match its reservation"
                    .to_string(),
            );
        }
        self.presenter
            .as_mut()
            .ok_or_else(|| "shared native presenter is between generations".to_string())?
            .configure_admitted(
                AdmittedPhysicalGeometry {
                    extent: (width, height),
                    geometry_generation: self.geometry_generation,
                },
                &mut self.owner,
            )
            .map_err(|error| format!("shared native Display configure failed: {error:?}"))?;
        self.reserved_geometry = None;
        Ok(())
    }

    pub fn geometry_generation(&self) -> u64 {
        self.geometry_generation
    }

    pub fn install_renderer<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
    ) -> Result<(), String> {
        self.ensure_usable()?;
        renderer.install_native_gpu_runtime(&mut self.owner)
    }

    pub fn bind_canonical_cpu_history<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        project_epoch: u64,
        request_sequence: u64,
        extent: (u32, u32),
        inputs: Vec<NativeResidentHistoryInput>,
    ) -> Result<(), String> {
        self.ensure_usable()?;
        if self.history.is_some() {
            return Err("shared native history is already bound for this session".into());
        }
        if extent.0 == 0 || extent.1 == 0 || self.geometry_generation == 0 || inputs.is_empty() {
            return Err(
                "shared native history requires admitted geometry and at least one input".into(),
            );
        }
        let frame = NativeFrameId {
            output_id: self.output_id,
            project_epoch,
            request_sequence,
            geometry_generation: self.geometry_generation,
        };
        let scopes = inputs
            .iter()
            .map(|input| HistoryScope {
                frame_scope: frame,
                input: input.input,
                layer_id: input.layer_id,
                queue_epoch: input.queue_epoch,
                source_generation: input.source_generation,
                extent,
            })
            .collect::<Vec<_>>();
        let host = match self.history_host.as_ref() {
            Some(host) => host.clone(),
            None => {
                let limit = HistoryHostBudget::planning_limit(inputs.len().max(3), extent)
                    .ok_or_else(|| "native history host capacity overflow".to_string())?;
                HistoryHostBudget::new(limit)
            }
        };
        let mut session = NativeOutputHistorySession::with_host_budget(
            self.owner.context().id(),
            scopes.clone(),
            extent,
            host,
        )
        .map_err(|error| format!("shared native history session rejected: {error:?}"))?;
        let mut sequences = Vec::with_capacity(inputs.len());
        let mut bindings = Vec::with_capacity(inputs.len());
        let mut queue_indices = Vec::with_capacity(inputs.len());
        for (index, scope) in scopes.iter().copied().enumerate() {
            let queue_index = renderer
                .runtime
                .queues
                .iter()
                .position(|queue| queue.key() == scope.input)
                .ok_or_else(|| format!("canonical CPU queue {:?} is absent", scope.input))?;
            let queue = &renderer.runtime.queues[queue_index];
            let frames = queue
                .iter_shared()
                .enumerate()
                .map(|(slot, frame)| {
                    let owner = if let Some(owner) = queue.native_owners[slot].as_ref() {
                        Some(
                            crate::native_gpu_history::adapter::CanonicalCpuHistoryOwner::History(
                                Arc::clone(owner.backing()),
                            ),
                        )
                    } else {
                        queue.native_cpu_owners[slot].as_ref().map(|owner| {
                            crate::native_gpu_history::adapter::CanonicalCpuHistoryOwner::Native(
                                Arc::clone(owner),
                            )
                        })
                    };
                    (Arc::clone(frame), owner)
                })
                .collect::<Vec<_>>();
            let mut sequence = CpuHistoryEntrySequence::new(scope, None);
            let binding = match session.import_canonical_queue(index, &mut sequence, frames) {
                Ok(binding) => binding,
                Err(failure) => {
                    // The canonical queue still owns every slot. Verify the
                    // rejected transfer returned all exact aliases before
                    // releasing those duplicate owners on this cold failure.
                    let complete = failure.frames.len() == queue.len()
                        && failure
                            .frames
                            .iter()
                            .zip(queue.iter_shared())
                            .all(|((frame, _), original)| Arc::ptr_eq(frame, original));
                    let error = format!("canonical CPU history import rejected: {:?}; complete owners returned={complete}", failure.error);
                    drop(failure.frames);
                    return Err(error);
                }
            };
            sequences.push(sequence);
            bindings.push(binding);
            queue_indices.push(queue_index);
        }
        if queue_indices.iter().zip(&bindings).any(|(index, binding)| {
            !renderer.runtime.queues[*index].accepts_native_entry_ids(binding)
        }) {
            return Err("canonical CPU queue rejected its native entry binding".to_string());
        }
        for (index, binding) in queue_indices.into_iter().zip(&bindings) {
            renderer.runtime.queues[index].install_native_entry_ids(binding);
        }
        self.history_host = Some(session.host_budget().clone());
        self.history = Some(ResidentHistoryState::Active(ResidentHistoryActive {
            session,
            sequences,
            inputs: inputs.into_iter().map(|input| input.input).collect(),
        }));
        Ok(())
    }

    pub fn handover_all_history_to_cpu<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<(), String> {
        let runtime = renderer
            .isf_runtime
            .as_mut()
            .and_then(|runtime| runtime.as_mut().ok())
            .ok_or_else(|| "renderer does not own the shared native ISF runtime".to_string())?;
        self.materialize_history_for_cpu(runtime, cancellation)?;
        self.publish_history_to_cpu(renderer)
    }

    fn materialize_history_for_cpu(
        &mut self,
        runtime: &mut crate::isf_runtime::IsfGpuRuntime,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<(), String> {
        self.ensure_usable()?;
        // A CPU handover consumes the history session that a resolved ring
        // predecessor was sealed against: it can no longer be published.
        self.discard_ring_resolved();
        self.ring.disarm();
        if self.ring.owns_in_flight() {
            return Err("CPU history handover requires the snapshot ring to drain first".into());
        }
        let state = self
            .history
            .take()
            .ok_or_else(|| "shared native history is not bound".to_string())?;
        let mut handover = match state {
            ResidentHistoryState::Active(active) => ResidentHistoryHandover {
                handovers: active
                    .session
                    .into_histories()
                    .into_iter()
                    .map(|history| Some(history.begin_cpu_handover()))
                    .collect(),
                completed: std::iter::repeat_with(|| None)
                    .take(active.inputs.len())
                    .collect(),
                snapshots: std::iter::repeat_with(|| None)
                    .take(active.inputs.len())
                    .collect(),
                inputs: active.inputs,
                sequences: active.sequences,
                retained_materializations: Vec::new(),
            },
            ResidentHistoryState::Handover(handover) => handover,
        };
        for index in 0..handover.handovers.len() {
            let Some(current) = handover.handovers[index].as_mut() else {
                continue;
            };
            while current.pending() != 0 {
                let Some(next) = current.next_entry() else {
                    self.history = Some(ResidentHistoryState::Handover(handover));
                    return Err("CPU handover pending count had no next entry".into());
                };
                let (entry, request) = match next {
                    CpuHistoryMaterializationRequest::ReadValidatedGpu { entry, resident } => (
                        entry,
                        CpuMaterializationRequest::ValidatedGpu {
                            entry,
                            lease: resident,
                        },
                    ),
                    CpuHistoryMaterializationRequest::ReplayToCpu { entry, frozen } => (
                        entry,
                        CpuMaterializationRequest::FrozenReplay {
                            entry,
                            recipe: frozen,
                        },
                    ),
                };
                let materialized = match NativeGpuMaterializer::new(runtime).materialize_cpu(
                    request,
                    &mut self.owner,
                    cancellation,
                ) {
                    Ok(materialized) => materialized,
                    Err(error) => {
                        self.history = Some(ResidentHistoryState::Handover(handover));
                        return Err(format!("CPU history materialization failed: {error:?}"));
                    }
                };
                if let Err(failure) =
                    accept_cpu_history_materialization(current, entry, materialized)
                {
                    handover
                        .retained_materializations
                        .push(failure.materialized);
                    self.history = Some(ResidentHistoryState::Handover(handover));
                    return Err(format!(
                        "CPU history handover rejected: {:?}",
                        failure.error
                    ));
                }
                for retired in current.take_retired() {
                    let (entry, owner) = retired.into_parts();
                    if owner.identity().entry != entry {
                        self.history = Some(ResidentHistoryState::Handover(handover));
                        return Err(
                            "retired historical resource has a foreign entry identity".into()
                        );
                    }
                    drop(owner);
                }
            }
        }
        if !handover.retained_materializations.is_empty() {
            self.history = Some(ResidentHistoryState::Handover(handover));
            return Err("CPU history handover retains rejected materializations".into());
        }
        for index in 0..handover.handovers.len() {
            if handover.completed[index].is_some() || handover.snapshots[index].is_some() {
                continue;
            }
            let current = handover.handovers[index]
                .take()
                .expect("unfinished CPU handover keeps its affine owner");
            let snapshot = match current.finish() {
                Ok(snapshot) => snapshot,
                Err(current) => {
                    handover.handovers[index] = Some(current);
                    self.history = Some(ResidentHistoryState::Handover(handover));
                    return Err("CPU history handover was not complete".into());
                }
            };
            handover.snapshots[index] = Some(snapshot);
        }
        for index in 0..handover.snapshots.len() {
            if handover.completed[index].is_some() {
                continue;
            }
            let snapshot = handover.snapshots[index]
                .take()
                .expect("finished handover keeps its charged snapshot until publication");
            let scope = snapshot.scope();
            let queue = match NativeCpuHistoryQueue::from_snapshot(scope, snapshot) {
                Ok(queue) => queue,
                Err(failure) => {
                    let error = failure.error;
                    handover.snapshots[index] = Some(failure.snapshot);
                    self.history = Some(ResidentHistoryState::Handover(handover));
                    return Err(format!("CPU history snapshot rejected: {error:?}"));
                }
            };
            handover.completed[index] = Some(queue);
        }
        self.history = Some(ResidentHistoryState::Handover(handover));
        Ok(())
    }

    fn publish_history_to_cpu<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
    ) -> Result<(), String> {
        let mut handover = match self.history.take() {
            Some(ResidentHistoryState::Handover(handover)) => handover,
            state => {
                self.history = state;
                return Err("CPU history publication requires its complete handover".into());
            }
        };
        if handover.completed.iter().any(Option::is_none) {
            self.history = Some(ResidentHistoryState::Handover(handover));
            return Err("CPU history publication refused an incomplete handover".into());
        }
        // Publish only after every per-input handover reached a complete charged
        // CPU queue. No canonical queue can observe a partial native->CPU barrier.
        let mut canonical_indices = Vec::with_capacity(handover.completed.len());
        for index in 0..handover.completed.len() {
            let key = handover.inputs[index];
            let canonical_index = match renderer
                .runtime
                .queues
                .iter()
                .position(|candidate| candidate.key() == key)
            {
                Some(index) => index,
                None => {
                    self.history = Some(ResidentHistoryState::Handover(handover));
                    return Err(format!("canonical CPU queue {:?} disappeared", key));
                }
            };
            let queue = handover.completed[index]
                .as_ref()
                .expect("all CPU queues completed before canonical publication");
            if !renderer.runtime.queues[canonical_index].accepts_native_history_replacement(queue) {
                self.history = Some(ResidentHistoryState::Handover(handover));
                return Err("canonical CPU queue rejected complete charged history".to_string());
            }
            canonical_indices.push(canonical_index);
        }
        if let Some((key, frame, owner)) = self.cache.last_valid_with_owner() {
            if let Err(error) = renderer.accepts_native_output_last_valid(key, frame, owner) {
                self.history = Some(ResidentHistoryState::Handover(handover));
                return Err(error);
            }
        }
        for index in 0..handover.completed.len() {
            let queue = handover.completed[index]
                .take()
                .expect("all CPU queues completed before canonical publication");
            renderer.runtime.queues[canonical_indices[index]]
                .install_native_history_replacement(queue);
        }
        if let Some((key, frame, owner)) = self.cache.take_last_valid_with_owner() {
            renderer
                .install_native_output_last_valid(key, frame, owner)
                .expect(
                    "LastValid and every canonical queue were validated before atomic publication",
                );
        }
        self.history = None;
        Ok(())
    }

    fn commit_issued_identities(
        sequences: &mut [CpuHistoryEntrySequence],
        inputs: &[protocol::VideoRenderInputKey],
        identities: &[crate::native_gpu_contract::LayerResultIdentity],
    ) -> Result<(), String> {
        let mut indices = Vec::with_capacity(identities.len());
        for identity in identities {
            let index = inputs
                .iter()
                .position(|input| *input == identity.entry.input)
                .ok_or_else(|| "prospective native identity has no session input".to_string())?;
            if indices.contains(&index)
                || sequences[index]
                    .prospective_next()
                    .map_err(|error| format!("native history sequence failed: {error:?}"))?
                    != identity.entry
            {
                return Err("prospective native identity does not match its next sequence".into());
            }
            indices.push(index);
        }
        for (identity, index) in identities.iter().zip(indices) {
            sequences[index]
                .commit_issued(identity.entry)
                .map_err(|error| format!("native history sequence commit failed: {error:?}"))?;
        }
        Ok(())
    }

    fn run_cpu_continuation(
        &mut self,
        mut continuation: crate::isf_native_gpu::continuation::NativeCpuContinuation<'_>,
        reason: crate::native_gpu_contract::NativeGpuDecline,
        entries: &[crate::native_gpu_contract::HistoryEntryId],
        request: &NativeOutputRequest,
        authority: &impl NativeOutputAuthority,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<NativeResidentExecution, String> {
        // Idle resident storage must not consume the CPU handover envelope.
        // Prepared checkout pins and active history/transactions remain charged.
        self.owner.release_resource_pools();
        crate::native_gpu_output::guard(
            request,
            crate::native_gpu_output::OutputBoundary::BeforeCpuFallback,
            authority,
            cancellation,
        )
        .map_err(|reason| format!("CPU continuation revoked before execution: {reason:?}"))?;
        // From here the tick admits brokered work. A later revocation lets it
        // complete on the healthy owner and is classified as zero presents.
        cancellation.enter_committed_work();
        // A late eligibility/budget decline retains the exact G4 preparation.
        // Complete frozen historical materialization before current CPU FX;
        // canonical publication follows when the renderer borrow is returned.
        if self.history.is_some() {
            continuation.with_runtime_before_execution(|runtime| {
                self.materialize_history_for_cpu(runtime, cancellation)
            })?;
        }
        let continuation = continuation
            .refresh()
            .map_err(|fault| format!("CPU continuation refresh failed: {fault:?}"))?;
        let plan = continuation
            .allocation_plan()
            .map_err(|decline| {
                format!("brokered CPU continuation declined before quote: {decline:?}")
            })?
            .clone();
        // Validate every sequence before allocating a reservation. The checked
        // commits below cannot fail while this affine continuation owns the tick.
        if let Some(ResidentHistoryState::Handover(history)) = self.history.as_mut() {
            let mut indices = Vec::with_capacity(entries.len());
            for entry in entries {
                let index = history
                    .inputs
                    .iter()
                    .position(|key| *key == entry.input)
                    .ok_or_else(|| "CPU continuation has no bound history input".to_string())?;
                if indices.contains(&index)
                    || history.sequences[index]
                        .prospective_next()
                        .map_err(|error| format!("CPU issuance rejected: {error:?}"))?
                        != *entry
                {
                    return Err(
                        "CPU continuation entry does not match its reserved next identity".into(),
                    );
                }
                indices.push(index);
            }
            for (entry, index) in entries.iter().zip(indices) {
                history.sequences[index]
                    .commit_issued(*entry)
                    .map_err(|error| format!("CPU sequence commit failed: {error:?}"))?;
            }
        }
        // Completed retired owners stay charged until the broker observes
        // them. Reap them before quoting so admission sees the real live set.
        self.owner.poll_retirement().map_err(|fault| {
            format!("native retirement poll failed before CPU quote: {fault:?}")
        })?;
        let quote = self
            .owner
            .quote(&plan)
            .map_err(|decline| format!("brokered CPU continuation quote declined: {decline:?}"))?;
        let reservation = self
            .owner
            .reserve(quote)
            .map_err(|error| format!("brokered CPU reservation failed: {error:?}"))?;
        match continuation.run(&mut self.owner, reservation, cancellation) {
            Ok(results) => {
                crate::native_gpu_output::guard(
                    request,
                    crate::native_gpu_output::OutputBoundary::AfterPoll,
                    authority,
                    cancellation,
                )
                .map_err(|reason| {
                    format!("CPU continuation revoked before publication: {reason:?}")
                })?;
                Ok(NativeResidentExecution::Cpu { reason, results })
            }
            Err(NativeCpuContinuationFailure::Admission {
                fault,
                reservation,
                continuation,
            }) => {
                drop(continuation);
                self.refund_unsubmitted(reservation)?;
                Err(format!("brokered CPU admission failed: {fault:?}"))
            }
            Err(NativeCpuContinuationFailure::Declined {
                reason,
                continuation,
            }) => {
                drop(continuation);
                Err(format!("brokered CPU execution declined: {reason:?}"))
            }
            Err(NativeCpuContinuationFailure::Cancelled { retirement }) => Err(format!(
                "brokered CPU execution cancelled; retirement={retirement:?}"
            )),
            Err(NativeCpuContinuationFailure::Fault { fault, retirement }) => Err(format!(
                "brokered CPU execution faulted: {fault:?}; retirement={retirement:?}"
            )),
        }
    }

    fn refund_unsubmitted(
        &mut self,
        reservation: crate::native_gpu_budget::FrameReservation,
    ) -> Result<(), String> {
        match self.owner.discard_unsubmitted_reservation(reservation) {
            Ok(()) => Ok(()),
            Err((fault, reservation)) => {
                self.unsubmitted.push(reservation);
                Err(format!(
                    "unused native reservation retained for cleanup: {fault:?}"
                ))
            }
        }
    }

    /// I7's affine native tick. The caller supplies already-decoded layer work;
    /// this method owns every step from G4 continuation preflight through P
    /// history selection/composition reservation and I's C3 completion. A P
    /// historical selection that is not the exact incoming layer declines to
    /// the same prepared CPU continuation before any GPU execution. This is the
    /// bounded bridge until CPU-post-FX upload is implemented.
    pub(crate) fn execute_prepared_layers<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        mut request: NativeOutputRequest,
        layers: StackBatch<NativeLayerPreparation>,
        composition_layers: &[CompositionLayerPlan],
        clear_rgba8: [u8; 4],
        authority: &impl NativeOutputAuthority,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<NativeResidentExecution, String> {
        self.ensure_usable()?;
        let shape = std::array::from_fn(|index| {
            layers.get(index).map(|layer| {
                (
                    layer.input.width,
                    layer.input.height,
                    layer.input.format,
                    layer
                        .chain
                        .stages
                        .iter()
                        .filter(|stage| stage.enabled)
                        .count(),
                )
            })
        });
        self.owner.configure_stack_pool_shape(shape);
        let history = match self.history.as_mut() {
            Some(ResidentHistoryState::Active(active)) => active,
            Some(ResidentHistoryState::Handover(_)) => {
                return Err("native tick refused while CPU history handover is active".into())
            }
            None => return Err("native tick requires a bound output history session".into()),
        };
        let issued = layers.iter().map(|layer| layer.entry).collect::<Vec<_>>();
        let continuation = renderer
            .prepare_native_layer_continuation(
                request.frame,
                layers,
                NativeBatchRestrictions::default(),
                Some(cancellation),
            )
            .map_err(|failure| format!("native layer preparation failed: {failure}"))?;
        // G4 represents resourceful/empty/otherwise ineligible native work as
        // an allocation decline. Preserve the affine preparation by converting
        // that exact continuation to CPU before asking for GPU-only identities.
        let mut allocation = match continuation.allocation_plan() {
            Ok(plan) => plan.clone(),
            Err(reason) => {
                let cpu = continuation.into_cpu().map_err(|fault| {
                    format!(
                        "native eligibility decline could not enter CPU continuation: {fault:?}"
                    )
                })?;
                return self.run_cpu_continuation(
                    cpu,
                    reason,
                    &issued,
                    &request,
                    authority,
                    cancellation,
                );
            }
        };
        let identities = continuation
            .prospective_identities()
            .map_err(|error| format!("native prospective identity failed: {error:?}"))?;
        if identities.len() != composition_layers.len() {
            return Err("native layer/composition cardinality mismatch".into());
        }
        let Some(target_ms) = composition_layers.first().map(|layer| layer.position_ms) else {
            let reason = crate::native_gpu_contract::NativeGpuDecline::UnsupportedComposition;
            let cpu = continuation.into_cpu().map_err(|fault| {
                format!("empty composition decline could not enter CPU continuation: {fault:?}")
            })?;
            return self.run_cpu_continuation(
                cpu,
                reason,
                &issued,
                &request,
                authority,
                cancellation,
            );
        };
        let targets = composition_layers
            .iter()
            .map(|layer| layer.position_ms)
            .collect::<Vec<_>>();
        let history_plan = if targets.iter().all(|target| *target == target_ms) {
            history.session.plan(&identities, target_ms)
        } else {
            history.session.plan_with_targets(&identities, &targets)
        }
        .map_err(|error| format!("native history plan failed: {error:?}"))?;
        let selections = history
            .session
            .selected_layers(&history_plan)
            .map_err(|error| format!("native history selection failed: {error:?}"))?;
        if selections
            .iter()
            .any(|selected| matches!(selected, NativeHistorySelected::NeedsMaterialization))
        {
            let reason = crate::native_gpu_contract::NativeGpuDecline::UnsupportedComposition;
            let cpu = continuation.into_cpu().map_err(|fault| {
                format!("native history decline could not enter CPU continuation: {fault:?}")
            })?;
            return self.run_cpu_continuation(
                cpu,
                reason,
                &issued,
                &request,
                authority,
                cancellation,
            );
        }
        let selected = composition_layers
            .iter()
            .zip(selections.iter())
            .map(|(plan, selected)| {
                let (identity, kind) = match selected {
                    NativeHistorySelected::Incoming(identity) => {
                        (*identity, SelectedTextureKind::InThisTransaction)
                    }
                    NativeHistorySelected::Resident(lease) => {
                        (*lease.identity(), SelectedTextureKind::Validated)
                    }
                    NativeHistorySelected::NeedsMaterialization => {
                        unreachable!("pre-execution decline above")
                    }
                };
                NativeCompositionLayer {
                    plan,
                    selected_entry: identity.entry,
                    identity,
                    kind,
                }
            })
            .collect::<Vec<_>>();
        let selected_history = selections
            .iter()
            .filter_map(|selected| match selected {
                NativeHistorySelected::Resident(lease) => Some(*lease),
                _ => None,
            })
            .collect::<Vec<_>>();
        for lease in &selected_history {
            allocation.add_existing(lease.allocation_id());
        }
        request.pts_ms = selected
            .iter()
            .map(|layer| layer.identity.pts_ms)
            .max()
            .unwrap_or(0);
        request.duration_ms = 0;
        let limits = &self.owner.capabilities().limits;
        let mut composition = match NativeCompositionPlan::preflight(
            request.context,
            request.frame,
            (request.key.width, request.key.height),
            clear_rgba8,
            &selected,
            limits.max_texture_dimension_2d,
            u64::from(limits.max_storage_buffer_binding_size),
        ) {
            Ok(composition) => composition,
            Err(CompositionPlanFailure::Fault(fault)) => {
                return Err(format!("native composition identity fault: {fault:?}"));
            }
            Err(CompositionPlanFailure::Declined(reason)) => {
                let cpu = continuation.into_cpu().map_err(|fault| {
                    format!("composition decline could not enter CPU continuation: {fault:?}")
                })?;
                return self.run_cpu_continuation(
                    cpu,
                    reason,
                    &issued,
                    &request,
                    authority,
                    cancellation,
                );
            }
        };
        // Snapshot ring: reuse a completed readback of this geometry instead of
        // charging a new one; a stale buffer is dropped here.
        if let Some(readback) = self
            .ring
            .take_free(request.context, composition.allocations.readback_bytes)
        {
            if let Err(stale) = composition.reuse_readback(readback) {
                drop(stale);
            }
        }
        if let Some(compositor) = &self.compositor {
            if let Err(reason) = compositor
                .prepare_pooled_output(&mut composition, self.owner.context().resource_pool_epoch())
            {
                let cpu = continuation.into_cpu().map_err(|fault| {
                    format!("premapping pool decline could not enter CPU continuation: {fault:?}")
                })?;
                return self.run_cpu_continuation(
                    cpu,
                    reason,
                    &issued,
                    &request,
                    authority,
                    cancellation,
                );
            }
        }
        allocation
            .merge(composition.allocation_plan())
            .map_err(|reason| format!("native allocation plans cannot merge: {reason:?}"))?;
        // Reap completed retirements (previous ticks' evicted residents and
        // superseded composition/snapshot owners) before quoting this tick.
        self.owner
            .poll_retirement()
            .map_err(|fault| format!("native retirement poll failed before quote: {fault:?}"))?;
        let quote = match self.owner.quote(&allocation) {
            Ok(quote) => quote,
            Err(reason) => {
                let cpu = continuation.into_cpu().map_err(|fault| {
                    format!("native quote decline could not enter CPU continuation: {fault:?}")
                })?;
                return self.run_cpu_continuation(
                    cpu,
                    reason,
                    &issued,
                    &request,
                    authority,
                    cancellation,
                );
            }
        };
        let admitted = match continuation.admit(&quote) {
            NativeContinuationAdmission::Accepted(admitted) => admitted,
            NativeContinuationAdmission::Declined {
                reason,
                continuation,
            } => {
                let cpu = continuation.into_cpu().map_err(|fault| {
                    format!("native admission decline could not enter CPU continuation: {fault:?}")
                })?;
                return self.run_cpu_continuation(
                    cpu,
                    reason,
                    &issued,
                    &request,
                    authority,
                    cancellation,
                );
            }
        };
        let reservation = match self.owner.reserve(quote) {
            Ok(reservation) => reservation,
            Err(error) => {
                let cpu = admitted.return_unexecuted().into_cpu().map_err(|fault| {
                    format!(
                        "native reserve failure could not return to CPU continuation: {fault:?}"
                    )
                })?;
                let reason = crate::native_gpu_contract::NativeGpuDecline::QuoteExpired;
                return self
                    .run_cpu_continuation(cpu, reason, &issued, &request, authority, cancellation)
                    .map_err(|cpu| {
                        format!(
                            "native reservation failed ({error:?}); CPU continuation failed: {cpu}"
                        )
                    });
            }
        };
        if let Err(reason) = crate::native_gpu_output::guard(
            &request,
            crate::native_gpu_output::OutputBoundary::BeforeSubmit,
            authority,
            cancellation,
        ) {
            drop(admitted);
            self.refund_unsubmitted(reservation)?;
            return Err(format!(
                "native submission revoked before execution: {reason:?}"
            ));
        }
        // Submission follows. Revocation after this point must not retire the
        // shared owner: the submitted output completes and is discarded.
        cancellation.enter_committed_work();
        if let Err(error) =
            Self::commit_issued_identities(&mut history.sequences, &history.inputs, &identities)
        {
            drop(admitted);
            self.refund_unsubmitted(reservation)?;
            return Err(error);
        }
        let Some(compositor) = self.compositor.as_ref() else {
            drop(admitted);
            self.refund_unsubmitted(reservation)?;
            return Err("shared native compositor is unavailable".into());
        };
        let submitted_request = request.clone();
        #[cfg(feature = "qa-native-frame-timing")]
        let submit_trace =
            crate::native_frame_timing::span(crate::native_frame_timing::Phase::IsfSubmit);
        let pending = admitted.with_runtime(|runtime, batch| {
            PendingSharedNativeOutput::submit_with_history(
                &mut self.owner,
                runtime,
                compositor,
                request,
                batch,
                reservation,
                &composition,
                &selected_history,
                &history.session,
                history_plan,
            )
        });
        #[cfg(feature = "qa-native-frame-timing")]
        drop(submit_trace);
        let pending = match pending {
            Ok(pending) => pending,
            Err((retiring, _plan)) => return Ok(NativeResidentExecution::Retiring(retiring)),
        };
        if let Err(reason) = crate::native_gpu_output::guard(
            &submitted_request,
            crate::native_gpu_output::OutputBoundary::AfterSubmit,
            authority,
            cancellation,
        ) {
            return Ok(NativeResidentExecution::Retiring(
                pending.retire(&mut self.owner, reason),
            ));
        }
        // Nonblocking: transaction/history only. The snapshot copy and cache
        // publication follow either synchronously or through the snapshot ring.
        let progress = pending.poll_phases_with_history(
            &mut self.owner,
            renderer
                .isf_runtime
                .as_mut()
                .and_then(|runtime| runtime.as_mut().ok())
                .ok_or_else(|| "shared native ISF runtime disappeared during poll".to_string())?,
            &mut self.cache,
            &mut history.session,
            authority,
            cancellation,
        );
        match progress {
            SharedNativeOutputProgress::Retiring(retiring) => {
                Ok(NativeResidentExecution::Retiring(retiring))
            }
            progress => Ok(NativeResidentExecution::Gpu(progress)),
        }
    }

    pub fn poll_present_retirements(&mut self) -> Result<usize, String> {
        self.ensure_usable()?;
        let pending = self
            .presenter
            .as_mut()
            .ok_or_else(|| "shared native presenter is between generations".to_string())?
            .poll_retirements(&mut self.owner)
            .map_err(|error| format!("shared native present retirement failed: {error:?}"))?;
        for retired in &mut self.retired_ready {
            retired
                .poll_with(|_| Ok(pending == 0))
                .map_err(|fault| format!("retained Ready cleanup failed: {fault:?}"))?;
        }
        self.retired_ready.retain(|retired| !retired.is_retired());
        Ok(pending)
    }

    /// Loss/re-arm keeps the original owner and its ledger.  The replacement is
    /// created from the same factory/surface and is adopted before any runtime,
    /// compositor or presenter is rebuilt for the new generation.
    pub fn rearm<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
    ) -> Result<(), String> {
        self.ensure_usable()?;
        if self.history.is_some() {
            return Err(
                "shared native history must complete its CPU handover before re-arm".into(),
            );
        }
        if !self.reset_ring_for_rearm() {
            return Err("shared native snapshot ring must retire before re-arm".into());
        }
        self.terminal_fault = Some("shared native Display re-arm is rebuilding".to_string());
        self.reserved_geometry = None;
        renderer.poison_native_gpu_runtime(
            "shared native Display re-arm is rebuilding; standalone ISF fallback is forbidden",
        );
        let presenter = self.presenter.take().ok_or_else(|| {
            let fault = "shared native presenter re-arm is already in progress".to_string();
            self.terminal_fault = Some(fault.clone());
            fault
        })?;
        let surface = presenter.into_unconfigured();
        let replacement = match self.factory.create_for_surface(
            surface.adapter_target(),
            self.output_id,
            NativeGpuRequirements::default(),
        ) {
            Ok(owner) => owner,
            Err(error) => {
                self.terminal_fault =
                    Some("restoring old presenter after replacement creation failure".into());
                let restored =
                    SharedNativePresenter::from_shared_unconfigured(surface, &mut self.owner)
                        .map_err(|restore| {
                            let fault = format!(
                                "replacement creation failed ({error:?}) and old presenter restore failed ({restore:?})"
                            );
                            self.terminal_fault = Some(fault.clone());
                            renderer.poison_native_gpu_runtime(fault.clone());
                            fault
                        })?;
                self.presenter = Some(restored);
                if let Err(runtime_error) = renderer.replace_native_gpu_runtime(&mut self.owner) {
                    self.terminal_fault = Some(runtime_error.clone());
                    return Err(format!(
                        "replacement creation failed ({error:?}); old renderer restore failed: {runtime_error}"
                    ));
                }
                self.terminal_fault = None;
                return Err(format!(
                    "shared native replacement creation failed: {error:?}"
                ));
            }
        };
        if let Err((fault, _replacement)) = self.owner.rearm_with_replacement(replacement) {
            self.terminal_fault = Some("restoring old presenter after rejected re-arm".into());
            let restored =
                SharedNativePresenter::from_shared_unconfigured(surface, &mut self.owner)
                    .map_err(|restore| {
                        let error = format!(
                            "shared native re-arm rejected ({fault:?}) and presenter restore failed ({restore:?})"
                        );
                        self.terminal_fault = Some(error.clone());
                        renderer.poison_native_gpu_runtime(error.clone());
                        error
                    })?;
            self.presenter = Some(restored);
            if let Err(runtime_error) = renderer.replace_native_gpu_runtime(&mut self.owner) {
                self.terminal_fault = Some(runtime_error.clone());
                return Err(format!(
                    "shared native re-arm rejected ({fault:?}); old renderer restore failed: {runtime_error}"
                ));
            }
            self.terminal_fault = None;
            return Err(format!("shared native re-arm rejected: {fault:?}"));
        }
        // The owner is now the new generation. Rollback to the old generation
        // is impossible, so publish an explicit terminal state before any
        // fallible dependent reconstruction. Every public operation rejects
        // this state until the complete new-generation aggregate is rebuilt.
        self.terminal_fault = Some("replacement generation reconstruction incomplete".to_string());
        self.compositor = None;
        renderer.poison_native_gpu_runtime(
            "shared native Display generation is rebuilding; standalone ISF fallback is forbidden",
        );
        let presenter =
            match SharedNativePresenter::from_shared_unconfigured(surface, &mut self.owner) {
                Ok(presenter) => presenter,
                Err(error) => {
                    self.presenter = None;
                    let fault = format!("replacement presenter creation failed: {error:?}");
                    self.terminal_fault = Some(fault.clone());
                    return Err(fault);
                }
            };
        self.presenter = Some(presenter);
        let compositor = match NativeGpuCompositor::from_shared(&mut self.owner) {
            Ok(compositor) => compositor,
            Err(error) => {
                self.compositor = None;
                let fault = format!("replacement compositor creation failed: {error:?}");
                self.terminal_fault = Some(fault.clone());
                return Err(fault);
            }
        };
        self.compositor = Some(compositor);
        if let Err(error) = renderer.replace_native_gpu_runtime(&mut self.owner) {
            self.compositor = None;
            self.presenter = None;
            self.terminal_fault = Some(error.clone());
            return Err(error);
        }
        self.terminal_fault = None;
        Ok(())
    }

    /// Cleanup-only stop entry. Unlike ordinary rendering operations this stays
    /// available after a failed re-arm so the authoritative owner/ledger can be
    /// driven to Reaped instead of being abandoned behind the terminal latch.
    pub fn begin_stop(&mut self) -> Result<(), String> {
        // The in-flight ring output joins `retirements` with its complete
        // transaction owner before the history it was planned against drops.
        self.retire_ring_for_stop();
        self.recovered_history = None;
        self.history = None;
        self.history_host = None;
        self.history_configuration = None;
        self.input_bindings.clear();
        self.follow_last_valid = None;
        let _ = self.cache.blackout();
        self.compositor = None;
        if let Some(presenter) = self.presenter.as_mut() {
            let stop = release_shared_presenter_for_stop(presenter, &mut self.owner)
                .map_err(|fault| format!("shared native presenter stop failed: {fault:?}"))?;
            if stop.context() != self.owner.context().id() {
                return Err("shared native Stop token belongs to another generation".into());
            }
        } else {
            let stop = self.owner.begin_stop();
            if stop.context() != self.owner.context().id() {
                return Err("shared native Stop token belongs to another generation".into());
            }
        }
        Ok(())
    }

    pub fn begin_stop_with_renderer<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
    ) -> Result<(), String> {
        // Rejected admission tokens still own ledger reservations. Transfer
        // them before releasing the renderer; Drop cannot refund them, and
        // outstanding logical work may prevent refund until a later poll.
        self.unsubmitted
            .append(&mut renderer.native_cpu_retained_reservations);
        // Drop every CPU alias while its complete accounting owner is alive.
        // Poisoning also prevents the old lazy standalone-runtime constructor
        // from creating a second device during cleanup or a rejected tick.
        if let Some(owner) = renderer.release_native_cpu_ownership_for_stop() {
            self.owner.restore(owner);
        }
        renderer.poison_native_gpu_runtime("shared native Display is stopping");
        self.begin_stop()
    }

    /// Nonblocking cleanup poll; callers retain this session until true.
    pub fn poll_stop(&mut self) -> Result<bool, String> {
        if let Some(presenter) = self.presenter.as_mut() {
            let pending = presenter
                .poll_retirements(&mut self.owner)
                .map_err(|fault| format!("shared present cleanup failed: {fault:?}"))?;
            for retired in &mut self.retired_ready {
                retired
                    .poll_with(|_| Ok(pending == 0))
                    .map_err(|fault| format!("retained Ready cleanup failed: {fault:?}"))?;
            }
            self.retired_ready.retain(|retired| !retired.is_retired());
        }
        for retirement in &mut self.retirements {
            // History outcomes belong to the retiring generation. Stop discards
            // them only as part of this retained owner, never as a success tick.
            drop(retirement.take_isf_outcome());
            drop(retirement.take_history_outcome());
            let completed = match self.presenter.as_mut() {
                Some(presenter) => retirement.poll_with_presenter(presenter, &mut self.owner),
                None => retirement.poll(&mut self.owner),
            };
            completed.map_err(|fault| format!("shared output cleanup failed: {fault:?}"))?;
        }
        let progress = self
            .owner
            .poll_retirement()
            .map_err(|fault| format!("shared native retirement poll failed: {fault:?}"))?;
        if matches!(
            progress,
            RetirementProgress::Pending {
                transactions: 0,
                ..
            }
        ) {
            while let Some(reservation) = self.unsubmitted.pop() {
                self.refund_unsubmitted(reservation)?;
            }
        }
        // P and each output-retirement aggregate also poll this same owner.
        // Any one of them can observe the Retiring -> Reaped transition first;
        // later polls need not return the transition receipt again.
        if matches!(progress, RetirementProgress::Reaped)
            || self.owner.state() == crate::native_gpu_contract::GpuOwnerState::Reaped
        {
            self.retirements.clear();
            self.retired_ready.clear();
            Ok(true)
        } else {
            Ok(false)
        }
    }
}
