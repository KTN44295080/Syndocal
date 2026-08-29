//! Bounded ownership transfer for short-lived capture subprocesses.
//!
//! FFmpeg owns the other end of the stdout/stderr pipes consumed by capture
//! readers.  A caller must therefore never wait for those readers after its
//! bounded kill/reap deadline expires: the reader may still be blocked until
//! the child actually exits.  This module consumes the `Child` at that
//! boundary and hands it to an owned background reaper.  If the reaper cannot
//! be started, the child remains in an in-process quarantine instead of being
//! dropped (which would silently orphan the external process).
//!
//! This is an in-process ownership guarantee only. A Windows Job Object is not
//! installed in this bounded tranche, so OS behavior when Syndocal itself exits
//! while a deferred reaper is still pending remains an explicit unverified
//! boundary rather than a claim that this module controls app-exit descendants.

use std::{
    process::Child,
    sync::{mpsc, LazyLock, Mutex},
    thread,
    time::{Duration, Instant},
};

const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum ProcessTermination {
    /// The caller observed a terminal child status. Pipe reader threads can
    /// now be joined without a child-owned pipe remaining open.
    Reaped,
    /// The caller transferred the child to background ownership. Reader
    /// handles must be dropped/detached rather than joined on this path.
    Deferred { detail: String },
}

impl ProcessTermination {
    pub(crate) fn is_reaped(&self) -> bool {
        matches!(self, Self::Reaped)
    }

    pub(crate) fn into_result(self) -> Result<(), String> {
        match self {
            Self::Reaped => Ok(()),
            Self::Deferred { detail } => Err(detail),
        }
    }
}

struct QuarantinedChild {
    child: Child,
    label: String,
}

/// Spawn failures are exceptionally rare, but dropping the `Child` at that
/// point would discard the only handle capable of reaping it. Keep ownership
/// here and retry reaper handoff during a later cleanup attempt.
static QUARANTINED_CHILDREN: LazyLock<Mutex<Vec<QuarantinedChild>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

pub(crate) fn terminate_process_or_defer(
    mut child: Child,
    label: &str,
    exit_grace: Duration,
) -> ProcessTermination {
    retry_quarantined_reapers();

    let inspection_error = match child.try_wait() {
        Ok(Some(_)) => return ProcessTermination::Reaped,
        Ok(None) => None,
        Err(error) => Some(format!("pre-termination status inspection failed: {error}")),
    };
    let kill_error = child
        .kill()
        .err()
        .map(|error| format!("kill failed: {error}"));
    let reap_result = wait_for_process_exit(&mut child, exit_grace, label);
    match reap_result {
        Ok(Some(_)) => ProcessTermination::Reaped,
        Ok(None) => defer_child(
            child,
            label,
            deferred_detail(inspection_error, kill_error, "kill/reap deadline expired"),
        ),
        Err(error) => defer_child(
            child,
            label,
            deferred_detail(
                inspection_error,
                kill_error,
                &format!("reap status inspection failed: {error}"),
            ),
        ),
    }
}

fn deferred_detail(
    inspection_error: Option<String>,
    kill_error: Option<String>,
    terminal_detail: &str,
) -> String {
    let mut details = Vec::new();
    if let Some(error) = inspection_error {
        details.push(error);
    }
    if let Some(error) = kill_error {
        details.push(error);
    }
    details.push(terminal_detail.to_string());
    details.join("; ")
}

fn defer_child(child: Child, label: &str, detail: String) -> ProcessTermination {
    defer_child_with(child, label, detail, hand_off_to_background_reaper)
}

fn defer_child_with<Handoff>(
    child: Child,
    label: &str,
    detail: String,
    handoff: Handoff,
) -> ProcessTermination
where
    Handoff: FnOnce(Child, String) -> Result<(), (Child, String)>,
{
    match handoff(child, label.to_string()) {
        Ok(()) => ProcessTermination::Deferred {
            detail: format!(
                "{label} process cleanup deferred to the background reaper after bounded kill/reap: {detail}"
            ),
        },
        Err((child, handoff_error)) => {
            quarantine_child(child, label.to_string());
            ProcessTermination::Deferred {
                detail: format!(
                    "{label} process cleanup deferred but the background reaper could not start ({handoff_error}); the child is retained in process quarantine for a later reaper handoff: {detail}"
                ),
            }
        }
    }
}

fn hand_off_to_background_reaper(child: Child, label: String) -> Result<(), (Child, String)> {
    hand_off_to_background_reaper_with(child, label, |receiver| {
        thread::Builder::new()
            .name("syndocal-capture-process-reaper".to_string())
            .spawn(move || {
                if let Ok(quarantined) = receiver.recv() {
                    let _ = reap_until_exit(quarantined.child, quarantined.label);
                }
            })
            .map_err(|error| format!("thread spawn failed: {error}"))
    })
}

