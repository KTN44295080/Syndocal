import { For, onMount, Show, createSignal } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { VideoOutputKind } from "../types";

type MaybePromise = void | Promise<unknown>;

export interface VideoDisplayMonitorDescriptor {
  index: number;
  identity: string;
  name: string;
  physicalWidth: number;
  physicalHeight: number;
  positionX: number;
  positionY: number;
  scaleFactor: number;
}

type VideoOutputCreatePanelProps = {
  label: string;
  kind: VideoOutputKind;
  width: number;
  height: number;
  fadeMs: number;
  monitorId: number;
  fullscreen: boolean;
  endpoint: string;
  onLabel: (value: string) => void;
  onKind: (value: VideoOutputKind) => void;
  onWidth: (value: number) => void;
  onHeight: (value: number) => void;
  onFadeMs: (value: number) => void;
  onMonitorId: (value: number) => void;
  onFullscreen: (value: boolean) => void;
  onEndpoint: (value: string) => void;
  invokeCommand: FrontendTauriInvoke;
  onAddDisplayOutput: (monitor: VideoDisplayMonitorDescriptor) => MaybePromise;
};

export function VideoOutputCreatePanel(props: VideoOutputCreatePanelProps) {
  const [monitors, setMonitors] = createSignal<VideoDisplayMonitorDescriptor[]>([]);
  const [selectedMonitorIdentity, setSelectedMonitorIdentity] = createSignal<string | null>(null);
  const [monitorDiscovery, setMonitorDiscovery] = createSignal<"idle" | "loading" | "ready" | "unavailable">("idle");

  const isTauriRuntime = () =>
    typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

  const monitorLabel = (monitor: VideoDisplayMonitorDescriptor) => {
    const name = monitor.name.trim() || `Display ${monitor.index + 1}`;
    return `${name} · ${monitor.physicalWidth}x${monitor.physicalHeight}`;
  };

  const selectMonitor = (monitorIndex: number) => {
    const monitor = monitors().find((candidate) => candidate.index === monitorIndex);
    if (!monitor) return;
    setSelectedMonitorIdentity(monitor.identity);
    props.onMonitorId(monitor.index);
    props.onWidth(Math.max(1, Math.round(monitor.physicalWidth)));
    props.onHeight(Math.max(1, Math.round(monitor.physicalHeight)));
    props.onFullscreen(true);
    props.onLabel(monitor.name.trim() || `Display ${monitor.index + 1}`);
  };

  onMount(() => {
    if (!isTauriRuntime()) {
      setMonitorDiscovery("unavailable");
      return;
    }
    setMonitorDiscovery("loading");
    void props.invokeCommand<VideoDisplayMonitorDescriptor[]>("list_video_display_monitors")
      .then((nextMonitors) => {
        const validMonitors = nextMonitors.filter((monitor) =>
          Number.isInteger(monitor.index) && monitor.index >= 0
          && typeof monitor.identity === "string" && /^[0-9a-f]{64}$/.test(monitor.identity)
          && typeof monitor.name === "string"
          && Number.isInteger(monitor.physicalWidth) && monitor.physicalWidth > 0
          && Number.isInteger(monitor.physicalHeight) && monitor.physicalHeight > 0
          && Number.isFinite(monitor.scaleFactor) && monitor.scaleFactor > 0,
        );
        setMonitors(validMonitors);
        setMonitorDiscovery(validMonitors.length > 0 ? "ready" : "unavailable");
        const currentId = Math.max(0, Math.round(props.monitorId));
        const current = validMonitors.find((monitor) => monitor.index === currentId);
        if (current) selectMonitor(current.index);
      })
      .catch(() => setMonitorDiscovery("unavailable"));
  });

  return (
    <section class="videoSetupDisclosure videoOutputCreateSurface" aria-label="New Output">
      <div class="videoSetupDisclosureHeading">New Output</div>
      <div class="videoOutputForm">
        <div class="videoOutputQuickCreate" data-video-output-quick-create>
          <Show when={props.kind === "Display"} fallback={
            <>
              <p class="hint videoOutputQuickHint" role="status">
                Configure the output type and endpoint in Advanced output settings.
              </p>
              <button class="primary" data-video-output-add disabled>
                Add configured output
              </button>
              <p class="inlineWarning" role="status" data-video-output-kind-status>
                External output creation is unavailable until its canonical transport is configured; nothing was applied.
              </p>
            </>
          }>
            <label>
              Screen
              <select
                data-video-output-display-target
                value={props.monitorId}
                disabled={monitorDiscovery() !== "ready" || monitors().length === 0}
                onChange={(event) => selectMonitor(Number(event.currentTarget.value))}
              >
                <Show when={monitors().length > 0} fallback={<option value={props.monitorId} disabled>Screen detection unavailable</option>}>
                  <For each={monitors()}>{(monitor) => <option value={monitor.index}>{monitorLabel(monitor)}</option>}</For>
                </Show>
              </select>
            </label>
            <button
              class="primary"
              data-video-output-add
              disabled={monitorDiscovery() !== "ready" || monitors().length === 0}
              onClick={() => {
                const monitor = monitors().find((candidate) => candidate.identity === selectedMonitorIdentity())
                  ?? monitors().find((candidate) => candidate.index === props.monitorId);
                if (monitor) void props.onAddDisplayOutput(monitor);
              }}
            >
              Add display output
            </button>
            <p class="hint videoOutputQuickHint">
              {`${props.label} · ${props.width}x${props.height} · fullscreen`}
            </p>
            <Show when={monitorDiscovery() === "loading"}>
              <p class="hint" role="status" data-video-output-monitor-status>Detecting connected screens…</p>
            </Show>
            <Show when={monitorDiscovery() === "unavailable"}>
              <p class="inlineWarning" role="alert" data-video-output-monitor-status>
                No connected screen was detected. Refresh the display list or check Windows display settings.
              </p>
            </Show>
          </Show>
        </div>

        <details class="ioDisclosure videoOutputAdvanced" data-video-output-advanced>
          <summary>Advanced output settings</summary>
          <div class="ioDisclosureBody">
            <div class="split">
              <label>
                Output label
                <input value={props.label} onInput={(event) => props.onLabel(event.currentTarget.value)} />
              </label>
              <label>
                Kind
                <select value={props.kind} onInput={(event) => props.onKind(event.currentTarget.value as VideoOutputKind)}>
                  <option value="Display">Display</option>
                  <option value="NdiSender">NDI Sender</option>
                  <option value="SpoutSender">Spout Sender</option>
                  <option value="SyphonServer">Syphon Server</option>
                </select>
              </label>
            </div>
            <div class="split">
              <label>
                Width
                <input type="number" min="1" value={props.width} onInput={(event) => props.onWidth(Number(event.currentTarget.value))} />
              </label>
              <label>
                Height
                <input type="number" min="1" value={props.height} onInput={(event) => props.onHeight(Number(event.currentTarget.value))} />
              </label>
            </div>
            <label>
              Output Fade ms
              <input
                type="number"
                min="0"
                step="10"
                value={props.fadeMs}
                onInput={(event) => props.onFadeMs(Number(event.currentTarget.value))}
              />
            </label>
            <Show when={props.kind === "Display"}>
              <div class="split">
                <label>
                  Monitor index
                  <input type="number" min="0" value={props.monitorId} onInput={(event) => props.onMonitorId(Number(event.currentTarget.value))} />
                </label>
                <label class="checkbox inlineCheckbox">
                  <input type="checkbox" checked={props.fullscreen} onChange={(event) => props.onFullscreen(event.currentTarget.checked)} />
                  Fullscreen
                </label>
              </div>
            </Show>
            <Show when={props.kind !== "Display"}>
              <label>
                Endpoint
                <input value={props.endpoint} onInput={(event) => props.onEndpoint(event.currentTarget.value)} />
              </label>
            </Show>
          </div>
        </details>
      </div>
    </section>
  );
}
