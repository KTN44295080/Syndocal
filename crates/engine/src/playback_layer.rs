//! Authored base versus the playback layer (DEC-PLAYBACK-LAYER-001,
//! `qa/PLAYBACK_LAYER_AUTHORITY_ADR_2026-10-17.md`).
//!
//! Live output reads one merged level per fixture attribute, exactly as
//! before. Each write says which layer it belongs to:
//!
//! - an authored write (fader, preset, palette, programmer commit, patch,
//!   load) sets the live level and makes it the authored level;
//! - a playback write (Cue snap/fade/release, move-in-black, Scene Block,
//!   Timeline automation) sets only the live level. The first playback write
//!   to a key keeps the authored level underneath it.
//!
//! The persistence image reports the authored level, so playback never moves
//! the project checkpoint hash, Undo/Redo or exact authority tokens. An
//! authored write to a key takes it back from playback (last writer wins, as
//! before).

use std::collections::{BTreeMap, HashMap, HashSet};
use std::ops::Deref;

use protocol::{
    CueId, EffectId, EngineSnapshot, FixtureId, NodeGraphId, PlaybackCueListActiveCue,
    PlaybackCueReleaseTargets, PlaybackEnabledFlag, PlaybackLayerSnapshot, PlaybackLevel,
    PlaybackVideoOutput, VideoOutputId, VideoOutputSummary,
};

use super::{
    sanitize_live_modifier_phase, sanitize_live_modifier_segment, sanitize_live_modifier_size,
    sanitize_live_modifier_speed, CueLiveModifierSettings, EngineRuntime, RuntimeVideoOutput,
};

/// How a project load treats the playback layer.
#[derive(Debug, Clone, Default, PartialEq)]
pub enum PlaybackLoadPolicy {
    /// Open, recent, template, backup restore, recovery: no Cue live.
    #[default]
    Discard,
    /// Undo/Redo: keep the current playback layer over the incoming authored
    /// state. A live Cue the incoming image no longer has is released, as
    /// Remove Cue releases it; overrides for removed items are dropped.
    KeepCurrent,
    /// Standby: install the Primary's layer. A reference the incoming image
    /// cannot satisfy rejects the load unchanged.
    Apply(PlaybackLayerSnapshot),
}

/// Authored video-output fields under playback overrides, one per field:
/// `Some` holds the authored value of a field playback owns.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct VideoOutputPlaybackFields {
    pub(crate) enabled: Option<bool>,
    pub(crate) opacity: Option<f32>,
    pub(crate) blackout: Option<bool>,
}

/// Which video-output fields a write touches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VideoOutputFieldMask {
    pub(crate) enabled: bool,
    pub(crate) opacity: bool,
    pub(crate) blackout: bool,
}

impl VideoOutputFieldMask {
    pub(crate) const ENABLED: Self = Self {
        enabled: true,
        opacity: false,
        blackout: false,
    };
    pub(crate) const OPACITY: Self = Self {
        enabled: false,
        opacity: true,
        blackout: false,
    };
    pub(crate) const BLACKOUT: Self = Self {
        enabled: false,
        opacity: false,
        blackout: true,
    };

    /// The fields whose values differ between two summaries.
    pub(crate) fn changed(before: &VideoOutputSummary, after: &VideoOutputSummary) -> Self {
        Self {
            enabled: before.enabled != after.enabled,
            opacity: before.opacity.to_bits() != after.opacity.to_bits(),
            blackout: before.blackout != after.blackout,
        }
    }
}

impl VideoOutputPlaybackFields {
    pub(crate) fn is_empty(&self) -> bool {
        self.enabled.is_none() && self.opacity.is_none() && self.blackout.is_none()
    }

    /// An authored write to these fields takes them back from playback.
    pub(crate) fn take_back(&mut self, fields: VideoOutputFieldMask) {
        if fields.enabled {
            self.enabled = None;
        }
        if fields.opacity {
            self.opacity = None;
        }
        if fields.blackout {
            self.blackout = None;
        }
    }
}

