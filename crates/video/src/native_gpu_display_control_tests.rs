//! Public preparation-control regressions with a real headless broker.
//!
//! No surface, Ready token, completion proof, or Presented receipt is invented.
//! The pending-work case submits a real charged buffer clear and retains its
//! transaction plus allocation until the ordinary Stop path actually reaps it.
use super::present::ResidentDisplayContent;
use super::*;
use crate::native_gpu_budget::{ChargeKind, NativeAllocationPlan};
use crate::native_gpu_contract::{GpuOwnerState, GpuPhase};
use crate::{
    VideoEffectRenderContext, VideoFrame, VideoFrameProviderError, VideoOutputRenderFreshness,
    VideoPixelFormat, VideoRenderInput, VideoRuntimeConfig,
};
use protocol::{VideoLayerTransitionRuntimeSnapshot, VideoSnapshot};
use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

const OUTPUT: u64 = 101;
const EPOCH: u64 = 7;
const EXTENT: (u32, u32) = (3, 2);

struct Authority {
    decision: Cell<NativeResidentPresentDecision>,
    calls: Cell<usize>,
    revoke_on_call: Option<usize>,
}

impl Authority {
    fn new(decision: NativeResidentPresentDecision) -> Self {
        Self {
            decision: Cell::new(decision),
            calls: Cell::new(0),
            revoke_on_call: None,
        }
    }
}

impl NativeResidentPresentationAuthority for Authority {
    fn revalidate(&self, boundary: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        assert_eq!(
            boundary,
            NativeResidentPresentBoundary::BeforePrepare,
            "preparation must never acquire or physically present"
        );
        let call = self.calls.get() + 1;
        self.calls.set(call);
        if self.revoke_on_call.is_some_and(|first| call >= first) {
            NativeResidentPresentDecision::Revoked
        } else {
            self.decision.get()
        }
    }
}

struct Provider {
    authority: Rc<Authority>,
    reads: usize,
    pulse_revocation: bool,
    fail_and_revoke: bool,
    observed_rejection: bool,
    observed_restored_permit: bool,
}

