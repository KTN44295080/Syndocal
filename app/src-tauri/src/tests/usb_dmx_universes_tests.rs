use super::*;

fn port(name: &str, instance: &str, serial: &str) -> SerialPortSummary {
    SerialPortSummary {
        name: name.to_string(),
        port_type: "USB 0403:6001 USB Serial Port".to_string(),
        usb_vid: Some(0x0403),
        usb_pid: Some(0x6001),
        serial_number: Some(serial.to_string()),
        manufacturer: Some("FTDI".to_string()),
        product: Some("USB Serial Port".to_string()),
        macos_device_instance_id: None,
        windows_device_instance_id: Some(instance.to_string()),
        recommended_protocol: None,
    }
}

fn binding(universe: u16, port: &SerialPortSummary) -> UsbDmxUniverseBindingV1 {
    UsbDmxUniverseBindingV1 {
        universe,
        identity: SerialDmxMachineBindingIdentityV2::from_summary(port).unwrap(),
    }
}

fn path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syndocal-usb-dmx-universes-{name}-{}",
        std::process::id()
    ))
}

#[test]
fn usb_dmx_universe_bindings_round_trip_and_report_presence_without_substitution() {
    let path = path("round-trip");
    let _ = std::fs::remove_file(&path);
    assert!(
        load_bindings(&path).unwrap().is_empty(),
        "a missing file is no binding"
    );
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    persist_bindings(&path, &[binding(2, &b), binding(1, &a)]).unwrap();
    let loaded = load_bindings(&path).unwrap();
    assert_eq!(
        loaded
            .iter()
            .map(|binding| binding.universe)
            .collect::<Vec<_>>(),
        vec![1, 2],
        "stored sorted by universe"
    );
    assert_eq!(
        binding_status(&loaded[0], &[a.clone(), b.clone()]).state,
        SerialDmxMachineBindingStateV1::SelectedAndPresent
    );
    let mut a_renumbered = a.clone();
    a_renumbered.name = "COM9".to_string();
    assert_eq!(
        binding_status(&loaded[0], &[a_renumbered, b.clone()]).state,
        SerialDmxMachineBindingStateV1::StaleOrMissing,
        "a renumbered cable is not substituted"
    );
    assert_eq!(
        binding_status(&loaded[0], &[a.clone(), a]).state,
        SerialDmxMachineBindingStateV1::Ambiguous
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn usb_dmx_universe_bindings_reject_universe_zero_duplicates_shared_cables_and_overflow() {
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    assert!(
        validate_bindings(&[binding(0, &a)]).is_err(),
        "Universe 0 is Show DMX"
    );
    assert!(validate_bindings(&[binding(32_768, &a)]).is_err());
    assert!(validate_bindings(&[binding(1, &a), binding(1, &b)]).is_err());
    assert!(
        validate_bindings(&[binding(1, &a), binding(2, &a)]).is_err(),
        "one cable outputs one universe"
    );
    // A clone with the same VID/PID/serial on another port is the same cable
    // identity for binding purposes (a port swap could otherwise swap universes).
    let clone = port("COM7", r"FTDIBUS\C\0000", "A-SERIAL");
    assert!(validate_bindings(&[binding(1, &a), binding(2, &clone)]).is_err());
    let cables = (1..=4)
        .map(|index| {
            port(
                &format!("COM{}", 10 + index),
                &format!(r"FTDIBUS\{index}\0000"),
                &format!("S{index}"),
            )
        })
        .collect::<Vec<_>>();
    let four = cables
        .iter()
        .enumerate()
        .map(|(index, cable)| binding(index as u16 + 1, cable))
        .collect::<Vec<_>>();
    assert!(validate_bindings(&four[..3]).is_ok());
    assert!(
        validate_bindings(&four).is_err(),
        "at most three cables besides Universe 0"
    );
}

#[test]
fn usb_dmx_universe_bindings_fail_closed_on_unknown_versions_fields_and_size() {
    let path = path("fail-closed");
    std::fs::write(&path, br#"{"version":2,"universes":[]}"#).unwrap();
    assert!(load_bindings(&path).unwrap_err().contains("version 2"));
    std::fs::write(&path, br#"{"version":1,"universes":[],"extra":true}"#).unwrap();
    assert!(load_bindings(&path).is_err());
    std::fs::write(&path, vec![b' '; 9 * 1024]).unwrap();
    assert!(load_bindings(&path).unwrap_err().contains("safety limit"));
    let _ = std::fs::remove_file(path);
}

fn u0_status(
    state: SerialDmxMachineBindingStateV1,
    selected: Option<&SerialPortSummary>,
) -> serial_dmx_machine::SerialDmxMachineBindingStatusV1 {
    serial_dmx_machine::SerialDmxMachineBindingStatusV1 {
        state,
        selected: selected
            .map(|port| SerialDmxMachineBindingIdentityV2::from_summary(port).unwrap()),
        detail: "test".to_string(),
    }
}

fn request(universe: u16, port: &SerialPortSummary) -> SelectUsbDmxUniverseBindingRequestV1 {
    SelectUsbDmxUniverseBindingRequestV1 {
        universe,
        port_name: port.name.clone(),
        windows_device_instance_id: port.windows_device_instance_id.clone(),
        macos_device_instance_id: None,
    }
}

#[test]
fn usb_dmx_universe_binding_keeps_cables_disjoint_from_universe_zero_and_each_other() {
    let path = path("disjoint");
    let _ = std::fs::remove_file(&path);
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    let ports = [a.clone(), b.clone()];
    let mut admitted = || Ok(());
    let unreadable = u0_status(SerialDmxMachineBindingStateV1::BlockedPersistence, None);
    let error =
        select_binding_from_ports(&path, &unreadable, &request(1, &a), &ports, &mut admitted)
            .unwrap_err();
    assert!(error.contains("cannot be read"), "{error}");
    let u0_is_a = u0_status(SerialDmxMachineBindingStateV1::SelectedAndPresent, Some(&a));
    let error = select_binding_from_ports(&path, &u0_is_a, &request(1, &a), &ports, &mut admitted)
        .unwrap_err();
    assert!(error.contains("Universe 0"), "{error}");
    select_binding_from_ports(&path, &u0_is_a, &request(1, &b), &ports, &mut admitted).unwrap();
    let error = select_binding_from_ports(&path, &u0_is_a, &request(2, &b), &ports, &mut admitted)
        .unwrap_err();
    assert!(error.contains("already bound to Universe 1"), "{error}");
    // Rebinding the same universe to the same cable is allowed.
    select_binding_from_ports(&path, &u0_is_a, &request(1, &b), &ports, &mut admitted).unwrap();
    assert_eq!(load_bindings(&path).unwrap().len(), 1);
    let _ = std::fs::remove_file(path);
}

#[test]
fn usb_dmx_universe_binding_restores_the_exact_file_when_admission_changes_after_the_write() {
    let path = path("restore");
    let _ = std::fs::remove_file(&path);
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    let ports = [a.clone(), b.clone()];
    let u0 = u0_status(SerialDmxMachineBindingStateV1::MissingSelection, None);
    select_binding_from_ports(&path, &u0, &request(1, &a), &ports, &mut || Ok(())).unwrap();
    let before = std::fs::read(&path).unwrap();
    let mut calls = 0;
    let error = select_binding_from_ports(&path, &u0, &request(2, &b), &ports, &mut || {
        calls += 1;
        if calls >= 3 {
            Err("the cable started meanwhile".to_string())
        } else {
            Ok(())
        }
    })
    .unwrap_err();
    assert_eq!(calls, 3, "checked twice before and once after the write");
    assert!(error.contains("previous bindings were restored"), "{error}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "the exact previous bytes"
    );
    let _ = std::fs::remove_file(path);
}
