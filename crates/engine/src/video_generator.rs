use protocol::VideoGenerator;

use crate::*;

impl EngineHandle {
    /// Create a generator through the same admitted, publication-coupled media
    /// transaction used by other sources. The caller owns identity allocation
    /// and the surrounding durable project history/ticket boundary.
    pub fn create_video_generator_layer_published(
        &self,
        layer_id: VideoLayerId,
        asset_id: MediaAssetId,
        clip_slot_id: VideoClipSlotId,
        label: String,
        generator: VideoGenerator,
    ) -> Result<(), String> {
        self.media_asset_transaction_published(generator_layer_transaction(
            layer_id,
            asset_id,
            clip_slot_id,
            label,
            generator,
        )?)
    }

    /// Update only parameters of the current generator. Catalog resolution is
    /// performed at runtime admission, so a stale UI cannot overwrite unrelated
    /// source fields or convert an existing file/live source into a generator.
    pub fn set_video_layer_generator_published(
        &self,
        layer_id: VideoLayerId,
        generator: VideoGenerator,
    ) -> Result<(), String> {
        generator.validate()?;
        self.media_asset_transaction_published(MediaAssetTransaction::UpdateGenerator {
            layer_id,
            generator,
        })
    }
}

fn generator_layer_transaction(
    layer_id: VideoLayerId,
    asset_id: MediaAssetId,
    clip_slot_id: VideoClipSlotId,
    label: String,
    generator: VideoGenerator,
) -> Result<MediaAssetTransaction, String> {
    let source = VideoSourceSummary::from_generator(generator)?;
    let label = sanitize_video_layer_label(label, layer_id);
    Ok(MediaAssetTransaction::Import(MediaAssetImportCandidate {
        assets: vec![MediaAssetSummary {
            id: asset_id,
            label: label.clone(),
            source: source.clone(),
            content_hash: None,
            byte_size: None,
        }],
        layers: vec![VideoLayerSummary {
            id: layer_id,
            label,
            source,
            media_asset_id: Some(asset_id),
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
            clip_slots: vec![VideoClipSlotSummary {
                id: clip_slot_id,
                media_asset_id: asset_id,
                in_point_ms: 0,
                out_point_ms: None,
                loop_mode: VideoClipLoopMode::Once,
                speed: 1.0,
                cue_points: Vec::new(),
                launch_quantization: VideoClipLaunchQuantization::Immediate,
                effect_overrides: Vec::new(),
            }],
            default_clip_slot_id: Some(clip_slot_id),
        }],
    }))
}

impl EngineRuntime {
    pub(super) fn updated_video_generator_asset(
        &self,
        layer_id: VideoLayerId,
        generator: VideoGenerator,
    ) -> Result<MediaAssetSummary, String> {
        generator.validate()?;
        let layer = self
            .video_layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .ok_or_else(|| format!("Video layer {layer_id} was not found"))?;
        if !matches!(layer.source.kind, VideoSourceKind::Generator(_)) {
            return Err(format!("Video layer {layer_id} is not a generator"));
        }
        let mut asset = self
            .media_assets
            .iter()
            .find(|asset| asset.id == layer.media_asset_id)
            .cloned()
            .ok_or_else(|| format!("Video layer {layer_id} generator media asset was not found"))?;
        if !matches!(asset.source.kind, VideoSourceKind::Generator(_)) {
            return Err(format!(
                "Video layer {layer_id} media asset is not a generator"
            ));
        }
        asset.source.kind = VideoSourceKind::Generator(generator);
        generator.validate_source(&asset.source)?;
        Ok(asset)
    }
}

