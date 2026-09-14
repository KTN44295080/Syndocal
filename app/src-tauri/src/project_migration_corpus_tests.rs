//! Bounded, in-memory migration corpus using the production preparation/writer.
//! No EngineHandle, AppState, device, network, or project-file publication is used.

use crate::{
    prepare_project_load, project_and_control_mappings_from_value,
    project_file_for_save_from_parts, project_json_for_write_with_control_mappings_and_dj,
    PreparedProjectLoad, APP_NAME, PHASE1_SAMPLE_PROJECT_JSON, PROJECT_FILE_VERSION,
};
use protocol::{EngineSnapshot, ProjectFile};
use serde_json::{json, Value};
use std::panic::{catch_unwind, AssertUnwindSafe};

const GOLDEN: &str = include_str!("../../../qa/migration/phase1-project-expectations.json");
const GENERATED_CASES: usize = 128;
const GENERATOR_SEED: u64 = 0x5344_435f_2026_0913;
const HOSTILE_BYTE_CASES: usize = 4096;
const HOSTILE_MAX_BYTES: usize = 2048;

fn sample() -> Value {
    serde_json::from_str(PHASE1_SAMPLE_PROJECT_JSON).expect("checked-in sample JSON")
}

fn empty() -> Value {
    serde_json::to_value(ProjectFile {
        version: PROJECT_FILE_VERSION,
        app: APP_NAME.to_string(),
        operator_policy: None,
        custom_profiles: Vec::new(),
        fixture_groups: Vec::new(),
        snapshot: EngineSnapshot::default(),
    })
    .expect("empty project serialization")
}

fn prepare(input: &Value) -> Result<PreparedProjectLoad, String> {
    let (project, mappings) = project_and_control_mappings_from_value(input.clone())?;
    assert_no_media_io(input)?;
    prepare_project_load(
        project,
        mappings,
        "In-memory migration corpus".to_string(),
        None,
    )
}

