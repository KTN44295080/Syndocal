use super::*;
use crate::{
    VideoDecodeError, VideoFrameDecoder, VideoFrameRequest, VideoPixelFormat, VideoRenderInput,
    VideoRuntimeConfig,
};
use protocol::{VideoLayerState, VideoLayerSummary, VideoSourceKind, VideoSourceSummary};
use std::sync::{
    atomic::{AtomicBool, AtomicUsize, Ordering},
    Arc,
};

#[derive(Default)]
struct Calls {
    ordinary: AtomicUsize,
    cancellable: AtomicUsize,
}
struct Decoder {
    calls: Arc<Calls>,
    cancel: Arc<AtomicBool>,
    cancel_during_decode: bool,
    return_cancel_error: bool,
}
fn frame(request: &VideoFrameRequest) -> VideoFrame {
    VideoFrame {
        layer_id: request.layer_id,
        width: request.width,
        height: request.height,
        pts_ms: request.position_ms,
        duration_ms: 33,
        format: VideoPixelFormat::Rgba8,
        data: [32, 96, 224, 255].repeat((request.width * request.height) as usize),
    }
}
impl VideoFrameDecoder for Decoder {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}
    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.calls.ordinary.fetch_add(1, Ordering::Relaxed);
        Ok(Some(frame(request)))
    }
    fn decode_input_frame_with_cancellation(
        &mut self,
        input: &VideoRenderInput,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.calls.cancellable.fetch_add(1, Ordering::Relaxed);
        assert!(
            !cancellation.is_cancelled(),
            "already-cancelled work reached decode"
        );
        if self.cancel_during_decode {
            self.cancel.store(true, Ordering::Release);
        }
        if self.return_cancel_error {
            return Err(VideoDecodeError::Cancelled);
        }
        // An uncooperative decoder's late success must be rejected by the owner.
        Ok(Some(frame(&input.request)))
    }
}
fn snapshot() -> VideoSnapshot {
    let mut snapshot = VideoSnapshot::default();
    snapshot.layers.push(VideoLayerSummary {
        id: 7,
        label: "Thumbnail".into(),
        media_asset_id: None,
        source: VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some("fixture-not-opened.mp4".into()),
            name: None,
            codec: None,
            metadata: None,
        },
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState::default(),
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    });
    snapshot
}
fn decoder(calls: &Arc<Calls>, cancel: &Arc<AtomicBool>, late: bool, error: bool) -> Decoder {
    Decoder {
        calls: Arc::clone(calls),
        cancel: Arc::clone(cancel),
        cancel_during_decode: late,
        return_cancel_error: error,
    }
}

#[test]
fn precancelled_thumbnail_does_not_start_decoder_or_fallback() {
    let calls = Arc::new(Calls::default());
    let cancel = Arc::new(AtomicBool::new(true));
    let mut renderer = VideoPreviewRenderer::with_decoder(
        VideoRuntimeConfig::default(),
        decoder(&calls, &cancel, false, false),
    );
    let result = renderer
        .render_layer_preview_cancellable(&snapshot(), 7, 4, 2, &|| cancel.load(Ordering::Acquire));
    assert_eq!(result, Err(VideoPreviewError::Cancelled));
    assert_eq!(calls.ordinary.load(Ordering::Relaxed), 0);
    assert_eq!(calls.cancellable.load(Ordering::Relaxed), 0);
}

#[test]
fn normal_and_cancellable_thumbnail_pixels_match_with_distinct_decoder_paths() {
    let cancel = Arc::new(AtomicBool::new(false));
    let plain_calls = Arc::new(Calls::default());
    let cancel_calls = Arc::new(Calls::default());
    let mut plain = VideoPreviewRenderer::with_decoder(
        VideoRuntimeConfig::default(),
        decoder(&plain_calls, &cancel, false, false),
    );
    let mut cancellable = VideoPreviewRenderer::with_decoder(
        VideoRuntimeConfig::default(),
        decoder(&cancel_calls, &cancel, false, false),
    );
    let source = snapshot();
    let expected = plain.render_layer_preview(&source, 7, 4, 2).unwrap();
    let actual = cancellable
        .render_layer_preview_cancellable(&source, 7, 4, 2, &|| false)
        .unwrap();
    assert_eq!(actual, expected);
    assert!(actual
        .data
        .chunks_exact(4)
        .any(|pixel| pixel[0] > 0 && pixel[3] > 0));
    assert!(plain_calls.ordinary.load(Ordering::Relaxed) > 0);
    assert_eq!(plain_calls.cancellable.load(Ordering::Relaxed), 0);
    assert_eq!(cancel_calls.ordinary.load(Ordering::Relaxed), 0);
    assert!(cancel_calls.cancellable.load(Ordering::Relaxed) > 0);
}

#[test]
fn cancelled_decode_cannot_publish_a_late_success_and_renderer_can_be_reused() {
    let calls = Arc::new(Calls::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut renderer = VideoPreviewRenderer::with_decoder(
        VideoRuntimeConfig::default(),
        decoder(&calls, &cancel, true, false),
    );
    assert_eq!(
        renderer.render_layer_preview_cancellable(&snapshot(), 7, 4, 2, &|| cancel
            .load(Ordering::Acquire)),
        Err(VideoPreviewError::Cancelled)
    );
    assert_eq!(calls.ordinary.load(Ordering::Relaxed), 0);
    cancel.store(false, Ordering::Release);
    renderer
        .frame_provider_mut()
        .decoder_mut()
        .cancel_during_decode = false;
    assert!(renderer
        .render_layer_preview_cancellable(&snapshot(), 7, 4, 2, &|| cancel.load(Ordering::Acquire))
        .is_ok());
}

#[test]
fn decoder_cancellation_error_is_not_a_placeholder_or_sync_retry() {
    let calls = Arc::new(Calls::default());
    let cancel = Arc::new(AtomicBool::new(false));
    let mut renderer = VideoPreviewRenderer::with_decoder(
        VideoRuntimeConfig::default(),
        decoder(&calls, &cancel, false, true),
    );
    assert_eq!(
        renderer.render_layer_preview_cancellable(&snapshot(), 7, 4, 2, &|| false),
        Err(VideoPreviewError::Cancelled)
    );
    assert_eq!(calls.ordinary.load(Ordering::Relaxed), 0);
    assert_eq!(calls.cancellable.load(Ordering::Relaxed), 1);
}

#[test]
fn invalid_size_and_missing_layer_keep_the_normal_error_contract() {
    let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
    for (id, width, height) in [(7, 0, 2), (7, 4, 0), (999, 4, 2)] {
        let ordinary = renderer.render_layer_preview(&snapshot(), id, width, height);
        let cancellable =
            renderer.render_layer_preview_cancellable(&snapshot(), id, width, height, &|| false);
        assert!(ordinary.is_err());
        assert_eq!(cancellable, ordinary);
    }
}

#[test]
fn cancellation_is_checked_before_snapshot_validation() {
    let mut renderer = VideoPreviewRenderer::new(VideoRuntimeConfig::default());
    assert_eq!(
        renderer.render_layer_preview_cancellable(&VideoSnapshot::default(), 0, 0, 0, &|| true),
        Err(VideoPreviewError::Cancelled)
    );
}
