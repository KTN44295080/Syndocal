fn image_blend_effect(layer_id: Option<VideoLayerId>) -> VideoIsfEffectSummary {
    let source = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"other","TYPE":"image"}]}*/
void main() {
    gl_FragColor = vec4(mix(IMG_THIS_NORM_PIXEL(inputImage).rgb,
        IMG_THIS_NORM_PIXEL(other).rgb, 0.5), 1.0);
}"#;
    let prepared = prepare_isf_shader(source).expect("two-image fixture prepares");
    let mut effect = isf_effect_from_prepared("T32 fixture".into(), source, None, &prepared);
    if let Some(layer_id) = layer_id {
        effect.image_bindings.push(protocol::VideoIsfImageBinding {
            name: "other".into(),
            source: protocol::VideoIsfImageSource::Layer { layer_id },
        });
    }
    effect
}

fn image_binding_snapshot() -> VideoSnapshot {
    let mut first = c1_test_layer(1, VideoBlendMode::Normal);
    first.isf_effect = Some(image_blend_effect(Some(2)));
    let mut second = c1_test_layer(2, VideoBlendMode::Normal);
    second.isf_effect = builtin_isf_effect("invert").unwrap();
    let mut snapshot = c1_test_snapshot(vec![first, second]);
    // The sampled layer need not be a visible composition member.
    snapshot.compositions[0].layer_ids = vec![1];
    snapshot
}

fn image_binding_renderer() -> VideoPreviewRenderer<C1SolidFrameProvider> {
    VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        C1SolidFrameProvider {
            pixels: HashMap::from([(1, [255, 0, 0, 255]), (2, [0, 0, 255, 255])]),
            fail: false,
        },
    )
}

fn assert_image_pixel(frame: &VideoFrame, pixel: [u8; 4]) {
    assert_eq!(
        frame.data.len(),
        frame.width as usize * frame.height as usize * 4
    );
    for actual in frame.data.chunks_exact(4) {
        for (actual, expected) in actual.iter().zip(pixel) {
            assert!(actual.abs_diff(expected) <= 1, "{actual:?} != {expected:?}");
        }
    }
}

#[test]
fn isf_layer_images_use_post_fx_outputs_and_survive_layer_reordering() {
    let mut snapshot = image_binding_snapshot();
    let mut renderer = image_binding_renderer();
    let first = renderer.render_output(&snapshot, 80).unwrap();
    assert!(renderer.last_effect_stage_faults().is_empty());
    assert_image_pixel(&first, [255, 128, 0, 255]);

    snapshot.layers.reverse();
    let reordered = renderer.render_output(&snapshot, 80).unwrap();
    assert_eq!(first.data, reordered.data);
    let thumbnail = renderer.render_layer_preview(&snapshot, 1, 1, 1).unwrap();
    assert_eq!(first.data, thumbnail.data);
}

#[test]
fn isf_layer_images_reject_removed_layers_instead_of_sampling_old_frames() {
    let mut snapshot = image_binding_snapshot();
    let mut renderer = image_binding_renderer();
    renderer.render_output(&snapshot, 80).unwrap();
    snapshot.layers.retain(|layer| layer.id != 2);
    match renderer.render_output(&snapshot, 80) {
        Err(VideoPreviewError::InvalidIsfImageBindings(message)) => {
            assert!(message.contains("missing layer 2"), "{message}");
        }
        result => panic!("removed image input must fail: {result:?}"),
    }
    assert!(renderer.isf_frame_inputs.layers.is_empty());
}

#[test]
fn isf_layer_images_reject_direct_and_indirect_cycles() {
    let mut snapshot = image_binding_snapshot();
    snapshot.layers[1].isf_effect = Some(image_blend_effect(Some(1)));
    let mut renderer = image_binding_renderer();
    assert!(matches!(
        renderer.render_output(&snapshot, 80),
        Err(VideoPreviewError::InvalidIsfImageBindings(message)) if message.contains("cycle")
    ));
    snapshot.layers[0].isf_effect = Some(image_blend_effect(Some(1)));
    assert!(matches!(
        renderer.render_output(&snapshot, 80),
        Err(VideoPreviewError::InvalidIsfImageBindings(message)) if message.contains("cycle")
    ));
}

