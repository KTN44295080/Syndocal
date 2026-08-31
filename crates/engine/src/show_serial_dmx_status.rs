//! Runtime-only status and revision fence for the machine-local Open DMX route.
//!
//! The engine owns this state because a serial worker can fault independently
//! of an IPC request.  Tauri registers an observer, but the engine keeps no
//! dependency on Tauri or any frontend serialization type.

use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

/// Read-only truth for the machine-local Open DMX worker. This is never an
/// output snapshot. Queue fields mean only that the worker accepted its newest
/// bounded-queue frame. `zero_frame_physical_write_completed` is narrower and
/// stronger: it records one bounded zero BREAK/MAB/write_all/flush receipt.
/// Neither form of status claims fixture or wire delivery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSerialDmxRouteStatus {
    pub active: bool,
    pub zero_frame_queued: bool,
    pub zero_frame_physical_write_completed: bool,
    pub live_frame_queued: bool,
    /// True only after the retained Open DMX sender has no worker left to
    /// write. A bounded shutdown timeout/detach remains false even though the
    /// route is no longer Active, so machine binding cannot race that worker.
    pub worker_shutdown_completed: bool,
    pub faulted: bool,
    /// Configuration/sender truth only. It does not claim UDP receiver,
    /// Art-Net wire, or Unity delivery.
    pub artnet_mirror_live: bool,
    pub artnet_mirror_detail: String,
    pub detail: String,
}

impl Default for ShowSerialDmxRouteStatus {
    fn default() -> Self {
        Self {
            active: false,
            zero_frame_queued: false,
            zero_frame_physical_write_completed: false,
            live_frame_queued: false,
            worker_shutdown_completed: true,
            faulted: false,
            artnet_mirror_live: false,
            artnet_mirror_detail:
                "Art-Net mirror route state was not observed by the USB-DMX worker.".to_string(),
            detail: "USB-DMX worker is stopped.".to_string(),
        }
    }
}

/// Decide whether it is safe to construct a replacement Open DMX worker from
/// the last authoritative worker state.  This deliberately distinguishes a
/// completed fault from a bounded-shutdown timeout: a faulted worker that has
/// actually stopped may be recovered only under both S0 latches, while a
/// detached worker remains an in-process hard fence even after its route slot
/// was removed.
pub(super) fn require_show_serial_dmx_enable_admission(
    status: &ShowSerialDmxRouteStatus,
    safety_authority_engaged: bool,
    physical_s0_latched: bool,
) -> Result<(), String> {
    if status.active || status.live_frame_queued {
        return Err(
            "the previous Open DMX worker is still active or has a live frame queued".to_string(),
        );
    }
    if !status.worker_shutdown_completed {
        return Err(
            "the previous Open DMX worker did not complete bounded shutdown; a replacement worker is prohibited"
                .to_string(),
        );
    }
    // A non-faulted incomplete S0 transaction is internally inconsistent with
    // a completed worker. Never reinterpret it as a benign stopped state.
    // A faulted joined worker is different: it may have lost its final receipt
    // during disconnect, and a fresh initial S0 transaction is the only path
    // that can clear the sticky fault.
    if status.zero_frame_queued && !status.zero_frame_physical_write_completed && !status.faulted {
        return Err(
            "the previous Open DMX worker reports an incomplete S0 transaction without a fault"
                .to_string(),
        );
    }
    if status.faulted && (!safety_authority_engaged || !physical_s0_latched) {
        return Err(
            "faulted Open DMX recovery requires both logical and physical S0 latches to remain engaged"
                .to_string(),
        );
    }
    Ok(())
}

/// One coherent status/revision snapshot. The revision is a process-lifetime
/// monotonic fence; callers serialize it as a decimal string before crossing
/// into JavaScript so no `u64` precision is lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShowSerialDmxRouteStatusSnapshot {
    pub revision: u64,
    pub status: ShowSerialDmxRouteStatus,
}

type ShowSerialDmxRouteStatusObserver = Arc<dyn Fn(ShowSerialDmxRouteStatusSnapshot) + Send + Sync>;

#[derive(Debug, Clone)]
pub(super) struct StoredShowSerialDmxRouteStatus {
    revision: u64,
    status: ShowSerialDmxRouteStatus,
}

/// Internal synchronization owner for Open DMX route status. Every normal
/// publication carries its revision under one mutex. A poisoned status mutex
/// is never converted to an Active answer: query callers receive an error and
/// the registered observer receives an explicit fault-shaped revision.
pub(super) struct ShowSerialDmxRouteStatusStore {
    current: Mutex<StoredShowSerialDmxRouteStatus>,
    latest_revision: AtomicU64,
    /// A `u64` route revision is process-lifetime only. If it is exhausted,
    /// there is no representable newer fence, so IPC must reject every status
    /// read instead of ever reusing an Active revision.
    revision_exhausted: AtomicBool,
    observer: Mutex<Option<ShowSerialDmxRouteStatusObserver>>,
}

impl ShowSerialDmxRouteStatusStore {
    pub(super) fn new() -> Self {
        Self {
            current: Mutex::new(StoredShowSerialDmxRouteStatus {
                revision: 1,
                status: ShowSerialDmxRouteStatus::default(),
            }),
            latest_revision: AtomicU64::new(1),
            revision_exhausted: AtomicBool::new(false),
            observer: Mutex::new(None),
        }
    }

