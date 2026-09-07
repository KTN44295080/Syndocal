//! Isolated renderer for recording frames.
//!
//! The application owns the recording protocol and encoder, while this child
//! owns synchronous video decode, HAP/libav work, GPU composition and effects.
//! A bounded length-prefixed IPC stream keeps the hot path single-process and
//! makes the whole renderer killable without detaching an in-process thread.

use super::video_recording::encoder::process::ReapingChild;
use serde::{Deserialize, Serialize};
use std::{
    env,
    ffi::OsString,
    io::{self, Read, Write},
    process::{Command, Stdio},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

const WORKER_ARGUMENT: &str = "--syndocal-recording-renderer-worker";
const MAX_WORKER_REQUEST_BYTES: usize = 128 * 1024 * 1024;
const MAX_FRAME_BYTES: usize = 4096 * 4096 * 4;
const RESPONSE_POLL_INTERVAL: Duration = Duration::from_millis(25);
const RENDER_RESPONSE_DEADLINE: Duration = Duration::from_secs(10);
const CHILD_REAP_DEADLINE: Duration = Duration::from_millis(250);
const REQUEST_MAGIC: &[u8; 4] = b"SRF1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct RecordingRenderRequest {
    pub(crate) video: protocol::VideoSnapshot,
    pub(crate) clip_runtime: protocol::VideoClipRuntimeSnapshot,
    pub(crate) transition_runtime: protocol::VideoLayerTransitionRuntimeSnapshot,
    pub(crate) project_render_epoch: u64,
    pub(crate) bpm: f32,
    pub(crate) output_id: protocol::VideoOutputId,
    pub(crate) width: u32,
    pub(crate) height: u32,
    /// Camera/Screen/NDI/Spout frames are owned by the parent transport. They
    /// cross the broker as data; all decode/composition/effect work remains in
    /// the killable child.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) injected_frames: Vec<video::VideoFrame>,
}

