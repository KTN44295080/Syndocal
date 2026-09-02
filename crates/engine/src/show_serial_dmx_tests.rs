use super::*;

use std::{
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Barrier, Mutex, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

fn disabled_open_dmx_output() -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: false,
        protocol: DmxOutputProtocol::EnttecOpenDmx,
        target_ip: String::new(),
        port: 0,
        universe: 0,
        serial_port: String::new(),
        serial_baud_rate: 250_000,
    }
}

fn live_open_dmx_runtime_with_test_sender(
    shared_telemetry: Arc<EngineSharedTelemetry>,
    sender: EnttecOpenDmxSender,
) -> EngineRuntime {
    let mut runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), shared_telemetry);
    runtime.output.enabled = true;
    runtime.dmx_sender = Some(DmxSender::EnttecOpenDmx(sender));
    runtime.apply_command(EngineCommand::PatchFixture {
        fixture_id: 1,
        request: PatchFixtureRequest {
            profile_path: "memory://open-dmx-s0-physical-proof.gdtf".to_string(),
            mode_name: Some("Standard".to_string()),
            label: "Open DMX S0 physical-proof fixture".to_string(),
            universe: 0,
            address: 1,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Default::default(),
        },
        profile: sample_profile(),
    });
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".to_string(),
        value: u16::MAX,
    });
    runtime
}

fn wait_for_open_dmx_test_write_count(
    observation: &io::serial_dmx::OpenDmxTestSerialObservation,
    minimum: usize,
) -> Vec<io::serial_dmx::OpenDmxTestSerialOperation> {
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        let operations = observation
            .operations()
            .expect("test serial observation must remain available");
        if operations
            .iter()
            .filter(|operation| {
                matches!(
                    operation,
                    io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(_)
                )
            })
            .count()
            >= minimum
        {
            return operations;
        }
        thread::sleep(Duration::from_millis(1));
    }
    panic!("Open DMX fake serial port did not complete {minimum} write_all operations");
}

fn assert_open_dmx_physical_operation_order(
    operations: &[io::serial_dmx::OpenDmxTestSerialOperation],
) {
    assert!(!operations.is_empty());
    assert_eq!(operations.len() % 4, 0);
    for transaction in operations.chunks_exact(4) {
        assert!(matches!(
            transaction,
            [
                io::serial_dmx::OpenDmxTestSerialOperation::SetBreak,
                io::serial_dmx::OpenDmxTestSerialOperation::ClearBreak,
                io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(_),
                io::serial_dmx::OpenDmxTestSerialOperation::Flush,
            ]
        ));
    }
}

#[test]
fn bounded_show_serial_dmx_sender_open_times_out_and_blocks_stacked_opens() {
    let identity = exact_show_serial_dmx_test_identity();
    let safety_gate = OpenDmxSafetyWriteGate::new();
    let started = Instant::now();
    let error = match run_bounded_show_serial_dmx_sender_open(
        identity.clone(),
        safety_gate.clone(),
        Duration::from_millis(10),
        |_identity, _safety_gate| {
            thread::sleep(Duration::from_millis(75));
            Ok(DmxSender::TestExactArtNetRoute)
        },
    ) {
        Ok(_) => panic!("a sender open beyond its deadline must fail closed"),
        Err(error) => error,
    };
    assert!(started.elapsed() < Duration::from_millis(60));
    assert!(error.contains("exceeded 10ms"));

    // The late worker is allowed to finish and the reaper owns its result;
    // only then may a fresh open become eligible.  This also proves the
    // process-wide single-flight barrier is released on a normal return.
    thread::sleep(Duration::from_millis(100));
    let sender = run_bounded_show_serial_dmx_sender_open(
        identity,
        safety_gate,
        Duration::from_secs(1),
        |_identity, _safety_gate| Ok(DmxSender::TestExactArtNetRoute),
    )
    .expect("a completed late result must release the single-flight barrier");
    assert!(matches!(sender, DmxSender::TestExactArtNetRoute));
}