pub(crate) type FixtureValueKey = (FixtureId, String);

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct FixtureValueLayers {
    live: HashMap<FixtureValueKey, u16>,
    /// The authored level under every key playback currently owns. `None`
    /// means the key had no authored level (the fixture default applies).
    authored_under_playback: HashMap<FixtureValueKey, Option<u16>>,
}

impl Deref for FixtureValueLayers {
    type Target = HashMap<FixtureValueKey, u16>;

    fn deref(&self) -> &Self::Target {
        &self.live
    }
}

impl FixtureValueLayers {
    pub(crate) fn clear(&mut self) {
        self.live.clear();
        self.authored_under_playback.clear();
    }

    pub(crate) fn set_authored(&mut self, key: FixtureValueKey, value: u16) {
        self.authored_under_playback.remove(&key);
        self.live.insert(key, value);
    }

    pub(crate) fn extend_authored(
        &mut self,
        values: impl IntoIterator<Item = (FixtureValueKey, u16)>,
    ) {
        for (key, value) in values {
            self.set_authored(key, value);
        }
    }

    /// Drops keys from both layers, e.g. when a fixture is removed.
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&FixtureValueKey) -> bool) {
        self.live.retain(|key, _| keep(key));
        self.authored_under_playback.retain(|key, _| keep(key));
    }

    /// A playback write. This sits on the per-tick fade path: it costs one
    /// lookup when the key is already owned and does not clone the key.
    pub(crate) fn set_playback(&mut self, key: &FixtureValueKey, value: u16) {
        if !self.authored_under_playback.contains_key(key) {
            let authored = self.live.get(key).copied();
            self.authored_under_playback.insert(key.clone(), authored);
        }
        match self.live.get_mut(key) {
            Some(slot) => *slot = value,
            None => {
                self.live.insert(key.clone(), value);
            }
        }
    }

    /// A playback write to a key that must already exist, matched without
    /// building an owned key (child-Timeline automation lanes).
    pub(crate) fn set_playback_existing(
        &mut self,
        fixture_id: FixtureId,
        attribute: &str,
        value: u16,
    ) {
        let Some((key, slot)) =
            self.live
                .iter_mut()
                .find(|((candidate, candidate_attribute), _)| {
                    *candidate == fixture_id && candidate_attribute == attribute
                })
        else {
            return;
        };
        if !self.authored_under_playback.contains_key(key) {
            self.authored_under_playback
                .insert(key.clone(), Some(*slot));
        }
        *slot = value;
    }

    pub(crate) fn extend_playback<'a>(
        &mut self,
        values: impl IntoIterator<Item = (&'a FixtureValueKey, &'a u16)>,
    ) {
        for (key, value) in values {
            self.set_playback(key, *value);
        }
    }

    /// Every playback-owned key with the authored level under it.
    pub(crate) fn authored_under_playback(
        &self,
    ) -> impl Iterator<Item = (&FixtureValueKey, Option<u16>)> {
        self.authored_under_playback
            .iter()
            .map(|(key, authored)| (key, *authored))
    }

    /// Live levels of every key playback owns, sorted for a deterministic
    /// playback-layer snapshot.
    pub(crate) fn playback_levels(&self) -> Vec<(FixtureValueKey, u16)> {
        let mut levels = self
            .authored_under_playback
            .keys()
            .filter_map(|key| self.live.get(key).map(|value| (key.clone(), *value)))
            .collect::<Vec<_>>();
        levels.sort();
        levels
    }
}

