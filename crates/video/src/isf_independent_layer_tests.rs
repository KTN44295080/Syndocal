use std::collections::HashMap;

struct BatchLayerProvider {
    inputs: HashMap<VideoLayerId, Arc<VideoFrame>>,
    calls: Vec<VideoLayerId>,
    fail_at: Option<VideoLayerId>,
    cancel_at: Option<VideoLayerId>,
}

impl VideoFrameProvider for BatchLayerProvider {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}

    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        _: u32,
        _: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        Ok(self.inputs[&layer.id].as_ref().clone())
    }

    fn frames_for_layer_shared(
        &mut self,
        layer: &VideoLayerSummary,
        _: u32,
        _: u32,
    ) -> Result<Vec<Arc<VideoFrame>>, VideoFrameProviderError> {
        self.calls.push(layer.id);
        if self.cancel_at == Some(layer.id) {
            return Err(VideoFrameProviderError::Cancelled);
        }
        if self.fail_at == Some(layer.id) {
            return Err(VideoFrameProviderError::Decode {
                layer_id: layer.id,
                label: layer.label.clone(),
                error: VideoDecodeError::Decode {
                    layer_id: layer.id,
                    label: layer.label.clone(),
                    message: "batch integration decode failure".into(),
                },
            });
        }
        Ok(vec![Arc::clone(&self.inputs[&layer.id])])
    }

    fn frames_for_layer_shared_with_cancellation(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Vec<Arc<VideoFrame>>, VideoFrameProviderError> {
        if cancellation.is_cancelled() {
            return Err(VideoFrameProviderError::Cancelled);
        }
        self.frames_for_layer_shared(layer, width, height)
    }
}

fn batch_effect(label: &str, source: &str) -> VideoIsfEffectSummary {
    isf_effect_from_prepared(
        label.into(),
        source,
        None,
        &prepare_isf_shader(source).unwrap(),
    )
}

fn projection(effects: &[VideoIsfEffectSummary]) -> VideoIsfEffectSummary {
    let mut root = effects[0].clone();
    root.stack = effects[1..]
        .iter()
        .map(|effect| protocol::VideoIsfEffectStageSummary {
            enabled: effect.enabled,
            label: effect.label.clone(),
            source: Arc::clone(&effect.source),
            source_path: effect.source_path.clone(),
            description: effect.description.clone(),
            categories: effect.categories.clone(),
            controls: effect.controls.clone(),
            image_bindings: effect.image_bindings.clone(),
        })
        .collect();
    root
}

fn batch_snapshot(fail: u8) -> VideoSnapshot {
    let clock = batch_effect(
        "clock",
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){ vec4 p=IMG_THIS_PIXEL(inputImage); p.r=float(FRAMEINDEX)/255.0; gl_FragColor=p; }"#,
    );
    let blue = batch_effect(
        "blue",
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){ vec4 p=IMG_THIS_PIXEL(inputImage); p.b+=5.0/255.0; gl_FragColor=p; }"#,
    );
    let invert = builtin_isf_effect("invert").unwrap().unwrap();
    let mut snapshot = c1_test_snapshot(
        (1..=3)
            .map(|id| c1_test_layer(id, VideoBlendMode::Normal))
            .collect(),
    );
    for layer in &mut snapshot.layers {
        let mut effects = vec![clock.clone(), blue.clone(), invert.clone()];
        if layer.id == 2 && fail == 1 {
            effects[1] = c1_invalid_effect("bad middle");
        } else if layer.id == 2 && fail == 2 {
            effects = (0..3)
                .map(|i| c1_invalid_effect(&format!("bad {i}")))
                .collect();
        }
        layer.isf_effect = Some(projection(&effects));
        snapshot.effect_chains.push(c1_chain(
            500 + layer.id,
            VideoEffectScope::Layer { layer_id: layer.id },
            false,
            effects,
        ));
    }
    snapshot
}

fn batch_layer_renderer() -> VideoPreviewRenderer<BatchLayerProvider> {
    let inputs = (1..=3)
        .map(|id| {
            (
                id,
                Arc::new(VideoFrame {
                    layer_id: id,
                    width: 1,
                    height: 1,
                    pts_ms: 0,
                    duration_ms: 16,
                    format: VideoPixelFormat::Rgba8,
                    data: vec![20 * id as u8, 30 * id as u8, 40 * id as u8, 255],
                }),
            )
        })
        .collect();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        BatchLayerProvider {
            inputs,
            calls: Vec::new(),
            fail_at: None,
            cancel_at: None,
        },
    );
    renderer.set_isf_date_override(Some([2026.0, 10.0, 25.0, 0.0]));
    renderer
}

fn batch_render(
    renderer: &mut VideoPreviewRenderer<BatchLayerProvider>,
    snapshot: &VideoSnapshot,
    cancellation: Option<&dyn VideoRenderCancellation>,
) -> Result<VideoOutputArtisticRenderResult, VideoPreviewError> {
    let clips = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let plan = build_video_output_render_plan(snapshot, 80).unwrap();
    renderer.isf_render_namespace = (51, Some(80), false);
    renderer.prepare_output_artistic_render_result_impl_with_cancellation(
        snapshot,
        VideoEffectRenderContext {
            audio_frame: None,
            clip_runtime: &clips,
            project_render_epoch: 51,
        },
        Some(&transitions),
        &plan,
        1,
        1,
        cancellation,
        None,
    )
}

