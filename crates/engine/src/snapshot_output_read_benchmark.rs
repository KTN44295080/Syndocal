//! Opt-in comparison on a preserved show image; never starts an engine worker
//! or opens a media/device/output resource.
use super::{allocator_test_handle, EngineRuntime};
use crate::snapshot_public;
use protocol::EngineSnapshot;
use std::{
    hint::black_box,
    sync::{mpsc, Arc, RwLock},
    thread,
    time::Instant,
};

fn measure_contended_snapshot_read(
    handle: &super::EngineHandle,
    published: Arc<RwLock<EngineSnapshot>>,
    iterations: usize,
    operation: impl Fn(&super::EngineHandle) -> bool,
) -> (Vec<u128>, usize) {
    let (go_tx, go_rx) = mpsc::sync_channel(0);
    let (ready_tx, ready_rx) = mpsc::sync_channel(0);
    let (started_tx, started_rx) = mpsc::sync_channel(0);
    let writer = thread::spawn(move || {
        for _ in 0..iterations {
            go_rx.recv().unwrap();
            let guard = published.write().unwrap();
            ready_tx.send(()).unwrap();
            started_rx.recv().unwrap();
            thread::sleep(std::time::Duration::from_millis(2));
            drop(guard);
        }
    });

    let mut samples = Vec::with_capacity(iterations);
    let mut successful_reads = 0;
    for _ in 0..iterations {
        go_tx.send(()).unwrap();
        ready_rx.recv().unwrap();
        let started = Instant::now();
        started_tx.send(()).unwrap();
        if operation(handle) {
            successful_reads += 1;
        }
        samples.push(started.elapsed().as_nanos());
    }
    writer.join().unwrap();
    (samples, successful_reads)
}

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

#[test]
#[ignore = "fixed representative fixture; measurement only, not FPS or lock-wait acceptance"]
fn benchmark_show_snapshot_clone_and_payload() {
    let path = std::env::var_os("SYNDOCAL_SNAPSHOT_BENCH_PROJECT")
        .expect("provide an explicit preserved .sdc path");
    let bytes = std::fs::read(path).expect("read benchmark show without modification");
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    let public = snapshot_public::clone_public_snapshot(&snapshot);
    let payload = serde_json::to_vec(&public).unwrap();
    let combined_payload =
        serde_json::to_vec(&snapshot_public::clone_public_snapshot(&snapshot)).unwrap();
    assert_eq!(payload, combined_payload);

    const ITERATIONS: usize = 1_000;
    let measure = |operation: &dyn Fn()| {
        let started = Instant::now();
        for _ in 0..ITERATIONS {
            operation();
        }
        started.elapsed().as_nanos()
    };
    let clone = || {
        black_box(snapshot_public::clone_public_snapshot(black_box(&snapshot)));
    };
    let encode = || {
        black_box(serde_json::to_vec(black_box(&public)).unwrap());
    };
    let clone_and_encode = || {
        black_box(
            serde_json::to_vec(black_box(&snapshot_public::clone_public_snapshot(
                black_box(&snapshot),
            )))
            .unwrap(),
        );
    };

    black_box(measure(&clone));
    black_box(measure(&encode));
    black_box(measure(&clone_and_encode));
    let mut clone_times = Vec::new();
    let mut encode_times = Vec::new();
    let mut combined_times = Vec::new();
    for round in 0..5 {
        let (clone_ns, encode_ns, combined_ns) = match round % 3 {
            0 => {
                let clone_ns = measure(&clone);
                let encode_ns = measure(&encode);
                let combined_ns = measure(&clone_and_encode);
                (clone_ns, encode_ns, combined_ns)
            }
            1 => {
                let encode_ns = measure(&encode);
                let combined_ns = measure(&clone_and_encode);
                let clone_ns = measure(&clone);
                (clone_ns, encode_ns, combined_ns)
            }
            _ => {
                let combined_ns = measure(&clone_and_encode);
                let clone_ns = measure(&clone);
                let encode_ns = measure(&encode);
                (clone_ns, encode_ns, combined_ns)
            }
        };
        println!(
            "round={round} iterations={ITERATIONS} clone_ns={clone_ns} encode_ns={encode_ns} clone_encode_ns={combined_ns} payload_bytes={}",
            payload.len()
        );
        clone_times.push(clone_ns);
        encode_times.push(encode_ns);
        combined_times.push(combined_ns);
    }
    clone_times.sort_unstable();
    encode_times.sort_unstable();
    combined_times.sort_unstable();
    println!(
        "median iterations={ITERATIONS} clone_ns={} encode_ns={} clone_encode_ns={} payload_bytes={}",
        clone_times[2],
        encode_times[2],
        combined_times[2],
        payload.len()
    );
}

