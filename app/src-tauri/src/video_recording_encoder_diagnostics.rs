use std::{
    io::{self, Read},
    process::ChildStderr,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[cfg(windows)]
use std::time::Instant;

#[cfg(windows)]
const CANCEL_RETRY_INTERVAL: Duration = Duration::from_millis(5);

/// Owns the stderr reader join handle and the state needed to interrupt a
/// pending Windows pipe read without closing a handle from another thread.
pub(super) struct DiagnosticsReader {
    join: Option<JoinHandle<io::Result<Vec<u8>>>>,
    #[cfg(windows)]
    control: Arc<ReaderControl>,
}

impl DiagnosticsReader {
    pub(super) fn spawn(stderr: ChildStderr) -> io::Result<Self> {
        let control = Arc::new(ReaderControl::new(&stderr));
        let worker_control = Arc::clone(&control);
        let join = thread::Builder::new()
            .name("syndocal-recording-stderr".into())
            .spawn(move || ReaderTask::new(stderr, worker_control).run())?;
        Ok(Self {
            join: Some(join),
            #[cfg(windows)]
            control,
        })
    }

    #[cfg(test)]
    pub(super) fn from_join_handle(join: JoinHandle<io::Result<Vec<u8>>>) -> Self {
        Self {
            join: Some(join),
            #[cfg(windows)]
            control: Arc::new(ReaderControl::for_test()),
        }
    }

    pub(super) fn join_after_reap(
        mut self,
        cancel_required: bool,
        grace: Duration,
    ) -> Result<Vec<u8>, String> {
        #[cfg(windows)]
        let cancelled = cancel_required && !self.join_finished_within(grace);
        #[cfg(not(windows))]
        let cancelled = {
            // Unix keeps the existing blocking reader/join contract. The
            // platform has no corresponding cancellation in this tranche.
            let _ = (cancel_required, grace);
            false
        };

        #[cfg(windows)]
        let cancel_error = if cancelled {
            self.control
                .request_cancel(self.join.as_ref().expect("diagnostics reader join handle"))
                .err()
        } else {
            None
        };
        #[cfg(not(windows))]
        let cancel_error: Option<String> = None;
        let joined = self
            .join
            .take()
            .expect("diagnostics reader join handle is present")
            .join()
            .map_err(|_| "FFmpeg diagnostic reader panicked".to_string())?;
        if cancelled {
            let mut error = "FFmpeg diagnostic reader was cancelled before EOF".to_string();
            if let Some(cancel_error) = cancel_error {
                error.push_str("; ");
                error.push_str(&cancel_error);
            }
            if let Err(read_error) = joined {
                error.push_str(&format!("; {read_error}"));
            }
            return Err(error);
        }
        joined.map_err(|error| format!("Cannot finish reading FFmpeg diagnostics: {error}"))
    }

    #[cfg(windows)]
    fn join_finished_within(&self, grace: Duration) -> bool {
        let join = self.join.as_ref().expect("diagnostics reader join handle");
        let deadline = Instant::now() + grace;
        while !join.is_finished() {
            if Instant::now() >= deadline {
                return false;
            }
            thread::sleep(CANCEL_RETRY_INTERVAL);
        }
        true
    }
}

impl Drop for DiagnosticsReader {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            // Unwinding cannot abandon a live diagnostic reader.
            let _ = join.join();
        }
    }
}

struct ReaderControl {
    cancel_requested: AtomicBool,
    /// Serializes handle cancellation with the reader's final handle drop.
    drop_gate: Mutex<Option<usize>>,
}

impl ReaderControl {
    fn new(stderr: &ChildStderr) -> Self {
        #[cfg(windows)]
        {
            use std::os::windows::io::AsRawHandle;
            return Self {
                cancel_requested: AtomicBool::new(false),
                drop_gate: Mutex::new(Some(stderr.as_raw_handle() as usize)),
            };
        }
        #[cfg(not(windows))]
        {
            let _ = stderr;
            Self::without_handle()
        }
    }

    #[cfg(any(test, not(windows)))]
    fn without_handle() -> Self {
        Self {
            cancel_requested: AtomicBool::new(false),
            drop_gate: Mutex::new(None),
        }
    }

    #[cfg(all(test, windows))]
    fn for_test() -> Self {
        Self::without_handle()
    }

    #[cfg(windows)]
    fn request_cancel(&self, join: &JoinHandle<io::Result<Vec<u8>>>) -> Result<(), String> {
        use windows::Win32::{
            Foundation::{ERROR_NOT_FOUND, HANDLE},
            System::IO::CancelIoEx,
        };
        // Latch before touching the OS handle. A late successful read must
        // not turn an incomplete diagnostics stream into a successful finish.
        self.cancel_requested.store(true, Ordering::SeqCst);
        loop {
            if join.is_finished() {
                return Ok(());
            }
            let result = {
                // ReaderTask takes this gate before dropping ChildStderr. Do
                // not hold it while waiting: completion must be able to close
                // the handle and let JoinHandle::is_finished become true.
                let guard = self
                    .drop_gate
                    .lock()
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
                if join.is_finished() {
                    return Ok(());
                }
                let Some(raw_handle) = *guard else {
                    return Ok(());
                };
                let handle = HANDLE(raw_handle as *mut std::ffi::c_void);
                unsafe { CancelIoEx(handle, None) }
            };
            match result {
                Ok(()) => return Ok(()),
                Err(error)
                    if error.code() == windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0) =>
                {
                    // The reader can check cancel_requested just before its
                    // ReadFile begins. Releasing the gate and retrying closes
                    // that race; the flag prevents a later read once it sees
                    // the cancellation.
                    thread::sleep(CANCEL_RETRY_INTERVAL);
                }
                Err(error) => {
                    return Err(format!("Cannot cancel FFmpeg diagnostics read: {error}"));
                }
            }
        }
    }
}

struct ReaderTask {
    stderr: Option<ChildStderr>,
    control: Arc<ReaderControl>,
}

impl ReaderTask {
    fn new(stderr: ChildStderr, control: Arc<ReaderControl>) -> Self {
        Self {
            stderr: Some(stderr),
            control,
        }
    }

    fn run(mut self) -> io::Result<Vec<u8>> {
        let result = {
            let stderr = self
                .stderr
                .as_mut()
                .expect("diagnostics reader owns stderr until completion");
            super::super::recording_artifact::drain_encoder_stderr(CancellableStderr {
                stderr,
                control: Arc::clone(&self.control),
            })
        };
        self.finish();
        result
    }

    fn finish(&mut self) {
        let mut guard = self
            .control
            .drop_gate
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        *guard = None;
        self.stderr.take();
    }
}

impl Drop for ReaderTask {
    fn drop(&mut self) {
        self.finish();
    }
}

struct CancellableStderr<'a> {
    stderr: &'a mut ChildStderr,
    control: Arc<ReaderControl>,
}

impl Read for CancellableStderr<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        if self.control.cancel_requested.load(Ordering::SeqCst) {
            return Err(cancelled_error());
        }
        let result = self.stderr.read(buffer);
        // CancelIoEx can race with a successful ReadFile completion. Latch
        // the cancellation in either case so the drain loop never reads again.
        if self.control.cancel_requested.load(Ordering::SeqCst) {
            Err(cancelled_error())
        } else {
            result
        }
    }
}

fn cancelled_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "FFmpeg diagnostic reader cancellation requested",
    )
}
