//! Bounded write-ahead facts for exact managed artifact deletion. A prepared
//! fact is unresolved, never permission to repeat an unknown filesystem effect.
use std::{collections::HashSet, fs, io::{Read, Write}, path::{Path, PathBuf}, sync::atomic::Ordering};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use protocol::control_plane_file::{ProjectBackupDeleteRequestV1 as Request,
    ProjectBackupDeleteReceiptV1 as Receipt, ProjectBackupDeleteStatusV1 as Status,
    ProjectBackupDeletePhaseV1 as StatusPhase};

pub(super) const FILE_NAME: &str = "project-backup-deletions-v1.json";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RECORDS: usize = 256;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase { Prepared, Succeeded, Rejected }
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Record {
    origin: String,
    shape_sha256: String,
    phase: Phase,
    receipt: Receipt,
    error: Option<String>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Journal {
    version: u16,
    records: Vec<Record>,
}
impl Default for Journal {
    fn default() -> Self { Self { version: 1, records: Vec::new() } }
}

pub(super) fn path_for_recovery_journal(path: &Path) -> PathBuf { path.with_file_name(FILE_NAME) }
pub(super) fn origin(caller: &str, owner: &str) -> Result<String, String> {
    serde_json::to_vec(&("backup-deletion-origin-v1",caller,owner))
        .map(|bytes|format!("{:x}",Sha256::digest(bytes)))
        .map_err(|_|"project_backup_delete_journal_origin_encoding".into())
}
fn shape(request: &Request) -> Result<String, String> {
    serde_json::to_vec(request).map(|bytes|format!("{:x}",Sha256::digest(bytes)))
        .map_err(|_|"project_backup_delete_journal_request_encoding".into())
}
fn hash_valid(value: &str) -> bool { value.len()==64 && value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)) }

impl Journal {
    fn validate(&self) -> Result<(), String> {
        if self.version!=1 { return Err("project_backup_delete_journal_version_unsupported".into()); }
        if self.records.len()>MAX_RECORDS { return Err("project_backup_delete_journal_capacity".into()); }
        let mut keys=HashSet::new();
        for record in &self.records {
            record.receipt.validate().map_err(str::to_string)?;
            if !hash_valid(&record.origin) || record.shape_sha256!=shape(&record.receipt.request)?
                || !keys.insert((&record.origin,record.receipt.request.request_id))
                || (record.phase==Phase::Rejected)!=record.error.is_some()
                || record.error.as_ref().is_some_and(|e|e.is_empty()||e.len()>1024) {
                return Err("project_backup_delete_journal_record_invalid".into());
            }
        }
        Ok(())
    }
    pub(super) fn status(&self, origin: &str, request: &Request) -> Result<Status, String> {
        request.validate().map_err(str::to_string)?;
        let found=self.records.iter().find(|r|r.origin==origin&&r.receipt.request.request_id==request.request_id);
        let expected_shape=shape(request)?;
        if found.is_some_and(|r|r.shape_sha256!=expected_shape) {
            return Err("Authored mutation request id was reused with a different shape".into());
        }
        let (phase,receipt,error)=match found {
            None=>(StatusPhase::Unknown,None,None),
            Some(r)=>match r.phase {
                Phase::Prepared=>(StatusPhase::Indeterminate,None,Some("Deletion outcome is unresolved; inspect status and preserve the artifact; do not repeat the mutation".into())),
                Phase::Succeeded=>(StatusPhase::Succeeded,Some(r.receipt.clone()),None),
                Phase::Rejected=>(StatusPhase::Rejected,None,r.error.clone()),
            },
        };
        let status=Status {schema_version:1,request:request.clone(),phase,receipt,error};
        status.validate().map_err(str::to_string)?; Ok(status)
    }
    pub(super) fn ensure_unresolved_backup_absent(&self, id: u64) -> Result<(), String> {
        if self.records.iter().any(|r|r.phase==Phase::Prepared&&r.receipt.request.backup_id==id) {
            return Err("project_backup_delete_unresolved_journal_protects_artifact; inspect deletion status; do not repeat the mutation".into());
        }
        Ok(())
    }
    pub(super) fn prepare(&mut self, origin: &str, receipt: Receipt) -> Result<usize, String> {
        if self.records.len()>=MAX_RECORDS { return Err("project_backup_delete_journal_capacity; preserve journal and inspect deletion status".into()); }
        if self.status(origin,&receipt.request)?.phase!=StatusPhase::Unknown { return Err("project_backup_delete_journal_identity_already_recorded".into()); }
        self.ensure_unresolved_backup_absent(receipt.request.backup_id)?;
        let record=Record {origin:origin.into(),shape_sha256:shape(&receipt.request)?,phase:Phase::Prepared,receipt,error:None};
        let index=self.records.len();self.records.push(record);self.validate()?;Ok(index)
    }
    pub(super) fn succeeded(&mut self, index: usize) -> Result<(), String> {
        let record=self.records.get_mut(index).ok_or("project_backup_delete_journal_record_missing")?;
        if record.phase!=Phase::Prepared { return Err("project_backup_delete_journal_phase_conflict".into()); }
        record.phase=Phase::Succeeded;self.validate()
    }
    pub(super) fn rejected(&mut self, index: usize, error: String) -> Result<(), String> {
        let record=self.records.get_mut(index).ok_or("project_backup_delete_journal_record_missing")?;
        if record.phase!=Phase::Prepared { return Err("project_backup_delete_journal_phase_conflict".into()); }
        record.phase=Phase::Rejected;record.error=Some(error);self.validate()
    }
}

