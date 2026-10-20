use super::*;

fn osc_mapping(action: &str) -> OscControlMapping {
    serde_json::from_value(serde_json::json!({
        "address": "/perform/control",
        "action": action,
        "layer_id": 4,
        "clip_slot_id": 21,
        "isf_stage_id": 31,
        "attribute": "amount",
        "low": 0.0,
        "high": 1.0
    }))
    .unwrap()
}

fn dmx_mapping(action: &str) -> DmxControlMapping {
    serde_json::from_value(serde_json::json!({
        "universe": 0,
        "channel": 12,
        "action": action,
        "layer_id": 4,
        "clip_slot_id": 21,
        "isf_stage_id": 31,
        "attribute": "amount",
        "low": 0.0,
        "high": 1.0
    }))
    .unwrap()
}

#[test]
fn perform_input_all_targets_validate_and_preserve_stable_ids_for_osc_and_dmx() {
    for action in [
        "VideoClipSlotTrigger",
        "VideoTake",
        "VideoLayerStop",
        "VideoLayerBlendMode",
        "VideoTransitionKind",
        "VideoTransitionTime",
        "VideoIsfStageEnabled",
        "VideoIsfParam",
    ] {
        let osc = validate_osc_control_mapping(1, osc_mapping(action)).unwrap();
        let dmx = validate_dmx_control_mapping(1, dmx_mapping(action)).unwrap();
        for json in [
            serde_json::to_value(osc).unwrap(),
            serde_json::to_value(dmx).unwrap(),
        ] {
            assert_eq!(json["action"], action);
            assert_eq!(json["clip_slot_id"], 21);
            assert_eq!(json["isf_stage_id"], 31);
            assert_eq!(json["layer_id"], 4);
        }
    }
}

#[test]
fn perform_input_missing_required_target_identity_fails_closed() {
    for (action, field, message) in [
        ("VideoClipSlotTrigger", "layer_id", "video layer"),
        ("VideoClipSlotTrigger", "clip_slot_id", "clip slot"),
        ("VideoLayerStop", "layer_id", "video layer"),
        ("VideoLayerBlendMode", "layer_id", "video layer"),
        ("VideoIsfStageEnabled", "isf_stage_id", "effect stage"),
        ("VideoIsfParam", "isf_stage_id", "effect stage"),
        ("VideoIsfParam", "attribute", "attribute"),
    ] {
        let mut osc = serde_json::to_value(osc_mapping(action)).unwrap();
        osc.as_object_mut().unwrap().remove(field);
        let error =
            validate_osc_control_mapping(1, serde_json::from_value(osc).unwrap()).unwrap_err();
        assert!(error.contains(message), "{action}/{field}: {error}");
        let mut dmx = serde_json::to_value(dmx_mapping(action)).unwrap();
        dmx.as_object_mut().unwrap().remove(field);
        let error =
            validate_dmx_control_mapping(1, serde_json::from_value(dmx).unwrap()).unwrap_err();
        assert!(error.contains(message), "{action}/{field}: {error}");
    }
}

#[test]
fn perform_input_parameter_name_normalization_is_shared_and_empty_names_are_rejected() {
    let mut osc = osc_mapping("VideoIsfParam");
    let mut dmx = dmx_mapping("VideoIsfParam");
    osc.attribute = Some("  amount  ".to_string());
    dmx.attribute = osc.attribute.clone();
    assert_eq!(
        validate_osc_control_mapping(1, osc)
            .unwrap()
            .attribute
            .as_deref(),
        Some("amount")
    );
    assert_eq!(
        validate_dmx_control_mapping(1, dmx)
            .unwrap()
            .attribute
            .as_deref(),
        Some("amount")
    );
    let mut empty = osc_mapping("VideoIsfParam");
    empty.attribute = Some("   ".to_string());
    assert!(validate_osc_control_mapping(1, empty).is_err());
    let mut invalid = dmx_mapping("VideoIsfParam");
    invalid.high = f32::NAN;
    assert!(validate_dmx_control_mapping(1, invalid).is_err());
}

#[test]
fn perform_input_legacy_mapping_json_omits_new_optional_ids() {
    for (json, is_osc) in [
        (
            serde_json::json!({
                "address": "/master", "action": "LightingMaster", "low": 0.0, "high": 1.0
            }),
            true,
        ),
        (
            serde_json::json!({
                "universe": 0, "channel": 1, "action": "LightingMaster", "low": 0.0, "high": 1.0
            }),
            false,
        ),
    ] {
        let encoded = if is_osc {
            let mapping =
                validate_osc_control_mapping(1, serde_json::from_value(json).unwrap()).unwrap();
            serde_json::to_value(mapping).unwrap()
        } else {
            let mapping =
                validate_dmx_control_mapping(1, serde_json::from_value(json).unwrap()).unwrap();
            serde_json::to_value(mapping).unwrap()
        };
        assert_eq!(encoded["action"], "LightingMaster");
        assert!(encoded.get("clip_slot_id").is_none());
        assert!(encoded.get("isf_stage_id").is_none());
        assert!(encoded.get("video_group_id").is_none());
    }
}

fn video_group_json(action: &str) -> serde_json::Value {
    serde_json::json!({
        "action": action, "video_group_id": 41, "low": 0.0, "high": 1.0
    })
}

fn video_group_midi_json(action: &str) -> serde_json::Value {
    let mut json = video_group_json(action);
    json["message"] = serde_json::json!("ControlChange");
    json["number"] = serde_json::json!(7);
    json
}

fn video_group_osc_json(action: &str) -> serde_json::Value {
    let mut json = video_group_json(action);
    json["address"] = serde_json::json!("/groups/control");
    json
}

