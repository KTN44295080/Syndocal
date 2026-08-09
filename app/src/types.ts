export type AttributeResolution = "EightBit" | "SixteenBit";

export interface CieColorSummary {
  x: number;
  y: number;
  luminance: number;
}

export interface EmitterCalibrationSummary {
  name: string;
  color?: CieColorSummary | null;
  dominant_wavelength_nm?: number | null;
}

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
  emitter?: EmitterCalibrationSummary | null;
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

export interface MidiFeedbackMessage {
  message: MidiControlMessage;
  channel: number;
  number: number;
  value: number;
}

export interface MidiControlFeedback {
  off?: MidiFeedbackMessage | null;
  on?: MidiFeedbackMessage | null;
  unknown?: MidiFeedbackMessage | null;
}

export type MidiControlAction =
  | "FixtureAttribute"
  | "SelectedFeatureFader"
  | "FixtureHighlight"
  | "FixtureSolo"
  | "FixturePark"
  | "GroupHighlight"
  | "GroupSolo"
  | "GroupPark"
  | "TriggerCue"
  | "FlashCue"
  | "TriggerCueDirection"
  | "FlashCueDirection"
  | "TriggerCueListNext"
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
  | "VideoMaster"
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
  feedback?: MidiControlFeedback | null;
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
  usb_vid?: number | null;
  usb_pid?: number | null;
  serial_number?: string | null;
  manufacturer?: string | null;
  product?: string | null;
  recommended_protocol?: DmxOutputProtocol | null;
}

export interface OscInputConfig {
  bind_ip: string;
  port: number;
}

export type OscControlAction =
  | "FixtureAttribute"
  | "SelectedFeatureFader"
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
  | "VideoMaster"
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

export type DmxControlAction = OscControlAction;

