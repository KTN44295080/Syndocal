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
  /** The identity of the open project. A queued command never runs on another one. */
  projectEpoch: () => number;
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
  /** Resolves true once no live FX command is running (a running one drops a second).
   * Stage indices are resolved only after this, so a move or remove that was in
   * flight cannot leave the command on the wrong stage. */
  fxIdle: () => Promise<boolean>;
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

/** Minimum spacing between two authoritative ISF parameter commits. Every commit
 * remains a ticketed project revision; consecutive values share the native idle
 * Undo window. The controller stream is thinned to this rate (newest value wins). */
export const isfParamCommitSpacingMs = 200;

const sleep = (ms: number) => new Promise<void>((done) => setTimeout(done, ms));

type Task = () => Promise<unknown> | unknown;

/** Discrete triggers run in arrival order on one lane (a pad press must reach the
 * runtime before the Take that follows it); a continuous control keeps only its
 * newest value while an earlier one is still in flight; a pulse is dropped while
 * its previous pulse is running. */
function createLanes() {
  const ordered = new Map<string, Promise<unknown>>();
  const latest = new Map<string, { running: boolean; pending: Task | null; lastStart: number }>();
  const pulsing = new Set<string>();
  return {
    ordered<T>(key: string, task: () => Promise<T> | T): Promise<T | undefined> {
      const previous = ordered.get(key) ?? Promise.resolve();
      const run = previous.then(() => task());
      // A failed command reports itself; it must not stall the lane.
      ordered.set(key, run.then(() => undefined, () => undefined));
      return run.catch(() => undefined);
    },
    /** `onIdle` runs when the lane has nothing left in flight or pending. */
    latest(key: string, task: Task, options: { spacingMs?: number; onIdle?: () => void } = {}): Promise<unknown> {
      const lane = latest.get(key) ?? { running: false, pending: null, lastStart: 0 };
      latest.set(key, lane);
      if (lane.running) {
        lane.pending = task;
        return Promise.resolve();
      }
      const run = async () => {
        lane.running = true;
        let current: Task | null = task;
        try {
          while (current) {
            const wait = (options.spacingMs ?? 0) - (Date.now() - lane.lastStart);
            if (wait > 0) {
              await sleep(wait);
              // The value that arrived while waiting is the one to apply.
              current = lane.pending ?? current;
            }
            lane.pending = null;
            lane.lastStart = Date.now();
            try { await current(); } catch { /* the command reports its own failure */ }
            current = lane.pending;
          }
        } finally {
          lane.running = false;
          options.onIdle?.();
        }
      };
      return run();
    },
    pulse(key: string, task: Task): Promise<unknown> {
      if (pulsing.has(key)) return Promise.resolve();
      pulsing.add(key);
      return Promise.resolve().then(task).catch(() => undefined).finally(() => pulsing.delete(key));
    },
  };
}