fn hand_off_to_background_reaper_with<Start>(
    child: Child,
    label: String,
    start: Start,
) -> Result<(), (Child, String)>
where
    Start: FnOnce(mpsc::Receiver<QuarantinedChild>) -> Result<std::thread::JoinHandle<()>, String>,
{
    // Start the thread before moving the child into the channel. On a spawn
    // failure the caller still owns `child`, so it can be quarantined instead
    // of being dropped/orphaned.
    let (sender, receiver) = mpsc::sync_channel::<QuarantinedChild>(1);
    let reaper = match start(receiver) {
        Ok(reaper) => reaper,
        Err(error) => return Err((child, error)),
    };
    match sender.send(QuarantinedChild { child, label }) {
        Ok(()) => {
            // This detached thread owns the child until it has a terminal
            // status. Joining it here would recreate the UI-path stall this
            // lifecycle boundary is intended to prevent.
            drop(reaper);
            Ok(())
        }
        Err(mpsc::SendError(quarantined)) => {
            drop(reaper);
            Err((
                quarantined.child,
                "background reaper stopped before child handoff".to_string(),
            ))
        }
    }
}

fn reap_until_exit(mut child: Child, _label: String) -> std::process::ExitStatus {
    #[cfg(test)]
    let child_id = child.id();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                #[cfg(test)]
                record_test_reaped_child(child_id);
                return status;
            }
            Ok(None) | Err(_) => {
                // Repeating kill is deliberate: a failed/early kill must not
                // turn into an unowned process merely because this run was
                // racing startup or a driver teardown.
                let _ = child.kill();
                thread::sleep(PROCESS_EXIT_POLL_INTERVAL);
            }
        }
    }
}

fn quarantine_child(child: Child, label: String) {
    let mut quarantine = match QUARANTINED_CHILDREN.lock() {
        Ok(quarantine) => quarantine,
        Err(poisoned) => poisoned.into_inner(),
    };
    quarantine.push(QuarantinedChild { child, label });
}

fn retry_quarantined_reapers() {
    retry_quarantined_reapers_with(hand_off_to_background_reaper);
}

fn retry_quarantined_reapers_with<Handoff>(mut handoff: Handoff)
where
    Handoff: FnMut(Child, String) -> Result<(), (Child, String)>,
{
    let pending = {
        let mut quarantine = match QUARANTINED_CHILDREN.lock() {
            Ok(quarantine) => quarantine,
            Err(poisoned) => poisoned.into_inner(),
        };
        std::mem::take(&mut *quarantine)
    };
    for quarantined in pending {
        if let Err((child, handoff_error)) = handoff(quarantined.child, quarantined.label.clone()) {
            quarantine_child(child, quarantined.label);
            // Do not log/process UI work from the cleanup path. The active
            // caller receives a visible deferred error for its own child, and
            // this retained child is retried on the next cleanup attempt.
            let _ = handoff_error;
        }
    }
}

fn wait_for_process_exit(
    child: &mut Child,
    grace: Duration,
    label: &str,
) -> Result<Option<std::process::ExitStatus>, String> {
    // `Duration::ZERO` is a deliberate no-wait contract. Production passes a
    // finite grace, while this exact boundary also lets tests prove the
    // deferred path without a scheduler-dependent post-kill race.
    if grace.is_zero() {
        return Ok(None);
    }
    let started = Instant::now();
    loop {
        match child
            .try_wait()
            .map_err(|error| format!("Unable to inspect {label} completion: {error}"))?
        {
            Some(status) => return Ok(Some(status)),
            None => {
                let Some(remaining) = grace.checked_sub(started.elapsed()) else {
                    return Ok(None);
                };
                thread::sleep(remaining.min(PROCESS_EXIT_POLL_INTERVAL));
            }
        }
    }
}

#[cfg(test)]
pub(crate) fn deferred_termination_for_test(detail: &str) -> ProcessTermination {
    ProcessTermination::Deferred {
        detail: format!("test deferred cleanup: {detail}"),
    }
}

#[cfg(test)]
static TEST_LIFECYCLE_SERIAL: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));
#[cfg(test)]
static TEST_REAPED_CHILDREN: LazyLock<Mutex<Vec<u32>>> = LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
fn record_test_reaped_child(child_id: u32) {
    let mut reaped = match TEST_REAPED_CHILDREN.lock() {
        Ok(reaped) => reaped,
        Err(poisoned) => poisoned.into_inner(),
    };
    reaped.push(child_id);
}

#[cfg(test)]
fn wait_for_test_reaped_child(child_id: u32) -> bool {
    let deadline = Instant::now() + Duration::from_secs(1);
    loop {
        let was_reaped = {
            let mut reaped = match TEST_REAPED_CHILDREN.lock() {
                Ok(reaped) => reaped,
                Err(poisoned) => poisoned.into_inner(),
            };
            reaped
                .iter()
                .position(|recorded| *recorded == child_id)
                .map(|index| reaped.swap_remove(index))
                .is_some()
        };
        if was_reaped {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(5));
    }
}

