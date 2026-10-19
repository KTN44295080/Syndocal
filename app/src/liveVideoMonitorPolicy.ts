/** Monitor-only preference and pressure policy. This never changes output cadence. */
export const liveVideoMonitorQualityOptions = [
  { id: "smooth", label: "Smooth" },
  { id: "balanced", label: "Balanced" },
  { id: "low-cpu", label: "Low CPU" },
] as const;
export type LiveVideoMonitorQuality = typeof liveVideoMonitorQualityOptions[number]["id"];
export const liveVideoMonitorQualityStorageKey = "syndocal.monitorQuality.v1";

export const monitorQualityFromUnknown = (value: unknown): LiveVideoMonitorQuality =>
  liveVideoMonitorQualityOptions.find((option) => option.id === value)?.id ?? "smooth";

export function loadLiveVideoMonitorQuality(): LiveVideoMonitorQuality {
  try { return monitorQualityFromUnknown(window.localStorage.getItem(liveVideoMonitorQualityStorageKey)); }
  catch { return "smooth"; }
}

export function saveLiveVideoMonitorQuality(quality: LiveVideoMonitorQuality): void {
  try { window.localStorage.setItem(liveVideoMonitorQualityStorageKey, quality); }
  catch { /* Keep the session preference when storage is unavailable. */ }
}

export interface LiveVideoOutputHealth {
  outputId: number;
  kind: "display" | "spout" | "ndi";
  deliveredFps: number;
  frameTimeP50Ms: number;
  frameTimeP95Ms: number;
  frameTimeP99Ms: number;
  droppedFrames: number;
  targetFps: number;
  deliveredFrames: number;
  sampleCount: number;
  measurementBoundary: "present" | "sender";
  lastFrameAgeMs: number | null;
  /** Missed intervals during the last `recentWindowSeconds`, derived in the UI from the cumulative deficit. */
  recentMissedFrames?: number;
  recentWindowSeconds?: number;
}

/** Recent-window missed intervals: more than this in the window shows the warning tone. */
export const outputMissedWarningThreshold = 6;
export const outputMissedWindowSeconds = 10;

/** Counts missed intervals inside a sliding window. The first sample of an output is the
 * baseline, so deficit accumulated before the operator could see it (startup) never counts. */
export function createRecentMissedTracker(windowMs = outputMissedWindowSeconds * 1_000) {
  const history = new Map<string, Array<{ at: number; dropped: number }>>();
  return (now: number, outputs: readonly LiveVideoOutputHealth[]): LiveVideoOutputHealth[] => {
    const seen = new Set<string>();
    const result = outputs.map((output) => {
      const key = `${output.kind}:${output.outputId}`;
      seen.add(key);
      let samples = history.get(key) ?? [];
      // A smaller cumulative value means the output restarted: forget its history.
      if (samples.length > 0 && output.droppedFrames < samples[samples.length - 1].dropped) samples = [];
      samples.push({ at: now, dropped: output.droppedFrames });
      while (samples.length > 1 && samples[1].at <= now - windowMs) samples.shift();
      history.set(key, samples);
      return { ...output, recentMissedFrames: output.droppedFrames - samples[0].dropped, recentWindowSeconds: windowMs / 1_000 };
    });
    for (const key of history.keys()) if (!seen.has(key)) history.delete(key);
    return result;
  };
}

export function monitorTargetFps(quality: LiveVideoMonitorQuality, previewActive: boolean) {
  if (quality === "low-cpu") return { program: 10, preview: 5 };
  if (quality === "balanced") return { program: 15, preview: 15 };
  return { program: 30, preview: previewActive ? 30 : 15 };
}

/** A one-slot-per-second deficit is ordinary sleep/vsync jitter; a burst of missed slots in one poll is contention. */
const MISSED_SLOT_BURST = 3;

/** Shared protection tier: react promptly, recover one tier after five healthy seconds.
 * Calls do not create timers, queues or output mutations. Native telemetry counts
 * missed target slots at present/send acceptance, not physical receiver losses. */