#[test]
fn published_s0_linearizes_with_the_actual_open_dmx_serial_worker_transaction() {
    // S0-first: the public priority API latches the shared zero-only
    // direction before its queue item is consumed. A subsequently queued
    // live frame must reach the real worker as BREAK/MAB/write_all/flush
    // zero-only.
    let s0_first_shared = Arc::new(EngineSharedTelemetry::new());
    let (s0_first_port, s0_first_observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let s0_first_sender = EnttecOpenDmxSender::from_test_serial_port(
        s0_first_port,
        s0_first_shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut s0_first_runtime =
        live_open_dmx_runtime_with_test_sender(Arc::clone(&s0_first_shared), s0_first_sender);
    let s0_first_snapshot = Arc::new(RwLock::new(s0_first_runtime.build_snapshot(0)));
    let mut s0_first_handle = allocator_test_handle(Arc::clone(&s0_first_snapshot));
    s0_first_handle.shared_telemetry = Arc::clone(&s0_first_shared);
    let s0_first_public_handle = s0_first_handle.clone();
    let s0_first = thread::spawn(move || {
        s0_first_public_handle
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(1))
    });
    let deadline = Instant::now() + Duration::from_secs(1);
    while s0_first_handle.safety_queue.is_empty() && Instant::now() < deadline {
        thread::yield_now();
    }
    assert_eq!(s0_first_handle.safety_queue.len(), 1);
    assert!(s0_first_shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("public S0 must reserve the physical worker gate"));
    s0_first_runtime.tick(1, &s0_first_snapshot);
    let s0_first_operations = wait_for_open_dmx_test_write_count(&s0_first_observation, 1);
    s0_first_runtime.consume_commands(&s0_first_handle.safety_queue);
    s0_first_runtime.publish_pending_command_acks(0, &s0_first_snapshot);
    assert_eq!(
        s0_first.join().expect("public S0 caller must not deadlock"),
        Ok(SafetyBlackoutEngageDisposition::Applied)
    );
    s0_first_runtime.dmx_sender = None;
    assert_open_dmx_physical_operation_order(&s0_first_operations);
    assert!(s0_first_operations.iter().all(|operation| {
        match operation {
            io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload) => {
                payload.len() == io::serial_dmx::ENTTEC_OPEN_DMX_PAYLOAD_LEN
                    && payload[0] == io::serial_dmx::ENTTEC_OPEN_DMX_START_CODE
                    && payload[1..].iter().all(|value| *value == 0)
            }
            _ => true,
        }
    }));

    // Worker-first: block the fake serial port immediately before the
    // actual write_all, while the worker owns the physical gate. Public
    // S0 must still enqueue and latch immediately; the one already
    // selected live transaction may finish, while every later selection
    // is zero-only.
    let worker_first_shared = Arc::new(EngineSharedTelemetry::new());
    let physical_entered = Arc::new(Barrier::new(2));
    let physical_release = Arc::new(Barrier::new(2));
    let physical_once = Arc::new(AtomicBool::new(true));
    let (worker_first_port, worker_first_observation) =
        io::serial_dmx::OpenDmxTestSerialPort::new_with_before_write_all({
            let physical_entered = Arc::clone(&physical_entered);
            let physical_release = Arc::clone(&physical_release);
            let physical_once = Arc::clone(&physical_once);
            move || {
                if physical_once.swap(false, Ordering::AcqRel) {
                    physical_entered.wait();
                    physical_release.wait();
                }
            }
        });
    let worker_first_sender = EnttecOpenDmxSender::from_test_serial_port(
        worker_first_port,
        worker_first_shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut worker_first_runtime = live_open_dmx_runtime_with_test_sender(
        Arc::clone(&worker_first_shared),
        worker_first_sender,
    );
    let worker_first_snapshot = Arc::new(RwLock::new(worker_first_runtime.build_snapshot(0)));
    let mut worker_first_handle = allocator_test_handle(Arc::clone(&worker_first_snapshot));
    worker_first_handle.shared_telemetry = Arc::clone(&worker_first_shared);
    worker_first_runtime.tick(0, &worker_first_snapshot);
    physical_entered.wait();
    let (public_s0_started_tx, public_s0_started_rx) = mpsc::sync_channel(1);
    let worker_first_public_handle = worker_first_handle.clone();
    let worker_first_s0 = thread::spawn(move || {
        public_s0_started_tx
            .send(())
            .expect("test must observe the public S0 call");
        worker_first_public_handle
            .safety_blackout_engage_published(Instant::now() + Duration::from_secs(1))
    });
    public_s0_started_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("public S0 thread must begin while write_all is held");
    let deadline = Instant::now() + Duration::from_secs(1);
    while worker_first_handle.safety_queue.is_empty() && Instant::now() < deadline {
        thread::yield_now();
    }
    assert_eq!(
        worker_first_handle.safety_queue.len(),
        1,
        "the public S0 queue admission must not wait for the wedged USB physical gate"
    );
    assert!(worker_first_shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("the atomic S0 latch must be visible while write_all is held"),);
    physical_release.wait();
    let worker_first_operations = wait_for_open_dmx_test_write_count(&worker_first_observation, 2);
    worker_first_runtime.consume_commands(&worker_first_handle.safety_queue);
    worker_first_runtime.publish_pending_command_acks(0, &worker_first_snapshot);
    assert_eq!(
        worker_first_s0
            .join()
            .expect("public S0 caller must not deadlock"),
        Ok(SafetyBlackoutEngageDisposition::Applied)
    );
    worker_first_runtime.dmx_sender = None;
    assert_open_dmx_physical_operation_order(&worker_first_operations);
    let writes = worker_first_operations
        .iter()
        .filter_map(|operation| match operation {
            io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload) => Some(payload),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(writes[0][1..].iter().any(|value| *value != 0));
    assert!(writes[1..]
        .iter()
        .all(|payload| payload[1..].iter().all(|value| *value == 0)));
}

fn show_serial_dmx_mirror_runtime_with_test_sender(
    shared_telemetry: Arc<EngineSharedTelemetry>,
    sender: EnttecOpenDmxSender,
) -> EngineRuntime {
    // Keep the production configuration exact, but use the no-wire test
    // sender so focused engine tests never send Art-Net to a real local
    // Unity listener while an operator has a physical rig connected.
    let mut runtime = EngineRuntime::new_with_shared_telemetry(
        DmxOutputConfig {
            enabled: true,
            ..staged_show_artnet_loopback_output()
        },
        shared_telemetry,
    );
    runtime.dmx_sender = Some(DmxSender::TestExactArtNetRoute);
    runtime.show_serial_dmx_route = Some(RuntimeShowSerialDmxRoute {
        sender: Some(DmxSender::EnttecOpenDmx(sender)),
    });
    runtime.apply_command(EngineCommand::PatchFixture {
        fixture_id: 1,
        request: PatchFixtureRequest {
            profile_path: "memory://show-serial-dmx-mirror.gdtf".to_string(),
            mode_name: Some("Standard".to_string()),
            label: "Show serial DMX mirror fixture".to_string(),
            universe: SHOW_SERIAL_DMX_UNIVERSE,
            address: 1,
            group_ids: Vec::new(),
            position: Vec3::default(),
            rotation: Default::default(),
        },
        profile: sample_profile(),
    });
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".to_string(),
        value: u16::MAX,
    });
    runtime
}

