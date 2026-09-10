use super::*;

#[test]
fn video_clip_slot_b3_direct_result_bookkeeping_recovers_poison_after_ack() {
    let created_slot_ids = Mutex::new(vec![VideoClipSlotId(71), VideoClipSlotId(72)]);
    let imported_asset_ids = Mutex::new(vec![81, 82]);
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _guard = created_slot_ids.lock().expect("poison direct created IDs");
        panic!("inject post-ACK direct result-cell poison");
    }));
    let (created, imported) =
        video_clip_slot_direct_result_ids(&created_slot_ids, &imported_asset_ids);
    assert_eq!(created, vec![VideoClipSlotId(71), VideoClipSlotId(72)]);
    assert_eq!(imported, vec![81, 82]);
}

fn generate_long_sync_audio(ffmpeg: &OsStr, path: &Path, duration_seconds: u32) {
    let middle_duration = duration_seconds as f64 - 0.2;
    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-loglevel", "error", "-y"])
        .args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000:duration=0.1",
        ])
        .args([
            "-f",
            "lavfi",
            "-i",
            &format!("anullsrc=r=48000:cl=mono:d={middle_duration:.6}"),
        ])
        .args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=1000:sample_rate=48000:duration=0.1",
        ])
        .args([
            "-filter_complex",
            "[0:a][1:a][2:a]concat=n=3:v=0:a=1[aout]",
            "-map",
            "[aout]",
            "-c:a",
            "aac",
            "-b:a",
            "96k",
        ])
        .arg(path)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "failed to generate long sync audio: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn decode_recording_window(
    ffmpeg: &OsStr,
    path: &Path,
    stream: &str,
    seek_seconds: f64,
    duration_seconds: f64,
    output_args: &[&str],
) -> Vec<u8> {
    let mut command = Command::new(ffmpeg);
    command
        .args(["-hide_banner", "-loglevel", "error", "-ss"])
        .arg(format!("{seek_seconds:.6}"))
        .arg("-i")
        .arg(path)
        .args(["-map", stream, "-t"])
        .arg(format!("{duration_seconds:.6}"))
        .args(output_args)
        .arg("pipe:1");
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "failed to decode recording window: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    output.stdout
}

fn maximum_pcm_s16_amplitude(bytes: &[u8]) -> i16 {
    bytes
        .chunks_exact(2)
        .map(|sample| i16::from_le_bytes([sample[0], sample[1]]).unsigned_abs())
        .max()
        .unwrap_or_default()
        .min(i16::MAX as u16) as i16
}

fn average_luma(bytes: &[u8]) -> f64 {
    if bytes.is_empty() {
        return 0.0;
    }
    bytes.iter().map(|value| *value as u64).sum::<u64>() as f64 / bytes.len() as f64
}

fn write_test_tone_wav(path: &Path) {
    let sample_rate = 48_000_u32;
    let sample_count = sample_rate;
    let data_bytes = sample_count * 2;
    let mut wav = Vec::with_capacity((44 + data_bytes) as usize);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(36 + data_bytes).to_le_bytes());
    wav.extend_from_slice(b"WAVEfmt ");
    wav.extend_from_slice(&16_u32.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&1_u16.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    wav.extend_from_slice(&2_u16.to_le_bytes());
    wav.extend_from_slice(&16_u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&data_bytes.to_le_bytes());
    for index in 0..sample_count {
        let phase = index as f32 * 440.0 * std::f32::consts::TAU / sample_rate as f32;
        let sample = (phase.sin() * i16::MAX as f32 * 0.2) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
    }
    fs::write(path, wav).unwrap();
}

#[test]
fn recording_file_name_and_terminal_status_are_safe() {
    assert_eq!(sanitize_file_name_component("Main / LED:1"), "Main___LED_1");
    assert_eq!(sanitize_file_name_component("***"), "syndocal-output");
    let status = Mutex::new(VideoRecordingStatus {
        active: true,
        frames_written: 42,
        ..VideoRecordingStatus::default()
    });

    finish_video_recording_status(&status, Some("encoder failed".to_string()));

    let status = status.lock().unwrap();
    assert!(!status.active);
    assert_eq!(status.frames_written, 42);
    assert_eq!(status.last_error.as_deref(), Some("encoder failed"));
}

