use thiserror::Error;

pub const RDM_START_CODE: u8 = 0xcc;
pub const RDM_SUB_START_CODE: u8 = 0x01;
pub const RDM_MIN_MESSAGE_LENGTH: usize = 24;
pub const RDM_MAX_PARAMETER_DATA_LENGTH: usize = 231;

pub mod pid {
    pub const DISC_UNIQUE_BRANCH: u16 = 0x0001;
    pub const DISC_MUTE: u16 = 0x0002;
    pub const DISC_UN_MUTE: u16 = 0x0003;
    pub const QUEUED_MESSAGE: u16 = 0x0020;
    pub const STATUS_MESSAGES: u16 = 0x0030;
    pub const DEVICE_INFO: u16 = 0x0060;
    pub const DEVICE_MODEL_DESCRIPTION: u16 = 0x0080;
    pub const MANUFACTURER_LABEL: u16 = 0x0081;
    pub const DEVICE_LABEL: u16 = 0x0082;
    pub const SOFTWARE_VERSION_LABEL: u16 = 0x00c0;
    pub const DMX_PERSONALITY: u16 = 0x00e0;
    pub const DMX_START_ADDRESS: u16 = 0x00f0;
    pub const IDENTIFY_DEVICE: u16 = 0x1000;
}

pub mod status_type {
    pub const GET_LAST_MESSAGE: u8 = 0x01;
    pub const ADVISORY: u8 = 0x02;
    pub const WARNING: u8 = 0x03;
    pub const ERROR: u8 = 0x04;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RdmUid(pub [u8; 6]);

impl RdmUid {
    pub const BROADCAST: Self = Self([0xff; 6]);

    pub const fn new(manufacturer_id: u16, device_id: u32) -> Self {
        let manufacturer = manufacturer_id.to_be_bytes();
        let device = device_id.to_be_bytes();
        Self([
            manufacturer[0],
            manufacturer[1],
            device[0],
            device[1],
            device[2],
            device[3],
        ])
    }

    pub fn manufacturer_id(self) -> u16 {
        u16::from_be_bytes([self.0[0], self.0[1]])
    }

