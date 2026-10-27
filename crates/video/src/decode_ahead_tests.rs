//! A tick's frames are decoded ahead together on the live path; a later
//! request for exactly one of them returns that result, anything else
//! decodes as before, and the cancellable (recording) path never decodes
//! ahead.
use super::*;
use protocol::AutoVjSnapshot;

/// Records which layers each single and batch decode was asked for.
#[derive(Default)]
struct RecordingDecoder {
    single: Vec<VideoLayerId>,
    batches: Vec<Vec<VideoLayerId>>,
}

fn solid(request: &VideoFrameRequest) -> VideoFrame {
    debug_solid_frame_for_layer(
        request.layer_id,
        request.position_ms,
        request.width,
        request.height,
    )
}

impl VideoFrameDecoder for RecordingDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.single.push(request.layer_id);
        Ok(Some(solid(request)))
    }

    fn decode_input_frames_shared(
        &mut self,
        inputs: &[VideoRenderInput],
    ) -> Vec<Result<Option<Arc<VideoFrame>>, VideoDecodeError>> {
        self.batches
            .push(inputs.iter().map(|input| input.request.layer_id).collect());
        inputs
            .iter()
            .map(|input| Ok(Some(Arc::new(solid(&input.request)))))
            .collect()
    }
}

fn layer(id: VideoLayerId, kind: VideoSourceKind, position_ms: u64) -> VideoLayerSummary {
    VideoLayerSummary {
        id,
        label: format!("Layer {id}"),
        source: VideoSourceSummary {
            kind,
            path: Some(format!("layer-{id}.mp4")),
            name: None,
            codec: None,
            metadata: None,
        },
        media_asset_id: None,
        blend_mode: VideoBlendMode::Normal,
        state: VideoLayerState {
            position_ms,
            ..VideoLayerState::default()
        },
        isf_effect: None,
        clip_slots: Vec::new(),
        default_clip_slot_id: None,
    }
}

#[test]
fn decoded_ahead_frames_are_used_once_and_cleared_with_the_tick() {
    let mut provider = DecoderBackedFrameProvider::new(RecordingDecoder::default());
    let first = layer(1, VideoSourceKind::File, 100);
    let second = layer(2, VideoSourceKind::File, 200);
    let still = layer(3, VideoSourceKind::StillImage, 0);
    let mut inputs = Vec::new();
    for layer in [&first, &second, &still] {
        inputs.extend(provider.layer_frame_inputs(layer, 16, 9));
    }
    provider.decode_inputs_ahead(&inputs);
    // Still images never reach the decoder.
    assert_eq!(provider.decoder().batches, vec![vec![1, 2]]);

    let frame = provider.frame_for_layer(&first, 16, 9).unwrap();
    assert_eq!(frame, solid(&inputs[0].request));
    assert!(
        provider.decoder().single.is_empty(),
        "a decoded-ahead frame is reused"
    );
    provider.frame_for_layer(&first, 16, 9).unwrap();
    assert_eq!(provider.decoder().single, vec![1], "and used only once");
    // A different position is a different input.
    provider
        .frame_for_layer(&layer(2, VideoSourceKind::File, 233), 16, 9)
        .unwrap();
    assert_eq!(provider.decoder().single, vec![1, 2]);

    // The next tick's retention drops anything left over.
    provider.retain_layers(&[1, 2]);
    provider.frame_for_layer(&second, 16, 9).unwrap();
    assert_eq!(provider.decoder().single, vec![1, 2, 2]);

    // One input alone is not worth a batch.
    provider.decode_inputs_ahead(&inputs[..1]);
    assert_eq!(provider.decoder().batches.len(), 1);
}

fn two_layer_snapshot() -> VideoSnapshot {
    VideoSnapshot {
        layers: vec![
            layer(11, VideoSourceKind::File, 100),
            layer(12, VideoSourceKind::File, 200),
        ],
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        compositions: vec![CompositionSummary {
            id: 5,
            label: "Program".to_string(),
            layer_ids: vec![11, 12],
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
            width: 32,
            height: 18,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }],
        mapping_presets: Vec::new(),
        auto_vj: AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

#[test]
fn the_live_seam_decodes_a_tick_together_and_recording_does_not() {
    let snapshot = two_layer_snapshot();
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };

    let mut live = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(RecordingDecoder::default()),
    );
    let rendered = live
        .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
            &snapshot,
            context,
            &transitions,
            6,
            32,
            18,
        )
        .unwrap();
    assert!(rendered.frame().is_some());
    assert_eq!(live.frame_provider().decoder().batches, vec![vec![11, 12]]);
    assert!(live.frame_provider().decoder().single.is_empty());

    let mut recording = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(RecordingDecoder::default()),
    );
    let recorded = recording
        .render_output_preview_with_effects_and_transitions_cancellable(
            &snapshot,
            context,
            &transitions,
            6,
            32,
            18,
            &NeverCancelled,
        )
        .unwrap();
    assert!(recording.frame_provider().decoder().batches.is_empty());
    assert_eq!(recording.frame_provider().decoder().single, vec![11, 12]);
    // Decoding ahead does not change the pixels.
    assert_eq!(rendered.frame().unwrap(), &recorded);
}

