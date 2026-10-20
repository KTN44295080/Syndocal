use super::*;
use protocol::audio_mapping::{AudioControlRuntimeValue, AudioControlTarget};

fn audio_control_test_runtime() -> (EngineRuntime, Instant) {
    let (mut runtime, base) = auto_vj_test_runtime(27);
    runtime.set_auto_vj_armed(false, base).unwrap();
    runtime.video_layers[0].state.opacity = 0.9;
    runtime.video_master_opacity = 0.8;
    runtime.video_layer_groups = vec![serde_json::from_value(serde_json::json!({
        "id": 27, "label": "Audio group", "composition_id": 1,
        "layer_ids": [1], "opacity": 0.7
    }))
    .unwrap()];
    runtime.apply_live_audio_frame(test_live_audio_frame(27, 1, &[]), base, base);
    runtime.last_tick = base;
    (runtime, base)
}

fn audio_value(target: AudioControlTarget, value: f32) -> AudioControlRuntimeValue {
    AudioControlRuntimeValue { target, value }
}

#[test]
fn audio_control_repeated_values_do_not_change_authored_persistence() {
    let (mut runtime, base) = audio_control_test_runtime();
    let before = runtime.build_persistence_snapshot().authored_video.unwrap();
    for sequence in 1..=500 {
        let now = base + Duration::from_micros(sequence);
        runtime.apply_live_audio_frame(test_live_audio_frame(27, sequence, &[]), now, now);
        runtime.last_tick = now;
        let fraction = (sequence % 101) as f32 / 100.0;
        runtime
            .set_audio_control_runtime(
                vec![
                    audio_value(AudioControlTarget::LayerOpacity { layer_id: 1 }, fraction),
                    audio_value(
                        AudioControlTarget::GroupOpacity {
                            group_id: protocol::VideoLayerGroupId(27),
                        },
                        0.25,
                    ),
                    audio_value(AudioControlTarget::MasterOpacity, 0.5),
                ],
                27,
                sequence,
                now,
            )
            .unwrap();
        let live = runtime.video_snapshot();
        assert_eq!(
            live.layers
                .iter()
                .find(|layer| layer.id == 1)
                .unwrap()
                .state
                .opacity,
            fraction
        );
        assert_eq!(live.layer_groups[0].opacity, 0.25);
        assert_eq!(live.master_opacity, 0.5);
        let saved = runtime.build_persistence_snapshot();
        assert!(saved.live_audio_textures.is_none());
        let authored = saved.authored_video.unwrap();
        assert_eq!(authored.layers, before.layers);
        assert_eq!(authored.effect_chains, before.effect_chains);
        assert_eq!(authored.layer_groups, before.layer_groups);
        assert_eq!(authored.master_opacity, before.master_opacity);
    }
}

#[test]
fn audio_control_expires_and_rejects_old_generation_and_removed_targets() {
    let (mut runtime, base) = audio_control_test_runtime();
    let values = || {
        vec![audio_value(
            AudioControlTarget::LayerOpacity { layer_id: 1 },
            0.2,
        )]
    };
    runtime
        .set_audio_control_runtime(values(), 27, 1, base)
        .unwrap();
    assert_eq!(runtime.video_snapshot().layers[0].state.opacity, 0.2);
    let fresh = base + Duration::from_millis(240);
    runtime.apply_live_audio_frame(test_live_audio_frame(27, 2, &[]), fresh, fresh);
    runtime.last_tick = base + Duration::from_millis(251);
    assert_eq!(runtime.video_snapshot().layers[0].state.opacity, 0.9);
    let next = base + Duration::from_millis(252);
    runtime.apply_live_audio_frame(test_live_audio_frame(28, 1, &[]), next, next);
    runtime.last_tick = next;
    assert!(runtime
        .set_audio_control_runtime(values(), 27, 2, next)
        .is_err());
    runtime
        .set_audio_control_runtime(values(), 28, 1, next)
        .unwrap();
    runtime.video_layers.retain(|layer| layer.id != 1);
    assert!(runtime
        .set_audio_control_runtime(values(), 28, 1, next)
        .is_err());
    runtime
        .set_audio_control_runtime(Vec::new(), 0, 0, next)
        .unwrap();
    assert_eq!(runtime.video_snapshot().master_opacity, 0.8);
}

