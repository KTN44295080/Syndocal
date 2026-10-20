import { For, Show, createSignal, onCleanup, onMount } from "solid-js";
import type { VideoOutputConfigDraft } from "../editorDrafts";
import type { VideoOutputKind, VideoOutputSummary } from "../types";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import { normalizeVideoDisplayMonitors, type VideoDisplayMonitorDescriptor } from "./VideoOutputCreatePanel";
import { videoOutputAuthoringErrorMessage } from "../videoOutputAuthoring";

type MaybePromise = void | Promise<unknown>;
type VideoOutputConfigPanelProps = {
  output: VideoOutputSummary;
  draft: VideoOutputConfigDraft;
  syphonAvailable?: boolean;
  spoutAvailable?: boolean;
  ndiAvailable?: boolean;
  invokeCommand?: FrontendTauriInvoke;
  onDraft: (output: VideoOutputSummary, patch: Partial<VideoOutputConfigDraft>) => void;
  onApply: (output: VideoOutputSummary, monitor?: VideoDisplayMonitorDescriptor) => MaybePromise;
  onRemove?: (outputId: number) => MaybePromise;
};

export function VideoOutputConfigPanel(props: VideoOutputConfigPanelProps) {
  const [monitors, setMonitors] = createSignal<VideoDisplayMonitorDescriptor[]>([]);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [status, setStatus] = createSignal<string | null>(null);
  const selectedMonitor = () => monitors().find((monitor) => monitor.index === props.draft.monitor_id);
  const available = () => props.draft.kind === "Display" ? Boolean(selectedMonitor())
    : props.draft.kind === "SpoutSender" ? props.spoutAvailable
    : props.draft.kind === "NdiSender" ? props.ndiAvailable : props.syphonAvailable;
  let monitorQueryEpoch = 0;
  onCleanup(() => { monitorQueryEpoch += 1; });
  const refreshMonitors = async () => {
    if (!props.invokeCommand) return;
    const epoch = ++monitorQueryEpoch;
    try {
      const discovered = normalizeVideoDisplayMonitors(await props.invokeCommand("list_video_display_monitors"));
      if (epoch === monitorQueryEpoch) setMonitors(discovered);
    } catch {
      if (epoch === monitorQueryEpoch) setMonitors([]);
    }
  };
  onMount(() => { void refreshMonitors(); });
  const apply = async () => {
    if (busy() || !available()) return;
    setBusy(true); setError(null); setStatus(null);
    const outputId = props.output.id;
    try {
      const currentProject = await props.onApply(props.output, selectedMonitor());
      if (currentProject !== false && props.output.id === outputId) setStatus("Video output updated.");
    } catch (cause) {
      if (props.output.id === outputId) setError(videoOutputAuthoringErrorMessage(cause));
    } finally { setBusy(false); }
  };
  const remove = async () => {
    if (busy() || !props.onRemove) return;
    setBusy(true); setError(null); setStatus(null);
    const outputId = props.output.id;
    try { await props.onRemove(outputId); }
    catch (cause) { if (props.output.id === outputId) setError(videoOutputAuthoringErrorMessage(cause)); }
    finally { setBusy(false); }
  };
  return (
    <div class="videoOutputConfig" data-video-output-config>
      <h3>Video Output Config</h3>
      <div class="split">
        <label>Label<input value={props.draft.label} disabled={busy()}
          onInput={(event) => props.onDraft(props.output, { label: event.currentTarget.value })} /></label>
        <label>Kind<select value={props.draft.kind} disabled={busy()}
          onInput={(event) => props.onDraft(props.output, { kind: event.currentTarget.value as VideoOutputKind })}>
          <option value="Display">Display</option>
          <Show when={props.ndiAvailable} fallback={<Show when={props.draft.kind === "NdiSender"}><option value="NdiSender" disabled>NDI Sender (unavailable)</option></Show>}>
            <option value="NdiSender">NDI Sender</option>
          </Show>
          <Show when={props.spoutAvailable} fallback={<Show when={props.draft.kind === "SpoutSender"}><option value="SpoutSender" disabled>Spout Sender (Windows only)</option></Show>}>
            <option value="SpoutSender">Spout Sender</option>
          </Show>
          <Show when={props.syphonAvailable} fallback={<Show when={props.draft.kind === "SyphonServer"}><option value="SyphonServer" disabled>Syphon Server (macOS only)</option></Show>}>
            <option value="SyphonServer">Syphon Server</option>
          </Show>
        </select></label>
      </div>
      <div class="split">
        <label>Width<input type="number" min="1" max={props.draft.kind === "Display" ? 16384 : 8192}
          value={props.draft.width} disabled={busy()} onInput={(event) => props.onDraft(props.output, { width: Number(event.currentTarget.value) })} /></label>
        <label>Height<input type="number" min="1" max={props.draft.kind === "Display" ? 16384 : 8192}
          value={props.draft.height} disabled={busy()} onInput={(event) => props.onDraft(props.output, { height: Number(event.currentTarget.value) })} /></label>
      </div>
      <Show when={props.draft.kind === "Display"}>
        <div class="split">
          <label>Screen<select data-video-output-config-monitor value={selectedMonitor()?.index ?? ""} disabled={busy()}
            onChange={(event) => props.onDraft(props.output, { monitor_id: Number(event.currentTarget.value) })}>
            <Show when={!selectedMonitor()}><option value="" disabled>Select a connected screen</option></Show>
            <For each={monitors()}>{(monitor) => <option value={monitor.index} data-no-localize>{monitor.name} · {monitor.physicalWidth}×{monitor.physicalHeight}</option>}</For>
          </select></label>
          <label class="checkbox inlineCheckbox"><input type="checkbox" checked={props.draft.fullscreen} disabled={busy()}
            onChange={(event) => props.onDraft(props.output, { fullscreen: event.currentTarget.checked })} />Fullscreen</label>
        </div>
        <button type="button" disabled={busy()} onClick={() => void refreshMonitors()}>Refresh display list</button>
        <Show when={selectedMonitor()?.isEditorMonitor}><p class="inlineWarning" role="status">This editor screen overlaps the Syndocal control surface. Choose a different screen when possible.</p></Show>
      </Show>
      <Show when={props.draft.kind === "SyphonServer"}>
        <p class={props.syphonAvailable ? "hint" : "inlineWarning"} role="status" data-syphon-output-help>
          {props.syphonAvailable
            ? "Publishes video to Syphon clients on this Mac and follows the current output state."
            : "Syphon output is available on macOS only. This saved output is preserved; choose another type to use it on this computer."}
        </p>
      </Show>
      <Show when={props.draft.kind !== "Display"}>
        <label>{props.draft.kind === "SyphonServer" ? "Server name" : "Endpoint"}<input value={props.draft.endpoint_name} disabled={busy()}
          onInput={(event) => props.onDraft(props.output, { endpoint_name: event.currentTarget.value })} /></label>
      </Show>
      <Show when={!available()}><p class="inlineWarning" role="status">Choose an available output type and target before applying.</p></Show>
      <div class="buttonRow">
        <button class="primary" data-video-output-apply disabled={busy() || !available()} onClick={() => void apply()}>
          {busy() ? "Applying output change…" : "Apply Output"}
        </button>
        <Show when={props.onRemove}><button type="button" data-video-output-remove disabled={busy()} onClick={() => void remove()}>Remove output</button></Show>
      </div>
      <Show when={props.onRemove}><p class="hint">Removing an output can be undone.</p></Show>
      <Show when={error()}>{(message) => <p class="inlineWarning" role="alert" data-video-output-config-error>{message()}</p>}</Show>
      <Show when={status()}>{(message) => <p class="hint" role="status">{message()}</p>}</Show>
    </div>
  );
}
