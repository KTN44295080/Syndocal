use crate::{
    VideoEffectKind, VideoEffectScope, VideoIsfEffectSummary, VideoIsfImageBinding,
    VideoIsfImageSource, VideoLayerId, VideoSnapshot, VideoTransitionEffectOwner,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const MAX_VIDEO_ISF_IMAGE_BINDINGS: usize = 4;
pub const MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH: usize = 32;

pub fn validate_video_isf_image_bindings(bindings: &[VideoIsfImageBinding]) -> Result<(), String> {
    if bindings.len() > MAX_VIDEO_ISF_IMAGE_BINDINGS {
        return Err(format!(
            "ISF image input binding limit exceeded: at most {MAX_VIDEO_ISF_IMAGE_BINDINGS} images are supported"
        ));
    }
    let mut names = BTreeSet::new();
    for binding in bindings {
        let mut bytes = binding.name.bytes();
        let valid_first = bytes
            .next()
            .is_some_and(|byte| byte == b'_' || byte.is_ascii_alphabetic());
        if binding.name.len() > 128
            || !valid_first
            || !bytes.all(|byte| byte == b'_' || byte.is_ascii_alphanumeric())
        {
            return Err(format!(
                "ISF image input binding name is invalid: {}",
                binding.name
            ));
        }
        if !names.insert(binding.name.as_str()) {
            return Err(format!(
                "ISF image input binding name is duplicated: {}",
                binding.name
            ));
        }
        if let VideoIsfImageSource::Imported { path } = &binding.source {
            if path.is_empty()
                || path.len() > 4096
                || path.chars().any(|ch| ch == ':' || ch.is_control())
                || path
                    .split(['/', '\\'])
                    .any(|part| part.is_empty() || part == "." || part == "..")
            {
                return Err(format!(
                    "ISF imported image path is invalid: {path}; choose a child PNG/JPEG path relative to the shader"
                ));
            }
        }
    }
    Ok(())
}

/// Admission/persistence checks all authored choices, including disabled stages
/// and inactive clips. Runtime resolves only its requested active closure.
/// Composition/group/output/bus effects read completed layers without owning edges.
/// Timeline Follow rejects layer sources. Preset references must still exist.
pub fn validate_video_isf_image_graph(video: &VideoSnapshot) -> Result<(), String> {
    let mut dependencies = BTreeMap::<VideoLayerId, BTreeSet<VideoLayerId>>::new();
    for layer in &video.layers {
        if dependencies.insert(layer.id, BTreeSet::new()).is_some() {
            return Err(format!(
                "ISF image input layer identity is duplicated: {}",
                layer.id
            ));
        }
    }
    for layer in &video.layers {
        if let Some(effect) = &layer.isf_effect {
            collect_effect_bindings(effect, Some(layer.id), &mut dependencies)?;
        }
    }
    for chain in &video.effect_chains {
        if matches!(
            &chain.scope,
            VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::TimelineFollow { .. }
            }
        ) && chain.stages.iter().any(|stage| {
            let VideoEffectKind::Isf { effect } = &stage.effect.kind;
            effect
                .image_bindings
                .iter()
                .chain(
                    effect
                        .stack
                        .iter()
                        .flat_map(|stage| stage.image_bindings.iter()),
                )
                .any(|binding| matches!(&binding.source, VideoIsfImageSource::Layer { .. }))
        }) {
            return Err("ISF layer image bindings are unavailable for Timeline Follow transitions; choose an imported still or none".to_string());
        }
        let owner = match &chain.scope {
            VideoEffectScope::Clip { layer_id, .. }
            | VideoEffectScope::Layer { layer_id }
            | VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::ClipTake { layer_id },
            } => Some(*layer_id),
            VideoEffectScope::Transition { .. }
            | VideoEffectScope::Composition { .. }
            | VideoEffectScope::Group { .. }
            | VideoEffectScope::Output { .. } => None,
        };
        for stage in &chain.stages {
            let VideoEffectKind::Isf { effect } = &stage.effect.kind;
            collect_effect_bindings(effect, owner, &mut dependencies)?;
        }
    }
    for preset in &video.effect_presets {
        for stage in &preset.payload.stages {
            let VideoEffectKind::Isf { effect } = &stage.effect;
            collect_effect_bindings(effect, None, &mut dependencies)?;
        }
    }
    validate_layer_dependencies(&dependencies)
}

fn collect_effect_bindings(
    effect: &VideoIsfEffectSummary,
    owner: Option<VideoLayerId>,
    dependencies: &mut BTreeMap<VideoLayerId, BTreeSet<VideoLayerId>>,
) -> Result<(), String> {
    collect_stage_bindings(&effect.image_bindings, owner, dependencies)?;
    for stage in &effect.stack {
        collect_stage_bindings(&stage.image_bindings, owner, dependencies)?;
    }
    Ok(())
}

