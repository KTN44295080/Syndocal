//! Seeded, deterministic hostile-input corpora for the network ingress
//! parsers: Art-Net, sACN, RDM, OSC and the Web Remote request parser.
//! Everything is in-memory; no socket, device or file is opened.  The
//! generator mirrors the project migration corpus: one fixed seed xor a
//! per-target constant, fixed edge cases first, `catch_unwind` per case and a
//! summary line carrying the seed and accepted/rejected counts.

use std::panic::{catch_unwind, AssertUnwindSafe};

use protocol::{OscControlMapping, VideoParam};
use rosc::{
    encoder, OscArray, OscBundle, OscColor, OscMessage, OscMidiMessage, OscPacket, OscTime, OscType,
};
use serde_json::{json, Value};

use crate::{
    artnet::{
        build_art_dmx_packet_with_sequence, build_art_rdm_packet, parse_art_dmx_packet,
        parse_art_rdm_packet, parse_art_tod_data, ArtNetError, ART_DMX_HEADER_LEN,
    },
    osc::{events_from_packet_with_mappings, learned_control_from_packet},
    rdm::{
        decode_discovery_response, RdmCommandClass, RdmMessage, RdmUid,
        RDM_MAX_PARAMETER_DATA_LENGTH,
    },
    remote_ws::{
        remote_parse_error_response_json, request_from_text, RemoteParseError, RemoteWireOperation,
    },
    sacn::{build_sacn_dmx_packet, parse_sacn_dmx_packet, SACN_PACKET_LEN},
};

const GENERATOR_SEED: u64 = 0x5344_435f_2026_0913;
/// Largest datagram the Art-Net/sACN/OSC receive loops hand to a parser.
const DATAGRAM_MAX_BYTES: usize = 1536;
const DMX_MAX_SLOTS: usize = 512;

struct CorpusRng(u64);

impl CorpusRng {
    fn new(target: u64) -> Self {
        Self(GENERATOR_SEED ^ target)
    }

    fn next_u64(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        self.0
    }

    fn below(&mut self, bound: usize) -> usize {
        ((self.next_u64() >> 32) % bound as u64) as usize
    }

    fn byte(&mut self) -> u8 {
        (self.next_u64() >> 56) as u8
    }

    fn bytes(&mut self, length: usize) -> Vec<u8> {
        (0..length).map(|_| self.byte()).collect()
    }

    fn chance(&mut self, percent: usize) -> bool {
        self.below(100) < percent
    }

    fn pick<'a, T>(&mut self, values: &'a [T]) -> &'a T {
        &values[self.below(values.len())]
    }
}

/// Structure-aware mutation of a valid seed: bit flips, boundary bytes,
/// truncation, trailing junk, boundary length fields or pure noise.
fn mutate(rng: &mut CorpusRng, seed: &[u8], max_len: usize) -> Vec<u8> {
    let mut bytes = seed.to_vec();
    match rng.below(6) {
        0 => {
            for _ in 0..1 + rng.below(8) {
                if bytes.is_empty() {
                    break;
                }
                let index = rng.below(bytes.len());
                bytes[index] ^= 1 << rng.below(8);
            }
        }
        1 if !bytes.is_empty() => {
            let index = rng.below(bytes.len());
            bytes[index] = *rng.pick(&[0x00, 0x01, 0x7f, 0x80, 0xaa, 0xcc, 0xdd, 0xfe, 0xff]);
        }
        2 => {
            let length = rng.below(bytes.len() + 1);
            bytes.truncate(length);
        }
        3 => {
            let extra = 1 + rng.below(64);
            let tail = rng.bytes(extra);
            bytes.extend(tail);
        }
        4 if bytes.len() >= 2 => {
            let index = rng.below(bytes.len() - 1);
            let value = *rng.pick(&[
                0_u16, 1, 2, 3, 24, 25, 231, 511, 512, 513, 514, 0x7fff, 0xffff,
            ]);
            let encoded = if rng.chance(50) {
                value.to_be_bytes()
            } else {
                value.to_le_bytes()
            };
            bytes[index..index + 2].copy_from_slice(&encoded);
        }
        _ => {
            let length = rng.below(max_len + 1);
            bytes = rng.bytes(length);
        }
    }
    bytes.truncate(max_len);
    bytes
}

