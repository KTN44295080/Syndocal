export type AttributeResolution = "EightBit" | "SixteenBit";

export interface ChannelFunctionSummary {
  name: string;
  attribute: string;
  parent_function?: string | null;
  dmx_from: number;
  dmx_to: number;
  physical_from?: number | null;
  physical_to?: number | null;
  wheel_slot?: string | null;
  wheel_slot_name?: string | null;
  wheel_slot_color?: string | null;
  wheel_slot_media?: string | null;
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
  | "FixtureHighlight"
  | "FixtureSolo"
  | "FixturePark"
  | "GroupHighlight"
  | "GroupSolo"
  | "GroupPark"
  | "TriggerCue"
  | "TriggerNextCue"
  | "TriggerPreviousCue"
  | "EffectEnabled"
  | "NodeGraphEnabled"
  | "VideoParam"
  | "VideoCuePointAdd"
  | "VideoCuePointRemove"
  | "VideoCuePointJump"
  | "VideoCuePointPrevious"
  | "VideoCuePointNext"
  | "VideoLayerEnabled"
  | "VideoLayerSolo"
  | "VideoPlay"
  | "VideoLoop"
  | "VideoLayerFade"
  | "VideoOutputEnabled"
  | "VideoOutputOpacity"
  | "VideoOutputFade"
  | "VideoOutputMappingField"
  | "VideoOutputMappingPreset"
  | "VideoOutputBlackout"
  | "TimelinePlay"
  | "TimelineSeek"
  | "TimelineBeatPrevious"
  | "TimelineBeatNext"
  | "LightingMaster"
  | "GroupSubmaster"
  | "SetBpm"
  | "TapBpm"
  | "CueFadePause"
  | "Blackout"
  | "AllBlackout"
  | "VideoBlackout"
  | "ClearFixtureFlags";

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
  | "FixtureHighlight"
  | "FixtureSolo"
  | "FixturePark"
  | "GroupHighlight"
  | "GroupSolo"
  | "GroupPark"
  | "TriggerCue"
  | "TriggerNextCue"
  | "TriggerPreviousCue"
  | "EffectEnabled"
  | "NodeGraphEnabled"
  | "VideoParam"
  | "VideoCuePointAdd"
  | "VideoCuePointRemove"
  | "VideoCuePointJump"
  | "VideoCuePointPrevious"
  | "VideoCuePointNext"
  | "VideoLayerEnabled"
  | "VideoLayerSolo"
  | "VideoPlay"
  | "VideoLoop"
  | "VideoLayerFade"
  | "VideoOutputEnabled"
  | "VideoOutputOpacity"
  | "VideoOutputFade"
  | "VideoOutputMappingField"
  | "VideoOutputMappingPreset"
  | "VideoOutputBlackout"
  | "TimelinePlay"
  | "TimelineSeek"
  | "TimelineBeatPrevious"
  | "TimelineBeatNext"
  | "LightingMaster"
  | "GroupSubmaster"
  | "SetBpm"
  | "TapBpm"
  | "CueFadePause"
  | "Blackout"
  | "AllBlackout"
  | "VideoBlackout"
  | "ClearFixtureFlags";

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

export type ClockSource = "Manual" | "Tap" | "MidiClock" | "MidiTimecode" | "Ltc" | "AbletonLink";
export type LfoShape = "Sine" | "Cosine" | "Triangle" | "Saw" | "Square" | "Random" | "Perlin";
export type EffectKind = "Lfo" | "PositionWave";
export type EffectBlendMode = "Override" | "Add" | "Multiply";
export type NodeGraphNodeKind = "Lfo" | "PositionWave" | "Transform" | "Output";
export type NodeGraphTransformOp = "Scale" | "Offset" | "Clamp" | "Invert" | "Abs";

export interface DmxModeSummary {
  name: string;
  controls: AttributeControl[];
}

export interface GeometrySummary {
  name: string;
  kind: string;
  parent?: string | null;
  matrix: number[];
  model_name?: string | null;
  model_file?: string | null;
  model_primitive?: string | null;
  model_dimensions?: Vec3 | null;
  beam_type?: string | null;
  beam_angle_deg?: number | null;
  field_angle_deg?: number | null;
  beam_radius?: number | null;
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

export interface GdtfShareModeSummary {
  name: string;
  dmx_footprint?: number | null;
}

export interface GdtfShareFixtureSummary {
  rid?: number | null;
  uuid?: string | null;
  manufacturer: string;
  fixture: string;
  revision: string;
  uploader?: string | null;
  rating?: string | null;
  version?: string | null;
  creator?: string | null;
  filesize?: number | null;
  modes: GdtfShareModeSummary[];
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
  profile_source_path: string;
  profile_name: string;
  manufacturer: string;
  mode_name: string;
  universe: number;
  address: number;
  group_ids: string[];
  position: Vec3;
  rotation: { pitch: number; yaw: number; roll: number };
  geometries: GeometrySummary[];
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
  geometry_name?: string | null;
  beam_type?: string | null;
  beam_angle_deg?: number | null;
  field_angle_deg?: number | null;
  origin: Vec3;
  direction: Vec3;
  length: number;
  radius: number;
  color: [number, number, number];
  intensity: number;
}

export type GeometryModelMeshKind = "None" | "Box" | "Cylinder" | "Sphere" | "Plane" | "Mesh" | "Unknown";
export type FixtureModelDrawKind = "BuiltInPrimitive" | "ExternalMesh" | "BoundsFallback";
export type VisualizerExternalModelAssetStatus = "Loaded" | "Missing" | "Skipped" | "Error";
export type VisualizerExternalModelAssetFormat = "Glb" | "Gltf" | "Obj" | "ThreeDs" | "Collada" | "Unknown";

export interface VisualizerFixtureGeometryNode {
  fixture_id: number;
  profile_source_path: string;
  name: string;
  kind: string;
  parent?: string | null;
  model_name?: string | null;
  model_file?: string | null;
  model_primitive?: string | null;
  model_mesh_kind: GeometryModelMeshKind;
  model_dimensions?: Vec3 | null;
  position: Vec3;
  local_position: Vec3;
  right: Vec3;
  up: Vec3;
  direction: Vec3;
  local_right: Vec3;
  local_up: Vec3;
  local_direction: Vec3;
  beam_type?: string | null;
  beam_angle_deg?: number | null;
  field_angle_deg?: number | null;
  beam_radius?: number | null;
  mapped_channel_count: number;
}

export interface VisualizerFixtureModelNode {
  fixture_id: number;
  profile_source_path: string;
  geometry_name: string;
  mesh_kind: GeometryModelMeshKind;
  model_name?: string | null;
  model_file?: string | null;
  model_primitive?: string | null;
  dimensions?: Vec3 | null;
  bounding_radius: number;
  position: Vec3;
  right: Vec3;
  up: Vec3;
  direction: Vec3;
}

export interface VisualizerFixtureModelRenderPlan {
  fixture_id: number;
  profile_source_path: string;
  geometry_name: string;
  draw_kind: FixtureModelDrawKind;
  fallback_mesh_kind: GeometryModelMeshKind;
  model_file?: string | null;
  dimensions?: Vec3 | null;
  bounding_radius: number;
  position: Vec3;
  right: Vec3;
  up: Vec3;
  direction: Vec3;
}

export interface VisualizerModelPrimitiveVertex {
  position: Vec3;
  normal: Vec3;
}

export interface VisualizerFixtureModelPrimitiveMesh {
  fixture_id: number;
  geometry_name: string;
  mesh_kind: GeometryModelMeshKind;
  vertices: VisualizerModelPrimitiveVertex[];
  indices: number[];
}

export interface VisualizerVideoSurfaceNode {
  id: number;
  label: string;
  kind: VideoOutputKind;
  composition_id: number;
  position: Vec3;
  width: number;
  height: number;
  rotation_deg: number;
  opacity: number;
  enabled: boolean;
  blackout: boolean;
}

export interface VisualizerScene {
  fixtures: VisualizerFixtureNode[];
  fixture_geometries: VisualizerFixtureGeometryNode[];
  fixture_models: VisualizerFixtureModelNode[];
  beams: VisualizerBeamNode[];
  video_surfaces: VisualizerVideoSurfaceNode[];
  bounds: {
    min: Vec3;
    max: Vec3;
  };
}

export interface VisualizerRenderPayload {
  scene: VisualizerScene;
  model_render_plans: VisualizerFixtureModelRenderPlan[];
  primitive_meshes: VisualizerFixtureModelPrimitiveMesh[];
}

export interface VisualizerExternalModelAsset {
  asset_key: string;
  profile_source_path: string;
  model_file: string;
  status: VisualizerExternalModelAssetStatus;
  format: VisualizerExternalModelAssetFormat;
  byte_length: number;
  meshes: VisualizerFixtureModelPrimitiveMesh[];
  error?: string | null;
  mesh_error?: string | null;
}

export interface VisualizerResolvedRenderPayload {
  scene: VisualizerScene;
  model_render_plans: VisualizerFixtureModelRenderPlan[];
  primitive_meshes: VisualizerFixtureModelPrimitiveMesh[];
  external_model_assets: VisualizerExternalModelAsset[];
  resolved_meshes: VisualizerFixtureModelPrimitiveMesh[];
}

export interface VisualizerModelAssetCacheSummary {
  entry_count: number;
  loaded_count: number;
  missing_count: number;
  byte_length: number;
  limit: number;
}

export interface FixturePreset {
  version: number;
  manufacturer: string;
  profile_name: string;
  profile_source_path?: string | null;
  mode_name: string;
  values: AttributeValueSummary[];
}

export interface NodeGraphLfoNode {
  shape: LfoShape;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  phase: number;
  amplitude: number;
  bias: number;
}

export interface NodeGraphPositionWaveNode {
  shape: LfoShape;
  origin: Vec3;
  direction: Vec3;
  speed: number;
  wavelength: number;
  clock_sync?: EffectClockSync | null;
  phase: number;
}

export interface NodeGraphTransformNode {
  op: NodeGraphTransformOp;
  amount: number;
  min: number;
  max: number;
}

export interface NodeGraphOutputNode {
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
  low: number;
  high: number;
  blend_mode: EffectBlendMode;
}

export interface NodeGraphNodeSummary {
  id: number;
  label: string;
  kind: NodeGraphNodeKind;
  x: number;
  y: number;
  lfo?: NodeGraphLfoNode | null;
  position_wave?: NodeGraphPositionWaveNode | null;
  transform?: NodeGraphTransformNode | null;
  output?: NodeGraphOutputNode | null;
}

export interface NodeGraphEdgeSummary {
  from_node: number;
  from_port: string;
  to_node: number;
  to_port: string;
}

export interface NodeGraphSummary {
  id: number;
  label: string;
  enabled: boolean;
  nodes: NodeGraphNodeSummary[];
  edges: NodeGraphEdgeSummary[];
}

export interface NodeGraphPresetFile {
  version: number;
  app: string;
  graph: NodeGraphSummary;
}

export interface ProjectFile {
  version: number;
  app: string;
  custom_profiles?: FixtureProfileSummary[];
  snapshot: EngineSnapshot;
}

export interface ProjectLoadResult {
  path: string;
  profiles: FixtureProfileSummary[];
}

export interface Phase1SmokeReport {
  path: string;
  cue_id: number;
  cue_label: string;
  active_cue_id?: number | null;
  first_8: number[];
  expected_first_8: number[];
  non_zero_first_8: number;
  passed: boolean;
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

export type VideoBackendState = "Available" | "Missing" | "NotBuilt";

export interface VideoBackendStatus {
  id: string;
  label: string;
  state: VideoBackendState;
  detail: string;
}

export interface VideoRuntimeStatus {
  backends: VideoBackendStatus[];
}

export interface VideoPreviewQueueSummary {
  layer_id: number;
  label: string;
  queue_len: number;
}

export interface VideoPreviewDiagnostics {
  queue_count: number;
  still_image_cache_len: number;
  decoder_cache_len: number;
  prefetch_count: number;
  prefetch_interval_ms: number;
  bpm?: number | null;
  layer_queues: VideoPreviewQueueSummary[];
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

export interface VideoCuePointSummary {
  position_ms: number;
  label: string;
  color?: string | null;
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
  cue_points: VideoCuePointSummary[];
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
  stage_x: number;
  stage_y: number;
  stage_z: number;
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
  node_graph_targets: CueNodeGraphTarget[];
}

export interface CueNodeGraphTarget {
  graph_id: number;
  enabled: boolean;
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
  clock_sync?: EffectClockSync | null;
  low: number;
  high: number;
  phase: number;
  blend_mode: EffectBlendMode;
}

export interface EffectClockSync {
  beats: number;
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
  clock_sync?: EffectClockSync | null;
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
  position?: Vec3 | null;
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
  clock_sync?: EffectClockSync | null;
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

export interface DmxTestFrameResult {
  protocol: DmxOutputProtocol;
  universe: number;
  channel: number;
  width: number;
  value: number;
  bytes: number;
}

export interface DmxOutputRouteTelemetry {
  index: number;
  universe: number;
  attempted: boolean;
  success: boolean;
  bytes: number;
  error?: string | null;
}

export type TelemetryBudgetStatus = "Pass" | "Warn" | "Fail" | "InsufficientSamples" | "Idle";

export interface TelemetryBudgetCheck {
  name: string;
  status: TelemetryBudgetStatus;
  measured_us?: number | null;
  target_us?: number | null;
  samples: number;
  detail: string;
}

export interface EngineTelemetryBudgetReport {
  overall: TelemetryBudgetStatus;
  target_dmx_frame_rate_hz: number;
  target_tick_interval_us: number;
  tick_jitter_p99_target_us: number;
  command_queue_p99_target_us: number;
  command_to_dmx_p99_target_us: number;
  dmx_send_interval_tolerance_us: number;
  checks: TelemetryBudgetCheck[];
}

export interface EngineTelemetryReport {
  version: number;
  captured_at_unix_ms: number;
  fixture_count: number;
  cue_count: number;
  effect_count: number;
  node_graph_count: number;
  video_layer_count: number;
  video_output_count: number;
  dmx_output_count: number;
  enabled_dmx_output_count: number;
  dmx_preview_universe_count: number;
  clock: EngineSnapshot["clock"];
  primary_output: DmxOutputConfig;
  dmx_outputs: DmxOutputConfig[];
  budget: EngineTelemetryBudgetReport;
  telemetry: EngineSnapshot["telemetry"];
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

export interface StageMapConfig {
  locked: boolean;
  min_x: number;
  max_x: number;
  min_z: number;
  max_z: number;
}

export interface StageMapPresetSummary {
  label: string;
  config: StageMapConfig;
}

export interface EngineSnapshot {
  fixtures: PatchedFixtureSummary[];
  cues: CueSummary[];
  active_cue_id?: number | null;
  active_fade?: ActiveFadeSummary | null;
  timeline: TimelineSnapshot;
  video: VideoSnapshot;
  effects: EffectSummary[];
  node_graphs: NodeGraphSummary[];
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
  stage_map: StageMapConfig;
  stage_map_presets: StageMapPresetSummary[];
  dmx_preview: number[];
  dmx_previews: DmxUniversePreview[];
  telemetry: {
    frame_counter: number;
    queue_depth: number;
    queue_depth_abs_max: number;
    queue_push_failure_count: number;
    last_tick_interval_us: number;
    tick_jitter_last_us: number;
    tick_jitter_abs_max_us: number;
    tick_jitter_stddev_us: number;
    tick_jitter_p95_us: number;
    tick_jitter_p99_us: number;
    tick_jitter_samples: number;
    last_command_queue_latency_us: number;
    command_queue_latency_abs_max_us: number;
    command_queue_latency_p95_us: number;
    command_queue_latency_p99_us: number;
    command_queue_latency_samples: number;
    last_command_drain_count: number;
    command_drain_abs_max: number;
    command_drain_limit_hit_count: number;
    last_command_to_dmx_tick_latency_us: number;
    command_to_dmx_tick_latency_abs_max_us: number;
    command_to_dmx_tick_latency_p95_us: number;
    command_to_dmx_tick_latency_p99_us: number;
    command_to_dmx_tick_latency_samples: number;
    last_dmx_send_interval_us: number;
    dmx_send_interval_min_us: number;
    dmx_send_interval_max_us: number;
    dmx_send_interval_samples: number;
    low_latency_dmx_tick_request_count: number;
    low_latency_dmx_tick_advance_count: number;
    low_latency_dmx_tick_defer_count: number;
    last_packet_bytes: number;
    last_dmx_output_count: number;
    last_dmx_send_success_count: number;
    last_dmx_send_failure_count: number;
    total_dmx_send_success_count: number;
    total_dmx_send_failure_count: number;
    last_dmx_route_results: DmxOutputRouteTelemetry[];
    last_error?: string | null;
  };
}
