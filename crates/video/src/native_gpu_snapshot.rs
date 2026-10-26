//! P-owned snapshot policy for INTERFACE-01 sections 6.2–6.3.
//!
//! The mapper is an owner-thread port, not a callback that may publish pixels.
//! This test-gated policy neither creates a device nor acquires a surface. The
//! real G adapter must drive completion/scopes through the shared broker. Mock
//! phase receipts below are not GPU proof. I must additionally validate the ISF
//! receipt before committing a completed snapshot or constructing its Ready.

use crate::native_gpu_contract::{
    CacheCommitDescriptor, CacheCommitReceipt, CompletionProgress, CpuSnapshotCommit, GpuPhase,
    NativeGpuFault, NativeGpuFaultKind, SnapshotCommitRejection, SnapshotIdentity,
};
use crate::{VideoFrame, VideoPixelFormat, VideoRenderCancellation};

/// Affine evidence that the composition/snapshot phase and map have completed.
/// Production construction must be added at the G/P broker integration seam;
/// a matching descriptor by itself is never sufficient evidence.
#[derive(Debug)]
pub(crate) struct SnapshotPhaseReceipt {
    identity: SnapshotIdentity,
}

impl SnapshotPhaseReceipt {
    #[cfg(test)]
    pub(crate) fn for_test(identity: SnapshotIdentity) -> Self {
        Self { identity }
    }
}

/// Holds the readback owner, callback marker and access to the mutable G broker.
/// poll_phase_and_map must resolve both P scopes and map completion. Callbacks
/// may only set markers. Failure leaves all owners in the port for retirement.
pub(crate) trait SnapshotMapPort {
    type View<'a>: AsRef<[u8]>
    where
        Self: 'a;

    fn poll_phase_and_map(&mut self) -> CompletionProgress<SnapshotPhaseReceipt, NativeGpuFault>;
    fn mapped_view(&mut self) -> Result<Self::View<'_>, NativeGpuFault>;
    fn unmap(&mut self) -> Result<(), NativeGpuFault>;
}

/// Metadata and owners from an encoded phase, not a successful result.
pub(crate) struct SnapshotEncoding<R, M> {
    pub descriptor: CacheCommitDescriptor,
    pub gpu: R,
    pub mapper: M,
    pub readback_row_pitch: u64,
}

pub(crate) struct PendingPremappingSnapshot<R, M> {
    descriptor: CacheCommitDescriptor,
    gpu: Option<R>,
    mapper: M,
    readback_row_pitch: u64,
    terminal: bool,
}

pub(crate) struct CompletedPremappingSnapshot<R> {
    descriptor: CacheCommitDescriptor,
    gpu: R,
    pixels: VideoFrame,
    phase: SnapshotPhaseReceipt,
}

/// This is P's input to I's Ready constructor, not Ready or Fresh evidence.
/// I also requires its validated ISF receipt, frozen mapping and authority.
pub(crate) struct CacheBoundSnapshot<R> {
    gpu: R,
    receipt: CacheCommitReceipt,
}

impl<R> CacheBoundSnapshot<R> {
    pub(crate) fn into_parts(self) -> (R, CacheCommitReceipt) {
        (self.gpu, self.receipt)
    }
}

/// The caller transfers this intact into the transaction retirement registry.
/// No Drop implementation polls, presents or starts detached cleanup.
pub(crate) struct RetiringPremappingSnapshot<R, M> {
    gpu: Option<R>,
    mapper: M,
}

impl<R, M> RetiringPremappingSnapshot<R, M> {
    /// Only the enclosing broker may decide when these owners are reaped.
    pub(crate) fn into_parts(self) -> (Option<R>, M) {
        (self.gpu, self.mapper)
    }
}

pub(crate) enum SnapshotProgress<R> {
    Pending,
    Cancelled,
    Rejected(SnapshotCommitRejection),
    Complete(CompletedPremappingSnapshot<R>),
    Failed(NativeGpuFault),
}

pub(crate) struct SnapshotCommitFailure<R> {
    pub reason: SnapshotCommitRejection,
    pub gpu: R,
}

pub(crate) struct NativeGpuSnapshotter;

