use crate::native_thumbnail_ticket::{ThumbnailLane, ThumbnailTicket};
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
    active: Arc<Mutex<Option<ActiveThumbnail>>>,
}

#[derive(Clone)]
struct ActiveThumbnail {
    owner: String,
    request_id: String,
    cancelled: Arc<AtomicBool>,
}

pub(crate) struct ThumbnailJob {
    active: Arc<Mutex<Option<ActiveThumbnail>>>,
    request: ActiveThumbnail,
}

pub(crate) struct CancelOnDrop(Arc<AtomicBool>);

impl Drop for CancelOnDrop {
    fn drop(&mut self) {
        self.0.store(true, Ordering::Release);
    }
}

impl ThumbnailGate {
    pub(crate) fn try_acquire(&self, owner: &str) -> Result<ThumbnailJob, String> {
        if owner.is_empty() || owner.len() > 256 {
            return Err("Invalid thumbnail window owner".to_string());
        }
        let mut active = self
            .active
            .lock()
            .map_err(|_| "Thumbnail gate lock was poisoned".to_string())?;
        if active.is_some() {
            return Err(
                "Thumbnail work is already queued or running; retry after it completes".to_string(),
            );
        }
        let mut random = [0u8; 16];
        getrandom::getrandom(&mut random)
            .map_err(|_| "Unable to create thumbnail request identity".to_string())?;
        let mut request_id = String::with_capacity(32);
        const HEX: &[u8; 16] = b"0123456789abcdef";
        for byte in random {
            request_id.push(HEX[(byte >> 4) as usize] as char);
            request_id.push(HEX[(byte & 15) as usize] as char);
        }
        let request = ActiveThumbnail {
            owner: owner.to_string(),
            request_id,
            cancelled: Arc::new(AtomicBool::new(false)),
        };
        *active = Some(request.clone());
        Ok(ThumbnailJob {
            active: Arc::clone(&self.active),
            request,
        })
    }

    pub(crate) fn cancel(&self, owner: &str, ticket: &ThumbnailTicket) -> Result<bool, String> {
        ticket.validate()?;
        let active = self
            .active
            .lock()
            .map_err(|_| "Thumbnail gate lock was poisoned".to_string())?;
        match active.as_ref() {
            Some(request) if request.owner == owner && request.request_id == ticket.request_id => {
                request.cancelled.store(true, Ordering::Release);
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

impl NativeThumbnailWork {
    pub(crate) fn cancel(&self, owner: &str, ticket: &ThumbnailTicket) -> Result<bool, String> {
        match ticket.lane {
            ThumbnailLane::Layer => self.layers.cancel(owner, ticket),
            ThumbnailLane::Asset => self.assets.cancel(owner, ticket),
        }
    }
}

impl ThumbnailJob {
    pub(crate) fn ticket(&self, lane: ThumbnailLane) -> ThumbnailTicket {
        ThumbnailTicket {
            schema_version: 1,
            lane,
            request_id: self.request.request_id.clone(),
        }
    }

    pub(crate) fn request_guard(&self) -> CancelOnDrop {
        CancelOnDrop(Arc::clone(&self.request.cancelled))
    }

    pub(crate) fn execute<T>(
        self,
        operation: impl FnOnce(&AtomicBool) -> Result<T, String>,
    ) -> Result<T, String> {
        ensure_not_cancelled(&self.request.cancelled)?;
        let result = operation(&self.request.cancelled);
        ensure_not_cancelled(&self.request.cancelled)?;
        result
    }
}

impl Drop for ThumbnailJob {
    fn drop(&mut self) {
        if let Ok(mut active) = self.active.lock() {
            if active
                .as_ref()
                .is_some_and(|request| request.request_id == self.request.request_id)
            {
                *active = None;
            }
        }
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

#[cfg(test)]
#[path = "native_thumbnail_ticket_tests.rs"]
mod ticket_tests;
