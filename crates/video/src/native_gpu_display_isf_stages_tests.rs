//! The actual Display facade, including full history and CPU handover. No
//! window, physical output, or per-layer CPU readback in resident execution.
use super::*;

const MULTIPASS: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"half","WIDTH":"$WIDTH * 0.5","HEIGHT":"$HEIGHT * 0.5"},{}]}*/
void main(){if(PASSINDEX==0){gl_FragColor=IMG_THIS_NORM_PIXEL(inputImage);}else{gl_FragColor=IMG_NORM_PIXEL(half,isf_FragNormCoord);}}"#;
const FEEDBACK: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"trail","PERSISTENT":true},{}]}*/
void main(){if(PASSINDEX==0){gl_FragColor=mix(IMG_THIS_NORM_PIXEL(inputImage),IMG_NORM_PIXEL(trail,isf_FragNormCoord),0.5);}else{gl_FragColor=IMG_NORM_PIXEL(trail,isf_FragNormCoord);}}"#;

fn set_effect(snapshot: &mut VideoSnapshot, source: &str) {
    let shader = crate::prepare_isf_shader(source).unwrap();
    for layer in &mut snapshot.layers {
        layer.isf_effect = Some(crate::isf_effect_from_prepared(
            "resident stages".into(),
            source,
            None,
            &shader,
        ));
    }
}

fn render_context<'a>(
    clips: &'a protocol::VideoClipRuntimeSnapshot,
) -> VideoEffectRenderContext<'a> {
    VideoEffectRenderContext {
        clip_runtime: clips,
        project_render_epoch: EPOCH,
        audio_frame: None,
    }
}

#[test]
fn native_gpu_output_display_extended_isf_matches_canonical_and_hands_over_its_history() {
    for (name, source) in [
        ("multipass custom extent", MULTIPASS),
        ("persistent feedback", FEEDBACK),
    ] {
        let mut snapshot = scene(2, EXTENT);
        set_effect(&mut snapshot, source);
        let mut session = session_with_compositor(EXTENT);
        let provider = || Provider {
            pattern: true,
            ..Provider::default()
        };
        let mut resident =
            VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider());
        let mut canonical =
            VideoPreviewRenderer::with_frame_provider(VideoRuntimeConfig::default(), provider());
        let clips = protocol::VideoClipRuntimeSnapshot::default();
        for tick_index in 0..12 {
            for layer in &mut snapshot.layers {
                layer.state.position_ms = tick_index * 16;
            }
            session.ring.disarm();
            let frame = prepare_with(&mut session, &mut resident, &snapshot, EXTENT, &Permit)
                .unwrap_or_else(|error| panic!("{name} tick {tick_index}: {error}"));
            let ResidentDisplayContent::Resident(ready) = &frame.content else {
                panic!(
                    "{name} tick {tick_index} must stay resident, got {:?}",
                    path_of(frame)
                );
            };
            let actual = &session.cache.last_valid(&ready.request.key).unwrap().data;
            let plan = crate::build_video_output_render_plan(&snapshot, OUTPUT).unwrap();
            let expected = canonical
                .prepare_output_artistic_frame_with_effects(
                    &snapshot,
                    render_context(&clips),
                    &plan,
                    EXTENT.0,
                    EXTENT.1,
                )
                .unwrap();
            assert_eq!(actual.len(), expected.data.len());
            assert!(
                actual
                    .iter()
                    .zip(&expected.data)
                    .all(|(a, b)| a.abs_diff(*b) <= 1),
                "{name} tick {tick_index}: resident pixels differ from canonical"
            );
            assert!(
                session.owner.budget_snapshot().managed_bytes()
                    <= crate::native_gpu_budget::NATIVE_MANAGED_LIMIT
            );
        }
        let Some(ResidentHistoryState::Active(active)) = &session.history else {
            panic!("{name}: history absent")
        };
        assert_eq!(active.inputs.len(), 2);
        // Stateful results are not replayable, so each input keeps only its
        // resident entries (two), bounding memory like an ordinary input.
        for index in 0..2 {
            assert_eq!(active.session.history(index).unwrap().metadata().len(), 2);
        }
        // A real handover consumes every surviving validated layer. Any attempt
        // to replay stateful history would fail this barrier.
        session
            .handover_all_history_to_cpu(&mut resident, &|| false)
            .unwrap();
        assert!(
            session.history.is_none(),
            "{name}: complete handover publishes atomically"
        );
        stop_and_reap(&mut session, &mut resident);
    }
}

/// QA diagnostic at the C4 physical extent: extended stages keep their exact
/// results resident for the whole history, so report whether one or three
/// such layers still fit the managed ledger on the pipelined route.
/// Run explicitly with --ignored.
#[test]
#[ignore = "QA diagnostic: allocates 2560x1600 frames for three layers"]
fn native_gpu_output_display_extended_isf_budget_trace_at_c4_extent() {
    const C4: (u32, u32) = (2560, 1600);
    for (name, source, extended_layers) in [
        ("feedback x1", FEEDBACK, 1),
        ("multipass x1", MULTIPASS, 1),
        ("feedback x3", FEEDBACK, 3),
    ] {
        let mut session = session_with_compositor(C4);
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            Provider::default(),
        );
        let mut snapshot = scene(3, C4);
        let ordinary = snapshot.layers[0].isf_effect.clone();
        set_effect(&mut snapshot, source);
        for layer in snapshot.layers.iter_mut().skip(extended_layers) {
            layer.isf_effect = ordinary.clone();
        }
        let mut paths = Vec::new();
        let mut high = 0;
        for index in 0..240 {
            for layer in &mut snapshot.layers {
                layer.state.position_ms = index * 16;
            }
            // Over the ledger the tick is deferred (an error), not a fault.
            paths.push(
                prepare_with(&mut session, &mut renderer, &snapshot, C4, &Permit)
                    .map_or(None, |frame| Some(path_of(frame))),
            );
            let (bytes, line) = budget_line(&session, C4);
            high = high.max(bytes);
            if index < 12 || index % 60 == 0 {
                println!(
                    "{name} tick={index} path={:?} {line}",
                    paths[index as usize]
                );
            }
        }
        let resident = paths
            .iter()
            .filter(|path| matches!(path, Some(Path::Resident | Path::InFlight)))
            .count();
        let deferred = paths.iter().filter(|path| path.is_none()).count();
        println!(
            "{name}: resident {resident}/240, deferred {deferred}, live high water {:.1} MiB",
            high as f64 / 1048576.0
        );
        stop_and_reap(&mut session, &mut renderer);
    }
}