fn exact_show_serial_dmx_test_identity() -> io::serial_dmx::VerifiedUsbSerialPortIdentity {
    io::serial_dmx::VerifiedUsbSerialPortIdentity {
        port_name: "COM3".to_string(),
        port_type: "USB 0403:6001 USB Serial Port".to_string(),
        usb_vid: 0x0403,
        usb_pid: 0x6001,
        serial_number: "SHOW-A".to_string(),
        manufacturer: "FTDI".to_string(),
        product: "USB Serial Port".to_string(),
        windows_device_instance_id: Some(r"FTDIBUS\A\0000".to_string()),
    }
}

fn enable_exact_unity_artnet_mirror_for_serial_test(runtime: &mut EngineRuntime) {
    runtime.output = DmxOutputConfig {
        enabled: true,
        ..staged_show_artnet_loopback_output()
    };
    runtime.dmx_sender = Some(DmxSender::TestExactArtNetRoute);
}

#[test]
fn show_serial_dmx_activation_rejects_a_staged_or_missing_unity_artnet_mirror_before_opening_usb() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = EngineRuntime::new_with_shared_telemetry(
        staged_show_artnet_loopback_output(),
        Arc::clone(&shared),
    );
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm before the negative activation proof");
    runtime.safety_blackout_engaged = true;
    let expected_identity = exact_show_serial_dmx_test_identity();
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |_identity, _safety_gate| {
                panic!(
                    "USB sender construction must be unreachable without the enabled Unity mirror"
                )
            },
        )
        .expect_err("a staged Art-Net route must not admit stand-alone USB output");
    assert!(error.contains("exact enabled Art-Net"));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(!shared.show_serial_dmx_route_status().active);
}

fn open_dmx_write_payloads(
    observation: &io::serial_dmx::OpenDmxTestSerialObservation,
    minimum: usize,
) -> Vec<Vec<u8>> {
    wait_for_open_dmx_test_write_count(observation, minimum)
        .into_iter()
        .filter_map(|operation| match operation {
            io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload) => Some(payload),
            _ => None,
        })
        .collect()
}

fn first_open_dmx_write_blocks_until_released() -> (
    io::serial_dmx::OpenDmxTestSerialPort,
    mpsc::Receiver<()>,
    mpsc::SyncSender<()>,
) {
    let (physical_entered_tx, physical_entered_rx) = mpsc::sync_channel(1);
    let (physical_release_tx, physical_release_rx) = mpsc::sync_channel(1);
    let physical_release_rx = Arc::new(Mutex::new(physical_release_rx));
    let first_write = Arc::new(AtomicBool::new(true));
    let (port, _observation) = io::serial_dmx::OpenDmxTestSerialPort::new_with_before_write_all({
        let physical_release_rx = Arc::clone(&physical_release_rx);
        let first_write = Arc::clone(&first_write);
        move || {
            if first_write.swap(false, Ordering::AcqRel) {
                physical_entered_tx
                    .send(())
                    .expect("the test must observe the wedged serial write");
                physical_release_rx
                    .lock()
                    .expect("the test release channel must not be poisoned")
                    .recv_timeout(Duration::from_secs(5))
                    .expect("the test must eventually release the wedged serial write");
            }
        }
    });
    (port, physical_entered_rx, physical_release_tx)
}

#[test]
fn show_serial_dmx_mirror_is_s0_first_live_after_release_and_zero_after_reengage() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let mut sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm the shared physical gate before activation");
    sender
        .send_dmx_frame(&[0u8; 512])
        .expect("S0-first zero must queue before the runtime worker is retained");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.safety_blackout_engaged = true;
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);
    let s0_first = open_dmx_write_payloads(&observation, 1);
    assert!(s0_first[0][1..].iter().all(|value| *value == 0));

    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .expect("the exact S0 authority may release");
    runtime.safety_blackout_engaged = released.engaged;
    runtime.tick(2, &snapshot);
    let live = open_dmx_write_payloads(&observation, 2);
    assert!(
        live[1][1..].iter().any(|value| *value != 0),
        "the same retained worker must receive the completed live U0 mirror after Release Blackout"
    );
    let status = shared.show_serial_dmx_route_status();
    assert!(status.active);
    assert!(status.live_frame_queued);
    assert!(!status.faulted);

    let (_, reengaged) = shared
        .engage_safety_blackout()
        .expect("re-engaged S0 must reserve the physical gate before the next mirror enqueue");
    runtime.safety_blackout_engaged = reengaged.engaged;
    runtime.tick(3, &snapshot);
    let reengaged_writes = open_dmx_write_payloads(&observation, 3);
    assert!(
        reengaged_writes.last().unwrap()[1..]
            .iter()
            .all(|value| *value == 0),
        "every later worker transaction must be zero after S0 re-engages"
    );
    runtime.show_serial_dmx_route = None;
}

