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

#[test]
fn full_response_shares_immutable_history_image_and_serializes_after_eviction() {
    #[derive(Serialize)]
    struct OwnedPayload {
        revision: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        full: Option<EngineSnapshot>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<EngineSnapshotDelta>,
    }

    let mut sync = SnapshotSyncState::default();
    let mut captured = EngineSnapshot::default();
    captured.cues.push(protocol::CueSummary {
        id: 42,
        label: "captured cue".into(),
        ..protocol::CueSummary::default()
    });
    let expected = serde_json::json!({ "revision": 1, "full": &captured });
    let expected_bytes = serde_json::to_vec(&OwnedPayload {
        revision: 1,
        full: Some(captured.clone()),
        delta: None,
    })
    .unwrap();
    let mut response = sync.publish(None, captured).unwrap();
    let full = response.full.as_mut().unwrap();
    assert!(Arc::ptr_eq(full, &sync.history[0].1));
    assert_eq!(Arc::strong_count(full), 2);
    assert!(Arc::get_mut(full).is_none());
    let retained = Arc::downgrade(full);

    for index in 0..SNAPSHOT_HISTORY_CAPACITY {
        let mut next = EngineSnapshot::default();
        next.clock.bpm = 130.0 + index as f32;
        sync.publish(None, next).unwrap();
    }
    assert_eq!(sync.history.len(), SNAPSHOT_HISTORY_CAPACITY);
    assert!(sync.history.iter().all(|(revision, _)| *revision != 1));
    assert_eq!(Arc::strong_count(response.full.as_ref().unwrap()), 1);
    drop(sync);
    // The original image remains intact even when IPC serialization is delayed
    // beyond its eviction from the four retained client bases.
    assert_eq!(serde_json::to_value(&response).unwrap(), expected);
    assert_eq!(serde_json::to_vec(&response).unwrap(), expected_bytes);
    drop(response);
    assert!(retained.upgrade().is_none());
}

#[test]
#[ignore = "fixed publication workloads; excludes serialization and real-show performance"]
fn benchmark_full_snapshot_sync_shared_capture() {
    use std::{
        hint::black_box,
        time::{Duration, Instant},
    };

    #[derive(Serialize)]
    struct OwnedPayload {
        revision: u64,
        #[serde(skip_serializing_if = "Option::is_none")]
        full: Option<EngineSnapshot>,
        #[serde(skip_serializing_if = "Option::is_none")]
        delta: Option<EngineSnapshotDelta>,
    }

    #[derive(Default)]
    struct PreviousSync {
        revision: u64,
        history: VecDeque<(u64, EngineSnapshot)>,
    }

    impl PreviousSync {
        // Exact pre-Arc publication algorithm, including four retained bases.
        fn publish(
            &mut self,
            client_revision: Option<u64>,
            current: EngineSnapshot,
        ) -> Result<OwnedPayload, String> {
            let revision = self.revision.checked_add(1).ok_or_else(||
                "Snapshot synchronization revision exhausted; restart Syndocal to establish a new session".to_string())?;
            let before = client_revision.and_then(|requested| {
                self.history
                    .iter()
                    .find(|(revision, _)| *revision == requested)
                    .map(|(_, snapshot)| snapshot)
            });
            let payload = match before {
                Some(before) => OwnedPayload {
                    revision,
                    full: None,
                    delta: Some(engine_snapshot_delta(before, &current)),
                },
                None => OwnedPayload {
                    revision,
                    full: Some(current.clone()),
                    delta: None,
                },
            };
            if self.history.len() == SNAPSHOT_HISTORY_CAPACITY {
                self.history.pop_front();
            }
            self.history.push_back((revision, current));
            self.revision = revision;
            Ok(payload)
        }
    }

    #[derive(Clone, Copy)]
    enum Schedule {
        Full,
        Mixed,
        Steady,
    }

    const ITERATIONS: usize = 1_000;
    fn measure<P: Serialize>(
        captured: &EngineSnapshot,
        schedule: Schedule,
        mut publish: impl FnMut(Option<u64>, EngineSnapshot) -> (u64, bool, P),
    ) -> (Duration, (usize, usize), Vec<u8>) {
        let mut clients = [None; 2];
        let mut full_count = 0;
        let mut last = None;
        let started = Instant::now();
        for step in 0..ITERATIONS {
            let client = if matches!(schedule, Schedule::Mixed) {
                step % 2
            } else {
                0
            };
            let requested = match schedule {
                Schedule::Full => None,
                Schedule::Mixed if step % 32 == 0 => None,
                _ => clients[client],
            };
            let mut current = black_box(captured).clone();
            // Clock changes are typical during playback; all authored collections
            // remain identical. Both implementations pay the same capture clone.
            current.clock.bpm = 120.0 + (step % 3) as f32;
            let (revision, full, payload) = publish(requested, current);
            clients[client] = Some(revision);
            full_count += usize::from(full);
            last = Some(black_box(payload));
        }
        let elapsed = started.elapsed();
        // Serialization and assertions are deliberately outside the timed loop.
        (
            elapsed,
            (full_count, ITERATIONS - full_count),
            serde_json::to_vec(&last.unwrap()).unwrap(),
        )
    }

    let mut captured = EngineSnapshot::default();
    captured.cues = (0..1_024)
        .map(|id| protocol::CueSummary {
            id,
            label: format!("cue {id}: {}", "authored label ".repeat(16)),
            ..protocol::CueSummary::default()
        })
        .collect();
    let light = EngineSnapshot::default();
    for (name, input, schedule, expected_full) in [
        ("full-heavy", &captured, Schedule::Full, 1_000),
        ("mixed-heavy", &captured, Schedule::Mixed, 33),
        ("steady-heavy", &captured, Schedule::Steady, 1),
        ("steady-light", &light, Schedule::Steady, 1),
    ] {
        let mut previous_times = Vec::new();
        let mut shared_times = Vec::new();
        for round in 0..3 {
            let mut previous = PreviousSync::default();
            let mut shared = SnapshotSyncState::default();
            let mut measure_previous = || {
                measure(input, schedule, |requested, current| {
                    let payload = previous.publish(requested, current).unwrap();
                    (payload.revision, payload.full.is_some(), payload)
                })
            };
            let mut measure_shared = || {
                measure(input, schedule, |requested, current| {
                    let payload = shared.publish(requested, current).unwrap();
                    (payload.revision, payload.full.is_some(), payload)
                })
            };
            let (old, new) = if round % 2 == 0 {
                (measure_previous(), measure_shared())
            } else {
                let new = measure_shared();
                (measure_previous(), new)
            };
            assert_eq!(old.1, (expected_full, ITERATIONS - expected_full));
            assert_eq!(old.1, new.1);
            assert_eq!(old.2, new.2, "final response wire differs for {name}");
            assert_eq!(previous.history.len(), SNAPSHOT_HISTORY_CAPACITY);
            assert_eq!(shared.history.len(), SNAPSHOT_HISTORY_CAPACITY);
            assert_eq!(
                serde_json::to_vec(&previous.history).unwrap(),
                serde_json::to_vec(&shared.history).unwrap(),
                "retained images differ for {name}"
            );
            println!(
                "{name} round {round}: previous={:?}, shared={:?}, full/delta={:?}",
                old.0, new.0, old.1
            );
            previous_times.push(old.0);
            shared_times.push(new.0);
        }
        previous_times.sort_unstable();
        shared_times.sort_unstable();
        println!("{name} median, {ITERATIONS} requests, history 4: previous={:?}, shared={:?}; serialization excluded",
            previous_times[1], shared_times[1]);
    }
}