#[test]
fn recording_audio_filter_supports_seek_loop_gain_speed_and_mix() {
    assert_eq!(ffmpeg_atempo_filter(0.25), "atempo=0.5,atempo=0.500000");
    assert_eq!(ffmpeg_atempo_filter(4.0), "atempo=2,atempo=2.000000");
    let inputs = vec![
        RecordingAudioInput {
            path: PathBuf::from("a.mp4"),
            position_ms: 1_250,
            volume: 0.8,
            speed: 1.0,
            loop_enabled: true,
        },
        RecordingAudioInput {
            path: PathBuf::from("b.mov"),
            position_ms: 500,
            volume: 0.4,
            speed: 2.0,
            loop_enabled: false,
        },
    ];
    let mut command = Command::new("ffmpeg");
    let filter = configure_recording_audio_inputs(&mut command, &inputs).unwrap();
    let args = command
        .get_args()
        .map(|arg| arg.to_string_lossy().to_string())
        .collect::<Vec<_>>();

    assert!(args.windows(2).any(|pair| pair == ["-stream_loop", "-1"]));
    assert!(args.windows(2).any(|pair| pair == ["-ss", "1.250000"]));
    assert!(filter.contains("[1:a]volume=0.800000,atempo=1.000000,apad[a0]"));
    assert!(filter.contains("[2:a]volume=0.400000,atempo=2.000000,apad[a1]"));
    assert!(filter.contains("[a0][a1]amix=inputs=2:duration=longest"));
}

#[test]
fn recording_audio_inputs_select_only_playing_monitored_layers_on_output() {
    let mut snapshot = EngineSnapshot::default();
    snapshot.video.layers = vec![protocol::VideoLayerSummary {
        id: 7,
        label: "Program".to_string(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("program.mp4".to_string()),
            name: None,
            codec: Some("H264".to_string()),
            metadata: None,
        },
        media_asset_id: None,
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState {
            enabled: true,
            playing: true,
            position_ms: 2_500,
            speed: 1.5,
            loop_enabled: true,
            ..VideoLayerState::default()
        },
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    }];
    snapshot.video.compositions = vec![CompositionSummary {
        id: 3,
        label: "Program".to_string(),
        layer_ids: vec![7],
        timeline_layer_ids: Vec::new(),
        output_ids: vec![9],
    }];
    snapshot.video.outputs = vec![VideoOutputSummary {
        id: 9,
        label: "Record".to_string(),
        kind: VideoOutputKind::Display,
        enabled: true,
        composition_id: 3,
        fullscreen: false,
        monitor_id: None,
        monitor_identity: None,
        width: 1280,
        height: 720,
        endpoint_name: None,
        opacity: 1.0,
        blackout: false,
        mapping: VideoOutputMapping::default(),
    }];
    let sources = HashMap::from([(
        7,
        MediaAudioSourceConfig {
            path: PathBuf::from("program.mp4"),
            volume: 0.75,
            requested_device_name: None,
        },
    )]);

    let inputs = recording_audio_inputs(&snapshot.video, 9, &sources);

    assert_eq!(inputs.len(), 1);
    assert_eq!(inputs[0].position_ms, 2_500);
    assert_eq!(inputs[0].speed, 1.5);
    assert!(inputs[0].loop_enabled);
}