#[test]
#[ignore = "fixed representative fixture; measurement only, not tick budget or FPS acceptance"]
fn benchmark_show_tick_snapshot_construction() {
    let path = std::env::var_os("SYNDOCAL_SNAPSHOT_BENCH_PROJECT")
        .expect("provide an explicit preserved .sdc path");
    let bytes = std::fs::read(path).expect("read benchmark show without modification");
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    let mut runtime = EngineRuntime::new(protocol::DmxOutputConfig {
        enabled: false,
        ..protocol::DmxOutputConfig::default()
    });
    runtime.load_project_snapshot(snapshot);
    assert_eq!(runtime.last_error, None);

    const ITERATIONS: usize = 1_000;
    let measure = || {
        let started = Instant::now();
        for _ in 0..ITERATIONS {
            black_box(runtime.build_snapshot(0));
        }
        started.elapsed().as_nanos()
    };
    black_box(measure());
    let mut times = Vec::new();
    for round in 0..5 {
        let elapsed = measure();
        println!("round={round} iterations={ITERATIONS} build_snapshot_ns={elapsed}");
        times.push(elapsed);
    }
    times.sort_unstable();
    println!(
        "median iterations={ITERATIONS} build_snapshot_ns={}",
        times[2]
    );
}

#[test]
#[ignore = "fixed representative fixture; synthetic writer-wait report, not FPS acceptance"]
fn benchmark_show_snapshot_reads_under_writer_contention() {
    let path = std::env::var_os("SYNDOCAL_SNAPSHOT_BENCH_PROJECT")
        .expect("provide an explicit preserved .sdc path");
    let bytes = std::fs::read(path).expect("read benchmark show without modification");
    let document: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    let snapshot: EngineSnapshot = serde_json::from_value(document["snapshot"].clone()).unwrap();
    let published = Arc::new(RwLock::new(snapshot));
    let handle = allocator_test_handle(Arc::clone(&published));
    const ITERATIONS: usize = 100;

    let (mut full, full_successes) =
        measure_contended_snapshot_read(&handle, Arc::clone(&published), ITERATIONS, |handle| {
            black_box(handle.snapshot());
            true
        });
    let (mut narrow, narrow_successes) =
        measure_contended_snapshot_read(&handle, Arc::clone(&published), ITERATIONS, |handle| {
            black_box(handle.video_outputs_snapshot());
            true
        });
    let (mut try_read, try_successes) =
        measure_contended_snapshot_read(&handle, published, ITERATIONS, |handle| {
            black_box(handle.try_snapshot()).is_some()
        });

    full.sort_unstable();
    narrow.sort_unstable();
    try_read.sort_unstable();
    println!(
        "writer_contention iterations={ITERATIONS} hold_ms=2 full_median_ns={} narrow_median_ns={} try_median_ns={} full_successes={full_successes} narrow_successes={narrow_successes} try_successes={try_successes}",
        full[ITERATIONS / 2],
        narrow[ITERATIONS / 2],
        try_read[ITERATIONS / 2],
    );
}
