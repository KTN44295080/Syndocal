import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { TakeoverRecordingStatus, VideoRecordingStatus } from "../types";

const TAKEOVER_RECORDING_POLL_MS = 2_000;

interface TakeoverRecordingRowProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
}

type TakeoverRecordingAction = "choose_folder" | "clear_folder" | "cancel_pending";

const takeoverStateLabel = (status: TakeoverRecordingStatus | null): string => {
  switch (status?.state) {
    case "pending": return "Pending";
    case "blocked": return "Blocked";
    case "started": return "Takeover take started";
    case "failed": return "Failed";
    case "cancelled": return "Cancelled";
    default: return "Idle";
  }
};

/**
 * DEC-RECORD-OWN-001 phase 3: the folder a takeover take records into, and
 * whether one is waiting. After Take Over and Arm, the previous Primary's
 * recording continues here automatically once its take has provably ended.
 */
export function TakeoverRecordingRow(props: TakeoverRecordingRowProps) {
  const [status, setStatus] = createSignal<TakeoverRecordingStatus | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let disposed = false;

  const refresh = async () => {
    try {
      const recording = await props.invokeCommand<VideoRecordingStatus>("video_output_recording_status");
      if (!disposed) setStatus(recording.takeover ?? null);
    } catch { /* Background status polling is best-effort. */ }
  };

  const run = async (action: TakeoverRecordingAction) => {
    setBusy(true);
    setError(null);
    try {
      const next = await props.invokeCommand<TakeoverRecordingStatus>("configure_takeover_recording_v1", { action });
      if (!disposed) setStatus(next);
    } catch (failure) {
      if (!disposed) setError(String(failure));
    } finally {
      if (!disposed) setBusy(false);
    }
  };

  onMount(() => {
    if (!props.backendAvailable) return;
    void refresh();
    const timer = window.setInterval(() => void refresh(), TAKEOVER_RECORDING_POLL_MS);
    onCleanup(() => {
      disposed = true;
      window.clearInterval(timer);
    });
  });

  return (
    <div class="takeoverRecordingRow" aria-label="Takeover recording">
      <label>
        Takeover recording folder
        <div class="standbyDirectoryRow">
          <input
            value={status()?.folder ?? ""}
            readOnly
            placeholder="Not set: a takeover cannot continue the recording"
          />
          <button onClick={() => void run("choose_folder")} disabled={!props.backendAvailable || busy()}>Browse</button>
          <button onClick={() => void run("clear_folder")} disabled={!props.backendAvailable || busy() || !status()?.folder}>Clear</button>
        </div>
      </label>
      <small class="textPretty">
        After Take Over and Arm, the previous Primary's recording continues here once its take has ended.
      </small>
      <Show when={status() && status()!.state !== "idle"}>
        <p class={status()!.state === "failed" || status()!.state === "blocked" ? "inlineWarning textPretty" : "textPretty"} role="status" aria-live="polite">
          <strong>{takeoverStateLabel(status())}</strong>
          <Show when={status()!.reason}>{" · "}<span>{status()!.reason}</span></Show>
          <Show when={status()!.state === "pending" || status()!.state === "blocked"}>
            {" "}
            <button onClick={() => void run("cancel_pending")} disabled={busy()}>Cancel</button>
          </Show>
        </p>
      </Show>
      <Show when={error()}>
        <p class="fieldError textPretty" role="alert">{error()}</p>
      </Show>
    </div>
  );
}
