// Perform control targets that MIDI Learn and beat-division assignment can drive.
//
// The native input worker only forwards an accepted input for these actions; the
// renderer then runs the very command the on-screen control runs (queue, Take,
// stop, blend, transition, effect stage). Targets use stable layer, clip-slot
// and effect-stage ids, never list positions, so reordering cannot retarget a
// mapping. Nothing here talks to the runtime; `performControlDispatcher.ts`
// owns that boundary.
import type { MidiControlMapping, VideoBlendMode, VideoClipTakeKind } from "./types";

export const performRoutedActions = [
  "VideoClipSlotTrigger",
  "VideoTake",
  "VideoLayerStop",
  "VideoLayerBlendMode",
  "VideoTransitionKind",
  "VideoTransitionTime",
  "VideoIsfStageEnabled",
  "VideoIsfParam",
] as const;
export type PerformRoutedAction = (typeof performRoutedActions)[number];

/** Mirrors `MidiControlAction::is_renderer_routed` in the protocol crate. */
export const isPerformRoutedAction = (action: string): action is PerformRoutedAction =>
  (performRoutedActions as readonly string[]).includes(action);

/** Actions that fire once per trigger; only these accept a beat division
 * (mirrors `is_renderer_routed_trigger`). */
export const performTriggerActions: ReadonlySet<string> = new Set<PerformRoutedAction>([
  "VideoClipSlotTrigger",
  "VideoTake",
  "VideoLayerStop",
  "VideoIsfStageEnabled",
]);
export const isPerformTriggerAction = (action: string) => performTriggerActions.has(action);

export const performBlendModes: readonly VideoBlendMode[] = [
  "Normal", "Add", "Multiply", "Screen", "Overlay", "Subtract", "Difference", "Lighten", "Darken", "SoftLight",
];
export const performTransitionKinds: readonly VideoClipTakeKind[] = [
  "Cut", "Crossfade", "Dip", "Wipe", "Luma", "Displacement", "Blur", "Glitch", "Custom",
];

/** Position 0..1 -> index of an `count`-entry selector (the last entry owns 1.0). */
export const selectorIndex = (position: number, count: number): number => {
  if (count <= 0 || !Number.isFinite(position)) return 0;
  return Math.max(0, Math.min(count - 1, Math.floor(Math.max(0, Math.min(1, position)) * count)));
};

/** Mirrors `BEAT_DIVISION_BEATS` in the protocol crate; the mapping number
 * indexes this table. */
export const beatDivisions = [
  { number: 0, beats: 0.25, label: "1/4 beat" },
  { number: 1, beats: 0.5, label: "1/2 beat" },
  { number: 2, beats: 1, label: "1 beat" },
  { number: 3, beats: 2, label: "2 beats" },
  { number: 4, beats: 4, label: "1 bar" },
  { number: 5, beats: 8, label: "2 bars" },
  { number: 6, beats: 16, label: "4 bars" },
  { number: 7, beats: 32, label: "8 bars" },
] as const;

export const beatDivisionLabel = (number: number): string =>
  beatDivisions.find((division) => division.number === number)?.label ?? `Beat ${number}`;

/** The largest advance of the beat position that still counts as playing on: a
 * longer one is a clock restart, a seek or a pause, and fires nothing. */
export const beatResyncGapBeats = 4;

/** A beat boundary crossed while the beat position moved from `previous` to
 * `next`: at most one, so a stall or a coarse update never produces a burst.
 * A jump backwards or a gap over `beatResyncGapBeats` (clock reset, tap, pause)
 * is a resynchronization and fires nothing. */
export const beatDivisionCrossed = (previous: number, next: number, beats: number): boolean => {
  if (![previous, next, beats].every(Number.isFinite) || beats <= 0) return false;
  if (next <= previous || next - previous > beatResyncGapBeats) return false;
  return Math.floor(next / beats) > Math.floor(previous / beats);
};

/** One accepted input for a Perform action, as the native worker forwards it
 * or the beat scheduler synthesizes it. */
