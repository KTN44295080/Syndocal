use super::*;
use protocol::{VideoIsfTempoSync, VideoIsfTempoSyncUnit};

const RATE_SHADER: &str = r#"/*{
  "INPUTS": [
    {"NAME":"inputImage","TYPE":"image"},
    {"NAME":"rate","TYPE":"float","DEFAULT":0.5,"MIN":0.25,"MAX":8.0}
  ]
}*/
void main() {
  gl_FragColor = vec4(rate / 8.0, fract(TIME), TIMEDELTA, 1.0);
}"#;

fn synced_effect(unit: VideoIsfTempoSyncUnit, count: f32) -> VideoIsfEffectSummary {
    let shader = prepare_isf_shader(RATE_SHADER).unwrap();
    let mut effect = isf_effect_from_prepared("Tempo rate".to_string(), RATE_SHADER, None, &shader);
    effect.controls[0].tempo_sync = Some(VideoIsfTempoSync { unit, count });
    effect
}

#[test]
fn isf_tempo_sync_resolves_beats_and_bars_at_live_rates() {
    use VideoIsfTempoSyncUnit::{Bars, Beats};
    let shader = prepare_isf_shader(RATE_SHADER).unwrap();
    for (bpm, unit, count, expected) in [
        (60.0, Beats, 1.0, 1.0),
        (90.0, Beats, 1.0, 1.5),
        (120.0, Beats, 0.25, 8.0),
        (120.0, Beats, 0.5, 4.0),
        (120.0, Beats, 2.0, 1.0),
        (120.0, Beats, 4.0, 0.5),
        (180.0, Bars, 1.0, 0.75),
        (120.0, Bars, 0.25, 2.0),
        (120.0, Bars, 0.5, 1.0),
        (120.0, Bars, 2.0, 0.25),
    ] {
        let effect = synced_effect(unit, count);
        let authored = effect.clone();
        let values = resolved_isf_control_values(&shader, &effect, Some(bpm));
        assert_eq!(values[0], [expected, 0.0, 0.0, 0.0]);
        assert_eq!(
            effect, authored,
            "render resolution must not edit the project"
        );
    }
}

#[test]
fn isf_tempo_sync_uses_shader_bounds_and_preserves_raw_controls() {
    let shader = prepare_isf_shader(RATE_SHADER).unwrap();
    let mut effect = synced_effect(VideoIsfTempoSyncUnit::Beats, 0.25);
    // The prepared shader remains authoritative even with misleading summaries.
    effect.controls[0].minimum[0] = -100.0;
    effect.controls[0].maximum[0] = 100.0;
    assert_eq!(
        resolved_isf_control_values(&shader, &effect, Some(300.0))[0][0],
        8.0
    );
    effect.controls[0].tempo_sync = Some(VideoIsfTempoSync {
        unit: VideoIsfTempoSyncUnit::Bars,
        count: 4.0,
    });
    assert_eq!(
        resolved_isf_control_values(&shader, &effect, Some(60.0))[0][0],
        0.25
    );
    effect.controls[0].tempo_sync = None;
    assert_eq!(
        resolved_isf_control_values(&shader, &effect, Some(180.0))[0][0],
        0.5
    );
    assert_eq!(
        resolved_isf_control_values_from_controls(&shader, &[], Some(180.0))[0],
        shader.controls[0].default
    );
}

#[test]
fn isf_tempo_sync_missing_or_invalid_tempo_keeps_authored_value() {
    let shader = prepare_isf_shader(RATE_SHADER).unwrap();
    let effect = synced_effect(VideoIsfTempoSyncUnit::Beats, 1.0);
    for bpm in [
        None,
        Some(0.0),
        Some(-120.0),
        Some(f32::NAN),
        Some(f32::INFINITY),
    ] {
        let values = resolved_isf_control_values(&shader, &effect, bpm);
        assert_eq!(values[0][0], 0.5);
        assert!(values.iter().flatten().all(|value| value.is_finite()));
    }
    let mut malformed = effect;
    for count in [0.0, -1.0, 3.0, f32::NAN, f32::INFINITY] {
        malformed.controls[0].tempo_sync.as_mut().unwrap().count = count;
        assert_eq!(
            resolved_isf_control_values(&shader, &malformed, Some(120.0))[0][0],
            0.5
        );
    }
    malformed.controls[0].tempo_sync.as_mut().unwrap().count = 1.0;
    malformed.controls[0].kind = protocol::VideoIsfControlKind::Long;
    assert_eq!(
        resolved_isf_control_values(&shader, &malformed, Some(120.0))[0][0],
        0.5
    );
    let mut non_float_shader = shader;
    non_float_shader.controls[0].kind = IsfControlKind::Long;
    malformed.controls[0].kind = protocol::VideoIsfControlKind::Float;
    malformed.controls[0].value[0] = 3.0;
    assert_eq!(
        resolved_isf_control_values(&non_float_shader, &malformed, Some(120.0))[0][0],
        3.0
    );
}

struct FixedFrameDecoder;

impl VideoFrameDecoder for FixedFrameDecoder {
    fn retain_layers(&mut self, _layer_ids: &[VideoLayerId]) {}

    fn decode_frame(
        &mut self,
        request: &VideoFrameRequest,
    ) -> Result<Option<VideoFrame>, VideoDecodeError> {
        assert_eq!((request.width, request.height), (1, 1));
        Ok(Some(VideoFrame {
            layer_id: request.layer_id,
            width: 1,
            height: 1,
            pts_ms: request.position_ms,
            duration_ms: 125,
            format: VideoPixelFormat::Rgba8,
            data: vec![0, 0, 0, 255],
        }))
    }
}

#[test]
fn isf_tempo_sync_renderer_follows_mid_run_tempo_with_unchanged_snapshot_and_time() {
    let effect = synced_effect(VideoIsfTempoSyncUnit::Beats, 1.0);
    let snapshot = VideoSnapshot {
        layers: vec![VideoLayerSummary {
            id: 1,
            label: "Tempo fixture".to_string(),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some("tempo-fixture.mp4".to_string()),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 1_250,
                ..VideoLayerState::default()
            },
            isf_effect: Some(effect),
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        }],
        ..VideoSnapshot::default()
    };
    let authored = snapshot.clone();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        DecoderBackedFrameProvider::new(FixedFrameDecoder),
    );
    // Repeated 180 BPM simulates a stopped clock retaining its last tempo.
    // PTS stays fixed to prove tempo sync does not retime Preview or shader TIME.
    for (bpm, expected_red) in [
        (Some(60.0), 32),
        (Some(120.0), 64),
        (Some(180.0), 96),
        (Some(180.0), 96),
        (None, 16),
        (Some(0.0), 16),
        (Some(120.0), 64),
    ] {
        renderer.frame_provider_mut().set_bpm(bpm);
        let frame = renderer.render(&snapshot, 1, 1).unwrap();
        assert_eq!(renderer.last_isf_error(), None);
        for (actual, expected) in frame.data.iter().zip([expected_red, 64, 32, 255]) {
            assert!(
                actual.abs_diff(expected) <= 1,
                "BPM {bpm:?}: pixel {:?}",
                frame.data
            );
        }
        assert_eq!(renderer.isf_shader_cache_count(), 1);
        assert_eq!(renderer.isf_pipeline_count(), 1);
        assert_eq!(snapshot, authored);
    }
}
