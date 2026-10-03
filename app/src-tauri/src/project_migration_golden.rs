//! Independently authored full JSON expectations for the in-memory corpus.
//! No production defaults or normalizers construct the expected images.
use serde_json::{json, Value};

const PHASE1: &str = include_str!("../../../qa/migration/phase1-current-canonical.json");
const EMPTY: &str = include_str!("../../../qa/migration/empty-current-canonical.json");
const MANIFEST: &str = include_str!("../../../qa/migration/whole-project-oracles.json");

fn read(text: &str) -> Value {
    crate::project_file_json::parse_project_json(text).expect("whole-project golden JSON")
}

pub(super) fn phase1_expected() -> Value {
    read(PHASE1)
}

fn scalar_leaves(value: &Value) -> usize {
    match value {
        Value::Object(fields) => fields.values().map(scalar_leaves).sum(),
        Value::Array(values) => values.iter().map(scalar_leaves).sum(),
        _ => 1,
    }
}

fn child_path(parent: &str, key: &str) -> String {
    format!("{parent}/{}", key.replace('~', "~0").replace('/', "~1"))
}

// Compare all keys, array positions and leaves, including number types. No
// ignored paths or float tolerance. A path-only error avoids dumping a whole
// project into the log. Integer identity is never rounded through f64.
fn first_difference(actual: &Value, expected: &Value, path: &str) -> Option<String> {
    match (actual, expected) {
        (Value::Object(actual), Value::Object(expected)) => {
            for key in expected.keys() {
                if !actual.contains_key(key) {
                    return Some(format!("missing {}", child_path(path, key)));
                }
            }
            for key in actual.keys() {
                if !expected.contains_key(key) {
                    return Some(format!("unexpected {}", child_path(path, key)));
                }
            }
            expected.iter().find_map(|(key, value)| {
                first_difference(&actual[key], value, &child_path(path, key))
            })
        }
        (Value::Array(actual), Value::Array(expected)) => {
            if actual.len() != expected.len() {
                return Some(format!("array length at {path}"));
            }
            actual
                .iter()
                .zip(expected)
                .enumerate()
                .find_map(|(index, (actual, expected))| {
                    first_difference(actual, expected, &format!("{path}/{index}"))
                })
        }
        _ if actual == expected => None,
        _ => Some(format!("value/type at {path}")),
    }
}

pub(super) fn assert_whole(actual: &Value, expected: &Value, label: &str) {
    assert_eq!(
        first_difference(actual, expected, ""),
        None,
        "{label}: whole-project golden mismatch"
    );
}

pub(super) fn assert_phase1_source_unchanged() {
    let manifest = read(MANIFEST);
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["phase1_source"], "samples/phase1-mini-show.sdc");
    assert_eq!(
        crate::sha256_hex(
            crate::PHASE1_SAMPLE_PROJECT_JSON
                .replace("\r\n", "\n")
                .as_bytes()
        ),
        manifest["phase1_source_sha256_lf"].as_str().unwrap(),
        "sample changed: independently review/author the oracle before accepting a new fixture"
    );
    assert_eq!(scalar_leaves(&super::sample()), 630);
    assert_eq!(manifest["phase1_source_scalar_leaves"], 630);
    assert_eq!(scalar_leaves(&phase1_expected()), 858);
    assert_eq!(manifest["phase1_expected_scalar_leaves"], 858);
}

#[test]
fn migration_corpus_empty_new_project_whole_golden_load_save_reload() {
    let output = super::round_trip(&super::empty(), "empty new project");
    let expected = read(EMPTY);
    assert_eq!(scalar_leaves(&expected), 118);
    assert_eq!(read(MANIFEST)["empty_expected_scalar_leaves"], 118);
    assert_whole(&output, &expected, "empty new project");
}

#[test]
fn migration_corpus_current_projects_whole_golden_load_save_reload() {
    for (label, text) in [("current Phase 1", PHASE1), ("current empty", EMPTY)] {
        let expected = read(text);
        let output = super::round_trip(&expected, label);
        assert_whole(&output, &expected, label);
    }
}

