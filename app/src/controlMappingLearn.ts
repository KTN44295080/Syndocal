import type {
  LearnedMidiControl,
  LearnedOscControl,
  MidiControlAction,
  MidiControlMapping,
  OscControlAction,
  OscControlMapping,
  TouchControlBinding,
  VideoParam,
} from "./types";

export type ControlLearnMode = "midi" | "osc";

export interface ControlMappingTarget {
  action: MidiControlAction & OscControlAction;
  label: string;
  fixture_id?: number | null;
  attribute?: string | null;
  group_id?: string | null;
  cue_id?: number | null;
  layer_id?: number | null;
  output_id?: number | null;
  video_param?: VideoParam | null;
  cue_point_index?: number | null;
  duration_ms?: number | null;
  low?: number;
  high?: number;
}

type ControlMappingDataAttributes = {
  "data-control-map-target"?: string;
  "data-control-map-label"?: string;
};

const finiteOr = (value: number | undefined, fallback: number) =>
  Number.isFinite(value) ? Number(value) : fallback;

const rangeForTarget = (target: ControlMappingTarget): [number, number] => {
  if (target.action === "FixtureAttribute") return [0, 65_535];
  if (target.action === "SetBpm") return [20, 300];
  return [0, 1];
};

const normalizedTarget = (target: ControlMappingTarget): ControlMappingTarget => {
  const [defaultLow, defaultHigh] = rangeForTarget(target);
  return {
    ...target,
    label: target.label.trim() || target.action,
    low: finiteOr(target.low, defaultLow),
    high: finiteOr(target.high, defaultHigh),
  };
};

export const controlMappingTargetData = (
  target: ControlMappingTarget | readonly ControlMappingTarget[] | null | undefined,
): ControlMappingDataAttributes => {
  const targets = (Array.isArray(target) ? target : target ? [target] : [])
    .map(normalizedTarget);
  if (targets.length === 0) return {};
  return {
    "data-control-map-target": JSON.stringify(targets),
    "data-control-map-label": targets.length === 1
      ? targets[0].label
      : `${targets[0].label} +${targets.length - 1}`,
  };
};

export const controlMappingTargetsFromElement = (
  element: HTMLElement,
): ControlMappingTarget[] => {
  const encoded = element.dataset.controlMapTarget;
  if (!encoded) return [];
  try {
    const decoded = JSON.parse(encoded) as unknown;
    if (!Array.isArray(decoded)) return [];
    return decoded
      .filter((candidate): candidate is ControlMappingTarget =>
        Boolean(candidate)
        && typeof candidate === "object"
        && typeof (candidate as ControlMappingTarget).action === "string"
        && typeof (candidate as ControlMappingTarget).label === "string")
      .map(normalizedTarget);
  } catch {
    return [];
  }
};

export const controlMappingTargetKey = (target: ControlMappingTarget) => JSON.stringify({
  action: target.action,
  fixture_id: target.fixture_id ?? null,
  attribute: target.attribute ?? null,
  group_id: target.group_id ?? null,
  cue_id: target.cue_id ?? null,
  layer_id: target.layer_id ?? null,
  output_id: target.output_id ?? null,
  video_param: target.video_param ?? null,
  cue_point_index: target.cue_point_index ?? null,
  duration_ms: target.duration_ms ?? null,
});

const mappingFieldsForTarget = (target: ControlMappingTarget) => ({
  action: target.action,
  fixture_id: target.fixture_id ?? null,
  attribute: target.attribute ?? null,
  group_id: target.group_id ?? null,
  cue_id: target.cue_id ?? null,
  layer_id: target.layer_id ?? null,
  output_id: target.output_id ?? null,
  video_param: target.video_param ?? null,
  cue_point_index: target.cue_point_index ?? null,
  duration_ms: target.duration_ms ?? null,
  low: target.low ?? rangeForTarget(target)[0],
  high: target.high ?? rangeForTarget(target)[1],
});

export const midiMappingsFromLearnedControl = (
  targets: readonly ControlMappingTarget[],
  learned: LearnedMidiControl,
): MidiControlMapping[] => targets.map((target) => ({
  channel: learned.channel,
  message: learned.message,
  number: learned.number,
  ...mappingFieldsForTarget(normalizedTarget(target)),
}));

export const oscMappingsFromLearnedControl = (
  targets: readonly ControlMappingTarget[],
  learned: LearnedOscControl,
): OscControlMapping[] => targets.map((target) => ({
  address: learned.address.startsWith("/") ? learned.address : `/${learned.address}`,
  ...mappingFieldsForTarget(normalizedTarget(target)),
}));

export const sameMidiSource = (mapping: MidiControlMapping, learned: LearnedMidiControl) =>
  mapping.message === learned.message
  && mapping.number === learned.number
  && (mapping.channel ?? null) === learned.channel;

export const sameOscSource = (mapping: OscControlMapping, learned: LearnedOscControl) =>
  mapping.address.replace(/\/+$/g, "") === learned.address.replace(/\/+$/g, "");

export const controlMappingTargetsForTouchBinding = (
  binding: TouchControlBinding | null | undefined,
  label: string,
  selectedFixtureId: number | null | undefined,
): ControlMappingTarget[] => {
  if (!binding) return [];
  switch (binding.kind) {
    case "fixture_attribute":
      return [{ action: "FixtureAttribute", fixture_id: binding.fixture_id, attribute: binding.attribute, label }];
    case "selected_fixture_attribute":
      return selectedFixtureId === null || selectedFixtureId === undefined
        ? []
        : [{ action: "FixtureAttribute", fixture_id: selectedFixtureId, attribute: binding.attribute, label }];
    case "feature_preset":
      return binding.targets.map((target) => ({
        action: "FixtureAttribute",
        fixture_id: target.fixture_id,
        attribute: target.attribute,
        low: binding.inverted ? binding.max_value : binding.min_value,
        high: binding.inverted ? binding.min_value : binding.max_value,
        label: `${label} · Fixture ${target.fixture_id} ${target.attribute}`,
      }));
    case "cue":
      return [{ action: "TriggerCue", cue_id: binding.cue_id, label }];
    case "group_submaster":
      return [{ action: "GroupSubmaster", group_id: binding.group_id, label }];
    case "tap_tempo":
      return [{ action: "TapBpm", label }];
    case "lighting_master":
      return [{ action: "LightingMaster", label }];
    case "video_master":
      return [{ action: "VideoMaster", label }];
    case "blackout":
      return [{ action: "Blackout", label }];
    case "video_blackout":
      return [{ action: "VideoBlackout", label }];
    case "all_blackout":
      return [{ action: "AllBlackout", label }];
    case "cue_next":
      return [{ action: "TriggerNextCue", label }];
    case "cue_previous":
      return [{ action: "TriggerPreviousCue", label }];
    case "cue_fade_pause":
      return [{ action: "CueFadePause", label }];
    default:
      return [];
  }
};
