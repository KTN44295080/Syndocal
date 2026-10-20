use super::*;

const ACTIONS: [&str; 5] = [
    "VideoGroupOpacity",
    "VideoGroupSolo",
    "VideoGroupBypass",
    "VideoGroupBlendMode",
    "VideoGroupEnabled",
];

fn legacy_osc() -> serde_json::Value {
    serde_json::json!({
        "address": "/master", "action": "VideoMaster", "fixture_id": null,
        "attribute": null, "group_id": null, "cue_id": null, "layer_id": null,
        "output_id": null, "video_param": null, "cue_point_index": null,
        "duration_ms": null, "low": 0.0, "high": 1.0
    })
}

fn as_midi(mut json: serde_json::Value) -> serde_json::Value {
    let object = json.as_object_mut().unwrap();
    object.remove("address");
    object.insert("channel".to_string(), serde_json::json!(0));
    object.insert("message".to_string(), serde_json::json!("ControlChange"));
    object.insert("number".to_string(), serde_json::json!(7));
    json
}

fn as_dmx(mut json: serde_json::Value) -> serde_json::Value {
    let object = json.as_object_mut().unwrap();
    object.remove("address");
    object.insert("universe".to_string(), serde_json::json!(0));
    object.insert("channel".to_string(), serde_json::json!(7));
    json
}

fn roundtrip<T>(json: serde_json::Value)
where
    T: serde::de::DeserializeOwned + serde::Serialize + PartialEq + std::fmt::Debug,
{
    let first: T = serde_json::from_value(json.clone()).unwrap();
    assert_eq!(serde_json::to_value(&first).unwrap(), json);
    let again: T = serde_json::from_value(serde_json::to_value(&first).unwrap()).unwrap();
    assert_eq!(first, again);
}

#[test]
fn video_group_mappings_legacy_json_keeps_its_shape() {
    let osc = legacy_osc();
    roundtrip::<OscControlMapping>(osc.clone());
    roundtrip::<MidiControlMapping>(as_midi(osc.clone()));
    roundtrip::<DmxControlMapping>(as_dmx(osc));
}

#[test]
fn video_group_mappings_all_actions_roundtrip_without_positional_targets() {
    for action in ACTIONS {
        let mut osc = legacy_osc();
        osc["action"] = serde_json::json!(action);
        osc["video_group_id"] = serde_json::json!(41);
        roundtrip::<OscControlMapping>(osc.clone());
        roundtrip::<MidiControlMapping>(as_midi(osc.clone()));
        roundtrip::<DmxControlMapping>(as_dmx(osc));
        roundtrip::<RendererRoutedInput>(serde_json::json!({
            "action": action, "video_group_id": 41,
            "position": 0.25, "value": 0.25, "active": true
        }));
    }
}

#[test]
fn video_group_mappings_reject_non_integer_and_negative_identities() {
    for id in [
        serde_json::json!(-1),
        serde_json::json!(1.5),
        serde_json::json!("41"),
    ] {
        let mut osc = legacy_osc();
        osc["action"] = serde_json::json!("VideoGroupOpacity");
        osc["video_group_id"] = id;
        assert!(serde_json::from_value::<OscControlMapping>(osc.clone()).is_err());
        assert!(serde_json::from_value::<MidiControlMapping>(as_midi(osc.clone())).is_err());
        assert!(serde_json::from_value::<DmxControlMapping>(as_dmx(osc)).is_err());
    }
}

#[test]
fn video_group_mappings_only_boolean_actions_accept_beat_triggers() {
    for action in ACTIONS {
        let action: MidiControlAction = serde_json::from_value(serde_json::json!(action)).unwrap();
        assert!(action.is_renderer_routed());
        assert!(action.is_video_group());
        let switch = matches!(
            action,
            MidiControlAction::VideoGroupSolo
                | MidiControlAction::VideoGroupBypass
                | MidiControlAction::VideoGroupEnabled
        );
        assert_eq!(action.is_renderer_routed_trigger(), switch);
        assert_eq!(action.is_renderer_routed_switch(), switch);
    }
}
