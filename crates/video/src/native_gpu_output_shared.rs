//! I-owned join of the real G transaction and P composition/snapshot/present.
//! The resident Display facade joins canonical renderer, charged history handover
//! and G/P production consumers through this shared owner.
//! No window operations, new device, or whole-tick retry lives here.
use std::sync::Arc;

use super::history::{
    NativeOutputHistoryFrame, NativeOutputHistoryPlan, NativeOutputHistorySession,
};
use super::*;
use crate::gpu_surface::native::{
    AdmittedPhysicalGeometry, CpuArtisticContent, HardBlackoutContent, NativePresentTarget,
    NativeSurfaceTarget, SharedPresentSink, SharedSurfacePresenter,
};
use crate::gpu_surface::UnconfiguredNativeSurface;
use crate::isf_native_gpu::PreparedNativeBatch;
use crate::isf_runtime::{IsfGpuRuntime, StackBatch};
use crate::native_gpu_budget::FrameReservation;
use crate::native_gpu_composition::{
    EncodedPremappingSnapshot, NativeChargedBuffer, NativeCompositionPlan, NativeGpuCompositor,
    SelectedLayerInput, SelectedTextureKind,
};
use crate::native_gpu_context::{
    NativeGpuOwner, PendingGpuTransaction, RetirementProgress, RetiringGpuTransaction,
    StopRetirement,
};
use crate::native_gpu_history::HistoryError;
use crate::native_gpu_result::{
    EncodedIsfBatch, GpuLayerLease, NativeCandidateAvailability, NativeIsfCompletion,
    ValidatedLayerResult,
};
use crate::native_gpu_snapshot::{
    NativeGpuSnapshotter, NativeSnapshotGpu, NativeSnapshotMap, RecycledCpuSnapshot,
    RetiringPremappingSnapshot, SharedPremappingSnapshot, SnapshotProgress,
};

/// I's presenter retains the generation identity captured by its constructor.
/// P's retirement methods do not validate the broker; keep the presenter private
/// and reject a foreign owner before it can consume any P transaction token.
pub(crate) struct SharedNativePresenter<T: NativePresentTarget> {
    context: GpuContextId,
    presenter: SharedSurfacePresenter<T>,
}

impl<T: NativePresentTarget> SharedNativePresenter<T> {
    #[cfg(test)]
    pub(crate) fn new(target: T, owner: &mut NativeGpuOwner) -> Result<Self, NativeGpuFault> {
        let presenter = SharedSurfacePresenter::new(target, owner)?;
        Ok(Self {
            context: owner.context().id(),
            presenter,
        })
    }

    fn check_owner(&self, owner: &NativeGpuOwner) -> Result<(), NativeGpuFault> {
        if owner.context().id() == self.context {
            Ok(())
        } else {
            Err(NativeGpuFault {
                context: self.context,
                frame: None,
                phase: GpuPhase::MappingPresent,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "presenter and broker belong to different GPU generations".into(),
            })
        }
    }

    /// GPU configuration only. The app owns window resizing on its window thread.
    pub(crate) fn configure_admitted(
        &mut self,
        geometry: AdmittedPhysicalGeometry,
        owner: &mut NativeGpuOwner,
    ) -> Result<(), NativeGpuFault> {
        self.check_owner(owner)?;
        self.presenter.configure_admitted(geometry, owner)
    }

    pub(crate) fn poll_retirements(
        &mut self,
        owner: &mut NativeGpuOwner,
    ) -> Result<usize, NativeGpuFault> {
        self.check_owner(owner)?;
        self.presenter.poll_retirements(owner)
    }

    #[cfg(test)]
    pub(crate) fn drain_retirements(
        &mut self,
        owner: &mut NativeGpuOwner,
    ) -> Result<(), NativeGpuFault> {
        self.check_owner(owner)?;
        self.presenter.drain_retirements(owner)
    }

    pub(crate) fn sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> Result<SharedPresentSink<'p, T, R, P>, NativeGpuFault> {
        self.check_owner(owner)?;
        Ok(self.presenter.sink(owner))
    }

    pub(crate) fn presents(&self) -> u64 {
        self.presenter.presents()
    }

    pub(crate) fn configured_extent(&self) -> Option<(u32, u32)> {
        self.presenter.configured_extent()
    }

    pub(crate) fn cpu_artistic_sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> Result<SharedPresentSink<'p, T, R, P, CpuArtisticContent>, NativeGpuFault> {
        self.check_owner(owner)?;
        Ok(self.presenter.cpu_artistic_sink(owner))
    }

    pub(crate) fn hard_blackout_sink<'p, R, P>(
        &'p mut self,
        owner: &'p mut NativeGpuOwner,
    ) -> Result<SharedPresentSink<'p, T, R, P, HardBlackoutContent>, NativeGpuFault> {
        self.check_owner(owner)?;
        Ok(self.presenter.hard_blackout_sink(owner))
    }

    pub(crate) fn hard_blackout_presents(&self) -> u64 {
        self.presenter.hard_blackout_presents()
    }

    fn release_for_stop(&mut self, owner: &NativeGpuOwner) -> Result<(), NativeGpuFault> {
        self.check_owner(owner)?;
        self.presenter.release_for_stop();
        Ok(())
    }
}

impl SharedNativePresenter<NativeSurfaceTarget> {
    pub(crate) fn from_shared_unconfigured(
        surface: UnconfiguredNativeSurface,
        owner: &mut NativeGpuOwner,
    ) -> Result<Self, crate::native_gpu_context::ContextCreateError> {
        let context = owner.context().id();
        let presenter = SharedSurfacePresenter::from_shared_unconfigured(surface, owner)?;
        Ok(Self { context, presenter })
    }

    pub(crate) fn into_unconfigured(self) -> UnconfiguredNativeSurface {
        self.presenter.into_target().into_unconfigured()
    }
}

/// The CPU pixels and their accounting owner have the same outer lifetime.
/// Until P provides a CPU-charge split, this conservatively retains the
/// premapping allocation too. Both old and new owners remain in G's one ledger.
pub(crate) struct SharedNativeOutputCache {
    cache: NativeOutputCache,
    snapshot_owner: Option<Arc<NativeSnapshotGpu>>,
    /// The displaced LastValid's buffer and charge, for the next snapshot.
    recycled: Option<RecycledCpuSnapshot>,
    #[cfg(test)]
    recycled_commits: u64,
}