#[derive(Debug, Serialize)]
struct RecordingRenderRequestHeader<'a> {
    video: &'a protocol::VideoSnapshot,
    clip_runtime: &'a protocol::VideoClipRuntimeSnapshot,
    transition_runtime: &'a protocol::VideoLayerTransitionRuntimeSnapshot,
    project_render_epoch: u64,
    bpm: f32,
    output_id: protocol::VideoOutputId,
    width: u32,
    height: u32,
    injected_frames: Vec<RecordingInjectedFrameHeader>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordingRenderRequestHeaderOwned {
    video: protocol::VideoSnapshot,
    clip_runtime: protocol::VideoClipRuntimeSnapshot,
    transition_runtime: protocol::VideoLayerTransitionRuntimeSnapshot,
    project_render_epoch: u64,
    bpm: f32,
    output_id: protocol::VideoOutputId,
    width: u32,
    height: u32,
    injected_frames: Vec<RecordingInjectedFrameHeader>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RecordingInjectedFrameHeader {
    layer_id: u64,
    width: u32,
    height: u32,
    pts_ms: u64,
    duration_ms: u64,
    format: video::VideoPixelFormat,
    data_len: u32,
}

pub(crate) struct RecordingRendererProcess {
    child: Arc<Mutex<ReapingChild>>,
    stdin: Mutex<std::process::ChildStdin>,
    responses: mpsc::Receiver<Result<video::VideoFrame, String>>,
    stop_watchdog: Arc<std::sync::atomic::AtomicBool>,
    watchdog: Option<thread::JoinHandle<()>>,
}

impl RecordingRendererProcess {
    pub(crate) fn spawn(stop: Arc<std::sync::atomic::AtomicBool>) -> Result<Self, String> {
        let executable = env::current_exe()
            .map_err(|error| format!("Unable to locate Syndocal recording renderer: {error}"))?;
        let mut command = Command::new(executable);
        command
            .arg(WORKER_ARGUMENT)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            // A worker error is returned over the framed protocol. Keeping
            // stderr closed prevents a hung child from filling an inherited
            // pipe and masking the termination boundary.
            .stderr(Stdio::null());
        let mut child = ReapingChild::new(
            command
                .spawn()
                .map_err(|error| format!("Unable to start recording renderer: {error}"))?,
        )?;
        let stdin = child
            .take_stdin()
            .ok_or_else(|| "Recording renderer stdin was unavailable".to_string())?;
        let stdout = child
            .take_stdout()
            .ok_or_else(|| "Recording renderer stdout was unavailable".to_string())?;
        let child = Arc::new(Mutex::new(child));
        let (sender, responses) = mpsc::channel();
        thread::Builder::new()
            .name("syndocal-recording-renderer-reader".to_string())
            .spawn(move || renderer_response_reader(stdout, sender))
            .map_err(|error| format!("Unable to start recording renderer reader: {error}"))?;
        let stop_watchdog = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let watchdog_stop = Arc::clone(&stop_watchdog);
        let watchdog_child = Arc::clone(&child);
        let watchdog = thread::Builder::new()
            .name("syndocal-recording-renderer-watchdog".to_string())
            .spawn(move || {
                while !watchdog_stop.load(std::sync::atomic::Ordering::Acquire)
                    && !stop.load(std::sync::atomic::Ordering::Acquire)
                {
                    thread::sleep(Duration::from_millis(5));
                }
                if stop.load(std::sync::atomic::Ordering::Acquire) {
                    if let Ok(mut child) = watchdog_child.lock() {
                        let _ = child.terminate();
                    }
                }
            })
            .map_err(|error| format!("Unable to start recording renderer watchdog: {error}"))?;
        Ok(Self {
            child,
            stdin: Mutex::new(stdin),
            responses,
            stop_watchdog,
            watchdog: Some(watchdog),
        })
    }

    pub(crate) fn render(
        &self,
        request: &RecordingRenderRequest,
        stop: &std::sync::atomic::AtomicBool,
    ) -> Result<video::VideoFrame, String> {
        let request_bytes = encode_render_request(request)?;
        if request_bytes.len() > MAX_WORKER_REQUEST_BYTES {
            return Err("Recording renderer request exceeded 128 MiB".to_string());
        }
        {
            let mut stdin = self
                .stdin
                .lock()
                .map_err(|_| "Recording renderer stdin lock was poisoned".to_string())?;
            write_payload(&mut *stdin, &request_bytes)
                .map_err(|error| format!("Recording renderer request pipe failed: {error}"))?;
        }
        let deadline = Instant::now() + RENDER_RESPONSE_DEADLINE;
        loop {
            if stop.load(std::sync::atomic::Ordering::Acquire) {
                self.terminate_and_reap(CHILD_REAP_DEADLINE);
                return Err("Recording renderer cancelled".to_string());
            }
            match self.responses.recv_timeout(RESPONSE_POLL_INTERVAL) {
                Ok(Ok(frame)) => return Ok(frame),
                Ok(Err(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err("Recording renderer exited before returning a frame".to_string())
                }
                Err(mpsc::RecvTimeoutError::Timeout) if Instant::now() >= deadline => {
                    self.terminate_and_reap(CHILD_REAP_DEADLINE);
                    return Err("Recording renderer exceeded its 10 s frame deadline".to_string());
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    pub(crate) fn terminate_and_reap(&self, deadline: Duration) {
        let Ok(mut child) = self.child.lock() else {
            return;
        };
        let _ = child.terminate();
        let end = Instant::now() + deadline;
        loop {
            match child.poll() {
                Ok(Some(_)) | Err(_) => break,
                Ok(None) if Instant::now() >= end => break,
                Ok(None) => thread::sleep(Duration::from_millis(5)),
            }
        }
    }
}

impl Drop for RecordingRendererProcess {
    fn drop(&mut self) {
        self.stop_watchdog
            .store(true, std::sync::atomic::Ordering::Release);
        if let Some(watchdog) = self.watchdog.take() {
            let _ = watchdog.join();
        }
        self.terminate_and_reap(CHILD_REAP_DEADLINE);
    }
}

pub(crate) fn worker_cli_requested(mut args: impl Iterator<Item = OsString>) -> bool {
    matches!(args.next().as_deref(), Some(value) if value == std::ffi::OsStr::new(WORKER_ARGUMENT))
        && args.next().is_none()
}

pub(crate) fn run_worker() -> Result<(), String> {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut input = stdin.lock();
    let mut output = stdout.lock();
    let decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::from_env();
    let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
        video::VideoRuntimeConfig::default(),
        video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
    );
    while let Some(bytes) = read_payload(&mut input)
        .map_err(|error| format!("Recording renderer request read failed: {error}"))?
    {
        let request = match decode_render_request(&bytes) {
            Ok(request) => request,
            Err(error) => {
                write_error(
                    &mut output,
                    format!("Invalid recording renderer request: {error}"),
                )?;
                continue;
            }
        };
        if request.width == 0
            || request.height == 0
            || request.width > 4096
            || request.height > 4096
            || !request.bpm.is_finite()
        {
            write_error(
                &mut output,
                "Recording renderer request dimensions or BPM are invalid",
            )?;
            continue;
        }
        let expected_len = request.width as usize * request.height as usize * 4;
        if expected_len > MAX_FRAME_BYTES
            || request.injected_frames.iter().any(|frame| {
                frame.width == 0
                    || frame.height == 0
                    || frame.width as usize * frame.height as usize * 4 != frame.data.len()
                    || frame.data.len() > MAX_FRAME_BYTES
            })
        {
            write_error(&mut output, "Recording renderer frame payload is invalid")?;
            continue;
        }
        renderer
            .frame_provider_mut()
            .decoder_mut()
            .replace_injected_frames(
                request
                    .injected_frames
                    .into_iter()
                    .map(|frame| (frame.layer_id, frame))
                    .collect(),
            );
        renderer.frame_provider_mut().set_bpm(Some(request.bpm));
        let never_cancelled = || false;
        let result = renderer.render_output_preview_with_effects_and_transitions_cancellable(
            &request.video,
            video::VideoEffectRenderContext {
                clip_runtime: &request.clip_runtime,
                project_render_epoch: request.project_render_epoch,
            },
            &request.transition_runtime,
            request.output_id,
            request.width,
            request.height,
            &never_cancelled,
        );
        match result {
            Ok(frame) => write_frame(&mut output, &frame)?,
            Err(error) => {
                write_error(&mut output, format!("Recording renderer failed: {error:?}"))?
            }
        }
    }
    Ok(())
}

fn renderer_response_reader(
    mut stdout: std::process::ChildStdout,
    sender: mpsc::Sender<Result<video::VideoFrame, String>>,
) {
    loop {
        let payload = match read_payload(&mut stdout) {
            Ok(Some(payload)) => payload,
            Ok(None) | Err(_) => return,
        };
        let result = decode_response(&payload);
        if sender.send(result).is_err() {
            return;
        }
    }
}

fn write_payload(writer: &mut impl Write, payload: &[u8]) -> io::Result<()> {
    let length = u32::try_from(payload.len())
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "worker payload is too large"))?;
    writer.write_all(&length.to_le_bytes())?;
    writer.write_all(payload)?;
    writer.flush()
}

/// External capture frames are already opaque bytes. Keep the descriptive
/// render request JSON, but carry those bytes in one binary trailer instead of
/// expanding every pixel into a JSON number array on every recording frame.
fn encode_render_request(request: &RecordingRenderRequest) -> Result<Vec<u8>, String> {
    let mut frame_bytes = 0usize;
    let mut injected_frames = Vec::with_capacity(request.injected_frames.len());
    for frame in &request.injected_frames {
        let data_len = u32::try_from(frame.data.len())
            .map_err(|_| "Recording renderer frame length overflow".to_string())?;
        frame_bytes = frame_bytes
            .checked_add(frame.data.len())
            .ok_or_else(|| "Recording renderer request length overflow".to_string())?;
        injected_frames.push(RecordingInjectedFrameHeader {
            layer_id: frame.layer_id,
            width: frame.width,
            height: frame.height,
            pts_ms: frame.pts_ms,
            duration_ms: frame.duration_ms,
            format: frame.format,
            data_len,
        });
    }
    let header = serde_json::to_vec(&RecordingRenderRequestHeader {
        video: &request.video,
        clip_runtime: &request.clip_runtime,
        transition_runtime: &request.transition_runtime,
        project_render_epoch: request.project_render_epoch,
        bpm: request.bpm,
        output_id: request.output_id,
        width: request.width,
        height: request.height,
        injected_frames,
    })
    .map_err(|error| format!("Recording renderer request encode failed: {error}"))?;
    let total = REQUEST_MAGIC
        .len()
        .checked_add(4)
        .and_then(|length| length.checked_add(header.len()))
        .and_then(|length| length.checked_add(frame_bytes))
        .ok_or_else(|| "Recording renderer request length overflow".to_string())?;
    if total > MAX_WORKER_REQUEST_BYTES {
        return Err("Recording renderer request exceeded 128 MiB".to_string());
    }
    let header_len = u32::try_from(header.len())
        .map_err(|_| "Recording renderer request header is too large".to_string())?;
    let mut payload = Vec::with_capacity(total);
    payload.extend_from_slice(REQUEST_MAGIC);
    payload.extend_from_slice(&header_len.to_le_bytes());
    payload.extend_from_slice(&header);
    for frame in &request.injected_frames {
        payload.extend_from_slice(&frame.data);
    }
    Ok(payload)
}

fn decode_render_request(payload: &[u8]) -> Result<RecordingRenderRequest, String> {
    if payload.len() < REQUEST_MAGIC.len() + 4 || &payload[..REQUEST_MAGIC.len()] != REQUEST_MAGIC {
        return Err("Recording renderer request envelope is invalid".to_string());
    }
    let header_len = u32::from_le_bytes(
        payload[REQUEST_MAGIC.len()..REQUEST_MAGIC.len() + 4]
            .try_into()
            .map_err(|_| "Recording renderer request header length is invalid".to_string())?,
    ) as usize;
    let header_start = REQUEST_MAGIC.len() + 4;
    let header_end = header_start
        .checked_add(header_len)
        .ok_or_else(|| "Recording renderer request header length overflow".to_string())?;
    let header = payload
        .get(header_start..header_end)
        .ok_or_else(|| "Recording renderer request header is truncated".to_string())?;
    let header: RecordingRenderRequestHeaderOwned = serde_json::from_slice(header)
        .map_err(|error| format!("Recording renderer request header is invalid: {error}"))?;
    let mut offset = header_end;
    let mut injected_frames = Vec::with_capacity(header.injected_frames.len());
    for frame in header.injected_frames {
        let data_end = offset
            .checked_add(frame.data_len as usize)
            .ok_or_else(|| "Recording renderer frame length overflow".to_string())?;
        let data = payload
            .get(offset..data_end)
            .ok_or_else(|| "Recording renderer frame payload is truncated".to_string())?
            .to_vec();
        offset = data_end;
        injected_frames.push(video::VideoFrame {
            layer_id: frame.layer_id,
            width: frame.width,
            height: frame.height,
            pts_ms: frame.pts_ms,
            duration_ms: frame.duration_ms,
            format: frame.format,
            data,
        });
    }
    if offset != payload.len() {
        return Err("Recording renderer request has trailing bytes".to_string());
    }
    Ok(RecordingRenderRequest {
        video: header.video,
        clip_runtime: header.clip_runtime,
        transition_runtime: header.transition_runtime,
        project_render_epoch: header.project_render_epoch,
        bpm: header.bpm,
        output_id: header.output_id,
        width: header.width,
        height: header.height,
        injected_frames,
    })
}

fn read_payload(reader: &mut impl Read) -> io::Result<Option<Vec<u8>>> {
    let mut length = [0_u8; 4];
    match reader.read_exact(&mut length) {
        Ok(()) => {}
        Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(error) => return Err(error),
    }
    let length = u32::from_le_bytes(length) as usize;
    if length > MAX_WORKER_REQUEST_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "worker payload exceeds the protocol bound",
        ));
    }
    let mut payload = vec![0_u8; length];
    reader.read_exact(&mut payload)?;
    Ok(Some(payload))
}

