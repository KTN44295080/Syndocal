use std::{
    net::{Ipv4Addr, SocketAddr, UdpSocket},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use protocol::{
    DmxControlMapping, DmxInputConfig, DmxInputProtocol, DmxInputStatus, DmxMergeMode,
    LearnedDmxControl, OscControlMapping,
};
use thiserror::Error;

#[derive(Debug, Clone)]
pub enum DmxInputEvent {
    Frame {
        universe: u16,
        values: Box<[u8; 512]>,
        merge_mode: DmxMergeMode,
    },
    SignalLost {
        universe: u16,
    },
}

macro_rules! control_plane_variant_inventory {
    (
        $names:ident,
        $matcher:ident,
        $uniqueness:ident,
        $enum_type:ty;
        $( $variant:ident => $pattern:pat ),+ $(,)?
    ) => {
        pub(crate) const $names: &[&str] = &[$(stringify!($variant)),+];

        #[allow(dead_code)]
        enum $uniqueness {
            $($variant),+
        }

        #[allow(dead_code)]
        fn $matcher(value: &$enum_type) -> &'static str {
            match value {
                $($pattern => stringify!($variant)),+
            }
        }
    };
}

control_plane_variant_inventory!(
    DMX_INPUT_PROTOCOL_VARIANT_NAMES,
    dmx_input_protocol_variant_name,
    DmxInputProtocolInventoryUniqueness,
    DmxInputProtocol;
    ArtNet => DmxInputProtocol::ArtNet,
    Sacn => DmxInputProtocol::Sacn,
);

control_plane_variant_inventory!(
    DMX_INPUT_EVENT_VARIANT_NAMES,
    dmx_input_event_variant_name,
    DmxInputEventInventoryUniqueness,
    DmxInputEvent;
    Frame => DmxInputEvent::Frame { .. },
    SignalLost => DmxInputEvent::SignalLost { .. },
);

/// Converts the protocol-native universe carried on the wire into the
/// zero-based universe used by persisted control mappings. Raw DMX merge keeps
/// the protocol-native universe so this conversion cannot change its existing
/// routing contract.
pub fn control_mapping_universe(protocol: DmxInputProtocol, network_universe: u16) -> Option<u16> {
    match protocol {
        DmxInputProtocol::ArtNet => Some(network_universe),
        DmxInputProtocol::Sacn => network_universe.checked_sub(1),
    }
}

pub fn control_events_from_frame(
    universe: u16,
    values: &[u8; 512],
    mappings: &[DmxControlMapping],
) -> Vec<crate::osc::OscInputEvent> {
    control_events_from_changed_frame(universe, values, None, mappings)
}

pub fn control_events_from_changed_frame(
    universe: u16,
    values: &[u8; 512],
    previous: Option<&[u8; 512]>,
    mappings: &[DmxControlMapping],
) -> Vec<crate::osc::OscInputEvent> {
    mappings
        .iter()
        .filter(|mapping| mapping.universe == universe && (1..=512).contains(&mapping.channel))
        .filter(|mapping| {
            let index = usize::from(mapping.channel - 1);
            previous.is_none_or(|previous| previous[index] != values[index])
        })
        .filter_map(|mapping| {
            let normalized = values[usize::from(mapping.channel - 1)] as f32 / 255.0;
            let shared = OscControlMapping {
                address: "/dmx-input".to_string(),
                action: mapping.action.clone(),
                fixture_id: mapping.fixture_id,
                attribute: mapping.attribute.clone(),
                group_id: mapping.group_id.clone(),
                cue_id: mapping.cue_id,
                layer_id: mapping.layer_id,
                output_id: mapping.output_id,
                video_param: mapping.video_param.clone(),
                cue_point_index: mapping.cue_point_index,
                duration_ms: mapping.duration_ms,
                low: mapping.low,
                high: mapping.high,
            };
            crate::osc::event_from_control_value(normalized, &shared)
        })
        .collect()
}

#[derive(Debug, Error)]
pub enum DmxInputError {
    #[error("invalid DMX input bind address '{0}'")]
    InvalidBindAddress(String),
    #[error("failed to bind DMX input socket: {0}")]
    Bind(std::io::Error),
    #[error("failed to configure DMX input socket: {0}")]
    Configure(std::io::Error),
    #[error("failed to start DMX input thread: {0}")]
    Thread(std::io::Error),
}

pub struct DmxInput {
    stop: Arc<AtomicBool>,
    status: Arc<Mutex<DmxInputStatus>>,
    learn: Arc<Mutex<DmxLearnState>>,
    thread: Option<thread::JoinHandle<()>>,
}

