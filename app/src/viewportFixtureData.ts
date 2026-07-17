import { defaultFixtureLimits } from "./fixtureLimits";
import type {
  AttributeControl,
  AudioAnalysisSummary,
  CompositionSummary,
  CueSummary,
  EffectSummary,
  FixtureProfileSummary,
  NodeGraphSummary,
  PatchedFixtureSummary,
  StageObjectSummary,
  TimelineAudioClipSummary,
  TimelineCueEventSummary,
  TimelineLayerSummary,
  VideoIsfEffectSummary,
  VideoLayerSummary,
  VideoOutputMapping,
  VideoOutputSummary,
} from "./types";
import { defaultColorAdjust, defaultFxAdjust, defaultTransform } from "./videoLayerDefaults";
import { defaultVideoOutputMapping } from "./videoOutputMapping";

export const browserViewportFixture = (tauriRuntime: boolean) => {
  if (tauriRuntime || typeof window === "undefined") {
    return "";
  }
  return new URLSearchParams(window.location.search).get("syndocalViewportFixture") ?? "";
};

// T12: pane-window mode works in BOTH browser and Tauri runtimes - a pane
// window is the full app collapsed to one pane by a root class.
export const paneWindowMode = (): "" | "stage" | "timeline" => {
  if (typeof window === "undefined") return "";
  const value = new URLSearchParams(window.location.search).get("syndocalPaneWindow") ?? "";
  return value === "stage" || value === "timeline" ? value : "";
};

// T12: harness-only simulation of popped panes in the main window.
export const browserPoppedPanes = (): string[] => {
  if (typeof window === "undefined") return [];
  const value = new URLSearchParams(window.location.search).get("syndocalPoppedPanes") ?? "";
  return value.split(",").map((pane) => pane.trim()).filter((pane) => pane === "stage" || pane === "timeline");
};

const viewportFixtureControls: AttributeControl[] = [
  {
    attribute: "Dimmer",
    channel_name: "Dimmer",
    offsets: [1],
    resolution: "EightBit",
    default_value: 0,
    functions: [],
  },
  {
    attribute: "Pan",
    channel_name: "Pan",
    offsets: [2, 3],
    resolution: "SixteenBit",
    default_value: 32_768,
    functions: [],
  },
  {
    attribute: "Tilt",
    channel_name: "Tilt",
    offsets: [4, 5],
    resolution: "SixteenBit",
    default_value: 32_768,
    functions: [],
  },
  {
    attribute: "ColorRed",
    channel_name: "Red",
    offsets: [6],
    resolution: "EightBit",
    default_value: 0,
    functions: [],
  },
  {
    attribute: "ColorGreen",
    channel_name: "Green",
    offsets: [7],
    resolution: "EightBit",
    default_value: 0,
    functions: [],
  },
  {
    attribute: "ColorBlue",
    channel_name: "Blue",
    offsets: [8],
    resolution: "EightBit",
    default_value: 0,
    functions: [],
  },
];

const profile: FixtureProfileSummary = {
  source_path: "viewport://syndocal-mini-par",
  manufacturer: "Syndocal",
  name: "Viewport Mini Par",
  short_name: "Viewport Par",
  fixture_type_id: "viewport-mini-par",
  dmx_modes: [
    {
      name: "Dimmer Pan Tilt RGB",
      controls: viewportFixtureControls,
    },
  ],
  geometries: [],
  warnings: [],
};

export const viewportPatchedFixture = (
  id: number,
  label: string,
  address: number,
  x: number,
  z: number,
): PatchedFixtureSummary => ({
  id,
  label,
  profile_source_path: "viewport://syndocal-mini-par",
  profile_name: "Viewport Mini Par",
  manufacturer: "Syndocal",
  mode_name: "Dimmer Pan Tilt RGB",
  universe: 0,
  address,
  group_ids: ["front"],
  position: { x, y: 3, z },
  rotation: { pitch: 0, yaw: 0, roll: 0 },
  geometries: [],
  controls: viewportFixtureControls,
  attribute_values: [
    { attribute: "Dimmer", value: 0 },
    { attribute: "Pan", value: 32_768 },
    { attribute: "Tilt", value: 32_768 },
    { attribute: "ColorRed", value: 65_535 },
    { attribute: "ColorGreen", value: 0 },
    { attribute: "ColorBlue", value: 0 },
  ],
  limits: defaultFixtureLimits,
  highlighted: false,
  soloed: false,
  parked: false,
});

