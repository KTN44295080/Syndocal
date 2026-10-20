import { createEffect, createSignal, onCleanup, type Accessor } from "solid-js";
import type { FrontendTauriInvoke } from "./tauriInvokeCommands";
import { createRecentMissedTracker, type LiveVideoOutputHealth } from "./liveVideoMonitorPolicy";

/** One read-only telemetry request at a time; hidden/pane teardown fences replies. */
export function createLiveVideoOutputHealthController(options: {
  invoke: FrontendTauriInvoke;
  backendAvailable: Accessor<boolean>;
}) {
  const [outputs, setOutputs] = createSignal<LiveVideoOutputHealth[]>([]);
  const [error, setError] = createSignal<string | null>(null);
  const [hidden, setHidden] = createSignal(document.hidden);
  let generation = 0;
  let disposed = false;
  let inFlight = false;
  let timer: number | null = null;
  let expiryTimer: number | null = null;
  const recentMissed = createRecentMissedTracker();
  const clearTimer = () => { if (timer !== null) window.clearTimeout(timer); timer = null; };
  const clearExpiry = () => { if (expiryTimer !== null) window.clearTimeout(expiryTimer); expiryTimer = null; };
  const canRun = () => !disposed && options.backendAvailable() && !hidden();
  const schedule = (delay: number) => {
    clearTimer();
    if (canRun() && !inFlight) timer = window.setTimeout(() => void poll(), delay);
  };
  const poll = async () => {
    timer = null;
    if (inFlight || !canRun()) return;
    const requestGeneration = generation;
    let expired = false;
    inFlight = true;
    // A hung query must not leave old preparation numbers advertising headroom.
    // Expire only the UI evidence; keep the request single-flight until it settles.
    expiryTimer = window.setTimeout(() => {
      expiryTimer = null;
      expired = true;
      if (disposed || requestGeneration !== generation) return;
      setOutputs(recentMissed(performance.now(), []));
      setError("Output health unavailable");
    }, 1_000);
    try {
      const next = await options.invoke<LiveVideoOutputHealth[]>("get_live_video_output_health");
      if (expired || disposed || requestGeneration !== generation) return;
      setOutputs(recentMissed(performance.now(), next));
      setError(null);
    } catch {
      if (expired || disposed || requestGeneration !== generation) return;
      setOutputs(recentMissed(performance.now(), []));
      setError("Output health unavailable");
    } finally {
      clearExpiry();
      inFlight = false;
      schedule(!expired && requestGeneration === generation ? 1_000 : 0);
    }
  };
  const visibility = () => setHidden(document.hidden);
  document.addEventListener("visibilitychange", visibility);
  createEffect(() => {
    options.backendAvailable(); hidden();
    generation += 1;
    clearTimer(); clearExpiry();
    if (!options.backendAvailable() || hidden()) { setOutputs([]); setError(null); }
    schedule(0);
  });
  onCleanup(() => {
    disposed = true; generation += 1; clearTimer(); clearExpiry();
    document.removeEventListener("visibilitychange", visibility);
  });
  return { outputs, error };
}
