use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) const MAX_REQUEST_BYTES: usize = 64 * 1024;
pub(super) const MAX_RESULT_BYTES: usize = 255 * 1024;
pub(super) const CANONICAL_OPERATION_IDS: &[&str] = &[
    "syndocal.query.control_plane.registry.v1",
    "syndocal.query.control_plane.canonical_registry.v3",
    "syndocal.query.control_plane.schemas.v1",
    "syndocal.query.control_plane.capabilities.v1",
    "syndocal.query.project.authority.v1",
    "syndocal.query.runtime.generations.v1",
    "syndocal.query.runtime.timeline.transport.authority.v1",
    "syndocal.query.runtime.timeline.loop.authority.v1",
    "syndocal.query.runtime.timeline.follow.abort.authority.v1",
    "syndocal.query.output.control.authority.v1",
    "syndocal.query.output.dsf2026_artnet_acceptance_probe.status.v1",
    "syndocal.query.output.display.add.authority.v1",
    "syndocal.query.output.ownership.v1",
    "syndocal.query.video.display_monitors.v1",
    "syndocal.query.video.camera_profiles.v1",
    "syndocal.query.video.camera_profile_probe.v1",
    "syndocal.query.video.output_window_observation.v1",
    "syndocal.query.events.observations.v1",
    "syndocal.effects.set_enabled.v1",
    "syndocal.runtime.timeline.transport.set_playing.v1",
    "syndocal.runtime.timeline.loop.commit.v1",
    "syndocal.runtime.timeline.follow.abort.v1",
    "syndocal.safety.blackout.engage.v1",
    "syndocal.output.blackout.release.v2",
    "syndocal.output.blackout.set.v2",
    "syndocal.output.ownership.arm.v2",
    "syndocal.output.standby.takeover.v2",
    "syndocal.output.display.add.v2",
    "syndocal.output.display.window.set_open.v2",
    "syndocal.output.video.composition.assign.v2",
    "syndocal.output.show_artnet_loopback_route.enable.v1",
    "syndocal.output.show_serial_dmx_s0_route.enable.v1",
    "syndocal.output.show_serial_dmx_s0_route.stop.v1",
    "syndocal.output.show_spout_outputs.enable.v2",
    "syndocal.output.show_spout_outputs.reset.v1",
    "syndocal.output.dsf2026_artnet_acceptance_probe.send.v1",
    "syndocal.output.dsf2026_artnet_acceptance_probe.reconcile.v1",
    "syndocal.output.enable.v2",
    "syndocal.output.lease.acquire.v2",
    "syndocal.output.lease.renew.v2",
    "syndocal.output.lease.recover.v2",
    "syndocal.output.lease.relinquish.v2",
    "syndocal.output.lease.force_transfer.v2",
    "syndocal.cue_lists.reorder.v1",
    "syndocal.cue_lists.rename.v1",
    "syndocal.cue_lists.delete.v1",
    "syndocal.scenes.create.v1",
];

pub(super) fn canonical_operation_is_mutation(operation_id: &str) -> bool {
    !operation_id.starts_with("syndocal.query.")
}

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
    #[serde(rename = "control_plane.get_capabilities")]
    ControlPlaneCapabilities(Empty),
    #[serde(rename = "recording.get_status")]
    RecordingStatus(Empty),
    #[serde(rename = "control_plane.execute")]
    ControlPlaneExecute(CanonicalOperation),
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
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub(super) struct CanonicalOperation {
    #[serde(rename = "operationId")]
    pub operation_id: String,
    pub request: Value,
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
                | "control_plane.get_capabilities"
                | "recording.get_status"
                | "control_plane.execute"
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
            Command::ControlPlaneExecute(value) => {
                if value.operation_id.is_empty()
                    || value.operation_id.len() > 512
                    || !CANONICAL_OPERATION_IDS.contains(&value.operation_id.as_str())
                    || !value.request.is_object()
                {
                    return Err("invalid_canonical_operation");
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
