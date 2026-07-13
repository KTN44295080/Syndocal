import { Show } from "solid-js";
import type { LiveVideoMonitorBusState, LiveVideoMonitorStatus } from "../createLiveVideoMonitorController";

export interface LiveVideoMonitorPanelProps {
  preview: LiveVideoMonitorBusState;
  program: LiveVideoMonitorBusState;
  previewLabel: string | null;
  programLabel: string | null;
  onRetry: () => void;
}

const statusLabel = (kind: "preview" | "program", status: LiveVideoMonitorStatus) => {
  switch (status) {
    case "desktop-required": return "Desktop required";
    case "paused": return "Paused";
    case "idle": return kind === "preview" ? "Select a clip" : "Select an output";
    case "empty": return "No video source";
    case "starting": return "Starting";
    case "live": return "LIVE";
    case "error": return "Monitor offline";
  }
};

const fallbackLabel = (kind: "preview" | "program", status: LiveVideoMonitorStatus) => {
  if (status === "desktop-required") return "Open the desktop app for live video.";
  if (status === "paused") return "Monitor paused outside the VJ desk.";
  if (status === "error") return "The last frame is preserved while the monitor retries.";
  return kind === "preview" ? "Stage a clip with its P button." : "Waiting for a program source.";
};

const timingLabel = (state: LiveVideoMonitorBusState) => {
  if (state.status !== "live") return null;
  const processingMs = state.renderMs + state.encodeMs;
  return `${state.fps.toFixed(1)} fps · ${processingMs.toFixed(1)} ms`;
};

interface BusProps {
  kind: "preview" | "program";
  state: LiveVideoMonitorBusState;
  label: string | null;
}

function LiveVideoMonitorBus(props: BusProps) {
  const title = () => props.kind === "preview" ? "PREVIEW" : "PROGRAM";
  return (
    <section
      class={`liveVideoMonitor liveVideoMonitor-${props.kind}`}
      data-live-video-monitor={props.kind}
      aria-label={`${title()} live video monitor`}
    >
      <header>
        <strong>{title()}</strong>
        <Show
          when={props.label}
          fallback={<span>{props.kind === "preview" ? "Staged clip" : "Selected output"}</span>}
        >
          {(label) => <span data-no-localize title={label()}>{label()}</span>}
        </Show>
      </header>
      <div class="liveVideoMonitorViewport">
        <Show
          when={props.state.frameUrl}
          fallback={<span>{fallbackLabel(props.kind, props.state.status)}</span>}
        >
          {(url) => (
            <img
              src={url()}
              alt={props.kind === "preview" ? "Live preview monitor" : "Live program monitor"}
              draggable={false}
            />
          )}
        </Show>
      </div>
      <footer>
        <span class={`liveVideoMonitorStatus ${props.state.status}`}>{statusLabel(props.kind, props.state.status)}</span>
        <Show when={timingLabel(props.state)}>
          {(timing) => <span class="tabularNums" data-no-localize>{timing()}</span>}
        </Show>
        <Show when={props.state.busyDrops > 0}>
          <span class="tabularNums" data-no-localize>{props.state.busyDrops} busy</span>
        </Show>
      </footer>
      <Show when={props.state.error}>
        {(error) => <small class="liveVideoMonitorError" data-no-localize title={error()}>{error()}</small>}
      </Show>
    </section>
  );
}

export function LiveVideoMonitorPanel(props: LiveVideoMonitorPanelProps) {
  const failed = () => props.preview.status === "error" || props.program.status === "error";
  return (
    <div class="liveVideoMonitorPanel" aria-label="Live video monitors">
      <div class="liveVideoMonitorBuses">
        <LiveVideoMonitorBus kind="preview" state={props.preview} label={props.previewLabel} />
        <LiveVideoMonitorBus kind="program" state={props.program} label={props.programLabel} />
      </div>
      <Show when={failed()}>
        <button class="liveVideoMonitorRetry" onClick={props.onRetry}>Retry monitors</button>
      </Show>
    </div>
  );
}
