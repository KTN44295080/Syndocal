use super::*;

#[test]
fn persistence_video_reuse_matches_reconstruction_during_live_event_pulse() {
    let mut runtime = runtime_with_two_identity_distinct_isf_event_stages();
    runtime
        .pulse_video_layer_isf_event(
            1,
            1,
            "trigger".to_string(),
            Duration::from_millis(50),
            Instant::now(),
        )
        .unwrap();
    // Seed a preset from the live chain too, covering the third persistence
    // clearing surface alongside layer legacy state and authoritative chains.
    runtime.video_effect_presets.push(VideoEffectPresetSummary {
        id: protocol::VideoEffectPresetId(1),
        label: "Pulse capture".into(),
        payload: protocol::VideoEffectPresetPayload {
            bypassed: false,
            stages: vec![protocol::VideoEffectPresetStagePayload {
                enabled: true,
                label: "Event".into(),
                effect: runtime.video_effect_chains[0].stages[0].effect.kind.clone(),
            }],
        },
    });
    let initial = runtime.build_snapshot(0);
    let mut expected = runtime.authored_video_snapshot_from_rendered(&initial.video);
    assert_eq!(initial.authored_video.as_ref(), Some(&expected));
    // Reference the previous reconstruct-then-clear behavior independently of
    // the new in-place reuse. No runtime mutations occur between the captures.
    for layer in &mut expected.layers {
        if let Some(effect) = &mut layer.isf_effect {
            clear_transient_video_isf_event_controls(effect);
        }
    }
    for chain in &mut expected.effect_chains {
        for stage in &mut chain.stages {
            clear_transient_video_effect_kind(&mut stage.effect.kind);
        }
    }
    for preset in &mut expected.effect_presets {
        for stage in &mut preset.payload.stages {
            clear_transient_video_effect_kind(&mut stage.effect);
        }
    }

    let persisted = runtime.build_persistence_snapshot();
    assert_eq!(persisted.authored_video.as_ref(), Some(&expected));
    assert_ne!(initial.authored_video, persisted.authored_video);
    assert_eq!(persisted.video, initial.video);
    let VideoEffectKind::Isf { effect } = &expected.effect_presets[0].payload.stages[0].effect;
    assert_eq!(effect.controls[0].value[0], 0.0);
    let after = runtime.build_snapshot(0);
    assert_eq!(after.video, initial.video);
    assert_eq!(after.authored_video, initial.authored_video);
}
