use std::{
    collections::BTreeMap,
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    sync::atomic::{AtomicU8, Ordering},
    time::{Duration, Instant},
};

use crate::rdm::{RdmCommandClass, RdmError, RdmMessage, RdmResponseType};
use thiserror::Error;

pub const ARTNET_PORT: u16 = 6454;
pub const ART_DMX_HEADER_LEN: usize = 18;
pub const ART_RDM_HEADER_LEN: usize = 24;
const ART_DMX_PACKET_LEN: usize = 18 + 512;
const ARTNET_PROTOCOL_VERSION: u16 = 14;
const ART_RDM_OPCODE: u16 = 0x8300;
const ART_TOD_REQUEST_OPCODE: u16 = 0x8000;
const ART_TOD_DATA_OPCODE: u16 = 0x8100;
const ART_TOD_CONTROL_OPCODE: u16 = 0x8200;
const ART_TOD_REQUEST_LEN: usize = 56;
const ART_TOD_DATA_HEADER_LEN: usize = 28;

#[derive(Debug, Error)]
pub enum ArtNetError {
    #[error("failed to bind UDP socket: {0}")]
    Bind(std::io::Error),
    #[error("failed to resolve target address {0}")]
    Resolve(String),
    #[error("failed to send Art-Net packet: {0}")]
    Send(std::io::Error),
    #[error("failed to receive Art-Net packet: {0}")]
    Receive(std::io::Error),
    #[error("ArtRdm Port-Address {0} exceeds the 15-bit range")]
    InvalidPortAddress(u16),
    #[error("invalid ArtRdm packet: {0}")]
    InvalidRdmPacket(String),
    #[error("RDM packet error: {0}")]
    Rdm(#[from] RdmError),
    #[error("timed out waiting for matching ArtRdm response")]
    RdmTimeout,
    #[error("ArtRdm follow-up limit was exceeded")]
    RdmFollowupLimit,
    #[error("unexpected ArtRdm response: {0}")]
    UnexpectedRdmResponse(String),
    #[error("gateway reported that its RDM Table of Devices is unavailable")]
    TodUnavailable,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtRdmPacket {
    pub port_address: u16,
    pub fifo_available: u8,
    pub fifo_max: u8,
    pub message: RdmMessage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtRdmTransaction {
    pub packet: ArtRdmPacket,
    pub response_blocks: u16,
    pub ack_timer_count: u16,
    pub queued_message_polls: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtTodData {
    pub rdm_version: u8,
    pub physical_port: u8,
    pub bind_index: u8,
    pub port_address: u16,
    pub uid_total: u16,
    pub block_count: u8,
    pub uids: Vec<crate::rdm::RdmUid>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ArtTodControlCommand {
    None = 0x00,
    FlushAndDiscover = 0x01,
    EndDiscovery = 0x02,
    IncrementalDiscoveryOn = 0x03,
    IncrementalDiscoveryOff = 0x04,
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

    pub fn send_rdm_message(
        &self,
        port_address: u16,
        message: &RdmMessage,
    ) -> Result<usize, ArtNetError> {
        let packet = build_art_rdm_packet(port_address, message, 0, 0)?;
        self.socket
            .send_to(&packet, self.target)
            .map_err(ArtNetError::Send)
    }

    pub fn transact_rdm(
        &self,
        port_address: u16,
        request: &RdmMessage,
        timeout: Duration,
    ) -> Result<ArtRdmPacket, ArtNetError> {
        self.send_rdm_message(port_address, request)?;
        let deadline = Instant::now() + timeout;
        let mut buffer = [0_u8; 1024];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ArtNetError::RdmTimeout);
            }
            self.socket
                .set_read_timeout(Some(remaining))
                .map_err(ArtNetError::Receive)?;
            let (received, source) = match self.socket.recv_from(&mut buffer) {
                Ok(value) => value,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Err(ArtNetError::RdmTimeout);
                }
                Err(error) => return Err(ArtNetError::Receive(error)),
            };
            if source.ip() != self.target.ip() {
                continue;
            }
            let response = match parse_art_rdm_packet(&buffer[..received]) {
                Ok(response) => response,
                Err(_) => continue,
            };
            if response.port_address == port_address
                && response.message.transaction_number == request.transaction_number
                && response.message.destination == request.source
                && response.message.source == request.destination
            {
                return Ok(response);
            }
        }
    }

    pub fn transact_rdm_complete(
        &self,
        port_address: u16,
        request: &RdmMessage,
        timeout: Duration,
    ) -> Result<ArtRdmTransaction, ArtNetError> {
        const MAX_FOLLOWUPS: u16 = 64;
        let deadline = Instant::now() + timeout;
        let original_request = request.clone();
        let mut next_transaction_number = request.transaction_number;
        let mut current_request = request.clone();
        let mut overflow_data = Vec::new();
        let mut response_blocks = 0_u16;
        let mut ack_timer_count = 0_u16;
        let mut queued_message_polls = 0_u16;
        let mut waiting_for_queued_response = false;

        loop {
            if response_blocks
                .saturating_add(ack_timer_count)
                .saturating_add(queued_message_polls)
                >= MAX_FOLLOWUPS
            {
                return Err(ArtNetError::RdmFollowupLimit);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ArtNetError::RdmTimeout);
            }
            let mut packet = self.transact_rdm(port_address, &current_request, remaining)?;
            match packet.message.response_type()? {
                RdmResponseType::Ack => {
                    if waiting_for_queued_response
                        && packet.message.parameter_id != original_request.parameter_id
                    {
                        queued_message_polls = queued_message_polls.saturating_add(1);
                        std::thread::sleep(
                            Duration::from_millis(25)
                                .min(deadline.saturating_duration_since(Instant::now())),
                        );
                        current_request = queued_message_request(
                            &original_request,
                            advance_rdm_transaction_number(&mut next_transaction_number),
                        );
                        continue;
                    }
                    response_blocks = response_blocks.saturating_add(1);
                    if !overflow_data.is_empty() {
                        overflow_data.extend_from_slice(&packet.message.parameter_data);
                        packet.message.parameter_data = std::mem::take(&mut overflow_data);
                    }
                    return Ok(ArtRdmTransaction {
                        packet,
                        response_blocks,
                        ack_timer_count,
                        queued_message_polls,
                    });
                }
                RdmResponseType::AckOverflow => {
                    if packet.message.parameter_id != original_request.parameter_id {
                        return Err(ArtNetError::UnexpectedRdmResponse(format!(
                            "ACK_OVERFLOW PID 0x{:04x} does not match requested PID 0x{:04x}",
                            packet.message.parameter_id, original_request.parameter_id
                        )));
                    }
                    if original_request.command_class != RdmCommandClass::GetCommand {
                        return Err(ArtNetError::UnexpectedRdmResponse(
                            "ACK_OVERFLOW is only valid for a GET transfer".to_string(),
                        ));
                    }
                    overflow_data.extend_from_slice(&packet.message.parameter_data);
                    response_blocks = response_blocks.saturating_add(1);
                    current_request = original_request.clone();
                    current_request.transaction_number =
                        advance_rdm_transaction_number(&mut next_transaction_number);
                    waiting_for_queued_response = false;
                }
                RdmResponseType::AckTimer => {
                    let delay_tenths = packet.message.ack_timer_delay_tenths()?;
                    ack_timer_count = ack_timer_count.saturating_add(1);
                    let delay = Duration::from_millis(u64::from(delay_tenths) * 100);
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if delay >= remaining {
                        return Err(ArtNetError::RdmTimeout);
                    }
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                    current_request = queued_message_request(
                        &original_request,
                        advance_rdm_transaction_number(&mut next_transaction_number),
                    );
                    waiting_for_queued_response = true;
                }
                RdmResponseType::NackReason => {
                    let _ = packet.message.nack_reason()?;
                    response_blocks = response_blocks.saturating_add(1);
                    return Ok(ArtRdmTransaction {
                        packet,
                        response_blocks,
                        ack_timer_count,
                        queued_message_polls,
                    });
                }
            }
        }
    }

