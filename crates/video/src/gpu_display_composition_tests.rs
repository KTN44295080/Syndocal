//! The native Display seam hands plain outputs with two or more layers to the
//! presenter for GPU composition at any output size, only when
//! the presenter's capabilities are known, never with an executing effect,
//! a transition, a rotated layer or a colour adjustment, a frame format it
//! cannot read as the CPU does or a layer larger than the device binds,
//! never for a blackout; the composition carries exactly what the CPU path
//! composites, a fallback does not prepare the frames twice, and admission
//! refuses anything else.
use super::*;

/// Frames of a fixed small size whose pixels depend on the layer, so a
/// wrong layer order or plan shows in the composite. Counts its decodes.
struct GradientDecoder {
    format: VideoPixelFormat,
    decodes: usize,
}

impl Default for GradientDecoder {
    fn default() -> Self {
        Self {
            format: VideoPixelFormat::Rgba8,
            decodes: 0,
        }
    }
}

impl VideoFrameDecoder for GradientDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decodes += 1;
        let seed = (request.layer_id as u8).wrapping_mul(53);
        let (width, height) = (16_u32, 9_u32);
        let bytes = match self.format {
            VideoPixelFormat::Rgba8 | VideoPixelFormat::Bgra8 => (width * height * 4) as usize,
            VideoPixelFormat::Dxt1 => (width.div_ceil(4) * height.div_ceil(4) * 8) as usize,
            _ => (width.div_ceil(4) * height.div_ceil(4) * 16) as usize,
        };
        Ok(Some(VideoFrame {
            layer_id: request.layer_id,
            width,
            height,
            pts_ms: request.position_ms,
            duration_ms: 16,
            format: self.format,
            data: (0..bytes / 4)
                .flat_map(|index| {
                    let value = (index as u8).wrapping_mul(29).wrapping_add(seed);
                    [
                        value,
                        255u8.wrapping_sub(value.wrapping_mul(3)),
                        value.wrapping_mul(7),
                        150u8.wrapping_add(value / 3),
                    ]
                })
                .collect(),
        }))
    }
}

type TestRenderer = VideoPreviewRenderer<DecoderBackedFrameProvider<GradientDecoder>>;

fn renderer_for(format: VideoPixelFormat) -> TestRenderer {
    VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(GradientDecoder {
            format,
            ..GradientDecoder::default()
        }),
    )
}

fn renderer() -> TestRenderer {
    renderer_for(VideoPixelFormat::Rgba8)
}

const GPU: Option<VideoOutputGpuSupport> = Some(VideoOutputGpuSupport {
    max_layer_bytes: u64::MAX,
});
const SMALL: (u32, u32) = (32, 18);

fn snapshot(layer_count: u64) -> VideoSnapshot {
    let blend_modes = [
        VideoBlendMode::Normal,
        VideoBlendMode::Screen,
        VideoBlendMode::Add,
        VideoBlendMode::Multiply,
        VideoBlendMode::Overlay,
        VideoBlendMode::Subtract,
        VideoBlendMode::Difference,
        VideoBlendMode::Lighten,
        VideoBlendMode::Darken,
        VideoBlendMode::SoftLight,
    ];
    VideoSnapshot {
        layers: (1..=layer_count)
            .map(|id| VideoLayerSummary {
                id,
                label: format!("Layer {id}"),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some(format!("layer-{id}.mp4")),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                blend_mode: blend_modes[(id as usize - 1) % blend_modes.len()].clone(),
                state: VideoLayerState {
                    opacity: 1.0 - id as f32 * 0.07,
                    position_ms: id * 40,
                    transform: Transform2D {
                        x: id as f32 * 0.03,
                        scale_x: 0.6 + id as f32 * 0.1,
                        ..Transform2D::default()
                    },
                    ..VideoLayerState::default()
                },
                isf_effect: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            })
            .collect(),
        compositions: vec![CompositionSummary {
            id: 5,
            label: "Program".to_string(),
            layer_ids: (1..=layer_count).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![6],
        }],
        outputs: vec![VideoOutputSummary {
            id: 6,
            label: "Projector".to_string(),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id: 5,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: SMALL.0,
            height: SMALL.1,
            endpoint_name: None,
            opacity: 0.9,
            blackout: false,
            mapping: VideoOutputMapping {
                offset_x: 0.05,
                black_level: 0.02,
                ..VideoOutputMapping::default()
            },
        }],
        master_opacity: 0.95,
        ..VideoSnapshot::default()
    }
}

