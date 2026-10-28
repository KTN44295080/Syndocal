use super::*;
use protocol::network_dmx_routes::{NetworkDmxProtocol, NetworkDmxRoute};

fn runtime() -> EngineRuntime {
    let mut runtime = EngineRuntime::new(staged_show_artnet_loopback_output());
    runtime.output.enabled = true;
    runtime.dmx_sender = Some(DmxSender::TestExactArtNetRoute);
    runtime
}
fn route(universe: u16, port: u16, protocol: NetworkDmxProtocol) -> NetworkDmxRoute {
    NetworkDmxRoute {
        enabled: true,
        protocol,
        universe,
        port,
        target_ip: "127.0.0.1".into(),
    }
}
fn configure(
    runtime: &mut EngineRuntime,
    routes: Vec<NetworkDmxRoute>,
    fail: bool,
) -> Result<(), String> {
    let before = runtime.build_snapshot(0);
    let safety = runtime.shared_telemetry.safety_blackout_authority();
    let (ack, receiver) = mpsc::sync_channel(1);
    runtime.apply_command(EngineCommand::SetNetworkDmxRoutesPublished {
        expected_outputs: before.dmx_outputs.clone(),
        routes,
        expected_safety_epoch: safety.epoch,
        expected_safety_generation: safety.generation,
        expires_at: Instant::now() + Duration::from_secs(1),
        ack,
    });
    runtime.fail_next_pending_publication = fail;
    let published = RwLock::new(before);
    runtime.publish_pending_command_acks(0, &published);
    let result = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
    if result.is_ok() {
        assert_eq!(
            published.read().unwrap().dmx_outputs,
            runtime.build_snapshot(0).dmx_outputs
        );
    }
    result
}

#[test]
fn network_dmx_routes_publish_two_independent_universes_to_loopback() {
    for protocol in [NetworkDmxProtocol::ArtNet, NetworkDmxProtocol::Sacn] {
        let mut runtime = runtime();
        let receivers = [
            UdpSocket::bind("127.0.0.1:0").unwrap(),
            UdpSocket::bind("127.0.0.1:0").unwrap(),
        ];
        let first_universe = if protocol == NetworkDmxProtocol::Sacn {
            1
        } else {
            0
        };
        let routes = receivers
            .iter()
            .enumerate()
            .map(|(index, socket)| {
                route(
                    first_universe + index as u16,
                    socket.local_addr().unwrap().port(),
                    protocol,
                )
            })
            .collect::<Vec<_>>();
        for (index, universe) in [first_universe, first_universe + 1].into_iter().enumerate() {
            let mut request = sample_patch_request("Independent universe fixture", 1);
            request.universe = universe;
            runtime.apply_command(EngineCommand::PatchFixture {
                fixture_id: index as u64 + 1,
                request,
                profile: sample_profile(),
            });
            runtime.apply_command(EngineCommand::SetAttribute {
                fixture_id: index as u64 + 1,
                attribute: "Dimmer".into(),
                value: if index == 0 { 64 * 257 } else { 192 * 257 },
            });
        }
        configure(&mut runtime, routes.clone(), false).unwrap();
        assert_eq!(runtime.additional_dmx_outputs.len(), 1);
        let published = RwLock::new(runtime.build_snapshot(0));
        runtime.tick(0, &published);
        for (index, receiver) in receivers.iter().enumerate() {
            receiver
                .set_read_timeout(Some(Duration::from_secs(1)))
                .unwrap();
            let mut packet = [0u8; 700];
            let (size, _) = receiver.recv_from(&mut packet).unwrap();
            let (universe, value) = match protocol {
                NetworkDmxProtocol::ArtNet => {
                    assert_eq!(size, 530);
                    (u16::from_le_bytes([packet[14], packet[15]]), packet[18])
                }
                NetworkDmxProtocol::Sacn => {
                    assert_eq!(size, 638);
                    (u16::from_be_bytes([packet[113], packet[114]]), packet[126])
                }
            };
            assert_eq!(universe, routes[index].universe);
            assert_eq!(value, if index == 0 { 64 } else { 192 });
        }
    }
}

