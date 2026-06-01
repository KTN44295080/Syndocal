use std::{
    collections::{BTreeSet, HashMap, HashSet},
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc, RwLock,
    },
    thread,
    time::{Duration, Instant},
};

use crossbeam_queue::ArrayQueue;
use io::{
    artnet::ArtNetSender,
    sacn::SacnSender,
    serial_dmx::{EnttecOpenDmxSender, EnttecUsbProSender},
};
use protocol::{
    ActiveFadeSummary, AttributeControl, AttributeResolution, AttributeValueSummary,
    AudioAnalysisSummary, AutomationId, AutomationInterpolation, AutomationKeyframeSummary,
    ClockSnapshot, ClockSource, CompositionId, CompositionSummary, CueFixtureTarget, CueId,
    CueSummary, DmxModeSummary, DmxOutputConfig, DmxOutputProtocol, DmxUniversePreview,
    EffectBlendMode, EffectId, EffectKind, EffectSummary, EngineSnapshot, EngineTelemetry,
    FixtureId, FixtureLimits, FixtureProfileSummary, GeometrySummary, LfoEffectRequest, LfoShape,
    PatchFixtureRequest, PatchedFixtureSummary, PositionWaveEffectRequest, Rotation3,
    SubmasterSummary, TimelineAutomationSummary, TimelineCueEventSummary, TimelineEventId,
    TimelineSnapshot, TimelineTrackKind, TimelineVideoAutomationSummary, Transform2D, Vec3,
    VideoAutomationKeyframeSummary, VideoBlendMode, VideoColorAdjust, VideoEffectTarget,
    VideoFxAdjust, VideoLayerId, VideoLayerState, VideoLayerSummary, VideoLayerTarget,
    VideoOutputId, VideoOutputKind, VideoOutputMapping, VideoOutputMappingPresetSummary,
    VideoOutputSummary, VideoOutputTarget, VideoParam, VideoSnapshot, VideoSourceSummary,
};
use thiserror::Error;

const ENGINE_QUEUE_CAPACITY: usize = 4096;
const COMMANDS_PER_TICK_LIMIT: usize = 512;
const DMX_TICK_INTERVAL: Duration = Duration::from_micros(22_727);

#[derive(Debug, Error)]
pub enum EngineError {
    #[error("engine command queue is full")]
    QueueFull,
}

