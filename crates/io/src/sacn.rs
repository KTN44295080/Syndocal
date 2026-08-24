use std::{
    net::{Ipv4Addr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::atomic::{AtomicU8, Ordering},
    time::Duration,
};

use thiserror::Error;

pub const SACN_PORT: u16 = 5568;
pub const SACN_PACKET_LEN: usize = 126 + 512;
const ACN_PACKET_IDENTIFIER: &[u8; 12] = b"ASC-E1.17\0\0\0";
const SOURCE_NAME_LEN: usize = 64;
// Stable Syndocal sACN component identifier: 49ce4498-aa14-41f4-a889-7afa6b7b10f1.
const SYNDOCAL_CID: [u8; 16] = [
    0x49, 0xce, 0x44, 0x98, 0xaa, 0x14, 0x41, 0xf4, 0xa8, 0x89, 0x7a, 0xfa, 0x6b, 0x7b, 0x10, 0xf1,
];

#[derive(Debug, Error)]
pub enum SacnError {
    #[error("failed to bind UDP socket: {0}")]
    Bind(std::io::Error),
    #[error("failed to resolve target address {0}")]
    Resolve(String),
    #[error("failed to send sACN packet: {0}")]
    Send(std::io::Error),
}

pub struct SacnSender {
    socket: UdpSocket,
    target: Option<SocketAddr>,
    port: u16,
    cid: [u8; 16],
    source_name: String,
    sequence: AtomicU8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SacnDmxPacket<'a> {
    pub universe: u16,
    pub sequence: u8,
    pub priority: u8,
    pub data: &'a [u8],
}

impl SacnSender {
    pub fn new(target_ip: &str, port: u16) -> Result<Self, SacnError> {
        let socket = UdpSocket::bind("0.0.0.0:0").map_err(SacnError::Bind)?;
        let _ = socket.set_write_timeout(Some(Duration::from_millis(2)));
        let _ = socket.set_multicast_ttl_v4(20);
        let target = if is_sacn_multicast_target(target_ip) {
            None
        } else {
            Some(
                (target_ip, port)
                    .to_socket_addrs()
                    .map_err(|_| SacnError::Resolve(format!("{target_ip}:{port}")))?
                    .next()
                    .ok_or_else(|| SacnError::Resolve(format!("{target_ip}:{port}")))?,
            )
        };
        Ok(Self {
            socket,
            target,
            port,
            cid: SYNDOCAL_CID,
            source_name: "Syndocal".to_string(),
            sequence: AtomicU8::new(1),
        })
    }

    pub fn send_dmx_frame(&self, universe: u16, frame: &[u8; 512]) -> Result<usize, SacnError> {
        let sequence = self.next_sequence();
        let packet = build_sacn_dmx_packet(universe, sequence, &self.cid, &self.source_name, frame);
        let target = self
            .target
            .unwrap_or_else(|| sacn_multicast_addr(universe, self.port));
        self.socket
            .send_to(&packet, target)
            .map_err(SacnError::Send)
    }

    fn next_sequence(&self) -> u8 {
        self.sequence
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |current| {
                Some(if current == u8::MAX { 1 } else { current + 1 })
            })
            .unwrap_or(1)
    }
}

pub fn is_sacn_multicast_target(target_ip: &str) -> bool {
    let target = target_ip.trim();
    target.is_empty()
        || target.eq_ignore_ascii_case("multicast")
        || target.eq_ignore_ascii_case("auto")
}

pub fn sacn_multicast_addr(universe: u16, port: u16) -> SocketAddr {
    SocketAddr::from((
        Ipv4Addr::new(239, 255, (universe >> 8) as u8, (universe & 0xff) as u8),
        port,
    ))
}

pub fn build_sacn_dmx_packet(
    universe: u16,
    sequence: u8,
    cid: &[u8; 16],
    source_name: &str,
    frame: &[u8; 512],
) -> [u8; SACN_PACKET_LEN] {
    let mut packet = [0u8; SACN_PACKET_LEN];

    packet[0..2].copy_from_slice(&0x0010u16.to_be_bytes());
    packet[2..4].copy_from_slice(&0x0000u16.to_be_bytes());
    packet[4..16].copy_from_slice(ACN_PACKET_IDENTIFIER);

    write_flags_and_length(&mut packet, 16, SACN_PACKET_LEN - 16);
    packet[18..22].copy_from_slice(&0x0000_0004u32.to_be_bytes());
    packet[22..38].copy_from_slice(cid);

    write_flags_and_length(&mut packet, 38, SACN_PACKET_LEN - 38);
    packet[40..44].copy_from_slice(&0x0000_0002u32.to_be_bytes());
    write_source_name(&mut packet[44..108], source_name);
    packet[108] = 100;
    packet[109..111].copy_from_slice(&0u16.to_be_bytes());
    packet[111] = sequence;
    packet[112] = 0;
    packet[113..115].copy_from_slice(&universe.to_be_bytes());

    write_flags_and_length(&mut packet, 115, SACN_PACKET_LEN - 115);
    packet[117] = 0x02;
    packet[118] = 0xa1;
    packet[119..121].copy_from_slice(&0u16.to_be_bytes());
    packet[121..123].copy_from_slice(&1u16.to_be_bytes());
    packet[123..125].copy_from_slice(&513u16.to_be_bytes());
    packet[125] = 0;
    packet[126..].copy_from_slice(frame);

    packet
}

