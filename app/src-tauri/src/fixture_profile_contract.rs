//! Pure fixture profile, geometry, and emitter consistency contracts.
//! Callers own project transactions, patch allocation, and profile file I/O.
use protocol::{
    AttributeControl, EngineSnapshot, FixtureProfileSummary, GeometrySummary, PatchedFixtureSummary,
};
use std::collections::{HashMap, HashSet};

pub(super) fn validate_project_fixture_geometries(fixtures: &[PatchedFixtureSummary]) -> Result<(), String> {
    for fixture in fixtures {
        validate_geometry_collection(
            &format!("fixture {} '{}'", fixture.id, fixture.label),
            &fixture.geometries,
            &fixture.controls,
        )?;
    }
    Ok(())
}

fn validate_geometry_collection(
    owner_label: &str,
    geometries: &[GeometrySummary],
    controls: &[AttributeControl],
) -> Result<(), String> {
    let mut geometry_names = HashSet::new();
    let mut parents_by_name = HashMap::new();

    for geometry in geometries {
        let name = geometry.name.trim();
        if name.is_empty() {
            return Err(format!(
                "Project {owner_label} has a geometry node with an empty name"
            ));
        }
        if !geometry_names.insert(name.to_string()) {
            return Err(format!(
                "Project {owner_label} contains duplicate geometry node '{name}'"
            ));
        }
        if geometry.kind.trim().is_empty() {
            return Err(format!(
                "Project {owner_label} geometry '{name}' has an empty kind"
            ));
        }
        if geometry.matrix.iter().any(|value| !value.is_finite()) {
            return Err(format!(
                "Project {owner_label} geometry '{name}' has a non-finite transform matrix"
            ));
        }
        let parent = geometry
            .parent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if let Some(parent) = parent {
            if parent == name {
                return Err(format!(
                    "Project {owner_label} geometry '{name}' cannot parent itself"
                ));
            }
            parents_by_name.insert(name.to_string(), parent.to_string());
        }
        if let Some(dimensions) = geometry.model_dimensions {
            if !dimensions.x.is_finite()
                || !dimensions.y.is_finite()
                || !dimensions.z.is_finite()
                || dimensions.x < 0.0
                || dimensions.y < 0.0
                || dimensions.z < 0.0
            {
                return Err(format!(
                    "Project {owner_label} geometry '{name}' has invalid model dimensions"
                ));
            }
        }
        for (label, value) in [
            ("beam angle", geometry.beam_angle_deg),
            ("field angle", geometry.field_angle_deg),
            ("beam radius", geometry.beam_radius),
        ] {
            if let Some(value) = value {
                if !value.is_finite() || value < 0.0 {
                    return Err(format!(
                        "Project {owner_label} geometry '{name}' has invalid {label}"
                    ));
                }
            }
        }
    }

    for (name, parent) in &parents_by_name {
        if !geometry_names.contains(parent) {
            return Err(format!(
                "Project {owner_label} geometry '{name}' references missing parent '{parent}'"
            ));
        }
    }

    for geometry in geometries {
        let mut seen = HashSet::new();
        let mut current = geometry.name.trim().to_string();
        while let Some(parent) = parents_by_name.get(&current) {
            if !seen.insert(current.clone()) {
                return Err(format!(
                    "Project {owner_label} geometry '{}' contains a parent cycle",
                    geometry.name
                ));
            }
            current = parent.clone();
        }
    }

    for control in controls {
        let Some(geometry_name) = control
            .geometry
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        else {
            continue;
        };
        if !geometry_names.contains(geometry_name) {
            return Err(format!(
                "Project {owner_label} control '{}' references missing geometry '{geometry_name}'",
                control.attribute
            ));
        }
    }

    Ok(())
}

pub(super) fn validate_project_custom_profiles(profiles: &[FixtureProfileSummary]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for profile in profiles {
        let source_path = profile.source_path.trim();
        if source_path.is_empty() {
            return Err("Project custom profile has an empty source path".to_string());
        }
        if !seen.insert(source_path.to_string()) {
            return Err(format!(
                "Project contains duplicate custom profile source path {source_path}"
            ));
        }
        if profile.manufacturer.trim().is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has an empty manufacturer"
            ));
        }
        if profile.name.trim().is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has an empty name"
            ));
        }
        if profile.dmx_modes.is_empty() {
            return Err(format!(
                "Project custom profile {source_path} has no DMX modes"
            ));
        }
        for mode in &profile.dmx_modes {
            if mode.name.trim().is_empty() {
                return Err(format!(
                    "Project custom profile {source_path} has an empty DMX mode name"
                ));
            }
            fixture_controls_footprint(&mode.controls).ok_or_else(|| {
                format!(
                    "Project custom profile {source_path} mode '{}' has no DMX channel offsets",
                    mode.name
                )
            })?;
            validate_emitter_calibrations(
                &mode.controls,
                &format!("Project custom profile {source_path} mode '{}'", mode.name),
            )?;
            validate_geometry_collection(
                &format!("custom profile {source_path} mode '{}'", mode.name),
                &profile.geometries,
                &mode.controls,
            )?;
        }
    }
    Ok(())
}

pub(super) fn validate_emitter_calibrations(
    controls: &[AttributeControl],
    context: &str,
) -> Result<(), String> {
    for control in controls {
        for function in &control.functions {
            let Some(emitter) = function.emitter.as_ref() else {
                continue;
            };
            if emitter.name.trim().is_empty() {
                return Err(format!(
                    "{context} control '{}' has an emitter with an empty name",
                    control.attribute
                ));
            }
            if let Some(color) = emitter.color.as_ref() {
                if !color.x.is_finite()
                    || !color.y.is_finite()
                    || !color.luminance.is_finite()
                    || color.x < 0.0
                    || color.y <= 0.0
                    || color.luminance <= 0.0
                    || color.x + color.y > 1.000_1
                {
                    return Err(format!(
                        "{context} control '{}' emitter '{}' has invalid CIE xyY calibration",
                        control.attribute, emitter.name
                    ));
                }
            }
            if emitter
                .dominant_wavelength_nm
                .is_some_and(|wavelength| !wavelength.is_finite() || wavelength <= 0.0)
            {
                return Err(format!(
                    "{context} control '{}' emitter '{}' has invalid dominant wavelength",
                    control.attribute, emitter.name
                ));
            }
        }
    }
    Ok(())
}

pub(super) fn validate_project_custom_profile_refs(
    snapshot: &EngineSnapshot,
    profiles: &[FixtureProfileSummary],
) -> Result<(), String> {
    let custom_profile_paths = profiles
        .iter()
        .map(|profile| profile.source_path.as_str())
        .collect::<HashSet<_>>();
    for fixture in &snapshot.fixtures {
        let source_path = fixture.profile_source_path.trim();
        if source_path.starts_with("memory://custom/")
            && !custom_profile_paths.contains(source_path)
        {
            return Err(format!(
                "Project fixture {} '{}' references missing custom profile {}",
                fixture.id, fixture.label, source_path
            ));
        }
    }
    Ok(())
}

pub(super) fn fixture_controls_footprint(controls: &[AttributeControl]) -> Option<u16> {
    controls
        .iter()
        .flat_map(|control| control.offsets.iter().copied())
        .max()
}
