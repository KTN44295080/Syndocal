//! Bounded managed-backup observation. No reservation, engine or output effects.
use protocol::control_plane_file::{
    ProjectBackupInspectRequestV1, ProjectBackupInspectionV1, ProjectFileBackupSummaryV1,
};
use sha2::{Digest, Sha256};
use std::{fs, path::Path};

pub(super) struct ObservedBackup {
    pub(super) backup: super::ProjectBackupEnvelope,
    pub(super) artifact_sha256: String,
    pub(super) bytes: u64,
    pub(super) restore_source_path: Option<String>,
}

impl ObservedBackup {
    pub(super) fn inspection(&self) -> ProjectBackupInspectionV1 {
        ProjectBackupInspectionV1 {
            schema_version: 1,
            backup: ProjectFileBackupSummaryV1 {
                id: self.backup.id,
                created_at_unix_ms: self.backup.created_at_unix_ms,
                source_path: self.backup.source_path.clone(),
                reason: self.backup.reason.clone(),
                bytes: self.bytes,
            },
            artifact_sha256: self.artifact_sha256.clone(),
            restore_source_path: self.restore_source_path.clone(),
        }
    }
}

pub(super) fn inspect<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    request: ProjectBackupInspectRequestV1,
) -> Result<ProjectBackupInspectionV1, String> {
    request.validate().map_err(str::to_string)?;
    let directory = super::app_data_subdirectory_path(app, super::PROJECT_BACKUP_DIRECTORY)?;
    inspect_in(&directory, request)
}

fn inspect_in(
    directory: &Path,
    request: ProjectBackupInspectRequestV1,
) -> Result<ProjectBackupInspectionV1, String> {
    request.validate().map_err(str::to_string)?;
    Ok(read_in(directory, request.backup_id)?.inspection())
}

// Query and restore must hash and decode one identical bounded read. The
// observation contains the full candidate without cloning its project image.
pub(super) fn read_in(directory: &Path, backup_id: u64) -> Result<ObservedBackup, String> {
    read_in_bounded(directory, backup_id, super::project_backup_json::PROJECT_BACKUP_MAX_BYTES)
}

pub(super) fn read_in_bounded(directory: &Path, backup_id: u64, remaining_bytes: u64) -> Result<ObservedBackup, String> {
    let request = ProjectBackupInspectRequestV1 {
        schema_version: 1,
        backup_id,
    };
    request.validate().map_err(str::to_string)?;
    let path = super::project_file_managed_backup::path_for_id(directory, request.backup_id)?;
    let metadata = fs::symlink_metadata(&path)
        .map_err(|error| format!("project_backup_inspect_metadata: {error}"))?;
    if !metadata.is_file() || metadata.file_type().is_symlink() {
        return Err("project_backup_inspect_not_regular_file".into());
    }
    let mut options = fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        use windows::Win32::Storage::FileSystem::{FILE_FLAG_OPEN_REPARSE_POINT, FILE_SHARE_READ};
        // The opened leaf must not follow a reparse point, and no writer/deleter
        // may change this handle's bytes while reading on Windows.
        options
            .custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0)
            .share_mode(FILE_SHARE_READ.0);
    }
    let file = options
        .open(&path)
        .map_err(|error| format!("project_backup_inspect_open: {error}"))?;
    read_opened(&path, &file, remaining_bytes)
}

