import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";
import { emptyLiveVideoMonitorBusState, type LiveVideoMonitorBusState } from "./createLiveVideoMonitorController";
import { parseLiveVideoMonitorPacket } from "./liveVideoMonitorPacket";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import type { VideoOutputSummary } from "./types";

interface Options {
  invoke: FrontendTauriInvoke;
  backendAvailable: Accessor<boolean>;
  active: Accessor<boolean>;
  outputs: Accessor<readonly VideoOutputSummary[]>;
}

interface OutputRuntime {
  key: string;
  width: number;
  height: number;
  dueAt: number;
  errors: number;
  lastFrameAt: number;
  url: string | null;
}

/** Presentation size only: output mapping still uses the authored output configuration. */
export const timelineOutputMonitorSize = (width: number, height: number) => {
  if (!Number.isSafeInteger(width) || !Number.isSafeInteger(height) || width < 1 || height < 1) return null;
  const scale = Math.min(1, 320 / width, 180 / height);
  return { width: Math.max(1, Math.round(width * scale)), height: Math.max(1, Math.round(height * scale)) };
};

/** One local readback request at a time across all outputs, at most ten starts per second. */
export const createTimelineOutputMonitorController = (options: Options) => {
  const [states, setStates] = createSignal<ReadonlyMap<number, LiveVideoMonitorBusState>>(new Map());
  const [hidden, setHidden] = createSignal(typeof document !== "undefined" && document.hidden);
  const runtime = new Map<number, OutputRuntime>();
  let order: number[] = [];
  let cursor = 0;
  let generation = 0;
  let active = false;
  let disposed = false;
  let inFlight = false;
  let timer: number | null = null;
  let nextStartAt = 0;

  const publish = (id: number, update: (state: LiveVideoMonitorBusState) => LiveVideoMonitorBusState) => {
    setStates((current) => {
      const previous = current.get(id);
      if (!previous) return current;
      const next = new Map(current);
      next.set(id, update(previous));
      return next;
    });
  };
  const revoke = (entry: OutputRuntime) => {
    if (entry.url) URL.revokeObjectURL(entry.url);
    entry.url = null;
  };
  const clearTimer = () => {
    if (timer !== null) window.clearTimeout(timer);
    timer = null;
  };
  const schedule = () => {
    clearTimer();
    if (disposed || !active || inFlight || runtime.size === 0) return;
    const due = Math.min(...Array.from(runtime.values(), (entry) => entry.dueAt));
    if (!Number.isFinite(due)) return;
    timer = window.setTimeout(() => void runNext(), Math.max(0, Math.max(nextStartAt, due) - performance.now()));
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
    nextStartAt = now + 100;
    try {
      const response = await options.invoke<unknown>("get_live_video_monitor_frame", {
        monitorKind: "program", outputId: id, layerId: null,
        width: entry.width, height: entry.height, quality: 68, decodeBudget: 1,
      });
      if (!current()) return;
      const packet = parseLiveVideoMonitorPacket(response);
      if (packet.kind !== "program") throw new Error("Output monitor returned the wrong video bus.");
      if (packet.status === "busy") {
        revoke(entry);
        entry.lastFrameAt = 0;
        publish(id, (state) => ({ ...state, status: "starting", frameUrl: null, fps: 0,
          busyDrops: state.busyDrops + 1, error: null }));
        return;
      }
      if (packet.width !== entry.width || packet.height !== entry.height) throw new Error("Output monitor returned an unexpected frame size.");
      const jpeg = Uint8Array.from(packet.jpeg);
      const url = URL.createObjectURL(new Blob([jpeg.buffer], { type: "image/jpeg" }));
      const previousUrl = entry.url;
      entry.url = url;
      const frameAt = performance.now();
      const fps = entry.lastFrameAt ? 1_000 / Math.max(1, frameAt - entry.lastFrameAt) : 0;
      entry.lastFrameAt = frameAt;
      entry.errors = 0;
      entry.dueAt = 0;
      publish(id, (state) => ({ ...state, status: "live", frameUrl: url, error: null,
        fps, width: packet.width, height: packet.height, sequence: packet.sequence, ptsMs: packet.ptsMs,
        renderMs: packet.renderUs / 1_000, encodeMs: packet.encodeUs / 1_000,
      }));
      if (previousUrl) URL.revokeObjectURL(previousUrl);
    } catch (error) {
      if (!current()) return;
      entry.errors += 1;
      entry.dueAt = performance.now() + Math.min(4_000, 250 * 2 ** Math.min(4, entry.errors - 1));
      // An error must not leave a previous playhead/configuration image looking current.
      revoke(entry);
      const message = error instanceof Error ? error.message : String(error);
      publish(id, (state) => ({ ...state, status: "error", frameUrl: null, error: message.slice(0, 180) }));
    } finally {
      inFlight = false;
      schedule();
    }
  };

  createEffect(() => {
    const outputs = options.outputs();
    const available = options.backendAvailable();
    const nextActive = available && options.active() && !hidden();
    if (active !== nextActive) {
      generation += 1;
      active = nextActive;
      for (const entry of runtime.values()) {
        revoke(entry);
        entry.lastFrameAt = 0;
        if (Number.isFinite(entry.dueAt)) entry.dueAt = 0;
      }
    }
    const ids = new Set(outputs.map((output) => output.id));
    for (const [id, entry] of runtime) {
      if (!ids.has(id)) { revoke(entry); runtime.delete(id); }
    }
    order = outputs.map((output) => output.id);
    if (cursor >= order.length) cursor = 0;
    setStates((previous) => {
      const next = new Map<number, LiveVideoMonitorBusState>();
      for (const output of outputs) {
        // Structural identity prevents snapshot publication from repeatedly cancelling work.
        const key = JSON.stringify(output);
        let entry = runtime.get(output.id);
        let state = previous.get(output.id);
        if (!entry || entry.key !== key) {
          if (entry) revoke(entry);
          const size = timelineOutputMonitorSize(output.width, output.height);
          entry = { key, width: size?.width ?? 0, height: size?.height ?? 0,
            dueAt: size ? 0 : Infinity, errors: 0, lastFrameAt: 0, url: null };
          runtime.set(output.id, entry);
          state = emptyLiveVideoMonitorBusState("starting");
        }
        state ??= emptyLiveVideoMonitorBusState("starting");
        if (!entry.width || !entry.height) {
          state = { ...state, status: "error", frameUrl: null, error: "Output resolution must contain positive integer dimensions." };
        } else if (!active) {
          state = { ...state, status: available ? "paused" : "desktop-required", frameUrl: null, fps: 0 };
        } else if (!entry.url && state.status !== "error") {
          state = { ...state, status: "starting", frameUrl: null, fps: 0 };
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
    if (typeof document !== "undefined") document.removeEventListener("visibilitychange", visibility);
    for (const entry of runtime.values()) revoke(entry);
  });
  return { states, retry };
};