#[test]
fn audio_control_invalid_batch_is_atomic_and_bounded() {
    let (mut runtime, base) = audio_control_test_runtime();
    runtime
        .set_audio_control_runtime(
            vec![audio_value(AudioControlTarget::MasterOpacity, 0.3)],
            27,
            1,
            base,
        )
        .unwrap();
    for invalid in [
        vec![
            audio_value(AudioControlTarget::MasterOpacity, 0.6),
            audio_value(AudioControlTarget::LayerOpacity { layer_id: 999 }, 0.6),
        ],
        vec![audio_value(AudioControlTarget::MasterOpacity, f32::NAN)],
        vec![audio_value(AudioControlTarget::MasterOpacity, 0.6); 129],
    ] {
        assert!(runtime
            .set_audio_control_runtime(invalid, 27, 1, base)
            .is_err());
        assert_eq!(runtime.video_snapshot().master_opacity, 0.3);
    }
    assert!(runtime
        .set_audio_control_runtime(
            vec![audio_value(AudioControlTarget::MasterOpacity, 0.6)],
            27,
            2,
            base,
        )
        .is_err());
}

#[test]
fn audio_control_publication_failure_rolls_back_without_authored_mutation() {
    let (mut runtime, base) = audio_control_test_runtime();
    runtime
        .set_audio_control_runtime(
            vec![audio_value(AudioControlTarget::MasterOpacity, 0.3)],
            27,
            1,
            base,
        )
        .unwrap();
    let shared = RwLock::new(runtime.build_snapshot(0));
    let held = shared.read().unwrap();
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_audio_control_runtime_published(
        vec![audio_value(AudioControlTarget::MasterOpacity, 0.6)],
        27,
        1,
        Instant::now() + Duration::from_millis(250),
        ProjectSnapshotLoadAdmission::new(),
        ack,
    );
    runtime.publish_pending_command_acks(0, &shared);
    assert!(receiver.recv().unwrap().is_err());
    assert_eq!(held.video.master_opacity, 0.3);
    drop(held);
    assert_eq!(runtime.video_snapshot().master_opacity, 0.3);
    assert_eq!(
        runtime
            .build_persistence_snapshot()
            .authored_video
            .unwrap()
            .master_opacity,
        0.8
    );
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_audio_control_runtime_published(
        vec![audio_value(AudioControlTarget::MasterOpacity, 0.6)],
        27,
        1,
        Instant::now() + Duration::from_millis(250),
        ProjectSnapshotLoadAdmission::new(),
        ack,
    );
    runtime.publish_pending_command_acks(0, &shared);
    assert!(receiver.recv().unwrap().is_ok());
    assert_eq!(shared.read().unwrap().video.master_opacity, 0.6);
}

