//! The runtime playback layer (DEC-PLAYBACK-LAYER-001,
//! `qa/PLAYBACK_LAYER_AUTHORITY_ADR_2026-10-17.md`).
//!
//! Live Cue state is never part of the authored project image: `.sdc`,
//! backups, recovery captures and the project checkpoint hash carry only
//! authored state. A project load keeps the current playback layer for
//! Undo/Redo and applies a Primary's layer on a Standby machine; opening a
//! file starts with no Cue live.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CueId, CueListId, CueLiveModifierState, EffectId, EngineSnapshot, FixtureId, NodeGraphId,
    VideoOutputId,
};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackLayerSnapshot {
    pub active_cue_id: Option<CueId>,
    pub cue_list_active_cue_ids: Vec<PlaybackCueListActiveCue>,
    pub active_group_cue_ids: BTreeMap<String, CueId>,
    /// Every live Cue a release can still act on, including Coexist Cues no
    /// longer named by a Bank's active Cue.
    pub releasable_cue_ids: Vec<CueId>,
    /// Latched live modifiers of live Cues.
    pub live_modifiers: Vec<CueLiveModifierState>,
    /// Live levels of every fixture attribute that playback owns.
    pub levels: Vec<PlaybackLevel>,
    pub effects: Vec<PlaybackEnabledFlag<EffectId>>,
    pub node_graphs: Vec<PlaybackEnabledFlag<NodeGraphId>>,
    pub video_outputs: Vec<PlaybackVideoOutput>,
    /// Exact release targets of each releasable Cue. Carried within one
    /// process (Undo/Redo) only; a Standby rebuilds them from the Cue, so
    /// they never travel in a sidecar.
    #[serde(skip)]
    pub release_targets: Vec<PlaybackCueReleaseTargets>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlaybackCueReleaseTargets {
    pub cue_id: CueId,
    /// (fixture, attribute, value) in key order.
    pub values: Vec<(FixtureId, String, u16)>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackCueListActiveCue {
    pub cue_list_id: CueListId,
    pub cue_id: CueId,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackLevel {
    pub fixture_id: FixtureId,
    pub attribute: String,
    pub value: u16,
    /// The Bank whose executor and playback master scale this level.
    pub cue_list_id: Option<CueListId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackEnabledFlag<Id> {
    pub id: Id,
    pub enabled: bool,
}

/// Live video-output fields that playback owns; `None` fields follow the
/// authored output.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PlaybackVideoOutput {
    pub output_id: VideoOutputId,
    pub enabled: Option<bool>,
    pub opacity: Option<f32>,
    pub blackout: Option<bool>,
}

impl PlaybackLayerSnapshot {
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Whether a project image still carries live Cue references. Version-1
/// files saved before DEC-PLAYBACK-LAYER-001 can; current saves never do.
pub fn has_active_cue_references(snapshot: &EngineSnapshot) -> bool {
    snapshot.active_cue_id.is_some()
        || !snapshot.active_group_cue_ids.is_empty()
        || snapshot
            .cue_lists
            .iter()
            .any(|cue_list| cue_list.active_cue_id.is_some())
}

/// Remove live Cue references from a project image. Callers validate the
/// references first (FC-27), so a malformed file is still rejected rather
/// than silently cleared.
pub fn clear_active_cue_references(snapshot: &mut EngineSnapshot) {
    snapshot.active_cue_id = None;
    snapshot.active_group_cue_ids.clear();
    for cue_list in &mut snapshot.cue_lists {
        cue_list.active_cue_id = None;
    }
}
