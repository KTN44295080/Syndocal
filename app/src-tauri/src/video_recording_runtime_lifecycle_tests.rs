use super::*;
use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
    time::Duration,
};

struct ReleaseWorkerOnDrop(Option<mpsc::Sender<()>>);

impl Drop for ReleaseWorkerOnDrop {
    fn drop(&mut self) {
        if let Some(release) = self.0.take() {
            let _ = release.send(());
        }
    }
}

#[test]
fn start_admission_rejects_inactive_status_with_live_worker_then_allows_late_replacement() {
    let stop = Arc::new(AtomicBool::new(false));
    let (ready, ready_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        ready.send(()).expect("recording worker readiness");
        release_rx.recv().expect("recording worker release");
    });
    ready_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("recording worker must start");

    let mut runtime = VideoRecordingRuntime::default();
    assert!(!runtime.status.lock().expect("recording status").active);
    runtime.worker = Some(video_recording_lifecycle::RecordingWorkerLifecycle::new(
        stop, worker,
    ));
    // Declared after runtime so assertion unwinding releases the worker before
    // runtime's deliberately blocking Drop joins it.
    let mut release_on_drop = ReleaseWorkerOnDrop(Some(release));

    let error = runtime
        .ensure_recording_worker_available()
        .expect_err("live worker ownership must gate Start even when status is inactive");
    assert_eq!(
        error,
        "A previous video output recording worker is still finishing"
    );
    assert!(runtime.worker.is_some());

    release_on_drop
        .0
        .take()
        .expect("recording release sender")
        .send(())
        .expect("recording worker must receive release");
    assert_eq!(
        runtime
            .worker
            .as_ref()
            .expect("late worker ownership")
            .wait_for_completion(Duration::from_secs(1)),
        video_recording_lifecycle::CompletionWait::Completed
    );
    runtime
        .ensure_recording_worker_available()
        .expect("completed worker can be reaped before replacement");
    assert!(runtime.worker.is_none());
}

#[test]
fn start_admission_reports_worker_panic_after_reaping_it() {
    let stop = Arc::new(AtomicBool::new(false));
    let worker = thread::spawn(|| panic!("recording worker panic"));
    let mut runtime = VideoRecordingRuntime::default();
    runtime.worker = Some(video_recording_lifecycle::RecordingWorkerLifecycle::new(
        stop, worker,
    ));
    assert_eq!(
        runtime
            .worker
            .as_ref()
            .expect("panicking worker ownership")
            .wait_for_completion(Duration::from_secs(1)),
        video_recording_lifecycle::CompletionWait::Completed
    );

    let error = runtime
        .ensure_recording_worker_available()
        .expect_err("worker panic must remain visible to Start");
    assert_eq!(error, "Video recording worker panicked");
    assert!(runtime.worker.is_none());
    let status = runtime.status.lock().expect("recording status");
    assert!(!status.active);
    assert_eq!(
        status.last_error.as_deref(),
        Some("Video recording worker panicked")
    );
}

#[test]
fn start_admission_rejects_poisoned_status() {
    let mut runtime = VideoRecordingRuntime::default();
    let status = Arc::clone(&runtime.status);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _status = status.lock().expect("recording status");
        panic!("poison recording status");
    }));

    let error = runtime
        .ensure_recording_worker_available()
        .expect_err("poisoned status must fail closed during Start admission");
    assert_eq!(error, "Video recording status lock was poisoned");
}

#[test]
fn recording_stop_timeout_retains_worker_then_late_stop_reaps_it() {
    let stop = Arc::new(AtomicBool::new(false));
    let mut recording = VideoRecordingRuntime::default();
    recording.status.lock().unwrap().active = true;
    let worker_status = Arc::clone(&recording.status);
    let (release, release_rx) = mpsc::channel();
    let worker = thread::spawn(move || {
        // A broken Stop implementation must fail this test, not hang the suite.
        let _ = release_rx.recv_timeout(Duration::from_secs(2));
        worker_status.lock().unwrap().active = false;
    });
    recording.worker = Some(video_recording_lifecycle::RecordingWorkerLifecycle::new(
        Arc::clone(&stop),
        worker,
    ));
    let recording = Mutex::new(recording);
    let mut release_on_drop = ReleaseWorkerOnDrop(Some(release));

    let error = stop_video_output_recording_runtime(&recording)
        .expect_err("Stop must report pending while the controlled worker is still running");
    assert!(error.contains("still in progress"));
    assert!(stop.load(Ordering::Acquire));
    {
        let mut runtime = recording.lock().unwrap();
        assert!(runtime.worker.is_some());
        assert!(runtime.status.lock().unwrap().active);
        assert_eq!(
            runtime.ensure_recording_worker_available().unwrap_err(),
            "A previous video output recording is still stopping"
        );
    }

    release_on_drop.0.take().unwrap().send(()).unwrap();
    assert_eq!(
        recording
            .lock()
            .unwrap()
            .worker
            .as_ref()
            .unwrap()
            .wait_for_completion(Duration::from_secs(1)),
        video_recording_lifecycle::CompletionWait::Completed
    );
    let status = stop_video_output_recording_runtime(&recording)
        .expect("a subsequent Stop must reap the original completed worker");
    assert!(!status.active);
    assert!(recording.lock().unwrap().worker.is_none());
}
