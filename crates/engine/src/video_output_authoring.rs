//! Exact, acknowledged video-output authoring without reloading the project.
use super::*;
use protocol::control_plane_command::VideoOutputConfigurationV2;

/// Produce the persisted B image used by the native project transaction.
/// Runtime playback is deliberately absent from this authoring projection.
pub fn candidate_snapshot(
    before: &EngineSnapshot,
    output_id: VideoOutputId,
    expected: &Option<VideoOutputSummary>,
    replacement: &Option<VideoOutputSummary>,
) -> Result<EngineSnapshot, String> {
    let authored = before.authored_video.as_ref().unwrap_or(&before.video);
    validate_change(authored, output_id, expected, replacement)?;
    validate_generic_show_spout_policy(authored, expected, replacement)?;
    let mut after = before.clone();
    update_video_image(&mut after.video, output_id, replacement);
    if let Some(video) = &mut after.authored_video {
        update_video_image(video, output_id, replacement);
    }
    if replacement.is_none() {
        for cue in &mut after.cues {
            cue.video_output_targets
                .retain(|target| target.output_id != output_id);
            for part in &mut cue.parts {
                part.video_output_ids.retain(|id| *id != output_id);
            }
        }
    }
    Ok(after)
}

/// The dedicated Show Spout route owns its reserved destinations, including
/// staged or incomplete saved images. Reject these generic intents before the
/// native caller creates a Pending transaction or retires any physical route.
/// Reuse the same exact-name predicate as the runtime's strict pair guard.
fn validate_generic_show_spout_policy(
    video: &VideoSnapshot,
    expected: &Option<VideoOutputSummary>,
    replacement: &Option<VideoOutputSummary>,
) -> Result<(), String> {
    if expected
        .as_ref()
        .is_some_and(show_spout_reserved_name_collision)
        || replacement
            .as_ref()
            .is_some_and(show_spout_reserved_name_collision)
    {
        return Err(
            "Generic video output authoring is blocked for reserved Show Spout names; use the dedicated Show Spout route"
                .to_string(),
        );
    }
    // An exact pair owns the entire Spout sender set. Disabled or partial
    // reserved images also reject additional Spout routes in the app's generic
    // synchronizer. Ordinary non-Spout edits and removal of an extra ordinary
    // sender do not claim or modify either managed destination.
    if replacement
        .as_ref()
        .is_some_and(|output| output.kind == VideoOutputKind::SpoutSender)
        && video.outputs.iter().any(show_spout_reserved_name_collision)
    {
        return Err(
            "Generic Spout creation or configuration is blocked while reserved Show Spout outputs are authored; use the dedicated Show Spout route"
                .to_string(),
        );
    }
    Ok(())
}

fn update_video_image(
    video: &mut VideoSnapshot,
    output_id: VideoOutputId,
    replacement: &Option<VideoOutputSummary>,
) {
    if let Some(index) = video
        .outputs
        .iter()
        .position(|output| output.id == output_id)
    {
        if let Some(replacement) = replacement {
            video.outputs[index] = replacement.clone();
        } else {
            video.outputs.remove(index);
        }
    } else if let Some(replacement) = replacement {
        video.outputs.push(replacement.clone());
    }
    for composition in &mut video.compositions {
        composition.output_ids = video
            .outputs
            .iter()
            .filter(|output| output.composition_id == composition.id)
            .map(|output| output.id)
            .collect();
    }
    if replacement.is_none() {
        video
            .effect_chains
            .retain(|chain| chain.scope != (VideoEffectScope::Output { output_id }));
    }
}

