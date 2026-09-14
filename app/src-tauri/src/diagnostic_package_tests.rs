//! Deterministic, memory-only tests. No runtime snapshots, filesystem, devices,
//! subprocesses, clocks, or external services are needed to exercise this module.

use super::*;
use serde_json::json;
use std::io::Read;
use std::panic::{catch_unwind, AssertUnwindSafe};

fn source_entries() -> Vec<(&'static str, Vec<u8>)> {
    [
        json!({
            "version": 1,
            "app": "Syndocal",
            "app_version": env!("CARGO_PKG_VERSION"),
            "captured_at_unix_ms": 1_700_000_000_000_u64,
            "os": "windows",
            "arch": "x86_64",
            "current_project_path": "C:\\Users\\PRIVATE_PATH\\show.sdc",
            "application_update": {
                "endpoint": "https://user:PRIVATE_CREDENTIAL@example.invalid/update",
                "token": "PRIVATE_TOKEN"
            }
        }),
        json!({
            "fixtures": 42, "cues": 7, "effects": 3, "node_graphs": 2,
            "timeline_events": 10, "timeline_automations": 11,
            "timeline_video_automations": 12, "video_layers": 4,
            "video_outputs": 2, "dmx_outputs": 1,
            "PRIVATE_KEY_SECRET": {"password": "PRIVATE_VALUE_SECRET"},
            "pairing_pin": 123456, "label": "PRIVATE_SHOW_LABEL"
        }),
        json!({
            "version": 1, "captured_at_unix_ms": 1_700_000_000_000_u64,
            "fixture_count": 42, "cue_count": 7, "effect_count": 3,
            "node_graph_count": 2, "video_layer_count": 4,
            "video_output_count": 2, "dmx_output_count": 1,
            "enabled_dmx_output_count": 1, "dmx_preview_universe_count": 1,
            "clock": {
                "bpm": 128.5, "beat_phase": 0.25, "beat_counter": 99,
                "tap_count": 3, "source": "DjLink", "external_sync_age_ms": null,
                "external_sync_locked": true, "PRIVATE_CLOCK_KEY": 123
            },
            "primary_output": {
                "enabled": true, "protocol": "ArtNet", "universe": 1,
                "serial_baud_rate": 57600, "target_ip": "192.0.2.37",
                "port": 6454, "serial_port": "PRIVATE_SERIAL_PORT"
            },
            "dmx_outputs": [{
                "enabled": true, "protocol": "ArtNet", "universe": 1,
                "serial_baud_rate": 57600, "target_ip": "192.0.2.37",
                "serial_port": "PRIVATE_SERIAL_PORT", "PRIVATE_ROUTE_KEY": false
            }],
            "budget": {
                "overall": "Warn", "target_dmx_frame_rate_hz": 44,
                "target_tick_interval_us": 22727, "tick_jitter_p99_target_us": 500,
                "command_queue_p99_target_us": 1000, "command_to_dmx_p99_target_us": 2000,
                "dmx_send_interval_tolerance_us": 500,
                "checks": [{
                    "name": "dmx_send_success", "status": "Pass", "measured_us": null,
                    "target_us": null, "samples": 123, "detail": "PRIVATE_BUDGET_DETAIL"
                }]
            },
            "telemetry": {
                "frame_counter": 123, "queue_depth": 2,
                "enabled_effect_count": 3, "supported_effect_count": 512,
                "effects_over_supported_envelope": false,
                "queue_push_failure_count": 4, "tick_jitter_last_us": -12,
                "tick_jitter_stddev_us": 2.5, "total_dmx_send_failure_count": 9,
                "last_error": "PRIVATE_FREEFORM_ERROR /home/private/show.sdc",
                "PRIVATE_TELEMETRY_KEY": "PRIVATE_NESTED_VALUE",
                "last_dmx_route_results": [{
                    "index": 0, "universe": 1, "attempted": true, "success": false,
                    "bytes": 512, "consecutive_failures": 3, "reconnect_attempts": 2,
                    "reconnecting": true, "retry_in_ms": 100,
                    "last_success_unix_ms": null,
                    "error": "PRIVATE_ROUTE_ERROR", "PRIVATE_NESTED_KEY": 777
                }]
            },
            "crash_reports": [{"PRIVATE_CRASH_FILENAME": "PRIVATE_RAW_CRASH_LOG"}]
        }),
        json!({
            "backends": [
                {"id": "ffmpeg", "state": "Missing", "label": "PRIVATE_BACKEND_LABEL",
                 "detail": "C:\\PRIVATE_BINARY_PATH\\ffmpeg.exe failed: PRIVATE_BACKEND_ERROR",
                 "PRIVATE_BACKEND_KEY": "PRIVATE_BACKEND_VALUE"},
                {"id": "spout", "state": "Available", "detail": "PRIVATE_DETAIL"}
            ],
            "environment": {"PATH": "PRIVATE_ENV_PATH"}
        }),
    ]
    .into_iter()
    .enumerate()
    .map(|(index, value)| (PAYLOAD_NAMES[index], serde_json::to_vec(&value).unwrap()))
    .collect()
}

