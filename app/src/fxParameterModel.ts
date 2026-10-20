import type { VideoEffectCatalog, VideoEffectPresetSummary, VideoIsfControlSummary, VideoIsfEffectStageSummary, VideoIsfEffectSummary, VideoIsfTempoSync } from "./types";
import { builtinFxEntries, readIsfMetadata } from "./fxCatalogModel";

export type FxValue = [number, number, number, number];
type IsfInput = { NAME?: string; LABEL?: string; TYPE?: string };
const normalizedSource = (source = "") => source.replace(/\r\n/g, "\n");
const builtinLabel = (stage?: VideoIsfEffectStageSummary) => stage
  ? builtinFxEntries.find((entry) => normalizedSource(entry.source) === normalizedSource(stage.source))?.label : undefined;

export function cloneFxStage(stage: VideoIsfEffectStageSummary): VideoIsfEffectStageSummary {
  return { enabled: stage.enabled, label: stage.label, source: stage.source,
    ...(stage.source_path !== undefined ? { source_path: stage.source_path } : {}),
    ...(stage.image_bindings !== undefined ? { image_bindings: stage.image_bindings.map((binding) => ({ name: binding.name, source: { ...binding.source } })) } : {}),
    ...(stage.description !== undefined ? { description: stage.description } : {}),
    categories: [...(stage.categories ?? [])], controls: stage.controls.map((control) => ({ ...control,
      value: [...control.value] as FxValue, default: [...control.default] as FxValue,
      minimum: [...control.minimum] as FxValue, maximum: [...control.maximum] as FxValue,
      labels: [...control.labels], values: [...control.values],
      // A copy, so a store proxy or a draft never shares the sync object with the chain it came from.
      ...(control.tempo_sync ? { tempo_sync: { unit: control.tempo_sync.unit, count: control.tempo_sync.count } } : {}),
    })) };
}

/** Read presentation metadata only. Validated runtime summaries own ranges and values. */
export function fxInputMetadata(source = ""): IsfInput[] {
  const inputs = readIsfMetadata(source).INPUTS;
  return Array.isArray(inputs) ? inputs.filter((input): input is IsfInput =>
    input != null && typeof input === "object" && typeof input.NAME === "string") : [];
}

export function fxControlLabel(control: VideoIsfControlSummary, source: string): string {
  const label = fxInputMetadata(source).find((input) => input.NAME === control.name)?.LABEL;
  return typeof label === "string" && label.trim() ? label.trim()
    : control.name.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/[_-]+/g, " ").replace(/^./, (letter) => letter.toUpperCase());
}

export const fxComponentCount = (control: VideoIsfControlSummary) =>
  control.kind === "Color" ? 4 : control.kind === "Point2d" ? 2 : 1;

export function boundedFxValue(control: VideoIsfControlSummary, value: readonly number[]): FxValue {
  const next = [...control.value] as FxValue;
  for (let component = 0; component < fxComponentCount(control); component++) {
    const minimum = control.minimum[component];
    const maximum = control.maximum[component];
    if (!Number.isFinite(value[component]) || !Number.isFinite(minimum) || !Number.isFinite(maximum) || minimum > maximum) continue;
    next[component] = Math.max(minimum, Math.min(maximum, value[component]));
    if (control.kind === "Bool") next[component] = next[component] >= 0.5 ? 1 : 0;
    if (control.kind === "Long") {
      const choices = control.values.filter((choice) => choice >= minimum && choice <= maximum);
      next[component] = choices.length ? choices.reduce((closest, choice) =>
        Math.abs(choice - next[component]) < Math.abs(closest - next[component]) ? choice : closest)
        : Math.max(Math.ceil(minimum), Math.min(Math.floor(maximum), Math.round(next[component])));
    }
  }
  return next;
}

const isBuiltinAngle = (control: VideoIsfControlSummary, stage?: VideoIsfEffectStageSummary) => {
  const builtin = builtinLabel(stage);
  return (builtin === "Rotate" && control.name === "angle") || (builtin === "Kaleidoscope" && control.name === "rotation");
};