fn validate_change(
    video: &VideoSnapshot,
    output_id: VideoOutputId,
    expected: &Option<VideoOutputSummary>,
    replacement: &Option<VideoOutputSummary>,
) -> Result<(), String> {
    if output_id == 0 || output_id > 9_007_199_254_740_991 {
        return Err("Video output identity is outside the safe range".into());
    }
    if expected.is_none() && replacement.is_none() {
        return Err("Video output authoring requires a target image".into());
    }
    let current = video.outputs.iter().find(|output| output.id == output_id);
    if current != expected.as_ref()
        || expected
            .as_ref()
            .is_some_and(|output| output.id != output_id)
    {
        return Err("Video output changed before authoritative publication".into());
    }
    let Some(replacement) = replacement else {
        return Ok(());
    };
    if replacement.id != output_id {
        return Err("Video output authoring cannot replace its identity".into());
    }
    VideoOutputConfigurationV2 {
        kind: replacement.kind.clone(),
        label: replacement.label.clone(),
        width: replacement.width,
        height: replacement.height,
        fullscreen: replacement.fullscreen,
        monitor_index: replacement.monitor_id,
        monitor_identity: replacement.monitor_identity.clone(),
        endpoint_name: replacement.endpoint_name.clone(),
    }
    .validate()
    .map_err(|error| format!("Invalid video output configuration: {error:?}"))?;
    if replacement.label != replacement.label.trim()
        || replacement
            .endpoint_name
            .as_deref()
            .is_some_and(|name| name != name.trim())
    {
        return Err("Video output names must be normalized before publication".into());
    }
    if replacement.composition_id == 0
        || !video
            .compositions
            .iter()
            .any(|composition| composition.id == replacement.composition_id)
    {
        return Err(format!(
            "Video composition {} was not found",
            replacement.composition_id
        ));
    }
    if let Some(expected) = expected {
        if replacement.enabled != expected.enabled
            || replacement.composition_id != expected.composition_id
            || replacement.opacity != expected.opacity
            || replacement.blackout != expected.blackout
            || replacement.mapping != expected.mapping
        {
            return Err("Configuration cannot alter video output playback or routing".into());
        }
    } else if !replacement.enabled {
        return Err("New video outputs must be enabled".into());
    }
    for sibling in video.outputs.iter().filter(|output| output.id != output_id) {
        if sibling.kind != replacement.kind {
            continue;
        }
        if replacement.kind == VideoOutputKind::Display {
            if sibling.monitor_id == replacement.monitor_id
                || sibling.monitor_identity == replacement.monitor_identity
            {
                return Err("The selected display already has a video output".into());
            }
        } else if sibling
            .endpoint_name
            .as_deref()
            .zip(replacement.endpoint_name.as_deref())
            .is_some_and(|(left, right)| left.trim().eq_ignore_ascii_case(right.trim()))
        {
            return Err("An output of this kind already uses that endpoint name".into());
        }
    }
    Ok(())
}

impl EnginePersistenceMutationSubmission<'_> {
    /// Holds the same persistence submission interval across candidate
    /// capture, admission, acknowledged publication and native verification.
    pub fn author_video_output_published(
        &self,
        output_id: VideoOutputId,
        expected: Option<VideoOutputSummary>,
        replacement: Option<VideoOutputSummary>,
    ) -> Result<(), SnapshotPublicationFailure> {
        let timeout = PROJECT_SNAPSHOT_LOAD_ACK_TIMEOUT;
        let deadline = Instant::now() + timeout;
        let admission = ProjectSnapshotLoadAdmission::new();
        let (ack, receiver) = mpsc::sync_channel(1);
        self.engine
            .send_without_persistence_mutation_submission_gate(
                EngineCommand::AuthorVideoOutputPublished {
                    output_id,
                    expected,
                    replacement,
                    expires_at: deadline,
                    admission: admission.clone(),
                    ack,
                },
            )
            .map_err(|error| SnapshotPublicationFailure::Definitive(error.to_string()))?;
        receive_classified_snapshot_ack(
            receiver,
            &admission,
            deadline,
            timeout,
            "Video output publication acknowledgement disconnected after admission; reconcile the shared snapshot before retrying".into(),
        ).map_err(|error| {
            if error.allocator_rewind_safe {
                SnapshotPublicationFailure::Definitive(error.message)
            } else {
                SnapshotPublicationFailure::Indeterminate(error.message)
            }
        })
    }
}

/// Everything the existing destructive output lifecycle can touch. This is
/// captured once per authoring operation, never on the rendering hot path.
#[derive(Clone)]
pub(super) struct AuthoringRollback {
    outputs: Vec<RuntimeVideoOutput>,
    fades: Vec<RuntimeVideoOutputFade>,
    cues: Vec<RuntimeCue>,
    active_fade: Option<RuntimeFade>,
    chains: Vec<VideoEffectChainSummary>,
    groups: Vec<VideoLayerGroupSummary>,
    buses: Vec<VideoLayerTransitionBusSummary>,
    active_buses: Vec<RuntimeVideoLayerTransitionBus>,
    resets: Vec<PendingVideoIsfEventReset>,
    last_error: Option<String>,
}

