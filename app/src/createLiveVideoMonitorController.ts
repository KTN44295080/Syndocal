import { createEffect, createMemo, createSignal, onCleanup, type Accessor } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import {
  parseLiveVideoMonitorPacket,
  type LiveVideoMonitorKind,
} from "./liveVideoMonitorPacket";
import { createMonitorPressurePolicy, monitorTargetFps, type LiveVideoMonitorQuality, type LiveVideoOutputHealth } from "./liveVideoMonitorPolicy";

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
  frame: ImageBitmap | ImageData | null;
  throttled: boolean;
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
  frame: null,
  throttled: false,
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
  previewTransportGeneration?: Accessor<number>;
  programOutputId: Accessor<number | null>;
  quality?: Accessor<LiveVideoMonitorQuality>;
  previewActive?: Accessor<boolean>;
  outputHealth?: Accessor<readonly LiveVideoOutputHealth[]>;
  outputHealthError?: Accessor<string | null>;
  pixelFormat?: "jpeg" | "rgba";
  presentation?: "image" | "canvas";
  width?: number;
  height?: number;
  jpegQuality?: number;
}

interface BusRuntime {
  dueAt: number;
  timerId: number | null;
  inFlight: boolean;
  frameTimes: number[];
  errorCount: number;
  frameUrl: string | null;
  frame: ImageBitmap | ImageData | null;
}

const busKinds = ["program", "preview"] as const;

const cleanError = (error: unknown) => {
  const message = error instanceof Error ? error.message : String(error);
  return message.length > 180 ? `${message.slice(0, 177)}...` : message;
};

