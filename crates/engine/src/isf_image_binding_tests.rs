#[test]
fn isf_image_binding_migration_roundtrip_and_source_deletion_are_atomic() {
    let mut runtime = EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    let source = media_asset_test_source("isf-image-binding");
    runtime
        .apply_media_asset_transaction(MediaAssetTransaction::Import(MediaAssetImportCandidate {
            assets: vec![media_asset_test_summary(
                30,
                "ISF image source",
                source.clone(),
            )],
            layers: vec![
                media_asset_test_layer_with_default_slot(1, 30, 31, "Consumer", source.clone()),
                media_asset_test_layer_with_default_slot(2, 30, 32, "Source", source),
            ],
        }))
        .unwrap();
    let mut video = runtime.video_snapshot();
    let binding = protocol::VideoIsfImageBinding {
        name: "other".to_string(),
        source: protocol::VideoIsfImageSource::Layer { layer_id: 2 },
    };
    video.layers[0].isf_effect = Some(VideoIsfEffectSummary {
        enabled: true, label: "Bound image".to_string(),
        source: Arc::from("/*{\"INPUTS\":[{\"NAME\":\"inputImage\",\"TYPE\":\"image\"},{\"NAME\":\"other\",\"TYPE\":\"image\"}]}*/\nvoid main(){gl_FragColor=IMG_THIS_PIXEL(other);}"),
        source_path: None, description: None, categories: Vec::new(),
        controls: Vec::new(), image_bindings: vec![binding.clone()], stack: Vec::new(),
    });
    protocol::normalize_legacy_video_effect_chains(&mut video).unwrap();
    let encoded = serde_json::to_string(&video).unwrap();
    let decoded: VideoSnapshot = serde_json::from_str(&encoded).unwrap();
    assert_eq!(decoded, video);
    runtime.apply_command(EngineCommand::LoadProjectSnapshot(EngineSnapshot {
        video: decoded,
        ..EngineSnapshot::default()
    }));
    let before = runtime.video_snapshot();
    assert_eq!(
        before.layers[0].isf_effect.as_ref().unwrap().image_bindings,
        vec![binding]
    );
    assert!(!before.effect_chains.is_empty());
    protocol::validate_engine_ready_video_effect_chains(&before).unwrap();
    runtime.apply_command(EngineCommand::RemoveVideoLayer(2));
    assert_eq!(
        runtime.video_snapshot(),
        before,
        "removing a bound layer must fail before authored/runtime mutation"
    );
    runtime.apply_command(EngineCommand::RemoveVideoLayer(1));
    let after_consumer_removal = runtime.video_snapshot();
    assert_eq!(after_consumer_removal.layers.len(), 1);
    assert_eq!(after_consumer_removal.layers[0].id, 2);
    assert!(after_consumer_removal.effect_chains.is_empty());
    protocol::validate_engine_ready_video_effect_chains(&after_consumer_removal).unwrap();
}

#[test]
fn isf_image_bindings_follow_stages_through_move_and_legacy_projection() {
    let first = VideoIsfEffectStageSummary {
        enabled: true,
        label: "Layer image".to_string(),
        source: Arc::from("/*{}*/void main(){gl_FragColor=vec4(1.0);}"),
        source_path: None,
        description: None,
        categories: Vec::new(),
        controls: Vec::new(),
        image_bindings: vec![protocol::VideoIsfImageBinding {
            name: "other".to_string(),
            source: protocol::VideoIsfImageSource::Layer { layer_id: 42 },
        }],
    };
    let mut second = first.clone();
    second.label = "Still image".to_string();
    second.image_bindings[0].source = protocol::VideoIsfImageSource::Imported {
        path: "images/still.png".to_string(),
    };
    let original = video_isf_effect_from_stages(vec![first.clone(), second.clone()]);
    let moved = mutate_video_isf_effect_stack(
        original,
        VideoIsfStackMutation::Move {
            stage_index: 1,
            delta: -1,
        },
    )
    .unwrap();
    assert_eq!(
        video_isf_effect_into_stages(moved.clone()),
        vec![second.clone(), first.clone()]
    );
    let chain = VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope: VideoEffectScope::Layer { layer_id: 7 },
        bypassed: false,
        stages: [second, first]
            .into_iter()
            .enumerate()
            .map(|(index, stage)| {
                let effect = video_isf_effect_from_stages(vec![stage]).unwrap();
                VideoEffectStageSummary {
                    id: VideoEffectStageId(index as u64 + 1),
                    enabled: true,
                    label: effect.label.clone(),
                    effect: protocol::VideoEffectSummary {
                        id: VideoEffectId(index as u64 + 1),
                        kind: VideoEffectKind::Isf { effect },
                    },
                }
            })
            .collect(),
    };
    assert_eq!(
        project_legacy_isf_from_canonical_chain(&chain).unwrap(),
        moved
    );
}
