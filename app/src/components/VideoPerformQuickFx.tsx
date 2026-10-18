import { createEffect, createMemo, createSignal, For, Match, on, Show, Switch, type ComponentProps } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { VideoIsfControlSummary, VideoLayerSummary } from "../types";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./VideoPerformQuickFx.css";

interface VideoPerformQuickFxProps {
  layer: VideoLayerSummary | null;
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  onOpenEditor: () => void;
}

interface QuickStage {
  id: string;
  layerId: number;
  index: number;
  label: string;
  enabled: boolean;
  controls: (VideoIsfControlSummary & { id: string })[];
}

/** Value readout for a control. ISF summaries carry no unit metadata, so the only
 * unit shown is the percent sign of a normalised 0-1 float. */
export const quickFxValueText = (control: VideoIsfControlSummary): string => {
  const first = control.value[0];
  switch (control.kind) {
    case "Float": {
      if (control.minimum[0] === 0 && control.maximum[0] === 1) return `${Math.round(first * 100)} %`;
      return String(Number(first.toFixed(2)));
    }
    case "Long": {
      const at = control.values.indexOf(first);
      return at >= 0 && control.labels[at] ? control.labels[at] : String(Math.round(first));
    }
    case "Point2d":
      return `${Number(control.value[0].toFixed(2))}, ${Number(control.value[1].toFixed(2))}`;
    default:
      return "";
  }
};

/** A view of the existing layer-scoped ISF stack, with no separate FX state or
 * publication path. Stable stage/control stores retain focused DOM across polls. */
