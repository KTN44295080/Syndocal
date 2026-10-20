//! UI authoring controls retain their base values while native video keeps modulation.
use crate::EngineHandle;
use protocol::audio_mapping::AudioControlTarget;
use protocol::{EngineSnapshot, VideoEffectKind, VideoEffectScope};

impl EngineHandle {
    /// A UI poll must not turn a live value into project data on the next catalog edit.
    /// Capture the rendered and authored halves under the same publication lock.
    pub fn snapshot_for_ui(&self) -> EngineSnapshot {
        self.snapshot
            .read()
            .map(|snapshot| clone_ui_snapshot(&snapshot))
            .unwrap_or_default()
    }
}

pub(super) fn clone_ui_snapshot(snapshot: &EngineSnapshot) -> EngineSnapshot {
    let mut ui = crate::snapshot_public::clone_public_snapshot(snapshot);
    let (Some(authored), Some(targets)) = (
        snapshot.authored_video.as_ref(),
        snapshot.live_audio_control_targets.as_ref(),
    ) else {
        return ui;
    };
    for target in targets.iter() {
        match target {
            AudioControlTarget::MasterOpacity => ui.video.master_opacity = authored.master_opacity,
            AudioControlTarget::LayerOpacity { layer_id } => {
                if let (Some(base), Some(layer)) = (
                    authored.layers.iter().find(|layer| layer.id == *layer_id),
                    ui.video
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == *layer_id),
                ) {
                    layer.state.opacity = base.state.opacity;
                }
            }
            AudioControlTarget::GroupOpacity { group_id } => {
                if let (Some(base), Some(group)) = (
                    authored
                        .layer_groups
                        .iter()
                        .find(|group| group.id == *group_id),
                    ui.video
                        .layer_groups
                        .iter_mut()
                        .find(|group| group.id == *group_id),
                ) {
                    group.opacity = base.opacity;
                }
            }
            AudioControlTarget::IsfParam {
                layer_id,
                stage_id,
                control_name,
            } => {
                let scope = VideoEffectScope::Layer {
                    layer_id: *layer_id,
                };
                let base = authored
                    .effect_chains
                    .iter()
                    .find(|chain| chain.scope == scope)
                    .and_then(|chain| chain.stages.iter().find(|stage| stage.id == *stage_id));
                let Some(base) = base else { continue };
                let VideoEffectKind::Isf { effect } = &base.effect.kind;
                let Some(control) = effect
                    .controls
                    .iter()
                    .find(|control| control.name == *control_name)
                else {
                    continue;
                };
                let Some(chain) = ui
                    .video
                    .effect_chains
                    .iter_mut()
                    .find(|chain| chain.scope == scope)
                else {
                    continue;
                };
                let Some(index) = chain.stages.iter().position(|stage| stage.id == *stage_id)
                else {
                    continue;
                };
                let VideoEffectKind::Isf { effect } = &mut chain.stages[index].effect.kind;
                if let Some(current) = effect
                    .controls
                    .iter_mut()
                    .find(|current| current.name == *control_name)
                {
                    current.value = control.value;
                    current.tempo_sync = control.tempo_sync.clone();
                }
                if let Some(root) = ui
                    .video
                    .layers
                    .iter_mut()
                    .find(|layer| layer.id == *layer_id)
                    .and_then(|layer| layer.isf_effect.as_mut())
                {
                    let controls = if index == 0 {
                        Some(&mut root.controls)
                    } else {
                        root.stack
                            .get_mut(index - 1)
                            .map(|stage| &mut stage.controls)
                    };
                    if let Some(current) = controls.and_then(|controls| {
                        controls
                            .iter_mut()
                            .find(|current| current.name == *control_name)
                    }) {
                        current.value = control.value;
                        current.tempo_sync = control.tempo_sync.clone();
                    }
                }
            }
        }
    }
    ui
}