fn regular(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    { use std::os::windows::fs::MetadataExt;
      use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
      metadata.is_file()&&!metadata.file_type().is_symlink()&&metadata.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT.0==0 }
    #[cfg(not(windows))]
    { metadata.is_file()&&!metadata.file_type().is_symlink() }
}
fn exists_regular(path: &Path) -> Result<bool, String> {
    match fs::symlink_metadata(path) {
        Ok(m) if regular(&m)=>Ok(true),
        Ok(_)=>Err("project_backup_delete_journal_not_regular_file".into()),
        Err(e) if e.kind()==std::io::ErrorKind::NotFound=>Ok(false),
        Err(e)=>Err(format!("project_backup_delete_journal_metadata: {e}")),
    }
}
pub(super) fn load(path: &Path) -> Result<Journal, String> {
    if !exists_regular(path)? { return Ok(Journal::default()); }
    let mut options=fs::OpenOptions::new();options.read(true);
    #[cfg(windows)]
    { use std::os::windows::fs::OpenOptionsExt;
      use windows::Win32::Storage::FileSystem::{FILE_SHARE_READ,FILE_FLAG_OPEN_REPARSE_POINT};
      options.share_mode(FILE_SHARE_READ.0).custom_flags(FILE_FLAG_OPEN_REPARSE_POINT.0); }
    let file=options.open(path).map_err(|e|format!("project_backup_delete_journal_open: {e}"))?;
    let metadata=file.metadata().map_err(|e|format!("project_backup_delete_journal_opened_metadata: {e}"))?;
    if !regular(&metadata)||metadata.len()>MAX_BYTES { return Err("project_backup_delete_journal_not_bounded_regular_file".into()); }
    let mut bytes=Vec::new();(&file).take(MAX_BYTES+1).read_to_end(&mut bytes)
        .map_err(|e|format!("project_backup_delete_journal_read: {e}"))?;
    if bytes.len() as u64!=metadata.len()||file.metadata().map_err(|e|e.to_string())?.len()!=metadata.len() {
        return Err("project_backup_delete_journal_size_changed".into());
    }
    let json=std::str::from_utf8(&bytes).map_err(|_|"project_backup_delete_journal_invalid_utf8")?;
    let value=super::project_file_json::parse_json_with_byte_limit(json,MAX_BYTES,"Backup deletion journal")?;
    let journal: Journal=serde_json::from_value(value).map_err(|e|format!("project_backup_delete_journal_invalid: {e}"))?;
    journal.validate()?;Ok(journal)
}

