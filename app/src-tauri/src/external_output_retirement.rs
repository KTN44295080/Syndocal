//! Cold-path, bounded retirement wait. The caller retains the worker and all
//! ownership leases when this returns false; only a finished owner may be joined.
use std::time::{Duration, Instant};

#[cfg(test)]
pub(super) fn stop_after_denied_admission_for_test(stop: Option<&std::sync::atomic::AtomicBool>) {
    std::thread_local! {
        static STOP_ON_DENIAL: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }
    STOP_ON_DENIAL.with(|pending| match stop {
        None => pending.set(true),
        Some(stop) if pending.replace(false) => stop.store(true, std::sync::atomic::Ordering::Release),
        Some(_) => {}
    });
}

pub(crate) const OUTPUT_WORKER_STOP_TIMEOUT: Duration = Duration::from_secs(1);

pub(crate) fn wait_for_output_worker(
    timeout: Duration,
    mut finished: impl FnMut() -> bool,
) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if finished() {
            return true;
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return false;
        }
        std::thread::sleep(remaining.min(Duration::from_millis(1)));
    }
}