export interface DmxControlMapping {
  universe: number;
  channel: number;
  action: DmxControlAction;
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

export interface LearnedDmxControl {
  universe: number;
  channel: number;
  value: number;
}

export interface LearnedOscControl {
  address: string;
  value?: number | null;
  argument_count: number;
}

export interface RemoteControlConfig {
  bind_ip: string;
  port: number;
  pairing_pin: string;
  allow_lan: boolean;
  max_connections: number;
  max_message_bytes: number;
  max_messages_per_second: number;
}

export interface RemoteClientSummary {
  id: number;
  peer_addr: string;
  connected_at_unix_ms: number;
  last_activity_unix_ms: number;
  messages_received: number;
}

export interface RemoteControlStatus {
  running: boolean;
  active_connections: number;
  rejected_connections: number;
  clients: RemoteClientSummary[];
}

export type ClockSource = "Manual" | "Tap" | "MidiClock" | "MidiTimecode" | "Ltc" | "AbletonLink";
export type LfoShape = "Sine" | "Cosine" | "Triangle" | "Saw" | "Square" | "Strobe" | "Random" | "Perlin";
export type EffectKind = "Lfo" | "PositionWave" | "Color" | "Chaser" | "Move" | "Value" | "Curve" | "Mapping" | "ColorMapping";
export type EffectBlendMode = "Override" | "Add" | "Multiply";
export type ColorEffectAlgorithm = "Cycle" | "Bounce" | "Sequence" | "Random";
export type ColorEffectInterpolation = "Rgb" | "HsvShortest" | "HsvLongest";
export type ChaserDirection = "Forward" | "Reverse" | "Bounce" | "BuildUpDown" | "Random";
export type MoveInterpolation = "Line" | "Smooth";
export type MoveCoordinateMode = "Absolute" | "Relative";
export type MoveDirection = "Forward" | "Reverse" | "Bounce";
export type ValueEffectInterpolation = "Step" | "Line" | "Smooth";
export type ValueEffectMode = "Absolute" | "Relative";
export type ValueEffectDirection = "Forward" | "Reverse" | "Bounce";
export type MappingEffectDirection = "Forward" | "Reverse" | "Bounce" | "Static";
export type NodeGraphNodeKind = "Lfo" | "PositionWave" | "Audio" | "Transform" | "Output";
export type AudioSpectrumBand = "Bass" | "Mid" | "High";
export type AudioSpectrumSource = "Timeline" | "Live";
export type AudioReactiveFeature =
  | "LegacyBand"
  | "Band"
  | "Rms"
  | "Peak"
  | "SpectralFlux"
  | "Onset"
  | "OnsetStrength"
  | "BeatPhase"
  | "Bpm"
  | "BpmConfidence"
  | "SpectralCentroid"
  | "SpectralDensitySlow"
  | "SpectralDensityFast"
  | "Kick"
  | "KickStrength"
  | "Snare"
  | "SnareStrength";
export type AudioReactiveCurve = "Linear" | "Smoothstep" | "Exponential" | "Logarithmic";
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

export interface AudioSpectrumPoint {
  time_ms: number;
  bass: number;
  mid: number;
  high: number;
}

export interface AudioAnalysisSummary {
  path: string;
  sample_rate: number;
  channels: number;
  duration_ms: number;
  estimated_bpm?: number | null;
  waveform: AudioWaveformPoint[];
  spectrum: AudioSpectrumPoint[];
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

export interface VisualizerStageObjectNode {
  id: number;
  label: string;
  kind: StageObjectKind;
  position: Vec3;
  width: number;
  depth: number;
  rotation_deg: number;
  color?: string | null;
}

export interface VisualizerScene {
  fixtures: VisualizerFixtureNode[];
  fixture_geometries: VisualizerFixtureGeometryNode[];
  fixture_models: VisualizerFixtureModelNode[];
  beams: VisualizerBeamNode[];
  video_surfaces: VisualizerVideoSurfaceNode[];
  stage_objects: VisualizerStageObjectNode[];
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

export interface FixturePresetGroupLoadResult {
  path: string;
  applied_count: number;
  skipped_count: number;
}

export interface TimelineGroupAutomationAddResult {
  automation_ids: number[];
  applied_count: number;
  skipped_count: number;
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
  audio?: NodeGraphAudioNode | null;
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
  audio_runtime?: NodeGraphAudioRuntimeStatus[];
}

export interface NodeGraphAudioRuntimeStatus {
  node_id: number;
  input_value: number;
  output_value: number;
  source_available: boolean;
  safety_zeroed: boolean;
  held: boolean;
  feature_sequence: number;
}

export interface NodeGraphPresetFile {
  version: number;
  app: string;
  graph: NodeGraphSummary;
}

export interface OperatorSelectionContext {
  fixture_ids: number[];
  attributes: string[];
}

export interface OperatorFeatureFaderResult {
  target_index: number;
  attribute: string;
  fixture_ids: number[];
  value: number;
}

export type OperatorLockMode = "Full" | "Partial";

export interface OperatorCredentialVerifier {
  scheme: "PBKDF2-SHA256";
  iterations: number;
  salt_b64: string;
  verifier_b64: string;
}

export interface OperatorPolicy {
  lock_mode: OperatorLockMode;
  lock_on_load: boolean;
  credential: OperatorCredentialVerifier;
}

export interface FixtureGroupSummary {
  id: string;
  label: string;
  color?: string | null;
}

export interface ProjectFile {
  version: number;
  app: string;
  operator_policy?: OperatorPolicy | null;
  custom_profiles?: FixtureProfileSummary[];
  fixture_groups?: FixtureGroupSummary[];
  midi_mappings?: MidiControlMapping[];
  osc_mappings?: OscControlMapping[];
  dmx_mappings?: DmxControlMapping[];
  snapshot: EngineSnapshot;
}

export interface ProjectLoadResult {
  path: string;
  profiles: FixtureProfileSummary[];
  midi_mappings: MidiControlMapping[];
  osc_mappings: OscControlMapping[];
  dmx_mappings: DmxControlMapping[];
  warnings: string[];
}

export interface DvcImportDetail {
  item: string;
  message: string;
}

export interface DvcImportCategory {
  count: number;
  details: DvcImportDetail[];
}

export interface DvcImportSummary {
  fixtures: number;
  profiles: number;
  fixture_groups: number;
  groups: number;
  cues: number;
  values_decoded: number;
  values_skipped: number;
  beam_records: number;
  beam_feature_checks: number;
  beam_feature_mismatches: number;
  timeline_audio_clips: number;
  timeline_scene_blocks: number;
  effects_converted: number;
  effects_skipped: number;
  unknown_channel_types: number;
  missing_audio_files: number;
}

export interface DvcImportReport {
  path: string;
  das_build: string;
  version_file: string;
  summary: DvcImportSummary;
  converted: DvcImportCategory;
  approximate: DvcImportCategory;
  skipped: DvcImportCategory;
  unsupported: DvcImportCategory;
  warnings: string[];
  midi_mappings: MidiControlMapping[];
  dmx_mappings: DmxControlMapping[];
}

export interface UserTemplateLoadResult extends ProjectLoadResult {
  label: string;
  midi_mappings: MidiControlMapping[];
  osc_mappings: OscControlMapping[];
  dmx_mappings: DmxControlMapping[];
}

export interface NodeGraphAudioNode {
  source?: AudioSpectrumSource;
  band: AudioSpectrumBand;
  feature: AudioReactiveFeature;
  band_index: number;
  gain: number;
  bias: number;
  attack_ms: number;
  release_ms: number;
  gate: number;
  curve: AudioReactiveCurve;
  invert: boolean;
  hold_ms: number;
}

export interface ProjectBackupSummary {
  id: number;
  created_at_unix_ms: number;
  source_path?: string | null;
  reason: string;
  bytes: number;
}

export interface ApplicationUpdateConfiguration {
  enabled: boolean;
  current_version: string;
  channel: string;
  endpoint_origin?: string | null;
  reason?: string | null;
}

export interface ApplicationUpdateCheck {
  available: boolean;
  current_version: string;
  channel: string;
  version?: string | null;
  date?: string | null;
  notes?: string | null;
}

export interface ApplicationUpdateProgress {
  phase: "downloading" | "verifying" | "verified";
  downloaded_bytes: number;
  total_bytes?: number | null;
}

export interface ProjectHistoryStatus {
  can_undo: boolean;
  can_redo: boolean;
  undo_depth: number;
  redo_depth: number;
  undo_label?: string | null;
  redo_label?: string | null;
}

export interface EngineSnapshotSyncResponse {
  revision: number;
  full?: EngineSnapshot | null;
  delta?: Partial<EngineSnapshot> | null;
}

export interface Phase1SmokeReport {
  path: string;
  cue_id: number;
  cue_label: string;
  active_cue_id?: number | null;
  timeline_event_count: number;
  timeline_automation_count: number;
  timeline_video_automation_count: number;
  timeline_duration_ms: number;
  timeline_probe_ms: number;
  timeline_probe_dimmer_byte: number;
  timeline_probe_video_opacity?: number | null;
  dmx_output_count: number;
  enabled_dmx_output_count: number;
  dmx_preview_universe_count: number;
  primary_output_label: string;
  video_layer_count: number;
  video_layer_label?: string | null;
  video_layer_playing: boolean;
  video_layer_opacity?: number | null;
  first_8: number[];
  expected_first_8: number[];
  non_zero_first_8: number;
  passed: boolean;
}

export interface CueFixtureTarget {
  fixture_id: number;
  values: AttributeValueSummary[];
}

export type VideoSourceKind = "File" | "Camera" | "ScreenCapture" | "Ndi" | "Spout" | "Syphon" | "StillImage";

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
  has_audio?: boolean;
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

export interface VideoAudioMonitorStatus {
  output_open: boolean;
  device_name?: string | null;
  active_layer_ids: number[];
  resync_count: number;
  last_drift_ms: number;
  max_abs_drift_ms: number;
  last_sync_error?: string | null;
}

export interface VideoRecordingStatus {
  active: boolean;
  output_id?: number | null;
  path?: string | null;
  width: number;
  height: number;
  frame_rate: number;
  frames_written: number;
  dropped_frames: number;
  audio_requested: boolean;
  audio_included: boolean;
  audio_track_count: number;
  started_unix_ms?: number | null;
  last_error?: string | null;
}

export interface LiveAudioInputStatus {
  running: boolean;
  stale: boolean;
  safety_clear_pending: boolean;
  device_id?: string | null;
  device_name?: string | null;
  backend?: string | null;
  sample_format?: string | null;
  sample_rate: number;
  channels: number;
  configured_buffer_frames?: number | null;
  applied_buffer_frames?: number | null;
  channel_mix: LiveAudioChannelMix;
  bass: number;
  mid: number;
  high: number;
  bands: number[];
  band_count: number;
  rms: number;
  peak: number;
  spectral_flux: number;
  spectral_centroid: number;
  spectral_density_fast: number;
  spectral_density_slow: number;
  kick_strength: number;
  snare_strength: number;
  kick_event: boolean;
  snare_event: boolean;
  onset: boolean;
  onset_strength: number;
  bpm?: number | null;
  bpm_confidence: number;
  beat_phase: number;
  feature_sequence: number;
  analyzed_windows: number;
  dropped_chunks: number;
  dropped_frames: number;
  backend_xruns: number;
  callback_count: number;
  last_callback_frames: number;
  min_callback_frames: number;
  max_callback_frames: number;
  capture_to_worker_us: number;
  max_capture_to_worker_us: number;
  queue_depth: number;
  queue_capacity: number;
  queue_depth_high_water: number;
  last_error?: string | null;
}

export type LiveAudioInputBackendId = "wasapi_shared" | "asio";

export interface LiveAudioInputBackendSummary {
  id: LiveAudioInputBackendId;
  label: string;
  built: boolean;
  requires_explicit_device: boolean;
  distribution: string;
}

export interface LiveAudioInputDeviceSummary {
  id: string;
  name: string;
  label: string;
  backend: string;
}

export type LiveAudioChannelMix =
  | { mode: "average_all" }
  | { mode: "single"; channel_index: number }
  | { mode: "stereo_pair"; left_channel_index: number; right_channel_index: number };

export interface LiveAudioInputConfig {
  channels: number;
  sample_rate: number;
  sample_format: string;
}

export type LiveAudioBufferCapability =
  | { kind: "range"; min_frames: number; max_frames: number }
  | { kind: "unknown" };

export interface LiveAudioInputConfigRange {
  channels: number;
  min_sample_rate: number;
  max_sample_rate: number;
  sample_format: string;
  buffer_size: LiveAudioBufferCapability;
}

export interface LiveAudioInputResolvedConfig {
  channels: number;
  sample_rate: number;
  sample_format: string;
  buffer_size: LiveAudioBufferCapability;
}

export interface LiveAudioInputCapabilities {
  device_id?: string | null;
  device_name: string;
  backend: string;
  default_config: LiveAudioInputConfig;
  supported_configs: LiveAudioInputConfigRange[];
  resolved_config: LiveAudioInputResolvedConfig;
  max_capture_frames: number;
}

export interface LiveAudioInputStartRequest {
  backend: LiveAudioInputBackendId;
  device_id?: string | null;
  sample_rate?: number | null;
  stream_channels?: number | null;
  sample_format?: string | null;
  buffer_frames?: number | null;
  channel_mix: LiveAudioChannelMix;
}

export interface VideoPreviewQueueSummary {
  layer_id: number;
  label: string;
  source_kind: VideoSourceKind;
  position_ms: number;
  source_duration_ms?: number | null;
  playing: boolean;
  effective_speed: number;
  queue_len: number;
  expected_queue_len: number;
  expected_positions_ms: number[];
  ready: boolean;
}

export interface VideoDecodeEnqueueReport {
  layers_considered: number;
  requests_attempted: number;
  inserted: number;
  duplicate: number;
  reprioritized: number;
  dropped_lower_priority: number;
  rejected_full: number;
  pending: number;
}

export interface VideoOutputDecodePreviewSummary {
  output_id: number;
  label: string;
  width: number;
  height: number;
  enabled: boolean;
  blackout: boolean;
  report?: VideoDecodeEnqueueReport | null;
  error?: string | null;
}

export interface VideoPreviewDiagnostics {
  queue_count: number;
  frame_queue_capacity: number;
  still_image_cache_len: number;
  decoder_cache_len: number;
  decoder_diagnostics: VideoDecoderDiagnostics;
  isf_pipeline_count: number;
  isf_last_stack_stage_count: number;
  isf_last_stack_render_us: number;
  last_isf_error?: string | null;
  isf_stage_errors: VideoIsfStageError[];
  prefetch_count: number;
  prefetch_interval_ms: number;
  bpm?: number | null;
  layer_queues: VideoPreviewQueueSummary[];
  output_decode_previews: VideoOutputDecodePreviewSummary[];
}

export interface VideoDecoderDiagnostics {
  total_requests: number;
  hap_requests: number;
  hap_successes: number;
  hap_failures: number;
  libav_requests: number;
  libav_successes: number;
  libav_failures: number;
  cli_fallback_requests: number;
  cli_fallback_successes: number;
  cli_fallback_failures: number;
  deferred_requests: number;
  decode_failures: number;
  hap_cache_len: number;
  libav_cache_len: number;
  libav_session_count: number;
  libav_session_open_count: number;
  libav_session_reset_count: number;
  libav_sequential_continue_count: number;
  libav_frame_reuse_count: number;
  libav_working_set_eviction_count: number;
  libav_session_error_count: number;
  cli_cache_len: number;
}

export interface ExternalVideoInputPlan {
  layer_id: number;
  label: string;
  kind: VideoSourceKind;
  backend_id: string;
  backend_label?: string | null;
  backend_state?: VideoBackendState | null;
  backend_detail?: string | null;
  endpoint_name: string;
  enabled: boolean;
  ready: boolean;
  live: boolean;
  issue?: string | null;
}

export interface ExternalVideoOutputPlan {
  output_id: number;
  label: string;
  kind: VideoOutputKind;
  backend_id: string;
  backend_label?: string | null;
  backend_state?: VideoBackendState | null;
  backend_detail?: string | null;
  endpoint_name: string;
  enabled: boolean;
  width: number;
  height: number;
  opacity: number;
  blackout: boolean;
  composition_id: number;
  ready: boolean;
  live: boolean;
  issue?: string | null;
}

export interface ExternalVideoIoPlans {
  inputs: ExternalVideoInputPlan[];
  outputs: ExternalVideoOutputPlan[];
}

export type ExternalVideoTransportDirection = "Input" | "Output";

export interface ExternalVideoTransportRoute {
  direction: ExternalVideoTransportDirection;
  route_id: number;
  label: string;
  backend_id: string;
  endpoint_name: string;
}

export interface ExternalVideoTransportBlockedRoute {
  route: ExternalVideoTransportRoute;
  issue: string;
}

export type ExternalVideoTransportDriverAction = "Start" | "Stop";

export interface ExternalVideoTransportDriverEvent {
  sequence: number;
  action: ExternalVideoTransportDriverAction;
  route: ExternalVideoTransportRoute;
  message: string;
}

export interface ExternalVideoTransportSyncReport {
  started: ExternalVideoTransportRoute[];
  kept: ExternalVideoTransportRoute[];
  stopped: ExternalVideoTransportRoute[];
  blocked: ExternalVideoTransportBlockedRoute[];
  start_failed: ExternalVideoTransportBlockedRoute[];
  stop_failed: ExternalVideoTransportBlockedRoute[];
  idle: ExternalVideoTransportRoute[];
  active_count: number;
}

export interface ExternalVideoTransportStatus {
  active_routes: ExternalVideoTransportRoute[];
  active_count: number;
}

export interface ExternalVideoTransportSyncResponse {
  report: ExternalVideoTransportSyncReport;
  events: ExternalVideoTransportDriverEvent[];
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

export type VideoIsfControlKind = "Event" | "Bool" | "Long" | "Float" | "Point2d" | "Color";

export interface VideoIsfControlSummary {
  name: string;
  kind: VideoIsfControlKind;
  value: [number, number, number, number];
  default: [number, number, number, number];
  minimum: [number, number, number, number];
  maximum: [number, number, number, number];
  labels: string[];
  values: number[];
}

export interface VideoIsfEffectStageSummary {
  enabled: boolean;
  label: string;
  source: string;
  source_path?: string | null;
  description?: string | null;
  categories: string[];
  controls: VideoIsfControlSummary[];
}

export interface VideoIsfEffectSummary extends VideoIsfEffectStageSummary {
  stack?: VideoIsfEffectStageSummary[];
}

export interface VideoIsfStageError {
  layer_id: number;
  stage_index?: number | null;
  stage_label?: string | null;
  message: string;
}

export interface VideoLayerSummary {
  id: number;
  label: string;
  source: VideoSourceSummary;
  blend_mode: VideoBlendMode;
  state: VideoLayerState;
  isf_effect?: VideoIsfEffectSummary | null;
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

export interface VideoMaskPoint {
  x: number;
  y: number;
}

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
  edge_blend_left: number;
  edge_blend_right: number;
  edge_blend_top: number;
  edge_blend_bottom: number;
  edge_blend_gamma: number;
  black_level: number;
  mask_point_count: number;
  mask_invert: boolean;
  mask_softness: number;
  mask_points: VideoMaskPoint[];
  bitmap_mask_width: number;
  bitmap_mask_height: number;
  bitmap_mask_luma_words: number[];
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

export interface VideoBitmapMaskImportResult {
  width: number;
  height: number;
  luma_words: number[];
  source_name: string;
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

export interface CompositionLayerPlan {
  layer_id: number;
  label: string;
  source: VideoSourceSummary;
  blend_mode: VideoBlendMode;
  opacity: number;
  position_ms: number;
  transform: Transform2D;
  color: VideoColorAdjust;
  fx: VideoFxAdjust;
}

export interface CompositionPlan {
  composition_id: number;
  label: string;
  output_ids: number[];
  master_opacity: number;
  blackout: boolean;
  layers: CompositionLayerPlan[];
}

export interface VideoOutputRenderPlan {
  output_id: number;
  label: string;
  kind: VideoOutputKind;
  enabled: boolean;
  width: number;
  height: number;
  fullscreen: boolean;
  monitor_id?: number | null;
  endpoint_name?: string | null;
  output_opacity: number;
  output_blackout: boolean;
  mapping: VideoOutputMapping;
  composition: CompositionPlan;
}

export interface VideoOutputWindowStatus {
  output_id: number;
  label: string;
  live_open: boolean;
  test_pattern_open: boolean;
  live_window_label: string;
  test_pattern_window_label: string;
  performance?: NativeVideoOutputPerformance | null;
}

export interface NativeVideoOutputPerformance {
  frame_count: number;
  average_frame_us: number;
  last_frame_us: number;
  max_frame_us: number;
  deadline_miss_count: number;
  frame_budget_pass?: boolean | null;
  width: number;
  height: number;
  output_capacity_bytes: number;
  layer_slots: number;
  output_reallocations: number;
  layer_reallocations: number;
  compressed_layer_uploads: number;
  decoder_diagnostics: VideoDecoderDiagnostics;
  last_error?: string | null;
  warmup_remaining: number;
}

export interface VideoOutputWindowSyncSummary {
  synced_live: number;
  synced_test_pattern: number;
  skipped_closed: number;
}

export interface VideoOutputWindowCloseSummary {
  closed_live: number;
  closed_test_pattern: number;
  skipped_closed: number;
}

export interface VideoSnapshot {
  layers: VideoLayerSummary[];
  compositions: CompositionSummary[];
  outputs: VideoOutputSummary[];
  mapping_presets: VideoOutputMappingPresetSummary[];
  master_opacity: number;
  blackout: boolean;
  auto_vj?: AutoVjSnapshot;
}

export interface AutoVjConfig {
  eligible_layer_ids: number[];
  seed: number;
  beats_per_change: number;
  transition_ms: number;
  avoid_immediate_repeat: boolean;
  rhythm_source: AutoVjRhythmSource;
}

export type AutoVjRhythmSource = "Clock" | "LiveAudio";

export type AutoVjMode = "Off" | "Armed" | "Running" | "Hold" | "Fault";

export interface AutoVjAction {
  sequence: number;
  boundary_index: number;
  beat: number;
  layer_id: number;
  transition_ms: number;
  selection_token: number;
  seed: number;
  show_revision: number;
  trigger: "ClockBoundary" | "LiveAudioOnset";
  live_audio_feature_sequence?: number | null;
}

export interface AutoVjStatus {
  mode: AutoVjMode;
  armed: boolean;
  hold: boolean;
  show_revision: number;
  action_sequence: number;
  last_consumed_boundary?: number | null;
  next_boundary_beat?: number | null;
  last_action?: AutoVjAction | null;
  action_log: AutoVjAction[];
  fault?: string | null;
  live_audio_beat_counter: number;
  last_live_audio_feature_sequence?: number | null;
}

export interface AutoVjSnapshot {
  config: AutoVjConfig;
  status: AutoVjStatus;
}

/**
 * Runtime-only state for the VJ preview deck. This is deliberately kept out of
 * VideoSnapshot so preview transport never becomes project history or changes
 * the Program bus.
 */
export interface VjPreviewTransportSummary {
  layer_id: number | null;
  playing: boolean;
  position_ms: number;
  duration_ms: number | null;
  speed: number;
  loop_enabled: boolean;
  loop_start_ms: number;
  loop_end_ms: number;
  updated_at_ms: number;
  generation: number;
  source_name?: string | null;
}

export interface CueSummary {
  id: number;
  cue_list_id: number;
  cue_number: string;
  label: string;
  group_id?: string | null;
  recall_mode?: RecallMode;
  fade_ms: number;
  /** Intrinsic musical length used when a placed Scene Block conforms to tempo. */
  authored_beats?: number | null;
  pre_wait_ms: number;
  follow_ms?: number | null;
  ifcb_timing: CueIfcbTiming;
  parts: CuePartSummary[];
  mark: boolean;
  mib_fixture_ids: number[];
  palette_targets: CuePaletteTarget[];
  tracking: boolean;
  notes: string;
  targets: CueFixtureTarget[];
  video_targets: VideoLayerTarget[];
  video_output_targets: VideoOutputTarget[];
  node_graph_targets: CueNodeGraphTarget[];
  effect_targets: CueEffectTarget[];
  steps?: CueStepSummary[];
  child_timeline?: ChildTimelineSummary | null;
  /** Persistent identity color (#rrggbb); absent inherits the group identity before cue hash fallback. */
  color?: string | null;
  /** T17 authored live-modifier dial defaults; absent means neutral, no flash. */
  live_modifiers?: CueLiveModifierSettings | null;
}

export type CueLiveDirection = "Authored" | "Forward" | "Reverse" | "Bounce";

/** T17/T20 authored starting position of the per-scene live controls. */
export interface CueLiveModifierSettings {
  speed: number;
  size: number;
  phase: number;
  direction?: CueLiveDirection;
  /** One-based Cue Step segment; 0 follows normal playback. */
  segment?: number;
  flash: boolean;
}

/** T17 runtime-only latched live override for one active scene. */
export interface CueLiveModifierState {
  cue_id: number;
  speed: number;
  size: number;
  phase: number;
  direction?: CueLiveDirection;
  segment?: number;
}

export interface CueStepSummary {
  values: CueFixtureTarget[];
  fade_ms: number;
  hold_ms: number;
}

export interface CueNodeGraphTarget {
  graph_id: number;
  enabled: boolean;
}

export type RecallMode = "Coexist" | "ReplaceGroup";

export type EffectParamsSnapshot =
  | { Lfo: LfoEffectRequest }
  | { PositionWave: PositionWaveEffectRequest }
  | { Color: ColorEffectRequest }
  | { Chaser: ChaserEffectRequest }
  | { Move: MoveEffectRequest }
  | { Value: ValueEffectRequest }
  | { Curve: CurveEffectRequest }
  | { Mapping: MappingEffectRequest }
  | { ColorMapping: ColorMappingEffectRequest };

export interface CueEffectTarget {
  effect_id: number;
  enabled: boolean;
  params?: EffectParamsSnapshot | null;
  /** Optional cross-Cue Effect fade; absent preserves instant legacy recall. */
  transition_ms?: number | null;
}

export interface ActiveFadeSummary {
  cue_id: number;
  progress: number;
  remaining_ms: number;
  paused: boolean;
}

export type TimelineTrackKind = "Lighting" | "Video";

export type TimelineLayerKind = "Lighting" | "Video" | "Audio";

export interface TimelineLayerSummary {
  id: number;
  label: string;
  order: number;
  muted: boolean;
  locked: boolean;
  solo: boolean;
  expanded?: boolean;
  kind: TimelineLayerKind;
}

export interface TimelineAudioClipSummary {
  id: number;
  layer_id: number;
  path: string;
  start_ms: number;
  offset_ms: number;
  duration_ms: number;
  gain: number;
  fade_in_ms: number;
  fade_out_ms: number;
}

export interface TimelineCueEventSummary {
  id: number;
  cue_id: number;
  time_ms: number;
  /** Beat-domain placement intent; time_ms remains playback truth. */
  time_beats?: number | null;
  track: TimelineTrackKind;
  layer_id?: number | null;
  /** Legacy iteration length; for conformed blocks this is the fixed playback window. Zero preserves a point event. */
  duration_ms: number;
  /** Beat-domain block-window intent used by tempo reconform. */
  duration_beats?: number | null;
  /** Rewrites placement/window milliseconds from beat intent on BPM changes. */
  conform_to_tempo?: boolean;
  /** Keeps duration_ms as the block window and derives retrigger count to fill it. */
  loop_fill?: boolean;
  /** Cue-content phase offset at Scene Block activation; omitted legacy values default to zero. */
  source_offset_ms?: number;
  /** Derived display/cadence multiplier; recomputed by the engine on load. */
  rate?: number | null;
  /** Scene Block fade-in override, clamped to the block window. */
  fade_in_ms?: number;
  /** Scene Block release fade, clamped to the block window. */
  fade_out_ms?: number;
  /** Number of Cue iterations; derived by the engine when loop_fill is enabled. */
  loop_count: number;
  /** Optional placement to seek to after the final iteration completes. */
  jump_to_event_id: number | null;
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

export interface ChildTimelineSummary {
  layers?: TimelineLayerSummary[];
  events?: TimelineCueEventSummary[];
  automations?: TimelineAutomationSummary[];
  video_automations?: TimelineVideoAutomationSummary[];
  audio?: AudioAnalysisSummary | null;
  audio_clips?: TimelineAudioClipSummary[];
  /** Scale this Timeline scene's authored millisecond grid to the owning Scene duration in beats. */
  tempo_driven?: boolean;
  metronome_enabled?: boolean;
  count_in_beats?: number;
  duration_ms?: number;
}

export interface TimelineSnapshot {
  layers?: TimelineLayerSummary[];
  events: TimelineCueEventSummary[];
  automations: TimelineAutomationSummary[];
  video_automations: TimelineVideoAutomationSummary[];
  audio?: AudioAnalysisSummary | null;
  audio_clips?: TimelineAudioClipSummary[];
  audio_offset_ms?: number;
  audio_muted?: boolean;
  metronome_enabled?: boolean;
  count_in_beats?: number;
  count_in_remaining_ms?: number;
  playing: boolean;
  position_ms: number;
  duration_ms: number;
}

export interface DaslightCurveSource {
  rate: number;
  size: number;
  offset: number;
  sample_ms: number;
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
  fixture_spread?: number;
  beam_targets?: EffectBeamTarget[];
  blend_mode: EffectBlendMode;
  daslight_curve?: DaslightCurveSource | null;
}

export interface EffectClockSync {
  beats: number;
}

export interface ColorEffectColor {
  red: number;
  green: number;
  blue: number;
}

export interface ColorEffectStop {
  position: number;
  color: ColorEffectColor;
}

export interface ColorEffectBeamTarget {
  fixture_id: number;
  beam_index: number;
  selection_index: number;
  feature_attribute?: string | null;
}

export type ColorEffectSpatialRecipe =
  | { KnightRider: { size: number; one_way: boolean; fading: boolean; go_outside: boolean; gradient: number } }
  | { Sweep: { direction_change: boolean } }
  | { Burst: { color_width: number; gradient: number } }
  | { RandomFill: { point_width: number } }
  | { Sparkle: { number: number; lifespan: number; width: number } }
  | { Plasma: { grayscale: boolean; vertical_symmetry: boolean; size_x: number; param_x: number; size_y: number; param_y: number; speed_x: number; param_sx: number; speed_y: number; param_sy: number } }
  | { ColorRainbow: { grayscale: boolean; vertical_symmetry: boolean; color_width: number; angle_degrees: number; gradient: number } }
  | { Rainbow: { vertical_symmetry: boolean; horizontal_symmetry: boolean; rotation_degrees: number; color_width: number; angle_degrees: number; gradient: number } }
  | { Perlin: { octaves: number; zoom: number; direction_degrees: number; speed: number; amplitude: number } };

export interface ColorEffectSpatialPattern {
  recipe: ColorEffectSpatialRecipe;
  beam_targets?: ColorEffectBeamTarget[];
}

export interface ColorEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  stops: ColorEffectStop[];
  algorithm: ColorEffectAlgorithm;
  interpolation: ColorEffectInterpolation;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  phase: number;
  fixture_spread: number;
  blend_mode: EffectBlendMode;
  spatial_pattern?: ColorEffectSpatialPattern | null;
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

export interface EffectBeamTarget {
  fixture_id: number;
  beam_index: number;
  selection_index: number;
  feature_attribute: string;
}

export interface ChaserStep {
  fixture_ids: number[];
  target_group_ids: string[];
  beam_targets?: EffectBeamTarget[];
  level: number;
}

export interface ChaserFeature {
  attribute: string;
  low: number;
  high: number;
}

export interface ChaserEffectRequest {
  label: string;
  steps: ChaserStep[];
  features: ChaserFeature[];
  step_duration_ms: number;
  clock_sync?: EffectClockSync | null;
  direction: ChaserDirection;
  wings: number;
  active_step_count: number;
  duty_cycle: number;
  overlap: number;
  phase: number;
  fixture_spread: number;
  random_seed: number;
  random_cycle_count?: number;
  blend_mode: EffectBlendMode;
}

export interface MovePathPoint {
  x: number;
  y: number;
}

export interface MoveEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  points: MovePathPoint[];
  closed: boolean;
  interpolation: MoveInterpolation;
  coordinate_mode: MoveCoordinateMode;
  center_x: number;
  center_y: number;
  size_x: number;
  size_y: number;
  rotation_degrees: number;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  direction: MoveDirection;
  phase: number;
  fixture_spread: number;
  /** Daslight-compatible Pan mirror for the second half of the resolved fixture order. */
  symmetry?: boolean;
  blend_mode: EffectBlendMode;
}

export interface ValueEffectPoint {
  position: number;
  value: number;
}

export interface ValueEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  features?: ChaserFeature[];
  points: ValueEffectPoint[];
  spatial_pattern?: ColorEffectSpatialPattern | null;
  interpolation: ValueEffectInterpolation;
  mode: ValueEffectMode;
  direction: ValueEffectDirection;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  low: number;
  high: number;
  phase: number;
  fixture_spread: number;
  blend_mode: EffectBlendMode;
}

export interface CurveEffectPoint {
  position: number;
  value: number;
  in_tangent: number;
  out_tangent: number;
}

export interface CurveEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  features?: ChaserFeature[];
  points: CurveEffectPoint[];
  mode: ValueEffectMode;
  direction: ValueEffectDirection;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  low: number;
  high: number;
  phase: number;
  fixture_spread: number;
  blend_mode: EffectBlendMode;
}

export interface MappingEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  features?: ChaserFeature[];
  shape: LfoShape;
  mode: ValueEffectMode;
  direction: MappingEffectDirection;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  low: number;
  high: number;
  phase: number;
  fixture_spread: number;
  repetitions: number;
  blend_mode: EffectBlendMode;
}

export type ColorMappingSourceKind = "Image" | "Text" | "Video";
export type ColorMappingPlaybackDirection = "Forward" | "Reverse" | "Bounce";
export type ColorMappingWrapMode = "Clamp" | "Repeat" | "Mirror";
export type ColorMappingSampling = "Nearest" | "Bilinear";

export interface ColorMappingCellTarget {
  fixture_id: number;
  beam_index: number;
  selection_index: number;
  u: number;
  v: number;
  feature_attribute?: string | null;
  feature_low?: number | null;
  feature_high?: number | null;
}

export interface ColorMappingFrame {
  /** Packed RGB16 in an exact JavaScript integer: 0xRRRRGGGGBBBB. */
  pixels: number[];
}

export interface ColorMappingEffectRequest {
  label: string;
  fixture_ids: number[];
  target_group_ids: string[];
  source_kind: ColorMappingSourceKind;
  width: number;
  height: number;
  frames: ColorMappingFrame[];
  cells?: ColorMappingCellTarget[];
  playback_direction: ColorMappingPlaybackDirection;
  period_ms: number;
  clock_sync?: EffectClockSync | null;
  phase: number;
  offset_u: number;
  offset_v: number;
  scale_u: number;
  scale_v: number;
  rotation_degrees: number;
  wrap_mode: ColorMappingWrapMode;
  sampling: ColorMappingSampling;
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
  fixture_spread?: number;
  blend_mode: EffectBlendMode;
  origin?: { x: number; y: number; z: number } | null;
  direction?: { x: number; y: number; z: number } | null;
  speed?: number | null;
  wavelength?: number | null;
  enabled: boolean;
  /** Present for Cue-owned FX synthesized from the owning Cue target. */
  params?: EffectParamsSnapshot | null;
  lfo?: LfoEffectRequest | null;
  color?: ColorEffectRequest | null;
  chaser?: ChaserEffectRequest | null;
  move_effect?: MoveEffectRequest | null;
  value?: ValueEffectRequest | null;
  curve?: CurveEffectRequest | null;
  mapping?: MappingEffectRequest | null;
  color_mapping?: ColorMappingEffectRequest | null;
}

export interface EffectPreset {
  version: number;
  effect_type: EffectKind;
  enabled: boolean;
  lfo?: LfoEffectRequest | null;
  position_wave?: PositionWaveEffectRequest | null;
  color?: ColorEffectRequest | null;
  chaser?: ChaserEffectRequest | null;
  move_effect?: MoveEffectRequest | null;
  value?: ValueEffectRequest | null;
  curve?: CurveEffectRequest | null;
  mapping?: MappingEffectRequest | null;
  color_mapping?: ColorMappingEffectRequest | null;
}

export type DmxOutputProtocol =
  | "ArtNet"
  | "Sacn"
  | "EnttecUsbPro"
  | "DmxKingUltraDmx"
  | "EnttecOpenDmx";

export interface DmxOutputConfig {
  enabled: boolean;
  protocol: DmxOutputProtocol;
  target_ip: string;
  port: number;
  universe: number;
  serial_port: string;
  serial_baud_rate: number;
}

export interface ProgrammerValueSummary {
  fixture_id: number;
  attribute: string;
  value: number;
}

export interface ProgrammerSnapshot {
  enabled: boolean;
  blind: boolean;
  values: ProgrammerValueSummary[];
  dmx_previews: DmxUniversePreview[];
}

export interface CuePartSummary {
  number: number;
  label: string;
  delay_ms: number;
  fade_ms?: number | null;
  fixture_ids: number[];
  video_layer_ids: number[];
  video_output_ids: number[];
}

export interface CueListSummary {
  id: number;
  label: string;
  active_cue_id?: number | null;
}

export type PaletteKind = "Intensity" | "Position" | "Color" | "Beam" | "All";

export interface ReferencePaletteSummary {
  id: number;
  label: string;
  kind: PaletteKind;
  values: AttributeValueSummary[];
  color_stops?: ColorEffectStop[];
}

export interface PlaybackExecutorSummary {
  id: number;
  label: string;
  cue_list_id: number;
  page: number;
  slot: number;
  level: number;
}

export interface CuePaletteTarget {
  palette_id: number;
  fixture_ids: number[];
}

export interface CueIfcbTiming {
  intensity_fade_ms?: number | null;
  intensity_delay_ms: number;
  focus_fade_ms?: number | null;
  focus_delay_ms: number;
  color_fade_ms?: number | null;
  color_delay_ms: number;
  beam_fade_ms?: number | null;
  beam_delay_ms: number;
}

export type DmxInputProtocol = "ArtNet" | "Sacn";
export type DmxMergeMode = "Htp" | "Ltp";

export interface DmxInputConfig {
  protocol: DmxInputProtocol;
  bind_ip: string;
  port: number;
  universe: number;
  merge_enabled: boolean;
  merge_mode: DmxMergeMode;
  timeout_ms: number;
}

export interface DmxInputStatus {
  running: boolean;
  signal_present: boolean;
  packets_received: number;
  invalid_packets: number;
  last_packet_unix_ms?: number | null;
  source_address?: string | null;
}

export interface ArtRdmRequest {
  gateway_ip: string;
  port_address: number;
  source_uid: string;
  target_uid: string;
  command: "Get" | "Set";
  parameter_id: number;
  parameter_data_hex: string;
  timeout_ms: number;
}

export interface UsbRdmRequest {
  serial_port: string;
  serial_baud_rate: number;
  source_uid: string;
  target_uid: string;
  command: "Get" | "Set";
  parameter_id: number;
  parameter_data_hex: string;
  timeout_ms: number;
}

export interface ArtRdmDeviceInfoResponse {
  protocol_version: number;
  model_id: number;
  product_category: number;
  software_version_id: number;
  dmx_footprint: number;
  current_personality: number;
  personality_count: number;
  dmx_start_address: number;
  sub_device_count: number;
  sensor_count: number;
}

export interface ArtRdmResponse {
  source_uid: string;
  destination_uid: string;
  command_class: string;
  parameter_id: number;
  parameter_data_hex: string;
  response_type: "Ack" | "AckTimer" | "NackReason" | "AckOverflow";
  response_blocks: number;
  ack_timer_count: number;
  queued_message_polls: number;
  nack_reason?: number | null;
  fifo_available: number;
  fifo_max: number;
  device_info?: ArtRdmDeviceInfoResponse | null;
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
  consecutive_failures: number;
  reconnect_attempts: number;
  reconnecting: boolean;
  retry_in_ms?: number | null;
  last_success_unix_ms?: number | null;
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
  strobe_hz?: number;
  strobe_fixture_count?: number;
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

export type StageObjectKind = "Stage" | "Truss" | "Screen" | "Riser" | "Mask";

export interface StageObjectSummary {
  id: number;
  label: string;
  kind: StageObjectKind;
  x: number;
  z: number;
  width: number;
  depth: number;
  rotation_deg: number;
  color?: string | null;
}

export interface StageMapPresetSummary {
  label: string;
  config: StageMapConfig;
  stage_objects?: StageObjectSummary[] | null;
}

export type TouchControlKind =
  | "Label"
  | "Image"
  | "Button"
  | "Fader"
  | "Dial"
  | "IncrementalWheel"
  | "ColorWheel"
  | "XyGrid";

export interface TouchFeaturePresetTarget {
  fixture_id: number;
  attribute: string;
}

export type TouchControlBinding =
  | { kind: "fixture_attribute"; fixture_id: number; attribute: string }
  | { kind: "group_attribute"; group_id: string; attribute: string }
  | {
      kind: "feature_preset";
      targets: TouchFeaturePresetTarget[];
      min_value: number;
      max_value: number;
      inverted: boolean;
    }
  | { kind: "fixture_color"; fixture_id: number }
  | { kind: "group_color"; group_id: string }
  | { kind: "fixture_pan_tilt"; fixture_id: number; pan_attribute: string; tilt_attribute: string }
  | { kind: "group_pan_tilt"; group_id: string; pan_attribute: string; tilt_attribute: string }
  | { kind: "cue"; cue_id: number }
  | { kind: "group_select"; group_id: string }
  | { kind: "group_submaster"; group_id: string }
  | { kind: "tap_tempo" }
  | { kind: "lighting_master" }
  | { kind: "video_master" }
  | { kind: "blackout" }
  | { kind: "video_blackout" }
  | { kind: "all_blackout" }
  | { kind: "cue_next" }
  | { kind: "cue_previous" }
  | { kind: "cue_fade_pause" }
  | { kind: "selected_fixture_attribute"; attribute: string }
  | { kind: "selected_fixture_color" }
  | { kind: "selected_fixture_pan_tilt"; pan_attribute: string; tilt_attribute: string };

export interface TouchControlSummary {
  id: number;
  kind: TouchControlKind;
  x: number;
  y: number;
  w: number;
  h: number;
  label: string;
  binding?: TouchControlBinding | null;
}

export interface TouchPageSummary {
  id: number;
  label: string;
  controls: TouchControlSummary[];
}

export interface TouchSurfaceSummary {
  pages: TouchPageSummary[];
}

export interface DirectChildTimelineTransportSummary {
  cue_id: number;
  position_ms: number;
  duration_ms: number;
  playing: boolean;
  generation: number;
  count_in_remaining_ms?: number;
}

export interface EngineSnapshot {
  fixtures: PatchedFixtureSummary[];
  cues: CueSummary[];
  cue_lists: CueListSummary[];
  palettes: ReferencePaletteSummary[];
  playback_executors: PlaybackExecutorSummary[];
  playback_master: number;
  active_cue_id?: number | null;
  /** Runtime-only transport state for directly triggered Super Scenes. */
  direct_child_timeline_transports?: DirectChildTimelineTransportSummary[];
  active_group_cue_ids?: Record<string, number>;
  /** T17 runtime-only latched scene live overrides; never part of `.sdc` data. */
  cue_live_modifiers?: CueLiveModifierState[];
  /** T7 persistent identity colors per group path (#rrggbb). */
  group_colors?: Record<string, string>;
  active_fade?: ActiveFadeSummary | null;
  programmer: ProgrammerSnapshot;
  timeline: TimelineSnapshot;
  video: VideoSnapshot;
  authored_video?: VideoSnapshot | null;
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
    external_sync_age_ms: number | null;
    external_sync_locked: boolean;
  };
  stage_map: StageMapConfig;
  stage_map_presets: StageMapPresetSummary[];
  stage_objects: StageObjectSummary[];
  touch_surface?: TouchSurfaceSummary;
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
