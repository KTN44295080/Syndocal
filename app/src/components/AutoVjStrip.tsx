import { createMemo, For, Show } from "solid-js";
import type { AutoVjConfig, AutoVjSnapshot, VideoLayerSummary } from "../types";

export interface AutoVjStripProps {
  snapshot: AutoVjSnapshot;
  layers: VideoLayerSummary[];
  busy: boolean;
  onSetConfig: (config: AutoVjConfig) => void | Promise<void>;
  onSetArmed: (armed: boolean) => void | Promise<void>;
  onSetHold: (hold: boolean) => void | Promise<void>;
}

const changeBeatOptions = [1, 2, 4, 8, 16, 32] as const;
const transitionOptions = [0, 80, 160, 250, 500, 1_000, 2_000] as const;

export const defaultAutoVjSnapshot = (): AutoVjSnapshot => ({
  config: {
    eligible_layer_ids: [],
    seed: 0,
    beats_per_change: 4,
    transition_ms: 500,
    avoid_immediate_repeat: true,
    rhythm_source: "Clock",
  },
  status: {
    mode: "Off",
    armed: false,
    hold: false,
    show_revision: 0,
    action_sequence: 0,
    last_consumed_boundary: null,
    next_boundary_beat: null,
    last_action: null,
    action_log: [],
    fault: null,
    live_audio_beat_counter: 0,
    last_live_audio_feature_sequence: null,
  },
});

const layerLabel = (layers: VideoLayerSummary[], layerId: number): string =>
  layers.find((layer) => layer.id === layerId)?.label ?? `Missing ${layerId}`;

