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
        assert_eq!(runtime.additional_dmx_outputs.len(), 2);
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
    let prior_sender = runtime.additional_dmx_outputs[0]
        .sender
        .as_ref()
        .map(|s| s as *const DmxSender);
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
        runtime.additional_dmx_outputs[0]
            .sender
            .as_ref()
            .map(|s| s as *const DmxSender),
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
