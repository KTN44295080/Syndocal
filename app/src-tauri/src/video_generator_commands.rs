//! Generator authoring uses the existing owner-bound media publication lane.
//! Rendering and parameter policy remain in video/protocol, respectively.
use super::*;

pub(super) struct GeneratorEditTicket {
    pub project_transaction_id: u64,
    pub expected_epoch: u64,
    pub owner_id: String,
}

fn publish<T>(
    state: &AppState,
    window_label: &str,
    ticket: GeneratorEditTicket,
    mutation: impl FnOnce(&EngineHandle) -> Result<T, String>,
) -> Result<T, String> {
    let owner_id = normalize_project_transaction_owner_id(ticket.owner_id)?;
    project_transaction_owner_binding_for_window(state, window_label, &owner_id)?;
    let (_external_admission, mut coordinator) = (
        lock_project_external_command_admission_for_display_finalize(state)?,
        lock_project_coordinator(state)?,
    );
    let authority = media_asset_prepare_authority(&coordinator);
    let _transaction_admission = validate_media_asset_commit_ticket_and_authority(
        state,
        &mut coordinator,
        ticket.project_transaction_id,
        ticket.expected_epoch,
        &owner_id,
        &authority,
    )?;
    mutation(&state.engine)
}

pub(super) fn create(
    state: &AppState,
    window_label: &str,
    ticket: GeneratorEditTicket,
    label: String,
    generator: protocol::VideoGenerator,
) -> Result<VideoLayerId, String> {
    generator.validate()?;
    let label = normalize_video_layer_label(label)?;
    publish(state, window_label, ticket, |engine| {
        let layer_id = engine.allocate_video_layer_id();
        engine.create_video_generator_layer_published(
            layer_id,
            engine.allocate_media_asset_id(),
            engine.allocate_video_clip_slot_id(),
            label,
            generator,
        )?;
        Ok(layer_id)
    })
}

pub(super) fn update(
    state: &AppState,
    window_label: &str,
    ticket: GeneratorEditTicket,
    layer_id: VideoLayerId,
    generator: protocol::VideoGenerator,
) -> Result<(), String> {
    generator.validate()?;
    publish(state, window_label, ticket, |engine| {
        engine.set_video_layer_generator_published(layer_id, generator)
    })
}