impl SharedNativeOutputCache {
    pub(crate) fn new(output: VideoOutputId) -> Self {
        Self {
            cache: NativeOutputCache::new(output),
            snapshot_owner: None,
            recycled: None,
            #[cfg(test)]
            recycled_commits: 0,
        }
    }

    /// The displaced LastValid's packed buffer and charge, for the next
    /// composition plan (`NativeCompositionPlan::reuse_cpu_snapshot`).
    pub(crate) fn take_recycled_cpu_snapshot(&mut self) -> Option<RecycledCpuSnapshot> {
        self.recycled.take()
    }

    /// Commits whose displaced LastValid became the recycled buffer.
    #[cfg(test)]
    pub(crate) fn recycled_commits(&self) -> u64 {
        self.recycled_commits
    }

    pub(crate) fn begin_request(
        &mut self,
        key: VideoOutputLastValidKey,
        frame: NativeFrameId,
    ) -> Result<CachePublicationFence, CacheFenceError> {
        let result = self.cache.begin_request(key, frame);
        if self.cache.cached.is_none() {
            self.snapshot_owner = None;
            self.recycled = None;
        }
        result
    }

    #[cfg(test)]
    pub(crate) fn last_valid(&self, key: &VideoOutputLastValidKey) -> Option<&VideoFrame> {
        self.cache.last_valid(key)
    }

    pub(crate) fn last_valid_with_owner(
        &self,
    ) -> Option<(
        &VideoOutputLastValidKey,
        &VideoFrame,
        &Arc<NativeSnapshotGpu>,
    )> {
        let (key, frame) = self.cache.cached.as_ref()?;
        Some((key, frame, self.snapshot_owner.as_ref()?))
    }

    pub(crate) fn take_last_valid_with_owner(
        &mut self,
    ) -> Option<(VideoOutputLastValidKey, VideoFrame, Arc<NativeSnapshotGpu>)> {
        let owner = self.snapshot_owner.take()?;
        self.recycled = None;
        match self.cache.take_last_valid() {
            Some((key, frame)) => Some((key, frame, owner)),
            None => {
                self.snapshot_owner = Some(owner);
                None
            }
        }
    }

    /// Whether this request's completed snapshot may still be published:
    /// the current request, or the snapshot ring's displaced predecessor.
    pub(crate) fn accepts_publication(&self, request: &NativeOutputRequest) -> bool {
        self.cache
            .accepts_publication(&request.key, request.frame, request.fence)
    }

    pub(crate) fn blackout(&mut self) -> Result<(), CacheFenceError> {
        let result = self.cache.blackout();
        // Pixels are destroyed before releasing their charge owner, on overflow too.
        self.snapshot_owner = None;
        self.recycled = None;
        result
    }