fn slice_within(slice: &[u8], input: &[u8]) -> bool {
    let start = input.as_ptr() as usize;
    let slice_start = slice.as_ptr() as usize;
    slice_start >= start && slice_start + slice.len() <= start + input.len()
}

struct Tally {
    target: &'static str,
    seed: u64,
    cases: usize,
    accepted: usize,
    rejected: usize,
}

impl Tally {
    fn new(target: &'static str, seed: u64) -> Self {
        Self {
            target,
            seed,
            cases: 0,
            accepted: 0,
            rejected: 0,
        }
    }

    fn record(&mut self, accepted: bool) {
        self.cases += 1;
        if accepted {
            self.accepted += 1;
        } else {
            self.rejected += 1;
        }
    }

    fn finish(self) {
        assert!(
            self.accepted > 0,
            "{} corpus must exercise accepted input",
            self.target
        );
        assert!(
            self.rejected > 0,
            "{} corpus must exercise rejection",
            self.target
        );
        eprintln!(
            "{} hostile corpus: seed={GENERATOR_SEED:#x}^{:#x}, {} cases, {} accepted, {} rejected",
            self.target, self.seed, self.cases, self.accepted, self.rejected
        );
    }
}

fn random_frame(rng: &mut CorpusRng) -> [u8; DMX_MAX_SLOTS] {
    let mut frame = [0_u8; DMX_MAX_SLOTS];
    for slot in &mut frame {
        *slot = rng.byte();
    }
    frame
}

/// Independent statement of the ArtDmx acceptance rule.
fn art_dmx_expected_length(packet: &[u8]) -> Option<usize> {
    if packet.len() < ART_DMX_HEADER_LEN
        || &packet[..8] != b"Art-Net\0"
        || packet[8..10] != [0x00, 0x50]
        || u16::from_be_bytes([packet[10], packet[11]]) < 14
    {
        return None;
    }
    let length = u16::from_be_bytes([packet[16], packet[17]]) as usize;
    ((2..=DMX_MAX_SLOTS).contains(&length)
        && length % 2 == 0
        && packet.len() >= ART_DMX_HEADER_LEN + length)
        .then_some(length)
}

#[test]
fn artnet_dmx_hostile_corpus_is_bounded_and_round_trips() {
    const TARGET: u64 = 0x4152_544e_4554_0001;
    const CASES: usize = 4096;
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("Art-Net ArtDmx", TARGET);
    let base = build_art_dmx_packet_with_sequence(1, 7, &[0x80; DMX_MAX_SLOTS]);
    let with_length = |length: u16| {
        let mut packet = base.to_vec();
        packet[16..18].copy_from_slice(&length.to_be_bytes());
        packet
    };
    let mut fixed = vec![
        Vec::new(),
        b"Art-Net\0".to_vec(),
        base[..ART_DMX_HEADER_LEN].to_vec(),
        with_length(0),
        with_length(1),
        with_length(3),
        with_length(514),
        with_length(0xffff),
        with_length(2)[..ART_DMX_HEADER_LEN + 2].to_vec(),
        with_length(2)[..ART_DMX_HEADER_LEN + 1].to_vec(),
    ];
    let mut old_version = base.to_vec();
    old_version[10..12].copy_from_slice(&13_u16.to_be_bytes());
    fixed.push(old_version);
    let mut art_poll = base.to_vec();
    art_poll[8..10].copy_from_slice(&0x2000_u16.to_le_bytes());
    fixed.push(art_poll);

    for index in 0..CASES {
        let (packet, built) = if index < fixed.len() {
            (fixed[index].clone(), None)
        } else if rng.chance(25) {
            let universe = rng.next_u64() as u16;
            let sequence = rng.byte();
            let frame = random_frame(&mut rng);
            let packet = build_art_dmx_packet_with_sequence(universe, sequence, &frame);
            (packet.to_vec(), Some((universe, sequence, frame)))
        } else {
            (mutate(&mut rng, &base, DATAGRAM_MAX_BYTES), None)
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| parse_art_dmx_packet(&packet)))
            .unwrap_or_else(|_| panic!("ArtDmx case {index} panicked"));
        assert_eq!(
            parsed.as_ref().map(|packet| packet.data.len()),
            art_dmx_expected_length(&packet),
            "ArtDmx case {index} acceptance differs from the ArtDmx rule"
        );
        if let Some(parsed) = &parsed {
            assert!(slice_within(parsed.data, &packet), "case {index}");
            assert!(parsed.data.len() <= DMX_MAX_SLOTS, "case {index}");
            assert_eq!(
                parsed.universe,
                u16::from_le_bytes([packet[14], packet[15]])
            );
        }
        if let Some((universe, sequence, frame)) = built {
            let parsed = parsed.as_ref().expect("built ArtDmx must parse");
            assert_eq!(
                (parsed.universe, parsed.sequence, parsed.data),
                (universe, sequence, frame.as_slice())
            );
        }
        tally.record(parsed.is_some());
    }
    tally.finish();
}

