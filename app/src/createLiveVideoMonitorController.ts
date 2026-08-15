import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  parseLiveVideoMonitorPacket,
  type LiveVideoMonitorKind,
} from "./liveVideoMonitorPacket";

export type LiveVideoMonitorStatus =
  | "desktop-required"
  | "paused"
  | "idle"
  | "empty"
  | "starting"
  | "live"
  | "error";

export interface LiveVideoMonitorBusState {
  status: LiveVideoMonitorStatus;
  frameUrl: string | null;
  fps: number;
  busyDrops: number;
  renderMs: number;
  encodeMs: number;
  width: number;
  height: number;
  sequence: bigint;
  ptsMs: bigint;
  error: string | null;
}

export const emptyLiveVideoMonitorBusState = (
  status: LiveVideoMonitorStatus = "desktop-required",
): LiveVideoMonitorBusState => ({
  status,
  frameUrl: null,
  fps: 0,
  busyDrops: 0,
  renderMs: 0,
  encodeMs: 0,
  width: 0,
  height: 0,
  sequence: 0n,
  ptsMs: 0n,
  error: null,
});

interface LiveVideoMonitorControllerOptions {
  invoke: FrontendTauriInvoke;
  backendAvailable: Accessor<boolean>;
  active: Accessor<boolean>;
  layerCount: Accessor<number>;
  previewLayerId: Accessor<number | null>;
  programOutputId: Accessor<number | null>;
  width?: number;
  height?: number;
  jpegQuality?: number;
}

interface BusRuntime {
  dueAt: number;
  intervalMs: number;
  lastFrameAt: number;
  errorCount: number;
  frameUrl: string | null;
}

const busKinds = ["program", "preview"] as const;

const cleanError = (error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  return message.length > 180 ? `${message.slice(0, 177)}...` : message;
};

