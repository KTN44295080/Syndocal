fn shared_input() -> Arc<VideoFrame> {
    Arc::new(VideoFrame {
        layer_id: 1,
        width: 1,
        height: 1,
        pts_ms: 123,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: vec![10, 20, 30, 255],
    })
}

fn resolved(effects: Vec<VideoIsfEffectSummary>) -> ResolvedVideoEffectChain {
    resolved_chain_from_canonical(&c1_chain(
        201,
        VideoEffectScope::Layer { layer_id: 1 },
        false,
        effects,
    ))
    .unwrap()
}

fn renderer() -> VideoPreviewRenderer<C1SolidFrameProvider> {
    VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        C1SolidFrameProvider::default(),
    )
}

#[test]
fn no_op_and_prepare_failures_preserve_pixels_and_defer_ownership() {
    let source = shared_input();
    let original = source.as_ref().clone();
    let mut bypassed = resolved(vec![c1_invalid_effect("bypassed")]);
    bypassed.bypassed = true;
    let mut disabled = c1_invalid_effect("disabled");
    disabled.enabled = false;
    for (chain, expected_faults) in [
        (resolved(Vec::new()), 0),
        (bypassed, 0),
        (resolved(vec![disabled]), 0),
        (
            resolved(vec![
                c1_invalid_effect("first"),
                c1_invalid_effect("second"),
            ]),
            2,
        ),
    ] {
        let (borrowed, faults) = renderer()
            .apply_resolved_effect_chain_cow_with_cancellation(
                &chain,
                Cow::Borrowed(source.as_ref()),
                None,
                None,
            )
            .unwrap();
        assert_eq!(borrowed.as_ref(), &original);
        assert!(std::ptr::eq(borrowed.as_ref(), source.as_ref()));
        assert_eq!(faults.len(), expected_faults);
        for (index, fault) in faults.iter().enumerate() {
            assert_eq!(fault.scope, chain.scope);
            assert_eq!(fault.chain_id, chain.chain_id);
            assert_eq!(fault.stage_index, Some(index));
            assert_eq!(
                fault.stage_id,
                Some(VideoEffectStageId(20_101 + index as u64))
            );
        }
        let owned = original.clone();
        let allocation = owned.data.as_ptr();
        let (returned, owned_faults) = renderer()
            .apply_resolved_effect_chain_to_frame_with_cancellation(&chain, owned, None, None)
            .unwrap();
        assert_eq!(returned, original);
        assert_eq!(returned.data.as_ptr(), allocation);
        assert_eq!(owned_faults, faults);
    }
    assert_eq!(source.as_ref(), &original);
    assert_eq!(Arc::strong_count(&source), 1);
}

#[test]
fn all_runtime_failures_preserve_shared_pixels_faults_and_frame_index() {
    let source = shared_input();
    let original = source.as_ref().clone();
    let invert = builtin_isf_effect("invert").unwrap().unwrap();
    let chain = resolved(vec![invert.clone(), invert]);
    let mut borrowed_renderer = renderer();
    let mut owned_renderer = renderer();
    for renderer in [&mut borrowed_renderer, &mut owned_renderer] {
        renderer.set_isf_date_override(Some([2026.0, 13.0, 22.0, 0.0]));
    }
    let (borrowed, faults) = borrowed_renderer
        .apply_resolved_effect_chain_cow_with_cancellation(
            &chain,
            Cow::Borrowed(source.as_ref()),
            None,
            None,
        )
        .unwrap();
    let (owned, owned_faults) = owned_renderer
        .apply_resolved_effect_chain_to_frame_with_cancellation(
            &chain,
            original.clone(),
            None,
            None,
        )
        .unwrap();
    assert_eq!(borrowed.as_ref(), &original);
    assert_eq!(owned, original);
    assert_eq!(faults, owned_faults);
    assert_eq!(faults.len(), 2);
    for (index, fault) in faults.iter().enumerate() {
        assert_eq!(fault.stage_index, Some(index));
        assert!(fault.message.contains("DATE"));
    }
    const CLOCK: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main() {
            vec4 pixel = IMG_THIS_PIXEL(inputImage);
            pixel.r = float(FRAMEINDEX) / 255.0;
            gl_FragColor = pixel;
        }"#;
    let clock = isf_effect_from_prepared(
        "clock".into(),
        CLOCK,
        None,
        &prepare_isf_shader(CLOCK).unwrap(),
    );
    for renderer in [&mut borrowed_renderer, &mut owned_renderer] {
        renderer.set_isf_date_override(Some([2026.0, 10.0, 22.0, 0.0]));
        let (output, faults) = renderer
            .apply_resolved_effect_chain_cow_with_cancellation(
                &resolved(vec![clock.clone()]),
                Cow::Borrowed(source.as_ref()),
                None,
                None,
            )
            .unwrap();
        assert_eq!(output.data, [1, 20, 30, 255]);
        assert!(faults.is_empty());
    }
    assert_eq!(source.as_ref(), &original);
    assert_eq!(Arc::strong_count(&source), 1);
}

struct SharedProvider(Arc<VideoFrame>);

