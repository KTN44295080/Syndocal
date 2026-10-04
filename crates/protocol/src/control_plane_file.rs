//! Typed, owner-free project file publication requests.
use super::control_plane_command::{ProjectMutationFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER};
use serde::{Deserialize, Serialize};

pub const SAVE_ID: &str = "syndocal.project.save.v1";
pub const SAVE_AS_ID: &str = "syndocal.project.save_as.v1";
pub const TEMPLATE_ID: &str = "syndocal.project.template.save.v1";
pub const AUTHORITY_ID: &str = "syndocal.query.project.file.authority.v1";
pub const STATUS_ID: &str = "syndocal.query.project.file.status.v1";
pub const ACK_ID: &str = "syndocal.project.file.acknowledge.v1";

pub fn publication_id(id: &str) -> bool {
    matches!(id, SAVE_ID | SAVE_AS_ID | TEMPLATE_ID)
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
}

#[cfg(test)]
mod tests {
    use super::*;
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
}
