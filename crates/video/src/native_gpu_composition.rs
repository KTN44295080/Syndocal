//! P lane's texture-source composition plan (INTERFACE-01 section 6.1).
//!
//! This policy owns no device, submission, scope or surface. A broker adapter
//! resolves selected identities and encodes through the port below. Layer
//! opacity already includes master opacity in the canonical composition plan;
//! applying master opacity here again would change its pixels.

use crate::native_gpu_contract::{
    GpuContextId, GpuPhase, HistoryEntryId, LayerResultIdentity, NativeFrameId, NativeGpuDecline,
    NativeGpuFault, NativeGpuFaultKind,
};
use crate::{
    axis_aligned_source_lookup, gpu_composition_layer_plan, CompositionLayerPlan, VideoPixelFormat,
};

pub(crate) const TEXTURE_COMPOSITION_SHADER: &str =
    include_str!("gpu_display_composition_texture.wgsl");

/// Planning metadata, never evidence of GPU completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SelectedTextureKind {
    Validated,
    InThisTransaction,
}

pub(crate) struct NativeCompositionLayer<'a> {
    pub plan: &'a CompositionLayerPlan,
    pub selected_entry: HistoryEntryId,
    pub identity: LayerResultIdentity,
    pub kind: SelectedTextureKind,
}

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CompositionPlanFailure {
    Declined(NativeGpuDecline),
    Fault(NativeGpuFault),
}

#[derive(Debug)]
pub(crate) struct CompositionLayerEncoding {
    pub selected_entry: HistoryEntryId,
    pub identity: LayerResultIdentity,
    pub kind: SelectedTextureKind,
    /// Canonical CPU sampling indices; no full-frame pixel upload.
    pub lookup: Vec<u32>,
    pub params: [u8; 32],
}

#[derive(Debug)]
pub(crate) struct NativeCompositionPlan {
    pub context: GpuContextId,
    pub frame: NativeFrameId,
    pub extent: (u32, u32),
    pub clear_color: [u8; 4],
    pub layers_bottom_first: Vec<CompositionLayerEncoding>,
    pub allocations: CompositionAllocationPlan,
}

/// Checked minimum capacities. G must also account for actual grown capacities,
/// old/new resources and outstanding uses in its one shared allocation ledger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct CompositionAllocationPlan {
    pub premapping_bytes: u64,
    pub readback_bytes: u64,
    pub lookup_bytes: u64,
    pub uniform_bytes: u64,
}

impl NativeCompositionPlan {
    pub(crate) fn preflight(
        context: GpuContextId,
        frame: NativeFrameId,
        extent: (u32, u32),
        clear_color: [u8; 4],
        layers_bottom_first: &[NativeCompositionLayer<'_>],
        max_texture_dimension_2d: u32,
        max_storage_buffer_bytes: u64,
    ) -> Result<Self, CompositionPlanFailure> {
        let decline = |reason| CompositionPlanFailure::Declined(reason);
        let mismatch = || {
            CompositionPlanFailure::Fault(NativeGpuFault {
                context,
                frame: Some(frame),
                phase: GpuPhase::CompositionSnapshot,
                kind: NativeGpuFaultKind::IdentityMismatch,
                message: "composition selected entry/context/frame does not match its texture"
                    .into(),
            })
        };
        let (width, height) = extent;
        let output_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            // WGSL's packed-buffer indices are u32, independent of host size.
            .filter(|pixels| *pixels <= u64::from(u32::MAX))
            .and_then(|pixels| pixels.checked_mul(4))
            .filter(|bytes| *bytes <= max_storage_buffer_bytes)
            .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
        if width == 0
            || height == 0
            || width > max_texture_dimension_2d
            || height > max_texture_dimension_2d
        {
            return Err(decline(NativeGpuDecline::UnsupportedExtent));
        }
        if layers_bottom_first.is_empty() {
            return Err(decline(NativeGpuDecline::UnsupportedComposition));
        }
        let mut layers = Vec::with_capacity(layers_bottom_first.len());
        let mut lookup_bytes = 0u64;
        let mut uniform_bytes = 0u64;
        for layer in layers_bottom_first {
            let identity = layer.identity;
            if identity.context != context
                || identity.entry != layer.selected_entry
                || identity.layer_id != layer.plan.layer_id
                || identity.frame.output_id != frame.output_id
                || identity.frame.project_epoch != frame.project_epoch
                || identity.frame.geometry_generation != frame.geometry_generation
                || identity.entry.input.project_render_epoch != frame.project_epoch
                || (layer.kind == SelectedTextureKind::InThisTransaction && identity.frame != frame)
            {
                return Err(mismatch());
            }
            if identity.format != VideoPixelFormat::Rgba8 {
                return Err(decline(NativeGpuDecline::UnsupportedFormat));
            }
            if (identity.width, identity.height) != extent {
                return Err(decline(NativeGpuDecline::UnsupportedExtent));
            }
            if !gpu_composition_layer_plan(layer.plan) {
                return Err(decline(NativeGpuDecline::UnsupportedComposition));
            }
            let lookup = axis_aligned_source_lookup(&layer.plan.transform, extent, extent);
            let bytes = u64::try_from(lookup.len())
                .ok()
                .and_then(|len| len.checked_mul(4))
                .filter(|bytes| *bytes <= max_storage_buffer_bytes)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            lookup_bytes = lookup_bytes
                .checked_add(bytes)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            uniform_bytes = uniform_bytes
                .checked_add(32)
                .ok_or_else(|| decline(NativeGpuDecline::UnsupportedExtent))?;
            let mut params = [0u8; 32];
            for (index, word) in [
                width,
                height,
                identity.width,
                identity.height,
                layer.plan.opacity.clamp(0.0, 1.0).to_bits(),
                crate::gpu_compositor::blend_mode_index(&layer.plan.blend_mode),
                u32::from_le_bytes(clear_color),
                0,
            ]
            .into_iter()
            .enumerate()
            {
                params[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
            }
            layers.push(CompositionLayerEncoding {
                selected_entry: layer.selected_entry,
                identity,
                kind: layer.kind,
                lookup,
                params,
            });
        }
        Ok(Self {
            context,
            frame,
            extent,
            clear_color,
            layers_bottom_first: layers,
            allocations: CompositionAllocationPlan {
                premapping_bytes: output_bytes,
                readback_bytes: output_bytes,
                lookup_bytes,
                uniform_bytes,
            },
        })
    }

    /// The adapter reserves the whole plan, resolves identities and retains
    /// selected read uses before encoding. No wait/map/cache/acquire here.
    pub(crate) fn encode<P: CompositionEncodingPort>(
        &self,
        port: &mut P,
    ) -> Result<P::Encoded, P::Error> {
        port.begin(self)?;
        for layer in &self.layers_bottom_first {
            port.composite_layer(layer)?;
        }
        port.copy_premapping_to_snapshot(self.allocations.premapping_bytes)?;
        port.finish()
    }
}

pub(crate) trait CompositionEncodingPort {
    type Encoded;
    type Error;
    fn begin(&mut self, plan: &NativeCompositionPlan) -> Result<(), Self::Error>;
    fn composite_layer(&mut self, layer: &CompositionLayerEncoding) -> Result<(), Self::Error>;
    fn copy_premapping_to_snapshot(&mut self, bytes: u64) -> Result<(), Self::Error>;
    fn finish(&mut self) -> Result<Self::Encoded, Self::Error>;
}

#[cfg(test)]
#[path = "native_gpu_composition_tests.rs"]
mod tests;