#[derive(Debug, Default)]
struct DmxLearnState {
    armed: bool,
    baseline: Option<Box<[u8; 512]>>,
    learned: Option<LearnedDmxControl>,
}

impl DmxInput {
    pub fn start<F>(config: DmxInputConfig, on_event: F) -> Result<Self, DmxInputError>
    where
        F: Fn(DmxInputEvent) + Send + Sync + 'static,
    {
        let bind_ip = config
            .bind_ip
            .trim()
            .parse::<Ipv4Addr>()
            .map_err(|_| DmxInputError::InvalidBindAddress(config.bind_ip.clone()))?;
        let socket = UdpSocket::bind(SocketAddr::from((bind_ip, config.port)))
            .map_err(DmxInputError::Bind)?;
        socket
            .set_read_timeout(Some(Duration::from_millis(100)))
            .map_err(DmxInputError::Configure)?;
        if config.protocol == DmxInputProtocol::Sacn {
            let multicast = Ipv4Addr::new(
                239,
                255,
                (config.universe >> 8) as u8,
                (config.universe & 0xff) as u8,
            );
            socket
                .join_multicast_v4(&multicast, &Ipv4Addr::UNSPECIFIED)
                .map_err(DmxInputError::Configure)?;
        }

        let stop = Arc::new(AtomicBool::new(false));
        let status = Arc::new(Mutex::new(DmxInputStatus {
            running: true,
            ..DmxInputStatus::default()
        }));
        let learn = Arc::new(Mutex::new(DmxLearnState::default()));
        let thread_stop = Arc::clone(&stop);
        let thread_status = Arc::clone(&status);
        let thread_learn = Arc::clone(&learn);
        let callback = Arc::new(on_event);
        let thread = thread::Builder::new()
            .name("syndocal-dmx-input".to_string())
            .spawn(move || {
                let mut buffer = [0u8; 1536];
                let mut last_packet_at: Option<Instant> = None;
                while !thread_stop.load(Ordering::Relaxed) {
                    match socket.recv_from(&mut buffer) {
                        Ok((received, source)) => {
                            let parsed = match config.protocol {
                                DmxInputProtocol::ArtNet => {
                                    crate::artnet::parse_art_dmx_packet(&buffer[..received])
                                        .map(|packet| (packet.universe, packet.data))
                                }
                                DmxInputProtocol::Sacn => {
                                    crate::sacn::parse_sacn_dmx_packet(&buffer[..received])
                                        .map(|packet| (packet.universe, packet.data))
                                }
                            };
                            let Some((universe, data)) = parsed else {
                                if let Ok(mut status) = thread_status.lock() {
                                    status.invalid_packets =
                                        status.invalid_packets.saturating_add(1);
                                }
                                continue;
                            };
                            if universe != config.universe {
                                continue;
                            }
                            let mut values = Box::new([0u8; 512]);
                            let length = data.len().min(values.len());
                            values[..length].copy_from_slice(&data[..length]);
                            last_packet_at = Some(Instant::now());
                            if let Ok(mut status) = thread_status.lock() {
                                status.signal_present = true;
                                status.packets_received = status.packets_received.saturating_add(1);
                                status.last_packet_unix_ms = Some(current_unix_ms());
                                status.source_address = Some(source.to_string());
                            }
                            if let Some(mapping_universe) =
                                control_mapping_universe(config.protocol, universe)
                            {
                                update_dmx_learning(&thread_learn, mapping_universe, &values);
                            }
                            callback(DmxInputEvent::Frame {
                                universe,
                                values,
                                merge_mode: config.merge_mode,
                            });
                        }
                        Err(error)
                            if matches!(
                                error.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(_) => break,
                    }
                    if last_packet_at.is_some_and(|last| {
                        last.elapsed() >= Duration::from_millis(config.timeout_ms.max(100))
                    }) {
                        last_packet_at = None;
                        if let Ok(mut status) = thread_status.lock() {
                            status.signal_present = false;
                        }
                        callback(DmxInputEvent::SignalLost {
                            universe: config.universe,
                        });
                    }
                }
                callback(DmxInputEvent::SignalLost {
                    universe: config.universe,
                });
                if let Ok(mut status) = thread_status.lock() {
                    status.running = false;
                    status.signal_present = false;
                }
            })
            .map_err(DmxInputError::Thread)?;
        Ok(Self {
            stop,
            status,
            learn,
            thread: Some(thread),
        })
    }

    pub fn status(&self) -> DmxInputStatus {
        self.status
            .lock()
            .map(|status| status.clone())
            .unwrap_or_default()
    }

    pub fn learn_control(&self, timeout: Duration) -> Option<LearnedDmxControl> {
        if let Ok(mut learn) = self.learn.lock() {
            learn.armed = true;
            learn.baseline = None;
            learn.learned = None;
        } else {
            return None;
        }
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if let Ok(mut learn) = self.learn.lock() {
                if let Some(learned) = learn.learned.take() {
                    learn.armed = false;
                    learn.baseline = None;
                    return Some(learned);
                }
            }
            thread::sleep(Duration::from_millis(10));
        }
        if let Ok(mut learn) = self.learn.lock() {
            learn.armed = false;
            learn.baseline = None;
        }
        None
    }
}

