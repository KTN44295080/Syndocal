use std::{
    net::UdpSocket,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

use protocol::{
    ClockSource, CueId, EffectId, FixtureId, LearnedOscControl, NodeGraphId, OscControlAction,
    OscControlMapping, OscInputConfig, VideoLayerId, VideoOutputId, VideoParam,
};
use rosc::{decoder, OscMessage, OscPacket, OscType};
use thiserror::Error;

use crate::{parse_clock_source_label, parse_timecode_position_ms};

const OSC_BUFFER_SIZE: usize = 1536;

#[derive(Debug, Clone, PartialEq)]
pub enum OscInputEvent {
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    SetFixtureHighlight {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetFixtureSolo {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetFixturePark {
        fixture_id: FixtureId,
        enabled: bool,
    },
    SetGroupHighlight {
        group_id: String,
        enabled: bool,
    },
    SetGroupSolo {
        group_id: String,
        enabled: bool,
    },
    SetGroupPark {
        group_id: String,
        enabled: bool,
    },
    Blackout(bool),
    AllBlackout(bool),
    ClearFixtureFlags {
        kind: String,
    },
    TriggerCue(CueId),
    TriggerNextCue,
    TriggerPreviousCue,
    SetEffectEnabled {
        effect_id: EffectId,
        enabled: bool,
    },
    SetNodeGraphEnabled {
        graph_id: NodeGraphId,
        enabled: bool,
    },
    SetCueFadePaused(bool),
    SetTimelinePlaying(bool),
    SeekTimeline {
        position_ms: u64,
    },
    SeekTimelineBeat {
        direction: i32,
    },
    SyncTimelineTimecode {
        position_ms: u64,
        source: ClockSource,
    },
    SetVideoParam {
        layer_id: VideoLayerId,
        param: VideoParam,
        value: f32,
    },
    SetVideoPlaying {
        layer_id: VideoLayerId,
        playing: bool,
    },
    SetVideoLoop {
        layer_id: VideoLayerId,
        enabled: bool,
        loop_start_ms: Option<u64>,
        loop_end_ms: Option<u64>,
    },
    FadeVideoLayerOpacity {
        layer_id: VideoLayerId,
        opacity: f32,
        duration_ms: u64,
    },
    SeekVideoLayer {
        layer_id: VideoLayerId,
        position_ms: u64,
    },
    AddVideoCuePoint {
        layer_id: VideoLayerId,
        position_ms: Option<u64>,
    },
    RemoveVideoCuePoint {
        layer_id: VideoLayerId,
        position_ms: u64,
    },
    JumpVideoCuePoint {
        layer_id: VideoLayerId,
        cue_point_index: usize,
    },
    JumpVideoCuePointRelative {
        layer_id: VideoLayerId,
        direction: i32,
    },
    SetVideoLayerEnabled {
        layer_id: VideoLayerId,
        enabled: bool,
    },
    SetVideoLayerSolo {
        layer_id: VideoLayerId,
        solo: bool,
    },
    SetVideoOutputEnabled {
        output_id: VideoOutputId,
        enabled: bool,
    },
    SetVideoOutputOpacity {
        output_id: VideoOutputId,
        opacity: f32,
    },
    FadeVideoOutputOpacity {
        output_id: VideoOutputId,
        opacity: f32,
        duration_ms: u64,
    },
    SetVideoOutputMappingField {
        output_id: VideoOutputId,
        field: String,
        value: f32,
    },
    ApplyVideoOutputMappingPreset {
        output_id: VideoOutputId,
        label: String,
    },
    SetVideoOutputBlackout {
        output_id: VideoOutputId,
        blackout: bool,
    },
    VideoMasterOpacity(f32),
    VideoBlackout(bool),
    LightingMaster(f32),
    SetGroupSubmaster {
        group_id: String,
        level: f32,
    },
    SetBpm(f32),
    TapBpm,
    SyncExternalClock {
        bpm: f32,
        beat_phase: f32,
        source: ClockSource,
    },
}

#[derive(Debug, Error)]
pub enum OscInputError {
    #[error("OSC bind address is required")]
    MissingBindAddress,
    #[error("OSC port must be greater than 0")]
    InvalidPort,
    #[error("failed to bind OSC UDP socket {bind}: {source}")]
    Bind {
        bind: String,
        #[source]
        source: std::io::Error,
    },
}

pub struct OscInput {
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl OscInput {
    pub fn start<F>(config: OscInputConfig, callback: F) -> Result<Self, OscInputError>
    where
        F: Fn(OscInputEvent) + Send + 'static,
    {
        Self::start_with_mappings(config, Vec::new(), callback)
    }

    pub fn start_with_mappings<F>(
        config: OscInputConfig,
        mappings: Vec<OscControlMapping>,
        callback: F,
    ) -> Result<Self, OscInputError>
    where
        F: Fn(OscInputEvent) + Send + 'static,
    {
        let (socket, bind) = bind_osc_socket(&config)?;
        let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
        let stop = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("syndocal-osc-input".to_string())
            .spawn(move || {
                let mut buffer = [0u8; OSC_BUFFER_SIZE];
                while !thread_stop.load(Ordering::Relaxed) {
                    let Ok((received, _)) = socket.recv_from(&mut buffer) else {
                        continue;
                    };
                    let Ok((_, packet)) = decoder::decode_udp(&buffer[..received]) else {
                        continue;
                    };
                    dispatch_packet(&packet, &mappings, &callback);
                }
            })
            .map_err(|source| OscInputError::Bind { bind, source })?;

        Ok(Self {
            stop,
            thread: Some(thread),
        })
    }
}

pub fn learn_osc_control(
    config: OscInputConfig,
    timeout: Duration,
) -> Result<Option<LearnedOscControl>, OscInputError> {
    let (socket, _) = bind_osc_socket(&config)?;
    let _ = socket.set_read_timeout(Some(Duration::from_millis(100)));
    let deadline = Instant::now() + timeout;
    let mut buffer = [0u8; OSC_BUFFER_SIZE];

    while Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(Instant::now());
        let _ = socket.set_read_timeout(Some(remaining.min(Duration::from_millis(100))));
        let Ok((received, _)) = socket.recv_from(&mut buffer) else {
            continue;
        };
        let Ok((_, packet)) = decoder::decode_udp(&buffer[..received]) else {
            continue;
        };
        if let Some(learned) = learned_control_from_packet(&packet) {
            return Ok(Some(learned));
        }
    }

    Ok(None)
}

impl Drop for OscInput {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub fn events_from_packet(packet: &OscPacket) -> Vec<OscInputEvent> {
    events_from_packet_with_mappings(packet, &[])
}

pub fn events_from_packet_with_mappings(
    packet: &OscPacket,
    mappings: &[OscControlMapping],
) -> Vec<OscInputEvent> {
    let mut events = Vec::new();
    collect_events(packet, mappings, &mut events);
    events
}

pub fn learned_control_from_packet(packet: &OscPacket) -> Option<LearnedOscControl> {
    match packet {
        OscPacket::Message(message) => Some(LearnedOscControl {
            address: normalize_osc_address(&message.addr),
            value: message.args.first().and_then(osc_value_to_f32),
            argument_count: message.args.len(),
        }),
        OscPacket::Bundle(bundle) => bundle.content.iter().find_map(learned_control_from_packet),
    }
}

fn dispatch_packet<F>(packet: &OscPacket, mappings: &[OscControlMapping], callback: &F)
where
    F: Fn(OscInputEvent),
{
    for event in events_from_packet_with_mappings(packet, mappings) {
        callback(event);
    }
}

fn collect_events(
    packet: &OscPacket,
    mappings: &[OscControlMapping],
    events: &mut Vec<OscInputEvent>,
) {
    match packet {
        OscPacket::Message(message) => {
            if let Some(event) = event_from_message(message) {
                events.push(event);
            }
            events.extend(events_from_mapped_message(message, mappings));
        }
        OscPacket::Bundle(bundle) => {
            for nested in &bundle.content {
                collect_events(nested, mappings, events);
            }
        }
    }
}

fn bind_osc_socket(config: &OscInputConfig) -> Result<(UdpSocket, String), OscInputError> {
    if config.bind_ip.trim().is_empty() {
        return Err(OscInputError::MissingBindAddress);
    }
    if config.port == 0 {
        return Err(OscInputError::InvalidPort);
    }

    let bind = format!("{}:{}", config.bind_ip, config.port);
    let socket = UdpSocket::bind(&bind).map_err(|source| OscInputError::Bind {
        bind: bind.clone(),
        source,
    })?;
    Ok((socket, bind))
}

fn event_from_message(message: &OscMessage) -> Option<OscInputEvent> {
    let path = message.addr.trim_matches('/');
    let segments = path.split('/').collect::<Vec<_>>();
    let segments = segments
        .strip_prefix(&["syndocal"])
        .unwrap_or(segments.as_slice());

    match segments {
        ["fixture", fixture_id, "highlight"] => {
            let fixture_id = fixture_id.parse::<FixtureId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetFixtureHighlight {
                fixture_id,
                enabled,
            })
        }
        ["fixture", fixture_id, "solo"] => {
            let fixture_id = fixture_id.parse::<FixtureId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetFixtureSolo {
                fixture_id,
                enabled,
            })
        }
        ["fixture", fixture_id, "park"] => {
            let fixture_id = fixture_id.parse::<FixtureId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetFixturePark {
                fixture_id,
                enabled,
            })
        }
        ["fixture", fixture_id, attribute] => {
            let fixture_id = fixture_id.parse::<FixtureId>().ok()?;
            let value = message.args.first().and_then(osc_value_to_u16)?;
            Some(OscInputEvent::SetAttribute {
                fixture_id,
                attribute: (*attribute).to_string(),
                value,
            })
        }
        ["blackout"] => message
            .args
            .first()
            .and_then(osc_value_to_bool)
            .map(OscInputEvent::Blackout),
        ["all_blackout"] | ["all", "blackout"] | ["blackout", "all"] => message
            .args
            .first()
            .and_then(osc_value_to_bool)
            .map(OscInputEvent::AllBlackout),
        ["clear", "fixture_flags"] | ["clear", "fixtures"] | ["fixtures", "clear_flags"] => {
            is_positive_osc_trigger(message.args.first()).then_some(
                OscInputEvent::ClearFixtureFlags {
                    kind: "all".to_string(),
                },
            )
        }
        ["clear", "fixture_flags", kind]
        | ["clear", "fixtures", kind]
        | ["fixtures", "clear_flags", kind] => is_positive_osc_trigger(message.args.first())
            .then_some(OscInputEvent::ClearFixtureFlags {
                kind: (*kind).to_string(),
            }),
        ["cue", "pause"] => {
            let paused = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetCueFadePaused(paused))
        }
        ["cue", "resume"] => Some(OscInputEvent::SetCueFadePaused(false)),
        ["cue", "go" | "next"] => {
            is_positive_osc_trigger(message.args.first()).then_some(OscInputEvent::TriggerNextCue)
        }
        ["cue", "back" | "previous"] => is_positive_osc_trigger(message.args.first())
            .then_some(OscInputEvent::TriggerPreviousCue),
        ["cue", cue_id] | ["cue", cue_id, "go" | "trigger"] => {
            let cue_id = cue_id.parse::<CueId>().ok()?;
            is_positive_osc_trigger(message.args.first())
                .then_some(OscInputEvent::TriggerCue(cue_id))
        }
        ["timeline", "play" | "playing"] => {
            let playing = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetTimelinePlaying(playing))
        }
        ["timeline", "pause" | "stop"] => Some(OscInputEvent::SetTimelinePlaying(false)),
        ["timeline", "seek" | "position"] => {
            let position_ms = message.args.first().and_then(osc_value_to_u64)?;
            Some(OscInputEvent::SeekTimeline { position_ms })
        }
        ["timecode"] | ["timeline", "timecode"] | ["timeline", "sync_timecode"] => {
            let position_ms = message
                .args
                .first()
                .and_then(osc_value_to_timecode_position_ms)?;
            let source = message
                .args
                .get(1)
                .and_then(osc_value_to_clock_source)
                .unwrap_or(ClockSource::Ltc);
            Some(OscInputEvent::SyncTimelineTimecode {
                position_ms,
                source,
            })
        }
        ["timeline", "beat", "next"] | ["timeline", "next_beat"] => {
            is_positive_osc_trigger(message.args.first())
                .then_some(OscInputEvent::SeekTimelineBeat { direction: 1 })
        }
        ["timeline", "beat", "previous" | "prev" | "back"]
        | ["timeline", "previous_beat" | "prev_beat"] => {
            is_positive_osc_trigger(message.args.first())
                .then_some(OscInputEvent::SeekTimelineBeat { direction: -1 })
        }
        ["master"] | ["lighting", "master"] => message
            .args
            .first()
            .and_then(osc_value_to_f32)
            .map(OscInputEvent::LightingMaster),
        ["submaster", group_id] => {
            let level = message.args.first().and_then(osc_value_to_f32)?;
            Some(OscInputEvent::SetGroupSubmaster {
                group_id: (*group_id).to_string(),
                level,
            })
        }
        ["group", group_id, "highlight"] => {
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetGroupHighlight {
                group_id: (*group_id).to_string(),
                enabled,
            })
        }
        ["group", group_id, "solo"] => {
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetGroupSolo {
                group_id: (*group_id).to_string(),
                enabled,
            })
        }
        ["group", group_id, "park"] => {
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetGroupPark {
                group_id: (*group_id).to_string(),
                enabled,
            })
        }
        ["effect", effect_id, "enabled" | "enable"] => {
            let effect_id = effect_id.parse::<EffectId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetEffectEnabled { effect_id, enabled })
        }
        ["node", "graph", graph_id, "enabled" | "enable"]
        | ["node_graph", graph_id, "enabled" | "enable"]
        | ["graph", graph_id, "enabled" | "enable"] => {
            let graph_id = graph_id.parse::<NodeGraphId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetNodeGraphEnabled { graph_id, enabled })
        }
        ["video", "layer", layer_id, "enabled" | "enable"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetVideoLayerEnabled { layer_id, enabled })
        }
        ["video", "layer", layer_id, "solo"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let solo = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetVideoLayerSolo { layer_id, solo })
        }
        ["video", "layer", layer_id, "play" | "playing"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let playing = message.args.first().and_then(osc_value_to_bool)?;
            Some(OscInputEvent::SetVideoPlaying { layer_id, playing })
        }
        ["video", "layer", layer_id, "seek"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let position_ms = message.args.first().and_then(osc_value_to_u64)?;
            Some(OscInputEvent::SeekVideoLayer {
                layer_id,
                position_ms,
            })
        }
        ["video", "layer", layer_id, "loop"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            let loop_start_ms = message.args.get(1).and_then(osc_value_to_u64);
            let loop_end_ms = message.args.get(2).and_then(osc_value_to_u64);
            Some(OscInputEvent::SetVideoLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            })
        }
        ["video", "layer", layer_id, "fade"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let opacity = message.args.first().and_then(osc_value_to_f32)?;
            let duration_ms = message
                .args
                .get(1)
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoLayerOpacity {
                layer_id,
                opacity,
                duration_ms,
            })
        }
        ["video", "layer", layer_id, "fade_in" | "fadein"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let duration_ms = message
                .args
                .first()
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoLayerOpacity {
                layer_id,
                opacity: 1.0,
                duration_ms,
            })
        }
        ["video", "layer", layer_id, "fade_out" | "fadeout"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let duration_ms = message
                .args
                .first()
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoLayerOpacity {
                layer_id,
                opacity: 0.0,
                duration_ms,
            })
        }
        ["video", "layer", layer_id, "cue" | "cue_point", "add"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let position_ms = message.args.first().and_then(osc_value_to_u64);
            Some(OscInputEvent::AddVideoCuePoint {
                layer_id,
                position_ms,
            })
        }
        ["video", "layer", layer_id, "cue" | "cue_point", "remove"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let position_ms = message.args.first().and_then(osc_value_to_u64)?;
            Some(OscInputEvent::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            })
        }
        ["video", "layer", layer_id, "cue" | "cue_point", "jump"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let cue_point_index = message.args.first().and_then(osc_value_to_u64)? as usize;
            Some(OscInputEvent::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            })
        }
        ["video", "layer", layer_id, "cue" | "cue_point", "previous" | "prev" | "back"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            is_positive_osc_trigger(message.args.first()).then_some(
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id,
                    direction: -1,
                },
            )
        }
        ["video", "layer", layer_id, "cue" | "cue_point", "next"] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            is_positive_osc_trigger(message.args.first()).then_some(
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id,
                    direction: 1,
                },
            )
        }
        ["video", "layer", layer_id, param] => {
            let layer_id = layer_id.parse::<VideoLayerId>().ok()?;
            let param = video_param_from_str(param)?;
            let value = message.args.first().and_then(osc_value_to_f32)?;
            Some(OscInputEvent::SetVideoParam {
                layer_id,
                param,
                value,
            })
        }
        ["video", "output", output_id, "enabled"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let enabled = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetVideoOutputEnabled { output_id, enabled })
        }
        ["video", "output", output_id, "opacity"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let opacity = message.args.first().and_then(osc_value_to_f32)?;
            Some(OscInputEvent::SetVideoOutputOpacity { output_id, opacity })
        }
        ["video", "output", output_id, "fade"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let opacity = message.args.first().and_then(osc_value_to_f32)?;
            let duration_ms = message
                .args
                .get(1)
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            })
        }
        ["video", "output", output_id, "fade_in" | "fadein"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let duration_ms = message
                .args
                .first()
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoOutputOpacity {
                output_id,
                opacity: 1.0,
                duration_ms,
            })
        }
        ["video", "output", output_id, "fade_out" | "fadeout"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let duration_ms = message
                .args
                .first()
                .and_then(osc_value_to_u64)
                .unwrap_or(1_000);
            Some(OscInputEvent::FadeVideoOutputOpacity {
                output_id,
                opacity: 0.0,
                duration_ms,
            })
        }
        ["video", "output", output_id, "mapping" | "map", field] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let value = message.args.first().and_then(osc_value_to_f32)?;
            Some(OscInputEvent::SetVideoOutputMappingField {
                output_id,
                field: (*field).to_string(),
                value,
            })
        }
        ["video", "output", output_id, "blackout"] => {
            let output_id = output_id.parse::<VideoOutputId>().ok()?;
            let blackout = message
                .args
                .first()
                .and_then(osc_value_to_bool)
                .unwrap_or(true);
            Some(OscInputEvent::SetVideoOutputBlackout {
                output_id,
                blackout,
            })
        }
        ["video", "master"] => message
            .args
            .first()
            .and_then(osc_value_to_f32)
            .map(OscInputEvent::VideoMasterOpacity),
        ["video", "blackout"] => message
            .args
            .first()
            .and_then(osc_value_to_bool)
            .map(OscInputEvent::VideoBlackout),
        ["bpm"] => message
            .args
            .first()
            .and_then(osc_value_to_f32)
            .map(OscInputEvent::SetBpm),
        ["clock", "link"] | ["clock", "ableton_link"] | ["ableton_link"] => {
            external_clock_event(message, ClockSource::AbletonLink)
        }
        ["clock", "sync"] => {
            let source = message
                .args
                .get(2)
                .and_then(osc_value_to_clock_source)
                .unwrap_or(ClockSource::AbletonLink);
            external_clock_event(message, source)
        }
        ["tap"] => Some(OscInputEvent::TapBpm),
        _ => None,
    }
}

