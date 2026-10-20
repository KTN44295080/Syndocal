use super::*;
use crate::dmx_input::control_events_from_changed_frame;
use protocol::{DmxControlMapping, VideoClipSlotId, VideoEffectStageId};

fn mapping(action: OscControlAction) -> OscControlMapping {
    let clip_slot_id =
        (action == OscControlAction::VideoClipSlotTrigger).then_some(VideoClipSlotId(21));
    let isf_stage_id = matches!(
        action,
        OscControlAction::VideoIsfStageEnabled | OscControlAction::VideoIsfParam
    )
    .then_some(VideoEffectStageId(9));
    let attribute = (action == OscControlAction::VideoIsfParam).then(|| "amount".to_string());
    OscControlMapping {
        address: "/perform/control".to_string(),
        action,
        fixture_id: None,
        attribute,
        group_id: None,
        cue_id: None,
        layer_id: Some(4),
        clip_slot_id,
        isf_stage_id,
        video_group_id: None,
        output_id: None,
        video_param: None,
        cue_point_index: None,
        duration_ms: None,
        low: 0.0,
        high: 1.0,
    }
}
fn mapped(mapping: &OscControlMapping, args: Vec<OscType>) -> Option<RendererRoutedInput> {
    match event_from_mapping(
        &OscMessage {
            addr: mapping.address.clone(),
            args,
        },
        mapping,
    ) {
        Some(OscInputEvent::RendererRouted(input)) => Some(input),
        None => None,
        other => panic!("Perform must only forward renderer input: {other:?}"),
    }
}
#[test]
fn network_perform_all_actions_forward_the_same_renderer_contract() {
    let cases = [
        (
            OscControlAction::VideoClipSlotTrigger,
            MidiControlAction::VideoClipSlotTrigger,
        ),
        (OscControlAction::VideoTake, MidiControlAction::VideoTake),
        (
            OscControlAction::VideoLayerStop,
            MidiControlAction::VideoLayerStop,
        ),
        (
            OscControlAction::VideoLayerBlendMode,
            MidiControlAction::VideoLayerBlendMode,
        ),
        (
            OscControlAction::VideoTransitionKind,
            MidiControlAction::VideoTransitionKind,
        ),
        (
            OscControlAction::VideoTransitionTime,
            MidiControlAction::VideoTransitionTime,
        ),
        (
            OscControlAction::VideoIsfStageEnabled,
            MidiControlAction::VideoIsfStageEnabled,
        ),
        (
            OscControlAction::VideoIsfParam,
            MidiControlAction::VideoIsfParam,
        ),
    ];
    for (action, expected) in cases {
        let mut mapping = mapping(action);
        mapping.low = 0.25;
        mapping.high = 8.25;
        let input = mapped(&mapping, vec![OscType::Float(0.5)]).unwrap();
        assert_eq!(input.action, expected);
        assert_eq!(input.layer_id, Some(4));
        assert_eq!(input.clip_slot_id, mapping.clip_slot_id);
        assert_eq!(input.isf_stage_id, mapping.isf_stage_id);
        assert_eq!(input.control, mapping.attribute);
        assert_eq!(input.position, 0.5);
        assert_eq!(input.value, 4.25);
        assert!(input.active && !input.toggle);
        let json = serde_json::to_value(&mapping).unwrap();
        assert_eq!(
            serde_json::from_value::<OscControlMapping>(json).unwrap(),
            mapping
        );
    }
}
#[test]
fn network_perform_rejects_malformed_samples_ranges_and_missing_identities() {
    let parameter = mapping(OscControlAction::VideoIsfParam);
    for value in [
        OscType::Float(f32::NAN),
        OscType::Float(f32::INFINITY),
        OscType::Double(f64::MAX),
        OscType::String("1".to_string()),
    ] {
        assert!(mapped(&parameter, vec![value]).is_none());
    }
    assert!(mapped(&parameter, vec![]).is_none());
    for (low, high) in [(f32::NAN, 1.0), (0.0, f32::INFINITY), (-f32::MAX, f32::MAX)] {
        let mut invalid = parameter.clone();
        invalid.low = low;
        invalid.high = high;
        assert!(mapped(&invalid, vec![OscType::Float(0.5)]).is_none());
    }
    let mut no_layer = parameter.clone();
    no_layer.layer_id = None;
    let mut no_stage = parameter.clone();
    no_stage.isf_stage_id = None;
    let mut no_name = parameter;
    no_name.attribute = Some(" ".to_string());
    let mut no_slot = mapping(OscControlAction::VideoClipSlotTrigger);
    no_slot.clip_slot_id = None;
    for invalid in [no_layer, no_stage, no_name, no_slot] {
        assert!(mapped(&invalid, vec![OscType::Float(1.0)]).is_none());
    }
}
#[test]
fn network_perform_switches_preserve_false_and_distinguish_pulses() {
    let stage = mapping(OscControlAction::VideoIsfStageEnabled);
    let off = mapped(&stage, vec![OscType::Bool(false)]).unwrap();
    assert!(!off.active && !off.toggle);
    assert_eq!(off.position, 0.0);
    let on = mapped(&stage, vec![OscType::Bool(true)]).unwrap();
    assert!(on.active && !on.toggle);
    let pulse = mapped(&stage, vec![]).unwrap();
    assert!(pulse.active && pulse.toggle);
    for action in [
        OscControlAction::VideoClipSlotTrigger,
        OscControlAction::VideoTake,
        OscControlAction::VideoLayerStop,
    ] {
        let trigger = mapping(action);
        assert!(mapped(&trigger, vec![]).unwrap().active);
        assert!(mapped(&trigger, vec![OscType::Bool(false)]).is_none());
        assert!(mapped(&trigger, vec![OscType::Float(-1.0)]).is_none());
    }
}
#[test]
fn network_perform_learned_address_never_also_runs_a_builtin_command() {
    let mut take = mapping(OscControlAction::VideoTake);
    take.address = "/syndocal/video/master".to_string();
    let packet = OscPacket::Message(OscMessage {
        addr: take.address.clone(),
        args: vec![OscType::Float(1.0)],
    });
    assert!(matches!(
        events_from_packet_with_mappings(&packet, &[take]).as_slice(),
        [OscInputEvent::RendererRouted(_)]
    ));
    let mut invalid = mapping(OscControlAction::VideoClipSlotTrigger);
    invalid.address = "/syndocal/video/master".to_string();
    invalid.clip_slot_id = None;
    assert!(events_from_packet_with_mappings(&packet, &[invalid]).is_empty());
}
fn dmx_mapping(action: OscControlAction) -> DmxControlMapping {
    let mut json = serde_json::to_value(mapping(action)).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("address");
    object.insert("universe".to_string(), serde_json::json!(0));
    object.insert("channel".to_string(), serde_json::json!(25));
    serde_json::from_value(json).unwrap()
}
#[test]
fn network_perform_dmx_triggers_require_a_new_positive_edge() {
    for action in [
        OscControlAction::VideoClipSlotTrigger,
        OscControlAction::VideoTake,
        OscControlAction::VideoLayerStop,
    ] {
        let mapping = dmx_mapping(action);
        let mut previous = [0; 512];
        for (value, expected) in [(80, 1), (160, 0), (160, 0), (0, 0), (200, 1)] {
            let mut values = previous;
            values[24] = value;
            let events = control_events_from_changed_frame(
                0,
                &values,
                Some(&previous),
                std::slice::from_ref(&mapping),
            );
            assert_eq!(
                events.len(),
                expected,
                "action {:?}, value {value}",
                mapping.action
            );
            for event in events {
                let OscInputEvent::RendererRouted(input) = event else {
                    panic!("direct engine event");
                };
                assert!(input.active && !input.toggle);
                assert_eq!(input.layer_id, mapping.layer_id);
                assert_eq!(input.clip_slot_id, mapping.clip_slot_id);
            }
            previous = values;
        }
    }
}
#[test]
fn network_perform_dmx_faders_preserve_stable_ids_range_and_zero() {
    let mut parameter = dmx_mapping(OscControlAction::VideoIsfParam);
    parameter.low = 1.0;
    parameter.high = -1.0;
    let mut previous = None;
    for level in [0, 64, 128, 255] {
        let mut values = [0; 512];
        values[24] = level;
        let events = control_events_from_changed_frame(
            0,
            &values,
            previous.as_ref(),
            std::slice::from_ref(&parameter),
        );
        let [OscInputEvent::RendererRouted(input)] = events.as_slice() else {
            panic!("one renderer event expected: {events:?}");
        };
        assert_eq!(input.layer_id, Some(4));
        assert_eq!(input.isf_stage_id, Some(VideoEffectStageId(9)));
        assert_eq!(input.control.as_deref(), Some("amount"));
        assert_eq!(input.position, f32::from(level) / 255.0);
        assert_eq!(input.value, 1.0 - 2.0 * input.position);
        assert_eq!(input.active, level > 0);
        assert!(!input.toggle);
        assert!(control_events_from_changed_frame(
            0,
            &values,
            Some(&values),
            std::slice::from_ref(&parameter)
        )
        .is_empty());
        previous = Some(values);
    }
    let json = serde_json::to_value(&parameter).unwrap();
    assert_eq!(json["isf_stage_id"], serde_json::json!(9));
    assert_eq!(
        serde_json::from_value::<DmxControlMapping>(json).unwrap(),
        parameter
    );
}
#[test]
fn network_perform_legacy_mappings_keep_their_serialized_shape() {
    let old_osc = serde_json::json!({
        "address": "/master", "action": "VideoMaster", "fixture_id": null,
        "attribute": null, "group_id": null, "cue_id": null, "layer_id": null,
        "output_id": null, "video_param": null, "cue_point_index": null,
        "duration_ms": null, "low": 0.0, "high": 1.0
    });
    let osc: OscControlMapping = serde_json::from_value(old_osc.clone()).unwrap();
    assert!(osc.clip_slot_id.is_none() && osc.isf_stage_id.is_none());
    assert_eq!(serde_json::to_value(osc).unwrap(), old_osc);
    let mut old_dmx = old_osc;
    let object = old_dmx.as_object_mut().unwrap();
    object.remove("address");
    object.insert("universe".to_string(), serde_json::json!(2));
    object.insert("channel".to_string(), serde_json::json!(25));
    let dmx: DmxControlMapping = serde_json::from_value(old_dmx.clone()).unwrap();
    assert!(dmx.clip_slot_id.is_none() && dmx.isf_stage_id.is_none());
    assert_eq!(serde_json::to_value(dmx).unwrap(), old_dmx);
}

