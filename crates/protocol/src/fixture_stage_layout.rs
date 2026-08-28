use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use crate::{FixtureId, PatchedFixtureSummary, ProjectReferenceIntegrityError};

/// Maximum number of authored cells or logical segments in a fixture stage
/// layout. The limit is deliberately small and deterministic because the
/// layout is persisted in every patched fixture summary.
pub const FIXTURE_STAGE_LAYOUT_MAX_ENTRIES: usize = 64;

/// Absolute coordinate bound for fixture stage-layout cell offsets and the
/// positive cell extents. This matches the existing stage-object world cap
/// used by the Tauri stage-map path (`-1_000.0..=1_000.0`).
pub const FIXTURE_STAGE_LAYOUT_WORLD_CAP: f32 = 1_000.0;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FixtureStageColorRole {
    Red,
    Green,
    Blue,
    Amber,
    White,
    Uv,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FixtureStageColorBinding {
    pub control_index: u16,
    pub role: FixtureStageColorRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FixtureStageLayoutCell {
    pub beam_index: u16,
    pub offset_x: f32,
    pub offset_z: f32,
    pub logical_segment_index: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FixtureStageLogicalSegment {
    pub logical_index: u16,
    pub color_controls: Vec<FixtureStageColorBinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct FixtureStageLayout {
    pub version: u8,
    pub cell_width: f32,
    pub cell_depth: f32,
    pub cells: Vec<FixtureStageLayoutCell>,
    pub logical_segments: Vec<FixtureStageLogicalSegment>,
    #[serde(default)]
    pub global_dimmer_control_index: Option<u16>,
    #[serde(default)]
    pub global_strobe_control_index: Option<u16>,
}

fn fixture_stage_layout_mismatch(
    fixture_id: FixtureId,
    detail: impl Into<String>,
) -> ProjectReferenceIntegrityError {
    ProjectReferenceIntegrityError::ReferenceMismatch {
        owner: format!("fixture {fixture_id} stage layout"),
        detail: detail.into(),
    }
}

fn validate_cell_coordinate(
    fixture_id: FixtureId,
    cell_index: usize,
    field: &str,
    value: f32,
) -> Result<(), ProjectReferenceIntegrityError> {
    if !value.is_finite() || value.abs() > FIXTURE_STAGE_LAYOUT_WORLD_CAP {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "cells[{cell_index}].{field} must be finite within ±{FIXTURE_STAGE_LAYOUT_WORLD_CAP}"
            ),
        ));
    }
    Ok(())
}

/// Check the complete rotated footprint of one authored cell. Stage-map and
/// fixture yaw are expressed in degrees, so all four corners are rotated
/// around the cell centre before applying the world-cap check. Checking only
/// the centre would allow a large cell to cross the safety boundary.
fn validate_rotated_cell_footprint(
    fixture: &PatchedFixtureSummary,
    cell_index: usize,
    cell_width: f32,
    cell_depth: f32,
    offset_x: f32,
    offset_z: f32,
) -> Result<(), ProjectReferenceIntegrityError> {
    if !fixture.rotation.yaw.is_finite() {
        return Err(fixture_stage_layout_mismatch(
            fixture.id,
            "fixture yaw must be finite when validating a stage-layout footprint",
        ));
    }
    let (sin_yaw, cos_yaw) = fixture.rotation.yaw.to_radians().sin_cos();
    let half_width = cell_width * 0.5;
    let half_depth = cell_depth * 0.5;
    if !fixture.position.x.is_finite() || !fixture.position.z.is_finite() {
        return Err(fixture_stage_layout_mismatch(
            fixture.id,
            "fixture position must be finite when validating a stage-layout footprint",
        ));
    }
    // Cell offsets are fixture-local. Rotate the centre as well as the cell
    // rectangle; rotating only the corner extents silently translates a
    // multi-cell bar/strobe to the wrong world position at non-zero yaw.
    let origin_x = fixture.position.x + offset_x * cos_yaw - offset_z * sin_yaw;
    let origin_z = fixture.position.z + offset_x * sin_yaw + offset_z * cos_yaw;
    if !origin_x.is_finite() || !origin_z.is_finite() {
        return Err(fixture_stage_layout_mismatch(
            fixture.id,
            format!(
                "cells[{cell_index}] world origin must be finite within ±{FIXTURE_STAGE_LAYOUT_WORLD_CAP}"
            ),
        ));
    }
    for (sign_x, sign_z) in [(-1.0_f32, -1.0_f32), (-1.0, 1.0), (1.0, -1.0), (1.0, 1.0)] {
        let local_x = sign_x * half_width;
        let local_z = sign_z * half_depth;
        let world_x = origin_x + local_x * cos_yaw - local_z * sin_yaw;
        let world_z = origin_z + local_x * sin_yaw + local_z * cos_yaw;
        if !world_x.is_finite()
            || !world_z.is_finite()
            || world_x.abs() > FIXTURE_STAGE_LAYOUT_WORLD_CAP
            || world_z.abs() > FIXTURE_STAGE_LAYOUT_WORLD_CAP
        {
            return Err(fixture_stage_layout_mismatch(
                fixture.id,
                format!(
                    "cells[{cell_index}] rotated footprint must remain within ±{FIXTURE_STAGE_LAYOUT_WORLD_CAP}"
                ),
            ));
        }
    }
    Ok(())
}

/// Validate the optional v1 stage-layout projection carried by one patched
/// fixture. `None` is an explicit legacy marker and is valid for pre-layout
/// snapshots. A present layout is never repaired, truncated, or inferred:
/// every structural, numeric, and control/segment association invariant is
/// rejected at the current-schema project boundary.
pub fn validate_fixture_stage_layout(
    fixture: &PatchedFixtureSummary,
) -> Result<(), ProjectReferenceIntegrityError> {
    let Some(layout) = fixture.stage_layout.as_ref() else {
        return Ok(());
    };
    let fixture_id = fixture.id;

    if layout.version != 1 {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "unsupported version {}; only version 1 is accepted",
                layout.version
            ),
        ));
    }
    if !(1..=FIXTURE_STAGE_LAYOUT_MAX_ENTRIES).contains(&layout.cells.len()) {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "cells count {} must be within 1..={FIXTURE_STAGE_LAYOUT_MAX_ENTRIES}",
                layout.cells.len()
            ),
        ));
    }
    if layout.logical_segments.len() > FIXTURE_STAGE_LAYOUT_MAX_ENTRIES {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "logical_segments count {} must be within 0..={FIXTURE_STAGE_LAYOUT_MAX_ENTRIES}",
                layout.logical_segments.len()
            ),
        ));
    }
    if !layout.cell_width.is_finite()
        || layout.cell_width <= 0.0
        || layout.cell_width > FIXTURE_STAGE_LAYOUT_WORLD_CAP
    {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "cell_width must be finite, greater than zero, and at most {FIXTURE_STAGE_LAYOUT_WORLD_CAP}"
            ),
        ));
    }
    if !layout.cell_depth.is_finite()
        || layout.cell_depth <= 0.0
        || layout.cell_depth > FIXTURE_STAGE_LAYOUT_WORLD_CAP
    {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            format!(
                "cell_depth must be finite, greater than zero, and at most {FIXTURE_STAGE_LAYOUT_WORLD_CAP}"
            ),
        ));
    }

    let mut global_control_indices = BTreeSet::new();
    for (field, control_index) in [
        (
            "global_dimmer_control_index",
            layout.global_dimmer_control_index,
        ),
        (
            "global_strobe_control_index",
            layout.global_strobe_control_index,
        ),
    ] {
        let Some(control_index) = control_index else {
            continue;
        };
        if usize::from(control_index) >= fixture.controls.len() {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!(
                    "{field} {control_index} is outside {} controls",
                    fixture.controls.len()
                ),
            ));
        }
        if !global_control_indices.insert(control_index) {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!("{field} repeats global control_index {control_index}"),
            ));
        }
    }

    let mut association_state = None;
    let mut associated_logical_indices = BTreeSet::new();
    for (cell_index, cell) in layout.cells.iter().enumerate() {
        validate_cell_coordinate(fixture_id, cell_index, "offset_x", cell.offset_x)?;
        validate_cell_coordinate(fixture_id, cell_index, "offset_z", cell.offset_z)?;
        validate_rotated_cell_footprint(
            fixture,
            cell_index,
            layout.cell_width,
            layout.cell_depth,
            cell.offset_x,
            cell.offset_z,
        )?;
        if usize::from(cell.beam_index) >= layout.cells.len() {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!(
                    "cells[{cell_index}].beam_index {} is outside {} cells",
                    cell.beam_index,
                    layout.cells.len()
                ),
            ));
        }
        let associated = cell.logical_segment_index.is_some();
        match association_state {
            None => association_state = Some(associated),
            Some(expected) if expected != associated => {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    "logical_segment_index association must be all None or all Some",
                ));
            }
            Some(_) => {}
        }
        if let Some(logical_index) = cell.logical_segment_index {
            if layout.logical_segments.is_empty()
                || usize::from(logical_index) >= layout.logical_segments.len()
            {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "cells[{cell_index}].logical_segment_index {logical_index} is outside {} logical segments",
                        layout.logical_segments.len()
                    ),
                ));
            }
            associated_logical_indices.insert(logical_index);
        }
    }

    let mut logical_indices = BTreeSet::new();
    let mut segment_control_indices = BTreeSet::new();
    for (segment_position, segment) in layout.logical_segments.iter().enumerate() {
        if usize::from(segment.logical_index) != segment_position {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!(
                    "logical_segments[{segment_position}].logical_index {} must equal its vector position",
                    segment.logical_index
                ),
            ));
        }
        if !logical_indices.insert(segment.logical_index) {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!(
                    "logical_segments[{segment_position}] duplicates logical_index {}",
                    segment.logical_index
                ),
            ));
        }
        let mut roles = BTreeSet::new();
        let mut control_indices = BTreeSet::new();
        for (binding_index, binding) in segment.color_controls.iter().enumerate() {
            if usize::from(binding.control_index) >= fixture.controls.len() {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "logical_segments[{segment_position}].color_controls[{binding_index}] control_index {} is outside {} controls",
                        binding.control_index,
                        fixture.controls.len()
                    ),
                ));
            }
            if !control_indices.insert(binding.control_index) {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "logical_segments[{segment_position}].color_controls[{binding_index}] repeats control_index {}",
                        binding.control_index
                    ),
                ));
            }
            if global_control_indices.contains(&binding.control_index) {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "logical_segments[{segment_position}].color_controls[{binding_index}] aliases global control_index {}",
                        binding.control_index
                    ),
                ));
            }
            if !segment_control_indices.insert(binding.control_index) {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "logical_segments[{segment_position}].color_controls[{binding_index}] repeats control_index {} across logical segments",
                        binding.control_index
                    ),
                ));
            }
            if !roles.insert(binding.role.clone()) {
                return Err(fixture_stage_layout_mismatch(
                    fixture_id,
                    format!(
                        "logical_segments[{segment_position}] repeats color role {:?}",
                        binding.role
                    ),
                ));
            }
        }
        if segment.color_controls.len() > 6 {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!(
                    "logical_segments[{segment_position}].color_controls cannot exceed the six supported roles"
                ),
            ));
        }
    }

    let mut beam_indices = layout
        .cells
        .iter()
        .map(|cell| cell.beam_index)
        .collect::<Vec<_>>();
    beam_indices.sort_unstable();
    for (expected, actual) in beam_indices.iter().enumerate() {
        if usize::from(*actual) != expected {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                format!("beam_index values must be contiguous 0..n-1; expected {expected}"),
            ));
        }
    }
    if layout.logical_segments.is_empty() {
        if association_state == Some(true) {
            return Err(fixture_stage_layout_mismatch(
                fixture_id,
                "cells must have no logical_segment_index when logical_segments is empty",
            ));
        }
    } else if association_state != Some(true) {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            "cells must have logical_segment_index when logical_segments are present",
        ));
    }
    if association_state == Some(true)
        && associated_logical_indices.len() != layout.logical_segments.len()
    {
        return Err(fixture_stage_layout_mismatch(
            fixture_id,
            "when logical segments are associated, every logical segment must be used",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stage_layout_fixture() -> crate::PatchedFixtureSummary {
        let controls = (0..5)
            .map(|index| crate::AttributeControl {
                attribute: format!("Color {index}"),
                channel_name: format!("Color {index}"),
                geometry: None,
                offsets: vec![index + 1],
                resolution: crate::AttributeResolution::EightBit,
                default_value: 0,
                functions: Vec::new(),
            })
            .collect();
        crate::PatchedFixtureSummary {
            id: 1,
            label: "Fixture 1".to_string(),
            profile_source_path: "fixture.gdtf".to_string(),
            profile_name: "Test".to_string(),
            manufacturer: "Test".to_string(),
            mode_name: "Mode".to_string(),
            universe: 0,
            address: 1,
            group_ids: Vec::new(),
            position: crate::Vec3::default(),
            rotation: crate::Rotation3::default(),
            geometries: Vec::new(),
            controls,
            stage_layout: Some(FixtureStageLayout {
                version: 1,
                cell_width: 0.5,
                cell_depth: 0.5,
                cells: vec![FixtureStageLayoutCell {
                    beam_index: 0,
                    offset_x: 0.0,
                    offset_z: 0.0,
                    logical_segment_index: Some(0),
                }],
                logical_segments: vec![FixtureStageLogicalSegment {
                    logical_index: 0,
                    color_controls: vec![
                        FixtureStageColorBinding {
                            control_index: 0,
                            role: FixtureStageColorRole::Red,
                        },
                        FixtureStageColorBinding {
                            control_index: 1,
                            role: FixtureStageColorRole::Green,
                        },
                        FixtureStageColorBinding {
                            control_index: 2,
                            role: FixtureStageColorRole::Blue,
                        },
                    ],
                }],
                global_dimmer_control_index: Some(3),
                global_strobe_control_index: Some(4),
            }),
            attribute_values: Vec::new(),
            limits: crate::FixtureLimits::default(),
            highlighted: false,
            soloed: false,
            parked: false,
        }
    }

    fn segment(logical_index: u16) -> FixtureStageLogicalSegment {
        FixtureStageLogicalSegment {
            logical_index,
            color_controls: vec![
                FixtureStageColorBinding {
                    control_index: 0,
                    role: FixtureStageColorRole::Red,
                },
                FixtureStageColorBinding {
                    control_index: 1,
                    role: FixtureStageColorRole::Green,
                },
                FixtureStageColorBinding {
                    control_index: 2,
                    role: FixtureStageColorRole::Blue,
                },
            ],
        }
    }

    #[test]
    fn absent_layout_is_legacy_valid_and_omitted() {
        let mut fixture = stage_layout_fixture();
        fixture.stage_layout = None;
        assert!(validate_fixture_stage_layout(&fixture).is_ok());
        let encoded = serde_json::to_value(&fixture).unwrap();
        assert!(encoded.get("stage_layout").is_none());
        let decoded: crate::PatchedFixtureSummary = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded.stage_layout, None);
    }

    #[test]
    fn v1_layout_persists_and_roundtrips() {
        let fixture = stage_layout_fixture();
        assert!(validate_fixture_stage_layout(&fixture).is_ok());
        let encoded = serde_json::to_value(&fixture).unwrap();
        assert!(encoded.get("stage_layout").is_some());
        let decoded: crate::PatchedFixtureSummary = serde_json::from_value(encoded).unwrap();
        assert_eq!(decoded, fixture);
    }

    #[test]
    fn unknown_layout_and_nested_fields_are_rejected() {
        let fixture = stage_layout_fixture();
        let mut unknown_layout = serde_json::to_value(&fixture).unwrap();
        unknown_layout["stage_layout"]["future_layout_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<crate::PatchedFixtureSummary>(unknown_layout).is_err());

        let mut unknown_cell = serde_json::to_value(&fixture).unwrap();
        unknown_cell["stage_layout"]["cells"][0]["future_cell_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<crate::PatchedFixtureSummary>(unknown_cell).is_err());

        let mut unknown_binding = serde_json::to_value(&fixture).unwrap();
        unknown_binding["stage_layout"]["logical_segments"][0]["color_controls"][0]
            ["future_binding_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<crate::PatchedFixtureSummary>(unknown_binding).is_err());
    }

    #[test]
    fn snapshot_reference_validation_invokes_stage_layout_gate() {
        let mut snapshot = crate::EngineSnapshot::default();
        snapshot.fixtures = vec![stage_layout_fixture()];
        snapshot.timeline_bank = vec![snapshot.timeline.clone()];
        assert!(crate::validate_current_engine_snapshot_reference_integrity(&snapshot).is_ok());

        snapshot.fixtures[0].stage_layout.as_mut().unwrap().version = 2;
        assert!(crate::validate_current_engine_snapshot_reference_integrity(&snapshot).is_err());
    }

    #[test]
    fn legacy_empty_timeline_bank_rejects_malformed_layout_first() {
        let mut snapshot = crate::EngineSnapshot::default();
        let mut fixture = stage_layout_fixture();
        fixture.stage_layout.as_mut().unwrap().version = 2;
        snapshot.fixtures = vec![fixture];
        assert!(matches!(
            crate::validate_current_engine_snapshot_reference_integrity(&snapshot),
            Err(crate::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, ref detail })
                if owner == "fixture 1 stage layout" && detail.contains("unsupported version")
        ));
    }

    #[test]
    fn future_version_is_rejected() {
        let mut fixture = stage_layout_fixture();
        fixture.stage_layout.as_mut().unwrap().version = 2;
        assert!(matches!(
            validate_fixture_stage_layout(&fixture),
            Err(crate::ProjectReferenceIntegrityError::ReferenceMismatch { ref owner, ref detail })
                if owner == "fixture 1 stage layout" && detail.contains("unsupported version")
        ));
    }

    #[test]
    fn nonfinite_dimensions_and_offsets_are_rejected() {
        let mut dimension = stage_layout_fixture();
        dimension.stage_layout.as_mut().unwrap().cell_width = f32::NAN;
        assert!(validate_fixture_stage_layout(&dimension).is_err());

        let mut offset = stage_layout_fixture();
        offset.stage_layout.as_mut().unwrap().cells[0].offset_x = f32::INFINITY;
        assert!(validate_fixture_stage_layout(&offset).is_err());
    }

    #[test]
    fn duplicate_beams_and_logical_indices_are_rejected() {
        let mut duplicate_beam = stage_layout_fixture();
        duplicate_beam
            .stage_layout
            .as_mut()
            .unwrap()
            .cells
            .push(FixtureStageLayoutCell {
                beam_index: 0,
                offset_x: 1.0,
                offset_z: 1.0,
                logical_segment_index: Some(0),
            });
        assert!(validate_fixture_stage_layout(&duplicate_beam).is_err());

        let mut duplicate_logical = stage_layout_fixture();
        let layout = duplicate_logical.stage_layout.as_mut().unwrap();
        layout.cells.push(FixtureStageLayoutCell {
            beam_index: 1,
            offset_x: 1.0,
            offset_z: 1.0,
            logical_segment_index: Some(1),
        });
        layout.logical_segments.push(segment(0));
        assert!(validate_fixture_stage_layout(&duplicate_logical).is_err());
    }

    #[test]
    fn mixed_and_unresolved_associations_are_rejected() {
        let mut mixed = stage_layout_fixture();
        mixed
            .stage_layout
            .as_mut()
            .unwrap()
            .cells
            .push(FixtureStageLayoutCell {
                beam_index: 1,
                offset_x: 1.0,
                offset_z: 1.0,
                logical_segment_index: None,
            });
        assert!(validate_fixture_stage_layout(&mixed).is_err());

        let mut out_of_range = stage_layout_fixture();
        out_of_range.stage_layout.as_mut().unwrap().cells[0].logical_segment_index = Some(1);
        assert!(validate_fixture_stage_layout(&out_of_range).is_err());

        let mut unused = stage_layout_fixture();
        let layout = unused.stage_layout.as_mut().unwrap();
        layout.logical_segments.push(segment(1));
        assert!(validate_fixture_stage_layout(&unused).is_err());
    }

    #[test]
    fn out_of_range_control_bindings_are_rejected() {
        let mut global = stage_layout_fixture();
        global
            .stage_layout
            .as_mut()
            .unwrap()
            .global_dimmer_control_index = Some(5);
        assert!(validate_fixture_stage_layout(&global).is_err());

        let mut strobe = stage_layout_fixture();
        strobe
            .stage_layout
            .as_mut()
            .unwrap()
            .global_strobe_control_index = Some(5);
        assert!(validate_fixture_stage_layout(&strobe).is_err());

        let mut color = stage_layout_fixture();
        color.stage_layout.as_mut().unwrap().logical_segments[0].color_controls[0].control_index =
            3;
        assert!(validate_fixture_stage_layout(&color).is_err());
    }

    #[test]
    fn global_and_cross_segment_control_aliases_are_rejected() {
        let mut duplicate_global = stage_layout_fixture();
        duplicate_global
            .stage_layout
            .as_mut()
            .unwrap()
            .global_strobe_control_index = Some(3);
        assert!(validate_fixture_stage_layout(&duplicate_global).is_err());

        let mut aliases_global = stage_layout_fixture();
        aliases_global
            .stage_layout
            .as_mut()
            .unwrap()
            .logical_segments[0]
            .color_controls[0]
            .control_index = 3;
        assert!(validate_fixture_stage_layout(&aliases_global).is_err());

        let mut cross_segment = stage_layout_fixture();
        let layout = cross_segment.stage_layout.as_mut().unwrap();
        layout.cells.push(FixtureStageLayoutCell {
            beam_index: 1,
            offset_x: 1.0,
            offset_z: 0.0,
            logical_segment_index: Some(1),
        });
        layout.logical_segments.push(segment(1));
        assert!(validate_fixture_stage_layout(&cross_segment).is_err());
    }

    #[test]
    fn logical_segments_allow_empty_or_non_rgb_roles_but_reject_duplicates() {
        let mut duplicate = stage_layout_fixture();
        duplicate.stage_layout.as_mut().unwrap().logical_segments[0].color_controls[2].role =
            FixtureStageColorRole::Red;
        assert!(validate_fixture_stage_layout(&duplicate).is_err());

        let mut duplicate_control = stage_layout_fixture();
        duplicate_control
            .stage_layout
            .as_mut()
            .unwrap()
            .logical_segments[0]
            .color_controls[2]
            .control_index = 0;
        assert!(validate_fixture_stage_layout(&duplicate_control).is_err());

        let mut amber_only = stage_layout_fixture();
        amber_only.stage_layout.as_mut().unwrap().logical_segments[0].color_controls =
            vec![FixtureStageColorBinding {
                control_index: 0,
                role: FixtureStageColorRole::Amber,
            }];
        assert!(validate_fixture_stage_layout(&amber_only).is_ok());

        let mut physical_only = stage_layout_fixture();
        let layout = physical_only.stage_layout.as_mut().unwrap();
        layout.logical_segments.clear();
        layout.cells[0].logical_segment_index = None;
        assert!(validate_fixture_stage_layout(&physical_only).is_ok());

        let mut unresolved = physical_only;
        unresolved.stage_layout.as_mut().unwrap().cells[0].logical_segment_index = Some(0);
        assert!(validate_fixture_stage_layout(&unresolved).is_err());
    }

    #[test]
    fn yaw_rotated_cell_footprint_is_inside_world_cap() {
        let mut valid = stage_layout_fixture();
        valid.stage_layout.as_mut().unwrap().cell_width = 2.0;
        valid.stage_layout.as_mut().unwrap().cell_depth = 2.0;
        valid.rotation.yaw = 45.0;
        assert!(validate_fixture_stage_layout(&valid).is_ok());

        let mut invalid = valid;
        invalid.rotation.yaw = 0.0;
        invalid.stage_layout.as_mut().unwrap().cells[0].offset_x = FIXTURE_STAGE_LAYOUT_WORLD_CAP;
        assert!(validate_fixture_stage_layout(&invalid).is_err());

        let mut rotated_offset = stage_layout_fixture();
        rotated_offset.position.z = 990.0;
        rotated_offset.rotation.yaw = 90.0;
        rotated_offset.stage_layout.as_mut().unwrap().cell_width = 2.0;
        rotated_offset.stage_layout.as_mut().unwrap().cell_depth = 2.0;
        rotated_offset.stage_layout.as_mut().unwrap().cells[0].offset_x = 10.0;
        assert!(
            validate_fixture_stage_layout(&rotated_offset).is_err(),
            "fixture-local cell centres must rotate with fixture yaw before the world-cap check"
        );
    }
}
