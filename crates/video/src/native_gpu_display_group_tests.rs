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

/// Replaces group `group_id`'s FX chain with `effects`, in order.
fn group_fx(
    snapshot: &mut VideoSnapshot,
    group_id: u64,
    effects: &[protocol::VideoIsfEffectSummary],
) {
    let scope = VideoEffectScope::Group {
        group_id: VideoLayerGroupId(group_id),
    };
    snapshot.effect_chains.retain(|chain| chain.scope != scope);
    snapshot.effect_chains.push(VideoEffectChainSummary {
        id: VideoEffectChainId(100 + group_id),
        scope,
        bypassed: false,
        stages: effects
            .iter()
            .enumerate()
            .map(|(index, effect)| protocol::VideoEffectStageSummary {
                id: VideoEffectStageId(group_id * 10 + index as u64 + 1),
                enabled: true,
                label: effect.label.clone(),
                effect: protocol::VideoEffectSummary {
                    id: VideoEffectId(group_id * 10 + index as u64 + 1),
                    kind: VideoEffectKind::Isf {
                        effect: effect.clone(),
                    },
                },
            })
            .collect(),
    });
}

fn builtin(id: &str) -> protocol::VideoIsfEffectSummary {
    crate::builtin_isf_effect(id).unwrap().unwrap()
}

/// Stateless, but its pixels depend on the canonical logical clock.
fn frame_index_tint() -> protocol::VideoIsfEffectSummary {
    const TINT: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}]}*/
void main(){vec4 c=IMG_THIS_NORM_PIXEL(inputImage);gl_FragColor=vec4(c.rgb*(0.3+0.1*mod(float(FRAMEINDEX),7.0)),c.a);}"#;
    let shader = crate::prepare_isf_shader(TINT).unwrap();
    crate::isf_effect_from_prepared("frame index tint".into(), TINT, None, &shader)
}

/// A stateful (persistent feedback) chain stays on the canonical route.
fn feedback() -> protocol::VideoIsfEffectSummary {
    const FEEDBACK: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"trail","PERSISTENT":true},{}]}*/
void main(){if(PASSINDEX==0){gl_FragColor=mix(IMG_THIS_NORM_PIXEL(inputImage),IMG_NORM_PIXEL(trail,isf_FragNormCoord),0.5);}else{gl_FragColor=IMG_NORM_PIXEL(trail,isf_FragNormCoord);}}"#;
    let shader = crate::prepare_isf_shader(FEEDBACK).unwrap();
    crate::isf_effect_from_prepared("group feedback".into(), FEEDBACK, None, &shader)
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

fn assert_matches_canonical(case: &str, snapshot: VideoSnapshot) {
    assert_matches_canonical_after(case, snapshot, 6);
}

/// A fresh canonical renderer has consumed no logical clock, so a clock-
/// dependent case compares against the resident session's first tick.
fn assert_matches_canonical_after(case: &str, mut snapshot: VideoSnapshot, ticks: u64) {
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

    let mut two = scene(3, EXTENT);
    let mut first = group(&[1, 2], 0.8, VideoBlendMode::Overlay);
    first.transform = Some(rotated(25.0));
    let mut second = group(&[3], 0.6, VideoBlendMode::Difference);
    second.id = VideoLayerGroupId(10);
    second.transform = Some(rotated(-60.0));
    two.layer_groups.extend([first, second]);
    assert_matches_canonical("two transformed groups share targets", two);

    let mut layer = scene(2, EXTENT);
    layer.layers[1].state.transform = rotated(-40.0);
    assert_matches_canonical("rotated layer without a group", layer);
}

#[test]
fn native_gpu_group_fx_matches_cpu_parity() {
    let mut fx = scene(3, EXTENT);
    let mut screen = group(&[1, 2], 0.65, VideoBlendMode::Screen);
    screen.transform = Some(rotated(25.0));
    fx.layer_groups.push(screen);
    let plain = canonical(&fx);
    group_fx(&mut fx, 9, &[builtin("posterize")]);
    assert_ne!(canonical(&fx), plain, "the group FX must change the image");
    assert_matches_canonical("posterize FX on a rotated Screen group", fx);

    // An otherwise organizational group isolates its members for its FX.
    let mut pass_through = scene(3, EXTENT);
    pass_through
        .layer_groups
        .push(group(&[2, 3], 1.0, VideoBlendMode::Normal));
    group_fx(
        &mut pass_through,
        9,
        &[
            builtin("invert"),
            builtin("monochrome"),
            builtin("rgb-split"),
        ],
    );
    assert_matches_canonical("three-stage FX on a pass-through group", pass_through);

    let mut singleton = scene(2, EXTENT);
    singleton
        .layer_groups
        .push(group(&[2], 0.4, VideoBlendMode::Multiply));
    group_fx(&mut singleton, 9, &[builtin("vignette")]);
    assert_matches_canonical("vignette FX on a singleton group", singleton);

    let mut two = scene(3, EXTENT);
    let first = group(&[1, 2], 0.8, VideoBlendMode::Overlay);
    let mut second = group(&[3], 0.6, VideoBlendMode::Difference);
    second.id = VideoLayerGroupId(10);
    second.transform = Some(rotated(-60.0));
    two.layer_groups.extend([first, second]);
    group_fx(&mut two, 9, &[builtin("rgb-split"), builtin("posterize")]);
    group_fx(&mut two, 10, &[builtin("invert")]);
    assert_matches_canonical("two FX groups share one target set", two);
}