impl NativeGpuSnapshotter {
    pub(crate) fn begin<R, M>(encoded: SnapshotEncoding<R, M>) -> PendingPremappingSnapshot<R, M> {
        PendingPremappingSnapshot {
            descriptor: encoded.descriptor,
            gpu: Some(encoded.gpu),
            mapper: encoded.mapper,
            readback_row_pitch: encoded.readback_row_pitch,
            terminal: false,
        }
    }

    pub(crate) fn poll<R, M: SnapshotMapPort>(
        pending: &mut PendingPremappingSnapshot<R, M>,
        current: &CacheCommitDescriptor,
        cancel: &dyn VideoRenderCancellation,
    ) -> SnapshotProgress<R> {
        if pending.terminal {
            return SnapshotProgress::Failed(fault(
                &pending.descriptor.snapshot,
                NativeGpuFaultKind::StateMismatch,
                "terminal snapshot cannot be polled again",
            ));
        }
        if cancel.is_cancelled() {
            pending.terminal = true;
            return SnapshotProgress::Cancelled;
        }
        if let Err(reason) = publication_matches(&pending.descriptor, current) {
            pending.terminal = true;
            return SnapshotProgress::Rejected(reason);
        }
        if let Err(error) = validate_encoding(&pending.descriptor, pending.readback_row_pitch) {
            pending.terminal = true;
            return SnapshotProgress::Failed(error);
        }
        let phase = match pending.mapper.poll_phase_and_map() {
            CompletionProgress::Pending => return SnapshotProgress::Pending,
            CompletionProgress::Cancelled => {
                pending.terminal = true;
                return SnapshotProgress::Cancelled;
            }
            CompletionProgress::Complete(Err(error)) => {
                pending.terminal = true;
                return SnapshotProgress::Failed(error);
            }
            CompletionProgress::Complete(Ok(phase)) => phase,
        };
        pending.terminal = true;
        if phase.identity != pending.descriptor.snapshot {
            return SnapshotProgress::Failed(fault(
                &pending.descriptor.snapshot,
                NativeGpuFaultKind::IdentityMismatch,
                "foreign or late composition/snapshot receipt",
            ));
        }
        let pixels = match copy_and_unmap(
            &mut pending.mapper,
            &phase.identity,
            pending.readback_row_pitch,
        ) {
            Ok(pixels) => pixels,
            Err(error) => return SnapshotProgress::Failed(error),
        };
        if cancel.is_cancelled() {
            // Completion does not outrun a Stop that arrived during copy/unmap.
            // The source and readback stay pending-owned for retirement.
            return SnapshotProgress::Cancelled;
        }
        // The map view is gone and unmap succeeded before the owned frame exits.
        SnapshotProgress::Complete(CompletedPremappingSnapshot {
            descriptor: pending.descriptor.clone(),
            gpu: pending
                .gpu
                .take()
                .expect("nonterminal snapshot owns GPU source"),
            pixels,
            phase,
        })
    }
}

impl<R, M> PendingPremappingSnapshot<R, M> {
    pub(crate) fn into_retirement(self) -> RetiringPremappingSnapshot<R, M> {
        RetiringPremappingSnapshot {
            gpu: self.gpu,
            mapper: self.mapper,
        }
    }
}

