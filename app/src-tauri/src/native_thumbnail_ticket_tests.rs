use super::*;
use std::sync::mpsc;

#[test]
fn ticket_is_window_and_lane_scoped_and_retained_until_worker_return() {
    let work = NativeThumbnailWork::default();
    let job = work.layers.try_acquire("main").unwrap();
    let ticket = job.ticket(ThumbnailLane::Layer);
    assert!(!work.cancel("other", &ticket).unwrap());
    let mut wrong_lane = ticket.clone();
    wrong_lane.lane = ThumbnailLane::Asset;
    assert!(!work.cancel("main", &wrong_lane).unwrap());
    assert!(!job.request.cancelled.load(Ordering::Acquire));
    assert!(work.cancel("main", &ticket).unwrap());
    assert!(work.cancel("main", &ticket).unwrap());
    assert!(work.layers.try_acquire("main").is_err());
    let result: Result<(), String> = job.execute(|_| panic!("cancelled queued job executed"));
    assert!(result.unwrap_err().contains("cancelled"));
    let successor = work.layers.try_acquire("main").unwrap();
    assert_ne!(
        successor.ticket(ThumbnailLane::Layer).request_id,
        ticket.request_id
    );
    assert!(!work.cancel("main", &ticket).unwrap());
    assert!(!successor.request.cancelled.load(Ordering::Acquire));
}

#[test]
fn invalid_or_unknown_tickets_do_not_signal_the_active_job() {
    let work = NativeThumbnailWork::default();
    let job = work.assets.try_acquire("main").unwrap();
    let ticket = job.ticket(ThumbnailLane::Asset);
    for id in ["", "x", "ABCDEF", "../path"] {
        let mut invalid = ticket.clone();
        invalid.request_id = id.into();
        assert!(work.cancel("main", &invalid).is_err());
    }
    let mut future = ticket.clone();
    future.schema_version = 2;
    assert!(work.cancel("main", &future).is_err());
    let unknown = ThumbnailTicket {
        request_id: "0".repeat(32),
        ..ticket.clone()
    };
    if unknown.request_id != ticket.request_id {
        assert!(!work.cancel("main", &unknown).unwrap());
    }
    assert!(!job.request.cancelled.load(Ordering::Acquire));
    let encoded = serde_json::to_value(&ticket).unwrap();
    assert!(encoded.get("schemaVersion").is_some());
    assert!(encoded.get("requestId").is_some());
    let mut extra = encoded;
    extra["owner"] = serde_json::json!("other");
    assert!(serde_json::from_value::<ThumbnailTicket>(extra).is_err());
    assert!(work.assets.try_acquire("main").is_err());
}

#[test]
fn explicit_running_cancel_retains_slot_and_rejects_late_success() {
    let work = NativeThumbnailWork::default();
    let job = work.layers.try_acquire("main").unwrap();
    let ticket = job.ticket(ThumbnailLane::Layer);
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
    assert!(work.cancel("main", &ticket).unwrap());
    assert!(work.layers.try_acquire("main").is_err());
    release_tx.send(()).unwrap();
    assert!(worker.join().unwrap().unwrap_err().contains("cancelled"));
    assert!(!work.cancel("main", &ticket).unwrap());
    assert!(work.layers.try_acquire("main").is_ok());
}

#[test]
fn concurrent_lanes_only_cancel_the_exact_target() {
    let work = NativeThumbnailWork::default();
    let layer = work.layers.try_acquire("main").unwrap();
    let asset = work.assets.try_acquire("main").unwrap();
    let ticket = layer.ticket(ThumbnailLane::Layer);
    assert!(work.cancel("main", &ticket).unwrap());
    assert!(!asset.request.cancelled.load(Ordering::Acquire));
    assert_eq!(asset.execute(|_| Ok(42)).unwrap(), 42);
    assert!(work.layers.try_acquire("main").is_err());
    let result: Result<(), String> = layer.execute(|_| panic!("cancelled layer executed"));
    assert!(result.is_err());
}

#[test]
fn tauri_channel_serializes_a_valid_exact_ticket() {
    let gate = ThumbnailGate::default();
    let job = gate.try_acquire("main").unwrap();
    let ticket = job.ticket(ThumbnailLane::Layer);
    let (tx, rx) = mpsc::sync_channel(1);
    let channel = tauri::ipc::Channel::<ThumbnailTicket>::new(move |body| {
        tx.send(body).unwrap();
        Ok(())
    });
    channel.send(ticket.clone()).unwrap();
    let tauri::ipc::InvokeResponseBody::Json(json) = rx.recv().unwrap() else {
        panic!("expected JSON");
    };
    let received: ThumbnailTicket = serde_json::from_str(&json).unwrap();
    assert_eq!(received.request_id, ticket.request_id);
    received.validate().unwrap();
}

#[test]
fn failed_channel_announcement_drops_the_unstarted_job() {
    let gate = ThumbnailGate::default();
    let result = (|| -> Result<(), String> {
        let job = gate.try_acquire("main")?;
        let channel = tauri::ipc::Channel::<ThumbnailTicket>::new(|_| {
            Err(std::io::Error::other("closed channel fixture").into())
        });
        channel
            .send(job.ticket(ThumbnailLane::Layer))
            .map_err(|error| error.to_string())?;
        panic!("failed announcement must not start decode");
    })();
    assert!(result.unwrap_err().contains("closed channel fixture"));
    assert!(gate.try_acquire("main").is_ok());
}
