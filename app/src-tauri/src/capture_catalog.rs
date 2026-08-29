//! Windows DirectShow camera profile discovery and exact-profile probing.
//!
//! The human-readable DirectShow device name is intentionally never used as a
//! capture endpoint.  DirectShow's alternative name is the exact device
//! identity and the only name accepted by the opaque endpoint selector.

use std::{
    collections::{BTreeMap, BTreeSet},
    io::Read,
    path::PathBuf,
    process::{Command, Stdio},
    sync::mpsc,
    time::Duration,
};

#[cfg(test)]
use std::io::Write;

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::capture_process_lifecycle::{terminate_process_or_defer, ProcessTermination};

const CAMERA_ENDPOINT_PREFIX: &str = "syndocal-camera-v1:";
const FFMPEG_CAMERA_COMMAND_TIMEOUT: Duration = Duration::from_secs(7);
// A process can close both inherited pipes just before its exit status becomes
// observable.  Give that normal exit race a small, finite reap window before
// treating it as a stuck FFmpeg process.  This is deliberately much shorter
// than the command timeout and never replaces the kill/wait cleanup path.
const FFMPEG_PROCESS_EXIT_GRACE: Duration = Duration::from_millis(250);
const PROCESS_EXIT_POLL_INTERVAL: Duration = Duration::from_millis(5);
const PROCESS_TERMINATION_REAP_TIMEOUT: Duration = Duration::from_millis(250);
const MAX_FFMPEG_COMMAND_OUTPUT_BYTES: usize = 1024 * 1024;
const MAX_CAMERA_WIDTH: u32 = 4096;
const MAX_CAMERA_HEIGHT: u32 = 2160;
const MAX_CAMERA_FRAME_RATE: FrameRate = FrameRate {
    numerator: 120,
    denominator: 1,
};
const MAX_HIGH_RESOLUTION_FRAME_RATE: FrameRate = FrameRate {
    numerator: 30,
    denominator: 1,
};
const MAX_FULL_HD_FRAME_RATE: FrameRate = FrameRate {
    numerator: 60,
    denominator: 1,
};
const STANDARD_RANGE_FRAME_RATES: [FrameRate; 6] = [
    FrameRate {
        numerator: 24,
        denominator: 1,
    },
    FrameRate {
        numerator: 25,
        denominator: 1,
    },
    FrameRate {
        numerator: 30,
        denominator: 1,
    },
    FrameRate {
        numerator: 50,
        denominator: 1,
    },
    FrameRate {
        numerator: 60,
        denominator: 1,
    },
    FrameRate {
        numerator: 120,
        denominator: 1,
    },
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoCameraProfileDescriptor {
    pub device_identity: String,
    pub device_name: String,
    pub profile_identity: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub frame_rate_label: String,
    pub pixel_format: Option<String>,
    pub codec: Option<String>,
    /// Opaque, canonical camera selection.  It is the only endpoint accepted
    /// by the Windows camera transport.
    pub endpoint_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub struct VideoCameraProfileProbeResult {
    pub success: bool,
    pub endpoint_name: String,
    pub actual_width: u32,
    pub actual_height: u32,
    pub frame_rate_numerator: u32,
    pub frame_rate_denominator: u32,
    pub frame_rate_label: String,
    pub pixel_format: Option<String>,
    pub codec: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub(crate) enum CameraInputFormat {
    Vcodec(String),
    PixelFormat(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CameraEndpointPayloadV1 {
    device_alternative_name: String,
    input_format: CameraInputFormat,
    width: u32,
    height: u32,
    frame_rate_numerator: u32,
    frame_rate_denominator: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct FrameRate {
    numerator: u32,
    denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CanonicalCameraSelection {
    pub(crate) device_alternative_name: String,
    pub(crate) input_format: CameraInputFormat,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) frame_rate_numerator: u32,
    pub(crate) frame_rate_denominator: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DirectShowVideoDevice {
    display_name: String,
    alternative_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AdvertisedCameraProfile {
    input_format: CameraInputFormat,
    width: u32,
    height: u32,
    frame_rate: FrameRate,
}

/// Resolve and sanity-check the executable used by all capture subprocesses.
/// A successful `-version` response is the executable-level verification
/// available at runtime; profile discovery never routes through a shell.
pub(crate) fn verified_ffmpeg_executable() -> Result<PathBuf, String> {
    let executable = std::env::var_os("SYNDOCAL_FFMPEG")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("ffmpeg"));
    let mut command = Command::new(&executable);
    command
        .arg("-version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = bounded_process_output(
        &mut command,
        "FFmpeg version verification",
        FFMPEG_CAMERA_COMMAND_TIMEOUT,
    )
    .map_err(|error| {
        format!(
            "Configured FFmpeg executable '{}' could not be started or verified: {error}",
            executable.display()
        )
    })?;
    let version_output = combined_process_output(&output.stdout, &output.stderr);
    if !output.success
        || !version_output
            .lines()
            .any(|line| line.trim_start().starts_with("ffmpeg version"))
    {
        return Err(format!(
            "Configured FFmpeg executable '{}' did not pass the FFmpeg version verification{}",
            executable.display(),
            process_output_suffix(&version_output)
        ));
    }
    Ok(executable)
}

/// Enumerate only actual DirectShow video devices and advertised, bounded
/// tuples.  Audio and `(none)` entries are deliberately omitted.
#[tauri::command]
pub fn list_video_camera_profiles() -> Result<Vec<VideoCameraProfileDescriptor>, String> {
    if !cfg!(target_os = "windows") {
        return Err(
            "Windows DirectShow camera profile discovery is unavailable on this platform"
                .to_string(),
        );
    }
    let ffmpeg = verified_ffmpeg_executable()?;
    let devices = list_dshow_video_devices(&ffmpeg)?;
    let mut profiles = BTreeMap::new();
    for device in devices {
        for profile in list_dshow_device_profiles(&ffmpeg, &device)? {
            let selection = CanonicalCameraSelection {
                device_alternative_name: device.alternative_name.clone(),
                input_format: profile.input_format.clone(),
                width: profile.width,
                height: profile.height,
                frame_rate_numerator: profile.frame_rate.numerator,
                frame_rate_denominator: profile.frame_rate.denominator,
            };
            let descriptor = descriptor_from_selection(&device.display_name, &selection)?;
            // DirectShow can print duplicate lines that differ only in color
            // metadata.  Keep one exact device/codec-or-pixel-format tuple.
            profiles
                .entry(descriptor.endpoint_name.clone())
                .or_insert(descriptor);
        }
    }
    Ok(profiles.into_values().collect())
}

/// Open one currently advertised exact tuple and require a complete raw RGBA
/// frame.  This is a probe, not a default-camera fallback path.
#[tauri::command]
pub fn probe_video_camera_profile(
    endpoint_name: String,
) -> Result<VideoCameraProfileProbeResult, String> {
    if !cfg!(target_os = "windows") {
        return Err(
            "Windows DirectShow camera profile probing is unavailable on this platform".to_string(),
        );
    }
    let selection = canonical_camera_selection(&endpoint_name)?;
    let ffmpeg = verified_ffmpeg_executable()?;
    let device = list_dshow_video_devices(&ffmpeg)?
        .into_iter()
        .find(|device| device.alternative_name == selection.device_alternative_name)
        .ok_or_else(|| {
            "Selected DirectShow camera is not currently enumerated by its exact alternative device identity"
                .to_string()
        })?;
    let advertised = list_dshow_device_profiles(&ffmpeg, &device)?;
    if !advertised.iter().any(|profile| {
        profile.input_format == selection.input_format
            && profile.width == selection.width
            && profile.height == selection.height
            && profile.frame_rate.numerator == selection.frame_rate_numerator
            && profile.frame_rate.denominator == selection.frame_rate_denominator
    }) {
        return Err(format!(
            "Selected DirectShow camera profile is not currently advertised by '{}'",
            device.display_name
        ));
    }
    let plan = crate::capture_transport::capture_ffmpeg_plan("camera", &endpoint_name)
        .map_err(|error| error.message)?;
    let mut process = Some(
        Command::new(&ffmpeg)
            .args(&plan.args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|error| {
                format!(
                    "Failed to open the selected DirectShow camera profile '{}': {error}",
                    device.display_name
                )
            })?,
    );
    let stdout = match process.as_mut().and_then(|process| process.stdout.take()) {
        Some(stdout) => stdout,
        None => {
            let error = "FFmpeg camera probe did not provide a raw-video stdout pipe".to_string();
            return Err(append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    "DirectShow camera profile probe",
                )
                .into_result(),
            ));
        }
    };
    let frame_bytes = match checked_frame_bytes(plan.frame.width, plan.frame.height) {
        Ok(frame_bytes) => frame_bytes,
        Err(error) => {
            drop(stdout);
            return Err(append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    "DirectShow camera profile probe",
                )
                .into_result(),
            ));
        }
    };
    let (frame_tx, frame_rx) = mpsc::sync_channel(1);
    let reader = std::thread::Builder::new()
        .name("syndocal-camera-profile-probe".to_string())
        .spawn(move || {
            let mut stdout = stdout;
            let mut frame = vec![0_u8; frame_bytes];
            let result = stdout
                .read_exact(&mut frame)
                .map(|()| frame)
                .map_err(|error| {
                    format!(
                    "Selected DirectShow camera profile did not produce one complete frame: {error}"
                )
                });
            let _ = frame_tx.send(result);
        })
        .map_err(|error| {
            let error = format!("Failed to start the bounded camera probe frame reader: {error}");
            append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    "DirectShow camera profile probe",
                )
                .into_result(),
            )
        })?;
    let read_result = await_probe_frame(&frame_rx, Duration::from_secs(7));
    let cleanup = terminate_capture_process(
        process.take().expect("spawned process must remain owned"),
        "DirectShow camera profile probe",
    );
    finish_reader_after_process_termination(reader, &cleanup);
    let cleanup_result = cleanup.into_result();
    match (read_result, cleanup_result) {
        (Ok(_), Ok(())) => {}
        (Ok(_), Err(cleanup_error)) => return Err(cleanup_error),
        (Err(error), Ok(())) => return Err(error),
        (Err(error), Err(cleanup_error)) => {
            return Err(format!("{error}; process cleanup failed: {cleanup_error}"))
        }
    }

    Ok(VideoCameraProfileProbeResult {
        success: true,
        endpoint_name,
        actual_width: selection.width,
        actual_height: selection.height,
        frame_rate_numerator: selection.frame_rate_numerator,
        frame_rate_denominator: selection.frame_rate_denominator,
        frame_rate_label: frame_rate_label(FrameRate {
            numerator: selection.frame_rate_numerator,
            denominator: selection.frame_rate_denominator,
        }),
        pixel_format: match &selection.input_format {
            CameraInputFormat::PixelFormat(value) => Some(value.clone()),
            CameraInputFormat::Vcodec(_) => None,
        },
        codec: match &selection.input_format {
            CameraInputFormat::Vcodec(value) => Some(value.clone()),
            CameraInputFormat::PixelFormat(_) => None,
        },
    })
}

fn list_dshow_video_devices(ffmpeg: &PathBuf) -> Result<Vec<DirectShowVideoDevice>, String> {
    let listing = run_dshow_listing_output(
        ffmpeg,
        [
            "-hide_banner",
            "-f",
            "dshow",
            "-list_devices",
            "true",
            "-i",
            "dummy",
        ],
        "DirectShow camera device listing",
    )?;
    let devices = parse_dshow_video_devices(&listing.text);
    require_dshow_video_device_listing_evidence(
        listing.success,
        &listing.text,
        &devices,
        "DirectShow camera device listing",
    )?;
    Ok(devices)
}

fn list_dshow_device_profiles(
    ffmpeg: &PathBuf,
    device: &DirectShowVideoDevice,
) -> Result<Vec<AdvertisedCameraProfile>, String> {
    let input = format!("video={}", device.alternative_name);
    let output = run_dshow_listing(
        ffmpeg,
        [
            "-hide_banner",
            "-f",
            "dshow",
            "-list_options",
            "true",
            "-i",
            input.as_str(),
        ],
        &format!(
            "DirectShow camera option listing for '{}'",
            device.display_name
        ),
        "DirectShow video device options",
    )?;
    Ok(parse_dshow_advertised_profiles(&output))
}

fn await_probe_frame<T>(
    receiver: &mpsc::Receiver<Result<T, String>>,
    timeout: Duration,
) -> Result<T, String> {
    match receiver.recv_timeout(timeout) {
        Ok(result) => result,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            Err("Timed out waiting for one complete DirectShow camera probe frame".to_string())
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            Err("DirectShow camera probe frame reader stopped unexpectedly".to_string())
        }
    }
}

