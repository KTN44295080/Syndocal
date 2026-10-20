//! Per-render ISF clock and layer-image ownership.
//!
//! Entries are published only after a dependency's Clip/Layer effects finish.
//! Clearing the map for every preparation prevents a removed, reordered or
//! unavailable layer from being sampled through a previous frame's binding.

use super::{VideoFrame, VideoFrameProvider, VideoLayerId, VideoPreviewRenderer};
use std::{collections::HashMap, sync::Arc};

pub(super) type IsfShaderCacheKey = (
    Arc<str>,
    Option<String>,
    Vec<protocol::VideoIsfImageBinding>,
);

pub(super) struct IsfFrameInputs {
    pub(super) date_override: Option<[f32; 4]>,
    pub(super) date: [f32; 4],
    pub(super) layers: HashMap<VideoLayerId, Result<Arc<VideoFrame>, String>>,
    pub(super) sampled_layers: std::collections::HashSet<VideoLayerId>,
}

impl Default for IsfFrameInputs {
    fn default() -> Self {
        Self {
            date_override: None,
            date: [1970.0, 1.0, 1.0, 0.0],
            layers: HashMap::new(),
            sampled_layers: std::collections::HashSet::new(),
        }
    }
}

impl IsfFrameInputs {
    pub(super) fn resolve_layer_images(
        &self,
        bindings: &[protocol::VideoIsfImageBinding],
    ) -> Result<Vec<(String, Arc<VideoFrame>)>, String> {
        let mut frames = Vec::new();
        for binding in bindings {
            if let protocol::VideoIsfImageSource::Layer { layer_id } = binding.source {
                let frame = self.layers.get(&layer_id).ok_or_else(|| {
                    format!("ISF image input layer {layer_id} is unavailable in this render context")
                })?.as_ref().map_err(Clone::clone)?;
                frames.push((binding.name.clone(), Arc::clone(frame)));
            }
        }
        Ok(frames)
    }

    pub(super) fn begin_frame(&mut self) {
        self.date = self
            .date_override
            .unwrap_or_else(super::isf_runtime::isf_date_now);
        self.layers.clear();
        self.sampled_layers.clear();
    }
}

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    /// Freeze the ISF DATE uniform for deterministic tests or offline work.
    /// Passing None resumes UTC wall-clock time, sampled once per render.
    /// The vector contains year, month (1..=12), day and seconds within the day.
    pub fn set_isf_date_override(&mut self, date: Option<[f32; 4]>) {
        self.isf_frame_inputs.date_override = date;
        if let Some(date) = date {
            self.isf_frame_inputs.date = date;
        }
    }

    pub(super) fn publish_isf_layer_frame(
        &mut self,
        layer_id: VideoLayerId,
        position_ms: u64,
        produced_frame: bool,
        fault: Option<&str>,
    ) {
        if !self.isf_frame_inputs.sampled_layers.contains(&layer_id) {
            return;
        }
        let frame = match fault {
            Some(message) => Err(format!(
                "ISF image input layer {layer_id} failed to render: {message}"
            )),
            None if produced_frame => self
                .runtime
                .queues
                .iter()
                .find(|queue| queue.layer_id() == layer_id)
                .and_then(|queue| {
                    queue
                        .frame_at_or_before_shared(position_ms)
                        .or_else(|| queue.nearest_shared(position_ms))
                })
                .cloned()
                .ok_or_else(|| format!("ISF image input layer {layer_id} did not produce a frame")),
            None => Err(format!(
                "ISF image input layer {layer_id} did not produce a frame"
            )),
        };
        self.isf_frame_inputs.layers.insert(layer_id, frame);
    }
}

const MAX_LAYER_DEPENDENCY_DEPTH: usize = protocol::MAX_VIDEO_ISF_IMAGE_DEPENDENCY_DEPTH;

fn append_bindings(
    bindings: &[protocol::VideoIsfImageBinding],
    dependencies: &mut Vec<VideoLayerId>,
) {
    for binding in bindings {
        if let protocol::VideoIsfImageSource::Layer { layer_id } = binding.source {
            if !dependencies.contains(&layer_id) {
                dependencies.push(layer_id);
            }
        }
    }
}

