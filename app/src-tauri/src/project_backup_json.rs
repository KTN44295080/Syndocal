//! Backup envelope byte/JSON policy. No engine, publication or output ownership.
use crate::{
    input_diagnostic::{bounded_diagnostic, deserialize_input},
    project_backup_id_from_reserved_target,
    project_file_json::{parse_json_with_byte_limit, read_json_with_byte_limit},
    validate_app_name, validate_project_file, ProjectBackupEnvelope, PROJECT_BACKUP_VERSION,
};
use std::path::Path;

// Separate from the 64 MiB .sdc bound: the envelope adds indentation and metadata.
// The writer and reader share this limit; source backups are never rewritten.
pub(super) const PROJECT_BACKUP_MAX_BYTES: u64 = 128 * 1024 * 1024;

fn validate_backup_envelope(backup: &ProjectBackupEnvelope) -> Result<(), String> {
    if backup.version != PROJECT_BACKUP_VERSION {
        return Err(format!(
            "Unsupported project backup version {}",
            backup.version
        ));
    }
    if backup.id == 0 {
        return Err("Project backup ID must be positive".to_string());
    }
    validate_app_name("project backup", &backup.app)?;
    validate_project_file(&backup.project)
}

fn parse_project_backup_json(json: &str) -> Result<ProjectBackupEnvelope, String> {
    let value = parse_json_with_byte_limit(json, PROJECT_BACKUP_MAX_BYTES, "Project backup JSON")?;
    // Consume the unique-key Value; no second byte parse or persistence-image clone.
    let backup = deserialize_input(value, "Project backup JSON")?;
    validate_backup_envelope(&backup)?;
    Ok(backup)
}

pub(super) fn read_project_backup_json(path: &Path) -> Result<ProjectBackupEnvelope, String> {
    expected_backup_id(path)?;
    let json = read_json_with_byte_limit(path, PROJECT_BACKUP_MAX_BYTES, "Project backup file")
        .map_err(|error| {
            bounded_diagnostic(format_args!(
                "Unable to read project backup: {error}; file {}",
                path.display()
            ))
        })?;
    decode_project_backup_json_at_path(path, &json)
}

pub(super) fn decode_project_backup_json_at_path(
    path: &Path,
    json: &str,
) -> Result<ProjectBackupEnvelope, String> {
    let expected_id = expected_backup_id(path)?;
    let backup = parse_project_backup_json(json).map_err(|error| {
        bounded_diagnostic(format_args!(
            "Invalid project backup: {error}; file {}",
            path.display()
        ))
    })?;
    if backup.id != expected_id {
        return Err(format!(
            "Project backup ID {} does not match filename ID {expected_id}; keep the original file and select a matching backup",
            backup.id
        ));
    }
    Ok(backup)
}

fn expected_backup_id(path: &Path) -> Result<u64, String> {
    let directory = path
        .parent()
        .ok_or_else(|| "Project backup has no managed parent directory".to_string())?;
    project_backup_id_from_reserved_target(directory, path)
}

pub(super) fn project_backup_json_bytes(backup: &ProjectBackupEnvelope) -> Result<Vec<u8>, String> {
    backup_json_bytes_with_limit(backup, PROJECT_BACKUP_MAX_BYTES).map_err(bounded_diagnostic)
}

