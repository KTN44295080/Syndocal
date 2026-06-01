use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use midir::{Ignore, MidiInput, MidiInputConnection, MidiOutput, MidiOutputConnection};
use protocol::{
    CueId, EngineSnapshot, FixtureId, LearnedMidiControl, MidiControlAction, MidiControlMapping,
    MidiControlMessage, MidiInputSummary, MidiOutputSummary, VideoLayerId, VideoLayerState,
    VideoOutputId, VideoParam,
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
    last_timecode: Option<MtcTimecode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MidiControlEvent {
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    TriggerCue(CueId),
    TriggerNextCue,
    TriggerPreviousCue,
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
    SetVideoOutputBlackout {
        output_id: VideoOutputId,
        blackout: bool,
    },
    SetTimelinePlaying(bool),
    SeekTimeline {
        position_ms: u64,
    },
    LightingMaster(f32),
    SetGroupSubmaster {
        group_id: String,
        level: f32,
    },
    SetCueFadePaused(bool),
    Blackout(bool),
    VideoBlackout(bool),
}

#[derive(Debug, Clone, PartialEq)]
struct MidiMessage {
    message: MidiControlMessage,
    channel: u8,
    number: u8,
    value: u8,
}

pub fn list_midi_inputs() -> Result<Vec<MidiInputSummary>, MidiError> {
    let input = MidiInput::new("kdmx-midi-list")?;
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
    let output = MidiOutput::new("kdmx-midi-output-list")?;
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
    let output = MidiOutput::new("kdmx-midi-feedback")?;
    let ports = output.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = output
        .connect(port, "kdmx-midi-feedback-output")
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
    let mut input = MidiInput::new("kdmx-midi-clock")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = input
        .connect(
            port,
            "kdmx-midi-clock-input",
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
    let mut input = MidiInput::new("kdmx-midi-control")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let connection = input
        .connect(
            port,
            "kdmx-midi-control-input",
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
    let mut input = MidiInput::new("kdmx-midi-learn")?;
    input.ignore(Ignore::None);
    let ports = input.ports();
    let port = ports
        .get(port_index)
        .ok_or(MidiError::MissingPort(port_index))?;
    let (sender, receiver) = mpsc::channel::<LearnedMidiControl>();
    let connection = input
        .connect(
            port,
            "kdmx-midi-learn-input",
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
        MidiControlAction::TriggerCue => {
            let cue_id = mapping.cue_id?;
            Some(if snapshot.active_cue_id == Some(cue_id) {
                1.0
            } else {
                0.0
            })
        }
        MidiControlAction::TriggerNextCue | MidiControlAction::TriggerPreviousCue => None,
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
        | MidiControlAction::VideoCuePointJump => None,
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
    if let Some(timecode) = mtc_decoder.push_message(message) {
        events.push(MidiClockEvent::Timecode(timecode));
    }
    events
}

impl MtcQuarterFrameDecoder {
    fn push_message(&mut self, message: &[u8]) -> Option<MtcTimecode> {
        if message.first().copied() != Some(0xf1) {
            return None;
        }
        let data = *message.get(1)?;
        let piece = ((data >> 4) & 0x07) as usize;
        let value = data & 0x0f;
        self.nibbles[piece] = Some(value);
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
        MidiControlAction::VideoBlackout => {
            Some(MidiControlEvent::VideoBlackout(match message.message {
                MidiControlMessage::NoteOff => false,
                _ => message.value > 0,
            }))
        }
    }
}

fn is_positive_trigger(message: &MidiMessage) -> bool {
    !matches!(message.message, MidiControlMessage::NoteOff) && message.value > 0
}

fn finite_mapping_ms(value: f32) -> Option<u64> {
    value.is_finite().then(|| value.max(0.0).round() as u64)
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
                        solo: true,
                        loop_enabled: true,
                        bpm_sync: protocol::VideoBpmSync {
                            enabled: true,
                            ratio: 1.0,
                            loop_bars: 1.0,
                        },
                        ..VideoLayerState::default()
                    },
                }],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(
                &snapshot,
                &[bpm_sync_param, layer_enabled, layer_solo, loop_toggle]
            ),
            vec![
                vec![0xb2, 14, 127],
                vec![0x90, 12, 127],
                vec![0x90, 13, 127],
                vec![0x90, 11, 127]
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
            events_from_midi_message(&[0xb0, 43, 127], std::slice::from_ref(&blackout)),
            vec![MidiControlEvent::SetVideoOutputBlackout {
                output_id: 7,
                blackout: true,
            }]
        );

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
                    mapping: Default::default(),
                }],
                ..protocol::VideoSnapshot::default()
            },
            ..EngineSnapshot::default()
        };

        assert_eq!(
            build_feedback_messages(&snapshot, &[enabled, opacity, fade, blackout]),
            vec![
                vec![0x90, 40, 0],
                vec![0xb0, 41, 32],
                vec![0xb0, 42, 32],
                vec![0xb0, 43, 127],
            ]
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
            }],
            ..EngineSnapshot::default()
        };
        assert_eq!(
            build_feedback_messages(&snapshot, &[mapping]),
            vec![vec![0xb0, 13, 32]]
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
        ];

        assert_eq!(
            build_feedback_messages(&snapshot, &mappings),
            vec![vec![0xb0, 11, 64], vec![0x90, 61, 127]]
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
        ];

        assert_eq!(
            build_feedback_messages(&snapshot, &mappings),
            vec![vec![0x90, 30, 127], vec![0xb0, 31, 127]]
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
}
