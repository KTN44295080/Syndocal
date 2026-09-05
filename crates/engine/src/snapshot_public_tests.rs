use super::*;

fn previous_public(snapshot: &EngineSnapshot) -> EngineSnapshot {
    let mut public = snapshot.clone();
    public.authored_video = None;
    public
}

fn populated_publication() -> EngineSnapshot {
    let mut runtime = runtime_with_lfo_effects(&[(1, true)]);
    add_runtime_test_video_layer(&mut runtime, 1, VideoLayerState::default());
    create_effect_only_cue(&mut runtime, 1, Vec::new());
    runtime.timeline_transport_epoch = 23;
    runtime.timeline_transport_generation = 24;
    runtime.timeline_follow_runtime.fault = Some("captured runtime fault".into());
    let mut snapshot = runtime.build_snapshot(0);
    snapshot
        .group_colors
        .insert("stage".into(), "#123456".into());
    snapshot.dmx_preview = vec![127; 512];
    snapshot
}

#[test]
fn public_snapshot_copy_matches_all_previous_fields_and_preserves_independent_ownership() {
    for snapshot in [EngineSnapshot::default(), populated_publication()] {
        let published = Arc::new(RwLock::new(snapshot.clone()));
        let handle = allocator_test_handle(Arc::clone(&published));
        let expected = previous_public(&snapshot);
        assert_eq!(handle.snapshot(), expected);
        assert_eq!(handle.try_snapshot(), Some(expected.clone()));
        let mut retained = handle.snapshot();
        retained.timeline.label.push_str(" reader edit");
        retained.video.layers.clear();
        assert_eq!(*published.read().unwrap(), snapshot);
        *published.write().unwrap() = EngineSnapshot::default();
        assert_eq!(expected, previous_public(&snapshot));
        assert_eq!(
            handle.snapshot(),
            previous_public(&EngineSnapshot::default())
        );
    }
}

#[test]
fn public_snapshot_copy_keeps_try_lock_and_distinct_poison_policies() {
    let snapshot = populated_publication();
    let published = Arc::new(RwLock::new(snapshot.clone()));
    let handle = allocator_test_handle(Arc::clone(&published));
    let guard = published.write().unwrap();
    assert!(handle.try_snapshot().is_none());
    drop(guard);
    assert_eq!(handle.try_snapshot(), Some(previous_public(&snapshot)));
    let poison = std::thread::spawn(move || {
        let _guard = published.write().unwrap();
        panic!("deliberate public snapshot poison");
    });
    assert!(poison.join().is_err());
    assert_eq!(handle.snapshot(), EngineSnapshot::default());
    assert_eq!(handle.try_snapshot(), Some(previous_public(&snapshot)));
}

#[test]
#[ignore = "fixed public-copy workload; not end-to-end FPS or lock-wait acceptance"]
fn benchmark_public_snapshot_copy_without_authored_video() {
    use std::hint::black_box;
    fn measure(input: &EngineSnapshot, copy: fn(&EngineSnapshot) -> EngineSnapshot) -> Duration {
        let started = Instant::now();
        for _ in 0..1_000 {
            black_box(copy(black_box(input)));
        }
        started.elapsed()
    }
    let light = EngineSnapshot::default();
    let mut heavy = populated_publication();
    let video = heavy.authored_video.as_mut().unwrap();
    let seed = video.layers[0].clone();
    video.layers = (0..1_024)
        .map(|index| {
            let mut layer = seed.clone();
            layer.id = index + 1;
            layer.label = format!("{index}: {}", "authored label ".repeat(16));
            layer
        })
        .collect();
    for (name, input) in [("light", &light), ("1024 authored layers", &heavy)] {
        assert_eq!(
            previous_public(input),
            snapshot_public::clone_public_snapshot(input)
        );
        let mut old_times = Vec::new();
        let mut new_times = Vec::new();
        for round in 0..3 {
            let (old, new) = if round % 2 == 0 {
                (
                    measure(input, previous_public),
                    measure(input, snapshot_public::clone_public_snapshot),
                )
            } else {
                let new = measure(input, snapshot_public::clone_public_snapshot);
                (measure(input, previous_public), new)
            };
            println!("public-copy {name} round {round}: previous={old:?}, current={new:?}");
            old_times.push(old);
            new_times.push(new);
        }
        old_times.sort_unstable();
        new_times.sort_unstable();
        println!(
            "public-copy {name} median1000: previous={:?}, current={:?}",
            old_times[1], new_times[1]
        );
    }
}
