//! One session's managed-allocation ledger (INTERFACE-01 section 8).
//!
//! Tokens describe application-managed capacity, not driver/private RSS. A
//! resource owner must keep its charge alive. Dropping a charge or reservation
//! never refunds the ledger: only the broker may acknowledge GPU completion,
//! and a reap additionally requires every resource/read owner to be gone.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Weak};

use crate::native_gpu_contract::{
    GpuAllocationId, GpuContextId, GpuPhase, GpuSubmissionId, NativeFrameId, NativeGpuDecline,
    NativeGpuFault, NativeGpuFaultKind,
};

pub(crate) const NATIVE_MANAGED_LIMIT: u64 = 512 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(usize)]
pub(crate) enum ChargeKind {
    IsfScratch,
    ResidentLayer,
    CompositionBuffer,
    MappingBuffer,
    SnapshotReadback,
    QueueUploadStaging,
    ActiveCpuInput,
    CpuSnapshot,
    CpuLastValid,
    CachedEffectResource,
}

const KIND_COUNT: usize = 10;

#[derive(Clone, Debug)]
struct PlannedAllocation {
    plan: Arc<()>,
    slot: usize,
    kind: ChargeKind,
    capacity: u64,
}

impl PartialEq for PlannedAllocation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.plan, &other.plan)
            && self.slot == other.slot
            && self.kind == other.kind
            && self.capacity == other.capacity
    }
}
impl Eq for PlannedAllocation {}

/// Exact new allocations plus aliases of allocations already in the ledger.
/// `merge` permits G and P to contribute to one reservation before execution.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeAllocationPlan {
    identity: Arc<()>,
    frame: NativeFrameId,
    new: Vec<PlannedAllocation>,
    existing: Vec<GpuAllocationId>,
    requested: u64,
}

impl NativeAllocationPlan {
    pub(crate) fn new(frame: NativeFrameId) -> Self {
        Self {
            identity: Arc::new(()),
            frame,
            new: Vec::new(),
            existing: Vec::new(),
            requested: 0,
        }
    }

    pub(crate) fn add_new(
        &mut self,
        kind: ChargeKind,
        capacity: u64,
    ) -> Result<(), NativeGpuDecline> {
        if capacity == 0 {
            return Err(NativeGpuDecline::UnsupportedExtent);
        }
        let requested =
            self.requested
                .checked_add(capacity)
                .ok_or(NativeGpuDecline::BudgetExceeded {
                    live: 0,
                    requested: u64::MAX,
                    limit: NATIVE_MANAGED_LIMIT,
                })?;
        self.new.push(PlannedAllocation {
            plan: Arc::clone(&self.identity),
            slot: self.new.len(),
            kind,
            capacity,
        });
        self.requested = requested;
        Ok(())
    }

    pub(crate) fn add_existing(&mut self, id: GpuAllocationId) {
        if !self.existing.contains(&id) {
            self.existing.push(id);
        }
    }

