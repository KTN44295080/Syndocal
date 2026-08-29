import { For, onMount, Show, createMemo, createSignal } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import { displayAddErrorMessage, loadUiLocale } from "../uiLocalization";
import type { VideoOutputKind } from "../types";

type VideoOutputAddStatus = {
  kind: "pending" | "success" | "error";
  message: string;
};

export interface VideoDisplayMonitorDescriptor {
  index: number;
  identity: string;
  name: string;
  physicalWidth: number;
  physicalHeight: number;
  positionX: number;
  positionY: number;
  scaleFactor: number;
  isEditorMonitor: boolean;
}

export interface VideoDisplayTargetSummary {
  label: string;
  displayName: string;
  width: number;
  height: number;
  fullscreen: true;
}

const displayNumberFromNativeName = (name: string): number | undefined => {
  const match = name.trim().match(/^(?:\\\\\\\.\\)?DISPLAY([1-9]\d*)$/i);
  if (!match) return undefined;
  const displayNumber = Number(match[1]);
  return Number.isSafeInteger(displayNumber) ? displayNumber : undefined;
};

export const videoDisplayTargetSummary = (
  monitor: VideoDisplayMonitorDescriptor,
): VideoDisplayTargetSummary => ({
  // Windows exposes names such as `\\.\\DISPLAY2`. Keep that human-readable
  // name in the selector, but strip only the exact device-path syntax from the
  // wire label. The array index is only a fallback for non-Windows names.
  label: `Display ${displayNumberFromNativeName(monitor.name) ?? monitor.index + 1}`,
  displayName: monitor.name.trim() || `Display ${monitor.index + 1}`,
  width: monitor.physicalWidth,
  height: monitor.physicalHeight,
  fullscreen: true,
});

const isValidVideoDisplayMonitor = (value: unknown): value is VideoDisplayMonitorDescriptor => {
  if (!value || typeof value !== "object") return false;
  const monitor = value as Partial<VideoDisplayMonitorDescriptor>;
  return typeof monitor.index === "number" && Number.isInteger(monitor.index) && monitor.index >= 0 && monitor.index <= 255
    && typeof monitor.identity === "string" && /^[0-9a-f]{64}$/.test(monitor.identity)
    && typeof monitor.name === "string"
    && typeof monitor.physicalWidth === "number" && Number.isInteger(monitor.physicalWidth) && monitor.physicalWidth > 0
    && typeof monitor.physicalHeight === "number" && Number.isInteger(monitor.physicalHeight) && monitor.physicalHeight > 0
    && typeof monitor.positionX === "number" && Number.isInteger(monitor.positionX)
    && typeof monitor.positionY === "number" && Number.isInteger(monitor.positionY)
    && typeof monitor.scaleFactor === "number" && Number.isFinite(monitor.scaleFactor) && monitor.scaleFactor > 0
    && typeof monitor.isEditorMonitor === "boolean";
};

export const normalizeVideoDisplayMonitors = (value: unknown): VideoDisplayMonitorDescriptor[] => {
  if (!Array.isArray(value)) return [];
  const validMonitors = value.filter(isValidVideoDisplayMonitor);
  if (validMonitors.length !== value.length
    || new Set(validMonitors.map((monitor) => monitor.index)).size !== validMonitors.length
    || new Set(validMonitors.map((monitor) => monitor.identity)).size !== validMonitors.length) {
    return [];
  }
  if (validMonitors.filter((monitor) => monitor.isEditorMonitor).length !== 1) {
    return [];
  }
  return validMonitors;
};

export const initialVideoDisplayMonitor = (
  monitors: readonly VideoDisplayMonitorDescriptor[],
): VideoDisplayMonitorDescriptor | undefined =>
  monitors.find((monitor) => !monitor.isEditorMonitor) ?? monitors[0];

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
  onAddDisplayOutput: (monitor: VideoDisplayMonitorDescriptor) => Promise<void>;
  onEnableShowSpoutOutputs: () => void | Promise<unknown>;
};