impl Provider {
    fn finish_acquisition(
        &mut self,
        frame: VideoFrame,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Arc<VideoFrame>, VideoFrameProviderError> {
        if self.pulse_revocation {
            self.pulse_revocation = false;
            self.authority
                .decision
                .set(NativeResidentPresentDecision::Revoked);
            self.observed_rejection = cancellation.is_cancelled();
            assert!(self.observed_rejection);
            self.authority
                .decision
                .set(NativeResidentPresentDecision::Permit);
            self.observed_restored_permit = !cancellation.is_cancelled();
            assert!(self.observed_restored_permit);
            // The completed decode belongs to the rejected tick. A later
            // Permit cannot undo the cancellation this provider already saw.
            return Err(VideoFrameProviderError::Cancelled);
        }
        if cancellation.is_cancelled() {
            return Err(VideoFrameProviderError::Cancelled);
        }
        Ok(Arc::new(frame))
    }
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
        if self.fail_and_revoke {
            self.fail_and_revoke = false;
            let failure = VideoFrameProviderError::MissingStillImagePath {
                layer_id: layer.id,
                label: layer.label.clone(),
            };
            self.authority
                .decision
                .set(NativeResidentPresentDecision::Revoked);
            return Err(failure);
        }
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

    fn layer_frame_inputs(
        &self,
        layer: &protocol::VideoLayerSummary,
        width: u32,
        height: u32,
    ) -> Vec<VideoRenderInput> {
        vec![crate::legacy_layer_input(layer, width, height, None)]
    }

    fn frame_for_input_shared_with_cancellation(
        &mut self,
        input: &VideoRenderInput,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<Arc<VideoFrame>, VideoFrameProviderError> {
        if cancellation.is_cancelled() {
            return Err(VideoFrameProviderError::Cancelled);
        }
        let frame = self.frame_for_input(input)?;
        self.finish_acquisition(frame, cancellation)
    }
}

fn renderer(authority: Rc<Authority>) -> VideoPreviewRenderer<Provider> {
    VideoPreviewRenderer::with_frame_provider(
        VideoRuntimeConfig::default(),
        Provider {
            authority,
            reads: 0,
            pulse_revocation: false,
            fail_and_revoke: false,
            observed_rejection: false,
            observed_restored_permit: false,
        },
    )
}

fn headless_session() -> NativeResidentDisplaySession {
    let factory = NativeGpuFactory::new();
    let owner = factory
        .create_headless(OUTPUT, NativeGpuRequirements::default())
        .expect("preparation controls require a real headless adapter");
    let mut session = NativeResidentDisplaySession {
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
    };
    session.reserve_geometry(EXTENT.0, EXTENT.1).unwrap();
    session
}

fn scene() -> VideoSnapshot {
    let source = crate::isf_native_gpu::tests::IDENTITY;
    let shader = crate::prepare_isf_shader(source).unwrap();
    let effect = crate::isf_effect_from_prepared("control fixture".into(), source, None, &shader);
    VideoSnapshot {
        layers: (1..=2)
            .map(|id| protocol::VideoLayerSummary {
                id,
                label: format!("control layer {id}"),
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
                isf_effect: Some(effect.clone()),
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
            label: "control".into(),
            layer_ids: vec![1, 2],
            timeline_layer_ids: Vec::new(),
            output_ids: vec![OUTPUT],
        }],
        outputs: vec![protocol::VideoOutputSummary {
            id: OUTPUT,
            label: "headless control".into(),
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

fn prepare(
    session: &mut NativeResidentDisplaySession,
    renderer: &mut VideoPreviewRenderer<Provider>,
    snapshot: &VideoSnapshot,
    follow: bool,
    cancellation: &dyn VideoRenderCancellation,
    authority: &Authority,
) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
    let clips = protocol::VideoClipRuntimeSnapshot::default();
    let transitions = VideoLayerTransitionRuntimeSnapshot::default();
    let context = VideoEffectRenderContext {
        clip_runtime: &clips,
        project_render_epoch: EPOCH,
        audio_frame: None,
    };
    if follow {
        session.prepare_follow_output(
            renderer,
            snapshot,
            snapshot,
            context,
            &transitions,
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
    } else {
        session.prepare_output(
            renderer,
            snapshot,
            context,
            &transitions,
            EXTENT,
            cancellation,
            authority,
        )
    }
}

fn assert_no_execution(
    session: &NativeResidentDisplaySession,
    renderer: &VideoPreviewRenderer<Provider>,
) {
    assert_eq!(renderer.isf_shader_cache_clock, 0);
    if let Some(Ok(runtime)) = renderer.isf_runtime.as_ref() {
        assert_eq!(runtime.native_frame_index(), 0);
    }
    assert_eq!(session.presented_frames(), 0);
    assert_eq!(session.hard_blackout_frames_presented(), 0);
    assert!(renderer.output_last_valid_frames.is_empty());
    assert!(session.follow_last_valid.is_none());
}

fn reap_started(session: &mut NativeResidentDisplaySession) {
    let started = Instant::now();
    while !session.poll_stop().unwrap() {
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "real control cleanup failed to reap"
        );
        std::thread::yield_now();
    }
    assert_eq!(session.owner.state(), GpuOwnerState::Reaped);
    assert_eq!(session.owner.budget_snapshot().managed_bytes(), 0);
    assert_eq!(session.owner.budget_snapshot().reservation_count, 0);
}

fn stop(session: &mut NativeResidentDisplaySession, renderer: &mut VideoPreviewRenderer<Provider>) {
    session.begin_stop_with_renderer(renderer).unwrap();
    reap_started(session);
}

#[test]
fn native_gpu_output_display_stop_observes_reaped_by_another_cleanup_consumer() {
    for decision in [
        NativeResidentPresentDecision::Permit,
        NativeResidentPresentDecision::Revoked,
    ] {
        let authority = Rc::new(Authority::new(decision));
        let mut renderer = renderer(authority);
        let mut session = headless_session();
        session.begin_stop_with_renderer(&mut renderer).unwrap();
        // P's retirement poll runs before the facade's own poll in production.
        // Reproduce that real owner transition without fabricating a receipt.
        assert!(matches!(
            session.owner.poll_retirement().unwrap(),
            RetirementProgress::Reaped
        ));
        assert!(session.poll_stop().unwrap());
        assert!(
            session.poll_stop().unwrap(),
            "terminal cleanup stays complete"
        );
    }
}

#[test]
fn native_gpu_output_display_stop_refunds_renderer_rejected_admission() {
    let authority = Rc::new(Authority::new(NativeResidentPresentDecision::Permit));
    let mut renderer = renderer(authority);
    let mut session = headless_session();
    let frame = NativeFrameId {
        output_id: OUTPUT,
        project_epoch: EPOCH,
        request_sequence: 1,
        geometry_generation: session.geometry_generation,
    };
    let mut plan = NativeAllocationPlan::new(frame);
    plan.add_new(ChargeKind::CpuSnapshot, 64).unwrap();
    let quote = session.owner.quote(&plan).unwrap();
    let reservation = session.owner.reserve(quote).unwrap();
    renderer.native_cpu_retained_reservations.push(reservation);
    assert_eq!(session.owner.budget_snapshot().reservation_count, 1);
    session.begin_stop_with_renderer(&mut renderer).unwrap();
    assert!(renderer.native_cpu_retained_reservations.is_empty());
    assert_eq!(session.unsubmitted.len(), 1);
    reap_started(&mut session);
    assert!(session.unsubmitted.is_empty());
}

#[test]
fn native_gpu_output_display_prepare_before_prepare_revocation_has_zero_decode_fx_and_healthy_owner(
) {
    for follow in [false, true] {
        let authority = Rc::new(Authority::new(NativeResidentPresentDecision::Revoked));
        let mut renderer = renderer(Rc::clone(&authority));
        let mut session = headless_session();
        let result = prepare(
            &mut session,
            &mut renderer,
            &scene(),
            follow,
            &|| false,
            &authority,
        );
        assert!(matches!(
            result,
            Err(NativeResidentPrepareError::Revoked {
                retirement_required: false
            })
        ));
        assert_eq!(authority.calls.get(), 1);
        assert_eq!(renderer.frame_provider.reads, 0);
        assert!(
            renderer.isf_runtime.is_none(),
            "early revocation must precede runtime installation"
        );
        assert_no_execution(&session, &renderer);
        assert_eq!(session.request_sequence, 0);
        assert_eq!(session.owner.state(), GpuOwnerState::Active);
        assert!(!session.owner.has_unfinished_logical_frame());
        assert_eq!(session.owner.budget_snapshot().managed_bytes(), 0);
        stop(&mut session, &mut renderer);
    }
}

#[test]
fn native_gpu_output_display_prepare_initial_stop_is_typed_without_authority_or_decode() {
    for follow in [false, true] {
        let authority = Rc::new(Authority::new(NativeResidentPresentDecision::Permit));
        let mut renderer = renderer(Rc::clone(&authority));
        let mut session = headless_session();
        let result = prepare(
            &mut session,
            &mut renderer,
            &scene(),
            follow,
            &|| true,
            &authority,
        );
        assert!(matches!(
            result,
            Err(NativeResidentPrepareError::Cancelled {
                retirement_required: false
            })
        ));
        assert_eq!(authority.calls.get(), 0);
        assert_eq!(renderer.frame_provider.reads, 0);
        assert_no_execution(&session, &renderer);
        assert_eq!(session.owner.state(), GpuOwnerState::Active);
        assert!(!session.owner.has_unfinished_logical_frame());
        stop(&mut session, &mut renderer);
    }
}

#[test]
fn native_gpu_output_display_prepare_first_revocation_latches_and_only_a_new_tick_can_run() {
    let authority = Rc::new(Authority::new(NativeResidentPresentDecision::Permit));
    let mut renderer = renderer(Rc::clone(&authority));
    renderer.frame_provider.pulse_revocation = true;
    let mut session = headless_session();
    let mut snapshot = scene();
    let result = prepare(
        &mut session,
        &mut renderer,
        &snapshot,
        false,
        &|| false,
        &authority,
    );
    assert!(renderer.frame_provider.observed_rejection);
    assert!(renderer.frame_provider.observed_restored_permit);
    assert_eq!(
        authority.decision.get(),
        NativeResidentPresentDecision::Permit
    );
    assert!(matches!(result, Err(NativeResidentPrepareError::Revoked { retirement_required: false })),
        "restored live authority must not relabel the already revoked preparation as a fault or success");
    assert_eq!(renderer.frame_provider.reads, 1);
    assert_no_execution(&session, &renderer);
    assert_eq!(session.request_sequence, 1);
    assert_eq!(session.owner.state(), GpuOwnerState::Active);
    assert!(!session.owner.has_unfinished_logical_frame());

    // A genuinely new output plan applies the user's new blackout state. It
    // must not inherit the prior invocation's latch or re-use its decoded frame.
    snapshot.outputs[0].blackout = true;
    let next = prepare(
        &mut session,
        &mut renderer,
        &snapshot,
        false,
        &|| false,
        &authority,
    )
    .unwrap();
    assert!(matches!(
        next.content,
        ResidentDisplayContent::HardBlackout(_)
    ));
    assert_eq!(next.evidence().freshness, VideoOutputRenderFreshness::Fresh);
    assert!(next.evidence().error.is_none());
    assert_eq!(session.request_sequence, 2);
    assert_eq!(renderer.frame_provider.reads, 1);
    assert_no_execution(&session, &renderer);
    drop(next);
    stop(&mut session, &mut renderer);
}

#[test]
fn native_gpu_output_display_prepare_plan_fault_is_not_relabelled_by_a_later_authority_probe() {
    for follow in [false, true] {
        let authority = Rc::new(Authority {
            revoke_on_call: Some(3),
            ..Authority::new(NativeResidentPresentDecision::Permit)
        });
        let mut renderer = renderer(Rc::clone(&authority));
        let mut session = headless_session();
        let mut snapshot = scene();
        snapshot.outputs.clear();
        let result = prepare(
            &mut session,
            &mut renderer,
            &snapshot,
            follow,
            &|| false,
            &authority,
        );
        assert!(
            matches!(result, Err(NativeResidentPrepareError::Fault(message)) if message.contains("plan rejected"))
        );
        assert_eq!(
            authority.calls.get(),
            2,
            "the failed plan must not trigger a new final authority probe"
        );
        assert_eq!(
            authority.revalidate(NativeResidentPresentBoundary::BeforePrepare),
            NativeResidentPresentDecision::Revoked,
            "a later probe really would revoke"
        );
        assert_eq!(renderer.frame_provider.reads, 0);
        assert_no_execution(&session, &renderer);
        assert_eq!(session.owner.state(), GpuOwnerState::Active);
        assert!(!session.owner.has_unfinished_logical_frame());
        stop(&mut session, &mut renderer);
    }
}

#[test]
fn native_gpu_output_display_prepare_decode_fault_keeps_its_error_when_authority_changes_after_failure(
) {
    let authority = Rc::new(Authority::new(NativeResidentPresentDecision::Permit));
    let mut renderer = renderer(Rc::clone(&authority));
    renderer.frame_provider.fail_and_revoke = true;
    let mut session = headless_session();
    let result = prepare(
        &mut session,
        &mut renderer,
        &scene(),
        false,
        &|| false,
        &authority,
    );
    assert!(
        matches!(result, Err(NativeResidentPrepareError::Fault(message))
        if message.contains("native raw input acquisition failed") && message.contains("MissingStillImagePath"))
    );
    assert_eq!(
        authority.decision.get(),
        NativeResidentPresentDecision::Revoked
    );
    assert_eq!(renderer.frame_provider.reads, 1);
    assert_no_execution(&session, &renderer);
    assert_eq!(session.owner.state(), GpuOwnerState::Active);
    assert!(!session.owner.has_unfinished_logical_frame());
    stop(&mut session, &mut renderer);
}

#[test]
fn native_gpu_output_display_prepare_in_flight_controls_retain_real_work_until_complete_stop() {
    for follow in [false, true] {
        for decision in [
            NativeResidentPresentDecision::Cancelled,
            NativeResidentPresentDecision::Revoked,
        ] {
            let authority = Rc::new(Authority::new(decision));
            let mut renderer = renderer(Rc::clone(&authority));
            let mut session = headless_session();
            let frame = NativeFrameId {
                output_id: OUTPUT,
                project_epoch: EPOCH,
                request_sequence: 1,
                geometry_generation: session.geometry_generation,
            };
            let mut plan = NativeAllocationPlan::new(frame);
            plan.add_new(ChargeKind::IsfScratch, 16).unwrap();
            let quote = session.owner.quote(&plan).unwrap();
            let reservation = session.owner.reserve(quote).unwrap();
            let mut transaction = session.owner.begin_frame(frame, reservation).unwrap();
            let mut phase = transaction.begin_phase(GpuPhase::IsfBatch).unwrap();
            let charge = phase.bind_allocation(ChargeKind::IsfScratch, 16).unwrap();
            struct BufferOwner {
                buffer: wgpu::Buffer,
                charge: AllocationCharge,
            }
            let resource = Arc::new(BufferOwner {
                buffer: phase.device().create_buffer(&wgpu::BufferDescriptor {
                    label: Some("preparation control retained buffer"),
                    size: 16,
                    usage: wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                }),
                charge,
            });
            let weak = Arc::downgrade(&resource);
            phase
                .retain_resource(Arc::clone(&resource), &[resource.charge.id()])
                .unwrap();
            let mut encoder =
                phase
                    .device()
                    .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                        label: Some("preparation control real clear"),
                    });
            encoder.clear_buffer(&resource.buffer, 0, None);
            let encoded = phase.finish(vec![encoder.finish()]);
            let receipt = transaction.submit_phase(encoded).unwrap();
            assert!(receipt.submission().sequence > 0);
            let pending = transaction.finish();
            assert!(session.owner.has_unfinished_logical_frame());
            assert_eq!(session.owner.budget_snapshot().reservation_count, 1);
            assert_eq!(session.owner.budget_snapshot().managed_bytes(), 16);

            let result = prepare(
                &mut session,
                &mut renderer,
                &scene(),
                follow,
                &|| false,
                &authority,
            );
            match (decision, result) {
                (
                    NativeResidentPresentDecision::Cancelled,
                    Err(NativeResidentPrepareError::Cancelled {
                        retirement_required: true,
                    }),
                )
                | (
                    NativeResidentPresentDecision::Revoked,
                    Err(NativeResidentPrepareError::Revoked {
                        retirement_required: true,
                    }),
                ) => {}
                _ => panic!(
                    "a rejected tick with real unfinished GPU work requires complete retirement"
                ),
            }
            assert_eq!(renderer.frame_provider.reads, 0);
            assert_no_execution(&session, &renderer);
            assert!(session.owner.has_unfinished_logical_frame());
            assert!(weak.upgrade().is_some());
            session.begin_stop_with_renderer(&mut renderer).unwrap();
            assert!(
                !session.poll_stop().unwrap(),
                "Stop cannot reap while its real allocation owner remains live"
            );
            assert_eq!(session.owner.budget_snapshot().managed_bytes(), 16);
            assert!(weak.upgrade().is_some());
            drop(resource);
            drop(pending);
            drop(receipt);
            reap_started(&mut session);
            assert!(
                weak.upgrade().is_none(),
                "the complete allocation owner drops only after real cleanup"
            );
        }
    }
}
