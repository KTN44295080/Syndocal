import { createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type { FrontendTauriInvoke } from "../tauriInvokeCommands";
import type { MidiOutputSummary, MtcOutputFrameRate, MtcOutputStatus } from "../types";

const MTC_OUTPUT_POLL_MS = 1_000;

const FRAME_RATES: { value: MtcOutputFrameRate; label: string }[] = [
  { value: "fps24", label: "24 fps" },
  { value: "fps25", label: "25 fps" },
  { value: "fps2997_drop", label: "29.97 fps drop-frame" },
  { value: "fps30", label: "30 fps" },
];

interface MtcOutputSectionProps {
  backendAvailable: boolean;
  invokeCommand: FrontendTauriInvoke;
  midiOutputs: MidiOutputSummary[];
}

/** Sends MIDI Time Code that follows the Timeline playhead to one MIDI output. */
export function MtcOutputSection(props: MtcOutputSectionProps) {
  const [status, setStatus] = createSignal<MtcOutputStatus | null>(null);
  const [outputIndex, setOutputIndex] = createSignal<number | null>(null);
  const [frameRate, setFrameRate] = createSignal<MtcOutputFrameRate>("fps30");
  const [busy, setBusy] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  let disposed = false;

  const refresh = async () => {
    try {
      const next = await props.invokeCommand<MtcOutputStatus>("mtc_output_status_v1");
      if (!disposed) setStatus(next);
    } catch { /* Background status polling is best-effort. */ }
  };

  const start = async () => {
    const index = outputIndex();
    const output = props.midiOutputs.find((candidate) => candidate.index === index);
    if (!output) return;
    setBusy(true);
    setError(null);
    try {
      const next = await props.invokeCommand<MtcOutputStatus>("start_mtc_output_v1", {
        outputIndex: output.index,
        expectedOutputName: output.name,
        frameRate: frameRate(),
      });
      if (!disposed) setStatus(next);
    } catch (failure) {
      if (!disposed) setError(String(failure));
    } finally {
      if (!disposed) setBusy(false);
    }
  };

  const stop = async () => {
    setBusy(true);
    setError(null);
    try {
      const next = await props.invokeCommand<MtcOutputStatus>("stop_mtc_output_v1");
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
    const timer = window.setInterval(() => void refresh(), MTC_OUTPUT_POLL_MS);
    onCleanup(() => {
      disposed = true;
      window.clearInterval(timer);
    });
  });

  const running = () => status()?.running === true;
  const stateText = () => {
    const current = status();
    if (!current?.running) return current?.last_error ? `Stopped · ${current.last_error}` : "Stopped";
    if (current.held_reason) return current.held_reason;
    return current.transmitting ? "Sending" : "Ready · sends while the Timeline plays";
  };

  return (
    <details class="ioDisclosure mtcOutputSection" data-io-disclosure="mtc-output">
      <summary>MTC output (optional)</summary>
      <div class="ioDisclosureBody">
        <p class="hint">Sends MIDI Time Code that follows the Timeline to another device. Only a PC whose output is armed sends.</p>
        <div class="mtcOutputControls">
          <label>
            MIDI output
            <select
              data-io-control="mtc-output"
              disabled={busy() || running()}
              value={outputIndex() ?? ""}
              onInput={(event) => setOutputIndex(event.currentTarget.value === "" ? null : Number(event.currentTarget.value))}
            >
              <option value="">Select output</option>
              <For each={props.midiOutputs}>{(output) => <option data-no-localize value={output.index}>{output.name}</option>}</For>
            </select>
          </label>
          <label>
            Frame rate
            <select
              data-io-control="mtc-frame-rate"
              disabled={busy() || running()}
              value={frameRate()}
              onInput={(event) => setFrameRate(event.currentTarget.value as MtcOutputFrameRate)}
            >
              <For each={FRAME_RATES}>{(rate) => <option value={rate.value}>{rate.label}</option>}</For>
            </select>
          </label>
          <Show when={running()} fallback={
            <button
              data-io-control="mtc-output-start"
              class="primary"
              disabled={!props.backendAvailable || busy() || outputIndex() === null}
              onClick={() => void start()}
            >Start MTC</button>
          }>
            <button data-io-control="mtc-output-stop" disabled={busy()} onClick={() => void stop()}>Stop MTC</button>
          </Show>
        </div>
        <p class="mtcOutputState" role="status" aria-live="polite">
          <span>{stateText()}</span>
          <Show when={running() && status()?.timecode}>
            {" · "}<span class="tabularNums" data-no-localize>{status()?.timecode}</span>
          </Show>
          <Show when={running() && status()?.port_name}>
            {" · "}<span data-no-localize>{status()?.port_name}</span>
          </Show>
        </p>
        <Show when={error()}>
          <p class="fieldError textPretty" role="alert">{error()}</p>
        </Show>
      </div>
    </details>
  );
}
