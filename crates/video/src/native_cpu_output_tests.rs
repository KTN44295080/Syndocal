use super::*;
use crate::isf_native_gpu::continuation::NativeLayerPreparation;
use crate::isf_native_gpu::tests::{gpu, jobs, request, shader, stop, Fixture, IDENTITY};
use crate::isf_native_gpu::NativeBatchRestrictions;

#[derive(Default)]
struct Provider {
    decodes: usize,
    fail: bool,
}
impl VideoFrameProvider for Provider {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}
    fn frame_for_layer(
        &mut self,
        layer: &VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.decodes += 1;
        if self.fail {
            return Err(VideoFrameProviderError::MissingStillImagePath {
                layer_id: layer.id,
                label: layer.label.clone(),
            });
        }
        Ok(VideoFrame {
            layer_id: layer.id,
            width,
            height,
            pts_ms: layer.state.position_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: [10, 20, 30, 255].repeat(width as usize * height as usize),
        })
    }
}

fn snapshot(count: u64) -> VideoSnapshot {
    let layers = (1..=count)
        .map(|id| VideoLayerSummary {
            id,
            label: format!("CPU layer {id}"),
            source: VideoSourceSummary {
                kind: VideoSourceKind::File,
                path: Some(format!("{id}.mp4")),
                name: None,
                codec: None,
                metadata: None,
            },
            media_asset_id: None,
            blend_mode: VideoBlendMode::Normal,
            state: VideoLayerState::default(),
            isf_effect: None,
            clip_slots: Vec::new(),
            default_clip_slot_id: None,
        })
        .collect();
    VideoSnapshot {
        layers,
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        compositions: vec![CompositionSummary {
            id: 70,
            label: "CPU".into(),
            layer_ids: (1..=count).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![101],
        }],
        outputs: vec![VideoOutputSummary {
            id: 101,
            label: "CPU Display".into(),
            kind: VideoOutputKind::Display,
            enabled: true,
            composition_id: 70,
            fullscreen: false,
            monitor_id: Some(0),
            monitor_identity: None,
            width: 3,
            height: 2,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: VideoOutputMapping::default(),
        }],
        mapping_presets: Vec::new(),
        auto_vj: protocol::AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

fn renderer(runtime: IsfGpuRuntime) -> VideoPreviewRenderer<Provider> {
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    renderer.isf_runtime = Some(Ok(runtime));
    renderer.isf_render_namespace = (7, Some(101), false);
    renderer.set_isf_date_override(Some([2026.0, 10.0, 27.0, 42.0]));
    renderer
}

fn chain(
    id: u64,
    scope: VideoEffectScope,
    operation: &str,
) -> (VideoEffectChainSummary, VideoIsfEffectSummary) {
    let source = format!("/*{{\"INPUTS\":[{{\"NAME\":\"inputImage\",\"TYPE\":\"image\"}}]}}*/\nvoid main() {{ vec4 c=IMG_THIS_PIXEL(inputImage); gl_FragColor=vec4({operation},c.gba); }}");
    let prepared = prepare_isf_shader(&source).unwrap();
    let effect = isf_effect_from_prepared(format!("scope {id}"), source.as_str(), None, &prepared);
    (
        VideoEffectChainSummary {
            id: VideoEffectChainId(id),
            scope,
            bypassed: false,
            stages: vec![protocol::VideoEffectStageSummary {
                id: VideoEffectStageId(id * 100),
                enabled: true,
                label: format!("scope {id}"),
                effect: protocol::VideoEffectSummary {
                    id: VideoEffectId(id * 1000),
                    kind: VideoEffectKind::Isf {
                        effect: effect.clone(),
                    },
                },
            }],
        },
        effect,
    )
}

#[test]
fn native_gpu_output_cpu_canonical_clip_layer_group_composition_output_order_and_charges() {
    let Fixture { owner, runtime } = gpu();
    let context_id = owner.context().id();
    let mut renderer = renderer(runtime);
    let mut snapshot = snapshot(1);
    snapshot
        .layer_groups
        .push(protocol::VideoLayerGroupSummary {
            id: protocol::VideoLayerGroupId(7),
            label: "group".into(),
            composition_id: 70,
            layer_ids: vec![1],
            ..protocol::VideoLayerGroupSummary::default()
        });
    let specs = [
        (
            VideoEffectScope::Clip {
                layer_id: 1,
                slot_id: VideoClipSlotId(2),
            },
            "c.r+1.0/255.0",
        ),
        (VideoEffectScope::Layer { layer_id: 1 }, "c.r*2.0"),
        (
            VideoEffectScope::Group {
                group_id: protocol::VideoLayerGroupId(7),
            },
            "c.r+3.0/255.0",
        ),
        (
            VideoEffectScope::Composition { composition_id: 70 },
            "c.r*3.0",
        ),
        (VideoEffectScope::Output { output_id: 101 }, "c.r+5.0/255.0"),
    ];
    for (index, (scope, op)) in specs.into_iter().enumerate() {
        let (chain, effect) = chain(index as u64 + 1, scope.clone(), op);
        if scope == (VideoEffectScope::Layer { layer_id: 1 }) {
            snapshot.layers[0].isf_effect = Some(effect);
        }
        snapshot.effect_chains.push(chain);
    }
    let clips = VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 1,
            active_slot_id: Some(VideoClipSlotId(2)),
            ..Default::default()
        }],
        ..Default::default()
    };
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (mut owner, output) = renderer.prepare_native_cpu_output(
        owner,
        request(1),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &clips,
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &plan,
        3,
        2,
        &|| false,
    );
    let output = output.unwrap();
    let VideoOutputArtisticPayload::Frame(frame) = &output.result.payload else {
        panic!("artistic");
    };
    assert!(frame
        .data
        .chunks_exact(4)
        .all(|pixel| pixel == [80, 20, 30, 255]));
    assert_eq!(
        output.result.evidence.freshness,
        VideoOutputRenderFreshness::Fresh
    );
    assert_eq!(renderer.frame_provider.decodes, 1);
    assert_eq!(
        renderer.isf_shader_cache_clock, 5,
        "each canonical scope prepared once"
    );
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        5
    );
    assert_eq!(owner.context().id(), context_id);
    assert_eq!(owner.budget_snapshot().for_kind(ChargeKind::ActiveCpuInput).high_water_bytes, 24,
        "the post-FX selected frame must alias its CpuSnapshot owner instead of being charged as another raw input");
    assert!(output.charge.as_ref().unwrap().capacity() >= frame.data.capacity() as u64);
    let queue = &renderer.runtime.queues[0];
    assert!(queue.native_cpu_owners[0].is_some());
    assert!(Arc::ptr_eq(
        &queue.native_cpu_owners[0].as_ref().unwrap().frame,
        queue.slots[0].as_ref().unwrap()
    ));
    assert!(renderer.output_last_valid_frames[0]
        .native_cpu_charge
        .is_some());
    drop(output);
    drop(renderer);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_last_valid_blackout_and_cancel_are_canonical() {
    let Fixture { owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let mut snapshot = snapshot(1);
    let clips = VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        clip_runtime: &clips,
        project_render_epoch: 7,
        audio_frame: None,
    };
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (owner, first) = renderer.prepare_native_cpu_output(
        owner,
        request(1),
        &snapshot,
        context,
        &transitions,
        &plan,
        3,
        2,
        &|| false,
    );
    let first = first.unwrap();
    renderer.frame_provider.fail = true;
    let (owner, fallback) = renderer.prepare_native_cpu_output(
        owner,
        request(2),
        &snapshot,
        context,
        &transitions,
        &plan,
        3,
        2,
        &|| false,
    );
    let fallback = fallback.unwrap();
    assert_eq!(
        fallback.result.evidence.freshness,
        VideoOutputRenderFreshness::LastValid
    );
    assert_eq!(fallback.result.payload, first.result.payload);
    assert!(fallback.charge.is_some());
    snapshot.outputs[0].blackout = true;
    let blackout = build_video_output_render_plan(&snapshot, 101).unwrap();
    let before = renderer.frame_provider.decodes;
    let (owner, result) = renderer.prepare_native_cpu_output(
        owner,
        request(3),
        &snapshot,
        context,
        &transitions,
        &blackout,
        3,
        2,
        &|| false,
    );
    let result = result.unwrap();
    assert_eq!(
        result.result.payload,
        VideoOutputArtisticPayload::HardBlackout
    );
    assert!(result.charge.is_none());
    assert!(renderer.output_last_valid_frames.is_empty());
    assert_eq!(renderer.frame_provider.decodes, before);
    let (mut owner, result) = renderer.prepare_native_cpu_output(
        owner,
        request(4),
        &snapshot,
        context,
        &transitions,
        &plan,
        3,
        2,
        &|| true,
    );
    assert!(matches!(result, Err(VideoPreviewError::Cancelled)));
    assert_eq!(renderer.frame_provider.decodes, before);
    drop(first);
    drop(fallback);
    drop(renderer);
    stop(&mut owner);
}