#[test]
fn show_serial_dmx_live_enqueue_matches_the_same_completed_u0_buffer_as_artnet() {
    let receiver =
        UdpSocket::bind("127.0.0.1:0").expect("the test-only Unity Art-Net receiver must bind");
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("the test receiver must accept a bounded read timeout");
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.output = DmxOutputConfig {
        enabled: true,
        ..staged_show_artnet_loopback_output()
    };
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new("127.0.0.1", receiver.local_addr().unwrap().port())
            .expect("the exact-config test sender must be constructible"),
    ));
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm before the two-route mirror test");
    runtime.safety_blackout_engaged = true;
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(0, &snapshot);
    let mut ignored = [0u8; 600];
    let _ = receiver
        .recv_from(&mut ignored)
        .expect("the initial Art-Net S0 frame must arrive at the test receiver");
    let _ = open_dmx_write_payloads(&observation, 1);

    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .expect("the healthy two-route worker may explicitly release S0");
    runtime.safety_blackout_engaged = released.engaged;
    runtime.tick(1, &snapshot);

    let mut packet = [0u8; 600];
    let (received, _) = receiver
        .recv_from(&mut packet)
        .expect("the same tick must enqueue the Unity Art-Net mirror");
    let artnet = parse_art_dmx_packet(&packet[..received])
        .expect("the Unity mirror payload must remain a valid ArtDmx packet");
    assert!(
        artnet.data.iter().any(|value| *value != 0),
        "the post-release Art-Net frame must be the completed live U0 buffer"
    );
    let usb_payload = open_dmx_write_payloads(&observation, 2)
        .into_iter()
        .rev()
        .find(|payload| payload[1..].iter().any(|value| *value != 0))
        .expect("the same tick's queued Open DMX mirror must eventually write its live U0 payload");
    assert_eq!(usb_payload[0], io::serial_dmx::ENTTEC_OPEN_DMX_START_CODE);
    assert_eq!(
        &usb_payload[1..],
        artnet.data,
        "Art-Net and USB-DMX must receive byte-identical completed U0 buffers from one engine tick"
    );
    assert_eq!(artnet.data[SHOW_ARTNET_LOOPBACK_UNUSED_CHANNEL_INDEX], 0);
    assert_eq!(
        usb_payload[SHOW_ARTNET_LOOPBACK_UNUSED_CHANNEL_INDEX + 1],
        0
    );
    runtime.show_serial_dmx_route = None;
    runtime.dmx_sender = None;
}

#[test]
fn show_serial_dmx_activation_retains_the_worker_only_after_its_s0_zero_receipt() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let status_was_inactive_before_write = Arc::new(AtomicBool::new(false));
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new_with_before_write_all({
        let shared = Arc::clone(&shared);
        let status_was_inactive_before_write = Arc::clone(&status_was_inactive_before_write);
        move || {
            status_was_inactive_before_write.store(
                !shared.show_serial_dmx_route_status().active,
                Ordering::Release,
            );
        }
    });
    let mut runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), Arc::clone(&shared));
    enable_exact_unity_artnet_mirror_for_serial_test(&mut runtime);
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm before the Open DMX worker can be constructed");
    runtime.safety_blackout_engaged = true;
    let expected_identity = exact_show_serial_dmx_test_identity();

    runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |identity, safety_gate| {
                assert_eq!(identity, &expected_identity);
                EnttecOpenDmxSender::from_test_serial_port(port, safety_gate.clone(), || {})
                    .map(DmxSender::EnttecOpenDmx)
                    .map_err(|error| error.to_string())
            },
        )
        .expect("the route cannot be retained until its initial physical S0 zero receipt succeeds");

    assert!(
        status_was_inactive_before_write.load(Ordering::Acquire),
        "the worker must not become active before the physical zero transaction reaches write_all"
    );
    let writes = open_dmx_write_payloads(&observation, 1);
    assert!(writes[0][1..].iter().all(|value| *value == 0));
    let status = shared.show_serial_dmx_route_status();
    assert!(status.active);
    assert!(status.zero_frame_queued);
    assert!(status.zero_frame_physical_write_completed);
    assert!(!status.live_frame_queued);
    assert!(!status.faulted);
    runtime.show_serial_dmx_route = None;
}

#[test]
fn show_serial_dmx_activation_zero_write_fault_never_retains_a_live_worker() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    observation
        .fail_next_write()
        .expect("the fake serial port must inject the initial zero write fault");
    let mut runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), Arc::clone(&shared));
    enable_exact_unity_artnet_mirror_for_serial_test(&mut runtime);
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm before the worker creation attempt");
    runtime.safety_blackout_engaged = true;
    let expected_identity = exact_show_serial_dmx_test_identity();

    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |_identity, safety_gate| {
                EnttecOpenDmxSender::from_test_serial_port(port, safety_gate.clone(), || {})
                    .map(DmxSender::EnttecOpenDmx)
                    .map_err(|error| error.to_string())
            },
        )
        .expect_err("an initial physical zero write fault must fail closed");
    assert!(error.contains("initial physical S0 zero write"));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(runtime.safety_blackout_engaged);
    assert!(shared.safety_blackout_authority().engaged);
    let status = shared.show_serial_dmx_route_status();
    assert!(!status.active);
    assert!(status.zero_frame_queued);
    assert!(!status.zero_frame_physical_write_completed);
    assert!(status.faulted);
}

