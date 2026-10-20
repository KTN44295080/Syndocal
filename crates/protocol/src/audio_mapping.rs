//! Audio source settings share the existing project control-mapping envelope.
use crate::{MidiControlAction, MidiControlMapping, MidiControlMessage, VideoParam};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub const MAX_AUDIO_CONTROL_MAPPINGS: usize = 128;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioControlSource {
    Low,
    Mid,
    High,
    Level,
    Peak,
    SpectralFlux,
    SpectralCentroid,
    DensityFast,
    DensitySlow,
    Kick,
    Snare,
    Onset,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AudioControlMode {
    Continuous,
    Threshold,
    Onset,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AudioMappingSettings {
    pub id: String,
    pub source: AudioControlSource,
    pub mode: AudioControlMode,
    pub gain: f32,
    pub attack_ms: f32,
    pub release_ms: f32,
    pub threshold: f32,
}

/// Resolved stable targets used only by ephemeral engine modulation.
#[derive(Debug, Clone, PartialEq)]
pub enum AudioControlTarget {
    LayerOpacity {
        layer_id: crate::VideoLayerId,
    },
    GroupOpacity {
        group_id: crate::VideoLayerGroupId,
    },
    MasterOpacity,
    IsfParam {
        layer_id: crate::VideoLayerId,
        stage_id: crate::VideoEffectStageId,
        control_name: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct AudioControlRuntimeValue {
    pub target: AudioControlTarget,
    pub value: f32,
}

pub fn audio_control_target(
    mapping: &MidiControlMapping,
) -> Result<Option<AudioControlTarget>, String> {
    let nonzero = |id: Option<u64>| {
        id.filter(|id| *id != 0)
            .ok_or_else(|| "Audio mapping requires a stable nonzero layer id".to_string())
    };
    match mapping.action {
        MidiControlAction::VideoParam if mapping.video_param == Some(VideoParam::Opacity) => {
            if let Some(group) = mapping.group_id.as_ref() {
                if mapping.layer_id.is_some() {
                    return Err(
                        "Audio opacity mapping cannot target both a layer and a group".to_string(),
                    );
                }
                let id = group
                    .parse::<u64>()
                    .ok()
                    .filter(|id| *id != 0)
                    .ok_or_else(|| {
                        "Audio group opacity requires a stable nonzero group id".to_string()
                    })?;
                Ok(Some(AudioControlTarget::GroupOpacity {
                    group_id: crate::VideoLayerGroupId(id),
                }))
            } else {
                Ok(Some(AudioControlTarget::LayerOpacity {
                    layer_id: nonzero(mapping.layer_id)?,
                }))
            }
        }
        MidiControlAction::VideoLayerFade => Ok(Some(AudioControlTarget::LayerOpacity {
            layer_id: nonzero(mapping.layer_id)?,
        })),
        MidiControlAction::VideoMaster => Ok(Some(AudioControlTarget::MasterOpacity)),
        MidiControlAction::VideoIsfParam => {
            let stage_id = mapping
                .isf_stage_id
                .filter(|id| id.0 != 0)
                .ok_or_else(|| "Audio ISF mapping requires a stable stage id".to_string())?;
            let control_name = mapping
                .attribute
                .as_ref()
                .filter(|name| !name.trim().is_empty() && name.len() <= 128)
                .ok_or_else(|| "Audio ISF mapping requires a control name".to_string())?
                .clone();
            Ok(Some(AudioControlTarget::IsfParam {
                layer_id: nonzero(mapping.layer_id)?,
                stage_id,
                control_name,
            }))
        }
        MidiControlAction::VideoClipSlotTrigger => {
            nonzero(mapping.layer_id)?;
            if mapping.clip_slot_id.is_none_or(|id| id.0 == 0) {
                return Err("Audio clip trigger requires a stable slot id".to_string());
            }
            Ok(None)
        }
        MidiControlAction::VideoTake => Ok(None),
        _ => Err("Audio mapping target is unsupported".to_string()),
    }
}

pub fn validate_audio_control_mapping(mapping: &MidiControlMapping) -> Result<(), String> {
    if mapping.message != MidiControlMessage::Audio {
        if mapping.audio.is_some() {
            return Err("Audio source settings require an Audio mapping".to_string());
        }
        return Ok(());
    }
    let settings = mapping
        .audio
        .as_ref()
        .ok_or_else(|| "Audio mapping requires source settings".to_string())?;
    if settings.id.is_empty()
        || settings.id.len() > 80
        || !settings
            .id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(
            "Audio mapping id must contain 1-80 ASCII letters, digits, hyphens or underscores"
                .to_string(),
        );
    }
    if mapping.channel.is_some() || mapping.number != 0 || mapping.feedback.is_some() {
        return Err("Audio mappings have no MIDI channel, number or feedback".to_string());
    }
    for (label, value, max) in [
        ("gain", settings.gain, 16.0),
        ("attack", settings.attack_ms, 10_000.0),
        ("release", settings.release_ms, 10_000.0),
        ("threshold", settings.threshold, 1.0),
        ("low", mapping.low, 1.0),
        ("high", mapping.high, 1.0),
    ] {
        if !value.is_finite() || !(0.0..=max).contains(&value) {
            return Err(format!(
                "Audio mapping {label} must be finite and between 0 and {max}"
            ));
        }
    }
    let continuous = audio_control_target(mapping)?.is_some();
    if continuous != (settings.mode == AudioControlMode::Continuous) {
        return Err("Audio mapping mode must match its continuous or trigger target".to_string());
    }
    Ok(())
}

pub fn validate_audio_control_mappings(mappings: &[MidiControlMapping]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for mapping in mappings {
        validate_audio_control_mapping(mapping)?;
        if let Some(settings) = mapping.audio.as_ref() {
            if !ids.insert(settings.id.as_str()) {
                return Err(format!("Duplicate audio mapping id '{}'", settings.id));
            }
            if ids.len() > MAX_AUDIO_CONTROL_MAPPINGS {
                return Err(format!(
                    "At most {MAX_AUDIO_CONTROL_MAPPINGS} audio mappings are supported"
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping() -> MidiControlMapping {
        serde_json::from_value(serde_json::json!({
            "message": "Audio", "channel": null, "number": 0,
            "action": "VideoParam", "layer_id": 7, "video_param": "Opacity",
            "low": 0.15, "high": 0.8,
            "audio": {
                "id": "audio-unique-1", "source": "Low", "mode": "Continuous",
                "gain": 2.0, "attack_ms": 25.0, "release_ms": 180.0, "threshold": 0.5
            }
        }))
        .unwrap()
    }

    #[test]
    fn audio_mapping_persistence_roundtrip_keeps_stable_target_and_legacy_shape() {
        let mapping = mapping();
        validate_audio_control_mappings(std::slice::from_ref(&mapping)).unwrap();
        let encoded = serde_json::to_value(&mapping).unwrap();
        let decoded: MidiControlMapping = serde_json::from_value(encoded.clone()).unwrap();
        assert_eq!(decoded, mapping);
        assert_eq!(
            audio_control_target(&decoded).unwrap(),
            Some(AudioControlTarget::LayerOpacity { layer_id: 7 })
        );
        let mut legacy = encoded;
        legacy.as_object_mut().unwrap().remove("audio");
        legacy["message"] = serde_json::json!("ControlChange");
        let legacy_mapping: MidiControlMapping = serde_json::from_value(legacy.clone()).unwrap();
        assert!(legacy_mapping.audio.is_none());
        assert_eq!(serde_json::to_value(legacy_mapping).unwrap(), legacy);
    }

    #[test]
    fn audio_mapping_rejects_invalid_numbers_ids_modes_and_hardware_aliases() {
        let valid = mapping();
        for value in [f32::NAN, f32::INFINITY, -0.1, 17.0] {
            let mut invalid = valid.clone();
            invalid.audio.as_mut().unwrap().gain = value;
            assert!(validate_audio_control_mapping(&invalid).is_err());
        }
        assert!(validate_audio_control_mappings(&[valid.clone(), valid.clone()]).is_err());
        let mut invalid = valid.clone();
        invalid.audio.as_mut().unwrap().mode = AudioControlMode::Threshold;
        assert!(validate_audio_control_mapping(&invalid).is_err());
        invalid = valid.clone();
        invalid.channel = Some(0);
        assert!(validate_audio_control_mapping(&invalid).is_err());
        invalid = valid.clone();
        invalid.message = MidiControlMessage::ControlChange;
        assert!(validate_audio_control_mapping(&invalid).is_err());
        let mut trigger = valid;
        trigger.action = MidiControlAction::VideoClipSlotTrigger;
        trigger.clip_slot_id = Some(crate::VideoClipSlotId(91));
        trigger.audio.as_mut().unwrap().mode = AudioControlMode::Onset;
        validate_audio_control_mapping(&trigger).unwrap();
        assert_eq!(audio_control_target(&trigger).unwrap(), None);
    }

    #[test]
    fn audio_group_opacity_resolves_stable_group_id() {
        let mut group = mapping();
        group.layer_id = None;
        group.group_id = Some("12".to_string());
        validate_audio_control_mapping(&group).unwrap();
        assert_eq!(
            audio_control_target(&group).unwrap(),
            Some(AudioControlTarget::GroupOpacity {
                group_id: crate::VideoLayerGroupId(12)
            })
        );
        group.layer_id = Some(1);
        assert!(validate_audio_control_mapping(&group).is_err());
    }
}