/** Typed values for a control use its display unit: degrees for known built-in angles, else the raw value. */
export const fxDisplayScale = (control: VideoIsfControlSummary, stage?: VideoIsfEffectStageSummary): number =>
  isBuiltinAngle(control, stage) ? 180 / Math.PI : 1;

/** Units are inferred only for built-in controls whose shader math is known. */
export function fxControlValueText(control: VideoIsfControlSummary, stage?: VideoIsfEffectStageSummary, value = control.value): string {
  const number = (value: number) => String(Number(value.toFixed(3)));
  const builtin = builtinLabel(stage);
  if (control.kind === "Event") return "";
  if (control.kind === "Bool") return value[0] >= 0.5 ? "On" : "Off";
  if (control.kind === "Long" && control.values.length) return control.labels[control.values.indexOf(value[0])] ?? number(value[0]);
  if (control.kind === "Point2d") return `${number(value[0])}, ${number(value[1])}`;
  if (control.kind === "Color") {
    const hex = value.slice(0, 3).map((component) => Math.round(Math.max(0, Math.min(1, component)) * 255).toString(16).padStart(2, "0")).join("").toUpperCase();
    return `#${hex} · ${Math.round(Math.max(0, Math.min(1, value[3])) * 100)} %`;
  }
  if (isBuiltinAngle(control, stage)) return `${Number((value[0] * 180 / Math.PI).toFixed(1))} °`;
  if ((builtin === "Strobe" && control.name === "rate") || (builtin === "Glitch Shift" && control.name === "speed")) return `${number(value[0])} Hz`;
  if (builtin === "Zoom" && control.name === "zoom") return `${number(value[0])} ×`;
  if (control.kind === "Float" && control.minimum[0] === 0 && control.maximum[0] === 1) return `${Number((value[0] * 100).toFixed(1))} %`;
  return number(value[0]);
}

export function randomizeFxStage(stage: VideoIsfEffectStageSummary, random: () => number = Math.random): VideoIsfEffectStageSummary {
  return { ...stage, controls: stage.controls.map((control) => {
    // A tempo-synced control follows the clock, so Randomize leaves it alone.
    if (control.kind === "Event" || control.tempo_sync) return { ...control, value: [...control.value] as FxValue };
    const next = [...control.value] as FxValue;
    const unit = () => Math.max(0, Math.min(1, random()));
    for (let component = 0; component < fxComponentCount(control); component++) {
      next[component] = control.minimum[component] + unit() * (control.maximum[component] - control.minimum[component]);
    }
    if (control.kind === "Long" && control.values.length) next[0] = control.values[Math.min(control.values.length - 1, Math.floor(unit() * control.values.length))];
    return { ...control, value: boundedFxValue(control, next) };
  }) };
}

export function replaceFxStage(effect: VideoIsfEffectSummary, index: number, replacement: VideoIsfEffectStageSummary): VideoIsfEffectSummary {
  const stages = [effect, ...(effect.stack ?? [])].map((stage) => {
    const { stack: _stack, ...single } = stage as VideoIsfEffectSummary;
    return single;
  });
  if (index < 0 || index >= stages.length || normalizedSource(stages[index].source) !== normalizedSource(replacement.source)) return effect;
  stages[index] = replacement;
  return { ...stages[0], stack: stages.slice(1) };
}

export function saveFxStagePreset(catalog: VideoEffectCatalog, stage: VideoIsfEffectStageSummary, label: string): VideoEffectCatalog {
  const trimmed = label.trim();
  if (!trimmed) return catalog;
  return { ...catalog, effect_presets: [...catalog.effect_presets, { id: 0, label: trimmed.slice(0, 120), payload: {
    bypassed: false, stages: [{ enabled: stage.enabled, label: stage.label, effect: { kind: "isf", effect: { ...cloneFxStage(stage), stack: [] } } }],
  } }] };
}

