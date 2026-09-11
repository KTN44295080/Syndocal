use crate::{EngineHandle, TimelineTransportAuthority};
use protocol::{
    AutoVjAction, AutomationId, ClockSnapshot, CompositionId, CompositionSummary, CueId, CueListId,
    CueSummary, DmxOutputConfig, DmxUniversePreview, EffectId, EffectKind, EffectSummary,
    EngineSnapshot, EngineTelemetry, FixtureId, NodeGraphId, NodeGraphSummary, PaletteId,
    PatchedFixtureSummary, PlaybackExecutorSummary, StageObjectSummary, TimelineAudioClipId,
    TimelineAudioOutputBus,
    TimelineCueEventSummary, TimelineEventId, TimelineFollowRuntimeStatus,
    TimelineFollowRuntimeStatusSnapshot, TimelineFollowRuntimeSummary, TimelineId,
    TimelineLayerSummary, TimelineLoopRuntimeStatus, VideoClipRuntimeSnapshot,
    VideoIsfEffectSummary, VideoLayerId, VideoLayerState, VideoLayerTransitionBusSummary,
    VideoLayerTransitionRuntimeSnapshot, VideoOutputId, VideoOutputSummary, VideoSnapshot,
    VideoSourceSummary,
};

/// The runtime-only projection required by the control-plane query adapter.
/// Keeping this projection under one snapshot read preserves the existing
/// publication consistency while avoiding a deep copy of authored/project
/// collections on every query capture.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlPlaneRuntimeSnapshot {
    pub timeline_transport_epoch: u64,
    pub timeline_transport_generation: u64,
    pub timeline_transport_active: bool,
    pub timeline_follow_generation: u64,
    pub timeline_follow_active: bool,
    pub timeline_loop_generation: u64,
    pub timeline_loop_active: bool,
    pub video_clip_slots_active: bool,
    pub video_transitions_active: bool,
}

/// The authored counts, DMX routes, clock, and telemetry needed by the
/// diagnostics report without cloning the rest of the public snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct EngineTelemetrySnapshot {
    pub fixture_count: usize,
    pub cue_count: usize,
    pub effect_count: usize,
    pub node_graph_count: usize,
    pub timeline_event_count: usize,
    pub timeline_automation_count: usize,
    pub timeline_video_automation_count: usize,
    pub video_layer_count: usize,
    pub video_output_count: usize,
    pub dmx_output_count: usize,
    pub enabled_dmx_output_count: usize,
    pub dmx_preview_universe_count: usize,
    pub clock: ClockSnapshot,
    pub primary_output: DmxOutputConfig,
    pub dmx_outputs: Vec<DmxOutputConfig>,
    pub telemetry: EngineTelemetry,
}

fn video_isf_effect_source_bytes(effect: &VideoIsfEffectSummary) -> Result<usize, String> {
    effect
        .stack
        .iter()
        .try_fold(effect.source.len(), |total, stage| {
            total
                .checked_add(stage.source.len())
                .ok_or_else(|| "Project ISF source size overflowed".to_string())
        })
}

/// The authored Cue fields required by metadata admission. Keeping the
/// current Cue body together with same-bank numbering preserves one
/// publication generation without cloning unrelated project collections.
#[derive(Debug, Clone, PartialEq)]
pub struct CueMetadataAdmissionSnapshot {
    pub cue: Option<CueSummary>,
    pub same_bank_numbers: Vec<(CueId, String)>,
}

/// The published fields required to build a read-only Visualizer scene.
/// Keeping this projection separate avoids cloning authored and runtime
/// collections that the Visualizer never reads.
#[derive(Debug, Clone, PartialEq)]
pub struct VisualizerSnapshot {
    pub fixtures: Vec<PatchedFixtureSummary>,
    pub dmx_previews: Vec<DmxUniversePreview>,
    pub primary_dmx_universe: u16,
    pub primary_dmx_values: Vec<u8>,
    pub video_outputs: Vec<VideoOutputSummary>,
    pub stage_objects: Vec<StageObjectSummary>,
}

