use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};

const ACK_WAIT_TIMEOUT: Duration = Duration::from_secs(3);
const STOP_WAIT_SLICE: Duration = Duration::from_millis(10);

pub(super) fn require_running(stop: Option<&AtomicBool>) -> Result<(), String> {
    if stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
        return Err("Timeline Follow settlement acknowledgement stopped".to_string());
    }
    Ok(())
}

fn wait_error(error: mpsc::RecvTimeoutError) -> String {
    format!("Timeline Follow settlement acknowledgement failed: {error}")
}

pub(super) fn wait_for_publication(
    receiver: &mpsc::Receiver<Result<(), String>>,
    stop: Option<&AtomicBool>,
) -> Result<(), String> {
    let Some(stop) = stop else {
        // Preserve the legacy API's single wait, timeout and error contract.
        return receiver.recv_timeout(ACK_WAIT_TIMEOUT).map_err(wait_error)?;
    };
    wait_for_publication_until(
        stop,
        Instant::now() + ACK_WAIT_TIMEOUT,
        || receiver.try_recv(),
        |timeout| receiver.recv_timeout(timeout),
        Instant::now,
    )
}

/// The clock and receive operations are explicit so the stop/deadline races
/// can be exercised without scheduling sleeps or a live engine thread.
pub(super) fn wait_for_publication_until(
    stop: &AtomicBool,
    deadline: Instant,
    mut try_receive: impl FnMut() -> Result<Result<(), String>, mpsc::TryRecvError>,
    mut receive: impl FnMut(Duration) -> Result<Result<(), String>, mpsc::RecvTimeoutError>,
    mut now: impl FnMut() -> Instant,
) -> Result<(), String> {
    loop {
        // A definitive reply that is already available wins over Stop. Once
        // applied, the engine owns the receipt even if this waiter goes away.
        match try_receive() {
            Ok(result) => return result,
            Err(mpsc::TryRecvError::Disconnected) => {
                return Err(wait_error(mpsc::RecvTimeoutError::Disconnected));
            }
            Err(mpsc::TryRecvError::Empty) => {}
        }
        require_running(Some(stop))?;
        let remaining = deadline.saturating_duration_since(now());
        if remaining.is_zero() {
            return Err(wait_error(mpsc::RecvTimeoutError::Timeout));
        }
        match receive(remaining.min(STOP_WAIT_SLICE)) {
            Ok(result) => return result,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(error) => return Err(wait_error(error)),
        }
    }
}