fn change_json(entries: &mut [(&str, Vec<u8>)], index: usize, change: impl FnOnce(&mut Value)) {
    let mut value: Value = serde_json::from_slice(&entries[index].1).unwrap();
    change(&mut value);
    entries[index].1 = serde_json::to_vec(&value).unwrap();
}

// The ordinary ZIP reader is used only for independently inspecting known test
// artifacts. Production validation never trusts its duplicate-name handling.
fn unpack(bytes: &[u8]) -> Vec<(String, Vec<u8>)> {
    let mut archive = zip::ZipArchive::new(Cursor::new(bytes)).unwrap();
    (0..archive.len())
        .map(|index| {
            let mut file = archive.by_index(index).unwrap();
            let name = file.name().to_owned();
            let mut contents = Vec::new();
            file.read_to_end(&mut contents).unwrap();
            (name, contents)
        })
        .collect()
}

fn repack(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut archive = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Stored)
        .last_modified_time(zip::DateTime::default())
        .unix_permissions(0o600);
    for (name, contents) in entries {
        archive.start_file(name.as_str(), options).unwrap();
        archive.write_all(contents).unwrap();
    }
    archive.finish().unwrap().into_inner()
}

fn json_entry(entries: &[(String, Vec<u8>)], name: &str) -> Value {
    let contents = &entries.iter().find(|(entry, _)| entry == name).unwrap().1;
    serde_json::from_slice(contents).unwrap()
}

fn next_hostile_byte(state: &mut u64) -> u8 {
    *state ^= *state << 7;
    *state ^= *state >> 9;
    *state ^= *state << 8;
    (*state & 0xff) as u8
}

#[test]
fn diagnostic_package_preserves_known_counters_and_validates_integrity() {
    let bytes = build_diagnostic_package(&source_entries()).unwrap();
    validate_diagnostic_package(&bytes).unwrap();
    let files = unpack(&bytes);
    assert_eq!(
        files
            .iter()
            .map(|entry| entry.0.as_str())
            .collect::<Vec<_>>(),
        ZIP_NAMES
    );
    assert!(bytes.len() <= MAX_ARCHIVE_BYTES);
    let project = json_entry(&files, "project-summary.json");
    assert_eq!(project["fixtures"], 42);
    assert_eq!(project["timeline_video_automations"], 12);
    assert_eq!(project.as_object().unwrap().len(), 10);
    let engine = json_entry(&files, "engine-telemetry.json");
    assert_eq!(engine["clock"]["bpm"], 128.5);
    assert_eq!(engine["clock"]["external_sync_locked"], true);
    assert_eq!(engine["telemetry"]["tick_jitter_last_us"], -12);
    assert_eq!(
        engine["telemetry"]["effects_over_supported_envelope"],
        false
    );
    assert_eq!(
        engine["telemetry"]["last_dmx_route_results"][0]["reconnect_attempts"],
        2
    );
    assert_eq!(engine["budget"]["checks"][0]["status"], "Pass");
    assert_eq!(engine["primary_output"]["protocol"], "ArtNet");
    let video = json_entry(&files, "video-runtime.json");
    assert_eq!(
        video,
        json!({"backends": [
            {"id":"ffmpeg", "state":"Missing"}, {"id":"spout", "state":"Available"}
        ]})
    );
    let integrity = json_entry(&files, INTEGRITY_NAME);
    assert_eq!(integrity["schema_version"], 1);
    assert_eq!(integrity["redaction_schema_version"], 1);
    assert_eq!(integrity["format"], "syndocal-diagnostic-package");
    assert_eq!(integrity["entries"].as_array().unwrap().len(), 4);
    for (index, name) in PAYLOAD_NAMES.iter().enumerate() {
        let contents = &files.iter().find(|entry| entry.0 == *name).unwrap().1;
        assert_eq!(integrity["entries"][index]["name"], *name);
        assert_eq!(
            integrity["entries"][index]["size_bytes"],
            contents.len() as u64
        );
        assert_eq!(
            integrity["entries"][index]["sha256"],
            format!("{:x}", Sha256::digest(contents))
        );
    }
}

