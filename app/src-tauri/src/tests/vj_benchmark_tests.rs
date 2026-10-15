//! VJ benchmark, Tier A (`qa/VJ_BENCHMARK_2026-10-14.md`): the production
//! decoder and compositor rendering the media pack from
//! `qa/harnesses/New-VjBenchMedia.ps1`, headless, timed per frame.
//!
//! Run with (a debug build is several times slower and meaningless here):
//! `SYNDOCAL_VJ_BENCH_MEDIA=<dir> cargo test --release -p syndocal --bin syndocal vj_benchmark -- --ignored --nocapture`
//! `SYNDOCAL_VJ_BENCH_ONLY=codec,x2` keeps only the scenarios whose name
//! contains one of the comma-separated parts.
//!
//! Frames start on a 60 Hz cadence, as the output's frame clock starts them,
//! so work the decoder does between frames (decoding the next frame ahead)
//! overlaps the idle time it would in the app. Only each frame's render is
//! timed. `SYNDOCAL_VJ_BENCH_UNPACED=1` renders back to back instead.
//! Results go to stderr and to `target/qa/vj-bench/<unix ms>.json`.
//!
//! Each frame goes through the same seam as the native output window
//! (`prepare_output_artistic_render_result_preview_with_effects_and_transitions`):
//! decode, CPU composite and effects. The window's GPU output mapping and
//! presentation, NDI, Spout and the WebView monitors are outside it; the
//! native output's own 600-frame deadline window covers presentation.

