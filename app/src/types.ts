export type AttributeResolution = "EightBit" | "SixteenBit";

export interface ChannelFunctionSummary {
  name: string;
  attribute: string;
  dmx_from: number;
  dmx_to: number;
  physical_from?: number | null;
  physical_to?: number | null;
  wheel_slot?: string | null;
  wheel_slot_name?: string | null;
  wheel_slot_color?: string | null;
}

export interface AttributeControl {
  attribute: string;
  channel_name: string;
  geometry?: string | null;
  offsets: number[];
  resolution: AttributeResolution;
  default_value: number;
  functions?: ChannelFunctionSummary[];
}

export interface Vec3 {
  x: number;
  y: number;
  z: number;
}

export interface MidiInputSummary {
  index: number;
  name: string;
}

export interface MidiOutputSummary {
  index: number;
  name: string;
}

export type MidiControlMessage = "NoteOn" | "NoteOff" | "ControlChange" | "ProgramChange";
export type MidiControlAction =
  | "FixtureAttribute"
  | "TriggerCue"
  | "TriggerNextCue"
  | "TriggerPreviousCue"
  | "VideoParam"
  | "VideoCuePointAdd"
  | "VideoCuePointRemove"
  | "VideoCuePointJump"
  | "VideoLayerEnabled"
  | "VideoLayerSolo"
  | "VideoPlay"
  | "VideoLoop"
  | "VideoOutputEnabled"
  | "VideoOutputOpacity"
  | "VideoOutputFade"
  | "VideoOutputBlackout"
  | "TimelinePlay"
  | "TimelineSeek"
  | "LightingMaster"
  | "GroupSubmaster"
  | "CueFadePause"
  | "Blackout"
  | "VideoBlackout";

export interface MidiControlMapping {
  channel?: number | null;
  message: MidiControlMessage;
  number: number;
  action: MidiControlAction;
  fixture_id?: number | null;
  attribute?: string | null;
  group_id?: string | null;
  cue_id?: number | null;
  layer_id?: number | null;
  output_id?: number | null;
  video_param?: VideoParam | null;
  cue_point_index?: number | null;
  duration_ms?: number | null;
  low: number;
  high: number;
}

export interface LearnedMidiControl {
  channel: number;
  message: MidiControlMessage;
  number: number;
  value: number;
}

export interface SerialPortSummary {
  name: string;
  port_type: string;
}

export interface OscInputConfig {
  bind_ip: string;
  port: number;
}

export type OscControlAction =
  | "FixtureAttribute"
  | "TriggerCue"
  | "TriggerNextCue"
  | "TriggerPreviousCue"
  | "VideoParam"
  | "VideoCuePointAdd"
  | "VideoCuePointRemove"
  | "VideoCuePointJump"
  | "VideoLayerEnabled"
  | "VideoLayerSolo"
  | "VideoPlay"
  | "VideoLoop"
  | "VideoOutputEnabled"
  | "VideoOutputOpacity"
  | "VideoOutputFade"
  | "VideoOutputBlackout"
  | "TimelinePlay"
  | "TimelineSeek"
  | "LightingMaster"
  | "GroupSubmaster"
  | "CueFadePause"
  | "Blackout"
  | "VideoBlackout";

export interface OscControlMapping {
  address: string;
  action: OscControlAction;
  fixture_id?: number | null;
  attribute?: string | null;
  group_id?: string | null;
  cue_id?: number | null;
  layer_id?: number | null;
  output_id?: number | null;
  video_param?: VideoParam | null;
  cue_point_index?: number | null;
  duration_ms?: number | null;
  low: number;
  high: number;
}

export interface LearnedOscControl {
  address: string;
  value?: number | null;
  argument_count: number;
}

export interface RemoteControlConfig {
  bind_ip: string;
  port: number;
}

export type ClockSource = "Manual" | "Tap" | "MidiClock";
export type LfoShape = "Sine" | "Cosine" | "Triangle" | "Saw" | "Square" | "Random" | "Perlin";
export type EffectKind = "Lfo" | "PositionWave";
export type EffectBlendMode = "Override" | "Add" | "Multiply";

export interface DmxModeSummary {
  name: string;
  controls: AttributeControl[];
}

