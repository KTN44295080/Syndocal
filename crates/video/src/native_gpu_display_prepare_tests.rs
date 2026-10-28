//! Facade regressions. These use a real headless G owner; no OS surface, Ready,
//! completion proof or Presented receipt is fabricated. Identity-only cases
//! exercise facade preflight and must stop before a sink could be reached.
use super::*;
use crate::native_gpu_budget::ChargeKind;
use crate::native_gpu_contract::GpuOwnerState;
use crate::{
    VideoFrame, VideoFrameProviderError, VideoOutputRenderFreshness, VideoPixelFormat,
    VideoRuntimeConfig,
};
use std::cell::Cell;
use std::time::{Duration, Instant};

const OUTPUT: u64 = 101;
const EPOCH: u64 = 7;
const EXTENT: (u32, u32) = (3, 2);

fn headless_session() -> NativeResidentDisplaySession {
    let factory = NativeGpuFactory::new();
    let owner = factory
        .create_headless(OUTPUT, NativeGpuRequirements::default())
        .expect("facade ownership tests require an actual headless adapter");
    NativeResidentDisplaySession {
        output_id: OUTPUT,
        factory,
        owner: ResidentOwner(Some(owner)),
        presenter: None,
        compositor: None,
        cache: SharedNativeOutputCache::new(OUTPUT),
        geometry_generation: 0,
        reserved_geometry: None,
        history: None,
        terminal_fault: None,
        delivery: crate::native_gpu_output::NativeDeliveryCounter::default(),
        retired_ready: Vec::new(),
        retirements: Vec::new(),
        unsubmitted: Vec::new(),
        recovered_history: None,
        request_sequence: 0,
        history_configuration: None,
        input_bindings: Vec::new(),
        next_input_id: 1,
        history_host: None,
        follow_last_valid: None,
        ring: Default::default(),
        deferred_budget: None,
        evidence: Default::default(),
    }
}

fn reap_started(session: &mut NativeResidentDisplaySession) {
    let started = Instant::now();
    while !session.poll_stop().unwrap() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "facade Stop failed to reach Reaped"
        );
        std::thread::yield_now();
    }
    assert_eq!(session.owner.state(), GpuOwnerState::Reaped);
    assert_eq!(session.owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(session.owner.budget_snapshot().reservation_count, 0);
}

#[derive(Default)]
struct Provider {
    reads: usize,
    forbid_read: bool,
}
impl VideoFrameProvider for Provider {
    fn retain_layers(&mut self, _: &[protocol::VideoLayerId]) {}
    fn frame_for_layer(
        &mut self,
        layer: &protocol::VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Result<VideoFrame, VideoFrameProviderError> {
        self.reads += 1;
        assert!(
            !self.forbid_read,
            "control-state blackout must finish before raw acquisition"
        );
        Ok(VideoFrame {
            layer_id: layer.id,
            width,
            height,
            pts_ms: layer.state.position_ms,
            duration_ms: 16,
            format: VideoPixelFormat::Rgba8,
            data: [12, 34, 56, 255].repeat(width as usize * height as usize),
        })
    }
}

fn scene(layers: u64) -> VideoSnapshot {
    VideoSnapshot {
        layers: (1..=layers)
            .map(|id| protocol::VideoLayerSummary {
                id,
                label: format!("facade layer {id}"),
                source: protocol::VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some(format!("{id}.mp4")),
                    name: None,
                    codec: None,
                    metadata: None,
                },
                media_asset_id: None,
                blend_mode: protocol::VideoBlendMode::Normal,
                state: protocol::VideoLayerState::default(),
                isf_effect: None,
                clip_slots: Vec::new(),
                default_clip_slot_id: None,
            })
            .collect(),
        media_assets: Vec::new(),
        effect_chains: Vec::new(),
        effect_presets: Vec::new(),
        layer_groups: Vec::new(),
        transition_buses: Vec::new(),
        compositions: vec![protocol::CompositionSummary {
            id: 70,
            label: "facade".into(),
            layer_ids: (1..=layers).collect(),
            timeline_layer_ids: Vec::new(),
            output_ids: vec![OUTPUT],
        }],
        outputs: vec![protocol::VideoOutputSummary {
            id: OUTPUT,
            label: "headless facade".into(),
            kind: protocol::VideoOutputKind::Display,
            enabled: true,
            composition_id: 70,
            fullscreen: false,
            monitor_id: Some(0),
            monitor_identity: None,
            width: EXTENT.0,
            height: EXTENT.1,
            endpoint_name: None,
            opacity: 1.0,
            blackout: false,
            mapping: protocol::VideoOutputMapping::default(),
        }],
        mapping_presets: Vec::new(),
        auto_vj: protocol::AutoVjSnapshot::default(),
        master_opacity: 1.0,
        blackout: false,
    }
}