// Callers hold the existing project publication lock. No background writer,
// implicit eviction, clock-driven retry or reconstruction of corrupt state.
pub(super) fn persist(path: &Path, journal: &Journal) -> Result<(), String> {
    journal.validate()?;
    let bytes=serde_json::to_vec(journal).map_err(|_|"project_backup_delete_journal_encoding")?;
    if bytes.len() as u64>MAX_BYTES { return Err("project_backup_delete_journal_capacity".into()); }
    let parent=path.parent().ok_or("project_backup_delete_journal_parent_missing")?;
    let parent_metadata=fs::symlink_metadata(parent).map_err(|e|e.to_string())?;
    #[cfg(windows)]
    let parent_reparse={use std::os::windows::fs::MetadataExt;
        use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_REPARSE_POINT;
        parent_metadata.file_attributes()&FILE_ATTRIBUTE_REPARSE_POINT.0!=0};
    #[cfg(not(windows))]
    let parent_reparse=parent_metadata.file_type().is_symlink();
    if !parent_metadata.is_dir()||parent_reparse {
        return Err("project_backup_delete_journal_parent_invalid".into());
    }
    let existed=exists_regular(path)?;
    let sequence=super::PROJECT_SAVE_TEMP_SEQUENCE.fetch_add(1,Ordering::Relaxed);
    let temp=parent.join(format!(".{FILE_NAME}.{}.{sequence}.tmp",std::process::id()));
    let mut owned_temp=false;
    let result=(|| {
        let mut file=fs::OpenOptions::new().write(true).create_new(true).open(&temp)
            .map_err(|e|format!("project_backup_delete_journal_temp: {e}"))?;
        owned_temp=true;
        file.write_all(&bytes).and_then(|_|file.sync_all())
            .map_err(|e|format!("project_backup_delete_journal_flush: {e}"))?;
        drop(file);
        if existed { super::replace_file_atomically(&temp,path) }
        else { super::publish_new_file_atomically(&temp,path) }
    })();
    if owned_temp&&temp.exists() { fs::remove_file(&temp).map_err(|e|format!("project_backup_delete_journal_temp_cleanup: {e}"))?; }
    result
}