    pub(super) fn try_snapshot(&self) -> Result<ShowSerialDmxRouteStatusSnapshot, String> {
        if self.revision_exhausted.load(Ordering::Acquire) {
            return Err(
                "USB-DMX route status revision was exhausted; route truth is permanently unavailable and remains fail-closed"
                    .to_string(),
            );
        }
        self.current
            .lock()
            .map(|current| ShowSerialDmxRouteStatusSnapshot {
                revision: current.revision,
                status: current.status.clone(),
            })
            .map_err(|_| {
                "USB-DMX route status lock was poisoned; route truth is unavailable and remains fail-closed"
                    .to_string()
            })
    }

    /// Internal engine callers still need a safe status shape while handling
    /// a fault. IPC and binding mutation paths use `try_snapshot` instead.
    pub(super) fn fail_closed_snapshot(&self) -> ShowSerialDmxRouteStatusSnapshot {
        self.try_snapshot()
            .unwrap_or_else(|detail| ShowSerialDmxRouteStatusSnapshot {
                revision: self.latest_revision.load(Ordering::Acquire),
                status: Self::fault_status(detail),
            })
    }

    pub(super) fn set(&self, status: ShowSerialDmxRouteStatus) {
        if self.revision_exhausted.load(Ordering::Acquire) {
            // The terminal fault has already been emitted. Never reuse its
            // u64::MAX fence for a later Active or recovery publication.
            return;
        }
        let snapshot = match self.current.lock() {
            Ok(mut current) => {
                // The live tick republishes the same status at show cadence.
                // A duplicate must not create a new IPC/event revision.
                if current.status == status {
                    return;
                }
                match current.revision.checked_add(1) {
                    Some(revision) => {
                        current.revision = revision;
                        current.status = status;
                        self.latest_revision.store(revision, Ordering::Release);
                        ShowSerialDmxRouteStatusSnapshot {
                            revision,
                            status: current.status.clone(),
                        }
                    }
                    None => {
                        self.revision_exhausted.store(true, Ordering::Release);
                        current.status = Self::fault_status(
                            "USB-DMX route status revision was exhausted; no newer status fence exists, so route truth is permanently fail-closed"
                                .to_string(),
                        );
                        self.latest_revision
                            .store(current.revision, Ordering::Release);
                        ShowSerialDmxRouteStatusSnapshot {
                            revision: current.revision,
                            status: current.status.clone(),
                        }
                    }
                }
            }
            Err(_) => {
                let revision = self.advance_latest_revision_or_exhaust();
                ShowSerialDmxRouteStatusSnapshot {
                    revision,
                    status: Self::fault_status(
                        "USB-DMX route status lock was poisoned while publishing; route truth is unavailable and remains fail-closed".to_string(),
                    ),
                }
            }
        };
        let observer = self.observer.lock().ok().and_then(|slot| slot.clone());
        if let Some(observer) = observer {
            observer(snapshot);
        }
    }

    /// Advance a lock-free poison-path revision without wrapping at `u64::MAX`.
    /// A poisoned status lock is already fail-closed; revision exhaustion keeps
    /// that terminal state visible without manufacturing a smaller generation.
    fn advance_latest_revision_or_exhaust(&self) -> u64 {
        let mut observed = self.latest_revision.load(Ordering::Acquire);
        loop {
            let Some(next) = observed.checked_add(1) else {
                self.revision_exhausted.store(true, Ordering::Release);
                return observed;
            };
            match self.latest_revision.compare_exchange_weak(
                observed,
                next,
                Ordering::AcqRel,
                Ordering::Acquire,
            ) {
                Ok(_) => return next,
                Err(actual) => observed = actual,
            }
        }
    }

    pub(super) fn set_observer(
        &self,
        observer: Option<Arc<dyn Fn(ShowSerialDmxRouteStatusSnapshot) + Send + Sync>>,
    ) -> Result<(), String> {
        let mut slot = self
            .observer
            .lock()
            .map_err(|_| "USB-DMX route status observer lock was poisoned".to_string())?;
        *slot = observer;
        Ok(())
    }

    #[cfg(test)]
    pub(super) fn test_hold_current(
        &self,
    ) -> std::sync::MutexGuard<'_, StoredShowSerialDmxRouteStatus> {
        self.current
            .lock()
            .expect("the test USB-DMX status lock must be available")
    }

    #[cfg(test)]
    pub(super) fn test_set_current_revision(&self, revision: u64) {
        let mut current = self
            .current
            .lock()
            .expect("the test USB-DMX status lock must be available");
        current.revision = revision;
        self.latest_revision.store(revision, Ordering::Release);
    }

    fn fault_status(detail: String) -> ShowSerialDmxRouteStatus {
        ShowSerialDmxRouteStatus {
            active: false,
            zero_frame_queued: false,
            zero_frame_physical_write_completed: false,
            live_frame_queued: false,
            worker_shutdown_completed: false,
            faulted: true,
            artnet_mirror_live: false,
            artnet_mirror_detail:
                "Art-Net mirror route state is unavailable because USB-DMX status locking failed."
                    .to_string(),
            detail,
        }
    }
}
