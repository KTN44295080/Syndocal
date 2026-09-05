use super::{create_effect_only_cue, runtime_with_lfo_effects};
use crate::EngineRuntime;
use protocol::{TimelineId, TimelineSnapshot, TimelineTrackKind};

fn previous_bank_snapshot(runtime: &EngineRuntime) -> Vec<TimelineSnapshot> {
    let active = runtime.authored_timeline_snapshot();
    let mut bank = runtime.timeline_bank.clone();
    if let Some(entry) = bank
        .iter_mut()
        .find(|entry| entry.id == runtime.timeline_id)
    {
        *entry = active;
    } else {
        bank.push(active);
    }
    bank
}

fn bank_entry(id: u64, index: usize) -> TimelineSnapshot {
    TimelineSnapshot {
        id: TimelineId(id),
        label: format!("stored {index}"),
        duration_ms: 1_000 + index as u64,
        position_ms: 100 + index as u64,
        playing: true,
        ..TimelineSnapshot::default()
    }
}

#[test]
fn timeline_bank_snapshot_matches_previous_for_active_positions_missing_and_duplicates() {
    let mut runtime = runtime_with_lfo_effects(&[]);
    runtime.timeline_id = TimelineId(42);
    runtime.timeline_label = "fresh active".to_string();
    runtime.timeline_playing = true;
    runtime.timeline_position_ms = 875;
    // Duplicates are deliberately tested at this internal boundary: preserve
    // first-match replacement even though project validation rejects duplicates.
    for ids in [
        vec![],
        vec![42],
        vec![42, 2, 3],
        vec![1, 42, 3],
        vec![1, 2, 42],
        vec![1, 2, 3],
        vec![1, 42, 42, 3],
    ] {
        runtime.timeline_bank = ids
            .iter()
            .enumerate()
            .map(|(index, id)| bank_entry(*id, index))
            .collect();
        let stored_before = runtime.timeline_bank.clone();
        let expected = previous_bank_snapshot(&runtime);
        let mut actual = runtime.timeline_bank_snapshot();
        assert_eq!(actual, expected, "bank IDs {ids:?}");
        assert_eq!(runtime.timeline_bank, stored_before);

        let active_index = ids.iter().position(|id| *id == 42).unwrap_or(ids.len());
        assert_eq!(actual[active_index].label, "fresh active");
        assert!(!actual[active_index].playing);
        assert_eq!(actual[active_index].position_ms, 0);
        for entry in &mut actual {
            entry.label.push_str(" changed by reader");
            entry.layers.clear();
        }
        assert_eq!(runtime.timeline_bank, stored_before);
        assert_eq!(runtime.timeline_label, "fresh active");
        assert_eq!(runtime.timeline_position_ms, 875);
    }
}

#[test]
#[ignore = "fixed synthetic clone-cost measurement, not a real-show performance gate"]
fn benchmark_timeline_bank_snapshot_selective_clone() {
    use std::{hint::black_box, time::Instant};

    let mut runtime = runtime_with_lfo_effects(&[]);
    create_effect_only_cue(&mut runtime, 1, Vec::new());
    runtime
        .add_timeline_cue_event_state(1, 1, 0, None, TimelineTrackKind::Lighting, None)
        .unwrap();
    let seed = runtime.timeline_events[0].clone();
    runtime.timeline_events = (0..4_096)
        .map(|index| {
            let mut event = seed.clone();
            event.id = index + 1;
            event.time_ms = index * 1_000;
            event
        })
        .collect();
    runtime.timeline_id = TimelineId(42);
    runtime.timeline_bank = vec![
        bank_entry(1, 0),
        runtime.authored_timeline_snapshot(),
        bank_entry(3, 2),
    ];
    assert_eq!(runtime.timeline_bank[1].events.len(), 4_096);
    assert_eq!(
        runtime.timeline_bank_snapshot(),
        previous_bank_snapshot(&runtime)
    );

    let iterations = 10_000;
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(previous_bank_snapshot(black_box(&runtime)));
    }
    let previous = started.elapsed();
    let started = Instant::now();
    for _ in 0..iterations {
        black_box(black_box(&runtime).timeline_bank_snapshot());
    }
    let selective = started.elapsed();
    println!(
        "timeline bank, {iterations} iterations, 4096 active events, 3 entries: previous={previous:?}, selective={selective:?}"
    );
}