// Both observation and deletion decode/hash the same opened artifact. The
// caller owns sharing/access policy and keeps its original handle alive.
pub(super) fn read_opened(path: &Path, file: &fs::File, remaining_bytes: u64) -> Result<ObservedBackup, String> {
    let metadata = file
        .metadata()
        .map_err(|error| format!("project_backup_inspect_opened_metadata: {error}"))?;
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        if metadata.file_attributes() & FILE_ATTRIBUTE_REPARSE_POINT.0 != 0 {
            return Err("project_backup_inspect_reparse_point".into());
        }
    }
    if !metadata.is_file() {
        return Err("project_backup_inspect_not_regular_file".into());
    }
    let bytes = super::project_file_json::read_bounded_project_bytes(
        file,
        metadata.len(),
        remaining_bytes.min(super::project_backup_json::PROJECT_BACKUP_MAX_BYTES),
        "Project backup inspection",
    )?;
    let sha256 = format!("{:x}", Sha256::digest(&bytes));
    let json = std::str::from_utf8(&bytes)
        .map_err(|error| format!("Project backup inspection is not valid UTF-8: {error}"))?;
    let backup = super::project_backup_json::decode_project_backup_json_at_path(path, json)?;
    let restore_source_path = backup
        .source_path
        .as_ref()
        .filter(|path| super::is_syndocal_project_path(Path::new(path)))
        .cloned();
    let result = ObservedBackup {
        backup,
        artifact_sha256: sha256,
        bytes: bytes.len() as u64,
        restore_source_path,
    };
    result.inspection().validate().map_err(str::to_string)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn directory() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "syndocal-backup-inspect-{}-{}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        root
    }
    fn request(id: u64) -> ProjectBackupInspectRequestV1 {
        ProjectBackupInspectRequestV1 {
            schema_version: 1,
            backup_id: id,
        }
    }
    fn sample(id: u64) -> super::super::ProjectBackupEnvelope {
        super::super::ProjectBackupEnvelope {
            version: super::super::PROJECT_BACKUP_VERSION,
            app: super::super::APP_NAME.into(),
            id,
            created_at_unix_ms: 97,
            source_path: Some("C:/公演/日本語.sdc".into()),
            reason: "before update 日本語".into(),
            project: serde_json::from_str(super::super::PHASE1_SAMPLE_PROJECT_JSON).unwrap(),
            midi_mappings: Vec::new(),
            osc_mappings: Vec::new(),
            dmx_mappings: Vec::new(),
            dj_track_triggers: Vec::new(),
        }
    }
    #[test]
    fn project_backup_inspection_exact_original_bytes_and_metadata_without_writes() {
        let root = directory();
        let path = super::super::project_backup_path(&root, 1);
        let backup = sample(1);
        // Whitespace differs from a serializer: hash must bind original bytes.
        let mut bytes =
            super::super::project_backup_json::project_backup_json_bytes(&backup).unwrap();
        bytes.extend_from_slice(b" \r\n\t");
        fs::write(&path, &bytes).unwrap();
        let result = inspect_in(&root, request(1)).unwrap();
        assert_eq!(
            result.artifact_sha256,
            format!("{:x}", Sha256::digest(&bytes))
        );
        assert_eq!(result.backup.bytes, bytes.len() as u64);
        assert_eq!(result.backup.reason, backup.reason);
        assert_eq!(result.backup.source_path, backup.source_path);
        assert_eq!(result.restore_source_path, backup.source_path);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        #[cfg(windows)]
        {
            let writer = fs::OpenOptions::new().write(true).open(&path).unwrap();
            assert!(inspect_in(&root, request(1))
                .unwrap_err()
                .contains("inspect_open"));
            drop(writer);
            assert_eq!(
                inspect_in(&root, request(1)).unwrap().artifact_sha256,
                result.artifact_sha256
            );
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        let mut backup = backup;
        backup.source_path = Some("C:/公演/template.json".into());
        fs::write(
            &path,
            super::super::project_backup_json::project_backup_json_bytes(&backup).unwrap(),
        )
        .unwrap();
        assert!(inspect_in(&root, request(1))
            .unwrap()
            .restore_source_path
            .is_none());
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_inspection_missing_invalid_and_oversize_preserve_filesystem() {
        let root = directory();
        let managed = root.join("missing");
        assert!(inspect_in(&managed, request(1)).is_err());
        assert!(!managed.exists());
        for id in [
            0,
            protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER + 1,
        ] {
            assert!(inspect_in(&managed, request(id)).is_err());
            assert!(!managed.exists());
        }
        let path = super::super::project_backup_path(&root, 1);
        fs::create_dir(&path).unwrap();
        assert_eq!(
            inspect_in(&root, request(1)).unwrap_err(),
            "project_backup_inspect_not_regular_file"
        );
        fs::remove_dir(&path).unwrap();
        let original =
            super::super::project_backup_json::project_backup_json_bytes(&sample(2)).unwrap();
        for bytes in [original.as_slice(), b"{\"x\":0,\"x\":1}", b"\xff{"] {
            fs::write(&path, bytes).unwrap();
            assert!(inspect_in(&root, request(1)).is_err());
            assert_eq!(fs::read(&path).unwrap(), bytes);
        }
        let file = fs::File::create(&path).unwrap();
        file.set_len(super::super::project_backup_json::PROJECT_BACKUP_MAX_BYTES + 1)
            .unwrap();
        drop(file);
        assert!(inspect_in(&root, request(1)).unwrap_err().contains("limit"));
        assert_eq!(
            fs::metadata(&path).unwrap().len(),
            super::super::project_backup_json::PROJECT_BACKUP_MAX_BYTES + 1
        );
        fs::remove_file(path).unwrap();
        fs::remove_dir(root).unwrap();
    }
}
