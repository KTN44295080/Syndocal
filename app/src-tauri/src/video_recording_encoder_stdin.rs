use std::{
    io::{self, Write},
    process::ChildStdin,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};

/// Writes the caller's frame directly; no frame queue or copied payload.
pub(crate) struct RecordingStdin {
    stdin: Option<ChildStdin>,
    control: Arc<InputControl>,
}

pub(super) struct InputControl {
    cancelled: AtomicBool,
    // The owner must acquire this gate before closing the raw handle.
    handle: Mutex<Option<usize>>,
}

impl RecordingStdin {
    pub(super) fn new(stdin: ChildStdin) -> (Self, Arc<InputControl>) {
        #[cfg(windows)]
        let handle = {
            use std::os::windows::io::AsRawHandle;
            stdin.as_raw_handle() as usize
        };
        #[cfg(not(windows))]
        let handle = 0;
        let control = Arc::new(InputControl {
            cancelled: AtomicBool::new(false),
            handle: Mutex::new(Some(handle)),
        });
        (
            Self {
                stdin: Some(stdin),
                control: Arc::clone(&control),
            },
            control,
        )
    }
}

impl Write for RecordingStdin {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        if self.control.cancelled.load(Ordering::SeqCst) {
            return Err(cancelled_error());
        }
        let result = self.stdin.as_mut().expect("owned stdin").write(buffer);
        // CancelIoEx can race with a successful partial write. Never permit
        // write_all to submit the remaining frame after cancellation.
        if self.control.cancelled.load(Ordering::SeqCst) {
            Err(cancelled_error())
        } else {
            result
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdin.as_mut().expect("owned stdin").flush()
    }
}

impl Drop for RecordingStdin {
    fn drop(&mut self) {
        let mut handle = self
            .control
            .handle
            .lock()
            .unwrap_or_else(|p| p.into_inner());
        *handle = None;
        self.stdin.take();
    }
}

impl InputControl {
    pub(super) fn after_reap(
        &self,
        stop: &AtomicBool,
        aborting: &AtomicBool,
        process_failed: bool,
        grace: Duration,
    ) -> Result<(), String> {
        #[cfg(not(windows))]
        {
            let _ = (stop, aborting, process_failed, grace);
            Ok(())
        }
        #[cfg(windows)]
        {
            use std::{thread, time::Instant};
            use windows::Win32::{
                Foundation::{ERROR_NOT_FOUND, HANDLE},
                System::IO::CancelIoEx,
            };
            let mut deadline = None;
            let mut cancel_error = None;
            loop {
                {
                    let handle = self.handle.lock().unwrap_or_else(|p| p.into_inner());
                    let Some(raw) = *handle else { break };
                    if stop.load(Ordering::Acquire) && deadline.is_none() {
                        deadline = Some(Instant::now() + grace);
                    }
                    if process_failed
                        || aborting.load(Ordering::Acquire)
                        || deadline.is_some_and(|deadline| Instant::now() >= deadline)
                    {
                        self.cancelled.store(true, Ordering::SeqCst);
                        // Retry even after success: cancellation can race with
                        // completion or with write() entering the OS. The latch
                        // prevents subsequent writes; the gate prevents reuse.
                        if let Err(error) = unsafe { CancelIoEx(HANDLE(raw as *mut _), None) } {
                            if error.code() != windows::core::HRESULT::from_win32(ERROR_NOT_FOUND.0)
                            {
                                cancel_error.get_or_insert_with(|| error.to_string());
                            }
                        }
                    }
                }
                thread::sleep(Duration::from_millis(5));
            }
            if self.cancelled.load(Ordering::SeqCst) {
                let mut error =
                    "FFmpeg stdin write was cancelled before complete frame EOF".to_string();
                if let Some(detail) = cancel_error {
                    error.push_str(&format!("; Cannot cancel FFmpeg stdin: {detail}"));
                }
                Err(error)
            } else {
                Ok(())
            }
        }
    }
}

fn cancelled_error() -> io::Error {
    io::Error::new(
        io::ErrorKind::Other,
        "FFmpeg stdin write cancellation requested",
    )
}
