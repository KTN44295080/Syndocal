//! Focused non-I/O proof for the USB-DMX machine-local identity boundary.

use super::show_serial_dmx_route::{
    is_exact_enabled_show_artnet_loopback_route,
    select_machine_binding_from_ports_with_route_status,
    validate_exact_show_serial_dmx_device_matches_with_capture, MachineBindingRouteAdmission,
};
use super::*;

fn selected_a() -> serial_dmx_machine::SerialDmxMachineBindingIdentityV1 {
    serial_dmx_machine::SerialDmxMachineBindingIdentityV1 {
        port_name: "COM3".to_string(),
        port_type: "USB 0403:6001 USB Serial Port".to_string(),
        usb_vid: 0x0403,
        usb_pid: 0x6001,
        serial_number: "SHOW-A".to_string(),
        manufacturer: "FTDI".to_string(),
        product: "USB Serial Port".to_string(),
        windows_device_instance_id: r"FTDIBUS\A\0000".to_string(),
    }
}

fn selected_b() -> serial_dmx_machine::SerialDmxMachineBindingIdentityV1 {
    serial_dmx_machine::SerialDmxMachineBindingIdentityV1 {
        port_name: "COM4".to_string(),
        port_type: "USB 0403:6001 USB Serial Port".to_string(),
        usb_vid: 0x0403,
        usb_pid: 0x6001,
        serial_number: "SHOW-B".to_string(),
        manufacturer: "FTDI".to_string(),
        product: "USB Serial Port".to_string(),
        windows_device_instance_id: r"FTDIBUS\B\0000".to_string(),
    }
}

fn select_request(
    selected: &serial_dmx_machine::SerialDmxMachineBindingIdentityV1,
) -> serial_dmx_machine::SelectSerialDmxMachineBindingRequestV1 {
    serial_dmx_machine::SelectSerialDmxMachineBindingRequestV1 {
        port_name: selected.port_name.clone(),
        windows_device_instance_id: selected.windows_device_instance_id.clone(),
    }
}

fn binding_path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syndocal-show-serial-dmx-route-{name}-{}",
        std::process::id()
    ))
}

fn route_admission(
    revision: u64,
    status: engine::ShowSerialDmxRouteStatus,
) -> MachineBindingRouteAdmission {
    MachineBindingRouteAdmission {
        snapshot: engine::ShowSerialDmxRouteStatusSnapshot { revision, status },
        safety_authority_engaged: false,
        open_dmx_safety_latched: false,
    }
}

fn fault_recovery_admission(
    revision: u64,
    status: engine::ShowSerialDmxRouteStatus,
) -> MachineBindingRouteAdmission {
    MachineBindingRouteAdmission {
        snapshot: engine::ShowSerialDmxRouteStatusSnapshot { revision, status },
        safety_authority_engaged: true,
        open_dmx_safety_latched: true,
    }
}

fn port_from_selected(
    selected: &serial_dmx_machine::SerialDmxMachineBindingIdentityV1,
) -> SerialPortSummary {
    SerialPortSummary {
        name: selected.port_name.clone(),
        port_type: selected.port_type.clone(),
        usb_vid: Some(selected.usb_vid),
        usb_pid: Some(selected.usb_pid),
        serial_number: Some(selected.serial_number.clone()),
        manufacturer: Some(selected.manufacturer.clone()),
        product: Some(selected.product.clone()),
        windows_device_instance_id: Some(selected.windows_device_instance_id.clone()),
        recommended_protocol: None,
    }
}

fn identity_from_selected(
    selected: &serial_dmx_machine::SerialDmxMachineBindingIdentityV1,
) -> io::serial_dmx::VerifiedUsbSerialPortIdentity {
    io::serial_dmx::VerifiedUsbSerialPortIdentity {
        port_name: selected.port_name.clone(),
        port_type: selected.port_type.clone(),
        usb_vid: selected.usb_vid,
        usb_pid: selected.usb_pid,
        serial_number: selected.serial_number.clone(),
        manufacturer: selected.manufacturer.clone(),
        product: selected.product.clone(),
        windows_device_instance_id: Some(selected.windows_device_instance_id.clone()),
    }
}