    pub fn request_tod(
        &self,
        port_address: u16,
        timeout: Duration,
    ) -> Result<Vec<crate::rdm::RdmUid>, ArtNetError> {
        let request = build_art_tod_request(port_address)?;
        self.socket
            .send_to(&request, self.target)
            .map_err(ArtNetError::Send)?;
        let deadline = Instant::now() + timeout;
        let mut blocks = BTreeMap::<u8, Vec<crate::rdm::RdmUid>>::new();
        let mut buffer = [0_u8; 2048];
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(ArtNetError::RdmTimeout);
            }
            self.socket
                .set_read_timeout(Some(remaining))
                .map_err(ArtNetError::Receive)?;
            let (received, source) = match self.socket.recv_from(&mut buffer) {
                Ok(value) => value,
                Err(error)
                    if matches!(
                        error.kind(),
                        std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                    ) =>
                {
                    return Err(ArtNetError::RdmTimeout);
                }
                Err(error) => return Err(ArtNetError::Receive(error)),
            };
            if source.ip() != self.target.ip() {
                continue;
            }
            let data = match parse_art_tod_data(&buffer[..received]) {
                Ok(data) => data,
                Err(ArtNetError::TodUnavailable) => return Err(ArtNetError::TodUnavailable),
                Err(_) => continue,
            };
            if data.port_address != port_address {
                continue;
            }
            let total = data.uid_total as usize;
            blocks.insert(data.block_count, data.uids);
            if total == 0 {
                return Ok(Vec::new());
            }
            let contiguous = blocks
                .keys()
                .copied()
                .enumerate()
                .all(|(index, block)| block as usize == index);
            let received_total = blocks.values().map(Vec::len).sum::<usize>();
            if contiguous && received_total >= total {
                return Ok(blocks.into_values().flatten().take(total).collect());
            }
        }
    }

    pub fn send_tod_control(
        &self,
        port_address: u16,
        command: ArtTodControlCommand,
    ) -> Result<usize, ArtNetError> {
        let packet = build_art_tod_control(port_address, command)?;
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

fn advance_rdm_transaction_number(current: &mut u8) -> u8 {
    *current = if *current == u8::MAX {
        1
    } else {
        current.saturating_add(1).max(1)
    };
    *current
}

fn queued_message_request(original: &RdmMessage, transaction_number: u8) -> RdmMessage {
    RdmMessage {
        destination: original.destination,
        source: original.source,
        transaction_number,
        port_id_or_response_type: original.port_id_or_response_type,
        message_count: 0,
        sub_device: original.sub_device,
        command_class: RdmCommandClass::GetCommand,
        parameter_id: crate::rdm::pid::QUEUED_MESSAGE,
        parameter_data: vec![crate::rdm::status_type::ERROR],
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

pub fn build_art_rdm_packet(
    port_address: u16,
    message: &RdmMessage,
    fifo_available: u8,
    fifo_max: u8,
) -> Result<Vec<u8>, ArtNetError> {
    if port_address > 0x7fff {
        return Err(ArtNetError::InvalidPortAddress(port_address));
    }
    let rdm = message.encode()?;
    let mut packet = Vec::with_capacity(ART_RDM_HEADER_LEN + rdm.len().saturating_sub(1));
    packet.extend_from_slice(b"Art-Net\0");
    packet.extend_from_slice(&ART_RDM_OPCODE.to_le_bytes());
    packet.extend_from_slice(&ARTNET_PROTOCOL_VERSION.to_be_bytes());
    packet.push(0x01);
    packet.push(0x00);
    packet.extend_from_slice(&[0; 5]);
    packet.push(fifo_available);
    packet.push(fifo_max);
    packet.push((port_address >> 8) as u8);
    packet.push(0x00);
    packet.push(port_address as u8);
    packet.extend_from_slice(&rdm[1..]);
    Ok(packet)
}

pub fn parse_art_rdm_packet(packet: &[u8]) -> Result<ArtRdmPacket, ArtNetError> {
    if packet.len() < ART_RDM_HEADER_LEN + crate::rdm::RDM_MIN_MESSAGE_LENGTH + 1 {
        return Err(ArtNetError::InvalidRdmPacket(
            "packet is too short".to_string(),
        ));
    }
    if &packet[0..8] != b"Art-Net\0" {
        return Err(ArtNetError::InvalidRdmPacket(
            "Art-Net ID mismatch".to_string(),
        ));
    }
    if u16::from_le_bytes([packet[8], packet[9]]) != ART_RDM_OPCODE {
        return Err(ArtNetError::InvalidRdmPacket(
            "opcode is not OpRdm".to_string(),
        ));
    }
    if u16::from_be_bytes([packet[10], packet[11]]) < ARTNET_PROTOCOL_VERSION {
        return Err(ArtNetError::InvalidRdmPacket(
            "protocol version is older than Art-Net 4".to_string(),
        ));
    }
    if packet[12] != 0x01 || packet[22] != 0x00 || packet[21] & 0x80 != 0 {
        return Err(ArtNetError::InvalidRdmPacket(
            "unsupported RDM version, command, or Port-Address".to_string(),
        ));
    }
    let mut rdm = Vec::with_capacity(packet.len() - ART_RDM_HEADER_LEN + 1);
    rdm.push(crate::rdm::RDM_START_CODE);
    rdm.extend_from_slice(&packet[ART_RDM_HEADER_LEN..]);
    Ok(ArtRdmPacket {
        port_address: ((packet[21] as u16) << 8) | packet[23] as u16,
        fifo_available: packet[19],
        fifo_max: packet[20],
        message: RdmMessage::decode(&rdm)?,
    })
}

pub fn build_art_tod_request(port_address: u16) -> Result<[u8; ART_TOD_REQUEST_LEN], ArtNetError> {
    if port_address > 0x7fff {
        return Err(ArtNetError::InvalidPortAddress(port_address));
    }
    let mut packet = [0_u8; ART_TOD_REQUEST_LEN];
    packet[..8].copy_from_slice(b"Art-Net\0");
    packet[8..10].copy_from_slice(&ART_TOD_REQUEST_OPCODE.to_le_bytes());
    packet[10..12].copy_from_slice(&ARTNET_PROTOCOL_VERSION.to_be_bytes());
    packet[21] = (port_address >> 8) as u8;
    packet[22] = 0x00;
    packet[23] = 1;
    packet[24] = port_address as u8;
    Ok(packet)
}

pub fn build_art_tod_control(
    port_address: u16,
    command: ArtTodControlCommand,
) -> Result<[u8; ART_RDM_HEADER_LEN], ArtNetError> {
    if port_address > 0x7fff {
        return Err(ArtNetError::InvalidPortAddress(port_address));
    }
    let mut packet = [0_u8; ART_RDM_HEADER_LEN];
    packet[..8].copy_from_slice(b"Art-Net\0");
    packet[8..10].copy_from_slice(&ART_TOD_CONTROL_OPCODE.to_le_bytes());
    packet[10..12].copy_from_slice(&ARTNET_PROTOCOL_VERSION.to_be_bytes());
    packet[21] = (port_address >> 8) as u8;
    packet[22] = command as u8;
    packet[23] = port_address as u8;
    Ok(packet)
}

pub fn parse_art_tod_data(packet: &[u8]) -> Result<ArtTodData, ArtNetError> {
    if packet.len() < ART_TOD_DATA_HEADER_LEN {
        return Err(ArtNetError::InvalidRdmPacket(
            "ArtTodData is too short".to_string(),
        ));
    }
    if &packet[..8] != b"Art-Net\0"
        || u16::from_le_bytes([packet[8], packet[9]]) != ART_TOD_DATA_OPCODE
        || u16::from_be_bytes([packet[10], packet[11]]) < ARTNET_PROTOCOL_VERSION
    {
        return Err(ArtNetError::InvalidRdmPacket(
            "ArtTodData header is invalid".to_string(),
        ));
    }
    if packet[22] == 0xff {
        return Err(ArtNetError::TodUnavailable);
    }
    if packet[22] != 0x00 || packet[21] & 0x80 != 0 {
        return Err(ArtNetError::InvalidRdmPacket(
            "ArtTodData command or Port-Address is invalid".to_string(),
        ));
    }
    let uid_count = packet[27] as usize;
    let expected_len = ART_TOD_DATA_HEADER_LEN + uid_count * 6;
    if packet.len() != expected_len {
        return Err(ArtNetError::InvalidRdmPacket(
            "ArtTodData UID count does not match packet length".to_string(),
        ));
    }
    let uids = packet[ART_TOD_DATA_HEADER_LEN..]
        .chunks_exact(6)
        .map(|bytes| {
            let mut uid = [0_u8; 6];
            uid.copy_from_slice(bytes);
            crate::rdm::RdmUid(uid)
        })
        .collect();
    Ok(ArtTodData {
        rdm_version: packet[12],
        physical_port: packet[13],
        bind_index: packet[20],
        port_address: ((packet[21] as u16) << 8) | packet[23] as u16,
        uid_total: u16::from_be_bytes([packet[24], packet[25]]),
        block_count: packet[26],
        uids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdm::{pid, RdmCommandClass, RdmUid};
    use std::net::UdpSocket;

    fn rdm_request() -> RdmMessage {
        RdmMessage {
            destination: RdmUid::new(0x1234, 0x0102_0304),
            source: RdmUid::new(0x7fff, 0x1122_3344),
            transaction_number: 41,
            port_id_or_response_type: 1,
            message_count: 0,
            sub_device: 0,
            command_class: RdmCommandClass::GetCommand,
            parameter_id: pid::DEVICE_INFO,
            parameter_data: Vec::new(),
        }
    }

    fn tod_data_packet(
        port_address: u16,
        uid_total: u16,
        block_count: u8,
        uids: &[RdmUid],
    ) -> Vec<u8> {
        let mut packet = vec![0_u8; ART_TOD_DATA_HEADER_LEN + uids.len() * 6];
        packet[..8].copy_from_slice(b"Art-Net\0");
        packet[8..10].copy_from_slice(&ART_TOD_DATA_OPCODE.to_le_bytes());
        packet[10..12].copy_from_slice(&ARTNET_PROTOCOL_VERSION.to_be_bytes());
        packet[12] = 1;
        packet[13] = 1;
        packet[20] = 1;
        packet[21] = (port_address >> 8) as u8;
        packet[22] = 0;
        packet[23] = port_address as u8;
        packet[24..26].copy_from_slice(&uid_total.to_be_bytes());
        packet[26] = block_count;
        packet[27] = uids.len() as u8;
        for (index, uid) in uids.iter().enumerate() {
            let offset = ART_TOD_DATA_HEADER_LEN + index * 6;
            packet[offset..offset + 6].copy_from_slice(&uid.0);
        }
        packet
    }

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

    #[test]
    fn art_rdm_codec_preserves_port_fifo_and_standard_rdm_message() {
        let request = rdm_request();

        let packet = build_art_rdm_packet(0x1234, &request, 6, 8).unwrap();
        let parsed = parse_art_rdm_packet(&packet).unwrap();

        assert_eq!(&packet[..8], b"Art-Net\0");
        assert_eq!(u16::from_le_bytes([packet[8], packet[9]]), ART_RDM_OPCODE);
        assert_eq!(packet[12], 1);
        assert_eq!(packet[ART_RDM_HEADER_LEN], crate::rdm::RDM_SUB_START_CODE);
        assert_eq!(parsed.port_address, 0x1234);
        assert_eq!(parsed.fifo_available, 6);
        assert_eq!(parsed.fifo_max, 8);
        assert_eq!(parsed.message, request);
        assert!(matches!(
            build_art_rdm_packet(0x8000, &request, 0, 0),
            Err(ArtNetError::InvalidPortAddress(0x8000))
        ));
    }

    #[test]
    fn art_rdm_parser_rejects_header_and_rdm_checksum_corruption() {
        let request = rdm_request();
        let mut packet = build_art_rdm_packet(1, &request, 0, 0).unwrap();
        packet[9] = 0x50;
        assert!(parse_art_rdm_packet(&packet).is_err());

        packet = build_art_rdm_packet(1, &request, 0, 0).unwrap();
        let last = packet.len() - 1;
        packet[last] ^= 0x01;
        assert!(matches!(
            parse_art_rdm_packet(&packet),
            Err(ArtNetError::Rdm(RdmError::ChecksumMismatch { .. }))
        ));
    }

    #[test]
    fn art_rdm_unicast_transaction_matches_response_identity() {
        let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let gateway_port = gateway.local_addr().unwrap().port();
        let gateway_thread = std::thread::spawn(move || {
            let mut buffer = [0_u8; 1024];
            let (received, source) = gateway.recv_from(&mut buffer).unwrap();
            let request = parse_art_rdm_packet(&buffer[..received]).unwrap();
            let response = RdmMessage {
                destination: request.message.source,
                source: request.message.destination,
                transaction_number: request.message.transaction_number,
                port_id_or_response_type: 0,
                message_count: 0,
                sub_device: request.message.sub_device,
                command_class: RdmCommandClass::GetCommandResponse,
                parameter_id: request.message.parameter_id,
                parameter_data: vec![1, 2, 3],
            };
            let packet = build_art_rdm_packet(request.port_address, &response, 4, 8).unwrap();
            gateway.send_to(&packet, source).unwrap();
        });
        let sender = ArtNetSender::new("127.0.0.1", gateway_port).unwrap();
        let request = rdm_request();

        let response = sender
            .transact_rdm(9, &request, Duration::from_secs(1))
            .unwrap();

        gateway_thread.join().unwrap();
        assert_eq!(response.port_address, 9);
        assert_eq!(response.fifo_available, 4);
        assert_eq!(
            response.message.command_class,
            RdmCommandClass::GetCommandResponse
        );
        assert_eq!(response.message.parameter_data, vec![1, 2, 3]);
    }

    #[test]
    fn art_rdm_complete_transaction_collects_ack_overflow_blocks() {
        let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let gateway_port = gateway.local_addr().unwrap().port();
        let gateway_thread = std::thread::spawn(move || {
            let mut buffer = [0_u8; 1024];
            for (index, (response_type, data)) in [
                (RdmResponseType::AckOverflow, vec![1, 2]),
                (RdmResponseType::Ack, vec![3, 4]),
            ]
            .into_iter()
            .enumerate()
            {
                let (received, source) = gateway.recv_from(&mut buffer).unwrap();
                let request = parse_art_rdm_packet(&buffer[..received]).unwrap();
                assert_eq!(request.message.parameter_id, crate::rdm::pid::DEVICE_INFO);
                if index == 1 {
                    assert_ne!(request.message.transaction_number, 7);
                }
                let response = RdmMessage {
                    destination: request.message.source,
                    source: request.message.destination,
                    transaction_number: request.message.transaction_number,
                    port_id_or_response_type: response_type as u8,
                    message_count: 0,
                    sub_device: request.message.sub_device,
                    command_class: RdmCommandClass::GetCommandResponse,
                    parameter_id: request.message.parameter_id,
                    parameter_data: data,
                };
                let packet = build_art_rdm_packet(request.port_address, &response, 0, 8).unwrap();
                gateway.send_to(&packet, source).unwrap();
            }
        });
        let sender = ArtNetSender::new("127.0.0.1", gateway_port).unwrap();

        let result = sender
            .transact_rdm_complete(9, &rdm_request(), Duration::from_secs(1))
            .unwrap();

        gateway_thread.join().unwrap();
        assert_eq!(result.packet.message.parameter_data, vec![1, 2, 3, 4]);
        assert_eq!(result.response_blocks, 2);
        assert_eq!(result.ack_timer_count, 0);
    }

    #[test]
    fn art_rdm_complete_transaction_resolves_ack_timer_through_queued_message() {
        let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let gateway_port = gateway.local_addr().unwrap().port();
        let gateway_thread = std::thread::spawn(move || {
            let mut buffer = [0_u8; 1024];
            let (received, source) = gateway.recv_from(&mut buffer).unwrap();
            let request = parse_art_rdm_packet(&buffer[..received]).unwrap();
            let timer = RdmMessage {
                destination: request.message.source,
                source: request.message.destination,
                transaction_number: request.message.transaction_number,
                port_id_or_response_type: RdmResponseType::AckTimer as u8,
                message_count: 0,
                sub_device: request.message.sub_device,
                command_class: RdmCommandClass::GetCommandResponse,
                parameter_id: request.message.parameter_id,
                parameter_data: vec![0, 0],
            };
            gateway
                .send_to(
                    &build_art_rdm_packet(request.port_address, &timer, 0, 8).unwrap(),
                    source,
                )
                .unwrap();

            let (received, source) = gateway.recv_from(&mut buffer).unwrap();
            let queued = parse_art_rdm_packet(&buffer[..received]).unwrap();
            assert_eq!(queued.message.parameter_id, crate::rdm::pid::QUEUED_MESSAGE);
            assert_eq!(
                queued.message.parameter_data,
                vec![crate::rdm::status_type::ERROR]
            );
            let response = RdmMessage {
                destination: queued.message.source,
                source: queued.message.destination,
                transaction_number: queued.message.transaction_number,
                port_id_or_response_type: RdmResponseType::Ack as u8,
                message_count: 0,
                sub_device: queued.message.sub_device,
                command_class: RdmCommandClass::GetCommandResponse,
                parameter_id: crate::rdm::pid::DEVICE_INFO,
                parameter_data: vec![9, 8, 7],
            };
            gateway
                .send_to(
                    &build_art_rdm_packet(queued.port_address, &response, 0, 8).unwrap(),
                    source,
                )
                .unwrap();
        });
        let sender = ArtNetSender::new("127.0.0.1", gateway_port).unwrap();

        let result = sender
            .transact_rdm_complete(9, &rdm_request(), Duration::from_secs(1))
            .unwrap();

        gateway_thread.join().unwrap();
        assert_eq!(result.packet.message.parameter_data, vec![9, 8, 7]);
        assert_eq!(result.ack_timer_count, 1);
        assert_eq!(result.response_blocks, 1);
    }

    #[test]
    fn art_tod_request_and_data_codec_preserve_port_and_uids() {
        let request = build_art_tod_request(0x2345).unwrap();
        assert_eq!(request.len(), ART_TOD_REQUEST_LEN);
        assert_eq!(
            u16::from_le_bytes([request[8], request[9]]),
            ART_TOD_REQUEST_OPCODE
        );
        assert_eq!(request[21], 0x23);
        assert_eq!(request[23], 1);
        assert_eq!(request[24], 0x45);
        assert!(request[25..].iter().all(|byte| *byte == 0));
        let control =
            build_art_tod_control(0x2345, ArtTodControlCommand::FlushAndDiscover).unwrap();
        assert_eq!(
            u16::from_le_bytes([control[8], control[9]]),
            ART_TOD_CONTROL_OPCODE
        );
        assert_eq!(control[21], 0x23);
        assert_eq!(control[22], 0x01);
        assert_eq!(control[23], 0x45);

        let uids = [RdmUid::new(0x1111, 1), RdmUid::new(0x2222, 2)];
        let packet = tod_data_packet(0x2345, 2, 0, &uids);
        let data = parse_art_tod_data(&packet).unwrap();
        assert_eq!(data.port_address, 0x2345);
        assert_eq!(data.uid_total, 2);
        assert_eq!(data.uids, uids);

        let mut malformed = packet.clone();
        malformed[27] = 3;
        assert!(parse_art_tod_data(&malformed).is_err());
    }

    #[test]
    fn art_tod_request_collects_out_of_order_blocks_from_gateway() {
        let gateway = UdpSocket::bind("127.0.0.1:0").unwrap();
        gateway
            .set_read_timeout(Some(Duration::from_secs(1)))
            .unwrap();
        let gateway_port = gateway.local_addr().unwrap().port();
        let expected = vec![
            RdmUid::new(0x1111, 1),
            RdmUid::new(0x2222, 2),
            RdmUid::new(0x3333, 3),
        ];
        let gateway_expected = expected.clone();
        let gateway_thread = std::thread::spawn(move || {
            let mut request = [0_u8; 128];
            let (received, source) = gateway.recv_from(&mut request).unwrap();
            assert_eq!(received, ART_TOD_REQUEST_LEN);
            assert_eq!(request[21], 0);
            assert_eq!(request[24], 9);
            let block_one = tod_data_packet(9, 3, 1, &gateway_expected[2..]);
            let block_zero = tod_data_packet(9, 3, 0, &gateway_expected[..2]);
            gateway.send_to(&block_one, source).unwrap();
            gateway.send_to(&block_zero, source).unwrap();
        });
        let sender = ArtNetSender::new("127.0.0.1", gateway_port).unwrap();

        let discovered = sender.request_tod(9, Duration::from_secs(1)).unwrap();

        gateway_thread.join().unwrap();
        assert_eq!(discovered, expected);
    }
}
