//! I3 headless join of the real G transaction and P composition/snapshot.
//! This remains behind native_gpu_output's test gate. Candidate preflight
//! metadata must come from G's prospective-identity API before production use.
//! No surface, mapping, display routing, new device, or whole-tick retry lives here.
use std::sync::Arc;

use super::*;
use crate::isf_native_gpu::PreparedNativeBatch;
use crate::isf_runtime::{IsfGpuRuntime, StackBatch};
use crate::native_gpu_budget::FrameReservation;
use crate::native_gpu_composition::{
    EncodedPremappingSnapshot, NativeCompositionPlan, NativeGpuCompositor, SelectedLayerInput,
    SelectedTextureKind,
};
use crate::native_gpu_context::{
    NativeGpuOwner, PendingGpuTransaction, RetirementProgress, RetiringGpuTransaction,
};
use crate::native_gpu_result::{
    EncodedIsfBatch, GpuLayerLease, NativeIsfCompletion, ValidatedLayerResult,
};
use crate::native_gpu_snapshot::{
    NativeGpuSnapshotter, NativeSnapshotGpu, NativeSnapshotMap, RetiringPremappingSnapshot,
    SharedPremappingSnapshot, SnapshotProgress,
};

/// The CPU pixels and their accounting owner have the same outer lifetime.
/// Until P provides a CPU-charge split, this conservatively retains the
/// premapping allocation too. Both old and new owners remain in G's one ledger.
pub(crate) struct SharedNativeOutputCache {
    cache: NativeOutputCache,
    snapshot_owner: Option<Arc<NativeSnapshotGpu>>,
}

impl SharedNativeOutputCache {
    pub(crate) fn new(output: VideoOutputId) -> Self {
        Self {
            cache: NativeOutputCache::new(output),
            snapshot_owner: None,
        }
    }

    pub(crate) fn begin_request(
        &mut self,
        key: VideoOutputLastValidKey,
        frame: NativeFrameId,
    ) -> Result<CachePublicationFence, CacheFenceError> {
        let result = self.cache.begin_request(key, frame);
        if self.cache.cached.is_none() {
            self.snapshot_owner = None;
        }
        result
    }

    pub(crate) fn last_valid(&self, key: &VideoOutputLastValidKey) -> Option<&VideoFrame> {
        self.cache.last_valid(key)
    }

