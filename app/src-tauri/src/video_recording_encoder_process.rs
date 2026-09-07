use std::{
    process::{Child, ChildStderr, ChildStdin, ExitStatus},
    sync::Arc,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, LazyLock, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

const POLL_INTERVAL: Duration = Duration::from_millis(25);
const KILL_RETRY_INTERVAL: Duration = Duration::from_millis(250);

/// A Child handle must never be dropped while process termination is unknown.
/// Windows also owns a Job Object so descendants holding inherited pipes are
/// terminated with the encoder.
pub(super) struct ReapingChild {
    child: Option<Child>,
    #[cfg(windows)]
    job: Option<Arc<ProcessJob>>,
}

#[cfg(windows)]
pub(super) type ProcessTreeGuard = Arc<dyn Send + Sync>;

struct QuarantinedChild {
    child: ReapingChild,
    label: &'static str,
}

/// A thread creation failure must not turn a live process into an unowned
/// process. Quarantine is process-lifetime ownership and is retried on the
/// next encoder teardown.
static QUARANTINED_CHILDREN: LazyLock<Mutex<Vec<QuarantinedChild>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

impl ReapingChild {
    pub(super) fn new(mut child: Child) -> Result<Self, String> {
        retry_quarantined_children();
        #[cfg(windows)]
        let job = match ProcessJob::for_child(&child) {
            Ok(job) => Some(Arc::new(job)),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!(
                    "cannot establish FFmpeg process-tree ownership: {error}"
                ));
            }
        };
        Ok(Self {
            child: Some(child),
            #[cfg(windows)]
            job,
        })
    }

    #[cfg(windows)]
    pub(super) fn process_tree_guard(&self) -> ProcessTreeGuard {
        self.job
            .as_ref()
            .map(|job| Arc::clone(job) as ProcessTreeGuard)
            .expect("recording process tree job is installed")
    }
    pub(super) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.child.as_mut().and_then(|child| child.stdin.take())
    }
    pub(super) fn take_stderr(&mut self) -> Option<ChildStderr> {
        self.child.as_mut().and_then(|child| child.stderr.take())
    }
    fn poll(&mut self) -> std::io::Result<Option<ExitStatus>> {
        let result = self
            .child
            .as_mut()
            .expect("process remains owned until reaped")
            .try_wait()?;
        if result.is_some() {
            self.child.take();
        }
        Ok(result)
    }
    fn terminate(&mut self) -> std::io::Result<()> {
        #[cfg(windows)]
        if let Some(job) = &self.job {
            return job.terminate();
        }
        match self.child.as_mut() {
            Some(child) => child.kill(),
            None => Ok(()),
        }
    }

    fn defer_background(self, label: &'static str) -> String {
        let (sender, receiver) = mpsc::sync_channel(1);
        let reaper = match thread::Builder::new()
            .name(format!("syndocal-{label}-process-reaper"))
            .spawn(move || {
                if let Ok(child) = receiver.recv() {
                    reap_until_exit(child);
                }
            }) {
            Ok(reaper) => reaper,
            Err(error) => {
                let mut quarantine = QUARANTINED_CHILDREN
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                quarantine.push(QuarantinedChild { child: self, label });
                return format!(
                    "{label} process reaper start failed: {error}; process quarantined"
                );
            }
        };
        match sender.send(self) {
            Ok(()) => {
                drop(reaper);
                format!("{label} process cleanup deferred to its owned background reaper")
            }
            Err(mpsc::SendError(child)) => {
                let mut quarantine = QUARANTINED_CHILDREN
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                quarantine.push(QuarantinedChild { child, label });
                drop(reaper);
                format!("{label} process reaper handoff failed; process quarantined")
            }
        }
    }
}

impl Drop for ReapingChild {
    fn drop(&mut self) {
        if self.child.is_some() {
            // Do not block a panic/unwind or app teardown on an OS wait. Move
            // the complete owner to a named reaper while retaining a bounded
            // retry/quarantine path if thread creation fails.
            let owned = Self {
                child: self.child.take(),
                #[cfg(windows)]
                job: self.job.take(),
            };
            let _ = owned.defer_background("recording");
        }
    }
}

