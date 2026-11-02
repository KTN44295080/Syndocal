//! Follow budget pressure must skip output without retiring a healthy owner.
use super::*;
use crate::native_gpu_budget::{FrameReservation, NATIVE_MANAGED_LIMIT};
use crate::VideoPreviewError;

fn follow<P: VideoFrameProvider>(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<P>,
    outgoing: &VideoSnapshot,
    incoming: &VideoSnapshot,
    cancellation: &dyn VideoRenderCancellation,
    authority: &impl NativeResidentPresentationAuthority,
) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
    session.prepare_follow_output(
        renderer,
        outgoing,
        incoming,
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
        cancellation,
        authority,
    )
}

fn pressure(session: &mut NativeResidentDisplaySession, headroom: u64) -> FrameReservation {
    session.owner.poll_retirement().unwrap();
    let live = session.owner.budget_snapshot().managed_bytes();
    let mut plan = crate::native_gpu_budget::NativeAllocationPlan::new(
        crate::native_gpu_contract::NativeFrameId {
            output_id: OUTPUT,
            project_epoch: EPOCH,
            request_sequence: 900,
            geometry_generation: session.geometry_generation(),
        },
    );
    plan.add_new(
        ChargeKind::SerialCpuAdmission,
        NATIVE_MANAGED_LIMIT - live - headroom,
    )
    .unwrap();
    let quote = session.owner.quote(&plan).unwrap();
    session.owner.reserve(quote).unwrap()
}

fn distinct_sides() -> (VideoSnapshot, VideoSnapshot) {
    let outgoing = scene(1);
    let mut incoming = outgoing.clone();
    incoming.layers[0].id = 2;
    incoming.layers[0].source.path = Some("incoming.mp4".into());
    incoming.compositions[0].layer_ids = vec![2];
    (outgoing, incoming)
}

