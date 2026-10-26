//! P-owned history policy (INTERFACE-01 ﾂｧ5), independent of G's concrete GPU.
//! Eviction transfers strong owners; it never releases a GPU budget charge.
use crate::frame_history_policy::{plan_history_update, HistoryMeta, HistoryPlan};
use crate::native_gpu_contract::{
    ExactCpuLayer, GpuAllocationId, GpuContextId, HistoryEntryId, LayerOrigin, LayerResultIdentity,
    MaterializationProof, NativeFrameId,
};
use crate::{VideoFrame, VideoPixelFormat};
use protocol::{VideoLayerId, VideoRenderInputKey};
use std::sync::Arc;

const CAPACITY: usize = 8;
const RESIDENT_CAPACITY: usize = 2;

/// Implemented by the future G lease adapter, never by a second allocator.
pub(crate) trait HistoryResident {
    fn identity(&self) -> &LayerResultIdentity;
    fn allocation(&self) -> GpuAllocationId;
}
pub(crate) enum HistoryBacking<R> {
    Replay(R),
    Cpu(Arc<VideoFrame>),
}
struct Entry<R, T> {
    identity: LayerResultIdentity,
    // Original G receipt/lease identity; layer normalization changes metadata only.
    resource_identity: LayerResultIdentity,
    backing: HistoryBacking<R>,
    resident: Option<Arc<T>>,
}
impl<R, T> Entry<R, T> {
    fn meta(&self) -> HistoryMeta {
        HistoryMeta {
            id: self.identity.entry,
            pts_ms: self.identity.pts_ms,
            duration_ms: self.identity.duration_ms,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryError {
    InvalidPolicy,
    StateMismatch,
    IdentityMismatch,
    SequenceExhausted,
    IsfFailed,
    Cancelled,
    InvalidPixels,
}
#[derive(Clone, Copy)]
pub(crate) enum HistoryPhaseOutcome {
    IsfValid { composition_valid: bool },
    IsfFailed,
    Cancelled,
}
/// No production success constructor. P's integration adapter must consume
/// G's validated receipt or an exact canonical CPU outcome.
pub(crate) struct ValidatedHistoryEntries<R, T> {
    frame: NativeFrameId,
    phase: HistoryPhaseOutcome,
    entries: Vec<Entry<R, T>>,
}
/// Rejected affine inputs are returned intact, not silently dropped.
pub(crate) struct HistoryCommitFailure<R, T> {
    pub error: HistoryError,
    pub plan: HistoryPlan,
    pub entries: ValidatedHistoryEntries<R, T>,
}
pub(crate) struct RetiredResident<T> {
    pub entry: HistoryEntryId,
    owner: Arc<T>,
}
impl<T> RetiredResident<T> {
    pub(crate) fn resource(&self) -> &T {
        &self.owner
    }
}
pub(crate) struct HistoryReadPin<T> {
    pub entry: HistoryEntryId,
    owner: Arc<T>,
}
impl<T> HistoryReadPin<T> {
    pub(crate) fn resource(&self) -> &T {
        &self.owner
    }
}
pub(crate) struct HistoryCommit<T> {
    pub retired: Vec<RetiredResident<T>>,
    pub output_phases_valid: bool,
}
pub(crate) struct ResidentChange<T> {
    pub retired: Vec<RetiredResident<T>>,
}
pub(crate) struct MaterializationAttachFailure<T> {
    pub error: HistoryError,
    pub resident: T,
    pub proof: MaterializationProof,
}
pub(crate) enum HistoryMaterializationRequest<'a, R, T> {
    Incoming {
        entry: HistoryEntryId,
    },
    Resident(&'a T),
    Replay {
        entry: HistoryEntryId,
        frozen: &'a R,
    },
    UploadCpuPostFx {
        entry: HistoryEntryId,
        frame: &'a Arc<VideoFrame>,
    },
}
pub(crate) struct HistorySelection<'a, R, T> {
    pub layer_id: VideoLayerId,
    pub selected_identity: Option<LayerResultIdentity>,
    pub selected: Option<HistoryMaterializationRequest<'a, R, T>>,
    /// Must be included in pre-execution allocation quote.
    pub cold_materialization_required: bool,
}
pub(crate) struct NativeHistory<R, T> {
    context: GpuContextId,
    frame_scope: NativeFrameId,
    input: VideoRenderInputKey,
    layer_id: VideoLayerId,
    queue_epoch: u64,
    source_generation: u64,
    extent: (u32, u32),
    revision: u64,
    last_sequence: Option<u64>,
    entries: Vec<Entry<R, T>>,
}
impl<R, T: HistoryResident> NativeHistory<R, T> {
    pub(crate) fn new(
        context: GpuContextId,
        frame_scope: NativeFrameId,
        input: VideoRenderInputKey,
        layer_id: VideoLayerId,
        queue_epoch: u64,
        source_generation: u64,
        extent: (u32, u32),
    ) -> Result<Self, HistoryError> {
        if extent.0 == 0 || extent.1 == 0 {
            return Err(HistoryError::InvalidPixels);
        }
        if frame_scope.project_epoch != input.project_render_epoch {
            return Err(HistoryError::IdentityMismatch);
        }
        Ok(Self {
            context,
            frame_scope,
            input,
            layer_id,
            queue_epoch,
            source_generation,
            extent,
            revision: 0,
            last_sequence: None,
            entries: Vec::new(),
        })
    }
    pub(crate) fn metadata(&self) -> Vec<HistoryMeta> {
        self.entries.iter().map(Entry::meta).collect()
    }
    pub(crate) fn resident_count(&self) -> usize {
        self.entries.iter().filter(|e| e.resident.is_some()).count()
    }
    pub(crate) fn plan(
        &self,
        incoming: &[HistoryMeta],
        target_ms: u64,
    ) -> Result<HistoryPlan, HistoryError> {
        let mut previous = self.last_sequence;
        for meta in incoming {
            if meta.id.input != self.input || meta.id.queue_epoch != self.queue_epoch {
                return Err(HistoryError::IdentityMismatch);
            }
            if previous == Some(u64::MAX) {
                return Err(HistoryError::SequenceExhausted);
            }
            if previous.is_some_and(|last| meta.id.sequence <= last) {
                return Err(HistoryError::StateMismatch);
            }
            previous = Some(meta.id.sequence);
        }
        plan_history_update(&self.metadata(), incoming, CAPACITY, target_ms)
            .map(|p| p.bind_revision(self.revision))
            .map_err(|_| HistoryError::InvalidPolicy)
    }
    fn check_plan(&self, plan: &HistoryPlan) -> Result<(), HistoryError> {
        if plan.expected_revision() != self.revision || plan.current() != self.metadata() {
            return Err(HistoryError::StateMismatch);
        }
        let checked = self.plan(plan.incoming(), 0)?;
        if checked.survivors() != plan.survivors() {
            return Err(HistoryError::StateMismatch);
        }
        Ok(())
    }
    pub(crate) fn select_materialization_requests(
        &self,
        plan: &HistoryPlan,
    ) -> Result<HistorySelection<'_, R, T>, HistoryError> {
        self.check_plan(plan)?;
        let selected = match plan.selected() {
            None => None,
            Some(id) if plan.incoming().iter().any(|m| m.id == id) => {
                Some(HistoryMaterializationRequest::Incoming { entry: id })
            }
            Some(id) => {
                let e = self
                    .entries
                    .iter()
                    .find(|e| e.identity.entry == id)
                    .ok_or(HistoryError::IdentityMismatch)?;
                Some(match &e.resident {
                    Some(owner) => HistoryMaterializationRequest::Resident(owner.as_ref()),
                    None => match &e.backing {
                        HistoryBacking::Replay(frozen) => {
                            HistoryMaterializationRequest::Replay { entry: id, frozen }
                        }
                        HistoryBacking::Cpu(frame) => {
                            HistoryMaterializationRequest::UploadCpuPostFx { entry: id, frame }
                        }
                    },
                })
            }
        };
        let cold_materialization_required = matches!(
            selected,
            Some(
                HistoryMaterializationRequest::Replay { .. }
                    | HistoryMaterializationRequest::UploadCpuPostFx { .. }
            )
        );
        let selected_identity = plan
            .selected()
            .and_then(|id| self.entries.iter().find(|e| e.identity.entry == id))
            .map(|e| e.identity);
        Ok(HistorySelection {
            layer_id: self.layer_id,
            selected_identity,
            selected,
            cold_materialization_required,
        })
    }
    pub(crate) fn pin(&self, id: HistoryEntryId) -> Result<HistoryReadPin<T>, HistoryError> {
        let e = self
            .entries
            .iter()
            .find(|e| e.identity.entry == id)
            .ok_or(HistoryError::IdentityMismatch)?;
        let owner = e.resident.as_ref().ok_or(HistoryError::StateMismatch)?;
        Ok(HistoryReadPin {
            entry: id,
            owner: Arc::clone(owner),
        })
    }
    fn check_identity(&self, i: &LayerResultIdentity) -> Result<(), HistoryError> {
        if i.frame.output_id != self.frame_scope.output_id
            || i.frame.geometry_generation != self.frame_scope.geometry_generation
            || i.context != self.context
            || i.entry.input != self.input
            || i.entry.queue_epoch != self.queue_epoch
            || i.frame.project_epoch != self.input.project_render_epoch
            || i.source_generation != self.source_generation
            || (i.width, i.height) != self.extent
            || i.format != VideoPixelFormat::Rgba8
        {
            return Err(HistoryError::IdentityMismatch);
        }
        Ok(())
    }
    pub(crate) fn commit_validated(
        &mut self,
        plan: HistoryPlan,
        mut entries: ValidatedHistoryEntries<R, T>,
    ) -> Result<HistoryCommit<T>, HistoryCommitFailure<R, T>> {
        let checked = (|| {
            self.check_plan(&plan)?;
            let composition_valid = match entries.phase {
                HistoryPhaseOutcome::IsfValid { composition_valid } => composition_valid,
                HistoryPhaseOutcome::IsfFailed => return Err(HistoryError::IsfFailed),
                HistoryPhaseOutcome::Cancelled => return Err(HistoryError::Cancelled),
            };
            let next_revision = self
                .revision
                .checked_add(1)
                .ok_or(HistoryError::SequenceExhausted)?;
            if entries.entries.len() != plan.incoming().len() {
                return Err(HistoryError::IdentityMismatch);
            }
            for (e, meta) in entries.entries.iter().zip(plan.incoming()) {
                self.check_identity(&e.identity)?;
                if e.meta() != *meta || e.identity.frame != entries.frame {
                    return Err(HistoryError::IdentityMismatch);
                }
                if e.resource_identity != e.identity {
                    return Err(HistoryError::IdentityMismatch);
                }
                if let Some(r) = &e.resident {
                    if r.identity() != &e.identity || r.allocation().context != self.context {
                        return Err(HistoryError::IdentityMismatch);
                    }
                }
                if let HistoryBacking::Cpu(frame) = &e.backing {
                    check_pixels(frame, &e.resource_identity)?;
                }
            }
            Ok((next_revision, composition_valid))
        })();
        let (next_revision, composition_valid) = match checked {
            Ok(v) => v,
            Err(error) => {
                return Err(HistoryCommitFailure {
                    error,
                    plan,
                    entries,
                })
            }
        };
        let mut pool = std::mem::take(&mut self.entries);
        for e in &mut entries.entries {
            e.identity.layer_id = self.layer_id;
        }
        pool.append(&mut entries.entries);
        let mut survivors = Vec::with_capacity(CAPACITY);
        for meta in plan.survivors() {
            let i = pool
                .iter()
                .position(|e| e.identity.entry == meta.id)
                .expect("checked survivor");
            survivors.push(pool.remove(i));
        }
        let mut retired = Vec::new();
        for e in &mut pool {
            retire(e, &mut retired);
        }
        self.entries = survivors;
        self.trim_residents(plan.selected(), &mut retired);
        self.revision = next_revision;
        if let Some(last) = plan.incoming().last() {
            self.last_sequence = Some(last.id.sequence);
        }
        Ok(HistoryCommit {
            retired,
            output_phases_valid: composition_valid,
        })
    }
    fn trim_residents(
        &mut self,
        selected: Option<HistoryEntryId>,
        retired: &mut Vec<RetiredResident<T>>,
    ) {
        let mut order: Vec<usize> = (0..self.entries.len())
            .filter(|&i| self.entries[i].resident.is_some())
            .collect();
        order.sort_by_key(|&i| {
            let e = &self.entries[i];
            (
                std::cmp::Reverse(Arc::strong_count(e.resident.as_ref().unwrap()) > 1),
                std::cmp::Reverse(Some(e.identity.entry) == selected),
                std::cmp::Reverse(e.identity.entry.sequence),
            )
        });
        for i in order.into_iter().skip(RESIDENT_CAPACITY) {
            retire(&mut self.entries[i], retired);
        }
    }
    pub(crate) fn install_materialized(
        &mut self,
        expected: HistoryEntryId,
        resident: T,
        proof: MaterializationProof,
    ) -> Result<ResidentChange<T>, MaterializationAttachFailure<T>> {
        let checked = (|| {
            let e = self
                .entries
                .iter()
                .find(|e| e.identity.entry == expected)
                .ok_or(HistoryError::IdentityMismatch)?;
            let d = proof.descriptor();
            if d.layer != e.resource_identity
                || resident.identity() != &e.resource_identity
                || d.allocation != resident.allocation()
                || d.allocation.context != self.context
                || d.completed_submission.context != self.context
            {
                return Err(HistoryError::IdentityMismatch);
            }
            self.revision
                .checked_add(1)
                .ok_or(HistoryError::SequenceExhausted)
        })();
        let revision = match checked {
            Ok(v) => v,
            Err(error) => {
                return Err(MaterializationAttachFailure {
                    error,
                    resident,
                    proof,
                })
            }
        };
        let mut retired = Vec::new();
        let e = self
            .entries
            .iter_mut()
            .find(|e| e.identity.entry == expected)
            .unwrap();
        retire(e, &mut retired);
        e.resident = Some(Arc::new(resident));
        self.trim_residents(Some(expected), &mut retired);
        self.revision = revision;
        Ok(ResidentChange { retired })
    }
    pub(crate) fn begin_cpu_handover(self) -> CpuHistoryHandover<R, T> {
        let mut handover = CpuHistoryHandover {
            history: self,
            retired: Vec::new(),
        };
        for e in &mut handover.history.entries {
            if matches!(e.backing, HistoryBacking::Cpu(_)) {
                retire(e, &mut handover.retired);
            }
        }
        handover
    }
    pub(crate) fn import_cpu_history(
        snapshot: CpuHistorySnapshot,
        context: GpuContextId,
    ) -> Result<Self, HistoryError> {
        let mut h = Self::new(
            context,
            snapshot.frame_scope,
            snapshot.input,
            snapshot.layer_id,
            snapshot.queue_epoch,
            snapshot.source_generation,
            snapshot.extent,
        )?;
        if snapshot.entries.len() > CAPACITY {
            return Err(HistoryError::InvalidPolicy);
        }
        for (mut identity, frame) in snapshot.entries {
            identity.context = context;
            identity.origin = LayerOrigin::ImportedCpu;
            h.check_identity(&identity)?;
            let resource_identity = LayerResultIdentity {
                layer_id: frame.layer_id,
                ..identity
            };
            check_pixels(&frame, &resource_identity)?;
            identity.layer_id = h.layer_id;
            h.last_sequence = Some(h.last_sequence.unwrap_or(0).max(identity.entry.sequence));
            h.entries.push(Entry {
                identity,
                resource_identity,
                backing: HistoryBacking::Cpu(frame),
                resident: None,
            });
        }
        plan_history_update(&h.metadata(), &[], CAPACITY, 0)
            .map_err(|_| HistoryError::InvalidPolicy)?;
        Ok(h)
    }
}
fn retire<R, T>(e: &mut Entry<R, T>, retired: &mut Vec<RetiredResident<T>>) {
    if let Some(owner) = e.resident.take() {
        retired.push(RetiredResident {
            entry: e.identity.entry,
            owner,
        });
    }
}
fn check_pixels(frame: &VideoFrame, i: &LayerResultIdentity) -> Result<(), HistoryError> {
    let expected = u64::from(i.width)
        .checked_mul(u64::from(i.height))
        .and_then(|n| n.checked_mul(4));
    if frame.layer_id != i.layer_id
        || frame.width != i.width
        || frame.height != i.height
        || frame.pts_ms != i.pts_ms
        || frame.duration_ms != i.duration_ms
        || frame.format != VideoPixelFormat::Rgba8
        || expected != Some(frame.data.len() as u64)
    {
        return Err(HistoryError::InvalidPixels);
    }
    Ok(())
}
pub(crate) struct CpuHistorySnapshot {
    frame_scope: NativeFrameId,
    input: VideoRenderInputKey,
    layer_id: VideoLayerId,
    queue_epoch: u64,
    source_generation: u64,
    extent: (u32, u32),
    entries: Vec<(LayerResultIdentity, Arc<VideoFrame>)>,
}
impl CpuHistorySnapshot {
    /// Identity carries the canonical queue layer. Shared CPU backing retains
    /// its diagnostic tag; the CPU adapter publishes by queue identity.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&LayerResultIdentity, &Arc<VideoFrame>)> {
        self.entries.iter().map(|(i, f)| (i, f))
    }
}
pub(crate) enum CpuHistoryMaterializationRequest<'a, R, T> {
    ReadValidatedGpu {
        entry: HistoryEntryId,
        resident: &'a T,
    },
    ReplayToCpu {
        entry: HistoryEntryId,
        frozen: &'a R,
    },
}
pub(crate) struct CpuHistoryHandover<R, T> {
    history: NativeHistory<R, T>,
    retired: Vec<RetiredResident<T>>,
}
pub(crate) struct CpuHandoverFailure {
    pub error: HistoryError,
    pub exact: ExactCpuLayer,
}
impl<R, T: HistoryResident> CpuHistoryHandover<R, T> {
    pub(crate) fn next_entry(&self) -> Option<CpuHistoryMaterializationRequest<'_, R, T>> {
        self.history.entries.iter().find_map(|e| match &e.backing {
            HistoryBacking::Cpu(_) => None,
            HistoryBacking::Replay(frozen) => Some(match &e.resident {
                Some(resident) => CpuHistoryMaterializationRequest::ReadValidatedGpu {
                    entry: e.identity.entry,
                    resident: resident.as_ref(),
                },
                None => CpuHistoryMaterializationRequest::ReplayToCpu {
                    entry: e.identity.entry,
                    frozen,
                },
            }),
        })
    }
    pub(crate) fn accept_materialized(
        &mut self,
        expected: HistoryEntryId,
        exact: ExactCpuLayer,
    ) -> Result<(), CpuHandoverFailure> {
        let checked = (|| {
            let e = self
                .history
                .entries
                .iter()
                .find(|e| e.identity.entry == expected)
                .ok_or(HistoryError::IdentityMismatch)?;
            let d = exact.proof().descriptor();
            if matches!(e.backing, HistoryBacking::Cpu(_))
                || d.layer != e.resource_identity
                || d.allocation.context != self.history.context
                || d.completed_submission.context != self.history.context
                || e.resident
                    .as_ref()
                    .is_some_and(|r| r.allocation() != d.allocation)
            {
                return Err(HistoryError::IdentityMismatch);
            }
            check_pixels(exact.frame(), &e.resource_identity)
        })();
        if let Err(error) = checked {
            return Err(CpuHandoverFailure { error, exact });
        }
        let e = self
            .history
            .entries
            .iter_mut()
            .find(|e| e.identity.entry == expected)
            .unwrap();
        let (mut frame, _proof) = exact.into_parts();
        frame.layer_id = self.history.layer_id;
        // Release this raw recipe before materializing the next entry.
        e.backing = HistoryBacking::Cpu(Arc::new(frame));
        e.resource_identity = e.identity;
        retire(e, &mut self.retired);
        Ok(())
    }
    pub(crate) fn take_retired(&mut self) -> Vec<RetiredResident<T>> {
        std::mem::take(&mut self.retired)
    }
    pub(crate) fn finish(self) -> Result<CpuHistorySnapshot, Self> {
        if self.next_entry().is_some() || !self.retired.is_empty() {
            return Err(self);
        }
        let h = self.history;
        Ok(CpuHistorySnapshot {
            frame_scope: h.frame_scope,
            input: h.input,
            layer_id: h.layer_id,
            queue_epoch: h.queue_epoch,
            source_generation: h.source_generation,
            extent: h.extent,
            entries: h
                .entries
                .into_iter()
                .map(|e| {
                    let HistoryBacking::Cpu(frame) = e.backing else {
                        unreachable!("checked all slots")
                    };
                    (e.identity, frame)
                })
                .collect(),
        })
    }
}
#[cfg(test)]
#[path = "native_gpu_history_tests.rs"]
mod tests;
