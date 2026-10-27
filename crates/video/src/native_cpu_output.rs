//! ON-only canonical CPU orchestration on the resident session's GPU owner.
//!
//! The public renderer still owns the canonical scope order. This bridge only
//! supplies its existing shared ISF runtime's broker and carries actual CPU
//! charges across the canonical owned-frame moves. No shader is prepared twice.
use super::*;
use crate::isf_chain_execution::PreparedChain;
use crate::isf_native_gpu::{NativeCpuRenderFailure, PreparedNativeCpuInvocation};
use crate::native_gpu_budget::{
    AllocationCharge, ChargeKind, FrameReservation, NativeAllocationPlan,
};
use crate::native_gpu_context::NativeGpuOwner;
use crate::native_gpu_contract::{MaterializationFailure, NativeFrameId};
use crate::native_gpu_result::{NativeCpuFrameOwner, NativeCpuOutcome, NativeCpuResults};

pub(crate) struct NativeCpuOutput {
    pub(crate) result: VideoOutputArtisticRenderResult,
    pub(crate) charge: Option<Arc<AllocationCharge>>,
}

pub(crate) struct NativeFollowTransitionRequest<'a> {
    pub(crate) outgoing: Option<&'a VideoFrame>,
    pub(crate) incoming: Option<&'a VideoFrame>,
    pub(crate) extent: (u32, u32),
    pub(crate) kind: protocol::VideoClipTakeKind,
    pub(crate) curve: VideoLayerTransitionCurve,
    pub(crate) progress_millis: u16,
    pub(crate) source_timeline_id: TimelineId,
    pub(crate) transition_chain: Option<&'a ResolvedVideoEffectChain>,
    pub(crate) last_valid: Option<&'a VideoFrame>,
    pub(crate) audio_frame: Option<&'a protocol::LiveAudioTextureFrame>,
}

pub(crate) struct NativeFollowTransitionResult {
    pub(crate) payload: VideoOutputArtisticPayload,
    pub(crate) evidence: VideoFollowOutputTransitionRenderEvidence,
}

// A missing side is the existing Follow combiner's transparent-zero source,
// sampled analytically. It is never materialized as a CPU frame or GPU Ready.
fn native_follow_pixel(frame: Option<&VideoFrame>, width: usize, x: usize, y: usize) -> [u8; 4] {
    match frame {
        Some(frame) => {
            let offset = (y * width + x) * 4;
            frame.data[offset..offset + 4]
                .try_into()
                .expect("validated RGBA source")
        }
        None => [0; 4],
    }
}

fn native_follow_blurred_pixel(
    frame: Option<&VideoFrame>,
    width: usize,
    x: usize,
    y: usize,
    radius: usize,
) -> [u8; 4] {
    let Some(frame) = frame else {
        return [0; 4];
    };
    let left = x.saturating_sub(radius);
    let right = (x + radius).min(width - 1);
    let mut sum = [0u32; 4];
    for source_x in left..=right {
        let pixel = native_follow_pixel(Some(frame), width, source_x, y);
        for channel in 0..4 {
            sum[channel] += u32::from(pixel[channel]);
        }
    }
    let count = (right - left + 1) as u32;
    sum.map(|value| (value / count) as u8)
}

fn native_follow_mix_frame(
    outgoing: Option<&VideoFrame>,
    incoming: Option<&VideoFrame>,
    extent: (u32, u32),
    kind: protocol::VideoClipTakeKind,
    progress: u16,
    cancel: &dyn VideoRenderCancellation,
) -> Result<VideoFrame, VideoRuntimeError> {
    use protocol::VideoClipTakeKind as Kind;
    let (width, height) = extent;
    let length = rgba_frame_len(width, height).ok_or(CpuCompositeError::InvalidOutputSize)?;
    let mut data = vec![0u8; length];
    let p = u32::from(progress);
    let weight = |out: [u8; 4], input: [u8; 4]| {
        std::array::from_fn::<u8, 4, _>(|channel| {
            ((u32::from(out[channel]) * (1000 - p) + u32::from(input[channel]) * p + 500) / 1000)
                as u8
        })
    };
    let w = width as usize;
    for y in 0..height as usize {
        check_runtime_cancellation(Some(cancel))?;
        for x in 0..w {
            let out = native_follow_pixel(outgoing, w, x, y);
            let input = native_follow_pixel(incoming, w, x, y);
            let pixel = match kind {
                Kind::Cut => input,
                Kind::Crossfade | Kind::Custom => weight(out, input),
                Kind::Dip => {
                    let (source, source_weight) = if p <= 500 {
                        (out, 1000 - p * 2)
                    } else {
                        (input, (p - 500) * 2)
                    };
                    let mut result = source
                        .map(|channel| ((u32::from(channel) * source_weight + 500) / 1000) as u8);
                    result[3] = ((u32::from(source[3]) * source_weight
                        + 255 * (1000 - source_weight)
                        + 500)
                        / 1000) as u8;
                    result
                }
                Kind::Wipe | Kind::Luma => {
                    if p == 0 {
                        out
                    } else if p == 1000 {
                        input
                    } else {
                        let threshold = if kind == Kind::Luma {
                            (u32::from(input[0]) * 2126
                                + u32::from(input[1]) * 7152
                                + u32::from(input[2]) * 722)
                                / 2550
                        } else {
                            (x as u32 + 1) * 1000 / width
                        };
                        if threshold <= p {
                            input
                        } else {
                            out
                        }
                    }
                }
                Kind::Displacement => {
                    let span = width.min(64) as i32;
                    let out_shift = -(span * i32::from(progress) / 1000);
                    let in_shift = span * (1000 - i32::from(progress)) / 1000;
                    let out_x = (x as i32 - out_shift).rem_euclid(width as i32) as usize;
                    let in_x = (x as i32 - in_shift).rem_euclid(width as i32) as usize;
                    weight(
                        native_follow_pixel(outgoing, w, out_x, y),
                        native_follow_pixel(incoming, w, in_x, y),
                    )
                }
                Kind::Blur => weight(
                    native_follow_blurred_pixel(outgoing, w, x, y, (p * 6 / 1000) as usize),
                    native_follow_blurred_pixel(
                        incoming,
                        w,
                        x,
                        y,
                        ((1000 - p) * 6 / 1000) as usize,
                    ),
                ),
                Kind::Glitch => {
                    let band = ((y as u32 * 17 + p / 25) % 7) as i32 - 3;
                    let shift = band * (1000 - (p as i32 - 500).unsigned_abs() * 2) as i32 / 250;
                    let source_x = (x as i32 - shift).rem_euclid(width as i32) as usize;
                    weight(out, native_follow_pixel(incoming, w, source_x, y))
                }
            };
            let offset = (y * w + x) * 4;
            data[offset..offset + 4].copy_from_slice(&pixel);
        }
    }
    let selected = match kind {
        Kind::Cut => Some(incoming),
        Kind::Wipe | Kind::Luma if p == 0 => Some(outgoing),
        Kind::Wipe | Kind::Luma if p == 1000 => Some(incoming),
        _ => None,
    };
    let (layer_id, pts_ms, duration_ms) = match selected {
        Some(selected) => selected
            .map(|frame| (frame.layer_id, frame.pts_ms, frame.duration_ms))
            .unwrap_or((0, 0, 0)),
        None => (
            outgoing.map_or(0, |frame| frame.layer_id),
            outgoing
                .map_or(0, |frame| frame.pts_ms)
                .max(incoming.map_or(0, |frame| frame.pts_ms)),
            outgoing
                .map_or(0, |frame| frame.duration_ms)
                .min(incoming.map_or(0, |frame| frame.duration_ms)),
        ),
    };
    Ok(VideoFrame {
        layer_id,
        width,
        height,
        pts_ms,
        duration_ms,
        format: VideoPixelFormat::Rgba8,
        data,
    })
}