fn write_frame(writer: &mut impl Write, frame: &video::VideoFrame) -> Result<(), String> {
    if frame.data.len() > MAX_FRAME_BYTES {
        return Err("Recording renderer response frame is too large".to_string());
    }
    let format = match frame.format {
        video::VideoPixelFormat::Rgba8 => 0,
        video::VideoPixelFormat::Bgra8 => 1,
        video::VideoPixelFormat::Dxt1 => 2,
        video::VideoPixelFormat::Dxt5 => 3,
        video::VideoPixelFormat::YcoCgDxt5 => 4,
        video::VideoPixelFormat::Bc7 => 5,
    };
    let mut payload = Vec::with_capacity(1 + 4 + 4 + 8 + 8 + 1 + 4 + frame.data.len());
    payload.push(0);
    payload.extend_from_slice(&frame.width.to_le_bytes());
    payload.extend_from_slice(&frame.height.to_le_bytes());
    payload.extend_from_slice(&frame.pts_ms.to_le_bytes());
    payload.extend_from_slice(&frame.duration_ms.to_le_bytes());
    payload.push(format);
    payload.extend_from_slice(
        &u32::try_from(frame.data.len())
            .map_err(|_| "Recording renderer response length overflow".to_string())?
            .to_le_bytes(),
    );
    payload.extend_from_slice(&frame.data);
    write_payload(writer, &payload)
        .map_err(|error| format!("Recording renderer response failed: {error}"))
}

