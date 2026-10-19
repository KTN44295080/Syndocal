// Runs an accepted Perform input (a MIDI mapping or a beat division) through the
// renderer's own ticketed commands: the same functions the on-screen pad,
// Take button, selects and effect controls call. There is no second command
// path; this module only resolves stable ids against the live project and
// serializes the calls.
import {
  performBlendModes,
  performTransitionKinds,
  selectorIndex,
  type PerformRoutedInput,
} from "./controlMappingPerform";
import type {
  VideoBlendMode,
  VideoClipLayerRuntimeSummary,
  VideoClipTakeKind,
  VideoEffectCatalog,
  VideoIsfControlSummary,
  VideoIsfEffectStageSummary,
  VideoLayerSummary,
} from "./types";

/** Layer-scope effect chains keep one stable stage id per stage, in the same
 * order as the layer's `isf_effect` stack. */
export function isfStageIndexForId(
  catalog: VideoEffectCatalog | undefined,
  layerId: number,
  stageId: number,
): number {
  const chain = catalog?.effect_chains.find((candidate) =>
    candidate.scope.scope === "layer" && candidate.scope.layer_id === layerId);
  return chain?.stages.findIndex((stage) => stage.id === stageId) ?? -1;
}

export function isfStageIdForIndex(
  catalog: VideoEffectCatalog | undefined,
  layerId: number,
  index: number,
): number | null {
  const chain = catalog?.effect_chains.find((candidate) =>
    candidate.scope.scope === "layer" && candidate.scope.layer_id === layerId);
  return chain?.stages[index]?.id ?? null;
}

export const layerIsfStages = (layer: VideoLayerSummary): VideoIsfEffectStageSummary[] =>
  layer.isf_effect ? [layer.isf_effect, ...(layer.isf_effect.stack ?? [])] : [];

/** ISF controls a single hardware control can drive. */
export const isfControlIsMappable = (control: VideoIsfControlSummary): boolean =>
  control.kind === "Float" || control.kind === "Long" || control.kind === "Bool" || control.kind === "Event";

const clamp01 = (value: number) => Math.max(0, Math.min(1, value));

/** The value a Float/Long/Bool control takes at `fraction` of its range. */
export function isfControlValueAt(control: VideoIsfControlSummary, fraction: number): number | null {
  const minimum = control.minimum[0];
  const maximum = control.maximum[0];
  if (!Number.isFinite(minimum) || !Number.isFinite(maximum) || minimum > maximum) return null;
  const position = clamp01(fraction);
  if (control.kind === "Bool") return position >= 0.5 ? 1 : 0;
  if (control.kind === "Long") {
    const choices = control.values.filter((choice) => choice >= minimum && choice <= maximum);
    if (choices.length > 0) return choices[selectorIndex(position, choices.length)];
    return Math.round(minimum + (maximum - minimum) * position);
  }
  if (control.kind === "Float") return minimum + (maximum - minimum) * position;
  return null;
}

export type PerformDispatchOutcome =
  | { handled: true }
  | { handled: false; reason: "missing-layer" | "missing-slot" | "missing-stage" | "missing-control"
    | "nothing-queued" | "transition-running" | "unchanged" | "unsupported" };

export interface PerformControlDispatcherDeps {
  layers: () => readonly VideoLayerSummary[];
  effectCatalog: () => VideoEffectCatalog | undefined;
  clipRuntime: (layerId: number) => VideoClipLayerRuntimeSummary | null;
  /** The selected layer's Take can run: a clip is queued or a transition is live. */
  takeEnabled: () => boolean;
  selectLayer: (layerId: number) => void;
  selectSlot: (slotId: number | null) => void;
  queueSlot: (layerId: number, slotId: number) => Promise<unknown>;
  take: () => Promise<unknown>;
  stopLayer: (layerId: number) => Promise<unknown>;
  setBlendMode: (layerId: number, mode: VideoBlendMode) => Promise<unknown> | unknown;
  transitionKind: () => VideoClipTakeKind;
  transitionDurationMilliunits: () => number;
  /** Take transition changes are locked while the selected layer is mid-transition. */
  transitionLocked: () => boolean;
  setTransitionKind: (kind: VideoClipTakeKind) => void;
  setTransitionDurationMilliunits: (milliunits: number) => void;
  setIsfStageEnabled: (layerId: number, index: number, enabled: boolean) => Promise<unknown> | unknown;
  setIsfControl: (
    layerId: number,
    index: number,
    name: string,
    value: [number, number, number, number],
  ) => Promise<unknown> | unknown;
  triggerIsfEvent: (layerId: number, index: number, name: string) => Promise<unknown> | unknown;
  report?: (message: string) => void;
}