fn retry_quarantined_children() {
    let pending = {
        let mut quarantine = QUARANTINED_CHILDREN
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        std::mem::take(&mut *quarantine)
    };
    for pending in pending {
        let detail = pending.child.defer_background(pending.label);
        if detail.contains("quarantined") {
            // `defer_background` has already restored ownership to the
            // quarantine in this case.
        }
    }
}

fn reap_until_exit(mut child: ReapingChild) {
    while child.child.is_some() {
        let _ = child.terminate();
        if matches!(child.poll(), Ok(Some(_))) {
            break;
        }
        thread::sleep(KILL_RETRY_INTERVAL);
    }
}

#[cfg(windows)]
struct ProcessJob(std::os::windows::io::OwnedHandle);

#[cfg(windows)]
impl ProcessJob {
    fn for_child(child: &Child) -> std::io::Result<Self> {
        use std::{
            mem::size_of,
            os::windows::io::{AsRawHandle, FromRawHandle},
        };
        use windows::Win32::{
            Foundation::HANDLE,
            System::JobObjects::{
                AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            },
        };
        let job = unsafe { CreateJobObjectW(None, None) }
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        if let Err(error) = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        } {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
            return Err(std::io::Error::other(error.to_string()));
        }
        if let Err(error) =
            unsafe { AssignProcessToJobObject(job, HANDLE(child.as_raw_handle() as *mut _)) }
        {
            let _ = unsafe { windows::Win32::Foundation::CloseHandle(job) };
            return Err(std::io::Error::other(error.to_string()));
        }
        Ok(Self(unsafe {
            std::os::windows::io::OwnedHandle::from_raw_handle(job.0)
        }))
    }

    fn terminate(&self) -> std::io::Result<()> {
        use std::os::windows::io::AsRawHandle;
        let handle = windows::Win32::Foundation::HANDLE(self.0.as_raw_handle());
        unsafe { windows::Win32::System::JobObjects::TerminateJobObject(handle, 1) }
            .map_err(|error| std::io::Error::other(error.to_string()))
    }
}

pub(super) fn supervise(
    mut child: ReapingChild,
    stop: &AtomicBool,
    finishing: &AtomicBool,
    aborting: &AtomicBool,
    grace: Duration,
    mut observe_cancellation: impl FnMut(),
) -> Result<(), String> {
    let mut deadline = None;
    let mut total_deadline = None;
    let mut failure = None;
    let mut next_kill = Instant::now();
    loop {
        let now = Instant::now();
        observe_cancellation();
        if deadline.is_none() && (stop.load(Ordering::Acquire) || finishing.load(Ordering::Acquire))
        {
            deadline = Some(now + grace);
        }
        if total_deadline.is_none()
            && (stop.load(Ordering::Acquire)
                || finishing.load(Ordering::Acquire)
                || aborting.load(Ordering::Acquire))
        {
            total_deadline = Some(now + crate::video_recording_lifecycle::TOTAL_STOP_DEADLINE);
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
                total_deadline
                    .get_or_insert(now + crate::video_recording_lifecycle::TOTAL_STOP_DEADLINE);
            }
        }
        if failure.is_some() && total_deadline.is_none() {
            total_deadline = Some(now + crate::video_recording_lifecycle::TOTAL_STOP_DEADLINE);
        }
        if failure.is_some() && now >= next_kill {
            // A kill acknowledgement alone is not completion. Continue polling
            // and retain ownership until try_wait confirms termination.
            let _ = child.terminate();
            next_kill = now + KILL_RETRY_INTERVAL;
        }
        if failure.is_some() && total_deadline.is_some_and(|deadline| now >= deadline) {
            let detail = failure.expect("failure is present at total stop deadline");
            let deferred = child.defer_background("recording-encoder");
            return Err(format!("{detail}; {deferred}"));
        }
        std::thread::sleep(POLL_INTERVAL);
    }
}
