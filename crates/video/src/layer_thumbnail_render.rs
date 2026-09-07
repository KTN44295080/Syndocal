use super::{
    check_render_cancellation, sanitize_layer_state, CompositionLayerPlan, CompositionPlan,
    VideoBlendMode, VideoClipRuntimeSnapshot, VideoFrame, VideoFrameProvider, VideoLayerId,
    VideoPreviewError, VideoPreviewRenderer, VideoRenderCancellation, VideoSnapshot,
};

impl<P: VideoFrameProvider> VideoPreviewRenderer<P> {
    /// Cooperatively stops thumbnail decode/effect stages; synchronous operations
    /// are checked at their boundaries and remain owned until they return.
    pub fn render_layer_preview_cancellable(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
        cancellation: &dyn VideoRenderCancellation,
    ) -> Result<VideoFrame, VideoPreviewError> {
        self.render_layer_thumbnail(snapshot, layer_id, width, height, Some(cancellation))
    }

    pub(super) fn render_layer_thumbnail(
        &mut self,
        snapshot: &VideoSnapshot,
        layer_id: VideoLayerId,
        width: u32,
        height: u32,
        cancellation: Option<&dyn VideoRenderCancellation>,
    ) -> Result<VideoFrame, VideoPreviewError> {
        check_render_cancellation(cancellation)?;
        if width == 0 || height == 0 {
            return Err(VideoPreviewError::InvalidSize);
        }
        let layer = snapshot
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .ok_or(VideoPreviewError::MissingLayer { layer_id })?;
        let state = sanitize_layer_state(layer.state.clone());
        let position_ms = if state.position_ms > 0 {
            state.position_ms
        } else {
            state.loop_start_ms
        };
        let plan = CompositionPlan {
            composition_id: 0,
            label: format!("{} Thumbnail", layer.label),
            output_ids: Vec::new(),
            master_opacity: 1.0,
            blackout: false,
            layers: vec![CompositionLayerPlan {
                layer_id,
                label: layer.label.clone(),
                source: layer.source.clone(),
                blend_mode: VideoBlendMode::Normal,
                opacity: 1.0,
                position_ms,
                transform: state.transform,
                color: state.color,
                fx: state.fx,
            }],
        };
        let faults = self.prepare_frames_with_effects_with_cancellation(
            snapshot,
            &VideoClipRuntimeSnapshot::default(),
            0,
            &[layer_id],
            width,
            height,
            cancellation,
        )?;
        self.record_effect_faults(faults, snapshot);
        check_render_cancellation(cancellation)?;
        let frame = self
            .runtime
            .compose_plan(&plan, width, height)
            .map_err(VideoPreviewError::Runtime)?;
        check_render_cancellation(cancellation)?;
        Ok(frame)
    }
}

#[cfg(test)]
#[path = "layer_thumbnail_render_tests.rs"]
mod tests;
