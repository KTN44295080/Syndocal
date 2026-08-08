use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use protocol::{
    video_output_mapping_field_value, CueId, EffectId, EngineSnapshot, FixtureId,
    LearnedMidiControl, MidiControlAction, MidiControlMapping, MidiControlMessage,
    MidiInputSummary, MidiOutputSummary, NodeGraphId, VideoLayerId, VideoLayerState, VideoOutputId,
    VideoParam,
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MidiError {
    #[error("failed to create MIDI input client: {0}")]
    Init(#[from] midir::InitError),
    #[error("failed to inspect MIDI input ports: {0}")]
    PortInfo(#[from] midir::PortInfoError),
    #[error("MIDI input port index {0} was not found")]
    MissingPort(usize),
    #[error("failed to connect MIDI input: {0}")]
    Connect(String),
    #[error("failed to send MIDI output: {0}")]
    Send(String),
}

pub struct MidiClockInput {
    _connection: MidiInputConnection<()>,
}

pub struct MidiControlInput {
    _connection: MidiInputConnection<()>,
}

pub struct MidiFeedbackOutput {
    connection: MidiOutputConnection,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MidiClockEvent {
    ClockPulse,
    Start,
    Continue,
    Stop,
    SongPositionPointer(u16),
    Timecode(MtcTimecode),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MtcFrameRate {
    Fps24,
    Fps25,
    Fps2997Drop,
    Fps30,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MtcTimecode {
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
    pub frames: u8,
    pub frame_rate: MtcFrameRate,
}

impl MtcFrameRate {
    fn fps(self) -> f64 {
        match self {
            Self::Fps24 => 24.0,
            Self::Fps25 => 25.0,
            Self::Fps2997Drop => 29.97,
            Self::Fps30 => 30.0,
        }
    }

    fn max_frame_count(self) -> u8 {
        match self {
            Self::Fps24 => 24,
            Self::Fps25 => 25,
            Self::Fps2997Drop | Self::Fps30 => 30,
        }
    }
}

impl MtcTimecode {
    pub fn position_ms(self) -> u64 {
        let seconds =
            u64::from(self.hours) * 3_600 + u64::from(self.minutes) * 60 + u64::from(self.seconds);
        let frame_ms = (f64::from(self.frames) * 1_000.0 / self.frame_rate.fps()).round() as u64;
        seconds * 1_000 + frame_ms
    }
}

#[derive(Debug, Clone, Default)]
struct MtcQuarterFrameDecoder {
    nibbles: [Option<u8>; 8],
    updated_mask: u8,
    last_timecode: Option<MtcTimecode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MidiControlEvent {
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
    SetVideoParam {
        layer_id: VideoLayerId,
        param: VideoParam,
        value: f32,
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
    SetTimelinePlaying(bool),
    SeekTimeline {
        position_ms: u64,
    },
    SeekTimelineBeat {
        direction: i32,
    },
    SetBpm(f32),
    TapBpm,
    LightingMaster(f32),
    SetGroupSubmaster {
        group_id: String,
        level: f32,
    },
    SetCueFadePaused(bool),
    Blackout(bool),
    AllBlackout(bool),
    VideoBlackout(bool),
    ClearFixtureFlags {
        kind: String,
    },
}

#[derive(Debug, Clone, PartialEq)]
struct MidiMessage {
    message: MidiControlMessage,
    channel: u8,
    number: u8,
    value: u8,
}

pub fn list_midi_inputs() -> Result<Vec<MidiInputSummary>, MidiError> {
    let input = MidiInput::new("syndocal-midi-list")?;
    input
        .ports()
        .iter()
        .enumerate()
        .map(|(index, port)| {
            Ok(MidiInputSummary {
                index,
                name: input.port_name(port)?,
            })
        })
        .collect()
}

pub fn list_midi_outputs() -> Result<Vec<MidiOutputSummary>, MidiError> {
    let output = MidiOutput::new("syndocal-midi-output-list")?;
    output
        .ports()
        .iter()
        .enumerate()
        .map(|(index, port)| {
            Ok(MidiOutputSummary {
                index,
                name: output.port_name(port)?,
            })
        })
        .collect()
}

pub fn connect_midi_feedback_output(port_index: usize) -> Result<MidiFeedbackOutput, MidiError> {
    let output = MidiOutput::new("syndocal-midi-feedback")?;
    let ports = output.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = output
        .connect(port, "syndocal-midi-feedback-output")
        .map_err(|error| MidiError::Connect(error.to_string()))?;
    Ok(MidiFeedbackOutput { connection })
}

pub fn connect_midi_clock<F>(
    port_index: usize,
    mut callback: F,
) -> Result<MidiClockInput, MidiError>
where
    F: FnMut(MidiClockEvent) + Send + 'static,
{
    let mut input = MidiInput::new("syndocal-midi-clock")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = input
        .connect(
            port,
            "syndocal-midi-clock-input",
            {
                let mut mtc_decoder = MtcQuarterFrameDecoder::default();
                move |_timestamp, message, _| {
                    for event in midi_clock_events_from_message(message, &mut mtc_decoder) {
                        callback(event);
                    }
                }
            },
            (),
        )
        .map_err(|error| MidiError::Connect(error.to_string()))?;

    Ok(MidiClockInput {
        _connection: connection,
    })
}

pub fn connect_midi_control<F>(
    port_index: usize,
    mappings: Vec<MidiControlMapping>,
    mut callback: F,
) -> Result<MidiControlInput, MidiError>
where
    F: FnMut(MidiControlEvent) + Send + 'static,
{
    let mut input = MidiInput::new("syndocal-midi-control")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = input
        .connect(
            port,
            "syndocal-midi-control-input",
            move |_timestamp, message, _| {
                for event in events_from_midi_message(message, &mappings) {
                    callback(event);
                }
            },
            (),
        )
        .map_err(|error| MidiError::Connect(error.to_string()))?;

    Ok(MidiControlInput {
        _connection: connection,
    })
}

pub fn learn_midi_control(
    port_index: usize,
    timeout: Duration,
) -> Result<Option<LearnedMidiControl>, MidiError> {
    let mut input = MidiInput::new("syndocal-midi-learn")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let (sender, receiver) = mpsc::channel::<LearnedMidiControl>();
    let connection = input
        .connect(
            port,
            "syndocal-midi-learn-input",
            move |_timestamp, message, _| {
                if let Some(learned) = learned_control_from_midi_message(message) {
                    let _ = sender.send(learned);
                }
            },
            (),
        )
        .map_err(|error| MidiError::Connect(error.to_string()))?;

    let deadline = Instant::now() + timeout;
    let result = loop {
        let now = Instant::now();
        if now >= deadline {
            break Ok(None);
        }
        match receiver.recv_timeout(deadline.saturating_duration_since(now)) {
            Ok(learned) => break Ok(Some(learned)),
            Err(mpsc::RecvTimeoutError::Timeout) => break Ok(None),
            Err(mpsc::RecvTimeoutError::Disconnected) => break Ok(None),
        }
    };
    drop(connection);
    result
}

pub fn learned_control_from_midi_message(message: &[u8]) -> Option<LearnedMidiControl> {
    let parsed = parse_midi_message(message)?;
    Some(LearnedMidiControl {
        channel: parsed.channel,
        message: parsed.message,
        number: parsed.number,
        value: parsed.value,
    })
}

impl MidiFeedbackOutput {
    pub fn send_feedback_messages(&mut self, messages: &[Vec<u8>]) -> Result<usize, MidiError> {
        for message in messages {
            self.connection
                .send(message)
                .map_err(|error| MidiError::Send(error.to_string()))?;
        }
        Ok(messages.len())
    }
}

pub fn build_feedback_messages(
    snapshot: &EngineSnapshot,
    mappings: &[MidiControlMapping],
) -> Vec<Vec<u8>> {
    mappings
        .iter()
        .filter_map(|mapping| {
            let normalized = feedback_value_for_mapping(snapshot, mapping)?;
            midi_feedback_message(mapping, normalized)
        })
        .collect()
}

pub fn events_from_midi_message(
    message: &[u8],
    mappings: &[MidiControlMapping],
) -> Vec<MidiControlEvent> {
    let Some(parsed) = parse_midi_message(message) else {
        return Vec::new();
    };
    mappings
        .iter()
        .filter(|mapping| mapping_matches(mapping, &parsed))
        .filter_map(|mapping| event_from_mapping(mapping, &parsed))
        .collect()
}

fn feedback_value_for_mapping(
    snapshot: &EngineSnapshot,
    mapping: &MidiControlMapping,
) -> Option<f32> {
    match mapping.action {
        MidiControlAction::FixtureAttribute => {
            let fixture_id = mapping.fixture_id?;
            let attribute = mapping.attribute.as_ref()?;
            let value = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)?
                .attribute_values
                .iter()
                .find(|candidate| candidate.attribute == *attribute)?
                .value as f32;
            Some(normalize_feedback_range(
                value,
                mapping.low,
                mapping.high,
                0.0,
                65_535.0,
            ))
        }
        MidiControlAction::FixtureHighlight => {
            let fixture_id = mapping.fixture_id?;
            let highlighted = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)?
                .highlighted;
            Some(if highlighted { 1.0 } else { 0.0 })
        }
        MidiControlAction::FixtureSolo => {
            let fixture_id = mapping.fixture_id?;
            let soloed = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)?
                .soloed;
            Some(if soloed { 1.0 } else { 0.0 })
        }
        MidiControlAction::FixturePark => {
            let fixture_id = mapping.fixture_id?;
            let parked = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)?
                .parked;
            Some(if parked { 1.0 } else { 0.0 })
        }
        MidiControlAction::GroupHighlight => {
            let group_id = mapping.group_id.as_ref()?;
            let highlighted = snapshot.fixtures.iter().any(|fixture| {
                fixture_matches_group(&fixture.group_ids, group_id) && fixture.highlighted
            });
            Some(if highlighted { 1.0 } else { 0.0 })
        }
        MidiControlAction::GroupSolo => {
            let group_id = mapping.group_id.as_ref()?;
            let soloed = snapshot.fixtures.iter().any(|fixture| {
                fixture_matches_group(&fixture.group_ids, group_id) && fixture.soloed
            });
            Some(if soloed { 1.0 } else { 0.0 })
        }
        MidiControlAction::GroupPark => {
            let group_id = mapping.group_id.as_ref()?;
            let parked = snapshot.fixtures.iter().any(|fixture| {
                fixture_matches_group(&fixture.group_ids, group_id) && fixture.parked
            });
            Some(if parked { 1.0 } else { 0.0 })
        }
        MidiControlAction::TriggerCue => {
            let cue_id = mapping.cue_id?;
            Some(if snapshot.active_cue_id == Some(cue_id) {
                1.0
            } else {
                0.0
            })
        }
        MidiControlAction::EffectEnabled => {
            let effect_id = mapping.cue_id?;
            let enabled = snapshot
                .effects
                .iter()
                .find(|effect| effect.id == effect_id)?
                .enabled;
            Some(if enabled { 1.0 } else { 0.0 })
        }
        MidiControlAction::NodeGraphEnabled => {
            let graph_id = mapping.cue_id?;
            let enabled = snapshot
                .node_graphs
                .iter()
                .find(|graph| graph.id == graph_id)?
                .enabled;
            Some(if enabled { 1.0 } else { 0.0 })
        }
        MidiControlAction::TriggerNextCue
        | MidiControlAction::TriggerPreviousCue
        | MidiControlAction::TimelineBeatPrevious
        | MidiControlAction::TimelineBeatNext
        | MidiControlAction::TapBpm
        | MidiControlAction::ClearFixtureFlags
        | MidiControlAction::VideoOutputMappingPreset => None,
        MidiControlAction::VideoParam => {
            let layer_id = mapping.layer_id?;
            let param = mapping.video_param.as_ref()?;
            let state = &snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state;
            let value = video_param_value(state, param);
            Some(normalize_feedback_range(
                value,
                mapping.low,
                mapping.high,
                0.0,
                1.0,
            ))
        }
        MidiControlAction::VideoCuePointAdd
        | MidiControlAction::VideoCuePointRemove
        | MidiControlAction::VideoCuePointJump
        | MidiControlAction::VideoCuePointPrevious
        | MidiControlAction::VideoCuePointNext => None,
        MidiControlAction::VideoLayerEnabled => {
            let layer_id = mapping.layer_id?;
            let enabled = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state
                .enabled;
            Some(if enabled { 1.0 } else { 0.0 })
        }
        MidiControlAction::VideoLayerSolo => {
            let layer_id = mapping.layer_id?;
            let solo = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state
                .solo;
            Some(if solo { 1.0 } else { 0.0 })
        }
        MidiControlAction::VideoPlay => {
            let layer_id = mapping.layer_id?;
            let playing = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state
                .playing;
            Some(if playing { 1.0 } else { 0.0 })
        }
        MidiControlAction::VideoLoop => {
            let layer_id = mapping.layer_id?;
            let loop_enabled = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state
                .loop_enabled;
            Some(if loop_enabled { 1.0 } else { 0.0 })
        }
        MidiControlAction::VideoLayerFade => {
            let layer_id = mapping.layer_id?;
            let opacity = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)?
                .state
                .opacity;
            Some(normalize_feedback_range(
                opacity,
                mapping.low,
                mapping.high,
                0.0,
                1.0,
            ))
        }
        MidiControlAction::VideoOutputEnabled => {
            let output_id = mapping.output_id?;
            let enabled = snapshot
                .video
                .outputs
                .iter()
                .find(|output| output.id == output_id)?
                .enabled;
            Some(if enabled { 1.0 } else { 0.0 })
        }
        MidiControlAction::VideoOutputOpacity | MidiControlAction::VideoOutputFade => {
            let output_id = mapping.output_id?;
            let opacity = snapshot
                .video
                .outputs
                .iter()
                .find(|output| output.id == output_id)?
                .opacity;
            Some(normalize_feedback_range(
                opacity,
                mapping.low,
                mapping.high,
                0.0,
                1.0,
            ))
        }
        MidiControlAction::VideoOutputMappingField => {
            let output_id = mapping.output_id?;
            let field = mapping.attribute.as_ref()?;
            let output_mapping = &snapshot
                .video
                .outputs
                .iter()
                .find(|output| output.id == output_id)?
                .mapping;
            let value = video_output_mapping_field_value(output_mapping, field)?;
            Some(normalize_feedback_range(
                value,
                mapping.low,
                mapping.high,
                -1.0,
                1.0,
            ))
        }
        MidiControlAction::VideoOutputBlackout => {
            let output_id = mapping.output_id?;
            let blackout = snapshot
                .video
                .outputs
                .iter()
                .find(|output| output.id == output_id)?
                .blackout;
            Some(if blackout { 1.0 } else { 0.0 })
        }
        MidiControlAction::TimelinePlay => Some(if snapshot.timeline.playing { 1.0 } else { 0.0 }),
        MidiControlAction::TimelineSeek => Some(normalize_feedback_range(
            snapshot.timeline.position_ms as f32,
            mapping.low,
            mapping.high,
            0.0,
            snapshot.timeline.duration_ms.max(1) as f32,
        )),
        MidiControlAction::SetBpm => Some(normalize_feedback_range(
            snapshot.clock.bpm,
            mapping.low,
            mapping.high,
            20.0,
            300.0,
        )),
        MidiControlAction::LightingMaster => Some(snapshot.lighting_master),
        MidiControlAction::GroupSubmaster => {
            let group_id = mapping.group_id.as_ref()?;
            let level = snapshot
                .submasters
                .iter()
                .find(|submaster| submaster.group_id == *group_id)?
                .level;
            Some(level)
        }
        MidiControlAction::CueFadePause => Some(
            if snapshot
                .active_fade
                .as_ref()
                .map(|fade| fade.paused)
                .unwrap_or(false)
            {
                1.0
            } else {
                0.0
            },
        ),
        MidiControlAction::Blackout => Some(if snapshot.blackout { 1.0 } else { 0.0 }),
        MidiControlAction::AllBlackout => Some(if snapshot.blackout && snapshot.video.blackout {
            1.0
        } else {
            0.0
        }),
        MidiControlAction::VideoBlackout => Some(if snapshot.video.blackout { 1.0 } else { 0.0 }),
    }
}

fn midi_feedback_message(mapping: &MidiControlMapping, normalized: f32) -> Option<Vec<u8>> {
    let channel = mapping.channel.unwrap_or(0).min(15);
    let number = mapping.number.min(127);
    let value = (normalized.clamp(0.0, 1.0) * 127.0).round() as u8;
    match mapping.message {
        MidiControlMessage::NoteOn => Some(vec![0x90 | channel, number, value]),
        MidiControlMessage::NoteOff => Some(vec![0x80 | channel, number, value]),
        MidiControlMessage::ControlChange => Some(vec![0xb0 | channel, number, value]),
        MidiControlMessage::ProgramChange => None,
    }
}

fn normalize_feedback_range(
    value: f32,
    low: f32,
    high: f32,
    fallback_low: f32,
    fallback_high: f32,
) -> f32 {
    let (low, high) = if low.is_finite() && high.is_finite() && (high - low).abs() > f32::EPSILON {
        (low, high)
    } else {
        (fallback_low, fallback_high)
    };
    ((value - low) / (high - low)).clamp(0.0, 1.0)
}

fn video_param_value(state: &VideoLayerState, param: &VideoParam) -> f32 {
    match param {
        VideoParam::Opacity => state.opacity,
        VideoParam::Speed => state.speed,
        VideoParam::PositionMs => state.position_ms as f32,
        VideoParam::BpmSyncEnabled => {
            if state.bpm_sync.enabled {
                1.0
            } else {
                0.0
            }
        }
        VideoParam::BpmSyncRatio => state.bpm_sync.ratio,
        VideoParam::BpmSyncLoopBars => state.bpm_sync.loop_bars,
        VideoParam::TransformX => state.transform.x,
        VideoParam::TransformY => state.transform.y,
        VideoParam::TransformScaleX => state.transform.scale_x,
        VideoParam::TransformScaleY => state.transform.scale_y,
        VideoParam::TransformRotationDeg => state.transform.rotation_deg,
        VideoParam::TransformCropLeft => state.transform.crop_left,
        VideoParam::TransformCropTop => state.transform.crop_top,
        VideoParam::TransformCropRight => state.transform.crop_right,
        VideoParam::TransformCropBottom => state.transform.crop_bottom,
        VideoParam::ColorBrightness => state.color.brightness,
        VideoParam::ColorContrast => state.color.contrast,
        VideoParam::ColorHueDeg => state.color.hue_deg,
        VideoParam::ColorSaturation => state.color.saturation,
        VideoParam::ColorGamma => state.color.gamma,
        VideoParam::FxPixelate => state.fx.pixelate,
        VideoParam::FxBlur => state.fx.blur,
        VideoParam::FxGlow => state.fx.glow,
        VideoParam::FxEdge => state.fx.edge,
        VideoParam::FxKeyRed => state.fx.key_red,
        VideoParam::FxKeyGreen => state.fx.key_green,
        VideoParam::FxKeyBlue => state.fx.key_blue,
        VideoParam::FxKeyThreshold => state.fx.key_threshold,
    }
}

fn parse_midi_message(message: &[u8]) -> Option<MidiMessage> {
    let status = *message.first()?;
    let message_type = status & 0xf0;
    let channel = status & 0x0f;
    match message_type {
        0x80 => Some(MidiMessage {
            message: MidiControlMessage::NoteOff,
            channel,
            number: *message.get(1)?,
            value: *message.get(2).unwrap_or(&0),
        }),
        0x90 => {
            let value = *message.get(2).unwrap_or(&0);
            Some(MidiMessage {
                message: if value == 0 {
                    MidiControlMessage::NoteOff
                } else {
                    MidiControlMessage::NoteOn
                },
                channel,
                number: *message.get(1)?,
                value,
            })
        }
        0xb0 => Some(MidiMessage {
            message: MidiControlMessage::ControlChange,
            channel,
            number: *message.get(1)?,
            value: *message.get(2).unwrap_or(&0),
        }),
        0xc0 => Some(MidiMessage {
            message: MidiControlMessage::ProgramChange,
            channel,
            number: *message.get(1)?,
            value: 127,
        }),
        _ => None,
    }
}

fn midi_clock_events_from_message(
    message: &[u8],
    mtc_decoder: &mut MtcQuarterFrameDecoder,
) -> Vec<MidiClockEvent> {
    let mut events = Vec::new();
    for byte in message {
        match *byte {
            0xf8 => events.push(MidiClockEvent::ClockPulse),
            0xfa => events.push(MidiClockEvent::Start),
            0xfb => events.push(MidiClockEvent::Continue),
            0xfc => events.push(MidiClockEvent::Stop),
            _ => {}
        }
    }
    if let Some(position) = song_position_pointer(message) {
        events.push(MidiClockEvent::SongPositionPointer(position));
    }
    if let Some(timecode) = mtc_decoder.push_message(message) {
        events.push(MidiClockEvent::Timecode(timecode));
    }
    if let Some(timecode) = mtc_full_frame_timecode(message) {
        events.push(MidiClockEvent::Timecode(timecode));
    }
    events
}

fn song_position_pointer(message: &[u8]) -> Option<u16> {
    if message.first().copied() != Some(0xf2) {
        return None;
    }
    let lsb = *message.get(1)?;
    let msb = *message.get(2)?;
    if lsb > 0x7f || msb > 0x7f {
        return None;
    }
    Some(u16::from(lsb) | (u16::from(msb) << 7))
}

fn mtc_full_frame_timecode(message: &[u8]) -> Option<MtcTimecode> {
    if message.len() != 10
        || message.first().copied() != Some(0xf0)
        || message.get(1).copied() != Some(0x7f)
        || message.get(3).copied() != Some(0x01)
        || message.get(4).copied() != Some(0x01)
        || message.get(9).copied() != Some(0xf7)
    {
        return None;
    }

    let hour_and_rate = message[5];
    let frame_rate = match (hour_and_rate >> 5) & 0x03 {
        0 => MtcFrameRate::Fps24,
        1 => MtcFrameRate::Fps25,
        2 => MtcFrameRate::Fps2997Drop,
        _ => MtcFrameRate::Fps30,
    };
    let hours = hour_and_rate & 0x1f;
    let minutes = message[6];
    let seconds = message[7];
    let frames = message[8];
    if hours >= 24 || minutes >= 60 || seconds >= 60 || frames >= frame_rate.max_frame_count() {
        return None;
    }

    Some(MtcTimecode {
        hours,
        minutes,
        seconds,
        frames,
        frame_rate,
    })
}

impl MtcQuarterFrameDecoder {
    fn push_message(&mut self, message: &[u8]) -> Option<MtcTimecode> {
        if message.first().copied() != Some(0xf1) {
            return None;
        }
        let data = *message.get(1)?;
        if data > 0x7f {
            return None;
        }
        let piece = ((data >> 4) & 0x07) as usize;
        let value = data & 0x0f;
        self.nibbles[piece] = Some(value);
        self.updated_mask |= 1 << piece;
        if self.updated_mask != 0xff {
            return None;
        }
        self.updated_mask = 0;
        let timecode = self.timecode()?;
        if self.last_timecode == Some(timecode) {
            return None;
        }
        self.last_timecode = Some(timecode);
        Some(timecode)
    }

    fn timecode(&self) -> Option<MtcTimecode> {
        let frame_low = self.nibbles[0]?;
        let frame_high = self.nibbles[1]?;
        let second_low = self.nibbles[2]?;
        let second_high = self.nibbles[3]?;
        let minute_low = self.nibbles[4]?;
        let minute_high = self.nibbles[5]?;
        let hour_low = self.nibbles[6]?;
        let hour_high_and_rate = self.nibbles[7]?;

        let frame_rate = match (hour_high_and_rate >> 1) & 0x03 {
            0 => MtcFrameRate::Fps24,
            1 => MtcFrameRate::Fps25,
            2 => MtcFrameRate::Fps2997Drop,
            _ => MtcFrameRate::Fps30,
        };
        let frames = frame_low | ((frame_high & 0x01) << 4);
        let seconds = second_low | ((second_high & 0x03) << 4);
        let minutes = minute_low | ((minute_high & 0x03) << 4);
        let hours = hour_low | ((hour_high_and_rate & 0x01) << 4);
        if frames >= frame_rate.max_frame_count() || seconds >= 60 || minutes >= 60 || hours >= 24 {
            return None;
        }
        Some(MtcTimecode {
            hours,
            minutes,
            seconds,
            frames,
            frame_rate,
        })
    }
}

fn mapping_matches(mapping: &MidiControlMapping, message: &MidiMessage) -> bool {
    mapping.message == message.message
        && mapping.number == message.number
        && mapping
            .channel
            .map(|channel| channel == message.channel)
            .unwrap_or(true)
}

fn event_from_mapping(
    mapping: &MidiControlMapping,
    message: &MidiMessage,
) -> Option<MidiControlEvent> {
    let normalized = f32::from(message.value) / 127.0;
    let ranged_value = if mapping.low.is_finite() && mapping.high.is_finite() {
        mapping.low + (mapping.high - mapping.low) * normalized
    } else {
        normalized
    };
    match mapping.action {
        MidiControlAction::FixtureAttribute => Some(MidiControlEvent::SetAttribute {
            fixture_id: mapping.fixture_id?,
            attribute: mapping.attribute.as_ref()?.clone(),
            value: (ranged_value.round()).clamp(0.0, 65_535.0) as u16,
        }),
        MidiControlAction::FixtureHighlight => Some(MidiControlEvent::SetFixtureHighlight {
            fixture_id: mapping.fixture_id?,
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::FixtureSolo => Some(MidiControlEvent::SetFixtureSolo {
            fixture_id: mapping.fixture_id?,
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::FixturePark => Some(MidiControlEvent::SetFixturePark {
            fixture_id: mapping.fixture_id?,
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::GroupHighlight => Some(MidiControlEvent::SetGroupHighlight {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::GroupSolo => Some(MidiControlEvent::SetGroupSolo {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::GroupPark => Some(MidiControlEvent::SetGroupPark {
            group_id: mapping.group_id.as_ref()?.clone(),
            enabled: midi_message_enabled(message),
        }),
        MidiControlAction::TriggerCue => {
            if is_positive_trigger(message) {
                Some(MidiControlEvent::TriggerCue(mapping.cue_id?))
            } else {
                None
            }
        }
        MidiControlAction::TriggerNextCue => {
            is_positive_trigger(message).then_some(MidiControlEvent::TriggerNextCue)
        }
        MidiControlAction::TriggerPreviousCue => {
            is_positive_trigger(message).then_some(MidiControlEvent::TriggerPreviousCue)
        }
        MidiControlAction::EffectEnabled => Some(MidiControlEvent::SetEffectEnabled {
            effect_id: mapping.cue_id?,
            enabled: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::NodeGraphEnabled => Some(MidiControlEvent::SetNodeGraphEnabled {
            graph_id: mapping.cue_id?,
            enabled: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::VideoParam => Some(MidiControlEvent::SetVideoParam {
            layer_id: mapping.layer_id?,
            param: mapping.video_param.clone()?,
            value: ranged_value,
        }),
        MidiControlAction::VideoCuePointAdd => {
            is_positive_trigger(message).then_some(MidiControlEvent::AddVideoCuePoint {
                layer_id: mapping.layer_id?,
                position_ms: mapping.duration_ms,
            })
        }
        MidiControlAction::VideoCuePointRemove => {
            is_positive_trigger(message).then_some(MidiControlEvent::RemoveVideoCuePoint {
                layer_id: mapping.layer_id?,
                position_ms: mapping.duration_ms?,
            })
        }
        MidiControlAction::VideoCuePointJump => {
            if is_positive_trigger(message) {
                Some(MidiControlEvent::JumpVideoCuePoint {
                    layer_id: mapping.layer_id?,
                    cue_point_index: mapping.cue_point_index.unwrap_or(0),
                })
            } else {
                None
            }
        }
        MidiControlAction::VideoCuePointPrevious => {
            is_positive_trigger(message).then_some(MidiControlEvent::JumpVideoCuePointRelative {
                layer_id: mapping.layer_id?,
                direction: -1,
            })
        }
        MidiControlAction::VideoCuePointNext => {
            is_positive_trigger(message).then_some(MidiControlEvent::JumpVideoCuePointRelative {
                layer_id: mapping.layer_id?,
                direction: 1,
            })
        }
        MidiControlAction::VideoLayerEnabled => Some(MidiControlEvent::SetVideoLayerEnabled {
            layer_id: mapping.layer_id?,
            enabled: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::VideoLayerSolo => Some(MidiControlEvent::SetVideoLayerSolo {
            layer_id: mapping.layer_id?,
            solo: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::VideoPlay => Some(MidiControlEvent::SetVideoPlaying {
            layer_id: mapping.layer_id?,
            playing: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::VideoLoop => Some(MidiControlEvent::SetVideoLoop {
            layer_id: mapping.layer_id?,
            enabled: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
            loop_start_ms: finite_mapping_ms(mapping.low),
            loop_end_ms: finite_mapping_ms(mapping.high),
        }),
        MidiControlAction::VideoLayerFade => Some(MidiControlEvent::FadeVideoLayerOpacity {
            layer_id: mapping.layer_id?,
            opacity: ranged_value,
            duration_ms: mapping.duration_ms.unwrap_or(1_000),
        }),
        MidiControlAction::VideoOutputEnabled => Some(MidiControlEvent::SetVideoOutputEnabled {
            output_id: mapping.output_id?,
            enabled: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::VideoOutputOpacity => Some(MidiControlEvent::SetVideoOutputOpacity {
            output_id: mapping.output_id?,
            opacity: ranged_value,
        }),
        MidiControlAction::VideoOutputFade => Some(MidiControlEvent::FadeVideoOutputOpacity {
            output_id: mapping.output_id?,
            opacity: ranged_value,
            duration_ms: mapping.duration_ms.unwrap_or(1_000),
        }),
        MidiControlAction::VideoOutputMappingField => {
            Some(MidiControlEvent::SetVideoOutputMappingField {
                output_id: mapping.output_id?,
                field: mapping.attribute.as_ref()?.clone(),
                value: ranged_value,
            })
        }
        MidiControlAction::VideoOutputMappingPreset => is_positive_trigger(message).then_some(
            MidiControlEvent::ApplyVideoOutputMappingPreset {
                output_id: mapping.output_id?,
                label: mapping.attribute.as_ref()?.clone(),
            },
        ),
        MidiControlAction::VideoOutputBlackout => Some(MidiControlEvent::SetVideoOutputBlackout {
            output_id: mapping.output_id?,
            blackout: match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        }),
        MidiControlAction::TimelinePlay => Some(MidiControlEvent::SetTimelinePlaying(
            match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            },
        )),
        MidiControlAction::TimelineSeek => Some(MidiControlEvent::SeekTimeline {
            position_ms: ranged_value.max(0.0).round() as u64,
        }),
        MidiControlAction::TimelineBeatPrevious => is_positive_trigger(message)
            .then_some(MidiControlEvent::SeekTimelineBeat { direction: -1 }),
        MidiControlAction::TimelineBeatNext => is_positive_trigger(message)
            .then_some(MidiControlEvent::SeekTimelineBeat { direction: 1 }),
        MidiControlAction::SetBpm => Some(MidiControlEvent::SetBpm(ranged_value)),
        MidiControlAction::TapBpm => {
            is_positive_trigger(message).then_some(MidiControlEvent::TapBpm)
        }
        MidiControlAction::LightingMaster => Some(MidiControlEvent::LightingMaster(ranged_value)),
        MidiControlAction::GroupSubmaster => Some(MidiControlEvent::SetGroupSubmaster {
            group_id: mapping.group_id.as_ref()?.clone(),
            level: ranged_value,
        }),
        MidiControlAction::CueFadePause => {
            Some(MidiControlEvent::SetCueFadePaused(match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            }))
        }
        MidiControlAction::Blackout => Some(MidiControlEvent::Blackout(match message.message {
            MidiControlMessage::NoteOff => false,
            _ => message.value > 0,
        })),
        MidiControlAction::AllBlackout => {
            Some(MidiControlEvent::AllBlackout(match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            }))
        }
        MidiControlAction::VideoBlackout => {
            Some(MidiControlEvent::VideoBlackout(match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            }))
        }
        MidiControlAction::ClearFixtureFlags => {
            is_positive_trigger(message).then_some(MidiControlEvent::ClearFixtureFlags {
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

fn is_positive_trigger(message: &MidiMessage) -> bool {
    !matches!(message.message, MidiControlMessage::NoteOff) && message.value > 0
}

fn midi_message_enabled(message: &MidiMessage) -> bool {
    match message.message {
        MidiControlMessage::NoteOff => false,
        _ => message.value > 0,
    }
}

fn finite_mapping_ms(value: f32) -> Option<u64> {
    value.is_finite().then(|| value.max(0.0).round() as u64)
}

fn fixture_matches_group(fixture_groups: &[String], requested_group_id: &str) -> bool {
    let requested = normalize_group_path(requested_group_id);
    !requested.is_empty()
        && fixture_groups.iter().any(|group| {
            let group = normalize_group_path(group);
            group == requested || group.starts_with(&format!("{requested}/"))
        })
}

fn normalize_group_path(group_id: &str) -> String {
    group_id
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.trim().is_empty())
        .map(str::trim)
        .collect::<Vec<_>>()
        .join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_cc_mapping() -> MidiControlMapping {
        MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 7,
            action: MidiControlAction::FixtureAttribute,
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
        }
    }

    fn fixture_flag_mapping(
        action: MidiControlAction,
        number: u8,
        fixture_id: FixtureId,
    ) -> MidiControlMapping {
        MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number,
            action,
            fixture_id: Some(fixture_id),
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
        }
    }

    fn group_flag_mapping(
        action: MidiControlAction,
        number: u8,
        group_id: &str,
    ) -> MidiControlMapping {
        MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number,
            action,
            fixture_id: None,
            attribute: None,
            group_id: Some(group_id.to_string()),
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        }
    }

    fn flagged_fixture(
        id: FixtureId,
        group_ids: &[&str],
        highlighted: bool,
        soloed: bool,
        parked: bool,
    ) -> protocol::PatchedFixtureSummary {
        protocol::PatchedFixtureSummary {
            id,
            label: format!("Fixture {id}"),
            profile_source_path: "memory://fixture.gdtf".to_string(),
            profile_name: "Mini Spot".to_string(),
            manufacturer: "Syndocal".to_string(),
            mode_name: "Standard".to_string(),
            universe: 1,
            address: 1,
            group_ids: group_ids.iter().map(|group| group.to_string()).collect(),
            position: protocol::Vec3::default(),
            rotation: protocol::Rotation3::default(),
            geometries: Vec::new(),
            controls: Vec::new(),
            attribute_values: Vec::new(),
            limits: protocol::FixtureLimits::default(),
            highlighted,
            soloed,
            parked,
        }
    }

    #[test]
    fn maps_control_change_to_fixture_attribute() {
        assert_eq!(
            events_from_midi_message(&[0xb0, 7, 64], &[fixture_cc_mapping()]),
            vec![MidiControlEvent::SetAttribute {
                fixture_id: 3,
                attribute: "Dimmer".to_string(),
                value: 33_026,
            }]
        );
    }

    #[test]
    fn extracts_learned_control_from_midi_message() {
        assert_eq!(
            learned_control_from_midi_message(&[0xb3, 74, 12]),
            Some(LearnedMidiControl {
                channel: 3,
                message: MidiControlMessage::ControlChange,
                number: 74,
                value: 12,
            })
        );
        assert_eq!(
            learned_control_from_midi_message(&[0xc2, 5]),
            Some(LearnedMidiControl {
                channel: 2,
                message: MidiControlMessage::ProgramChange,
                number: 5,
                value: 127,
            })
        );
        assert_eq!(learned_control_from_midi_message(&[0xf8]), None);
    }

    #[test]
    fn decodes_mtc_quarter_frame_timecode_to_timeline_position() {
        let mut decoder = MtcQuarterFrameDecoder::default();
        let messages = [
            [0xf1, 0x04],
            [0xf1, 0x10],
            [0xf1, 0x23],
            [0xf1, 0x30],
            [0xf1, 0x42],
            [0xf1, 0x50],
            [0xf1, 0x61],
            [0xf1, 0x72],
        ];

        for message in &messages[..7] {
            assert_eq!(decoder.push_message(message), None);
        }
        let timecode = decoder.push_message(&messages[7]).unwrap();

        assert_eq!(
            timecode,
            MtcTimecode {
                hours: 1,
                minutes: 2,
                seconds: 3,
                frames: 4,
                frame_rate: MtcFrameRate::Fps25,
            }
        );
        assert_eq!(timecode.position_ms(), 3_723_160);
        assert_eq!(decoder.push_message(&messages[7]), None);
    }

    #[test]
    fn suppresses_partial_mtc_quarter_frame_rollover_until_full_refresh() {
        let mut decoder = MtcQuarterFrameDecoder::default();
        let initial = [
            [0xf1, 0x08],
            [0xf1, 0x11],
            [0xf1, 0x2b],
            [0xf1, 0x33],
            [0xf1, 0x40],
            [0xf1, 0x50],
            [0xf1, 0x60],
            [0xf1, 0x72],
        ];
        let next = [
            [0xf1, 0x00],
            [0xf1, 0x10],
            [0xf1, 0x20],
            [0xf1, 0x30],
            [0xf1, 0x41],
            [0xf1, 0x50],
            [0xf1, 0x60],
            [0xf1, 0x72],
        ];

        for message in &initial[..7] {
            assert_eq!(decoder.push_message(message), None);
        }
        assert_eq!(
            decoder.push_message(&initial[7]),
            Some(MtcTimecode {
                hours: 0,
                minutes: 0,
                seconds: 59,
                frames: 24,
                frame_rate: MtcFrameRate::Fps25,
            })
        );

        for message in &next[..7] {
            assert_eq!(decoder.push_message(message), None);
        }
        assert_eq!(
            decoder.push_message(&next[7]),
            Some(MtcTimecode {
                hours: 0,
                minutes: 1,
                seconds: 0,
                frames: 0,
                frame_rate: MtcFrameRate::Fps25,
            })
        );
    }

    #[test]
    fn extracts_midi_clock_transport_realtime_events() {
        let mut decoder = MtcQuarterFrameDecoder::default();

        assert_eq!(
            midi_clock_events_from_message(&[0xfa], &mut decoder),
            vec![MidiClockEvent::Start]
        );
        assert_eq!(
            midi_clock_events_from_message(&[0xfb], &mut decoder),
            vec![MidiClockEvent::Continue]
        );
        assert_eq!(
            midi_clock_events_from_message(&[0xfc], &mut decoder),
            vec![MidiClockEvent::Stop]
        );
        assert_eq!(
            midi_clock_events_from_message(&[0xf8], &mut decoder),
            vec![MidiClockEvent::ClockPulse]
        );
        assert_eq!(
            midi_clock_events_from_message(&[0xf2, 0x10, 0x00], &mut decoder),
            vec![MidiClockEvent::SongPositionPointer(16)]
        );
        assert!(midi_clock_events_from_message(&[0xf2, 0x10], &mut decoder).is_empty());
    }

    #[test]
    fn decodes_mtc_full_frame_sysex_timecode() {
        let mut decoder = MtcQuarterFrameDecoder::default();
        let message = [0xf0, 0x7f, 0x7f, 0x01, 0x01, 0x21, 0x02, 0x03, 0x04, 0xf7];
        let expected = MtcTimecode {
            hours: 1,
            minutes: 2,
            seconds: 3,
            frames: 4,
            frame_rate: MtcFrameRate::Fps25,
        };

        assert_eq!(mtc_full_frame_timecode(&message), Some(expected));
        assert_eq!(
            midi_clock_events_from_message(&message, &mut decoder),
            vec![MidiClockEvent::Timecode(expected)]
        );
        assert_eq!(expected.position_ms(), 3_723_160);
    }

    #[test]
    fn ignores_invalid_mtc_full_frame_sysex_timecode() {
        let mut decoder = MtcQuarterFrameDecoder::default();

        assert_eq!(
            mtc_full_frame_timecode(&[0xf0, 0x7f, 0x7f, 0x01, 0x01, 0x61, 0x02, 0x03, 0x30, 0xf7]),
            None
        );
        assert!(midi_clock_events_from_message(
            &[0xf0, 0x7f, 0x7f, 0x01, 0x01, 0x21, 0x60, 0x03, 0x04, 0xf7],
            &mut decoder
        )
        .is_empty());
    }

    #[test]
    fn maps_note_to_cue_and_ignores_note_off_for_triggers() {
        let mapping = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 60,
            action: MidiControlAction::TriggerCue,
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
        };
        let next = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 61,
            action: MidiControlAction::TriggerNextCue,
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
        let previous = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 62,
            action: MidiControlAction::TriggerPreviousCue,
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
            events_from_midi_message(&[0x91, 60, 127], &[mapping.clone()]),
            vec![MidiControlEvent::TriggerCue(9)]
        );
        assert!(events_from_midi_message(&[0x91, 60, 0], &[mapping]).is_empty());
        assert_eq!(
            events_from_midi_message(&[0x90, 61, 127], &[next]),
            vec![MidiControlEvent::TriggerNextCue]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 62, 127], &[previous]),
            vec![MidiControlEvent::TriggerPreviousCue]
        );
    }

    #[test]
    fn maps_video_param_and_hot_jump() {
        let video_param = MidiControlMapping {
            channel: Some(2),
            message: MidiControlMessage::ControlChange,
            number: 10,
            action: MidiControlAction::VideoParam,
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
        };
        let bpm_sync_param = MidiControlMapping {
            channel: Some(2),
            message: MidiControlMessage::ControlChange,
            number: 14,
            action: MidiControlAction::VideoParam,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: Some(4),
            video_param: Some(VideoParam::BpmSyncEnabled),
            cue_point_index: None,
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let hot_jump = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::ProgramChange,
            number: 2,
            action: MidiControlAction::VideoCuePointJump,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: Some(4),
            video_param: None,
            cue_point_index: Some(1),
            output_id: None,
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let cue_previous = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 5,
            action: MidiControlAction::VideoCuePointPrevious,
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
        };
        let cue_next = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 6,
            action: MidiControlAction::VideoCuePointNext,
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
        };
        let cue_add = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 3,
            action: MidiControlAction::VideoCuePointAdd,
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
        };
        let cue_remove = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 4,
            action: MidiControlAction::VideoCuePointRemove,
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
        };
        let layer_enabled = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 12,
            action: MidiControlAction::VideoLayerEnabled,
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
        };
        let layer_solo = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 13,
            action: MidiControlAction::VideoLayerSolo,
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
        };
        let loop_toggle = MidiControlMapping {
            channel: None,
            message: MidiControlMessage::NoteOn,
            number: 11,
            action: MidiControlAction::VideoLoop,
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
        };
        let layer_fade = MidiControlMapping {
            channel: Some(2),
            message: MidiControlMessage::ControlChange,
            number: 15,
            action: MidiControlAction::VideoLayerFade,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: Some(4),
            video_param: None,
            cue_point_index: None,
            output_id: None,
            duration_ms: Some(750),
            low: 0.0,
            high: 1.0,
        };

        assert_eq!(
            events_from_midi_message(&[0xb2, 10, 64], &[video_param]),
            vec![MidiControlEvent::SetVideoParam {
                layer_id: 4,
                param: VideoParam::Opacity,
                value: 64.0 / 127.0,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb2, 14, 127], std::slice::from_ref(&bpm_sync_param)),
            vec![MidiControlEvent::SetVideoParam {
                layer_id: 4,
                param: VideoParam::BpmSyncEnabled,
                value: 1.0,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xc0, 2], &[hot_jump]),
            vec![MidiControlEvent::JumpVideoCuePoint {
                layer_id: 4,
                cue_point_index: 1,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 5, 127], std::slice::from_ref(&cue_previous)),
            vec![MidiControlEvent::JumpVideoCuePointRelative {
                layer_id: 4,
                direction: -1,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 6, 127], std::slice::from_ref(&cue_next)),
            vec![MidiControlEvent::JumpVideoCuePointRelative {
                layer_id: 4,
                direction: 1,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 3, 127], std::slice::from_ref(&cue_add)),
            vec![MidiControlEvent::AddVideoCuePoint {
                layer_id: 4,
                position_ms: Some(2_500),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 4, 127], std::slice::from_ref(&cue_remove)),
            vec![MidiControlEvent::RemoveVideoCuePoint {
                layer_id: 4,
                position_ms: 2_500,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 12, 127], std::slice::from_ref(&layer_enabled)),
            vec![MidiControlEvent::SetVideoLayerEnabled {
                layer_id: 4,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 13, 127], std::slice::from_ref(&layer_solo)),
            vec![MidiControlEvent::SetVideoLayerSolo {
                layer_id: 4,
                solo: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 11, 127], std::slice::from_ref(&loop_toggle)),
            vec![MidiControlEvent::SetVideoLoop {
                layer_id: 4,
                enabled: true,
                loop_start_ms: Some(500),
                loop_end_ms: Some(2_000),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb2, 15, 64], std::slice::from_ref(&layer_fade)),
            vec![MidiControlEvent::FadeVideoLayerOpacity {
                layer_id: 4,
                opacity: 64.0 / 127.0,
                duration_ms: 750,
            }]
        );

        let snapshot = EngineSnapshot {
            video: protocol::VideoSnapshot {
                layers: vec![protocol::VideoLayerSummary {
                    id: 4,
                    label: "Layer 4".to_string(),
                    source: protocol::VideoSourceSummary {
                        kind: protocol::VideoSourceKind::File,
                        path: Some("clip.mp4".to_string()),
                        name: None,
                        codec: Some("H264".to_string()),
                        metadata: None,
                    },
                    blend_mode: protocol::VideoBlendMode::Normal,
                    state: VideoLayerState {
                        opacity: 0.5,
                        solo: true,
                        loop_enabled: true,
                        bpm_sync: protocol::VideoBpmSync {
                            enabled: true,
                            ratio: 1.0,
                            loop_bars: 1.0,
                        },
                        ..VideoLayerState::default()
                    },
                    isf_effect: None,
                }],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(
                &snapshot,
                &[
                    bpm_sync_param,
                    layer_enabled,
                    layer_solo,
                    loop_toggle,
                    layer_fade
                ]
            ),
            vec![
                vec![0xb2, 14, 127],
                vec![0x90, 12, 127],
                vec![0x90, 13, 127],
                vec![0x90, 11, 127],
                vec![0xb2, 15, 64],
            ]
        );
    }

    #[test]
    fn maps_video_output_controls_and_feedback() {
        let enabled = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::NoteOn,
            number: 40,
            action: MidiControlAction::VideoOutputEnabled,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let opacity = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 41,
            action: MidiControlAction::VideoOutputOpacity,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let fade = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 42,
            action: MidiControlAction::VideoOutputFade,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: Some(500),
            low: 0.0,
            high: 1.0,
        };
        let mapping_field = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 44,
            action: MidiControlAction::VideoOutputMappingField,
            fixture_id: None,
            attribute: Some("key_h".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: -1.0,
            high: 1.0,
        };
        let stage_mapping_field = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 45,
            action: MidiControlAction::VideoOutputMappingField,
            fixture_id: None,
            attribute: Some("stage_z".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: -1000.0,
            high: 1000.0,
        };
        let mapping_preset = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::NoteOn,
            number: 46,
            action: MidiControlAction::VideoOutputMappingPreset,
            fixture_id: None,
            attribute: Some("Front Projector".to_string()),
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };
        let blackout = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 43,
            action: MidiControlAction::VideoOutputBlackout,
            fixture_id: None,
            attribute: None,
            group_id: None,
            cue_id: None,
            layer_id: None,
            video_param: None,
            cue_point_index: None,
            output_id: Some(7),
            duration_ms: None,
            low: 0.0,
            high: 1.0,
        };

        assert_eq!(
            events_from_midi_message(&[0x90, 40, 127], std::slice::from_ref(&enabled)),
            vec![MidiControlEvent::SetVideoOutputEnabled {
                output_id: 7,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 41, 64], std::slice::from_ref(&opacity)),
            vec![MidiControlEvent::SetVideoOutputOpacity {
                output_id: 7,
                opacity: 64.0 / 127.0,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 42, 32], std::slice::from_ref(&fade)),
            vec![MidiControlEvent::FadeVideoOutputOpacity {
                output_id: 7,
                opacity: 32.0 / 127.0,
                duration_ms: 500,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 44, 32], std::slice::from_ref(&mapping_field)),
            vec![MidiControlEvent::SetVideoOutputMappingField {
                output_id: 7,
                field: "key_h".to_string(),
                value: -1.0 + (2.0 * 32.0 / 127.0),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 45, 95], std::slice::from_ref(&stage_mapping_field)),
            vec![MidiControlEvent::SetVideoOutputMappingField {
                output_id: 7,
                field: "stage_z".to_string(),
                value: -1000.0 + (2000.0 * 95.0 / 127.0),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 46, 127], std::slice::from_ref(&mapping_preset)),
            vec![MidiControlEvent::ApplyVideoOutputMappingPreset {
                output_id: 7,
                label: "Front Projector".to_string(),
            }]
        );
        assert!(
            events_from_midi_message(&[0x90, 46, 0], std::slice::from_ref(&mapping_preset))
                .is_empty()
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 43, 127], std::slice::from_ref(&blackout)),
            vec![MidiControlEvent::SetVideoOutputBlackout {
                output_id: 7,
                blackout: true,
            }]
        );

        let mut output_mapping = protocol::VideoOutputMapping::default();
        output_mapping.keystone_x = 0.5;
        output_mapping.stage_z = 500.0;
        let snapshot = EngineSnapshot {
            video: protocol::VideoSnapshot {
                outputs: vec![protocol::VideoOutputSummary {
                    id: 7,
                    label: "Projector".to_string(),
                    kind: protocol::VideoOutputKind::Display,
                    enabled: false,
                    composition_id: 1,
                    fullscreen: true,
                    monitor_id: None,
                    width: 1920,
                    height: 1080,
                    endpoint_name: None,
                    opacity: 0.25,
                    blackout: true,
                    mapping: output_mapping,
                }],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(
                &snapshot,
                &[
                    enabled,
                    opacity,
                    fade,
                    mapping_field,
                    stage_mapping_field,
                    mapping_preset,
                    blackout
                ]
            ),
            vec![
                vec![0x90, 40, 0],
                vec![0xb0, 41, 32],
                vec![0xb0, 42, 32],
                vec![0xb0, 44, 95],
                vec![0xb0, 45, 95],
                vec![0xb0, 43, 127],
            ]
        );
    }

    #[test]
    fn maps_effect_enabled_control_and_feedback() {
        let mapping = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 46,
            action: MidiControlAction::EffectEnabled,
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
        };

        assert_eq!(
            events_from_midi_message(&[0xb0, 46, 127], std::slice::from_ref(&mapping)),
            vec![MidiControlEvent::SetEffectEnabled {
                effect_id: 9,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 46, 0], std::slice::from_ref(&mapping)),
            vec![MidiControlEvent::SetEffectEnabled {
                effect_id: 9,
                enabled: false,
            }]
        );

        let snapshot = EngineSnapshot {
            effects: vec![protocol::EffectSummary {
                id: 9,
                label: "Dimmer chase".to_string(),
                effect_type: protocol::EffectKind::Lfo,
                fixture_ids: vec![1, 2],
                target_group_ids: Vec::new(),
                attribute: "Dimmer".to_string(),
                video_targets: Vec::new(),
                shape: protocol::LfoShape::Sine,
                period_ms: Some(500),
                clock_sync: None,
                low: 0,
                high: 65_535,
                phase: 0.0,
                fixture_spread: 0.0,
                blend_mode: protocol::EffectBlendMode::Override,
                origin: None,
                direction: None,
                speed: None,
                wavelength: None,
                enabled: true,
                lfo: None,
                color: None,
                chaser: None,
                move_effect: None,
                value: None,
                curve: None,
                mapping: None,
                color_mapping: None,
            }],
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(&snapshot, &[mapping]),
            vec![vec![0xb0, 46, 127]]
        );
    }

    #[test]
    fn maps_node_graph_enabled_control_and_feedback() {
        let mapping = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 47,
            action: MidiControlAction::NodeGraphEnabled,
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
        };

        assert_eq!(
            events_from_midi_message(&[0xb0, 47, 127], std::slice::from_ref(&mapping)),
            vec![MidiControlEvent::SetNodeGraphEnabled {
                graph_id: 12,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 47, 0], std::slice::from_ref(&mapping)),
            vec![MidiControlEvent::SetNodeGraphEnabled {
                graph_id: 12,
                enabled: false,
            }]
        );

        let snapshot = EngineSnapshot {
            node_graphs: vec![protocol::NodeGraphSummary {
                id: 12,
                label: "Wave graph".to_string(),
                enabled: true,
                nodes: Vec::new(),
                edges: Vec::new(),
                audio_runtime: Vec::new(),
            }],
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(&snapshot, &[mapping]),
            vec![vec![0xb0, 47, 127]]
        );
    }

    #[test]
    fn maps_timeline_transport_controls() {
        let play = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 20,
            action: MidiControlAction::TimelinePlay,
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
        let seek = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 21,
            action: MidiControlAction::TimelineSeek,
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
            high: 127_000.0,
        };
        let beat_previous = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::NoteOn,
            number: 22,
            action: MidiControlAction::TimelineBeatPrevious,
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
        let beat_next = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::NoteOn,
            number: 23,
            action: MidiControlAction::TimelineBeatNext,
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
        let set_bpm = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 24,
            action: MidiControlAction::SetBpm,
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
        };
        let tap_bpm = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::NoteOn,
            number: 25,
            action: MidiControlAction::TapBpm,
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
            events_from_midi_message(&[0xb0, 20, 127], std::slice::from_ref(&play)),
            vec![MidiControlEvent::SetTimelinePlaying(true)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 20, 0], &[play]),
            vec![MidiControlEvent::SetTimelinePlaying(false)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 21, 64], &[seek]),
            vec![MidiControlEvent::SeekTimeline {
                position_ms: 64_000,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 22, 127], std::slice::from_ref(&beat_previous)),
            vec![MidiControlEvent::SeekTimelineBeat { direction: -1 }]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 23, 127], std::slice::from_ref(&beat_next)),
            vec![MidiControlEvent::SeekTimelineBeat { direction: 1 }]
        );
        assert!(events_from_midi_message(&[0x90, 23, 0], &[beat_next]).is_empty());
        assert_eq!(
            events_from_midi_message(&[0xb0, 24, 64], std::slice::from_ref(&set_bpm)),
            vec![MidiControlEvent::SetBpm(20.0 + 280.0 * (64.0 / 127.0))]
        );
        assert_eq!(
            events_from_midi_message(&[0x90, 25, 127], std::slice::from_ref(&tap_bpm)),
            vec![MidiControlEvent::TapBpm]
        );
        assert!(events_from_midi_message(&[0x90, 25, 0], &[tap_bpm]).is_empty());
    }

    #[test]
    fn maps_master_and_cue_fade_pause_controls() {
        let master = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 11,
            action: MidiControlAction::LightingMaster,
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
        let pause = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 12,
            action: MidiControlAction::CueFadePause,
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

        let master_events = events_from_midi_message(&[0xb0, 11, 64], &[master]);
        assert_eq!(master_events.len(), 1);
        match master_events[0] {
            MidiControlEvent::LightingMaster(value) => {
                assert!((value - (64.0 / 127.0)).abs() < 0.0001);
            }
            _ => panic!("expected lighting master event"),
        }
        assert_eq!(
            events_from_midi_message(&[0xb0, 12, 127], std::slice::from_ref(&pause)),
            vec![MidiControlEvent::SetCueFadePaused(true)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 12, 0], &[pause]),
            vec![MidiControlEvent::SetCueFadePaused(false)]
        );
    }

    #[test]
    fn maps_group_submaster_control_and_feedback() {
        let mapping = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 13,
            action: MidiControlAction::GroupSubmaster,
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
        };

        let events = events_from_midi_message(&[0xb0, 13, 64], std::slice::from_ref(&mapping));
        assert_eq!(events.len(), 1);
        match &events[0] {
            MidiControlEvent::SetGroupSubmaster { group_id, level } => {
                assert_eq!(group_id, "front");
                assert!((*level - (64.0 / 127.0)).abs() < 0.0001);
            }
            _ => panic!("expected group submaster event"),
        }

        let snapshot = EngineSnapshot {
            submasters: vec![protocol::SubmasterSummary {
                group_id: "front".to_string(),
                label: "front".to_string(),
                level: 0.25,
                strobe_hz: 0.0,
                strobe_fixture_count: 0,
            }],
            ..EngineSnapshot::default()
        };
        assert_eq!(
            build_feedback_messages(&snapshot, &[mapping]),
            vec![vec![0xb0, 13, 32]]
        );
    }

    #[test]
    fn maps_fixture_and_group_flag_controls_and_feedback() {
        let fixture_highlight = fixture_flag_mapping(MidiControlAction::FixtureHighlight, 40, 3);
        let fixture_solo = fixture_flag_mapping(MidiControlAction::FixtureSolo, 41, 3);
        let fixture_park = fixture_flag_mapping(MidiControlAction::FixturePark, 42, 3);
        let group_highlight = group_flag_mapping(MidiControlAction::GroupHighlight, 43, "front");
        let group_solo = group_flag_mapping(MidiControlAction::GroupSolo, 44, "front");
        let group_park = group_flag_mapping(MidiControlAction::GroupPark, 45, "/front/");

        assert_eq!(
            events_from_midi_message(&[0xb0, 40, 127], std::slice::from_ref(&fixture_highlight)),
            vec![MidiControlEvent::SetFixtureHighlight {
                fixture_id: 3,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 41, 0], std::slice::from_ref(&fixture_solo)),
            vec![MidiControlEvent::SetFixtureSolo {
                fixture_id: 3,
                enabled: false,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 42, 64], std::slice::from_ref(&fixture_park)),
            vec![MidiControlEvent::SetFixturePark {
                fixture_id: 3,
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 43, 127], std::slice::from_ref(&group_highlight)),
            vec![MidiControlEvent::SetGroupHighlight {
                group_id: "front".to_string(),
                enabled: true,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 44, 0], std::slice::from_ref(&group_solo)),
            vec![MidiControlEvent::SetGroupSolo {
                group_id: "front".to_string(),
                enabled: false,
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 45, 64], std::slice::from_ref(&group_park)),
            vec![MidiControlEvent::SetGroupPark {
                group_id: "/front/".to_string(),
                enabled: true,
            }]
        );

        let snapshot = EngineSnapshot {
            fixtures: vec![
                flagged_fixture(3, &["front/left"], true, false, true),
                flagged_fixture(4, &["front/right"], false, true, false),
            ],
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(
                &snapshot,
                &[
                    fixture_highlight,
                    fixture_solo,
                    fixture_park,
                    group_highlight,
                    group_solo,
                    group_park,
                ],
            ),
            vec![
                vec![0xb0, 40, 127],
                vec![0xb0, 41, 0],
                vec![0xb0, 42, 127],
                vec![0xb0, 43, 127],
                vec![0xb0, 44, 127],
                vec![0xb0, 45, 127],
            ]
        );
    }

    #[test]
    fn maps_blackout_controls() {
        let blackout = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 30,
            action: MidiControlAction::Blackout,
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
        let video_blackout = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 31,
            action: MidiControlAction::VideoBlackout,
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
        let all_blackout = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 32,
            action: MidiControlAction::AllBlackout,
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
        let clear_fixture_flags = MidiControlMapping {
            channel: Some(0),
            message: MidiControlMessage::ControlChange,
            number: 33,
            action: MidiControlAction::ClearFixtureFlags,
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
        let mut clear_solo_flags = clear_fixture_flags.clone();
        clear_solo_flags.number = 34;
        clear_solo_flags.attribute = Some("solo".to_string());

        assert_eq!(
            events_from_midi_message(&[0xb0, 30, 127], std::slice::from_ref(&blackout)),
            vec![MidiControlEvent::Blackout(true)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 30, 0], &[blackout]),
            vec![MidiControlEvent::Blackout(false)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 31, 127], &[video_blackout]),
            vec![MidiControlEvent::VideoBlackout(true)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 32, 127], &[all_blackout]),
            vec![MidiControlEvent::AllBlackout(true)]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 33, 127], std::slice::from_ref(&clear_fixture_flags)),
            vec![MidiControlEvent::ClearFixtureFlags {
                kind: "all".to_string(),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 34, 127], std::slice::from_ref(&clear_solo_flags)),
            vec![MidiControlEvent::ClearFixtureFlags {
                kind: "solo".to_string(),
            }]
        );
        assert_eq!(
            events_from_midi_message(&[0xb0, 33, 0], &[clear_fixture_flags]),
            Vec::<MidiControlEvent>::new()
        );
    }

    #[test]
    fn builds_midi_feedback_messages_for_active_cue() {
        let snapshot = EngineSnapshot {
            active_cue_id: Some(9),
            ..EngineSnapshot::default()
        };
        let mapping = MidiControlMapping {
            channel: Some(2),
            message: MidiControlMessage::NoteOn,
            number: 60,
            action: MidiControlAction::TriggerCue,
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
        };

        assert_eq!(
            build_feedback_messages(&snapshot, &[mapping]),
            vec![vec![0x92, 60, 127]]
        );
    }

    #[test]
    fn builds_midi_feedback_for_master_and_cue_pause() {
        let snapshot = EngineSnapshot {
            lighting_master: 0.5,
            clock: protocol::ClockSnapshot {
                bpm: 160.0,
                ..protocol::ClockSnapshot::default()
            },
            active_fade: Some(protocol::ActiveFadeSummary {
                cue_id: 1,
                progress: 0.25,
                remaining_ms: 500,
                paused: true,
            }),
            ..EngineSnapshot::default()
        };
        let mappings = vec![
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::ControlChange,
                number: 11,
                action: MidiControlAction::LightingMaster,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::NoteOn,
                number: 61,
                action: MidiControlAction::CueFadePause,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::ControlChange,
                number: 62,
                action: MidiControlAction::SetBpm,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::NoteOn,
                number: 63,
                action: MidiControlAction::TapBpm,
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
            build_feedback_messages(&snapshot, &mappings),
            vec![vec![0xb0, 11, 64], vec![0x90, 61, 127], vec![0xb0, 62, 64]]
        );
    }

    #[test]
    fn builds_midi_feedback_for_blackouts() {
        let snapshot = EngineSnapshot {
            blackout: true,
            video: protocol::VideoSnapshot {
                blackout: true,
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };
        let mappings = vec![
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::NoteOn,
                number: 30,
                action: MidiControlAction::Blackout,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::ControlChange,
                number: 31,
                action: MidiControlAction::VideoBlackout,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::ControlChange,
                number: 32,
                action: MidiControlAction::AllBlackout,
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
            build_feedback_messages(&snapshot, &mappings),
            vec![
                vec![0x90, 30, 127],
                vec![0xb0, 31, 127],
                vec![0xb0, 32, 127]
            ]
        );
    }

    #[test]
    fn builds_midi_feedback_for_timeline_transport() {
        let snapshot = EngineSnapshot {
            timeline: protocol::TimelineSnapshot {
                playing: true,
                position_ms: 30_000,
                duration_ms: 120_000,
                ..protocol::TimelineSnapshot::default()
            },
            ..EngineSnapshot::default()
        };
        let mappings = vec![
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::NoteOn,
                number: 20,
                action: MidiControlAction::TimelinePlay,
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
            MidiControlMapping {
                channel: Some(0),
                message: MidiControlMessage::ControlChange,
                number: 21,
                action: MidiControlAction::TimelineSeek,
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
        ];

        assert_eq!(
            build_feedback_messages(&snapshot, &mappings),
            vec![vec![0x90, 20, 127], vec![0xb0, 21, 32]]
        );
    }

    #[test]
    #[ignore = "requires physical MIDI ports named by SYNDOCAL_TEST_MIDI_INPUT/OUTPUT"]
    fn physical_midi_ports_enumerate_open_and_send_feedback() {
        let input_match = std::env::var("SYNDOCAL_TEST_MIDI_INPUT")
            .expect("set SYNDOCAL_TEST_MIDI_INPUT to part of a physical input port name");
        let output_match = std::env::var("SYNDOCAL_TEST_MIDI_OUTPUT")
            .expect("set SYNDOCAL_TEST_MIDI_OUTPUT to part of a physical output port name");
        let inputs = list_midi_inputs().unwrap();
        let outputs = list_midi_outputs().unwrap();
        println!("MIDI inputs: {inputs:?}");
        println!("MIDI outputs: {outputs:?}");
        let input = inputs
            .iter()
            .find(|port| {
                port.name
                    .to_lowercase()
                    .contains(&input_match.to_lowercase())
            })
            .unwrap_or_else(|| panic!("no MIDI input matched '{input_match}'"));
        let output = outputs
            .iter()
            .find(|port| {
                port.name
                    .to_lowercase()
                    .contains(&output_match.to_lowercase())
            })
            .unwrap_or_else(|| panic!("no MIDI output matched '{output_match}'"));

        let _clock_input = connect_midi_clock(input.index, |_| {}).unwrap();
        let mut feedback = connect_midi_feedback_output(output.index).unwrap();
        assert_eq!(
            feedback
                .send_feedback_messages(&[vec![0xB0, 123, 0]])
                .unwrap(),
            1
        );
    }
}
