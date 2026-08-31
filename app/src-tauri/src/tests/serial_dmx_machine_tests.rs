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
        windows_device_instance_id: Some(instance.to_string()),
        recommended_protocol: None,
    }
}

fn path(name: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "syndocal-serial-dmx-machine-{name}-{}",
        std::process::id()
    ))
}

#[test]
fn machine_binding_rejects_missing_selection_com_renumber_and_ambiguity_without_substitution() {
    let path = path("identity");
    let _ = std::fs::remove_file(&path);
    let a = port("COM3", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM4", r"FTDIBUS\B\0000", "B-SERIAL");
    assert_eq!(
        binding_status_from_path(&path, &[a.clone()]).state,
        SerialDmxMachineBindingStateV1::MissingSelection
    );
    let selected = select_binding_from_ports(
        &path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: a.name.clone(),
            windows_device_instance_id: a.windows_device_instance_id.clone().unwrap(),
        },
        &[a.clone()],
    )
    .unwrap();
    assert_eq!(
        selected.state,
        SerialDmxMachineBindingStateV1::SelectedAndPresent
    );
    assert_eq!(
        binding_status_from_path(&path, &[b.clone()]).state,
        SerialDmxMachineBindingStateV1::StaleOrMissing,
        "B must not inherit A authority"
    );
    let mut a_renumbered = a.clone();
    a_renumbered.name = "COM9".to_string();
    assert_eq!(
        binding_status_from_path(&path, &[a_renumbered]).state,
        SerialDmxMachineBindingStateV1::StaleOrMissing,
        "COM renumber requires explicit reselection"
    );
    assert_eq!(
        binding_status_from_path(&path, &[a.clone(), a.clone()]).state,
        SerialDmxMachineBindingStateV1::Ambiguous
    );
    assert_eq!(
        binding_status_from_path(&path, &[a]).state,
        SerialDmxMachineBindingStateV1::SelectedAndPresent,
        "A→B→A only restores the original exact selection"
    );
    let _ = std::fs::remove_file(path);
}

#[test]
fn machine_binding_selection_does_not_write_a_project_file() {
    let binding_path = path("project-boundary");
    let project = path("project.sdc");
    let _ = std::fs::remove_file(&binding_path);
    let _ = std::fs::remove_file(&project);
    std::fs::write(&project, b"project-content-must-not-change").unwrap();
    let before = std::fs::read(&project).unwrap();
    let a = port("COM3", r"FTDIBUS\A\0000", "A-SERIAL");
    select_binding_from_ports(
        &binding_path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: a.name.clone(),
            windows_device_instance_id: a.windows_device_instance_id.clone().unwrap(),
        },
        &[a],
    )
    .unwrap();
    assert_eq!(std::fs::read(&project).unwrap(), before);
    let _ = std::fs::remove_file(binding_path);
    let _ = std::fs::remove_file(project);
}

#[test]
fn machine_binding_corrupt_future_and_unknown_json_fail_closed_without_rewrite() {
    let path = path("invalid-json");
    let _ = std::fs::remove_file(&path);
    let a = port("COM3", r"FTDIBUS\A\0000", "A-SERIAL");
    select_binding_from_ports(
        &path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: a.name.clone(),
            windows_device_instance_id: a.windows_device_instance_id.clone().unwrap(),
        },
        &[a],
    )
    .unwrap();
    let valid = std::fs::read(&path).unwrap();
    let invalid_cases = [
        b"{ not valid JSON".as_slice().to_vec(),
        br#"{"version":2,"selected":{"port_name":"COM3","port_type":"USB","usb_vid":1027,"usb_pid":24577,"serial_number":"A","manufacturer":"FTDI","product":"USB","windows_device_instance_id":"FTDIBUS\\A\\0000"}}"#.to_vec(),
        br#"{"version":1,"selected":{"port_name":"COM3","port_type":"USB","usb_vid":1027,"usb_pid":24577,"serial_number":"A","manufacturer":"FTDI","product":"USB","windows_device_instance_id":"FTDIBUS\\A\\0000"},"unknown":true}"#.to_vec(),
    ];
    for invalid in invalid_cases {
        std::fs::write(&path, &invalid).unwrap();
        assert_eq!(
            binding_status_from_path(&path, &[]).state,
            SerialDmxMachineBindingStateV1::BlockedPersistence
        );
        assert_eq!(std::fs::read(&path).unwrap(), invalid);
    }
    std::fs::write(&path, valid).unwrap();
    let _ = std::fs::remove_file(path);
}

#[test]
fn machine_binding_failed_or_ambiguous_reselection_preserves_prior_local_bytes() {
    let path = path("reselection-bytes");
    let _ = std::fs::remove_file(&path);
    let a = port("COM3", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM4", r"FTDIBUS\B\0000", "B-SERIAL");
    select_binding_from_ports(
        &path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: a.name.clone(),
            windows_device_instance_id: a.windows_device_instance_id.clone().unwrap(),
        },
        &[a.clone()],
    )
    .unwrap();
    let before = std::fs::read(&path).unwrap();
    let missing = select_binding_from_ports(
        &path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: b.name,
            windows_device_instance_id: b.windows_device_instance_id.unwrap(),
        },
        &[a.clone()],
    )
    .expect_err("disappeared interface cannot replace A");
    assert!(missing.contains("no longer enumerated"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let ambiguous = select_binding_from_ports(
        &path,
        &SelectSerialDmxMachineBindingRequestV1 {
            port_name: a.name.clone(),
            windows_device_instance_id: a.windows_device_instance_id.clone().unwrap(),
        },
        &[a.clone(), a],
    )
    .expect_err("ambiguous exact evidence cannot replace prior binding");
    assert!(ambiguous.contains("ambiguous"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    let _ = std::fs::remove_file(path);
}

#[test]
fn machine_binding_rejects_zero_vid_or_pid_on_selection_and_reload() {
    let path = path("zero-usb-id");
    let _ = std::fs::remove_file(&path);
    let a = port("COM3", r"FTDIBUS\A\0000", "A-SERIAL");
    for (field, value) in [("VID", true), ("PID", false)] {
        let mut zero = a.clone();
        if value {
            zero.usb_vid = Some(0);
        } else {
            zero.usb_pid = Some(0);
        }
        let error = select_binding_from_ports(
            &path,
            &SelectSerialDmxMachineBindingRequestV1 {
                port_name: zero.name.clone(),
                windows_device_instance_id: zero.windows_device_instance_id.clone().unwrap(),
            },
            &[zero],
        )
        .expect_err(&format!(
            "a zero USB {field} cannot become a machine-local identity"
        ));
        assert!(error.contains("nonzero USB VID and PID"));
    }

    std::fs::write(
        &path,
        br#"{"version":1,"selected":{"port_name":"COM3","port_type":"USB","usb_vid":0,"usb_pid":24577,"serial_number":"A","manufacturer":"FTDI","product":"USB","windows_device_instance_id":"FTDIBUS\\A\\0000"}}"#,
    )
    .unwrap();
    assert_eq!(
        binding_status_from_path(&path, &[a]).state,
        SerialDmxMachineBindingStateV1::BlockedPersistence,
        "a persisted zero USB identity must never recover into an active selection"
    );
    let _ = std::fs::remove_file(path);
}
