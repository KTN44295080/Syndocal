//! One native Display tick: canonical preparation, affine completion and CPU handover.

use super::present::{
    NativeCpuArtisticDisplay, NativeHardBlackoutDisplay, ResidentAuthority, ResidentDisplayContent,
};
use super::*;
use crate::native_gpu_contract::{GpuPhase, NativeGpuDecline, NativeGpuFault, NativeGpuFaultKind};
use crate::native_gpu_output::{OutputBoundary, OutputRejection};
use crate::native_gpu_result::NativeCpuFrameOwner;
use crate::{
    ResolvedVideoEffectChain, VideoEffectRenderContext, VideoOutputArtisticPayload,
    VideoOutputLastValidKey, VideoOutputMappingIdentity, VideoOutputPresentationContract,
    VideoOutputRenderPlan, VideoRenderInput,
};
use protocol::{VideoEffectScope, VideoLayerTransitionRuntimeSnapshot, VideoSnapshot};

struct LivePreparationCancellation<'a, A> {
    cancellation: &'a dyn VideoRenderCancellation,
    authority: &'a A,
    observed: std::sync::atomic::AtomicU8,
}

impl<A> LivePreparationCancellation<'_, A> {
    fn record(&self, decision: NativeResidentPresentDecision) {
        let value = match decision {
            NativeResidentPresentDecision::Permit => return,
            NativeResidentPresentDecision::Cancelled => 1,
            NativeResidentPresentDecision::Revoked => 2,
        };
        let _ = self.observed.compare_exchange(
            0,
            value,
            std::sync::atomic::Ordering::Relaxed,
            std::sync::atomic::Ordering::Relaxed,
        );
    }
}

impl<A: NativeResidentPresentationAuthority> VideoRenderCancellation
    for LivePreparationCancellation<'_, A>
{
    fn is_cancelled(&self) -> bool {
        if self.cancellation.is_cancelled() {
            self.record(NativeResidentPresentDecision::Cancelled);
            return true;
        }
        let permit = self
            .authority
            .revalidate(NativeResidentPresentBoundary::BeforePrepare);
        self.record(permit);
        if self.cancellation.is_cancelled() {
            self.record(NativeResidentPresentDecision::Cancelled);
            return true;
        }
        permit != NativeResidentPresentDecision::Permit
    }
}

impl<A: NativeResidentPresentationAuthority> NativeResidentPresentationAuthority
    for LivePreparationCancellation<'_, A>
{
    fn revalidate(&self, boundary: NativeResidentPresentBoundary) -> NativeResidentPresentDecision {
        let decision = self.authority.revalidate(boundary);
        self.record(decision);
        decision
    }
}

#[derive(PartialEq)]
pub(super) struct ResidentHistoryConfiguration {
    project_epoch: u64,
    geometry_generation: u64,
    extent: (u32, u32),
    inputs: Vec<(
        protocol::VideoRenderInputKey,
        protocol::VideoLayerId,
        protocol::VideoSourceSummary,
    )>,
}

pub(super) struct ResidentInputBinding {
    /// The pre-C0 queue/decoder owner being migrated, if this is an I key.
    legacy_layer_id: Option<protocol::VideoLayerId>,
    key: protocol::VideoRenderInputKey,
    source: protocol::VideoSourceSummary,
    source_generation: u64,
    queue_epoch: u64,
    content_sequence: u64,
}

pub(super) struct ResidentFollowLastValid {
    key: VideoOutputLastValidKey,
    source_timeline_id: protocol::TimelineId,
    generation: u64,
    owner: Arc<NativeCpuFrameOwner>,
}

enum ResidentOutputCompletion {
    Ready(NativeResidentDisplayFrame),
    Retiring(SharedNativeOutputRetirement),
}

/// Scope decisions precede decode and FX. Preparation failures stay on the
/// canonical path, where their exact scope/stage evidence is published.
fn resident_layer_chains(
    snapshot: &VideoSnapshot,
    context: VideoEffectRenderContext<'_>,
    transitions: &VideoLayerTransitionRuntimeSnapshot,
    plan: &VideoOutputRenderPlan,
) -> Option<Vec<ResolvedVideoEffectChain>> {
    if !(2..=3).contains(&plan.composition.layers.len()) {
        return None;
    }
    let composition_id = plan.composition.composition_id;
    let idle = |scope| match crate::resolve_video_effect_chain(snapshot, &scope) {
        Ok(None) => true,
        Ok(Some(chain)) => !crate::resolved_chain_executes(&chain),
        Err(_) => false,
    };
    if !idle(VideoEffectScope::Composition { composition_id })
        || !idle(VideoEffectScope::Output {
            output_id: plan.output_id,
        })
        || snapshot
            .layer_groups
            .iter()
            .any(|group| group.composition_id == composition_id && group.enabled && !group.bypassed)
        || transitions.buses.iter().any(|active| {
            snapshot
                .transition_buses
                .iter()
                .any(|bus| bus.id == active.bus_id && bus.composition_id == composition_id)
        })
    {
        return None;
    }
    let ids = plan
        .composition
        .layers
        .iter()
        .map(|layer| layer.layer_id)
        .collect::<Vec<_>>();
    if ids
        .iter()
        .enumerate()
        .any(|(index, id)| ids[..index].contains(id))
    {
        return None;
    }
    let (_, sampled) = crate::isf_layer_inputs::image_render_order(
        snapshot,
        context.clip_runtime,
        &ids,
        Some(plan.output_id),
        Some(composition_id),
        Some(transitions),
    )
    .ok()?;
    if !sampled.is_empty() {
        return None;
    }
    let mut chains = Vec::with_capacity(ids.len());
    for layer_plan in &plan.composition.layers {
        if !crate::gpu_composition_layer_plan(layer_plan) {
            return None;
        }
        let layer = snapshot
            .layers
            .iter()
            .find(|layer| layer.id == layer_plan.layer_id)?;
        let runtime = context
            .clip_runtime
            .layers
            .iter()
            .find(|runtime| runtime.layer_id == layer.id);
        if runtime.is_some_and(|runtime| runtime.transition.is_some()) {
            return None;
        }
        if let Some(slot_id) = runtime.and_then(|runtime| runtime.active_slot_id) {
            if !idle(VideoEffectScope::Clip {
                layer_id: layer.id,
                slot_id,
            }) {
                return None;
            }
        }
        let chain = crate::resolve_effective_layer_chain(snapshot, layer).ok()??;
        if !crate::resolved_chain_executes(&chain) {
            return None;
        }
        chains.push(chain);
    }
    Some(chains)
}