#[derive(Debug)]
pub enum EngineCommand {
    PatchFixture {
        fixture_id: FixtureId,
        request: PatchFixtureRequest,
        profile: FixtureProfileSummary,
    },
    RemoveFixture(FixtureId),
    SetAttribute {
        fixture_id: FixtureId,
        attribute: String,
        value: u16,
    },
    SetGroupAttribute {
        group_id: String,
        attribute: String,
        value: u16,
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
    SetFixtureTransform {
        fixture_id: FixtureId,
        position: Vec3,
        rotation: Rotation3,
    },
    SetFixturePatch {
        fixture_id: FixtureId,
        label: String,
        universe: u16,
        address: u16,
    },
    SetFixtureLimits {
        fixture_id: FixtureId,
        limits: FixtureLimits,
    },
    SetFixtureGroups {
        fixture_id: FixtureId,
        group_ids: Vec<String>,
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
    ApplyAttributeValues {
        fixture_id: FixtureId,
        values: Vec<AttributeValueSummary>,
    },
    LoadProjectSnapshot(EngineSnapshot),
    SetOutput(DmxOutputConfig),
    SetDmxOutputs(Vec<DmxOutputConfig>),
    Blackout(bool),
    SetLightingMaster(f32),
    SetGroupSubmaster {
        group_id: String,
        level: f32,
    },
    SetBpm(f32),
    TapBpm,
    MidiClockPulse,
    AddLfoEffect {
        effect_id: EffectId,
        request: LfoEffectRequest,
    },
    AddPositionWaveEffect {
        effect_id: EffectId,
        request: PositionWaveEffectRequest,
    },
    SetEffectEnabled {
        effect_id: EffectId,
        enabled: bool,
    },
    MoveEffect {
        effect_id: EffectId,
        delta: i32,
    },
    RemoveEffect(EffectId),
    CreateCue {
        cue_id: CueId,
        label: String,
        fade_ms: u64,
        targets: Vec<CueFixtureTarget>,
        video_targets: Vec<VideoLayerTarget>,
        video_output_targets: Vec<VideoOutputTarget>,
    },
    UpdateCue {
        cue_id: CueId,
        label: String,
        fade_ms: u64,
        targets: Vec<CueFixtureTarget>,
        video_targets: Vec<VideoLayerTarget>,
        video_output_targets: Vec<VideoOutputTarget>,
    },
    SetCueMetadata {
        cue_id: CueId,
        label: String,
        fade_ms: u64,
    },
    MoveCue {
        cue_id: CueId,
        delta: i32,
    },
    DuplicateCue {
        source_cue_id: CueId,
        cue_id: CueId,
        label: String,
    },
    TriggerCue(CueId),
    TriggerNextCue,
    TriggerPreviousCue,
    SetCueFadePaused(bool),
    RemoveCue(CueId),
    AddTimelineCueEvent {
        event_id: TimelineEventId,
        cue_id: CueId,
        time_ms: u64,
        track: TimelineTrackKind,
    },
    SetTimelineCueEvent {
        event_id: TimelineEventId,
        cue_id: CueId,
        time_ms: u64,
        track: TimelineTrackKind,
    },
    RemoveTimelineEvent(TimelineEventId),
    AddTimelineAutomation {
        automation_id: AutomationId,
        fixture_id: FixtureId,
        attribute: String,
        keyframes: Vec<AutomationKeyframeSummary>,
    },
    SetTimelineAutomation {
        automation_id: AutomationId,
        fixture_id: FixtureId,
        attribute: String,
        keyframes: Vec<AutomationKeyframeSummary>,
    },
    AddTimelineVideoAutomation {
        automation_id: AutomationId,
        layer_id: VideoLayerId,
        param: VideoParam,
        keyframes: Vec<VideoAutomationKeyframeSummary>,
    },
    SetTimelineVideoAutomation {
        automation_id: AutomationId,
        layer_id: VideoLayerId,
        param: VideoParam,
        keyframes: Vec<VideoAutomationKeyframeSummary>,
    },
    RemoveTimelineAutomation(AutomationId),
    SetTimelineAudio(Option<AudioAnalysisSummary>),
    SetTimelinePlaying(bool),
    SeekTimeline(u64),
    SyncTimelineTimecode(u64),
    AddVideoLayer {
        layer_id: VideoLayerId,
        label: String,
        source: VideoSourceSummary,
    },
    DuplicateVideoLayer {
        source_layer_id: VideoLayerId,
        new_layer_id: VideoLayerId,
        label: String,
    },
    RemoveVideoLayer(VideoLayerId),
    SetVideoLayerOrder(Vec<VideoLayerId>),
    SetVideoLayerLabel {
        layer_id: VideoLayerId,
        label: String,
    },
    SetVideoLayerState {
        layer_id: VideoLayerId,
        state: VideoLayerState,
    },
    SetVideoLayerParam {
        layer_id: VideoLayerId,
        param: VideoParam,
        value: f32,
    },
    SetVideoLayerEnabled {
        layer_id: VideoLayerId,
        enabled: bool,
    },
    SetVideoLayerSolo {
        layer_id: VideoLayerId,
        solo: bool,
    },
    SetVideoLayerPlaying {
        layer_id: VideoLayerId,
        playing: bool,
    },
    SetVideoLayerLoop {
        layer_id: VideoLayerId,
        enabled: bool,
        loop_start_ms: Option<u64>,
        loop_end_ms: Option<u64>,
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
    SetVideoLayerBlendMode {
        layer_id: VideoLayerId,
        blend_mode: VideoBlendMode,
    },
    SetVideoMasterOpacity(f32),
    SetVideoBlackout(bool),
    AddVideoComposition(CompositionSummary),
    RemoveVideoComposition(CompositionId),
    SetVideoCompositionLayers {
        composition_id: CompositionId,
        layer_ids: Vec<VideoLayerId>,
    },
    AddVideoOutput(VideoOutputSummary),
    RemoveVideoOutput(VideoOutputId),
    SetVideoOutputConfig {
        output_id: VideoOutputId,
        label: String,
        kind: VideoOutputKind,
        fullscreen: bool,
        monitor_id: Option<u32>,
        width: u32,
        height: u32,
        endpoint_name: Option<String>,
    },
    SetVideoOutputEnabled {
        output_id: VideoOutputId,
        enabled: bool,
    },
    SetVideoOutputRouting {
        output_id: VideoOutputId,
        composition_id: CompositionId,
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
    SetVideoOutputMapping {
        output_id: VideoOutputId,
        mapping: VideoOutputMapping,
    },
    SaveVideoOutputMappingPreset {
        label: String,
        mapping: VideoOutputMapping,
    },
    RemoveVideoOutputMappingPreset {
        label: String,
    },
}

#[derive(Clone)]
pub struct EngineHandle {
    queue: Arc<ArrayQueue<EngineCommand>>,
    snapshot: Arc<RwLock<EngineSnapshot>>,
    next_fixture_id: Arc<AtomicU64>,
    next_effect_id: Arc<AtomicU64>,
    next_cue_id: Arc<AtomicU64>,
    next_timeline_event_id: Arc<AtomicU64>,
    next_automation_id: Arc<AtomicU64>,
    next_video_layer_id: Arc<AtomicU64>,
    next_composition_id: Arc<AtomicU64>,
    next_video_output_id: Arc<AtomicU64>,
}

impl EngineHandle {
    pub fn start(output: DmxOutputConfig) -> Self {
        let queue = Arc::new(ArrayQueue::new(ENGINE_QUEUE_CAPACITY));
        let snapshot = Arc::new(RwLock::new(EngineSnapshot {
            output: output.clone(),
            ..EngineSnapshot::default()
        }));
        let next_fixture_id = Arc::new(AtomicU64::new(1));
        let next_effect_id = Arc::new(AtomicU64::new(1));
        let next_cue_id = Arc::new(AtomicU64::new(1));
        let next_timeline_event_id = Arc::new(AtomicU64::new(1));
        let next_automation_id = Arc::new(AtomicU64::new(1));
        let next_video_layer_id = Arc::new(AtomicU64::new(1));
        let next_composition_id = Arc::new(AtomicU64::new(2));
        let next_video_output_id = Arc::new(AtomicU64::new(1));

        let runtime_queue = Arc::clone(&queue);
        let runtime_snapshot = Arc::clone(&snapshot);
        thread::Builder::new()
            .name("kdmx-engine".to_string())
            .spawn(move || {
                let mut runtime = EngineRuntime::new(output);
                runtime.run(runtime_queue, runtime_snapshot);
            })
            .expect("failed to start KDMX engine thread");

        Self {
            queue,
            snapshot,
            next_fixture_id,
            next_effect_id,
            next_cue_id,
            next_timeline_event_id,
            next_automation_id,
            next_video_layer_id,
            next_composition_id,
            next_video_output_id,
        }
    }

    pub fn allocate_fixture_id(&self) -> FixtureId {
        self.next_fixture_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_effect_id(&self) -> EffectId {
        self.next_effect_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_cue_id(&self) -> CueId {
        self.next_cue_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_timeline_event_id(&self) -> TimelineEventId {
        self.next_timeline_event_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_automation_id(&self) -> AutomationId {
        self.next_automation_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_video_layer_id(&self) -> VideoLayerId {
        self.next_video_layer_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_composition_id(&self) -> CompositionId {
        self.next_composition_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn allocate_video_output_id(&self) -> VideoOutputId {
        self.next_video_output_id.fetch_add(1, Ordering::Relaxed)
    }

    pub fn send(&self, command: EngineCommand) -> Result<(), EngineError> {
        self.queue.push(command).map_err(|_| EngineError::QueueFull)
    }

    pub fn load_project_snapshot(&self, snapshot: EngineSnapshot) -> Result<(), EngineError> {
        self.sync_allocator_counters(&snapshot);
        self.send(EngineCommand::LoadProjectSnapshot(snapshot))
    }

    pub fn snapshot(&self) -> EngineSnapshot {
        self.snapshot
            .read()
            .map(|snapshot| snapshot.clone())
            .unwrap_or_default()
    }

    fn sync_allocator_counters(&self, snapshot: &EngineSnapshot) {
        store_next_id(
            &self.next_fixture_id,
            snapshot
                .fixtures
                .iter()
                .map(|fixture| fixture.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        store_next_id(
            &self.next_effect_id,
            snapshot
                .effects
                .iter()
                .map(|effect| effect.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        store_next_id(
            &self.next_cue_id,
            snapshot
                .cues
                .iter()
                .map(|cue| cue.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        store_next_id(
            &self.next_timeline_event_id,
            snapshot
                .timeline
                .events
                .iter()
                .map(|event| event.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        let max_automation_id = snapshot
            .timeline
            .automations
            .iter()
            .map(|automation| automation.id)
            .chain(
                snapshot
                    .timeline
                    .video_automations
                    .iter()
                    .map(|automation| automation.id),
            )
            .max()
            .unwrap_or(0);
        store_next_id(
            &self.next_automation_id,
            max_automation_id.saturating_add(1),
        );
        store_next_id(
            &self.next_video_layer_id,
            snapshot
                .video
                .layers
                .iter()
                .map(|layer| layer.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        store_next_id(
            &self.next_composition_id,
            snapshot
                .video
                .compositions
                .iter()
                .map(|composition| composition.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
        store_next_id(
            &self.next_video_output_id,
            snapshot
                .video
                .outputs
                .iter()
                .map(|output| output.id)
                .max()
                .unwrap_or(0)
                .saturating_add(1),
        );
    }
}

struct RuntimeFixture {
    id: FixtureId,
    request: PatchFixtureRequest,
    profile: FixtureProfileSummary,
    mode_index: usize,
    limits: FixtureLimits,
}

struct RuntimeEffect {
    id: EffectId,
    kind: RuntimeEffectKind,
    enabled: bool,
    created_at: Instant,
}

enum RuntimeEffectKind {
    Lfo(LfoEffectRequest),
    PositionWave(PositionWaveEffectRequest),
}

#[derive(Clone)]
struct RuntimeCue {
    id: CueId,
    label: String,
    fade_ms: u64,
    targets: Vec<CueFixtureTarget>,
    video_targets: Vec<VideoLayerTarget>,
    video_output_targets: Vec<VideoOutputTarget>,
}

struct RuntimeFade {
    cue_id: CueId,
    started_at: Instant,
    duration: Duration,
    paused_at: Option<Instant>,
    paused_duration: Duration,
    start_values: HashMap<(FixtureId, String), u16>,
    target_values: HashMap<(FixtureId, String), u16>,
    video_start_states: HashMap<VideoLayerId, VideoLayerState>,
    video_target_states: HashMap<VideoLayerId, VideoLayerState>,
    video_output_start_opacities: HashMap<VideoOutputId, f32>,
    video_output_target_opacities: HashMap<VideoOutputId, f32>,
}

#[derive(Clone)]
struct RuntimeTimelineEvent {
    id: TimelineEventId,
    cue_id: CueId,
    time_ms: u64,
    track: TimelineTrackKind,
}

#[derive(Clone)]
struct RuntimeTimelineAutomation {
    id: AutomationId,
    fixture_id: FixtureId,
    attribute: String,
    track: TimelineTrackKind,
    keyframes: Vec<AutomationKeyframeSummary>,
    enabled: bool,
}

#[derive(Clone)]
struct RuntimeTimelineVideoAutomation {
    id: AutomationId,
    layer_id: VideoLayerId,
    param: VideoParam,
    track: TimelineTrackKind,
    keyframes: Vec<VideoAutomationKeyframeSummary>,
    enabled: bool,
}

#[derive(Clone)]
struct RuntimeVideoLayer {
    id: VideoLayerId,
    label: String,
    source: VideoSourceSummary,
    blend_mode: VideoBlendMode,
    state: VideoLayerState,
}

#[derive(Clone)]
struct RuntimeVideoComposition {
    summary: CompositionSummary,
}

#[derive(Clone)]
struct RuntimeVideoOutput {
    summary: VideoOutputSummary,
}

#[derive(Clone)]
struct RuntimeVideoOutputFade {
    output_id: VideoOutputId,
    started_at: Instant,
    duration: Duration,
    start_opacity: f32,
    target_opacity: f32,
}

struct RuntimeDmxOutput {
    config: DmxOutputConfig,
    sender: Option<DmxSender>,
}

struct EngineRuntime {
    fixtures: Vec<RuntimeFixture>,
    effects: Vec<RuntimeEffect>,
    cues: Vec<RuntimeCue>,
    timeline_events: Vec<RuntimeTimelineEvent>,
    timeline_automations: Vec<RuntimeTimelineAutomation>,
    timeline_video_automations: Vec<RuntimeTimelineVideoAutomation>,
    timeline_audio: Option<AudioAnalysisSummary>,
    timeline_playing: bool,
    timeline_position_ms: u64,
    video_layers: Vec<RuntimeVideoLayer>,
    video_compositions: Vec<RuntimeVideoComposition>,
    video_outputs: Vec<RuntimeVideoOutput>,
    video_output_fades: Vec<RuntimeVideoOutputFade>,
    video_output_mapping_presets: Vec<VideoOutputMappingPresetSummary>,
    video_master_opacity: f32,
    video_blackout: bool,
    lighting_master: f32,
    group_submaster_levels: HashMap<String, f32>,
    highlighted_fixtures: HashSet<FixtureId>,
    soloed_fixtures: HashSet<FixtureId>,
    parked_fixture_values: HashMap<FixtureId, HashMap<String, u16>>,
    values: HashMap<(FixtureId, String), u16>,
    active_cue_id: Option<CueId>,
    active_fade: Option<RuntimeFade>,
    output: DmxOutputConfig,
    additional_dmx_outputs: Vec<RuntimeDmxOutput>,
    blackout: bool,
    clock: BpmClock,
    frame_counter: u64,
    last_tick: Instant,
    last_tick_interval: Duration,
    last_tick_jitter_us: i64,
    tick_jitter_abs_max_us: u64,
    tick_jitter_samples: u64,
    tick_jitter_mean_us: f64,
    tick_jitter_m2_us: f64,
    last_packet_bytes: usize,
    last_dmx_output_count: usize,
    last_dmx_send_success_count: usize,
    last_dmx_send_failure_count: usize,
    total_dmx_send_success_count: u64,
    total_dmx_send_failure_count: u64,
    last_error: Option<String>,
    last_frame: [u8; 512],
    last_frames_by_universe: HashMap<u16, [u8; 512]>,
    dmx_sender: Option<DmxSender>,
}

enum DmxSender {
    ArtNet(ArtNetSender),
    Sacn(SacnSender),
    EnttecUsbPro(EnttecUsbProSender),
    EnttecOpenDmx(EnttecOpenDmxSender),
}

impl RuntimeDmxOutput {
    fn new(config: DmxOutputConfig) -> Self {
        let sender = create_dmx_sender(&config).ok();
        Self { config, sender }
    }
}

impl EngineRuntime {
    fn new(output: DmxOutputConfig) -> Self {
        let dmx_sender = create_dmx_sender(&output).ok();
        Self {
            fixtures: Vec::new(),
            effects: Vec::new(),
            cues: Vec::new(),
            timeline_events: Vec::new(),
            timeline_automations: Vec::new(),
            timeline_video_automations: Vec::new(),
            timeline_audio: None,
            timeline_playing: false,
            timeline_position_ms: 0,
            video_layers: Vec::new(),
            video_compositions: Vec::new(),
            video_outputs: Vec::new(),
            video_output_fades: Vec::new(),
            video_output_mapping_presets: Vec::new(),
            video_master_opacity: 1.0,
            video_blackout: false,
            lighting_master: 1.0,
            group_submaster_levels: HashMap::new(),
            highlighted_fixtures: HashSet::new(),
            soloed_fixtures: HashSet::new(),
            parked_fixture_values: HashMap::new(),
            values: HashMap::new(),
            active_cue_id: None,
            active_fade: None,
            output,
            additional_dmx_outputs: Vec::new(),
            blackout: false,
            clock: BpmClock::new(120.0, Instant::now()),
            frame_counter: 0,
            last_tick: Instant::now(),
            last_tick_interval: Duration::ZERO,
            last_tick_jitter_us: 0,
            tick_jitter_abs_max_us: 0,
            tick_jitter_samples: 0,
            tick_jitter_mean_us: 0.0,
            tick_jitter_m2_us: 0.0,
            last_packet_bytes: 0,
            last_dmx_output_count: 0,
            last_dmx_send_success_count: 0,
            last_dmx_send_failure_count: 0,
            total_dmx_send_success_count: 0,
            total_dmx_send_failure_count: 0,
            last_error: dmx_sender
                .as_ref()
                .map(|_| None)
                .unwrap_or_else(|| Some("DMX sender is not initialized".to_string())),
            last_frame: [0u8; 512],
            last_frames_by_universe: HashMap::new(),
            dmx_sender,
        }
    }

    fn load_project_snapshot(&mut self, snapshot: EngineSnapshot) {
        let now = Instant::now();
        self.fixtures = snapshot
            .fixtures
            .iter()
            .map(runtime_fixture_from_snapshot)
            .collect();
        self.values.clear();
        for fixture in &snapshot.fixtures {
            for value in &fixture.attribute_values {
                self.values
                    .insert((fixture.id, value.attribute.clone()), value.value);
            }
        }

        self.highlighted_fixtures = snapshot
            .fixtures
            .iter()
            .filter(|fixture| fixture.highlighted)
            .map(|fixture| fixture.id)
            .collect();
        self.soloed_fixtures = snapshot
            .fixtures
            .iter()
            .filter(|fixture| fixture.soloed)
            .map(|fixture| fixture.id)
            .collect();
        self.parked_fixture_values = snapshot
            .fixtures
            .iter()
            .filter(|fixture| fixture.parked)
            .map(|fixture| {
                let values = fixture
                    .attribute_values
                    .iter()
                    .map(|value| (value.attribute.clone(), value.value))
                    .collect();
                (fixture.id, values)
            })
            .collect();

        self.cues = snapshot.cues.iter().map(runtime_cue_from_summary).collect();
        self.timeline_events = snapshot
            .timeline
            .events
            .iter()
            .map(runtime_timeline_event_from_summary)
            .collect();
        self.timeline_events.sort_by_key(|event| event.time_ms);
        self.timeline_automations = snapshot
            .timeline
            .automations
            .iter()
            .map(runtime_timeline_automation_from_summary)
            .collect();
        self.timeline_video_automations = snapshot
            .timeline
            .video_automations
            .iter()
            .map(runtime_timeline_video_automation_from_summary)
            .collect();
        self.timeline_audio = snapshot.timeline.audio.clone();
        self.timeline_playing = snapshot.timeline.playing;

        self.video_layers = snapshot
            .video
            .layers
            .iter()
            .map(runtime_video_layer_from_summary)
            .collect();
        self.video_compositions = snapshot
            .video
            .compositions
            .iter()
            .filter(|composition| composition.id != 1)
            .map(|composition| RuntimeVideoComposition {
                summary: sanitize_video_composition(composition.clone(), &self.video_layers),
            })
            .collect();
        self.video_outputs = snapshot
            .video
            .outputs
            .iter()
            .map(|output| RuntimeVideoOutput {
                summary: sanitize_video_output(output.clone()),
            })
            .collect();
        self.video_output_fades.clear();
        self.video_output_mapping_presets = snapshot
            .video
            .mapping_presets
            .iter()
            .cloned()
            .filter_map(sanitize_video_output_mapping_preset)
            .collect();
        self.video_master_opacity = if snapshot.video.master_opacity.is_finite() {
            snapshot.video.master_opacity.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.video_blackout = snapshot.video.blackout;

        self.effects = snapshot
            .effects
            .iter()
            .filter_map(|effect| runtime_effect_from_summary(effect, now))
            .collect();
        let mut outputs = if snapshot.dmx_outputs.is_empty() {
            vec![snapshot.output.clone()]
        } else {
            snapshot.dmx_outputs.clone()
        };
        self.output = outputs.remove(0);
        self.dmx_sender = create_dmx_sender(&self.output).ok();
        self.additional_dmx_outputs = outputs.into_iter().map(RuntimeDmxOutput::new).collect();
        self.lighting_master = if snapshot.lighting_master.is_finite() {
            snapshot.lighting_master.clamp(0.0, 1.0)
        } else {
            1.0
        };
        self.group_submaster_levels = snapshot
            .submasters
            .iter()
            .filter(|submaster| submaster.level.is_finite())
            .map(|submaster| (submaster.group_id.clone(), submaster.level.clamp(0.0, 1.0)))
            .collect();
        self.blackout = snapshot.blackout;
        self.clock = BpmClock::new(snapshot.clock.bpm, now);
        self.timeline_position_ms = snapshot
            .timeline
            .position_ms
            .min(self.timeline_duration_ms());
        self.active_cue_id = snapshot
            .active_cue_id
            .filter(|cue_id| self.cues.iter().any(|cue| cue.id == *cue_id));
        self.active_fade = None;
        self.frame_counter = 0;
        self.last_tick = now;
        self.last_tick_interval = Duration::ZERO;
        self.last_tick_jitter_us = 0;
        self.tick_jitter_abs_max_us = 0;
        self.tick_jitter_samples = 0;
        self.tick_jitter_mean_us = 0.0;
        self.tick_jitter_m2_us = 0.0;
        self.last_packet_bytes = 0;
        self.last_dmx_output_count = 0;
        self.last_dmx_send_success_count = 0;
        self.last_dmx_send_failure_count = 0;
        self.total_dmx_send_success_count = 0;
        self.total_dmx_send_failure_count = 0;
        self.last_frame = dmx_preview_to_frame(&snapshot.dmx_preview);
        self.last_frames_by_universe = dmx_previews_to_frames(&snapshot.dmx_previews);
        self.last_frames_by_universe
            .insert(self.output.universe, self.last_frame);
        self.last_error = None;
    }

    fn run(
        &mut self,
        queue: Arc<ArrayQueue<EngineCommand>>,
        snapshot: Arc<RwLock<EngineSnapshot>>,
    ) {
        let mut next_tick = Instant::now();
        loop {
            self.consume_commands(&queue);

            let now = Instant::now();
            if now >= next_tick {
                self.tick(queue.len(), &snapshot);
                next_tick += DMX_TICK_INTERVAL;
                if next_tick < now {
                    next_tick = now + DMX_TICK_INTERVAL;
                }
            }

            let sleep_for = next_tick.saturating_duration_since(Instant::now());
            thread::sleep(sleep_for.min(Duration::from_millis(1)));
        }
    }

    fn consume_commands(&mut self, queue: &ArrayQueue<EngineCommand>) {
        for _ in 0..COMMANDS_PER_TICK_LIMIT {
            let Some(command) = queue.pop() else {
                break;
            };
            self.apply_command(command);
        }
    }

    fn apply_command(&mut self, command: EngineCommand) {
        match command {
            EngineCommand::PatchFixture {
                fixture_id,
                request,
                profile,
            } => {
                let mode_index = select_mode_index(&profile, request.mode_name.as_deref());
                seed_default_values(fixture_id, &profile, mode_index, &mut self.values);
                self.highlighted_fixtures.remove(&fixture_id);
                self.soloed_fixtures.remove(&fixture_id);
                self.parked_fixture_values.remove(&fixture_id);
                self.fixtures.push(RuntimeFixture {
                    id: fixture_id,
                    request,
                    profile,
                    mode_index,
                    limits: FixtureLimits::default(),
                });
                self.last_error = None;
            }
            EngineCommand::RemoveFixture(fixture_id) => {
                self.remove_fixture(fixture_id);
            }
            EngineCommand::SetAttribute {
                fixture_id,
                attribute,
                value,
            } => {
                self.active_cue_id = None;
                self.active_fade = None;
                self.values.insert((fixture_id, attribute), value);
            }
            EngineCommand::SetGroupAttribute {
                group_id,
                attribute,
                value,
            } => {
                self.active_cue_id = None;
                self.active_fade = None;
                for fixture in &self.fixtures {
                    if fixture.request.group_ids.iter().any(|id| id == &group_id) {
                        self.values.insert((fixture.id, attribute.clone()), value);
                    }
                }
                self.last_error = None;
            }
            EngineCommand::SetGroupHighlight { group_id, enabled } => {
                let fixture_ids = self.fixture_ids_in_group(&group_id);
                for fixture_id in fixture_ids {
                    set_fixture_flag(&mut self.highlighted_fixtures, fixture_id, enabled);
                }
                self.last_error = None;
            }
            EngineCommand::SetGroupSolo { group_id, enabled } => {
                let fixture_ids = self.fixture_ids_in_group(&group_id);
                for fixture_id in fixture_ids {
                    set_fixture_flag(&mut self.soloed_fixtures, fixture_id, enabled);
                }
                self.last_error = None;
            }
            EngineCommand::SetGroupPark { group_id, enabled } => {
                let fixture_ids = self.fixture_ids_in_group(&group_id);
                if enabled {
                    let now = Instant::now();
                    for fixture_id in fixture_ids {
                        if let Some(values) = self.capture_park_values(fixture_id, now) {
                            self.parked_fixture_values.insert(fixture_id, values);
                        }
                    }
                } else {
                    for fixture_id in fixture_ids {
                        self.parked_fixture_values.remove(&fixture_id);
                    }
                }
                self.last_error = None;
            }
            EngineCommand::SetFixtureTransform {
                fixture_id,
                position,
                rotation,
            } => {
                if let Some(fixture) = self
                    .fixtures
                    .iter_mut()
                    .find(|fixture| fixture.id == fixture_id)
                {
                    fixture.request.position = position;
                    fixture.request.rotation = rotation;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixturePatch {
                fixture_id,
                label,
                universe,
                address,
            } => {
                if let Some(fixture) = self
                    .fixtures
                    .iter_mut()
                    .find(|fixture| fixture.id == fixture_id)
                {
                    fixture.request.label = label;
                    fixture.request.universe = universe;
                    fixture.request.address = address;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixtureLimits { fixture_id, limits } => {
                if let Some(fixture) = self
                    .fixtures
                    .iter_mut()
                    .find(|fixture| fixture.id == fixture_id)
                {
                    fixture.limits = normalized_fixture_limits(limits);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixtureGroups {
                fixture_id,
                group_ids,
            } => {
                if let Some(fixture) = self
                    .fixtures
                    .iter_mut()
                    .find(|fixture| fixture.id == fixture_id)
                {
                    fixture.request.group_ids = group_ids;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixtureHighlight {
                fixture_id,
                enabled,
            } => {
                if self.fixtures.iter().any(|fixture| fixture.id == fixture_id) {
                    set_fixture_flag(&mut self.highlighted_fixtures, fixture_id, enabled);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixtureSolo {
                fixture_id,
                enabled,
            } => {
                if self.fixtures.iter().any(|fixture| fixture.id == fixture_id) {
                    set_fixture_flag(&mut self.soloed_fixtures, fixture_id, enabled);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::SetFixturePark {
                fixture_id,
                enabled,
            } => {
                if enabled {
                    if let Some(values) = self.capture_park_values(fixture_id, Instant::now()) {
                        self.parked_fixture_values.insert(fixture_id, values);
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                    }
                } else if self.fixtures.iter().any(|fixture| fixture.id == fixture_id) {
                    self.parked_fixture_values.remove(&fixture_id);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Fixture {fixture_id} was not found"));
                }
            }
            EngineCommand::ApplyAttributeValues { fixture_id, values } => {
                self.active_cue_id = None;
                self.active_fade = None;
                for value in values {
                    self.values
                        .insert((fixture_id, value.attribute), value.value);
                }
            }
            EngineCommand::LoadProjectSnapshot(snapshot) => {
                self.load_project_snapshot(snapshot);
            }
            EngineCommand::SetOutput(output) => {
                self.output = output;
                self.dmx_sender = create_dmx_sender(&self.output)
                    .map_err(|error| {
                        self.last_error = Some(error);
                    })
                    .ok();
            }
            EngineCommand::SetDmxOutputs(mut outputs) => {
                if outputs.is_empty() {
                    outputs.push(DmxOutputConfig {
                        enabled: false,
                        ..DmxOutputConfig::default()
                    });
                }
                self.output = outputs.remove(0);
                self.dmx_sender = create_dmx_sender(&self.output).ok();
                self.additional_dmx_outputs =
                    outputs.into_iter().map(RuntimeDmxOutput::new).collect();
                self.last_error = None;
            }
            EngineCommand::Blackout(enabled) => {
                self.blackout = enabled;
            }
            EngineCommand::SetLightingMaster(master) => {
                if master.is_finite() {
                    self.lighting_master = master.clamp(0.0, 1.0);
                    self.last_error = None;
                } else {
                    self.last_error = Some("Invalid lighting master value".to_string());
                }
            }
            EngineCommand::SetGroupSubmaster { group_id, level } => {
                let group_id = group_id.trim().to_string();
                if group_id.is_empty() {
                    self.last_error = Some("Submaster group id is required".to_string());
                } else if level.is_finite() {
                    self.group_submaster_levels
                        .insert(group_id, level.clamp(0.0, 1.0));
                    self.last_error = None;
                } else {
                    self.last_error = Some("Invalid submaster level".to_string());
                }
            }
            EngineCommand::SetBpm(bpm) => {
                self.clock.set_bpm(bpm, Instant::now());
            }
            EngineCommand::TapBpm => {
                self.clock.tap(Instant::now());
            }
            EngineCommand::MidiClockPulse => {
                self.clock.midi_clock_pulse(Instant::now());
            }
            EngineCommand::AddLfoEffect { effect_id, request } => {
                self.effects.push(RuntimeEffect {
                    id: effect_id,
                    kind: RuntimeEffectKind::Lfo(request),
                    enabled: true,
                    created_at: Instant::now(),
                });
                self.last_error = None;
            }
            EngineCommand::AddPositionWaveEffect { effect_id, request } => {
                self.effects.push(RuntimeEffect {
                    id: effect_id,
                    kind: RuntimeEffectKind::PositionWave(request),
                    enabled: true,
                    created_at: Instant::now(),
                });
                self.last_error = None;
            }
            EngineCommand::SetEffectEnabled { effect_id, enabled } => {
                if let Some(effect) = self
                    .effects
                    .iter_mut()
                    .find(|effect| effect.id == effect_id)
                {
                    effect.enabled = enabled;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Effect {effect_id} was not found"));
                }
            }
            EngineCommand::MoveEffect { effect_id, delta } => {
                if let Some(index) = self
                    .effects
                    .iter()
                    .position(|effect| effect.id == effect_id)
                {
                    let last_index = self.effects.len().saturating_sub(1);
                    let next_index = if delta < 0 {
                        index.saturating_sub(1)
                    } else if delta > 0 {
                        (index + 1).min(last_index)
                    } else {
                        index
                    };
                    if index != next_index {
                        self.effects.swap(index, next_index);
                    }
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Effect {effect_id} was not found"));
                }
            }
            EngineCommand::RemoveEffect(effect_id) => {
                self.effects.retain(|effect| effect.id != effect_id);
            }
            EngineCommand::CreateCue {
                cue_id,
                label,
                fade_ms,
                targets,
                video_targets,
                video_output_targets,
            } => {
                self.cues.retain(|cue| cue.id != cue_id);
                self.cues.push(RuntimeCue {
                    id: cue_id,
                    label,
                    fade_ms,
                    targets,
                    video_targets,
                    video_output_targets,
                });
                self.last_error = None;
            }
            EngineCommand::UpdateCue {
                cue_id,
                label,
                fade_ms,
                targets,
                video_targets,
                video_output_targets,
            } => {
                if let Some(cue) = self.cues.iter_mut().find(|cue| cue.id == cue_id) {
                    cue.label = label;
                    cue.fade_ms = fade_ms;
                    cue.targets = targets;
                    cue.video_targets = video_targets;
                    cue.video_output_targets = video_output_targets;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Cue {cue_id} was not found"));
                }
            }
            EngineCommand::SetCueMetadata {
                cue_id,
                label,
                fade_ms,
            } => {
                if label.trim().is_empty() {
                    self.last_error = Some("Cue label is required".to_string());
                } else if let Some(cue) = self.cues.iter_mut().find(|cue| cue.id == cue_id) {
                    cue.label = label.trim().to_string();
                    cue.fade_ms = fade_ms;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Cue {cue_id} was not found"));
                }
            }
            EngineCommand::MoveCue { cue_id, delta } => {
                if let Some(index) = self.cues.iter().position(|cue| cue.id == cue_id) {
                    let last_index = self.cues.len().saturating_sub(1);
                    let next_index = if delta < 0 {
                        index.saturating_sub(1)
                    } else if delta > 0 {
                        (index + 1).min(last_index)
                    } else {
                        index
                    };
                    if index != next_index {
                        self.cues.swap(index, next_index);
                    }
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Cue {cue_id} was not found"));
                }
            }
            EngineCommand::DuplicateCue {
                source_cue_id,
                cue_id,
                label,
            } => {
                if label.trim().is_empty() {
                    self.last_error = Some("Cue label is required".to_string());
                } else if let Some(source_index) =
                    self.cues.iter().position(|cue| cue.id == source_cue_id)
                {
                    let mut cue = self.cues[source_index].clone();
                    cue.id = cue_id;
                    cue.label = label.trim().to_string();
                    self.cues.retain(|existing| existing.id != cue_id);
                    let insert_index = self
                        .cues
                        .iter()
                        .position(|existing| existing.id == source_cue_id)
                        .map(|index| index + 1)
                        .unwrap_or(self.cues.len());
                    self.cues.insert(insert_index, cue);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Cue {source_cue_id} was not found"));
                }
            }
            EngineCommand::TriggerCue(cue_id) => {
                self.start_cue(cue_id, Instant::now());
            }
            EngineCommand::TriggerNextCue => {
                if let Some(cue_id) = self.relative_cue_id(1) {
                    self.start_cue(cue_id, Instant::now());
                }
            }
            EngineCommand::TriggerPreviousCue => {
                if let Some(cue_id) = self.relative_cue_id(-1) {
                    self.start_cue(cue_id, Instant::now());
                }
            }
            EngineCommand::SetCueFadePaused(paused) => {
                self.set_active_fade_paused(paused, Instant::now());
            }
            EngineCommand::RemoveCue(cue_id) => {
                self.cues.retain(|cue| cue.id != cue_id);
                self.timeline_events.retain(|event| event.cue_id != cue_id);
                if self.active_cue_id == Some(cue_id) {
                    self.active_cue_id = None;
                    self.active_fade = None;
                }
            }
            EngineCommand::AddTimelineCueEvent {
                event_id,
                cue_id,
                time_ms,
                track,
            } => {
                if self.cues.iter().any(|cue| cue.id == cue_id) {
                    self.timeline_events.retain(|event| event.id != event_id);
                    self.timeline_events.push(RuntimeTimelineEvent {
                        id: event_id,
                        cue_id,
                        time_ms,
                        track,
                    });
                    self.timeline_events.sort_by_key(|event| event.time_ms);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Cue {cue_id} was not found"));
                }
            }
            EngineCommand::SetTimelineCueEvent {
                event_id,
                cue_id,
                time_ms,
                track,
            } => {
                if !self.cues.iter().any(|cue| cue.id == cue_id) {
                    self.last_error = Some(format!("Cue {cue_id} was not found"));
                } else if let Some(event) = self
                    .timeline_events
                    .iter_mut()
                    .find(|event| event.id == event_id)
                {
                    event.cue_id = cue_id;
                    event.time_ms = time_ms;
                    event.track = track;
                    self.timeline_events.sort_by_key(|event| event.time_ms);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Timeline event {event_id} was not found"));
                }
            }
            EngineCommand::RemoveTimelineEvent(event_id) => {
                self.timeline_events.retain(|event| event.id != event_id);
            }
            EngineCommand::AddTimelineAutomation {
                automation_id,
                fixture_id,
                attribute,
                keyframes,
            } => {
                if self.fixtures.iter().any(|fixture| fixture.id == fixture_id)
                    && !attribute.trim().is_empty()
                    && !keyframes.is_empty()
                {
                    let mut keyframes = keyframes;
                    keyframes.sort_by_key(|keyframe| keyframe.time_ms);
                    self.timeline_automations
                        .retain(|automation| automation.id != automation_id);
                    self.timeline_automations.push(RuntimeTimelineAutomation {
                        id: automation_id,
                        fixture_id,
                        attribute,
                        track: TimelineTrackKind::Lighting,
                        keyframes,
                        enabled: true,
                    });
                    self.last_error = None;
                } else {
                    self.last_error = Some("Invalid timeline automation".to_string());
                }
            }
            EngineCommand::SetTimelineAutomation {
                automation_id,
                fixture_id,
                attribute,
                keyframes,
            } => {
                if !self
                    .timeline_automations
                    .iter()
                    .any(|automation| automation.id == automation_id)
                {
                    self.last_error = Some(format!("Automation {automation_id} was not found"));
                } else if self.fixtures.iter().any(|fixture| fixture.id == fixture_id)
                    && !attribute.trim().is_empty()
                    && !keyframes.is_empty()
                {
                    let mut keyframes = keyframes;
                    keyframes.sort_by_key(|keyframe| keyframe.time_ms);
                    if let Some(automation) = self
                        .timeline_automations
                        .iter_mut()
                        .find(|automation| automation.id == automation_id)
                    {
                        automation.fixture_id = fixture_id;
                        automation.attribute = attribute;
                        automation.track = TimelineTrackKind::Lighting;
                        automation.keyframes = keyframes;
                        automation.enabled = true;
                        self.last_error = None;
                    }
                } else {
                    self.last_error = Some("Invalid timeline automation".to_string());
                }
            }
            EngineCommand::AddTimelineVideoAutomation {
                automation_id,
                layer_id,
                param,
                keyframes,
            } => {
                if self.video_layers.iter().any(|layer| layer.id == layer_id)
                    && !keyframes.is_empty()
                    && keyframes.iter().all(|keyframe| keyframe.value.is_finite())
                {
                    let mut keyframes = keyframes;
                    keyframes.sort_by_key(|keyframe| keyframe.time_ms);
                    self.timeline_video_automations
                        .retain(|automation| automation.id != automation_id);
                    self.timeline_video_automations
                        .push(RuntimeTimelineVideoAutomation {
                            id: automation_id,
                            layer_id,
                            param,
                            track: TimelineTrackKind::Video,
                            keyframes,
                            enabled: true,
                        });
                    self.last_error = None;
                } else {
                    self.last_error = Some("Invalid video timeline automation".to_string());
                }
            }
            EngineCommand::SetTimelineVideoAutomation {
                automation_id,
                layer_id,
                param,
                keyframes,
            } => {
                if !self
                    .timeline_video_automations
                    .iter()
                    .any(|automation| automation.id == automation_id)
                {
                    self.last_error =
                        Some(format!("Video automation {automation_id} was not found"));
                } else if self.video_layers.iter().any(|layer| layer.id == layer_id)
                    && !keyframes.is_empty()
                    && keyframes.iter().all(|keyframe| keyframe.value.is_finite())
                {
                    let mut keyframes = keyframes;
                    keyframes.sort_by_key(|keyframe| keyframe.time_ms);
                    if let Some(automation) = self
                        .timeline_video_automations
                        .iter_mut()
                        .find(|automation| automation.id == automation_id)
                    {
                        automation.layer_id = layer_id;
                        automation.param = param;
                        automation.track = TimelineTrackKind::Video;
                        automation.keyframes = keyframes;
                        automation.enabled = true;
                        self.last_error = None;
                    }
                } else {
                    self.last_error = Some("Invalid video timeline automation".to_string());
                }
            }
            EngineCommand::RemoveTimelineAutomation(automation_id) => {
                self.timeline_automations
                    .retain(|automation| automation.id != automation_id);
                self.timeline_video_automations
                    .retain(|automation| automation.id != automation_id);
            }
            EngineCommand::SetTimelineAudio(audio) => {
                self.timeline_audio = audio;
                self.timeline_position_ms =
                    self.timeline_position_ms.min(self.timeline_duration_ms());
                self.last_error = None;
            }
            EngineCommand::SetTimelinePlaying(playing) => {
                self.timeline_playing = playing;
            }
            EngineCommand::SeekTimeline(position_ms) => {
                self.timeline_position_ms = position_ms.min(self.timeline_duration_ms());
                self.apply_timeline_automations();
                self.apply_timeline_video_automations();
            }
            EngineCommand::SyncTimelineTimecode(position_ms) => {
                self.sync_timeline_position(position_ms, Instant::now());
                self.apply_timeline_automations();
                self.apply_timeline_video_automations();
            }
            EngineCommand::AddVideoLayer {
                layer_id,
                label,
                source,
            } => {
                self.video_layers.retain(|layer| layer.id != layer_id);
                self.video_layers.push(RuntimeVideoLayer {
                    id: layer_id,
                    label: sanitize_video_layer_label(label, layer_id),
                    source,
                    blend_mode: VideoBlendMode::Normal,
                    state: VideoLayerState::default(),
                });
                self.last_error = None;
            }
            EngineCommand::DuplicateVideoLayer {
                source_layer_id,
                new_layer_id,
                label,
            } => {
                self.duplicate_video_layer(source_layer_id, new_layer_id, label);
            }
            EngineCommand::RemoveVideoLayer(layer_id) => {
                self.video_layers.retain(|layer| layer.id != layer_id);
                for cue in &mut self.cues {
                    cue.video_targets
                        .retain(|target| target.layer_id != layer_id);
                }
                for composition in &mut self.video_compositions {
                    composition
                        .summary
                        .layer_ids
                        .retain(|candidate| *candidate != layer_id);
                }
                self.timeline_video_automations
                    .retain(|automation| automation.layer_id != layer_id);
            }
            EngineCommand::SetVideoLayerOrder(layer_ids) => {
                reorder_video_layers(&mut self.video_layers, layer_ids);
                self.last_error = None;
            }
            EngineCommand::SetVideoLayerLabel { layer_id, label } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.label = sanitize_video_layer_label(label, layer_id);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerState { layer_id, state } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state = video::sanitize_layer_state(state);
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerParam {
                layer_id,
                param,
                value,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    apply_video_param(&mut layer.state, &param, value);
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerEnabled { layer_id, enabled } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.enabled = enabled;
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerSolo { layer_id, solo } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.solo = solo;
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerPlaying { layer_id, playing } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.playing = playing;
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerLoop {
                layer_id,
                enabled,
                loop_start_ms,
                loop_end_ms,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state.loop_enabled = enabled;
                    if let Some(loop_start_ms) = loop_start_ms {
                        layer.state.loop_start_ms = loop_start_ms;
                    }
                    if let Some(loop_end_ms) = loop_end_ms {
                        layer.state.loop_end_ms = loop_end_ms;
                    }
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    let position_ms = position_ms.unwrap_or(layer.state.position_ms);
                    layer.state.cue_points_ms.push(position_ms);
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::RemoveVideoCuePoint {
                layer_id,
                position_ms,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer
                        .state
                        .cue_points_ms
                        .retain(|candidate| *candidate != position_ms);
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::JumpVideoCuePoint {
                layer_id,
                cue_point_index,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.state = video::sanitize_layer_state(layer.state.clone());
                    if let Some(position_ms) = layer.state.cue_points_ms.get(cue_point_index) {
                        layer.state.position_ms = *position_ms;
                        self.last_error = None;
                    } else {
                        self.last_error = Some(format!(
                            "Video cue point {cue_point_index} was not found on layer {layer_id}"
                        ));
                    }
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoLayerBlendMode {
                layer_id,
                blend_mode,
            } => {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == layer_id)
                {
                    layer.blend_mode = blend_mode;
                    self.last_error = None;
                } else {
                    self.last_error = Some(format!("Video layer {layer_id} was not found"));
                }
            }
            EngineCommand::SetVideoMasterOpacity(opacity) => {
                if opacity.is_finite() {
                    self.video_master_opacity = opacity.clamp(0.0, 1.0);
                    self.last_error = None;
                } else {
                    self.last_error = Some("Invalid video master opacity".to_string());
                }
            }
            EngineCommand::SetVideoBlackout(enabled) => {
                self.video_blackout = enabled;
                self.last_error = None;
            }
            EngineCommand::AddVideoComposition(composition) => {
                let composition = sanitize_video_composition(composition, &self.video_layers);
                if composition.id == 1 {
                    self.last_error = Some("Main video composition cannot be replaced".to_string());
                } else {
                    self.video_compositions
                        .retain(|candidate| candidate.summary.id != composition.id);
                    self.video_compositions.push(RuntimeVideoComposition {
                        summary: composition,
                    });
                    self.last_error = None;
                }
            }
            EngineCommand::RemoveVideoComposition(composition_id) => {
                if composition_id == 1 {
                    self.last_error = Some("Main video composition cannot be removed".to_string());
                } else {
                    self.video_compositions
                        .retain(|composition| composition.summary.id != composition_id);
                    for output in &mut self.video_outputs {
                        if output.summary.composition_id == composition_id {
                            output.summary.composition_id = 1;
                        }
                    }
                    self.last_error = None;
                }
            }
            EngineCommand::SetVideoCompositionLayers {
                composition_id,
                layer_ids,
            } => {
                if composition_id == 1 {
                    self.last_error =
                        Some("Main video composition always contains all layers".to_string());
                } else if let Some(composition) = self
                    .video_compositions
                    .iter_mut()
                    .find(|composition| composition.summary.id == composition_id)
                {
                    composition.summary.layer_ids =
                        valid_unique_layer_ids(layer_ids, &self.video_layers);
                    self.last_error = None;
                } else {
                    self.last_error =
                        Some(format!("Video composition {composition_id} was not found"));
                }
            }
            EngineCommand::AddVideoOutput(output) => {
                self.video_outputs
                    .retain(|candidate| candidate.summary.id != output.id);
                let mut output = sanitize_video_output(output);
                if !self.video_composition_exists(output.composition_id) {
                    output.composition_id = 1;
                }
                self.video_outputs
                    .push(RuntimeVideoOutput { summary: output });
                self.last_error = None;
            }
            EngineCommand::RemoveVideoOutput(output_id) => {
                self.video_outputs
                    .retain(|output| output.summary.id != output_id);
                self.video_output_fades
                    .retain(|fade| fade.output_id != output_id);
                self.last_error = None;
            }
            EngineCommand::SetVideoOutputConfig {
                output_id,
                label,
                kind,
                fullscreen,
                monitor_id,
                width,
                height,
                endpoint_name,
            } => {
                let is_display = matches!(kind, VideoOutputKind::Display);
                self.update_video_output(output_id, |output| {
                    output.label = label;
                    output.kind = kind;
                    output.fullscreen = is_display && fullscreen;
                    output.monitor_id = if is_display { monitor_id } else { None };
                    output.width = width;
                    output.height = height;
                    output.endpoint_name = if is_display { None } else { endpoint_name };
                });
            }
            EngineCommand::SetVideoOutputEnabled { output_id, enabled } => {
                self.update_video_output(output_id, |output| output.enabled = enabled);
            }
            EngineCommand::SetVideoOutputRouting {
                output_id,
                composition_id,
            } => {
                if self.video_composition_exists(composition_id) {
                    self.update_video_output(output_id, |output| {
                        output.composition_id = composition_id
                    });
                } else {
                    self.last_error =
                        Some(format!("Video composition {composition_id} was not found"));
                }
            }
            EngineCommand::SetVideoOutputOpacity { output_id, opacity } => {
                if opacity.is_finite() {
                    self.video_output_fades
                        .retain(|fade| fade.output_id != output_id);
                    self.update_video_output(output_id, |output| {
                        output.opacity = opacity.clamp(0.0, 1.0)
                    });
                } else {
                    self.last_error = Some("Invalid video output opacity".to_string());
                }
            }
            EngineCommand::FadeVideoOutputOpacity {
                output_id,
                opacity,
                duration_ms,
            } => {
                if !opacity.is_finite() {
                    self.last_error = Some("Invalid video output opacity".to_string());
                    return;
                }
                let target_opacity = opacity.clamp(0.0, 1.0);
                let Some(output) = self
                    .video_outputs
                    .iter()
                    .find(|output| output.summary.id == output_id)
                else {
                    self.last_error = Some(format!("Video output {output_id} was not found"));
                    return;
                };
                let start_opacity = output.summary.opacity;
                self.video_output_fades
                    .retain(|fade| fade.output_id != output_id);
                if duration_ms == 0 {
                    self.update_video_output(output_id, |output| {
                        output.opacity = target_opacity;
                    });
                } else {
                    self.video_output_fades.push(RuntimeVideoOutputFade {
                        output_id,
                        started_at: self.last_tick,
                        duration: Duration::from_millis(duration_ms),
                        start_opacity,
                        target_opacity,
                    });
                    self.last_error = None;
                }
            }
            EngineCommand::SetVideoOutputBlackout {
                output_id,
                blackout,
            } => {
                self.update_video_output(output_id, |output| output.blackout = blackout);
            }
            EngineCommand::SetVideoOutputMapping { output_id, mapping } => {
                self.update_video_output(output_id, |output| output.mapping = mapping);
            }
            EngineCommand::SaveVideoOutputMappingPreset { label, mapping } => {
                let preset = VideoOutputMappingPresetSummary { label, mapping };
                if let Some(preset) = sanitize_video_output_mapping_preset(preset) {
                    self.video_output_mapping_presets
                        .retain(|candidate| candidate.label != preset.label);
                    self.video_output_mapping_presets.push(preset);
                    self.video_output_mapping_presets
                        .sort_by(|left, right| left.label.cmp(&right.label));
                    self.last_error = None;
                } else {
                    self.last_error =
                        Some("Video output mapping preset label is required".to_string());
                }
            }
            EngineCommand::RemoveVideoOutputMappingPreset { label } => {
                let label = label.trim();
                let before = self.video_output_mapping_presets.len();
                self.video_output_mapping_presets
                    .retain(|preset| preset.label != label);
                if self.video_output_mapping_presets.len() == before {
                    self.last_error =
                        Some(format!("Video output mapping preset {label} was not found"));
                } else {
                    self.last_error = None;
                }
            }
        }
    }

    fn remove_fixture(&mut self, fixture_id: FixtureId) {
        let before = self.fixtures.len();
        self.fixtures.retain(|fixture| fixture.id != fixture_id);
        if self.fixtures.len() == before {
            self.last_error = Some(format!("Fixture {fixture_id} was not found"));
            return;
        }

        self.values
            .retain(|(candidate_id, _), _| *candidate_id != fixture_id);
        self.highlighted_fixtures.remove(&fixture_id);
        self.soloed_fixtures.remove(&fixture_id);
        self.parked_fixture_values.remove(&fixture_id);
        self.timeline_automations
            .retain(|automation| automation.fixture_id != fixture_id);
        for cue in &mut self.cues {
            cue.targets.retain(|target| target.fixture_id != fixture_id);
        }
        if let Some(fade) = &mut self.active_fade {
            fade.start_values
                .retain(|(candidate_id, _), _| *candidate_id != fixture_id);
            fade.target_values
                .retain(|(candidate_id, _), _| *candidate_id != fixture_id);
        }
        self.effects.retain_mut(|effect| match &mut effect.kind {
            RuntimeEffectKind::Lfo(request) => {
                request.fixture_ids.retain(|id| *id != fixture_id);
                !request.fixture_ids.is_empty()
                    || !request.target_group_ids.is_empty()
                    || !request.video_targets.is_empty()
            }
            RuntimeEffectKind::PositionWave(request) => {
                request.fixture_ids.retain(|id| *id != fixture_id);
                !request.fixture_ids.is_empty()
                    || !request.target_group_ids.is_empty()
                    || !request.video_targets.is_empty()
            }
        });
        self.last_error = None;
    }

    fn tick(&mut self, queue_depth: usize, snapshot: &RwLock<EngineSnapshot>) {
        let now = Instant::now();
        self.last_tick_interval = now.saturating_duration_since(self.last_tick);
        self.last_tick = now;
        self.frame_counter = self.frame_counter.wrapping_add(1);
        self.record_tick_jitter();
        self.advance_timeline(now);
        self.apply_timeline_automations();
        self.apply_timeline_video_automations();
        self.apply_active_fade(now);
        self.advance_video_layers(self.last_tick_interval);
        self.advance_video_output_fades(now);

        let mut frames_by_universe = self.render_dmx_preview_frames(now);
        let frame = frames_by_universe
            .get(&self.output.universe)
            .copied()
            .unwrap_or_else(|| self.render_dmx_frame_for_universe(self.output.universe, now));
        self.last_frame = frame;
        self.last_frames_by_universe = frames_by_universe.clone();

        self.last_packet_bytes = 0;
        self.last_dmx_output_count = 0;
        self.last_dmx_send_success_count = 0;
        self.last_dmx_send_failure_count = 0;
        if self.output.enabled {
            self.last_dmx_output_count = self.last_dmx_output_count.saturating_add(1);
            match send_output_frame(
                &mut self.dmx_sender,
                &self.output,
                self.output.universe,
                &frame,
            ) {
                Ok(bytes) => self.record_dmx_send_success(bytes),
                Err(error) => self.record_dmx_send_failure(error),
            }
        }
        for index in 0..self.additional_dmx_outputs.len() {
            let config = self.additional_dmx_outputs[index].config.clone();
            let frame = frames_by_universe
                .remove(&config.universe)
                .unwrap_or_else(|| self.render_dmx_frame_for_universe(config.universe, now));
            if !config.enabled {
                continue;
            }
            self.last_dmx_output_count = self.last_dmx_output_count.saturating_add(1);
            let output = &mut self.additional_dmx_outputs[index];
            match send_output_frame(&mut output.sender, &output.config, config.universe, &frame) {
                Ok(bytes) => self.record_dmx_send_success(bytes),
                Err(error) => self.record_dmx_send_failure(error),
            }
        }

        if let Ok(mut guard) = snapshot.write() {
            *guard = self.build_snapshot(queue_depth);
        }
    }

    fn record_dmx_send_success(&mut self, bytes: usize) {
        self.last_packet_bytes = self.last_packet_bytes.saturating_add(bytes);
        self.last_dmx_send_success_count = self.last_dmx_send_success_count.saturating_add(1);
        self.total_dmx_send_success_count = self.total_dmx_send_success_count.saturating_add(1);
    }

    fn record_dmx_send_failure(&mut self, error: String) {
        self.last_dmx_send_failure_count = self.last_dmx_send_failure_count.saturating_add(1);
        self.total_dmx_send_failure_count = self.total_dmx_send_failure_count.saturating_add(1);
        self.last_error = Some(error);
    }

    fn record_tick_jitter(&mut self) {
        if self.frame_counter <= 1 {
            return;
        }
        let interval_us = self.last_tick_interval.as_micros() as i128;
        let target_us = DMX_TICK_INTERVAL.as_micros() as i128;
        let jitter = interval_us - target_us;
        self.last_tick_jitter_us = jitter.clamp(i64::MIN as i128, i64::MAX as i128) as i64;
        let abs_jitter = jitter.unsigned_abs().min(u64::MAX as u128) as u64;
        self.tick_jitter_abs_max_us = self.tick_jitter_abs_max_us.max(abs_jitter);

        self.tick_jitter_samples = self.tick_jitter_samples.saturating_add(1);
        let sample = self.last_tick_jitter_us as f64;
        let delta = sample - self.tick_jitter_mean_us;
        self.tick_jitter_mean_us += delta / self.tick_jitter_samples as f64;
        let delta_after = sample - self.tick_jitter_mean_us;
        self.tick_jitter_m2_us += delta * delta_after;
    }

    fn render_dmx_frame_for_universe(&self, universe: u16, now: Instant) -> [u8; 512] {
        let mut frame = [0u8; 512];
        if !self.blackout {
            self.render_dmx_frame(&mut frame, universe, now);
        }
        frame
    }

    fn render_dmx_preview_frames(&self, now: Instant) -> HashMap<u16, [u8; 512]> {
        self.dmx_preview_universes()
            .into_iter()
            .map(|universe| (universe, self.render_dmx_frame_for_universe(universe, now)))
            .collect()
    }

    fn dmx_preview_universes(&self) -> BTreeSet<u16> {
        let mut universes = BTreeSet::new();
        universes.insert(self.output.universe);
        for output in &self.additional_dmx_outputs {
            universes.insert(output.config.universe);
        }
        for fixture in &self.fixtures {
            universes.insert(fixture.request.universe);
        }
        universes
    }

    fn render_dmx_frame(&self, frame: &mut [u8; 512], universe: u16, now: Instant) {
        for fixture in self
            .fixtures
            .iter()
            .filter(|fixture| fixture.request.universe == universe)
        {
            if !self.soloed_fixtures.is_empty() && !self.soloed_fixtures.contains(&fixture.id) {
                continue;
            }
            let Some(mode) = fixture.profile.dmx_modes.get(fixture.mode_index) else {
                continue;
            };

            let parked_values = self.parked_fixture_values.get(&fixture.id);
            for control in &mode.controls {
                let value = self.render_control_value(
                    fixture,
                    mode.controls.as_slice(),
                    control,
                    now,
                    parked_values,
                );
                write_control_value(frame, fixture.request.address, control, value);
            }
        }
    }

    fn render_control_value(
        &self,
        fixture: &RuntimeFixture,
        controls: &[AttributeControl],
        control: &AttributeControl,
        now: Instant,
        parked_values: Option<&HashMap<String, u16>>,
    ) -> u16 {
        let raw_value = self.raw_control_value(fixture, control, now, parked_values);
        apply_fixture_limits(
            &fixture.limits,
            &control.attribute,
            raw_value,
            |attribute| {
                control_for_attribute(controls, attribute)
                    .map(|source_control| {
                        self.raw_control_value(fixture, source_control, now, parked_values)
                    })
                    .unwrap_or(raw_value)
            },
        )
    }

    fn raw_control_value(
        &self,
        fixture: &RuntimeFixture,
        control: &AttributeControl,
        now: Instant,
        parked_values: Option<&HashMap<String, u16>>,
    ) -> u16 {
        parked_values
            .and_then(|values| values.get(&control.attribute))
            .copied()
            .unwrap_or_else(|| self.render_unlimited_control_value(fixture, control, now))
    }

    fn render_unlimited_control_value(
        &self,
        fixture: &RuntimeFixture,
        control: &AttributeControl,
        now: Instant,
    ) -> u16 {
        let value = self
            .values
            .get(&(fixture.id, control.attribute.clone()))
            .copied()
            .unwrap_or(control.default_value);
        let value = self.apply_effects(fixture, &control.attribute, value, now);
        let value = apply_highlight(
            &control.attribute,
            value,
            self.highlighted_fixtures.contains(&fixture.id),
        );
        let value = apply_group_submasters(
            &control.attribute,
            value,
            fixture.request.group_ids.as_slice(),
            &self.group_submaster_levels,
        );
        apply_lighting_master(&control.attribute, value, self.lighting_master)
    }

    fn capture_park_values(
        &self,
        fixture_id: FixtureId,
        now: Instant,
    ) -> Option<HashMap<String, u16>> {
        let fixture = self
            .fixtures
            .iter()
            .find(|fixture| fixture.id == fixture_id)?;
        let mode = fixture.profile.dmx_modes.get(fixture.mode_index)?;
        let values = mode
            .controls
            .iter()
            .map(|control| {
                (
                    control.attribute.clone(),
                    self.render_unlimited_control_value(fixture, control, now),
                )
            })
            .collect::<HashMap<_, _>>();
        Some(values)
    }

    fn fixture_ids_in_group(&self, group_id: &str) -> Vec<FixtureId> {
        self.fixtures
            .iter()
            .filter(|fixture| fixture.request.group_ids.iter().any(|id| id == group_id))
            .map(|fixture| fixture.id)
            .collect()
    }

    fn apply_effects(
        &self,
        fixture: &RuntimeFixture,
        attribute: &str,
        base_value: u16,
        now: Instant,
    ) -> u16 {
        let mut value = base_value;
        for effect in &self.effects {
            if !effect.enabled || !effect_targets_fixture_attribute(effect, fixture, attribute) {
                continue;
            }
            let (effect_value, blend_mode) = match &effect.kind {
                RuntimeEffectKind::Lfo(request) => (
                    evaluate_lfo_effect(request, effect.created_at, now),
                    &request.blend_mode,
                ),
                RuntimeEffectKind::PositionWave(request) => (
                    evaluate_position_wave_effect(
                        request,
                        fixture.request.position,
                        effect.created_at,
                        now,
                    ),
                    &request.blend_mode,
                ),
            };
            value = blend_effect_value(value, effect_value, blend_mode);
        }
        value
    }

    fn start_cue(&mut self, cue_id: CueId, now: Instant) {
        let Some(cue) = self.cues.iter().find(|cue| cue.id == cue_id).cloned() else {
            self.last_error = Some(format!("Cue {cue_id} was not found"));
            return;
        };

        self.active_cue_id = Some(cue_id);
        let target_values = cue_target_values(&cue.targets);
        let video_target_states = cue_video_target_states(&cue.video_targets, &self.video_layers);
        let video_output_targets =
            cue_video_output_targets(&cue.video_output_targets, &self.video_outputs);
        if target_values.is_empty()
            && video_target_states.is_empty()
            && video_output_targets.is_empty()
        {
            self.active_fade = None;
            self.last_error = None;
            return;
        }
        let video_start_states = video_target_states
            .keys()
            .filter_map(|layer_id| {
                self.video_layers
                    .iter()
                    .find(|layer| layer.id == *layer_id)
                    .map(|layer| (*layer_id, layer.state.clone()))
            })
            .collect::<HashMap<_, _>>();
        let start_values = target_values
            .keys()
            .map(|key| (key.clone(), self.values.get(key).copied().unwrap_or(0)))
            .collect::<HashMap<_, _>>();
        let video_output_start_opacities = video_output_targets
            .iter()
            .filter_map(|(output_id, _)| {
                self.video_outputs
                    .iter()
                    .find(|output| output.summary.id == *output_id)
                    .map(|output| (*output_id, output.summary.opacity.clamp(0.0, 1.0)))
            })
            .collect::<HashMap<_, _>>();
        let video_output_target_opacities = video_output_targets
            .iter()
            .map(|(output_id, target)| (*output_id, target.opacity.clamp(0.0, 1.0)))
            .collect::<HashMap<_, _>>();
        let duration = Duration::from_millis(cue.fade_ms);
        if duration.is_zero() {
            for (key, value) in target_values {
                self.values.insert(key, value);
            }
            for (layer_id, state) in video_target_states {
                self.apply_video_target_state(layer_id, state);
            }
            for (_, target) in video_output_targets {
                self.apply_video_output_target(target, true);
            }
            self.active_fade = None;
        } else {
            for (layer_id, target_state) in &video_target_states {
                if let Some(layer) = self
                    .video_layers
                    .iter_mut()
                    .find(|layer| layer.id == *layer_id)
                {
                    layer.state = video_trigger_state(&layer.state, target_state);
                }
            }
            for (_, target) in &video_output_targets {
                self.apply_video_output_target(target.clone(), false);
            }
            self.active_fade = Some(RuntimeFade {
                cue_id,
                started_at: now,
                duration,
                paused_at: None,
                paused_duration: Duration::ZERO,
                start_values,
                target_values,
                video_start_states,
                video_target_states,
                video_output_start_opacities,
                video_output_target_opacities,
            });
        }
        self.last_error = None;
    }

    fn apply_video_target_state(&mut self, layer_id: VideoLayerId, state: VideoLayerState) {
        if let Some(layer) = self
            .video_layers
            .iter_mut()
            .find(|layer| layer.id == layer_id)
        {
            layer.state = video::sanitize_layer_state(state);
        }
    }

    fn apply_video_output_target(&mut self, target: VideoOutputTarget, include_opacity: bool) {
        self.video_output_fades
            .retain(|fade| fade.output_id != target.output_id);
        if let Some(output) = self
            .video_outputs
            .iter_mut()
            .find(|output| output.summary.id == target.output_id)
        {
            output.summary.enabled = target.enabled;
            output.summary.blackout = target.blackout;
            if include_opacity {
                output.summary.opacity = target.opacity.clamp(0.0, 1.0);
            }
        }
    }

    fn advance_video_layers(&mut self, delta: Duration) {
        let bpm = self.clock.snapshot(self.last_tick).bpm;
        for layer in &mut self.video_layers {
            let source_duration_ms = layer
                .source
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.duration_ms);
            layer.state = video::advance_layer_state_with_bpm_and_duration(
                layer.state.clone(),
                delta,
                Some(bpm),
                source_duration_ms,
            );
        }
    }

    fn advance_video_output_fades(&mut self, now: Instant) {
        if self.video_output_fades.is_empty() {
            return;
        }
        let mut completed = Vec::new();
        for fade in &self.video_output_fades {
            let elapsed = now.saturating_duration_since(fade.started_at);
            let progress = if fade.duration.is_zero() {
                1.0
            } else {
                (elapsed.as_secs_f32() / fade.duration.as_secs_f32()).clamp(0.0, 1.0)
            };
            if let Some(output) = self
                .video_outputs
                .iter_mut()
                .find(|output| output.summary.id == fade.output_id)
            {
                output.summary.opacity = (fade.start_opacity
                    + (fade.target_opacity - fade.start_opacity) * progress)
                    .clamp(0.0, 1.0);
            }
            if progress >= 1.0 {
                completed.push(fade.output_id);
            }
        }
        self.video_output_fades
            .retain(|fade| !completed.contains(&fade.output_id));
    }

    fn apply_active_fade(&mut self, now: Instant) {
        let Some(fade) = &self.active_fade else {
            return;
        };
        if fade.paused_at.is_some() {
            return;
        }
        let progress = fade_progress(fade, now);
        for (key, target) in &fade.target_values {
            let start = fade.start_values.get(key).copied().unwrap_or(0);
            let value = interpolate_u16(start, *target, progress);
            self.values.insert(key.clone(), value);
        }
        let video_updates = fade
            .video_target_states
            .iter()
            .filter_map(|(layer_id, target_state)| {
                let start_state = fade.video_start_states.get(layer_id)?;
                Some((
                    *layer_id,
                    interpolate_video_layer_state(start_state, target_state, progress),
                ))
            })
            .collect::<Vec<_>>();
        for (layer_id, state) in video_updates {
            if let Some(layer) = self
                .video_layers
                .iter_mut()
                .find(|layer| layer.id == layer_id)
            {
                layer.state = state;
            }
        }
        for (output_id, target_opacity) in &fade.video_output_target_opacities {
            let start_opacity = fade
                .video_output_start_opacities
                .get(output_id)
                .copied()
                .unwrap_or(1.0);
            if let Some(output) = self
                .video_outputs
                .iter_mut()
                .find(|output| output.summary.id == *output_id)
            {
                output.summary.opacity =
                    (start_opacity + (target_opacity - start_opacity) * progress).clamp(0.0, 1.0);
            }
        }
        if progress >= 1.0 {
            self.active_fade = None;
        }
    }

    fn set_active_fade_paused(&mut self, paused: bool, now: Instant) {
        let Some(fade) = &mut self.active_fade else {
            self.last_error = None;
            return;
        };
        match (paused, fade.paused_at) {
            (true, None) => {
                fade.paused_at = Some(now);
            }
            (false, Some(paused_at)) => {
                fade.paused_duration += now.saturating_duration_since(paused_at);
                fade.paused_at = None;
            }
            _ => {}
        }
        self.last_error = None;
    }

    fn advance_timeline(&mut self, now: Instant) {
        if !self.timeline_playing {
            return;
        }
        let duration = self.timeline_duration_ms();
        if duration == 0 {
            self.timeline_playing = false;
            return;
        }

        let previous_position = self.timeline_position_ms;
        let delta_ms = self
            .last_tick_interval
            .as_millis()
            .try_into()
            .unwrap_or(u64::MAX);
        let current_position = previous_position.saturating_add(delta_ms).min(duration);
        self.timeline_position_ms = current_position;

        self.trigger_timeline_events_between(previous_position, current_position, now);

        if current_position >= duration {
            self.timeline_playing = false;
        }
    }

    fn sync_timeline_position(&mut self, position_ms: u64, now: Instant) {
        let current_position = position_ms.min(self.timeline_duration_ms());
        let previous_position = self.timeline_position_ms;
        self.timeline_position_ms = current_position;
        self.trigger_timeline_events_between(previous_position, current_position, now);
    }

    fn trigger_timeline_events_between(
        &mut self,
        previous_position: u64,
        current_position: u64,
        now: Instant,
    ) {
        if current_position < previous_position {
            return;
        }
        let due_cues = self
            .timeline_events
            .iter()
            .filter(|event| {
                event.time_ms >= previous_position
                    && event.time_ms <= current_position
                    && (event.time_ms > previous_position || previous_position == 0)
            })
            .map(|event| event.cue_id)
            .collect::<Vec<_>>();
        for cue_id in due_cues {
            self.start_cue(cue_id, now);
        }
    }

    fn apply_timeline_automations(&mut self) {
        for automation in self
            .timeline_automations
            .iter()
            .filter(|automation| automation.enabled)
        {
            if !matches!(automation.track, TimelineTrackKind::Lighting) {
                continue;
            }
            let Some(value) =
                evaluate_automation_keyframes(&automation.keyframes, self.timeline_position_ms)
            else {
                continue;
            };
            self.values
                .insert((automation.fixture_id, automation.attribute.clone()), value);
        }
    }

    fn apply_timeline_video_automations(&mut self) {
        let updates = self
            .timeline_video_automations
            .iter()
            .filter(|automation| automation.enabled)
            .filter_map(|automation| {
                let value = evaluate_video_automation_keyframes(
                    &automation.keyframes,
                    self.timeline_position_ms,
                )?;
                Some((automation.layer_id, automation.param.clone(), value))
            })
            .collect::<Vec<_>>();

        for (layer_id, param, value) in updates {
            if let Some(layer) = self
                .video_layers
                .iter_mut()
                .find(|layer| layer.id == layer_id)
            {
                apply_video_param(&mut layer.state, &param, value);
                layer.state = video::sanitize_layer_state(layer.state.clone());
            }
        }
    }

    fn relative_cue_id(&self, offset: isize) -> Option<CueId> {
        if self.cues.is_empty() {
            return None;
        }
        let current_index = self
            .active_cue_id
            .and_then(|cue_id| self.cues.iter().position(|cue| cue.id == cue_id));
        let index = match current_index {
            Some(index) => (index as isize + offset).rem_euclid(self.cues.len() as isize) as usize,
            None if offset < 0 => self.cues.len() - 1,
            None => 0,
        };
        self.cues.get(index).map(|cue| cue.id)
    }

    fn timeline_duration_ms(&self) -> u64 {
        let event_duration = self
            .timeline_events
            .iter()
            .map(|event| event.time_ms)
            .max()
            .unwrap_or(0);
        let automation_duration = self
            .timeline_automations
            .iter()
            .flat_map(|automation| automation.keyframes.iter().map(|keyframe| keyframe.time_ms))
            .max()
            .unwrap_or(0);
        let video_automation_duration = self
            .timeline_video_automations
            .iter()
            .flat_map(|automation| automation.keyframes.iter().map(|keyframe| keyframe.time_ms))
            .max()
            .unwrap_or(0);
        let audio_duration = self
            .timeline_audio
            .as_ref()
            .map(|audio| audio.duration_ms)
            .unwrap_or(0);
        let video_source_duration = self
            .video_layers
            .iter()
            .filter_map(|layer| layer.source.metadata.as_ref())
            .filter_map(|metadata| metadata.duration_ms)
            .max()
            .unwrap_or(0);
        event_duration
            .max(automation_duration)
            .max(video_automation_duration)
            .max(audio_duration)
            .max(video_source_duration)
    }

    fn timeline_snapshot(&self) -> TimelineSnapshot {
        TimelineSnapshot {
            events: self
                .timeline_events
                .iter()
                .map(timeline_event_summary)
                .collect(),
            automations: self
                .timeline_automations
                .iter()
                .map(timeline_automation_summary)
                .collect(),
            video_automations: self
                .timeline_video_automations
                .iter()
                .map(timeline_video_automation_summary)
                .collect(),
            audio: self.timeline_audio.clone(),
            playing: self.timeline_playing,
            position_ms: self.timeline_position_ms,
            duration_ms: self.timeline_duration_ms(),
        }
    }

    fn video_snapshot(&self) -> VideoSnapshot {
        let now = self.last_tick;
        let outputs = self
            .video_outputs
            .iter()
            .map(|output| output.summary.clone())
            .collect::<Vec<_>>();
        let main_layer_ids = self
            .video_layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        let mut compositions = vec![CompositionSummary {
            id: 1,
            label: "Main".to_string(),
            layer_ids: main_layer_ids,
            output_ids: output_ids_for_composition(&outputs, 1),
        }];
        compositions.extend(self.video_compositions.iter().map(|composition| {
            let mut summary =
                sanitize_video_composition(composition.summary.clone(), &self.video_layers);
            summary.output_ids = output_ids_for_composition(&outputs, summary.id);
            summary
        }));
        VideoSnapshot {
            layers: self
                .video_layers
                .iter()
                .map(|layer| self.video_layer_summary_with_effects(layer, now))
                .collect(),
            compositions,
            outputs,
            mapping_presets: self.video_output_mapping_presets.clone(),
            master_opacity: self.video_master_opacity,
            blackout: self.video_blackout,
        }
    }

    fn video_composition_exists(&self, composition_id: CompositionId) -> bool {
        composition_id == 1
            || self
                .video_compositions
                .iter()
                .any(|composition| composition.summary.id == composition_id)
    }

    fn duplicate_video_layer(
        &mut self,
        source_layer_id: VideoLayerId,
        new_layer_id: VideoLayerId,
        label: String,
    ) {
        if source_layer_id == new_layer_id
            || self
                .video_layers
                .iter()
                .any(|layer| layer.id == new_layer_id)
        {
            self.last_error = Some(format!("Video layer {new_layer_id} already exists"));
            return;
        }

        let Some(source_index) = self
            .video_layers
            .iter()
            .position(|layer| layer.id == source_layer_id)
        else {
            self.last_error = Some(format!("Video layer {source_layer_id} was not found"));
            return;
        };

        let mut duplicate = self.video_layers[source_index].clone();
        duplicate.id = new_layer_id;
        duplicate.label = sanitize_video_layer_label(label, new_layer_id);
        self.video_layers.insert(source_index + 1, duplicate);

        for composition in &mut self.video_compositions {
            if composition.summary.layer_ids.contains(&new_layer_id) {
                continue;
            }
            if let Some(index) = composition
                .summary
                .layer_ids
                .iter()
                .position(|layer_id| *layer_id == source_layer_id)
            {
                composition
                    .summary
                    .layer_ids
                    .insert(index + 1, new_layer_id);
            }
        }
        self.last_error = None;
    }

    fn update_video_output<F>(&mut self, output_id: VideoOutputId, update: F)
    where
        F: FnOnce(&mut VideoOutputSummary),
    {
        if let Some(output) = self
            .video_outputs
            .iter_mut()
            .find(|output| output.summary.id == output_id)
        {
            update(&mut output.summary);
            output.summary = sanitize_video_output(output.summary.clone());
            self.last_error = None;
        } else {
            self.last_error = Some(format!("Video output {output_id} was not found"));
        }
    }

    fn video_layer_summary_with_effects(
        &self,
        layer: &RuntimeVideoLayer,
        now: Instant,
    ) -> VideoLayerSummary {
        let mut summary = video_layer_summary(layer);
        self.apply_video_effects(layer.id, &mut summary.state, now);
        summary
    }

    fn apply_video_effects(
        &self,
        layer_id: VideoLayerId,
        state: &mut VideoLayerState,
        now: Instant,
    ) {
        for effect in &self.effects {
            if !effect.enabled {
                continue;
            }
            match &effect.kind {
                RuntimeEffectKind::Lfo(request) => {
                    for target in request
                        .video_targets
                        .iter()
                        .filter(|target| target.layer_ids.contains(&layer_id))
                    {
                        let value =
                            evaluate_lfo_video_effect(request, target, effect.created_at, now);
                        apply_video_effect_param(state, &target.param, value, &request.blend_mode);
                    }
                }
                RuntimeEffectKind::PositionWave(request) => {
                    for target in request
                        .video_targets
                        .iter()
                        .filter(|target| target.layer_ids.contains(&layer_id))
                    {
                        let value = evaluate_position_wave_video_effect(
                            request,
                            target,
                            Vec3::default(),
                            effect.created_at,
                            now,
                        );
                        apply_video_effect_param(state, &target.param, value, &request.blend_mode);
                    }
                }
            }
        }
        *state = video::sanitize_layer_state(state.clone());
    }

    fn active_fade_summary(&self, now: Instant) -> Option<ActiveFadeSummary> {
        self.active_fade.as_ref().map(|fade| {
            let progress = fade_progress(fade, now);
            let elapsed = fade_elapsed(fade, now);
            let remaining_ms = fade
                .duration
                .saturating_sub(elapsed)
                .as_millis()
                .try_into()
                .unwrap_or(u64::MAX);
            ActiveFadeSummary {
                cue_id: fade.cue_id,
                progress,
                remaining_ms,
                paused: fade.paused_at.is_some(),
            }
        })
    }

    fn build_snapshot(&self, queue_depth: usize) -> EngineSnapshot {
        let fixtures = self
            .fixtures
            .iter()
            .filter_map(|fixture| {
                let mode = fixture.profile.dmx_modes.get(fixture.mode_index)?;
                Some(PatchedFixtureSummary {
                    id: fixture.id,
                    label: fixture.request.label.clone(),
                    profile_name: fixture.profile.name.clone(),
                    manufacturer: fixture.profile.manufacturer.clone(),
                    mode_name: mode.name.clone(),
                    universe: fixture.request.universe,
                    address: fixture.request.address,
                    group_ids: fixture.request.group_ids.clone(),
                    position: fixture.request.position,
                    rotation: fixture.request.rotation,
                    controls: mode.controls.clone(),
                    attribute_values: self
                        .fixture_attribute_values(fixture, mode.controls.as_slice()),
                    limits: fixture.limits,
                    highlighted: self.highlighted_fixtures.contains(&fixture.id),
                    soloed: self.soloed_fixtures.contains(&fixture.id),
                    parked: self.parked_fixture_values.contains_key(&fixture.id),
                })
            })
            .collect();

        EngineSnapshot {
            fixtures,
            cues: self.cues.iter().map(cue_summary).collect(),
            active_cue_id: self.active_cue_id,
            active_fade: self.active_fade_summary(self.last_tick),
            timeline: self.timeline_snapshot(),
            video: self.video_snapshot(),
            effects: self.effects.iter().map(effect_summary).collect(),
            output: self.output.clone(),
            dmx_outputs: self.dmx_output_snapshot(),
            lighting_master: self.lighting_master,
            submasters: self.submaster_snapshot(),
            blackout: self.blackout,
            clock: self.clock.snapshot(self.last_tick),
            dmx_preview: self.last_frame.to_vec(),
            dmx_previews: self.dmx_preview_snapshot(),
            telemetry: EngineTelemetry {
                frame_counter: self.frame_counter,
                queue_depth,
                last_tick_interval_us: self.last_tick_interval.as_micros() as u64,
                tick_jitter_last_us: self.last_tick_jitter_us,
                tick_jitter_abs_max_us: self.tick_jitter_abs_max_us,
                tick_jitter_stddev_us: self.tick_jitter_stddev_us(),
                tick_jitter_samples: self.tick_jitter_samples,
                last_packet_bytes: self.last_packet_bytes,
                last_dmx_output_count: self.last_dmx_output_count,
                last_dmx_send_success_count: self.last_dmx_send_success_count,
                last_dmx_send_failure_count: self.last_dmx_send_failure_count,
                total_dmx_send_success_count: self.total_dmx_send_success_count,
                total_dmx_send_failure_count: self.total_dmx_send_failure_count,
                last_error: self.last_error.clone(),
            },
        }
    }

    fn tick_jitter_stddev_us(&self) -> f32 {
        if self.tick_jitter_samples <= 1 {
            return 0.0;
        }
        (self.tick_jitter_m2_us / (self.tick_jitter_samples - 1) as f64).sqrt() as f32
    }

    fn dmx_output_snapshot(&self) -> Vec<DmxOutputConfig> {
        std::iter::once(self.output.clone())
            .chain(
                self.additional_dmx_outputs
                    .iter()
                    .map(|output| output.config.clone()),
            )
            .collect()
    }

    fn dmx_preview_snapshot(&self) -> Vec<DmxUniversePreview> {
        self.dmx_preview_universes()
            .into_iter()
            .map(|universe| DmxUniversePreview {
                universe,
                values: self
                    .last_frames_by_universe
                    .get(&universe)
                    .copied()
                    .unwrap_or([0u8; 512])
                    .to_vec(),
            })
            .collect()
    }

    fn submaster_snapshot(&self) -> Vec<SubmasterSummary> {
        let mut group_ids = BTreeSet::new();
        for fixture in &self.fixtures {
            group_ids.extend(fixture.request.group_ids.iter().cloned());
        }
        group_ids.extend(self.group_submaster_levels.keys().cloned());
        group_ids
            .into_iter()
            .map(|group_id| SubmasterSummary {
                level: self
                    .group_submaster_levels
                    .get(&group_id)
                    .copied()
                    .unwrap_or(1.0),
                label: group_id.clone(),
                group_id,
            })
            .collect()
    }

    fn fixture_attribute_values(
        &self,
        fixture: &RuntimeFixture,
        controls: &[AttributeControl],
    ) -> Vec<AttributeValueSummary> {
        controls
            .iter()
            .map(|control| AttributeValueSummary {
                attribute: control.attribute.clone(),
                value: self
                    .values
                    .get(&(fixture.id, control.attribute.clone()))
                    .copied()
                    .unwrap_or(control.default_value),
            })
            .collect()
    }
}

fn cue_summary(cue: &RuntimeCue) -> CueSummary {
    CueSummary {
        id: cue.id,
        label: cue.label.clone(),
        fade_ms: cue.fade_ms,
        targets: cue.targets.clone(),
        video_targets: cue.video_targets.clone(),
        video_output_targets: cue.video_output_targets.clone(),
    }
}

fn runtime_fixture_from_snapshot(fixture: &PatchedFixtureSummary) -> RuntimeFixture {
    RuntimeFixture {
        id: fixture.id,
        request: PatchFixtureRequest {
            profile_path: format!("snapshot://fixture/{}", fixture.id),
            mode_name: Some(fixture.mode_name.clone()),
            label: fixture.label.clone(),
            universe: fixture.universe,
            address: fixture.address,
            group_ids: fixture.group_ids.clone(),
            position: fixture.position,
            rotation: fixture.rotation,
        },
        profile: FixtureProfileSummary {
            source_path: format!("snapshot://fixture/{}", fixture.id),
            manufacturer: fixture.manufacturer.clone(),
            name: fixture.profile_name.clone(),
            short_name: None,
            fixture_type_id: None,
            dmx_modes: vec![DmxModeSummary {
                name: fixture.mode_name.clone(),
                controls: fixture.controls.clone(),
            }],
            geometries: Vec::<GeometrySummary>::new(),
            warnings: vec!["Loaded from KDMX project snapshot".to_string()],
        },
        mode_index: 0,
        limits: normalized_fixture_limits(fixture.limits),
    }
}

fn runtime_cue_from_summary(cue: &CueSummary) -> RuntimeCue {
    RuntimeCue {
        id: cue.id,
        label: cue.label.clone(),
        fade_ms: cue.fade_ms,
        targets: cue.targets.clone(),
        video_targets: cue.video_targets.clone(),
        video_output_targets: cue.video_output_targets.clone(),
    }
}

fn video_layer_summary(layer: &RuntimeVideoLayer) -> VideoLayerSummary {
    VideoLayerSummary {
        id: layer.id,
        label: layer.label.clone(),
        source: layer.source.clone(),
        blend_mode: layer.blend_mode.clone(),
        state: layer.state.clone(),
    }
}

fn runtime_video_layer_from_summary(layer: &VideoLayerSummary) -> RuntimeVideoLayer {
    RuntimeVideoLayer {
        id: layer.id,
        label: sanitize_video_layer_label(layer.label.clone(), layer.id),
        source: layer.source.clone(),
        blend_mode: layer.blend_mode.clone(),
        state: video::sanitize_layer_state(layer.state.clone()),
    }
}

fn timeline_event_summary(event: &RuntimeTimelineEvent) -> TimelineCueEventSummary {
    TimelineCueEventSummary {
        id: event.id,
        cue_id: event.cue_id,
        time_ms: event.time_ms,
        track: event.track.clone(),
    }
}

fn runtime_timeline_event_from_summary(event: &TimelineCueEventSummary) -> RuntimeTimelineEvent {
    RuntimeTimelineEvent {
        id: event.id,
        cue_id: event.cue_id,
        time_ms: event.time_ms,
        track: event.track.clone(),
    }
}

fn timeline_automation_summary(
    automation: &RuntimeTimelineAutomation,
) -> TimelineAutomationSummary {
    TimelineAutomationSummary {
        id: automation.id,
        fixture_id: automation.fixture_id,
        attribute: automation.attribute.clone(),
        track: automation.track.clone(),
        keyframes: automation.keyframes.clone(),
        enabled: automation.enabled,
    }
}

fn runtime_timeline_automation_from_summary(
    automation: &TimelineAutomationSummary,
) -> RuntimeTimelineAutomation {
    RuntimeTimelineAutomation {
        id: automation.id,
        fixture_id: automation.fixture_id,
        attribute: automation.attribute.clone(),
        track: automation.track.clone(),
        keyframes: automation.keyframes.clone(),
        enabled: automation.enabled,
    }
}

fn timeline_video_automation_summary(
    automation: &RuntimeTimelineVideoAutomation,
) -> TimelineVideoAutomationSummary {
    TimelineVideoAutomationSummary {
        id: automation.id,
        layer_id: automation.layer_id,
        param: automation.param.clone(),
        track: automation.track.clone(),
        keyframes: automation.keyframes.clone(),
        enabled: automation.enabled,
    }
}

fn runtime_timeline_video_automation_from_summary(
    automation: &TimelineVideoAutomationSummary,
) -> RuntimeTimelineVideoAutomation {
    RuntimeTimelineVideoAutomation {
        id: automation.id,
        layer_id: automation.layer_id,
        param: automation.param.clone(),
        track: automation.track.clone(),
        keyframes: automation.keyframes.clone(),
        enabled: automation.enabled,
    }
}

fn cue_target_values(targets: &[CueFixtureTarget]) -> HashMap<(FixtureId, String), u16> {
    let mut values = HashMap::new();
    for target in targets {
        for value in &target.values {
            values.insert((target.fixture_id, value.attribute.clone()), value.value);
        }
    }
    values
}

fn cue_video_target_states(
    targets: &[VideoLayerTarget],
    layers: &[RuntimeVideoLayer],
) -> HashMap<VideoLayerId, VideoLayerState> {
    let mut states = HashMap::new();
    for target in targets {
        if layers.iter().any(|layer| layer.id == target.layer_id) {
            states.insert(
                target.layer_id,
                video::sanitize_layer_state(target.state.clone()),
            );
        }
    }
    states
}

fn cue_video_output_targets(
    targets: &[VideoOutputTarget],
    outputs: &[RuntimeVideoOutput],
) -> HashMap<VideoOutputId, VideoOutputTarget> {
    let mut states = HashMap::new();
    for target in targets {
        if outputs
            .iter()
            .any(|output| output.summary.id == target.output_id)
        {
            let mut target = target.clone();
            target.opacity = target.opacity.clamp(0.0, 1.0);
            states.insert(target.output_id, target);
        }
    }
    states
}

fn set_fixture_flag(flags: &mut HashSet<FixtureId>, fixture_id: FixtureId, enabled: bool) {
    if enabled {
        flags.insert(fixture_id);
    } else {
        flags.remove(&fixture_id);
    }
}

fn apply_highlight(attribute: &str, value: u16, highlighted: bool) -> u16 {
    if highlighted && is_highlight_attribute(attribute) {
        65_535
    } else {
        value
    }
}

fn apply_group_submasters(
    attribute: &str,
    value: u16,
    group_ids: &[String],
    submaster_levels: &HashMap<String, f32>,
) -> u16 {
    if !is_mastered_intensity_attribute(attribute) || group_ids.is_empty() {
        return value;
    }
    let level = group_ids
        .iter()
        .filter_map(|group_id| submaster_levels.get(group_id))
        .fold(1.0, |acc, level| acc * level.clamp(0.0, 1.0));
    scale_u16(value, level)
}

fn is_highlight_attribute(attribute: &str) -> bool {
    let normalized = attribute.to_ascii_lowercase();
    is_mastered_intensity_attribute(attribute)
        || normalized.contains("colorred")
        || normalized == "red"
        || normalized.contains("colorgreen")
        || normalized == "green"
        || normalized.contains("colorblue")
        || normalized == "blue"
        || normalized.contains("colorwhite")
        || normalized == "white"
}

fn apply_lighting_master(attribute: &str, value: u16, master: f32) -> u16 {
    if is_mastered_intensity_attribute(attribute) {
        scale_u16(value, master)
    } else {
        value
    }
}

fn is_mastered_intensity_attribute(attribute: &str) -> bool {
    let normalized = attribute.to_ascii_lowercase();
    normalized.contains("dimmer") || normalized.contains("intensity")
}

fn scale_u16(value: u16, scale: f32) -> u16 {
    if !scale.is_finite() {
        return value;
    }
    (value as f32 * scale.clamp(0.0, 1.0))
        .round()
        .clamp(0.0, 65_535.0) as u16
}

fn fade_progress(fade: &RuntimeFade, now: Instant) -> f32 {
    if fade.duration.is_zero() {
        return 1.0;
    }
    (fade_elapsed(fade, now).as_secs_f32() / fade.duration.as_secs_f32()).clamp(0.0, 1.0)
}

fn fade_elapsed(fade: &RuntimeFade, now: Instant) -> Duration {
    let effective_now = fade.paused_at.unwrap_or(now);
    effective_now
        .saturating_duration_since(fade.started_at)
        .saturating_sub(fade.paused_duration)
}

fn video_trigger_state(start: &VideoLayerState, target: &VideoLayerState) -> VideoLayerState {
    interpolate_video_layer_state(start, target, 0.0)
}

fn interpolate_video_layer_state(
    start: &VideoLayerState,
    target: &VideoLayerState,
    progress: f32,
) -> VideoLayerState {
    let progress = progress.clamp(0.0, 1.0);
    let mut state = target.clone();
    state.opacity = interpolate_f32(start.opacity, target.opacity, progress);
    state.speed = interpolate_f32(start.speed, target.speed, progress);
    state.transform = interpolate_transform(start.transform, target.transform, progress);
    state.color = interpolate_color(start.color, target.color, progress);
    state.fx = interpolate_fx(start.fx, target.fx, progress);
    video::sanitize_layer_state(state)
}

fn interpolate_transform(start: Transform2D, target: Transform2D, progress: f32) -> Transform2D {
    Transform2D {
        x: interpolate_f32(start.x, target.x, progress),
        y: interpolate_f32(start.y, target.y, progress),
        scale_x: interpolate_f32(start.scale_x, target.scale_x, progress),
        scale_y: interpolate_f32(start.scale_y, target.scale_y, progress),
        rotation_deg: interpolate_f32(start.rotation_deg, target.rotation_deg, progress),
        crop_left: interpolate_f32(start.crop_left, target.crop_left, progress),
        crop_top: interpolate_f32(start.crop_top, target.crop_top, progress),
        crop_right: interpolate_f32(start.crop_right, target.crop_right, progress),
        crop_bottom: interpolate_f32(start.crop_bottom, target.crop_bottom, progress),
    }
}

fn interpolate_color(
    start: VideoColorAdjust,
    target: VideoColorAdjust,
    progress: f32,
) -> VideoColorAdjust {
    VideoColorAdjust {
        brightness: interpolate_f32(start.brightness, target.brightness, progress),
        contrast: interpolate_f32(start.contrast, target.contrast, progress),
        hue_deg: interpolate_f32(start.hue_deg, target.hue_deg, progress),
        saturation: interpolate_f32(start.saturation, target.saturation, progress),
        gamma: interpolate_f32(start.gamma, target.gamma, progress),
    }
}

fn interpolate_fx(start: VideoFxAdjust, target: VideoFxAdjust, progress: f32) -> VideoFxAdjust {
    VideoFxAdjust {
        pixelate: interpolate_f32(start.pixelate, target.pixelate, progress),
        blur: interpolate_f32(start.blur, target.blur, progress),
        glow: interpolate_f32(start.glow, target.glow, progress),
        edge: interpolate_f32(start.edge, target.edge, progress),
        key_red: interpolate_f32(start.key_red, target.key_red, progress),
        key_green: interpolate_f32(start.key_green, target.key_green, progress),
        key_blue: interpolate_f32(start.key_blue, target.key_blue, progress),
        key_threshold: interpolate_f32(start.key_threshold, target.key_threshold, progress),
    }
}

fn interpolate_f32(start: f32, target: f32, progress: f32) -> f32 {
    start + (target - start) * progress
}

fn interpolate_u16(start: u16, target: u16, progress: f32) -> u16 {
    (start as f32 + (target as f32 - start as f32) * progress)
        .round()
        .clamp(0.0, 65_535.0) as u16
}

fn evaluate_automation_keyframes(
    keyframes: &[AutomationKeyframeSummary],
    position_ms: u64,
) -> Option<u16> {
    let first = keyframes.first()?;
    if position_ms <= first.time_ms {
        return Some(first.value);
    }

    for pair in keyframes.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        if position_ms < current.time_ms || position_ms > next.time_ms {
            continue;
        }
        return Some(match current.interpolation {
            AutomationInterpolation::Step => current.value,
            AutomationInterpolation::Linear | AutomationInterpolation::Bezier => {
                let progress = automation_segment_progress(
                    current.time_ms,
                    next.time_ms,
                    position_ms,
                    &current.interpolation,
                );
                interpolate_u16(current.value, next.value, progress)
            }
        });
    }

    keyframes.last().map(|keyframe| keyframe.value)
}

fn evaluate_video_automation_keyframes(
    keyframes: &[VideoAutomationKeyframeSummary],
    position_ms: u64,
) -> Option<f32> {
    let first = keyframes.first()?;
    if position_ms <= first.time_ms {
        return Some(first.value);
    }

    for pair in keyframes.windows(2) {
        let current = &pair[0];
        let next = &pair[1];
        if position_ms < current.time_ms || position_ms > next.time_ms {
            continue;
        }
        return Some(match current.interpolation {
            AutomationInterpolation::Step => current.value,
            AutomationInterpolation::Linear | AutomationInterpolation::Bezier => {
                let progress = automation_segment_progress(
                    current.time_ms,
                    next.time_ms,
                    position_ms,
                    &current.interpolation,
                );
                current.value + (next.value - current.value) * progress
            }
        });
    }

    keyframes.last().map(|keyframe| keyframe.value)
}

fn automation_segment_progress(
    start_ms: u64,
    end_ms: u64,
    position_ms: u64,
    interpolation: &AutomationInterpolation,
) -> f32 {
    let span = end_ms.saturating_sub(start_ms);
    let linear = if span == 0 {
        1.0
    } else {
        (position_ms.saturating_sub(start_ms) as f32 / span as f32).clamp(0.0, 1.0)
    };
    match interpolation {
        AutomationInterpolation::Step => 0.0,
        AutomationInterpolation::Linear => linear,
        AutomationInterpolation::Bezier => cubic_ease_in_out(linear),
    }
}

fn cubic_ease_in_out(progress: f32) -> f32 {
    let t = progress.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn apply_video_param(state: &mut VideoLayerState, param: &VideoParam, value: f32) {
    match param {
        VideoParam::Opacity => {
            state.opacity = value;
        }
        VideoParam::Speed => {
            state.speed = value;
        }
        VideoParam::PositionMs => {
            state.position_ms = value.max(0.0).round() as u64;
        }
        VideoParam::BpmSyncEnabled => {
            state.bpm_sync.enabled = value > 0.0;
        }
        VideoParam::BpmSyncRatio => {
            state.bpm_sync.ratio = value;
        }
        VideoParam::BpmSyncLoopBars => {
            state.bpm_sync.loop_bars = value;
        }
        VideoParam::TransformX => {
            state.transform.x = value;
        }
        VideoParam::TransformY => {
            state.transform.y = value;
        }
        VideoParam::TransformScaleX => {
            state.transform.scale_x = value;
        }
        VideoParam::TransformScaleY => {
            state.transform.scale_y = value;
        }
        VideoParam::TransformRotationDeg => {
            state.transform.rotation_deg = value;
        }
        VideoParam::TransformCropLeft => {
            state.transform.crop_left = value;
        }
        VideoParam::TransformCropTop => {
            state.transform.crop_top = value;
        }
        VideoParam::TransformCropRight => {
            state.transform.crop_right = value;
        }
        VideoParam::TransformCropBottom => {
            state.transform.crop_bottom = value;
        }
        VideoParam::ColorBrightness => {
            state.color.brightness = value;
        }
        VideoParam::ColorContrast => {
            state.color.contrast = value;
        }
        VideoParam::ColorHueDeg => {
            state.color.hue_deg = value;
        }
        VideoParam::ColorSaturation => {
            state.color.saturation = value;
        }
        VideoParam::ColorGamma => {
            state.color.gamma = value;
        }
        VideoParam::FxPixelate => {
            state.fx.pixelate = value;
        }
        VideoParam::FxBlur => {
            state.fx.blur = value;
        }
        VideoParam::FxGlow => {
            state.fx.glow = value;
        }
        VideoParam::FxEdge => {
            state.fx.edge = value;
        }
        VideoParam::FxKeyRed => {
            state.fx.key_red = value;
        }
        VideoParam::FxKeyGreen => {
            state.fx.key_green = value;
        }
        VideoParam::FxKeyBlue => {
            state.fx.key_blue = value;
        }
        VideoParam::FxKeyThreshold => {
            state.fx.key_threshold = value;
        }
    }
}

fn effect_targets_fixture_attribute(
    effect: &RuntimeEffect,
    fixture: &RuntimeFixture,
    attribute: &str,
) -> bool {
    match &effect.kind {
        RuntimeEffectKind::Lfo(request) => {
            request.attribute == attribute
                && request_targets_fixture(
                    request.fixture_ids.as_slice(),
                    request.target_group_ids.as_slice(),
                    fixture,
                )
        }
        RuntimeEffectKind::PositionWave(request) => {
            request.attribute == attribute
                && request_targets_fixture(
                    request.fixture_ids.as_slice(),
                    request.target_group_ids.as_slice(),
                    fixture,
                )
        }
    }
}

fn request_targets_fixture(
    fixture_ids: &[FixtureId],
    target_group_ids: &[String],
    fixture: &RuntimeFixture,
) -> bool {
    fixture_ids.contains(&fixture.id)
        || target_group_ids.iter().any(|group_id| {
            fixture
                .request
                .group_ids
                .iter()
                .any(|fixture_group| fixture_group == group_id)
        })
}

fn effect_summary(effect: &RuntimeEffect) -> EffectSummary {
    match &effect.kind {
        RuntimeEffectKind::Lfo(request) => EffectSummary {
            id: effect.id,
            label: request.label.clone(),
            effect_type: EffectKind::Lfo,
            fixture_ids: request.fixture_ids.clone(),
            target_group_ids: request.target_group_ids.clone(),
            attribute: request.attribute.clone(),
            video_targets: request.video_targets.clone(),
            shape: request.shape.clone(),
            period_ms: Some(request.period_ms),
            low: request.low,
            high: request.high,
            phase: request.phase,
            blend_mode: request.blend_mode.clone(),
            origin: None,
            direction: None,
            speed: None,
            wavelength: None,
            enabled: effect.enabled,
        },
        RuntimeEffectKind::PositionWave(request) => EffectSummary {
            id: effect.id,
            label: request.label.clone(),
            effect_type: EffectKind::PositionWave,
            fixture_ids: request.fixture_ids.clone(),
            target_group_ids: request.target_group_ids.clone(),
            attribute: request.attribute.clone(),
            video_targets: request.video_targets.clone(),
            shape: request.shape.clone(),
            period_ms: None,
            low: request.low,
            high: request.high,
            phase: request.phase,
            blend_mode: request.blend_mode.clone(),
            origin: Some(request.origin),
            direction: Some(request.direction),
            speed: Some(request.speed),
            wavelength: Some(request.wavelength),
            enabled: effect.enabled,
        },
    }
}

fn runtime_effect_from_summary(effect: &EffectSummary, now: Instant) -> Option<RuntimeEffect> {
    let kind = match effect.effect_type {
        EffectKind::Lfo => RuntimeEffectKind::Lfo(LfoEffectRequest {
            label: effect.label.clone(),
            fixture_ids: effect.fixture_ids.clone(),
            target_group_ids: effect.target_group_ids.clone(),
            attribute: effect.attribute.clone(),
            video_targets: effect.video_targets.clone(),
            shape: effect.shape.clone(),
            period_ms: effect.period_ms?,
            low: effect.low,
            high: effect.high,
            phase: effect.phase,
            blend_mode: effect.blend_mode.clone(),
        }),
        EffectKind::PositionWave => RuntimeEffectKind::PositionWave(PositionWaveEffectRequest {
            label: effect.label.clone(),
            fixture_ids: effect.fixture_ids.clone(),
            target_group_ids: effect.target_group_ids.clone(),
            attribute: effect.attribute.clone(),
            video_targets: effect.video_targets.clone(),
            shape: effect.shape.clone(),
            origin: effect.origin?,
            direction: effect.direction?,
            speed: effect.speed?,
            wavelength: effect.wavelength?,
            low: effect.low,
            high: effect.high,
            phase: effect.phase,
            blend_mode: effect.blend_mode.clone(),
        }),
    };
    Some(RuntimeEffect {
        id: effect.id,
        kind,
        enabled: effect.enabled,
        created_at: now,
    })
}

fn dmx_preview_to_frame(preview: &[u8]) -> [u8; 512] {
    let mut frame = [0u8; 512];
    let len = preview.len().min(frame.len());
    frame[..len].copy_from_slice(&preview[..len]);
    frame
}

fn dmx_previews_to_frames(previews: &[DmxUniversePreview]) -> HashMap<u16, [u8; 512]> {
    previews
        .iter()
        .map(|preview| (preview.universe, dmx_preview_to_frame(&preview.values)))
        .collect()
}

fn blend_effect_value(base: u16, effect: u16, blend_mode: &EffectBlendMode) -> u16 {
    match blend_mode {
        EffectBlendMode::Override => effect,
        EffectBlendMode::Add => base.saturating_add(effect),
        EffectBlendMode::Multiply => ((base as u32 * effect as u32) / 65_535) as u16,
    }
}

fn blend_effect_float(base: f32, effect: f32, blend_mode: &EffectBlendMode) -> f32 {
    match blend_mode {
        EffectBlendMode::Override => effect,
        EffectBlendMode::Add => base + effect,
        EffectBlendMode::Multiply => base * effect,
    }
}

fn evaluate_lfo_effect(request: &LfoEffectRequest, created_at: Instant, now: Instant) -> u16 {
    scale_effect_u16(
        request.low,
        request.high,
        evaluate_lfo_effect_normalized(request, created_at, now),
    )
}

fn evaluate_lfo_video_effect(
    request: &LfoEffectRequest,
    target: &VideoEffectTarget,
    created_at: Instant,
    now: Instant,
) -> f32 {
    scale_effect_float(
        target.low,
        target.high,
        evaluate_lfo_effect_normalized(request, created_at, now),
    )
}

fn evaluate_lfo_effect_normalized(
    request: &LfoEffectRequest,
    created_at: Instant,
    now: Instant,
) -> f32 {
    let period = request.period_ms.max(10) as f32 / 1000.0;
    let elapsed = now.saturating_duration_since(created_at).as_secs_f32();
    let phase = (elapsed / period + request.phase).rem_euclid(1.0);
    evaluate_lfo_shape(&request.shape, phase)
}

fn evaluate_position_wave_effect(
    request: &PositionWaveEffectRequest,
    fixture_position: Vec3,
    created_at: Instant,
    now: Instant,
) -> u16 {
    scale_effect_u16(
        request.low,
        request.high,
        evaluate_position_wave_effect_normalized(request, fixture_position, created_at, now),
    )
}

fn evaluate_position_wave_video_effect(
    request: &PositionWaveEffectRequest,
    target: &VideoEffectTarget,
    layer_position: Vec3,
    created_at: Instant,
    now: Instant,
) -> f32 {
    scale_effect_float(
        target.low,
        target.high,
        evaluate_position_wave_effect_normalized(request, layer_position, created_at, now),
    )
}

fn evaluate_position_wave_effect_normalized(
    request: &PositionWaveEffectRequest,
    position: Vec3,
    created_at: Instant,
    now: Instant,
) -> f32 {
    let wavelength = request.wavelength.abs().max(0.001);
    let elapsed = now.saturating_duration_since(created_at).as_secs_f32();
    let distance_phase =
        projected_distance(position, request.origin, request.direction) / wavelength;
    let time_phase = elapsed * request.speed / wavelength;
    let phase = (request.phase + time_phase - distance_phase).rem_euclid(1.0);
    evaluate_lfo_shape(&request.shape, phase)
}

fn scale_effect_u16(low: u16, high: u16, normalized: f32) -> u16 {
    let low_value = low.min(high) as f32;
    let high_value = low.max(high) as f32;
    (low_value + (high_value - low_value) * normalized)
        .round()
        .clamp(0.0, 65535.0) as u16
}

fn scale_effect_float(low: f32, high: f32, normalized: f32) -> f32 {
    if !low.is_finite() || !high.is_finite() {
        return 0.0;
    }
    let low_value = low.min(high);
    let high_value = low.max(high);
    low_value + (high_value - low_value) * normalized.clamp(0.0, 1.0)
}

fn apply_video_effect_param(
    state: &mut VideoLayerState,
    param: &VideoParam,
    effect_value: f32,
    blend_mode: &EffectBlendMode,
) {
    let mut base = state.clone();
    apply_video_param(
        &mut base,
        param,
        blend_effect_float(video_param_value(state, param), effect_value, blend_mode),
    );
    *state = base;
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

fn projected_distance(position: Vec3, origin: Vec3, direction: Vec3) -> f32 {
    let dx = position.x - origin.x;
    let dy = position.y - origin.y;
    let dz = position.z - origin.z;
    let direction_len =
        (direction.x * direction.x + direction.y * direction.y + direction.z * direction.z).sqrt();
    if direction_len <= f32::EPSILON {
        return (dx * dx + dy * dy + dz * dz).sqrt();
    }
    (dx * direction.x + dy * direction.y + dz * direction.z) / direction_len
}

fn evaluate_lfo_shape(shape: &LfoShape, phase: f32) -> f32 {
    let phase = phase.rem_euclid(1.0);
    match shape {
        LfoShape::Sine => ((phase * std::f32::consts::TAU).sin() + 1.0) * 0.5,
        LfoShape::Cosine => ((phase * std::f32::consts::TAU).cos() + 1.0) * 0.5,
        LfoShape::Triangle => {
            if phase < 0.5 {
                phase * 2.0
            } else {
                (1.0 - phase) * 2.0
            }
        }
        LfoShape::Saw => phase,
        LfoShape::Square => {
            if phase < 0.5 {
                1.0
            } else {
                0.0
            }
        }
        LfoShape::Random => stepped_noise(phase, 16),
        LfoShape::Perlin => smooth_periodic_noise(phase, 8),
    }
}

fn stepped_noise(phase: f32, steps: u32) -> f32 {
    let step = (phase * steps as f32).floor() as u32;
    hash_unit_float(step)
}

fn smooth_periodic_noise(phase: f32, steps: u32) -> f32 {
    let x = phase * steps as f32;
    let left = x.floor() as u32;
    let right = (left + 1) % steps;
    let progress = x - left as f32;
    let eased = progress * progress * (3.0 - 2.0 * progress);
    let a = hash_unit_float(left);
    let b = hash_unit_float(right);
    a + (b - a) * eased
}

fn hash_unit_float(seed: u32) -> f32 {
    let mut value = seed.wrapping_add(0x9E37_79B9);
    value ^= value >> 16;
    value = value.wrapping_mul(0x7FEB_352D);
    value ^= value >> 15;
    value = value.wrapping_mul(0x846C_A68B);
    value ^= value >> 16;
    value as f32 / u32::MAX as f32
}

struct BpmClock {
    bpm: f32,
    anchor: Instant,
    taps: [Option<Instant>; 4],
    tap_cursor: usize,
    tap_count: usize,
    midi_pulses: [Option<Instant>; 48],
    midi_cursor: usize,
    midi_count: usize,
    source: ClockSource,
}

impl BpmClock {
    fn new(bpm: f32, now: Instant) -> Self {
        Self {
            bpm: clamp_bpm(bpm),
            anchor: now,
            taps: [None; 4],
            tap_cursor: 0,
            tap_count: 0,
            midi_pulses: [None; 48],
            midi_cursor: 0,
            midi_count: 0,
            source: ClockSource::Manual,
        }
    }

    fn set_bpm(&mut self, bpm: f32, now: Instant) {
        self.bpm = clamp_bpm(bpm);
        self.anchor = now;
        self.tap_count = 0;
        self.tap_cursor = 0;
        self.taps = [None; 4];
        self.reset_midi_clock();
        self.source = ClockSource::Manual;
    }

    fn tap(&mut self, now: Instant) {
        let previous = self.latest_tap();
        if previous
            .map(|tap| now.saturating_duration_since(tap) > Duration::from_secs(2))
            .unwrap_or(false)
        {
            self.tap_count = 0;
            self.tap_cursor = 0;
            self.taps = [None; 4];
        }

        self.taps[self.tap_cursor] = Some(now);
        self.tap_cursor = (self.tap_cursor + 1) % self.taps.len();
        self.tap_count = (self.tap_count + 1).min(self.taps.len());

        let taps = self.ordered_taps();
        if taps.len() >= 2 {
            let total: f32 = taps
                .windows(2)
                .map(|pair| pair[1].saturating_duration_since(pair[0]).as_secs_f32())
                .sum();
            let average = total / (taps.len() - 1) as f32;
            if average > 0.0 {
                self.bpm = clamp_bpm(60.0 / average);
                self.anchor = now;
                self.source = ClockSource::Tap;
            }
        }
    }

    fn midi_clock_pulse(&mut self, now: Instant) {
        let previous = self.latest_midi_pulse();
        if previous
            .map(|pulse| now.saturating_duration_since(pulse) > Duration::from_millis(500))
            .unwrap_or(false)
        {
            self.reset_midi_clock();
        }

        self.midi_pulses[self.midi_cursor] = Some(now);
        self.midi_cursor = (self.midi_cursor + 1) % self.midi_pulses.len();
        self.midi_count = (self.midi_count + 1).min(self.midi_pulses.len());

        let pulses = self.ordered_midi_pulses();
        if pulses.len() >= 2 {
            let total: f32 = pulses
                .windows(2)
                .map(|pair| pair[1].saturating_duration_since(pair[0]).as_secs_f32())
                .sum();
            let average_pulse = total / (pulses.len() - 1) as f32;
            if average_pulse > 0.0 {
                self.bpm = clamp_bpm(60.0 / (average_pulse * 24.0));
                self.anchor = now;
                self.source = ClockSource::MidiClock;
            }
        }
    }

    fn snapshot(&self, now: Instant) -> ClockSnapshot {
        let elapsed_beats =
            now.saturating_duration_since(self.anchor).as_secs_f32() * self.bpm / 60.0;
        ClockSnapshot {
            bpm: self.bpm,
            beat_phase: elapsed_beats.fract(),
            beat_counter: elapsed_beats.floor() as u64,
            tap_count: self.tap_count,
            source: self.source.clone(),
        }
    }

    fn latest_tap(&self) -> Option<Instant> {
        if self.tap_count == 0 {
            return None;
        }
        let index = (self.tap_cursor + self.taps.len() - 1) % self.taps.len();
        self.taps[index]
    }

    fn ordered_taps(&self) -> Vec<Instant> {
        let mut taps = self.taps.iter().filter_map(|tap| *tap).collect::<Vec<_>>();
        taps.sort();
        taps
    }

    fn latest_midi_pulse(&self) -> Option<Instant> {
        if self.midi_count == 0 {
            return None;
        }
        let index = (self.midi_cursor + self.midi_pulses.len() - 1) % self.midi_pulses.len();
        self.midi_pulses[index]
    }

    fn ordered_midi_pulses(&self) -> Vec<Instant> {
        let mut pulses = self
            .midi_pulses
            .iter()
            .filter_map(|pulse| *pulse)
            .collect::<Vec<_>>();
        pulses.sort();
        pulses
    }

    fn reset_midi_clock(&mut self) {
        self.midi_pulses = [None; 48];
        self.midi_cursor = 0;
        self.midi_count = 0;
    }
}

fn clamp_bpm(bpm: f32) -> f32 {
    if bpm.is_finite() {
        bpm.clamp(20.0, 300.0)
    } else {
        120.0
    }
}

fn sanitize_video_output(mut output: VideoOutputSummary) -> VideoOutputSummary {
    if output.label.trim().is_empty() {
        output.label = format!("Video Output {}", output.id);
    } else {
        output.label = output.label.trim().to_string();
    }
    if output.composition_id == 0 {
        output.composition_id = 1;
    }
    output.width = output.width.max(1);
    output.height = output.height.max(1);
    output.opacity = if output.opacity.is_finite() {
        output.opacity.clamp(0.0, 1.0)
    } else {
        1.0
    };
    output.endpoint_name = output.endpoint_name.and_then(|name| {
        let trimmed = name.trim().to_string();
        (!trimmed.is_empty()).then_some(trimmed)
    });
    output.mapping = sanitize_video_output_mapping(output.mapping);
    output
}

fn sanitize_video_output_mapping(mut mapping: VideoOutputMapping) -> VideoOutputMapping {
    mapping.offset_x = finite_or(mapping.offset_x, 0.0).clamp(-4.0, 4.0);
    mapping.offset_y = finite_or(mapping.offset_y, 0.0).clamp(-4.0, 4.0);
    mapping.scale_x = finite_or(mapping.scale_x, 1.0).clamp(0.01, 8.0);
    mapping.scale_y = finite_or(mapping.scale_y, 1.0).clamp(0.01, 8.0);
    mapping.rotation_deg = finite_or(mapping.rotation_deg, 0.0).clamp(-180.0, 180.0);
    mapping.aspect_ratio = finite_or(mapping.aspect_ratio, 1.0).clamp(0.1, 10.0);
    mapping.lens_distortion = finite_or(mapping.lens_distortion, 0.0).clamp(-1.0, 1.0);
    mapping.keystone_x = finite_or(mapping.keystone_x, 0.0).clamp(-1.0, 1.0);
    mapping.keystone_y = finite_or(mapping.keystone_y, 0.0).clamp(-1.0, 1.0);
    mapping.corner_top_left_x = finite_or(mapping.corner_top_left_x, 0.0).clamp(-1.0, 1.0);
    mapping.corner_top_left_y = finite_or(mapping.corner_top_left_y, 0.0).clamp(-1.0, 1.0);
    mapping.corner_top_right_x = finite_or(mapping.corner_top_right_x, 0.0).clamp(-1.0, 1.0);
    mapping.corner_top_right_y = finite_or(mapping.corner_top_right_y, 0.0).clamp(-1.0, 1.0);
    mapping.corner_bottom_right_x = finite_or(mapping.corner_bottom_right_x, 0.0).clamp(-1.0, 1.0);
    mapping.corner_bottom_right_y = finite_or(mapping.corner_bottom_right_y, 0.0).clamp(-1.0, 1.0);
    mapping.corner_bottom_left_x = finite_or(mapping.corner_bottom_left_x, 0.0).clamp(-1.0, 1.0);
    mapping.corner_bottom_left_y = finite_or(mapping.corner_bottom_left_y, 0.0).clamp(-1.0, 1.0);
    mapping
}

fn sanitize_video_output_mapping_preset(
    mut preset: VideoOutputMappingPresetSummary,
) -> Option<VideoOutputMappingPresetSummary> {
    let label = preset.label.trim();
    if label.is_empty() {
        return None;
    }
    preset.label = label.to_string();
    preset.mapping = sanitize_video_output_mapping(preset.mapping);
    Some(preset)
}

fn finite_or(value: f32, fallback: f32) -> f32 {
    if value.is_finite() {
        value
    } else {
        fallback
    }
}

fn sanitize_video_composition(
    mut composition: CompositionSummary,
    layers: &[RuntimeVideoLayer],
) -> CompositionSummary {
    if composition.id == 0 {
        composition.id = 1;
    }
    if composition.label.trim().is_empty() {
        composition.label = format!("Composition {}", composition.id);
    } else {
        composition.label = composition.label.trim().to_string();
    }
    composition.layer_ids = valid_unique_layer_ids(composition.layer_ids, layers);
    composition.output_ids.clear();
    composition
}

fn valid_unique_layer_ids(
    layer_ids: Vec<VideoLayerId>,
    layers: &[RuntimeVideoLayer],
) -> Vec<VideoLayerId> {
    let mut valid_ids = Vec::new();
    for layer_id in layer_ids {
        if valid_ids.contains(&layer_id) {
            continue;
        }
        if layers.iter().any(|layer| layer.id == layer_id) {
            valid_ids.push(layer_id);
        }
    }
    valid_ids
}

fn sanitize_video_layer_label(label: String, layer_id: VideoLayerId) -> String {
    let trimmed = label.trim();
    if trimmed.is_empty() {
        format!("Layer {layer_id}")
    } else {
        trimmed.to_string()
    }
}

fn reorder_video_layers(layers: &mut Vec<RuntimeVideoLayer>, layer_ids: Vec<VideoLayerId>) {
    let mut reordered = Vec::with_capacity(layers.len());
    for layer_id in layer_ids {
        if let Some(index) = layers.iter().position(|layer| layer.id == layer_id) {
            reordered.push(layers.remove(index));
        }
    }
    reordered.append(layers);
    *layers = reordered;
}

fn output_ids_for_composition(
    outputs: &[VideoOutputSummary],
    composition_id: CompositionId,
) -> Vec<VideoOutputId> {
    outputs
        .iter()
        .filter(|output| output.composition_id == composition_id)
        .map(|output| output.id)
        .collect()
}

fn send_output_frame(
    sender: &mut Option<DmxSender>,
    config: &DmxOutputConfig,
    universe: u16,
    frame: &[u8; 512],
) -> Result<usize, String> {
    if sender.is_none() {
        *sender = Some(create_dmx_sender(config)?);
    }
    sender
        .as_mut()
        .ok_or_else(|| "DMX sender is not initialized".to_string())?
        .send_dmx_frame(universe, frame)
}

impl DmxSender {
    fn send_dmx_frame(&mut self, universe: u16, frame: &[u8; 512]) -> Result<usize, String> {
        match self {
            DmxSender::ArtNet(sender) => sender
                .send_dmx_frame(universe, frame)
                .map_err(|error| error.to_string()),
            DmxSender::Sacn(sender) => sender
                .send_dmx_frame(universe, frame)
                .map_err(|error| error.to_string()),
            DmxSender::EnttecUsbPro(sender) => sender
                .send_dmx_frame(frame)
                .map_err(|error| error.to_string()),
            DmxSender::EnttecOpenDmx(sender) => sender
                .send_dmx_frame(frame)
                .map_err(|error| error.to_string()),
        }
    }
}

fn store_next_id(counter: &AtomicU64, next_id: u64) {
    let mut current = counter.load(Ordering::Relaxed);
    while current < next_id {
        match counter.compare_exchange(current, next_id, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(observed) => current = observed,
        }
    }
}

fn create_dmx_sender(output: &DmxOutputConfig) -> Result<DmxSender, String> {
    match output.protocol {
        DmxOutputProtocol::ArtNet => ArtNetSender::new(&output.target_ip, output.port)
            .map(DmxSender::ArtNet)
            .map_err(|error| error.to_string()),
        DmxOutputProtocol::Sacn => SacnSender::new(&output.target_ip, output.port)
            .map(DmxSender::Sacn)
            .map_err(|error| error.to_string()),
        DmxOutputProtocol::EnttecUsbPro => {
            EnttecUsbProSender::new(&output.serial_port, output.serial_baud_rate)
                .map(DmxSender::EnttecUsbPro)
                .map_err(|error| error.to_string())
        }
        DmxOutputProtocol::EnttecOpenDmx => EnttecOpenDmxSender::new(&output.serial_port)
            .map(DmxSender::EnttecOpenDmx)
            .map_err(|error| error.to_string()),
    }
}

fn select_mode_index(profile: &FixtureProfileSummary, mode_name: Option<&str>) -> usize {
    mode_name
        .and_then(|name| profile.dmx_modes.iter().position(|mode| mode.name == name))
        .unwrap_or(0)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MovementAxis {
    Pan,
    Tilt,
}

fn normalized_fixture_limits(limits: FixtureLimits) -> FixtureLimits {
    FixtureLimits {
        dimmer_min: limits.dimmer_min.min(limits.dimmer_max),
        dimmer_max: limits.dimmer_min.max(limits.dimmer_max),
        pan_min: limits.pan_min.min(limits.pan_max),
        pan_max: limits.pan_min.max(limits.pan_max),
        tilt_min: limits.tilt_min.min(limits.tilt_max),
        tilt_max: limits.tilt_min.max(limits.tilt_max),
        invert_pan: limits.invert_pan,
        invert_tilt: limits.invert_tilt,
        swap_pan_tilt: limits.swap_pan_tilt,
    }
}

fn apply_fixture_limits<F>(
    limits: &FixtureLimits,
    attribute: &str,
    raw_value: u16,
    read_axis_source: F,
) -> u16
where
    F: FnOnce(MovementAxis) -> u16,
{
    if is_dimmer_attribute(attribute) {
        return clamp_to_range(raw_value, limits.dimmer_min, limits.dimmer_max, false);
    }

    match movement_axis(attribute) {
        Some(MovementAxis::Pan) => {
            let value = if limits.swap_pan_tilt {
                read_axis_source(MovementAxis::Tilt)
            } else {
                raw_value
            };
            clamp_to_range(value, limits.pan_min, limits.pan_max, limits.invert_pan)
        }
        Some(MovementAxis::Tilt) => {
            let value = if limits.swap_pan_tilt {
                read_axis_source(MovementAxis::Pan)
            } else {
                raw_value
            };
            clamp_to_range(value, limits.tilt_min, limits.tilt_max, limits.invert_tilt)
        }
        None => raw_value,
    }
}

fn control_for_attribute<'a>(
    controls: &'a [AttributeControl],
    axis: MovementAxis,
) -> Option<&'a AttributeControl> {
    controls
        .iter()
        .find(|control| movement_axis(&control.attribute) == Some(axis))
}

fn clamp_to_range(value: u16, min: u16, max: u16, invert: bool) -> u16 {
    let range_min = min.min(max);
    let range_max = min.max(max);
    let clamped = value.clamp(range_min, range_max);
    if invert {
        range_min.saturating_add(range_max.saturating_sub(clamped))
    } else {
        clamped
    }
}

fn is_dimmer_attribute(attribute: &str) -> bool {
    let normalized = normalized_attribute_name(attribute);
    normalized.contains("dimmer") || normalized.contains("intensity")
}

fn movement_axis(attribute: &str) -> Option<MovementAxis> {
    let normalized = normalized_attribute_name(attribute);
    if normalized.contains("tilt") {
        Some(MovementAxis::Tilt)
    } else if normalized.contains("pan") {
        Some(MovementAxis::Pan)
    } else {
        None
    }
}

fn normalized_attribute_name(attribute: &str) -> String {
    attribute
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(|character| character.to_lowercase())
        .collect()
}

fn seed_default_values(
    fixture_id: FixtureId,
    profile: &FixtureProfileSummary,
    mode_index: usize,
    values: &mut HashMap<(FixtureId, String), u16>,
) {
    if let Some(mode) = profile.dmx_modes.get(mode_index) {
        for control in &mode.controls {
            values.insert(
                (fixture_id, control.attribute.clone()),
                control.default_value,
            );
        }
    }
}

fn write_control_value(
    frame: &mut [u8; 512],
    fixture_start_address: u16,
    control: &AttributeControl,
    value: u16,
) {
    if fixture_start_address == 0 {
        return;
    }

    match control.resolution {
        AttributeResolution::EightBit => {
            if let Some(offset) = control.offsets.first() {
                write_byte(frame, fixture_start_address, *offset, (value >> 8) as u8);
            }
        }
        AttributeResolution::SixteenBit => {
            if let Some(offset) = control.offsets.first() {
                write_byte(frame, fixture_start_address, *offset, (value >> 8) as u8);
            }
            if let Some(offset) = control.offsets.get(1) {
                write_byte(frame, fixture_start_address, *offset, (value & 0xff) as u8);
            }
        }
    }
}

fn write_byte(frame: &mut [u8; 512], fixture_start_address: u16, offset: u16, byte: u8) {
    if offset == 0 {
        return;
    }
    let absolute = fixture_start_address as usize + offset as usize - 2;
    if absolute < frame.len() {
        frame[absolute] = byte;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use io::{artnet::parse_art_dmx_packet, sacn::parse_sacn_dmx_packet};
    use protocol::{AudioWaveformPoint, DmxModeSummary, Vec3, VideoMediaMetadata};
    use std::net::UdpSocket;

    fn sample_profile() -> FixtureProfileSummary {
        FixtureProfileSummary {
            source_path: "memory://fixture.gdtf".to_string(),
            manufacturer: "KDMX".to_string(),
            name: "Mini Spot".to_string(),
            short_name: None,
            fixture_type_id: None,
            dmx_modes: vec![DmxModeSummary {
                name: "Standard".to_string(),
                controls: vec![
                    AttributeControl {
                        attribute: "Dimmer".to_string(),
                        channel_name: "Dimmer".to_string(),
                        geometry: None,
                        offsets: vec![1],
                        resolution: AttributeResolution::EightBit,
                        default_value: 0,
                        functions: Vec::new(),
                    },
                    AttributeControl {
                        attribute: "Pan".to_string(),
                        channel_name: "Pan".to_string(),
                        geometry: None,
                        offsets: vec![2, 3],
                        resolution: AttributeResolution::SixteenBit,
                        default_value: 0,
                        functions: Vec::new(),
                    },
                ],
            }],
            geometries: Vec::new(),
            warnings: Vec::new(),
        }
    }

    #[test]
    fn writes_8bit_attribute_as_msb() {
        let mut frame = [0u8; 512];
        let control = AttributeControl {
            attribute: "Dimmer".to_string(),
            channel_name: "Dimmer".to_string(),
            geometry: None,
            offsets: vec![1],
            resolution: AttributeResolution::EightBit,
            default_value: 0,
            functions: Vec::new(),
        };

        write_control_value(&mut frame, 10, &control, 0xffff);

        assert_eq!(frame[9], 255);
    }

    #[test]
    fn writes_16bit_attribute_as_coarse_and_fine() {
        let mut frame = [0u8; 512];
        let control = AttributeControl {
            attribute: "Pan".to_string(),
            channel_name: "Pan".to_string(),
            geometry: None,
            offsets: vec![2, 3],
            resolution: AttributeResolution::SixteenBit,
            default_value: 0,
            functions: Vec::new(),
        };

        write_control_value(&mut frame, 1, &control, 0x1234);

        assert_eq!(frame[1], 0x12);
        assert_eq!(frame[2], 0x34);
    }

    #[test]
    fn fixture_limits_clamp_swap_and_invert_rendered_values() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        let mut profile = sample_profile();
        profile.dmx_modes[0].controls.push(AttributeControl {
            attribute: "Tilt".to_string(),
            channel_name: "Tilt".to_string(),
            geometry: None,
            offsets: vec![4, 5],
            resolution: AttributeResolution::SixteenBit,
            default_value: 0,
            functions: Vec::new(),
        });
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile,
            })
            .unwrap();
        engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id,
                values: vec![
                    AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 0x9000,
                    },
                    AttributeValueSummary {
                        attribute: "Pan".to_string(),
                        value: 0x5000,
                    },
                    AttributeValueSummary {
                        attribute: "Tilt".to_string(),
                        value: 0x1500,
                    },
                ],
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureLimits {
                fixture_id,
                limits: FixtureLimits {
                    dimmer_min: 0x2000,
                    dimmer_max: 0x8000,
                    pan_min: 0x1000,
                    pan_max: 0x3000,
                    tilt_min: 0x2000,
                    tilt_max: 0x4000,
                    invert_pan: true,
                    invert_tilt: false,
                    swap_pan_tilt: true,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 0x80
                && snapshot.dmx_preview[1] == 0x2b
                && snapshot.dmx_preview[2] == 0x00
                && snapshot.dmx_preview[3] == 0x40
                && snapshot.dmx_preview[4] == 0x00
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.dmx_preview[0], 0x80);
        assert_eq!(snapshot.dmx_preview[1], 0x2b);
        assert_eq!(snapshot.dmx_preview[2], 0x00);
        assert_eq!(snapshot.dmx_preview[3], 0x40);
        assert_eq!(snapshot.dmx_preview[4], 0x00);
        assert_eq!(
            snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)
                .map(|fixture| fixture.limits.swap_pan_tilt),
            Some(true)
        );
    }

    #[test]
    fn engine_sends_rendered_fixture_state_to_artnet_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let output = DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        };
        let engine = EngineHandle::start(output);
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 10,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Dimmer".to_string(),
                value: 0xffff,
            })
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_expected_dimmer = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 0 && packet.data[9] == 255 {
                saw_expected_dimmer = true;
                break;
            }
        }

        assert!(saw_expected_dimmer);
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[9] == 255 && snapshot.telemetry.last_dmx_send_success_count >= 1
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_preview[9], 255);
        assert_eq!(snapshot.telemetry.last_dmx_output_count, 1);
        assert_eq!(snapshot.telemetry.last_dmx_send_success_count, 1);
        assert_eq!(snapshot.telemetry.last_dmx_send_failure_count, 0);
        assert!(snapshot.telemetry.total_dmx_send_success_count >= 1);
    }

    #[test]
    fn engine_sends_rendered_fixture_state_to_multiple_artnet_routes() {
        let receiver_a = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_a
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let receiver_b = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver_b
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port_a = receiver_a.local_addr().unwrap().port();
        let port_b = receiver_b.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine
            .send(EngineCommand::SetDmxOutputs(vec![
                DmxOutputConfig {
                    enabled: true,
                    protocol: DmxOutputProtocol::ArtNet,
                    target_ip: "127.0.0.1".to_string(),
                    port: port_a,
                    universe: 0,
                    serial_port: String::new(),
                    serial_baud_rate: 57_600,
                },
                DmxOutputConfig {
                    enabled: true,
                    protocol: DmxOutputProtocol::ArtNet,
                    target_ip: "127.0.0.1".to_string(),
                    port: port_b,
                    universe: 1,
                    serial_port: String::new(),
                    serial_baud_rate: 57_600,
                },
            ]))
            .unwrap();
        for universe in [0_u16, 1_u16] {
            let fixture_id = engine.allocate_fixture_id();
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture U{universe}"),
                        universe,
                        address: 1,
                        group_ids: Vec::new(),
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
            engine
                .send(EngineCommand::SetAttribute {
                    fixture_id,
                    attribute: "Dimmer".to_string(),
                    value: 0xffff,
                })
                .unwrap();
        }

        let mut buffer = [0u8; 600];
        let mut saw_a = false;
        let mut saw_b = false;
        for _ in 0..20 {
            let (received, _) = receiver_a.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 0 && packet.data[0] == 255 {
                saw_a = true;
                break;
            }
        }
        for _ in 0..20 {
            let (received, _) = receiver_b.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 1 && packet.data[0] == 255 {
                saw_b = true;
                break;
            }
        }

        assert!(saw_a);
        assert!(saw_b);
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.last_dmx_output_count == 2
                && snapshot.telemetry.last_dmx_send_success_count == 2
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_outputs.len(), 2);
        assert_eq!(snapshot.dmx_outputs[1].universe, 1);
        assert_eq!(
            snapshot
                .dmx_previews
                .iter()
                .map(|preview| preview.universe)
                .collect::<Vec<_>>(),
            vec![0, 1]
        );
        assert_eq!(
            snapshot
                .dmx_previews
                .iter()
                .find(|preview| preview.universe == 1)
                .and_then(|preview| preview.values.first())
                .copied(),
            Some(255)
        );
        assert_eq!(snapshot.telemetry.last_dmx_output_count, 2);
        assert_eq!(snapshot.telemetry.last_dmx_send_success_count, 2);
        assert_eq!(snapshot.telemetry.last_dmx_send_failure_count, 0);
        assert!(snapshot.telemetry.total_dmx_send_success_count >= 2);
    }

    #[test]
    fn project_snapshot_load_restores_core_show_state_and_advances_ids() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let dimmer = AttributeControl {
            attribute: "Dimmer".to_string(),
            channel_name: "Dimmer".to_string(),
            geometry: None,
            offsets: vec![1],
            resolution: AttributeResolution::EightBit,
            default_value: 0,
            functions: Vec::new(),
        };
        let snapshot = EngineSnapshot {
            fixtures: vec![PatchedFixtureSummary {
                id: 40,
                label: "Loaded Fixture".to_string(),
                profile_name: "Snapshot Fixture".to_string(),
                manufacturer: "KDMX".to_string(),
                mode_name: "Default".to_string(),
                universe: 1,
                address: 12,
                group_ids: vec!["front".to_string()],
                position: Vec3 {
                    x: 1.0,
                    y: 2.0,
                    z: 3.0,
                },
                rotation: Default::default(),
                controls: vec![dimmer.clone()],
                attribute_values: vec![AttributeValueSummary {
                    attribute: "Dimmer".to_string(),
                    value: 40_000,
                }],
                limits: FixtureLimits::default(),
                highlighted: true,
                soloed: false,
                parked: true,
            }],
            cues: vec![CueSummary {
                id: 41,
                label: "Loaded Cue".to_string(),
                fade_ms: 250,
                targets: vec![CueFixtureTarget {
                    fixture_id: 40,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            }],
            active_cue_id: Some(41),
            timeline: TimelineSnapshot {
                events: vec![TimelineCueEventSummary {
                    id: 42,
                    cue_id: 41,
                    time_ms: 500,
                    track: TimelineTrackKind::Lighting,
                }],
                automations: vec![TimelineAutomationSummary {
                    id: 43,
                    fixture_id: 40,
                    attribute: "Dimmer".to_string(),
                    track: TimelineTrackKind::Lighting,
                    keyframes: vec![
                        AutomationKeyframeSummary {
                            time_ms: 0,
                            value: 0,
                            interpolation: AutomationInterpolation::Linear,
                        },
                        AutomationKeyframeSummary {
                            time_ms: 500,
                            value: 65_535,
                            interpolation: AutomationInterpolation::Linear,
                        },
                    ],
                    enabled: false,
                }],
                video_automations: Vec::new(),
                audio: Some(AudioAnalysisSummary {
                    path: "C:/media/show.wav".to_string(),
                    sample_rate: 48_000,
                    channels: 2,
                    duration_ms: 4_000,
                    estimated_bpm: Some(120.0),
                    waveform: vec![AudioWaveformPoint {
                        time_ms: 0,
                        peak: 0.5,
                        rms: 0.25,
                    }],
                    beats: vec![0, 500, 1_000],
                }),
                playing: false,
                position_ms: 250,
                duration_ms: 500,
            },
            video: VideoSnapshot {
                layers: vec![VideoLayerSummary {
                    id: 44,
                    label: "Loaded Layer".to_string(),
                    source: VideoSourceSummary {
                        kind: protocol::VideoSourceKind::StillImage,
                        path: Some("C:/media/still.png".to_string()),
                        name: None,
                        codec: None,
                        metadata: None,
                    },
                    blend_mode: VideoBlendMode::Add,
                    state: VideoLayerState {
                        opacity: 0.5,
                        ..VideoLayerState::default()
                    },
                }],
                compositions: vec![CompositionSummary {
                    id: 45,
                    label: "Loaded Comp".to_string(),
                    layer_ids: vec![44],
                    output_ids: Vec::new(),
                }],
                outputs: vec![VideoOutputSummary {
                    id: 46,
                    label: "Loaded Output".to_string(),
                    kind: protocol::VideoOutputKind::Display,
                    enabled: true,
                    composition_id: 45,
                    fullscreen: false,
                    monitor_id: Some(0),
                    width: 640,
                    height: 480,
                    endpoint_name: None,
                    opacity: 0.75,
                    blackout: false,
                    mapping: Default::default(),
                }],
                mapping_presets: vec![VideoOutputMappingPresetSummary {
                    label: "Loaded Projector".to_string(),
                    mapping: VideoOutputMapping {
                        aspect_ratio: 16.0 / 9.0,
                        ..Default::default()
                    },
                }],
                master_opacity: 0.8,
                blackout: true,
            },
            effects: vec![EffectSummary {
                id: 47,
                label: "Loaded Effect".to_string(),
                effect_type: EffectKind::Lfo,
                fixture_ids: vec![40],
                target_group_ids: Vec::new(),
                attribute: "Dimmer".to_string(),
                video_targets: Vec::new(),
                shape: LfoShape::Sine,
                period_ms: Some(1_000),
                low: 0,
                high: 65_535,
                phase: 0.0,
                blend_mode: EffectBlendMode::Override,
                origin: None,
                direction: None,
                speed: None,
                wavelength: None,
                enabled: false,
            }],
            output: DmxOutputConfig {
                enabled: false,
                universe: 1,
                ..DmxOutputConfig::default()
            },
            dmx_outputs: vec![
                DmxOutputConfig {
                    enabled: false,
                    universe: 1,
                    ..DmxOutputConfig::default()
                },
                DmxOutputConfig {
                    enabled: false,
                    universe: 2,
                    ..DmxOutputConfig::default()
                },
            ],
            lighting_master: 0.5,
            submasters: vec![SubmasterSummary {
                group_id: "front".to_string(),
                label: "front".to_string(),
                level: 0.25,
            }],
            blackout: true,
            ..EngineSnapshot::default()
        };

        engine.load_project_snapshot(snapshot).unwrap();
        let mut loaded = engine.snapshot();
        for _ in 0..20 {
            if loaded.fixtures.len() == 1
                && loaded.cues.len() == 1
                && loaded.video.layers.len() == 1
                && loaded.effects.len() == 1
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            loaded = engine.snapshot();
        }

        assert_eq!(loaded.fixtures[0].id, 40);
        assert_eq!(loaded.fixtures[0].attribute_values[0].value, 40_000);
        assert!(loaded.fixtures[0].highlighted);
        assert!(loaded.fixtures[0].parked);
        assert_eq!(loaded.cues[0].id, 41);
        assert_eq!(loaded.timeline.events[0].id, 42);
        assert_eq!(loaded.timeline.automations[0].id, 43);
        assert_eq!(
            loaded
                .timeline
                .audio
                .as_ref()
                .map(|audio| audio.path.as_str()),
            Some("C:/media/show.wav")
        );
        assert_eq!(loaded.timeline.duration_ms, 4_000);
        assert_eq!(loaded.video.layers[0].id, 44);
        assert_eq!(loaded.video.compositions[1].id, 45);
        assert_eq!(loaded.video.outputs[0].id, 46);
        assert_eq!(loaded.video.mapping_presets[0].label, "Loaded Projector");
        assert_eq!(loaded.effects[0].id, 47);
        assert!(!loaded.effects[0].enabled);
        assert_eq!(loaded.dmx_outputs.len(), 2);
        assert_eq!(loaded.dmx_outputs[1].universe, 2);
        assert_eq!(loaded.lighting_master, 0.5);
        assert_eq!(loaded.submasters[0].level, 0.25);
        assert!(loaded.blackout);

        assert!(engine.allocate_fixture_id() > 40);
        assert!(engine.allocate_cue_id() > 41);
        assert!(engine.allocate_timeline_event_id() > 42);
        assert!(engine.allocate_automation_id() > 43);
        assert!(engine.allocate_video_layer_id() > 44);
        assert!(engine.allocate_composition_id() > 45);
        assert!(engine.allocate_video_output_id() > 46);
        assert!(engine.allocate_effect_id() > 47);
    }

    #[test]
    fn timeline_audio_command_updates_snapshot_duration() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let audio = AudioAnalysisSummary {
            path: "C:/media/timeline.wav".to_string(),
            sample_rate: 44_100,
            channels: 2,
            duration_ms: 123_000,
            estimated_bpm: Some(128.0),
            waveform: vec![AudioWaveformPoint {
                time_ms: 0,
                peak: 0.8,
                rms: 0.4,
            }],
            beats: vec![0, 469, 938],
        };

        engine
            .send(EngineCommand::SetTimelineAudio(Some(audio)))
            .unwrap();
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.audio.is_some() {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.timeline.duration_ms, 123_000);
        assert_eq!(snapshot.timeline.audio.unwrap().estimated_bpm, Some(128.0));

        engine.send(EngineCommand::SetTimelineAudio(None)).unwrap();
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.audio.is_none() && snapshot.timeline.duration_ms == 0 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert!(snapshot.timeline.audio.is_none());
        assert_eq!(snapshot.timeline.duration_ms, 0);
    }

    #[test]
    fn video_source_metadata_extends_timeline_duration() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();

        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Clip".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("C:/media/clip.mp4".to_string()),
                    name: None,
                    codec: Some("h264".to_string()),
                    metadata: Some(VideoMediaMetadata {
                        duration_ms: Some(98_000),
                        width: Some(1920),
                        height: Some(1080),
                        frame_rate: Some(29.97),
                    }),
                },
            })
            .unwrap();
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.duration_ms == 98_000 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.timeline.duration_ms, 98_000);
        assert_eq!(
            snapshot.video.layers[0]
                .source
                .metadata
                .as_ref()
                .and_then(|metadata| metadata.width),
            Some(1920)
        );
    }

    #[test]
    fn engine_sends_rendered_fixture_state_to_sacn_loopback() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::Sacn,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 12,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 12,
                    address: 10,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Dimmer".to_string(),
                value: 0xffff,
            })
            .unwrap();

        let mut buffer = [0u8; 700];
        let mut saw_expected_dimmer = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_sacn_dmx_packet(&buffer[..received]).unwrap();
            if packet.universe == 12 && packet.data[9] == 255 {
                saw_expected_dimmer = true;
                break;
            }
        }

        assert!(saw_expected_dimmer);
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[9] == 255 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_preview[9], 255);
    }

    #[test]
    fn apply_attribute_values_updates_fixture_snapshot_and_dmx_output() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id,
                values: vec![
                    AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 0x8000,
                    },
                    AttributeValueSummary {
                        attribute: "Pan".to_string(),
                        value: 0x3456,
                    },
                ],
            })
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_expected_values = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 0x80 && packet.data[1] == 0x34 && packet.data[2] == 0x56 {
                saw_expected_values = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .fixtures
                .iter()
                .any(|fixture| fixture.id == fixture_id)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        let fixture = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == fixture_id)
            .unwrap();
        assert!(saw_expected_values);
        assert_eq!(snapshot.dmx_preview[0], 0x80);
        assert_eq!(snapshot.dmx_preview[1], 0x34);
        assert_eq!(snapshot.dmx_preview[2], 0x56);
        assert_eq!(
            fixture
                .attribute_values
                .iter()
                .find(|value| value.attribute == "Dimmer")
                .map(|value| value.value),
            Some(0x8000)
        );
        assert_eq!(
            fixture
                .attribute_values
                .iter()
                .find(|value| value.attribute == "Pan")
                .map(|value| value.value),
            Some(0x3456)
        );
    }

    #[test]
    fn set_fixture_transform_updates_snapshot_position_and_rotation() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureTransform {
                fixture_id,
                position: Vec3 {
                    x: 12.0,
                    y: 3.5,
                    z: -4.0,
                },
                rotation: Rotation3 {
                    pitch: 10.0,
                    yaw: 45.0,
                    roll: -5.0,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.fixtures.iter().any(|fixture| {
                fixture.id == fixture_id
                    && fixture.position.x == 12.0
                    && fixture.rotation.yaw == 45.0
            }) {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let fixture = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == fixture_id)
            .unwrap();
        assert_eq!(
            fixture.position,
            Vec3 {
                x: 12.0,
                y: 3.5,
                z: -4.0
            }
        );
        assert_eq!(
            fixture.rotation,
            Rotation3 {
                pitch: 10.0,
                yaw: 45.0,
                roll: -5.0
            }
        );
    }

    #[test]
    fn remove_fixture_clears_patch_state_and_fixture_owned_targets() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let start_frame = engine.snapshot().telemetry.frame_counter;
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Dimmer".to_string(),
                value: 65_535,
            })
            .unwrap();
        engine
            .send(EngineCommand::CreateCue {
                cue_id: 1,
                label: "Cue".to_string(),
                fade_ms: 0,
                targets: vec![CueFixtureTarget {
                    fixture_id,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineAutomation {
                automation_id: 1,
                fixture_id,
                attribute: "Dimmer".to_string(),
                keyframes: vec![AutomationKeyframeSummary {
                    time_ms: 0,
                    value: 65_535,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: 1,
                request: LfoEffectRequest {
                    label: "Fixture LFO".to_string(),
                    fixture_ids: vec![fixture_id],
                    target_group_ids: Vec::new(),
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: LfoShape::Sine,
                    period_ms: 1_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.0,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureHighlight {
                fixture_id,
                enabled: true,
            })
            .unwrap();
        engine
            .send(EngineCommand::RemoveFixture(fixture_id))
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.frame_counter > start_frame && snapshot.telemetry.queue_depth == 0
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert!(snapshot.fixtures.is_empty());
        assert_eq!(snapshot.dmx_preview[0], 0);
        assert_eq!(snapshot.cues.len(), 1);
        assert!(snapshot.cues[0].targets.is_empty());
        assert!(snapshot.timeline.automations.is_empty());
        assert!(snapshot.effects.is_empty());
    }

    #[test]
    fn update_cue_replaces_existing_snapshot_without_reordering() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Old".to_string(),
                fade_ms: 100,
                targets: vec![CueFixtureTarget {
                    fixture_id: 1,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 1_000,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::UpdateCue {
                cue_id,
                label: "Updated".to_string(),
                fade_ms: 250,
                targets: vec![CueFixtureTarget {
                    fixture_id: 1,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 50_000,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .cues
                .first()
                .map(|cue| cue.label.as_str() == "Updated")
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.cues.len(), 1);
        assert_eq!(snapshot.cues[0].id, cue_id);
        assert_eq!(snapshot.cues[0].label, "Updated");
        assert_eq!(snapshot.cues[0].fade_ms, 250);
        assert_eq!(snapshot.cues[0].targets[0].values[0].value, 50_000);
    }

    #[test]
    fn set_cue_metadata_preserves_stored_targets() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Old".to_string(),
                fade_ms: 100,
                targets: vec![CueFixtureTarget {
                    fixture_id: 1,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 12_345,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetCueMetadata {
                cue_id,
                label: "  Renamed  ".to_string(),
                fade_ms: 750,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .cues
                .first()
                .map(|cue| cue.label.as_str() == "Renamed")
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.cues[0].label, "Renamed");
        assert_eq!(snapshot.cues[0].fade_ms, 750);
        assert_eq!(snapshot.cues[0].targets[0].values[0].value, 12_345);
    }

    #[test]
    fn move_cue_changes_go_back_order() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        for cue_id in [1, 2, 3] {
            engine
                .send(EngineCommand::CreateCue {
                    cue_id,
                    label: format!("Cue {cue_id}"),
                    fade_ms: 0,
                    targets: Vec::new(),
                    video_targets: Vec::new(),
                    video_output_targets: Vec::new(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::MoveCue {
                cue_id: 3,
                delta: -1,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            let order = snapshot.cues.iter().map(|cue| cue.id).collect::<Vec<_>>();
            if order == vec![1, 3, 2] {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(
            snapshot.cues.iter().map(|cue| cue.id).collect::<Vec<_>>(),
            vec![1, 3, 2]
        );

        engine.send(EngineCommand::TriggerNextCue).unwrap();
        engine.send(EngineCommand::TriggerNextCue).unwrap();
        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.active_cue_id == Some(3) {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.active_cue_id, Some(3));
    }

    #[test]
    fn duplicate_cue_copies_targets_and_inserts_after_source() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        for (cue_id, label, value) in [(1, "One", 1_000), (2, "Two", 2_000)] {
            engine
                .send(EngineCommand::CreateCue {
                    cue_id,
                    label: label.to_string(),
                    fade_ms: 100,
                    targets: vec![CueFixtureTarget {
                        fixture_id: cue_id,
                        values: vec![AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value,
                        }],
                    }],
                    video_targets: Vec::new(),
                    video_output_targets: Vec::new(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::DuplicateCue {
                source_cue_id: 1,
                cue_id: 3,
                label: "One Copy".to_string(),
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            let order = snapshot.cues.iter().map(|cue| cue.id).collect::<Vec<_>>();
            if order == vec![1, 3, 2] {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(
            snapshot.cues.iter().map(|cue| cue.id).collect::<Vec<_>>(),
            vec![1, 3, 2]
        );
        assert_eq!(snapshot.cues[1].label, "One Copy");
        assert_eq!(snapshot.cues[1].targets[0].fixture_id, 1);
        assert_eq!(snapshot.cues[1].targets[0].values[0].value, 1_000);
    }

    #[test]
    fn set_timeline_cue_event_updates_and_sorts_events() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        for cue_id in [1, 2] {
            engine
                .send(EngineCommand::CreateCue {
                    cue_id,
                    label: format!("Cue {cue_id}"),
                    fade_ms: 0,
                    targets: Vec::new(),
                    video_targets: Vec::new(),
                    video_output_targets: Vec::new(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::AddTimelineCueEvent {
                event_id: 10,
                cue_id: 1,
                time_ms: 2_000,
                track: TimelineTrackKind::Lighting,
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineCueEvent {
                event_id: 11,
                cue_id: 1,
                time_ms: 4_000,
                track: TimelineTrackKind::Lighting,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetTimelineCueEvent {
                event_id: 11,
                cue_id: 2,
                time_ms: 1_000,
                track: TimelineTrackKind::Video,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.events.first().map(|event| event.id == 11) == Some(true) {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(
            snapshot
                .timeline
                .events
                .iter()
                .map(|event| event.id)
                .collect::<Vec<_>>(),
            vec![11, 10]
        );
        assert_eq!(snapshot.timeline.events[0].cue_id, 2);
        assert_eq!(snapshot.timeline.events[0].time_ms, 1_000);
        assert_eq!(snapshot.timeline.events[0].track, TimelineTrackKind::Video);
    }

    #[test]
    fn set_timeline_automation_replaces_keyframes_and_resorts() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineAutomation {
                automation_id: 7,
                fixture_id,
                attribute: "Dimmer".to_string(),
                keyframes: vec![AutomationKeyframeSummary {
                    time_ms: 0,
                    value: 1_000,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine
            .send(EngineCommand::SetTimelineAutomation {
                automation_id: 7,
                fixture_id,
                attribute: "Pan".to_string(),
                keyframes: vec![
                    AutomationKeyframeSummary {
                        time_ms: 1_000,
                        value: 30_000,
                        interpolation: AutomationInterpolation::Step,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 250,
                        value: 10_000,
                        interpolation: AutomationInterpolation::Bezier,
                    },
                ],
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .timeline
                .automations
                .first()
                .map(|automation| automation.attribute.as_str() == "Pan")
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let automation = &snapshot.timeline.automations[0];
        assert_eq!(automation.fixture_id, fixture_id);
        assert_eq!(automation.attribute, "Pan");
        assert_eq!(automation.keyframes[0].time_ms, 250);
        assert_eq!(automation.keyframes[0].value, 10_000);
        assert_eq!(
            automation.keyframes[0].interpolation,
            AutomationInterpolation::Bezier
        );
        assert_eq!(automation.keyframes[1].time_ms, 1_000);
    }

    #[test]
    fn set_timeline_video_automation_replaces_keyframes_and_resorts() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: 8,
                layer_id,
                param: VideoParam::Opacity,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 1.0,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine
            .send(EngineCommand::SetTimelineVideoAutomation {
                automation_id: 8,
                layer_id,
                param: VideoParam::TransformX,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 1_000,
                        value: 0.75,
                        interpolation: AutomationInterpolation::Step,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 250,
                        value: -0.25,
                        interpolation: AutomationInterpolation::Bezier,
                    },
                ],
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .timeline
                .video_automations
                .first()
                .map(|automation| automation.param == VideoParam::TransformX)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let automation = &snapshot.timeline.video_automations[0];
        assert_eq!(automation.layer_id, layer_id);
        assert_eq!(automation.param, VideoParam::TransformX);
        assert_eq!(automation.keyframes.len(), 2);
        assert_eq!(automation.keyframes[0].time_ms, 250);
        assert_eq!(automation.keyframes[0].value, -0.25);
        assert_eq!(
            automation.keyframes[0].interpolation,
            AutomationInterpolation::Bezier
        );
        assert_eq!(automation.keyframes[1].time_ms, 1_000);
    }

    #[test]
    fn set_fixture_groups_updates_snapshot_and_submasters() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        let start_frame = engine.snapshot().telemetry.frame_counter;
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: vec!["old".to_string()],
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureGroups {
                fixture_id,
                group_ids: vec!["front".to_string(), "movers".to_string()],
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.frame_counter > start_frame
                && snapshot.telemetry.queue_depth == 0
                && snapshot.fixtures.len() == 1
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.fixtures[0].group_ids, vec!["front", "movers"]);
        assert_eq!(
            snapshot
                .submasters
                .iter()
                .map(|submaster| submaster.group_id.as_str())
                .collect::<Vec<_>>(),
            vec!["front", "movers"]
        );
    }

    #[test]
    fn set_fixture_patch_updates_snapshot_and_dmx_address() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        let start_frame = engine.snapshot().telemetry.frame_counter;
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetAttribute {
                fixture_id,
                attribute: "Dimmer".to_string(),
                value: 65_535,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixturePatch {
                fixture_id,
                label: "Renamed".to_string(),
                universe: 0,
                address: 10,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.frame_counter > start_frame
                && snapshot.telemetry.queue_depth == 0
                && snapshot.fixtures.len() == 1
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.fixtures[0].label, "Renamed");
        assert_eq!(snapshot.fixtures[0].universe, 0);
        assert_eq!(snapshot.fixtures[0].address, 10);
        assert_eq!(snapshot.dmx_preview[0], 0);
        assert_eq!(snapshot.dmx_preview[9], 255);
    }

    #[test]
    fn set_group_attribute_updates_every_fixture_in_group() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let first_id = engine.allocate_fixture_id();
        let second_id = engine.allocate_fixture_id();
        let third_id = engine.allocate_fixture_id();
        let start_frame = engine.snapshot().telemetry.frame_counter;

        for (fixture_id, address, group_ids) in [
            (first_id, 1, vec!["front".to_string()]),
            (second_id, 10, vec!["front".to_string(), "side".to_string()]),
            (third_id, 20, vec!["back".to_string()]),
        ] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids,
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::SetGroupAttribute {
                group_id: "front".to_string(),
                attribute: "Dimmer".to_string(),
                value: 65_535,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.frame_counter > start_frame && snapshot.telemetry.queue_depth == 0
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.dmx_preview[9], 255);
        assert_eq!(snapshot.dmx_preview[19], 0);
    }

    #[test]
    fn set_group_fixture_flags_updates_only_group_members() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let first_id = engine.allocate_fixture_id();
        let second_id = engine.allocate_fixture_id();
        let third_id = engine.allocate_fixture_id();
        let start_frame = engine.snapshot().telemetry.frame_counter;

        for (fixture_id, address, group_ids) in [
            (first_id, 1, vec!["front".to_string()]),
            (second_id, 10, vec!["front".to_string(), "side".to_string()]),
            (third_id, 20, vec!["back".to_string()]),
        ] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids,
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
            engine
                .send(EngineCommand::ApplyAttributeValues {
                    fixture_id,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 16_384,
                    }],
                })
                .unwrap();
        }

        for command in [
            EngineCommand::SetGroupHighlight {
                group_id: "front".to_string(),
                enabled: true,
            },
            EngineCommand::SetGroupSolo {
                group_id: "front".to_string(),
                enabled: true,
            },
            EngineCommand::SetGroupPark {
                group_id: "front".to_string(),
                enabled: true,
            },
        ] {
            engine.send(command).unwrap();
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            let first_ready = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == first_id)
                .is_some_and(|fixture| fixture.highlighted && fixture.soloed && fixture.parked);
            let second_ready = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == second_id)
                .is_some_and(|fixture| fixture.highlighted && fixture.soloed && fixture.parked);
            let third_ready = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == third_id)
                .is_some_and(|fixture| !fixture.highlighted && !fixture.soloed && !fixture.parked);
            if snapshot.telemetry.frame_counter > start_frame
                && first_ready
                && second_ready
                && third_ready
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let first = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == first_id)
            .unwrap();
        let second = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == second_id)
            .unwrap();
        let third = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == third_id)
            .unwrap();
        assert!(first.highlighted);
        assert!(first.soloed);
        assert!(first.parked);
        assert!(second.highlighted);
        assert!(second.soloed);
        assert!(second.parked);
        assert!(!third.highlighted);
        assert!(!third.soloed);
        assert!(!third.parked);

        let clear_start_frame = snapshot.telemetry.frame_counter;
        for command in [
            EngineCommand::SetGroupHighlight {
                group_id: "front".to_string(),
                enabled: false,
            },
            EngineCommand::SetGroupSolo {
                group_id: "front".to_string(),
                enabled: false,
            },
            EngineCommand::SetGroupPark {
                group_id: "front".to_string(),
                enabled: false,
            },
        ] {
            engine.send(command).unwrap();
        }

        for _ in 0..20 {
            let first_clear = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == first_id)
                .is_some_and(|fixture| !fixture.highlighted && !fixture.soloed && !fixture.parked);
            let second_clear = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == second_id)
                .is_some_and(|fixture| !fixture.highlighted && !fixture.soloed && !fixture.parked);
            let third_clear = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == third_id)
                .is_some_and(|fixture| !fixture.highlighted && !fixture.soloed && !fixture.parked);
            if snapshot.telemetry.frame_counter > clear_start_frame
                && first_clear
                && second_clear
                && third_clear
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let first = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == first_id)
            .unwrap();
        let second = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == second_id)
            .unwrap();
        let third = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == third_id)
            .unwrap();
        assert!(!first.highlighted);
        assert!(!first.soloed);
        assert!(!first.parked);
        assert!(!second.highlighted);
        assert!(!second.soloed);
        assert!(!second.parked);
        assert!(!third.highlighted);
        assert!(!third.soloed);
        assert!(!third.parked);
    }

    #[test]
    fn telemetry_accumulates_tick_jitter_statistics() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.telemetry.tick_jitter_samples >= 3 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert!(snapshot.telemetry.tick_jitter_samples >= 3);
        assert!(snapshot.telemetry.last_tick_interval_us > 0);
        assert!(snapshot.telemetry.tick_jitter_stddev_us.is_finite());
    }

    #[test]
    fn telemetry_counts_dmx_send_failures() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::EnttecUsbPro,
            target_ip: "127.0.0.1".to_string(),
            port: 6454,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let mut snapshot = engine.snapshot();

        for _ in 0..20 {
            if snapshot.telemetry.total_dmx_send_failure_count > 0 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.telemetry.last_dmx_output_count, 1);
        assert_eq!(snapshot.telemetry.last_dmx_send_success_count, 0);
        assert_eq!(snapshot.telemetry.last_dmx_send_failure_count, 1);
        assert!(snapshot.telemetry.total_dmx_send_failure_count > 0);
        assert!(snapshot
            .telemetry
            .last_error
            .as_deref()
            .unwrap_or_default()
            .contains("serial port path is required"));
    }

    #[test]
    fn lighting_master_scales_intensity_without_touching_movement_channels() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id,
                values: vec![
                    AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    },
                    AttributeValueSummary {
                        attribute: "Pan".to_string(),
                        value: 0x3456,
                    },
                ],
            })
            .unwrap();
        engine.send(EngineCommand::SetLightingMaster(0.5)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 0x80
                && snapshot.dmx_preview[1] == 0x34
                && snapshot.dmx_preview[2] == 0x56
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.lighting_master, 0.5);
        assert_eq!(snapshot.dmx_preview[0], 0x80);
        assert_eq!(snapshot.dmx_preview[1], 0x34);
        assert_eq!(snapshot.dmx_preview[2], 0x56);
        let fixture = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == fixture_id)
            .unwrap();
        assert_eq!(
            fixture
                .attribute_values
                .iter()
                .find(|value| value.attribute == "Dimmer")
                .map(|value| value.value),
            Some(65_535)
        );
    }

    #[test]
    fn group_submaster_scales_grouped_intensity_without_touching_movement_channels() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let front_id = engine.allocate_fixture_id();
        let back_id = engine.allocate_fixture_id();
        for (fixture_id, address, group_ids) in [
            (front_id, 1, vec!["front".to_string()]),
            (back_id, 10, vec!["back".to_string()]),
        ] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids,
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
            engine
                .send(EngineCommand::ApplyAttributeValues {
                    fixture_id,
                    values: vec![
                        AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value: 65_535,
                        },
                        AttributeValueSummary {
                            attribute: "Pan".to_string(),
                            value: 0x1234,
                        },
                    ],
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::SetGroupSubmaster {
                group_id: "front".to_string(),
                level: 0.25,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 64
                && snapshot.dmx_preview[1] == 0x12
                && snapshot.dmx_preview[2] == 0x34
                && snapshot.dmx_preview[9] == 255
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.dmx_preview[0], 64);
        assert_eq!(snapshot.dmx_preview[1], 0x12);
        assert_eq!(snapshot.dmx_preview[2], 0x34);
        assert_eq!(snapshot.dmx_preview[9], 255);
        let front_submaster = snapshot
            .submasters
            .iter()
            .find(|submaster| submaster.group_id == "front")
            .unwrap();
        assert!((front_submaster.level - 0.25).abs() < f32::EPSILON);
    }

    #[test]
    fn highlight_and_solo_affect_dmx_output_without_changing_base_values() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let first_id = engine.allocate_fixture_id();
        let second_id = engine.allocate_fixture_id();
        for (fixture_id, address) in [(first_id, 1), (second_id, 10)] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids: Vec::new(),
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
            engine
                .send(EngineCommand::ApplyAttributeValues {
                    fixture_id,
                    values: vec![
                        AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value: 0,
                        },
                        AttributeValueSummary {
                            attribute: "Pan".to_string(),
                            value: 0x3456,
                        },
                    ],
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::SetFixtureHighlight {
                fixture_id: first_id,
                enabled: true,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixtureSolo {
                fixture_id: first_id,
                enabled: true,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 255
                && snapshot.dmx_preview[1] == 0x34
                && snapshot.dmx_preview[2] == 0x56
                && snapshot.dmx_preview[9] == 0
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.dmx_preview[1], 0x34);
        assert_eq!(snapshot.dmx_preview[2], 0x56);
        assert_eq!(snapshot.dmx_preview[9], 0);
        let first = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == first_id)
            .unwrap();
        let second = snapshot
            .fixtures
            .iter()
            .find(|fixture| fixture.id == second_id)
            .unwrap();
        assert!(first.highlighted);
        assert!(first.soloed);
        assert!(!second.highlighted);
        assert!(!second.soloed);
        assert_eq!(
            first
                .attribute_values
                .iter()
                .find(|value| value.attribute == "Dimmer")
                .map(|value| value.value),
            Some(0)
        );
    }

    #[test]
    fn park_holds_rendered_fixture_values_until_cleared() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Parked Fixture".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id,
                values: vec![
                    AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 32_768,
                    },
                    AttributeValueSummary {
                        attribute: "Pan".to_string(),
                        value: 0x3456,
                    },
                ],
            })
            .unwrap();
        engine
            .send(EngineCommand::SetFixturePark {
                fixture_id,
                enabled: true,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 128
                && snapshot.dmx_preview[1] == 0x34
                && snapshot.dmx_preview[2] == 0x56
                && snapshot
                    .fixtures
                    .iter()
                    .any(|fixture| fixture.id == fixture_id && fixture.parked)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_preview[0], 128);
        assert_eq!(snapshot.dmx_preview[1], 0x34);
        assert_eq!(snapshot.dmx_preview[2], 0x56);

        engine
            .send(EngineCommand::ApplyAttributeValues {
                fixture_id,
                values: vec![
                    AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 0,
                    },
                    AttributeValueSummary {
                        attribute: "Pan".to_string(),
                        value: 0,
                    },
                ],
            })
            .unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            let fixture = snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)
                .unwrap();
            let base_dimmer = fixture
                .attribute_values
                .iter()
                .find(|value| value.attribute == "Dimmer")
                .map(|value| value.value);
            if base_dimmer == Some(0) && snapshot.dmx_preview[0] == 128 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert_eq!(snapshot.dmx_preview[0], 128);
        assert_eq!(snapshot.dmx_preview[1], 0x34);
        assert_eq!(snapshot.dmx_preview[2], 0x56);

        engine
            .send(EngineCommand::SetFixturePark {
                fixture_id,
                enabled: false,
            })
            .unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot.dmx_preview[0] == 0
                && snapshot.dmx_preview[1] == 0
                && snapshot.dmx_preview[2] == 0
                && snapshot
                    .fixtures
                    .iter()
                    .any(|fixture| fixture.id == fixture_id && !fixture.parked)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert_eq!(snapshot.dmx_preview[0], 0);
        assert_eq!(snapshot.dmx_preview[1], 0);
        assert_eq!(snapshot.dmx_preview[2], 0);
        assert!(
            !snapshot
                .fixtures
                .iter()
                .find(|fixture| fixture.id == fixture_id)
                .unwrap()
                .parked
        );
    }

    #[test]
    fn cue_go_and_back_apply_stored_fixture_targets() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        let cue_one = engine.allocate_cue_id();
        let cue_two = engine.allocate_cue_id();
        for (cue_id, label, value) in [(cue_one, "Cue 1", 0), (cue_two, "Cue 2", 65_535)] {
            engine
                .send(EngineCommand::CreateCue {
                    cue_id,
                    label: label.to_string(),
                    fade_ms: 0,
                    targets: vec![CueFixtureTarget {
                        fixture_id,
                        values: vec![AttributeValueSummary {
                            attribute: "Dimmer".to_string(),
                            value,
                        }],
                    }],
                    video_targets: Vec::new(),
                    video_output_targets: Vec::new(),
                })
                .unwrap();
        }
        engine.send(EngineCommand::TriggerNextCue).unwrap();
        engine.send(EngineCommand::TriggerNextCue).unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_cue_two = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 255 {
                saw_cue_two = true;
                break;
            }
        }
        assert!(saw_cue_two);

        engine.send(EngineCommand::TriggerPreviousCue).unwrap();
        let mut saw_cue_one = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 0 {
                saw_cue_one = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.active_cue_id == Some(cue_one) && snapshot.dmx_preview[0] == 0 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_cue_one);
        assert_eq!(snapshot.cues.len(), 2);
        assert_eq!(snapshot.active_cue_id, Some(cue_one));
        assert_eq!(snapshot.dmx_preview[0], 0);
    }

    #[test]
    fn cue_fade_can_pause_and_resume() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Slow Fade".to_string(),
                fade_ms: 1_000,
                targets: vec![CueFixtureTarget {
                    fixture_id,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine.send(EngineCommand::TriggerCue(cue_id)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..30 {
            if snapshot.active_fade.is_some() && (20..=220).contains(&snapshot.dmx_preview[0]) {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(snapshot.active_fade.is_some());

        engine.send(EngineCommand::SetCueFadePaused(true)).unwrap();
        for _ in 0..10 {
            snapshot = engine.snapshot();
            if snapshot.active_fade.as_ref().map(|fade| fade.paused) == Some(true) {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        let paused_value = snapshot.dmx_preview[0];
        assert!(snapshot.active_fade.as_ref().unwrap().paused);
        std::thread::sleep(Duration::from_millis(120));
        snapshot = engine.snapshot();
        assert_eq!(snapshot.dmx_preview[0], paused_value);

        engine.send(EngineCommand::SetCueFadePaused(false)).unwrap();
        for _ in 0..30 {
            snapshot = engine.snapshot();
            if snapshot.active_fade.as_ref().map(|fade| !fade.paused) == Some(true)
                && snapshot.dmx_preview[0] > paused_value
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert!(snapshot.dmx_preview[0] > paused_value);
        assert!(!snapshot.active_fade.unwrap().paused);
    }

    #[test]
    fn timeline_playback_triggers_cue_events() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Flash".to_string(),
                fade_ms: 0,
                targets: vec![CueFixtureTarget {
                    fixture_id,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineCueEvent {
                event_id: engine.allocate_timeline_event_id(),
                cue_id,
                time_ms: 45,
                track: TimelineTrackKind::Lighting,
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(0)).unwrap();
        engine
            .send(EngineCommand::SetTimelinePlaying(true))
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_timeline_cue = false;
        for _ in 0..30 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 255 {
                saw_timeline_cue = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.active_cue_id == Some(cue_id) && snapshot.dmx_preview[0] == 255 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_timeline_cue);
        assert_eq!(snapshot.timeline.events.len(), 1);
        assert_eq!(snapshot.active_cue_id, Some(cue_id));
        assert_eq!(snapshot.dmx_preview[0], 255);
    }

    #[test]
    fn timecode_sync_updates_timeline_and_triggers_due_cues() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Timecode Flash".to_string(),
                fade_ms: 0,
                targets: vec![CueFixtureTarget {
                    fixture_id,
                    values: vec![AttributeValueSummary {
                        attribute: "Dimmer".to_string(),
                        value: 65_535,
                    }],
                }],
                video_targets: Vec::new(),
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineCueEvent {
                event_id: engine.allocate_timeline_event_id(),
                cue_id,
                time_ms: 1_000,
                track: TimelineTrackKind::Lighting,
            })
            .unwrap();
        engine
            .send(EngineCommand::SyncTimelineTimecode(1_000))
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.active_cue_id == Some(cue_id) && snapshot.timeline.position_ms == 1_000 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.timeline.position_ms, 1_000);
        assert!(!snapshot.timeline.playing);
        assert_eq!(snapshot.active_cue_id, Some(cue_id));
        assert_eq!(snapshot.dmx_preview[0], 255);
    }

    #[test]
    fn timeline_attribute_automation_interpolates_dmx_values() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineAutomation {
                automation_id: engine.allocate_automation_id(),
                fixture_id,
                attribute: "Dimmer".to_string(),
                keyframes: vec![
                    AutomationKeyframeSummary {
                        time_ms: 0,
                        value: 0,
                        interpolation: AutomationInterpolation::Linear,
                    },
                    AutomationKeyframeSummary {
                        time_ms: 100,
                        value: 65_535,
                        interpolation: AutomationInterpolation::Step,
                    },
                ],
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(50)).unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_mid_value = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if (120..=136).contains(&packet.data[0]) {
                saw_mid_value = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if (120..=136).contains(&snapshot.dmx_preview[0])
                && snapshot.timeline.automations.len() == 1
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_mid_value);
        assert_eq!(snapshot.timeline.automations.len(), 1);
        assert!((120..=136).contains(&snapshot.dmx_preview[0]));
    }

    #[test]
    fn bezier_automation_eases_lighting_and_video_values() {
        let lighting_keys = vec![
            AutomationKeyframeSummary {
                time_ms: 0,
                value: 0,
                interpolation: AutomationInterpolation::Bezier,
            },
            AutomationKeyframeSummary {
                time_ms: 1_000,
                value: 65_535,
                interpolation: AutomationInterpolation::Step,
            },
        ];
        assert_eq!(
            evaluate_automation_keyframes(&lighting_keys, 250),
            Some(10_240)
        );
        assert_eq!(
            evaluate_automation_keyframes(&lighting_keys, 500),
            Some(32_768)
        );
        assert_eq!(
            evaluate_automation_keyframes(&lighting_keys, 750),
            Some(55_295)
        );

        let video_keys = vec![
            VideoAutomationKeyframeSummary {
                time_ms: 0,
                value: 0.0,
                interpolation: AutomationInterpolation::Bezier,
            },
            VideoAutomationKeyframeSummary {
                time_ms: 1_000,
                value: 1.0,
                interpolation: AutomationInterpolation::Step,
            },
        ];
        assert!(
            (evaluate_video_automation_keyframes(&video_keys, 250).unwrap() - 0.15625).abs()
                < 0.0001
        );
        assert!(
            (evaluate_video_automation_keyframes(&video_keys, 750).unwrap() - 0.84375).abs()
                < 0.0001
        );
    }

    #[test]
    fn cue_can_trigger_video_layer_state() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Video Cue".to_string(),
                fade_ms: 0,
                targets: Vec::new(),
                video_targets: vec![VideoLayerTarget {
                    layer_id,
                    state: VideoLayerState {
                        enabled: true,
                        solo: false,
                        opacity: 0.5,
                        speed: 2.0,
                        playing: true,
                        position_ms: 1_000,
                        loop_enabled: true,
                        loop_start_ms: 500,
                        loop_end_ms: 2_000,
                        bpm_sync: Default::default(),
                        cue_points_ms: Vec::new(),
                        transform: Default::default(),
                        color: Default::default(),
                        fx: Default::default(),
                    },
                }],
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine.send(EngineCommand::TriggerCue(cue_id)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.playing)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.cues[0].video_targets.len(), 1);
        assert_eq!(layer.id, layer_id);
        assert_eq!(layer.state.opacity, 0.5);
        assert_eq!(layer.state.speed, 2.0);
        assert!(layer.state.playing);
        assert!((1_000..=1_100).contains(&layer.state.position_ms));
    }

    #[test]
    fn duplicate_video_layer_copies_state_and_composition_membership() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let source_layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id: source_layer_id,
                label: "Source".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerBlendMode {
                layer_id: source_layer_id,
                blend_mode: VideoBlendMode::Add,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id: source_layer_id,
                state: VideoLayerState {
                    opacity: 0.42,
                    speed: 2.0,
                    playing: true,
                    ..VideoLayerState::default()
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddVideoComposition(CompositionSummary {
                id: 2,
                label: "Aux".to_string(),
                layer_ids: vec![source_layer_id],
                output_ids: Vec::new(),
            }))
            .unwrap();

        let duplicate_layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::DuplicateVideoLayer {
                source_layer_id,
                new_layer_id: duplicate_layer_id,
                label: "  Copy  ".to_string(),
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerLabel {
                layer_id: duplicate_layer_id,
                label: "  Renamed Copy  ".to_string(),
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.video.layers.len() == 2
                && snapshot
                    .video
                    .layers
                    .iter()
                    .any(|layer| layer.id == duplicate_layer_id && layer.label == "Renamed Copy")
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(
            snapshot
                .video
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>(),
            vec![source_layer_id, duplicate_layer_id]
        );
        let duplicate = snapshot
            .video
            .layers
            .iter()
            .find(|layer| layer.id == duplicate_layer_id)
            .unwrap();
        assert_eq!(duplicate.label, "Renamed Copy");
        assert_eq!(duplicate.source.path.as_deref(), Some("memory://clip.mp4"));
        assert_eq!(duplicate.source.codec.as_deref(), Some("H264"));
        assert_eq!(duplicate.blend_mode, VideoBlendMode::Add);
        assert_eq!(duplicate.state.opacity, 0.42);
        assert_eq!(duplicate.state.speed, 2.0);
        assert!(duplicate.state.playing);
        let aux = snapshot
            .video
            .compositions
            .iter()
            .find(|composition| composition.id == 2)
            .unwrap();
        assert_eq!(aux.layer_ids, vec![source_layer_id, duplicate_layer_id]);
    }

    #[test]
    fn video_layer_enabled_solo_commands_update_state_without_overwriting_level() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Remote Layer".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id,
                state: VideoLayerState {
                    opacity: 0.33,
                    playing: true,
                    ..VideoLayerState::default()
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerEnabled {
                layer_id,
                enabled: false,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerSolo {
                layer_id,
                solo: true,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            let ready = snapshot
                .video
                .layers
                .iter()
                .find(|layer| layer.id == layer_id)
                .map(|layer| !layer.state.enabled && layer.state.solo)
                == Some(true);
            if ready {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot
            .video
            .layers
            .iter()
            .find(|layer| layer.id == layer_id)
            .unwrap();
        assert!(!layer.state.enabled);
        assert!(layer.state.solo);
        assert_eq!(layer.state.opacity, 0.33);
        assert!(layer.state.playing);
    }

    #[test]
    fn cue_fade_interpolates_video_layer_visual_state() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id,
                state: VideoLayerState {
                    opacity: 0.0,
                    speed: 1.0,
                    transform: Transform2D {
                        scale_x: 1.0,
                        scale_y: 1.0,
                        ..Transform2D::default()
                    },
                    ..VideoLayerState::default()
                },
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Video Fade".to_string(),
                fade_ms: 1_000,
                targets: Vec::new(),
                video_targets: vec![VideoLayerTarget {
                    layer_id,
                    state: VideoLayerState {
                        opacity: 1.0,
                        speed: 3.0,
                        playing: true,
                        position_ms: 2_000,
                        transform: Transform2D {
                            x: 0.5,
                            scale_x: 2.0,
                            scale_y: 1.5,
                            ..Transform2D::default()
                        },
                        color: VideoColorAdjust {
                            hue_deg: 180.0,
                            saturation: 0.5,
                            ..VideoColorAdjust::default()
                        },
                        fx: VideoFxAdjust {
                            blur: 4.0,
                            glow: 0.5,
                            ..VideoFxAdjust::default()
                        },
                        ..VideoLayerState::default()
                    },
                }],
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine.send(EngineCommand::TriggerCue(cue_id)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..30 {
            let Some(layer) = snapshot.video.layers.first() else {
                std::thread::sleep(DMX_TICK_INTERVAL);
                snapshot = engine.snapshot();
                continue;
            };
            if snapshot.active_fade.is_some()
                && layer.state.playing
                && (0.05..0.95).contains(&layer.state.opacity)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(snapshot.active_fade.is_some());
        assert!(layer.state.playing);
        assert!(layer.state.position_ms >= 2_000);
        assert!((0.05..0.95).contains(&layer.state.opacity));
        assert!((1.0..3.0).contains(&layer.state.speed));
        assert!((1.0..2.0).contains(&layer.state.transform.scale_x));
        assert!((0.0..180.0).contains(&layer.state.color.hue_deg));
        assert!((0.0..4.0).contains(&layer.state.fx.blur));

        for _ in 0..70 {
            if snapshot.active_fade.is_none()
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| layer.state.opacity >= 0.99)
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(snapshot.active_fade.is_none());
        assert!((layer.state.opacity - 1.0).abs() < 0.01);
        assert!((layer.state.speed - 3.0).abs() < 0.01);
        assert!((layer.state.transform.scale_x - 2.0).abs() < 0.01);
        assert!((layer.state.color.hue_deg - 180.0).abs() < 0.01);
        assert!((layer.state.fx.blur - 4.0).abs() < 0.01);
    }

    #[test]
    fn playing_video_layer_position_advances_on_engine_tick() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id,
                state: VideoLayerState {
                    playing: true,
                    position_ms: 100,
                    speed: 2.0,
                    ..VideoLayerState::default()
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.position_ms > 100)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(layer.state.playing);
        assert!(layer.state.position_ms > 100);
    }

    #[test]
    fn playing_video_layer_stops_at_source_duration() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: Some(VideoMediaMetadata {
                        duration_ms: Some(120),
                        width: None,
                        height: None,
                        frame_rate: None,
                    }),
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id,
                state: VideoLayerState {
                    playing: true,
                    position_ms: 100,
                    speed: 2.0,
                    ..VideoLayerState::default()
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| !layer.state.playing && layer.state.position_ms == 120)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(layer.state.position_ms, 120);
        assert!(!layer.state.playing);
    }

    #[test]
    fn video_cue_point_commands_add_remove_and_jump() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::PositionMs,
                value: 250.0,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerPlaying {
                layer_id,
                playing: false,
            })
            .unwrap();
        engine
            .send(EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms: None,
            })
            .unwrap();
        engine
            .send(EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms: Some(100),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddVideoCuePoint {
                layer_id,
                position_ms: Some(100),
            })
            .unwrap();
        engine
            .send(EngineCommand::JumpVideoCuePoint {
                layer_id,
                cue_point_index: 1,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.playing && layer.state.position_ms == 250)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(!layer.state.playing);
        assert_eq!(layer.state.cue_points_ms, vec![100, 250]);
        assert_eq!(layer.state.position_ms, 250);

        engine
            .send(EngineCommand::RemoveVideoCuePoint {
                layer_id,
                position_ms: 100,
            })
            .unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.cue_points_ms == vec![250])
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }

        assert_eq!(
            snapshot.video.layers.first().unwrap().state.cue_points_ms,
            vec![250]
        );
    }

    #[test]
    fn video_layer_loop_command_updates_bounds_and_sanitizes() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://loop.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerLoop {
                layer_id,
                enabled: true,
                loop_start_ms: Some(900),
                loop_end_ms: Some(500),
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.loop_enabled && layer.state.loop_end_ms == 901)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(layer.state.loop_enabled);
        assert_eq!(layer.state.loop_start_ms, 900);
        assert_eq!(layer.state.loop_end_ms, 901);

        engine
            .send(EngineCommand::SetVideoLayerLoop {
                layer_id,
                enabled: false,
                loop_start_ms: None,
                loop_end_ms: None,
            })
            .unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| !layer.state.loop_enabled)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }

        assert!(!snapshot.video.layers.first().unwrap().state.loop_enabled);
    }

    #[test]
    fn video_bpm_sync_params_update_and_sanitize_layer_state() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://loop.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::BpmSyncEnabled,
                value: 1.0,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::BpmSyncRatio,
                value: 10.0,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::BpmSyncLoopBars,
                value: 0.0,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.video.layers.first().map(|layer| {
                layer.state.bpm_sync.enabled
                    && (layer.state.bpm_sync.ratio - 4.0).abs() < f32::EPSILON
                    && (layer.state.bpm_sync.loop_bars - 0.25).abs() < f32::EPSILON
            }) == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(layer.state.bpm_sync.enabled);
        assert_eq!(layer.state.bpm_sync.ratio, 4.0);
        assert_eq!(layer.state.bpm_sync.loop_bars, 0.25);
    }

    #[test]
    fn bpm_synced_video_layer_advances_even_with_manual_speed_zero() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://loop.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerState {
                layer_id,
                state: VideoLayerState {
                    playing: true,
                    speed: 0.0,
                    loop_enabled: true,
                    loop_start_ms: 0,
                    loop_end_ms: 2_000,
                    bpm_sync: protocol::VideoBpmSync {
                        enabled: true,
                        ratio: 1.0,
                        loop_bars: 1.0,
                    },
                    ..VideoLayerState::default()
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.state.position_ms > 0)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert!(layer.state.bpm_sync.enabled);
        assert!(layer.state.position_ms > 0);
    }

    #[test]
    fn video_layer_blend_mode_and_default_composition_are_in_snapshot() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerBlendMode {
                layer_id,
                blend_mode: VideoBlendMode::Screen,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoLayerParam {
                layer_id,
                param: VideoParam::Opacity,
                value: 0.4,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoMasterOpacity(0.5))
            .unwrap();
        engine.send(EngineCommand::SetVideoBlackout(true)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .layers
                .first()
                .map(|layer| layer.blend_mode == VideoBlendMode::Screen)
                == Some(true)
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (0.39..=0.41).contains(&layer.state.opacity))
                    == Some(true)
                && snapshot.video.master_opacity == 0.5
                && snapshot.video.blackout
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(layer.blend_mode, VideoBlendMode::Screen);
        assert!((0.39..=0.41).contains(&layer.state.opacity));
        assert_eq!(snapshot.video.compositions.len(), 1);
        assert_eq!(snapshot.video.compositions[0].label, "Main");
        assert_eq!(snapshot.video.compositions[0].layer_ids, vec![layer_id]);
        assert_eq!(snapshot.video.master_opacity, 0.5);
        assert!(snapshot.video.blackout);
    }

    #[test]
    fn video_layer_order_updates_main_composition_order() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_one = engine.allocate_video_layer_id();
        let layer_two = engine.allocate_video_layer_id();
        let layer_three = engine.allocate_video_layer_id();
        for (layer_id, label) in [
            (layer_one, "Layer 1"),
            (layer_two, "Layer 2"),
            (layer_three, "Layer 3"),
        ] {
            engine
                .send(EngineCommand::AddVideoLayer {
                    layer_id,
                    label: label.to_string(),
                    source: VideoSourceSummary {
                        kind: protocol::VideoSourceKind::File,
                        path: Some(format!("memory://{label}.mp4")),
                        name: None,
                        codec: Some("H264".to_string()),
                        metadata: None,
                    },
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::SetVideoLayerOrder(vec![
                layer_three,
                layer_one,
                99_999,
                layer_three,
            ]))
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            let order = snapshot
                .video
                .layers
                .iter()
                .map(|layer| layer.id)
                .collect::<Vec<_>>();
            if order == vec![layer_three, layer_one, layer_two] {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer_order = snapshot
            .video
            .layers
            .iter()
            .map(|layer| layer.id)
            .collect::<Vec<_>>();
        assert_eq!(layer_order, vec![layer_three, layer_one, layer_two]);
        assert_eq!(
            snapshot.video.compositions[0].layer_ids,
            vec![layer_three, layer_one, layer_two]
        );
    }

    #[test]
    fn video_outputs_are_routed_to_composition_snapshot() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let output_id = engine.allocate_video_output_id();
        engine
            .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
                id: output_id,
                label: "Projector".to_string(),
                kind: protocol::VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: Some(2),
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 1.5,
                blackout: false,
                mapping: Default::default(),
            }))
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputBlackout {
                output_id,
                blackout: true,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputOpacity {
                output_id,
                opacity: 0.25,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputMapping {
                output_id,
                mapping: VideoOutputMapping {
                    aspect_ratio: 1.33,
                    keystone_x: 0.2,
                    ..Default::default()
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .outputs
                .first()
                .map(|output| output.blackout && (output.opacity - 0.25).abs() < f32::EPSILON)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.video.outputs.len(), 1);
        assert_eq!(snapshot.video.outputs[0].id, output_id);
        assert_eq!(snapshot.video.outputs[0].opacity, 0.25);
        assert!(snapshot.video.outputs[0].blackout);
        assert!((snapshot.video.outputs[0].mapping.aspect_ratio - 1.33).abs() < f32::EPSILON);
        assert!((snapshot.video.outputs[0].mapping.keystone_x - 0.2).abs() < f32::EPSILON);
        assert_eq!(snapshot.video.compositions[0].output_ids, vec![output_id]);
    }

    #[test]
    fn video_output_opacity_can_fade_over_engine_ticks() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let output_id = engine.allocate_video_output_id();
        engine
            .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
                id: output_id,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: Some(0),
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }))
            .unwrap();
        engine
            .send(EngineCommand::FadeVideoOutputOpacity {
                output_id,
                opacity: 0.25,
                duration_ms: 45,
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .outputs
                .first()
                .map(|output| (output.opacity - 0.25).abs() < 0.001)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.video.outputs.len(), 1);
        assert!((snapshot.video.outputs[0].opacity - 0.25).abs() < 0.001);
        assert!(!snapshot.video.outputs[0].blackout);

        engine
            .send(EngineCommand::SetVideoOutputOpacity {
                output_id,
                opacity: 0.75,
            })
            .unwrap();
        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot
                .video
                .outputs
                .first()
                .map(|output| (output.opacity - 0.75).abs() < 0.001)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert!((snapshot.video.outputs[0].opacity - 0.75).abs() < 0.001);
    }

    #[test]
    fn cue_can_recall_and_fade_video_output_state() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let output_id = engine.allocate_video_output_id();
        engine
            .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
                id: output_id,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: Some(0),
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }))
            .unwrap();

        let snap_cue = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id: snap_cue,
                label: "Output Snap".to_string(),
                fade_ms: 0,
                targets: Vec::new(),
                video_targets: Vec::new(),
                video_output_targets: vec![VideoOutputTarget {
                    output_id,
                    enabled: false,
                    opacity: 0.25,
                    blackout: true,
                }],
            })
            .unwrap();
        engine.send(EngineCommand::TriggerCue(snap_cue)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .outputs
                .first()
                .map(|output| !output.enabled && output.blackout)
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        let output = snapshot.video.outputs.first().unwrap();
        assert!(!output.enabled);
        assert!(output.blackout);
        assert!((output.opacity - 0.25).abs() < 0.001);
        assert_eq!(snapshot.cues[0].video_output_targets.len(), 1);

        let fade_cue = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id: fade_cue,
                label: "Output Fade".to_string(),
                fade_ms: 90,
                targets: Vec::new(),
                video_targets: Vec::new(),
                video_output_targets: vec![VideoOutputTarget {
                    output_id,
                    enabled: true,
                    opacity: 0.8,
                    blackout: false,
                }],
            })
            .unwrap();
        engine.send(EngineCommand::TriggerCue(fade_cue)).unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot.active_fade.is_none()
                && snapshot
                    .video
                    .outputs
                    .first()
                    .map(|output| (output.opacity - 0.8).abs() < 0.001)
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }

        let output = snapshot.video.outputs.first().unwrap();
        assert!(output.enabled);
        assert!(!output.blackout);
        assert!((output.opacity - 0.8).abs() < 0.001);
        assert_eq!(snapshot.active_fade, None);
    }

    #[test]
    fn video_output_mapping_presets_are_saved_replaced_removed_and_sanitized() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        engine
            .send(EngineCommand::SaveVideoOutputMappingPreset {
                label: "  Front Screen  ".to_string(),
                mapping: VideoOutputMapping {
                    aspect_ratio: 16.0 / 9.0,
                    scale_x: 0.0,
                    keystone_y: 0.25,
                    ..Default::default()
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::SaveVideoOutputMappingPreset {
                label: "Front Screen".to_string(),
                mapping: VideoOutputMapping {
                    aspect_ratio: 4.0 / 3.0,
                    lens_distortion: 2.0,
                    ..Default::default()
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.video.mapping_presets.len() == 1
                && (snapshot.video.mapping_presets[0].mapping.aspect_ratio - 4.0 / 3.0).abs()
                    < f32::EPSILON
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        assert_eq!(snapshot.video.mapping_presets.len(), 1);
        let preset = &snapshot.video.mapping_presets[0];
        assert_eq!(preset.label, "Front Screen");
        assert!((preset.mapping.aspect_ratio - 4.0 / 3.0).abs() < f32::EPSILON);
        assert_eq!(preset.mapping.scale_x, 1.0);
        assert_eq!(preset.mapping.lens_distortion, 1.0);

        engine
            .send(EngineCommand::RemoveVideoOutputMappingPreset {
                label: "Front Screen".to_string(),
            })
            .unwrap();
        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot.video.mapping_presets.is_empty() {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert!(snapshot.video.mapping_presets.is_empty());
    }

    #[test]
    fn video_output_config_update_preserves_state_and_mapping() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let output_id = engine.allocate_video_output_id();
        engine
            .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
                id: output_id,
                label: "Projector".to_string(),
                kind: VideoOutputKind::Display,
                enabled: true,
                composition_id: 1,
                fullscreen: true,
                monitor_id: Some(2),
                width: 1920,
                height: 1080,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: VideoOutputMapping {
                    aspect_ratio: 1.33,
                    keystone_x: 0.2,
                    ..Default::default()
                },
            }))
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputOpacity {
                output_id,
                opacity: 0.4,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputBlackout {
                output_id,
                blackout: true,
            })
            .unwrap();
        engine
            .send(EngineCommand::SetVideoOutputConfig {
                output_id,
                label: "  Main NDI  ".to_string(),
                kind: VideoOutputKind::NdiSender,
                fullscreen: true,
                monitor_id: Some(5),
                width: 1280,
                height: 720,
                endpoint_name: Some("  Stage Feed  ".to_string()),
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot
                .video
                .outputs
                .first()
                .map(|output| output.label == "Main NDI")
                == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let output = &snapshot.video.outputs[0];
        assert_eq!(output.kind, VideoOutputKind::NdiSender);
        assert!(!output.fullscreen);
        assert_eq!(output.monitor_id, None);
        assert_eq!(output.width, 1280);
        assert_eq!(output.height, 720);
        assert_eq!(output.endpoint_name.as_deref(), Some("Stage Feed"));
        assert_eq!(output.composition_id, 1);
        assert!((output.opacity - 0.4).abs() < f32::EPSILON);
        assert!(output.blackout);
        assert!((output.mapping.aspect_ratio - 1.33).abs() < f32::EPSILON);
        assert!((output.mapping.keystone_x - 0.2).abs() < f32::EPSILON);
    }

    #[test]
    fn custom_video_compositions_filter_layers_and_reroute_outputs() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_one = engine.allocate_video_layer_id();
        let layer_two = engine.allocate_video_layer_id();
        for (layer_id, label) in [(layer_one, "Layer 1"), (layer_two, "Layer 2")] {
            engine
                .send(EngineCommand::AddVideoLayer {
                    layer_id,
                    label: label.to_string(),
                    source: VideoSourceSummary {
                        kind: protocol::VideoSourceKind::File,
                        path: Some(format!("memory://{label}.mp4")),
                        name: None,
                        codec: Some("H264".to_string()),
                        metadata: None,
                    },
                })
                .unwrap();
        }

        let composition_id = engine.allocate_composition_id();
        engine
            .send(EngineCommand::AddVideoComposition(CompositionSummary {
                id: composition_id,
                label: "Aux".to_string(),
                layer_ids: vec![layer_one, 99_999, layer_one],
                output_ids: vec![99_999],
            }))
            .unwrap();
        let output_id = engine.allocate_video_output_id();
        engine
            .send(EngineCommand::AddVideoOutput(VideoOutputSummary {
                id: output_id,
                label: "Aux Projector".to_string(),
                kind: protocol::VideoOutputKind::Display,
                enabled: true,
                composition_id,
                fullscreen: true,
                monitor_id: Some(1),
                width: 1280,
                height: 720,
                endpoint_name: None,
                opacity: 1.0,
                blackout: false,
                mapping: Default::default(),
            }))
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.video.compositions.len() == 2
                && snapshot.video.compositions.iter().any(|composition| {
                    composition.id == composition_id
                        && composition.layer_ids == vec![layer_one]
                        && composition.output_ids == vec![output_id]
                })
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        let main = snapshot
            .video
            .compositions
            .iter()
            .find(|composition| composition.id == 1)
            .unwrap();
        let aux = snapshot
            .video
            .compositions
            .iter()
            .find(|composition| composition.id == composition_id)
            .unwrap();
        assert_eq!(main.layer_ids, vec![layer_one, layer_two]);
        assert_eq!(aux.layer_ids, vec![layer_one]);
        assert_eq!(aux.output_ids, vec![output_id]);
        assert_eq!(snapshot.video.outputs[0].composition_id, composition_id);

        engine
            .send(EngineCommand::SetVideoCompositionLayers {
                composition_id,
                layer_ids: vec![layer_two, layer_one, layer_two],
            })
            .unwrap();
        for _ in 0..20 {
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
            let aux = snapshot
                .video
                .compositions
                .iter()
                .find(|composition| composition.id == composition_id)
                .unwrap();
            if aux.layer_ids == vec![layer_two, layer_one] {
                break;
            }
        }
        let aux = snapshot
            .video
            .compositions
            .iter()
            .find(|composition| composition.id == composition_id)
            .unwrap();
        assert_eq!(aux.layer_ids, vec![layer_two, layer_one]);

        engine
            .send(EngineCommand::RemoveVideoLayer(layer_two))
            .unwrap();
        for _ in 0..20 {
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
            let aux = snapshot
                .video
                .compositions
                .iter()
                .find(|composition| composition.id == composition_id)
                .unwrap();
            if aux.layer_ids == vec![layer_one] {
                break;
            }
        }
        let aux = snapshot
            .video
            .compositions
            .iter()
            .find(|composition| composition.id == composition_id)
            .unwrap();
        assert_eq!(aux.layer_ids, vec![layer_one]);
        assert_eq!(snapshot.video.compositions[0].layer_ids, vec![layer_one]);

        engine
            .send(EngineCommand::RemoveVideoComposition(composition_id))
            .unwrap();
        for _ in 0..20 {
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
            if snapshot.video.compositions.len() == 1
                && snapshot.video.outputs[0].composition_id == 1
                && snapshot.video.compositions[0].output_ids == vec![output_id]
            {
                break;
            }
        }
        assert_eq!(snapshot.video.compositions.len(), 1);
        assert_eq!(snapshot.video.outputs[0].composition_id, 1);
        assert_eq!(snapshot.video.compositions[0].output_ids, vec![output_id]);
    }

    #[test]
    fn timeline_video_track_event_can_trigger_video_only_cue() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        let cue_id = engine.allocate_cue_id();
        engine
            .send(EngineCommand::CreateCue {
                cue_id,
                label: "Video Only".to_string(),
                fade_ms: 0,
                targets: Vec::new(),
                video_targets: vec![VideoLayerTarget {
                    layer_id,
                    state: VideoLayerState {
                        opacity: 0.25,
                        playing: true,
                        position_ms: 250,
                        ..VideoLayerState::default()
                    },
                }],
                video_output_targets: Vec::new(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineCueEvent {
                event_id: engine.allocate_timeline_event_id(),
                cue_id,
                time_ms: 45,
                track: TimelineTrackKind::Video,
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(0)).unwrap();
        engine
            .send(EngineCommand::SetTimelinePlaying(true))
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.active_cue_id == Some(cue_id)
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| layer.state.playing)
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.active_cue_id, Some(cue_id));
        assert_eq!(snapshot.timeline.events[0].track, TimelineTrackKind::Video);
        assert_eq!(layer.state.opacity, 0.25);
        assert!(layer.state.playing);
    }

    #[test]
    fn timeline_video_automation_interpolates_layer_opacity() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::Opacity,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 0,
                        value: 0.0,
                        interpolation: AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 100,
                        value: 1.0,
                        interpolation: AutomationInterpolation::Step,
                    },
                ],
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(50)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.video_automations.len() == 1
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (0.49..=0.51).contains(&layer.state.opacity))
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.timeline.video_automations.len(), 1);
        assert!((0.49..=0.51).contains(&layer.state.opacity));
    }

    #[test]
    fn timeline_video_automation_interpolates_transform_params() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::TransformX,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 0,
                        value: -0.5,
                        interpolation: AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 100,
                        value: 0.5,
                        interpolation: AutomationInterpolation::Step,
                    },
                ],
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::TransformCropLeft,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 2.0,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(50)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.video_automations.len() == 2
                && snapshot.video.layers.first().map(|layer| {
                    (-0.01..=0.01).contains(&layer.state.transform.x)
                        && layer.state.transform.crop_left == 1.0
                }) == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.timeline.video_automations.len(), 2);
        assert!((-0.01..=0.01).contains(&layer.state.transform.x));
        assert_eq!(layer.state.transform.crop_left, 1.0);
    }

    #[test]
    fn timeline_video_automation_interpolates_color_params() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::ColorBrightness,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 0,
                        value: -0.5,
                        interpolation: AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 100,
                        value: 0.5,
                        interpolation: AutomationInterpolation::Step,
                    },
                ],
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::ColorGamma,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 10.0,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(50)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.video_automations.len() == 2
                && snapshot.video.layers.first().map(|layer| {
                    (-0.01..=0.01).contains(&layer.state.color.brightness)
                        && layer.state.color.gamma == 4.0
                }) == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.timeline.video_automations.len(), 2);
        assert!((-0.01..=0.01).contains(&layer.state.color.brightness));
        assert_eq!(layer.state.color.gamma, 4.0);
    }

    #[test]
    fn timeline_video_automation_interpolates_fx_params() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::FxPixelate,
                keyframes: vec![
                    VideoAutomationKeyframeSummary {
                        time_ms: 0,
                        value: 1.0,
                        interpolation: AutomationInterpolation::Linear,
                    },
                    VideoAutomationKeyframeSummary {
                        time_ms: 100,
                        value: 9.0,
                        interpolation: AutomationInterpolation::Step,
                    },
                ],
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::FxBlur,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 20.0,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine
            .send(EngineCommand::AddTimelineVideoAutomation {
                automation_id: engine.allocate_automation_id(),
                layer_id,
                param: VideoParam::FxKeyThreshold,
                keyframes: vec![VideoAutomationKeyframeSummary {
                    time_ms: 0,
                    value: 2.0,
                    interpolation: AutomationInterpolation::Step,
                }],
            })
            .unwrap();
        engine.send(EngineCommand::SeekTimeline(50)).unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.timeline.video_automations.len() == 3
                && snapshot.video.layers.first().map(|layer| {
                    (4.9..=5.1).contains(&layer.state.fx.pixelate)
                        && layer.state.fx.blur == 8.0
                        && layer.state.fx.key_threshold == 1.0
                }) == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.timeline.video_automations.len(), 3);
        assert!((4.9..=5.1).contains(&layer.state.fx.pixelate));
        assert_eq!(layer.state.fx.blur, 8.0);
        assert_eq!(layer.state.fx.key_threshold, 1.0);
    }

    #[test]
    fn lfo_effect_can_target_video_layer_opacity() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: engine.allocate_effect_id(),
                request: LfoEffectRequest {
                    label: "Video Opacity Pulse".to_string(),
                    fixture_ids: Vec::new(),
                    target_group_ids: Vec::new(),
                    attribute: String::new(),
                    video_targets: vec![VideoEffectTarget {
                        layer_ids: vec![layer_id],
                        param: VideoParam::Opacity,
                        low: 0.25,
                        high: 0.75,
                    }],
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.effects.len() == 1
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (0.74..=0.76).contains(&layer.state.opacity))
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.effects.len(), 1);
        assert!((0.74..=0.76).contains(&layer.state.opacity));
        assert_eq!(
            snapshot.effects[0].video_targets[0].param,
            VideoParam::Opacity
        );
    }

    #[test]
    fn lfo_effect_can_target_video_layer_color() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: engine.allocate_effect_id(),
                request: LfoEffectRequest {
                    label: "Video Saturation Pulse".to_string(),
                    fixture_ids: Vec::new(),
                    target_group_ids: Vec::new(),
                    attribute: String::new(),
                    video_targets: vec![VideoEffectTarget {
                        layer_ids: vec![layer_id],
                        param: VideoParam::ColorSaturation,
                        low: 0.5,
                        high: 2.0,
                    }],
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.effects.len() == 1
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (1.99..=2.01).contains(&layer.state.color.saturation))
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.effects.len(), 1);
        assert!((1.99..=2.01).contains(&layer.state.color.saturation));
        assert_eq!(
            snapshot.effects[0].video_targets[0].param,
            VideoParam::ColorSaturation
        );
    }

    #[test]
    fn lfo_effect_can_target_video_layer_fx() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let layer_id = engine.allocate_video_layer_id();
        engine
            .send(EngineCommand::AddVideoLayer {
                layer_id,
                label: "Layer 1".to_string(),
                source: VideoSourceSummary {
                    kind: protocol::VideoSourceKind::File,
                    path: Some("memory://clip.mp4".to_string()),
                    name: None,
                    codec: Some("H264".to_string()),
                    metadata: None,
                },
            })
            .unwrap();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: engine.allocate_effect_id(),
                request: LfoEffectRequest {
                    label: "Video Edge Pulse".to_string(),
                    fixture_ids: Vec::new(),
                    target_group_ids: Vec::new(),
                    attribute: String::new(),
                    video_targets: vec![VideoEffectTarget {
                        layer_ids: vec![layer_id],
                        param: VideoParam::FxEdge,
                        low: 0.0,
                        high: 4.0,
                    }],
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.effects.len() == 1
                && snapshot
                    .video
                    .layers
                    .first()
                    .map(|layer| (3.99..=4.01).contains(&layer.state.fx.edge))
                    == Some(true)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }

        let layer = snapshot.video.layers.first().unwrap();
        assert_eq!(snapshot.effects.len(), 1);
        assert!((3.99..=4.01).contains(&layer.state.fx.edge));
        assert_eq!(
            snapshot.effects[0].video_targets[0].param,
            VideoParam::FxEdge
        );
    }

    #[test]
    fn lfo_effect_overrides_target_attribute_in_artnet_and_preview() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: engine.allocate_effect_id(),
                request: LfoEffectRequest {
                    label: "Dimmer Pulse".to_string(),
                    fixture_ids: vec![fixture_id],
                    target_group_ids: Vec::new(),
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_expected_dimmer = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 255 {
                saw_expected_dimmer = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 255 && snapshot.effects.len() == 1 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_expected_dimmer);
        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.effects.len(), 1);
    }

    #[test]
    fn effect_enabled_toggle_suspends_effect_without_removing_it() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();
        let effect_id = engine.allocate_effect_id();
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id,
                request: LfoEffectRequest {
                    label: "Dimmer Pulse".to_string(),
                    fixture_ids: vec![fixture_id],
                    target_group_ids: Vec::new(),
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 255 && snapshot.effects.len() == 1 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_preview[0], 255);

        engine
            .send(EngineCommand::SetEffectEnabled {
                effect_id,
                enabled: false,
            })
            .unwrap();
        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot.dmx_preview[0] == 0
                && snapshot
                    .effects
                    .iter()
                    .any(|effect| effect.id == effect_id && !effect.enabled)
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert_eq!(snapshot.dmx_preview[0], 0);
        assert_eq!(snapshot.effects.len(), 1);
        assert!(!snapshot.effects[0].enabled);
    }

    #[test]
    fn move_effect_changes_stack_order_and_rendered_result() {
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: false,
            ..DmxOutputConfig::default()
        });
        let fixture_id = engine.allocate_fixture_id();
        engine
            .send(EngineCommand::PatchFixture {
                fixture_id,
                request: PatchFixtureRequest {
                    profile_path: "memory://fixture.gdtf".to_string(),
                    mode_name: Some("Standard".to_string()),
                    label: "Fixture 1".to_string(),
                    universe: 0,
                    address: 1,
                    group_ids: Vec::new(),
                    position: Vec3::default(),
                    rotation: Default::default(),
                },
                profile: sample_profile(),
            })
            .unwrap();

        let add_effect_id = engine.allocate_effect_id();
        let override_effect_id = engine.allocate_effect_id();
        for (effect_id, label, low, blend_mode) in [
            (add_effect_id, "Add 10000", 10_000, EffectBlendMode::Add),
            (
                override_effect_id,
                "Override 20000",
                20_000,
                EffectBlendMode::Override,
            ),
        ] {
            engine
                .send(EngineCommand::AddLfoEffect {
                    effect_id,
                    request: LfoEffectRequest {
                        label: label.to_string(),
                        fixture_ids: vec![fixture_id],
                        target_group_ids: Vec::new(),
                        attribute: "Dimmer".to_string(),
                        video_targets: Vec::new(),
                        shape: LfoShape::Sine,
                        period_ms: 10_000,
                        low,
                        high: low,
                        phase: 0.0,
                        blend_mode,
                    },
                })
                .unwrap();
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == (20_000 >> 8) as u8 && snapshot.effects.len() == 2 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert_eq!(snapshot.dmx_preview[0], (20_000 >> 8) as u8);
        assert_eq!(snapshot.effects[0].id, add_effect_id);
        assert_eq!(snapshot.effects[1].id, override_effect_id);

        engine
            .send(EngineCommand::MoveEffect {
                effect_id: override_effect_id,
                delta: -1,
            })
            .unwrap();

        for _ in 0..20 {
            snapshot = engine.snapshot();
            if snapshot.dmx_preview[0] == (30_000 >> 8) as u8
                && snapshot.effects[0].id == override_effect_id
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
        }
        assert_eq!(snapshot.effects[0].id, override_effect_id);
        assert_eq!(snapshot.effects[1].id, add_effect_id);
        assert_eq!(snapshot.dmx_preview[0], (30_000 >> 8) as u8);
    }

    #[test]
    fn lfo_effect_can_target_fixture_groups() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let grouped_id = engine.allocate_fixture_id();
        let ungrouped_id = engine.allocate_fixture_id();
        for (fixture_id, address, group_ids) in [
            (grouped_id, 1, vec!["front".to_string()]),
            (ungrouped_id, 10, Vec::new()),
        ] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids,
                        position: Vec3::default(),
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::AddLfoEffect {
                effect_id: engine.allocate_effect_id(),
                request: LfoEffectRequest {
                    label: "Front Pulse".to_string(),
                    fixture_ids: Vec::new(),
                    target_group_ids: vec!["front".to_string()],
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: LfoShape::Square,
                    period_ms: 10_000,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_group_only = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 255 && packet.data[9] == 0 {
                saw_group_only = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.fixtures.len() == 2 && snapshot.effects.len() == 1 {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_group_only);
        assert_eq!(snapshot.fixtures.len(), 2);
        assert_eq!(snapshot.effects.len(), 1);
        assert_eq!(snapshot.fixtures[0].group_ids, vec!["front"]);
        assert_eq!(snapshot.effects[0].target_group_ids, vec!["front"]);
    }

    #[test]
    fn position_wave_offsets_phase_by_fixture_position() {
        let receiver = UdpSocket::bind("127.0.0.1:0").unwrap();
        receiver
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        let port = receiver.local_addr().unwrap().port();
        let engine = EngineHandle::start(DmxOutputConfig {
            enabled: true,
            protocol: DmxOutputProtocol::ArtNet,
            target_ip: "127.0.0.1".to_string(),
            port,
            universe: 0,
            serial_port: String::new(),
            serial_baud_rate: 57_600,
        });
        let first_id = engine.allocate_fixture_id();
        let second_id = engine.allocate_fixture_id();
        for (fixture_id, address, x) in [(first_id, 1, 0.0), (second_id, 10, 1.0)] {
            engine
                .send(EngineCommand::PatchFixture {
                    fixture_id,
                    request: PatchFixtureRequest {
                        profile_path: "memory://fixture.gdtf".to_string(),
                        mode_name: Some("Standard".to_string()),
                        label: format!("Fixture {fixture_id}"),
                        universe: 0,
                        address,
                        group_ids: Vec::new(),
                        position: Vec3 { x, y: 0.0, z: 0.0 },
                        rotation: Default::default(),
                    },
                    profile: sample_profile(),
                })
                .unwrap();
        }
        engine
            .send(EngineCommand::AddPositionWaveEffect {
                effect_id: engine.allocate_effect_id(),
                request: PositionWaveEffectRequest {
                    label: "X Wave".to_string(),
                    fixture_ids: vec![first_id, second_id],
                    target_group_ids: Vec::new(),
                    attribute: "Dimmer".to_string(),
                    video_targets: Vec::new(),
                    shape: LfoShape::Square,
                    origin: Vec3::default(),
                    direction: Vec3 {
                        x: 1.0,
                        y: 0.0,
                        z: 0.0,
                    },
                    speed: 0.0,
                    wavelength: 2.0,
                    low: 0,
                    high: 65_535,
                    phase: 0.25,
                    blend_mode: EffectBlendMode::Override,
                },
            })
            .unwrap();

        let mut buffer = [0u8; 600];
        let mut saw_expected_wave = false;
        for _ in 0..20 {
            let (received, _) = receiver.recv_from(&mut buffer).unwrap();
            let packet = parse_art_dmx_packet(&buffer[..received]).unwrap();
            if packet.data[0] == 255 && packet.data[9] == 0 {
                saw_expected_wave = true;
                break;
            }
        }

        let mut snapshot = engine.snapshot();
        for _ in 0..20 {
            if snapshot.dmx_preview[0] == 255
                && snapshot.dmx_preview[9] == 0
                && !snapshot.effects.is_empty()
            {
                break;
            }
            std::thread::sleep(DMX_TICK_INTERVAL);
            snapshot = engine.snapshot();
        }
        assert!(saw_expected_wave);
        assert_eq!(snapshot.dmx_preview[0], 255);
        assert_eq!(snapshot.dmx_preview[9], 0);
        assert_eq!(snapshot.effects[0].effect_type, EffectKind::PositionWave);
    }

    #[test]
    fn evaluates_lfo_shapes() {
        assert!((evaluate_lfo_shape(&LfoShape::Sine, 0.25) - 1.0).abs() < 0.001);
        assert!((evaluate_lfo_shape(&LfoShape::Cosine, 0.0) - 1.0).abs() < 0.001);
        assert!((evaluate_lfo_shape(&LfoShape::Cosine, 0.5) - 0.0).abs() < 0.001);
        assert!((evaluate_lfo_shape(&LfoShape::Triangle, 0.25) - 0.5).abs() < 0.001);
        assert!((evaluate_lfo_shape(&LfoShape::Saw, 0.25) - 0.25).abs() < 0.001);
        assert_eq!(evaluate_lfo_shape(&LfoShape::Square, 0.25), 1.0);
        assert_eq!(
            evaluate_lfo_shape(&LfoShape::Random, 0.01),
            evaluate_lfo_shape(&LfoShape::Random, 0.02)
        );
        assert_ne!(
            evaluate_lfo_shape(&LfoShape::Random, 0.01),
            evaluate_lfo_shape(&LfoShape::Random, 0.20)
        );
        assert!((0.0..=1.0).contains(&evaluate_lfo_shape(&LfoShape::Perlin, 0.42)));
        assert!(
            (evaluate_lfo_shape(&LfoShape::Perlin, 0.0)
                - evaluate_lfo_shape(&LfoShape::Perlin, 1.0))
            .abs()
                < 0.001
        );
    }

    #[test]
    fn blends_effect_values() {
        assert_eq!(
            blend_effect_value(10_000, 20_000, &EffectBlendMode::Override),
            20_000
        );
        assert_eq!(
            blend_effect_value(60_000, 10_000, &EffectBlendMode::Add),
            65_535
        );
        assert_eq!(
            blend_effect_value(32_768, 32_768, &EffectBlendMode::Multiply),
            16_384
        );
    }

    #[test]
    fn projected_distance_uses_direction_or_radial_fallback() {
        let position = Vec3 {
            x: 3.0,
            y: 4.0,
            z: 0.0,
        };
        assert_eq!(
            projected_distance(
                position,
                Vec3::default(),
                Vec3 {
                    x: 1.0,
                    y: 0.0,
                    z: 0.0
                }
            ),
            3.0
        );
        assert_eq!(
            projected_distance(position, Vec3::default(), Vec3::default()),
            5.0
        );
    }

    #[test]
    fn bpm_clock_clamps_manual_bpm_and_tracks_phase() {
        let now = Instant::now();
        let mut clock = BpmClock::new(500.0, now);

        assert_eq!(clock.snapshot(now).bpm, 300.0);

        clock.set_bpm(60.0, now);
        let snapshot = clock.snapshot(now + Duration::from_millis(1500));

        assert_eq!(snapshot.bpm, 60.0);
        assert_eq!(snapshot.beat_counter, 1);
        assert!(snapshot.beat_phase > 0.49 && snapshot.beat_phase < 0.51);
    }

    #[test]
    fn bpm_clock_estimates_bpm_from_taps() {
        let now = Instant::now();
        let mut clock = BpmClock::new(120.0, now);

        clock.tap(now);
        clock.tap(now + Duration::from_millis(500));
        clock.tap(now + Duration::from_millis(1000));

        let snapshot = clock.snapshot(now + Duration::from_millis(1000));

        assert!((snapshot.bpm - 120.0).abs() < 0.01);
        assert_eq!(snapshot.tap_count, 3);
        assert_eq!(snapshot.source, ClockSource::Tap);
    }

    #[test]
    fn bpm_clock_estimates_bpm_from_midi_clock_pulses() {
        let now = Instant::now();
        let mut clock = BpmClock::new(90.0, now);

        for pulse in 0..24 {
            clock.midi_clock_pulse(now + Duration::from_secs_f32(pulse as f32 / 48.0));
        }

        let snapshot = clock.snapshot(now + Duration::from_millis(500));

        assert!((snapshot.bpm - 120.0).abs() < 0.05);
        assert_eq!(snapshot.source, ClockSource::MidiClock);
    }
}
