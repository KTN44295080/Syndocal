//! Stop-aware access to the shared renderer, without detaching a rendering task.
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex, MutexGuard, TryLockError,
};
use std::time::Duration;

pub(super) fn acquire<'a, T>(
    renderer: &'a Mutex<T>,
    stop: &AtomicBool,
) -> Result<Option<MutexGuard<'a, T>>, &'static str> {
    acquire_with_wait(renderer, stop, || {
        std::thread::sleep(Duration::from_millis(5))
    })
}

fn acquire_with_wait<'a, T>(
    renderer: &'a Mutex<T>,
    stop: &AtomicBool,
    mut wait: impl FnMut(),
) -> Result<Option<MutexGuard<'a, T>>, &'static str> {
    loop {
        if stop.load(Ordering::Acquire) {
            return Ok(None);
        }
        match renderer.try_lock() {
            Ok(guard) => {
                return Ok((!stop.load(Ordering::Acquire)).then_some(guard));
            }
            Err(TryLockError::Poisoned(_)) => return Err("Recording renderer lock is poisoned"),
            Err(TryLockError::WouldBlock) => wait(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{mpsc, Arc};

    #[test]
    fn recording_renderer_wait_observes_stop_while_lock_stays_held() {
        let renderer = Arc::new(Mutex::new(()));
        let held = renderer.lock().unwrap();
        let stop = Arc::new(AtomicBool::new(false));
        let (started_tx, started_rx) = mpsc::channel();
        let (resume_tx, resume_rx) = mpsc::channel();
        let (done_tx, done_rx) = mpsc::channel();
        let waiting_renderer = Arc::clone(&renderer);
        let waiting_stop = Arc::clone(&stop);
        let worker = std::thread::spawn(move || {
            let cancelled = acquire_with_wait(&waiting_renderer, &waiting_stop, || {
                started_tx.send(()).unwrap();
                resume_rx.recv().unwrap();
            })
            .unwrap()
            .is_none();
            done_tx.send(cancelled).unwrap();
        });
        let observed_contention = started_rx.recv_timeout(Duration::from_secs(1));
        stop.store(true, Ordering::Release);
        let _ = resume_tx.send(());
        let cancelled = done_rx.recv_timeout(Duration::from_secs(1));
        // Release even on failure, so the regression does not strand a test thread.
        drop(held);
        worker.join().unwrap();
        assert!(
            observed_contention.is_ok(),
            "test must reach renderer contention"
        );
        assert_eq!(cancelled.unwrap(), true);
    }

    #[test]
    fn recording_renderer_access_preserves_value_and_rejects_poison() {
        let renderer = Mutex::new(17);
        let stop = AtomicBool::new(false);
        assert_eq!(**acquire(&renderer, &stop).unwrap().as_ref().unwrap(), 17);
        let _ = std::panic::catch_unwind(|| {
            let _held = renderer.lock().unwrap();
            panic!("poison test renderer");
        });
        assert!(acquire(&renderer, &stop).is_err());
        stop.store(true, Ordering::Release);
        assert!(acquire(&renderer, &stop).unwrap().is_none());
    }
}
