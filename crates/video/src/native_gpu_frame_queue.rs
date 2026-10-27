//! Atomic, whole-owner publication of the complete native CPU history barrier.
//!
//! The session preflights every target queue while exclusively borrowing the
//! renderer, then commits each queue without fallible work. Pixel Arcs and their
//! charged backing owners always occupy the same canonical slot.

use crate::native_gpu_output::cpu::history::NativeCpuHistoryQueue;
use crate::FrameQueue;

impl FrameQueue {
    pub(crate) fn accepts_native_history_replacement(
        &self,
        history: &NativeCpuHistoryQueue,
    ) -> bool {
        let scope = history.scope();
        scope.input == self.key
            && scope.layer_id == self.layer_id
            && history.len() <= self.capacity()
            && self.policy_revision.checked_add(1).is_some()
            && history
                .entries()
                .zip(history.metadata())
                .all(|(entry, meta)| {
                    let identity = entry.identity();
                    identity.entry == meta.id
                        && identity.entry.input == self.key
                        && identity.entry.queue_epoch == scope.queue_epoch
                        && identity.layer_id == self.layer_id
                        && (identity.width, identity.height) == scope.extent
                        && identity.pts_ms == meta.pts_ms
                        && identity.duration_ms == meta.duration_ms
                })
    }

    /// Called only after all queues in the renderer have passed the borrowed
    /// preflight. No callback or renderer mutation intervenes in that barrier.
    pub(crate) fn install_native_history_replacement(&mut self, history: NativeCpuHistoryQueue) {
        assert!(self.accepts_native_history_replacement(&history));
        let next_revision = self.policy_revision + 1;
        // Release old pixels before their companion owners, then move every
        // current native survivor, including entries absent from the old queue.
        self.slots.fill(None);
        self.native_owners.fill(None);
        self.native_cpu_owners.fill(None);
        self.native_entry_ids.fill(None);
        self.policy.clear();
        self.len = history.len();
        for (slot, entry) in history.into_entries().into_iter().enumerate() {
            let identity = entry.identity().entry;
            let (pixels, owner) = entry.into_canonical_parts();
            self.policy.push(self.slot_meta(slot, &pixels));
            self.slots[slot] = Some(pixels);
            self.native_owners[slot] = Some(owner);
            self.native_entry_ids[slot] = Some(identity);
        }
        self.policy_revision = next_revision;
    }
}
