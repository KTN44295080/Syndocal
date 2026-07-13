import { defaultFixtureLimits } from "./fixtureLimits";
import type {
  AttributeControl,
  CompositionSummary,
  CueSummary,
  EffectSummary,
  FixtureProfileSummary,
  NodeGraphSummary,
  PatchedFixtureSummary,
  StageObjectSummary,
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
  video_targets: [],
  video_output_targets: [],
  node_graph_targets: [{ graph_id: cueRecallNodeGraph.id, enabled: true }],
  effect_targets: [{ effect_id: cueRecallEffect.id, enabled: true }],
};

export const viewportFixtureData = {
  profile,
  projectorMapping,
  videoLayer,
  composition,
  videoOutput,
  stageObject,
  cueRecallEffect,
  cueRecallNodeGraph,
  cueRecallCue,
} as const;