fn append_effect(effect: &protocol::VideoIsfEffectSummary, dependencies: &mut Vec<VideoLayerId>) {
    if effect.enabled {
        append_bindings(&effect.image_bindings, dependencies);
    }
    for stage in &effect.stack {
        if stage.enabled {
            append_bindings(&stage.image_bindings, dependencies);
        }
    }
}

fn append_chain(chain: &protocol::VideoEffectChainSummary, dependencies: &mut Vec<VideoLayerId>) {
    if chain.bypassed {
        return;
    }
    for stage in &chain.stages {
        if stage.enabled {
            let protocol::VideoEffectKind::Isf { effect } = &stage.effect.kind;
            append_bindings(&effect.image_bindings, dependencies);
        }
    }
}

/// Dependencies use the same active clip and legacy Layer projection as the
/// render loop. Dormant clips do not make an unrelated output render fail.
fn layer_dependencies(
    snapshot: &protocol::VideoSnapshot,
    clip_runtime: &protocol::VideoClipRuntimeSnapshot,
    layer: &protocol::VideoLayerSummary,
) -> Vec<VideoLayerId> {
    use protocol::{VideoEffectScope, VideoTransitionEffectOwner};
    let mut dependencies = Vec::new();
    let layer_bypassed = snapshot.effect_chains.iter().any(|chain| {
        chain.scope == VideoEffectScope::Layer { layer_id: layer.id } && chain.bypassed
    });
    if !layer_bypassed {
        if let Some(effect) = &layer.isf_effect {
            append_effect(effect, &mut dependencies);
        }
    }
    let runtime = clip_runtime
        .layers
        .iter()
        .find(|runtime| runtime.layer_id == layer.id);
    for chain in &snapshot.effect_chains {
        let active = match &chain.scope {
            VideoEffectScope::Clip { layer_id, slot_id } if *layer_id == layer.id => runtime
                .is_some_and(|runtime| match &runtime.transition {
                    Some(transition) => {
                        *slot_id == transition.outgoing_slot_id
                            || *slot_id == transition.incoming_slot_id
                    }
                    None => runtime.active_slot_id == Some(*slot_id),
                }),
            VideoEffectScope::Transition {
                owner: VideoTransitionEffectOwner::ClipTake { layer_id },
            } => {
                *layer_id == layer.id && runtime.is_some_and(|runtime| runtime.transition.is_some())
            }
            _ => false,
        };
        if active {
            append_chain(chain, &mut dependencies);
        }
    }
    dependencies
}

struct LayerVisit<'a> {
    snapshot: &'a protocol::VideoSnapshot,
    clip_runtime: &'a protocol::VideoClipRuntimeSnapshot,
    visiting: Vec<VideoLayerId>,
    depths: std::collections::HashMap<VideoLayerId, usize>,
    order: Vec<VideoLayerId>,
    sampled: std::collections::HashSet<VideoLayerId>,
}

impl LayerVisit<'_> {
    fn visit(&mut self, id: VideoLayerId) -> Result<usize, String> {
        if let Some(depth) = self.depths.get(&id) {
            return Ok(*depth);
        }
        if self.visiting.contains(&id) {
            return Err(format!("ISF image input cycle detected at layer {id}"));
        }
        if self.visiting.len() > MAX_LAYER_DEPENDENCY_DEPTH {
            return Err(format!(
                "ISF image input dependency depth exceeds {MAX_LAYER_DEPENDENCY_DEPTH} links"
            ));
        }
        let layer = self
            .snapshot
            .layers
            .iter()
            .find(|layer| layer.id == id)
            .ok_or_else(|| format!("ISF image input references missing layer {id}"))?;
        self.visiting.push(id);
        let mut depth = 0;
        for dependency in layer_dependencies(self.snapshot, self.clip_runtime, layer) {
            self.sampled.insert(dependency);
            depth = depth.max(self.visit(dependency)? + 1);
            if depth > MAX_LAYER_DEPENDENCY_DEPTH {
                return Err(format!(
                    "ISF image input dependency depth exceeds {MAX_LAYER_DEPENDENCY_DEPTH} links"
                ));
            }
        }
        self.visiting.pop();
        self.depths.insert(id, depth);
        self.order.push(id);
        Ok(depth)
    }
}

