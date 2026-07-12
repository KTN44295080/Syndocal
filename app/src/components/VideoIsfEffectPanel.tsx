import { For, Show } from "solid-js";
import type { VideoIsfControlSummary, VideoIsfEffectSummary } from "../types";

interface VideoIsfEffectPanelProps {
  layerId: number;
  effect?: VideoIsfEffectSummary | null;
  runtimeError?: string | null;
  onImport: (layerId: number) => void | Promise<void>;
  onSetEffect: (layerId: number, effect: VideoIsfEffectSummary | null) => void | Promise<void>;
}

const replaceControl = (
  effect: VideoIsfEffectSummary,
  name: string,
  value: [number, number, number, number],
): VideoIsfEffectSummary => ({
  ...effect,
  controls: effect.controls.map((control) => (control.name === name ? { ...control, value } : control)),
});

const componentCount = (control: VideoIsfControlSummary) =>
  control.kind === "Color" ? 4 : control.kind === "Point2d" ? 2 : 1;

export function VideoIsfEffectPanel(props: VideoIsfEffectPanelProps) {
  const setControlComponent = (control: VideoIsfControlSummary, index: number, next: number) => {
    const effect = props.effect;
    if (!effect) return;
    const value = [...control.value] as [number, number, number, number];
    value[index] = next;
    void props.onSetEffect(props.layerId, replaceControl(effect, control.name, value));
  };

  const triggerEvent = (control: VideoIsfControlSummary) => {
    const effect = props.effect;
    if (!effect) return;
    const active = replaceControl(effect, control.name, [1, 0, 0, 0]);
    void Promise.resolve(props.onSetEffect(props.layerId, active)).then(() => {
      window.setTimeout(() => {
        void props.onSetEffect(props.layerId, replaceControl(active, control.name, [0, 0, 0, 0]));
      }, 50);
    });
  };

  return (
    <details class="videoIsfPanel" open={Boolean(props.runtimeError)}>
      <summary>
        <strong>ISF Shader</strong>
        <Show when={props.effect} fallback={<span>None</span>}>
          {(effect) => <span data-no-localize>{effect().label}</span>}
        </Show>
      </summary>
      <div class="videoIsfBody">
        <div class="buttonRow">
          <button onClick={() => void props.onImport(props.layerId)}>{props.effect ? "Replace ISF" : "Import ISF"}</button>
          <Show when={props.effect}>
            {(effect) => (
              <>
                <button
                  class={effect().enabled ? "active" : ""}
                  aria-pressed={effect().enabled}
                  onClick={() => void props.onSetEffect(props.layerId, { ...effect(), enabled: !effect().enabled })}
                >
                  {effect().enabled ? "Enabled" : "Bypassed"}
                </button>
                <button onClick={() => void props.onSetEffect(props.layerId, null)}>Clear ISF</button>
              </>
            )}
          </Show>
        </div>
        <p class="fieldHint">
          Portable single-pass ISF is embedded in the project. Unsupported resources and unsafe shader constructs are rejected before GPU use.
        </p>
        <Show when={props.runtimeError}>
          {(runtimeError) => <p class="inlineError" role="alert">{runtimeError()}</p>}
        </Show>
        <Show when={props.effect}>
          {(effect) => (
            <>
              <Show when={effect().description}>
                {(description) => <p class="fieldHint">{description()}</p>}
              </Show>
              <For each={effect().controls}>
                {(control) => (
                  <div class="videoIsfControl">
                    <span data-no-localize>{control.name}</span>
                    <Show
                      when={control.kind !== "Event"}
                      fallback={<button onClick={() => triggerEvent(control)}>Trigger</button>}
                    >
                      <Show
                        when={control.kind !== "Bool"}
                        fallback={
                          <label class="checkbox">
                            <input
                              type="checkbox"
                              checked={control.value[0] >= 0.5}
                              onChange={(event) =>
                                setControlComponent(control, 0, event.currentTarget.checked ? 1 : 0)
                              }
                            />
                            {control.value[0] >= 0.5 ? "On" : "Off"}
                          </label>
                        }
                      >
                        <For each={Array.from({ length: componentCount(control) }, (_, index) => index)}>
                          {(index) => (
                            <input
                              type="number"
                              min={control.minimum[index]}
                              max={control.maximum[index]}
                              step={control.kind === "Long" ? 1 : 0.01}
                              value={control.value[index]}
                              aria-label={`${control.name} component ${index + 1}`}
                              onChange={(event) =>
                                setControlComponent(control, index, Number(event.currentTarget.value))
                              }
                            />
                          )}
                        </For>
                      </Show>
                    </Show>
                  </div>
                )}
              </For>
              <Show when={effect().controls.length === 0}>
                <p class="emptyHint">This shader has no operator controls.</p>
              </Show>
            </>
          )}
        </Show>
      </div>
    </details>
  );
}
