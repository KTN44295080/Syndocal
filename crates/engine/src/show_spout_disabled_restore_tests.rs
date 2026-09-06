use super::*;

fn show_spout_restore_fade_shape(
    runtime: &EngineRuntime,
) -> Vec<(VideoOutputId, Instant, Duration, f32, f32)> {
    runtime
        .video_output_fades
        .iter()
        .map(|fade| {
            (
                fade.output_id,
                fade.started_at,
                fade.duration,
                fade.start_opacity,
                fade.target_opacity,
            )
        })
        .collect()
}

fn show_spout_restore_runtime(outputs: Vec<VideoOutputSummary>) -> EngineRuntime {
    let mut runtime = EngineRuntime::new(staged_show_artnet_loopback_output());
    seed_show_spout_v2_compositions(&mut runtime);
    runtime.video_outputs = outputs
        .into_iter()
        .map(|summary| RuntimeVideoOutput { summary })
        .collect();
    let started_at = Instant::now();
    runtime.video_output_fades = vec![
        RuntimeVideoOutputFade {
            output_id: 11,
            started_at,
            duration: Duration::from_secs(3),
            start_opacity: 0.2,
            target_opacity: 1.0,
        },
        RuntimeVideoOutputFade {
            output_id: 12,
            started_at,
            duration: Duration::from_secs(5),
            start_opacity: 1.0,
            target_opacity: 0.4,
        },
    ];
    runtime
}

#[test]
fn show_spout_disabled_restore_rejects_invalid_pairs_without_mutation() {
    let (background, foreground) = show_spout_v2_pair();
    let disabled_background = show_spout_disabled_variant(&background);
    let disabled_foreground = show_spout_disabled_variant(&foreground);
    let mut changed_background = disabled_background.clone();
    changed_background.label = "Changed Spout background".to_string();
    let generic = show_spout_output(99, "Generic Spout", 1);
    let cases = [
        (
            "mixed",
            vec![disabled_background.clone(), foreground.clone()],
        ),
        (
            "changed",
            vec![changed_background, disabled_foreground.clone()],
        ),
        ("missing", vec![disabled_background.clone()]),
        (
            "extra",
            vec![disabled_background.clone(), disabled_foreground.clone(), generic],
        ),
    ];

    for (case, outputs) in cases {
        let mut runtime = show_spout_restore_runtime(outputs);
        let before_outputs = runtime.build_snapshot(0).video.outputs;
        let before_fades = show_spout_restore_fade_shape(&runtime);

        let result = runtime.apply_show_spout_outputs_restore_disabled(
            &background,
            &foreground,
            &disabled_background,
            &disabled_foreground,
        );

        assert!(result.is_err(), "{case} pair must be rejected");
        assert_eq!(
            runtime.build_snapshot(0).video.outputs,
            before_outputs,
            "{case} rejection must preserve snapshot video outputs"
        );
        assert_eq!(
            show_spout_restore_fade_shape(&runtime),
            before_fades,
            "{case} rejection must preserve output-fade references"
        );
    }
}

#[test]
fn show_spout_disabled_restore_is_idempotent_in_place() {
    let mut runtime = {
        let (background, foreground) = show_spout_v2_pair();
        show_spout_restore_runtime(vec![background, foreground])
    };
    let (background, foreground) = show_spout_v2_pair();
    let disabled_background = show_spout_disabled_variant(&background);
    let disabled_foreground = show_spout_disabled_variant(&foreground);
    let before_fades = show_spout_restore_fade_shape(&runtime);

    runtime
        .apply_show_spout_outputs_restore_disabled(
            &background,
            &foreground,
            &disabled_background,
            &disabled_foreground,
        )
        .expect("the active exact pair restores to the disabled pair");
    let restored_outputs = runtime.build_snapshot(0).video.outputs;
    assert_eq!(
        restored_outputs,
        vec![disabled_background.clone(), disabled_foreground.clone()]
    );
    assert_eq!(show_spout_restore_fade_shape(&runtime), before_fades);

    runtime
        .apply_show_spout_outputs_restore_disabled(
            &background,
            &foreground,
            &disabled_background,
            &disabled_foreground,
        )
        .expect("a repeated restore of the already-disabled pair is idempotent");
    assert_eq!(runtime.build_snapshot(0).video.outputs, restored_outputs);
    assert_eq!(show_spout_restore_fade_shape(&runtime), before_fades);
}