#[test]
fn audio_textures_publish_atomically_and_never_serialize() {
    let (mut runtime, base) = audio_control_test_runtime();
    let mut frame = test_live_audio_frame(27, 2, &[]);
    frame.textures = Some(Arc::new(protocol::LiveAudioTextureFrame {
        generation: 27,
        feature_sequence: 2,
        sample_rate: 48_000,
        waveform: [0.25; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
        fft: [0.5; protocol::LIVE_AUDIO_TEXTURE_SAMPLES],
    }));
    runtime.apply_live_audio_frame(frame, base, base);
    let snapshot = runtime.build_snapshot(0);
    let textures = snapshot.live_audio_textures.as_ref().unwrap();
    assert_eq!(textures.feature_sequence, 2);
    assert_eq!(textures.waveform[100], 0.25);
    assert!(serde_json::to_value(&snapshot)
        .unwrap()
        .get("live_audio_textures")
        .is_none());
    assert!(runtime
        .build_persistence_snapshot()
        .live_audio_textures
        .is_none());
    runtime.clear_live_audio_generation(27);
    assert!(runtime.build_snapshot(0).live_audio_textures.is_none());
}

fn audio_test_effect_stage(id: u64) -> VideoEffectStageSummary {
    let effect = VideoIsfEffectSummary {
        enabled: true,
        label: format!("Stage {id}"),
        source: Arc::from(
            "/*{\"ISFVSN\":\"2\",\"INPUTS\":[]}*/ void main() { gl_FragColor=vec4(1.0); }",
        ),
        source_path: None,
        description: None,
        categories: Vec::new(),
        image_bindings: Vec::new(),
        controls: vec![protocol::VideoIsfControlSummary {
            name: "speed".to_string(),
            kind: VideoIsfControlKind::Float,
            value: [0.2, 0.0, 0.0, 0.0],
            default: [0.2, 0.0, 0.0, 0.0],
            minimum: [0.0; 4],
            maximum: [2.0, 0.0, 0.0, 0.0],
            labels: Vec::new(),
            values: Vec::new(),
            tempo_sync: Some(protocol::VideoIsfTempoSync {
                unit: protocol::VideoIsfTempoSyncUnit::Beats,
                count: 1.0,
            }),
        }],
        stack: Vec::new(),
    };
    VideoEffectStageSummary {
        id: VideoEffectStageId(id),
        enabled: true,
        label: format!("Stage {id}"),
        effect: protocol::VideoEffectSummary {
            id: VideoEffectId(id),
            kind: VideoEffectKind::Isf { effect },
        },
    }
}

fn sync_audio_test_legacy_effect(runtime: &mut EngineRuntime) {
    let effects: Vec<_> = runtime.video_effect_chains[0]
        .stages
        .iter()
        .map(|stage| {
            let VideoEffectKind::Isf { effect } = &stage.effect.kind;
            effect.clone()
        })
        .collect();
    let mut legacy = effects[0].clone();
    legacy.stack = effects[1..]
        .iter()
        .map(|effect| VideoIsfEffectStageSummary {
            enabled: effect.enabled,
            label: effect.label.clone(),
            source: effect.source.clone(),
            source_path: effect.source_path.clone(),
            description: effect.description.clone(),
            categories: effect.categories.clone(),
            image_bindings: effect.image_bindings.clone(),
            controls: effect.controls.clone(),
        })
        .collect();
    runtime.video_layers[0].isf_effect = Some(legacy);
}

#[test]
fn audio_control_isf_stable_stage_survives_reorder_and_preserves_authored_tempo_sync() {
    let (mut runtime, base) = audio_control_test_runtime();
    runtime.video_effect_chains = vec![VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope: VideoEffectScope::Layer { layer_id: 1 },
        bypassed: false,
        stages: vec![audio_test_effect_stage(27), audio_test_effect_stage(28)],
    }];
    sync_audio_test_legacy_effect(&mut runtime);
    for target_index in [0, 1] {
        if target_index == 1 {
            runtime.video_effect_chains[0].stages.swap(0, 1);
            sync_audio_test_legacy_effect(&mut runtime);
        }
        let authored = runtime.build_persistence_snapshot().authored_video.unwrap();
        runtime
            .set_audio_control_runtime(
                vec![audio_value(
                    AudioControlTarget::IsfParam {
                        layer_id: 1,
                        stage_id: VideoEffectStageId(27),
                        control_name: "speed".to_string(),
                    },
                    0.75,
                )],
                27,
                1,
                base,
            )
            .unwrap();
        let live = runtime.video_snapshot();
        let chain = &live.effect_chains[0];
        assert_eq!(chain.stages[target_index].id, VideoEffectStageId(27));
        let VideoEffectKind::Isf { effect } = &chain.stages[target_index].effect.kind;
        assert_eq!(effect.controls[0].value[0], 1.5);
        assert!(effect.controls[0].tempo_sync.is_none());
        let legacy = live.layers[0].isf_effect.as_ref().unwrap();
        let live_control = if target_index == 0 {
            &legacy.controls[0]
        } else {
            &legacy.stack[0].controls[0]
        };
        assert_eq!(live_control.value[0], 1.5);
        assert!(live_control.tempo_sync.is_none());
        let saved = runtime.build_persistence_snapshot().authored_video.unwrap();
        assert_eq!(saved.effect_chains, authored.effect_chains);
        assert_eq!(saved.layers[0].isf_effect, authored.layers[0].isf_effect);
    }
    runtime.video_effect_chains[0]
        .stages
        .retain(|stage| stage.id != VideoEffectStageId(27));
    assert!(runtime
        .set_audio_control_runtime(
            vec![audio_value(
                AudioControlTarget::IsfParam {
                    layer_id: 1,
                    stage_id: VideoEffectStageId(27),
                    control_name: "speed".to_string(),
                },
                0.5
            )],
            27,
            1,
            base
        )
        .is_err());
}