const projectorMapping: VideoOutputMapping = {
  ...defaultVideoOutputMapping,
  stage_x: 0,
  stage_y: 2.8,
  stage_z: 3.2,
  offset_x: 0.08,
  offset_y: -0.04,
  scale_x: 1.06,
  scale_y: 0.94,
  aspect_ratio: 16 / 9,
  aspect_mode: "Fit",
  lens_distortion: -0.05,
  keystone_x: 0.12,
  keystone_y: -0.08,
  corner_top_left_x: -0.04,
  corner_top_right_y: 0.05,
  corner_bottom_right_x: 0.03,
  corner_bottom_left_y: -0.04,
};

const videoLayer: VideoLayerSummary = {
  id: 1,
  label: "Viewport Visual",
  source: {
    kind: "StillImage",
    path: "viewport://visual.png",
    name: "Viewport Visual",
    codec: "RGBA",
    metadata: {
      duration_ms: 4000,
      width: 1920,
      height: 1080,
      frame_rate: 60,
    },
  },
  blend_mode: "Add",
  state: {
    enabled: true,
    solo: false,
    opacity: 0.72,
    speed: 1,
    playing: true,
    position_ms: 1000,
    loop_enabled: true,
    loop_start_ms: 0,
    loop_end_ms: 4000,
    bpm_sync: {
      enabled: true,
      ratio: 1,
      loop_bars: 1,
    },
    cue_points: [
      { position_ms: 0, label: "Start", color: "#4aa8ff" },
      { position_ms: 2000, label: "Drop", color: "#f2c14e" },
    ],
    cue_points_ms: [0, 2000],
    transform: { ...defaultTransform },
    color: { ...defaultColorAdjust },
    fx: { ...defaultFxAdjust },
  },
};

const composition: CompositionSummary = {
  id: 1,
  label: "Viewport Comp",
  layer_ids: [1],
  output_ids: [1],
};

const videoOutput: VideoOutputSummary = {
  id: 1,
  label: "Viewport Video Output",
  kind: "Display",
  enabled: true,
  composition_id: 1,
  fullscreen: false,
  monitor_id: 1,
  width: 1920,
  height: 1080,
  endpoint_name: null,
  opacity: 1,
  blackout: false,
  mapping: projectorMapping,
};

