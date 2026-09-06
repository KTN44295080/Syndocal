//! Direct encoder process supervision, independent of frame rendering and writes.
//! Inherited pipe handles and a stuck renderer still retain the outer worker;
//! this is a termination deadline for the owned child, not thread cancellation.
use super::super::recording_artifact;
use std::{
    io,
    process::{ChildStdin, Command},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread::{self, JoinHandle},
    time::Duration,
};

#[path = "video_recording_encoder_process.rs"]
mod process;

const ENCODER_STOP_GRACE: Duration = Duration::from_secs(5);

pub(crate) struct RecordingEncoder {
    stdin: Option<ChildStdin>,
    diagnostics: Option<JoinHandle<io::Result<Vec<u8>>>>,
    supervisor: Option<JoinHandle<Result<(), String>>>,
    finishing: Arc<AtomicBool>,
    aborting: Arc<AtomicBool>,
}

impl RecordingEncoder {
    pub(crate) fn spawn(command: &mut Command, stop: Arc<AtomicBool>) -> Result<Self, String> {
        Self::spawn_with_grace(command, stop, ENCODER_STOP_GRACE)
    }

    fn spawn_with_grace(
        command: &mut Command,
        stop: Arc<AtomicBool>,
        grace: Duration,
    ) -> Result<Self, String> {
        let mut child = process::ReapingChild::new(
            command
                .spawn()
                .map_err(|error| format!("Failed to start FFmpeg: {error}"))?,
        );
        let stdin = child.take_stdin().ok_or("FFmpeg stdin was unavailable")?;
        let stderr = child.take_stderr().ok_or("FFmpeg stderr was unavailable")?;
        let finishing = Arc::new(AtomicBool::new(false));
        let aborting = Arc::new(AtomicBool::new(false));
        // A failed thread spawn drops its closure. Keep a second owner of this
        // handoff cell so dropping a closure cannot abandon a live Child.
        let handoff = Arc::new(Mutex::new(Some(child)));
        let worker_handoff = Arc::clone(&handoff);
        let worker_finishing = Arc::clone(&finishing);
        let worker_aborting = Arc::clone(&aborting);
        let supervisor = thread::Builder::new()
            .name("syndocal-recording-encoder".into())
            .spawn(move || {
                let child = worker_handoff
                    .lock()
                    .unwrap_or_else(|p| p.into_inner())
                    .take()
                    .expect("encoder handoff has one consumer");
                process::supervise(child, &stop, &worker_finishing, &worker_aborting, grace)
            })
            .map_err(|error| format!("Cannot supervise FFmpeg: {error}"))?;
        let mut encoder = Self {
            stdin: Some(stdin),
            diagnostics: None,
            supervisor: Some(supervisor),
            finishing,
            aborting,
        };
        match thread::Builder::new()
            .name("syndocal-recording-stderr".into())
            .spawn(move || recording_artifact::drain_encoder_stderr(stderr))
        {
            Ok(reader) => encoder.diagnostics = Some(reader),
            Err(error) => {
                let error = encoder
                    .fail(format!("Cannot read FFmpeg diagnostics: {error}"))
                    .expect_err("a primary setup error cannot become encoder success");
                return Err(error);
            }
        }
        Ok(encoder)
    }

    pub(crate) fn take_stdin(&mut self) -> Option<ChildStdin> {
        self.stdin.take()
    }

    // The caller must drop its taken stdin before finishing, allowing MP4 EOF.
    pub(crate) fn finish(mut self) -> Result<Vec<u8>, String> {
        self.stdin.take();
        self.finishing.store(true, Ordering::Release);
        self.join_owned(None)
    }

    pub(crate) fn fail(mut self, error: String) -> Result<Vec<u8>, String> {
        self.stdin.take();
        self.aborting.store(true, Ordering::Release);
        self.join_owned(Some(error))
    }

    fn join_owned(&mut self, primary: Option<String>) -> Result<Vec<u8>, String> {
        let outcome = self
            .supervisor
            .take()
            .map(|worker| {
                worker
                    .join()
                    .unwrap_or_else(|_| Err("FFmpeg supervisor panicked".into()))
            })
            .unwrap_or(Ok(()));
        // A process exit does not prove pipe EOF: inherited handles can keep
        // this join pending. Retain the outer recording worker in that case.
        let diagnostics = self
            .diagnostics
            .take()
            .map(|reader| {
                reader
                    .join()
                    .map_err(|_| "FFmpeg diagnostic reader panicked".to_string())?
                    .map_err(|error| format!("Cannot finish reading FFmpeg diagnostics: {error}"))
            })
            .unwrap_or_else(|| Ok(Vec::new()));
        let mut errors = Vec::new();
        if let Some(primary) = primary {
            errors.push(primary);
        }
        if let Err(error) = outcome {
            errors.push(error);
        }
        match diagnostics {
            Ok(tail) if errors.is_empty() => Ok(tail),
            Ok(tail) => {
                let diagnostic = String::from_utf8_lossy(&tail);
                if !diagnostic.trim().is_empty() {
                    errors.push(diagnostic.trim().to_string());
                }
                Err(errors.join("; "))
            }
            Err(error) => {
                errors.push(error);
                Err(errors.join("; "))
            }
        }
    }
}

impl Drop for RecordingEncoder {
    fn drop(&mut self) {
        self.stdin.take();
        self.aborting.store(true, Ordering::Release);
        let _ = self.join_owned(None);
    }
}

#[cfg(test)]
#[path = "video_recording_encoder_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "video_recording_diagnostics_tests.rs"]
mod diagnostics_tests;