impl EngineTelemetrySnapshot {
    pub fn from_snapshot(snapshot: &EngineSnapshot) -> Self {
        let enabled_dmx_output_count = snapshot
            .dmx_outputs
            .iter()
            .filter(|output| output.enabled)
            .count();
        Self {
            fixture_count: snapshot.fixtures.len(),
            cue_count: snapshot.cues.len(),
            effect_count: snapshot.effects.len(),
            node_graph_count: snapshot.node_graphs.len(),
            timeline_event_count: snapshot.timeline.events.len(),
            timeline_automation_count: snapshot.timeline.automations.len(),
            timeline_video_automation_count: snapshot.timeline.video_automations.len(),
            video_layer_count: snapshot.video.layers.len(),
            video_output_count: snapshot.video.outputs.len(),
            dmx_output_count: snapshot.dmx_outputs.len(),
            enabled_dmx_output_count,
            dmx_preview_universe_count: snapshot.dmx_previews.len(),
            clock: snapshot.clock.clone(),
            primary_output: snapshot.output.clone(),
            dmx_outputs: snapshot.dmx_outputs.clone(),
            telemetry: snapshot.telemetry.clone(),
        }
    }
}

impl EngineHandle {
    // Keep the blocking/public snapshot reader's poison behavior and derive
    // transport tokens from the same Timeline defaults as that public reader.
    fn read_snapshot_field<T>(&self, read: impl FnOnce(&EngineSnapshot) -> T) -> T {
        match self.snapshot.read() {
            Ok(snapshot) => read(&snapshot),
            Err(_) => read(&EngineSnapshot::default()),
        }
    }

    /// The exact engine-owned fence used by the local runtime control plane.
    /// It is non-persistent and becomes visible atomically with the snapshot.
    pub fn timeline_transport_generation(&self) -> u64 {
        self.read_snapshot_field(|snapshot| snapshot.timeline.transport_generation)
    }

    /// Capture the epoch and generation from one shared publication guard.
    pub fn timeline_transport_authority(&self) -> TimelineTransportAuthority {
        self.read_snapshot_field(|snapshot| TimelineTransportAuthority {
            epoch: snapshot.timeline.transport_epoch,
            generation: snapshot.timeline.transport_generation,
        })
    }

    /// Return public Follow status stamped with the caller's authority epoch.
    pub fn timeline_follow_runtime_status(
        &self,
        epoch: u64,
    ) -> TimelineFollowRuntimeStatusSnapshot {
        self.read_snapshot_field(|snapshot| {
            TimelineFollowRuntimeStatusSnapshot::from_runtime(
                epoch,
                &snapshot.timeline.follow_runtime,
            )
        })
    }

    /// Read runtime-only Follow state without cloning unrelated authored data.
    pub fn timeline_follow_runtime_summary(&self) -> TimelineFollowRuntimeSummary {
        self.read_snapshot_field(|snapshot| snapshot.timeline.follow_runtime.clone())
    }