#[test]
fn diagnostic_package_discards_secrets_in_keys_values_errors_paths_and_numeric_unknowns() {
    let bytes = build_diagnostic_package(&source_entries()).unwrap();
    // STORED records let this inspect payloads and every ZIP metadata byte too.
    let text = String::from_utf8_lossy(&bytes);
    for forbidden in [
        "PRIVATE_",
        "current_project_path",
        "application_update",
        "192.0.2.37",
        "target_ip",
        "serial_port",
        "pairing_pin",
        "last_error",
        "crash_reports",
        "crash-reports",
        "environment",
        "README.txt",
        "example.invalid",
    ] {
        assert!(
            !text.contains(forbidden),
            "retained forbidden field or marker: {forbidden}"
        );
    }
    let files = unpack(&bytes);
    let engine = json_entry(&files, "engine-telemetry.json");
    assert!(engine["primary_output"].get("port").is_none());
    assert!(engine["budget"]["checks"][0].get("detail").is_none());
    assert!(engine["telemetry"]["last_dmx_route_results"][0]
        .get("error")
        .is_none());
}

#[test]
fn diagnostic_package_is_deterministic_and_hashes_only_sanitized_data() {
    let mut entries = source_entries();
    let first = build_diagnostic_package(&entries).unwrap();
    assert_eq!(first, build_diagnostic_package(&entries).unwrap());
    change_json(&mut entries, 0, |value| {
        value["current_project_path"] = json!("/a/completely/different/private/path.sdc");
        value["application_update"] = json!({"api_key": "different secret"});
    });
    change_json(&mut entries, 1, |value| {
        value.as_object_mut().unwrap().remove("PRIVATE_KEY_SECRET");
        value["A_DIFFERENT_PRIVATE_KEY"] = json!(["another secret", 9, true]);
    });
    change_json(&mut entries, 2, |value| {
        value["telemetry"]["last_error"] = json!("a different private error");
    });
    entries.reverse();
    assert_eq!(first, build_diagnostic_package(&entries).unwrap());
}

#[test]
fn diagnostic_package_rejects_wrong_entry_sets_and_unsafe_names_without_echoing_them() {
    let entries = source_entries();
    assert_eq!(
        build_diagnostic_package(&[]).unwrap_err(),
        DiagnosticPackageError::EntryCount
    );
    assert_eq!(
        build_diagnostic_package(&entries[..3]).unwrap_err(),
        DiagnosticPackageError::EntryCount
    );
    let mut extra = entries.clone();
    extra.push(("raw-crash.log", b"secret".to_vec()));
    assert_eq!(
        build_diagnostic_package(&extra).unwrap_err(),
        DiagnosticPackageError::EntryCount
    );
    for name in [
        "../PRIVATE_SECRET",
        "/manifest.json",
        "C:\\PRIVATE_SECRET",
        "manifest.json/..",
        "manifest.json\0PRIVATE_SECRET",
        "MANIFEST.JSON",
        "README.txt",
        INTEGRITY_NAME,
    ] {
        let mut changed = entries.clone();
        changed[0].0 = name;
        let error = build_diagnostic_package(&changed).unwrap_err();
        assert_eq!(error, DiagnosticPackageError::EntryName);
        assert!(!format!("{error:?}: {error}").contains("PRIVATE_SECRET"));
    }
    let mut duplicate = entries;
    duplicate[3].0 = "manifest.json";
    assert_eq!(
        build_diagnostic_package(&duplicate).unwrap_err(),
        DiagnosticPackageError::DuplicateEntry
    );
}