#[test]
fn native_gpu_group_fx_consumes_the_canonical_frame_index_clock() {
    let mut snapshot = scene(3, EXTENT);
    let mut second = group(&[3], 0.6, VideoBlendMode::Difference);
    second.id = VideoLayerGroupId(10);
    snapshot
        .layer_groups
        .extend([group(&[1, 2], 0.8, VideoBlendMode::Screen), second]);
    group_fx(&mut snapshot, 9, &[frame_index_tint()]);
    group_fx(&mut snapshot, 10, &[frame_index_tint()]);
    // Two groups read successive FRAMEINDEX values after the layer chains.
    assert_matches_canonical_after("FRAMEINDEX FX on two groups", snapshot.clone(), 1);

    // Each route advances the clock by the same amount per tick, so switching
    // routes keeps one progression.
    let clock = |renderer: &VideoPreviewRenderer<Provider>| match renderer.isf_runtime.as_ref() {
        Some(Ok(runtime)) => runtime.native_frame_index(),
        _ => panic!("ISF runtime is installed"),
    };
    let provider = || Provider {
        pattern: true,
        ..Provider::default()
    };
    let mut session = session_with_compositor(EXTENT);
    let mut resident =
        VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider());
    let mut canonical =
        VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider());
    let plan = crate::build_video_output_render_plan(&snapshot, OUTPUT).unwrap();
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let mut steps = Vec::new();
    for index in 0..3 {
        for layer in &mut snapshot.layers {
            layer.state.position_ms = index * 16;
        }
        session.ring.disarm();
        let frame = prepare_with(&mut session, &mut resident, &snapshot, EXTENT, &Permit).unwrap();
        assert_eq!(path_of(frame), Path::Resident, "tick {index}");
        canonical
            .prepare_output_artistic_frame_with_effects(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &clips,
                    project_render_epoch: EPOCH,
                    audio_frame: None,
                },
                &plan,
                EXTENT.0,
                EXTENT.1,
            )
            .unwrap();
        steps.push((clock(&resident), clock(&canonical)));
    }
    stop_and_reap(&mut session, &mut resident);
    assert!(
        steps
            .iter()
            .all(|(resident, canonical)| resident == canonical),
        "resident vs canonical clock after each tick: {steps:?}"
    );
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

    // Stateless group FX runs on the resident compositor (formerly every
    // executing group FX declined here, before resident group FX existed).
    group_fx(&mut snapshot, 9, &[builtin("posterize")]);
    assert!(admitted(&snapshot), "stateless group FX");
    let (paths, _) = resident(&mut snapshot, 2);
    assert!(
        paths.iter().all(|path| *path == Path::Resident),
        "{paths:?}"
    );

    // Stateful group FX still declines to the canonical route on a tick.
    group_fx(&mut snapshot, 9, &[builtin("posterize"), feedback()]);
    let (paths, _) = resident(&mut snapshot, 2);
    assert!(paths.iter().all(|path| *path == Path::Cpu), "{paths:?}");
    snapshot.layer_groups[0].bypassed = true;
    let (paths, _) = resident(&mut snapshot, 2);
    assert!(
        paths.iter().all(|path| *path == Path::Resident),
        "a bypassed group's FX does not run: {paths:?}"
    );
}