fn late_results_case(output_extent: (u32, u32)) {
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let identity = shader(IDENTITY);
    let input = jobs(3, 2, &[identity]).map(|job| NativeLayerPreparation {
        input: job.input,
        chain: job.chain.source.into_owned(),
        entry: job.entry,
        source_generation: job.source_generation,
        content_sequence: job.content_sequence,
    });
    let continuation = renderer
        .prepare_native_layer_continuation(
            request(1),
            input,
            NativeBatchRestrictions::default(),
            Some(&|| false),
        )
        .unwrap()
        .into_cpu()
        .unwrap();
    let quote = owner
        .quote(continuation.allocation_plan().unwrap())
        .unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let results = continuation
        .run(&mut owner, reservation, &|| false)
        .unwrap();
    let pointers = results
        .outcomes()
        .iter()
        .map(|outcome| outcome.frame().data.as_ptr())
        .collect::<Vec<_>>();
    let source_frames = results
        .outcomes()
        .iter()
        .map(|outcome| Arc::clone(outcome.frame()))
        .collect::<Vec<_>>();
    let preparations = renderer.isf_shader_cache_clock;
    let clocks = renderer
        .isf_runtime
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_frame_index();
    let snapshot = snapshot(3);
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let expected = composite_frames_rgba8(
        &plan.composition,
        &source_frames,
        output_extent.0,
        output_extent.1,
    )
    .unwrap();
    let (mut owner, output) = renderer.finish_native_cpu_layers(
        owner,
        request(1),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &plan,
        output_extent.0,
        output_extent.1,
        &|| false,
        results,
    );
    let output = output.unwrap();
    assert_eq!(renderer.frame_provider.decodes, 0);
    assert_eq!(renderer.isf_shader_cache_clock, preparations);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        clocks
    );
    for (queue, pointer) in renderer.runtime.queues.iter().zip(pointers) {
        assert_eq!(queue.slots[0].as_ref().unwrap().data.as_ptr(), pointer);
        assert!(queue.native_cpu_owners[0].is_some());
    }
    assert_eq!(
        output.result.evidence.freshness,
        VideoOutputRenderFreshness::Fresh
    );
    assert_eq!(
        output.result.payload,
        VideoOutputArtisticPayload::Frame(expected)
    );
    drop(source_frames);
    drop(output);
    drop(renderer);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_late_results_keep_exact_pixels_and_skip_decode_prepare() {
    late_results_case((3, 2));
}

#[test]
fn native_gpu_output_cpu_late_different_extent_uses_canonical_scaling_without_rerender() {
    late_results_case((6, 4));
}

