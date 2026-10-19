use super::*;
use protocol::{VideoLayerGroupId, VideoLayerGroupSummary};

#[derive(Default)]
struct Provider {
    decodes: usize,
}

impl VideoFrameProvider for Provider {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}

    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.decodes += 1;
        let pixel = match layer.id {
            1 => [32, 80, 160, 255],
            2 => [200, 40, 100, 128],
            _ => [20, 180, 60, 192],
        };
        Ok(VideoFrame {
            layer_id: layer.id,
            width,
            height,
            pts_ms: layer.state.position_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: pixel.repeat((width * height) as usize),
        })
    }
}

const SIZE: (u32, u32) = (16, 12);

fn renderer() -> VideoPreviewRenderer<Provider> {
    VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), Provider::default())
}

pub(crate) fn scene(blend_mode: VideoBlendMode, transform: Option<Transform2D>) -> VideoSnapshot {
    VideoSnapshot {
        layers: (1..=3)
            .map(|id| VideoLayerSummary {
                id,
                label: format!("Layer {id}"),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some(format!("{id}.mov")),
                    name: None,
                    codec: None,
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
        compositions: vec![CompositionSummary {
            id: 5,
            label: "Program and Preview".into(),
            layer_ids: vec![1, 2, 3],
            timeline_layer_ids: Vec::new(),
            output_ids: vec![6],
        }],
        outputs: vec![VideoOutputSummary {
            id: 6,
            label: "Display".into(),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id: 5,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: SIZE.0,
            height: SIZE.1,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }],
        layer_groups: vec![VideoLayerGroupSummary {
            id: VideoLayerGroupId(7),
            label: "Group".into(),
            composition_id: 5,
            layer_ids: vec![2, 3],
            opacity: 0.55,
            blend_mode,
            transform,
            ..VideoLayerGroupSummary::default()
        }],
        ..VideoSnapshot::default()
    }
}

fn display(
    renderer: &mut VideoPreviewRenderer<Provider>,
    snapshot: &VideoSnapshot,
    gpu: bool,
) -> VideoOutputDisplayRender {
    renderer
        .prepare_output_display_render_preview_with_effects_and_transitions(
            snapshot,
            VideoEffectRenderContext {
                clip_runtime: &VideoClipRuntimeSnapshot::default(),
                project_render_epoch: 1,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            6,
            SIZE.0,
            SIZE.1,
            gpu.then_some(VideoOutputGpuSupport {
                max_layer_bytes: u64::MAX,
            }),
        )
        .unwrap()
}

pub(crate) fn parity_fixture(snapshot: &VideoSnapshot) -> (VideoOutputGpuComposition, VideoFrame) {
    let VideoOutputDisplayRender::GpuComposition(gpu) = display(&mut renderer(), snapshot, true)
    else {
        panic!("group fixture must exercise GPU composition");
    };
    let VideoOutputDisplayRender::Artistic(cpu) = display(&mut renderer(), snapshot, false) else {
        panic!("expected CPU artistic result");
    };
    let VideoOutputArtisticPayload::Frame(cpu) = cpu.payload else {
        panic!("expected frame")
    };
    (gpu, cpu)
}

#[test]
fn layer_group_controls_apply_after_children_and_preserve_translucent_energy() {
    let snapshot = scene(VideoBlendMode::Normal, None);
    let (_, cpu) = parity_fixture(&snapshot);
    // Independent reference: composite children over transparent black, then
    // apply the group's coverage exactly once against the opaque backdrop.
    let mut composite = [0, 0, 0, 0];
    blend_pixel(
        &mut composite,
        &[200, 40, 100, 128],
        1.0,
        &VideoBlendMode::Normal,
    );
    blend_pixel(
        &mut composite,
        &[20, 180, 60, 192],
        1.0,
        &VideoBlendMode::Normal,
    );
    let alpha = u32::from(composite[3]);
    for channel in &mut composite[..3] {
        *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha) as u8;
    }
    let mut expected = [32, 80, 160, 255];
    blend_pixel(&mut expected, &composite, 0.55, &VideoBlendMode::Normal);
    assert_eq!(&cpu.data[..4], &expected);
    let mut per_child = [32, 80, 160, 255];
    blend_pixel(
        &mut per_child,
        &[200, 40, 100, 128],
        0.55,
        &VideoBlendMode::Normal,
    );
    blend_pixel(
        &mut per_child,
        &[20, 180, 60, 192],
        0.55,
        &VideoBlendMode::Normal,
    );
    assert_ne!(expected, per_child);
}

#[test]
fn layer_group_passthrough_has_no_intermediate_and_matches_plain_pixels() {
    let mut grouped = scene(VideoBlendMode::Normal, None);
    grouped.layer_groups[0].opacity = 1.0;
    let mut plain = grouped.clone();
    plain.layer_groups.clear();
    let mut plain_renderer = renderer();
    let mut grouped_renderer = renderer();
    assert_eq!(
        display(&mut plain_renderer, &plain, true),
        display(&mut grouped_renderer, &grouped, true)
    );
    assert_eq!(plain_renderer.frame_provider.decodes, 3);
    assert_eq!(grouped_renderer.frame_provider.decodes, 3);
    let plan = build_composition_plans(&grouped).remove(0);
    assert!(grouped_renderer
        .active_layer_groups(&grouped, &plan, &mut Vec::new())
        .is_empty());
}

#[test]
fn layer_group_single_normal_child_folds_without_allocating_or_uploading_an_intermediate() {
    let mut snapshot = scene(VideoBlendMode::Screen, None);
    snapshot.compositions[0].layer_ids = vec![1, 2];
    snapshot.layers.pop();
    snapshot.layer_groups[0].layer_ids = vec![2];
    let mut renderer = renderer();
    let VideoOutputDisplayRender::GpuComposition(gpu) = display(&mut renderer, &snapshot, true)
    else {
        panic!("GPU")
    };
    assert_eq!(gpu.layers.len(), 2);
    assert_eq!(renderer.frame_provider.decodes, 2);
    let original = renderer
        .runtime
        .select_frames_for_plan(&build_composition_plans(&snapshot)[0]);
    assert!(Arc::ptr_eq(&original[1], &gpu.layers[1].frame));
    assert_eq!(gpu.layers[1].plan.opacity, 0.55);
    assert_eq!(gpu.layers[1].plan.blend_mode, VideoBlendMode::Screen);
}

#[test]
fn layer_group_solo_enabled_and_bypass_filter_program_and_preview_identically() {
    let mut snapshot = scene(VideoBlendMode::Normal, None);
    snapshot.layer_groups[0].solo = true;
    assert_eq!(
        build_composition_plans(&snapshot)[0]
            .layers
            .iter()
            .map(|l| l.layer_id)
            .collect::<Vec<_>>(),
        vec![2, 3]
    );
    let preview = renderer().render(&snapshot, SIZE.0, SIZE.1).unwrap();
    let program = renderer().render_output(&snapshot, 6).unwrap();
    assert_eq!(preview.data, program.data);
    snapshot.layer_groups[0].bypassed = true;
    snapshot.layer_groups[0].opacity = 0.0;
    assert_eq!(build_composition_plans(&snapshot)[0].layers.len(), 3);
    let mut plain = snapshot.clone();
    plain.layer_groups.clear();
    assert_eq!(
        renderer().render(&snapshot, SIZE.0, SIZE.1).unwrap().data,
        renderer().render(&plain, SIZE.0, SIZE.1).unwrap().data
    );
    snapshot.layer_groups[0].enabled = false;
    assert_eq!(
        build_composition_plans(&snapshot)[0]
            .layers
            .iter()
            .map(|l| l.layer_id)
            .collect::<Vec<_>>(),
        vec![1]
    );
}

#[test]
fn layer_group_transform_moves_the_complete_group_image() {
    let snapshot = scene(
        VideoBlendMode::Normal,
        Some(Transform2D {
            x: 0.5,
            ..Transform2D::default()
        }),
    );
    let (_, cpu) = parity_fixture(&snapshot);
    assert_eq!(&cpu.data[..4], &[32, 80, 160, 255]);
    assert_ne!(
        &cpu.data[SIZE.0 as usize * 2..SIZE.0 as usize * 2 + 4],
        &[32, 80, 160, 255]
    );
}

#[test]
fn layer_group_fx_runs_once_on_the_group_composite() {
    IsfGpuRuntime::new().expect("layer-group FX test requires a GPU adapter");
    let mut snapshot = scene(VideoBlendMode::Normal, None);
    snapshot.layer_groups[0].opacity = 1.0;
    // A nonlinear effect distinguishes a composite-image chain from a chain
    // incorrectly applied to every child before the children overlap.
    snapshot.layers[1].state.opacity = 0.5;
    let effect = builtin_isf_effect("posterize").unwrap().unwrap();
    snapshot.effect_chains.push(VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope: VideoEffectScope::Group {
            group_id: VideoLayerGroupId(7),
        },
        bypassed: false,
        stages: vec![protocol::VideoEffectStageSummary {
            id: VideoEffectStageId(1),
            enabled: true,
            label: "Posterize group".into(),
            effect: protocol::VideoEffectSummary {
                id: VideoEffectId(1),
                kind: VideoEffectKind::Isf { effect },
            },
        }],
    });
    let mut renderer = renderer();
    let rendered = renderer.render(&snapshot, SIZE.0, SIZE.1).unwrap();
    assert!(renderer.last_effect_stage_faults.is_empty());
    // Applying Posterize to children separately is not equivalent when their
    // coverage overlaps, so compare directly against the flattened variant.
    let mut each_child = snapshot.clone();
    let mut chain = each_child.effect_chains.remove(0);
    chain.scope = VideoEffectScope::Layer { layer_id: 2 };
    each_child.effect_chains.push(chain.clone());
    chain.id = VideoEffectChainId(2);
    chain.scope = VideoEffectScope::Layer { layer_id: 3 };
    each_child.effect_chains.push(chain);
    each_child.layer_groups.clear();
    let separate = self::renderer()
        .render(&each_child, SIZE.0, SIZE.1)
        .unwrap();
    assert_ne!(rendered.data, separate.data);
}