fn batch_count(renderer: &VideoPreviewRenderer<BatchLayerProvider>) -> u64 {
    renderer
        .isf_runtime
        .as_ref()
        .and_then(|r| r.as_ref().ok())
        .map_or(0, IsfGpuRuntime::independent_batch_count)
}

#[test]
fn independent_layers_match_callback_sequential_pixels_faults_clocks_and_cache() {
    for fail in [0, 1, 2] {
        let snapshot = batch_snapshot(fail);
        let mut batched = batch_layer_renderer();
        let mut sequential = batch_layer_renderer();
        let originals = batched
            .frame_provider
            .inputs
            .iter()
            .map(|(id, frame)| (*id, frame.as_ref().clone()))
            .collect::<HashMap<_, _>>();
        for round in 0..2 {
            let actual = batch_render(&mut batched, &snapshot, None).unwrap();
            let expected = batch_render(&mut sequential, &snapshot, Some(&|| false)).unwrap();
            assert_eq!(actual, expected, "fail={fail} round={round}");
            assert_eq!(
                batched.last_effect_stage_faults,
                sequential.last_effect_stage_faults
            );
            assert_eq!(
                batched.output_last_valid_frames.len(),
                usize::from(fail == 0)
            );
            for id in 1..=3 {
                let a = batched
                    .runtime
                    .queues
                    .iter()
                    .find(|q| q.layer_id() == id)
                    .unwrap()
                    .frame_at_or_before(0)
                    .unwrap();
                let b = sequential
                    .runtime
                    .queues
                    .iter()
                    .find(|q| q.layer_id() == id)
                    .unwrap()
                    .frame_at_or_before(0)
                    .unwrap();
                assert_eq!(a, b);
                if id != 2 || fail != 2 {
                    assert_eq!(a.data[0], 255 - (round * 3 + id - 1) as u8);
                }
            }
            if fail != 0 {
                assert_eq!(actual.evidence.freshness, VideoOutputRenderFreshness::Error);
                for fault in &batched.last_effect_stage_faults {
                    assert_eq!(fault.scope, VideoEffectScope::Layer { layer_id: 2 });
                    assert_eq!(fault.chain_id, Some(VideoEffectChainId(502)));
                    if fail == 1 {
                        assert_eq!(fault.stage_index, Some(1));
                        assert_eq!(fault.stage_id, Some(VideoEffectStageId(50_202)));
                    }
                }
            }
        }
        assert_eq!(batched.frame_provider.calls, [1, 2, 3, 1, 2, 3]);
        assert_eq!(
            sequential.frame_provider.calls,
            batched.frame_provider.calls
        );
        assert_eq!(batch_count(&sequential), 0);
        assert_eq!(batch_count(&batched), if fail == 2 { 0 } else { 2 });
        for (id, source) in &batched.frame_provider.inputs {
            assert_eq!(source.as_ref(), &originals[id]);
            assert_eq!(
                Arc::strong_count(source),
                1,
                "pending input must be released"
            );
        }
    }
}

#[test]
fn independent_layers_provider_cancel_discards_unsubmitted_jobs_without_fallback() {
    let mut renderer = batch_layer_renderer();
    renderer.frame_provider.cancel_at = Some(2);
    let result = batch_render(&mut renderer, &batch_snapshot(0), None);
    assert_eq!(result, Err(VideoPreviewError::Cancelled));
    assert_eq!(renderer.frame_provider.calls, [1, 2]);
    assert_eq!(batch_count(&renderer), 0);
    assert!(
        renderer.isf_runtime.is_none(),
        "unsubmitted work must not start after cancel"
    );
    assert!(renderer.output_last_valid_frames.is_empty());
    assert_eq!(renderer.queue_len(1), 0);
    assert!(renderer
        .frame_provider
        .inputs
        .values()
        .all(|frame| Arc::strong_count(frame) == 1));
}

#[test]
fn independent_layers_decode_error_flushes_earlier_jobs_once_and_preserves_last_valid() {
    let snapshot = batch_snapshot(0);
    let mut batched = batch_layer_renderer();
    let mut sequential = batch_layer_renderer();
    assert_eq!(
        batch_render(&mut batched, &snapshot, None).unwrap(),
        batch_render(&mut sequential, &snapshot, Some(&|| false)).unwrap(),
    );
    batched.frame_provider.fail_at = Some(2);
    sequential.frame_provider.fail_at = Some(2);
    let actual = batch_render(&mut batched, &snapshot, None).unwrap();
    let expected = batch_render(&mut sequential, &snapshot, Some(&|| false)).unwrap();
    assert_eq!(actual, expected);
    assert_eq!(
        actual.evidence.freshness,
        VideoOutputRenderFreshness::LastValid
    );
    assert_eq!(batched.frame_provider.calls, [1, 2, 3, 1, 2]);
    assert_eq!(
        sequential.frame_provider.calls,
        batched.frame_provider.calls
    );
    assert_eq!(batch_count(&batched), 1);
    let a = batched.runtime.queues[0].frame_at_or_before(0).unwrap();
    let b = sequential.runtime.queues[0].frame_at_or_before(0).unwrap();
    assert_eq!(a, b, "successful preceding layer executes exactly once");
    assert_eq!(a.data[0], 252);
}
