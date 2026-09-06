#![cfg(windows)]

use super::*;
use std::{
    fs,
    io::Write,
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
    "video_recording::encoder::inherited_stdin_tests::recording_input_child_process";

struct ChildFiles {
    ready: PathBuf,
    release: PathBuf,
    done: PathBuf,
    holder: Option<OwnedHandle>,
    direct: Option<OwnedHandle>,
}

impl ChildFiles {
    fn new() -> Self {
        let stem = format!(
            "syndocal-recording-inherited-stdin-{}-{}",
            std::process::id(),
            NEXT_INHERITED_PIPE_TEST.fetch_add(1, Ordering::Relaxed)
        );
        let temp = std::env::temp_dir();
        let files = Self {
            ready: temp.join(format!("{stem}.ready")),
            release: temp.join(format!("{stem}.release")),
            done: temp.join(format!("{stem}.done")),
            holder: None,
            direct: None,
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
            .env("SYNDOCAL_RECORDING_INPUT_CHILD", mode)
            .env("SYNDOCAL_RECORDING_INPUT_READY", &self.ready)
            .env("SYNDOCAL_RECORDING_INPUT_RELEASE", &self.release)
            .env("SYNDOCAL_RECORDING_INPUT_DONE", &self.done)
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .stdout(Stdio::null());
        command
    }

    fn release(&self) {
        let _ = fs::write(&self.release, b"release");
    }

    fn acquire_holder(&mut self) {
        let ids = fs::read_to_string(&self.ready).unwrap();
        let mut ids = ids.split_whitespace().map(|id| id.parse::<u32>().unwrap());
        let pid = ids.next().unwrap();
        let handle = unsafe { OpenProcess(PROCESS_SYNCHRONIZE | PROCESS_TERMINATE, false, pid) }
            .expect("retain exact holder process handle");
        self.holder = Some(unsafe { OwnedHandle::from_raw_handle(handle.0) });
        let direct = unsafe {
            OpenProcess(
                PROCESS_SYNCHRONIZE | PROCESS_TERMINATE,
                false,
                ids.next().unwrap(),
            )
        }
        .expect("retain direct process handle before exit");
        self.direct = Some(unsafe { OwnedHandle::from_raw_handle(direct.0) });
        fs::write(self.ready.with_extension("exit"), b"exit").unwrap();
        assert_eq!(
            unsafe { WaitForSingleObject(direct, 5000) },
            WAIT_OBJECT_0,
            "direct encoder must exit before the blocked write and Stop"
        );
    }

    fn holder_alive(&self) -> bool {
        let handle = HANDLE(self.holder.as_ref().unwrap().as_raw_handle());
        unsafe { WaitForSingleObject(handle, 0) == WAIT_TIMEOUT }
    }

    fn release_and_wait(&self) -> bool {
        self.release();
        let _ = fs::write(self.ready.with_extension("exit"), b"exit");
        if let Some(direct) = &self.direct {
            let handle = HANDLE(direct.as_raw_handle());
            if unsafe { WaitForSingleObject(handle, 5000) } != WAIT_OBJECT_0 {
                let _ = unsafe { TerminateProcess(handle, 1) };
                if unsafe { WaitForSingleObject(handle, 5000) } != WAIT_OBJECT_0 {
                    return false;
                }
            }
        }
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
            let _ = fs::remove_file(self.ready.with_extension("exit"));
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
#[ignore = "subprocess helper; selected only by the inherited stdin reader test"]
fn recording_input_child_process() {
    let mode = std::env::var("SYNDOCAL_RECORDING_INPUT_CHILD").expect("input child mode");
    let ready = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_INPUT_READY").expect("input child ready path"),
    );
    let release = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_INPUT_RELEASE").expect("input child release path"),
    );
    let done = PathBuf::from(
        std::env::var_os("SYNDOCAL_RECORDING_INPUT_DONE").expect("input child done path"),
    );

    if mode == "stdin_holder" {
        let deadline = Instant::now() + Duration::from_secs(30);
        while !release.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        fs::write(done, b"holder exited").expect("holder completion marker");
        return;
    }

    assert_eq!(mode, "inherited_stdin");
    let mut holder = Command::new(std::env::current_exe().expect("test executable path"));
    holder
        .args(["--exact", CHILD_TEST_NAME, "--ignored", "--nocapture"])
        .env("SYNDOCAL_RECORDING_INPUT_CHILD", "stdin_holder")
        .env("SYNDOCAL_RECORDING_INPUT_READY", &ready)
        .env("SYNDOCAL_RECORDING_INPUT_RELEASE", &release)
        .env("SYNDOCAL_RECORDING_INPUT_DONE", &done)
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        // The holder inherits the pipe reader and deliberately never reads.
        // The direct child exits without closing this inherited copy.
        .stdin(Stdio::inherit());
    let holder = holder.spawn().expect("stdin holder process");
    let pending = ready.with_extension("pending");
    fs::write(&pending, format!("{} {}", holder.id(), std::process::id()))
        .expect("owned process PID marker");
    fs::rename(pending, &ready).expect("publish complete direct child ready marker");
    let deadline = Instant::now() + Duration::from_secs(30);
    while !ready.with_extension("exit").exists() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn recording_encoder_stop_cancels_inherited_stdin_write() {
    let mut files = ChildFiles::new();
    let mut command = files.command("inherited_stdin");
    let stop = Arc::new(AtomicBool::new(false));
    let mut encoder = RecordingEncoder::spawn_with_grace(
        &mut command,
        Arc::clone(&stop),
        Duration::from_millis(150),
    )
    .expect("encoder process");
    let mut stdin = encoder.take_stdin().unwrap();
    assert!(
        wait_for_file(&files.ready, Duration::from_secs(10)),
        "direct child must spawn the stdin holder"
    );
    files.acquire_holder();
    assert!(files.holder_alive(), "holder must retain stdin before Stop");

    let (finished, finished_rx) = mpsc::channel();
    let (entered, entered_rx) = mpsc::channel();
    let finish_thread = thread::spawn(move || {
        entered.send(stdin.write_all(b"frame prefix")).unwrap();
        let written = stdin.write_all(&vec![0; 4 * 1024 * 1024]);
        drop(stdin);
        // Even a caller that ignores the write error cannot publish success.
        finished
            .send((written, encoder.finish()))
            .expect("finish result receiver");
    });
    let prefix_written = entered_rx.recv_timeout(Duration::from_secs(3));
    thread::sleep(Duration::from_millis(100));
    let was_blocked = !finish_thread.is_finished();
    stop.store(true, Ordering::Release);

    let outcome = match finished_rx.recv_timeout(Duration::from_secs(3)) {
        Ok(outcome) => outcome,
        Err(error) => {
            // Always release the holder before asserting the cancellation
            // bound, so a failed implementation cannot leave a test process.
            let reaped = files.release_and_wait();
            let _ = finished_rx.recv_timeout(Duration::from_secs(5));
            let _ = finish_thread.join();
            assert!(reaped, "stdin holder process must terminate");
            panic!("inherited stdin write did not stop: {error}");
        }
    };
    let still_alive = files.holder_alive();
    let reaped = files.release_and_wait();
    finish_thread.join().expect("finish thread");
    assert!(
        still_alive,
        "write must finish while the inherited reader is still held"
    );
    assert!(reaped, "stdin holder process must terminate");
    assert!(was_blocked, "test must exercise a blocked write");
    assert!(
        matches!(prefix_written, Ok(Ok(()))),
        "pipe must accept the frame prefix"
    );
    assert!(outcome.0.is_err(), "partial frame must fail");
    assert!(
        outcome
            .1
            .as_ref()
            .is_err_and(|error| error.contains("cancelled before complete frame EOF")),
        "cancelled input must not publish a successful encoder result"
    );
}

#[test]
fn recording_encoder_drop_cancels_inherited_stdin_write() {
    let mut files = ChildFiles::new();
    let mut encoder = RecordingEncoder::spawn(
        &mut files.command("inherited_stdin"),
        Arc::new(AtomicBool::new(false)),
    )
    .unwrap();
    let mut stdin = encoder.take_stdin().unwrap();
    assert!(wait_for_file(&files.ready, Duration::from_secs(10)));
    files.acquire_holder();
    let (entered, entered_rx) = mpsc::channel();
    let writer = thread::spawn(move || {
        entered.send(stdin.write_all(b"frame prefix")).unwrap();
        stdin.write_all(&vec![0; 4 * 1024 * 1024])
    });
    let prefix_written = entered_rx.recv_timeout(Duration::from_secs(3));
    thread::sleep(Duration::from_millis(100));
    let was_blocked = !writer.is_finished();
    let (finished, finished_rx) = mpsc::channel();
    let teardown = thread::spawn(move || {
        drop(encoder);
        finished.send(()).unwrap();
    });
    let completed = finished_rx.recv_timeout(Duration::from_secs(3)).is_ok();
    let still_alive = files.holder_alive();
    let reaped = files.release_and_wait();
    teardown.join().unwrap();
    let written = writer.join().unwrap();
    assert!(
        matches!(prefix_written, Ok(Ok(()))),
        "pipe must accept the frame prefix"
    );
    assert!(was_blocked, "test must exercise a blocked write");
    assert!(
        completed,
        "Drop must cancel the outstanding write without Stop"
    );
    assert!(
        still_alive,
        "Drop must finish before the inherited reader exits"
    );
    assert!(reaped, "owned helper must terminate");
    assert!(written.is_err(), "cancelled partial frame must fail");
}
