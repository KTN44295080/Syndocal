//! Explicit deletion-journal maintenance. Resource hashes are observations,
//! never caller-provided filesystem paths, owners or principal identities.
use serde::{Deserialize, Serialize};
use super::control_plane_command::{ProjectMutationFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER as MAX};
use super::control_plane_file::valid_hash;

pub const QUERY_ID: &str = "syndocal.query.project.backup.delete.journal.v1";
pub const STATUS_ID: &str = "syndocal.query.project.backup.delete.journal.status.v1";
pub const MANAGE_ID: &str = "syndocal.project.backup.delete.journal.manage.v1";
pub const MAX_ITEMS: usize = 16;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ArtifactObservationV1 { Missing {}, Present { sha256: String } }
impl ArtifactObservationV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if matches!(self, Self::Present { sha256 } if !valid_hash(sha256)) {
            return Err("invalid deletion management artifact observation");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ManagementActionV1 {
    Acknowledge { record_ids: Vec<String> },
    ReleaseUnknownProtection { record_id: String, expected_artifact: ArtifactObservationV1 },
    CompactRetired { record_ids: Vec<String> },
    Reclaim { acknowledge_record_ids: Vec<String>, compact_retired_record_ids: Vec<String> },
}
impl ManagementActionV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        match self {
            Self::Acknowledge { record_ids } | Self::CompactRetired { record_ids } => {
                let unique = record_ids.iter().collect::<std::collections::HashSet<_>>();
                if record_ids.is_empty() || record_ids.len() > MAX_ITEMS || unique.len() != record_ids.len()
                    || record_ids.iter().any(|id| !valid_hash(id)) {
                    return Err("invalid deletion management record selection");
                }
            }
            Self::ReleaseUnknownProtection { record_id, expected_artifact } => {
                if !valid_hash(record_id) { return Err("invalid deletion management record selection"); }
                expected_artifact.validate()?;
            }
            Self::Reclaim { acknowledge_record_ids, compact_retired_record_ids } => {
                if acknowledge_record_ids.is_empty() || compact_retired_record_ids.is_empty() {
                    return Err("invalid deletion management reclaim selection");
                }
                let selected=acknowledge_record_ids.iter().chain(compact_retired_record_ids);
                let ids=selected.collect::<Vec<_>>();
                if ids.len()>MAX_ITEMS || ids.iter().any(|id| !valid_hash(id))
                    || ids.iter().collect::<std::collections::HashSet<_>>().len()!=ids.len() {
                    return Err("invalid deletion management reclaim selection");
                }
            }
        }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementRequestV1 {
    pub schema_version: u16,
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: ProjectMutationFenceV1,
    pub expected_generation: u64,
    pub expected_journal_sha256: String,
    pub action: ManagementActionV1,
}
impl ManagementRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.operation_id != MANAGE_ID || self.request_id == 0
            || self.request_id > MAX || self.expected_generation > MAX || !valid_hash(&self.expected_journal_sha256) {
            return Err("invalid deletion management request");
        }
        self.expected_fence.validate().map_err(|_| "invalid deletion management fence")?;
        self.action.validate()
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ManagementReceiptV1 {
    pub schema_version: u16,
    pub request: ManagementRequestV1,
    pub generation_before: u64,
    pub generation_after: u64,
    pub storage_version_before: u16,
    pub affected_record_ids: Vec<String>,
    pub observed_artifact: Option<ArtifactObservationV1>,
}
impl ManagementReceiptV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.request.validate()?;
        if self.schema_version != 1 || self.generation_before != self.request.expected_generation
            || self.generation_after > MAX || self.generation_before.checked_add(1) != Some(self.generation_after)
            || !matches!(self.storage_version_before, 1 | 2)
            || (self.storage_version_before == 1 && self.generation_before != 0) {
            return Err("invalid deletion management receipt");
        }
        let (ids, observation) = match &self.request.action {
            ManagementActionV1::Acknowledge { record_ids } | ManagementActionV1::CompactRetired { record_ids } => (record_ids.clone(), None),
            ManagementActionV1::ReleaseUnknownProtection { record_id, expected_artifact } => (vec![record_id.clone()], Some(expected_artifact)),
            ManagementActionV1::Reclaim { acknowledge_record_ids, compact_retired_record_ids } => {
                let mut ids=acknowledge_record_ids.clone();ids.extend(compact_retired_record_ids.iter().cloned());(ids,None)
            },
        };
        if self.affected_record_ids != ids || self.observed_artifact.as_ref() != observation {
            return Err("deletion management receipt action mismatch");
        }
        if let Some(value) = &self.observed_artifact { value.validate()?; }
        Ok(())
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalQueryRequestV1 {
    pub schema_version: u16,
    pub limit: u16,
    pub after_record_id: Option<String>,
    pub expected_journal_sha256: Option<String>,
}
impl JournalQueryRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || self.limit == 0 || self.limit as usize > MAX_ITEMS
            || self.after_record_id.as_ref().is_some_and(|id| !valid_hash(id))
            || self.expected_journal_sha256.as_ref().is_some_and(|id| !valid_hash(id)) {
            return Err("invalid deletion journal query");
        }
        Ok(())
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JournalRecordKindV1 { Deletion, Management, Retired }
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalRecordSummaryV1 {
    pub record_id: String,
    pub kind: JournalRecordKindV1,
    pub phase: String,
    pub request_id: u64,
    pub process_incarnation: u64,
    pub backup_id: Option<u64>,
    pub artifact_sha256: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalViewV1 {
    pub schema_version: u16,
    pub storage_schema_version: u16,
    pub current_process_incarnation: u64,
    pub generation: u64,
    pub journal_sha256: String,
    pub deletion_count: u16,
    pub management_count: u16,
    pub retired_count: u16,
    pub records: Vec<JournalRecordSummaryV1>,
    pub next_after_record_id: Option<String>,
}
impl JournalViewV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != 1 || !matches!(self.storage_schema_version, 1 | 2)
            || self.current_process_incarnation == 0 || self.current_process_incarnation > MAX
            || self.generation > MAX || !valid_hash(&self.journal_sha256)
            || self.deletion_count > 256 || self.management_count > 128 || self.retired_count > 256
            || self.records.len() > MAX_ITEMS
            || self.records.len() > self.deletion_count as usize + self.management_count as usize + self.retired_count as usize
            || (self.storage_schema_version == 1 && (self.generation != 0 || self.management_count != 0 || self.retired_count != 0))
            || self.records.windows(2).any(|pair| pair[0].record_id >= pair[1].record_id)
            || self.next_after_record_id.as_ref().is_some_and(|id| self.records.last().map(|row| &row.record_id) != Some(id)) {
            return Err("invalid deletion journal view");
        }
        for row in &self.records {
            let deletion = row.kind == JournalRecordKindV1::Deletion;
            let phase_valid = match row.kind {
                JournalRecordKindV1::Deletion => matches!(row.phase.as_str(), "prepared" | "succeeded" | "rejected" | "resolved_unknown"),
                JournalRecordKindV1::Management => row.phase == "succeeded",
                JournalRecordKindV1::Retired => row.phase == "receipt_expired",
            };
            if !valid_hash(&row.record_id) || row.request_id == 0 || row.request_id > MAX
                || row.process_incarnation == 0 || row.process_incarnation > MAX || !phase_valid
                || deletion != row.backup_id.is_some() || deletion != row.artifact_sha256.is_some()
                || row.backup_id.is_some_and(|id| id == 0 || id > MAX)
                || row.artifact_sha256.as_ref().is_some_and(|hash| !valid_hash(hash)) {
                return Err("invalid deletion journal record summary");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn management_actions_are_explicit_bounded_unique_and_owner_free() {
        let id = "a".repeat(64);
        let mut action = ManagementActionV1::Acknowledge { record_ids: vec![id.clone()] };
        assert!(action.validate().is_ok());
        action = ManagementActionV1::Acknowledge { record_ids: vec![id.clone(), id.clone()] };
        assert!(action.validate().is_err());
        assert!(ManagementActionV1::CompactRetired { record_ids: vec![] }.validate().is_err());
        assert!(ManagementActionV1::CompactRetired { record_ids: vec![id.clone(); MAX_ITEMS + 1] }.validate().is_err());
        let mut value = serde_json::to_value(ManagementActionV1::ReleaseUnknownProtection {
            record_id: id, expected_artifact: ArtifactObservationV1::Missing {} }).unwrap();
        value["skip_confirmation"] = true.into();
        assert!(serde_json::from_value::<ManagementActionV1>(value).is_err());
        assert!(serde_json::from_value::<ArtifactObservationV1>(serde_json::json!({"kind":"missing","path":"forged"})).is_err());
        assert!(serde_json::from_value::<ArtifactObservationV1>(serde_json::json!({})).is_err());
        assert!(ArtifactObservationV1::Present { sha256: "A".repeat(64) }.validate().is_err());
        let reclaim=|ack,compact|ManagementActionV1::Reclaim {acknowledge_record_ids:ack,compact_retired_record_ids:compact};
        assert!(reclaim(vec!["a".repeat(64)],vec!["b".repeat(64)]).validate().is_ok());
        assert!(reclaim(vec![],vec!["b".repeat(64)]).validate().is_err());
        assert!(reclaim(vec!["a".repeat(64)],vec!["a".repeat(64)]).validate().is_err());
        assert!(reclaim(vec!["a".repeat(64);16],vec!["b".repeat(64)]).validate().is_err());
    }
    #[test]
    fn journal_query_requires_bounded_hash_cursor_and_snapshot_identity() {
        let mut request = JournalQueryRequestV1 { schema_version: 1, limit: 16,
            after_record_id: None, expected_journal_sha256: None };
        assert!(request.validate().is_ok());request.limit = 17;assert!(request.validate().is_err());
        request.limit = 1;request.expected_journal_sha256 = Some("bad".into());assert!(request.validate().is_err());
        request.expected_journal_sha256 = None;request.schema_version = 2;assert!(request.validate().is_err());
    }
    #[test]
    fn management_receipts_reject_changed_action_generation_and_false_legacy_migration() {
        let request=ManagementRequestV1 {schema_version:1,operation_id:MANAGE_ID.into(),request_id:1,
            expected_fence:ProjectMutationFenceV1 {process_incarnation:1,session_incarnation:2,project_epoch:0,
                project_revision:0,project_checkpoint_hash:"a".repeat(64),project_publication_generation:0},
            expected_generation:0,expected_journal_sha256:"b".repeat(64),
            action:ManagementActionV1::ReleaseUnknownProtection {record_id:"c".repeat(64),expected_artifact:ArtifactObservationV1::Missing {}}};
        let original=ManagementReceiptV1 {schema_version:1,request,generation_before:0,generation_after:1,
            storage_version_before:1,affected_record_ids:vec!["c".repeat(64)],observed_artifact:Some(ArtifactObservationV1::Missing {})};
        assert!(original.validate().is_ok());
        let mut bad=original.clone();bad.affected_record_ids[0]="d".repeat(64);assert!(bad.validate().is_err());
        let mut bad=original.clone();bad.observed_artifact=None;assert!(bad.validate().is_err());
        let mut bad=original.clone();bad.generation_after=2;assert!(bad.validate().is_err());
        let mut bad=original;bad.request.expected_generation=1;bad.generation_before=1;bad.generation_after=2;
        assert!(bad.validate().is_err());bad.storage_version_before=2;assert!(bad.validate().is_ok());
    }
}