struct MovedCpuAllocation {
    pointer: usize,
    capacity: usize,
    charge: Arc<AllocationCharge>,
}

pub(crate) struct NativeCpuBridge {
    owner: NativeGpuOwner,
    frame: NativeFrameId,
    // An owned VideoFrame moves through the canonical scoped functions. Until
    // publication the enclosing affine renderer invocation owns its charge.
    // Queues, LastValid and returned output take the charge before this ends.
    moved: Vec<MovedCpuAllocation>,
    inputs: Vec<Arc<VideoFrame>>,
    aliases: Vec<Arc<NativeCpuFrameOwner>>,
    rejected: Vec<FrameReservation>,
}

struct NeverCancelled;
impl VideoRenderCancellation for NeverCancelled {
    fn is_cancelled(&self) -> bool {
        false
    }
}
static NEVER_CANCELLED: NeverCancelled = NeverCancelled;

fn native_error(message: impl Into<String>) -> VideoRuntimeError {
    VideoRuntimeError::NativeGpu(message.into())
}

fn validate_native_cpu_outcome_timing(outcome: &NativeCpuOutcome) -> Result<(), VideoRuntimeError> {
    if let Some(clocks) = outcome.clocks() {
        let frame = outcome.frame();
        // G freezes these values from its invocation input and preserves the
        // timestamp metadata through every successful chunk or fault fallback.
        // FRAMEINDEX is independent of the output request sequence.
        if clocks.time_seconds_bits != (frame.pts_ms as f32 / 1000.0).to_bits()
            || clocks.time_delta_seconds_bits != (frame.duration_ms as f32 / 1000.0).to_bits()
        {
            return Err(native_error(
                "frozen CPU timing metadata does not match its charged frame",
            ));
        }
    }
    Ok(())
}