fn enabled_unity_artnet_route() -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: true,
        protocol: DmxOutputProtocol::ArtNet,
        target_ip: SHOW_ARTNET_LOOPBACK_TARGET_IP.to_string(),
        port: SHOW_ARTNET_LOOPBACK_PORT,
        universe: SHOW_ARTNET_LOOPBACK_UNIVERSE,
        serial_port: String::new(),
        serial_baud_rate: DmxOutputConfig::default().serial_baud_rate,
    }
}

#[test]
fn show_serial_dmx_requires_the_exact_enabled_unity_artnet_mirror_not_a_staged_route() {
    let enabled = enabled_unity_artnet_route();
    assert!(is_exact_enabled_show_artnet_loopback_route(&enabled));
    for invalid in [
        DmxOutputConfig {
            enabled: false,
            ..enabled.clone()
        },
        DmxOutputConfig {
            protocol: DmxOutputProtocol::Sacn,
            ..enabled.clone()
        },
        DmxOutputConfig {
            target_ip: "192.168.50.2".to_string(),
            ..enabled.clone()
        },
        DmxOutputConfig {
            port: 6455,
            ..enabled.clone()
        },
        DmxOutputConfig {
            universe: 1,
            ..enabled.clone()
        },
        DmxOutputConfig {
            serial_port: "COM3".to_string(),
            ..enabled
        },
    ] {
        assert!(
            !is_exact_enabled_show_artnet_loopback_route(&invalid),
            "USB-DMX must not arm without the exact enabled Unity mirror: {invalid:?}"
        );
    }
}

#[test]
fn show_serial_dmx_device_matcher_requires_the_exact_machine_local_selection() {
    let selected = selected_a();
    let port = port_from_selected(&selected);
    let identity = validate_exact_show_serial_dmx_device_matches_with_capture(
        &[port.clone()],
        &selected,
        |candidate| {
            assert_eq!(candidate, &port);
            Ok(identity_from_selected(&selected))
        },
    )
    .expect("selected identity must be capturable");
    assert_eq!(
        identity.windows_device_instance_id.as_deref(),
        Some(selected.windows_device_instance_id.as_str())
    );

    let mut other = selected.clone();
    other.windows_device_instance_id = r"FTDIBUS\B\0000".to_string();
    let mismatch = validate_exact_show_serial_dmx_device_matches_with_capture(
        &[port_from_selected(&other)],
        &selected,
        |_| unreachable!("a substituted interface must never be opened"),
    )
    .expect_err("different PnP instance must fail closed");
    assert!(mismatch.contains("absent or stale"));

    let mut renumbered = port.clone();
    renumbered.name = "COM9".to_string();
    assert!(validate_exact_show_serial_dmx_device_matches_with_capture(
        &[renumbered],
        &selected,
        |_| unreachable!("COM renumber requires reselection"),
    )
    .is_err());
    assert!(validate_exact_show_serial_dmx_device_matches_with_capture(
        &[port.clone(), port],
        &selected,
        |_| unreachable!("ambiguous selection must not open"),
    )
    .is_err());
}

#[test]
fn show_serial_dmx_identity_capture_cannot_promote_a_b_a_replacement() {
    let selected = selected_a();
    let port = port_from_selected(&selected);
    let mut replacement = identity_from_selected(&selected);
    replacement.product = "replacement adapter".to_string();
    let error =
        validate_exact_show_serial_dmx_device_matches_with_capture(&[port], &selected, |_| {
            Ok(replacement)
        })
        .expect_err("changed opened-handle identity must not be promoted after COM returns to A");
    assert!(error.contains("hardware identity changed after selection"));
}