    /// Read the runtime-only fields needed by the control-plane query source
    /// capture without cloning the public project snapshot.
    pub fn control_plane_runtime_snapshot(&self) -> ControlPlaneRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| ControlPlaneRuntimeSnapshot {
            timeline_transport_epoch: snapshot.timeline.transport_epoch,
            timeline_transport_generation: snapshot.timeline.transport_generation,
            timeline_transport_active: snapshot.timeline.playing,
            timeline_follow_generation: snapshot.timeline.follow_runtime.generation,
            timeline_follow_active: snapshot.timeline.follow_runtime.status
                != TimelineFollowRuntimeStatus::Idle,
            timeline_loop_generation: snapshot.timeline.loop_runtime.generation,
            timeline_loop_active: snapshot.timeline.loop_runtime.status
                != TimelineLoopRuntimeStatus::Disabled,
            video_clip_slots_active: snapshot.video_clip_runtime.layers.iter().any(|layer| {
                layer.playing
                    || layer.active_slot_id.is_some()
                    || layer.pending_launch.is_some()
                    || layer.transition.is_some()
            }),
            video_transitions_active: !snapshot.video_transition_runtime.buses.is_empty(),
        })
    }

    /// Read the diagnostics report inputs without cloning unrelated project
    /// collections or runtime surfaces.
    pub fn engine_telemetry_snapshot(&self) -> EngineTelemetrySnapshot {
        self.read_snapshot_field(EngineTelemetrySnapshot::from_snapshot)
    }

    /// Read only the published fields required to build MIDI feedback.
    /// Keeping the existing EngineSnapshot seam here avoids cloning the
    /// unrelated project, output, and diagnostic collections on each send.
    pub fn midi_feedback_snapshot(&self) -> EngineSnapshot {
        self.read_snapshot_field(|snapshot| {
            let mut feedback = EngineSnapshot::default();
            feedback.fixtures = snapshot.fixtures.clone();
            feedback.cue_lists = snapshot.cue_lists.clone();
            feedback.active_cue_id = snapshot.active_cue_id;
            feedback.active_group_cue_ids = snapshot.active_group_cue_ids.clone();
            feedback.effects = snapshot.effects.clone();
            feedback.node_graphs = snapshot.node_graphs.clone();
            feedback.video.layers = snapshot.video.layers.clone();
            feedback.video.outputs = snapshot.video.outputs.clone();
            feedback.video.master_opacity = snapshot.video.master_opacity;
            feedback.video.blackout = snapshot.video.blackout;
            feedback.timeline.playing = snapshot.timeline.playing;
            feedback.timeline.loop_runtime = snapshot.timeline.loop_runtime.clone();
            feedback.timeline.position_ms = snapshot.timeline.position_ms;
            feedback.timeline.duration_ms = snapshot.timeline.duration_ms;
            feedback.clock.bpm = snapshot.clock.bpm;
            feedback.lighting_master = snapshot.lighting_master;
            feedback.submasters = snapshot.submasters.clone();
            feedback.active_fade = snapshot.active_fade.clone();
            feedback.blackout = snapshot.blackout;
            feedback
        })
    }

    /// Read only the published fields required by Visualizer queries.
    pub fn visualizer_snapshot(&self) -> VisualizerSnapshot {
        self.read_snapshot_field(|snapshot| VisualizerSnapshot {
            fixtures: snapshot.fixtures.clone(),
            dmx_previews: snapshot.dmx_previews.clone(),
            primary_dmx_universe: snapshot.output.universe,
            primary_dmx_values: snapshot.dmx_preview.clone(),
            video_outputs: snapshot.video.outputs.clone(),
            stage_objects: snapshot.stage_objects.clone(),
        })
    }

    /// Read Clip Slot transport from the latest complete engine publication.
    pub fn video_clip_runtime_snapshot(&self) -> VideoClipRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| snapshot.video_clip_runtime.clone())
    }

    /// Read the video image and BPM needed by the native layer-thumbnail
    /// renderer from one published snapshot without cloning unrelated state.
    pub fn video_layer_thumbnail_snapshot(&self) -> (VideoSnapshot, f32) {
        self.read_snapshot_field(|snapshot| (snapshot.video.clone(), snapshot.clock.bpm))
    }

    /// Read the video image, clip/transition runtime, and BPM needed by the
    /// debug output preview without cloning unrelated project collections.
    pub fn video_output_preview_snapshot(
        &self,
    ) -> (
        VideoSnapshot,
        VideoClipRuntimeSnapshot,
        VideoLayerTransitionRuntimeSnapshot,
        f32,
    ) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.video.clone(),
                snapshot.video_clip_runtime.clone(),
                snapshot.video_transition_runtime.clone(),
                snapshot.clock.bpm,
            )
        })
    }

    /// Read the published video projection without cloning unrelated engine
    /// collections for read-only plan queries.
    pub fn video_snapshot(&self) -> VideoSnapshot {
        self.read_snapshot_field(|snapshot| snapshot.video.clone())
    }

    /// Read the last Auto VJ action needed by the manual Program handoff
    /// without cloning the complete public video/project snapshot.
    pub fn auto_vj_last_action(&self) -> Option<AutoVjAction> {
        self.read_snapshot_field(|snapshot| snapshot.video.auto_vj.status.last_action.clone())
    }

    /// Read Layer Transition Bus state from the latest complete publication.
    pub fn video_layer_transition_runtime_snapshot(&self) -> VideoLayerTransitionRuntimeSnapshot {
        self.read_snapshot_field(|snapshot| snapshot.video_transition_runtime.clone())
    }

    /// Read public/rendered video output summaries without cloning unrelated
    /// project and runtime collections.
    pub fn video_outputs_snapshot(&self) -> Vec<VideoOutputSummary> {
        self.read_snapshot_field(|snapshot| snapshot.video.outputs.clone())
    }

    /// Read authored video transition-bus definitions without cloning the
    /// unrelated public project and runtime collections.
    pub fn video_transition_buses_snapshot(&self) -> Vec<VideoLayerTransitionBusSummary> {
        self.read_snapshot_field(|snapshot| snapshot.video.transition_buses.clone())
    }

    /// Read the authored video collections needed to validate an output
    /// composition assignment without cloning unrelated public state.
    pub fn video_outputs_and_compositions_snapshot(
        &self,
    ) -> (Vec<VideoOutputSummary>, Vec<CompositionSummary>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.video.outputs.clone(),
                snapshot.video.compositions.clone(),
            )
        })
    }

    /// Read the two authored DMX routes required by the managed serial-DMX
    /// admission check without cloning unrelated project and runtime state.
    pub fn dmx_outputs_and_output_snapshot(&self) -> (Vec<DmxOutputConfig>, DmxOutputConfig) {
        self.read_snapshot_field(|snapshot| (snapshot.dmx_outputs.clone(), snapshot.output.clone()))
    }

    /// Read only the authored Timeline fields needed by enqueue-time legacy
    /// audio allocator reservation. The public projection is intentionally
    /// preserved so implicit/derived layer semantics stay identical.
    pub fn timeline_audio_allocator_snapshot(&self) -> (bool, bool, Vec<TimelineLayerSummary>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.timeline.audio.is_some(),
                snapshot.timeline.audio_clips.is_empty(),
                snapshot.timeline.layers.clone(),
            )
        })
    }

    /// Read authored timeline layers for layer-order allocation without
    /// cloning unrelated project and runtime collections.
    pub fn timeline_layers_snapshot(&self) -> Vec<TimelineLayerSummary> {
        self.read_snapshot_field(|snapshot| snapshot.timeline.layers.clone())
    }

    /// Read published cue IDs for timeline-event admission without cloning
    /// unrelated authored and runtime collections.
    pub fn timeline_cue_ids_snapshot(&self) -> Vec<CueId> {
        self.read_snapshot_field(|snapshot| snapshot.cues.iter().map(|cue| cue.id).collect())
    }

    /// Read authored Timeline IDs for control-mapping admission without
    /// cloning unrelated project and runtime collections.
    pub fn timeline_ids_snapshot(&self) -> Vec<TimelineId> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .timeline_bank
                .iter()
                .map(|timeline| timeline.id)
                .chain(std::iter::once(snapshot.timeline.id))
                .collect()
        })
    }

    /// Read Cue and Effect IDs required by cue-effect admission without
    /// cloning unrelated authored and runtime collections.
    pub fn cue_effect_target_admission_snapshot(&self) -> (Vec<CueId>, Vec<EffectId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.cues.iter().map(|cue| cue.id).collect(),
                snapshot.effects.iter().map(|effect| effect.id).collect(),
            )
        })
    }

    /// Read one Cue body and the numbering of its bank for metadata admission
    /// without cloning unrelated authored and runtime collections.
    pub fn cue_metadata_admission_snapshot(&self, cue_id: CueId) -> CueMetadataAdmissionSnapshot {
        self.read_snapshot_field(|snapshot| {
            let cue = snapshot.cues.iter().find(|cue| cue.id == cue_id).cloned();
            let same_bank_numbers = cue
                .as_ref()
                .map(|current| {
                    snapshot
                        .cues
                        .iter()
                        .filter(|candidate| candidate.cue_list_id == current.cue_list_id)
                        .map(|candidate| (candidate.id, candidate.cue_number.clone()))
                        .collect()
                })
                .unwrap_or_default();
            CueMetadataAdmissionSnapshot {
                cue,
                same_bank_numbers,
            }
        })
    }

    /// Read published cue and timeline-event IDs from one generation for
    /// timeline-event replacement admission.
    pub fn timeline_cue_event_ids_snapshot(&self) -> (Vec<CueId>, Vec<TimelineEventId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.cues.iter().map(|cue| cue.id).collect(),
                snapshot
                    .timeline
                    .events
                    .iter()
                    .map(|event| event.id)
                    .collect(),
            )
        })
    }

    /// Read the cue timing references and timeline-event IDs required by
    /// scene-block admission from one published generation.
    pub fn timeline_scene_block_admission_snapshot(
        &self,
    ) -> (Vec<(CueId, Option<f32>)>, Vec<TimelineCueEventSummary>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot
                    .cues
                    .iter()
                    .map(|cue| (cue.id, cue.authored_beats))
                    .collect(),
                snapshot.timeline.events.clone(),
            )
        })
    }

    /// Read authored DMX and video automation IDs for enable/disable
    /// admission without cloning unrelated timeline state.
    pub fn timeline_automation_ids_snapshot(&self) -> Vec<AutomationId> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .timeline
                .automations
                .iter()
                .map(|automation| automation.id)
                .chain(
                    snapshot
                        .timeline
                        .video_automations
                        .iter()
                        .map(|automation| automation.id),
                )
                .collect()
        })
    }

    /// Read fixtures and authored DMX automation IDs for automation
    /// replacement admission from one published generation.
    pub fn timeline_dmx_automation_admission_snapshot(
        &self,
    ) -> (Vec<PatchedFixtureSummary>, Vec<AutomationId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.fixtures.clone(),
                snapshot
                    .timeline
                    .automations
                    .iter()
                    .map(|automation| automation.id)
                    .collect(),
            )
        })
    }

    /// Read video-layer IDs and authored video automation IDs for video
    /// automation replacement admission from one published generation.
    pub fn timeline_video_automation_admission_snapshot(
        &self,
    ) -> (Vec<VideoLayerId>, Vec<AutomationId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.video.layers.iter().map(|layer| layer.id).collect(),
                snapshot
                    .timeline
                    .video_automations
                    .iter()
                    .map(|automation| automation.id)
                    .collect(),
            )
        })
    }

    /// Read timeline layers and one audio clip's logical output bus for clip
    /// replacement admission from one published generation.
    pub fn timeline_audio_clip_admission_snapshot(
        &self,
        clip_id: TimelineAudioClipId,
    ) -> (Vec<TimelineLayerSummary>, Option<TimelineAudioOutputBus>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.timeline.layers.clone(),
                snapshot
                    .timeline
                    .audio_clips
                    .iter()
                    .find(|clip| clip.id == clip_id)
                    .map(|clip| clip.output_bus),
            )
        })
    }

    /// Read the authored IDs required by cue palette-target admission from
    /// one published generation.
    pub fn cue_palette_target_admission_snapshot(
        &self,
    ) -> (Vec<CueId>, Vec<PaletteId>, Vec<FixtureId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.cues.iter().map(|cue| cue.id).collect(),
                snapshot.palettes.iter().map(|palette| palette.id).collect(),
                snapshot.fixtures.iter().map(|fixture| fixture.id).collect(),
            )
        })
    }

    /// Read the authored IDs required by reference-palette admission from
    /// one published generation.
    pub fn reference_palette_admission_snapshot(&self) -> (Vec<PaletteId>, Vec<FixtureId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.palettes.iter().map(|palette| palette.id).collect(),
                snapshot.fixtures.iter().map(|fixture| fixture.id).collect(),
            )
        })
    }

    /// Read the authored collections required by Playback Executor admission
    /// from one published generation.
    pub fn playback_executor_admission_snapshot(
        &self,
    ) -> (Vec<CueListId>, Vec<PlaybackExecutorSummary>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot
                    .cue_lists
                    .iter()
                    .map(|cue_list| cue_list.id)
                    .collect(),
                snapshot.playback_executors.clone(),
            )
        })
    }

    /// Read one authored effect kind for update admission without cloning the
    /// complete effect or runtime snapshot.
    pub fn effect_kind_snapshot(&self, effect_id: EffectId) -> Option<EffectKind> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .map(|effect| effect.effect_type)
        })
    }

    /// Read one authored Effect body for preset export without cloning
    /// unrelated effects, authored collections, or runtime state.
    pub fn effect_summary_snapshot(&self, effect_id: EffectId) -> Option<EffectSummary> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)
                .cloned()
        })
    }

    /// Read the effect target and published layer IDs needed by one video
    /// effect-target admission without cloning unrelated effect bodies or
    /// runtime collections.
    pub fn effect_video_target_admission_snapshot(
        &self,
        effect_id: EffectId,
    ) -> Option<(Vec<VideoLayerId>, Vec<VideoLayerId>)> {
        self.read_snapshot_field(|snapshot| {
            let effect = snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)?;
            let target_layer_ids = effect
                .video_targets
                .iter()
                .flat_map(|target| target.layer_ids.iter().copied())
                .collect();
            let published_layer_ids = snapshot.video.layers.iter().map(|layer| layer.id).collect();
            Some((target_layer_ids, published_layer_ids))
        })
    }

    /// Read the fixture and video-layer references required by effect-preset
    /// admission without cloning effect bodies or runtime collections.
    pub fn effect_target_admission_snapshot(
        &self,
    ) -> (Vec<PatchedFixtureSummary>, Vec<VideoLayerId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.fixtures.clone(),
                snapshot.video.layers.iter().map(|layer| layer.id).collect(),
            )
        })
    }

    /// Check one authored node-graph ID for enable/disable admission without
    /// cloning the complete graph bodies or runtime snapshot.
    pub fn node_graph_exists(&self, graph_id: NodeGraphId) -> bool {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .node_graphs
                .iter()
                .any(|graph| graph.id == graph_id)
        })
    }

    /// Read the authored references required by node-graph admission without
    /// cloning unrelated node-graph, authored, or runtime collections.
    pub fn node_graph_admission_snapshot(&self) -> (Vec<PatchedFixtureSummary>, Vec<VideoLayerId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.fixtures.clone(),
                snapshot.video.layers.iter().map(|layer| layer.id).collect(),
            )
        })
    }

    /// Read one authored node-graph body for preset export without cloning
    /// unrelated graph bodies, authored collections, or runtime state.
    pub fn node_graph_summary_snapshot(&self, graph_id: NodeGraphId) -> Option<NodeGraphSummary> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .node_graphs
                .iter()
                .find(|graph| graph.id == graph_id)
                .cloned()
        })
    }

    /// Read only the stage objects attached to one published preset for
    /// enqueue-time allocator reservation. Trimming and first-match policy
    /// remain owned by the caller, matching the existing command path.
    pub fn stage_map_preset_allocator_objects(
        &self,
        label: &str,
    ) -> Option<Vec<StageObjectSummary>> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .stage_map_presets
                .iter()
                .find(|preset| preset.label == label)
                .and_then(|preset| preset.stage_objects.clone())
        })
    }

    /// Read authored stage objects for standalone Stage Map export without
    /// cloning unrelated public and runtime collections.
    pub fn stage_objects_snapshot(&self) -> Vec<StageObjectSummary> {
        self.read_snapshot_field(|snapshot| snapshot.stage_objects.clone())
    }

    /// Read authored fixture summaries for profile-health inspection without
    /// cloning unrelated published and runtime collections.
    pub fn fixtures_snapshot(&self) -> Vec<PatchedFixtureSummary> {
        self.read_snapshot_field(|snapshot| snapshot.fixtures.clone())
    }

    /// Read one authored Fixture body for preset export without cloning the
    /// complete fixture catalog or unrelated published/runtime collections.
    pub fn fixture_summary_snapshot(&self, fixture_id: FixtureId) -> Option<PatchedFixtureSummary> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)
                .cloned()
        })
    }

    /// Read the authored references needed to validate one Touch Surface
    /// update without cloning unrelated published and runtime collections.
    pub fn touch_surface_admission_snapshot(&self) -> (Vec<PatchedFixtureSummary>, Vec<CueId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot.fixtures.clone(),
                snapshot.cues.iter().map(|cue| cue.id).collect(),
            )
        })
    }

    /// Read only fixture group memberships for group-scoped validation.
    pub fn fixture_group_ids_snapshot(&self) -> Vec<Vec<String>> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .fixtures
                .iter()
                .map(|fixture| fixture.group_ids.clone())
                .collect()
        })
    }

    /// Check one published video-output ID without cloning the public video
    /// image or unrelated project/runtime collections.
    pub fn video_output_exists(&self, output_id: VideoOutputId) -> bool {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .video
                .outputs
                .iter()
                .any(|output| output.id == output_id)
        })
    }

    /// Check one editable video-composition ID without cloning the full video
    /// image or unrelated authored and runtime collections.
    pub fn video_composition_exists(&self, composition_id: CompositionId) -> bool {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .video
                .compositions
                .iter()
                .any(|composition| composition.id == composition_id)
        })
    }

    /// Read the editable composition presence and published layer IDs under
    /// one snapshot guard for composition-layer admission.
    pub fn video_composition_layer_admission_snapshot(
        &self,
        composition_id: CompositionId,
    ) -> (bool, Vec<VideoLayerId>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot
                    .video
                    .compositions
                    .iter()
                    .any(|composition| composition.id == composition_id),
                snapshot.video.layers.iter().map(|layer| layer.id).collect(),
            )
        })
    }

    /// Read editable composition presence and all authored timeline layers
    /// needed by timeline-video admission under one snapshot guard.
    pub fn video_composition_timeline_layer_admission_snapshot(
        &self,
        composition_id: CompositionId,
    ) -> (bool, Vec<(TimelineId, Vec<TimelineLayerSummary>)>) {
        self.read_snapshot_field(|snapshot| {
            (
                snapshot
                    .video
                    .compositions
                    .iter()
                    .any(|composition| composition.id == composition_id),
                snapshot
                    .timeline_bank
                    .iter()
                    .chain(std::iter::once(&snapshot.timeline))
                    .map(|timeline| (timeline.id, timeline.layers.clone()))
                    .collect(),
            )
        })
    }

    /// Read only published video-layer IDs for admission checks that do not
    /// need the authored layer bodies or unrelated project/runtime state.
    pub fn video_layer_ids_snapshot(&self) -> Vec<VideoLayerId> {
        self.read_snapshot_field(|snapshot| {
            snapshot.video.layers.iter().map(|layer| layer.id).collect()
        })
    }

    /// Read one layer's ISF addition and the current project ISF total under
    /// one snapshot guard for duplicate-layer admission.
    pub fn video_layer_duplicate_admission_snapshot(
        &self,
        layer_id: VideoLayerId,
    ) -> Option<Result<(usize, usize), String>> {
        self.read_snapshot_field(|snapshot| {
            let source_layer = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?;
            let added_source_bytes = match source_layer
                .isf_effect
                .as_ref()
                .map(video_isf_effect_source_bytes)
                .transpose()
            {
                Ok(bytes) => bytes.unwrap_or(0),
                Err(error) => return Some(Err(error)),
            };
            let current_source_bytes = match snapshot
                .video
                .layers
                .iter()
                .filter_map(|layer| layer.isf_effect.as_ref())
                .try_fold(0_usize, |total, effect| {
                    video_isf_effect_source_bytes(effect)?
                        .checked_add(total)
                        .ok_or_else(|| "Project ISF source size overflowed".to_string())
                }) {
                Ok(bytes) => bytes,
                Err(error) => return Some(Err(error)),
            };
            Some(Ok((added_source_bytes, current_source_bytes)))
        })
    }

    /// Read one published layer state for a launch admission without cloning
    /// the public video/project snapshot or unrelated runtime collections.
    pub fn video_layer_state_snapshot(&self, layer_id: VideoLayerId) -> Option<VideoLayerState> {
        self.video_layer_states_snapshot(&[layer_id])
            .into_iter()
            .next()
            .map(|(_, state)| state)
    }

    /// Read several published layer states under one guard for paired
    /// controls that must not mix two public snapshot generations.
    pub fn video_layer_states_snapshot(
        &self,
        layer_ids: &[VideoLayerId],
    ) -> Vec<(VideoLayerId, VideoLayerState)> {
        self.read_snapshot_field(|snapshot| {
            layer_ids
                .iter()
                .filter_map(|layer_id| {
                    snapshot
                        .video
                        .layers
                        .iter()
                        .find(|layer| layer.id == *layer_id)
                        .map(|layer| (*layer_id, layer.state.clone()))
                })
                .collect()
        })
    }

    /// Read the layer state/source and Auto VJ action needed by the direct
    /// audio monitor path from one published snapshot generation.
    pub fn video_layer_audio_monitor_snapshot(
        &self,
        layer_id: VideoLayerId,
    ) -> Option<(VideoLayerState, VideoSourceSummary, Option<AutoVjAction>)> {
        self.read_snapshot_field(|snapshot| {
            snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .map(|layer| {
                    (
                        layer.state.clone(),
                        layer.source.clone(),
                        snapshot.video.auto_vj.status.last_action.clone(),
                    )
                })
        })
    }

    /// Read the published Timeline playing flag without cloning unrelated
    /// public project and runtime collections.
    pub fn timeline_playing(&self) -> bool {
        self.read_snapshot_field(|snapshot| snapshot.timeline.playing)
    }
}
