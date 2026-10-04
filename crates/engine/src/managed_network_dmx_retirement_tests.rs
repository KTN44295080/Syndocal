use super::*;

fn route(socket: &UdpSocket, protocol: DmxOutputProtocol, universe: u16) -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: true,
        protocol,
        target_ip: "127.0.0.1".into(),
        port: socket.local_addr().unwrap().port(),
        universe,
        serial_port: String::new(),
        serial_baud_rate: 57600,
    }
}

fn fixture(
    config: DmxOutputConfig,
) -> (
    EngineRuntime,
    SafetyBlackoutAuthority,
    OutputOwnershipTeardownLease,
) {
    let telemetry = Arc::new(EngineSharedTelemetry::new());
    let mut runtime = EngineRuntime::new_with_shared_telemetry(config, Arc::clone(&telemetry));
    let (_, safety) = telemetry.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let lease = runtime
        .output_ownership_gate
        .begin_failure_fence("configured network retirement test");
    (runtime, safety, lease)
}

fn socket() -> UdpSocket {
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
    socket
        .set_read_timeout(Some(Duration::from_millis(200)))
        .unwrap();
    socket
}

fn retire(
    runtime: &mut EngineRuntime,
    safety: SafetyBlackoutAuthority,
    epoch: u64,
    operation: &ManagedShowDmxFailStopOperationInner,
) -> Result<ManagedShowDmxFailStopReceipt, ManagedShowDmxFailStopError> {
    runtime.apply_managed_dmx_retirement_for_scope(
        ManagedDmxRetirementScope::ConfiguredNetwork,
        safety.epoch,
        safety.generation,
        epoch,
        operation,
    )
}

#[test]
fn configured_network_retirement_zeros_multiple_owned_destinations_and_preserves_configuration() {
    let primary = socket();
    let secondary = socket();
    let first = route(&primary, DmxOutputProtocol::ArtNet, 7);
    let second = route(&secondary, DmxOutputProtocol::Sacn, 9);
    let (mut runtime, safety, _lease) = fixture(first.clone());
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new("127.0.0.1", first.port).unwrap(),
    ));
    runtime
        .additional_dmx_outputs
        .push(RuntimeDmxOutput::new_with_lighting_allowed(
            second.clone(),
            true,
            &runtime.shared_telemetry.open_dmx_safety_write_gate,
        ));
    let epoch = runtime.output_ownership_gate.status().epoch;
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let receipt = retire(&mut runtime, safety, epoch, &op).unwrap();
    assert!(
        !receipt.artnet_zero_accepted(),
        "general retirement cannot project the exact-show local datagram receipt"
    );
    let mut packet = [0u8; 1024];
    let count = primary.recv(&mut packet).unwrap();
    assert_eq!(count, 530);
    assert_eq!(u16::from_le_bytes([packet[14], packet[15]]), 7);
    assert_eq!(&packet[18..count], &[0; 512]);
    let count = secondary.recv(&mut packet).unwrap();
    assert_eq!(count, io::sacn::SACN_PACKET_LEN);
    assert_eq!(u16::from_be_bytes([packet[113], packet[114]]), 9);
    assert_eq!(&packet[126..count], &[0; 512]);
    assert!(primary.recv(&mut packet).is_err());
    assert!(secondary.recv(&mut packet).is_err());
    assert_eq!(runtime.output, first);
    assert_eq!(runtime.additional_dmx_outputs[0].config, second);
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert_eq!(runtime.output_ownership_gate.status().epoch, epoch);
    assert!(
        runtime
            .shared_telemetry
            .try_safety_blackout_authority()
            .unwrap()
            .engaged
    );
}

#[test]
fn configured_network_retirement_stale_authority_preserves_live_sender_without_a_zero_send() {
    let receiver = socket();
    let config = route(&receiver, DmxOutputProtocol::ArtNet, 3);
    let (mut runtime, safety, _lease) = fixture(config.clone());
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new("127.0.0.1", config.port).unwrap(),
    ));
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let stale_epoch = runtime.output_ownership_gate.status().epoch + 1;
    assert!(matches!(
        retire(&mut runtime, safety, stale_epoch, &op),
        Err(ManagedShowDmxFailStopError::RejectedBeforeCommit(_))
    ));
    assert!(!op.is_committing());
    assert!(runtime.dmx_sender.is_some());
    assert_eq!(runtime.output, config);
    assert!(receiver.recv(&mut [0; 1024]).is_err());
}