fn collect_stage_bindings(
    bindings: &[VideoIsfImageBinding],
    owner: Option<VideoLayerId>,
    dependencies: &mut BTreeMap<VideoLayerId, BTreeSet<VideoLayerId>>,
) -> Result<(), String> {
    validate_video_isf_image_bindings(bindings)?;
    for binding in bindings {
        if let VideoIsfImageSource::Layer { layer_id } = &binding.source {
            if !dependencies.contains_key(layer_id) {
                return Err(format!(
                    "ISF image input layer is missing: {layer_id} for {}",
                    binding.name
                ));
            }
            if let Some(owner_id) = owner {
                let inputs = dependencies
                    .get_mut(&owner_id)
                    .ok_or_else(|| format!("ISF image input layer is missing: {owner_id}"))?;
                inputs.insert(*layer_id);
            }
        }
    }
    Ok(())
}

fn validate_layer_dependencies(
    dependencies: &BTreeMap<VideoLayerId, BTreeSet<VideoLayerId>>,
) -> Result<(), String> {
    let mut remaining: BTreeMap<_, _> = dependencies
        .iter()
        .map(|(&id, inputs)| (id, inputs.len()))
        .collect();
    let mut dependents: BTreeMap<VideoLayerId, Vec<VideoLayerId>> = BTreeMap::new();
    let mut depth: BTreeMap<VideoLayerId, usize> =
        dependencies.keys().copied().map(|id| (id, 0)).collect();
    for (&owner, inputs) in dependencies {
        for &input in inputs {
            if !dependencies.contains_key(&input) {
                return Err(format!("ISF image input layer is missing: {input}"));
            }
            dependents.entry(input).or_default().push(owner);
        }
    }
    let mut ready: VecDeque<_> = remaining
        .iter()
        .filter_map(|(&id, &count)| (count == 0).then_some(id))
        .collect();
    let mut visited = 0;
    while let Some(id) = ready.pop_front() {
        visited += 1;
        if let Some(owners) = dependents.get(&id) {
            for &owner in owners {
                let owner_depth = depth[&owner].max(depth[&id] + 1);
                if owner_depth > MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH {
                    return Err(format!(
                        "ISF image input dependency depth limit exceeded: layer {owner}; maximum {MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH}"
                    ));
                }
                depth.insert(owner, owner_depth);
                if let Some(count) = remaining.get_mut(&owner) {
                    *count -= 1;
                    if *count == 0 {
                        ready.push_back(owner);
                    }
                }
            }
        }
    }
    if visited != dependencies.len() {
        let blocked = remaining
            .iter()
            .find(|(_, count)| **count > 0)
            .map(|(&id, _)| id)
            .unwrap_or_default();
        return Err(format!(
            "ISF image input dependency cycle: layer {blocked} cannot resolve its inputs"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        VideoBlendMode, VideoEffectChainId, VideoEffectChainSummary, VideoEffectId,
        VideoEffectPresetId, VideoEffectPresetPayload, VideoEffectPresetStagePayload,
        VideoEffectPresetSummary, VideoEffectStageId, VideoEffectStageSummary, VideoEffectSummary,
        VideoIsfEffectStageSummary, VideoLayerState, VideoLayerSummary, VideoSourceKind,
        VideoSourceSummary,
    };
    use serde_json::json;
    use std::sync::Arc;

    fn layer_binding(name: &str, layer_id: VideoLayerId) -> VideoIsfImageBinding {
        VideoIsfImageBinding {
            name: name.to_string(),
            source: VideoIsfImageSource::Layer { layer_id },
        }
    }

    fn effect(bindings: Vec<VideoIsfImageBinding>) -> VideoIsfEffectSummary {
        VideoIsfEffectSummary {
            enabled: true, label: "Image fixture".to_string(),
            source: Arc::from("/*{\"INPUTS\":[{\"NAME\":\"inputImage\",\"TYPE\":\"image\"},{\"NAME\":\"other\",\"TYPE\":\"image\"}]}*/\nvoid main(){gl_FragColor=IMG_THIS_PIXEL(other);}"),
            source_path: Some("fixtures/blend.fs".to_string()),
            description: None, categories: Vec::new(), controls: Vec::new(),
            image_bindings: bindings, stack: Vec::new(),
        }
    }

    fn layer(id: VideoLayerId, input: Option<VideoLayerId>) -> VideoLayerSummary {
        VideoLayerSummary {
            id,
            label: format!("Layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: None,
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: input.map(|input| effect(vec![layer_binding("other", input)])),
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        }
    }

    fn chain(
        id: u64,
        scope: VideoEffectScope,
        effect: VideoIsfEffectSummary,
    ) -> VideoEffectChainSummary {
        VideoEffectChainSummary {
            id: VideoEffectChainId(id),
            scope,
            bypassed: false,
            stages: vec![VideoEffectStageSummary {
                id: VideoEffectStageId(id),
                enabled: true,
                label: "Fixture".to_string(),
                effect: VideoEffectSummary {
                    id: VideoEffectId(id),
                    kind: VideoEffectKind::Isf { effect },
                },
            }],
        }
    }

    #[test]
    fn isf_image_project_roundtrip_preserves_stable_layer_imported_and_none_bindings() {
        let bindings = vec![
            layer_binding("other", 42),
            VideoIsfImageBinding {
                name: "still".to_string(),
                source: VideoIsfImageSource::Imported {
                    path: "images/still.png".to_string(),
                },
            },
            VideoIsfImageBinding {
                name: "empty".to_string(),
                source: VideoIsfImageSource::None {},
            },
        ];
        let mut authored = effect(bindings.clone());
        authored.stack.push(VideoIsfEffectStageSummary {
            enabled: true,
            label: "Second".to_string(),
            source: authored.source.clone(),
            source_path: authored.source_path.clone(),
            description: None,
            categories: Vec::new(),
            controls: Vec::new(),
            image_bindings: bindings,
        });
        let mut video = VideoSnapshot {
            layers: vec![layer(7, None), layer(42, None)],
            ..VideoSnapshot::default()
        };
        video.layers[0].isf_effect = Some(authored);
        validate_video_isf_image_graph(&video).unwrap();
        let encoded = serde_json::to_string(&video).unwrap();
        let decoded: VideoSnapshot = serde_json::from_str(&encoded).unwrap();
        assert_eq!(decoded, video);
        assert!(encoded.contains("\"layer_id\":42"));
        assert!(encoded.contains("\"kind\":\"none\""));
        validate_video_isf_image_graph(&decoded).unwrap();
    }

    #[test]
    fn isf_image_legacy_projects_default_bindings_without_changing_serialized_shape() {
        let encoded = serde_json::to_value(effect(Vec::new())).unwrap();
        assert!(encoded.get("image_bindings").is_none());
        let mut legacy = encoded.clone();
        legacy["stack"] = json!([encoded]);
        let decoded: VideoIsfEffectSummary = serde_json::from_value(legacy).unwrap();
        assert!(decoded.image_bindings.is_empty());
        assert!(decoded.stack[0].image_bindings.is_empty());
        assert!(serde_json::to_value(decoded.stack[0].clone())
            .unwrap()
            .get("image_bindings")
            .is_none());
    }

    #[test]
    fn isf_image_unknown_or_ambiguous_source_payloads_are_rejected() {
        for source in [
            json!({"kind":"unknown"}),
            json!({"kind":"none","layer_id":7}),
            json!({"kind":"layer","layer_id":7,"path":"still.png"}),
        ] {
            assert!(serde_json::from_value::<VideoIsfImageBinding>(
                json!({"name":"other","source":source})
            )
            .is_err());
        }
    }

    #[test]
    fn isf_image_binding_shape_rejects_duplicates_limits_names_and_unsafe_paths() {
        let duplicate = layer_binding("other", 7);
        assert!(
            validate_video_isf_image_bindings(&[duplicate.clone(), duplicate])
                .unwrap_err()
                .contains("duplicated")
        );
        let too_many: Vec<_> = (0..=MAX_VIDEO_ISF_IMAGE_BINDINGS)
            .map(|index| layer_binding(&format!("image{index}"), 7))
            .collect();
        assert!(validate_video_isf_image_bindings(&too_many)
            .unwrap_err()
            .contains("limit exceeded"));
        for name in ["", "1image", "bad-name", "画像"] {
            assert!(validate_video_isf_image_bindings(&[layer_binding(name, 7)])
                .unwrap_err()
                .contains("name is invalid"));
        }
        for path in [
            "",
            "../still.png",
            "dir/../still.png",
            "C:\\still.png",
            "\\\\server\\still.png",
            "https://host/still.png",
            "still.png:stream",
        ] {
            let binding = VideoIsfImageBinding {
                name: "still".to_string(),
                source: VideoIsfImageSource::Imported {
                    path: path.to_string(),
                },
            };
            assert!(validate_video_isf_image_bindings(&[binding])
                .unwrap_err()
                .contains("path is invalid"));
        }
        let safe = VideoIsfImageBinding {
            name: "still".to_string(),
            source: VideoIsfImageSource::Imported {
                path: "images\\still.png".to_string(),
            },
        };
        validate_video_isf_image_bindings(&[safe]).unwrap();
    }

    #[test]
    fn isf_image_graph_rejects_direct_and_indirect_cycles_even_when_disabled() {
        let direct = VideoSnapshot {
            layers: vec![layer(1, Some(1))],
            ..VideoSnapshot::default()
        };
        assert!(validate_video_isf_image_graph(&direct)
            .unwrap_err()
            .contains("dependency cycle"));
        let mut indirect = VideoSnapshot {
            layers: vec![layer(1, Some(2)), layer(2, Some(3)), layer(3, Some(1))],
            ..VideoSnapshot::default()
        };
        indirect.layers[1].isf_effect.as_mut().unwrap().enabled = false;
        assert!(validate_video_isf_image_graph(&indirect)
            .unwrap_err()
            .contains("dependency cycle"));
    }

    #[test]
    fn isf_image_graph_survives_reordering_and_rejects_deleted_sources() {
        let mut video = VideoSnapshot {
            layers: vec![layer(1, Some(2)), layer(2, None)],
            ..VideoSnapshot::default()
        };
        validate_video_isf_image_graph(&video).unwrap();
        video.layers.reverse();
        validate_video_isf_image_graph(&video).unwrap();
        let owner = video.layers.iter().find(|layer| layer.id == 1).unwrap();
        assert_eq!(
            owner.isf_effect.as_ref().unwrap().image_bindings[0],
            layer_binding("other", 2)
        );
        video.layers.retain(|layer| layer.id != 2);
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("layer is missing: 2"));
    }

    #[test]
    fn isf_image_graph_checks_canonical_chains_and_inactive_clip_take_dependencies() {
        let mut video = VideoSnapshot {
            layers: vec![layer(1, None), layer(2, Some(1))],
            ..VideoSnapshot::default()
        };
        let mut authored = chain(
            10,
            VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::ClipTake { layer_id: 1 },
            },
            effect(vec![layer_binding("other", 2)]),
        );
        authored.bypassed = true;
        authored.stages[0].enabled = false;
        video.effect_chains.push(authored);
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("dependency cycle"));
        video.effect_chains[0].scope = VideoEffectScope::Layer { layer_id: 1 };
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("dependency cycle"));
    }

    #[test]
    fn isf_image_detached_scope_and_preset_sources_must_exist() {
        let mut video = VideoSnapshot {
            layers: vec![layer(1, None)],
            ..VideoSnapshot::default()
        };
        video.effect_chains.push(chain(
            10,
            VideoEffectScope::Output { output_id: 1 },
            effect(vec![layer_binding("other", 99)]),
        ));
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("layer is missing: 99"));
        video.effect_chains.clear();
        video.effect_presets.push(VideoEffectPresetSummary {
            id: VideoEffectPresetId(1),
            label: "Bound preset".to_string(),
            payload: VideoEffectPresetPayload {
                bypassed: false,
                stages: vec![VideoEffectPresetStagePayload {
                    enabled: true,
                    label: "Bound".to_string(),
                    effect: VideoEffectKind::Isf {
                        effect: effect(vec![layer_binding("other", 99)]),
                    },
                }],
            },
        });
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("layer is missing: 99"));
    }

    #[test]
    fn isf_image_graph_bounds_longest_dependency_path_without_recursion() {
        let count = MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH as u64 + 1;
        let mut video = VideoSnapshot {
            layers: (1..=count)
                .map(|id| layer(id, (id > 1).then_some(id - 1)))
                .collect(),
            ..VideoSnapshot::default()
        };
        validate_video_isf_image_graph(&video).unwrap();
        video.layers.push(layer(count + 1, Some(count)));
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("dependency depth limit exceeded"));
    }
    #[test]
    fn isf_image_follow_scope_accepts_static_or_none_and_rejects_layer_bindings() {
        let mut video = VideoSnapshot {
            layers: vec![layer(1, None)],
            ..VideoSnapshot::default()
        };
        video.effect_chains.push(chain(
            10,
            VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::TimelineFollow {
                    source_timeline_id: crate::TimelineId(7),
                },
            },
            effect(vec![layer_binding("other", 1)]),
        ));
        assert!(validate_video_isf_image_graph(&video)
            .unwrap_err()
            .contains("Timeline Follow"));
        let VideoEffectKind::Isf { effect } = &mut video.effect_chains[0].stages[0].effect.kind;
        effect.image_bindings[0].source = VideoIsfImageSource::None {};
        validate_video_isf_image_graph(&video).unwrap();
        let VideoEffectKind::Isf { effect } = &mut video.effect_chains[0].stages[0].effect.kind;
        effect.image_bindings[0].source = VideoIsfImageSource::Imported {
            path: "still.png".into(),
        };
        validate_video_isf_image_graph(&video).unwrap();
    }
}
