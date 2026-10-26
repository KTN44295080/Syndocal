//! I's receiver for P's complete charged CPU history barrier.
//!
//! This is an ownership adapter for the existing frame-history policy. It does
//! not choose when an Error/LastValid result is published, advance FX clocks, or
//! create a host budget. The caller supplies a P history created with the output
//! session's shared budget. Wiring these owners into lib.rs::FrameQueue remains
//! a separate integration step; its raw-frame storage is not changed here.

use std::sync::Arc;

use crate::frame_history_policy::{
    self, plan_history_update, HistoryMeta, HistoryPushKind, NATIVE_HISTORY_CAPACITY,
};
use crate::native_gpu_budget::AllocationCharge;
use crate::native_gpu_contract::{GpuContextId, HistoryEntryId, LayerResultIdentity};
use crate::native_gpu_history::adapter::NativeGpuHistory;
use crate::native_gpu_history::{
    CpuHandoverFailure, CpuHistoryBacking, CpuHistoryHandover, CpuHistorySnapshot, HistoryError,
    HistoryHostBudget, HistoryScope,
};
use crate::native_gpu_result::{FrozenStatelessRecipe, GpuLayerLease, ValidatedCpuMaterialization};
use crate::{VideoFrame, VideoPixelFormat};

pub(crate) type NativeCpuHistoryHandover =
    CpuHistoryHandover<FrozenStatelessRecipe, GpuLayerLease, AllocationCharge>;

/// Pass the actual affine G outcome directly to P. Its failure still owns the
/// unsplit pixels, descriptor and charge; an ExactCpuLayer is not sufficient.
pub(crate) fn accept_cpu_history_materialization(
    handover: &mut NativeCpuHistoryHandover,
    expected: HistoryEntryId,
    materialized: ValidatedCpuMaterialization,
) -> Result<(), CpuHandoverFailure<ValidatedCpuMaterialization>> {
    handover.accept_materialized(expected, materialized)
}

/// A CPU consumer's alias always retains the complete P owner. The identity is
/// the canonical input/layer identity; the frame's diagnostic layer tag may be
/// different after P imports an already shared CPU frame.
#[derive(Clone)]
pub(crate) struct OwnedCpuHistoryEntry {
    scope: HistoryScope,
    identity: LayerResultIdentity,
    backing: Arc<CpuHistoryBacking<AllocationCharge>>,
}

impl OwnedCpuHistoryEntry {
    pub(crate) fn identity(&self) -> &LayerResultIdentity {
        &self.identity
    }

    pub(crate) fn frame(&self) -> &VideoFrame {
        self.backing.frame()
    }

    /// Clone this whole owner when a consumer must outlive the queue.
    pub(crate) fn backing(&self) -> &Arc<CpuHistoryBacking<AllocationCharge>> {
        &self.backing
    }
}

/// A full eight-slot queue backed by charged P owners. Ordering, replacement,
/// eviction and selection are delegated to the canonical metadata policy.
pub(crate) struct NativeCpuHistoryQueue {
    scope: HistoryScope,
    entries: Vec<OwnedCpuHistoryEntry>,
    metadata: Vec<HistoryMeta>,
    last_sequence: Option<u64>,
}

#[must_use]
pub(crate) struct CpuHistorySnapshotRejection {
    pub error: HistoryError,
    pub snapshot: CpuHistorySnapshot<AllocationCharge>,
}

#[must_use]
pub(crate) struct CpuHistoryEntryRejection {
    pub error: HistoryError,
    pub entry: OwnedCpuHistoryEntry,
}

impl NativeCpuHistoryQueue {
    /// Inspect the borrowed, sealed snapshot before consuming it. A rejection
    /// returns the complete snapshot and every retained charge unchanged.
    pub(crate) fn from_snapshot(
        expected_scope: HistoryScope,
        snapshot: CpuHistorySnapshot<AllocationCharge>,
    ) -> Result<Self, CpuHistorySnapshotRejection> {
        let checked = (|| {
            if snapshot.scope() != expected_scope {
                return Err(HistoryError::IdentityMismatch);
            }
            let mut metadata = Vec::with_capacity(NATIVE_HISTORY_CAPACITY);
            for (identity, backing) in snapshot.entries() {
                validate_entry(expected_scope, identity, backing.frame())?;
                metadata.push(entry_meta(identity));
            }
            plan_history_update(&metadata, &[], NATIVE_HISTORY_CAPACITY, 0)
                .map_err(|_| HistoryError::InvalidPolicy)?;
            Ok(metadata)
        })();
        let metadata = match checked {
            Ok(metadata) => metadata,
            Err(error) => return Err(CpuHistorySnapshotRejection { error, snapshot }),
        };
        let last_sequence = metadata.iter().map(|meta| meta.id.sequence).max();
        let entries = snapshot
            .into_entries()
            .into_iter()
            .map(|(identity, backing)| OwnedCpuHistoryEntry {
                scope: expected_scope,
                identity,
                backing,
            })
            .collect();
        Ok(Self {
            scope: expected_scope,
            entries,
            metadata,
            last_sequence,
        })
    }

