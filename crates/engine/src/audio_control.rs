//! Ephemeral audio modulation. Authored values and history never read this state.
use super::*;
use protocol::audio_mapping::{
    AudioControlRuntimeValue, AudioControlTarget, MAX_AUDIO_CONTROL_MAPPINGS,
};
use protocol::VideoIsfControlSummary;

const AUDIO_CONTROL_TTL: Duration = Duration::from_millis(250);

#[derive(Debug, Clone, Default)]
pub(super) struct RuntimeAudioControls {
    values: Vec<AudioControlRuntimeValue>,
    targets: Arc<[AudioControlTarget]>,
    generation: u64,
    feature_sequence: u64,
    expires_at: Option<Instant>,
}

impl EngineHandle {
    pub fn set_audio_control_runtime_published(
        &self,
        values: Vec<AudioControlRuntimeValue>,
        generation: u64,
        feature_sequence: u64,
    ) -> Result<(), String> {
        let deadline = Instant::now() + AUDIO_CONTROL_TTL;
        let admission = ProjectSnapshotLoadAdmission::new();
        let (ack, receiver) = mpsc::sync_channel(1);
        self.send(EngineCommand::SetAudioControlRuntime {
            values,
            generation,
            feature_sequence,
            expires_at: deadline,
            admission: admission.clone(),
            ack,
        })
        .map_err(|error| error.to_string())?;
        receive_project_snapshot_load_ack(receiver, &admission, deadline, AUDIO_CONTROL_TTL)
    }
}

fn control_value(control: &VideoIsfControlSummary, fraction: f32) -> Result<f32, String> {
    let minimum = control.minimum[0];
    let maximum = control.maximum[0];
    if !minimum.is_finite() || !maximum.is_finite() || minimum > maximum {
        return Err("Audio ISF control has an invalid scalar range".to_string());
    }
    let value = match control.kind {
        VideoIsfControlKind::Float => minimum + (maximum - minimum) * fraction,
        VideoIsfControlKind::Bool => {
            if fraction >= 0.5 {
                1.0
            } else {
                0.0
            }
        }
        VideoIsfControlKind::Long => {
            let count = control
                .values
                .iter()
                .filter(|value| (**value as f32) >= minimum && (**value as f32) <= maximum)
                .count();
            if count == 0 {
                (minimum + (maximum - minimum) * fraction).round()
            } else {
                let index = ((fraction * count as f32).floor() as usize).min(count - 1);
                *control
                    .values
                    .iter()
                    .filter(|value| (**value as f32) >= minimum && (**value as f32) <= maximum)
                    .nth(index)
                    .expect("counted scalar choice") as f32
            }
        }
        _ => return Err("Audio mapping requires a Float, Long or Bool ISF control".to_string()),
    };
    if value.is_finite() {
        Ok(value)
    } else {
        Err("Audio ISF value overflowed".to_string())
    }
}

impl EngineRuntime {
    pub(super) fn apply_audio_control_runtime_published(
        &mut self,
        values: Vec<AudioControlRuntimeValue>,
        generation: u64,
        feature_sequence: u64,
        expires_at: Instant,
        admission: ProjectSnapshotLoadAdmission,
        ack: mpsc::SyncSender<Result<(), String>>,
    ) {
        let rollback = PendingCommandRollback::RestoreAudioControls {
            previous: self.audio_controls.clone(),
        };
        let result = if admission.try_admit_before(expires_at) {
            self.set_audio_control_runtime(values, generation, feature_sequence, Instant::now())
        } else {
            Err("Audio control update expired before engine admission".to_string())
        };
        self.pending_command_acks.push(PendingCommandAck {
            ack: PendingCommandAckSender::Plain(ack),
            result,
            rollback,
            publication_error: "Engine snapshot was busy; audio control update was rolled back",
        });
    }