fn render_with(
    renderer: &mut TestRenderer,
    snapshot: &VideoSnapshot,
    transitions: &VideoLayerTransitionRuntimeSnapshot,
    (width, height): (u32, u32),
    gpu: Option<VideoOutputGpuSupport>,
) -> VideoOutputDisplayRender {
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    renderer
        .prepare_output_display_render_preview_with_effects_and_transitions(
            snapshot,
            VideoEffectRenderContext {
                clip_runtime: &clip_runtime,
                project_render_epoch: 1,
            },
            transitions,
            6,
            width,
            height,
            gpu,
        )
        .unwrap()
}

fn render(
    renderer: &mut TestRenderer,
    snapshot: &VideoSnapshot,
    size: (u32, u32),
    gpu: Option<VideoOutputGpuSupport>,
) -> VideoOutputDisplayRender {
    render_with(
        renderer,
        snapshot,
        &VideoLayerTransitionRuntimeSnapshot::default(),
        size,
        gpu,
    )
}

fn gpu_composition(render: VideoOutputDisplayRender) -> VideoOutputGpuComposition {
    match render {
        VideoOutputDisplayRender::GpuComposition(composition) => composition,
        VideoOutputDisplayRender::Artistic(result) => {
            panic!("expected a GPU composition, got an artistic result: {result:?}")
        }
    }
}

fn artistic(render: VideoOutputDisplayRender) -> VideoOutputArtisticRenderResult {
    match render {
        VideoOutputDisplayRender::Artistic(result) => result,
        VideoOutputDisplayRender::GpuComposition(composition) => {
            panic!("expected an artistic result, got a GPU composition: {composition:?}")
        }
    }
}

fn contract(
    snapshot: &VideoSnapshot,
    (width, height): (u32, u32),
) -> VideoOutputPresentationContract {
    VideoOutputPresentationContract::new(1, 6, width, height, &snapshot.outputs[0].mapping)
}

#[test]
fn two_or_more_layers_at_any_size_with_known_capabilities_go_to_the_gpu() {
    let mut renderer = renderer();
    // A single layer keeps the CPU identity-copy path at any size.
    artistic(render(&mut renderer, &snapshot(1), SMALL, GPU));
    artistic(render(&mut renderer, &snapshot(1), (3_840, 2_160), GPU));
    // Two plain layers qualify even on a small output.
    let two = gpu_composition(render(&mut renderer, &snapshot(2), SMALL, GPU));
    assert_eq!(two.layers.len(), 2);
    assert_eq!(
        contract(&snapshot(2), SMALL).admit_gpu_composition(&two),
        Ok(())
    );
    // No presenter yet.
    artistic(render(&mut renderer, &snapshot(6), SMALL, None));

    let six = snapshot(6);
    let composition = gpu_composition(render(&mut renderer, &six, SMALL, GPU));
    assert_eq!(
        composition
            .layers
            .iter()
            .map(|layer| (layer.plan.layer_id, layer.frame.layer_id))
            .collect::<Vec<_>>(),
        (1..=6).map(|id| (id, id)).collect::<Vec<_>>()
    );
    assert_eq!(
        composition.evidence,
        VideoOutputRenderEvidence {
            project_render_epoch: 1,
            output_id: 6,
            freshness: VideoOutputRenderFreshness::Fresh,
            error: None,
        }
    );
    assert_eq!(composition.output_mapping, six.outputs[0].mapping);
    assert_eq!(
        contract(&six, SMALL).admit_gpu_composition(&composition),
        Ok(())
    );

    // Two layers also qualify at 4K UHD.
    let uhd = (3_840, 2_160);
    let composition = gpu_composition(render(&mut renderer, &snapshot(2), uhd, GPU));
    assert_eq!(composition.layers.len(), 2);
    assert_eq!(
        contract(&snapshot(2), uhd).admit_gpu_composition(&composition),
        Ok(())
    );
}