export const createLiveVideoMonitorController = (options: LiveVideoMonitorControllerOptions) => {
  const [program, setProgram] = createSignal<LiveVideoMonitorBusState>(emptyLiveVideoMonitorBusState());
  const [preview, setPreview] = createSignal<LiveVideoMonitorBusState>(emptyLiveVideoMonitorBusState());
  const [pageHidden, setPageHidden] = createSignal(typeof document === "undefined" ? false : document.hidden);
  const runtime: Record<LiveVideoMonitorKind, BusRuntime> = {
    program: { dueAt: 0, intervalMs: 100, lastFrameAt: 0, errorCount: 0, frameUrl: null },
    preview: { dueAt: 0, intervalMs: 200, lastFrameAt: 0, errorCount: 0, frameUrl: null },
  };
  let generation = 0;
  let timerId: number | null = null;
  let inFlight = false;
  let disposed = false;
  let lastPreviewLayerId: number | null = null;
  let lastProgramOutputId: number | null = null;

  const setBus = (
    kind: LiveVideoMonitorKind,
    update: (current: LiveVideoMonitorBusState) => LiveVideoMonitorBusState,
  ) => kind === "program" ? setProgram(update) : setPreview(update);

  const revokeBusUrl = (kind: LiveVideoMonitorKind) => {
    const current = runtime[kind].frameUrl;
    runtime[kind].frameUrl = null;
    if (current) URL.revokeObjectURL(current);
  };

  const clearTimer = () => {
    if (timerId !== null) window.clearTimeout(timerId);
    timerId = null;
  };

  const canRun = () => options.backendAvailable() && options.active() && !pageHidden();
  const kindEligible = (kind: LiveVideoMonitorKind) =>
    options.layerCount() > 0 && (kind === "program"
      ? options.programOutputId() !== null
      : options.previewLayerId() !== null);

  const idleStatus = (kind: LiveVideoMonitorKind): LiveVideoMonitorStatus => {
    if (!options.backendAvailable()) return "desktop-required";
    if (!options.active() || pageHidden()) return "paused";
    if (options.layerCount() === 0) return "empty";
    if (kind === "preview" && options.previewLayerId() === null) return "idle";
    if (kind === "program" && options.programOutputId() === null) return "idle";
    return "starting";
  };

  const syncIdleState = (kind: LiveVideoMonitorKind) => {
    if (canRun() && kindEligible(kind)) {
      setBus(kind, (current) => current.status === "live" && current.frameUrl
        ? current
        : { ...current, status: "starting", error: null });
      return;
    }
    setBus(kind, (current) => ({ ...current, status: idleStatus(kind), error: null }));
  };

  const schedule = (delayMs: number) => {
    clearTimer();
    if (disposed || !canRun() || inFlight) return;
    timerId = window.setTimeout(() => void runNext(), Math.max(0, delayMs));
  };

  const runNext = async () => {
    timerId = null;
    if (disposed || inFlight || !canRun()) return;
    const eligible = busKinds.filter(kindEligible);
    if (eligible.length === 0) return;
    const now = performance.now();
    const kind = eligible.reduce((next, candidate) =>
      runtime[candidate].dueAt < runtime[next].dueAt ? candidate : next,
    );
    const waitMs = runtime[kind].dueAt - now;
    if (waitMs > 1) {
      schedule(waitMs);
      return;
    }

    const requestGeneration = generation;
    const targetId = kind === "program" ? options.programOutputId() : options.previewLayerId();
    inFlight = true;
    runtime[kind].dueAt = now + runtime[kind].intervalMs;
    try {
      const response = await options.invoke<unknown>("get_live_video_monitor_frame", {
        monitorKind: kind,
        outputId: kind === "program" ? targetId : null,
        layerId: kind === "preview" ? targetId : null,
        width: options.width ?? 320,
        height: options.height ?? 180,
        quality: options.jpegQuality ?? 68,
        decodeBudget: kind === "program" ? 1 : null,
      });
      if (disposed || requestGeneration !== generation) return;
      const packet = parseLiveVideoMonitorPacket(response);
      if (packet.kind !== kind) throw new Error(`Live monitor returned ${packet.kind} for ${kind}.`);
      if (packet.status === "busy") {
        setBus(kind, (current) => ({ ...current, busyDrops: current.busyDrops + 1 }));
        return;
      }

      const jpeg = Uint8Array.from(packet.jpeg);
      const nextUrl = URL.createObjectURL(new Blob([jpeg.buffer], { type: "image/jpeg" }));
      const previousUrl = runtime[kind].frameUrl;
      runtime[kind].frameUrl = nextUrl;
      const frameAt = performance.now();
      const instantaneousFps = runtime[kind].lastFrameAt > 0
        ? 1_000 / Math.max(1, frameAt - runtime[kind].lastFrameAt)
        : 0;
      runtime[kind].lastFrameAt = frameAt;
      runtime[kind].errorCount = 0;
      setBus(kind, (current) => ({
        ...current,
        status: "live",
        frameUrl: nextUrl,
        fps: current.fps > 0 && instantaneousFps > 0
          ? current.fps * 0.75 + instantaneousFps * 0.25
          : instantaneousFps,
        renderMs: packet.renderUs / 1_000,
        encodeMs: packet.encodeUs / 1_000,
        width: packet.width,
        height: packet.height,
        sequence: packet.sequence,
        ptsMs: packet.ptsMs,
        error: null,
      }));
      if (previousUrl) URL.revokeObjectURL(previousUrl);
    } catch (error) {
      if (disposed || requestGeneration !== generation) return;
      runtime[kind].errorCount += 1;
      runtime[kind].dueAt = performance.now() + Math.min(4_000, 250 * (2 ** (runtime[kind].errorCount - 1)));
      setBus(kind, (current) => ({ ...current, status: "error", error: cleanError(error) }));
    } finally {
      inFlight = false;
      if (!disposed && canRun()) {
        const nextDueAt = busKinds.filter(kindEligible).reduce(
          (dueAt, candidate) => Math.min(dueAt, runtime[candidate].dueAt),
          Number.POSITIVE_INFINITY,
        );
        schedule(Number.isFinite(nextDueAt) ? nextDueAt - performance.now() : 250);
      }
    }
  };

  const invalidate = () => {
    generation += 1;
    clearTimer();
    const now = performance.now();
    for (const kind of busKinds) runtime[kind].dueAt = now;
    if (!inFlight) schedule(0);
  };

  const retry = () => {
    for (const kind of busKinds) runtime[kind].errorCount = 0;
    invalidate();
  };

  const onVisibilityChange = () => setPageHidden(document.hidden);
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibilityChange);

  createEffect(() => {
    options.backendAvailable();
    options.active();
    options.layerCount();
    pageHidden();
    const previewLayerId = options.previewLayerId();
    const programOutputId = options.programOutputId();
    if (previewLayerId !== lastPreviewLayerId) {
      lastPreviewLayerId = previewLayerId;
      revokeBusUrl("preview");
      runtime.preview.lastFrameAt = 0;
      setPreview((current) => ({ ...current, frameUrl: null, fps: 0, error: null }));
    }
    if (programOutputId !== lastProgramOutputId) {
      lastProgramOutputId = programOutputId;
      revokeBusUrl("program");
      runtime.program.lastFrameAt = 0;
      setProgram((current) => ({ ...current, frameUrl: null, fps: 0, error: null }));
    }
    syncIdleState("program");
    syncIdleState("preview");
    invalidate();
  });

  onCleanup(() => {
    disposed = true;
    generation += 1;
    clearTimer();
    if (typeof document !== "undefined") document.removeEventListener("visibilitychange", onVisibilityChange);
    revokeBusUrl("program");
    revokeBusUrl("preview");
  });

  return { program, preview, retry };
};
