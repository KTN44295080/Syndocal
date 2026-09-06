use std::{
    process::{Child, ChildStderr, ChildStdin, ExitStatus},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(25);
const KILL_RETRY_INTERVAL: Duration = Duration::from_millis(250);

/// A Child handle must never be dropped while process termination is unknown.
/// OS failure keeps this owner alive, including during panic unwinding.
pub(super) struct ReapingChild(Option<Child>);

impl ReapingChild {
    pub(super) fn new(child: Child) -> Self {
        Self(Some(child))
    }
    pub(super) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.0.as_mut().and_then(|child| child.stdin.take())
    }
    pub(super) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.0.as_mut().and_then(|child| child.stderr.take())
    }
    fn poll(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let result = self
            .0
            .as_mut()
            .expect("process remains owned until reaped")
            .try_wait()?;
        if result.is_some() {
            self.0.take();
        }
        Ok(result)
    }
    fn terminate(&mut self) -> std::io::Result<()> {
        match self.0.as_mut() {
            Some(child) => child.kill(),
            None => Ok(()),
        }
    }
}

impl Drop for ReapingChild {
    fn drop(&mut self) {
        while self.0.is_some() {
            let _ = self.terminate();
            if matches!(self.poll(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(KILL_RETRY_INTERVAL);
        }
    }
}

pub(super) fn supervise(
    mut child: ReapingChild,
    stop: &AtomicBool,
    finishing: &AtomicBool,
    aborting: &AtomicBool,
    grace: Duration,
) -> Result<(), String> {
    let mut deadline = None;
    let mut failure = None;
    let mut next_kill = Instant::now();
    loop {
        let now = Instant::now();
        if deadline.is_none() && (stop.load(Ordering::Acquire) || finishing.load(Ordering::Acquire))
        {
            deadline = Some(now + grace);
        }
        // Latch before checking exit: even an exit code of zero cannot publish
        // after abort/deadline, including a process that exits during kill.
        if failure.is_none() {
            if aborting.load(Ordering::Acquire) {
                failure = Some("FFmpeg encoding was aborted".to_string());
            } else if deadline.is_some_and(|deadline| now >= deadline) {
                failure = Some(format!(
                    "FFmpeg did not finish within {} ms after Stop; encoder termination was required",
                    grace.as_millis()
                ));
            }
        }
        match child.poll() {
            Ok(Some(status)) => {
                return match failure {
                    Some(error) => Err(error),
                    None if status.success() => Ok(()),
                    None => Err(format!("FFmpeg exited with {status}")),
                }
            }
            Ok(None) => {}
            Err(error) => {
                failure.get_or_insert_with(|| format!("Cannot poll FFmpeg termination: {error}"));
            }
        }
        if failure.is_some() && now >= next_kill {
            // A kill acknowledgement alone is not completion. Continue polling
            // and retain ownership until try_wait confirms termination.
            let _ = child.terminate();
            next_kill = now + KILL_RETRY_INTERVAL;
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
