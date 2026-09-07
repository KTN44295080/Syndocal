use super::video_recording_lifecycle;
use serde::Serialize;
use std::{
    sync::{atomic::AtomicBool, Arc, Mutex},
    thread::JoinHandle,
};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct VideoRecordingStatus {
    pub(crate) active: bool,
    pub(crate) output_id: Option<protocol::VideoOutputId>,
    pub(crate) path: Option<String>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frame_rate: u32,
    pub(crate) frames_written: u64,
    pub(crate) dropped_frames: u64,
    pub(crate) audio_requested: bool,
    pub(crate) audio_included: bool,
    pub(crate) audio_track_count: usize,
    pub(crate) started_unix_ms: Option<u64>,
    pub(crate) renderer_process_isolated: bool,
    pub(crate) renderer_force_termination: bool,
    pub(crate) stop_deadline_ms: u64,
    pub(crate) last_error: Option<String>,
}

impl Default for VideoRecordingStatus {
    fn default() -> Self {
        Self {
            active: false,
            output_id: None,
            path: None,
            width: 0,
            height: 0,
            frame_rate: 30,
            frames_written: 0,
            dropped_frames: 0,
            audio_requested: false,
            audio_included: false,
            audio_track_count: 0,
            started_unix_ms: None,
            renderer_process_isolated: true,
            renderer_force_termination: true,
            stop_deadline_ms: video_recording_lifecycle::TOTAL_STOP_DEADLINE.as_millis() as u64,
            last_error: None,
        }
    }
}

#[derive(Default)]
pub(crate) struct VideoRecordingRuntime {
    worker: Option<video_recording_lifecycle::RecordingWorkerLifecycle>,
    status: Arc<Mutex<VideoRecordingStatus>>,
}

impl Drop for VideoRecordingRuntime {
    fn drop(&mut self) {
        if let Some(mut worker) = self.worker.take() {
            // Runtime teardown has a hard deadline. A renderer call or
            // inherited encoder pipe can still be pending, so the lifecycle
            // transfers the original owner to its named reaper on expiry
            // instead of blocking app teardown or abandoning the worker.
            worker.request_stop();
            let _ = worker.reap_with_deadline(video_recording_lifecycle::TOTAL_STOP_DEADLINE);
        }
    }
}

impl VideoRecordingRuntime {
    pub(crate) fn ensure_recording_worker_available(&mut self) -> Result<(), String> {
        self.reap_completed_worker()?;
        if let Some(worker) = self.worker.as_ref() {
            return Err(if worker.is_stopping() {
                "A previous video output recording is still stopping".to_string()
            } else {
                "A previous video output recording worker is still finishing".to_string()
            });
        }
        let active = self
            .status
            .lock()
            .map_err(|_| "Video recording status lock was poisoned".to_string())?
            .active;
        if active {
            return Err("A video output recording is already active".to_string());
        }
        Ok(())
    }

    pub(crate) fn install_worker(
        &mut self,
        stop: Arc<AtomicBool>,
        worker: JoinHandle<()>,
        status: Arc<Mutex<VideoRecordingStatus>>,
    ) -> Option<VideoRecordingStatus> {
        self.worker = Some(video_recording_lifecycle::RecordingWorkerLifecycle::new(
            stop, worker,
        ));
        self.status = status;
        self.status.lock().ok().map(|status| status.clone())
    }

    pub(crate) fn status_snapshot(&self) -> Result<VideoRecordingStatus, String> {
        self.status
            .lock()
            .map_err(|_| "Video recording status lock was poisoned".to_string())
            .map(|status| status.clone())
    }

    fn reap_completed_worker(&mut self) -> Result<(), String> {
        let reap = {
            let Some(worker) = self.worker.as_mut() else {
                return Ok(());
            };
            worker.reap_if_complete()
        };
        let Some(reap) = reap else {
            return Ok(());
        };
        self.worker = None;
        if reap == video_recording_lifecycle::WorkerReap::Panicked {
            let error = "Video recording worker panicked".to_string();
            match self.status.lock() {
                Ok(mut status) => {
                    status.active = false;
                    status.last_error = Some(error.clone());
                }
                Err(_) => {
                    return Err(format!("{error}; video recording status lock was poisoned"));
                }
            }
            return Err(error);
        }
        Ok(())
    }
}

pub(crate) fn stop_video_output_recording_runtime(
    video_recording: &Mutex<VideoRecordingRuntime>,
) -> Result<VideoRecordingStatus, String> {
    let mut runtime = video_recording
        .lock()
        .map_err(|_| "Video recording state lock was poisoned".to_string())?;
    let Some(worker) = runtime.worker.as_mut() else {
        return runtime.status_snapshot();
    };
    worker.request_stop();
    if worker.wait_for_completion(video_recording_lifecycle::STOP_ACKNOWLEDGEMENT_TIMEOUT)
        == video_recording_lifecycle::CompletionWait::TimedOut
    {
        return Err(
            "Video recording stop is still in progress; worker ownership was retained".to_string(),
        );
    }
    runtime.reap_completed_worker()?;
    runtime.status_snapshot()
}

#[cfg(test)]
mod video_recording_runtime_lifecycle_tests {
    include!("video_recording_runtime_lifecycle_tests.rs");
}