#[test]
fn audio_control_ui_edits_cannot_capture_live_values_but_render_snapshots_keep_them() {
    let (mut runtime, base) = audio_control_test_runtime();
    runtime.video_effect_chains = vec![VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope: VideoEffectScope::Layer { layer_id: 1 },
        bypassed: false,
        stages: vec![audio_test_effect_stage(27)],
    }];
    sync_audio_test_legacy_effect(&mut runtime);
    let authored = runtime.build_persistence_snapshot().authored_video.unwrap();
    runtime
        .set_audio_control_runtime(
            vec![
                audio_value(AudioControlTarget::LayerOpacity { layer_id: 1 }, 0.2),
                audio_value(AudioControlTarget::MasterOpacity, 0.3),
                audio_value(
                    AudioControlTarget::GroupOpacity {
                        group_id: protocol::VideoLayerGroupId(27),
                    },
                    0.4,
                ),
                audio_value(
                    AudioControlTarget::IsfParam {
                        layer_id: 1,
                        stage_id: VideoEffectStageId(27),
                        control_name: "speed".into(),
                    },
                    0.75,
                ),
            ],
            27,
            1,
            base,
        )
        .unwrap();
    let published = runtime.build_snapshot(0);
    let rendered = snapshot_public::clone_public_snapshot(&published);
    assert_eq!(rendered.video.layers[0].state.opacity, 0.2);
    assert_eq!(rendered.video.master_opacity, 0.3);
    assert_eq!(rendered.video.layer_groups[0].opacity, 0.4);
    let VideoEffectKind::Isf { effect } = &rendered.video.effect_chains[0].stages[0].effect.kind;
    assert_eq!(effect.controls[0].value[0], 1.5);
    let ui = audio_control_snapshot::clone_ui_snapshot(&published);
    assert_eq!(
        ui.video.layers[0].state.opacity,
        authored.layers[0].state.opacity
    );
    assert_eq!(ui.video.master_opacity, authored.master_opacity);
    assert_eq!(ui.video.layer_groups, authored.layer_groups);
    assert_eq!(ui.video.effect_chains, authored.effect_chains);
    assert_eq!(ui.video.layers[0].isf_effect, authored.layers[0].isf_effect);
    assert!(ui.authored_video.is_none());
    assert!(ui.live_audio_control_targets.is_none());
    assert!(serde_json::to_value(&published)
        .unwrap()
        .get("live_audio_control_targets")
        .is_none());
    // Simulate an unrelated whole-catalog UI edit while modulation is running.
    runtime.video_layer_groups = ui.video.layer_groups;
    runtime.video_layer_groups[0].label = "Renamed during audio".into();
    runtime.video_effect_chains = ui.video.effect_chains;
    let saved = runtime.build_persistence_snapshot().authored_video.unwrap();
    assert_eq!(saved.layer_groups[0].opacity, 0.7);
    assert_eq!(saved.effect_chains, authored.effect_chains);
    assert_eq!(runtime.video_snapshot().layer_groups[0].opacity, 0.4);
}
