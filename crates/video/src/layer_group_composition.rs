//! Group-image policy shared by artistic CPU rendering and native GPU preparation.
//! The latter uploads only an isolated group's result; output blending and mapping
//! still run through the presenter's existing, parity-tested GPU passes.
use super::*;

pub(super) struct ActiveLayerGroup<'a> {
    pub group: &'a protocol::VideoLayerGroupSummary,
    chain: Option<ResolvedVideoEffectChain>,
}

fn group_transform(group: &protocol::VideoLayerGroupSummary) -> Transform2D {
    // Same bounds as a layer's sanitised transform; protocol validation already
    // rejects values outside them, so this only guards unvalidated snapshots.
    let mut transform = group.transform.unwrap_or_default();
    transform.scale_x = transform.scale_x.clamp(0.001, 100.0);
    transform.scale_y = transform.scale_y.clamp(0.001, 100.0);
    transform
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    pub(super) fn prepare_grouped_gpu_layers(
        &mut self,
        snapshot: &VideoSnapshot,
        plan: &CompositionPlan,
        frames: &[Arc<VideoFrame>],
        size: (u32, u32),
    ) -> Result<(Vec<VideoOutputGpuLayer>, Vec<VideoEffectStageFault>), VideoRuntimeError> {
        let mut faults = Vec::new();
        let active_groups = self.active_layer_groups(snapshot, plan, &mut faults);
        let mut layers = Vec::with_capacity(plan.layers.len());
        for layer in &plan.layers {
            let active = active_groups
                .iter()
                .find(|active| active.group.layer_ids.contains(&layer.layer_id));
            let (outer, isolated) = if let Some(active) = active {
                if plan
                    .layers
                    .iter()
                    .find(|candidate| active.group.layer_ids.contains(&candidate.layer_id))
                    .map(|candidate| candidate.layer_id)
                    != Some(layer.layer_id)
                {
                    continue;
                }
                let (outer, frame, group_faults) =
                    self.prepare_group_layer(active, plan, frames, size, None)?;
                faults.extend(group_faults);
                (outer, frame)
            } else {
                (layer.clone(), None)
            };
            let frame = match isolated {
                Some(frame) => frame,
                None => Arc::clone(
                    frames
                        .iter()
                        .find(|frame| frame.layer_id == layer.layer_id)
                        .ok_or(CpuCompositeError::MissingFrame {
                            layer_id: layer.layer_id,
                        })?,
                ),
            };
            layers.push(VideoOutputGpuLayer { plan: outer, frame });
        }
        Ok((layers, faults))
    }

    pub(super) fn active_layer_groups<'a>(
        &self,
        snapshot: &'a VideoSnapshot,
        plan: &CompositionPlan,
        faults: &mut Vec<VideoEffectStageFault>,
    ) -> Vec<ActiveLayerGroup<'a>> {
        snapshot
            .layer_groups
            .iter()
            .filter(|group| {
                group.composition_id == plan.composition_id && group.enabled && !group.bypassed
            })
            .filter_map(|group| {
                let chain = match resolve_video_effect_chain(
                    snapshot,
                    &VideoEffectScope::Group { group_id: group.id },
                ) {
                    Ok(Some(chain)) if resolved_chain_executes(&chain) => Some(chain),
                    Ok(_) => None,
                    Err(fault) => {
                        faults.push(*fault);
                        None
                    }
                };
                // Default groups are organizational, preserving each child's
                // blending against the existing backdrop with no frame allocation.
                (chain.is_some()
                    || group.opacity != 1.0
                    || group.blend_mode != VideoBlendMode::Normal
                    || group_transform(group) != Transform2D::default())
                .then_some(ActiveLayerGroup { group, chain })
            })
            .collect()
    }

    pub(super) fn prepare_group_layer<F: std::borrow::Borrow<VideoFrame>>(
        &mut self,
        active: &ActiveLayerGroup<'_>,
        plan: &CompositionPlan,
        frames: &[F],
        size: (u32, u32),
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<
        (
            CompositionLayerPlan,
            Option<Arc<VideoFrame>>,
            Vec<VideoEffectStageFault>,
        ),
        VideoRuntimeError,
    > {
        check_runtime_cancellation(cancellation)?;
        let group = active.group;
        let layers = plan
            .layers
            .iter()
            .filter(|layer| group.layer_ids.contains(&layer.layer_id))
            .cloned()
            .collect::<Vec<_>>();
        let Some(first) = layers.first() else {
            return Err(CpuCompositeError::MissingFrame {
                layer_id: group.layer_ids.first().copied().unwrap_or_default(),
            }
            .into());
        };
        let mut outer = first.clone();
        // A singleton Normal child can directly use its original frame and
        // sampler. This is exact, including translucent inputs, and has the
        // same pixel work and upload count as a plain layer.
        if layers.len() == 1
            && active.chain.is_none()
            && group_transform(group) == Transform2D::default()
            && outer.blend_mode == VideoBlendMode::Normal
        {
            outer.opacity *= group.opacity;
            outer.blend_mode = group.blend_mode.clone();
            return Ok((outer, None, Vec::new()));
        }
        let group_plan = CompositionPlan {
            composition_id: plan.composition_id,
            label: group.label.clone(),
            output_ids: Vec::new(),
            master_opacity: plan.master_opacity,
            blackout: plan.blackout,
            layers,
        };
        let (width, height) = size;
        let group_charge = self.reserve_native_cpu_rgba(width, height, cancellation)?;
        let conversion_charge = self.reserve_native_cpu_conversion_scratch(frames, cancellation)?;
        let mut frame = composite_frames_rgba8(&group_plan, frames, width, height)?;
        drop(conversion_charge);
        self.track_native_cpu_frame(&frame, group_charge)?;
        // The compositor's transparent destination contains premultiplied RGB.
        // Effects and the outer layer sampler consume straight-alpha pixels.
        straighten_group_frame(&mut frame);
        let mut faults = Vec::new();
        if let Some(chain) = &active.chain {
            let (rendered, effect_faults) = self
                .apply_resolved_effect_chain_to_frame_with_cancellation(
                    chain,
                    frame,
                    None,
                    cancellation,
                )?;
            frame = rendered;
            faults.extend(effect_faults);
        }
        check_runtime_cancellation(cancellation)?;
        let transform = group_transform(group);
        if transform != Transform2D::default() {
            let transformed_charge = self.reserve_native_cpu_rgba(width, height, cancellation)?;
            let mut transformed = vec![0u8; width as usize * height as usize * 4];
            blend_transformed_rgba8(BlendTransformedRgba8Params {
                destination: &mut transformed,
                frame: &frame,
                width,
                height,
                opacity: 1.0,
                blend_mode: &VideoBlendMode::Normal,
                transform: &transform,
                color: &VideoColorAdjust::default(),
                fx: &VideoFxAdjust::default(),
            });
            frame.data = transformed;
            self.track_native_cpu_frame(&frame, transformed_charge)?;
            straighten_group_frame(&mut frame);
        }
        frame.layer_id = outer.layer_id;
        outer.label = group.label.clone();
        outer.opacity = group.opacity;
        outer.blend_mode = group.blend_mode.clone();
        outer.transform = Transform2D::default();
        outer.color = VideoColorAdjust::default();
        outer.fx = VideoFxAdjust::default();
        Ok((outer, Some(Arc::new(frame)), faults))
    }

    pub(super) fn blend_group_onto<F: std::borrow::Borrow<VideoFrame>>(
        &mut self,
        output: &mut [u8],
        active: &ActiveLayerGroup<'_>,
        plan: &CompositionPlan,
        frames: &[F],
        size: (u32, u32),
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<Vec<VideoEffectStageFault>, VideoRuntimeError> {
        let (outer, frame, faults) =
            self.prepare_group_layer(active, plan, frames, size, cancellation)?;
        match frame {
            Some(frame) => blend_composition_layer_onto(output, &outer, &[frame], size.0, size.1)?,
            None => blend_composition_layer_onto(output, &outer, frames, size.0, size.1)?,
        }
        Ok(faults)
    }
}

fn straighten_group_frame(frame: &mut VideoFrame) {
    for pixel in frame.data.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha == 0 {
            pixel[..3].fill(0);
        } else if alpha < 255 {
            for channel in &mut pixel[..3] {
                *channel = ((u32::from(*channel) * 255 + alpha / 2) / alpha).min(255) as u8;
            }
        }
    }
}