#[test]
fn native_gpu_output_cpu_mismatched_frozen_timing_is_terminal_before_publication() {
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let input = jobs(3, 2, &[shader(IDENTITY)]).map(|job| NativeLayerPreparation {
        input: job.input,
        chain: job.chain.source.into_owned(),
        entry: job.entry,
        source_generation: job.source_generation,
        content_sequence: job.content_sequence,
    });
    let continuation = renderer
        .prepare_native_layer_continuation(
            request(1),
            input,
            NativeBatchRestrictions::default(),
            Some(&|| false),
        )
        .unwrap()
        .into_cpu()
        .unwrap();
    let quote = owner
        .quote(continuation.allocation_plan().unwrap())
        .unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let results = continuation
        .run(&mut owner, reservation, &|| false)
        .unwrap();
    let preparations = renderer.isf_shader_cache_clock;
    let clocks = renderer
        .isf_runtime
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_frame_index();
    for outcome in results.outcomes() {
        validate_native_cpu_outcome_timing(outcome).unwrap();
    }
    let mut mismatched = None;
    for delta in [false, true] {
        let altered = NativeCpuResults::new(
            results
                .outcomes()
                .iter()
                .enumerate()
                .map(|(index, outcome)| {
                    let mut clocks = outcome.clocks();
                    if index == 0 {
                        let clocks = clocks.as_mut().unwrap();
                        if delta {
                            clocks.time_delta_seconds_bits ^= 1;
                        } else {
                            clocks.time_seconds_bits ^= 1;
                        }
                    }
                    NativeCpuOutcome::new(
                        outcome.charged_owner(),
                        outcome.faults().to_vec(),
                        outcome.scope().clone(),
                        clocks,
                    )
                })
                .collect(),
        );
        assert!(matches!(
            validate_native_cpu_outcome_timing(&altered.outcomes()[0]),
            Err(VideoRuntimeError::NativeGpu(_))
        ));
        if !delta {
            mismatched = Some(altered);
        }
    }
    let snapshot = snapshot(3);
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (mut owner, rejected) = renderer.finish_native_cpu_layers(
        owner,
        request(1),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &plan,
        3,
        2,
        &|| false,
        mismatched.unwrap(),
    );
    assert!(
        matches!(rejected, Err(VideoPreviewError::Runtime(VideoRuntimeError::NativeGpu(message)))
        if message.contains("frozen CPU timing"))
    );
    assert!(renderer.runtime.queues.is_empty());
    assert!(renderer.output_last_valid_frames.is_empty());
    assert_eq!(renderer.frame_provider.decodes, 0);
    assert_eq!(renderer.isf_shader_cache_clock, preparations);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        clocks
    );
    drop(results);
    drop(renderer);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_recovered_history_finishes_without_redecode_or_reprepare_and_keeps_faults()
{
    use crate::isf_native_gpu::tests::{complete, encode};
    use crate::native_gpu_history::adapter::{NativeGpuHistory, NativeGpuValidatedEntries};
    use crate::native_gpu_history::{HistoryHostBudget, HistoryScope};
    use crate::native_gpu_output::cpu::history::NativeCpuHistoryQueue;
    use crate::native_gpu_result::NativeIsfCompletion;
    let mut fixture = gpu();
    let mut prepared = jobs(3, 2, &[shader(IDENTITY)]);
    Arc::make_mut(&mut prepared[1].chain.stages[0].shader).wgsl = "not valid WGSL".into();
    let (encoded, pending) = encode(&mut fixture, 1, prepared);
    let recovered = match complete(&mut fixture, encoded, pending) {
        NativeIsfCompletion::RecoveredCpu { layers } => layers,
        NativeIsfCompletion::Failed { fault, .. } => {
            panic!("fixed-clock recovery failed: {fault:?}")
        }
        _ => panic!("invalid middle shader must exercise actual fixed-clock CPU recovery"),
    };
    assert!(recovered[0].faults.is_empty());
    assert!(!recovered[1].faults.is_empty());
    assert_eq!(fixture.runtime.native_frame_index(), 3);
    let host = HistoryHostBudget::new(HistoryHostBudget::planning_limit(3, (3, 2)).unwrap());
    let mut queues = Vec::new();
    let mut faults = Vec::new();
    let mut pointers = Vec::new();
    for outcome in recovered {
        let identity = outcome.identity;
        pointers.push(outcome.frame.data.as_ptr());
        let scope = HistoryScope {
            frame_scope: identity.frame,
            input: identity.entry.input,
            layer_id: identity.layer_id,
            queue_epoch: identity.entry.queue_epoch,
            source_generation: identity.source_generation,
            extent: (identity.width, identity.height),
        };
        let (sealed, recovered_faults) =
            NativeGpuValidatedEntries::seal_recovered_cpu(request(1), outcome, &host)
                .unwrap_or_else(|failure| panic!("recovery seal {:?}", failure.error));
        faults.extend(recovered_faults);
        let mut history =
            NativeGpuHistory::new(fixture.owner.context().id(), scope, host.clone()).unwrap();
        let append = history.plan(&sealed.metadata(), identity.pts_ms).unwrap();
        drop(
            history
                .commit_validated(append, sealed)
                .unwrap_or_else(|failure| panic!("recovery commit {:?}", failure.error)),
        );
        let handover = history.begin_cpu_handover();
        assert_eq!(handover.pending(), 0);
        let cpu_snapshot = handover
            .finish()
            .unwrap_or_else(|_| panic!("CPU recovery handover complete"));
        let entry = NativeCpuHistoryQueue::from_snapshot(scope, cpu_snapshot)
            .unwrap_or_else(|failure| panic!("recovery receive {:?}", failure.error))
            .into_entries()
            .pop()
            .unwrap();
        let (frame, entry) = entry.into_canonical_parts();
        let mut queue = FrameQueue::new_for_input(scope.input, scope.layer_id, 8);
        queue.push_shared(frame);
        queue.native_owners[0] = Some(entry);
        queues.push(queue);
    }
    let Fixture { owner, runtime } = fixture;
    let mut renderer = renderer(runtime);
    renderer.frame_provider.fail = true;
    renderer.runtime.queues = queues;
    let mut snapshot = snapshot(3);
    snapshot.compositions[0].layer_ids = vec![1];
    let clean_plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let expected = composite_frames_rgba8(
        &clean_plan.composition,
        &renderer
            .runtime
            .select_frames_for_plan(&clean_plan.composition),
        3,
        2,
    )
    .unwrap();
    let preparations = renderer.isf_shader_cache_clock;
    let (owner, clean) = renderer.finish_native_cpu_history(
        owner,
        request(1),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &clean_plan,
        3,
        2,
        &|| false,
        Vec::new(),
    );
    let clean = clean.unwrap();
    assert_eq!(
        clean.result.evidence.freshness,
        VideoOutputRenderFreshness::Fresh
    );
    assert_eq!(
        clean.result.payload,
        VideoOutputArtisticPayload::Frame(expected)
    );
    let last_valid = renderer.output_last_valid_frames[0].frame.clone();
    snapshot.compositions[0].layer_ids = vec![1, 2];
    let fault_plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (mut owner, faulted) = renderer.finish_native_cpu_history(
        owner,
        request(2),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &fault_plan,
        3,
        2,
        &|| false,
        faults.clone(),
    );
    let faulted = faulted.unwrap();
    assert_eq!(
        faulted.result.evidence.freshness,
        VideoOutputRenderFreshness::Error
    );
    assert!(faulted.result.evidence.error.is_some());
    assert_eq!(renderer.last_effect_stage_faults, faults);
    assert_eq!(
        renderer.output_last_valid_frames[0].frame, last_valid,
        "faulted recovery cannot seed Fresh LastValid"
    );
    assert_eq!(renderer.frame_provider.decodes, 0);
    assert_eq!(renderer.isf_shader_cache_clock, preparations);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    for (queue, pointer) in renderer.runtime.queues.iter().zip(pointers) {
        assert_eq!(queue.slots[0].as_ref().unwrap().data.as_ptr(), pointer);
        assert!(queue.native_owners[0].is_some());
    }
    // A later canonical composition error must not erase the recovered fault
    // publication while it returns the previously valid artistic frame. This
    // deliberately invalid plain CPU queue input is not a fabricated G proof.
    let selected = renderer.runtime.queues[0].slots[0].as_ref().unwrap();
    let invalid = VideoFrame {
        layer_id: selected.layer_id,
        width: selected.width,
        height: selected.height,
        pts_ms: selected.pts_ms,
        duration_ms: selected.duration_ms,
        format: VideoPixelFormat::Rgba8,
        data: vec![0],
    };
    renderer.runtime.push_frame(invalid);
    let (returned, fallback) = renderer.finish_native_cpu_history(
        owner,
        request(3),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &fault_plan,
        3,
        2,
        &|| false,
        faults.clone(),
    );
    owner = returned;
    let fallback = fallback.unwrap();
    assert_eq!(
        fallback.result.evidence.freshness,
        VideoOutputRenderFreshness::LastValid
    );
    assert_eq!(renderer.last_effect_stage_faults, faults);
    assert_eq!(renderer.output_last_valid_frames[0].frame, last_valid);
    assert_eq!(renderer.frame_provider.decodes, 0);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        3
    );
    drop(clean);
    drop(faulted);
    drop(fallback);
    drop(renderer);
    assert_eq!(host.snapshot().unique_bytes, 0);
    stop(&mut owner);
}

