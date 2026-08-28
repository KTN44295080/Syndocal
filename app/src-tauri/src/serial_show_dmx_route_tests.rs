//! Focused proof for the one local USB-DMX activation boundary.
//!
//! The project contains only the logical Open-DMX route; an exact physical
//! interface comes from a separate machine-local selection and is re-captured
//! immediately before the real Windows handle opens.

use super::*;

fn staged_route() -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: false,
        protocol: DmxOutputProtocol::EnttecOpenDmx,
        target_ip: String::new(),
        port: 0,
        universe: 0,
        serial_port: String::new(),
        serial_baud_rate: SHOW_SERIAL_DMX_BAUD_RATE,
    }
}

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

#[test]
fn show_serial_dmx_route_matcher_accepts_only_the_disabled_logical_authored_route() {
    let staged = staged_route();
    assert!(is_exact_staged_show_serial_dmx_route(&staged));
    for invalid in [
        DmxOutputConfig {
            enabled: true,
            ..staged.clone()
        },
        DmxOutputConfig {
            protocol: DmxOutputProtocol::ArtNet,
            ..staged.clone()
        },
        DmxOutputConfig {
            serial_port: "COM3".to_string(),
            ..staged.clone()
        },
        DmxOutputConfig {
            serial_baud_rate: 115_200,
            ..staged.clone()
        },
        DmxOutputConfig {
            universe: 1,
            ..staged
        },
    ] {
        assert!(
            !is_exact_staged_show_serial_dmx_route(&invalid),
            "ineligible authored route: {invalid:?}"
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
fn show_serial_dmx_machine_selection_does_not_mutate_the_project_route() {
    let route = staged_route();
    let before = route.clone();
    let selected = selected_a();
    let _ = validate_exact_show_serial_dmx_device_matches_with_capture(
        &[port_from_selected(&selected)],
        &selected,
        |_| Ok(identity_from_selected(&selected)),
    )
    .expect("captured machine selection is independent of authored route");
    assert_eq!(
        route, before,
        "machine binding must not write a COM alias into the project route"
    );
}