    pub fn device_id(self) -> u32 {
        u32::from_be_bytes([self.0[2], self.0[3], self.0[4], self.0[5]])
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RdmCommandClass {
    DiscoveryCommand = 0x10,
    DiscoveryCommandResponse = 0x11,
    GetCommand = 0x20,
    GetCommandResponse = 0x21,
    SetCommand = 0x30,
    SetCommandResponse = 0x31,
}

impl TryFrom<u8> for RdmCommandClass {
    type Error = RdmError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x10 => Ok(Self::DiscoveryCommand),
            0x11 => Ok(Self::DiscoveryCommandResponse),
            0x20 => Ok(Self::GetCommand),
            0x21 => Ok(Self::GetCommandResponse),
            0x30 => Ok(Self::SetCommand),
            0x31 => Ok(Self::SetCommandResponse),
            _ => Err(RdmError::UnsupportedCommandClass(value)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RdmResponseType {
    Ack = 0x00,
    AckTimer = 0x01,
    NackReason = 0x02,
    AckOverflow = 0x03,
}

impl TryFrom<u8> for RdmResponseType {
    type Error = RdmError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x00 => Ok(Self::Ack),
            0x01 => Ok(Self::AckTimer),
            0x02 => Ok(Self::NackReason),
            0x03 => Ok(Self::AckOverflow),
            _ => Err(RdmError::UnsupportedResponseType(value)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdmMessage {
    pub destination: RdmUid,
    pub source: RdmUid,
    pub transaction_number: u8,
    pub port_id_or_response_type: u8,
    pub message_count: u8,
    pub sub_device: u16,
    pub command_class: RdmCommandClass,
    pub parameter_id: u16,
    pub parameter_data: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RdmDeviceInfo {
    pub protocol_version: u16,
    pub model_id: u16,
    pub product_category: u16,
    pub software_version_id: u32,
    pub dmx_footprint: u16,
    pub current_personality: u8,
    pub personality_count: u8,
    pub dmx_start_address: u16,
    pub sub_device_count: u16,
    pub sensor_count: u8,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RdmError {
    #[error("RDM parameter data exceeds {RDM_MAX_PARAMETER_DATA_LENGTH} bytes")]
    ParameterDataTooLong,
    #[error("RDM packet is shorter than the minimum packet length")]
    PacketTooShort,
    #[error("invalid RDM start code 0x{0:02x}")]
    InvalidStartCode(u8),
    #[error("invalid RDM sub-start code 0x{0:02x}")]
    InvalidSubStartCode(u8),
    #[error("RDM message length does not match packet length")]
    InvalidMessageLength,
    #[error("RDM parameter data length does not match message length")]
    InvalidParameterDataLength,
    #[error("unsupported RDM command class 0x{0:02x}")]
    UnsupportedCommandClass(u8),
    #[error("unsupported RDM response type 0x{0:02x}")]
    UnsupportedResponseType(u8),
    #[error("RDM ACK_TIMER response must contain exactly two bytes")]
    InvalidAckTimerLength,
    #[error("RDM NACK_REASON response must contain exactly two bytes")]
    InvalidNackReasonLength,
    #[error("RDM checksum mismatch: expected 0x{expected:04x}, received 0x{received:04x}")]
    ChecksumMismatch { expected: u16, received: u16 },
    #[error("RDM discovery response is malformed")]
    InvalidDiscoveryResponse,
    #[error("RDM DEVICE_INFO response must contain exactly 19 bytes")]
    InvalidDeviceInfoLength,
}

impl RdmMessage {
    pub fn response_type(&self) -> Result<RdmResponseType, RdmError> {
        self.port_id_or_response_type.try_into()
    }

    pub fn ack_timer_delay_tenths(&self) -> Result<u16, RdmError> {
        if self.response_type()? != RdmResponseType::AckTimer || self.parameter_data.len() != 2 {
            return Err(RdmError::InvalidAckTimerLength);
        }
        Ok(u16::from_be_bytes([
            self.parameter_data[0],
            self.parameter_data[1],
        ]))
    }

    pub fn nack_reason(&self) -> Result<u16, RdmError> {
        if self.response_type()? != RdmResponseType::NackReason || self.parameter_data.len() != 2 {
            return Err(RdmError::InvalidNackReasonLength);
        }
        Ok(u16::from_be_bytes([
            self.parameter_data[0],
            self.parameter_data[1],
        ]))
    }

    pub fn encode(&self) -> Result<Vec<u8>, RdmError> {
        if self.parameter_data.len() > RDM_MAX_PARAMETER_DATA_LENGTH {
            return Err(RdmError::ParameterDataTooLong);
        }
        let message_length = RDM_MIN_MESSAGE_LENGTH + self.parameter_data.len();
        let mut packet = Vec::with_capacity(message_length + 2);
        packet.extend_from_slice(&[RDM_START_CODE, RDM_SUB_START_CODE, message_length as u8]);
        packet.extend_from_slice(&self.destination.0);
        packet.extend_from_slice(&self.source.0);
        packet.push(self.transaction_number);
        packet.push(self.port_id_or_response_type);
        packet.push(self.message_count);
        packet.extend_from_slice(&self.sub_device.to_be_bytes());
        packet.push(self.command_class as u8);
        packet.extend_from_slice(&self.parameter_id.to_be_bytes());
        packet.push(self.parameter_data.len() as u8);
        packet.extend_from_slice(&self.parameter_data);
        packet.extend_from_slice(&rdm_checksum(&packet).to_be_bytes());
        Ok(packet)
    }

    pub fn decode(packet: &[u8]) -> Result<Self, RdmError> {
        if packet.len() < RDM_MIN_MESSAGE_LENGTH + 2 {
            return Err(RdmError::PacketTooShort);
        }
        if packet[0] != RDM_START_CODE {
            return Err(RdmError::InvalidStartCode(packet[0]));
        }
        if packet[1] != RDM_SUB_START_CODE {
            return Err(RdmError::InvalidSubStartCode(packet[1]));
        }
        let message_length = packet[2] as usize;
        if message_length < RDM_MIN_MESSAGE_LENGTH || packet.len() != message_length + 2 {
            return Err(RdmError::InvalidMessageLength);
        }
        let parameter_data_length = packet[23] as usize;
        if parameter_data_length > RDM_MAX_PARAMETER_DATA_LENGTH
            || message_length != RDM_MIN_MESSAGE_LENGTH + parameter_data_length
        {
            return Err(RdmError::InvalidParameterDataLength);
        }
        let expected = rdm_checksum(&packet[..message_length]);
        let received = u16::from_be_bytes([packet[message_length], packet[message_length + 1]]);
        if expected != received {
            return Err(RdmError::ChecksumMismatch { expected, received });
        }
        let mut destination = [0_u8; 6];
        destination.copy_from_slice(&packet[3..9]);
        let mut source = [0_u8; 6];
        source.copy_from_slice(&packet[9..15]);
        Ok(Self {
            destination: RdmUid(destination),
            source: RdmUid(source),
            transaction_number: packet[15],
            port_id_or_response_type: packet[16],
            message_count: packet[17],
            sub_device: u16::from_be_bytes([packet[18], packet[19]]),
            command_class: packet[20].try_into()?,
            parameter_id: u16::from_be_bytes([packet[21], packet[22]]),
            parameter_data: packet[24..message_length].to_vec(),
        })
    }
}

impl RdmDeviceInfo {
    pub fn decode(parameter_data: &[u8]) -> Result<Self, RdmError> {
        if parameter_data.len() != 19 {
            return Err(RdmError::InvalidDeviceInfoLength);
        }
        Ok(Self {
            protocol_version: u16::from_be_bytes([parameter_data[0], parameter_data[1]]),
            model_id: u16::from_be_bytes([parameter_data[2], parameter_data[3]]),
            product_category: u16::from_be_bytes([parameter_data[4], parameter_data[5]]),
            software_version_id: u32::from_be_bytes([
                parameter_data[6],
                parameter_data[7],
                parameter_data[8],
                parameter_data[9],
            ]),
            dmx_footprint: u16::from_be_bytes([parameter_data[10], parameter_data[11]]),
            current_personality: parameter_data[12],
            personality_count: parameter_data[13],
            dmx_start_address: u16::from_be_bytes([parameter_data[14], parameter_data[15]]),
            sub_device_count: u16::from_be_bytes([parameter_data[16], parameter_data[17]]),
            sensor_count: parameter_data[18],
        })
    }
}

pub fn rdm_checksum(packet_without_checksum: &[u8]) -> u16 {
    packet_without_checksum
        .iter()
        .fold(0_u16, |sum, byte| sum.wrapping_add(*byte as u16))
}

pub fn decode_discovery_response(response: &[u8]) -> Result<RdmUid, RdmError> {
    let separator = response
        .iter()
        .position(|byte| *byte == 0xaa)
        .ok_or(RdmError::InvalidDiscoveryResponse)?;
    let encoded = response
        .get(separator + 1..separator + 17)
        .ok_or(RdmError::InvalidDiscoveryResponse)?;
    let mut decoded = [0_u8; 8];
    for (index, pair) in encoded.chunks_exact(2).enumerate() {
        decoded[index] = pair[0] & pair[1];
    }
    let expected_checksum = encoded[..12]
        .iter()
        .fold(0_u16, |sum, byte| sum.wrapping_add(*byte as u16));
    let received_checksum = u16::from_be_bytes([decoded[6], decoded[7]]);
    if expected_checksum != received_checksum {
        return Err(RdmError::ChecksumMismatch {
            expected: expected_checksum,
            received: received_checksum,
        });
    }
    let mut uid = [0_u8; 6];
    uid.copy_from_slice(&decoded[..6]);
    Ok(RdmUid(uid))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_message() -> RdmMessage {
        RdmMessage {
            destination: RdmUid::new(0x1234, 0x5678_9abc),
            source: RdmUid::new(0xcba9, 0x8765_4321),
            transaction_number: 7,
            port_id_or_response_type: 1,
            message_count: 0,
            sub_device: 0,
            command_class: RdmCommandClass::GetCommand,
            parameter_id: pid::DEVICE_INFO,
            parameter_data: Vec::new(),
        }
    }

    #[test]
    fn rdm_message_round_trips_with_big_endian_fields_and_checksum() {
        let message = sample_message();
        let packet = message.encode().unwrap();

        assert_eq!(packet.len(), 26);
        assert_eq!(packet[0], RDM_START_CODE);
        assert_eq!(packet[2], 24);
        assert_eq!(&packet[3..9], &[0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc]);
        assert_eq!(RdmMessage::decode(&packet).unwrap(), message);
    }

    #[test]
    fn rdm_message_rejects_length_checksum_and_command_corruption() {
        let packet = sample_message().encode().unwrap();
        let mut bad_length = packet.clone();
        bad_length[2] = 25;
        assert_eq!(
            RdmMessage::decode(&bad_length),
            Err(RdmError::InvalidMessageLength)
        );

        let mut bad_checksum = packet.clone();
        bad_checksum[15] ^= 1;
        assert!(matches!(
            RdmMessage::decode(&bad_checksum),
            Err(RdmError::ChecksumMismatch { .. })
        ));

        let mut bad_command = packet;
        bad_command[20] = 0xff;
        let checksum = rdm_checksum(&bad_command[..24]).to_be_bytes();
        bad_command[24..].copy_from_slice(&checksum);
        assert_eq!(
            RdmMessage::decode(&bad_command),
            Err(RdmError::UnsupportedCommandClass(0xff))
        );
    }

    #[test]
    fn parses_device_info_parameter_data() {
        let data = [
            0x01, 0x00, 0x12, 0x34, 0x01, 0x02, 0x01, 0x23, 0x45, 0x67, 0x00, 0x20, 0x02, 0x04,
            0x00, 0x65, 0x00, 0x03, 0x05,
        ];
        let info = RdmDeviceInfo::decode(&data).unwrap();
        assert_eq!(info.protocol_version, 0x0100);
        assert_eq!(info.software_version_id, 0x0123_4567);
        assert_eq!(info.dmx_footprint, 32);
        assert_eq!(info.dmx_start_address, 101);
        assert_eq!(info.sensor_count, 5);
    }

    #[test]
    fn parses_response_types_ack_timer_and_nack_reason() {
        let mut response = sample_message();
        response.command_class = RdmCommandClass::GetCommandResponse;
        response.port_id_or_response_type = RdmResponseType::AckTimer as u8;
        response.parameter_data = vec![0x00, 0x19];
        assert_eq!(response.response_type().unwrap(), RdmResponseType::AckTimer);
        assert_eq!(response.ack_timer_delay_tenths().unwrap(), 25);

        response.port_id_or_response_type = RdmResponseType::NackReason as u8;
        response.parameter_data = vec![0x00, 0x05];
        assert_eq!(response.nack_reason().unwrap(), 5);

        response.parameter_data.pop();
        assert_eq!(
            response.nack_reason(),
            Err(RdmError::InvalidNackReasonLength)
        );
        response.port_id_or_response_type = 0xff;
        assert_eq!(
            response.response_type(),
            Err(RdmError::UnsupportedResponseType(0xff))
        );
    }

    #[test]
    fn decodes_collision_safe_discovery_response() {
        let uid = RdmUid::new(0x1234, 0x5678_9abc);
        let mut encoded_uid = Vec::new();
        for byte in uid.0 {
            encoded_uid.push(byte | 0xaa);
            encoded_uid.push(byte | 0x55);
        }
        let checksum = encoded_uid
            .iter()
            .fold(0_u16, |sum, byte| sum.wrapping_add(*byte as u16));
        let mut response = vec![0xfe; 7];
        response.push(0xaa);
        response.extend_from_slice(&encoded_uid);
        for byte in checksum.to_be_bytes() {
            response.push(byte | 0xaa);
            response.push(byte | 0x55);
        }

        assert_eq!(decode_discovery_response(&response).unwrap(), uid);
        response[10] ^= 1;
        assert!(decode_discovery_response(&response).is_err());
    }
}