/** Discrete triggers run in arrival order on one lane (a pad press must reach the
 * runtime before the Take that follows it); a continuous control keeps only its
 * newest value while an earlier one is still in flight. */
function createLanes() {
  const ordered = new Map<string, Promise<unknown>>();
  const latest = new Map<string, { running: boolean; pending: (() => Promise<unknown> | unknown) | null }>();
  return {
    ordered<T>(key: string, task: () => Promise<T> | T): Promise<T | undefined> {
      const previous = ordered.get(key) ?? Promise.resolve();
      const run = previous.then(() => task());
      // A failed command reports itself; it must not stall the lane.
      ordered.set(key, run.then(() => undefined, () => undefined));
      return run.catch(() => undefined);
    },
    latest(key: string, task: () => Promise<unknown> | unknown): Promise<unknown> {
      const lane = latest.get(key) ?? { running: false, pending: null };
      latest.set(key, lane);
      if (lane.running) {
        lane.pending = task;
        return Promise.resolve();
      }
      const run = async () => {
        lane.running = true;
        let current: (() => Promise<unknown> | unknown) | null = task;
        try {
          while (current) {
            lane.pending = null;
            try { await current(); } catch { /* the command reports its own failure */ }
            current = lane.pending;
          }
        } finally {
          lane.running = false;
        }
      };
      return run();
    },
  };
}

