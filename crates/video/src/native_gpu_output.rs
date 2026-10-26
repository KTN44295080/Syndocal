//! I lane: output ownership and cache publication (INTERFACE-01 sections 6 and 9).
//! G's resolved ISF/composition receipts and P's copied/unmapped snapshot are
//! joined here before cache publication. Native routing stays test-only until C4.

use protocol::{VideoOutputId, VideoOutputMapping};

#[path = "native_gpu_output_cpu.rs"]
pub(crate) mod cpu;
#[path = "native_gpu_output_history.rs"]
pub(crate) mod history;
#[path = "native_gpu_output_shared.rs"]
pub(crate) mod shared;

use crate::native_gpu_context::ResolvedGpuPhase;
use crate::native_gpu_contract::*;
use crate::native_gpu_result::IsfValidatedReceipt;
use crate::native_gpu_snapshot::{CacheBoundSnapshot, CompletedPremappingSnapshot};
use crate::{
    VideoFrame, VideoOutputLastValidKey, VideoOutputMappingIdentity, VideoOutputRenderEvidence,
    VideoOutputRenderFreshness, VideoPixelFormat, VideoRenderCancellation,
};

#[derive(Clone, Debug)]
pub(crate) struct NativeOutputRequest {
    pub context: GpuContextId,
    pub frame: NativeFrameId,
    pub key: VideoOutputLastValidKey,
    pub fence: CachePublicationFence,
    pub pts_ms: u64,
    pub duration_ms: u64,
    pub mapping: VideoOutputMapping,
    pub mapping_identity: VideoOutputMappingIdentity,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OutputBoundary {
    BeforePreflight,
    BeforeCpuFallback,
    BeforeSubmit,
    AfterSubmit,
    BeforePoll,
    AfterPoll,
    PhasesResolved,
    SnapshotCopiedUnmapped,
    BeforeCacheCommit,
    AfterCacheCommit,
    BeforeReady,
    BeforePresentCall,
    BeforeAcquire,
    BeforePhysicalPresent,
}

/// A live facade over the existing native authority contract, not a saved bool.
/// Implementations compare current context/frame/key/fences and full mapping,
/// plus the route/owner/Follow identities that remain app-owned.
pub(crate) trait NativeOutputAuthority {
    fn revalidate(
        &self,
        request: &NativeOutputRequest,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection>;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum OutputRejection {
    Cancelled,
    Authority(PresentRejection),
    Cache(SnapshotCommitRejection),
    Fault(NativeGpuFault),
}

fn fault(
    request: &NativeOutputRequest,
    phase: GpuPhase,
    kind: NativeGpuFaultKind,
) -> NativeGpuFault {
    NativeGpuFault {
        context: request.context,
        frame: Some(request.frame),
        phase,
        kind,
        message: "native output contract mismatch".into(),
    }
}

fn copy_fault(value: &NativeGpuFault) -> NativeGpuFault {
    NativeGpuFault {
        context: value.context,
        frame: value.frame,
        phase: value.phase,
        kind: value.kind,
        message: value.message.clone(),
    }
}

fn cancelled_or_fault(
    error: NativeGpuFault,
    cancel: &dyn VideoRenderCancellation,
) -> OutputRejection {
    if cancel.is_cancelled() {
        OutputRejection::Cancelled
    } else {
        OutputRejection::Fault(error)
    }
}

fn cancelled_or_cache(
    reason: SnapshotCommitRejection,
    cancel: &dyn VideoRenderCancellation,
) -> OutputRejection {
    if cancel.is_cancelled() || reason == SnapshotCommitRejection::Cancelled {
        OutputRejection::Cancelled
    } else {
        OutputRejection::Cache(reason)
    }
}

fn packed_len(width: u32, height: u32) -> Option<usize> {
    u64::from(width)
        .checked_mul(u64::from(height))?
        .checked_mul(4)
        .and_then(|bytes| usize::try_from(bytes).ok())
}

fn check_request(request: &NativeOutputRequest) -> Result<(), OutputRejection> {
    if request.key.width == 0 || request.key.height == 0 {
        return Err(OutputRejection::Authority(PresentRejection::Suspended));
    }
    if request.key.output_id != request.frame.output_id
        || request.key.project_render_epoch != request.frame.project_epoch
        || request.fence.request_sequence != request.frame.request_sequence
        || packed_len(request.key.width, request.key.height).is_none()
    {
        return Err(OutputRejection::Fault(fault(
            request,
            GpuPhase::Initialize,
            NativeGpuFaultKind::IdentityMismatch,
        )));
    }
    Ok(())
}

fn guard(
    request: &NativeOutputRequest,
    boundary: OutputBoundary,
    authority: &impl NativeOutputAuthority,
    cancel: &dyn VideoRenderCancellation,
) -> Result<(), OutputRejection> {
    if cancel.is_cancelled() {
        return Err(OutputRejection::Cancelled);
    }
    if request.mapping_identity != VideoOutputMappingIdentity::from_mapping(&request.mapping) {
        return Err(OutputRejection::Authority(PresentRejection::StaleMapping));
    }
    let result = authority.revalidate(request, boundary);
    // Cancellation may arrive inside the authority/short-admission callback.
    if cancel.is_cancelled() {
        return Err(OutputRejection::Cancelled);
    }
    result.map_err(|reason| {
        if reason == PresentRejection::Cancelled {
            OutputRejection::Cancelled
        } else {
            OutputRejection::Authority(reason)
        }
    })
}

pub(crate) enum NativePreflight<I, A> {
    Admitted(A),
    Declined { inputs: I, reason: NativeGpuDecline },
    Failed { inputs: I, fault: NativeGpuFault },
}

pub(crate) enum CpuExecution<T> {
    Complete(T),
    Cancelled,
    Failed(NativeGpuFault),
}

pub(crate) struct SubmissionFailure<R> {
    pub fault: NativeGpuFault,
    pub retirement: R,
}

/// The backend retains strong owners in its registry even when a lookup ticket
/// is dropped. Whole-tick CPU execution is exposed only to the preflight route.
pub(crate) trait NativeOutputDriver:
    GpuCompletionPort<Completed = CompletedNativeOutput<Self::Gpu>>
{
    type Inputs;
    type Admitted;
    type Gpu;
    type Cpu;

    fn preflight(
        &mut self,
        inputs: Self::Inputs,
        request: &NativeOutputRequest,
    ) -> NativePreflight<Self::Inputs, Self::Admitted>;
    fn submit(
        &mut self,
        admitted: Self::Admitted,
        request: &NativeOutputRequest,
    ) -> Result<Self::Pending, SubmissionFailure<Self::Retiring>>;
    fn execute_cpu(
        &mut self,
        inputs: Self::Inputs,
        cancel: &dyn VideoRenderCancellation,
    ) -> CpuExecution<Self::Cpu>;
    fn retire_completed(&mut self, completed: CompletedNativeOutput<Self::Gpu>) -> Self::Retiring;
    fn retire_ready(&mut self, ready: ReadyNativeGpuOutput<Self::Gpu>) -> Self::Retiring;
    /// false/Err retains the ticket and its registry owner; neither is Stop ACK.
    fn poll_retirement(&mut self, retiring: &mut Self::Retiring) -> Result<bool, NativeGpuFault>;
}

pub(crate) struct PreparedNativeOutput<I> {
    request: NativeOutputRequest,
    inputs: I,
}

pub(crate) enum UnsubmittedOutput<D: NativeOutputDriver> {
    Inputs(D::Inputs),
    Admitted(D::Admitted),
}

pub(crate) enum BegunNativeOutput<D: NativeOutputDriver> {
    Native(PendingNativeGpuOutput<D::Pending>),
    Cpu {
        reason: NativeGpuDecline,
        result: CpuExecution<D::Cpu>,
    },
    Rejected {
        reason: OutputRejection,
        retained: UnsubmittedOutput<D>,
    },
    Retiring(RetiringNativeOutput<D::Retiring>),
}

impl<I> PreparedNativeOutput<I> {
    pub(crate) fn new(request: NativeOutputRequest, inputs: I) -> Self {
        Self { request, inputs }
    }

    /// Consuming Prepared makes a decline/CPU route single-use. Once admitted,
    /// no branch below calls execute_cpu, including submission/completion faults.
    pub(crate) fn begin<D: NativeOutputDriver<Inputs = I>>(
        self,
        driver: &mut D,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> BegunNativeOutput<D> {
        let Self { request, inputs } = self;
        let validation = if cancel.is_cancelled() {
            Err(OutputRejection::Cancelled)
        } else {
            check_request(&request)
                .and_then(|()| guard(&request, OutputBoundary::BeforePreflight, authority, cancel))
        };
        if let Err(reason) = validation {
            return BegunNativeOutput::Rejected {
                reason,
                retained: UnsubmittedOutput::Inputs(inputs),
            };
        }
        match driver.preflight(inputs, &request) {
            NativePreflight::Declined { inputs, reason } => {
                if let Err(rejection) = guard(
                    &request,
                    OutputBoundary::BeforeCpuFallback,
                    authority,
                    cancel,
                ) {
                    return BegunNativeOutput::Rejected {
                        reason: rejection,
                        retained: UnsubmittedOutput::Inputs(inputs),
                    };
                }
                let result = driver.execute_cpu(inputs, cancel);
                // The CPU adapter remains single-use even if Stop arrives while
                // it runs; a completed value cannot turn a late Stop into Fresh.
                let result = if cancel.is_cancelled() {
                    CpuExecution::Cancelled
                } else {
                    result
                };
                BegunNativeOutput::Cpu { reason, result }
            }
            NativePreflight::Failed { inputs, fault } => BegunNativeOutput::Rejected {
                reason: cancelled_or_fault(fault, cancel),
                retained: UnsubmittedOutput::Inputs(inputs),
            },
            NativePreflight::Admitted(admitted) => {
                if let Err(reason) =
                    guard(&request, OutputBoundary::BeforeSubmit, authority, cancel)
                {
                    return BegunNativeOutput::Rejected {
                        reason,
                        retained: UnsubmittedOutput::Admitted(admitted),
                    };
                }
                match driver.submit(admitted, &request) {
                    Err(error) => BegunNativeOutput::Retiring(RetiringNativeOutput::new(
                        cancelled_or_fault(error.fault, cancel),
                        error.retirement,
                    )),
                    Ok(pending) => {
                        if let Err(reason) =
                            guard(&request, OutputBoundary::AfterSubmit, authority, cancel)
                        {
                            return BegunNativeOutput::Retiring(RetiringNativeOutput::new(
                                reason,
                                driver.cancel_and_retire(pending),
                            ));
                        }
                        BegunNativeOutput::Native(PendingNativeGpuOutput { request, pending })
                    }
                }
            }
        }
    }
}

/// Each state retains the affine GPU owner. In particular, a failed P cache
/// handoff must not drop that owner before the driver's retirement path sees it.
enum OutputSnapshotOwnership<G> {
    Copied(CompletedPremappingSnapshot<G>),
    Bound(CacheBoundSnapshot<G>),
    Retained { _gpu: G },
}

pub(crate) struct CompletedNativeOutput<G> {
    isf: IsfValidatedReceipt,
    composition: ResolvedGpuPhase,
    descriptor: CacheCommitDescriptor,
    snapshot: Option<OutputSnapshotOwnership<G>>,
}

impl<G> CompletedNativeOutput<G> {
    /// Construction requires real G proofs and P's completed copy/unmap owner.
    /// The policy still validates their common transaction, frame, and binding.
    pub(crate) fn new(
        isf: IsfValidatedReceipt,
        composition: ResolvedGpuPhase,
        descriptor: CacheCommitDescriptor,
        snapshot: CompletedPremappingSnapshot<G>,
    ) -> Self {
        Self {
            isf,
            composition,
            descriptor,
            snapshot: Some(OutputSnapshotOwnership::Copied(snapshot)),
        }
    }

    pub(crate) fn descriptor(&self) -> &CacheCommitDescriptor {
        &self.descriptor
    }

    fn validate_phases(&self, request: &NativeOutputRequest) -> Result<(), NativeGpuFault> {
        let isf = self.isf.phase();
        for (resolved, expected_phase) in [
            (isf, GpuPhase::IsfBatch),
            (&self.composition, GpuPhase::CompositionSnapshot),
        ] {
            resolved.validation().map_err(copy_fault)?;
            if resolved.context() != request.context
                || resolved.phase() != expected_phase
                || !request.frame.history_scope().matches(resolved.frame())
                || resolved.frame().request_sequence != request.frame.request_sequence
            {
                return Err(fault(
                    request,
                    expected_phase,
                    NativeGpuFaultKind::IdentityMismatch,
                ));
            }
        }
        if isf.transaction() != self.composition.transaction()
            || isf.submission().sequence >= self.composition.submission().sequence
            || self.composition.submission() != self.descriptor.snapshot.completed_submission
        {
            return Err(fault(
                request,
                GpuPhase::CompositionSnapshot,
                NativeGpuFaultKind::IdentityMismatch,
            ));
        }
        Ok(())
    }

    fn validate_snapshot(&self, request: &NativeOutputRequest) -> Result<(), NativeGpuFault> {
        let identity = self.descriptor.snapshot;
        if self.descriptor.key != request.key
            || self.descriptor.fence != request.fence
            || identity.frame != request.frame
            || identity.allocation.context != request.context
            || identity.completed_submission.context != request.context
            || identity.width != request.key.width
            || identity.height != request.key.height
            || identity.pts_ms != request.pts_ms
            || identity.duration_ms != request.duration_ms
            || identity.format != VideoPixelFormat::Rgba8
            || packed_len(identity.width, identity.height).is_none()
        {
            return Err(fault(
                request,
                GpuPhase::CompositionSnapshot,
                NativeGpuFaultKind::IdentityMismatch,
            ));
        }
        Ok(())
    }

    /// This is the only Ready constructor. A copied snapshot alone cannot enter
    /// it: P must already have bound the same snapshot to an actual cache receipt.
    fn into_ready(
        mut self,
        request: NativeOutputRequest,
    ) -> Result<ReadyNativeGpuOutput<G>, (OutputRejection, Self)> {
        if let Err(error) = self
            .validate_phases(&request)
            .and_then(|()| self.validate_snapshot(&request))
        {
            return Err((OutputRejection::Fault(error), self));
        }
        if !matches!(self.snapshot, Some(OutputSnapshotOwnership::Bound(_))) {
            return Err((
                OutputRejection::Fault(fault(
                    &request,
                    GpuPhase::CompositionSnapshot,
                    NativeGpuFaultKind::StateMismatch,
                )),
                self,
            ));
        }
        let Some(OutputSnapshotOwnership::Bound(bound)) = self.snapshot.take() else {
            unreachable!("bound ownership checked before Ready handoff");
        };
        let (gpu, receipt) = bound.into_parts();
        if receipt.descriptor() != &self.descriptor {
            self.snapshot = Some(OutputSnapshotOwnership::Retained { _gpu: gpu });
            return Err((
                OutputRejection::Fault(fault(
                    &request,
                    GpuPhase::CompositionSnapshot,
                    NativeGpuFaultKind::IdentityMismatch,
                )),
                self,
            ));
        }
        Ok(ReadyNativeGpuOutput {
            gpu,
            snapshot: self.descriptor.snapshot,
            request,
            receipt,
            retry_remaining: true,
        })
    }
}

/// Only the output policy creates this after all phase/copy/identity checks.
/// The cache adapter must still validate live fences at the mutation point.
pub(crate) struct ValidatedSnapshotBinding {
    descriptor: CacheCommitDescriptor,
}

pub(crate) trait BoundSnapshotCache: CpuSnapshotCommit {
    fn bind_validated(
        &mut self,
        binding: ValidatedSnapshotBinding,
    ) -> Result<(), SnapshotCommitRejection>;
    /// Clear a one-attempt binding even if P rejects before calling commit.
    fn discard_binding(&mut self);
}

pub(crate) struct PendingNativeGpuOutput<P> {
    request: NativeOutputRequest,
    pending: P,
}

pub(crate) enum PolledNativeOutput<P, G, R> {
    Pending(PendingNativeGpuOutput<P>),
    Ready(ReadyNativeGpuOutput<G>),
    Retiring(RetiringNativeOutput<R>),
}

impl<P> PendingNativeGpuOutput<P> {
    /// Consuming each poll prevents a terminal pending from being polled twice.
    pub(crate) fn poll<D: NativeOutputDriver<Pending = P>>(
        mut self,
        driver: &mut D,
        cache: &mut impl BoundSnapshotCache,
        authority: &impl NativeOutputAuthority,
        cancel: &dyn VideoRenderCancellation,
    ) -> PolledNativeOutput<P, D::Gpu, D::Retiring> {
        if let Err(reason) = guard(&self.request, OutputBoundary::BeforePoll, authority, cancel) {
            return PolledNativeOutput::Retiring(RetiringNativeOutput::new(
                reason,
                driver.cancel_and_retire(self.pending),
            ));
        }
        let progress = driver.poll_completion(&mut self.pending, cancel);
        match progress {
            CompletionProgress::Pending => {
                if let Err(reason) =
                    guard(&self.request, OutputBoundary::AfterPoll, authority, cancel)
                {
                    PolledNativeOutput::Retiring(RetiringNativeOutput::new(
                        reason,
                        driver.cancel_and_retire(self.pending),
                    ))
                } else {
                    PolledNativeOutput::Pending(self)
                }
            }
            CompletionProgress::Cancelled => {
                PolledNativeOutput::Retiring(RetiringNativeOutput::new(
                    OutputRejection::Cancelled,
                    driver.cancel_and_retire(self.pending),
                ))
            }
            CompletionProgress::Complete(Err(error)) => {
                PolledNativeOutput::Retiring(RetiringNativeOutput::new(
                    cancelled_or_fault(error, cancel),
                    driver.cancel_and_retire(self.pending),
                ))
            }
            CompletionProgress::Complete(Ok(mut completed)) => {
                // Completed now owns the resources. Never retire the emptied
                // pending as a substitute for retaining this payload.
                drop(self.pending);
                let result =
                    complete_and_commit(&self.request, &mut completed, cache, authority, cancel);
                match result {
                    Ok(()) => match completed.into_ready(self.request) {
                        Ok(ready) => PolledNativeOutput::Ready(ready),
                        Err((reason, completed)) => PolledNativeOutput::Retiring(
                            RetiringNativeOutput::new(reason, driver.retire_completed(completed)),
                        ),
                    },
                    Err(reason) => PolledNativeOutput::Retiring(RetiringNativeOutput::new(
                        reason,
                        driver.retire_completed(completed),
                    )),
                }
            }
        }
    }
}

fn complete_and_commit<G>(
    request: &NativeOutputRequest,
    completed: &mut CompletedNativeOutput<G>,
    cache: &mut impl BoundSnapshotCache,
    authority: &impl NativeOutputAuthority,
    cancel: &dyn VideoRenderCancellation,
) -> Result<(), OutputRejection> {
    guard(request, OutputBoundary::AfterPoll, authority, cancel)?;
    completed
        .validate_phases(request)
        .map_err(|error| cancelled_or_fault(error, cancel))?;
    guard(request, OutputBoundary::PhasesResolved, authority, cancel)?;
    if !matches!(completed.snapshot, Some(OutputSnapshotOwnership::Copied(_))) {
        return Err(OutputRejection::Fault(fault(
            request,
            GpuPhase::CompositionSnapshot,
            NativeGpuFaultKind::StateMismatch,
        )));
    }
    guard(
        request,
        OutputBoundary::SnapshotCopiedUnmapped,
        authority,
        cancel,
    )?;
    completed
        .validate_snapshot(request)
        .map_err(|error| cancelled_or_fault(error, cancel))?;
    guard(
        request,
        OutputBoundary::BeforeCacheCommit,
        authority,
        cancel,
    )?;
    if let Err(reason) = cache.bind_validated(ValidatedSnapshotBinding {
        descriptor: completed.descriptor.clone(),
    }) {
        cache.discard_binding();
        return Err(cancelled_or_cache(reason, cancel));
    }
    let Some(OutputSnapshotOwnership::Copied(snapshot)) = completed.snapshot.take() else {
        unreachable!("copy ownership checked before binding");
    };
    let publication = snapshot.commit(cache, &completed.descriptor, cancel);
    // P can reject the current descriptor or Stop without invoking cache.commit.
    // A one-attempt binding must never survive to a later completion.
    cache.discard_binding();
    match publication {
        Ok(bound) => completed.snapshot = Some(OutputSnapshotOwnership::Bound(bound)),
        Err(failure) => {
            completed.snapshot = Some(OutputSnapshotOwnership::Retained { _gpu: failure.gpu });
            return Err(cancelled_or_cache(failure.reason, cancel));
        }
    }
    guard(request, OutputBoundary::AfterCacheCommit, authority, cancel)?;
    guard(request, OutputBoundary::BeforeReady, authority, cancel)?;
    Ok(())
}

/// Non-Clone. There is no public/test constructor from Pending or metadata.
pub(crate) struct ReadyNativeGpuOutput<G> {
    gpu: G,
    request: NativeOutputRequest,
    snapshot: SnapshotIdentity,
    receipt: CacheCommitReceipt,
    retry_remaining: bool,
}

impl<G> ReadyNativeGpuOutput<G> {
    pub(crate) fn payload(&self) -> &G {
        &self.gpu
    }
    pub(crate) fn snapshot_identity(&self) -> SnapshotIdentity {
        self.snapshot
    }
    pub(crate) fn cache_receipt(&self) -> &CacheCommitReceipt {
        &self.receipt
    }
    /// The exact mapping frozen with this output request, before submission.
    pub(crate) fn mapping(&self) -> &VideoOutputMapping {
        &self.request.mapping
    }
    pub(crate) fn evidence(&self) -> VideoOutputRenderEvidence {
        VideoOutputRenderEvidence {
            project_render_epoch: self.request.frame.project_epoch,
            output_id: self.request.frame.output_id,
            freshness: VideoOutputRenderFreshness::Fresh,
            error: None,
        }
    }
}

impl crate::gpu_surface::native::FrozenPresentation
    for ReadyNativeGpuOutput<std::sync::Arc<crate::native_gpu_snapshot::NativeSnapshotGpu>>
{
    fn cache_receipt(&self) -> &CacheCommitReceipt {
        ReadyNativeGpuOutput::cache_receipt(self)
    }

    fn snapshot_gpu(&self) -> &crate::native_gpu_snapshot::NativeSnapshotGpu {
        self.payload().as_ref()
    }

    fn mapping(&self) -> &VideoOutputMapping {
        ReadyNativeGpuOutput::mapping(self)
    }
}

pub(crate) struct RetiringNativeOutput<R> {
    reason: OutputRejection,
    ticket: Option<R>,
}

impl<R> RetiringNativeOutput<R> {
    fn new(reason: OutputRejection, ticket: R) -> Self {
        Self {
            reason,
            ticket: Some(ticket),
        }
    }
    pub(crate) fn reason(&self) -> &OutputRejection {
        &self.reason
    }
    pub(crate) fn is_retired(&self) -> bool {
        self.ticket.is_none()
    }
    pub(crate) fn poll<D: NativeOutputDriver<Retiring = R>>(
        &mut self,
        driver: &mut D,
    ) -> Result<bool, NativeGpuFault> {
        self.poll_with(|ticket| driver.poll_retirement(ticket))
    }

    /// Poll a concrete retained owner after the present sink's broker borrow
    /// ended. A false result or fault leaves the entire ticket in this owner.
    pub(crate) fn poll_with(
        &mut self,
        poll: impl FnOnce(&mut R) -> Result<bool, NativeGpuFault>,
    ) -> Result<bool, NativeGpuFault> {
        let Some(ticket) = self.ticket.as_mut() else {
            return Ok(true);
        };
        if poll(ticket)? {
            self.ticket = None;
        }
        Ok(self.is_retired())
    }
}

/// Borrowed for one sink call, and revalidatable inside that call.
pub(crate) struct NativePresentPermit<'a, A> {
    authority: &'a A,
    cancel: &'a dyn VideoRenderCancellation,
}

impl<A: NativeOutputAuthority> NativePresentPermit<'_, A> {
    pub(crate) fn before_acquire<G>(
        &self,
        ready: &ReadyNativeGpuOutput<G>,
    ) -> Result<(), PresentRejection> {
        self.check(ready, OutputBoundary::BeforeAcquire)
    }
    pub(crate) fn before_present<G>(
        &self,
        ready: &ReadyNativeGpuOutput<G>,
    ) -> Result<(), PresentRejection> {
        self.check(ready, OutputBoundary::BeforePhysicalPresent)
    }
    fn check<G>(
        &self,
        ready: &ReadyNativeGpuOutput<G>,
        boundary: OutputBoundary,
    ) -> Result<(), PresentRejection> {
        guard(&ready.request, boundary, self.authority, self.cancel).map_err(
            |reason| match reason {
                OutputRejection::Cancelled => PresentRejection::Cancelled,
                OutputRejection::Authority(reason) => reason,
                _ => PresentRejection::Revoked,
            },
        )
    }
}

#[derive(Default)]
pub(crate) struct NativeDeliveryCounter {
    delivered: u64,
}
impl NativeDeliveryCounter {
    pub(crate) fn delivered(&self) -> u64 {
        self.delivered
    }
}

/// Only Presented can construct this delivery/Follow evidence. It does not
/// certify third-submission completion; the sink retains that registry owner.
pub(crate) struct NativeDelivery<R> {
    frame: NativeFrameId,
    receipt: R,
}
impl<R> NativeDelivery<R> {
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    pub(crate) fn receipt(&self) -> &R {
        &self.receipt
    }
}

pub(crate) enum NativePresentation<G, R, P> {
    Presented(NativeDelivery<P>),
    Retry(ReadyNativeGpuOutput<G>),
    Retiring(RetiringNativeOutput<R>),
    /// The consuming sink, not this reason, owns submitted resources.
    NotPresented(PresentRejection),
    Failed(NativeGpuFault),
}

pub(crate) fn present_native_output<'a, D, S, A>(
    ready: ReadyNativeGpuOutput<D::Gpu>,
    driver: &mut D,
    sink: &mut S,
    authority: &'a A,
    cancel: &'a dyn VideoRenderCancellation,
    delivery: &mut NativeDeliveryCounter,
) -> NativePresentation<D::Gpu, D::Retiring, S::Receipt>
where
    D: NativeOutputDriver,
    A: NativeOutputAuthority,
    S: NativePresentSink<Ready = ReadyNativeGpuOutput<D::Gpu>, Permit = NativePresentPermit<'a, A>>,
{
    present_native_output_with_retirement(
        ready,
        |ready| driver.retire_ready(ready),
        sink,
        authority,
        cancel,
        delivery,
    )
}

/// Canonical present policy for an already validated Ready. Retirement only
/// transfers its whole owner; it need not borrow the GPU broker while P's
/// consuming sink holds that broker. Poll the returned retirement afterward.
pub(crate) fn present_native_output_with_retirement<'a, G, R, S, A>(
    ready: ReadyNativeGpuOutput<G>,
    mut retire_ready: impl FnMut(ReadyNativeGpuOutput<G>) -> R,
    sink: &mut S,
    authority: &'a A,
    cancel: &'a dyn VideoRenderCancellation,
    delivery: &mut NativeDeliveryCounter,
) -> NativePresentation<G, R, S::Receipt>
where
    A: NativeOutputAuthority,
    S: NativePresentSink<Ready = ReadyNativeGpuOutput<G>, Permit = NativePresentPermit<'a, A>>,
{
    let next = delivery.delivered.checked_add(1);
    let reason = guard(
        &ready.request,
        OutputBoundary::BeforePresentCall,
        authority,
        cancel,
    )
    .err()
    .or_else(|| {
        next.is_none().then(|| {
            OutputRejection::Fault(fault(
                &ready.request,
                GpuPhase::MappingPresent,
                NativeGpuFaultKind::SequenceExhausted,
            ))
        })
    });
    if let Some(reason) = reason {
        return NativePresentation::Retiring(RetiringNativeOutput::new(
            reason,
            retire_ready(ready),
        ));
    }
    let frame = ready.request.frame;
    let expected_context = ready.request.context;
    let expected_snapshot = ready.snapshot;
    let expected_descriptor = ready.receipt.descriptor().clone();
    let expected_mapping = ready.request.mapping_identity;
    let retry_remaining = ready.retry_remaining;
    match sink.present_once(ready, NativePresentPermit { authority, cancel }) {
        Ok(PresentOutcome::Presented(receipt)) => {
            delivery.delivered = next.expect("delivery counter checked before submission");
            NativePresentation::Presented(NativeDelivery { frame, receipt })
        }
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: Some(mut ready),
        }) => {
            if cancel.is_cancelled() || reason == PresentRejection::Cancelled {
                return NativePresentation::Retiring(RetiringNativeOutput::new(
                    OutputRejection::Cancelled,
                    retire_ready(ready),
                ));
            }
            if ready.request.frame != frame
                || ready.snapshot != expected_snapshot
                || ready.request.context != expected_context
                || ready.request.key != expected_descriptor.key
                || ready.request.fence != expected_descriptor.fence
                || ready.request.pts_ms != expected_snapshot.pts_ms
                || ready.request.duration_ms != expected_snapshot.duration_ms
                || ready.request.mapping_identity != expected_mapping
                || VideoOutputMappingIdentity::from_mapping(&ready.request.mapping)
                    != expected_mapping
                || ready.receipt.descriptor() != &expected_descriptor
            {
                let error = NativeGpuFault {
                    context: expected_context,
                    frame: Some(frame),
                    phase: GpuPhase::MappingPresent,
                    kind: NativeGpuFaultKind::IdentityMismatch,
                    message: "native output retry identity mismatch".into(),
                };
                return NativePresentation::Retiring(RetiringNativeOutput::new(
                    OutputRejection::Fault(error),
                    retire_ready(ready),
                ));
            }
            if reason == PresentRejection::SurfaceLostOrOutdated && retry_remaining {
                ready.retry_remaining = false;
                NativePresentation::Retry(ready)
            } else {
                NativePresentation::Retiring(RetiringNativeOutput::new(
                    OutputRejection::Authority(reason),
                    retire_ready(ready),
                ))
            }
        }
        Ok(PresentOutcome::NotPresented {
            reason,
            retry: None,
        }) => NativePresentation::NotPresented(if cancel.is_cancelled() {
            PresentRejection::Cancelled
        } else {
            reason
        }),
        Err(error) => {
            // The sink still owns submitted resources. A late Stop is not a
            // retirement ACK, and only an actual Presented bypasses this rule.
            if cancel.is_cancelled() {
                NativePresentation::NotPresented(PresentRejection::Cancelled)
            } else {
                NativePresentation::Failed(error)
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CacheFenceError {
    Rejected(SnapshotCommitRejection),
    SequenceExhausted,
}

/// CPU publication policy, independent of a GPU backend. Its one-attempt writer
/// mints the shared receipt only after commit_bound moves pixels into this cache.
pub(crate) struct NativeOutputCache {
    output_id: VideoOutputId,
    target: Option<(VideoOutputLastValidKey, NativeFrameId)>,
    fence: CachePublicationFence,
    last_request: Option<u64>,
    published_request: Option<u64>,
    cached: Option<(VideoOutputLastValidKey, VideoFrame)>,
    poisoned: bool,
}

pub(crate) struct PublishedSnapshot {
    descriptor: CacheCommitDescriptor,
}
impl PublishedSnapshot {
    pub(crate) fn into_descriptor(self) -> CacheCommitDescriptor {
        self.descriptor
    }
}

/// Short mutable borrow of the actual cache. It cannot accept unbound metadata
/// or carry a previous attempt's binding across a failed snapshot handoff.
pub(crate) struct NativeCacheWriter<'a> {
    cache: &'a mut NativeOutputCache,
    cancel: &'a dyn VideoRenderCancellation,
    binding: Option<ValidatedSnapshotBinding>,
}

impl BoundSnapshotCache for NativeCacheWriter<'_> {
    fn bind_validated(
        &mut self,
        binding: ValidatedSnapshotBinding,
    ) -> Result<(), SnapshotCommitRejection> {
        if self.binding.take().is_some() {
            return Err(SnapshotCommitRejection::SnapshotIdentityMismatch);
        }
        self.binding = Some(binding);
        Ok(())
    }

    fn discard_binding(&mut self) {
        self.binding = None;
    }
}

