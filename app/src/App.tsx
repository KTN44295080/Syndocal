import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createEffect, createMemo, createSignal, For, onCleanup, Show } from "solid-js";
import { CueCapturePreviewPanel } from "./components/CueCapturePreviewPanel";
import { DmxOutputConfigPanel } from "./components/DmxOutputConfigPanel";
import { DmxRawMonitor } from "./components/DmxRawMonitor";
import { DmxRoutesPanel } from "./components/DmxRoutesPanel";
import { DmxTestFramePanel } from "./components/DmxTestFramePanel";
import { EngineTelemetryPanel } from "./components/EngineTelemetryPanel";
import { TimelineOverview, type TimelineOverviewEvent } from "./components/TimelineOverview";
import type {
  AttributeControl,
  AttributeResolution,
  AudioAnalysisSummary,
  AutomationInterpolation,
  CustomFixtureProfileRequest,
  CueSummary,
  DmxOutputConfig,
  DmxTestFrameResult,
  EffectBlendMode,
  EffectKind,
  EngineTelemetryReport,
  EngineSnapshot,
  FixtureLimits,
  FixtureProfileSummary,
  LearnedMidiControl,
  LearnedOscControl,
  LfoShape,
  MidiControlAction,
  MidiControlMapping,
  MidiControlMessage,
  MidiInputSummary,
  MidiOutputSummary,
  OscControlAction,
  OscControlMapping,
  OscInputConfig,
  PatchFixtureRequest,
  PatchedFixtureSummary,
  Phase1SmokeReport,
  ProjectLoadResult,
  RemoteControlConfig,
  SerialPortSummary,
  StageMapConfig,
  TimelineCueEventSummary,
  TimelineAutomationSummary,
  TimelineTrackKind,
  TimelineVideoAutomationSummary,
  VideoBlendMode,
  VideoFrame,
  VideoEffectTarget,
  VideoLayerState,
  VideoOutputAspectMode,
  VideoOutputKind,
  VideoOutputMapping,
  VideoOutputSummary,
  VideoParam,
  VideoPreviewDiagnostics,
  VideoSourceKind,
} from "./types";

type TimelineSnapMode = "Off" | "Beat" | "Bar" | "Grid";
type WorkspaceTab = "setup" | "control" | "touch";
type SetupSubTab = "library" | "profiles" | "patch" | "mapping" | "output";
type FixtureLayoutMode = "line" | "grid" | "circle";
type MappingAxis = "x" | "z";
type VideoOutputPreviewMode = "output" | "test";
type DmxPatchViewMode = "grid" | "list";
type ControlCategory = "dimmer" | "color" | "position" | "gobo" | "beam" | "focus" | "other" | "fader";
type MappingBulkGroupMode = "add" | "remove" | "set";
type MappingFixtureFlag = "highlight" | "solo" | "park";
type MappingStageTool = "select" | "place" | "rotate" | "pan";
type WaveStageDragMode = "origin" | "direction" | "videoTarget";
type CueCaptureScopeMode = "all" | "lighting" | "selectedFixture" | "selectedGroup" | "video";

interface EffectTargetOverride {
  fixture_ids: number[];
  target_group_ids: string[];
  attribute: string;
  video_targets: VideoEffectTarget[];
}

type CueCaptureScopeRequest =
  | { kind: "all" }
  | { kind: "lightingOnly" }
  | { kind: "selectedFixture"; fixtureId: number }
  | { kind: "selectedGroup"; groupId: string }
  | { kind: "videoOnly" };

const defaultOutput: DmxOutputConfig = {
  enabled: true,
  protocol: "ArtNet",
  target_ip: "127.0.0.1",
  port: 6454,
  universe: 0,
  serial_port: "",
  serial_baud_rate: 57_600,
};

const defaultTransform = {
  x: 0,
  y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation_deg: 0,
  crop_left: 0,
  crop_top: 0,
  crop_right: 0,
  crop_bottom: 0,
};

const defaultColorAdjust = {
  brightness: 0,
  contrast: 1,
  hue_deg: 0,
  saturation: 1,
  gamma: 1,
};

const defaultFxAdjust = {
  pixelate: 1,
  blur: 0,
  glow: 0,
  edge: 0,
  key_red: 0,
  key_green: 1,
  key_blue: 0,
  key_threshold: 0,
};

const defaultVideoOutputMapping: VideoOutputMapping = {
  stage_x: 0,
  stage_y: 0,
  stage_z: 0,
  offset_x: 0,
  offset_y: 0,
  scale_x: 1,
  scale_y: 1,
  rotation_deg: 0,
  aspect_ratio: 1,
  aspect_mode: "Stretch",
  lens_distortion: 0,
  keystone_x: 0,
  keystone_y: 0,
  corner_top_left_x: 0,
  corner_top_left_y: 0,
  corner_top_right_x: 0,
  corner_top_right_y: 0,
  corner_bottom_right_x: 0,
  corner_bottom_right_y: 0,
  corner_bottom_left_x: 0,
  corner_bottom_left_y: 0,
};

const stageViewBoxSize = 100;
const stagePadding = 10;
const cuePadSize = 10;
const enttecUsbProBaudRate = 57_600;
const enttecOpenDmxBaudRate = 250_000;
const mappingSnapPresets = [0.25, 0.5, 1, 2];
const setupSubTabs: { id: SetupSubTab; label: string; description: string }[] = [
  { id: "library", label: "Library", description: "GDTF import and share lookup" },
  { id: "profiles", label: "Profiles", description: "Fixture profile authoring" },
  { id: "patch", label: "Patch", description: "DMX addressing and fixture assignment" },
  { id: "mapping", label: "Mapping", description: "2D fixture and projector mapping" },
  { id: "output", label: "Output", description: "DMX and video output setup" },
];
const controlCategories: { id: ControlCategory; label: string }[] = [
  { id: "dimmer", label: "Dimmer" },
  { id: "color", label: "Color" },
  { id: "position", label: "Position" },
  { id: "gobo", label: "Gobo" },
  { id: "beam", label: "Beam" },
  { id: "focus", label: "Focus" },
  { id: "other", label: "Other" },
  { id: "fader", label: "Fader" },
];
const defaultColorPalette = [
  "#ff0000",
  "#00ff00",
  "#0000ff",
  "#ffff00",
  "#ff00ff",
  "#00ffff",
  "#ffffff",
  "#ff7a00",
  "#7a2cff",
  "#1ee6a8",
  "#ff4fa3",
  "#b5ff2f",
];
const panTiltNudgeSteps = [
  { label: "Fine", value: 256 },
  { label: "Small", value: 1024 },
  { label: "Medium", value: 2048 },
  { label: "Coarse", value: 8192 },
];
const positionFavoritesStorageKey = "rayard.positionFavorites.v1";
const colorFavoritesStorageKey = "rayard.colorFavorites.v1";
const defaultFixtureLimits: FixtureLimits = {
  dimmer_min: 0,
  dimmer_max: 65535,
  pan_min: 0,
  pan_max: 65535,
  tilt_min: 0,
  tilt_max: 65535,
  invert_pan: false,
  invert_tilt: false,
  swap_pan_tilt: false,
};

const isSerialDmxProtocol = (protocol: DmxOutputConfig["protocol"]) =>
  protocol === "EnttecUsbPro" || protocol === "EnttecOpenDmx";

const isEditableShortcutTarget = (target: EventTarget | null) => {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  const tagName = target.tagName.toLowerCase();
  return target.isContentEditable || tagName === "input" || tagName === "textarea" || tagName === "select";
};

const controlCueHotkeyIndex = (code: string) => {
  const digitMatch = code.match(/^(Digit|Numpad)(\d)$/);
  if (!digitMatch) {
    return null;
  }
  const digit = Number(digitMatch[2]);
  return digit === 0 ? 9 : digit - 1;
};

const mappingStageToolFromHotkey = (code: string): MappingStageTool | null => {
  switch (code) {
    case "KeyS":
      return "select";
    case "KeyP":
      return "place";
    case "KeyR":
      return "rotate";
    case "KeyH":
      return "pan";
    default:
      return null;
  }
};

const controlCategoryForAttribute = (attribute: string): Exclude<ControlCategory, "fader"> => {
  const normalized = attribute.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (/(dimmer|intensity)/.test(normalized)) {
    return "dimmer";
  }
  if (/(pan|tilt|position|move|movement)/.test(normalized)) {
    return "position";
  }
  if (
    /(color|colour|red|green|blue|cyan|magenta|yellow|amber|white|warmwhite|coldwhite|uv|hue|saturation|cto|ctc|ctb)/.test(
      normalized,
    )
  ) {
    return "color";
  }
  if (/(gobo|animationwheel)/.test(normalized)) {
    return "gobo";
  }
  if (/(focus|focal)/.test(normalized)) {
    return "focus";
  }
  if (/(zoom|iris|prism|frost|beam|shutter|strobe|blade|framing|wash|spot)/.test(normalized)) {
    return "beam";
  }
  return "other";
};

const normalizeCustomResolutionText = (value: string) => value.toLowerCase().replace(/[-_\s]/g, "");

const parseCustomAttributeStartOffset = (attribute: string, rawStart: string) => {
  const startOffset = Number(rawStart);
  if (!Number.isInteger(startOffset) || startOffset < 1 || startOffset > 512) {
    return {
      startOffset: null,
      error: `Invalid start channel '${rawStart}' for ${attribute || "attribute"}`,
    };
  }
  return { startOffset, error: null };
};

const parseCustomAttributePreviewSpec = (value: string) => {
  const trimmed = value.trim();
  let attribute = trimmed;
  let rawResolution = "";
  let startOffset: number | null = null;
  let startError: string | null = null;

  const channelSeparatorIndex = trimmed.lastIndexOf("@");
  if (channelSeparatorIndex >= 0) {
    attribute = trimmed.slice(0, channelSeparatorIndex).trim();
    const channelAndResolution = trimmed.slice(channelSeparatorIndex + 1).trim();
    const resolutionSeparatorIndex = channelAndResolution.indexOf(":");
    if (resolutionSeparatorIndex >= 0) {
      const parsedStart = parseCustomAttributeStartOffset(
        attribute,
        channelAndResolution.slice(0, resolutionSeparatorIndex).trim(),
      );
      startOffset = parsedStart.startOffset;
      startError = parsedStart.error;
      rawResolution = channelAndResolution.slice(resolutionSeparatorIndex + 1).trim();
    } else if (channelAndResolution.length > 0 && /^\d+$/.test(channelAndResolution)) {
      const parsedStart = parseCustomAttributeStartOffset(attribute, channelAndResolution);
      startOffset = parsedStart.startOffset;
      startError = parsedStart.error;
    } else {
      rawResolution = channelAndResolution;
    }
  } else {
    const resolutionSeparatorIndex = trimmed.lastIndexOf(":");
    if (resolutionSeparatorIndex >= 0) {
      attribute = trimmed.slice(0, resolutionSeparatorIndex).trim();
      rawResolution = trimmed.slice(resolutionSeparatorIndex + 1).trim();
    }
  }

  const normalizedResolution = normalizeCustomResolutionText(rawResolution);
  const resolution: AttributeResolution =
    normalizedResolution === "" || normalizedResolution === "8" || normalizedResolution === "8bit"
      ? "EightBit"
      : normalizedResolution === "16" || normalizedResolution === "16bit"
        ? "SixteenBit"
        : "EightBit";
  const error =
    attribute.length === 0
      ? "Attribute name is required"
      : startError
        ? startError
        : normalizedResolution !== "" &&
            normalizedResolution !== "8" &&
            normalizedResolution !== "8bit" &&
            normalizedResolution !== "16" &&
            normalizedResolution !== "16bit"
          ? `Invalid resolution '${rawResolution}' for ${attribute}`
          : null;
  return { attribute, resolution, startOffset, error };
};

const customProfilePreviewFromText = (value: string): CustomProfilePreview => {
  const controls: CustomProfileAttributePreview[] = [];
  const errors: string[] = [];
  const seen = new Set<string>();
  const occupied = new Set<number>();
  let nextOffset = 1;
  let footprint = 0;

  for (const rawAttribute of value.split(",")) {
    const trimmed = rawAttribute.trim();
    if (!trimmed) {
      continue;
    }
    const spec = parseCustomAttributePreviewSpec(trimmed);
    if (spec.error) {
      errors.push(spec.error);
      continue;
    }
    const key = spec.attribute.toLowerCase();
    if (seen.has(key)) {
      errors.push(`Duplicate attribute '${spec.attribute}'`);
      continue;
    }
    seen.add(key);
    const width = spec.resolution === "SixteenBit" ? 2 : 1;
    const startOffset = spec.startOffset ?? nextOffset;
    const offsets = Array.from({ length: width }, (_, index) => startOffset + index);
    const outOfRange = offsets.find((offset) => offset < 1 || offset > 512);
    if (outOfRange !== undefined) {
      errors.push(`Attribute '${spec.attribute}' exceeds 512 DMX channels`);
      continue;
    }
    const overlap = offsets.find((offset) => occupied.has(offset));
    if (overlap !== undefined) {
      errors.push(`Attribute '${spec.attribute}' overlaps DMX channel ${overlap}`);
      continue;
    }
    offsets.forEach((offset) => occupied.add(offset));
    footprint = Math.max(footprint, ...offsets);
    controls.push({
      attribute: spec.attribute,
      resolution: spec.resolution,
      startOffset: spec.startOffset,
      offsets,
    });
    nextOffset = Math.max(nextOffset, startOffset + width);
  }

  if (footprint > 512) {
    errors.push(`Footprint ${footprint} exceeds 512 DMX channels`);
  }

  return { controls, footprint, errors };
};

const outputProtocolLabel = (protocol: DmxOutputConfig["protocol"]) => {
  switch (protocol) {
    case "ArtNet":
      return "Art-Net";
    case "Sacn":
      return "sACN";
    case "EnttecUsbPro":
      return "Enttec USB PRO";
    case "EnttecOpenDmx":
      return "Enttec Open DMX";
  }
};

type MappingFixtureVisualKind = "point" | "moving" | "bar" | "panel" | "laser" | "par";

interface VisualizerFixture {
  id: number;
  label: string;
  dmxLabel: string;
  groupLabel: string;
  typeKey: string;
  visualKind: MappingFixtureVisualKind;
  x: number;
  z: number;
  width: number;
  height: number;
  yaw: number;
  beamPoints: string;
  intensity: number;
  color: string;
  inGroupFilter: boolean;
  highlighted: boolean;
  soloed: boolean;
  parked: boolean;
}

interface VisualizerVideoSurface2d {
  id: number;
  label: string;
  x: number;
  z: number;
  width: number;
  height: number;
  rotationDeg: number;
  opacity: number;
  active: boolean;
}

interface MappingFixtureTypeRow {
  key: string;
  label: string;
  manufacturer: string;
  mode: string;
  visualKind: MappingFixtureVisualKind;
  count: number;
}

type MappingDragState =
  | {
      kind: "fixture";
      pointerId: number;
      fixtureIds: number[];
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startPositions: Record<number, PatchFixtureRequest["position"]>;
    }
  | {
      kind: "videoOutput";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "videoOutputRotate";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      centerWorld: { x: number; z: number };
      startAngleDeg: number;
      startMapping: VideoOutputMapping;
    }
  | {
      kind: "videoOutputScale";
      pointerId: number;
      outputId: number;
      startWorld: { x: number; z: number };
      currentWorld: { x: number; z: number };
      centerWorld: { x: number; z: number };
      startDistance: number;
      startMapping: VideoOutputMapping;
    };

interface MappingMarqueeState {
  pointerId: number;
  start: { x: number; z: number };
  current: { x: number; z: number };
  additive: boolean;
}

interface MappingViewportPanDragState {
  pointerId: number;
  startClientX: number;
  startClientY: number;
  startCenterX: number;
  startCenterZ: number;
  viewBoxSize: number;
  rectWidth: number;
  rectHeight: number;
}

interface MappingSnapLine {
  axis: "x" | "z";
  svg: number;
}

interface MappingSvgBounds {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
}

interface StageWorldBounds {
  minX: number;
  maxX: number;
  minZ: number;
  maxZ: number;
}

const fixtureVisualKind = (fixture: PatchedFixtureSummary): MappingFixtureVisualKind => {
  const text = `${fixture.manufacturer} ${fixture.profile_name} ${fixture.mode_name} ${fixture.label}`.toLowerCase();
  const attributes = fixture.controls.map((control) => control.attribute.toLowerCase()).join(" ");
  if (/(laser)/.test(text)) {
    return "laser";
  }
  if (/(matrix|panel|pixel)/.test(text)) {
    return "panel";
  }
  if (/(bar|strip|batten|tube|linear)/.test(text)) {
    return "bar";
  }
  if (/(pan|tilt)/.test(attributes)) {
    return "moving";
  }
  if (/(par|wash|rgb|rgba|rgbw|led)/.test(text) || /(red|green|blue|amber|white|uv)/.test(attributes)) {
    return "par";
  }
  return "point";
};

const fixtureTypeKey = (fixture: PatchedFixtureSummary) =>
  `${fixture.manufacturer}::${fixture.profile_name}::${fixture.mode_name}`;

const fixtureTypeLabel = (fixture: PatchedFixtureSummary) => `${fixture.profile_name} / ${fixture.mode_name}`;

const mappingTypeGlyphClass = (kind: MappingFixtureVisualKind) => `mappingTypeGlyph kind-${kind}`;

const surfaceWorldHalfSize = (output: VideoOutputSummary) => {
  const aspect = output.height > 0 ? output.width / output.height : 1;
  const baseHeight = 4.5 * clampRange(output.mapping.scale_y, 0.25, 3);
  return {
    width: baseHeight * Math.max(0.35, aspect) * clampRange(output.mapping.scale_x, 0.25, 3),
    height: baseHeight,
  };
};

interface ColorControlSet {
  red: string;
  green: string;
  blue: string;
  value: string;
}

interface PositionControlSet {
  pan: string;
  tilt: string;
  panValue: number;
  tiltValue: number;
}

interface PositionFavorite {
  id: string;
  label: string;
  pan: number;
  tilt: number;
}

interface CategoryQuickLook {
  id: string;
  label: string;
  description: string;
}

interface DimmerControlSet {
  attribute: string;
  value: number;
}

interface MovementLimitPoint {
  pan: number;
  tilt: number;
}

interface MovementLimitDragState {
  anchor: MovementLimitPoint;
}

interface DmxPatchSegment {
  fixture: PatchedFixtureSummary;
  start: number;
  end: number;
  left: number;
  width: number;
}

interface DmxAddressRange {
  start: number;
  end: number;
}

interface DmxUniverseMap {
  universe: number;
  used: number;
  free: number;
  largestFree: number;
  segments: DmxPatchSegment[];
}

interface DmxAddressCell {
  channel: number;
  segment: DmxPatchSegment | null;
  isStart: boolean;
  isSelected: boolean;
  plannedIndex: number | null;
  plannedStart: boolean;
  plannedConflict: boolean;
}

interface VideoOutputConfigDraft {
  label: string;
  kind: VideoOutputKind;
  width: number;
  height: number;
  fullscreen: boolean;
  monitor_id: number;
  endpoint_name: string;
}

interface CueMetadataDraft {
  label: string;
  fade_ms: number;
}

interface TimelineEventDraft {
  cue_id: number;
  time_ms: number;
  track: TimelineTrackKind;
}

interface TimelineAutomationDraft {
  fixture_id: number;
  attribute: string;
  start_ms: number;
  end_ms: number;
  start_value: number;
  end_value: number;
  interpolation: AutomationInterpolation;
}

interface TimelineVideoAutomationDraft {
  layer_id: number;
  param: VideoParam;
  start_ms: number;
  end_ms: number;
  start_value: number;
  end_value: number;
  interpolation: AutomationInterpolation;
}

interface CustomProfileAttributePreview {
  attribute: string;
  resolution: AttributeResolution;
  startOffset: number | null;
  offsets: number[];
}

interface CustomProfilePreview {
  controls: CustomProfileAttributePreview[];
  footprint: number;
  errors: string[];
}

const clamp01 = (value: number) => Math.min(1, Math.max(0, value));
const colorCandidates = {
  red: ["ColorRed", "Red"],
  green: ["ColorGreen", "Green"],
  blue: ["ColorBlue", "Blue"],
};

const videoOutputConfigDraftFromSummary = (output: VideoOutputSummary): VideoOutputConfigDraft => ({
  label: output.label,
  kind: output.kind,
  width: output.width,
  height: output.height,
  fullscreen: output.fullscreen,
  monitor_id: output.monitor_id ?? 0,
  endpoint_name: output.endpoint_name ?? "",
});

const cueMetadataDraftFromSummary = (cue: CueSummary): CueMetadataDraft => ({
  label: cue.label,
  fade_ms: cue.fade_ms,
});

const timelineEventDraftFromSummary = (event: TimelineCueEventSummary): TimelineEventDraft => ({
  cue_id: event.cue_id,
  time_ms: event.time_ms,
  track: event.track,
});

const timelineAutomationDraftFromSummary = (automation: TimelineAutomationSummary): TimelineAutomationDraft => {
  const first = automation.keyframes[0];
  const last = automation.keyframes[automation.keyframes.length - 1] ?? first;
  return {
    fixture_id: automation.fixture_id,
    attribute: automation.attribute,
    start_ms: first?.time_ms ?? 0,
    end_ms: last?.time_ms ?? first?.time_ms ?? 0,
    start_value: first?.value ?? 0,
    end_value: last?.value ?? first?.value ?? 0,
    interpolation: first?.interpolation ?? "Linear",
  };
};

const timelineVideoAutomationDraftFromSummary = (
  automation: TimelineVideoAutomationSummary,
): TimelineVideoAutomationDraft => {
  const first = automation.keyframes[0];
  const last = automation.keyframes[automation.keyframes.length - 1] ?? first;
  return {
    layer_id: automation.layer_id,
    param: automation.param,
    start_ms: first?.time_ms ?? 0,
    end_ms: last?.time_ms ?? first?.time_ms ?? 0,
    start_value: first?.value ?? 0,
    end_value: last?.value ?? first?.value ?? 0,
    interpolation: first?.interpolation ?? "Linear",
  };
};

const findControlAttribute = (fixture: PatchedFixtureSummary, names: string[]) => {
  const normalizedNames = names.map((name) => name.toLowerCase());
  return fixture.controls.find((control) => normalizedNames.includes(control.attribute.toLowerCase()))?.attribute;
};

const valueToHexByte = (value: number) => (value >> 8).toString(16).padStart(2, "0");

const outputAspectRatio = (width: number, height: number) => {
  if (!Number.isFinite(width) || !Number.isFinite(height) || height <= 0) {
    return 1;
  }
  return Number((width / height).toFixed(4));
};

const videoOutputAspectModes: VideoOutputAspectMode[] = ["Stretch", "Fit", "Fill"];

const projectorMapViewBoxSize = 100;
const projectorCornerGain = 18;

const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
const clampDmxValue = (value: number) => Math.round(clampRange(Number.isFinite(value) ? value : 0, 0, 65_535));
const formatDmxPercent = (value: number) => `${Math.round((clampDmxValue(value) / 65_535) * 1000) / 10}%`;
const dmxValueToPercent = (value: number) => Math.round((clampDmxValue(value) / 65_535) * 1000) / 10;
const percentToDmxValue = (value: number) => clampDmxValue((clampRange(value, 0, 100) / 100) * 65_535);
const formatShortDmxPercent = (value: number) => `${Math.round(dmxValueToPercent(value))}%`;
const normalizeLimitRange = (min: number, max: number) => {
  const a = clampDmxValue(min);
  const b = clampDmxValue(max);
  return { min: Math.min(a, b), max: Math.max(a, b) };
};
const applyAxisLimit = (value: number, min: number, max: number, invert: boolean) => {
  const range = normalizeLimitRange(min, max);
  const clamped = clampDmxValue(clampRange(value, range.min, range.max));
  return invert ? range.min + range.max - clamped : clamped;
};
const sourceValueForAxisLimit = (desiredValue: number, min: number, max: number, invert: boolean) => {
  const range = normalizeLimitRange(min, max);
  const clamped = clampDmxValue(clampRange(desiredValue, range.min, range.max));
  return invert ? range.min + range.max - clamped : clamped;
};
const dimmerValueWithinLimits = (fixture: PatchedFixtureSummary, value: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const range = normalizeLimitRange(limits.dimmer_min, limits.dimmer_max);
  return clampDmxValue(clampRange(value, range.min, range.max));
};
const effectivePanTiltValues = (fixture: PatchedFixtureSummary, panValue: number, tiltValue: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const panSource = limits.swap_pan_tilt ? tiltValue : panValue;
  const tiltSource = limits.swap_pan_tilt ? panValue : tiltValue;
  return {
    pan: applyAxisLimit(panSource, limits.pan_min, limits.pan_max, limits.invert_pan),
    tilt: applyAxisLimit(tiltSource, limits.tilt_min, limits.tilt_max, limits.invert_tilt),
  };
};
const sourcePanTiltValues = (fixture: PatchedFixtureSummary, panValue: number, tiltValue: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const panSource = sourceValueForAxisLimit(panValue, limits.pan_min, limits.pan_max, limits.invert_pan);
  const tiltSource = sourceValueForAxisLimit(tiltValue, limits.tilt_min, limits.tilt_max, limits.invert_tilt);
  return limits.swap_pan_tilt
    ? { pan: tiltSource, tilt: panSource }
    : { pan: panSource, tilt: tiltSource };
};
const roundedMappingValue = (value: number) => Number(clampRange(value, -1, 1).toFixed(3));

const projectorCorners = [
  {
    key: "topLeft",
    label: "TL",
    baseX: -1,
    baseY: -1,
    xField: "corner_top_left_x",
    yField: "corner_top_left_y",
  },
  {
    key: "topRight",
    label: "TR",
    baseX: 1,
    baseY: -1,
    xField: "corner_top_right_x",
    yField: "corner_top_right_y",
  },
  {
    key: "bottomRight",
    label: "BR",
    baseX: 1,
    baseY: 1,
    xField: "corner_bottom_right_x",
    yField: "corner_bottom_right_y",
  },
  {
    key: "bottomLeft",
    label: "BL",
    baseX: -1,
    baseY: 1,
    xField: "corner_bottom_left_x",
    yField: "corner_bottom_left_y",
  },
] as const;

type ProjectorCorner = (typeof projectorCorners)[number];

const projectorMapBasePoint = (mapping: VideoOutputMapping, corner: ProjectorCorner) => {
  const aspect = clampRange(finiteOr(mapping.aspect_ratio, 1), 0.25, 4);
  const aspectScaleX = aspect >= 1 ? 1 : aspect;
  const aspectScaleY = aspect >= 1 ? 1 / Math.min(aspect, 2.8) : 1;
  const scaleX = clampRange(finiteOr(mapping.scale_x, 1), 0.25, 2.5);
  const scaleY = clampRange(finiteOr(mapping.scale_y, 1), 0.25, 2.5);
  const radius = 30;
  const localX =
    corner.baseX * radius * aspectScaleX * scaleX +
    clampRange(finiteOr(mapping.offset_x, 0), -1, 1) * 20 +
    clampRange(finiteOr(mapping.keystone_x, 0), -1, 1) * corner.baseY * 12;
  const localY =
    corner.baseY * radius * aspectScaleY * scaleY +
    clampRange(finiteOr(mapping.offset_y, 0), -1, 1) * 20 +
    clampRange(finiteOr(mapping.keystone_y, 0), -1, 1) * corner.baseX * 12;
  const rotation = finiteOr(mapping.rotation_deg, 0) * (Math.PI / 180);
  const sin = Math.sin(rotation);
  const cos = Math.cos(rotation);
  return {
    x: projectorMapViewBoxSize / 2 + localX * cos - localY * sin,
    y: projectorMapViewBoxSize / 2 + localX * sin + localY * cos,
  };
};

const projectorMapPoint = (mapping: VideoOutputMapping, corner: ProjectorCorner) => {
  const base = projectorMapBasePoint(mapping, corner);
  return {
    x: base.x + clampRange(finiteOr(mapping[corner.xField], 0), -1, 1) * projectorCornerGain,
    y: base.y + clampRange(finiteOr(mapping[corner.yField], 0), -1, 1) * projectorCornerGain,
  };
};

const projectorMapPoints = (mapping: VideoOutputMapping) =>
  projectorCorners.map((corner) => {
    const point = projectorMapPoint(mapping, corner);
    return `${point.x},${point.y}`;
  });

const projectorMapBasePoints = (mapping: VideoOutputMapping) =>
  projectorCorners.map((corner) => {
    const point = projectorMapBasePoint(mapping, corner);
    return `${point.x},${point.y}`;
  });

const bulkPatchLabel = (baseLabel: string, index: number, count: number) => {
  if (count === 1) {
    return baseLabel;
  }
  const match = baseLabel.match(/^(.*?)(\d+)$/);
  if (match) {
    return `${match[1]}${Number(match[2]) + index}`;
  }
  return `${baseLabel} ${index + 1}`;
};

const stageWorldToSvgPoint = (x: number, z: number, bounds: StageWorldBounds) => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const rangeX = Math.max(Number.EPSILON, bounds.maxX - bounds.minX);
  const rangeZ = Math.max(Number.EPSILON, bounds.maxZ - bounds.minZ);
  return {
    x: stagePadding + ((x - bounds.minX) / rangeX) * drawableSize,
    z: stagePadding + ((z - bounds.minZ) / rangeZ) * drawableSize,
  };
};

const svgPointToStageWorld = (x: number, z: number, bounds: StageWorldBounds) => {
  const drawableSize = stageViewBoxSize - stagePadding * 2;
  const normalizedX = clamp01((x - stagePadding) / drawableSize);
  const normalizedZ = clamp01((z - stagePadding) / drawableSize);
  return {
    x: bounds.minX + normalizedX * (bounds.maxX - bounds.minX),
    z: bounds.minZ + normalizedZ * (bounds.maxZ - bounds.minZ),
  };
};

const readFixtureAttribute = (
  fixture: PatchedFixtureSummary,
  currentValues: Record<string, number>,
  names: string[],
) => {
  const attributeValues = new Map(
    fixture.attribute_values.map((value) => [value.attribute.toLowerCase(), value.value]),
  );
  for (const name of names) {
    const directValue = currentValues[`${fixture.id}:${name}`];
    if (directValue !== undefined) {
      return directValue;
    }
    const snapshotValue = attributeValues.get(name.toLowerCase());
    if (snapshotValue !== undefined) {
      return snapshotValue;
    }
  }
  return undefined;
};

const projectComparableSnapshot = (snapshot: EngineSnapshot) => {
  const comparable = JSON.parse(JSON.stringify(snapshot)) as EngineSnapshot;
  comparable.active_cue_id = null;
  comparable.active_fade = null;
  comparable.timeline = {
    ...comparable.timeline,
    playing: false,
    position_ms: 0,
  };
  comparable.video = {
    ...comparable.video,
    layers: comparable.video.layers.map((layer) => ({
      ...layer,
      state: {
        ...layer.state,
        playing: false,
        position_ms: 0,
      },
    })),
  };
  comparable.clock = {
    ...comparable.clock,
    beat_phase: 0,
    beat_counter: 0,
    tap_count: 0,
  };
  comparable.dmx_preview = [];
  comparable.dmx_previews = [];
  comparable.telemetry = {} as EngineSnapshot["telemetry"];
  return comparable;
};

const projectSnapshotSignature = (snapshot: EngineSnapshot) => JSON.stringify(projectComparableSnapshot(snapshot));

const hsvToRgb = (hue: number, saturation: number, value: number) => {
  const h = ((hue % 360) + 360) % 360;
  const s = clamp01(saturation);
  const v = clamp01(value);
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let red = 0;
  let green = 0;
  let blue = 0;
  if (h < 60) {
    red = c;
    green = x;
  } else if (h < 120) {
    red = x;
    green = c;
  } else if (h < 180) {
    green = c;
    blue = x;
  } else if (h < 240) {
    green = x;
    blue = c;
  } else if (h < 300) {
    red = x;
    blue = c;
  } else {
    red = c;
    blue = x;
  }
  return {
    red: Math.round((red + m) * 255),
    green: Math.round((green + m) * 255),
    blue: Math.round((blue + m) * 255),
  };
};

const rgbToHsv = (red: number, green: number, blue: number) => {
  const r = clamp01(red / 255);
  const g = clamp01(green / 255);
  const b = clamp01(blue / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  let hue = 0;
  if (delta !== 0) {
    if (max === r) {
      hue = 60 * (((g - b) / delta) % 6);
    } else if (max === g) {
      hue = 60 * ((b - r) / delta + 2);
    } else {
      hue = 60 * ((r - g) / delta + 4);
    }
  }
  if (hue < 0) {
    hue += 360;
  }
  return {
    hue,
    saturation: max === 0 ? 0 : delta / max,
    value: max,
  };
};

const rgbToHex = (red: number, green: number, blue: number) =>
  `#${[red, green, blue].map((value) => Math.round(value).toString(16).padStart(2, "0")).join("")}`;

const defaultColorFavorites = () => ["#ff0000", "#00ff00", "#0000ff", "#ffffff"];

const normalizeHexColor = (value: unknown) => {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  return /^#[0-9a-f]{6}$/.test(normalized) ? normalized : null;
};

const loadColorFavorites = () => {
  if (typeof window === "undefined") {
    return defaultColorFavorites();
  }
  try {
    const raw = window.localStorage.getItem(colorFavoritesStorageKey);
    if (!raw) {
      return defaultColorFavorites();
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return defaultColorFavorites();
    }
    const unique = new Set<string>();
    for (const candidate of parsed) {
      const color = normalizeHexColor(candidate);
      if (color) {
        unique.add(color);
      }
    }
    const favorites = [...unique].slice(0, 12);
    return favorites.length > 0 ? favorites : defaultColorFavorites();
  } catch {
    return defaultColorFavorites();
  }
};

const saveColorFavorites = (favorites: string[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    const colors = favorites
      .map(normalizeHexColor)
      .filter((color): color is string => Boolean(color))
      .slice(0, 12);
    window.localStorage.setItem(colorFavoritesStorageKey, JSON.stringify(colors));
  } catch {
    // Local storage can be unavailable in hardened WebViews; the live palette still works in memory.
  }
};

const quickLooksForCategory = (category: ControlCategory): CategoryQuickLook[] => {
  switch (category) {
    case "gobo":
      return [
        { id: "open", label: "Open", description: "Wheel open / index zero" },
        { id: "slot1", label: "Slot 1", description: "First approximate gobo slot" },
        { id: "slot2", label: "Slot 2", description: "Second approximate gobo slot" },
        { id: "spin", label: "Spin", description: "High-range rotate/spin area" },
      ];
    case "beam":
      return [
        { id: "open", label: "Open", description: "Open shutter, no strobe/prism/frost" },
        { id: "tight", label: "Tight", description: "Narrow iris/zoom style look" },
        { id: "wide", label: "Wide", description: "Wide beam/iris style look" },
        { id: "soft", label: "Soft", description: "Frost/soft beam emphasis" },
      ];
    case "focus":
      return [
        { id: "near", label: "Near", description: "Low focus range" },
        { id: "mid", label: "Mid", description: "Middle focus range" },
        { id: "far", label: "Far", description: "High focus range" },
        { id: "default", label: "Default", description: "Fixture profile default" },
      ];
    default:
      return [];
  }
};

const normalizedFunctionText = (control: AttributeControl, fn: NonNullable<AttributeControl["functions"]>[number]) =>
  `${fn.name} ${fn.attribute} ${fn.wheel_slot ?? ""} ${control.channel_name}`.toLowerCase().replace(/[^a-z0-9]+/g, " ");

const channelFunctionValue = (fn: NonNullable<AttributeControl["functions"]>[number]) =>
  clampDmxValue((fn.dmx_from + fn.dmx_to) / 2);

const sortedChannelFunctions = (control: AttributeControl) =>
  [...(control.functions ?? [])]
    .filter((fn) => Number.isFinite(fn.dmx_from) && Number.isFinite(fn.dmx_to))
    .sort((first, second) => first.dmx_from - second.dmx_from);

const pickFunctionValue = (
  control: AttributeControl,
  predicate: (text: string, fn: NonNullable<AttributeControl["functions"]>[number]) => boolean,
) => {
  const fn = sortedChannelFunctions(control).find((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  return fn ? channelFunctionValue(fn) : null;
};

const indexedFunctionValue = (
  control: AttributeControl,
  predicate: (text: string, fn: NonNullable<AttributeControl["functions"]>[number]) => boolean,
  index: number,
) => {
  const matches = sortedChannelFunctions(control).filter((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  return matches[index] ? channelFunctionValue(matches[index]) : null;
};

const rankedFunctionValue = (
  control: AttributeControl,
  rank: "first" | "middle" | "last",
  predicate: (text: string, fn: NonNullable<AttributeControl["functions"]>[number]) => boolean = () => true,
) => {
  const matches = sortedChannelFunctions(control).filter((candidate) =>
    predicate(normalizedFunctionText(control, candidate), candidate),
  );
  if (matches.length === 0) {
    return null;
  }
  const index = rank === "first" ? 0 : rank === "last" ? matches.length - 1 : Math.floor(matches.length / 2);
  return channelFunctionValue(matches[index]);
};

const channelFunctionLabel = (fn: NonNullable<AttributeControl["functions"]>[number]) =>
  (fn.name || fn.attribute || "Function").trim();

const channelFunctionRangeLabel = (fn: NonNullable<AttributeControl["functions"]>[number]) =>
  `${formatShortDmxPercent(fn.dmx_from)}-${formatShortDmxPercent(fn.dmx_to)}`;

const channelFunctionDetail = (fn: NonNullable<AttributeControl["functions"]>[number]) => {
  if (fn.wheel_slot_name) {
    const color = normalizeHexColor(fn.wheel_slot_color);
    return color ? `${fn.wheel_slot_name} ${color}` : fn.wheel_slot_name;
  }
  if (fn.wheel_slot) {
    return fn.wheel_slot;
  }
  if (fn.physical_from !== null && fn.physical_from !== undefined && fn.physical_to !== null && fn.physical_to !== undefined) {
    return `Phys ${Number(fn.physical_from).toFixed(2)}-${Number(fn.physical_to).toFixed(2)}`;
  }
  return fn.attribute;
};

const quickLookValueForControl = (
  category: ControlCategory,
  lookId: string,
  control: AttributeControl,
) => {
  const normalized = control.attribute.toLowerCase().replace(/[^a-z0-9]/g, "");
  if (category === "gobo") {
    const openValue = pickFunctionValue(control, (text) => /\b(open|clear|empty|none|white)\b/.test(text));
    const goboValue = (index: number) =>
      indexedFunctionValue(
        control,
        (text, fn) =>
          (Boolean(fn.wheel_slot) || /\b(gobo|slot|pattern|breakup)\b/.test(text)) &&
          !/\b(open|clear|empty|none|white|spin|rotate|rotation|shake)\b/.test(text),
        index,
      );
    if (lookId === "slot1") {
      return goboValue(0) ?? 8192;
    }
    if (lookId === "slot2") {
      return goboValue(1) ?? 16_384;
    }
    if (lookId === "spin") {
      return pickFunctionValue(control, (text) => /\b(spin|rotate|rotation|continuous|shake)\b/.test(text)) ?? 49_152;
    }
    return openValue ?? 0;
  }
  if (category === "focus") {
    if (lookId === "near") {
      return rankedFunctionValue(control, "first") ?? 0;
    }
    if (lookId === "mid") {
      return rankedFunctionValue(control, "middle") ?? 32_768;
    }
    if (lookId === "far") {
      return rankedFunctionValue(control, "last") ?? 65_535;
    }
    return control.default_value;
  }
  if (category === "beam") {
    const isShutter = /shutter/.test(normalized);
    const isStrobe = /strobe/.test(normalized);
    const isIris = /iris/.test(normalized);
    const isZoomOrBeam = /(zoom|beam|wash|spot)/.test(normalized);
    const isFrost = /frost/.test(normalized);
    const isPrism = /prism/.test(normalized);
    const openOrOff = () =>
      pickFunctionValue(control, (text) => /\b(open|off|none|disable|disabled|clear|home)\b/.test(text));
    const narrow = () => pickFunctionValue(control, (text) => /\b(tight|narrow|small|min|minimum)\b/.test(text));
    const wide = () => pickFunctionValue(control, (text) => /\b(wide|large|max|maximum|open)\b/.test(text));
    const soft = () => pickFunctionValue(control, (text) => /\b(soft|frost|diffusion|diffuse|on|enable)\b/.test(text));
    if (lookId === "open") {
      if (isShutter || isIris) {
        return openOrOff() ?? 65_535;
      }
      if (isStrobe || isFrost || isPrism) {
        return openOrOff() ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "tight") {
      if (isShutter) {
        return openOrOff() ?? 65_535;
      }
      if (isIris || isZoomOrBeam || isStrobe || isFrost || isPrism) {
        return (isIris || isZoomOrBeam ? narrow() : openOrOff()) ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "wide") {
      if (isShutter || isIris || isZoomOrBeam) {
        return (isIris || isZoomOrBeam ? wide() : openOrOff()) ?? 65_535;
      }
      if (isStrobe || isFrost || isPrism) {
        return openOrOff() ?? 0;
      }
      return control.default_value;
    }
    if (lookId === "soft") {
      if (isShutter || isIris || isZoomOrBeam || isFrost) {
        return (isFrost ? soft() : wide()) ?? 65_535;
      }
      if (isStrobe || isPrism) {
        return openOrOff() ?? 0;
      }
    }
  }
  return control.default_value;
};

const defaultPositionFavorites = (): PositionFavorite[] => [
  { id: "home", label: "Home", pan: 32768, tilt: 32768 },
  { id: "down", label: "Down", pan: 32768, tilt: 0 },
  { id: "up", label: "Up", pan: 32768, tilt: 65535 },
];

const positionFavoriteFromUnknown = (candidate: unknown): PositionFavorite | null => {
  if (!candidate || typeof candidate !== "object") {
    return null;
  }
  const source = candidate as Partial<PositionFavorite>;
  if (typeof source.id !== "string" || typeof source.label !== "string") {
    return null;
  }
  return {
    id: source.id.trim() || `position-${Date.now().toString(36)}`,
    label: source.label.trim().slice(0, 16) || "Position",
    pan: clampDmxValue(Number(source.pan)),
    tilt: clampDmxValue(Number(source.tilt)),
  };
};

const loadPositionFavorites = () => {
  if (typeof window === "undefined") {
    return defaultPositionFavorites();
  }
  try {
    const raw = window.localStorage.getItem(positionFavoritesStorageKey);
    if (!raw) {
      return defaultPositionFavorites();
    }
    const parsed = JSON.parse(raw);
    if (!Array.isArray(parsed)) {
      return defaultPositionFavorites();
    }
    const favorites = parsed
      .map(positionFavoriteFromUnknown)
      .filter((favorite): favorite is PositionFavorite => Boolean(favorite))
      .slice(0, 24);
    return favorites.length > 0 ? favorites : defaultPositionFavorites();
  } catch {
    return defaultPositionFavorites();
  }
};

const savePositionFavorites = (favorites: PositionFavorite[]) => {
  if (typeof window === "undefined") {
    return;
  }
  try {
    window.localStorage.setItem(positionFavoritesStorageKey, JSON.stringify(favorites.slice(0, 24)));
  } catch {
    // Local storage can be unavailable in hardened WebViews; the live palette still works in memory.
  }
};

const videoFrameToDataUrl = (frame: VideoFrame) => {
  if (frame.format !== "Rgba8") {
    throw new Error(`Unsupported preview format ${frame.format}`);
  }
  const expectedLength = frame.width * frame.height * 4;
  if (frame.width <= 0 || frame.height <= 0 || frame.data.length !== expectedLength) {
    throw new Error("Invalid preview frame size");
  }
  const canvas = document.createElement("canvas");
  canvas.width = frame.width;
  canvas.height = frame.height;
  const context = canvas.getContext("2d");
  if (!context) {
    throw new Error("Canvas 2D context is unavailable");
  }
  context.putImageData(
    new ImageData(new Uint8ClampedArray(frame.data), frame.width, frame.height),
    0,
    0,
  );
  return canvas.toDataURL("image/png");
};

const readVideoOutputWindowId = () => {
  const raw = new URLSearchParams(window.location.search).get("videoOutputId");
  if (!raw) {
    return null;
  }
  const outputId = Number(raw);
  return Number.isInteger(outputId) && outputId > 0 ? outputId : null;
};

const readVideoOutputTestPattern = () => new URLSearchParams(window.location.search).get("testPattern") === "1";

const videoSourceInputLabel = (kind: VideoSourceKind) => {
  switch (kind) {
    case "File":
    case "StillImage":
      return "Source path";
    case "Ndi":
      return "NDI source";
    case "Spout":
      return "Spout sender";
    case "Syphon":
      return "Syphon server";
  }
};

const videoSourceInputPlaceholder = (kind: VideoSourceKind) => {
  switch (kind) {
    case "File":
      return "C:\\path\\clip.mp4";
    case "StillImage":
      return "C:\\path\\image.png";
    case "Ndi":
      return "OBS / Program";
    case "Spout":
      return "Spout sender name";
    case "Syphon":
      return "Syphon server name";
  }
};

const videoSourceCanBrowseFile = (kind: VideoSourceKind) => kind === "File" || kind === "StillImage";

const mediaLabelFromPath = (path: string) => {
  const fileName = path.split(/[\\/]/).pop()?.trim() || path.trim();
  return fileName.replace(/\.[^/.]+$/, "") || fileName;
};

const shouldReplaceVideoLayerDraftLabel = (label: string) => {
  const trimmed = label.trim();
  return trimmed.length === 0 || /^Layer \d+$/i.test(trimmed);
};

const formatDuration = (durationMs?: number | null) => {
  if (!durationMs) {
    return null;
  }
  const totalSeconds = Math.round(durationMs / 1000);
  const minutes = Math.floor(totalSeconds / 60);
  const seconds = totalSeconds % 60;
  return `${minutes}:${seconds.toString().padStart(2, "0")}`;
};

const formatVideoTime = (positionMs: number, durationMs?: number | null) => {
  const position = formatDuration(Math.max(0, positionMs)) ?? "0:00";
  const duration = formatDuration(durationMs);
  return duration ? `${position} / ${duration}` : position;
};

const videoSourceMetadataLabel = (source: { codec?: string | null; metadata?: { duration_ms?: number | null; width?: number | null; height?: number | null; frame_rate?: number | null } | null }) => {
  const metadata = source.metadata;
  const parts = [
    source.codec,
    metadata?.width && metadata?.height ? `${metadata.width}x${metadata.height}` : null,
    metadata?.frame_rate ? `${metadata.frame_rate.toFixed(2)} fps` : null,
    formatDuration(metadata?.duration_ms),
  ].filter((part): part is string => Boolean(part));
  return parts.join(" / ");
};

const videoOutputWindowRenderSize = () => {
  const dpr = Math.min(Math.max(window.devicePixelRatio || 1, 1), 2);
  let width = Math.max(1, Math.floor(window.innerWidth * dpr));
  let height = Math.max(1, Math.floor(window.innerHeight * dpr));
  const maxPixels = 1280 * 720;
  if (width * height > maxPixels) {
    const scale = Math.sqrt(maxPixels / (width * height));
    width = Math.max(1, Math.floor(width * scale));
    height = Math.max(1, Math.floor(height * scale));
  }
  return { width, height };
};

function VideoOutputWindow(props: { outputId: number; testPattern: boolean }) {
  const [previewUrl, setPreviewUrl] = createSignal("");
  const [status, setStatus] = createSignal("Waiting for output frame");

  createEffect(() => {
    document.body.classList.add("outputBody");
    let cancelled = false;
    let timeoutId = 0;

    const renderFrame = async () => {
      const { width, height } = videoOutputWindowRenderSize();
      try {
        const frame = await invoke<VideoFrame>(props.testPattern ? "get_debug_video_output_test_pattern" : "get_debug_video_output_preview", {
          outputId: props.outputId,
          width,
          height,
        });
        if (!cancelled) {
          setPreviewUrl(videoFrameToDataUrl(frame));
          setStatus(`${props.testPattern ? "Test pattern" : "Output"} ${props.outputId} / ${frame.width}x${frame.height} / pts ${frame.pts_ms}ms`);
        }
      } catch (error) {
        if (!cancelled) {
          setStatus(String(error));
        }
      } finally {
        if (!cancelled) {
          timeoutId = window.setTimeout(renderFrame, 33);
        }
      }
    };

    void renderFrame();
    onCleanup(() => {
      cancelled = true;
      window.clearTimeout(timeoutId);
      document.body.classList.remove("outputBody");
    });
  });

  return (
    <main class="videoOutputWindow">
      <Show when={previewUrl()} fallback={<div class="videoOutputStatus">{status()}</div>}>
        {(url) => <img class="videoOutputFrame" src={url()} alt={`Video output ${props.outputId}`} />}
      </Show>
    </main>
  );
}

const beamPoints = (x: number, z: number, yawDegrees: number, intensity: number) => {
  const yaw = (yawDegrees * Math.PI) / 180;
  const angle = -Math.PI / 2 + yaw;
  const beamLength = 18 + intensity * 34;
  const beamWidth = 5 + intensity * 15;
  const tipX = x + Math.cos(angle) * beamLength;
  const tipZ = z + Math.sin(angle) * beamLength;
  const leftX = tipX + Math.cos(angle + Math.PI / 2) * beamWidth;
  const leftZ = tipZ + Math.sin(angle + Math.PI / 2) * beamWidth;
  const rightX = tipX + Math.cos(angle - Math.PI / 2) * beamWidth;
  const rightZ = tipZ + Math.sin(angle - Math.PI / 2) * beamWidth;
  return `${x},${z} ${leftX},${leftZ} ${rightX},${rightZ}`;
};

export default function App() {
  const outputWindowId = readVideoOutputWindowId();
  if (outputWindowId !== null) {
    return <VideoOutputWindow outputId={outputWindowId} testPattern={readVideoOutputTestPattern()} />;
  }

  const setupPanelRefs: Partial<Record<SetupSubTab, HTMLElement>> = {};
  const [gdtfPath, setGdtfPath] = createSignal("");
  const [gdtfShareUrl, setGdtfShareUrl] = createSignal("");
  const [currentProjectPath, setCurrentProjectPath] = createSignal<string | null>(null);
  const [projectDirty, setProjectDirty] = createSignal(false);
  const [cleanProjectSignature, setCleanProjectSignature] = createSignal<string | null>(null);
  const [workspaceTab, setWorkspaceTab] = createSignal<WorkspaceTab>("setup");
  const [setupSubTab, setSetupSubTab] = createSignal<SetupSubTab>("patch");
  const [profile, setProfile] = createSignal<FixtureProfileSummary | null>(null);
  const [selectedMode, setSelectedMode] = createSignal("");
  const [customManufacturer, setCustomManufacturer] = createSignal("Rayard");
  const [customProfileName, setCustomProfileName] = createSignal("Custom Fixture");
  const [customModeName, setCustomModeName] = createSignal("Default");
  const [customAttributes, setCustomAttributes] = createSignal("Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8, ColorGreen@7:8, ColorBlue@8:8");
  const [label, setLabel] = createSignal("Fixture 1");
  const [address, setAddress] = createSignal(1);
  const [universe, setUniverse] = createSignal(0);
  const [patchCount, setPatchCount] = createSignal(1);
  const [patchAddressStride, setPatchAddressStride] = createSignal(0);
  const [patchLayoutMode, setPatchLayoutMode] = createSignal<FixtureLayoutMode>("line");
  const [patchGridColumns, setPatchGridColumns] = createSignal(4);
  const [patchCircleRadius, setPatchCircleRadius] = createSignal(4);
  const [patchX, setPatchX] = createSignal(0);
  const [patchY, setPatchY] = createSignal(0);
  const [patchZ, setPatchZ] = createSignal(0);
  const [patchXStep, setPatchXStep] = createSignal(1);
  const [patchZStep, setPatchZStep] = createSignal(0);
  const [patchPitch, setPatchPitch] = createSignal(0);
  const [patchYaw, setPatchYaw] = createSignal(0);
  const [patchRoll, setPatchRoll] = createSignal(0);
  const [groupText, setGroupText] = createSignal("");
  const [selectedFixtureGroupFilter, setSelectedFixtureGroupFilter] = createSignal<string | null>(null);
  const [selectedFixtureLabelDraft, setSelectedFixtureLabelDraft] = createSignal("");
  const [selectedFixtureUniverseDraft, setSelectedFixtureUniverseDraft] = createSignal(0);
  const [selectedFixtureAddressDraft, setSelectedFixtureAddressDraft] = createSignal(1);
  const [selectedFixtureGroupText, setSelectedFixtureGroupText] = createSignal("");
  const [selectedFixtureLimitsDraft, setSelectedFixtureLimitsDraft] = createSignal<FixtureLimits>(defaultFixtureLimits);
  const [movementLimitDrag, setMovementLimitDrag] = createSignal<MovementLimitDragState | null>(null);
  const [selectedFixtureId, setSelectedFixtureId] = createSignal<number | null>(null);
  const [selectedMappingFixtureIds, setSelectedMappingFixtureIds] = createSignal<number[]>([]);
  const [mappingSelectionGroupText, setMappingSelectionGroupText] = createSignal("");
  const [mappingStageTool, setMappingStageTool] = createSignal<MappingStageTool>("select");
  const [mappingViewportZoom, setMappingViewportZoom] = createSignal(1);
  const [mappingViewportCenterX, setMappingViewportCenterX] = createSignal(stageViewBoxSize / 2);
  const [mappingViewportCenterZ, setMappingViewportCenterZ] = createSignal(stageViewBoxSize / 2);
  const [mappingSnapEnabled, setMappingSnapEnabled] = createSignal(false);
  const [mappingSnapSize, setMappingSnapSize] = createSignal(0.5);
  const [mappingFixtureSearch, setMappingFixtureSearch] = createSignal("");
  const [selectedFixtureTypeFilter, setSelectedFixtureTypeFilter] = createSignal<string | null>(null);
  const [selectedVideoOutputId, setSelectedVideoOutputId] = createSignal<number | null>(null);
  const [mappingShowLabels, setMappingShowLabels] = createSignal(true);
  const [mappingShowBeams, setMappingShowBeams] = createSignal(true);
  const [mappingShowProjectors, setMappingShowProjectors] = createSignal(true);
  const [mappingDrag, setMappingDrag] = createSignal<MappingDragState | null>(null);
  const [mappingMarquee, setMappingMarquee] = createSignal<MappingMarqueeState | null>(null);
  const [mappingViewportPanDrag, setMappingViewportPanDrag] = createSignal<MappingViewportPanDragState | null>(null);
  const [patchGridUniverse, setPatchGridUniverse] = createSignal(0);
  const [dmxPatchViewMode, setDmxPatchViewMode] = createSignal<DmxPatchViewMode>("grid");
  const [controlCategory, setControlCategory] = createSignal<ControlCategory>("position");
  const [panTiltNudgeAmount, setPanTiltNudgeAmount] = createSignal(2048);
  const [positionFavorites, setPositionFavorites] = createSignal<PositionFavorite[]>(loadPositionFavorites());
  const [colorFavorites, setColorFavorites] = createSignal<string[]>(loadColorFavorites());
  const [faderValues, setFaderValues] = createSignal<Record<string, number>>({});
  const [rawDmxUniverse, setRawDmxUniverse] = createSignal(0);
  const [bpmDraft, setBpmDraft] = createSignal("120");
  const [midiInputs, setMidiInputs] = createSignal<MidiInputSummary[]>([]);
  const [midiOutputs, setMidiOutputs] = createSignal<MidiOutputSummary[]>([]);
  const [selectedMidiInput, setSelectedMidiInput] = createSignal<number | null>(null);
  const [selectedMidiOutput, setSelectedMidiOutput] = createSignal<number | null>(null);
  const [midiConnected, setMidiConnected] = createSignal(false);
  const [midiControlConnected, setMidiControlConnected] = createSignal(false);
  const [midiFeedbackConnected, setMidiFeedbackConnected] = createSignal(false);
  const [midiFeedbackEnabled, setMidiFeedbackEnabled] = createSignal(false);
  const [midiMappings, setMidiMappings] = createSignal<MidiControlMapping[]>([]);
  const [midiMapMessage, setMidiMapMessage] = createSignal<MidiControlMessage>("ControlChange");
  const [midiMapChannel, setMidiMapChannel] = createSignal(-1);
  const [midiMapNumber, setMidiMapNumber] = createSignal(7);
  const [midiMapAction, setMidiMapAction] = createSignal<MidiControlAction>("FixtureAttribute");
  const [midiMapAttribute, setMidiMapAttribute] = createSignal("Dimmer");
  const [midiMapGroupId, setMidiMapGroupId] = createSignal("front");
  const [midiMapCueId, setMidiMapCueId] = createSignal<number | null>(null);
  const [midiMapLayerId, setMidiMapLayerId] = createSignal<number | null>(null);
  const [midiMapVideoOutputId, setMidiMapVideoOutputId] = createSignal<number | null>(null);
  const [midiMapVideoParam, setMidiMapVideoParam] = createSignal<VideoParam>("Opacity");
  const [midiMapCuePointIndex, setMidiMapCuePointIndex] = createSignal(0);
  const [midiMapDurationMs, setMidiMapDurationMs] = createSignal(1000);
  const [midiMapLow, setMidiMapLow] = createSignal(0);
  const [midiMapHigh, setMidiMapHigh] = createSignal(65535);
  const [serialPorts, setSerialPorts] = createSignal<SerialPortSummary[]>([]);
  const [oscBindIp, setOscBindIp] = createSignal("0.0.0.0");
  const [oscPort, setOscPort] = createSignal(9000);
  const [oscRunning, setOscRunning] = createSignal(false);
  const [oscMappings, setOscMappings] = createSignal<OscControlMapping[]>([]);
  const [oscMapAddress, setOscMapAddress] = createSignal("/touchosc/fader1");
  const [oscMapAction, setOscMapAction] = createSignal<OscControlAction>("FixtureAttribute");
  const [oscMapAttribute, setOscMapAttribute] = createSignal("Dimmer");
  const [oscMapGroupId, setOscMapGroupId] = createSignal("front");
  const [oscMapCueId, setOscMapCueId] = createSignal<number | null>(null);
  const [oscMapLayerId, setOscMapLayerId] = createSignal<number | null>(null);
  const [oscMapVideoOutputId, setOscMapVideoOutputId] = createSignal<number | null>(null);
  const [oscMapVideoParam, setOscMapVideoParam] = createSignal<VideoParam>("Opacity");
  const [oscMapCuePointIndex, setOscMapCuePointIndex] = createSignal(0);
  const [oscMapDurationMs, setOscMapDurationMs] = createSignal(1000);
  const [oscMapLow, setOscMapLow] = createSignal(0);
  const [oscMapHigh, setOscMapHigh] = createSignal(65535);
  const [remoteBindIp, setRemoteBindIp] = createSignal("0.0.0.0");
  const [remotePort, setRemotePort] = createSignal(9100);
  const [remoteRunning, setRemoteRunning] = createSignal(false);
  const [cueLabel, setCueLabel] = createSignal("Cue 1");
  const [cueFadeMs, setCueFadeMs] = createSignal(1000);
  const [cueCaptureScope, setCueCaptureScope] = createSignal<CueCaptureScopeMode>("all");
  const [cueMetadataDrafts, setCueMetadataDrafts] = createSignal<Record<number, CueMetadataDraft>>({});
  const [cuePadBank, setCuePadBank] = createSignal(0);
  const [cuePadFollowActive, setCuePadFollowActive] = createSignal(true);
  const [timelineCueId, setTimelineCueId] = createSignal<number | null>(null);
  const [timelineEventTimeMs, setTimelineEventTimeMs] = createSignal(0);
  const [timelineTrack, setTimelineTrack] = createSignal<TimelineTrackKind>("Lighting");
  const [timelineEventDrafts, setTimelineEventDrafts] = createSignal<Record<number, TimelineEventDraft>>({});
  const [timelineSnapMode, setTimelineSnapMode] = createSignal<TimelineSnapMode>("Off");
  const [timelineGridMs, setTimelineGridMs] = createSignal(500);
  const [timelineAutomationDrafts, setTimelineAutomationDrafts] = createSignal<Record<number, TimelineAutomationDraft>>({});
  const [automationStartMs, setAutomationStartMs] = createSignal(0);
  const [automationEndMs, setAutomationEndMs] = createSignal(1000);
  const [automationStartValue, setAutomationStartValue] = createSignal(0);
  const [automationEndValue, setAutomationEndValue] = createSignal(65535);
  const [automationInterpolation, setAutomationInterpolation] = createSignal<AutomationInterpolation>("Linear");
  const [videoLabel, setVideoLabel] = createSignal("Layer 1");
  const [videoSourceKind, setVideoSourceKind] = createSignal<VideoSourceKind>("File");
  const [videoPath, setVideoPath] = createSignal("");
  const [videoPreviewInfo, setVideoPreviewInfo] = createSignal("No preview");
  const [videoPreviewUrl, setVideoPreviewUrl] = createSignal("");
  const [videoPreviewDiagnostics, setVideoPreviewDiagnostics] = createSignal<VideoPreviewDiagnostics | null>(null);
  const [videoOutputPreviewInfo, setVideoOutputPreviewInfo] = createSignal("No output preview");
  const [videoOutputPreviewUrl, setVideoOutputPreviewUrl] = createSignal("");
  const [videoOutputPreviewId, setVideoOutputPreviewId] = createSignal<number | null>(null);
  const [videoOutputPreviewMode, setVideoOutputPreviewMode] = createSignal<VideoOutputPreviewMode>("output");
  const [videoOutputLabel, setVideoOutputLabel] = createSignal("Projector 1");
  const [videoOutputKind, setVideoOutputKind] = createSignal<VideoOutputKind>("Display");
  const [videoOutputWidth, setVideoOutputWidth] = createSignal(1920);
  const [videoOutputHeight, setVideoOutputHeight] = createSignal(1080);
  const [videoOutputFullscreen, setVideoOutputFullscreen] = createSignal(true);
  const [videoOutputMonitorId, setVideoOutputMonitorId] = createSignal(0);
  const [videoOutputEndpoint, setVideoOutputEndpoint] = createSignal("");
  const [videoOutputConfigDrafts, setVideoOutputConfigDrafts] = createSignal<Record<number, VideoOutputConfigDraft>>({});
  const [videoOutputFadeMs, setVideoOutputFadeMs] = createSignal(1000);
  const [videoOutputMappingPresetLabel, setVideoOutputMappingPresetLabel] = createSignal("Projector preset");
  const [selectedVideoOutputMappingPresetLabel, setSelectedVideoOutputMappingPresetLabel] = createSignal("");
  const [stageMapPresetLabel, setStageMapPresetLabel] = createSignal("Stage map preset");
  const [selectedStageMapPresetLabel, setSelectedStageMapPresetLabel] = createSignal("");
  const [videoCompositionLabel, setVideoCompositionLabel] = createSignal("Composition 1");
  const [videoCompositionLayerIds, setVideoCompositionLayerIds] = createSignal<number[]>([]);
  const [videoAutomationLayerId, setVideoAutomationLayerId] = createSignal<number | null>(null);
  const [videoAutomationParam, setVideoAutomationParam] = createSignal<VideoParam>("Opacity");
  const [videoAutomationStartMs, setVideoAutomationStartMs] = createSignal(0);
  const [videoAutomationEndMs, setVideoAutomationEndMs] = createSignal(1000);
  const [videoAutomationStartValue, setVideoAutomationStartValue] = createSignal(1);
  const [videoAutomationEndValue, setVideoAutomationEndValue] = createSignal(0);
  const [videoAutomationInterpolation, setVideoAutomationInterpolation] =
    createSignal<AutomationInterpolation>("Linear");
  const [timelineVideoAutomationDrafts, setTimelineVideoAutomationDrafts] = createSignal<
    Record<number, TimelineVideoAutomationDraft>
  >({});
  const [effectShape, setEffectShape] = createSignal<LfoShape>("Sine");
  const [effectType, setEffectType] = createSignal<EffectKind>("Lfo");
  const [effectTargetMode, setEffectTargetMode] = createSignal<"fixture" | "group" | "video">("fixture");
  const [effectTargetGroups, setEffectTargetGroups] = createSignal("");
  const [effectVideoLayerId, setEffectVideoLayerId] = createSignal<number | null>(null);
  const [effectVideoParam, setEffectVideoParam] = createSignal<VideoParam>("Opacity");
  const [effectVideoLow, setEffectVideoLow] = createSignal(0);
  const [effectVideoHigh, setEffectVideoHigh] = createSignal(1);
  const [effectVideoPositionX, setEffectVideoPositionX] = createSignal(0);
  const [effectVideoPositionY, setEffectVideoPositionY] = createSignal(0);
  const [effectVideoPositionZ, setEffectVideoPositionZ] = createSignal(0);
  const [effectPeriod, setEffectPeriod] = createSignal(1000);
  const [effectLow, setEffectLow] = createSignal(0);
  const [effectHigh, setEffectHigh] = createSignal(65535);
  const [effectPhase, setEffectPhase] = createSignal(0);
  const [effectBlendMode, setEffectBlendMode] = createSignal<EffectBlendMode>("Override");
  const [effectAttribute, setEffectAttribute] = createSignal("");
  const [waveOriginX, setWaveOriginX] = createSignal(0);
  const [waveOriginY, setWaveOriginY] = createSignal(0);
  const [waveOriginZ, setWaveOriginZ] = createSignal(0);
  const [waveDirectionX, setWaveDirectionX] = createSignal(1);
  const [waveDirectionY, setWaveDirectionY] = createSignal(0);
  const [waveDirectionZ, setWaveDirectionZ] = createSignal(0);
  const [waveStageDrag, setWaveStageDrag] = createSignal<WaveStageDragMode | null>(null);
  const [waveSpeed, setWaveSpeed] = createSignal(1);
  const [waveWavelength, setWaveWavelength] = createSignal(2);
  const [snapshot, setSnapshot] = createSignal<EngineSnapshot>({
    fixtures: [],
    cues: [],
    active_cue_id: null,
    active_fade: null,
    timeline: {
      events: [],
      automations: [],
      video_automations: [],
      audio: null,
      playing: false,
      position_ms: 0,
      duration_ms: 0,
    },
    video: {
      layers: [],
      compositions: [],
      outputs: [],
      mapping_presets: [],
      master_opacity: 1,
      blackout: false,
    },
    effects: [],
    node_graphs: [],
    output: defaultOutput,
    dmx_outputs: [defaultOutput],
    lighting_master: 1,
    submasters: [],
    blackout: false,
    clock: {
      bpm: 120,
      beat_phase: 0,
      beat_counter: 0,
      tap_count: 0,
      source: "Manual",
    },
    stage_map: {
      locked: false,
      min_x: -10,
      max_x: 10,
      min_z: -10,
      max_z: 10,
    },
    stage_map_presets: [],
    dmx_preview: Array.from({ length: 512 }, () => 0),
    dmx_previews: [{ universe: 0, values: Array.from({ length: 512 }, () => 0) }],
    telemetry: {
      frame_counter: 0,
      queue_depth: 0,
      queue_depth_abs_max: 0,
      queue_push_failure_count: 0,
      last_tick_interval_us: 0,
      tick_jitter_last_us: 0,
      tick_jitter_abs_max_us: 0,
      tick_jitter_stddev_us: 0,
      tick_jitter_p95_us: 0,
      tick_jitter_p99_us: 0,
      tick_jitter_samples: 0,
      last_command_queue_latency_us: 0,
      command_queue_latency_abs_max_us: 0,
      command_queue_latency_p95_us: 0,
      command_queue_latency_p99_us: 0,
      command_queue_latency_samples: 0,
      last_command_drain_count: 0,
      command_drain_abs_max: 0,
      command_drain_limit_hit_count: 0,
      last_command_to_dmx_tick_latency_us: 0,
      command_to_dmx_tick_latency_abs_max_us: 0,
      command_to_dmx_tick_latency_p95_us: 0,
      command_to_dmx_tick_latency_p99_us: 0,
      command_to_dmx_tick_latency_samples: 0,
      last_dmx_send_interval_us: 0,
      dmx_send_interval_min_us: 0,
      dmx_send_interval_max_us: 0,
      dmx_send_interval_samples: 0,
      low_latency_dmx_tick_request_count: 0,
      low_latency_dmx_tick_advance_count: 0,
      low_latency_dmx_tick_defer_count: 0,
      last_packet_bytes: 0,
      last_dmx_output_count: 0,
      last_dmx_send_success_count: 0,
      last_dmx_send_failure_count: 0,
      total_dmx_send_success_count: 0,
      total_dmx_send_failure_count: 0,
      last_dmx_route_results: [],
      last_error: null,
    },
  });
  const [output, setOutput] = createSignal<DmxOutputConfig>(defaultOutput);
  const [dmxOutputRoutes, setDmxOutputRoutes] = createSignal<DmxOutputConfig[]>([defaultOutput]);
  const [engineTelemetryReport, setEngineTelemetryReport] = createSignal<EngineTelemetryReport | null>(null);
  const [dmxTestChannel, setDmxTestChannel] = createSignal(1);
  const [dmxTestWidth, setDmxTestWidth] = createSignal(1);
  const [dmxTestValue, setDmxTestValue] = createSignal(255);
  const audioAnalysis = createMemo<AudioAnalysisSummary | null>(() => snapshot().timeline.audio ?? null);
  const [message, setMessage] = createSignal("Ready");

  createEffect(() => {
    savePositionFavorites(positionFavorites());
  });

  createEffect(() => {
    saveColorFavorites(colorFavorites());
  });

  const selectedFixture = createMemo<PatchedFixtureSummary | undefined>(() =>
    snapshot().fixtures.find((fixture) => fixture.id === selectedFixtureId()),
  );
  const normalizedSelectedFixtureLimitsDraft = createMemo<FixtureLimits>(() => {
    const limits = selectedFixtureLimitsDraft();
    const dimmer = normalizeLimitRange(limits.dimmer_min, limits.dimmer_max);
    const pan = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tilt = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    return {
      ...limits,
      dimmer_min: dimmer.min,
      dimmer_max: dimmer.max,
      pan_min: pan.min,
      pan_max: pan.max,
      tilt_min: tilt.min,
      tilt_max: tilt.max,
    };
  });
  const selectedFixtureLimitWindowStyle = createMemo(() => {
    const limits = normalizedSelectedFixtureLimitsDraft();
    const panWidth = ((limits.pan_max - limits.pan_min) / 65_535) * 100;
    const tiltHeight = ((limits.tilt_max - limits.tilt_min) / 65_535) * 100;
    return {
      left: `${(limits.pan_min / 65_535) * 100}%`,
      width: `${Math.max(1.5, panWidth)}%`,
      top: `${((65_535 - limits.tilt_max) / 65_535) * 100}%`,
      height: `${Math.max(1.5, tiltHeight)}%`,
    };
  });
  const selectedFixtureLimits = createMemo(() => selectedFixture()?.limits ?? defaultFixtureLimits);
  const selectedFixtureLimitOverlayStyle = createMemo(() => {
    const limits = selectedFixtureLimits();
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    const panWidth = ((panRange.max - panRange.min) / 65_535) * 100;
    const tiltHeight = ((tiltRange.max - tiltRange.min) / 65_535) * 100;
    return {
      left: `${(panRange.min / 65_535) * 100}%`,
      width: `${Math.max(1.5, panWidth)}%`,
      top: `${((65_535 - tiltRange.max) / 65_535) * 100}%`,
      height: `${Math.max(1.5, tiltHeight)}%`,
    };
  });
  const fixtureGroupRows = createMemo(() => {
    const counts = new Map<string, number>();
    for (const fixture of snapshot().fixtures) {
      for (const groupId of fixture.group_ids) {
        counts.set(groupId, (counts.get(groupId) ?? 0) + 1);
      }
    }
    return [...counts.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([groupId, count]) => ({ groupId, count }));
  });
  const filteredFixtures = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    if (!groupId) {
      return snapshot().fixtures;
    }
    return snapshot().fixtures.filter((fixture) => fixture.group_ids.includes(groupId));
  });
  const fixtureTypeRows = createMemo<MappingFixtureTypeRow[]>(() => {
    const rows = new Map<string, MappingFixtureTypeRow>();
    for (const fixture of filteredFixtures()) {
      const key = fixtureTypeKey(fixture);
      const current = rows.get(key);
      if (current) {
        current.count += 1;
      } else {
        rows.set(key, {
          key,
          label: fixtureTypeLabel(fixture),
          manufacturer: fixture.manufacturer,
          mode: fixture.mode_name,
          visualKind: fixtureVisualKind(fixture),
          count: 1,
        });
      }
    }
    return [...rows.values()].sort((left, right) => left.label.localeCompare(right.label));
  });
  const fixtureMatchesMappingSearch = (fixture: PatchedFixtureSummary, search: string) => {
    if (!search) {
      return true;
    }
    const statusText = [
      fixture.highlighted ? "highlight" : "",
      fixture.soloed ? "solo" : "",
      fixture.parked ? "park" : "",
    ].filter(Boolean).join(" ");
    const haystack = [
      fixture.label,
      fixture.manufacturer,
      fixture.profile_name,
      fixture.mode_name,
      `u${fixture.universe}`,
      `a${fixture.address}`,
      `u${fixture.universe} a${fixture.address}`,
      fixture.group_ids.join(" "),
      statusText,
    ].join(" ").toLowerCase();
    return haystack.includes(search);
  };
  const mappingFilteredFixtures = createMemo(() => {
    const typeKey = selectedFixtureTypeFilter();
    const search = mappingFixtureSearch().trim().toLowerCase();
    return filteredFixtures().filter((fixture) => {
      if (typeKey && fixtureTypeKey(fixture) !== typeKey) {
        return false;
      }
      return fixtureMatchesMappingSearch(fixture, search);
    });
  });
  const selectedMappingFixtureIdSet = createMemo(() => new Set(selectedMappingFixtureIds()));
  const selectedMappingFixtures = createMemo(() => {
    const selectedIds = selectedMappingFixtureIdSet();
    return snapshot().fixtures.filter((fixture) => selectedIds.has(fixture.id));
  });
  const selectedMappingFlagState = createMemo(() => {
    const fixtures = selectedMappingFixtures();
    const count = fixtures.length;
    return {
      count,
      anyHighlighted: fixtures.some((fixture) => fixture.highlighted),
      allHighlighted: count > 0 && fixtures.every((fixture) => fixture.highlighted),
      anySoloed: fixtures.some((fixture) => fixture.soloed),
      allSoloed: count > 0 && fixtures.every((fixture) => fixture.soloed),
      anyParked: fixtures.some((fixture) => fixture.parked),
      allParked: count > 0 && fixtures.every((fixture) => fixture.parked),
    };
  });
  const selectedMappingFixture = createMemo(() => {
    const selectedId = selectedFixtureId();
    if (selectedId === null || !selectedMappingFixtureIdSet().has(selectedId)) {
      return null;
    }
    return snapshot().fixtures.find((fixture) => fixture.id === selectedId) ?? null;
  });
  const selectedGroupFixtures = createMemo(() => {
    if (!selectedFixtureGroupFilter()) {
      return [];
    }
    return filteredFixtures();
  });
  const selectedGroupFlagState = createMemo(() => {
    const fixtures = selectedGroupFixtures();
    return {
      count: fixtures.length,
      anyHighlighted: fixtures.some((fixture) => fixture.highlighted),
      anySoloed: fixtures.some((fixture) => fixture.soloed),
      anyParked: fixtures.some((fixture) => fixture.parked),
    };
  });
  const selectedGroupSubmaster = createMemo(() => {
    const groupId = selectedFixtureGroupFilter();
    if (!groupId) {
      return undefined;
    }
    return snapshot().submasters.find((submaster) => submaster.group_id === groupId);
  });

  const selectedModeSummary = createMemo(() => {
    const imported = profile();
    if (!imported) {
      return undefined;
    }
    return imported.dmx_modes.find((mode) => mode.name === selectedMode()) ?? imported.dmx_modes[0];
  });

  const selectedFootprint = createMemo(() => {
    const mode = selectedModeSummary();
    if (!mode) {
      return 0;
    }
    return Math.max(0, ...mode.controls.flatMap((control) => control.offsets));
  });

  const fixtureFootprint = (fixture: PatchedFixtureSummary) =>
    Math.max(0, ...fixture.controls.flatMap((control) => control.offsets));
  const addressRange = (start: number, footprint: number): [number, number] | null => {
    if (!Number.isFinite(start) || !Number.isFinite(footprint) || start < 1 || footprint < 1) {
      return null;
    }
    return [start, start + footprint - 1];
  };
  const rangesOverlap = (first: [number, number], second: [number, number]) =>
    first[0] <= second[1] && second[0] <= first[1];
  const patchCountValue = createMemo(() => Math.min(256, Math.max(1, Math.floor(patchCount() || 1))));
  const patchAddressStrideValue = createMemo(() => {
    const manualStride = Math.floor(patchAddressStride() || 0);
    return manualStride > 0 ? manualStride : Math.max(selectedFootprint(), 1);
  });
  const patchAddressRanges = createMemo(() =>
    Array.from({ length: patchCountValue() }, (_, index) => {
      const start = address() + index * patchAddressStrideValue();
      return {
        index,
        start,
        range: addressRange(start, selectedFootprint()),
      };
    }),
  );
  const endAddress = createMemo(() => patchAddressRanges().at(-1)?.range?.[1] ?? address());
  const patchAddressConflictText = createMemo(() => {
    const ranges = patchAddressRanges();
    for (const candidate of ranges) {
      if (!candidate.range) {
        continue;
      }
      const conflict = snapshot().fixtures.find((fixture) => {
        if (fixture.universe !== universe()) {
          return false;
        }
        const existingRange = addressRange(fixture.address, fixtureFootprint(fixture));
        return existingRange ? rangesOverlap(candidate.range!, existingRange) : false;
      });
      if (conflict) {
        const conflictRange = addressRange(conflict.address, fixtureFootprint(conflict));
        const rangeText = conflictRange ? `A${conflictRange[0]}-${conflictRange[1]}` : `A${conflict.address}`;
        return `Fixture ${candidate.index + 1} overlaps ${conflict.label} (${rangeText})`;
      }
      for (const existing of ranges.slice(0, candidate.index)) {
        if (existing.range && rangesOverlap(candidate.range, existing.range)) {
          return `Fixture ${candidate.index + 1} overlaps new fixture ${existing.index + 1} (A${existing.range[0]}-${existing.range[1]})`;
        }
      }
    }
    return "";
  });
  const patchAddressInvalid = createMemo(() => selectedFootprint() === 0 || endAddress() > 512 || Boolean(patchAddressConflictText()));
  const canPatchAtAddress = (startAddress: number, targetUniverse: number) => {
    if (selectedFootprint() <= 0 || startAddress < 1) {
      return false;
    }
    const ranges = Array.from({ length: patchCountValue() }, (_, index) => {
      const start = startAddress + index * patchAddressStrideValue();
      return addressRange(start, selectedFootprint());
    });
    if (ranges.some((range) => !range || range[1] > 512)) {
      return false;
    }
    for (let index = 0; index < ranges.length; index += 1) {
      const range = ranges[index];
      if (!range) {
        return false;
      }
      for (let previousIndex = 0; previousIndex < index; previousIndex += 1) {
        const previousRange = ranges[previousIndex];
        if (previousRange && rangesOverlap(range, previousRange)) {
          return false;
        }
      }
      const conflictsExisting = snapshot().fixtures.some((fixture) => {
        if (fixture.universe !== targetUniverse) {
          return false;
        }
        const existingRange = addressRange(fixture.address, fixtureFootprint(fixture));
        return existingRange ? rangesOverlap(range, existingRange) : false;
      });
      if (conflictsExisting) {
        return false;
      }
    }
    return true;
  };
  const nextFreePatchAddress = createMemo(() => {
    const targetUniverse = universe();
    for (let candidate = 1; candidate <= 512; candidate += 1) {
      if (canPatchAtAddress(candidate, targetUniverse)) {
        return candidate;
      }
    }
    return null;
  });
  const patchGridColumnsValue = createMemo(() => Math.min(64, Math.max(1, Math.floor(patchGridColumns() || 1))));
  const patchCircleRadiusValue = createMemo(() => Math.max(0.1, Number.isFinite(patchCircleRadius()) ? patchCircleRadius() : 4));
  const patchFixturePosition = (index: number, count: number) => {
    if (patchLayoutMode() === "grid") {
      const column = index % patchGridColumnsValue();
      const row = Math.floor(index / patchGridColumnsValue());
      return {
        x: patchX() + column * patchXStep(),
        y: patchY(),
        z: patchZ() + row * patchZStep(),
      };
    }
    if (patchLayoutMode() === "circle" && count > 1) {
      const angle = (index / count) * Math.PI * 2 - Math.PI / 2;
      const radius = patchCircleRadiusValue();
      return {
        x: patchX() + Math.cos(angle) * radius,
        y: patchY(),
        z: patchZ() + Math.sin(angle) * radius,
      };
    }
    return {
      x: patchX() + index * patchXStep(),
      y: patchY(),
      z: patchZ() + index * patchZStep(),
    };
  };
  const dmxUniverseMaps = createMemo<DmxUniverseMap[]>(() => {
    const universes = new Map<number, DmxPatchSegment[]>();
    for (const fixture of snapshot().fixtures) {
      const footprint = fixtureFootprint(fixture);
      const range = addressRange(fixture.address, footprint);
      if (!range) {
        continue;
      }
      const segments = universes.get(fixture.universe) ?? [];
      segments.push({
        fixture,
        start: range[0],
        end: Math.min(512, range[1]),
        left: ((range[0] - 1) / 512) * 100,
        width: ((Math.min(512, range[1]) - range[0] + 1) / 512) * 100,
      });
      universes.set(fixture.universe, segments);
    }
    if (!universes.has(universe())) {
      universes.set(universe(), []);
    }

    return Array.from(universes.entries())
      .sort(([firstUniverse], [secondUniverse]) => firstUniverse - secondUniverse)
      .map(([universeId, segments]) => {
        const sortedSegments = [...segments].sort((first, second) => first.start - second.start);
        let cursor = 1;
        let largestFree = 0;
        for (const segment of sortedSegments) {
          if (segment.start > cursor) {
            largestFree = Math.max(largestFree, segment.start - cursor);
          }
          cursor = Math.max(cursor, segment.end + 1);
        }
        largestFree = Math.max(largestFree, 513 - cursor);
        const used = sortedSegments.reduce((total, segment) => total + Math.max(0, segment.end - segment.start + 1), 0);
        return {
          universe: universeId,
          used,
          free: Math.max(0, 512 - used),
          largestFree,
          segments: sortedSegments,
        };
      });
  });

  const patchGridUniverseOptions = createMemo(() => {
    const universeIds = new Set<number>([universe(), output().universe, patchGridUniverse()]);
    for (const route of dmxOutputRoutes()) {
      universeIds.add(route.universe);
    }
    for (const fixture of snapshot().fixtures) {
      universeIds.add(fixture.universe);
    }
    return [...universeIds].sort((first, second) => first - second);
  });

  const activePatchGridUniverse = createMemo(() => {
    const options = patchGridUniverseOptions();
    return options.includes(patchGridUniverse()) ? patchGridUniverse() : (options[0] ?? 0);
  });

  createEffect(() => {
    const activeUniverse = activePatchGridUniverse();
    if (activeUniverse !== patchGridUniverse()) {
      setPatchGridUniverse(activeUniverse);
    }
  });

  const activePatchGridMap = createMemo<DmxUniverseMap>(() => {
    const activeUniverse = activePatchGridUniverse();
    return (
      dmxUniverseMaps().find((map) => map.universe === activeUniverse) ?? {
        universe: activeUniverse,
        used: 0,
        free: 512,
        largestFree: 512,
        segments: [],
      }
    );
  });

  const patchPlannedRanges = createMemo(() =>
    universe() === activePatchGridUniverse()
      ? patchAddressRanges().filter((candidate): candidate is { index: number; start: number; range: [number, number] } =>
          Boolean(candidate.range),
        )
      : [],
  );
  const plannedAddressSummary = createMemo(() => {
    const ranges = patchPlannedRanges();
    if (ranges.length === 0) {
      return "No pending patch in this universe";
    }
    const first = ranges[0].range;
    const last = ranges[ranges.length - 1].range;
    return ranges.length === 1 ? `Pending A${first[0]}-${first[1]}` : `Pending A${first[0]}-${last[1]} (${ranges.length} fixtures)`;
  });

  const dmxAddressCells = createMemo<DmxAddressCell[]>(() => {
    const map = activePatchGridMap();
    const selectedId = selectedFixtureId();
    const plannedRanges = patchPlannedRanges();
    return Array.from({ length: 512 }, (_, index) => {
      const channel = index + 1;
      const segment = map.segments.find((candidate) => channel >= candidate.start && channel <= candidate.end) ?? null;
      const planned = plannedRanges.find((candidate) => channel >= candidate.range[0] && channel <= candidate.range[1]);
      return {
        channel,
        segment,
        isStart: segment?.start === channel,
        isSelected: Boolean(segment && segment.fixture.id === selectedId),
        plannedIndex: planned?.index ?? null,
        plannedStart: planned?.range[0] === channel,
        plannedConflict: Boolean(planned && segment),
      };
    });
  });

  const activeControls = createMemo(() => selectedFixture()?.controls ?? []);
  const selectedDimmerControl = createMemo<DimmerControlSet | undefined>(() => {
    const fixture = selectedFixture();
    if (!fixture) {
      return undefined;
    }
    const attribute = findControlAttribute(fixture, ["Dimmer", "Intensity", "MasterIntensity"]);
    if (!attribute) {
      return undefined;
    }
    return {
      attribute,
      value: dimmerValueWithinLimits(fixture, readFixtureAttribute(fixture, faderValues(), [attribute]) ?? 0),
    };
  });
  const selectedPositionControls = createMemo<PositionControlSet | undefined>(() => {
    const fixture = selectedFixture();
    if (!fixture) {
      return undefined;
    }
    const pan = findControlAttribute(fixture, ["Pan"]);
    const tilt = findControlAttribute(fixture, ["Tilt"]);
    if (!pan || !tilt) {
      return undefined;
    }
    const currentValues = faderValues();
    const rawPanValue = readFixtureAttribute(fixture, currentValues, [pan]) ?? 32768;
    const rawTiltValue = readFixtureAttribute(fixture, currentValues, [tilt]) ?? 32768;
    const effective = effectivePanTiltValues(fixture, rawPanValue, rawTiltValue);
    return {
      pan,
      tilt,
      panValue: effective.pan,
      tiltValue: effective.tilt,
    };
  });
  const selectedColorControls = createMemo<ColorControlSet | undefined>(() => {
    const fixture = selectedFixture();
    if (!fixture) {
      return undefined;
    }
    const red = findControlAttribute(fixture, colorCandidates.red);
    const green = findControlAttribute(fixture, colorCandidates.green);
    const blue = findControlAttribute(fixture, colorCandidates.blue);
    if (!red || !green || !blue) {
      return undefined;
    }
    const currentValues = faderValues();
    const redValue = readFixtureAttribute(fixture, currentValues, [red]) ?? 0;
    const greenValue = readFixtureAttribute(fixture, currentValues, [green]) ?? 0;
    const blueValue = readFixtureAttribute(fixture, currentValues, [blue]) ?? 0;
    return {
      red,
      green,
      blue,
      value: `#${valueToHexByte(redValue)}${valueToHexByte(greenValue)}${valueToHexByte(blueValue)}`,
    };
  });
  const selectedColorHsv = createMemo(() => {
    const color = selectedColorControls()?.value ?? "#ffffff";
    return rgbToHsv(
      Number.parseInt(color.slice(1, 3), 16),
      Number.parseInt(color.slice(3, 5), 16),
      Number.parseInt(color.slice(5, 7), 16),
    );
  });
  const selectedColorHex = createMemo(() => normalizeHexColor(selectedColorControls()?.value) ?? "#000000");
  const selectedColorChannelValues = createMemo(() => {
    const color = selectedColorHex();
    return {
      red: Number.parseInt(color.slice(1, 3), 16) * 257,
      green: Number.parseInt(color.slice(3, 5), 16) * 257,
      blue: Number.parseInt(color.slice(5, 7), 16) * 257,
    };
  });
  const controlCategoryCounts = createMemo(() => {
    const counts = new Map<ControlCategory, number>();
    for (const category of controlCategories) {
      counts.set(category.id, 0);
    }
    const controls = activeControls();
    counts.set("fader", controls.length);
    for (const control of controls) {
      const category = controlCategoryForAttribute(control.attribute);
      counts.set(category, (counts.get(category) ?? 0) + 1);
    }
    return counts;
  });
  const categoryHasVisualControl = (category: ControlCategory) =>
    (category === "dimmer" && Boolean(selectedDimmerControl())) ||
    (category === "position" && Boolean(selectedPositionControls())) ||
    (category === "color" && Boolean(selectedColorControls())) ||
    category === "fader";
  const activeControlCategory = createMemo<ControlCategory>(() => {
    const requested = controlCategory();
    const counts = controlCategoryCounts();
    if ((counts.get(requested) ?? 0) > 0 || categoryHasVisualControl(requested)) {
      return requested;
    }
    return (
      controlCategories.find((category) => (counts.get(category.id) ?? 0) > 0 || categoryHasVisualControl(category.id))
        ?.id ?? "fader"
    );
  });
  const controlCategoryRows = createMemo(() => {
    const counts = controlCategoryCounts();
    return controlCategories.map((category) => ({
      ...category,
      count: counts.get(category.id) ?? 0,
      hasVisual: categoryHasVisualControl(category.id),
    }));
  });
  const visibleControls = createMemo(() => {
    const category = activeControlCategory();
    const controls = activeControls();
    if (category === "fader") {
      return controls;
    }
    return controls.filter((control) => controlCategoryForAttribute(control.attribute) === category);
  });
  const showDimmerPanel = createMemo(() => Boolean(selectedDimmerControl()) && activeControlCategory() === "dimmer");
  const showPositionPad = createMemo(() => Boolean(selectedPositionControls()) && activeControlCategory() === "position");
  const showColorPad = createMemo(() => Boolean(selectedColorControls()) && activeControlCategory() === "color");
  const showCategoryQuickPanel = createMemo(() =>
    Boolean(selectedFixture()) &&
    visibleControls().length > 0 &&
    !["dimmer", "color", "position"].includes(activeControlCategory()),
  );
  const categoryQuickLooks = createMemo(() => quickLooksForCategory(activeControlCategory()));
  const visibleFunctionControls = createMemo(() =>
    visibleControls()
      .map((control) => ({
        control,
        functions: sortedChannelFunctions(control),
      }))
      .filter((entry) => entry.functions.length > 0),
  );
  const selectedEffectAttribute = createMemo(() => {
    const controls = activeControls();
    const current = effectAttribute();
    if (controls.some((control) => control.attribute === current)) {
      return current;
    }
    return controls[0]?.attribute ?? "";
  });
  const dmxPreviewOptions = createMemo(() =>
    snapshot().dmx_previews.length > 0
      ? snapshot().dmx_previews
      : [{ universe: snapshot().output.universe, values: snapshot().dmx_preview }],
  );
  const activeDmxPreview = createMemo(() => {
    const previews = dmxPreviewOptions();
    return previews.find((preview) => preview.universe === rawDmxUniverse()) ?? previews[0];
  });
  const activeDmxPreviewUniverse = createMemo(() => activeDmxPreview()?.universe ?? snapshot().output.universe);
  const activeDmxPreviewValues = createMemo(() => activeDmxPreview()?.values ?? snapshot().dmx_preview);
  const dmxCells = createMemo(() =>
    activeDmxPreviewValues().map((value, index) => ({
      channel: index + 1,
      value,
    })),
  );
  const nonZeroDmxCount = createMemo(() => activeDmxPreviewValues().filter((value) => value !== 0).length);
  const audioWaveformPoints = createMemo(() => {
    const analysis = audioAnalysis();
    if (!analysis || analysis.waveform.length === 0) {
      return "";
    }
    const width = 100;
    const height = 36;
    const duration = Math.max(analysis.duration_ms, 1);
    return analysis.waveform
      .map((point) => {
        const x = Math.min(width, (point.time_ms / duration) * width);
        const y = height - Math.min(height, point.peak * height);
        return `${x.toFixed(2)},${y.toFixed(2)}`;
      })
      .join(" ");
  });
  const audioBeatMarkers = createMemo(() => {
    const analysis = audioAnalysis();
    if (!analysis || analysis.duration_ms === 0) {
      return [];
    }
    return analysis.beats.slice(0, 128).map((beat) => ({
      time_ms: beat,
      x: Math.min(100, (beat / analysis.duration_ms) * 100),
    }));
  });
  const audioBeatTimes = createMemo(() => audioAnalysis()?.beats ?? []);
  const beatIntervalMs = createMemo(() => {
    const bpm = audioAnalysis()?.estimated_bpm ?? snapshot().clock.bpm;
    return bpm > 0 ? 60_000 / bpm : 500;
  });
  const nearestTimelineValue = (timeMs: number, candidates: number[]) => {
    if (candidates.length === 0) {
      return Math.max(0, Math.round(timeMs));
    }
    let nearest = candidates[0];
    let nearestDistance = Math.abs(timeMs - nearest);
    for (let index = 1; index < candidates.length; index += 1) {
      const candidate = candidates[index];
      const distance = Math.abs(timeMs - candidate);
      if (distance < nearestDistance) {
        nearest = candidate;
        nearestDistance = distance;
      }
    }
    return Math.max(0, Math.round(nearest));
  };
  const snapTimeMs = (timeMs: number) => {
    const clamped = Math.max(0, Number.isFinite(timeMs) ? timeMs : 0);
    switch (timelineSnapMode()) {
      case "Beat": {
        const beats = audioBeatTimes();
        if (beats.length > 0) {
          return nearestTimelineValue(clamped, beats);
        }
        const beat = Math.max(1, beatIntervalMs());
        return Math.round(Math.round(clamped / beat) * beat);
      }
      case "Bar": {
        const bar = Math.max(1, beatIntervalMs() * 4);
        return Math.round(Math.round(clamped / bar) * bar);
      }
      case "Grid": {
        const grid = Math.max(1, timelineGridMs());
        return Math.round(clamped / grid) * grid;
      }
      default:
        return Math.round(clamped);
    }
  };
  const timelineCueOptions = createMemo(() => snapshot().cues.map((cue) => ({ id: cue.id, label: cue.label })));
  const selectedTimelineCueId = createMemo(() => {
    const current = timelineCueId();
    const cues = timelineCueOptions();
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const timelineEventRows = createMemo(() =>
    snapshot().timeline.events.map((event) => {
      const cue = snapshot().cues.find((candidate) => candidate.id === event.cue_id);
      return {
        ...event,
        cue_label: cue?.label ?? `Cue ${event.cue_id}`,
      };
    }),
  );
  const timelineOverviewDurationMs = createMemo(() =>
    Math.max(1, snapshot().timeline.duration_ms, ...snapshot().timeline.events.map((event) => event.time_ms + 1000)),
  );
  const timelineOverviewPlayheadX = createMemo(() =>
    clampRange((snapshot().timeline.position_ms / timelineOverviewDurationMs()) * 100, 0, 100),
  );
  const timelineOverviewEvents = createMemo<TimelineOverviewEvent[]>(() =>
    timelineEventRows().map((event) => ({
      id: event.id,
      cue_id: event.cue_id,
      cue_label: event.cue_label,
      track: event.track,
      time_ms: event.time_ms,
      x: clampRange((event.time_ms / timelineOverviewDurationMs()) * 100, 0, 100),
      y: event.track === "Lighting" ? 14 : 31,
      active: snapshot().active_cue_id === event.cue_id,
    })),
  );
  const cueTimelinePlacementsForCue = (cueId: number) =>
    timelineEventRows()
      .filter((event) => event.cue_id === cueId)
      .sort((left, right) => left.time_ms - right.time_ms || left.id - right.id);
  const timelinePlacementNudgeMs = createMemo(() => {
    switch (timelineSnapMode()) {
      case "Beat":
        return Math.max(1, beatIntervalMs());
      case "Bar":
        return Math.max(1, beatIntervalMs() * 4);
      case "Grid":
        return Math.max(1, timelineGridMs());
      default:
        return 1000;
    }
  });
  const fixtureAttributeOptions = (fixtureId: number) =>
    snapshot().fixtures.find((fixture) => fixture.id === fixtureId)?.controls.map((control) => control.attribute) ?? [];
  const timelineAutomationRows = createMemo(() =>
    snapshot().timeline.automations.map((automation) => {
      const fixture = snapshot().fixtures.find((candidate) => candidate.id === automation.fixture_id);
      return {
        ...automation,
        fixture_label: fixture?.label ?? `Fixture ${automation.fixture_id}`,
      };
    }),
  );
  const selectedVideoAutomationLayerId = createMemo(() => {
    const current = videoAutomationLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedMidiCueId = createMemo(() => {
    const current = midiMapCueId();
    const cues = snapshot().cues;
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const selectedMidiLayerId = createMemo(() => {
    const current = midiMapLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedMidiVideoOutputId = createMemo(() => {
    const current = midiMapVideoOutputId();
    const outputs = snapshot().video.outputs;
    if (current !== null && outputs.some((output) => output.id === current)) {
      return current;
    }
    return outputs[0]?.id ?? null;
  });
  const selectedOscCueId = createMemo(() => {
    const current = oscMapCueId();
    const cues = snapshot().cues;
    if (current !== null && cues.some((cue) => cue.id === current)) {
      return current;
    }
    return cues[0]?.id ?? null;
  });
  const selectedOscLayerId = createMemo(() => {
    const current = oscMapLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const selectedOscVideoOutputId = createMemo(() => {
    const current = oscMapVideoOutputId();
    const outputs = snapshot().video.outputs;
    if (current !== null && outputs.some((output) => output.id === current)) {
      return current;
    }
    return outputs[0]?.id ?? null;
  });
  const selectedEffectVideoLayerId = createMemo(() => {
    const current = effectVideoLayerId();
    const layers = snapshot().video.layers;
    if (current !== null && layers.some((layer) => layer.id === current)) {
      return current;
    }
    return layers[0]?.id ?? null;
  });
  const videoPreviewDiagnosticsText = createMemo(() => {
    const diagnostics = videoPreviewDiagnostics();
    if (!diagnostics) {
      return "No preview diagnostics";
    }
    const queuedFrames = diagnostics.layer_queues.reduce((total, row) => total + row.queue_len, 0);
    const activeQueues = diagnostics.layer_queues.filter((row) => row.queue_len > 0).length;
    return `queues ${activeQueues}/${diagnostics.queue_count}, frames ${queuedFrames}, still ${diagnostics.still_image_cache_len}, decode ${diagnostics.decoder_cache_len}, prefetch ${diagnostics.prefetch_count}x${diagnostics.prefetch_interval_ms}ms`;
  });
  const projectFileLabel = createMemo(() => {
    const path = currentProjectPath();
    const label = (() => {
      if (!path) {
        return "Untitled.ry";
      }
      const normalizedPath = path.replaceAll("\\", "/");
      return normalizedPath.split("/").pop() || path;
    })();
    return projectDirty() ? `${label} *` : label;
  });
  const customProfilePreview = createMemo(() => customProfilePreviewFromText(customAttributes()));
  const hasCueSources = createMemo(
    () => snapshot().fixtures.length > 0 || snapshot().video.layers.length > 0 || snapshot().video.outputs.length > 0,
  );
  const cueCaptureScopeError = createMemo(() => {
    switch (cueCaptureScope()) {
      case "all":
        return hasCueSources() ? "" : "Patch fixtures or add video layers/outputs first.";
      case "lighting":
        return snapshot().fixtures.length > 0 ? "" : "Patch fixtures before storing a lighting cue.";
      case "selectedFixture":
        return selectedFixture() ? "" : "Select a fixture before storing a selected-fixture cue.";
      case "selectedGroup":
        return selectedFixtureGroupFilter() ? "" : "Select a group before storing a group cue.";
      case "video":
        return snapshot().video.layers.length > 0 || snapshot().video.outputs.length > 0
          ? ""
          : "Add video layers or outputs before storing a video cue.";
    }
  });
  const cueCaptureScopeRequest = (): CueCaptureScopeRequest | null => {
    switch (cueCaptureScope()) {
      case "all":
        return { kind: "all" };
      case "lighting":
        return { kind: "lightingOnly" };
      case "selectedFixture": {
        const fixture = selectedFixture();
        return fixture ? { kind: "selectedFixture", fixtureId: fixture.id } : null;
      }
      case "selectedGroup": {
        const groupId = selectedFixtureGroupFilter();
        return groupId ? { kind: "selectedGroup", groupId } : null;
      }
      case "video":
        return { kind: "videoOnly" };
    }
  };
  const timelineVideoAutomationRows = createMemo(() =>
    snapshot().timeline.video_automations.map((automation) => {
      const layer = snapshot().video.layers.find((candidate) => candidate.id === automation.layer_id);
      return {
        ...automation,
        layer_label: layer?.label ?? `Layer ${automation.layer_id}`,
      };
    }),
  );
  const activeCue = createMemo(() => {
    const activeCueId = snapshot().active_cue_id;
    return snapshot().cues.find((cue) => cue.id === activeCueId) ?? null;
  });
  const nextCue = createMemo(() => {
    const cues = snapshot().cues;
    if (cues.length === 0) {
      return null;
    }
    const activeCueId = snapshot().active_cue_id;
    const activeIndex = activeCueId === null || activeCueId === undefined
      ? -1
      : cues.findIndex((cue) => cue.id === activeCueId);
    return cues[(activeIndex + 1 + cues.length) % cues.length];
  });
  const cuePadBankCount = createMemo(() => Math.max(1, Math.ceil(snapshot().cues.length / cuePadSize)));
  const cuePadStartIndex = createMemo(() => cuePadBank() * cuePadSize);
  const activeCueIndex = createMemo(() => {
    const activeCueId = snapshot().active_cue_id;
    if (activeCueId === null || activeCueId === undefined) {
      return -1;
    }
    return snapshot().cues.findIndex((cue) => cue.id === activeCueId);
  });
  const liveCuePads = createMemo(() =>
    Array.from({ length: cuePadSize }, (_, index) => ({
      slot: index === 9 ? "0" : String(index + 1),
      index: cuePadStartIndex() + index,
      cue: snapshot().cues[cuePadStartIndex() + index] ?? null,
    })),
  );
  const enabledDmxOutputCount = createMemo(() => snapshot().dmx_outputs.filter((route) => route.enabled).length);
  const enabledVideoOutputCount = createMemo(() => snapshot().video.outputs.filter((output) => output.enabled).length);
  const activeEffectCount = createMemo(() => snapshot().effects.filter((effect) => effect.enabled).length);
  const touchLayoutClass = createMemo(() =>
    workspaceTab() === "setup"
      ? `layoutSetup setupMode-${setupSubTab()}`
      : workspaceTab() === "touch"
        ? "layoutTouch"
        : "layoutControl",
  );
  const remoteUrl = createMemo(() => {
    const host = remoteBindIp().trim();
    const displayHost = host === "" || host === "0.0.0.0" ? "localhost" : host;
    return `http://${displayHost}:${remotePort()}/remote`;
  });
  const autoStageWorldBounds = createMemo<StageWorldBounds>(() => {
    const fixtures = snapshot().fixtures;
    const outputPoints = snapshot().video.outputs.flatMap((output) => {
      const size = surfaceWorldHalfSize(output);
      const x = output.mapping.stage_x;
      const z = output.mapping.stage_z;
      return [
        { x: x - size.width, z: z - size.height },
        { x: x + size.width, z: z + size.height },
      ];
    });
    const points = [
      ...fixtures.map((fixture) => ({ x: fixture.position.x, z: fixture.position.z })),
      ...outputPoints,
    ];
    const halfX = Math.max(10, ...points.map((point) => Math.abs(point.x))) * 1.1;
    const halfZ = Math.max(10, ...points.map((point) => Math.abs(point.z))) * 1.1;
    return {
      minX: -halfX,
      maxX: halfX,
      minZ: -halfZ,
      maxZ: halfZ,
    };
  });
  const stageWorldBounds = createMemo<StageWorldBounds>(() => {
    const stageMap = snapshot().stage_map;
    if (
      stageMap.locked &&
      Number.isFinite(stageMap.min_x) &&
      Number.isFinite(stageMap.max_x) &&
      Number.isFinite(stageMap.min_z) &&
      Number.isFinite(stageMap.max_z) &&
      stageMap.max_x > stageMap.min_x &&
      stageMap.max_z > stageMap.min_z
    ) {
      return {
        minX: stageMap.min_x,
        maxX: stageMap.max_x,
        minZ: stageMap.min_z,
        maxZ: stageMap.max_z,
      };
    }
    return autoStageWorldBounds();
  });
  const stageOrigin2d = createMemo(() => stageWorldToSvgPoint(0, 0, stageWorldBounds()));
  const cueCapturePreview = createMemo(() => {
    const current = snapshot();
    const scope = cueCaptureScope();
    const currentValues = faderValues();
    const bounds = stageWorldBounds();
    const selectedGroupId = selectedFixtureGroupFilter();
    const fixtures = (() => {
      if (scope === "video") {
        return [];
      }
      if (scope === "selectedFixture") {
        const fixture = selectedFixture();
        return fixture ? [fixture] : [];
      }
      if (scope === "selectedGroup") {
        return selectedGroupId
          ? current.fixtures.filter((fixture) => fixture.group_ids.includes(selectedGroupId))
          : [];
      }
      return current.fixtures;
    })();
    const includeVideo = scope === "all" || scope === "video";
    const layers = includeVideo ? current.video.layers : [];
    const outputs = includeVideo ? current.video.outputs : [];
    const nodeGraphs = includeVideo ? current.node_graphs : [];
    const scopeLabel =
      scope === "all"
        ? "Lighting + Video"
        : scope === "lighting"
          ? "Lighting Only"
          : scope === "selectedFixture"
            ? "Selected Fixture"
            : scope === "selectedGroup"
              ? "Selected Group"
              : "Video Only";
    const scopeDetail =
      scope === "selectedFixture"
        ? selectedFixture()?.label ?? "No fixture selected"
        : scope === "selectedGroup"
          ? selectedGroupId ?? "No group selected"
          : scopeLabel;
    const previewFixtures = fixtures.map((fixture) => {
      const point = stageWorldToSvgPoint(fixture.position.x, fixture.position.z, bounds);
      const dimmer = readFixtureAttribute(fixture, currentValues, ["Dimmer", "Intensity"]) ?? 0;
      const red = readFixtureAttribute(fixture, currentValues, ["ColorRed", "Red"]);
      const green = readFixtureAttribute(fixture, currentValues, ["ColorGreen", "Green"]);
      const blue = readFixtureAttribute(fixture, currentValues, ["ColorBlue", "Blue"]);
      const color =
        red !== undefined || green !== undefined || blue !== undefined
          ? `#${valueToHexByte(clampDmxValue(red ?? 0))}${valueToHexByte(clampDmxValue(green ?? 0))}${valueToHexByte(clampDmxValue(blue ?? 0))}`
          : "#58a7f6";
      return {
        id: fixture.id,
        label: fixture.label,
        dmxLabel: `U${fixture.universe} A${fixture.address}`,
        groupLabel: fixture.group_ids.length > 0 ? fixture.group_ids.join(", ") : "No group",
        x: point.x,
        z: point.z,
        color,
        intensity: dmxValueToPercent(dimmer),
        attributes: fixture.attribute_values.length,
      };
    });
    const videoRows = [
      ...layers.map((layer) => ({
        label: layer.label,
        meta: `Layer / ${layer.blend_mode} / ${Math.round(layer.state.opacity * 100)}%`,
      })),
      ...outputs.map((output) => ({
        label: output.label,
        meta: `Output / ${output.kind} / ${Math.round(output.opacity * 100)}%`,
      })),
      ...nodeGraphs.map((graph) => ({
        label: graph.label,
        meta: graph.enabled ? "Node graph enabled" : "Node graph disabled",
      })),
    ];
    return {
      scopeLabel,
      scopeDetail,
      fixtures: previewFixtures,
      videoRows,
      layerCount: layers.length,
      outputCount: outputs.length,
      nodeGraphCount: nodeGraphs.length,
      fixtureAttributeCount: fixtures.reduce((total, fixture) => total + fixture.attribute_values.length, 0),
    };
  });
  const normalizedMappingViewportZoom = createMemo(() => clampRange(mappingViewportZoom(), 1, 4));
  const mappingViewportSize = createMemo(() => stageViewBoxSize / normalizedMappingViewportZoom());
  const mappingViewportBox = createMemo(() => {
    const size = mappingViewportSize();
    return {
      x: clampRange(mappingViewportCenterX() - size / 2, 0, stageViewBoxSize - size),
      z: clampRange(mappingViewportCenterZ() - size / 2, 0, stageViewBoxSize - size),
      size,
    };
  });
  const mappingStageViewBox = createMemo(() => {
    const box = mappingViewportBox();
    return `${box.x} ${box.z} ${box.size} ${box.size}`;
  });
  const mappingViewportZoomLabel = createMemo(() => `${Math.round(normalizedMappingViewportZoom() * 100)}%`);
  const setMappingViewport = (zoom: number, centerX = mappingViewportCenterX(), centerZ = mappingViewportCenterZ()) => {
    const nextZoom = clampRange(zoom, 1, 4);
    const nextSize = stageViewBoxSize / nextZoom;
    setMappingViewportZoom(nextZoom);
    setMappingViewportCenterX(clampRange(centerX, nextSize / 2, stageViewBoxSize - nextSize / 2));
    setMappingViewportCenterZ(clampRange(centerZ, nextSize / 2, stageViewBoxSize - nextSize / 2));
  };
  const zoomMappingViewport = (direction: -1 | 1) => {
    const nextZoom = direction > 0 ? normalizedMappingViewportZoom() * 1.25 : normalizedMappingViewportZoom() / 1.25;
    setMappingViewport(Number(nextZoom.toFixed(3)));
  };
  const zoomMappingViewportAtPoint = (direction: -1 | 1, point: { x: number; z: number }) => {
    const currentBox = mappingViewportBox();
    const nextZoom = clampRange(
      Number((direction > 0 ? normalizedMappingViewportZoom() * 1.18 : normalizedMappingViewportZoom() / 1.18).toFixed(3)),
      1,
      4,
    );
    const nextSize = stageViewBoxSize / nextZoom;
    const anchorX = clampRange((point.x - currentBox.x) / currentBox.size, 0, 1);
    const anchorZ = clampRange((point.z - currentBox.z) / currentBox.size, 0, 1);
    setMappingViewport(nextZoom, point.x + (0.5 - anchorX) * nextSize, point.z + (0.5 - anchorZ) * nextSize);
  };
  const resetMappingViewport = () => {
    setMappingViewport(1, stageViewBoxSize / 2, stageViewBoxSize / 2);
    setMessage("Reset 2D mapping viewport.");
  };
  const normalizedMappingSnapSize = createMemo(() => {
    const size = Math.abs(mappingSnapSize());
    return Number.isFinite(size) ? clampRange(size, 0.05, 20) : 0.5;
  });
  const snapStageCoordinate = (value: number) => {
    const nextValue = mappingSnapEnabled()
      ? Math.round(value / normalizedMappingSnapSize()) * normalizedMappingSnapSize()
      : value;
    return Number(nextValue.toFixed(2));
  };
  const snapStagePoint = (point: { x: number; z: number }) => ({
    x: snapStageCoordinate(point.x),
    z: snapStageCoordinate(point.z),
  });
  const snapStagePosition = (position: PatchFixtureRequest["position"]) => ({
    ...position,
    x: snapStageCoordinate(position.x),
    z: snapStageCoordinate(position.z),
  });
  const mappingSnapLines = createMemo<MappingSnapLine[]>(() => {
    if (!mappingSnapEnabled()) {
      return [];
    }
    const step = normalizedMappingSnapSize();
    const bounds = stageWorldBounds();
    const firstX = Math.ceil(bounds.minX / step) * step;
    const lastX = Math.floor(bounds.maxX / step) * step;
    const firstZ = Math.ceil(bounds.minZ / step) * step;
    const lastZ = Math.floor(bounds.maxZ / step) * step;
    const xCount = Math.max(0, Math.floor((lastX - firstX) / step) + 1);
    const zCount = Math.max(0, Math.floor((lastZ - firstZ) / step) + 1);
    if (xCount + zCount > 180) {
      return [];
    }
    const lines: MappingSnapLine[] = [];
    for (let index = 0; index < xCount; index += 1) {
      const x = firstX + index * step;
      lines.push({ axis: "x", svg: stageWorldToSvgPoint(x, 0, bounds).x });
    }
    for (let index = 0; index < zCount; index += 1) {
      const z = firstZ + index * step;
      lines.push({ axis: "z", svg: stageWorldToSvgPoint(0, z, bounds).z });
    }
    return lines;
  });
  const mappingMarqueeBox = createMemo(() => {
    const marquee = mappingMarquee();
    if (!marquee) {
      return null;
    }
    return {
      x: Math.min(marquee.start.x, marquee.current.x),
      z: Math.min(marquee.start.z, marquee.current.z),
      width: Math.abs(marquee.current.x - marquee.start.x),
      height: Math.abs(marquee.current.z - marquee.start.z),
    };
  });
  const dragWorldDelta = (drag: MappingDragState) => ({
    x: drag.currentWorld.x - drag.startWorld.x,
    z: drag.currentWorld.z - drag.startWorld.z,
  });
  const mappingOutputHandleAngleDeg = (center: { x: number; z: number }, point: { x: number; z: number }) =>
    (Math.atan2(point.z - center.z, point.x - center.x) * 180) / Math.PI;
  const mappingOutputHandleDistance = (center: { x: number; z: number }, point: { x: number; z: number }) => {
    const dx = point.x - center.x;
    const dz = point.z - center.z;
    return Math.sqrt(dx * dx + dz * dz);
  };
  const mappingVideoOutputPreviewMapping = (drag: MappingDragState, output: VideoOutputSummary): VideoOutputMapping => {
    if (drag.kind === "videoOutput") {
      const delta = dragWorldDelta(drag);
      const point = snapStagePoint({
        x: drag.startMapping.stage_x + delta.x,
        z: drag.startMapping.stage_z + delta.z,
      });
      return {
        ...drag.startMapping,
        stage_x: point.x,
        stage_z: point.z,
      };
    }
    if (drag.kind === "videoOutputRotate") {
      const currentAngle = mappingOutputHandleAngleDeg(drag.centerWorld, drag.currentWorld);
      return {
        ...drag.startMapping,
        rotation_deg: Number((drag.startMapping.rotation_deg + currentAngle - drag.startAngleDeg).toFixed(1)),
      };
    }
    if (drag.kind === "videoOutputScale") {
      const currentDistance = mappingOutputHandleDistance(drag.centerWorld, drag.currentWorld);
      const ratio = currentDistance / Math.max(0.001, drag.startDistance);
      return {
        ...drag.startMapping,
        scale_x: Number(clampRange(drag.startMapping.scale_x * ratio, 0.25, 3).toFixed(3)),
        scale_y: Number(clampRange(drag.startMapping.scale_y * ratio, 0.25, 3).toFixed(3)),
      };
    }
    return output.mapping;
  };
  const mappingFixturePosition = (fixture: PatchedFixtureSummary) => {
    const drag = mappingDrag();
    if (drag?.kind !== "fixture" || !drag.fixtureIds.includes(fixture.id)) {
      return fixture.position;
    }
    const startPosition = drag.startPositions[fixture.id];
    if (!startPosition) {
      return fixture.position;
    }
    const delta = dragWorldDelta(drag);
    return snapStagePosition({
      ...startPosition,
      x: startPosition.x + delta.x,
      z: startPosition.z + delta.z,
    });
  };
  const mappingVideoOutputMapping = (output: VideoOutputSummary): VideoOutputMapping => {
    const drag = mappingDrag();
    if (!drag || drag.kind === "fixture" || drag.outputId !== output.id) {
      return output.mapping;
    }
    return mappingVideoOutputPreviewMapping(drag, output);
  };
  const isDraggingMappingFixture = (fixtureId: number) => {
    const drag = mappingDrag();
    return drag?.kind === "fixture" && drag.fixtureIds.includes(fixtureId);
  };
  const isDraggingMappingVideoOutput = (outputId: number) => {
    const drag = mappingDrag();
    return Boolean(drag && drag.kind !== "fixture" && drag.outputId === outputId);
  };
  const visualizerFixtures = createMemo<VisualizerFixture[]>(() => {
    const fixtures = mappingFilteredFixtures();
    if (fixtures.length === 0) {
      return [];
    }

    const bounds = stageWorldBounds();
    const currentValues = faderValues();

    return fixtures.map((fixture) => {
      const position = mappingFixturePosition(fixture);
      const point = stageWorldToSvgPoint(position.x, position.z, bounds);
      const dimmer = readFixtureAttribute(fixture, currentValues, ["Dimmer", "Intensity"]) ?? 0;
      const pan = readFixtureAttribute(fixture, currentValues, ["Pan"]);
      const red = readFixtureAttribute(fixture, currentValues, ["ColorRed", "Red"]);
      const green = readFixtureAttribute(fixture, currentValues, ["ColorGreen", "Green"]);
      const blue = readFixtureAttribute(fixture, currentValues, ["ColorBlue", "Blue"]);
      const color =
        red !== undefined || green !== undefined || blue !== undefined
          ? `rgb(${red ? red >> 8 : 0}, ${green ? green >> 8 : 0}, ${blue ? blue >> 8 : 0})`
          : "rgb(88, 167, 246)";
      const intensity = clamp01(dimmer / 65_535);
      const panDegrees = pan === undefined ? 0 : ((pan - 32_768) / 65_535) * 540;
      const visualKind = fixtureVisualKind(fixture);
      const size =
        visualKind === "bar"
          ? { width: 5.8, height: 1.4 }
          : visualKind === "panel"
            ? { width: 4.8, height: 3.2 }
            : visualKind === "laser"
              ? { width: 3.4, height: 3.4 }
              : { width: 3.2, height: 3.2 };
      return {
        id: fixture.id,
        label: fixture.label,
        dmxLabel: `U${fixture.universe} A${fixture.address}`,
        groupLabel: fixture.group_ids.length > 0 ? fixture.group_ids.join(", ") : "No group",
        typeKey: fixtureTypeKey(fixture),
        visualKind,
        x: point.x,
        z: point.z,
        width: size.width,
        height: size.height,
        yaw: fixture.rotation.yaw,
        beamPoints: beamPoints(point.x, point.z, fixture.rotation.yaw + panDegrees, intensity),
        intensity,
        color,
        inGroupFilter: selectedFixtureGroupFilter() ? fixture.group_ids.includes(selectedFixtureGroupFilter()!) : true,
        highlighted: fixture.highlighted,
        soloed: fixture.soloed,
        parked: fixture.parked,
      };
    });
  });

  const visualizerVideoSurfaces2d = createMemo<VisualizerVideoSurface2d[]>(() => {
    const bounds = stageWorldBounds();
    return snapshot().video.outputs.map((output) => {
      const mapping = mappingVideoOutputMapping(output);
      const center = stageWorldToSvgPoint(mapping.stage_x, mapping.stage_z, bounds);
      const size = surfaceWorldHalfSize(output);
      const xEdge = stageWorldToSvgPoint(mapping.stage_x + size.width, mapping.stage_z, bounds);
      const zEdge = stageWorldToSvgPoint(mapping.stage_x, mapping.stage_z + size.height, bounds);
      return {
        id: output.id,
        label: output.label,
        x: center.x,
        z: center.z,
        width: Math.max(3, Math.abs(xEdge.x - center.x) * 2),
        height: Math.max(2, Math.abs(zEdge.z - center.z) * 2),
        rotationDeg: mapping.rotation_deg,
        opacity: clampRange(output.opacity, 0, 1),
        active: output.enabled && !output.blackout,
      };
    });
  });
  const waveOriginSvgPoint = createMemo(() => stageWorldToSvgPoint(waveOriginX(), waveOriginZ(), stageWorldBounds()));
  const waveDirectionLength = createMemo(() => {
    const x = waveDirectionX();
    const y = waveDirectionY();
    const z = waveDirectionZ();
    return Math.sqrt(x * x + y * y + z * z);
  });
  const waveDirectionIsRadial = createMemo(() => waveDirectionLength() <= Number.EPSILON);
  const waveDirectionSvgPoint = createMemo(() => {
    const bounds = stageWorldBounds();
    const length = waveDirectionLength();
    if (length <= Number.EPSILON) {
      return waveOriginSvgPoint();
    }
    const scale = Math.min(bounds.maxX - bounds.minX, bounds.maxZ - bounds.minZ) * 0.18;
    return stageWorldToSvgPoint(
      waveOriginX() + (waveDirectionX() / length) * scale,
      waveOriginZ() + (waveDirectionZ() / length) * scale,
      bounds,
    );
  });
  const waveRadialRadius = createMemo(() => {
    const bounds = stageWorldBounds();
    const edge = stageWorldToSvgPoint(waveOriginX() + Math.min(bounds.maxX - bounds.minX, bounds.maxZ - bounds.minZ) * 0.16, waveOriginZ(), bounds);
    return Math.max(4, Math.abs(edge.x - waveOriginSvgPoint().x));
  });
  const waveTargetFixtureIds = createMemo(() => {
    const mode = effectTargetMode();
    if (mode === "fixture") {
      const fixture = selectedFixture();
      return new Set(fixture ? [fixture.id] : []);
    }
    if (mode === "group") {
      const groups = parseGroupIds(effectTargetGroups());
      return new Set(
        snapshot()
          .fixtures.filter((fixture) => groups.some((groupId) => fixture.group_ids.includes(groupId)))
          .map((fixture) => fixture.id),
      );
    }
    return new Set<number>();
  });

  const selectedMappingVideoOutput = createMemo(() => {
    const outputId = selectedVideoOutputId();
    if (outputId === null) {
      return snapshot().video.outputs[0] ?? null;
    }
    return snapshot().video.outputs.find((output) => output.id === outputId) ?? snapshot().video.outputs[0] ?? null;
  });
  const fixtureSvgBounds = (fixture: VisualizerFixture): MappingSvgBounds => {
    const halfWidth = Math.max(2.4, fixture.width / 2 + 1.6);
    const halfHeight = Math.max(2.4, fixture.height / 2 + 1.6);
    return {
      minX: fixture.x - halfWidth,
      maxX: fixture.x + halfWidth,
      minZ: fixture.z - halfHeight,
      maxZ: fixture.z + halfHeight,
    };
  };
  const videoSurfaceSvgBounds = (surface: VisualizerVideoSurface2d): MappingSvgBounds => {
    const angle = (surface.rotationDeg * Math.PI) / 180;
    const cos = Math.cos(angle);
    const sin = Math.sin(angle);
    const corners = [
      { x: -surface.width / 2, z: -surface.height / 2 },
      { x: surface.width / 2, z: -surface.height / 2 },
      { x: surface.width / 2, z: surface.height / 2 },
      { x: -surface.width / 2, z: surface.height / 2 },
    ].map((corner) => ({
      x: surface.x + corner.x * cos - corner.z * sin,
      z: surface.z + corner.x * sin + corner.z * cos,
    }));
    return {
      minX: Math.min(...corners.map((corner) => corner.x)) - 2,
      maxX: Math.max(...corners.map((corner) => corner.x)) + 2,
      minZ: Math.min(...corners.map((corner) => corner.z)) - 2,
      maxZ: Math.max(...corners.map((corner) => corner.z)) + 2,
    };
  };
  const mergeSvgBounds = (bounds: MappingSvgBounds[]) => {
    if (bounds.length === 0) {
      return null;
    }
    return {
      minX: Math.min(...bounds.map((bound) => bound.minX)),
      maxX: Math.max(...bounds.map((bound) => bound.maxX)),
      minZ: Math.min(...bounds.map((bound) => bound.minZ)),
      maxZ: Math.max(...bounds.map((bound) => bound.maxZ)),
    };
  };
  const fitMappingViewportToSvgBounds = (bounds: MappingSvgBounds | null, label: string) => {
    if (!bounds) {
      setMessage(`Nothing to fit for ${label}.`);
      return;
    }
    const width = Math.max(1, bounds.maxX - bounds.minX);
    const height = Math.max(1, bounds.maxZ - bounds.minZ);
    const size = clampRange(Math.max(width, height) * 1.28, stageViewBoxSize / 4, stageViewBoxSize);
    const zoom = stageViewBoxSize / size;
    setMappingViewport(zoom, (bounds.minX + bounds.maxX) / 2, (bounds.minZ + bounds.maxZ) / 2);
    setMessage(`Fit 2D mapping viewport to ${label}.`);
  };
  const fitMappingViewportToVisible = () => {
    const fixtureBounds = visualizerFixtures().map(fixtureSvgBounds);
    const surfaceBounds = visualizerVideoSurfaces2d().map(videoSurfaceSvgBounds);
    fitMappingViewportToSvgBounds(mergeSvgBounds([...fixtureBounds, ...surfaceBounds]), "visible stage items");
  };
  const fitMappingViewportToSelection = () => {
    const selectedIds = selectedMappingFixtureIdSet();
    const fixtureBounds = visualizerFixtures()
      .filter((fixture) => selectedIds.has(fixture.id))
      .map(fixtureSvgBounds);
    const surfaceBounds = selectedVideoOutputId() === null
      ? []
      : visualizerVideoSurfaces2d()
          .filter((surface) => surface.id === selectedVideoOutputId())
          .map(videoSurfaceSvgBounds);
    fitMappingViewportToSvgBounds(mergeSvgBounds([...fixtureBounds, ...surfaceBounds]), "selection");
  };
  const canFitMappingViewportToVisible = createMemo(
    () => visualizerFixtures().length > 0 || visualizerVideoSurfaces2d().length > 0,
  );
  const canFitMappingViewportToSelection = createMemo(
    () => selectedMappingFixtures().length > 0 || selectedVideoOutputId() !== null,
  );
  const effectVideoTargetPosition = () => ({
    x: effectVideoPositionX(),
    y: effectVideoPositionY(),
    z: effectVideoPositionZ(),
  });
  const effectVideoTargetSvgPoint = createMemo(() =>
    stageWorldToSvgPoint(effectVideoPositionX(), effectVideoPositionZ(), stageWorldBounds()),
  );
  const setEffectVideoPosition = (position: { x: number; y: number; z: number }, nextMessage?: string) => {
    setEffectVideoPositionX(Number(position.x.toFixed(2)));
    setEffectVideoPositionY(Number(position.y.toFixed(2)));
    setEffectVideoPositionZ(Number(position.z.toFixed(2)));
    if (nextMessage) {
      setMessage(nextMessage);
    }
  };
  const setEffectVideoPositionFromStagePoint = (point: { x: number; z: number }) => {
    setEffectVideoPosition({ x: point.x, y: effectVideoPositionY(), z: point.z });
  };
  const setEffectVideoPositionFromVideoOutput = (output: VideoOutputSummary, nextMessage?: string) => {
    const mapping = mappingVideoOutputMapping(output);
    setSelectedVideoOutputId(output.id);
    setEffectVideoPosition(
      {
        x: mapping.stage_x,
        y: 0,
        z: mapping.stage_z,
      },
      nextMessage ?? `Video effect target uses ${output.label} stage position.`,
    );
  };
  const setEffectVideoPositionFromSelectedOutput = () => {
    const output = selectedMappingVideoOutput();
    if (!output) {
      setMessage("Add or select a video output before using its stage position.");
      return;
    }
    setEffectVideoPositionFromVideoOutput(output);
  };
  const setEffectVideoPositionFromWaveOrigin = () => {
    setEffectVideoPosition(
      {
        x: waveOriginX(),
        y: waveOriginY(),
        z: waveOriginZ(),
      },
      "Video effect target uses the current wave origin.",
    );
  };
  const setEffectVideoPositionFromStageCenter = () => {
    const bounds = stageWorldBounds();
    setEffectVideoPosition(
      {
        x: (bounds.minX + bounds.maxX) / 2,
        y: 0,
        z: (bounds.minZ + bounds.maxZ) / 2,
      },
      "Video effect target uses stage center.",
    );
  };

  const parseGroupIds = (value: string) =>
    value
      .split(",")
      .map((group) => group.trim())
      .filter((group) => group.length > 0);

  const uniqueGroupIds = (groupIds: string[]) => [...new Set(groupIds)];

  const registerSetupPanel = (tabs: SetupSubTab[]) => (element: HTMLElement) => {
    for (const tab of tabs) {
      setupPanelRefs[tab] = element;
    }
  };

  const selectSetupMode = (tab: SetupSubTab) => {
    setSetupSubTab(tab);
    requestAnimationFrame(() => {
      const panel = setupPanelRefs[tab];
      if (!panel) {
        return;
      }
      panel.scrollIntoView({
        behavior: "smooth",
        block: "nearest",
        inline: "center",
      });
      panel.focus({ preventScroll: true });
    });
  };

  const setupPanelClass = (baseClass: string, tabs: SetupSubTab[]) =>
    `${baseClass} ${tabs.includes(setupSubTab()) ? "setupPanelFocus" : ""}`;

  const activateFixture = (fixture: PatchedFixtureSummary) => {
    setSelectedFixtureId(fixture.id);
    setSelectedFixtureLabelDraft(fixture.label);
    setSelectedFixtureUniverseDraft(fixture.universe);
    setSelectedFixtureAddressDraft(fixture.address);
    setSelectedFixtureGroupText(fixture.group_ids.join(", "));
    setSelectedFixtureLimitsDraft(fixture.limits ?? defaultFixtureLimits);
  };

  const selectFixture = (fixture: PatchedFixtureSummary) => {
    activateFixture(fixture);
    setSelectedMappingFixtureIds([fixture.id]);
  };

  const isAdditiveMappingSelectionEvent = (event: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">) =>
    event.ctrlKey || event.metaKey || event.shiftKey;

  const selectMappingFixture = (
    fixture: PatchedFixtureSummary,
    event?: Pick<MouseEvent, "ctrlKey" | "metaKey" | "shiftKey">,
  ) => {
    if (!event || !isAdditiveMappingSelectionEvent(event)) {
      selectFixture(fixture);
      return;
    }

    const currentIds = selectedMappingFixtureIds();
    if (currentIds.includes(fixture.id)) {
      if (currentIds.length === 1) {
        activateFixture(fixture);
        setSelectedMappingFixtureIds([fixture.id]);
        return;
      }
      const nextIds = currentIds.filter((id) => id !== fixture.id);
      const nextActiveFixture =
        selectedFixtureId() === fixture.id
          ? snapshot().fixtures.find((candidate) => candidate.id === nextIds[0])
          : fixture;
      if (nextActiveFixture) {
        activateFixture(nextActiveFixture);
      }
      setSelectedMappingFixtureIds(nextIds);
      return;
    }

    activateFixture(fixture);
    setSelectedMappingFixtureIds([...currentIds, fixture.id]);
  };

  const pickVisibleMappingFixtures = () => {
    const fixtures = mappingFilteredFixtures();
    if (fixtures.length === 0) {
      setSelectedMappingFixtureIds([]);
      return;
    }
    activateFixture(fixtures[0]);
    setSelectedMappingFixtureIds(fixtures.map((fixture) => fixture.id));
  };

  const clearMappingFixtureSelection = () => {
    setSelectedMappingFixtureIds([]);
  };

  const handleDmxAddressCellClick = (cell: DmxAddressCell) => {
    setUniverse(activePatchGridUniverse());
    setAddress(cell.segment?.start ?? cell.channel);
    if (cell.segment) {
      selectFixture(cell.segment.fixture);
    }
  };

  const selectNextFreePatchAddress = () => {
    const nextAddress = nextFreePatchAddress();
    if (nextAddress === null) {
      setMessage(`No free ${selectedFootprint()}ch range for ${patchCountValue()} fixture(s) in universe ${universe()}`);
      return;
    }
    setAddress(nextAddress);
    setPatchGridUniverse(universe());
    setDmxPatchViewMode("grid");
  };

  const selectFixtureGroupFilter = (groupId: string | null) => {
    setSelectedFixtureGroupFilter(groupId);
    if (!groupId) {
      return;
    }
    setEffectTargetMode("group");
    setEffectTargetGroups(groupId);
    const firstFixture = snapshot().fixtures.find((fixture) => fixture.group_ids.includes(groupId));
    if (firstFixture) {
      selectFixture(firstFixture);
    }
  };

  const toggleEffectTargetGroup = (groupId: string) => {
    const currentGroups = parseGroupIds(effectTargetGroups());
    const nextGroups = currentGroups.includes(groupId)
      ? currentGroups.filter((candidate) => candidate !== groupId)
      : [...currentGroups, groupId];
    setEffectTargetMode("group");
    setEffectTargetGroups(nextGroups.join(", "));
  };

  const snapshotFaderValues = (next: EngineSnapshot) => {
    const values: Record<string, number> = {};
    for (const fixture of next.fixtures) {
      for (const attributeValue of fixture.attribute_values) {
        values[`${fixture.id}:${attributeValue.attribute}`] = attributeValue.value;
      }
    }
    return values;
  };

  const syncVideoOutputConfigDrafts = (outputs: VideoOutputSummary[]) => {
    setVideoOutputConfigDrafts((current) => {
      const nextDrafts: Record<number, VideoOutputConfigDraft> = {};
      for (const output of outputs) {
        nextDrafts[output.id] = current[output.id] ?? videoOutputConfigDraftFromSummary(output);
      }
      return nextDrafts;
    });
  };

  const videoOutputConfigDraft = (output: VideoOutputSummary) =>
    videoOutputConfigDrafts()[output.id] ?? videoOutputConfigDraftFromSummary(output);

  const updateVideoOutputConfigDraft = (
    output: VideoOutputSummary,
    patch: Partial<VideoOutputConfigDraft>,
  ) => {
    setVideoOutputConfigDrafts((current) => ({
      ...current,
      [output.id]: {
        ...(current[output.id] ?? videoOutputConfigDraftFromSummary(output)),
        ...patch,
      },
    }));
  };

  const syncCueMetadataDrafts = (cues: CueSummary[]) => {
    setCueMetadataDrafts((current) => {
      const nextDrafts: Record<number, CueMetadataDraft> = {};
      for (const cue of cues) {
        nextDrafts[cue.id] = current[cue.id] ?? cueMetadataDraftFromSummary(cue);
      }
      return nextDrafts;
    });
  };

  const cueMetadataDraft = (cue: CueSummary) =>
    cueMetadataDrafts()[cue.id] ?? cueMetadataDraftFromSummary(cue);

  const updateCueMetadataDraft = (cue: CueSummary, patch: Partial<CueMetadataDraft>) => {
    setCueMetadataDrafts((current) => ({
      ...current,
      [cue.id]: {
        ...(current[cue.id] ?? cueMetadataDraftFromSummary(cue)),
        ...patch,
      },
    }));
  };

  const syncTimelineEventDrafts = (events: TimelineCueEventSummary[]) => {
    setTimelineEventDrafts((current) => {
      const nextDrafts: Record<number, TimelineEventDraft> = {};
      for (const event of events) {
        nextDrafts[event.id] = current[event.id] ?? timelineEventDraftFromSummary(event);
      }
      return nextDrafts;
    });
  };

  const timelineEventDraft = (event: TimelineCueEventSummary) =>
    timelineEventDrafts()[event.id] ?? timelineEventDraftFromSummary(event);

  const updateTimelineEventDraft = (event: TimelineCueEventSummary, patch: Partial<TimelineEventDraft>) => {
    setTimelineEventDrafts((current) => ({
      ...current,
      [event.id]: {
        ...(current[event.id] ?? timelineEventDraftFromSummary(event)),
        ...patch,
      },
    }));
  };

  const syncTimelineAutomationDrafts = (automations: TimelineAutomationSummary[]) => {
    setTimelineAutomationDrafts((current) => {
      const nextDrafts: Record<number, TimelineAutomationDraft> = {};
      for (const automation of automations) {
        nextDrafts[automation.id] = current[automation.id] ?? timelineAutomationDraftFromSummary(automation);
      }
      return nextDrafts;
    });
  };

  const timelineAutomationDraft = (automation: TimelineAutomationSummary) =>
    timelineAutomationDrafts()[automation.id] ?? timelineAutomationDraftFromSummary(automation);

  const updateTimelineAutomationDraft = (
    automation: TimelineAutomationSummary,
    patch: Partial<TimelineAutomationDraft>,
  ) => {
    setTimelineAutomationDrafts((current) => {
      const currentDraft = current[automation.id] ?? timelineAutomationDraftFromSummary(automation);
      const nextDraft = { ...currentDraft, ...patch };
      if (patch.fixture_id !== undefined) {
        const attributes = fixtureAttributeOptions(patch.fixture_id);
        if (!attributes.includes(nextDraft.attribute)) {
          nextDraft.attribute = attributes[0] ?? "";
        }
      }
      return {
        ...current,
        [automation.id]: nextDraft,
      };
    });
  };

  const syncTimelineVideoAutomationDrafts = (automations: TimelineVideoAutomationSummary[]) => {
    setTimelineVideoAutomationDrafts((current) => {
      const nextDrafts: Record<number, TimelineVideoAutomationDraft> = {};
      for (const automation of automations) {
        nextDrafts[automation.id] =
          current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation);
      }
      return nextDrafts;
    });
  };

  const timelineVideoAutomationDraft = (automation: TimelineVideoAutomationSummary) =>
    timelineVideoAutomationDrafts()[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation);

  const updateTimelineVideoAutomationDraft = (
    automation: TimelineVideoAutomationSummary,
    patch: Partial<TimelineVideoAutomationDraft>,
  ) => {
    setTimelineVideoAutomationDrafts((current) => ({
      ...current,
      [automation.id]: {
        ...(current[automation.id] ?? timelineVideoAutomationDraftFromSummary(automation)),
        ...patch,
      },
    }));
  };

  const markProjectClean = (nextSnapshot = snapshot()) => {
    setCleanProjectSignature(projectSnapshotSignature(nextSnapshot));
    setProjectDirty(false);
  };

  const applyEngineSnapshot = (next: EngineSnapshot) => {
    setSnapshot(next);
    const signature = projectSnapshotSignature(next);
    const cleanSignature = cleanProjectSignature();
    if (cleanSignature === null) {
      setCleanProjectSignature(signature);
      setProjectDirty(false);
    } else {
      setProjectDirty(signature !== cleanSignature);
    }
    setOutput(next.output);
    setDmxOutputRoutes(next.dmx_outputs.length > 0 ? next.dmx_outputs : [next.output]);
    setFaderValues((current) => ({ ...current, ...snapshotFaderValues(next) }));
    syncVideoOutputConfigDrafts(next.video.outputs);
    syncCueMetadataDrafts(next.cues);
    syncTimelineEventDrafts(next.timeline.events);
    syncTimelineAutomationDrafts(next.timeline.automations);
    syncTimelineVideoAutomationDrafts(next.timeline.video_automations);
    const groupId = selectedFixtureGroupFilter();
    if (groupId && !next.fixtures.some((fixture) => fixture.group_ids.includes(groupId))) {
      setSelectedFixtureGroupFilter(null);
    }
    const selectedId = selectedFixtureId();
    const selectedExists = selectedId !== null && next.fixtures.some((fixture) => fixture.id === selectedId);
    const liveFixtureIds = new Set(next.fixtures.map((fixture) => fixture.id));
    const nextMappingSelection = selectedMappingFixtureIds().filter((id) => liveFixtureIds.has(id));
    if (!selectedExists && next.fixtures.length > 0) {
      selectFixture(next.fixtures[0]);
    } else if (!selectedExists) {
      setSelectedFixtureId(null);
      setSelectedMappingFixtureIds([]);
      setSelectedFixtureLabelDraft("");
      setSelectedFixtureUniverseDraft(0);
      setSelectedFixtureAddressDraft(1);
      setSelectedFixtureGroupText("");
    } else if (nextMappingSelection.length !== selectedMappingFixtureIds().length) {
      setSelectedMappingFixtureIds(nextMappingSelection.length > 0 ? nextMappingSelection : [selectedId]);
    }
  };

  const refreshSnapshot = async () => {
    try {
      const next = await invoke<EngineSnapshot>("get_snapshot");
      applyEngineSnapshot(next);
      return next;
    } catch (error) {
      setMessage(String(error));
      return null;
    }
  };

  const refreshEngineTelemetryReport = async () => {
    try {
      setEngineTelemetryReport(await invoke<EngineTelemetryReport>("get_engine_telemetry_report"));
    } catch (error) {
      setMessage(String(error));
    }
  };

  const timer = window.setInterval(refreshSnapshot, 250);
  const telemetryReportTimer = window.setInterval(refreshEngineTelemetryReport, 1000);
  onCleanup(() => {
    window.clearInterval(timer);
    window.clearInterval(telemetryReportTimer);
  });
  createEffect(() => {
    void refreshSnapshot();
    void refreshEngineTelemetryReport();
    void refreshMidiInputs();
    void refreshMidiOutputs();
    void refreshSerialPorts();
  });
  createEffect(() => {
    void loadStartupProject();
    void loadQueuedOpenProjects();
    let disposed = false;
    let unlistenOpenProject: (() => void) | null = null;
    void listen<string[]>("rayard://open-project", (event) => {
      const paths = Array.isArray(event.payload) ? event.payload : [];
      const path = paths[paths.length - 1];
      if (path) {
        void loadProjectPath(path);
      }
    })
      .then((unlisten) => {
        if (disposed) {
          unlisten();
        } else {
          unlistenOpenProject = unlisten;
        }
      })
      .catch((error) => setMessage(String(error)));
    onCleanup(() => {
      disposed = true;
      unlistenOpenProject?.();
    });
  });
  createEffect(() => {
    const attribute = selectedEffectAttribute();
    if (attribute && !midiMapAttribute()) {
      setMidiMapAttribute(attribute);
    }
    if (attribute && !oscMapAttribute()) {
      setOscMapAttribute(attribute);
    }
  });
  createEffect(() => {
    const lastBank = cuePadBankCount() - 1;
    if (cuePadBank() > lastBank) {
      setCuePadBank(lastBank);
    }
  });
  createEffect(() => {
    if (!cuePadFollowActive()) {
      return;
    }
    const index = activeCueIndex();
    if (index < 0) {
      return;
    }
    const activeBank = Math.floor(index / cuePadSize);
    if (cuePadBank() !== activeBank) {
      setCuePadBank(activeBank);
    }
  });
  createEffect(() => {
    const typeKey = selectedFixtureTypeFilter();
    if (typeKey && !fixtureTypeRows().some((row) => row.key === typeKey)) {
      setSelectedFixtureTypeFilter(null);
    }
  });
  createEffect(() => {
    const outputs = snapshot().video.outputs;
    const outputId = selectedVideoOutputId();
    if (outputs.length === 0) {
      if (outputId !== null) {
        setSelectedVideoOutputId(null);
      }
      return;
    }
    if (outputId === null || !outputs.some((output) => output.id === outputId)) {
      setSelectedVideoOutputId(outputs[0].id);
    }
  });

  const selectGdtfFile = async () => {
    try {
      const path = await invoke<string | null>("select_gdtf_file");
      if (path) {
        setGdtfPath(path);
        setMessage(`Selected ${path}`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadGdtfProfile = async (path: string, loadedMessage = "Loaded") => {
    const imported = await invoke<FixtureProfileSummary>("import_gdtf", { path });
    setProfile(imported);
    setSelectedMode(imported.dmx_modes[0]?.name ?? "");
    setMessage(`${loadedMessage} ${imported.manufacturer} ${imported.name}`);
  };

  const importGdtf = async () => {
    try {
      await loadGdtfProfile(gdtfPath());
    } catch (error) {
      setMessage(String(error));
    }
  };

  const selectVideoSourceFile = async () => {
    const sourceKind = videoSourceKind();
    if (!videoSourceCanBrowseFile(sourceKind)) {
      setMessage("Named network/GPU sources do not use a local file picker.");
      return;
    }
    try {
      const path = await invoke<string | null>("select_video_source_file", { kind: sourceKind });
      if (!path) {
        setMessage("Video source selection canceled.");
        return;
      }
      setVideoPath(path);
      if (shouldReplaceVideoLayerDraftLabel(videoLabel())) {
        setVideoLabel(mediaLabelFromPath(path));
      }
      setMessage(`Selected ${path}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const downloadGdtfFromUrl = async () => {
    try {
      const path = await invoke<string | null>("download_gdtf_from_url", { url: gdtfShareUrl() });
      if (!path) {
        setMessage("GDTF download canceled.");
        return;
      }
      setGdtfPath(path);
      await loadGdtfProfile(path, "Downloaded and loaded");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const customProfileRequest = (): CustomFixtureProfileRequest => ({
    manufacturer: customManufacturer(),
    name: customProfileName(),
    mode_name: customModeName(),
    attributes: customAttributes()
      .split(",")
      .map((attribute) => attribute.trim())
      .filter(Boolean),
  });

  const createCustomProfile = async () => {
    const request = customProfileRequest();
    try {
      const created = await invoke<FixtureProfileSummary>("create_custom_fixture_profile", { request });
      setProfile(created);
      setGdtfPath(created.source_path);
      setSelectedMode(created.dmx_modes[0]?.name ?? "");
      setMessage(`Created custom profile ${created.manufacturer} ${created.name}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveCustomProfile = async () => {
    const request = customProfileRequest();
    try {
      const path = await invoke<string | null>("save_custom_fixture_profile", { request });
      setMessage(path ? `Saved custom profile ${path}` : "Custom profile save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadCustomProfile = async () => {
    try {
      const created = await invoke<FixtureProfileSummary | null>("load_custom_fixture_profile");
      if (!created) {
        setMessage("Custom profile load canceled.");
        return;
      }
      setProfile(created);
      setGdtfPath(created.source_path);
      setSelectedMode(created.dmx_modes[0]?.name ?? "");
      setCustomManufacturer(created.manufacturer);
      setCustomProfileName(created.name);
      setCustomModeName(created.dmx_modes[0]?.name ?? "Default");
      setCustomAttributes(
        created.dmx_modes[0]?.controls
          .map((control) => `${control.attribute}@${control.offsets[0] ?? 1}:${control.resolution === "SixteenBit" ? "16" : "8"}`)
          .join(", ") ?? "",
      );
      setMessage(`Loaded custom profile ${created.manufacturer} ${created.name}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const useFixtureProfileForPatch = async (fixture: PatchedFixtureSummary) => {
    try {
      const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
      const footprint = Math.max(1, fixtureFootprint(fixture));
      setProfile(imported);
      setGdtfPath(imported.source_path);
      setSelectedMode(fixture.mode_name);
      setLabel(`${fixture.label} Copy`);
      setUniverse(fixture.universe);
      setAddress(Math.min(512, fixture.address + footprint));
      setPatchCount(1);
      setPatchAddressStride(footprint);
      setGroupText(fixture.group_ids.join(", "));
      setPatchX(fixture.position.x + 1);
      setPatchY(fixture.position.y);
      setPatchZ(fixture.position.z);
      setPatchPitch(fixture.rotation.pitch);
      setPatchYaw(fixture.rotation.yaw);
      setPatchRoll(fixture.rotation.roll);
      setMessage(`Using ${fixture.label}'s profile for patching`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateFixture = async (fixture: PatchedFixtureSummary) => {
    try {
      const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
      const footprint = Math.max(1, fixtureFootprint(fixture));
      const address = fixture.address + footprint;
      if (address > 512) {
        setMessage(`Cannot duplicate ${fixture.label}: next address ${address} exceeds universe 512.`);
        return;
      }
      const request: PatchFixtureRequest = {
        profile_path: imported.source_path,
        mode_name: fixture.mode_name,
        label: `${fixture.label} Copy`,
        universe: fixture.universe,
        address,
        group_ids: fixture.group_ids,
        position: {
          x: Number((fixture.position.x + 1).toFixed(2)),
          y: Number(fixture.position.y.toFixed(2)),
          z: Number(fixture.position.z.toFixed(2)),
        },
        rotation: fixture.rotation,
      };
      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests: [request] });
      const fixtureId = fixtureIds[0];
      if (fixtureId === undefined) {
        setMessage("Fixture duplicate did not return a fixture id.");
        return;
      }
      await invoke("set_fixture_limits", { fixtureId, limits: fixture.limits });
      setSelectedFixtureId(fixtureId);
      setSelectedMappingFixtureIds([fixtureId]);
      setSelectedFixtureLabelDraft(request.label);
      setSelectedFixtureUniverseDraft(request.universe);
      setSelectedFixtureAddressDraft(request.address);
      setSelectedFixtureGroupText(request.group_ids.join(", "));
      setSelectedFixtureLimitsDraft(fixture.limits);
      setMessage(`Duplicated ${fixture.label} as ${request.label} at U${request.universe} A${request.address}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const buildOccupiedDmxRanges = () => {
    const ranges = new Map<number, DmxAddressRange[]>();
    for (const fixture of snapshot().fixtures) {
      const range = addressRange(fixture.address, fixtureFootprint(fixture));
      if (!range) {
        continue;
      }
      const universeRanges = ranges.get(fixture.universe) ?? [];
      universeRanges.push({ start: range[0], end: Math.min(512, range[1]) });
      ranges.set(fixture.universe, universeRanges);
    }
    return ranges;
  };

  const dmxAddressIsFree = (
    ranges: Map<number, DmxAddressRange[]>,
    universe: number,
    start: number,
    footprint: number,
  ) => {
    const end = start + footprint - 1;
    if (start < 1 || end > 512) {
      return false;
    }
    return !(ranges.get(universe) ?? []).some((range) => rangesOverlap([start, end], [range.start, range.end]));
  };

  const reserveDmxAddressRange = (
    ranges: Map<number, DmxAddressRange[]>,
    universe: number,
    start: number,
    footprint: number,
  ) => {
    const universeRanges = ranges.get(universe) ?? [];
    universeRanges.push({ start, end: start + footprint - 1 });
    ranges.set(universe, universeRanges);
  };

  const findFreeDmxAddress = (
    ranges: Map<number, DmxAddressRange[]>,
    universe: number,
    footprint: number,
    preferredStart: number,
  ) => {
    const maxStart = 512 - footprint + 1;
    if (maxStart < 1) {
      return null;
    }
    const startHint = clampRange(Math.floor(preferredStart), 1, maxStart);
    for (let candidate = startHint; candidate <= maxStart; candidate += 1) {
      if (dmxAddressIsFree(ranges, universe, candidate, footprint)) {
        return candidate;
      }
    }
    for (let candidate = 1; candidate < startHint; candidate += 1) {
      if (dmxAddressIsFree(ranges, universe, candidate, footprint)) {
        return candidate;
      }
    }
    return null;
  };

  const duplicateSelectedMappingFixtures = async () => {
    const fixtures = [...selectedMappingFixtures()].sort((left, right) =>
      left.universe === right.universe ? left.address - right.address : left.universe - right.universe,
    );
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }

    const occupiedRanges = buildOccupiedDmxRanges();
    const nextAddressByUniverse = new Map<number, number>();
    for (const [universeId, ranges] of occupiedRanges) {
      const maxEnd = Math.max(0, ...ranges.map((range) => range.end));
      nextAddressByUniverse.set(universeId, maxEnd + 1);
    }
    const offset = mappingSnapEnabled() ? normalizedMappingSnapSize() : 1;
    const sourceFixtures: PatchedFixtureSummary[] = [];
    const requests: PatchFixtureRequest[] = [];

    try {
      for (const fixture of fixtures) {
        const imported = await invoke<FixtureProfileSummary>("use_fixture_profile", { fixtureId: fixture.id });
        const footprint = Math.max(1, fixtureFootprint(fixture));
        const preferredAddress = Math.max(
          fixture.address + footprint,
          nextAddressByUniverse.get(fixture.universe) ?? 1,
        );
        const nextAddress = findFreeDmxAddress(occupiedRanges, fixture.universe, footprint, preferredAddress);
        if (nextAddress === null) {
          setMessage(`Cannot duplicate ${fixture.label}: no ${footprint}ch gap remains in universe ${fixture.universe}.`);
          return;
        }
        reserveDmxAddressRange(occupiedRanges, fixture.universe, nextAddress, footprint);
        nextAddressByUniverse.set(fixture.universe, nextAddress + footprint);
        sourceFixtures.push(fixture);
        requests.push({
          profile_path: imported.source_path,
          mode_name: fixture.mode_name,
          label: `${fixture.label} Copy`,
          universe: fixture.universe,
          address: nextAddress,
          group_ids: [...fixture.group_ids],
          position: snapStagePosition({
            x: fixture.position.x + offset,
            y: fixture.position.y,
            z: fixture.position.z + offset,
          }),
          rotation: { ...fixture.rotation },
        });
      }

      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests });
      if (fixtureIds.length !== requests.length) {
        setMessage(`Fixture duplicate returned ${fixtureIds.length}/${requests.length} fixture id(s).`);
        await refreshSnapshot();
        return;
      }

      const nextFaderValues: Record<string, number> = {};
      for (const [index, fixtureId] of fixtureIds.entries()) {
        const source = sourceFixtures[index];
        await invoke("set_fixture_limits", { fixtureId, limits: source.limits });
        for (const value of source.attribute_values) {
          await invoke("set_attribute", {
            fixtureId,
            attribute: value.attribute,
            value: value.value,
          });
          nextFaderValues[`${fixtureId}:${value.attribute}`] = value.value;
        }
      }

      setFaderValues((current) => ({ ...current, ...nextFaderValues }));
      setSelectedMappingFixtureIds(fixtureIds);
      const firstRequest = requests[0];
      const firstId = fixtureIds[0];
      if (firstRequest && firstId !== undefined) {
        setSelectedFixtureId(firstId);
        setSelectedFixtureLabelDraft(firstRequest.label);
        setSelectedFixtureUniverseDraft(firstRequest.universe);
        setSelectedFixtureAddressDraft(firstRequest.address);
        setSelectedFixtureGroupText(firstRequest.group_ids.join(", "));
        setSelectedFixtureLimitsDraft(sourceFixtures[0]?.limits ?? defaultFixtureLimits);
      }
      setMessage(
        fixtures.length === 1
          ? `Duplicated ${fixtures[0].label} as ${firstRequest?.label ?? "copy"}.`
          : `Duplicated ${fixtures.length} selected fixtures.`,
      );
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const patchFixture = async () => {
    const imported = profile();
    if (!imported) {
      setMessage("Load a GDTF profile first.");
      return;
    }
    if (selectedFootprint() === 0) {
      setMessage("Selected GDTF mode has no DMX channel offsets.");
      return;
    }
    if (endAddress() > 512) {
      setMessage(`Fixture exceeds DMX universe: start ${address()}, footprint ${selectedFootprint()}ch, end ${endAddress()}`);
      return;
    }
    const conflict = patchAddressConflictText();
    if (conflict) {
      setMessage(`DMX address conflict: ${patchAddressConflictText()}`);
      return;
    }
    const count = patchCountValue();
    const addressStride = patchAddressStrideValue();
    const groupIds = parseGroupIds(groupText());
    const baseLabel = label().trim() || "Fixture";
    const requests: PatchFixtureRequest[] = Array.from({ length: count }, (_, index) => {
      const position = patchFixturePosition(index, count);
      return {
        profile_path: imported.source_path,
        mode_name: selectedMode() || null,
        label: bulkPatchLabel(baseLabel, index, count),
        universe: universe(),
        address: address() + index * addressStride,
        group_ids: groupIds,
        position: {
          x: Number(position.x.toFixed(2)),
          y: Number(position.y.toFixed(2)),
          z: Number(position.z.toFixed(2)),
        },
        rotation: { pitch: patchPitch(), yaw: patchYaw(), roll: patchRoll() },
      };
    });

    try {
      const fixtureIds = await invoke<number[]>("patch_fixtures", { requests });
      const fixtureId = fixtureIds[fixtureIds.length - 1];
      const request = requests[requests.length - 1];
      setSelectedFixtureId(fixtureId);
      setSelectedFixtureLabelDraft(request.label);
      setSelectedFixtureUniverseDraft(request.universe);
      setSelectedFixtureAddressDraft(request.address);
      setSelectedFixtureGroupText(request.group_ids.join(", "));
      const initialValues: Record<string, number> = {};
      for (const patchedFixtureId of fixtureIds) {
        for (const control of selectedModeSummary()?.controls ?? []) {
          initialValues[`${patchedFixtureId}:${control.attribute}`] = control.default_value;
        }
      }
      setFaderValues((current) => ({ ...current, ...initialValues }));
      setMessage(count === 1 ? `Patched fixture ${fixtureId}` : `Patched ${fixtureIds.length} fixtures`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeFixture = async (fixtureId: number) => {
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    if (fixture && !window.confirm(`Remove fixture ${fixture.label}?`)) {
      return;
    }
    try {
      await invoke("remove_fixture", { fixtureId });
      if (selectedFixtureId() === fixtureId) {
        setSelectedFixtureId(null);
        setSelectedFixtureLabelDraft("");
        setSelectedFixtureUniverseDraft(0);
        setSelectedFixtureAddressDraft(1);
        setSelectedFixtureGroupText("");
      }
      setFaderValues((current) => {
        const next = { ...current };
        for (const key of Object.keys(next)) {
          if (key.startsWith(`${fixtureId}:`)) {
            delete next[key];
          }
        }
        return next;
      });
      setMessage(`Removed fixture ${fixtureId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setAttribute = async (fixtureId: number, attribute: string, value: number) => {
    setFaderValues((current) => ({ ...current, [`${fixtureId}:${attribute}`]: value }));
    try {
      await invoke("set_attribute", {
        fixtureId,
        attribute,
        value,
      });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupAttribute = async (groupId: string, attribute: string, value: number) => {
    const fixtureIds = snapshot()
      .fixtures
      .filter((fixture) => fixture.group_ids.includes(groupId))
      .map((fixture) => fixture.id);
    setFaderValues((current) => {
      const next = { ...current };
      for (const fixtureId of fixtureIds) {
        next[`${fixtureId}:${attribute}`] = value;
      }
      return next;
    });
    try {
      await invoke("set_group_attribute", {
        groupId,
        attribute,
        value,
      });
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureTransform = async (
    fixture: PatchedFixtureSummary,
    next: {
      position?: PatchFixtureRequest["position"];
      rotation?: PatchFixtureRequest["rotation"];
    },
    refresh = true,
  ) => {
    try {
      await invoke("set_fixture_transform", {
        fixtureId: fixture.id,
        position: next.position ?? fixture.position,
        rotation: next.rotation ?? fixture.rotation,
      });
      if (refresh) {
        await refreshSnapshot();
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const stageSvgPointFromClient = (clientX: number, clientY: number, targetSvg: SVGSVGElement) => {
    const rect = targetSvg.getBoundingClientRect();
    const viewBox = mappingViewportBox();
    return {
      x: clampRange(viewBox.x + ((clientX - rect.left) / rect.width) * viewBox.size, 0, stageViewBoxSize),
      z: clampRange(viewBox.z + ((clientY - rect.top) / rect.height) * viewBox.size, 0, stageViewBoxSize),
    };
  };

  const stageWorldPointFromPointer = (event: PointerEvent, svg?: SVGSVGElement) => {
    const point = stageSvgPointFromPointer(event, svg);
    return svgPointToStageWorld(point.x, point.z, stageWorldBounds());
  };

  const stageSvgPointFromPointer = (event: PointerEvent, svg?: SVGSVGElement) => {
    const targetSvg = svg ?? (event.currentTarget as SVGSVGElement);
    return stageSvgPointFromClient(event.clientX, event.clientY, targetSvg);
  };

  const handleMappingStageWheel = (event: WheelEvent & { currentTarget: SVGSVGElement }) => {
    event.preventDefault();
    const point = stageSvgPointFromClient(event.clientX, event.clientY, event.currentTarget);
    zoomMappingViewportAtPoint(event.deltaY < 0 ? 1 : -1, point);
  };

  const placeSelectedFixtureFromStage = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    const fixture = selectedFixture();
    if (!fixture) {
      return;
    }
    const point = snapStagePoint(stageWorldPointFromPointer(event));
    await setFixtureTransform(fixture, {
      position: {
        ...fixture.position,
        x: point.x,
        z: point.z,
      },
    });
  };

  const rotateSelectedFixtureFromStage = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    const fixture = selectedFixture();
    if (!fixture) {
      return;
    }
    const point = stageWorldPointFromPointer(event);
    const dx = point.x - fixture.position.x;
    const dz = point.z - fixture.position.z;
    if (Math.abs(dx) < 0.001 && Math.abs(dz) < 0.001) {
      return;
    }
    const yaw = Math.round(((Math.atan2(dz, dx) * 180) / Math.PI + 90 + 360) % 360);
    await setFixtureTransform(fixture, {
      rotation: {
        ...fixture.rotation,
        yaw,
      },
    });
  };

  const beginMappingFixtureDrag = (event: PointerEvent, fixtureId: number) => {
    if (event.button !== 0 || mappingStageTool() !== "select") {
      return;
    }
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!fixture || !svg) {
      return;
    }
    if (isAdditiveMappingSelectionEvent(event)) {
      selectMappingFixture(fixture, event);
      return;
    }
    const selectedIds = selectedMappingFixtureIdSet();
    const fixtureIds = selectedIds.has(fixtureId) ? selectedMappingFixtureIds() : [fixtureId];
    if (!selectedIds.has(fixtureId)) {
      selectFixture(fixture);
    } else {
      activateFixture(fixture);
    }
    const startPositions = Object.fromEntries(
      snapshot()
        .fixtures
        .filter((candidate) => fixtureIds.includes(candidate.id))
        .map((candidate) => [candidate.id, candidate.position]),
    ) as Record<number, PatchFixtureRequest["position"]>;
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    setMappingDrag({
      kind: "fixture",
      pointerId: event.pointerId,
      fixtureIds,
      startWorld: point,
      currentWorld: point,
      startPositions,
    });
  };

  const beginMappingVideoOutputDrag = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || mappingStageTool() !== "select") {
      return;
    }
    const output = snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) {
      return;
    }
    const point = stageWorldPointFromPointer(event, svg);
    svg.setPointerCapture(event.pointerId);
    setMappingDrag({
      kind: "videoOutput",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      startMapping: output.mapping,
    });
  };

  const beginMappingVideoOutputRotate = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || mappingStageTool() !== "select") {
      return;
    }
    const output = snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) {
      return;
    }
    const point = stageWorldPointFromPointer(event, svg);
    const centerWorld = {
      x: output.mapping.stage_x,
      z: output.mapping.stage_z,
    };
    svg.setPointerCapture(event.pointerId);
    setMappingDrag({
      kind: "videoOutputRotate",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      centerWorld,
      startAngleDeg: mappingOutputHandleAngleDeg(centerWorld, point),
      startMapping: output.mapping,
    });
  };

  const beginMappingVideoOutputScale = (event: PointerEvent, outputId: number) => {
    if (event.button !== 0 || mappingStageTool() !== "select") {
      return;
    }
    const output = snapshot().video.outputs.find((candidate) => candidate.id === outputId);
    const svg = (event.currentTarget as SVGElement).ownerSVGElement;
    if (!output || !svg) {
      return;
    }
    const point = stageWorldPointFromPointer(event, svg);
    const centerWorld = {
      x: output.mapping.stage_x,
      z: output.mapping.stage_z,
    };
    svg.setPointerCapture(event.pointerId);
    setMappingDrag({
      kind: "videoOutputScale",
      pointerId: event.pointerId,
      outputId,
      startWorld: point,
      currentWorld: point,
      centerWorld,
      startDistance: mappingOutputHandleDistance(centerWorld, point),
      startMapping: output.mapping,
    });
  };

  const beginMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (event.button !== 0 || mappingStageTool() !== "pan") {
      return;
    }
    const rect = event.currentTarget.getBoundingClientRect();
    const box = mappingViewportBox();
    event.preventDefault();
    event.currentTarget.setPointerCapture(event.pointerId);
    setMappingViewportPanDrag({
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startCenterX: box.x + box.size / 2,
      startCenterZ: box.z + box.size / 2,
      viewBoxSize: box.size,
      rectWidth: Math.max(1, rect.width),
      rectHeight: Math.max(1, rect.height),
    });
  };

  const updateMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const drag = mappingViewportPanDrag();
    if (!drag || drag.pointerId !== event.pointerId) {
      return false;
    }
    const deltaX = ((event.clientX - drag.startClientX) / drag.rectWidth) * drag.viewBoxSize;
    const deltaZ = ((event.clientY - drag.startClientY) / drag.rectHeight) * drag.viewBoxSize;
    setMappingViewport(normalizedMappingViewportZoom(), drag.startCenterX - deltaX, drag.startCenterZ - deltaZ);
    return true;
  };

  const finishMappingViewportPan = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const drag = mappingViewportPanDrag();
    if (!drag || drag.pointerId !== event.pointerId) {
      return false;
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setMappingViewportPanDrag(null);
    return true;
  };

  const beginMappingMarquee = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (event.button !== 0 || mappingStageTool() !== "select") {
      return;
    }
    const point = stageSvgPointFromPointer(event);
    event.currentTarget.setPointerCapture(event.pointerId);
    setMappingMarquee({
      pointerId: event.pointerId,
      start: point,
      current: point,
      additive: isAdditiveMappingSelectionEvent(event),
    });
  };

  const handleMappingStagePointerDown = async (
    event: PointerEvent & { currentTarget: SVGSVGElement },
  ) => {
    if (event.button !== 0) {
      return;
    }
    if (mappingStageTool() === "place") {
      await placeSelectedFixtureFromStage(event);
    } else if (mappingStageTool() === "rotate") {
      await rotateSelectedFixtureFromStage(event);
    } else if (mappingStageTool() === "pan") {
      beginMappingViewportPan(event);
    } else {
      beginMappingMarquee(event);
    }
  };

  const handleMappingStagePointerMove = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (updateMappingViewportPan(event)) {
      return;
    }

    const drag = mappingDrag();
    if (drag && drag.pointerId === event.pointerId) {
      setMappingDrag({
        ...drag,
        currentWorld: stageWorldPointFromPointer(event),
      });
      return;
    }

    const marquee = mappingMarquee();
    if (!marquee || marquee.pointerId !== event.pointerId) {
      return;
    }
    setMappingMarquee({
      ...marquee,
      current: stageSvgPointFromPointer(event),
    });
  };

  const finishMappingStageDrag = async (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    if (finishMappingViewportPan(event)) {
      return;
    }

    const drag = mappingDrag();
    if (drag && drag.pointerId === event.pointerId) {
      if (event.currentTarget.hasPointerCapture(event.pointerId)) {
        event.currentTarget.releasePointerCapture(event.pointerId);
      }
      const delta = dragWorldDelta(drag);
      setMappingDrag(null);
      if (Math.abs(delta.x) < 0.01 && Math.abs(delta.z) < 0.01) {
        return;
      }

      if (drag.kind === "fixture") {
        const movedFixtures = snapshot().fixtures.filter((candidate) => drag.fixtureIds.includes(candidate.id));
        if (movedFixtures.length === 0) {
          return;
        }
        await Promise.all(
          movedFixtures.map((fixture) => {
            const startPosition = drag.startPositions[fixture.id] ?? fixture.position;
            const nextPosition = snapStagePosition({
              ...startPosition,
              x: startPosition.x + delta.x,
              z: startPosition.z + delta.z,
            });
            return setFixtureTransform(fixture, { position: nextPosition }, false);
          }),
        );
        await refreshSnapshot();
        setMessage(
          movedFixtures.length === 1
            ? `Moved ${movedFixtures[0].label}`
            : `Moved ${movedFixtures.length} selected fixtures`,
        );
        return;
      }

      const output = snapshot().video.outputs.find((candidate) => candidate.id === drag.outputId);
      if (!output) {
        return;
      }
      const nextMapping = mappingVideoOutputPreviewMapping(drag, output);
      await setVideoOutputMapping(output.id, nextMapping);
      if (drag.kind === "videoOutput") {
        setMessage(`Moved ${output.label} to X ${nextMapping.stage_x}, Z ${nextMapping.stage_z}`);
      } else if (drag.kind === "videoOutputRotate") {
        setMessage(`Rotated ${output.label} to ${nextMapping.rotation_deg} deg`);
      } else {
        setMessage(`Scaled ${output.label} to ${nextMapping.scale_x.toFixed(2)} x ${nextMapping.scale_y.toFixed(2)}`);
      }
      return;
    }

    const marquee = mappingMarquee();
    if (!marquee || marquee.pointerId !== event.pointerId) {
      return;
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    const box = {
      x: Math.min(marquee.start.x, marquee.current.x),
      z: Math.min(marquee.start.z, marquee.current.z),
      width: Math.abs(marquee.current.x - marquee.start.x),
      height: Math.abs(marquee.current.z - marquee.start.z),
    };
    setMappingMarquee(null);
    if (box.width < 0.8 && box.height < 0.8) {
      if (!marquee.additive) {
        clearMappingFixtureSelection();
      }
      return;
    }
    const pickedIds = visualizerFixtures()
      .filter((fixture) =>
        fixture.x >= box.x &&
        fixture.x <= box.x + box.width &&
        fixture.z >= box.z &&
        fixture.z <= box.z + box.height,
      )
      .map((fixture) => fixture.id);
    const nextIds = marquee.additive ? [...new Set([...selectedMappingFixtureIds(), ...pickedIds])] : pickedIds;
    setSelectedMappingFixtureIds(nextIds);
    const active = snapshot().fixtures.find((fixture) => fixture.id === nextIds[0]);
    if (active) {
      activateFixture(active);
    } else if (!marquee.additive) {
      setSelectedFixtureId(null);
    }
    setMessage(`Selected ${nextIds.length} fixture${nextIds.length === 1 ? "" : "s"}`);
  };

  const layoutFixtures = async (fixtures: PatchedFixtureSummary[], mode: FixtureLayoutMode, scopeLabel: string) => {
    if (fixtures.length === 0) {
      setMessage(`No fixtures to arrange for ${scopeLabel}.`);
      return;
    }
    const count = fixtures.length;
    const spacing = 2;
    const radius = Math.max(3, count * 0.55);
    const columns = Math.ceil(Math.sqrt(count));
    const rows = Math.ceil(count / columns);

    await Promise.all(
      fixtures.map((fixture, index) => {
        let x = fixture.position.x;
        let z = fixture.position.z;
        if (mode === "line") {
          x = (index - (count - 1) / 2) * spacing;
          z = 0;
        } else if (mode === "grid") {
          const column = index % columns;
          const row = Math.floor(index / columns);
          x = (column - (columns - 1) / 2) * spacing;
          z = (row - (rows - 1) / 2) * spacing;
        } else {
          const angle = (index / count) * Math.PI * 2 - Math.PI / 2;
          x = Math.cos(angle) * radius;
          z = Math.sin(angle) * radius;
        }
        return setFixtureTransform(
          fixture,
          {
            position: snapStagePosition({
              ...fixture.position,
              x,
              z,
            }),
          },
          false,
        );
      }),
    );
    setMessage(
      `Applied ${mode} layout to ${count} fixture${count === 1 ? "" : "s"} ${scopeLabel}.`,
    );
    await refreshSnapshot();
  };

  const layoutFixturePositions = async (mode: FixtureLayoutMode) => {
    await layoutFixtures(
      filteredFixtures(),
      mode,
      selectedFixtureGroupFilter() ? `in ${selectedFixtureGroupFilter()}` : "in patch",
    );
  };

  const layoutSelectedMappingFixtures = async (mode: FixtureLayoutMode) => {
    await layoutFixtures(selectedMappingFixtures(), mode, "in selection");
  };

  const alignSelectedMappingFixtures = async (axis: MappingAxis) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length < 2) {
      setMessage("Select at least two fixtures to align.");
      return;
    }
    const active = selectedFixture();
    const anchor = active && fixtures.some((fixture) => fixture.id === active.id) ? active : fixtures[0];
    const targetValue = anchor.position[axis];
    try {
      await Promise.all(
        fixtures.map((fixture) =>
          setFixtureTransform(
            fixture,
            {
              position: snapStagePosition({
                ...fixture.position,
                [axis]: targetValue,
              }),
            },
            false,
          ),
        ),
      );
      await refreshSnapshot();
      setMessage(`Aligned ${fixtures.length} selected fixture(s) on ${axis.toUpperCase()} ${targetValue.toFixed(2)}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const distributeSelectedMappingFixtures = async (axis: MappingAxis) => {
    const fixtures = [...selectedMappingFixtures()].sort((left, right) => left.position[axis] - right.position[axis]);
    if (fixtures.length < 3) {
      setMessage("Select at least three fixtures to distribute.");
      return;
    }
    const first = fixtures[0].position[axis];
    const last = fixtures[fixtures.length - 1].position[axis];
    if (Math.abs(last - first) < 0.001) {
      setMessage(`Selected fixtures need different ${axis.toUpperCase()} positions before distributing.`);
      return;
    }
    const step = (last - first) / (fixtures.length - 1);
    try {
      await Promise.all(
        fixtures.map((fixture, index) =>
          setFixtureTransform(
            fixture,
            {
              position: snapStagePosition({
                ...fixture.position,
                [axis]: first + step * index,
              }),
            },
            false,
          ),
        ),
      );
      await refreshSnapshot();
      setMessage(`Distributed ${fixtures.length} selected fixture(s) along ${axis.toUpperCase()}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const nudgeSelectedMappingFixtures = async (deltaX: number, deltaZ: number) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    try {
      await Promise.all(
        fixtures.map((fixture) =>
          setFixtureTransform(
            fixture,
            {
              position: snapStagePosition({
                ...fixture.position,
                x: fixture.position.x + deltaX,
                z: fixture.position.z + deltaZ,
              }),
            },
            false,
          ),
        ),
      );
      await refreshSnapshot();
      setMessage(`Nudged ${fixtures.length} selected fixture(s).`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setStageMapConfig = async (updates: Partial<StageMapConfig>, successMessage = "Updated 2D stage map.") => {
    const nextConfig = {
      ...snapshot().stage_map,
      ...updates,
    };
    try {
      await invoke("set_stage_map_config", { config: nextConfig });
      setMessage(successMessage);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const lockStageMapToCurrentBounds = async () => {
    const bounds = autoStageWorldBounds();
    await setStageMapConfig(
      {
        locked: true,
        min_x: Number(bounds.minX.toFixed(2)),
        max_x: Number(bounds.maxX.toFixed(2)),
        min_z: Number(bounds.minZ.toFixed(2)),
        max_z: Number(bounds.maxZ.toFixed(2)),
      },
      "Locked 2D stage map to current bounds.",
    );
  };

  const saveStageMapPreset = async () => {
    try {
      const label = await invoke<string>("save_stage_map_preset", {
        label: stageMapPresetLabel(),
        config: snapshot().stage_map,
      });
      setStageMapPresetLabel(label);
      setSelectedStageMapPresetLabel(label);
      setMessage(`Saved stage map preset ${label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyStageMapPreset = async (label: string) => {
    try {
      await invoke("apply_stage_map_preset", { label });
      setSelectedStageMapPresetLabel(label);
      setMessage(`Applied stage map preset ${label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeStageMapPreset = async (label: string) => {
    try {
      await invoke("remove_stage_map_preset", { label });
      if (selectedStageMapPresetLabel() === label) {
        setSelectedStageMapPresetLabel("");
      }
      setMessage(`Removed stage map preset ${label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const exportStageMapPreset = async () => {
    try {
      const path = await invoke<string | null>("save_stage_map_preset_file", {
        label: stageMapPresetLabel(),
        config: snapshot().stage_map,
      });
      setMessage(path ? `Exported stage map preset ${path}` : "Stage map preset export canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const importStageMapPreset = async () => {
    try {
      const label = await invoke<string | null>("load_stage_map_preset_file");
      if (label === null) {
        setMessage("Stage map preset import canceled.");
        return;
      }
      setStageMapPresetLabel(label);
      setSelectedStageMapPresetLabel(label);
      setMessage(`Imported stage map preset ${label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyMappingSelectionGroups = async (mode: MappingBulkGroupMode) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const groupIds = uniqueGroupIds(parseGroupIds(mappingSelectionGroupText()));
    if (groupIds.length === 0) {
      setMessage("Enter one or more group IDs.");
      return;
    }
    const groupIdSet = new Set(groupIds);
    try {
      for (const fixture of fixtures) {
        const nextGroupIds =
          mode === "set"
            ? groupIds
            : mode === "add"
              ? uniqueGroupIds([...fixture.group_ids, ...groupIds])
              : fixture.group_ids.filter((groupId) => !groupIdSet.has(groupId));
        await invoke("set_fixture_groups", {
          fixtureId: fixture.id,
          groupIds: nextGroupIds,
        });
      }
      setMappingSelectionGroupText(groupIds.join(", "));
      const actionLabel = mode === "set" ? "Set" : mode === "add" ? "Added" : "Removed";
      setMessage(`${actionLabel} groups for ${fixtures.length} selected fixture(s).`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setMappingSelectionFlag = async (flag: MappingFixtureFlag, enabled: boolean) => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const command =
      flag === "highlight"
        ? "set_fixture_highlight"
        : flag === "solo"
          ? "set_fixture_solo"
          : "set_fixture_park";
    try {
      for (const fixture of fixtures) {
        await invoke(command, { fixtureId: fixture.id, enabled });
      }
      const label = flag === "highlight" ? "Highlight" : flag === "solo" ? "Solo" : "Park";
      setMessage(`${enabled ? "Set" : "Cleared"} ${label} for ${fixtures.length} selected fixture(s).`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeSelectedMappingFixtures = async () => {
    const fixtures = selectedMappingFixtures();
    if (fixtures.length === 0) {
      setMessage("Select fixtures on the mapping stage first.");
      return;
    }
    const removeLabel = fixtures.length === 1 ? fixtures[0].label : `${fixtures.length} selected fixtures`;
    if (!window.confirm(`Remove ${removeLabel}?`)) {
      return;
    }
    const removedIds = new Set(fixtures.map((fixture) => fixture.id));
    try {
      for (const fixture of fixtures) {
        await invoke("remove_fixture", { fixtureId: fixture.id });
      }
      setFaderValues((current) => {
        const next = { ...current };
        for (const key of Object.keys(next)) {
          const fixtureId = Number(key.split(":")[0]);
          if (removedIds.has(fixtureId)) {
            delete next[key];
          }
        }
        return next;
      });
      setSelectedMappingFixtureIds([]);
      setSelectedFixtureId(null);
      setSelectedFixtureLabelDraft("");
      setSelectedFixtureUniverseDraft(0);
      setSelectedFixtureAddressDraft(1);
      setSelectedFixtureGroupText("");
      setMessage(`Removed ${removeLabel}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixturePatch = async (fixture: PatchedFixtureSummary) => {
    try {
      await invoke("set_fixture_patch", {
        fixtureId: fixture.id,
        label: selectedFixtureLabelDraft(),
        universe: selectedFixtureUniverseDraft(),
        address: selectedFixtureAddressDraft(),
      });
      setMessage(`Updated patch for ${selectedFixtureLabelDraft().trim() || fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureGroups = async (fixture: PatchedFixtureSummary) => {
    const groupIds = parseGroupIds(selectedFixtureGroupText());
    try {
      await invoke("set_fixture_groups", {
        fixtureId: fixture.id,
        groupIds,
      });
      setSelectedFixtureGroupText(groupIds.join(", "));
      setMessage(`Updated groups for ${fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateSelectedFixtureLimit = <Key extends keyof FixtureLimits>(
    key: Key,
    value: FixtureLimits[Key],
  ) => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      [key]: typeof value === "number" ? clampDmxValue(value) : value,
    }));
  };

  const movementLimitPointFromPointer = (
    event: PointerEvent & { currentTarget: HTMLElement },
  ): MovementLimitPoint => {
    const bounds = event.currentTarget.getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    return {
      pan: clampDmxValue(x * 65_535),
      tilt: clampDmxValue((1 - y) * 65_535),
    };
  };

  const setMovementLimitRange = (anchor: MovementLimitPoint, point: MovementLimitPoint) => {
    setSelectedFixtureLimitsDraft((limits) => ({
      ...limits,
      pan_min: Math.min(anchor.pan, point.pan),
      pan_max: Math.max(anchor.pan, point.pan),
      tilt_min: Math.min(anchor.tilt, point.tilt),
      tilt_max: Math.max(anchor.tilt, point.tilt),
    }));
  };

  const startMovementLimitDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const anchor = movementLimitPointFromPointer(event);
    event.currentTarget.setPointerCapture(event.pointerId);
    setMovementLimitDrag({ anchor });
    setMovementLimitRange(anchor, anchor);
  };

  const dragMovementLimit = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const drag = movementLimitDrag();
    if (!drag || event.buttons !== 1) {
      return;
    }
    setMovementLimitRange(drag.anchor, movementLimitPointFromPointer(event));
  };

  const endMovementLimitDrag = (event: PointerEvent & { currentTarget: HTMLElement }) => {
    const drag = movementLimitDrag();
    if (drag) {
      setMovementLimitRange(drag.anchor, movementLimitPointFromPointer(event));
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setMovementLimitDrag(null);
  };

  const setFixtureLimits = async (fixture: PatchedFixtureSummary) => {
    const limits = normalizedSelectedFixtureLimitsDraft();
    try {
      await invoke("set_fixture_limits", {
        fixtureId: fixture.id,
        limits,
      });
      setSelectedFixtureLimitsDraft(limits);
      setMessage(`Updated limits for ${fixture.label}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const savePreset = async () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture first.");
      return;
    }
    try {
      const path = await invoke<string | null>("save_fixture_preset", { fixtureId: fixture.id });
      setMessage(path ? `Saved preset ${path}` : "Preset save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const newProject = async () => {
    try {
      await invoke("new_project");
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      setMessage("Created new untitled project.");
      const next = await refreshSnapshot();
      if (next) {
        markProjectClean(next);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveProject = async () => {
    try {
      const path = await invoke<string | null>("save_project");
      if (path) {
        setCurrentProjectPath(path);
        const next = await refreshSnapshot();
        if (next) {
          markProjectClean(next);
        }
      }
      setMessage(path ? `Saved project ${path}` : "Project save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveProjectAs = async () => {
    try {
      const path = await invoke<string | null>("save_project_as");
      if (path) {
        setCurrentProjectPath(path);
        const next = await refreshSnapshot();
        if (next) {
          markProjectClean(next);
        }
      }
      setMessage(path ? `Saved project ${path}` : "Project save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadedProjectMessage = (result: ProjectLoadResult) => {
    const profileLabel = result.profiles.length === 1 ? "1 embedded profile" : `${result.profiles.length} embedded profiles`;
    return `Loaded project ${result.path} (${profileLabel})`;
  };

  const applyLoadedProjectResult = async (result: ProjectLoadResult, currentPath: string | null) => {
    setCurrentProjectPath(currentPath);
    setMessage(loadedProjectMessage(result));
    const next = await refreshSnapshot();
    if (next) {
      markProjectClean(next);
    }
  };

  const loadProject = async () => {
    try {
      const result = await invoke<ProjectLoadResult | null>("load_project");
      if (!result) {
        setMessage("Project load canceled.");
        return;
      }
      await applyLoadedProjectResult(result, result.path);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadProjectPath = async (path: string) => {
    try {
      const result = await invoke<ProjectLoadResult>("load_project_path", { path });
      await applyLoadedProjectResult(result, result.path);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadStartupProject = async () => {
    try {
      const result = await invoke<ProjectLoadResult | null>("load_startup_project");
      if (result) {
        await applyLoadedProjectResult(result, result.path);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadQueuedOpenProjects = async () => {
    try {
      const paths = await invoke<string[]>("take_open_project_paths");
      const path = paths.at(-1);
      if (path) {
        await loadProjectPath(path);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadPhase1SampleProject = async () => {
    try {
      const result = await invoke<ProjectLoadResult>("load_phase1_sample_project");
      setCurrentProjectPath(null);
      setWorkspaceTab("setup");
      setSetupSubTab("patch");
      setMessage(loadedProjectMessage(result));
      const next = await refreshSnapshot();
      if (next) {
        markProjectClean(next);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const runPhase1Smoke = async () => {
    try {
      const report = await invoke<Phase1SmokeReport>("run_phase1_smoke");
      setCurrentProjectPath(null);
      setWorkspaceTab("control");
      setRawDmxUniverse(0);
      setDmxTestChannel(1);
      setDmxTestWidth(8);
      setDmxTestValue(255);
      const afterSmoke = await refreshSnapshot();
      if (afterSmoke) {
        markProjectClean(afterSmoke);
      }
      const values = report.first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
      const expected = report.expected_first_8.map((value) => value.toString(16).padStart(2, "0").toUpperCase()).join(" ");
      setMessage(
        report.passed
          ? `Smoke passed: ${report.path}, ${report.cue_label}, U0 A1-A8 ${values}.`
          : `Smoke failed: active cue ${report.active_cue_id ?? "none"}, got ${values}, expected ${expected}.`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadPreset = async () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture first.");
      return;
    }
    try {
      const path = await invoke<string | null>("load_fixture_preset", { fixtureId: fixture.id });
      if (path) {
        await refreshSnapshot();
        setMessage(`Loaded preset ${path}`);
      } else {
        setMessage("Preset load canceled.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureColor = async (hexColor: string) => {
    const fixture = selectedFixture();
    const controls = selectedColorControls();
    if (!fixture || !controls) {
      setMessage("Selected fixture has no RGB color controls.");
      return;
    }
    const red = Number.parseInt(hexColor.slice(1, 3), 16) * 257;
    const green = Number.parseInt(hexColor.slice(3, 5), 16) * 257;
    const blue = Number.parseInt(hexColor.slice(5, 7), 16) * 257;
    const updates = [
      { attribute: controls.red, value: red },
      { attribute: controls.green, value: green },
      { attribute: controls.blue, value: blue },
    ];
    const groupId = selectedFixtureGroupFilter();
    const fixtureIds = groupId
      ? snapshot()
          .fixtures
          .filter((candidate) => candidate.group_ids.includes(groupId))
          .map((candidate) => candidate.id)
      : [fixture.id];
    setFaderValues((current) => {
      const next = { ...current };
      for (const fixtureId of fixtureIds) {
        for (const update of updates) {
          next[`${fixtureId}:${update.attribute}`] = update.value;
        }
      }
      return next;
    });

    try {
      for (const update of updates) {
        if (groupId) {
          await invoke("set_group_attribute", {
            groupId,
            attribute: update.attribute,
            value: update.value,
          });
        } else {
          await invoke("set_attribute", {
            fixtureId: fixture.id,
            attribute: update.attribute,
            value: update.value,
          });
        }
      }
      setMessage(`Set ${groupId ? `group ${groupId}` : fixture.label} color ${hexColor.toUpperCase()}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setPanTiltFromPointer = async (event: PointerEvent) => {
    const fixture = selectedFixture();
    const controls = selectedPositionControls();
    if (!fixture || !controls) {
      return;
    }
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    const panValue = Math.round(x * 65535);
    const tiltValue = Math.round((1 - y) * 65535);
    const sourceValues = sourcePanTiltValues(fixture, panValue, tiltValue);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      await setGroupAttribute(groupId, controls.pan, sourceValues.pan);
      await setGroupAttribute(groupId, controls.tilt, sourceValues.tilt);
    } else {
      await setAttribute(fixture.id, controls.pan, sourceValues.pan);
      await setAttribute(fixture.id, controls.tilt, sourceValues.tilt);
    }
  };

  const setPanTiltValues = async (panValue: number, tiltValue: number) => {
    const fixture = selectedFixture();
    const controls = selectedPositionControls();
    if (!fixture || !controls) {
      setMessage("Selected fixture has no Pan/Tilt controls.");
      return;
    }
    const { pan: nextPan, tilt: nextTilt } = sourcePanTiltValues(fixture, panValue, tiltValue);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      await setGroupAttribute(groupId, controls.pan, nextPan);
      await setGroupAttribute(groupId, controls.tilt, nextTilt);
    } else {
      await setAttribute(fixture.id, controls.pan, nextPan);
      await setAttribute(fixture.id, controls.tilt, nextTilt);
    }
  };

  const setPanTiltPercent = (axis: "pan" | "tilt", percent: number) => {
    const controls = selectedPositionControls();
    if (!controls) {
      return;
    }
    const value = percentToDmxValue(percent);
    void setPanTiltValues(
      axis === "pan" ? value : controls.panValue,
      axis === "tilt" ? value : controls.tiltValue,
    );
  };

  const centerPanTilt = () => {
    const fixture = selectedFixture();
    if (!fixture) {
      void setPanTiltValues(32768, 32768);
      return;
    }
    const limits = fixture.limits ?? defaultFixtureLimits;
    const panRange = normalizeLimitRange(limits.pan_min, limits.pan_max);
    const tiltRange = normalizeLimitRange(limits.tilt_min, limits.tilt_max);
    void setPanTiltValues((panRange.min + panRange.max) / 2, (tiltRange.min + tiltRange.max) / 2);
  };

  const nudgePanTilt = (deltaPan: number, deltaTilt: number) => {
    const controls = selectedPositionControls();
    if (!controls) {
      return;
    }
    void setPanTiltValues(controls.panValue + deltaPan, controls.tiltValue + deltaTilt);
  };

  const addCurrentPositionFavorite = () => {
    const controls = selectedPositionControls();
    if (!controls) {
      setMessage("Selected fixture has no Pan/Tilt controls.");
      return;
    }
    const favorite = {
      id: `position-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 7)}`,
      label: `P${positionFavorites().length + 1}`,
      pan: clampDmxValue(controls.panValue),
      tilt: clampDmxValue(controls.tiltValue),
    };
    setPositionFavorites((current) => [
      favorite,
      ...current.filter((candidate) => candidate.pan !== favorite.pan || candidate.tilt !== favorite.tilt),
    ].slice(0, 24));
    setMessage(`Stored position ${favorite.label} (${formatShortDmxPercent(favorite.pan)} / ${formatShortDmxPercent(favorite.tilt)})`);
  };

  const removePositionFavorite = (favoriteId: string) => {
    setPositionFavorites((current) => current.filter((favorite) => favorite.id !== favoriteId));
  };

  const resetPositionFavorites = () => {
    setPositionFavorites(defaultPositionFavorites());
    setMessage("Reset position favorites.");
  };

  const setFixtureColorFromHsv = (hue: number, saturation: number, value: number) => {
    const { red, green, blue } = hsvToRgb(hue, saturation, value);
    void setFixtureColor(rgbToHex(red, green, blue));
  };

  const setColorHsvValue = (updates: Partial<{ hue: number; saturation: number; value: number }>) => {
    const hsv = selectedColorHsv();
    setFixtureColorFromHsv(
      updates.hue ?? hsv.hue,
      updates.saturation ?? hsv.saturation,
      updates.value ?? hsv.value,
    );
  };

  const setColorChannelValue = (channel: "red" | "green" | "blue", value: number) => {
    const current = selectedColorChannelValues();
    const next = {
      ...current,
      [channel]: clampDmxValue(value),
    };
    void setFixtureColor(`#${valueToHexByte(next.red)}${valueToHexByte(next.green)}${valueToHexByte(next.blue)}`);
  };

  const setColorFromPointer = (event: PointerEvent) => {
    const bounds = (event.currentTarget as HTMLElement).getBoundingClientRect();
    const x = clamp01((event.clientX - bounds.left) / bounds.width);
    const y = clamp01((event.clientY - bounds.top) / bounds.height);
    const current = selectedColorHsv();
    setFixtureColorFromHsv(x * 360, current.saturation > 0.05 ? current.saturation : 1, 1 - y);
  };

  const addCurrentColorFavorite = () => {
    const color = normalizeHexColor(selectedColorControls()?.value);
    if (!color) {
      return;
    }
    setColorFavorites((current) => [color, ...current.filter((candidate) => candidate !== color)].slice(0, 12));
  };

  const removeColorFavorite = (color: string) => {
    setColorFavorites((current) => current.filter((candidate) => candidate !== color));
  };

  const resetColorFavorites = () => {
    setColorFavorites(defaultColorFavorites());
    setMessage("Reset color favorites.");
  };

  const setDimmerValue = (value: number) => {
    const fixture = selectedFixture();
    const control = selectedDimmerControl();
    if (!fixture || !control) {
      setMessage("Selected fixture has no dimmer control.");
      return;
    }
    const nextValue = dimmerValueWithinLimits(fixture, value);
    const groupId = selectedFixtureGroupFilter();
    if (groupId) {
      void setGroupAttribute(groupId, control.attribute, nextValue);
    } else {
      void setAttribute(fixture.id, control.attribute, nextValue);
    }
  };

  const applyVisibleControlValues = async (mode: "zero" | "mid" | "full" | "default") => {
    const fixture = selectedFixture();
    const controls = visibleControls();
    if (!fixture || controls.length === 0) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    for (const control of controls) {
      const value =
        mode === "default"
          ? control.default_value
          : mode === "full"
            ? 65535
            : mode === "mid"
              ? 32768
              : 0;
      if (groupId) {
        await setGroupAttribute(groupId, control.attribute, value);
      } else {
        await setAttribute(fixture.id, control.attribute, value);
      }
    }
    setMessage(
      `Set ${activeControlCategory()} ${mode} for ${groupId ? `group ${groupId}` : fixture.label} (${controls.length} attributes)`,
    );
  };

  const applyCategoryQuickLook = async (look: CategoryQuickLook) => {
    const fixture = selectedFixture();
    const controls = visibleControls();
    const category = activeControlCategory();
    if (!fixture || controls.length === 0) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    for (const control of controls) {
      const value = quickLookValueForControl(category, look.id, control);
      if (groupId) {
        await setGroupAttribute(groupId, control.attribute, value);
      } else {
        await setAttribute(fixture.id, control.attribute, value);
      }
    }
    setMessage(
      `Applied ${category} look ${look.label} to ${groupId ? `group ${groupId}` : fixture.label} (${controls.length} attributes)`,
    );
  };

  const applyChannelFunction = async (
    control: AttributeControl,
    fn: NonNullable<AttributeControl["functions"]>[number],
  ) => {
    const fixture = selectedFixture();
    if (!fixture) {
      return;
    }
    const groupId = selectedFixtureGroupFilter();
    const value = channelFunctionValue(fn);
    if (groupId) {
      await setGroupAttribute(groupId, control.attribute, value);
    } else {
      await setAttribute(fixture.id, control.attribute, value);
    }
    setMessage(
      `Applied ${control.attribute} ${channelFunctionLabel(fn)} to ${groupId ? `group ${groupId}` : fixture.label}`,
    );
  };

  const setFixtureHighlight = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_highlight", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixtureSolo = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_solo", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setFixturePark = async (fixtureId: number, enabled: boolean) => {
    try {
      await invoke("set_fixture_park", { fixtureId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupHighlight = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_highlight", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupSolo = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_solo", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupPark = async (groupId: string, enabled: boolean) => {
    try {
      await invoke("set_group_park", { groupId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const resetEngineTelemetry = async () => {
    try {
      await invoke("reset_engine_telemetry");
      setMessage("Reset engine telemetry.");
      await refreshSnapshot();
      await refreshEngineTelemetryReport();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveEngineTelemetryReport = async () => {
    try {
      const path = await invoke<string | null>("save_engine_telemetry_report");
      setMessage(path ? `Saved telemetry report ${path}` : "Telemetry report save canceled.");
      await refreshEngineTelemetryReport();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const faderValue = (fixtureId: number, attribute: string, defaultValue: number) => {
    const localValue = faderValues()[`${fixtureId}:${attribute}`];
    if (localValue !== undefined) {
      return localValue;
    }
    const fixture = snapshot().fixtures.find((candidate) => candidate.id === fixtureId);
    return fixture?.attribute_values.find((value) => value.attribute === attribute)?.value ?? defaultValue;
  };

  const applyOutput = async () => {
    try {
      await invoke("set_output_config", { config: output() });
      setDmxOutputRoutes((current) => [output(), ...current.slice(1)]);
      if (isSerialDmxProtocol(output().protocol)) {
        setMessage(`${outputProtocolLabel(output().protocol)} target ${output().serial_port} @ ${output().serial_baud_rate}`);
      } else {
        setMessage(
          `${outputProtocolLabel(output().protocol)} target ${output().target_ip}:${output().port} universe ${output().universe}`,
        );
      }
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const sendDmxTestFrame = async () => {
    try {
      const result = await invoke<DmxTestFrameResult>("send_dmx_test_frame", {
        request: {
          config: output(),
          channel: dmxTestChannel(),
          width: dmxTestWidth(),
          value: dmxTestValue(),
        },
      });
      setMessage(
        `Sent ${outputProtocolLabel(result.protocol)} test U${result.universe} CH${result.channel} +${result.width} @ ${result.value} (${result.bytes} bytes)`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const sendDmxRoutesTestFrame = async () => {
    const routes = [output(), ...dmxOutputRoutes().slice(1)];
    try {
      const results = await invoke<DmxTestFrameResult[]>("send_dmx_routes_test_frame", {
        request: {
          configs: routes,
          channel: dmxTestChannel(),
          width: dmxTestWidth(),
          value: dmxTestValue(),
        },
      });
      const bytes = results.reduce((sum, result) => sum + result.bytes, 0);
      setMessage(`Sent test frame to ${results.length} DMX route(s) (${bytes} bytes).`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const dmxRouteLabel = (route: DmxOutputConfig) =>
    isSerialDmxProtocol(route.protocol)
      ? `${outputProtocolLabel(route.protocol)} ${route.serial_port || "(no port)"}`
      : `${outputProtocolLabel(route.protocol)} ${route.target_ip}:${route.port} U${route.universe}`;

  const applyDmxOutputRoutes = async (routes: DmxOutputConfig[]) => {
    try {
      await invoke("set_dmx_outputs", { configs: routes });
      setDmxOutputRoutes(routes);
      setOutput(routes[0] ?? defaultOutput);
      setMessage(`Applied ${routes.length} DMX output route(s).`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyCurrentDmxRoutes = async () => {
    const routes = [output(), ...dmxOutputRoutes().slice(1)];
    await applyDmxOutputRoutes(routes);
  };

  const addCurrentDmxRoute = async () => {
    const routes = [...dmxOutputRoutes(), output()];
    await applyDmxOutputRoutes(routes);
  };

  const removeDmxRoute = async (index: number) => {
    const routes = dmxOutputRoutes().filter((_, candidate) => candidate !== index);
    await applyDmxOutputRoutes(routes.length > 0 ? routes : [{ ...defaultOutput, enabled: false }]);
  };

  const setOutputProtocol = (protocol: DmxOutputConfig["protocol"]) => {
    const current = output();
    const port =
      protocol === "Sacn" && current.port === 6454
        ? 5568
        : protocol === "ArtNet" && current.port === 5568
          ? 6454
          : current.port;
    const universe = protocol === "Sacn" && current.universe === 0 ? 1 : current.universe;
    const target_ip =
      protocol === "Sacn" && (current.target_ip === defaultOutput.target_ip || current.target_ip.trim() === "")
        ? "multicast"
        : protocol === "ArtNet" && current.target_ip === "multicast"
          ? defaultOutput.target_ip
          : current.target_ip;
    const serial_port =
      isSerialDmxProtocol(protocol) && !current.serial_port && serialPorts().length > 0
        ? serialPorts()[0].name
        : current.serial_port;
    const serial_baud_rate =
      protocol === "EnttecOpenDmx"
        ? enttecOpenDmxBaudRate
        : protocol === "EnttecUsbPro" && current.serial_baud_rate === enttecOpenDmxBaudRate
          ? enttecUsbProBaudRate
          : current.serial_baud_rate;
    setOutput({ ...current, protocol, target_ip, port, universe, serial_port, serial_baud_rate });
  };

  const setBlackout = async (enabled: boolean) => {
    try {
      await invoke("set_blackout", { enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setLightingMaster = async (master: number) => {
    try {
      await invoke("set_lighting_master", { master });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setGroupSubmaster = async (groupId: string, level: number) => {
    try {
      await invoke("set_group_submaster", { groupId, level });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyBpm = async () => {
    const bpm = Number(bpmDraft());
    try {
      await invoke("set_bpm", { bpm });
      setMessage(`BPM set to ${bpm.toFixed(1)}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const tapBpm = async () => {
    try {
      await invoke("tap_bpm");
      await refreshSnapshot();
      setBpmDraft(snapshot().clock.bpm.toFixed(1));
      setMessage(`Tapped BPM ${snapshot().clock.bpm.toFixed(1)}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const analyzeAudioFile = async () => {
    try {
      const analysis = await invoke<AudioAnalysisSummary | null>("analyze_audio_file");
      if (!analysis) {
        setMessage("Audio analysis canceled.");
        return;
      }
      if (analysis.estimated_bpm) {
        setBpmDraft(analysis.estimated_bpm.toFixed(1));
      }
      await refreshSnapshot();
      setMessage(
        `Analyzed audio ${Math.round(analysis.duration_ms / 1000)}s${
          analysis.estimated_bpm ? ` / ${analysis.estimated_bpm.toFixed(1)} BPM` : ""
        }`,
      );
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyAudioBpm = async () => {
    const bpm = audioAnalysis()?.estimated_bpm;
    if (!bpm) {
      setMessage("Analyze audio with a BPM estimate first.");
      return;
    }
    try {
      await invoke("set_bpm", { bpm });
      await refreshSnapshot();
      setBpmDraft(bpm.toFixed(1));
      setMessage(`Applied audio BPM ${bpm.toFixed(1)}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const clearTimelineAudio = async () => {
    try {
      await invoke("clear_timeline_audio");
      await refreshSnapshot();
      setMessage("Cleared audio analysis.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const refreshMidiInputs = async () => {
    try {
      const inputs = await invoke<MidiInputSummary[]>("list_midi_inputs");
      setMidiInputs(inputs);
      if (selectedMidiInput() === null && inputs.length > 0) {
        setSelectedMidiInput(inputs[0].index);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const refreshMidiOutputs = async () => {
    try {
      const outputs = await invoke<MidiOutputSummary[]>("list_midi_outputs");
      setMidiOutputs(outputs);
      if (selectedMidiOutput() === null && outputs.length > 0) {
        setSelectedMidiOutput(outputs[0].index);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const refreshSerialPorts = async () => {
    try {
      const ports = await invoke<SerialPortSummary[]>("list_serial_ports");
      setSerialPorts(ports);
      if (!output().serial_port && ports.length > 0) {
        setOutput((current) => ({ ...current, serial_port: ports[0].name }));
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const connectMidiClock = async () => {
    const inputIndex = selectedMidiInput();
    if (inputIndex === null) {
      setMessage("No MIDI input selected.");
      return;
    }
    try {
      await invoke("connect_midi_clock", { inputIndex });
      setMidiConnected(true);
      setMessage("MIDI Clock connected.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const disconnectMidiClock = async () => {
    try {
      await invoke("disconnect_midi_clock");
      setMidiConnected(false);
      setMessage("MIDI Clock disconnected.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addMidiMapping = () => {
    const action = midiMapAction();
    const fixture = selectedFixture();
    const cueId = selectedMidiCueId();
    const layerId = selectedMidiLayerId();
    const outputId = selectedMidiVideoOutputId();
    const attribute = midiMapAttribute() || selectedEffectAttribute();
    if (action === "FixtureAttribute" && (!fixture || !attribute)) {
      setMessage("Select a fixture and MIDI attribute target first.");
      return;
    }
    if (action === "TriggerCue" && cueId === null) {
      setMessage("Create a cue before mapping MIDI to cues.");
      return;
    }
    if (action === "GroupSubmaster" && !midiMapGroupId().trim()) {
      setMessage("Enter a group ID before mapping MIDI to a submaster.");
      return;
    }
    if (
      (action === "VideoParam" ||
        action === "VideoCuePointAdd" ||
        action === "VideoCuePointRemove" ||
        action === "VideoCuePointJump" ||
        action === "VideoLayerEnabled" ||
        action === "VideoLayerSolo" ||
        action === "VideoPlay" ||
        action === "VideoLoop") &&
      layerId === null
    ) {
      setMessage("Add a video layer before mapping MIDI to video.");
      return;
    }
    if (
      (action === "VideoOutputEnabled" ||
        action === "VideoOutputOpacity" ||
        action === "VideoOutputFade" ||
        action === "VideoOutputBlackout") &&
      outputId === null
    ) {
      setMessage("Add a video output before mapping MIDI to video output.");
      return;
    }

    const mapping: MidiControlMapping = {
      channel: midiMapChannel() >= 0 ? midiMapChannel() : null,
      message: midiMapMessage(),
      number: Math.max(0, Math.min(127, Math.round(midiMapNumber()))),
      action,
      fixture_id: action === "FixtureAttribute" ? fixture?.id ?? null : null,
      attribute: action === "FixtureAttribute" ? attribute : null,
      group_id: action === "GroupSubmaster" ? midiMapGroupId().trim() : null,
      cue_id: action === "TriggerCue" ? cueId : null,
      layer_id:
        action === "VideoParam" ||
        action === "VideoCuePointAdd" ||
        action === "VideoCuePointRemove" ||
        action === "VideoCuePointJump" ||
        action === "VideoLayerEnabled" ||
        action === "VideoLayerSolo" ||
        action === "VideoPlay" ||
        action === "VideoLoop"
          ? layerId
          : null,
      output_id:
        action === "VideoOutputEnabled" ||
        action === "VideoOutputOpacity" ||
        action === "VideoOutputFade" ||
        action === "VideoOutputBlackout"
          ? outputId
          : null,
      video_param: action === "VideoParam" ? midiMapVideoParam() : null,
      cue_point_index: action === "VideoCuePointJump" ? Math.max(0, Math.round(midiMapCuePointIndex())) : null,
      duration_ms:
        action === "VideoOutputFade" || action === "VideoCuePointAdd" || action === "VideoCuePointRemove"
          ? Math.max(0, Math.round(midiMapDurationMs()))
          : null,
      low: midiMapLow(),
      high: midiMapHigh(),
    };
    setMidiMappings((current) => [...current, mapping]);
    setMessage(`Added MIDI mapping ${mapping.message} ${mapping.number}`);
  };

  const removeMidiMapping = (index: number) => {
    setMidiMappings((current) => current.filter((_, candidate) => candidate !== index));
  };

  const midiMappingTargetLabel = (mapping: MidiControlMapping) => {
    switch (mapping.action) {
      case "FixtureAttribute":
        return `Fixture ${mapping.fixture_id} ${mapping.attribute}`;
      case "TriggerCue":
        return `Cue ${mapping.cue_id}`;
      case "TriggerNextCue":
        return "Cue next";
      case "TriggerPreviousCue":
        return "Cue previous";
      case "VideoParam":
        return `Layer ${mapping.layer_id} ${mapping.video_param}`;
      case "VideoCuePointAdd":
        return `Layer ${mapping.layer_id} add cue ${
          mapping.duration_ms === null || mapping.duration_ms === undefined ? "current" : `${mapping.duration_ms}ms`
        }`;
      case "VideoCuePointRemove":
        return `Layer ${mapping.layer_id} remove cue ${mapping.duration_ms ?? 0}ms`;
      case "VideoCuePointJump":
        return `Layer ${mapping.layer_id} cue ${mapping.cue_point_index ?? 0}`;
      case "VideoLayerEnabled":
        return `Layer ${mapping.layer_id} enabled`;
      case "VideoLayerSolo":
        return `Layer ${mapping.layer_id} solo`;
      case "VideoPlay":
        return `Layer ${mapping.layer_id} play`;
      case "VideoLoop":
        return `Layer ${mapping.layer_id} loop ${mapping.low}-${mapping.high}ms`;
      case "VideoOutputEnabled":
        return `Output ${mapping.output_id} enabled`;
      case "VideoOutputOpacity":
        return `Output ${mapping.output_id} opacity`;
      case "VideoOutputFade":
        return `Output ${mapping.output_id} fade ${mapping.duration_ms ?? 1000}ms`;
      case "VideoOutputBlackout":
        return `Output ${mapping.output_id} blackout`;
      case "TimelinePlay":
        return "Timeline play";
      case "TimelineSeek":
        return "Timeline seek";
      case "LightingMaster":
        return "Lighting master";
      case "GroupSubmaster":
        return `Group ${mapping.group_id} submaster`;
      case "CueFadePause":
        return "Cue fade pause";
      case "Blackout":
        return "Lighting blackout";
      case "VideoBlackout":
        return "Video blackout";
    }
  };

  const applyLearnedMidiControl = (learned: LearnedMidiControl) => {
    setMidiMapChannel(learned.channel);
    setMidiMapMessage(learned.message);
    setMidiMapNumber(learned.number);
    setMessage(`Learned ${learned.message} ch ${learned.channel} #${learned.number}`);
  };

  const learnMidiControl = async () => {
    const inputIndex = selectedMidiInput();
    if (inputIndex === null) {
      setMessage("No MIDI input selected.");
      return;
    }
    try {
      setMessage("Waiting for MIDI input...");
      const learned = await invoke<LearnedMidiControl | null>("learn_midi_control", { inputIndex });
      if (learned) {
        applyLearnedMidiControl(learned);
      } else {
        setMessage("MIDI learn timed out.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveMidiMappings = async () => {
    try {
      const path = await invoke<string | null>("save_midi_mappings", { mappings: midiMappings() });
      if (path) {
        setMessage(`Saved MIDI mappings to ${path}`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadMidiMappings = async () => {
    try {
      const loaded = await invoke<MidiControlMapping[] | null>("load_midi_mappings");
      if (loaded) {
        setMidiMappings(loaded);
        setMessage(`Loaded ${loaded.length} MIDI mapping(s).`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const connectMidiControl = async () => {
    const inputIndex = selectedMidiInput();
    if (inputIndex === null) {
      setMessage("No MIDI input selected.");
      return;
    }
    if (midiMappings().length === 0) {
      setMessage("Add at least one MIDI mapping first.");
      return;
    }
    try {
      await invoke("connect_midi_control", { inputIndex, mappings: midiMappings() });
      setMidiControlConnected(true);
      setMessage(`MIDI control connected with ${midiMappings().length} mapping(s).`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const disconnectMidiControl = async () => {
    try {
      await invoke("disconnect_midi_control");
      setMidiControlConnected(false);
      setMessage("MIDI control disconnected.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const connectMidiFeedback = async () => {
    const outputIndex = selectedMidiOutput();
    if (outputIndex === null) {
      setMessage("No MIDI output selected.");
      return;
    }
    try {
      await invoke("connect_midi_feedback", { outputIndex });
      setMidiFeedbackConnected(true);
      setMessage("MIDI feedback connected.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const disconnectMidiFeedback = async () => {
    try {
      await invoke("disconnect_midi_feedback");
      setMidiFeedbackConnected(false);
      setMidiFeedbackEnabled(false);
      setMessage("MIDI feedback disconnected.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const sendMidiFeedback = async (report = true) => {
    if (!midiFeedbackConnected() || midiMappings().length === 0) {
      if (report) {
        setMessage("Connect MIDI feedback and add mappings first.");
      }
      return;
    }
    try {
      const sent = await invoke<number>("send_midi_feedback", { mappings: midiMappings() });
      if (report) {
        setMessage(`Sent ${sent} MIDI feedback message(s).`);
      }
    } catch (error) {
      setMidiFeedbackEnabled(false);
      setMessage(String(error));
    }
  };

  const midiFeedbackTimer = window.setInterval(() => {
    if (midiFeedbackEnabled() && midiFeedbackConnected() && midiMappings().length > 0) {
      void sendMidiFeedback(false);
    }
  }, 500);
  onCleanup(() => window.clearInterval(midiFeedbackTimer));

  const addOscMapping = () => {
    const action = oscMapAction();
    const fixture = selectedFixture();
    const cueId = selectedOscCueId();
    const layerId = selectedOscLayerId();
    const outputId = selectedOscVideoOutputId();
    const attribute = oscMapAttribute() || selectedEffectAttribute();
    if (!oscMapAddress().trim()) {
      setMessage("Enter an OSC address.");
      return;
    }
    if (action === "FixtureAttribute" && (!fixture || !attribute)) {
      setMessage("Select a fixture and OSC attribute target first.");
      return;
    }
    if (action === "TriggerCue" && cueId === null) {
      setMessage("Create a cue before mapping OSC to cues.");
      return;
    }
    if (action === "GroupSubmaster" && !oscMapGroupId().trim()) {
      setMessage("Enter a group ID before mapping OSC to a submaster.");
      return;
    }
    if (
      (action === "VideoParam" ||
        action === "VideoCuePointAdd" ||
        action === "VideoCuePointRemove" ||
        action === "VideoCuePointJump" ||
        action === "VideoLayerEnabled" ||
        action === "VideoLayerSolo" ||
        action === "VideoPlay" ||
        action === "VideoLoop") &&
      layerId === null
    ) {
      setMessage("Add a video layer before mapping OSC to video.");
      return;
    }
    if (
      (action === "VideoOutputEnabled" ||
        action === "VideoOutputOpacity" ||
        action === "VideoOutputFade" ||
        action === "VideoOutputBlackout") &&
      outputId === null
    ) {
      setMessage("Add a video output before mapping OSC to video output.");
      return;
    }
    const mapping: OscControlMapping = {
      address: oscMapAddress().startsWith("/") ? oscMapAddress() : `/${oscMapAddress()}`,
      action,
      fixture_id: action === "FixtureAttribute" ? fixture?.id ?? null : null,
      attribute: action === "FixtureAttribute" ? attribute : null,
      group_id: action === "GroupSubmaster" ? oscMapGroupId().trim() : null,
      cue_id: action === "TriggerCue" ? cueId : null,
      layer_id:
        action === "VideoParam" ||
        action === "VideoCuePointAdd" ||
        action === "VideoCuePointRemove" ||
        action === "VideoCuePointJump" ||
        action === "VideoLayerEnabled" ||
        action === "VideoLayerSolo" ||
        action === "VideoPlay" ||
        action === "VideoLoop"
          ? layerId
          : null,
      output_id:
        action === "VideoOutputEnabled" ||
        action === "VideoOutputOpacity" ||
        action === "VideoOutputFade" ||
        action === "VideoOutputBlackout"
          ? outputId
          : null,
      video_param: action === "VideoParam" ? oscMapVideoParam() : null,
      cue_point_index: action === "VideoCuePointJump" ? Math.max(0, Math.round(oscMapCuePointIndex())) : null,
      duration_ms:
        action === "VideoOutputFade" || action === "VideoCuePointAdd" || action === "VideoCuePointRemove"
          ? Math.max(0, Math.round(oscMapDurationMs()))
          : null,
      low: oscMapLow(),
      high: oscMapHigh(),
    };
    setOscMappings((current) => [...current, mapping]);
    setMessage(`Added OSC mapping ${mapping.address}`);
  };

  const removeOscMapping = (index: number) => {
    setOscMappings((current) => current.filter((_, candidate) => candidate !== index));
  };

  const oscMappingTargetLabel = (mapping: OscControlMapping) => {
    switch (mapping.action) {
      case "FixtureAttribute":
        return `Fixture ${mapping.fixture_id} ${mapping.attribute}`;
      case "TriggerCue":
        return `Cue ${mapping.cue_id}`;
      case "TriggerNextCue":
        return "Cue next";
      case "TriggerPreviousCue":
        return "Cue previous";
      case "VideoParam":
        return `Layer ${mapping.layer_id} ${mapping.video_param}`;
      case "VideoCuePointAdd":
        return `Layer ${mapping.layer_id} add cue ${
          mapping.duration_ms === null || mapping.duration_ms === undefined ? "current" : `${mapping.duration_ms}ms`
        }`;
      case "VideoCuePointRemove":
        return `Layer ${mapping.layer_id} remove cue ${mapping.duration_ms ?? 0}ms`;
      case "VideoCuePointJump":
        return `Layer ${mapping.layer_id} cue ${mapping.cue_point_index ?? 0}`;
      case "VideoLayerEnabled":
        return `Layer ${mapping.layer_id} enabled`;
      case "VideoLayerSolo":
        return `Layer ${mapping.layer_id} solo`;
      case "VideoPlay":
        return `Layer ${mapping.layer_id} play`;
      case "VideoLoop":
        return `Layer ${mapping.layer_id} loop ${mapping.low}-${mapping.high}ms`;
      case "VideoOutputEnabled":
        return `Output ${mapping.output_id} enabled`;
      case "VideoOutputOpacity":
        return `Output ${mapping.output_id} opacity`;
      case "VideoOutputFade":
        return `Output ${mapping.output_id} fade ${mapping.duration_ms ?? 1000}ms`;
      case "VideoOutputBlackout":
        return `Output ${mapping.output_id} blackout`;
      case "TimelinePlay":
        return "Timeline play";
      case "TimelineSeek":
        return "Timeline seek";
      case "LightingMaster":
        return "Lighting master";
      case "GroupSubmaster":
        return `Group ${mapping.group_id} submaster`;
      case "CueFadePause":
        return "Cue fade pause";
      case "Blackout":
        return "Lighting blackout";
      case "VideoBlackout":
        return "Video blackout";
    }
  };

  const saveOscMappings = async () => {
    try {
      const path = await invoke<string | null>("save_osc_mappings", { mappings: oscMappings() });
      if (path) {
        setMessage(`Saved OSC mappings to ${path}`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadOscMappings = async () => {
    try {
      const loaded = await invoke<OscControlMapping[] | null>("load_osc_mappings");
      if (loaded) {
        setOscMappings(loaded);
        setMessage(`Loaded ${loaded.length} OSC mapping(s).`);
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyLearnedOscControl = (learned: LearnedOscControl) => {
    setOscMapAddress(learned.address);
    const valueLabel = learned.value === null || learned.value === undefined ? "no numeric value" : `value ${learned.value}`;
    setMessage(`Learned OSC ${learned.address} (${learned.argument_count} arg(s), ${valueLabel}).`);
  };

  const learnOscControl = async () => {
    if (oscRunning()) {
      setMessage("Stop OSC input before OSC learn.");
      return;
    }
    const config: OscInputConfig = {
      bind_ip: oscBindIp(),
      port: oscPort(),
    };
    try {
      setMessage("Waiting for OSC input...");
      const learned = await invoke<LearnedOscControl | null>("learn_osc_control", { config });
      if (learned) {
        applyLearnedOscControl(learned);
      } else {
        setMessage("OSC learn timed out.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startOscInput = async () => {
    const config: OscInputConfig = {
      bind_ip: oscBindIp(),
      port: oscPort(),
    };
    try {
      await invoke("start_osc_input", { config, mappings: oscMappings() });
      setOscRunning(true);
      setMessage(`OSC input listening on ${config.bind_ip}:${config.port} with ${oscMappings().length} mapping(s)`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const stopOscInput = async () => {
    try {
      await invoke("stop_osc_input");
      setOscRunning(false);
      setMessage("OSC input stopped.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const startRemoteControl = async () => {
    const config: RemoteControlConfig = {
      bind_ip: remoteBindIp(),
      port: remotePort(),
    };
    try {
      await invoke("start_remote_control", { config });
      setRemoteRunning(true);
      setMessage(`Remote WebSocket listening on ${config.bind_ip}:${config.port}`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const stopRemoteControl = async () => {
    try {
      await invoke("stop_remote_control");
      setRemoteRunning(false);
      setMessage("Remote WebSocket stopped.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const createCue = async () => {
    const scopeError = cueCaptureScopeError();
    if (scopeError) {
      setMessage(scopeError);
      return;
    }
    const captureScope = cueCaptureScopeRequest();
    if (!captureScope) {
      setMessage("Select a valid cue capture scope.");
      return;
    }
    try {
      const cueId = await invoke<number>("create_cue_from_current", {
        label: cueLabel(),
        fadeMs: cueFadeMs(),
        captureScope,
      });
      setCueLabel(`Cue ${snapshot().cues.length + 2}`);
      setMessage(`Created cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerCue = async (cueId: number) => {
    try {
      await invoke("trigger_cue", { cueId });
      setMessage(`Triggered cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const updateCue = async (cueId: number, label: string, fadeMs: number) => {
    const normalizedFadeMs = Math.max(0, Math.round(Number.isFinite(fadeMs) ? fadeMs : cueFadeMs()));
    const scopeError = cueCaptureScopeError();
    if (scopeError) {
      setMessage(scopeError);
      return;
    }
    const captureScope = cueCaptureScopeRequest();
    if (!captureScope) {
      setMessage("Select a valid cue capture scope.");
      return;
    }
    try {
      await invoke("update_cue_from_current", { cueId, label, fadeMs: normalizedFadeMs, captureScope });
      setMessage(`Updated cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueMetadata = async (cue: CueSummary) => {
    const draft = cueMetadataDraft(cue);
    const fadeMs = Math.max(0, Math.round(Number.isFinite(draft.fade_ms) ? draft.fade_ms : cue.fade_ms));
    try {
      await invoke("set_cue_metadata", {
        cueId: cue.id,
        label: draft.label,
        fadeMs,
      });
      setCueMetadataDrafts((current) => ({
        ...current,
        [cue.id]: {
          label: draft.label,
          fade_ms: fadeMs,
        },
      }));
      setMessage(`Saved cue ${cue.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveCue = async (cueId: number, delta: -1 | 1) => {
    try {
      await invoke("move_cue", { cueId, delta });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateCue = async (cue: CueSummary) => {
    const draft = cueMetadataDraft(cue);
    const baseLabel = draft.label.trim() || cue.label;
    try {
      const cueId = await invoke<number>("duplicate_cue", {
        sourceCueId: cue.id,
        label: `${baseLabel} Copy`,
      });
      setMessage(`Duplicated cue ${cue.id} as ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerNextCue = async () => {
    try {
      await invoke("trigger_next_cue");
      setMessage("GO");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const triggerPreviousCue = async () => {
    try {
      await invoke("trigger_previous_cue");
      setMessage("Back");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setCueFadePaused = async (paused: boolean) => {
    try {
      await invoke("set_cue_fade_paused", { paused });
      setMessage(paused ? "Cue fade paused." : "Cue fade resumed.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeCue = async (cueId: number) => {
    try {
      await invoke("remove_cue", { cueId });
      setMessage(`Removed cue ${cueId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const snapTimelineDrafts = () => {
    setTimelineEventTimeMs(snapTimeMs(timelineEventTimeMs()));
    setAutomationStartMs(snapTimeMs(automationStartMs()));
    setAutomationEndMs(snapTimeMs(automationEndMs()));
    setVideoAutomationStartMs(snapTimeMs(videoAutomationStartMs()));
    setVideoAutomationEndMs(snapTimeMs(videoAutomationEndMs()));
    setMessage(`Snapped timeline inputs to ${timelineSnapMode().toLowerCase()}.`);
  };

  const addTimelineCueEventAt = async (cueId: number | null, timeMs: number, track: TimelineTrackKind, nextDraftTime = true) => {
    if (cueId === null) {
      setMessage("Create a cue before adding timeline events.");
      return;
    }
    const snappedTimeMs = snapTimeMs(timeMs);
    try {
      const eventId = await invoke<number>("add_timeline_cue_event", {
        cueId,
        timeMs: snappedTimeMs,
        track,
      });
      if (nextDraftTime) {
        setTimelineEventTimeMs(snapTimeMs(snappedTimeMs + 1000));
      } else {
        setTimelineEventTimeMs(snappedTimeMs);
      }
      setMessage(`Added timeline event ${eventId} at ${snappedTimeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineCueEvent = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), timelineEventTimeMs(), timelineTrack());
  };

  const addTimelineCueEventAtPlayhead = async () => {
    await addTimelineCueEventAt(selectedTimelineCueId(), snapshot().timeline.position_ms, timelineTrack(), false);
  };

  const setTimelineCueEvent = async (event: TimelineCueEventSummary) => {
    const draft = timelineEventDraft(event);
    const cueId = Math.max(0, Math.round(draft.cue_id));
    const timeMs = snapTimeMs(draft.time_ms);
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId,
        timeMs,
        track: draft.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: cueId,
          time_ms: timeMs,
          track: draft.track,
        },
      }));
      setMessage(`Saved timeline event ${event.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveTimelineCueEvent = async (event: TimelineCueEventSummary, deltaMs: number) => {
    const timeMs = snapTimeMs(Math.max(0, event.time_ms + deltaMs));
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: event.cue_id,
        timeMs,
        track: event.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: event.cue_id,
          time_ms: timeMs,
          track: event.track,
        },
      }));
      setMessage(`Moved timeline event ${event.id} to ${timeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveTimelineCueEventToRatio = async (eventId: number, ratio: number) => {
    const event = timelineEventRows().find((candidate) => candidate.id === eventId);
    if (!event) {
      setMessage(`Timeline event ${eventId} was not found.`);
      return;
    }
    const timeMs = snapTimeMs(Math.round(clampRange(ratio, 0, 1) * timelineOverviewDurationMs()));
    try {
      await invoke("set_timeline_cue_event", {
        eventId: event.id,
        cueId: event.cue_id,
        timeMs,
        track: event.track,
      });
      setTimelineEventDrafts((current) => ({
        ...current,
        [event.id]: {
          cue_id: event.cue_id,
          time_ms: timeMs,
          track: event.track,
        },
      }));
      setMessage(`Moved ${event.cue_label} to ${timeMs} ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeTimelineEvent = async (eventId: number) => {
    try {
      await invoke("remove_timeline_event", { eventId });
      setMessage(`Removed timeline event ${eventId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineAutomation = async () => {
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    if (!fixture || !attribute) {
      setMessage("Select a fixture and attribute first.");
      return;
    }
    const startMs = snapTimeMs(automationStartMs());
    const endMs = Math.max(startMs, snapTimeMs(automationEndMs()));
    try {
      const automationId = await invoke<number>("add_timeline_automation", {
        fixtureId: fixture.id,
        attribute,
        keyframes: [
          {
            time_ms: startMs,
            value: automationStartValue(),
            interpolation: automationInterpolation(),
          },
          {
            time_ms: endMs,
            value: automationEndValue(),
            interpolation: "Step",
          },
        ],
      });
      setMessage(`Added automation ${automationId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setTimelineAutomation = async (automation: TimelineAutomationSummary) => {
    const draft = timelineAutomationDraft(automation);
    if (!draft.attribute.trim()) {
      setMessage("Select an automation attribute.");
      return;
    }
    const startMs = snapTimeMs(draft.start_ms);
    const endMs = Math.max(startMs, snapTimeMs(draft.end_ms));
    const startValue = Math.max(0, Math.min(65535, Math.round(draft.start_value)));
    const endValue = Math.max(0, Math.min(65535, Math.round(draft.end_value)));
    try {
      await invoke("set_timeline_automation", {
        automationId: automation.id,
        fixtureId: Math.max(0, Math.round(draft.fixture_id)),
        attribute: draft.attribute,
        keyframes: [
          {
            time_ms: startMs,
            value: startValue,
            interpolation: draft.interpolation,
          },
          {
            time_ms: endMs,
            value: endValue,
            interpolation: "Step",
          },
        ],
      });
      setTimelineAutomationDrafts((current) => ({
        ...current,
        [automation.id]: {
          ...draft,
          start_ms: startMs,
          end_ms: endMs,
          start_value: startValue,
          end_value: endValue,
        },
      }));
      setMessage(`Saved automation ${automation.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const addTimelineVideoAutomation = async () => {
    const layerId = selectedVideoAutomationLayerId();
    if (layerId === null) {
      setMessage("Add a video layer before adding video automation.");
      return;
    }
    const startMs = snapTimeMs(videoAutomationStartMs());
    const endMs = Math.max(startMs, snapTimeMs(videoAutomationEndMs()));
    try {
      const automationId = await invoke<number>("add_timeline_video_automation", {
        layerId,
        param: videoAutomationParam(),
        keyframes: [
          {
            time_ms: startMs,
            value: videoAutomationStartValue(),
            interpolation: videoAutomationInterpolation(),
          },
          {
            time_ms: endMs,
            value: videoAutomationEndValue(),
            interpolation: "Step",
          },
        ],
      });
      setMessage(`Added video automation ${automationId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setTimelineVideoAutomation = async (automation: TimelineVideoAutomationSummary) => {
    const draft = timelineVideoAutomationDraft(automation);
    if (!Number.isFinite(draft.start_value) || !Number.isFinite(draft.end_value)) {
      setMessage("Video automation values must be finite.");
      return;
    }
    const startMs = snapTimeMs(draft.start_ms);
    const endMs = Math.max(startMs, snapTimeMs(draft.end_ms));
    try {
      await invoke("set_timeline_video_automation", {
        automationId: automation.id,
        layerId: Math.max(0, Math.round(draft.layer_id)),
        param: draft.param,
        keyframes: [
          {
            time_ms: startMs,
            value: draft.start_value,
            interpolation: draft.interpolation,
          },
          {
            time_ms: endMs,
            value: draft.end_value,
            interpolation: "Step",
          },
        ],
      });
      setTimelineVideoAutomationDrafts((current) => ({
        ...current,
        [automation.id]: {
          ...draft,
          start_ms: startMs,
          end_ms: endMs,
        },
      }));
      setMessage(`Saved video automation ${automation.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeTimelineAutomation = async (automationId: number) => {
    try {
      await invoke("remove_timeline_automation", { automationId });
      setMessage(`Removed automation ${automationId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const playTimeline = async () => {
    try {
      await invoke("set_timeline_playing", { playing: true });
      setMessage("Timeline playing.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const pauseTimeline = async () => {
    try {
      await invoke("set_timeline_playing", { playing: false });
      setMessage("Timeline paused.");
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const seekTimeline = async (positionMs: number) => {
    try {
      await invoke("seek_timeline", { positionMs });
      setMessage(`Timeline seek ${positionMs}ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const seekTimelineFromOverviewRatio = (ratio: number) => {
    void seekTimeline(snapTimeMs(Math.round(ratio * timelineOverviewDurationMs())));
  };

  const addVideoLayer = async () => {
    try {
      const sourceKind = videoSourceKind();
      const layerId =
        sourceKind === "File" || sourceKind === "StillImage"
          ? await invoke<number>(sourceKind === "StillImage" ? "add_still_image_layer" : "add_video_file_layer", {
              label: videoLabel(),
              path: videoPath(),
            })
          : await invoke<number>("add_video_input_layer", {
              label: videoLabel(),
              kind: sourceKind,
              name: videoPath(),
            });
      setVideoLabel(`Layer ${snapshot().video.layers.length + 2}`);
      setMessage(`Added video layer ${layerId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoLayer = async (layerId: number) => {
    try {
      await invoke("remove_video_layer", { layerId });
      setMessage(`Removed video layer ${layerId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const duplicateVideoLayer = async (layer: { id: number; label: string }) => {
    try {
      const layerId = await invoke<number>("duplicate_video_layer", {
        sourceLayerId: layer.id,
        label: `${layer.label} Copy`,
      });
      setMessage(`Duplicated video layer ${layerId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveVideoLayer = async (layerId: number, delta: -1 | 1) => {
    const layerIds = snapshot().video.layers.map((layer) => layer.id);
    const index = layerIds.indexOf(layerId);
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || nextIndex >= layerIds.length) {
      return;
    }
    const nextLayerIds = [...layerIds];
    [nextLayerIds[index], nextLayerIds[nextIndex]] = [nextLayerIds[nextIndex], nextLayerIds[index]];
    try {
      await invoke("set_video_layer_order", { layerIds: nextLayerIds });
      setMessage(`Moved video layer ${layerId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoLayerLabel = async (layerId: number, label: string) => {
    try {
      await invoke("set_video_layer_label", { layerId, label });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const renderDebugVideoPreview = async () => {
    try {
      const frame = await invoke<VideoFrame>("get_debug_video_preview", { width: 64, height: 36 });
      setVideoPreviewUrl(videoFrameToDataUrl(frame));
      setVideoPreviewInfo(`${frame.width}x${frame.height} ${frame.format} / pts ${frame.pts_ms}ms / ${frame.data.length} bytes`);
      setMessage("Rendered CPU video preview.");
      await refreshVideoPreviewDiagnostics(true);
    } catch (error) {
      setVideoPreviewUrl("");
      setMessage(String(error));
    }
  };

  const refreshVideoPreviewDiagnostics = async (silent = false) => {
    try {
      const diagnostics = await invoke<VideoPreviewDiagnostics>("get_video_preview_diagnostics");
      setVideoPreviewDiagnostics(diagnostics);
      if (!silent) {
        setMessage("Updated video preview diagnostics.");
      }
    } catch (error) {
      setMessage(String(error));
    }
  };

  const renderDebugVideoOutputPreview = async (outputId: number, testPattern = false) => {
    try {
      const frame = await invoke<VideoFrame>(testPattern ? "get_debug_video_output_test_pattern" : "get_debug_video_output_preview", {
        outputId,
        width: 128,
        height: 72,
      });
      const previewUrl = videoFrameToDataUrl(frame);
      const label = testPattern ? "Pattern" : "Output";
      const info = `${frame.width}x${frame.height} ${frame.format} / ${frame.data.length} bytes`;
      setVideoOutputPreviewUrl(previewUrl);
      setVideoOutputPreviewInfo(info);
      setVideoOutputPreviewId(outputId);
      setVideoOutputPreviewMode(testPattern ? "test" : "output");
      setVideoPreviewUrl(previewUrl);
      setVideoPreviewInfo(`${label} ${outputId}: ${info}`);
      setMessage(`Rendered ${testPattern ? "test pattern" : "output"} ${outputId} preview.`);
      if (!testPattern) {
        await refreshVideoPreviewDiagnostics(true);
      }
    } catch (error) {
      setVideoOutputPreviewUrl("");
      setVideoOutputPreviewId(outputId);
      setVideoPreviewUrl("");
      setMessage(String(error));
    }
  };

  const setVideoLayerState = async (layerId: number, stateValue: VideoLayerState) => {
    try {
      await invoke("set_video_layer_state", { layerId, stateValue });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoLayerTransform = (
    layerId: number,
    stateValue: VideoLayerState,
    transformPatch: Partial<VideoLayerState["transform"]>,
  ) =>
    setVideoLayerState(layerId, {
      ...stateValue,
      transform: {
        ...defaultTransform,
        ...stateValue.transform,
        ...transformPatch,
      },
    });

  const setVideoLayerColor = (
    layerId: number,
    stateValue: VideoLayerState,
    colorPatch: Partial<VideoLayerState["color"]>,
  ) =>
    setVideoLayerState(layerId, {
      ...stateValue,
      color: {
        ...defaultColorAdjust,
        ...stateValue.color,
        ...colorPatch,
      },
    });

  const setVideoLayerFx = (
    layerId: number,
    stateValue: VideoLayerState,
    fxPatch: Partial<VideoLayerState["fx"]>,
  ) =>
    setVideoLayerState(layerId, {
      ...stateValue,
      fx: {
        ...defaultFxAdjust,
        ...stateValue.fx,
        ...fxPatch,
      },
    });

  const addVideoCuePoint = async (layerId: number, positionMs?: number) => {
    try {
      await invoke("add_video_cue_point", { layerId, positionMs });
      setMessage(`Added video cue point${positionMs === undefined ? "" : ` at ${positionMs}ms`}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoCuePoint = async (layerId: number, positionMs: number) => {
    try {
      await invoke("remove_video_cue_point", { layerId, positionMs });
      setMessage(`Removed video cue point ${positionMs}ms`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const jumpVideoCuePoint = async (layerId: number, cuePointIndex: number) => {
    try {
      await invoke("jump_video_cue_point", { layerId, cuePointIndex });
      setMessage(`Jumped video layer ${layerId} to cue point ${cuePointIndex + 1}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoLayerBlendMode = async (layerId: number, blendMode: VideoBlendMode) => {
    try {
      await invoke("set_video_layer_blend_mode", { layerId, blendMode });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoMasterOpacity = async (opacity: number) => {
    try {
      await invoke("set_video_master_opacity", { opacity });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoBlackout = async (enabled: boolean) => {
    try {
      await invoke("set_video_blackout", { enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const toggleVideoCompositionLayer = (layerId: number, checked: boolean) => {
    setVideoCompositionLayerIds((current) => {
      if (checked) {
        return current.includes(layerId) ? current : [...current, layerId];
      }
      return current.filter((candidate) => candidate !== layerId);
    });
  };

  const addVideoComposition = async () => {
    try {
      const compositionId = await invoke<number>("add_video_composition", {
        label: videoCompositionLabel(),
        layerIds: videoCompositionLayerIds(),
      });
      setVideoCompositionLabel(`Composition ${snapshot().video.compositions.length + 1}`);
      setVideoCompositionLayerIds([]);
      setMessage(`Added video composition ${compositionId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoComposition = async (compositionId: number) => {
    try {
      await invoke("remove_video_composition", { compositionId });
      setMessage(`Removed video composition ${compositionId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoCompositionLayers = async (compositionId: number, layerIds: number[]) => {
    try {
      await invoke("set_video_composition_layers", { compositionId, layerIds });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveVideoCompositionLayer = (
    compositionId: number,
    layerIds: number[],
    layerId: number,
    delta: -1 | 1,
  ) => {
    const index = layerIds.indexOf(layerId);
    const nextIndex = index + delta;
    if (index < 0 || nextIndex < 0 || nextIndex >= layerIds.length) {
      return;
    }
    const nextLayerIds = [...layerIds];
    [nextLayerIds[index], nextLayerIds[nextIndex]] = [nextLayerIds[nextIndex], nextLayerIds[index]];
    void setVideoCompositionLayers(compositionId, nextLayerIds);
  };

  const addVideoOutput = async () => {
    try {
      const outputId = await invoke<number>("add_video_output", {
        label: videoOutputLabel(),
        kind: videoOutputKind(),
        width: videoOutputWidth(),
        height: videoOutputHeight(),
        fullscreen: videoOutputFullscreen(),
        monitorId: videoOutputKind() === "Display" ? videoOutputMonitorId() : null,
        endpointName: videoOutputKind() === "Display" ? null : videoOutputEndpoint(),
      });
      setVideoOutputLabel(`Projector ${snapshot().video.outputs.length + 2}`);
      setMessage(`Added video output ${outputId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeVideoOutput = async (outputId: number) => {
    try {
      await invoke("remove_video_output", { outputId });
      setMessage(`Removed video output ${outputId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputConfig = async (output: VideoOutputSummary) => {
    const draft = videoOutputConfigDraft(output);
    const width = Math.max(1, Math.round(Number.isFinite(draft.width) ? draft.width : output.width));
    const height = Math.max(1, Math.round(Number.isFinite(draft.height) ? draft.height : output.height));
    const monitorId = Math.max(0, Math.round(Number.isFinite(draft.monitor_id) ? draft.monitor_id : 0));
    try {
      await invoke("set_video_output_config", {
        outputId: output.id,
        label: draft.label,
        kind: draft.kind,
        width,
        height,
        fullscreen: draft.kind === "Display" ? draft.fullscreen : false,
        monitorId: draft.kind === "Display" ? monitorId : null,
        endpointName: draft.kind === "Display" ? null : draft.endpoint_name,
      });
      setVideoOutputConfigDrafts((current) => ({
        ...current,
        [output.id]: {
          ...draft,
          width,
          height,
          fullscreen: draft.kind === "Display" ? draft.fullscreen : false,
          monitor_id: draft.kind === "Display" ? monitorId : 0,
          endpoint_name: draft.kind === "Display" ? "" : draft.endpoint_name,
        },
      }));
      setMessage(`Updated video output ${output.id}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputEnabled = async (outputId: number, enabled: boolean) => {
    try {
      await invoke("set_video_output_enabled", { outputId, enabled });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputRouting = async (outputId: number, compositionId: number) => {
    try {
      await invoke("set_video_output_routing", { outputId, compositionId });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputOpacity = async (outputId: number, opacity: number) => {
    try {
      await invoke("set_video_output_opacity", { outputId, opacity });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const fadeVideoOutputOpacity = async (outputId: number, opacity: number) => {
    try {
      await invoke("fade_video_output_opacity", {
        outputId,
        opacity,
        durationMs: Math.max(0, Math.round(videoOutputFadeMs())),
      });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputBlackout = async (outputId: number, blackout: boolean) => {
    try {
      await invoke("set_video_output_blackout", { outputId, blackout });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setVideoOutputMapping = async (outputId: number, mapping: VideoOutputMapping) => {
    try {
      await invoke("set_video_output_mapping", { outputId, mapping });
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveVideoOutputMappingPreset = async (mapping: VideoOutputMapping) => {
    try {
      const label = await invoke<string>("save_video_output_mapping_preset", {
        label: videoOutputMappingPresetLabel(),
        mapping,
      });
      setVideoOutputMappingPresetLabel(label);
      setSelectedVideoOutputMappingPresetLabel(label);
      await refreshSnapshot();
      setMessage(`Saved projector mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const applyVideoOutputMappingPreset = async (outputId: number, label: string) => {
    const preset = snapshot().video.mapping_presets.find((candidate) => candidate.label === label);
    if (!preset) {
      return;
    }
    await setVideoOutputMapping(outputId, preset.mapping);
    setMessage(`Applied projector mapping preset ${preset.label}.`);
  };

  const removeVideoOutputMappingPreset = async (label: string) => {
    try {
      await invoke("remove_video_output_mapping_preset", { label });
      if (selectedVideoOutputMappingPresetLabel() === label) {
        setSelectedVideoOutputMappingPresetLabel("");
      }
      await refreshSnapshot();
      setMessage(`Removed projector mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const exportVideoOutputMappingPreset = async (mapping: VideoOutputMapping) => {
    try {
      const path = await invoke<string | null>("save_video_output_mapping_preset_file", {
        label: videoOutputMappingPresetLabel(),
        mapping,
      });
      setMessage(path ? `Exported projector mapping preset ${path}` : "Projector mapping preset export canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const importVideoOutputMappingPreset = async () => {
    try {
      const label = await invoke<string | null>("load_video_output_mapping_preset_file");
      if (!label) {
        setMessage("Projector mapping preset import canceled.");
        return;
      }
      setVideoOutputMappingPresetLabel(label);
      setSelectedVideoOutputMappingPresetLabel(label);
      await refreshSnapshot();
      setMessage(`Imported projector mapping preset ${label}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setWaveOriginFromStageCenter = () => {
    const bounds = stageWorldBounds();
    setWaveOriginX(Number(((bounds.minX + bounds.maxX) / 2).toFixed(2)));
    setWaveOriginY(0);
    setWaveOriginZ(Number(((bounds.minZ + bounds.maxZ) / 2).toFixed(2)));
  };

  const setWaveOriginFromSelectedFixture = () => {
    const fixture = selectedFixture();
    if (!fixture) {
      setMessage("Select a fixture before using it as the wave origin.");
      return;
    }
    setWaveOriginX(Number(fixture.position.x.toFixed(2)));
    setWaveOriginY(Number(fixture.position.y.toFixed(2)));
    setWaveOriginZ(Number(fixture.position.z.toFixed(2)));
  };

  const setWaveDirectionPreset = (x: number, y: number, z: number) => {
    setWaveDirectionX(x);
    setWaveDirectionY(y);
    setWaveDirectionZ(z);
  };

  const waveStageWorldFromPointer = (event: PointerEvent & { currentTarget: SVGElement }) => {
    const svg = event.currentTarget instanceof SVGSVGElement ? event.currentTarget : event.currentTarget.ownerSVGElement;
    if (!svg) {
      return null;
    }
    const rect = svg.getBoundingClientRect();
    const svgX = clampRange(((event.clientX - rect.left) / rect.width) * stageViewBoxSize, 0, stageViewBoxSize);
    const svgZ = clampRange(((event.clientY - rect.top) / rect.height) * stageViewBoxSize, 0, stageViewBoxSize);
    return svgPointToStageWorld(svgX, svgZ, stageWorldBounds());
  };

  const setWaveOriginFromStagePoint = (point: { x: number; z: number }) => {
    setWaveOriginX(Number(point.x.toFixed(2)));
    setWaveOriginY(0);
    setWaveOriginZ(Number(point.z.toFixed(2)));
  };

  const setWaveDirectionFromStagePoint = (point: { x: number; z: number }) => {
    const dx = point.x - waveOriginX();
    const dz = point.z - waveOriginZ();
    const length = Math.sqrt(dx * dx + dz * dz);
    if (length <= 0.001) {
      setWaveDirectionPreset(0, 0, 0);
      return;
    }
    setWaveDirectionPreset(Number((dx / length).toFixed(3)), 0, Number((dz / length).toFixed(3)));
  };

  const updateWaveStageFromPointer = (event: PointerEvent & { currentTarget: SVGElement }, mode: WaveStageDragMode) => {
    const point = waveStageWorldFromPointer(event);
    if (!point) {
      return;
    }
    if (mode === "origin") {
      setWaveOriginFromStagePoint(point);
    } else if (mode === "direction") {
      setWaveDirectionFromStagePoint(point);
    } else {
      setEffectVideoPositionFromStagePoint(point);
    }
  };

  const startWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }, mode: WaveStageDragMode) => {
    event.preventDefault();
    event.stopPropagation();
    event.currentTarget.setPointerCapture(event.pointerId);
    setWaveStageDrag(mode);
    updateWaveStageFromPointer(event, mode);
  };

  const moveWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }) => {
    const mode = waveStageDrag();
    if (!mode) {
      return;
    }
    event.preventDefault();
    updateWaveStageFromPointer(event, mode);
  };

  const endWaveStageDrag = (event: PointerEvent & { currentTarget: SVGElement }) => {
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setWaveStageDrag(null);
  };

  const setProjectorCornerFromPointer = (
    event: PointerEvent & { currentTarget: SVGCircleElement },
    output: VideoOutputSummary,
    corner: ProjectorCorner,
  ) => {
    event.preventDefault();
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) {
      return;
    }
    const rect = svg.getBoundingClientRect();
    const pointerX = ((event.clientX - rect.left) / rect.width) * projectorMapViewBoxSize;
    const pointerY = ((event.clientY - rect.top) / rect.height) * projectorMapViewBoxSize;
    const base = projectorMapBasePoint(output.mapping, corner);
    void setVideoOutputMapping(output.id, {
      ...output.mapping,
      [corner.xField]: roundedMappingValue((pointerX - base.x) / projectorCornerGain),
      [corner.yField]: roundedMappingValue((pointerY - base.y) / projectorCornerGain),
    });
  };

  const openVideoOutputWindow = async (outputId: number, testPattern = false) => {
    try {
      await invoke("open_video_output_window", { outputId, testPattern });
      setMessage(`Opened video output ${outputId} ${testPattern ? "test pattern" : "window"}.`);
    } catch (error) {
      setMessage(String(error));
    }
  };

  const buildEffectVideoTargets = (includePosition: boolean): VideoEffectTarget[] => {
    const videoLayerId = selectedEffectVideoLayerId();
    if (effectTargetMode() !== "video" || videoLayerId === null) {
      return [];
    }
    return [
      {
        layer_ids: [videoLayerId],
        param: effectVideoParam(),
        low: effectVideoLow(),
        high: effectVideoHigh(),
        position: includePosition ? effectVideoTargetPosition() : null,
      },
    ];
  };

  const addEffect = async () => {
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    const targetMode = effectTargetMode();
    const isVideoTarget = targetMode === "video";
    if (!isVideoTarget && (!fixture || !attribute)) {
      setMessage("Select a fixture and attribute first.");
      return;
    }
    const targetGroupIds = parseGroupIds(effectTargetGroups());
    if (targetMode === "group" && targetGroupIds.length === 0) {
      setMessage("Enter at least one target group.");
      return;
    }
    const videoLayerId = selectedEffectVideoLayerId();
    if (isVideoTarget && videoLayerId === null) {
      setMessage("Add a video layer before adding a video effect.");
      return;
    }
    const lightAttribute = attribute ?? "";

    try {
      const videoTargets = buildEffectVideoTargets(effectType() === "PositionWave");
      const requestBase = {
        label: `${isVideoTarget ? effectVideoParam() : lightAttribute} ${effectType()}`,
        fixture_ids: targetMode === "fixture" && fixture ? [fixture.id] : [],
        target_group_ids: targetMode === "group" ? targetGroupIds : [],
        attribute: isVideoTarget ? "" : lightAttribute,
        video_targets: videoTargets,
        shape: effectShape(),
        low: effectLow(),
        high: effectHigh(),
        phase: effectPhase(),
        blend_mode: effectBlendMode(),
      };
      const effectId =
        effectType() === "Lfo"
          ? await invoke<number>("add_lfo_effect", {
              request: {
                ...requestBase,
                period_ms: effectPeriod(),
              },
            })
          : await invoke<number>("add_position_wave_effect", {
              request: {
                ...requestBase,
                origin: { x: waveOriginX(), y: waveOriginY(), z: waveOriginZ() },
                direction: { x: waveDirectionX(), y: waveDirectionY(), z: waveDirectionZ() },
                speed: waveSpeed(),
                wavelength: waveWavelength(),
              },
            });
      setMessage(`Added ${effectType() === "PositionWave" ? "position wave" : "LFO"} effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const removeEffect = async (effectId: number) => {
    try {
      await invoke("remove_effect", { effectId });
      setMessage(`Removed effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const saveEffectPreset = async (effectId: number) => {
    try {
      const path = await invoke<string | null>("save_effect_preset", { effectId });
      setMessage(path ? `Saved effect preset ${path}` : "Effect preset save canceled.");
    } catch (error) {
      setMessage(String(error));
    }
  };

  const effectTargetOverrideError = () => {
    const targetMode = effectTargetMode();
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    if (targetMode === "fixture" && (!fixture || !attribute)) {
      return "Select a fixture and attribute first.";
    }
    if (targetMode === "group") {
      if (parseGroupIds(effectTargetGroups()).length === 0) {
        return "Enter at least one target group.";
      }
      if (!attribute) {
        return "Select a fixture profile attribute before targeting a group.";
      }
    }
    if (targetMode === "video" && selectedEffectVideoLayerId() === null) {
      return "Add a video layer before loading a video effect preset.";
    }
    return "";
  };

  const effectTargetOverrideFromForm = (): EffectTargetOverride | null => {
    if (effectTargetOverrideError()) {
      return null;
    }
    const targetMode = effectTargetMode();
    const fixture = selectedFixture();
    const attribute = selectedEffectAttribute();
    const videoTargets = buildEffectVideoTargets(true);
    return {
      fixture_ids: targetMode === "fixture" && fixture ? [fixture.id] : [],
      target_group_ids: targetMode === "group" ? parseGroupIds(effectTargetGroups()) : [],
      attribute: targetMode === "video" ? "" : attribute,
      video_targets: videoTargets,
    };
  };

  const loadEffectPreset = async () => {
    try {
      const effectId = await invoke<number | null>("load_effect_preset");
      if (effectId === null) {
        setMessage("Effect preset load canceled.");
        return;
      }
      setMessage(`Loaded effect preset as effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const loadEffectPresetForCurrentTarget = async () => {
    const targetOverride = effectTargetOverrideFromForm();
    if (!targetOverride) {
      setMessage(effectTargetOverrideError());
      return;
    }
    try {
      const effectId = await invoke<number | null>("load_effect_preset_for_target", { targetOverride });
      if (effectId === null) {
        setMessage("Effect preset load canceled.");
        return;
      }
      setMessage(`Loaded effect preset as effect ${effectId} for current target`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setEffectVideoTargetsFromSelectedOutput = async (effectId: number, targets: VideoEffectTarget[]) => {
    const output = selectedMappingVideoOutput();
    if (!output) {
      setMessage("Add or select a video output before updating video target positions.");
      return;
    }
    const layerIds = [...new Set(targets.flatMap((target) => target.layer_ids))];
    if (layerIds.length === 0) {
      setMessage(`Effect ${effectId} has no video target layers.`);
      return;
    }
    const mapping = mappingVideoOutputMapping(output);
    const position = {
      x: mapping.stage_x,
      y: 0,
      z: mapping.stage_z,
    };
    try {
      await Promise.all(
        layerIds.map((layerId) =>
          invoke("set_effect_video_target_position", {
            effectId,
            layerId,
            position,
          }),
        ),
      );
      setEffectVideoPosition(position);
      setSelectedVideoOutputId(output.id);
      setMessage(`Updated effect ${effectId} video target position from ${output.label}.`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const setEffectEnabled = async (effectId: number, enabled: boolean) => {
    try {
      await invoke("set_effect_enabled", { effectId, enabled });
      setMessage(`${enabled ? "Enabled" : "Disabled"} effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const moveEffect = async (effectId: number, delta: -1 | 1) => {
    try {
      await invoke("move_effect", { effectId, delta });
      setMessage(`Moved effect ${effectId}`);
      await refreshSnapshot();
    } catch (error) {
      setMessage(String(error));
    }
  };

  const handleControlKeyDown = (event: KeyboardEvent) => {
    if (
      event.repeat ||
      event.altKey ||
      isEditableShortcutTarget(event.target)
    ) {
      return;
    }

    if (workspaceTab() === "setup" && setupSubTab() === "mapping") {
      if ((event.ctrlKey || event.metaKey) && event.code === "KeyD") {
        event.preventDefault();
        void duplicateSelectedMappingFixtures();
        return;
      }
      if (event.ctrlKey || event.metaKey) {
        return;
      }
      const nextTool = mappingStageToolFromHotkey(event.code);
      if (nextTool) {
        event.preventDefault();
        setMappingStageTool(nextTool);
        setMessage(`2D mapping tool: ${nextTool.toUpperCase()}.`);
        return;
      }
      const nudgeAmount = normalizedMappingSnapSize() * (event.shiftKey ? 5 : 1);
      if (event.code === "ArrowLeft") {
        event.preventDefault();
        void nudgeSelectedMappingFixtures(-nudgeAmount, 0);
        return;
      }
      if (event.code === "ArrowRight") {
        event.preventDefault();
        void nudgeSelectedMappingFixtures(nudgeAmount, 0);
        return;
      }
      if (event.code === "ArrowUp") {
        event.preventDefault();
        void nudgeSelectedMappingFixtures(0, -nudgeAmount);
        return;
      }
      if (event.code === "ArrowDown") {
        event.preventDefault();
        void nudgeSelectedMappingFixtures(0, nudgeAmount);
        return;
      }
      if (event.code === "Delete" || event.code === "Backspace") {
        event.preventDefault();
        void removeSelectedMappingFixtures();
        return;
      }
      if (event.code === "Escape") {
        event.preventDefault();
        setMappingStageTool("select");
        setMappingDrag(null);
        setMappingMarquee(null);
        setMappingViewportPanDrag(null);
        return;
      }
    }

    if (workspaceTab() !== "control") {
      return;
    }
    if (event.ctrlKey || event.metaKey) {
      return;
    }

    if (event.code === "Space") {
      event.preventDefault();
      if (snapshot().cues.length === 0) {
        return;
      }
      if (event.shiftKey) {
        void triggerPreviousCue();
      } else {
        void triggerNextCue();
      }
      return;
    }

    if (event.shiftKey) {
      return;
    }

    const cueHotkeyIndex = controlCueHotkeyIndex(event.code);
    if (cueHotkeyIndex !== null) {
      const cue = snapshot().cues[cuePadStartIndex() + cueHotkeyIndex];
      if (cue) {
        event.preventDefault();
        void triggerCue(cue.id);
      }
      return;
    }

    if (event.code === "KeyG" || event.code === "Enter") {
      event.preventDefault();
      if (snapshot().cues.length > 0) {
        void triggerNextCue();
      }
      return;
    }
    if (event.code === "KeyP") {
      event.preventDefault();
      if (snapshot().active_fade) {
        void setCueFadePaused(!snapshot().active_fade?.paused);
      }
      return;
    }
    if (event.code === "KeyT") {
      event.preventDefault();
      if (snapshot().timeline.playing) {
        void pauseTimeline();
      } else if (snapshot().timeline.duration_ms > 0) {
        void playTimeline();
      }
      return;
    }
    if (event.code === "KeyB") {
      event.preventDefault();
      void setBlackout(!snapshot().blackout);
      return;
    }
    if (event.code === "KeyV") {
      event.preventDefault();
      void setVideoBlackout(!snapshot().video.blackout);
      return;
    }
    if (event.code === "KeyK") {
      event.preventDefault();
      void tapBpm();
    }
  };

  window.addEventListener("keydown", handleControlKeyDown);
  onCleanup(() => window.removeEventListener("keydown", handleControlKeyDown));

  return (
    <main class="app">
      <header class="topbar">
        <div>
          <h1>Rayard</h1>
          <p>Unified lighting and video control: GDTF patch, cues, timeline, effects, DMX output.</p>
        </div>
        <div class="status">
          <span class={snapshot().blackout || snapshot().video.blackout ? "pill danger" : "pill ok"}>
            {snapshot().blackout && snapshot().video.blackout
              ? "All BO"
              : snapshot().blackout
                ? "DMX BO"
                : snapshot().video.blackout
                  ? "Video BO"
                  : "Live"}
          </span>
          <span class="metric">{snapshot().clock.bpm.toFixed(1)} BPM</span>
          <span class="metric">{Math.round(snapshot().telemetry.last_tick_interval_us / 1000)} ms tick</span>
          <span class="metric">{Math.round(snapshot().telemetry.tick_jitter_stddev_us)} us jitter</span>
          <span class="metric">{snapshot().telemetry.last_packet_bytes} bytes</span>
          <span class="metric">
            {snapshot().telemetry.last_dmx_send_success_count}/{snapshot().telemetry.last_dmx_output_count} outputs
          </span>
          <span
            class={projectDirty() ? "metric projectFile dirty" : "metric projectFile"}
            title={currentProjectPath() ?? "Unsaved project"}
          >
            {projectFileLabel()}
          </span>
          <button onClick={newProject}>New</button>
          <button onClick={saveProject}>Save</button>
          <button onClick={saveProjectAs}>Save As</button>
          <button onClick={loadProject}>Load</button>
          <button onClick={loadPhase1SampleProject}>Load Sample</button>
          <button onClick={runPhase1Smoke}>Run Smoke</button>
        </div>
      </header>

      <nav class="workspaceTabs" aria-label="Workspace">
        <button
          class={workspaceTab() === "setup" ? "active" : ""}
          onClick={() => setWorkspaceTab("setup")}
          aria-pressed={workspaceTab() === "setup"}
        >
          Setup
        </button>
        <button
          class={workspaceTab() === "control" ? "active" : ""}
          onClick={() => setWorkspaceTab("control")}
          aria-pressed={workspaceTab() === "control"}
        >
          Control
        </button>
        <button
          class={workspaceTab() === "touch" ? "active" : ""}
          onClick={() => setWorkspaceTab("touch")}
          aria-pressed={workspaceTab() === "touch"}
        >
          Touch
        </button>
      </nav>

      <Show when={workspaceTab() === "setup"}>
        <nav class="setupModeTabs" aria-label="Setup mode">
          <For each={setupSubTabs}>
            {(tab) => (
              <button
                class={setupSubTab() === tab.id ? "active" : ""}
                title={tab.description}
                onClick={() => selectSetupMode(tab.id)}
                aria-pressed={setupSubTab() === tab.id}
              >
                {tab.label}
              </button>
            )}
          </For>
        </nav>
      </Show>

      <section class={`layout ${touchLayoutClass()}`}>
        <section class="panel liveControlPanel controlPanel">
          <div class="panelHeader">
            <h2>Live Desk</h2>
            <span>{snapshot().blackout || snapshot().video.blackout ? "Guarded" : "Ready"}</span>
          </div>
          <div class="liveStatusGrid">
            <div class="liveStatusItem">
              <span>Active cue</span>
              <strong>{activeCue()?.label ?? "None"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Next cue</span>
              <strong>{nextCue()?.label ?? "None"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Fixtures</span>
              <strong>{snapshot().fixtures.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Effects</span>
              <strong>{activeEffectCount()} / {snapshot().effects.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>DMX routes</span>
              <strong>{enabledDmxOutputCount()} / {snapshot().dmx_outputs.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Video outs</span>
              <strong>{enabledVideoOutputCount()} / {snapshot().video.outputs.length}</strong>
            </div>
            <div class="liveStatusItem">
              <span>Timeline</span>
              <strong>{snapshot().timeline.playing ? "Playing" : "Stopped"}</strong>
            </div>
            <div class="liveStatusItem">
              <span>BPM</span>
              <strong>{snapshot().clock.bpm.toFixed(1)}</strong>
            </div>
          </div>
          <div class="liveTransportGrid">
            <button onClick={triggerPreviousCue} disabled={snapshot().cues.length === 0}>
              Back
            </button>
            <button class="primary liveGoButton" onClick={triggerNextCue} disabled={snapshot().cues.length === 0}>
              GO
            </button>
            <button
              onClick={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
              disabled={!snapshot().active_fade}
            >
              {snapshot().active_fade?.paused ? "Resume Fade" : "Pause Fade"}
            </button>
            <button
              onClick={() => void (snapshot().timeline.playing ? pauseTimeline() : playTimeline())}
              disabled={!snapshot().timeline.playing && snapshot().timeline.duration_ms === 0}
            >
              {snapshot().timeline.playing ? "Pause Timeline" : "Play Timeline"}
            </button>
            <button
              class={snapshot().blackout ? "primary" : ""}
              onClick={() => void setBlackout(!snapshot().blackout)}
            >
              {snapshot().blackout ? "Clear DMX BO" : "DMX BO"}
            </button>
            <button
              class={snapshot().video.blackout ? "primary" : ""}
              onClick={() => void setVideoBlackout(!snapshot().video.blackout)}
            >
              {snapshot().video.blackout ? "Clear Video BO" : "Video BO"}
            </button>
          </div>
          <div class="liveStagePanel">
            <div class="liveStageHeader">
              <h3>Live Stage</h3>
              <span>{visualizerFixtures().length} fixture(s) / {snapshot().video.outputs.length} projector(s)</span>
            </div>
            <svg class="visualizerStage liveStage" viewBox={`0 0 ${stageViewBoxSize} ${stageViewBoxSize}`}>
              <defs>
                <pattern id="live-stage-grid" width="10" height="10" patternUnits="userSpaceOnUse">
                  <path d="M 10 0 L 0 0 0 10" />
                </pattern>
              </defs>
              <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
              <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
              <line class="stageAxis2d" x1={stageOrigin2d().x} y1="0" x2={stageOrigin2d().x} y2={stageViewBoxSize} />
              <line class="stageAxis2d" x1="0" y1={stageOrigin2d().z} x2={stageViewBoxSize} y2={stageOrigin2d().z} />
              <For each={visualizerVideoSurfaces2d()}>
                {(surface) => (
                  <g
                    class={[
                      "stageVideoSurface2d",
                      selectedVideoOutputId() === surface.id ? "selected" : "",
                      surface.active ? "" : "inactive",
                    ].filter(Boolean).join(" ")}
                    transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
                    opacity={Math.max(0.22, surface.opacity)}
                    onPointerDown={(event) => {
                      event.stopPropagation();
                      setSelectedVideoOutputId(surface.id);
                    }}
                  >
                    <rect
                      class="stageVideoSurfaceShape"
                      x={-surface.width / 2}
                      y={-surface.height / 2}
                      width={surface.width}
                      height={surface.height}
                    />
                    <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
                    <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
                    <text x={-surface.width / 2 + 1.2} y={-surface.height / 2 - 1.6}>
                      {surface.label}
                    </text>
                  </g>
                )}
              </For>
              <For each={visualizerFixtures()}>
                {(fixture) => (
                  <polygon
                    class="stageBeam"
                    points={fixture.beamPoints}
                    fill={fixture.color}
                    opacity={Math.max(0.08, fixture.intensity * 0.55)}
                  />
                )}
              </For>
              <For each={visualizerFixtures()}>
                {(fixture) => {
                  const className = () => [
                    "stageFixture",
                    "stageFixtureBlock",
                    `kind-${fixture.visualKind}`,
                    selectedFixtureId() === fixture.id ? "selected picked" : "",
                    selectedFixtureGroupFilter() && fixture.inGroupFilter ? "groupMatch" : "",
                    selectedFixtureTypeFilter() && fixture.typeKey === selectedFixtureTypeFilter() ? "typeMatch" : "",
                    fixture.highlighted ? "highlighted" : "",
                    fixture.soloed ? "soloed" : "",
                    fixture.parked ? "parked" : "",
                  ].filter(Boolean).join(" ");
                  return (
                    <g
                      class={className()}
                      transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
                      onPointerDown={(event) => {
                        event.stopPropagation();
                        const patchedFixture = snapshot().fixtures.find((candidate) => candidate.id === fixture.id);
                        if (patchedFixture) {
                          activateFixture(patchedFixture);
                        }
                      }}
                    >
                      <Show
                        when={fixture.visualKind === "bar" || fixture.visualKind === "panel"}
                        fallback={
                          <Show
                            when={fixture.visualKind === "laser"}
                            fallback={
                              <circle
                                class="stageFixtureShape"
                                cx="0"
                                cy="0"
                                r={Math.max(fixture.width, fixture.height) / 2 + fixture.intensity * 1.5}
                                fill={fixture.color}
                              />
                            }
                          >
                            <polygon
                              class="stageFixtureShape"
                              points={`0,${-fixture.height / 2} ${fixture.width / 2},${fixture.height / 2} ${-fixture.width / 2},${fixture.height / 2}`}
                              fill={fixture.color}
                            />
                          </Show>
                        }
                      >
                        <rect
                          class="stageFixtureShape"
                          x={-fixture.width / 2}
                          y={-fixture.height / 2}
                          width={fixture.width}
                          height={fixture.height}
                          fill={fixture.color}
                        />
                      </Show>
                      <line class="stageFixtureCenterLine" x1="0" y1="0" x2="0" y2="-7" />
                      <circle class="stageFixtureLaserMark" cx="0" cy="-7" r="0.9" />
                      <title>{`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}</title>
                    </g>
                  );
                }}
              </For>
            </svg>
          </div>
          <div class="liveCuePadHeader">
            <h3>Cue Pads</h3>
            <span>
              {snapshot().cues.length === 0 ? "0" : cuePadStartIndex() + 1}-
              {Math.min(cuePadStartIndex() + cuePadSize, snapshot().cues.length)} / {snapshot().cues.length}
            </span>
            <label class="checkbox compactCheckbox">
              <input
                type="checkbox"
                checked={cuePadFollowActive()}
                onChange={(event) => setCuePadFollowActive(event.currentTarget.checked)}
              />
              Follow
            </label>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.max(0, cuePadBank() - 1));
              }}
              disabled={cuePadBank() === 0}
            >
              Prev
            </button>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.min(cuePadBankCount() - 1, cuePadBank() + 1));
              }}
              disabled={cuePadBank() >= cuePadBankCount() - 1}
            >
              Next
            </button>
          </div>
          <div class="liveCuePadGrid">
            <For each={liveCuePads()}>
              {(pad) => (
                <button
                  class={`liveCuePad ${pad.cue?.id === snapshot().active_cue_id ? "active" : ""} ${
                    pad.cue?.id === nextCue()?.id ? "next" : ""
                  }`}
                  disabled={!pad.cue}
                  onClick={() => {
                    if (pad.cue) {
                      void triggerCue(pad.cue.id);
                    }
                  }}
                >
                  <span>{pad.slot}</span>
                  <strong>{pad.cue?.label ?? "Empty"}</strong>
                  <small>{pad.cue ? `#${pad.index + 1} / ${pad.cue.fade_ms} ms` : "-"}</small>
                </button>
              )}
            </For>
          </div>
          <Show when={snapshot().active_fade}>
            {(fade) => (
              <div class="liveFadeMeter">
                <span>
                  Fade {Math.round(fade().progress * 100)}% / {fade().remaining_ms} ms
                </span>
                <progress max="1" value={fade().progress} />
              </div>
            )}
          </Show>
          <div class="liveMasterGrid">
            <label>
              Lighting Master
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().lighting_master}
                onChange={(event) => void setLightingMaster(Number(event.currentTarget.value))}
              />
            </label>
            <label>
              Video Master
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().video.master_opacity}
                onChange={(event) => void setVideoMasterOpacity(Number(event.currentTarget.value))}
              />
            </label>
            <label>
              BPM
              <input
                type="number"
                min="20"
                max="300"
                step="0.1"
                value={bpmDraft()}
                onInput={(event) => setBpmDraft(event.currentTarget.value)}
              />
            </label>
            <button class="primary" onClick={tapBpm}>
              Tap
            </button>
          </div>
        </section>
        <section class="panel touchPanel touchCuePanel">
          <div class="panelHeader">
            <h2>Touch Cues</h2>
            <span>{activeCue()?.label ?? "Standby"}</span>
          </div>
          <div class="touchGoDeck">
            <button onClick={triggerPreviousCue} disabled={snapshot().cues.length === 0}>
              Back
            </button>
            <button class="primary" onClick={triggerNextCue} disabled={snapshot().cues.length === 0}>
              GO
            </button>
            <button
              onClick={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
              disabled={!snapshot().active_fade}
            >
              {snapshot().active_fade?.paused ? "Resume" : "Pause"}
            </button>
          </div>
          <div class="touchCueStatus">
            <div>
              <span>Active</span>
              <strong>{activeCue()?.label ?? "None"}</strong>
            </div>
            <div>
              <span>Next</span>
              <strong>{nextCue()?.label ?? "None"}</strong>
            </div>
          </div>
          <div class="liveCuePadHeader">
            <h3>Cue Pads</h3>
            <span>
              {snapshot().cues.length === 0 ? "0" : cuePadStartIndex() + 1}-
              {Math.min(cuePadStartIndex() + cuePadSize, snapshot().cues.length)} / {snapshot().cues.length}
            </span>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.max(0, cuePadBank() - 1));
              }}
              disabled={cuePadBank() === 0}
            >
              Prev
            </button>
            <button
              onClick={() => {
                setCuePadFollowActive(false);
                setCuePadBank(Math.min(cuePadBankCount() - 1, cuePadBank() + 1));
              }}
              disabled={cuePadBank() >= cuePadBankCount() - 1}
            >
              Next
            </button>
          </div>
          <div class="touchCuePadGrid">
            <For each={liveCuePads()}>
              {(pad) => (
                <button
                  class={`liveCuePad ${pad.cue?.id === snapshot().active_cue_id ? "active" : ""} ${
                    pad.cue?.id === nextCue()?.id ? "next" : ""
                  }`}
                  disabled={!pad.cue}
                  onClick={() => {
                    if (pad.cue) {
                      void triggerCue(pad.cue.id);
                    }
                  }}
                >
                  <span>{pad.slot}</span>
                  <strong>{pad.cue?.label ?? "Empty"}</strong>
                  <small>{pad.cue ? `${pad.cue.fade_ms} ms` : "-"}</small>
                </button>
              )}
            </For>
          </div>
          <Show when={snapshot().active_fade}>
            {(fade) => (
              <div class="liveFadeMeter touchFadeMeter">
                <span>{Math.round(fade().progress * 100)}%</span>
                <progress max="1" value={fade().progress} />
              </div>
            )}
          </Show>
          <div class="touchMasterGrid">
            <label>
              Lighting
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().lighting_master}
                onInput={(event) => void setLightingMaster(Number(event.currentTarget.value))}
              />
              <strong>{Math.round(snapshot().lighting_master * 100)}%</strong>
            </label>
            <label>
              Video
              <input
                type="range"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().video.master_opacity}
                onInput={(event) => void setVideoMasterOpacity(Number(event.currentTarget.value))}
              />
              <strong>{Math.round(snapshot().video.master_opacity * 100)}%</strong>
            </label>
          </div>
          <div class="touchGuardRow">
            <button class={snapshot().blackout ? "primary" : ""} onClick={() => void setBlackout(!snapshot().blackout)}>
              {snapshot().blackout ? "Clear DMX BO" : "DMX BO"}
            </button>
            <button
              class={snapshot().video.blackout ? "primary" : ""}
              onClick={() => void setVideoBlackout(!snapshot().video.blackout)}
            >
              {snapshot().video.blackout ? "Clear Video BO" : "Video BO"}
            </button>
          </div>
        </section>
        <section class="panel touchPanel touchFixturePanel">
          <div class="panelHeader">
            <h2>Touch Fixtures</h2>
            <span>{selectedFixtureGroupFilter() ? `Group ${selectedFixtureGroupFilter()}` : selectedFixture()?.label ?? "No selection"}</span>
          </div>
          <div class="touchGroupScroller">
            <button
              class={!selectedFixtureGroupFilter() ? "groupChip active" : "groupChip"}
              onClick={() => selectFixtureGroupFilter(null)}
            >
              All
              <span>{snapshot().fixtures.length}</span>
            </button>
            <For each={fixtureGroupRows()}>
              {(group) => (
                <button
                  class={selectedFixtureGroupFilter() === group.groupId ? "groupChip active" : "groupChip"}
                  onClick={() => selectFixtureGroupFilter(group.groupId)}
                >
                  {group.groupId}
                  <span>{group.count}</span>
                </button>
              )}
            </For>
          </div>
          <div class="touchFixtureScroller">
            <For each={filteredFixtures()}>
              {(fixture) => (
                <button
                  class={fixture.id === selectedFixtureId() ? "fixture selected" : "fixture"}
                  onClick={() => selectFixture(fixture)}
                >
                  <strong>{fixture.label}</strong>
                  <span>U{fixture.universe} A{fixture.address}</span>
                  <small>{fixture.group_ids.join(", ") || fixture.mode_name}</small>
                </button>
              )}
            </For>
            <Show when={filteredFixtures().length === 0}>
              <p class="empty">No fixtures.</p>
            </Show>
          </div>
          <Show when={selectedFixture()}>
            {(fixture) => (
              <div class="touchFixtureActions">
                <div class="touchGuardRow">
                  <Show
                    when={selectedFixtureGroupFilter()}
                    fallback={
                      <>
                        <button onClick={() => void setFixtureHighlight(fixture().id, !fixture().highlighted)}>
                          {fixture().highlighted ? "Clear Highlight" : "Highlight"}
                        </button>
                        <button onClick={() => void setFixtureSolo(fixture().id, !fixture().soloed)}>
                          {fixture().soloed ? "Clear Solo" : "Solo"}
                        </button>
                        <button onClick={() => void setFixturePark(fixture().id, !fixture().parked)}>
                          {fixture().parked ? "Clear Park" : "Park"}
                        </button>
                      </>
                    }
                  >
                    {(groupId) => (
                      <>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupHighlight(groupId(), !selectedGroupFlagState().anyHighlighted)}
                        >
                          {selectedGroupFlagState().anyHighlighted ? "Clear Group Highlight" : "Group Highlight"}
                        </button>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupSolo(groupId(), !selectedGroupFlagState().anySoloed)}
                        >
                          {selectedGroupFlagState().anySoloed ? "Clear Group Solo" : "Group Solo"}
                        </button>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupPark(groupId(), !selectedGroupFlagState().anyParked)}
                        >
                          {selectedGroupFlagState().anyParked ? "Clear Group Park" : "Group Park"}
                        </button>
                      </>
                    )}
                  </Show>
                </div>
                <Show when={selectedDimmerControl()}>
                  {(dimmer) => (
                    <label class="touchSlider">
                      Dimmer
                      <input
                        type="range"
                        min={normalizeLimitRange(selectedFixtureLimits().dimmer_min, selectedFixtureLimits().dimmer_max).min}
                        max={normalizeLimitRange(selectedFixtureLimits().dimmer_min, selectedFixtureLimits().dimmer_max).max}
                        value={dimmer().value}
                        onInput={(event) => setDimmerValue(Number(event.currentTarget.value))}
                      />
                      <strong>{formatDmxPercent(dimmer().value)}</strong>
                    </label>
                  )}
                </Show>
                <Show when={selectedPositionControls()}>
                  {(positionControls) => (
                    <div
                      class="panTiltPad touchPanTiltPad"
                      role="slider"
                      aria-label="Touch pan tilt pad"
                      aria-valuetext={`Pan ${positionControls().panValue}, Tilt ${positionControls().tiltValue}`}
                      onPointerDown={(event) => {
                        event.currentTarget.setPointerCapture(event.pointerId);
                        void setPanTiltFromPointer(event);
                      }}
                      onPointerMove={(event) => {
                        if (event.buttons === 1) {
                          void setPanTiltFromPointer(event);
                        }
                      }}
                      onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                    >
                      <b class="panTiltLimitWindow" style={selectedFixtureLimitOverlayStyle()} />
                      <i
                        style={{
                          left: `${(positionControls().panValue / 65535) * 100}%`,
                          top: `${100 - (positionControls().tiltValue / 65535) * 100}%`,
                        }}
                      />
                    </div>
                  )}
                </Show>
                <Show when={selectedColorControls()}>
                  <div class="touchColorGrid">
                    <For each={defaultColorPalette.slice(0, 8)}>
                      {(color) => (
                        <button
                          class="colorSwatch"
                          style={{ "background-color": color }}
                          title={color}
                          onClick={() => void setFixtureColor(color)}
                          aria-label={`Set color ${color}`}
                        />
                      )}
                    </For>
                  </div>
                </Show>
              </div>
            )}
          </Show>
        </section>
        <section class="panel touchPanel touchRemotePanel">
          <div class="panelHeader">
            <h2>Touch Remote</h2>
            <span>{remoteRunning() ? "Running" : "Stopped"}</span>
          </div>
          <div class="touchRemoteUrl">
            <span>Remote URL</span>
            <strong>{remoteUrl()}</strong>
          </div>
          <div class="split">
            <label>
              Bind IP
              <input value={remoteBindIp()} onInput={(event) => setRemoteBindIp(event.currentTarget.value)} />
            </label>
            <label>
              Port
              <input
                type="number"
                min="1"
                value={remotePort()}
                onInput={(event) => setRemotePort(Number(event.currentTarget.value))}
              />
            </label>
          </div>
          <div class="touchGuardRow">
            <button class="primary" onClick={startRemoteControl} disabled={remoteRunning()}>
              Start Remote
            </button>
            <button onClick={stopRemoteControl} disabled={!remoteRunning()}>
              Stop Remote
            </button>
          </div>
          <div class="touchMasterGrid compact">
            <label>
              BPM
              <input
                type="number"
                min="20"
                max="300"
                step="0.1"
                value={bpmDraft()}
                onInput={(event) => setBpmDraft(event.currentTarget.value)}
              />
            </label>
            <button onClick={applyBpm}>Set BPM</button>
            <button class="primary" onClick={tapBpm}>Tap</button>
          </div>
          <Show when={snapshot().submasters.length > 0}>
            <div class="submasterList">
              <h3>Submasters</h3>
              <For each={snapshot().submasters}>
                {(submaster) => (
                  <label class="submasterControl">
                    <span>{submaster.label}</span>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={submaster.level}
                      onInput={(event) => void setGroupSubmaster(submaster.group_id, Number(event.currentTarget.value))}
                    />
                    <strong>{Math.round(submaster.level * 100)}%</strong>
                  </label>
                )}
              </For>
            </div>
          </Show>
        </section>
        <section class="panel touchPanel touchVideoPanel">
          <div class="panelHeader">
            <h2>Touch Video</h2>
            <span>{snapshot().video.layers.length} layer(s)</span>
          </div>
          <Show when={snapshot().video.layers.length > 0} fallback={<p class="empty">No video layers.</p>}>
            <div class="touchVideoDeckGrid">
              <For each={snapshot().video.layers}>
                {(layer) => {
                  const durationMs = () => layer.source.metadata?.duration_ms ?? null;
                  const progress = () => {
                    const duration = durationMs();
                    return duration && duration > 0
                      ? clampRange((layer.state.position_ms / duration) * 100, 0, 100)
                      : 0;
                  };
                  const seekTo = (positionMs: number) =>
                    setVideoLayerState(layer.id, {
                      ...layer.state,
                      position_ms: durationMs()
                        ? Math.min(durationMs()!, Math.max(0, Math.round(positionMs)))
                        : Math.max(0, Math.round(positionMs)),
                    });
                  return (
                    <div class={layer.state.enabled ? "touchVideoDeck active" : "touchVideoDeck"}>
                      <div class="touchVideoHeader">
                        <div>
                          <strong>{layer.label}</strong>
                          <span>{layer.source.path ?? layer.source.name ?? layer.source.kind}</span>
                        </div>
                        <small>{formatVideoTime(layer.state.position_ms, durationMs())}</small>
                      </div>
                      <div class="touchVideoProgress">
                        <span style={{ width: `${progress()}%` }} />
                      </div>
                      <div class="touchTransportRow">
                        <button
                          class={layer.state.playing ? "primary" : ""}
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              playing: !layer.state.playing,
                            })
                          }
                        >
                          {layer.state.playing ? "Pause" : "Play"}
                        </button>
                        <button
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              position_ms: layer.state.loop_enabled ? layer.state.loop_start_ms : 0,
                            })
                          }
                        >
                          Cue In
                        </button>
                        <button
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              speed: -Math.abs(layer.state.speed || 1),
                              playing: true,
                            })
                          }
                        >
                          Rev
                        </button>
                        <button
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              speed: Math.abs(layer.state.speed || 1),
                              playing: true,
                            })
                          }
                        >
                          Fwd
                        </button>
                      </div>
                      <label class="touchSlider">
                        Opacity
                        <input
                          type="range"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.opacity}
                          onInput={(event) =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              opacity: Number(event.currentTarget.value),
                            })
                          }
                        />
                        <strong>{Math.round(layer.state.opacity * 100)}%</strong>
                      </label>
                      <label class="touchSlider">
                        Speed
                        <input
                          type="range"
                          min="-4"
                          max="4"
                          step="0.25"
                          value={layer.state.speed}
                          onChange={(event) =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              speed: Number(event.currentTarget.value),
                            })
                          }
                        />
                        <strong>{layer.state.speed.toFixed(2)}x</strong>
                      </label>
                      <Show when={durationMs()}>
                        {(duration) => (
                          <label class="touchSlider">
                            Seek
                            <input
                              type="range"
                              min="0"
                              max={duration()}
                              step="1"
                              value={layer.state.position_ms}
                              onChange={(event) => void seekTo(Number(event.currentTarget.value))}
                            />
                            <strong>{Math.round(progress())}%</strong>
                          </label>
                        )}
                      </Show>
                      <div class="touchTransportRow">
                        <button
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              enabled: !layer.state.enabled,
                            })
                          }
                        >
                          {layer.state.enabled ? "Disable" : "Enable"}
                        </button>
                        <button
                          class={layer.state.solo ? "primary" : ""}
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              solo: !layer.state.solo,
                            })
                          }
                        >
                          Solo
                        </button>
                        <button
                          class={layer.state.loop_enabled ? "primary" : ""}
                          onClick={() =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              loop_enabled: !layer.state.loop_enabled,
                            })
                          }
                        >
                          Loop
                        </button>
                        <button onClick={() => void addVideoCuePoint(layer.id)}>
                          Add Cue
                        </button>
                      </div>
                      <Show when={layer.state.cue_points_ms.length > 0}>
                        <div class="touchCuePointRow">
                          <For each={layer.state.cue_points_ms.slice(0, 6)}>
                            {(cuePoint, cuePointIndex) => (
                              <button onClick={() => void jumpVideoCuePoint(layer.id, cuePointIndex())}>
                                {formatDuration(cuePoint) ?? `${cuePoint}ms`}
                              </button>
                            )}
                          </For>
                        </div>
                      </Show>
                    </div>
                  );
                }}
              </For>
            </div>
          </Show>
        </section>
        <aside
          class={setupPanelClass("panel setup setupPanel", ["library", "profiles", "patch"])}
          ref={registerSetupPanel(["library", "profiles"])}
          tabIndex={-1}
        >
          <h2>Patch</h2>
          <label>
            GDTF path
            <input
              value={gdtfPath()}
              onInput={(event) => setGdtfPath(event.currentTarget.value)}
              placeholder="C:\\path\\fixture.gdtf"
            />
          </label>
          <div class="buttonRow">
            <button onClick={selectGdtfFile}>Browse</button>
            <button class="primary" onClick={importGdtf}>Load GDTF</button>
          </div>
          <label>
            GDTF Share URL
            <input
              value={gdtfShareUrl()}
              onInput={(event) => setGdtfShareUrl(event.currentTarget.value)}
              placeholder="https://gdtf-share.com/.../fixture.gdtf"
            />
          </label>
          <button onClick={downloadGdtfFromUrl}>Download GDTF URL</button>
          <div class="customProfileForm">
            <h3>Custom Profile</h3>
            <div class="split">
              <label>
                Maker
                <input value={customManufacturer()} onInput={(event) => setCustomManufacturer(event.currentTarget.value)} />
              </label>
              <label>
                Name
                <input value={customProfileName()} onInput={(event) => setCustomProfileName(event.currentTarget.value)} />
              </label>
            </div>
            <label>
              Mode
              <input value={customModeName()} onInput={(event) => setCustomModeName(event.currentTarget.value)} />
            </label>
            <label>
              Attributes
              <input
                value={customAttributes()}
                onInput={(event) => setCustomAttributes(event.currentTarget.value)}
                placeholder="Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8"
              />
            </label>
            <div class={`customProfilePreview ${customProfilePreview().errors.length > 0 ? "bad" : ""}`}>
              <div class="customProfilePreviewHeader">
                <strong>DMX Footprint</strong>
                <span>{customProfilePreview().footprint} ch</span>
              </div>
              <Show when={customProfilePreview().errors.length > 0}>
                <ul class="warnings">
                  <For each={customProfilePreview().errors}>{(error) => <li>{error}</li>}</For>
                </ul>
              </Show>
              <div class="customProfileChannelList">
                <For each={customProfilePreview().controls}>
                  {(control) => (
                    <div class="customProfileChannel">
                      <strong>{control.attribute}</strong>
                      <span>{control.resolution === "SixteenBit" ? "16-bit" : "8-bit"}</span>
                      <small>CH {control.offsets.join("/")}</small>
                    </div>
                  )}
                </For>
              </div>
            </div>
            <div class="buttonRow">
              <button
                onClick={createCustomProfile}
                disabled={customProfilePreview().errors.length > 0 || customProfilePreview().controls.length === 0}
              >
                Create Custom Profile
              </button>
              <button
                onClick={saveCustomProfile}
                disabled={customProfilePreview().errors.length > 0 || customProfilePreview().controls.length === 0}
              >
                Save Custom
              </button>
            </div>
            <button onClick={loadCustomProfile}>Load Custom Profile</button>
          </div>

          <Show when={profile()}>
            {(loaded) => (
              <div class="profile">
                <strong>{loaded().manufacturer} {loaded().name}</strong>
                <span>{loaded().dmx_modes.length} mode(s), {loaded().geometries.length} geometry node(s)</span>
                <label>
                  Mode
                  <select value={selectedMode()} onInput={(event) => setSelectedMode(event.currentTarget.value)}>
                    <For each={loaded().dmx_modes}>
                      {(mode) => <option value={mode.name}>{mode.name}</option>}
                    </For>
                  </select>
                </label>
                <label>
                  Label
                  <input value={label()} onInput={(event) => setLabel(event.currentTarget.value)} />
                </label>
                <div class="split">
                  <label>
                    Universe
                    <input
                      type="number"
                      min="0"
                      value={universe()}
                      onInput={(event) => setUniverse(Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    Address
                    <input
                      type="number"
                      min="1"
                      max="512"
                      value={address()}
                      onInput={(event) => setAddress(Number(event.currentTarget.value))}
                    />
                  </label>
                </div>
                <div class="split">
                  <label>
                    Count
                    <input
                      type="number"
                      min="1"
                      max="256"
                      value={patchCount()}
                      onInput={(event) => setPatchCount(Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    Addr Step
                    <input
                      type="number"
                      min="0"
                      value={patchAddressStride()}
                      onInput={(event) => setPatchAddressStride(Number(event.currentTarget.value))}
                    />
                  </label>
                </div>
                <div class="split">
                  <label>
                    Layout
                    <select
                      value={patchLayoutMode()}
                      onInput={(event) => setPatchLayoutMode(event.currentTarget.value as FixtureLayoutMode)}
                    >
                      <option value="line">Line</option>
                      <option value="grid">Grid</option>
                      <option value="circle">Circle</option>
                    </select>
                  </label>
                  <Show
                    when={patchLayoutMode() === "circle"}
                    fallback={
                      <label>
                        Grid Cols
                        <input
                          type="number"
                          min="1"
                          max="64"
                          value={patchGridColumns()}
                          onInput={(event) => setPatchGridColumns(Number(event.currentTarget.value))}
                          disabled={patchLayoutMode() !== "grid"}
                        />
                      </label>
                    }
                  >
                    <label>
                      Radius
                      <input
                        type="number"
                        min="0.1"
                        step="0.1"
                        value={patchCircleRadius()}
                        onInput={(event) => setPatchCircleRadius(Number(event.currentTarget.value))}
                      />
                    </label>
                  </Show>
                </div>
                <div class="triple">
                  <label>
                    X
                    <input type="number" value={patchX()} onInput={(event) => setPatchX(Number(event.currentTarget.value))} />
                  </label>
                  <label>
                    Y
                    <input type="number" value={patchY()} onInput={(event) => setPatchY(Number(event.currentTarget.value))} />
                  </label>
                  <label>
                    Z
                    <input type="number" value={patchZ()} onInput={(event) => setPatchZ(Number(event.currentTarget.value))} />
                  </label>
                </div>
                <div class="split">
                  <label>
                    X Step
                    <input type="number" value={patchXStep()} onInput={(event) => setPatchXStep(Number(event.currentTarget.value))} />
                  </label>
                  <label>
                    Z Step
                    <input type="number" value={patchZStep()} onInput={(event) => setPatchZStep(Number(event.currentTarget.value))} />
                  </label>
                </div>
                <div class="triple">
                  <label>
                    Pitch
                    <input type="number" value={patchPitch()} onInput={(event) => setPatchPitch(Number(event.currentTarget.value))} />
                  </label>
                  <label>
                    Yaw
                    <input type="number" value={patchYaw()} onInput={(event) => setPatchYaw(Number(event.currentTarget.value))} />
                  </label>
                  <label>
                    Roll
                    <input type="number" value={patchRoll()} onInput={(event) => setPatchRoll(Number(event.currentTarget.value))} />
                  </label>
                </div>
                <label>
                  Groups
                  <input
                    value={groupText()}
                    onInput={(event) => setGroupText(event.currentTarget.value)}
                    placeholder="front, movers"
                  />
                </label>
                <div class={patchAddressInvalid() ? "footprint bad" : "footprint"}>
                  <span>Footprint {selectedFootprint()}ch</span>
                  <span>Count {patchCountValue()}</span>
                  <span>Step {patchAddressStrideValue()}ch</span>
                  <span>
                    {patchLayoutMode() === "grid"
                      ? `Grid ${patchGridColumnsValue()} col`
                      : patchLayoutMode() === "circle"
                        ? `Circle r${patchCircleRadiusValue().toFixed(1)}`
                        : "Line"}
                  </span>
                  <span>End address {endAddress()}</span>
                  <Show when={patchAddressConflictText()}>
                    {(text) => <span>{text()}</span>}
                  </Show>
                </div>
                <button class="primary" onClick={patchFixture} disabled={patchAddressInvalid()}>
                  {patchCountValue() === 1 ? "Patch Fixture" : "Patch Fixtures"}
                </button>
                <button onClick={selectNextFreePatchAddress} disabled={nextFreePatchAddress() === null}>
                  Next Free A{nextFreePatchAddress() ?? "-"}
                </button>
                <Show when={loaded().warnings.length > 0}>
                  <ul class="warnings">
                    <For each={loaded().warnings}>{(warning) => <li>{warning}</li>}</For>
                  </ul>
                </Show>
              </div>
            )}
          </Show>
        </aside>

        <section
          class={setupPanelClass("panel fixtures setupPanel", ["patch", "mapping"])}
          ref={registerSetupPanel(["patch"])}
          tabIndex={-1}
        >
          <div class="panelHeader">
            <h2>Fixtures</h2>
            <span>{filteredFixtures().length} / {snapshot().fixtures.length}</span>
          </div>
          <div class="groupOverview">
            <button
              class={!selectedFixtureGroupFilter() ? "groupChip active" : "groupChip"}
              onClick={() => selectFixtureGroupFilter(null)}
            >
              All
              <span>{snapshot().fixtures.length}</span>
            </button>
            <For each={fixtureGroupRows()}>
              {(group) => (
                <button
                  class={selectedFixtureGroupFilter() === group.groupId ? "groupChip active" : "groupChip"}
                  onClick={() => selectFixtureGroupFilter(group.groupId)}
                >
                  {group.groupId}
                  <span>{group.count}</span>
                </button>
              )}
            </For>
          </div>
          <div class="dmxPatchMap">
            <div class="panelHeader">
              <h3>DMX Patch Grid</h3>
              <div class="panelHeaderActions">
                <button onClick={selectNextFreePatchAddress} disabled={nextFreePatchAddress() === null}>
                  Next Free
                </button>
                <select
                  value={activePatchGridUniverse()}
                  onInput={(event) => setPatchGridUniverse(Number(event.currentTarget.value))}
                >
                  <For each={patchGridUniverseOptions()}>
                    {(universeId) => <option value={universeId}>Universe {universeId}</option>}
                  </For>
                </select>
                <div class="viewToggle" aria-label="DMX map view">
                  <button
                    class={dmxPatchViewMode() === "grid" ? "active" : ""}
                    onClick={() => setDmxPatchViewMode("grid")}
                    aria-pressed={dmxPatchViewMode() === "grid"}
                  >
                    Grid
                  </button>
                  <button
                    class={dmxPatchViewMode() === "list" ? "active" : ""}
                    onClick={() => setDmxPatchViewMode("list")}
                    aria-pressed={dmxPatchViewMode() === "list"}
                  >
                    List
                  </button>
                </div>
              </div>
            </div>
            <Show
              when={dmxPatchViewMode() === "grid"}
              fallback={
                <div class="dmxPatchList">
                  <For each={activePatchGridMap().segments}>
                    {(segment) => (
                      <button
                        class={segment.fixture.id === selectedFixtureId() ? "dmxPatchListItem selected" : "dmxPatchListItem"}
                        onClick={() => selectFixture(segment.fixture)}
                      >
                        <strong>{segment.fixture.label}</strong>
                        <span>
                          U{activePatchGridUniverse()} A{segment.start}-{segment.end} / {segment.end - segment.start + 1}ch
                        </span>
                        <small>{segment.fixture.manufacturer} {segment.fixture.profile_name}</small>
                      </button>
                    )}
                  </For>
                  <Show when={activePatchGridMap().segments.length === 0}>
                    <p class="empty">No fixtures patched in this universe.</p>
                  </Show>
                </div>
              }
            >
              <div class="dmxAddressGrid" role="grid" aria-label={`Universe ${activePatchGridUniverse()} DMX addresses`}>
                <For each={dmxAddressCells()}>
                  {(cell) => (
                    <button
                      class={`dmxAddressCell ${cell.segment ? "occupied" : ""} ${cell.isStart ? "start" : ""} ${
                        cell.plannedIndex !== null ? "planned" : ""
                      } ${cell.plannedStart ? "plannedStart" : ""} ${cell.plannedConflict ? "plannedConflict" : ""} ${
                        cell.isSelected ? "selected" : ""
                      }`}
                      title={
                        cell.segment || cell.plannedIndex !== null
                          ? [
                              `U${activePatchGridUniverse()} A${cell.channel}`,
                              cell.segment ? `${cell.segment.fixture.label} (${cell.segment.start}-${cell.segment.end})` : "",
                              cell.plannedIndex !== null ? `Pending fixture ${cell.plannedIndex + 1}` : "",
                            ]
                              .filter(Boolean)
                              .join(" / ")
                          : `U${activePatchGridUniverse()} A${cell.channel}`
                      }
                      onClick={() => handleDmxAddressCellClick(cell)}
                      aria-label={
                        cell.segment
                          ? `Address ${cell.channel}, ${cell.segment.fixture.label}`
                          : `Address ${cell.channel}, empty`
                      }
                    >
                      {cell.channel}
                    </button>
                  )}
                </For>
              </div>
            </Show>
            <div class="dmxGridSummary">
              <span>{activePatchGridMap().used}/512 used</span>
              <span>{activePatchGridMap().largestFree}ch max free</span>
              <span>{plannedAddressSummary()}</span>
              <span>{snapshot().fixtures.length} fixture(s)</span>
            </div>
          </div>
          <div class="dmxPatchMap compact">
            <div class="panelHeader">
              <h3>Universe Overview</h3>
              <span>{dmxUniverseMaps().length} universe(s)</span>
            </div>
            <For each={dmxUniverseMaps()}>
              {(map) => (
                <div class="dmxUniverseMap">
                  <div class="dmxUniverseHeader">
                    <strong>U{map.universe}</strong>
                    <span>{map.used}/512 used</span>
                    <span>{map.largestFree}ch max free</span>
                  </div>
                  <div class="dmxUniverseTrack">
                    <For each={map.segments}>
                      {(segment) => (
                        <button
                          class={segment.fixture.id === selectedFixtureId() ? "dmxPatchSegment selected" : "dmxPatchSegment"}
                          style={{
                            left: `${segment.left}%`,
                            width: `${Math.max(segment.width, 0.7)}%`,
                          }}
                          title={`${segment.fixture.label} A${segment.start}-${segment.end}`}
                          onClick={() => selectFixture(segment.fixture)}
                        />
                      )}
                    </For>
                  </div>
                  <div class="dmxUniverseLegend">
                    <For each={map.segments.slice(0, 6)}>
                      {(segment) => (
                        <button onClick={() => selectFixture(segment.fixture)}>
                          A{segment.start}-{segment.end} {segment.fixture.label}
                        </button>
                      )}
                    </For>
                    <Show when={map.segments.length === 0}>
                      <span>Empty</span>
                    </Show>
                    <Show when={map.segments.length > 6}>
                      <span>+{map.segments.length - 6}</span>
                    </Show>
                  </div>
                </div>
              )}
            </For>
          </div>
          <div class="fixtureList">
            <For each={filteredFixtures()}>
              {(fixture) => (
                <button
                  class={fixture.id === selectedFixtureId() ? "fixture selected" : "fixture"}
                  onClick={() => selectFixture(fixture)}
                >
                  <strong>{fixture.label}</strong>
                  <span>{fixture.manufacturer} {fixture.profile_name}</span>
                  <small>U{fixture.universe} A{fixture.address} / {fixture.mode_name}</small>
                  <Show when={fixture.highlighted || fixture.soloed || fixture.parked}>
                    <small>{[fixture.highlighted ? "Highlight" : "", fixture.soloed ? "Solo" : "", fixture.parked ? "Park" : ""].filter(Boolean).join(" / ")}</small>
                  </Show>
                  <Show when={fixture.group_ids.length > 0}>
                    <small>{fixture.group_ids.join(", ")}</small>
                  </Show>
                </button>
              )}
            </For>
            <Show when={snapshot().fixtures.length === 0}>
              <p class="empty">No fixtures patched.</p>
            </Show>
            <Show when={snapshot().fixtures.length > 0 && filteredFixtures().length === 0}>
              <p class="empty">No fixtures in group.</p>
            </Show>
          </div>
          <Show when={selectedFixture()}>
            {(fixture) => (
              <div class="fixtureSetupEditor">
                <div class="panelHeader">
                  <h3>Fixture Setup</h3>
                  <span>{fixture().label}</span>
                </div>
                <button onClick={() => void useFixtureProfileForPatch(fixture())}>
                  Use Profile for Patch
                </button>
                <button onClick={() => void duplicateFixture(fixture())}>
                  Duplicate Fixture
                </button>
                <label>
                  Label
                  <input
                    value={selectedFixtureLabelDraft()}
                    onInput={(event) => setSelectedFixtureLabelDraft(event.currentTarget.value)}
                  />
                </label>
                <div class="split">
                  <label>
                    Universe
                    <input
                      type="number"
                      min="0"
                      value={selectedFixtureUniverseDraft()}
                      onInput={(event) => setSelectedFixtureUniverseDraft(Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    Address
                    <input
                      type="number"
                      min="1"
                      max="512"
                      value={selectedFixtureAddressDraft()}
                      onInput={(event) => setSelectedFixtureAddressDraft(Number(event.currentTarget.value))}
                    />
                  </label>
                </div>
                <button onClick={() => void setFixturePatch(fixture())}>Apply Patch</button>
                <label>
                  Groups
                  <input
                    value={selectedFixtureGroupText()}
                    onInput={(event) => setSelectedFixtureGroupText(event.currentTarget.value)}
                    placeholder="front, movers"
                  />
                </label>
                <button onClick={() => void setFixtureGroups(fixture())}>Apply Groups</button>
                <div class="limitEditor">
                  <div class="limitEditorHeader">
                    <strong>Dimmer Limits</strong>
                    <span>
                      {formatDmxPercent(normalizedSelectedFixtureLimitsDraft().dimmer_min)} -{" "}
                      {formatDmxPercent(normalizedSelectedFixtureLimitsDraft().dimmer_max)}
                    </span>
                  </div>
                  <div class="split">
                    <label>
                      Min
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={selectedFixtureLimitsDraft().dimmer_min}
                        onInput={(event) => updateSelectedFixtureLimit("dimmer_min", Number(event.currentTarget.value))}
                      />
                    </label>
                    <label>
                      Max
                      <input
                        type="number"
                        min="0"
                        max="65535"
                        value={selectedFixtureLimitsDraft().dimmer_max}
                        onInput={(event) => updateSelectedFixtureLimit("dimmer_max", Number(event.currentTarget.value))}
                      />
                    </label>
                  </div>
                  <div class="limitEditorHeader">
                    <strong>Movement Limits</strong>
                    <span>
                      Pan {formatDmxPercent(normalizedSelectedFixtureLimitsDraft().pan_min)} -{" "}
                      {formatDmxPercent(normalizedSelectedFixtureLimitsDraft().pan_max)}
                    </span>
                  </div>
                  <div class="movementLimitEditor">
                    <div
                      class={movementLimitDrag() ? "movementLimitMap dragging" : "movementLimitMap"}
                      aria-label="Pan tilt movement limits"
                      role="slider"
                      aria-valuetext={`Pan ${normalizedSelectedFixtureLimitsDraft().pan_min}-${normalizedSelectedFixtureLimitsDraft().pan_max}, Tilt ${normalizedSelectedFixtureLimitsDraft().tilt_min}-${normalizedSelectedFixtureLimitsDraft().tilt_max}`}
                      onPointerDown={startMovementLimitDrag}
                      onPointerMove={dragMovementLimit}
                      onPointerUp={endMovementLimitDrag}
                      onPointerCancel={endMovementLimitDrag}
                    >
                      <i style={selectedFixtureLimitWindowStyle()} />
                    </div>
                    <div class="movementLimitFields">
                      <div class="split">
                        <label>
                          Pan Min
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={selectedFixtureLimitsDraft().pan_min}
                            onInput={(event) => updateSelectedFixtureLimit("pan_min", Number(event.currentTarget.value))}
                          />
                        </label>
                        <label>
                          Pan Max
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={selectedFixtureLimitsDraft().pan_max}
                            onInput={(event) => updateSelectedFixtureLimit("pan_max", Number(event.currentTarget.value))}
                          />
                        </label>
                      </div>
                      <div class="split">
                        <label>
                          Tilt Min
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={selectedFixtureLimitsDraft().tilt_min}
                            onInput={(event) => updateSelectedFixtureLimit("tilt_min", Number(event.currentTarget.value))}
                          />
                        </label>
                        <label>
                          Tilt Max
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={selectedFixtureLimitsDraft().tilt_max}
                            onInput={(event) => updateSelectedFixtureLimit("tilt_max", Number(event.currentTarget.value))}
                          />
                        </label>
                      </div>
                      <div class="limitToggleRow">
                        <label>
                          <input
                            type="checkbox"
                            checked={selectedFixtureLimitsDraft().invert_pan}
                            onChange={(event) => updateSelectedFixtureLimit("invert_pan", event.currentTarget.checked)}
                          />
                          Invert Pan
                        </label>
                        <label>
                          <input
                            type="checkbox"
                            checked={selectedFixtureLimitsDraft().invert_tilt}
                            onChange={(event) => updateSelectedFixtureLimit("invert_tilt", event.currentTarget.checked)}
                          />
                          Invert Tilt
                        </label>
                        <label>
                          <input
                            type="checkbox"
                            checked={selectedFixtureLimitsDraft().swap_pan_tilt}
                            onChange={(event) => updateSelectedFixtureLimit("swap_pan_tilt", event.currentTarget.checked)}
                          />
                          Swap
                        </label>
                      </div>
                    </div>
                  </div>
                  <div class="presetRow">
                    <button onClick={() => setSelectedFixtureLimitsDraft(defaultFixtureLimits)}>
                      Reset
                    </button>
                    <button class="primary" onClick={() => void setFixtureLimits(fixture())}>
                      Apply Limits
                    </button>
                  </div>
                </div>
                <div class="transformEditor">
                  <strong>2D Mapping</strong>
                  <div class="triple">
                    <label>
                      X
                      <input
                        type="number"
                        value={fixture().position.x}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, x: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Y
                      <input
                        type="number"
                        value={fixture().position.y}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, y: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Z
                      <input
                        type="number"
                        value={fixture().position.z}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, z: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                  <div class="triple">
                    <label>
                      Pitch
                      <input
                        type="number"
                        value={fixture().rotation.pitch}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, pitch: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Yaw
                      <input
                        type="number"
                        value={fixture().rotation.yaw}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, yaw: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Roll
                      <input
                        type="number"
                        value={fixture().rotation.roll}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, roll: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                  <div class="presetRow">
                    <button onClick={() => void layoutFixturePositions("line")} disabled={filteredFixtures().length === 0}>
                      Line X
                    </button>
                    <button onClick={() => void layoutFixturePositions("grid")} disabled={filteredFixtures().length === 0}>
                      Grid
                    </button>
                    <button onClick={() => void layoutFixturePositions("circle")} disabled={filteredFixtures().length === 0}>
                      Circle
                    </button>
                  </div>
                </div>
              </div>
            )}
          </Show>
          <div class="mappingVisualizer visualizer">
            <div class="panelHeader">
              <h2>2D Mapping</h2>
              <span>{mappingFilteredFixtures().length} fixture(s) / {snapshot().video.outputs.length} projector(s)</span>
            </div>
            <div class="mappingGroupStrip">
              <span>Groups</span>
              <button
                class={!selectedFixtureGroupFilter() ? "active" : ""}
                onClick={() => selectFixtureGroupFilter(null)}
              >
                All
                <small>{snapshot().fixtures.length}</small>
              </button>
              <For each={fixtureGroupRows()}>
                {(group) => (
                  <button
                    class={selectedFixtureGroupFilter() === group.groupId ? "active" : ""}
                    onClick={() => selectFixtureGroupFilter(group.groupId)}
                  >
                    {group.groupId}
                    <small>{group.count}</small>
                  </button>
                )}
              </For>
            </div>
            <div class="mappingTypeStrip">
              <span>Types</span>
              <button
                class={!selectedFixtureTypeFilter() ? "active" : ""}
                onClick={() => setSelectedFixtureTypeFilter(null)}
              >
                <span class="mappingTypeGlyph kind-all" />
                All
                <small>{filteredFixtures().length}</small>
              </button>
              <For each={fixtureTypeRows()}>
                {(row) => (
                  <button
                    class={selectedFixtureTypeFilter() === row.key ? "active" : ""}
                    onClick={() => setSelectedFixtureTypeFilter(row.key)}
                    title={`${row.manufacturer} ${row.label}`}
                  >
                    <span class={mappingTypeGlyphClass(row.visualKind)} />
                    {row.label}
                    <small>{row.count}</small>
                  </button>
                )}
              </For>
            </div>
            <div class="mappingStageShell">
              <div class="mappingToolRail" aria-label="2D mapping tools">
                <button
                  class={mappingStageTool() === "select" ? "active" : ""}
                  onClick={() => setMappingStageTool("select")}
                  title="Select fixture or projector (S)"
                >
                  S
                </button>
                <button
                  class={mappingStageTool() === "place" ? "active" : ""}
                  onClick={() => setMappingStageTool("place")}
                  title="Place selected fixture (P)"
                >
                  P
                </button>
                <button
                  class={mappingStageTool() === "rotate" ? "active" : ""}
                  onClick={() => setMappingStageTool("rotate")}
                  title="Set selected fixture yaw (R)"
                >
                  R
                </button>
                <button
                  class={mappingStageTool() === "pan" ? "active" : ""}
                  onClick={() => setMappingStageTool("pan")}
                  title="Pan stage view (H)"
                >
                  H
                </button>
                <div class="mappingToolDivider" />
                <button
                  class={mappingShowLabels() ? "active" : ""}
                  onClick={() => setMappingShowLabels((value) => !value)}
                  title="Toggle labels"
                >
                  L
                </button>
                <button
                  class={mappingShowBeams() ? "active" : ""}
                  onClick={() => setMappingShowBeams((value) => !value)}
                  title="Toggle beams"
                >
                  B
                </button>
                <button
                  class={mappingShowProjectors() ? "active" : ""}
                  onClick={() => setMappingShowProjectors((value) => !value)}
                  title="Toggle projector surfaces"
                >
                  V
                </button>
              </div>
              <div class="mappingStageViewport">
                <div class="mappingViewportControls">
                  <span>Tool</span>
                  <strong>{mappingStageTool().toUpperCase()}</strong>
                  <span>Fixtures</span>
                  <strong>{mappingFilteredFixtures().length}</strong>
                  <span>Outputs</span>
                  <strong>{snapshot().video.outputs.length}</strong>
                </div>
                <div class="mappingViewControls" aria-label="2D mapping viewport">
                  <span>View</span>
                  <button onClick={fitMappingViewportToVisible} disabled={!canFitMappingViewportToVisible()}>
                    Fit All
                  </button>
                  <button onClick={fitMappingViewportToSelection} disabled={!canFitMappingViewportToSelection()}>
                    Fit Sel
                  </button>
                  <button onClick={() => zoomMappingViewport(-1)} disabled={normalizedMappingViewportZoom() <= 1.001}>
                    Zoom -
                  </button>
                  <strong>{mappingViewportZoomLabel()}</strong>
                  <button onClick={() => zoomMappingViewport(1)} disabled={normalizedMappingViewportZoom() >= 3.999}>
                    Zoom +
                  </button>
                  <button onClick={resetMappingViewport} disabled={normalizedMappingViewportZoom() <= 1.001}>
                    Reset
                  </button>
                </div>
                <div class="mappingStageSnapControls">
                  <button
                    class={!mappingSnapEnabled() ? "active" : ""}
                    onClick={() => setMappingSnapEnabled(false)}
                  >
                    Snap Off
                  </button>
                  <For each={mappingSnapPresets}>
                    {(preset) => (
                      <button
                        class={mappingSnapEnabled() && Math.abs(normalizedMappingSnapSize() - preset) < 0.001 ? "active" : ""}
                        onClick={() => {
                          setMappingSnapSize(preset);
                          setMappingSnapEnabled(true);
                        }}
                      >
                        {preset}m
                      </button>
                    )}
                  </For>
                  <label>
                    Grid
                    <input
                      type="number"
                      min="0.05"
                      max="20"
                      step="0.05"
                      value={normalizedMappingSnapSize()}
                      onChange={(event) => {
                        setMappingSnapSize(Number(event.currentTarget.value));
                        setMappingSnapEnabled(true);
                      }}
                    />
                  </label>
                </div>
                <div class="mappingLayerToggles">
                  <label>
                    <input
                      type="checkbox"
                      checked={mappingShowLabels()}
                      onChange={(event) => setMappingShowLabels(event.currentTarget.checked)}
                    />
                    Labels
                  </label>
                  <label>
                    <input
                      type="checkbox"
                      checked={mappingShowBeams()}
                      onChange={(event) => setMappingShowBeams(event.currentTarget.checked)}
                    />
                    Beams
                  </label>
                  <label>
                    <input
                      type="checkbox"
                      checked={mappingShowProjectors()}
                      onChange={(event) => setMappingShowProjectors(event.currentTarget.checked)}
                    />
                    Projectors
                  </label>
                </div>
                <div class="mappingStageBoundsPanel">
                  <div class="mappingStageBoundsStatus">
                    <strong>Stage Bounds</strong>
                    <span>
                      {snapshot().stage_map.locked ? "Locked" : "Auto fit"} / X {stageWorldBounds().minX.toFixed(1)} to{" "}
                      {stageWorldBounds().maxX.toFixed(1)} / Z {stageWorldBounds().minZ.toFixed(1)} to{" "}
                      {stageWorldBounds().maxZ.toFixed(1)}
                    </span>
                  </div>
                  <div class="mappingStageBoundsActions">
                    <button
                      class={snapshot().stage_map.locked ? "active" : ""}
                      onClick={() => void lockStageMapToCurrentBounds()}
                    >
                      Fit Current
                    </button>
                    <button
                      class={!snapshot().stage_map.locked ? "active" : ""}
                      onClick={() => void setStageMapConfig({ locked: false }, "Unlocked 2D stage map auto-fit.")}
                    >
                      Auto Fit
                    </button>
                  </div>
                  <div class="mappingStageBoundsGrid">
                    <label>
                      Min X
                      <input
                        type="number"
                        step="0.1"
                        value={snapshot().stage_map.min_x}
                        onChange={(event) =>
                          void setStageMapConfig({ locked: true, min_x: Number(event.currentTarget.value) })
                        }
                      />
                    </label>
                    <label>
                      Max X
                      <input
                        type="number"
                        step="0.1"
                        value={snapshot().stage_map.max_x}
                        onChange={(event) =>
                          void setStageMapConfig({ locked: true, max_x: Number(event.currentTarget.value) })
                        }
                      />
                    </label>
                    <label>
                      Min Z
                      <input
                        type="number"
                        step="0.1"
                        value={snapshot().stage_map.min_z}
                        onChange={(event) =>
                          void setStageMapConfig({ locked: true, min_z: Number(event.currentTarget.value) })
                        }
                      />
                    </label>
                    <label>
                      Max Z
                      <input
                        type="number"
                        step="0.1"
                        value={snapshot().stage_map.max_z}
                        onChange={(event) =>
                          void setStageMapConfig({ locked: true, max_z: Number(event.currentTarget.value) })
                        }
                      />
                    </label>
                  </div>
                  <div class="mappingStagePresetPanel">
                    <label>
                      Preset
                      <input
                        type="text"
                        value={stageMapPresetLabel()}
                        onInput={(event) => setStageMapPresetLabel(event.currentTarget.value)}
                      />
                    </label>
                    <label>
                      Stored
                      <select
                        value={selectedStageMapPresetLabel()}
                        disabled={snapshot().stage_map_presets.length === 0}
                        onInput={(event) => setSelectedStageMapPresetLabel(event.currentTarget.value)}
                      >
                        <option value="">Select preset</option>
                        <For each={snapshot().stage_map_presets}>
                          {(preset) => <option value={preset.label}>{preset.label}</option>}
                        </For>
                      </select>
                    </label>
                    <div class="mappingStagePresetActions">
                      <button onClick={() => void saveStageMapPreset()}>
                        Save
                      </button>
                      <button onClick={() => void exportStageMapPreset()}>
                        Export
                      </button>
                      <button onClick={() => void importStageMapPreset()}>
                        Import
                      </button>
                      <button
                        disabled={!selectedStageMapPresetLabel()}
                        onClick={() => void applyStageMapPreset(selectedStageMapPresetLabel())}
                      >
                        Apply
                      </button>
                      <button
                        disabled={!selectedStageMapPresetLabel()}
                        onClick={() => void removeStageMapPreset(selectedStageMapPresetLabel())}
                      >
                        Delete
                      </button>
                    </div>
                  </div>
                </div>
                <svg
                  class={[
                    "visualizerStage",
                    "editableStage",
                    mappingDrag() || mappingViewportPanDrag() ? "dragging" : "",
                    mappingStageTool() === "place"
                      ? "placeMode"
                      : mappingStageTool() === "rotate"
                        ? "rotateMode"
                        : mappingStageTool() === "pan"
                          ? "panMode"
                          : "selectMode",
                  ].filter(Boolean).join(" ")}
                  viewBox={mappingStageViewBox()}
                  role="img"
                  aria-label="2D fixture and projector mapping stage"
                  onPointerDown={(event) => void handleMappingStagePointerDown(event)}
                  onPointerMove={handleMappingStagePointerMove}
                  onPointerUp={(event) => void finishMappingStageDrag(event)}
                  onPointerCancel={(event) => void finishMappingStageDrag(event)}
                  onWheel={handleMappingStageWheel}
                >
                  <defs>
                    <pattern id="stage-grid" width="10" height="10" patternUnits="userSpaceOnUse">
                      <path d="M 10 0 L 0 0 0 10" />
                    </pattern>
                  </defs>
                  <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
                  <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
                  <line class="stageAxis2d" x1={stageOrigin2d().x} y1="0" x2={stageOrigin2d().x} y2={stageViewBoxSize} />
                  <line class="stageAxis2d" x1="0" y1={stageOrigin2d().z} x2={stageViewBoxSize} y2={stageOrigin2d().z} />
                  <Show when={mappingSnapEnabled()}>
                    <g>
                      <For each={mappingSnapLines()}>
                        {(line) => (
                          <line
                            class={`stageSnapLine ${line.axis}`}
                            x1={line.axis === "x" ? line.svg : 0}
                            y1={line.axis === "z" ? line.svg : 0}
                            x2={line.axis === "x" ? line.svg : stageViewBoxSize}
                            y2={line.axis === "z" ? line.svg : stageViewBoxSize}
                          />
                        )}
                      </For>
                    </g>
                  </Show>
                  <Show when={mappingMarqueeBox()}>
                    {(box) => (
                      <rect
                        class="mappingMarquee"
                        x={box().x}
                        y={box().z}
                        width={box().width}
                        height={box().height}
                      />
                    )}
                  </Show>
                  <Show when={mappingShowProjectors()}>
                    <For each={visualizerVideoSurfaces2d()}>
                      {(surface) => (
                        <g
                          class={[
                            "stageVideoSurface2d",
                            selectedVideoOutputId() === surface.id ? "selected" : "",
                            isDraggingMappingVideoOutput(surface.id) ? "dragging" : "",
                            surface.active ? "" : "inactive",
                          ].filter(Boolean).join(" ")}
                          transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
                          opacity={Math.max(0.22, surface.opacity)}
                          onPointerDown={(event) => {
                            if (mappingStageTool() === "pan") {
                              return;
                            }
                            event.stopPropagation();
                            setSelectedVideoOutputId(surface.id);
                            beginMappingVideoOutputDrag(event, surface.id);
                          }}
                        >
                          <rect
                            class="stageVideoSurfaceShape"
                            x={-surface.width / 2}
                            y={-surface.height / 2}
                            width={surface.width}
                            height={surface.height}
                          />
                          <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
                          <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
                          <text x={-surface.width / 2 + 1.2} y={-surface.height / 2 - 1.6}>
                            {surface.label}
                          </text>
                          <Show when={selectedVideoOutputId() === surface.id}>
                            <line
                              class="stageVideoSurfaceHandleLine"
                              x1="0"
                              y1={-surface.height / 2}
                              x2="0"
                              y2={-surface.height / 2 - 5}
                            />
                            <circle
                              class="stageVideoSurfaceRotateHandle"
                              cx="0"
                              cy={-surface.height / 2 - 5}
                              r="1.8"
                              onPointerDown={(event) => {
                                if (mappingStageTool() === "pan") {
                                  return;
                                }
                                event.preventDefault();
                                event.stopPropagation();
                                setSelectedVideoOutputId(surface.id);
                                beginMappingVideoOutputRotate(event, surface.id);
                              }}
                            />
                            <circle
                              class="stageVideoSurfaceScaleHandle"
                              cx={surface.width / 2 + 2.4}
                              cy={surface.height / 2 + 2.4}
                              r="1.8"
                              onPointerDown={(event) => {
                                if (mappingStageTool() === "pan") {
                                  return;
                                }
                                event.preventDefault();
                                event.stopPropagation();
                                setSelectedVideoOutputId(surface.id);
                                beginMappingVideoOutputScale(event, surface.id);
                              }}
                            />
                          </Show>
                        </g>
                      )}
                    </For>
                  </Show>
                  <Show when={mappingShowBeams()}>
                    <For each={visualizerFixtures()}>
                      {(fixture) => (
                        <polygon
                          class="stageBeam"
                          points={fixture.beamPoints}
                          fill={fixture.color}
                          opacity={Math.max(0.08, fixture.intensity * 0.55)}
                        />
                      )}
                    </For>
                  </Show>
                  <For each={visualizerFixtures()}>
                    {(fixture) => {
                      const selected = () => selectedMappingFixtureIdSet().has(fixture.id);
                      const className = () => [
                        "stageFixture",
                        "stageFixtureBlock",
                        `kind-${fixture.visualKind}`,
                        selected() ? "selected picked" : "",
                        isDraggingMappingFixture(fixture.id) ? "dragging" : "",
                        selectedFixtureGroupFilter() && fixture.inGroupFilter ? "groupMatch" : "",
                        selectedFixtureTypeFilter() && fixture.typeKey === selectedFixtureTypeFilter() ? "typeMatch" : "",
                        fixture.highlighted ? "highlighted" : "",
                        fixture.soloed ? "soloed" : "",
                        fixture.parked ? "parked" : "",
                      ].filter(Boolean).join(" ");
                      return (
                        <>
                          <g
                            class={className()}
                            transform={`translate(${fixture.x} ${fixture.z}) rotate(${fixture.yaw})`}
                            onPointerDown={(event) => {
                              if (mappingStageTool() === "pan") {
                                return;
                              }
                              event.stopPropagation();
                              const patchedFixture = snapshot().fixtures.find((candidate) => candidate.id === fixture.id);
                              if (patchedFixture && mappingStageTool() === "rotate") {
                                selectMappingFixture(patchedFixture, event);
                              }
                              beginMappingFixtureDrag(event, fixture.id);
                            }}
                          >
                            <Show
                              when={fixture.visualKind === "bar" || fixture.visualKind === "panel"}
                              fallback={
                                <Show
                                  when={fixture.visualKind === "laser"}
                                  fallback={
                                    <circle
                                      class="stageFixtureShape"
                                      cx="0"
                                      cy="0"
                                      r={Math.max(fixture.width, fixture.height) / 2 + fixture.intensity * 1.5}
                                      fill={fixture.color}
                                    />
                                  }
                                >
                                  <polygon
                                    class="stageFixtureShape"
                                    points={`0,${-fixture.height / 2} ${fixture.width / 2},${fixture.height / 2} ${-fixture.width / 2},${fixture.height / 2}`}
                                    fill={fixture.color}
                                  />
                                </Show>
                              }
                            >
                              <rect
                                class="stageFixtureShape"
                                x={-fixture.width / 2}
                                y={-fixture.height / 2}
                                width={fixture.width}
                                height={fixture.height}
                                fill={fixture.color}
                              />
                            </Show>
                            <line class="stageFixtureCenterLine" x1="0" y1="0" x2="0" y2="-7" />
                            <circle class="stageFixtureLaserMark" cx="0" cy="-7" r="0.9" />
                            <title>{`${fixture.label} / ${fixture.dmxLabel} / ${fixture.groupLabel}`}</title>
                          </g>
                          <Show when={mappingShowLabels()}>
                            <text class="stageLabel" x={fixture.x + 3.5} y={fixture.z - 3.5}>
                              {fixture.label}
                            </text>
                          </Show>
                        </>
                      );
                    }}
                  </For>
                </svg>
              </div>
              <aside class="mappingSelectionPanel">
                <div class="mappingSelectionHeader">
                  <strong>Selections</strong>
                  <span>
                    {selectedMappingFixtures().length > 0
                      ? `${selectedMappingFixtures().length} picked / ${mappingFilteredFixtures().length}`
                      : `${mappingFilteredFixtures().length} fixture(s)`}
                  </span>
                </div>
                <div class="mappingSearchPanel">
                  <label>
                    Search
                    <input
                      type="search"
                      value={mappingFixtureSearch()}
                      onInput={(event) => setMappingFixtureSearch(event.currentTarget.value)}
                      placeholder="label, U1 A24, group"
                    />
                  </label>
                  <button onClick={() => setMappingFixtureSearch("")} disabled={mappingFixtureSearch().trim().length === 0}>
                    Clear
                  </button>
                  <div class="mappingSearchActions">
                    <button onClick={pickVisibleMappingFixtures} disabled={mappingFilteredFixtures().length === 0}>
                      Pick Visible
                      <span>{mappingFilteredFixtures().length}</span>
                    </button>
                    <button
                      onClick={() => void duplicateSelectedMappingFixtures()}
                      disabled={selectedMappingFixtures().length === 0}
                    >
                      Duplicate
                      <span>{selectedMappingFixtures().length}</span>
                    </button>
                    <button
                      onClick={() => void removeSelectedMappingFixtures()}
                      disabled={selectedMappingFixtures().length === 0}
                    >
                      Remove
                      <span>{selectedMappingFixtures().length}</span>
                    </button>
                    <button onClick={clearMappingFixtureSelection} disabled={selectedMappingFixtures().length === 0}>
                      Clear Pick
                      <span>{selectedMappingFixtures().length}</span>
                    </button>
                  </div>
                </div>
                <div class="mappingSelectionAssign">
                  <label>
                    Selected Groups
                    <input
                      type="text"
                      value={mappingSelectionGroupText()}
                      onInput={(event) => setMappingSelectionGroupText(event.currentTarget.value)}
                      placeholder="front, movers, floor"
                    />
                  </label>
                  <div class="mappingSelectionQuickActions">
                    <button
                      onClick={() => void applyMappingSelectionGroups("add")}
                      disabled={selectedMappingFixtures().length === 0 || parseGroupIds(mappingSelectionGroupText()).length === 0}
                    >
                      Add
                    </button>
                    <button
                      onClick={() => void applyMappingSelectionGroups("remove")}
                      disabled={selectedMappingFixtures().length === 0 || parseGroupIds(mappingSelectionGroupText()).length === 0}
                    >
                      Remove
                    </button>
                    <button
                      onClick={() => void applyMappingSelectionGroups("set")}
                      disabled={selectedMappingFixtures().length === 0 || parseGroupIds(mappingSelectionGroupText()).length === 0}
                    >
                      Set
                    </button>
                  </div>
                </div>
                <div class="mappingTransformInspector">
                  <div class="mappingTransformTitle">
                    <strong>Selection Flags</strong>
                    <span>
                      {selectedMappingFlagState().count > 0
                        ? `${selectedMappingFlagState().count} fixture(s) picked`
                        : "Pick fixtures on the map or list."}
                    </span>
                  </div>
                  <div class="mappingFlagActions">
                    <button
                      class={selectedMappingFlagState().allHighlighted ? "active" : ""}
                      onClick={() => void setMappingSelectionFlag("highlight", !selectedMappingFlagState().allHighlighted)}
                      disabled={selectedMappingFlagState().count === 0}
                    >
                      {selectedMappingFlagState().allHighlighted ? "Clear High" : "Highlight"}
                    </button>
                    <button
                      class={selectedMappingFlagState().allSoloed ? "active" : ""}
                      onClick={() => void setMappingSelectionFlag("solo", !selectedMappingFlagState().allSoloed)}
                      disabled={selectedMappingFlagState().count === 0}
                    >
                      {selectedMappingFlagState().allSoloed ? "Clear Solo" : "Solo"}
                    </button>
                    <button
                      class={selectedMappingFlagState().allParked ? "active" : ""}
                      onClick={() => void setMappingSelectionFlag("park", !selectedMappingFlagState().allParked)}
                      disabled={selectedMappingFlagState().count === 0}
                    >
                      {selectedMappingFlagState().allParked ? "Clear Park" : "Park"}
                    </button>
                  </div>
                  <div class="mappingNudgePanel">
                    <span>Nudge {normalizedMappingSnapSize()}m</span>
                    <div class="mappingNudgeGrid">
                      <button
                        onClick={() => void nudgeSelectedMappingFixtures(0, -normalizedMappingSnapSize())}
                        disabled={selectedMappingFlagState().count === 0}
                      >
                        Up
                      </button>
                      <button
                        onClick={() => void nudgeSelectedMappingFixtures(-normalizedMappingSnapSize(), 0)}
                        disabled={selectedMappingFlagState().count === 0}
                      >
                        Left
                      </button>
                      <button
                        onClick={() => void nudgeSelectedMappingFixtures(normalizedMappingSnapSize(), 0)}
                        disabled={selectedMappingFlagState().count === 0}
                      >
                        Right
                      </button>
                      <button
                        onClick={() => void nudgeSelectedMappingFixtures(0, normalizedMappingSnapSize())}
                        disabled={selectedMappingFlagState().count === 0}
                      >
                        Down
                      </button>
                    </div>
                  </div>
                </div>
                <Show when={selectedMappingFixtures().length > 1}>
                  <div class="mappingTransformInspector">
                    <div class="mappingTransformTitle">
                      <strong>{selectedMappingFixtures().length} fixtures selected</strong>
                      <span>Drag any selected fixture to move the group.</span>
                    </div>
                    <div class="mappingTransformActions">
                      <button onClick={() => void layoutSelectedMappingFixtures("line")}>Line</button>
                      <button onClick={() => void layoutSelectedMappingFixtures("grid")}>Grid</button>
                      <button onClick={() => void layoutSelectedMappingFixtures("circle")}>Circle</button>
                      <button onClick={() => void alignSelectedMappingFixtures("x")}>Align X</button>
                      <button onClick={() => void alignSelectedMappingFixtures("z")}>Align Z</button>
                      <button
                        onClick={() => void distributeSelectedMappingFixtures("x")}
                        disabled={selectedMappingFixtures().length < 3}
                      >
                        Distribute X
                      </button>
                      <button
                        onClick={() => void distributeSelectedMappingFixtures("z")}
                        disabled={selectedMappingFixtures().length < 3}
                      >
                        Distribute Z
                      </button>
                      <button onClick={() => void duplicateSelectedMappingFixtures()}>Duplicate</button>
                      <button onClick={() => void removeSelectedMappingFixtures()}>Remove</button>
                      <button onClick={() => setWorkspaceTab("control")}>Control Active</button>
                      <button onClick={clearMappingFixtureSelection}>Clear</button>
                    </div>
                  </div>
                </Show>
                <Show
                  when={selectedMappingFixture()}
                  fallback={
                    <div class="mappingTransformInspector">
                      <div class="mappingTransformTitle">
                        <strong>No fixture selected</strong>
                        <span>Select a fixture on the map or patch list.</span>
                      </div>
                    </div>
                  }
                >
                  {(fixture) => (
                    <div class="mappingTransformInspector">
                      <div class="mappingTransformTitle">
                        <strong>{fixture().label}</strong>
                        <span>{fixture().manufacturer} {fixture().profile_name} / U{fixture().universe} A{fixture().address}</span>
                      </div>
                      <div class="mappingTransformGrid">
                        <label>
                          X
                          <input
                            type="number"
                            step="0.1"
                            value={fixture().position.x}
                            onChange={(event) =>
                              void setFixtureTransform(fixture(), {
                                position: { ...fixture().position, x: Number(event.currentTarget.value) },
                              })
                            }
                          />
                        </label>
                        <label>
                          Z
                          <input
                            type="number"
                            step="0.1"
                            value={fixture().position.z}
                            onChange={(event) =>
                              void setFixtureTransform(fixture(), {
                                position: { ...fixture().position, z: Number(event.currentTarget.value) },
                              })
                            }
                          />
                        </label>
                        <label>
                          Y
                          <input
                            type="number"
                            step="0.1"
                            value={fixture().position.y}
                            onChange={(event) =>
                              void setFixtureTransform(fixture(), {
                                position: { ...fixture().position, y: Number(event.currentTarget.value) },
                              })
                            }
                          />
                        </label>
                        <label>
                          Yaw
                          <input
                            type="number"
                            step="1"
                            value={fixture().rotation.yaw}
                            onChange={(event) =>
                              void setFixtureTransform(fixture(), {
                                rotation: { ...fixture().rotation, yaw: Number(event.currentTarget.value) },
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="mappingFlagActions">
                        <button
                          class={fixture().highlighted ? "active" : ""}
                          onClick={() => void setFixtureHighlight(fixture().id, !fixture().highlighted)}
                        >
                          Highlight
                        </button>
                        <button
                          class={fixture().soloed ? "active" : ""}
                          onClick={() => void setFixtureSolo(fixture().id, !fixture().soloed)}
                        >
                          Solo
                        </button>
                        <button
                          class={fixture().parked ? "active" : ""}
                          onClick={() => void setFixturePark(fixture().id, !fixture().parked)}
                        >
                          Park
                        </button>
                      </div>
                      <div class="mappingControlShortcuts">
                        <button onClick={() => setWorkspaceTab("control")}>Control</button>
                        <button onClick={() => selectSetupMode("patch")}>Patch</button>
                        <button onClick={() => void duplicateSelectedMappingFixtures()}>Duplicate</button>
                        <button onClick={() => void removeSelectedMappingFixtures()}>Remove</button>
                      </div>
                    </div>
                  )}
                </Show>
                <div class="mappingSelectionHeader secondary">
                  <strong>Fixtures</strong>
                  <span>{mappingFilteredFixtures().length}</span>
                </div>
                <For each={mappingFilteredFixtures()}>
                  {(fixture) => (
                    <button
                      class={selectedMappingFixtureIdSet().has(fixture.id) ? "active" : ""}
                      onClick={(event) => selectMappingFixture(fixture, event)}
                    >
                      <strong>{fixture.label}</strong>
                      <span>U{fixture.universe} A{fixture.address}</span>
                      <small>{fixture.manufacturer} {fixture.profile_name} / {fixture.group_ids.join(", ") || "No group"}</small>
                    </button>
                  )}
                </For>
                <Show when={mappingFilteredFixtures().length === 0}>
                  <div class="mappingTransformInspector">
                    <div class="mappingTransformTitle">
                      <strong>No fixtures</strong>
                      <span>Patch fixtures or adjust filters.</span>
                    </div>
                  </div>
                </Show>
                <div class="mappingSelectionHeader secondary">
                  <strong>Projectors</strong>
                  <span>{snapshot().video.outputs.length}</span>
                </div>
                <For each={snapshot().video.outputs}>
                  {(output) => (
                    <button
                      class={`projector ${selectedVideoOutputId() === output.id ? "active" : ""}`}
                      onClick={() => setSelectedVideoOutputId(output.id)}
                    >
                      <strong>{output.label}</strong>
                      <span>{output.width}x{output.height}</span>
                      <small>{output.kind} / {output.enabled ? "Enabled" : "Disabled"} / {output.blackout ? "Blackout" : "Live"}</small>
                    </button>
                  )}
                </For>
                <Show when={selectedMappingVideoOutput()}>
                  {(output) => (
                    <div class="mappingProjectorControls">
                      <div class="mappingProjectorPreview">
                        <div class="outputMappingMiniSurface">
                          <svg viewBox="0 0 100 100" role="img" aria-label={`${output().label} mapping preview`}>
                            <polygon class="projectorMapBase" points={projectorMapBasePoints(output().mapping).join(" ")} />
                            <polygon class="projectorMapWarp" points={projectorMapPoints(output().mapping).join(" ")} />
                          </svg>
                        </div>
                        <div>
                          <strong>{output().label}</strong>
                          <span>{output().mapping.aspect_mode} / {output().mapping.aspect_ratio.toFixed(2)} / lens {output().mapping.lens_distortion.toFixed(2)}</span>
                        </div>
                      </div>
                      <div class="mappingProjectorActionRow">
                        <button onClick={() => void setVideoOutputEnabled(output().id, !output().enabled)}>
                          {output().enabled ? "Disable" : "Enable"}
                        </button>
                        <button onClick={() => void setVideoOutputBlackout(output().id, !output().blackout)}>
                          {output().blackout ? "Clear BO" : "Blackout"}
                        </button>
                        <button onClick={() => void openVideoOutputWindow(output().id, true)}>Pattern</button>
                      </div>
                      <div class="mappingProjectorFieldGrid">
                        <label>
                          X
                          <input
                            type="number"
                            step="0.1"
                            value={output().mapping.stage_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                stage_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Z
                          <input
                            type="number"
                            step="0.1"
                            value={output().mapping.stage_z}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                stage_z: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Y
                          <input
                            type="number"
                            step="0.1"
                            value={output().mapping.stage_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                stage_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Rot
                          <input
                            type="number"
                            step="1"
                            value={output().mapping.rotation_deg}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                rotation_deg: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="mappingProjectorWarpGrid">
                        <label>
                          Aspect
                          <input
                            type="number"
                            step="0.01"
                            min="0.1"
                            value={output().mapping.aspect_ratio}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                aspect_ratio: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Mode
                          <select
                            value={output().mapping.aspect_mode}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                aspect_mode: event.currentTarget.value as VideoOutputAspectMode,
                              })
                            }
                          >
                            <For each={videoOutputAspectModes}>
                              {(mode) => <option value={mode}>{mode}</option>}
                            </For>
                          </select>
                        </label>
                        <label>
                          Lens
                          <input
                            type="number"
                            step="0.01"
                            min="-1"
                            max="1"
                            value={output().mapping.lens_distortion}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                lens_distortion: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Keystone X
                          <input
                            type="number"
                            step="0.01"
                            min="-1"
                            max="1"
                            value={output().mapping.keystone_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output().id, {
                                ...output().mapping,
                                keystone_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <button onClick={() => void setVideoOutputMapping(output().id, defaultVideoOutputMapping)}>
                          Reset Mapping
                        </button>
                      </div>
                    </div>
                  )}
                </Show>
              </aside>
            </div>
          </div>
        </section>

        <section
          class={setupPanelClass("panel videoSetupPanel setupPanel", ["mapping", "output"])}
          ref={registerSetupPanel(["mapping"])}
          tabIndex={-1}
        >
          <div class="panelHeader">
            <h2>Video Setup</h2>
            <span>{snapshot().video.outputs.length} output(s)</span>
          </div>
          <div class="compositionList">
            <For each={snapshot().video.compositions}>
              {(composition) => (
                <div class="compositionItem">
                  <div class="compositionHeader">
                    <strong>{composition.label}</strong>
                    <span>{composition.layer_ids.length} layer(s) / {composition.output_ids.length} output(s)</span>
                  </div>
                  <Show when={composition.id !== 1}>
                    <Show when={composition.layer_ids.length > 0}>
                      <div class="compositionLayerOrder">
                        <For each={composition.layer_ids}>
                          {(layerId, orderIndex) => {
                            const layerLabel = () =>
                              snapshot().video.layers.find((candidate) => candidate.id === layerId)?.label ?? `Layer ${layerId}`;
                            return (
                              <div class="compositionOrderRow">
                                <span>{orderIndex() + 1}. {layerLabel()}</span>
                                <div class="buttonRow">
                                  <button
                                    onClick={() => moveVideoCompositionLayer(composition.id, composition.layer_ids, layerId, -1)}
                                    disabled={orderIndex() === 0}
                                  >
                                    Up
                                  </button>
                                  <button
                                    onClick={() => moveVideoCompositionLayer(composition.id, composition.layer_ids, layerId, 1)}
                                    disabled={orderIndex() === composition.layer_ids.length - 1}
                                  >
                                    Down
                                  </button>
                                </div>
                              </div>
                            );
                          }}
                        </For>
                      </div>
                    </Show>
                    <div class="compositionLayerPicker">
                      <For each={snapshot().video.layers}>
                        {(layer) => (
                          <label class="checkbox">
                            <input
                              type="checkbox"
                              checked={composition.layer_ids.includes(layer.id)}
                              onChange={(event) => {
                                const layerIds = event.currentTarget.checked
                                  ? [...composition.layer_ids, layer.id]
                                  : composition.layer_ids.filter((candidate) => candidate !== layer.id);
                                void setVideoCompositionLayers(composition.id, layerIds);
                              }}
                            />
                            {layer.label}
                          </label>
                        )}
                      </For>
                    </div>
                    <button onClick={() => void removeVideoComposition(composition.id)}>Remove Composition</button>
                  </Show>
                </div>
              )}
            </For>
          </div>
          <div class="videoCompositionForm">
            <h3>Composition</h3>
            <label>
              Name
              <input value={videoCompositionLabel()} onInput={(event) => setVideoCompositionLabel(event.currentTarget.value)} />
            </label>
            <div class="compositionLayerPicker">
              <For each={snapshot().video.layers}>
                {(layer) => (
                  <label class="checkbox">
                    <input
                      type="checkbox"
                      checked={videoCompositionLayerIds().includes(layer.id)}
                      onChange={(event) => toggleVideoCompositionLayer(layer.id, event.currentTarget.checked)}
                    />
                    {layer.label}
                  </label>
                )}
              </For>
            </div>
            <button class="primary" onClick={addVideoComposition}>
              Add Composition
            </button>
          </div>
          <div class="videoOutputForm">
            <h3>Composition Output</h3>
            <div class="split">
              <label>
                Output label
                <input value={videoOutputLabel()} onInput={(event) => setVideoOutputLabel(event.currentTarget.value)} />
              </label>
              <label>
                Kind
                <select value={videoOutputKind()} onInput={(event) => setVideoOutputKind(event.currentTarget.value as VideoOutputKind)}>
                  <option value="Display">Display</option>
                  <option value="NdiSender">NDI Sender</option>
                  <option value="SpoutSender">Spout Sender</option>
                  <option value="SyphonServer">Syphon Server</option>
                </select>
              </label>
            </div>
            <div class="split">
              <label>
                Width
                <input type="number" min="1" value={videoOutputWidth()} onInput={(event) => setVideoOutputWidth(Number(event.currentTarget.value))} />
              </label>
              <label>
                Height
                <input type="number" min="1" value={videoOutputHeight()} onInput={(event) => setVideoOutputHeight(Number(event.currentTarget.value))} />
              </label>
            </div>
            <label>
              Output Fade ms
              <input
                type="number"
                min="0"
                step="10"
                value={videoOutputFadeMs()}
                onInput={(event) => setVideoOutputFadeMs(Number(event.currentTarget.value))}
              />
            </label>
            <Show when={videoOutputKind() === "Display"}>
              <div class="split">
                <label>
                  Monitor
                  <input type="number" min="0" value={videoOutputMonitorId()} onInput={(event) => setVideoOutputMonitorId(Number(event.currentTarget.value))} />
                </label>
                <label class="checkbox inlineCheckbox">
                  <input type="checkbox" checked={videoOutputFullscreen()} onChange={(event) => setVideoOutputFullscreen(event.currentTarget.checked)} />
                  Fullscreen
                </label>
              </div>
            </Show>
            <Show when={videoOutputKind() !== "Display"}>
              <label>
                Endpoint
                <input value={videoOutputEndpoint()} onInput={(event) => setVideoOutputEndpoint(event.currentTarget.value)} />
              </label>
            </Show>
            <button class="primary" onClick={addVideoOutput}>
              Add Output
            </button>
            <div class="timelineList">
              <For each={snapshot().video.outputs}>
                {(output) => {
                  const configDraft = () => videoOutputConfigDraft(output);
                  return (
                  <div class="timelineItem">
                    <strong>{output.label}</strong>
                    <span>
                      {output.kind} / {output.width}x{output.height} / C{output.composition_id}
                      {output.endpoint_name ? ` / ${output.endpoint_name}` : ""}
                    </span>
                    <div class="videoOutputConfig">
                      <h3>Output Config</h3>
                      <div class="split">
                        <label>
                          Label
                          <input
                            value={configDraft().label}
                            onInput={(event) => updateVideoOutputConfigDraft(output, { label: event.currentTarget.value })}
                          />
                        </label>
                        <label>
                          Kind
                          <select
                            value={configDraft().kind}
                            onInput={(event) =>
                              updateVideoOutputConfigDraft(output, {
                                kind: event.currentTarget.value as VideoOutputKind,
                              })
                            }
                          >
                            <option value="Display">Display</option>
                            <option value="NdiSender">NDI Sender</option>
                            <option value="SpoutSender">Spout Sender</option>
                            <option value="SyphonServer">Syphon Server</option>
                          </select>
                        </label>
                      </div>
                      <div class="split">
                        <label>
                          Width
                          <input
                            type="number"
                            min="1"
                            value={configDraft().width}
                            onInput={(event) => updateVideoOutputConfigDraft(output, { width: Number(event.currentTarget.value) })}
                          />
                        </label>
                        <label>
                          Height
                          <input
                            type="number"
                            min="1"
                            value={configDraft().height}
                            onInput={(event) => updateVideoOutputConfigDraft(output, { height: Number(event.currentTarget.value) })}
                          />
                        </label>
                      </div>
                      <Show when={configDraft().kind === "Display"}>
                        <div class="split">
                          <label>
                            Monitor
                            <input
                              type="number"
                              min="0"
                              value={configDraft().monitor_id}
                              onInput={(event) => updateVideoOutputConfigDraft(output, { monitor_id: Number(event.currentTarget.value) })}
                            />
                          </label>
                          <label class="checkbox inlineCheckbox">
                            <input
                              type="checkbox"
                              checked={configDraft().fullscreen}
                              onChange={(event) => updateVideoOutputConfigDraft(output, { fullscreen: event.currentTarget.checked })}
                            />
                            Fullscreen
                          </label>
                        </div>
                      </Show>
                      <Show when={configDraft().kind !== "Display"}>
                        <label>
                          Endpoint
                          <input
                            value={configDraft().endpoint_name}
                            onInput={(event) => updateVideoOutputConfigDraft(output, { endpoint_name: event.currentTarget.value })}
                          />
                        </label>
                      </Show>
                      <div class="buttonRow">
                        <button class="primary" onClick={() => void setVideoOutputConfig(output)}>
                          Apply Output
                        </button>
                      </div>
                    </div>
                    <label>
                      Route
                      <select
                        value={output.composition_id}
                        onInput={(event) => void setVideoOutputRouting(output.id, Number(event.currentTarget.value))}
                      >
                        <For each={snapshot().video.compositions}>
                          {(composition) => <option value={composition.id}>{composition.label}</option>}
                        </For>
                      </select>
                    </label>
                    <div class="videoOutputMapping">
                      <h3>Projector Map</h3>
                      <div class="split">
                        <label>
                          Preset Label
                          <input
                            value={videoOutputMappingPresetLabel()}
                            onInput={(event) => setVideoOutputMappingPresetLabel(event.currentTarget.value)}
                          />
                        </label>
                        <label>
                          Saved Preset
                          <select
                            value={selectedVideoOutputMappingPresetLabel()}
                            disabled={snapshot().video.mapping_presets.length === 0}
                            onInput={(event) => setSelectedVideoOutputMappingPresetLabel(event.currentTarget.value)}
                          >
                            <option value="">Select preset</option>
                            <For each={snapshot().video.mapping_presets}>
                              {(preset) => <option value={preset.label}>{preset.label}</option>}
                            </For>
                          </select>
                        </label>
                      </div>
                      <div class="presetRow">
                        <button class="primary" onClick={() => void saveVideoOutputMappingPreset(output.mapping)}>
                          Save Preset
                        </button>
                        <button onClick={() => void exportVideoOutputMappingPreset(output.mapping)}>
                          Export
                        </button>
                        <button onClick={() => void importVideoOutputMappingPreset()}>
                          Import
                        </button>
                        <button
                          disabled={!selectedVideoOutputMappingPresetLabel()}
                          onClick={() => void applyVideoOutputMappingPreset(output.id, selectedVideoOutputMappingPresetLabel())}
                        >
                          Apply Preset
                        </button>
                        <button
                          disabled={!selectedVideoOutputMappingPresetLabel()}
                          onClick={() => void removeVideoOutputMappingPreset(selectedVideoOutputMappingPresetLabel())}
                        >
                          Delete Preset
                        </button>
                        <button onClick={() => void setVideoOutputMapping(output.id, defaultVideoOutputMapping)}>
                          Reset
                        </button>
                        <button
                          onClick={() =>
                            void setVideoOutputMapping(output.id, {
                              ...output.mapping,
                              lens_distortion: 0,
                              keystone_x: 0,
                              keystone_y: 0,
                              corner_top_left_x: 0,
                              corner_top_left_y: 0,
                              corner_top_right_x: 0,
                              corner_top_right_y: 0,
                              corner_bottom_right_x: 0,
                              corner_bottom_right_y: 0,
                              corner_bottom_left_x: 0,
                              corner_bottom_left_y: 0,
                            })
                          }
                        >
                          Clear Warp
                        </button>
                        <button
                          onClick={() =>
                            void setVideoOutputMapping(output.id, {
                              ...output.mapping,
                              aspect_ratio: outputAspectRatio(output.width, output.height),
                              aspect_mode: "Fit",
                            })
                          }
                        >
                          Output Ratio
                        </button>
                        <button
                          onClick={() =>
                            void setVideoOutputMapping(output.id, {
                              ...output.mapping,
                              aspect_ratio: 16 / 9,
                              aspect_mode: "Fit",
                            })
                          }
                        >
                          16:9
                        </button>
                        <button
                          onClick={() =>
                            void setVideoOutputMapping(output.id, {
                              ...output.mapping,
                              aspect_ratio: 4 / 3,
                              aspect_mode: "Fit",
                            })
                          }
                        >
                          4:3
                        </button>
                        <button
                          onClick={() =>
                            void setVideoOutputMapping(output.id, {
                              ...output.mapping,
                              aspect_ratio: 1,
                              aspect_mode: "Fit",
                            })
                          }
                        >
                          1:1
                        </button>
                      </div>
                      <div class="projectorMapEditor">
                        <svg
                          class="projectorMapSurface"
                          viewBox={`0 0 ${projectorMapViewBoxSize} ${projectorMapViewBoxSize}`}
                          role="img"
                        >
                          <defs>
                            <pattern
                              id={`projector-map-grid-${output.id}`}
                              width="10"
                              height="10"
                              patternUnits="userSpaceOnUse"
                            >
                              <path d="M 10 0 L 0 0 0 10" />
                            </pattern>
                          </defs>
                          <rect class="projectorMapFloor" x="0" y="0" width="100" height="100" />
                          <rect
                            class="projectorMapGrid"
                            x="0"
                            y="0"
                            width="100"
                            height="100"
                            fill={`url(#projector-map-grid-${output.id})`}
                          />
                          <line class="projectorMapAxis" x1="50" y1="0" x2="50" y2="100" />
                          <line class="projectorMapAxis" x1="0" y1="50" x2="100" y2="50" />
                          <polygon class="projectorMapBase" points={projectorMapBasePoints(output.mapping).join(" ")} />
                          <polygon class="projectorMapWarp" points={projectorMapPoints(output.mapping).join(" ")} />
                          <For each={projectorCorners}>
                            {(corner) => {
                              const basePoint = () => projectorMapBasePoint(output.mapping, corner);
                              const point = () => projectorMapPoint(output.mapping, corner);
                              return (
                                <>
                                  <line
                                    class="projectorMapHandleGuide"
                                    x1={basePoint().x}
                                    y1={basePoint().y}
                                    x2={point().x}
                                    y2={point().y}
                                  />
                                  <circle class="projectorMapBasePoint" cx={basePoint().x} cy={basePoint().y} r="1.6" />
                                  <circle
                                    class="projectorMapHandle"
                                    cx={point().x}
                                    cy={point().y}
                                    r="4"
                                    tabIndex={0}
                                    onPointerDown={(event) => {
                                      event.currentTarget.setPointerCapture(event.pointerId);
                                      setProjectorCornerFromPointer(event, output, corner);
                                    }}
                                    onPointerMove={(event) => {
                                      if (event.buttons === 1) {
                                        setProjectorCornerFromPointer(event, output, corner);
                                      }
                                    }}
                                    onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                                  >
                                    <title>
                                      {corner.label} {output.label}
                                    </title>
                                  </circle>
                                  <text class="projectorMapHandleLabel" x={point().x + 5} y={point().y - 5}>
                                    {corner.label}
                                  </text>
                                </>
                              );
                            }}
                          </For>
                        </svg>
                      </div>
                      <div class="videoOutputPreviewCard">
                        <div class="sectionHeader">
                          <h4>Output Preview</h4>
                          <span>
                            {videoOutputPreviewId() === output.id
                              ? `${videoOutputPreviewMode() === "test" ? "Pattern" : "Output"} / ${videoOutputPreviewInfo()}`
                              : "No preview"}
                          </span>
                        </div>
                        <Show
                          when={videoOutputPreviewId() === output.id && videoOutputPreviewUrl()}
                          fallback={<span class="emptyState">No preview</span>}
                        >
                          {(url) => <img src={url()} alt={`${output.label} preview`} />}
                        </Show>
                      </div>
                      <div class="triple">
                        <label>
                          X
                          <input
                            type="number"
                            step="0.01"
                            value={output.mapping.offset_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                offset_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Y
                          <input
                            type="number"
                            step="0.01"
                            value={output.mapping.offset_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                offset_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Rotate
                          <input
                            type="number"
                            step="0.1"
                            value={output.mapping.rotation_deg}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                rotation_deg: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="triple">
                        <label>
                          Scale X
                          <input
                            type="number"
                            min="0.01"
                            step="0.01"
                            value={output.mapping.scale_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                scale_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Scale Y
                          <input
                            type="number"
                            min="0.01"
                            step="0.01"
                            value={output.mapping.scale_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                scale_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Aspect
                          <input
                            type="number"
                            min="0.1"
                            step="0.01"
                            value={output.mapping.aspect_ratio}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                aspect_ratio: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="split">
                        <label>
                          Aspect Mode
                          <select
                            value={output.mapping.aspect_mode}
                            onInput={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                aspect_mode: event.currentTarget.value as VideoOutputAspectMode,
                              })
                            }
                          >
                            <For each={videoOutputAspectModes}>{(mode) => <option value={mode}>{mode}</option>}</For>
                          </select>
                        </label>
                        <label>
                          Lens Distortion
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.lens_distortion}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                lens_distortion: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="split">
                        <label>
                          Keystone H
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.keystone_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                keystone_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Keystone V
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.keystone_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                keystone_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="quad">
                        <label>
                          TL X
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_top_left_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_top_left_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          TL Y
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_top_left_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_top_left_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          TR X
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_top_right_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_top_right_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          TR Y
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_top_right_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_top_right_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <div class="quad">
                        <label>
                          BR X
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_bottom_right_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_bottom_right_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          BR Y
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_bottom_right_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_bottom_right_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          BL X
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_bottom_left_x}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_bottom_left_x: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          BL Y
                          <input
                            type="number"
                            min="-1"
                            max="1"
                            step="0.01"
                            value={output.mapping.corner_bottom_left_y}
                            onChange={(event) =>
                              void setVideoOutputMapping(output.id, {
                                ...output.mapping,
                                corner_bottom_left_y: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                    </div>
                    <div class="buttonRow">
                      <button onClick={() => void setVideoOutputEnabled(output.id, !output.enabled)}>
                        {output.enabled ? "Disable" : "Enable"}
                      </button>
                      <button onClick={() => void setVideoOutputBlackout(output.id, !output.blackout)}>
                        {output.blackout ? "Clear" : "Blackout"}
                      </button>
                      <button onClick={() => void setVideoOutputOpacity(output.id, output.opacity >= 1 ? 0.5 : 1)}>
                        {output.opacity >= 1 ? "Half" : "Full"}
                      </button>
                      <button onClick={() => void fadeVideoOutputOpacity(output.id, 0)}>
                        Fade Out
                      </button>
                      <button onClick={() => void fadeVideoOutputOpacity(output.id, 1)}>
                        Fade In
                      </button>
                      <button onClick={() => void renderDebugVideoOutputPreview(output.id)}>
                        Preview
                      </button>
                      <button onClick={() => void renderDebugVideoOutputPreview(output.id, true)}>
                        Pattern Preview
                      </button>
                      <Show when={output.kind === "Display"}>
                        <button onClick={() => void openVideoOutputWindow(output.id)}>
                          Open Window
                        </button>
                        <button onClick={() => void openVideoOutputWindow(output.id, true)}>
                          Test Pattern
                        </button>
                      </Show>
                      <button onClick={() => void removeVideoOutput(output.id)}>Remove</button>
                    </div>
                  </div>
                  );
                }}
              </For>
            </div>
          </div>
        </section>

        <section class="panel videoControlPanel controlPanel">
          <div class="panelHeader">
            <h2>Video Control</h2>
            <span>{snapshot().video.layers.length} layer(s)</span>
          </div>
          <div class="previewDebug">
            <button onClick={renderDebugVideoPreview} disabled={snapshot().video.layers.length === 0}>
              CPU Preview
            </button>
            <button onClick={() => void refreshVideoPreviewDiagnostics()}>
              Preview Status
            </button>
            <span>{videoPreviewInfo()}</span>
            <small>{videoPreviewDiagnosticsText()}</small>
          </div>
          <div class="videoMasterControls">
            <label>
              Master
              <input
                type="number"
                min="0"
                max="1"
                step="0.01"
                value={snapshot().video.master_opacity}
                onChange={(event) => void setVideoMasterOpacity(Number(event.currentTarget.value))}
              />
            </label>
            <div class="buttonRow">
              <button onClick={() => setVideoBlackout(true)} disabled={snapshot().video.blackout}>
                V Blackout
              </button>
              <button onClick={() => setVideoBlackout(false)} disabled={!snapshot().video.blackout}>
                V Clear
              </button>
            </div>
          </div>
          <div class="videoOutputControlList">
            <div class="sectionHeader">
              <h3>Outputs</h3>
              <label>
                Fade ms
                <input
                  type="number"
                  min="0"
                  step="10"
                  value={videoOutputFadeMs()}
                  onInput={(event) => setVideoOutputFadeMs(Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <Show when={snapshot().video.outputs.length > 0} fallback={<span class="emptyState">No video outputs</span>}>
              <For each={snapshot().video.outputs}>
                {(output) => {
                  const compositionLabel = () =>
                    snapshot().video.compositions.find((composition) => composition.id === output.composition_id)?.label ??
                    `Composition ${output.composition_id}`;
                  const outputLive = () => output.enabled && !output.blackout && output.opacity > 0;
                  return (
                    <div class={outputLive() ? "videoOutputControlItem active" : "videoOutputControlItem"}>
                      <div>
                        <strong>{output.label}</strong>
                        <span>
                          {output.kind} / {compositionLabel()} / {Math.round(output.opacity * 100)}%
                          {!output.enabled ? " / Disabled" : ""}
                          {output.blackout ? " / Blackout" : ""}
                        </span>
                        <div class="outputOpacityMeter">
                          <span style={{ width: `${Math.round(output.opacity * 100)}%` }} />
                        </div>
                      </div>
                      <div class="buttonRow">
                        <button onClick={() => void setVideoOutputEnabled(output.id, !output.enabled)}>
                          {output.enabled ? "Disable" : "Enable"}
                        </button>
                        <button onClick={() => void setVideoOutputBlackout(output.id, !output.blackout)}>
                          {output.blackout ? "Clear" : "Blackout"}
                        </button>
                        <button onClick={() => void fadeVideoOutputOpacity(output.id, 0)}>
                          Fade Out
                        </button>
                        <button onClick={() => void fadeVideoOutputOpacity(output.id, 1)}>
                          Fade In
                        </button>
                        <button onClick={() => void setVideoOutputOpacity(output.id, 1)}>Full</button>
                      </div>
                    </div>
                  );
                }}
              </For>
            </Show>
          </div>
          <div class="videoPreview">
            <Show
              when={videoPreviewUrl()}
              fallback={<span>{snapshot().video.layers.length === 0 ? "Add a video layer" : "Render a CPU preview"}</span>}
            >
              {(url) => <img src={url()} alt="CPU video preview" />}
            </Show>
          </div>
          <label>
            Source
            <select
              value={videoSourceKind()}
              onInput={(event) => setVideoSourceKind(event.currentTarget.value as VideoSourceKind)}
            >
              <option value="File">Video file</option>
              <option value="StillImage">Still image</option>
              <option value="Ndi">NDI input</option>
              <option value="Spout">Spout input</option>
              <option value="Syphon">Syphon input</option>
            </select>
          </label>
          <label>
            Layer label
            <input value={videoLabel()} onInput={(event) => setVideoLabel(event.currentTarget.value)} />
          </label>
          <label>
            {videoSourceInputLabel(videoSourceKind())}
            <input
              value={videoPath()}
              placeholder={videoSourceInputPlaceholder(videoSourceKind())}
              onInput={(event) => setVideoPath(event.currentTarget.value)}
            />
          </label>
          <button onClick={selectVideoSourceFile} disabled={!videoSourceCanBrowseFile(videoSourceKind())}>
            Browse Source
          </button>
          <button class="primary" onClick={addVideoLayer}>
            Add Video Layer
          </button>
          <div class="videoLayerList">
            <For each={snapshot().video.layers}>
              {(layer, index) => (
                <div class="videoLayerItem">
                  <div>
                    <label>
                      Layer name
                      <input
                        value={layer.label}
                        onChange={(event) => void setVideoLayerLabel(layer.id, event.currentTarget.value)}
                      />
                    </label>
                    <span>{layer.source.path ?? layer.source.name ?? layer.source.kind}</span>
                    <Show when={videoSourceMetadataLabel(layer.source)}>
                      {(metadata) => <small>{metadata()}</small>}
                    </Show>
                  </div>
                  <div class="buttonRow">
                    <button onClick={() => void moveVideoLayer(layer.id, -1)} disabled={index() === 0}>
                      Up
                    </button>
                    <button
                      onClick={() => void moveVideoLayer(layer.id, 1)}
                      disabled={index() === snapshot().video.layers.length - 1}
                    >
                      Down
                    </button>
                    <button onClick={() => void duplicateVideoLayer(layer)}>Duplicate</button>
                  </div>
                  <label>
                    Blend
                    <select
                      value={layer.blend_mode}
                      onInput={(event) =>
                        void setVideoLayerBlendMode(layer.id, event.currentTarget.value as VideoBlendMode)
                      }
                    >
                      <option value="Normal">Normal</option>
                      <option value="Add">Add</option>
                      <option value="Multiply">Multiply</option>
                      <option value="Screen">Screen</option>
                    </select>
                  </label>
                  <label class="checkbox">
                    <input
                      type="checkbox"
                      checked={layer.state.enabled}
                      onChange={(event) =>
                        void setVideoLayerState(layer.id, {
                          ...layer.state,
                          enabled: event.currentTarget.checked,
                        })
                      }
                    />
                    Layer enabled
                  </label>
                  <label class="checkbox">
                    <input
                      type="checkbox"
                      checked={layer.state.solo}
                      onChange={(event) =>
                        void setVideoLayerState(layer.id, {
                          ...layer.state,
                          solo: event.currentTarget.checked,
                        })
                      }
                    />
                    Solo layer
                  </label>
                  <div class="split">
                    <label>
                      Opacity
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.01"
                        value={layer.state.opacity}
                        onInput={(event) =>
                          void setVideoLayerState(layer.id, {
                            ...layer.state,
                            opacity: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                    <label>
                      Speed
                      <input
                        type="number"
                        min="-4"
                        max="4"
                        step="0.1"
                        value={layer.state.speed}
                        onInput={(event) =>
                          void setVideoLayerState(layer.id, {
                            ...layer.state,
                            speed: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                  </div>
                  <div class="videoTransformControls">
                    <h3>Transform</h3>
                    <div class="split">
                      <label>
                        X
                        <input
                          type="number"
                          step="0.01"
                          value={layer.state.transform.x}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              x: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Y
                        <input
                          type="number"
                          step="0.01"
                          value={layer.state.transform.y}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              y: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <div class="split">
                      <label>
                        Scale X
                        <input
                          type="number"
                          min="0.001"
                          step="0.01"
                          value={layer.state.transform.scale_x}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              scale_x: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Scale Y
                        <input
                          type="number"
                          min="0.001"
                          step="0.01"
                          value={layer.state.transform.scale_y}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              scale_y: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <label>
                      Rotation
                      <input
                        type="number"
                        step="1"
                        value={layer.state.transform.rotation_deg}
                        onChange={(event) =>
                          void setVideoLayerTransform(layer.id, layer.state, {
                            rotation_deg: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                    <div class="split">
                      <label>
                        Crop L
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.transform.crop_left}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              crop_left: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Crop R
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.transform.crop_right}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              crop_right: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <div class="split">
                      <label>
                        Crop T
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.transform.crop_top}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              crop_top: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Crop B
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.transform.crop_bottom}
                          onChange={(event) =>
                            void setVideoLayerTransform(layer.id, layer.state, {
                              crop_bottom: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                  </div>
                  <div class="videoColorControls">
                    <h3>Color</h3>
                    <div class="split">
                      <label>
                        Bright
                        <input
                          type="number"
                          min="-1"
                          max="1"
                          step="0.01"
                          value={layer.state.color?.brightness ?? defaultColorAdjust.brightness}
                          onChange={(event) =>
                            void setVideoLayerColor(layer.id, layer.state, {
                              brightness: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Contrast
                        <input
                          type="number"
                          min="0"
                          max="4"
                          step="0.01"
                          value={layer.state.color?.contrast ?? defaultColorAdjust.contrast}
                          onChange={(event) =>
                            void setVideoLayerColor(layer.id, layer.state, {
                              contrast: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <div class="split">
                      <label>
                        Hue
                        <input
                          type="number"
                          step="1"
                          value={layer.state.color?.hue_deg ?? defaultColorAdjust.hue_deg}
                          onChange={(event) =>
                            void setVideoLayerColor(layer.id, layer.state, {
                              hue_deg: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Sat
                        <input
                          type="number"
                          min="0"
                          max="4"
                          step="0.01"
                          value={layer.state.color?.saturation ?? defaultColorAdjust.saturation}
                          onChange={(event) =>
                            void setVideoLayerColor(layer.id, layer.state, {
                              saturation: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <label>
                      Gamma
                      <input
                        type="number"
                        min="0.1"
                        max="4"
                        step="0.01"
                        value={layer.state.color?.gamma ?? defaultColorAdjust.gamma}
                        onChange={(event) =>
                          void setVideoLayerColor(layer.id, layer.state, {
                            gamma: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                  </div>
                  <div class="videoFxControls">
                    <h3>FX</h3>
                    <div class="split">
                      <label>
                        Pixelate
                        <input
                          type="number"
                          min="1"
                          max="128"
                          step="1"
                          value={layer.state.fx?.pixelate ?? defaultFxAdjust.pixelate}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              pixelate: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Blur
                        <input
                          type="number"
                          min="0"
                          max="8"
                          step="1"
                          value={layer.state.fx?.blur ?? defaultFxAdjust.blur}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              blur: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <div class="split">
                      <label>
                        Glow
                        <input
                          type="number"
                          min="0"
                          max="4"
                          step="0.01"
                          value={layer.state.fx?.glow ?? defaultFxAdjust.glow}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              glow: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Edge
                        <input
                          type="number"
                          min="0"
                          max="4"
                          step="0.01"
                          value={layer.state.fx?.edge ?? defaultFxAdjust.edge}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              edge: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <div class="triple">
                      <label>
                        Key R
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.fx?.key_red ?? defaultFxAdjust.key_red}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              key_red: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Key G
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.fx?.key_green ?? defaultFxAdjust.key_green}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              key_green: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Key B
                        <input
                          type="number"
                          min="0"
                          max="1"
                          step="0.01"
                          value={layer.state.fx?.key_blue ?? defaultFxAdjust.key_blue}
                          onChange={(event) =>
                            void setVideoLayerFx(layer.id, layer.state, {
                              key_blue: Number(event.currentTarget.value),
                            })
                          }
                        />
                      </label>
                    </div>
                    <label>
                      Key Threshold
                      <input
                        type="number"
                        min="0"
                        max="1"
                        step="0.01"
                        value={layer.state.fx?.key_threshold ?? defaultFxAdjust.key_threshold}
                        onChange={(event) =>
                          void setVideoLayerFx(layer.id, layer.state, {
                            key_threshold: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                  </div>
                  <label class="checkbox">
                    <input
                      type="checkbox"
                      checked={layer.state.playing}
                      onChange={(event) =>
                        void setVideoLayerState(layer.id, {
                          ...layer.state,
                          playing: event.currentTarget.checked,
                        })
                      }
                    />
                    Playing
                  </label>
                  <div class="split">
                    <label>
                      Position ms
                      <input
                        type="number"
                        min="0"
                        max={layer.source.metadata?.duration_ms ?? undefined}
                        value={layer.state.position_ms}
                        onChange={(event) =>
                          void setVideoLayerState(layer.id, {
                            ...layer.state,
                            position_ms: Number(event.currentTarget.value),
                          })
                        }
                      />
                    </label>
                    <label class="checkbox inlineCheckbox">
                      <input
                        type="checkbox"
                        checked={layer.state.loop_enabled}
                        onChange={(event) =>
                          void setVideoLayerState(layer.id, {
                            ...layer.state,
                            loop_enabled: event.currentTarget.checked,
                          })
                        }
                      />
                      Loop
                    </label>
                  </div>
                  <Show when={layer.state.loop_enabled}>
                    <>
                      <div class="split">
                        <label>
                          Loop in
                          <input
                            type="number"
                            min="0"
                            max={layer.source.metadata?.duration_ms ?? undefined}
                            value={layer.state.loop_start_ms}
                            onChange={(event) =>
                              void setVideoLayerState(layer.id, {
                                ...layer.state,
                                loop_start_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Loop out
                          <input
                            type="number"
                            min="0"
                            max={layer.source.metadata?.duration_ms ?? undefined}
                            value={layer.state.loop_end_ms}
                            onChange={(event) =>
                              void setVideoLayerState(layer.id, {
                                ...layer.state,
                                loop_end_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                      </div>
                      <Show when={layer.source.metadata?.duration_ms}>
                        {(durationMs) => (
                          <button
                            onClick={() =>
                              void setVideoLayerState(layer.id, {
                                ...layer.state,
                                loop_enabled: true,
                                loop_start_ms: 0,
                                loop_end_ms: durationMs(),
                              })
                            }
                          >
                            Use Source Length
                          </button>
                        )}
                      </Show>
                    </>
                  </Show>
                  <div class="videoSyncControls">
                    <label class="checkbox">
                      <input
                        type="checkbox"
                        checked={layer.state.bpm_sync.enabled}
                        onChange={(event) =>
                          void setVideoLayerState(layer.id, {
                            ...layer.state,
                            bpm_sync: {
                              ...layer.state.bpm_sync,
                              enabled: event.currentTarget.checked,
                            },
                          })
                        }
                      />
                      BPM sync
                    </label>
                    <div class="split">
                      <label>
                        Ratio
                        <select
                          value={layer.state.bpm_sync.ratio}
                          onInput={(event) =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              bpm_sync: {
                                ...layer.state.bpm_sync,
                                ratio: Number(event.currentTarget.value),
                              },
                            })
                          }
                        >
                          <option value="0.25">0.25x</option>
                          <option value="0.5">0.5x</option>
                          <option value="1">1x</option>
                          <option value="2">2x</option>
                          <option value="4">4x</option>
                        </select>
                      </label>
                      <label>
                        Bars
                        <input
                          type="number"
                          min="0.25"
                          step="0.25"
                          value={layer.state.bpm_sync.loop_bars}
                          onChange={(event) =>
                            void setVideoLayerState(layer.id, {
                              ...layer.state,
                              bpm_sync: {
                                ...layer.state.bpm_sync,
                                loop_bars: Number(event.currentTarget.value),
                              },
                            })
                          }
                        />
                      </label>
                    </div>
                  </div>
                  <div class="cuePointList">
                    <div class="buttonRow">
                      <button
                        onClick={() => void addVideoCuePoint(layer.id)}
                      >
                        Add Cue Pt
                      </button>
                      <button
                        onClick={() => void jumpVideoCuePoint(layer.id, 0)}
                        disabled={layer.state.cue_points_ms.length === 0}
                      >
                        Jump First
                      </button>
                    </div>
                    <For each={layer.state.cue_points_ms}>
                      {(cuePoint, cuePointIndex) => (
                        <div class="cuePointItem">
                          <button
                            onClick={() => void jumpVideoCuePoint(layer.id, cuePointIndex())}
                          >
                            {cuePoint} ms
                          </button>
                          <button
                            onClick={() => void removeVideoCuePoint(layer.id, cuePoint)}
                          >
                            Remove
                          </button>
                        </div>
                      )}
                    </For>
                  </div>
                  <div class="buttonRow">
                    <button
                      onClick={() =>
                        setVideoLayerState(layer.id, {
                          ...layer.state,
                          opacity: 0,
                        })
                      }
                    >
                      Fade Out
                    </button>
                    <button onClick={() => removeVideoLayer(layer.id)}>Remove</button>
                  </div>
                </div>
              )}
            </For>
          </div>
          <div class="videoAutomationForm">
            <h3>Timeline Automation</h3>
            <label>
              Layer
              <select
                value={selectedVideoAutomationLayerId() ?? ""}
                disabled={snapshot().video.layers.length === 0}
                onInput={(event) => setVideoAutomationLayerId(Number(event.currentTarget.value))}
              >
                <For each={snapshot().video.layers}>
                  {(layer) => <option value={layer.id}>{layer.label}</option>}
                </For>
              </select>
            </label>
            <div class="split">
              <label>
                Param
                <select
                  value={videoAutomationParam()}
                  onInput={(event) => setVideoAutomationParam(event.currentTarget.value as VideoParam)}
                >
                  <option value="Opacity">Opacity</option>
                  <option value="Speed">Speed</option>
                  <option value="PositionMs">Position</option>
                  <option value="BpmSyncEnabled">BPM Sync</option>
                  <option value="BpmSyncRatio">BPM Sync Ratio</option>
                  <option value="BpmSyncLoopBars">BPM Loop Bars</option>
                  <option value="TransformX">Transform X</option>
                  <option value="TransformY">Transform Y</option>
                  <option value="TransformScaleX">Scale X</option>
                  <option value="TransformScaleY">Scale Y</option>
                  <option value="TransformRotationDeg">Rotation</option>
                  <option value="TransformCropLeft">Crop L</option>
                  <option value="TransformCropTop">Crop T</option>
                  <option value="TransformCropRight">Crop R</option>
                  <option value="TransformCropBottom">Crop B</option>
                  <option value="ColorBrightness">Brightness</option>
                  <option value="ColorContrast">Contrast</option>
                  <option value="ColorHueDeg">Hue</option>
                  <option value="ColorSaturation">Saturation</option>
                  <option value="ColorGamma">Gamma</option>
                  <option value="FxPixelate">Pixelate</option>
                  <option value="FxBlur">Blur</option>
                  <option value="FxGlow">Glow</option>
                  <option value="FxEdge">Edge</option>
                  <option value="FxKeyRed">Key R</option>
                  <option value="FxKeyGreen">Key G</option>
                  <option value="FxKeyBlue">Key B</option>
                  <option value="FxKeyThreshold">Key Threshold</option>
                </select>
              </label>
              <label>
                Curve
                <select
                  value={videoAutomationInterpolation()}
                  onInput={(event) =>
                    setVideoAutomationInterpolation(event.currentTarget.value as AutomationInterpolation)
                  }
                >
                  <option value="Linear">Linear</option>
                  <option value="Step">Step</option>
                  <option value="Bezier">Bezier</option>
                </select>
              </label>
            </div>
            <div class="split">
              <label>
                Start
                <input
                  type="number"
                  min="0"
                  value={videoAutomationStartMs()}
                  onInput={(event) => setVideoAutomationStartMs(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                End
                <input
                  type="number"
                  min="0"
                  value={videoAutomationEndMs()}
                  onInput={(event) => setVideoAutomationEndMs(Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <div class="split">
              <label>
                From
                <input
                  type="number"
                  step="0.01"
                  value={videoAutomationStartValue()}
                  onInput={(event) => setVideoAutomationStartValue(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                To
                <input
                  type="number"
                  step="0.01"
                  value={videoAutomationEndValue()}
                  onInput={(event) => setVideoAutomationEndValue(Number(event.currentTarget.value))}
                />
              </label>
            </div>
            <button class="primary" onClick={addTimelineVideoAutomation} disabled={snapshot().video.layers.length === 0}>
              Add Video Automation
            </button>
            <div class="timelineList">
              <For each={timelineVideoAutomationRows()}>
                {(automation) => {
                  const draft = () => timelineVideoAutomationDraft(automation);
                  return (
                    <div class="timelineItem timelineAutomationItem">
                      <div>
                        <strong>{automation.layer_label} / {automation.param}</strong>
                        <span>
                          {automation.keyframes[0]?.time_ms ?? 0}-
                          {automation.keyframes[automation.keyframes.length - 1]?.time_ms ?? 0} ms / {automation.keyframes.length} keys
                        </span>
                      </div>
                      <div class="automationEditGrid">
                        <label>
                          Layer
                          <select
                            value={draft().layer_id}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                layer_id: Number(event.currentTarget.value),
                              })
                            }
                          >
                            <For each={snapshot().video.layers}>
                              {(layer) => <option value={layer.id}>{layer.label}</option>}
                            </For>
                          </select>
                        </label>
                        <label>
                          Param
                          <select
                            value={draft().param}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                param: event.currentTarget.value as VideoParam,
                              })
                            }
                          >
                            <option value="Opacity">Opacity</option>
                            <option value="Speed">Speed</option>
                            <option value="PositionMs">Position</option>
                            <option value="BpmSyncEnabled">BPM Sync</option>
                            <option value="BpmSyncRatio">BPM Sync Ratio</option>
                            <option value="BpmSyncLoopBars">BPM Loop Bars</option>
                            <option value="TransformX">Transform X</option>
                            <option value="TransformY">Transform Y</option>
                            <option value="TransformScaleX">Scale X</option>
                            <option value="TransformScaleY">Scale Y</option>
                            <option value="TransformRotationDeg">Rotation</option>
                            <option value="TransformCropLeft">Crop L</option>
                            <option value="TransformCropTop">Crop T</option>
                            <option value="TransformCropRight">Crop R</option>
                            <option value="TransformCropBottom">Crop B</option>
                            <option value="ColorBrightness">Brightness</option>
                            <option value="ColorContrast">Contrast</option>
                            <option value="ColorHueDeg">Hue</option>
                            <option value="ColorSaturation">Saturation</option>
                            <option value="ColorGamma">Gamma</option>
                            <option value="FxPixelate">Pixelate</option>
                            <option value="FxBlur">Blur</option>
                            <option value="FxGlow">Glow</option>
                            <option value="FxEdge">Edge</option>
                            <option value="FxKeyRed">Key R</option>
                            <option value="FxKeyGreen">Key G</option>
                            <option value="FxKeyBlue">Key B</option>
                            <option value="FxKeyThreshold">Key Threshold</option>
                          </select>
                        </label>
                        <label>
                          Start
                          <input
                            type="number"
                            min="0"
                            value={draft().start_ms}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                start_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          End
                          <input
                            type="number"
                            min="0"
                            value={draft().end_ms}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                end_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          From
                          <input
                            type="number"
                            step="0.01"
                            value={draft().start_value}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                start_value: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          To
                          <input
                            type="number"
                            step="0.01"
                            value={draft().end_value}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                end_value: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Curve
                          <select
                            value={draft().interpolation}
                            onInput={(event) =>
                              updateTimelineVideoAutomationDraft(automation, {
                                interpolation: event.currentTarget.value as AutomationInterpolation,
                              })
                            }
                          >
                            <option value="Linear">Linear</option>
                            <option value="Step">Step</option>
                            <option value="Bezier">Bezier</option>
                          </select>
                        </label>
                      </div>
                      <div class="buttonRow">
                        <button onClick={() => void setTimelineVideoAutomation(automation)}>Save</button>
                        <button onClick={() => removeTimelineAutomation(automation.id)}>Remove</button>
                      </div>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
        </section>

        <section class="panel faders controlPanel">
          <div class="panelHeader">
            <h2>Faders</h2>
            <span>{selectedFixtureGroupFilter() ? `Group ${selectedFixtureGroupFilter()}` : selectedFixture()?.label}</span>
          </div>
          <Show when={selectedFixtureGroupFilter()}>
            {(groupId) => (
              <div class="groupControlBanner">
                <div>
                  <strong>{groupId()}</strong>
                  <span>{filteredFixtures().length} fixture(s)</span>
                </div>
                <label class="groupSubmasterControl">
                  Submaster
                  <input
                    type="range"
                    min="0"
                    max="1"
                    step="0.01"
                    value={selectedGroupSubmaster()?.level ?? 1}
                    onChange={(event) => void setGroupSubmaster(groupId(), Number(event.currentTarget.value))}
                  />
                  <span>{Math.round((selectedGroupSubmaster()?.level ?? 1) * 100)}%</span>
                </label>
              </div>
            )}
          </Show>
          <Show when={selectedFixture()}>
            {(fixture) => (
              <>
                <div class="buttonRow">
                  <button onClick={savePreset}>Save Preset</button>
                  <button onClick={loadPreset}>Load Preset</button>
                  <button onClick={() => void duplicateFixture(fixture())}>Duplicate</button>
                  <button onClick={() => void removeFixture(fixture().id)}>Remove Fixture</button>
                </div>
                <div class="buttonRow">
                  <Show
                    when={selectedFixtureGroupFilter()}
                    fallback={
                      <>
                        <button onClick={() => void setFixtureHighlight(fixture().id, !fixture().highlighted)}>
                          {fixture().highlighted ? "Clear Highlight" : "Highlight"}
                        </button>
                        <button onClick={() => void setFixtureSolo(fixture().id, !fixture().soloed)}>
                          {fixture().soloed ? "Clear Solo" : "Solo"}
                        </button>
                        <button onClick={() => void setFixturePark(fixture().id, !fixture().parked)}>
                          {fixture().parked ? "Clear Park" : "Park"}
                        </button>
                      </>
                    }
                  >
                    {(groupId) => (
                      <>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupHighlight(groupId(), !selectedGroupFlagState().anyHighlighted)}
                        >
                          {selectedGroupFlagState().anyHighlighted ? "Clear Group Highlight" : "Group Highlight"}
                        </button>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupSolo(groupId(), !selectedGroupFlagState().anySoloed)}
                        >
                          {selectedGroupFlagState().anySoloed ? "Clear Group Solo" : "Group Solo"}
                        </button>
                        <button
                          disabled={selectedGroupFlagState().count === 0}
                          onClick={() => void setGroupPark(groupId(), !selectedGroupFlagState().anyParked)}
                        >
                          {selectedGroupFlagState().anyParked ? "Clear Group Park" : "Group Park"}
                        </button>
                      </>
                    )}
                  </Show>
                </div>
                <div class="transformEditor">
                  <strong>Transform</strong>
                  <div class="triple">
                    <label>
                      X
                      <input
                        type="number"
                        value={fixture().position.x}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, x: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Y
                      <input
                        type="number"
                        value={fixture().position.y}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, y: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Z
                      <input
                        type="number"
                        value={fixture().position.z}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            position: { ...fixture().position, z: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                  <div class="triple">
                    <label>
                      Pitch
                      <input
                        type="number"
                        value={fixture().rotation.pitch}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, pitch: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Yaw
                      <input
                        type="number"
                        value={fixture().rotation.yaw}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, yaw: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                    <label>
                      Roll
                      <input
                        type="number"
                        value={fixture().rotation.roll}
                        onInput={(event) =>
                          void setFixtureTransform(fixture(), {
                            rotation: { ...fixture().rotation, roll: Number(event.currentTarget.value) },
                          })
                        }
                      />
                    </label>
                  </div>
                </div>
              </>
            )}
          </Show>
          <div class="attributeEditor">
            <nav class="attributeCategoryRail" aria-label="Attribute category">
              <For each={controlCategoryRows()}>
                {(category) => (
                  <button
                    class={activeControlCategory() === category.id ? "active" : ""}
                    disabled={!category.hasVisual && category.count === 0}
                    onClick={() => setControlCategory(category.id)}
                    aria-pressed={activeControlCategory() === category.id}
                  >
                    <span>{category.label}</span>
                    <small>{category.count}</small>
                  </button>
                )}
              </For>
            </nav>
            <div class="attributeEditorBody">
          <Show when={showDimmerPanel() ? selectedDimmerControl() : undefined}>
            {(dimmerControl) => (
              <div class="visualControlPanel dimmerControlPanel">
                <div class="visualControlHeader">
                  <div>
                    <strong>Dimmer</strong>
                    <span>{Math.round((dimmerControl().value / 65535) * 1000) / 10}%</span>
                  </div>
                  <span>{dimmerControl().attribute}</span>
                </div>
                <div class="dimmerQuickRow">
                  <button onClick={() => setDimmerValue(0)}>Out</button>
                  <button onClick={() => setDimmerValue(32768)}>Half</button>
                  <button class="primary" onClick={() => setDimmerValue(65535)}>Full</button>
                </div>
                <label class="dimmerVisualSlider">
                  Level
                  <input
                    type="range"
                    min={normalizeLimitRange(selectedFixtureLimits().dimmer_min, selectedFixtureLimits().dimmer_max).min}
                    max={normalizeLimitRange(selectedFixtureLimits().dimmer_min, selectedFixtureLimits().dimmer_max).max}
                    value={dimmerControl().value}
                    onInput={(event) => setDimmerValue(Number(event.currentTarget.value))}
                  />
                  <span>{dimmerControl().value}</span>
                </label>
              </div>
            )}
          </Show>
          <Show when={showPositionPad() ? selectedPositionControls() : undefined}>
            {(positionControls) => (
              <div class="visualControlPanel">
                <div class="visualControlHeader">
                  <div>
                    <strong>Position</strong>
                    <span>
                      Pan {Math.round((positionControls().panValue / 65535) * 1000) / 10}% / Tilt{" "}
                      {Math.round((positionControls().tiltValue / 65535) * 1000) / 10}%
                    </span>
                  </div>
                  <span>{positionControls().pan} / {positionControls().tilt}</span>
                </div>
                <div class="positionNudgeGrid">
                  <span />
                  <button onClick={() => nudgePanTilt(0, panTiltNudgeAmount())}>Tilt +</button>
                  <span />
                  <button onClick={() => nudgePanTilt(-panTiltNudgeAmount(), 0)}>Pan -</button>
                  <button class="primary" onClick={centerPanTilt}>Center</button>
                  <button onClick={() => nudgePanTilt(panTiltNudgeAmount(), 0)}>Pan +</button>
                  <span />
                  <button onClick={() => nudgePanTilt(0, -panTiltNudgeAmount())}>Tilt -</button>
                  <span />
                </div>
                <div class="visualNumberGrid positionDirectGrid">
                  <label>
                    Pan %
                    <input
                      type="number"
                      min="0"
                      max="100"
                      step="0.1"
                      value={dmxValueToPercent(positionControls().panValue)}
                      onInput={(event) => setPanTiltPercent("pan", Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    Tilt %
                    <input
                      type="number"
                      min="0"
                      max="100"
                      step="0.1"
                      value={dmxValueToPercent(positionControls().tiltValue)}
                      onInput={(event) => setPanTiltPercent("tilt", Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    Nudge
                    <select
                      value={panTiltNudgeAmount()}
                      onChange={(event) => setPanTiltNudgeAmount(Number(event.currentTarget.value))}
                    >
                      <For each={panTiltNudgeSteps}>
                        {(step) => <option value={step.value}>{step.label}</option>}
                      </For>
                    </select>
                  </label>
                </div>
                <div class="positionFavoritePanel">
                  <div class="swatchHeader">
                    <strong>Position Favorites</strong>
                    <div class="miniButtonRow">
                      <button onClick={addCurrentPositionFavorite}>+</button>
                      <button onClick={resetPositionFavorites}>Reset</button>
                    </div>
                  </div>
                  <div class="positionFavoriteGrid">
                    <For each={positionFavorites()}>
                      {(favorite) => (
                        <button
                          class={
                            favorite.pan === clampDmxValue(positionControls().panValue) &&
                            favorite.tilt === clampDmxValue(positionControls().tiltValue)
                              ? "positionFavorite active"
                              : "positionFavorite"
                          }
                          title={`${favorite.label}: Pan ${formatShortDmxPercent(favorite.pan)} / Tilt ${formatShortDmxPercent(favorite.tilt)}. Shift-click removes.`}
                          onClick={(event) => {
                            if (event.shiftKey) {
                              removePositionFavorite(favorite.id);
                            } else {
                              void setPanTiltValues(favorite.pan, favorite.tilt);
                            }
                          }}
                        >
                          <span class="positionFavoriteMap">
                            <i
                              style={{
                                left: `${(favorite.pan / 65_535) * 100}%`,
                                top: `${100 - (favorite.tilt / 65_535) * 100}%`,
                              }}
                            />
                          </span>
                          <strong>{favorite.label}</strong>
                          <small>
                            {formatShortDmxPercent(favorite.pan)} / {formatShortDmxPercent(favorite.tilt)}
                          </small>
                        </button>
                      )}
                    </For>
                  </div>
                </div>
                <div class="positionPadRow">
                  <div
                    class="panTiltPad"
                    role="slider"
                    aria-label="Pan tilt pad"
                    aria-valuetext={`Pan ${positionControls().panValue}, Tilt ${positionControls().tiltValue}`}
                    onPointerDown={(event) => {
                      event.currentTarget.setPointerCapture(event.pointerId);
                      void setPanTiltFromPointer(event);
                    }}
                    onPointerMove={(event) => {
                      if (event.buttons === 1) {
                        void setPanTiltFromPointer(event);
                      }
                    }}
                    onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  >
                    <b class="panTiltLimitWindow" style={selectedFixtureLimitOverlayStyle()} />
                    <i
                      style={{
                        left: `${(positionControls().panValue / 65535) * 100}%`,
                        top: `${100 - (positionControls().tiltValue / 65535) * 100}%`,
                      }}
                    />
                  </div>
                  <div class="axisSliderRack">
                    <label class="axisSlider">
                      <span>Pan</span>
                      <input
                        type="range"
                        min={normalizeLimitRange(selectedFixtureLimits().pan_min, selectedFixtureLimits().pan_max).min}
                        max={normalizeLimitRange(selectedFixtureLimits().pan_min, selectedFixtureLimits().pan_max).max}
                        value={positionControls().panValue}
                        onInput={(event) => void setPanTiltValues(Number(event.currentTarget.value), positionControls().tiltValue)}
                      />
                      <small>{formatShortDmxPercent(positionControls().panValue)}</small>
                    </label>
                    <label class="axisSlider">
                      <span>Tilt</span>
                      <input
                        type="range"
                        min={normalizeLimitRange(selectedFixtureLimits().tilt_min, selectedFixtureLimits().tilt_max).min}
                        max={normalizeLimitRange(selectedFixtureLimits().tilt_min, selectedFixtureLimits().tilt_max).max}
                        value={positionControls().tiltValue}
                        onInput={(event) => void setPanTiltValues(positionControls().panValue, Number(event.currentTarget.value))}
                      />
                      <small>{formatShortDmxPercent(positionControls().tiltValue)}</small>
                    </label>
                  </div>
                </div>
              </div>
            )}
          </Show>
          <Show when={showColorPad() ? selectedColorControls() : undefined}>
            {(colorControls) => (
              <div class="visualControlPanel colorControlPanel">
                <div class="visualControlHeader">
                  <div>
                    <strong>Color</strong>
                    <span>
                      Hue {Math.round(selectedColorHsv().hue)} deg / Sat{" "}
                      {Math.round(selectedColorHsv().saturation * 100)}%
                    </span>
                  </div>
                  <span>{colorControls().red} / {colorControls().green} / {colorControls().blue}</span>
                </div>
                <div class="colorPickerRow">
                  <div
                    class="colorPlane"
                    role="slider"
                    aria-label="Hue and brightness pad"
                    aria-valuetext={colorControls().value}
                    onPointerDown={(event) => {
                      event.currentTarget.setPointerCapture(event.pointerId);
                      setColorFromPointer(event);
                    }}
                    onPointerMove={(event) => {
                      if (event.buttons === 1) {
                        setColorFromPointer(event);
                      }
                    }}
                    onPointerUp={(event) => event.currentTarget.releasePointerCapture(event.pointerId)}
                  >
                    <i
                      style={{
                        left: `${(selectedColorHsv().hue / 360) * 100}%`,
                        top: `${(1 - selectedColorHsv().value) * 100}%`,
                      }}
                    />
                  </div>
                  <div class="colorPickerSide">
                    <label>
                      Color
                      <input
                        type="color"
                        value={colorControls().value}
                        onInput={(event) => void setFixtureColor(event.currentTarget.value)}
                      />
                    </label>
                    <div class="rgbChannelRack">
                      <label class="rgbChannelSlider red">
                        <span>Red</span>
                        <input
                          type="range"
                          min="0"
                          max="65535"
                          value={selectedColorChannelValues().red}
                          onInput={(event) => setColorChannelValue("red", Number(event.currentTarget.value))}
                        />
                        <small>{formatShortDmxPercent(selectedColorChannelValues().red)}</small>
                      </label>
                      <label class="rgbChannelSlider green">
                        <span>Green</span>
                        <input
                          type="range"
                          min="0"
                          max="65535"
                          value={selectedColorChannelValues().green}
                          onInput={(event) => setColorChannelValue("green", Number(event.currentTarget.value))}
                        />
                        <small>{formatShortDmxPercent(selectedColorChannelValues().green)}</small>
                      </label>
                      <label class="rgbChannelSlider blue">
                        <span>Blue</span>
                        <input
                          type="range"
                          min="0"
                          max="65535"
                          value={selectedColorChannelValues().blue}
                          onInput={(event) => setColorChannelValue("blue", Number(event.currentTarget.value))}
                        />
                        <small>{formatShortDmxPercent(selectedColorChannelValues().blue)}</small>
                      </label>
                    </div>
                  </div>
                </div>
                <div class="visualNumberGrid hsvDirectGrid">
                  <label>
                    Hue
                    <input
                      type="number"
                      min="0"
                      max="360"
                      step="1"
                      value={Math.round(selectedColorHsv().hue)}
                      onInput={(event) => setColorHsvValue({ hue: Number(event.currentTarget.value) })}
                    />
                  </label>
                  <label>
                    Sat %
                    <input
                      type="number"
                      min="0"
                      max="100"
                      step="1"
                      value={Math.round(selectedColorHsv().saturation * 100)}
                      onInput={(event) =>
                        setColorHsvValue({ saturation: clampRange(Number(event.currentTarget.value), 0, 100) / 100 })
                      }
                    />
                  </label>
                  <label>
                    Val %
                    <input
                      type="number"
                      min="0"
                      max="100"
                      step="1"
                      value={Math.round(selectedColorHsv().value * 100)}
                      onInput={(event) =>
                        setColorHsvValue({ value: clampRange(Number(event.currentTarget.value), 0, 100) / 100 })
                      }
                    />
                  </label>
                </div>
                <div class="colorSwatchPanel">
                  <div class="swatchSection">
                    <strong>Palette</strong>
                    <div class="colorSwatchGrid">
                      <For each={defaultColorPalette}>
                        {(color) => (
                          <button
                            class={normalizeHexColor(color) === selectedColorHex() ? "colorSwatch active" : "colorSwatch"}
                            style={{ "background-color": color }}
                            title={color}
                            onClick={() => void setFixtureColor(color)}
                            aria-label={`Set color ${color}`}
                          />
                        )}
                      </For>
                    </div>
                  </div>
                  <div class="swatchSection">
                    <div class="swatchHeader">
                      <strong>Favorites</strong>
                      <div class="miniButtonRow">
                        <button onClick={addCurrentColorFavorite}>+</button>
                        <button onClick={resetColorFavorites}>Reset</button>
                      </div>
                    </div>
                    <div class="colorSwatchGrid">
                      <For each={colorFavorites()}>
                        {(color) => (
                          <button
                            class={normalizeHexColor(color) === selectedColorHex() ? "colorSwatch favorite active" : "colorSwatch favorite"}
                            style={{ "background-color": color }}
                            title={`${color} / Shift-click removes`}
                            onClick={(event) => {
                              if (event.shiftKey) {
                                removeColorFavorite(color);
                              } else {
                                void setFixtureColor(color);
                              }
                            }}
                            aria-label={`Favorite color ${color}`}
                          />
                        )}
                      </For>
                    </div>
                  </div>
                </div>
              </div>
            )}
          </Show>
          <Show when={showCategoryQuickPanel()}>
            <div class="visualControlPanel categoryQuickPanel">
              <div class="visualControlHeader">
                <div>
                  <strong>Category Actions</strong>
                  <span>{visibleControls().length} attribute(s)</span>
                </div>
                <span>{activeControlCategory()}</span>
              </div>
              <div class="categoryQuickRow">
                <button onClick={() => void applyVisibleControlValues("zero")}>Zero</button>
                <button onClick={() => void applyVisibleControlValues("mid")}>Mid</button>
                <button onClick={() => void applyVisibleControlValues("full")}>Full</button>
                <button class="primary" onClick={() => void applyVisibleControlValues("default")}>Default</button>
              </div>
              <Show when={categoryQuickLooks().length > 0}>
                <div class="categoryLookGrid">
                  <For each={categoryQuickLooks()}>
                    {(look) => (
                      <button title={look.description} onClick={() => void applyCategoryQuickLook(look)}>
                        <strong>{look.label}</strong>
                        <small>{look.description}</small>
                      </button>
                    )}
                  </For>
                </div>
              </Show>
            </div>
          </Show>
          <Show when={visibleFunctionControls().length > 0}>
            <div class="visualControlPanel channelFunctionPanel">
              <div class="visualControlHeader">
                <div>
                  <strong>GDTF Functions</strong>
                  <span>{visibleFunctionControls().length} attribute(s) with function ranges</span>
                </div>
                <span>{activeControlCategory()}</span>
              </div>
              <div class="channelFunctionList">
                <For each={visibleFunctionControls()}>
                  {(entry) => (
                    <div class="channelFunctionGroup">
                      <div class="channelFunctionTitle">
                        <strong>{entry.control.attribute}</strong>
                        <small>{entry.control.channel_name}</small>
                      </div>
                      <div class="channelFunctionGrid">
                        <For each={entry.functions}>
                          {(fn) => (
                            <button
                              title={`${channelFunctionLabel(fn)} / ${channelFunctionRangeLabel(fn)} / ${channelFunctionDetail(fn)}`}
                              onClick={() => void applyChannelFunction(entry.control, fn)}
                            >
                              <strong>{channelFunctionLabel(fn)}</strong>
                              <span>{channelFunctionRangeLabel(fn)}</span>
                              <small>
                                <Show when={normalizeHexColor(fn.wheel_slot_color)}>
                                  {(color) => <i class="channelFunctionSwatch" style={{ "background-color": color() }} />}
                                </Show>
                                {channelFunctionDetail(fn)}
                              </small>
                            </button>
                          )}
                        </For>
                      </div>
                    </div>
                  )}
                </For>
              </div>
            </div>
          </Show>
          <div class="faderGrid">
            <For each={visibleControls()}>
              {(control) => (
                <div class="fader">
                  <input
                    type="range"
                    min="0"
                    max="65535"
                    value={
                      selectedFixture()
                        ? faderValue(selectedFixture()!.id, control.attribute, control.default_value)
                        : control.default_value
                    }
                    onInput={(event) => {
                      const fixture = selectedFixture();
                      const groupId = selectedFixtureGroupFilter();
                      if (fixture) {
                        const value = Number(event.currentTarget.value);
                        if (groupId) {
                          void setGroupAttribute(groupId, control.attribute, value);
                        } else {
                          void setAttribute(fixture.id, control.attribute, value);
                        }
                      }
                    }}
                  />
                  <strong>{control.attribute}</strong>
                  <span>{control.resolution === "SixteenBit" ? "16-bit" : "8-bit"}</span>
                  <small>{control.offsets.join(", ")}</small>
                </div>
              )}
            </For>
            <Show when={!selectedFixture()}>
              <p class="empty">Select or patch a fixture.</p>
            </Show>
            <Show when={selectedFixture() && visibleControls().length === 0}>
              <p class="empty">No controls in this category.</p>
            </Show>
          </div>
          </div>
          </div>
          <div class="cuePanel">
            <div class="panelHeader">
              <h2>Cues</h2>
              <span>{snapshot().cues.length}</span>
            </div>
            <div class="cueForm">
              <label>
                Label
                <input value={cueLabel()} onInput={(event) => setCueLabel(event.currentTarget.value)} />
              </label>
              <label>
                Fade ms
                <input
                  type="number"
                  min="0"
                  value={cueFadeMs()}
                  onInput={(event) => setCueFadeMs(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Scope
                <select
                  value={cueCaptureScope()}
                  onInput={(event) => setCueCaptureScope(event.currentTarget.value as CueCaptureScopeMode)}
                >
                  <option value="all">Lighting + Video</option>
                  <option value="lighting">Lighting Only</option>
                  <option value="selectedFixture">Selected Fixture</option>
                  <option value="selectedGroup">Selected Group</option>
                  <option value="video">Video Only</option>
                </select>
              </label>
              <Show when={cueCaptureScopeError()}>
                {(error) => <span class="cueScopeHint invalid">{error()}</span>}
              </Show>
              <button class="primary" onClick={createCue} disabled={!hasCueSources() || Boolean(cueCaptureScopeError())}>
                Store Cue
              </button>
            </div>
            <CueCapturePreviewPanel
              preview={cueCapturePreview()}
              invalid={Boolean(cueCaptureScopeError())}
              stageViewBoxSize={stageViewBoxSize}
              stageOrigin={stageOrigin2d()}
              selectedFixtureId={selectedFixtureId()}
              onSelectFixture={setSelectedFixtureId}
            />
            <div class="buttonRow">
              <button onClick={triggerPreviousCue} disabled={snapshot().cues.length === 0}>
                Back
              </button>
              <button class="primary" onClick={triggerNextCue} disabled={snapshot().cues.length === 0}>
                GO
              </button>
              <button
                onClick={() => void setCueFadePaused(!snapshot().active_fade?.paused)}
                disabled={!snapshot().active_fade}
              >
                {snapshot().active_fade?.paused ? "Resume" : "Pause"}
              </button>
            </div>
            <Show when={snapshot().active_fade}>
              {(fade) => (
                <div class="fadeMeter">
                  <span>{fade().paused ? "Paused " : ""}{Math.round(fade().progress * 100)}%</span>
                  <div>
                    <i style={{ width: `${Math.round(fade().progress * 100)}%` }} />
                  </div>
                </div>
              )}
            </Show>
            <div class="cueList">
              <For each={snapshot().cues}>
                {(cue, index) => {
                  const draft = () => cueMetadataDraft(cue);
                  return (
                    <div class={cue.id === snapshot().active_cue_id ? "cueItem active" : "cueItem"}>
                      <div class="cueMetaLine">
                        <strong>{cue.label}</strong>
                        <span>
                          {cue.targets.length} fixture(s) / {cue.video_targets.length} video /{" "}
                          {cue.video_output_targets.length} output(s) / {cue.fade_ms}ms
                        </span>
                      </div>
                      <div class="cueEditRow">
                        <input
                          value={draft().label}
                          onInput={(event) => updateCueMetadataDraft(cue, { label: event.currentTarget.value })}
                        />
                        <input
                          type="number"
                          min="0"
                          value={draft().fade_ms}
                          onInput={(event) => updateCueMetadataDraft(cue, { fade_ms: Number(event.currentTarget.value) })}
                        />
                      </div>
                      <div class="cueActionRow">
                        <button onClick={() => void moveCue(cue.id, -1)} disabled={index() === 0}>
                          Up
                        </button>
                        <button onClick={() => void moveCue(cue.id, 1)} disabled={index() === snapshot().cues.length - 1}>
                          Down
                        </button>
                        <button onClick={() => void setCueMetadata(cue)}>
                          Save
                        </button>
                        <button onClick={() => void duplicateCue(cue)}>
                          Copy
                        </button>
                        <button
                          onClick={() => updateCue(cue.id, draft().label, draft().fade_ms)}
                          disabled={!hasCueSources() || Boolean(cueCaptureScopeError())}
                        >
                          Update
                        </button>
                        <button onClick={() => triggerCue(cue.id)}>GO</button>
                        <button
                          onClick={() => void addTimelineCueEventAt(cue.id, snapshot().timeline.position_ms, timelineTrack(), false)}
                        >
                          At Playhead
                        </button>
                        <button onClick={() => removeCue(cue.id)}>Remove</button>
                      </div>
                      <Show when={cueTimelinePlacementsForCue(cue.id).length > 0}>
                        <div class="cueTimelinePlacements">
                          <For each={cueTimelinePlacementsForCue(cue.id)}>
                            {(placement) => (
                              <span
                                class={[
                                  "cueTimelinePlacementChip",
                                  placement.track === "Lighting" ? "lighting" : "video",
                                  placement.time_ms < snapshot().timeline.position_ms ? "past" : "",
                                ].filter(Boolean).join(" ")}
                              >
                                <button
                                  class="cueTimelinePlacementMain"
                                  title={`Seek to ${placement.time_ms} ms / ${placement.track}`}
                                  onClick={() => void seekTimeline(placement.time_ms)}
                                >
                                  <b>{placement.track === "Lighting" ? "L" : "V"}</b>
                                  <small>{placement.time_ms} ms</small>
                                </button>
                                <button
                                  class="cueTimelinePlacementMove"
                                  title={`Nudge ${timelinePlacementNudgeMs()} ms. Shift-click nudges left.`}
                                  onClick={(event) =>
                                    void moveTimelineCueEvent(
                                      placement,
                                      event.shiftKey ? -timelinePlacementNudgeMs() : timelinePlacementNudgeMs(),
                                    )
                                  }
                                >
                                  &gt;
                                </button>
                                <button
                                  class="cueTimelinePlacementRemove"
                                  title="Remove timeline placement"
                                  onClick={() => removeTimelineEvent(placement.id)}
                                >
                                  x
                                </button>
                              </span>
                            )}
                          </For>
                        </div>
                      </Show>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
          <div class="timelinePanel">
            <div class="panelHeader">
              <h2>Timeline</h2>
              <span>
                {snapshot().timeline.position_ms} / {snapshot().timeline.duration_ms} ms
              </span>
            </div>
            <div class="timelineTransport">
              <button onClick={() => seekTimeline(0)}>|&lt;</button>
              <button onClick={pauseTimeline} disabled={!snapshot().timeline.playing}>
                Pause
              </button>
              <button class="primary" onClick={playTimeline} disabled={snapshot().timeline.duration_ms === 0 || snapshot().timeline.playing}>
                Play
              </button>
            </div>
            <input
              type="range"
              min="0"
              max={Math.max(snapshot().timeline.duration_ms, 1)}
              value={snapshot().timeline.position_ms}
              onInput={(event) => void seekTimeline(Number(event.currentTarget.value))}
            />
            <TimelineOverview
              events={timelineOverviewEvents()}
              playheadX={timelineOverviewPlayheadX()}
              onSeekRatio={seekTimelineFromOverviewRatio}
              onSeekTime={(timeMs) => void seekTimeline(timeMs)}
              onMoveEventRatio={(eventId, ratio) => void moveTimelineCueEventToRatio(eventId, ratio)}
            />
            <div class="audioAnalysisPanel">
              <div class="panelHeader">
                <h3>Audio</h3>
                <div class="buttonRow">
                  <button onClick={analyzeAudioFile}>Analyze WAV</button>
                  <button onClick={clearTimelineAudio} disabled={!audioAnalysis()}>
                    Clear
                  </button>
                </div>
              </div>
              <Show when={audioAnalysis()}>
                {(analysis) => (
                  <>
                    <div class="audioStats">
                      <span>{Math.round(analysis().duration_ms / 1000)}s</span>
                      <span>{analysis().sample_rate} Hz</span>
                      <span>{analysis().channels} ch</span>
                      <span>{analysis().estimated_bpm ? `${analysis().estimated_bpm!.toFixed(1)} BPM` : "BPM n/a"}</span>
                      <button onClick={applyAudioBpm} disabled={!analysis().estimated_bpm}>
                        Apply BPM
                      </button>
                    </div>
                    <svg class="waveformView" viewBox="0 0 100 36" role="img">
                      <rect x="0" y="0" width="100" height="36" />
                      <polyline points={audioWaveformPoints()} />
                      <For each={audioBeatMarkers()}>
                        {(beat) => <line class="beatMarker" x1={beat.x} x2={beat.x} y1="0" y2="36" />}
                      </For>
                    </svg>
                    <small>{analysis().path}</small>
                  </>
                )}
              </Show>
            </div>
            <div class="timelineSnapControls">
              <label>
                Snap
                <select
                  value={timelineSnapMode()}
                  onInput={(event) => setTimelineSnapMode(event.currentTarget.value as TimelineSnapMode)}
                >
                  <option value="Off">Off</option>
                  <option value="Beat">Beat</option>
                  <option value="Bar">Bar</option>
                  <option value="Grid">Grid</option>
                </select>
              </label>
              <label>
                Grid ms
                <input
                  type="number"
                  min="1"
                  value={timelineGridMs()}
                  disabled={timelineSnapMode() !== "Grid"}
                  onInput={(event) => setTimelineGridMs(Number(event.currentTarget.value))}
                />
              </label>
              <button onClick={snapTimelineDrafts} disabled={timelineSnapMode() === "Off"}>
                Snap Times
              </button>
            </div>
            <div class="timelineForm">
              <label>
                Cue
                <select
                  value={selectedTimelineCueId() ?? ""}
                  onInput={(event) => setTimelineCueId(Number(event.currentTarget.value))}
                  disabled={snapshot().cues.length === 0}
                >
                  <For each={timelineCueOptions()}>
                    {(cue) => <option value={cue.id}>{cue.label}</option>}
                  </For>
                </select>
              </label>
              <label>
                Time ms
                <input
                  type="number"
                  min="0"
                  value={timelineEventTimeMs()}
                  onInput={(event) => setTimelineEventTimeMs(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Track
                <select
                  value={timelineTrack()}
                  onInput={(event) => setTimelineTrack(event.currentTarget.value as TimelineTrackKind)}
                >
                  <option value="Lighting">Lighting</option>
                  <option value="Video">Video</option>
                </select>
              </label>
              <button class="primary" onClick={addTimelineCueEvent} disabled={snapshot().cues.length === 0}>
                Add Event
              </button>
              <button onClick={addTimelineCueEventAtPlayhead} disabled={snapshot().cues.length === 0}>
                At Playhead
              </button>
            </div>
            <div class="timelineList">
              <For each={timelineEventRows()}>
                {(event) => {
                  const draft = () => timelineEventDraft(event);
                  return (
                    <div class="timelineItem timelineEventItem">
                      <div>
                        <strong>{event.time_ms} ms</strong>
                        <span>{event.cue_label} / {event.track}</span>
                      </div>
                      <label>
                        Cue
                        <select
                          value={draft().cue_id}
                          onInput={(inputEvent) =>
                            updateTimelineEventDraft(event, {
                              cue_id: Number(inputEvent.currentTarget.value),
                            })
                          }
                        >
                          <For each={timelineCueOptions()}>
                            {(cue) => <option value={cue.id}>{cue.label}</option>}
                          </For>
                        </select>
                      </label>
                      <label>
                        Time
                        <input
                          type="number"
                          min="0"
                          value={draft().time_ms}
                          onInput={(inputEvent) =>
                            updateTimelineEventDraft(event, {
                              time_ms: Number(inputEvent.currentTarget.value),
                            })
                          }
                        />
                      </label>
                      <label>
                        Track
                        <select
                          value={draft().track}
                          onInput={(inputEvent) =>
                            updateTimelineEventDraft(event, {
                              track: inputEvent.currentTarget.value as TimelineTrackKind,
                            })
                          }
                        >
                          <option value="Lighting">Lighting</option>
                          <option value="Video">Video</option>
                        </select>
                      </label>
                      <div class="buttonRow">
                        <button onClick={() => void setTimelineCueEvent(event)}>Save</button>
                        <button onClick={() => removeTimelineEvent(event.id)}>Remove</button>
                      </div>
                    </div>
                  );
                }}
              </For>
            </div>
            <div class="automationForm">
              <label>
                Automation
                <select
                  value={selectedEffectAttribute()}
                  disabled={!selectedFixture()}
                  onInput={(event) => setEffectAttribute(event.currentTarget.value)}
                >
                  <For each={activeControls()}>
                    {(control) => <option value={control.attribute}>{control.attribute}</option>}
                  </For>
                </select>
              </label>
              <label>
                Start
                <input
                  type="number"
                  min="0"
                  value={automationStartMs()}
                  onInput={(event) => setAutomationStartMs(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                End
                <input
                  type="number"
                  min="0"
                  value={automationEndMs()}
                  onInput={(event) => setAutomationEndMs(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                From
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={automationStartValue()}
                  onInput={(event) => setAutomationStartValue(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                To
                <input
                  type="number"
                  min="0"
                  max="65535"
                  value={automationEndValue()}
                  onInput={(event) => setAutomationEndValue(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Curve
                <select
                  value={automationInterpolation()}
                  onInput={(event) => setAutomationInterpolation(event.currentTarget.value as AutomationInterpolation)}
                >
                  <option value="Linear">Linear</option>
                  <option value="Step">Step</option>
                  <option value="Bezier">Bezier</option>
                </select>
              </label>
              <button class="primary" onClick={addTimelineAutomation} disabled={!selectedFixture()}>
                Add Automation
              </button>
            </div>
            <div class="timelineList">
              <For each={timelineAutomationRows()}>
                {(automation) => {
                  const draft = () => timelineAutomationDraft(automation);
                  const attributes = () => fixtureAttributeOptions(draft().fixture_id);
                  return (
                    <div class="timelineItem timelineAutomationItem">
                      <div>
                        <strong>{automation.fixture_label} / {automation.attribute}</strong>
                        <span>
                          {automation.keyframes[0]?.time_ms ?? 0}-
                          {automation.keyframes[automation.keyframes.length - 1]?.time_ms ?? 0} ms / {automation.keyframes.length} keys
                        </span>
                      </div>
                      <div class="automationEditGrid">
                        <label>
                          Fixture
                          <select
                            value={draft().fixture_id}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                fixture_id: Number(event.currentTarget.value),
                              })
                            }
                          >
                            <For each={snapshot().fixtures}>
                              {(fixture) => <option value={fixture.id}>{fixture.label}</option>}
                            </For>
                          </select>
                        </label>
                        <label>
                          Attribute
                          <select
                            value={draft().attribute}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                attribute: event.currentTarget.value,
                              })
                            }
                          >
                            <For each={attributes()}>
                              {(attribute) => <option value={attribute}>{attribute}</option>}
                            </For>
                          </select>
                        </label>
                        <label>
                          Start
                          <input
                            type="number"
                            min="0"
                            value={draft().start_ms}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                start_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          End
                          <input
                            type="number"
                            min="0"
                            value={draft().end_ms}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                end_ms: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          From
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={draft().start_value}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                start_value: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          To
                          <input
                            type="number"
                            min="0"
                            max="65535"
                            value={draft().end_value}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                end_value: Number(event.currentTarget.value),
                              })
                            }
                          />
                        </label>
                        <label>
                          Curve
                          <select
                            value={draft().interpolation}
                            onInput={(event) =>
                              updateTimelineAutomationDraft(automation, {
                                interpolation: event.currentTarget.value as AutomationInterpolation,
                              })
                            }
                          >
                            <option value="Linear">Linear</option>
                            <option value="Step">Step</option>
                            <option value="Bezier">Bezier</option>
                          </select>
                        </label>
                      </div>
                      <div class="buttonRow">
                        <button onClick={() => void setTimelineAutomation(automation)}>Save</button>
                        <button onClick={() => removeTimelineAutomation(automation.id)}>Remove</button>
                      </div>
                    </div>
                  );
                }}
              </For>
            </div>
          </div>
          <div class="effectEditor">
            <div class="panelHeader">
              <h2>Effects</h2>
              <div class="panelHeaderActions">
                <span>{snapshot().effects.length}</span>
                <button onClick={loadEffectPreset}>Load</button>
                <button onClick={loadEffectPresetForCurrentTarget} disabled={Boolean(effectTargetOverrideError())}>
                  Load Target
                </button>
              </div>
            </div>
            <div class="effectForm">
              <Show when={effectTargetMode() !== "video"}>
                <label>
                  Attribute
                  <select
                    value={selectedEffectAttribute()}
                    disabled={!selectedFixture()}
                    onInput={(event) => setEffectAttribute(event.currentTarget.value)}
                  >
                    <For each={activeControls()}>
                      {(control) => <option value={control.attribute}>{control.attribute}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <div class="split">
                <label>
                  Target
                  <select value={effectTargetMode()} onInput={(event) => setEffectTargetMode(event.currentTarget.value as "fixture" | "group" | "video")}>
                    <option value="fixture">Selected fixture</option>
                    <option value="group">Group</option>
                    <option value="video">Video layer</option>
                  </select>
                </label>
                <label>
                  Type
                  <select value={effectType()} onInput={(event) => setEffectType(event.currentTarget.value as EffectKind)}>
                    <option value="Lfo">LFO</option>
                    <option value="PositionWave">Position Wave</option>
                  </select>
                </label>
              </div>
              <Show when={effectTargetMode() === "group"}>
                <div class="effectGroupPicker">
                  <label>
                    Target groups
                    <input
                      value={effectTargetGroups()}
                      onInput={(event) => setEffectTargetGroups(event.currentTarget.value)}
                      placeholder="front, movers"
                    />
                  </label>
                  <Show when={fixtureGroupRows().length > 0}>
                    <div class="groupOverview">
                      <For each={fixtureGroupRows()}>
                        {(group) => {
                          const isActive = () => parseGroupIds(effectTargetGroups()).includes(group.groupId);
                          return (
                            <button
                              class={isActive() ? "groupChip active" : "groupChip"}
                              onClick={() => toggleEffectTargetGroup(group.groupId)}
                            >
                              {group.groupId}
                              <span>{group.count}</span>
                            </button>
                          );
                        }}
                      </For>
                    </div>
                  </Show>
                </div>
              </Show>
              <Show when={effectTargetMode() === "video"}>
                <div class="videoEffectTarget">
                  <label>
                    Layer
                    <select
                      value={selectedEffectVideoLayerId() ?? ""}
                      disabled={snapshot().video.layers.length === 0}
                      onInput={(event) => setEffectVideoLayerId(Number(event.currentTarget.value))}
                    >
                      <For each={snapshot().video.layers}>
                        {(layer) => <option value={layer.id}>{layer.label}</option>}
                      </For>
                    </select>
                  </label>
                  <label>
                    Param
                    <select value={effectVideoParam()} onInput={(event) => setEffectVideoParam(event.currentTarget.value as VideoParam)}>
                      <option value="Opacity">Opacity</option>
                      <option value="Speed">Speed</option>
                      <option value="BpmSyncEnabled">BPM Sync</option>
                      <option value="BpmSyncRatio">BPM Sync Ratio</option>
                      <option value="BpmSyncLoopBars">BPM Loop Bars</option>
                      <option value="TransformX">Transform X</option>
                      <option value="TransformY">Transform Y</option>
                      <option value="TransformScaleX">Scale X</option>
                      <option value="TransformScaleY">Scale Y</option>
                      <option value="TransformRotationDeg">Rotation</option>
                      <option value="TransformCropLeft">Crop L</option>
                      <option value="TransformCropTop">Crop T</option>
                      <option value="TransformCropRight">Crop R</option>
                      <option value="TransformCropBottom">Crop B</option>
                      <option value="ColorBrightness">Brightness</option>
                      <option value="ColorContrast">Contrast</option>
                      <option value="ColorHueDeg">Hue</option>
                      <option value="ColorSaturation">Saturation</option>
                      <option value="ColorGamma">Gamma</option>
                      <option value="FxPixelate">Pixelate</option>
                      <option value="FxBlur">Blur</option>
                      <option value="FxGlow">Glow</option>
                      <option value="FxEdge">Edge</option>
                      <option value="FxKeyRed">Key R</option>
                      <option value="FxKeyGreen">Key G</option>
                      <option value="FxKeyBlue">Key B</option>
                      <option value="FxKeyThreshold">Key Threshold</option>
                    </select>
                  </label>
                  <div class="split">
                    <label>
                      Low
                      <input
                        type="number"
                        step="0.01"
                        value={effectVideoLow()}
                        onInput={(event) => setEffectVideoLow(Number(event.currentTarget.value))}
                      />
                    </label>
                    <label>
                      High
                      <input
                        type="number"
                        step="0.01"
                        value={effectVideoHigh()}
                        onInput={(event) => setEffectVideoHigh(Number(event.currentTarget.value))}
                      />
                    </label>
                  </div>
                  <Show when={effectType() === "PositionWave"}>
                    <div class="videoTargetPositionPanel">
                      <div class="waveStageHeader">
                        <div>
                          <strong>Video Position</strong>
                          <span>
                            X {effectVideoPositionX().toFixed(1)}, Y {effectVideoPositionY().toFixed(1)}, Z{" "}
                            {effectVideoPositionZ().toFixed(1)}
                          </span>
                        </div>
                      </div>
                      <div class="buttonRow">
                        <button onClick={setEffectVideoPositionFromSelectedOutput} disabled={snapshot().video.outputs.length === 0}>
                          Output Surface
                        </button>
                        <button onClick={setEffectVideoPositionFromWaveOrigin}>Wave Origin</button>
                        <button onClick={setEffectVideoPositionFromStageCenter}>Stage Center</button>
                      </div>
                      <div class="triple">
                        <label>
                          X
                          <input
                            type="number"
                            step="0.1"
                            value={effectVideoPositionX()}
                            onInput={(event) => setEffectVideoPositionX(Number(event.currentTarget.value))}
                          />
                        </label>
                        <label>
                          Y
                          <input
                            type="number"
                            step="0.1"
                            value={effectVideoPositionY()}
                            onInput={(event) => setEffectVideoPositionY(Number(event.currentTarget.value))}
                          />
                        </label>
                        <label>
                          Z
                          <input
                            type="number"
                            step="0.1"
                            value={effectVideoPositionZ()}
                            onInput={(event) => setEffectVideoPositionZ(Number(event.currentTarget.value))}
                          />
                        </label>
                      </div>
                    </div>
                  </Show>
                </div>
              </Show>
              <div class="split">
                <label>
                  Shape
                  <select value={effectShape()} onInput={(event) => setEffectShape(event.currentTarget.value as LfoShape)}>
                    <option value="Sine">Sine</option>
                    <option value="Cosine">Cosine</option>
                    <option value="Triangle">Triangle</option>
                    <option value="Saw">Saw</option>
                    <option value="Square">Square</option>
                    <option value="Random">Random</option>
                    <option value="Perlin">Perlin</option>
                  </select>
                </label>
              </div>
              <Show when={effectType() === "Lfo"}>
                <label>
                  Period ms
                  <input
                    type="number"
                    min="10"
                    value={effectPeriod()}
                    onInput={(event) => setEffectPeriod(Number(event.currentTarget.value))}
                  />
                </label>
              </Show>
              <Show when={effectType() === "PositionWave"}>
                <div class="waveControls">
                  <div class="waveStagePicker">
                    <div class="waveStageHeader">
                      <div>
                        <strong>Stage Wave</strong>
                        <span>
                          O {waveOriginX().toFixed(1)}, {waveOriginY().toFixed(1)}, {waveOriginZ().toFixed(1)} / D{" "}
                          {waveDirectionX().toFixed(1)}, {waveDirectionY().toFixed(1)}, {waveDirectionZ().toFixed(1)}
                        </span>
                      </div>
                    </div>
                    <div class="buttonRow">
                      <button onClick={setWaveOriginFromStageCenter}>Stage Center</button>
                      <button onClick={setWaveOriginFromSelectedFixture} disabled={!selectedFixture()}>
                        Selected Fixture
                      </button>
                      <button onClick={() => setWaveDirectionPreset(1, 0, 0)}>X</button>
                      <button onClick={() => setWaveDirectionPreset(0, 0, 1)}>Z</button>
                      <button onClick={() => setWaveDirectionPreset(0, 0, 0)}>Radial</button>
                    </div>
                    <svg
                      class="waveStageMap"
                      viewBox={`0 0 ${stageViewBoxSize} ${stageViewBoxSize}`}
                      onPointerDown={(event) =>
                        startWaveStageDrag(event, event.altKey ? "videoTarget" : event.shiftKey ? "direction" : "origin")
                      }
                      onPointerMove={moveWaveStageDrag}
                      onPointerUp={endWaveStageDrag}
                      onPointerCancel={endWaveStageDrag}
                    >
                      <defs>
                        <pattern id="wave-stage-grid" width="5" height="5" patternUnits="userSpaceOnUse">
                          <path d="M 5 0 L 0 0 0 5" />
                        </pattern>
                      </defs>
                      <rect class="stageFloor" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
                      <rect class="stageGrid" x="0" y="0" width={stageViewBoxSize} height={stageViewBoxSize} />
                      <line class="waveGuideLine" x1={stageOrigin2d().x} y1="0" x2={stageOrigin2d().x} y2={stageViewBoxSize} />
                      <line class="waveGuideLine" x1="0" y1={stageOrigin2d().z} x2={stageViewBoxSize} y2={stageOrigin2d().z} />
                      <For each={visualizerVideoSurfaces2d()}>
                        {(surface) => (
                          <g
                            class={[
                              "waveVideoSurface",
                              selectedVideoOutputId() === surface.id ? "selected" : "",
                              surface.active ? "" : "inactive",
                              effectTargetMode() === "video" ? "" : "inert",
                            ]
                              .filter(Boolean)
                              .join(" ")}
                            transform={`translate(${surface.x} ${surface.z}) rotate(${surface.rotationDeg})`}
                            onPointerDown={(event) => {
                              if (effectTargetMode() !== "video") {
                                return;
                              }
                              const output = snapshot().video.outputs.find((candidate) => candidate.id === surface.id);
                              if (!output) {
                                return;
                              }
                              event.preventDefault();
                              event.stopPropagation();
                              setEffectVideoPositionFromVideoOutput(output);
                            }}
                          >
                            <rect x={-surface.width / 2} y={-surface.height / 2} width={surface.width} height={surface.height} />
                            <line x1={-surface.width / 2} y1="0" x2={surface.width / 2} y2="0" />
                            <line x1="0" y1={-surface.height / 2} x2="0" y2={surface.height / 2} />
                          </g>
                        )}
                      </For>
                      <For each={visualizerFixtures()}>
                        {(fixture) => (
                          <circle
                            class={[
                              "waveStageFixture",
                              waveTargetFixtureIds().has(fixture.id) ? "target" : "muted",
                              selectedFixtureId() === fixture.id ? "selected" : "",
                            ]
                              .filter(Boolean)
                              .join(" ")}
                            cx={fixture.x}
                            cy={fixture.z}
                            r={waveTargetFixtureIds().has(fixture.id) ? 1.8 : 1.35}
                            fill={fixture.color}
                          />
                        )}
                      </For>
                      <Show when={waveDirectionIsRadial()}>
                        <circle class="waveGuideRing" cx={waveOriginSvgPoint().x} cy={waveOriginSvgPoint().z} r={waveRadialRadius()} />
                      </Show>
                      <Show when={!waveDirectionIsRadial()}>
                        <line
                          class="waveDirectionLine"
                          x1={waveOriginSvgPoint().x}
                          y1={waveOriginSvgPoint().z}
                          x2={waveDirectionSvgPoint().x}
                          y2={waveDirectionSvgPoint().z}
                        />
                      </Show>
                      <Show when={effectTargetMode() === "video"}>
                        <line
                          class="waveVideoTargetLine"
                          x1={waveOriginSvgPoint().x}
                          y1={waveOriginSvgPoint().z}
                          x2={effectVideoTargetSvgPoint().x}
                          y2={effectVideoTargetSvgPoint().z}
                        />
                      </Show>
                      <circle
                        class={waveStageDrag() === "origin" ? "waveOriginHandle dragging" : "waveOriginHandle"}
                        cx={waveOriginSvgPoint().x}
                        cy={waveOriginSvgPoint().z}
                        r="2.1"
                      />
                      <Show when={!waveDirectionIsRadial()}>
                        <circle
                          class={waveStageDrag() === "direction" ? "waveDirectionHandle dragging" : "waveDirectionHandle"}
                          cx={waveDirectionSvgPoint().x}
                          cy={waveDirectionSvgPoint().z}
                          r="2"
                          onPointerDown={(event) => startWaveStageDrag(event, "direction")}
                          onPointerMove={moveWaveStageDrag}
                          onPointerUp={endWaveStageDrag}
                          onPointerCancel={endWaveStageDrag}
                        />
                      </Show>
                      <Show when={effectTargetMode() === "video"}>
                        <circle
                          class={waveStageDrag() === "videoTarget" ? "waveVideoTargetHandle dragging" : "waveVideoTargetHandle"}
                          cx={effectVideoTargetSvgPoint().x}
                          cy={effectVideoTargetSvgPoint().z}
                          r="2.1"
                          onPointerDown={(event) => startWaveStageDrag(event, "videoTarget")}
                          onPointerMove={moveWaveStageDrag}
                          onPointerUp={endWaveStageDrag}
                          onPointerCancel={endWaveStageDrag}
                        />
                      </Show>
                    </svg>
                    <div class="waveStageReadout">
                      <span>X {stageWorldBounds().minX.toFixed(1)} to {stageWorldBounds().maxX.toFixed(1)}</span>
                      <span>Z {stageWorldBounds().minZ.toFixed(1)} to {stageWorldBounds().maxZ.toFixed(1)}</span>
                    </div>
                  </div>
                  <div class="triple">
                    <label>
                      Origin X
                      <input type="number" value={waveOriginX()} onInput={(event) => setWaveOriginX(Number(event.currentTarget.value))} />
                    </label>
                    <label>
                      Origin Y
                      <input type="number" value={waveOriginY()} onInput={(event) => setWaveOriginY(Number(event.currentTarget.value))} />
                    </label>
                    <label>
                      Origin Z
                      <input type="number" value={waveOriginZ()} onInput={(event) => setWaveOriginZ(Number(event.currentTarget.value))} />
                    </label>
                  </div>
                  <div class="triple">
                    <label>
                      Dir X
                      <input type="number" value={waveDirectionX()} onInput={(event) => setWaveDirectionX(Number(event.currentTarget.value))} />
                    </label>
                    <label>
                      Dir Y
                      <input type="number" value={waveDirectionY()} onInput={(event) => setWaveDirectionY(Number(event.currentTarget.value))} />
                    </label>
                    <label>
                      Dir Z
                      <input type="number" value={waveDirectionZ()} onInput={(event) => setWaveDirectionZ(Number(event.currentTarget.value))} />
                    </label>
                  </div>
                  <div class="split">
                    <label>
                      Speed
                      <input type="number" step="0.1" value={waveSpeed()} onInput={(event) => setWaveSpeed(Number(event.currentTarget.value))} />
                    </label>
                    <label>
                      Wavelength
                      <input type="number" min="0.001" step="0.1" value={waveWavelength()} onInput={(event) => setWaveWavelength(Number(event.currentTarget.value))} />
                    </label>
                  </div>
                </div>
              </Show>
              <Show when={effectTargetMode() !== "video"}>
                <div class="split">
                  <label>
                    Low
                    <input
                      type="number"
                      min="0"
                      max="65535"
                      value={effectLow()}
                      onInput={(event) => setEffectLow(Number(event.currentTarget.value))}
                    />
                  </label>
                  <label>
                    High
                    <input
                      type="number"
                      min="0"
                      max="65535"
                      value={effectHigh()}
                      onInput={(event) => setEffectHigh(Number(event.currentTarget.value))}
                    />
                  </label>
                </div>
              </Show>
              <label>
                Phase
                <input
                  type="number"
                  min="0"
                  max="1"
                  step="0.01"
                  value={effectPhase()}
                  onInput={(event) => setEffectPhase(Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Blend
                <select value={effectBlendMode()} onInput={(event) => setEffectBlendMode(event.currentTarget.value as EffectBlendMode)}>
                  <option value="Override">Override</option>
                  <option value="Add">Add</option>
                  <option value="Multiply">Multiply</option>
                </select>
              </label>
              <button
                class="primary"
                onClick={addEffect}
                disabled={effectTargetMode() === "video" ? snapshot().video.layers.length === 0 : !selectedFixture()}
              >
                Add Effect
              </button>
            </div>
            <div class="effectList">
              <For each={snapshot().effects}>
                {(effect, index) => (
                  <div class="effectItem">
                    <div>
                      <strong>{effect.label}</strong>
                      <span>
                        {effect.enabled ? "" : "Disabled / "}
                        {effect.attribute} / {effect.effect_type === "PositionWave" ? "Wave" : "LFO"} / {effect.shape}
                        {effect.period_ms ? ` / ${effect.period_ms}ms` : ""}
                        {effect.wavelength ? ` / wl ${effect.wavelength}` : ""}
                        {effect.target_group_ids.length > 0 ? ` / groups ${effect.target_group_ids.join(",")}` : ""}
                        {effect.video_targets.length > 0
                          ? ` / video ${effect.video_targets.map((target) => target.param).join(",")}`
                          : ""}
                        {` / ${effect.blend_mode}`}
                      </span>
                    </div>
                    <button onClick={() => void moveEffect(effect.id, -1)} disabled={index() === 0}>
                      Up
                    </button>
                    <button onClick={() => void moveEffect(effect.id, 1)} disabled={index() === snapshot().effects.length - 1}>
                      Down
                    </button>
                    <button onClick={() => void setEffectEnabled(effect.id, !effect.enabled)}>
                      {effect.enabled ? "Disable" : "Enable"}
                    </button>
                    <Show when={effect.effect_type === "PositionWave" && effect.video_targets.length > 0}>
                      <button onClick={() => void setEffectVideoTargetsFromSelectedOutput(effect.id, effect.video_targets)}>
                        Use Output Pos
                      </button>
                    </Show>
                    <button onClick={() => saveEffectPreset(effect.id)}>Save</button>
                    <button onClick={() => removeEffect(effect.id)}>Remove</button>
                  </div>
                )}
              </For>
            </div>
          </div>
          <DmxRawMonitor
            previews={dmxPreviewOptions()}
            activeUniverse={activeDmxPreviewUniverse()}
            activeCount={nonZeroDmxCount()}
            cells={dmxCells()}
            onUniverseChange={setRawDmxUniverse}
          />
        </section>

        <aside
          class={setupPanelClass("panel output setupPanel controlPanel", ["output"])}
          ref={registerSetupPanel(["output"])}
          tabIndex={-1}
        >
          <DmxOutputConfigPanel
            output={output()}
            serialPorts={serialPorts()}
            isSerialProtocol={isSerialDmxProtocol}
            onOutputChange={(nextOutput) => setOutput(nextOutput)}
            onProtocolChange={setOutputProtocol}
            onRefreshSerialPorts={refreshSerialPorts}
            onApply={applyOutput}
          />
          <DmxTestFramePanel
            protocolLabel={outputProtocolLabel(output().protocol)}
            channel={dmxTestChannel()}
            width={dmxTestWidth()}
            value={dmxTestValue()}
            onChannelChange={setDmxTestChannel}
            onWidthChange={setDmxTestWidth}
            onValueChange={setDmxTestValue}
            onSendTest={sendDmxTestFrame}
            onSendRoutes={sendDmxRoutesTestFrame}
          />
          <DmxRoutesPanel
            routes={dmxOutputRoutes()}
            routeLabel={dmxRouteLabel}
            onAddCurrent={addCurrentDmxRoute}
            onApplyRoutes={applyCurrentDmxRoutes}
            onRemoveRoute={removeDmxRoute}
          />
          <label>
            Lighting Master
            <input
              type="number"
              min="0"
              max="1"
              step="0.01"
              value={snapshot().lighting_master}
              onChange={(event) => void setLightingMaster(Number(event.currentTarget.value))}
            />
          </label>
          <Show when={snapshot().submasters.length > 0}>
            <div class="submasterList">
              <h3>Submasters</h3>
              <For each={snapshot().submasters}>
                {(submaster) => (
                  <label class="submasterControl">
                    <span>{submaster.label}</span>
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={submaster.level}
                      onChange={(event) => void setGroupSubmaster(submaster.group_id, Number(event.currentTarget.value))}
                    />
                    <strong>{Math.round(submaster.level * 100)}%</strong>
                  </label>
                )}
              </For>
            </div>
          </Show>
          <div class="blackout">
            <button onClick={() => setBlackout(true)}>Blackout</button>
            <button onClick={() => setBlackout(false)}>Clear</button>
          </div>
          <div class="clock">
            <h3>Clock</h3>
            <div class="clockSource">
              <span>Source</span>
              <strong>{snapshot().clock.source === "MidiClock" ? "MIDI Clock" : snapshot().clock.source}</strong>
            </div>
            <div class="split">
              <label>
                BPM
                <input
                  type="number"
                  min="20"
                  max="300"
                  step="0.1"
                  value={bpmDraft()}
                  onInput={(event) => setBpmDraft(event.currentTarget.value)}
                />
              </label>
              <label>
                Beat
                <input value={`${snapshot().clock.beat_counter}.${Math.floor(snapshot().clock.beat_phase * 100)}`} readOnly />
              </label>
            </div>
            <div class="blackout">
              <button onClick={applyBpm}>Set BPM</button>
              <button class="primary" onClick={tapBpm}>Tap</button>
            </div>
            <div class="midiClock">
              <div class="buttonRow">
                <button onClick={() => {
                  void refreshMidiInputs();
                  void refreshMidiOutputs();
                }}>Scan MIDI</button>
                <button onClick={disconnectMidiClock} disabled={!midiConnected()}>Disconnect</button>
              </div>
              <select
                value={selectedMidiInput() ?? ""}
                onInput={(event) => setSelectedMidiInput(Number(event.currentTarget.value))}
              >
                <For each={midiInputs()}>
                  {(input) => <option value={input.index}>{input.name}</option>}
                </For>
              </select>
              <button class="primary" onClick={connectMidiClock} disabled={midiInputs().length === 0}>
                Connect MIDI Clock / MTC
              </button>
              <h3>MIDI Feedback</h3>
              <select
                value={selectedMidiOutput() ?? ""}
                onInput={(event) => setSelectedMidiOutput(Number(event.currentTarget.value))}
              >
                <For each={midiOutputs()}>
                  {(output) => <option value={output.index}>{output.name}</option>}
                </For>
              </select>
              <div class="buttonRow">
                <button class="primary" onClick={connectMidiFeedback} disabled={midiOutputs().length === 0 || midiFeedbackConnected()}>
                  Connect Feedback
                </button>
                <button onClick={disconnectMidiFeedback} disabled={!midiFeedbackConnected()}>
                  Disconnect Feedback
                </button>
                <button onClick={() => void sendMidiFeedback()} disabled={!midiFeedbackConnected() || midiMappings().length === 0}>
                  Send Feedback
                </button>
              </div>
              <label class="checkbox">
                <input
                  type="checkbox"
                  checked={midiFeedbackEnabled()}
                  disabled={!midiFeedbackConnected()}
                  onChange={(event) => setMidiFeedbackEnabled(event.currentTarget.checked)}
                />
                Auto feedback
              </label>
              <h3>MIDI Control</h3>
              <div class="split">
                <label>
                  Message
                  <select value={midiMapMessage()} onInput={(event) => setMidiMapMessage(event.currentTarget.value as MidiControlMessage)}>
                    <option value="ControlChange">CC</option>
                    <option value="NoteOn">Note On</option>
                    <option value="NoteOff">Note Off</option>
                    <option value="ProgramChange">Program</option>
                  </select>
                </label>
                <label>
                  Number
                  <input type="number" min="0" max="127" value={midiMapNumber()} onInput={(event) => setMidiMapNumber(Number(event.currentTarget.value))} />
                </label>
              </div>
              <div class="split">
                <label>
                  Channel
                  <input type="number" min="-1" max="15" value={midiMapChannel()} onInput={(event) => setMidiMapChannel(Number(event.currentTarget.value))} />
                </label>
                <label>
                  Action
                  <select value={midiMapAction()} onInput={(event) => setMidiMapAction(event.currentTarget.value as MidiControlAction)}>
                    <option value="FixtureAttribute">Fixture Attribute</option>
                    <option value="TriggerCue">Trigger Cue</option>
                    <option value="TriggerNextCue">Cue Next</option>
                    <option value="TriggerPreviousCue">Cue Previous</option>
                    <option value="VideoParam">Video Param</option>
                    <option value="VideoCuePointAdd">Video Cue Add</option>
                    <option value="VideoCuePointRemove">Video Cue Remove</option>
                    <option value="VideoCuePointJump">Video Cue Jump</option>
                    <option value="VideoLayerEnabled">Layer Enable</option>
                    <option value="VideoLayerSolo">Layer Solo</option>
                    <option value="VideoPlay">Video Play</option>
                    <option value="VideoLoop">Video A-B Loop</option>
                    <option value="VideoOutputEnabled">Output Enable</option>
                    <option value="VideoOutputOpacity">Output Opacity</option>
                    <option value="VideoOutputFade">Output Fade</option>
                    <option value="VideoOutputBlackout">Output Blackout</option>
                    <option value="TimelinePlay">Timeline Play</option>
                    <option value="TimelineSeek">Timeline Seek</option>
                    <option value="LightingMaster">Lighting Master</option>
                    <option value="GroupSubmaster">Group Submaster</option>
                    <option value="CueFadePause">Cue Fade Pause</option>
                    <option value="Blackout">Blackout</option>
                    <option value="VideoBlackout">Video Blackout</option>
                  </select>
                </label>
              </div>
              <Show when={midiMapAction() === "FixtureAttribute"}>
                <label>
                  Attribute
                  <input value={midiMapAttribute()} onInput={(event) => setMidiMapAttribute(event.currentTarget.value)} />
                </label>
              </Show>
              <Show when={midiMapAction() === "TriggerCue"}>
                <label>
                  Cue
                  <select value={selectedMidiCueId() ?? ""} onInput={(event) => setMidiMapCueId(Number(event.currentTarget.value))}>
                    <For each={snapshot().cues}>
                      {(cue) => <option value={cue.id}>{cue.id}: {cue.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={midiMapAction() === "GroupSubmaster"}>
                <label>
                  Group ID
                  <input value={midiMapGroupId()} onInput={(event) => setMidiMapGroupId(event.currentTarget.value)} />
                </label>
              </Show>
              <Show
                when={
                  midiMapAction() === "VideoParam" ||
                  midiMapAction() === "VideoCuePointAdd" ||
                  midiMapAction() === "VideoCuePointRemove" ||
                  midiMapAction() === "VideoCuePointJump" ||
                  midiMapAction() === "VideoLayerEnabled" ||
                  midiMapAction() === "VideoLayerSolo" ||
                  midiMapAction() === "VideoPlay" ||
                  midiMapAction() === "VideoLoop"
                }
              >
                <label>
                  Layer
                  <select value={selectedMidiLayerId() ?? ""} onInput={(event) => setMidiMapLayerId(Number(event.currentTarget.value))}>
                    <For each={snapshot().video.layers}>
                      {(layer) => <option value={layer.id}>{layer.id}: {layer.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show
                when={
                  midiMapAction() === "VideoOutputEnabled" ||
                  midiMapAction() === "VideoOutputOpacity" ||
                  midiMapAction() === "VideoOutputFade" ||
                  midiMapAction() === "VideoOutputBlackout"
                }
              >
                <label>
                  Output
                  <select
                    value={selectedMidiVideoOutputId() ?? ""}
                    onInput={(event) => setMidiMapVideoOutputId(Number(event.currentTarget.value))}
                  >
                    <For each={snapshot().video.outputs}>
                      {(output) => <option value={output.id}>{output.id}: {output.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={midiMapAction() === "VideoParam"}>
                <label>
                  Video Param
                  <select value={midiMapVideoParam()} onInput={(event) => setMidiMapVideoParam(event.currentTarget.value as VideoParam)}>
                    <option value="Opacity">Opacity</option>
                    <option value="Speed">Speed</option>
                    <option value="PositionMs">Position</option>
                    <option value="BpmSyncEnabled">BPM Sync</option>
                    <option value="BpmSyncRatio">BPM Sync Ratio</option>
                    <option value="BpmSyncLoopBars">BPM Loop Bars</option>
                    <option value="TransformX">X</option>
                    <option value="TransformY">Y</option>
                    <option value="TransformScaleX">Scale X</option>
                    <option value="TransformScaleY">Scale Y</option>
                    <option value="TransformRotationDeg">Rotation</option>
                    <option value="ColorBrightness">Brightness</option>
                    <option value="ColorContrast">Contrast</option>
                    <option value="ColorHueDeg">Hue</option>
                    <option value="ColorSaturation">Saturation</option>
                    <option value="ColorGamma">Gamma</option>
                    <option value="FxPixelate">Pixelate</option>
                    <option value="FxBlur">Blur</option>
                    <option value="FxGlow">Glow</option>
                    <option value="FxEdge">Edge</option>
                    <option value="FxKeyRed">Key R</option>
                    <option value="FxKeyGreen">Key G</option>
                    <option value="FxKeyBlue">Key B</option>
                    <option value="FxKeyThreshold">Key Threshold</option>
                  </select>
                </label>
              </Show>
              <Show when={midiMapAction() === "VideoCuePointJump"}>
                <label>
                  Cue point index
                  <input type="number" min="0" value={midiMapCuePointIndex()} onInput={(event) => setMidiMapCuePointIndex(Number(event.currentTarget.value))} />
                </label>
              </Show>
              <Show when={midiMapAction() === "VideoOutputFade" || midiMapAction() === "VideoCuePointAdd" || midiMapAction() === "VideoCuePointRemove"}>
                <label>
                  {midiMapAction() === "VideoOutputFade" ? "Fade ms" : "Cue point ms"}
                  <input type="number" min="0" step="10" value={midiMapDurationMs()} onInput={(event) => setMidiMapDurationMs(Number(event.currentTarget.value))} />
                </label>
              </Show>
              <div class="split">
                <label>
                  {midiMapAction() === "VideoLoop" ? "Loop In ms" : "Low"}
                  <input type="number" value={midiMapLow()} onInput={(event) => setMidiMapLow(Number(event.currentTarget.value))} />
                </label>
                <label>
                  {midiMapAction() === "VideoLoop" ? "Loop Out ms" : "High"}
                  <input type="number" value={midiMapHigh()} onInput={(event) => setMidiMapHigh(Number(event.currentTarget.value))} />
                </label>
              </div>
              <div class="buttonRow">
                <button onClick={learnMidiControl} disabled={midiInputs().length === 0}>
                  Learn
                </button>
                <button onClick={addMidiMapping}>Add Mapping</button>
                <button class="primary" onClick={connectMidiControl} disabled={midiInputs().length === 0 || midiMappings().length === 0 || midiControlConnected()}>
                  Connect MIDI Control
                </button>
                <button onClick={disconnectMidiControl} disabled={!midiControlConnected()}>
                  Disconnect Control
                </button>
              </div>
              <div class="buttonRow">
                <button onClick={loadMidiMappings}>Load Mapping</button>
                <button onClick={saveMidiMappings} disabled={midiMappings().length === 0}>Save Mapping</button>
              </div>
              <div class="timelineList">
                <For each={midiMappings()}>
                  {(mapping, index) => (
                    <div class="timelineItem">
                      <strong>{mapping.message} {mapping.number}</strong>
                      <span>{mapping.channel === null || mapping.channel === undefined ? "Any ch" : `Ch ${mapping.channel}`} / {midiMappingTargetLabel(mapping)}</span>
                      <button onClick={() => removeMidiMapping(index())}>Remove</button>
                    </div>
                  )}
                </For>
              </div>
            </div>
            <div class="oscInput">
              <h3>OSC Input</h3>
              <div class="split">
                <label>
                  Bind IP
                  <input value={oscBindIp()} onInput={(event) => setOscBindIp(event.currentTarget.value)} />
                </label>
                <label>
                  Port
                  <input
                    type="number"
                    min="1"
                    value={oscPort()}
                    onInput={(event) => setOscPort(Number(event.currentTarget.value))}
                  />
                </label>
              </div>
              <div class="buttonRow">
                <button class="primary" onClick={startOscInput} disabled={oscRunning()}>
                  Start OSC
                </button>
                <button onClick={stopOscInput} disabled={!oscRunning()}>
                  Stop OSC
                </button>
              </div>
              <label>
                Address
                <input
                  value={oscMapAddress()}
                  placeholder="/touchosc/page/*/fader/1"
                  onInput={(event) => setOscMapAddress(event.currentTarget.value)}
                />
              </label>
              <div class="split">
                <label>
                  Action
                  <select value={oscMapAction()} onInput={(event) => setOscMapAction(event.currentTarget.value as OscControlAction)}>
                    <option value="FixtureAttribute">Fixture Attribute</option>
                    <option value="TriggerCue">Trigger Cue</option>
                    <option value="TriggerNextCue">Cue Next</option>
                    <option value="TriggerPreviousCue">Cue Previous</option>
                    <option value="VideoParam">Video Param</option>
                    <option value="VideoCuePointAdd">Video Cue Add</option>
                    <option value="VideoCuePointRemove">Video Cue Remove</option>
                    <option value="VideoCuePointJump">Video Cue Jump</option>
                    <option value="VideoLayerEnabled">Layer Enable</option>
                    <option value="VideoLayerSolo">Layer Solo</option>
                    <option value="VideoPlay">Video Play</option>
                    <option value="VideoLoop">Video A-B Loop</option>
                    <option value="VideoOutputEnabled">Output Enable</option>
                    <option value="VideoOutputOpacity">Output Opacity</option>
                    <option value="VideoOutputFade">Output Fade</option>
                    <option value="VideoOutputBlackout">Output Blackout</option>
                    <option value="TimelinePlay">Timeline Play</option>
                    <option value="TimelineSeek">Timeline Seek</option>
                    <option value="LightingMaster">Lighting Master</option>
                    <option value="GroupSubmaster">Group Submaster</option>
                    <option value="CueFadePause">Cue Fade Pause</option>
                    <option value="Blackout">Blackout</option>
                    <option value="VideoBlackout">Video Blackout</option>
                  </select>
                </label>
                <label>
                  Attribute
                  <input value={oscMapAttribute()} onInput={(event) => setOscMapAttribute(event.currentTarget.value)} disabled={oscMapAction() !== "FixtureAttribute"} />
                </label>
              </div>
              <Show when={oscMapAction() === "TriggerCue"}>
                <label>
                  Cue
                  <select value={selectedOscCueId() ?? ""} onInput={(event) => setOscMapCueId(Number(event.currentTarget.value))}>
                    <For each={snapshot().cues}>
                      {(cue) => <option value={cue.id}>{cue.id}: {cue.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={oscMapAction() === "GroupSubmaster"}>
                <label>
                  Group ID
                  <input value={oscMapGroupId()} onInput={(event) => setOscMapGroupId(event.currentTarget.value)} />
                </label>
              </Show>
              <Show
                when={
                  oscMapAction() === "VideoParam" ||
                  oscMapAction() === "VideoCuePointAdd" ||
                  oscMapAction() === "VideoCuePointRemove" ||
                  oscMapAction() === "VideoCuePointJump" ||
                  oscMapAction() === "VideoLayerEnabled" ||
                  oscMapAction() === "VideoLayerSolo" ||
                  oscMapAction() === "VideoPlay" ||
                  oscMapAction() === "VideoLoop"
                }
              >
                <label>
                  Layer
                  <select value={selectedOscLayerId() ?? ""} onInput={(event) => setOscMapLayerId(Number(event.currentTarget.value))}>
                    <For each={snapshot().video.layers}>
                      {(layer) => <option value={layer.id}>{layer.id}: {layer.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show
                when={
                  oscMapAction() === "VideoOutputEnabled" ||
                  oscMapAction() === "VideoOutputOpacity" ||
                  oscMapAction() === "VideoOutputFade" ||
                  oscMapAction() === "VideoOutputBlackout"
                }
              >
                <label>
                  Output
                  <select
                    value={selectedOscVideoOutputId() ?? ""}
                    onInput={(event) => setOscMapVideoOutputId(Number(event.currentTarget.value))}
                  >
                    <For each={snapshot().video.outputs}>
                      {(output) => <option value={output.id}>{output.id}: {output.label}</option>}
                    </For>
                  </select>
                </label>
              </Show>
              <Show when={oscMapAction() === "VideoParam"}>
                <label>
                  Video Param
                  <select value={oscMapVideoParam()} onInput={(event) => setOscMapVideoParam(event.currentTarget.value as VideoParam)}>
                    <option value="Opacity">Opacity</option>
                    <option value="Speed">Speed</option>
                    <option value="PositionMs">Position</option>
                    <option value="BpmSyncEnabled">BPM Sync</option>
                    <option value="BpmSyncRatio">BPM Sync Ratio</option>
                    <option value="BpmSyncLoopBars">BPM Loop Bars</option>
                    <option value="TransformX">X</option>
                    <option value="TransformY">Y</option>
                    <option value="TransformScaleX">Scale X</option>
                    <option value="TransformScaleY">Scale Y</option>
                    <option value="TransformRotationDeg">Rotation</option>
                    <option value="ColorBrightness">Brightness</option>
                    <option value="ColorContrast">Contrast</option>
                    <option value="ColorHueDeg">Hue</option>
                    <option value="ColorSaturation">Saturation</option>
                    <option value="ColorGamma">Gamma</option>
                    <option value="FxPixelate">Pixelate</option>
                    <option value="FxBlur">Blur</option>
                    <option value="FxGlow">Glow</option>
                    <option value="FxEdge">Edge</option>
                    <option value="FxKeyRed">Key R</option>
                    <option value="FxKeyGreen">Key G</option>
                    <option value="FxKeyBlue">Key B</option>
                    <option value="FxKeyThreshold">Key Threshold</option>
                  </select>
                </label>
              </Show>
              <Show when={oscMapAction() === "VideoCuePointJump"}>
                <label>
                  Cue point index
                  <input type="number" min="0" value={oscMapCuePointIndex()} onInput={(event) => setOscMapCuePointIndex(Number(event.currentTarget.value))} />
                </label>
              </Show>
              <Show when={oscMapAction() === "VideoOutputFade" || oscMapAction() === "VideoCuePointAdd" || oscMapAction() === "VideoCuePointRemove"}>
                <label>
                  {oscMapAction() === "VideoOutputFade" ? "Fade ms" : "Cue point ms"}
                  <input type="number" min="0" step="10" value={oscMapDurationMs()} onInput={(event) => setOscMapDurationMs(Number(event.currentTarget.value))} />
                </label>
              </Show>
              <div class="split">
                <label>
                  {oscMapAction() === "VideoLoop" ? "Loop In ms" : "Low"}
                  <input type="number" value={oscMapLow()} onInput={(event) => setOscMapLow(Number(event.currentTarget.value))} />
                </label>
                <label>
                  {oscMapAction() === "VideoLoop" ? "Loop Out ms" : "High"}
                  <input type="number" value={oscMapHigh()} onInput={(event) => setOscMapHigh(Number(event.currentTarget.value))} />
                </label>
              </div>
              <div class="buttonRow">
                <button onClick={learnOscControl} disabled={oscRunning()}>
                  Learn
                </button>
                <button onClick={addOscMapping}>Add OSC Mapping</button>
                <button onClick={loadOscMappings}>Load Mapping</button>
                <button onClick={saveOscMappings} disabled={oscMappings().length === 0}>Save Mapping</button>
              </div>
              <div class="timelineList">
                <For each={oscMappings()}>
                  {(mapping, index) => (
                    <div class="timelineItem">
                      <strong>{mapping.address}</strong>
                      <span>{oscMappingTargetLabel(mapping)}</span>
                      <button onClick={() => removeOscMapping(index())}>Remove</button>
                    </div>
                  )}
                </For>
              </div>
            </div>
            <div class="remoteControl">
              <h3>Web Remote</h3>
              <div class="split">
                <label>
                  Bind IP
                  <input value={remoteBindIp()} onInput={(event) => setRemoteBindIp(event.currentTarget.value)} />
                </label>
                <label>
                  Port
                  <input
                    type="number"
                    min="1"
                    value={remotePort()}
                    onInput={(event) => setRemotePort(Number(event.currentTarget.value))}
                  />
                </label>
              </div>
              <div class="buttonRow">
                <button class="primary" onClick={startRemoteControl} disabled={remoteRunning()}>
                  Start Remote
                </button>
                <button onClick={stopRemoteControl} disabled={!remoteRunning()}>
                  Stop Remote
                </button>
              </div>
            </div>
          </div>
          <EngineTelemetryPanel
            telemetry={snapshot().telemetry}
            budget={engineTelemetryReport()?.budget ?? null}
            onReset={resetEngineTelemetry}
            onSaveReport={saveEngineTelemetryReport}
          />
        </aside>
      </section>

      <footer>{message()}</footer>
    </main>
  );
}