#[test]
fn the_gpu_composition_carries_what_the_cpu_composites() {
    let snapshot = snapshot(10);
    let mut gpu_renderer = renderer();
    let mut cpu_renderer = renderer();
    let composition = gpu_composition(render(&mut gpu_renderer, &snapshot, SMALL, GPU));
    let cpu = artistic(render(&mut cpu_renderer, &snapshot, SMALL, None));

    let plan = CompositionPlan {
        composition_id: 5,
        label: "Program".to_string(),
        output_ids: Vec::new(),
        master_opacity: 1.0,
        blackout: false,
        layers: composition
            .layers
            .iter()
            .map(|layer| layer.plan.clone())
            .collect(),
    };
    let frames = composition
        .layers
        .iter()
        .map(|layer| Arc::clone(&layer.frame))
        .collect::<Vec<_>>();
    let composed = composite_frames_rgba8(&plan, &frames, SMALL.0, SMALL.1).unwrap();
    assert_eq!(Some(&composed.data), cpu.frame().map(|frame| &frame.data));
    assert_eq!(composition.output_mapping, cpu.output_mapping);
    assert_eq!(
        composition.output_mapping_identity,
        cpu.output_mapping_identity
    );
    assert_eq!(composition.evidence, cpu.evidence);

    // The renderer's bookkeeping matches a fresh render with no effects,
    // and there is no CPU frame to keep as this output's last valid one.
    assert_eq!(gpu_renderer.last_output_render_error, None);
    assert!(gpu_renderer.last_effect_stage_faults.is_empty());
    assert!(gpu_renderer
        .output_last_valid_frames
        .iter()
        .all(|entry| entry.key.output_id != 6));
}

#[test]
fn generator_rgba_frames_follow_clip_blend_transform_effect_and_gpu_paths() {
    let mut snapshot = snapshot(10);
    for (index, layer) in snapshot.layers.iter_mut().enumerate().skip(1) {
        let generator = match index % 3 {
            0 => protocol::VideoGenerator::Solid {
                color: [20, 100, 230, 180],
            },
            1 => protocol::VideoGenerator::LinearGradient {
                color_a: [10, 40, 200, 255],
                color_b: [240, 80, 20, 120],
                angle_degrees: 47,
            },
            _ => protocol::VideoGenerator::Noise {
                seed: index as u32,
                scale: 8,
                speed: 100,
                color_a: [0, 20, 60, 100],
                color_b: [240, 200, 120, 255],
            },
        };
        layer.source = VideoSourceSummary::from_generator(generator).unwrap();
    }
    let mut gpu_renderer = renderer();
    let composition = gpu_composition(render(&mut gpu_renderer, &snapshot, SMALL, GPU));
    assert_eq!(gpu_renderer.frame_provider().decoder().decodes, 1);
    for layer in composition.layers.iter().skip(1) {
        assert_eq!((layer.frame.width, layer.frame.height), SMALL);
        assert_eq!(layer.frame.format, VideoPixelFormat::Rgba8);
    }
    assert_eq!(
        contract(&snapshot, SMALL).admit_gpu_composition(&composition),
        Ok(())
    );
    let cpu = artistic(render(&mut renderer(), &snapshot, SMALL, None));
    let plan = CompositionPlan {
        composition_id: 5,
        label: "Generator mix".into(),
        output_ids: Vec::new(),
        master_opacity: 1.0,
        blackout: false,
        layers: composition
            .layers
            .iter()
            .map(|layer| layer.plan.clone())
            .collect(),
    };
    let frames = composition
        .layers
        .iter()
        .map(|layer| Arc::clone(&layer.frame))
        .collect::<Vec<_>>();
    let composed = composite_frames_rgba8(&plan, &frames, SMALL.0, SMALL.1).unwrap();
    assert_eq!(Some(&composed.data), cpu.frame().map(|frame| &frame.data));
    // A cached generator still goes through normal artistic adjustments.
    snapshot.layers[1].state.color.brightness = 0.35;
    snapshot.layers[1].state.fx.pixelate = 3.0;
    let adjusted = artistic(render(&mut gpu_renderer, &snapshot, SMALL, GPU));
    assert_ne!(adjusted.frame().unwrap().data, cpu.frame().unwrap().data);
}

