use std::{
    env,
    net::UdpSocket,
    process::{Child, Command},
    time::Duration,
};

use io::show_clock_lan::{ShowClockLanMessage, ShowClockLanTransport};
use protocol::show_clock::{
    AuthenticatedShowClockSample, ShowClockHash, ShowClockNodeId, ShowClockNonce, ShowClockSample,
    ShowClockSessionId, ShowClockSource, ShowTransportState, SHOW_CLOCK_PROTOCOL_VERSION,
    SHOW_CLOCK_SCHEMA_VERSION,
};
use protocol::show_clock_runtime::{ShowClockEstimatorState, ShowClockPeerEstimator};

const CHILD_ENV: &str = "SYNDOCAL_SHOWCLOCK_TWO_PROCESS_CHILD";
const KEY: [u8; 32] = [0x42; 32];

struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        match self.0.try_wait() {
            Ok(Some(_)) => {}
            Ok(None) | Err(_) => {
                let _ = self.0.kill();
                let _ = self.0.wait();
            }
        }
    }
}

fn sample(sequence: u64) -> AuthenticatedShowClockSample {
    AuthenticatedShowClockSample::sign(
        ShowClockSample {
            schema_version: SHOW_CLOCK_SCHEMA_VERSION,
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            session_id: ShowClockSessionId::new("two-process-session").unwrap(),
            sender: ShowClockNodeId::new("node-primary").unwrap(),
            clock_generation: 1,
            fencing_generation: 1,
            sequence,
            sender_monotonic_us: 1_000_000 + (sequence - 1) * 250_000,
            show_time_us: 1_000_000 + (sequence - 1) * 250_000,
            bpm_milli: 120_000,
            beat_phase_ppm: 0,
            transport: ShowTransportState::Playing,
            source: ShowClockSource::ShowClock,
            project_hash: ShowClockHash([0x11; 32]),
            media_hash: ShowClockHash([0x22; 32]),
            expires_after_us: 250_000,
            nonce: ShowClockNonce([sequence as u8; 16]),
        },
        &KEY,
    )
    .unwrap()
}

#[test]
fn child_sender() {
    let Some(receiver_address) = env::var_os(CHILD_ENV) else {
        return;
    };
    let sender_address = env::var("SYNDOCAL_SHOWCLOCK_CHILD_BIND").unwrap();
    let sender = ShowClockLanTransport::bind(
        sender_address.parse().unwrap(),
        receiver_address.to_string_lossy().parse().unwrap(),
    )
    .unwrap();
    for sequence in 1..=3 {
        sender.send_sample(&sample(sequence)).unwrap();
    }
}

#[test]
fn paired_transport_round_trips_across_two_processes() {
    let receiver_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
    let sender_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
    let receiver_address = receiver_probe.local_addr().unwrap();
    let sender_address = sender_probe.local_addr().unwrap();
    drop(receiver_probe);
    drop(sender_probe);
    let receiver = ShowClockLanTransport::bind(receiver_address, sender_address).unwrap();

    let mut child = ChildGuard(
        Command::new(env::current_exe().unwrap())
            .arg("--exact")
            .arg("child_sender")
            .arg("--nocapture")
            .env(CHILD_ENV, receiver_address.to_string())
            .env("SYNDOCAL_SHOWCLOCK_CHILD_BIND", sender_address.to_string())
            .spawn()
            .unwrap(),
    );
    let mut validator = protocol::show_clock::ShowClockPeerValidator::new(
        ShowClockSessionId::new("two-process-session").unwrap(),
        ShowClockNodeId::new("node-primary").unwrap(),
        ShowClockHash([0x11; 32]),
        ShowClockHash([0x22; 32]),
        1,
        1,
        KEY,
    )
    .unwrap();
    let mut estimator = ShowClockPeerEstimator::new(Default::default(), 1, 1).unwrap();
    for sequence in 1..=3 {
        let received = receiver.receive(Duration::from_secs(2)).unwrap();
        assert_eq!(received, ShowClockLanMessage::Sample(sample(sequence)));
        let received_at = 1_000_000 + sequence;
        let ShowClockLanMessage::Sample(received) = received else {
            unreachable!("the child only sends samples");
        };
        estimator
            .accept_authenticated_sample(&mut validator, &received, received_at)
            .unwrap();
        estimator.estimator_mut().advance(received_at);
    }
    let exit = child.0.wait().unwrap();
    assert!(exit.success());
    assert_eq!(validator.last_sequence(), 3);
    assert_eq!(
        estimator.estimator().state(),
        ShowClockEstimatorState::Locked
    );
}