export const createLiveVideoMonitorController = (options: LiveVideoMonitorControllerOptions) => {
  // Polling publishes new snapshot objects even when eligibility is unchanged.
  // Memo equality prevents those snapshots from restarting the monitor clocks.
  const eligibility = {
    backendAvailable: options.backendAvailable,
    active: createMemo(options.active),
    layerCount: createMemo(options.layerCount),
    previewLayerId: createMemo(options.previewLayerId),
    previewTransportGeneration: createMemo(() => options.previewTransportGeneration?.() ?? 0),
    programOutputId: createMemo(options.programOutputId),
    quality: createMemo(() => options.quality?.() ?? "smooth"),
    previewActive: createMemo(() => options.previewActive?.() ?? options.previewLayerId() !== null),
  };
  const [program, setProgram] = createSignal<LiveVideoMonitorBusState>(emptyLiveVideoMonitorBusState());
  const [preview, setPreview] = createSignal<LiveVideoMonitorBusState>(emptyLiveVideoMonitorBusState());
  const [pageHidden, setPageHidden] = createSignal(typeof document === "undefined" ? false : document.hidden);
  const runtime: Record<LiveVideoMonitorKind, BusRuntime> = {
    program: { dueAt: 0, timerId: null, inFlight: false, frameTimes: [], errorCount: 0, frameUrl: null, frame: null },
    preview: { dueAt: 0, timerId: null, inFlight: false, frameTimes: [], errorCount: 0, frameUrl: null, frame: null },
  };
  const pressure = createMonitorPressurePolicy();
  let generation = 0;
  let disposed = false;
  let lastPreviewLayerId: number | null = null;
  let lastPreviewTransportGeneration = 0;
  let lastProgramOutputId: number | null = null;

  const setBus = (
    kind: LiveVideoMonitorKind,
    update: (current: LiveVideoMonitorBusState) => LiveVideoMonitorBusState,
  ) => kind === "program" ? setProgram(update) : setPreview(update);

  const closeFrame = (frame: ImageBitmap | ImageData | null) => {
    if (frame && "close" in frame) frame.close();
  };
  const revokeBusUrl = (kind: LiveVideoMonitorKind) => {
    const current = runtime[kind].frameUrl;
    runtime[kind].frameUrl = null;
    if (current) URL.revokeObjectURL(current);
    closeFrame(runtime[kind].frame);
    runtime[kind].frame = null;
  };

  const clearTimer = (kind: LiveVideoMonitorKind) => {
    if (runtime[kind].timerId !== null) window.clearTimeout(runtime[kind].timerId!);
    runtime[kind].timerId = null;
  };

  const canRun = () => eligibility.backendAvailable() && eligibility.active() && !pageHidden();
  const kindEligible = (kind: LiveVideoMonitorKind) =>
    eligibility.layerCount() > 0 && (kind === "program"
      ? eligibility.programOutputId() !== null
      : eligibility.previewLayerId() !== null);

  const idleStatus = (kind: LiveVideoMonitorKind): LiveVideoMonitorStatus => {
    if (!eligibility.backendAvailable()) return "desktop-required";
    if (!eligibility.active() || pageHidden()) return "paused";
    if (eligibility.layerCount() === 0) return "empty";
    if (kind === "preview" && eligibility.previewLayerId() === null) return "idle";
    if (kind === "program" && eligibility.programOutputId() === null) return "idle";
    return "starting";
  };

  const syncIdleState = (kind: LiveVideoMonitorKind) => {
    if (canRun() && kindEligible(kind)) {
      setBus(kind, (current) => current.status === "live" && (current.frameUrl || current.frame)
        ? current
        : { ...current, status: "starting", error: null });
      return;
    }
    setBus(kind, (current) => ({ ...current, status: idleStatus(kind), error: null }));
  };

  const targetFps = (kind: LiveVideoMonitorKind) => monitorTargetFps(
    eligibility.quality(), eligibility.previewActive(),
  )[kind];
  const measuredFps = (kind: LiveVideoMonitorKind, now: number) => {
    const times = runtime[kind].frameTimes;
    while (times.length > 0 && times[0] < now - 1_000) times.shift();
    return times.length >= 2 ? (times.length - 1) * 1_000 / Math.max(1, now - times[0]) : 0;
  };
  const intervalMs = (kind: LiveVideoMonitorKind) => pressure.intervalMs(performance.now(), targetFps(kind));
  const schedule = (kind: LiveVideoMonitorKind, delayMs: number) => {
    clearTimer(kind);
    if (disposed || !canRun() || runtime[kind].inFlight || !kindEligible(kind)) return;
    runtime[kind].timerId = window.setTimeout(() => void runNext(kind), Math.max(0, delayMs));
  };

  const runNext = async (kind: LiveVideoMonitorKind) => {
    runtime[kind].timerId = null;
    if (disposed || runtime[kind].inFlight || !canRun() || !kindEligible(kind)) return;
    const now = performance.now();
    const outputPauseMs = pressure.outputPauseMs(now);
    if (outputPauseMs > 0) {
      setBus(kind, (current) => ({ ...current, throttled: true, fps: measuredFps(kind, now) }));
      // One-second pauses yield to real output; stagger recovery probes by bus.
      schedule(kind, outputPauseMs + (kind === "preview" ? intervalMs(kind) / 2 : 0));
      return;
    }
    const waitMs = runtime[kind].dueAt - now;
    if (waitMs > 1) {
      schedule(kind, waitMs);
      return;
    }

    const requestGeneration = generation;
    const targetId = kind === "program" ? eligibility.programOutputId() : eligibility.previewLayerId();
    runtime[kind].inFlight = true;
    const budgetMs = intervalMs(kind);
    runtime[kind].dueAt = now + budgetMs;
    try {
      const response = await options.invoke<unknown>("get_live_video_monitor_frame", {
        monitorKind: kind,
        outputId: kind === "program" ? targetId : null,
        layerId: kind === "preview" ? targetId : null,
        width: options.width ?? 320,
        height: options.height ?? 180,
        quality: options.jpegQuality ?? 68,
        pixelFormat: options.pixelFormat ?? "jpeg",
      });
      if (disposed || requestGeneration !== generation) return;
      const packet = parseLiveVideoMonitorPacket(response);
      if (packet.encoding !== (options.pixelFormat ?? "jpeg")) throw new Error("Live monitor returned an unexpected pixel format.");
      if (packet.kind !== kind) throw new Error(`Live monitor returned ${packet.kind} for ${kind}.`);
      if (packet.status === "busy") {
        pressure.observeMonitor(kind, performance.now(), true, 0, budgetMs);
        setBus(kind, (current) => ({ ...current, busyDrops: current.busyDrops + 1, throttled: pressure.throttled(), fps: measuredFps(kind, performance.now()) }));
        return;
      }

      let nextUrl: string | null = null;
      let nextFrame: ImageBitmap | ImageData | null = null;
      if ((options.presentation ?? "canvas") === "canvas") {
        nextFrame = packet.encoding === "rgba"
          ? new ImageData(packet.rgba, packet.width, packet.height)
          : await createImageBitmap(new Blob([packet.jpeg], { type: "image/jpeg" }));
      } else {
        if (packet.encoding !== "jpeg") throw new Error("Image monitors require JPEG packets.");
        nextUrl = URL.createObjectURL(new Blob([packet.jpeg], { type: "image/jpeg" }));
      }
      // The source can change while JPEG decoding awaits the browser worker.
      if (disposed || requestGeneration !== generation) {
        closeFrame(nextFrame);
        if (nextUrl) URL.revokeObjectURL(nextUrl);
        return;
      }
      const previousUrl = runtime[kind].frameUrl;
      const previousFrame = runtime[kind].frame;
      runtime[kind].frameUrl = nextUrl;
      runtime[kind].frame = nextFrame;
      const frameAt = performance.now();
      pressure.observeMonitor(kind, frameAt, false, packet.renderUs / 1_000 + packet.encodeUs / 1_000, budgetMs);
      runtime[kind].frameTimes.push(frameAt);
      runtime[kind].errorCount = 0;
      setBus(kind, (current) => ({
        ...current,
        status: "live",
        frameUrl: nextUrl,
        frame: nextFrame,
        throttled: pressure.throttled(),
        fps: measuredFps(kind, frameAt),
        renderMs: packet.renderUs / 1_000,
        encodeMs: packet.encodeUs / 1_000,
        width: packet.width,
        height: packet.height,
        sequence: packet.sequence,
        ptsMs: packet.ptsMs,
        error: null,
      }));
      if (previousUrl) URL.revokeObjectURL(previousUrl);
      closeFrame(previousFrame);
    } catch (error) {
      if (disposed || requestGeneration !== generation) return;
      runtime[kind].errorCount += 1;
      runtime[kind].dueAt = performance.now() + Math.min(4_000, 250 * (2 ** (runtime[kind].errorCount - 1)));
      setBus(kind, (current) => ({ ...current, status: "error", error: cleanError(error) }));
    } finally {
      runtime[kind].inFlight = false;
      // Drop elapsed slots. Never catch up with bursts after slow IPC/decode.
      const finishedAt = performance.now();
      const interval = intervalMs(kind);
      if (runtime[kind].dueAt < finishedAt) runtime[kind].dueAt = finishedAt + interval;
      schedule(kind, runtime[kind].dueAt - finishedAt);
    }
  };

  const invalidate = () => {
    generation += 1;
    const now = performance.now();
    for (const kind of busKinds) {
      clearTimer(kind);
      runtime[kind].dueAt = now;
      schedule(kind, kind === "preview" ? intervalMs(kind) / 2 : 0);
    }
  };

  const retry = () => {
    for (const kind of busKinds) runtime[kind].errorCount = 0;
    invalidate();
  };

  const onVisibilityChange = () => setPageHidden(document.hidden);
  if (typeof document !== "undefined") document.addEventListener("visibilitychange", onVisibilityChange);

  createEffect(() => {
    const now = performance.now();
    pressure.observeOutputs(now, options.outputHealth?.() ?? []);
    if (options.outputHealthError?.()) pressure.observeUnavailable(now);
    for (const kind of busKinds) setBus(kind, current => ({ ...current, fps: measuredFps(kind, now), throttled: pressure.throttled() }));
  });

  createEffect(() => {
    eligibility.backendAvailable();
    eligibility.active();
    eligibility.layerCount();
    pageHidden();
    eligibility.quality();
    eligibility.previewActive();
    const previewLayerId = eligibility.previewLayerId();
    const previewTransportGeneration = eligibility.previewTransportGeneration();
    const programOutputId = eligibility.programOutputId();
    if (previewLayerId !== lastPreviewLayerId || previewTransportGeneration !== lastPreviewTransportGeneration) {
      lastPreviewLayerId = previewLayerId;
      lastPreviewTransportGeneration = previewTransportGeneration;
      revokeBusUrl("preview");
      runtime.preview.frameTimes.length = 0;
      setPreview((current) => ({ ...current, frameUrl: null, frame: null, fps: 0, error: null }));
    }
    if (programOutputId !== lastProgramOutputId) {
      lastProgramOutputId = programOutputId;
      revokeBusUrl("program");
      runtime.program.frameTimes.length = 0;
      setProgram((current) => ({ ...current, frameUrl: null, frame: null, fps: 0, error: null }));
    }
    syncIdleState("program");
    syncIdleState("preview");
    invalidate();
  });

  onCleanup(() => {
    disposed = true;
    generation += 1;
    for (const kind of busKinds) clearTimer(kind);
    if (typeof document !== "undefined") document.removeEventListener("visibilitychange", onVisibilityChange);
    revokeBusUrl("program");
    revokeBusUrl("preview");
  });

  return { program, preview, retry };
};
