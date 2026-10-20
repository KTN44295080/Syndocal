import type {
  DmxControlMapping,
  LearnedDmxControl,
  LearnedMidiControl,
  LearnedOscControl,
  MidiControlAction,
  MidiControlMapping,
  OscControlAction,
  OscControlMapping,
  TouchControlBinding,
  VideoParam,
} from "./types";
import { isPerformTriggerAction, type PerformRoutedAction } from "./controlMappingPerform";

export type ControlLearnMode = "midi" | "osc" | "dmx";

/** Actions both MIDI and OSC/DMX mappings can carry. */
type SharedControlAction = MidiControlAction & OscControlAction;

export interface ControlMappingTarget {
  /** Perform targets share renderer dispatch across MIDI, OSC and DMX. */
  action: SharedControlAction | PerformRoutedAction;
  label: string;
  /** MIDI behavior may differ from one-shot OSC actions, e.g. a scene pad. */
  midi_action?: MidiControlAction;
  fixture_id?: number | null;
  attribute?: string | null;
  group_id?: string | null;
  cue_id?: number | null;
  layer_id?: number | null;
  output_id?: number | null;
  video_param?: VideoParam | null;
  cue_point_index?: number | null;
  duration_ms?: number | null;
  /** Stable clip-slot id (`VideoClipSlotTrigger`); never a column index. */
  clip_slot_id?: number | null;
  /** Stable effect-stage id (`VideoIsfStageEnabled` / `VideoIsfParam`). */
  isf_stage_id?: number | null;
  /** Stable video group id, never a strip position or lighting group id. */
  video_group_id?: number | null;
  /** Explicitly transport-specific targets may opt out of OSC/DMX. */
  midi_only?: boolean;
  low?: number;
  high?: number;
}

/** A target OSC and DMX mappings can carry. */
type SharedControlMappingTarget = ControlMappingTarget & { action: SharedControlAction };

/** Perform targets are available in every Learn mode. */
export const controlMappingTargetsForMode = (
  mode: ControlLearnMode,
  targets: readonly ControlMappingTarget[],
): ControlMappingTarget[] => mode === "midi"
  ? [...targets]
  : targets.filter((target) => !target.midi_only);

const sharedTargets = (targets: readonly ControlMappingTarget[]): SharedControlMappingTarget[] =>
  targets.filter((target): target is SharedControlMappingTarget =>
    !target.midi_only);

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

const normalizedTarget = <Target extends ControlMappingTarget>(target: Target): Target => {
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
  midi_action: target.midi_action ?? null,
  fixture_id: target.fixture_id ?? null,
  attribute: target.attribute ?? null,
  group_id: target.group_id ?? null,
  cue_id: target.cue_id ?? null,
  layer_id: target.layer_id ?? null,
  output_id: target.output_id ?? null,
  video_param: target.video_param ?? null,
  cue_point_index: target.cue_point_index ?? null,
  duration_ms: target.duration_ms ?? null,
  clip_slot_id: target.clip_slot_id ?? null,
  isf_stage_id: target.isf_stage_id ?? null,
  video_group_id: target.video_group_id ?? null,
});

const mappingFieldsForTarget = (target: SharedControlMappingTarget) => ({
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
  ...(target.clip_slot_id !== null && target.clip_slot_id !== undefined
    ? { clip_slot_id: target.clip_slot_id } : {}),
  ...(target.isf_stage_id !== null && target.isf_stage_id !== undefined
    ? { isf_stage_id: target.isf_stage_id } : {}),
  ...(target.video_group_id !== null && target.video_group_id !== undefined
    ? { video_group_id: target.video_group_id } : {}),
});

/** The persisted fields of a MIDI mapping. Perform ids are written only when the
 * target carries them, so every other mapping keeps its exact JSON shape. */
const midiMappingFieldsForTarget = (target: ControlMappingTarget): Omit<MidiControlMapping,
  "channel" | "message" | "number"> => {
  const normalized = normalizedTarget(target);
  const { action: _unusedAction, ...fields } = mappingFieldsForTarget(normalized as SharedControlMappingTarget);
  return {
    ...fields,
    action: normalized.midi_action ?? normalized.action,
    ...(normalized.clip_slot_id !== null && normalized.clip_slot_id !== undefined
      ? { clip_slot_id: normalized.clip_slot_id } : {}),
    ...(normalized.isf_stage_id !== null && normalized.isf_stage_id !== undefined
      ? { isf_stage_id: normalized.isf_stage_id } : {}),
  };
};