export function AutoVjStrip(props: AutoVjStripProps) {
  const status = () => props.snapshot.status;
  const config = () => props.snapshot.config;
  const selectedIds = createMemo(() => new Set(config().eligible_layer_ids));
  const configLocked = () => props.busy || status().armed;
  const eligibleLayers = createMemo(() =>
    props.layers.filter((layer) => selectedIds().has(layer.id)),
  );
  const eligibleLabel = createMemo(() =>
    config().eligible_layer_ids.length === 0
      ? "Not set"
      : `${eligibleLayers().length}/${props.layers.length}`,
  );
  const secondaryStatusLabel = createMemo(() => {
    if (config().rhythm_source === "LiveAudio") {
      const beat = status().live_audio_beat_counter;
      const interval = Math.max(1, config().beats_per_change);
      const step = beat % interval;
      const feature = status().last_live_audio_feature_sequence;
      return `INPUT PULSE ${beat} · STEP ${step}/${interval} · FSEQ ${feature ?? "—"}`;
    }
    const next = status().next_boundary_beat;
    return `NEXT ${next ?? "—"} · REV ${status().show_revision}`;
  });
  const updateConfig = (patch: Partial<AutoVjConfig>) =>
    props.onSetConfig({ ...config(), ...patch });
  const toggleEligibleLayer = (layerId: number, enabled: boolean) => {
    const explicit = config().eligible_layer_ids;
    const next = enabled
      ? [...new Set([...explicit, layerId])]
      : explicit.filter((candidate) => candidate !== layerId);
    void updateConfig({ eligible_layer_ids: next });
  };

  return (
    <section
      class={`autoVjStrip mode-${status().mode.toLowerCase()}`}
      aria-label="Deterministic Auto VJ director"
      data-mode={status().mode}
      data-rhythm-source={config().rhythm_source}
      aria-busy={props.busy}
    >
      <div class="autoVjIdentity">
        <small>AUTO VJ</small>
        <strong>{status().mode}</strong>
      </div>
      <button
        class={status().armed ? "danger" : "primary"}
        disabled={props.busy || (!status().armed && props.layers.length === 0)}
        aria-pressed={status().armed}
        title={!status().armed && props.layers.length === 0 ? "Add a video clip to enable Auto VJ" : undefined}
        onClick={() => void props.onSetArmed(!status().armed)}
      >
        {status().armed ? "Disarm" : "Arm"}
      </button>
      <button
        class={status().hold ? "active" : ""}
        disabled={props.busy || !status().armed}
        aria-pressed={status().hold}
        onClick={() => void props.onSetHold(!status().hold)}
      >
        {status().hold ? "Resume" : "Hold"}
      </button>
      <label class="autoVjRhythm">
        <span>Rhythm</span>
        <select
          aria-label="Auto VJ rhythm source"
          disabled={configLocked()}
          value={config().rhythm_source}
          onInput={(event) =>
            void updateConfig({
              rhythm_source: event.currentTarget.value === "LiveAudio" ? "LiveAudio" : "Clock",
            })
          }
        >
          <option value="Clock">CLOCK</option>
          <option value="LiveAudio">LIVE INPUT</option>
        </select>
      </label>
      <label class="autoVjChange">
        <span>Change</span>
        <select
          aria-label={config().rhythm_source === "LiveAudio" ? "Auto VJ pulses per change" : "Auto VJ beats per change"}
          disabled={configLocked()}
          value={config().beats_per_change}
          onInput={(event) => void updateConfig({ beats_per_change: Number(event.currentTarget.value) })}
        >
          <For each={changeBeatOptions}>
            {(beats) => (
              <option value={beats}>
                {beats} {config().rhythm_source === "LiveAudio" ? `pulse${beats === 1 ? "" : "s"}` : `beat${beats === 1 ? "" : "s"}`}
              </option>
            )}
          </For>
        </select>
      </label>
      <label class="autoVjTransition">
        <span>Transition</span>
        <select
          aria-label="Auto VJ transition duration"
          disabled={configLocked()}
          value={config().transition_ms}
          onInput={(event) => void updateConfig({ transition_ms: Number(event.currentTarget.value) })}
        >
          <For each={transitionOptions}>
            {(durationMs) => <option value={durationMs}>{durationMs === 0 ? "Cut" : `${durationMs} ms`}</option>}
          </For>
        </select>
      </label>
      <label class="autoVjSeed">
        <span>Seed</span>
        <input
          aria-label="Auto VJ deterministic seed"
          type="number"
          min="0"
          max="4294967295"
          step="1"
          disabled={configLocked()}
          value={config().seed}
          onChange={(event) =>
            void updateConfig({
              seed: Math.max(0, Math.min(4_294_967_295, Math.trunc(Number(event.currentTarget.value) || 0))),
            })
          }
        />
      </label>
      <details class="autoVjEligiblePicker">
        <summary
          title={status().armed ? "Disarm Auto VJ to edit configuration" : "Choose clips eligible for automatic Takes"}
          aria-disabled={status().armed ? "true" : undefined}
        >
          Clips {eligibleLabel()}
        </summary>
        <div>
          <button
            disabled={configLocked() || props.layers.length === 0}
            onClick={() => void updateConfig({ eligible_layer_ids: props.layers.map((layer) => layer.id) })}
          >
            Use all clips
          </button>
          <Show
            when={props.layers.length > 0}
            fallback={<p>Add a video clip to enable Auto VJ.</p>}
          >
            <For each={props.layers}>
              {(layer) => (
                <label>
                  <input
                    type="checkbox"
                    checked={selectedIds().has(layer.id)}
                    disabled={
                      configLocked() ||
                      (selectedIds().size === 1 && selectedIds().has(layer.id))
                    }
                    onChange={(event) => toggleEligibleLayer(layer.id, event.currentTarget.checked)}
                  />
                  <span data-no-localize>{layer.label}</span>
                </label>
              )}
            </For>
          </Show>
        </div>
      </details>
      <label class="autoVjAvoidRepeat">
        <input
          type="checkbox"
          checked={config().avoid_immediate_repeat}
          disabled={configLocked()}
          onChange={(event) => void updateConfig({ avoid_immediate_repeat: event.currentTarget.checked })}
        />
        <span>No repeat</span>
      </label>
      <div class="autoVjReadout" role="status" aria-live="polite">
        <Show
          when={status().fault}
          fallback={
            <Show when={status().last_action} fallback={<span>Waiting for a quantized boundary</span>}>
              {(action) => (
                <span>
                  <b>{`#${action().sequence} · `}</b>
                  <b data-no-localize>{layerLabel(props.layers, action().layer_id)}</b>
                  <b>{` · ${action().trigger === "LiveAudioOnset" ? "INPUT PULSE" : "CLOCK BEAT"} ${action().beat}`}</b>
                </span>
              )}
            </Show>
          }
        >
          {(fault) => <span class="danger" data-no-localize>{fault()}</span>}
        </Show>
        <small>{secondaryStatusLabel()}</small>
      </div>
    </section>
  );
}