const VIDEO_GROUP_ACTIONS: [&str; 5] = [
    "VideoGroupOpacity",
    "VideoGroupSolo",
    "VideoGroupBypass",
    "VideoGroupBlendMode",
    "VideoGroupEnabled",
];

fn video_group_mapping(action: &str) -> OscControlMapping {
    serde_json::from_value(serde_json::json!({
        "address": "/groups/control", "action": action,
        "video_group_id": 41, "low": 0.25, "high": 0.75
    }))
    .unwrap()
}

fn video_group_dmx_mapping(action: &str) -> DmxControlMapping {
    let mut json = serde_json::to_value(video_group_mapping(action)).unwrap();
    let object = json.as_object_mut().unwrap();
    object.remove("address");
    object.insert("universe".to_string(), serde_json::json!(2));
    object.insert("channel".to_string(), serde_json::json!(25));
    serde_json::from_value(json).unwrap()
}

#[test]
fn network_video_group_all_actions_forward_only_stable_group_identity() {
    for action in VIDEO_GROUP_ACTIONS {
        let mapping = video_group_mapping(action);
        for position in [0.0f32, 0.5, 1.0] {
            let input = mapped(&mapping, vec![OscType::Float(position)]).unwrap();
            assert_eq!(serde_json::to_value(&input.action).unwrap(), action);
            assert_eq!(input.video_group_id, Some(protocol::VideoLayerGroupId(41)));
            assert_eq!(input.layer_id, None);
            assert_eq!(input.clip_slot_id, None);
            assert_eq!(input.isf_stage_id, None);
            assert_eq!(input.control, None);
            assert_eq!(input.position, position);
            assert_eq!(input.value, 0.25 + 0.5 * position);
            assert_eq!(input.active, position > 0.0);
            assert!(!input.toggle);
        }
    }
}