export interface PerformRoutedInput {
  action: PerformRoutedAction;
  layer_id?: number | null;
  clip_slot_id?: number | null;
  isf_stage_id?: number | null;
  /** ISF control name for `VideoIsfParam`. */
  control?: string | null;
  /** Input position 0..1 before the mapping range. */
  position: number;
  /** The mapping's `low..high` range applied to the position. */
  value: number;
  active: boolean;
  /** A press that flips the target instead of setting it to `active`. */
  toggle?: boolean;
  /** Identifies the transport; MIDI preserves the existing omitted source. */
  source?: "beat" | "osc" | "dmx" | "audio";
  audioMappingId?: string;
  /** Renderer-only source lease; never decoded from a native payload. */
  sourceIsCurrent?: () => boolean;
}

const positiveInteger = (value: unknown): value is number =>
  typeof value === "number" && Number.isSafeInteger(value) && value > 0;
const optionalId = (value: unknown): number | null | undefined =>
  value === undefined || value === null ? null : positiveInteger(value) ? value : undefined;

/** Strict decode of the native `RendererRoutedInput`; anything else is rejected. */
export function performRoutedInputFromPayload(value: unknown): PerformRoutedInput | null {
  if (!value || typeof value !== "object") return null;
  const raw = value as Record<string, unknown>;
  if (typeof raw.action !== "string" || !isPerformRoutedAction(raw.action)) return null;
  const layer = optionalId(raw.layer_id);
  const slot = optionalId(raw.clip_slot_id);
  const stage = optionalId(raw.isf_stage_id);
  if (layer === undefined || slot === undefined || stage === undefined) return null;
  if (raw.control !== undefined && raw.control !== null
    && (typeof raw.control !== "string" || raw.control.length === 0 || raw.control.length > 128)) return null;
  if (typeof raw.position !== "number" || !Number.isFinite(raw.position)) return null;
  if (typeof raw.value !== "number" || !Number.isFinite(raw.value)) return null;
  if (typeof raw.active !== "boolean") return null;
  if (raw.toggle !== undefined && typeof raw.toggle !== "boolean") return null;
  return {
    action: raw.action,
    layer_id: layer,
    clip_slot_id: slot,
    isf_stage_id: stage,
    control: typeof raw.control === "string" ? raw.control : null,
    position: raw.position,
    value: raw.value,
    active: raw.active,
    toggle: raw.toggle === true,
  };
}

/** The input a beat-division mapping fires on each boundary. */
export function performRoutedInputFromBeatMapping(mapping: MidiControlMapping): PerformRoutedInput | null {
  if (mapping.message !== "BeatDivision" || !isPerformTriggerAction(mapping.action)) return null;
  const action = mapping.action as PerformRoutedAction;
  return {
    action,
    layer_id: mapping.layer_id ?? null,
    clip_slot_id: mapping.clip_slot_id ?? null,
    isf_stage_id: mapping.isf_stage_id ?? null,
    control: null,
    position: 1,
    value: 1,
    active: true,
    // A beat flips a stage once per boundary.
    toggle: action === "VideoIsfStageEnabled",
    source: "beat",
  };
}

/** The renderer sees the engine clock only as often as it applies a snapshot
 * (every 250 ms). Between snapshots the beat position is extrapolated at the
 * snapshot's tempo for at most this long, so a stalled poll cannot run ahead. */
export const beatExtrapolationLimitMs = 750;

export interface BeatClockObservation {
  bpm: number;
  beat_counter: number;
  beat_phase: number;
}

/** Fires the mapped trigger on each beat boundary of the clock. The scheduler
 * keeps one previous position and never fires for a resynchronization.
 * `observe` takes each applied snapshot's clock; `advance` (called from a short
 * timer) extrapolates between snapshots so a division fires within a timer
 * period of its boundary instead of a whole snapshot period late. */