fn materialization_error(error: MaterializationFailure) -> VideoRuntimeError {
    match error {
        MaterializationFailure::Cancelled { .. } => VideoRuntimeError::Cancelled,
        MaterializationFailure::Fault { fault, retirement } => native_error(format!(
            "{}; retained retirement: {retirement:?}",
            fault.message,
        )),
    }
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    /// The owner remains stored in the renderer while the closure executes,
    /// including unwinding. The session can reclaim it through the Stop hook.
    pub(crate) fn with_native_cpu_owner<T>(
        &mut self,
        owner: NativeGpuOwner,
        frame: NativeFrameId,
        operation: impl FnOnce(&mut Self) -> Result<T, VideoPreviewError>,
    ) -> (NativeGpuOwner, Result<T, VideoPreviewError>) {
        if self.native_cpu_bridge.is_some()
            || owner.context().id().owner_id != frame.output_id
            || self
                .isf_runtime
                .as_ref()
                .and_then(|runtime| runtime.as_ref().ok())
                .and_then(IsfGpuRuntime::native_context)
                .is_none_or(|context| !Arc::ptr_eq(context, owner.context()))
        {
            return (
                owner,
                Err(VideoPreviewError::Runtime(native_error(
                    "canonical CPU bridge requires its sole shared owner and runtime",
                ))),
            );
        }
        let mut aliases = Vec::<Arc<NativeCpuFrameOwner>>::new();
        for queue in &self.runtime.queues {
            for index in 0..queue.len {
                let whole = match queue.native_cpu_owners[index].as_ref() {
                    Some(whole) => Some(Arc::clone(whole)),
                    None => match queue.native_owners[index].as_ref() {
                        Some(entry) => match entry.native_cpu_owner() {
                            Ok(whole) => Some(whole),
                            Err(error) => return (owner, Err(VideoPreviewError::Runtime(native_error(
                                format!("canonical history owner cannot enter shared CPU execution: {error:?}"),
                            )))),
                        },
                        None => None,
                    },
                };
                if let Some(whole) = whole {
                    match aliases
                        .iter_mut()
                        .find(|prior| Arc::ptr_eq(&prior.frame, &whole.frame))
                    {
                        Some(prior) if prior.charge.is_none() && whole.charge.is_some() => {
                            *prior = whole
                        }
                        Some(_) => {}
                        None => aliases.push(whole),
                    }
                }
            }
        }
        let runtime = self.isf_runtime.as_mut().unwrap().as_mut().unwrap();
        for whole in &aliases {
            runtime.remember_native_cpu_frame(whole);
        }
        self.native_cpu_bridge = Some(NativeCpuBridge {
            owner,
            frame,
            moved: Vec::new(),
            inputs: Vec::new(),
            aliases,
            rejected: Vec::new(),
        });
        let result = operation(self);
        let bridge = self
            .native_cpu_bridge
            .take()
            .expect("affine native CPU operation");
        self.native_cpu_retained_reservations
            .extend(bridge.rejected);
        (bridge.owner, result)
    }

    pub(crate) fn prepare_native_cpu_output(
        &mut self,
        owner: NativeGpuOwner,
        frame: NativeFrameId,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
        cancel: &dyn VideoRenderCancellation,
    ) -> (NativeGpuOwner, Result<NativeCpuOutput, VideoPreviewError>) {
        self.with_native_cpu_owner(owner, frame, |renderer| {
            let result = renderer.prepare_native_cpu_artistic_with_plan(
                snapshot,
                context,
                transitions,
                plan,
                width,
                height,
                cancel,
            )?;
            renderer.capture_native_cpu_output(result)
        })
    }

    /// Runs the canonical remaining scopes after the exact G4 Layer decline.
    /// The caller has already completed the all-history handover. No input is
    /// acquired and no Clip/Layer chain, DATE or clock is prepared again here.
    pub(crate) fn finish_native_cpu_layers(
        &mut self,
        owner: NativeGpuOwner,
        frame: NativeFrameId,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
        cancel: &dyn VideoRenderCancellation,
        results: NativeCpuResults,
    ) -> (NativeGpuOwner, Result<NativeCpuOutput, VideoPreviewError>) {
        self.with_native_cpu_owner(owner, frame, |renderer| {
            check_render_cancellation(Some(cancel))?;
            let outcomes = results.outcomes();
            if outcomes.len() != plan.composition.layers.len()
                || outcomes
                    .iter()
                    .zip(&plan.composition.layers)
                    .any(|(outcome, layer)| {
                        outcome.scope()
                            != &(VideoEffectScope::Layer {
                                layer_id: layer.layer_id,
                            })
                            || outcome.frame().layer_id != layer.layer_id
                            || outcome.frame().width == 0
                            || outcome.frame().height == 0
                            || outcome.frame().format != VideoPixelFormat::Rgba8
                            || gpu_composition_frame_len(outcome.frame())
                                != Some(outcome.frame().data.len())
                    })
            {
                return Err(VideoPreviewError::Runtime(native_error(
                    "prepared CPU layer outcomes do not match the frozen output plan",
                )));
            }
            let mut faults = Vec::new();
            let mut owners = Vec::with_capacity(outcomes.len());
            for outcome in outcomes {
                validate_native_cpu_outcome_timing(outcome).map_err(VideoPreviewError::Runtime)?;
                owners.push(outcome.charged_owner());
                faults.extend_from_slice(outcome.faults());
            }
            renderer.clear_full_output_effect_diagnostics(snapshot);
            renderer.record_effect_faults(faults.clone(), snapshot);
            for cpu in owners {
                renderer.publish_native_cpu_owner(cpu);
            }
            for layer in &plan.composition.layers {
                let fault = faults.iter().find(|fault| match fault.scope {
                    VideoEffectScope::Layer { layer_id }
                    | VideoEffectScope::Clip { layer_id, .. } => layer_id == layer.layer_id,
                    _ => false,
                });
                renderer.publish_isf_layer_frame(
                    layer.layer_id,
                    layer.position_ms,
                    true,
                    fault.map(|fault| fault.message.as_str()),
                );
            }
            let result = renderer.prepare_native_cpu_artistic_impl(
                snapshot,
                context,
                Some(transitions),
                plan,
                width,
                height,
                Some(cancel),
                Some(faults),
            )?;
            renderer.capture_native_cpu_output(result)
        })
    }

    /// Complete the remaining canonical scopes after fixed-clock G recovery
    /// has been sealed into P and its full CPU handover installed these queues.
    /// Selection/publication uses those exact frames; no source or Layer work
    /// is acquired or prepared here and recovery faults remain canonical faults.
    pub(crate) fn finish_native_cpu_history(
        &mut self,
        owner: NativeGpuOwner,
        frame: NativeFrameId,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
        cancel: &dyn VideoRenderCancellation,
        faults: Vec<VideoEffectStageFault>,
    ) -> (NativeGpuOwner, Result<NativeCpuOutput, VideoPreviewError>) {
        self.with_native_cpu_owner(owner, frame, |renderer| {
            check_render_cancellation(Some(cancel))?;
            renderer.set_isf_audio_frame(context.audio_frame);
            renderer.isf_render_namespace =
                (context.project_render_epoch, Some(plan.output_id), false);
            renderer.clear_full_output_effect_diagnostics(snapshot);
            renderer.record_effect_faults(faults.clone(), snapshot);
            for layer in &plan.composition.layers {
                let fault = faults.iter().find(|fault| match fault.scope {
                    VideoEffectScope::Layer { layer_id }
                    | VideoEffectScope::Clip { layer_id, .. } => layer_id == layer.layer_id,
                    _ => false,
                });
                renderer.publish_isf_layer_frame(
                    layer.layer_id,
                    layer.position_ms,
                    true,
                    fault.map(|fault| fault.message.as_str()),
                );
            }
            let result = renderer.prepare_native_cpu_artistic_impl(
                snapshot,
                context,
                Some(transitions),
                plan,
                width,
                height,
                Some(cancel),
                Some(faults),
            )?;
            renderer.capture_native_cpu_output(result)
        })
    }

    /// Also used for each side of Timeline Follow while one owner is installed.
    /// The supplied plan has already received the canonical transition weights.
    pub(crate) fn prepare_native_cpu_artistic_with_plan(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: &VideoLayerTransitionRuntimeSnapshot,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<VideoOutputArtisticRenderResult, VideoPreviewError> {
        if self.native_cpu_bridge.is_none() {
            return Err(VideoPreviewError::Runtime(native_error(
                "native CPU owner is not installed",
            )));
        }
        self.set_isf_audio_frame(context.audio_frame);
        self.isf_render_namespace = (context.project_render_epoch, Some(plan.output_id), false);
        self.prepare_native_cpu_artistic_impl(
            snapshot,
            context,
            Some(transitions),
            plan,
            width,
            height,
            Some(cancel),
            None,
        )
    }

    pub(crate) fn render_native_follow_output_transition(
        &mut self,
        request: NativeFollowTransitionRequest<'_>,
        cancel: &dyn VideoRenderCancellation,
    ) -> Result<NativeFollowTransitionResult, VideoPreviewError> {
        check_render_cancellation(Some(cancel))?;
        let NativeFollowTransitionRequest {
            outgoing,
            incoming,
            extent,
            kind,
            curve,
            progress_millis,
            source_timeline_id,
            transition_chain,
            last_valid,
            audio_frame,
        } = request;
        let (width, height) = extent;
        let length = rgba_frame_len(width, height)
            .filter(|length| *length != 0)
            .ok_or(VideoPreviewError::InvalidSize)?;
        if progress_millis > 1000 {
            return Err(VideoPreviewError::Runtime(
                CpuCompositeError::InvalidTransitionProgress { progress_millis }.into(),
            ));
        }
        let curved = video_follow_output_transition_progress_millis(curve, progress_millis);
        let transition = VideoFollowOutputTransitionEvidence {
            kind,
            curve,
            raw_progress_millis: progress_millis,
            curved_progress_millis: curved,
        };
        if outgoing.is_none() && incoming.is_none() {
            return Ok(NativeFollowTransitionResult {
                payload: VideoOutputArtisticPayload::HardBlackout,
                evidence: VideoFollowOutputTransitionRenderEvidence {
                    transition,
                    freshness: VideoOutputRenderFreshness::Fresh,
                    error: None,
                    stage_faults: Vec::new(),
                },
            });
        }
        for frame in outgoing.into_iter().chain(incoming).chain(last_valid) {
            if frame.format != VideoPixelFormat::Rgba8
                || frame.width != width
                || frame.height != height
                || frame.data.len() != length
            {
                return Err(VideoPreviewError::Runtime(
                    CpuCompositeError::FrameSizeMismatch {
                        layer_id: frame.layer_id,
                    }
                    .into(),
                ));
            }
        }
        if let Some(chain) = transition_chain {
            match &chain.scope {
                VideoEffectScope::Transition { owner }
                    if *owner
                        == (VideoTransitionEffectOwner::TimelineFollow { source_timeline_id }) => {}
                VideoEffectScope::Transition { owner } => {
                    return Err(VideoPreviewError::Runtime(
                        CpuCompositeError::InvalidFollowTransitionEffectOwner {
                            expected_source_timeline_id: source_timeline_id,
                            owner: owner.clone(),
                        }
                        .into(),
                    ))
                }
                _ => {
                    return Err(VideoPreviewError::Runtime(
                        CpuCompositeError::InvalidTransitionEffectScope {
                            scope: chain.scope.clone(),
                        }
                        .into(),
                    ))
                }
            }
        }
        // One exact-sized mixed output allocation. Missing sides and the
        // displacement/blur/glitch intermediates are sampled directly.
        let charge = self
            .reserve_native_cpu_bytes(length as u64, ChargeKind::CpuSnapshot, Some(cancel))
            .map_err(VideoPreviewError::Runtime)?;
        let base = native_follow_mix_frame(outgoing, incoming, extent, kind, curved, cancel)
            .map_err(VideoPreviewError::Runtime)?;
        self.track_native_cpu_frame(&base, Some(charge))
            .map_err(VideoPreviewError::Runtime)?;
        self.set_isf_audio_frame(audio_frame);
        self.isf_frame_inputs.begin_frame();
        let (rendered, faults) = match transition_chain {
            Some(chain) => self
                .apply_resolved_effect_chain_to_frame_with_cancellation(
                    chain,
                    base,
                    None,
                    Some(cancel),
                )
                .map_err(VideoPreviewError::Runtime)?,
            None => (base, Vec::new()),
        };
        check_render_cancellation(Some(cancel))?;
        let error = (!faults.is_empty()).then(|| {
            faults
                .iter()
                .map(|fault| match fault.stage_label.as_deref() {
                    Some(label) => format!("{label}: {}", fault.message),
                    None => fault.message.clone(),
                })
                .collect::<Vec<_>>()
                .join("; ")
        });
        let (frame, freshness) = if error.is_none() {
            (rendered, VideoOutputRenderFreshness::Fresh)
        } else if let Some(fallback) = last_valid {
            let charge = self
                .reserve_native_cpu_bytes(length as u64, ChargeKind::CpuSnapshot, Some(cancel))
                .map_err(VideoPreviewError::Runtime)?;
            let frame = fallback.clone();
            self.track_native_cpu_frame(&frame, Some(charge))
                .map_err(VideoPreviewError::Runtime)?;
            (frame, VideoOutputRenderFreshness::LastValid)
        } else {
            (rendered, VideoOutputRenderFreshness::Error)
        };
        Ok(NativeFollowTransitionResult {
            payload: VideoOutputArtisticPayload::Frame(frame),
            evidence: VideoFollowOutputTransitionRenderEvidence {
                transition,
                freshness,
                error,
                stage_faults: faults,
            },
        })
    }

    pub(crate) fn reserve_native_cpu_scratch(
        &mut self,
        width: u32,
        height: u32,
        frames: u64,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Option<Arc<AllocationCharge>>, VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            return Ok(None);
        }
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .and_then(u64::checked_next_power_of_two)
            .and_then(|bytes| bytes.checked_mul(frames))
            .filter(|bytes| *bytes > 0)
            .ok_or_else(|| native_error("CPU temporary capacity overflow"))?;
        self.reserve_native_cpu_bytes(bytes, ChargeKind::CpuSnapshot, cancel)
            .map(Some)
    }

    pub(crate) fn reserve_native_cpu_conversion_scratch<F: std::borrow::Borrow<VideoFrame>>(
        &mut self,
        frames: &[F],
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Option<Arc<AllocationCharge>>, VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            return Ok(None);
        }
        let bytes = frames
            .iter()
            .map(std::borrow::Borrow::borrow)
            .filter(|frame: &&VideoFrame| frame.format != VideoPixelFormat::Rgba8)
            .try_fold(0u64, |peak, frame| {
                u64::from(frame.width)
                    .checked_mul(u64::from(frame.height))
                    .and_then(|pixels| pixels.checked_mul(4))
                    .and_then(u64::checked_next_power_of_two)
                    .map(|bytes| peak.max(bytes))
                    .ok_or_else(|| native_error("CPU normalization capacity overflow"))
            })?;
        if bytes == 0 {
            return Ok(None);
        }
        self.reserve_native_cpu_bytes(bytes, ChargeKind::CpuSnapshot, cancel)
            .map(Some)
    }

    pub(crate) fn capture_native_cpu_output(
        &mut self,
        result: VideoOutputArtisticRenderResult,
    ) -> Result<NativeCpuOutput, VideoPreviewError> {
        let charge = match &result.payload {
            VideoOutputArtisticPayload::HardBlackout => None,
            VideoOutputArtisticPayload::Frame(frame) => {
                Some(self.take_native_cpu_charge(frame).ok_or_else(|| {
                    VideoPreviewError::Runtime(native_error(
                        "canonical CPU output has no retained allocation charge",
                    ))
                })?)
            }
        };
        Ok(NativeCpuOutput { result, charge })
    }

    pub(crate) fn release_native_cpu_ownership_for_stop(&mut self) -> Option<NativeGpuOwner> {
        // Image bindings alias queue pixels without carrying their charge.
        // Release those views before their complete queue/cache owners.
        self.isf_frame_inputs.layers.clear();
        self.isf_frame_inputs.sampled_layers.clear();
        self.runtime.queues.clear();
        self.output_last_valid_frames.clear();
        // Undiscardable reservations remain owned until this explicit terminal
        // cleanup boundary, beside the same broker that admitted them.
        self.native_cpu_retained_reservations.clear();
        self.native_cpu_bridge.take().map(|bridge| bridge.owner)
    }

    pub(crate) fn reserve_native_cpu_rgba(
        &mut self,
        width: u32,
        height: u32,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Option<Arc<AllocationCharge>>, VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            return Ok(None);
        }
        let bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes > 0)
            .and_then(u64::checked_next_power_of_two)
            .ok_or_else(|| native_error("native CPU extent overflow"))?;
        self.reserve_native_cpu_bytes(bytes, ChargeKind::CpuSnapshot, cancel)
            .map(Some)
    }

    fn reserve_native_cpu_bytes(
        &mut self,
        bytes: u64,
        kind: ChargeKind,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Arc<AllocationCharge>, VideoRuntimeError> {
        check_runtime_cancellation(cancel)?;
        let bridge = self
            .native_cpu_bridge
            .as_mut()
            .ok_or_else(|| native_error("native CPU allocation has no broker"))?;
        bridge
            .owner
            .poll_retirement()
            .map_err(|fault| native_error(fault.message))?;
        let mut plan = NativeAllocationPlan::new(bridge.frame);
        plan.add_new(kind, bytes)
            .map_err(|reason| native_error(format!("CPU allocation plan: {reason:?}")))?;
        let quote = bridge
            .owner
            .quote(&plan)
            .map_err(|reason| native_error(format!("CPU allocation budget: {reason:?}")))?;
        let reservation = bridge
            .owner
            .reserve(quote)
            .map_err(|reason| native_error(format!("CPU allocation reservation: {reason:?}")))?;
        let mut transaction = bridge
            .owner
            .begin_frame(bridge.frame, reservation)
            .map_err(|fault| native_error(fault.message))?;
        let charge = Arc::new(
            transaction
                .bind_allocation(kind, bytes)
                .map_err(|fault| native_error(fault.message))?,
        );
        let pending = transaction.finish();
        crate::isf_native_gpu::await_transaction(
            &mut bridge.owner,
            pending,
            cancel.unwrap_or(&NEVER_CANCELLED),
        )
        .map_err(materialization_error)?;
        check_runtime_cancellation(cancel)?;
        Ok(charge)
    }

    pub(crate) fn track_native_cpu_bytes(
        &mut self,
        data: &Vec<u8>,
        charge: Option<Arc<AllocationCharge>>,
    ) -> Result<(), VideoRuntimeError> {
        let Some(charge) = charge else {
            return Ok(());
        };
        if data.capacity() as u64 > charge.capacity() {
            return Err(native_error(
                "CPU allocation exceeded its admitted capacity",
            ));
        }
        let bridge = self
            .native_cpu_bridge
            .as_mut()
            .ok_or_else(|| native_error("charged CPU frame escaped its owning invocation"))?;
        let pointer = data.as_ptr() as usize;
        bridge.moved.retain(|prior| prior.pointer != pointer);
        bridge.moved.push(MovedCpuAllocation {
            pointer,
            capacity: data.capacity(),
            charge,
        });
        Ok(())
    }

    pub(crate) fn track_native_cpu_frame(
        &mut self,
        frame: &VideoFrame,
        charge: Option<Arc<AllocationCharge>>,
    ) -> Result<(), VideoRuntimeError> {
        self.track_native_cpu_bytes(&frame.data, charge)
    }

    fn take_native_cpu_charge(&mut self, frame: &VideoFrame) -> Option<Arc<AllocationCharge>> {
        let bridge = self.native_cpu_bridge.as_mut()?;
        let index = bridge.moved.iter().position(|entry| {
            entry.pointer == frame.data.as_ptr() as usize && entry.capacity == frame.data.capacity()
        })?;
        Some(bridge.moved.swap_remove(index).charge)
    }

    fn retain_native_cpu_input(
        &mut self,
        frame: &Arc<VideoFrame>,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Arc<NativeCpuFrameOwner>, VideoRuntimeError> {
        check_runtime_cancellation(cancel)?;
        let prior = self.native_cpu_bridge.as_ref().and_then(|bridge| {
            bridge
                .aliases
                .iter()
                .find(|owner| Arc::ptr_eq(&owner.frame, frame))
                .cloned()
        });
        if let Some(owner) = &prior {
            if owner.charge.is_some() || frame.data.capacity() == 0 {
                return Ok(Arc::clone(owner));
            }
        }
        let charge = if frame.data.capacity() == 0 {
            None
        } else {
            Some(self.reserve_native_cpu_bytes(
                frame.data.capacity() as u64,
                ChargeKind::ActiveCpuInput,
                cancel,
            )?)
        };
        // Imported P backing has a host-history owner but no managed charge.
        // Keep that complete owner when activating these same pixels in G's
        // ledger; dropping the upstream owner here would sever its host budget.
        let owner = match (prior, charge) {
            (Some(prior), Some(charge)) => {
                crate::native_gpu_history::adapter::activate_native_cpu_owner(prior, charge)
            }
            (Some(prior), None) => prior,
            (None, charge) => Arc::new(NativeCpuFrameOwner::new(Arc::clone(frame), charge)),
        };
        let runtime = self
            .isf_runtime
            .as_mut()
            .and_then(|runtime| runtime.as_mut().ok())
            .ok_or_else(|| native_error("native CPU input lost its shared runtime"))?;
        runtime.remember_native_cpu_frame(&owner);
        let aliases = &mut self
            .native_cpu_bridge
            .as_mut()
            .expect("input reservation has broker")
            .aliases;
        aliases.retain(|prior| !Arc::ptr_eq(&prior.frame, frame));
        aliases.push(Arc::clone(&owner));
        Ok(owner)
    }

    pub(crate) fn remember_native_cpu_input(
        &mut self,
        frame: &Arc<VideoFrame>,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(), VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            return Ok(());
        }
        self.retain_native_cpu_input(frame, cancel)?;
        let bridge = self
            .native_cpu_bridge
            .as_mut()
            .expect("native input owner installed");
        if !bridge.inputs.iter().any(|input| Arc::ptr_eq(input, frame)) {
            bridge.inputs.push(Arc::clone(frame));
        }
        Ok(())
    }

    pub(crate) fn remember_native_cpu_owned_input(
        &mut self,
        frame: &VideoFrame,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(), VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() || frame.data.capacity() == 0 {
            return Ok(());
        }
        // Owned ClipTake acquisitions are distinct live allocations. Admit
        // both sources before either Clip chain can consume GPU resources.
        let charge = self.reserve_native_cpu_bytes(
            frame.data.capacity() as u64,
            ChargeKind::ActiveCpuInput,
            cancel,
        )?;
        self.track_native_cpu_frame(frame, Some(charge))
    }

    pub(crate) fn publish_native_cpu_raw_frame(
        &mut self,
        frame: Arc<VideoFrame>,
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(), VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            self.runtime.push_shared_frame(frame);
        } else {
            let owner = self.retain_native_cpu_input(&frame, cancel)?;
            self.publish_native_cpu_owner(owner);
        }
        Ok(())
    }

    pub(crate) fn publish_native_cpu_owner(&mut self, owner: Arc<NativeCpuFrameOwner>) {
        let frame = Arc::clone(&owner.frame);
        if let Some(bridge) = self.native_cpu_bridge.as_mut() {
            // A newly completed Layer/transition frame may be selected again
            // during this same invocation. Register its whole existing charge
            // before composition/image inputs can admit it as a raw source.
            bridge
                .aliases
                .retain(|prior| !Arc::ptr_eq(&prior.frame, &frame));
            bridge.aliases.push(Arc::clone(&owner));
            self.isf_runtime
                .as_mut()
                .and_then(|runtime| runtime.as_mut().ok())
                .expect("native CPU publication retains its shared runtime")
                .remember_native_cpu_frame(&owner);
        }
        self.runtime.push_shared_frame(Arc::clone(&frame));
        if let Some(queue) = self
            .runtime
            .queues
            .iter_mut()
            .find(|queue| queue.layer_id == frame.layer_id)
        {
            if let Some(index) = queue.slots[..queue.len]
                .iter()
                .position(|slot| slot.as_ref().is_some_and(|slot| Arc::ptr_eq(slot, &frame)))
            {
                queue.native_cpu_owners[index] = Some(owner);
            }
        }
    }

    pub(crate) fn publish_native_cpu_frame(
        &mut self,
        frame: VideoFrame,
    ) -> Result<(), VideoRuntimeError> {
        if self.native_cpu_bridge.is_none() {
            self.runtime.push_frame(frame);
            return Ok(());
        }
        let charge = self.take_native_cpu_charge(&frame);
        let frame = Arc::new(frame);
        self.publish_native_cpu_owner(Arc::new(NativeCpuFrameOwner::new(frame, charge)));
        Ok(())
    }

    pub(crate) fn publish_native_cpu_cow(
        &mut self,
        frame: Cow<'_, VideoFrame>,
        original: Arc<VideoFrame>,
    ) -> Result<(), VideoRuntimeError> {
        match frame {
            Cow::Owned(frame) => self.publish_native_cpu_frame(frame),
            Cow::Borrowed(_) => {
                let owner = self.native_cpu_bridge.as_ref().and_then(|bridge| {
                    bridge
                        .aliases
                        .iter()
                        .find(|owner| Arc::ptr_eq(&owner.frame, &original))
                        .cloned()
                });
                if let Some(owner) = owner {
                    self.publish_native_cpu_owner(owner);
                } else {
                    self.runtime.push_shared_frame(original);
                }
                Ok(())
            }
        }
    }

    pub(crate) fn execute_native_prepared_chain<'frame>(
        &mut self,
        prepared: PreparedChain<'_>,
        frame: Cow<'frame, VideoFrame>,
        date: [f32; 4],
        cancel: Option<&dyn VideoRenderCancellation>,
    ) -> Result<(Cow<'frame, VideoFrame>, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        check_runtime_cancellation(cancel)?;
        let borrowed = match &frame {
            Cow::Borrowed(frame) => Some(*frame),
            Cow::Owned(_) => None,
        };
        // Only owned canonical intermediates use the affine Vec move table.
        // Borrowed input pixels already have an exact whole-owner alias; their
        // allocator address may match a retired intermediate's old address.
        let charge = if borrowed.is_none() {
            self.take_native_cpu_charge(&frame)
        } else {
            None
        };
        let input = match frame {
            Cow::Owned(frame) => Arc::new(frame),
            Cow::Borrowed(frame) => match self.native_cpu_bridge.as_ref().and_then(|bridge| {
                bridge
                    .inputs
                    .iter()
                    .find(|input| std::ptr::eq(input.as_ref(), frame))
                    .cloned()
            }) {
                Some(frame) => frame,
                None => {
                    return Err(native_error(
                        "borrowed canonical input has no exact shared owner",
                    ))
                }
            },
        };
        let charged_input = charge
            .map(|charge| Arc::new(NativeCpuFrameOwner::new(Arc::clone(&input), Some(charge))));
        let prepared = PreparedChain {
            source: Cow::Owned(prepared.source.into_owned()),
            stages: prepared.stages,
            keys: prepared.keys,
            faults: prepared.faults,
        };
        let invocation =
            PreparedNativeCpuInvocation::new(input, prepared, date, self.isf_audio_frame.clone());
        let runtime = self
            .isf_runtime
            .as_mut()
            .and_then(|runtime| runtime.as_mut().ok())
            .ok_or_else(|| native_error("native CPU scoped executor lost its shared runtime"))?;
        if let Some(input) = &charged_input {
            runtime.remember_native_cpu_frame(input);
        }
        let bridge = self
            .native_cpu_bridge
            .as_mut()
            .expect("native branch owns broker");
        let inputs = runtime
            .prepare_native_cpu_inputs(bridge.frame, invocation)
            .map_err(|fault| native_error(fault.message))?;
        let plan = inputs
            .allocation_plan()
            .map_err(|reason| native_error(format!("CPU scoped plan declined: {reason:?}")))?;
        let quote = bridge
            .owner
            .quote(plan)
            .map_err(|reason| native_error(format!("CPU scoped budget declined: {reason:?}")))?;
        let reservation = bridge.owner.reserve(quote).map_err(|reason| {
            native_error(format!("CPU scoped reservation declined: {reason:?}"))
        })?;
        let results = runtime
            .render_native_cpu_fallback(
                &mut bridge.owner,
                inputs,
                reservation,
                cancel.unwrap_or(&NEVER_CANCELLED),
            )
            .map_err(|error| match error {
                NativeCpuRenderFailure::Cancelled { .. } => VideoRuntimeError::Cancelled,
                NativeCpuRenderFailure::Fault { fault, retirement } => native_error(format!(
                    "{}; retained retirement: {retirement:?}",
                    fault.message
                )),
                NativeCpuRenderFailure::Admission {
                    fault, reservation, ..
                } => {
                    if let Err((_, reservation)) =
                        bridge.owner.discard_unsubmitted_reservation(reservation)
                    {
                        bridge.rejected.push(reservation);
                    }
                    native_error(fault.message)
                }
                NativeCpuRenderFailure::Declined { reason, .. } => native_error(format!(
                    "CPU scoped execution declined before execution: {reason:?}"
                )),
            })?;
        drop(charged_input);
        if results.outcomes().len() != 1 {
            return Err(native_error(
                "scoped CPU invocation returned a different result count",
            ));
        }
        let outcome = &results.outcomes()[0];
        validate_native_cpu_outcome_timing(outcome)?;
        let owner = outcome.charged_owner();
        let faults = outcome.faults().to_vec();
        drop(results);
        if let Some(original) =
            borrowed.filter(|original| std::ptr::eq(*original, owner.frame.as_ref()))
        {
            self.native_cpu_bridge
                .as_mut()
                .expect("bridge retained")
                .aliases
                .push(owner);
            return Ok((Cow::Borrowed(original), faults));
        }
        let owner = Arc::try_unwrap(owner)
            .map_err(|_| native_error("completed CPU result still has another whole owner"))?;
        let frame = Arc::try_unwrap(owner.frame)
            .map_err(|_| native_error("completed CPU result pixels still have another owner"))?;
        self.track_native_cpu_frame(&frame, owner.charge)?;
        check_runtime_cancellation(cancel)?;
        Ok((Cow::Owned(frame), faults))
    }

    pub(crate) fn prepare_native_cpu_artistic_impl(
        &mut self,
        snapshot: &VideoSnapshot,
        context: VideoEffectRenderContext<'_>,
        transitions: Option<&VideoLayerTransitionRuntimeSnapshot>,
        plan: &VideoOutputRenderPlan,
        width: u32,
        height: u32,
        cancel: Option<&dyn VideoRenderCancellation>,
        prepared: Option<Vec<VideoEffectStageFault>>,
    ) -> Result<VideoOutputArtisticRenderResult, VideoPreviewError> {
        check_render_cancellation(cancel)?;
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let make = |payload, freshness, error| VideoOutputArtisticRenderResult {
            payload,
            output_mapping: plan.mapping,
            output_mapping_identity: VideoOutputMappingIdentity::from_mapping(&plan.mapping),
            evidence: VideoOutputRenderEvidence {
                project_render_epoch: context.project_render_epoch,
                output_id: plan.output_id,
                freshness,
                error,
            },
        };
        if plan.output_blackout || plan.output_opacity <= f32::EPSILON || plan.composition.blackout
        {
            self.fence_output_artistic_blackout(snapshot, plan.output_id);
            return Ok(make(
                VideoOutputArtisticPayload::HardBlackout,
                VideoOutputRenderFreshness::Fresh,
                None,
            ));
        }
        let key = VideoOutputLastValidKey {
            project_render_epoch: context.project_render_epoch,
            output_id: plan.output_id,
            width,
            height,
            composition_id: plan.composition.composition_id,
            kind: plan.kind.clone(),
            fullscreen: plan.fullscreen,
            monitor_id: plan.monitor_id,
            endpoint_name: plan.endpoint_name.clone(),
        };
        self.output_last_valid_frames.retain(|entry| {
            entry.key.project_render_epoch == key.project_render_epoch
                && (entry.key.output_id != key.output_id || entry.key == key)
        });
        let rendered = self.prepare_output_artistic_frame_with_effects_impl_with_cancellation(
            snapshot,
            context,
            transitions,
            plan,
            width,
            height,
            cancel,
            prepared,
        );
        match rendered {
            Ok(frame) if self.last_effect_stage_faults.is_empty() => {
                check_render_cancellation(cancel)?;
                let charge = self
                    .reserve_native_cpu_bytes(
                        frame.data.len() as u64,
                        ChargeKind::CpuLastValid,
                        cancel,
                    )
                    .map_err(VideoPreviewError::Runtime)?;
                let cached = frame.clone();
                if cached.data.capacity() as u64 > charge.capacity() {
                    return Err(VideoPreviewError::Runtime(native_error(
                        "LastValid clone exceeds charged capacity",
                    )));
                }
                check_render_cancellation(cancel)?;
                self.output_last_valid_frames
                    .retain(|entry| entry.key != key);
                self.output_last_valid_frames
                    .push(VideoOutputLastValidFrame {
                        key,
                        frame: cached,
                        native_snapshot_owner: None,
                        native_cpu_charge: Some(charge),
                    });
                self.last_output_render_error = None;
                Ok(make(
                    VideoOutputArtisticPayload::Frame(frame),
                    VideoOutputRenderFreshness::Fresh,
                    None,
                ))
            }
            Ok(frame) => {
                let error = self
                    .last_isf_error
                    .clone()
                    .unwrap_or_else(|| "artistic effect chain reported a stage fault".into());
                self.last_output_render_error = Some(error.clone());
                Ok(make(
                    VideoOutputArtisticPayload::Frame(frame),
                    VideoOutputRenderFreshness::Error,
                    Some(error),
                ))
            }
            Err(error) if error.is_cancelled() => Err(VideoPreviewError::Cancelled),
            Err(error @ VideoPreviewError::Runtime(VideoRuntimeError::NativeGpu(_))) => Err(error),
            Err(error) => {
                let message = format!("{error:?}");
                self.last_output_render_error = Some(message.clone());
                let index = self
                    .output_last_valid_frames
                    .iter()
                    .position(|entry| entry.key == key);
                let charge = self
                    .reserve_native_cpu_rgba(width, height, cancel)
                    .map_err(VideoPreviewError::Runtime)?;
                let (frame, freshness) = if let Some(index) = index {
                    let entry = &self.output_last_valid_frames[index];
                    if entry.native_cpu_charge.as_ref().is_some_and(|charge| {
                        charge.capacity() < entry.frame.data.capacity() as u64
                    }) {
                        return Err(VideoPreviewError::Runtime(native_error(
                            "LastValid CPU charge is smaller than its allocation",
                        )));
                    }
                    (entry.frame.clone(), VideoOutputRenderFreshness::LastValid)
                } else {
                    (
                        video_output_transparent_black_frame(width, height),
                        VideoOutputRenderFreshness::Error,
                    )
                };
                self.track_native_cpu_frame(&frame, charge)
                    .map_err(VideoPreviewError::Runtime)?;
                Ok(make(
                    VideoOutputArtisticPayload::Frame(frame),
                    freshness,
                    Some(message),
                ))
            }
        }
    }
}

#[cfg(test)]
#[path = "native_cpu_output_tests.rs"]
mod tests;
