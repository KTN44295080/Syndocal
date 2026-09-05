import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";
import { parseLiveVideoMonitorPacket } from "./liveVideoMonitorPacket";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { VideoOutputSummary } from "./types";

interface Options {
  invoke: FrontendTauriInvoke;
  backendAvailable: Accessor<boolean>;
  active: Accessor<boolean>;
  outputs: Accessor<readonly VideoOutputSummary[]>;
  projectEpoch: Accessor<number>;
}

export interface TimelineOutputMonitorFrame {
  width: number;
  height: number;
  data: Uint8ClampedArray<ArrayBuffer>;
}
export interface TimelineOutputMonitorState {
  status: "starting" | "live" | "error" | "paused" | "desktop-required";
  frame: TimelineOutputMonitorFrame | null;
  error: string | null;
  fps: number;
  width: number;
  height: number;
  sequence: bigint;
  ptsMs: bigint;
  renderMs: number;
  encodeMs: number;
  busyDrops: number;
}
const emptyState = (): TimelineOutputMonitorState => ({
  status: "starting", frame: null, error: null, fps: 0, width: 0, height: 0,
  sequence: 0n, ptsMs: 0n, renderMs: 0, encodeMs: 0, busyDrops: 0,
});

interface OutputRuntime {
  key: string;
  width: number;
  height: number;
  dueAt: number;
  errors: number;
  lastFrameAt: number;
  frame: TimelineOutputMonitorFrame | null;
}

/** Presentation size only: output mapping still uses the authored output configuration. */
export const timelineOutputMonitorSize = (width: number, height: number) => {
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 1 || height < 1) return null;
  const scale = Math.min(1, 320 / width, 180 / height);
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
};

// A transient busy reply can retain only a recently received frame. This is
// wall-clock freshness, independent of Timeline playing state or clip PTS.
export const TIMELINE_OUTPUT_FRAME_MAX_AGE_MS = 1_000;
const OUTPUT_FRAME_INTERVAL_MS = 1_000 / 30;
const AGGREGATE_START_INTERVAL_MS = 1_000 / 60;

