use super::*;
use std::{cell::Cell, rc::Rc};

struct StopDuringDecode {
    cancelled: Rc<Cell<bool>>,
    calls: Rc<Cell<usize>>,
}

impl VideoFrameDecoder for StopDuringDecode {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.calls.set(self.calls.get() + 1);
        if self.cancelled.get() {
            return Err(VideoDecodeError::Cancelled);
        }
        Ok(Some(VideoFrame {
            layer_id: request.layer_id,
            width: request.width,
            height: request.height,
            pts_ms: request.position_ms,
            duration_ms: 33,
            format: VideoPixelFormat::Rgba8,
            data: [11, 77, 203, 255].repeat((request.width * request.height) as usize),
        }))
    }
}

fn cancellation_output(layer_count: u64) -> VideoSnapshot {
    VideoSnapshot {
        layers: (1..=layer_count)
            .map(|id| VideoLayerSummary {
                id,
                label: format!("Cancellation {id}"),
                source: VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some(format!("stop-{id}.mkv")),
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
            id: 1,
            label: "Program".into(),
            layer_ids: (1..=layer_count).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![1],
        }],
        outputs: vec![VideoOutputSummary {
            id: 1,
            label: "Display".into(),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id: 1,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 16,
            height: 16,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }],
        ..VideoSnapshot::default()
    }
}

#[test]
fn cancelled_output_never_emits_last_valid_or_first_frame_black_fallback() {
    let snapshot = cancellation_output(1);
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };
    for warm_cache in [false, true] {
        let cancelled = Rc::new(Cell::new(false));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            DecoderBackedFrameProvider::new(StopDuringDecode {
                cancelled: cancelled.clone(),
                calls: Rc::new(Cell::new(0)),
            })
            .with_prefetch(0, 33),
        );
        if warm_cache {
            let fresh = renderer
                .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                    &snapshot,
                    context,
                    &transitions,
                    1,
                    16,
                    16,
                )
                .unwrap();
            assert_eq!(fresh.evidence.freshness, VideoOutputRenderFreshness::Fresh);
            assert_eq!(renderer.output_last_valid_frames.len(), 1);
        }
        cancelled.set(true);
        let result = renderer
            .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                &snapshot,
                context,
                &transitions,
                1,
                16,
                16,
            );
        assert_eq!(
            result,
            Err(VideoPreviewError::Cancelled),
            "warm_cache={warm_cache}"
        );
        assert_eq!(
            renderer.output_last_valid_frames.len(),
            usize::from(warm_cache),
            "cancellation cannot publish a new cache entry"
        );
    }
}

#[test]
fn cancelled_gpu_preparation_never_retries_the_inputs_through_cpu_composition() {
    let snapshot = cancellation_output(2);
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };
    let cancelled = Rc::new(Cell::new(true));
    let calls = Rc::new(Cell::new(0));
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(StopDuringDecode {
            cancelled,
            calls: calls.clone(),
        })
        .with_prefetch(0, 33),
    );
    let result = renderer.prepare_output_display_render_preview_with_effects_and_transitions(
        &snapshot,
        context,
        &VideoLayerTransitionRuntimeSnapshot::default(),
        1,
        16,
        16,
        Some(VideoOutputGpuSupport {
            max_layer_bytes: u64::MAX,
        }),
    );
    assert_eq!(result, Err(VideoPreviewError::Cancelled));
    assert_eq!(
        calls.get(),
        2,
        "each input is demanded once by the GPU attempt; cancellation cannot start a CPU retry"
    );
    assert!(renderer.output_last_valid_frames.is_empty());
}

#[test]
fn cancellable_output_preview_fails_closed_before_starting_render_work() {
    let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
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
