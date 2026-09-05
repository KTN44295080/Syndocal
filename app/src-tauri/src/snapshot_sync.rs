//! Bounded snapshot synchronization service; no Tauri or engine ownership dependencies.
//! Revisions identify captured snapshots globally. Keep a small recent history so
//! independent windows can diff against their own last response without forcing full payloads.
use protocol::{
    ClockSnapshot, CueId, DmxOutputConfig, EffectSummary, EngineSnapshot, EngineTelemetry,
    NodeGraphSummary, PatchedFixtureSummary, StageMapConfig, StageMapPresetSummary,
    StageObjectSummary,
};
use serde::Serialize;
use std::{
    collections::{BTreeMap, VecDeque},
    sync::Arc,
};

const SNAPSHOT_HISTORY_CAPACITY: usize = 4;

#[derive(Debug, Default)]
pub(crate) struct SnapshotSyncState {
    revision: u64,
    history: VecDeque<(u64, Arc<EngineSnapshot>)>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SnapshotSyncPayload {
    pub(crate) revision: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) full: Option<Arc<EngineSnapshot>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) delta: Option<EngineSnapshotDelta>,
}

impl SnapshotSyncState {
    /// The caller serializes capture and publication together. A revision must
    /// never describe a sample captured before a previously published sample.
    pub(crate) fn publish(
        &mut self,
        client_revision: Option<u64>,
        current: EngineSnapshot,
    ) -> Result<SnapshotSyncPayload, String> {
        let revision = self.revision.checked_add(1).ok_or_else(||
            "Snapshot synchronization revision exhausted; restart Syndocal to establish a new session".to_string())?;
        // A full response and its retained base share one immutable captured image.
        // Eviction can release the history's ownership before response serialization.
        let current = Arc::new(current);
        let before = client_revision.and_then(|requested| {
            self.history
                .iter()
                .find(|(revision, _)| *revision == requested)
                .map(|(_, snapshot)| snapshot)
        });
        let payload = match before {
            Some(before) => SnapshotSyncPayload {
                revision,
                full: None,
                delta: Some(engine_snapshot_delta(before, &current)),
            },
            None => SnapshotSyncPayload {
                revision,
                full: Some(Arc::clone(&current)),
                delta: None,
            },
        };
        // Retain at most four complete snapshots, regardless of client count.
        if self.history.len() == SNAPSHOT_HISTORY_CAPACITY {
            self.history.pop_front();
        }
        self.history.push_back((revision, current));
        self.revision = revision;
        Ok(payload)
    }
}

#[derive(Debug, Clone, Default, Serialize, PartialEq)]
pub(crate) struct EngineSnapshotDelta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) fixtures: Option<Vec<PatchedFixtureSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cues: Option<Vec<protocol::CueSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cue_lists: Option<Vec<protocol::CueListSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) palettes: Option<Vec<protocol::ReferencePaletteSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) playback_executors: Option<Vec<protocol::PlaybackExecutorSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) playback_master: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) active_cue_id: Option<Option<CueId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) direct_child_timeline_transports:
        Option<Vec<protocol::DirectChildTimelineTransportSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) active_group_cue_ids: Option<BTreeMap<String, CueId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) cue_live_modifiers: Option<Vec<protocol::CueLiveModifierState>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) group_colors: Option<BTreeMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) active_fade: Option<Option<protocol::ActiveFadeSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) programmer: Option<protocol::ProgrammerSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) timeline: Option<protocol::TimelineSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) video: Option<protocol::VideoSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) effects: Option<Vec<EffectSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) node_graphs: Option<Vec<NodeGraphSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output: Option<DmxOutputConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dmx_outputs: Option<Vec<DmxOutputConfig>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lighting_master: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) submasters: Option<Vec<protocol::SubmasterSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) blackout: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) clock: Option<ClockSnapshot>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stage_map: Option<StageMapConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stage_map_presets: Option<Vec<StageMapPresetSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) stage_objects: Option<Vec<StageObjectSummary>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dmx_preview: Option<Vec<u8>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) dmx_previews: Option<Vec<protocol::DmxUniversePreview>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) telemetry: Option<EngineTelemetry>,
}

pub(crate) fn engine_snapshot_delta(
    before: &EngineSnapshot,
    after: &EngineSnapshot,
) -> EngineSnapshotDelta {
    macro_rules! changed {
        ($field:ident) => {
            (before.$field != after.$field).then(|| after.$field.clone())
        };
    }
    EngineSnapshotDelta {
        fixtures: changed!(fixtures),
        cues: changed!(cues),
        cue_lists: changed!(cue_lists),
        palettes: changed!(palettes),
        playback_executors: changed!(playback_executors),
        playback_master: (before.playback_master != after.playback_master)
            .then_some(after.playback_master),
        active_cue_id: (before.active_cue_id != after.active_cue_id).then_some(after.active_cue_id),
        direct_child_timeline_transports: changed!(direct_child_timeline_transports),
        active_group_cue_ids: changed!(active_group_cue_ids),
        cue_live_modifiers: changed!(cue_live_modifiers),
        group_colors: changed!(group_colors),
        active_fade: (before.active_fade != after.active_fade).then(|| after.active_fade.clone()),
        programmer: changed!(programmer),
        timeline: changed!(timeline),
        video: changed!(video),
        effects: changed!(effects),
        node_graphs: changed!(node_graphs),
        output: changed!(output),
        dmx_outputs: changed!(dmx_outputs),
        lighting_master: (before.lighting_master != after.lighting_master)
            .then_some(after.lighting_master),
        submasters: changed!(submasters),
        blackout: (before.blackout != after.blackout).then_some(after.blackout),
        clock: changed!(clock),
        stage_map: changed!(stage_map),
        stage_map_presets: changed!(stage_map_presets),
        stage_objects: changed!(stage_objects),
        dmx_preview: changed!(dmx_preview),
        dmx_previews: changed!(dmx_previews),
        telemetry: changed!(telemetry),
    }
}

#[cfg(test)]
#[path = "snapshot_sync_tests.rs"]
mod tests;
