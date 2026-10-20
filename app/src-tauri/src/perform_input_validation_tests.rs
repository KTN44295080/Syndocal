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
    }
}