impl NativeResidentDisplaySession {
    fn bind_input_keys(
        &mut self,
        inputs: &mut [VideoRenderInput],
        project_epoch: u64,
    ) -> Result<(), String> {
        let explicit = inputs
            .iter()
            .filter(|input| input.key != protocol::VideoRenderInputKey::LEGACY)
            .map(|input| input.key)
            .collect::<Vec<_>>();
        for input in inputs.iter_mut() {
            let legacy = input.key == protocol::VideoRenderInputKey::LEGACY;
            let existing = self.input_bindings.iter().position(|binding| {
                binding.key.project_render_epoch == project_epoch
                    && if legacy {
                        binding.legacy_layer_id == Some(input.layer_id())
                    } else {
                        binding.legacy_layer_id.is_none() && binding.key == input.key
                    }
            });
            let index = if let Some(index) = existing {
                index
            } else {
                let key = if legacy {
                    loop {
                        let input_id = protocol::VideoRenderInputId(self.next_input_id);
                        self.next_input_id =
                            self.next_input_id.checked_add(1).ok_or_else(|| {
                                "native input identity sequence exhausted".to_string()
                            })?;
                        let key = protocol::VideoRenderInputKey {
                            project_render_epoch: project_epoch,
                            input_id,
                        };
                        if !explicit.contains(&key)
                            && !self.input_bindings.iter().any(|binding| binding.key == key)
                        {
                            break key;
                        }
                    }
                } else {
                    if self
                        .input_bindings
                        .iter()
                        .any(|binding| binding.key == input.key)
                    {
                        return Err(
                            "explicit input collides with an allocated native graph input".into(),
                        );
                    }
                    input.key
                };
                self.input_bindings.push(ResidentInputBinding {
                    legacy_layer_id: legacy.then_some(input.layer_id()),
                    key,
                    source: input.request.source.clone(),
                    source_generation: 1,
                    queue_epoch: 0,
                    content_sequence: 0,
                });
                self.input_bindings.len() - 1
            };
            let binding = &mut self.input_bindings[index];
            if binding.source != input.request.source {
                binding.source_generation = binding
                    .source_generation
                    .checked_add(1)
                    .ok_or_else(|| "native source generation exhausted".to_string())?;
                binding.source = input.request.source.clone();
            }
            input.key = binding.key;
        }
        for (index, input) in inputs.iter().enumerate() {
            if input.key.project_render_epoch != project_epoch
                || inputs[..index].iter().any(|old| old.key == input.key)
            {
                return Err(
                    "native input allocation collided with an explicit input identity".into(),
                );
            }
        }
        Ok(())
    }

    fn request_key(
        &self,
        plan: &VideoOutputRenderPlan,
        project_epoch: u64,
        extent: (u32, u32),
    ) -> VideoOutputLastValidKey {
        VideoOutputLastValidKey {
            project_render_epoch: project_epoch,
            output_id: self.output_id,
            width: extent.0,
            height: extent.1,
            composition_id: plan.composition.composition_id,
            kind: plan.kind.clone(),
            fullscreen: plan.fullscreen,
            monitor_id: plan.monitor_id,
            endpoint_name: plan.endpoint_name.clone(),
        }
    }

    fn next_request(
        &mut self,
        plan: &VideoOutputRenderPlan,
        project_epoch: u64,
        extent: (u32, u32),
    ) -> Result<NativeOutputRequest, String> {
        self.ensure_usable()?;
        if extent.0 == 0 || extent.1 == 0 || self.geometry_generation == 0 {
            return Err("native preparation requires a reserved physical extent".into());
        }
        if let Some((width, height, _)) = self.reserved_geometry {
            if extent != (width, height) {
                return Err("native preparation differs from reserved physical geometry".into());
            }
        }
        self.request_sequence = self
            .request_sequence
            .checked_add(1)
            .ok_or_else(|| "native request sequence exhausted".to_string())?;
        let frame = NativeFrameId {
            output_id: self.output_id,
            project_epoch,
            request_sequence: self.request_sequence,
            geometry_generation: self.geometry_generation,
        };
        let key = self.request_key(plan, project_epoch, extent);
        let fence = self
            .cache
            .begin_request(key.clone(), frame)
            .map_err(|error| format!("native cache request rejected: {error:?}"))?;
        Ok(NativeOutputRequest {
            context: self.owner.context().id(),
            frame,
            key,
            fence,
            pts_ms: plan
                .composition
                .layers
                .iter()
                .map(|layer| layer.position_ms)
                .max()
                .unwrap_or(0),
            duration_ms: 0,
            mapping: plan.mapping.clone(),
            mapping_identity: VideoOutputMappingIdentity::from_mapping(&plan.mapping),
        })
    }

