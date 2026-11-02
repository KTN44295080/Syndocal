//! Lane 1: layer groups and rotated layers stay on the resident route and
//! match the canonical CPU composition of the same snapshot.
use super::*;
use protocol::{
    Transform2D, VideoBlendMode, VideoEffectChainId, VideoEffectChainSummary, VideoEffectId,
    VideoEffectKind, VideoEffectScope, VideoEffectStageId, VideoLayerGroupId,
    VideoLayerGroupSummary,
};

/// Canonical and resident rotation both sample nearest-neighbour in `f32`;
/// a fused multiply-add on the GPU may move a sample that lands exactly on
/// a pixel boundary to its neighbour. Everything else is exact to 1 LSB.
const MAX_BOUNDARY_PIXELS: usize = 8;

fn group(layer_ids: &[u64], opacity: f32, blend_mode: VideoBlendMode) -> VideoLayerGroupSummary {
    VideoLayerGroupSummary {
        id: VideoLayerGroupId(9),
        label: "resident group".into(),
        composition_id: 70,
        layer_ids: layer_ids.to_vec(),
        enabled: true,
        opacity,
        blend_mode,
        solo: false,
        bypassed: false,
        transform: None,
    }
}

fn rotated(rotation_deg: f32) -> Transform2D {
    Transform2D {
        x: 0.05,
        y: -0.03,
        scale_x: 0.8,
        scale_y: 0.9,
        rotation_deg,
        crop_left: 0.1,
        ..Transform2D::default()
    }
}

/// The canonical CPU composition of `snapshot` at its tick position.
fn canonical(snapshot: &VideoSnapshot) -> Vec<u8> {
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider {
            pattern: true,
            ..Provider::default()
        },
    );
    let plan = crate::build_video_output_render_plan(snapshot, OUTPUT).unwrap();
    renderer
        .prepare_output_artistic_frame_with_effects(
            snapshot,
            VideoEffectRenderContext {
                clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                project_render_epoch: EPOCH,
                audio_frame: None,
            },
            &plan,
            EXTENT.0,
            EXTENT.1,
        )
        .unwrap()
        .data
}

/// Ticks `snapshot` on a synchronous resident session and returns the path of
/// every tick and the last resident pixels.
fn resident(snapshot: &mut VideoSnapshot, ticks: u64) -> (Vec<Path>, Vec<u8>) {
    let mut session = session_with_compositor(EXTENT);
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider {
            pattern: true,
            ..Provider::default()
        },
    );
    let mut paths = Vec::new();
    let mut pixels = Vec::new();
    for index in 0..ticks {
        for layer in &mut snapshot.layers {
            layer.state.position_ms = index * 16;
        }
        // Synchronous: this tick's own pixels reach LastValid.
        session.ring.disarm();
        let frame = prepare_with(&mut session, &mut renderer, snapshot, EXTENT, &Permit)
            .unwrap_or_else(|error| panic!("grouped tick {index} failed after {paths:?}: {error}"));
        if let ResidentDisplayContent::Resident(ready) = &frame.content {
            pixels = session
                .cache
                .last_valid(&ready.request.key)
                .unwrap()
                .data
                .clone();
        }
        paths.push(path_of(frame));
    }
    stop_and_reap(&mut session, &mut renderer);
    (paths, pixels)
}

fn assert_matches_canonical(case: &str, mut snapshot: VideoSnapshot) {
    let ticks = 6;
    let (paths, actual) = resident(&mut snapshot, ticks);
    assert!(
        paths.iter().all(|path| *path == Path::Resident),
        "{case}: every tick must stay resident: {paths:?}"
    );
    let expected = canonical(&snapshot);
    assert_eq!(actual.len(), expected.len(), "{case}");
    let mut boundary = 0;
    for (index, (actual, expected)) in actual
        .chunks_exact(4)
        .zip(expected.chunks_exact(4))
        .enumerate()
    {
        if actual
            .iter()
            .zip(expected)
            .any(|(actual, expected)| actual.abs_diff(*expected) > 1)
        {
            boundary += 1;
            assert!(
                boundary <= MAX_BOUNDARY_PIXELS,
                "{case}: pixel {index} {actual:?} != canonical {expected:?}"
            );
        }
    }
    println!("{case}: {boundary} boundary pixels of {}", actual.len() / 4);
}

