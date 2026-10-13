//! Explicit scene-editor values are validated before touching a capture snapshot.
use protocol::{AttributeValueSummary, EngineSnapshot, ProgrammerValueSummary};
use std::collections::{HashMap, HashSet};

pub fn apply(
    snapshot: &mut EngineSnapshot,
    fixture_ids: &HashSet<u64>,
    values: Vec<ProgrammerValueSummary>,
) -> Result<(), String> {
    if values.is_empty() || values.len() > 65_536 {
        return Err("Scene edit requires between 1 and 65536 attribute values".into());
    }
    let mut by_fixture: HashMap<u64, HashMap<String, u16>> = HashMap::new();
    for value in values {
        if !fixture_ids.contains(&value.fixture_id) {
            return Err("Scene edit value is outside its capture scope".into());
        }
        let attributes = by_fixture.entry(value.fixture_id).or_default();
        if attributes.insert(value.attribute, value.value).is_some() {
            return Err("Scene edit contains a duplicate attribute".into());
        }
    }
    for fixture_id in fixture_ids {
        let fixture = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == *fixture_id)
            .ok_or("Scene edit fixture no longer exists")?;
        let attributes = by_fixture
            .get(fixture_id)
            .ok_or("Scene edit is missing a fixture")?;
        if attributes.len() != fixture.controls.len()
            || fixture
                .controls
                .iter()
                .any(|control| !attributes.contains_key(&control.attribute))
        {
            return Err("Scene edit requires the fixture's exact attribute set".into());
        }
    }
    // All identities and complete attribute sets are admitted before mutation.
    for fixture in &mut snapshot.fixtures {
        if let Some(attributes) = by_fixture.get(&fixture.id) {
            fixture.attribute_values = fixture
                .controls
                .iter()
                .map(|control| AttributeValueSummary {
                    attribute: control.attribute.clone(),
                    value: attributes[&control.attribute],
                })
                .collect();
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> protocol::PatchedFixtureSummary {
        serde_json::from_value(serde_json::json!({
            "id": 1, "label": "Fixture", "profile_name": "RGB", "manufacturer": "Test",
            "mode_name": "RGB", "universe": 1, "address": 1, "group_ids": [],
            "position": {"x": 0.0, "y": 0.0, "z": 0.0}, "rotation": {"pitch": 0.0, "yaw": 0.0, "roll": 0.0},
            "controls": (["Dimmer", "Red"].map(|attribute| serde_json::json!({
                "attribute": attribute, "channel_name": attribute, "geometry": null,
                "offsets": [1], "resolution": "EightBit", "default_value": 0
            }))),
            "attribute_values": [{"attribute": "Dimmer", "value": 100}, {"attribute": "Red", "value": 60000}],
            "highlighted": false, "soloed": false, "parked": false
        })).unwrap()
    }
    fn values(red: &str) -> Vec<ProgrammerValueSummary> {
        vec![
            ProgrammerValueSummary {
                fixture_id: 1,
                attribute: "Dimmer".into(),
                value: 42000,
            },
            ProgrammerValueSummary {
                fixture_id: 1,
                attribute: red.into(),
                value: 5000,
            },
        ]
    }
    #[test]
    fn explicit_scene_values_replace_capture_without_reading_the_running_look() {
        let mut snapshot = EngineSnapshot::default();
        snapshot.fixtures.push(fixture());
        apply(&mut snapshot, &HashSet::from([1]), values("Red")).unwrap();
        assert_eq!(snapshot.fixtures[0].attribute_values[0].value, 42000);
        assert_eq!(snapshot.fixtures[0].attribute_values[1].value, 5000);
    }
    #[test]
    fn invalid_duplicate_incomplete_or_out_of_scope_values_leave_snapshot_unchanged() {
        for (ids, edits) in [
            (HashSet::from([1]), values("Unknown")),
            (HashSet::from([1]), values("Dimmer")),
            (HashSet::from([2]), values("Red")),
            (HashSet::from([1]), vec![values("Red")[0].clone()]),
        ] {
            let mut snapshot = EngineSnapshot::default();
            snapshot.fixtures.push(fixture());
            let before = snapshot.clone();
            assert!(apply(&mut snapshot, &ids, edits).is_err());
            assert_eq!(snapshot, before);
        }
    }
}
