//! LGPL-only recording executable admission and bounded real encoder probes.
use super::encoder::process::ReapingChild;
use protocol::hidden_console_command::hidden_command;
use sha2::{Digest, Sha256};
use std::{
    env,
    ffi::OsStr,
    fs::{self, File},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        LazyLock, Mutex,
    },
    time::{Duration, Instant},
};

pub(super) const CHAIN: [&str; 5] = [
    "h264_nvenc",
    "h264_amf",
    "h264_qsv",
    "h264_mf",
    "libopenh264",
];
const PROBE_TIMEOUT: Duration = Duration::from_secs(8);
static CACHE: LazyLock<Mutex<Vec<(Identity, &'static str)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static TEMP_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, PartialEq, Eq)]
struct Identity {
    path: PathBuf,
    sha256: Vec<u8>,
}

fn identity(path: &Path) -> Result<Identity, String> {
    let path = path.canonicalize().map_err(|e| {
        format!(
            "recording.encoderExecutableInvalid: {}: {e}",
            path.display()
        )
    })?;
    let mut file = File::open(&path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 65536];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(Identity {
        path,
        sha256: hash.finalize().to_vec(),
    })
}

fn resolve_from(
    bundled: &Path,
    override_path: Option<&OsStr>,
    paths: Option<&OsStr>,
) -> Result<PathBuf, String> {
    if bundled.exists() {
        return Ok(bundled.to_path_buf());
    }
    if let Some(path) = override_path {
        return resolve_named(path, paths);
    }
    resolve_named(
        OsStr::new(if cfg!(windows) {
            "ffmpeg.exe"
        } else {
            "ffmpeg"
        }),
        paths,
    )
}

fn resolve_named(name: &OsStr, paths: Option<&OsStr>) -> Result<PathBuf, String> {
    let path = Path::new(name);
    if path.is_absolute() || path.components().count() > 1 {
        return Ok(path.to_path_buf());
    }
    for directory in env::split_paths(paths.unwrap_or_default()) {
        let candidate = directory.join(path);
        if candidate.is_file() {
            return Ok(candidate);
        }
        #[cfg(windows)]
        if path.extension().is_none() {
            let candidate = candidate.with_extension("exe");
            if candidate.is_file() {
                return Ok(candidate);
            }
        }
    }
    Err("recording.encoderExecutableInvalid: Install the bundled LGPL FFmpeg runtime or set SYNDOCAL_FFMPEG to an LGPL ffmpeg.exe".into())
}

const EXECUTABLE_ERROR: &str = "Recording unavailable: Restore the bundled LGPL FFmpeg or set SYNDOCAL_FFMPEG to a valid LGPL executable.";
const LICENSE_ERROR: &str = "Recording unavailable: FFmpeg license verification failed. Restore the bundled LGPL FFmpeg; GPL and nonfree builds cannot record.";
const ENCODER_ERROR: &str = "Recording unavailable: No supported H.264 encoder works. Update the GPU driver or restore the bundled LGPL FFmpeg with OpenH264.";

fn actionable_error(detail: String) -> String {
    eprintln!("Syndocal recording admission: {detail}");
    if detail.starts_with("recording.encoderLicenseRejected:") {
        LICENSE_ERROR.into()
    } else if detail.starts_with("recording.encoderUnavailable:") {
        ENCODER_ERROR.into()
    } else {
        EXECUTABLE_ERROR.into()
    }
}

pub(super) fn selected(stop: &AtomicBool) -> Result<(PathBuf, &'static str), String> {
    selected_inner(stop).map_err(actionable_error)
}

fn selected_inner(stop: &AtomicBool) -> Result<(PathBuf, &'static str), String> {
    let exe = env::current_exe().map_err(|e| e.to_string())?;
    let bundled = exe.with_file_name(if cfg!(windows) {
        "ffmpeg.exe"
    } else {
        "ffmpeg"
    });
    let path = resolve_from(
        &bundled,
        env::var_os("SYNDOCAL_FFMPEG").as_deref(),
        env::var_os("PATH").as_deref(),
    )?;
    select_inner(&path, Some(stop))
}

#[cfg(test)]
pub(crate) fn select(path: &Path) -> Result<(PathBuf, &'static str), String> {
    select_inner(path, None).map_err(actionable_error)
}

fn select_inner(path: &Path, stop: Option<&AtomicBool>) -> Result<(PathBuf, &'static str), String> {
    if stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
        return Err("recording.encoderUnavailable: Recording preparation cancelled".into());
    }
    let path = resolve_named(path.as_os_str(), env::var_os("PATH").as_deref())?;
    let id = identity(&path)?;
    let mut cache = CACHE
        .lock()
        .map_err(|_| "recording.encoderUnavailable: encoder cache unavailable".to_string())?;
    if let Some((_, encoder)) = cache.iter().find(|(cached, _)| cached == &id) {
        return Ok((id.path, *encoder));
    }
    validate_executable_inner(&id.path, stop)?;
    let mut failures = Vec::new();
    for encoder in CHAIN {
        if stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
            return Err("recording.encoderUnavailable: Recording preparation cancelled".into());
        }
        match probe_inner(&id.path, encoder, stop) {
            Ok(()) => {
                eprintln!(
                    "Syndocal recording encoder: {encoder} ({})",
                    id.path.display()
                );
                if identity(&id.path)? != id {
                    return Err("recording.encoderExecutableInvalid: FFmpeg changed during admission; restore the bundled runtime".into());
                }
                cache.push((id.clone(), encoder));
                return Ok((id.path, encoder));
            }
            Err(error) => failures.push(format!("{encoder}: {error}")),
        }
    }
    Err(format!("recording.encoderUnavailable: No supported H.264 encoder works. Install/update the GPU driver or restore the bundled LGPL FFmpeg with OpenH264. {}", failures.join("; ")))
}

