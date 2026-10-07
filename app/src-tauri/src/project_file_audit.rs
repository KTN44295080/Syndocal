//! Borrow typed terminal results; retain no result bodies, paths or raw errors.
use protocol::{control_plane_backup_management as management, control_plane_file as file};

pub(crate) struct Outcome {
    pub code: &'static str,
    pub succeeded: Option<bool>,
    pub generation_before: Option<u64>,
    pub generation_after: Option<u64>,
}
impl Outcome {
    fn new(code: &'static str, succeeded: Option<bool>) -> Self {
        Self { code, succeeded, generation_before: None, generation_after: None }
    }
}
pub(crate) trait AuditResult {
    fn audit_outcome(&self, operation: &str, request_id: u64) -> Result<Outcome, &'static str>;
}
impl AuditResult for file::ProjectFileStatusV1 {
    fn audit_outcome(&self, operation: &str, request_id: u64) -> Result<Outcome, &'static str> {
        self.request.validate()?;
        if self.schema_version != 1 || self.request.request_id != request_id
            || (operation != file::ACK_ID && operation != self.request.operation_id) {
            return Err("diagnostic_file_receipt_identity_invalid");
        }
        use file::ProjectFilePhaseV1::*;
        let (code, succeeded) = match self.phase {
            Succeeded => ("succeeded", Some(true)), Acknowledged => ("acknowledged", Some(true)),
            Cancelled => ("cancelled", Some(false)), Abandoned => ("abandoned", Some(false)),
            Failed => ("failed", Some(false)), Reserved => ("reserved", None),
            Selecting => ("selecting", None), Selected => ("selected", None), Prepared => ("prepared", None),
            Indeterminate => ("indeterminate", None), Missing => ("missing", None),
        };
        Ok(Outcome::new(code, succeeded))
    }
}
impl AuditResult for file::ProjectBackupDeleteReceiptV1 {
    fn audit_outcome(&self, operation: &str, request_id: u64) -> Result<Outcome, &'static str> {
        self.validate()?;
        if operation != file::BACKUP_DELETE_ID || self.request.request_id != request_id {
            return Err("diagnostic_file_receipt_identity_invalid");
        }
        Ok(Outcome::new("backup_deleted", Some(true)))
    }
}
impl AuditResult for management::ManagementReceiptV1 {
    fn audit_outcome(&self, operation: &str, request_id: u64) -> Result<Outcome, &'static str> {
        self.validate()?;
        if operation != management::MANAGE_ID || self.request.request_id != request_id {
            return Err("diagnostic_file_receipt_identity_invalid");
        }
        Ok(Outcome { code: "deletion_journal_managed", succeeded: Some(true),
            generation_before: Some(self.generation_before), generation_after: Some(self.generation_after) })
    }
}