// The production preparation seam can inspect/hash File and StillImage assets.
// Keep this corpus in-memory even if its checked-in sample later changes. The
// broad path check also prevents newly added media/audio path fields from
// silently expanding the test's side-effect boundary. Embedded fixture
// profile_source_path is separate: preparation uses its embedded profile image.
fn assert_no_media_io(value: &Value) -> Result<(), String> {
    match value {
        Value::Object(fields) => {
            if fields
                .get("kind")
                .and_then(Value::as_str)
                .is_some_and(|kind| matches!(kind, "File" | "StillImage"))
                || fields.get("path").is_some_and(|path| !path.is_null())
            {
                return Err("Migration corpus forbids file/media I/O candidates".to_string());
            }
            for child in fields.values() {
                assert_no_media_io(child)?;
            }
        }
        Value::Array(values) => {
            for child in values {
                assert_no_media_io(child)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[test]
fn migration_corpus_rejects_file_media_before_production_preparation() {
    assert_no_media_io(&sample()).expect("sample must remain free of media I/O");
    for candidate in [
        json!({"nested": [{"kind": "File", "path": null}]}),
        json!({"nested": [{"kind": "StillImage", "path": null}]}),
        json!({"path": "C:/must-not-open/corpus.mp4"}),
        json!({"audio": {"path": "//must-not-contact/share/corpus.wav"}}),
    ] {
        assert!(assert_no_media_io(&candidate)
            .unwrap_err()
            .contains("Migration corpus forbids file/media I/O"));
    }
    let mut candidate = sample();
    candidate["snapshot"]["video"]["layers"][0]["source"]["kind"] = json!("File");
    candidate["snapshot"]["video"]["layers"][0]["source"]["path"] =
        json!("C:/must-not-open/corpus.mp4");
    assert!(prepare(&candidate)
        .err()
        .expect("guard must reject before prepare")
        .contains("Migration corpus forbids file/media I/O"));
}

fn canonical_save(input: &Value) -> Result<(Value, String), String> {
    let prepared = prepare(input)?;
    let project = project_file_for_save_from_parts(prepared.snapshot, &prepared.ancillary);
    let mappings = prepared.mappings;
    let bytes = project_json_for_write_with_control_mappings_and_dj(
        &project,
        mappings.midi_mappings,
        mappings.osc_mappings,
        mappings.dmx_mappings,
        mappings.dj_track_triggers,
    )?;
    let value = serde_json::from_str(&bytes).map_err(|error| error.to_string())?;
    Ok((value, bytes))
}

fn round_trip(input: &Value, label: &str) -> Value {
    let original = serde_json::to_vec(input).expect("input serialization");
    let (first, first_bytes) = canonical_save(input)
        .unwrap_or_else(|error| panic!("{label}: initial load/save failed: {error}"));
    let (second, second_bytes) = canonical_save(&first)
        .unwrap_or_else(|error| panic!("{label}: canonical reload/save failed: {error}"));
    assert_eq!(first, second, "{label}: canonical semantics drifted");
    assert_eq!(
        first_bytes, second_bytes,
        "{label}: second save is not byte-idempotent"
    );
    assert_eq!(
        serde_json::to_vec(input).unwrap(),
        original,
        "{label}: input mutated"
    );
    second
}

#[test]
fn migration_corpus_phase1_golden_load_save_reload() {
    let output = round_trip(&sample(), "phase1-mini-show");
    let expected: Value = serde_json::from_str(GOLDEN).expect("golden expectations JSON");
    assert_eq!(expected["schema_version"], 1);
    assert_eq!(expected["source"], "samples/phase1-mini-show.sdc");
    for (pointer, value) in expected["exact"].as_object().expect("exact expectations") {
        assert_eq!(
            output.pointer(pointer),
            Some(value),
            "golden mismatch at {pointer}"
        );
    }
    for (pointer, length) in expected["array_lengths"]
        .as_object()
        .expect("array lengths")
    {
        let actual = output
            .pointer(pointer)
            .and_then(Value::as_array)
            .unwrap_or_else(|| panic!("golden array is missing at {pointer}"));
        assert_eq!(
            actual.len() as u64,
            length.as_u64().unwrap(),
            "length at {pointer}"
        );
    }
}

#[test]
fn migration_corpus_legacy_defaults_and_unknown_fields() {
    let mut input = empty();
    input.as_object_mut().unwrap().remove("custom_profiles");
    input.as_object_mut().unwrap().remove("fixture_groups");
    for key in [
        "node_graphs",
        "stage_map",
        "stage_map_presets",
        "stage_objects",
        "dmx_previews",
        "palettes",
        "playback_executors",
        "playback_master",
    ] {
        input["snapshot"].as_object_mut().unwrap().remove(key);
    }
    input["future_optional_metadata"] = json!({"label": "追加情報", "enabled": true});
    input["snapshot"]["future_optional_metadata"] = json!([1, 2, 3]);
    let output = round_trip(&input, "legacy defaults/unknown fields");
    assert_eq!(output["custom_profiles"], json!([]));
    assert_eq!(output["fixture_groups"], json!([]));
    assert_eq!(output["snapshot"]["node_graphs"], json!([]));
    assert_eq!(output["snapshot"]["playback_master"], 1.0);
    assert_eq!(
        output["snapshot"]["playback_executors"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(output.get("future_optional_metadata").is_none());
    assert!(output["snapshot"].get("future_optional_metadata").is_none());
}

#[test]
fn migration_corpus_control_mappings_survive_canonical_save() {
    let mut input = sample();
    input["midi_mappings"] = json!([{
        "channel": 0, "message": "ControlChange", "number": 74,
        "action": "LightingMaster", "low": 0.0, "high": 1.0
    }]);
    input["osc_mappings"] = json!([{
        "address": "/corpus/go", "action": "TriggerNextCue", "low": 0.0, "high": 1.0
    }]);
    input["dmx_mappings"] = json!([{
        "universe": 0, "channel": 25, "action": "FixtureAttribute",
        "fixture_id": 1, "attribute": "ColorRed", "low": 0.0, "high": 65535.0
    }]);
    let before = project_and_control_mappings_from_value(input.clone())
        .unwrap()
        .1;
    let output = round_trip(&input, "MIDI/OSC/DMX mappings");
    let after = project_and_control_mappings_from_value(output).unwrap().1;
    assert_eq!(
        before, after,
        "canonical writer lost or changed control mappings"
    );
    assert_eq!(after.midi_mappings.len(), 1);
    assert_eq!(after.osc_mappings.len(), 1);
    assert_eq!(after.dmx_mappings.len(), 1);
}

#[test]
fn migration_corpus_version_rejection_precedes_migration() {
    let versions = [
        Value::Null,
        json!(0),
        json!(2),
        json!(u32::MAX),
        json!(u64::MAX),
        json!("1"),
        json!(-1),
        json!(1.5),
    ];
    for version in versions {
        let mut input = empty();
        input["version"] = version.clone();
        // This malformed migration candidate must never supersede version admission.
        input["migration_probe"] = json!({"spatial_pattern": {"recipe": null}});
        let error = prepare(&input)
            .err()
            .expect("unsupported version must be rejected");
        assert!(
            error.contains("Project version") || error.contains("Unsupported project version"),
            "version {version} reached migration instead of admission: {error}"
        );
    }
    let mut missing = empty();
    missing.as_object_mut().unwrap().remove("version");
    assert!(prepare(&missing).err().unwrap().contains("Project version"));
}

#[test]
fn migration_corpus_duplicate_ids_fail_without_mutating_candidates() {
    for pointer in [
        "/snapshot/fixtures",
        "/snapshot/cues",
        "/snapshot/timeline/events",
        "/snapshot/video/layers",
        "/snapshot/stage_objects",
    ] {
        let mut input = sample();
        let list = input.pointer_mut(pointer).unwrap().as_array_mut().unwrap();
        list.push(list[0].clone());
        let original = serde_json::to_vec(&input).unwrap();
        let error = prepare(&input)
            .err()
            .unwrap_or_else(|| panic!("duplicate IDs accepted at {pointer}"));
        assert!(!error.is_empty(), "duplicate rejection needs a diagnostic");
        assert_eq!(
            serde_json::to_vec(&input).unwrap(),
            original,
            "mutated {pointer}"
        );
    }
}

#[test]
fn migration_corpus_corrupt_reference_and_value_matrix() {
    let mutations = [
        ("/app", json!("Other")),
        ("/snapshot/fixtures/0/address", json!(513)),
        ("/snapshot/cues/0/targets/0/fixture_id", json!(999999)),
        ("/snapshot/cues/0/fade_ms", json!(-1)),
        (
            "/snapshot/video/layers/0/source/kind",
            json!("UnsupportedSource"),
        ),
        ("/snapshot/video/outputs/0/composition_id", json!(999999)),
        ("/snapshot/timeline/events/0/cue_id", json!(999999)),
        ("/snapshot/stage_objects/0/width", json!(-1.0)),
    ];
    for (pointer, value) in mutations {
        let mut input = sample();
        *input.pointer_mut(pointer).expect("mutation target exists") = value;
        let original = serde_json::to_vec(&input).unwrap();
        let error = prepare(&input)
            .err()
            .unwrap_or_else(|| panic!("corrupt input accepted at {pointer}"));
        assert!(
            !error.is_empty(),
            "corrupt input needs a diagnostic at {pointer}"
        );
        assert_eq!(serde_json::to_vec(&input).unwrap(), original);
    }
}

#[test]
fn migration_corpus_bounded_truncation_utf8_and_depth_inputs() {
    let bytes = PHASE1_SAMPLE_PROJECT_JSON.trim_end().as_bytes();
    let mut truncations = 0;
    for cut in (0..bytes.len())
        .step_by(97)
        .chain(std::iter::once(bytes.len() - 1))
    {
        assert!(
            serde_json::from_slice::<Value>(&bytes[..cut]).is_err(),
            "truncated JSON accepted at byte {cut}"
        );
        truncations += 1;
    }
    for malformed in [
        b"{\"x\":\"\xff\"}".as_slice(),
        b"{\"x\":NaN}",
        b"{\"x\":Infinity}",
        b"{\"x\":1e9999}",
        b"{\"x\":0,}",
    ] {
        assert!(serde_json::from_slice::<Value>(malformed).is_err());
    }
    for depth in [128, 256, 512] {
        let text = format!("{}0{}", "[".repeat(depth), "]".repeat(depth));
        assert!(
            serde_json::from_str::<Value>(&text).is_err(),
            "parser recursion limit not enforced at depth {depth}"
        );
    }
    eprintln!(
        "migration corpus: {truncations} truncations, 5 malformed byte/number cases, 3 depth cases"
    );
}

#[test]
fn migration_corpus_seeded_hostile_json_bytes_are_bounded_and_panic_free() {
    let mut seed = GENERATOR_SEED ^ 0xA11C_E5ED_5AFE_2026;
    let mut parsed = 0;
    let mut rejected = 0;
    let mut prepared = 0;
    for index in 0..HOSTILE_BYTE_CASES {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let bytes = match index {
            0 => b"null".to_vec(),
            1 => b"[]".to_vec(),
            2 => b"{}".to_vec(),
            3 => b"0".to_vec(),
            4 => b"true".to_vec(),
            _ => {
                let length = 1 + (seed as usize % HOSTILE_MAX_BYTES);
                let mut random_bytes = Vec::with_capacity(length);
                for offset in 0..length {
                    seed = seed
                        .wrapping_mul(2862933555777941757)
                        .wrapping_add(3037000493);
                    random_bytes.push((seed >> ((offset % 8) * 8)) as u8);
                }
                random_bytes
            }
        };
        let parsed_value = match serde_json::from_slice::<Value>(&bytes) {
            Ok(value) => {
                parsed += 1;
                value
            }
            Err(_) => {
                rejected += 1;
                continue;
            }
        };
        let original = serde_json::to_vec(&parsed_value).expect("parsed JSON must reserialize");
        let result = catch_unwind(AssertUnwindSafe(|| prepare(&parsed_value)));
        assert!(
            result.is_ok(),
            "hostile JSON case {index} panicked after parsing"
        );
        assert_eq!(
            serde_json::to_vec(&parsed_value).unwrap(),
            original,
            "hostile JSON case {index} mutated its input"
        );
        if result.unwrap().is_ok() {
            prepared += 1;
        }
    }
    assert!(
        rejected > 0,
        "hostile corpus must exercise parser rejection"
    );
    assert!(
        parsed > 0,
        "hostile corpus must exercise valid JSON handling"
    );
    eprintln!(
        "migration hostile corpus: seed={GENERATOR_SEED:#x}, {HOSTILE_BYTE_CASES} cases, {rejected} parser-rejected, {parsed} parsed, {prepared} prepared, max_bytes={HOSTILE_MAX_BYTES}"
    );
}

#[test]
fn migration_corpus_seeded_semantic_round_trip_property() {
    let mut seed = GENERATOR_SEED;
    for index in 0..GENERATED_CASES {
        seed = seed
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut input = sample();
        let label = format!("移行-{index}-{:08x}-é-😀", seed as u32);
        input["snapshot"]["fixtures"][0]["label"] = json!(label);
        input["snapshot"]["fixtures"][0]["address"] = json!(1 + seed % 504);
        input["snapshot"]["clock"]["bpm"] = json!((90 + seed % 90) as f64);
        input["snapshot"]["clock"]["beat_counter"] = json!(seed % 10000);
        input["snapshot"]["telemetry"]["frame_counter"] = json!(seed % 10000);
        input["future_optional_metadata"] = json!({"index": index, "nested": [true, null]});
        let output = round_trip(&input, &format!("seed={GENERATOR_SEED:#x}, case={index}"));
        assert_eq!(output["snapshot"]["fixtures"][0]["label"], json!(label));
        assert_eq!(
            output["snapshot"]["fixtures"][0]["address"],
            json!(1 + seed % 504)
        );
        assert_eq!(
            output["snapshot"]["clock"]["bpm"],
            json!((90 + seed % 90) as f64)
        );
        assert_eq!(output["snapshot"]["clock"]["beat_counter"], 0);
        assert_eq!(output["snapshot"]["telemetry"]["frame_counter"], 0);
    }
    eprintln!(
        "migration corpus: seed={GENERATOR_SEED:#x}, {GENERATED_CASES} semantic/idempotency cases"
    );
}

#[test]
fn migration_corpus_retired_dj_transition_is_observable_and_not_resaved() {
    let mut input = empty();
    input["dj_transition"] = json!({"enabled": true});
    let prepared = prepare(&input).expect("legacy mapping is intentionally discarded");
    assert!(prepared
        .result
        .warnings
        .iter()
        .any(|warning| warning.contains("Legacy dj_transition")));
    let output = round_trip(&input, "retired DJ mapping");
    assert!(output.get("dj_transition").is_none());
}

#[test]
fn migration_corpus_embedded_profile_unicode_paths_are_not_reopened() {
    for source_path in [
        "C:/Shows/照明/機材.GDTF",
        "c:/shows/照明/機材.gdtf",
        "/旧環境/照明/機材.gdtf",
    ] {
        let mut input = sample();
        input["custom_profiles"] = json!([]);
        input["snapshot"]["fixtures"][0]["profile_source_path"] = json!(source_path);
        let output = round_trip(&input, source_path);
        assert_eq!(
            output["snapshot"]["fixtures"][0]["profile_source_path"],
            "snapshot://fixture/1"
        );
        assert_eq!(
            output["snapshot"]["fixtures"][0]["controls"]
                .as_array()
                .unwrap()
                .len(),
            6
        );
        assert_eq!(
            output["snapshot"]["fixtures"][0]["geometries"]
                .as_array()
                .unwrap()
                .len(),
            3
        );
    }
}