impl AuthoringRollback {
    fn capture(runtime: &EngineRuntime) -> Self {
        Self {
            outputs: runtime.video_outputs.clone(),
            fades: runtime.video_output_fades.clone(),
            cues: runtime.cues.clone(),
            active_fade: runtime.active_fade.clone(),
            chains: runtime.video_effect_chains.clone(),
            groups: runtime.video_layer_groups.clone(),
            buses: runtime.video_transition_buses.clone(),
            active_buses: runtime.active_video_transition_buses.clone(),
            resets: runtime.pending_video_isf_event_resets.clone(),
            last_error: runtime.last_error.clone(),
        }
    }

    pub(super) fn restore(self, runtime: &mut EngineRuntime) {
        runtime.video_outputs = self.outputs;
        runtime.video_output_fades = self.fades;
        runtime.cues = self.cues;
        runtime.active_fade = self.active_fade;
        runtime.video_effect_chains = self.chains;
        runtime.video_layer_groups = self.groups;
        runtime.video_transition_buses = self.buses;
        runtime.active_video_transition_buses = self.active_buses;
        runtime.pending_video_isf_event_resets = self.resets;
        runtime.last_error = self.last_error;
    }
}

impl EngineRuntime {
    pub(super) fn apply_video_output_authoring(
        &mut self,
        output_id: VideoOutputId,
        expected: Option<VideoOutputSummary>,
        replacement: Option<VideoOutputSummary>,
        expires_at: Instant,
        admission: ProjectSnapshotLoadAdmission,
        ack: mpsc::SyncSender<Result<(), String>>,
    ) {
        let rollback = AuthoringRollback::capture(self);
        let result = if !admission.try_admit_before(expires_at) {
            Err("Video output authoring expired or was cancelled before admission".into())
        } else {
            self.author_video_output_state(output_id, &expected, &replacement)
        };
        self.last_error = if result.is_ok() {
            None
        } else {
            rollback.last_error.clone()
        };
        self.pending_command_acks.push(PendingCommandAck {
            ack: PendingCommandAckSender::Plain(ack),
            result,
            rollback: PendingCommandRollback::RestoreVideoOutputAuthoring(rollback),
            publication_error: "Video output authoring could not publish an acknowledged snapshot",
        });
    }