#[test]
fn show_serial_dmx_activation_receipt_failure_is_bounded_and_sticky_visible() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    observation
        .fail_next_write()
        .expect("the fake serial port must inject an already-completed worker failure");
    let mut failed_sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    failed_sender
        .send_dmx_frame(&[0u8; 512])
        .expect("the pre-activation fake frame must enter the worker queue");
    let deadline = Instant::now() + Duration::from_secs(1);
    while failed_sender.zero_write_receipt().is_ok() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(1));
    }
    assert!(
        failed_sender.zero_write_receipt().is_err(),
        "the test must reach the activation-time receipt failure before ownership transfers"
    );

    let mut runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), Arc::clone(&shared));
    enable_exact_unity_artnet_mirror_for_serial_test(&mut runtime);
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must arm before activation consumes the failed worker");
    runtime.safety_blackout_engaged = true;
    let expected_identity = exact_show_serial_dmx_test_identity();

    let started = Instant::now();
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |_identity, _safety_gate| Ok(DmxSender::EnttecOpenDmx(failed_sender)),
        )
        .expect_err("an immediate receipt failure must not leave a default stopped status");
    assert!(started.elapsed() < Duration::from_secs(1));
    assert!(error.contains("could not prepare its initial S0 blackout receipt"));
    assert!(error.contains("bounded shutdown fault=none"));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(runtime.safety_blackout_engaged);
    assert!(shared.safety_blackout_authority().engaged);
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("the receipt failure must leave the atomic S0 latch engaged"));
    let status = shared.show_serial_dmx_route_status();
    assert!(!status.active);
    assert!(!status.zero_frame_queued);
    assert!(!status.zero_frame_physical_write_completed);
    assert!(status.faulted);
    assert!(status.detail.contains("bounded shutdown fault=none"));
}

#[test]
fn show_serial_dmx_stop_waits_for_a_physical_s0_zero_before_worker_shutdown() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must be held before bounded USB-DMX shutdown");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.safety_blackout_engaged = true;

    runtime
        .apply_show_serial_dmx_safety_blackout_route_stop(armed.epoch, armed.generation)
        .expect("the worker must stop only after its physical zero receipt completes");

    let writes = open_dmx_write_payloads(&observation, 1);
    assert!(
        writes.last().unwrap()[1..].iter().all(|value| *value == 0),
        "the final observed worker payload must be physical S0 zero, not merely a queue replacement"
    );
    let status = shared.show_serial_dmx_route_status();
    assert!(!status.active);
    assert!(status.zero_frame_queued);
    assert!(status.zero_frame_physical_write_completed);
    assert!(!status.faulted);
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn show_serial_dmx_worker_write_fault_engages_s0_and_is_visible() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let mut sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let (_, armed) = shared.engage_safety_blackout().unwrap();
    sender.send_dmx_frame(&[0u8; 512]).unwrap();
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.safety_blackout_engaged = true;
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);
    let _ = open_dmx_write_payloads(&observation, 1);
    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    observation
        .fail_next_write()
        .expect("the fake serial port must accept an injected failure");
    runtime.tick(2, &snapshot);
    let deadline = Instant::now() + Duration::from_secs(1);
    while Instant::now() < deadline {
        runtime.tick(3, &snapshot);
        if shared.show_serial_dmx_route_status().faulted {
            break;
        }
        thread::sleep(Duration::from_millis(1));
    }
    let status = shared.show_serial_dmx_route_status();
    assert!(
        status.faulted,
        "a worker write failure must remain observable"
    );
    assert!(!status.active);
    assert!(status.artnet_mirror_live);
    assert!(status
        .artnet_mirror_detail
        .contains("route and sender are present"));
    assert!(shared.safety_blackout_authority().engaged);
    assert!(runtime.safety_blackout_engaged);
    assert!(runtime.show_serial_dmx_route.is_none());
}

fn wedged_live_show_serial_dmx_runtime(
    shared: Arc<EngineSharedTelemetry>,
) -> (EngineRuntime, SafetyBlackoutAuthority, mpsc::SyncSender<()>) {
    let (port, physical_entered, physical_release) = first_open_dmx_write_blocks_until_released();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);
    physical_entered
        .recv_timeout(Duration::from_secs(1))
        .expect("the test must wedge the worker inside its physical write gate");
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must latch while the previous live write owns the physical gate");
    runtime.safety_blackout_engaged = true;
    (runtime, armed, physical_release)
}

#[test]
fn show_serial_dmx_stop_wedged_gate_stays_s0_faulted_and_refuses_a_replacement_worker() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (mut runtime, armed, physical_release) =
        wedged_live_show_serial_dmx_runtime(Arc::clone(&shared));

    let started = Instant::now();
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_stop(armed.epoch, armed.generation)
        .expect_err("a wedged physical gate must not make USB-DMX stop unbounded");
    assert!(started.elapsed() < Duration::from_secs(2));
    physical_release
        .send(())
        .expect("the test must release the detached serial worker");
    assert!(error.contains("No physical zero transaction was queued or confirmed"));
    let status = shared.show_serial_dmx_route_status();
    assert!(!status.active);
    assert!(!status.zero_frame_queued);
    assert!(!status.zero_frame_physical_write_completed);
    assert!(status.faulted);
    assert!(status.detail.contains("fixture may retain its last look"));
    assert!(status
        .detail
        .contains("USB electrical safety is unverified"));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(shared.safety_blackout_authority().engaged);
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("the gate latch must remain S0 after an unconfirmed stop"));
    assert!(shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .expect_err("a physical USB fault must keep S0 latched until re-arm")
        .contains("fault remains latched"));

    // Releasing the fake driver's blocked call does not manufacture a joined
    // worker receipt for the engine. The old JoinHandle was deliberately
    // detached to keep Stop bounded, so creating a second USB worker in this
    // process would race a late old-A write. The I/O gate has its own
    // reservation-release proof; route activation must remain fail-closed.
    let replacement_factory_called = Arc::new(AtomicBool::new(false));
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &exact_show_serial_dmx_test_identity(),
            armed.epoch,
            armed.generation,
            {
                let replacement_factory_called = Arc::clone(&replacement_factory_called);
                move |_identity, _safety_gate| {
                    replacement_factory_called.store(true, Ordering::Release);
                    panic!("a detached worker must reject before a replacement sender can open")
                }
            },
        )
        .expect_err("a detached worker must prohibit a same-process replacement sender");
    assert!(error.contains("did not complete bounded shutdown"));
    assert!(!replacement_factory_called.load(Ordering::Acquire));
    let status = shared.show_serial_dmx_route_status();
    assert!(status.faulted);
    assert!(!status.worker_shutdown_completed);
}

