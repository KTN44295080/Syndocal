//! Project schema/mapping decode policy, before project replacement begins.
use crate::{
    input_diagnostic::{bounded_diagnostic, deserialize_input},
    migrate_legacy_spatial_parameter_models, validate_dj_track_triggers,
    validate_dj_track_triggers_against_snapshot, validate_dmx_control_mappings,
    validate_midi_control_mappings, validate_osc_control_mappings, ProjectControlMappings,
    ProjectFile, PROJECT_FILE_VERSION,
};
use serde_json::Value;

pub(super) fn project_and_control_mappings_from_value(
    mut value: Value,
) -> Result<(ProjectFile, ProjectControlMappings), String> {
    let decode = || {
        let version = value
            .get("version")
            .and_then(Value::as_u64)
            .ok_or_else(|| "Project version must be an unsigned integer".to_string())?;
        if version != u64::from(PROJECT_FILE_VERSION) {
            return Err(format!("Unsupported project version {version}"));
        }
        migrate_legacy_spatial_parameter_models(&mut value)?;
        let legacy_dj_transition_discarded = value.get("dj_transition").is_some();
        // Borrow for ProjectFile, then consume for mappings. Avoid cloning the
        // entire input Value (including ignored fields) before typed decoding.
        let project: ProjectFile = deserialize_input(&value, "Project JSON")?;
        let mappings: ProjectControlMappings =
            deserialize_input(value, "Project control mappings")?;
        let mappings = ProjectControlMappings {
            midi_mappings: validate_midi_control_mappings(mappings.midi_mappings)?,
            osc_mappings: validate_osc_control_mappings(mappings.osc_mappings)?,
            dmx_mappings: validate_dmx_control_mappings(mappings.dmx_mappings)?,
            dj_track_triggers: validate_dj_track_triggers(mappings.dj_track_triggers)?,
            legacy_dj_transition_discarded,
        };
        validate_dj_track_triggers_against_snapshot(
            &mappings.dj_track_triggers,
            &project.snapshot,
        )?;
        Ok((project, mappings))
    };
    decode().map_err(bounded_diagnostic)
}
