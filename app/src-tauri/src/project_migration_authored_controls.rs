//! Nonempty current-schema authoring corpus; no engine, device or file-media I/O.
use serde_json::{json, Value};

const GOLDEN: &str = include_str!("../../../qa/migration/authored-control-project.json");
const MANIFEST: &str = include_str!("../../../qa/migration/authored-control-project-oracle.json");

fn expected() -> Value {
    crate::project_file_json::parse_project_json(GOLDEN).expect("authored control project oracle")
}

fn assert_round_trip(input: &Value, label: &str) {
    let output = super::round_trip(input, label);
    super::whole_project_golden::assert_whole(&output, &expected(), label);
}

#[test]
fn migration_corpus_authored_control_whole_golden_load_save_reload() {
    let manifest = crate::project_file_json::parse_project_json(MANIFEST).unwrap();
    assert_eq!(manifest["scalar_leaves"], 1265);
    assert_eq!(manifest["production_writer_generation_calls"], 0);
    assert_eq!(
        crate::sha256_hex(GOLDEN.replace("\r\n", "\n").as_bytes()),
        manifest["reference_sha256_lf"].as_str().unwrap(),
        "independently review the fixture before updating its identity"
    );
    let input = expected();
    let (project, _) = crate::project_and_control_mappings_from_value(input.clone()).unwrap();
    protocol::validate_current_engine_snapshot_reference_integrity(&project.snapshot)
        .expect("fixture declares a real nonempty current Timeline bank");
    protocol::validate_project_file_reference_integrity(&project)
        .expect("fixture groups and every authored reference agree");
    assert_round_trip(&input, "nonempty current control project");
}

#[test]
fn migration_corpus_authored_node_graph_runtime_is_not_resaved() {
    let mut input = expected();
    input["snapshot"]["node_graphs"][0]["audio_runtime"] = json!([{
        "node_id": 14, "input_value": 0.75, "output_value": 0.5,
        "source_available": true, "safety_zeroed": false, "held": true,
        "feature_sequence": 987
    }]);
    assert_round_trip(&input, "nonempty graph runtime cleanup");
}

#[test]
fn migration_corpus_root_timeline_transport_is_not_resaved() {
    let mut input = expected();
    input["snapshot"]["timeline"]["playing"] = json!(true);
    input["snapshot"]["timeline"]["position_ms"] = json!(750);
    input["snapshot"]["timeline"]["count_in_remaining_ms"] = json!(1000);
    input["snapshot"]["timeline"]["audio_transport_revision"] = json!(777);
    assert_round_trip(&input, "root Timeline runtime cleanup");
}

#[test]
fn migration_corpus_bank_timeline_transport_is_not_resaved() {
    let mut input = expected();
    for (index, timeline) in input["snapshot"]["timeline_bank"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .enumerate()
    {
        timeline["playing"] = json!(true);
        timeline["position_ms"] = json!(500 + 1000 * index);
        timeline["count_in_remaining_ms"] = json!(1000);
        timeline["audio_transport_revision"] = json!(777);
    }
    assert_round_trip(&input, "bank Timeline runtime cleanup");
}

#[test]
fn migration_corpus_prepared_ingress_clears_root_and_bank_transport() {
    let mut input = expected();
    input["snapshot"]["timeline"]["playing"] = json!(true);
    input["snapshot"]["timeline"]["position_ms"] = json!(750);
    input["snapshot"]["timeline"]["count_in_remaining_ms"] = json!(1_000);
    input["snapshot"]["timeline"]["audio_transport_revision"] = json!(777);
    for timeline in input["snapshot"]["timeline_bank"].as_array_mut().unwrap() {
        timeline["playing"] = json!(true);
        timeline["position_ms"] = json!(500);
        timeline["count_in_remaining_ms"] = json!(1_000);
        timeline["audio_transport_revision"] = json!(777);
    }
    let prepared = super::prepare(&input).expect("runtime fields are not authored references");
    for timeline in
        std::iter::once(&prepared.snapshot.timeline).chain(prepared.snapshot.timeline_bank.iter())
    {
        assert!(!timeline.playing, "prepared load must not resume playback");
        assert_eq!(timeline.position_ms, 0);
        assert_eq!(timeline.count_in_remaining_ms, 0);
        assert_eq!(timeline.audio_transport_revision, 0);
    }
}

#[test]
fn migration_corpus_authored_control_corrupt_reference_matrix() {
    for (pointer, replacement) in [
        (
            "/snapshot/timeline_bank/0/label",
            json!("Conflicting active authored image"),
        ),
        ("/snapshot/timeline_bank/1/id", json!(1)),
        ("/snapshot/node_graphs/0/edges/0/to_node", json!(999)),
        (
            "/snapshot/node_graphs/0/nodes/2/output/fixture_ids/0",
            json!(999),
        ),
        (
            "/snapshot/touch_surface/pages/0/controls/1/binding/cue_id",
            json!(999),
        ),
        (
            "/snapshot/touch_surface/pages/1/controls/0/binding/group_id",
            json!("Missing"),
        ),
        ("/snapshot/touch_surface/pages/0/controls/2/x", json!(12)),
        ("/snapshot/cues/0/effect_targets/0/effect_id", json!(999)),
        (
            "/snapshot/video/layers/0/clip_slots/1/media_asset_id",
            json!(999),
        ),
    ] {
        let mut input = expected();
        *input
            .pointer_mut(pointer)
            .expect("authored corruption target") = replacement;
        let original = serde_json::to_vec(&input).unwrap();
        let error = super::prepare(&input)
            .err()
            .unwrap_or_else(|| panic!("corrupt current input accepted at {pointer}"));
        assert!(
            !error.is_empty(),
            "actionable failure required at {pointer}"
        );
        assert_eq!(
            serde_json::to_vec(&input).unwrap(),
            original,
            "failure mutated input at {pointer}"
        );
    }
}
