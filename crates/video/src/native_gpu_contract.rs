//! C0 shared vocabulary for INTERFACE-01 v1.
//!
//! This module is test-gated until the G/P/I production consumers are integrated.
//! It contains no GPU backend, cache implementation, or successful production
//! stub. IDs and descriptors are metadata, never evidence of GPU completion.
//! Proof/receipt construction below is explicitly test-only even after the
//! module gate is removed. Production minting must require validated owners.

use protocol::{VideoLayerId, VideoOutputId, VideoRenderInputKey};

use crate::{VideoFrame, VideoOutputLastValidKey, VideoPixelFormat, VideoRenderCancellation};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GpuContextId {
    pub owner_id: u64,
    pub device_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct NativeFrameId {
    pub output_id: VideoOutputId,
    pub project_epoch: u64,
    pub request_sequence: u64,
    pub geometry_generation: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct HistoryEntryId {
    pub input: VideoRenderInputKey,
    pub queue_epoch: u64,
    pub sequence: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GpuSubmissionId {
    pub context: GpuContextId,
    pub sequence: u64,
}

/// Context-qualified view of G's allocation identity, not a second allocator.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct GpuAllocationId {
    pub context: GpuContextId,
    pub sequence: u64,
}

/// Only FRAMEINDEX wraps (u32); owner/request/queue sequences are checked u64s.
/// Floating-point bits preserve the original TIME/TIMEDELTA/DATE on replay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct FrozenChainClocks {
    pub frame_index: u32,
    pub time_seconds_bits: u32,
    pub time_delta_seconds_bits: u32,
    pub date_bits: [u32; 4],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LayerOrigin {
    Executed(FrozenChainClocks),
    /// Existing post-FX CPU history has no invented FRAMEINDEX.
    ImportedCpu,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct LayerResultIdentity {
    pub context: GpuContextId,
    pub frame: NativeFrameId,
    pub entry: HistoryEntryId,
    pub layer_id: VideoLayerId,
    pub source_generation: u64,
    pub content_sequence: u64,
    pub pts_ms: u64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub format: VideoPixelFormat,
    pub origin: LayerOrigin,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuPhase {
    Initialize,
    IsfBatch,
    HistoricalReplay,
    CompositionSnapshot,
    MappingPresent,
    Retirement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GpuOwnerState {
    Active,
    Retiring,
    DeviceLostRetained,
    PoisonedRetained,
    Reaped,
}

/// Eligibility reasons before admission; none imply a runtime GPU fault.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum UnsupportedNativeFx {
    EmptyChain,
    UnsupportedJobCount,
    NonLayerScope,
    ClipOrTransitionExecution,
    InterLayerDependency,
    Callback,
    PersistentTarget,
    AuxiliaryResource,
    Multipass,
    CustomTarget,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeGpuDecline {
    UnsupportedChain {
        reason: UnsupportedNativeFx,
    },
    UnsupportedComposition,
    UnsupportedFormat,
    UnsupportedExtent,
    UnknownUploadPitch,
    BudgetExceeded {
        live: u64,
        requested: u64,
        limit: u64,
    },
    QuoteExpired,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum NativeGpuFaultKind {
    DeviceLost,
    OutOfMemory,
    Validation,
    Submission,
    Map,
    SnapshotCopy,
    ScopeDrain,
    HistoricalMaterialization { entry: HistoryEntryId },
    IdentityMismatch,
    StateMismatch,
    SequenceExhausted,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) struct NativeGpuFault {
    pub context: GpuContextId,
    pub frame: Option<NativeFrameId>,
    pub phase: GpuPhase,
    pub kind: NativeGpuFaultKind,
    /// Diagnostic text only; adapters must branch on typed kind/phase.
    pub message: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum PresentRejection {
    Cancelled,
    Revoked,
    StaleGeometry,
    StaleMapping,
    SurfaceTimeout,
    SurfaceLostOrOutdated,
    Suspended,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SnapshotCommitRejection {
    Cancelled,
    StaleRequest,
    StaleEpoch,
    BlackoutFence,
    ConfigurationChanged,
    KeyMismatch,
    SnapshotIdentityMismatch,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CompletionProgress<T, E> {
    Pending,
    Cancelled,
    Complete(Result<T, E>),
}

/// Lookup identity only. The retirement registry must retain the strong owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) struct RetirementId {
    pub context: GpuContextId,
    pub sequence: u64,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum MaterializationFailure {
    Cancelled {
        retirement: Option<RetirementId>,
    },
    Fault {
        fault: NativeGpuFault,
        retirement: Option<RetirementId>,
    },
}

/// A terminal poll must not be repeated. On Cancelled/fault the pending owner
/// stays intact until explicitly consumed into retirement; no detached cleanup.
pub(crate) trait GpuCompletionPort {
    type Pending;
    type Completed;
    type Retiring;

    fn poll_completion(
        &mut self,
        pending: &mut Self::Pending,
        cancel: &dyn VideoRenderCancellation,
    ) -> CompletionProgress<Self::Completed, NativeGpuFault>;

    fn cancel_and_retire(&mut self, pending: Self::Pending) -> Self::Retiring;
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum PresentOutcome<R, P> {
    Presented(P),
    NotPresented {
        reason: PresentRejection,
        retry: Option<R>,
    },
}

/// Ready/Permit are consumed. Only Presented is delivery evidence, not GPU
/// completion; the implementation must separately retain submitted resources.
pub(crate) trait NativePresentSink {
    type Ready;
    type Permit;
    type Receipt;

    fn present_once(
        &mut self,
        ready: Self::Ready,
        permit: Self::Permit,
    ) -> Result<PresentOutcome<Self::Ready, Self::Receipt>, NativeGpuFault>;
}

/// Read-only matching data, not an unforgeable completion token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct MaterializationDescriptor {
    pub layer: LayerResultIdentity,
    /// Source post-FX GPU allocation, not the readback buffer or CPU Vec.
    pub allocation: GpuAllocationId,
    /// Validated completion covering the replay/readback in this context.
    pub completed_submission: GpuSubmissionId,
}

/// Affine proof. G will mint production proofs only from validated completion.
#[derive(Debug)]
pub(crate) struct MaterializationProof {
    descriptor: MaterializationDescriptor,
}

impl MaterializationProof {
    pub(crate) fn descriptor(&self) -> &MaterializationDescriptor {
        &self.descriptor
    }

    /// Simulated evidence for contract tests; performs no GPU validation.
    #[cfg(test)]
    pub(crate) fn for_test(descriptor: MaterializationDescriptor) -> Self {
        Self { descriptor }
    }
}

/// Pixels and proof cannot be independently replaced while paired.
#[derive(Debug)]
pub(crate) struct ExactCpuLayer {
    frame: VideoFrame,
    proof: MaterializationProof,
}

impl ExactCpuLayer {
    pub(crate) fn frame(&self) -> &VideoFrame {
        &self.frame
    }

    pub(crate) fn proof(&self) -> &MaterializationProof {
        &self.proof
    }

    pub(crate) fn into_parts(self) -> (VideoFrame, MaterializationProof) {
        (self.frame, self.proof)
    }

    /// Test fixture only, including intentionally mismatched negative fixtures.
    #[cfg(test)]
    pub(crate) fn for_test(frame: VideoFrame, proof: MaterializationProof) -> Self {
        Self { frame, proof }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CachePublicationFence {
    pub request_sequence: u64,
    pub blackout_generation: u64,
    pub configuration_generation: u64,
}

/// Identity of one premapping snapshot; it does not prove copy/unmap success.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SnapshotIdentity {
    pub frame: NativeFrameId,
    /// Premapping GPU composition source, not the map buffer or CPU Vec.
    pub allocation: GpuAllocationId,
    /// Validated composition/snapshot submission, not the later present.
    pub completed_submission: GpuSubmissionId,
    pub pts_ms: u64,
    pub duration_ms: u64,
    pub width: u32,
    pub height: u32,
    pub format: VideoPixelFormat,
}

/// The existing CPU key is reused unchanged; output mapping is not part of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CacheCommitDescriptor {
    pub key: VideoOutputLastValidKey,
    pub snapshot: SnapshotIdentity,
    pub fence: CachePublicationFence,
}

/// Affine cache publication receipt, not constructible in production C0.
#[derive(Debug)]
pub(crate) struct CacheCommitReceipt {
    descriptor: CacheCommitDescriptor,
}

impl CacheCommitReceipt {
    pub(crate) fn descriptor(&self) -> &CacheCommitDescriptor {
        &self.descriptor
    }

    /// Simulated cache publication for tests, never a production success path.
    #[cfg(test)]
    pub(crate) fn for_test(descriptor: CacheCommitDescriptor) -> Self {
        Self { descriptor }
    }
}

/// Commit consumes owned packed pixels after successful copy/unmap and phase
/// validation. A stale/cancelled snapshot must not mutate the existing cache.
/// The adapter must already own the validated snapshot identity and phase
/// proofs, and bind the returned receipt to them. A bare VideoFrame or matching
/// metadata alone is insufficient to mint a production CacheCommitReceipt.
pub(crate) trait CpuSnapshotCommit {
    fn commit(
        &mut self,
        key: &VideoOutputLastValidKey,
        frame: NativeFrameId,
        snapshot: VideoFrame,
        fence: CachePublicationFence,
    ) -> Result<CacheCommitReceipt, SnapshotCommitRejection>;

    fn invalidate_output(&mut self, output: VideoOutputId);
}

#[cfg(test)]
#[path = "native_gpu_contract_tests.rs"]
mod tests;