    pub(crate) fn merge(&mut self, other: &Self) -> Result<(), NativeGpuDecline> {
        if self.frame != other.frame {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        let additions: Vec<_> = other
            .new
            .iter()
            .filter(|entry| !self.new.contains(entry))
            .cloned()
            .collect();
        if additions.iter().any(|entry| {
            self.new
                .iter()
                .any(|prior| Arc::ptr_eq(&entry.plan, &prior.plan) && entry.slot == prior.slot)
        }) {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        let additional_bytes = additions.iter().map(|entry| entry.capacity).sum::<u64>();
        let requested = self.requested.checked_add(additional_bytes).ok_or(
            NativeGpuDecline::BudgetExceeded {
                live: 0,
                requested: u64::MAX,
                limit: NATIVE_MANAGED_LIMIT,
            },
        )?;
        self.new.extend(additions);
        for id in &other.existing {
            self.add_existing(*id);
        }
        self.requested = requested;
        Ok(())
    }

    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    pub(crate) fn requested_bytes(&self) -> u64 {
        self.requested
    }
}

#[derive(Debug)]
pub(crate) struct BudgetQuote {
    ledger: Arc<()>,
    context: GpuContextId,
    revision: u64,
    plan: NativeAllocationPlan,
}

impl BudgetQuote {
    pub(crate) fn context(&self) -> GpuContextId {
        self.context
    }
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.plan.frame
    }
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    pub(crate) fn matches_plan(&self, plan: &NativeAllocationPlan) -> bool {
        Arc::ptr_eq(&self.plan.identity, &plan.identity) && self.plan == *plan
    }

    /// Admission of G's prepared sub-plan against the complete G/P quote. New
    /// allocation identity includes its private originating plan and slot; equal
    /// byte counts from a separately constructed plan do not establish coverage.
    pub(crate) fn covers_plan(&self, plan: &NativeAllocationPlan) -> bool {
        self.plan.frame == plan.frame
            && plan.new.iter().all(|entry| self.plan.new.contains(entry))
            && plan
                .existing
                .iter()
                .all(|id| self.plan.existing.contains(id))
    }
}

/// Affine reservation. Its authoritative unbound entries remain in the ledger
/// even if a caller abandons this token, so cancellation cannot hide capacity.
#[derive(Debug)]
pub(crate) struct FrameReservation {
    ledger: Arc<()>,
    id: u64,
    context: GpuContextId,
    frame: NativeFrameId,
    remaining: u64,
}

impl FrameReservation {
    pub(crate) fn context(&self) -> GpuContextId {
        self.context
    }
    pub(crate) fn frame(&self) -> NativeFrameId {
        self.frame
    }
    pub(crate) fn capacity_bytes(&self) -> u64 {
        self.remaining
    }
}

#[derive(Debug)]
struct AllocationLiveness {
    id: GpuAllocationId,
    kind: ChargeKind,
    capacity: u64,
}

/// Keep this inside the actual GPU/CPU allocation owner, never beside a raw
/// handle whose lifetime may outlive it. Aliases use pins of this same charge.
#[derive(Debug)]
pub(crate) struct AllocationCharge {
    live: Arc<AllocationLiveness>,
}

impl AllocationCharge {
    pub(crate) fn id(&self) -> GpuAllocationId {
        self.live.id
    }
    pub(crate) fn kind(&self) -> ChargeKind {
        self.live.kind
    }
    pub(crate) fn capacity(&self) -> u64 {
        self.live.capacity
    }
    pub(crate) fn pin(&self) -> AllocationPin {
        AllocationPin {
            live: Arc::clone(&self.live),
        }
    }
}

#[derive(Debug)]
pub(crate) struct AllocationPin {
    live: Arc<AllocationLiveness>,
}

impl AllocationPin {
    pub(crate) fn id(&self) -> GpuAllocationId {
        self.live.id
    }
}

struct AllocationRecord {
    live: Weak<AllocationLiveness>,
    kind: ChargeKind,
    capacity: u64,
    last_use: Option<GpuSubmissionId>,
    retired: bool,
    quarantined: bool,
    reap_issued: bool,
}

struct ReservationRecord {
    root: u64,
    context: GpuContextId,
    frame: NativeFrameId,
    entries: Vec<PlannedAllocation>,
    existing: Vec<GpuAllocationId>,
}

/// Minted only after GPU completion and the final strong owner disappeared.
#[derive(Debug)]
pub(crate) struct ReapedAllocationReceipt {
    ledger: Arc<()>,
    id: GpuAllocationId,
    capacity: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct BudgetKindSnapshot {
    pub active_bytes: u64,
    pub retired_bytes: u64,
    pub quarantined_bytes: u64,
    pub reserved_bytes: u64,
    pub high_water_bytes: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NativeGpuBudgetSnapshot {
    pub context: GpuContextId,
    pub revision: u64,
    pub allocated_bytes: u64,
    pub reserved_bytes: u64,
    pub high_water_bytes: u64,
    pub allocation_count: usize,
    pub reservation_count: usize,
    pub by_kind: [BudgetKindSnapshot; KIND_COUNT],
}

impl NativeGpuBudgetSnapshot {
    pub(crate) fn managed_bytes(&self) -> u64 {
        self.allocated_bytes + self.reserved_bytes
    }
    pub(crate) fn for_kind(&self, kind: ChargeKind) -> &BudgetKindSnapshot {
        &self.by_kind[kind as usize]
    }
}

pub(crate) struct NativeGpuBudget {
    identity: Arc<()>,
    context: GpuContextId,
    revision: u64,
    allocation_sequence: u64,
    reservation_sequence: u64,
    unbound_ids: HashSet<GpuAllocationId>,
    allocations: HashMap<GpuAllocationId, AllocationRecord>,
    reservations: HashMap<u64, ReservationRecord>,
    completed: HashMap<GpuContextId, u64>,
    high_water: u64,
    kind_high_water: [u64; KIND_COUNT],
    quarantined: bool,
}

impl NativeGpuBudget {
    pub(crate) fn new(context: GpuContextId) -> Self {
        Self {
            identity: Arc::new(()),
            context,
            revision: 0,
            allocation_sequence: 0,
            reservation_sequence: 0,
            unbound_ids: HashSet::new(),
            allocations: HashMap::new(),
            reservations: HashMap::new(),
            completed: HashMap::new(),
            high_water: 0,
            kind_high_water: [0; KIND_COUNT],
            quarantined: false,
        }
    }

    fn validate_plan(&self, plan: &NativeAllocationPlan) -> Result<(), NativeGpuDecline> {
        if self.quarantined
            || plan.existing.iter().any(|id| {
                id.context != self.context
                    || self.allocations.get(id).is_none_or(|a| {
                        a.reap_issued || a.retired || a.quarantined || a.live.strong_count() == 0
                    })
            })
        {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        let live = self.snapshot().managed_bytes();
        if plan.requested > NATIVE_MANAGED_LIMIT - live {
            return Err(NativeGpuDecline::BudgetExceeded {
                live,
                requested: plan.requested,
                limit: NATIVE_MANAGED_LIMIT,
            });
        }
        if self.revision == u64::MAX || self.reservation_sequence == u64::MAX {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        Ok(())
    }

    pub(crate) fn quote(
        &self,
        plan: &NativeAllocationPlan,
    ) -> Result<BudgetQuote, NativeGpuDecline> {
        self.validate_plan(plan)?;
        Ok(BudgetQuote {
            ledger: Arc::clone(&self.identity),
            context: self.context,
            revision: self.revision,
            plan: plan.clone(),
        })
    }

    pub(crate) fn reserve(
        &mut self,
        quote: BudgetQuote,
    ) -> Result<FrameReservation, NativeGpuDecline> {
        if !Arc::ptr_eq(&self.identity, &quote.ledger)
            || quote.context != self.context
            || quote.revision != self.revision
            || self.quarantined
        {
            return Err(NativeGpuDecline::QuoteExpired);
        }
        // Recheck capacity/identities, rather than trusting a detached quote.
        self.validate_plan(&quote.plan)?;
        let id = self
            .reservation_sequence
            .checked_add(1)
            .ok_or(NativeGpuDecline::QuoteExpired)?;
        let revision = self
            .revision
            .checked_add(1)
            .ok_or(NativeGpuDecline::QuoteExpired)?;
        let reservation = FrameReservation {
            ledger: Arc::clone(&self.identity),
            id,
            context: self.context,
            frame: quote.plan.frame,
            remaining: quote.plan.requested,
        };
        self.reservations.insert(
            id,
            ReservationRecord {
                root: id,
                context: self.context,
                frame: quote.plan.frame,
                entries: quote.plan.new,
                existing: quote.plan.existing,
            },
        );
        self.reservation_sequence = id;
        self.revision = revision;
        self.update_high_water();
        Ok(reservation)
    }

    pub(crate) fn validate_reservation(
        &self,
        reservation: &FrameReservation,
        frame: NativeFrameId,
    ) -> Result<(), NativeGpuFault> {
        let valid = Arc::ptr_eq(&self.identity, &reservation.ledger)
            && reservation.context == self.context
            && reservation.frame == frame
            && self
                .reservations
                .get(&reservation.id)
                .is_some_and(|record| {
                    record.context == reservation.context
                        && record.frame == frame
                        && record.entries.iter().map(|e| e.capacity).sum::<u64>()
                            == reservation.remaining
                });
        if !valid {
            return Err(self.fault(
                Some(frame),
                NativeGpuFaultKind::IdentityMismatch,
                "foreign or stale frame reservation",
            ));
        }
        if self.quarantined {
            return Err(self.fault(
                Some(frame),
                NativeGpuFaultKind::StateMismatch,
                "allocation ledger is quarantined",
            ));
        }
        Ok(())
    }

    pub(crate) fn next_allocation_id(&mut self) -> Result<GpuAllocationId, NativeGpuFault> {
        let next = self.allocation_sequence.checked_add(1).ok_or_else(|| {
            self.fault(
                None,
                NativeGpuFaultKind::SequenceExhausted,
                "allocation sequence exhausted",
            )
        })?;
        self.allocation_sequence = next;
        let id = GpuAllocationId {
            context: self.context,
            sequence: next,
        };
        self.unbound_ids.insert(id);
        Ok(id)
    }

    /// A whole frame owns the reservation family; a P sub-reservation cannot
    /// masquerade as a second independently admitted frame.
    pub(crate) fn validate_frame_reservation(
        &self,
        reservation: &FrameReservation,
        frame: NativeFrameId,
    ) -> Result<(), NativeGpuFault> {
        self.validate_reservation(reservation, frame)?;
        if self.reservations[&reservation.id].root != reservation.id {
            return Err(self.fault(
                Some(frame),
                NativeGpuFaultKind::StateMismatch,
                "a sub-reservation cannot admit a frame",
            ));
        }
        Ok(())
    }

    /// Revalidate a prepared G/P sub-plan against the reservation actually used
    /// to admit this frame. A same-sized replacement quote is not authority to
    /// execute the original plan, and already-bound slots cannot be reused.
    /// Sibling sub-reservations remain part of this root's original admission.
    pub(crate) fn validate_reserved_plan(
        &self,
        reservation: &FrameReservation,
        plan: &NativeAllocationPlan,
    ) -> Result<(), NativeGpuFault> {
        self.validate_frame_reservation(reservation, plan.frame)?;
        let root = &self.reservations[&reservation.id];
        let covered = plan.new.iter().all(|entry| {
            self.reservations
                .values()
                .any(|record| record.root == reservation.id && record.entries.contains(entry))
        }) && plan.existing.iter().all(|id| {
            root.existing.contains(id)
                && id.context == self.context
                && self.allocations.get(id).is_some_and(|allocation| {
                    !allocation.retired
                        && !allocation.quarantined
                        && !allocation.reap_issued
                        && allocation.live.strong_count() > 0
                })
        });
        if !covered {
            return Err(self.fault(
                Some(plan.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "prepared allocation plan is not unconsumed in this frame admission",
            ));
        }
        Ok(())
    }

    /// A phase may spend only the sub-reservation split from its admitted
    /// frame root, not another independently reserved plan for the same frame.
    pub(crate) fn validate_subreservation(
        &self,
        root: &FrameReservation,
        child: &FrameReservation,
    ) -> Result<(), NativeGpuFault> {
        self.validate_frame_reservation(root, root.frame)?;
        self.validate_reservation(child, root.frame)?;
        if child.id == root.id || self.reservations[&child.id].root != root.id {
            return Err(self.fault(
                Some(root.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "sub-reservation belongs to another frame admission",
            ));
        }
        Ok(())
    }

    /// Called before the physical allocation/queue write. A larger capacity or
    /// a different charge kind cannot consume an approximately matching plan.
    pub(crate) fn bind_allocation(
        &mut self,
        reservation: &mut FrameReservation,
        id: GpuAllocationId,
        kind: ChargeKind,
        actual_capacity_bytes: u64,
    ) -> Result<AllocationCharge, NativeGpuFault> {
        self.validate_reservation(reservation, reservation.frame)?;
        if id.context != self.context
            || !self.unbound_ids.contains(&id)
            || self.allocations.contains_key(&id)
        {
            return Err(self.fault(
                Some(reservation.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "unknown or duplicate allocation identity",
            ));
        }
        let position = self.reservations[&reservation.id]
            .entries
            .iter()
            .position(|entry| entry.kind == kind && entry.capacity == actual_capacity_bytes)
            .ok_or_else(|| {
                self.fault(
                    Some(reservation.frame),
                    NativeGpuFaultKind::StateMismatch,
                    "allocation capacity was not reserved",
                )
            })?;
        let revision = self.next_revision(Some(reservation.frame))?;
        let live = Arc::new(AllocationLiveness {
            id,
            kind,
            capacity: actual_capacity_bytes,
        });
        self.reservations
            .get_mut(&reservation.id)
            .expect("validated reservation")
            .entries
            .remove(position);
        reservation.remaining -= actual_capacity_bytes;
        self.unbound_ids.remove(&id);
        self.allocations.insert(
            id,
            AllocationRecord {
                live: Arc::downgrade(&live),
                kind,
                capacity: actual_capacity_bytes,
                last_use: None,
                retired: false,
                quarantined: false,
                reap_issued: false,
            },
        );
        self.revision = revision;
        Ok(AllocationCharge { live })
    }

    /// Transfer an exact subset, once, without granting additional capacity.
    /// A composition reservation is a P wrapper around this same affine token.
    pub(crate) fn split_reservation(
        &mut self,
        parent: &mut FrameReservation,
        plan: &NativeAllocationPlan,
    ) -> Result<FrameReservation, NativeGpuFault> {
        self.validate_reservation(parent, plan.frame)?;
        if plan.new.is_empty() {
            return Err(self.fault(
                Some(plan.frame),
                NativeGpuFaultKind::StateMismatch,
                "empty sub-reservation",
            ));
        }
        let record = &self.reservations[&parent.id];
        let root = record.root;
        if plan.existing.iter().any(|id| !record.existing.contains(id)) {
            return Err(self.fault(
                Some(plan.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "sub-plan contains an unreserved alias",
            ));
        }
        let mut remaining = record.entries.clone();
        for entry in &plan.new {
            let index = remaining
                .iter()
                .position(|other| other == entry)
                .ok_or_else(|| {
                    self.fault(
                        Some(plan.frame),
                        NativeGpuFaultKind::StateMismatch,
                        "sub-plan capacity already consumed or not reserved",
                    )
                })?;
            remaining.remove(index);
        }
        let id = self.reservation_sequence.checked_add(1).ok_or_else(|| {
            self.fault(
                Some(plan.frame),
                NativeGpuFaultKind::SequenceExhausted,
                "reservation sequence exhausted",
            )
        })?;
        let revision = self.next_revision(Some(plan.frame))?;
        self.reservations
            .get_mut(&parent.id)
            .expect("validated parent")
            .entries = remaining;
        parent.remaining -= plan.requested;
        self.reservations.insert(
            id,
            ReservationRecord {
                root,
                context: self.context,
                frame: plan.frame,
                entries: plan.new.clone(),
                existing: plan.existing.clone(),
            },
        );
        self.reservation_sequence = id;
        self.revision = revision;
        Ok(FrameReservation {
            ledger: Arc::clone(&self.identity),
            id,
            context: self.context,
            frame: plan.frame,
            remaining: plan.requested,
        })
    }

    /// Only unused reservations are refunded. Bound or pending allocations are
    /// unaffected; a fault path must separately retire their actual owners.
    pub(crate) fn cancel_reservation(
        &mut self,
        reservation: FrameReservation,
    ) -> Result<(), NativeGpuFault> {
        if !Arc::ptr_eq(&self.identity, &reservation.ledger)
            || !self
                .reservations
                .get(&reservation.id)
                .is_some_and(|r| r.context == reservation.context && r.frame == reservation.frame)
        {
            return Err(self.fault(
                Some(reservation.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "unknown reservation cancellation",
            ));
        }
        let revision = self.next_revision(Some(reservation.frame))?;
        self.reservations.remove(&reservation.id);
        self.revision = revision;
        Ok(())
    }

    /// The frame registry owns the root token and uses this at its terminal
    /// boundary. Escaped/dropped child tokens cannot strand unused capacity or
    /// remain usable after the frame ends. Bound allocations are not refunded.
    pub(crate) fn cancel_reservation_tree(
        &mut self,
        reservation: FrameReservation,
    ) -> Result<(), NativeGpuFault> {
        if !Arc::ptr_eq(&self.identity, &reservation.ledger)
            || !self
                .reservations
                .get(&reservation.id)
                .is_some_and(|r| r.context == reservation.context && r.frame == reservation.frame)
        {
            return Err(self.fault(
                Some(reservation.frame),
                NativeGpuFaultKind::IdentityMismatch,
                "unknown reservation family cancellation",
            ));
        }
        if self.reservations[&reservation.id].root != reservation.id {
            return Err(self.fault(
                Some(reservation.frame),
                NativeGpuFaultKind::StateMismatch,
                "only the frame's root reservation may cancel the family",
            ));
        }
        let revision = self.next_revision(Some(reservation.frame))?;
        self.reservations
            .retain(|_, record| record.root != reservation.id);
        self.revision = revision;
        Ok(())
    }

    pub(crate) fn record_use(
        &mut self,
        id: GpuAllocationId,
        submission: GpuSubmissionId,
    ) -> Result<(), NativeGpuFault> {
        if submission.context != id.context
            || id.context != self.context
            || submission.sequence == 0
        {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::IdentityMismatch,
                "foreign allocation submission",
            ));
        }
        let invalid = self.allocations.get(&id).is_none_or(|a| {
            a.reap_issued
                || a.quarantined
                || a.live.strong_count() == 0
                || a.last_use
                    .is_some_and(|last| last.sequence > submission.sequence)
        });
        if invalid {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::StateMismatch,
                "allocation has no live owner or has retired beyond this submission",
            ));
        }
        self.allocations
            .get_mut(&id)
            .expect("validated allocation")
            .last_use = Some(submission);
        Ok(())
    }

    /// Broker-only completion acknowledgement after the real driver proves all
    /// queue work through this submission finished. Metadata equality is not
    /// completion evidence; callers must not invoke this from an ISF success stub.
    pub(crate) fn acknowledge_completed(
        &mut self,
        submission: GpuSubmissionId,
    ) -> Result<(), NativeGpuFault> {
        if submission.context.owner_id != self.context.owner_id
            || submission.context.device_generation > self.context.device_generation
            || submission.sequence == 0
        {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::IdentityMismatch,
                "foreign completion acknowledgement",
            ));
        }
        let completed = self.completed.entry(submission.context).or_default();
        *completed = (*completed).max(submission.sequence);
        Ok(())
    }

    pub(crate) fn mark_retired(&mut self, id: GpuAllocationId) -> Result<(), NativeGpuFault> {
        let revision = self.next_revision(None)?;
        let record = self
            .allocations
            .get_mut(&id)
            .ok_or_else(|| NativeGpuFault {
                context: self.context,
                frame: None,
                phase: GpuPhase::Retirement,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "unknown retired allocation".into(),
            })?;
        record.retired = true;
        self.revision = revision;
        Ok(())
    }

    /// Mark the ledger, including still-reserved capacity, as retained after an
    /// unproved cleanup. A future device generation cannot reset these bytes.
    pub(crate) fn quarantine(&mut self) {
        self.quarantined = true;
        for record in self.allocations.values_mut() {
            record.quarantined = true;
        }
    }

    /// Keep old-generation charges while the same session creates a new device.
    /// This is unavailable during unproved cleanup and does not reset counters.
    pub(crate) fn advance_generation(&mut self, next: GpuContextId) -> Result<(), NativeGpuFault> {
        if self.quarantined || !self.reservations.is_empty() {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::StateMismatch,
                "generation transition has live reservations or quarantined owners",
            ));
        }
        let expected = self
            .context
            .device_generation
            .checked_add(1)
            .ok_or_else(|| {
                self.fault(
                    None,
                    NativeGpuFaultKind::SequenceExhausted,
                    "device generation exhausted",
                )
            })?;
        if next.owner_id != self.context.owner_id || next.device_generation != expected {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::IdentityMismatch,
                "invalid device generation successor",
            ));
        }
        let revision = self.next_revision(None)?;
        self.context = next;
        self.revision = revision;
        Ok(())
    }

