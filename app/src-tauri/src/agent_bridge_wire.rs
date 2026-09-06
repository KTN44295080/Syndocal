use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub(super) const MAX_RESULT_BYTES: usize = 255 * 1024;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Request {
    pub token: String,
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method", content = "params", deny_unknown_fields)]
pub(super) enum Command {
    #[serde(rename = "fixtures.list")]
    List(Empty),
    #[serde(rename = "fixtures.get")]
    Get(Fixture),
    #[serde(rename = "fixtures.set_transform")]
    SetTransform(Transform),
    #[serde(rename = "output.set_video_blackout")]
    SetVideoBlackout(VideoBlackout),
    #[serde(rename = "request.status")]
    Status(Status),
    #[serde(rename = "runtime.get")]
    RuntimeGet(Empty),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Empty {}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Fixture {
    #[serde(rename = "fixtureId")]
    pub fixture_id: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Transform {
    #[serde(rename = "fixtureId")]
    pub fixture_id: u64,
    pub position: Position,
    pub rotation: Rotation,
    #[serde(rename = "expectedProject")]
    pub expected_project: Project,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct VideoBlackout {
    pub enabled: bool,
    #[serde(rename = "expectedProject")]
    pub expected_project: Project,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Rotation {
    pub pitch: f64,
    pub yaw: f64,
    pub roll: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Project {
    pub project_epoch: u64,
    pub project_revision: u64,
    pub checkpoint_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Status {
    #[serde(rename = "requestId")]
    pub request_id: String,
}

pub(super) fn valid_uuid(value: &str) -> bool {
    value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
        && value.bytes().any(|b| b.is_ascii_hexdigit() && b != b'0')
}
impl Request {
    pub(super) fn command(&self) -> Result<Command, &'static str> {
        if !valid_uuid(&self.request_id) {
            return Err("invalid_request_id");
        }
        if !matches!(
            self.method.as_str(),
            "fixtures.list"
                | "fixtures.get"
                | "fixtures.set_transform"
                | "output.set_video_blackout"
                | "request.status"
                | "runtime.get"
        ) {
            return Err("unknown_method");
        }
        let command: Command =
            serde_json::from_value(serde_json::json!({"method":self.method,"params":self.params}))
                .map_err(|_| "invalid_params")?;
        const MAX_SAFE: u64 = 9_007_199_254_740_991;
        match &command {
            Command::Get(value) if value.fixture_id == 0 || value.fixture_id > MAX_SAFE => {
                return Err("invalid_fixture_id")
            }
            Command::SetTransform(value) => {
                if value.fixture_id == 0 || value.fixture_id > MAX_SAFE {
                    return Err("invalid_fixture_id");
                }
                if [
                    value.position.x,
                    value.position.y,
                    value.position.z,
                    value.rotation.pitch,
                    value.rotation.yaw,
                    value.rotation.roll,
                ]
                .iter()
                .any(|value| !value.is_finite())
                {
                    return Err("invalid_transform");
                }
                let project = &value.expected_project;
                if project.project_epoch > MAX_SAFE
                    || project.project_revision > MAX_SAFE
                    || project.checkpoint_hash.len() != 64
                    || !project
                        .checkpoint_hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err("invalid_project_fence");
                }
            }
            Command::SetVideoBlackout(value) => {
                let project = &value.expected_project;
                if project.project_epoch > MAX_SAFE
                    || project.project_revision > MAX_SAFE
                    || project.checkpoint_hash.len() != 64
                    || !project
                        .checkpoint_hash
                        .bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
                    return Err("invalid_project_fence");
                }
            }
            Command::Status(value) if !valid_uuid(&value.request_id) => {
                return Err("invalid_request_id")
            }
            _ => {}
        }
        Ok(command)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct Response {
    #[serde(rename = "requestId")]
    pub request_id: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
impl Response {
    pub(super) fn status(id: &str, status: &str) -> Self {
        Self {
            request_id: id.to_string(),
            status: status.to_string(),
            result: None,
            error: None,
        }
    }
    pub(super) fn rejected(id: &str, code: &str) -> Self {
        let mut result = Self::status(id, "rejected");
        result.error = Some(code.to_string());
        result
    }
}
