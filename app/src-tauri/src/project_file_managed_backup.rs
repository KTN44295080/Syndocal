//! Managed backup destination policy. Queries observe a candidate, never reserve
//! IDs or create directories; the existing publication service owns that work.
use protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER as MAX;
use std::{
    fs,
    path::{Component, Path, PathBuf},
    sync::atomic::Ordering,
};
use tauri::AppHandle;

fn canonical_directory(directory: &Path) -> Result<PathBuf, String> {
    match fs::symlink_metadata(directory) {
        Ok(metadata) if !metadata.is_dir() || metadata.file_type().is_symlink() => {
            return Err("project_file_backup_directory_invalid".into())
        }
        Ok(_) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(format!("project_file_backup_directory_metadata: {error}")),
    }
    super::normalized_recovery_target_key(directory)
}

fn key_in(directory: &Path, destination: &Path) -> Result<PathBuf, String> {
    super::validate_project_publication_target_path_v1(
        super::ProjectPublicationSurfaceV1::Backup,
        destination,
    )?;
    if destination
        .components()
        .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("project_file_backup_target_parent_traversal".into());
    }
    let root = canonical_directory(directory)?;
    let id = super::project_backup_id_from_reserved_target(&root, destination)?;
    if id == 0 || id > MAX {
        return Err("project_file_backup_target_id_unsafe".into());
    }
    Ok(super::project_backup_path(&root, id))
}

pub(super) fn key(app: &AppHandle, destination: &str) -> Result<PathBuf, String> {
    let directory = super::app_data_subdirectory(app, super::PROJECT_BACKUP_DIRECTORY)?;
    key_in(&directory, Path::new(destination))
}

fn observe_in(directory: &Path, now: u64, high_water: u64) -> Result<PathBuf, String> {
    let root = canonical_directory(directory)?;
    let mut candidate = now.max(
        high_water
            .checked_add(1)
            .ok_or("project_file_backup_id_exhausted")?,
    );
    // Fixed inspection bound; a saturated target range fails closed.
    for _ in 0..64 {
        if candidate == 0 || candidate > MAX {
            return Err("project_file_backup_id_exhausted".into());
        }
        let target = super::project_backup_path(&root, candidate);
        match fs::symlink_metadata(&target) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(target),
            Ok(_) => {}
            Err(error) => return Err(format!("project_file_backup_target_metadata: {error}")),
        }
        candidate += 1;
    }
    Err("project_file_backup_target_range_busy".into())
}

pub(super) fn observe(app: &AppHandle) -> Result<PathBuf, String> {
    let directory = super::app_data_subdirectory(app, super::PROJECT_BACKUP_DIRECTORY)?;
    observe_in(
        &directory,
        super::current_unix_ms().min(MAX as u128) as u64,
        super::PROJECT_BACKUP_ID_SEQUENCE.load(Ordering::Acquire),
    )
}

pub(super) fn legacy_target(directory: &Path, destination: &str) -> Result<PathBuf, String> {
    let canonical = key_in(directory, Path::new(destination))?;
    let name = canonical
        .file_name()
        .ok_or("project_file_backup_target_name_invalid")?;
    let id = super::project_backup_id_from_reserved_target(
        &canonical_directory(directory)?,
        &canonical,
    )?;
    // Allocation happens only on write. Keep the ordinary managed allocator
    // above every accepted target, including explicit MCP targets.
    super::PROJECT_BACKUP_ID_SEQUENCE.fetch_max(id, Ordering::AcqRel);
    // Keep the existing service's path spelling and persisted schema.
    Ok(directory.join(name))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn directory() -> PathBuf {
        let parent = std::env::temp_dir().join(format!(
            "syndocal-project-file-backup-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir(&parent).unwrap();
        parent
    }
    #[test]
    fn project_file_managed_backup_query_does_not_create_or_reserve_and_is_bounded() {
        let parent = directory();
        let managed = parent.join("project-backups");
        let first = observe_in(&managed, 10, 0).unwrap();
        assert_eq!(first, observe_in(&managed, 10, 0).unwrap());
        assert!(!managed.exists());
        fs::create_dir(&managed).unwrap();
        for id in 10..74 {
            fs::write(super::super::project_backup_path(&managed, id), b"existing").unwrap();
        }
        assert_eq!(
            observe_in(&managed, 10, 0).unwrap_err(),
            "project_file_backup_target_range_busy"
        );
        assert!(observe_in(&managed, MAX, MAX).is_err());
        for id in 10..74 {
            fs::remove_file(super::super::project_backup_path(&managed, id)).unwrap();
        }
        fs::remove_dir(&managed).unwrap();
        fs::remove_dir(parent).unwrap();
    }
    #[test]
    fn project_file_managed_backup_rejects_outside_noncanonical_unsafe_and_directory_file() {
        let parent = directory();
        let managed = parent.join("project-backups");
        let root = canonical_directory(&managed).unwrap();
        let valid = root.join("backup-1.json");
        assert_eq!(key_in(&managed, &valid).unwrap(), valid);
        assert_eq!(
            legacy_target(&managed, valid.to_str().unwrap()).unwrap(),
            managed.join("backup-1.json")
        );
        for name in [
            "backup-01.json",
            "backup-0.json",
            "backup-9007199254740992.json",
            "arbitrary.json",
        ] {
            assert!(key_in(&managed, &root.join(name)).is_err());
        }
        assert!(key_in(&managed, &parent.join("backup-1.json")).is_err());
        assert!(key_in(&managed, &root.join("../backup-1.json")).is_err());
        fs::write(&managed, b"not a directory").unwrap();
        assert!(canonical_directory(&managed).is_err());
        fs::remove_file(&managed).unwrap();
        fs::remove_dir(parent).unwrap();
    }
}