    pub(crate) fn live_allocation_ids(&self) -> impl Iterator<Item = GpuAllocationId> + '_ {
        self.allocations.keys().copied()
    }

    pub(crate) fn try_reap(
        &mut self,
        id: GpuAllocationId,
    ) -> Result<Option<ReapedAllocationReceipt>, NativeGpuFault> {
        let record = self.allocations.get(&id).ok_or_else(|| {
            self.fault(
                None,
                NativeGpuFaultKind::IdentityMismatch,
                "unknown reap allocation",
            )
        })?;
        let gpu_complete = record.last_use.is_none_or(|last| {
            self.completed.get(&last.context).copied().unwrap_or(0) >= last.sequence
        });
        if self.quarantined
            || record.quarantined
            || record.reap_issued
            || record.live.strong_count() != 0
            || !gpu_complete
        {
            return Ok(None);
        }
        let receipt = ReapedAllocationReceipt {
            ledger: Arc::clone(&self.identity),
            id,
            capacity: record.capacity,
        };
        self.allocations
            .get_mut(&id)
            .expect("validated reap allocation")
            .reap_issued = true;
        Ok(Some(receipt))
    }

    pub(crate) fn release_reaped(
        &mut self,
        receipt: ReapedAllocationReceipt,
    ) -> Result<(), NativeGpuFault> {
        if !Arc::ptr_eq(&self.identity, &receipt.ledger)
            || !self.allocations.get(&receipt.id).is_some_and(|r| {
                r.reap_issued
                    && r.capacity == receipt.capacity
                    && r.live.strong_count() == 0
                    && !r.quarantined
                    && !self.quarantined
            })
        {
            return Err(self.fault(
                None,
                NativeGpuFaultKind::IdentityMismatch,
                "invalid reaped allocation receipt",
            ));
        }
        let revision = self.next_revision(None)?;
        self.allocations.remove(&receipt.id);
        self.revision = revision;
        Ok(())
    }

    pub(crate) fn snapshot(&self) -> NativeGpuBudgetSnapshot {
        let mut by_kind = [BudgetKindSnapshot::default(); KIND_COUNT];
        let mut allocated_bytes = 0;
        for record in self.allocations.values() {
            let item = &mut by_kind[record.kind as usize];
            if record.quarantined {
                item.quarantined_bytes += record.capacity;
            } else if record.retired {
                item.retired_bytes += record.capacity;
            } else {
                item.active_bytes += record.capacity;
            }
            allocated_bytes += record.capacity;
        }
        let mut reserved_bytes = 0;
        for record in self.reservations.values() {
            for entry in &record.entries {
                by_kind[entry.kind as usize].reserved_bytes += entry.capacity;
                reserved_bytes += entry.capacity;
            }
        }
        for (kind, high_water) in by_kind.iter_mut().zip(self.kind_high_water) {
            kind.high_water_bytes = high_water;
        }
        NativeGpuBudgetSnapshot {
            context: self.context,
            revision: self.revision,
            allocated_bytes,
            reserved_bytes,
            high_water_bytes: self.high_water,
            allocation_count: self.allocations.len(),
            reservation_count: self.reservations.len(),
            by_kind,
        }
    }

    fn update_high_water(&mut self) {
        let snapshot = self.snapshot();
        self.high_water = self.high_water.max(snapshot.managed_bytes());
        for (high_water, kind) in self.kind_high_water.iter_mut().zip(snapshot.by_kind) {
            *high_water = (*high_water).max(
                kind.active_bytes
                    + kind.retired_bytes
                    + kind.quarantined_bytes
                    + kind.reserved_bytes,
            );
        }
    }

    fn next_revision(&self, frame: Option<NativeFrameId>) -> Result<u64, NativeGpuFault> {
        self.revision.checked_add(1).ok_or_else(|| {
            self.fault(
                frame,
                NativeGpuFaultKind::SequenceExhausted,
                "ledger revision exhausted",
            )
        })
    }

    fn fault(
        &self,
        frame: Option<NativeFrameId>,
        kind: NativeGpuFaultKind,
        message: &str,
    ) -> NativeGpuFault {
        NativeGpuFault {
            context: self.context,
            frame,
            phase: GpuPhase::Retirement,
            kind,
            message: message.into(),
        }
    }
}

/// Conservative queue staging, including the actual backend row-pitch bound.
pub(crate) fn checked_upload_staging_bytes(
    width: u32,
    height: u32,
    pitch: Option<u64>,
) -> Result<u64, NativeGpuDecline> {
    let pitch = pitch
        .filter(|pitch| pitch.is_power_of_two() && *pitch <= u64::from(u32::MAX))
        .ok_or(NativeGpuDecline::UnknownUploadPitch)?;
    if width == 0 || height == 0 {
        return Err(NativeGpuDecline::UnsupportedExtent);
    }
    let alignment = pitch.max(u64::from(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT));
    u64::from(width)
        .checked_mul(4)
        .and_then(|row| row.checked_add(alignment - 1))
        .and_then(|row| row.checked_div(alignment))
        .and_then(|rows| rows.checked_mul(alignment))
        .and_then(|row| row.checked_mul(u64::from(height)))
        .ok_or(NativeGpuDecline::UnsupportedExtent)
}

#[cfg(test)]
#[path = "native_gpu_budget_tests.rs"]
mod tests;
