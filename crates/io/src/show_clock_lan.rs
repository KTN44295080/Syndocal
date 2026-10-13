//! Manually paired, authenticated ShowClock unicast transport.
//!
//! The transport carries already-authenticated protocol envelopes over an
//! explicitly configured peer address.  It has no discovery path and never
//! grants output ownership; callers must run samples/actions through the
//! protocol admission and fence boundaries after receiving them.

use std::{
    net::{SocketAddr, UdpSocket},
    sync::atomic::{AtomicU64, Ordering},
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
    /// Saturating count of dropped datagrams: oversized or undecodable ones
    /// from the paired address, plus on Windows every datagram too large to
    /// receive, which the OS discards without reporting its sender.
    dropped_datagrams: AtomicU64,
}

impl std::fmt::Debug for ShowClockLanTransport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ShowClockLanTransport")
            .field("local", &self.socket.local_addr().ok())
            .field("peer", &self.peer)
            .field("dropped_datagrams", &self.dropped_datagrams())
            .finish()
    }
}

impl ShowClockLanTransport {
    pub fn bind(bind: SocketAddr, peer: SocketAddr) -> Result<Self, ShowClockLanError> {
        let socket = UdpSocket::bind(bind).map_err(ShowClockLanError::Bind)?;
        socket
            .set_nonblocking(false)
            .map_err(ShowClockLanError::Configure)?;
        Ok(Self {
            socket,
            peer,
            dropped_datagrams: AtomicU64::new(0),
        })
    }

    pub fn local_addr(&self) -> Result<SocketAddr, ShowClockLanError> {
        self.socket.local_addr().map_err(ShowClockLanError::Receive)
    }

    pub fn peer_addr(&self) -> SocketAddr {
        self.peer
    }

    /// Datagrams `receive` dropped since this transport was bound: oversized
    /// or undecodable ones from the paired address, and on Windows any
    /// datagram too large to receive, whatever its sender.
    pub fn dropped_datagrams(&self) -> u64 {
        self.dropped_datagrams.load(Ordering::Relaxed)
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

    /// Receive one decodable packet from the exact paired endpoint before the
    /// deadline.  Traffic from other endpoints is ignored, and oversized or
    /// undecodable datagrams from the paired address are dropped and counted
    /// (on Windows a datagram too large to receive is counted whatever its
    /// sender); neither can extend the deadline.  A UDP source address is spoofable,
    /// so a datagram that does not even decode carries no evidence that the
    /// paired peer sent it and must not stop the receiver.  The returned
    /// message is still unauthenticated: callers run it through the protocol
    /// validator, which stays fail-closed for authenticated-protocol faults.
    pub fn receive(&self, timeout: Duration) -> Result<ShowClockLanMessage, ShowClockLanError> {
        let deadline = Instant::now() + timeout;
        // One spare byte distinguishes a maximum-size datagram from a
        // truncated oversized one.
        let mut buffer = [0_u8; SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES + 1];
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
                // Windows discards a datagram larger than the buffer and
                // reports WSAEMSGSIZE without its source, so it is counted
                // whatever its sender: it may have been the paired peer.
                Err(error) if is_discarded_oversized_datagram(&error) => {
                    self.count_dropped_datagram();
                    continue;
                }
                // On a connectionless socket a reset is only an ICMP report,
                // which any host can forge; it is not a local socket fault.
                Err(error) if error.kind() == std::io::ErrorKind::ConnectionReset => continue,
                Err(error) => return Err(ShowClockLanError::Receive(error)),
            };
            if source != self.peer {
                continue;
            }
            match decode_datagram(&buffer[..received]) {
                Ok(message) => return Ok(message),
                Err(_) => self.count_dropped_datagram(),
            }
        }
    }

    fn count_dropped_datagram(&self) {
        let _ =
            self.dropped_datagrams
                .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |count| {
                    Some(count.saturating_add(1))
                });
    }
}

fn is_discarded_oversized_datagram(error: &std::io::Error) -> bool {
    const WSAEMSGSIZE: i32 = 10_040;
    cfg!(windows) && error.raw_os_error() == Some(WSAEMSGSIZE)
}

