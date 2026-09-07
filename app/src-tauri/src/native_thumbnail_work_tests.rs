use super::*;
use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    sync::mpsc,
};

#[test]
fn one_job_per_lane_and_independent_asset_lane() {
    let work = NativeThumbnailWork::default();
    let layer = work.layers.try_acquire().unwrap();
    for _ in 0..100 {
        assert!(work.layers.try_acquire().is_err());
    }
    let asset = work.assets.try_acquire().unwrap();
    assert!(work.assets.try_acquire().is_err());
    drop(layer);
    assert!(work.layers.try_acquire().is_ok());
    assert!(work.assets.try_acquire().is_err());
    drop(asset);
    assert!(work.assets.try_acquire().is_ok());
}

#[test]
fn cancelled_queued_job_never_starts_and_retains_slot_until_dropped() {
    let gate = ThumbnailGate::default();
    let job = gate.try_acquire().unwrap();
    drop(job.request_guard());
    assert!(gate.try_acquire().is_err());
    let result: Result<(), String> = job.execute(|_| panic!("cancelled job started"));
    assert!(result.unwrap_err().contains("cancelled"));
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn cancellation_keeps_running_worker_owned_and_rejects_late_success() {
    let gate = ThumbnailGate::default();
    let job = gate.try_acquire().unwrap();
    let waiter = job.request_guard();
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let worker = thread::spawn(move || {
        job.execute(|cancel| {
            started_tx.send(()).unwrap();
            release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
            assert!(cancel.load(Ordering::Acquire));
            Ok(42)
        })
    });
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(waiter);
    assert!(gate.try_acquire().is_err());
    release_tx.send(()).unwrap();
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn success_error_and_unwind_release_the_slot() {
    let gate = ThumbnailGate::default();
    assert_eq!(gate.try_acquire().unwrap().execute(|_| Ok(7)).unwrap(), 7);
    assert_eq!(
        gate.try_acquire()
            .unwrap()
            .execute::<()>(|_| Err("decode".into()))
            .unwrap_err(),
        "decode"
    );
    assert!(catch_unwind(AssertUnwindSafe(|| gate
        .try_acquire()
        .unwrap()
        .execute::<()>(|_| panic!("fixture"))))
    .is_err());
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn concurrent_admission_has_one_winner() {
    let gate = Arc::new(ThumbnailGate::default());
    let start = Arc::new(std::sync::Barrier::new(17));
    let workers: Vec<_> = (0..16)
        .map(|_| {
            let gate = Arc::clone(&gate);
            let start = Arc::clone(&start);
            thread::spawn(move || {
                start.wait();
                gate.try_acquire().ok()
            })
        })
        .collect();
    start.wait();
    let jobs: Vec<_> = workers
        .into_iter()
        .filter_map(|worker| worker.join().unwrap())
        .collect();
    assert_eq!(jobs.len(), 1);
    assert!(gate.try_acquire().is_err());
    drop(jobs);
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn renderer_wait_rejects_cancelled_and_expired_requests() {
    let renderer = Mutex::new(42);
    let cancelled = AtomicBool::new(true);
    assert!(lock_renderer(&renderer, &cancelled)
        .unwrap_err()
        .contains("cancelled"));
    let active = AtomicBool::new(false);
    assert!(lock_renderer_until(&renderer, &active, Instant::now())
        .unwrap_err()
        .contains("timed out"));
    assert_eq!(*lock_renderer(&renderer, &active).unwrap(), 42);
}

#[test]
fn held_renderer_wait_expires_without_taking_ownership() {
    let renderer = Mutex::new(42);
    let held = renderer.lock().unwrap();
    let active = AtomicBool::new(false);
    let result = lock_renderer_until(
        &renderer,
        &active,
        Instant::now() + Duration::from_millis(20),
    );
    assert!(result.unwrap_err().contains("timed out"));
    assert_eq!(*held, 42);
}

#[test]
fn held_renderer_wait_observes_cancellation() {
    let renderer = Arc::new(Mutex::new(42));
    let held = renderer.lock().unwrap();
    let cancel = Arc::new(AtomicBool::new(false));
    let worker_renderer = Arc::clone(&renderer);
    let worker_cancel = Arc::clone(&cancel);
    let worker = thread::spawn(move || lock_renderer(&worker_renderer, &worker_cancel).map(|_| ()));
    cancel.store(true, Ordering::Release);
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    assert_eq!(*held, 42);
}

#[test]
fn poisoned_renderer_is_not_recovered_permissively() {
    let renderer = Mutex::new(42);
    let _ = catch_unwind(AssertUnwindSafe(|| {
        let _held = renderer.lock().unwrap();
        panic!("fixture");
    }));
    assert!(lock_renderer(&renderer, &AtomicBool::new(false))
        .unwrap_err()
        .contains("poisoned"));
}
