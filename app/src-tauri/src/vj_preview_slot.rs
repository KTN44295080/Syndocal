use protocol::{
    MediaAssetSummary, VideoClipLoopMode, VideoClipSlotId, VideoClipSlotSummary, VideoLayerSummary,
    VideoSnapshot, VideoSourceKind,
};

/// Exact authored clip and media identity for an ephemeral Preview session.
/// Queue state, selection and layer ordering never change this identity.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VjPreviewSlotAudition {
    pub slot: VideoClipSlotSummary,
    pub asset: MediaAssetSummary,
}

impl VjPreviewSlotAudition {
    pub fn resolve(
        snapshot: &VideoSnapshot,
        layer: &VideoLayerSummary,
        slot_id: VideoClipSlotId,
    ) -> Result<Self, String> {
        let slot = layer
            .clip_slots
            .iter()
            .find(|slot| slot.id == slot_id)
            .ok_or_else(|| {
                format!(
                    "Preview clip slot {slot_id:?} was not found in layer {}",
                    layer.id
                )
            })?
            .clone();
        let asset = snapshot
            .media_assets
            .iter()
            .find(|asset| asset.id == slot.media_asset_id)
            .ok_or_else(|| format!("Preview clip slot {slot_id:?} has no media asset"))?
            .clone();
        let audition = Self { slot, asset };
        audition.bounds()?;
        if !audition.slot.speed.is_finite() || !(-4.0..=4.0).contains(&audition.slot.speed) {
            return Err("Preview clip speed must be finite and between -4 and 4".to_string());
        }
        Ok(audition)
    }

    pub fn is_current(&self, snapshot: &VideoSnapshot, layer: &VideoLayerSummary) -> bool {
        layer.clip_slots.iter().any(|slot| slot == &self.slot)
            && snapshot
                .media_assets
                .iter()
                .any(|asset| asset == &self.asset)
    }

    pub fn bounds(&self) -> Result<(u64, Option<u64>), String> {
        if self.asset.source.kind == VideoSourceKind::StillImage {
            return Ok((0, Some(0)));
        }
        let duration = self
            .asset
            .source
            .metadata
            .and_then(|metadata| metadata.duration_ms);
        let start = self.slot.in_point_ms;
        let end = match (self.slot.out_point_ms, duration) {
            (Some(out), Some(duration)) => Some(out.min(duration)),
            (out, duration) => out.or(duration),
        };
        if end.is_some_and(|end| end <= start) {
            return Err("Preview clip out-point must be after its in-point".to_string());
        }
        if self.slot.speed < 0.0 && end.is_none() {
            return Err(
                "Reverse Preview needs a clip out-point or a known source duration".to_string(),
            );
        }
        if self.slot.loop_mode != VideoClipLoopMode::Once && end.is_none() {
            return Err(
                "Looping Preview needs a clip out-point or a known source duration".to_string(),
            );
        }
        Ok((start, end))
    }

    pub fn preview_layer(
        &self,
        mut preview: VideoLayerSummary,
    ) -> Result<VideoLayerSummary, String> {
        let (start, end) = self.bounds()?;
        preview.source = self.asset.source.clone();
        preview.media_asset_id = Some(self.asset.id);
        preview.state.position_ms = start;
        preview.state.speed = self.slot.speed;
        preview.state.loop_enabled = self.slot.loop_mode != VideoClipLoopMode::Once;
        preview.state.loop_start_ms = start;
        preview.state.loop_end_ms = end.unwrap_or(0);
        Ok(preview)
    }
}

