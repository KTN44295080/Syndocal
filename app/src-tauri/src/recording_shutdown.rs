//! Owned background handoff for recording work that cannot be joined by a
//! bounded caller deadline.
//!
//! A Rust thread cannot be safely force-killed while it owns a GPU/libav
//! object.  At that boundary this module keeps the original `JoinHandle`
//! alive in a named reaper.  The runtime therefore returns on its deadline
//! without abandoning the worker, and a later lifecycle poll can observe the
//! reaper's terminal result.

use std::{
    sync::{
        atomic::{AtomicU8, Ordering},
        Arc, LazyLock, Mutex,
    },
    thread::{self, JoinHandle},
};

const REAPED: u8 = 1;
const PANICKED: u8 = 2;

pub(crate) struct DeferredWorker {
    done: Arc<AtomicU8>,
}

impl DeferredWorker {
    pub(crate) fn is_finished(&self) -> bool {
        self.done.load(Ordering::Acquire) != 0
    }

    pub(crate) fn outcome(&self) -> super::video_recording_lifecycle::WorkerReap {
        if self.done.load(Ordering::Acquire) == PANICKED {
            super::video_recording_lifecycle::WorkerReap::Panicked
        } else {
            super::video_recording_lifecycle::WorkerReap::Joined
        }
    }
}

struct QuarantinedWorker {
    worker: JoinHandle<()>,
    done: Arc<AtomicU8>,
}

/// Extremely rare thread-creation failure must not drop the only owner. A
/// later recording lifecycle call retries this bounded quarantine handoff.
static QUARANTINED_WORKERS: LazyLock<Mutex<Vec<QuarantinedWorker>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub(crate) fn defer_worker(worker: JoinHandle<()>, label: &'static str) -> DeferredWorker {
    retry_quarantined_workers();
    let done = Arc::new(AtomicU8::new(0));
    let worker_done = Arc::clone(&done);
    let (sender, receiver) = std::sync::mpsc::sync_channel::<JoinHandle<()>>(1);
    match thread::Builder::new()
        .name(format!("syndocal-{label}-reaper"))
        .spawn(move || {
            if let Ok(worker) = receiver.recv() {
                let outcome = worker.join();
                worker_done.store(
                    if outcome.is_ok() { REAPED } else { PANICKED },
                    Ordering::Release,
                );
            }
        }) {
        Ok(reaper) => {
            if let Err(std::sync::mpsc::SendError(worker)) = sender.send(worker) {
                let mut quarantine = QUARANTINED_WORKERS
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                quarantine.push(QuarantinedWorker {
                    worker,
                    done: Arc::clone(&done),
                });
            }
            // The reaper owns the original worker until JoinHandle::join has
            // returned. Dropping only this outer handle does not detach the
            // recording work; it transfers ownership to the named reaper.
            drop(reaper);
        }
        Err(_) => {
            let mut quarantine = QUARANTINED_WORKERS
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            quarantine.push(QuarantinedWorker {
                worker,
                done: Arc::clone(&done),
            });
        }
    }
    DeferredWorker { done }
}

fn retry_quarantined_workers() {
    let pending = {
        let mut quarantine = QUARANTINED_WORKERS
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *quarantine)
    };
    for pending in pending {
        let QuarantinedWorker { worker, done } = pending;
        let worker_done = Arc::clone(&done);
        let (sender, receiver) = std::sync::mpsc::sync_channel::<JoinHandle<()>>(1);
        match thread::Builder::new()
            .name("syndocal-recording-quarantine-reaper".to_string())
            .spawn(move || {
                if let Ok(worker) = receiver.recv() {
                    let outcome = worker.join();
                    worker_done.store(
                        if outcome.is_ok() { REAPED } else { PANICKED },
                        Ordering::Release,
                    );
                }
            }) {
            Ok(reaper) => {
                if let Err(std::sync::mpsc::SendError(worker)) = sender.send(worker) {
                    let mut quarantine = QUARANTINED_WORKERS
                        .lock()
                        .unwrap_or_else(|poisoned| poisoned.into_inner());
                    quarantine.push(QuarantinedWorker { worker, done });
                }
                drop(reaper);
            }
            Err(_) => {
                let mut quarantine = QUARANTINED_WORKERS
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                quarantine.push(QuarantinedWorker { worker, done });
            }
        }
    }
}