    pub(crate) fn blackout(&mut self) -> Result<(), CacheFenceError> {
        let result = self.cache.blackout();
        // Pixels are destroyed before releasing their charge owner, on overflow too.
        self.snapshot_owner = None;
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
        drop(writer);
        match publication {
            Ok(bound) => {
                let (gpu, receipt) = bound.into_parts();
                let owner = Arc::new(gpu);
                // No callbacks/authority hooks occur between pixels publication
                // and this owner move. A later Stop leaves this exact pair alive.
                self.snapshot_owner = Some(Arc::clone(&owner));
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
    validated: Option<(StackBatch<ValidatedLayerResult>, IsfValidatedReceipt)>,
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
    reaped: bool,
}

impl SharedNativeOutputRetirement {
    pub(crate) fn reason(&self) -> Option<&OutputRejection> {
        self.reason.as_ref()
    }

    /// ISF history remains available even if composition/map/cache failed.
    /// Recovered CPU outcomes retain their exact typed faults, pixels and charges.
    pub(crate) fn take_isf_outcome(&mut self) -> Option<NativeIsfCompletion> {
        self.isf_outcome.take()
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
        if self.reaped {
            return Ok(true);
        }
        if let Some(reservation) = self.unsubmitted.take() {
            if let Err((error, reservation)) = owner.discard_unsubmitted_reservation(reservation) {
                self.unsubmitted = Some(reservation);
                return Err(error);
            }
            self.admitted = None;
        }
        owner.begin_stop();
        match owner.poll_retirement()? {
            RetirementProgress::Reaped => self.reaped = true,
            RetirementProgress::Pending {
                transactions: 0, ..
            } if owner.state() == GpuOwnerState::Retiring => {
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
        let validation = check_request(&request).and_then(|()| {
            if request.context != owner.context().id()
                || reservation.context() != owner.context().id()
                || reservation.frame() != request.frame
                || composition.context != request.context
                || composition.frame != request.frame
                || composition.extent != (request.key.width, request.key.height)
            {
                Err(OutputRejection::Fault(fault(
                    &request,
                    GpuPhase::Initialize,
                    NativeGpuFaultKind::IdentityMismatch,
                )))
            } else {
                Ok(())
            }
        });
        if let Err(reason) = validation {
            return Err(SharedNativeOutputRetirement {
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
                reaped: false,
            });
        }
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
        let encoded = match selected {
            Ok(selected) => compositor.encode_composition_and_snapshot(
                &mut txn,
                composition,
                &selected,
                p_reservation,
            ),
            Err(error) => Err(error),
        };
        let pending = txn.finish();
        let (snapshot, failed_snapshot, composition_failure) = match encoded {
            Ok(encoded) => {
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
            Err(error) => (None, None, Some(error)),
        };
        Ok(Self {
            request,
            transaction_id,
            pending: Some(pending),
            isf: Some(isf),
            snapshot,
            failed_snapshot,
            composition_failure,
            validated: None,
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
            reaped: false,
        }
    }

    fn retire(
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
        let transaction = self
            .pending
            .take()
            .map(|pending| owner.cancel_and_retire(pending));
        owner.begin_stop();
        let isf_outcome = self
            .validated
            .take()
            .map(|(layers, receipt)| NativeIsfCompletion::Validated { layers, receipt });
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
            reaped: false,
        }
    }

    pub(crate) fn poll(
        mut self,
        owner: &mut NativeGpuOwner,
        runtime: &mut IsfGpuRuntime,
        cache: &mut SharedNativeOutputCache,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
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
                        self.retire(owner, cancelled_or_fault(error, cancel)),
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
                        self.retire(owner, cancelled_or_fault(error, cancel)),
                    );
                }
            };
            match runtime.complete_native_batch(encoded, phase, owner, cancel) {
                NativeIsfCompletion::Validated { layers, receipt } => {
                    self.validated = Some((layers, receipt));
                }
                outcome => {
                    let reason = match &outcome {
                        NativeIsfCompletion::Cancelled { .. } => Some(OutputRejection::Cancelled),
                        NativeIsfCompletion::Failed { fault, .. } => {
                            Some(cancelled_or_fault(copy_fault(fault), cancel))
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
            if let Some(error) = self.composition_failure.take() {
                return SharedNativeOutputProgress::Retiring(
                    self.retire(owner, cancelled_or_fault(error, cancel)),
                );
            }
            if let Some(snapshot) = self.snapshot.as_mut() {
                if let Err(error) = snapshot.resolve_shared_phase(&mut resolved) {
                    return SharedNativeOutputProgress::Retiring(
                        self.retire(owner, cancelled_or_fault(error, cancel)),
                    );
                }
            }
        }
        let validation = guard(
            &self.request,
            OutputBoundary::PhasesResolved,
            authority,
            cancel,
        )
        .and_then(|()| self.validate_completion());
        if let Err(reason) = validation {
            return SharedNativeOutputProgress::Retiring(self.retire(owner, reason));
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
                        self.retire(owner, cancelled_or_fault(error, cancel)),
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
        let (layers, isf) = self
            .validated
            .take()
            .expect("ISF proof validated before cache");
        // P's completed map/pixels and real ISF proof were joined above. No
        // duplicated ResolvedGpuPhase or metadata-only success receipt is minted.
        SharedNativeOutputProgress::Ready {
            ready: ReadyNativeGpuOutput {
                gpu,
                request: self.request,
                snapshot: descriptor.snapshot,
                receipt,
                retry_remaining: true,
            },
            layers,
            isf,
        }
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
        let (_, receipt) = self.validated.as_ref().ok_or_else(mismatch)?;
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

#[cfg(test)]
#[path = "native_gpu_output_shared_tests.rs"]
mod tests;