    fn normalize_cpu_queue_keys<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
    ) {
        if let Some(configuration) = self.history_configuration.take() {
            for (key, layer_id, _) in configuration.inputs {
                if !self
                    .input_bindings
                    .iter()
                    .any(|binding| binding.key == key && binding.legacy_layer_id == Some(layer_id))
                {
                    continue;
                }
                if let Some(queue) = renderer
                    .runtime
                    .queues
                    .iter_mut()
                    .find(|queue| queue.key == key && queue.layer_id == layer_id)
                {
                    queue.key = protocol::VideoRenderInputKey::LEGACY;
                }
            }
        }
    }

    fn classify_preparation<P: VideoFrameProvider, A: NativeResidentPresentationAuthority>(
        &self,
        result: Result<NativeResidentDisplayFrame, String>,
        cancellation: &LivePreparationCancellation<'_, A>,
        renderer: &VideoPreviewRenderer<P>,
    ) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
        // A successful frame needs the final authority check. An earlier
        // genuine error must not be relabeled by a later plan edit: failures
        // use only the control rejection observed while executing that tick.
        if result.is_ok() {
            cancellation.is_cancelled();
        } else if cancellation.cancellation.is_cancelled() {
            cancellation.record(NativeResidentPresentDecision::Cancelled);
        }
        let observed = cancellation
            .observed
            .load(std::sync::atomic::Ordering::Relaxed);
        let owner = self.owner.0.as_ref();
        let physical_fault = owner.is_some_and(|owner| {
            matches!(
                owner.state(),
                crate::native_gpu_contract::GpuOwnerState::DeviceLostRetained
                    | crate::native_gpu_contract::GpuOwnerState::PoisonedRetained
            )
        }) || self
            .retirements
            .iter()
            .any(|retirement| matches!(retirement.reason(), Some(OutputRejection::Fault(_))));
        if observed == 0 || physical_fault {
            return result.map_err(NativeResidentPrepareError::Fault);
        }
        let retirement_required = !self.retirements.is_empty()
            || !self.unsubmitted.is_empty()
            || self.recovered_history.is_some()
            || matches!(self.history, Some(ResidentHistoryState::Handover(_)))
            || renderer.native_cpu_bridge.is_some()
            || !renderer.native_cpu_retained_reservations.is_empty()
            || owner.is_none_or(|owner| {
                owner.state() != crate::native_gpu_contract::GpuOwnerState::Active
                    || owner.has_unfinished_logical_frame()
            });
        // A completed affine frame may be discarded here: its G/P completion
        // is already real, while outstanding work stays in the session above.
        drop(result);
        Err(if observed == 1 {
            NativeResidentPrepareError::Cancelled {
                retirement_required,
            }
        } else {
            NativeResidentPrepareError::Revoked {
                retirement_required,
            }
        })
    }

    fn prepare_cpu_output<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        request: NativeOutputRequest,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<NativeResidentDisplayFrame, String> {
        if self.history.is_some() {
            self.handover_all_history_to_cpu(renderer, cancellation)?;
        }
        self.normalize_cpu_queue_keys(renderer);
        let owner = self.owner.take();
        let (owner, result) = renderer.prepare_native_cpu_output(
            owner,
            request.frame,
            snapshot,
            context,
            transitions,
            plan,
            request.key.width,
            request.key.height,
            cancellation,
        );
        self.owner.restore(owner);
        self.cpu_display_frame(
            request,
            result.map_err(|error| format!("canonical shared CPU output failed: {error:?}"))?,
        )
    }

    fn cpu_display_frame(
        &self,
        request: NativeOutputRequest,
        output: crate::native_cpu_output::NativeCpuOutput,
    ) -> Result<NativeResidentDisplayFrame, String> {
        let crate::native_cpu_output::NativeCpuOutput { result, charge } = output;
        let extent = (request.key.width, request.key.height);
        let pts_ms = result.frame().map_or(0, |frame| frame.pts_ms);
        let evidence = result.evidence.clone();
        let output_mapping = result.output_mapping.clone();
        if evidence.freshness != crate::VideoOutputRenderFreshness::Fresh
            || evidence.error.is_some()
        {
            return Ok(NativeResidentDisplayFrame {
                content: ResidentDisplayContent::Rejected,
                evidence,
                extent,
                pts_ms,
                output_mapping,
            });
        }
        VideoOutputPresentationContract {
            project_render_epoch: request.frame.project_epoch,
            output_id: request.frame.output_id,
            width: extent.0,
            height: extent.1,
            output_mapping_identity: request.mapping_identity,
        }
        .admit(&result)
        .map_err(|error| format!("canonical CPU presentation rejected: {error:?}"))?;
        let content = match result.payload {
            VideoOutputArtisticPayload::HardBlackout => {
                if charge.is_some() {
                    return Err("payload-less blackout unexpectedly owns a CPU pixel charge".into());
                }
                ResidentDisplayContent::HardBlackout(NativeHardBlackoutDisplay { request })
            }
            VideoOutputArtisticPayload::Frame(frame) => {
                let charge = charge
                    .ok_or_else(|| "canonical CPU output has no accounting owner".to_string())?;
                if charge.id().context != request.context
                    || charge.capacity() < frame.data.capacity() as u64
                {
                    return Err(
                        "canonical CPU output charge does not own its pixel capacity".into(),
                    );
                }
                let owner = Arc::new(NativeCpuFrameOwner::new(Arc::new(frame), Some(charge)));
                ResidentDisplayContent::Cpu(NativeCpuArtisticDisplay { request, owner })
            }
        };
        Ok(NativeResidentDisplayFrame {
            content,
            evidence,
            extent,
            pts_ms,
            output_mapping,
        })
    }

    /// Prepare one physical output exactly once. The result remains affine
    /// through hidden-window geometry convergence and at most one P retry.
    pub fn prepare_output<P: VideoFrameProvider, A: NativeResidentPresentationAuthority>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        extent: (u32, u32),
        cancellation: &dyn VideoRenderCancellation,
        authority: &A,
    ) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
        let live = LivePreparationCancellation {
            cancellation,
            authority,
            observed: std::sync::atomic::AtomicU8::new(0),
        };
        let result = self.prepare_output_inner(
            renderer,
            snapshot,
            context,
            transitions,
            extent,
            &live,
            &live,
        );
        self.classify_preparation(result, &live, renderer)
    }

    fn prepare_output_inner<P: VideoFrameProvider, A: NativeResidentPresentationAuthority>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        extent: (u32, u32),
        cancellation: &dyn VideoRenderCancellation,
        authority: &A,
    ) -> Result<NativeResidentDisplayFrame, String> {
        self.ensure_usable()?;
        if cancellation.is_cancelled()
            || authority.revalidate(NativeResidentPresentBoundary::BeforePrepare)
                != NativeResidentPresentDecision::Permit
        {
            return Err("native output preparation cancelled or revoked".into());
        }
        // Leaving Follow releases its complete cached CPU owner before any
        // ordinary frame admission, including an ordinary preparation failure.
        self.follow_last_valid = None;
        self.install_renderer(renderer)?;
        let mut plan = crate::build_video_output_render_plan(snapshot, self.output_id)
            .map_err(|error| format!("native output plan rejected: {error:?}"))?;
        crate::apply_video_layer_transition_weights(snapshot, transitions, &mut plan.composition)
            .map_err(|error| format!("native transition plan rejected: {error:?}"))?;
        if plan.output_blackout || plan.output_opacity <= f32::EPSILON || plan.composition.blackout
        {
            self.follow_last_valid = None;
            self.cache
                .blackout()
                .map_err(|error| format!("native blackout fence failed: {error:?}"))?;
            let request = self.next_request(&plan, context.project_render_epoch, extent)?;
            // Canonical control-state preparation returns HardBlackout before
            // decode or any FX. No pixel buffer or resident Ready is invented.
            let result = renderer
                .prepare_output_artistic_render_result_impl_with_cancellation(
                    snapshot,
                    context,
                    Some(transitions),
                    &plan,
                    extent.0,
                    extent.1,
                    Some(cancellation),
                    Some(Vec::new()),
                )
                .map_err(|error| format!("canonical blackout preparation failed: {error:?}"))?;
            return self.cpu_display_frame(
                request,
                crate::native_cpu_output::NativeCpuOutput {
                    result,
                    charge: None,
                },
            );
        }
        let request = self.next_request(&plan, context.project_render_epoch, extent)?;
        let Some(chains) = resident_layer_chains(snapshot, context, transitions, &plan) else {
            return self.prepare_cpu_output(
                renderer,
                snapshot,
                context,
                transitions,
                &plan,
                request,
                cancellation,
            );
        };
        let mut inputs = Vec::with_capacity(chains.len());
        for layer_plan in &plan.composition.layers {
            let layer = snapshot
                .layers
                .iter()
                .find(|layer| layer.id == layer_plan.layer_id)
                .expect("resident scope validation retained each layer");
            let mut layer_inputs = renderer
                .frame_provider
                .layer_frame_inputs(layer, extent.0, extent.1);
            if layer_inputs.len() != 1 {
                return self.prepare_cpu_output(
                    renderer,
                    snapshot,
                    context,
                    transitions,
                    &plan,
                    request,
                    cancellation,
                );
            }
            let input = layer_inputs.remove(0);
            if input.layer_id() != layer.id {
                return Err("native decoder input identity is inconsistent".into());
            }
            inputs.push(input);
        }
        self.bind_input_keys(&mut inputs, context.project_render_epoch)?;
        let configuration = ResidentHistoryConfiguration {
            project_epoch: context.project_render_epoch,
            geometry_generation: self.geometry_generation,
            extent,
            inputs: inputs
                .iter()
                .map(|input| (input.key, input.layer_id(), input.request.source.clone()))
                .collect(),
        };
        if self.history_configuration.as_ref() != Some(&configuration) && self.history.is_some() {
            self.handover_all_history_to_cpu(renderer, cancellation)?;
            self.normalize_cpu_queue_keys(renderer);
        }
        renderer.isf_render_namespace = (context.project_render_epoch, Some(self.output_id), false);
        renderer.set_isf_audio_frame(context.audio_frame);
        renderer.reset_isf_stack_metrics();
        let mut retained = Vec::new();
        for layer in snapshot
            .layers
            .iter()
            .filter(|layer| !inputs.iter().any(|input| input.layer_id() == layer.id))
        {
            let runtime = context
                .clip_runtime
                .layers
                .iter()
                .find(|runtime| runtime.layer_id == layer.id);
            if let Some(transition) = runtime.and_then(|runtime| runtime.transition.as_ref()) {
                for (slot, position) in [
                    (
                        transition.outgoing_slot_id,
                        runtime.map_or(0, |runtime| runtime.playhead_ms),
                    ),
                    (transition.incoming_slot_id, transition.incoming_playhead_ms),
                ] {
                    // Match canonical retention: invalid unrelated output input
                    // is omitted without introducing a new error for this output.
                    if let Ok(input) = crate::video_render_input_for_clip_slot(
                        snapshot,
                        layer,
                        slot,
                        position,
                        context.project_render_epoch,
                        extent.0,
                        extent.1,
                    ) {
                        retained.push(input);
                    }
                }
            } else {
                retained.push(crate::legacy_layer_input(layer, extent.0, extent.1, None));
            }
        }
        retained.extend(inputs.iter().cloned());
        renderer.frame_provider.retain_inputs(&retained);
        renderer.generator_queue_sources.sync(
            &mut renderer.runtime,
            true,
            snapshot
                .layers
                .iter()
                .map(|layer| {
                    let generator = match layer.source.kind {
                        protocol::VideoSourceKind::Generator(generator) => Some(generator),
                        _ => None,
                    };
                    (protocol::VideoRenderInputKey::LEGACY, layer.id, generator)
                })
                .chain(
                    retained
                        .iter()
                        .filter(|input| {
                            input.key != protocol::VideoRenderInputKey::LEGACY
                                && !inputs.iter().any(|native| native.key == input.key)
                        })
                        .map(crate::generator::input_identity),
                ),
        );
        renderer.runtime.sync_layers(
            &snapshot
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>(),
        );
        let mut raw = Vec::with_capacity(inputs.len());
        for input in &inputs {
            let frame = renderer
                .frame_provider
                .frame_for_input_shared_with_cancellation(input, cancellation)
                .map_err(|error| format!("native raw input acquisition failed: {error:?}"))?;
            let binding = self
                .input_bindings
                .iter_mut()
                .find(|binding| binding.key == input.key)
                .expect("input allocation created its private binding");
            binding.content_sequence = binding
                .content_sequence
                .checked_add(1)
                .ok_or_else(|| "native input content sequence exhausted".to_string())?;
            raw.push(frame);
        }
        let host_fits = self.history_host.as_ref().is_none_or(|host| {
            HistoryHostBudget::planning_limit(inputs.len(), extent)
                .is_some_and(|required| required <= host.snapshot().limit)
        });
        let compatible = host_fits
            && renderer.runtime.config.frame_queue_capacity
                >= crate::frame_history_policy::NATIVE_HISTORY_CAPACITY
            && raw.iter().all(|frame| {
                frame.format == crate::VideoPixelFormat::Rgba8
                    && (frame.width, frame.height) == extent
            })
            && inputs.iter().all(|input| {
                renderer
                    .runtime
                    .queues
                    .iter()
                    .find(|queue| queue.layer_id == input.layer_id())
                    .is_some_and(|queue| {
                        queue.iter_shared().all(|frame| {
                            frame.format == crate::VideoPixelFormat::Rgba8
                                && (frame.width, frame.height) == extent
                        })
                    })
            });
        if compatible && self.history.is_none() {
            let mut history_inputs = Vec::with_capacity(inputs.len());
            for input in &inputs {
                let queue = renderer
                    .runtime
                    .queues
                    .iter_mut()
                    .find(|queue| queue.layer_id == input.layer_id())
                    .expect("canonical queues synchronized before native binding");
                queue.key = input.key;
                let binding = self
                    .input_bindings
                    .iter_mut()
                    .find(|binding| binding.key == input.key)
                    .expect("native input retains its allocation binding");
                binding.queue_epoch = binding
                    .queue_epoch
                    .checked_add(1)
                    .ok_or_else(|| "native history queue epoch exhausted".to_string())?;
                history_inputs.push(NativeResidentHistoryInput {
                    input: input.key,
                    layer_id: input.layer_id(),
                    queue_epoch: binding.queue_epoch,
                    source_generation: binding.source_generation,
                });
            }
            self.bind_canonical_cpu_history(
                renderer,
                request.frame.project_epoch,
                request.frame.request_sequence,
                extent,
                history_inputs,
            )?;
            self.history_configuration = Some(configuration);
        }
        let mut layers = StackBatch::Empty;
        for ((input, frame), chain) in inputs.iter().zip(raw).zip(chains) {
            let binding = self
                .input_bindings
                .iter()
                .find(|binding| binding.key == input.key)
                .expect("raw acquisition retains its private content binding");
            let (entry, source_generation) = match self.history.as_ref() {
                Some(ResidentHistoryState::Active(active)) => {
                    let index = active
                        .inputs
                        .iter()
                        .position(|key| *key == input.key)
                        .ok_or_else(|| "native input is absent from its history".to_string())?;
                    let entry = active.sequences[index]
                        .prospective_next()
                        .map_err(|error| format!("native history sequence failed: {error:?}"))?;
                    let source_generation = active
                        .session
                        .history(index)
                        .ok_or_else(|| "native history scope disappeared".to_string())?
                        .scope()
                        .source_generation;
                    (entry, source_generation)
                }
                Some(ResidentHistoryState::Handover(_)) => {
                    return Err("native preparation encountered an unfinished handover".into())
                }
                None => (
                    crate::native_gpu_contract::HistoryEntryId {
                        input: input.key,
                        queue_epoch: binding.queue_epoch,
                        sequence: binding.content_sequence,
                    },
                    binding.source_generation,
                ),
            };
            layers.push(NativeLayerPreparation {
                input: frame,
                chain,
                entry,
                source_generation,
                content_sequence: binding.content_sequence,
            });
        }
        let live = ResidentAuthority(authority);
        crate::native_gpu_output::guard(
            &request,
            OutputBoundary::BeforePreflight,
            &live,
            cancellation,
        )
        .map_err(|error| format!("native output preflight revoked: {error:?}"))?;
        let execution = if compatible {
            self.execute_prepared_layers(
                renderer,
                request.clone(),
                layers,
                &plan.composition.layers,
                [0; 4],
                &live,
                cancellation,
            )?
        } else {
            let issued = layers.iter().map(|layer| layer.entry).collect::<Vec<_>>();
            let continuation = renderer
                .prepare_native_layer_continuation(
                    request.frame,
                    layers,
                    NativeBatchRestrictions::default(),
                    Some(cancellation),
                )
                .map_err(|error| format!("CPU continuation preparation failed: {error}"))?
                .into_cpu()
                .map_err(|error| format!("CPU continuation rejected: {error:?}"))?;
            self.run_cpu_continuation(
                continuation,
                NativeGpuDecline::UnsupportedExtent,
                &issued,
                &request,
                &live,
                cancellation,
            )?
        };
        match execution {
            NativeResidentExecution::Cpu { reason, results } => {
                if self.history.is_some() {
                    self.publish_history_to_cpu(renderer)?;
                }
                self.normalize_cpu_queue_keys(renderer);
                let owner = self.owner.take();
                let (owner, output) = renderer.finish_native_cpu_layers(
                    owner,
                    request.frame,
                    snapshot,
                    context,
                    transitions,
                    &plan,
                    extent.0,
                    extent.1,
                    cancellation,
                    results,
                );
                self.owner.restore(owner);
                self.cpu_display_frame(
                    request,
                    output.map_err(|error| {
                        format!("canonical CPU continuation ({reason:?}) failed: {error:?}")
                    })?,
                )
            }
            NativeResidentExecution::Gpu(progress) => {
                match self.finish_resident_output(renderer, progress, &live, cancellation)? {
                    ResidentOutputCompletion::Ready(frame) => Ok(frame),
                    ResidentOutputCompletion::Retiring(retirement) => self.finish_recovered_output(
                        renderer,
                        retirement,
                        request,
                        snapshot,
                        context,
                        transitions,
                        &plan,
                        extent,
                        &live,
                        cancellation,
                    ),
                }
            }
            NativeResidentExecution::Retiring(retirement) => self.finish_recovered_output(
                renderer,
                retirement,
                request,
                snapshot,
                context,
                transitions,
                &plan,
                extent,
                &live,
                cancellation,
            ),
        }
    }

    pub fn prepare_follow_output<P: VideoFrameProvider, A: NativeResidentPresentationAuthority>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        outgoing: &VideoSnapshot,
        incoming: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        kind: protocol::VideoClipTakeKind,
        curve: protocol::VideoLayerTransitionCurve,
        progress_millis: u16,
        source_timeline_id: protocol::TimelineId,
        follow_generation: u64,
        transition_chain: Option<&ResolvedVideoEffectChain>,
        extent: (u32, u32),
        cancellation: &dyn VideoRenderCancellation,
        authority: &A,
    ) -> Result<NativeResidentDisplayFrame, NativeResidentPrepareError> {
        let live = LivePreparationCancellation {
            cancellation,
            authority,
            observed: std::sync::atomic::AtomicU8::new(0),
        };
        let result = self.prepare_follow_output_inner(
            renderer,
            outgoing,
            incoming,
            context,
            transitions,
            kind,
            curve,
            progress_millis,
            source_timeline_id,
            follow_generation,
            transition_chain,
            extent,
            &live,
            &live,
        );
        self.classify_preparation(result, &live, renderer)
    }

    fn prepare_follow_output_inner<
        P: VideoFrameProvider,
        A: NativeResidentPresentationAuthority,
    >(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        outgoing: &VideoSnapshot,
        incoming: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        kind: protocol::VideoClipTakeKind,
        curve: protocol::VideoLayerTransitionCurve,
        progress_millis: u16,
        source_timeline_id: protocol::TimelineId,
        follow_generation: u64,
        transition_chain: Option<&ResolvedVideoEffectChain>,
        extent: (u32, u32),
        cancellation: &dyn VideoRenderCancellation,
        authority: &A,
    ) -> Result<NativeResidentDisplayFrame, String> {
        self.ensure_usable()?;
        if cancellation.is_cancelled()
            || authority.revalidate(NativeResidentPresentBoundary::BeforePrepare)
                != NativeResidentPresentDecision::Permit
        {
            return Err("native Follow preparation cancelled or revoked".into());
        }
        self.follow_last_valid = self.follow_last_valid.take().filter(|cached| {
            cached.source_timeline_id == source_timeline_id
                && cached.generation == follow_generation
        });
        self.install_renderer(renderer)?;
        let mut outgoing_plan = crate::build_video_output_render_plan(outgoing, self.output_id)
            .map_err(|error| format!("native Follow outgoing plan rejected: {error:?}"))?;
        let mut incoming_plan = crate::build_video_output_render_plan(incoming, self.output_id)
            .map_err(|error| format!("native Follow incoming plan rejected: {error:?}"))?;
        let incoming_key = self.request_key(&incoming_plan, context.project_render_epoch, extent);
        self.follow_last_valid = self
            .follow_last_valid
            .take()
            .filter(|cached| cached.key == incoming_key);
        crate::apply_video_layer_transition_weights(
            outgoing,
            transitions,
            &mut outgoing_plan.composition,
        )
        .map_err(|error| format!("native Follow outgoing transition rejected: {error:?}"))?;
        crate::apply_video_layer_transition_weights(
            incoming,
            transitions,
            &mut incoming_plan.composition,
        )
        .map_err(|error| format!("native Follow incoming transition rejected: {error:?}"))?;
        if outgoing_plan.kind != incoming_plan.kind
            || outgoing_plan.fullscreen != incoming_plan.fullscreen
            || outgoing_plan.monitor_id != incoming_plan.monitor_id
            || outgoing_plan.endpoint_name != incoming_plan.endpoint_name
            || VideoOutputMappingIdentity::from_mapping(&outgoing_plan.mapping)
                != VideoOutputMappingIdentity::from_mapping(&incoming_plan.mapping)
        {
            return Err("native Follow sides do not share one physical output contract".into());
        }
        let both_black = (outgoing_plan.output_blackout
            || outgoing_plan.output_opacity <= f32::EPSILON
            || outgoing_plan.composition.blackout)
            && (incoming_plan.output_blackout
                || incoming_plan.output_opacity <= f32::EPSILON
                || incoming_plan.composition.blackout);
        if both_black {
            self.follow_last_valid = None;
            self.cache
                .blackout()
                .map_err(|error| format!("native Follow blackout fence failed: {error:?}"))?;
            let request =
                self.next_request(&incoming_plan, context.project_render_epoch, extent)?;
            let result = renderer
                .prepare_output_artistic_render_result_impl_with_cancellation(
                    incoming,
                    context,
                    Some(transitions),
                    &incoming_plan,
                    extent.0,
                    extent.1,
                    Some(cancellation),
                    Some(Vec::new()),
                )
                .map_err(|error| format!("canonical Follow blackout failed: {error:?}"))?;
            return self.cpu_display_frame(
                request,
                crate::native_cpu_output::NativeCpuOutput {
                    result,
                    charge: None,
                },
            );
        }
        if self.history.is_some() {
            self.handover_all_history_to_cpu(renderer, cancellation)?;
        }
        self.normalize_cpu_queue_keys(renderer);
        let request = self.next_request(&incoming_plan, context.project_render_epoch, extent)?;
        let last_valid = self
            .follow_last_valid
            .as_ref()
            .filter(|cached| {
                cached.key == request.key
                    && cached.source_timeline_id == source_timeline_id
                    && cached.generation == follow_generation
            })
            .map(|cached| Arc::clone(&cached.owner));
        let owner = self.owner.take();
        let (owner, output) = renderer.with_native_cpu_owner(owner, request.frame, |renderer| {
            let outgoing_result = renderer.prepare_native_cpu_artistic_with_plan(
                outgoing,
                context,
                transitions,
                &outgoing_plan,
                extent.0,
                extent.1,
                cancellation,
            )?;
            if outgoing_result.evidence.freshness != crate::VideoOutputRenderFreshness::Fresh
                || outgoing_result.evidence.error.is_some()
            {
                return renderer.capture_native_cpu_output(outgoing_result);
            }
            let incoming_result = renderer.prepare_native_cpu_artistic_with_plan(
                incoming,
                context,
                transitions,
                &incoming_plan,
                extent.0,
                extent.1,
                cancellation,
            )?;
            if incoming_result.evidence.freshness != crate::VideoOutputRenderFreshness::Fresh
                || incoming_result.evidence.error.is_some()
            {
                return renderer.capture_native_cpu_output(incoming_result);
            }
            let contract = VideoOutputPresentationContract {
                project_render_epoch: context.project_render_epoch,
                output_id: self.output_id,
                width: extent.0,
                height: extent.1,
                output_mapping_identity: request.mapping_identity,
            };
            for result in [&outgoing_result, &incoming_result] {
                contract.admit(result).map_err(|error| {
                    crate::VideoPreviewError::Runtime(crate::VideoRuntimeError::NativeGpu(format!(
                        "Follow side admission rejected: {error:?}"
                    )))
                })?;
            }
            let combined = renderer.render_native_follow_output_transition(
                crate::native_cpu_output::NativeFollowTransitionRequest {
                    outgoing: outgoing_result.frame(),
                    incoming: incoming_result.frame(),
                    extent,
                    kind,
                    curve,
                    progress_millis,
                    source_timeline_id,
                    transition_chain,
                    last_valid: last_valid.as_ref().map(|owner| owner.frame.as_ref()),
                    audio_frame: context.audio_frame,
                },
                cancellation,
            )?;
            renderer.capture_native_cpu_output(crate::VideoOutputArtisticRenderResult {
                payload: combined.payload,
                output_mapping: request.mapping.clone(),
                output_mapping_identity: request.mapping_identity,
                evidence: crate::VideoOutputRenderEvidence {
                    project_render_epoch: context.project_render_epoch,
                    output_id: self.output_id,
                    freshness: combined.evidence.freshness,
                    error: combined.evidence.error,
                },
            })
        });
        self.owner.restore(owner);
        let key = request.key.clone();
        let frame = self.cpu_display_frame(
            request,
            output.map_err(|error| format!("canonical shared Follow failed: {error:?}"))?,
        )?;
        if let ResidentDisplayContent::Cpu(cpu) = &frame.content {
            self.follow_last_valid = Some(ResidentFollowLastValid {
                key,
                source_timeline_id,
                generation: follow_generation,
                owner: Arc::clone(&cpu.owner),
            });
        }
        Ok(frame)
    }

    fn reject_retiring_output(
        &mut self,
        retirement: SharedNativeOutputRetirement,
    ) -> Result<NativeResidentDisplayFrame, String> {
        let reason = format!(
            "native output requires retained retirement: {:?}",
            retirement.reason()
        );
        self.retirements.push(retirement);
        self.terminal_fault = Some(reason.clone());
        Err(reason)
    }

    fn finish_recovered_output<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        retirement: SharedNativeOutputRetirement,
        request: NativeOutputRequest,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        extent: (u32, u32),
        authority: &impl NativeOutputAuthority,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<NativeResidentDisplayFrame, String> {
        if retirement.reason().is_some() {
            return self.reject_retiring_output(retirement);
        }
        if let Err(reason) = crate::native_gpu_output::guard(
            &request,
            OutputBoundary::AfterPoll,
            authority,
            cancellation,
        ) {
            self.retirements.push(retirement);
            return Err(format!(
                "recovered output revoked before publication: {reason:?}"
            ));
        }
        let recovered = match retirement.into_recovered_history(&mut self.owner) {
            Ok(history) => history,
            Err((retirement, fault)) => {
                self.retirements.push(retirement);
                return Err(format!(
                    "recovered output retains its failed owner: {fault:?}"
                ));
            }
        };
        self.recovered_history = Some(recovered);
        let recovered = self
            .recovered_history
            .as_mut()
            .expect("affine recovered history");
        let Some(ResidentHistoryState::Active(history)) = self.history.as_mut() else {
            return Err("recovered output lost its complete history session".into());
        };
        let faults = recovered.faults().to_vec();
        history
            .session
            .commit_recovered_with(recovered, |faults| {
                renderer.clear_full_output_effect_diagnostics(snapshot);
                renderer.record_effect_faults(faults.to_vec(), snapshot);
                Ok(())
            })
            .map_err(|error| {
                format!("canonical recovered history publication rejected: {error:?}")
            })?;
        if !recovered.is_committed() {
            return Err("recovered history publication did not commit every input".into());
        }
        // The failed batched transaction has drained; only now can its retired
        // resident owners leave the retained, committed history aggregate.
        for retired in recovered.take_retired() {
            let (entry, owner) = retired.into_parts();
            if owner.identity().entry != entry {
                return Err(
                    "recovered retirement does not match its original history entry".into(),
                );
            }
            drop(owner);
        }
        self.recovered_history = None;
        self.handover_all_history_to_cpu(renderer, cancellation)?;
        self.normalize_cpu_queue_keys(renderer);
        crate::native_gpu_output::guard(
            &request,
            OutputBoundary::BeforeCpuFallback,
            authority,
            cancellation,
        )
        .map_err(|reason| format!("recovered CPU output revoked: {reason:?}"))?;
        let owner = self.owner.take();
        let (owner, output) = renderer.finish_native_cpu_history(
            owner,
            request.frame,
            snapshot,
            context,
            transitions,
            plan,
            extent.0,
            extent.1,
            cancellation,
            faults,
        );
        self.owner.restore(owner);
        crate::native_gpu_output::guard(
            &request,
            OutputBoundary::BeforeCacheCommit,
            authority,
            cancellation,
        )
        .map_err(|reason| format!("recovered CPU output revoked after completion: {reason:?}"))?;
        self.cpu_display_frame(
            request,
            output.map_err(|error| format!("canonical recovered CPU output failed: {error:?}"))?,
        )
    }

    fn finish_resident_output<P: VideoFrameProvider>(
        &mut self,
        renderer: &mut VideoPreviewRenderer<P>,
        mut progress: SharedNativeOutputProgress,
        authority: &impl NativeOutputAuthority,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<ResidentOutputCompletion, String> {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            match progress {
                SharedNativeOutputProgress::HistoryReady { ready, mut history } => {
                    if !history.output_phases_valid() {
                        return Err("native Ready lacks committed output history".into());
                    }
                    for retired in history.take_retired() {
                        let (entry, owner) = retired.into_parts();
                        if owner.identity().entry != entry {
                            return Err("completed history retirement has a foreign entry".into());
                        }
                        drop(owner);
                    }
                    let evidence = ready.evidence();
                    let extent = (ready.request.key.width, ready.request.key.height);
                    let pts_ms = ready.request.pts_ms;
                    let output_mapping = ready.mapping().clone();
                    return Ok(ResidentOutputCompletion::Ready(
                        NativeResidentDisplayFrame {
                            content: ResidentDisplayContent::Resident(ready),
                            evidence,
                            extent,
                            pts_ms,
                            output_mapping,
                        },
                    ));
                }
                SharedNativeOutputProgress::Ready { ready, layers, isf } => {
                    return Err(format!(
                        "resident Display completion omitted its bound history: frame {:?}, {} validated layers, ISF submission {:?}",
                        ready.request.frame, layers.len(), isf.phase().submission(),
                    ));
                }
                SharedNativeOutputProgress::Retiring(retirement) => {
                    return Ok(ResidentOutputCompletion::Retiring(retirement))
                }
                SharedNativeOutputProgress::Rejected { pending, reason } => {
                    let retirement = pending.retire(&mut self.owner, reason);
                    return Ok(ResidentOutputCompletion::Retiring(retirement));
                }
                SharedNativeOutputProgress::Pending(pending) => {
                    if std::time::Instant::now() >= deadline {
                        let fault = NativeGpuFault {
                            context: self.owner.context().id(),
                            frame: None,
                            phase: GpuPhase::Retirement,
                            kind: NativeGpuFaultKind::ScopeDrain,
                            message: "native completion deadline expired; complete owner retained"
                                .into(),
                        };
                        let retirement =
                            pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                        return Ok(ResidentOutputCompletion::Retiring(retirement));
                    }
                    std::thread::park_timeout(std::time::Duration::from_millis(1));
                    let runtime = renderer
                        .isf_runtime
                        .as_mut()
                        .and_then(|runtime| runtime.as_mut().ok())
                        .expect("affine native preparation retains its shared runtime");
                    let Some(ResidentHistoryState::Active(history)) = self.history.as_mut() else {
                        let fault = NativeGpuFault {
                            context: self.owner.context().id(),
                            frame: None,
                            phase: GpuPhase::Retirement,
                            kind: NativeGpuFaultKind::StateMismatch,
                            message: "native pending output lost its history owner".into(),
                        };
                        let retirement =
                            pending.retire(&mut self.owner, OutputRejection::Fault(fault));
                        return Ok(ResidentOutputCompletion::Retiring(retirement));
                    };
                    progress = pending.poll_with_history(
                        &mut self.owner,
                        runtime,
                        &mut self.cache,
                        &mut history.session,
                        authority,
                        cancellation,
                    );
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "native_gpu_display_prepare_tests.rs"]
mod tests;