export interface GeometrySummary {
  name: string;
  kind: string;
  parent?: string | null;
  matrix: number[];
}

export interface FixtureProfileSummary {
  source_path: string;
  manufacturer: string;
  name: string;
  short_name?: string | null;
  fixture_type_id?: string | null;
  dmx_modes: DmxModeSummary[];
  geometries: GeometrySummary[];
  warnings: string[];
}

export interface PatchFixtureRequest {
  profile_path: string;
  mode_name?: string | null;
  label: string;
  universe: number;
  address: number;
  group_ids: string[];
  position: Vec3;
  rotation: { pitch: number; yaw: number; roll: number };
}

export interface FixtureLimits {
  dimmer_min: number;
  dimmer_max: number;
  pan_min: number;
  pan_max: number;
  tilt_min: number;
  tilt_max: number;
  invert_pan: boolean;
  invert_tilt: boolean;
  swap_pan_tilt: boolean;
}

export interface CustomFixtureProfileRequest {
  manufacturer: string;
  name: string;
  mode_name: string;
  attributes: string[];
}

export interface CustomFixtureProfileFile {
  version: number;
  request: CustomFixtureProfileRequest;
}

export interface AudioWaveformPoint {
  time_ms: number;
  peak: number;
  rms: number;
}

export interface AudioAnalysisSummary {
  path: string;
  sample_rate: number;
  channels: number;
  duration_ms: number;
  estimated_bpm?: number | null;
  waveform: AudioWaveformPoint[];
  beats: number[];
}

export interface AttributeValueSummary {
  attribute: string;
  value: number;
}

export interface PatchedFixtureSummary {
  id: number;
  label: string;
  profile_name: string;
  manufacturer: string;
  mode_name: string;
  universe: number;
  address: number;
  group_ids: string[];
  position: Vec3;
  rotation: { pitch: number; yaw: number; roll: number };
  controls: AttributeControl[];
  attribute_values: AttributeValueSummary[];
  limits: FixtureLimits;
  highlighted: boolean;
  soloed: boolean;
  parked: boolean;
}

export interface VisualizerFixtureNode {
  id: number;
  label: string;
  position: Vec3;
  yaw_deg: number;
  pitch_deg: number;
  roll_deg: number;
  color: [number, number, number];
  intensity: number;
}

export interface VisualizerBeamNode {
  fixture_id: number;
  origin: Vec3;
  direction: Vec3;
  length: number;
  radius: number;
  color: [number, number, number];
  intensity: number;
}

export interface VisualizerScene {
  fixtures: VisualizerFixtureNode[];
  beams: VisualizerBeamNode[];
  bounds: {
    min: Vec3;
    max: Vec3;
  };
}

export interface FixturePreset {
  version: number;
  manufacturer: string;
  profile_name: string;
  mode_name: string;
  values: AttributeValueSummary[];
}

export interface ProjectFile {
  version: number;
  app: string;
  snapshot: EngineSnapshot;
}

export interface CueFixtureTarget {
  fixture_id: number;
  values: AttributeValueSummary[];
}

export type VideoSourceKind = "File" | "Ndi" | "Spout" | "Syphon" | "StillImage";

export interface VideoSourceSummary {
  kind: VideoSourceKind;
  path?: string | null;
  name?: string | null;
  codec?: string | null;
  metadata?: VideoMediaMetadata | null;
}

export interface VideoMediaMetadata {
  duration_ms?: number | null;
  width?: number | null;
  height?: number | null;
  frame_rate?: number | null;
}

export type VideoBlendMode = "Normal" | "Add" | "Multiply" | "Screen";
export type VideoPixelFormat = "Rgba8" | "Bgra8" | "Dxt1" | "Dxt5";

export interface VideoFrame {
  layer_id: number;
  width: number;
  height: number;
  pts_ms: number;
  duration_ms: number;
  format: VideoPixelFormat;
  data: number[];
}

export interface VideoBpmSync {
  enabled: boolean;
  ratio: number;
  loop_bars: number;
}

export interface Transform2D {
  x: number;
  y: number;
  scale_x: number;
  scale_y: number;
  rotation_deg: number;
  crop_left: number;
  crop_top: number;
  crop_right: number;
  crop_bottom: number;
}

