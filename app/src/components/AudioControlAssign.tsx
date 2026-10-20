import { For, Show, createSignal } from "solid-js";
import { audioSources, type AudioAssignmentSettings, type AudioMappingSource } from "../audioControlMapping";

export function AudioControlAssign(props: {
  trigger: boolean; low?: number; high?: number; onAssign: (settings: AudioAssignmentSettings) => void;
}) {
  const [expanded, setExpanded] = createSignal(false);
  const [source, setSource] = createSignal<AudioMappingSource>("Level");
  const [mode, setMode] = createSignal<"Threshold" | "Onset">("Threshold");
  const [gain, setGain] = createSignal(1);
  const [attack, setAttack] = createSignal(50);
  const [release, setRelease] = createSignal(250);
  const [threshold, setThreshold] = createSignal(0.5);
  const [low, setLow] = createSignal(props.low ?? 0);
  const [high, setHigh] = createSignal(props.high ?? 1);
  const valid = () => [gain(), attack(), release(), threshold(), low(), high()].every(Number.isFinite)
    && gain() >= 0 && gain() <= 16 && attack() >= 0 && attack() <= 10000
    && release() >= 0 && release() <= 10000 && threshold() >= 0 && threshold() <= 1
    && low() >= 0 && low() <= 1 && high() >= 0 && high() <= 1;
  return <div class="controlAssignMenuGroup">
    <button type="button" role="menuitem" data-control-assign-audio aria-expanded={expanded()}
      onClick={() => setExpanded(value => !value)}>Audio</button>
    <Show when={expanded()}>
      <div class="controlAssignAudioFields" role="group" aria-label="Audio mapping">
        <Show when={props.trigger}>
          <label>Trigger mode<select aria-label="Trigger mode" value={mode()} onChange={event => setMode(event.currentTarget.value as "Threshold" | "Onset")}>
            <option value="Threshold">Threshold crossing</option><option value="Onset">Audio onset</option>
          </select></label>
        </Show>
        <Show when={!props.trigger || mode() === "Threshold"}>
          <label>Audio source<select aria-label="Audio source" value={source()} onChange={event => setSource(event.currentTarget.value as AudioMappingSource)}>
            <For each={audioSources}>{item => <option value={item.value}>{item.label}</option>}</For>
          </select></label>
          <label>Audio gain<input type="number" min="0" max="16" step="0.1" value={gain()} onInput={event => setGain(event.currentTarget.valueAsNumber)} /></label>
          <div class="controlAssignAudioPair">
            <label>Attack (ms)<input type="number" min="0" max="10000" step="10" value={attack()} onInput={event => setAttack(event.currentTarget.valueAsNumber)} /></label>
            <label>Release (ms)<input type="number" min="0" max="10000" step="10" value={release()} onInput={event => setRelease(event.currentTarget.valueAsNumber)} /></label>
          </div>
        </Show>
        <Show when={props.trigger && mode() === "Threshold"}>
          <label>Trigger threshold<input type="number" min="0" max="1" step="0.01" value={threshold()} onInput={event => setThreshold(event.currentTarget.valueAsNumber)} /></label>
        </Show>
        <Show when={!props.trigger}>
          <div class="controlAssignAudioPair">
            <label>Output minimum<input type="number" min="0" max="1" step="0.01" value={low()} onInput={event => setLow(event.currentTarget.valueAsNumber)} /></label>
            <label>Output maximum<input type="number" min="0" max="1" step="0.01" value={high()} onInput={event => setHigh(event.currentTarget.valueAsNumber)} /></label>
          </div>
        </Show>
        <p class="controlAssignMenuNote">Audio follows the live input in Setup. Live values do not create Undo entries.</p>
        <button type="button" data-control-assign-audio-save disabled={!valid()} onClick={() => props.onAssign({
          source: props.trigger && mode() === "Onset" ? "Onset" : source(), gain: gain(), attack_ms: attack(),
          release_ms: release(), threshold: threshold(), mode: props.trigger ? mode() : "Continuous", low: low(), high: high(),
        })}>Assign audio</button>
      </div>
    </Show>
  </div>;
}
