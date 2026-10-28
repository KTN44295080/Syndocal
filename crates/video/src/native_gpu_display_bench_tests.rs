//! QA-only per-phase bench of the C4 ordinary fixture with the real decoder.
//!
//! Ignored and compiled only with `qa-native-frame-timing`. It drives the
//! resident facade (headless broker: no surface, so no present phase) and the
//! legacy canonical output preparation on the same media, extent and FX stack,
//! records `native_frame_timing` spans and writes the export to
//! `SYNDOCAL_QA_BENCH_OUT/<route>-<layers>l.json`. Inputs: the C4 media path
//! in `SYNDOCAL_QA_C4_MEDIA`. Host-wall attribution only; not a present FPS.
use super::*;
use crate::native_frame_timing as timing;
use crate::{
    DecoderBackedFrameProvider, PreferredVideoFrameDecoder, VideoEffectRenderContext,
    VideoRuntimeConfig,
};
use protocol::{VideoLayerTransitionRuntimeSnapshot, VideoSnapshot};

const OUTPUT: u64 = 101;
const EPOCH: u64 = 7;
const EXTENT: (u32, u32) = (2560, 1600);
const TICKS: u64 = 240;
/// The C4 fixture pass: `gl_FragColor = vec4(c.rgb * 0.99, c.a)`, three stages.
const QA_PASS: &str = "/*{\"ISFVSN\":\"2\",\"INPUTS\":[{\"NAME\":\"inputImage\",\"TYPE\":\"image\"}]}*/\nvoid main(){ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4(c.rgb * 0.99, c.a); }";

struct Permit;
impl NativeResidentPresentationAuthority for Permit {
    fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        NativeResidentPresentDecision::Permit
    }
}

fn scene(media: &str, layers: u64) -> VideoSnapshot {
    let shader = crate::prepare_isf_shader(QA_PASS).unwrap();
    let mut effect =
        crate::isf_effect_from_prepared("QA color pass".into(), QA_PASS, None, &shader);
    let stage: protocol::VideoIsfEffectStageSummary =
        serde_json::from_value(serde_json::to_value(&effect).unwrap()).unwrap();
    effect.stack = vec![stage; 2];
    VideoSnapshot {
        layers: (1..=layers)
            .map(|id| protocol::VideoLayerSummary {
                id,
                label: format!("QA {id}"),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some(media.to_string()),
                    name: None,
                    codec: Some("h264".into()),
                    metadata: None,
                },
                media_asset_id: None,
                blend_mode: protocol::VideoBlendMode::Normal,
                state: protocol::VideoLayerState {
                    opacity: 0.7,
                    playing: true,
                    ..protocol::VideoLayerState::default()
                },
                isf_effect: Some(effect.clone()),
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            })
            .collect(),
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        compositions: vec![protocol::CompositionSummary {
            id: 1,
            label: "QA".into(),
            layer_ids: (1..=layers).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![OUTPUT],
        }],
        outputs: vec![protocol::VideoOutputSummary {
            id: OUTPUT,
            label: "QA bench".into(),
            kind: protocol::VideoOutputKind::Display,
            enabled: true,
            composition_id: 1,
            fullscreen: true,
            monitor_id: Some(0),
            monitor_identity: None,
            width: EXTENT.0,
            height: EXTENT.1,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: protocol::VideoOutputMapping::default(),
        }],
        mapping_presets: Vec::new(),
        auto_vj: protocol::AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

fn renderer() -> VideoPreviewRenderer<DecoderBackedFrameProvider<PreferredVideoFrameDecoder>> {
    VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(PreferredVideoFrameDecoder::from_env())
            .with_prefetch(0, 33),
    )
}

fn session() -> NativeResidentDisplaySession {
    let factory = NativeGpuFactory::new();
    let owner = factory
        .create_headless(OUTPUT, NativeGpuRequirements::default())
        .expect("bench requires a headless adapter");
    let mut session = NativeResidentDisplaySession {
        output_id: OUTPUT,
        factory,
        owner: ResidentOwner(Some(owner)),
        presenter: None,
        compositor: None,
        cache: SharedNativeOutputCache::new(OUTPUT),
        geometry_generation: 0,
        reserved_geometry: None,
        history: None,
        terminal_fault: None,
        delivery: crate::native_gpu_output::NativeDeliveryCounter::default(),
        retired_ready: Vec::new(),
        retirements: Vec::new(),
        unsubmitted: Vec::new(),
        recovered_history: None,
        request_sequence: 0,
        history_configuration: None,
        input_bindings: Vec::new(),
        next_input_id: 1,
        history_host: None,
        follow_last_valid: None,
        ring: Default::default(),
        deferred_budget: None,
        evidence: Default::default(),
    };
    session.compositor = Some(NativeGpuCompositor::from_shared(&mut session.owner).unwrap());
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    session.reserved_geometry = None;
    session
}

