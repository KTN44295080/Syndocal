//! Ownership and bounded acknowledgement for the video recording worker.
//!
//! The recording worker can still be inside renderer or encoder I/O when Stop
//! is requested. This module owns the stop flag, the worker join handle, and
//! the bounded fallback handoff. A timeout never makes the runtime forget a
//! live worker: it is retained locally until the deadline, then transferred to
//! the named recording reaper in `recording_shutdown`.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use super::recording_shutdown::DeferredWorker;

/// The Stop command's acknowledgement window. This matches the existing
/// 250ms process lifecycle convention while deliberately covering only the
/// worker termination check, not renderer or encoder I/O.
pub(crate) const STOP_ACKNOWLEDGEMENT_TIMEOUT: Duration = Duration::from_millis(250);
/// Maximum time runtime teardown may wait for a renderer/encoder worker after
/// cancellation. In-process code is not thread-killable safely; expiry moves
/// the original join owner to the recording reaper instead of abandoning it.
pub(crate) const TOTAL_STOP_DEADLINE: Duration = Duration::from_secs(10);
const TERMINATION_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CompletionWait {
    Completed,
    TimedOut,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum WorkerReap {
    Joined,
    Panicked,
}

pub(crate) struct RecordingWorkerLifecycle {
    stop: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
    deferred: Option<DeferredWorker>,
    stopping: bool,
}

impl RecordingWorkerLifecycle {
    pub(crate) fn new(stop: Arc<AtomicBool>, worker: JoinHandle<()>) -> Self {
        Self {
            stop,
            worker: Some(worker),
            deferred: None,
            stopping: false,
        }
    }

    pub(crate) fn request_stop(&mut self) -> bool {
        let first_request = !self.stopping;
        self.stopping = true;
        self.stop.store(true, Ordering::Release);
        first_request
    }

    pub(crate) fn wait_for_completion(&self, timeout: Duration) -> CompletionWait {
        let deadline = Instant::now() + timeout;
        loop {
            if self
                .worker
                .as_ref()
                .is_some_and(|worker| worker.is_finished())
                || self
                    .deferred
                    .as_ref()
                    .is_some_and(DeferredWorker::is_finished)
            {
                return CompletionWait::Completed;
            }
            let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
                return CompletionWait::TimedOut;
            };
            std::thread::sleep(remaining.min(TERMINATION_POLL_INTERVAL));
        }
    }

    pub(crate) fn is_stopping(&self) -> bool {
        self.stopping
    }

    pub(crate) fn has_worker(&self) -> bool {
        self.worker.is_some() || self.deferred.is_some()
    }

    /// Reap a worker only after `JoinHandle::is_finished` reports termination.
    /// A timeout therefore leaves the original stop flag and join handle in
    /// this object.
    pub(crate) fn reap_if_complete(&mut self) -> Option<WorkerReap> {
        if self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return Some(self.reap());
        }
        if self
            .deferred
            .as_ref()
            .is_some_and(DeferredWorker::is_finished)
        {
            let deferred = self.deferred.take().expect("deferred worker owner");
            self.stopping = false;
            return Some(deferred.outcome());
        }
        None
    }

    /// Wait for a bounded shutdown window. If the worker is still live at the
    /// deadline, the original join handle moves to an owned background reaper
    /// and this lifecycle remains occupied until that reaper finishes.
    pub(crate) fn reap_with_deadline(&mut self, timeout: Duration) -> CompletionWait {
        if self.wait_for_completion(timeout) == CompletionWait::Completed {
            let _ = self.reap_if_complete();
            return CompletionWait::Completed;
        }
        if let Some(worker) = self.worker.take() {
            self.deferred = Some(super::recording_shutdown::defer_worker(
                worker,
                "recording-worker",
            ));
        }
        CompletionWait::TimedOut
    }

    fn reap(&mut self) -> WorkerReap {
        self.stopping = false;
        match self.worker.take() {
            Some(worker) => match worker.join() {
                Ok(()) => WorkerReap::Joined,
                Err(_) => WorkerReap::Panicked,
            },
            None => WorkerReap::Joined,
        }
    }
}