export function matchingFxStagePresets(catalog: VideoEffectCatalog | undefined, stage: VideoIsfEffectStageSummary): VideoEffectPresetSummary[] {
  return (catalog?.effect_presets ?? []).filter((preset) => preset.payload.stages.length === 1 && normalizedSource(preset.payload.stages[0].effect.effect.source) === normalizedSource(stage.source));
}

const recipes: Record<string, [string, Record<string, number | FxValue>][]> = {
  Monochrome: [["Subtle", { amount: 0.35 }], ["Black and white", { amount: 1 }]],
  Threshold: [["Soft contrast", { level: 0.45, softness: 0.2 }], ["Hard silhouette", { level: 0.6, softness: 0.02 }]],
  Posterize: [["Graphic", { levels: 4 }], ["Fine steps", { levels: 12 }]],
  "RGB Split": [["Subtle", { amount: 0.008 }], ["Wide split", { amount: 0.06 }]],
  Mirror: [["Horizontal", { mode: 0 }], ["Both", { mode: 2 }]],
  Kaleidoscope: [["Four facets", { segments: 4, rotation: 0 }], ["Twelve facets", { segments: 12, rotation: 0.25 }]],
  Zoom: [["Close up", { zoom: 2 }], ["Wide view", { zoom: 0.75 }]],
  Rotate: [["Quarter turn", { angle: Math.PI / 2 }], ["Half turn", { angle: Math.PI }]],
  Strobe: [["Slow pulse", { rate: 1, duty: 0.7 }], ["Short pulse", { rate: 2, duty: 0.2 }]],
  Scanlines: [["Fine lines", { density: 360, amount: 0.2 }], ["Bold lines", { density: 90, amount: 0.65 }]],
  Colorize: [["Cool tint", { tint: [0.1, 0.7, 1, 1], amount: 0.65 }], ["Warm tint", { tint: [1, 0.35, 0.1, 1], amount: 0.8 }]],
  Vignette: [["Subtle", { amount: 0.3, softness: 0.6 }], ["Spotlight", { amount: 0.95, softness: 0.25 }]],
  "Glitch Shift": [["Subtle", { amount: 0.03, bands: 24, speed: 4 }], ["Wide slices", { amount: 0.2, bands: 8, speed: 12 }]],
};

export function builtinFxStagePresets(stage: VideoIsfEffectStageSummary): { label: string; stage: VideoIsfEffectStageSummary }[] {
  const builtin = builtinLabel(stage);
  if (!builtin) return [];
  return (recipes[builtin] ?? []).map(([label, values]) => ({ label, stage: { ...stage,
    controls: stage.controls.map((control) => {
      const value = values[control.name];
      const recalled = { ...control, value: boundedFxValue(control, value === undefined ? control.default : typeof value === "number" ? [value, ...control.default.slice(1)] : value) };
      // A recipe value is a raw number, so it replaces any tempo sync on that control.
      if (value !== undefined) delete recalled.tempo_sync;
      return recalled;
    }),
  } }));
}

/* ---- Tempo sync (beats and bars) --------------------------------------------------
 * A time control (rate, speed, frequency) can be expressed as one cycle per 1/4, 1/2, 1,
 * 2 or 4 beats or bars of the global tempo. The choice is authored project state
 * (`tempo_sync` on the control). The runtime and editor derive the effective value
 * from the current engine BPM without publishing changes to the authored value. */

export const FX_TEMPO_SYNC_COUNTS = [0.25, 0.5, 1, 2, 4] as const;
export const FX_BEATS_PER_BAR = 4;
const TIME_CONTROL_WORDS = new Set(["rate", "speed", "freq", "frequency", "hz", "lfo", "strobe"]);

/** Built-ins list the controls whose shader maths is known to be cycles per second. Other shaders
 * qualify only when a Float control is named like a rate, speed, frequency or LFO. */