fn export(route: &str, layers: u64) {
    timing::stop_capture().unwrap();
    timing::retire_thread();
    timing::acknowledge_stop().unwrap();
    let snapshot = timing::snapshot().unwrap();
    let dir = std::path::PathBuf::from(std::env::var_os("SYNDOCAL_QA_BENCH_OUT").unwrap());
    let path = dir.join(format!("{route}-{layers}l.json"));
    std::fs::write(&path, serde_json::to_vec(&snapshot).unwrap()).unwrap();
    println!("bench export {}", path.display());
}

#[test]
#[ignore = "QA bench: real decoder, C4 media and 2560x1600 frames"]
fn native_gpu_output_display_c4_fixture_per_phase_bench() {
    let Some(media) = std::env::var_os("SYNDOCAL_QA_C4_MEDIA") else {
        panic!("SYNDOCAL_QA_C4_MEDIA is required");
    };
    let media = media.into_string().unwrap();
    timing::enable_for_tests();
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        clip_runtime: &clips,
        project_render_epoch: EPOCH,
        audio_frame: None,
    };
    for layers in [1, 3] {
        for route in ["resident", "legacy"] {
            let mut snapshot = scene(&media, layers);
            let mut renderer = renderer();
            let mut resident = (route == "resident").then(session);
            timing::start().unwrap();
            timing::prepare_thread().unwrap();
            let started = std::time::Instant::now();
            for tick in 0..TICKS {
                for layer in &mut snapshot.layers {
                    layer.state.position_ms = tick * 1000 / 60;
                }
                let parent = timing::begin_parent(OUTPUT, EPOCH, EXTENT.0, EXTENT.1);
                let presented = {
                    let _prepare = timing::span(timing::Phase::Prepare);
                    match resident.as_mut() {
                        Some(session) => session
                            .prepare_output(
                                &mut renderer,
                                &snapshot,
                                context,
                                &transitions,
                                EXTENT,
                                &|| false,
                                &Permit,
                            )
                            .map(|frame| {
                                matches!(
                                    frame.content,
                                    super::present::ResidentDisplayContent::Resident(_)
                                        | super::present::ResidentDisplayContent::Cpu(_)
                                )
                            })
                            .unwrap_or_else(|error| panic!("resident tick {tick}: {error}")),
                        None => renderer
                            .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                                &snapshot,
                                context,
                                &transitions,
                                OUTPUT,
                                EXTENT.0,
                                EXTENT.1,
                            )
                            .map(|result| result.evidence.freshness == crate::VideoOutputRenderFreshness::Fresh)
                            .unwrap_or_else(|error| panic!("legacy tick {tick}: {error:?}")),
                    }
                };
                timing::set_outcome(if presented {
                    timing::Outcome::Presented
                } else {
                    timing::Outcome::Rejected
                });
                drop(parent);
            }
            println!(
                "{route} {layers}l: {TICKS} ticks in {:?}; decoder {:?}",
                started.elapsed(),
                renderer.frame_provider().decoder().diagnostics()
            );
            export(route, layers);
            if let Some(mut session) = resident {
                session.begin_stop_with_renderer(&mut renderer).unwrap();
                let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
                while !session.poll_stop().unwrap() {
                    assert!(
                        std::time::Instant::now() < deadline,
                        "bench Stop did not reap"
                    );
                    std::thread::yield_now();
                }
            }
        }
    }
}

/// Live D1b oracle: three real file decoders at the live 3L tick spacing
/// (85 ms) through two 12-second wraps. Before task66, tick 277 (11545 ms in
/// the second pass) selected a previous-pass history entry whose resident was
/// trimmed and failed the 12P CPU continuation quote at 27P live.
#[test]
#[ignore = "QA C4 media and full-size three-layer loop regression"]
fn native_gpu_output_display_c4_sparse_loop_remains_resident() {
    let media = std::env::var("SYNDOCAL_QA_C4_MEDIA").expect("C4 media required");
    let mut snapshot = scene(&media, 3);
    let mut renderer = renderer();
    let mut session = session();
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    for tick in 0..300_u64 {
        let target = tick * 85 % 12000;
        for layer in &mut snapshot.layers {
            layer.state.position_ms = target;
        }
        let frame = session
            .prepare_output(
                &mut renderer,
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &clips,
                    project_render_epoch: EPOCH,
                    audio_frame: None,
                },
                &transitions,
                EXTENT,
                &|| false,
                &Permit,
            )
            .unwrap_or_else(|error| {
                panic!("sparse loop tick={tick} target={target} error={error}")
            });
        assert!(
            matches!(
                frame.content,
                super::present::ResidentDisplayContent::Resident(_)
                    | super::present::ResidentDisplayContent::InFlight
            ),
            "sparse loop tick={tick} target={target} left resident"
        );
    }
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while !session.poll_stop().unwrap() {
        assert!(std::time::Instant::now() < deadline);
        std::thread::yield_now();
    }
}