export function createMonitorPressurePolicy() {
  let tier = 0;
  let lastPressureAt = Number.NEGATIVE_INFINITY;
  let lastIncreaseAt = Number.NEGATIVE_INFINITY;
  let busyWindowAt = 0;
  let busyCount = 0;
  const slowCount = { program: 0, preview: 0 };
  let unhealthyOutput = false;
  let pauseUntil = 0;
  const dropped = new Map<string, number>();
  const p95Baselines = new Map<string, number>();
  const pressure = (now: number) => {
    lastPressureAt = now;
    if (now - lastIncreaseAt >= 1_000) {
      tier = Math.min(3, tier + 1);
      lastIncreaseAt = now;
    }
  };
  return {
    observeMonitor(kind: "program" | "preview", now: number, busy: boolean, processingMs: number, budgetMs: number) {
      if (now - busyWindowAt >= 1_000) { busyWindowAt = now; busyCount = 0; }
      if (busy) busyCount += 1;
      if (!busy) slowCount[kind] = processingMs > budgetMs * 0.6 ? slowCount[kind] + 1 : 0;
      if (busyCount >= 3 || slowCount[kind] >= 3) pressure(now);
    },
    observeOutputs(now: number, outputs: readonly LiveVideoOutputHealth[]) {
      let pressureObserved = false;
      let severePressure = false;
      const activeKeys = new Set<string>();
      for (const output of outputs) {
        const key = `${output.kind}:${output.outputId}`;
        activeKeys.add(key);
        const previousDrops = dropped.get(key);
        dropped.set(key, output.droppedFrames);
        if (output.targetFps <= 0) continue;
        const budget = 1_000 / output.targetFps;
        // A producer can stall after its first present, before cadence has two
        // intervals. Age protection must not depend on a completed percentile.
        if (output.lastFrameAgeMs !== null && output.lastFrameAgeMs > Math.max(100, budget * 3)) {
          pressureObserved = true;
          if (output.lastFrameAgeMs > Math.max(250, budget * 6)) severePressure = true;
        }
        if (output.sampleCount < 2) continue;
        if (output.frameTimeP95Ms > budget * 2 && output.deliveredFps < output.targetFps * 0.8) severePressure = true;
        const baseline = p95Baselines.get(key);
        if (output.sampleCount >= 15 && output.frameTimeP95Ms > 0) {
          p95Baselines.set(key, Math.min(baseline ?? output.frameTimeP95Ms, output.frameTimeP95Ms));
        }
        if (output.frameTimeP95Ms > budget * 1.35 ||
          (baseline !== undefined && output.frameTimeP95Ms > Math.max(budget * 1.1, baseline * 1.2)) ||
          (output.lastFrameAgeMs !== null && output.lastFrameAgeMs > Math.max(100, budget * 3)) ||
          (previousDrops !== undefined && output.droppedFrames >= previousDrops + MISSED_SLOT_BURST)) {
          pressureObserved = true;
        }
      }
      for (const key of dropped.keys()) if (!activeKeys.has(key)) { dropped.delete(key); p95Baselines.delete(key); }
      unhealthyOutput = pressureObserved;
      if (pressureObserved) pressure(now);
      if (severePressure) pauseUntil = now + 1_000;
    },
    observeUnavailable(now: number) {
      // Telemetry failure must not restore full monitor load. Keep output untouched.
      tier = Math.max(tier, 2);
      lastPressureAt = now;
    },
    intervalMs(now: number, targetFps: number) {
      if (unhealthyOutput) pressure(now);
      if (!unhealthyOutput && tier > 0 && now - lastPressureAt >= 5_000) {
        tier -= 1;
        lastPressureAt = now;
        busyCount = 0;
        slowCount.program = 0;
        slowCount.preview = 0;
      }
      return 1_000 / Math.max(3, targetFps * [1, 0.75, 0.5, 0.25][tier]);
    },
    throttled: () => tier > 0,
    outputPauseMs: (now: number) => Math.max(0, pauseUntil - now),
  };
}