const operatorThresholdEffect: VideoIsfEffectSummary = {
  enabled: true,
  label: "Threshold",
  source: "viewport://operator-vj/threshold.fs",
  source_path: null,
  description: "Operator fixture threshold with adjustable level and softness.",
  categories: ["Color", "Viewport Fixture"],
  controls: [
    {
      name: "level",
      kind: "Float",
      value: [0.5, 0, 0, 0],
      default: [0.5, 0, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [1, 0, 0, 0],
      labels: [],
      values: [],
    },
    {
      name: "useSourceAlpha",
      kind: "Bool",
      value: [1, 0, 0, 0],
      default: [1, 0, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [1, 0, 0, 0],
      labels: [],
      values: [],
    },
    {
      name: "toneCount",
      kind: "Long",
      value: [6, 0, 0, 0],
      default: [4, 0, 0, 0],
      minimum: [2, 0, 0, 0],
      maximum: [16, 0, 0, 0],
      labels: [],
      values: [],
    },
    {
      name: "center",
      kind: "Point2d",
      value: [0.5, 0.5, 0, 0],
      default: [0.5, 0.5, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [1, 1, 0, 0],
      labels: [],
      values: [],
    },
    {
      name: "tint",
      kind: "Color",
      value: [1, 0.75, 0.4, 1],
      default: [1, 1, 1, 1],
      minimum: [0, 0, 0, 0],
      maximum: [1, 1, 1, 1],
      labels: [],
      values: [],
    },
    {
      name: "pulse",
      kind: "Event",
      value: [0, 0, 0, 0],
      default: [0, 0, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [1, 0, 0, 0],
      labels: [],
      values: [],
    },
  ],
};

const operatorBypassedMonochromeEffect: VideoIsfEffectSummary = {
  enabled: false,
  label: "Monochrome",
  source: "viewport://operator-vj/monochrome.fs",
  source_path: null,
  description: "Bypassed fixture effect for fast-state coverage.",
  categories: ["Color", "Viewport Fixture"],
  controls: [
    {
      name: "amount",
      kind: "Float",
      value: [0.75, 0, 0, 0],
      default: [1, 0, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [1, 0, 0, 0],
      labels: [],
      values: [],
    },
  ],
};

const operatorRgbSplitEffect: VideoIsfEffectSummary = {
  enabled: true,
  label: "RGB Split",
  source: "viewport://operator-vj/rgb-split.fs",
  source_path: null,
  description: "Enabled fixture effect for mixed rack-state coverage.",
  categories: ["Glitch", "Viewport Fixture"],
  controls: [
    {
      name: "amount",
      kind: "Float",
      value: [0.04, 0, 0, 0],
      default: [0.015, 0, 0, 0],
      minimum: [0, 0, 0, 0],
      maximum: [0.2, 0, 0, 0],
      labels: [],
      values: [],
    },
  ],
};

const operatorInvertEffect: VideoIsfEffectSummary = {
  enabled: true,
  label: "Invert",
  source: "viewport://operator-vj/invert.fs",
  source_path: null,
  description: "Control-free fixture effect for empty advanced-state coverage.",
  categories: ["Color", "Viewport Fixture"],
  controls: [],
};

const operatorMirrorEffect: VideoIsfEffectSummary = {
  ...operatorInvertEffect,
  label: "Mirror",
  source: "viewport://operator-vj/mirror.fs",
  description: "Fixture stage four for maximum-stack coverage.",
  categories: ["Geometry", "Viewport Fixture"],
};

const operatorScanlinesEffect: VideoIsfEffectSummary = {
  ...operatorInvertEffect,
  label: "Scanlines",
  source: "viewport://operator-vj/scanlines.fs",
  description: "Fixture stage five for maximum-stack coverage.",
  categories: ["Glitch", "Viewport Fixture"],
};

const operatorVignetteEffect: VideoIsfEffectSummary = {
  ...operatorInvertEffect,
  label: "Vignette",
  source: "viewport://operator-vj/vignette.fs",
  description: "Fixture stage six for maximum-stack coverage.",
  categories: ["Color", "Viewport Fixture"],
};

const operatorPosterizeEffect: VideoIsfEffectSummary = {
  ...operatorInvertEffect,
  label: "Posterize",
  source: "viewport://operator-vj/posterize.fs",
  description: "Fixture stage seven for maximum-stack coverage.",
  categories: ["Color", "Viewport Fixture"],
};

const operatorThresholdStackEffect: VideoIsfEffectSummary = {
  ...operatorThresholdEffect,
  stack: [
    operatorBypassedMonochromeEffect,
    operatorInvertEffect,
    operatorRgbSplitEffect,
    operatorMirrorEffect,
    operatorScanlinesEffect,
    operatorVignetteEffect,
    operatorPosterizeEffect,
  ],
};

const operatorVjLayerLabels = [
  "Threshold Pulse",
  "Bypassed Mono",
  "Clean Plate",
  "RGB Split Echo",
  "Camera Matte",
  "Logo Overlay",
  "Emergency Loop",
] as const;

const operatorVjLayerEffects: Array<VideoIsfEffectSummary | null> = [
  operatorThresholdStackEffect,
  operatorBypassedMonochromeEffect,
  null,
  operatorRgbSplitEffect,
  null,
  operatorInvertEffect,
  null,
];

const operatorVjLayers: VideoLayerSummary[] = operatorVjLayerLabels.map((layerLabel, index) => ({
  ...videoLayer,
  id: index + 1,
  label: layerLabel,
  source: {
    kind: "File",
    path: `viewport://operator-vj/layer-${index + 1}.mp4`,
    name: layerLabel,
    codec: "H264",
    metadata: {
      duration_ms: 8_000 + index * 1_000,
      width: 1_920,
      height: 1_080,
      frame_rate: 60,
      has_audio: index < 2,
    },
  },
  blend_mode: index === 0 || index === 3 ? "Add" : "Normal",
  state: {
    ...videoLayer.state,
    opacity: Math.max(0.4, 1 - index * 0.08),
    position_ms: 500 + index * 250,
    loop_end_ms: 8_000 + index * 1_000,
    bpm_sync: { ...videoLayer.state.bpm_sync },
    cue_points: videoLayer.state.cue_points.map((cuePoint) => ({ ...cuePoint })),
    cue_points_ms: [...videoLayer.state.cue_points_ms],
    transform: { ...videoLayer.state.transform },
    color: { ...videoLayer.state.color },
    fx: { ...videoLayer.state.fx },
  },
  isf_effect: operatorVjLayerEffects[index],
}));

const operatorVjComposition: CompositionSummary = {
  id: 1,
  label: "Operator Program",
  layer_ids: operatorVjLayers.map((layer) => layer.id),
  output_ids: [1, 2, 3],
};

const operatorVjOutputs: VideoOutputSummary[] = [
  {
    ...videoOutput,
    id: 1,
    label: "Main LED",
    enabled: true,
    opacity: 1,
    blackout: false,
    monitor_id: 1,
    mapping: { ...projectorMapping, stage_x: 0, stage_z: 3.2 },
  },
  {
    ...videoOutput,
    id: 2,
    label: "Side Projection",
    enabled: false,
    opacity: 1,
    blackout: false,
    monitor_id: 2,
    mapping: { ...projectorMapping, stage_x: -4.5, stage_z: 2.4 },
  },
  {
    ...videoOutput,
    id: 3,
    label: "Stream Fill",
    enabled: true,
    opacity: 1,
    blackout: true,
    monitor_id: 3,
    mapping: { ...projectorMapping, stage_x: 4.5, stage_z: 2.4 },
  },
];

const stageObject: StageObjectSummary = {
  id: 1,
  label: "Viewport Screen",
  kind: "Screen",
  x: 0,
  z: 3.2,
  width: 6.4,
  depth: 3.6,
  rotation_deg: 0,
  color: "#2f6f9f",
};

const cueRecallEffect: EffectSummary = {
  id: 101,
  label: "Viewport Video Pulse",
  effect_type: "Lfo",
  fixture_ids: [],
  target_group_ids: [],
  attribute: "Dimmer",
  video_targets: [{ layer_ids: [1], param: "Opacity", low: 0.2, high: 1, position: null }],
  shape: "Sine",
  period_ms: 2_000,
  clock_sync: null,
  low: 0,
  high: 65_535,
  phase: 0,
  blend_mode: "Override",
  enabled: true,
  color: null,
};

const cueRecallNodeGraph: NodeGraphSummary = {
  id: 201,
  label: "Viewport Node Graph",
  enabled: true,
  nodes: [],
  edges: [],
};

const cueRecallCue: CueSummary = {
  id: 301,
  cue_list_id: 1,
  cue_number: "1",
  label: "Viewport Cue",
  fade_ms: 1_000,
  authored_beats: 4,
  pre_wait_ms: 0,
  follow_ms: null,
  ifcb_timing: {
    intensity_fade_ms: null,
    intensity_delay_ms: 0,
    focus_fade_ms: null,
    focus_delay_ms: 0,
    color_fade_ms: null,
    color_delay_ms: 0,
    beam_fade_ms: null,
    beam_delay_ms: 0,
  },
  parts: [],
  mark: false,
  mib_fixture_ids: [],
  palette_targets: [],
  tracking: true,
  notes: "",
  targets: [{ fixture_id: 1, values: [{ attribute: "Dimmer", value: 32_768 }] }],
  steps: [
    {
      values: [{ fixture_id: 1, values: [{ attribute: "Dimmer", value: 12_000 }] }],
      fade_ms: 250,
      hold_ms: 750,
    },
    {
      values: [{ fixture_id: 1, values: [{ attribute: "Dimmer", value: 42_000 }] }],
      fade_ms: 500,
      hold_ms: 500,
    },
    {
      values: [{ fixture_id: 1, values: [{ attribute: "Dimmer", value: 65_535 }] }],
      fade_ms: 1_000,
      hold_ms: 250,
    },
  ],
  video_targets: [],
  video_output_targets: [],
  node_graph_targets: [{ graph_id: cueRecallNodeGraph.id, enabled: true }],
  effect_targets: [{ effect_id: cueRecallEffect.id, enabled: true }],
};

const sceneMatrixCues: CueSummary[] = [
  {
    ...cueRecallCue,
    id: 301,
    cue_number: "1",
    label: "Front Base",
    group_id: "front",
    recall_mode: "Coexist",
    node_graph_targets: [],
    effect_targets: [],
  },
  {
    ...cueRecallCue,
    id: 302,
    cue_number: "2",
    label: "Front Hit",
    group_id: "front",
    recall_mode: "ReplaceGroup",
    node_graph_targets: [],
    effect_targets: [],
  },
  {
    ...cueRecallCue,
    id: 303,
    cue_number: "3",
    label: "Back Sweep",
    group_id: "back",
    recall_mode: "ReplaceGroup",
    node_graph_targets: [],
    effect_targets: [],
  },
  {
    ...cueRecallCue,
    id: 304,
    cue_number: "4",
    label: "Show Blackout",
    group_id: null,
    recall_mode: "Coexist",
    node_graph_targets: [],
    effect_targets: [],
  },
];

const layeredTimelineLayers: TimelineLayerSummary[] = [
  {
    id: 10,
    label: "Audio Bed",
    order: 0,
    muted: false,
    locked: false,
    solo: false,
    kind: "Audio",
  },
  {
    id: 11,
    label: "Audio Hits",
    order: 1,
    muted: false,
    locked: false,
    solo: false,
    kind: "Audio",
  },
  {
    id: 12,
    label: "Front Wash",
    order: 2,
    muted: false,
    locked: false,
    solo: false,
    kind: "Lighting",
  },
  {
    id: 13,
    label: "Lighting FX",
    order: 3,
    muted: false,
    locked: false,
    solo: false,
    kind: "Lighting",
  },
  {
    id: 14,
    label: "Video Main",
    order: 4,
    muted: false,
    locked: false,
    solo: false,
    kind: "Video",
  },
  {
    id: 15,
    label: "Video Accent",
    order: 5,
    muted: false,
    locked: false,
    solo: false,
    kind: "Video",
  },
];

const layeredTimelineAudioAnalysis: AudioAnalysisSummary = {
  path: "C:/fixture/audio/main-bed.wav",
  sample_rate: 48_000,
  channels: 2,
  duration_ms: 4_000,
  estimated_bpm: 120,
  waveform: Array.from({ length: 41 }, (_, index) => ({
    time_ms: index * 100,
    peak: 0.2 + (index % 7) * 0.11,
    rms: 0.1 + (index % 5) * 0.08,
  })),
  spectrum: [],
  beats: [0, 500, 1_000, 1_500, 2_000, 2_500, 3_000, 3_500],
};

const layeredTimelineAudioClips: TimelineAudioClipSummary[] = [
  {
    id: 700,
    layer_id: 10,
    path: layeredTimelineAudioAnalysis.path,
    start_ms: 250,
    offset_ms: 100,
    duration_ms: 3_200,
    gain: 1,
    fade_in_ms: 400,
    fade_out_ms: 600,
  },
  {
    id: 701,
    layer_id: 11,
    path: "C:/fixture/audio/hit.wav",
    start_ms: 1_800,
    offset_ms: 0,
    duration_ms: 1_400,
    gain: 0.8,
    fade_in_ms: 0,
    fade_out_ms: 200,
  },
];

const layeredTimelineSuperSceneCue: CueSummary = {
  ...cueRecallCue,
  id: 350,
  cue_number: "SS1",
  label: "Shin",
  child_timeline: {
    layers: [
      { id: 50, label: "Shin Audio", order: 0, muted: false, locked: false, solo: false, kind: "Audio" },
      { id: 51, label: "Shin Wash", order: 1, muted: false, locked: false, solo: false, kind: "Lighting" },
      { id: 52, label: "Shin Accent", order: 2, muted: false, locked: false, solo: false, kind: "Lighting" },
    ],
    events: [
      {
        id: 8_501,
        cue_id: cueRecallCue.id,
        time_ms: 0,
        time_beats: null,
        track: "Lighting",
        layer_id: 51,
        duration_ms: 1_500,
        duration_beats: null,
        conform_to_tempo: false,
        loop_fill: false,
        rate: null,
        fade_in_ms: 150,
        fade_out_ms: 200,
        loop_count: 1,
        jump_to_event_id: null,
      },
      {
        id: 8_502,
        cue_id: cueRecallCue.id,
        time_ms: 1_750,
        time_beats: null,
        track: "Lighting",
        layer_id: 52,
        duration_ms: 1_250,
        duration_beats: null,
        conform_to_tempo: false,
        loop_fill: false,
        rate: null,
        fade_in_ms: 100,
        fade_out_ms: 150,
        loop_count: 1,
        jump_to_event_id: null,
      },
    ],
    automations: [],
    video_automations: [],
    audio: null,
    audio_clips: [{
      id: 8_503,
      layer_id: 50,
      path: "C:/fixture/audio/shin-child.wav",
      start_ms: 250,
      offset_ms: 0,
      duration_ms: 2_500,
      gain: 0.9,
      fade_in_ms: 200,
      fade_out_ms: 300,
    }],
    duration_ms: 3_000,
  },
};

const layeredTimelineSuperSceneEvent: TimelineCueEventSummary = {
  id: 8_500,
  cue_id: layeredTimelineSuperSceneCue.id,
  time_ms: 2_250,
  time_beats: null,
  track: "Lighting",
  layer_id: 13,
  duration_ms: 2_500,
  duration_beats: null,
  conform_to_tempo: false,
  loop_fill: false,
  rate: null,
  fade_in_ms: 200,
  fade_out_ms: 250,
  loop_count: 1,
  jump_to_event_id: null,
};

const vjBankLayerLabels = [
  "Opener Loop",
  "Strobe Wash",
  "Logo Sting",
  "Liquid Ink",
  "Crowd Cam",
  "Geo Tunnel",
  "Glitch Burst",
  "Smoke Drift",
  "Neon Grid",
  "Kanji Flash",
  "Particle Rain",
  "Blackout Card",
  "Encore Loop",
  "Credits Roll",
] as const;

const vjBankLayers: VideoLayerSummary[] = vjBankLayerLabels.map((layerLabel, index) => ({
  ...videoLayer,
  id: index + 1,
  label: layerLabel,
  source: {
    kind: "File",
    path: `viewport://vj-bank/clip-${index + 1}.mp4`,
    name: layerLabel,
    codec: "H264",
    metadata: {
      duration_ms: 6_000 + index * 500,
      width: 1_920,
      height: 1_080,
      frame_rate: 60,
      has_audio: index % 3 === 0,
    },
  },
  blend_mode: "Normal",
  state: {
    ...videoLayer.state,
    opacity: index === 0 ? 1 : 0.85,
    playing: index === 0,
    enabled: index < 2,
    bpm_sync: { ...videoLayer.state.bpm_sync },
    cue_points: videoLayer.state.cue_points.map((cuePoint) => ({ ...cuePoint })),
    cue_points_ms: [...videoLayer.state.cue_points_ms],
    transform: { ...videoLayer.state.transform },
    color: { ...videoLayer.state.color },
    fx: { ...videoLayer.state.fx },
  },
}));

const vjBankComposition: CompositionSummary = {
  id: 1,
  label: "Bank Program",
  layer_ids: vjBankLayers.map((layer) => layer.id),
  output_ids: [1, 2],
};

const vjBankOutputs: VideoOutputSummary[] = [
  {
    ...videoOutput,
    id: 1,
    label: "Main Screen",
    enabled: true,
    opacity: 1,
    blackout: false,
    monitor_id: 1,
    mapping: { ...projectorMapping, stage_x: 0, stage_z: 3.2 },
  },
  {
    ...videoOutput,
    id: 2,
    label: "Side Fill",
    enabled: true,
    opacity: 0.9,
    blackout: false,
    monitor_id: 2,
    mapping: { ...projectorMapping, stage_x: -4.2, stage_z: 2.6 },
  },
];

export const viewportFixtureData = {
  profile,
  projectorMapping,
  videoLayer,
  composition,
  videoOutput,
  operatorVjLayers,
  operatorVjComposition,
  operatorVjOutputs,
  vjBankLayers,
  vjBankComposition,
  vjBankOutputs,
  stageObject,
  cueRecallEffect,
  cueRecallNodeGraph,
  cueRecallCue,
  sceneMatrixCues,
  layeredTimelineLayers,
  layeredTimelineAudioAnalysis,
  layeredTimelineAudioClips,
  layeredTimelineSuperSceneCue,
  layeredTimelineSuperSceneEvent,
} as const;