pub(super) fn ensure_backup_not_unresolved(recovery: &Path, id: u64) -> Result<(), String> {
    load(&path_for_recovery_journal(recovery))?.ensure_unresolved_backup_absent(id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::control_plane_command::ProjectMutationFenceV1;
    use protocol::control_plane_file::{BACKUP_DELETE_ID,ProjectBackupInspectionV1,ProjectFileBackupSummaryV1};
    fn receipt(id:u64)->Receipt {
        Receipt {schema_version:1,request:Request {schema_version:1,operation_id:BACKUP_DELETE_ID.into(),request_id:id,
            expected_fence:ProjectMutationFenceV1 {process_incarnation:1,session_incarnation:2,project_epoch:0,
                project_revision:0,project_checkpoint_hash:"a".repeat(64),project_publication_generation:0},
            backup_id:1,expected_artifact_sha256:"c".repeat(64)},deleted_backup:ProjectBackupInspectionV1 {schema_version:1,
                backup:ProjectFileBackupSummaryV1 {id:1,created_at_unix_ms:1,source_path:None,reason:"journal test".into(),bytes:1},
                artifact_sha256:"c".repeat(64),restore_source_path:None}}
    }
    fn directory()->PathBuf {
        let path=std::env::temp_dir().join(format!("syndocal-delete-journal-{}-{}",std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        assert!(!path.exists());fs::create_dir(&path).unwrap();path
    }
    #[test]
    fn project_backup_deletion_journal_persists_exact_phases_identity_shape_and_bounded_capacity() {
        let root=directory();let path=root.join(FILE_NAME);let origin=origin("one","owner").unwrap();
        let value=receipt(1);let mut journal=load(&path).unwrap();assert!(!path.exists());
        assert_eq!(journal.status(&origin,&value.request).unwrap().phase,StatusPhase::Unknown);
        let index=journal.prepare(&origin,value.clone()).unwrap();persist(&path,&journal).unwrap();
        let mut loaded=load(&path).unwrap();assert_eq!(loaded.status(&origin,&value.request).unwrap().phase,StatusPhase::Indeterminate);
        assert!(loaded.ensure_unresolved_backup_absent(1).is_err());
        assert_eq!(loaded.status(&"d".repeat(64),&value.request).unwrap().phase,StatusPhase::Unknown);
        loaded.succeeded(index).unwrap();persist(&path,&loaded).unwrap();
        let restored=load(&path).unwrap().status(&origin,&value.request).unwrap();
        assert_eq!(restored.phase,StatusPhase::Succeeded);assert_eq!(serde_json::to_value(restored.receipt.unwrap()).unwrap(),serde_json::to_value(&value).unwrap());
        assert!(load(&path).unwrap().ensure_unresolved_backup_absent(1).is_ok());
        let mut different=value.request.clone();different.expected_artifact_sha256="f".repeat(64);
        assert!(load(&path).unwrap().status(&origin,&different).unwrap_err().contains("different shape"));
        let mut saturated=Journal::default();
        for id in 1..=MAX_RECORDS as u64 {let index=saturated.prepare(&origin,receipt(id)).unwrap();saturated.succeeded(index).unwrap();}
        assert!(saturated.prepare(&origin,receipt(MAX_RECORDS as u64+1)).unwrap_err().contains("capacity"));
        assert_eq!(saturated.records.len(),MAX_RECORDS);assert_eq!(saturated.status(&origin,&value.request).unwrap().phase,StatusPhase::Succeeded);
        fs::remove_file(path).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_deletion_journal_rejects_future_duplicate_utf8_oversize_shape_and_phase_without_repair() {
        let root=directory();let path=root.join(FILE_NAME);let origin=origin("one","owner").unwrap();
        let mut valid=Journal::default();let index=valid.prepare(&origin,receipt(1)).unwrap();
        valid.rejected(index,"revoked at final effect".into()).unwrap();persist(&path,&valid).unwrap();
        assert_eq!(load(&path).unwrap().status(&origin,&receipt(1).request).unwrap().error.as_deref(),Some("revoked at final effect"));
        let encoded=serde_json::to_value(&valid).unwrap();
        let mut invalids=vec![b"{\"version\":2,\"records\":[]}".to_vec(),b"{\"version\":1,\"version\":1,\"records\":[]}".to_vec(),vec![255,123]];
        for index in 0..4 {
            let mut bad=encoded.clone();match index {
                0=>bad["records"][0]["shape_sha256"]="a".repeat(64).into(),
                1=>bad["records"][0]["phase"]="succeeded".into(),
                2=>{let row=bad["records"][0].clone();bad["records"].as_array_mut().unwrap().push(row);},
                _=>bad["records"][0]["owner_override"]="forged".into(),
            }invalids.push(serde_json::to_vec(&bad).unwrap());
        }
        for bytes in invalids {fs::write(&path,&bytes).unwrap();assert!(load(&path).is_err());assert_eq!(fs::read(&path).unwrap(),bytes);}
        fs::File::create(&path).unwrap().set_len(MAX_BYTES+1).unwrap();assert!(load(&path).unwrap_err().contains("not_bounded_regular_file"));
        assert_eq!(fs::metadata(&path).unwrap().len(),MAX_BYTES+1);fs::remove_file(&path).unwrap();
        fs::create_dir(&path).unwrap();assert!(load(&path).unwrap_err().contains("not_regular_file"));
        assert!(persist(&path,&valid).is_err());fs::remove_dir(path).unwrap();fs::remove_dir(root).unwrap();
    }
    #[cfg(windows)]
    #[test]
    fn project_backup_deletion_journal_competing_writer_and_failed_atomic_publication_preserve_original() {
        use std::os::windows::fs::OpenOptionsExt;
        let root=directory();let path=root.join(FILE_NAME);let origin=origin("one","owner").unwrap();
        let mut journal=Journal::default();let index=journal.prepare(&origin,receipt(1)).unwrap();persist(&path,&journal).unwrap();
        let before=fs::read(&path).unwrap();
        let writer=fs::OpenOptions::new().write(true).open(&path).unwrap();
        assert!(load(&path).unwrap_err().contains("journal_open"));drop(writer);assert_eq!(fs::read(&path).unwrap(),before);
        let reader=fs::OpenOptions::new().read(true).share_mode(windows::Win32::Storage::FileSystem::FILE_SHARE_READ.0).open(&path).unwrap();
        journal.succeeded(index).unwrap();assert!(persist(&path,&journal).is_err());
        assert_eq!(fs::read(&path).unwrap(),before);assert_eq!(fs::read_dir(&root).unwrap().count(),1);drop(reader);
        persist(&path,&journal).unwrap();assert_eq!(load(&path).unwrap().status(&origin,&receipt(1).request).unwrap().phase,StatusPhase::Succeeded);
        fs::remove_file(path).unwrap();fs::remove_dir(root).unwrap();
    }
}