#[test]
fn network_dmx_routes_failed_publication_restores_exact_prior_senders() {
    let mut runtime = runtime();
    configure(
        &mut runtime,
        vec![route(1, 16455, NetworkDmxProtocol::ArtNet)],
        false,
    )
    .unwrap();
    let prior = runtime.build_snapshot(0).dmx_outputs;
    let generation = runtime.dmx_route_configuration_generation;
    let prior_sender = runtime.dmx_sender.as_ref().map(|s| s as *const DmxSender);
    assert!(
        configure(
            &mut runtime,
            vec![route(2, 16456, NetworkDmxProtocol::ArtNet)],
            true
        )
        .is_err()
    );
    assert_eq!(runtime.build_snapshot(0).dmx_outputs, prior);
    assert_eq!(runtime.dmx_route_configuration_generation, generation);
    assert_eq!(
        runtime.dmx_sender.as_ref().map(|s| s as *const DmxSender),
        prior_sender
    );
}

#[test]
fn network_dmx_routes_stale_expired_and_safety_changes_do_not_mutate_routes() {
    let mut runtime = runtime();
    let before = runtime.build_snapshot(0).dmx_outputs;
    let safety = runtime.shared_telemetry.safety_blackout_authority();
    let requested = vec![route(1, 16455, NetworkDmxProtocol::ArtNet)];
    assert!(
        runtime
            .apply_network_dmx_routes(
                &[],
                &requested,
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1)
            )
            .is_err()
    );
    assert!(
        runtime
            .apply_network_dmx_routes(
                &before,
                &requested,
                safety.epoch,
                safety.generation,
                Instant::now() - Duration::from_secs(1)
            )
            .is_err()
    );
    assert!(
        runtime
            .apply_network_dmx_routes(
                &before,
                &requested,
                safety.epoch,
                safety.generation + 1,
                Instant::now() + Duration::from_secs(1)
            )
            .is_err()
    );
    assert!(
        runtime
            .apply_network_dmx_routes(
                &before,
                &[requested[0].clone(), requested[0].clone()],
                safety.epoch,
                safety.generation,
                Instant::now() + Duration::from_secs(1)
            )
            .is_err()
    );
    assert_eq!(runtime.build_snapshot(0).dmx_outputs, before);
    assert!(runtime.additional_dmx_outputs.is_empty());
}

#[test]
fn complete_network_table_needs_no_u0_and_empty_sentinel_round_trips() {
    let mut runtime = EngineRuntime::new(DmxOutputConfig {
        enabled: false,
        ..DmxOutputConfig::default()
    });
    configure(
        &mut runtime,
        vec![route(7, 16455, NetworkDmxProtocol::ArtNet)],
        false,
    )
    .unwrap();
    assert_eq!(runtime.output.universe, 7);
    assert_eq!(runtime.dmx_output_snapshot().len(), 1);
    configure(&mut runtime, vec![], false).unwrap();
    assert!(protocol::network_dmx_routes::is_empty_network_dmx_output(
        &runtime.output
    ));
    assert!(runtime.dmx_sender.is_none());
    let persisted = runtime.build_persistence_snapshot();
    let prepared = prepare_project_snapshot_persistence(persisted).unwrap();
    assert!(protocol::network_dmx_routes::is_empty_network_dmx_output(
        &prepared.output
    ));
    let explicit_disabled_u0 = NetworkDmxRoute {
        enabled: false,
        ..route(0, 6454, NetworkDmxProtocol::ArtNet)
    };
    configure(&mut runtime, vec![explicit_disabled_u0.clone()], false).unwrap();
    assert_eq!(runtime.output, explicit_disabled_u0.config());
    assert!(!protocol::network_dmx_routes::is_empty_network_dmx_output(
        &runtime.output
    ));
}

