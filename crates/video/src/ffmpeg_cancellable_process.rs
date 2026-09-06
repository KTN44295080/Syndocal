use super::VideoRenderCancellation;
use std::{
    io::Read,
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

pub(crate) struct CancellableProcessOutput {
    pub(crate) status: ExitStatus,
    pub(crate) stdout: Vec<u8>,
    pub(crate) stderr: Vec<u8>,
}

pub(crate) enum CancellableProcessError {
    Cancelled,
    Failed(String),
}

pub(crate) fn run_cancellable_process(
    mut command: Command,
    cancellation: &dyn VideoRenderCancellation,
) -> Result<CancellableProcessOutput, CancellableProcessError> {
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| CancellableProcessError::Failed(error.to_string()))?;

    let mut stdout = child
        .stdout
        .take()
        .expect("cancellable process stdout was configured as piped");
    let mut stderr = child
        .stderr
        .take()
        .expect("cancellable process stderr was configured as piped");
    let stdout_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stdout.read_to_end(&mut bytes).map(|_| bytes)
    });
    let stderr_reader = thread::spawn(move || {
        let mut bytes = Vec::new();
        stderr.read_to_end(&mut bytes).map(|_| bytes)
    });

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if cancellation.is_cancelled() => {
                terminate_and_join(&mut child, stdout_reader, stderr_reader);
                return Err(CancellableProcessError::Cancelled);
            }
            Ok(None) => thread::sleep(Duration::from_millis(5)),
            Err(error) => {
                terminate_and_join(&mut child, stdout_reader, stderr_reader);
                return Err(CancellableProcessError::Failed(format!(
                    "failed to poll child process: {error}"
                )));
            }
        }
    };

    let stdout = stdout_reader
        .join()
        .map_err(|_| CancellableProcessError::Failed("stdout reader panicked".to_string()))?
        .map_err(|error| {
            CancellableProcessError::Failed(format!("failed to read stdout: {error}"))
        })?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| CancellableProcessError::Failed("stderr reader panicked".to_string()))?
        .map_err(|error| {
            CancellableProcessError::Failed(format!("failed to read stderr: {error}"))
        })?;

    Ok(CancellableProcessOutput {
        status,
        stdout,
        stderr,
    })
}

fn terminate_and_join(
    child: &mut Child,
    stdout_reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
    stderr_reader: thread::JoinHandle<std::io::Result<Vec<u8>>>,
) {
    let _ = child.kill();
    let _ = child.wait();
    let _ = stdout_reader.join();
    let _ = stderr_reader.join();
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        time::{Duration, Instant},
    };

    #[test]
    fn cancellation_terminates_and_reaps_a_running_process() {
        let command = if cfg!(windows) {
            let mut command = Command::new("powershell.exe");
            command.args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Start-Sleep -Seconds 30",
            ]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", "sleep 30"]);
            command
        };
        let started = Instant::now();
        let cancellation = || started.elapsed() >= Duration::from_millis(50);

        let result = run_cancellable_process(command, &cancellation);

        assert!(matches!(result, Err(CancellableProcessError::Cancelled)));
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "cancelled child must be reaped promptly"
        );
    }
}