impl Drop for RecordingWorkerLifecycle {
    fn drop(&mut self) {
        if self.has_worker() {
            self.request_stop();
            let _ = self.reap_with_deadline(TOTAL_STOP_DEADLINE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::mpsc::{self, Sender},
        thread,
    };

    struct TestWorker {
        lifecycle: Option<RecordingWorkerLifecycle>,
        release: Option<Sender<()>>,
        stop: Arc<AtomicBool>,
    }

    impl TestWorker {
        fn delayed(panics: bool) -> Self {
            let stop = Arc::new(AtomicBool::new(false));
            let (ready, ready_rx) = mpsc::channel();
            let (release, release_rx) = mpsc::channel();
            let worker = thread::spawn(move || {
                ready.send(()).expect("test worker readiness");
                release_rx.recv().expect("test worker release");
                if panics {
                    panic!("test recording worker panic");
                }
            });
            ready_rx
                .recv_timeout(Duration::from_secs(1))
                .expect("test worker must start");
            Self {
                lifecycle: Some(RecordingWorkerLifecycle::new(Arc::clone(&stop), worker)),
                release: Some(release),
                stop,
            }
        }

        fn lifecycle(&self) -> &RecordingWorkerLifecycle {
            self.lifecycle.as_ref().expect("test lifecycle")
        }

        fn lifecycle_mut(&mut self) -> &mut RecordingWorkerLifecycle {
            self.lifecycle.as_mut().expect("test lifecycle")
        }

        fn release(&mut self) {
            self.release
                .take()
                .expect("test release sender")
                .send(())
                .expect("test worker must still receive release");
        }
    }

    impl Drop for TestWorker {
        fn drop(&mut self) {
            if let Some(release) = self.release.take() {
                let _ = release.send(());
            }
            if let Some(lifecycle) = self.lifecycle.as_mut() {
                lifecycle.request_stop();
                let _ = lifecycle.reap_with_deadline(Duration::from_secs(1));
            }
        }
    }

    #[test]
    fn prompt_completion_is_reaped() {
        let mut worker = TestWorker::delayed(false);
        worker.release();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_secs(1)),
            CompletionWait::Completed
        );
        assert_eq!(
            worker.lifecycle_mut().reap_if_complete(),
            Some(WorkerReap::Joined)
        );
        assert!(!worker.lifecycle().has_worker());
    }

    #[test]
    fn timeout_retains_stop_and_join_ownership() {
        let mut worker = TestWorker::delayed(false);
        assert!(worker.lifecycle_mut().request_stop());
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_millis(1)),
            CompletionWait::TimedOut
        );
        assert!(worker.lifecycle().is_stopping());
        assert!(worker.lifecycle().has_worker());
        assert!(worker.stop.load(Ordering::Acquire));
        assert!(!worker.lifecycle_mut().request_stop());
        worker.release();
    }

    #[test]
    fn pending_worker_blocks_restart_until_late_completion_is_reaped() {
        let mut worker = TestWorker::delayed(false);
        worker.lifecycle_mut().request_stop();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_millis(1)),
            CompletionWait::TimedOut
        );
        assert!(worker.lifecycle().has_worker());
        worker.release();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_secs(1)),
            CompletionWait::Completed
        );
        assert!(worker.lifecycle().has_worker());
        assert_eq!(
            worker.lifecycle_mut().reap_if_complete(),
            Some(WorkerReap::Joined)
        );
        assert!(!worker.lifecycle().has_worker());
    }

    #[test]
    fn deadline_handoffs_live_worker_to_owned_reaper() {
        let mut worker = TestWorker::delayed(false);
        worker.lifecycle_mut().request_stop();
        assert_eq!(
            worker
                .lifecycle_mut()
                .reap_with_deadline(Duration::from_millis(1)),
            CompletionWait::TimedOut
        );
        assert!(worker.lifecycle().has_worker());
        worker.release();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_secs(1)),
            CompletionWait::Completed
        );
        assert_eq!(
            worker.lifecycle_mut().reap_if_complete(),
            Some(WorkerReap::Joined)
        );
        assert!(!worker.lifecycle().has_worker());
    }

    #[test]
    fn worker_panic_is_reported_when_reaped() {
        let mut worker = TestWorker::delayed(true);
        worker.release();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_secs(1)),
            CompletionWait::Completed
        );
        assert_eq!(
            worker.lifecycle_mut().reap_if_complete(),
            Some(WorkerReap::Panicked)
        );
        assert!(!worker.lifecycle().has_worker());
    }
}