export function createBeatTriggerScheduler(options: {
  mappings: () => readonly MidiControlMapping[];
  dispatch: (input: PerformRoutedInput) => unknown;
  /** Suspends firing, for example while a Learn mode is editing the show. */
  suspended?: () => boolean;
  /** Monotonic milliseconds; the default is `performance.now`. */
  now?: () => number;
}) {
  const now = options.now ?? (() => performance.now());
  let previous: number | null = null;
  let anchor: { position: number; at: number; perMs: number } | null = null;
  let lastClock: string | null = null;
  return {
    /** Takes the clock of a newly applied snapshot. A snapshot that repeats the
     * clock it already had (the UI rewrites the object for unrelated state)
     * carries no news and must not move the extrapolation anchor forward. */
    observe(clock: BeatClockObservation | null | undefined): number {
      if (!clock || !Number.isFinite(clock.beat_counter) || !Number.isFinite(clock.beat_phase)) {
        anchor = null;
        previous = null;
        lastClock = null;
        return 0;
      }
      const signature = `${clock.beat_counter}:${clock.beat_phase}:${clock.bpm}`;
      if (signature === lastClock && anchor) return 0;
      lastClock = signature;
      const bpm = Number.isFinite(clock.bpm) && clock.bpm > 0 && clock.bpm <= 1000 ? clock.bpm : 0;
      anchor = { position: clock.beat_counter + clock.beat_phase, at: now(), perMs: bpm / 60_000 };
      return this.tick(anchor.position);
    },
    /** Extrapolates the observed clock to now. */
    advance(): number {
      if (!anchor) return 0;
      const elapsed = Math.max(0, Math.min(beatExtrapolationLimitMs, now() - anchor.at));
      return this.tick(anchor.position + elapsed * anchor.perMs);
    },
    /** Feed the current beat position (`beat_counter + beat_phase`). */
    tick(position: number | null | undefined): number {
      if (position === null || position === undefined || !Number.isFinite(position)) {
        previous = null;
        return 0;
      }
      const before = previous;
      // A small step backwards is clock jitter between counter and phase;
      // keep the later position. Anything larger is a restart.
      previous = before !== null && position < before && before - position < 1 ? before : position;
      if (before === null || options.suspended?.()) return 0;
      const due: PerformRoutedInput[] = [];
      for (const mapping of options.mappings()) {
        if (mapping.message !== "BeatDivision") continue;
        const beats = beatDivisions[mapping.number]?.beats;
        if (beats === undefined || !beatDivisionCrossed(before, position, beats)) continue;
        const input = performRoutedInputFromBeatMapping(mapping);
        if (input) due.push(input);
      }
      // A boundary that queues a clip and takes it must queue first, whatever
      // order the two mappings were made in.
      due.sort((a, b) => Number(a.action === "VideoTake") - Number(b.action === "VideoTake"));
      for (const input of due) options.dispatch(input);
      return due.length;
    },
    reset() { previous = null; anchor = null; lastClock = null; },
  };
}

/** A short operator-facing description of what drives a mapping: the controller
 * message or the beat division. */
export const midiSourceDescription = (mapping: Pick<MidiControlMapping, "message" | "number" | "channel">): string => {
  if (mapping.message === "BeatDivision") return beatDivisionLabel(mapping.number);
  if (mapping.message === "Audio") return "Audio";
  const kind = mapping.message === "ControlChange" ? "CC"
    : mapping.message === "RelativeControlChange" ? "Encoder CC"
      : mapping.message === "NoteOn" ? "Note"
        : mapping.message === "NoteOff" ? "Note off"
          : mapping.message === "PitchBend" ? "Pitch bend" : "Program";
  const number = mapping.message === "PitchBend" ? "" : ` ${mapping.number}`;
  const channel = mapping.channel === null || mapping.channel === undefined ? "any channel" : `ch ${mapping.channel + 1}`;
  return `${kind}${number} · ${channel}`;
};

/** The stored action of a mapping created from a target. */
export const performMappingKey = (mapping: Pick<MidiControlMapping,
  "action" | "layer_id" | "clip_slot_id" | "isf_stage_id" | "attribute">): string => JSON.stringify([
  mapping.action,
  mapping.layer_id ?? null,
  mapping.clip_slot_id ?? null,
  mapping.isf_stage_id ?? null,
  mapping.action === "VideoIsfParam" ? mapping.attribute ?? null : null,
]);