pub(crate) fn canonical_camera_selection(
    endpoint_name: &str,
) -> Result<CanonicalCameraSelection, String> {
    if endpoint_name != endpoint_name.trim() {
        return Err(
            "Camera endpoint selection must be canonical without surrounding whitespace"
                .to_string(),
        );
    }
    let encoded = endpoint_name.strip_prefix(CAMERA_ENDPOINT_PREFIX).ok_or_else(|| {
        "Camera endpoint must be a canonical DirectShow profile selection; raw camera names are not accepted"
            .to_string()
    })?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| {
        "Camera endpoint selection is not valid canonical base64url data".to_string()
    })?;
    let payload = serde_json::from_slice::<CameraEndpointPayloadV1>(&bytes).map_err(|_| {
        "Camera endpoint selection is not a valid canonical DirectShow profile".to_string()
    })?;
    let selection = CanonicalCameraSelection {
        device_alternative_name: payload.device_alternative_name,
        input_format: payload.input_format,
        width: payload.width,
        height: payload.height,
        frame_rate_numerator: payload.frame_rate_numerator,
        frame_rate_denominator: payload.frame_rate_denominator,
    };
    validate_camera_selection(&selection)?;
    if canonical_camera_endpoint(&selection)? != endpoint_name {
        return Err("Camera endpoint selection is not in canonical form".to_string());
    }
    Ok(selection)
}

pub(crate) fn canonical_camera_endpoint(
    selection: &CanonicalCameraSelection,
) -> Result<String, String> {
    validate_camera_selection(selection)?;
    let payload = CameraEndpointPayloadV1 {
        device_alternative_name: selection.device_alternative_name.clone(),
        input_format: selection.input_format.clone(),
        width: selection.width,
        height: selection.height,
        frame_rate_numerator: selection.frame_rate_numerator,
        frame_rate_denominator: selection.frame_rate_denominator,
    };
    let bytes = serde_json::to_vec(&payload)
        .map_err(|error| format!("Failed to encode the canonical camera endpoint: {error}"))?;
    Ok(format!(
        "{CAMERA_ENDPOINT_PREFIX}{}",
        URL_SAFE_NO_PAD.encode(bytes)
    ))
}

pub(crate) fn checked_frame_bytes(width: u32, height: u32) -> Result<usize, String> {
    let bytes = u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or_else(|| "Camera frame byte length overflowed".to_string())?;
    usize::try_from(bytes).map_err(|_| "Camera frame byte length exceeds this process".to_string())
}

fn validate_camera_selection(selection: &CanonicalCameraSelection) -> Result<(), String> {
    if selection.device_alternative_name.is_empty()
        || selection.device_alternative_name.len() > 1024
        || selection
            .device_alternative_name
            .chars()
            .any(char::is_control)
    {
        return Err(
            "Camera endpoint has an invalid DirectShow alternative device name".to_string(),
        );
    }
    validate_input_format(&selection.input_format)?;
    let profile = AdvertisedCameraProfile {
        input_format: selection.input_format.clone(),
        width: selection.width,
        height: selection.height,
        frame_rate: FrameRate {
            numerator: selection.frame_rate_numerator,
            denominator: selection.frame_rate_denominator,
        },
    };
    if !profile_within_bounds(&profile) {
        return Err(
            "Camera endpoint profile is outside the supported DirectShow bounds".to_string(),
        );
    }
    checked_frame_bytes(selection.width, selection.height)?;
    Ok(())
}