#[test]
fn failed_complete_publication_restores_primary_and_additional_endpoints() {
    let sockets = [
        UdpSocket::bind("127.0.0.1:0").unwrap(),
        UdpSocket::bind("127.0.0.1:0").unwrap(),
    ];
    let mut runtime = runtime();
    let routes = sockets
        .iter()
        .enumerate()
        .map(|(i, socket)| {
            route(
                i as u16 + 7,
                socket.local_addr().unwrap().port(),
                NetworkDmxProtocol::ArtNet,
            )
        })
        .collect::<Vec<_>>();
    configure(&mut runtime, routes.clone(), false).unwrap();
    let before = runtime.dmx_output_snapshot();
    assert!(
        configure(
            &mut runtime,
            vec![route(11, 16456, NetworkDmxProtocol::ArtNet)],
            true
        )
        .is_err()
    );
    assert_eq!(runtime.dmx_output_snapshot(), before);
    runtime
        .dmx_sender
        .as_mut()
        .unwrap()
        .send_dmx_frame(7, &[71; 512])
        .unwrap();
    runtime.additional_dmx_outputs[0]
        .sender
        .as_mut()
        .unwrap()
        .send_dmx_frame(8, &[81; 512])
        .unwrap();
    for (index, socket) in sockets.iter().enumerate() {
        socket
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let mut packet = [0; 530];
        assert_eq!(socket.recv(&mut packet).unwrap(), 530);
        assert_eq!(
            u16::from_le_bytes([packet[14], packet[15]]),
            index as u16 + 7
        );
        assert_eq!(&packet[18..], &[if index == 0 { 71 } else { 81 }; 512]);
    }
}

#[test]
fn complete_network_table_rejects_standby() {
    let mut standby = EngineRuntime::new_with_shared_telemetry_and_ownership(
        DmxOutputConfig::default(),
        Arc::new(EngineSharedTelemetry::new()),
        OutputOwnershipGate::for_role(MachineOutputRole::Standby),
        Arc::new(RwLock::new(HashMap::new())),
        Arc::new(RwLock::new(TimelineAudioProjectionAuthority::default())),
        Arc::new(AtomicU64::new(1)),
        Arc::new(AtomicU64::new(1)),
        (
            Arc::new(AtomicBool::new(false)),
            Arc::new(AtomicBool::new(false)),
        ),
    );
    assert!(
        configure(
            &mut standby,
            vec![route(7, 16455, NetworkDmxProtocol::ArtNet)],
            false
        )
        .is_err()
    );
}

#[test]
fn complete_network_table_configures_under_exact_s0_without_releasing_it() {
    let mut runtime = runtime();
    let mut request = sample_patch_request("Blackout configured fixture", 1);
    request.universe = 7;
    runtime.apply_command(EngineCommand::PatchFixture {
        fixture_id: 1,
        request,
        profile: sample_profile(),
    });
    runtime.apply_command(EngineCommand::SetAttribute {
        fixture_id: 1,
        attribute: "Dimmer".into(),
        value: 149 * 257,
    });
    let (_, safety) = runtime.shared_telemetry.engage_safety_blackout().unwrap();
    runtime.safety_blackout_engaged = true;
    let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
    receiver
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    configure(
        &mut runtime,
        vec![route(
            7,
            receiver.local_addr().unwrap().port(),
            NetworkDmxProtocol::ArtNet,
        )],
        false,
    )
    .unwrap();
    assert_eq!(runtime.shared_telemetry.safety_blackout_authority(), safety);
    let snapshot = RwLock::new(runtime.build_snapshot(0));
    runtime.tick(1, &snapshot);
    let mut packet = [0; 530];
    assert_eq!(receiver.recv(&mut packet).unwrap(), 530);
    assert_eq!(&packet[18..], &[0; 512]);
    let (_, _, released) = runtime
        .shared_telemetry
        .release_safety_blackout(safety.epoch, safety.generation)
        .unwrap();
    runtime.safety_blackout_engaged = released.engaged;
    runtime.tick(2, &snapshot);
    assert_eq!(receiver.recv(&mut packet).unwrap(), 530);
    assert_eq!(packet[18], 149);
}
