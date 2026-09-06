//! Opt-in comparison on a preserved show image; never starts an engine worker
//! or opens a media/device/output resource.
use super::allocator_test_handle;
use protocol::EngineSnapshot;
use std::{
    hint::black_box,
    sync::{Arc, RwLock},
    time::Instant,
};

#[test]
#[ignore = "requires SYNDOCAL_SNAPSHOT_BENCH_PROJECT; isolated read cost, not native FPS"]
fn benchmark_show_video_outputs_snapshot() {
    let path = std::env::var_os("SYNDOCAL_SNAPSHOT_BENCH_PROJECT")
        .expect("provide an explicit preserved .sdc path");
    let bytes = std::fs::read(path).expect("read benchmark show without modification");
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    assert!(
        !snapshot.video.outputs.is_empty(),
        "use a populated output fixture"
    );
    println!(
        "show_bytes={} fixtures={} cues={} outputs={}",
        bytes.len(),
        snapshot.fixtures.len(),
        snapshot.cues.len(),
        snapshot.video.outputs.len()
    );
    let handle = allocator_test_handle(Arc::new(RwLock::new(snapshot)));
    assert_eq!(
        handle.snapshot().video.outputs,
        handle.video_outputs_snapshot()
    );
    assert_eq!(
        handle.snapshot().timeline.playing,
        handle.timeline_playing()
    );
    let mut previous = Vec::new();
    let mut narrow = Vec::new();
    const ITERATIONS: usize = 2_000;
    let measure = |old: bool| {
        let start = Instant::now();
        for _ in 0..ITERATIONS {
            if old {
                black_box(handle.snapshot().timeline.playing);
                black_box(handle.snapshot().video.outputs);
            } else {
                black_box(handle.timeline_playing());
                black_box(handle.video_outputs_snapshot());
            }
        }
        start.elapsed().as_nanos()
    };
    // Warm both paths and alternate order to avoid attributing startup/order
    // bias to the new reader. No environment-dependent speed assertion.
    black_box(measure(true));
    black_box(measure(false));
    for round in 0..5 {
        let (old, new) = if round % 2 == 0 {
            (measure(true), measure(false))
        } else {
            let new = measure(false);
            (measure(true), new)
        };
        previous.push(old);
        narrow.push(new);
        println!("round={round} iterations={ITERATIONS} full_ns={old} narrow_ns={new}");
    }
    previous.sort_unstable();
    narrow.sort_unstable();
    println!(
        "median iterations={ITERATIONS} full_ns={} narrow_ns={}",
        previous[2], narrow[2]
    );
}
