import { createSignal, Show } from "solid-js";
import type { LiveVideoMonitorBusState, LiveVideoMonitorStatus } from "../createLiveVideoMonitorController";
import type { VjPreviewTransportSummary } from "../types";

export interface LiveVideoMonitorPanelProps {
  preview: LiveVideoMonitorBusState;
  program: LiveVideoMonitorBusState;
  previewLabel: string | null;
  programLabel: string | null;
  previewTransport: VjPreviewTransportSummary;
  previewTransportBusy: boolean;
  previewTransportError: string | null;
  previewTransportBackendAvailable: boolean;
  onSetPreviewPlaying: (playing: boolean) => void | Promise<void>;
  onSeekPreview: (positionMs: number) => void | Promise<void>;
  onSetPreviewSpeed: (speed: number) => void | Promise<void>;
  onClearPreview: () => void | Promise<unknown>;
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

const formatTransportTime = (milliseconds: number | null) => {
  if (milliseconds === null || !Number.isFinite(milliseconds)) return "--:--.---";
  const clamped = Math.max(0, Math.round(milliseconds));
  const minutes = Math.floor(clamped / 60_000);
  const seconds = Math.floor((clamped % 60_000) / 1_000);
  const millis = clamped % 1_000;
  return `${minutes.toString().padStart(2, "0")}:${seconds.toString().padStart(2, "0")}.${millis.toString().padStart(3, "0")}`;
};

function PreviewTransport(props: LiveVideoMonitorPanelProps) {
  const [seekDraftMs, setSeekDraftMs] = createSignal<number | null>(null);
  const [seekScrubbing, setSeekScrubbing] = createSignal(false);
  let seekDraftCommitted = false;
  let seekCommitGeneration = 0;
  const staged = () => props.previewTransport.layer_id !== null;
  const controlsDisabled = () =>
    !props.previewTransportBackendAvailable || !staged() || props.previewTransportBusy;
  const seekMaximum = () => Math.max(
    1,
    props.previewTransport.duration_ms ?? props.previewTransport.position_ms,
  );
  const clampSeekPosition = (positionMs: number) => Math.max(
    0,
    Math.min(seekMaximum(), Math.round(positionMs)),
  );
  const displayedSeekPosition = () => clampSeekPosition(
    seekDraftMs() ?? props.previewTransport.position_ms,
  );
  const updateSeekDraft = (positionMs: number) => {
    seekDraftCommitted = false;
    setSeekDraftMs(clampSeekPosition(positionMs));
  };
  const cancelSeekDraft = () => {
    seekDraftCommitted = false;
    setSeekScrubbing(false);
    setSeekDraftMs(null);
  };
  const commitSeekDraft = (positionMs: number) => {
    const nextPositionMs = clampSeekPosition(positionMs);
    setSeekDraftMs(nextPositionMs);
    setSeekScrubbing(false);
    if (seekDraftCommitted) return;

    seekDraftCommitted = true;
    const commitGeneration = ++seekCommitGeneration;
    const releaseSeekDraft = () => {
      if (seekCommitGeneration === commitGeneration && !seekScrubbing()) {
        setSeekDraftMs(null);
      }
    };
    void Promise.resolve(props.onSeekPreview(nextPositionMs)).then(releaseSeekDraft, releaseSeekDraft);
  };
  const loopLabel = () => {
    if (!staged()) return "LOOP —";
    if (!props.previewTransport.loop_enabled) return "LOOP OFF";
    return `LOOP ON · ${formatTransportTime(props.previewTransport.loop_start_ms)}–${formatTransportTime(props.previewTransport.loop_end_ms)}`;
  };
  const slowerSpeed = () => props.previewTransport.speed === 0.25
    ? -0.25
    : Math.max(-4, props.previewTransport.speed - 0.25);
  const fasterSpeed = () => props.previewTransport.speed === -0.25
    ? 0.25
    : Math.min(4, props.previewTransport.speed + 0.25);
  return (
    <section
      class="vjPreviewTransport"
      aria-label="Preview transport"
      aria-busy={props.previewTransportBusy}
    >
      <header>
        <strong>PREVIEW TRANSPORT</strong>
        <span class="tabularNums">{loopLabel()}</span>
      </header>
      <div class="vjPreviewTransportControls">
        <button
          class="vjPreviewPlay"
          aria-label={props.previewTransport.playing ? "Pause preview" : "Play preview"}
          title={props.previewTransport.playing ? "Pause preview" : "Play preview"}
          disabled={controlsDisabled()}
          onClick={() => void props.onSetPreviewPlaying(!props.previewTransport.playing)}
        >
          {props.previewTransport.playing ? "❚❚" : "▶"}
        </button>
        <label class="vjPreviewSeek">
          <span class="tabularNums">
            {formatTransportTime(displayedSeekPosition())} / {formatTransportTime(props.previewTransport.duration_ms)}
          </span>
          <input
            type="range"
            min="0"
            max={seekMaximum()}
            step="1"
            value={displayedSeekPosition()}
            disabled={controlsDisabled()}
            aria-label="Preview position"
            aria-valuetext={formatTransportTime(displayedSeekPosition())}
            onPointerDown={() => {
              seekDraftCommitted = false;
              setSeekDraftMs(displayedSeekPosition());
              setSeekScrubbing(true);
            }}
            onInput={(event) => updateSeekDraft(Number(event.currentTarget.value))}
            onChange={(event) => commitSeekDraft(Number(event.currentTarget.value))}
            onPointerUp={(event) => {
              if (seekScrubbing()) commitSeekDraft(Number(event.currentTarget.value));
            }}
            onPointerCancel={cancelSeekDraft}
          />
        </label>
        <div class="vjPreviewSpeed" aria-label="Preview speed">
          <button
            aria-label="Decrease preview speed"
            title="Decrease preview speed"
            disabled={controlsDisabled() || props.previewTransport.speed <= -4}
            onClick={() => void props.onSetPreviewSpeed(slowerSpeed())}
          >
            −
          </button>
          <output class="tabularNums" aria-live="polite">
            {props.previewTransport.speed.toFixed(2)}×
          </output>
          <button
            aria-label="Increase preview speed"
            title="Increase preview speed"
            disabled={controlsDisabled() || props.previewTransport.speed >= 4}
            onClick={() => void props.onSetPreviewSpeed(fasterSpeed())}
          >
            +
          </button>
        </div>
        <button
          class="vjPreviewClear"
          disabled={controlsDisabled()}
          onClick={() => void props.onClearPreview()}
        >
          Clear
        </button>
      </div>
      <Show when={!props.previewTransportBackendAvailable}>
        <small class="vjPreviewTransportNotice">Desktop required — preview transport is unavailable in the browser fixture.</small>
      </Show>
      <Show when={props.previewTransportError}>
        {(error) => <small class="vjPreviewTransportError" role="alert" data-no-localize>{error()}</small>}
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
      <PreviewTransport {...props} />
      <Show when={failed()}>
        <button class="liveVideoMonitorRetry" onClick={props.onRetry}>Retry monitors</button>
      </Show>
    </div>
  );
}
