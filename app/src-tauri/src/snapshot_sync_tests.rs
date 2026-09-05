use super::*;

#[test]
fn engine_snapshot_delta_contains_only_changed_top_level_sections() {
    let before = EngineSnapshot::default();
    let mut after = before.clone();
    after.blackout = true;
    after.clock.bpm = 127.0;
    after.stage_map.max_x = 24.0;

    let delta = engine_snapshot_delta(&before, &after);
    assert_eq!(delta.blackout, Some(true));
    assert_eq!(delta.clock.as_ref().map(|clock| clock.bpm), Some(127.0));
    assert_eq!(delta.stage_map.as_ref().map(|map| map.max_x), Some(24.0));
    assert!(delta.fixtures.is_none());
    assert!(delta.cues.is_none());
    assert!(delta.video.is_none());
    assert!(delta.dmx_previews.is_none());
    assert!(delta.telemetry.is_none());

    let json = serde_json::to_value(delta).unwrap();
    let object = json.as_object().unwrap();
    assert_eq!(object.len(), 3);
    assert!(object.contains_key("blackout"));
    assert!(object.contains_key("clock"));
    assert!(object.contains_key("stage_map"));
}

#[test]
fn alternating_clients_use_their_own_exact_snapshot_base() {
    let mut sync = SnapshotSyncState::default();
    let initial = EngineSnapshot::default();
    let first = sync.publish(None, initial.clone()).unwrap();
    let mut middle = initial.clone();
    middle.blackout = true;
    let second = sync.publish(None, middle.clone()).unwrap();
    let from_first = sync.publish(Some(first.revision), middle.clone()).unwrap();
    assert_eq!(from_first.delta.unwrap().blackout, Some(true));
    let from_second = sync.publish(Some(second.revision), middle).unwrap();
    assert!(from_second.delta.unwrap().blackout.is_none());
}

#[test]
fn eviction_unknown_and_future_revisions_resynchronize_with_full_snapshot() {
    let mut sync = SnapshotSyncState::default();
    for _ in 0..SNAPSHOT_HISTORY_CAPACITY {
        sync.publish(None, EngineSnapshot::default()).unwrap();
    }
    assert!(sync
        .publish(Some(1), EngineSnapshot::default())
        .unwrap()
        .delta
        .is_some());
    assert_eq!(sync.history.len(), SNAPSHOT_HISTORY_CAPACITY);
    for missing in [Some(1), Some(0), Some(u64::MAX), None] {
        let response = sync.publish(missing, EngineSnapshot::default()).unwrap();
        assert!(response.full.is_some());
        assert!(response.delta.is_none());
        assert_eq!(sync.history.len(), SNAPSHOT_HISTORY_CAPACITY);
    }
}

#[test]
fn nullable_clearing_is_explicit_null_and_unchanged_fields_are_absent() {
    let before = EngineSnapshot {
        active_cue_id: Some(42),
        ..EngineSnapshot::default()
    };
    let mut sync = SnapshotSyncState::default();
    let first = sync.publish(None, before).unwrap();
    let response = sync
        .publish(Some(first.revision), EngineSnapshot::default())
        .unwrap();
    let json = serde_json::to_value(response).unwrap();
    assert_eq!(json["delta"]["active_cue_id"], serde_json::Value::Null);
    assert!(json["delta"]
        .as_object()
        .unwrap()
        .contains_key("active_cue_id"));
    assert!(!json["delta"]
        .as_object()
        .unwrap()
        .contains_key("active_fade"));
    assert!(!json.as_object().unwrap().contains_key("full"));
}

#[test]
fn revision_exhaustion_fails_without_mutating_or_wrapping_history() {
    let mut sync = SnapshotSyncState::default();
    sync.publish(None, EngineSnapshot::default()).unwrap();
    sync.revision = u64::MAX;
    let error = sync
        .publish(Some(1), EngineSnapshot::default())
        .unwrap_err();
    assert!(error.contains("revision exhausted"));
    assert!(error.contains("restart Syndocal"));
    assert_eq!(sync.revision, u64::MAX);
    assert_eq!(sync.history.len(), 1);
    assert_eq!(sync.history[0].0, 1);
}

