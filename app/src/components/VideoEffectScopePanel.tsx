import { For, Show, createMemo, createSignal } from "solid-js";
import type {
  VideoEffectCatalog,
  VideoEffectChainSummary,
  VideoEffectPresetId,
  VideoEffectScope,
} from "../types";

interface VideoEffectScopePanelProps {
  scopeLabel: string;
  title?: string;
  scope: VideoEffectScope;
  catalog: VideoEffectCatalog;
  compact?: boolean;
  busy?: boolean;
  onApplyCatalog: (catalog: VideoEffectCatalog) => void | Promise<unknown>;
  onImportIsf: (scope: VideoEffectScope) => void | Promise<unknown>;
  onApplyPreset: (scope: VideoEffectScope, presetId: VideoEffectPresetId) => void | Promise<unknown>;
  onRemoveChain: (scope: VideoEffectScope) => void | Promise<unknown>;
}

const sameScope = (left: VideoEffectScope, right: VideoEffectScope) =>
  JSON.stringify(left) === JSON.stringify(right);

export function VideoEffectScopePanel(props: VideoEffectScopePanelProps) {
  const [presetId, setPresetId] = createSignal<number | null>(null);
  const chain = createMemo(() => props.catalog.effect_chains.find((entry) => sameScope(entry.scope, props.scope)) ?? null);
  const replaceChain = (next: VideoEffectChainSummary) => props.onApplyCatalog({
    effect_chains: props.catalog.effect_chains.map((entry) => entry.id === next.id ? next : entry),
    effect_presets: props.catalog.effect_presets,
    layer_groups: props.catalog.layer_groups,
    transition_buses: props.catalog.transition_buses,
  });
  const setBypassed = (value: boolean) => {
    const current = chain();
    if (current) return replaceChain({ ...current, bypassed: value });
  };
  const setStageEnabled = (stageId: number, enabled: boolean) => {
    const current = chain();
    if (!current) return;
    return replaceChain({
      ...current,
      stages: current.stages.map((stage) => stage.id === stageId ? { ...stage, enabled } : stage),
    });
  };
  const moveStage = (stageId: number, delta: -1 | 1) => {
    const current = chain();
    if (!current) return;
    const index = current.stages.findIndex((stage) => stage.id === stageId);
    const target = index + delta;
    if (index < 0 || target < 0 || target >= current.stages.length) return;
    const stages = [...current.stages];
    [stages[index], stages[target]] = [stages[target], stages[index]];
    return replaceChain({ ...current, stages });
  };
  const resetStage = (stageId: number) => {
    const current = chain();
    if (!current) return;
    return replaceChain({
      ...current,
      stages: current.stages.map((stage) => stage.id !== stageId ? stage : {
        ...stage,
        effect: {
          ...stage.effect,
          kind: {
            ...stage.effect.kind,
            effect: {
              ...stage.effect.kind.effect,
              controls: stage.effect.kind.effect.controls.map((control) => ({
                ...control,
                value: control.kind === "Event" ? [0, 0, 0, 0] : [...control.default],
              })),
            },
          },
        },
      }),
    });
  };
  const setControlComponent = (stageId: number, controlName: string, component: number, value: number) => {
    const current = chain();
    if (!current || !Number.isFinite(value)) return;
    return replaceChain({
      ...current,
      stages: current.stages.map((stage) => stage.id !== stageId ? stage : {
        ...stage,
        effect: {
          ...stage.effect,
          kind: {
            ...stage.effect.kind,
            effect: {
              ...stage.effect.kind.effect,
              controls: stage.effect.kind.effect.controls.map((control) => {
                if (control.name !== controlName || control.kind === "Event") return control;
                const next = [...control.value] as [number, number, number, number];
                next[component] = value;
                return { ...control, value: next };
              }),
            },
          },
        },
      }),
    });
  };
  const removeStage = (stageId: number) => {
    const current = chain();
    if (!current) return;
    const stage = current.stages.find((entry) => entry.id === stageId);
    if (!stage || !globalThis.confirm(`Remove FX “${stage.label}”?`)) return;
    if (current.stages.length === 1) return props.onRemoveChain(props.scope);
    return replaceChain({ ...current, stages: current.stages.filter((entry) => entry.id !== stageId) });
  };

  return (
    <details class={`videoEffectScopePanel ${props.compact ? "compact" : ""}`} data-effect-scope={props.scope.scope}>
      <summary>
        <strong>
          <span>{props.scopeLabel}</span>
          <Show when={props.title}>{(title) => <span data-no-localize> · {title()}</span>}</Show>
        </strong>
        <span>{chain()?.stages.length ?? 0} FX</span>
        <Show when={chain()?.bypassed}><span class="statusPill warning">Bypassed</span></Show>
      </summary>
      <div class="videoEffectScopeBody" aria-busy={props.busy ?? false}>
        <Show when={chain()} fallback={<p class="emptyState">No effect chain on this scope.</p>}>
          {(current) => (
            <>
              <label class="checkbox">
                <input
                  type="checkbox"
                  checked={!current().bypassed}
                  disabled={props.busy}
                  onChange={(event) => void setBypassed(!event.currentTarget.checked)}
                />
                Effect chain enabled
              </label>
              <ol class="videoEffectStageList">
                <For each={current().stages}>
                  {(stage, index) => (
                    <li data-effect-stage-id={stage.id} data-effect-id={stage.effect.id}>
                      <div>
                        <strong data-no-localize>{stage.label}</strong>
                        <small data-no-localize>Stage {stage.id} · Effect {stage.effect.id}</small>
                      </div>
                      <div class="buttonRow">
                        <button
                          class={stage.enabled ? "active" : ""}
                          disabled={props.busy}
                          onClick={() => void setStageEnabled(stage.id, !stage.enabled)}
                        >
                          {stage.enabled ? "On" : "Bypass"}
                        </button>
                        <button disabled={props.busy || index() === 0} onClick={() => void moveStage(stage.id, -1)} aria-label={`Move ${stage.label} earlier`}>Up</button>
                        <button disabled={props.busy || index() === current().stages.length - 1} onClick={() => void moveStage(stage.id, 1)} aria-label={`Move ${stage.label} later`}>Down</button>
                        <button disabled={props.busy} onClick={() => void resetStage(stage.id)}>Reset</button>
                        <button class="danger" disabled={props.busy} onClick={() => void removeStage(stage.id)}>Remove</button>
                      </div>
                      <Show when={stage.effect.kind.effect.controls.length > 0}>
                        <details class="videoEffectStageControls">
                          <summary>Parameters</summary>
                          <For each={stage.effect.kind.effect.controls}>
                            {(control) => (
                              <div class="videoEffectStageControl" data-effect-control={control.name}>
                                <span data-no-localize>{control.name}</span>
                                <Show when={control.kind === "Event"} fallback={
                                  <For each={control.kind === "Point2d" ? [0, 1] : control.kind === "Color" ? [0, 1, 2, 3] : [0]}>
                                    {(component) => (
                                      <input
                                        type={control.kind === "Bool" ? "checkbox" : "number"}
                                        checked={control.kind === "Bool" ? control.value[0] >= 0.5 : undefined}
                                        min={control.kind === "Bool" ? undefined : control.minimum[component]}
                                        max={control.kind === "Bool" ? undefined : control.maximum[component]}
                                        step={control.kind === "Long" ? 1 : 0.01}
                                        value={control.kind === "Bool" ? undefined : control.value[component]}
                                        disabled={props.busy}
                                        aria-label={`${control.name} ${component + 1}`}
                                        onChange={(event) => void setControlComponent(
                                          stage.id,
                                          control.name,
                                          component,
                                          control.kind === "Bool" ? (event.currentTarget.checked ? 1 : 0) : Number(event.currentTarget.value),
                                        )}
                                      />
                                    )}
                                  </For>
                                }>
                                  <button disabled title="Use the live Layer Event control for momentary pulses.">Event pulse</button>
                                </Show>
                              </div>
                            )}
                          </For>
                        </details>
                      </Show>
                    </li>
                  )}
                </For>
              </ol>
            </>
          )}
        </Show>
        <div class="videoEffectScopeActions">
          <button disabled={props.busy} onClick={() => void props.onImportIsf(props.scope)}>Add ISF</button>
          <Show when={props.catalog.effect_presets.length > 0}>
            <label>
              Preset
              <select value={presetId() ?? ""} onChange={(event) => setPresetId(Number(event.currentTarget.value) || null)}>
                <option value="">Choose preset</option>
                <For each={props.catalog.effect_presets}>{(preset) => <option value={preset.id} data-no-localize>{preset.label}</option>}</For>
              </select>
            </label>
            <button
              disabled={props.busy || presetId() === null}
              onClick={() => { const id = presetId(); if (id !== null) void props.onApplyPreset(props.scope, id); }}
            >
              Apply preset
            </button>
          </Show>
          <Show when={chain()}>
            <button
              class="danger"
              disabled={props.busy}
              onClick={() => { if (globalThis.confirm(`Remove ${props.scopeLabel} effect chain?`)) void props.onRemoveChain(props.scope); }}
            >
              Remove chain
            </button>
          </Show>
        </div>
      </div>
    </details>
  );
}
