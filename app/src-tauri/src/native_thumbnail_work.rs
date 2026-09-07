use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex, MutexGuard, TryLockError,
    },
    thread,
    time::{Duration, Instant},
};

/// Application-owned admission, shared by every caller of the two thumbnail commands.
#[derive(Default)]
pub(crate) struct NativeThumbnailWork {
    pub(crate) layers: ThumbnailGate,
    pub(crate) assets: ThumbnailGate,
}

#[derive(Default)]
pub(crate) struct ThumbnailGate {
    active: Arc<AtomicBool>,
}

pub(crate) struct ThumbnailJob {
    active: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
}

pub(crate) struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl ThumbnailGate {
    pub(crate) fn try_acquire(&self) -> Result<ThumbnailJob, String> {
        self.active
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| {
                "Thumbnail work is already queued or running; retry after it completes".to_string()
            })?;
        Ok(ThumbnailJob {
            active: Arc::clone(&self.active),
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }
}

impl ThumbnailJob {
    pub(crate) fn request_guard(&self) -> CancelOnDrop {
        CancelOnDrop(Arc::clone(&self.cancelled))
    }

    pub(crate) fn execute<T>(
        self,
        operation: impl FnOnce(&AtomicBool) -> Result<T, String>,
    ) -> Result<T, String> {
        ensure_not_cancelled(&self.cancelled)?;
        let result = operation(&self.cancelled);
        ensure_not_cancelled(&self.cancelled)?;
        result
    }
}

impl Drop for ThumbnailJob {
    fn drop(&mut self) {
        self.active.store(false, Ordering::Release);
    }
}

pub(crate) fn ensure_not_cancelled(cancel: &AtomicBool) -> Result<(), String> {
    if cancel.load(Ordering::Acquire) {
        Err(
            "Thumbnail request was cancelled; worker ownership is retained until return"
                .to_string(),
        )
    } else {
        Ok(())
    }
}

/// Only background thumbnail work waits here; no render/output-loop changes.
pub(crate) fn lock_renderer<'a, T>(
    renderer: &'a Mutex<T>,
    cancel: &AtomicBool,
) -> Result<MutexGuard<'a, T>, String> {
    lock_renderer_until(renderer, cancel, Instant::now() + Duration::from_secs(2))
}

fn lock_renderer_until<'a, T>(
    renderer: &'a Mutex<T>,
    cancel: &AtomicBool,
    deadline: Instant,
) -> Result<MutexGuard<'a, T>, String> {
    loop {
        ensure_not_cancelled(cancel)?;
        let now = Instant::now();
        if now >= deadline {
            return Err(
                "Thumbnail renderer wait timed out; retry after preview work completes".to_string(),
            );
        }
        match renderer.try_lock() {
            Ok(guard) => {
                ensure_not_cancelled(cancel)?;
                return Ok(guard);
            }
            Err(TryLockError::Poisoned(_)) => {
                return Err("Video preview renderer lock was poisoned".to_string())
            }
            Err(TryLockError::WouldBlock) => {
                thread::sleep((deadline - now).min(Duration::from_millis(10)))
            }
        }
    }
}

#[cfg(test)]
#[path = "native_thumbnail_work_tests.rs"]
mod tests;