struct NeverCancelled;

impl VideoRenderCancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}

/// Hands out the same shared frame for every request of a layer.
#[derive(Default)]
struct SharedDecoder {
    frames: Vec<Arc<VideoFrame>>,
}

impl VideoFrameDecoder for SharedDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.decode_input_frame_shared(&VideoRenderInput::legacy(request.clone()))
            .map(|frame| frame.map(Arc::unwrap_or_clone))
    }

    fn decode_input_frame_shared(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<Option<Arc<VideoFrame>>, VideoDecodeError> {
        let request = &input.request;
        if let Some(frame) = self
            .frames
            .iter()
            .find(|frame| frame.layer_id == request.layer_id)
        {
            return Ok(Some(Arc::clone(frame)));
        }
        let frame = Arc::new(solid(request));
        self.frames.push(Arc::clone(&frame));
        Ok(Some(frame))
    }
}

#[test]
fn a_layer_without_effects_queues_the_decoders_own_frame() {
    let snapshot = two_layer_snapshot();
    let clip_runtime = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        audio_frame: None,
        clip_runtime: &clip_runtime,
        project_render_epoch: 1,
    };
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(SharedDecoder::default()),
    );
    for _ in 0..3 {
        renderer
            .prepare_output_artistic_render_result_preview_with_effects_and_transitions(
                &snapshot,
                context,
                &transitions,
                6,
                32,
                18,
            )
            .unwrap();
    }
    let decoded = &renderer.frame_provider().decoder().frames;
    assert_eq!(decoded.len(), 2);
    for frame in decoded {
        let queued = renderer
            .runtime
            .queues
            .iter()
            .find(|queue| queue.layer_id() == frame.layer_id)
            .and_then(|queue| queue.iter_shared().next())
            .unwrap();
        assert!(
            Arc::ptr_eq(queued, frame),
            "layer {} queued a copy",
            frame.layer_id
        );
        assert_eq!(Arc::strong_count(frame), 2, "decoder and queue only");
    }
}

/// Counts which cancellable route the provider takes; the shared route hands
/// out the decoder's own frame.
#[derive(Default)]
struct CancellableRouteDecoder {
    shared: SharedDecoder,
    shared_cancellable: usize,
    owned_cancellable: usize,
}

impl VideoFrameDecoder for CancellableRouteDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.shared.decode_frame(request)
    }

    fn decode_input_frame_with_cancellation(
        &mut self,
        input: &VideoRenderInput,
        _cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        self.owned_cancellable += 1;
        self.shared.decode_frame(&input.request)
    }

    fn decode_input_frame_shared_with_cancellation(
        &mut self,
        input: &VideoRenderInput,
        _cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Option<Arc<VideoFrame>>, VideoDecodeError> {
        self.shared_cancellable += 1;
        self.shared.decode_input_frame_shared(input)
    }
}

/// Resident native preparation acquires each layer through the cancellable
/// shared route: the decoder's own frame (no full-frame copy) via the decoder
/// method that keeps its live look-ahead. The owned cancellable route used by
/// canonical CPU callers is unchanged, and Stop still precedes any decode.
#[test]
fn resident_cancellable_acquisition_keeps_the_decoders_shared_frame() {
    let snapshot = two_layer_snapshot();
    let input = legacy_layer_input(&snapshot.layers[0], 32, 18, None);
    let mut provider = DecoderBackedFrameProvider::new(CancellableRouteDecoder::default());
    let first = provider
        .frame_for_input_shared_with_cancellation(&input, &NeverCancelled)
        .unwrap();
    let second = provider
        .frame_for_input_shared_with_cancellation(&input, &NeverCancelled)
        .unwrap();
    let decoder = provider.decoder();
    assert_eq!(decoder.shared_cancellable, 2);
    assert_eq!(decoder.owned_cancellable, 0);
    assert!(
        Arc::ptr_eq(&first, &decoder.shared.frames[0]),
        "first acquisition copied"
    );
    assert!(Arc::ptr_eq(&second, &first), "repeat acquisition copied");

    let owned = provider
        .frame_for_input_with_cancellation(&input, &NeverCancelled)
        .unwrap();
    assert_eq!(owned.layer_id, first.layer_id);
    assert_eq!(provider.decoder().owned_cancellable, 1);
    assert_eq!(provider.decoder().shared_cancellable, 2);

    let cancelled = provider.frame_for_input_shared_with_cancellation(&input, &|| true);
    assert!(matches!(cancelled, Err(VideoFrameProviderError::Cancelled)));
    assert_eq!(
        provider.decoder().shared_cancellable,
        2,
        "Stop precedes decode"
    );
}
