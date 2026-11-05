import { For, Show, createEffect, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import { parseVideoSyphonServers, syphonServerAllowsAdd, syphonServerEndpoint, syphonServerLabel, type VideoSyphonServer } from "../videoSyphonSources";
import "./VideoSyphonSourcePicker.css";

interface VideoSyphonSourcePickerProps {
  path: string;
  onSetPath: (path: string) => void;
  invokeCommand?: FrontendTauriInvoke;
  onAdmissionChange?: (allowed: boolean) => void;
}

export function VideoSyphonSourcePicker(props: VideoSyphonSourcePickerProps) {
  const [servers, setServers] = createSignal<VideoSyphonServer[] | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let generation = 0;
  const allowed = createMemo(() => syphonServerAllowsAdd(props.path, servers(), busy()));
  createEffect(() => props.onAdmissionChange?.(allowed()));
  onCleanup(() => { generation += 1; });
  const refresh = async () => {
    if (busy()) return;
    const request = ++generation;
    setBusy(true);
    setServers(null);
    setError(null);
    props.onSetPath("");
    try {
      if (!props.invokeCommand) throw new Error("Syphon discovery is unavailable until the native IPC bridge is connected.");
      const value = await props.invokeCommand<unknown>("discover_syphon_sources");
      if (request === generation) setServers(parseVideoSyphonServers(value));
    } catch (reason) {
      if (request === generation) setError(String(reason instanceof Error ? reason.message : reason).slice(0, 2048));
    } finally {
      if (request === generation) setBusy(false);
    }
  };
  onMount(() => { void refresh(); });
  return (
    <section class="videoSourceSyphonPanel" data-video-syphon-source>
      <button type="button" data-video-syphon-refresh disabled={busy()} onClick={() => { void refresh(); }}>
        {busy() ? "Refreshing Syphon servers…" : "Refresh Syphon servers"}
      </button>
      <label>
        Syphon server
        <select data-video-syphon-select value={props.path} disabled={busy() || !servers()?.length}
          onChange={event => props.onSetPath(syphonServerAllowsAdd(event.currentTarget.value, servers(), busy()) ? event.currentTarget.value : "")}>
          <option value="">Select a Syphon server</option>
          <For each={servers() ?? []}>{server => <option data-no-localize value={syphonServerEndpoint(server)}>{syphonServerLabel(server)}</option>}</For>
        </select>
      </label>
      <p class="fieldHint" data-syphon-discovery-state={busy() ? "loading" : error() ? "error" : servers() ? "available" : "idle"} role="status" aria-live="polite">
        {busy() ? "Searching for Syphon servers…" : error() ? "Syphon server discovery failed. Refresh to try again."
          : servers()?.length ? "Select a Syphon server. Receiving starts after the layer is added."
          : "No Syphon servers found. Start Simple Server or another sender on this Mac, then refresh."}
      </p>
      <Show when={error()}><details class="fieldHint"><summary>Syphon diagnostic details</summary><p>{error()}</p></details></Show>
    </section>
  );
}