fn validate_license(license: &str, version: &str) -> Result<(), String> {
    let combined = format!("{license}\n{version}").to_ascii_lowercase();
    if combined.contains("--enable-gpl")
        || combined.contains("--enable-nonfree")
        || combined.contains("nonfree and unredistributable")
        || license
            .to_ascii_lowercase()
            .contains("gnu general public license")
        || !license
            .to_ascii_lowercase()
            .contains("gnu lesser general public license")
        || !version.to_ascii_lowercase().contains("ffmpeg version")
    {
        return Err("recording.encoderLicenseRejected: Restore the bundled LGPL FFmpeg; GPL, nonfree, or unverifiable builds cannot record".into());
    }
    Ok(())
}

#[cfg(test)]
pub(crate) fn validate_executable(path: &Path) -> Result<(), String> {
    validate_executable_inner(path, None).map_err(actionable_error)
}

fn validate_executable_inner(path: &Path, stop: Option<&AtomicBool>) -> Result<(), String> {
    let license = capture(path, "-L", stop)?;
    let version = capture(path, "-version", stop)?;
    validate_license(&license, &version)
}

// Disk-backed capture avoids pipe-capacity deadlocks and detached reader threads.
fn capture(path: &Path, argument: &str, stop: Option<&AtomicBool>) -> Result<String, String> {
    let path_tmp = env::temp_dir().join(format!(
        "syndocal-ffmpeg-license-{}-{}",
        std::process::id(),
        TEMP_ID.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut output = File::options()
            .read(true)
            .write(true)
            .create_new(true)
            .open(&path_tmp)
            .map_err(|e| e.to_string())?;
        let mut command = hidden_command(path);
        command
            .arg(argument)
            .stdin(Stdio::null())
            .stdout(output.try_clone().map_err(|e| e.to_string())?)
            .stderr(Stdio::null());
        bounded_inner(&mut command, PROBE_TIMEOUT, stop)?;
        if output.metadata().map_err(|e| e.to_string())?.len() > 262144 {
            return Err("FFmpeg license response exceeded limit".into());
        }
        output.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        let mut text = String::new();
        output
            .read_to_string(&mut text)
            .map_err(|e| e.to_string())?;
        Ok(text)
    })();
    let _ = fs::remove_file(path_tmp);
    result.map_err(|e: String| {
        format!("recording.encoderLicenseRejected: Cannot verify LGPL FFmpeg: {e}")
    })
}

