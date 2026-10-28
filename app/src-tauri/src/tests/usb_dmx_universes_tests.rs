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
        legacy_u0_invariants: false,
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
fn usb_dmx_universe_bindings_accept_zero_and_reject_duplicates_shared_cables_and_overflow() {
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    assert!(
        validate_bindings(&[binding(0, &a)]).is_ok(),
        "Universe 0 is an independent cable"
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
        validate_bindings(&four).is_ok(),
        "four independent cables including Universe 0"
    );
}

#[test]
fn usb_dmx_universe_bindings_fail_closed_on_unknown_versions_fields_and_size() {
    let path = path("fail-closed");
    std::fs::write(&path, br#"{"version":99,"universes":[]}"#).unwrap();
    assert!(load_bindings(&path).unwrap_err().contains("version 99"));
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
        expected_project: None,
        universe,
        port_name: port.name.clone(),
        windows_device_instance_id: port.windows_device_instance_id.clone(),
        macos_device_instance_id: None,
    }
}

#[test]
fn usb_dmx_universe_binding_keeps_all_universes_disjoint_including_zero() {
    let path = path("disjoint");
    let _ = std::fs::remove_file(&path);
    let a = port("COM5", r"FTDIBUS\A\0000", "A-SERIAL");
    let b = port("COM6", r"FTDIBUS\B\0000", "B-SERIAL");
    let ports = [a.clone(), b.clone()];
    let mut admitted = || Ok(());
    let u0_is_a = u0_status(SerialDmxMachineBindingStateV1::SelectedAndPresent, Some(&a));
    // Once unified, the legacy saved selection is no longer a reservation.
    select_binding_from_ports(&path, &u0_is_a, &request(0, &a), &ports, &mut admitted).unwrap();
    select_binding_from_ports(&path, &u0_is_a, &request(1, &b), &ports, &mut admitted).unwrap();
    let error = select_binding_from_ports(&path, &u0_is_a, &request(2, &b), &ports, &mut admitted)
        .unwrap_err();
    assert!(error.contains("already bound to Universe 1"), "{error}");
    // Rebinding the same universe to the same cable is allowed.
    select_binding_from_ports(&path, &u0_is_a, &request(1, &b), &ports, &mut admitted).unwrap();
    assert_eq!(load_bindings(&path).unwrap().len(), 2);
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

fn migration_fixture(
    name: &str,
    legacy: &SerialPortSummary,
    others: &[UsbDmxUniverseBindingV1],
) -> PathBuf {
    let directory = path(name);
    std::fs::create_dir_all(&directory).unwrap();
    let legacy_path = serial_dmx_machine::serial_dmx_machine_binding_path(&directory);
    std::fs::write(legacy_path, serde_json::to_vec(&serde_json::json!({
        "version": 2, "selected": SerialDmxMachineBindingIdentityV2::from_summary(legacy).unwrap()
    })).unwrap()).unwrap();
    let unified = directory.join(USB_DMX_UNIVERSE_BINDINGS_FILE);
    std::fs::write(
        &unified,
        serde_json::to_vec(&UsbDmxUniverseBindingsFileV1 {
            version: 1,
            universes: others.to_vec(),
        })
        .unwrap(),
    )
    .unwrap();
    unified
}

#[test]
fn usb_dmx_migration_preserves_exact_u0_and_other_bindings_and_empty_does_not_resurrect() {
    let a = port("COM51", r"FTDIBUS\MIGRATION_A\0000", "MIGRATION-A");
    let b = port("COM52", r"FTDIBUS\MIGRATION_B\0000", "MIGRATION-B");
    let path = migration_fixture("migration-preserve", &a, &[binding(7, &b)]);
    let legacy_path = serial_dmx_machine::serial_dmx_machine_binding_path(path.parent().unwrap());
    let legacy_before = std::fs::read(&legacy_path).unwrap();
    // Only an upgraded store may take the unlocked status read.
    assert!(!binding_store_is_current(&path));
    let migrated = load_bindings(&path).unwrap();
    assert!(binding_store_is_current(&path));
    assert!(!binding_store_is_current(&path.with_extension("missing")));
    let mut old_u0 = binding(0, &a);
    old_u0.legacy_u0_invariants = true;
    assert_eq!(migrated, vec![binding(7, &b), old_u0]);
    assert!(
        load_bindings(&path)
            .unwrap()
            .iter()
            .find(|binding| binding.universe == 0)
            .unwrap()
            .legacy_u0_invariants
    );
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy_before);
    let file: UsbDmxUniverseBindingsFileV1 =
        serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    assert_eq!(file.version, 2);
    assert_eq!(
        binding_status(&binding(0, &a), &[a.clone()]).state,
        SerialDmxMachineBindingStateV1::SelectedAndPresent
    );
    select_binding_from_ports(
        &path,
        &u0_status(SerialDmxMachineBindingStateV1::SelectedAndPresent, Some(&a)),
        &request(0, &a),
        &[a],
        &mut || Ok(()),
    )
    .unwrap();
    assert!(
        !load_bindings(&path)
            .unwrap()
            .iter()
            .find(|binding| binding.universe == 0)
            .unwrap()
            .legacy_u0_invariants,
        "explicit selection clears the migration-only policy"
    );
    persist_bindings(&path, &[]).unwrap();
    assert!(load_bindings(&path).unwrap().is_empty());
    assert_eq!(std::fs::read(&legacy_path).unwrap(), legacy_before);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn usb_dmx_migration_conflict_and_corruption_leave_original_bytes_untouched() {
    let a = port("COM53", r"FTDIBUS\MIGRATION_C\0000", "MIGRATION-C");
    let path = migration_fixture("migration-conflict", &a, &[binding(7, &a)]);
    let before = std::fs::read(&path).unwrap();
    assert!(load_bindings(&path).unwrap_err().contains("both Universe"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(&path, br#"{"version":99,"universes":[]}"#).unwrap();
    assert!(load_bindings(&path).unwrap_err().contains("version 99"));
    std::fs::write(&path, br#"{"version":1,"universes":[]}"#).unwrap();
    let legacy_path = serial_dmx_machine::serial_dmx_machine_binding_path(path.parent().unwrap());
    std::fs::write(&legacy_path, br#"{"version":99,"selected":null}"#).unwrap();
    let before = std::fs::read(&path).unwrap();
    assert!(load_bindings(&path)
        .unwrap_err()
        .contains("migration blocked"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
}

#[test]
fn usb_dmx_u0_late_admission_failure_restores_exact_v2_empty_marker() {
    let path = path("u0-admission");
    persist_bindings(&path, &[]).unwrap();
    let before = std::fs::read(&path).unwrap();
    let a = port("COM54", r"FTDIBUS\MIGRATION_D\0000", "MIGRATION-D");
    let mut calls = 0;
    let error = select_binding_from_ports(
        &path,
        &u0_status(SerialDmxMachineBindingStateV1::SelectedAndPresent, Some(&a)),
        &request(0, &a),
        &[a.clone()],
        &mut || {
            calls += 1;
            if calls == 3 {
                Err("late rejection".to_string())
            } else {
                Ok(())
            }
        },
    )
    .unwrap_err();
    assert!(error.contains("previous bindings were restored"));
    assert_eq!(std::fs::read(&path).unwrap(), before);
    assert!(load_bindings(&path).unwrap().is_empty());
    std::fs::remove_file(path).unwrap();
}

#[test]
fn usb_dmx_legacy_u0_policy_is_opt_in_and_rejected_for_other_universes() {
    let a = port("COM55", r"FTDIBUS\POLICY\0000", "POLICY");
    let fresh = binding(0, &a);
    assert!(!fresh.legacy_u0_invariants);
    let encoded = serde_json::to_value(&fresh).unwrap();
    assert!(encoded.get("legacy_u0_invariants").is_none());
    let mut imported = fresh;
    imported.legacy_u0_invariants = true;
    assert!(validate_bindings(&[imported.clone()]).is_ok());
    imported.universe = 1;
    assert!(validate_bindings(&[imported])
        .unwrap_err()
        .contains("only for Universe 0"));
}

#[test]
fn usb_dmx_binding_project_fence_accepts_exact_and_legacy_but_rejects_each_mismatch() {
    let exact = DmxTableProjectIdentity {
        project_epoch: 7,
        project_revision: 11,
        checkpoint_hash: "checkpoint-a".into(),
    };
    assert!(ensure_expected_project(Some(&exact), 7, 11, "checkpoint-a").is_ok());
    assert!(
        ensure_expected_project(None, 99, 88, "checkpoint-b").is_ok(),
        "old clients retain explicit unfenced machine-local behavior"
    );
    for (epoch, revision, hash) in [
        (8, 11, "checkpoint-a"),
        (7, 12, "checkpoint-a"),
        (7, 11, "checkpoint-b"),
    ] {
        assert!(ensure_expected_project(Some(&exact), epoch, revision, hash).is_err());
    }
    let empty = DmxTableProjectIdentity {
        checkpoint_hash: "  ".into(),
        ..exact
    };
    assert!(ensure_expected_project(Some(&empty), 7, 11, "  ").is_err());
}

#[test]
fn usb_dmx_binding_project_fence_request_is_optional_and_nested_shape_is_strict() {
    let old: ClearUsbDmxUniverseBindingRequestV1 =
        serde_json::from_value(serde_json::json!({"universe":0})).unwrap();
    assert!(old.expected_project.is_none());
    let mut request = serde_json::json!({"universe":0,"expectedProject":{"project_epoch":7,"project_revision":11,"checkpoint_hash":"checkpoint-a"}});
    let exact: ClearUsbDmxUniverseBindingRequestV1 =
        serde_json::from_value(request.clone()).unwrap();
    assert!(
        ensure_expected_project(exact.expected_project.as_ref(), 7, 11, "checkpoint-a").is_ok()
    );
    request["expectedProject"]["extra"] = serde_json::json!(true);
    assert!(serde_json::from_value::<ClearUsbDmxUniverseBindingRequestV1>(request).is_err());
}
