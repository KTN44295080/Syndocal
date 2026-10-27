//! P-owned history policy (INTERFACE-01 section 5), independent of G's concrete
//! GPU types. Eviction transfers strong owners; it never releases a GPU budget
//! charge. Long-term recipe inputs and CPU backing are bounded by the separate
//! HistoryHostBudget (section 8.1). The only production constructors of
//! validated entries and the G type adapters live in the `adapter` child module.
use crate::frame_history_policy::{
    plan_history_update, HistoryMeta, HistoryPlan, NATIVE_HISTORY_CAPACITY,
};
#[cfg(test)]
use crate::native_gpu_contract::LayerOrigin;
use crate::native_gpu_contract::{
    GpuAllocationId, GpuContextId, HistoryEntryId, LayerResultIdentity, MaterializationDescriptor,
    NativeFrameId,
};
use crate::{VideoFrame, VideoPixelFormat};
use protocol::{VideoLayerId, VideoRenderInputKey};
use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

const CAPACITY: usize = NATIVE_HISTORY_CAPACITY;
const RESIDENT_CAPACITY: usize = 2;

/// Implemented by G's lease (adapter), never by a second allocator.
pub(crate) trait HistoryResident {
    fn identity(&self) -> &LayerResultIdentity;
    fn allocation(&self) -> GpuAllocationId;
}
/// Long-lived host allocations a frozen recipe keeps alive (its raw inputs).
pub(crate) trait HistoryHostFootprint {
    fn host_allocations(&self) -> Vec<HostAllocation>;
}
/// An affine validated CPU readback: pixels, completion descriptor and the
/// charge of that same CPU allocation (adapter: G's ValidatedCpuMaterialization).
/// The three parts are only split inside the history, into one owner.
pub(crate) trait MaterializedCpuLayer: Sized {
    type Charge;
    fn frame(&self) -> &VideoFrame;
    fn descriptor(&self) -> &MaterializationDescriptor;
    fn into_charged_parts(self) -> (VideoFrame, MaterializationDescriptor, Self::Charge);
}
/// An affine validated GPU materialization (adapter: G's MaterializedGpuLayer).
#[cfg(test)]
pub(crate) trait MaterializedResident: Sized {
    type Resident;
    fn descriptor(&self) -> MaterializationDescriptor;
    fn resident(&self) -> &Self::Resident;
    fn into_resident(self) -> Self::Resident;
}

