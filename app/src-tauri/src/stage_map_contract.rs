//! Pure Stage geometry, color, bounds, and preset canonicalization contracts.
//! Callers own allocation, transaction authority, and persistence file formats.
use protocol::{StageMapConfig, StageMapPresetSummary, StageObjectSummary};
use std::collections::HashSet;

pub(super) fn canonical_stage_project_objects(
    objects: Vec<StageObjectSummary>,
) -> Result<Vec<StageObjectSummary>, String> {
    let mut objects = objects
        .into_iter()
        .map(normalize_stage_object)
        .collect::<Result<Vec<_>, _>>()?;
    objects.sort_by_key(|object| object.id);
    validate_project_stage_objects(&objects)?;
    Ok(objects)
}

pub(super) fn prepare_stage_map_preset_import(
    mut preset: StageMapPresetSummary,
) -> Result<StageMapPresetSummary, String> {
    preset.label = normalize_stage_map_preset_label(preset.label)?;
    validate_stage_map_config(&preset.config)?;
    if let Some(objects) = preset.stage_objects.take() {
        preset.stage_objects = Some(canonical_stage_project_objects(objects)?);
    }
    validate_project_stage_map_presets(std::slice::from_ref(&preset))?;
    Ok(preset)
}

pub(super) fn validate_stage_map_config(config: &StageMapConfig) -> Result<(), String> {
    if !config.min_x.is_finite()
        || !config.max_x.is_finite()
        || !config.min_z.is_finite()
        || !config.max_z.is_finite()
    {
        return Err("Stage map bounds must be finite".to_string());
    }
    if config.min_x >= config.max_x || config.min_z >= config.max_z {
        return Err("Stage map min bounds must be lower than max bounds".to_string());
    }
    if config.max_x - config.min_x < 0.1 || config.max_z - config.min_z < 0.1 {
        return Err("Stage map bounds must span at least 0.1m".to_string());
    }
    Ok(())
}

pub(super) fn normalize_stage_object(mut object: StageObjectSummary) -> Result<StageObjectSummary, String> {
    if object.id == 0 {
        return Err("Stage object id must be greater than zero".to_string());
    }
    object.label = object.label.trim().to_string();
    if object.label.is_empty() {
        return Err("Stage object label is required".to_string());
    }
    if [
        object.x,
        object.z,
        object.width,
        object.depth,
        object.rotation_deg,
    ]
    .iter()
    .any(|value| !value.is_finite())
    {
        return Err("Stage object values must be finite".to_string());
    }
    if object.width <= 0.0 || object.depth <= 0.0 {
        return Err("Stage object size must be greater than zero".to_string());
    }
    object.x = object.x.clamp(-1_000.0, 1_000.0);
    object.z = object.z.clamp(-1_000.0, 1_000.0);
    object.width = object.width.clamp(0.05, 1_000.0);
    object.depth = object.depth.clamp(0.05, 1_000.0);
    object.rotation_deg = object.rotation_deg.clamp(-360.0, 360.0);
    object.color = match object.color {
        Some(color) if color.trim().is_empty() => None,
        Some(color) => Some(normalize_stage_object_color(color)?),
        None => None,
    };
    Ok(object)
}

fn normalize_stage_object_color(color: String) -> Result<String, String> {
    let trimmed = color.trim();
    let hex = trimmed
        .strip_prefix('#')
        .ok_or_else(|| "Stage object color must be a #rrggbb value".to_string())?;
    if hex.len() == 6 && hex.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(format!("#{hex}"))
    } else {
        Err("Stage object color must be a #rrggbb value".to_string())
    }
}

pub(super) fn validate_project_stage_objects(objects: &[StageObjectSummary]) -> Result<(), String> {
    let mut ids = HashSet::new();
    for object in objects {
        let normalized = normalize_stage_object(object.clone())?;
        if !ids.insert(normalized.id) {
            return Err(format!(
                "Project contains duplicate stage object id {}",
                normalized.id
            ));
        }
    }
    Ok(())
}

pub(super) fn normalize_stage_map_preset_label(label: String) -> Result<String, String> {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        return Err("Stage map preset label is required".to_string());
    }
    Ok(trimmed.to_string())
}

pub(super) fn validate_project_stage_map_presets(presets: &[StageMapPresetSummary]) -> Result<(), String> {
    let mut labels = HashSet::new();
    for preset in presets {
        let label = normalize_stage_map_preset_label(preset.label.clone())?;
        if !labels.insert(label.clone()) {
            return Err(format!(
                "Project contains duplicate stage map preset label {label}"
            ));
        }
        validate_stage_map_config(&preset.config)?;
        if let Some(stage_objects) = &preset.stage_objects {
            validate_project_stage_objects(stage_objects)?;
        }
    }
    Ok(())
}