#[test]
fn generator_source_edits_replace_animated_queue_frames_at_a_nonzero_playhead() {
    let mut snapshot = snapshot(2);
    snapshot.layers[1].state.position_ms = 1234;
    let mut reused = renderer();
    let mut previous = None;
    for generator in [
        protocol::VideoGenerator::Noise {
            seed: 11,
            scale: 8,
            speed: 100,
            color_a: [0, 20, 60, 255],
            color_b: [240, 200, 120, 255],
        },
        protocol::VideoGenerator::Noise {
            seed: 11,
            scale: 8,
            speed: 0,
            color_a: [0, 20, 60, 255],
            color_b: [240, 200, 120, 255],
        },
        protocol::VideoGenerator::Noise {
            seed: 11,
            scale: 8,
            speed: 0,
            color_a: [0, 180, 20, 255],
            color_b: [40, 10, 255, 255],
        },
        protocol::VideoGenerator::Solid {
            color: [230, 25, 90, 255],
        },
        protocol::VideoGenerator::LinearGradient {
            color_a: [10, 240, 150, 255],
            color_b: [240, 10, 80, 255],
            angle_degrees: 37,
        },
    ] {
        snapshot.layers[1].source = VideoSourceSummary::from_generator(generator).unwrap();
        let actual = artistic(render(&mut reused, &snapshot, SMALL, None));
        let expected = artistic(render(&mut renderer(), &snapshot, SMALL, None));
        let bytes = actual.frame().unwrap().data.clone();
        assert_eq!(Some(&bytes), expected.frame().map(|frame| &frame.data));
        if let Some(previous) = previous {
            assert_ne!(bytes, previous);
        }
        previous = Some(bytes);
    }
    // Leaving a generator also retires its static frame from the same queue.
    snapshot.layers[1].source = snapshot.layers[0].source.clone();
    let actual = artistic(render(&mut reused, &snapshot, SMALL, None));
    let expected = artistic(render(&mut renderer(), &snapshot, SMALL, None));
    assert_eq!(
        actual.frame().map(|frame| &frame.data),
        expected.frame().map(|frame| &frame.data)
    );
}

