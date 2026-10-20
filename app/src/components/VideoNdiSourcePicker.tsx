import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import "./VideoNdiSourcePicker.css";
import { ndiDiscoveryStateMessage, ndiSourceAllowsAdd, parseVideoNdiDiscovery, type VideoNdiDiscovery } from "../videoNdiSources";

export interface VideoNdiSourcePickerProps {
  path: string;
  onSetPath: (path: string) => void;
  invokeCommand?: FrontendTauriInvoke;
  onAdmissionChange?: (allowed: boolean) => void;
}

export function VideoNdiSourcePicker(props: VideoNdiSourcePickerProps) {
  const [discovery, setDiscovery] = createSignal<VideoNdiDiscovery | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [bridgeUnavailable, setBridgeUnavailable] = createSignal(false);
  let generation = 0;
  const allowed = createMemo(() => ndiSourceAllowsAdd(props.path, discovery(), busy()));
  createEffect(() => props.onAdmissionChange?.(allowed()));
  onCleanup(() => { generation += 1; });

  const refresh = async () => {
    if (busy()) return;
    const request = ++generation;
    setBusy(true);
    setDiscovery(null);
    setBridgeUnavailable(false);
    props.onSetPath("");
    const invokeCommand = props.invokeCommand;
    if (!invokeCommand) {
      setBridgeUnavailable(true);
      setBusy(false);
      return;
    }
    try {
      const value = await invokeCommand<unknown>("list_ndi_sources_v1");
      if (request !== generation) return;
      setDiscovery(parseVideoNdiDiscovery(value));
    } catch {
      if (request === generation) setDiscovery({ state: "error", sources: [], message: null });
    } finally {
      if (request === generation) setBusy(false);
    }
  };

  const selectSource = (endpoint: string) => {
    props.onSetPath(ndiSourceAllowsAdd(endpoint, discovery(), busy()) ? endpoint : "");
  };
  onMount(() => { void refresh(); });

  return (
    <section class="videoSourceCameraPanel videoSourceNdiPanel" data-video-ndi-source>
      <div class="videoSourceCameraActions">
        <button type="button" data-video-ndi-refresh disabled={busy()} onClick={() => { void refresh(); }}>
          {busy() ? "Refreshing NDI sources…" : "Refresh NDI sources"}
        </button>
      </div>
      <label>
        NDI source
        <select
          data-video-ndi-select
          value={props.path}
          disabled={busy() || discovery()?.state !== "available" || !discovery()?.sources.length}
          onChange={(event) => selectSource(event.currentTarget.value)}
        >
          <option value="">Select an NDI source</option>
          <For each={discovery()?.sources ?? []}>
            {(source) => <option value={source.endpoint_name} data-no-localize>{source.name}</option>}
          </For>
        </select>
      </label>
      <p class="fieldHint" data-ndi-discovery-state={busy() ? "loading" : bridgeUnavailable() ? "unavailable" : discovery()?.state ?? "idle"} role="status" aria-live="polite">
        {busy() ? "Searching for NDI sources…" : bridgeUnavailable()
          ? "NDI discovery is unavailable until the native IPC bridge is connected."
          : ndiDiscoveryStateMessage(discovery())}
      </p>
      <Show when={discovery()?.message && discovery()?.state !== "available"}>
        <details class="fieldHint">
          <summary>NDI diagnostic details</summary>
          <p data-no-localize>{discovery()?.message}</p>
        </details>
      </Show>
    </section>
  );
}