#[test]
fn diagnostic_package_rejects_malformed_json_and_duplicate_decoded_keys() {
    for bad in [
        b"".as_slice(),
        b"{",
        b"{\"fixtures\":NaN}",
        b"{\"fixtures\":1e9999}",
        b"{\"fixtures\":1} trailing",
        b"{\"PRIVATE_SECRET\":1,\"PRIVATE_SECRET\":2}",
        br#"{"fixtures":1,"\u0066ixtures":2}"#,
        b"{\"invalid\":\"\xff\"}",
    ] {
        let mut entries = source_entries();
        entries[1].1 = bad.to_vec();
        let error = build_diagnostic_package(&entries).unwrap_err();
        assert_eq!(error, DiagnosticPackageError::InvalidJson);
        assert!(!format!("{error:?}: {error}").contains("PRIVATE_SECRET"));
    }
    for non_object in [b"null".as_slice(), b"[]", b"42", b"true"] {
        let mut entries = source_entries();
        entries[1].1 = non_object.to_vec();
        assert_eq!(
            build_diagnostic_package(&entries).unwrap_err(),
            DiagnosticPackageError::InvalidField
        );
    }
}

#[test]
fn diagnostic_package_hostile_archive_corpus_is_bounded_and_panic_free() {
    const CASES: usize = 512;
    let valid = build_diagnostic_package(&source_entries()).unwrap();
    validate_diagnostic_package(&valid).unwrap();

    let mut corpus: Vec<(Vec<u8>, bool)> = Vec::with_capacity(CASES);
    corpus.push((valid.clone(), true));
    corpus.push((vec![0; MAX_ARCHIVE_BYTES + 1], false));

    let mut state = 0x5344_435f_2026_0914_u64;
    for _ in 0..128 {
        let length = 1 + (next_hostile_byte(&mut state) as usize % MAX_ARCHIVE_BYTES);
        let mut bytes = Vec::with_capacity(length);
        for _ in 0..length {
            bytes.push(next_hostile_byte(&mut state));
        }
        // Keep random cases guaranteed outside the ZIP local-header signature.
        bytes[0] = 0xa5;
        corpus.push((bytes, false));
    }

    for index in 0..128 {
        let length = index * (valid.len() - 1) / 127;
        corpus.push((valid[..length].to_vec(), false));
    }

    for index in 0..(CASES - corpus.len()) {
        let mut mutated = valid.clone();
        let offset = (index * 37) % mutated.len();
        mutated[offset] ^= (index as u8).wrapping_mul(17) | 1;
        corpus.push((mutated, false));
    }
    assert_eq!(corpus.len(), CASES);

    let mut accepted = 0;
    let mut rejected = 0;
    let mut panics = 0;
    for (index, (bytes, expected_valid)) in corpus.into_iter().enumerate() {
        assert!(bytes.len() <= MAX_ARCHIVE_BYTES + 1);
        let result = catch_unwind(AssertUnwindSafe(|| validate_diagnostic_package(&bytes)));
        if result.is_err() {
            panics += 1;
            continue;
        }
        let validation = result.unwrap();
        if validation.is_ok() {
            accepted += 1;
        } else {
            rejected += 1;
        }
        assert_eq!(
            validation.is_ok(),
            expected_valid,
            "unexpected validation result for corpus case {index}"
        );
    }
    assert_eq!(accepted, 1);
    assert_eq!(rejected, CASES - 1);
    assert_eq!(panics, 0);
    println!(
        "diagnostic hostile archive corpus: {CASES} cases, {rejected} rejected, {panics} panics, max_bytes={}",
        MAX_ARCHIVE_BYTES + 1
    );
}

#[test]
fn diagnostic_package_rejects_invalid_known_values_and_fixed_identities() {
    for (index, pointer, bad) in [
        (0, "/version", json!(2)),
        (0, "/app", json!("PRIVATE_SECRET")),
        (0, "/app_version", json!("1.2.0-PRIVATE_SECRET")),
        (0, "/os", json!("C:\\PRIVATE_SECRET")),
        (0, "/arch", json!("PRIVATE_SECRET")),
        (1, "/fixtures", json!("PRIVATE_SECRET")),
        (1, "/fixtures", json!(-1)),
        (1, "/fixtures", json!(2.5)),
        (2, "/clock/source", json!("PRIVATE_SECRET")),
        (2, "/clock/external_sync_locked", json!("true")),
        (2, "/telemetry/tick_jitter_stddev_us", json!(-0.5)),
        (2, "/budget/checks/0/name", json!("PRIVATE_SECRET")),
        (2, "/primary_output/protocol", json!("PRIVATE_SECRET")),
        (3, "/backends/0/id", json!("PRIVATE_SECRET")),
        (3, "/backends/0/state", json!("PRIVATE_SECRET")),
    ] {
        let mut entries = source_entries();
        change_json(&mut entries, index, |value| {
            *value.pointer_mut(pointer).unwrap() = bad
        });
        let error = build_diagnostic_package(&entries).unwrap_err();
        assert_eq!(
            error,
            DiagnosticPackageError::InvalidField,
            "case {pointer}"
        );
        assert!(!format!("{error:?}: {error}").contains("PRIVATE_SECRET"));
    }
}