    pub(crate) fn scope(&self) -> HistoryScope {
        self.scope
    }

    pub(crate) fn len(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn entries(&self) -> impl Iterator<Item = &OwnedCpuHistoryEntry> {
        self.entries.iter()
    }

    pub(crate) fn into_entries(self) -> Vec<OwnedCpuHistoryEntry> {
        self.entries
    }

    pub(crate) fn metadata(&self) -> &[HistoryMeta] {
        &self.metadata
    }

    pub(crate) fn select(&self, target_ms: u64) -> Option<OwnedCpuHistoryEntry> {
        self.selected(frame_history_policy::select_at_or_before_or_nearest(
            &self.metadata,
            target_ms,
        ))
    }

    pub(crate) fn nearest(&self, target_ms: u64) -> Option<OwnedCpuHistoryEntry> {
        self.selected(frame_history_policy::select_nearest(
            &self.metadata,
            target_ms,
        ))
    }

    fn selected(&self, selected: Option<HistoryEntryId>) -> Option<OwnedCpuHistoryEntry> {
        selected.and_then(|id| {
            self.entries
                .iter()
                .find(|entry| entry.identity.entry == id)
                .cloned()
        })
    }

    /// Apply one already sealed CPU owner. Sequence validation survives
    /// eviction/drop_before, so an old completion cannot replace a newer slot.
    pub(crate) fn push(
        &mut self,
        entry: OwnedCpuHistoryEntry,
    ) -> Result<HistoryPushKind, CpuHistoryEntryRejection> {
        let checked = (|| {
            if entry.scope != self.scope {
                return Err(HistoryError::IdentityMismatch);
            }
            if self.last_sequence == Some(u64::MAX) {
                return Err(HistoryError::SequenceExhausted);
            }
            if self
                .last_sequence
                .is_some_and(|last| entry.identity.entry.sequence <= last)
            {
                return Err(HistoryError::StateMismatch);
            }
            plan_history_update(
                &self.metadata,
                &[entry_meta(&entry.identity)],
                NATIVE_HISTORY_CAPACITY,
                0,
            )
            .map_err(|_| HistoryError::InvalidPolicy)
        })();
        let plan = match checked {
            Ok(plan) => plan,
            Err(error) => return Err(CpuHistoryEntryRejection { error, entry }),
        };
        let incoming = entry.identity.entry;
        // The policy decides which owner retires and where the new owner goes.
        self.entries.retain(|existing| {
            plan.survivors()
                .iter()
                .any(|meta| meta.id == existing.identity.entry)
        });
        let index = plan
            .survivors()
            .iter()
            .position(|meta| meta.id == incoming)
            .expect("one incoming owner always survives its canonical push");
        self.entries.insert(index, entry);
        self.metadata.clear();
        self.metadata.extend_from_slice(plan.survivors());
        self.last_sequence = Some(incoming.sequence);
        Ok(plan.pushes()[0])
    }

    pub(crate) fn drop_before(&mut self, target_ms: u64) -> usize {
        let dropped = frame_history_policy::drop_before(&mut self.metadata, target_ms);
        self.entries.retain(|entry| {
            self.metadata
                .iter()
                .any(|meta| meta.id == entry.identity.entry)
        });
        dropped
    }
}

/// Ownership only: the canonical caller decides when a result is LastValid.
/// This is per input/layer, separate from the output premapping CPU cache.
pub(crate) struct NativeCpuHistoryLastValid {
    scope: HistoryScope,
    entry: Option<OwnedCpuHistoryEntry>,
}

impl NativeCpuHistoryLastValid {
    pub(crate) fn new(scope: HistoryScope) -> Self {
        Self { scope, entry: None }
    }

    pub(crate) fn retain(
        &mut self,
        entry: OwnedCpuHistoryEntry,
    ) -> Result<(), CpuHistoryEntryRejection> {
        if entry.scope != self.scope {
            return Err(CpuHistoryEntryRejection {
                error: HistoryError::IdentityMismatch,
                entry,
            });
        }
        self.entry = Some(entry);
        Ok(())
    }

