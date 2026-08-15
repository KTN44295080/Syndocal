import { For, Show } from "solid-js";
import { clockSourceLabel, clockSyncStatusLabel, isExternalClockSource } from "../clockDisplay";
import type { EngineSnapshot, MidiInputSummary, SubmasterSummary } from "../types";
import { BlackoutReleaseControl } from "./BlackoutReleaseControl";

interface LightingRuntimeControlsPanelProps {
  lightingMaster: number;
  submasters: SubmasterSummary[];
  clock: EngineSnapshot["clock"];
  bpmDraft: string;
  midiInputs: MidiInputSummary[];
  selectedMidiInput: number | null;
  midiConnected: boolean;
  onLightingMaster: (level: number) => void | Promise<void>;
  onSubmaster: (groupId: string, level: number) => void | Promise<void>;
  onBlackout: (enabled: boolean) => void | Promise<void>;
  onAllBlackout: (enabled: boolean) => void | Promise<void>;
  onBpmDraft: (value: string) => void;
  onApplyBpm: () => void | Promise<void>;
  onTapBpm: () => void | Promise<void>;
  onRefreshMidi: () => void | Promise<void>;
  onDisconnectMidiClock: () => void | Promise<void>;
  onSelectedMidiInput: (index: number) => void;
  onConnectMidiClock: () => void | Promise<void>;
}

export function LightingRuntimeControlsPanel(props: LightingRuntimeControlsPanelProps) {
  return (
    <section class="lightingRuntimeDesk">
      <header class="ioDeskHeader">
        <h2>Runtime</h2>
        <span>{clockSourceLabel(props.clock.source)}</span>
      </header>
      <label>
        Lighting Master
        <input
          type="number"
          min="0"
          max="1"
          step="0.01"
          value={props.lightingMaster}
          onChange={(event) => void props.onLightingMaster(Number(event.currentTarget.value))}
        />
      </label>
      <Show when={props.submasters.length > 0}>
        <div class="submasterList">
          <h3>Submasters</h3>
          <For each={props.submasters}>
            {(submaster) => (
              <label class="submasterControl">
                <span data-no-localize>{submaster.label}</span>
                <input
                  type="range"
                  min="0"
                  max="1"
                  step="0.01"
                  value={submaster.level}
                  onChange={(event) => void props.onSubmaster(submaster.group_id, Number(event.currentTarget.value))}
                />
                <strong>{Math.round(submaster.level * 100)}%</strong>
              </label>
            )}
          </For>
        </div>
      </Show>
      <div class="blackout">
        <button onClick={() => void props.onBlackout(true)}>DMX BO</button>
        <BlackoutReleaseControl />
        <button onClick={() => void props.onAllBlackout(true)}>All BO</button>
        <button onClick={() => void props.onAllBlackout(false)}>All Clear</button>
      </div>
      <div class="clock">
        <h3>Clock</h3>
        <div class="clockSource">
          <span>Source</span>
          <strong>{clockSourceLabel(props.clock.source)}</strong>
        </div>
        <div
          class={`clockSyncHealth ${props.clock.external_sync_locked ? "locked" : isExternalClockSource(props.clock.source) ? "stale" : "internal"}`}
          role="status"
        >
          <span>{clockSyncStatusLabel(props.clock)}</span>
          <small>
            <Show
              when={props.clock.external_sync_age_ms !== null}
              fallback={isExternalClockSource(props.clock.source) ? "Awaiting external sync" : "Local clock"}
            >
              {props.clock.external_sync_age_ms} ms since sync
            </Show>
          </small>
        </div>
        <div class="split">
          <label>
            BPM
            <input
              type="number"
              min="20"
              max="300"
              step="0.1"
              value={props.bpmDraft}
              onInput={(event) => props.onBpmDraft(event.currentTarget.value)}
            />
          </label>
          <label>
            Beat
            <input value={`${props.clock.beat_counter}.${Math.floor(props.clock.beat_phase * 100)}`} readOnly />
          </label>
        </div>
        <div class="blackout">
          <button onClick={() => void props.onApplyBpm()}>Set BPM</button>
          <button class="primary" onClick={() => void props.onTapBpm()}>
            Tap
          </button>
        </div>
        <div class="midiClock">
          <div class="buttonRow">
            <button onClick={() => void props.onRefreshMidi()}>Scan MIDI</button>
            <button onClick={() => void props.onDisconnectMidiClock()} disabled={!props.midiConnected}>
              Disconnect
            </button>
          </div>
          <select value={props.selectedMidiInput ?? ""} onInput={(event) => props.onSelectedMidiInput(Number(event.currentTarget.value))}>
            <For each={props.midiInputs}>{(input) => <option value={input.index}>{input.name}</option>}</For>
          </select>
          <button class="primary" onClick={() => void props.onConnectMidiClock()} disabled={props.midiInputs.length === 0}>
            Connect MIDI Clock / MTC
          </button>
        </div>
      </div>
    </section>
  );
}