#[test]
fn network_video_group_switches_distinguish_pulses_from_explicit_false() {
    for action in ["VideoGroupSolo", "VideoGroupBypass", "VideoGroupEnabled"] {
        let mapping = video_group_mapping(action);
        let pulse = mapped(&mapping, vec![]).unwrap();
        assert!(pulse.toggle && pulse.active);
        let off = mapped(&mapping, vec![OscType::Bool(false)]).unwrap();
        assert!(!off.toggle && !off.active);
        let on = mapped(&mapping, vec![OscType::Bool(true)]).unwrap();
        assert!(!on.toggle && on.active);
    }
    for action in ["VideoGroupOpacity", "VideoGroupBlendMode"] {
        assert!(mapped(&video_group_mapping(action), vec![]).is_none());
    }
}

#[test]
fn network_video_group_rejects_missing_unsafe_ids_and_non_finite_samples() {
    for action in VIDEO_GROUP_ACTIONS {
        for id in [
            None,
            Some(protocol::VideoLayerGroupId(0)),
            Some(protocol::VideoLayerGroupId(9_007_199_254_740_992)),
        ] {
            let mut mapping = video_group_mapping(action);
            mapping.video_group_id = id;
            assert!(
                mapped(&mapping, vec![OscType::Float(1.0)]).is_none(),
                "{action}/{id:?}"
            );
        }
        for value in [
            OscType::Float(f32::NAN),
            OscType::Float(f32::INFINITY),
            OscType::Double(f64::MAX),
            OscType::String("1".to_string()),
        ] {
            assert!(mapped(&video_group_mapping(action), vec![value]).is_none());
        }
    }
}