/// Resolve a parameter edit against the rendered source exactly once before
/// projecting a candidate into its authored/default persistence image.
pub(super) fn candidate_asset(
    video: &protocol::VideoSnapshot,
    layer_id: VideoLayerId,
    generator: protocol::VideoGenerator,
) -> Result<MediaAssetSummary, String> {
    generator.validate()?;
    let layer = video
        .layers
        .iter()
        .find(|layer| layer.id == layer_id)
        .ok_or_else(|| format!("Video layer {layer_id} was not found"))?;
    if !matches!(layer.source.kind, VideoSourceKind::Generator(_)) {
        return Err(format!("Video layer {layer_id} is not a generator"));
    }
    let mut asset = video
        .media_assets
        .iter()
        .find(|asset| Some(asset.id) == layer.media_asset_id)
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

pub(super) fn resolve_candidate_transaction(
    snapshot: &mut EngineSnapshot,
    transaction: &MediaAssetTransaction,
) -> Result<Option<MediaAssetTransaction>, String> {
    match transaction {
        MediaAssetTransaction::UpdateGenerator {
            layer_id,
            generator,
        } => {
            // The rendered asset can differ from the authored default.
            // Resolve its identity before projecting persistence truth, then
            // update that asset in the canonical authored candidate. Resolving
            // from the authored default instead would change the wrong clip.
            let asset = candidate_asset(&snapshot.video, *layer_id, *generator)?;
            use_authored_video_snapshot(snapshot);
            Ok(Some(MediaAssetTransaction::Update(asset)))
        }
        MediaAssetTransaction::Import(_)
        | MediaAssetTransaction::BootstrapVjShow { .. }
        | MediaAssetTransaction::Update(_) => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn begin(state: &AppState, command: &str, sequence: u64) -> ProjectTransactionTicket {
        let coordinator = state.project_coordinator.lock().unwrap();
        let request = BeginProjectTransactionRequest {
            label: command.to_string(),
            coalesce_key: String::new(),
            expected_epoch: coordinator.epoch,
            expected_revision: coordinator.revision,
            expected_checkpoint_hash: coordinator.checkpoint_hash.clone(),
            owner_id: crate::tests::MEDIA_ASSET_A6_OWNER.to_string(),
            client_operation_id: format!("project-op:{sequence}:{command}"),
            shape_fingerprint: canonical_project_transaction_shape(command, command, ""),
            command_name: command.to_string(),
            schema_version: PROJECT_TRANSACTION_SCHEMA_VERSION,
        };
        drop(coordinator);
        begin_project_transaction_for_window_label(state, "media-asset-a6", request).unwrap()
    }

    fn edit_ticket(ticket: &ProjectTransactionTicket) -> GeneratorEditTicket {
        GeneratorEditTicket {
            project_transaction_id: ticket.transaction_id,
            expected_epoch: ticket.project_epoch,
            owner_id: ticket.owner_id.clone(),
        }
    }

    fn commit(state: &AppState, ticket: ProjectTransactionTicket, command: &str) {
        commit_project_transaction_for_window_label(
            state,
            "media-asset-a6",
            CommitProjectTransactionRequest {
                transaction_id: ticket.transaction_id,
                expected_epoch: ticket.project_epoch,
                client_operation_id: ticket.client_operation_id,
                shape_fingerprint: ticket.shape_fingerprint,
                command_name: command.to_string(),
                schema_version: PROJECT_TRANSACTION_SCHEMA_VERSION,
                owner_id: ticket.owner_id,
            },
        )
        .unwrap();
    }

    #[test]
    fn generator_candidate_resolves_active_asset_before_authored_projection() {
        let harness = crate::tests::MediaAssetA6CommandHarness::new();
        let engine = &harness.state.engine;
        let first = protocol::VideoGenerator::Solid { color: [10; 4] };
        let second = protocol::VideoGenerator::Solid { color: [20; 4] };
        for (layer, asset, slot, generator) in [(1, 2, 3, first), (4, 5, 6, second)] {
            engine
                .create_video_generator_layer_published(
                    layer,
                    asset,
                    VideoClipSlotId(slot),
                    format!("Generator {layer}"),
                    generator,
                )
                .unwrap();
        }
        let mut before = engine.persistence_snapshot().unwrap();
        let mut active_slot = before.video.layers[1].clip_slots[0].clone();
        active_slot.id = VideoClipSlotId(7);
        before.video.layers[0].clip_slots.push(active_slot.clone());
        before.authored_video.as_mut().unwrap().layers[0]
            .clip_slots
            .push(active_slot);
        // Layer 1 is rendering its non-default asset; its authored projection
        // still belongs to the first asset. Another layer shares the active
        // asset, so its authored source must change with the catalog.
        before.video.layers[0].source = before.video.layers[1].source.clone();
        before.video.layers[0].media_asset_id = Some(5);
        let replacement = protocol::VideoGenerator::Solid { color: [30; 4] };
        let candidate = media_asset_transaction_candidate_snapshot(
            before.clone(),
            &MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: replacement,
            },
        )
        .unwrap();
        assert!(candidate.authored_video.is_none());
        let authored = &candidate.video;
        assert_eq!(
            authored.layers[0].source.kind,
            VideoSourceKind::Generator(first)
        );
        assert_eq!(authored.layers[0].media_asset_id, Some(2));
        assert_eq!(
            authored.layers[1].source.kind,
            VideoSourceKind::Generator(replacement)
        );
        assert_eq!(
            authored.media_assets[0].source.kind,
            VideoSourceKind::Generator(first)
        );
        assert_eq!(
            authored.media_assets[1].source.kind,
            VideoSourceKind::Generator(replacement)
        );
        assert_eq!(
            authored.layers[0].clip_slots,
            before.video.layers[0].clip_slots
        );

        let invalid = protocol::VideoGenerator::LinearGradient {
            color_a: [0; 4],
            color_b: [255; 4],
            angle_degrees: 360,
        };
        assert!(media_asset_transaction_candidate_snapshot(
            before.clone(),
            &MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: invalid
            },
        )
        .is_err());
        before.video.layers[0].source.kind = VideoSourceKind::File;
        let error = media_asset_transaction_candidate_snapshot(
            before,
            &MediaAssetTransaction::UpdateGenerator {
                layer_id: 1,
                generator: replacement,
            },
        )
        .unwrap_err();
        assert!(error.contains("is not a generator"));
    }

    #[test]
    fn generator_commands_enforce_ticket_fences_and_record_durable_history() {
        let harness = crate::tests::MediaAssetA6CommandHarness::new();
        let state = &harness.state;
        let first = protocol::VideoGenerator::Solid {
            color: [10, 20, 30, 255],
        };
        let ticket = begin(state, "add_video_generator_layer", 1);
        let before = state.engine.persistence_snapshot().unwrap().authored_video;
        let mut stale = edit_ticket(&ticket);
        stale.expected_epoch += 1;
        assert!(create(state, "media-asset-a6", stale, "Solid".into(), first).is_err());
        assert!(create(
            state,
            "wrong-window",
            edit_ticket(&ticket),
            "Solid".into(),
            first
        )
        .is_err());
        let mut wrong_owner = edit_ticket(&ticket);
        wrong_owner.owner_id = "renderer:other".into();
        assert!(create(state, "media-asset-a6", wrong_owner, "Solid".into(), first).is_err());
        assert_eq!(
            state.engine.persistence_snapshot().unwrap().authored_video,
            before
        );
        let layer_id = create(
            state,
            "media-asset-a6",
            edit_ticket(&ticket),
            "Solid".into(),
            first,
        )
        .unwrap();
        commit(state, ticket, "add_video_generator_layer");

        let ticket = begin(state, "set_video_layer_generator", 2);
        let invalid = protocol::VideoGenerator::Noise {
            seed: 0,
            scale: 0,
            speed: 0,
            color_a: [0; 4],
            color_b: [255; 4],
        };
        assert!(update(
            state,
            "media-asset-a6",
            edit_ticket(&ticket),
            layer_id,
            invalid
        )
        .is_err());
        let second = protocol::VideoGenerator::Solid {
            color: [40, 50, 60, 255],
        };
        update(
            state,
            "media-asset-a6",
            edit_ticket(&ticket),
            layer_id,
            second,
        )
        .unwrap();
        commit(state, ticket, "set_video_layer_generator");

        let coordinator = state.project_coordinator.lock().unwrap();
        assert_eq!(coordinator.history.undo.len(), 2);
        assert!(coordinator.history.pending.is_empty());
        let edit = coordinator.history.undo.last().unwrap();
        assert_eq!(
            edit.before.project.snapshot.video.layers[0].source.kind,
            VideoSourceKind::Generator(first)
        );
        assert_eq!(
            edit.after.project.snapshot.video.layers[0].source.kind,
            VideoSourceKind::Generator(second)
        );
        assert_eq!(
            state
                .engine
                .persistence_snapshot()
                .unwrap()
                .authored_video
                .unwrap()
                .layers[0]
                .source
                .kind,
            VideoSourceKind::Generator(second)
        );
    }

    #[test]
    fn generators_are_pathless_preview_sources_and_reject_contradictory_media() {
        let generator = protocol::VideoGenerator::Noise {
            seed: 42,
            scale: 8,
            speed: 100,
            color_a: [0, 0, 0, 255],
            color_b: [255, 255, 255, 255],
        };
        let mut layer = protocol::VideoLayerSummary {
            id: 1,
            label: "Generator preview".into(),
            source: VideoSourceSummary::from_generator(generator).unwrap(),
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        };
        validate_project_video_source(&layer.source, "generator").unwrap();
        validate_vj_preview_layer(&layer).unwrap();
        assert_eq!(video_input_backend(&layer.source.kind), None);
        assert!(video_source_file_dialog_filter(&layer.source.kind).is_err());
        assert!(legacy_media_asset_refresh_message(&layer.source).is_err());
        let now = Instant::now();
        let mut transport = VjPreviewTransportRuntime::default();
        transport.stage_at(&layer, now, 0).unwrap();
        transport.set_playing_at(true, now, 0).unwrap();
        let advanced = transport.summary_at(now + Duration::from_millis(500), 500);
        assert!(advanced.playing);
        assert_eq!(advanced.position_ms, 500);
        assert_eq!(advanced.duration_ms, None);
        layer.source.path = Some("unexpected.mp4".into());
        assert!(validate_project_video_source(&layer.source, "generator").is_err());
        assert!(validate_vj_preview_layer(&layer).is_err());
    }
}