use protocol::{
    AutoVjSnapshot, CompositionSummary, VideoBlendMode, VideoClipRuntimeSnapshot, VideoLayerState,
    VideoLayerSummary, VideoLayerTransitionRuntimeSnapshot, VideoOutputKind, VideoOutputMapping,
    VideoOutputSummary, VideoSnapshot, VideoSourceKind, VideoSourceSummary,
};
use std::{
    hint::black_box,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

/// One 60 fps frame.
const FRAME_BUDGET: Duration = Duration::from_micros(16_667);
const WARMUP_FRAMES: usize = 30;
const MEASURED_FRAMES: usize = 240;

fn scene(
    paths: &[PathBuf],
    blend_modes: &[VideoBlendMode],
    width: u32,
    height: u32,
) -> VideoSnapshot {
    let layers = paths
        .iter()
        .enumerate()
        .map(|(index, path)| VideoLayerSummary {
            id: 100 + index as u64,
            label: format!("Bench {index}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(path.to_string_lossy().to_string()),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: blend_modes[index % blend_modes.len()].clone(),
            state: VideoLayerState {
                // Every layer but the base is half transparent, so each one
                // is really blended.
                opacity: if index == 0 { 1.0 } else { 0.5 },
                ..VideoLayerState::default()
            },
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        })
        .collect::<Vec<_>>();
    VideoSnapshot {
        compositions: vec![CompositionSummary {
            id: 10,
            label: "Bench".into(),
            layer_ids: layers.iter().map(|layer| layer.id).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![20],
        }],
        layers,
        outputs: vec![VideoOutputSummary {
            id: 20,
            label: "Bench output".into(),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id: 10,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width,
            height,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }],
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        mapping_presets: Vec::new(),
        auto_vj: AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

#[derive(Debug)]
struct Result {
    scenario: String,
    layers: usize,
    output: (u32, u32),
    p50_ms: f64,
    p95_ms: f64,
    p99_ms: f64,
    max_ms: f64,
    over_budget: usize,
    frames: usize,
    hardware_frames: u64,
    cli_fallbacks: u64,
}

fn percentile_ms(sorted: &[Duration], percentile: f64) -> f64 {
    let index = ((sorted.len() as f64 * percentile).ceil() as usize).saturating_sub(1);
    sorted[index.min(sorted.len() - 1)].as_secs_f64() * 1_000.0
}

/// Renders the scene at a 60 fps media clock and times every frame.
fn run(scenario: &str, snapshot: &mut VideoSnapshot, width: u32, height: u32) -> Result {
    // The production decoder (as the native output builds it), with the
    // FFmpeg CLI fallback pinned off so every frame is an in-process decode.
    let decoder = crate::ndi_transport::NdiAwareVideoFrameDecoder::with_file_decoder(
        video::PreferredVideoFrameDecoder::new(video::FfmpegCliFrameDecoder::new(
            "missing-benchmark-ffmpeg.exe",
        )),
    );
    let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
        video::VideoRuntimeConfig::default(),
        video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
    );
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let mut samples = Vec::with_capacity(MEASURED_FRAMES);
    let paced = std::env::var_os("SYNDOCAL_VJ_BENCH_UNPACED").is_none();
    let mut next_frame_at = Instant::now();
    for frame in 0..WARMUP_FRAMES + MEASURED_FRAMES {
        if paced {
            let now = Instant::now();
            if next_frame_at > now {
                std::thread::sleep(next_frame_at - now);
            }
            next_frame_at = next_frame_at.max(now) + FRAME_BUDGET;
        }
        let position_ms = (frame as u64 * 1_000) / 60 % 11_000;
        for layer in &mut snapshot.layers {
            layer.state.position_ms = position_ms;
        }
        let started = Instant::now();
        let rendered = renderer
            .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                black_box(snapshot),
                video::VideoEffectRenderContext {
                    clip_runtime: &clip_runtime,
                    project_render_epoch: 1,
                },
                &transitions,
                20,
                width,
                height,
            )
            .expect("benchmark frame renders");
        assert!(
            rendered.frame().is_some(),
            "{scenario}: the benchmark scene must render pixels, not a blackout"
        );
        black_box(&rendered);
        if frame >= WARMUP_FRAMES {
            samples.push(started.elapsed());
        }
    }
    let stats = renderer.frame_provider().decoder().diagnostics();
    if stats.libav_lookahead_decode_count > 0 {
        eprintln!(
            "{scenario}: decoded ahead {}, used {}, discarded {}",
            stats.libav_lookahead_decode_count,
            stats.libav_lookahead_hit_count,
            stats.libav_lookahead_discard_count
        );
    }
    samples.sort_unstable();
    Result {
        scenario: scenario.to_string(),
        layers: snapshot.layers.len(),
        output: (width, height),
        p50_ms: percentile_ms(&samples, 0.50),
        p95_ms: percentile_ms(&samples, 0.95),
        p99_ms: percentile_ms(&samples, 0.99),
        max_ms: percentile_ms(&samples, 1.0),
        over_budget: samples
            .iter()
            .filter(|sample| **sample > FRAME_BUDGET)
            .count(),
        frames: samples.len(),
        hardware_frames: stats.libav_hardware_frame_count,
        cli_fallbacks: stats.cli_fallback_requests,
    }
}

fn media(directory: &Path, name: &str) -> Option<PathBuf> {
    let path = directory.join(name);
    path.is_file().then_some(path)
}

#[test]
#[ignore = "requires SYNDOCAL_VJ_BENCH_MEDIA (qa/harnesses/New-VjBenchMedia.ps1) and a GPU"]
fn vj_benchmark_tier_a() {
    let directory = PathBuf::from(
        std::env::var("SYNDOCAL_VJ_BENCH_MEDIA").expect("set SYNDOCAL_VJ_BENCH_MEDIA"),
    );
    let only = std::env::var("SYNDOCAL_VJ_BENCH_ONLY").unwrap_or_default();
    let selected = |scenario: &str| {
        only.trim().is_empty()
            || only
                .split(',')
                .map(str::trim)
                .any(|part| !part.is_empty() && scenario.contains(part))
    };
    let mut results = Vec::new();
    let full_hd = (1_920, 1_080);

    // 1. Decode and render cost per codec: one layer, 1080p output.
    for clip in [
        "h264_1080p30.mp4",
        "h264_1080p60.mp4",
        "hevc_1080p30.mp4",
        "prores422_1080p30.mov",
        "hap_1080p30.mov",
        "hapq_1080p30.mov",
    ] {
        let scenario = format!("codec {clip}");
        if !selected(&scenario) {
            continue;
        }
        let Some(path) = media(&directory, clip) else {
            eprintln!("vj benchmark: {clip} missing, skipped");
            continue;
        };
        let mut snapshot = scene(&[path], &[VideoBlendMode::Normal], full_hd.0, full_hd.1);
        results.push(run(&scenario, &mut snapshot, full_hd.0, full_hd.1));
    }

    // 2. Layer scaling: N layers of 1080p H.264 (and HAP) into 1080p, mixed
    // blend modes.
    let blends = [
        VideoBlendMode::Normal,
        VideoBlendMode::Add,
        VideoBlendMode::Screen,
        VideoBlendMode::Multiply,
    ];
    for clip in ["h264_1080p30.mp4", "hap_1080p30.mov"] {
        let Some(path) = media(&directory, clip) else {
            continue;
        };
        for layers in [1_usize, 2, 4, 6, 8] {
            let scenario = format!("layers {clip} x{layers}");
            if !selected(&scenario) {
                continue;
            }
            let paths = vec![path.clone(); layers];
            let mut snapshot = scene(&paths, &blends, full_hd.0, full_hd.1);
            results.push(run(&scenario, &mut snapshot, full_hd.0, full_hd.1));
        }
    }

    // 3. 4K: one 4K source into a 4K output, and four 1080p layers into 4K.
    if let Some(path) =
        media(&directory, "h264_2160p30.mp4").filter(|_| selected("4k h264_2160p30 x1"))
    {
        let mut snapshot = scene(&[path], &[VideoBlendMode::Normal], 3_840, 2_160);
        results.push(run("4k h264_2160p30 x1", &mut snapshot, 3_840, 2_160));
    }
    if let Some(path) =
        media(&directory, "h264_1080p30.mp4").filter(|_| selected("4k output, h264_1080p30 x4"))
    {
        let mut snapshot = scene(&vec![path; 4], &blends, 3_840, 2_160);
        results.push(run(
            "4k output, h264_1080p30 x4",
            &mut snapshot,
            3_840,
            2_160,
        ));
    }

    let mut report = String::from("scenario | layers | output | p50 ms | p95 ms | p99 ms | max ms | over 16.7 ms | hw frames\n");
    for result in &results {
        let line = format!(
            "{} | {} | {}x{} | {:.2} | {:.2} | {:.2} | {:.2} | {}/{} | {}{}\n",
            result.scenario,
            result.layers,
            result.output.0,
            result.output.1,
            result.p50_ms,
            result.p95_ms,
            result.p99_ms,
            result.max_ms,
            result.over_budget,
            result.frames,
            result.hardware_frames,
            if result.cli_fallbacks > 0 {
                " (CLI fallback)"
            } else {
                ""
            }
        );
        report.push_str(&line);
    }
    eprintln!("{report}");
    let json = results
        .iter()
        .map(|result| {
            serde_json::json!({
                "scenario": result.scenario,
                "layers": result.layers,
                "output": [result.output.0, result.output.1],
                "p50_ms": result.p50_ms,
                "p95_ms": result.p95_ms,
                "p99_ms": result.p99_ms,
                "max_ms": result.max_ms,
                "over_16_7_ms": result.over_budget,
                "frames": result.frames,
                "hardware_frames": result.hardware_frames,
                "cli_fallbacks": result.cli_fallbacks,
            })
        })
        .collect::<Vec<_>>();
    let out = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/qa/vj-bench");
    std::fs::create_dir_all(&out).unwrap();
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();
    std::fs::write(
        out.join(format!("{stamp}.json")),
        serde_json::to_vec_pretty(&serde_json::json!({
            "media": directory,
            "budget_ms": 16.667,
            "seam": "prepare_output_artistic_render_result_preview_with_effects_and_transitions",
            "results": json,
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(
        !results.is_empty(),
        "no benchmark media found in {directory:?}"
    );
}