#[test]
fn show_serial_dmx_joined_fault_recovery_requires_s0_then_clears_only_after_a_fresh_zero() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), Arc::clone(&shared));
    enable_exact_unity_artnet_mirror_for_serial_test(&mut runtime);
    // Model a disconnected A worker which did join, but whose old physical
    // zero receipt was unavailable. Explicit B binding occurs outside this
    // engine test; this is the fresh B activation boundary it must reach.
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus {
        active: false,
        zero_frame_queued: true,
        zero_frame_physical_write_completed: false,
        live_frame_queued: false,
        worker_shutdown_completed: true,
        faulted: true,
        artnet_mirror_live: true,
        artnet_mirror_detail: "test exact Art-Net mirror".to_string(),
        detail: "test joined fault without an old-A zero receipt".to_string(),
    });
    let factory_called_without_s0 = Arc::new(AtomicBool::new(false));
    let expected_identity = exact_show_serial_dmx_test_identity();
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            1,
            1,
            {
                let factory_called_without_s0 = Arc::clone(&factory_called_without_s0);
                move |_identity, _safety_gate| {
                    factory_called_without_s0.store(true, Ordering::Release);
                    panic!("faulted recovery must prove both S0 latches before opening B")
                }
            },
        )
        .expect_err("joined fault recovery without S0 must fail closed");
    assert!(error.contains("logical and physical S0 latches"));
    assert!(!factory_called_without_s0.load(Ordering::Acquire));

    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("the explicit recovery path must engage logical and physical S0");
    runtime.safety_blackout_engaged = true;
    let (failing_port, failing_observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    failing_observation
        .fail_next_write()
        .expect("the joined-recovery test must inject B's initial S0 failure");
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |_identity, safety_gate| {
                EnttecOpenDmxSender::from_test_serial_port(failing_port, safety_gate.clone(), || {})
                    .map(DmxSender::EnttecOpenDmx)
                    .map_err(|error| error.to_string())
            },
        )
        .expect_err("a fresh B initial S0 failure must not clear the prior fault");
    assert!(error.contains("initial physical S0 zero write"));
    let failed_b_status = shared.show_serial_dmx_route_status();
    assert!(failed_b_status.faulted);
    assert!(!failed_b_status.active);
    assert!(
        shared.safety_blackout_authority().engaged,
        "a failed B initial zero must preserve the logical S0 latch"
    );
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("a failed B initial zero must preserve the physical S0 latch"));

    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &expected_identity,
            armed.epoch,
            armed.generation,
            |_identity, safety_gate| {
                EnttecOpenDmxSender::from_test_serial_port(port, safety_gate.clone(), || {})
                    .map(DmxSender::EnttecOpenDmx)
                    .map_err(|error| error.to_string())
            },
        )
        .expect("only the fresh B initial physical S0 zero may clear the joined fault");
    let writes = open_dmx_write_payloads(&observation, 1);
    assert!(writes[0][1..].iter().all(|value| *value == 0));
    let status = shared.show_serial_dmx_route_status();
    assert!(status.active);
    assert!(status.zero_frame_physical_write_completed);
    assert!(!status.faulted);
    runtime.show_serial_dmx_route = None;
}

#[test]
fn show_serial_dmx_fault_latches_s0_before_a_contended_status_publish() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let runtime =
        EngineRuntime::new_with_shared_telemetry(disabled_open_dmx_output(), Arc::clone(&shared));
    // Hold the first status publication point. A correct fault entry must
    // still flip the lock-free worker direction before this mutex can release.
    let status_publish_lock = shared.show_serial_dmx_route_status.test_hold_current();
    let (entered_tx, entered_rx) = mpsc::sync_channel(1);
    let fault_thread = thread::spawn(move || {
        entered_tx
            .send(())
            .expect("the test must observe the fault thread entry");
        let mut runtime = runtime;
        runtime.fault_show_serial_dmx_route("deterministic contended-status fault");
    });
    entered_rx
        .recv_timeout(Duration::from_secs(1))
        .expect("the fault thread must begin");
    let deadline = Instant::now() + Duration::from_secs(1);
    while !shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("the atomic S0 latch must be queryable")
    {
        assert!(
            Instant::now() < deadline,
            "fault entry must latch S0 without waiting for the status mutex"
        );
        thread::yield_now();
    }
    drop(status_publish_lock);
    fault_thread
        .join()
        .expect("the fault thread must complete after status publication unblocks");
    assert!(shared.show_serial_dmx_route_status().faulted);
}

#[test]
fn show_serial_dmx_status_revision_is_monotonic_and_skips_identical_ticks() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_for_callback = Arc::clone(&observed);
    shared
        .show_serial_dmx_route_status
        .set_observer(Some(Arc::new(move |snapshot| {
            observed_for_callback
                .lock()
                .expect("status observer test collection must be available")
                .push(snapshot);
        })))
        .expect("status observer must install");

    let initial = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("initial status must be queryable");
    let mut active = initial.status.clone();
    active.active = true;
    active.live_frame_queued = true;
    active.detail = "deterministic active worker".to_string();
    shared.set_show_serial_dmx_route_status(active.clone());
    let active_snapshot = shared
        .try_show_serial_dmx_route_status_snapshot()
        .expect("active status must be queryable");
    assert!(active_snapshot.revision > initial.revision);
    assert_eq!(active_snapshot.status, active);

    // The normal engine tick republishes this same state frequently. It must
    // not create a false route event or invalidate an otherwise current IPC
    // response merely because another 44Hz frame completed.
    shared.set_show_serial_dmx_route_status(active);
    let observed = observed
        .lock()
        .expect("status observer test collection must be readable")
        .clone();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0], active_snapshot);
}

