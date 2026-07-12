use std::{
    collections::BTreeSet,
    io::{self, Read, Write},
    time::{Duration, Instant},
};

use serialport::SerialPort;
use thiserror::Error;

use crate::rdm::{pid, status_type, RdmCommandClass, RdmError, RdmMessage, RdmResponseType};

pub const ENTTEC_PRO_START_DELIMITER: u8 = 0x7e;
pub const ENTTEC_PRO_END_DELIMITER: u8 = 0xe7;
pub const ENTTEC_PRO_RECEIVED_DMX_LABEL: u8 = 0x05;
pub const ENTTEC_PRO_SEND_RDM_LABEL: u8 = 0x07;
pub const ENTTEC_PRO_SEND_RDM_DISCOVERY_LABEL: u8 = 0x0b;
pub const ENTTEC_PRO_MAX_PAYLOAD_LENGTH: usize = 600;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnttecRdmTransaction {
    pub message: RdmMessage,
    pub response_blocks: u16,
    pub ack_timer_count: u16,
    pub queued_message_polls: u16,
}

#[derive(Debug, Error)]
pub enum SerialRdmError {
    #[error("serial port path is required")]
    MissingPort,
    #[error("failed to open serial RDM port {path}: {source}")]
    Open {
        path: String,
        #[source]
        source: serialport::Error,
    },
    #[error("serial RDM I/O failed: {0}")]
    Io(#[source] io::Error),
    #[error("ENTTEC USB Pro payload exceeds {ENTTEC_PRO_MAX_PAYLOAD_LENGTH} bytes")]
    PayloadTooLong,
    #[error("ENTTEC USB Pro message has an invalid end delimiter")]
    InvalidEndDelimiter,
    #[error("ENTTEC USB Pro reported receive status 0x{0:02x}")]
    ReceiveStatus(u8),
    #[error("RDM transaction timed out")]
    Timeout,
    #[error("RDM response follow-up limit was exceeded")]
    FollowupLimit,
    #[error("RDM discovery probe limit was exceeded")]
    DiscoveryLimit,
    #[error("unexpected RDM response: {0}")]
    UnexpectedResponse(String),
    #[error(transparent)]
    Rdm(#[from] RdmError),
}

pub struct EnttecUsbProRdmController {
    port: Box<dyn SerialPort>,
}

impl EnttecUsbProRdmController {
    pub fn new(path: &str, baud_rate: u32) -> Result<Self, SerialRdmError> {
        if path.trim().is_empty() {
            return Err(SerialRdmError::MissingPort);
        }
        let port = serialport::new(path, baud_rate.max(1))
            .timeout(Duration::from_millis(20))
            .open()
            .map_err(|source| SerialRdmError::Open {
                path: path.to_string(),
                source,
            })?;
        Ok(Self { port })
    }

    pub fn transact_rdm(
        &mut self,
        request: &RdmMessage,
        timeout: Duration,
    ) -> Result<RdmMessage, SerialRdmError> {
        let rdm_packet = request.encode()?;
        write_enttec_message(&mut self.port, ENTTEC_PRO_SEND_RDM_LABEL, &rdm_packet)?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SerialRdmError::Timeout);
            }
            self.port
                .set_timeout(remaining.min(Duration::from_millis(100)))
                .map_err(|error| SerialRdmError::Io(serial_error_to_io(error)))?;
            let (label, payload) = match read_enttec_message(&mut self.port) {
                Ok(message) => message,
                Err(SerialRdmError::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if label != ENTTEC_PRO_RECEIVED_DMX_LABEL || payload.len() < 2 {
                continue;
            }
            if payload[0] != 0 {
                return Err(SerialRdmError::ReceiveStatus(payload[0]));
            }
            if payload[1] != crate::rdm::RDM_START_CODE {
                continue;
            }
            let response = match RdmMessage::decode(&payload[1..]) {
                Ok(response) => response,
                Err(_) => continue,
            };
            if response.transaction_number == request.transaction_number
                && response.destination == request.source
                && response.source == request.destination
            {
                return Ok(response);
            }
        }
    }

    pub fn transact_rdm_complete(
        &mut self,
        request: &RdmMessage,
        timeout: Duration,
    ) -> Result<EnttecRdmTransaction, SerialRdmError> {
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
                return Err(SerialRdmError::FollowupLimit);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SerialRdmError::Timeout);
            }
            let mut message = self.transact_rdm(&current_request, remaining)?;
            match message.response_type()? {
                RdmResponseType::Ack => {
                    if waiting_for_queued_response
                        && message.parameter_id != original_request.parameter_id
                    {
                        queued_message_polls = queued_message_polls.saturating_add(1);
                        std::thread::sleep(
                            Duration::from_millis(25)
                                .min(deadline.saturating_duration_since(Instant::now())),
                        );
                        current_request = queued_message_request(
                            &original_request,
                            advance_transaction_number(&mut next_transaction_number),
                        );
                        continue;
                    }
                    response_blocks = response_blocks.saturating_add(1);
                    if !overflow_data.is_empty() {
                        overflow_data.extend_from_slice(&message.parameter_data);
                        message.parameter_data = std::mem::take(&mut overflow_data);
                    }
                    return Ok(EnttecRdmTransaction {
                        message,
                        response_blocks,
                        ack_timer_count,
                        queued_message_polls,
                    });
                }
                RdmResponseType::AckOverflow => {
                    if message.parameter_id != original_request.parameter_id {
                        return Err(SerialRdmError::UnexpectedResponse(format!(
                            "ACK_OVERFLOW PID 0x{:04x} does not match requested PID 0x{:04x}",
                            message.parameter_id, original_request.parameter_id
                        )));
                    }
                    if original_request.command_class != RdmCommandClass::GetCommand {
                        return Err(SerialRdmError::UnexpectedResponse(
                            "ACK_OVERFLOW is only valid for a GET transfer".to_string(),
                        ));
                    }
                    overflow_data.extend_from_slice(&message.parameter_data);
                    response_blocks = response_blocks.saturating_add(1);
                    current_request = original_request.clone();
                    current_request.transaction_number =
                        advance_transaction_number(&mut next_transaction_number);
                    waiting_for_queued_response = false;
                }
                RdmResponseType::AckTimer => {
                    let delay_tenths = message.ack_timer_delay_tenths()?;
                    ack_timer_count = ack_timer_count.saturating_add(1);
                    let delay = Duration::from_millis(u64::from(delay_tenths) * 100);
                    let remaining = deadline.saturating_duration_since(Instant::now());
                    if delay >= remaining {
                        return Err(SerialRdmError::Timeout);
                    }
                    if !delay.is_zero() {
                        std::thread::sleep(delay);
                    }
                    current_request = queued_message_request(
                        &original_request,
                        advance_transaction_number(&mut next_transaction_number),
                    );
                    waiting_for_queued_response = true;
                }
                RdmResponseType::NackReason => {
                    let _ = message.nack_reason()?;
                    response_blocks = response_blocks.saturating_add(1);
                    return Ok(EnttecRdmTransaction {
                        message,
                        response_blocks,
                        ack_timer_count,
                        queued_message_polls,
                    });
                }
            }
        }
    }

    pub fn discover_devices(
        &mut self,
        controller_uid: crate::rdm::RdmUid,
        timeout: Duration,
    ) -> Result<Vec<crate::rdm::RdmUid>, SerialRdmError> {
        const MAX_DISCOVERY_PROBES: usize = 16_384;
        let deadline = Instant::now() + timeout;
        let mut transaction_number = 1_u8;
        let unmute = discovery_request(
            crate::rdm::RdmUid::BROADCAST,
            controller_uid,
            transaction_number,
            pid::DISC_UN_MUTE,
            Vec::new(),
        );
        self.send_rdm_without_response(&unmute)?;
        std::thread::sleep(Duration::from_millis(20));

        let mut found = BTreeSet::new();
        let mut stack = vec![(0_u64, 0xffff_ffff_ffff_u64)];
        let mut probes = 0_usize;
        while let Some((lower, upper)) = stack.pop() {
            probes += 1;
            if probes > MAX_DISCOVERY_PROBES {
                return Err(SerialRdmError::DiscoveryLimit);
            }
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Err(SerialRdmError::Timeout);
            }
            transaction_number = advance_transaction_number(&mut transaction_number);
            let request = discovery_request(
                crate::rdm::RdmUid::BROADCAST,
                controller_uid,
                transaction_number,
                pid::DISC_UNIQUE_BRANCH,
                [uid_from_u64(lower).0, uid_from_u64(upper).0].concat(),
            );
            match self
                .probe_discovery_branch(&request, remaining.min(Duration::from_millis(120)))?
            {
                DiscoveryProbe::Empty => {}
                DiscoveryProbe::Collision if lower < upper => {
                    let midpoint = lower + (upper - lower) / 2;
                    stack.push((midpoint + 1, upper));
                    stack.push((lower, midpoint));
                }
                DiscoveryProbe::Collision => {}
                DiscoveryProbe::Unique(uid) => {
                    if found.insert(uid.0) {
                        transaction_number = advance_transaction_number(&mut transaction_number);
                        let mute = discovery_request(
                            uid,
                            controller_uid,
                            transaction_number,
                            pid::DISC_MUTE,
                            Vec::new(),
                        );
                        self.transact_rdm(
                            &mute,
                            deadline
                                .saturating_duration_since(Instant::now())
                                .min(Duration::from_millis(500)),
                        )?;
                        stack.push((lower, upper));
                    }
                }
            }
        }
        Ok(found.into_iter().map(crate::rdm::RdmUid).collect())
    }

    fn send_rdm_without_response(&mut self, request: &RdmMessage) -> Result<(), SerialRdmError> {
        let packet = request.encode()?;
        write_enttec_message(&mut self.port, ENTTEC_PRO_SEND_RDM_LABEL, &packet)?;
        Ok(())
    }

    fn probe_discovery_branch(
        &mut self,
        request: &RdmMessage,
        timeout: Duration,
    ) -> Result<DiscoveryProbe, SerialRdmError> {
        let packet = request.encode()?;
        write_enttec_message(&mut self.port, ENTTEC_PRO_SEND_RDM_DISCOVERY_LABEL, &packet)?;
        let deadline = Instant::now() + timeout;
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            if remaining.is_zero() {
                return Ok(DiscoveryProbe::Empty);
            }
            self.port
                .set_timeout(remaining.min(Duration::from_millis(50)))
                .map_err(|error| SerialRdmError::Io(serial_error_to_io(error)))?;
            let (label, payload) = match read_enttec_message(&mut self.port) {
                Ok(message) => message,
                Err(SerialRdmError::Io(error))
                    if matches!(
                        error.kind(),
                        io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock
                    ) =>
                {
                    continue;
                }
                Err(error) => return Err(error),
            };
            if label != ENTTEC_PRO_RECEIVED_DMX_LABEL || payload.len() < 2 {
                continue;
            }
            if payload[0] != 0 {
                return Ok(DiscoveryProbe::Collision);
            }
            let response = &payload[1..];
            if !response.iter().any(|byte| *byte == 0xaa) {
                continue;
            }
            return Ok(match crate::rdm::decode_discovery_response(response) {
                Ok(uid) => DiscoveryProbe::Unique(uid),
                Err(_) => DiscoveryProbe::Collision,
            });
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DiscoveryProbe {
    Empty,
    Collision,
    Unique(crate::rdm::RdmUid),
}

pub fn build_enttec_message(label: u8, payload: &[u8]) -> Result<Vec<u8>, SerialRdmError> {
    if payload.len() > ENTTEC_PRO_MAX_PAYLOAD_LENGTH {
        return Err(SerialRdmError::PayloadTooLong);
    }
    let length = payload.len() as u16;
    let mut packet = Vec::with_capacity(payload.len() + 5);
    packet.push(ENTTEC_PRO_START_DELIMITER);
    packet.push(label);
    packet.extend_from_slice(&length.to_le_bytes());
    packet.extend_from_slice(payload);
    packet.push(ENTTEC_PRO_END_DELIMITER);
    Ok(packet)
}

pub fn write_enttec_message<W: Write + ?Sized>(
    writer: &mut W,
    label: u8,
    payload: &[u8],
) -> Result<usize, SerialRdmError> {
    let packet = build_enttec_message(label, payload)?;
    writer.write_all(&packet).map_err(SerialRdmError::Io)?;
    writer.flush().map_err(SerialRdmError::Io)?;
    Ok(packet.len())
}

pub fn read_enttec_message<R: Read + ?Sized>(
    reader: &mut R,
) -> Result<(u8, Vec<u8>), SerialRdmError> {
    let mut byte = [0_u8; 1];
    loop {
        reader.read_exact(&mut byte).map_err(SerialRdmError::Io)?;
        if byte[0] == ENTTEC_PRO_START_DELIMITER {
            break;
        }
    }
    let mut header = [0_u8; 3];
    reader.read_exact(&mut header).map_err(SerialRdmError::Io)?;
    let length = u16::from_le_bytes([header[1], header[2]]) as usize;
    if length > ENTTEC_PRO_MAX_PAYLOAD_LENGTH {
        return Err(SerialRdmError::PayloadTooLong);
    }
    let mut payload = vec![0_u8; length];
    reader
        .read_exact(&mut payload)
        .map_err(SerialRdmError::Io)?;
    reader.read_exact(&mut byte).map_err(SerialRdmError::Io)?;
    if byte[0] != ENTTEC_PRO_END_DELIMITER {
        return Err(SerialRdmError::InvalidEndDelimiter);
    }
    Ok((header[0], payload))
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
        parameter_id: pid::QUEUED_MESSAGE,
        parameter_data: vec![status_type::ERROR],
    }
}