struct SharedRawProvider {
    frame: Arc<VideoFrame>,
    reads: usize,
}
impl VideoFrameProvider for SharedRawProvider {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}
    fn frame_for_layer(
        &mut self,
        _: &VideoLayerSummary,
        _: u32,
        _: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        panic!("canonical live renderer must use shared raw acquisition");
    }
    fn frames_for_layer_shared_with_cancellation(
        &mut self,
        _: &VideoLayerSummary,
        _: u32,
        _: u32,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<Vec<Arc<VideoFrame>>, VideoFrameProviderError> {
        if cancel.is_cancelled() {
            return Err(VideoFrameProviderError::Cancelled);
        }
        self.reads += 1;
        Ok(vec![Arc::clone(&self.frame)])
    }
}

fn imported_cpu_history(
    owner: &NativeGpuOwner,
    raw: &Arc<VideoFrame>,
) -> (
    crate::native_gpu_history::HistoryHostBudget,
    crate::native_gpu_output::cpu::history::OwnedCpuHistoryEntry,
) {
    use crate::native_gpu_history::{HistoryHostBudget, HistoryScope};
    use crate::native_gpu_output::cpu::history::{
        import_legacy_cpu_history, CpuHistoryEntrySequence, NativeCpuHistoryQueue,
    };
    let scope = HistoryScope {
        frame_scope: request(1),
        input: protocol::VideoRenderInputKey {
            project_render_epoch: 7,
            input_id: protocol::VideoRenderInputId(raw.layer_id + 1000),
        },
        layer_id: raw.layer_id,
        queue_epoch: 9,
        source_generation: 13,
        extent: (raw.width, raw.height),
    };
    let host = HistoryHostBudget::new(raw.data.capacity() as u64);
    let mut sequence = CpuHistoryEntrySequence::new(scope, None);
    let history = import_legacy_cpu_history(
        owner.context().id(),
        scope,
        &host,
        &mut sequence,
        vec![Arc::clone(raw)],
    )
    .unwrap_or_else(|failure| panic!("import {:?}", failure.error));
    let handover = history.begin_cpu_handover();
    assert_eq!(handover.pending(), 0);
    let cpu_snapshot = handover
        .finish()
        .unwrap_or_else(|_| panic!("imported handover is complete"));
    let imported = NativeCpuHistoryQueue::from_snapshot(scope, cpu_snapshot)
        .unwrap_or_else(|failure| panic!("receive {:?}", failure.error))
        .into_entries()
        .pop()
        .unwrap();
    (host, imported)
}

