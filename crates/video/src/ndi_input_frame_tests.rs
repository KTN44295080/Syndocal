use super::*;

fn request(kind: VideoSourceKind) -> VideoRenderInput {
    VideoRenderInput::legacy(VideoFrameRequest {
        layer_id: 91,
        label: "Live input".to_string(),
        source: VideoSourceSummary {
            kind,
            path: None,
            name: Some("STAGE-PC (Program)".to_string()),
            codec: None,
            metadata: None,
        },
        position_ms: 250,
        width: 4,
        height: 2,
    })
}

#[test]
fn ndi_no_signal_is_transparent_in_both_placeholder_modes_and_decode_paths() {
    for placeholder in [false, true] {
        let mut provider = DecoderBackedFrameProvider::new(NullVideoDecoder)
            .with_placeholder_when_missing(placeholder);
        let input = request(VideoSourceKind::Ndi);
        let shared = provider
            .decode_or_placeholder_for_input_shared(&input)
            .unwrap();
        let cancelled_path = provider
            .frame_for_input_with_cancellation(&input, &|| false)
            .unwrap();
        assert_eq!(*shared, cancelled_path);
        assert_eq!((shared.layer_id, shared.pts_ms), (91, 250));
        assert_eq!((shared.width, shared.height), (1, 1));
        assert_eq!(shared.data, [0, 0, 0, 0]);
        assert!(matches!(
            provider.frame_for_input_with_cancellation(&input, &|| true),
            Err(VideoFrameProviderError::Cancelled)
        ));
    }
}

struct SwitchableDecoder {
    frame: Option<VideoFrame>,
}
impl VideoFrameDecoder for SwitchableDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}
    fn decode_frame(
        &mut self,
        _request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        Ok(self.frame.clone())
    }
}

#[test]
fn ndi_loss_replaces_previous_pixels_and_recovery_uses_the_new_frame() {
    let input = request(VideoSourceKind::Ndi);
    let mut live = ndi_no_signal_frame(&input.request);
    live.data = vec![17, 83, 201, 255];
    let mut provider = DecoderBackedFrameProvider::new(SwitchableDecoder {
        frame: Some(live.clone()),
    })
    .with_placeholder_when_missing(false);
    assert_eq!(provider.frame_for_input(&input).unwrap().data, live.data);
    provider.decoder_mut().frame = None;
    assert_eq!(provider.frame_for_input(&input).unwrap().data, [0, 0, 0, 0]);
    live.data = vec![201, 83, 17, 255];
    provider.decoder_mut().frame = Some(live.clone());
    assert_eq!(provider.frame_for_input(&input).unwrap().data, live.data);
}

#[test]
fn ndi_no_signal_policy_does_not_change_other_source_kinds() {
    for kind in [
        VideoSourceKind::File,
        VideoSourceKind::Camera,
        VideoSourceKind::ScreenCapture,
        VideoSourceKind::Spout,
    ] {
        let input = request(kind);
        let mut strict =
            DecoderBackedFrameProvider::new(NullVideoDecoder).with_placeholder_when_missing(false);
        assert!(matches!(
            strict.frame_for_input(&input),
            Err(VideoFrameProviderError::Decode {
                error: VideoDecodeError::UnsupportedSource { .. },
                ..
            })
        ));
        let mut preview = DecoderBackedFrameProvider::new(NullVideoDecoder);
        let frame = preview.frame_for_input(&input).unwrap();
        assert_eq!((frame.width, frame.height), (4, 2));
        assert_eq!(frame.data[3], 255);
    }
}
