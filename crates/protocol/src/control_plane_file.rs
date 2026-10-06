//! Typed, owner-free project file publication requests.
use super::control_plane_command::{ProjectMutationFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER};
use serde::{Deserialize, Serialize};

pub const SAVE_ID: &str = "syndocal.project.save.v1";
pub const SAVE_AS_ID: &str = "syndocal.project.save_as.v1";
pub const TEMPLATE_ID: &str = "syndocal.project.template.save.v1";
pub const BACKUP_ID: &str = "syndocal.project.backup.create.v1";
pub const BACKUP_AUTHORITY_ID: &str = "syndocal.query.project.backup.authority.v1";
pub const BACKUP_INSPECT_ID: &str = "syndocal.query.project.backup.inspect.v1";
pub const BACKUP_LIST_ID: &str = "syndocal.query.project.backup.list.v1";
pub const BACKUP_LIST_MAX_ITEMS: usize = 16;
pub const BACKUP_DELETE_ID: &str = "syndocal.project.backup.delete.v1";
pub const BACKUP_DELETE_STATUS_ID: &str = "syndocal.query.project.backup.delete.status.v1";
pub const AUTHORITY_ID: &str = "syndocal.query.project.file.authority.v1";
pub const STATUS_ID: &str = "syndocal.query.project.file.status.v1";
pub const ACK_ID: &str = "syndocal.project.file.acknowledge.v1";

pub fn publication_id(id: &str) -> bool {
    matches!(id, SAVE_ID | SAVE_AS_ID | TEMPLATE_ID | BACKUP_ID)
}
pub fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn valid_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 4096
        && path == path.trim()
        && !path.chars().any(char::is_control)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFileAuthorityRequestV1 {
    pub schema_version: u16,
    pub operation_id: String,
    pub destination: String,
}
impl ProjectFileAuthorityRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || !publication_id(&self.operation_id)
            || !valid_path(&self.destination)
        {
            return Err("invalid project file authority request");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupAuthorityRequestV1 {
    pub schema_version: u16,
}
impl ProjectBackupAuthorityRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 {
            return Err("invalid project backup authority schema");
        }
        Ok(())
    }
}

/// A managed ID, never a caller-selected filesystem path or owner identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupInspectRequestV1 {
    pub schema_version: u16,
    pub backup_id: u64,
}
impl ProjectBackupInspectRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1
            || self.backup_id == 0
            || self.backup_id > MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err("invalid project backup inspection request");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupInspectionV1 {
    pub schema_version: u16,
    pub backup: ProjectFileBackupSummaryV1,
    /// Exact bytes consumed by the same parse; not a later reopening of the file.
    pub artifact_sha256: String,
    /// Existing restore policy keeps only .sdc source paths. This is metadata,
    /// not a claim that the source file exists or is authorized for writing.
    pub restore_source_path: Option<String>,
}
impl ProjectBackupInspectionV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        ProjectBackupInspectRequestV1 {
            schema_version: self.schema_version,
            backup_id: self.backup.id,
        }
        .validate()?;
        if self.backup.created_at_unix_ms > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.backup.bytes == 0
            || self.backup.bytes > 128 * 1024 * 1024
            || self.backup.reason.len() > 4096
            || self
                .backup
                .source_path
                .as_ref()
                .is_some_and(|path| !valid_path(path))
            || self.restore_source_path.as_ref().is_some_and(|path| {
                !valid_path(path) || self.backup.source_path.as_ref() != Some(path)
            })
            || !valid_hash(&self.artifact_sha256)
        {
            return Err("invalid project backup inspection metadata");
        }
        Ok(())
    }
}

