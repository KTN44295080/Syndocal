use super::*;

#[test]
fn pending_head_burst_is_refused_before_worker_spawn() {
    let mut server = RemoteWsServer::start(
        RemoteControlConfig {
            bind_ip: "127.0.0.1".to_string(),
            port: 0,
            pairing_pin: "123456".to_string(),
            ..RemoteControlConfig::default()
        },
        |_| {},
    )
    .unwrap();
    let address = server.local_addr();
    let cap = REMOTE_MAX_PENDING_HEADS_PER_IP as usize;
    // Hold real workers before head processing starts. Admission must remain
    // bounded even when worker scheduling is delayed; no timer holds this test
    // open and slow test-host scheduling cannot expire the admitted slots.
    let worker_start_gate = server.worker_start_gate.lock().unwrap();
    let holders = (0..cap)
        .map(|_| {
            let mut stream = TcpStream::connect(address).unwrap();
            // Buffer an incomplete head for the admitted worker.
            stream.write_all(b"GET /").unwrap();
            stream
        })
        .collect::<Vec<_>>();
    let live_workers = || {
        server
            .client_workers
            .lock()
            .unwrap()
            .iter()
            .filter(|worker| !worker.is_finished())
            .count()
    };
    let ready_deadline = Instant::now() + Duration::from_secs(2);
    while live_workers() < cap {
        assert!(Instant::now() < ready_deadline, "holders were not admitted");
        thread::yield_now();
    }

    // Simultaneously connect four times as many peers as the per-IP bound.
    // No request is sent: admission must refuse them before any head read.
    let excess = cap * 4;
    let barrier = Arc::new(std::sync::Barrier::new(excess + 1));
    let (results, received) = mpsc::channel();
    let peers = (0..excess)
        .map(|_| {
            let barrier = Arc::clone(&barrier);
            let results = results.clone();
            thread::spawn(move || {
                barrier.wait();
                let mut stream = TcpStream::connect(address).unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(2)))
                    .unwrap();
                let mut response = Vec::new();
                stream.read_to_end(&mut response).unwrap();
                results.send(String::from_utf8(response).unwrap()).unwrap();
            })
        })
        .collect::<Vec<_>>();
    barrier.wait();
    let mut peak_live_workers = live_workers();
    for _ in 0..excess {
        let response = received.recv_timeout(Duration::from_secs(3)).unwrap();
        assert!(response.starts_with("HTTP/1.1 503"), "{response}");
        peak_live_workers = peak_live_workers.max(live_workers());
        assert!(peak_live_workers <= cap, "pending workers exceeded {cap}");
        // This monotonic counter also catches short-lived rejected workers
        // that could finish between live-worker observations.
        assert_eq!(
            server.next_worker_id.load(Ordering::Acquire) - 1,
            cap as u64
        );
    }
    for peer in peers {
        peer.join().unwrap();
    }
    assert_eq!(peak_live_workers, cap);
    assert_eq!(server.status().rejected_connections, excess as u64);
    println!(
        "pending-head bound proof: holders={cap}, simultaneous_excess={excess}, peak_live_workers={peak_live_workers}, spawned_workers={cap}, refused={excess}"
    );
    drop(holders);
    drop(worker_start_gate);
    server.stop_and_join();
    assert!(server.client_workers.lock().unwrap().is_empty());
    assert!(server.shutdown_sockets.lock().unwrap().is_empty());
}