#[test]
fn native_gpu_output_follow_repeated_budget_refusal_releases_only_queues_and_recovers() {
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let outgoing = scene(1);
    let mut incoming = outgoing.clone();
    let seed = follow(
        &mut session,
        &mut renderer,
        &outgoing,
        &incoming,
        &|| false,
        &Permit,
    )
    .unwrap();
    let cached = match &seed.content {
        ResidentDisplayContent::Cpu(cpu) => Arc::downgrade(&cpu.owner),
        _ => panic!("Follow must own actual charged CPU output"),
    };
    drop(seed);
    let queued = renderer
        .runtime
        .queues
        .iter()
        .map(|queue| queue.len())
        .sum::<usize>();
    assert_eq!(queued, 1);
    let queue_owner = renderer
        .runtime
        .queues
        .iter()
        .find_map(|queue| {
            queue
                .native_cpu_owners
                .iter()
                .flatten()
                .next()
                .map(Arc::downgrade)
        })
        .expect("queued source must own its allocation");
    let hold = pressure(&mut session, 1);
    for refusal in 1..=2 {
        let error = follow(
            &mut session,
            &mut renderer,
            &outgoing,
            &incoming,
            &|| false,
            &Permit,
        )
        .err()
        .expect("pressure must refuse a real allocation");
        assert!(
            matches!(error, NativeResidentPrepareError::BudgetDeferred { .. }),
            "{error:?}"
        );
        assert_eq!(session.budget_refusals, refusal);
        assert_eq!(session.owner.state(), GpuOwnerState::Active);
        assert!(!session.owner.has_unfinished_logical_frame());
        assert!(session.terminal_fault.is_none());
        assert!(
            cached.upgrade().is_some(),
            "budget pressure preserves Follow LastValid"
        );
        assert_eq!(session.presented_frames(), 0);
        let remaining = renderer
            .runtime
            .queues
            .iter()
            .map(|queue| queue.len())
            .sum::<usize>();
        assert_eq!(remaining, if refusal == 1 { queued } else { 0 });
        assert_eq!(queue_owner.upgrade().is_some(), refusal == 1);
    }
    session.owner.discard_unsubmitted_reservation(hold).unwrap();
    session.owner.poll_retirement().unwrap();
    // Recovery must also admit a genuinely different incoming source.
    incoming.layers[0].source.path = Some("incoming-after-pressure.mp4".into());
    incoming.layers[0].state.position_ms = 1000;
    let recovered = follow(
        &mut session,
        &mut renderer,
        &outgoing,
        &incoming,
        &|| false,
        &Permit,
    )
    .unwrap();
    assert_eq!(
        recovered.evidence().freshness,
        VideoOutputRenderFreshness::Fresh
    );
    assert_eq!(session.budget_refusals, 0);
    assert_eq!(session.path_evidence().budget_declines, 2);
    assert!(
        cached.upgrade().is_none(),
        "a successful new output replaces the prior owner"
    );
    drop(recovered);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_follow_budget_after_outgoing_scope_does_not_rerun_or_publish_it() {
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let (mut outgoing, mut incoming) = distinct_sides();
    outgoing.layers[0].isf_effect = Some(crate::builtin_isf_effect("invert").unwrap().unwrap());
    const LARGE: &str = r#"/*{"ISFVSN":"2","INPUTS":[{"NAME":"inputImage","TYPE":"image"}],"PASSES":[{"TARGET":"large","WIDTH":4096,"HEIGHT":4096},{}]}*/
void main(){gl_FragColor=IMG_THIS_NORM_PIXEL(inputImage);}"#;
    let shader = crate::prepare_isf_shader(LARGE).unwrap();
    incoming.layers[0].isf_effect = Some(crate::isf_effect_from_prepared(
        "incoming needs a larger envelope".into(),
        LARGE,
        None,
        &shader,
    ));
    session.install_renderer(&mut renderer).unwrap();
    let before = renderer
        .isf_runtime
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_frame_index();
    let hold = pressure(&mut session, 1024 * 1024);
    let error = follow(
        &mut session,
        &mut renderer,
        &outgoing,
        &incoming,
        &|| false,
        &Permit,
    )
    .err()
    .expect("incoming scoped allocation must not fit");
    assert!(
        matches!(error, NativeResidentPrepareError::BudgetDeferred { .. }),
        "{error:?}"
    );
    let after = renderer
        .isf_runtime
        .as_ref()
        .unwrap()
        .as_ref()
        .unwrap()
        .native_frame_index();
    assert_eq!(
        after,
        before + 1,
        "the completed outgoing scope executes once; refused incoming never executes"
    );
    assert!(session.follow_last_valid.is_none());
    assert_eq!(session.presented_frames(), 0);
    assert_eq!(session.owner.state(), GpuOwnerState::Active);
    session.owner.discard_unsubmitted_reservation(hold).unwrap();
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_follow_caught_budget_cause_never_relabels_a_native_fault() {
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    session.install_renderer(&mut renderer).unwrap();
    let request = session
        .next_request(
            &crate::build_video_output_render_plan(&scene(1), OUTPUT).unwrap(),
            EPOCH,
            EXTENT,
        )
        .unwrap();
    let owner = session.owner.take();
    let (owner, result, refusal) =
        renderer.with_native_cpu_owner_budgeted::<()>(owner, request.frame, |renderer| {
            let budget = renderer.reserve_native_cpu_scratch(3, 2, 1 << 25, None);
            assert!(budget.is_err(), "exercise a genuine typed broker refusal");
            Err(VideoPreviewError::Runtime(
                crate::VideoRuntimeError::NativeGpu("injected native fault".into()),
            ))
        });
    session.owner.restore(owner);
    assert!(
        matches!(result, Err(VideoPreviewError::Runtime(crate::VideoRuntimeError::NativeGpu(ref message))) if message == "injected native fault")
    );
    assert!(
        refusal.is_none(),
        "a caught admission failure cannot relabel a different failure"
    );
    assert!(session.deferred_budget.is_none());
    assert_eq!(session.budget_refusals, 0);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_follow_control_rejections_do_not_become_budget_deferrals() {
    struct Revoked;
    impl NativeResidentPresentationAuthority for Revoked {
        fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
            NativeResidentPresentDecision::Revoked
        }
    }
    let mut session = headless_session();
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider::default(),
    );
    let (outgoing, incoming) = distinct_sides();
    let hold = pressure(&mut session, 1);
    let stopped = follow(
        &mut session,
        &mut renderer,
        &outgoing,
        &incoming,
        &|| true,
        &Permit,
    )
    .err()
    .unwrap();
    assert!(matches!(
        stopped,
        NativeResidentPrepareError::Cancelled { .. }
    ));
    let revoked = follow(
        &mut session,
        &mut renderer,
        &outgoing,
        &incoming,
        &|| false,
        &Revoked,
    )
    .err()
    .unwrap();
    assert!(matches!(
        revoked,
        NativeResidentPrepareError::Revoked { .. }
    ));
    assert_eq!(session.budget_refusals, 0);
    assert_eq!(renderer.frame_provider.reads, 0);
    session.owner.discard_unsubmitted_reservation(hold).unwrap();
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}

#[test]
fn native_gpu_output_follow_partial_side_control_changes_never_defer_budget() {
    use std::sync::atomic::{AtomicBool, Ordering};
    struct SwitchingProvider {
        inner: Provider,
        changed: Arc<AtomicBool>,
    }
    impl VideoFrameProvider for SwitchingProvider {
        fn retain_layers(&mut self, layers: &[protocol::VideoLayerId]) {
            self.inner.retain_layers(layers);
        }
        fn frame_for_layer(
            &mut self,
            layer: &protocol::VideoLayerSummary,
            width: u32,
            height: u32,
        ) -> Result<VideoFrame, VideoFrameProviderError> {
            let frame = self.inner.frame_for_layer(layer, width, height)?;
            if layer.id == 2 {
                self.changed.store(true, Ordering::Release);
            }
            Ok(frame)
        }
    }
    struct ChangingAuthority(Arc<AtomicBool>);
    impl NativeResidentPresentationAuthority for ChangingAuthority {
        fn revalidate(&self, _: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
            if self.0.load(Ordering::Acquire) {
                NativeResidentPresentDecision::Revoked
            } else {
                NativeResidentPresentDecision::Permit
            }
        }
    }
    for stop in [true, false] {
        let mut session = headless_session();
        session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
        let changed = Arc::new(AtomicBool::new(false));
        let mut renderer = VideoPreviewRenderer::with_frame_provider(
            VideoRuntimeConfig::default(),
            SwitchingProvider {
                inner: Provider::default(),
                changed: Arc::clone(&changed),
            },
        );
        let (mut outgoing, incoming) = distinct_sides();
        outgoing.layers[0].isf_effect = Some(crate::builtin_isf_effect("invert").unwrap().unwrap());
        let cancel = || stop && changed.load(Ordering::Acquire);
        let authority = ChangingAuthority(Arc::clone(&changed));
        let error = follow(
            &mut session,
            &mut renderer,
            &outgoing,
            &incoming,
            &cancel,
            &authority,
        )
        .err()
        .expect("a changed control boundary must reject output");
        assert!(
            if stop {
                matches!(error, NativeResidentPrepareError::Cancelled { .. })
            } else {
                matches!(error, NativeResidentPrepareError::Revoked { .. })
            },
            "{error:?}"
        );
        assert_eq!(
            renderer
                .isf_runtime
                .as_ref()
                .unwrap()
                .as_ref()
                .unwrap()
                .native_frame_index(),
            1,
            "the outgoing scope completed before the incoming acquisition changed authority"
        );
        assert_eq!(session.budget_refusals, 0);
        assert_eq!(session.path_evidence().budget_declines, 0);
        assert_eq!(session.presented_frames(), 0);
        session.begin_stop_with_renderer(&mut renderer).unwrap();
        reap_started(&mut session);
    }
}

#[test]
fn native_gpu_output_follow_budget_cleanup_preserves_a_complete_cpu_handover() {
    struct ResidentProvider(Provider);
    impl VideoFrameProvider for ResidentProvider {
        fn retain_layers(&mut self, layers: &[protocol::VideoLayerId]) {
            self.0.retain_layers(layers);
        }
        fn frame_for_layer(
            &mut self,
            layer: &protocol::VideoLayerSummary,
            width: u32,
            height: u32,
        ) -> Result<VideoFrame, VideoFrameProviderError> {
            self.0.frame_for_layer(layer, width, height)
        }
        fn layer_frame_inputs(
            &self,
            layer: &protocol::VideoLayerSummary,
            width: u32,
            height: u32,
        ) -> Vec<crate::VideoRenderInput> {
            vec![crate::legacy_layer_input(layer, width, height, None)]
        }
    }
    let mut session = headless_session();
    session.compositor = Some(NativeGpuCompositor::from_shared(&mut session.owner).unwrap());
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    session.reserved_geometry = None;
    let mut renderer = VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        ResidentProvider(Provider::default()),
    );
    let mut snapshot = scene(1);
    snapshot.layers[0].isf_effect = Some(crate::builtin_isf_effect("invert").unwrap().unwrap());
    let resident = session
        .prepare_output(
            &mut renderer,
            &snapshot,
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
        .unwrap();
    assert!(matches!(
        resident.content,
        ResidentDisplayContent::Resident(_)
    ));
    drop(resident);
    session
        .materialize_history_for_cpu(
            renderer.isf_runtime.as_mut().unwrap().as_mut().unwrap(),
            &|| false,
        )
        .unwrap();
    assert!(matches!(
        session.history,
        Some(ResidentHistoryState::Handover(_))
    ));
    let hold = pressure(&mut session, 1);
    let mut plan = crate::native_gpu_budget::NativeAllocationPlan::new(
        crate::native_gpu_contract::NativeFrameId {
            output_id: OUTPUT,
            project_epoch: EPOCH,
            request_sequence: 901,
            geometry_generation: session.geometry_generation(),
        },
    );
    plan.add_new(ChargeKind::CpuSnapshot, 24).unwrap();
    let reason = session
        .owner
        .quote(&plan)
        .err()
        .expect("real broker quote must refuse");
    assert!(matches!(reason, NativeGpuDecline::BudgetExceeded { .. }));
    session.budget_refusals = 1;
    assert!(session.defer_budget_output(&mut renderer, reason).is_err());
    assert!(
        session.history.is_none(),
        "the complete handover is published rather than dropped"
    );
    assert_eq!(
        renderer
            .runtime
            .queues
            .iter()
            .map(|q| q.len())
            .sum::<usize>(),
        1,
        "the actual materialized history must reach its canonical queue"
    );
    assert!(
        renderer
            .runtime
            .queues
            .iter()
            .any(|q| q.native_owners.iter().any(Option::is_some)),
        "publication preserves the complete history entry owner"
    );
    session.owner.discard_unsubmitted_reservation(hold).unwrap();
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    reap_started(&mut session);
}