#[test]
fn native_gpu_group_composition_matches_cpu_parity() {
    assert_matches_canonical("ungrouped control", scene(3, EXTENT));
    let mut transformed = scene(3, EXTENT);
    let mut screen = group(&[1, 2], 0.65, VideoBlendMode::Screen);
    screen.transform = Some(rotated(25.0));
    transformed.layer_groups.push(screen);
    assert_matches_canonical("2-layer Screen group rotated 25deg", transformed);

    let modes = [
        VideoBlendMode::Normal,
        VideoBlendMode::Add,
        VideoBlendMode::Multiply,
        VideoBlendMode::Screen,
        VideoBlendMode::Overlay,
        VideoBlendMode::SoftLight,
        VideoBlendMode::Difference,
    ];
    for mode in modes {
        let mut snapshot = scene(3, EXTENT);
        snapshot.layers[1].blend_mode = VideoBlendMode::Add;
        snapshot
            .layer_groups
            .push(group(&[2, 3], 0.55, mode.clone()));
        assert_matches_canonical(&format!("unrotated 2-layer {mode:?} group"), snapshot);
    }

    let mut axis = scene(2, EXTENT);
    let mut moved = group(&[1, 2], 1.0, VideoBlendMode::Normal);
    moved.transform = Some(Transform2D {
        rotation_deg: 0.0,
        ..rotated(0.0)
    });
    axis.layer_groups.push(moved);
    assert_matches_canonical("axis-aligned group transform", axis);

    let mut singleton = scene(2, EXTENT);
    singleton
        .layer_groups
        .push(group(&[2], 0.4, VideoBlendMode::Multiply));
    assert_matches_canonical("singleton Normal child folds", singleton);

    let mut layer = scene(2, EXTENT);
    layer.layers[1].state.transform = rotated(-40.0);
    assert_matches_canonical("rotated layer without a group", layer);
}

#[test]
fn resident_layer_chains_admits_isolated_group_and_rotated_transform() {
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let context = || VideoEffectRenderContext {
        clip_runtime: &clips,
        project_render_epoch: EPOCH,
        audio_frame: None,
    };
    let admitted = |snapshot: &VideoSnapshot| {
        let plan = crate::build_video_output_render_plan(snapshot, OUTPUT).unwrap();
        super::super::prepare::resident_layer_chains(
            snapshot,
            context(),
            &VideoLayerTransitionRuntimeSnapshot::default(),
            &plan,
        )
        .is_some()
    };
    let mut snapshot = scene(3, EXTENT);
    snapshot.layers[0].state.transform = rotated(30.0);
    let mut grouped = group(&[1, 2], 0.65, VideoBlendMode::Screen);
    grouped.transform = Some(rotated(25.0));
    snapshot.layer_groups.push(grouped);
    assert!(admitted(&snapshot), "rotated layer in a transformed group");

    // Group FX keeps the canonical route until it runs on resident textures.
    let effect = crate::builtin_isf_effect("posterize").unwrap().unwrap();
    snapshot.effect_chains.push(VideoEffectChainSummary {
        id: VideoEffectChainId(1),
        scope: VideoEffectScope::Group {
            group_id: VideoLayerGroupId(9),
        },
        bypassed: false,
        stages: vec![protocol::VideoEffectStageSummary {
            id: VideoEffectStageId(1),
            enabled: true,
            label: "Posterize group".into(),
            effect: protocol::VideoEffectSummary {
                id: VideoEffectId(1),
                kind: VideoEffectKind::Isf { effect },
            },
        }],
    });
    assert!(!admitted(&snapshot), "executing group FX");
    snapshot.layer_groups[0].bypassed = true;
    assert!(admitted(&snapshot), "a bypassed group's FX does not run");

    // The executing group-FX case declines to the canonical route on a tick.
    snapshot.layer_groups[0].bypassed = false;
    let (paths, _) = resident(&mut snapshot, 2);
    assert!(paths.iter().all(|path| *path == Path::Cpu), "{paths:?}");
}
