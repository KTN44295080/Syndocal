use engine::{EngineCommand, EngineHandle};
use protocol::{EffectPreset, ProjectFile, VideoEffectTarget, VideoParam};
use serde_json::json;
use std::{
    env, fs,
    path::PathBuf,
    process::ExitCode,
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use video::{NullVideoDecoder, VideoPreviewRenderer, VideoRuntimeConfig};

const PROJECT_JSON: &str = include_str!("../../../samples/phase1-mini-show.sdc");
const PULSE_EFFECT_JSON: &str = include_str!("../../../samples/front-dimmer-pulse.effect");
const WAVE_EFFECT_JSON: &str = include_str!("../../../samples/front-dimmer-wave.effect");
const FRAME_RATE: u64 = 30;
const LOOP_MS: u64 = 4_000;
const TICK_JITTER_P99_BUDGET_US: u64 = 1_000;
const COMMAND_QUEUE_P99_BUDGET_US: u64 = 1_000;
const COMMAND_TO_DMX_P99_BUDGET_US: u64 = 5_000;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("Syndocal soak failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let (duration, report_path) = parse_args()?;
    let project: ProjectFile = serde_json::from_str(PROJECT_JSON).map_err(|e| e.to_string())?;
    let output = project.snapshot.output.clone();
    let engine = EngineHandle::start(output);
    engine
        .load_project_snapshot(project.snapshot)
        .map_err(|e| e.to_string())?;

    add_effect(&engine, PULSE_EFFECT_JSON, false)?;
    add_effect(&engine, WAVE_EFFECT_JSON, true)?;
    engine
        .send(EngineCommand::TriggerCue(1))
        .map_err(|e| e.to_string())?;
    engine
        .send(EngineCommand::SetTimelinePlaying(true))
        .map_err(|e| e.to_string())?;

    wait_for_loaded_state(&engine)?;
    engine
        .send(EngineCommand::ResetTelemetry)
        .map_err(|e| e.to_string())?;
    thread::sleep(Duration::from_millis(50));
    let mut renderer =
        VideoPreviewRenderer::with_decoder(VideoRuntimeConfig::default(), NullVideoDecoder);
    let frame_interval = Duration::from_nanos(1_000_000_000 / FRAME_RATE);
    let started_at = Instant::now();
    let mut next_frame = started_at;
    let mut next_loop = started_at + Duration::from_millis(LOOP_MS);
    let mut rendered_frames = 0_u64;
    let mut nonblank_frames = 0_u64;
    let mut dropped_frames = 0_u64;
    let mut live_audio_updates = 0_u64;
    let mut render_errors = Vec::new();

    while started_at.elapsed() < duration {
        let now = Instant::now();
        if now < next_frame {
            thread::sleep(next_frame - now);
        } else if now.duration_since(next_frame) >= frame_interval {
            let missed = now.duration_since(next_frame).as_nanos() / frame_interval.as_nanos();
            dropped_frames = dropped_frames.saturating_add(missed as u64);
            next_frame += frame_interval.saturating_mul(missed as u32);
        }

        if Instant::now() >= next_loop {
            engine
                .send(EngineCommand::SeekTimeline(0))
                .map_err(|e| e.to_string())?;
            engine
                .send(EngineCommand::TriggerCue(1))
                .map_err(|e| e.to_string())?;
            next_loop += Duration::from_millis(LOOP_MS);
        }

        let live_phase = (live_audio_updates % 120) as f32 / 120.0;
        engine
            .send(EngineCommand::SetLiveAudioSpectrum(Some(
                protocol::AudioSpectrumPoint {
                    time_ms: 0,
                    bass: (std::f32::consts::TAU * live_phase).sin().abs(),
                    mid: (std::f32::consts::TAU * (live_phase + 0.333)).sin().abs(),
                    high: (std::f32::consts::TAU * (live_phase + 0.666)).sin().abs(),
                },
            )))
            .map_err(|e| e.to_string())?;
        live_audio_updates = live_audio_updates.saturating_add(1);

        let snapshot = engine.snapshot();
        match renderer.render(&snapshot.video, 320, 180) {
            Ok(frame) => {
                rendered_frames += 1;
                if frame.data.iter().any(|value| *value != 0) {
                    nonblank_frames += 1;
                }
            }
            Err(error) => {
                if render_errors.len() < 8 {
                    render_errors.push(format!("{error:?}"));
                }
            }
        }
        next_frame += frame_interval;
    }

    let snapshot = engine.snapshot();
    let telemetry = snapshot.telemetry;
    let expected_frames = duration.as_secs_f64() * FRAME_RATE as f64;
    let frame_ratio = if expected_frames > 0.0 {
        rendered_frames as f64 / expected_frames
    } else {
        1.0
    };
    let telemetry_budget_passed = telemetry.tick_jitter_p99_us <= TICK_JITTER_P99_BUDGET_US
        && telemetry.command_queue_latency_p99_us <= COMMAND_QUEUE_P99_BUDGET_US
        && telemetry.command_to_dmx_tick_latency_p99_us <= COMMAND_TO_DMX_P99_BUDGET_US
        && telemetry.queue_push_failure_count == 0
        && telemetry.command_drain_limit_hit_count == 0
        && telemetry.total_dmx_send_failure_count == 0
        && telemetry.last_error.is_none();
    let passed = render_errors.is_empty()
        && nonblank_frames == rendered_frames
        && frame_ratio >= 0.98
        && telemetry_budget_passed;
    let captured_at_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let report = json!({
        "app": "Syndocal",
        "test": "M5 one-hour soak",
        "passed": passed,
        "captured_at_unix_ms": captured_at_unix_ms,
        "duration_seconds": started_at.elapsed().as_secs_f64(),
        "target_frame_rate": FRAME_RATE,
        "rendered_frames": rendered_frames,
        "nonblank_frames": nonblank_frames,
        "dropped_frames": dropped_frames,
        "live_audio_updates": live_audio_updates,
        "frame_ratio": frame_ratio,
        "render_errors": render_errors,
        "telemetry_budget": {
            "passed": telemetry_budget_passed,
            "tick_jitter_p99_budget_us": TICK_JITTER_P99_BUDGET_US,
            "command_queue_p99_budget_us": COMMAND_QUEUE_P99_BUDGET_US,
            "command_to_dmx_p99_budget_us": COMMAND_TO_DMX_P99_BUDGET_US,
        },
        "fixtures": snapshot.fixtures.len(),
        "effects": snapshot.effects.len(),
        "video_layers": snapshot.video.layers.len(),
        "telemetry": telemetry,
    });
    if let Some(parent) = report_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    fs::write(
        &report_path,
        serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!("{}", report_path.display());

    if passed {
        Ok(())
    } else {
        Err("one or more soak gates failed".to_string())
    }
}

fn add_effect(engine: &EngineHandle, json: &str, target_video: bool) -> Result<(), String> {
    let preset: EffectPreset = serde_json::from_str(json).map_err(|e| e.to_string())?;
    let video_target = VideoEffectTarget {
        layer_ids: vec![1],
        param: VideoParam::Opacity,
        low: 0.25,
        high: 1.0,
        position: None,
    };
    let effect_id = engine.allocate_effect_id();
    let command = if let Some(mut request) = preset.lfo {
        if target_video {
            request.video_targets.push(video_target);
        }
        EngineCommand::AddLfoEffect { effect_id, request }
    } else if let Some(mut request) = preset.position_wave {
        if target_video {
            request.video_targets.push(VideoEffectTarget {
                layer_ids: vec![1],
                param: VideoParam::Opacity,
                low: 0.25,
                high: 1.0,
                position: None,
            });
        }
        EngineCommand::AddPositionWaveEffect { effect_id, request }
    } else {
        return Err("soak effect preset has no effect body".to_string());
    };
    engine.send(command).map_err(|e| e.to_string())
}

fn wait_for_loaded_state(engine: &EngineHandle) -> Result<(), String> {
    let deadline = Instant::now() + Duration::from_secs(2);
    while Instant::now() < deadline {
        let snapshot = engine.snapshot();
        if snapshot.fixtures.len() == 1
            && snapshot.effects.len() == 2
            && snapshot.video.layers.len() == 1
        {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(10));
    }
    Err("engine did not load the soak project within two seconds".to_string())
}

fn parse_args() -> Result<(Duration, PathBuf), String> {
    let mut duration_seconds = 3_600_u64;
    let mut report_path = PathBuf::from("target/qa/m5-soak.json");
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--duration-seconds" => {
                duration_seconds = args
                    .next()
                    .ok_or_else(|| "--duration-seconds requires a value".to_string())?
                    .parse::<u64>()
                    .map_err(|_| "duration must be an integer".to_string())?;
            }
            "--report" => {
                report_path = PathBuf::from(
                    args.next()
                        .ok_or_else(|| "--report requires a path".to_string())?,
                );
            }
            _ => return Err(format!("unknown argument '{arg}'")),
        }
    }
    if duration_seconds == 0 {
        return Err("duration must be greater than zero".to_string());
    }
    Ok((Duration::from_secs(duration_seconds), report_path))
}
