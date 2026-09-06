//! FFmpeg recording orchestration; UI/runtime ownership stays in the application adapter.
use super::{
    capture_video_output_preview_effect_snapshot, recording_artifact, AppVideoPreviewRenderer,
    VideoRecordingStatus,
};
use engine::EngineHandle;
use protocol::VideoOutputId;
use std::{
    env,
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

#[path = "video_recording_encoder.rs"]
pub(crate) mod encoder;

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RecordingAudioInput {
    pub(super) path: PathBuf,
    pub(super) position_ms: u64,
    pub(super) volume: f32,
    pub(super) speed: f32,
    pub(super) loop_enabled: bool,
}

pub(super) fn video_recording_ffmpeg_command(
    ffmpeg: impl AsRef<OsStr>,
    path: &Path,
    width: u32,
    height: u32,
    frame_rate: u32,
    audio_inputs: &[RecordingAudioInput],
) -> Command {
    let dimensions = format!("{width}x{height}");
    let rate = frame_rate.to_string();
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-loglevel", "error", "-y", "-f", "rawvideo"])
        .args([
            "-pix_fmt",
            "rgba",
            "-s",
            &dimensions,
            "-r",
            &rate,
            "-i",
            "pipe:0",
        ]);
    let audio_filter = configure_recording_audio_inputs(&mut command, audio_inputs);
    command.args(["-map", "0:v:0"]);
    if let Some(audio_filter) = audio_filter {
        command
            .args(["-filter_complex", &audio_filter, "-map", "[aout]"])
            .args(["-c:a", "aac", "-b:a", "192k", "-shortest"]);
    } else {
        command.arg("-an");
    }
    command
        .args([
            "-c:v", "libx264", "-preset", "veryfast", "-pix_fmt", "yuv420p",
        ])
        .args(["-f", "mp4"])
        .arg(path)
        .stdin(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

pub(super) struct VideoOutputRecordingContext {
    pub(super) engine: EngineHandle,
    pub(super) renderer: Arc<Mutex<AppVideoPreviewRenderer>>,
    pub(super) output_id: VideoOutputId,
    pub(super) path: PathBuf,
    pub(super) width: u32,
    pub(super) height: u32,
    pub(super) frame_rate: u32,
    pub(super) audio_inputs: Vec<RecordingAudioInput>,
    pub(super) stop: Arc<AtomicBool>,
    pub(super) status: Arc<Mutex<VideoRecordingStatus>>,
}

pub(super) fn run_video_output_recording(context: VideoOutputRecordingContext) {
    let VideoOutputRecordingContext {
        engine,
        renderer,
        output_id,
        path,
        width,
        height,
        frame_rate,
        audio_inputs,
        stop,
        status,
    } = context;
    let mut artifact = match recording_artifact::RecordingArtifact::reserve(&path) {
        Ok(artifact) => artifact,
        Err(error) => {
            finish_video_recording_status(&status, Some(error));
            return;
        }
    };
    let ffmpeg = env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let mut command = video_recording_ffmpeg_command(
        ffmpeg,
        artifact.path(),
        width,
        height,
        frame_rate,
        &audio_inputs,
    );
    let mut encoder = match encoder::RecordingEncoder::spawn(&mut command, Arc::clone(&stop)) {
        Ok(encoder) => encoder,
        Err(error) => {
            finish_failed_video_recording(&status, &mut artifact, error);
            return;
        }
    };
    let Some(mut stdin) = encoder.take_stdin() else {
        let error = encoder
            .fail("FFmpeg stdin was unavailable".into())
            .unwrap_err();
        finish_failed_video_recording(&status, &mut artifact, error);
        return;
    };
    let mut frames_written = 0_u64;
    let mut pipe_error = None;
    let frame_interval = Duration::from_secs_f64(1.0 / frame_rate as f64);
    let mut next_frame_at = Instant::now();
    while !stop.load(Ordering::Acquire) {
        let output_preview = capture_video_output_preview_effect_snapshot(&engine);
        let frame = renderer.lock().ok().and_then(|mut renderer| {
            renderer
                .frame_provider_mut()
                .set_bpm(Some(output_preview.snapshot.clock.bpm));
            renderer
                .render_output_preview_with_effects_and_transitions(
                    &output_preview.snapshot.video,
                    output_preview.render_context(),
                    &output_preview.snapshot.video_transition_runtime,
                    output_id,
                    width,
                    height,
                )
                .ok()
        });
        if stop.load(Ordering::Acquire) {
            break;
        }
        match frame {
            Some(frame)
                if frame.format == video::VideoPixelFormat::Rgba8
                    && frame.width == width
                    && frame.height == height
                    && frame.data.len() == width as usize * height as usize * 4 =>
            {
                if let Err(error) = stdin.write_all(&frame.data) {
                    pipe_error = Some(format!("FFmpeg pipe failed: {error}"));
                    break;
                }
                frames_written = frames_written.saturating_add(1);
                if let Ok(mut current) = status.lock() {
                    current.frames_written = current.frames_written.saturating_add(1);
                }
            }
            _ => {
                if let Ok(mut current) = status.lock() {
                    current.dropped_frames = current.dropped_frames.saturating_add(1);
                }
            }
        }
        next_frame_at += frame_interval;
        let now = Instant::now();
        if next_frame_at > now {
            std::thread::sleep(next_frame_at - now);
        } else if now.duration_since(next_frame_at) > frame_interval {
            let missed = (now.duration_since(next_frame_at).as_secs_f64()
                / frame_interval.as_secs_f64()) as u64;
            if let Ok(mut current) = status.lock() {
                current.dropped_frames = current.dropped_frames.saturating_add(missed);
            }
            next_frame_at = now;
        }
    }
    drop(stdin);
    let encoded = match pipe_error {
        Some(error) => encoder.fail(error),
        None => encoder.finish(),
    };
    let result = encoded.and_then(|_| artifact.publish(frames_written));
    match result {
        Ok(()) => finish_video_recording_status(&status, None),
        Err(error) => finish_failed_video_recording(&status, &mut artifact, error),
    }
}

fn finish_failed_video_recording(
    status: &Mutex<VideoRecordingStatus>,
    artifact: &mut recording_artifact::RecordingArtifact,
    error: String,
) {
    let error = match artifact.discard() {
        Ok(()) => error,
        Err(cleanup) => format!("{error}; cleanup failed: {cleanup}"),
    };
    finish_video_recording_status(status, Some(error));
}

pub(super) fn configure_recording_audio_inputs(
    command: &mut Command,
    inputs: &[RecordingAudioInput],
) -> Option<String> {
    if inputs.is_empty() {
        return None;
    }
    let mut chains = Vec::with_capacity(inputs.len() + 1);
    for (index, input) in inputs.iter().enumerate() {
        if input.loop_enabled {
            command.args(["-stream_loop", "-1"]);
        }
        command
            .args(["-ss", &format!("{:.6}", input.position_ms as f64 / 1_000.0)])
            .arg("-i")
            .arg(&input.path);
        let output_label = if inputs.len() == 1 {
            "aout".to_string()
        } else {
            format!("a{index}")
        };
        chains.push(format!(
            "[{}:a]volume={:.6},{},apad[{}]",
            index + 1,
            input.volume,
            ffmpeg_atempo_filter(input.speed),
            output_label
        ));
    }
    if inputs.len() > 1 {
        let labels = (0..inputs.len())
            .map(|index| format!("[a{index}]"))
            .collect::<String>();
        chains.push(format!(
            "{labels}amix=inputs={}:duration=longest:dropout_transition=0[aout]",
            inputs.len()
        ));
    }
    Some(chains.join(";"))
}

pub(super) fn ffmpeg_atempo_filter(speed: f32) -> String {
    let mut remaining = if speed.is_finite() {
        speed.abs().clamp(0.25, 4.0)
    } else {
        1.0
    };
    let mut filters = Vec::new();
    while remaining < 0.5 - f32::EPSILON {
        filters.push("atempo=0.5".to_string());
        remaining /= 0.5;
    }
    while remaining > 2.0 + f32::EPSILON {
        filters.push("atempo=2".to_string());
        remaining /= 2.0;
    }
    filters.push(format!("atempo={remaining:.6}"));
    filters.join(",")
}

pub(super) fn finish_video_recording_status(
    status: &Mutex<VideoRecordingStatus>,
    error: Option<String>,
) {
    if let Ok(mut current) = status.lock() {
        current.active = false;
        current.last_error = error;
    }
}