#[test]
fn generator_outgoing_clip_edits_retire_the_blended_layer_queue() {
    let mut snapshot = snapshot(1);
    let incoming = VideoSourceSummary::from_generator(protocol::VideoGenerator::Solid {
        color: [30, 220, 80, 255],
    })
    .unwrap();
    snapshot.layers[0].source = incoming.clone();
    snapshot.layers[0].media_asset_id = Some(21);
    snapshot.layers[0].default_clip_slot_id = Some(VideoClipSlotId(21));
    snapshot.layers[0].state.position_ms = 1234;
    snapshot.layers[0].clip_slots = (20..=21)
        .map(|id| protocol::VideoClipSlotSummary {
            id: VideoClipSlotId(id),
            media_asset_id: id,
            in_point_ms: 0,
            out_point_ms: None,
            loop_mode: protocol::VideoClipLoopMode::Once,
            speed: 1.0,
            cue_points: Vec::new(),
            launch_quantization: protocol::VideoClipLaunchQuantization::Immediate,
            effect_overrides: Vec::new(),
        })
        .collect();
    snapshot.media_assets = (20..=21)
        .map(|id| protocol::MediaAssetSummary {
            id,
            label: format!("Clip {id}"),
            source: incoming.clone(),
            content_hash: None,
            byte_size: None,
        })
        .collect();
    let runtime = VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 1,
            active_slot_id: Some(VideoClipSlotId(20)),
            queued_slot_id: Some(VideoClipSlotId(21)),
            playhead_ms: 1234,
            transition: Some(protocol::VideoClipTakeTransitionSummary {
                origin_slot_id: VideoClipSlotId(20),
                outgoing_slot_id: VideoClipSlotId(20),
                incoming_slot_id: VideoClipSlotId(21),
                kind: protocol::VideoClipTakeKind::Crossfade,
                elapsed_ms: 250,
                duration_ms: 1000,
                duration: protocol::VideoClipTakeDuration::milliseconds(1000),
                progress_millis: 250,
                incoming_playhead_ms: 0,
                incoming_playing: true,
            }),
            ..protocol::VideoClipLayerRuntimeSummary::default()
        }],
        ..VideoClipRuntimeSnapshot::default()
    };
    let render = |renderer: &mut TestRenderer, snapshot: &VideoSnapshot| {
        renderer
            .prepare_output_display_render_preview_with_effects_and_transitions(
                snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &runtime,
                    project_render_epoch: 1,
                },
                &VideoLayerTransitionRuntimeSnapshot::default(),
                6,
                SMALL.0,
                SMALL.1,
                None,
            )
            .unwrap()
    };
    let mut reused = renderer();
    let mut previous = None;
    for speed in [100, 0] {
        snapshot.media_assets[0].source =
            VideoSourceSummary::from_generator(protocol::VideoGenerator::Noise {
                seed: 11,
                scale: 8,
                speed,
                color_a: [0, 20, 60, 255],
                color_b: [240, 200, 120, 255],
            })
            .unwrap();
        protocol::validate_video_clip_runtime_against_authored_slots(&runtime, &snapshot).unwrap();
        let actual = artistic(render(&mut reused, &snapshot));
        let expected = artistic(render(&mut renderer(), &snapshot));
        let bytes = actual.frame().unwrap().data.clone();
        assert_eq!(Some(&bytes), expected.frame().map(|frame| &frame.data));
        if let Some(previous) = previous {
            assert_ne!(bytes, previous);
        }
        previous = Some(bytes);
    }
}

fn isf_effect() -> VideoIsfEffectSummary {
    VideoIsfEffectSummary {
        enabled: true,
        label: "Invert".to_string(),
        source: Arc::<str>::from("invalid ISF source"),
        source_path: None,
        description: None,
        categories: Vec::new(),
        controls: Vec::new(),
        image_bindings: Vec::new(),
        stack: Vec::new(),
    }
}

fn executing_chain(scope: VideoEffectScope, bypassed: bool) -> VideoEffectChainSummary {
    VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope,
        bypassed,
        stages: vec![protocol::VideoEffectStageSummary {
            id: VideoEffectStageId(1),
            enabled: true,
            label: "Invert".to_string(),
            effect: protocol::VideoEffectSummary {
                id: VideoEffectId(1),
                kind: VideoEffectKind::Isf {
                    effect: isf_effect(),
                },
            },
        }],
    }
}