#[test]
fn network_video_group_dmx_forwards_levels_and_zero_without_repeat_or_toggle() {
    for action in VIDEO_GROUP_ACTIONS {
        let mapping = video_group_dmx_mapping(action);
        let mut previous = None;
        for value in [0u8, 128, 255, 0] {
            let mut values = [0u8; 512];
            values[24] = value;
            let events = control_events_from_changed_frame(
                2,
                &values,
                previous.as_ref(),
                std::slice::from_ref(&mapping),
            );
            let [OscInputEvent::RendererRouted(input)] = events.as_slice() else {
                panic!("expected exactly one renderer-routed group input: {action}/{value}: {events:?}");
            };
            assert_eq!(input.video_group_id, Some(protocol::VideoLayerGroupId(41)));
            assert_eq!(input.position, f32::from(value) / 255.0);
            assert_eq!(input.value, 0.25 + 0.5 * input.position);
            assert_eq!(input.active, value > 0);
            assert!(!input.toggle);
            assert!(control_events_from_changed_frame(
                2,
                &values,
                Some(&values),
                std::slice::from_ref(&mapping)
            )
            .is_empty());
            assert!(control_events_from_changed_frame(
                1,
                &values,
                None,
                std::slice::from_ref(&mapping)
            )
            .is_empty());
            previous = Some(values);
        }
    }
}
