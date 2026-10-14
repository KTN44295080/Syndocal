use super::*;

use std::{
    sync::{Arc, RwLock},
    thread,
    time::{Duration, Instant},
};

fn usb_identity(port: &str, serial: &str) -> io::serial_dmx::VerifiedUsbSerialPortIdentity {
    io::serial_dmx::VerifiedUsbSerialPortIdentity {
        port_name: port.to_string(),
        port_type: "USB 0403:6001 USB Serial Port".to_string(),
        usb_vid: 0x0403,
        usb_pid: 0x6001,
        serial_number: serial.to_string(),
        manufacturer: "FTDI".to_string(),
        product: "USB Serial Port".to_string(),
        macos_device_instance_id: None,
        windows_device_instance_id: Some(format!(r"FTDIBUS\{serial}\0000")),
    }
}

/// A runtime with no Art-Net route at all and one Dimmer fixture on each of
/// Universes 1 and 2 at different levels.
fn runtime_with_fixtures_on_u1_u2(shared: &Arc<EngineSharedTelemetry>) -> EngineRuntime {
    let mut runtime = EngineRuntime::new_with_shared_telemetry(
        DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        },
        Arc::clone(shared),
    );
    for (fixture_id, universe, value) in [(1, 1, u16::MAX), (2, 2, 0x8000)] {
        runtime.apply_command(EngineCommand::PatchFixture {
            fixture_id,
            request: PatchFixtureRequest {
                profile_path: format!("memory://usb-u{universe}.gdtf"),
                mode_name: Some("Standard".to_string()),
                label: format!("USB U{universe} fixture"),
                universe,
                address: 1,
                group_ids: Vec::new(),
                position: Vec3::default(),
                rotation: Default::default(),
            },
            profile: sample_profile(),
        });
        runtime.apply_command(EngineCommand::SetAttribute {
            fixture_id,
            attribute: "Dimmer".to_string(),
            value,
        });
    }
    assert_eq!(runtime.last_error, None);
    runtime
}

fn start_cable(
    runtime: &mut EngineRuntime,
    universe: u16,
    identity: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
    port: io::serial_dmx::OpenDmxTestSerialPort,
) -> Result<(), String> {
    let safety = runtime.shared_telemetry.safety_blackout_authority();
    runtime.apply_independent_usb_dmx_route_enable_with_sender_factory(
        universe,
        identity,
        safety.epoch,
        safety.generation,
        move |_identity, gate| {
            EnttecOpenDmxSender::from_test_serial_port(port, gate.clone(), || {})
                .map(DmxSender::EnttecOpenDmx)
                .map_err(|error| error.to_string())
        },
    )
}

fn payloads(observation: &io::serial_dmx::OpenDmxTestSerialObservation) -> Vec<Vec<u8>> {
    observation
        .operations()
        .expect("test serial observation")
        .into_iter()
        .filter_map(|operation| match operation {
            io::serial_dmx::OpenDmxTestSerialOperation::WriteAll(payload) => Some(payload),
            _ => None,
        })
        .collect()
}

/// Tick until the cable has written at least `count` frames; return the last.
fn last_payload_after(
    runtime: &mut EngineRuntime,
    snapshot: &Arc<RwLock<EngineSnapshot>>,
    observation: &io::serial_dmx::OpenDmxTestSerialObservation,
    count: usize,
) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut tick = 10;
    while Instant::now() < deadline {
        runtime.tick(tick, snapshot);
        tick += 1;
        let written = payloads(observation);
        if written.len() >= count {
            return written.last().cloned().unwrap();
        }
        thread::sleep(Duration::from_millis(2));
    }
    panic!("cable did not write {count} frames");
}

fn slot_status(shared: &EngineSharedTelemetry, universe: u16) -> ShowSerialDmxRouteStatus {
    shared
        .try_independent_usb_dmx_route_statuses()
        .unwrap()
        .into_iter()
        .find(|slot| slot.universe == Some(universe))
        .expect("the universe has a cable slot")
        .status
}