#[test]
fn effects_transitions_rotation_and_adjustments_stay_on_the_cpu() {
    let mut adjusted: Vec<(&str, VideoSnapshot)> = Vec::new();
    let mut variant = |name, change: &dyn Fn(&mut VideoSnapshot)| {
        let mut snapshot = snapshot(6);
        change(&mut snapshot);
        adjusted.push((name, snapshot));
    };
    variant("rotation", &|snapshot| {
        snapshot.layers[2].state.transform.rotation_deg = 15.0;
    });
    variant("colour", &|snapshot| {
        snapshot.layers[2].state.color.brightness = 0.1;
    });
    variant("glow", &|snapshot| snapshot.layers[2].state.fx.glow = 0.5);
    variant("edge", &|snapshot| snapshot.layers[2].state.fx.edge = 0.5);
    variant("key", &|snapshot| {
        snapshot.layers[2].state.fx.key_threshold = 0.3;
    });
    variant("pixelate", &|snapshot| {
        snapshot.layers[2].state.fx.pixelate = 2.0
    });
    variant("blur", &|snapshot| snapshot.layers[2].state.fx.blur = 1.0);
    for (name, scope) in [
        ("output chain", VideoEffectScope::Output { output_id: 6 }),
        (
            "composition chain",
            VideoEffectScope::Composition { composition_id: 5 },
        ),
    ] {
        variant(name, &move |snapshot| {
            snapshot
                .effect_chains
                .push(executing_chain(scope.clone(), false));
        });
    }
    // A Layer chain renders through the layer's legacy ISF projection.
    variant("layer chain", &|snapshot| {
        snapshot.layers[2].isf_effect = Some(isf_effect());
        snapshot.effect_chains.push(executing_chain(
            VideoEffectScope::Layer { layer_id: 3 },
            false,
        ));
    });
    for (name, snapshot) in &adjusted {
        let mut renderer = renderer();
        if let VideoOutputDisplayRender::GpuComposition(_) =
            render(&mut renderer, snapshot, SMALL, GPU)
        {
            panic!("{name}: composited on the GPU");
        }
    }

    // A bypassed chain does not execute.
    let mut snapshot = snapshot(6);
    snapshot.effect_chains.push(executing_chain(
        VideoEffectScope::Output { output_id: 6 },
        true,
    ));
    gpu_composition(render(&mut renderer(), &snapshot, SMALL, GPU));
}

#[test]
fn an_active_transition_bus_stays_on_the_cpu() {
    let mut snapshot = snapshot(6);
    let bus_id = protocol::VideoTransitionBusId(1);
    let from = VideoLayerTransitionTarget::Layer { layer_id: 1 };
    let to = VideoLayerTransitionTarget::Layer { layer_id: 2 };
    snapshot
        .transition_buses
        .push(protocol::VideoLayerTransitionBusSummary {
            id: bus_id,
            label: "Program bus".to_string(),
            composition_id: 5,
            enabled: true,
            members: vec![from.clone(), to.clone()],
            default_from: from.clone(),
            default_to: to.clone(),
            default_kind: protocol::VideoClipTakeKind::Crossfade,
            default_duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
            default_curve: VideoLayerTransitionCurve::Linear,
            matte_source: None,
        });
    let idle = VideoLayerTransitionRuntimeSnapshot::default();
    gpu_composition(render_with(&mut renderer(), &snapshot, &idle, SMALL, GPU));

    let active = VideoLayerTransitionRuntimeSnapshot {
        buses: vec![protocol::VideoLayerTransitionBusRuntimeSummary {
            bus_id,
            origin_from: from.clone(),
            from,
            to,
            kind: protocol::VideoClipTakeKind::Crossfade,
            curve: VideoLayerTransitionCurve::Linear,
            elapsed_ms: 250,
            duration_ms: 1_000,
            duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
            progress_millis: 250,
        }],
    };
    artistic(render_with(&mut renderer(), &snapshot, &active, SMALL, GPU));
}

#[test]
fn a_blackout_is_never_a_gpu_composition() {
    let blackouts: [&dyn Fn(&mut VideoSnapshot); 4] = [
        &|snapshot| snapshot.blackout = true,
        &|snapshot| snapshot.outputs[0].blackout = true,
        &|snapshot| snapshot.outputs[0].enabled = false,
        &|snapshot| snapshot.outputs[0].opacity = 0.0,
    ];
    for blackout in blackouts {
        let mut snapshot = snapshot(6);
        blackout(&mut snapshot);
        let mut renderer = renderer();
        // A GPU composition first, so the blackout is not merely the
        // renderer's first frame.
        gpu_composition(render(&mut renderer, &self::snapshot(6), SMALL, GPU));
        let result = artistic(render(&mut renderer, &snapshot, SMALL, GPU));
        assert_eq!(result.payload, VideoOutputArtisticPayload::HardBlackout);
        assert_eq!(
            contract(&snapshot, SMALL).admit(&result),
            Ok(VideoOutputArtisticAdmission::PresentHardBlackout)
        );
    }
}

