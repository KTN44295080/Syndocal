import { createEffect, createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { VideoEffectCatalog, VideoIsfControlSummary, VideoIsfEffectStageSummary } from "../types";
import { boundedFxValue, builtinFxStagePresets, fxControlLabel, fxControlValueText, fxDisplayScale, matchingFxStagePresets, randomizeFxStage, saveFxStagePreset, type FxValue } from "../fxParameterModel";
import "./VideoFxParameterEditor.css";
import { VideoFxPointPad } from "./VideoFxPointPad";
import { translateUiText } from "../uiLocalization";
import type { FxLiveSession } from "../createFxLiveSession";

interface VideoFxParameterEditorProps {
  projectEpoch?: number;
  layerId: number;
  liveSession?: FxLiveSession;
  /** Overrides the live Undo, for example with a Preview audition's own history. */
  history?: { canUndo: () => boolean; undo: () => unknown | Promise<unknown> };
  /** Event controls fire on Program, so an audition cannot offer them. */
  auditioning?: boolean;
  locale?: string;
  stage: VideoIsfEffectStageSummary;
  identity: string;
  busy: boolean;
  effectCatalog?: VideoEffectCatalog;
  onApplyEffectCatalog?: (catalog: VideoEffectCatalog) => unknown | Promise<unknown>;
  onSetStage: (stage: VideoIsfEffectStageSummary) => unknown | Promise<unknown>;
  onSetControl: (name: string, value: FxValue) => unknown | Promise<unknown>;
  onTriggerEvent: (name: string) => unknown | Promise<unknown>;
}

/** A single editor for Perform and the full FX workspace. Reconcile keyed controls
 * so polling changes values without replacing focused inputs. */
export function VideoFxParameterEditor(props: VideoFxParameterEditorProps) {
  const [controls, setControls] = createStore<VideoIsfControlSummary[]>([]);
  const [name, setName] = createSignal("");
  const [presetId, setPresetId] = createSignal("");
  const [pending, setPending] = createSignal(false);
  const [error, setError] = createSignal(false);
  const identity = createMemo(() => `${props.projectEpoch}:${props.identity}:${props.stage.source}`);
  createEffect(() => { identity(); setPresetId(""); setName(""); });
  createEffect(() => setControls(reconcile(props.stage.controls, { key: "name" })));
  const disabled = () => props.busy || pending();
  const presets = createMemo(() => matchingFxStagePresets(props.effectCatalog, props.stage));
  const builtins = createMemo(() => builtinFxStagePresets(props.stage));
  const canUndo = () => props.history
    ? props.history.canUndo()
    : props.liveSession?.current()?.layerId === props.layerId && props.liveSession.canUndo();
  const run = async (action: () => unknown | Promise<unknown>) => {
    if (disabled()) return;
    setPending(true); setError(false);
    try { await action(); } catch { setError(true); } finally { setPending(false); }
  };
  const apply = (stage: VideoIsfEffectStageSummary) => run(() => props.onSetStage(stage));
  // Range/color widgets commit on native change (gesture completion). Publishing
  // every pointer move would race the authoritative single-flight FX command.
  const setComponent = (control: VideoIsfControlSummary, component: number, value: number) => {
    const next = [...control.value] as FxValue; next[component] = value;
    void props.onSetControl(control.name, boundedFxValue(control, next));
  };
  const colorHex = (control: VideoIsfControlSummary) => `#${control.value.slice(0, 3).map((value) =>
    Math.round(Math.max(0, Math.min(1, value)) * 255).toString(16).padStart(2, "0")).join("")}`;
  const recall = () => {
    const id = presetId();
    const stage = id.startsWith("builtin:") ? builtins()[Number(id.slice(8))]?.stage
      : presets().find((preset) => String(preset.id) === id)?.payload.stages[0].effect.effect;
    if (stage) void apply({ ...stage, enabled: props.stage.enabled });
  };
  return (
    <div class="videoFxParameterEditor" data-fx-parameter-editor>
      <For each={controls}>{(control) => (
        <div class="videoFxParameter" data-perform-fx-control={control.name} data-perform-fx-kind={control.kind}
          data-video-isf-control-kind={control.kind} data-video-isf-control-name={control.name}>
          <div class="videoFxParameterHead">
            <label title={control.name}>{fxControlLabel(control, props.stage.source)}</label>
            <output>{fxControlValueText(control, props.stage)}</output>
          </div>
          <Switch>
            <Match when={control.kind === "Event"}><button type="button" disabled={disabled() || props.auditioning} title={props.auditioning ? "Event controls fire on Program, so they are off during an audition." : undefined} aria-label={fxControlLabel(control, props.stage.source)} onClick={() => void props.onTriggerEvent(control.name)}>Trigger</button></Match>
            <Match when={control.kind === "Bool"}><button type="button" disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} aria-pressed={control.value[0] >= 0.5} onClick={() => setComponent(control, 0, control.value[0] >= 0.5 ? 0 : 1)}>{control.value[0] >= 0.5 ? "On" : "Off"}</button></Match>
            <Match when={control.kind === "Color"}>
              <div class="videoFxColorPicker"><input type="color" disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} value={colorHex(control)} onChange={(event) => {
                const hex = event.currentTarget.value;
                void props.onSetControl(control.name, boundedFxValue(control, [parseInt(hex.slice(1, 3), 16) / 255, parseInt(hex.slice(3, 5), 16) / 255, parseInt(hex.slice(5, 7), 16) / 255, control.value[3]]));
              }} /><label><span>Alpha</span><input type="range" min={control.minimum[3]} max={control.maximum[3]} step="any" value={control.value[3]} disabled={disabled()} aria-label="Alpha" onChange={(event) => setComponent(control, 3, Number(event.currentTarget.value))} /></label></div>
            </Match>
            <Match when={control.kind === "Long" && control.values.length > 0}><select disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} value={control.value[0]} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value))}><For each={control.values}>{(value, index) => <option value={value}>{control.labels[index()] ?? value}</option>}</For></select></Match>
            <Match when={control.kind === "Point2d"}>
              <div class="videoFxPointPicker" role="group" aria-label={fxControlLabel(control, props.stage.source)}>
                <VideoFxPointPad control={control} label={fxControlLabel(control, props.stage.source)} disabled={disabled()}
                  onCommit={(x, y) => void props.onSetControl(control.name, boundedFxValue(control, [x, y, control.value[2], control.value[3]]))} />
                <For each={[0, 1]}>{(component) => <label><span>{component === 0 ? "X" : "Y"}</span><input type="range" min={control.minimum[component]} max={control.maximum[component]} step="any" value={control.value[component]} disabled={disabled()} aria-label={`${fxControlLabel(control, props.stage.source)} ${component === 0 ? "X" : "Y"}`} onChange={(event) => setComponent(control, component, Number(event.currentTarget.value))} /><input type="number" min={control.minimum[component]} max={control.maximum[component]} step="0.01" value={control.value[component]} disabled={disabled()} aria-label={`${fxControlLabel(control, props.stage.source)} ${component === 0 ? "X" : "Y"}`} onChange={(event) => setComponent(control, component, Number(event.currentTarget.value))} /></label>}</For>
              </div>
            </Match>
            <Match when={control.kind === "Float" || control.kind === "Long"}><div class="videoFxScalarPicker"><input type="range" min={control.minimum[0]} max={control.maximum[0]} step={control.kind === "Long" ? 1 : "any"} value={control.value[0]} aria-valuetext={fxControlValueText(control, props.stage)} disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value))} /><input type="number" min={Number((control.minimum[0] * fxDisplayScale(control, props.stage)).toFixed(3))} max={Number((control.maximum[0] * fxDisplayScale(control, props.stage)).toFixed(3))} step={control.kind === "Long" ? 1 : fxDisplayScale(control, props.stage) === 1 ? 0.01 : 1} value={Number((control.value[0] * fxDisplayScale(control, props.stage)).toFixed(3))} disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value) / fxDisplayScale(control, props.stage))} /></div></Match>
          </Switch>
          <Show when={control.kind !== "Event"}><div class="videoFxParameterFoot"><Show when={control.kind !== "Color"} fallback={<span />}><span><span>Range</span>: <span data-no-localize>{fxControlValueText(control, props.stage, control.minimum)} – {fxControlValueText(control, props.stage, control.maximum)}</span></span></Show><button type="button" disabled={disabled()} onClick={() => void props.onSetControl(control.name, boundedFxValue(control, control.default))}>Reset default</button></div></Show>
        </div>
      )}</For>
      <div class="videoFxParameterActions">
        <button type="button" data-fx-randomize disabled={disabled() || controls.every((control) => control.kind === "Event")} onClick={() => void apply(randomizeFxStage(props.stage))}>Randomize</button>
        <button type="button" data-fx-undo disabled={disabled() || !canUndo()} onClick={() => {
          if (canUndo()) void run(() => props.history ? props.history.undo() : props.liveSession!.undo());
        }}>Undo FX change</button>
      </div>
      <details class="videoFxPresets" data-fx-presets>
        <summary>Effect presets</summary>
        <div class="videoFxPresetBody">
          <label><span>Preset</span><select aria-label="Effect preset" value={presetId()} onChange={(event) => setPresetId(event.currentTarget.value)}>
            <option value="">Choose preset</option>
            <For each={builtins()}>{(preset, index) => <option data-no-localize value={`builtin:${index()}`}>{translateUiText(preset.label, props.locale === "ja" ? "ja" : "en")}</option>}</For>
            <For each={presets()}>{(preset) => <option data-no-localize value={preset.id}>{preset.label}</option>}</For>
          </select></label>
          <div class="videoFxParameterActions">
            <button type="button" data-fx-preset-recall disabled={disabled() || !presetId()} onClick={recall}>Recall preset</button>
            <button type="button" data-fx-preset-delete disabled={disabled() || !presetId() || presetId().startsWith("builtin:") || !props.onApplyEffectCatalog} onClick={() => void run(async () => {
              if (!props.effectCatalog || !props.onApplyEffectCatalog) return;
              const result = await props.onApplyEffectCatalog({ ...props.effectCatalog, effect_presets: props.effectCatalog.effect_presets.filter((preset) => String(preset.id) !== presetId()) });
              if (result === null) throw new Error("FX preset mutation rejected");
              setPresetId("");
            })}>Delete preset</button>
          </div>
          <label><span>Preset name</span><input type="text" aria-label="Preset name" data-no-localize value={name()} maxLength={120} onInput={(event) => setName(event.currentTarget.value)} /></label>
          <button type="button" data-fx-preset-save disabled={disabled() || !name().trim() || !props.effectCatalog || !props.onApplyEffectCatalog} onClick={() => void run(async () => {
            if (!props.effectCatalog || !props.onApplyEffectCatalog) return;
            const result = await props.onApplyEffectCatalog(saveFxStagePreset(props.effectCatalog, props.stage, name()));
            if (result === null) throw new Error("FX preset mutation rejected");
            setName("");
          })}>Save preset in project</button>
        </div>
      </details>
      <Show when={error()}><p class="inlineError" role="alert">FX change failed. Check project status and try again.</p></Show>
      <Show when={!controls.length}><p class="emptyHint">This shader has no operator controls.</p></Show>
    </div>
  );
}