export interface VideoColorAdjust {
  brightness: number;
  contrast: number;
  hue_deg: number;
  saturation: number;
  gamma: number;
}

export interface VideoFxAdjust {
  pixelate: number;
  blur: number;
  glow: number;
  edge: number;
  key_red: number;
  key_green: number;
  key_blue: number;
  key_threshold: number;
}

export interface VideoLayerState {
  enabled: boolean;
  solo: boolean;
  opacity: number;
  speed: number;
  playing: boolean;
  position_ms: number;
  loop_enabled: boolean;
  loop_start_ms: number;
  loop_end_ms: number;
  bpm_sync: VideoBpmSync;
  cue_points_ms: number[];
  transform: Transform2D;
  color: VideoColorAdjust;
  fx: VideoFxAdjust;
}

export interface VideoLayerSummary {
  id: number;
  label: string;
  source: VideoSourceSummary;
  blend_mode: VideoBlendMode;
  state: VideoLayerState;
}

export interface VideoLayerTarget {
  layer_id: number;
  state: VideoLayerState;
}

export interface VideoOutputTarget {
  output_id: number;
  enabled: boolean;
  opacity: number;
  blackout: boolean;
}

export type VideoOutputKind = "Display" | "NdiSender" | "SpoutSender" | "SyphonServer";
export type VideoOutputAspectMode = "Stretch" | "Fit" | "Fill";

export interface VideoOutputMapping {
  offset_x: number;
  offset_y: number;
  scale_x: number;
  scale_y: number;
  rotation_deg: number;
  aspect_ratio: number;
  aspect_mode: VideoOutputAspectMode;
  lens_distortion: number;
  keystone_x: number;
  keystone_y: number;
  corner_top_left_x: number;
  corner_top_left_y: number;
  corner_top_right_x: number;
  corner_top_right_y: number;
  corner_bottom_right_x: number;
  corner_bottom_right_y: number;
  corner_bottom_left_x: number;
  corner_bottom_left_y: number;
}

export interface VideoOutputSummary {
  id: number;
  label: string;
  kind: VideoOutputKind;
  enabled: boolean;
  composition_id: number;
  fullscreen: boolean;
  monitor_id?: number | null;
  width: number;
  height: number;
  endpoint_name?: string | null;
  opacity: number;
  blackout: boolean;
  mapping: VideoOutputMapping;
}

export interface VideoOutputMappingPresetSummary {
  label: string;
  mapping: VideoOutputMapping;
}

export interface CompositionSummary {
  id: number;
  label: string;
  layer_ids: number[];
  output_ids: number[];
}

export interface VideoSnapshot {
  layers: VideoLayerSummary[];
  compositions: CompositionSummary[];
  outputs: VideoOutputSummary[];
  mapping_presets: VideoOutputMappingPresetSummary[];
  master_opacity: number;
  blackout: boolean;
}

export interface CueSummary {
  id: number;
  label: string;
  fade_ms: number;
  targets: CueFixtureTarget[];
  video_targets: VideoLayerTarget[];
  video_output_targets: VideoOutputTarget[];
}

export interface ActiveFadeSummary {
  cue_id: number;
  progress: number;
  remaining_ms: number;
  paused: boolean;
}

export type TimelineTrackKind = "Lighting" | "Video";

export interface TimelineCueEventSummary {
  id: number;
  cue_id: number;
  time_ms: number;
  track: TimelineTrackKind;
}

export type AutomationInterpolation = "Step" | "Linear" | "Bezier";

export interface AutomationKeyframeSummary {
  time_ms: number;
  value: number;
  interpolation: AutomationInterpolation;
}

export interface TimelineAutomationSummary {
  id: number;
  fixture_id: number;
  attribute: string;
  track: TimelineTrackKind;
  keyframes: AutomationKeyframeSummary[];
  enabled: boolean;
}

