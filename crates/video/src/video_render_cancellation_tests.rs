use super::*;

#[test]
fn cancellable_output_preview_fails_closed_before_starting_render_work() {
    let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };
    let cancellation = || true;

    assert_eq!(
        renderer.render_output_preview_with_effects_and_transitions_cancellable(
            &VideoSnapshot::default(),
            context,
            &VideoLayerTransitionRuntimeSnapshot::default(),
            1,
            320,
            180,
            &cancellation,
        ),
        Err(VideoPreviewError::Cancelled)
    );
}

#[test]
fn cancellable_ffmpeg_decode_does_not_spawn_when_already_stopped() {
    let cancellation = || true;
    let request = VideoFrameRequest {
        layer_id: 1,
        label: "cancelled".to_string(),
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("missing-video.mov".to_string()),
            name: None,
            codec: None,
            metadata: None,
        },
        position_ms: 0,
        width: 1,
        height: 1,
    };

    assert_eq!(
        decode_ffmpeg_cli_frame_with_cancellation(
            &request,
            "definitely-not-started-ffmpeg",
            "missing-video.mov",
            &cancellation,
        ),
        Err(VideoDecodeError::Cancelled)
    );
}
