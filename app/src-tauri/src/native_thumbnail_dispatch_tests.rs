use super::*;
use crate::native_thumbnail_work::ThumbnailGate;
use std::{
    future::Future,
    sync::{mpsc, Arc},
    task::{Context, Poll, Wake, Waker},
    time::{Duration, Instant},
};

#[test]
fn blocking_dispatch_returns_success_and_errors_without_leaking_admission() {
    let gate = ThumbnailGate::default();
    let result = tauri::async_runtime::block_on(run(gate.try_acquire().unwrap(), |_| Ok(42)));
    assert_eq!(result.unwrap(), 42);
    let result: Result<(), String> =
        tauri::async_runtime::block_on(run(gate.try_acquire().unwrap(), |_| {
            Err("decode failed".into())
        }));
    assert_eq!(result.unwrap_err(), "decode failed");
    assert!(gate.try_acquire().is_ok());
}

#[test]
fn dropping_async_waiter_does_not_admit_a_second_running_job() {
    struct NoopWake;
    impl Wake for NoopWake {
        fn wake(self: Arc<Self>) {}
    }
    let gate = ThumbnailGate::default();
    let (started_tx, started_rx) = mpsc::sync_channel(1);
    let (release_tx, release_rx) = mpsc::sync_channel(1);
    let (cancel_tx, cancel_rx) = mpsc::sync_channel(1);
    let mut request = Box::pin(run(gate.try_acquire().unwrap(), move |cancel| {
        started_tx.send(()).unwrap();
        release_rx.recv_timeout(Duration::from_secs(5)).unwrap();
        cancel_tx
            .send(cancel.load(std::sync::atomic::Ordering::Acquire))
            .unwrap();
        Ok(42)
    }));
    let waker = Waker::from(Arc::new(NoopWake));
    assert!(matches!(
        request.as_mut().poll(&mut Context::from_waker(&waker)),
        Poll::Pending
    ));
    started_rx.recv_timeout(Duration::from_secs(5)).unwrap();
    drop(request);
    assert!(
        gate.try_acquire().is_err(),
        "dropped waiter must not release the executing worker"
    );
    release_tx.send(()).unwrap();
    assert!(cancel_rx.recv_timeout(Duration::from_secs(5)).unwrap());
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Ok(job) = gate.try_acquire() {
            drop(job);
            break;
        }
        assert!(
            Instant::now() < deadline,
            "finished worker retained its admission"
        );
        std::thread::sleep(Duration::from_millis(1));
    }
}

#[test]
fn panicked_worker_reports_failure_and_releases_admission() {
    let gate = ThumbnailGate::default();
    let result: Result<(), String> =
        tauri::async_runtime::block_on(run(gate.try_acquire().unwrap(), |_| panic!("fixture")));
    assert!(result.unwrap_err().contains("Thumbnail worker failed"));
    assert!(gate.try_acquire().is_ok());
}