impl VideoFrameProvider for SharedProvider {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn frame_for_layer(
        &mut self,
        _layer: &VideoLayerSummary,
        _width: u32,
        _height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        panic!("the live path must use the shared provider");
    }

    fn frames_for_layer_shared(
        &mut self,
        _layer: &VideoLayerSummary,
        _width: u32,
        _height: u32,
    ) -> Result<Vec<Arc<VideoFrame>>, VideoFrameProviderError> {
        Ok(vec![Arc::clone(&self.0)])
    }
}

#[test]
fn shared_clip_then_layer_preserves_order_fault_identity_and_decoder_input() {
    const ADD_RED: &str = r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
        void main() {
            vec4 pixel = IMG_THIS_PIXEL(inputImage);
            pixel.r += 10.0 / 255.0;
            gl_FragColor = pixel;
        }"#;
    let increment = isf_effect_from_prepared(
        "add red".into(),
        ADD_RED,
        None,
        &prepare_isf_shader(ADD_RED).unwrap(),
    );
    let clips = VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 1,
            active_slot_id: Some(VideoClipSlotId(2)),
            ..protocol::VideoClipLayerRuntimeSummary::default()
        }],
        ..VideoClipRuntimeSnapshot::default()
    };
    for (failure, missing_projection) in [(false, false), (true, false), (false, true)] {
        let source = shared_input();
        let original = source.as_ref().clone();
        let mut snapshot = c1_test_snapshot(vec![c1_test_layer(1, VideoBlendMode::Normal)]);
        let invert = builtin_isf_effect("invert").unwrap().unwrap();
        if !missing_projection {
            snapshot.layers[0].isf_effect = Some(invert.clone());
        }
        let mut clip_stages = Vec::new();
        if failure {
            clip_stages.push(c1_invalid_effect("broken clip stage"));
        }
        clip_stages.push(increment.clone());
        snapshot.effect_chains.extend([
            c1_chain(
                202,
                VideoEffectScope::Clip {
                    layer_id: 1,
                    slot_id: VideoClipSlotId(2),
                },
                false,
                clip_stages,
            ),
            c1_chain(
                203,
                VideoEffectScope::Layer { layer_id: 1 },
                false,
                vec![invert],
            ),
        ]);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            SharedProvider(Arc::clone(&source)),
        );
        let output = renderer
            .render_output_with_effects_evidenced(
                &snapshot,
                VideoEffectRenderContext {
                    audio_frame: None,
                    clip_runtime: &clips,
                    project_render_epoch: 1,
                },
                80,
            )
            .unwrap();
        // Reversing Clip/Layer would produce red=255, not 235.
        assert_eq!(
            output.frame.data,
            if missing_projection {
                [20, 20, 30, 255]
            } else {
                [235, 235, 225, 255]
            }
        );
        assert_eq!(
            renderer.last_effect_stage_faults().len(),
            usize::from(failure || missing_projection)
        );
        if failure {
            let fault = &renderer.last_effect_stage_faults()[0];
            assert_eq!(
                fault.scope,
                VideoEffectScope::Clip {
                    layer_id: 1,
                    slot_id: VideoClipSlotId(2)
                }
            );
            assert_eq!(fault.stage_id, Some(VideoEffectStageId(20_201)));
            assert_eq!(fault.stage_index, Some(0));
        }
        if missing_projection {
            let fault = &renderer.last_effect_stage_faults()[0];
            assert_eq!(fault.scope, VideoEffectScope::Layer { layer_id: 1 });
            assert_eq!(fault.chain_id, Some(VideoEffectChainId(203)));
            assert_eq!(fault.stage_index, None);
            assert!(fault
                .message
                .contains("missing its rendered legacy projection"));
        }
        if failure || missing_projection {
            assert_eq!(output.evidence.freshness, VideoOutputRenderFreshness::Error);
            assert!(renderer.output_last_valid_frames.is_empty());
        }
        assert_eq!(source.as_ref(), &original);
        assert_eq!(
            Arc::strong_count(&source),
            2,
            "no source reference retained by FX"
        );
    }
}

#[test]
fn borrowed_chain_cancellation_before_and_after_gpu_preserves_source() {
    use std::cell::Cell;
    let source = shared_input();
    let original = source.as_ref().clone();
    let chain = resolved(vec![builtin_isf_effect("invert").unwrap().unwrap()]);
    for stop_at in [1, 4] {
        let calls = Cell::new(0);
        let cancellation = || {
            calls.set(calls.get() + 1);
            calls.get() >= stop_at
        };
        let mut renderer = renderer();
        let result = renderer.apply_resolved_effect_chain_cow_with_cancellation(
            &chain,
            Cow::Borrowed(source.as_ref()),
            None,
            Some(&cancellation),
        );
        assert!(matches!(result, Err(VideoRuntimeError::Cancelled)));
        assert_eq!(calls.get(), stop_at);
        assert_eq!(source.as_ref(), &original);
        assert!(renderer.output_last_valid_frames.is_empty());
        assert_eq!(renderer.queue_len(1), 0);
    }
    assert_eq!(Arc::strong_count(&source), 1);
}
