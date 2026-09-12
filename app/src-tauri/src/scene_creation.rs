//! Typed, versioned Scene-create request and receipt boundaries.
//!
//! Coordinator locking and engine publication remain in `main.rs`, where the
//! private project transaction machinery lives. This module intentionally owns
//! only the request shape, candidate image, and acknowledged receipt so the
//! two Scene-create modes cannot drift into separate IPC contracts.

use protocol::{
    CueEffectTarget, CueId, CueNodeGraphTarget, CueSummary, EngineSnapshot, RecallMode,
    VideoLayerTarget, VideoOutputTarget,
};
use serde::{Deserialize, Serialize};

use super::{CueCaptureScope, ProjectHistoryMutationResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "mode",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(super) enum AuthoritativeSceneCreateRequest {
    Empty {
        request_id: String,
        cue_list_id: protocol::CueListId,
        expected_epoch: u64,
        expected_revision: u64,
        expected_checkpoint_hash: String,
        owner_id: String,
    },
    CaptureCurrent {
        request_id: String,
        cue_list_id: protocol::CueListId,
        label: String,
        fade_ms: u64,
        authored_beats: Option<f32>,
        capture_scope: CueCaptureScope,
        effect_targets: Vec<CueEffectTarget>,
        expected_epoch: u64,
        expected_revision: u64,
        expected_checkpoint_hash: String,
        owner_id: String,
    },
}

impl AuthoritativeSceneCreateRequest {
    pub(super) fn request_id(&self) -> &str {
        match self {
            Self::Empty { request_id, .. } | Self::CaptureCurrent { request_id, .. } => request_id,
        }
    }
}

#[derive(Debug, Clone)]
pub(super) struct CapturedSceneCreate {
    pub(super) label: String,
    pub(super) group_id: Option<String>,
    pub(super) fade_ms: u64,
    pub(super) authored_beats: Option<f32>,
    pub(super) targets: Vec<protocol::CueFixtureTarget>,
    pub(super) video_targets: Vec<VideoLayerTarget>,
    pub(super) video_output_targets: Vec<VideoOutputTarget>,
    pub(super) node_graph_targets: Vec<CueNodeGraphTarget>,
    pub(super) effect_targets: Vec<CueEffectTarget>,
}

#[derive(Debug, Clone)]
pub(super) enum AuthoritativeSceneCreateKind {
    Empty,
    CaptureCurrent {
        label: String,
        fade_ms: u64,
        authored_beats: Option<f32>,
        capture_scope: CueCaptureScope,
        effect_targets: Vec<CueEffectTarget>,
    },
}

#[derive(Debug, Clone)]
pub(super) enum PreparedAuthoritativeSceneCreateKind {
    Empty,
    CaptureCurrent(CapturedSceneCreate),
}

pub(super) fn add_captured_scene_in_candidate(
    mut snapshot: EngineSnapshot,
    cue_id: CueId,
    cue_list_id: protocol::CueListId,
    captured: &CapturedSceneCreate,
) -> Result<EngineSnapshot, String> {
    if cue_id == 0 {
        return Err("Captured Cue ID must be greater than zero".to_string());
    }
    if !snapshot
        .cue_lists
        .iter()
        .any(|cue_list| cue_list.id == cue_list_id)
    {
        return Err(format!("Bank {cue_list_id} was not found"));
    }
    if snapshot.cues.iter().any(|cue| cue.id == cue_id) {
        return Err(format!("Cue {cue_id} already exists"));
    }
    snapshot.cues.push(CueSummary {
        id: cue_id,
        cue_list_id,
        cue_number: cue_id.to_string(),
        label: captured.label.clone(),
        group_id: captured.group_id.clone(),
        recall_mode: RecallMode::Coexist,
        fade_ms: captured.fade_ms,
        authored_beats: captured.authored_beats,
        targets: captured.targets.clone(),
        video_targets: captured.video_targets.clone(),
        video_output_targets: captured.video_output_targets.clone(),
        node_graph_targets: captured.node_graph_targets.clone(),
        effect_targets: captured.effect_targets.clone(),
        ..CueSummary::default()
    });
    Ok(snapshot)
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct SceneCreationMutationReceipt {
    #[serde(flatten)]
    pub(super) mutation: ProjectHistoryMutationResult,
    pub(super) cue_id: CueId,
}