/// Independent statement of the E1.31 DMX-levels acceptance rule.
fn sacn_expected_length(packet: &[u8]) -> Option<usize> {
    if packet.len() < SACN_PACKET_LEN
        || &packet[4..16] != b"ASC-E1.17\0\0\0"
        || packet[18..22] != [0, 0, 0, 4]
        || packet[40..44] != [0, 0, 0, 2]
        || packet[117] != 0x02
        || packet[118] != 0xa1
        || packet[125] != 0x00
    {
        return None;
    }
    // The property count includes the start code slot.
    let property_count = u16::from_be_bytes([packet[123], packet[124]]) as usize;
    (1..=DMX_MAX_SLOTS + 1)
        .contains(&property_count)
        .then(|| property_count - 1)
}

#[test]
fn sacn_dmx_hostile_corpus_is_bounded_and_round_trips() {
    const TARGET: u64 = 0x5341_434e_0000_0002;
    const CASES: usize = 4096;
    const CID: [u8; 16] = [0x5a; 16];
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("sACN DMX", TARGET);
    let base = build_sacn_dmx_packet(1, 7, &CID, "Corpus", &[0x80; DMX_MAX_SLOTS]);
    let with = |offset: usize, bytes: &[u8]| {
        let mut packet = base.to_vec();
        packet[offset..offset + bytes.len()].copy_from_slice(bytes);
        packet
    };
    let mut padded = base.to_vec();
    padded.resize(DATAGRAM_MAX_BYTES, 0x55);
    let fixed = vec![
        Vec::new(),
        base[..SACN_PACKET_LEN - 1].to_vec(),
        with(125, &[0xdd]),
        with(125, &[0x17]),
        with(123, &0_u16.to_be_bytes()),
        with(123, &1_u16.to_be_bytes()),
        with(123, &514_u16.to_be_bytes()),
        with(123, &0xffff_u16.to_be_bytes()),
        padded.clone(),
        {
            padded[123..125].copy_from_slice(&0xffff_u16.to_be_bytes());
            padded
        },
    ];

    for index in 0..CASES {
        let (packet, built) = if index < fixed.len() {
            (fixed[index].clone(), None)
        } else if rng.chance(25) {
            let universe = rng.next_u64() as u16;
            let sequence = rng.byte();
            let frame = random_frame(&mut rng);
            let packet = build_sacn_dmx_packet(universe, sequence, &CID, "Corpus", &frame);
            (packet.to_vec(), Some((universe, sequence, frame)))
        } else {
            (mutate(&mut rng, &base, DATAGRAM_MAX_BYTES), None)
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| parse_sacn_dmx_packet(&packet)))
            .unwrap_or_else(|_| panic!("sACN case {index} panicked"));
        assert_eq!(
            parsed.as_ref().map(|packet| packet.data.len()),
            sacn_expected_length(&packet),
            "sACN case {index} acceptance differs from the E1.31 levels rule"
        );
        if let Some(parsed) = &parsed {
            assert!(slice_within(parsed.data, &packet), "case {index}");
            assert!(parsed.data.len() <= DMX_MAX_SLOTS, "case {index}");
        }
        if let Some((universe, sequence, frame)) = built {
            let parsed = parsed.as_ref().expect("built sACN packet must parse");
            assert_eq!(
                (parsed.universe, parsed.sequence, parsed.data),
                (universe, sequence, frame.as_slice())
            );
        }
        tally.record(parsed.is_some());
    }
    tally.finish();
}