#[cfg(test)]
fn quarantined_child_count() -> usize {
    match QUARANTINED_CHILDREN.lock() {
        Ok(quarantine) => quarantine.len(),
        Err(poisoned) => poisoned.into_inner().len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(windows)]
    #[test]
    fn real_zero_grace_deadline_defers_and_background_reaper_proves_the_child_reaped() {
        let _serial = TEST_LIFECYCLE_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(quarantined_child_count(), 0);
        let child = long_running_test_child();
        let child_id = child.id();
        let termination = terminate_process_or_defer(
            child,
            "real deadline background-reaper test",
            Duration::ZERO,
        );
        match termination {
            ProcessTermination::Deferred { detail } => {
                assert!(detail.contains("background reaper"));
                assert!(detail.contains("kill/reap deadline expired"));
            }
            ProcessTermination::Reaped => {
                panic!("zero-grace cleanup must exercise deferred handoff")
            }
        }
        assert!(
            wait_for_test_reaped_child(child_id),
            "the background reaper must observe a terminal child status"
        );
        assert_eq!(quarantined_child_count(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn spawn_failure_quarantines_then_a_later_retry_hands_off_and_reaps() {
        let _serial = TEST_LIFECYCLE_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(quarantined_child_count(), 0);
        let termination = defer_child_with(
            long_running_test_child(),
            "forced spawn failure test",
            "kill/reap deadline expired".to_string(),
            |child, label| {
                hand_off_to_background_reaper_with(child, label, |_receiver| {
                    Err("forced reaper spawn failure".to_string())
                })
            },
        );
        let detail = match termination {
            ProcessTermination::Deferred { detail } => detail,
            ProcessTermination::Reaped => panic!("forced handoff failure must defer to quarantine"),
        };
        assert!(detail.contains("forced reaper spawn failure"));
        assert!(detail.contains("retained in process quarantine"));
        assert_eq!(quarantined_child_count(), 1);

        let (reaped_tx, reaped_rx) = mpsc::sync_channel(1);
        retry_quarantined_reapers_with(move |child, label| {
            let reaped_tx = reaped_tx.clone();
            thread::spawn(move || {
                let _ = reap_until_exit(child, label);
                let _ = reaped_tx.send(());
            });
            Ok(())
        });
        reaped_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("retry reaper must report a terminal child status");
        assert_eq!(quarantined_child_count(), 0);
    }

    #[cfg(windows)]
    #[test]
    fn receiver_disconnect_returns_child_to_quarantine_then_retry_reaps_it() {
        let _serial = TEST_LIFECYCLE_SERIAL
            .lock()
            .unwrap_or_else(|error| error.into_inner());
        assert_eq!(quarantined_child_count(), 0);
        let termination = defer_child_with(
            long_running_test_child(),
            "forced handoff send failure test",
            "kill/reap deadline expired".to_string(),
            force_receiver_disconnect_handoff,
        );
        let detail = match termination {
            ProcessTermination::Deferred { detail } => detail,
            ProcessTermination::Reaped => {
                panic!("receiver disconnect must preserve child ownership")
            }
        };
        assert!(detail.contains("background reaper stopped before child handoff"));
        assert!(detail.contains("retained in process quarantine"));
        assert_eq!(quarantined_child_count(), 1);

        let (reaped_tx, reaped_rx) = mpsc::sync_channel(1);
        retry_quarantined_reapers_with(move |child, label| {
            let reaped_tx = reaped_tx.clone();
            thread::spawn(move || {
                let _ = reap_until_exit(child, label);
                let _ = reaped_tx.send(());
            });
            Ok(())
        });
        reaped_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("retry reaper must report a terminal child status");
        assert_eq!(quarantined_child_count(), 0);
    }

    #[cfg(windows)]
    fn long_running_test_child() -> Child {
        let command_shell = std::env::var_os("COMSPEC").expect("COMSPEC must be available");
        std::process::Command::new(command_shell)
            .args(["/d", "/s", "/c", "for /L %i in (1,1,2147483647) do @rem"])
            .spawn()
            .expect("long-running test child must start")
    }

    #[cfg(windows)]
    fn force_receiver_disconnect_handoff(
        child: Child,
        label: String,
    ) -> Result<(), (Child, String)> {
        hand_off_to_background_reaper_with(child, label, |receiver| {
            let (dropped_tx, dropped_rx) = mpsc::sync_channel(1);
            let reaper = thread::spawn(move || {
                drop(receiver);
                let _ = dropped_tx.send(());
            });
            dropped_rx
                .recv_timeout(Duration::from_secs(1))
                .map_err(|error| {
                    format!("forced receiver disconnect did not become ready: {error}")
                })?;
            Ok(reaper)
        })
    }
}