export const midiMappingsFromLearnedControl = (
  targets: readonly ControlMappingTarget[],
  learned: LearnedMidiControl,
): MidiControlMapping[] => targets.map((target) => ({
  channel: learned.channel,
  message: learned.message,
  number: learned.number,
  ...midiMappingFieldsForTarget(target),
  ...((target.midi_action === "ToggleCue" || target.midi_action === "FlashCue")
      && (learned.message === "NoteOn" || learned.message === "ControlChange")
    ? { feedback: {
        off: { message: learned.message, channel: learned.channel, number: learned.number, value: 0 },
        on: { message: learned.message, channel: learned.channel, number: learned.number, value: 127 },
      } } : {}),
}));

/** A tempo-driven mapping: the beat division fires the target without hardware.
 * Only Perform trigger targets accept it (the native validator enforces this). */
export const beatDivisionMappingsFromTargets = (
  targets: readonly ControlMappingTarget[],
  divisionNumber: number,
): MidiControlMapping[] => targets
  .filter((target) => isPerformTriggerAction(target.midi_action ?? target.action))
  .map((target) => ({
    channel: null,
    message: "BeatDivision" as const,
    number: divisionNumber,
    ...midiMappingFieldsForTarget(target),
  }));

/** Whether a stored mapping drives the given Learn target. Compares the stable
 * identity (action, layer, clip slot, effect stage, control), not the source. */
export const midiMappingDrivesTarget = (mapping: MidiControlMapping, target: ControlMappingTarget): boolean => {
  const fields = midiMappingFieldsForTarget(target);
  return mapping.action === fields.action
    && (mapping.fixture_id ?? null) === (fields.fixture_id ?? null)
    && (mapping.attribute ?? null) === (fields.attribute ?? null)
    && (mapping.group_id ?? null) === (fields.group_id ?? null)
    && (mapping.cue_id ?? null) === (fields.cue_id ?? null)
    && (mapping.layer_id ?? null) === (fields.layer_id ?? null)
    && (mapping.output_id ?? null) === (fields.output_id ?? null)
    && (mapping.video_param ?? null) === (fields.video_param ?? null)
    && (mapping.cue_point_index ?? null) === (fields.cue_point_index ?? null)
    && (mapping.clip_slot_id ?? null) === (fields.clip_slot_id ?? null)
    && (mapping.isf_stage_id ?? null) === (fields.isf_stage_id ?? null)
    && (mapping.video_group_id ?? null) === (fields.video_group_id ?? null);
};

export const oscMappingsFromLearnedControl = (
  targets: readonly ControlMappingTarget[],
  learned: LearnedOscControl,
): OscControlMapping[] => sharedTargets(targets).map((target) => ({
  address: learned.address.startsWith("/") ? learned.address : `/${learned.address}`,
  ...mappingFieldsForTarget(normalizedTarget(target)),
}));

export const dmxMappingsFromLearnedControl = (
  targets: readonly ControlMappingTarget[],
  learned: LearnedDmxControl,
): DmxControlMapping[] => sharedTargets(targets)
  .map((target) => ({
    universe: learned.universe,
    channel: learned.channel,
    ...mappingFieldsForTarget(normalizedTarget(target)),
  }));

/** Positional targets; relative encoders map only to these (mirrors Rust). */
export const midiContinuousActions: ReadonlySet<string> = new Set([
  "FixtureAttribute",
  "SelectedFeatureFader",
  "VideoParam",
  "VideoLayerFade",
  "VideoOutputOpacity",
  "VideoOutputFade",
  "VideoOutputMappingField",
  "TimelineSeek",
  "SetBpm",
  "LightingMaster",
  "GroupSubmaster",
  "VideoMaster",
]);

// A relative encoder is still the same physical CC.
const midiSourceMessage = (message: MidiControlMapping["message"]) =>
  message === "RelativeControlChange" ? "ControlChange" : message;

export const sameMidiSource = (mapping: MidiControlMapping, learned: LearnedMidiControl) =>
  midiSourceMessage(mapping.message) === midiSourceMessage(learned.message)
  && mapping.number === learned.number
  && (mapping.channel ?? null) === learned.channel;

export const sameOscSource = (mapping: OscControlMapping, learned: LearnedOscControl) =>
  mapping.address.replace(/\/+$/g, "") === learned.address.replace(/\/+$/g, "");

export const sameDmxSource = (mapping: DmxControlMapping, learned: LearnedDmxControl) =>
  mapping.universe === learned.universe && mapping.channel === learned.channel;

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