/// Scope effects consume layer outputs but do not contribute a layer-to-layer
/// edge: their results are never exposed as another layer's output.
pub(super) fn image_render_order(
    snapshot: &protocol::VideoSnapshot,
    clip_runtime: &protocol::VideoClipRuntimeSnapshot,
    requested: &[VideoLayerId],
    output_id: Option<protocol::VideoOutputId>,
    scope_composition: Option<protocol::CompositionId>,
    transition_runtime: Option<&protocol::VideoLayerTransitionRuntimeSnapshot>,
) -> Result<(Vec<VideoLayerId>, std::collections::HashSet<VideoLayerId>), String> {
    use protocol::{VideoEffectScope, VideoTransitionEffectOwner};
    let mut roots = snapshot
        .layers
        .iter()
        .filter(|layer| requested.contains(&layer.id))
        .map(|layer| layer.id)
        .collect::<Vec<_>>();
    let is_layer = |binding: &protocol::VideoIsfImageBinding| {
        matches!(binding.source, protocol::VideoIsfImageSource::Layer { .. })
    };
    let has_layer_bindings = snapshot.layers.iter().any(|layer| {
        layer.isf_effect.as_ref().is_some_and(|effect| {
            effect.image_bindings.iter().any(is_layer)
                || effect
                    .stack
                    .iter()
                    .any(|stage| stage.image_bindings.iter().any(is_layer))
        })
    }) || snapshot.effect_chains.iter().any(|chain| {
        chain.stages.iter().any(|stage| {
            let protocol::VideoEffectKind::Isf { effect } = &stage.effect.kind;
            effect.image_bindings.iter().any(is_layer)
        })
    });
    if !has_layer_bindings {
        return Ok((roots, std::collections::HashSet::new()));
    }
    let mut scoped_dependencies = Vec::new();
    if let Some(scope_composition) = scope_composition {
        for chain in &snapshot.effect_chains {
            let active = match &chain.scope {
                VideoEffectScope::Output { output_id: owner } => output_id == Some(*owner),
                VideoEffectScope::Composition { composition_id } => {
                    *composition_id == scope_composition
                }
                VideoEffectScope::Group { group_id } => snapshot.layer_groups.iter().any(|group| {
                    group.id == *group_id
                        && group.enabled
                        && !group.bypassed
                        && group.composition_id == scope_composition
                        && group.layer_ids.iter().any(|id| requested.contains(id))
                }),
                VideoEffectScope::Transition {
                    owner: VideoTransitionEffectOwner::LayerBus { bus_id },
                } => {
                    transition_runtime.is_some_and(|runtime| {
                        runtime.buses.iter().any(|active| active.bus_id == *bus_id)
                    }) && snapshot.transition_buses.iter().any(|bus| {
                        bus.id == *bus_id
                            && bus.composition_id == scope_composition
                            && bus.members.iter().any(|target| {
                                super::video_layer_transition_target_ids(snapshot, target)
                                    .is_ok_and(|ids| ids.iter().any(|id| requested.contains(id)))
                            })
                    })
                }
                _ => false,
            };
            if active {
                append_chain(chain, &mut scoped_dependencies);
            }
        }
        for id in &scoped_dependencies {
            if !roots.contains(id) {
                roots.push(*id);
            }
        }
    }
    let mut visit = LayerVisit {
        snapshot,
        clip_runtime,
        visiting: Vec::new(),
        depths: std::collections::HashMap::new(),
        order: Vec::new(),
        sampled: scoped_dependencies.into_iter().collect(),
    };
    for root in roots {
        visit.visit(root)?;
    }
    Ok((visit.order, visit.sampled))
}