    fn author_video_output_state(
        &mut self,
        output_id: VideoOutputId,
        expected: &Option<VideoOutputSummary>,
        replacement: &Option<VideoOutputSummary>,
    ) -> Result<(), String> {
        let before = self.build_persistence_snapshot();
        candidate_snapshot(&before, output_id, expected, replacement)?;
        match (expected, replacement) {
            (None, Some(output)) => {
                self.reject_generic_show_spout_output_add(output)?;
                self.video_outputs
                    .push(RuntimeVideoOutput::authored(output.clone()));
            }
            (Some(_), Some(replacement)) => {
                self.reject_generic_show_spout_output_config(
                    output_id,
                    &replacement.label,
                    &replacement.kind,
                    replacement.endpoint_name.as_deref(),
                )?;
                let output = self
                    .video_outputs
                    .iter_mut()
                    .find(|output| output.summary.id == output_id)
                    .ok_or_else(|| "Video output disappeared before publication".to_string())?;
                // Preserve the live values currently owned by Cue playback.
                // Only destination configuration belongs to this operation.
                output.summary.label = replacement.label.clone();
                output.summary.kind = replacement.kind.clone();
                output.summary.fullscreen = replacement.fullscreen;
                output.summary.monitor_id = replacement.monitor_id;
                output.summary.monitor_identity = replacement.monitor_identity.clone();
                output.summary.width = replacement.width;
                output.summary.height = replacement.height;
                output.summary.endpoint_name = replacement.endpoint_name.clone();
            }
            (Some(_), None) => {
                self.reject_generic_show_spout_output_mutation(output_id, "removal")?;
                self.video_outputs
                    .retain(|output| output.summary.id != output_id);
                self.remove_video_output_references(output_id);
            }
            (None, None) => return Err("Video output authoring requires a target image".into()),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn runtime() -> EngineRuntime {
        EngineRuntime::new(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        })
    }

    fn output(id: u64, kind: VideoOutputKind) -> VideoOutputSummary {
        VideoOutputSummary {
            id,
            label: format!("Output {id}"),
            kind,
            enabled: true,
            composition_id: 1,
            fullscreen: false,
            monitor_id: None,
            monitor_identity: None,
            width: 1280,
            height: 720,
            endpoint_name: Some(format!("task33-{id}")),
            opacity: 1.0,
            blackout: false,
            mapping: Default::default(),
        }
    }

    fn publish(
        runtime: &mut EngineRuntime,
        expected: Option<VideoOutputSummary>,
        replacement: Option<VideoOutputSummary>,
        fail_publication: bool,
    ) -> Result<(), String> {
        let id = expected.as_ref().or(replacement.as_ref()).unwrap().id;
        let published = RwLock::new(runtime.build_snapshot(0));
        let (ack, receiver) = mpsc::sync_channel(1);
        runtime.apply_command(EngineCommand::AuthorVideoOutputPublished {
            output_id: id,
            expected,
            replacement,
            expires_at: Instant::now() + Duration::from_secs(1),
            admission: ProjectSnapshotLoadAdmission::new(),
            ack,
        });
        runtime.fail_next_pending_publication = fail_publication;
        runtime.publish_pending_command_acks(0, &published);
        receiver.recv().unwrap()
    }

    fn candidate_with_outputs(outputs: &[VideoOutputSummary]) -> EngineSnapshot {
        let mut snapshot = runtime().build_persistence_snapshot();
        for output in outputs {
            let replacement = Some(output.clone());
            update_video_image(&mut snapshot.video, output.id, &replacement);
            if let Some(video) = &mut snapshot.authored_video {
                update_video_image(video, output.id, &replacement);
            }
        }
        snapshot
    }

    fn managed_pair_candidate(enabled: bool) -> EngineSnapshot {
        let mut background = output(41, VideoOutputKind::SpoutSender);
        background.label = SHOW_SPOUT_BACKGROUND_NAME.into();
        background.endpoint_name = Some(SHOW_SPOUT_BACKGROUND_NAME.into());
        background.width = SHOW_SPOUT_BACKGROUND_WIDTH;
        background.height = SHOW_SPOUT_BACKGROUND_HEIGHT;
        background.composition_id = 2;
        background.enabled = enabled;
        let mut foreground = output(42, VideoOutputKind::SpoutSender);
        foreground.label = SHOW_SPOUT_FOREGROUND_NAME.into();
        foreground.endpoint_name = Some(SHOW_SPOUT_FOREGROUND_NAME.into());
        foreground.width = SHOW_SPOUT_FOREGROUND_WIDTH;
        foreground.height = SHOW_SPOUT_FOREGROUND_HEIGHT;
        foreground.composition_id = 3;
        foreground.enabled = enabled;
        let mut snapshot = candidate_with_outputs(&[background, foreground]);
        for (id, label, output_id) in [
            (2, SHOW_SPOUT_BACKGROUND_COMPOSITION_LABEL, 41),
            (3, SHOW_SPOUT_FOREGROUND_COMPOSITION_LABEL, 42),
        ] {
            let composition = CompositionSummary {
                id,
                label: label.into(),
                layer_ids: Vec::new(),
                timeline_layer_ids: Vec::new(),
                output_ids: vec![output_id],
            };
            snapshot.video.compositions.push(composition.clone());
            if let Some(video) = &mut snapshot.authored_video {
                video.compositions.push(composition);
            }
        }
        snapshot
    }

    #[test]
    fn authoring_candidate_rejects_reserved_show_names_before_add_configure_or_remove() {
        let empty = candidate_with_outputs(&[]);
        for name in [SHOW_SPOUT_BACKGROUND_NAME, SHOW_SPOUT_FOREGROUND_NAME] {
            for kind in [
                VideoOutputKind::SpoutSender,
                VideoOutputKind::NdiSender,
                VideoOutputKind::SyphonServer,
            ] {
                for reserve_label in [false, true] {
                    let mut reserved = output(51, kind.clone());
                    if reserve_label {
                        reserved.label = name.into();
                    } else {
                        reserved.endpoint_name = Some(name.into());
                    }
                    let error =
                        candidate_snapshot(&empty, reserved.id, &None, &Some(reserved.clone()))
                            .unwrap_err();
                    assert!(error.contains("reserved Show Spout names"), "{error}");
                    for enabled in [false, true] {
                        reserved.enabled = enabled;
                        let before = candidate_with_outputs(&[reserved.clone()]);
                        let mut renamed = reserved.clone();
                        renamed.label = "Ordinary destination".into();
                        renamed.endpoint_name = Some("Ordinary destination".into());
                        let error = candidate_snapshot(
                            &before,
                            reserved.id,
                            &Some(reserved.clone()),
                            &Some(renamed),
                        )
                        .unwrap_err();
                        assert!(error.contains("reserved Show Spout names"), "{error}");
                        let error = candidate_snapshot(
                            &before,
                            reserved.id,
                            &Some(reserved.clone()),
                            &None,
                        )
                        .unwrap_err();
                        assert!(error.contains("reserved Show Spout names"), "{error}");
                        assert_eq!(before.video.outputs, vec![reserved.clone()]);
                    }
                }
            }
        }
        assert!(empty.video.outputs.is_empty());
    }

    #[test]
    fn authoring_candidate_protects_enabled_disabled_and_partial_managed_spout_sets() {
        for enabled in [false, true] {
            let pair = managed_pair_candidate(enabled);
            let exact_background = pair.video.outputs[0].clone();
            let exact_foreground = pair.video.outputs[1].clone();
            if enabled {
                assert!(show_spout_pair_is_exact(
                    &exact_background,
                    &exact_foreground
                ));
            }
            for partial in [false, true] {
                let mut before = pair.clone();
                if partial {
                    update_video_image(&mut before.video, exact_foreground.id, &None);
                    if let Some(video) = &mut before.authored_video {
                        update_video_image(video, exact_foreground.id, &None);
                    }
                }
                let extra = output(53, VideoOutputKind::SpoutSender);
                let error = candidate_snapshot(&before, extra.id, &None, &Some(extra)).unwrap_err();
                assert!(
                    error.contains("reserved Show Spout outputs are authored"),
                    "{error}"
                );
                let ordinary = output(54, VideoOutputKind::NdiSender);
                let with_ordinary =
                    candidate_snapshot(&before, ordinary.id, &None, &Some(ordinary.clone()))
                        .unwrap();
                let mut converted = ordinary.clone();
                converted.kind = VideoOutputKind::SpoutSender;
                let error = candidate_snapshot(
                    &with_ordinary,
                    ordinary.id,
                    &Some(ordinary.clone()),
                    &Some(converted),
                )
                .unwrap_err();
                assert!(
                    error.contains("reserved Show Spout outputs are authored"),
                    "{error}"
                );
                let mut resized = exact_background.clone();
                resized.width += 1;
                assert!(candidate_snapshot(
                    &before,
                    exact_background.id,
                    &Some(exact_background.clone()),
                    &Some(resized)
                )
                .is_err());
                assert!(candidate_snapshot(
                    &before,
                    exact_background.id,
                    &Some(exact_background.clone()),
                    &None
                )
                .is_err());
                assert!(
                    candidate_snapshot(&with_ordinary, ordinary.id, &Some(ordinary), &None).is_ok()
                );
            }
        }
    }

    #[test]
    fn authoring_candidate_allows_ordinary_live_outputs_without_claiming_show_routes() {
        let runtime = runtime();
        runtime
            .output_ownership_gate
            .publish_runtime_role(MachineOutputRole::Both);
        let before = runtime.build_persistence_snapshot();
        let ownership = runtime.output_ownership_gate.status();
        for kind in [
            VideoOutputKind::SpoutSender,
            VideoOutputKind::NdiSender,
            VideoOutputKind::SyphonServer,
        ] {
            let mut ordinary = output(61, kind);
            ordinary.label = "Syndocal Background auxiliary".into();
            ordinary.endpoint_name = Some("Syndocal Foreground auxiliary".into());
            let added =
                candidate_snapshot(&before, ordinary.id, &None, &Some(ordinary.clone())).unwrap();
            let mut configured = ordinary.clone();
            configured.width = 1920;
            let changed = candidate_snapshot(
                &added,
                ordinary.id,
                &Some(ordinary),
                &Some(configured.clone()),
            )
            .unwrap();
            let removed =
                candidate_snapshot(&changed, configured.id, &Some(configured), &None).unwrap();
            assert!(removed.video.outputs.is_empty());
        }
        let pair = managed_pair_candidate(true);
        let ordinary = output(62, VideoOutputKind::SpoutSender);
        let mut with_extra = pair.clone();
        update_video_image(&mut with_extra.video, ordinary.id, &Some(ordinary.clone()));
        if let Some(video) = &mut with_extra.authored_video {
            update_video_image(video, ordinary.id, &Some(ordinary.clone()));
        }
        let repaired =
            candidate_snapshot(&with_extra, ordinary.id, &Some(ordinary), &None).unwrap();
        assert_eq!(repaired.video.outputs, pair.video.outputs);
        assert!(runtime.video_outputs.is_empty());
        assert!(runtime.pending_command_acks.is_empty());
        assert_eq!(runtime.output_ownership_gate.status(), ownership);
    }

    #[test]
    fn authoring_add_configure_remove_follow_live_role_without_changing_authority() {
        for kind in [
            VideoOutputKind::SpoutSender,
            VideoOutputKind::NdiSender,
            VideoOutputKind::SyphonServer,
        ] {
            for role in [
                MachineOutputRole::Standby,
                MachineOutputRole::Video,
                MachineOutputRole::Both,
            ] {
                let mut runtime = runtime();
                runtime.output_ownership_gate.publish_runtime_role(role);
                let ownership = runtime.output_ownership_gate.status();
                let before = output(42, kind.clone());
                publish(&mut runtime, None, Some(before.clone()), false).unwrap();
                assert!(runtime.video_outputs[0].summary.enabled);
                let mut after = before.clone();
                after.label = "Updated output".into();
                after.width = 1920;
                after.height = 1080;
                after.endpoint_name = Some("task33-updated".into());
                publish(&mut runtime, Some(before), Some(after.clone()), false).unwrap();
                assert_eq!(runtime.video_outputs[0].summary, after);
                publish(&mut runtime, Some(after), None, false).unwrap();
                assert!(runtime.video_outputs.is_empty());
                let current = runtime.output_ownership_gate.status();
                assert_eq!(current.effective_role, ownership.effective_role);
                assert_eq!(current.epoch, ownership.epoch);
                assert_eq!(current.generation, ownership.generation);
            }
        }
    }

    #[test]
    fn authoring_candidate_rejects_stale_identity_duplicates_and_nonconfiguration_changes() {
        let mut runtime = runtime();
        let first = output(1, VideoOutputKind::SpoutSender);
        let sibling = output(2, VideoOutputKind::SpoutSender);
        publish(&mut runtime, None, Some(first.clone()), false).unwrap();
        publish(&mut runtime, None, Some(sibling.clone()), false).unwrap();
        let snapshot = runtime.build_persistence_snapshot();
        let mut duplicate = first.clone();
        duplicate.endpoint_name = Some(sibling.endpoint_name.clone().unwrap().to_ascii_uppercase());
        assert!(
            candidate_snapshot(&snapshot, first.id, &Some(first.clone()), &Some(duplicate))
                .is_err()
        );
        let mut stale = first.clone();
        stale.width += 1;
        assert!(publish(&mut runtime, Some(stale), None, false).is_err());
        let mut wrong = first.clone();
        wrong.enabled = false;
        assert!(publish(&mut runtime, Some(first.clone()), Some(wrong), false).is_err());
        let mut disabled = output(3, VideoOutputKind::NdiSender);
        disabled.enabled = false;
        assert!(publish(&mut runtime, None, Some(disabled), false).is_err());
        let mut invalid = first.clone();
        invalid.width = 8193;
        assert!(publish(&mut runtime, Some(first.clone()), Some(invalid), false).is_err());
        assert_eq!(
            runtime.build_persistence_snapshot().video.outputs,
            snapshot.video.outputs
        );
    }

    #[test]
    fn authoring_preserves_cue_playback_and_replacement_ack_rollback() {
        let mut runtime = runtime();
        let before = output(7, VideoOutputKind::SpoutSender);
        publish(&mut runtime, None, Some(before.clone()), false).unwrap();
        runtime.apply_command(EngineCommand::CreateCue {
            authored_beats: None,
            cue_id: 9,
            label: "Live output".into(),
            fade_ms: 0,
            targets: Vec::new(),
            video_targets: Vec::new(),
            video_output_targets: vec![VideoOutputTarget {
                output_id: before.id,
                enabled: true,
                opacity: 0.25,
                blackout: true,
            }],
            node_graph_targets: Vec::new(),
            effect_targets: Vec::new(),
        });
        runtime.apply_command(EngineCommand::TriggerCue(9));
        let live_before = runtime.video_outputs[0].summary.clone();
        let authored = runtime.build_persistence_snapshot().video.outputs[0].clone();
        let mut after = authored.clone();
        after.width = 640;
        after.endpoint_name = Some("changed-live".into());
        publish(
            &mut runtime,
            Some(authored.clone()),
            Some(after.clone()),
            true,
        )
        .unwrap_err();
        assert_eq!(runtime.video_outputs[0].summary, live_before);
        publish(&mut runtime, Some(authored), Some(after.clone()), false).unwrap();
        assert_eq!(
            runtime.video_outputs[0].summary.opacity,
            live_before.opacity
        );
        assert_eq!(
            runtime.video_outputs[0].summary.blackout,
            live_before.blackout
        );
        assert_eq!(runtime.build_persistence_snapshot().video.outputs[0], after);
    }

    #[test]
    fn authoring_remove_rolls_back_references_and_persists_exact_candidate() {
        let mut runtime = runtime();
        let before = output(7, VideoOutputKind::NdiSender);
        publish(&mut runtime, None, Some(before.clone()), false).unwrap();
        runtime.apply_command(EngineCommand::CreateCue {
            authored_beats: None,
            cue_id: 8,
            label: "Output target".into(),
            fade_ms: 1000,
            targets: Vec::new(),
            video_targets: Vec::new(),
            video_output_targets: vec![VideoOutputTarget {
                output_id: before.id,
                enabled: true,
                opacity: 0.5,
                blackout: false,
            }],
            node_graph_targets: Vec::new(),
            effect_targets: Vec::new(),
        });
        let snapshot = runtime.build_persistence_snapshot();
        let candidate =
            candidate_snapshot(&snapshot, before.id, &Some(before.clone()), &None).unwrap();
        assert!(candidate.video.outputs.is_empty());
        assert!(candidate.cues[0].video_output_targets.is_empty());
        publish(&mut runtime, Some(before.clone()), None, true).unwrap_err();
        assert_eq!(
            runtime.build_persistence_snapshot().video.outputs,
            snapshot.video.outputs
        );
        assert_eq!(runtime.build_persistence_snapshot().cues, snapshot.cues);
        publish(&mut runtime, Some(before), None, false).unwrap();
        let actual = runtime.build_persistence_snapshot();
        assert_eq!(actual.video.outputs, candidate.video.outputs);
        assert_eq!(actual.cues, candidate.cues);
        let reopened: EngineSnapshot =
            serde_json::from_slice(&serde_json::to_vec(&actual).unwrap()).unwrap();
        assert!(reopened.video.outputs.is_empty());
        assert!(reopened.cues[0].video_output_targets.is_empty());
    }

    #[test]
    fn authoring_publication_cancelled_before_admission_has_no_effect() {
        let mut runtime = runtime();
        let before = runtime.build_persistence_snapshot();
        let admission = ProjectSnapshotLoadAdmission::new();
        assert!(admission.cancel_if_queued());
        let (ack, receiver) = mpsc::sync_channel(1);
        runtime.apply_command(EngineCommand::AuthorVideoOutputPublished {
            output_id: 11,
            expected: None,
            replacement: Some(output(11, VideoOutputKind::SpoutSender)),
            expires_at: Instant::now() + Duration::from_secs(1),
            admission,
            ack,
        });
        runtime.publish_pending_command_acks(0, &RwLock::new(runtime.build_snapshot(0)));
        assert!(receiver.recv().unwrap().is_err());
        assert_eq!(
            runtime.build_persistence_snapshot().video.outputs,
            before.video.outputs
        );
    }

    #[test]
    fn authoring_held_submission_publishes_without_reacquiring_its_gate() {
        let engine = EngineHandle::start_for_tests(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let created = output(
            engine.allocate_video_output_id(),
            VideoOutputKind::SpoutSender,
        );
        let submission = engine.begin_persistence_mutation_submission().unwrap();
        submission
            .author_video_output_published(created.id, None, Some(created.clone()))
            .unwrap();
        assert_eq!(
            submission.persistence_snapshot().unwrap().video.outputs,
            vec![created.clone()]
        );
        submission
            .author_video_output_published(created.id, Some(created), None)
            .unwrap();
        assert!(submission
            .persistence_snapshot()
            .unwrap()
            .video
            .outputs
            .is_empty());
    }
}
