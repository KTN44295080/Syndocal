#![cfg(windows)]

use super::*;
use std::{
    fs,
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc, Arc,
    },
    thread,
    time::{Duration, Instant},
};
use windows::Win32::{
    Foundation::{HANDLE, WAIT_OBJECT_0, WAIT_TIMEOUT},
    System::Threading::{
        OpenProcess, TerminateProcess, WaitForSingleObject, PROCESS_SYNCHRONIZE, PROCESS_TERMINATE,
    },
};

static NEXT_INHERITED_PIPE_TEST: AtomicU64 = AtomicU64::new(0);

const CHILD_TEST_NAME: &str =
    "video_recording::encoder::inherited_stderr_tests::recording_diagnostics_child_process";

struct ChildFiles {
    ready: PathBuf,
    release: PathBuf,
    done: PathBuf,
    holder: Option<OwnedHandle>,
}

impl ChildFiles {
    fn new() -> Self {
        let stem = format!(
            "syndocal-recording-inherited-stderr-{}-{}",
            std::process::id(),
            NEXT_INHERITED_PIPE_TEST.fetch_add(1, Ordering::Relaxed)
        );
        let temp = std::env::temp_dir();
        let files = Self {
            ready: temp.join(format!("{stem}.ready")),
            release: temp.join(format!("{stem}.release")),
            done: temp.join(format!("{stem}.done")),
            holder: None,
        };
        for path in [&files.ready, &files.release, &files.done] {
            let _ = fs::remove_file(path);
        }
        files
    }

    fn command(&self, mode: &str) -> Command {
        let mut command = Command::new(std::env::current_exe().expect("test executable path"));
        command
            .args(["--exact", CHILD_TEST_NAME, "--ignored", "--nocapture"])
            .env("SYNDOCAL_RECORDING_DIAGNOSTICS_CHILD", mode)
            .env("SYNDOCAL_RECORDING_DIAGNOSTICS_READY", &self.ready)
            .env("SYNDOCAL_RECORDING_DIAGNOSTICS_RELEASE", &self.release)
            .env("SYNDOCAL_RECORDING_DIAGNOSTICS_DONE", &self.done)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::null());
        command
    }

    fn release(&self) {
        let _ = fs::write(&self.release, b"release");
    }

    fn acquire_holder(&mut self) {
        let pid: u32 = fs::read_to_string(&self.ready).unwrap().parse().unwrap();
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) }
            .expect("retain exact holder process handle");
        self.holder = Some(unsafe { OwnedHandle::from_raw_handle(handle.0) });
    }

    fn holder_alive(&self) -> bool {
        let handle = HANDLE(self.holder.as_ref().unwrap().as_raw_handle());
        unsafe { WaitForSingleObject(handle, 0) == WAIT_TIMEOUT }
    }

    fn release_and_wait(&self) -> bool {
        self.release();
        let Some(holder) = &self.holder else {
            return false;
        };
        let handle = HANDLE(holder.as_raw_handle());
        if unsafe { WaitForSingleObject(handle, 5000) } == WAIT_OBJECT_0 {
            return true;
        }
        // Cleanup must also work if the helper fails to read its marker.
        let _ = unsafe { TerminateProcess(handle, 1) };
        unsafe { WaitForSingleObject(handle, 5000) == WAIT_OBJECT_0 }
    }
}

impl Drop for ChildFiles {
    fn drop(&mut self) {
        if self.release_and_wait() {
            let _ = fs::remove_file(&self.ready);
            let _ = fs::remove_file(&self.release);
            let _ = fs::remove_file(&self.done);
        }
    }
}

fn wait_for_file(path: &PathBuf, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while !path.exists() {
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(10));
    }
    true
}

#[test]
#[ignore = "subprocess helper; selected only by the inherited stderr reader test"]
fn recording_diagnostics_child_process() {
    let mode =
        std::env::var("SYNDOCAL_RECORDING_DIAGNOSTICS_CHILD").expect("diagnostics child mode");
    let ready = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_DIAGNOSTICS_READY")
            .expect("diagnostics child ready path"),
    );
    let release = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_DIAGNOSTICS_RELEASE")
            .expect("diagnostics child release path"),
    );
    let done = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_DIAGNOSTICS_DONE")
            .expect("diagnostics child done path"),
    );

    if mode == "stderr_holder" {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !release.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        fs::write(done, b"holder exited").expect("holder completion marker");
        return;
    }

    assert_eq!(mode, "inherited_stderr");
    let mut holder = Command::new(std::env::current_exe().expect("test executable path"));
    holder
        .args(["--exact", CHILD_TEST_NAME, "--ignored", "--nocapture"])
        .env("SYNDOCAL_RECORDING_DIAGNOSTICS_CHILD", "stderr_holder")
        .env("SYNDOCAL_RECORDING_DIAGNOSTICS_READY", &ready)
        .env("SYNDOCAL_RECORDING_DIAGNOSTICS_RELEASE", &release)
        .env("SYNDOCAL_RECORDING_DIAGNOSTICS_DONE", &done)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        // This is the pipe writer inherited by the holder. The direct child
        // exits below, leaving only the controlled holder to delay EOF.
        .stderr(Stdio::inherit());
    let holder = holder.spawn().expect("stderr holder process");
    let pending = ready.with_extension("pending");
    fs::write(&pending, holder.id().to_string()).expect("direct child PID marker");
    fs::rename(pending, ready).expect("publish complete direct child ready marker");
}

#[test]
fn recording_encoder_stop_cancels_inherited_stderr_reader() {
    let mut files = ChildFiles::new();
    let mut command = files.command("inherited_stderr");
    let stop = Arc::new(AtomicBool::new(false));
    let mut encoder = RecordingEncoder::spawn_with_grace(
        &mut command,
        Arc::clone(&stop),
        Duration::from_millis(150),
    )
    .expect("encoder process");
    drop(encoder.take_stdin());
    assert!(
        wait_for_file(&files.ready, Duration::from_secs(10)),
        "direct child must spawn the stderr holder"
    );
    files.acquire_holder();
    assert!(
        files.holder_alive(),
        "holder must retain stderr before Stop"
    );

    stop.store(true, Ordering::Release);
    let (finished, finished_rx) = mpsc::channel();
    let finish_thread = thread::spawn(move || {
        finished
            .send(encoder.finish())
            .expect("finish result receiver");
    });

    let outcome = match finished_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(outcome) => outcome,
        Err(error) => {
            // Always release the holder before asserting the cancellation
            // bound, so a failed implementation cannot leave a test process.
            let reaped = files.release_and_wait();
            let _ = finished_rx.recv_timeout(Duration::from_secs(5));
            let _ = finish_thread.join();
            assert!(reaped, "stderr holder process must terminate");
            panic!("inherited stderr reader did not stop: {error}");
        }
    };
    let still_alive = files.holder_alive();
    let reaped = files.release_and_wait();
    finish_thread.join().expect("finish thread");
    assert!(
        still_alive,
        "reader must finish while the writer is still held"
    );
    assert!(reaped, "stderr holder process must terminate");
    assert!(
        outcome
            .as_ref()
            .is_err_and(|error| error.contains("cancelled before EOF")),
        "cancelled diagnostics must not publish a successful encoder result"
    );
}