#[test]
#[ignore = "requires FFmpeg and FFprobe with H.264/AAC support"]
fn recording_command_writes_a_real_video_and_audio_mp4() {
    let suffix = format!("{}-{}", std::process::id(), current_unix_ms());
    let audio_path = std::env::temp_dir().join(format!("syndocal-recording-{suffix}.wav"));
    let output_path = std::env::temp_dir().join(format!("syndocal-recording-{suffix}.mp4"));
    write_test_tone_wav(&audio_path);
    let sentinel = b"previous complete output must survive until publication";
    fs::write(&output_path, sentinel).unwrap();
    let mut artifact = recording_artifact::RecordingArtifact::reserve(&output_path).unwrap();
    let ffmpeg = env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let mut command = video_recording_ffmpeg_command(
        ffmpeg,
        artifact.path(),
        16,
        16,
        30,
        &[RecordingAudioInput {
            path: audio_path.clone(),
            position_ms: 0,
            volume: 0.5,
            speed: 1.0,
            loop_enabled: false,
        }],
    );
    let stop = Arc::new(AtomicBool::new(false));
    let mut encoder =
        video_recording::encoder::RecordingEncoder::spawn(&mut command, Arc::clone(&stop)).unwrap();
    let mut stdin = encoder.take_stdin().unwrap();
    let mut frame = vec![0_u8; 16 * 16 * 4];
    for pixel in frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[220, 30, 10, 255]);
    }
    for _ in 0..30 {
        stdin.write_all(&frame).unwrap();
    }
    stop.store(true, Ordering::Release);
    drop(stdin);
    let encoded = encoder.finish();
    assert_eq!(fs::read(&output_path).unwrap(), sentinel);
    encoded.expect("graceful Stop must finish a valid recording without force termination");
    artifact.publish(30).unwrap();
    assert!(!artifact.path().exists());

    let ffprobe = env::var_os("SYNDOCAL_FFPROBE").unwrap_or_else(|| "ffprobe".into());
    let probed = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type",
            "-of",
            "csv=p=0",
        ])
        .arg(&output_path)
        .output()
        .unwrap();
    let streams = String::from_utf8_lossy(&probed.stdout);
    // A stream entry alone does not prove that graceful shutdown wrote a
    // complete, decodable trailer and all submitted video frames.
    let ffmpeg = env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let decoded = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(&output_path)
        .args([
            "-map", "0:v:0", "-f", "rawvideo", "-pix_fmt", "rgba", "pipe:1",
        ])
        .output()
        .unwrap();
    let _ = fs::remove_file(&audio_path);
    let _ = fs::remove_file(&output_path);

    assert!(probed.status.success());
    assert!(streams.lines().any(|line| line.trim() == "video"));
    assert!(streams.lines().any(|line| line.trim() == "audio"));
    assert!(
        decoded.status.success(),
        "{}",
        String::from_utf8_lossy(&decoded.stderr)
    );
    assert_eq!(decoded.stdout.len(), 30 * 16 * 16 * 4);
}

