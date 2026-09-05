//! Pure output-mapping value, field, and preset-label contracts.
//! Callers own project references, file formats, and output mutations.
use protocol::{canonical_video_output_mapping_field, VideoOutputMapping};

pub(super) fn validate_video_output_mapping(mapping: &VideoOutputMapping) -> Result<(), String> {
    let values = [
        mapping.stage_x,
        mapping.stage_y,
        mapping.stage_z,
        mapping.offset_x,
        mapping.offset_y,
        mapping.scale_x,
        mapping.scale_y,
        mapping.rotation_deg,
        mapping.aspect_ratio,
        mapping.lens_distortion,
        mapping.edge_blend_left,
        mapping.edge_blend_right,
        mapping.edge_blend_top,
        mapping.edge_blend_bottom,
        mapping.edge_blend_gamma,
        mapping.black_level,
        mapping.mask_softness,
        mapping.keystone_x,
        mapping.keystone_y,
        mapping.corner_top_left_x,
        mapping.corner_top_left_y,
        mapping.corner_top_right_x,
        mapping.corner_top_right_y,
        mapping.corner_bottom_right_x,
        mapping.corner_bottom_right_y,
        mapping.corner_bottom_left_x,
        mapping.corner_bottom_left_y,
    ];
    if values.iter().any(|value| !value.is_finite()) {
        return Err("Video output mapping values must be finite".to_string());
    }
    if mapping.scale_x <= 0.0 || mapping.scale_y <= 0.0 {
        return Err("Video output mapping scale must be greater than 0".to_string());
    }
    if mapping.aspect_ratio <= 0.0 {
        return Err("Video output mapping aspect ratio must be greater than 0".to_string());
    }
    if !(0.1..=8.0).contains(&mapping.edge_blend_gamma) {
        return Err("Video output edge blend gamma must be between 0.1 and 8".to_string());
    }
    if [
        mapping.edge_blend_left,
        mapping.edge_blend_right,
        mapping.edge_blend_top,
        mapping.edge_blend_bottom,
        mapping.black_level,
    ]
    .iter()
    .any(|value| !(0.0..=1.0).contains(value))
    {
        return Err(
            "Video output edge blend and black level values must be between 0 and 1".to_string(),
        );
    }
    if !(0.0..=0.5).contains(&mapping.mask_softness) {
        return Err("Video output mask softness must be between 0 and 0.5".to_string());
    }
    if usize::from(mapping.mask_point_count) > mapping.mask_points.len() {
        return Err("Video output mask supports at most 8 points".to_string());
    }
    if mapping.mask_points[..usize::from(mapping.mask_point_count)]
        .iter()
        .any(|point| {
            !point.x.is_finite()
                || !point.y.is_finite()
                || !(0.0..=1.0).contains(&point.x)
                || !(0.0..=1.0).contains(&point.y)
        })
    {
        return Err("Video output mask points must be finite values between 0 and 1".to_string());
    }
    let bitmap_disabled = mapping.bitmap_mask_width == 0 && mapping.bitmap_mask_height == 0;
    let bitmap_valid = mapping.bitmap_mask_width > 0
        && mapping.bitmap_mask_width <= protocol::VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION
        && mapping.bitmap_mask_height > 0
        && mapping.bitmap_mask_height <= protocol::VIDEO_OUTPUT_BITMAP_MASK_MAX_DIMENSION
        && usize::from(mapping.bitmap_mask_width)
            .saturating_mul(usize::from(mapping.bitmap_mask_height))
            <= protocol::VIDEO_OUTPUT_BITMAP_MASK_WORD_CAPACITY * 8;
    if !bitmap_disabled && !bitmap_valid {
        return Err("Video output bitmap mask dimensions are invalid".to_string());
    }
    Ok(())
}

pub(super) fn normalize_video_output_mapping_field(field: String) -> Result<String, String> {
    let trimmed = field.trim();
    if trimmed.is_empty() {
        return Err("Video output mapping field is required".to_string());
    }
    canonical_video_output_mapping_field(trimmed)
        .map(str::to_string)
        .ok_or_else(|| format!("Video output mapping field '{trimmed}' is not supported"))
}

pub(super) fn normalize_video_output_mapping_preset_label(label: String) -> Result<String, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Video output mapping preset label is required".to_string());
    }
    Ok(trimmed.to_string())
}