#[test]
fn show_serial_dmx_binding_replacement_after_healthy_stop_requires_the_physical_zero_receipt() {
    let path = binding_path("healthy-stop-zero-receipt");
    let _ = std::fs::remove_file(&path);
    let a = selected_a();
    let b = selected_b();
    let ports = [port_from_selected(&a), port_from_selected(&b)];
    let stopped = engine::ShowSerialDmxRouteStatus::default();

    select_machine_binding_from_ports_with_route_status(&path, &select_request(&a), &ports, {
        let mut revisions = [1, 2, 3].into_iter();
        let initial_stopped = stopped.clone();
        move || {
            Ok(route_admission(
                revisions.next().unwrap(),
                initial_stopped.clone(),
            ))
        }
    })
    .expect("initial stopped A binding must be admitted");

    let mut stopped_after_physical_zero = stopped;
    stopped_after_physical_zero.zero_frame_queued = true;
    stopped_after_physical_zero.zero_frame_physical_write_completed = true;
    let (binding, route) =
        select_machine_binding_from_ports_with_route_status(&path, &select_request(&b), &ports, {
            let mut revisions = [4, 5, 6].into_iter();
            move || {
                Ok(route_admission(
                    revisions.next().unwrap(),
                    stopped_after_physical_zero.clone(),
                ))
            }
        })
        .expect(
            "A -> B may persist only after the stopped A worker has its physical S0 zero receipt",
        );
    assert_eq!(route.revision, 6);
    assert!(!route.status.faulted);
    assert!(route.status.worker_shutdown_completed);
    assert!(route.status.zero_frame_physical_write_completed);
    assert_eq!(binding.selected.as_ref(), Some(&b));
    assert_eq!(
        serial_dmx_machine::binding_status_from_path(&path, &ports)
            .selected
            .as_ref(),
        Some(&b),
        "the healthy receipt is the only admission path that may replace A with B",
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn show_serial_dmx_binding_replacement_requires_clean_stopped_receipt_and_rolls_back_post_persist_fault(
) {
    let path = binding_path("binding-mutation-admission");
    let _ = std::fs::remove_file(&path);
    let a = selected_a();
    let b = selected_b();
    let ports = [port_from_selected(&a), port_from_selected(&b)];
    let stopped = engine::ShowSerialDmxRouteStatus::default();

    let (_, initial_route) =
        select_machine_binding_from_ports_with_route_status(&path, &select_request(&a), &ports, {
            let mut revisions = [1, 2, 3].into_iter();
            let initial_stopped = stopped.clone();
            move || {
                Ok(route_admission(
                    revisions.next().unwrap(),
                    initial_stopped.clone(),
                ))
            }
        })
        .expect("an initial stopped binding must be admitted");
    assert_eq!(initial_route.revision, 3);
    let a_bytes = std::fs::read(&path).expect("A binding must be retained");

    let mut active = stopped.clone();
    active.active = true;
    active.live_frame_queued = true;
    let active_error = select_machine_binding_from_ports_with_route_status(
        &path,
        &select_request(&b),
        &ports,
        || Ok(route_admission(4, active.clone())),
    )
    .expect_err("A live worker must reject B before any binding write");
    assert!(active_error.contains("worker is active"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);
    assert_eq!(
        serial_dmx_machine::binding_status_from_path(&path, &ports)
            .selected
            .as_ref(),
        Some(&a),
        "A live -> B Confirm must not replace the persisted A identity"
    );

    let mut zero_incomplete = stopped.clone();
    zero_incomplete.zero_frame_queued = true;
    let zero_error = select_machine_binding_from_ports_with_route_status(
        &path,
        &select_request(&b),
        &ports,
        || Ok(route_admission(5, zero_incomplete.clone())),
    )
    .expect_err("an unconfirmed physical S0 zero transaction must reject B");
    assert!(zero_error.contains("physical zero transaction is incomplete"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);

    let query_error = select_machine_binding_from_ports_with_route_status(
        &path,
        &select_request(&b),
        &ports,
        || Err("deterministic route status query poison".to_string()),
    )
    .expect_err("a route status query failure must reject B before persistence");
    assert!(query_error.contains("query poison"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);

    let mut faulted_joined = stopped.clone();
    faulted_joined.faulted = true;
    let missing_latch_error = select_machine_binding_from_ports_with_route_status(
        &path,
        &select_request(&b),
        &ports,
        || Ok(route_admission(6, faulted_joined.clone())),
    )
    .expect_err("faulted A cannot be replaced when either S0 latch is not proven");
    assert!(missing_latch_error.contains("both logical and physical S0 latches"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);

    let missing_physical_latch_error = select_machine_binding_from_ports_with_route_status(
        &path,
        &select_request(&b),
        &ports,
        || {
            Ok(MachineBindingRouteAdmission {
                snapshot: engine::ShowSerialDmxRouteStatusSnapshot {
                    revision: 7,
                    status: faulted_joined.clone(),
                },
                safety_authority_engaged: true,
                open_dmx_safety_latched: false,
            })
        },
    )
    .expect_err("logical S0 without the physical latch cannot admit a faulted B replacement");
    assert!(missing_physical_latch_error.contains("both logical and physical S0 latches"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);

    let mut faulted_unjoined = stopped.clone();
    faulted_unjoined.faulted = true;
    faulted_unjoined.worker_shutdown_completed = false;
    let post_persist_error =
        select_machine_binding_from_ports_with_route_status(&path, &select_request(&b), &ports, {
            let stopped_before_post = stopped.clone();
            let mut answers = vec![
                route_admission(8, stopped_before_post.clone()),
                route_admission(9, stopped_before_post),
                fault_recovery_admission(10, faulted_unjoined),
            ]
            .into_iter();
            move || {
                Ok(answers
                    .next()
                    .expect("every admission read must be explicit"))
            }
        })
        .expect_err("a fault discovered after persistence must restore A");
    assert!(post_persist_error.contains("previous machine-local binding was restored"));
    assert_eq!(std::fs::read(&path).unwrap(), a_bytes);
    assert_eq!(
        serial_dmx_machine::binding_status_from_path(&path, &ports)
            .selected
            .as_ref(),
        Some(&a),
        "post-persist fault cannot leave B selected or replace a worker"
    );

    let mut stopped_after_receipt = stopped;
    stopped_after_receipt.faulted = true;
    // A joined disconnected A can retain the failed old zero receipt fields.
    // `worker_shutdown_completed` proves no transition is still in flight;
    // B selection sends nothing and its later fresh initial zero is the only
    // path that may clear this fault.
    stopped_after_receipt.zero_frame_queued = true;
    stopped_after_receipt.zero_frame_physical_write_completed = false;
    let (selected_b_status, final_route) =
        select_machine_binding_from_ports_with_route_status(&path, &select_request(&b), &ports, {
            let mut revisions = [11, 12, 13].into_iter();
            move || {
                Ok(fault_recovery_admission(
                    revisions.next().unwrap(),
                    stopped_after_receipt.clone(),
                ))
            }
        })
        .expect(
            "a faulted but joined worker may explicitly recover A to B only under both S0 latches",
        );
    assert_eq!(final_route.revision, 13);
    assert!(
        final_route.status.faulted,
        "selecting B is non-sending host-local recovery and must not clear the sticky fault or permit Release"
    );
    assert_eq!(selected_b_status.selected.as_ref(), Some(&b));
    assert_eq!(
        serial_dmx_machine::binding_status_from_path(&path, &ports)
            .selected
            .as_ref(),
        Some(&b),
        "B may persist only after A is joined, faulted S0 remains latched, and the old missing receipt is not mistaken for an in-flight transition"
    );
    let _ = std::fs::remove_file(path);
}