#[test]
fn show_serial_dmx_status_lock_poison_rejects_the_ipc_snapshot_and_never_returns_active() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let shared_for_poison = Arc::clone(&shared);
    let _ = std::panic::catch_unwind(move || {
        let _guard = shared_for_poison
            .show_serial_dmx_route_status
            .test_hold_current();
        panic!("deterministically poison USB-DMX status lock");
    });
    assert!(
        shared.try_show_serial_dmx_route_status_snapshot().is_err(),
        "a poisoned route status mutex must reject the IPC/query boundary"
    );
    let fail_closed = shared.show_serial_dmx_route_status();
    assert!(fail_closed.faulted);
    assert!(!fail_closed.active);
    assert!(!fail_closed.live_frame_queued);
}

#[test]
fn show_serial_dmx_status_revision_exhaustion_emits_one_terminal_fault_and_never_reuses_active() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let observed = Arc::new(Mutex::new(Vec::new()));
    let observed_for_callback = Arc::clone(&observed);
    shared
        .show_serial_dmx_route_status
        .set_observer(Some(Arc::new(move |snapshot| {
            observed_for_callback
                .lock()
                .expect("status observer test collection must be available")
                .push(snapshot);
        })))
        .expect("status observer must install");
    shared
        .show_serial_dmx_route_status
        .test_set_current_revision(u64::MAX);

    let mut attempted_active = ShowSerialDmxRouteStatus::default();
    attempted_active.active = true;
    attempted_active.live_frame_queued = true;
    attempted_active.worker_shutdown_completed = false;
    attempted_active.detail = "must never reuse u64::MAX as active".to_string();
    shared.set_show_serial_dmx_route_status(attempted_active);

    assert!(
        shared.try_show_serial_dmx_route_status_snapshot().is_err(),
        "revision exhaustion must reject the IPC/query boundary rather than return an Active max revision"
    );
    let fail_closed = shared.show_serial_dmx_route_status();
    assert!(fail_closed.faulted);
    assert!(!fail_closed.active);
    assert!(!fail_closed.live_frame_queued);
    assert!(fail_closed.detail.contains("revision was exhausted"));

    // The observer gets exactly the terminal u64::MAX event once. Later
    // writes cannot manufacture a wrapped/same-generation recovery event.
    shared.set_show_serial_dmx_route_status(ShowSerialDmxRouteStatus::default());
    let observed = observed
        .lock()
        .expect("status observer test collection must be readable")
        .clone();
    assert_eq!(observed.len(), 1);
    assert_eq!(observed[0].revision, u64::MAX);
    assert!(observed[0].status.faulted);
    assert!(!observed[0].status.active);
}

#[test]
fn show_serial_dmx_fault_wedged_gate_fences_stale_release_and_detaches_bounded() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (mut runtime, stale_authority, physical_release) =
        wedged_live_show_serial_dmx_runtime(Arc::clone(&shared));

    let started = Instant::now();
    runtime.fault_show_serial_dmx_route("deterministic wedged-driver fault");
    assert!(started.elapsed() < Duration::from_secs(2));
    physical_release
        .send(())
        .expect("the test must release the detached serial worker");

    let status = shared.show_serial_dmx_route_status();
    assert!(status.faulted);
    assert!(!status.active);
    assert!(
        !status.worker_shutdown_completed,
        "a detached wedged worker must remain ineligible for same-process binding recovery"
    );
    assert!(!status.zero_frame_physical_write_completed);
    assert!(status.detail.contains("physical gate is unconfirmed"));
    assert!(status.detail.contains("fixture may retain its last look"));
    assert!(status
        .detail
        .contains("USB electrical safety is unverified"));
    assert!(runtime.show_serial_dmx_route.is_none());
    let fault_authority = shared.safety_blackout_authority();
    assert!(fault_authority.engaged);
    assert_ne!(
        fault_authority, stale_authority,
        "fault must mint a new S0 authority that fences a pre-fault release"
    );
    assert!(shared
        .release_safety_blackout(stale_authority.epoch, stale_authority.generation)
        .expect_err("a pre-fault Release Blackout authority must be rejected")
        .contains("fault remains latched"));
    assert!(shared
        .release_safety_blackout(fault_authority.epoch, fault_authority.generation)
        .expect_err("the current authority must also refuse Release while USB fault is sticky")
        .contains("fault remains latched"));
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("fault must preserve the atomic S0 selection latch"));
}

