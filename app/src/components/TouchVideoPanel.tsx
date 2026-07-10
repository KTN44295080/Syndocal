import { For, Show } from "solid-js";
import { clampRange } from "../numericHelpers";
import type { VideoLayerState, VideoLayerSummary, VideoOutputSummary } from "../types";
import { formatDuration, formatVideoTime } from "../videoHelpers";

interface TouchVideoPanelProps {
  layers: VideoLayerSummary[];
  outputs: VideoOutputSummary[];
  selectedOutputId: number | null;
  onSetLayerState: (layerId: number, state: VideoLayerState) => void | Promise<void>;
  onAddCuePoint: (layerId: number) => void | Promise<void>;
  onJumpCuePoint: (layerId: number, cuePointIndex: number) => void | Promise<void>;
  onSelectOutput: (outputId: number) => void;
  onSetOutputEnabled: (outputId: number, enabled: boolean) => void | Promise<void>;
  onSetOutputBlackout: (outputId: number, blackout: boolean) => void | Promise<void>;
  onSetOutputOpacity: (outputId: number, opacity: number) => void | Promise<void>;
  onFadeOutputOpacity: (outputId: number, opacity: number) => void | Promise<void>;
  onOpenOutputWindow: (outputId: number, testPattern?: boolean) => void | Promise<void>;
}