fn random_rdm_message(rng: &mut CorpusRng) -> RdmMessage {
    let classes = [
        RdmCommandClass::DiscoveryCommand,
        RdmCommandClass::DiscoveryCommandResponse,
        RdmCommandClass::GetCommand,
        RdmCommandClass::GetCommandResponse,
        RdmCommandClass::SetCommand,
        RdmCommandClass::SetCommandResponse,
    ];
    let mut uid = || RdmUid::new(rng.next_u64() as u16, rng.next_u64() as u32);
    let destination = uid();
    let source = uid();
    let parameter_length = if rng.chance(10) {
        RDM_MAX_PARAMETER_DATA_LENGTH
    } else {
        rng.below(24)
    };
    RdmMessage {
        destination,
        source,
        transaction_number: rng.byte(),
        port_id_or_response_type: rng.byte(),
        message_count: rng.byte(),
        sub_device: rng.next_u64() as u16,
        command_class: *rng.pick(&classes),
        parameter_id: rng.next_u64() as u16,
        parameter_data: rng.bytes(parameter_length),
    }
}

#[test]
fn rdm_hostile_corpus_decodes_only_canonical_messages() {
    const TARGET: u64 = 0x5244_4d00_0000_0003;
    const CASES: usize = 4096;
    let mut rng = CorpusRng::new(TARGET);
    let mut messages = Tally::new("RDM message", TARGET);
    let mut discovery = Tally::new("RDM discovery", TARGET);
    let base = random_rdm_message(&mut rng).encode().unwrap();
    let fixed = vec![
        Vec::new(),
        base[..base.len() - 1].to_vec(),
        vec![0xcc, 0x01, 0x18],
        vec![0xcc; 26],
        vec![0xcc, 0x01, 0xff],
    ];
    for index in 0..CASES {
        let (packet, built) = if index < fixed.len() {
            (fixed[index].clone(), None)
        } else if rng.chance(30) {
            let message = random_rdm_message(&mut rng);
            (message.encode().unwrap(), Some(message))
        } else {
            (mutate(&mut rng, &base, 300), None)
        };
        let decoded = catch_unwind(AssertUnwindSafe(|| RdmMessage::decode(&packet)))
            .unwrap_or_else(|_| panic!("RDM case {index} panicked"));
        if let Ok(message) = &decoded {
            assert_eq!(
                message.encode().unwrap(),
                packet,
                "RDM case {index} decoded a non-canonical packet"
            );
        }
        if let Some(message) = built {
            assert_eq!(decoded.as_ref(), Ok(&message));
        }
        messages.record(decoded.is_ok());

        // Discovery responses: collision-safe UID encoding behind a preamble.
        let uid = RdmUid::new(rng.next_u64() as u16, rng.next_u64() as u32);
        let mut response = vec![0xfe; rng.below(8)];
        response.push(0xaa);
        let mut checksum = 0_u16;
        for byte in uid.0 {
            for encoded in [byte | 0xaa, byte | 0x55] {
                checksum = checksum.wrapping_add(u16::from(encoded));
                response.push(encoded);
            }
        }
        for byte in checksum.to_be_bytes() {
            response.extend([byte | 0xaa, byte | 0x55]);
        }
        let mutated = rng.chance(60);
        let response = if mutated {
            mutate(&mut rng, &response, 64)
        } else {
            response
        };
        let decoded = catch_unwind(AssertUnwindSafe(|| decode_discovery_response(&response)))
            .unwrap_or_else(|_| panic!("RDM discovery case {index} panicked"));
        if !mutated {
            assert_eq!(decoded, Ok(uid), "discovery case {index}");
        }
        discovery.record(decoded.is_ok());
    }
    messages.finish();
    discovery.finish();
}