#[test]
fn show_serial_dmx_ownership_drop_uses_the_bounded_fault_path_not_sender_drop() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (mut runtime, _armed, physical_release) =
        wedged_live_show_serial_dmx_runtime(Arc::clone(&shared));

    let started = Instant::now();
    runtime.drop_all_dmx_senders();
    assert!(started.elapsed() < Duration::from_secs(2));
    physical_release
        .send(())
        .expect("the test must release the detached serial worker");

    let status = shared.show_serial_dmx_route_status();
    assert!(status.faulted);
    assert!(!status.active);
    assert!(status.detail.contains("output ownership transition"));
    assert!(status.detail.contains("physical zero completion=false"));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(runtime.dmx_sender.is_none());
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn show_serial_dmx_artnet_send_failure_faults_before_any_usb_live_enqueue_and_cannot_auto_rearm() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.output = DmxOutputConfig {
        enabled: true,
        ..staged_show_artnet_loopback_output()
    };
    // This test-only sender makes the main Art-Net send fail. Production
    // `send_output_frame_with_recovery` clears `dmx_sender` on that error;
    // the USB guard below must then fault before a live Open-DMX enqueue.
    runtime.dmx_sender = Some(DmxSender::TestFailingArtNetSend);
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);

    let writes = open_dmx_write_payloads(&observation, 1);
    assert!(writes
        .iter()
        .all(|payload| payload[1..].iter().all(|value| *value == 0)));
    assert!(runtime.dmx_sender.is_none());
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(runtime.safety_blackout_engaged);
    let status = shared.show_serial_dmx_route_status();
    assert!(status.faulted);
    assert!(!status.active);
    assert!(!status.live_frame_queued);
    assert!(status
        .detail
        .contains("Unity mirror was absent, changed, or failed"));
    assert!(!status.artnet_mirror_live);
    assert!(status
        .artnet_mirror_detail
        .contains("No Art-Net delivery continuation is claimed"));

    // An Art-Net retry after the fault may restore Art-Net itself, but it
    // must never reconstruct/arm the retired USB worker implicitly.
    let retry_receiver = UdpSocket::bind("127.0.0.1:0").expect("the retry test receiver must bind");
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new(
            "127.0.0.1",
            retry_receiver
                .local_addr()
                .expect("receiver address")
                .port(),
        )
        .expect("the test-only retry Art-Net sender must construct"),
    ));
    runtime.tick(2, &snapshot);
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(shared.show_serial_dmx_route_status().faulted);
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn show_serial_dmx_missing_or_mutated_artnet_mirror_is_not_a_live_usb_precondition() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, observation) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.output = DmxOutputConfig {
        enabled: true,
        ..staged_show_artnet_loopback_output()
    };
    runtime.dmx_sender = None;
    assert!(
        !runtime.has_exact_live_show_serial_dmx_artnet_mirror(),
        "an exact config without an initialized Art-Net sender is not a USB live mirror"
    );
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new("127.0.0.1", 9).expect("the test-only Art-Net sender must construct"),
    ));
    assert!(runtime.has_exact_live_show_serial_dmx_artnet_mirror());
    runtime.output.enabled = false;
    assert!(
        !runtime.has_exact_live_show_serial_dmx_artnet_mirror(),
        "a config mutation must revoke the USB live mirror precondition even with a sender"
    );
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);
    let writes = open_dmx_write_payloads(&observation, 1);
    assert!(writes
        .iter()
        .all(|payload| payload[1..].iter().all(|value| *value == 0)));
    assert!(runtime.show_serial_dmx_route.is_none());
    assert!(shared.show_serial_dmx_route_status().faulted);
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn show_serial_dmx_wedged_worker_never_blocks_artnet_or_engine_s0_tick() {
    let receiver =
        UdpSocket::bind("127.0.0.1:0").expect("the test-only Art-Net receiver must bind");
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("the test receiver must use a bounded read timeout");
    let shared = Arc::new(EngineSharedTelemetry::new());
    let (port, physical_entered, physical_release) = first_open_dmx_write_blocks_until_released();
    let sender = EnttecOpenDmxSender::from_test_serial_port(
        port,
        shared.open_dmx_safety_write_gate.clone(),
        || {},
    )
    .expect("the fake serial port must start the real Open DMX worker");
    let mut runtime = show_serial_dmx_mirror_runtime_with_test_sender(Arc::clone(&shared), sender);
    runtime.output = DmxOutputConfig {
        enabled: true,
        ..staged_show_artnet_loopback_output()
    };
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new(
            "127.0.0.1",
            receiver.local_addr().expect("receiver address").port(),
        )
        .expect("the test-only Art-Net sender must construct"),
    ));
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    runtime.tick(1, &snapshot);
    let mut packet = [0u8; 600];
    receiver
        .recv_from(&mut packet)
        .expect("the initial Art-Net live frame must arrive before the wedge");
    physical_entered
        .recv_timeout(Duration::from_secs(1))
        .expect("the worker must own its physical write gate before S0");
    let (_, armed) = shared
        .engage_safety_blackout()
        .expect("S0 must latch without waiting for the wedged USB gate");
    runtime.safety_blackout_engaged = true;

    let started = Instant::now();
    runtime.tick(2, &snapshot);
    assert!(
        started.elapsed() < Duration::from_millis(200),
        "the engine tick must not wait for the Open-DMX physical mutex"
    );
    let (received, _) = receiver
        .recv_from(&mut packet)
        .expect("Art-Net must continue to receive the S0 frame while USB is wedged");
    let artnet = parse_art_dmx_packet(&packet[..received])
        .expect("the concurrent Art-Net frame must remain valid");
    assert!(
        artnet.data.iter().all(|value| *value == 0),
        "S0 must preempt Art-Net to zero even while USB cannot complete a physical write"
    );
    assert!(shared.safety_blackout_authority().engaged);
    physical_release
        .send(())
        .expect("the test must release the wedged serial writer");
    runtime.fault_show_serial_dmx_route("test cleanup after concurrent Art-Net proof");
    let cleanup_status = shared.show_serial_dmx_route_status();
    assert!(cleanup_status.faulted);
    assert!(!cleanup_status.active);
    assert!(shared.safety_blackout_authority().engaged);
    assert!(shared
        .open_dmx_safety_write_gate
        .blackout_engaged()
        .expect("cleanup fault must retain the atomic S0 latch"));
    assert!(armed.engaged);
}
