use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::atomic::{AtomicU8, Ordering},
    time::Duration,
};

use thiserror::Error;

pub const ARTNET_PORT: u16 = 6454;
pub const ART_DMX_HEADER_LEN: usize = 18;
const ART_DMX_PACKET_LEN: usize = 18 + 512;
const ARTNET_PROTOCOL_VERSION: u16 = 14;

#[derive(Debug, Error)]
pub enum ArtNetError {
    #[error("failed to bind UDP socket: {0}")]
    Bind(std::io::Error),
    #[error("failed to resolve target address {0}")]
    Resolve(String),
    #[error("failed to send Art-Net packet: {0}")]
    Send(std::io::Error),
}

pub struct ArtNetSender {
    socket: UdpSocket,
    target: SocketAddr,
    sequence: AtomicU8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtDmxPacket<'a> {
    pub universe: u16,
    pub sequence: u8,
    pub physical: u8,
    pub data: &'a [u8],
}

impl ArtNetSender {
    pub fn new(target_ip: &str, port: u16) -> Result<Self, ArtNetError> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(ArtNetError::Bind)?;
        let _ = socket.set_write_timeout(Some(Duration::from_millis(2)));
        let target = (target_ip, port)
            .to_socket_addrs()
            .map_err(|_| ArtNetError::Resolve(format!("{target_ip}:{port}")))?
            .next()
            .ok_or_else(|| ArtNetError::Resolve(format!("{target_ip}:{port}")))?;
        Ok(Self {
            socket,
            target,
            sequence: AtomicU8::new(1),
        })
    }

    pub fn send_dmx_frame(&self, universe: u16, frame: &[u8; 512]) -> Result<usize, ArtNetError> {
        let packet = build_art_dmx_packet_with_sequence(universe, self.next_sequence(), frame);
        self.socket
            .send_to(&packet, self.target)
            .map_err(ArtNetError::Send)
    }

    fn next_sequence(&self) -> u8 {
        self.sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(if current >= 255 { 1 } else { current + 1 })
            })
            .unwrap_or(1)
    }
}

pub fn build_art_dmx_packet(universe: u16, frame: &[u8; 512]) -> [u8; ART_DMX_PACKET_LEN] {
    build_art_dmx_packet_with_sequence(universe, 0, frame)
}

pub fn build_art_dmx_packet_with_sequence(
    universe: u16,
    sequence: u8,
    frame: &[u8; 512],
) -> [u8; ART_DMX_PACKET_LEN] {
    let mut packet = [0u8; ART_DMX_PACKET_LEN];
    packet[0..8].copy_from_slice(b"Art-Net\0");
    packet[8] = 0x00;
    packet[9] = 0x50;
    packet[10] = 0x00;
    packet[11] = 0x0e;
    packet[12] = sequence;
    packet[13] = 0x00;
    packet[14] = (universe & 0xff) as u8;
    packet[15] = (universe >> 8) as u8;
    packet[16] = 0x02;
    packet[17] = 0x00;
    packet[18..].copy_from_slice(frame);
    packet
}

pub fn parse_art_dmx_packet(packet: &[u8]) -> Option<ArtDmxPacket<'_>> {
    if packet.len() < ART_DMX_HEADER_LEN || &packet[0..8] != b"Art-Net\0" {
        return None;
    }

    let opcode = u16::from_le_bytes([packet[8], packet[9]]);
    if opcode != 0x5000 {
        return None;
    }

    let protocol_version = u16::from_be_bytes([packet[10], packet[11]]);
    if protocol_version < ARTNET_PROTOCOL_VERSION {
        return None;
    }

    let length = u16::from_be_bytes([packet[16], packet[17]]) as usize;
    if !(2..=512).contains(&length) || length % 2 != 0 {
        return None;
    }
    let end = ART_DMX_HEADER_LEN.checked_add(length)?;
    if packet.len() < end {
        return None;
    }

    Some(ArtDmxPacket {
        universe: u16::from_le_bytes([packet[14], packet[15]]),
        sequence: packet[12],
        physical: packet[13],
        data: &packet[ART_DMX_HEADER_LEN..end],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    #[test]
    fn builds_art_dmx_packet() {
        let mut frame = [0u8; 512];
        frame[0] = 255;
        frame[511] = 127;

        let packet = build_art_dmx_packet(3, &frame);

        assert_eq!(&packet[0..8], b"Art-Net\0");
        assert_eq!(packet[8], 0x00);
        assert_eq!(packet[9], 0x50);
        assert_eq!(packet[14], 3);
        assert_eq!(packet[16], 0x02);
        assert_eq!(packet[17], 0x00);
        assert_eq!(packet[18], 255);
        assert_eq!(packet[529], 127);
    }

    #[test]
    fn builds_art_dmx_packet_with_sequence() {
        let frame = [0u8; 512];

        let packet = build_art_dmx_packet_with_sequence(3, 42, &frame);
        let parsed = parse_art_dmx_packet(&packet).unwrap();

        assert_eq!(parsed.universe, 3);
        assert_eq!(parsed.sequence, 42);
    }

    #[test]
    fn parses_art_dmx_packet() {
        let mut frame = [0u8; 512];
        frame[4] = 123;

        let packet = build_art_dmx_packet(42, &frame);
        let parsed = parse_art_dmx_packet(&packet).unwrap();

        assert_eq!(parsed.universe, 42);
        assert_eq!(parsed.data.len(), 512);
        assert_eq!(parsed.data[4], 123);
    }

    #[test]
    fn rejects_invalid_art_dmx_packet_versions_and_lengths() {
        let frame = [0u8; 512];
        let mut packet = build_art_dmx_packet(1, &frame);

        packet[10] = 0x00;
        packet[11] = 0x0d;
        assert!(parse_art_dmx_packet(&packet).is_none());

        packet = build_art_dmx_packet(1, &frame);
        packet[16] = 0x00;
        packet[17] = 0x01;
        assert!(parse_art_dmx_packet(&packet).is_none());

        packet = build_art_dmx_packet(1, &frame);
        packet[16] = 0x00;
        packet[17] = 0x03;
        assert!(parse_art_dmx_packet(&packet).is_none());

        packet = build_art_dmx_packet(1, &frame);
        packet[16] = 0x02;
        packet[17] = 0x02;
        let mut oversized_packet = packet.to_vec();
        oversized_packet.extend_from_slice(&[0, 0]);
        assert!(parse_art_dmx_packet(&oversized_packet).is_none());
    }

    #[test]
    fn sends_art_dmx_to_udp_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let sender = ArtNetSender::new("127.0.0.1", port).unwrap();
        let mut frame = [0u8; 512];
        frame[0] = 199;

        let sent = sender.send_dmx_frame(7, &frame).unwrap();
        let mut buffer = [0u8; 600];
        let (received, _) = receiver.recv_from(&mut buffer).unwrap();
        let parsed = parse_art_dmx_packet(&buffer[..received]).unwrap();

        assert_eq!(sent, ART_DMX_PACKET_LEN);
        assert_eq!(parsed.universe, 7);
        assert_eq!(parsed.sequence, 1);
        assert_eq!(parsed.data[0], 199);
    }

    #[test]
    fn sender_sequence_increments_and_wraps_without_zero() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = receiver.local_addr().unwrap().port();
        let sender = ArtNetSender::new("127.0.0.1", port).unwrap();

        assert_eq!(sender.next_sequence(), 1);
        assert_eq!(sender.next_sequence(), 2);
        sender.sequence.store(255, Ordering::Relaxed);
        assert_eq!(sender.next_sequence(), 255);
        assert_eq!(sender.next_sequence(), 1);
    }
}