/// Delete exactly one observed managed artifact. No caller supplies a path,
/// owner, principal or confirmation flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupDeleteRequestV1 {
    pub schema_version: u16,
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: ProjectMutationFenceV1,
    pub backup_id: u64,
    pub expected_artifact_sha256: String,
}
impl ProjectBackupDeleteRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.operation_id != BACKUP_DELETE_ID
            || self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER
            || self.backup_id == 0 || self.backup_id > MAX_SAFE_JAVASCRIPT_INTEGER
            || !valid_hash(&self.expected_artifact_sha256)
        { return Err("invalid project backup delete request"); }
        self.expected_fence.validate().map_err(|_| "invalid project backup delete fence")
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupDeleteReceiptV1 {
    pub schema_version: u16,
    pub request: ProjectBackupDeleteRequestV1,
    pub deleted_backup: ProjectBackupInspectionV1,
}
impl ProjectBackupDeleteReceiptV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.request.validate()?;
        self.deleted_backup.validate()?;
        if self.schema_version != 1 || self.deleted_backup.backup.id != self.request.backup_id
            || self.deleted_backup.artifact_sha256 != self.request.expected_artifact_sha256
        { return Err("invalid project backup delete receipt"); }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectBackupDeletePhaseV1 { Unknown, Indeterminate, Succeeded, Rejected }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupDeleteStatusV1 {
    pub schema_version: u16,
    pub request: ProjectBackupDeleteRequestV1,
    pub phase: ProjectBackupDeletePhaseV1,
    pub receipt: Option<ProjectBackupDeleteReceiptV1>,
    pub error: Option<String>,
}
impl ProjectBackupDeleteStatusV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.request.validate()?;
        let succeeded=self.phase==ProjectBackupDeletePhaseV1::Succeeded;
        let error=self.phase==ProjectBackupDeletePhaseV1::Rejected||self.phase==ProjectBackupDeletePhaseV1::Indeterminate;
        if self.schema_version!=1 || succeeded!=self.receipt.is_some() || error!=self.error.is_some()
            || self.error.as_ref().is_some_and(|e|e.is_empty()||e.len()>1024) {
            return Err("invalid project backup delete status");
        }
        if let Some(receipt)=&self.receipt {
            receipt.validate()?;
            if receipt.request!=self.request { return Err("project backup delete status request mismatch"); }
        }
        Ok(())
    }
}