fn validate_input_format(format: &CameraInputFormat) -> Result<(), String> {
    let value = match format {
        CameraInputFormat::Vcodec(value) | CameraInputFormat::PixelFormat(value) => value,
    };
    if value.is_empty()
        || value.len() > 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        return Err("Camera endpoint has an invalid DirectShow input format".to_string());
    }
    Ok(())
}

fn descriptor_from_selection(
    device_name: &str,
    selection: &CanonicalCameraSelection,
) -> Result<VideoCameraProfileDescriptor, String> {
    let endpoint_name = canonical_camera_endpoint(selection)?;
    let frame_rate = FrameRate {
        numerator: selection.frame_rate_numerator,
        denominator: selection.frame_rate_denominator,
    };
    Ok(VideoCameraProfileDescriptor {
        device_identity: sha256_hex(selection.device_alternative_name.as_bytes()),
        device_name: device_name.to_string(),
        profile_identity: sha256_hex(endpoint_name.as_bytes()),
        width: selection.width,
        height: selection.height,
        frame_rate_numerator: frame_rate.numerator,
        frame_rate_denominator: frame_rate.denominator,
        frame_rate_label: frame_rate_label(frame_rate),
        pixel_format: match &selection.input_format {
            CameraInputFormat::PixelFormat(value) => Some(value.clone()),
            CameraInputFormat::Vcodec(_) => None,
        },
        codec: match &selection.input_format {
            CameraInputFormat::Vcodec(value) => Some(value.clone()),
            CameraInputFormat::PixelFormat(_) => None,
        },
        endpoint_name,
    })
}

fn run_dshow_listing<const N: usize>(
    ffmpeg: &PathBuf,
    args: [&str; N],
    label: &str,
    required_marker: &str,
) -> Result<String, String> {
    let listing = run_dshow_listing_output(ffmpeg, args, label)?;
    // FFmpeg normally exits non-zero after a list-only dshow invocation. It
    // is acceptable only after the operation-specific evidence marker proves
    // that this was a complete option listing rather than a dshow startup
    // error that happened to mention the input format.
    require_dshow_listing_marker(listing.success, &listing.text, label, required_marker)?;
    Ok(listing.text)
}

struct DshowListingOutput {
    success: bool,
    text: String,
}

fn run_dshow_listing_output<const N: usize>(
    ffmpeg: &PathBuf,
    args: [&str; N],
    label: &str,
) -> Result<DshowListingOutput, String> {
    let mut command = Command::new(ffmpeg);
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = bounded_process_output(&mut command, label, FFMPEG_CAMERA_COMMAND_TIMEOUT)?;
    let text = combined_process_output(&output.stdout, &output.stderr);
    Ok(DshowListingOutput {
        success: output.success,
        text,
    })
}

fn require_dshow_video_device_listing_evidence(
    output_success: bool,
    text: &str,
    devices: &[DirectShowVideoDevice],
    label: &str,
) -> Result<(), String> {
    // Older FFmpeg releases print this section heading. Newer FFmpeg builds
    // can omit it even though they emit complete `(video)` plus `Alternative
    // name` pairs. A complete list-only invocation must also reach FFmpeg's
    // exact dummy-input terminator; otherwise a prefix emitted before a driver
    // failure would be semantic partial output, not a proven catalog.
    let legacy_heading = text.lines().any(|line| {
        strict_dshow_message(line)
            .is_some_and(|(_, message)| message.starts_with("DirectShow video devices"))
    });
    let complete = text
        .lines()
        .rev()
        .find(|line| !line.trim().is_empty())
        .is_some_and(|line| line.trim() == "Error opening input file dummy.");
    if complete && (legacy_heading || !devices.is_empty()) {
        return Ok(());
    }
    let exit_detail = if output_success {
        "success"
    } else {
        "non-success"
    };
    Err(format!(
        "{label} was not proven by FFmpeg: require the complete dummy-input terminator and either the legacy DirectShow heading or a strict video-device identity (exit status: {exit_detail}); raw device identities were withheld"
    ))
}

fn require_dshow_listing_marker(
    output_success: bool,
    text: &str,
    label: &str,
    required_marker: &str,
) -> Result<(), String> {
    if text.contains(required_marker) {
        return Ok(());
    }
    let exit_detail = if output_success {
        "success"
    } else {
        "non-success"
    };
    Err(format!(
        "{label} was not proven by FFmpeg: missing required marker '{required_marker}' (exit status: {exit_detail}); raw device output was withheld"
    ))
}

struct BoundedProcessOutput {
    success: bool,
    stdout: Vec<u8>,
    stderr: Vec<u8>,
}

enum ProcessOutputPart {
    Stdout(Result<Vec<u8>, String>),
    Stderr(Result<Vec<u8>, String>),
}

fn bounded_process_output(
    command: &mut Command,
    label: &str,
    timeout: Duration,
) -> Result<BoundedProcessOutput, String> {
    let mut process = Some(
        command
            .spawn()
            .map_err(|error| format!("Failed to start {label}: {error}"))?,
    );
    let stdout = match process.as_mut().and_then(|process| process.stdout.take()) {
        Some(stdout) => stdout,
        None => {
            let error = format!("{label} did not provide a stdout pipe");
            return Err(append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    label,
                )
                .into_result(),
            ));
        }
    };
    let stderr = match process.as_mut().and_then(|process| process.stderr.take()) {
        Some(stderr) => stderr,
        None => {
            let error = format!("{label} did not provide a stderr pipe");
            drop(stdout);
            return Err(append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    label,
                )
                .into_result(),
            ));
        }
    };
    let (output_tx, output_rx) = mpsc::sync_channel(2);
    let stdout_reader = match spawn_process_output_reader(stdout, true, output_tx.clone(), label) {
        Ok(reader) => reader,
        Err(error) => {
            return Err(append_process_cleanup_error(
                error,
                terminate_capture_process(
                    process.take().expect("spawned process must remain owned"),
                    label,
                )
                .into_result(),
            ));
        }
    };
    let stderr_reader = match spawn_process_output_reader(stderr, false, output_tx, label) {
        Ok(reader) => reader,
        Err(error) => {
            let cleanup = terminate_capture_process(
                process.take().expect("spawned process must remain owned"),
                label,
            );
            finish_reader_after_process_termination(stdout_reader, &cleanup);
            return Err(append_process_cleanup_error(error, cleanup.into_result()));
        }
    };
    let output = collect_bounded_process_output(&output_rx, timeout);
    let status = match process
        .as_mut()
        .expect("spawned process must remain owned")
        .try_wait()
        .map_err(|error| format!("Unable to inspect {label} completion: {error}"))
    {
        Ok(Some(status)) => Ok(Some(status)),
        Ok(None) if output.is_ok() => wait_for_process_exit(
            process.as_mut().expect("spawned process must remain owned"),
            FFMPEG_PROCESS_EXIT_GRACE,
            label,
        ),
        Ok(None) => Ok(None),
        Err(error) => Err(error),
    };
    let cleanup = if output.is_err() || !matches!(&status, Ok(Some(_))) {
        Some(terminate_capture_process(
            process.take().expect("spawned process must remain owned"),
            label,
        ))
    } else {
        None
    };
    if let Some(cleanup) = &cleanup {
        finish_reader_after_process_termination(stdout_reader, cleanup);
        finish_reader_after_process_termination(stderr_reader, cleanup);
    } else {
        let _ = stdout_reader.join();
        let _ = stderr_reader.join();
    }
    let cleanup_result = cleanup
        .map(ProcessTermination::into_result)
        .unwrap_or(Ok(()));
    let output = match output {
        Ok(output) => Ok(output),
        Err(error) => Err(error),
    };
    let status = match status {
        Ok(Some(status)) => Ok(status),
        Ok(None) => Err(format!(
            "{label} remained running after its stdout/stderr closed and was terminated"
        )),
        Err(error) => Err(error),
    };
    match (output, status, cleanup_result) {
        (Ok((stdout, stderr)), Ok(status), Ok(())) => Ok(BoundedProcessOutput {
            success: status.success(),
            stdout,
            stderr,
        }),
        (output, status, cleanup) => {
            let mut errors = Vec::new();
            if let Err(error) = output {
                errors.push(error);
            }
            if let Err(error) = status {
                errors.push(error);
            }
            if let Err(error) = cleanup {
                errors.push(format!("process cleanup failed: {error}"));
            }
            Err(errors.join("; "))
        }
    }
}