export function VideoPerformQuickFx(props: VideoPerformQuickFxProps) {
  const [stages, setStages] = createStore<QuickStage[]>([]);
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  createEffect(() => {
    const layer = props.layer;
    const effect = layer?.isf_effect;
    const next = layer && effect ? [effect, ...(effect.stack ?? [])].map((stage, index) => ({
      id: `${layer.id}:${index}`,
      layerId: layer.id,
      index,
      label: stage.label,
      enabled: stage.enabled,
      // The shader's declared order supplies its primary operator controls.
      controls: stage.controls.map((control) => ({ ...control, id: control.name })),
    })) : [];
    setStages(reconcile(next, { key: "id" }));
  });
  // A different layer starts at its first stage; a shorter chain clamps.
  const layerId = createMemo(() => props.layer?.id ?? null);
  createEffect(on(layerId, () => setSelectedIndex(0), { defer: true }));
  const activeStage = createMemo(() => stages[Math.min(selectedIndex(), Math.max(0, stages.length - 1))] ?? null);

  const busy = () => typeof props.layerList.isfEventPulseBusy === "function"
    ? props.layerList.isfEventPulseBusy()
    : Boolean(props.layerList.isfEventPulseBusy);
  const controlLabel = (stage: QuickStage, control: VideoIsfControlSummary) =>
    `${control.name} for ${stage.label} FX on ${props.layer?.label ?? "Video layer"} (layer ${stage.layerId})`;
  const setComponent = (stage: QuickStage, control: VideoIsfControlSummary, component: number, value: number) => {
    if (!Number.isFinite(value)) return;
    const next = [...control.value] as [number, number, number, number];
    next[component] = value;
    void props.layerList.onSetIsfControl(stage.layerId, stage.index, control.name, next);
  };
  const colorValue = (control: VideoIsfControlSummary) => `#${control.value.slice(0, 3)
    .map((component) => Math.round(Math.max(0, Math.min(1, component)) * 255).toString(16).padStart(2, "0"))
    .join("")}`;

  return (
    <section class="videoPerformQuickFx" data-perform-quick-fx data-video-perform-region="quick-fx" aria-label="Selected layer effect chain">
      <header class="videoPerformQuickFxHeader">
        <div class="videoPerformQuickFxLayer">
          <span class="videoPerformQuickFxLayerName" data-no-localize title={props.layer?.label ?? ""}>{props.layer?.label ?? ""}</span>
          <Show when={stages.length > 0}>
            <span class="videoPerformQuickFxCount"><span data-no-localize>{stages.length}</span> <span>{stages.length === 1 ? "stage" : "stages"}</span></span>
          </Show>
        </div>
        <button type="button" data-perform-fx-editor data-video-perform-fx-editor disabled={!props.layer} onClick={props.onOpenEditor}>
          Full FX editor
        </button>
      </header>
      <div class="videoPerformQuickFxBody" data-fx-empty={stages.length === 0}>
        <div class="videoPerformQuickFxChain" role="list" aria-label="Selected layer effect chain"
          title="ISF stage MIDI Learn is unavailable in the existing control model.">
          <Show when={stages.length > 0} fallback={
            <p class="emptyState">{props.layer ? "No layer FX. Add effects in the full editor." : "Select a layer for Quick FX."}</p>
          }>
            <For each={stages}>
              {(stage) => (
                <div class="videoPerformQuickFxStage" data-perform-fx-stage={stage.index} data-perform-fx-layer={stage.layerId}
                  data-fx-enabled={stage.enabled} data-fx-selected={activeStage()?.id === stage.id} role="listitem">
                  <button type="button" class="videoPerformQuickFxSelect" data-perform-fx-select
                    aria-pressed={activeStage()?.id === stage.id} onClick={() => setSelectedIndex(stage.index)}>
                    <span class="videoPerformQuickFxNumber" aria-hidden="true">{stage.index + 1}</span>
                    <strong data-no-localize title={stage.label}>{stage.label}</strong>
                  </button>
                  <button type="button" class="videoPerformQuickFxSwitch" data-perform-fx-bypass aria-pressed={stage.enabled}
                    aria-label={`${stage.label} FX enabled for ${props.layer?.label ?? "Video layer"} (layer ${stage.layerId})`}
                    classList={{ active: stage.enabled }} disabled={busy()}
                    onClick={() => void props.layerList.onSetIsfEffectEnabled(stage.layerId, stage.index, !stage.enabled)}>
                    <span class="videoPerformQuickFxKnob" aria-hidden="true" />
                    <span>{stage.enabled ? "On" : "Off"}</span>
                  </button>
                </div>
              )}
            </For>
          </Show>
        </div>
        <div class="videoPerformQuickFxParameters" data-perform-fx-detail={activeStage()?.index ?? ""} aria-label="Selected stage parameters">
          <Show when={activeStage()}>
            {(stage) => (
              <Show when={stage().controls.length > 0} fallback={<span class="emptyHint">No inline parameters.</span>}>
                <For each={stage().controls}>
                  {(control) => (
                    <label class="videoPerformQuickFxParameter" data-perform-fx-control={control.name} data-perform-fx-kind={control.kind}>
                      <span class="videoPerformQuickFxParameterHead">
                        <span data-no-localize title={control.name}>{control.name}</span>
                        <Show when={quickFxValueText(control)}>
                          {(text) => <output class="videoPerformQuickFxValue" data-no-localize>{text()}</output>}
                        </Show>
                      </span>
                      <Switch>
                        <Match when={control.kind === "Event"}>
                          <button type="button" disabled={busy()} aria-label={controlLabel(stage(), control)}
                            onClick={() => void props.layerList.onTriggerIsfEvent(stage().layerId, stage().index, control.name)}>Trigger</button>
                        </Match>
                        <Match when={control.kind === "Bool"}>
                          <button type="button" disabled={busy()} aria-pressed={control.value[0] >= 0.5}
                            aria-label={controlLabel(stage(), control)} class={control.value[0] >= 0.5 ? "active" : ""}
                            onClick={() => setComponent(stage(), control, 0, control.value[0] >= 0.5 ? 0 : 1)}>
                            {control.value[0] >= 0.5 ? "On" : "Off"}
                          </button>
                        </Match>
                        <Match when={control.kind === "Color"}>
                          <input type="color" value={colorValue(control)} disabled={busy()} aria-label={controlLabel(stage(), control)}
                            onInput={(event) => {
                              const hex = event.currentTarget.value;
                              void props.layerList.onSetIsfControl(stage().layerId, stage().index, control.name,
                                [parseInt(hex.slice(1, 3), 16) / 255, parseInt(hex.slice(3, 5), 16) / 255,
                                  parseInt(hex.slice(5, 7), 16) / 255, control.value[3]]);
                            }} />
                        </Match>
                        <Match when={control.kind === "Long" && control.values.length > 0}>
                          <select disabled={busy()} value={control.value[0]} aria-label={controlLabel(stage(), control)}
                            onChange={(event) => setComponent(stage(), control, 0, Number(event.currentTarget.value))}>
                            <For each={control.values}>{(value, index) =>
                              <option value={value} data-no-localize>{control.labels[index()] ?? value}</option>}
                            </For>
                          </select>
                        </Match>
                        <Match when={control.kind === "Point2d"}>
                          <div class="videoPerformQuickFxPoint">
                            <For each={[0, 1]}>{(component) => (
                              <input type="number" disabled={busy()} min={control.minimum[component]} max={control.maximum[component]}
                                step="0.01" value={control.value[component]}
                                aria-label={`${control.name} component ${component + 1} for ${props.layer?.label ?? "Video layer"} (layer ${stage().layerId})`}
                                onChange={(event) => setComponent(stage(), control, component, Number(event.currentTarget.value))} />
                            )}</For>
                          </div>
                        </Match>
                        <Match when={control.kind === "Float" || control.kind === "Long"}>
                          <input type="range" disabled={busy()} min={control.minimum[0]} max={control.maximum[0]}
                            step={control.kind === "Long" ? 1 : "any"} value={control.value[0]} aria-label={controlLabel(stage(), control)}
                            aria-valuetext={String(Number(control.value[0].toFixed(3)))}
                            onInput={(event) => setComponent(stage(), control, 0, Number(event.currentTarget.value))} />
                        </Match>
                      </Switch>
                    </label>
                  )}
                </For>
              </Show>
            )}
          </Show>
        </div>
      </div>
    </section>
  );
}