struct Permit;
impl NativeResidentPresentationAuthority for Permit {
    fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        NativeResidentPresentDecision::Permit
    }
}

#[derive(Default)]
struct PresentProbe {
    calls: Cell<usize>,
}
impl NativeResidentPresentationAuthority for PresentProbe {
    fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        self.calls.set(self.calls.get() + 1);
        NativeResidentPresentDecision::Permit
    }
}

fn prepare(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<Provider>,
    snapshot: &VideoSnapshot,
) -> NativeResidentDisplayFrame {
    session
        .prepare_output(
            renderer,
            snapshot,
            VideoEffectRenderContext {
                clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                project_render_epoch: EPOCH,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            EXTENT,
            &|| false,
            &Permit,
        )
        .unwrap()
}

#[test]
fn native_gpu_output_display_zero_opacity_is_payloadless_before_provider_or_fx() {
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider {
            forbid_read: true,
            ..Default::default()
        },
    );
    let mut snapshot = scene(2);
    let source = crate::isf_native_gpu::tests::IDENTITY;
    let shader = crate::prepare_isf_shader(source).unwrap();
    let effect = crate::isf_effect_from_prepared("must not execute".into(), source, None, &shader);
    for layer in &mut snapshot.layers {
        layer.isf_effect = Some(effect.clone());
    }
    let plan = crate::build_video_output_render_plan(&snapshot, OUTPUT).unwrap();
    assert!(
        resident_layer_chains(
            &snapshot,
            VideoEffectRenderContext {
                clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                project_render_epoch: EPOCH,
                audio_frame: None
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            &plan
        )
        .is_some(),
        "the fixture must enter resident preparation if the early opacity fence is missing"
    );
    snapshot.outputs[0].opacity = 0.0;
    for _ in 0..2 {
        let frame = prepare(&mut session, &mut renderer, &snapshot);
        assert!(matches!(
            frame.content,
            ResidentDisplayContent::HardBlackout(_)
        ));
        assert_eq!(
            frame.evidence().freshness,
            VideoOutputRenderFreshness::Fresh
        );
        assert!(frame.evidence().error.is_none());
        assert_eq!(frame.extent(), EXTENT);
    }
    assert_eq!(renderer.frame_provider.reads, 0);
    assert_eq!(renderer.isf_shader_cache_clock, 0);
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
    assert!(renderer.runtime.queues.is_empty());
    assert!(renderer.output_last_valid_frames.is_empty());
    assert_eq!(session.owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(session.presented_frames(), 0);
    assert_eq!(session.hard_blackout_frames_presented(), 0);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_display_follow_cache_releases_before_ordinary_or_new_follow_failure() {
    for mode in 0..4 {
        let mut session = headless_session();
        session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            Provider::default(),
        );
        let mut outgoing = scene(1);
        let mut incoming = scene(1);
        let clip_runtime = protocol::VideoClipRuntimeSnapshot::default();
        let transitions = VideoLayerTransitionRuntimeSnapshot::default();
        let context = || VideoEffectRenderContext {
            clip_runtime: &clip_runtime,
            project_render_epoch: EPOCH,
            audio_frame: None,
        };
        let seed = session
            .prepare_follow_output(
                &mut renderer,
                &outgoing,
                &incoming,
                context(),
                &transitions,
                protocol::VideoClipTakeKind::Crossfade,
                protocol::VideoLayerTransitionCurve::Linear,
                500,
                protocol::TimelineId(55),
                1,
                None,
                EXTENT,
                &|| false,
                &Permit,
            )
            .unwrap();
        let weak = match &seed.content {
            ResidentDisplayContent::Cpu(cpu) => Arc::downgrade(&cpu.owner),
            _ => panic!("seed must own actual charged CPU Follow pixels"),
        };
        drop(seed);
        assert!(weak.upgrade().is_some());
        let reads = renderer.frame_provider.reads;
        if mode == 0 {
            let ordinary = prepare(&mut session, &mut renderer, &incoming);
            assert_eq!(
                ordinary.evidence().freshness,
                VideoOutputRenderFreshness::Fresh
            );
            drop(ordinary);
        } else {
            let timeline = protocol::TimelineId(if mode == 1 { 56 } else { 55 });
            let generation = if mode == 2 { 2 } else { 1 };
            if mode < 3 {
                // Even a subsequent invalid plan cannot retain a previous
                // Follow generation's now-ineligible managed owner.
                outgoing.outputs.clear();
            } else {
                // A full-key change must release the old owner before this
                // independent physical-contract mismatch rejects the request.
                incoming.outputs[0].monitor_id = Some(1);
            }
            let result = session.prepare_follow_output(
                &mut renderer,
                &outgoing,
                &incoming,
                context(),
                &transitions,
                protocol::VideoClipTakeKind::Crossfade,
                protocol::VideoLayerTransitionCurve::Linear,
                500,
                timeline,
                generation,
                None,
                EXTENT,
                &|| false,
                &Permit,
            );
            assert!(matches!(result, Err(NativeResidentPrepareError::Fault(_))));
            assert_eq!(renderer.frame_provider.reads, reads);
        }
        assert!(session.follow_last_valid.is_none());
        assert!(
            weak.upgrade().is_none(),
            "obsolete Follow owner must be released before admission"
        );
        session.begin_stop_with_renderer(&mut renderer).unwrap();
        reap_started(&mut session);
    }
}

#[test]
fn native_gpu_output_display_both_zero_opacity_follow_fences_real_cached_cpu_owner() {
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let mut outgoing = scene(1);
    let mut incoming = scene(1);
    let seed = session
        .prepare_follow_output(
            &mut renderer,
            &outgoing,
            &incoming,
            VideoEffectRenderContext {
                clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                project_render_epoch: EPOCH,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            protocol::VideoClipTakeKind::Crossfade,
            protocol::VideoLayerTransitionCurve::Linear,
            500,
            protocol::TimelineId(55),
            1,
            None,
            EXTENT,
            &|| false,
            &Permit,
        )
        .unwrap();
    assert_eq!(seed.evidence().freshness, VideoOutputRenderFreshness::Fresh);
    let weak = match &seed.content {
        ResidentDisplayContent::Cpu(cpu) => Arc::downgrade(&cpu.owner),
        _ => panic!("seed Follow frame must own real charged CPU pixels"),
    };
    drop(seed);
    assert!(weak.upgrade().is_some());
    assert!(session.follow_last_valid.is_some());
    let reads = renderer.frame_provider.reads;
    renderer.frame_provider.forbid_read = true;
    outgoing.outputs[0].opacity = 0.0;
    incoming.outputs[0].opacity = 0.0;
    let blackout = session
        .prepare_follow_output(
            &mut renderer,
            &outgoing,
            &incoming,
            VideoEffectRenderContext {
                clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                project_render_epoch: EPOCH,
                audio_frame: None,
            },
            &VideoLayerTransitionRuntimeSnapshot::default(),
            protocol::VideoClipTakeKind::Dip,
            protocol::VideoLayerTransitionCurve::EaseInOut,
            500,
            protocol::TimelineId(55),
            1,
            None,
            EXTENT,
            &|| false,
            &Permit,
        )
        .unwrap();
    assert!(matches!(
        blackout.content,
        ResidentDisplayContent::HardBlackout(_)
    ));
    assert_eq!(
        blackout.evidence().freshness,
        VideoOutputRenderFreshness::Fresh
    );
    assert!(session.follow_last_valid.is_none());
    assert!(
        weak.upgrade().is_none(),
        "the exact cached CPU frame must be fenced by both-black Follow"
    );
    assert_eq!(renderer.frame_provider.reads, reads);
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
    assert!(renderer.output_last_valid_frames.is_empty());
    assert_eq!(session.presented_frames(), 0);
    assert_eq!(session.hard_blackout_frames_presented(), 0);
    drop(blackout);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_display_runtime_install_requires_the_exact_existing_context() {
    let mut session = headless_session();
    let mut foreign = headless_session();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    session.install_renderer(&mut renderer).unwrap();
    let context = Arc::clone(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_context()
            .unwrap(),
    );
    session.install_renderer(&mut renderer).unwrap();
    assert!(Arc::ptr_eq(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_context()
            .unwrap(),
        &context
    ));
    assert!(foreign.install_renderer(&mut renderer).is_err());
    assert!(Arc::ptr_eq(
        renderer
            .isf_runtime
            .as_ref()
            .unwrap()
            .as_ref()
            .unwrap()
            .native_context()
            .unwrap(),
        &context
    ));
    // A separately created OFF runtime is retained intact and rejected; the
    // ON binding must neither adopt it nor silently create another runtime.
    let mut standalone = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    standalone.isf_runtime = Some(Ok(crate::IsfGpuRuntime::new().unwrap()));
    assert!(session.install_renderer(&mut standalone).is_err());
    assert!(standalone
        .isf_runtime
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_context()
        .is_none());
    renderer.poison_native_gpu_runtime("test terminal renderer");
    assert!(session.install_renderer(&mut renderer).is_err());
    assert!(
        matches!(renderer.isf_runtime.as_ref(), Some(Err(error)) if error.message == "test terminal renderer")
    );
    drop(standalone);
    drop(context);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
    foreign.begin_stop().unwrap();
    reap_started(&mut foreign);
}

#[test]
fn native_gpu_output_display_input_keys_are_stable_distinct_and_reject_explicit_collision() {
    // Identity-only assertions; the session still owns a real headless broker.
    let mut session = headless_session();
    let snapshot = scene(3);
    let legacy_one = crate::legacy_layer_input(&snapshot.layers[0], EXTENT.0, EXTENT.1, None);
    let legacy_two = crate::legacy_layer_input(&snapshot.layers[1], EXTENT.0, EXTENT.1, None);
    let mut explicit = crate::legacy_layer_input(&snapshot.layers[2], EXTENT.0, EXTENT.1, None);
    explicit.key = protocol::VideoRenderInputKey {
        project_render_epoch: EPOCH,
        input_id: protocol::VideoRenderInputId(1),
    };
    let authored = vec![legacy_one, explicit.clone(), legacy_two];
    let mut first = authored.clone();
    let budget = session.owner.budget_snapshot();
    session.bind_input_keys(&mut first, EPOCH).unwrap();
    let keys = first.iter().map(|input| input.key).collect::<Vec<_>>();
    assert_eq!(keys[1], explicit.key);
    assert!(keys.iter().all(|key| key.project_render_epoch == EPOCH));
    for (index, key) in keys.iter().enumerate() {
        assert!(!keys[..index].contains(key));
    }
    assert_ne!(keys[0], protocol::VideoRenderInputKey::LEGACY);
    assert_ne!(keys[2], protocol::VideoRenderInputKey::LEGACY);
    let next = session.next_input_id;
    let mut again = authored.clone();
    session.bind_input_keys(&mut again, EPOCH).unwrap();
    assert_eq!(
        again.iter().map(|input| input.key).collect::<Vec<_>>(),
        keys
    );
    assert_eq!(
        session.next_input_id, next,
        "stable bindings do not issue another identity"
    );
    let mut changed_source = authored;
    changed_source[0].request.source.path = Some("replacement.mp4".into());
    session.bind_input_keys(&mut changed_source, EPOCH).unwrap();
    assert_eq!(changed_source[0].key, keys[0]);
    assert_eq!(
        session
            .input_bindings
            .iter()
            .find(|binding| binding.key == keys[0])
            .unwrap()
            .source_generation,
        2
    );
    let count = session.input_bindings.len();
    explicit.key = keys[0];
    let mut collision = vec![explicit];
    let rejected = session.bind_input_keys(&mut collision, EPOCH).unwrap_err();
    assert!(rejected.contains("explicit input collides"));
    assert_eq!(session.input_bindings.len(), count);
    assert_eq!(session.next_input_id, next);
    assert_eq!(
        session.owner.budget_snapshot(),
        budget,
        "identity allocation has no GPU work"
    );
    session.begin_stop().unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_display_stop_recovers_panicked_cpu_broker_and_reaps_all_aliases() {
    let mut session = headless_session();
    let generation = session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    session.install_renderer(&mut renderer).unwrap();
    let snapshot = scene(1);
    let plan = crate::build_video_output_render_plan(&snapshot, OUTPUT).unwrap();
    let context = session.owner.context().id();
    let owner = session.owner.take();
    let request = NativeFrameId {
        output_id: OUTPUT,
        project_epoch: EPOCH,
        request_sequence: 1,
        geometry_generation: generation,
    };
    let mut retained_input = None;
    let mut retained_pixels = None;
    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _ = renderer.with_native_cpu_owner::<()>(owner, request, |renderer| {
            let artistic = renderer.prepare_native_cpu_artistic_with_plan(
                &snapshot,
                VideoEffectRenderContext {
                    clip_runtime: &protocol::VideoClipRuntimeSnapshot::default(),
                    project_render_epoch: EPOCH,
                    audio_frame: None,
                },
                &VideoLayerTransitionRuntimeSnapshot::default(),
                &plan,
                EXTENT.0,
                EXTENT.1,
                &|| false,
            )?;
            assert!(matches!(
                artistic.payload,
                VideoOutputArtisticPayload::Frame(_)
            ));
            let whole = renderer.runtime.queues[0].native_cpu_owners[0]
                .as_ref()
                .unwrap();
            assert_eq!(
                whole.charge.as_ref().unwrap().kind(),
                ChargeKind::ActiveCpuInput
            );
            retained_input = Some(Arc::downgrade(whole));
            retained_pixels = Some(Arc::downgrade(&whole.frame));
            renderer.isf_frame_inputs.sampled_layers.insert(1);
            renderer.publish_isf_layer_frame(1, 0, true, None);
            assert!(renderer.isf_frame_inputs.layers.get(&1).is_some());
            panic!("deliberate panic while the affine CPU bridge owns the sole broker");
        });
    }));
    assert!(panic.is_err());
    assert!(session.owner.0.is_none());
    assert!(renderer.native_cpu_bridge.is_some());
    assert!(retained_input.as_ref().unwrap().upgrade().is_some());
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    assert!(renderer.native_cpu_bridge.is_none());
    assert!(renderer.runtime.queues.is_empty());
    assert!(renderer.output_last_valid_frames.is_empty());
    assert!(renderer.isf_frame_inputs.layers.is_empty());
    assert!(renderer.isf_frame_inputs.sampled_layers.is_empty());
    assert!(matches!(renderer.isf_runtime, Some(Err(_))));
    assert!(retained_input.unwrap().upgrade().is_none());
    assert!(retained_pixels.unwrap().upgrade().is_none());
    assert_eq!(session.owner.context().id(), context);
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_display_creation_failure_keeps_partial_real_session_until_reaped() {
    // Exercise the constructor's failure carrier with real initialized G/P
    // resources; the failure itself is injected after successful headless setup.
    let mut session = headless_session();
    session.compositor = Some(NativeGpuCompositor::from_shared(&mut session.owner).unwrap());
    let context = session.owner.context().id();
    let weak = Arc::downgrade(session.owner.context());
    let failure =
        session.retain_creation_failure("injected failure after compositor initialization".into());
    let (message, retained) = failure.into_parts();
    assert!(message.contains("after compositor initialization"));
    let mut retained = retained.expect("partial construction must return its whole session owner");
    assert!(weak.upgrade().is_some());
    assert_eq!(retained.owner.context().id(), context);
    assert!(retained.compositor.is_some());
    assert_eq!(retained.terminal_fault.as_deref(), Some(message.as_str()));
    assert!(retained.reserve_geometry(EXTENT.0, EXTENT.1).is_err());
    retained.begin_stop().unwrap();
    reap_started(&mut retained);
    drop(retained);
    assert!(weak.upgrade().is_none());
}

#[test]
fn native_gpu_output_display_foreign_cpu_and_blackout_frames_are_rejected_before_sink() {
    let mut source = headless_session();
    let mut target = headless_session();
    assert_eq!(source.output_id(), target.output_id());
    assert_ne!(source.owner.context().id(), target.owner.context().id());
    source.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    target.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    // This headless receiver has no presenter. Clear only the reservation guard
    // to isolate context identity: if that guard were missing, the error would
    // be presenter-unavailable instead of the required foreign-frame rejection.
    target.reserved_geometry = None;
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let mut snapshot = scene(1);
    let cpu = prepare(&mut source, &mut renderer, &snapshot);
    assert_eq!(cpu.evidence().freshness, VideoOutputRenderFreshness::Fresh);
    let weak = match &cpu.content {
        ResidentDisplayContent::Cpu(cpu) => Arc::downgrade(&cpu.owner),
        _ => panic!("a one-layer structural fallback must return its actual charged CPU frame"),
    };
    let authority = PresentProbe::default();
    let before = target.owner.budget_snapshot();
    let error = target.present(cpu, &authority, &|| false).unwrap_err();
    assert!(error.contains("own configured session and frozen frame identity"));
    assert!(weak.upgrade().is_none());
    snapshot.outputs[0].opacity = 0.0;
    let blackout = prepare(&mut source, &mut renderer, &snapshot);
    assert!(matches!(
        blackout.content,
        ResidentDisplayContent::HardBlackout(_)
    ));
    let error = target.present(blackout, &authority, &|| false).unwrap_err();
    assert!(error.contains("own configured session and frozen frame identity"));
    assert_eq!(
        authority.calls.get(),
        0,
        "foreign frames stop before acquire/present authority or sink work"
    );
    assert_eq!(target.owner.budget_snapshot(), before);
    assert_eq!(target.presented_frames(), 0);
    assert_eq!(target.hard_blackout_frames_presented(), 0);
    assert_eq!(source.presented_frames(), 0);
    assert_eq!(renderer.frame_provider.reads, 1);
    source.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut source);
    target.begin_stop().unwrap();
    reap_started(&mut target);
}
