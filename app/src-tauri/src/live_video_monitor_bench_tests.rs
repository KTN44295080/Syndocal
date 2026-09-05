use super::*;
use protocol::{
    AutoVjSnapshot, CompositionSummary, VideoBlendMode, VideoClipRuntimeSnapshot, VideoLayerState,
    VideoLayerSummary, VideoLayerTransitionRuntimeSnapshot, VideoOutputKind, VideoOutputMapping,
    VideoOutputSummary, VideoSnapshot, VideoSourceKind, VideoSourceSummary,
};
use std::{hint::black_box, time::Duration};

fn two_video_outputs(paths: [String; 2]) -> VideoSnapshot {
    VideoSnapshot {
        layers: paths
            .into_iter()
            .enumerate()
            .map(|(index, path)| VideoLayerSummary {
                id: index as u64 + 8,
                label: format!("Fixture {index}"),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some(path),
                    name: None,
                    codec: Some("h264".into()),
                    metadata: None,
                },
                media_asset_id: None,
                blend_mode: VideoBlendMode::Normal,
                state: VideoLayerState::default(),
                isf_effect: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            })
            .collect(),
        compositions: (0..2)
            .map(|index| CompositionSummary {
                id: 70 + index,
                label: format!("Composition {index}"),
                layer_ids: vec![8 + index],
                timeline_layer_ids: Vec::new(),
                output_ids: vec![80 + index],
            })
            .collect(),
        outputs: (0..2)
            .map(|index| VideoOutputSummary {
                id: 80 + index,
                label: format!("Output {index}"),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 70 + index,
                fullscreen: false,
                monitor_id: Some(0),
                monitor_identity: None,
                width: 320,
                height: 128,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping::default(),
            })
            .collect(),
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

#[derive(Default)]
struct Samples {
    render: Vec<Duration>,
    raw_packet: Vec<Duration>,
    jpeg_encode: Vec<Duration>,
    jpeg_packet: Vec<Duration>,
    raw_bytes: usize,
    jpeg_bytes: usize,
}

fn percentile_us(samples: &[Duration], percentile: f64) -> f64 {
    let mut values = samples.to_vec();
    values.sort_unstable();
    values[((values.len() as f64 * percentile).ceil() as usize).saturating_sub(1)].as_secs_f64()
        * 1_000_000.0
}

fn assert_packet(
    packet: &[u8],
    frame: &video::VideoFrame,
    sequence: u64,
    format: LiveVideoMonitorPixelFormat,
) {
    assert_eq!(&packet[..4], b"SYLV");
    assert_eq!(packet[4], 2);
    assert_eq!(packet[5], LIVE_VIDEO_MONITOR_STATUS_FRAME);
    assert_eq!(packet[6], LiveVideoMonitorKind::Program.packet_value());
    assert_eq!(packet[7], format.packet_value());
    assert_eq!(
        u64::from_le_bytes(packet[8..16].try_into().unwrap()),
        sequence
    );
    assert_eq!(
        u64::from_le_bytes(packet[16..24].try_into().unwrap()),
        frame.pts_ms
    );
    assert_eq!(
        u16::from_le_bytes(packet[32..34].try_into().unwrap()) as u32,
        frame.width
    );
    assert_eq!(
        u16::from_le_bytes(packet[34..36].try_into().unwrap()) as u32,
        frame.height
    );
    assert_eq!(
        u32::from_le_bytes(packet[36..40].try_into().unwrap()) as usize,
        packet.len() - 40
    );
    if format == LiveVideoMonitorPixelFormat::Rgba {
        assert_eq!(&packet[40..], frame.data.as_slice());
    } else {
        let decoded = image::load_from_memory(&packet[40..]).unwrap();
        assert_eq!(
            (decoded.width(), decoded.height()),
            (frame.width, frame.height)
        );
    }
}

fn packet_samples(
    samples: &mut Samples,
    frame: &video::VideoFrame,
    sequence: u64,
    render: Duration,
) {
    samples.render.push(render);
    // Both transports consume the exact same rendered pixels. Alternate packet
    // ordering; JPEG decoding and all correctness checks stay outside timers.
    for format in if sequence % 2 == 0 {
        [
            LiveVideoMonitorPixelFormat::Rgba,
            LiveVideoMonitorPixelFormat::Jpeg,
        ]
    } else {
        [
            LiveVideoMonitorPixelFormat::Jpeg,
            LiveVideoMonitorPixelFormat::Rgba,
        ]
    } {
        let started = Instant::now();
        let jpeg = if format == LiveVideoMonitorPixelFormat::Jpeg {
            Some(encode_live_video_monitor_jpeg(black_box(frame), 68).unwrap())
        } else {
            None
        };
        let encode_time = started.elapsed();
        let payload = jpeg.as_deref().unwrap_or(&frame.data);
        let started = Instant::now();
        let packet = live_video_monitor_packet(
            LIVE_VIDEO_MONITOR_STATUS_FRAME,
            LiveVideoMonitorKind::Program,
            sequence,
            frame.pts_ms,
            render.as_micros() as u32,
            if jpeg.is_some() {
                encode_time.as_micros() as u32
            } else {
                0
            },
            frame.width,
            frame.height,
            black_box(payload),
            format,
        )
        .unwrap();
        black_box(&packet);
        let packet_time = started.elapsed();
        if jpeg.is_some() {
            samples.jpeg_encode.push(encode_time);
            samples.jpeg_packet.push(packet_time);
            samples.jpeg_bytes += packet.len();
        } else {
            samples.raw_packet.push(packet_time);
            samples.raw_bytes += packet.len();
            assert_eq!(&packet[28..32], &[0; 4]);
        }
        assert_packet(&packet, frame, sequence, format);
    }
}

#[test]
#[ignore = "requires SYNDOCAL_GPU_TEST_VIDEO and SYNDOCAL_GPU_TEST_BACKGROUND (>3s), supported GPU on Windows"]
fn live_video_monitor_benchmark_raw_vs_jpeg_two_real_outputs() {
    let mut snapshot = two_video_outputs([
        std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set original foreground path"),
        std::env::var("SYNDOCAL_GPU_TEST_BACKGROUND").expect("set original background path"),
    ]);
    // Preferred owns the production LibavFrameDecoder and exposes its counters.
    let decoder = video::PreferredVideoFrameDecoder::new(video::FfmpegCliFrameDecoder::new(
        "missing-benchmark-ffmpeg.exe",
    ));
    let mut renderer = video::VideoPreviewRenderer::with_frame_provider(
        video::VideoRuntimeConfig::default(),
        video::DecoderBackedFrameProvider::new(decoder).with_prefetch(0, 33),
    );
    let runtime = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let mut sequence = 0;
    let mut last_frames: Vec<video::VideoFrame> = Vec::new();
    let mut paused_hardware_frames = 0;
    for (phase, positions) in [
        (
            "sequential",
            (0..60).map(|index| index * 1000 / 30).collect::<Vec<u64>>(),
        ),
        ("paused", vec![59 * 1000 / 30; 10]),
        ("seek", vec![900, 2500, 500]),
    ] {
        let mut samples = Samples::default();
        for position in positions {
            for layer in &mut snapshot.layers {
                layer.state.position_ms = position;
            }
            for output in 0..2 {
                let started = Instant::now();
                let frame = renderer
                    .render_output_preview_with_effects_and_transitions(
                        black_box(&snapshot),
                        video::VideoEffectRenderContext {
                            clip_runtime: &runtime,
                            project_render_epoch: 1,
                        },
                        &transitions,
                        80 + output,
                        320,
                        128,
                    )
                    .unwrap();
                black_box(&frame);
                let render_time = started.elapsed();
                if phase == "paused" {
                    assert_eq!(frame, last_frames[output as usize]);
                }
                if phase == "sequential" && position == 59 * 1000 / 30 {
                    last_frames.push(frame.clone());
                }
                packet_samples(&mut samples, &frame, sequence, render_time);
                sequence += 1;
            }
        }
        let stats = renderer.frame_provider().decoder().diagnostics();
        assert_eq!(stats.cli_fallback_requests, 0);
        assert_eq!(stats.libav_session_count, 2);
        assert_eq!(stats.libav_session_error_count, 0);
        if phase != "seek" {
            assert_eq!(stats.libav_session_open_count, 2);
            assert_eq!(stats.libav_session_reset_count, 0);
        }
        if phase == "sequential" {
            paused_hardware_frames = stats.libav_hardware_frame_count;
        }
        if phase == "paused" {
            assert_eq!(stats.libav_hardware_frame_count, paused_hardware_frames);
        }
        #[cfg(target_os = "windows")]
        {
            assert_eq!(stats.libav_hardware_session_count, 2);
            assert!(stats.libav_hardware_frame_count > 0);
            assert_eq!(stats.libav_hardware_error_count, 0);
        }
        for (stage, values) in [
            ("render", &samples.render),
            ("raw_packet", &samples.raw_packet),
            ("jpeg_encode", &samples.jpeg_encode),
            ("jpeg_packet", &samples.jpeg_packet),
        ] {
            eprintln!(
                "monitor benchmark phase={phase} n={} stage={stage} p50_us={:.3} p95_us={:.3}",
                values.len(),
                percentile_us(values, 0.50),
                percentile_us(values, 0.95)
            );
        }
        eprintln!("monitor benchmark phase={phase} raw_bytes={} jpeg_bytes={} opens={} resets={} reuse={} hw_frames={}; excludes snapshot capture, IPC, WebView and physical output", samples.raw_bytes, samples.jpeg_bytes, stats.libav_session_open_count, stats.libav_session_reset_count, stats.libav_frame_reuse_count, stats.libav_hardware_frame_count);
    }
}