impl EngineRuntime {
    /// The live playback layer. A running fade is captured at its targets:
    /// a load cannot resume it, and a frozen intermediate level would belong
    /// to no Cue (a release fade) or never finish.
    pub(crate) fn capture_playback_layer(&self) -> PlaybackLayerSnapshot {
        let mut effects = self
            .effects
            .iter()
            .filter(|effect| self.effect_enabled_under_playback.contains_key(&effect.id))
            .map(|effect| PlaybackEnabledFlag {
                id: effect.id,
                enabled: effect.enabled,
            })
            .collect::<Vec<_>>();
        effects.sort_by_key(|flag| flag.id);
        let mut node_graphs = self
            .node_graphs
            .iter()
            .filter(|graph| graph.enabled_under_playback.is_some())
            .map(|graph| PlaybackEnabledFlag {
                id: graph.summary.id,
                enabled: graph.summary.enabled,
            })
            .collect::<Vec<_>>();
        node_graphs.sort_by_key(|flag| flag.id);
        let fade = self.active_fade.as_ref();
        let mut video_outputs = self
            .video_outputs
            .iter()
            .filter_map(|output| {
                let id = output.summary.id;
                let owned = &output.under_playback;
                let fade_opacity =
                    fade.and_then(|fade| fade.video_output_target_opacities.get(&id));
                let fade_target = fade.and_then(|fade| fade.video_output_targets.get(&id));
                if owned.is_empty() && fade_opacity.is_none() && fade_target.is_none() {
                    return None;
                }
                Some(PlaybackVideoOutput {
                    output_id: id,
                    enabled: fade_target
                        .map(|target| target.enabled)
                        .or(owned.enabled.map(|_| output.summary.enabled)),
                    opacity: fade_opacity
                        .copied()
                        .or(owned.opacity.map(|_| output.summary.opacity)),
                    blackout: fade_target
                        .map(|target| target.blackout)
                        .or(owned.blackout.map(|_| output.summary.blackout)),
                })
            })
            .collect::<Vec<_>>();
        video_outputs.sort_by_key(|output| output.output_id);
        let mut cue_list_active_cue_ids = self
            .cue_lists
            .iter()
            .filter_map(|cue_list| {
                cue_list
                    .active_cue_id
                    .map(|cue_id| PlaybackCueListActiveCue {
                        cue_list_id: cue_list.id,
                        cue_id,
                    })
            })
            .collect::<Vec<_>>();
        cue_list_active_cue_ids.sort_by_key(|entry| entry.cue_list_id);
        let mut releasable_cue_ids = self.cue_release_values.keys().copied().collect::<Vec<_>>();
        releasable_cue_ids.sort_unstable();
        let release_targets = releasable_cue_ids
            .iter()
            .map(|cue_id| {
                let mut values = self.cue_release_values[cue_id]
                    .iter()
                    .map(|((fixture_id, attribute), value)| {
                        (*fixture_id, attribute.clone(), *value)
                    })
                    .collect::<Vec<_>>();
                values.sort();
                PlaybackCueReleaseTargets {
                    cue_id: *cue_id,
                    values,
                }
            })
            .collect();
        let mut levels = self
            .values
            .playback_levels()
            .into_iter()
            .collect::<BTreeMap<_, _>>();
        if let Some(fade) = fade {
            levels.extend(
                fade.target_values
                    .iter()
                    .map(|(key, value)| (key.clone(), *value)),
            );
        }
        PlaybackLayerSnapshot {
            active_cue_id: self.active_cue_id,
            cue_list_active_cue_ids,
            active_group_cue_ids: self
                .active_group_cue_ids
                .iter()
                .map(|(group_id, cue_id)| (group_id.clone(), *cue_id))
                .collect(),
            releasable_cue_ids,
            live_modifiers: self.cue_live_modifier_states(),
            levels: levels
                .into_iter()
                .map(|(key, value)| PlaybackLevel {
                    cue_list_id: self.cue_value_origins.get(&key).copied(),
                    fixture_id: key.0,
                    attribute: key.1,
                    value,
                })
                .collect(),
            effects,
            node_graphs,
            video_outputs,
            release_targets,
        }
    }