impl crate::VjPreviewTransportRuntime {
    /// Validate the accepted session while its caller holds the Preview mutex.
    /// The engine repeats the authored identity check inside its commit lane.
    pub(super) fn slot_take_guard_at(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_id: protocol::VideoLayerId,
        slot_id: VideoClipSlotId,
        generation: u64,
        now: std::time::Instant,
        updated_at_ms: u64,
    ) -> Result<engine::VideoClipPreviewTakeGuard, String> {
        if !self.reconcile_at(snapshot, now, updated_at_ms) {
            return Err("Preview source changed before Take; stage it again".to_string());
        }
        let session = self
            .session
            .as_ref()
            .filter(|session| session.layer_id == layer_id && session.generation == generation)
            .ok_or_else(|| "Preview changed before Take; stage the clip again".to_string())?;
        let audition = session
            .slot_audition
            .as_ref()
            .filter(|audition| audition.slot.id == slot_id)
            .ok_or_else(|| "Preview Take must launch the exact auditioned slot".to_string())?;
        let layer = snapshot
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| "Preview layer was removed".to_string())?;
        Ok(engine::VideoClipPreviewTakeGuard {
            layer_source: layer.source.clone(),
            slot: audition.slot.clone(),
            asset: audition.asset.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::*;
    use std::time::{Duration, Instant};

    fn source(id: u64, duration_ms: Option<u64>) -> VideoSourceSummary {
        VideoSourceSummary {
            kind: VideoSourceKind::File,
            path: Some(format!("clip-{id}.mp4")),
            name: Some(format!("Source {id}")),
            codec: Some("h264".to_string()),
            metadata: Some(protocol::VideoMediaMetadata {
                duration_ms,
                width: Some(1_920),
                height: Some(1_080),
                frame_rate: Some(30.0),
                has_audio: true,
            }),
        }
    }

    fn fixture() -> EngineSnapshot {
        let mut snapshot = EngineSnapshot::default();
        snapshot.video.media_assets.push(MediaAssetSummary {
            id: 71,
            label: "Audition media".to_string(),
            source: source(8, Some(2_000)),
            content_hash: None,
            byte_size: None,
        });
        snapshot.video.layers.push(VideoLayerSummary {
            id: 7,
            label: "Program layer".to_string(),
            source: source(7, Some(5_000)),
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState {
                position_ms: 125,
                playing: true,
                speed: 1.0,
                ..VideoLayerState::default()
            },
            isf_effect: None,
            clip_slots: vec![VideoClipSlotSummary {
                id: VideoClipSlotId(72),
                media_asset_id: 71,
                in_point_ms: 200,
                out_point_ms: Some(800),
                loop_mode: VideoClipLoopMode::Once,
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: protocol::VideoClipLaunchQuantization::Immediate,
                effect_overrides: Vec::new(),
            }],
            default_clip_slot_id: Some(VideoClipSlotId(72)),
        });
        snapshot
    }

    fn stage(
        runtime: &mut VjPreviewTransportRuntime,
        snapshot: &EngineSnapshot,
        slot: u64,
        now: Instant,
    ) -> VjPreviewTransportSummary {
        runtime
            .stage_slot_at(
                &snapshot.video,
                &snapshot.video.layers[0],
                VideoClipSlotId(slot),
                now,
                1,
            )
            .unwrap()
    }

    fn render(
        runtime: &VjPreviewTransportRuntime,
        snapshot: &EngineSnapshot,
        position_ms: u64,
    ) -> VideoSnapshot {
        vj_preview_render_snapshot_for_slot(
            &snapshot.video,
            7,
            position_ms,
            runtime.effect_audition(),
            runtime.session.as_ref().unwrap().slot_audition.as_deref(),
        )
        .unwrap()
    }

    #[test]
    fn slot_preview_trimmed_playback_preserves_program_and_authored_state() {
        let snapshot = fixture();
        let before = serde_json::to_value(&snapshot).unwrap();
        let now = Instant::now();
        let mut runtime = VjPreviewTransportRuntime::default();
        let staged = stage(&mut runtime, &snapshot, 72, now);
        assert_eq!(staged.slot_id, Some(VideoClipSlotId(72)));
        assert_eq!(staged.position_ms, 200);
        assert!(staged.playing);
        let advanced = runtime.summary_at(now + Duration::from_millis(250), 251);
        assert_eq!(advanced.position_ms, 450);
        let preview = render(&runtime, &snapshot, advanced.position_ms);
        assert_eq!(preview.layers[0].source.path.as_deref(), Some("clip-8.mp4"));
        assert_eq!(preview.layers[0].state.position_ms, 450);
        assert!(preview.outputs.is_empty());
        assert!(preview.transition_buses.is_empty());
        assert_eq!(
            snapshot.video.layers[0].source.path.as_deref(),
            Some("clip-7.mp4")
        );
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
        assert!(runtime
            .current_take_state_at(7, now + Duration::from_millis(250), 252)
            .is_none());
        let stopped = runtime.summary_at(now + Duration::from_millis(900), 901);
        assert_eq!(stopped.position_ms, 800);
        assert!(!stopped.playing);
        assert_eq!(
            render(&runtime, &snapshot, stopped.position_ms).layers[0]
                .state
                .position_ms,
            799
        );
        assert_eq!(
            runtime
                .seek_at(0, now + Duration::from_millis(900), 902)
                .unwrap()
                .position_ms,
            200
        );
        runtime
            .seek_at(250, now + Duration::from_millis(900), 903)
            .unwrap();
        runtime
            .set_speed_at(-1.0, now + Duration::from_millis(900), 904)
            .unwrap();
        runtime
            .set_playing_at(true, now + Duration::from_millis(900), 905)
            .unwrap();
        let reversed = runtime.summary_at(now + Duration::from_millis(1_000), 1_001);
        assert_eq!(reversed.position_ms, 200);
        assert!(!reversed.playing);
    }

    #[test]
    fn slot_preview_loop_ping_pong_reverse_and_unknown_duration_respect_bounds() {
        let mut snapshot = fixture();
        snapshot.video.layers[0].clip_slots[0].loop_mode = VideoClipLoopMode::Loop;
        let now = Instant::now();
        let mut runtime = VjPreviewTransportRuntime::default();
        stage(&mut runtime, &snapshot, 72, now);
        assert_eq!(
            runtime
                .summary_at(now + Duration::from_millis(700), 701)
                .position_ms,
            300
        );
        runtime
            .seek_at(250, now + Duration::from_millis(700), 702)
            .unwrap();
        runtime
            .set_speed_at(-1.0, now + Duration::from_millis(700), 703)
            .unwrap();
        assert_eq!(
            runtime
                .summary_at(now + Duration::from_millis(800), 801)
                .position_ms,
            750
        );

        snapshot.video.layers[0].clip_slots[0].loop_mode = VideoClipLoopMode::PingPong;
        stage(&mut runtime, &snapshot, 72, now);
        let bounced = runtime.summary_at(now + Duration::from_millis(700), 702);
        assert_eq!(bounced.position_ms, 700);
        assert_eq!(bounced.speed, -1.0);
        let returned = runtime.summary_at(now + Duration::from_millis(1_300), 1_302);
        assert_eq!(returned.position_ms, 300);
        assert_eq!(returned.speed, 1.0);
        assert!(returned.playing);

        snapshot.video.layers[0].clip_slots[0].loop_mode = VideoClipLoopMode::Once;
        snapshot.video.layers[0].clip_slots[0].speed = -1.0;
        let reversed = stage(&mut runtime, &snapshot, 72, now);
        assert_eq!(reversed.position_ms, 799);
        assert!(reversed.playing);
        assert_eq!(
            runtime
                .summary_at(now + Duration::from_millis(250), 251)
                .position_ms,
            549
        );
        let end = runtime.summary_at(now + Duration::from_millis(700), 701);
        assert_eq!(end.position_ms, 200);
        assert!(!end.playing);

        snapshot.video.layers[0].clip_slots[0].speed = 1.0;
        snapshot.video.media_assets[0]
            .source
            .metadata
            .as_mut()
            .unwrap()
            .duration_ms = None;
        stage(&mut runtime, &snapshot, 72, now);
        let stopped = runtime.summary_at(now + Duration::from_millis(700), 703);
        assert_eq!(stopped.duration_ms, None);
        assert_eq!(stopped.position_ms, 800);
        assert!(!stopped.playing);
    }

    #[test]
    fn slot_preview_identity_survives_reorder_and_expires_on_replacement_or_removal() {
        let mut snapshot = fixture();
        let mut another = snapshot.video.layers[0].clip_slots[0].clone();
        another.id = VideoClipSlotId(73);
        snapshot.video.layers[0].clip_slots.push(another);
        let now = Instant::now();
        let mut runtime = VjPreviewTransportRuntime::default();
        stage(&mut runtime, &snapshot, 72, now);
        let mut reordered = snapshot.clone();
        reordered.video.layers[0].clip_slots.reverse();
        reordered.video.layers[0].default_clip_slot_id = Some(VideoClipSlotId(73));
        reordered.video.layers[0].state.position_ms = 999;
        assert!(runtime.reconcile_at(&reordered.video, now, 2));
        assert_eq!(
            runtime.summary_at(now, 3).slot_id,
            Some(VideoClipSlotId(72))
        );

        let mut changes = Vec::new();
        let mut changed = snapshot.clone();
        changed.video.layers.clear();
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.layers[0]
            .clip_slots
            .retain(|slot| slot.id != VideoClipSlotId(72));
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.media_assets.clear();
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.media_assets[0].source.path = Some("replacement.mp4".to_string());
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.layers[0].source.path = Some("other-program.mp4".to_string());
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.layers[0].clip_slots[0].in_point_ms = 300;
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.layers[0].clip_slots[0].media_asset_id = 999;
        changes.push(changed);
        let mut changed = snapshot.clone();
        changed.video.media_assets[0].byte_size = Some(999);
        changes.push(changed);
        for changed in changes {
            stage(&mut runtime, &snapshot, 72, now);
            assert!(!runtime.reconcile_at(&changed.video, now, 5));
            let cleared = runtime.summary_at(now, 6);
            assert_eq!(cleared.layer_id, None);
            assert_eq!(cleared.slot_id, None);
            assert!(!cleared.effect_audition);
        }
    }

    #[test]
    fn slot_preview_fx_draft_is_isolated_and_old_frames_or_takes_are_rejected() {
        let mut snapshot = fixture();
        snapshot.video.layers[0].isf_effect =
            Some(video::builtin_isf_effect("monochrome").unwrap().unwrap());
        let mut another = snapshot.video.layers[0].clip_slots[0].clone();
        another.id = VideoClipSlotId(73);
        another.in_point_ms = 350;
        snapshot.video.layers[0].clip_slots.push(another);
        let before = serde_json::to_value(&snapshot).unwrap();
        let now = Instant::now();
        let mut runtime = VjPreviewTransportRuntime::default();
        runtime.stage_at(&snapshot.video.layers[0], now, 1).unwrap();
        runtime
            .set_effect_audition_at(
                7,
                Some(Some(video::builtin_isf_effect("invert").unwrap().unwrap())),
                now,
                2,
            )
            .unwrap();
        let first = stage(&mut runtime, &snapshot, 72, now);
        assert!(first.effect_audition);
        assert_eq!(
            render(&runtime, &snapshot, first.position_ms).layers[0]
                .isf_effect
                .as_ref()
                .unwrap()
                .label,
            "Invert"
        );
        assert_eq!(serde_json::to_value(&snapshot).unwrap(), before);
        let guard = runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(72),
                first.generation,
                now,
                3,
            )
            .unwrap();
        assert_eq!(guard.slot.id, VideoClipSlotId(72));
        assert!(runtime
            .slot_take_guard_at(
                &snapshot.video,
                8,
                VideoClipSlotId(72),
                first.generation,
                now,
                3
            )
            .is_err());
        assert!(runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(73),
                first.generation,
                now,
                3
            )
            .is_err());

        let second = stage(&mut runtime, &snapshot, 73, now);
        assert_eq!(second.slot_id, Some(VideoClipSlotId(73)));
        assert_eq!(second.position_ms, 350);
        assert!(second.effect_audition);
        assert!(!vj_preview_frame_is_current(&first, &second, true));
        assert!(runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(72),
                first.generation,
                now,
                4
            )
            .is_err());
        let restaged = stage(&mut runtime, &snapshot, 73, now);
        assert!(restaged.generation > second.generation);
        assert!(runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(73),
                second.generation,
                now,
                5
            )
            .is_err());
        let sought = runtime.seek_at(400, now, 6).unwrap();
        assert!(!vj_preview_frame_is_current(&restaged, &sought, true));
        let mut wrong_slot = sought.clone();
        wrong_slot.slot_id = Some(VideoClipSlotId(72));
        assert!(!vj_preview_frame_is_current(&sought, &wrong_slot, true));
        let cleared = runtime.clear_at(7);
        assert_eq!(cleared.slot_id, None);
        assert!(!cleared.effect_audition);
        assert!(!vj_preview_frame_is_current(&sought, &cleared, true));
        assert!(runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(73),
                sought.generation,
                now,
                8
            )
            .is_err());
    }

    #[test]
    fn slot_preview_rejects_invalid_sources_and_keeps_zero_speed_and_stills_paused() {
        let mut snapshot = fixture();
        let now = Instant::now();
        let mut runtime = VjPreviewTransportRuntime::default();
        assert!(runtime
            .stage_slot_at(
                &snapshot.video,
                &snapshot.video.layers[0],
                VideoClipSlotId(999),
                now,
                1
            )
            .is_err());
        let mut invalid = snapshot.clone();
        invalid.video.layers[0].clip_slots[0].out_point_ms = Some(200);
        assert!(runtime
            .stage_slot_at(
                &invalid.video,
                &invalid.video.layers[0],
                VideoClipSlotId(72),
                now,
                2
            )
            .is_err());
        invalid = snapshot.clone();
        invalid.video.media_assets.clear();
        assert!(runtime
            .stage_slot_at(
                &invalid.video,
                &invalid.video.layers[0],
                VideoClipSlotId(72),
                now,
                3
            )
            .is_err());
        invalid = snapshot.clone();
        invalid.video.media_assets[0].source.kind = VideoSourceKind::Ndi;
        assert!(runtime
            .stage_slot_at(
                &invalid.video,
                &invalid.video.layers[0],
                VideoClipSlotId(72),
                now,
                4
            )
            .is_err());
        invalid = snapshot.clone();
        for speed in [f32::NAN, f32::INFINITY, -4.1, 4.1] {
            invalid.video.layers[0].clip_slots[0].speed = speed;
            assert!(runtime
                .stage_slot_at(
                    &invalid.video,
                    &invalid.video.layers[0],
                    VideoClipSlotId(72),
                    now,
                    4
                )
                .is_err());
        }
        invalid = snapshot.clone();
        invalid.video.layers[0].clip_slots[0].speed = -1.0;
        invalid.video.layers[0].clip_slots[0].out_point_ms = None;
        invalid.video.media_assets[0]
            .source
            .metadata
            .as_mut()
            .unwrap()
            .duration_ms = None;
        let reverse_error = runtime
            .stage_slot_at(
                &invalid.video,
                &invalid.video.layers[0],
                VideoClipSlotId(72),
                now,
                4,
            )
            .unwrap_err();
        assert!(reverse_error.contains("Reverse Preview needs"));
        snapshot.video.layers[0].clip_slots[0].speed = 0.0;
        let authored = snapshot.clone();
        let paused = stage(&mut runtime, &snapshot, 72, now);
        assert_eq!(paused.speed, 0.0);
        assert!(!paused.playing);
        let paused = runtime.set_playing_at(true, now, 5).unwrap();
        assert!(!paused.playing);
        assert_eq!(
            runtime
                .summary_at(now + Duration::from_secs(5), 5_005)
                .position_ms,
            200
        );
        let guard = runtime
            .slot_take_guard_at(
                &snapshot.video,
                7,
                VideoClipSlotId(72),
                paused.generation,
                now + Duration::from_secs(5),
                5_005,
            )
            .unwrap();
        assert_eq!(guard.slot.speed, 0.0);
        assert_eq!(snapshot, authored);
        snapshot.video.layers[0].clip_slots[0].speed = 1.0;
        snapshot.video.media_assets[0].source.kind = VideoSourceKind::StillImage;
        let staged = stage(&mut runtime, &snapshot, 72, now);
        assert!(!staged.playing);
        assert_eq!(staged.position_ms, 0);
        assert_eq!(staged.duration_ms, Some(0));
        assert!(!runtime.set_playing_at(true, now, 6).unwrap().playing);
        assert_eq!(
            runtime
                .summary_at(now + Duration::from_secs(10), 10_006)
                .position_ms,
            0
        );
    }

    #[test]
    fn slot_preview_take_request_keeps_legacy_fingerprint_shape_when_guard_is_absent() {
        let legacy: VideoClipSlotLaunchRequest = serde_json::from_value(serde_json::json!({
            "layer_id": 7, "slot_id": 72, "transition_kind": "Cut",
        }))
        .unwrap();
        assert_eq!(legacy.preview_generation, None);
        let legacy_value = serde_json::to_value(&legacy).unwrap();
        assert!(legacy_value.get("preview_generation").is_none());
        let guarded = VideoClipSlotLaunchRequest {
            preview_generation: Some(42),
            ..legacy
        };
        let guarded_value = serde_json::to_value(&guarded).unwrap();
        assert_eq!(guarded_value["preview_generation"], 42);
        assert_ne!(legacy_value, guarded_value);
    }
}