impl CpuSnapshotCommit for NativeCacheWriter<'_> {
    fn commit(
        &mut self,
        key: &VideoOutputLastValidKey,
        frame: NativeFrameId,
        snapshot: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection> {
        let binding = self
            .binding
            .take()
            .ok_or(SnapshotCommitRejection::SnapshotIdentityMismatch)?;
        self.cache
            .commit_bound(binding, key, frame, snapshot, fence, self.cancel)
            .map(CacheCommitReceipt::from_publication)
    }

    fn invalidate_output(&mut self, output_id: VideoOutputId) {
        if self.cache.output_id == output_id {
            self.discard_binding();
            // Overflow also clears/revokes the target and poisons future writes.
            // This trait has no success receipt to misreport on that path.
            let _ = self.cache.blackout();
        }
    }
}

impl NativeOutputCache {
    pub(crate) fn writer<'a>(
        &'a mut self,
        cancel: &'a dyn VideoRenderCancellation,
    ) -> NativeCacheWriter<'a> {
        NativeCacheWriter {
            cache: self,
            cancel,
            binding: None,
        }
    }

    pub(crate) fn new(output_id: VideoOutputId) -> Self {
        Self {
            output_id,
            target: None,
            fence: CachePublicationFence {
                request_sequence: 0,
                blackout_generation: 0,
                configuration_generation: 0,
            },
            last_request: None,
            published_request: None,
            cached: None,
            poisoned: false,
        }
    }

    pub(crate) fn begin_request(
        &mut self,
        key: VideoOutputLastValidKey,
        frame: NativeFrameId,
    ) -> Result<CachePublicationFence, CacheFenceError> {
        if self.poisoned {
            return Err(CacheFenceError::SequenceExhausted);
        }
        if key.output_id != self.output_id
            || frame.output_id != self.output_id
            || key.project_render_epoch != frame.project_epoch
            || key.width == 0
            || key.height == 0
            || packed_len(key.width, key.height).is_none()
        {
            return Err(CacheFenceError::Rejected(
                SnapshotCommitRejection::KeyMismatch,
            ));
        }
        if self
            .last_request
            .is_some_and(|sequence| frame.request_sequence <= sequence)
        {
            return Err(CacheFenceError::Rejected(
                SnapshotCommitRejection::StaleRequest,
            ));
        }
        let configuration_changed = self.target.as_ref().is_some_and(|(previous, old_frame)| {
            previous != &key || old_frame.geometry_generation != frame.geometry_generation
        });
        if configuration_changed {
            self.invalidate_configuration()?;
        }
        // LastValid is premapping. Fence-only/mapping changes keep exact-key
        // pixels, whereas a changed full CPU key invalidates the old cache.
        if self
            .cached
            .as_ref()
            .is_some_and(|(cached_key, _)| cached_key != &key)
        {
            self.cached = None;
        }
        self.last_request = Some(frame.request_sequence);
        self.fence.request_sequence = frame.request_sequence;
        self.target = Some((key, frame));
        Ok(self.fence)
    }

    fn invalidate(&mut self, blackout: bool) -> Result<(), CacheFenceError> {
        self.target = None;
        if blackout {
            self.cached = None;
        }
        let generation = if blackout {
            &mut self.fence.blackout_generation
        } else {
            &mut self.fence.configuration_generation
        };
        match generation.checked_add(1) {
            Some(next) if !self.poisoned => {
                *generation = next;
                Ok(())
            }
            _ => {
                self.poisoned = true;
                Err(CacheFenceError::SequenceExhausted)
            }
        }
    }

    pub(crate) fn blackout(&mut self) -> Result<(), CacheFenceError> {
        self.invalidate(true)
    }
    /// Revoke pending publications without deleting successful premapping
    /// pixels. The next begin_request enforces the unchanged full CPU key.
    pub(crate) fn invalidate_configuration(&mut self) -> Result<(), CacheFenceError> {
        self.invalidate(false)
    }
    pub(crate) fn last_valid(&self, key: &VideoOutputLastValidKey) -> Option<&VideoFrame> {
        self.cached
            .as_ref()
            .and_then(|(cached_key, frame)| (cached_key == key).then_some(frame))
    }
    /// Owned CPU APIs retain their independent-pixels contract. The native
    /// success path never calls this cloning accessor.
    pub(crate) fn owned_last_valid(&self, key: &VideoOutputLastValidKey) -> Option<VideoFrame> {
        self.last_valid(key).cloned()
    }

    pub(crate) fn commit_bound(
        &mut self,
        binding: ValidatedSnapshotBinding,
        key: &VideoOutputLastValidKey,
        frame: NativeFrameId,
        pixels: VideoFrame,
        fence: CachePublicationFence,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<PublishedSnapshot, SnapshotCommitRejection> {
        if cancel.is_cancelled() {
            return Err(SnapshotCommitRejection::Cancelled);
        }
        if self.poisoned {
            return Err(SnapshotCommitRejection::ConfigurationChanged);
        }
        if fence.blackout_generation != self.fence.blackout_generation {
            return Err(SnapshotCommitRejection::BlackoutFence);
        }
        if fence.configuration_generation != self.fence.configuration_generation {
            return Err(SnapshotCommitRejection::ConfigurationChanged);
        }
        let (current_key, current_frame) = self
            .target
            .as_ref()
            .ok_or(SnapshotCommitRejection::StaleRequest)?;
        if frame.project_epoch != current_frame.project_epoch {
            return Err(SnapshotCommitRejection::StaleEpoch);
        }
        if fence.request_sequence != self.fence.request_sequence
            || frame.request_sequence != current_frame.request_sequence
            || self
                .published_request
                .is_some_and(|sequence| frame.request_sequence <= sequence)
        {
            return Err(SnapshotCommitRejection::StaleRequest);
        }
        if key != current_key {
            return Err(SnapshotCommitRejection::KeyMismatch);
        }
        let descriptor = binding.descriptor;
        let image = descriptor.snapshot;
        if descriptor.key != *key
            || descriptor.fence != fence
            || image.frame != frame
            || frame != *current_frame
            || image.width != key.width
            || image.height != key.height
            || image.format != VideoPixelFormat::Rgba8
            || image.allocation.context != image.completed_submission.context
            || pixels.layer_id != 0
            || pixels.width != image.width
            || pixels.height != image.height
            || pixels.pts_ms != image.pts_ms
            || pixels.duration_ms != image.duration_ms
            || pixels.format != image.format
            || Some(pixels.data.len()) != packed_len(image.width, image.height)
        {
            return Err(SnapshotCommitRejection::SnapshotIdentityMismatch);
        }
        // Last cancellation check is the cache publication linearization point.
        // Adapter hooks must execute before this point, never after it.
        if cancel.is_cancelled() {
            return Err(SnapshotCommitRejection::Cancelled);
        }
        self.cached = Some((key.clone(), pixels));
        self.published_request = Some(frame.request_sequence);
        Ok(PublishedSnapshot { descriptor })
    }
}

#[cfg(test)]
#[path = "native_gpu_output_tests.rs"]
mod tests;