export function createPerformControlDispatcher(deps: PerformControlDispatcherDeps) {
  const lanes = createLanes();
  /** The newest value requested for each continuous lane while a command is in
   * flight. A repeated or flipping input compares against this, not against the
   * rendered state that still shows the value from before the command. */
  const intents = new Map<string, unknown>();
  const intentValidity = new Map<string, () => boolean>();
  const hasCurrentIntent = (key: string) => intents.has(key) && intentValidity.get(key)?.() === true;
  /** Layers this dispatcher queued whose runtime poll has not caught up yet, with
   * the time and project of the queue. The window is short: it only bridges the
   * poll, it never keeps a stale queue alive. */
  const justQueued = new Map<number, { at: number; epoch: number }>();
  const queuedWindowMs = 1500;
  const queuedRecently = () => {
    const now = Date.now();
    const epoch = deps.projectEpoch();
    for (const [layerId, entry] of justQueued) {
      if (now - entry.at > queuedWindowMs || entry.epoch !== epoch) justQueued.delete(layerId);
    }
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

  const handled: PerformDispatchOutcome = { handled: true };
  const unchanged: PerformDispatchOutcome = { handled: false, reason: "unchanged" };

  const dispatch = (input: PerformRoutedInput): Promise<PerformDispatchOutcome> => {
    const projectEpoch = deps.projectEpoch();
    const stillCurrent = () => deps.projectEpoch() === projectEpoch && (input.sourceIsCurrent?.() ?? true);
    const inThisProject = (task: Task): Task => () => stillCurrent() ? task() : undefined;
    if (!stillCurrent()) return Promise.resolve({ handled: false, reason: "unsupported" });
    switch (input.action) {
      case "VideoClipSlotTrigger": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        const slotId = input.clip_slot_id;
        if (slotId === null || slotId === undefined || !layer.clip_slots?.some((slot) => slot.id === slotId)) {
          return Promise.resolve({ handled: false, reason: "missing-slot" });
        }
        const epoch = deps.projectEpoch();
        return lanes.ordered("launch", inThisProject(async () => {
          // Same steps as pressing the pad: select the layer and clip, then queue it.
          // Selecting here, in order, keeps a following Take on this layer.
          deps.selectLayer(layer.id);
          deps.selectSlot(slotId);
          const result = await deps.queueSlot(layer.id, slotId);
          if (result && stillCurrent()) justQueued.set(layer.id, { at: Date.now(), epoch });
        })).then(() => handled);
      }
      case "VideoTake": {
        return lanes.ordered<PerformDispatchOutcome>("launch", async () => {
          if (!stillCurrent()) return { handled: false, reason: "unsupported" };
          if (!deps.takeEnabled() && !queuedRecently()) {
            // A beat fires every division; only a controller press deserves a message.
            if (input.source !== "beat") deps.report?.("Take ignored: no clip is queued.");
            return { handled: false, reason: "nothing-queued" };
          }
          justQueued.clear();
          await deps.take();
          return handled;
        }).then((outcome) => outcome ?? { handled: false, reason: "unsupported" });
      }
      case "VideoLayerStop": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        justQueued.delete(layer.id);
        return lanes.ordered("launch", inThisProject(() => deps.stopLayer(layer.id))).then(() => handled);
      }
      case "VideoLayerBlendMode": {
        const layer = layerById(input.layer_id);
        if (!layer) return Promise.resolve({ handled: false, reason: "missing-layer" });
        const mode = performBlendModes[selectorIndex(input.value, performBlendModes.length)];
        const key = `blend:${layer.id}`;
        if ((hasCurrentIntent(key) ? intents.get(key) : layer.blend_mode) === mode) return Promise.resolve(unchanged);
        intents.set(key, mode);
        intentValidity.set(key, stillCurrent);
        return lanes.latest(key, inThisProject(() => deps.setBlendMode(layer.id, mode)), { onIdle: () => { intents.delete(key); intentValidity.delete(key); } })
          .then(() => handled);
      }
      case "VideoTransitionKind": {
        if (deps.transitionLocked()) return Promise.resolve({ handled: false, reason: "transition-running" });
        const kind = performTransitionKinds[selectorIndex(input.value, performTransitionKinds.length)];
        if (deps.transitionKind() === kind) return Promise.resolve(unchanged);
        deps.setTransitionKind(kind);
        return Promise.resolve(handled);
      }
      case "VideoTransitionTime": {
        if (deps.transitionLocked()) return Promise.resolve({ handled: false, reason: "transition-running" });
        // The mapping range is in the displayed unit (seconds, beats or bars);
        // the stored value is thousandths of it.
        const milliunits = Math.max(1, Math.min(600_000, Math.round(input.value * 1000)));
        if (deps.transitionDurationMilliunits() === milliunits) return Promise.resolve(unchanged);
        deps.setTransitionDurationMilliunits(milliunits);
        return Promise.resolve(handled);
      }
      case "VideoIsfStageEnabled": {
        const resolved = resolveStage(input);
        if (!resolved.ok) return Promise.resolve({ handled: false, reason: resolved.reason });
        const stageId = input.isf_stage_id as number;
        const key = `stage:${resolved.layer.id}:${stageId}`;
        // A press flips what the last press asked for, so two presses inside one
        // command round trip still alternate.
        const base = hasCurrentIntent(key) ? intents.get(key) === true : resolved.stage.enabled;
        const enabled = input.toggle ? !base : input.active;
        if (base === enabled) return Promise.resolve(unchanged);
        intents.set(key, enabled);
        intentValidity.set(key, stillCurrent);
        const layerId = resolved.layer.id;
        return lanes.latest(key, inThisProject(async () => {
          const idle = await deps.fxIdle();
          if (!stillCurrent()) return;
          if (!idle) {
            deps.report?.("Effect stage change skipped: another effect command is still running.");
            return;
          }
          // Resolve the index now, after any move or remove has settled.
          const fresh = resolveStage(input);
          if (fresh.ok) await deps.setIsfStageEnabled(layerId, fresh.index, enabled);
        }), { onIdle: () => { intents.delete(key); intentValidity.delete(key); } }).then(() => handled);
      }
      case "VideoIsfParam": {
        const resolved = resolveStage(input);
        if (!resolved.ok) return Promise.resolve({ handled: false, reason: resolved.reason });
        const control = resolved.stage.controls.find((candidate) => candidate.name === input.control);
        if (!control || !isfControlIsMappable(control)) return Promise.resolve({ handled: false, reason: "missing-control" });
        const stageId = input.isf_stage_id as number;
        const layerId = resolved.layer.id;
        const name = control.name;
        if (control.kind === "Event") {
          if (!input.active) return Promise.resolve(unchanged);
          // A pulse already running owns the next one; a fader's stream is one pulse.
          return lanes.pulse(`isf-event:${layerId}:${stageId}:${name}`, inThisProject(async () => {
            const idle = await deps.fxIdle();
            if (!stillCurrent() || !idle) return;
            const fresh = resolveStage(input);
            if (fresh.ok) await deps.triggerIsfEvent(layerId, fresh.index, name);
          })).then(() => handled);
        }
        const next = isfControlValueAt(control, input.value);
        if (next === null) return Promise.resolve({ handled: false, reason: "unsupported" });
        const key = `isf:${layerId}:${stageId}:${name}`;
        const base = hasCurrentIntent(key) ? Number(intents.get(key)) : control.value[0];
        if (Math.abs(base - next) < 1e-6) return Promise.resolve(unchanged);
        intents.set(key, next);
        intentValidity.set(key, stillCurrent);
        return lanes.latest(key, inThisProject(async () => {
          const idle = await deps.fxIdle();
          if (!stillCurrent()) return;
          if (!idle) {
            deps.report?.("Effect parameter change skipped: another effect command is still running.");
            return;
          }
          const fresh = resolveStage(input);
          const current = fresh.ok ? fresh.stage.controls.find((candidate) => candidate.name === name) : undefined;
          if (!fresh.ok || !current) return;
          await deps.setIsfControl(layerId, fresh.index, name, [next, current.value[1], current.value[2], current.value[3]]);
        }), { spacingMs: isfParamCommitSpacingMs, onIdle: () => { intents.delete(key); intentValidity.delete(key); } }).then(() => handled);
      }
    }
  };

  return { dispatch };
}