/** Target 30 fps per output, sharing at most 60 starts/s with one readback in flight. */
export const createTimelineOutputMonitorController = (options: Options) => {
  const [states, setStates] = createSignal<ReadonlyMap<number, TimelineOutputMonitorState>>(new Map());
  const [hidden, setHidden] = createSignal(typeof document !== "undefined" && document.hidden);
  const runtime = new Map<number, OutputRuntime>();
  let order: number[] = [];
  let cursor = 0;
  let generation = 0;
  let projectEpoch: number | undefined;
  let active = false;
  let disposed = false;
  let inFlight = false;
  let timer: number | null = null;
  let expiryTimer: number | null = null;
  let nextStartAt = 0;

  const publish = (id: number, update: (state: TimelineOutputMonitorState) => TimelineOutputMonitorState) => {
    setStates((current) => {
      const previous = current.get(id);
      if (!previous) return current;
      const next = new Map(current);
      next.set(id, update(previous));
      return next;
    });
  };
  const clearFrame = (entry: OutputRuntime) => { entry.frame = null; };
  const clearTimer = () => {
    if (timer !== null) window.clearTimeout(timer);
    timer = null;
  };
  const clearExpiryTimer = () => {
    if (expiryTimer !== null) window.clearTimeout(expiryTimer);
    expiryTimer = null;
  };
  const scheduleExpiry = () => {
    clearExpiryTimer();
    if (disposed || !active) return;
    const expiresAt = Math.min(...Array.from(runtime.values(), (entry) =>
      entry.frame ? entry.lastFrameAt + TIMELINE_OUTPUT_FRAME_MAX_AGE_MS : Infinity));
    if (!Number.isFinite(expiresAt)) return;
    expiryTimer = window.setTimeout(() => {
      expiryTimer = null;
      const now = performance.now();
      for (const [id, entry] of runtime) {
        if (!entry.frame || now < entry.lastFrameAt + TIMELINE_OUTPUT_FRAME_MAX_AGE_MS) continue;
        clearFrame(entry);
        entry.lastFrameAt = 0;
        publish(id, (state) => ({ ...state, status: "error", frame: null, fps: 0,
          error: "Output preview frame expired; waiting for a current frame." }));
      }
      scheduleExpiry();
    }, Math.max(0, Math.ceil(expiresAt - performance.now())));
  };
  const schedule = () => {
    scheduleExpiry();
    clearTimer();
    if (disposed || !active || inFlight || runtime.size === 0) return;
    const due = Math.min(...Array.from(runtime.values(), (entry) => entry.dueAt));
    if (!Number.isFinite(due)) return;
    timer = window.setTimeout(() => void runNext(), Math.max(0, Math.ceil(Math.max(nextStartAt, due) - performance.now())));
  };
  const runNext = async () => {
    timer = null;
    if (disposed || !active || inFlight) return;
    const now = performance.now();
    let id: number | undefined;
    for (let offset = 0; offset < order.length; offset += 1) {
      const index = (cursor + offset) % order.length;
      const candidate = order[index];
      if ((runtime.get(candidate)?.dueAt ?? Infinity) <= now) {
        id = candidate;
        cursor = (index + 1) % order.length;
        break;
      }
    }
    if (id === undefined || now < nextStartAt) { schedule(); return; }
    const entry = runtime.get(id)!;
    const requestGeneration = generation;
    const current = () => !disposed && active && generation === requestGeneration && runtime.get(id!) === entry;
    inFlight = true;
    nextStartAt = now + AGGREGATE_START_INTERVAL_MS;
    entry.dueAt = now + OUTPUT_FRAME_INTERVAL_MS;
    try {
      const response = await options.invoke<unknown>("get_live_video_monitor_frame", {
        monitorKind: "program", outputId: id, layerId: null,
        width: entry.width, height: entry.height, pixelFormat: "rgba",
      });
      if (!current()) return;
      const packet = parseLiveVideoMonitorPacket(response);
      if (packet.encoding !== "rgba") throw new Error("Output monitor returned an unexpected pixel format.");
      if (packet.kind !== "program") throw new Error("Output monitor returned the wrong video bus.");
      if (packet.status === "busy") {
        // Only contention may hold the last valid image; it cannot extend the
        // independent freshness deadline or resurrect a retired generation.
        publish(id, (state) => ({ ...state,
          status: entry.frame ? "live" : state.status === "error" ? "error" : "starting",
          frame: entry.frame, fps: entry.frame ? state.fps : 0,
          busyDrops: state.busyDrops + 1,
        }));
        return;
      }
      if (packet.width !== entry.width || packet.height !== entry.height) throw new Error("Output monitor returned an unexpected frame size.");
      const frame: TimelineOutputMonitorFrame = { width: packet.width, height: packet.height, data: packet.rgba };
      entry.frame = frame;
      const frameAt = performance.now();
      const fps = entry.lastFrameAt ? 1_000 / Math.max(1, frameAt - entry.lastFrameAt) : 0;
      entry.lastFrameAt = frameAt;
      entry.errors = 0;
      publish(id, (state) => ({ ...state, status: "live", frame, error: null,
        fps, width: packet.width, height: packet.height, sequence: packet.sequence, ptsMs: packet.ptsMs,
        renderMs: packet.renderUs / 1_000, encodeMs: packet.encodeUs / 1_000,
      }));
    } catch (error) {
      if (!current()) return;
      entry.errors += 1;
      entry.dueAt = performance.now() + Math.min(4_000, 250 * 2 ** Math.min(4, entry.errors - 1));
      // An error must not leave a previous playhead/configuration image looking current.
      clearFrame(entry);
      const message = error instanceof Error ? error.message : String(error);
      publish(id, (state) => ({ ...state, status: "error", frame: null, error: message.slice(0, 180) }));
    } finally {
      inFlight = false;
      schedule();
    }
  };

  createEffect(() => {
    const outputs = options.outputs();
    const nextProjectEpoch = options.projectEpoch();
    if (projectEpoch !== nextProjectEpoch) {
      generation += 1;
      projectEpoch = nextProjectEpoch;
      for (const entry of runtime.values()) clearFrame(entry);
      runtime.clear();
    }
    const available = options.backendAvailable();
    const nextActive = available && options.active() && !hidden();
    if (active !== nextActive) {
      generation += 1;
      active = nextActive;
      for (const entry of runtime.values()) {
        clearFrame(entry);
        entry.lastFrameAt = 0;
        if (Number.isFinite(entry.dueAt)) entry.dueAt = 0;
      }
    }
    const ids = new Set(outputs.map((output) => output.id));
    for (const [id, entry] of runtime) {
      if (!ids.has(id)) { clearFrame(entry); runtime.delete(id); }
    }
    order = outputs.map((output) => output.id);
    if (cursor >= order.length) cursor = 0;
    setStates((previous) => {
      const next = new Map<number, TimelineOutputMonitorState>();
      for (const output of outputs) {
        // Structural identity prevents snapshot publication from repeatedly cancelling work.
        const key = JSON.stringify(output);
        let entry = runtime.get(output.id);
        let state = previous.get(output.id);
        if (!entry || entry.key !== key) {
          if (entry) clearFrame(entry);
          const size = timelineOutputMonitorSize(output.width, output.height);
          entry = { key, width: size?.width ?? 0, height: size?.height ?? 0,
            dueAt: size ? 0 : Infinity, errors: 0, lastFrameAt: 0, frame: null };
          runtime.set(output.id, entry);
          state = emptyState();
        }
        state ??= emptyState();
        if (!entry.width || !entry.height) {
          state = { ...state, status: "error", frame: null, error: "Output resolution must contain positive integer dimensions." };
        } else if (!active) {
          state = { ...state, status: available ? "paused" : "desktop-required", frame: null, fps: 0 };
        } else if (!entry.frame && state.status !== "error") {
          state = { ...state, status: "starting", frame: null, fps: 0 };
        }
        next.set(output.id, state);
      }
      return next;
    });
    schedule();
  });

  const retry = () => {
    for (const entry of runtime.values()) {
      entry.errors = 0;
      if (Number.isFinite(entry.dueAt)) entry.dueAt = 0;
    }
    schedule();
  };
  const visibility = () => setHidden(document.hidden);
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", visibility);
  onCleanup(() => {
    disposed = true;
    generation += 1;
    clearTimer();
    clearExpiryTimer();
    if (typeof document !== "undefined") document.removeEventListener("visibilitychange", visibility);
    runtime.clear();
    setStates(new Map());
  });
  return { states, retry };
};
