//! P lane's GPU-independent queue oracle (INTERFACE-01 section 5.1).
//!
//! The CPU queue remains unchanged until I wires its thin delegation. Plans are
//! prospective metadata only: selecting an incoming ID does not publish it.

use crate::native_gpu_contract::HistoryEntryId;

pub(crate) const NATIVE_HISTORY_CAPACITY: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct HistoryMeta {
    pub id: HistoryEntryId,
    pub pts_ms: u64,
    pub duration_ms: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryPushKind {
    Inserted,
    Replaced { old: HistoryEntryId },
    DroppedOldest { old: HistoryEntryId },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HistoryPolicyError {
    InvalidCapacity,
    OverCapacity,
    NotStrictlyAscending,
    DuplicateIdentity,
}

#[derive(Debug)]
pub(crate) struct HistoryPlan {
    expected_revision: u64,
    current: Vec<HistoryMeta>,
    incoming: Vec<HistoryMeta>,
    survivors: Vec<HistoryMeta>,
    pushes: Vec<HistoryPushKind>,
    selected: Option<HistoryEntryId>,
}

impl HistoryPlan {
    pub(crate) fn bind_revision(mut self, revision: u64) -> Self {
        self.expected_revision = revision;
        self
    }

    pub(crate) fn expected_revision(&self) -> u64 {
        self.expected_revision
    }

    pub(crate) fn current(&self) -> &[HistoryMeta] {
        &self.current
    }

    pub(crate) fn incoming(&self) -> &[HistoryMeta] {
        &self.incoming
    }

    pub(crate) fn survivors(&self) -> &[HistoryMeta] {
        &self.survivors
    }

    pub(crate) fn pushes(&self) -> &[HistoryPushKind] {
        &self.pushes
    }

    pub(crate) fn selected(&self) -> Option<HistoryEntryId> {
        self.selected
    }
}

pub(crate) fn plan_history_update(
    current_ascending: &[HistoryMeta],
    incoming_in_canonical_order: &[HistoryMeta],
    capacity: usize,
    target_ms: u64,
) -> Result<HistoryPlan, HistoryPolicyError> {
    if capacity == 0 {
        return Err(HistoryPolicyError::InvalidCapacity);
    }
    if current_ascending.len() > capacity {
        return Err(HistoryPolicyError::OverCapacity);
    }
    if current_ascending
        .windows(2)
        .any(|w| w[0].pts_ms >= w[1].pts_ms)
    {
        return Err(HistoryPolicyError::NotStrictlyAscending);
    }
    // Reusing an entry ID would make a late completion indistinguishable from
    // its replacement. Sequence issuance/epoch validation belongs to history.
    let all: Vec<_> = current_ascending
        .iter()
        .chain(incoming_in_canonical_order)
        .map(|meta| meta.id)
        .collect();
    if all.iter().enumerate().any(|(i, id)| all[..i].contains(id)) {
        return Err(HistoryPolicyError::DuplicateIdentity);
    }

    let mut survivors = current_ascending.to_vec();
    let mut pushes = Vec::with_capacity(incoming_in_canonical_order.len());
    for incoming in incoming_in_canonical_order {
        let kind = if let Some(index) = survivors.iter().position(|m| m.pts_ms == incoming.pts_ms) {
            let old = survivors[index].id;
            survivors[index] = *incoming;
            HistoryPushKind::Replaced { old }
        } else {
            // Evict BEFORE insertion: a new older PTS still survives this push.
            let kind = if survivors.len() == capacity {
                HistoryPushKind::DroppedOldest {
                    old: survivors.remove(0).id,
                }
            } else {
                HistoryPushKind::Inserted
            };
            let index = survivors.partition_point(|m| m.pts_ms < incoming.pts_ms);
            survivors.insert(index, *incoming);
            kind
        };
        pushes.push(kind);
    }
    let selected = select_at_or_before_or_nearest(&survivors, target_ms);
    Ok(HistoryPlan {
        expected_revision: 0,
        current: current_ascending.to_vec(),
        incoming: incoming_in_canonical_order.to_vec(),
        survivors,
        pushes,
        selected,
    })
}

/// Input is the validated ascending ledger. Ties choose its first/lower PTS.
pub(crate) fn select_nearest(ascending: &[HistoryMeta], target_ms: u64) -> Option<HistoryEntryId> {
    ascending
        .iter()
        .min_by_key(|m| m.pts_ms.abs_diff(target_ms))
        .map(|m| m.id)
}

pub(crate) fn select_at_or_before_or_nearest(
    ascending: &[HistoryMeta],
    target_ms: u64,
) -> Option<HistoryEntryId> {
    ascending
        .iter()
        .rev()
        .find(|m| m.pts_ms <= target_ms)
        .map(|m| m.id)
        .or_else(|| select_nearest(ascending, target_ms))
}

/// Retain equality, matching FrameQueue::drop_before.
pub(crate) fn drop_before(ascending: &mut Vec<HistoryMeta>, target_ms: u64) -> usize {
    let original_len = ascending.len();
    ascending.retain(|m| m.pts_ms >= target_ms);
    original_len - ascending.len()
}

#[cfg(test)]
#[path = "frame_history_policy_tests.rs"]
mod tests;