export function VideoOutputCreatePanel(props: VideoOutputCreatePanelProps) {
  const [monitors, setMonitors] = createSignal<VideoDisplayMonitorDescriptor[]>([]);
  const [selectedMonitorIdentity, setSelectedMonitorIdentity] = createSignal<string | null>(null);
  const [monitorDiscovery, setMonitorDiscovery] = createSignal<"idle" | "loading" | "ready" | "unavailable">("idle");
  const [addPending, setAddPending] = createSignal(false);
  const [addStatus, setAddStatus] = createSignal<VideoOutputAddStatus | null>(null);
  const selectedMonitor = createMemo(() => {
    const identity = selectedMonitorIdentity();
    return identity ? monitors().find((monitor) => monitor.identity === identity) : undefined;
  });

  const isTauriRuntime = () =>
    typeof window !== "undefined" && Boolean((window as Window & { __TAURI_INTERNALS__?: unknown }).__TAURI_INTERNALS__);

  const monitorLabel = (monitor: VideoDisplayMonitorDescriptor) => {
    const target = videoDisplayTargetSummary(monitor);
    return `${target.displayName} · ${target.width}x${target.height}`;
  };

  const selectMonitor = (identity: string) => {
    const monitor = monitors().find((candidate) => candidate.identity === identity);
    if (!monitor) return;
    setSelectedMonitorIdentity(monitor.identity);
    if (!addPending()) setAddStatus(null);
    props.onMonitorId(monitor.index);
    const target = videoDisplayTargetSummary(monitor);
    props.onWidth(target.width);
    props.onHeight(target.height);
    props.onFullscreen(target.fullscreen);
    props.onLabel(target.label);
  };

  const addSelectedDisplayOutput = async () => {
    if (addPending()) return;
    const monitor = selectedMonitor();
    if (!monitor) return;
    setAddPending(true);
    setAddStatus({ kind: "pending", message: "Adding display output…" });
    try {
      await props.onAddDisplayOutput(monitor);
      setAddStatus({ kind: "success", message: "Display output added." });
    } catch (error) {
      setAddStatus({ kind: "error", message: displayAddErrorMessage(error, loadUiLocale()) });
    } finally {
      setAddPending(false);
    }
  };

  onMount(() => {
    if (!isTauriRuntime()) {
      setMonitorDiscovery("unavailable");
      return;
    }
    setMonitorDiscovery("loading");
    void props.invokeCommand<VideoDisplayMonitorDescriptor[]>("list_video_display_monitors")
      .then((nextMonitors) => {
        const validMonitors = normalizeVideoDisplayMonitors(nextMonitors);
        const hasValidTargets = Array.isArray(nextMonitors)
          && validMonitors.length === nextMonitors.length
          && validMonitors.length > 0;
        setMonitors(hasValidTargets ? validMonitors : []);
        setMonitorDiscovery(hasValidTargets ? "ready" : "unavailable");
        // Selection follows detected order. The editor surface is the
        // explicit fallback when no separate target is available.
        const initial = hasValidTargets ? initialVideoDisplayMonitor(validMonitors) : undefined;
        if (initial) selectMonitor(initial.identity);
      })
      .catch(() => setMonitorDiscovery("unavailable"));
  });

  return (
    <section class="videoSetupDisclosure videoOutputCreateSurface" aria-label="New Output">
      <div class="videoSetupDisclosureHeading">New Output</div>
      <div class="videoOutputForm">
        <div class="videoOutputQuickCreate" data-show-spout-output-activation>
          <p class="hint videoOutputQuickHint">
            Same-PC show video uses exactly <strong>Syndocal Background</strong> and <strong>Syndocal Foreground</strong> Spout senders at 1920×1080. Stopped content keeps both senders alive with opaque RGB-black frames.
          </p>
          <button
            class="primary"
            data-video-output-enable-show-spout
            onClick={() => void props.onEnableShowSpoutOutputs()}
          >Confirm and enable show Spout outputs</button>
        </div>
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
                value={selectedMonitorIdentity() ?? ""}
                disabled={monitorDiscovery() !== "ready" || monitors().length === 0 || addPending()}
                onChange={(event) => selectMonitor(event.currentTarget.value)}
              >
                <Show when={monitors().length > 0} fallback={<option value="" disabled>Screen detection unavailable</option>}>
                  <For each={monitors()}>{(monitor) => (
                    <option value={monitor.identity} data-editor-monitor={monitor.isEditorMonitor ? "true" : undefined}>
                      {monitorLabel(monitor)}{monitor.isEditorMonitor ? " · Editor screen" : ""}
                    </option>
                  )}</For>
                </Show>
              </select>
            </label>
            <Show when={selectedMonitor()?.isEditorMonitor}>
              <p class="inlineWarning videoOutputQuickHint" role="alert" data-video-output-editor-warning>
                This editor screen overlaps the Syndocal control surface. Choose a different screen when possible.
              </p>
            </Show>
            <button
              class="primary"
              data-video-output-add
              disabled={monitorDiscovery() !== "ready" || !selectedMonitor() || addPending()}
              onClick={() => { void addSelectedDisplayOutput(); }}
            >
              Add display output
            </button>
            <Show when={addStatus()}>{(status) => (
              <p
                class={status().kind === "error" ? "inlineWarning videoOutputQuickHint" : "hint videoOutputQuickHint"}
                role={status().kind === "error" ? "alert" : "status"}
                data-video-output-add-status={status().kind}
              >
                {status().message}
              </p>
            )}</Show>
            <p class="hint videoOutputQuickHint" data-video-output-native-dialog-note>
              The native display confirmation dialog will appear when this output is added.
            </p>
            <p class="hint videoOutputQuickHint">
              {(() => {
                const monitor = selectedMonitor();
                if (!monitor) return "Screen detection unavailable";
                const target = videoDisplayTargetSummary(monitor);
                return `${target.displayName} · ${target.width}x${target.height} · fullscreen`;
              })()}
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