export function TouchVideoPanel(props: TouchVideoPanelProps) {
  return (
    <section class="panel touchPanel touchVideoPanel">
      <div class="panelHeader">
        <h2>Touch Video</h2>
        <span>{props.layers.length} layer(s) / {props.outputs.length} out(s)</span>
      </div>
      <Show when={props.outputs.length > 0}>
        <div class="touchVideoOutputGrid">
          <For each={props.outputs}>
            {(output) => {
              const selected = () => props.selectedOutputId === output.id;
              const selectOutput = () => props.onSelectOutput(output.id);
              const outputDeckClass = () =>
                [
                  "touchVideoOutputDeck",
                  output.enabled && !output.blackout ? "active" : "",
                  selected() ? "selected" : "",
                ]
                  .filter(Boolean)
                  .join(" ");

              return (
                <div class={outputDeckClass()}>
                  <div class="touchVideoHeader">
                    <div>
                      <strong>{output.label}</strong>
                      <span>
                        {output.kind} / {output.width}x{output.height}
                      </span>
                    </div>
                    <div class="touchVideoOutputStatus">
                      <button
                        class={selected() ? "touchVideoOutputSelect selected" : "touchVideoOutputSelect"}
                        aria-pressed={selected()}
                        onClick={selectOutput}
                      >
                        Sel
                      </button>
                      <small>{output.blackout ? "Blackout" : `${Math.round(output.opacity * 100)}%`}</small>
                    </div>
                  </div>
                  <label class="touchSlider">
                    Output
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={output.opacity}
                      onInput={(event) => {
                        selectOutput();
                        void props.onSetOutputOpacity(output.id, Number(event.currentTarget.value));
                      }}
                    />
                    <strong>{Math.round(output.opacity * 100)}%</strong>
                  </label>
                  <div class="touchTransportRow compact">
                    <button
                      class={output.enabled ? "primary" : ""}
                      onClick={() => {
                        selectOutput();
                        void props.onSetOutputEnabled(output.id, !output.enabled);
                      }}
                    >
                      {output.enabled ? "On" : "Off"}
                    </button>
                    <button
                      class={output.blackout ? "primary" : ""}
                      onClick={() => {
                        selectOutput();
                        void props.onSetOutputBlackout(output.id, !output.blackout);
                      }}
                    >
                      {output.blackout ? "Clear" : "BO"}
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onFadeOutputOpacity(output.id, 0);
                      }}
                    >
                      Out
                    </button>
                    <button
                      onClick={() => {
                        selectOutput();
                        void props.onFadeOutputOpacity(output.id, 1);
                      }}
                    >
                      In
                    </button>
                    <Show when={output.kind === "Display"}>
                      <button
                        onClick={() => {
                          selectOutput();
                          void props.onOpenOutputWindow(output.id, true);
                        }}
                      >
                        Pattern
                      </button>
                    </Show>
                  </div>
                </div>
              );
            }}
          </For>
        </div>
      </Show>
      <Show when={props.layers.length > 0} fallback={<p class="empty">No video layers. Add one in Setup &gt; Output.</p>}>
        <div class="touchVideoDeckGrid">
          <For each={props.layers}>
            {(layer) => {
              const durationMs = () => layer.source.metadata?.duration_ms ?? null;
              const progress = () => {
                const duration = durationMs();
                return duration && duration > 0 ? clampRange((layer.state.position_ms / duration) * 100, 0, 100) : 0;
              };
              const seekTo = (positionMs: number) =>
                props.onSetLayerState(layer.id, {
                  ...layer.state,
                  position_ms: durationMs()
                    ? Math.min(durationMs()!, Math.max(0, Math.round(positionMs)))
                    : Math.max(0, Math.round(positionMs)),
                });

              return (
                <div class={layer.state.enabled ? "touchVideoDeck active" : "touchVideoDeck"}>
                  <div class="touchVideoHeader">
                    <div>
                      <strong>{layer.label}</strong>
                      <span>{layer.source.path ?? layer.source.name ?? layer.source.kind}</span>
                    </div>
                    <small>{formatVideoTime(layer.state.position_ms, durationMs())}</small>
                  </div>
                  <div class="touchVideoProgress">
                    <span style={{ width: `${progress()}%` }} />
                  </div>
                  <div class="touchTransportRow">
                    <button
                      class={layer.state.playing ? "primary" : ""}
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          playing: !layer.state.playing,
                        })
                      }
                    >
                      {layer.state.playing ? "Pause" : "Play"}
                    </button>
                    <button
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          position_ms: layer.state.loop_enabled ? layer.state.loop_start_ms : 0,
                        })
                      }
                    >
                      Cue In
                    </button>
                    <button
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          speed: -Math.abs(layer.state.speed || 1),
                          playing: true,
                        })
                      }
                    >
                      Rev
                    </button>
                    <button
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          speed: Math.abs(layer.state.speed || 1),
                          playing: true,
                        })
                      }
                    >
                      Fwd
                    </button>
                  </div>
                  <label class="touchSlider">
                    Opacity
                    <input
                      type="range"
                      min="0"
                      max="1"
                      step="0.01"
                      value={layer.state.opacity}
                      onInput={(event) =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          opacity: Number(event.currentTarget.value),
                        })
                      }
                    />
                    <strong>{Math.round(layer.state.opacity * 100)}%</strong>
                  </label>
                  <label class="touchSlider">
                    Speed
                    <input
                      type="range"
                      min="-4"
                      max="4"
                      step="0.25"
                      value={layer.state.speed}
                      onChange={(event) =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          speed: Number(event.currentTarget.value),
                        })
                      }
                    />
                    <strong>{layer.state.speed.toFixed(2)}x</strong>
                  </label>
                  <Show when={durationMs()}>
                    {(duration) => (
                      <label class="touchSlider">
                        Seek
                        <input
                          type="range"
                          min="0"
                          max={duration()}
                          step="1"
                          value={layer.state.position_ms}
                          onChange={(event) => void seekTo(Number(event.currentTarget.value))}
                        />
                        <strong>{Math.round(progress())}%</strong>
                      </label>
                    )}
                  </Show>
                  <div class="touchTransportRow">
                    <button
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          enabled: !layer.state.enabled,
                        })
                      }
                    >
                      {layer.state.enabled ? "Disable" : "Enable"}
                    </button>
                    <button
                      class={layer.state.solo ? "primary" : ""}
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          solo: !layer.state.solo,
                        })
                      }
                    >
                      Solo
                    </button>
                    <button
                      class={layer.state.loop_enabled ? "primary" : ""}
                      onClick={() =>
                        void props.onSetLayerState(layer.id, {
                          ...layer.state,
                          loop_enabled: !layer.state.loop_enabled,
                        })
                      }
                    >
                      Loop
                    </button>
                    <button onClick={() => void props.onAddCuePoint(layer.id)}>Add Cue</button>
                  </div>
                  <Show when={layer.state.cue_points_ms.length > 0}>
                    <div class="touchCuePointRow">
                      <For each={layer.state.cue_points_ms.slice(0, 6)}>
                        {(cuePoint, cuePointIndex) => (
                          <button onClick={() => void props.onJumpCuePoint(layer.id, cuePointIndex())}>
                            {formatDuration(cuePoint) ?? `${cuePoint}ms`}
                          </button>
                        )}
                      </For>
                    </div>
                  </Show>
                </div>
              );
            }}
          </For>
        </div>
      </Show>
    </section>
  );
}
