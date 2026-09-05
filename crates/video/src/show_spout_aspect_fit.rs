//! Runtime-only source aspect fit for the two fixed show Spout canvases.
use protocol::{VideoOutputKind, VideoOutputSummary};

use crate::{CompositionPlan, VideoOutputRenderError};

pub(super) fn apply(
    output: &VideoOutputSummary,
    composition: &mut CompositionPlan,
) -> Result<(), VideoOutputRenderError> {
    let show_canvas = output.kind == VideoOutputKind::SpoutSender
        && matches!(
            (output.endpoint_name.as_deref(), output.width, output.height),
            (Some("Syndocal Foreground"), 3840, 2160)
                | (Some("Syndocal Background"), 1920, 1080)
        );
    if !show_canvas || composition.blackout {
        return Ok(());
    }
    for layer in &mut composition.layers {
        let dimensions = layer.source.metadata.as_ref().and_then(|metadata| {
            Some((metadata.width?, metadata.height?))
        });
        let Some((width, height)) = dimensions.filter(|(width, height)| *width > 0 && *height > 0) else {
            return Err(VideoOutputRenderError::ShowSpoutSourceDimensionsUnavailable {
                output_id: output.id,
                layer_id: layer.layer_id,
            });
        };
        let source_aspect = width as f64 / height as f64;
        let canvas_aspect = output.width as f64 / output.height as f64;
        // Fit in source-local axes before the existing user scale/rotation/
        // translation. Crop remains the user's normalized source rectangle.
        if source_aspect > canvas_aspect {
            layer.transform.scale_y *= (canvas_aspect / source_aspect) as f32;
        } else {
            layer.transform.scale_x *= (source_aspect / canvas_aspect) as f32;
        }
    }
    Ok(())
}

#[cfg(test)]
#[path = "show_spout_aspect_fit_tests.rs"]
mod tests;
