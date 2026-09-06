use super::*;
use std::{
    io::{self, Read},
    sync::mpsc,
    thread,
    time::Duration,
};

fn join_failed_recording_diagnostics(
    reader: thread::JoinHandle<io::Result<Vec<u8>>>,
    primary: String,
) -> String {
    let mut encoder = RecordingEncoder {
        stdin: None,
        diagnostics: Some(reader),
        supervisor: None,
        finishing: Arc::new(AtomicBool::new(false)),
        aborting: Arc::new(AtomicBool::new(false)),
    };
    encoder.join_owned(Some(primary)).unwrap_err()
}

struct BlockingDiagnosticReader {
    ready: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Read for BlockingDiagnosticReader {
    fn read(&mut self, _buffer: &mut [u8]) -> io::Result<usize> {
        self.ready
            .send(())
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "ready receiver dropped"))?;
        self.release
            .recv()
            .map_err(|_| io::Error::new(io::ErrorKind::BrokenPipe, "release sender dropped"))?;
        Ok(0)
    }
}

#[test]
fn failed_recording_diagnostics_join_waits_for_blocked_reader() {
    let (ready, ready_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel();
    let stderr_reader = thread::spawn(move || {
        recording_artifact::drain_encoder_stderr(BlockingDiagnosticReader {
            ready,
            release: release_rx,
        })
    });
    let reader_started = ready_rx.recv_timeout(Duration::from_secs(1)).is_ok();
    let (done, done_rx) = mpsc::channel();
    let joiner = thread::spawn(move || {
        let result = join_failed_recording_diagnostics(
            stderr_reader,
            "FFmpeg pipe failed: broken pipe".to_string(),
        );
        done.send(result).unwrap();
    });
    let still_blocked = done_rx.recv_timeout(Duration::from_millis(25)).is_err();
    release.send(()).unwrap();
    let result = done_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("diagnostic join must finish after the pipe reader is released");
    joiner.join().unwrap();
    assert!(reader_started, "the controlled reader must enter read");
    assert!(
        still_blocked,
        "failed recording cleanup must wait for its diagnostics reader"
    );
    assert_eq!(result, "FFmpeg pipe failed: broken pipe");
}

#[test]
fn failed_recording_diagnostics_join_preserves_primary_error_and_diagnostic_tail() {
    let reader = thread::spawn(|| Ok::<Vec<u8>, io::Error>(b"diagnostic tail".to_vec()));
    assert_eq!(
        join_failed_recording_diagnostics(reader, "FFmpeg wait failed".to_string()),
        "FFmpeg wait failed; diagnostic tail"
    );
}

#[test]
fn failed_recording_diagnostics_join_appends_reader_error_after_primary_error() {
    let reader = thread::spawn(|| {
        Err::<Vec<u8>, io::Error>(io::Error::new(
            io::ErrorKind::InvalidData,
            "diagnostic stream failed",
        ))
    });
    let error = join_failed_recording_diagnostics(reader, "FFmpeg pipe failed".to_string());
    assert_eq!(
        error,
        "FFmpeg pipe failed; Cannot finish reading FFmpeg diagnostics: diagnostic stream failed"
    );
}

#[test]
fn failed_recording_diagnostics_join_appends_reader_panic_after_primary_error() {
    let reader = thread::spawn(|| -> io::Result<Vec<u8>> {
        panic!("diagnostic reader panic");
    });
    let error = join_failed_recording_diagnostics(reader, "FFmpeg wait failed".to_string());
    assert_eq!(
        error,
        "FFmpeg wait failed; FFmpeg diagnostic reader panicked"
    );
}