/// Descending managed IDs. Each page is a fresh observation, not a snapshot
/// reservation; subsequent inspect/restore must bind its own expected digest.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupListRequestV1 {
    pub schema_version: u16,
    pub limit: u16,
    pub before_id: Option<u64>,
}
impl ProjectBackupListRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.limit == 0
            || self.limit as usize > BACKUP_LIST_MAX_ITEMS
            || self.before_id.is_some_and(|id| id == 0 || id > MAX_SAFE_JAVASCRIPT_INTEGER)
        { return Err("invalid project backup list request"); }
        Ok(())
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectBackupListV1 {
    pub schema_version: u16,
    pub backups: Vec<ProjectBackupInspectionV1>,
    pub next_before_id: Option<u64>,
}
impl ProjectBackupListV1 {
    pub fn validate(&self, request: &ProjectBackupListRequestV1) -> Result<(), &'static str> {
        request.validate()?;
        if self.schema_version != 1 || self.backups.len() > request.limit as usize
            || self.next_before_id.is_some_and(|id| self.backups.len() != request.limit as usize
                || self.backups.last().map(|row| row.backup.id) != Some(id))
        { return Err("invalid project backup list response"); }
        let mut prior = request.before_id.unwrap_or(MAX_SAFE_JAVASCRIPT_INTEGER + 1);
        for row in &self.backups {
            row.validate()?;
            if row.backup.id >= prior { return Err("invalid project backup list order"); }
            prior = row.backup.id;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFileRequestV1 {
    pub schema_version: u16,
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: ProjectMutationFenceV1,
    pub expected_path_generation: u64,
    pub expected_disposition_generation: u64,
    pub destination: String,
    pub expected_target_sha256: Option<String>,
}
impl ProjectFileRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        ProjectFileAuthorityRequestV1 {
            schema_version: self.schema_version,
            operation_id: self.operation_id.clone(),
            destination: self.destination.clone(),
        }
        .validate()?;
        self.expected_fence
            .validate()
            .map_err(|_| "invalid project mutation fence")?;
        if self.request_id == 0
            || [
                self.request_id,
                self.expected_path_generation,
                self.expected_disposition_generation,
            ]
            .iter()
            .any(|value| *value > MAX_SAFE_JAVASCRIPT_INTEGER)
            || self
                .expected_target_sha256
                .as_ref()
                .is_some_and(|value| !valid_hash(value))
            || (self.operation_id == BACKUP_ID && self.expected_target_sha256.is_some())
        {
            return Err("invalid project file request identity or target hash");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFileAuthorityV1 {
    pub schema_version: u16,
    pub fence: ProjectMutationFenceV1,
    pub path_generation: u64,
    pub disposition_generation: u64,
    pub next_request_id: u64,
    pub current_project_path: Option<String>,
    pub destination: String,
    pub target_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectFilePhaseV1 {
    Reserved,
    Selecting,
    Selected,
    Prepared,
    Succeeded,
    Cancelled,
    Abandoned,
    Failed,
    Indeterminate,
    Acknowledged,
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFileStatusV1 {
    pub schema_version: u16,
    pub request: ProjectFileRequestV1,
    pub phase: ProjectFilePhaseV1,
    pub target_path: Option<String>,
    pub artifact_sha256: Option<String>,
    pub recovery_authority_serial: u64,
    // Terminal E/R/H is recorded in the existing durable publication journal.
    // Never add a later live authority bundle to an exact terminal receipt.
    pub saved_project_epoch: u64,
    pub saved_project_revision: u64,
    pub saved_checkpoint_hash: String,
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup: Option<ProjectFileBackupSummaryV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectFileBackupSummaryV1 {
    pub id: u64,
    pub created_at_unix_ms: u64,
    pub source_path: Option<String>,
    pub reason: String,
    pub bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn backup_delete_status_phase_payload_and_request_are_exact() {
        let request=ProjectBackupDeleteRequestV1 {schema_version:1,operation_id:BACKUP_DELETE_ID.into(),request_id:1,
            expected_fence:ProjectMutationFenceV1 {process_incarnation:1,session_incarnation:2,project_epoch:0,
                project_revision:0,project_checkpoint_hash:"a".repeat(64),project_publication_generation:0},
            backup_id:1,expected_artifact_sha256:"c".repeat(64)};
        let mut status=ProjectBackupDeleteStatusV1 {schema_version:1,request,phase:ProjectBackupDeletePhaseV1::Unknown,
            receipt:None,error:None};assert!(status.validate().is_ok());
        status.phase=ProjectBackupDeletePhaseV1::Indeterminate;assert!(status.validate().is_err());
        status.error=Some("Preserve unresolved artifact".into());assert!(status.validate().is_ok());
        status.phase=ProjectBackupDeletePhaseV1::Rejected;assert!(status.validate().is_ok());
        status.phase=ProjectBackupDeletePhaseV1::Succeeded;assert!(status.validate().is_err());
        status.phase=ProjectBackupDeletePhaseV1::Unknown;status.error=None;
        let mut wire=serde_json::to_value(&status).unwrap();wire["owner"]="forged".into();
        assert!(serde_json::from_value::<ProjectBackupDeleteStatusV1>(wire).is_err());
        status.schema_version=2;assert!(status.validate().is_err());
    }
    #[test]
    fn project_backup_delete_strict_identity_hash_fence_and_receipt() {
        let request = ProjectBackupDeleteRequestV1 { schema_version:1, operation_id:BACKUP_DELETE_ID.into(),
            request_id:1, backup_id:2, expected_artifact_sha256:"a".repeat(64),
            expected_fence:ProjectMutationFenceV1 {process_incarnation:1,session_incarnation:2,
                project_epoch:3,project_revision:4,project_checkpoint_hash:"b".repeat(64),project_publication_generation:5} };
        assert!(request.validate().is_ok());
        for invalid in [ProjectBackupDeleteRequestV1 {schema_version:2,..request.clone()},
            ProjectBackupDeleteRequestV1 {operation_id:BACKUP_ID.into(),..request.clone()},
            ProjectBackupDeleteRequestV1 {request_id:0,..request.clone()},
            ProjectBackupDeleteRequestV1 {backup_id:0,..request.clone()},
            ProjectBackupDeleteRequestV1 {backup_id:MAX_SAFE_JAVASCRIPT_INTEGER+1,..request.clone()},
            ProjectBackupDeleteRequestV1 {expected_artifact_sha256:"A".repeat(64),..request.clone()}] {
            assert!(invalid.validate().is_err());
        }
        for field in ["path","owner_id","principal","skip_confirmation"] {
            let mut json=serde_json::to_value(&request).unwrap();json[field]=serde_json::json!(true);
            assert!(serde_json::from_value::<ProjectBackupDeleteRequestV1>(json).is_err());
        }
        let row=ProjectBackupInspectionV1 {schema_version:1,backup:ProjectFileBackupSummaryV1 {id:2,
            created_at_unix_ms:2,source_path:None,reason:"manual".into(),bytes:2},
            artifact_sha256:"a".repeat(64),restore_source_path:None};
        let receipt=ProjectBackupDeleteReceiptV1 {schema_version:1,request:request.clone(),deleted_backup:row.clone()};
        assert!(receipt.validate().is_ok());
        let mut wrong=receipt.clone();wrong.deleted_backup.backup.id=3;assert!(wrong.validate().is_err());
        let mut wrong=receipt;wrong.deleted_backup.artifact_sha256="c".repeat(64);assert!(wrong.validate().is_err());
    }
    #[test]
    fn project_backup_list_strict_request_and_cursor_bounds() {
        let request = ProjectBackupListRequestV1 { schema_version: 1, limit: 16, before_id: None };
        assert!(request.validate().is_ok());
        for invalid in [ProjectBackupListRequestV1 {schema_version:2,..request.clone()},
            ProjectBackupListRequestV1 {limit:0,..request.clone()}, ProjectBackupListRequestV1 {limit:17,..request.clone()},
            ProjectBackupListRequestV1 {before_id:Some(0),..request.clone()},
            ProjectBackupListRequestV1 {before_id:Some(MAX_SAFE_JAVASCRIPT_INTEGER+1),..request.clone()}] {
            assert!(invalid.validate().is_err());
        }
        for forged in ["path","owner_id","principal","skip_confirmation"] {
            let mut value=serde_json::to_value(&request).unwrap(); value[forged]=serde_json::json!("forged");
            assert!(serde_json::from_value::<ProjectBackupListRequestV1>(value).is_err());
        }
    }
    #[test]
    fn project_backup_list_response_rejects_duplicate_order_identity_and_cursor() {
        let row=|id| ProjectBackupInspectionV1 {schema_version:1,backup:ProjectFileBackupSummaryV1 {
            id,created_at_unix_ms:id,source_path:None,reason:"manual".into(),bytes:1,
        },artifact_sha256:"a".repeat(64),restore_source_path:None};
        let request=ProjectBackupListRequestV1 {schema_version:1,limit:2,before_id:Some(4)};
        let page=ProjectBackupListV1 {schema_version:1,backups:vec![row(3),row(2)],next_before_id:Some(2)};
        assert!(page.validate(&request).is_ok());
        for invalid in [ProjectBackupListV1 {schema_version:2,..page.clone()},
            ProjectBackupListV1 {next_before_id:Some(1),..page.clone()},
            ProjectBackupListV1 {backups:vec![row(2),row(3)],..page.clone()},
            ProjectBackupListV1 {backups:vec![row(2),row(2)],..page.clone()},
            ProjectBackupListV1 {backups:vec![row(4),row(2)],..page.clone()},
            ProjectBackupListV1 {backups:vec![row(3)],..page.clone()},
            ProjectBackupListV1 {backups:vec![row(3),row(2),row(1)],..page.clone()}] {
            assert!(invalid.validate(&request).is_err());
        }
    }
    fn request() -> ProjectFileRequestV1 {
        ProjectFileRequestV1 {
            schema_version: 1,
            operation_id: SAVE_AS_ID.into(),
            request_id: 1,
            expected_fence: ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 1,
                project_epoch: 1,
                project_revision: 0,
                project_checkpoint_hash: "a".repeat(64),
                project_publication_generation: 1,
            },
            expected_path_generation: 0,
            expected_disposition_generation: 0,
            destination: "C:/private/日本語.sdc".into(),
            expected_target_sha256: None,
        }
    }
    #[test]
    fn project_file_request_rejects_owner_origin_confirmation_and_unsafe_identity() {
        let valid = request();
        valid.validate().unwrap();
        for key in ["owner_id", "origin_id", "principal", "skip_confirmation"] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[key] = serde_json::json!(true);
            assert!(serde_json::from_value::<ProjectFileRequestV1>(value).is_err());
        }
        for field in [
            "request_id",
            "expected_path_generation",
            "expected_disposition_generation",
        ] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[field] = serde_json::json!(MAX_SAFE_JAVASCRIPT_INTEGER + 1);
            assert!(serde_json::from_value::<ProjectFileRequestV1>(value)
                .unwrap()
                .validate()
                .is_err());
        }
    }
    #[test]
    fn project_file_request_binds_surface_destination_and_existing_or_absent_target() {
        for id in [SAVE_ID, SAVE_AS_ID, TEMPLATE_ID] {
            let mut value = request();
            value.operation_id = id.into();
            value.validate().unwrap();
            value.expected_target_sha256 = Some("b".repeat(64));
            value.validate().unwrap();
            value.expected_target_sha256 = Some("B".repeat(64));
            assert!(value.validate().is_err());
        }
        let mut value = request();
        value.destination.push('\n');
        assert!(value.validate().is_err());
        value = request();
        value.operation_id = ACK_ID.into();
        assert!(value.validate().is_err());
    }
    #[test]
    fn project_file_backup_rejects_replace_hash_and_forged_authority_fields() {
        let mut value = request();
        value.operation_id = BACKUP_ID.into();
        value.destination = "C:/managed/backup-1.json".into();
        value.validate().unwrap();
        value.expected_target_sha256 = Some("b".repeat(64));
        assert!(value.validate().is_err());
        assert!(ProjectBackupAuthorityRequestV1 { schema_version: 2 }
            .validate()
            .is_err());
        for field in ["owner_id", "destination", "skip_confirmation"] {
            let mut query = serde_json::json!({"schema_version":1});
            query[field] = serde_json::json!(true);
            assert!(serde_json::from_value::<ProjectBackupAuthorityRequestV1>(query).is_err());
        }
    }

    #[test]
    fn project_file_backup_inspection_rejects_paths_identity_and_unsafe_metadata() {
        let valid = ProjectBackupInspectRequestV1 {
            schema_version: 1,
            backup_id: 1,
        };
        valid.validate().unwrap();
        for id in [0, MAX_SAFE_JAVASCRIPT_INTEGER + 1] {
            assert!(ProjectBackupInspectRequestV1 {
                backup_id: id,
                ..valid.clone()
            }
            .validate()
            .is_err());
        }
        assert!(ProjectBackupInspectRequestV1 {
            schema_version: 2,
            ..valid.clone()
        }
        .validate()
        .is_err());
        for field in [
            "path",
            "destination",
            "owner_id",
            "principal",
            "skip_confirmation",
        ] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[field] = serde_json::json!("forged");
            assert!(serde_json::from_value::<ProjectBackupInspectRequestV1>(value).is_err());
        }
        let response = ProjectBackupInspectionV1 {
            schema_version: 1,
            backup: ProjectFileBackupSummaryV1 {
                id: 1,
                created_at_unix_ms: 0,
                source_path: Some("C:/公演/日本語.sdc".into()),
                reason: "before update".into(),
                bytes: 100,
            },
            artifact_sha256: "a".repeat(64),
            restore_source_path: Some("C:/公演/日本語.sdc".into()),
        };
        response.validate().unwrap();
        for index in 0..7 {
            let mut invalid = response.clone();
            match index {
                0 => invalid.backup.created_at_unix_ms = MAX_SAFE_JAVASCRIPT_INTEGER + 1,
                1 => invalid.backup.bytes = 128 * 1024 * 1024 + 1,
                2 => invalid.backup.reason = "あ".repeat(1366),
                3 => invalid.backup.source_path = Some("line\nbreak.sdc".into()),
                4 => invalid.restore_source_path = Some("C:/another.sdc".into()),
                5 => invalid.artifact_sha256 = "A".repeat(64),
                _ => invalid.backup.bytes = 0,
            }
            assert!(invalid.validate().is_err(), "accepted case {index}");
        }
    }
}