    /// The layer Undo/Redo keeps over `incoming`. Only reads current state, so
    /// a load rejected later leaves the show untouched.
    ///
    /// A live Cue the incoming image no longer has is released as Remove Cue
    /// releases it. A live Cue that still exists keeps playing; its Bank and
    /// group references follow the incoming Cue.
    pub(crate) fn carried_playback_layer(
        &self,
        incoming: &EngineSnapshot,
    ) -> PlaybackLayerSnapshot {
        let mut layer = self.capture_playback_layer();
        let incoming_cue = |cue_id: CueId| incoming.cues.iter().find(|cue| cue.id == cue_id);
        let removed = layer
            .releasable_cue_ids
            .iter()
            .copied()
            .chain(layer.active_cue_id)
            .chain(
                layer
                    .cue_list_active_cue_ids
                    .iter()
                    .map(|entry| entry.cue_id),
            )
            .chain(layer.active_group_cue_ids.values().copied())
            .filter(|cue_id| incoming_cue(*cue_id).is_none())
            .collect::<HashSet<_>>();
        if !removed.is_empty() {
            let mut levels = std::mem::take(&mut layer.levels)
                .into_iter()
                .map(|level| {
                    (
                        (level.fixture_id, level.attribute),
                        (level.value, level.cue_list_id),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            // Remove Cue's immediate release, for every removed Cue at once:
            // a key returns to its release target unless a surviving Cue
            // still holds it. The level stays playback owned and leaves its
            // Bank's scaling.
            for cue_id in &removed {
                let Some(targets) = self.cue_release_values.get(cue_id) else {
                    continue;
                };
                for (key, value) in targets {
                    let held = self.cue_release_values.iter().any(|(other, values)| {
                        !removed.contains(other) && values.contains_key(key)
                    });
                    if !held {
                        levels.insert(key.clone(), (*value, None));
                    }
                }
            }
            layer.levels = levels
                .into_iter()
                .map(
                    |((fixture_id, attribute), (value, cue_list_id))| PlaybackLevel {
                        fixture_id,
                        attribute,
                        value,
                        cue_list_id,
                    },
                )
                .collect();
            layer
                .releasable_cue_ids
                .retain(|cue_id| !removed.contains(cue_id));
            layer
                .release_targets
                .retain(|targets| !removed.contains(&targets.cue_id));
            layer
                .live_modifiers
                .retain(|state| !removed.contains(&state.cue_id));
            layer.active_cue_id = layer
                .active_cue_id
                .filter(|cue_id| !removed.contains(cue_id));
            layer
                .cue_list_active_cue_ids
                .retain(|entry| !removed.contains(&entry.cue_id));
            layer
                .active_group_cue_ids
                .retain(|_, cue_id| !removed.contains(cue_id));
        }
        // A live Cue the undone step moved to another Bank or group stays
        // live under its incoming Bank and group.
        let mut entries = Vec::with_capacity(layer.cue_list_active_cue_ids.len());
        for entry in &layer.cue_list_active_cue_ids {
            let Some(cue) = incoming_cue(entry.cue_id) else {
                continue;
            };
            if !entries
                .iter()
                .any(|kept: &PlaybackCueListActiveCue| kept.cue_list_id == cue.cue_list_id)
            {
                entries.push(PlaybackCueListActiveCue {
                    cue_list_id: cue.cue_list_id,
                    cue_id: entry.cue_id,
                });
            }
        }
        entries.sort_by_key(|entry| entry.cue_list_id);
        layer.cue_list_active_cue_ids = entries;
        let mut groups = BTreeMap::new();
        for cue_id in layer.active_group_cue_ids.values() {
            if let Some(group_id) = incoming_cue(*cue_id).and_then(|cue| cue.group_id.clone()) {
                groups.entry(group_id).or_insert(*cue_id);
            }
        }
        layer.active_group_cue_ids = groups;
        let _ = reconcile_playback_layer(&mut layer, incoming, false);
        layer
    }

    /// Replaces the active Cue references a load installs: none, or the
    /// carried/applied layer's.
    pub(crate) fn install_loaded_active_cue_references(
        &mut self,
        layer: Option<&PlaybackLayerSnapshot>,
    ) {
        self.active_cue_id = None;
        self.active_group_cue_ids.clear();
        for cue_list in &mut self.cue_lists {
            cue_list.active_cue_id = None;
        }
        let Some(layer) = layer else {
            return;
        };
        self.active_cue_id = layer.active_cue_id;
        for entry in &layer.cue_list_active_cue_ids {
            if let Some(cue_list) = self
                .cue_lists
                .iter_mut()
                .find(|cue_list| cue_list.id == entry.cue_list_id)
            {
                cue_list.active_cue_id = Some(entry.cue_id);
            }
        }
        self.active_group_cue_ids = layer
            .active_group_cue_ids
            .iter()
            .map(|(group_id, cue_id)| (group_id.clone(), *cue_id))
            .collect();
    }

    /// Installs the layer's live overrides over freshly loaded authored state.
    pub(crate) fn install_playback_overrides(&mut self, layer: &PlaybackLayerSnapshot) {
        self.cue_value_origins.clear();
        for level in &layer.levels {
            let key = (level.fixture_id, level.attribute.clone());
            self.values.set_playback(&key, level.value);
            if let Some(cue_list_id) = level.cue_list_id {
                self.cue_value_origins.insert(key, cue_list_id);
            }
        }
        for targets in &layer.release_targets {
            if self.cue_release_values.contains_key(&targets.cue_id) {
                let values = targets
                    .values
                    .iter()
                    .map(|(fixture_id, attribute, value)| {
                        ((*fixture_id, attribute.clone()), *value)
                    })
                    .filter(|(key, _)| self.values.contains_key(key))
                    .collect();
                self.cue_release_values.insert(targets.cue_id, values);
            }
        }
        for state in &layer.live_modifiers {
            let Some(cue) = self.cues.iter().find(|cue| cue.id == state.cue_id) else {
                continue;
            };
            let settings = CueLiveModifierSettings {
                speed: sanitize_live_modifier_speed(state.speed),
                size: sanitize_live_modifier_size(state.size),
                phase: sanitize_live_modifier_phase(state.phase),
                direction: state.direction,
                segment: sanitize_live_modifier_segment(state.segment, cue.steps.len()),
                flash: cue.live_modifiers.unwrap_or_default().flash,
            };
            self.cue_live_modifier_overrides
                .insert(state.cue_id, settings);
        }
        for flag in &layer.effects {
            self.set_effect_enabled_by_playback(flag.id, flag.enabled);
        }
        for flag in &layer.node_graphs {
            self.set_node_graph_enabled_by_playback(flag.id, flag.enabled);
        }
        for target in &layer.video_outputs {
            self.set_video_output_fields_by_playback(
                target.output_id,
                target.enabled,
                target.opacity,
                target.blackout,
            );
        }
    }

    /// The single playback writer of an effect's enable flag.
    pub(crate) fn set_effect_enabled_by_playback(&mut self, effect_id: EffectId, enabled: bool) {
        if let Some(effect) = self
            .effects
            .iter_mut()
            .find(|effect| effect.id == effect_id)
        {
            self.effect_enabled_under_playback
                .entry(effect_id)
                .or_insert(effect.enabled);
            effect.enabled = enabled;
        }
    }

    /// The single playback writer of a node graph's enable flag.
    pub(crate) fn set_node_graph_enabled_by_playback(
        &mut self,
        graph_id: NodeGraphId,
        enabled: bool,
    ) {
        if let Some(graph) = self
            .node_graphs
            .iter_mut()
            .find(|graph| graph.summary.id == graph_id)
        {
            graph
                .enabled_under_playback
                .get_or_insert(graph.summary.enabled);
            graph.summary.enabled = enabled;
        }
    }

    /// The single playback writer of video-output enable/opacity/blackout.
    /// `None` leaves a field as it is.
    pub(crate) fn set_video_output_fields_by_playback(
        &mut self,
        output_id: VideoOutputId,
        enabled: Option<bool>,
        opacity: Option<f32>,
        blackout: Option<bool>,
    ) {
        if let Some(output) = self
            .video_outputs
            .iter_mut()
            .find(|output| output.summary.id == output_id)
        {
            write_video_output_by_playback(output, enabled, opacity, blackout);
        }
    }

    /// An authored write to these fields takes them back from playback.
    pub(crate) fn take_back_video_output_fields(
        &mut self,
        output_id: VideoOutputId,
        fields: VideoOutputFieldMask,
    ) {
        if let Some(output) = self
            .video_outputs
            .iter_mut()
            .find(|output| output.summary.id == output_id)
        {
            output.under_playback.take_back(fields);
        }
    }

    /// The authored projection of effect, node-graph and video-output flags.
    pub(crate) fn apply_authored_flag_projection(&self, snapshot: &mut EngineSnapshot) {
        for effect in &mut snapshot.effects {
            if let Some(authored) = self.effect_enabled_under_playback.get(&effect.id) {
                effect.enabled = *authored;
            }
        }
        for graph in &mut snapshot.node_graphs {
            if let Some(authored) = self
                .node_graphs
                .iter()
                .find(|runtime| runtime.summary.id == graph.id)
                .and_then(|runtime| runtime.enabled_under_playback)
            {
                graph.enabled = authored;
            }
        }
        let owned_outputs = self
            .video_outputs
            .iter()
            .filter(|output| !output.under_playback.is_empty())
            .collect::<Vec<_>>();
        if owned_outputs.is_empty() {
            return;
        }
        let video_snapshots =
            std::iter::once(&mut snapshot.video).chain(snapshot.authored_video.as_mut());
        for video in video_snapshots {
            for output in &mut video.outputs {
                if let Some(runtime) = owned_outputs
                    .iter()
                    .find(|runtime| runtime.summary.id == output.id)
                {
                    let authored = runtime.under_playback;
                    output.enabled = authored.enabled.unwrap_or(output.enabled);
                    output.opacity = authored.opacity.unwrap_or(output.opacity);
                    output.blackout = authored.blackout.unwrap_or(output.blackout);
                }
            }
        }
    }
}

/// Whether a Standby can apply `layer` with `snapshot` unchanged; the same
/// check a strict `PlaybackLoadPolicy::Apply` load runs.
pub fn validate_playback_layer(
    layer: &PlaybackLayerSnapshot,
    snapshot: &EngineSnapshot,
) -> Result<(), String> {
    reconcile_playback_layer(&mut layer.clone(), snapshot, true)
}

/// Checks `layer` against the project image it is loaded with. Strict
/// (Standby) rejects the first reference the image cannot satisfy; lenient
/// (Undo/Redo carry) drops it.
pub(crate) fn reconcile_playback_layer(
    layer: &mut PlaybackLayerSnapshot,
    snapshot: &EngineSnapshot,
    strict: bool,
) -> Result<(), String> {
    fn reject(strict: bool, message: String) -> Result<bool, String> {
        if strict {
            Err(format!(
                "Playback layer does not match the project: {message}"
            ))
        } else {
            Ok(false)
        }
    }
    let cue = |cue_id: CueId| snapshot.cues.iter().find(|cue| cue.id == cue_id);
    let mut seen_lists = HashSet::new();
    let mut keep = Vec::with_capacity(layer.cue_list_active_cue_ids.len());
    for entry in &layer.cue_list_active_cue_ids {
        let valid = seen_lists.insert(entry.cue_list_id)
            && snapshot
                .cue_lists
                .iter()
                .any(|cue_list| cue_list.id == entry.cue_list_id)
            && cue(entry.cue_id).is_some_and(|cue| cue.cue_list_id == entry.cue_list_id);
        keep.push(
            valid
                || reject(
                    strict,
                    format!("Bank {} active Cue {}", entry.cue_list_id, entry.cue_id),
                )?,
        );
    }
    let mut keep = keep.into_iter();
    layer
        .cue_list_active_cue_ids
        .retain(|_| keep.next().unwrap_or(false));
    if let Some(cue_id) = layer.active_cue_id {
        let valid = cue(cue_id).is_some_and(|cue| {
            layer
                .cue_list_active_cue_ids
                .iter()
                .any(|entry| entry.cue_list_id == cue.cue_list_id && entry.cue_id == cue_id)
        });
        if !valid && !reject(strict, format!("active Cue {cue_id}"))? {
            layer.active_cue_id = None;
        }
    }
    let mut invalid_groups = Vec::new();
    for (group_id, cue_id) in &layer.active_group_cue_ids {
        let valid =
            cue(*cue_id).is_some_and(|cue| cue.group_id.as_deref() == Some(group_id.as_str()));
        if !valid && !reject(strict, format!("group {group_id} active Cue {cue_id}"))? {
            invalid_groups.push(group_id.clone());
        }
    }
    for group_id in invalid_groups {
        layer.active_group_cue_ids.remove(&group_id);
    }
    let mut keep = Vec::with_capacity(layer.releasable_cue_ids.len());
    for cue_id in &layer.releasable_cue_ids {
        keep.push(cue(*cue_id).is_some() || reject(strict, format!("releasable Cue {cue_id}"))?);
    }
    let mut keep = keep.into_iter();
    layer
        .releasable_cue_ids
        .retain(|_| keep.next().unwrap_or(false));
    let mut keep = Vec::with_capacity(layer.live_modifiers.len());
    for state in &layer.live_modifiers {
        let valid = cue(state.cue_id).is_some()
            && [state.speed, state.size, state.phase]
                .iter()
                .all(|value| value.is_finite());
        keep.push(valid || reject(strict, format!("live modifier of Cue {}", state.cue_id))?);
    }
    let mut keep = keep.into_iter();
    layer
        .live_modifiers
        .retain(|_| keep.next().unwrap_or(false));
    let mut keep = Vec::with_capacity(layer.levels.len());
    for level in &mut layer.levels {
        let valid = snapshot.fixtures.iter().any(|fixture| {
            fixture.id == level.fixture_id
                && fixture
                    .attribute_values
                    .iter()
                    .any(|value| value.attribute == level.attribute)
        });
        keep.push(
            valid
                || reject(
                    strict,
                    format!("fixture {} attribute {}", level.fixture_id, level.attribute),
                )?,
        );
        let bank_exists = level.cue_list_id.is_none_or(|cue_list_id| {
            snapshot
                .cue_lists
                .iter()
                .any(|cue_list| cue_list.id == cue_list_id)
        });
        // A level whose Bank is gone keeps its value and leaves that Bank's
        // scaling, as deleting the Bank does live.
        if !bank_exists
            && !reject(
                strict,
                format!(
                    "Bank of fixture {} attribute {}",
                    level.fixture_id, level.attribute
                ),
            )?
        {
            level.cue_list_id = None;
        }
    }
    let mut keep = keep.into_iter();
    layer.levels.retain(|_| keep.next().unwrap_or(false));
    let mut keep = Vec::with_capacity(layer.effects.len());
    for flag in &layer.effects {
        let valid = snapshot.effects.iter().any(|effect| effect.id == flag.id);
        keep.push(valid || reject(strict, format!("effect {}", flag.id))?);
    }
    let mut keep = keep.into_iter();
    layer.effects.retain(|_| keep.next().unwrap_or(false));
    let mut keep = Vec::with_capacity(layer.node_graphs.len());
    for flag in &layer.node_graphs {
        let valid = snapshot.node_graphs.iter().any(|graph| graph.id == flag.id);
        keep.push(valid || reject(strict, format!("node graph {}", flag.id))?);
    }
    let mut keep = keep.into_iter();
    layer.node_graphs.retain(|_| keep.next().unwrap_or(false));
    let outputs = snapshot
        .authored_video
        .as_ref()
        .unwrap_or(&snapshot.video)
        .outputs
        .iter()
        .map(|output| output.id)
        .collect::<HashSet<_>>();
    let mut keep = Vec::with_capacity(layer.video_outputs.len());
    for output in &layer.video_outputs {
        let valid = outputs.contains(&output.output_id)
            && output
                .opacity
                .is_none_or(|opacity| opacity.is_finite() && (0.0..=1.0).contains(&opacity));
        keep.push(valid || reject(strict, format!("video output {}", output.output_id))?);
    }
    let mut keep = keep.into_iter();
    layer.video_outputs.retain(|_| keep.next().unwrap_or(false));
    Ok(())
}

/// Shared by every playback writer of video-output fields, including the
/// per-tick Cue fade, which holds other field borrows.
pub(crate) fn write_video_output_by_playback(
    output: &mut RuntimeVideoOutput,
    enabled: Option<bool>,
    opacity: Option<f32>,
    blackout: Option<bool>,
) {
    let owned = &mut output.under_playback;
    let summary = &mut output.summary;
    if let Some(enabled) = enabled {
        owned.enabled.get_or_insert(summary.enabled);
        summary.enabled = enabled;
    }
    if let Some(opacity) = opacity {
        owned.opacity.get_or_insert(summary.opacity);
        summary.opacity = opacity;
    }
    if let Some(blackout) = blackout {
        owned.blackout.get_or_insert(summary.blackout);
        summary.blackout = blackout;
    }
}