    pub(super) fn set_audio_control_runtime(
        &mut self,
        values: Vec<AudioControlRuntimeValue>,
        generation: u64,
        feature_sequence: u64,
        now: Instant,
    ) -> Result<(), String> {
        if values.len() > MAX_AUDIO_CONTROL_MAPPINGS {
            return Err("Audio control update exceeds the bounded mapping limit".to_string());
        }
        if values.is_empty() {
            self.audio_controls = RuntimeAudioControls::default();
            return Ok(());
        }
        if self.live_audio_generation != Some(generation)
            || !self.live_audio_source_is_fresh(now)
            || self.last_live_audio_frame_sequence.is_none_or(|current| {
                feature_sequence > current || current.saturating_sub(feature_sequence) > 8
            })
            || (self.audio_controls.generation == generation
                && feature_sequence < self.audio_controls.feature_sequence)
        {
            return Err(
                "Audio control source is stale or belongs to a retired generation".to_string(),
            );
        }
        for entry in &values {
            if !entry.value.is_finite() || !(0.0..=1.0).contains(&entry.value) {
                return Err("Audio control value must be a finite 0-1 fraction".to_string());
            }
            match &entry.target {
                AudioControlTarget::LayerOpacity { layer_id } => {
                    if !self.video_layers.iter().any(|layer| layer.id == *layer_id) {
                        return Err(format!("Audio target layer {layer_id} was removed"));
                    }
                }
                AudioControlTarget::GroupOpacity { group_id } => {
                    if !self
                        .video_layer_groups
                        .iter()
                        .any(|group| group.id == *group_id)
                    {
                        return Err("Audio target group was removed".to_string());
                    }
                }
                AudioControlTarget::MasterOpacity => {}
                AudioControlTarget::IsfParam {
                    layer_id,
                    stage_id,
                    control_name,
                } => {
                    let stage = self
                        .video_effect_chains
                        .iter()
                        .find(|chain| {
                            chain.scope
                                == VideoEffectScope::Layer {
                                    layer_id: *layer_id,
                                }
                        })
                        .and_then(|chain| chain.stages.iter().find(|stage| stage.id == *stage_id))
                        .ok_or_else(|| "Audio ISF target stage was removed".to_string())?;
                    let VideoEffectKind::Isf { effect } = &stage.effect.kind;
                    let control = effect
                        .controls
                        .iter()
                        .find(|control| control.name == *control_name)
                        .ok_or_else(|| "Audio ISF target control was removed".to_string())?;
                    control_value(control, entry.value)?;
                }
            }
        }
        let targets = values.iter().map(|entry| entry.target.clone()).collect();
        self.audio_controls = RuntimeAudioControls {
            values,
            targets,
            generation,
            feature_sequence,
            expires_at: Some(now + AUDIO_CONTROL_TTL),
        };
        Ok(())
    }

    fn audio_controls_are_fresh(&self) -> bool {
        self.audio_controls
            .expires_at
            .is_some_and(|deadline| self.last_tick < deadline)
            && self.live_audio_generation == Some(self.audio_controls.generation)
            && self.live_audio_source_is_fresh(self.last_tick)
    }

    pub(super) fn audio_control_ui_targets(&self) -> Option<Arc<[AudioControlTarget]>> {
        self.audio_controls_are_fresh()
            .then(|| self.audio_controls.targets.clone())
    }

    pub(super) fn apply_audio_controls_to_video(&self, snapshot: &mut VideoSnapshot) {
        if !self.audio_controls_are_fresh() {
            return;
        }
        for entry in &self.audio_controls.values {
            match &entry.target {
                AudioControlTarget::LayerOpacity { layer_id } => {
                    if let Some(layer) = snapshot
                        .layers
                        .iter_mut()
                        .find(|layer| layer.id == *layer_id)
                    {
                        layer.state.opacity = entry.value;
                    }
                }
                AudioControlTarget::GroupOpacity { group_id } => {
                    if let Some(group) = snapshot
                        .layer_groups
                        .iter_mut()
                        .find(|group| group.id == *group_id)
                    {
                        group.opacity = entry.value;
                    }
                }
                AudioControlTarget::MasterOpacity => snapshot.master_opacity = entry.value,
                AudioControlTarget::IsfParam {
                    layer_id,
                    stage_id,
                    control_name,
                } => {
                    let Some(chain) = snapshot.effect_chains.iter_mut().find(|chain| {
                        chain.scope
                            == VideoEffectScope::Layer {
                                layer_id: *layer_id,
                            }
                    }) else {
                        continue;
                    };
                    let Some(index) = chain.stages.iter().position(|stage| stage.id == *stage_id)
                    else {
                        continue;
                    };
                    let VideoEffectKind::Isf { effect } = &mut chain.stages[index].effect.kind;
                    let Some(control) = effect
                        .controls
                        .iter_mut()
                        .find(|control| control.name == *control_name)
                    else {
                        continue;
                    };
                    let Ok(value) = control_value(control, entry.value) else {
                        continue;
                    };
                    control.value[0] = value;
                    control.tempo_sync = None;
                    // Legacy layer filters and scoped chains must see identical live values.
                    if let Some(root) = snapshot
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
                        if let Some(control) = controls.and_then(|controls| {
                            controls
                                .iter_mut()
                                .find(|control| control.name == *control_name)
                        }) {
                            control.value[0] = value;
                            control.tempo_sync = None;
                        }
                    }
                }
            }
        }
    }
}