fn video_group_dmx_json(action: &str) -> serde_json::Value {
    let mut json = video_group_json(action);
    json["universe"] = serde_json::json!(0);
    json["channel"] = serde_json::json!(7);
    json
}

const VIDEO_GROUP_ACTIONS: [&str; 5] = [
    "VideoGroupOpacity",
    "VideoGroupSolo",
    "VideoGroupBypass",
    "VideoGroupBlendMode",
    "VideoGroupEnabled",
];

#[test]
fn video_group_input_all_transports_preserve_stable_group_identity() {
    for action in VIDEO_GROUP_ACTIONS {
        let midi = validate_midi_control_mapping(
            1,
            serde_json::from_value(video_group_midi_json(action)).unwrap(),
        )
        .unwrap();
        let osc = validate_osc_control_mapping(
            1,
            serde_json::from_value(video_group_osc_json(action)).unwrap(),
        )
        .unwrap();
        let dmx = validate_dmx_control_mapping(
            1,
            serde_json::from_value(video_group_dmx_json(action)).unwrap(),
        )
        .unwrap();
        for json in [
            serde_json::to_value(midi).unwrap(),
            serde_json::to_value(osc).unwrap(),
            serde_json::to_value(dmx).unwrap(),
        ] {
            assert_eq!(json["action"], action);
            assert_eq!(json["video_group_id"], 41);
            assert!(json["layer_id"].is_null());
            assert!(json.get("clip_slot_id").is_none());
            assert!(json.get("isf_stage_id").is_none());
        }
    }
}

#[test]
fn video_group_input_missing_unsafe_and_ambiguous_identities_fail_closed() {
    for action in VIDEO_GROUP_ACTIONS {
        for (field, value) in [
            ("video_group_id", serde_json::Value::Null),
            ("video_group_id", serde_json::json!(0)),
            (
                "video_group_id",
                serde_json::json!(9_007_199_254_740_992u64),
            ),
            ("layer_id", serde_json::json!(4)),
            ("clip_slot_id", serde_json::json!(21)),
            ("isf_stage_id", serde_json::json!(31)),
            ("group_id", serde_json::json!("lighting-group")),
            ("attribute", serde_json::json!("amount")),
        ] {
            let mut midi = video_group_midi_json(action);
            let mut osc = video_group_osc_json(action);
            let mut dmx = video_group_dmx_json(action);
            midi[field] = value.clone();
            osc[field] = value.clone();
            dmx[field] = value;
            assert!(
                validate_midi_control_mapping(1, serde_json::from_value(midi).unwrap()).is_err(),
                "MIDI {action}/{field}"
            );
            assert!(
                validate_osc_control_mapping(1, serde_json::from_value(osc).unwrap()).is_err(),
                "OSC {action}/{field}"
            );
            assert!(
                validate_dmx_control_mapping(1, serde_json::from_value(dmx).unwrap()).is_err(),
                "DMX {action}/{field}"
            );
        }
    }
}

#[test]
fn video_group_input_other_actions_cannot_carry_a_group_target() {
    for action in ["VideoMaster", "VideoTake"] {
        assert!(validate_midi_control_mapping(
            1,
            serde_json::from_value(video_group_midi_json(action)).unwrap()
        )
        .is_err());
        assert!(validate_osc_control_mapping(
            1,
            serde_json::from_value(video_group_osc_json(action)).unwrap()
        )
        .is_err());
        assert!(validate_dmx_control_mapping(
            1,
            serde_json::from_value(video_group_dmx_json(action)).unwrap()
        )
        .is_err());
    }
}

#[test]
fn video_group_input_retains_unresolved_stable_ids_for_save_and_reload() {
    // Catalog deletion must not make the remaining mappings unloadable.
    // A missing group is resolved as inert by the renderer, never by an index.
    for action in VIDEO_GROUP_ACTIONS {
        let mut midi = video_group_midi_json(action);
        let mut osc = video_group_osc_json(action);
        let mut dmx = video_group_dmx_json(action);
        for json in [&mut midi, &mut osc, &mut dmx] {
            json["video_group_id"] = serde_json::json!(999_999);
        }
        let midi = validate_midi_control_mapping(1, serde_json::from_value(midi).unwrap()).unwrap();
        let osc = validate_osc_control_mapping(1, serde_json::from_value(osc).unwrap()).unwrap();
        let dmx = validate_dmx_control_mapping(1, serde_json::from_value(dmx).unwrap()).unwrap();
        assert_eq!(
            serde_json::to_value(midi).unwrap()["video_group_id"],
            999_999
        );
        assert_eq!(
            serde_json::to_value(osc).unwrap()["video_group_id"],
            999_999
        );
        assert_eq!(
            serde_json::to_value(dmx).unwrap()["video_group_id"],
            999_999
        );
    }
}

#[test]
fn video_group_input_beat_accepts_only_switches_and_relative_encoders_are_rejected() {
    for action in VIDEO_GROUP_ACTIONS {
        let mut beat = video_group_midi_json(action);
        beat["message"] = serde_json::json!("BeatDivision");
        beat["number"] = serde_json::json!(2);
        let is_switch = matches!(
            action,
            "VideoGroupSolo" | "VideoGroupBypass" | "VideoGroupEnabled"
        );
        assert_eq!(
            validate_midi_control_mapping(1, serde_json::from_value(beat).unwrap()).is_ok(),
            is_switch,
            "{action}"
        );
        let mut relative = video_group_midi_json(action);
        relative["message"] = serde_json::json!("RelativeControlChange");
        assert!(
            validate_midi_control_mapping(1, serde_json::from_value(relative).unwrap()).is_err(),
            "renderer state cannot be read by the native relative-encoder path: {action}"
        );
    }
}
