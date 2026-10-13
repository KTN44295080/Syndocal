//! Narrow UI projection of accepted MIDI fader input; never physical-output proof.
use engine::EngineCommand;
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};

pub(crate) const EVENT: &str = "syndocal://midi-fader-input-v1";
static NEXT_SESSION: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Serialize)]
pub(crate) struct Connection {
    pub schema_version: u16,
    pub project_epoch: u64,
    pub input_session: u64,
}
impl Connection {
    pub(crate) fn new(project_epoch: u64) -> Result<Self, String> {
        let input_session = NEXT_SESSION
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |n| {
                (n < protocol::control_plane_command::MAX_SAFE_JAVASCRIPT_INTEGER).then_some(n + 1)
            })
            .map_err(|_| "MIDI input session identity exhausted".to_string())?;
        Ok(Self {
            schema_version: 1,
            project_epoch,
            input_session,
        })
    }
}
#[derive(Clone, Serialize)]
pub(crate) struct Value {
    pub fixture_id: u64,
    pub attribute: String,
    pub value: u16,
}
#[derive(Clone, Serialize)]
pub(crate) struct Input {
    #[serde(flatten)]
    pub connection: Connection,
    pub sequence: u64,
    pub values: Vec<Value>,
}
pub(crate) fn values(command: &EngineCommand) -> Vec<Value> {
    match command {
        EngineCommand::SetAttribute {
            fixture_id,
            attribute,
            value,
        } => vec![Value {
            fixture_id: *fixture_id,
            attribute: attribute.clone(),
            value: *value,
        }],
        EngineCommand::SetFixtureAttributeBatch { targets, value } => targets
            .iter()
            .map(|target| Value {
                fixture_id: target.fixture_id,
                attribute: target.attribute.clone(),
                value: *value,
            })
            .collect(),
        _ => Vec::new(),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sessions_never_reuse_and_projection_contains_only_fader_input() {
        assert_ne!(
            Connection::new(1).unwrap().input_session,
            Connection::new(1).unwrap().input_session
        );
        let input = values(&EngineCommand::SetAttribute {
            fixture_id: 7,
            attribute: "Dimmer".into(),
            value: 32000,
        });
        assert_eq!(input[0].fixture_id, 7);
        assert_eq!(input[0].value, 32000);
        assert!(values(&EngineCommand::Blackout(true)).is_empty());
    }
}