/// Deterministic payload workload, not a wall-clock/CPU benchmark. Both policies
/// receive identical snapshots and client scheduling; only cache retention differs.
#[test]
fn alternating_client_payload_workload_reports_full_delta_counts_and_bytes() {
    let mut snapshot = EngineSnapshot::default();
    snapshot.cues = (1..=256)
        .map(|id| protocol::CueSummary {
            id,
            label: format!("Authored show cue {id:03}: stage wash and scenic transition"),
            group_id: Some(format!("Stage group {}", id % 16)),
            ..protocol::CueSummary::default()
        })
        .collect();
    snapshot.group_colors = (0..16)
        .map(|id| (format!("Stage group {id}"), "#4488cc".into()))
        .collect();
    snapshot.dmx_preview = vec![0; 512];
    let mut sync = SnapshotSyncState::default();
    let mut clients = [None; 2];
    let mut old_clients = [None; 2];
    let mut client_snapshots = [serde_json::Value::Null, serde_json::Value::Null];
    let mut old_last: Option<EngineSnapshot> = None;
    let mut old_revision = 0;
    let (mut old_full, mut old_delta, mut old_bytes) = (0, 0, 0);
    let (mut new_full, mut new_delta, mut new_bytes) = (0, 0, 0);
    for step in 0..100 {
        snapshot.playback_master = step as f32 / 100.0;
        let client = step % 2;
        let old_payload = if old_clients[client] == Some(old_revision) && old_last.is_some() {
            SnapshotSyncPayload {
                revision: old_revision + 1,
                full: None,
                delta: Some(engine_snapshot_delta(old_last.as_ref().unwrap(), &snapshot)),
            }
        } else {
            SnapshotSyncPayload {
                revision: old_revision + 1,
                full: Some(snapshot.clone()),
                delta: None,
            }
        };
        old_full += usize::from(old_payload.full.is_some());
        old_delta += usize::from(old_payload.delta.is_some());
        old_bytes += serde_json::to_vec(&old_payload).unwrap().len();
        old_revision = old_payload.revision;
        old_clients[client] = Some(old_revision);
        old_last = Some(snapshot.clone());
        let payload = sync.publish(clients[client], snapshot.clone()).unwrap();
        new_full += usize::from(payload.full.is_some());
        new_delta += usize::from(payload.delta.is_some());
        new_bytes += serde_json::to_vec(&payload).unwrap().len();
        merge_wire_payload(&mut client_snapshots[client], &payload);
        assert_eq!(
            client_snapshots[client],
            serde_json::to_value(&snapshot).unwrap(),
            "client {client} failed to reconstruct request {step}"
        );
        clients[client] = Some(payload.revision);
    }
    println!("snapshot payload workload: old full={old_full} delta={old_delta} bytes={old_bytes}; bounded full={new_full} delta={new_delta} bytes={new_bytes}; retained={}", sync.history.len());
    assert_eq!((old_full, old_delta), (100, 0));
    assert_eq!((new_full, new_delta), (2, 98));
    assert!(
        new_bytes < old_bytes / 10,
        "authored payload traffic should fall by more than 90% for this workload"
    );
    assert_eq!(sync.history.len(), SNAPSHOT_HISTORY_CAPACITY);
}

// The frontend replaces changed top-level sections, including explicit null and
// empty collections. Exercise that wire contract independently of Rust delta fields.
fn merge_wire_payload(current: &mut serde_json::Value, payload: &SnapshotSyncPayload) {
    let wire = serde_json::to_value(payload).unwrap();
    if let Some(full) = wire.get("full") {
        assert!(wire.get("delta").is_none());
        *current = full.clone();
    } else {
        let delta = wire["delta"].as_object().unwrap();
        let object = current
            .as_object_mut()
            .expect("delta must follow a full snapshot");
        for (field, value) in delta {
            object.insert(field.clone(), value.clone());
        }
    }
}

#[test]
fn multi_field_round_trips_reconstruct_nullable_and_removed_collections() {
    let empty = EngineSnapshot::default();
    let mut authored = empty.clone();
    authored.blackout = true;
    authored.active_cue_id = Some(42);
    authored.playback_master = 0.25;
    authored.clock.bpm = 132.0;
    authored.stage_map.max_x = 24.0;
    authored.cues.push(protocol::CueSummary {
        id: 42,
        label: "Scenic cue".into(),
        ..protocol::CueSummary::default()
    });
    authored.active_group_cue_ids.insert("Scenic".into(), 42);
    authored
        .group_colors
        .insert("Scenic".into(), "#ff4488".into());
    authored.dmx_preview = vec![127; 512];
    let mut revised = authored.clone();
    revised.cues[0].label = "Revised scenic cue".into();
    revised.playback_master = 0.75;
    revised.blackout = false;
    let mut sync = SnapshotSyncState::default();
    let mut revisions = [None; 2];
    let mut reconstructed = [serde_json::Value::Null, serde_json::Value::Null];
    // Both clients observe population, replacement, removal, then repopulation.
    // Their bases are always different global revisions.
    for snapshot in [&empty, &authored, &revised, &empty, &authored, &empty] {
        for client in 0..2 {
            let payload = sync.publish(revisions[client], snapshot.clone()).unwrap();
            if reconstructed[client]["active_cue_id"] == serde_json::json!(42)
                && snapshot.active_cue_id.is_none()
            {
                let wire = serde_json::to_value(&payload).unwrap();
                let delta = wire["delta"].as_object().unwrap();
                assert_eq!(delta.get("active_cue_id"), Some(&serde_json::Value::Null));
                assert_eq!(delta.get("cues"), Some(&serde_json::json!([])));
                assert_eq!(
                    delta.get("active_group_cue_ids"),
                    Some(&serde_json::json!({}))
                );
                assert_eq!(delta.get("group_colors"), Some(&serde_json::json!({})));
            }
            merge_wire_payload(&mut reconstructed[client], &payload);
            // Full snapshots omit empty group maps; deltas must explicitly send
            // {} to remove prior values. Compare all decoded fields, preserving
            // the distinction between required clearing and wire omission above.
            let decoded: EngineSnapshot =
                serde_json::from_value(reconstructed[client].clone()).unwrap();
            assert_eq!(&decoded, snapshot);
            revisions[client] = Some(payload.revision);
        }
    }
}