#[test]
fn artnet_rdm_hostile_corpus_round_trips_accepted_packets() {
    const TARGET: u64 = 0x4152_544e_4554_0004;
    const CASES: usize = 4096;
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("Art-Net ArtRdm", TARGET);
    let base = build_art_rdm_packet(0x0123, &random_rdm_message(&mut rng), 4, 8).unwrap();
    let fixed = vec![
        Vec::new(),
        base[..48].to_vec(),
        {
            let mut packet = base.clone();
            packet[21] |= 0x80;
            packet
        },
        {
            let mut packet = base.clone();
            packet[22] = 0x01;
            packet
        },
    ];
    for index in 0..CASES {
        let (packet, built) = if index < fixed.len() {
            (fixed[index].clone(), None)
        } else if rng.chance(25) {
            let port_address = rng.below(0x8000) as u16;
            let message = random_rdm_message(&mut rng);
            let (available, max) = (rng.byte(), rng.byte());
            let packet = build_art_rdm_packet(port_address, &message, available, max).unwrap();
            (packet, Some((port_address, message)))
        } else {
            (mutate(&mut rng, &base, DATAGRAM_MAX_BYTES), None)
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| parse_art_rdm_packet(&packet)))
            .unwrap_or_else(|_| panic!("ArtRdm case {index} panicked"));
        if let Ok(parsed) = &parsed {
            assert!(parsed.port_address <= 0x7fff, "case {index}");
            let rebuilt = build_art_rdm_packet(
                parsed.port_address,
                &parsed.message,
                parsed.fifo_available,
                parsed.fifo_max,
            )
            .unwrap();
            // Bytes 10..19 carry the version floor and spare fields the
            // parser deliberately does not pin; everything after must match.
            assert_eq!(rebuilt.len(), packet.len(), "case {index}");
            assert_eq!(rebuilt[19..], packet[19..], "case {index}");
        }
        if let Some((port_address, message)) = built {
            let parsed = parsed.as_ref().expect("built ArtRdm must parse");
            assert_eq!(
                (parsed.port_address, &parsed.message),
                (port_address, &message)
            );
        }
        tally.record(parsed.is_ok());
    }
    tally.finish();
}

fn tod_data_packet(rng: &mut CorpusRng, uid_count: usize) -> Vec<u8> {
    let mut packet = vec![0_u8; 28 + uid_count * 6];
    packet[..8].copy_from_slice(b"Art-Net\0");
    packet[8..10].copy_from_slice(&0x8100_u16.to_le_bytes());
    packet[10..12].copy_from_slice(&14_u16.to_be_bytes());
    packet[12] = 0x01;
    packet[21] = rng.byte() & 0x7f;
    packet[23] = rng.byte();
    packet[24..26].copy_from_slice(&(uid_count as u16).to_be_bytes());
    packet[27] = uid_count as u8;
    for byte in &mut packet[28..] {
        *byte = rng.byte();
    }
    packet
}

#[test]
fn artnet_tod_data_hostile_corpus_matches_declared_uid_count() {
    const TARGET: u64 = 0x4152_544e_4554_0005;
    const CASES: usize = 4096;
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("Art-Net ArtTodData", TARGET);
    let base = tod_data_packet(&mut rng, 3);
    let mut unavailable = base.clone();
    unavailable[22] = 0xff;
    let fixed = vec![Vec::new(), base[..27].to_vec(), unavailable];
    for index in 0..CASES {
        let (packet, built) = if index < fixed.len() {
            (fixed[index].clone(), false)
        } else if rng.chance(25) {
            let count = rng.below(201);
            (tod_data_packet(&mut rng, count), true)
        } else {
            (mutate(&mut rng, &base, DATAGRAM_MAX_BYTES), false)
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| parse_art_tod_data(&packet)))
            .unwrap_or_else(|_| panic!("ArtTodData case {index} panicked"));
        match &parsed {
            Ok(tod) => {
                assert_eq!(tod.uids.len(), usize::from(packet[27]), "case {index}");
                assert_eq!(packet.len(), 28 + tod.uids.len() * 6, "case {index}");
                assert!(tod.port_address <= 0x7fff, "case {index}");
                for (uid, bytes) in tod.uids.iter().zip(packet[28..].chunks_exact(6)) {
                    assert_eq!(uid.0.as_slice(), bytes, "case {index}");
                }
            }
            Err(ArtNetError::TodUnavailable | ArtNetError::InvalidRdmPacket(_)) => {}
            Err(other) => panic!("ArtTodData case {index} returned {other:?}"),
        }
        if built {
            assert!(parsed.is_ok(), "built ArtTodData case {index} rejected");
        }
        tally.record(parsed.is_ok());
    }
    tally.finish();
}

