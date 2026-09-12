//! Manually paired, authenticated ShowClock unicast transport.
//!
//! The transport carries already-authenticated protocol envelopes over an
//! explicitly configured peer address.  It has no discovery path and never
//! grants output ownership; callers must run samples/actions through the
//! protocol admission and fence boundaries after receiving them.

use std::{
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use protocol::show_clock::{
    AuthenticatedShowClockAction, AuthenticatedShowClockSample, SHOW_CLOCK_PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use thiserror::Error;

pub const SHOW_CLOCK_LAN_PORT: u16 = 44_666;
pub const SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES: usize = 8 * 1024;

const SHOW_CLOCK_LAN_MAGIC: &str = "syndocal.show-clock";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "payload", deny_unknown_fields)]
pub enum ShowClockLanMessage {
    Sample(AuthenticatedShowClockSample),
    Action(AuthenticatedShowClockAction),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ShowClockLanEnvelope {
    magic: String,
    protocol_version: u16,
    message: ShowClockLanMessage,
}

#[derive(Debug, Error)]
pub enum ShowClockLanError {
    #[error("failed to bind ShowClock LAN socket: {0}")]
    Bind(#[source] std::io::Error),
    #[error("failed to configure ShowClock LAN socket: {0}")]
    Configure(#[source] std::io::Error),
    #[error("failed to encode ShowClock LAN packet: {0}")]
    Encode(#[source] serde_json::Error),
    #[error("failed to send ShowClock LAN packet: {0}")]
    Send(#[source] std::io::Error),
    #[error("failed to receive ShowClock LAN packet: {0}")]
    Receive(#[source] std::io::Error),
    #[error("ShowClock LAN receive timed out")]
    Timeout,
    #[error("ShowClock LAN packet exceeded {SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES} bytes")]
    PacketTooLarge,
    #[error("invalid ShowClock LAN packet: {0}")]
    Decode(String),
}

/// One socket with one exact manually paired peer.  The bind address and
/// peer address are persisted by the owning setup layer; this type does not
/// infer or discover them.
pub struct ShowClockLanTransport {
    socket: UdpSocket,
    peer: SocketAddr,
}

impl std::fmt::Debug for ShowClockLanTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShowClockLanTransport")
            .field("local", &self.socket.local_addr().ok())
            .field("peer", &self.peer)
            .finish()
    }
}

impl ShowClockLanTransport {
    pub fn bind(bind: SocketAddr, peer: SocketAddr) -> Result<Self, ShowClockLanError> {
        let socket = UdpSocket::bind(bind).map_err(ShowClockLanError::Bind)?;
        socket
            .set_nonblocking(false)
            .map_err(ShowClockLanError::Configure)?;
        Ok(Self { socket, peer })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ShowClockLanError> {
        self.socket.local_addr().map_err(ShowClockLanError::Receive)
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    pub fn send_sample(
        &self,
        sample: &AuthenticatedShowClockSample,
    ) -> Result<usize, ShowClockLanError> {
        self.send(ShowClockLanMessage::Sample(sample.clone()))
    }

    pub fn send_action(
        &self,
        action: &AuthenticatedShowClockAction,
    ) -> Result<usize, ShowClockLanError> {
        self.send(ShowClockLanMessage::Action(action.clone()))
    }

    pub fn send(&self, message: ShowClockLanMessage) -> Result<usize, ShowClockLanError> {
        let envelope = ShowClockLanEnvelope {
            magic: SHOW_CLOCK_LAN_MAGIC.to_string(),
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            message,
        };
        let encoded = serde_json::to_vec(&envelope).map_err(ShowClockLanError::Encode)?;
        if encoded.len() > SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES {
            return Err(ShowClockLanError::PacketTooLarge);
        }
        self.socket
            .send_to(&encoded, self.peer)
            .map_err(ShowClockLanError::Send)
    }

    /// Receive one packet from the exact paired endpoint before the deadline.
    /// Traffic from other endpoints is ignored and cannot extend the deadline.
    /// A malformed packet from the paired endpoint fails closed.
    pub fn receive(&self, timeout: Duration) -> Result<ShowClockLanMessage, ShowClockLanError> {
        let deadline = Instant::now() + timeout;
        let mut buffer = [0_u8; SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ShowClockLanError::Timeout);
            }
            self.socket
                .set_read_timeout(Some(remaining))
                .map_err(ShowClockLanError::Configure)?;
            let (received, source) = match self.socket.recv_from(&mut buffer) {
                Ok(packet) => packet,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                    ) =>
                {
                    return Err(ShowClockLanError::Timeout)
                }
                Err(error) => return Err(ShowClockLanError::Receive(error)),
            };
            if source != self.peer {
                continue;
            }
            if received == buffer.len() {
                return Err(ShowClockLanError::PacketTooLarge);
            }
            let envelope = serde_json::from_slice::<ShowClockLanEnvelope>(&buffer[..received])
                .map_err(|error| ShowClockLanError::Decode(error.to_string()))?;
            if envelope.magic != SHOW_CLOCK_LAN_MAGIC {
                return Err(ShowClockLanError::Decode(
                    "unexpected ShowClock LAN magic".to_string(),
                ));
            }
            if envelope.protocol_version != SHOW_CLOCK_PROTOCOL_VERSION {
                return Err(ShowClockLanError::Decode(
                    "unsupported ShowClock LAN protocol version".to_string(),
                ));
            }
            return Ok(envelope.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::show_clock::{
        AuthenticatedShowClockSample, ShowClockHash, ShowClockNodeId, ShowClockNonce,
        ShowClockSample, ShowClockSessionId, ShowClockSource, ShowTransportState,
        SHOW_CLOCK_SCHEMA_VERSION,
    };

    const KEY: [u8; 32] = [0x42; 32];

    fn sample() -> AuthenticatedShowClockSample {
        AuthenticatedShowClockSample::sign(
            ShowClockSample {
                schema_version: SHOW_CLOCK_SCHEMA_VERSION,
                protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
                session_id: ShowClockSessionId::new("session-1").unwrap(),
                sender: ShowClockNodeId::new("node-primary").unwrap(),
                clock_generation: 1,
                fencing_generation: 1,
                sequence: 1,
                sender_monotonic_us: 1_000_000,
                show_time_us: 1_000_000,
                bpm_milli: 120_000,
                beat_phase_ppm: 0,
                transport: ShowTransportState::Playing,
                source: ShowClockSource::ShowClock,
                project_hash: ShowClockHash([0x11; 32]),
                media_hash: ShowClockHash([0x22; 32]),
                expires_after_us: 250_000,
                nonce: ShowClockNonce([1; 16]),
            },
            &KEY,
        )
        .unwrap()
    }

    #[test]
    fn paired_loopback_round_trips_signed_sample_without_discovery() {
        let receiver_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let sender_probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = receiver_probe.local_addr().unwrap();
        let sender_addr = sender_probe.local_addr().unwrap();
        drop(receiver_probe);
        drop(sender_probe);
        let receiver = ShowClockLanTransport::bind(receiver_addr, sender_addr).unwrap();
        let sender = ShowClockLanTransport::bind(sender_addr, receiver_addr).unwrap();
        let expected = sample();
        sender.send_sample(&expected).unwrap();
        let received = receiver.receive(Duration::from_millis(250)).unwrap();
        assert_eq!(received, ShowClockLanMessage::Sample(expected));
    }

    #[test]
    fn wrong_source_does_not_extend_receive_deadline() {
        let receiver = ShowClockLanTransport::bind(
            "127.0.0.1:0".parse().unwrap(),
            "127.0.0.1:9".parse().unwrap(),
        )
        .unwrap();
        let unrelated = UdpSocket::bind("127.0.0.1:0").unwrap();
        let destination = receiver.local_addr().unwrap();
        unrelated.send_to(b"noise", destination).unwrap();
        assert!(matches!(
            receiver.receive(Duration::from_millis(30)),
            Err(ShowClockLanError::Timeout)
        ));
    }

    #[test]
    fn malformed_paired_packet_fails_closed() {
        let receiver_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = receiver_socket.local_addr().unwrap();
        let sender_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let sender_addr = sender_socket.local_addr().unwrap();
        drop(receiver_socket);
        let receiver = ShowClockLanTransport::bind(receiver_addr, sender_addr).unwrap();
        sender_socket.send_to(b"{}", receiver_addr).unwrap();
        assert!(matches!(
            receiver.receive(Duration::from_millis(250)),
            Err(ShowClockLanError::Decode(_))
        ));
    }
}
