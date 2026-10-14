import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { LtcInputDeviceSummary, LtcInputStatus } from "../types";

const LTC_INPUT_POLL_MS = 1_000;

interface LtcInputPanelProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
}

/** The Timeline chases SMPTE LTC arriving on one channel of an audio input. */
export function LtcInputPanel(props: LtcInputPanelProps) {
  const [status, setStatus] = createSignal<LtcInputStatus | null>(null);
  const [devices, setDevices] = createSignal<LtcInputDeviceSummary[]>([]);
  const [deviceName, setDeviceName] = createSignal<string>("");
  const [channelIndex, setChannelIndex] = createSignal(0);
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let disposed = false;

  const selectedDevice = () => devices().find((device) => device.name === deviceName())
    ?? devices().find((device) => device.is_default);
  const channelCount = () => selectedDevice()?.channels ?? 2;
  // A device with fewer channels never keeps a channel it does not have.
  const selectedChannel = () => Math.min(channelIndex(), Math.max(1, channelCount()) - 1);

  const refreshStatus = async () => {
    try {
      const next = await props.invokeCommand<LtcInputStatus>("ltc_input_status_v1");
      if (!disposed) setStatus(next);
    } catch { /* Background status polling is best-effort. */ }
  };

  const refreshDevices = async () => {
    setError(null);
    try {
      const next = await props.invokeCommand<LtcInputDeviceSummary[]>("list_ltc_input_devices_v1");
      if (!disposed) setDevices(next);
    } catch (failure) {
      if (!disposed) setError(String(failure));
    }
  };

  const run = async (command: "start_ltc_input_v1" | "stop_ltc_input_v1", args?: Record<string, unknown>) => {
    setBusy(true);
    setError(null);
    try {
      const next = await props.invokeCommand<LtcInputStatus>(command, args);
      if (!disposed) setStatus(next);
    } catch (failure) {
      if (!disposed) setError(String(failure));
    } finally {
      if (!disposed) setBusy(false);
    }
  };

  onMount(() => {
    if (!props.backendAvailable) return;
    void refreshDevices();
    void refreshStatus();
    const timer = window.setInterval(() => void refreshStatus(), LTC_INPUT_POLL_MS);
    onCleanup(() => {
      disposed = true;
      window.clearInterval(timer);
    });
  });

  const running = () => status()?.running === true;
  const stateText = () => {
    const current = status();
    if (!current?.running) return current?.last_error ? `Stopped · ${current.last_error}` : "Stopped";
    if (current.signal) return "Chasing LTC";
    return current.last_error ?? "Listening · no LTC signal";
  };

  return (
    <section class="ioConnectionDesk ltcInputPanel" data-ltc-input>
      <header class="ioDeskHeader">
        <h3 class="textBalance">LTC input</h3>
        <span class={`ioConnectionState ${running() && status()?.signal ? "ok" : "idle"}`}>
          <i aria-hidden="true" />{running() ? (status()?.signal ? "Signal" : "No signal") : "Stopped"}
        </span>
      </header>
      <p class="hint">The Timeline follows SMPTE timecode (LTC) on one channel of an audio input.</p>
      <div class="ltcInputControls">
        <label>
          Audio input
          <select
            data-io-control="ltc-input-device"
            disabled={busy() || running()}
            value={running() ? status()?.device_name ?? deviceName() : deviceName()}
            onInput={(event) => setDeviceName(event.currentTarget.value)}
          >
            <option value="">System default</option>
            <For each={devices()}>{(device) => <option data-no-localize value={device.name}>{device.name}</option>}</For>
          </select>
        </label>
        <label>
          Channel
          <select
            data-io-control="ltc-input-channel"
            disabled={busy() || running()}
            value={running() ? status()?.channel_index ?? selectedChannel() : selectedChannel()}
            onInput={(event) => setChannelIndex(Number(event.currentTarget.value))}
          >
            <For each={Array.from({ length: Math.max(1, channelCount()) }, (_, index) => index)}>
              {(index) => <option value={index}>{index + 1}</option>}
            </For>
          </select>
        </label>
        <button data-io-control="ltc-input-refresh" disabled={busy() || running()} onClick={() => void refreshDevices()}>Refresh</button>
        <Show when={running()} fallback={
          <button
            data-io-control="ltc-input-start"
            class="primary"
            disabled={!props.backendAvailable || busy()}
            onClick={() => void run("start_ltc_input_v1", {
              deviceName: deviceName() === "" ? null : deviceName(),
              channelIndex: selectedChannel(),
            })}
          >Start LTC</button>
        }>
          <button data-io-control="ltc-input-stop" disabled={busy()} onClick={() => void run("stop_ltc_input_v1")}>Stop LTC</button>
        </Show>
      </div>
      <p class="ltcInputState" role="status" aria-live="polite">
        <span>{stateText()}</span>
        <Show when={running() && status()?.timecode}>
          {" · "}<span class="tabularNums" data-no-localize>{status()?.timecode}</span>
        </Show>
        <Show when={running() && status()?.frame_rate}>
          {" · "}<span data-no-localize>{status()?.frame_rate} fps</span>
        </Show>
      </p>
      <Show when={error()}>
        <p class="fieldError textPretty" role="alert">{error()}</p>
      </Show>
    </section>
  );
}
