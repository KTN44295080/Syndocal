#[cfg(test)]
use std::sync::Mutex;
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};

#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimelineAudioLiveFenceTestPoint {
    EnteredBeforeSnapshot,
    AfterSnapshotGuardDrop,
    ExitedAfterSnapshot,
}

/// Published, non-persistent liveness word for the Timeline audio owner.
///
/// The low bit is the aggregate live state and the remaining bits carry a
/// monotonic epoch.  The shared atomic is intentionally kept behind an
/// `Arc`: the public engine handle and its worker must observe one word while
/// clones handed to native consumers remain read-only snapshots of that same
/// authority.
#[derive(Clone)]
pub struct TimelineAudioLiveFence {
    word: Arc<AtomicU64>,
    #[cfg(test)]
    test_hook: Arc<Mutex<Option<Arc<dyn Fn(TimelineAudioLiveFenceTestPoint) + Send + Sync>>>>,
}

impl std::fmt::Debug for TimelineAudioLiveFence {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TimelineAudioLiveFence")
            .field("word", &self.word())
            .finish()
    }
}

impl Default for TimelineAudioLiveFence {
    fn default() -> Self {
        Self {
            // epoch=1, inactive
            word: Arc::new(AtomicU64::new(2)),
            #[cfg(test)]
            test_hook: Arc::new(Mutex::new(None)),
        }
    }
}

impl TimelineAudioLiveFence {
    /// Return the exact packed epoch/liveness word.
    pub fn word(&self) -> u64 {
        self.word.load(Ordering::Acquire)
    }

    /// Return the published aggregate Timeline audio liveness bit.
    pub fn active(&self) -> bool {
        self.word() & 1 == 1
    }

    #[cfg(test)]
    pub(crate) fn set_test_hook(
        &self,
        hook: Option<Arc<dyn Fn(TimelineAudioLiveFenceTestPoint) + Send + Sync>>,
    ) {
        *self
            .test_hook
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = hook;
    }

    #[cfg(test)]
    fn notify_test_hook(&self, point: TimelineAudioLiveFenceTestPoint) {
        let hook = self
            .test_hook
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone();
        if let Some(hook) = hook {
            hook(point);
        }
    }

    #[cfg(test)]
    pub(crate) fn notify_after_snapshot_guard_drop_for_tests(&self) {
        self.notify_test_hook(TimelineAudioLiveFenceTestPoint::AfterSnapshotGuardDrop);
    }

    /// Enter live state immediately before a new B snapshot becomes visible.
    ///
    /// The worker is normally the sole writer, but the CAS keeps cloned
    /// handles deterministic under test and makes a concurrent stale writer
    /// unable to move the epoch backwards.  Once the packed word reaches
    /// `u64::MAX`, it remains active forever: wrapping an epoch would turn a
    /// stale native source into a potentially valid one.
    pub(crate) fn enter_before_publication(&self) {
        loop {
            let current = self.word.load(Ordering::Acquire);
            if current == u64::MAX || current & 1 == 1 {
                return;
            }
            let epoch = current >> 1;
            let next = if epoch >= u64::MAX >> 1 {
                u64::MAX
            } else {
                ((epoch + 1) << 1) | 1
            };
            if self
                .word
                .compare_exchange(current, next, Ordering::Release, Ordering::Acquire)
                .is_ok()
            {
                #[cfg(test)]
                self.notify_test_hook(TimelineAudioLiveFenceTestPoint::EnteredBeforeSnapshot);
                return;
            }
        }
    }

    /// Leave live state after the corresponding B snapshot write guard has
    /// been explicitly dropped.  The saturated terminal word is permanent.
    pub(crate) fn exit_after_publication(&self) {
        loop {
            let current = self.word.load(Ordering::Acquire);
            if current == u64::MAX || current & 1 == 0 {
                return;
            }
            if self
                .word
                .compare_exchange(current, current - 1, Ordering::Release, Ordering::Acquire)
                .is_ok()
            {
                #[cfg(test)]
                self.notify_test_hook(TimelineAudioLiveFenceTestPoint::ExitedAfterSnapshot);
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TimelineAudioLiveFence;
    use std::sync::atomic::Ordering;

    #[test]
    fn initial_word_and_directional_transitions_are_exact() {
        let fence = TimelineAudioLiveFence::default();
        assert_eq!(fence.word(), 2);
        assert!(!fence.active());

        fence.enter_before_publication();
        assert_eq!(fence.word(), 5);
        assert!(fence.active());
        fence.exit_after_publication();
        assert_eq!(fence.word(), 4);
        assert!(!fence.active());
    }

    #[test]
    fn active_enter_and_inactive_exit_are_idempotent_without_epoch_drift() {
        let fence = TimelineAudioLiveFence::default();
        fence.enter_before_publication();
        fence.enter_before_publication();
        assert_eq!(fence.word(), 5);
        fence.exit_after_publication();
        fence.exit_after_publication();
        assert_eq!(fence.word(), 4);
    }

    #[test]
    fn aba_is_impossible_and_terminal_overflow_is_permanently_active() {
        let fence = TimelineAudioLiveFence::default();
        fence.word.store(u64::MAX - 1, Ordering::Release);
        fence.enter_before_publication();
        assert_eq!(fence.word(), u64::MAX);
        assert!(fence.active());
        fence.exit_after_publication();
        assert_eq!(fence.word(), u64::MAX);
        fence.enter_before_publication();
        assert_eq!(fence.word(), u64::MAX);
    }
}