#[test]
fn native_gpu_output_cpu_plain_shared_raw_input_is_charged_once_before_composition() {
    let Fixture { mut owner, runtime } = gpu();
    let raw = Arc::new(VideoFrame {
        layer_id: 1,
        width: 3,
        height: 2,
        pts_ms: 0,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: [12, 34, 56, 255].repeat(6),
    });
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        SharedRawProvider {
            frame: Arc::clone(&raw),
            reads: 0,
        },
    );
    renderer.isf_runtime = Some(Ok(runtime));
    let snapshot = snapshot(1);
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    for sequence in 1..=2 {
        let (returned, output) = renderer.prepare_native_cpu_output(
            owner,
            request(sequence),
            &snapshot,
            VideoEffectRenderContext {
                clip_runtime: &VideoClipRuntimeSnapshot::default(),
                project_render_epoch: 7,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            &plan,
            3,
            2,
            &|| false,
        );
        owner = returned;
        let output = output.unwrap();
        let queue = &renderer.runtime.queues[0];
        assert_eq!(queue.len, 1);
        let whole = queue.native_cpu_owners[0]
            .as_ref()
            .expect("plain input must carry its charge");
        assert!(Arc::ptr_eq(&whole.frame, &raw));
        assert_eq!(
            whole.charge.as_ref().unwrap().kind(),
            ChargeKind::ActiveCpuInput
        );
        assert_eq!(
            owner
                .budget_snapshot()
                .for_kind(ChargeKind::ActiveCpuInput)
                .active_bytes,
            raw.data.capacity() as u64,
            "reusing the same raw Arc must alias the original whole allocation owner"
        );
        assert_eq!(
            output.result.payload,
            VideoOutputArtisticPayload::Frame(VideoFrame {
                layer_id: 0,
                duration_ms: 0,
                ..raw.as_ref().clone()
            })
        );
        drop(output);
    }
    assert_eq!(renderer.frame_provider.reads, 2);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        0
    );
    drop(renderer);
    drop(raw);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_plain_imported_history_gains_one_charge_and_keeps_whole_parent() {
    use crate::native_gpu_history::adapter::CanonicalCpuHistoryOwner;
    use crate::native_gpu_history::HistoryScope;
    use crate::native_gpu_output::cpu::history::{
        import_canonical_cpu_history_with_binding, CpuHistoryEntrySequence, NativeCpuHistoryQueue,
    };
    let Fixture { mut owner, runtime } = gpu();
    let raw = Arc::new(VideoFrame {
        layer_id: 1,
        width: 3,
        height: 2,
        pts_ms: 0,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: [12, 34, 56, 255].repeat(6),
    });
    let capacity = raw.data.capacity() as u64;
    let (host, imported) = imported_cpu_history(&owner, &raw);
    assert!(imported.native_cpu_owner().unwrap().charge.is_none());
    let identity = *imported.identity();
    let weak_backing = Arc::downgrade(imported.backing());
    let mut queue = FrameQueue::new_for_input(imported.identity().entry.input, 1, 8);
    queue.push_shared(Arc::clone(&raw));
    queue.native_owners[0] = Some(imported);
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        SharedRawProvider {
            frame: Arc::clone(&raw),
            reads: 0,
        },
    );
    renderer.isf_runtime = Some(Ok(runtime));
    renderer.runtime.queues.push(queue);
    let snapshot = snapshot(1);
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let mut allocation = None;
    for sequence in 1..=2 {
        let (returned, output) = renderer.prepare_native_cpu_output(
            owner,
            request(sequence),
            &snapshot,
            VideoEffectRenderContext {
                clip_runtime: &VideoClipRuntimeSnapshot::default(),
                project_render_epoch: 7,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            &plan,
            3,
            2,
            &|| false,
        );
        owner = returned;
        let output = output.unwrap();
        let queue = &renderer.runtime.queues[0];
        assert_eq!(queue.len, 1);
        let whole = queue.native_cpu_owners[0].as_ref().unwrap();
        assert!(Arc::ptr_eq(&whole.frame, &raw));
        let charge = whole
            .charge
            .as_ref()
            .expect("host-only input must gain managed admission");
        assert_eq!(charge.kind(), ChargeKind::ActiveCpuInput);
        assert_eq!(charge.capacity(), capacity);
        if let Some(allocation) = allocation {
            assert_eq!(charge.id(), allocation);
        } else {
            allocation = Some(charge.id());
        }
        assert!(whole.retained_owner().is_some());
        assert!(weak_backing.upgrade().is_some());
        assert_eq!(host.snapshot().unique_bytes, capacity);
        assert_eq!(host.snapshot().managed_alias_bytes, capacity);
        assert_eq!(
            host.snapshot().unmanaged_bytes(),
            0,
            "active managed pixels are not added to process bytes twice"
        );
        assert_eq!(
            owner
                .budget_snapshot()
                .for_kind(ChargeKind::ActiveCpuInput)
                .active_bytes,
            capacity
        );
        let cached = renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_cpu_cached_inputs();
        let same = cached
            .iter()
            .filter(|cached| Arc::ptr_eq(&cached.frame, &raw))
            .collect::<Vec<_>>();
        assert_eq!(
            same.len(),
            1,
            "the stale uncharged runtime alias must be replaced"
        );
        assert!(Arc::ptr_eq(same[0], whole));
        assert!(
            queue.native_owners[0].is_none(),
            "the complete parent now travels with the managed owner"
        );
        drop(output);
    }
    // Keep the original Imported backing while the managed wrapper makes the
    // full canonical -> P -> canonical roundtrip on the exact same host ledger.
    let imported_survivor = weak_backing.upgrade().unwrap();
    let scope = HistoryScope {
        frame_scope: identity.frame,
        input: identity.entry.input,
        layer_id: identity.layer_id,
        queue_epoch: identity.entry.queue_epoch,
        source_generation: identity.source_generation,
        extent: (identity.width, identity.height),
    };
    let mut issuance = CpuHistoryEntrySequence::new(scope, Some(identity.entry.sequence));
    let whole = Arc::clone(
        renderer.runtime.queues[0].native_cpu_owners[0]
            .as_ref()
            .unwrap(),
    );
    let (history, _) = import_canonical_cpu_history_with_binding(
        owner.context().id(),
        scope,
        &host,
        &mut issuance,
        vec![(
            Arc::clone(&raw),
            Some(CanonicalCpuHistoryOwner::Native(whole)),
        )],
    )
    .unwrap_or_else(|failure| panic!("managed re-import {:?}", failure.error));
    let handover = history.begin_cpu_handover();
    assert_eq!(handover.pending(), 0);
    let cpu_snapshot = handover
        .finish()
        .unwrap_or_else(|_| panic!("managed CPU handover complete"));
    let reimported = NativeCpuHistoryQueue::from_snapshot(scope, cpu_snapshot)
        .unwrap_or_else(|failure| panic!("managed receive {:?}", failure.error))
        .into_entries()
        .pop()
        .unwrap();
    let returned = reimported.native_cpu_owner().unwrap();
    assert!(Arc::ptr_eq(&returned.frame, &raw));
    assert_eq!(returned.charge.as_ref().unwrap().id(), allocation.unwrap());
    renderer.runtime.queues[0].native_cpu_owners[0] = None;
    renderer.runtime.queues[0].native_owners[0] = Some(reimported);
    drop(returned);
    let (returned, output) = renderer.prepare_native_cpu_output(
        owner,
        request(3),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &VideoClipRuntimeSnapshot::default(),
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &plan,
        3,
        2,
        &|| false,
    );
    owner = returned;
    drop(output.unwrap());
    assert_eq!(
        renderer.runtime.queues[0].native_cpu_owners[0]
            .as_ref()
            .unwrap()
            .charge
            .as_ref()
            .unwrap()
            .id(),
        allocation.unwrap()
    );
    assert_eq!(
        owner
            .budget_snapshot()
            .for_kind(ChargeKind::ActiveCpuInput)
            .active_bytes,
        capacity
    );
    assert_eq!(host.snapshot().unique_bytes, capacity);
    assert_eq!(host.snapshot().managed_alias_bytes, capacity);
    assert_eq!(renderer.frame_provider.reads, 3);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        0
    );
    drop(renderer);
    assert_eq!(host.snapshot().unique_bytes, capacity);
    assert_eq!(
        host.snapshot().managed_alias_bytes,
        0,
        "the last managed alias must release its classification"
    );
    assert_eq!(host.snapshot().unmanaged_bytes(), capacity);
    drop(imported_survivor);
    assert!(weak_backing.upgrade().is_none());
    assert_eq!(host.snapshot().unique_bytes, 0);
    drop(raw);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_sampled_imported_image_is_admitted_by_shared_g_executor() {
    let Fixture {
        mut owner,
        mut runtime,
    } = gpu();
    let sampled = Arc::new(VideoFrame {
        layer_id: 2,
        width: 3,
        height: 2,
        pts_ms: 0,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: [17, 39, 61, 255].repeat(6),
    });
    let (host, imported) = imported_cpu_history(&owner, &sampled);
    let prior = imported.native_cpu_owner().unwrap();
    assert!(prior.charge.is_none());
    runtime.remember_native_cpu_frame(&prior);
    let sampled_shader = shader(
        r#"/*{"INPUTS":[{"NAME":"inputImage","TYPE":"image"},{"NAME":"otherImage","TYPE":"image"}]}*/
void main() { gl_FragColor = IMG_THIS_PIXEL(otherImage); }"#,
    );
    let mut job = jobs(3, 2, &[sampled_shader]).into_iter().next().unwrap();
    job.chain.stages[0]
        .images
        .push(("otherImage".into(), Arc::clone(&sampled)));
    let capacity = job.input.data.capacity() as u64 + sampled.data.capacity() as u64;
    let invocation = PreparedNativeCpuInvocation::new(job.input, job.chain, job.date, None);
    let prepared = runtime
        .prepare_native_cpu_inputs(request(1), invocation)
        .unwrap();
    let quote = owner.quote(prepared.allocation_plan().unwrap()).unwrap();
    let reservation = owner.reserve(quote).unwrap();
    let results = runtime
        .render_native_cpu_fallback(&mut owner, prepared, reservation, &|| false)
        .unwrap();
    assert!(results.outcomes()[0].faults().is_empty());
    assert_eq!(results.outcomes()[0].frame().data, sampled.data);
    assert_eq!(runtime.native_frame_index(), 1);
    assert_eq!(
        owner
            .budget_snapshot()
            .for_kind(ChargeKind::ActiveCpuInput)
            .high_water_bytes,
        capacity,
        "G must admit an Existing(None) sampled input as well as its main input"
    );
    assert_eq!(host.snapshot().unique_bytes, sampled.data.capacity() as u64);
    assert_eq!(
        host.snapshot().managed_alias_bytes,
        0,
        "completed sampling releases only the temporary managed alias"
    );
    assert_eq!(
        host.snapshot().unmanaged_bytes(),
        sampled.data.capacity() as u64
    );
    drop(results);
    drop(runtime);
    drop(prior);
    drop(imported);
    assert_eq!(host.snapshot().unique_bytes, 0);
    drop(sampled);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_selected_imported_predecessor_is_admitted_before_composition() {
    let Fixture { owner, runtime } = gpu();
    let previous = Arc::new(VideoFrame {
        layer_id: 1,
        width: 3,
        height: 2,
        pts_ms: 0,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: [12, 34, 56, 255].repeat(6),
    });
    let current = Arc::new(VideoFrame {
        pts_ms: 100,
        data: [90, 80, 70, 255].repeat(6),
        ..previous.as_ref().clone()
    });
    let capacity = previous.data.capacity() as u64 + current.data.capacity() as u64;
    let (host, imported) = imported_cpu_history(&owner, &previous);
    let mut queue = FrameQueue::new(1, 8);
    queue.push_shared(Arc::clone(&previous));
    queue.native_owners[0] = Some(imported);
    let mut renderer = renderer(runtime);
    renderer.runtime.queues.push(queue);
    let snapshot = snapshot(1);
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (mut owner, output) = renderer.with_native_cpu_owner(owner, request(1), |renderer| {
        renderer
            .publish_native_cpu_raw_frame(Arc::clone(&current), Some(&|| false))
            .map_err(VideoPreviewError::Runtime)?;
        let result = renderer.prepare_native_cpu_artistic_impl(
            &snapshot,
            VideoEffectRenderContext {
                clip_runtime: &VideoClipRuntimeSnapshot::default(),
                project_render_epoch: 7,
                audio_frame: None,
            },
            Some(&VideoLayerTransitionRuntimeSnapshot::default()),
            &plan,
            3,
            2,
            Some(&|| false),
            Some(Vec::new()),
        )?;
        assert_eq!(
            renderer
                .native_cpu_bridge
                .as_ref()
                .unwrap()
                .owner
                .budget_snapshot()
                .for_kind(ChargeKind::ActiveCpuInput)
                .active_bytes,
            capacity,
            "the selected older backing and live newer raw input must both be admitted"
        );
        renderer.capture_native_cpu_output(result)
    });
    let output = output.unwrap();
    assert_eq!(
        output.result.payload,
        VideoOutputArtisticPayload::Frame(VideoFrame {
            layer_id: 0,
            duration_ms: 0,
            ..previous.as_ref().clone()
        })
    );
    assert_eq!(renderer.runtime.queues[0].len, 2);
    assert_eq!(renderer.frame_provider.decodes, 0);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        0
    );
    drop(output);
    drop(renderer);
    assert_eq!(host.snapshot().unique_bytes, 0);
    drop(current);
    drop(previous);
    stop(&mut owner);
}