fn poll_for_process_exit<T>(
    mut try_wait: impl FnMut() -> Result<Option<T>, String>,
    grace: Duration,
) -> Result<Option<T>, String> {
    let started = std::time::Instant::now();
    loop {
        match try_wait()? {
            Some(status) => return Ok(Some(status)),
            None => {
                let Some(remaining) = grace.checked_sub(started.elapsed()) else {
                    return Ok(None);
                };
                std::thread::sleep(remaining.min(PROCESS_EXIT_POLL_INTERVAL));
            }
        }
    }
}

fn wait_for_process_exit(
    process: &mut std::process::Child,
    grace: Duration,
    label: &str,
) -> Result<Option<std::process::ExitStatus>, String> {
    poll_for_process_exit(
        || {
            process
                .try_wait()
                .map_err(|error| format!("Unable to inspect {label} completion: {error}"))
        },
        grace,
    )
}

fn spawn_process_output_reader<R>(
    stream: R,
    stdout: bool,
    sender: mpsc::SyncSender<ProcessOutputPart>,
    label: &str,
) -> Result<std::thread::JoinHandle<()>, String>
where
    R: Read + Send + 'static,
{
    std::thread::Builder::new()
        .name(
            if stdout {
                "syndocal-ffmpeg-stdout"
            } else {
                "syndocal-ffmpeg-stderr"
            }
            .to_string(),
        )
        .spawn(move || {
            let result = read_bounded_process_output(stream);
            let part = if stdout {
                ProcessOutputPart::Stdout(result)
            } else {
                ProcessOutputPart::Stderr(result)
            };
            let _ = sender.send(part);
        })
        .map_err(|error| format!("Failed to start bounded {label} output reader: {error}"))
}

fn read_bounded_process_output<R: Read>(stream: R) -> Result<Vec<u8>, String> {
    // DirectShow listing streams are untrusted driver output.  The one-byte
    // sentinel proves overflow while bounding each reader allocation.
    let mut stream = stream.take((MAX_FFMPEG_COMMAND_OUTPUT_BYTES + 1) as u64);
    let mut bytes = Vec::new();
    stream
        .read_to_end(&mut bytes)
        .map_err(|error| format!("FFmpeg output reader failed: {error}"))?;
    if bytes.len() > MAX_FFMPEG_COMMAND_OUTPUT_BYTES {
        return Err(format!(
            "FFmpeg command output exceeded the {}-byte limit",
            MAX_FFMPEG_COMMAND_OUTPUT_BYTES
        ));
    }
    Ok(bytes)
}

fn collect_bounded_process_output(
    receiver: &mpsc::Receiver<ProcessOutputPart>,
    timeout: Duration,
) -> Result<(Vec<u8>, Vec<u8>), String> {
    let started = std::time::Instant::now();
    let mut stdout = None;
    let mut stderr = None;
    while stdout.is_none() || stderr.is_none() {
        let remaining = timeout
            .checked_sub(started.elapsed())
            .ok_or_else(|| "Timed out waiting for FFmpeg stdout/stderr to close".to_string())?;
        match receiver.recv_timeout(remaining) {
            Ok(ProcessOutputPart::Stdout(Ok(bytes))) if stdout.is_none() => stdout = Some(bytes),
            Ok(ProcessOutputPart::Stderr(Ok(bytes))) if stderr.is_none() => stderr = Some(bytes),
            Ok(ProcessOutputPart::Stdout(Err(error)) | ProcessOutputPart::Stderr(Err(error))) => {
                return Err(error);
            }
            Ok(ProcessOutputPart::Stdout(Ok(_)) | ProcessOutputPart::Stderr(Ok(_))) => {
                return Err("FFmpeg output reader completed the same stream twice".to_string());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                return Err("Timed out waiting for FFmpeg stdout/stderr to close".to_string());
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err("FFmpeg output readers stopped unexpectedly".to_string());
            }
        }
    }
    Ok((
        stdout.expect("checked stdout completion"),
        stderr.expect("checked stderr completion"),
    ))
}

fn terminate_capture_process(process: std::process::Child, label: &str) -> ProcessTermination {
    terminate_process_or_defer(process, label, PROCESS_TERMINATION_REAP_TIMEOUT)
}

fn finish_reader_after_process_termination(
    reader: std::thread::JoinHandle<()>,
    termination: &ProcessTermination,
) {
    if termination.is_reaped() {
        let _ = reader.join();
    } else {
        // The reaper owns the still-live child and will eventually close this
        // pipe. Joining here can deadlock the caller on a reader blocked in a
        // driver/pipe read, so deliberately detach it instead.
        drop(reader);
    }
}

fn append_process_cleanup_error(error: String, cleanup: Result<(), String>) -> String {
    match cleanup {
        Ok(()) => error,
        Err(cleanup_error) => format!("{error}; process cleanup failed: {cleanup_error}"),
    }
}

fn combined_process_output(stdout: &[u8], stderr: &[u8]) -> String {
    let mut output = String::from_utf8_lossy(stdout).into_owned();
    if !output.is_empty() && !stderr.is_empty() {
        output.push('\n');
    }
    output.push_str(&String::from_utf8_lossy(stderr));
    output
}

fn process_output_suffix(output: &str) -> String {
    let detail = output.trim();
    if detail.is_empty() {
        String::new()
    } else {
        format!(": {detail}")
    }
}

fn strict_dshow_message(line: &str) -> Option<(&str, &str)> {
    let end = line.find("] ")?;
    let source = &line[..=end];
    let instance = source.strip_prefix("[dshow @ ")?.strip_suffix(']')?;
    if instance.is_empty() || !instance.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some((source, &line[end + 2..]))
}

fn exact_dshow_device_declaration(message: &str) -> Option<(&str, &str)> {
    let message = message.trim();
    if !message.starts_with('"') {
        return None;
    }
    let display_name = quoted_value(message)?;
    if display_name.is_empty()
        || display_name.len() > 1024
        || display_name.chars().any(char::is_control)
    {
        return None;
    }
    let quoted_end = message.find('"')? + 1 + display_name.len() + 1;
    let kind = message.get(quoted_end..)?.trim();
    if !matches!(kind, "(video)" | "(audio)" | "(none)") {
        return None;
    }
    Some((display_name, kind))
}

fn exact_dshow_alternative_name(message: &str) -> Option<&str> {
    let value = message.trim().strip_prefix("Alternative name ")?;
    let value = value.strip_prefix('"')?.strip_suffix('"')?;
    if value.is_empty()
        || value.len() > 1024
        || value.contains('"')
        || value.chars().any(char::is_control)
        || !value.starts_with("@device_")
    {
        return None;
    }
    Some(value)
}