fn update_dmx_learning(learn: &Arc<Mutex<DmxLearnState>>, universe: u16, values: &[u8; 512]) {
    let Ok(mut learn) = learn.lock() else {
        return;
    };
    if !learn.armed || learn.learned.is_some() {
        return;
    }
    let Some(baseline) = learn.baseline.as_ref() else {
        learn.baseline = Some(Box::new(*values));
        return;
    };
    let changed = baseline
        .iter()
        .zip(values.iter())
        .enumerate()
        .map(|(index, (before, after))| (index, before.abs_diff(*after), *after))
        .filter(|(_, delta, _)| *delta >= 2)
        .max_by_key(|(index, delta, _)| (*delta, std::cmp::Reverse(*index)));
    if let Some((index, _, value)) = changed {
        learn.learned = Some(LearnedDmxControl {
            universe,
            channel: index as u16 + 1,
            value,
        });
    } else {
        learn.baseline = Some(Box::new(*values));
    }
}

impl Drop for DmxInput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn current_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use protocol::DmxControlAction;
    use std::sync::mpsc;

    fn fixture_attribute_mapping(universe: u16, channel: u16) -> DmxControlMapping {
        DmxControlMapping {
            universe,
            channel,
            action: DmxControlAction::FixtureAttribute,
            fixture_id: Some(42),
            attribute: Some("ColorRed".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            output_id: None,
            video_param: None,
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 65_535.0,
        }
    }

    #[test]
    fn dmx_control_universe_normalizes_artnet_and_sacn_numbering() {
        assert_eq!(
            control_mapping_universe(DmxInputProtocol::ArtNet, 0),
            Some(0)
        );
        assert_eq!(
            control_mapping_universe(DmxInputProtocol::ArtNet, 15),
            Some(15)
        );
        assert_eq!(control_mapping_universe(DmxInputProtocol::Sacn, 1), Some(0));
        assert_eq!(
            control_mapping_universe(DmxInputProtocol::Sacn, 16),
            Some(15)
        );
        assert_eq!(control_mapping_universe(DmxInputProtocol::Sacn, 0), None);
    }

    #[test]
    fn dmx_control_mapping_uses_one_based_channel_and_exact_u16_range() {
        let mut values = [0_u8; 512];
        values[24] = 128;
        let events = control_events_from_frame(0, &values, &[fixture_attribute_mapping(0, 25)]);
        assert_eq!(
            events,
            vec![crate::osc::OscInputEvent::SetAttribute {
                fixture_id: 42,
                attribute: "ColorRed".to_string(),
                value: 32_896,
            }]
        );
    }

    #[test]
    fn dmx_control_mapping_ignores_other_universes_and_invalid_channels() {
        let values = [255_u8; 512];
        let mappings = [
            fixture_attribute_mapping(1, 25),
            fixture_attribute_mapping(0, 0),
            fixture_attribute_mapping(0, 513),
        ];
        assert!(control_events_from_frame(0, &values, &mappings).is_empty());
    }

    #[test]
    fn dmx_control_mapping_reuses_trigger_action_semantics() {
        let mapping = DmxControlMapping {
            universe: 0,
            channel: 11,
            action: protocol::OscControlAction::TriggerCue,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: Some(77),
            layer_id: None,
            output_id: None,
            video_param: None,
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let mut values = [0_u8; 512];
        assert!(control_events_from_frame(0, &values, std::slice::from_ref(&mapping)).is_empty());
        values[10] = 1;
        assert_eq!(
            control_events_from_frame(0, &values, &[mapping]),
            vec![crate::osc::OscInputEvent::TriggerCue(77)]
        );
    }

    #[test]
    fn unchanged_streaming_dmx_does_not_retrigger_control_actions() {
        let mapping = DmxControlMapping {
            universe: 0,
            channel: 11,
            action: protocol::OscControlAction::TriggerCue,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: Some(77),
            layer_id: None,
            output_id: None,
            video_param: None,
            cue_point_index: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let mut previous = [0_u8; 512];
        let mut values = previous;
        values[10] = 255;
        assert_eq!(
            control_events_from_changed_frame(
                0,
                &values,
                Some(&previous),
                std::slice::from_ref(&mapping)
            ),
            vec![crate::osc::OscInputEvent::TriggerCue(77)]
        );
        previous = values;
        assert!(
            control_events_from_changed_frame(0, &values, Some(&previous), &[mapping]).is_empty()
        );
    }

    #[test]
    fn dmx_learn_selects_largest_changed_channel_after_baseline() {
        let learn = Arc::new(Mutex::new(DmxLearnState {
            armed: true,
            ..DmxLearnState::default()
        }));
        let baseline = [0_u8; 512];
        update_dmx_learning(&learn, 2, &baseline);
        let mut changed = baseline;
        changed[4] = 30;
        changed[24] = 180;
        update_dmx_learning(&learn, 2, &changed);
        assert_eq!(
            learn.lock().unwrap().learned,
            Some(LearnedDmxControl {
                universe: 2,
                channel: 25,
                value: 180,
            })
        );
    }

    #[test]
    fn artnet_input_receives_frames_and_emits_signal_loss() {
        let probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let (sender, receiver) = mpsc::channel();
        let input = DmxInput::start(
            DmxInputConfig {
                protocol: DmxInputProtocol::ArtNet,
                bind_ip: "127.0.0.1".to_string(),
                port,
                universe: 3,
                merge_enabled: true,
                merge_mode: DmxMergeMode::Htp,
                timeout_ms: 120,
            },
            move |event| {
                let _ = sender.send(event);
            },
        )
        .unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut frame = [0u8; 512];
        frame[0] = 200;
        frame[511] = 17;
        let packet = crate::artnet::build_art_dmx_packet_with_sequence(3, 1, &frame);
        socket.send_to(&packet, ("127.0.0.1", port)).unwrap();

        let event = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        match event {
            DmxInputEvent::Frame {
                universe,
                values,
                merge_mode,
            } => {
                assert_eq!(universe, 3);
                assert_eq!(values[0], 200);
                assert_eq!(values[511], 17);
                assert_eq!(merge_mode, DmxMergeMode::Htp);
            }
            _ => panic!("expected DMX frame"),
        }
        assert!(input.status().signal_present);
        assert_eq!(input.status().packets_received, 1);

        let lost = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(matches!(lost, DmxInputEvent::SignalLost { universe: 3 }));
        assert!(!input.status().signal_present);
    }

    #[test]
    fn artnet_packet_drives_a_mapped_control_once_until_the_channel_changes() {
        let probe = UdpSocket::bind("127.0.0.1:0").unwrap();
        let port = probe.local_addr().unwrap().port();
        drop(probe);
        let (sender, receiver) = mpsc::channel();
        let mapping = fixture_attribute_mapping(0, 25);
        let previous = Arc::new(Mutex::new(None::<Box<[u8; 512]>>));
        let callback_previous = Arc::clone(&previous);
        let input = DmxInput::start(
            DmxInputConfig {
                protocol: DmxInputProtocol::ArtNet,
                bind_ip: "127.0.0.1".to_string(),
                port,
                universe: 0,
                merge_enabled: false,
                merge_mode: DmxMergeMode::Htp,
                timeout_ms: 1_000,
            },
            move |event| {
                let DmxInputEvent::Frame {
                    universe, values, ..
                } = event
                else {
                    return;
                };
                let prior = callback_previous
                    .lock()
                    .ok()
                    .and_then(|mut frame| frame.replace(values.clone()));
                for event in control_events_from_changed_frame(
                    universe,
                    &values,
                    prior.as_deref(),
                    std::slice::from_ref(&mapping),
                ) {
                    let _ = sender.send(event);
                }
            },
        )
        .unwrap();
        let socket = UdpSocket::bind("127.0.0.1:0").unwrap();
        let mut frame = [0u8; 512];
        frame[24] = 128;

        for sequence in [1, 2] {
            let packet = crate::artnet::build_art_dmx_packet_with_sequence(0, sequence, &frame);
            socket.send_to(&packet, ("127.0.0.1", port)).unwrap();
            if sequence == 1 {
                assert_eq!(
                    receiver.recv_timeout(Duration::from_secs(1)).unwrap(),
                    crate::osc::OscInputEvent::SetAttribute {
                        fixture_id: 42,
                        attribute: "ColorRed".to_string(),
                        value: 32_896,
                    }
                );
            }
        }
        assert!(receiver.recv_timeout(Duration::from_millis(150)).is_err());
        assert_eq!(input.status().packets_received, 2);
    }
}