#[derive(Default)]
struct ClipTakeProvider {
    capacities: Vec<usize>,
}
impl VideoFrameProvider for ClipTakeProvider {
    fn retain_layers(&mut self, _: &[VideoLayerId]) {}
    fn frame_for_layer(
        &mut self,
        _: &VideoLayerSummary,
        _: u32,
        _: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        panic!("ClipTake must acquire its two exact render inputs");
    }
    fn frame_for_input(
        &mut self,
        input: &VideoRenderInput,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        assert_eq!(input.key.project_render_epoch, 7);
        let pixel = match input.clip_slot_id {
            Some(VideoClipSlotId(20)) => [10, 20, 30, 255],
            Some(VideoClipSlotId(21)) => [50, 60, 70, 255],
            other => panic!("unexpected ClipTake source {other:?}"),
        };
        let data = pixel.repeat(input.request.width as usize * input.request.height as usize);
        self.capacities.push(data.capacity());
        Ok(VideoFrame {
            layer_id: input.layer_id(),
            width: input.request.width,
            height: input.request.height,
            pts_ms: input.request.position_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data,
        })
    }
}

fn clip_take_input_admission_case(clip_effect: bool) {
    let Fixture { owner, runtime } = gpu();
    let mut snapshot = snapshot(1);
    let layer = &mut snapshot.layers[0];
    for (slot, media) in [(20, 100), (21, 101)] {
        layer.clip_slots.push(protocol::VideoClipSlotSummary {
            id: VideoClipSlotId(slot),
            media_asset_id: media,
            in_point_ms: 0,
            out_point_ms: None,
            loop_mode: protocol::VideoClipLoopMode::Once,
            speed: 1.0,
            cue_points: Vec::new(),
            launch_quantization: protocol::VideoClipLaunchQuantization::Immediate,
            effect_overrides: Vec::new(),
        });
        snapshot.media_assets.push(protocol::MediaAssetSummary {
            id: media,
            label: format!("slot {slot}"),
            source: layer.source.clone(),
            content_hash: None,
            byte_size: None,
        });
    }
    if clip_effect {
        snapshot.effect_chains.push(
            chain(
                8,
                VideoEffectScope::Clip {
                    layer_id: 1,
                    slot_id: VideoClipSlotId(20),
                },
                "c.r+2.0/255.0",
            )
            .0,
        );
    }
    let clips = VideoClipRuntimeSnapshot {
        layers: vec![protocol::VideoClipLayerRuntimeSummary {
            layer_id: 1,
            active_slot_id: Some(VideoClipSlotId(20)),
            queued_slot_id: Some(VideoClipSlotId(21)),
            transition: Some(protocol::VideoClipTakeTransitionSummary {
                origin_slot_id: VideoClipSlotId(20),
                outgoing_slot_id: VideoClipSlotId(20),
                incoming_slot_id: VideoClipSlotId(21),
                kind: protocol::VideoClipTakeKind::Crossfade,
                elapsed_ms: 500,
                duration_ms: 1000,
                duration: protocol::VideoClipTakeDuration::milliseconds(1000),
                progress_millis: 500,
                incoming_playhead_ms: 0,
                incoming_playing: true,
            }),
            ..Default::default()
        }],
        ..Default::default()
    };
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        ClipTakeProvider::default(),
    );
    renderer.isf_runtime = Some(Ok(runtime));
    let plan = build_video_output_render_plan(&snapshot, 101).unwrap();
    let (mut owner, output) = renderer.prepare_native_cpu_output(
        owner,
        request(1),
        &snapshot,
        VideoEffectRenderContext {
            clip_runtime: &clips,
            project_render_epoch: 7,
            audio_frame: None,
        },
        &VideoLayerTransitionRuntimeSnapshot::default(),
        &plan,
        3,
        2,
        &|| false,
    );
    let output = output.unwrap();
    let VideoOutputArtisticPayload::Frame(frame) = &output.result.payload else {
        panic!("ClipTake result");
    };
    let expected = if clip_effect {
        [31, 40, 50, 255]
    } else {
        [30, 40, 50, 255]
    };
    assert!(frame.data.chunks_exact(4).all(|pixel| pixel == expected));
    assert_eq!(renderer.frame_provider.capacities.len(), 2);
    let raw_peak: usize = renderer.frame_provider.capacities.iter().sum();
    assert_eq!(
        owner
            .budget_snapshot()
            .for_kind(ChargeKind::ActiveCpuInput)
            .high_water_bytes,
        raw_peak as u64,
        "both live owned sources must be charged before the first Clip execution"
    );
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        u32::from(clip_effect)
    );
    drop(output);
    drop(renderer);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_clip_take_no_fx_admits_both_owned_sources() {
    clip_take_input_admission_case(false);
}