fn parse_dshow_video_devices(output: &str) -> Vec<DirectShowVideoDevice> {
    let mut devices = Vec::new();
    let mut active_video: Option<(String, String)> = None;
    for raw_line in output.lines() {
        let Some((source, message)) = strict_dshow_message(raw_line) else {
            active_video = None;
            continue;
        };
        if let Some((display_name, kind)) = exact_dshow_device_declaration(message) {
            active_video =
                (kind == "(video)").then(|| (source.to_string(), display_name.to_string()));
            continue;
        }
        let pending = active_video.take();
        if let (Some((pending_source, display_name)), Some(alternative_name)) =
            (pending, exact_dshow_alternative_name(message))
        {
            if pending_source == source {
                devices.push(DirectShowVideoDevice {
                    display_name,
                    alternative_name: alternative_name.to_string(),
                });
            }
        }
    }
    let mut unique = BTreeMap::new();
    for device in devices {
        unique
            .entry(device.alternative_name.clone())
            .or_insert(device);
    }
    unique.into_values().collect()
}

fn parse_dshow_advertised_profiles(output: &str) -> Vec<AdvertisedCameraProfile> {
    let mut profiles = BTreeSet::new();
    for raw_line in output.lines() {
        let line = dshow_message(raw_line);
        let Some(input_format) = parse_input_format(line) else {
            continue;
        };
        let Some((width, height, rates)) = parse_option_dimensions_and_rates(line) else {
            continue;
        };
        for frame_rate in rates {
            let profile = AdvertisedCameraProfile {
                input_format: input_format.clone(),
                width,
                height,
                frame_rate,
            };
            if profile_within_bounds(&profile) {
                profiles.insert(profile_sort_key(&profile));
            }
        }
    }
    profiles
        .into_iter()
        .map(advertised_profile_from_sort_key)
        .collect()
}

type AdvertisedProfileSortKey = (u8, String, u32, u32, u32, u32);

fn profile_sort_key(profile: &AdvertisedCameraProfile) -> AdvertisedProfileSortKey {
    let (kind, value) = match &profile.input_format {
        CameraInputFormat::Vcodec(value) => (0, value.clone()),
        CameraInputFormat::PixelFormat(value) => (1, value.clone()),
    };
    (
        kind,
        value,
        profile.width,
        profile.height,
        profile.frame_rate.numerator,
        profile.frame_rate.denominator,
    )
}

fn advertised_profile_from_sort_key(key: AdvertisedProfileSortKey) -> AdvertisedCameraProfile {
    AdvertisedCameraProfile {
        input_format: if key.0 == 0 {
            CameraInputFormat::Vcodec(key.1)
        } else {
            CameraInputFormat::PixelFormat(key.1)
        },
        width: key.2,
        height: key.3,
        frame_rate: FrameRate {
            numerator: key.4,
            denominator: key.5,
        },
    }
}

fn dshow_message(line: &str) -> &str {
    line.split_once("] ").map_or(line, |(_, message)| message)
}

fn quoted_value(line: &str) -> Option<&str> {
    let start = line.find('"')? + 1;
    let remainder = &line[start..];
    let end = remainder.find('"')?;
    Some(&remainder[..end])
}

fn parse_input_format(line: &str) -> Option<CameraInputFormat> {
    for field in line.split_whitespace() {
        if let Some(value) = field.strip_prefix("vcodec=") {
            return valid_parsed_format(value)
                .map(|value| CameraInputFormat::Vcodec(value.to_string()));
        }
        if let Some(value) = field.strip_prefix("pixel_format=") {
            return valid_parsed_format(value)
                .map(|value| CameraInputFormat::PixelFormat(value.to_string()));
        }
    }
    None
}

fn valid_parsed_format(value: &str) -> Option<&str> {
    if value.is_empty()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
    {
        None
    } else {
        Some(value)
    }
}

fn parse_option_dimensions_and_rates(line: &str) -> Option<(u32, u32, Vec<FrameRate>)> {
    if let Some(min_offset) = line.find("min s=") {
        let min_section = &line[min_offset + 4..];
        let max_offset = min_section.find(" max s=")?;
        let max_section = &min_section[max_offset + 5..];
        let (minimum_width, minimum_height) = parse_dimensions(field_value(min_section, "s=")?)?;
        let (maximum_width, maximum_height) = parse_dimensions(field_value(max_section, "s=")?)?;
        // A resolution range is underdetermined.  Do not invent tuples by
        // combining either endpoint's fps with a different resolution.
        if (minimum_width, minimum_height) != (maximum_width, maximum_height) {
            return None;
        }
        let minimum_rate = parse_frame_rate(field_value(min_section, "fps=")?)?;
        let maximum_rate = parse_frame_rate(field_value(max_section, "fps=")?)?;
        return Some((
            minimum_width,
            minimum_height,
            bounded_advertised_rates(minimum_rate, maximum_rate),
        ));
    }

    let (width, height) = parse_dimensions(field_value(line, "s=")?)?;
    let rates = parse_rate_token(field_value(line, "fps=")?)?;
    Some((width, height, rates))
}

fn field_value<'a>(section: &'a str, label: &str) -> Option<&'a str> {
    let offset = section.find(label)? + label.len();
    section[offset..].split_whitespace().next()
}

fn parse_dimensions(value: &str) -> Option<(u32, u32)> {
    let (width, height) = value.split_once('x')?;
    let width = width.parse::<u32>().ok()?;
    let height = height.parse::<u32>().ok()?;
    (width > 0 && height > 0).then_some((width, height))
}

fn parse_rate_token(value: &str) -> Option<Vec<FrameRate>> {
    if let Some((minimum, maximum)) = value.split_once("..") {
        return Some(bounded_advertised_rates(
            parse_frame_rate(minimum)?,
            parse_frame_rate(maximum)?,
        ));
    }
    Some(vec![parse_frame_rate(value)?])
}

fn bounded_advertised_rates(minimum: FrameRate, maximum: FrameRate) -> Vec<FrameRate> {
    if compare_frame_rates(minimum, maximum).is_gt() {
        return Vec::new();
    }
    let mut rates = BTreeSet::new();
    rates.insert(minimum);
    rates.insert(maximum);
    for rate in STANDARD_RANGE_FRAME_RATES {
        if !compare_frame_rates(rate, minimum).is_lt()
            && !compare_frame_rates(rate, maximum).is_gt()
        {
            rates.insert(rate);
        }
    }
    let mut rates = rates.into_iter().collect::<Vec<_>>();
    rates.sort_by(|left, right| compare_frame_rates(*left, *right));
    rates
}

fn parse_frame_rate(value: &str) -> Option<FrameRate> {
    let value = value.trim_end_matches(',');
    if let Some((numerator, denominator)) = value.split_once('/') {
        let numerator = numerator.parse::<u32>().ok()?;
        let denominator = denominator.parse::<u32>().ok()?;
        return normalize_frame_rate(FrameRate {
            numerator,
            denominator,
        });
    }
    let (whole, fraction) = value.split_once('.').unwrap_or((value, ""));
    if whole.is_empty() || !whole.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    if !fraction.bytes().all(|byte| byte.is_ascii_digit()) || fraction.len() > 9 {
        return None;
    }
    let denominator = 10_u32.checked_pow(fraction.len() as u32)?;
    let whole = whole.parse::<u32>().ok()?;
    let fractional = if fraction.is_empty() {
        0
    } else {
        fraction.parse::<u32>().ok()?
    };
    let numerator = whole.checked_mul(denominator)?.checked_add(fractional)?;
    normalize_frame_rate(FrameRate {
        numerator,
        denominator,
    })
}

