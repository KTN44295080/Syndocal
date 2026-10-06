//! Bounded managed listing shares the canonical exact-byte inspection reader.
//! No ID allocation, directory creation, publication or project/output locks.
use protocol::control_plane_file::{ProjectBackupListRequestV1, ProjectBackupListV1};
use std::{fs, path::Path};

const MAX_DIRECTORY_ENTRIES: usize = 128;
const MAX_PAGE_READ_BYTES: u64 = 128 * 1024 * 1024;

pub(super) fn list<R: tauri::Runtime>(app: &tauri::AppHandle<R>, request: ProjectBackupListRequestV1) -> Result<ProjectBackupListV1, String> {
    request.validate().map_err(str::to_string)?;
    let directory = super::app_data_subdirectory_path(app, super::PROJECT_BACKUP_DIRECTORY)?;
    list_in(&directory, request)
}

fn list_in(directory: &Path, request: ProjectBackupListRequestV1) -> Result<ProjectBackupListV1, String> {
    request.validate().map_err(str::to_string)?;
    // Apply the shared canonical managed-root policy even to an empty directory.
    super::project_file_managed_backup::path_for_id(directory, 1)?;
    let entries = match fs::read_dir(directory) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(ProjectBackupListV1 {
            schema_version: 1, backups: Vec::new(), next_before_id: None,
        }),
        Err(error) => return Err(format!("project_backup_list_directory: {error}")),
    };
    let mut ids = Vec::new();
    for (index, entry) in entries.enumerate() {
        if index >= MAX_DIRECTORY_ENTRIES { return Err("project_backup_list_directory_limit_128".into()); }
        let entry = entry.map_err(|error| format!("project_backup_list_entry: {error}"))?;
        let name = entry.file_name();
        let name = name.to_str().ok_or("project_backup_list_filename_invalid")?;
        if !name.as_bytes().get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case(b"backup-")) { continue; }
        let id = name.strip_prefix("backup-").and_then(|value| value.strip_suffix(".json"))
            .and_then(|value| value.parse::<u64>().ok()).ok_or("project_backup_list_filename_invalid")?;
        if name != format!("backup-{id}.json") { return Err("project_backup_list_filename_invalid".into()); }
        super::project_file_managed_backup::path_for_id(directory, id)?;
        if request.before_id.is_none_or(|before| id < before) { ids.push(id); }
    }
    ids.sort_unstable_by(|left, right| right.cmp(left));
    let more = ids.len() > request.limit as usize;
    let mut backups = Vec::with_capacity(ids.len().min(request.limit as usize));
    let mut remaining = MAX_PAGE_READ_BYTES;
    for id in ids.into_iter().take(request.limit as usize) {
        let observed = super::project_backup_inspection::read_in_bounded(directory, id, remaining)?;
        remaining = remaining.checked_sub(observed.bytes).ok_or("project_backup_list_read_limit")?;
        backups.push(observed.inspection());
    }
    let next_before_id = if more { backups.last().map(|row| row.backup.id) } else { None };
    let result = ProjectBackupListV1 { schema_version: 1, backups, next_before_id };
    result.validate(&request).map_err(str::to_string)?;
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQUENCE: AtomicU64 = AtomicU64::new(0);
    fn root() -> std::path::PathBuf {
        let root = std::env::temp_dir().join(format!("syndocal-backup-list-{}-{}", std::process::id(), SEQUENCE.fetch_add(1, Ordering::Relaxed)));
        assert!(!root.exists()); root
    }
    fn request(limit: u16, before_id: Option<u64>) -> ProjectBackupListRequestV1 {
        ProjectBackupListRequestV1 { schema_version: 1, limit, before_id }
    }
    fn write(root: &Path, id: u64) -> Vec<u8> {
        let backup = super::super::ProjectBackupEnvelope {
            version: super::super::PROJECT_BACKUP_VERSION, app: super::super::APP_NAME.into(), id, created_at_unix_ms: id,
            source_path: Some("C:/公演/日本語.sdc".into()), reason: "list 日本語".into(),
            project: serde_json::from_str(super::super::PHASE1_SAMPLE_PROJECT_JSON).unwrap(),
            midi_mappings: Vec::new(), osc_mappings: Vec::new(), dmx_mappings: Vec::new(), dj_track_triggers: Vec::new(),
        };
        let mut bytes = super::super::project_backup_json::project_backup_json_bytes(&backup).unwrap();
        bytes.extend_from_slice(b" \r\n");
        fs::write(super::super::project_backup_path(root, id), &bytes).unwrap(); bytes
    }
    #[test]
    fn project_backup_listing_missing_directory_pagination_exact_bytes_and_no_allocation() {
        use sha2::{Digest, Sha256};
        let root = root();
        assert!(list_in(&root, request(2, None)).unwrap().backups.is_empty()); assert!(!root.exists());
        fs::create_dir(&root).unwrap();
        let originals: Vec<_> = [1, 3, 2].into_iter().map(|id| (id, write(&root, id))).collect();
        let sequence = super::super::PROJECT_BACKUP_ID_SEQUENCE.load(Ordering::Acquire);
        let first = list_in(&root, request(2, None)).unwrap();
        assert_eq!(first.backups.iter().map(|row| row.backup.id).collect::<Vec<_>>(), vec![3,2]);
        assert_eq!(first.next_before_id, Some(2));
        let next = list_in(&root, request(2, first.next_before_id)).unwrap();
        assert_eq!(next.backups[0].backup.id, 1); assert_eq!(next.next_before_id, None);
        for row in first.backups.iter().chain(next.backups.iter()) {
            let (_, bytes) = originals.iter().find(|(id,_)| *id == row.backup.id).unwrap();
            assert_eq!(row.artifact_sha256, format!("{:x}", Sha256::digest(bytes)));
            assert_eq!(fs::read(super::super::project_backup_path(&root, row.backup.id)).unwrap(), *bytes);
        }
        assert_eq!(sequence, super::super::PROJECT_BACKUP_ID_SEQUENCE.load(Ordering::Acquire));
        for (id,_) in originals { fs::remove_file(super::super::project_backup_path(&root,id)).unwrap(); } fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_listing_corrupt_nonregular_unsafe_and_directory_budget_fail_closed() {
        let root = root(); fs::create_dir(&root).unwrap();
        fs::write(root.join("backup-01.json"), b"original").unwrap();
        assert!(list_in(&root, request(1,None)).unwrap_err().contains("filename_invalid"));
        assert_eq!(fs::read(root.join("backup-01.json")).unwrap(), b"original"); fs::remove_file(root.join("backup-01.json")).unwrap();
        fs::write(root.join("BACKUP-1.json"), b"case-alias").unwrap();
        assert!(list_in(&root, request(1,None)).unwrap_err().contains("filename_invalid"));
        assert_eq!(fs::read(root.join("BACKUP-1.json")).unwrap(), b"case-alias"); fs::remove_file(root.join("BACKUP-1.json")).unwrap();
        fs::create_dir(root.join("backup-1.json")).unwrap();
        assert!(list_in(&root, request(1,None)).unwrap_err().contains("not_regular_file")); fs::remove_dir(root.join("backup-1.json")).unwrap();
        fs::write(root.join("backup-1.json"), b"{broken").unwrap();
        assert!(list_in(&root, request(1,None)).is_err()); assert_eq!(fs::read(root.join("backup-1.json")).unwrap(), b"{broken"); fs::remove_file(root.join("backup-1.json")).unwrap();
        for index in 0..=MAX_DIRECTORY_ENTRIES { fs::write(root.join(format!("ignored-{index}")), b"unchanged").unwrap(); }
        assert!(list_in(&root, request(1,None)).unwrap_err().contains("directory_limit_128"));
        assert_eq!(fs::read_dir(&root).unwrap().count(), MAX_DIRECTORY_ENTRIES+1);
        for index in 0..=MAX_DIRECTORY_ENTRIES { fs::remove_file(root.join(format!("ignored-{index}"))).unwrap(); } fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_listing_shared_reader_enforces_remaining_budget_and_windows_writer_guard() {
        let root = root(); fs::create_dir(&root).unwrap(); let bytes = write(&root,1);
        assert!(super::super::project_backup_inspection::read_in_bounded(&root,1,bytes.len() as u64-1).err().unwrap().contains("limit"));
        assert_eq!(fs::read(root.join("backup-1.json")).unwrap(),bytes);
        #[cfg(windows)] { let writer=fs::OpenOptions::new().write(true).open(root.join("backup-1.json")).unwrap();
            assert!(list_in(&root,request(1,None)).unwrap_err().contains("inspect_open")); drop(writer); }
        assert_eq!(list_in(&root,request(1,None)).unwrap().backups[0].backup.bytes,bytes.len() as u64);
        fs::remove_file(root.join("backup-1.json")).unwrap(); fs::remove_dir(root).unwrap();
    }
}