const OSC_ADDRESSES: &[&str] = &[
    "/syndocal/fixture/42/Dimmer",
    "/syndocal/fixture/42/highlight",
    "/syndocal/group/front/solo",
    "/syndocal/blackout",
    "/syndocal/all/blackout",
    "/syndocal/bpm",
    "/syndocal/tap",
    "/syndocal/master",
    "/syndocal/submaster/front",
    "/syndocal/cue/7",
    "/syndocal/cue/pause",
    "/syndocal/clock/sync",
    "/syndocal/timecode",
    "/syndocal/timeline/beat/next",
    "/syndocal/video/master",
    "/syndocal/video/layer/1/opacity",
    "/syndocal/video/layer/1/fade",
    "/syndocal/video/output/2/opacity",
    "/corpus/0",
    "/corpus/1",
    "/corpus/wildcard",
    "/",
];

fn osc_mappings() -> Vec<OscControlMapping> {
    crate::osc::OSC_CONTROL_ACTION_VARIANT_NAMES
        .iter()
        .enumerate()
        .map(|(index, name)| OscControlMapping {
            address: if index % 2 == 0 {
                format!("/corpus/{}", index % 4 / 2)
            } else {
                "/corpus/*".to_string()
            },
            action: serde_json::from_value(json!(name))
                .expect("inventory names are the serde variant names"),
            fixture_id: Some(42),
            attribute: Some("Dimmer".to_string()),
            group_id: Some("front".to_string()),
            cue_id: Some(7),
            layer_id: Some(1),
            output_id: Some(2),
            video_param: Some(VideoParam::Opacity),
            cue_point_index: Some(0),
            duration_ms: Some(250),
            low: if index % 5 == 0 { f32::NAN } else { 0.0 },
            high: if index % 7 == 0 { f32::INFINITY } else { 1.0 },
        })
        .collect()
}

fn osc_arg(rng: &mut CorpusRng, depth: usize) -> OscType {
    let random = rng.next_u64();
    match rng.below(14) {
        0 => OscType::Int(*rng.pick(&[0, 1, -1, i32::MIN, i32::MAX, random as i32])),
        1 | 2 => OscType::Float(*rng.pick(&[
            f32::NAN,
            f32::INFINITY,
            f32::NEG_INFINITY,
            -0.0,
            f32::MIN_POSITIVE / 2.0,
            f32::MAX,
            0.5,
            1.0,
            f32::from_bits(random as u32),
        ])),
        3 => OscType::Double(*rng.pick(&[
            f64::NAN,
            f64::INFINITY,
            f64::NEG_INFINITY,
            1e300,
            0.25,
            f64::from_bits(random),
        ])),
        4 => OscType::Long(rng.next_u64() as i64),
        5 => OscType::String(
            rng.pick(&["link", "mtc", "01:02:03:04@30", "", "x"])
                .to_string(),
        ),
        6 => OscType::Bool(rng.chance(50)),
        7 => OscType::Nil,
        8 => OscType::Inf,
        9 => {
            let length = rng.below(17);
            OscType::Blob(rng.bytes(length))
        }
        10 if depth < 3 => OscType::Array(OscArray {
            content: (0..rng.below(4)).map(|_| osc_arg(rng, depth + 1)).collect(),
        }),
        11 => OscType::Time(OscTime {
            seconds: rng.next_u64() as u32,
            fractional: rng.next_u64() as u32,
        }),
        12 => OscType::Midi(OscMidiMessage {
            port: rng.byte(),
            status: rng.byte(),
            data1: rng.byte(),
            data2: rng.byte(),
        }),
        _ => OscType::Color(OscColor {
            red: rng.byte(),
            green: rng.byte(),
            blue: rng.byte(),
            alpha: rng.byte(),
        }),
    }
}

