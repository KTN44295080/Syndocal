//! Renderer-routed Perform input accepted by the native MIDI worker.
//!
//! Clip slots, Take, the transition selection and effect-stage state are owned
//! by the renderer's ticketed commands. The native worker therefore forwards
//! the accepted input instead of issuing an engine command, so there is exactly
//! one path from a controller to those controls: the one the on-screen control
//! already uses. The event carries the same connection fence as the fader
//! projection and shares its sequence counter.
use crate::midi_fader_input::Connection;
use protocol::RendererRoutedInput;
use serde::Serialize;

pub(crate) const EVENT: &str = "syndocal://midi-perform-control-v1";
pub(crate) const OSC_EVENT: &str = "syndocal://osc-perform-control-v1";
pub(crate) const DMX_EVENT: &str = "syndocal://dmx-perform-control-v1";

#[derive(Clone, Serialize)]
pub(crate) struct Input {
    #[serde(flatten)]
    pub connection: Connection,
    pub sequence: u64,
    pub input: RendererRoutedInput,
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::{MidiControlAction, VideoClipSlotId};

    #[test]
    fn payload_carries_the_connection_fence_and_stable_identities_only() {
        let input = Input {
            connection: Connection {
                schema_version: 1,
                project_epoch: 7,
                input_session: 3,
            },
            sequence: 9,
            input: RendererRoutedInput {
                action: MidiControlAction::VideoClipSlotTrigger,
                layer_id: Some(4),
                clip_slot_id: Some(VideoClipSlotId(21)),
                isf_stage_id: None,
                control: None,
                position: 1.0,
                value: 1.0,
                active: true,
                toggle: false,
            },
        };
        let json = serde_json::to_value(&input).unwrap();
        assert_eq!(
            json,
            serde_json::json!({
                "schema_version": 1,
                "project_epoch": 7,
                "input_session": 3,
                "sequence": 9,
                "input": {
                    "action": "VideoClipSlotTrigger",
                    "layer_id": 4,
                    "clip_slot_id": 21,
                    "position": 1.0,
                    "value": 1.0,
                    "active": true,
                },
            })
        );
    }
}
