use super::*;
use protocol::VideoLayerGroupId;

const ACTIONS: [&str; 5] = [
    "VideoGroupOpacity",
    "VideoGroupSolo",
    "VideoGroupBypass",
    "VideoGroupBlendMode",
    "VideoGroupEnabled",
];

fn mapping(action: &str, message: &str) -> MidiControlMapping {
    serde_json::from_value(serde_json::json!({
        "channel": 0, "message": message, "number": 7, "action": action,
        "video_group_id": 41, "low": 0.0, "high": 1.0
    }))
    .unwrap()
}

fn routed(event: &MidiControlEvent) -> &RendererRoutedInput {
    match event {
        MidiControlEvent::RendererRouted(input) => input,
        other => panic!("group mappings must use the renderer's ticketed route: {other:?}"),
    }
}

#[test]
fn video_group_midi_all_five_actions_preserve_ids_zero_and_scaled_values() {
    for action in ACTIONS {
        let mut mapping = mapping(action, "ControlChange");
        mapping.low = 0.75;
        mapping.high = 0.25;
        for value in [0u8, 64, 127] {
            let events =
                events_from_midi_message(&[0xb0, 7, value], std::slice::from_ref(&mapping));
            assert_eq!(events.len(), 1, "{action}/{value}");
            let input = routed(&events[0]);
            assert_eq!(input.action, mapping.action);
            assert_eq!(input.video_group_id, Some(VideoLayerGroupId(41)));
            assert_eq!(input.layer_id, None);
            assert_eq!(input.clip_slot_id, None);
            assert_eq!(input.isf_stage_id, None);
            assert_eq!(input.control, None);
            assert_eq!(input.position, f32::from(value) / 127.0);
            assert_eq!(input.value, 0.75 - 0.5 * input.position);
            assert_eq!(input.active, value > 0);
            assert!(!input.toggle);
        }
    }
}

#[test]
fn video_group_midi_boolean_note_buttons_toggle_once_and_cc_zero_sets_false() {
    for action in ["VideoGroupSolo", "VideoGroupBypass", "VideoGroupEnabled"] {
        let mut mapping = mapping(action, "NoteOn");
        let press = events_from_midi_message(&[0x90, 7, 90], std::slice::from_ref(&mapping));
        assert_eq!(press.len(), 1);
        assert!(routed(&press[0]).toggle && routed(&press[0]).active);
        for release in [[0x80, 7, 0], [0x90, 7, 0]] {
            assert!(events_from_midi_message(&release, std::slice::from_ref(&mapping)).is_empty());
        }
        mapping.message = MidiControlMessage::ControlChange;
        let off = events_from_midi_message(&[0xb0, 7, 0], std::slice::from_ref(&mapping));
        assert_eq!(off.len(), 1);
        assert!(!routed(&off[0]).toggle && !routed(&off[0]).active);
    }
}

#[test]
fn video_group_midi_requires_a_positive_safe_group_identity() {
    for action in ACTIONS {
        for id in [
            None,
            Some(VideoLayerGroupId(0)),
            Some(VideoLayerGroupId(9_007_199_254_740_992)),
        ] {
            let mut mapping = mapping(action, "ControlChange");
            mapping.video_group_id = id;
            assert!(
                events_from_midi_message(&[0xb0, 7, 127], &[mapping]).is_empty(),
                "{action}/{id:?}"
            );
        }
    }
}

#[test]
fn video_group_midi_renderer_owned_actions_have_no_native_feedback_or_relative_value() {
    let snapshot = EngineSnapshot::default();
    for action in ACTIONS {
        let mapping = mapping(action, "ControlChange");
        assert!(!midi_action_is_continuous(&mapping.action));
        assert_eq!(feedback_value_for_mapping(&snapshot, &mapping, None), None);
    }
}

#[test]
fn video_group_midi_beat_sources_never_also_match_hardware() {
    for action in ["VideoGroupSolo", "VideoGroupBypass", "VideoGroupEnabled"] {
        let mut mapping = mapping(action, "BeatDivision");
        mapping.channel = None;
        mapping.number = 2;
        for message in [[0x90u8, 2, 127], [0xb0, 2, 127], [0x80, 2, 0], [0xc0, 2, 0]] {
            assert!(events_from_midi_message(&message, std::slice::from_ref(&mapping)).is_empty());
        }
        assert_eq!(midi_feedback_message(&mapping, 1.0), None);
    }
}