#[test]
fn independent_cables_start_zero_first_and_each_send_their_own_universe() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, armed) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let (port_b, cable_b) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    start_cable(&mut runtime, 2, &usb_identity("COM6", "CABLE-B"), port_b).unwrap();
    for cable in [&cable_a, &cable_b] {
        let first = payloads(cable);
        assert_eq!(
            first.len(),
            1,
            "exactly the initial zero before the cable is retained"
        );
        assert!(first[0][1..].iter().all(|value| *value == 0));
    }
    assert!(slot_status(&shared, 1).active && slot_status(&shared, 2).active);
    assert!(
        !runtime.output.enabled && runtime.dmx_sender.is_none(),
        "no Art-Net route is needed"
    );

    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    let a = last_payload_after(&mut runtime, &snapshot, &cable_a, 2);
    let b = last_payload_after(&mut runtime, &snapshot, &cable_b, 2);
    assert_eq!(a[1], 255, "cable A carries Universe 1");
    assert_eq!(b[1], 128, "cable B carries Universe 2");
    assert!(slot_status(&shared, 1).live_frame_queued);

    // A normal stop under S0 writes one confirmed zero and frees the slot.
    let (_, s0) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    runtime
        .apply_independent_usb_dmx_route_stop(2, s0.epoch, s0.generation)
        .unwrap();
    let stopped = slot_status(&shared, 2);
    assert!(!stopped.active && stopped.zero_frame_physical_write_completed && !stopped.faulted);
    assert!(payloads(&cable_b).last().unwrap()[1..]
        .iter()
        .all(|value| *value == 0));
    runtime
        .apply_independent_usb_dmx_route_stop(1, s0.epoch, s0.generation)
        .unwrap();
    assert!(runtime.independent_usb_dmx_routes_absent());
}

#[test]
fn a_cable_fault_engages_s0_for_every_cable_and_blocks_release_until_it_is_stopped() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, armed) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    let (port_b, cable_b) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    start_cable(&mut runtime, 2, &usb_identity("COM6", "CABLE-B"), port_b).unwrap();
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    assert_eq!(
        last_payload_after(&mut runtime, &snapshot, &cable_a, 2)[1],
        255
    );

    // Cable B's driver fails: B faults, S0 engages, cable A turns to zero.
    cable_b.fail_next_write().unwrap();
    let deadline = Instant::now() + Duration::from_secs(2);
    let mut tick = 100;
    while !slot_status(&shared, 2).faulted && Instant::now() < deadline {
        runtime.tick(tick, &snapshot);
        tick += 1;
        thread::sleep(Duration::from_millis(2));
    }
    assert!(slot_status(&shared, 2).faulted);
    let s0 = shared.safety_blackout_authority();
    assert!(s0.engaged && runtime.safety_blackout_engaged);
    let written_a = payloads(&cable_a).len();
    let a = last_payload_after(&mut runtime, &snapshot, &cable_a, written_a + 1);
    assert!(
        a[1..].iter().all(|value| *value == 0),
        "S0 zeros the healthy cable too"
    );
    assert!(
        slot_status(&shared, 1).active,
        "the healthy cable keeps running"
    );
    let release = shared.release_safety_blackout(s0.epoch, s0.generation);
    assert!(release.unwrap_err().contains("cable fault remains latched"));

    // The joined faulted cable is taken out of service under S0; then S0
    // may be released and cable A is live again.
    runtime
        .apply_independent_usb_dmx_route_stop(2, s0.epoch, s0.generation)
        .unwrap();
    assert!(!slot_status(&shared, 2).faulted);
    let (_, _, released) = shared
        .release_safety_blackout(s0.epoch, s0.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    let written_a = payloads(&cable_a).len();
    assert_eq!(
        last_payload_after(&mut runtime, &snapshot, &cable_a, written_a + 1)[1],
        255
    );
    let s0 = shared.engage_safety_blackout().unwrap().1;
    runtime
        .apply_independent_usb_dmx_route_stop(1, s0.epoch, s0.generation)
        .unwrap();
}