export type VideoParam =
  | "Opacity"
  | "Speed"
  | "PositionMs"
  | "BpmSyncEnabled"
  | "BpmSyncRatio"
  | "BpmSyncLoopBars"
  | "TransformX"
  | "TransformY"
  | "TransformScaleX"
  | "TransformScaleY"
  | "TransformRotationDeg"
  | "TransformCropLeft"
  | "TransformCropTop"
  | "TransformCropRight"
  | "TransformCropBottom"
  | "ColorBrightness"
  | "ColorContrast"
  | "ColorHueDeg"
  | "ColorSaturation"
  | "ColorGamma"
  | "FxPixelate"
  | "FxBlur"
  | "FxGlow"
  | "FxEdge"
  | "FxKeyRed"
  | "FxKeyGreen"
  | "FxKeyBlue"
  | "FxKeyThreshold";

export interface VideoAutomationKeyframeSummary {
  time_ms: number;
  value: number;
  interpolation: AutomationInterpolation;
}

export interface TimelineVideoAutomationSummary {
  id: number;
  layer_id: number;
  param: VideoParam;
  track: TimelineTrackKind;
  keyframes: VideoAutomationKeyframeSummary[];
  enabled: boolean;
}

export interface TimelineSnapshot {
  events: TimelineCueEventSummary[];
  automations: TimelineAutomationSummary[];
  video_automations: TimelineVideoAutomationSummary[];
  audio?: AudioAnalysisSummary | null;
  playing: boolean;
  position_ms: number;
  duration_ms: number;
}

export interface LfoEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
  shape: LfoShape;
  period_ms: number;
  low: number;
  high: number;
  phase: number;
  blend_mode: EffectBlendMode;
}

export interface PositionWaveEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
  shape: LfoShape;
  origin: { x: number; y: number; z: number };
  direction: { x: number; y: number; z: number };
  speed: number;
  wavelength: number;
  low: number;
  high: number;
  phase: number;
  blend_mode: EffectBlendMode;
}

export interface VideoEffectTarget {
  layer_ids: number[];
  param: VideoParam;
  low: number;
  high: number;
}

export interface EffectSummary {
  id: number;
  label: string;
  effect_type: EffectKind;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
  shape: LfoShape;
  period_ms?: number | null;
  low: number;
  high: number;
  phase: number;
  blend_mode: EffectBlendMode;
  origin?: { x: number; y: number; z: number } | null;
  direction?: { x: number; y: number; z: number } | null;
  speed?: number | null;
  wavelength?: number | null;
  enabled: boolean;
}

export interface EffectPreset {
  version: number;
  effect_type: EffectKind;
  enabled: boolean;
  lfo?: LfoEffectRequest | null;
  position_wave?: PositionWaveEffectRequest | null;
}

export type DmxOutputProtocol = "ArtNet" | "Sacn" | "EnttecUsbPro" | "EnttecOpenDmx";

export interface DmxOutputConfig {
  enabled: boolean;
  protocol: DmxOutputProtocol;
  target_ip: string;
  port: number;
  universe: number;
  serial_port: string;
  serial_baud_rate: number;
}

export interface SubmasterSummary {
  group_id: string;
  label: string;
  level: number;
}

export interface DmxUniversePreview {
  universe: number;
  values: number[];
}

export interface EngineSnapshot {
  fixtures: PatchedFixtureSummary[];
  cues: CueSummary[];
  active_cue_id?: number | null;
  active_fade?: ActiveFadeSummary | null;
  timeline: TimelineSnapshot;
  video: VideoSnapshot;
  effects: EffectSummary[];
  output: DmxOutputConfig;
  dmx_outputs: DmxOutputConfig[];
  lighting_master: number;
  submasters: SubmasterSummary[];
  blackout: boolean;
  clock: {
    bpm: number;
    beat_phase: number;
    beat_counter: number;
    tap_count: number;
    source: ClockSource;
  };
  dmx_preview: number[];
  dmx_previews: DmxUniversePreview[];
  telemetry: {
    frame_counter: number;
    queue_depth: number;
    last_tick_interval_us: number;
    tick_jitter_last_us: number;
    tick_jitter_abs_max_us: number;
    tick_jitter_stddev_us: number;
    tick_jitter_samples: number;
    last_packet_bytes: number;
    last_dmx_output_count: number;
    last_dmx_send_success_count: number;
    last_dmx_send_failure_count: number;
    total_dmx_send_success_count: number;
    total_dmx_send_failure_count: number;
    last_error?: string | null;
  };
}