/// One host allocation retained by a history owner. The key is the identity of
/// the shared allocation, so an alias held by several entries/layers is
/// counted once. The keepalive pins that identity for the charge's lifetime.
pub(crate) struct HostAllocation {
    key: usize,
    bytes: u64,
    managed: bool,
    keepalive: Arc<dyn Any + Send + Sync>,
}
impl HostAllocation {
    /// `managed`: the same bytes are also charged to G's 512MiB session ledger
    /// (reported as an alias, never added twice to process-unique bytes).
    pub(crate) fn shared_frame(frame: &Arc<VideoFrame>, managed: bool) -> Self {
        let keepalive: Arc<dyn Any + Send + Sync> = frame.clone();
        Self {
            key: Arc::as_ptr(frame) as usize,
            bytes: frame.data.capacity() as u64,
            managed,
            keepalive,
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct HistoryHostSnapshot {
    pub limit: u64,
    /// Unique retained bytes; an alias is counted once.
    pub unique_bytes: u64,
    /// Admitted for an owner that is being created (handover conversion).
    pub reserved_bytes: u64,
    /// Part of `unique_bytes` that G's session ledger also charges.
    pub managed_alias_bytes: u64,
    pub allocations: usize,
    pub charges: usize,
    pub high_water_bytes: u64,
}
impl HistoryHostSnapshot {
    /// Host bytes not already reported by G's managed ledger.
    #[cfg(test)]
    pub(crate) fn unmanaged_bytes(&self) -> u64 {
        self.unique_bytes - self.managed_alias_bytes
    }
}
struct HostRecord {
    bytes: u64,
    managed_refs: usize,
    refs: usize,
}
#[derive(Default)]
struct HostLedger {
    limit: u64,
    records: HashMap<usize, HostRecord>,
    unique_bytes: u64,
    reserved_bytes: u64,
    managed_alias_bytes: u64,
    charges: usize,
    high_water: u64,
}
impl HostLedger {
    fn admit(&mut self, bytes: u64) -> Result<(), HistoryError> {
        let live = self.unique_bytes + self.reserved_bytes;
        if bytes > self.limit.saturating_sub(live) {
            return Err(HistoryError::HostBudgetExceeded);
        }
        Ok(())
    }
    fn insert(&mut self, key: usize, bytes: u64, managed: bool) {
        self.charges += 1;
        match self.records.get_mut(&key) {
            Some(record) => {
                record.refs += 1;
                if managed {
                    if record.managed_refs == 0 {
                        self.managed_alias_bytes += record.bytes;
                    }
                    record.managed_refs += 1;
                }
            }
            None => {
                self.records.insert(
                    key,
                    HostRecord {
                        bytes,
                        managed_refs: usize::from(managed),
                        refs: 1,
                    },
                );
                self.unique_bytes += bytes;
                if managed {
                    self.managed_alias_bytes += bytes;
                }
            }
        }
        self.high_water = self.high_water.max(self.unique_bytes + self.reserved_bytes);
    }
}
/// INTERFACE-01 section 8.1: one bounded host ledger per output session for
/// long-term history owners (recipe inputs, post-FX CPU backing), separate
/// from G's 512MiB ledger. Locked only when an owner is admitted or dropped
/// (history commit/eviction/handover), never per pixel or per GPU phase.
#[derive(Clone)]
pub(crate) struct HistoryHostBudget {
    ledger: Arc<Mutex<HostLedger>>,
}
impl HistoryHostBudget {
    pub(crate) fn new(limit: u64) -> Self {
        Self {
            ledger: Arc::new(Mutex::new(HostLedger {
                limit,
                ..HostLedger::default()
            })),
        }
    }
    /// `layers` full histories plus one incoming or converting entry each.
    pub(crate) fn planning_limit(layers: usize, extent: (u32, u32)) -> Option<u64> {
        u64::from(extent.0)
            .checked_mul(u64::from(extent.1))?
            .checked_mul(4)?
            .checked_mul(u64::try_from(CAPACITY + 1).ok()?)?
            .checked_mul(u64::try_from(layers).ok()?)
    }
    fn lock(&self) -> MutexGuard<'_, HostLedger> {
        lock(&self.ledger)
    }
    /// All or nothing. Allocations already charged (aliases) add no bytes.
    pub(crate) fn charge_all(
        &self,
        allocations: Vec<HostAllocation>,
    ) -> Result<Vec<HistoryHostCharge>, HistoryError> {
        let mut ledger = self.lock();
        let mut fresh: Vec<usize> = Vec::new();
        let mut bytes = 0u64;
        for allocation in &allocations {
            if !ledger.records.contains_key(&allocation.key) && !fresh.contains(&allocation.key) {
                fresh.push(allocation.key);
                bytes = bytes.saturating_add(allocation.bytes);
            }
        }
        ledger.admit(bytes)?;
        Ok(allocations
            .into_iter()
            .map(|allocation| {
                ledger.insert(allocation.key, allocation.bytes, allocation.managed);
                HistoryHostCharge {
                    ledger: Arc::clone(&self.ledger),
                    key: allocation.key,
                    managed: allocation.managed,
                    _keepalive: allocation.keepalive,
                }
            })
            .collect())
    }
    /// Admit bytes for an owner that does not exist yet (affine materialization
    /// split), so a rejection can still return the complete owner.
    pub(crate) fn reserve(&self, bytes: u64) -> Result<HostReservation, HistoryError> {
        let mut ledger = self.lock();
        ledger.admit(bytes)?;
        ledger.reserved_bytes += bytes;
        ledger.high_water = ledger
            .high_water
            .max(ledger.unique_bytes + ledger.reserved_bytes);
        Ok(HostReservation {
            ledger: Arc::clone(&self.ledger),
            bytes,
        })
    }
    pub(crate) fn snapshot(&self) -> HistoryHostSnapshot {
        let ledger = self.lock();
        HistoryHostSnapshot {
            limit: ledger.limit,
            unique_bytes: ledger.unique_bytes,
            reserved_bytes: ledger.reserved_bytes,
            managed_alias_bytes: ledger.managed_alias_bytes,
            allocations: ledger.records.len(),
            charges: ledger.charges,
            high_water_bytes: ledger.high_water,
        }
    }
    fn owns(&self, charge: &HistoryHostCharge) -> bool {
        Arc::ptr_eq(&self.ledger, &charge.ledger)
    }
}
fn lock(ledger: &Mutex<HostLedger>) -> MutexGuard<'_, HostLedger> {
    ledger.lock().unwrap_or_else(PoisonError::into_inner)
}
/// Admitted bytes for one not-yet-created unique owner.
pub(crate) struct HostReservation {
    ledger: Arc<Mutex<HostLedger>>,
    bytes: u64,
}
impl HostReservation {
    /// Bind to the owner created for these bytes; the reservation is consumed.
    fn bind(mut self, frame: &Arc<VideoFrame>, managed: bool) -> HistoryHostCharge {
        let allocation = HostAllocation::shared_frame(frame, managed);
        let mut ledger = lock(&self.ledger);
        ledger.reserved_bytes -= std::mem::take(&mut self.bytes);
        ledger.insert(allocation.key, allocation.bytes, allocation.managed);
        drop(ledger);
        HistoryHostCharge {
            ledger: Arc::clone(&self.ledger),
            key: allocation.key,
            managed: allocation.managed,
            _keepalive: allocation.keepalive,
        }
    }
}
impl Drop for HostReservation {
    fn drop(&mut self) {
        if self.bytes != 0 {
            lock(&self.ledger).reserved_bytes -= self.bytes;
        }
    }
}
/// Host charge of one retained allocation; released when its last alias drops.
pub(crate) struct HistoryHostCharge {
    ledger: Arc<Mutex<HostLedger>>,
    key: usize,
    managed: bool,
    _keepalive: Arc<dyn Any + Send + Sync>,
}
impl HistoryHostCharge {
    /// Mark this same pinned allocation as managed for one active owner's
    /// lifetime. No new unique host bytes or ledger are introduced.
    fn managed_alias(&self) -> Self {
        let mut ledger = lock(&self.ledger);
        let bytes = ledger
            .records
            .get(&self.key)
            .expect("live host charge")
            .bytes;
        ledger.insert(self.key, bytes, true);
        drop(ledger);
        Self {
            ledger: Arc::clone(&self.ledger),
            key: self.key,
            managed: true,
            _keepalive: Arc::clone(&self._keepalive),
        }
    }
}
impl Drop for HistoryHostCharge {
    fn drop(&mut self) {
        let mut ledger = lock(&self.ledger);
        ledger.charges -= 1;
        let Some(record) = ledger.records.get_mut(&self.key) else {
            return;
        };
        record.refs -= 1;
        let released_managed_bytes = if self.managed {
            record.managed_refs -= 1;
            if record.managed_refs == 0 {
                record.bytes
            } else {
                0
            }
        } else {
            0
        };
        let remove = record.refs == 0;
        ledger.managed_alias_bytes -= released_managed_bytes;
        if remove {
            let record = ledger.records.remove(&self.key).expect("live record");
            ledger.unique_bytes -= record.bytes;
        }
    }
}

/// Where long-term CPU pixels came from. The G charge of the same CPU
/// allocation is owned here, so it lives exactly as long as the pixels.
pub(crate) enum CpuBackingOrigin<C> {
    /// Existing post-FX CPU queue entry: never minted or charged by G.
    Imported,
    /// G's validated readback of a GPU result or exact frozen replay.
    Materialized {
        descriptor: MaterializationDescriptor,
        charge: C,
    },
    /// G's fixed-clock CPU recovery output (canonical pixels, no recipe).
    Recovered { charge: C },
    /// A canonical queue already owns these pixels through a complete charged
    /// CPU owner. Retain that owner rather than extracting or recharging its G
    /// allocation. Re-imports flatten this alias instead of nesting backings.
    Canonical {
        owner: Arc<dyn Any + Send + Sync>,
        managed_allocation: Option<GpuAllocationId>,
    },
}
/// Post-FX CPU history backing: pixels, their G charge/proof and their host
/// charge as one owner. Fields drop in order, so pixels go before charges.
pub(crate) struct CpuHistoryBacking<C> {
    frame: Arc<VideoFrame>,
    origin: CpuBackingOrigin<C>,
    host: HistoryHostCharge,
}
impl<C> CpuHistoryBacking<C> {
    /// Read-only pixels. Clone the owning Arc<CpuHistoryBacking>, never the
    /// pixels alone, when a consumer must outlive this history.
    pub(crate) fn frame(&self) -> &VideoFrame {
        &self.frame
    }
    /// Move this whole backing into a canonical slot together with an alias of
    /// its exact pixel allocation. The receiver must retain both values in the
    /// same slot and move or release them together; this never extracts a G or
    /// host charge from its owner or creates another pixel allocation.
    pub(crate) fn into_canonical_parts(self: Arc<Self>) -> (Arc<VideoFrame>, Arc<Self>) {
        (Arc::clone(&self.frame), self)
    }
    pub(crate) fn origin(&self) -> &CpuBackingOrigin<C> {
        &self.origin
    }
}
pub(crate) enum HistoryBacking<R, C> {
    Replay(R),
    Cpu(Arc<CpuHistoryBacking<C>>),
}
struct Entry<R, T, C> {
    identity: LayerResultIdentity,
    // Original G receipt/lease identity; layer normalization changes metadata only.
    resource_identity: LayerResultIdentity,
    backing: HistoryBacking<R, C>,
    /// Host charges of the replay recipe's inputs; CPU backing owns its own.
    /// Declared after `backing`, so the recipe is released first.
    replay_host: Vec<HistoryHostCharge>,
    resident: Option<Arc<T>>,
}
impl<R, T, C> Entry<R, T, C> {
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
    #[cfg(test)]
    IsfFailed,
    Cancelled,
    InvalidPixels,
    HostBudgetExceeded,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryPhaseOutcome {
    IsfValid {
        composition_valid: bool,
    },
    /// G's fixed-clock recovery after an admitted ISF failure: CPU backing
    /// only, and the provisional GPU composition is never valid.
    RecoveredCpu,
    #[cfg(test)]
    IsfFailed,
    #[cfg(test)]
    Cancelled,
}
/// Sealed: fields are private and the only production constructors consume
/// actual validated G owners (adapter). Test fixtures assemble it directly.
pub(crate) struct ValidatedHistoryEntries<R, T, C> {
    frame: NativeFrameId,
    phase: HistoryPhaseOutcome,
    entries: Vec<Entry<R, T, C>>,
}
impl<R, T, C> ValidatedHistoryEntries<R, T, C> {
    #[cfg(test)]
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    #[cfg(test)]
    pub(crate) fn phase(&self) -> HistoryPhaseOutcome {
        self.phase
    }
    #[cfg(test)]
    pub(crate) fn metadata(&self) -> Vec<HistoryMeta> {
        self.entries.iter().map(Entry::meta).collect()
    }
    /// Same chain set after the composition/snapshot phase resolved. Only an
    /// ISF-valid batch has a composition outcome to record.
    pub(crate) fn record_composition(&mut self, valid: bool) -> Result<(), HistoryError> {
        match &mut self.phase {
            HistoryPhaseOutcome::IsfValid { composition_valid } => {
                *composition_valid = valid;
                Ok(())
            }
            _ => Err(HistoryError::StateMismatch),
        }
    }
}
/// Rejected affine inputs are returned intact, not silently dropped.
pub(crate) struct HistoryCommitFailure<R, T, C> {
    pub error: HistoryError,
    pub plan: HistoryPlan,
    pub entries: ValidatedHistoryEntries<R, T, C>,
}
pub(crate) struct RetiredResident<T> {
    pub entry: HistoryEntryId,
    owner: Arc<T>,
}
impl<T> RetiredResident<T> {
    pub(crate) fn into_parts(self) -> (HistoryEntryId, Arc<T>) {
        (self.entry, self.owner)
    }
    #[cfg(test)]
    pub(crate) fn resource(&self) -> &T {
        &self.owner
    }
}
#[cfg(test)]
pub(crate) struct HistoryReadPin<T> {
    pub entry: HistoryEntryId,
    owner: Arc<T>,
}
#[cfg(test)]
impl<T> HistoryReadPin<T> {
    pub(crate) fn resource(&self) -> &T {
        &self.owner
    }
}
pub(crate) struct HistoryCommit<T> {
    pub retired: Vec<RetiredResident<T>>,
    pub output_phases_valid: bool,
}
#[cfg(test)]
pub(crate) struct ResidentChange<T> {
    pub retired: Vec<RetiredResident<T>>,
}
#[cfg(test)]
pub(crate) struct MaterializationAttachFailure<M> {
    pub error: HistoryError,
    pub materialized: M,
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
        frame: &'a VideoFrame,
    },
}
pub(crate) struct HistorySelection<'a, R, T> {
    pub layer_id: VideoLayerId,
    pub selected_identity: Option<LayerResultIdentity>,
    pub selected: Option<HistoryMaterializationRequest<'a, R, T>>,
    /// Must be included in pre-execution allocation quote.
    pub cold_materialization_required: bool,
}
/// Fixed incarnation of one input's history (output, epochs, extent).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HistoryScope {
    pub frame_scope: NativeFrameId,
    pub input: VideoRenderInputKey,
    pub layer_id: VideoLayerId,
    pub queue_epoch: u64,
    pub source_generation: u64,
    pub extent: (u32, u32),
}
pub(crate) struct NativeHistory<R, T, C> {
    context: GpuContextId,
    frame_scope: NativeFrameId,
    input: VideoRenderInputKey,
    layer_id: VideoLayerId,
    queue_epoch: u64,
    source_generation: u64,
    extent: (u32, u32),
    revision: u64,
    last_sequence: Option<u64>,
    entries: Vec<Entry<R, T, C>>,
    host: HistoryHostBudget,
}
impl<R, T: HistoryResident, C> NativeHistory<R, T, C> {
    pub(crate) fn new(
        context: GpuContextId,
        scope: HistoryScope,
        host: HistoryHostBudget,
    ) -> Result<Self, HistoryError> {
        if scope.extent.0 == 0 || scope.extent.1 == 0 {
            return Err(HistoryError::InvalidPixels);
        }
        if scope.frame_scope.project_epoch != scope.input.project_render_epoch {
            return Err(HistoryError::IdentityMismatch);
        }
        Ok(Self {
            context,
            frame_scope: scope.frame_scope,
            input: scope.input,
            layer_id: scope.layer_id,
            queue_epoch: scope.queue_epoch,
            source_generation: scope.source_generation,
            extent: scope.extent,
            revision: 0,
            last_sequence: None,
            entries: Vec::new(),
            host,
        })
    }
    pub(crate) fn scope(&self) -> HistoryScope {
        HistoryScope {
            frame_scope: self.frame_scope,
            input: self.input,
            layer_id: self.layer_id,
            queue_epoch: self.queue_epoch,
            source_generation: self.source_generation,
            extent: self.extent,
        }
    }
    pub(crate) fn metadata(&self) -> Vec<HistoryMeta> {
        self.entries.iter().map(Entry::meta).collect()
    }
    #[cfg(test)]
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
                        HistoryBacking::Cpu(backing) => {
                            HistoryMaterializationRequest::UploadCpuPostFx {
                                entry: id,
                                frame: backing.frame(),
                            }
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
    #[cfg(test)]
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
    fn check_incoming(
        &self,
        e: &Entry<R, T, C>,
        phase: HistoryPhaseOutcome,
    ) -> Result<(), HistoryError> {
        self.check_identity(&e.identity)?;
        if e.resource_identity != e.identity {
            return Err(HistoryError::IdentityMismatch);
        }
        if let Some(r) = &e.resident {
            if r.identity() != &e.identity || r.allocation().context != self.context {
                return Err(HistoryError::IdentityMismatch);
            }
        }
        if e.replay_host.iter().any(|charge| !self.host.owns(charge)) {
            return Err(HistoryError::IdentityMismatch);
        }
        match &e.backing {
            HistoryBacking::Cpu(backing) => {
                if !self.host.owns(&backing.host) {
                    return Err(HistoryError::IdentityMismatch);
                }
                check_pixels(&backing.frame, &e.resource_identity)?;
                if phase == HistoryPhaseOutcome::RecoveredCpu
                    && (e.resident.is_some()
                        || !matches!(backing.origin, CpuBackingOrigin::Recovered { .. }))
                {
                    return Err(HistoryError::StateMismatch);
                }
            }
            HistoryBacking::Replay(_) if phase == HistoryPhaseOutcome::RecoveredCpu => {
                // A failed GPU phase never becomes a successful recipe.
                return Err(HistoryError::StateMismatch);
            }
            HistoryBacking::Replay(_) => {}
        }
        Ok(())
    }
    pub(crate) fn commit_validated(
        &mut self,
        plan: HistoryPlan,
        mut entries: ValidatedHistoryEntries<R, T, C>,
    ) -> Result<HistoryCommit<T>, HistoryCommitFailure<R, T, C>> {
        let checked = (|| {
            self.check_plan(&plan)?;
            let output_phases_valid = match entries.phase {
                HistoryPhaseOutcome::IsfValid { composition_valid } => composition_valid,
                HistoryPhaseOutcome::RecoveredCpu => false,
                #[cfg(test)]
                HistoryPhaseOutcome::IsfFailed => return Err(HistoryError::IsfFailed),
                #[cfg(test)]
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
                if e.meta() != *meta || e.identity.frame != entries.frame {
                    return Err(HistoryError::IdentityMismatch);
                }
                self.check_incoming(e, entries.phase)?;
            }
            Ok((next_revision, output_phases_valid))
        })();
        let (next_revision, output_phases_valid) = match checked {
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
        // Evicted recipes/CPU backing and their host charges drop here; any
        // GPU owner moved to `retired` keeps its G charge until retirement.
        drop(pool);
        self.entries = survivors;
        self.trim_residents(plan.selected(), &mut retired);
        self.revision = next_revision;
        if let Some(last) = plan.incoming().last() {
            self.last_sequence = Some(last.id.sequence);
        }
        Ok(HistoryCommit {
            retired,
            output_phases_valid,
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
    #[cfg(test)]
    pub(crate) fn install_materialized<M>(
        &mut self,
        expected: HistoryEntryId,
        materialized: M,
    ) -> Result<ResidentChange<T>, MaterializationAttachFailure<M>>
    where
        M: MaterializedResident<Resident = T>,
    {
        let checked = (|| {
            let e = self
                .entries
                .iter()
                .find(|e| e.identity.entry == expected)
                .ok_or(HistoryError::IdentityMismatch)?;
            let d = materialized.descriptor();
            let resident = materialized.resident();
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
                    materialized,
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
        e.resident = Some(Arc::new(materialized.into_resident()));
        self.trim_residents(Some(expected), &mut retired);
        self.revision = revision;
        Ok(ResidentChange { retired })
    }
    pub(crate) fn begin_cpu_handover(self) -> CpuHistoryHandover<R, T, C> {
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
    #[cfg(test)]
    pub(crate) fn import_cpu_history(
        snapshot: CpuHistorySnapshot<C>,
        context: GpuContextId,
    ) -> Result<Self, HistoryError> {
        let mut h = Self::new(context, snapshot.scope, snapshot.host)?;
        if snapshot.entries.len() > CAPACITY {
            return Err(HistoryError::InvalidPolicy);
        }
        for (mut identity, backing) in snapshot.entries {
            identity.context = context;
            identity.origin = LayerOrigin::ImportedCpu;
            h.check_identity(&identity)?;
            if !h.host.owns(&backing.host) {
                return Err(HistoryError::IdentityMismatch);
            }
            let resource_identity = LayerResultIdentity {
                layer_id: backing.frame.layer_id,
                ..identity
            };
            check_pixels(&backing.frame, &resource_identity)?;
            h.push_imported(identity, resource_identity, backing);
        }
        plan_history_update(&h.metadata(), &[], CAPACITY, 0)
            .map_err(|_| HistoryError::InvalidPolicy)?;
        Ok(h)
    }
    /// CPU-to-native entry from the canonical CPU queue (section 5.4): every
    /// post-FX survivor moves as Imported CPU backing; no clock or recipe is
    /// invented. The caller mints fresh entry IDs in canonical ascending order.
    #[cfg(test)]
    pub(crate) fn import_canonical_queue(
        context: GpuContextId,
        scope: HistoryScope,
        host: HistoryHostBudget,
        frames: Vec<(HistoryEntryId, Arc<VideoFrame>)>,
    ) -> Result<Self, HistoryError> {
        let mut h = Self::new(context, scope, host)?;
        if frames.len() > CAPACITY {
            return Err(HistoryError::InvalidPolicy);
        }
        let mut checked = Vec::with_capacity(frames.len());
        for (entry, frame) in frames {
            let identity = LayerResultIdentity {
                context,
                frame: h.frame_scope,
                entry,
                layer_id: h.layer_id,
                source_generation: h.source_generation,
                content_sequence: entry.sequence,
                pts_ms: frame.pts_ms,
                duration_ms: frame.duration_ms,
                width: frame.width,
                height: frame.height,
                format: frame.format,
                origin: LayerOrigin::ImportedCpu,
            };
            h.check_identity(&identity)?;
            let resource_identity = LayerResultIdentity {
                layer_id: frame.layer_id,
                ..identity
            };
            check_pixels(&frame, &resource_identity)?;
            checked.push((identity, resource_identity, frame));
        }
        let metadata: Vec<_> = checked
            .iter()
            .map(|(i, _, _)| HistoryMeta {
                id: i.entry,
                pts_ms: i.pts_ms,
                duration_ms: i.duration_ms,
            })
            .collect();
        plan_history_update(&metadata, &[], CAPACITY, 0)
            .map_err(|_| HistoryError::InvalidPolicy)?;
        let charges = h.host.charge_all(
            checked
                .iter()
                .map(|(_, _, frame)| HostAllocation::shared_frame(frame, false))
                .collect(),
        )?;
        for ((identity, resource_identity, frame), host) in checked.into_iter().zip(charges) {
            let backing = Arc::new(CpuHistoryBacking {
                frame,
                origin: CpuBackingOrigin::Imported,
                host,
            });
            h.push_imported(identity, resource_identity, backing);
        }
        Ok(h)
    }
    fn push_imported(
        &mut self,
        mut identity: LayerResultIdentity,
        resource_identity: LayerResultIdentity,
        backing: Arc<CpuHistoryBacking<C>>,
    ) {
        identity.layer_id = self.layer_id;
        self.last_sequence = Some(self.last_sequence.unwrap_or(0).max(identity.entry.sequence));
        self.entries.push(Entry {
            identity,
            resource_identity,
            backing: HistoryBacking::Cpu(backing),
            replay_host: Vec::new(),
            resident: None,
        });
    }
}
fn retire<R, T, C>(e: &mut Entry<R, T, C>, retired: &mut Vec<RetiredResident<T>>) {
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
/// Complete, published CPU history: all survivors, each with its whole owner.
pub(crate) struct CpuHistorySnapshot<C> {
    scope: HistoryScope,
    host: HistoryHostBudget,
    entries: Vec<(LayerResultIdentity, Arc<CpuHistoryBacking<C>>)>,
}
impl<C> CpuHistorySnapshot<C> {
    /// Verify every sealed backing belongs to this host ledger before I moves
    /// any whole owner into the canonical queue.
    pub(crate) fn validate_host_accounting(&self) -> Result<(), HistoryError> {
        if self
            .entries
            .iter()
            .any(|(_, backing)| !self.host.owns(&backing.host))
        {
            return Err(HistoryError::IdentityMismatch);
        }
        let snapshot = self.host.snapshot();
        if snapshot
            .unique_bytes
            .checked_add(snapshot.reserved_bytes)
            .is_none_or(|bytes| bytes > snapshot.limit)
            || snapshot.managed_alias_bytes > snapshot.unique_bytes
        {
            return Err(HistoryError::HostBudgetExceeded);
        }
        Ok(())
    }

    pub(crate) fn scope(&self) -> HistoryScope {
        self.scope
    }
    /// Identity carries the canonical queue layer. Shared CPU backing retains
    /// its diagnostic tag; the CPU adapter publishes by queue identity.
    pub(crate) fn entries(
        &self,
    ) -> impl Iterator<Item = (&LayerResultIdentity, &Arc<CpuHistoryBacking<C>>)> {
        self.entries.iter().map(|(i, b)| (i, b))
    }
    /// For the canonical CPU queue: it must retain each whole owner (pixels
    /// with their charges), not a pixel Arc split from it.
    pub(crate) fn into_entries(self) -> Vec<(LayerResultIdentity, Arc<CpuHistoryBacking<C>>)> {
        self.entries
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
/// Full-history barrier (section 5.4). Holds a mixed ledger of completed CPU
/// backing and unmaterialized recipe/resident owners until every survivor is
/// CPU. A cancelled or failed step leaves it intact; nothing is published.
pub(crate) struct CpuHistoryHandover<R, T, C> {
    history: NativeHistory<R, T, C>,
    retired: Vec<RetiredResident<T>>,
}
pub(crate) struct CpuHandoverFailure<M> {
    pub error: HistoryError,
    pub materialized: M,
}
impl<R, T: HistoryResident, C> CpuHistoryHandover<R, T, C> {
    #[cfg(test)]
    pub(crate) fn len(&self) -> usize {
        self.history.entries.len()
    }
    /// Survivors still owned as recipe/resident (not yet CPU).
    pub(crate) fn pending(&self) -> usize {
        self.history
            .entries
            .iter()
            .filter(|e| matches!(e.backing, HistoryBacking::Replay(_)))
            .count()
    }
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
    /// Converts one entry. Pixels, descriptor and G charge become one CPU
    /// owner; only then are this entry's recipe, its host charges and its
    /// resident released. Any rejection returns the complete owner unsplit.
    pub(crate) fn accept_materialized<M>(
        &mut self,
        expected: HistoryEntryId,
        materialized: M,
    ) -> Result<(), CpuHandoverFailure<M>>
    where
        M: MaterializedCpuLayer<Charge = C>,
    {
        let checked = (|| {
            let e = self
                .history
                .entries
                .iter()
                .find(|e| e.identity.entry == expected)
                .ok_or(HistoryError::IdentityMismatch)?;
            let d = materialized.descriptor();
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
            check_pixels(materialized.frame(), &e.resource_identity)?;
            self.history
                .host
                .reserve(materialized.frame().data.capacity() as u64)
        })();
        let reservation = match checked {
            Ok(reservation) => reservation,
            Err(error) => {
                return Err(CpuHandoverFailure {
                    error,
                    materialized,
                })
            }
        };
        let layer_id = self.history.layer_id;
        let e = self
            .history
            .entries
            .iter_mut()
            .find(|e| e.identity.entry == expected)
            .unwrap();
        let (mut frame, descriptor, charge) = materialized.into_charged_parts();
        frame.layer_id = layer_id;
        let frame = Arc::new(frame);
        let host = reservation.bind(&frame, true);
        // Release this raw recipe (and its host charges) before the next entry.
        e.backing = HistoryBacking::Cpu(Arc::new(CpuHistoryBacking {
            frame,
            origin: CpuBackingOrigin::Materialized { descriptor, charge },
            host,
        }));
        e.replay_host.clear();
        e.resource_identity = e.identity;
        retire(e, &mut self.retired);
        Ok(())
    }
    pub(crate) fn take_retired(&mut self) -> Vec<RetiredResident<T>> {
        std::mem::take(&mut self.retired)
    }
    pub(crate) fn finish(self) -> Result<CpuHistorySnapshot<C>, Self> {
        if self.next_entry().is_some() || !self.retired.is_empty() {
            return Err(self);
        }
        let scope = self.history.scope();
        let NativeHistory { entries, host, .. } = self.history;
        Ok(CpuHistorySnapshot {
            scope,
            host,
            entries: entries
                .into_iter()
                .map(|e| {
                    let HistoryBacking::Cpu(backing) = e.backing else {
                        unreachable!("checked all slots")
                    };
                    (e.identity, backing)
                })
                .collect(),
        })
    }
}

#[path = "native_gpu_history_adapter.rs"]
pub(crate) mod adapter;

#[cfg(test)]
#[path = "native_gpu_history_tests.rs"]
mod tests;
