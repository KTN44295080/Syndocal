use std::collections::{BTreeMap, BTreeSet};

use protocol::{
    FixtureStageColorBinding, FixtureStageColorRole, FixtureStageLayout, FixtureStageLayoutCell,
    FixtureStageLogicalSegment, PatchedFixtureSummary, FIXTURE_STAGE_LAYOUT_MAX_ENTRIES,
};

#[derive(Debug, Clone)]
pub(crate) struct DvcStageChannel {
    pub(crate) control_index: u16,
    pub(crate) channel_type: u16,
    pub(crate) beam_indices: Result<Vec<u16>, String>,
}

#[derive(Debug)]
pub(crate) struct DvcStageLayoutInput<'a> {
    pub(crate) fixture_label: &'a str,
    pub(crate) profile_name: &'a str,
    pub(crate) fixture_x: f32,
    pub(crate) fixture_z: f32,
    pub(crate) fixture_yaw_deg: f32,
    pub(crate) cell_size: f32,
    pub(crate) profile_beam_indices: Result<Vec<u16>, String>,
    pub(crate) patch_beam_positions: &'a BTreeMap<u16, (i64, i64)>,
    pub(crate) channels: &'a [DvcStageChannel],
}

#[derive(Debug)]
pub(crate) struct DvcStageLayoutProjection {
    pub(crate) layout: Option<FixtureStageLayout>,
    pub(crate) message: String,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct StageFootprint {
    pub(crate) min_x: f32,
    pub(crate) max_x: f32,
    pub(crate) min_z: f32,
    pub(crate) max_z: f32,
}

fn exact_contiguous_indices(indices: &[u16], label: &str) -> Result<Vec<u16>, String> {
    if indices.is_empty() {
        return Err(format!("{label} contains no beams"));
    }
    if indices.len() > FIXTURE_STAGE_LAYOUT_MAX_ENTRIES {
        return Err(format!(
            "{label} contains {} beams; the exact projection limit is {}",
            indices.len(),
            FIXTURE_STAGE_LAYOUT_MAX_ENTRIES
        ));
    }
    let unique = indices.iter().copied().collect::<BTreeSet<_>>();
    if unique.len() != indices.len() {
        return Err(format!("{label} contains duplicate beam indices"));
    }
    let expected = (0..u16::try_from(indices.len()).unwrap_or(u16::MAX)).collect::<Vec<_>>();
    let actual = unique.into_iter().collect::<Vec<_>>();
    if actual != expected {
        return Err(format!("{label} beam indices are not contiguous from zero"));
    }
    Ok(actual)
}

fn color_role(channel_type: u16) -> Option<FixtureStageColorRole> {
    match channel_type {
        25 => Some(FixtureStageColorRole::Red),
        26 => Some(FixtureStageColorRole::Green),
        27 => Some(FixtureStageColorRole::Blue),
        31 => Some(FixtureStageColorRole::White),
        45 => Some(FixtureStageColorRole::Amber),
        46 => Some(FixtureStageColorRole::Uv),
        _ => None,
    }
}

fn unique_global_control(channels: &[DvcStageChannel], channel_type: u16) -> Option<u16> {
    let matches = channels
        .iter()
        .filter(|channel| channel.channel_type == channel_type)
        .filter_map(|channel| {
            channel
                .beam_indices
                .as_ref()
                .ok()
                .filter(|indices| indices.is_empty())
                .map(|_| channel.control_index)
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        matches.first().copied()
    } else {
        None
    }
}

fn exact_logical_segments(
    beam_indices: &[u16],
    channels: &[DvcStageChannel],
) -> Result<(Vec<FixtureStageLogicalSegment>, BTreeMap<u16, u16>), String> {
    let color_channels = channels
        .iter()
        .filter_map(|channel| color_role(channel.channel_type).map(|role| (channel, role)))
        .collect::<Vec<_>>();
    if color_channels.is_empty() {
        return Err("profile has no beam-associated color channels".to_string());
    }

    let mut groups = BTreeMap::<Vec<u16>, Vec<FixtureStageColorBinding>>::new();
    for (channel, role) in color_channels {
        let mut associated = channel
            .beam_indices
            .as_ref()
            .map_err(|reason| format!("color channel association is invalid: {reason}"))?
            .clone();
        associated.sort_unstable();
        associated.dedup();
        if associated.is_empty() {
            return Err("a color channel has no exact beam association".to_string());
        }
        if associated.iter().any(|beam| !beam_indices.contains(beam)) {
            return Err(
                "a color channel references a beam outside the physical layout".to_string(),
            );
        }
        let bindings = groups.entry(associated).or_default();
        if bindings.iter().any(|binding| binding.role == role) {
            return Err("a logical segment has duplicate color roles".to_string());
        }
        bindings.push(FixtureStageColorBinding {
            control_index: channel.control_index,
            role,
        });
    }

    let mut occupied = BTreeSet::new();
    for beams in groups.keys() {
        for beam in beams {
            if !occupied.insert(*beam) {
                return Err("color channel groups overlap on a physical beam".to_string());
            }
        }
    }
    if occupied.iter().copied().collect::<Vec<_>>() != beam_indices {
        return Err("color channel groups do not partition every physical beam".to_string());
    }

    let mut ordered = groups.into_iter().collect::<Vec<_>>();
    ordered.sort_by_key(|(beams, _)| beams[0]);
    if ordered.len() > FIXTURE_STAGE_LAYOUT_MAX_ENTRIES {
        return Err("logical segment count exceeds the exact projection limit".to_string());
    }
    let mut beam_to_logical = BTreeMap::new();
    let mut segments = Vec::with_capacity(ordered.len());
    for (index, (beams, mut bindings)) in ordered.into_iter().enumerate() {
        bindings.sort_by_key(|binding| binding.role.clone());
        let logical_index = u16::try_from(index).map_err(|_| "logical index overflowed")?;
        for beam in beams {
            beam_to_logical.insert(beam, logical_index);
        }
        segments.push(FixtureStageLogicalSegment {
            logical_index,
            color_controls: bindings,
        });
    }
    Ok((segments, beam_to_logical))
}

fn collapse_strongpoint_duplicate_rows(
    profile_name: &str,
    cells: &[FixtureStageLayoutCell],
    logical_segment_count: usize,
) -> Option<Vec<FixtureStageLayoutCell>> {
    let normalized_profile = profile_name
        .to_ascii_lowercase()
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .collect::<String>();
    if normalized_profile != "960soundwavesstrongpoint"
        || cells.len() != 12
        || logical_segment_count != 4
    {
        return None;
    }
    let mut collapsed = Vec::with_capacity(logical_segment_count);
    let mut reference_rows: Option<Vec<f32>> = None;
    for logical_index in 0..logical_segment_count {
        let logical_index = u16::try_from(logical_index).ok()?;
        let mut group = cells
            .iter()
            .filter(|cell| cell.logical_segment_index == Some(logical_index))
            .collect::<Vec<_>>();
        if group.len() != 3 {
            return None;
        }
        group.sort_by(|left, right| left.offset_z.total_cmp(&right.offset_z));
        let first_x = group[0].offset_x;
        if group
            .iter()
            .any(|cell| (cell.offset_x - first_x).abs() > 0.001)
        {
            return None;
        }
        let rows = group.iter().map(|cell| cell.offset_z).collect::<Vec<_>>();
        if let Some(reference) = reference_rows.as_ref() {
            if rows
                .iter()
                .zip(reference)
                .any(|(actual, expected)| (actual - expected).abs() > 0.001)
            {
                return None;
            }
        } else {
            reference_rows = Some(rows);
        }
        collapsed.push(FixtureStageLayoutCell {
            // The three source beams are a known Daslight Strongpoint display
            // duplication. Syndocal persists the operator-confirmed physical
            // four-cell topology, one cell per exact logical RGB segment.
            beam_index: logical_index,
            offset_x: group.iter().map(|cell| cell.offset_x).sum::<f32>() / 3.0,
            offset_z: group.iter().map(|cell| cell.offset_z).sum::<f32>() / 3.0,
            logical_segment_index: Some(logical_index),
        });
    }
    Some(collapsed)
}

pub(crate) fn project_dvc_stage_layout(input: DvcStageLayoutInput<'_>) -> DvcStageLayoutProjection {
    let fail = |reason: String| DvcStageLayoutProjection {
        layout: None,
        message: format!("{}: {reason}", input.fixture_label),
    };
    if !input.fixture_x.is_finite()
        || !input.fixture_z.is_finite()
        || !input.fixture_yaw_deg.is_finite()
        || !input.cell_size.is_finite()
        || input.cell_size <= f32::EPSILON
    {
        return fail("fixture position, yaw, or SIZE is not a finite positive value".to_string());
    }
    let beam_indices = match input
        .profile_beam_indices
        .and_then(|indices| exact_contiguous_indices(&indices, "profile"))
    {
        Ok(indices) => indices,
        Err(reason) => return fail(reason),
    };
    let patch_indices = input
        .patch_beam_positions
        .keys()
        .copied()
        .collect::<Vec<_>>();
    if patch_indices != beam_indices {
        return fail("patch beam coordinates do not exactly match the profile beams".to_string());
    }

    let (sin_yaw, cos_yaw) = input.fixture_yaw_deg.to_radians().sin_cos();
    let logical = exact_logical_segments(&beam_indices, input.channels);
    let (logical_segments, beam_to_logical, logical_message) = match logical {
        Ok((segments, mapping)) => (
            segments,
            mapping,
            "physical cells and logical color segments converted exactly".to_string(),
        ),
        Err(reason) => (
            Vec::new(),
            BTreeMap::new(),
            format!("physical cells converted exactly; logical segments omitted: {reason}"),
        ),
    };
    let mut cells = Vec::with_capacity(beam_indices.len());
    for beam_index in beam_indices {
        let point = input.patch_beam_positions[&beam_index];
        let dx = point.0 as f32 - input.fixture_x;
        let dz = point.1 as f32 - input.fixture_z;
        if !dx.is_finite() || !dz.is_finite() {
            return fail("patch beam coordinate cannot be represented exactly".to_string());
        }
        // Daslight stores beam centres in stage/world coordinates. Persist
        // fixture-local offsets so the shared renderer can apply fixture yaw
        // exactly once at display time.
        let offset_x = dx * cos_yaw + dz * sin_yaw;
        let offset_z = -dx * sin_yaw + dz * cos_yaw;
        if !offset_x.is_finite() || !offset_z.is_finite() {
            return fail("fixture-local beam coordinate is not finite".to_string());
        }
        cells.push(FixtureStageLayoutCell {
            beam_index,
            offset_x,
            offset_z,
            logical_segment_index: beam_to_logical.get(&beam_index).copied(),
        });
    }
    let collapsed_strongpoint =
        collapse_strongpoint_duplicate_rows(input.profile_name, &cells, logical_segments.len());
    let did_collapse_strongpoint = collapsed_strongpoint.is_some();
    if let Some(collapsed) = collapsed_strongpoint {
        cells = collapsed;
    }

    DvcStageLayoutProjection {
        layout: Some(FixtureStageLayout {
            version: 1,
            cell_width: input.cell_size,
            cell_depth: input.cell_size,
            cells,
            logical_segments,
            global_dimmer_control_index: unique_global_control(input.channels, 7),
            global_strobe_control_index: unique_global_control(input.channels, 15),
        }),
        message: format!(
            "{}: {}",
            input.fixture_label,
            if did_collapse_strongpoint {
                "known Daslight three-row Strongpoint duplication collapsed to four physical/logical cells"
            } else {
                &logical_message
            }
        ),
    }
}

pub(crate) fn fixture_stage_footprint(fixture: &PatchedFixtureSummary) -> Option<StageFootprint> {
    let Some(layout) = fixture.stage_layout.as_ref() else {
        return (fixture.position.x.is_finite() && fixture.position.z.is_finite()).then_some(
            StageFootprint {
                min_x: fixture.position.x,
                max_x: fixture.position.x,
                min_z: fixture.position.z,
                max_z: fixture.position.z,
            },
        );
    };
    if !fixture.position.x.is_finite()
        || !fixture.position.z.is_finite()
        || !fixture.rotation.yaw.is_finite()
        || !layout.cell_width.is_finite()
        || !layout.cell_depth.is_finite()
    {
        return None;
    }
    let (sin_yaw, cos_yaw) = fixture.rotation.yaw.to_radians().sin_cos();
    let half_width = layout.cell_width * 0.5;
    let half_depth = layout.cell_depth * 0.5;
    let mut footprint = StageFootprint {
        min_x: f32::INFINITY,
        max_x: f32::NEG_INFINITY,
        min_z: f32::INFINITY,
        max_z: f32::NEG_INFINITY,
    };
    for cell in &layout.cells {
        let center_x = fixture.position.x + cell.offset_x * cos_yaw - cell.offset_z * sin_yaw;
        let center_z = fixture.position.z + cell.offset_x * sin_yaw + cell.offset_z * cos_yaw;
        for (x, z) in [
            (-half_width, -half_depth),
            (-half_width, half_depth),
            (half_width, -half_depth),
            (half_width, half_depth),
        ] {
            let world_x = center_x + x * cos_yaw - z * sin_yaw;
            let world_z = center_z + x * sin_yaw + z * cos_yaw;
            if !world_x.is_finite() || !world_z.is_finite() {
                return None;
            }
            footprint.min_x = footprint.min_x.min(world_x);
            footprint.max_x = footprint.max_x.max(world_x);
            footprint.min_z = footprint.min_z.min(world_z);
            footprint.max_z = footprint.max_z.max(world_z);
        }
    }
    (footprint.min_x.is_finite()
        && footprint.max_x.is_finite()
        && footprint.min_z.is_finite()
        && footprint.max_z.is_finite())
    .then_some(footprint)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb_channels(groups: &[&[u16]]) -> Vec<DvcStageChannel> {
        groups
            .iter()
            .flat_map(|beams| {
                [25_u16, 26, 27]
                    .into_iter()
                    .map(move |channel_type| (beams, channel_type))
            })
            .enumerate()
            .map(|(control_index, (beams, channel_type))| DvcStageChannel {
                control_index: u16::try_from(control_index).unwrap(),
                channel_type,
                beam_indices: Ok(beams.to_vec()),
            })
            .collect()
    }

    #[test]
    fn known_strongpoint_three_row_duplication_collapses_to_four_physical_cells() {
        let patch = (0..12_u16)
            .map(|beam| {
                (
                    beam,
                    (
                        100 + i64::from(beam / 3) * 30,
                        200 + i64::from(beam % 3) * 30,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let groups = [&[0, 1, 2][..], &[3, 4, 5], &[6, 7, 8], &[9, 10, 11]];
        let channels = rgb_channels(&groups);
        let projected = project_dvc_stage_layout(DvcStageLayoutInput {
            fixture_label: "strongpoint",
            profile_name: "960 sound waves strongpoint",
            fixture_x: 100.0,
            fixture_z: 200.0,
            fixture_yaw_deg: 0.0,
            cell_size: 30.0,
            profile_beam_indices: Ok((0..12).collect()),
            patch_beam_positions: &patch,
            channels: &channels,
        });
        let layout = projected.layout.unwrap();
        assert_eq!(layout.cells.len(), 4);
        assert_eq!(layout.logical_segments.len(), 4);
        assert_eq!(layout.cells[0].offset_z, 30.0);
        assert_eq!(layout.cells[1].offset_x, 30.0);
        assert_eq!(layout.cells[3].logical_segment_index, Some(3));
    }

    #[test]
    fn unrelated_twelve_cell_matrix_is_not_collapsed() {
        let patch = (0..12_u16)
            .map(|beam| {
                (
                    beam,
                    (
                        100 + i64::from(beam / 3) * 30,
                        200 + i64::from(beam % 3) * 30,
                    ),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let groups = [&[0, 1, 2][..], &[3, 4, 5], &[6, 7, 8], &[9, 10, 11]];
        let channels = rgb_channels(&groups);
        let projected = project_dvc_stage_layout(DvcStageLayoutInput {
            fixture_label: "960 sound waves strongpoint",
            profile_name: "Other Twelve Cell Matrix",
            fixture_x: 100.0,
            fixture_z: 200.0,
            fixture_yaw_deg: 0.0,
            cell_size: 30.0,
            profile_beam_indices: Ok((0..12).collect()),
            patch_beam_positions: &patch,
            channels: &channels,
        });
        assert_eq!(projected.layout.unwrap().cells.len(), 12);
    }

    #[test]
    fn yaw_is_removed_from_persisted_fixture_local_offsets() {
        let patch = BTreeMap::from([(0, (10, 20)), (1, (10, 50))]);
        let channels = rgb_channels(&[&[0][..], &[1]]);
        let projected = project_dvc_stage_layout(DvcStageLayoutInput {
            fixture_label: "rotated bar",
            profile_name: "Rotated Bar",
            fixture_x: 10.0,
            fixture_z: 20.0,
            fixture_yaw_deg: 90.0,
            cell_size: 30.0,
            profile_beam_indices: Ok(vec![0, 1]),
            patch_beam_positions: &patch,
            channels: &channels,
        });
        let layout = projected.layout.unwrap();
        assert!((layout.cells[1].offset_x - 30.0).abs() < 0.001);
        assert!(layout.cells[1].offset_z.abs() < 0.001);
    }

    #[test]
    fn profile_patch_mismatch_is_not_repaired_or_inferred() {
        let patch = BTreeMap::from([(0, (0, 0)), (2, (60, 0))]);
        let projected = project_dvc_stage_layout(DvcStageLayoutInput {
            fixture_label: "ambiguous",
            profile_name: "Ambiguous",
            fixture_x: 0.0,
            fixture_z: 0.0,
            fixture_yaw_deg: 0.0,
            cell_size: 30.0,
            profile_beam_indices: Ok(vec![0, 1]),
            patch_beam_positions: &patch,
            channels: &[],
        });
        assert!(projected.layout.is_none());
    }

    #[test]
    fn exact_physical_cells_survive_ambiguous_logical_channels_without_guessing() {
        let patch = BTreeMap::from([(0, (0, 0)), (1, (30, 0))]);
        let channels = vec![DvcStageChannel {
            control_index: 0,
            channel_type: 25,
            beam_indices: Err("duplicate beam metadata".to_string()),
        }];
        let projected = project_dvc_stage_layout(DvcStageLayoutInput {
            fixture_label: "physical only",
            profile_name: "Physical Only",
            fixture_x: 0.0,
            fixture_z: 0.0,
            fixture_yaw_deg: 0.0,
            cell_size: 30.0,
            profile_beam_indices: Ok(vec![0, 1]),
            patch_beam_positions: &patch,
            channels: &channels,
        });
        let layout = projected.layout.unwrap();
        assert_eq!(layout.cells.len(), 2);
        assert!(layout.logical_segments.is_empty());
        assert!(layout
            .cells
            .iter()
            .all(|cell| cell.logical_segment_index.is_none()));
    }
}