#[test]
fn migration_corpus_whole_golden_detects_unselected_fields_and_topology() {
    let expected = phase1_expected();
    let mutations = [
        ("/snapshot/fixtures/0/geometries/1/matrix/7", json!(0.5)),
        (
            "/snapshot/video/outputs/0/mapping/corner_top_left_y",
            json!(0.5),
        ),
        ("/snapshot/cues/0/targets/0/values/1/value", json!(1234)),
        (
            "/snapshot/video/layers/0/clip_slots/0/cue_points/1/name",
            json!("Changed"),
        ),
    ];
    for (pointer, replacement) in mutations {
        let mut actual = expected.clone();
        *actual.pointer_mut(pointer).expect("oracle mutation target") = replacement;
        assert_eq!(
            first_difference(&actual, &expected, ""),
            Some(format!("value/type at {pointer}"))
        );
    }
    let mut missing = expected.clone();
    missing["snapshot"]["fixtures"][0]
        .as_object_mut()
        .unwrap()
        .remove("manufacturer");
    assert_eq!(
        first_difference(&missing, &expected, ""),
        Some("missing /snapshot/fixtures/0/manufacturer".to_string())
    );
    let mut extra = expected.clone();
    extra["snapshot"]["video"]["media_assets"][0]["availability"] = json!("Must not persist");
    assert_eq!(
        first_difference(&extra, &expected, ""),
        Some("unexpected /snapshot/video/media_assets/0/availability".to_string())
    );
    let mut reordered = expected.clone();
    reordered["snapshot"]["stage_objects"]
        .as_array_mut()
        .unwrap()
        .swap(0, 1);
    assert!(first_difference(&reordered, &expected, "").is_some());
    let mut shortened = expected.clone();
    shortened["custom_profiles"][0]["geometries"]
        .as_array_mut()
        .unwrap()
        .pop();
    assert_eq!(
        first_difference(&shortened, &expected, ""),
        Some("array length at /custom_profiles/0/geometries".to_string())
    );
    assert!(first_difference(&json!(u64::MAX), &json!(u64::MAX - 1), "id").is_some());
}

#[test]
fn migration_corpus_runtime_image_does_not_change_whole_authored_golden() {
    let mut input = super::sample();
    input["snapshot"]["clock"]["beat_phase"] = json!(0.75);
    input["snapshot"]["clock"]["beat_counter"] = json!(999);
    input["snapshot"]["clock"]["tap_count"] = json!(8);
    input["snapshot"]["clock"]["external_sync_age_ms"] = json!(250);
    input["snapshot"]["clock"]["external_sync_locked"] = json!(true);
    input["snapshot"]["timeline"]["playing"] = json!(true);
    input["snapshot"]["telemetry"]["frame_counter"] = json!(123456);
    input["snapshot"]["programmer"] = json!({"enabled": true, "blind": true,
        "values": [{"fixture_id": 1, "attribute": "Dimmer", "value": 32768}],
        "dmx_previews": [{"universe": 0, "values": [1, 2, 3]}]});
    input["snapshot"]["dmx_preview"] = json!([1, 2, 3]);
    input["snapshot"]["dmx_previews"] = json!([{"universe": 0, "values": [1, 2, 3]}]);
    input["snapshot"]["submasters"][0]["strobe_hz"] = json!(8.0);
    input["snapshot"]["submasters"][0]["strobe_fixture_count"] = json!(1);
    input["snapshot"]["video"]["auto_vj"] = json!({"status": {"mode": "Armed",
        "armed": true, "hold": false, "show_revision": 7, "action_sequence": 3,
        "last_consumed_boundary": 4, "next_boundary_beat": 8, "last_action": null,
        "fault": null}});
    let output = super::round_trip(&input, "runtime-contaminated Phase 1");
    assert_whole(&output, &phase1_expected(), "runtime-stripped Phase 1");
}