    fn commit(
        &mut self,
        snapshot: CompletedPremappingSnapshot<NativeSnapshotGpu>,
        descriptor: &CacheCommitDescriptor,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<
        (Arc<NativeSnapshotGpu>, CacheCommitReceipt),
        (OutputRejection, Arc<NativeSnapshotGpu>),
    > {
        let mut writer = self.cache.writer(cancel);
        // This binding is minted only by the sealed real-completion path below.
        let binding = writer.bind_validated(ValidatedSnapshotBinding {
            descriptor: descriptor.clone(),
        });
        if let Err(reason) = binding {
            writer.discard_binding();
            drop(writer);
            // The only writer is freshly constructed above, so this cannot fail.
            // Keep the completed owner if that invariant ever changes.
            return Err(self.reject_completed(snapshot, descriptor, reason, cancel));
        }
        let publication = snapshot.commit(&mut writer, descriptor, cancel);
        writer.discard_binding();
        let displaced = writer.take_displaced();
        drop(writer);
        match publication {
            Ok(bound) => {
                let (gpu, receipt) = bound.into_parts();
                let owner = Arc::new(gpu);
                // No callbacks/authority hooks occur between pixels publication
                // and this owner move. A later Stop leaves this exact pair alive.
                let previous = self.snapshot_owner.replace(Arc::clone(&owner));
                // The displaced pixels and their previous owner leave together:
                // recycled as one charged buffer, or dropped as before.
                self.recycled = previous.zip(displaced).and_then(|(previous, displaced)| {
                    NativeSnapshotGpu::recycle(previous, displaced)
                });
                #[cfg(test)]
                if self.recycled.is_some() {
                    self.recycled_commits += 1;
                }
                Ok((owner, receipt))
            }
            Err(failure) => Err((
                cancelled_or_cache(failure.reason, cancel),
                Arc::new(failure.gpu),
            )),
        }
    }

    fn reject_completed(
        &mut self,
        snapshot: CompletedPremappingSnapshot<NativeSnapshotGpu>,
        descriptor: &CacheCommitDescriptor,
        reason: SnapshotCommitRejection,
        cancel: &dyn VideoRenderCancellation,
    ) -> (OutputRejection, Arc<NativeSnapshotGpu>) {
        // A rejecting cache never publishes pixels, but P returns its GPU owner.
        struct Reject(SnapshotCommitRejection);
        impl CpuSnapshotCommit for Reject {
            fn commit(
                &mut self,
                _: &VideoOutputLastValidKey,
                _: NativeFrameId,
                _: VideoFrame,
                _: CachePublicationFence,
            ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
                Err(self.0)
            }
            #[cfg(test)]
            fn invalidate_output(&mut self, _: VideoOutputId) {}
        }
        match snapshot.commit(&mut Reject(reason), descriptor, cancel) {
            Err(failure) => (
                cancelled_or_cache(failure.reason, cancel),
                Arc::new(failure.gpu),
            ),
            Ok(_) => unreachable!("rejecting cache cannot publish"),
        }
    }
}

/// The transaction binding is captured where I actually calls P's encoder.
/// It cannot be created from a caller-supplied completion descriptor.
pub(crate) struct PendingSharedNativeOutput {
    request: NativeOutputRequest,
    transaction_id: RetirementId,
    pending: Option<PendingGpuTransaction>,
    isf: Option<EncodedIsfBatch>,
    snapshot: Option<SharedPremappingSnapshot>,
    failed_snapshot: Option<EncodedPremappingSnapshot>,
    composition_failure: Option<NativeGpuFault>,
    composition_resolved: Option<bool>,
    validated: Option<SharedValidatedOutput>,
    history_plan: Option<NativeOutputHistoryPlan>,
}

enum SharedValidatedOutput {
    Unpublished {
        layers: StackBatch<ValidatedLayerResult>,
        receipt: IsfValidatedReceipt,
    },
    History(NativeOutputHistoryFrame),
}

impl SharedValidatedOutput {
    fn receipt(&self) -> Option<&IsfValidatedReceipt> {
        match self {
            Self::Unpublished { receipt, .. } => Some(receipt),
            Self::History(history) => history.receipt(),
        }
    }
}

pub(crate) enum SharedNativeOutputProgress {
    Pending(PendingSharedNativeOutput),
    Rejected {
        pending: PendingSharedNativeOutput,
        reason: OutputRejection,
    },
    Ready {
        ready: ReadyNativeGpuOutput<Arc<NativeSnapshotGpu>>,
        layers: StackBatch<ValidatedLayerResult>,
        isf: IsfValidatedReceipt,
    },
    HistoryReady {
        ready: ReadyNativeGpuOutput<Arc<NativeSnapshotGpu>>,
        history: NativeOutputHistoryFrame,
        /// The completed, unmapped snapshot readback, returned for reuse by
        /// the resident snapshot ring. `None` drops it with this output.
        readback: Option<Arc<NativeChargedBuffer>>,
    },
    Retiring(SharedNativeOutputRetirement),
}

pub(crate) struct SharedNativeOutputRetirement {
    context: GpuContextId,
    reason: Option<OutputRejection>,
    transaction: Option<RetiringGpuTransaction>,
    unsubmitted: Option<FrameReservation>,
    admitted: Option<PreparedNativeBatch>,
    snapshot: Option<RetiringPremappingSnapshot<NativeSnapshotGpu, NativeSnapshotMap>>,
    failed_snapshot: Option<EncodedPremappingSnapshot>,
    completed: Option<CompletedPremappingSnapshot<NativeSnapshotGpu>>,
    gpu: Option<Arc<NativeSnapshotGpu>>,
    encoded_isf: Option<EncodedIsfBatch>,
    isf_outcome: Option<NativeIsfCompletion>,
    history_outcome: Option<NativeOutputHistoryFrame>,
    reaped: bool,
}

/// Release P's pooled/Ready owners before marking the shared generation stopped.
/// Submitted resources remain in G's registry; this ticket is not a Stop ACK.
/// The caller retains the broker, cache/history and any I retirement through
/// actual Reaped. Window ownership and resize stay with the app's window thread.
pub(crate) fn release_shared_presenter_for_stop<T: NativePresentTarget>(
    presenter: &mut SharedNativePresenter<T>,
    owner: &mut NativeGpuOwner,
) -> Result<StopRetirement, NativeGpuFault> {
    presenter.release_for_stop(owner)?;
    Ok(owner.begin_stop())
}

impl SharedNativeOutputRetirement {
    /// A fixed-clock CPU recovery has already drained the real two-phase
    /// transaction. Release only its failed resident output buffers without
    /// stopping the healthy shared owner. Every other case returns the complete
    /// aggregate so cancellation/fault cleanup retains all outstanding work.
    pub(crate) fn into_recovered_history(
        mut self,
        owner: &mut NativeGpuOwner,
    ) -> Result<NativeOutputHistoryFrame, (Self, NativeGpuFault)> {
        let valid = self.context == owner.context().id()
            && owner.state() == GpuOwnerState::Active
            && self.reason.is_none()
            && self.transaction.is_none()
            && self.unsubmitted.is_none()
            && self.admitted.is_none()
            && self.encoded_isf.is_none()
            && self.isf_outcome.is_none()
            && self
                .history_outcome
                .as_ref()
                .is_some_and(|history| history.is_recovered() && history.error().is_none());
        if !valid {
            let fault = NativeGpuFault {
                context: self.context,
                frame: None,
                phase: GpuPhase::Retirement,
                kind: NativeGpuFaultKind::StateMismatch,
                message: "only drained fixed-clock CPU recovery may resume a healthy owner".into(),
            };
            return Err((self, fault));
        }
        // `transaction == None` in this recovery branch is established only
        // after poll_transaction returned its real resolved transaction. No
        // submission or error scope can still use these failed output owners.
        if let Some(snapshot) = self.snapshot.take() {
            let (gpu, mapper) = snapshot.into_parts();
            drop(mapper);
            drop(gpu);
        }
        self.failed_snapshot = None;
        self.completed = None;
        self.gpu = None;
        if let Err(fault) = owner.poll_retirement() {
            return Err((self, fault));
        }
        Ok(self
            .history_outcome
            .take()
            .expect("checked recovered history owner"))
    }

    pub(crate) fn reason(&self) -> Option<&OutputRejection> {
        self.reason.as_ref()
    }

    /// ISF history remains available even if composition/map/cache failed.
    /// Recovered CPU outcomes retain their exact typed faults, pixels and charges.
    pub(crate) fn take_isf_outcome(&mut self) -> Option<NativeIsfCompletion> {
        self.isf_outcome.take()
    }

    /// Includes partial seals/commits or recovered CPU backing and exact faults.
    /// The caller retains this owner while applying canonical history policy.
    pub(crate) fn take_history_outcome(&mut self) -> Option<NativeOutputHistoryFrame> {
        self.history_outcome.take()
    }

    /// C4 Stop entry: release the paired presenter before poll starts G Stop.
    /// A foreign broker does not release P's owners or consume this aggregate.
    pub(crate) fn poll_with_presenter<T: NativePresentTarget>(
        &mut self,
        presenter: &mut SharedNativePresenter<T>,
        owner: &mut NativeGpuOwner,
    ) -> Result<bool, NativeGpuFault> {
        if owner.context().id() != self.context || self.reaped {
            return self.poll(owner);
        }
        presenter.release_for_stop(owner)?;
        self.poll(owner)
    }