#[test]
fn independent_cables_reject_universe_zero_a_busy_universe_and_a_cable_already_in_use() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let cable_a = usb_identity("COM5", "CABLE-A");
    let never = |_: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
                 _: &OpenDmxSafetyWriteGate|
     -> Result<DmxSender, String> {
        panic!("no sender may open for a rejected activation")
    };
    let safety = shared.safety_blackout_authority();
    let zero = runtime
        .apply_independent_usb_dmx_route_enable_with_sender_factory(
            0,
            &cable_a,
            safety.epoch,
            safety.generation,
            never,
        )
        .unwrap_err();
    assert!(zero.contains("Universe 0 is the Unity-mirror cable"));

    let (port_a, _) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &cable_a, port_a).unwrap();
    let busy = runtime
        .apply_independent_usb_dmx_route_enable_with_sender_factory(
            1,
            &usb_identity("COM6", "CABLE-B"),
            safety.epoch,
            safety.generation,
            never,
        )
        .unwrap_err();
    assert!(busy.contains("already active"));
    let same_cable = runtime
        .apply_independent_usb_dmx_route_enable_with_sender_factory(
            2,
            &cable_a,
            safety.epoch,
            safety.generation,
            never,
        )
        .unwrap_err();
    assert!(same_cable.contains("already drives another universe"));
    runtime
        .apply_independent_usb_dmx_route_stop(1, safety.epoch, safety.generation)
        .unwrap();
}

#[test]
fn ownership_loss_faults_independent_cables_through_the_bounded_path() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    runtime.drop_all_dmx_senders();
    assert!(runtime.independent_usb_dmx_routes_absent());
    let status = slot_status(&shared, 1);
    assert!(status.faulted && status.worker_shutdown_completed && !status.active);
    assert!(
        status.zero_frame_physical_write_completed,
        "the fault path wrote one bounded zero"
    );
    assert!(payloads(&cable_a).last().unwrap()[1..]
        .iter()
        .all(|value| *value == 0));
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn configured_network_retirement_stops_independent_cables_with_a_physical_zero() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, safety) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    let _lease = runtime
        .output_ownership_gate
        .begin_failure_fence("independent cable retirement test");
    let epoch = runtime.output_ownership_gate.status().epoch;
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    runtime
        .apply_managed_dmx_retirement_for_scope(
            ManagedDmxRetirementScope::ConfiguredNetwork,
            safety.epoch,
            safety.generation,
            epoch,
            &operation,
        )
        .expect("configured retirement stops the cable with its own zero receipt");
    assert!(runtime.independent_usb_dmx_routes_absent());
    let status = slot_status(&shared, 1);
    assert!(!status.active && !status.faulted);
    assert!(status.zero_frame_physical_write_completed && status.worker_shutdown_completed);
    assert!(payloads(&cable_a).last().unwrap()[1..]
        .iter()
        .all(|value| *value == 0));
    assert!(shared.safety_blackout_authority().engaged);
}

#[test]
fn exact_show_fail_stop_refuses_independent_cables_and_the_cleanup_faults_them() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, safety) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, _cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    let _lease = runtime
        .output_ownership_gate
        .begin_failure_fence("exact show fail-stop with a cable");
    let epoch = runtime.output_ownership_gate.status().epoch;
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let result = runtime.apply_managed_dmx_retirement_for_scope(
        ManagedDmxRetirementScope::ExactShow,
        safety.epoch,
        safety.generation,
        epoch,
        &operation,
    );
    assert!(
        matches!(result, Err(ManagedShowDmxFailStopError::InDoubt(_))),
        "{result:?}"
    );
    runtime.best_effort_retire_managed_show_dmx_after_fail_stop_error("test cleanup");
    assert!(runtime.independent_usb_dmx_routes_absent());
    let status = slot_status(&shared, 1);
    assert!(status.faulted && status.worker_shutdown_completed);
}

#[test]
fn a_cable_taken_out_of_service_after_a_failed_first_zero_can_be_armed_again() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (broken_port, broken) = io::serial_dmx::OpenDmxTestSerialPort::new();
    broken.fail_next_write().unwrap();
    let error = start_cable(
        &mut runtime,
        1,
        &usb_identity("COM5", "CABLE-A"),
        broken_port,
    )
    .expect_err("a failed first zero never keeps the cable");
    assert!(error.contains("Universe 1"), "{error}");
    let faulted = slot_status(&shared, 1);
    assert!(faulted.faulted && !faulted.active);
    let s0 = shared.safety_blackout_authority();
    assert!(s0.engaged);

    runtime
        .apply_independent_usb_dmx_route_stop(1, s0.epoch, s0.generation)
        .expect("a joined faulted cable can be taken out of service under S0");
    let out_of_service = slot_status(&shared, 1);
    assert!(!out_of_service.faulted && !out_of_service.zero_frame_queued);

    let (port, cable) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM7", "CABLE-C"), port)
        .expect("the universe can be armed again from zero");
    assert!(slot_status(&shared, 1).active);
    assert!(payloads(&cable)[0][1..].iter().all(|value| *value == 0));
    let s0 = shared.safety_blackout_authority();
    runtime
        .apply_independent_usb_dmx_route_stop(1, s0.epoch, s0.generation)
        .unwrap();
}