fn normalize_frame_rate(rate: FrameRate) -> Option<FrameRate> {
    if rate.numerator == 0 || rate.denominator == 0 {
        return None;
    }
    // FFmpeg device listings frequently round 60fps to 60.0002.  Treat that
    // documented rounding noise as exactly 60, while retaining nonstandard
    // endpoints as exact reduced rationals.
    for known in [
        FrameRate {
            numerator: 24,
            denominator: 1,
        },
        FrameRate {
            numerator: 25,
            denominator: 1,
        },
        FrameRate {
            numerator: 30,
            denominator: 1,
        },
        FrameRate {
            numerator: 50,
            denominator: 1,
        },
        FrameRate {
            numerator: 60,
            denominator: 1,
        },
        FrameRate {
            numerator: 120,
            denominator: 1,
        },
        FrameRate {
            numerator: 24_000,
            denominator: 1_001,
        },
        FrameRate {
            numerator: 30_000,
            denominator: 1_001,
        },
        FrameRate {
            numerator: 60_000,
            denominator: 1_001,
        },
        FrameRate {
            numerator: 120_000,
            denominator: 1_001,
        },
    ] {
        let difference = (i128::from(rate.numerator) * i128::from(known.denominator)
            - i128::from(known.numerator) * i128::from(rate.denominator))
        .unsigned_abs();
        // Within 0.01 fps.  This catches `60.0002`, not an unrelated mode.
        if difference * 100 < u128::from(rate.denominator) * u128::from(known.denominator) {
            return Some(known);
        }
    }
    let divisor = gcd(rate.numerator, rate.denominator);
    Some(FrameRate {
        numerator: rate.numerator / divisor,
        denominator: rate.denominator / divisor,
    })
}

fn gcd(mut left: u32, mut right: u32) -> u32 {
    while right != 0 {
        (left, right) = (right, left % right);
    }
    left
}

fn compare_frame_rates(left: FrameRate, right: FrameRate) -> std::cmp::Ordering {
    (u128::from(left.numerator) * u128::from(right.denominator))
        .cmp(&(u128::from(right.numerator) * u128::from(left.denominator)))
}

fn profile_within_bounds(profile: &AdvertisedCameraProfile) -> bool {
    if profile.width == 0
        || profile.height == 0
        || profile.width > MAX_CAMERA_WIDTH
        || profile.height > MAX_CAMERA_HEIGHT
        || profile.frame_rate.numerator == 0
        || profile.frame_rate.denominator == 0
        || compare_frame_rates(profile.frame_rate, MAX_CAMERA_FRAME_RATE).is_gt()
    {
        return false;
    }
    if (profile.width > 1920 || profile.height > 1080)
        && compare_frame_rates(profile.frame_rate, MAX_HIGH_RESOLUTION_FRAME_RATE).is_gt()
    {
        return false;
    }
    if (profile.width > 1280 || profile.height > 720)
        && compare_frame_rates(profile.frame_rate, MAX_FULL_HD_FRAME_RATE).is_gt()
    {
        return false;
    }
    validate_input_format(&profile.input_format).is_ok()
}

fn frame_rate_label(rate: FrameRate) -> String {
    if rate.denominator == 1 {
        return format!("{} fps", rate.numerator);
    }
    let mut value = format!(
        "{:.3}",
        f64::from(rate.numerator) / f64::from(rate.denominator)
    );
    while value.ends_with('0') {
        value.pop();
    }
    if value.ends_with('.') {
        value.pop();
    }
    format!("{value} fps")
}