fn write_error(writer: &mut impl Write, error: impl Into<String>) -> Result<(), String> {
    let error = error.into();
    let error = error.chars().take(2048).collect::<String>();
    let mut payload = Vec::with_capacity(1 + error.len());
    payload.push(1);
    payload.extend_from_slice(error.as_bytes());
    write_payload(writer, &payload)
        .map_err(|io| format!("Recording renderer error response failed: {io}"))
}

fn decode_response(payload: &[u8]) -> Result<video::VideoFrame, String> {
    if payload.is_empty() {
        return Err("Recording renderer returned an empty response".to_string());
    }
    if payload[0] == 1 {
        return Err(String::from_utf8_lossy(&payload[1..])
            .chars()
            .take(2048)
            .collect());
    }
    if payload[0] != 0 || payload.len() < 1 + 4 + 4 + 8 + 8 + 1 + 4 {
        return Err("Recording renderer response header is invalid".to_string());
    }
    let mut offset = 1;
    let read_u32 = |payload: &[u8], offset: &mut usize| -> Option<u32> {
        let end = offset.checked_add(4)?;
        let bytes = payload.get(*offset..end)?;
        *offset = end;
        Some(u32::from_le_bytes(bytes.try_into().ok()?))
    };
    let read_u64 = |payload: &[u8], offset: &mut usize| -> Option<u64> {
        let end = offset.checked_add(8)?;
        let bytes = payload.get(*offset..end)?;
        *offset = end;
        Some(u64::from_le_bytes(bytes.try_into().ok()?))
    };
    let width = read_u32(payload, &mut offset).ok_or("Recording renderer width is invalid")?;
    let height = read_u32(payload, &mut offset).ok_or("Recording renderer height is invalid")?;
    let pts_ms = read_u64(payload, &mut offset).ok_or("Recording renderer PTS is invalid")?;
    let duration_ms =
        read_u64(payload, &mut offset).ok_or("Recording renderer duration is invalid")?;
    let format = match *payload
        .get(offset)
        .ok_or("Recording renderer format is invalid")?
    {
        0 => video::VideoPixelFormat::Rgba8,
        1 => video::VideoPixelFormat::Bgra8,
        2 => video::VideoPixelFormat::Dxt1,
        3 => video::VideoPixelFormat::Dxt5,
        4 => video::VideoPixelFormat::YcoCgDxt5,
        5 => video::VideoPixelFormat::Bc7,
        _ => return Err("Recording renderer format tag is invalid".to_string()),
    };
    offset += 1;
    let data_len =
        read_u32(payload, &mut offset).ok_or("Recording renderer data length is invalid")? as usize;
    let data_end = offset
        .checked_add(data_len)
        .ok_or("Recording renderer data length overflow")?;
    if data_len > MAX_FRAME_BYTES || data_end != payload.len() {
        return Err("Recording renderer frame payload is invalid".to_string());
    }
    let data = payload[offset..data_end].to_vec();
    if format == video::VideoPixelFormat::Rgba8
        && (width == 0 || height == 0 || width as usize * height as usize * 4 != data.len())
    {
        return Err("Recording renderer RGBA dimensions do not match payload".to_string());
    }
    Ok(video::VideoFrame {
        layer_id: 0,
        width,
        height,
        pts_ms,
        duration_ms,
        format,
        data,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_cli_requires_the_exact_single_argument() {
        assert!(worker_cli_requested(
            [OsString::from(WORKER_ARGUMENT)].into_iter()
        ));
        assert!(!worker_cli_requested(std::iter::empty()));
        assert!(!worker_cli_requested(
            [OsString::from(WORKER_ARGUMENT), OsString::from("--extra")].into_iter()
        ));
        assert!(!worker_cli_requested(
            [OsString::from("--syndocal-recording-renderer")].into_iter()
        ));
    }

    #[test]
    fn response_decoder_rejects_malformed_or_oversized_frames() {
        assert!(decode_response(&[]).is_err());
        assert!(decode_response(&[1]).is_err());
        let mut header = vec![0; 30];
        header[26..30].copy_from_slice(&(MAX_FRAME_BYTES as u32 + 1).to_le_bytes());
        assert!(decode_response(&header).is_err());
    }

    #[test]
    fn render_request_keeps_external_frame_bytes_binary() {
        let request = RecordingRenderRequest {
            video: protocol::VideoSnapshot::default(),
            clip_runtime: protocol::VideoClipRuntimeSnapshot::default(),
            transition_runtime: protocol::VideoLayerTransitionRuntimeSnapshot::default(),
            project_render_epoch: 4,
            bpm: 120.0,
            output_id: 2,
            width: 1,
            height: 1,
            injected_frames: vec![video::VideoFrame {
                layer_id: 7,
                width: 1,
                height: 1,
                pts_ms: 9,
                duration_ms: 33,
                format: video::VideoPixelFormat::Rgba8,
                data: vec![0, 17, 255, 255],
            }],
        };
        let encoded = encode_render_request(&request).unwrap();
        assert!(encoded.windows(4).any(|window| window == [0, 17, 255, 255]));
        assert!(!encoded.windows(4).any(|window| window == [48, 49, 55, 44]));
        assert_eq!(
            decode_render_request(&encoded).unwrap().injected_frames,
            request.injected_frames
        );
    }
}