fn osc_packet(rng: &mut CorpusRng, depth: usize) -> OscPacket {
    if depth < 4 && rng.chance(30) {
        OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: rng.next_u64() as u32,
                fractional: rng.next_u64() as u32,
            },
            content: (0..rng.below(4))
                .map(|_| osc_packet(rng, depth + 1))
                .collect(),
        })
    } else {
        OscPacket::Message(OscMessage {
            addr: rng.pick(OSC_ADDRESSES).to_string(),
            args: (0..rng.below(4)).map(|_| osc_arg(rng, 0)).collect(),
        })
    }
}

fn nested_bundle(depth: usize, innermost: OscPacket) -> OscPacket {
    (0..depth).fold(innermost, |inner, _| {
        OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![inner],
        })
    })
}

/// Events compared by `Debug` text so NaN payloads compare as equal.
fn osc_events(packet: &OscPacket, mappings: &[OscControlMapping]) -> Vec<String> {
    events_from_packet_with_mappings(packet, mappings)
        .iter()
        .map(|event| format!("{event:?}"))
        .collect()
}

#[test]
fn osc_hostile_corpus_decodes_dispatches_and_reencodes_without_panic() {
    const TARGET: u64 = 0x4f53_4300_0000_0006;
    const CASES: usize = 2048;
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("OSC", TARGET);
    let mappings = osc_mappings();
    let non_finite = OscPacket::Message(OscMessage {
        addr: "/corpus/0".to_string(),
        args: vec![
            OscType::Float(f32::NAN),
            OscType::Double(f64::INFINITY),
            OscType::Float(f32::NEG_INFINITY),
        ],
    });
    let mut fixed: Vec<Vec<u8>> = vec![
        Vec::new(),
        b"/".to_vec(),
        b"#bundle\0".to_vec(),
        b"#bundle\0\0\0\0\0\0\0\0\x01\0\0\0\x08".to_vec(),
        b"/a\0\0,[[[\0\0\0\0".to_vec(),
        b"/a\0\0,]\0\0".to_vec(),
        b"/a\0\0,s\0\0abc".to_vec(),
        b"/a\0\0,b\0\0\x7f\xff\xff\xff".to_vec(),
        encoder::encode(&non_finite).unwrap(),
    ];
    for name in ["/corpus/1", "/syndocal/bpm", "/syndocal/master"] {
        let mut message = non_finite.clone();
        if let OscPacket::Message(message) = &mut message {
            message.addr = name.to_string();
        }
        fixed.push(encoder::encode(&message).unwrap());
    }
    // The deepest bundle nesting that still fits one receive buffer.
    let deepest = (1..)
        .map(|depth| encoder::encode(&nested_bundle(depth, non_finite.clone())).unwrap())
        .take_while(|bytes| bytes.len() <= DATAGRAM_MAX_BYTES)
        .last()
        .unwrap();
    fixed.push(deepest);

    for index in 0..CASES {
        let bytes = if index < fixed.len() {
            fixed[index].clone()
        } else {
            let packet = osc_packet(&mut rng, 0);
            let encoded = encoder::encode(&packet).expect("generated OSC packets encode");
            if rng.chance(50) {
                encoded
            } else {
                mutate(&mut rng, &encoded, DATAGRAM_MAX_BYTES)
            }
        };
        if bytes.len() > DATAGRAM_MAX_BYTES {
            continue;
        }
        let outcome = catch_unwind(AssertUnwindSafe(|| {
            let Ok((_, packet)) = rosc::decoder::decode_udp(&bytes) else {
                return None;
            };
            let events = osc_events(&packet, &mappings);
            let _ = learned_control_from_packet(&packet);
            // Re-encoding a decoded packet and decoding it again must not
            // change what the dispatcher emits.
            let reencoded = encoder::encode(&packet).expect("decoded OSC packets re-encode");
            let (_, again) = rosc::decoder::decode_udp(&reencoded).expect("re-encoded OSC decodes");
            assert_eq!(osc_events(&again, &mappings), events);
            Some(events.len())
        }))
        .unwrap_or_else(|_| panic!("OSC case {index} panicked"));
        tally.record(outcome.is_some());
    }
    tally.finish();
}