#[test]
fn configured_network_retirement_sends_zero_after_the_prior_failure_fence_dropped_its_sender() {
    let receiver = socket();
    let config = route(&receiver, DmxOutputProtocol::ArtNet, 11);
    let (mut runtime, safety, _lease) = fixture(config.clone());
    // The safety/failure publication may have retired the persistent sender
    // before this consumed command reaches the engine queue.
    runtime.dmx_sender = None;
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let epoch = runtime.output_ownership_gate.status().epoch;
    retire(&mut runtime, safety, epoch, &op).unwrap();
    let mut packet = [0; 530];
    assert_eq!(receiver.recv(&mut packet).unwrap(), 530);
    assert_eq!(u16::from_le_bytes([packet[14], packet[15]]), 11);
    assert_eq!(&packet[18..], &[0; 512]);
    assert!(receiver.recv(&mut packet).is_err());
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert_eq!(runtime.output, config);
}

#[test]
fn configured_network_retirement_disabled_absent_route_does_not_emit_or_create_a_sender() {
    let receiver = socket();
    let mut config = route(&receiver, DmxOutputProtocol::ArtNet, 11);
    config.enabled = false;
    let (mut runtime, safety, _lease) = fixture(config.clone());
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let epoch = runtime.output_ownership_gate.status().epoch;
    retire(&mut runtime, safety, epoch, &op).unwrap();
    assert!(receiver.recv(&mut [0; 530]).is_err());
    assert!(runtime.managed_show_dmx_terminal_absent());
    assert_eq!(runtime.output, config);
}

#[test]
fn configured_network_retirement_cancelled_operation_preserves_live_sender() {
    let receiver = socket();
    let config = route(&receiver, DmxOutputProtocol::ArtNet, 3);
    let (mut runtime, safety, _lease) = fixture(config.clone());
    runtime.dmx_sender = Some(DmxSender::ArtNet(
        ArtNetSender::new("127.0.0.1", config.port).unwrap(),
    ));
    let op = Arc::new(ManagedShowDmxFailStopOperationInner::new());
    assert!(op.admit());
    assert!(matches!(
        (ManagedShowDmxFailStopOperation {
            inner: Arc::clone(&op)
        })
        .wait(Duration::ZERO),
        Err(ManagedShowDmxFailStopError::CancelledBeforeCommit(_))
    ));
    let epoch = runtime.output_ownership_gate.status().epoch;
    assert!(matches!(
        retire(&mut runtime, safety, epoch, &op),
        Err(ManagedShowDmxFailStopError::CancelledBeforeCommit(_))
    ));
    assert!(runtime.dmx_sender.is_some());
    assert!(receiver.recv(&mut [0; 1024]).is_err());
}

#[test]
fn configured_network_retirement_send_failure_still_retires_every_sender_without_success() {
    let receiver = socket();
    let config = route(&receiver, DmxOutputProtocol::ArtNet, 4);
    let (mut runtime, safety, _lease) = fixture(config.clone());
    runtime.dmx_sender = Some(DmxSender::TestFailingArtNetSend);
    runtime
        .additional_dmx_outputs
        .push(RuntimeDmxOutput::new_with_lighting_allowed(
            config.clone(),
            true,
            &runtime.shared_telemetry.open_dmx_safety_write_gate,
        ));
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let epoch = runtime.output_ownership_gate.status().epoch;
    assert!(matches!(
        retire(&mut runtime, safety, epoch, &op),
        Err(ManagedShowDmxFailStopError::InDoubt(_))
    ));
    assert!(runtime.managed_show_dmx_terminal_absent());
    let mut packet = [0; 530];
    assert_eq!(receiver.recv(&mut packet).unwrap(), 530);
    assert_eq!(&packet[18..], &[0; 512]);
    assert_eq!(runtime.output, config);
    assert!(
        runtime
            .shared_telemetry
            .try_safety_blackout_authority()
            .unwrap()
            .engaged
    );
}

#[test]
fn configured_network_retirement_does_not_substitute_network_proof_for_unverified_serial() {
    let receiver = socket();
    let mut config = route(&receiver, DmxOutputProtocol::ArtNet, 0);
    config.protocol = DmxOutputProtocol::EnttecUsbPro;
    // No serial device is opened by this negative admission fixture.
    let (mut runtime, safety, _lease) = fixture(config);
    let op = ManagedShowDmxFailStopOperationInner::new();
    assert!(op.admit());
    let epoch = runtime.output_ownership_gate.status().epoch;
    assert!(matches!(
        retire(&mut runtime, safety, epoch, &op),
        Err(ManagedShowDmxFailStopError::InDoubt(_))
    ));
    assert!(op.is_committing());
    assert!(receiver.recv(&mut [0; 1024]).is_err());
}