#[test]
fn admission_refuses_what_the_gpu_does_not_composite_as_the_cpu() {
    let snapshot = snapshot(6);
    let valid = gpu_composition(render(&mut renderer(), &snapshot, SMALL, GPU));
    let contract = contract(&snapshot, SMALL);
    assert_eq!(contract.admit_gpu_composition(&valid), Ok(()));
    let rejected = |change: &dyn Fn(&mut VideoOutputGpuComposition)| {
        let mut composition = valid.clone();
        change(&mut composition);
        contract.admit_gpu_composition(&composition).unwrap_err()
    };

    assert_eq!(
        rejected(&|composition| composition.evidence.project_render_epoch = 2),
        VideoOutputArtisticRejection::ProjectEpochMismatch {
            expected: 1,
            actual: 2,
        }
    );
    assert_eq!(
        rejected(&|composition| {
            composition.evidence.freshness = VideoOutputRenderFreshness::LastValid;
        }),
        VideoOutputArtisticRejection::NotFresh {
            freshness: VideoOutputRenderFreshness::LastValid,
        }
    );
    assert_eq!(
        rejected(&|composition| composition.evidence.error = Some("decode".to_string())),
        VideoOutputArtisticRejection::ReportedRenderError
    );
    assert_eq!(
        rejected(&|composition| composition.evidence.output_id = 7),
        VideoOutputArtisticRejection::OutputMismatch {
            expected: 6,
            actual: 7,
        }
    );
    assert!(matches!(
        rejected(&|composition| composition.output_mapping.black_level = 0.5),
        VideoOutputArtisticRejection::ResultMappingIdentityMismatch { .. }
    ));
    assert!(matches!(
        rejected(&|composition| {
            composition.output_mapping.black_level = 0.5;
            composition.output_mapping_identity =
                VideoOutputMappingIdentity::from_mapping(&composition.output_mapping);
        }),
        VideoOutputArtisticRejection::ContractMappingIdentityMismatch { .. }
    ));
    let mut bc7 = (*valid.layers[0].frame).clone();
    bc7.format = VideoPixelFormat::Bc7;
    assert_eq!(
        rejected(&|composition| composition.layers[0].frame = Arc::new(bc7.clone())),
        VideoOutputArtisticRejection::UnsupportedFrameFormat {
            format: VideoPixelFormat::Bc7,
        }
    );
    let mut truncated = (*valid.layers[0].frame).clone();
    truncated.data.truncate(truncated.data.len() - 4);
    let mismatches: [(&str, &dyn Fn(&mut VideoOutputGpuComposition)); 6] = [
        ("layer id", &|composition| {
            composition.layers[0].plan.layer_id = 9
        }),
        ("rotation", &|composition| {
            composition.layers[0].plan.transform.rotation_deg = 90.0;
        }),
        ("colour", &|composition| {
            composition.layers[0].plan.color.gamma = 1.2
        }),
        ("glow", &|composition| {
            composition.layers[0].plan.fx.glow = 0.2
        }),
        ("geometry", &|composition| {
            composition.layers[0].plan.transform.x = f32::NAN;
        }),
        ("frame size", &|composition| {
            composition.layers[0].frame = Arc::new(truncated.clone());
        }),
    ];
    for (name, change) in mismatches {
        assert_eq!(
            rejected(change),
            VideoOutputArtisticRejection::AdmissionPayloadMismatch,
            "{name}"
        );
    }
}

#[test]
fn hap_dxt_goes_to_the_gpu_and_what_it_cannot_read_stays_on_the_cpu() {
    for format in [
        VideoPixelFormat::Bgra8,
        VideoPixelFormat::Dxt1,
        VideoPixelFormat::Dxt5,
    ] {
        let mut renderer = renderer_for(format);
        let composition = gpu_composition(render(&mut renderer, &snapshot(6), SMALL, GPU));
        assert!(composition
            .layers
            .iter()
            .all(|layer| layer.frame.format == format));
        assert_eq!(
            contract(&snapshot(6), SMALL).admit_gpu_composition(&composition),
            Ok(())
        );
    }
    for format in [VideoPixelFormat::YcoCgDxt5, VideoPixelFormat::Bc7] {
        let mut renderer = renderer_for(format);
        artistic(render(&mut renderer, &snapshot(6), SMALL, GPU));
    }
}