#[test]
#[ignore = "fixed representative fixture; measurement only, not IPC or multi-window acceptance"]
fn benchmark_phase1_snapshot_sync_delta_and_serialization() {
    use std::{hint::black_box, time::Instant};

    let document: serde_json::Value =
        serde_json::from_str(include_str!("../../../samples/phase1-mini-show.sdc")).unwrap();
    let baseline: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    assert_eq!(baseline.fixtures.len(), 1);
    assert_eq!(baseline.cues.len(), 1);
    assert_eq!(baseline.video.outputs.len(), 1);

    const ITERATIONS: usize = 1_000;
    let measure = |serialize: bool| {
        let mut sync = SnapshotSyncState::default();
        let mut clients = [None; 2];
        let mut current = baseline.clone();
        let mut full_count = 0;
        let mut delta_count = 0;
        let mut serialized_bytes = 0;
        let started = Instant::now();
        for step in 0..ITERATIONS {
            current.clock.bpm = 120.0 + (step % 3) as f32;
            let client = step % 2;
            let payload = sync.publish(clients[client], current.clone()).unwrap();
            clients[client] = Some(payload.revision);
            full_count += usize::from(payload.full.is_some());
            delta_count += usize::from(payload.delta.is_some());
            if serialize {
                serialized_bytes += serde_json::to_vec(&payload).unwrap().len();
            }
            black_box(payload);
        }
        (
            started.elapsed().as_nanos(),
            full_count,
            delta_count,
            serialized_bytes,
        )
    };

    let warmup = measure(true);
    black_box(warmup);
    let mut publish_times = Vec::new();
    let mut serialized_times = Vec::new();
    for round in 0..5 {
        let (publish_ns, full_count, delta_count, _) = measure(false);
        let (serialized_ns, serialized_full, serialized_delta, serialized_bytes) = measure(true);
        assert_eq!((full_count, delta_count), (2, ITERATIONS - 2));
        assert_eq!((serialized_full, serialized_delta), (2, ITERATIONS - 2));
        println!(
            "round={round} iterations={ITERATIONS} publish_ns={publish_ns} publish_serialize_ns={serialized_ns} full/delta={full_count}/{delta_count} serialized_bytes={serialized_bytes}"
        );
        publish_times.push(publish_ns);
        serialized_times.push(serialized_ns);
    }
    publish_times.sort_unstable();
    serialized_times.sort_unstable();
    println!(
        "median iterations={ITERATIONS} publish_ns={} publish_serialize_ns={} full/delta=2/{}",
        publish_times[2],
        serialized_times[2],
        ITERATIONS - 2
    );
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
                full: Some(Arc::new(snapshot.clone())),
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

#[test]
fn snapshot_delta_transmits_authored_and_safety_blackout_independently_of_effective_blackout() {
    let both = EngineSnapshot {
        blackout: true,
        authored_blackout: true,
        safety_blackout_engaged: true,
        ..EngineSnapshot::default()
    };
    for field in ["authored_blackout", "safety_blackout_engaged"] {
        let mut single = both.clone();
        match field {
            "authored_blackout" => single.authored_blackout = false,
            "safety_blackout_engaged" => single.safety_blackout_engaged = false,
            _ => unreachable!(),
        }
        // Effective blackout remains true; the independent cause still changed.
        for (before, after, expected) in [(&both, &single, false), (&single, &both, true)] {
            let mut sync = SnapshotSyncState::default();
            let first = sync.publish(None, before.clone()).unwrap();
            let changed = sync.publish(Some(first.revision), after.clone()).unwrap();
            let json = serde_json::to_value(&changed).unwrap();
            assert_eq!(json["delta"], serde_json::json!({field: expected}));
            let unchanged = sync.publish(Some(changed.revision), after.clone()).unwrap();
            assert_eq!(
                serde_json::to_value(unchanged).unwrap()["delta"],
                serde_json::json!({})
            );
        }
    }
}
