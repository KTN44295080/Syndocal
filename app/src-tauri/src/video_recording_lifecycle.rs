//! Ownership and bounded acknowledgement for the video recording worker.
//!
//! The recording worker can still be inside renderer or encoder I/O when Stop
//! is requested. This module bounds only the caller's acknowledgement wait;
//! a timeout keeps the stop flag and join handle owned by the runtime. The
//! worker is never detached, and shutdown remains blocking when the runtime
//! itself is dropped because renderer and inherited-pipe I/O can remain pending
//! even after the encoder supervisor terminates the direct child.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

/// The Stop command's acknowledgement window. This matches the existing
/// 250ms process lifecycle convention while deliberately covering only the
/// worker termination check, not renderer or encoder I/O.
pub(crate) const STOP_ACKNOWLEDGEMENT_TIMEOUT: Duration = Duration::from_millis(250);
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
    stopping: bool,
}

impl RecordingWorkerLifecycle {
    pub(crate) fn new(stop: Arc<AtomicBool>, worker: JoinHandle<()>) -> Self {
        Self {
            stop,
            worker: Some(worker),
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

    /// Reap a worker only after `JoinHandle::is_finished` reports termination.
    /// A timeout therefore leaves the original stop flag and join handle in
    /// this object.
    pub(crate) fn reap_if_complete(&mut self) -> Option<WorkerReap> {
        if !self
            .worker
            .as_ref()
            .is_some_and(|worker| worker.is_finished())
        {
            return None;
        }
        Some(self.reap())
    }

    /// Used only for the existing runtime-drop ownership boundary. It is
    /// intentionally unbounded because dropping this object must not detach a
    /// live recording worker.
    pub(crate) fn reap_blocking(&mut self) -> WorkerReap {
        self.reap()
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
        if self.worker.is_some() {
            self.request_stop();
            let _ = self.reap_blocking();
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
                let _ = lifecycle.reap_blocking();
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
        assert!(worker.lifecycle().worker.is_none());
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
        assert!(worker.lifecycle().worker.is_some());
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
        assert!(worker.lifecycle().worker.is_some());
        worker.release();
        assert_eq!(
            worker
                .lifecycle()
                .wait_for_completion(Duration::from_secs(1)),
            CompletionWait::Completed
        );
        assert!(worker.lifecycle().worker.is_some());
        assert_eq!(
            worker.lifecycle_mut().reap_if_complete(),
            Some(WorkerReap::Joined)
        );
        assert!(worker.lifecycle().worker.is_none());
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
        assert!(worker.lifecycle().worker.is_none());
    }
}