#[test]
fn a_fallback_after_preparing_does_not_prepare_the_frames_again() {
    let snapshot = snapshot(6);
    // A frame the GPU cannot read, a layer larger than the device binds,
    // and a GPU composition: each decodes every layer exactly once.
    let too_small = Some(VideoOutputGpuSupport {
        max_layer_bytes: 64,
    });
    for (format, gpu) in [
        (VideoPixelFormat::YcoCgDxt5, GPU),
        (VideoPixelFormat::Rgba8, too_small),
        (VideoPixelFormat::Rgba8, GPU),
    ] {
        let mut renderer = renderer_for(format);
        let mut cpu_renderer = renderer_for(format);
        let rendered = render(&mut renderer, &snapshot, SMALL, gpu);
        assert_eq!(renderer.frame_provider().decoder().decodes, 6, "{format:?}");
        if let VideoOutputDisplayRender::Artistic(result) = rendered {
            // The same result as rendering on the CPU from the start.
            let cpu = artistic(render(&mut cpu_renderer, &snapshot, SMALL, None));
            assert_eq!(result, cpu, "{format:?}");
        }
    }
}

#[test]
fn group_and_clip_chains_and_clip_transitions_are_not_eligible() {
    let snapshot = snapshot(6);
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let plan = build_video_output_render_plan(&snapshot, 6).unwrap();
    let eligible = |snapshot: &VideoSnapshot, clip_runtime: &VideoClipRuntimeSnapshot| {
        gpu_composition_eligible(snapshot, clip_runtime, &transitions, &plan)
    };
    let idle_runtime = VideoClipRuntimeSnapshot::default();
    assert!(eligible(&snapshot, &idle_runtime));

    let group_id = protocol::VideoLayerGroupId(1);
    let mut grouped = snapshot.clone();
    grouped.layer_groups.push(protocol::VideoLayerGroupSummary {
        id: group_id,
        label: "Group".to_string(),
        composition_id: 5,
        layer_ids: vec![2, 3],
        ..protocol::VideoLayerGroupSummary::default()
    });
    assert!(eligible(&grouped, &idle_runtime), "a group without a chain");
    grouped
        .effect_chains
        .push(executing_chain(VideoEffectScope::Group { group_id }, false));
    assert!(!eligible(&grouped, &idle_runtime), "a group chain");

    let slot_id = VideoClipSlotId(4);
    let playing = |transition| VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 3,
            active_slot_id: Some(slot_id),
            transition,
            ..protocol::VideoClipLayerRuntimeSummary::default()
        }],
        ..VideoClipRuntimeSnapshot::default()
    };
    assert!(
        eligible(&snapshot, &playing(None)),
        "a clip without a chain"
    );
    let mut clip_chain = snapshot.clone();
    clip_chain.effect_chains.push(executing_chain(
        VideoEffectScope::Clip {
            layer_id: 3,
            slot_id,
        },
        false,
    ));
    assert!(!eligible(&clip_chain, &playing(None)), "a clip-slot chain");
    let take = protocol::VideoClipTakeTransitionSummary {
        origin_slot_id: VideoClipSlotId(3),
        outgoing_slot_id: VideoClipSlotId(3),
        incoming_slot_id: slot_id,
        kind: protocol::VideoClipTakeKind::Crossfade,
        elapsed_ms: 100,
        duration_ms: 1_000,
        duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
        progress_millis: 100,
        incoming_playhead_ms: 0,
        incoming_playing: true,
    };
    assert!(
        !eligible(&snapshot, &playing(Some(take))),
        "a clip transition"
    );
}