fn sha256_hex(value: &[u8]) -> String {
    format!("{:x}", Sha256::digest(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEVICE_LISTING: &str = r#"
[dshow @ 000001] DirectShow video devices (some may be both video and audio devices)
[dshow @ 000001]  "Insta360 Link" (video)
[dshow @ 000001]     Alternative name "@device_pnp_\\?\usb#vid_2e1a&pid_4c01#insta360"
[dshow @ 000001]  "Insta360 Link" (none)
[dshow @ 000001]     Alternative name "@device_cm_{not-a-video-device}"
[dshow @ 000001] DirectShow audio devices
[dshow @ 000001]  "Insta360 Link Microphone" (audio)
[dshow @ 000001]     Alternative name "@device_cm_{not-an-input}"
Error opening input file dummy.
"#;

    // FFmpeg 7 on the show PC emits complete video identities but omits the
    // historical "DirectShow video devices" section heading.
    const MODERN_DEVICE_LISTING_WITHOUT_SECTION_HEADING: &str = r#"
[dshow @ 000001]  "NDI Webcam Video 1" (video)
[dshow @ 000001]   Alternative name "@device_pnp_\\?\root#media#0003#vidsource0"
[dshow @ 000001]  "Insta360 Link" (video)
[dshow @ 000001]   Alternative name "@device_pnp_\\?\usb#vid_2e1a&pid_4c01#insta360"
[in#0 @ 000001] Error opening input: Immediate exit requested
Error opening input file dummy.
"#;

    const OPTION_LISTING: &str = r#"
[dshow @ 000001] DirectShow video device options (from video devices)
[dshow @ 000001]  Pin "Capture" (alternative pin name "0")
[dshow @ 000001]   vcodec=mjpeg min s=1920x1080 fps=24 max s=1920x1080 fps=60.0002
[dshow @ 000001]   vcodec=mjpeg min s=1920x1080 fps=24 max s=1920x1080 fps=60.0002 color_range=unknown
[dshow @ 000001]   vcodec=h264 s=3840x2160 fps=24..30
[dshow @ 000001]   pixel_format=yuyv422 s=640x480 fps=30
[dshow @ 000001]   pixel_format=yuyv422 s=4096x2160 fps=60
[dshow @ 000001]   pixel_format=yuyv422 s=4096x2160 fps=120
[dshow @ 000001]   vcodec=mjpeg min s=640x480 fps=30 max s=1920x1080 fps=60
"#;

    #[test]
    fn dshow_device_parser_keeps_only_video_devices_with_alternative_names() {
        let devices = parse_dshow_video_devices(DEVICE_LISTING);
        assert_eq!(
            devices,
            vec![DirectShowVideoDevice {
                display_name: "Insta360 Link".to_string(),
                alternative_name: "@device_pnp_\\\\?\\usb#vid_2e1a&pid_4c01#insta360".to_string(),
            }]
        );
        require_dshow_video_device_listing_evidence(
            false,
            DEVICE_LISTING,
            &devices,
            "DirectShow camera device listing",
        )
        .unwrap();
    }

    #[test]
    fn dshow_device_listing_accepts_complete_identities_without_legacy_section_heading() {
        assert!(!MODERN_DEVICE_LISTING_WITHOUT_SECTION_HEADING.contains("DirectShow video devices"));
        let devices = parse_dshow_video_devices(MODERN_DEVICE_LISTING_WITHOUT_SECTION_HEADING);
        assert_eq!(devices.len(), 2);
        for output_success in [false, true] {
            require_dshow_video_device_listing_evidence(
                output_success,
                MODERN_DEVICE_LISTING_WITHOUT_SECTION_HEADING,
                &devices,
                "DirectShow camera device listing",
            )
            .unwrap();
        }
    }

    #[test]
    fn dshow_device_parser_requires_exact_kind_source_and_adjacent_alternative_identity() {
        let hostile = r#"
[dshow @ 000001] "Mic (video)" (audio)
[dshow @ 000001] Alternative name "@device_cm_audio"
[dshow @ 000001] "Unavailable (video)" (none)
[dshow @ 000001] Alternative name "@device_sw_none"
[foreign @ 000001] "Forged" (video)
[foreign @ 000001] Alternative name "@device_pnp_forged"
[dshow @ 000001] "Intervened" (video)
[foreign @ 000001] unrelated
[dshow @ 000001] Alternative name "@device_pnp_intervened"
[dshow @ 000001] "Intervened by dshow" (video)
[dshow @ 000001] driver diagnostic
[dshow @ 000001] Alternative name "@device_pnp_dshow_intervened"
[dshow @ 000001] "Malformed alternative" (video)
[dshow @ 000001] Alternative name not-quoted
[dshow @ 000001] "Wrong source" (video)
[dshow @ 000002] Alternative name "@device_pnp_wrong_source"
[dshow @ 000001] arbitrary diagnostic "Forged diagnostic" (video)
[dshow @ 000001] Alternative name "@device_pnp_forged_diagnostic"
[dshow @ not-hex] "Forged instance" (video)
[dshow @ not-hex] Alternative name "@device_pnp_forged_instance"
Error opening input file dummy.
"#;
        assert!(parse_dshow_video_devices(hostile).is_empty());
        let error = require_dshow_video_device_listing_evidence(
            false,
            hostile,
            &[],
            "DirectShow camera device listing",
        )
        .unwrap_err();
        assert!(error.contains("strict video-device identity"));
    }

    #[test]
    fn dshow_device_listing_rejects_semantic_partial_without_dummy_terminator() {
        let partial = r#"
[dshow @ 000001] "Insta360 Link" (video)
[dshow @ 000001] Alternative name "@device_pnp_insta360"
[dshow @ 000001] Could not enumerate remaining DirectShow input devices
"#;
        let devices = parse_dshow_video_devices(partial);
        assert_eq!(devices.len(), 1);
        for output_success in [false, true] {
            let error = require_dshow_video_device_listing_evidence(
                output_success,
                partial,
                &devices,
                "DirectShow camera device listing",
            )
            .unwrap_err();
            assert!(error.contains("complete dummy-input terminator"));
        }
    }

    #[test]
    fn dshow_device_listing_rejects_identity_tail_after_dummy_terminator() {
        let forged_tail = r#"
[dshow @ 000001] "Insta360 Link" (video)
[dshow @ 000001] Alternative name "@device_pnp_insta360"
Error opening input file dummy.
[dshow @ 000001] "Late forged device" (video)
[dshow @ 000001] Alternative name "@device_pnp_late_forged"
"#;
        let devices = parse_dshow_video_devices(forged_tail);
        assert_eq!(devices.len(), 2);
        assert!(require_dshow_video_device_listing_evidence(
            false,
            forged_tail,
            &devices,
            "DirectShow camera device listing",
        )
        .is_err());
    }

    #[test]
    fn dshow_device_listing_legacy_heading_allows_proven_empty_catalog_only_after_terminal() {
        let complete_empty = concat!(
            "[dshow @ 000001] DirectShow video devices\n",
            "Error opening input file dummy.\n"
        );
        assert!(parse_dshow_video_devices(complete_empty).is_empty());
        require_dshow_video_device_listing_evidence(
            false,
            complete_empty,
            &[],
            "DirectShow camera device listing",
        )
        .unwrap();

        let incomplete_empty = "[dshow @ 000001] DirectShow video devices\n";
        assert!(require_dshow_video_device_listing_evidence(
            false,
            incomplete_empty,
            &[],
            "DirectShow camera device listing",
        )
        .is_err());
    }

    #[test]
    fn dshow_device_parser_deduplicates_alternative_identity_across_kind_ordering() {
        let listing = r#"
[dshow @ 000001] "First label" (video)
[dshow @ 000001] Alternative name "@device_pnp_same"
[dshow @ 000001] "Audio (video)" (audio)
[dshow @ 000001] Alternative name "@device_cm_audio"
[dshow @ 000001] "Second label" (video)
[dshow @ 000001] Alternative name "@device_pnp_same"
[dshow @ 000001] "None" (none)
[dshow @ 000001] Alternative name "@device_sw_none"
Error opening input file dummy.
"#;
        assert_eq!(
            parse_dshow_video_devices(listing),
            vec![DirectShowVideoDevice {
                display_name: "First label".to_string(),
                alternative_name: "@device_pnp_same".to_string(),
            }]
        );
    }

    #[test]
    fn dshow_device_listing_errors_withhold_raw_device_and_audio_names() {
        let sensitive = r#"
[dshow @ 000001] "Private Camera" (video)
[dshow @ 000001] Alternative name "@device_pnp_private"
[dshow @ 000001] "Private Microphone" (audio)
[dshow @ 000001] Alternative name "@device_cm_private"
"#;
        let devices = parse_dshow_video_devices(sensitive);
        let error = require_dshow_video_device_listing_evidence(
            false,
            sensitive,
            &devices,
            "DirectShow camera device listing",
        )
        .unwrap_err();
        assert!(error.contains("raw device identities were withheld"));
        assert!(!error.contains("Private Camera"));
        assert!(!error.contains("Private Microphone"));
        assert!(!error.contains("@device_"));
    }

    #[test]
    fn dshow_listing_rejects_generic_errors_without_required_evidence() {
        let error = require_dshow_video_device_listing_evidence(
            false,
            "[dshow @ 000001] Could not enumerate DirectShow input devices",
            &[],
            "DirectShow camera device listing",
        )
        .unwrap_err();
        assert!(error.contains("complete dummy-input terminator"));
        assert!(error.contains("exit status: non-success"));
        assert!(!error.contains("Could not enumerate"));
        assert!(require_dshow_listing_marker(
            false,
            "[dshow @ 000001] DirectShow video device options (from video devices)",
            "DirectShow camera option listing",
            "DirectShow video device options",
        )
        .is_ok());

        let profile_error = require_dshow_listing_marker(
            false,
            "[dshow @ 000001] selected @device_pnp_private for Private Camera",
            "DirectShow camera option listing",
            "DirectShow video device options",
        )
        .unwrap_err();
        assert!(profile_error.contains("raw device output was withheld"));
        assert!(!profile_error.contains("@device_pnp_private"));
        assert!(!profile_error.contains("Private Camera"));
    }

    #[test]
    fn dshow_options_expand_only_bounded_standard_rates_and_filter_unsafe_profiles() {
        let profiles = parse_dshow_advertised_profiles(OPTION_LISTING);
        let tuples = profiles
            .iter()
            .map(|profile| {
                (
                    profile.input_format.clone(),
                    profile.width,
                    profile.height,
                    profile.frame_rate,
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(
            tuples,
            vec![
                (
                    CameraInputFormat::Vcodec("h264".to_string()),
                    3840,
                    2160,
                    FrameRate {
                        numerator: 24,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("h264".to_string()),
                    3840,
                    2160,
                    FrameRate {
                        numerator: 25,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("h264".to_string()),
                    3840,
                    2160,
                    FrameRate {
                        numerator: 30,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("mjpeg".to_string()),
                    1920,
                    1080,
                    FrameRate {
                        numerator: 24,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("mjpeg".to_string()),
                    1920,
                    1080,
                    FrameRate {
                        numerator: 25,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("mjpeg".to_string()),
                    1920,
                    1080,
                    FrameRate {
                        numerator: 30,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("mjpeg".to_string()),
                    1920,
                    1080,
                    FrameRate {
                        numerator: 50,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::Vcodec("mjpeg".to_string()),
                    1920,
                    1080,
                    FrameRate {
                        numerator: 60,
                        denominator: 1
                    }
                ),
                (
                    CameraInputFormat::PixelFormat("yuyv422".to_string()),
                    640,
                    480,
                    FrameRate {
                        numerator: 30,
                        denominator: 1
                    }
                ),
            ]
        );
    }

    #[test]
    fn dshow_endpoint_is_opaque_canonical_and_uses_the_exact_alternative_name_for_identity() {
        let selection = CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_\\\\?\\usb#vid_2e1a&pid_4c01#insta360"
                .to_string(),
            input_format: CameraInputFormat::Vcodec("mjpeg".to_string()),
            width: 3840,
            height: 2160,
            frame_rate_numerator: 30,
            frame_rate_denominator: 1,
        };
        let endpoint = canonical_camera_endpoint(&selection).unwrap();
        assert!(endpoint.starts_with(CAMERA_ENDPOINT_PREFIX));
        assert_eq!(canonical_camera_selection(&endpoint).unwrap(), selection);
        assert!(canonical_camera_selection("Insta360 Link").is_err());
        assert!(canonical_camera_selection("video=Insta360 Link").is_err());
        let descriptor = descriptor_from_selection("Insta360 Link", &selection).unwrap();
        assert_eq!(
            descriptor.device_identity,
            sha256_hex(selection.device_alternative_name.as_bytes())
        );
        assert_eq!(descriptor.pixel_format, None);
        assert_eq!(descriptor.codec.as_deref(), Some("mjpeg"));
        let serialized = serde_json::to_value(descriptor).unwrap();
        assert!(serialized.get("device_identity").is_some());
        assert!(serialized.get("frame_rate_numerator").is_some());
        assert!(serialized.get("endpoint_name").is_some());
    }

    #[test]
    fn camera_profile_bounds_and_frame_byte_math_fail_closed() {
        assert!(canonical_camera_endpoint(&CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_1".to_string(),
            input_format: CameraInputFormat::PixelFormat("yuyv422".to_string()),
            width: 4096,
            height: 2160,
            frame_rate_numerator: 30,
            frame_rate_denominator: 1,
        })
        .is_ok());
        assert!(canonical_camera_endpoint(&CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_1".to_string(),
            input_format: CameraInputFormat::PixelFormat("yuyv422".to_string()),
            width: 4096,
            height: 2160,
            frame_rate_numerator: 60,
            frame_rate_denominator: 1,
        })
        .is_err());
        assert!(canonical_camera_endpoint(&CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_1".to_string(),
            input_format: CameraInputFormat::PixelFormat("yuyv422".to_string()),
            width: 1920,
            height: 1080,
            frame_rate_numerator: 60,
            frame_rate_denominator: 1,
        })
        .is_ok());
        assert!(canonical_camera_endpoint(&CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_1".to_string(),
            input_format: CameraInputFormat::PixelFormat("yuyv422".to_string()),
            width: 1280,
            height: 720,
            frame_rate_numerator: 120,
            frame_rate_denominator: 1,
        })
        .is_ok());
        assert!(canonical_camera_endpoint(&CanonicalCameraSelection {
            device_alternative_name: "@device_pnp_1".to_string(),
            input_format: CameraInputFormat::PixelFormat("yuyv422".to_string()),
            width: 1920,
            height: 1080,
            frame_rate_numerator: 120,
            frame_rate_denominator: 1,
        })
        .is_err());
        assert_eq!(checked_frame_bytes(4096, 2160).unwrap(), 35_389_440);
        assert!(checked_frame_bytes(u32::MAX, u32::MAX).is_err());
    }

    #[test]
    fn probe_frame_timeout_fails_closed_before_the_caller_owned_cleanup() {
        let (_sender, receiver) = mpsc::sync_channel::<Result<(), String>>(1);
        let error = await_probe_frame(&receiver, Duration::from_millis(1)).unwrap_err();
        assert!(error.contains("Timed out"));
    }

    #[test]
    fn ffmpeg_listing_timeout_fails_closed_before_the_caller_owned_cleanup() {
        let (_sender, receiver) = mpsc::sync_channel::<ProcessOutputPart>(2);
        let error =
            collect_bounded_process_output(&receiver, Duration::from_millis(1)).unwrap_err();
        assert!(error.contains("Timed out"));
    }

    #[test]
    fn ffmpeg_listing_output_overflow_fails_closed_before_the_caller_owned_cleanup() {
        let overflow = read_bounded_process_output(std::io::Cursor::new(vec![
            0_u8;
            MAX_FFMPEG_COMMAND_OUTPUT_BYTES
                + 1
        ]))
        .unwrap_err();
        assert!(overflow.contains("exceeded"));
        let (sender, receiver) = mpsc::sync_channel::<ProcessOutputPart>(2);
        sender
            .send(ProcessOutputPart::Stdout(Err(overflow)))
            .unwrap();
        let error = collect_bounded_process_output(&receiver, Duration::from_secs(1)).unwrap_err();
        assert!(error.contains("exceeded"));
    }

    #[test]
    fn ffmpeg_listing_reports_cleanup_failure_alongside_the_primary_error() {
        let (sender, receiver) = mpsc::sync_channel::<ProcessOutputPart>(2);
        sender
            .send(ProcessOutputPart::Stdout(Err("reader failed".to_string())))
            .unwrap();
        let error = collect_bounded_process_output(&receiver, Duration::from_secs(1)).unwrap_err();
        let error = append_process_cleanup_error(error, Err("kill/reap failed".to_string()));
        assert!(error.contains("reader failed"));
        assert!(error.contains("process cleanup failed: kill/reap failed"));
    }

    #[test]
    fn deferred_cleanup_does_not_join_a_reader_that_would_block() {
        let (entered_tx, entered_rx) = mpsc::sync_channel(1);
        let (release_tx, release_rx) = mpsc::sync_channel::<()>(1);
        let reader = std::thread::spawn(move || {
            entered_tx.send(()).unwrap();
            let _ = release_rx.recv();
        });
        entered_rx
            .recv_timeout(Duration::from_secs(1))
            .expect("reader must block before cleanup");
        let deferred = crate::capture_process_lifecycle::deferred_termination_for_test(
            "kill/reap deadline expired",
        );
        let started = std::time::Instant::now();
        finish_reader_after_process_termination(reader, &deferred);
        assert!(
            started.elapsed() < Duration::from_millis(50),
            "deferred cleanup must detach instead of joining the blocked reader"
        );
        release_tx.send(()).unwrap();
    }

    #[test]
    fn process_exit_grace_is_bounded_when_a_child_stays_running() {
        let started = std::time::Instant::now();
        let status =
            poll_for_process_exit(|| Ok::<Option<()>, String>(None), Duration::from_millis(15))
                .unwrap();
        assert!(status.is_none());
        assert!(started.elapsed() < Duration::from_millis(150));
    }

    #[test]
    fn process_exit_grace_reaps_a_child_after_an_initial_running_poll_and_release() {
        let mut command = if cfg!(target_os = "windows") {
            let mut command =
                Command::new(std::env::var_os("COMSPEC").expect("COMSPEC must be available"));
            command.args(["/d", "/s", "/c", "set /p _= & exit /b 0"]);
            command
        } else {
            let mut command = Command::new("sh");
            command.args(["-c", "IFS= read -r _"]);
            command
        };
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let mut process = command.spawn().expect("short-lived test child must start");
        let mut stdin = process.stdin.take().expect("test child must provide stdin");
        assert!(process
            .try_wait()
            .expect("initial child status must be observable")
            .is_none());
        stdin
            .write_all(b"\n")
            .expect("test child release must be writable");
        drop(stdin);
        let status = wait_for_process_exit(
            &mut process,
            FFMPEG_PROCESS_EXIT_GRACE,
            "process-exit-grace test",
        )
        .expect("test child status must be observable")
        .expect("test child must exit within the grace window");
        assert!(status.success());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn post_spawn_catalog_cleanup_kills_and_reaps_the_child() {
        let command_shell = std::env::var_os("COMSPEC").expect("COMSPEC must be available");
        let process = Command::new(command_shell)
            .args(["/d", "/s", "/c", "for /L %i in (1,1,2147483647) do @rem"])
            .spawn()
            .unwrap();
        let termination = terminate_capture_process(process, "test process termination");
        assert!(termination.is_reaped());
    }

    #[test]
    fn probe_result_serializes_the_frozen_frontend_shape() {
        let encoded = serde_json::to_value(VideoCameraProfileProbeResult {
            success: true,
            endpoint_name: "syndocal-camera-v1:opaque".to_string(),
            actual_width: 3840,
            actual_height: 2160,
            frame_rate_numerator: 30,
            frame_rate_denominator: 1,
            frame_rate_label: "30 fps".to_string(),
            pixel_format: None,
            codec: Some("h264".to_string()),
        })
        .unwrap();
        assert_eq!(
            encoded,
            serde_json::json!({
                "success": true,
                "endpoint_name": "syndocal-camera-v1:opaque",
                "actual_width": 3840,
                "actual_height": 2160,
                "frame_rate_numerator": 30,
                "frame_rate_denominator": 1,
                "frame_rate_label": "30 fps",
                "pixel_format": null,
                "codec": "h264",
            })
        );
    }
}