impl<R> CompletedPremappingSnapshot<R> {
    pub(crate) fn commit<C: CpuSnapshotCommit>(
        self,
        cache: &mut C,
        current: &CacheCommitDescriptor,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<CacheBoundSnapshot<R>, SnapshotCommitFailure<R>> {
        let reject = |reason, gpu| SnapshotCommitFailure { reason, gpu };
        if cancel.is_cancelled() {
            return Err(reject(SnapshotCommitRejection::Cancelled, self.gpu));
        }
        if let Err(reason) = publication_matches(&self.descriptor, current) {
            return Err(reject(reason, self.gpu));
        }
        if self.phase.identity != self.descriptor.snapshot {
            return Err(reject(
                SnapshotCommitRejection::SnapshotIdentityMismatch,
                self.gpu,
            ));
        }
        // Exactly one move of the owned packed Vec. No full-frame clone.
        let receipt = match cache.commit(
            &self.descriptor.key,
            self.descriptor.snapshot.frame,
            self.pixels,
            self.descriptor.fence,
        ) {
            Ok(receipt) => receipt,
            Err(reason) => return Err(reject(reason, self.gpu)),
        };
        if receipt.descriptor() != &self.descriptor {
            return Err(reject(
                SnapshotCommitRejection::SnapshotIdentityMismatch,
                self.gpu,
            ));
        }
        Ok(CacheBoundSnapshot {
            gpu: self.gpu,
            receipt,
        })
    }
}

fn publication_matches(
    captured: &CacheCommitDescriptor,
    current: &CacheCommitDescriptor,
) -> Result<(), SnapshotCommitRejection> {
    if captured.snapshot.frame.project_epoch != current.snapshot.frame.project_epoch {
        return Err(SnapshotCommitRejection::StaleEpoch);
    }
    if captured.snapshot.frame.request_sequence != current.snapshot.frame.request_sequence
        || captured.fence.request_sequence != current.fence.request_sequence
    {
        return Err(SnapshotCommitRejection::StaleRequest);
    }
    if captured.fence.blackout_generation != current.fence.blackout_generation {
        return Err(SnapshotCommitRejection::BlackoutFence);
    }
    if captured.fence.configuration_generation != current.fence.configuration_generation {
        return Err(SnapshotCommitRejection::ConfigurationChanged);
    }
    if captured.key != current.key {
        return Err(SnapshotCommitRejection::KeyMismatch);
    }
    if captured.snapshot != current.snapshot {
        return Err(SnapshotCommitRejection::SnapshotIdentityMismatch);
    }
    Ok(())
}

fn validate_encoding(
    descriptor: &CacheCommitDescriptor,
    row_pitch: u64,
) -> Result<(), NativeGpuFault> {
    let image = &descriptor.snapshot;
    if image.allocation.context != image.completed_submission.context
        || image.frame.project_epoch != descriptor.key.project_render_epoch
        || image.frame.output_id != descriptor.key.output_id
        || image.frame.request_sequence != descriptor.fence.request_sequence
        || image.width != descriptor.key.width
        || image.height != descriptor.key.height
    {
        return Err(fault(
            image,
            NativeGpuFaultKind::IdentityMismatch,
            "inconsistent snapshot encoding",
        ));
    }
    let packed_row = u64::from(image.width).checked_mul(4);
    let capacity = row_pitch.checked_mul(u64::from(image.height));
    if image.width == 0
        || image.height == 0
        || image.format != VideoPixelFormat::Rgba8
        || packed_row.is_none_or(|row| row_pitch < row)
        || capacity.is_none_or(|len| usize::try_from(len).is_err())
    {
        return Err(fault(
            image,
            NativeGpuFaultKind::SnapshotCopy,
            "invalid packed RGBA snapshot extent or pitch",
        ));
    }
    Ok(())
}

fn copy_and_unmap<M: SnapshotMapPort>(
    mapper: &mut M,
    identity: &SnapshotIdentity,
    row_pitch: u64,
) -> Result<VideoFrame, NativeGpuFault> {
    let copied = match mapper.mapped_view() {
        Err(error) => Err(error),
        Ok(view) => {
            let bytes = view.as_ref();
            let row = identity.width as usize * 4;
            let height = identity.height as usize;
            let pitch = row_pitch as usize;
            let required = pitch * (height - 1) + row;
            if bytes.len() < required {
                Err(fault(
                    identity,
                    NativeGpuFaultKind::SnapshotCopy,
                    "mapped snapshot is shorter than the admitted layout",
                ))
            } else {
                let mut packed = Vec::with_capacity(row * height);
                for y in 0..height {
                    packed.extend_from_slice(&bytes[y * pitch..y * pitch + row]);
                }
                Ok(packed)
            }
            // View drops here on both success and copy failure, before unmap.
        }
    };
    mapper.unmap()?;
    let data = copied?;
    Ok(VideoFrame {
        layer_id: 0,
        width: identity.width,
        height: identity.height,
        pts_ms: identity.pts_ms,
        duration_ms: identity.duration_ms,
        format: identity.format,
        data,
    })
}

fn fault(identity: &SnapshotIdentity, kind: NativeGpuFaultKind, message: &str) -> NativeGpuFault {
    NativeGpuFault {
        context: identity.allocation.context,
        frame: Some(identity.frame),
        phase: GpuPhase::CompositionSnapshot,
        kind,
        message: message.into(),
    }
}

#[cfg(test)]
#[path = "native_gpu_snapshot_tests.rs"]
mod tests;