fn backup_json_bytes_with_limit(
    backup: &ProjectBackupEnvelope,
    limit: u64,
) -> Result<Vec<u8>, String> {
    validate_backup_envelope(backup)?;
    let bytes = serde_json::to_vec_pretty(backup).map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "Project backup JSON is {} bytes; the limit is {limit} bytes",
            bytes.len()
        ));
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{project_backup_path, APP_NAME, PHASE1_SAMPLE_PROJECT_JSON};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicU64, Ordering},
    };

    static DIRECTORY_SEQUENCE: AtomicU64 = AtomicU64::new(0);
    struct TestDirectory(PathBuf);
    impl TestDirectory {
        fn new() -> Self {
            let directory = std::env::temp_dir().join(format!(
                "syndocal-backup-json-{}-{}",
                std::process::id(),
                DIRECTORY_SEQUENCE.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&directory).unwrap();
            Self(directory)
        }
    }
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            assert_eq!(self.0.parent(), Some(std::env::temp_dir().as_path()));
            assert!(self
                .0
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("syndocal-backup-json-"));
            fs::remove_dir_all(&self.0).unwrap();
        }
    }
    fn sample(id: u64) -> ProjectBackupEnvelope {
        ProjectBackupEnvelope {
            version: PROJECT_BACKUP_VERSION,
            app: APP_NAME.to_string(),
            id,
            created_at_unix_ms: 97,
            source_path: Some("C:/公演/日本語.sdc".to_string()),
            reason: "before update 日本語".to_string(),
            project: serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).unwrap(),
            midi_mappings: Vec::new(),
            osc_mappings: Vec::new(),
            dmx_mappings: Vec::new(),
            dj_track_triggers: Vec::new(),
        }
    }

    #[test]
    fn project_backup_json_valid_round_trip_retains_unicode_and_defaults() {
        let original = sample(u64::MAX);
        let bytes = project_backup_json_bytes(&original).unwrap();
        let loaded = parse_project_backup_json(std::str::from_utf8(&bytes).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&original).unwrap()
        );
        assert_eq!(project_backup_json_bytes(&loaded).unwrap(), bytes);
        assert!(loaded.midi_mappings.is_empty());
        let json = serde_json::to_string(&original).unwrap();
        let additive = format!("{{\"future\":{{\"x\":1}},{}", &json[1..]);
        let loaded = parse_project_backup_json(&additive).unwrap();
        assert_eq!(
            serde_json::to_value(loaded).unwrap(),
            serde_json::to_value(original).unwrap()
        );
    }

    #[test]
    fn project_backup_json_rejects_duplicate_known_ignored_and_escaped_keys() {
        let json = serde_json::to_string(&sample(1)).unwrap();
        for prefix in [
            r#""version":1,"#,
            r#""ver\u0073ion":1,"#,
            r#""future":0,"future":1,"#,
            r#""future":{"x":0,"x":1},"#,
        ] {
            let error =
                parse_project_backup_json(&format!("{{{prefix}{}", &json[1..])).unwrap_err();
            assert!(error.contains("duplicate object key"), "{error}");
            assert!(error.contains("line 1 column"), "{error}");
        }
        let error = parse_project_backup_json(
            &json.replace("\"project\":{", "\"project\":{\"extra\":{\"x\":0,\"x\":1},"),
        )
        .unwrap_err();
        assert!(error.contains("duplicate object key"));
    }

    #[test]
    fn project_backup_json_rejects_invalid_versions_schema_and_numbers() {
        let mut envelope = sample(1);
        envelope.version = 2;
        assert!(
            parse_project_backup_json(&serde_json::to_string(&envelope).unwrap())
                .unwrap_err()
                .contains("Unsupported project backup version 2")
        );
        envelope.version = PROJECT_BACKUP_VERSION;
        envelope.project.version = 2;
        assert!(
            parse_project_backup_json(&serde_json::to_string(&envelope).unwrap())
                .unwrap_err()
                .contains("Unsupported project version 2")
        );
        envelope = sample(1);
        envelope.app = "foreign".to_string();
        assert!(
            parse_project_backup_json(&serde_json::to_string(&envelope).unwrap())
                .unwrap_err()
                .contains("Unsupported project backup app")
        );
        for json in ["{}", "null", "{", "{} {}", "{\"x\":1e9999}"] {
            assert!(parse_project_backup_json(json).is_err());
        }
        let error =
            parse_project_backup_json(&serde_json::to_string(&sample(0)).unwrap()).unwrap_err();
        assert!(error.contains("backup ID must be positive"));
    }

    #[test]
    fn project_backup_json_reader_binds_exact_positive_canonical_filename_id() {
        let directory = TestDirectory::new();
        let bytes = project_backup_json_bytes(&sample(u64::MAX)).unwrap();
        let matching = project_backup_path(&directory.0, u64::MAX);
        fs::write(&matching, &bytes).unwrap();
        assert_eq!(read_project_backup_json(&matching).unwrap().id, u64::MAX);
        let mismatched = project_backup_path(&directory.0, 1);
        fs::write(&mismatched, &bytes).unwrap();
        assert!(read_project_backup_json(&mismatched)
            .unwrap_err()
            .contains("does not match filename ID 1"));
        assert_eq!(fs::read(&mismatched).unwrap(), bytes);
        let noncanonical = directory.0.join("backup-01.json");
        fs::write(
            &noncanonical,
            project_backup_json_bytes(&sample(1)).unwrap(),
        )
        .unwrap();
        assert!(read_project_backup_json(&noncanonical)
            .unwrap_err()
            .contains("not canonical"));
        assert!(matching.exists() && mismatched.exists() && noncanonical.exists());
    }

    #[test]
    fn project_backup_json_reader_rejects_oversize_utf8_and_truncation_without_writes() {
        let directory = TestDirectory::new();
        let oversized = project_backup_path(&directory.0, 1);
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(PROJECT_BACKUP_MAX_BYTES + 1).unwrap();
        drop(file);
        assert!(read_project_backup_json(&oversized)
            .unwrap_err()
            .contains("limit is 134217728"));
        assert_eq!(
            fs::metadata(&oversized).unwrap().len(),
            PROJECT_BACKUP_MAX_BYTES + 1
        );
        let invalid = project_backup_path(&directory.0, 2);
        for bytes in [b"\xff{".as_slice(), b"{\"version\":1"] {
            fs::write(&invalid, bytes).unwrap();
            assert!(read_project_backup_json(&invalid).is_err());
            assert_eq!(fs::read(&invalid).unwrap(), bytes);
        }
    }

    #[test]
    fn project_backup_json_parser_exact_128_mib_boundary_precedes_schema() {
        let mut json = " ".repeat(PROJECT_BACKUP_MAX_BYTES as usize - 2);
        json.push_str("{}");
        let error = parse_project_backup_json(&json).unwrap_err();
        assert!(
            error.starts_with("Invalid Project backup JSON schema at [root]:")
                && error.contains("missing required field")
                && !error.contains("limit"),
            "{error}"
        );
        json.push(' ');
        assert!(parse_project_backup_json(&json)
            .unwrap_err()
            .contains("limit is 134217728"));
    }

    #[test]
    fn project_backup_json_writer_uses_the_reader_limit_before_any_file_publication() {
        let original = sample(1);
        let bytes = project_backup_json_bytes(&original).unwrap();
        assert_eq!(
            backup_json_bytes_with_limit(&original, bytes.len() as u64).unwrap(),
            bytes
        );
        let error = backup_json_bytes_with_limit(&original, bytes.len() as u64 - 1).unwrap_err();
        assert!(error.contains("limit"));
        let directory = TestDirectory::new();
        let last_valid = project_backup_path(&directory.0, 1);
        fs::write(&last_valid, &bytes).unwrap();
        assert!(backup_json_bytes_with_limit(&original, 1).is_err());
        assert_eq!(fs::read(&last_valid).unwrap(), bytes);
        assert_eq!(fs::read_dir(&directory.0).unwrap().count(), 1);
    }
}
