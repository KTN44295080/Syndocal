use super::*;

#[derive(Default)]
struct SessionDecoder {
    cached: Vec<VideoFrame>,
    opens: usize,
    reuses: usize,
}

impl VideoFrameDecoder for SessionDecoder {
    fn retain_layers(&mut self, ids: &[VideoLayerId]) {
        self.cached.retain(|frame| ids.contains(&frame.layer_id));
    }

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        if let Some(frame) = self
            .cached
            .iter()
            .find(|frame| frame.layer_id == request.layer_id)
        {
            assert_eq!(
                frame.pts_ms, request.position_ms,
                "paused calls never look ahead"
            );
            self.reuses += 1;
            return Ok(Some(frame.clone()));
        }
        self.opens += 1;
        let frame = debug_solid_frame_for_layer(
            request.layer_id,
            request.position_ms,
            request.width,
            request.height,
        );
        self.cached.push(frame.clone());
        Ok(Some(frame))
    }
}

fn two_outputs() -> VideoSnapshot {
    let mut snapshot = c1_test_snapshot(vec![
        c1_test_layer(8, VideoBlendMode::Normal),
        c1_test_layer(9, VideoBlendMode::Normal),
    ]);
    snapshot.compositions[0].layer_ids = vec![8];
    let mut second = snapshot.compositions[0].clone();
    second.id = 71;
    second.layer_ids = vec![9];
    second.output_ids = vec![81];
    snapshot.compositions.push(second);
    let mut output = snapshot.outputs[0].clone();
    output.id = 81;
    output.composition_id = 71;
    snapshot.outputs.push(output);
    snapshot
}

fn render<D: VideoFrameDecoder>(
    renderer: &mut VideoPreviewRenderer<DecoderBackedFrameProvider<D>>,
    snapshot: &VideoSnapshot,
    output: VideoOutputId,
) -> VideoFrame {
    let warm = renderer
        .warm_output_decode_queue(snapshot, output, 32, 18, 1)
        .unwrap();
    assert!(warm.decode.errors.is_empty());
    renderer
        .render_output_preview_with_effects_and_transitions(
            snapshot,
            VideoEffectRenderContext {
                clip_runtime: &VideoClipRuntimeSnapshot::default(),
                project_render_epoch: 1,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            output,
            32,
            18,
        )
        .unwrap()
}

#[test]
fn output_preview_sessions_survive_alternating_paused_outputs_and_release_deleted_layers() {
    let mut snapshot = two_outputs();
    let provider = DecoderBackedFrameProvider::new(SessionDecoder::default()).with_prefetch(0, 33);
    let mut renderer =
        VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider);
    let a = render(&mut renderer, &snapshot, 80);
    let b = render(&mut renderer, &snapshot, 81);
    for _ in 0..20 {
        assert_eq!(render(&mut renderer, &snapshot, 80), a);
        assert_eq!(render(&mut renderer, &snapshot, 81), b);
    }
    assert_eq!(renderer.frame_provider().decoder().opens, 2);
    assert_eq!(renderer.frame_provider().decoder().reuses, 82);
    snapshot.layers.retain(|layer| layer.id == 8);
    render(&mut renderer, &snapshot, 80);
    assert_eq!(renderer.frame_provider().decoder().cached.len(), 1);
    assert_eq!(renderer.frame_provider().decoder().cached[0].layer_id, 8);
}

#[cfg(feature = "libav")]
#[test]
#[ignore = "requires SYNDOCAL_GPU_TEST_VIDEO and a supported GPU on Windows"]
fn output_preview_sessions_real_video_paused_and_forward_requests_reuse_two_sessions() {
    let path = std::env::var("SYNDOCAL_GPU_TEST_VIDEO").expect("set real video fixture path");
    let mut snapshot = two_outputs();
    for layer in &mut snapshot.layers {
        layer.source.path = Some(path.clone());
        layer.state.position_ms = 100;
    }
    let provider = DecoderBackedFrameProvider::new(
        crate::libav_decoder::LibavFrameDecoder::new(),
    )
    .with_prefetch(0, 33);
    let mut renderer =
        VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider);
    let a = render(&mut renderer, &snapshot, 80);
    let b = render(&mut renderer, &snapshot, 81);
    for _ in 0..20 {
        assert_eq!(render(&mut renderer, &snapshot, 80), a);
        assert_eq!(render(&mut renderer, &snapshot, 81), b);
    }
    for position in [134, 167, 200] {
        for layer in &mut snapshot.layers {
            layer.state.position_ms = position;
        }
        render(&mut renderer, &snapshot, 80);
        render(&mut renderer, &snapshot, 81);
    }
    let stats = renderer.frame_provider().decoder().session_diagnostics();
    assert_eq!(stats.active_sessions, 2);
    assert_eq!(stats.opens, 2);
    assert_eq!(stats.resets, 0);
    assert_eq!(stats.errors, 0);
    assert!(stats.frame_reuses >= 82);
    assert!(stats.sequential_continues > 0);
    #[cfg(target_os = "windows")]
    {
        assert_eq!(stats.hardware_sessions, 2);
        assert!(stats.hardware_frames > 0);
        assert_eq!(stats.hardware_errors, 0);
    }
    snapshot.layers.retain(|layer| layer.id == 8);
    render(&mut renderer, &snapshot, 80);
    assert_eq!(
        renderer
            .frame_provider()
            .decoder()
            .session_diagnostics()
            .active_sessions,
        1
    );
    eprintln!("two output paused/forward decoder evidence: {stats:?}");
}

#[test]
fn output_preview_sessions_invalid_other_output_transition_does_not_block_current_output() {
    let snapshot = two_outputs();
    let runtime = VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 9,
            transition: Some(protocol::VideoClipTakeTransitionSummary {
                origin_slot_id: VideoClipSlotId(20),
                outgoing_slot_id: VideoClipSlotId(20),
                incoming_slot_id: VideoClipSlotId(21),
                kind: protocol::VideoClipTakeKind::Custom,
                elapsed_ms: 250,
                duration_ms: 1_000,
                duration: protocol::VideoClipTakeDuration::milliseconds(1_000),
                progress_millis: 250,
                incoming_playhead_ms: 600,
                incoming_playing: true,
            }),
            ..protocol::VideoClipLayerRuntimeSummary::default()
        }],
        ..VideoClipRuntimeSnapshot::default()
    };
    let provider = DecoderBackedFrameProvider::new(SessionDecoder::default()).with_prefetch(0, 33);
    let mut renderer =
        VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider);
    renderer
        .prepare_frames_with_effects(&snapshot, &runtime, 1, &[8], 32, 18)
        .unwrap();
    assert!(matches!(
        renderer.prepare_frames_with_effects(&snapshot, &runtime, 1, &[9], 32, 18),
        Err(VideoPreviewError::MissingTransitionSlot { layer_id: 9, .. })
    ));
    assert_eq!(renderer.frame_provider().decoder().opens, 1);
}