/// QA diagnostic at the C4 physical extent: three layers in two rotated,
/// translucent groups stay resident on the pipelined route and
/// within the managed budget. Run explicitly with --ignored.
#[test]
#[ignore = "QA diagnostic: allocates 2560x1600 frames for three layers"]
fn native_gpu_group_composition_budget_trace_at_c4_extent() {
    const C4: (u32, u32) = (2560, 1600);
    let mut session = session_with_compositor(C4);
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let mut snapshot = scene(3, C4);
    let mut screen = group(&[1, 2], 0.65, VideoBlendMode::Screen);
    screen.transform = Some(rotated(25.0));
    let mut second = group(&[3], 0.6, VideoBlendMode::Difference);
    second.id = VideoLayerGroupId(10);
    second.transform = Some(rotated(-60.0));
    snapshot.layer_groups.extend([screen, second]);
    let mut high = 0;
    for index in 0..240 {
        for layer in &mut snapshot.layers {
            layer.state.position_ms = index * 16;
        }
        let path = tick(&mut session, &mut renderer, &snapshot, C4);
        let (bytes, line) = budget_line(&session, C4);
        high = high.max(bytes);
        println!("group c4 tick={index} path={path:?} {line}");
        assert!(
            matches!(path, Path::Resident | Path::InFlight),
            "tick {index} left the resident route: {path:?}"
        );
    }
    println!(
        "group c4 live high water {:.1} MiB",
        high as f64 / 1048576.0
    );
    // Any number of groups shares one isolated and one transform target: the
    // ungrouped 29P steady state plus two frames.
    let frame = u64::from(C4.0) * u64::from(C4.1) * 4;
    assert!(
        high < 31 * frame + frame / 10,
        "two transformed groups must share their targets"
    );
    stop_and_reap(&mut session, &mut renderer);
}

/// QA diagnostic at the C4 physical extent: 60 resident ticks, a 60-tick
/// Follow crossfade (canonical CPU, both sides alike) and 60 resident ticks
/// again, for an ungrouped control, transformed groups, and transformed groups
/// with group FX. Run explicitly with --ignored.
#[test]
#[ignore = "QA diagnostic: allocates 2560x1600 frames for three layers"]
fn native_gpu_group_fx_budget_trace_across_follow_crossfade_at_c4_extent() {
    const C4: (u32, u32) = (2560, 1600);
    let plain = scene(3, C4);
    let mut grouped = plain.clone();
    let mut screen = group(&[1, 2], 0.65, VideoBlendMode::Screen);
    screen.transform = Some(rotated(25.0));
    let mut second = group(&[3], 0.6, VideoBlendMode::Difference);
    second.id = VideoLayerGroupId(10);
    second.transform = Some(rotated(-60.0));
    grouped.layer_groups.extend([screen, second]);
    let mut fx = grouped.clone();
    group_fx(&mut fx, 9, &[builtin("rgb-split"), builtin("posterize")]);
    group_fx(&mut fx, 10, &[builtin("invert")]);
    let limit = crate::native_gpu_budget::NATIVE_MANAGED_LIMIT;
    for (case, mut snapshot) in [
        ("ungrouped", plain),
        ("transformed groups", grouped),
        ("transformed groups with FX", fx),
    ] {
        let mut session = session_with_compositor(C4);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            Provider::default(),
        );
        let clips = protocol::VideoClipRuntimeSnapshot::default();
        let mut high = [0u64; 3];
        // Per phase: resident (or in flight), CPU, and failed ticks.
        let mut paths = [[0usize; 3]; 3];
        for index in 0..180u64 {
            for layer in &mut snapshot.layers {
                layer.state.position_ms = index * 16;
            }
            let phase = (index / 60) as usize;
            let frame = if phase == 1 {
                session.prepare_follow_output(
                    &mut renderer,
                    &snapshot,
                    &snapshot,
                    VideoEffectRenderContext {
                        clip_runtime: &clips,
                        project_render_epoch: EPOCH,
                        audio_frame: None,
                    },
                    &VideoLayerTransitionRuntimeSnapshot::default(),
                    protocol::VideoClipTakeKind::Crossfade,
                    protocol::VideoLayerTransitionCurve::Linear,
                    ((index - 60) * 1000 / 60) as u16,
                    protocol::TimelineId(55),
                    1,
                    None,
                    C4,
                    &|| false,
                    &Permit,
                )
            } else {
                prepare_with(&mut session, &mut renderer, &snapshot, C4, &Permit)
            };
            let (bytes, line) = budget_line(&session, C4);
            high[phase] = high[phase].max(bytes);
            let path = match frame {
                Ok(frame) => format!("{:?}", path_of(frame)),
                Err(error) => format!("Err({error})"),
            };
            println!("{case} tick={index} phase={phase} path={path} {line}");
            paths[phase][match path.as_str() {
                "Resident" | "InFlight" => 0,
                "Cpu" => 1,
                _ => 2,
            }] += 1;
        }
        stop_and_reap(&mut session, &mut renderer);
        let mib = high.map(|bytes| (bytes as f64 / 1048576.0 * 10.0).round() / 10.0);
        println!("{case}: [resident, cpu, failed] per phase {paths:?}, high water MiB {mib:?}");
        assert!(high.iter().all(|bytes| *bytes <= limit), "{case}: {high:?}");
        // Recorded, not asserted (qa/VJ gap Lane 1 result): at this extent the
        // canonical Follow cannot fit beside its CPU history queues even
        // ungrouped, and recovery after it depends on the steady headroom.
        assert_eq!(paths[0], [60, 0, 0], "{case}: steady ticks stay resident");
    }
}