    /// Retain this aggregate and the broker through Reaped. The inner snapshot
    /// must be released only AFTER transaction drain: keeping its charges until
    /// Reaped would deadlock because Reaped itself requires zero allocations.
    pub(crate) fn poll(&mut self, owner: &mut NativeGpuOwner) -> Result<bool, NativeGpuFault> {
        if owner.context().id() != self.context {
            return Err(NativeGpuFault {
                context: self.context,
                frame: None,
                phase: GpuPhase::Retirement,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "retirement owner belongs to another context".into(),
            });
        }
        // Another cleanup consumer (including P's retirement poll) can observe
        // the broker's terminal result first. Reaped is sticky owner state even
        // when G's subsequent poll returns Pending; preserve that completion.
        if self.reaped || owner.state() == GpuOwnerState::Reaped {
            self.reaped = true;
            self.transaction = None;
            return Ok(true);
        }
        owner.begin_stop();
        match owner.poll_retirement()? {
            RetirementProgress::Reaped => self.reaped = true,
            RetirementProgress::Pending {
                transactions: 0, ..
            } if owner.state() == GpuOwnerState::Retiring => {
                // Keep this aggregate's unused root and payload owners until
                // its transactions drain. G3 permits earlier root refund when
                // only a present retires; this Stop path deliberately retains
                // the complete failure owner through the same cleanup boundary.
                if let Some(reservation) = self.unsubmitted.take() {
                    if let Err((error, reservation)) =
                        owner.discard_unsubmitted_reservation(reservation)
                    {
                        self.unsubmitted = Some(reservation);
                        return Err(error);
                    }
                    self.admitted = None;
                }
                // No scope/submission remains; mapper Drop can now safely unmap.
                self.snapshot = None;
                self.failed_snapshot = None;
                self.completed = None;
                self.gpu = None;
                self.encoded_isf = None;
                // An unconsumed ISF outcome stays with this aggregate. It is
                // intentionally not discarded: the history owner must take it.
            }
            RetirementProgress::Pending { .. } => {}
        }
        if self.reaped {
            self.transaction = None;
        }
        Ok(self.reaped)
    }
}

impl PendingSharedNativeOutput {
    /// History-aware C3 entry. Reject a foreign session/plan before begin_frame
    /// or ISF encoding and return the unsubmitted admission, reservation and plan.
    /// A caller with a presenter first performs its nonblocking per-tick poll.
    pub(crate) fn submit_with_history(
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        compositor: &NativeGpuCompositor,
        request: NativeOutputRequest,
        admitted: PreparedNativeBatch,
        reservation: FrameReservation,
        composition: &NativeCompositionPlan,
        selected_history: &[&GpuLayerLease],
        session: &NativeOutputHistorySession,
        history_plan: NativeOutputHistoryPlan,
    ) -> Result<Self, (SharedNativeOutputRetirement, NativeOutputHistoryPlan)> {
        if !session.accepts_plan(&history_plan)
            || history_plan.context() != request.context
            || history_plan.frame() != request.frame
        {
            return Err((
                Self::retain_unsubmitted(
                    history_rejection(&request, HistoryError::IdentityMismatch),
                    admitted,
                    reservation,
                ),
                history_plan,
            ));
        }
        match Self::submit(
            owner,
            runtime,
            compositor,
            request,
            admitted,
            reservation,
            composition,
            selected_history,
        ) {
            Ok(mut pending) => {
                pending.history_plan = Some(history_plan);
                Ok(pending)
            }
            Err(retirement) => Err((retirement, history_plan)),
        }
    }

    /// Bind the prospective history plan made before submit. Rejection returns
    /// both affine owners; no already-submitted work is discarded.
    #[cfg(test)]
    pub(crate) fn bind_history(
        mut self,
        plan: NativeOutputHistoryPlan,
    ) -> Result<Self, (HistoryError, Self, NativeOutputHistoryPlan)> {
        if plan.context() != self.request.context
            || plan.frame() != self.request.frame
            || self.history_plan.is_some()
            || self.validated.is_some()
        {
            return Err((HistoryError::IdentityMismatch, self, plan));
        }
        self.history_plan = Some(plan);
        Ok(self)
    }