#[test]
fn starting_a_second_cable_never_zeros_a_live_first_cable() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, armed) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    let snapshot = Arc::new(RwLock::new(runtime.build_snapshot(0)));
    let (_, _, released) = shared
        .release_safety_blackout(armed.epoch, armed.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    assert_eq!(
        last_payload_after(&mut runtime, &snapshot, &cable_a, 2)[1],
        255
    );

    let (port_b, cable_b) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 2, &usb_identity("COM6", "CABLE-B"), port_b).unwrap();
    assert!(
        payloads(&cable_b)[0][1..].iter().all(|value| *value == 0),
        "B starts at zero"
    );
    let written_a = payloads(&cable_a).len();
    assert_eq!(
        last_payload_after(&mut runtime, &snapshot, &cable_a, written_a + 1)[1],
        255,
        "A stays live through B's zero-first start"
    );
    assert!(payloads(&cable_a)
        .iter()
        .skip(1)
        .all(|frame| frame[1] == 255));
    let (_, s0) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    for universe in [1, 2] {
        runtime
            .apply_independent_usb_dmx_route_stop(universe, s0.epoch, s0.generation)
            .unwrap();
    }
}

#[test]
fn a_faulted_cable_is_not_absent_for_a_managed_retirement_proof() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let (port_a, _cable_a) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &usb_identity("COM5", "CABLE-A"), port_a).unwrap();
    runtime.fault_independent_usb_dmx_route(0, "test fault");
    assert!(runtime.independent_usb_dmx_routes_absent());
    assert!(!runtime.independent_usb_dmx_terminal_absent());
    let safety = shared.safety_blackout_authority();
    let _lease = runtime
        .output_ownership_gate
        .begin_failure_fence("faulted cable retirement test");
    let epoch = runtime.output_ownership_gate.status().epoch;
    let operation = ManagedShowDmxFailStopOperationInner::new();
    assert!(operation.admit());
    let result = runtime.apply_managed_dmx_retirement_for_scope(
        ManagedDmxRetirementScope::ConfiguredNetwork,
        safety.epoch,
        safety.generation,
        epoch,
        &operation,
    );
    assert!(
        matches!(result, Err(ManagedShowDmxFailStopError::InDoubt(_))),
        "a faulted cable keeps the retirement InDoubt: {result:?}"
    );
}

#[test]
fn one_cable_never_serves_universe_zero_and_another_universe() {
    let shared = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = runtime_with_fixtures_on_u1_u2(&shared);
    let (_, s0) = shared.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let never =
        |_: &io::serial_dmx::VerifiedUsbSerialPortIdentity,
         _: &OpenDmxSafetyWriteGate|
         -> Result<DmxSender, String> { panic!("no sender may open for a shared cable") };

    // The Universe 0 route runs on cable A: A cannot also serve Universe 1.
    let cable_a = usb_identity("COM5", "CABLE-A");
    runtime.show_serial_dmx_route = Some(RuntimeShowSerialDmxRoute { sender: None });
    runtime.show_serial_dmx_route_device = Some(cable_a.clone());
    let error = runtime
        .apply_independent_usb_dmx_route_enable_with_sender_factory(
            1,
            &cable_a,
            s0.epoch,
            s0.generation,
            never,
        )
        .unwrap_err();
    assert!(error.contains("already drives another universe"), "{error}");
    runtime.show_serial_dmx_route = None;

    // Cable B serves Universe 1: the Universe 0 route cannot start on B.
    let cable_b = usb_identity("COM6", "CABLE-B");
    let (port_b, _cable_b) = io::serial_dmx::OpenDmxTestSerialPort::new();
    start_cable(&mut runtime, 1, &cable_b, port_b).unwrap();
    let error = runtime
        .apply_show_serial_dmx_safety_blackout_route_enable_with_sender_factory(
            &cable_b,
            s0.epoch,
            s0.generation,
            never,
        )
        .unwrap_err();
    assert!(error.contains("already drives another universe"), "{error}");
    runtime
        .apply_independent_usb_dmx_route_stop(1, s0.epoch, s0.generation)
        .unwrap();
}
