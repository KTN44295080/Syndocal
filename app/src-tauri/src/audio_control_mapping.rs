//! Ticket-bound runtime audio writes. Mapping edits use the existing project
//! coordinator; these live values never publish an authored revision or Undo.
use super::*;
use protocol::audio_mapping::{
    audio_control_target, AudioControlRuntimeValue, MAX_AUDIO_CONTROL_MAPPINGS,
};

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub(super) struct MappedValue {
    mapping_id: String,
    value: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct RuntimeRequest {
    pub owner_id: String,
    pub expected_epoch: u64,
    pub expected_revision: u64,
    pub expected_checkpoint_hash: String,
    pub request_id: u64,
    pub audio_generation: u64,
    pub feature_sequence: u64,
    pub values: Vec<MappedValue>,
}

#[derive(Default)]
pub(super) struct RuntimeAdmission {
    last: Option<RuntimeRequest>,
    checked_external_authority: Option<(u64, u64)>,
}

#[derive(Debug, Serialize)]
pub(super) struct RuntimeResult {
    project_epoch: u64,
    project_revision: u64,
    checkpoint_hash: String,
}

pub(super) fn apply(
    state: &AppState,
    window_label: &str,
    request: RuntimeRequest,
) -> Result<RuntimeResult, String> {
    if request.request_id == 0 || request.request_id > 9_007_199_254_740_991 {
        return Err("Audio control request id must be a positive safe integer".to_string());
    }
    if request.values.len() > MAX_AUDIO_CONTROL_MAPPINGS {
        return Err("Audio control batch exceeds the bounded mapping limit".to_string());
    }
    project_transaction_owner_binding_for_window(state, window_label, &request.owner_id)?;
    let _external_admission = lock_project_external_command_admission(state)?;
    let mut coordinator = lock_project_coordinator(state)?;
    project_transaction_owner_binding_for_window(state, window_label, &request.owner_id)?;
    ensure_no_pending_project_transaction(&coordinator)?;
    if state.project_transaction_active.load(Ordering::Acquire) {
        return Err("Project transaction is active; retry the audio control update".to_string());
    }
    let mut admission = state
        .audio_control_runtime
        .lock()
        .map_err(|_| "Audio control runtime admission lock was poisoned".to_string())?;
    // Legacy MIDI/OSC/remote authored commands have a separate admission
    // generation. Reconcile only after one of those writes (or project load),
    // so a stable 20 Hz live stream does not clone/hash the entire project.
    let external_authority = (
        coordinator.epoch,
        state.project_external_command_admission.generation(),
    );
    if admission.checked_external_authority != Some(external_authority) {
        reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;
        admission.checked_external_authority = Some(external_authority);
    }
    if coordinator.epoch != request.expected_epoch
        || coordinator.revision != request.expected_revision
        || coordinator.checkpoint_hash != request.expected_checkpoint_hash
    {
        return Err("Audio control ticket belongs to a stale project authority".to_string());
    }
    ensure_project_operator_runtime_mutation_allowed(state, &coordinator, &request.owner_id)?;
    let result = || RuntimeResult {
        project_epoch: coordinator.epoch,
        project_revision: coordinator.revision,
        checkpoint_hash: coordinator.checkpoint_hash.clone(),
    };
    if let Some(last) = admission.last.as_ref().filter(|last| {
        last.owner_id == request.owner_id && last.expected_epoch == request.expected_epoch
    }) {
        if request.request_id < last.request_id {
            return Err("Audio control ticket was superseded".to_string());
        }
        if request.request_id == last.request_id {
            return if last == &request {
                Ok(result())
            } else {
                Err("Audio control request id was reused with different values".to_string())
            };
        }
    }
    let mut seen = HashSet::new();
    let mut values = Vec::with_capacity(request.values.len());
    for input in &request.values {
        if !seen.insert(input.mapping_id.as_str()) {
            return Err("Audio control batch contains duplicate mapping ids".to_string());
        }
        let mapping = coordinator
            .mappings
            .midi_mappings
            .iter()
            .find(|mapping| {
                mapping.message == protocol::MidiControlMessage::Audio
                    && mapping
                        .audio
                        .as_ref()
                        .is_some_and(|audio| audio.id == input.mapping_id)
            })
            .ok_or_else(|| "Audio control mapping was removed or replaced".to_string())?;
        let target = audio_control_target(mapping)?
            .ok_or_else(|| "Audio triggers must use the existing Perform dispatcher".to_string())?;
        if !input.value.is_finite()
            || input.value < mapping.low.min(mapping.high) - 0.000001
            || input.value > mapping.low.max(mapping.high) + 0.000001
        {
            return Err("Audio value falls outside its persisted output range".to_string());
        }
        values.push(AudioControlRuntimeValue {
            target,
            value: input.value.clamp(0.0, 1.0),
        });
    }
    state.engine.set_audio_control_runtime_published(
        values,
        request.audio_generation,
        request.feature_sequence,
    )?;
    admission.last = Some(request);
    Ok(result())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::{MediaAssetA6CommandHarness, MEDIA_ASSET_A6_OWNER};

    fn mapping() -> MidiControlMapping {
        serde_json::from_value(serde_json::json!({
            "message": "Audio", "number": 0, "channel": null,
            "action": "VideoMaster", "low": 0.1, "high": 0.9,
            "audio": {
                "id": "audio-master-27", "source": "Level", "mode": "Continuous",
                "gain": 1.0, "attack_ms": 20.0, "release_ms": 120.0, "threshold": 0.5
            }
        }))
        .unwrap()
    }

    fn prepare(harness: &MediaAssetA6CommandHarness) -> RuntimeRequest {
        prepare_mapping(harness, mapping())
    }

    fn prepare_mapping(
        harness: &MediaAssetA6CommandHarness,
        mapping: MidiControlMapping,
    ) -> RuntimeRequest {
        let mapping_id = mapping.audio.as_ref().unwrap().id.clone();
        let mut coordinator = harness.state.project_coordinator.lock().unwrap();
        coordinator.mappings.midi_mappings = validate_midi_control_mappings(vec![mapping]).unwrap();
        let checkpoint =
            reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator).unwrap();
        RuntimeRequest {
            owner_id: MEDIA_ASSET_A6_OWNER.to_string(),
            expected_epoch: checkpoint.epoch,
            expected_revision: checkpoint.revision,
            expected_checkpoint_hash: checkpoint.hash,
            request_id: 1,
            audio_generation: 27,
            feature_sequence: 1,
            values: vec![MappedValue {
                mapping_id,
                value: 0.5,
            }],
        }
    }

    fn publish_frame(harness: &MediaAssetA6CommandHarness, sequence: u64) {
        harness
            .state
            .engine
            .send(EngineCommand::PublishLiveAudioFrame {
                captured_at: Instant::now(),
                frame: protocol::LiveAudioFrame {
                    generation: 27,
                    feature_sequence: sequence,
                    spectrum: protocol::AudioSpectrumPoint {
                        time_ms: sequence,
                        bass: 0.5,
                        mid: 0.5,
                        high: 0.5,
                    },
                    features: protocol::LiveAudioReactiveFeatures::default(),
                    onset_feature_sequences: [0; protocol::MAX_LIVE_AUDIO_FRAME_ONSETS],
                    onset_count: 0,
                    textures: None,
                },
            })
            .unwrap();
    }

    #[test]
    fn audio_runtime_values_do_not_flood_undo_or_advance_project_authority() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut request = prepare(&harness);
        let before = {
            let coordinator = harness.state.project_coordinator.lock().unwrap();
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.checkpoint_hash.clone(),
                coordinator.history_generation,
                coordinator.publication_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
            )
        };
        for sequence in 1..=64 {
            publish_frame(&harness, sequence);
            request.request_id = sequence;
            request.feature_sequence = sequence;
            request.values[0].value = 0.1 + (sequence % 8) as f32 * 0.1;
            let result = apply(&harness.state, "media-asset-a6", request.clone()).unwrap();
            assert_eq!(result.project_revision, before.1);
            assert_eq!(result.checkpoint_hash, before.2);
        }
        let mut coordinator = harness.state.project_coordinator.lock().unwrap();
        let checkpoint =
            reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator).unwrap();
        assert_eq!(checkpoint.hash, before.2);
        assert_eq!(
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.checkpoint_hash.clone(),
                coordinator.history_generation,
                coordinator.publication_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
            ),
            before,
        );
        assert_eq!(checkpoint.project.snapshot.video.master_opacity, 1.0);
    }

    #[test]
    fn audio_group_runtime_uses_typed_target_without_authoring_opacity_or_history() {
        let harness = MediaAssetA6CommandHarness::new();
        let engine = &harness.state.engine;
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Audio group layer".to_string(),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://audio-group-27.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        let authored = engine
            .persistence_snapshot()
            .unwrap()
            .authored_video
            .unwrap();
        let composition_id = authored
            .compositions
            .iter()
            .find(|composition| composition.layer_ids.contains(&layer_id))
            .unwrap()
            .id;
        let group_id = protocol::VideoLayerGroupId(27);
        engine
            .apply_video_effect_catalog(
                authored.effect_chains,
                authored.effect_presets,
                vec![protocol::VideoLayerGroupSummary {
                    id: group_id,
                    label: "Audio group".to_string(),
                    composition_id,
                    layer_ids: vec![layer_id],
                    opacity: 0.7,
                    ..Default::default()
                }],
            )
            .unwrap();
        let mut group_mapping = mapping();
        group_mapping.action = protocol::MidiControlAction::VideoGroupOpacity;
        group_mapping.video_group_id = Some(group_id);
        let request = prepare_mapping(&harness, group_mapping.clone());
        let before = {
            let coordinator = harness.state.project_coordinator.lock().unwrap();
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.checkpoint_hash.clone(),
                coordinator.history_generation,
                coordinator.publication_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
            )
        };
        publish_frame(&harness, 1);
        apply(&harness.state, "media-asset-a6", request).unwrap();
        assert_eq!(engine.snapshot().video.layer_groups[0].opacity, 0.5);
        assert_eq!(engine.snapshot_for_ui().video.layer_groups[0].opacity, 0.7);
        assert_eq!(
            engine
                .persistence_snapshot()
                .unwrap()
                .authored_video
                .unwrap()
                .layer_groups[0]
                .opacity,
            0.7,
        );
        let mut coordinator = harness.state.project_coordinator.lock().unwrap();
        let checkpoint =
            reconcile_project_checkpoint_for_coordinator(&harness.state, &mut coordinator).unwrap();
        assert_eq!(checkpoint.hash, before.2);
        assert_eq!(coordinator.mappings.midi_mappings, vec![group_mapping]);
        assert_eq!(
            (
                coordinator.epoch,
                coordinator.revision,
                coordinator.checkpoint_hash.clone(),
                coordinator.history_generation,
                coordinator.publication_generation,
                coordinator.history.undo.len(),
                coordinator.history.redo.len(),
            ),
            before,
        );
    }

    #[test]
    fn audio_runtime_ticket_rejects_stale_owner_replay_range_and_removed_mapping() {
        let harness = MediaAssetA6CommandHarness::new();
        let request = prepare(&harness);
        publish_frame(&harness, 1);
        apply(&harness.state, "media-asset-a6", request.clone()).unwrap();
        apply(&harness.state, "media-asset-a6", request.clone()).unwrap();
        let mut changed = request.clone();
        changed.values[0].value = 0.6;
        assert!(apply(&harness.state, "media-asset-a6", changed)
            .unwrap_err()
            .contains("reused"));
        let mut stale = request.clone();
        stale.request_id = 2;
        stale.expected_revision += 1;
        assert!(apply(&harness.state, "media-asset-a6", stale)
            .unwrap_err()
            .contains("stale"));
        assert!(apply(&harness.state, "wrong-window", request.clone()).is_err());
        let mut outside = request.clone();
        outside.request_id = 2;
        outside.values[0].value = 1.0;
        assert!(apply(&harness.state, "media-asset-a6", outside)
            .unwrap_err()
            .contains("range"));
        harness
            .state
            .project_coordinator
            .lock()
            .unwrap()
            .mappings
            .midi_mappings
            .clear();
        let mut removed = request;
        removed.request_id = 2;
        assert!(apply(&harness.state, "media-asset-a6", removed)
            .unwrap_err()
            .contains("removed"));
    }
    #[test]
    fn audio_runtime_reconciles_external_authored_changes_and_blocks_active_transactions() {
        let harness = MediaAssetA6CommandHarness::new();
        let mut request = prepare(&harness);
        publish_frame(&harness, 1);
        apply(&harness.state, "media-asset-a6", request.clone()).unwrap();
        request.request_id = 2;
        harness
            .state
            .project_transaction_active
            .store(true, Ordering::Release);
        assert!(apply(&harness.state, "media-asset-a6", request.clone())
            .unwrap_err()
            .contains("transaction is active"));
        harness
            .state
            .project_transaction_active
            .store(false, Ordering::Release);
        {
            let _external = lock_project_external_command_admission(&harness.state).unwrap();
            harness
                .state
                .engine
                .send(EngineCommand::SetVideoMasterOpacity(0.7))
                .unwrap();
            harness
                .state
                .project_external_command_admission
                .note_admitted_external_command();
        }
        assert!(apply(&harness.state, "media-asset-a6", request.clone())
            .unwrap_err()
            .contains("stale project authority"));
        let coordinator = harness.state.project_coordinator.lock().unwrap();
        assert_eq!(coordinator.revision, request.expected_revision + 1);
        assert!(coordinator.history.undo.is_empty());
        assert_ne!(
            coordinator.checkpoint_hash,
            request.expected_checkpoint_hash
        );
    }
}