    /// Poll P's preceding present without waiting, then admit the next C3 frame.
    /// G3 lets sealed present work retire alongside one new logical frame.
    /// P drains that work only before its next MappingPresent phase. Recheck
    /// authority/Stop after the poll and before beginning the new transaction.
    /// A failure keeps the unsubmitted affine admission/reservation intact.
    #[cfg(test)]
    pub(crate) fn submit_after_present<T: NativePresentTarget>(
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        compositor: &NativeGpuCompositor,
        presenter: &mut SharedNativePresenter<T>,
        request: NativeOutputRequest,
        admitted: PreparedNativeBatch,
        reservation: FrameReservation,
        composition: &NativeCompositionPlan,
        history: &[&GpuLayerLease],
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<Self, SharedNativeOutputRetirement> {
        if let Err(reason) = Self::validate_submission(owner, &request, &reservation, composition) {
            return Err(Self::retain_unsubmitted(reason, admitted, reservation));
        }
        let allowed = presenter
            .check_owner(owner)
            .map_err(OutputRejection::Fault)
            .and_then(|()| guard(&request, OutputBoundary::BeforeSubmit, authority, cancel));
        if let Err(reason) = allowed {
            return Err(Self::retain_unsubmitted(reason, admitted, reservation));
        }
        if let Err(error) = presenter.poll_retirements(owner) {
            return Err(Self::retain_unsubmitted(
                OutputRejection::Fault(error),
                admitted,
                reservation,
            ));
        }
        if let Err(reason) = guard(&request, OutputBoundary::BeforeSubmit, authority, cancel) {
            return Err(Self::retain_unsubmitted(reason, admitted, reservation));
        }
        Self::submit(
            owner,
            runtime,
            compositor,
            request,
            admitted,
            reservation,
            composition,
            history,
        )
    }

    fn validate_submission(
        owner: &NativeGpuOwner,
        request: &NativeOutputRequest,
        reservation: &FrameReservation,
        composition: &NativeCompositionPlan,
    ) -> Result<(), OutputRejection> {
        check_request(request)?;
        if request.context != owner.context().id()
            || reservation.context() != owner.context().id()
            || reservation.frame() != request.frame
            || composition.context != request.context
            || composition.frame != request.frame
            || composition.extent != (request.key.width, request.key.height)
        {
            Err(OutputRejection::Fault(fault(
                request,
                GpuPhase::Initialize,
                NativeGpuFaultKind::IdentityMismatch,
            )))
        } else {
            Ok(())
        }
    }

    fn retain_unsubmitted(
        reason: OutputRejection,
        admitted: PreparedNativeBatch,
        reservation: FrameReservation,
    ) -> SharedNativeOutputRetirement {
        SharedNativeOutputRetirement {
            context: reservation.context(),
            reason: Some(reason),
            unsubmitted: Some(reservation),
            admitted: Some(admitted),
            transaction: None,
            snapshot: None,
            failed_snapshot: None,
            completed: None,
            gpu: None,
            encoded_isf: None,
            isf_outcome: None,
            history_outcome: None,
            reaped: false,
        }
    }

    /// Call after ONE merged quote/reservation including validated-history aliases.
    /// This function owns the order: P reservation split, ISF Batch3 submit,
    /// composition/snapshot submit, then map registration. There is no poll/wait
    /// or surface acquisition between the two submissions.
    pub(crate) fn submit(
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        compositor: &NativeGpuCompositor,
        request: NativeOutputRequest,
        admitted: PreparedNativeBatch,
        reservation: FrameReservation,
        composition: &NativeCompositionPlan,
        history: &[&GpuLayerLease],
    ) -> Result<Self, SharedNativeOutputRetirement> {
        if let Err(reason) = Self::validate_submission(owner, &request, &reservation, composition) {
            return Err(Self::retain_unsubmitted(reason, admitted, reservation));
        }
        let identities = match admitted.prospective_identities(runtime) {
            Ok(identities) => identities,
            Err(error) => {
                let reason = OutputRejection::Fault(NativeGpuFault {
                    context: request.context,
                    frame: Some(request.frame),
                    phase: GpuPhase::IsfBatch,
                    kind: NativeGpuFaultKind::StateMismatch,
                    message: format!("admitted native identities changed before submit: {error:?}"),
                });
                return Err(Self::retain_unsubmitted(reason, admitted, reservation));
            }
        };
        // Validate the original frozen selection even if G later cannot encode
        // its resident candidates. An independent P mismatch remains a fault,
        // while a valid selection does not invent one from G's missing textures.
        let selection_failure = composition
            .layers_bottom_first
            .iter()
            .any(|layer| {
                let count = match layer.kind {
                    SelectedTextureKind::InThisTransaction => identities
                        .iter()
                        .filter(|identity| {
                            **identity == layer.identity && identity.entry == layer.selected_entry
                        })
                        .count(),
                    SelectedTextureKind::Validated => history
                        .iter()
                        .filter(|lease| {
                            lease.identity() == &layer.identity
                                && lease.identity().entry == layer.selected_entry
                        })
                        .count(),
                };
                count != 1
            })
            .then(|| {
                fault(
                    &request,
                    GpuPhase::CompositionSnapshot,
                    NativeGpuFaultKind::IdentityMismatch,
                )
            });
        let transaction = owner.begin_frame(request.frame, reservation);
        let mut txn = match transaction {
            Ok(txn) => txn,
            Err(error) => {
                // No submitted GPU owner is lost; an invalid reservation remains
                // accounted by G. This is terminal, never a preflight decline.
                return Err(SharedNativeOutputRetirement {
                    context: request.context,
                    reason: Some(OutputRejection::Fault(error)),
                    transaction: None,
                    unsubmitted: None,
                    admitted: None,
                    snapshot: None,
                    failed_snapshot: None,
                    completed: None,
                    gpu: None,
                    encoded_isf: None,
                    isf_outcome: None,
                    history_outcome: None,
                    reaped: false,
                });
            }
        };
        let transaction_id = txn.id();
        let p_reservation = match compositor.reserve(&mut txn, composition) {
            Ok(reservation) => reservation,
            Err(error) => {
                let retirement = txn.cancel();
                return Err(Self::failed_submit(&request, error, retirement, None));
            }
        };
        let isf = match runtime.encode_native_batch(&mut txn, admitted) {
            Ok(isf) => isf,
            Err(error) => {
                let retirement = txn.cancel();
                return Err(Self::failed_submit(&request, error, retirement, None));
            }
        };
        // Every current chain executes, but only the canonical plan's selected
        // entry composes. A historical predecessor can replace current layer 1.
        let encoded = if let Some(error) = selection_failure {
            drop(p_reservation);
            Some(Err(error))
        } else if isf.candidate_availability()
            == NativeCandidateAvailability::UnavailableAfterEncodeFailure
        {
            // G owns an actual failed phase and all immutable recovery inputs.
            // No resident P phase can be encoded. Await G's real completion;
            // only a validated fixed-clock CPU recovery may resume canonically.
            drop(p_reservation);
            None
        } else {
            let selected = composition
                .layers_bottom_first
                .iter()
                .map(|layer| {
                    let mismatch = || {
                        fault(
                            &request,
                            GpuPhase::CompositionSnapshot,
                            NativeGpuFaultKind::IdentityMismatch,
                        )
                    };
                    match layer.kind {
                        SelectedTextureKind::Validated => {
                            let mut matches = history.iter().copied().filter(|lease| {
                                lease.identity() == &layer.identity
                                    && lease.identity().entry == layer.selected_entry
                            });
                            let lease = matches.next().ok_or_else(mismatch)?;
                            if matches.next().is_some() {
                                return Err(mismatch());
                            }
                            Ok(SelectedLayerInput::Validated(lease))
                        }
                        SelectedTextureKind::InThisTransaction => {
                            let mut matches = isf.candidates.iter().filter(|candidate| {
                                candidate.identity() == &layer.identity
                                    && candidate.identity().entry == layer.selected_entry
                            });
                            let candidate = matches.next().ok_or_else(mismatch)?;
                            if matches.next().is_some() {
                                return Err(mismatch());
                            }
                            Ok(SelectedLayerInput::InThisTransaction(candidate))
                        }
                    }
                })
                .collect::<Result<Vec<_>, NativeGpuFault>>();
            Some(match selected {
                Ok(selected) => compositor.encode_composition_and_snapshot(
                    &mut txn,
                    composition,
                    &selected,
                    p_reservation,
                ),
                Err(error) => Err(error),
            })
        };
        let pending = txn.finish();
        let (snapshot, failed_snapshot, composition_failure) = match encoded {
            Some(Ok(encoded)) => {
                // P's real constructor binds this submitted copy to the map.
                match NativeGpuSnapshotter::begin_shared(
                    encoded,
                    request.key.clone(),
                    request.fence,
                    request.pts_ms,
                    request.duration_ms,
                ) {
                    Ok(snapshot) => (Some(snapshot), None, None),
                    Err(failure) => {
                        // Retain the actual returned owner and resolve the real
                        // ISF phase before retiring. A P failure cannot discard
                        // successful current results needed by history.
                        (None, Some(failure.encoded), Some(failure.fault))
                    }
                }
            }
            Some(Err(error)) => (None, None, Some(error)),
            None => (None, None, None),
        };
        Ok(Self {
            request,
            transaction_id,
            pending: Some(pending),
            isf: Some(isf),
            snapshot,
            failed_snapshot,
            composition_failure,
            composition_resolved: None,
            validated: None,
            history_plan: None,
        })
    }

    fn failed_submit(
        request: &NativeOutputRequest,
        error: NativeGpuFault,
        transaction: RetiringGpuTransaction,
        isf: Option<EncodedIsfBatch>,
    ) -> SharedNativeOutputRetirement {
        SharedNativeOutputRetirement {
            context: request.context,
            reason: Some(OutputRejection::Fault(error)),
            transaction: Some(transaction),
            unsubmitted: None,
            admitted: None,
            snapshot: None,
            failed_snapshot: None,
            completed: None,
            gpu: None,
            encoded_isf: isf,
            isf_outcome: None,
            history_outcome: None,
            reaped: false,
        }
    }

    pub(crate) fn retire(
        self,
        owner: &mut NativeGpuOwner,
        reason: OutputRejection,
    ) -> SharedNativeOutputRetirement {
        self.retire_with_reason(owner, Some(reason))
    }

    fn retire_with_reason(
        mut self,
        owner: &mut NativeGpuOwner,
        reason: Option<OutputRejection>,
    ) -> SharedNativeOutputRetirement {
        // A P fault already observed during submit/resolution survives a later
        // Stop or authority revocation. Keep its typed diagnostic together with
        // the complete transaction owner; control must not erase an actual fault.
        let reason = self
            .composition_failure
            .take()
            .map(OutputRejection::Fault)
            .or(reason);
        let transaction = self
            .pending
            .take()
            .map(|pending| owner.cancel_and_retire(pending));
        // Defer begin_stop to the retirement consumer. If P is present, its
        // pooled/Ready owners must be released first via poll_with_presenter.
        let (isf_outcome, history_outcome) = match self.validated.take() {
            Some(SharedValidatedOutput::Unpublished { layers, receipt }) => (
                Some(NativeIsfCompletion::Validated { layers, receipt }),
                None,
            ),
            Some(SharedValidatedOutput::History(history)) => (None, Some(history)),
            None => (None, None),
        };
        SharedNativeOutputRetirement {
            context: self.request.context,
            reason,
            transaction,
            unsubmitted: None,
            admitted: None,
            snapshot: self
                .snapshot
                .take()
                .map(SharedPremappingSnapshot::into_retirement),
            failed_snapshot: self.failed_snapshot.take(),
            completed: None,
            gpu: None,
            encoded_isf: self.isf.take(),
            isf_outcome,
            history_outcome,
            reaped: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn poll(
        self,
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        cache: &mut SharedNativeOutputCache,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> SharedNativeOutputProgress {
        self.poll_inner(owner, runtime, cache, None, authority, cancel, false)
    }

    pub(crate) fn poll_with_history(
        self,
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        cache: &mut SharedNativeOutputCache,
        history: &mut NativeOutputHistorySession,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> SharedNativeOutputProgress {
        self.poll_inner(
            owner,
            runtime,
            cache,
            Some(history),
            authority,
            cancel,
            false,
        )
    }

    /// Snapshot-ring first stage: drive the G transaction, ISF completion and
    /// the canonical history commit exactly as `poll_with_history`, but stop
    /// before P's snapshot map/copy and the cache commit. The returned
    /// `Pending` keeps the resolved snapshot; a later `poll_with_history`
    /// resumes at the snapshot. Never yields Ready.
    pub(crate) fn poll_phases_with_history(
        self,
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        cache: &mut SharedNativeOutputCache,
        history: &mut NativeOutputHistorySession,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> SharedNativeOutputProgress {
        self.poll_inner(
            owner,
            runtime,
            cache,
            Some(history),
            authority,
            cancel,
            true,
        )
    }

    /// The G transaction drained and history committed: only the snapshot
    /// map/copy and cache publication remain. No GPU work uses its owners.
    pub(crate) fn phases_resolved(&self) -> bool {
        self.pending.is_none()
            && self.isf.is_none()
            && self.composition_resolved.is_none()
            && self.composition_failure.is_none()
            && self.snapshot.is_some()
            && matches!(&self.validated, Some(SharedValidatedOutput::History(frame))
                if frame.output_phases_valid())
    }

    pub(crate) fn request(&self) -> &NativeOutputRequest {
        &self.request
    }

    /// Release the residents the committed history evicted. They were last
    /// read by this already completed transaction.
    pub(crate) fn take_history_retired(
        &mut self,
    ) -> Vec<crate::native_gpu_history::RetiredResident<GpuLayerLease>> {
        match self.validated.as_mut() {
            Some(SharedValidatedOutput::History(frame)) => frame.take_retired(),
            _ => Vec::new(),
        }
    }

    /// Zero-present discard of a resolved output whose publication is no
    /// longer wanted (blackout, configuration or mapping change, a revoked
    /// tick, a non-resident tick). Its transaction completed and its history
    /// is committed, so no submission or scope uses these owners; G reaps the
    /// charges once they drop. Any other state is returned for retirement.
    pub(crate) fn discard_resolved(self) -> Result<(), Self> {
        if !self.phases_resolved() {
            return Err(self);
        }
        drop(self);
        Ok(())
    }

    fn poll_inner(
        mut self,
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        cache: &mut SharedNativeOutputCache,
        mut history: Option<&mut NativeOutputHistorySession>,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
        defer_snapshot: bool,
    ) -> SharedNativeOutputProgress {
        if owner.context().id() != self.request.context {
            let error = fault(
                &self.request,
                GpuPhase::Retirement,
                NativeGpuFaultKind::IdentityMismatch,
            );
            // Do not mutate a foreign owner. The caller must retain this pending
            // and resume with the correct broker; this check is before polling.
            return SharedNativeOutputProgress::Rejected {
                pending: self,
                reason: OutputRejection::Fault(error),
            };
        }
        let bound = self.history_plan.is_some()
            || matches!(self.validated, Some(SharedValidatedOutput::History(_)));
        let correct_session = match history.as_deref() {
            Some(session) => {
                self.history_plan
                    .as_ref()
                    .is_some_and(|plan| session.accepts_plan(plan))
                    || matches!(
                        &self.validated,
                        Some(SharedValidatedOutput::History(frame))
                            if session.accepts_frame(frame)
                    )
            }
            None => !bound,
        };
        if !correct_session {
            return SharedNativeOutputProgress::Rejected {
                reason: history_rejection(&self.request, HistoryError::IdentityMismatch),
                pending: self,
            };
        }
        if let Err(reason) = guard(&self.request, OutputBoundary::BeforePoll, authority, cancel) {
            return SharedNativeOutputProgress::Retiring(self.retire(owner, reason));
        }
        if let Some(pending) = self.pending.as_mut() {
            let mut resolved = match owner.poll_transaction(pending, cancel) {
                CompletionProgress::Pending => return SharedNativeOutputProgress::Pending(self),
                CompletionProgress::Cancelled => {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, OutputRejection::Cancelled),
                    )
                }
                CompletionProgress::Complete(Err(error)) => {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, OutputRejection::Fault(error)),
                    );
                }
                CompletionProgress::Complete(Ok(resolved)) => resolved,
            };
            // Terminal token is no longer used to cancel an already-finished record.
            self.pending = None;
            let mut encoded = self.isf.take().expect("submitted shared output owns ISF");
            let phase = match encoded
                .pending
                .take()
                .and_then(|receipt| resolved.take_phase(receipt))
            {
                Ok(phase) => phase,
                Err(error) => {
                    self.isf = Some(encoded);
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, OutputRejection::Fault(error)),
                    );
                }
            };
            match runtime.complete_native_batch(encoded, phase, owner, cancel) {
                NativeIsfCompletion::Validated { layers, receipt } => {
                    self.validated = Some(match self.history_plan.take() {
                        Some(plan) => SharedValidatedOutput::History(
                            history
                                .as_deref()
                                .expect("history session checked before polling")
                                .seal_validated(plan, layers, receipt),
                        ),
                        None => SharedValidatedOutput::Unpublished { layers, receipt },
                    });
                    if let Some(SharedValidatedOutput::History(frame)) = &self.validated {
                        if let Some(error) = frame.error() {
                            let reason = history_rejection(&self.request, error);
                            return SharedNativeOutputProgress::Retiring(
                                self.retire(owner, reason),
                            );
                        }
                    }
                }
                NativeIsfCompletion::RecoveredCpu { layers } if self.history_plan.is_some() => {
                    // G drains both submissions, but P's separate validation
                    // outcome still has to be consumed before healthy recovery.
                    if self.composition_failure.is_none() {
                        if let Some(snapshot) = self.snapshot.as_mut() {
                            if let Err(error) = snapshot.resolve_shared_phase(&mut resolved) {
                                self.composition_failure = Some(error);
                            }
                        }
                    }
                    let recovered = history
                        .as_deref()
                        .expect("history session checked before polling")
                        .seal_recovered(
                            self.history_plan.take().expect("recovery keeps its plan"),
                            layers,
                        );
                    let reason = self
                        .composition_failure
                        .take()
                        .map(OutputRejection::Fault)
                        .or_else(|| {
                            recovered
                                .error()
                                .map(|error| history_rejection(&self.request, error))
                        });
                    self.validated = Some(SharedValidatedOutput::History(recovered));
                    return SharedNativeOutputProgress::Retiring(
                        self.retire_with_reason(owner, reason),
                    );
                }
                outcome => {
                    let reason = match &outcome {
                        NativeIsfCompletion::Cancelled { retirement } => {
                            if retirement.is_some_and(|id| id.context != self.request.context) {
                                Some(OutputRejection::Fault(fault(
                                    &self.request,
                                    GpuPhase::Retirement,
                                    NativeGpuFaultKind::IdentityMismatch,
                                )))
                            } else {
                                Some(OutputRejection::Cancelled)
                            }
                        }
                        NativeIsfCompletion::Failed { fault, retirement } => {
                            let mut diagnostic = copy_fault(fault);
                            if let Some(retirement) = retirement {
                                diagnostic.message = format!(
                                    "{}; retained G retirement {retirement:?}",
                                    diagnostic.message
                                );
                            }
                            Some(OutputRejection::Fault(diagnostic))
                        }
                        NativeIsfCompletion::RecoveredCpu { .. } => None,
                        NativeIsfCompletion::Validated { .. } => unreachable!(),
                    };
                    let mut retiring = self.retire_with_reason(owner, reason);
                    // Canonical same-clock recovery is an outcome for the CPU
                    // history consumer, not an invented resident GPU fault.
                    retiring.isf_outcome = Some(outcome);
                    return SharedNativeOutputProgress::Retiring(retiring);
                }
            }
            if self.composition_failure.is_none() {
                if let Some(snapshot) = self.snapshot.as_mut() {
                    if let Err(error) = snapshot.resolve_shared_phase(&mut resolved) {
                        self.composition_failure = Some(error);
                    }
                }
            }
            self.composition_resolved = Some(self.composition_failure.is_none());
        }
        if let Err(reason) = guard(
            &self.request,
            OutputBoundary::PhasesResolved,
            authority,
            cancel,
        ) {
            return SharedNativeOutputProgress::Retiring(self.retire(owner, reason));
        }
        if let Some(valid) = self.composition_resolved.take() {
            let history_result = self.resolve_history(history.as_deref_mut(), valid);
            // Keep the original P fault as the primary error even when history
            // publication also rejects; its typed error remains in history_outcome.
            if let Some(error) = self.composition_failure.take() {
                return SharedNativeOutputProgress::Retiring(
                    self.retire(owner, OutputRejection::Fault(error)),
                );
            }
            if let Err(reason) = history_result {
                return SharedNativeOutputProgress::Retiring(self.retire(owner, reason));
            }
        }
        if let Err(reason) = self.validate_completion() {
            return SharedNativeOutputProgress::Retiring(self.retire(owner, reason));
        }
        if defer_snapshot {
            return SharedNativeOutputProgress::Pending(self);
        }
        let snapshot = self
            .snapshot
            .as_mut()
            .expect("validated shared snapshot exists");
        let descriptor = snapshot.descriptor().clone();
        let completed =
            match NativeGpuSnapshotter::poll_shared(snapshot, owner, &descriptor, cancel) {
                SnapshotProgress::Pending => return SharedNativeOutputProgress::Pending(self),
                SnapshotProgress::Cancelled => {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, OutputRejection::Cancelled),
                    )
                }
                SnapshotProgress::Rejected(reason) => {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, cancelled_or_cache(reason, cancel)),
                    )
                }
                SnapshotProgress::Failed(error) => {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, OutputRejection::Fault(error)),
                    )
                }
                SnapshotProgress::Complete(completed) => completed,
            };
        for boundary in [
            OutputBoundary::SnapshotCopiedUnmapped,
            OutputBoundary::BeforeCacheCommit,
        ] {
            if let Err(reason) = guard(&self.request, boundary, authority, cancel) {
                let mut retiring = self.retire(owner, reason);
                retiring.completed = Some(completed);
                return SharedNativeOutputProgress::Retiring(retiring);
            }
        }
        let (gpu, receipt) = match cache.commit(completed, &descriptor, cancel) {
            Ok(value) => value,
            Err((reason, gpu)) => {
                let mut retiring = self.retire(owner, reason);
                retiring.gpu = Some(gpu);
                return SharedNativeOutputProgress::Retiring(retiring);
            }
        };
        for boundary in [
            OutputBoundary::AfterCacheCommit,
            OutputBoundary::BeforeReady,
        ] {
            if let Err(reason) = guard(&self.request, boundary, authority, cancel) {
                let mut retiring = self.retire(owner, reason);
                retiring.gpu = Some(gpu);
                return SharedNativeOutputProgress::Retiring(retiring);
            }
        }
        // P's completed map/pixels and real ISF proof were joined above. No
        // duplicated ResolvedGpuPhase or metadata-only success receipt is minted.
        let ready = ReadyNativeGpuOutput {
            gpu,
            request: self.request,
            snapshot: descriptor.snapshot,
            receipt,
            retry_remaining: true,
        };
        match self
            .validated
            .take()
            .expect("ISF proof validated before cache")
        {
            SharedValidatedOutput::Unpublished { layers, receipt } => {
                SharedNativeOutputProgress::Ready {
                    ready,
                    layers,
                    isf: receipt,
                }
            }
            SharedValidatedOutput::History(history) => SharedNativeOutputProgress::HistoryReady {
                ready,
                history,
                readback: self
                    .snapshot
                    .take()
                    .and_then(SharedPremappingSnapshot::into_reusable_readback),
            },
        }
    }

    fn resolve_history(
        &mut self,
        session: Option<&mut NativeOutputHistorySession>,
        valid: bool,
    ) -> Result<(), OutputRejection> {
        if let Some(SharedValidatedOutput::History(frame)) = &mut self.validated {
            session
                .ok_or_else(|| history_rejection(&self.request, HistoryError::StateMismatch))?
                .resolve_composition(frame, valid)
                .map_err(|error| history_rejection(&self.request, error))?;
        }
        Ok(())
    }

    fn validate_completion(&self) -> Result<(), OutputRejection> {
        let mismatch = || {
            OutputRejection::Fault(fault(
                &self.request,
                GpuPhase::CompositionSnapshot,
                NativeGpuFaultKind::IdentityMismatch,
            ))
        };
        check_request(&self.request)?;
        let validated = self.validated.as_ref().ok_or_else(mismatch)?;
        if matches!(validated, SharedValidatedOutput::History(frame) if !frame.output_phases_valid())
        {
            return Err(mismatch());
        }
        let receipt = validated.receipt().ok_or_else(mismatch)?;
        let phase = receipt.phase();
        phase
            .validation()
            .map_err(|error| OutputRejection::Fault(copy_fault(error)))?;
        let descriptor = self.snapshot.as_ref().ok_or_else(mismatch)?.descriptor();
        let snapshot = descriptor.snapshot;
        if phase.context() != self.request.context
            || phase.frame() != self.request.frame
            || phase.phase() != GpuPhase::IsfBatch
            || phase.transaction() != self.transaction_id
            || phase.submission().sequence.checked_add(1)
                != Some(snapshot.completed_submission.sequence)
            || snapshot.completed_submission.context != self.request.context
            || descriptor.key != self.request.key
            || descriptor.fence != self.request.fence
            || snapshot.frame != self.request.frame
            || snapshot.allocation.context != self.request.context
            || snapshot.width != self.request.key.width
            || snapshot.height != self.request.key.height
            || snapshot.pts_ms != self.request.pts_ms
            || snapshot.duration_ms != self.request.duration_ms
            || snapshot.format != VideoPixelFormat::Rgba8
        {
            return Err(mismatch());
        }
        Ok(())
    }
}

fn history_rejection(request: &NativeOutputRequest, error: HistoryError) -> OutputRejection {
    if error == HistoryError::Cancelled {
        return OutputRejection::Cancelled;
    }
    let kind = match error {
        HistoryError::IdentityMismatch => NativeGpuFaultKind::IdentityMismatch,
        HistoryError::SequenceExhausted => NativeGpuFaultKind::SequenceExhausted,
        _ => NativeGpuFaultKind::StateMismatch,
    };
    OutputRejection::Fault(NativeGpuFault {
        context: request.context,
        frame: Some(request.frame),
        phase: GpuPhase::IsfBatch,
        kind,
        message: format!("native history publication rejected: {error:?}"),
    })
}

#[cfg(test)]
#[path = "native_gpu_output_shared_tests.rs"]
mod tests;