fn discovery_request(
    destination: crate::rdm::RdmUid,
    source: crate::rdm::RdmUid,
    transaction_number: u8,
    parameter_id: u16,
    parameter_data: Vec<u8>,
) -> RdmMessage {
    RdmMessage {
        destination,
        source,
        transaction_number,
        port_id_or_response_type: 1,
        message_count: 0,
        sub_device: 0,
        command_class: RdmCommandClass::DiscoveryCommand,
        parameter_id,
        parameter_data,
    }
}

fn uid_from_u64(value: u64) -> crate::rdm::RdmUid {
    let bytes = value.to_be_bytes();
    crate::rdm::RdmUid([bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7]])
}

fn advance_transaction_number(transaction_number: &mut u8) -> u8 {
    *transaction_number = if *transaction_number == u8::MAX {
        1
    } else {
        *transaction_number + 1
    };
    *transaction_number
}

fn serial_error_to_io(error: serialport::Error) -> io::Error {
    io::Error::new(io::ErrorKind::Other, error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rdm::{RdmUid, RDM_START_CODE};
    use std::io::Cursor;

    #[test]
    fn enttec_message_codec_preserves_label_length_and_payload() {
        let packet = build_enttec_message(ENTTEC_PRO_SEND_RDM_LABEL, &[0xcc, 1, 2]).unwrap();
        assert_eq!(packet, vec![0x7e, 0x07, 3, 0, 0xcc, 1, 2, 0xe7]);
        let (label, payload) = read_enttec_message(&mut Cursor::new(packet)).unwrap();
        assert_eq!(label, ENTTEC_PRO_SEND_RDM_LABEL);
        assert_eq!(payload, vec![0xcc, 1, 2]);
    }

    #[test]
    fn enttec_received_dmx_payload_contains_status_then_rdm_packet() {
        let response = RdmMessage {
            destination: RdmUid::new(0x7fff, 1),
            source: RdmUid::new(0x1234, 2),
            transaction_number: 7,
            port_id_or_response_type: RdmResponseType::Ack as u8,
            message_count: 0,
            sub_device: 0,
            command_class: RdmCommandClass::GetCommandResponse,
            parameter_id: pid::DEVICE_LABEL,
            parameter_data: b"Fixture".to_vec(),
        };
        let mut payload = vec![0];
        payload.extend_from_slice(&response.encode().unwrap());
        let packet = build_enttec_message(ENTTEC_PRO_RECEIVED_DMX_LABEL, &payload).unwrap();
        let (label, parsed_payload) = read_enttec_message(&mut Cursor::new(packet)).unwrap();
        assert_eq!(label, ENTTEC_PRO_RECEIVED_DMX_LABEL);
        assert_eq!(parsed_payload[1], RDM_START_CODE);
        assert_eq!(RdmMessage::decode(&parsed_payload[1..]).unwrap(), response);
    }

    #[test]
    fn enttec_message_reader_rejects_oversize_and_bad_end_delimiter() {
        let mut oversized = vec![0x7e, 7, 0x59, 0x02];
        assert!(matches!(
            read_enttec_message(&mut Cursor::new(&mut oversized)),
            Err(SerialRdmError::PayloadTooLong)
        ));
        let bad_end = vec![0x7e, 7, 1, 0, 0xcc, 0x00];
        assert!(matches!(
            read_enttec_message(&mut Cursor::new(bad_end)),
            Err(SerialRdmError::InvalidEndDelimiter)
        ));
    }

    #[test]
    fn discovery_unique_branch_packet_uses_full_48_bit_range_and_label_11() {
        let request = discovery_request(
            RdmUid::BROADCAST,
            RdmUid::new(0x7fff, 1),
            9,
            pid::DISC_UNIQUE_BRANCH,
            [
                uid_from_u64(0x0102_0304_0506).0,
                uid_from_u64(0x1112_1314_1516).0,
            ]
            .concat(),
        );
        let encoded = request.encode().unwrap();
        let widget = build_enttec_message(ENTTEC_PRO_SEND_RDM_DISCOVERY_LABEL, &encoded).unwrap();
        assert_eq!(encoded.len(), 38);
        assert_eq!(widget[1], 11);
        assert_eq!(&request.parameter_data[..6], &[1, 2, 3, 4, 5, 6]);
        assert_eq!(
            &request.parameter_data[6..],
            &[0x11, 0x12, 0x13, 0x14, 0x15, 0x16]
        );
    }
}
