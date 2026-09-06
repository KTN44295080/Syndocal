use super::*;
use std::{fs, io::Write, path::PathBuf, process::Stdio, sync::atomic::AtomicU64, time::Instant};

static NEXT_CHILD: AtomicU64 = AtomicU64::new(0);

struct ReadyFile(PathBuf);
impl Drop for ReadyFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}

fn child(mode: &str) -> (Command, ReadyFile) {
    let ready = ReadyFile(std::env::temp_dir().join(format!(
        "syndocal-encoder-test-{}-{}.ready",
        std::process::id(),
        NEXT_CHILD.fetch_add(1, Ordering::Relaxed)
    )));
    assert!(!ready.0.exists());
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "video_recording::encoder::tests::recording_encoder_child_process",
            "--ignored",
            "--nocapture",
        ])
        .env("SYNDOCAL_RECORDING_TEST_CHILD", mode)
        .env("SYNDOCAL_RECORDING_TEST_READY", &ready.0)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped())
        .stdout(Stdio::null());
    (command, ready)
}

fn await_ready(ready: &ReadyFile) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !ready.0.exists() {
        assert!(
            Instant::now() < deadline,
            "controlled encoder failed to become ready"
        );
        thread::sleep(Duration::from_millis(10));
    }
}

#[test]
#[ignore = "subprocess helper; selected only by recording encoder tests"]
fn recording_encoder_child_process() {
    let mode = std::env::var("SYNDOCAL_RECORDING_TEST_CHILD").expect("child test mode");
    let ready = std::env::var_os("SYNDOCAL_RECORDING_TEST_READY").expect("child readiness file");
    if mode == "eof_hang" || mode == "success" {
        io::copy(&mut io::stdin().lock(), &mut io::sink()).unwrap();
    }
    fs::write(ready, b"ready").unwrap();
    if mode == "success" {
        eprintln!("graceful diagnostic tail");
        return;
    }
    if mode == "failure" {
        eprintln!("controlled encoder failure");
        std::process::exit(7);
    }
    assert!(mode == "blocked_input" || mode == "eof_hang");
    thread::sleep(Duration::from_secs(30));
}

#[test]
fn recording_encoder_stop_reaps_child_that_ignores_eof() {
    let (mut command, ready) = child("eof_hang");
    let stop = Arc::new(AtomicBool::new(false));
    let mut encoder = RecordingEncoder::spawn_with_grace(
        &mut command,
        Arc::clone(&stop),
        Duration::from_millis(150),
    )
    .unwrap();
    drop(encoder.take_stdin());
    await_ready(&ready);
    let started = Instant::now();
    stop.store(true, Ordering::Release);
    let error = encoder.finish().unwrap_err();
    assert!(error.contains("termination was required"), "{error}");
    assert!(started.elapsed() < Duration::from_secs(5));
}

#[test]
fn recording_encoder_stop_unblocks_direct_child_stdin_write() {
    let (mut command, ready) = child("blocked_input");
    let stop = Arc::new(AtomicBool::new(false));
    let mut encoder = RecordingEncoder::spawn_with_grace(
        &mut command,
        Arc::clone(&stop),
        Duration::from_millis(150),
    )
    .unwrap();
    await_ready(&ready);
    let mut stdin = encoder.take_stdin().unwrap();
    let writer = thread::spawn(move || stdin.write_all(&vec![0; 4 * 1024 * 1024]));
    thread::sleep(Duration::from_millis(50));
    let was_blocked = !writer.is_finished();
    stop.store(true, Ordering::Release);
    let outcome = encoder.finish();
    let written = writer.join().unwrap();
    let error = outcome.unwrap_err();
    assert!(was_blocked, "test must exercise an outstanding write");
    assert!(
        written.is_err(),
        "forced child termination must break its input pipe"
    );
    assert!(error.contains("termination was required"), "{error}");
}

#[test]
fn recording_encoder_eof_finishes_without_forcing_and_keeps_diagnostics() {
    let (mut command, _ready) = child("success");
    let mut encoder =
        RecordingEncoder::spawn(&mut command, Arc::new(AtomicBool::new(false))).unwrap();
    let mut stdin = encoder.take_stdin().unwrap();
    stdin.write_all(b"one complete test frame").unwrap();
    drop(stdin);
    let tail = encoder.finish().unwrap();
    assert!(String::from_utf8_lossy(&tail).contains("graceful diagnostic tail"));
}

#[test]
fn recording_encoder_fail_preserves_primary_and_rejects_success_exit() {
    let (mut command, ready) = child("success");
    let mut encoder =
        RecordingEncoder::spawn(&mut command, Arc::new(AtomicBool::new(false))).unwrap();
    drop(encoder.take_stdin());
    await_ready(&ready);
    let error = encoder.fail("incomplete frame".into()).unwrap_err();
    assert!(error.starts_with("incomplete frame"), "{error}");
}

#[test]
fn recording_encoder_exit_failure_preserves_diagnostic_tail() {
    let (mut command, ready) = child("failure");
    let mut encoder =
        RecordingEncoder::spawn(&mut command, Arc::new(AtomicBool::new(false))).unwrap();
    drop(encoder.take_stdin());
    await_ready(&ready);
    let error = encoder.finish().unwrap_err();
    assert!(error.contains("controlled encoder failure"), "{error}");
}

#[test]
fn recording_encoder_drop_reaps_the_direct_child() {
    let (mut command, ready) = child("blocked_input");
    let encoder = RecordingEncoder::spawn(&mut command, Arc::new(AtomicBool::new(false))).unwrap();
    await_ready(&ready);
    let started = Instant::now();
    drop(encoder);
    assert!(started.elapsed() < Duration::from_secs(5));
}