    pub(crate) fn get(&self) -> Option<&OwnedCpuHistoryEntry> {
        self.entry.as_ref()
    }

    pub(crate) fn clear(&mut self) {
        self.entry = None;
    }
}

/// Queue-incarnation sequence issuance, independent of CPU slot indices or FX
/// FRAMEINDEX. The caller supplies the last issued ID for this exact scope.
pub(crate) struct CpuHistoryEntrySequence {
    scope: HistoryScope,
    last_issued: Option<u64>,
}

impl CpuHistoryEntrySequence {
    pub(crate) fn new(scope: HistoryScope, last_issued: Option<u64>) -> Self {
        Self { scope, last_issued }
    }

    pub(crate) fn last_issued(&self) -> Option<u64> {
        self.last_issued
    }
}

#[must_use]
pub(crate) struct LegacyCpuHistoryRejection {
    pub error: HistoryError,
    pub frames: Vec<Arc<VideoFrame>>,
}

/// Import an existing legacy CPU queue using the output session's host budget.
/// These are already raw CPU Arcs: this API never extracts one from a charged
/// backing. Clone only those Arc handles while P validates its consuming input,
/// so failure can return the complete original queue and leave issuance intact.
pub(crate) fn import_legacy_cpu_history(
    context: GpuContextId,
    scope: HistoryScope,
    host: &HistoryHostBudget,
    sequence: &mut CpuHistoryEntrySequence,
    frames: Vec<Arc<VideoFrame>>,
) -> Result<NativeGpuHistory, LegacyCpuHistoryRejection> {
    let checked = (|| {
        if sequence.scope != scope {
            return Err(HistoryError::IdentityMismatch);
        }
        if frames.len() > NATIVE_HISTORY_CAPACITY {
            return Err(HistoryError::InvalidPolicy);
        }
        let mut last_issued = sequence.last_issued;
        let mut entries = Vec::with_capacity(frames.len());
        for frame in &frames {
            let next = last_issued
                .unwrap_or(0)
                .checked_add(1)
                .ok_or(HistoryError::SequenceExhausted)?;
            entries.push((
                HistoryEntryId {
                    input: scope.input,
                    queue_epoch: scope.queue_epoch,
                    sequence: next,
                },
                Arc::clone(frame),
            ));
            last_issued = Some(next);
        }
        NativeGpuHistory::import_canonical_queue(context, scope, host.clone(), entries)
            .map(|history| (history, last_issued))
    })();
    match checked {
        Ok((history, last_issued)) => {
            sequence.last_issued = last_issued;
            Ok(history)
        }
        Err(error) => Err(LegacyCpuHistoryRejection { error, frames }),
    }
}

fn entry_meta(identity: &LayerResultIdentity) -> HistoryMeta {
    HistoryMeta {
        id: identity.entry,
        pts_ms: identity.pts_ms,
        duration_ms: identity.duration_ms,
    }
}

fn validate_entry(
    scope: HistoryScope,
    identity: &LayerResultIdentity,
    frame: &VideoFrame,
) -> Result<(), HistoryError> {
    if scope.frame_scope.project_epoch != scope.input.project_render_epoch
        || identity.frame.output_id != scope.frame_scope.output_id
        || identity.frame.project_epoch != scope.frame_scope.project_epoch
        || identity.frame.geometry_generation != scope.frame_scope.geometry_generation
        || identity.entry.input != scope.input
        || identity.entry.queue_epoch != scope.queue_epoch
        || identity.layer_id != scope.layer_id
        || identity.source_generation != scope.source_generation
        || (identity.width, identity.height) != scope.extent
        || identity.format != VideoPixelFormat::Rgba8
    {
        return Err(HistoryError::IdentityMismatch);
    }
    let packed_bytes = u64::from(identity.width)
        .checked_mul(u64::from(identity.height))
        .and_then(|pixels| pixels.checked_mul(4))
        .filter(|bytes| *bytes != 0);
    if frame.width != identity.width
        || frame.height != identity.height
        || frame.pts_ms != identity.pts_ms
        || frame.duration_ms != identity.duration_ms
        || frame.format != identity.format
        || packed_bytes != u64::try_from(frame.data.len()).ok()
    {
        return Err(HistoryError::InvalidPixels);
    }
    Ok(())
}

#[cfg(test)]
#[path = "native_gpu_output_cpu_history_tests.rs"]
mod tests;