#[test]
fn remote_request_hostile_corpus_uses_fixed_non_echoing_errors() {
    const TARGET: u64 = 0x5245_4d4f_5445_0007;
    const CASES: usize = 4096;
    const CANARY: &str = "CANARY-5d1f";
    let mut rng = CorpusRng::new(TARGET);
    let mut tally = Tally::new("Web Remote request", TARGET);
    let allowed = [
        RemoteParseError::InvalidJson,
        RemoteParseError::MissingType,
        RemoteParseError::UnknownType,
        RemoteParseError::MissingField(""),
        RemoteParseError::InvalidField(""),
    ]
    .map(|error| remote_parse_error_response_json(&error));
    let wire_types = RemoteWireOperation::CONTROL_PLANE_OPERATIONS
        .iter()
        .map(|(_, wire_type)| *wire_type)
        .collect::<Vec<_>>();
    let field_names = [
        "fixture_id",
        "fixture_ids",
        "attribute",
        "value",
        "bpm",
        "enabled",
        "cue_id",
        "effect_id",
        "graph_id",
        "group_id",
        "layer_id",
        "output_id",
        "param",
        "level",
        "position_ms",
        "duration_ms",
        "direction",
        "source",
        "index",
        "opacity",
        CANARY,
    ];
    let mut fixed = vec![
        String::new(),
        "{".to_string(),
        "null".to_string(),
        "[]".to_string(),
        "{}".to_string(),
        r#"{"type":1}"#.to_string(),
        format!(r#"{{"type":"{CANARY}"}}"#),
        r#"{"type":"getSnapshot"}"#.to_string(),
        r#"{"type":"triggerNextCue"}"#.to_string(),
        r#"{"type":"setBpm","bpm":1e999}"#.to_string(),
        format!(r#"{{"type":"setBpm","bpm":"{CANARY}"}}"#),
        format!("{}0{}", "[".repeat(256), "]".repeat(256)),
        "{\"type\":\"setBpm\",\"bpm\":120}\u{0}".to_string(),
    ];
    fixed.extend(
        wire_types
            .iter()
            .map(|wire_type| json!({"type": wire_type}).to_string()),
    );

    for index in 0..CASES {
        let text = if index < fixed.len() {
            fixed[index].clone()
        } else {
            let mut object = serde_json::Map::new();
            if rng.chance(90) {
                let wire_type = if rng.chance(90) {
                    json!(rng.pick(&wire_types))
                } else {
                    json!(CANARY)
                };
                object.insert("type".to_string(), wire_type);
            }
            for _ in 0..rng.below(7) {
                let value = match rng.below(10) {
                    0 => json!(rng.next_u64() % 1_000),
                    1 => json!(*rng.pick(&[-1.0, 0.5, 65_535.0, 65_536.0, 1e308, -0.0])),
                    2 => json!(u64::MAX),
                    3 => json!(CANARY),
                    4 => json!(rng.chance(50)),
                    5 => Value::Null,
                    6 => json!([1, CANARY, null]),
                    7 => json!({ CANARY: [CANARY] }),
                    8 => json!(*rng.pick(&["Dimmer", "opacity", "link", "front"])),
                    _ => json!(rng.next_u64() as i64),
                };
                object.insert(rng.pick(&field_names).to_string(), value);
            }
            let text = Value::Object(object).to_string();
            if rng.chance(30) {
                let mut chars = text.chars().collect::<Vec<_>>();
                let at = rng.below(chars.len() + 1);
                match rng.below(3) {
                    0 => {
                        chars.truncate(at);
                    }
                    1 => chars.insert(at, *rng.pick(&['"', '{', '}', ',', ':', '\\', '\u{0}'])),
                    _ => {
                        let canary = CANARY.chars().collect::<Vec<_>>();
                        chars.splice(at..at, canary);
                    }
                }
                chars.into_iter().collect()
            } else {
                text
            }
        };
        let parsed = catch_unwind(AssertUnwindSafe(|| request_from_text(&text)))
            .unwrap_or_else(|_| panic!("Web Remote case {index} panicked"));
        if let Err(error) = &parsed {
            let response = remote_parse_error_response_json(error);
            assert!(
                allowed.contains(&response),
                "case {index} produced an unlisted error: {response}"
            );
            assert!(!response.contains(CANARY), "case {index} echoed input");
        }
        tally.record(parsed.is_ok());
    }
    tally.finish();
}
