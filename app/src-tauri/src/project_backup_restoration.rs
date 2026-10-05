//! Prepare one byte-bound managed backup for the shared replacement lifecycle.
use protocol::control_plane_project::ProjectReplacementErrorV1 as Error;
use std::path::Path;

pub(super) fn prepare(
    directory: &Path,
    backup_id: u64,
    expected_file_sha256: &str,
    expected_source_path: &Option<String>,
) -> Result<super::PreparedProjectLoad, Error> {
    let observed = super::project_backup_inspection::read_in(directory, backup_id)
        .map_err(|_| Error::InvalidProject)?;
    if observed.artifact_sha256 != expected_file_sha256
        || observed.restore_source_path != *expected_source_path
    {
        return Err(Error::FileChanged);
    }
    let backup = observed.backup;
    let mut prepared = super::prepare_project_load(
        backup.project,
        super::ProjectControlMappings {
            midi_mappings: backup.midi_mappings,
            osc_mappings: backup.osc_mappings,
            dmx_mappings: backup.dmx_mappings,
            dj_track_triggers: backup.dj_track_triggers,
            legacy_dj_transition_discarded: false,
        },
        format!("Backup {}", backup.created_at_unix_ms),
        observed.restore_source_path.as_deref().map(Path::new),
    )
    .map_err(|_| Error::InvalidProject)?;
    // The projected original path is metadata. Restoring a backup does not
    // prove that its current source file contains this image or is writable.
    prepared.authority_disposition = super::ProjectAuthorityDisposition::UnsavedReplacement;
    Ok(prepared)
}