#[test]
fn isf_layer_images_none_is_transparent_and_binding_changes_take_effect() {
    let mut snapshot = image_binding_snapshot();
    let mut renderer = image_binding_renderer();
    assert_image_pixel(
        &renderer.render_output(&snapshot, 80).unwrap(),
        [255, 128, 0, 255],
    );
    snapshot.layers[0].isf_effect = Some(image_blend_effect(None));
    assert_image_pixel(
        &renderer.render_output(&snapshot, 80).unwrap(),
        [128, 0, 0, 255],
    );
    snapshot.layers[0].isf_effect = Some(image_blend_effect(Some(2)));
    renderer
        .frame_provider_mut()
        .pixels
        .insert(2, [255, 0, 255, 255]);
    assert_image_pixel(
        &renderer.render_output(&snapshot, 80).unwrap(),
        [128, 128, 0, 255],
    );
}

#[test]
fn isf_layer_images_propagate_dependency_stage_faults() {
    let mut snapshot = image_binding_snapshot();
    let mut renderer = image_binding_renderer();
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        clip_runtime: &clip_runtime,
        project_render_epoch: 32,
    };
    assert_eq!(
        renderer
            .render_output_with_effects_evidenced(&snapshot, context, 80)
            .unwrap()
            .evidence
            .freshness,
        VideoOutputRenderFreshness::Fresh
    );
    snapshot.layers[1].isf_effect = Some(c1_invalid_effect("broken dependency"));
    let result = renderer
        .render_output_with_effects_evidenced(&snapshot, context, 80)
        .unwrap();
    // Stage-local faults retain the produced diagnostic frame and report Error;
    // LastValid is reserved for renders that cannot produce a frame.
    assert_eq!(result.evidence.freshness, VideoOutputRenderFreshness::Error);
    assert!(result.evidence.error.is_some());
    assert_image_pixel(&result.frame, [255, 0, 0, 255]);
    assert!(renderer.last_effect_stage_faults().iter().any(|fault| {
        matches!(fault.scope, VideoEffectScope::Layer { layer_id: 1 })
            && fault.message.contains("layer 2 failed to render")
    }));
}

#[test]
fn isf_date_injection_and_generator_use_host_output_dimensions() {
    let source = r#"/*{"INPUTS":[]}*/
void main() {
    gl_FragColor = vec4(DATE.x / 4096.0, RENDERSIZE.x / 4.0, DATE.w / 8192.0, 1.0);
}"#;
    let prepared = prepare_isf_shader(source).unwrap();
    let mut layer = c1_test_layer(1, VideoBlendMode::Normal);
    layer.isf_effect = Some(isf_effect_from_prepared(
        "T32 fixture".into(),
        source,
        None,
        &prepared,
    ));
    let snapshot = c1_test_snapshot(vec![layer]);
    let mut renderer = image_binding_renderer();
    renderer.set_isf_date_override(Some([2048.0, 8.0, 16.0, 4096.0]));
    assert_image_pixel(
        &renderer.render_output_preview(&snapshot, 80, 2, 1).unwrap(),
        [128, 128, 128, 255],
    );
    renderer.set_isf_date_override(Some([1024.0, 8.0, 16.0, 2048.0]));
    assert_image_pixel(
        &renderer.render_output_preview(&snapshot, 80, 1, 1).unwrap(),
        [64, 64, 64, 255],
    );
}

#[test]
fn isf_layer_image_depth_bound_is_independent_of_root_order() {
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let bound = protocol::MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH as u64;
    for count in [bound + 1, bound + 2] {
        let layers = (1..=count)
            .map(|id| {
                let mut layer = c1_test_layer(id, VideoBlendMode::Normal);
                if id > 1 {
                    layer.isf_effect = Some(image_blend_effect(Some(id - 1)));
                }
                layer
            })
            .collect::<Vec<_>>();
        let mut snapshot = c1_test_snapshot(layers);
        let requested = (1..=count).collect::<Vec<_>>();
        for reverse in [false, true] {
            if reverse {
                snapshot.layers.reverse();
            }
            let result = isf_layer_inputs::image_render_order(
                &snapshot,
                &clip_runtime,
                &requested,
                None,
                None,
                None,
            );
            if count == bound + 1 {
                assert_eq!(result.unwrap().0.len(), count as usize);
            } else {
                assert!(result.unwrap_err().contains("dependency depth"));
            }
        }
    }
}