#[test]
fn diagnostic_package_requires_identity_fields_but_does_not_invent_missing_counters() {
    let mut entries = source_entries();
    entries[1].1 = b"{}".to_vec();
    entries[2].1 = br#"{"version":1}"#.to_vec();
    entries[3].1 = br#"{"backends":[]}"#.to_vec();
    let files = unpack(&build_diagnostic_package(&entries).unwrap());
    assert_eq!(json_entry(&files, "project-summary.json"), json!({}));
    assert_eq!(
        json_entry(&files, "engine-telemetry.json"),
        json!({"version": 1})
    );
    for key in ["version", "app", "app_version", "os", "arch"] {
        let mut missing = entries.clone();
        change_json(&mut missing, 0, |value| {
            value.as_object_mut().unwrap().remove(key);
        });
        assert_eq!(
            build_diagnostic_package(&missing).unwrap_err(),
            DiagnosticPackageError::InvalidField
        );
    }
    entries[3].1 = br#"{"backends":[{"state":"Available"}]}"#.to_vec();
    assert_eq!(
        build_diagnostic_package(&entries).unwrap_err(),
        DiagnosticPackageError::InvalidField
    );
}

#[test]
fn diagnostic_package_checks_raw_byte_limits_before_parsing() {
    let entries = source_entries();
    let original = build_diagnostic_package(&entries).unwrap();
    let mut boundary = entries.clone();
    boundary[1].1.resize(MAX_INPUT_ENTRY_BYTES, b' ');
    assert_eq!(build_diagnostic_package(&boundary).unwrap(), original);
    boundary[1].1.push(b' ');
    assert_eq!(
        build_diagnostic_package(&boundary).unwrap_err(),
        DiagnosticPackageError::InputLimit
    );
    let mut total_boundary = entries;
    let original_total: usize = total_boundary.iter().map(|entry| entry.1.len()).sum();
    let mut padding = MAX_TOTAL_INPUT_BYTES - original_total;
    for (_, contents) in &mut total_boundary {
        let extra = padding.min(MAX_INPUT_ENTRY_BYTES - contents.len());
        contents.resize(contents.len() + extra, b' ');
        padding -= extra;
    }
    assert_eq!(padding, 0);
    assert_eq!(build_diagnostic_package(&total_boundary).unwrap(), original);
    total_boundary
        .iter_mut()
        .find(|entry| entry.1.len() < MAX_INPUT_ENTRY_BYTES)
        .unwrap()
        .1
        .push(b' ');
    assert_eq!(
        build_diagnostic_package(&total_boundary).unwrap_err(),
        DiagnosticPackageError::InputLimit
    );
}

#[test]
fn diagnostic_package_bounds_depth_even_in_discarded_fields() {
    for (arrays, accepted) in [(MAX_JSON_DEPTH - 2, true), (MAX_JSON_DEPTH - 1, false)] {
        let mut entries = source_entries();
        let nested = format!("{}0{}", "[".repeat(arrays), "]".repeat(arrays));
        entries[1].1 = format!("{{\"ignored\":{nested}}}").into_bytes();
        assert_eq!(build_diagnostic_package(&entries).is_ok(), accepted);
    }
}

#[test]
fn diagnostic_package_bounds_strings_keys_containers_and_total_nodes() {
    let mut entries = source_entries();
    change_json(&mut entries, 1, |value| {
        value["ignored"] = json!("x".repeat(MAX_JSON_STRING_BYTES))
    });
    build_diagnostic_package(&entries).unwrap();
    change_json(&mut entries, 1, |value| {
        value["ignored"] = json!("x".repeat(MAX_JSON_STRING_BYTES + 1))
    });
    assert_eq!(
        build_diagnostic_package(&entries).unwrap_err(),
        DiagnosticPackageError::InvalidJson
    );
    for value in [
        json!({"ignored": vec![0; MAX_JSON_CONTAINER_ITEMS + 1]}),
        json!({"ignored": vec![vec![0; 512]; 32]}),
        Value::Object(
            [(
                String::from("x").repeat(MAX_JSON_STRING_BYTES + 1),
                json!(0),
            )]
            .into_iter()
            .collect(),
        ),
        Value::Object(
            (0..=MAX_JSON_CONTAINER_ITEMS)
                .map(|index| (format!("unknown{index}"), json!(0)))
                .collect(),
        ),
    ] {
        entries[1].1 = serde_json::to_vec(&value).unwrap();
        assert!(entries[1].1.len() < MAX_INPUT_ENTRY_BYTES);
        assert_eq!(
            build_diagnostic_package(&entries).unwrap_err(),
            DiagnosticPackageError::InvalidJson
        );
    }
    entries[1].1 =
        serde_json::to_vec(&json!({"ignored": vec![0; MAX_JSON_CONTAINER_ITEMS]})).unwrap();
    build_diagnostic_package(&entries).unwrap();
}