pub fn parse_sacn_dmx_packet(packet: &[u8]) -> Option<SacnDmxPacket<'_>> {
    if packet.len() < SACN_PACKET_LEN || &packet[4..16] != ACN_PACKET_IDENTIFIER {
        return None;
    }
    if u32::from_be_bytes(packet[18..22].try_into().ok()?) != 0x0000_0004 {
        return None;
    }
    if u32::from_be_bytes(packet[40..44].try_into().ok()?) != 0x0000_0002 {
        return None;
    }
    if packet[117] != 0x02 || packet[118] != 0xa1 {
        return None;
    }
    let property_count = u16::from_be_bytes(packet[123..125].try_into().ok()?) as usize;
    if property_count == 0 {
        return None;
    }
    let data_start = 126usize;
    let data_len = property_count - 1;
    let data_end = data_start.checked_add(data_len)?;
    if data_end > packet.len() {
        return None;
    }
    Some(SacnDmxPacket {
        universe: u16::from_be_bytes(packet[113..115].try_into().ok()?),
        sequence: packet[111],
        priority: packet[108],
        data: &packet[data_start..data_end],
    })
}

fn write_flags_and_length(packet: &mut [u8], offset: usize, length: usize) {
    let value = 0x7000u16 | (length as u16 & 0x0fff);
    packet[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
}

fn write_source_name(target: &mut [u8], source_name: &str) {
    debug_assert_eq!(target.len(), SOURCE_NAME_LEN);
    let bytes = source_name.as_bytes();
    let len = bytes.len().min(SOURCE_NAME_LEN - 1);
    target[..len].copy_from_slice(&bytes[..len]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::UdpSocket;

    #[test]
    fn builds_sacn_dmx_packet() {
        let mut frame = [0u8; 512];
        frame[0] = 255;
        frame[511] = 64;

        let packet = build_sacn_dmx_packet(12, 7, &SYNDOCAL_CID, "Syndocal Test", &frame);
        let parsed = parse_sacn_dmx_packet(&packet).unwrap();

        assert_eq!(packet.len(), SACN_PACKET_LEN);
        assert_eq!(&packet[4..16], ACN_PACKET_IDENTIFIER);
        assert_eq!(parsed.universe, 12);
        assert_eq!(parsed.sequence, 7);
        assert_eq!(parsed.priority, 100);
        assert_eq!(parsed.data.len(), 512);
        assert_eq!(parsed.data[0], 255);
        assert_eq!(parsed.data[511], 64);
    }

    #[test]
    fn syndocal_cid_uses_dedicated_identifier() {
        assert_eq!(
            SYNDOCAL_CID,
            [
                0x49, 0xce, 0x44, 0x98, 0xaa, 0x14, 0x41, 0xf4, 0xa8, 0x89, 0x7a, 0xfa, 0x6b, 0x7b,
                0x10, 0xf1,
            ]
        );
        assert_ne!(&SYNDOCAL_CID[0..4], [75, 68, 77, 88]);
    }

    #[test]
    fn derives_standard_sacn_multicast_address_from_universe() {
        assert_eq!(
            sacn_multicast_addr(1, SACN_PORT),
            "239.255.0.1:5568".parse::<SocketAddr>().unwrap()
        );
        assert_eq!(
            sacn_multicast_addr(63999, SACN_PORT),
            "239.255.249.255:5568".parse::<SocketAddr>().unwrap()
        );
        assert!(is_sacn_multicast_target("multicast"));
        assert!(is_sacn_multicast_target(" auto "));
        assert!(is_sacn_multicast_target(""));
        assert!(!is_sacn_multicast_target("127.0.0.1"));
    }

    #[test]
    fn sends_sacn_dmx_to_udp_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let sender = SacnSender::new("127.0.0.1", port).unwrap();
        let mut frame = [0u8; 512];
        frame[0] = 199;

        let sent = sender.send_dmx_frame(7, &frame).unwrap();
        let mut buffer = [0u8; 700];
        let (received, _) = receiver.recv_from(&mut buffer).unwrap();
        let parsed = parse_sacn_dmx_packet(&buffer[..received]).unwrap();

        assert_eq!(sent, SACN_PACKET_LEN);
        assert_eq!(parsed.universe, 7);
        assert_eq!(parsed.data[0], 199);
    }
}