export function fxIsTimeControl(control: VideoIsfControlSummary, stage?: VideoIsfEffectStageSummary): boolean {
  if (control.kind !== "Float" || !(control.maximum[0] > control.minimum[0]) || control.minimum[0] < 0) return false;
  const builtin = builtinLabel(stage);
  if (builtin) return (builtin === "Strobe" && control.name === "rate") || (builtin === "Glitch Shift" && control.name === "speed");
  return control.name.replace(/([a-z])([A-Z])/g, "$1 $2").toLowerCase().split(/[^a-z0-9]+/).some((word) => TIME_CONTROL_WORDS.has(word));
}

export const fxTempoSyncPeriodBeats = (sync: VideoIsfTempoSync): number => sync.count * (sync.unit === "bars" ? FX_BEATS_PER_BAR : 1);
export const fxValidBpm = (bpm: unknown): bpm is number => typeof bpm === "number" && Number.isFinite(bpm) && bpm > 0;

/** Cycles per second of one cycle per `sync` at `bpm`; NaN when the tempo is unusable. */
export const fxTempoSyncHz = (bpm: number | undefined, sync: VideoIsfTempoSync): number =>
  fxValidBpm(bpm) && (sync.unit === "beats" || sync.unit === "bars")
    && FX_TEMPO_SYNC_COUNTS.some((count) => count === sync.count)
    ? bpm / 60 / fxTempoSyncPeriodBeats(sync) : Number.NaN;

export interface FxTempoSyncState {
  target: number;
  /** The tempo asks for a value outside the control's range, so the range limit is used. */
  clamped: boolean;
}

/** Live nominal rate; a stopped transport keeps its last valid engine BPM.
 * Missing/invalid tempo leaves the authored value in use, as in the renderer. */
export function fxTempoSyncState(control: VideoIsfControlSummary, bpm: number | undefined, sync = control.tempo_sync): FxTempoSyncState | null {
  if (!sync || control.kind !== "Float") return null;
  const hz = fxTempoSyncHz(bpm, sync);
  if (!Number.isFinite(hz)) return null;
  const minimum = Math.min(control.minimum[0], control.maximum[0]);
  const maximum = Math.max(control.minimum[0], control.maximum[0]);
  const target = Math.max(minimum, Math.min(maximum, hz));
  return { target, clamped: target !== hz };
}

/** Presentation-only derivation: no stage mutation or project publication. */
export function effectiveFxValue(control: VideoIsfControlSummary, bpm: number | undefined): FxValue {
  const state = fxTempoSyncState(control, bpm);
  return state ? [state.target, control.value[1], control.value[2], control.value[3]] : control.value;
}

/** Explicit user edit: changing units stores the current rate, including when leaving sync. */
export function withFxTempoSync(control: VideoIsfControlSummary, sync: VideoIsfTempoSync | null, bpm: number | undefined): VideoIsfControlSummary {
  const next: VideoIsfControlSummary = { ...control, value: [...effectiveFxValue(control, bpm)] as FxValue };
  if (!sync) { delete next.tempo_sync; return next; }
  next.tempo_sync = { unit: sync.unit, count: sync.count };
  const state = fxTempoSyncState(next, bpm);
  if (state) next.value[0] = state.target;
  return next;
}

/** Capture the current rate during explicit preset recall; never run on clock updates. */
export function resyncFxStage(stage: VideoIsfEffectStageSummary, bpm: number | undefined): VideoIsfEffectStageSummary {
  return { ...stage, controls: stage.controls.map((control) => control.tempo_sync ? withFxTempoSync(control, control.tempo_sync, bpm) : control) };
}

export const fxTempoSyncCountText = (count: number): string => count === 0.25 ? "1/4" : count === 0.5 ? "1/2" : String(count);

/** "1/4 beat", "2 beats", "1/2 bar", "4 bars" (Japanese: 拍, 小節). */
export function fxTempoSyncLabel(sync: VideoIsfTempoSync, locale?: string): string {
  const ja = locale === "ja";
  const plural = sync.count > 1;
  return `${fxTempoSyncCountText(sync.count)}${ja ? "" : " "}${sync.unit === "bars" ? (ja ? "小節" : plural ? "bars" : "bar") : (ja ? "拍" : plural ? "beats" : "beat")}`;
}