#[test]
#[ignore = "30-minute production recording A/V sync acceptance test"]
fn recording_command_keeps_long_av_sync_with_first_and_last_flash_clicks() {
    const DURATION_SECONDS: u32 = 30 * 60;
    const FRAME_RATE: u32 = 30;
    const WIDTH: usize = 16;
    const HEIGHT: usize = 16;
    let suffix = format!("{}-{}", std::process::id(), current_unix_ms());
    let artifact_directory = env::var_os("SYNDOCAL_LONG_AV_QA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    fs::create_dir_all(&artifact_directory).unwrap();
    let audio_path = artifact_directory.join(format!("long-av-sync-source-{suffix}.m4a"));
    let output_path = artifact_directory.join("long-av-sync-30m.mp4");
    let report_path = artifact_directory.join("long-av-sync-30m.json");
    let ffmpeg = env::var_os("SYNDOCAL_FFMPEG").unwrap_or_else(|| "ffmpeg".into());
    let ffprobe = env::var_os("SYNDOCAL_FFPROBE").unwrap_or_else(|| "ffprobe".into());

    generate_long_sync_audio(&ffmpeg, &audio_path, DURATION_SECONDS);
    let mut command = video_recording_ffmpeg_command(
        &ffmpeg,
        &output_path,
        WIDTH as u32,
        HEIGHT as u32,
        FRAME_RATE,
        &[RecordingAudioInput {
            path: audio_path.clone(),
            position_ms: 0,
            volume: 1.0,
            speed: 1.0,
            loop_enabled: false,
        }],
    );
    let mut child = command.spawn().unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let black_frame = vec![0_u8; WIDTH * HEIGHT * 4];
    let mut white_frame = vec![0_u8; WIDTH * HEIGHT * 4];
    for pixel in white_frame.chunks_exact_mut(4) {
        pixel.copy_from_slice(&[255, 255, 255, 255]);
    }
    let frame_count = DURATION_SECONDS as u64 * FRAME_RATE as u64;
    for frame_index in 0..frame_count {
        let flash = frame_index < 3 || frame_index >= frame_count - 3;
        stdin
            .write_all(if flash { &white_frame } else { &black_frame })
            .unwrap();
    }
    drop(stdin);
    let encoded = child.wait_with_output().unwrap();
    assert!(
        encoded.status.success(),
        "FFmpeg failed: {}",
        String::from_utf8_lossy(&encoded.stderr)
    );

    let probe = Command::new(&ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "stream=codec_type,start_time,duration,nb_frames",
            "-of",
            "json",
        ])
        .arg(&output_path)
        .output()
        .unwrap();
    assert!(
        probe.status.success(),
        "FFprobe failed: {}",
        String::from_utf8_lossy(&probe.stderr)
    );
    let probe_json: serde_json::Value = serde_json::from_slice(&probe.stdout).unwrap();
    let timings = |codec_type: &str| {
        let stream = probe_json["streams"]
            .as_array()
            .unwrap()
            .iter()
            .find(|stream| stream["codec_type"] == codec_type)
            .unwrap();
        let number = |key: &str| stream[key].as_str().unwrap().parse::<f64>().unwrap();
        (number("start_time"), number("duration"))
    };
    let (video_start, video_duration) = timings("video");
    let (audio_start, audio_duration) = timings("audio");
    let start_drift_ms = (video_start - audio_start).abs() * 1_000.0;
    let end_drift_ms =
        ((video_start + video_duration) - (audio_start + audio_duration)).abs() * 1_000.0;

    let first_video = decode_recording_window(
        &ffmpeg,
        &output_path,
        "0:v:0",
        0.0,
        1.0 / FRAME_RATE as f64,
        &["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray"],
    );
    let middle_video = decode_recording_window(
        &ffmpeg,
        &output_path,
        "0:v:0",
        DURATION_SECONDS as f64 / 2.0,
        1.0 / FRAME_RATE as f64,
        &["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray"],
    );
    let last_video = decode_recording_window(
        &ffmpeg,
        &output_path,
        "0:v:0",
        DURATION_SECONDS as f64 - 0.1,
        0.1,
        &["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "gray"],
    );
    let audio_args = ["-f", "s16le", "-ac", "1", "-ar", "48000"];
    let first_audio =
        decode_recording_window(&ffmpeg, &output_path, "0:a:0", 0.0, 0.1, &audio_args);
    let middle_audio = decode_recording_window(
        &ffmpeg,
        &output_path,
        "0:a:0",
        DURATION_SECONDS as f64 / 2.0,
        0.1,
        &audio_args,
    );
    let last_audio = decode_recording_window(
        &ffmpeg,
        &output_path,
        "0:a:0",
        DURATION_SECONDS as f64 - 0.1,
        0.1,
        &audio_args,
    );
    let first_luma = average_luma(&first_video);
    let middle_luma = average_luma(&middle_video);
    let last_luma = average_luma(&last_video);
    let first_audio_peak = maximum_pcm_s16_amplitude(&first_audio);
    let middle_audio_peak = maximum_pcm_s16_amplitude(&middle_audio);
    let last_audio_peak = maximum_pcm_s16_amplitude(&last_audio);

    let report = serde_json::json!({
        "duration_seconds": DURATION_SECONDS,
        "frame_rate": FRAME_RATE,
        "frames_written": frame_count,
        "video_start_seconds": video_start,
        "video_duration_seconds": video_duration,
        "audio_start_seconds": audio_start,
        "audio_duration_seconds": audio_duration,
        "start_drift_ms": start_drift_ms,
        "end_drift_ms": end_drift_ms,
        "first_video_luma": first_luma,
        "middle_video_luma": middle_luma,
        "last_video_luma": last_luma,
        "first_audio_peak": first_audio_peak,
        "middle_audio_peak": middle_audio_peak,
        "last_audio_peak": last_audio_peak,
        "output": output_path,
    });
    fs::write(&report_path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    let _ = fs::remove_file(&audio_path);

    assert!(
        start_drift_ms <= 35.0,
        "start A/V drift was {start_drift_ms:.3} ms"
    );
    assert!(
        end_drift_ms <= 55.0,
        "end A/V drift was {end_drift_ms:.3} ms"
    );
    assert!(
        first_luma >= 240.0,
        "first flash was missing: {first_luma:.3}"
    );
    assert!(
        middle_luma <= 16.0,
        "middle frame was not black: {middle_luma:.3}"
    );
    assert!(last_luma >= 240.0, "last flash was missing: {last_luma:.3}");
    assert!(
        first_audio_peak >= 1_000,
        "first click was missing: {first_audio_peak}"
    );
    assert!(
        middle_audio_peak <= 128,
        "middle audio was not silent: {middle_audio_peak}"
    );
    assert!(
        last_audio_peak >= 1_000,
        "last click was missing: {last_audio_peak}"
    );
}