fn external_clock_event(message: &OscMessage, source: ClockSource) -> Option<OscInputEvent> {
    let bpm = message.args.first().and_then(finite_osc_f32)?;
    let beat_phase = message.args.get(1).and_then(finite_osc_f32).unwrap_or(0.0);
    Some(OscInputEvent::SyncExternalClock {
        bpm,
        beat_phase,
        source,
    })
}

fn events_from_mapped_message(
    message: &OscMessage,
    mappings: &[OscControlMapping],
) -> Vec<OscInputEvent> {
    let address = normalize_osc_address(&message.addr);
    mappings
        .iter()
        .filter(|mapping| osc_address_pattern_matches(&mapping.address, &address))
        .filter_map(|mapping| event_from_mapping(message, mapping))
        .collect()
}

fn event_from_mapping(message: &OscMessage, mapping: &OscControlMapping) -> Option<OscInputEvent> {
    let first_arg = message.args.first();
    let float_value = first_arg.and_then(osc_value_to_f32).unwrap_or(1.0);
    let ranged_value = if mapping.low.is_finite() && mapping.high.is_finite() {
        mapping.low + (mapping.high - mapping.low) * float_value
    } else {
        float_value
    };
    match mapping.action {
        OscControlAction::FixtureAttribute => Some(OscInputEvent::SetAttribute {
            fixture_id: mapping.fixture_id?,
            attribute: mapping.attribute.as_ref()?.clone(),
            value: ranged_value.round().clamp(0.0, 65_535.0) as u16,
        }),
        OscControlAction::FixtureHighlight => Some(OscInputEvent::SetFixtureHighlight {
            fixture_id: mapping.fixture_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::FixtureSolo => Some(OscInputEvent::SetFixtureSolo {
            fixture_id: mapping.fixture_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::FixturePark => Some(OscInputEvent::SetFixturePark {
            fixture_id: mapping.fixture_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::GroupHighlight => Some(OscInputEvent::SetGroupHighlight {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::GroupSolo => Some(OscInputEvent::SetGroupSolo {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::GroupPark => Some(OscInputEvent::SetGroupPark {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::TriggerCue => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::TriggerCue(mapping.cue_id?))
        }
        OscControlAction::TriggerNextCue => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::TriggerNextCue)
        }
        OscControlAction::TriggerPreviousCue => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::TriggerPreviousCue)
        }
        OscControlAction::EffectEnabled => Some(OscInputEvent::SetEffectEnabled {
            effect_id: mapping.cue_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::NodeGraphEnabled => Some(OscInputEvent::SetNodeGraphEnabled {
            graph_id: mapping.cue_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::VideoParam => Some(OscInputEvent::SetVideoParam {
            layer_id: mapping.layer_id?,
            param: mapping.video_param.clone()?,
            value: ranged_value,
        }),
        OscControlAction::VideoCuePointAdd => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::AddVideoCuePoint {
                layer_id: mapping.layer_id?,
                position_ms: mapping.duration_ms,
            })
        }
        OscControlAction::VideoCuePointRemove => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::RemoveVideoCuePoint {
                layer_id: mapping.layer_id?,
                position_ms: mapping.duration_ms?,
            })
        }
        OscControlAction::VideoCuePointJump => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::JumpVideoCuePoint {
                layer_id: mapping.layer_id?,
                cue_point_index: mapping.cue_point_index.unwrap_or(0),
            })
        }
        OscControlAction::VideoCuePointPrevious => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::JumpVideoCuePointRelative {
                layer_id: mapping.layer_id?,
                direction: -1,
            })
        }
        OscControlAction::VideoCuePointNext => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::JumpVideoCuePointRelative {
                layer_id: mapping.layer_id?,
                direction: 1,
            })
        }
        OscControlAction::VideoLayerEnabled => Some(OscInputEvent::SetVideoLayerEnabled {
            layer_id: mapping.layer_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::VideoLayerSolo => Some(OscInputEvent::SetVideoLayerSolo {
            layer_id: mapping.layer_id?,
            solo: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::VideoPlay => Some(OscInputEvent::SetVideoPlaying {
            layer_id: mapping.layer_id?,
            playing: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::VideoLoop => Some(OscInputEvent::SetVideoLoop {
            layer_id: mapping.layer_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
            loop_start_ms: finite_mapping_ms(mapping.low),
            loop_end_ms: finite_mapping_ms(mapping.high),
        }),
        OscControlAction::VideoLayerFade => Some(OscInputEvent::FadeVideoLayerOpacity {
            layer_id: mapping.layer_id?,
            opacity: ranged_value,
            duration_ms: mapping.duration_ms.unwrap_or(1_000),
        }),
        OscControlAction::VideoOutputEnabled => Some(OscInputEvent::SetVideoOutputEnabled {
            output_id: mapping.output_id?,
            enabled: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::VideoOutputOpacity => Some(OscInputEvent::SetVideoOutputOpacity {
            output_id: mapping.output_id?,
            opacity: ranged_value,
        }),
        OscControlAction::VideoOutputFade => Some(OscInputEvent::FadeVideoOutputOpacity {
            output_id: mapping.output_id?,
            opacity: ranged_value,
            duration_ms: mapping.duration_ms.unwrap_or(1_000),
        }),
        OscControlAction::VideoOutputMappingField => {
            Some(OscInputEvent::SetVideoOutputMappingField {
                output_id: mapping.output_id?,
                field: mapping.attribute.as_ref()?.clone(),
                value: ranged_value,
            })
        }
        OscControlAction::VideoOutputMappingPreset => is_positive_osc_trigger(first_arg).then_some(
            OscInputEvent::ApplyVideoOutputMappingPreset {
                output_id: mapping.output_id?,
                label: mapping.attribute.as_ref()?.clone(),
            },
        ),
        OscControlAction::VideoOutputBlackout => Some(OscInputEvent::SetVideoOutputBlackout {
            output_id: mapping.output_id?,
            blackout: first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        }),
        OscControlAction::TimelinePlay => Some(OscInputEvent::SetTimelinePlaying(
            first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        )),
        OscControlAction::TimelineSeek => Some(OscInputEvent::SeekTimeline {
            position_ms: ranged_value.max(0.0).round() as u64,
        }),
        OscControlAction::TimelineBeatPrevious => is_positive_osc_trigger(first_arg)
            .then_some(OscInputEvent::SeekTimelineBeat { direction: -1 }),
        OscControlAction::TimelineBeatNext => is_positive_osc_trigger(first_arg)
            .then_some(OscInputEvent::SeekTimelineBeat { direction: 1 }),
        OscControlAction::SetBpm => Some(OscInputEvent::SetBpm(ranged_value)),
        OscControlAction::TapBpm => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::TapBpm)
        }
        OscControlAction::LightingMaster => Some(OscInputEvent::LightingMaster(ranged_value)),
        OscControlAction::VideoMaster => Some(OscInputEvent::VideoMasterOpacity(ranged_value)),
        OscControlAction::GroupSubmaster => Some(OscInputEvent::SetGroupSubmaster {
            group_id: mapping.group_id.as_ref()?.clone(),
            level: ranged_value,
        }),
        OscControlAction::CueFadePause => Some(OscInputEvent::SetCueFadePaused(
            first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        )),
        OscControlAction::Blackout => Some(OscInputEvent::Blackout(
            first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        )),
        OscControlAction::AllBlackout => Some(OscInputEvent::AllBlackout(
            first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        )),
        OscControlAction::VideoBlackout => Some(OscInputEvent::VideoBlackout(
            first_arg.and_then(osc_value_to_bool).unwrap_or(true),
        )),
        OscControlAction::ClearFixtureFlags => {
            is_positive_osc_trigger(first_arg).then_some(OscInputEvent::ClearFixtureFlags {
                kind: mapping
                    .attribute
                    .as_deref()
                    .unwrap_or("all")
                    .trim()
                    .to_string(),
            })
        }
    }
}

fn normalize_osc_address(address: &str) -> String {
    let trimmed = address.trim().trim_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn osc_address_pattern_matches(pattern: &str, address: &str) -> bool {
    let pattern = normalize_osc_address(pattern);
    let address = normalize_osc_address(address);
    if pattern == address {
        return true;
    }
    let pattern_segments = osc_address_segments(&pattern);
    let address_segments = osc_address_segments(&address);
    if pattern_segments.len() != address_segments.len() {
        return false;
    }
    pattern_segments.iter().zip(address_segments.iter()).all(
        |(pattern_segment, address_segment)| {
            *pattern_segment == "*" || pattern_segment == address_segment
        },
    )
}

fn osc_address_segments(address: &str) -> Vec<&str> {
    let trimmed = address.trim().trim_matches('/');
    if trimmed.is_empty() {
        Vec::new()
    } else {
        trimmed.split('/').collect()
    }
}

fn is_positive_osc_trigger(value: Option<&OscType>) -> bool {
    value
        .and_then(osc_value_to_bool)
        .or_else(|| value.and_then(osc_value_to_f32).map(|value| value > 0.0))
        .unwrap_or(true)
}

fn finite_mapping_ms(value: f32) -> Option<u64> {
    value.is_finite().then(|| value.max(0.0).round() as u64)
}

fn video_param_from_str(value: &str) -> Option<VideoParam> {
    match value.to_ascii_lowercase().as_str() {
        "opacity" => Some(VideoParam::Opacity),
        "speed" => Some(VideoParam::Speed),
        "position" | "position_ms" | "positionms" => Some(VideoParam::PositionMs),
        "bpm_sync" | "bpm_sync_enabled" | "bpmsync" | "bpmsyncenabled" => {
            Some(VideoParam::BpmSyncEnabled)
        }
        "bpm_sync_ratio" | "bpmsyncratio" | "sync_ratio" | "syncratio" => {
            Some(VideoParam::BpmSyncRatio)
        }
        "bpm_sync_loop_bars" | "bpmsyncloopbars" | "loop_bars" | "loopbars" => {
            Some(VideoParam::BpmSyncLoopBars)
        }
        "transform_x" | "transformx" | "x" => Some(VideoParam::TransformX),
        "transform_y" | "transformy" | "y" => Some(VideoParam::TransformY),
        "scale_x" | "scalex" | "transform_scale_x" | "transformscalex" => {
            Some(VideoParam::TransformScaleX)
        }
        "scale_y" | "scaley" | "transform_scale_y" | "transformscaley" => {
            Some(VideoParam::TransformScaleY)
        }
        "rotation" | "rotation_deg" | "rotationdeg" | "transform_rotation_deg" => {
            Some(VideoParam::TransformRotationDeg)
        }
        "crop_left" | "cropleft" => Some(VideoParam::TransformCropLeft),
        "crop_top" | "croptop" => Some(VideoParam::TransformCropTop),
        "crop_right" | "cropright" => Some(VideoParam::TransformCropRight),
        "crop_bottom" | "cropbottom" => Some(VideoParam::TransformCropBottom),
        "brightness" | "color_brightness" | "colorbrightness" => Some(VideoParam::ColorBrightness),
        "contrast" | "color_contrast" | "colorcontrast" => Some(VideoParam::ColorContrast),
        "hue" | "hue_deg" | "huedeg" | "color_hue" | "color_hue_deg" => {
            Some(VideoParam::ColorHueDeg)
        }
        "saturation" | "sat" | "color_saturation" | "colorsaturation" => {
            Some(VideoParam::ColorSaturation)
        }
        "gamma" | "color_gamma" | "colorgamma" => Some(VideoParam::ColorGamma),
        "pixelate" | "fx_pixelate" | "fxpixelate" => Some(VideoParam::FxPixelate),
        "blur" | "fx_blur" | "fxblur" => Some(VideoParam::FxBlur),
        "glow" | "fx_glow" | "fxglow" => Some(VideoParam::FxGlow),
        "edge" | "edges" | "fx_edge" | "fxedge" => Some(VideoParam::FxEdge),
        "key_red" | "keyred" | "color_key_red" | "colorkeyred" => Some(VideoParam::FxKeyRed),
        "key_green" | "keygreen" | "color_key_green" | "colorkeygreen" => {
            Some(VideoParam::FxKeyGreen)
        }
        "key_blue" | "keyblue" | "color_key_blue" | "colorkeyblue" => Some(VideoParam::FxKeyBlue),
        "key_threshold" | "keythreshold" | "color_key_threshold" | "colorkeythreshold" => {
            Some(VideoParam::FxKeyThreshold)
        }
        _ => None,
    }
}

fn osc_value_to_u16(value: &OscType) -> Option<u16> {
    match value {
        OscType::Int(value) => Some((*value).clamp(0, 65_535) as u16),
        OscType::Long(value) => Some((*value).clamp(0, 65_535) as u16),
        OscType::Float(value) => Some(float_to_u16(*value as f64)),
        OscType::Double(value) => Some(float_to_u16(*value)),
        OscType::Bool(value) => Some(if *value { 65_535 } else { 0 }),
        _ => None,
    }
}

fn osc_value_to_bool(value: &OscType) -> Option<bool> {
    match value {
        OscType::Bool(value) => Some(*value),
        OscType::Int(value) => Some(*value != 0),
        OscType::Long(value) => Some(*value != 0),
        OscType::Float(value) => Some(*value != 0.0),
        OscType::Double(value) => Some(*value != 0.0),
        _ => None,
    }
}

fn osc_value_to_f32(value: &OscType) -> Option<f32> {
    match value {
        OscType::Int(value) => Some(*value as f32),
        OscType::Long(value) => Some(*value as f32),
        OscType::Float(value) => Some(*value),
        OscType::Double(value) => Some(*value as f32),
        _ => None,
    }
}

fn finite_osc_f32(value: &OscType) -> Option<f32> {
    osc_value_to_f32(value).filter(|value| value.is_finite())
}

fn osc_value_to_clock_source(value: &OscType) -> Option<ClockSource> {
    match value {
        OscType::String(value) => parse_clock_source_label(value),
        _ => None,
    }
}

fn osc_value_to_u64(value: &OscType) -> Option<u64> {
    match value {
        OscType::Int(value) => (*value >= 0).then_some(*value as u64),
        OscType::Long(value) => (*value >= 0).then_some(*value as u64),
        OscType::Float(value) if value.is_finite() && *value >= 0.0 => Some(value.round() as u64),
        OscType::Double(value) if value.is_finite() && *value >= 0.0 => Some(value.round() as u64),
        _ => None,
    }
}

fn osc_value_to_timecode_position_ms(value: &OscType) -> Option<u64> {
    match value {
        OscType::String(value) => parse_timecode_position_ms(value),
        _ => osc_value_to_u64(value),
    }
}

fn float_to_u16(value: f64) -> u16 {
    if !value.is_finite() {
        return 0;
    }
    let scaled = if (0.0..=1.0).contains(&value) {
        value * 65_535.0
    } else {
        value
    };
    scaled.round().clamp(0.0, 65_535.0) as u16
}

#[cfg(test)]
mod tests {
    use super::*;
    use rosc::{OscBundle, OscTime};

    #[test]
    fn maps_fixture_attribute_message_to_set_attribute_event() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/fixture/42/Dimmer".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/fixture/42/highlight".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/fixture/42/solo".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/fixture/42/park".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
            ],
        });

        assert_eq!(
            events_from_packet(&packet),
            vec![
                OscInputEvent::SetAttribute {
                    fixture_id: 42,
                    attribute: "Dimmer".to_string(),
                    value: 32_768,
                },
                OscInputEvent::SetFixtureHighlight {
                    fixture_id: 42,
                    enabled: true,
                },
                OscInputEvent::SetFixtureSolo {
                    fixture_id: 42,
                    enabled: false,
                },
                OscInputEvent::SetFixturePark {
                    fixture_id: 42,
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn maps_effect_enabled_message() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/effect/9/enabled".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/effect/10/enable".to_string(),
                    args: vec![OscType::Int(0)],
                }),
            ],
        });

        assert_eq!(
            events_from_packet(&packet),
            vec![
                OscInputEvent::SetEffectEnabled {
                    effect_id: 9,
                    enabled: true,
                },
                OscInputEvent::SetEffectEnabled {
                    effect_id: 10,
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn maps_node_graph_enabled_message() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/node/graph/12/enabled".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/graph/13/enable".to_string(),
                    args: vec![OscType::Int(0)],
                }),
            ],
        });

        assert_eq!(
            events_from_packet(&packet),
            vec![
                OscInputEvent::SetNodeGraphEnabled {
                    graph_id: 12,
                    enabled: true,
                },
                OscInputEvent::SetNodeGraphEnabled {
                    graph_id: 13,
                    enabled: false,
                },
            ]
        );
    }

    #[test]
    fn maps_global_clock_and_blackout_messages() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/all/blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/clear/fixture_flags".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/fixtures/clear_flags/park".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/cue/pause".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/cue/go".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/cue/back".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/cue/9".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/cue/10/go".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/play".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/seek".to_string(),
                    args: vec![OscType::Int(24_000)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/timecode".to_string(),
                    args: vec![
                        OscType::String("01:02:03:15@30".to_string()),
                        OscType::String("mtc".to_string()),
                    ],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/pause".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/beat/next".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/timeline/prev_beat".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/master".to_string(),
                    args: vec![OscType::Float(0.42)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/submaster/front".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/group/front/highlight".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/group/front/solo".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/group/front/park".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/bpm".to_string(),
                    args: vec![OscType::Float(128.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/clock/link".to_string(),
                    args: vec![OscType::Float(126.0), OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/clock/sync".to_string(),
                    args: vec![
                        OscType::Float(124.0),
                        OscType::Float(0.5),
                        OscType::String("ltc".to_string()),
                    ],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/tap".to_string(),
                    args: Vec::new(),
                }),
            ],
        });

        assert_eq!(
            events_from_packet(&packet),
            vec![
                OscInputEvent::Blackout(true),
                OscInputEvent::AllBlackout(true),
                OscInputEvent::ClearFixtureFlags {
                    kind: "all".to_string(),
                },
                OscInputEvent::ClearFixtureFlags {
                    kind: "park".to_string(),
                },
                OscInputEvent::SetCueFadePaused(true),
                OscInputEvent::TriggerNextCue,
                OscInputEvent::TriggerPreviousCue,
                OscInputEvent::TriggerCue(9),
                OscInputEvent::SetTimelinePlaying(true),
                OscInputEvent::SeekTimeline {
                    position_ms: 24_000,
                },
                OscInputEvent::SyncTimelineTimecode {
                    position_ms: 3_723_500,
                    source: ClockSource::MidiTimecode,
                },
                OscInputEvent::SetTimelinePlaying(false),
                OscInputEvent::SeekTimelineBeat { direction: 1 },
                OscInputEvent::SeekTimelineBeat { direction: -1 },
                OscInputEvent::LightingMaster(0.42),
                OscInputEvent::SetGroupSubmaster {
                    group_id: "front".to_string(),
                    level: 0.25,
                },
                OscInputEvent::SetGroupHighlight {
                    group_id: "front".to_string(),
                    enabled: true,
                },
                OscInputEvent::SetGroupSolo {
                    group_id: "front".to_string(),
                    enabled: false,
                },
                OscInputEvent::SetGroupPark {
                    group_id: "front".to_string(),
                    enabled: true,
                },
                OscInputEvent::SetBpm(128.5),
                OscInputEvent::SyncExternalClock {
                    bpm: 126.0,
                    beat_phase: 0.25,
                    source: ClockSource::AbletonLink,
                },
                OscInputEvent::SyncExternalClock {
                    bpm: 124.0,
                    beat_phase: 0.5,
                    source: ClockSource::Ltc,
                },
                OscInputEvent::TapBpm,
            ]
        );
    }

    #[test]
    fn maps_video_control_messages() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/opacity".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/transform_x".to_string(),
                    args: vec![OscType::Float(-0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/enabled".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/solo".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/bpm_sync".to_string(),
                    args: vec![OscType::Float(1.0)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/play".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/seek".to_string(),
                    args: vec![OscType::Int(1500)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/loop".to_string(),
                    args: vec![OscType::Bool(true), OscType::Int(500), OscType::Int(2500)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/fade".to_string(),
                    args: vec![OscType::Float(0.4), OscType::Int(900)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/fade_out".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/add".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/add".to_string(),
                    args: vec![OscType::Float(1750.4)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/jump".to_string(),
                    args: vec![OscType::Int(1)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/previous".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/next".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/layer/7/cue/remove".to_string(),
                    args: vec![OscType::Int(1750)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/enabled".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/opacity".to_string(),
                    args: vec![OscType::Float(0.6)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/fade".to_string(),
                    args: vec![OscType::Float(0.2), OscType::Int(750)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/fade_in".to_string(),
                    args: vec![OscType::Int(250)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/fade_out".to_string(),
                    args: Vec::new(),
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/mapping/keystone_x".to_string(),
                    args: vec![OscType::Float(0.35)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/map/stage_z".to_string(),
                    args: vec![OscType::Float(12.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/output/3/blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/master".to_string(),
                    args: vec![OscType::Float(0.75)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/syndocal/video/blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
            ],
        });

        assert_eq!(
            events_from_packet(&packet),
            vec![
                OscInputEvent::SetVideoParam {
                    layer_id: 7,
                    param: VideoParam::Opacity,
                    value: 0.5,
                },
                OscInputEvent::SetVideoParam {
                    layer_id: 7,
                    param: VideoParam::TransformX,
                    value: -0.25,
                },
                OscInputEvent::SetVideoLayerEnabled {
                    layer_id: 7,
                    enabled: false,
                },
                OscInputEvent::SetVideoLayerSolo {
                    layer_id: 7,
                    solo: true,
                },
                OscInputEvent::SetVideoParam {
                    layer_id: 7,
                    param: VideoParam::BpmSyncEnabled,
                    value: 1.0,
                },
                OscInputEvent::SetVideoPlaying {
                    layer_id: 7,
                    playing: true,
                },
                OscInputEvent::SeekVideoLayer {
                    layer_id: 7,
                    position_ms: 1500,
                },
                OscInputEvent::SetVideoLoop {
                    layer_id: 7,
                    enabled: true,
                    loop_start_ms: Some(500),
                    loop_end_ms: Some(2500),
                },
                OscInputEvent::FadeVideoLayerOpacity {
                    layer_id: 7,
                    opacity: 0.4,
                    duration_ms: 900,
                },
                OscInputEvent::FadeVideoLayerOpacity {
                    layer_id: 7,
                    opacity: 0.0,
                    duration_ms: 1_000,
                },
                OscInputEvent::AddVideoCuePoint {
                    layer_id: 7,
                    position_ms: None,
                },
                OscInputEvent::AddVideoCuePoint {
                    layer_id: 7,
                    position_ms: Some(1750),
                },
                OscInputEvent::JumpVideoCuePoint {
                    layer_id: 7,
                    cue_point_index: 1,
                },
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id: 7,
                    direction: -1,
                },
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id: 7,
                    direction: 1,
                },
                OscInputEvent::RemoveVideoCuePoint {
                    layer_id: 7,
                    position_ms: 1750,
                },
                OscInputEvent::SetVideoOutputEnabled {
                    output_id: 3,
                    enabled: false,
                },
                OscInputEvent::SetVideoOutputOpacity {
                    output_id: 3,
                    opacity: 0.6,
                },
                OscInputEvent::FadeVideoOutputOpacity {
                    output_id: 3,
                    opacity: 0.2,
                    duration_ms: 750,
                },
                OscInputEvent::FadeVideoOutputOpacity {
                    output_id: 3,
                    opacity: 1.0,
                    duration_ms: 250,
                },
                OscInputEvent::FadeVideoOutputOpacity {
                    output_id: 3,
                    opacity: 0.0,
                    duration_ms: 1_000,
                },
                OscInputEvent::SetVideoOutputMappingField {
                    output_id: 3,
                    field: "keystone_x".to_string(),
                    value: 0.35,
                },
                OscInputEvent::SetVideoOutputMappingField {
                    output_id: 3,
                    field: "stage_z".to_string(),
                    value: 12.5,
                },
                OscInputEvent::SetVideoOutputBlackout {
                    output_id: 3,
                    blackout: true,
                },
                OscInputEvent::VideoMasterOpacity(0.75),
                OscInputEvent::VideoBlackout(true),
            ]
        );
    }

    #[test]
    fn learns_first_osc_message_from_bundle() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![OscPacket::Message(OscMessage {
                addr: "touchosc/fader1".to_string(),
                args: vec![OscType::Float(0.25)],
            })],
        });

        assert_eq!(
            learned_control_from_packet(&packet),
            Some(LearnedOscControl {
                address: "/touchosc/fader1".to_string(),
                value: Some(0.25),
                argument_count: 1,
            })
        );
    }

    #[test]
    fn maps_custom_osc_video_master() {
        let packet = OscPacket::Message(OscMessage {
            addr: "/touchosc/video_master".to_string(),
            args: vec![OscType::Float(0.25)],
        });
        let mapping = OscControlMapping {
            address: "/touchosc/video_master".to_string(),
            action: OscControlAction::VideoMaster,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };

        assert_eq!(
            events_from_packet_with_mappings(&packet, &[mapping]),
            vec![OscInputEvent::VideoMasterOpacity(0.25)]
        );
    }

    #[test]
    fn maps_custom_osc_control_mappings() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/fader1".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/go".to_string(),
                    args: vec![OscType::Int(1)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/next".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/back".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_opacity".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_jump".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_cue_add".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_cue_remove".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_enable".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_solo".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_loop".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_fade".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_enable".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_opacity".to_string(),
                    args: vec![OscType::Float(0.6)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_fade".to_string(),
                    args: vec![OscType::Float(0.2)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_key".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_map_preset".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/output_blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/all_blackout".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/clear_flags".to_string(),
                    args: vec![OscType::Float(1.0)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/clear_park".to_string(),
                    args: vec![OscType::Float(1.0)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/master".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/front_sub".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/pause".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/timeline_play".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/timeline_seek".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/bpm".to_string(),
                    args: vec![OscType::Float(0.5)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/tap".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
            ],
        });
        let mappings = vec![
            OscControlMapping {
                address: "/touchosc/fader1".to_string(),
                action: OscControlAction::FixtureAttribute,
                fixture_id: Some(3),
                attribute: Some("Dimmer".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 65_535.0,
            },
            OscControlMapping {
                address: "touchosc/go".to_string(),
                action: OscControlAction::TriggerCue,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: Some(9),
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/next".to_string(),
                action: OscControlAction::TriggerNextCue,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/back".to_string(),
                action: OscControlAction::TriggerPreviousCue,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_opacity".to_string(),
                action: OscControlAction::VideoParam,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: Some(VideoParam::Opacity),
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_jump".to_string(),
                action: OscControlAction::VideoCuePointJump,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: Some(2),
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_cue_add".to_string(),
                action: OscControlAction::VideoCuePointAdd,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: Some(2_500),
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_cue_remove".to_string(),
                action: OscControlAction::VideoCuePointRemove,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: Some(2_500),
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_enable".to_string(),
                action: OscControlAction::VideoLayerEnabled,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_solo".to_string(),
                action: OscControlAction::VideoLayerSolo,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_loop".to_string(),
                action: OscControlAction::VideoLoop,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 500.0,
                high: 2_000.0,
            },
            OscControlMapping {
                address: "/touchosc/video_fade".to_string(),
                action: OscControlAction::VideoLayerFade,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: Some(600),
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_enable".to_string(),
                action: OscControlAction::VideoOutputEnabled,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_opacity".to_string(),
                action: OscControlAction::VideoOutputOpacity,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_fade".to_string(),
                action: OscControlAction::VideoOutputFade,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: Some(750),
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_key".to_string(),
                action: OscControlAction::VideoOutputMappingField,
                fixture_id: None,
                attribute: Some("key_h".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: None,
                low: -1.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_map_preset".to_string(),
                action: OscControlAction::VideoOutputMappingPreset,
                fixture_id: None,
                attribute: Some("Front Projector".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/output_blackout".to_string(),
                action: OscControlAction::VideoOutputBlackout,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: Some(8),
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/all_blackout".to_string(),
                action: OscControlAction::AllBlackout,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/clear_flags".to_string(),
                action: OscControlAction::ClearFixtureFlags,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/clear_park".to_string(),
                action: OscControlAction::ClearFixtureFlags,
                fixture_id: None,
                attribute: Some("park".to_string()),
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/master".to_string(),
                action: OscControlAction::LightingMaster,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/front_sub".to_string(),
                action: OscControlAction::GroupSubmaster,
                fixture_id: None,
                attribute: None,
                group_id: Some("front".to_string()),
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/pause".to_string(),
                action: OscControlAction::CueFadePause,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/timeline_play".to_string(),
                action: OscControlAction::TimelinePlay,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/timeline_seek".to_string(),
                action: OscControlAction::TimelineSeek,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 120_000.0,
            },
            OscControlMapping {
                address: "/touchosc/bpm".to_string(),
                action: OscControlAction::SetBpm,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 20.0,
                high: 300.0,
            },
            OscControlMapping {
                address: "/touchosc/tap".to_string(),
                action: OscControlAction::TapBpm,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
        ];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![
                OscInputEvent::SetAttribute {
                    fixture_id: 3,
                    attribute: "Dimmer".to_string(),
                    value: 32_768,
                },
                OscInputEvent::TriggerCue(9),
                OscInputEvent::TriggerNextCue,
                OscInputEvent::TriggerPreviousCue,
                OscInputEvent::SetVideoParam {
                    layer_id: 4,
                    param: VideoParam::Opacity,
                    value: 0.25,
                },
                OscInputEvent::JumpVideoCuePoint {
                    layer_id: 4,
                    cue_point_index: 2,
                },
                OscInputEvent::AddVideoCuePoint {
                    layer_id: 4,
                    position_ms: Some(2_500),
                },
                OscInputEvent::RemoveVideoCuePoint {
                    layer_id: 4,
                    position_ms: 2_500,
                },
                OscInputEvent::SetVideoLayerEnabled {
                    layer_id: 4,
                    enabled: false,
                },
                OscInputEvent::SetVideoLayerSolo {
                    layer_id: 4,
                    solo: true,
                },
                OscInputEvent::SetVideoLoop {
                    layer_id: 4,
                    enabled: true,
                    loop_start_ms: Some(500),
                    loop_end_ms: Some(2_000),
                },
                OscInputEvent::FadeVideoLayerOpacity {
                    layer_id: 4,
                    opacity: 0.5,
                    duration_ms: 600,
                },
                OscInputEvent::SetVideoOutputEnabled {
                    output_id: 8,
                    enabled: false,
                },
                OscInputEvent::SetVideoOutputOpacity {
                    output_id: 8,
                    opacity: 0.6,
                },
                OscInputEvent::FadeVideoOutputOpacity {
                    output_id: 8,
                    opacity: 0.2,
                    duration_ms: 750,
                },
                OscInputEvent::SetVideoOutputMappingField {
                    output_id: 8,
                    field: "key_h".to_string(),
                    value: -0.5,
                },
                OscInputEvent::ApplyVideoOutputMappingPreset {
                    output_id: 8,
                    label: "Front Projector".to_string(),
                },
                OscInputEvent::SetVideoOutputBlackout {
                    output_id: 8,
                    blackout: true,
                },
                OscInputEvent::AllBlackout(true),
                OscInputEvent::ClearFixtureFlags {
                    kind: "all".to_string(),
                },
                OscInputEvent::ClearFixtureFlags {
                    kind: "park".to_string(),
                },
                OscInputEvent::LightingMaster(0.5),
                OscInputEvent::SetGroupSubmaster {
                    group_id: "front".to_string(),
                    level: 0.25,
                },
                OscInputEvent::SetCueFadePaused(true),
                OscInputEvent::SetTimelinePlaying(false),
                OscInputEvent::SeekTimeline {
                    position_ms: 30_000,
                },
                OscInputEvent::SetBpm(160.0),
                OscInputEvent::TapBpm,
            ]
        );
    }

    #[test]
    fn maps_custom_osc_fixture_and_group_flag_controls() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/fixture_highlight".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/fixture_solo".to_string(),
                    args: vec![OscType::Float(0.0)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/fixture_park".to_string(),
                    args: vec![OscType::Int(1)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/group_highlight".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/group_solo".to_string(),
                    args: vec![OscType::Bool(false)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/group_park".to_string(),
                    args: vec![OscType::Float(1.0)],
                }),
            ],
        });
        let flag_mapping = |address: &str,
                            action: OscControlAction,
                            fixture_id: Option<FixtureId>,
                            group_id: Option<&str>| OscControlMapping {
            address: address.to_string(),
            action,
            fixture_id,
            attribute: None,
            group_id: group_id.map(str::to_string),
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let mappings = vec![
            flag_mapping(
                "/touchosc/fixture_highlight",
                OscControlAction::FixtureHighlight,
                Some(3),
                None,
            ),
            flag_mapping(
                "/touchosc/fixture_solo",
                OscControlAction::FixtureSolo,
                Some(3),
                None,
            ),
            flag_mapping(
                "/touchosc/fixture_park",
                OscControlAction::FixturePark,
                Some(3),
                None,
            ),
            flag_mapping(
                "/touchosc/group_highlight",
                OscControlAction::GroupHighlight,
                None,
                Some("front"),
            ),
            flag_mapping(
                "/touchosc/group_solo",
                OscControlAction::GroupSolo,
                None,
                Some("front"),
            ),
            flag_mapping(
                "/touchosc/group_park",
                OscControlAction::GroupPark,
                None,
                Some("/front/"),
            ),
        ];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![
                OscInputEvent::SetFixtureHighlight {
                    fixture_id: 3,
                    enabled: true,
                },
                OscInputEvent::SetFixtureSolo {
                    fixture_id: 3,
                    enabled: false,
                },
                OscInputEvent::SetFixturePark {
                    fixture_id: 3,
                    enabled: true,
                },
                OscInputEvent::SetGroupHighlight {
                    group_id: "front".to_string(),
                    enabled: true,
                },
                OscInputEvent::SetGroupSolo {
                    group_id: "front".to_string(),
                    enabled: false,
                },
                OscInputEvent::SetGroupPark {
                    group_id: "/front/".to_string(),
                    enabled: true,
                },
            ]
        );
    }

    #[test]
    fn maps_custom_osc_effect_enabled_control() {
        let packet = OscPacket::Message(OscMessage {
            addr: "/touchosc/effect/9".to_string(),
            args: vec![OscType::Float(0.0)],
        });
        let mappings = vec![OscControlMapping {
            address: "/touchosc/effect/9".to_string(),
            action: OscControlAction::EffectEnabled,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: Some(9),
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![OscInputEvent::SetEffectEnabled {
                effect_id: 9,
                enabled: false,
            }]
        );
    }

    #[test]
    fn maps_custom_osc_node_graph_enabled_control() {
        let packet = OscPacket::Message(OscMessage {
            addr: "/touchosc/graph/12".to_string(),
            args: vec![OscType::Float(1.0)],
        });
        let mappings = vec![OscControlMapping {
            address: "/touchosc/graph/12".to_string(),
            action: OscControlAction::NodeGraphEnabled,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: Some(12),
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![OscInputEvent::SetNodeGraphEnabled {
                graph_id: 12,
                enabled: true,
            }]
        );
    }

    #[test]
    fn maps_custom_osc_video_relative_cue_point_controls() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_prev_cue".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/video_next_cue".to_string(),
                    args: vec![OscType::Int(1)],
                }),
            ],
        });
        let mappings = vec![
            OscControlMapping {
                address: "/touchosc/video_prev_cue".to_string(),
                action: OscControlAction::VideoCuePointPrevious,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/video_next_cue".to_string(),
                action: OscControlAction::VideoCuePointNext,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: Some(4),
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
        ];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id: 4,
                    direction: -1,
                },
                OscInputEvent::JumpVideoCuePointRelative {
                    layer_id: 4,
                    direction: 1,
                },
            ]
        );
    }

    #[test]
    fn maps_custom_osc_timeline_beat_controls() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/beat_prev".to_string(),
                    args: vec![OscType::Bool(true)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/beat_next".to_string(),
                    args: vec![OscType::Int(1)],
                }),
            ],
        });
        let mappings = vec![
            OscControlMapping {
                address: "/touchosc/beat_prev".to_string(),
                action: OscControlAction::TimelineBeatPrevious,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
            OscControlMapping {
                address: "/touchosc/beat_next".to_string(),
                action: OscControlAction::TimelineBeatNext,
                fixture_id: None,
                attribute: None,
                group_id: None,
                cue_id: None,
                layer_id: None,
                video_param: None,
                cue_point_index: None,
                output_id: None,
                duration_ms: None,
                low: 0.0,
                high: 1.0,
            },
        ];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![
                OscInputEvent::SeekTimelineBeat { direction: -1 },
                OscInputEvent::SeekTimelineBeat { direction: 1 },
            ]
        );
    }

    #[test]
    fn maps_custom_osc_control_patterns_with_segment_wildcards() {
        let packet = OscPacket::Bundle(OscBundle {
            timetag: OscTime {
                seconds: 0,
                fractional: 1,
            },
            content: vec![
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/page/2/fader/1".to_string(),
                    args: vec![OscType::Float(0.25)],
                }),
                OscPacket::Message(OscMessage {
                    addr: "/touchosc/page/2/fader/1/extra".to_string(),
                    args: vec![OscType::Float(1.0)],
                }),
            ],
        });
        let mappings = vec![OscControlMapping {
            address: "/touchosc/page/*/fader/*".to_string(),
            action: OscControlAction::FixtureAttribute,
            fixture_id: Some(12),
            attribute: Some("Dimmer".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 65_535.0,
        }];

        assert_eq!(
            events_from_packet_with_mappings(&packet, &mappings),
            vec![OscInputEvent::SetAttribute {
                fixture_id: 12,
                attribute: "Dimmer".to_string(),
                value: 16_384,
            }]
        );
    }
}