#[cfg(test)]
pub(super) fn bounded(command: &mut Command, timeout: Duration) -> Result<(), String> {
    bounded_inner(command, timeout, None)
}

fn bounded_inner(
    command: &mut Command,
    timeout: Duration,
    stop: Option<&AtomicBool>,
) -> Result<(), String> {
    if stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
        return Err("recording preparation cancelled".into());
    }
    let mut child = ReapingChild::new(command.spawn().map_err(|e| e.to_string())?)?;
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(status) = child.poll().map_err(|e| e.to_string())? {
            return if status.success() {
                Ok(())
            } else {
                Err(format!("exit {status}"))
            };
        }
        if Instant::now() >= deadline || stop.is_some_and(|stop| stop.load(Ordering::Acquire)) {
            child
                .terminate()
                .map_err(|e| format!("probe timeout; termination failed: {e}"))?;
            // ReapingChild retains process ownership even if teardown fails.
            let reap_deadline = Instant::now() + Duration::from_secs(2);
            while child.poll().map_err(|e| e.to_string())?.is_none() {
                if Instant::now() >= reap_deadline {
                    return Err(
                        "probe timeout; termination requested, process retained by owned reaper"
                            .into(),
                    );
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            return Err("probe timed out and was terminated/reaped".into());
        }
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[cfg(test)]
pub(crate) fn probe(path: &Path, encoder: &str) -> Result<(), String> {
    probe_inner(path, encoder, None)
}

fn probe_inner(path: &Path, encoder: &str, stop: Option<&AtomicBool>) -> Result<(), String> {
    let mut command = hidden_command(path);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-nostdin",
        "-f",
        "lavfi",
        "-i",
        "color=c=black:s=256x144:r=30",
        "-frames:v",
        "2",
    ]);
    options(&mut command, encoder, 30)?;
    command
        .args(["-f", "null", "-"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    bounded_inner(&mut command, PROBE_TIMEOUT, stop)
}

/// Quality intent: fast hardware presets, quality-based RC where supported,
/// 2-second GOP, yuv420p. OpenH264 uses 8 Mbps VBR with frame skipping disabled.
pub(super) fn options(command: &mut Command, encoder: &str, rate: u32) -> Result<(), String> {
    command.args([
        "-c:v",
        encoder,
        "-pix_fmt",
        "yuv420p",
        "-g",
        &rate.saturating_mul(2).to_string(),
    ]);
    match encoder {
        "h264_nvenc" => {
            command.args(["-preset", "p2", "-rc", "vbr", "-cq", "23", "-b:v", "0"]);
        }
        "h264_amf" => {
            command.args([
                "-quality", "speed", "-rc", "cqp", "-qp_i", "23", "-qp_p", "23",
            ]);
        }
        "h264_qsv" => {
            command.args(["-preset", "veryfast", "-global_quality", "23"]);
        }
        "h264_mf" => {
            command.args([
                "-rate_control",
                "quality",
                "-quality",
                "75",
                "-scenario",
                "archive",
            ]);
        }
        "libopenh264" => {
            command.args([
                "-rc_mode",
                "bitrate",
                "-b:v",
                "8M",
                "-allow_skip_frames",
                "0",
            ]);
        }
        _ => return Err("recording.encoderUnavailable: unsupported recording encoder".into()),
    }
    Ok(())
}

#[cfg(test)]
#[path = "video_recording_selection_tests.rs"]
mod tests;
