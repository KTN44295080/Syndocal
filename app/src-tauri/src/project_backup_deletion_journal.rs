//! Bounded write-ahead facts for exact managed artifact deletion. A prepared
//! fact is unresolved, never permission to repeat an unknown filesystem effect.
use std::{collections::HashSet, fs, io::{Read, Write}, path::{Path, PathBuf}, sync::atomic::Ordering};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use protocol::control_plane_file::{ProjectBackupDeleteRequestV1 as Request,
    ProjectBackupDeleteReceiptV1 as Receipt, ProjectBackupDeleteStatusV1 as Status,
    ProjectBackupDeletePhaseV1 as StatusPhase};
use protocol::control_plane_backup_management::{ManagementRequestV1, ManagementReceiptV1,
    ManagementActionV1, ArtifactObservationV1, JournalQueryRequestV1, JournalViewV1,
    JournalRecordSummaryV1, JournalRecordKindV1};
use protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER as MAX;

pub(super) const FILE_NAME: &str = "project-backup-deletions-v1.json";
const MAX_BYTES: u64 = 8 * 1024 * 1024;
const MAX_RECORDS: usize = 256;
const MAX_MANAGEMENT: usize = 128;
const MAX_RETIRED: usize = 256;

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Phase { Prepared, Succeeded, Rejected, ResolvedUnknown }
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
    generation: u64,
    records: Vec<Record>,
    management: Vec<Management>,
    retired: Vec<Retired>,
}
impl Default for Journal {
    fn default() -> Self { Self { version: 2, generation: 0, records: Vec::new(), management: Vec::new(), retired: Vec::new() } }
}
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct LegacyJournal { version: u16, records: Vec<Record> }
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Management { origin: String, shape_sha256: String, receipt: ManagementReceiptV1 }
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
enum RetiredKind { Deletion, Management }
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Retired { kind: RetiredKind, origin: String, process_incarnation: u64, high_water_request_id: u64 }
pub(super) struct Observation { pub(super) journal: Journal, pub(super) sha256: String }
fn fingerprint(value: &impl Serialize) -> Result<String, String> {
    serde_json::to_vec(value).map(|bytes|format!("{:x}",Sha256::digest(bytes)))
        .map_err(|_|"project_backup_delete_journal_fingerprint_encoding".into())
}
fn deletion_id(record: &Record) -> Result<String,String> { fingerprint(&("deletion-fact-v1",record)) }
fn management_id(record: &Management) -> Result<String,String> { fingerprint(&("management-fact-v1",record)) }
fn retired_id(record: &Retired) -> Result<String,String> { fingerprint(&("retired-origin-v1",record)) }

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
    pub(super) fn generation(&self)->u64 {self.generation}
    fn validate(&self) -> Result<(), String> {
        if !matches!(self.version,1|2) { return Err("project_backup_delete_journal_version_unsupported".into()); }
        if self.generation>MAX || (self.version==1 && (self.generation!=0||!self.management.is_empty()||!self.retired.is_empty())) {
            return Err("project_backup_delete_journal_generation_invalid".into());
        }
        if self.version==2 && self.generation==0 && (!self.records.is_empty()||!self.management.is_empty()||!self.retired.is_empty()) {
            return Err("project_backup_delete_journal_generation_invalid".into());
        }
        if self.records.len()>MAX_RECORDS||self.management.len()>MAX_MANAGEMENT||self.retired.len()>MAX_RETIRED {
            return Err("project_backup_delete_journal_capacity".into());
        }
        let mut keys=HashSet::new();
        for record in &self.records {
            record.receipt.validate().map_err(str::to_string)?;
            if !hash_valid(&record.origin) || record.shape_sha256!=shape(&record.receipt.request)?
                || !keys.insert((&record.origin,record.receipt.request.request_id))
                || (matches!(record.phase,Phase::Rejected|Phase::ResolvedUnknown))!=record.error.is_some()
                || (self.version==1&&record.phase==Phase::ResolvedUnknown)
                || record.error.as_ref().is_some_and(|e|e.is_empty()||e.len()>1024) {
                return Err("project_backup_delete_journal_record_invalid".into());
            }
        }
        let mut management_keys=HashSet::new();
        for record in &self.management {
            record.receipt.validate().map_err(str::to_string)?;
            if !hash_valid(&record.origin)||record.shape_sha256!=fingerprint(&record.receipt.request)?
                || record.receipt.generation_after>self.generation
                || !management_keys.insert((&record.origin,record.receipt.request.request_id)) {
                return Err("project_backup_delete_journal_management_invalid".into());
            }
        }
        let mut retired_keys=HashSet::new();
        for record in &self.retired {
            if !hash_valid(&record.origin)||record.process_incarnation==0||record.process_incarnation>MAX
                ||record.high_water_request_id==0||record.high_water_request_id>MAX
                ||!retired_keys.insert((record.kind,&record.origin,record.process_incarnation)) {
                return Err("project_backup_delete_journal_retired_invalid".into());
            }
        }
        Ok(())
    }
    fn reserve_generations(&self, count:u64)->Result<(),String> {
        self.generation.checked_add(count).filter(|value|*value<=MAX)
            .map(|_|()).ok_or_else(||"project_backup_delete_journal_generation_exhausted".into())
    }
    fn advance(&mut self)->Result<(),String> { self.reserve_generations(1)?;self.generation+=1;self.version=2;Ok(()) }
    fn reject_retired(&self, kind:RetiredKind, origin:&str, process:u64, id:u64)->Result<(),String> {
        if self.retired.iter().any(|r|r.kind==kind&&r.origin==origin&&r.process_incarnation==process&&id<=r.high_water_request_id) {
            return Err("project_backup_delete_receipt_expired; acknowledged identity is retired; resnapshot and use a new request id".into());
        }
        Ok(())
    }
    pub(super) fn status(&self, origin: &str, request: &Request) -> Result<Status, String> {
        request.validate().map_err(str::to_string)?;
        let found=self.records.iter().find(|r|r.origin==origin&&r.receipt.request.request_id==request.request_id);
        let expected_shape=shape(request)?;
        if found.is_none() {self.reject_retired(RetiredKind::Deletion,origin,request.expected_fence.process_incarnation,request.request_id)?;}
        if found.is_some_and(|r|r.shape_sha256!=expected_shape) {
            return Err("Authored mutation request id was reused with a different shape".into());
        }
        let (phase,receipt,error)=match found {
            None=>(StatusPhase::Unknown,None,None),
            Some(r)=>match r.phase {
                Phase::Prepared=>(StatusPhase::Indeterminate,None,Some("Deletion outcome is unresolved; inspect status and preserve the artifact; do not repeat the mutation".into())),
                Phase::Succeeded=>(StatusPhase::Succeeded,Some(r.receipt.clone()),None),
                Phase::Rejected=>(StatusPhase::Rejected,None,r.error.clone()),
                Phase::ResolvedUnknown=>(StatusPhase::Indeterminate,None,r.error.clone()),
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
        self.reserve_generations(2)?;
        if self.records.len()>=MAX_RECORDS { return Err("project_backup_delete_journal_capacity; preserve journal and inspect deletion status".into()); }
        if self.status(origin,&receipt.request)?.phase!=StatusPhase::Unknown { return Err("project_backup_delete_journal_identity_already_recorded".into()); }
        self.ensure_unresolved_backup_absent(receipt.request.backup_id)?;
        let record=Record {origin:origin.into(),shape_sha256:shape(&receipt.request)?,phase:Phase::Prepared,receipt,error:None};
        let index=self.records.len();self.records.push(record);self.advance()?;self.validate()?;Ok(index)
    }
    pub(super) fn succeeded(&mut self, index: usize) -> Result<(), String> {
        let record=self.records.get_mut(index).ok_or("project_backup_delete_journal_record_missing")?;
        if record.phase!=Phase::Prepared { return Err("project_backup_delete_journal_phase_conflict".into()); }
        record.phase=Phase::Succeeded;self.advance()?;self.validate()
    }
    pub(super) fn rejected(&mut self, index: usize, error: String) -> Result<(), String> {
        let record=self.records.get_mut(index).ok_or("project_backup_delete_journal_record_missing")?;
        if record.phase!=Phase::Prepared { return Err("project_backup_delete_journal_phase_conflict".into()); }
        record.phase=Phase::Rejected;record.error=Some(error);self.advance()?;self.validate()
    }

    pub(super) fn management_status(&self, origin:&str, request:&ManagementRequestV1)
        ->Result<Option<ManagementReceiptV1>,String> {
        request.validate().map_err(str::to_string)?;
        if let Some(record)=self.management.iter().find(|r|r.origin==origin&&r.receipt.request.request_id==request.request_id) {
            if record.receipt.request!=*request {return Err("Authored mutation request id was reused with a different shape".into());}
            return Ok(Some(record.receipt.clone()));
        }
        self.reject_retired(RetiredKind::Management,origin,request.expected_fence.process_incarnation,request.request_id)?;
        Ok(None)
    }
    pub(super) fn release_backup_id(&self, action:&ManagementActionV1)->Result<Option<u64>,String> {
        if let ManagementActionV1::ReleaseUnknownProtection {record_id,..}=action {
            let record=self.records.iter().find(|record|deletion_id(record).as_ref()==Ok(record_id))
                .ok_or("project_backup_delete_management_record_missing")?;
            if record.phase!=Phase::Prepared {return Err("project_backup_delete_management_not_unresolved".into());}
            return Ok(Some(record.receipt.request.backup_id));
        }
        Ok(None)
    }
    fn retire(&mut self, kind:RetiredKind, origin:&str, process:u64, id:u64)->Result<(),String> {
        if let Some(record)=self.retired.iter_mut().find(|r|r.kind==kind&&r.origin==origin&&r.process_incarnation==process) {
            record.high_water_request_id=record.high_water_request_id.max(id);
        }else {
            if self.retired.len()>=MAX_RETIRED {return Err("project_backup_delete_journal_retired_capacity; compact confirmed retired processes explicitly".into());}
            self.retired.push(Retired {kind,origin:origin.into(),process_incarnation:process,high_water_request_id:id});
        }
        Ok(())
    }
    fn acknowledge_records(&mut self, record_ids:&[String])->Result<(),String> {
                for id in record_ids {
                    if let Some(index)=self.records.iter().position(|r|deletion_id(r).as_ref()==Ok(id)) {
                        if self.records[index].phase==Phase::Prepared {return Err("project_backup_delete_management_unresolved_cannot_acknowledge".into());}
                        let record=self.records.remove(index);
                        self.retire(RetiredKind::Deletion,&record.origin,record.receipt.request.expected_fence.process_incarnation,record.receipt.request.request_id)?;
                    }else if let Some(index)=self.management.iter().position(|r|management_id(r).as_ref()==Ok(id)) {
                        let record=self.management.remove(index);
                        self.retire(RetiredKind::Management,&record.origin,record.receipt.request.expected_fence.process_incarnation,record.receipt.request.request_id)?;
                    }else {return Err("project_backup_delete_management_record_missing".into());}
                }
        Ok(())
    }
    fn compact_retired(&mut self, record_ids:&[String], current_process:u64)->Result<(),String> {
                for id in record_ids {
                    let index=self.retired.iter().position(|r|retired_id(r).as_ref()==Ok(id))
                        .ok_or("project_backup_delete_management_record_missing")?;
                    if self.retired[index].process_incarnation==current_process {
                        return Err("project_backup_delete_management_process_still_current".into());
                    }
                    self.retired.remove(index);
                }
        Ok(())
    }
    pub(super) fn managed(&self, origin:&str, request:&ManagementRequestV1,
        observed:Option<ArtifactObservationV1>, current_process:u64)->Result<(Self,ManagementReceiptV1),String> {
        request.validate().map_err(str::to_string)?;self.reserve_generations(1)?;
        if request.expected_fence.process_incarnation!=current_process {
            return Err("project_backup_delete_management_process_mismatch".into());
        }
        if self.management_status(origin,request)?.is_some() {return Err("project_backup_delete_management_identity_already_recorded".into());}
        if request.expected_generation!=self.generation {return Err("project_backup_delete_management_generation_changed".into());}
        let mut next=self.clone();
        let ids=match &request.action {
            ManagementActionV1::Acknowledge {record_ids}=>{
                next.acknowledge_records(record_ids)?;
                if observed.is_some() {return Err("project_backup_delete_management_unexpected_observation".into());}
                record_ids.clone()
            }
            ManagementActionV1::ReleaseUnknownProtection {record_id,expected_artifact}=>{
                if observed.as_ref()!=Some(expected_artifact) {return Err("project_backup_delete_management_artifact_changed".into());}
                self.release_backup_id(&request.action)?;
                let record=next.records.iter_mut().find(|r|deletion_id(r).as_ref()==Ok(record_id))
                    .ok_or("project_backup_delete_management_record_missing")?;
                record.phase=Phase::ResolvedUnknown;
                record.error=Some("Original deletion outcome remains unknown; operator preserved the observed current artifact and released its protection; do not repeat the original mutation".into());
                vec![record_id.clone()]
            }
            ManagementActionV1::CompactRetired {record_ids}=>{
                next.compact_retired(record_ids,current_process)?;
                if observed.is_some() {return Err("project_backup_delete_management_unexpected_observation".into());}
                record_ids.clone()
            }
            ManagementActionV1::Reclaim {acknowledge_record_ids,compact_retired_record_ids}=>{
                // Compact first inside the unpublished clone to make room for
                // exact acknowledgement fences. This also makes room for the
                // new result when both bounded tables were saturated.
                next.compact_retired(compact_retired_record_ids,current_process)?;
                next.acknowledge_records(acknowledge_record_ids)?;
                if observed.is_some() {return Err("project_backup_delete_management_unexpected_observation".into());}
                let mut ids=acknowledge_record_ids.clone();ids.extend(compact_retired_record_ids.iter().cloned());ids
            }
        };
        next.advance()?;
        let receipt=ManagementReceiptV1 {schema_version:1,request:request.clone(),generation_before:self.generation,
            generation_after:next.generation,storage_version_before:self.version,affected_record_ids:ids,observed_artifact:observed};
        receipt.validate().map_err(str::to_string)?;
        if next.management.len()>=MAX_MANAGEMENT {return Err("project_backup_delete_journal_management_capacity; acknowledge prior management results explicitly".into());}
        next.management.push(Management {origin:origin.into(),shape_sha256:fingerprint(request)?,receipt:receipt.clone()});
        next.validate()?;Ok((next,receipt))
    }
    pub(super) fn view(&self, request:&JournalQueryRequestV1, sha256:&str, current_process:u64)->Result<JournalViewV1,String> {
        request.validate().map_err(str::to_string)?;
        if request.expected_journal_sha256.as_ref().is_some_and(|value|value!=sha256) {
            return Err("project_backup_delete_management_snapshot_changed".into());
        }
        let mut records=Vec::new();
        for r in &self.records {records.push(JournalRecordSummaryV1 {record_id:deletion_id(r)?,kind:JournalRecordKindV1::Deletion,
            phase:match r.phase {Phase::Prepared=>"prepared",Phase::Succeeded=>"succeeded",Phase::Rejected=>"rejected",Phase::ResolvedUnknown=>"resolved_unknown"}.into(),
            request_id:r.receipt.request.request_id,process_incarnation:r.receipt.request.expected_fence.process_incarnation,
            backup_id:Some(r.receipt.request.backup_id),artifact_sha256:Some(r.receipt.request.expected_artifact_sha256.clone())});}
        for r in &self.management {records.push(JournalRecordSummaryV1 {record_id:management_id(r)?,kind:JournalRecordKindV1::Management,
            phase:"succeeded".into(),request_id:r.receipt.request.request_id,process_incarnation:r.receipt.request.expected_fence.process_incarnation,
            backup_id:None,artifact_sha256:None});}
        for r in &self.retired {records.push(JournalRecordSummaryV1 {record_id:retired_id(r)?,kind:JournalRecordKindV1::Retired,
            phase:"receipt_expired".into(),request_id:r.high_water_request_id,process_incarnation:r.process_incarnation,backup_id:None,artifact_sha256:None});}
        records.sort_by(|a,b|a.record_id.cmp(&b.record_id));
        if let Some(cursor)=&request.after_record_id {records.retain(|r|r.record_id>*cursor);}
        let more=records.len()>request.limit as usize;records.truncate(request.limit as usize);
        let next_after_record_id=more.then(||records.last().expect("Nonzero query limit").record_id.clone());
        let value=JournalViewV1 {schema_version:1,storage_schema_version:self.version,current_process_incarnation:current_process,
            generation:self.generation,journal_sha256:sha256.into(),deletion_count:self.records.len() as u16,
            management_count:self.management.len() as u16,retired_count:self.retired.len() as u16,records,next_after_record_id};
        value.validate().map_err(str::to_string)?;Ok(value)
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
    Ok(observe(path)?.journal)
}
pub(super) fn observe(path:&Path)->Result<Observation,String> {
    if !exists_regular(path)? {return Ok(Observation {journal:Journal::default(),sha256:format!("{:x}",Sha256::digest(b"absent-backup-deletion-journal-v1"))});}
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
    let journal=match value.get("version").and_then(serde_json::Value::as_u64) {
        Some(1)=>{let legacy:LegacyJournal=serde_json::from_value(value).map_err(|e|format!("project_backup_delete_journal_invalid: {e}"))?;
            Journal {version:legacy.version,records:legacy.records,..Journal::default()}},
        Some(2)=>serde_json::from_value(value).map_err(|e|format!("project_backup_delete_journal_invalid: {e}"))?,
        _=>return Err("project_backup_delete_journal_version_unsupported".into()),
    };
    journal.validate()?;Ok(Observation {journal,sha256:format!("{:x}",Sha256::digest(&bytes))})
}

// Callers hold the existing project publication lock. No background writer,
// implicit eviction, clock-driven retry or reconstruction of corrupt state.
pub(super) fn persist(path: &Path, journal: &Journal) -> Result<(), String> {
    journal.validate()?;
    if journal.version!=2 {return Err("project_backup_delete_journal_migration_requires_explicit_mutation".into());}
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
        let mut invalids=vec![b"{\"version\":3,\"records\":[]}".to_vec(),b"{\"version\":1,\"version\":1,\"records\":[]}".to_vec(),vec![255,123]];
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

    fn management(journal:&Journal,id:u64,action:ManagementActionV1)->ManagementRequestV1 {
        ManagementRequestV1 {schema_version:1,operation_id:protocol::control_plane_backup_management::MANAGE_ID.into(),
            request_id:id,expected_fence:receipt(1).request.expected_fence,expected_generation:journal.generation,
            expected_journal_sha256:"e".repeat(64),action}
    }
    #[test]
    fn project_backup_deletion_management_legacy_read_is_pure_and_explicit_migration_preserves_facts() {
        let root=directory();let path=root.join(FILE_NAME);let actor=origin("one","owner").unwrap();
        let mut journal=Journal::default();let index=journal.prepare(&actor,receipt(1)).unwrap();journal.succeeded(index).unwrap();
        let legacy=serde_json::to_vec(&serde_json::json!({"version":1,"records":journal.records})).unwrap();
        fs::write(&path,&legacy).unwrap();let loaded=observe(&path).unwrap();
        assert_eq!(loaded.journal.version,1);assert_eq!(loaded.journal.generation,0);
        assert_eq!(loaded.sha256,format!("{:x}",Sha256::digest(&legacy)));
        assert_eq!(fs::read(&path).unwrap(),legacy);assert!(persist(&path,&loaded.journal).is_err());
        let request=management(&loaded.journal,1,ManagementActionV1::Acknowledge {record_ids:vec![deletion_id(&loaded.journal.records[0]).unwrap()]});
        let (next,result)=loaded.journal.managed(&actor,&request,None,1).unwrap();persist(&path,&next).unwrap();
        assert_eq!(result.storage_version_before,1);assert_eq!(result.generation_after,1);
        assert_eq!(load(&path).unwrap().version,2);
        assert!(load(&path).unwrap().status(&actor,&receipt(1).request).unwrap_err().contains("receipt_expired"));
        let mut invalid:serde_json::Value=serde_json::from_slice(&legacy).unwrap();invalid["generation"]=0.into();
        let invalid=serde_json::to_vec(&invalid).unwrap();fs::write(&path,&invalid).unwrap();
        assert!(load(&path).is_err());assert_eq!(fs::read(&path).unwrap(),invalid);
        fs::remove_file(path).unwrap();fs::remove_dir(root).unwrap();
    }
    #[test]
    fn project_backup_deletion_management_unknown_release_is_observational_and_ack_batches_are_atomic() {
        let actor=origin("one","owner").unwrap();let mut journal=Journal::default();
        journal.prepare(&actor,receipt(1)).unwrap();let record_id=deletion_id(&journal.records[0]).unwrap();
        let before=serde_json::to_value(&journal).unwrap();
        let ack=management(&journal,1,ManagementActionV1::Acknowledge {record_ids:vec![record_id.clone()]});
        assert!(journal.managed(&actor,&ack,None,1).unwrap_err().contains("unresolved_cannot_acknowledge"));
        let request=management(&journal,2,ManagementActionV1::ReleaseUnknownProtection {record_id,
            expected_artifact:ArtifactObservationV1::Present {sha256:"c".repeat(64)}});
        assert!(journal.managed(&actor,&request,Some(ArtifactObservationV1::Missing {}),1).unwrap_err().contains("artifact_changed"));
        assert_eq!(serde_json::to_value(&journal).unwrap(),before);
        let (next,result)=journal.managed(&actor,&request,Some(ArtifactObservationV1::Present {sha256:"c".repeat(64)}),1).unwrap();
        assert_eq!(next.status(&actor,&receipt(1).request).unwrap().phase,StatusPhase::Indeterminate);
        assert!(next.ensure_unresolved_backup_absent(1).is_ok());
        assert_eq!(serde_json::to_value(next.management_status(&actor,&request).unwrap().unwrap()).unwrap(),serde_json::to_value(result).unwrap());
        let ids=vec![deletion_id(&next.records[0]).unwrap(),"f".repeat(64)];
        assert!(next.managed(&actor,&management(&next,3,ManagementActionV1::Acknowledge {record_ids:ids}),None,1).is_err());
        assert_eq!(next.records.len(),1);assert!(next.retired.is_empty());
        let ids=vec![deletion_id(&next.records[0]).unwrap(),management_id(&next.management[0]).unwrap()];
        let (retired,_)=next.managed(&actor,&management(&next,4,ManagementActionV1::Acknowledge {record_ids:ids}),None,1).unwrap();
        assert!(retired.status(&actor,&receipt(1).request).unwrap_err().contains("receipt_expired"));
        assert!(retired.management_status(&actor,&request).unwrap_err().contains("receipt_expired"));
        assert_eq!(retired.records.len(),0);assert_eq!(retired.management.len(),1);assert_eq!(retired.retired.len(),2);
        let mut new_intent=retired.clone();assert!(new_intent.prepare(&actor,receipt(2)).is_ok());
    }
    #[test]
    fn project_backup_deletion_management_caps_generation_and_process_retirement_are_explicit() {
        let actor=origin("one","owner").unwrap();let mut journal=Journal::default();
        let index=journal.prepare(&actor,receipt(1)).unwrap();journal.succeeded(index).unwrap();
        let ack=management(&journal,1,ManagementActionV1::Acknowledge {record_ids:vec![deletion_id(&journal.records[0]).unwrap()]});
        let (mut next,_)=journal.managed(&actor,&ack,None,1).unwrap();
        let id=retired_id(&next.retired[0]).unwrap();
        let compact=management(&next,2,ManagementActionV1::CompactRetired {record_ids:vec![id]});
        assert!(next.managed(&actor,&compact,None,1).unwrap_err().contains("process_still_current"));
        next.retired[0].process_incarnation=2;let id=retired_id(&next.retired[0]).unwrap();
        let (compacted,_)=next.managed(&actor,&management(&next,3,ManagementActionV1::CompactRetired {record_ids:vec![id]}),None,1).unwrap();
        assert!(compacted.retired.is_empty());assert_eq!(compacted.records.len(),0);
        next.generation=MAX-1;assert!(next.prepare(&actor,receipt(2)).unwrap_err().contains("generation_exhausted"));
        assert_eq!(next.records.len(),0);next.generation=MAX;
        assert!(next.managed(&actor,&management(&next,4,ManagementActionV1::Acknowledge {record_ids:vec![management_id(&next.management[0]).unwrap()]}),None,1).unwrap_err().contains("generation_exhausted"));
        // Saturation is observable and recoverable only through explicit
        // acknowledgement/compaction; errors never mutate the original journal.
        let mut capacity=Journal::default();
        for id in 1..=MAX_RECORDS as u64 {let index=capacity.prepare(&actor,receipt(id)).unwrap();capacity.succeeded(index).unwrap();}
        let request=management(&capacity,1,ManagementActionV1::Acknowledge {record_ids:vec![deletion_id(&capacity.records[0]).unwrap()]});
        let (mut recovered,_)=capacity.managed(&actor,&request,None,1).unwrap();
        assert!(recovered.prepare(&actor,receipt(MAX_RECORDS as u64+1)).is_ok());
        let sample=recovered.management[0].clone();recovered.management.clear();
        for id in 1..=MAX_MANAGEMENT as u64 {let mut row=sample.clone();row.receipt.request.request_id=id;
            row.shape_sha256=fingerprint(&row.receipt.request).unwrap();recovered.management.push(row);}
        let request=management(&recovered,129,ManagementActionV1::Acknowledge {record_ids:vec![deletion_id(&recovered.records[0]).unwrap()]});
        assert!(recovered.managed(&actor,&request,None,1).unwrap_err().contains("management_capacity"));
        let request=management(&recovered,130,ManagementActionV1::Acknowledge {record_ids:vec![management_id(&recovered.management[0]).unwrap()]});
        assert_eq!(recovered.managed(&actor,&request,None,1).unwrap().0.management.len(),MAX_MANAGEMENT);
        let mut capped=Journal::default();
        for id in 1..=MAX_RETIRED {capped.retire(RetiredKind::Deletion,&format!("{id:064x}"),1,1).unwrap();}
        assert!(capped.retire(RetiredKind::Deletion,&"f".repeat(64),1,1).unwrap_err().contains("retired_capacity"));
        assert_eq!(capped.retired.len(),MAX_RETIRED);
    }
    #[test]
    fn project_backup_deletion_management_query_pages_are_redacted_bounded_and_snapshot_consistent() {
        let actor=origin("one","owner").unwrap();let mut journal=Journal::default();
        for id in 1..=33 {let index=journal.prepare(&actor,receipt(id)).unwrap();journal.succeeded(index).unwrap();}
        let mut query=JournalQueryRequestV1 {schema_version:1,limit:16,after_record_id:None,expected_journal_sha256:Some("a".repeat(64))};
        let before=serde_json::to_value(&journal).unwrap();let mut ids=Vec::new();
        loop {let view=journal.view(&query,&"a".repeat(64),1).unwrap();assert!(view.records.len()<=16);
            let value=serde_json::to_value(&view).unwrap();for row in value["records"].as_array().unwrap(){
                for field in ["path","origin","owner","principal","receipt"] {assert!(row.get(field).is_none());}}
            ids.extend(view.records.iter().map(|row|row.record_id.clone()));
            query.after_record_id=view.next_after_record_id;if query.after_record_id.is_none(){break;}}
        assert_eq!(ids.len(),33);assert!(ids.windows(2).all(|pair|pair[0]<pair[1]));
        assert!(journal.view(&query,&"b".repeat(64),1).unwrap_err().contains("snapshot_changed"));
        assert_eq!(serde_json::to_value(journal).unwrap(),before);
    }
    #[test]
    fn project_backup_deletion_management_atomic_reclaim_recovers_simultaneously_full_tables_without_partial_changes() {
        let actor=origin("one","owner").unwrap();let mut seed=Journal::default();
        let index=seed.prepare(&actor,receipt(1)).unwrap();seed.succeeded(index).unwrap();
        let request=management(&seed,1,ManagementActionV1::Acknowledge {record_ids:vec![deletion_id(&seed.records[0]).unwrap()]});
        let sample=seed.managed(&actor,&request,None,1).unwrap().0.management.remove(0);
        let mut capped=Journal::default();capped.generation=2000;
        for id in 1..=MAX_MANAGEMENT as u64 {
            let mut row=sample.clone();row.origin=format!("{:064x}",MAX_RETIRED as u64+id);
            row.receipt.request.request_id=id;row.receipt.request.expected_fence.process_incarnation=2;
            row.shape_sha256=fingerprint(&row.receipt.request).unwrap();capped.management.push(row);
        }
        for id in 1..=MAX_RETIRED {capped.retire(RetiredKind::Deletion,&format!("{id:064x}"),2,1).unwrap();}
        capped.validate().unwrap();let before=serde_json::to_value(&capped).unwrap();
        let ack=management_id(&capped.management[0]).unwrap();let compact=retired_id(&capped.retired[0]).unwrap();
        assert!(capped.managed(&actor,&management(&capped,129,ManagementActionV1::Acknowledge {record_ids:vec![ack.clone()]}),None,1)
            .unwrap_err().contains("retired_capacity"));
        assert!(capped.managed(&actor,&management(&capped,130,ManagementActionV1::CompactRetired {record_ids:vec![compact.clone()]}),None,1)
            .unwrap_err().contains("management_capacity"));
        assert_eq!(serde_json::to_value(&capped).unwrap(),before);
        let action=ManagementActionV1::Reclaim {acknowledge_record_ids:vec![ack.clone()],compact_retired_record_ids:vec![compact.clone()]};
        let request=management(&capped,131,action);
        let (recovered,result)=capped.managed(&actor,&request,None,1).unwrap();
        assert_eq!(recovered.management.len(),MAX_MANAGEMENT);assert_eq!(recovered.retired.len(),MAX_RETIRED);
        assert_eq!(result.affected_record_ids,vec![ack,compact]);assert_eq!(result.generation_after,2001);
        assert!(recovered.management_status(&capped.management[0].origin,&capped.management[0].receipt.request)
            .unwrap_err().contains("receipt_expired"));
        for row in &capped.management[1..] {assert!(recovered.management.iter().any(|value|management_id(value).unwrap()==management_id(row).unwrap()));}
        let mut current=capped.clone();current.retired[0].process_incarnation=1;
        let request=management(&current,132,ManagementActionV1::Reclaim {acknowledge_record_ids:vec![management_id(&current.management[0]).unwrap()],
            compact_retired_record_ids:vec![retired_id(&current.retired[0]).unwrap()]});
        assert!(current.managed(&actor,&request,None,1).unwrap_err().contains("process_still_current"));
        assert_eq!(current.management.len(),MAX_MANAGEMENT);assert_eq!(current.retired.len(),MAX_RETIRED);
    }
}