#[test]
fn native_gpu_output_cpu_clip_take_first_clip_fx_keeps_incoming_admitted() {
    clip_take_input_admission_case(true);
}

fn capture_follow(
    renderer: &mut VideoPreviewRenderer<Provider>,
    result: NativeFollowTransitionResult,
) -> Result<NativeCpuOutput, VideoPreviewError> {
    let mapping = VideoOutputMapping::default();
    renderer.capture_native_cpu_output(VideoOutputArtisticRenderResult {
        payload: result.payload,
        output_mapping: mapping,
        output_mapping_identity: VideoOutputMappingIdentity::from_mapping(&mapping),
        evidence: VideoOutputRenderEvidence {
            project_render_epoch: 7,
            output_id: 101,
            freshness: result.evidence.freshness,
            error: result.evidence.error,
        },
    })
}

#[test]
fn native_gpu_output_cpu_follow_payloadless_side_matches_all_canonical_transition_pixels() {
    use protocol::VideoClipTakeKind as Kind;
    let Fixture { mut owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let source = VideoFrame {
        layer_id: 0,
        width: 4,
        height: 2,
        pts_ms: 80,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: vec![
            10, 20, 30, 40, 200, 30, 10, 255, 70, 90, 110, 130, 255, 128, 0, 64, 20, 80, 140, 200,
            0, 10, 255, 100, 110, 100, 90, 80, 40, 30, 20, 10,
        ],
    };
    // This frame is only the old canonical test oracle. The ON path below
    // receives None and must retain just its actual mixed output allocation.
    let zero_oracle = VideoFrame {
        layer_id: 0,
        width: 4,
        height: 2,
        pts_ms: 0,
        duration_ms: 0,
        format: VideoPixelFormat::Rgba8,
        data: vec![0; 32],
    };
    let mut sequence = 1;
    for kind in [
        Kind::Cut,
        Kind::Crossfade,
        Kind::Custom,
        Kind::Dip,
        Kind::Wipe,
        Kind::Luma,
        Kind::Displacement,
        Kind::Blur,
        Kind::Glitch,
    ] {
        for progress in [0, 125, 500, 875, 1000] {
            for curve in [
                VideoLayerTransitionCurve::Linear,
                VideoLayerTransitionCurve::EaseInOut,
            ] {
                for missing_outgoing in [true, false] {
                    let (outgoing, incoming, oracle_outgoing, oracle_incoming) = if missing_outgoing
                    {
                        (None, Some(&source), &zero_oracle, &source)
                    } else {
                        (Some(&source), None, &source, &zero_oracle)
                    };
                    let expected = combine_follow_output_transition_rgba8(
                        oracle_outgoing,
                        oracle_incoming,
                        kind,
                        curve,
                        progress,
                    )
                    .unwrap();
                    let (returned, output) =
                        renderer.with_native_cpu_owner(owner, request(sequence), |renderer| {
                            let combined = renderer.render_native_follow_output_transition(
                                NativeFollowTransitionRequest {
                                    outgoing,
                                    incoming,
                                    extent: (4, 2),
                                    kind,
                                    curve,
                                    progress_millis: progress,
                                    source_timeline_id: protocol::TimelineId(55),
                                    transition_chain: None,
                                    last_valid: None,
                                    audio_frame: None,
                                },
                                &|| false,
                            )?;
                            assert_eq!(combined.evidence.transition, expected.evidence);
                            capture_follow(renderer, combined)
                        });
                    owner = returned;
                    sequence += 1;
                    let output = output.unwrap();
                    assert_eq!(output.result.payload, VideoOutputArtisticPayload::Frame(expected.frame),
                        "kind={kind:?} curve={curve:?} p={progress} missing_outgoing={missing_outgoing}");
                    assert_eq!(owner.budget_snapshot().for_kind(ChargeKind::CpuSnapshot).active_bytes, 32,
                        "no full-frame black source or transition temporary is allocated by the ON combiner");
                    drop(output);
                }
            }
        }
    }
    drop(renderer);
    stop(&mut owner);
}

#[test]
fn native_gpu_output_cpu_follow_one_black_is_analytic_and_both_black_bypass_fx() {
    let Fixture { owner, runtime } = gpu();
    let mut renderer = renderer(runtime);
    let incoming = VideoFrame {
        layer_id: 0,
        width: 1,
        height: 1,
        pts_ms: 30,
        duration_ms: 16,
        format: VideoPixelFormat::Rgba8,
        data: vec![40, 60, 80, 255],
    };
    let (_, effect) = chain(
        9,
        VideoEffectScope::Output { output_id: 101 },
        "c.r+1.0/255.0",
    );
    let transition_chain = ResolvedVideoEffectChain {
        chain_id: None,
        scope: VideoEffectScope::Transition {
            owner: VideoTransitionEffectOwner::TimelineFollow {
                source_timeline_id: protocol::TimelineId(55),
            },
        },
        bypassed: false,
        stages: vec![ResolvedVideoEffectStage {
            stage_id: None,
            effect_id: None,
            enabled: true,
            label: "transition increment".into(),
            effect,
        }],
    };
    let (owner, output) = renderer.with_native_cpu_owner(owner, request(1), |renderer| {
        let output = renderer.render_native_follow_output_transition(
            NativeFollowTransitionRequest {
                outgoing: None,
                incoming: Some(&incoming),
                extent: (1, 1),
                kind: protocol::VideoClipTakeKind::Crossfade,
                curve: VideoLayerTransitionCurve::Linear,
                progress_millis: 500,
                source_timeline_id: protocol::TimelineId(55),
                transition_chain: Some(&transition_chain),
                last_valid: None,
                audio_frame: None,
            },
            &|| false,
        )?;
        capture_follow(renderer, output)
    });
    let output = output.unwrap();
    let VideoOutputArtisticPayload::Frame(frame) = &output.result.payload else {
        panic!("mixed artwork");
    };
    assert_eq!(
        frame.data,
        vec![21, 30, 40, 128],
        "transparent-zero source, then the one transition invocation"
    );
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        1
    );
    drop(output);
    let before_clock = renderer.isf_shader_cache_clock;
    let (mut owner, blackout) = renderer.with_native_cpu_owner(owner, request(2), |renderer| {
        let before = renderer
            .native_cpu_bridge
            .as_ref()
            .unwrap()
            .owner
            .budget_snapshot();
        let output = renderer.render_native_follow_output_transition(
            NativeFollowTransitionRequest {
                outgoing: None,
                incoming: None,
                extent: (1, 1),
                kind: protocol::VideoClipTakeKind::Dip,
                curve: VideoLayerTransitionCurve::Linear,
                progress_millis: 500,
                source_timeline_id: protocol::TimelineId(55),
                transition_chain: Some(&transition_chain),
                last_valid: Some(&incoming),
                audio_frame: None,
            },
            &|| false,
        )?;
        assert_eq!(
            renderer
                .native_cpu_bridge
                .as_ref()
                .unwrap()
                .owner
                .budget_snapshot(),
            before
        );
        capture_follow(renderer, output)
    });
    let blackout = blackout.unwrap();
    assert_eq!(
        blackout.result.payload,
        VideoOutputArtisticPayload::HardBlackout
    );
    assert!(blackout.charge.is_none());
    assert_eq!(renderer.isf_shader_cache_clock, before_clock);
    assert_eq!(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_frame_index(),
        1
    );
    drop(blackout);
    drop(renderer);
    stop(&mut owner);
}