#[test]
fn diagnostic_package_bounds_schema_arrays_and_rejects_duplicate_fixed_ids() {
    for (index, pointer) in [(2, "/budget/checks"), (3, "/backends")] {
        let mut entries = source_entries();
        change_json(&mut entries, index, |value| {
            let items = value.pointer_mut(pointer).unwrap().as_array_mut().unwrap();
            items.push(items[0].clone());
        });
        assert_eq!(
            build_diagnostic_package(&entries).unwrap_err(),
            DiagnosticPackageError::InvalidField
        );
    }
    let mut entries = source_entries();
    change_json(&mut entries, 2, |value| {
        value["dmx_outputs"] = json!(vec![json!({"enabled": true}); 129])
    });
    assert_eq!(
        build_diagnostic_package(&entries).unwrap_err(),
        DiagnosticPackageError::InvalidField
    );
}

#[test]
fn diagnostic_package_detects_payload_tampering_with_valid_zip_crcs() {
    let bytes = build_diagnostic_package(&source_entries()).unwrap();
    let mut files = unpack(&bytes);
    let project = files
        .iter_mut()
        .find(|entry| entry.0 == "project-summary.json")
        .unwrap();
    let mut value: Value = serde_json::from_slice(&project.1).unwrap();
    value["fixtures"] = json!(43);
    project.1 = serde_json::to_vec(&value).unwrap();
    let altered = repack(&files);
    // Independent ZIP decode succeeds: this specifically reaches SHA256 checks.
    assert_eq!(
        json_entry(&unpack(&altered), "project-summary.json")["fixtures"],
        43
    );
    assert_eq!(
        validate_diagnostic_package(&altered).unwrap_err(),
        DiagnosticPackageError::IntegrityMismatch
    );
}

#[test]
fn diagnostic_package_rejects_private_fields_even_when_hashes_are_recomputed() {
    let files = unpack(&build_diagnostic_package(&source_entries()).unwrap());
    for value in [
        json!({"fixtures": 42, "PRIVATE_KEY_SECRET": "PRIVATE_VALUE_SECRET"}),
        json!({"fixtures": 42, "unknown_counter": 123456}),
        json!({"fixtures": "PRIVATE_VALUE_SECRET"}),
    ] {
        let mut payloads: [Vec<u8>; 4] = std::array::from_fn(|index| files[index].1.clone());
        payloads[1] = serde_json::to_vec(&value).unwrap();
        let integrity = integrity_bytes(&payloads).unwrap();
        // Bypass only the builder's sanitizer, keeping valid hashes and headers.
        let malicious = encode_archive(&payloads, &integrity).unwrap();
        assert_eq!(
            validate_diagnostic_package(&malicious).unwrap_err(),
            DiagnosticPackageError::InvalidField
        );
    }
}