/// Keep Auto VJ's stable revision sensitive to generator parameters as well
/// as source kind. The byte order is explicit and independent of std Hash.
pub(super) fn hash_video_generator_revision(
    mut hash: u64,
    generator: &VideoGenerator,
    hash_bytes: impl Fn(u64, &[u8]) -> u64,
) -> u64 {
    match generator {
        VideoGenerator::Solid { color } => {
            hash = hash_bytes(hash, &[0]);
            hash_bytes(hash, color)
        }
        VideoGenerator::LinearGradient {
            color_a,
            color_b,
            angle_degrees,
        } => {
            hash = hash_bytes(hash, &[1]);
            hash = hash_bytes(hash, color_a);
            hash = hash_bytes(hash, color_b);
            hash_bytes(hash, &angle_degrees.to_le_bytes())
        }
        VideoGenerator::Noise {
            seed,
            scale,
            speed,
            color_a,
            color_b,
        } => {
            hash = hash_bytes(hash, &[2]);
            hash = hash_bytes(hash, &seed.to_le_bytes());
            hash = hash_bytes(hash, &scale.to_le_bytes());
            hash = hash_bytes(hash, &speed.to_le_bytes());
            hash = hash_bytes(hash, color_a);
            hash_bytes(hash, color_b)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(value: u8) -> VideoGenerator {
        VideoGenerator::Solid {
            color: [value, 0, 0, 255],
        }
    }

    fn runtime() -> EngineRuntime {
        EngineRuntime::new(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        })
    }

    fn authored_video(snapshot: &EngineSnapshot) -> Option<VideoSnapshot> {
        snapshot.authored_video.clone().map(|mut video| {
            // Persistence snapshots retain this runtime observation. Loading
            // recomputes show_revision, so it is not part of the authored
            // generator/history image; keep the authored Auto VJ config.
            video.auto_vj.status = AutoVjStatus::default();
            video
        })
    }

    fn publish(
        runtime: &mut EngineRuntime,
        published: &RwLock<EngineSnapshot>,
        transaction: MediaAssetTransaction,
    ) -> Result<(), String> {
        let (ack, receiver) = mpsc::sync_channel(1);
        runtime.apply_command(EngineCommand::MediaAssetTransactionPublished {
            transaction,
            expires_at: Instant::now() + Duration::from_secs(1),
            admission: ProjectSnapshotLoadAdmission::new(),
            ack,
        });
        runtime.publish_pending_command_acks(0, published);
        receiver.recv_timeout(Duration::from_secs(1)).unwrap()
    }

    #[test]
    fn generator_publication_updates_catalog_and_active_source_with_rollback() {
        let mut runtime = runtime();
        let published = RwLock::new(runtime.build_snapshot(0));
        publish(
            &mut runtime,
            &published,
            generator_layer_transaction(1, 2, VideoClipSlotId(3), "Solid".into(), solid(10))
                .unwrap(),
        )
        .unwrap();
        let before = runtime.video_snapshot();
        // Media asset transactions wait for definitive publication when the
        // shared reader is busy. Exercise rollback through the explicit
        // publication-failure hook rather than deadlocking this thread on its
        // own reader.
        runtime.fail_next_pending_publication = true;
        let error = publish(
            &mut runtime,
            &published,
            MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: solid(20),
            },
        )
        .unwrap_err();
        assert!(error.contains("rolled back"));
        assert_eq!(runtime.video_snapshot(), before);
        publish(
            &mut runtime,
            &published,
            MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: solid(20),
            },
        )
        .unwrap();
        let after = runtime.video_snapshot();
        assert_eq!(
            after.layers[0].source.kind,
            VideoSourceKind::Generator(solid(20))
        );
        assert_eq!(after.media_assets[0].source, after.layers[0].source);
        assert_eq!(after.layers[0].clip_slots, before.layers[0].clip_slots);
        assert_eq!(
            published.read().unwrap().video.layers[0].source.kind,
            after.layers[0].source.kind
        );
    }

    #[test]
    fn generator_commands_roundtrip_history_images_and_reject_invalid_edits() {
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let before = engine.persistence_snapshot().unwrap();
        engine
            .create_video_generator_layer_published(
                1,
                2,
                VideoClipSlotId(3),
                "Solid".into(),
                solid(1),
            )
            .unwrap();
        let created = engine.persistence_snapshot().unwrap();
        engine
            .set_video_layer_generator_published(1, solid(2))
            .unwrap();
        let changed = engine.persistence_snapshot().unwrap();
        assert_ne!(authored_video(&changed), authored_video(&created));
        engine
            .load_project_snapshot_and_wait(created.clone())
            .unwrap();
        assert_eq!(
            authored_video(&engine.persistence_snapshot().unwrap()),
            authored_video(&created)
        );
        engine
            .load_project_snapshot_and_wait(changed.clone())
            .unwrap();
        assert_eq!(
            authored_video(&engine.persistence_snapshot().unwrap()),
            authored_video(&changed)
        );
        let invalid = VideoGenerator::Noise {
            seed: 0,
            scale: 0,
            speed: 0,
            color_a: [0; 4],
            color_b: [255; 4],
        };
        assert!(engine
            .set_video_layer_generator_published(1, invalid)
            .is_err());
        assert!(engine
            .set_video_layer_generator_published(99, solid(3))
            .is_err());
        assert_eq!(
            authored_video(&engine.persistence_snapshot().unwrap()),
            authored_video(&changed)
        );
        engine
            .load_project_snapshot_and_wait(before.clone())
            .unwrap();
        assert_eq!(
            authored_video(&engine.persistence_snapshot().unwrap()),
            authored_video(&before)
        );
    }

    #[test]
    fn generator_clock_is_unbounded_and_parameter_edits_preserve_transport() {
        let mut runtime = runtime();
        runtime
            .apply_media_asset_transaction(
                generator_layer_transaction(1, 2, VideoClipSlotId(3), "Noise".into(), solid(1))
                    .unwrap(),
            )
            .unwrap();
        runtime.video_layers[0].state.playing = true;
        runtime.advance_video_layers(Duration::from_millis(2345));
        assert_eq!(runtime.video_layers[0].state.position_ms, 2345);
        assert!(runtime.video_layers[0].state.playing);
        runtime
            .apply_media_asset_transaction(MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: solid(2),
            })
            .unwrap();
        assert_eq!(runtime.video_layers[0].state.position_ms, 2345);
        assert!(runtime.video_layers[0].state.playing);
        runtime.video_layers[0].state.playing = false;
        runtime.advance_video_layers(Duration::from_secs(5));
        assert_eq!(runtime.video_layers[0].state.position_ms, 2345);
    }

    #[test]
    fn generator_edit_rejects_non_generator_sources_without_replacing_them() {
        let mut runtime = runtime();
        runtime
            .add_video_layer_with_asset(
                1,
                2,
                VideoClipSlotId(3),
                "File".into(),
                VideoSourceSummary {
                    kind: VideoSourceKind::File,
                    path: Some("preserved.mp4".into()),
                    name: None,
                    codec: None,
                    metadata: None,
                },
            )
            .unwrap();
        let before = runtime.video_snapshot();
        let error = runtime
            .apply_media_asset_transaction(MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: solid(99),
            })
            .unwrap_err();
        assert!(error.contains("is not a generator"));
        assert_eq!(runtime.video_snapshot(), before);
    }
}