export function createPerformControlDispatcher(deps: PerformControlDispatcherDeps) {
  const lanes = createLanes();
  /** Layers this dispatcher queued whose runtime poll has not caught up yet,
   * with the time of the queue. The window is short: it only bridges the
   * poll, it never keeps a stale queue alive. */
  const justQueued = new Map<number, number>();
  const queuedWindowMs = 1500;
  const queuedRecently = () => {
    const now = Date.now();
    for (const [layerId, at] of justQueued) if (now - at > queuedWindowMs) justQueued.delete(layerId);
    return justQueued.size > 0;
  };

  const layerById = (layerId: number | null | undefined) =>
    layerId === null || layerId === undefined ? undefined : deps.layers().find((layer) => layer.id === layerId);

  type ResolvedStage =
    | { ok: false; reason: "missing-layer" | "missing-stage" }
    | { ok: true; layer: VideoLayerSummary; index: number; stage: VideoIsfEffectStageSummary };
  const resolveStage = (input: PerformRoutedInput): ResolvedStage => {
    const layer = layerById(input.layer_id);
    if (!layer) return { ok: false, reason: "missing-layer" };
    if (input.isf_stage_id === null || input.isf_stage_id === undefined) return { ok: false, reason: "missing-stage" };
    const index = isfStageIndexForId(deps.effectCatalog(), layer.id, input.isf_stage_id);
    const stage = index < 0 ? undefined : layerIsfStages(layer)[index];
    return stage ? { ok: true, layer, index, stage } : { ok: false, reason: "missing-stage" };
  };

  const dispatch = (input: PerformRoutedInput): Promise<PerformDispatchOutcome> => {
    switch (input.action) {
      case "VideoClipSlotTrigger": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        const slotId = input.clip_slot_id;
        if (slotId === null || slotId === undefined || !layer.clip_slots?.some((slot) => slot.id === slotId)) {
          return Promise.resolve({ handled: false, reason: "missing-slot" });
        }
        // Same steps as pressing the pad: select the layer and clip, then queue it.
        deps.selectLayer(layer.id);
        deps.selectSlot(slotId);
        return lanes.ordered("launch", async () => {
          const result = await deps.queueSlot(layer.id, slotId);
          if (result) justQueued.set(layer.id, Date.now());
        }).then(() => ({ handled: true as const }));
      }
      case "VideoTake": {
        return lanes.ordered<PerformDispatchOutcome>("launch", async () => {
          if (!deps.takeEnabled() && !queuedRecently()) {
            deps.report?.("Take ignored: no clip is queued.");
            return { handled: false, reason: "nothing-queued" };
          }
          justQueued.clear();
          await deps.take();
          return { handled: true };
        }).then((outcome) => outcome ?? { handled: false, reason: "unsupported" });
      }
      case "VideoLayerStop": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        justQueued.delete(layer.id);
        return lanes.ordered("launch", () => deps.stopLayer(layer.id)).then(() => ({ handled: true as const }));
      }
      case "VideoLayerBlendMode": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        const mode = performBlendModes[selectorIndex(input.value, performBlendModes.length)];
        if (layer.blend_mode === mode) return Promise.resolve({ handled: false, reason: "unchanged" });
        return lanes.latest(`blend:${layer.id}`, () => deps.setBlendMode(layer.id, mode))
          .then(() => ({ handled: true as const }));
      }
      case "VideoTransitionKind": {
        if (deps.transitionLocked()) return Promise.resolve({ handled: false, reason: "transition-running" });
        const kind = performTransitionKinds[selectorIndex(input.value, performTransitionKinds.length)];
        if (deps.transitionKind() === kind) return Promise.resolve({ handled: false, reason: "unchanged" });
        deps.setTransitionKind(kind);
        return Promise.resolve({ handled: true });
      }
      case "VideoTransitionTime": {
        if (deps.transitionLocked()) return Promise.resolve({ handled: false, reason: "transition-running" });
        // The mapping range is in the displayed unit (seconds, beats or bars);
        // the stored value is thousandths of it.
        const milliunits = Math.max(1, Math.min(600_000, Math.round(input.value * 1000)));
        if (deps.transitionDurationMilliunits() === milliunits) return Promise.resolve({ handled: false, reason: "unchanged" });
        deps.setTransitionDurationMilliunits(milliunits);
        return Promise.resolve({ handled: true });
      }
      case "VideoIsfStageEnabled": {
        const resolved = resolveStage(input);
        if (!resolved.ok) return Promise.resolve({ handled: false, reason: resolved.reason });
        const enabled = input.toggle ? !resolved.stage.enabled : input.active;
        if (resolved.stage.enabled === enabled) return Promise.resolve({ handled: false, reason: "unchanged" });
        return lanes.latest(`stage:${resolved.layer.id}:${resolved.index}`, () =>
          deps.setIsfStageEnabled(resolved.layer.id, resolved.index, enabled)).then(() => ({ handled: true as const }));
      }
      case "VideoIsfParam": {
        const resolved = resolveStage(input);
        if (!resolved.ok) return Promise.resolve({ handled: false, reason: resolved.reason });
        const control = resolved.stage.controls.find((candidate) => candidate.name === input.control);
        if (!control || !isfControlIsMappable(control)) return Promise.resolve({ handled: false, reason: "missing-control" });
        const { layer, index } = resolved;
        if (control.kind === "Event") {
          if (!input.active) return Promise.resolve({ handled: false, reason: "unchanged" });
          return lanes.ordered(`isf-event:${layer.id}:${index}:${control.name}`, () =>
            deps.triggerIsfEvent(layer.id, index, control.name)).then(() => ({ handled: true as const }));
        }
        const next = isfControlValueAt(control, input.value);
        if (next === null) return Promise.resolve({ handled: false, reason: "unsupported" });
        if (Math.abs(control.value[0] - next) < 1e-6) return Promise.resolve({ handled: false, reason: "unchanged" });
        const value: [number, number, number, number] = [next, control.value[1], control.value[2], control.value[3]];
        return lanes.latest(`isf:${layer.id}:${index}:${control.name}`, () =>
          deps.setIsfControl(layer.id, index, control.name, value)).then(() => ({ handled: true as const }));
      }
    }
  };

  return { dispatch };
}