#[test]
fn isf_scope_layer_images_match_preview_output_and_keep_thumbnails_layer_only() {
    for scope in [
        VideoEffectScope::Composition { composition_id: 70 },
        VideoEffectScope::Output { output_id: 80 },
    ] {
        let composition_scope = matches!(&scope, VideoEffectScope::Composition { .. });
        let mut snapshot = image_binding_snapshot();
        snapshot.layers[0].isf_effect = None;
        snapshot.effect_chains.push(c1_chain(
            321,
            scope,
            false,
            vec![image_blend_effect(Some(2))],
        ));
        let clip_runtime = VideoClipRuntimeSnapshot::default();
        let context = VideoEffectRenderContext {
            clip_runtime: &clip_runtime,
            project_render_epoch: 32,
        };
        let mut renderer = image_binding_renderer();
        let output = renderer
            .render_output_with_effects(&snapshot, context, 80)
            .unwrap();
        let preview = renderer
            .render_output_preview_with_effects(&snapshot, context, 80, 1, 1)
            .unwrap();
        assert_image_pixel(&output, [255, 128, 0, 255]);
        assert_eq!(output.data, preview.data);
        assert!(renderer.last_effect_stage_faults().is_empty());
        let thumbnail = renderer.render_layer_preview(&snapshot, 1, 1, 1).unwrap();
        assert_image_pixel(&thumbnail, [255, 0, 0, 255]);
        let legacy = renderer.render_output(&snapshot, 80).unwrap();
        let legacy_preview = renderer.render_output_preview(&snapshot, 80, 1, 1).unwrap();
        assert_eq!(legacy.data, legacy_preview.data);
        assert_image_pixel(
            &legacy,
            if composition_scope {
                [255, 128, 0, 255]
            } else {
                [255, 0, 0, 255]
            },
        );
        let composition = renderer.render(&snapshot, 1, 1).unwrap();
        assert_eq!(legacy.data, composition.data);
    }
}

#[test]
fn isf_scope_image_dependencies_do_not_cross_compositions_with_shared_layers() {
    let mut snapshot = image_binding_snapshot();
    snapshot.layers[0].isf_effect = None;
    let mut other_composition = snapshot.compositions[0].clone();
    other_composition.id = 71;
    other_composition.output_ids.clear();
    snapshot.compositions.push(other_composition);
    snapshot.effect_chains.push(c1_chain(
        322,
        VideoEffectScope::Composition { composition_id: 71 },
        false,
        vec![image_blend_effect(Some(999))],
    ));
    let clips = VideoClipRuntimeSnapshot::default();
    let (order, sampled) =
        isf_layer_inputs::image_render_order(&snapshot, &clips, &[1], Some(80), Some(70), None)
            .unwrap();
    assert_eq!(order, [1]);
    assert!(sampled.is_empty());
    assert!(
        isf_layer_inputs::image_render_order(&snapshot, &clips, &[1], None, Some(71), None,)
            .unwrap_err()
            .contains("missing layer 999")
    );
}

#[test]
fn isf_scope_image_dependencies_only_include_active_visible_layer_buses() {
    let mut snapshot = image_binding_snapshot();
    snapshot.layers[0].isf_effect = None;
    snapshot.compositions[0].layer_ids = vec![1, 2];
    let bus_id = protocol::VideoTransitionBusId(32);
    let from = VideoLayerTransitionTarget::Layer { layer_id: 1 };
    let to = VideoLayerTransitionTarget::Layer { layer_id: 2 };
    snapshot
        .transition_buses
        .push(protocol::VideoLayerTransitionBusSummary {
            id: bus_id,
            label: "T32 bus".into(),
            composition_id: 70,
            enabled: true,
            members: vec![from.clone(), to.clone()],
            default_from: from.clone(),
            default_to: to.clone(),
            default_kind: protocol::VideoClipTakeKind::Crossfade,
            default_duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
            default_curve: VideoLayerTransitionCurve::Linear,
            matte_source: None,
        });
    snapshot.effect_chains.push(c1_chain(
        323,
        VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::LayerBus { bus_id },
        },
        false,
        vec![image_blend_effect(Some(999))],
    ));
    let clips = VideoClipRuntimeSnapshot::default();
    let idle = VideoLayerTransitionRuntimeSnapshot::default();
    for runtime in [None, Some(&idle)] {
        let (order, sampled) = isf_layer_inputs::image_render_order(
            &snapshot,
            &clips,
            &[1],
            Some(80),
            Some(70),
            runtime,
        )
        .unwrap();
        assert_eq!(order, [1]);
        assert!(sampled.is_empty());
    }
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
    assert!(isf_layer_inputs::image_render_order(
        &snapshot,
        &clips,
        &[1],
        Some(80),
        Some(70),
        Some(&active),
    )
    .unwrap_err()
    .contains("missing layer 999"));
    let (order, sampled) = isf_layer_inputs::image_render_order(
        &snapshot,
        &clips,
        &[],
        Some(80),
        Some(70),
        Some(&active),
    )
    .unwrap();
    assert!(order.is_empty());
    assert!(sampled.is_empty());
}