/// Decode one ShowClock LAN datagram without trusting its source.  This is
/// the complete envelope check (size, JSON shape, magic and protocol
/// version); authentication remains the caller's protocol-validator step.
pub fn decode_datagram(datagram: &[u8]) -> Result<ShowClockLanMessage, ShowClockLanError> {
    if datagram.len() > SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES {
        return Err(ShowClockLanError::PacketTooLarge);
    }
    let envelope = serde_json::from_slice::<ShowClockLanEnvelope>(datagram)
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
    Ok(envelope.message)
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

    fn encoded(message: ShowClockLanMessage) -> Vec<u8> {
        serde_json::to_vec(&ShowClockLanEnvelope {
            magic: SHOW_CLOCK_LAN_MAGIC.to_string(),
            protocol_version: SHOW_CLOCK_PROTOCOL_VERSION,
            message,
        })
        .unwrap()
    }

    fn hostile_datagrams() -> Vec<Vec<u8>> {
        let valid = encoded(ShowClockLanMessage::Sample(sample()));
        let text = String::from_utf8(valid.clone()).unwrap();
        vec![
            Vec::new(),
            b"{}".to_vec(),
            b"noise".to_vec(),
            vec![0xff; 64],
            valid[..valid.len() / 2].to_vec(),
            text.replace(SHOW_CLOCK_LAN_MAGIC, "syndocal.show-clocx")
                .into_bytes(),
            text.replacen(
                &format!("\"protocol_version\":{SHOW_CLOCK_PROTOCOL_VERSION}"),
                &format!(
                    "\"protocol_version\":{}",
                    SHOW_CLOCK_PROTOCOL_VERSION.wrapping_add(1)
                ),
                1,
            )
            .into_bytes(),
            vec![b' '; SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES + 1],
        ]
    }

    #[test]
    fn decode_datagram_accepts_exact_envelopes_and_rejects_hostile_bytes() {
        let valid = encoded(ShowClockLanMessage::Sample(sample()));
        assert_eq!(
            decode_datagram(&valid).unwrap(),
            ShowClockLanMessage::Sample(sample())
        );
        let mut at_limit = valid.clone();
        at_limit.resize(SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES, b' ');
        assert_eq!(
            decode_datagram(&at_limit).unwrap(),
            ShowClockLanMessage::Sample(sample())
        );
        let hostile = hostile_datagrams();
        let (oversized, malformed) = hostile.split_last().unwrap();
        assert!(matches!(
            decode_datagram(oversized),
            Err(ShowClockLanError::PacketTooLarge)
        ));
        for datagram in malformed {
            assert!(matches!(
                decode_datagram(datagram),
                Err(ShowClockLanError::Decode(_))
            ));
        }
    }

    #[test]
    fn malformed_paired_datagrams_are_dropped_without_ending_receive() {
        let receiver_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let receiver_addr = receiver_socket.local_addr().unwrap();
        let sender_socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let sender_addr = sender_socket.local_addr().unwrap();
        drop(receiver_socket);
        let receiver = ShowClockLanTransport::bind(receiver_addr, sender_addr).unwrap();
        let mut hostile = hostile_datagrams();
        // Larger than the receive buffer: truncated on Unix, discarded with
        // WSAEMSGSIZE on Windows.  Either way it is a counted drop.
        hostile.push(vec![b' '; SHOW_CLOCK_LAN_MAX_DATAGRAM_BYTES + 512]);
        for datagram in &hostile {
            sender_socket.send_to(datagram, receiver_addr).unwrap();
        }
        let valid = encoded(ShowClockLanMessage::Sample(sample()));
        sender_socket.send_to(&valid, receiver_addr).unwrap();

        assert_eq!(
            receiver.receive(Duration::from_millis(500)).unwrap(),
            ShowClockLanMessage::Sample(sample())
        );
        assert_eq!(receiver.dropped_datagrams(), hostile.len() as u64);

        // Noise alone ends only at the caller's deadline, never as a decode
        // failure that would stop the receiving worker.
        sender_socket.send_to(b"{}", receiver_addr).unwrap();
        assert!(matches!(
            receiver.receive(Duration::from_millis(50)),
            Err(ShowClockLanError::Timeout)
        ));
        assert_eq!(receiver.dropped_datagrams(), hostile.len() as u64 + 1);
    }

    #[test]
    fn bit_flipped_signed_samples_never_verify() {
        let original = ShowClockLanMessage::Sample(sample());
        let datagram = encoded(original.clone());
        let mut undecodable = 0_usize;
        let mut unauthenticated = 0_usize;
        for bit in 0..datagram.len() * 8 {
            let mut flipped = datagram.clone();
            flipped[bit / 8] ^= 1 << (bit % 8);
            match decode_datagram(&flipped) {
                Err(_) => undecodable += 1,
                Ok(message) => {
                    assert_ne!(message, original, "bit {bit} flip decoded unchanged");
                    let ShowClockLanMessage::Sample(flipped_sample) = message else {
                        panic!("bit {bit} flip changed the message kind");
                    };
                    assert!(
                        flipped_sample.verify(&KEY).is_err(),
                        "bit {bit} flip produced a sample that still verifies"
                    );
                    unauthenticated += 1;
                }
            }
        }
        assert!(undecodable > 0 && unauthenticated > 0);
        eprintln!(
            "show clock bit flips: {} bits, {undecodable} undecodable, {unauthenticated} decoded but unauthenticated",
            datagram.len() * 8
        );
    }
}
