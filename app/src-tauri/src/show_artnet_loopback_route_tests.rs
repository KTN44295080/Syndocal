//! Focused proof for the one local Art-Net show-route activation boundary.
//!
//! The production show is intentionally fixed to the same-PC receiver:
//! ArtDmx, 127.0.0.1:6454, wire universe 0, and one 512-channel frame.

use super::*;

fn staged_route() -> DmxOutputConfig {
    DmxOutputConfig {
        enabled: false,
        protocol: DmxOutputProtocol::ArtNet,
        target_ip: SHOW_ARTNET_LOOPBACK_TARGET_IP.to_string(),
        port: SHOW_ARTNET_LOOPBACK_PORT,
        universe: SHOW_ARTNET_LOOPBACK_UNIVERSE,
        serial_port: String::new(),
        serial_baud_rate: DmxOutputConfig::default().serial_baud_rate,
    }
}

#[test]
fn show_artnet_loopback_matcher_accepts_only_the_disabled_same_pc_route() {
    let staged = staged_route();
    assert!(is_exact_staged_show_artnet_loopback_route(&staged));
    for invalid in [
        DmxOutputConfig {
            enabled: true,
            ..staged.clone()
        },
        DmxOutputConfig {
            protocol: DmxOutputProtocol::Sacn,
            ..staged.clone()
        },
        DmxOutputConfig {
            target_ip: "192.168.50.2".to_string(),
            ..staged.clone()
        },
        DmxOutputConfig {
            port: 6455,
            ..staged.clone()
        },
        DmxOutputConfig {
            universe: 1,
            ..staged.clone()
        },
        DmxOutputConfig {
            serial_port: "COM3".to_string(),
            ..staged
        },
    ] {
        assert!(
            !is_exact_staged_show_artnet_loopback_route(&invalid),
            "ineligible authored route: {invalid:?}"
        );
    }
}

#[test]
fn show_artnet_loopback_has_no_machine_local_serial_binding() {
    let route = staged_route();
    assert!(route.serial_port.is_empty());
    assert_eq!(route.target_ip, "127.0.0.1");
    assert_eq!(route.port, 6454);
    assert_eq!(route.universe, 0);
}

#[test]
fn show_artnet_loopback_commit_returns_the_next_project_fence_for_stage_four() {
    let p0 = OutputControlFenceV1 {
        process_incarnation: 11,
        session_incarnation: 12,
        project_epoch: 3,
        project_revision: 4,
        project_checkpoint_hash: "a".repeat(64),
        project_publication_generation: 5,
        output_epoch: 6,
        output_generation: 7,
        safety_blackout_epoch: 8,
        safety_blackout_generation: 9,
    };

    let p1 = committed_show_artnet_loopback_route_fence(
        &p0,
        p0.project_epoch,
        p0.project_revision + 1,
        &"b".repeat(64),
        p0.project_publication_generation + 1,
        p0.safety_blackout_epoch,
        p0.safety_blackout_generation,
    );

    assert_eq!(p1.process_incarnation, p0.process_incarnation);
    assert_eq!(p1.session_incarnation, p0.session_incarnation);
    assert_eq!(p1.project_epoch, p0.project_epoch);
    assert_eq!(p1.project_revision, p0.project_revision + 1);
    assert_ne!(p1.project_checkpoint_hash, p0.project_checkpoint_hash);
    assert_eq!(
        p1.project_publication_generation,
        p0.project_publication_generation + 1
    );
    assert_eq!(p1.output_epoch, p0.output_epoch);
    assert_eq!(p1.output_generation, p0.output_generation);
    assert_eq!(p1.safety_blackout_epoch, p0.safety_blackout_epoch);
    assert_eq!(p1.safety_blackout_generation, p0.safety_blackout_generation);
    assert!(p1.validate().is_ok());
}