#[test]
fn diagnostic_package_rejects_missing_extra_and_unsafe_zip_entries() {
    let files = unpack(&build_diagnostic_package(&source_entries()).unwrap());
    for removed in 0..ZIP_NAMES.len() {
        let mut missing = files.clone();
        missing.remove(removed);
        assert!(validate_diagnostic_package(&repack(&missing)).is_err());
    }
    let mut extra = files.clone();
    extra.push((
        "PRIVATE_CRASH.log".to_string(),
        b"PRIVATE_RAW_CRASH".to_vec(),
    ));
    assert_eq!(
        validate_diagnostic_package(&repack(&extra)).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
    for name in [
        "../PRIVATE_SECRET",
        "/manifest.json",
        "C:\\PRIVATE_SECRET",
        "README.txt",
    ] {
        let mut traversal = files.clone();
        traversal[0].0 = name.to_string();
        assert_eq!(
            validate_diagnostic_package(&repack(&traversal)).unwrap_err(),
            DiagnosticPackageError::EntryName
        );
    }
}

#[test]
fn diagnostic_package_rejects_duplicate_local_zip_names() {
    let mut files = unpack(&build_diagnostic_package(&source_entries()).unwrap());
    // Some ZipWriter versions refuse duplicate names; write an equal-length
    // alias, then rename both its local and central names without touching data.
    files[3] = ("manifesX.json".to_string(), files[0].1.clone());
    let mut duplicate = repack(&files);
    let from = b"manifesX.json";
    let to = b"manifest.json";
    assert_eq!(from.len(), to.len());
    let matches: Vec<_> = duplicate
        .windows(from.len())
        .enumerate()
        .filter_map(|(offset, window)| (window == from).then_some(offset))
        .collect();
    assert_eq!(matches.len(), 2);
    for offset in matches {
        duplicate[offset..offset + to.len()].copy_from_slice(to);
    }
    assert_eq!(
        validate_diagnostic_package(&duplicate).unwrap_err(),
        DiagnosticPackageError::DuplicateEntry
    );
}

#[test]
fn diagnostic_package_rejects_duplicate_central_records_hidden_by_generic_readers() {
    let mut bytes = build_diagnostic_package(&source_entries()).unwrap();
    let central = bytes
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    let record_len = 46 + PAYLOAD_NAMES[0].len();
    let extra = bytes[central..central + record_len].to_vec();
    let old_end = bytes.len() - 22;
    let old_size = u32::from_le_bytes(bytes[old_end + 12..old_end + 16].try_into().unwrap());
    bytes.splice(old_end..old_end, extra);
    let end = bytes.len() - 22;
    bytes[end + 8..end + 10].copy_from_slice(&6_u16.to_le_bytes());
    bytes[end + 10..end + 12].copy_from_slice(&6_u16.to_le_bytes());
    bytes[end + 12..end + 16].copy_from_slice(&(old_size + record_len as u32).to_le_bytes());
    assert_eq!(
        validate_diagnostic_package(&bytes).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
}

#[test]
fn diagnostic_package_rejects_wrong_manifest_versions_hashes_sizes_and_entry_sets() {
    let files = unpack(&build_diagnostic_package(&source_entries()).unwrap());
    for change in 0..8 {
        let mut changed = files.clone();
        let integrity = changed
            .iter_mut()
            .find(|entry| entry.0 == INTEGRITY_NAME)
            .unwrap();
        // Use the explicit field order so each change isolates the damaged
        // manifest, instead of merely failing because a Value map reordered it.
        let mut manifest = IntegrityManifest {
            format: "syndocal-diagnostic-package",
            schema_version: 1,
            redaction_schema_version: 1,
            entries: PAYLOAD_NAMES
                .iter()
                .enumerate()
                .map(|(index, &name)| IntegrityEntry {
                    name,
                    size_bytes: files[index].1.len() as u64,
                    sha256: format!("{:x}", Sha256::digest(&files[index].1)),
                })
                .collect(),
        };
        assert_eq!(serialize_bounded(&manifest).unwrap(), integrity.1);
        match change {
            0 => manifest.schema_version = 2,
            1 => manifest.redaction_schema_version = 2,
            2 => manifest.entries[0].sha256 = "0".repeat(64),
            3 => manifest.entries[0].size_bytes = 0,
            4 => {
                manifest.entries.pop();
            }
            5 => {
                manifest.entries.push(IntegrityEntry {
                    name: manifest.entries[0].name,
                    size_bytes: manifest.entries[0].size_bytes,
                    sha256: manifest.entries[0].sha256.clone(),
                });
            }
            6 => manifest.entries[0].name = "../PRIVATE_SECRET",
            _ => {}
        }
        integrity.1 = serialize_bounded(&manifest).unwrap();
        if change == 7 {
            assert_eq!(integrity.1.pop(), Some(b'}'));
            integrity
                .1
                .extend_from_slice(br#", "PRIVATE_KEY_SECRET":"PRIVATE_VALUE_SECRET"}"#);
        }
        assert_eq!(
            validate_diagnostic_package(&repack(&changed)).unwrap_err(),
            DiagnosticPackageError::IntegrityMismatch
        );
    }
}

#[test]
fn diagnostic_package_rejects_zip_metadata_trailing_data_crc_and_local_central_mismatch() {
    let original = build_diagnostic_package(&source_entries()).unwrap();
    let mut trailing = original.clone();
    trailing.extend_from_slice(b"PRIVATE_TRAILING_SECRET");
    assert_eq!(
        validate_diagnostic_package(&trailing).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
    let mut bad_crc = original.clone();
    bad_crc[14] ^= 1;
    assert_eq!(
        validate_diagnostic_package(&bad_crc).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
    let mut bad_central = original.clone();
    let central = bad_central
        .windows(4)
        .position(|window| window == b"PK\x01\x02")
        .unwrap();
    let replacement = b"../leaks.json";
    assert_eq!(replacement.len(), PAYLOAD_NAMES[0].len());
    bad_central[central + 46..central + 46 + replacement.len()].copy_from_slice(replacement);
    assert_eq!(
        validate_diagnostic_package(&bad_central).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
    let mut comment = original.clone();
    let end = comment.len() - 22;
    let secret = b"PRIVATE_COMMENT_SECRET";
    comment[end + 20..end + 22].copy_from_slice(&(secret.len() as u16).to_le_bytes());
    comment.extend_from_slice(secret);
    assert_eq!(
        validate_diagnostic_package(&comment).unwrap_err(),
        DiagnosticPackageError::InvalidArchive
    );
    for end in [0, 1, 29, original.len() / 2, original.len() - 1] {
        assert!(validate_diagnostic_package(&original[..end]).is_err());
    }
}

#[test]
fn diagnostic_package_rejects_compression_encryption_descriptors_and_oversize_headers() {
    let original = build_diagnostic_package(&source_entries()).unwrap();
    for (offset, value) in [(6, 1_u8), (6, 8), (8, 8), (28, 1)] {
        let mut changed = original.clone();
        changed[offset] = value;
        assert_eq!(
            validate_diagnostic_package(&changed).unwrap_err(),
            DiagnosticPackageError::InvalidArchive
        );
    }
    let mut oversized = original;
    oversized[18..22].copy_from_slice(&u32::MAX.to_le_bytes());
    oversized[22..26].copy_from_slice(&u32::MAX.to_le_bytes());
    assert_eq!(
        validate_diagnostic_package(&oversized).unwrap_err(),
        DiagnosticPackageError::OutputLimit
    );
    assert_eq!(
        validate_diagnostic_package(&vec![0; MAX_ARCHIVE_BYTES + 1]).unwrap_err(),
        DiagnosticPackageError::OutputLimit
    );
}

#[test]
fn diagnostic_package_output_writer_limits_apply_to_json_zip_and_seeks() {
    let mut writer = LimitedCursor::new(8);
    writer.write_all(b"12345678").unwrap();
    assert!(writer.write_all(b"9").is_err());
    assert!(writer.seek(SeekFrom::Start(9)).is_err());
    assert!(writer.seek(SeekFrom::Current(i64::MAX)).is_err());
    assert!(writer.seek(SeekFrom::End(-9)).is_err());
    assert_eq!(writer.seek(SeekFrom::End(-2)).unwrap(), 6);
    writer.write_all(b"ab").unwrap();
    assert_eq!(writer.into_inner(), b"123456ab");
    assert_eq!(
        serialize_bounded(&"x".repeat(MAX_SANITIZED_ENTRY_BYTES)).unwrap_err(),
        DiagnosticPackageError::OutputLimit
    );
    let payloads: [Vec<u8>; 4] = std::array::from_fn(|_| vec![0; MAX_SANITIZED_ENTRY_BYTES]);
    assert_eq!(
        encode_archive(&payloads, b"{}").unwrap_err(),
        DiagnosticPackageError::OutputLimit
    );
}

#[test]
fn diagnostic_package_preview_validates_first_and_lists_only_fixed_names_and_sizes() {
    let bytes = build_diagnostic_package(&source_entries()).unwrap();
    let preview = diagnostic_package_preview(&bytes).unwrap();
    assert!(preview.contains(&format!("5 entries, {} bytes", bytes.len())));
    for (name, contents) in unpack(&bytes) {
        assert!(preview.contains(&format!("{name}: {} bytes", contents.len())));
    }
    assert!(!preview.contains("PRIVATE_"));
    assert!(preview.contains("not authenticity"));
    assert!(diagnostic_package_preview(b"PRIVATE_INVALID_ARCHIVE").is_err());
}
