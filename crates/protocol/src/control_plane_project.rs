//! Typed project replacement. Caller identity and consent origin are backend facts.
use serde::{de::Error as _, Deserialize, Deserializer, Serialize, Serializer};

use crate::control_plane_command::{ProjectMutationFenceV1, MAX_SAFE_JAVASCRIPT_INTEGER};

pub const PROJECT_NEW_OPERATION_ID: &str = "syndocal.project.new.v1";
pub const PROJECT_OPEN_OPERATION_ID: &str = "syndocal.project.open.v1";
pub const PROJECT_REPLACEMENT_AUTHORITY_OPERATION_ID: &str =
    "syndocal.query.project.replacement.authority.v1";
pub const PROJECT_REPLACEMENT_SCHEMA_VERSION: u16 = 1;
pub const MAX_PROJECT_OPEN_PATH_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectReplacementAuthorityRequestV1 {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectReplacementAuthorityResponseV1 {
    pub fence: ProjectMutationFenceV1,
}

fn valid_hash(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn serialize_request_id<S: Serializer>(value: &u64, serializer: S) -> Result<S::Ok, S::Error> {
    if *value == 0 || *value > MAX_SAFE_JAVASCRIPT_INTEGER {
        return Err(serde::ser::Error::custom(
            "invalid project replacement response identity",
        ));
    }
    value.serialize(serializer)
}

fn deserialize_request_id<'de, D: Deserializer<'de>>(deserializer: D) -> Result<u64, D::Error> {
    let value = u64::deserialize(deserializer)?;
    if value == 0 || value > MAX_SAFE_JAVASCRIPT_INTEGER {
        return Err(D::Error::custom(
            "invalid project replacement response identity",
        ));
    }
    Ok(value)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum ProjectReplacementActionV1 {
    New {},
    Open {
        path: String,
        expected_file_sha256: String,
    },
}

impl ProjectReplacementActionV1 {
    pub fn operation_id(&self) -> &'static str {
        match self {
            Self::New {} => PROJECT_NEW_OPERATION_ID,
            Self::Open { .. } => PROJECT_OPEN_OPERATION_ID,
        }
    }

    fn validate(&self) -> Result<(), &'static str> {
        if let Self::Open {
            path,
            expected_file_sha256,
        } = self
        {
            if path.is_empty()
                || path.len() > MAX_PROJECT_OPEN_PATH_BYTES
                || path.chars().any(char::is_control)
                || !valid_hash(expected_file_sha256)
            {
                return Err("invalid project Open target");
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReplacementRequestV1 {
    pub schema_version: u16,
    pub operation_id: String,
    pub request_id: u64,
    pub expected_fence: ProjectMutationFenceV1,
    pub action: ProjectReplacementActionV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RequestWire {
    schema_version: u16,
    operation_id: String,
    request_id: u64,
    expected_fence: ProjectMutationFenceV1,
    action: ProjectReplacementActionV1,
}

impl ProjectReplacementRequestV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.schema_version != PROJECT_REPLACEMENT_SCHEMA_VERSION
            || self.operation_id != self.action.operation_id()
        {
            return Err("unsupported project replacement contract");
        }
        if self.request_id == 0 || self.request_id > MAX_SAFE_JAVASCRIPT_INTEGER {
            return Err("invalid project replacement request identity");
        }
        self.expected_fence
            .validate()
            .map_err(|_| "invalid project replacement fence")?;
        if self.expected_fence.project_epoch == MAX_SAFE_JAVASCRIPT_INTEGER
            || self.expected_fence.project_publication_generation == MAX_SAFE_JAVASCRIPT_INTEGER
        {
            return Err("project replacement generation is exhausted");
        }
        self.action.validate()
    }
}

impl Serialize for ProjectReplacementRequestV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        RequestWire {
            schema_version: self.schema_version,
            operation_id: self.operation_id.clone(),
            request_id: self.request_id,
            expected_fence: self.expected_fence.clone(),
            action: self.action.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProjectReplacementRequestV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = RequestWire::deserialize(deserializer)?;
        let request = Self {
            schema_version: wire.schema_version,
            operation_id: wire.operation_id,
            request_id: wire.request_id,
            expected_fence: wire.expected_fence,
            action: wire.action,
        };
        request.validate().map_err(D::Error::custom)?;
        Ok(request)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectReplacementAuthorityV1 {
    pub project_epoch: u64,
    pub project_revision: u64,
    pub checkpoint_hash: String,
    pub publication_generation: u64,
    pub recovery_authority_serial: u64,
    pub current_project_path: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "authority",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProjectReplacementOutcomeV1 {
    Applied(ProjectReplacementAuthorityV1),
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectReplacementReceiptV1 {
    pub request: ProjectReplacementRequestV1,
    pub outcome: ProjectReplacementOutcomeV1,
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReceiptWire {
    request: ProjectReplacementRequestV1,
    outcome: ProjectReplacementOutcomeV1,
}

impl ProjectReplacementReceiptV1 {
    pub fn validate(&self) -> Result<(), &'static str> {
        self.request.validate()?;
        if let ProjectReplacementOutcomeV1::Applied(after) = &self.outcome {
            let before = &self.request.expected_fence;
            if after.project_epoch != before.project_epoch + 1
                || after.project_revision != 0
                || after.publication_generation != before.project_publication_generation + 1
                || [
                    after.project_epoch,
                    after.publication_generation,
                    after.recovery_authority_serial,
                ]
                .iter()
                .any(|value| *value > MAX_SAFE_JAVASCRIPT_INTEGER)
                || after.recovery_authority_serial == 0
                || !valid_hash(&after.checkpoint_hash)
            {
                return Err("invalid project replacement terminal authority");
            }
            match (&self.request.action, &after.current_project_path) {
                (ProjectReplacementActionV1::New {}, None) => {}
                (ProjectReplacementActionV1::Open { path, .. }, Some(current))
                    if path == current => {}
                _ => return Err("project replacement terminal target mismatch"),
            }
        }
        Ok(())
    }
}

impl Serialize for ProjectReplacementReceiptV1 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.validate().map_err(serde::ser::Error::custom)?;
        ReceiptWire {
            request: self.request.clone(),
            outcome: self.outcome.clone(),
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for ProjectReplacementReceiptV1 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ReceiptWire::deserialize(deserializer)?;
        let receipt = Self {
            request: wire.request,
            outcome: wire.outcome,
        };
        receipt.validate().map_err(D::Error::custom)?;
        Ok(receipt)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectReplacementErrorV1 {
    InvalidRequest,
    Forbidden,
    StaleFence,
    Conflict,
    Busy,
    Overloaded,
    FileChanged,
    InvalidProject,
    PublicationFailed,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "value",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum ProjectReplacementResponseV1 {
    Receipt(ProjectReplacementReceiptV1),
    Rejected {
        #[serde(
            serialize_with = "serialize_request_id",
            deserialize_with = "deserialize_request_id"
        )]
        request_id: u64,
        code: ProjectReplacementErrorV1,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(action: ProjectReplacementActionV1) -> ProjectReplacementRequestV1 {
        ProjectReplacementRequestV1 {
            schema_version: 1,
            operation_id: action.operation_id().into(),
            request_id: 7,
            action,
            expected_fence: ProjectMutationFenceV1 {
                process_incarnation: 1,
                session_incarnation: 2,
                project_epoch: 3,
                project_revision: 4,
                project_checkpoint_hash: "a".repeat(64),
                project_publication_generation: 5,
            },
        }
    }

    fn actions() -> [ProjectReplacementActionV1; 2] {
        [
            ProjectReplacementActionV1::New {},
            ProjectReplacementActionV1::Open {
                path: "C:\\show\\日本語.sdc".into(),
                expected_file_sha256: "b".repeat(64),
            },
        ]
    }

    #[test]
    fn project_replacement_response_rejects_unsafe_identity_in_both_directions() {
        for id in [0, MAX_SAFE_JAVASCRIPT_INTEGER + 1] {
            assert!(
                serde_json::to_value(ProjectReplacementResponseV1::Rejected {
                    request_id: id,
                    code: ProjectReplacementErrorV1::InvalidRequest,
                })
                .is_err()
            );
            assert!(
                serde_json::from_value::<ProjectReplacementResponseV1>(serde_json::json!({
                    "status": "rejected", "value": { "request_id": id, "code": "invalid_request" },
                }))
                .is_err()
            );
        }
    }

    #[test]
    fn project_replacement_requests_round_trip_and_reject_forged_authority() {
        for action in actions() {
            let source = request(action);
            let wire = serde_json::to_value(&source).unwrap();
            assert_eq!(
                serde_json::from_value::<ProjectReplacementRequestV1>(wire.clone()).unwrap(),
                source
            );
            for field in [
                "owner_id",
                "principal",
                "skip_confirmation",
                "confirmation_origin",
                "project",
            ] {
                let mut bad = wire.clone();
                bad[field] = serde_json::json!("forged");
                assert!(
                    serde_json::from_value::<ProjectReplacementRequestV1>(bad).is_err(),
                    "{field}"
                );
            }
            for version in [0, 2] {
                let mut bad = wire.clone();
                bad["schema_version"] = serde_json::json!(version);
                assert!(serde_json::from_value::<ProjectReplacementRequestV1>(bad).is_err());
            }
        }
    }

    #[test]
    fn project_replacement_requires_exact_operation_safe_identity_and_complete_fence() {
        let source = request(actions()[0].clone());
        for id in [0, MAX_SAFE_JAVASCRIPT_INTEGER + 1] {
            let mut bad = source.clone();
            bad.request_id = id;
            assert!(serde_json::to_value(bad).is_err());
        }
        let wire = serde_json::to_value(&source).unwrap();
        let mut bad = wire.clone();
        bad["operation_id"] = serde_json::json!(PROJECT_OPEN_OPERATION_ID);
        assert!(serde_json::from_value::<ProjectReplacementRequestV1>(bad).is_err());
        for field in [
            "process_incarnation",
            "session_incarnation",
            "project_epoch",
            "project_revision",
            "project_checkpoint_hash",
            "project_publication_generation",
        ] {
            let mut bad = wire.clone();
            bad["expected_fence"].as_object_mut().unwrap().remove(field);
            assert!(
                serde_json::from_value::<ProjectReplacementRequestV1>(bad).is_err(),
                "{field}"
            );
        }
    }

    #[test]
    fn project_open_requires_bounded_target_and_exact_sha256() {
        for (path, hash) in [
            ("", "b".repeat(64)),
            ("C:\\show\0.sdc", "b".repeat(64)),
            ("show.sdc", "B".repeat(64)),
            ("show.sdc", "b".repeat(63)),
        ] {
            assert!(
                serde_json::to_value(request(ProjectReplacementActionV1::Open {
                    path: path.into(),
                    expected_file_sha256: hash,
                }))
                .is_err()
            );
        }
        assert!(
            serde_json::to_value(request(ProjectReplacementActionV1::Open {
                path: "x".repeat(MAX_PROJECT_OPEN_PATH_BYTES + 1),
                expected_file_sha256: "b".repeat(64),
            }))
            .is_err()
        );
    }

    #[test]
    fn project_replacement_receipt_requires_exact_successor_and_target() {
        for action in actions() {
            let path = match &action {
                ProjectReplacementActionV1::New {} => None,
                ProjectReplacementActionV1::Open { path, .. } => Some(path.clone()),
            };
            let receipt = ProjectReplacementReceiptV1 {
                request: request(action),
                outcome: ProjectReplacementOutcomeV1::Applied(ProjectReplacementAuthorityV1 {
                    project_epoch: 4,
                    project_revision: 0,
                    checkpoint_hash: "c".repeat(64),
                    publication_generation: 6,
                    recovery_authority_serial: 9,
                    current_project_path: path,
                }),
            };
            let wire = serde_json::to_value(&receipt).unwrap();
            assert_eq!(
                serde_json::from_value::<ProjectReplacementReceiptV1>(wire.clone()).unwrap(),
                receipt
            );
            for (field, value) in [
                ("project_epoch", 3),
                ("project_revision", 1),
                ("publication_generation", 5),
                ("publication_generation", 7),
                ("recovery_authority_serial", 0),
                ("recovery_authority_serial", MAX_SAFE_JAVASCRIPT_INTEGER + 1),
            ] {
                let mut bad = wire.clone();
                bad["outcome"]["authority"][field] = serde_json::json!(value);
                assert!(
                    serde_json::from_value::<ProjectReplacementReceiptV1>(bad).is_err(),
                    "{field}"
                );
            }
            let cancelled = ProjectReplacementReceiptV1 {
                outcome: ProjectReplacementOutcomeV1::Cancelled,
                ..receipt
            };
            assert_eq!(
                serde_json::from_value::<ProjectReplacementReceiptV1>(
                    serde_json::to_value(&cancelled).unwrap()
                )
                .unwrap(),
                cancelled
            );
        }
    }
}
