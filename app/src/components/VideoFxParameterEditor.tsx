import { createEffect, createMemo, createSignal, For, Match, Show, Switch } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { VideoEffectCatalog, VideoIsfControlSummary, VideoIsfEffectStageSummary, VideoIsfTempoSync, VideoIsfTempoSyncUnit, VideoLayerSummary } from "../types";
import { boundedFxValue, builtinFxStagePresets, effectiveFxValue, FX_TEMPO_SYNC_COUNTS, fxControlLabel, fxControlValueText, fxDisplayScale, fxIsTimeControl, fxTempoSyncCountText, fxTempoSyncLabel, fxTempoSyncState, fxValidBpm, matchingFxStagePresets, randomizeFxStage, resyncFxStage, saveFxStagePreset, withFxTempoSync, type FxValue } from "../fxParameterModel";
import "./VideoFxParameterEditor.css";
import { VideoFxPointPad } from "./VideoFxPointPad";
import { VideoFxImageInputs } from "./VideoFxImageInputs";
import { fxImageInputs } from "../fxImageInputModel";
import { translateUiText } from "../uiLocalization";
import type { FxLiveSession } from "../createFxLiveSession";
import { controlMappingTargetData, type ControlMappingTarget } from "../controlMappingLearn";

interface VideoFxParameterEditorProps {
  projectEpoch?: number;
  layerId: number;
  availableLayers?: readonly VideoLayerSummary[];
  /** Stable effect-stage id. With it, Float/Long/Bool/Event controls are MIDI-learnable
   * (never while auditioning, when the controls edit a Preview-only draft). */
  stageId?: number | null;
  liveSession?: FxLiveSession;
  /** Overrides the live Undo, for example with a Preview audition's own history. */
  history?: { canUndo: () => boolean; undo: () => unknown | Promise<unknown> };
  /** Event controls fire on Program, so an audition cannot offer them. */
  auditioning?: boolean;
  locale?: string;
  /** Global tempo (`snapshot.clock.bpm`) that beat and bar units are derived from. */
  bpm?: number;
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
  const identity = createMemo(() => [props.projectEpoch, props.identity, props.stageId ?? "", props.stage.source_path ?? "", props.stage.source].join(":"));
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
  const applyImageStage = async (stage: VideoIsfEffectStageSummary) => {
    if (disabled()) return null;
    setPending(true);
    try { return await props.onSetStage(stage); }
    finally { setPending(false); }
  };
  // Range/color widgets commit on native change (gesture completion). Publishing
  // every pointer move would race the authoritative single-flight FX command.
  const setComponent = (control: VideoIsfControlSummary, component: number, value: number) => {
    const next = [...control.value] as FxValue; next[component] = value;
    void props.onSetControl(control.name, boundedFxValue(control, next));
  };
  const mapTarget = (control: VideoIsfControlSummary): ControlMappingTarget | null =>
    props.stageId === null || props.stageId === undefined || props.auditioning
      || !(control.kind === "Float" || control.kind === "Long" || control.kind === "Bool" || control.kind === "Event")
      ? null
      : {
        action: "VideoIsfParam",
        label: `Layer ${props.layerId} ${fxControlLabel(control, props.stage.source)}`,
        layer_id: props.layerId,
        isf_stage_id: props.stageId,
        attribute: control.name,
        audio_disabled: control.kind === "Event",
      };
  const colorHex = (control: VideoIsfControlSummary) => `#${control.value.slice(0, 3).map((value) =>
    Math.round(Math.max(0, Math.min(1, value)) * 255).toString(16).padStart(2, "0")).join("")}`;
  const recall = () => {
    const id = presetId();
    const stage = id.startsWith("builtin:") ? builtins()[Number(id.slice(8))]?.stage
      : presets().find((preset) => String(preset.id) === id)?.payload.stages[0].effect.effect;
    // A preset saved with a tempo sync follows today's tempo, not the tempo it was saved at.
    if (stage) void apply(resyncFxStage({ ...stage, enabled: props.stage.enabled }, props.bpm));
  };
  const ja = () => props.locale === "ja";
  const effectiveValueText = (control: VideoIsfControlSummary) =>
    control.kind === "Float" && control.tempo_sync
      ? `${Number(effectiveFxValue(control, props.bpm)[0].toFixed(3))} Hz`
      : fxControlValueText(control, props.stage);
  const setSync = (control: VideoIsfControlSummary, sync: VideoIsfTempoSync | null) =>
    apply({ ...props.stage, controls: props.stage.controls.map((candidate) =>
      candidate.name === control.name ? withFxTempoSync(candidate, sync, props.bpm) : candidate) });
  const resetControl = (control: VideoIsfControlSummary) => {
    // A raw default replaces tempo sync so the default remains authoritative.
    if (control.tempo_sync) return void apply({ ...props.stage, controls: props.stage.controls.map((candidate) =>
      candidate.name === control.name ? { ...withFxTempoSync(candidate, null, props.bpm), value: boundedFxValue(candidate, candidate.default) } : candidate) });
    return void props.onSetControl(control.name, boundedFxValue(control, control.default));
  };
  const syncReadout = (control: VideoIsfControlSummary) => {
    const sync = control.tempo_sync;
    if (!sync) return "";
    const state = fxTempoSyncState(control, props.bpm);
    if (!state) {
      const hz = `${Number(effectiveFxValue(control, props.bpm)[0].toFixed(3))} Hz`;
      const reason = translateUiText("Tempo unavailable; using the saved rate.", ja() ? "ja" : "en");
      return `${fxTempoSyncLabel(sync, props.locale)} · ${hz} · ${reason}`;
    }
    const hz = `${Number(state.target.toFixed(3))} Hz`;
    const bpm = `${Number(props.bpm!.toFixed(1))} BPM`;
    return ja() ? `${fxTempoSyncLabel(sync, "ja")}ごとに1回 ＝ ${hz}（${bpm}）` : `One cycle per ${fxTempoSyncLabel(sync)} = ${hz} at ${bpm}`;
  };
  return (
    <div class="videoFxParameterEditor" data-fx-parameter-editor>
      <VideoFxImageInputs identity={identity()} layerId={props.layerId} stage={props.stage}
        layers={props.availableLayers} catalog={props.effectCatalog} locale={props.locale}
        disabled={disabled()} onSetStage={applyImageStage} />
      <For each={controls}>{(control) => (
        <div class="videoFxParameter" data-perform-fx-control={control.name} data-perform-fx-kind={control.kind}
          data-video-isf-control-kind={control.kind} data-video-isf-control-name={control.name}>
          <div class="videoFxParameterHead">
            <label title={control.name}>{fxControlLabel(control, props.stage.source)}</label>
            <output>{effectiveValueText(control)}</output>
          </div>
          <Switch>
            <Match when={control.kind === "Event"}><button type="button" disabled={disabled() || props.auditioning} {...controlMappingTargetData(mapTarget(control))} title={props.auditioning ? "Event controls fire on Program, so they are off during an audition." : undefined} aria-label={fxControlLabel(control, props.stage.source)} onClick={() => void props.onTriggerEvent(control.name)}>Trigger</button></Match>
            <Match when={control.kind === "Bool"}><button type="button" disabled={disabled()} {...controlMappingTargetData(mapTarget(control))} aria-label={fxControlLabel(control, props.stage.source)} aria-pressed={control.value[0] >= 0.5} onClick={() => setComponent(control, 0, control.value[0] >= 0.5 ? 0 : 1)}>{control.value[0] >= 0.5 ? "On" : "Off"}</button></Match>
            <Match when={control.kind === "Color"}>
              <div class="videoFxColorPicker"><input type="color" disabled={disabled()} aria-label={fxControlLabel(control, props.stage.source)} value={colorHex(control)} onChange={(event) => {
                const hex = event.currentTarget.value;
                void props.onSetControl(control.name, boundedFxValue(control, [parseInt(hex.slice(1, 3), 16) / 255, parseInt(hex.slice(3, 5), 16) / 255, parseInt(hex.slice(5, 7), 16) / 255, control.value[3]]));
              }} /><label><span>Alpha</span><input type="range" min={control.minimum[3]} max={control.maximum[3]} step="any" value={control.value[3]} disabled={disabled()} aria-label="Alpha" onChange={(event) => setComponent(control, 3, Number(event.currentTarget.value))} /></label></div>
            </Match>
            <Match when={control.kind === "Long" && control.values.length > 0}><select disabled={disabled()} {...controlMappingTargetData(mapTarget(control))} aria-label={fxControlLabel(control, props.stage.source)} value={control.value[0]} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value))}><For each={control.values}>{(value, index) => <option value={value}>{control.labels[index()] ?? value}</option>}</For></select></Match>
            <Match when={control.kind === "Point2d"}>
              <div class="videoFxPointPicker" role="group" aria-label={fxControlLabel(control, props.stage.source)}>
                <VideoFxPointPad control={control} label={fxControlLabel(control, props.stage.source)} disabled={disabled()}
                  onCommit={(x, y) => void props.onSetControl(control.name, boundedFxValue(control, [x, y, control.value[2], control.value[3]]))} />
                <For each={[0, 1]}>{(component) => <label><span>{component === 0 ? "X" : "Y"}</span><input type="range" min={control.minimum[component]} max={control.maximum[component]} step="any" value={control.value[component]} disabled={disabled()} aria-label={`${fxControlLabel(control, props.stage.source)} ${component === 0 ? "X" : "Y"}`} onChange={(event) => setComponent(control, component, Number(event.currentTarget.value))} /><input type="number" min={control.minimum[component]} max={control.maximum[component]} step="0.01" value={control.value[component]} disabled={disabled()} aria-label={`${fxControlLabel(control, props.stage.source)} ${component === 0 ? "X" : "Y"}`} onChange={(event) => setComponent(control, component, Number(event.currentTarget.value))} /></label>}</For>
              </div>
            </Match>
            <Match when={control.kind === "Float" || control.kind === "Long"}><div class="videoFxScalarPicker"><input type="range" {...controlMappingTargetData(mapTarget(control))} min={Math.min(control.minimum[0], control.maximum[0])} max={Math.max(control.minimum[0], control.maximum[0])} step={control.kind === "Long" ? 1 : "any"} value={effectiveFxValue(control, props.bpm)[0]} aria-valuetext={effectiveValueText(control)} disabled={disabled() || Boolean(control.tempo_sync)} aria-label={fxControlLabel(control, props.stage.source)} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value))} /><input type="number" min={Number((Math.min(control.minimum[0], control.maximum[0]) * fxDisplayScale(control, props.stage)).toFixed(3))} max={Number((Math.max(control.minimum[0], control.maximum[0]) * fxDisplayScale(control, props.stage)).toFixed(3))} step={control.kind === "Long" ? 1 : fxDisplayScale(control, props.stage) === 1 ? 0.01 : 1} value={Number((effectiveFxValue(control, props.bpm)[0] * fxDisplayScale(control, props.stage)).toFixed(3))} disabled={disabled() || Boolean(control.tempo_sync)} aria-label={fxControlLabel(control, props.stage.source)} onChange={(event) => setComponent(control, 0, Number(event.currentTarget.value) / fxDisplayScale(control, props.stage))} /></div></Match>
          </Switch>
          <Show when={fxIsTimeControl(control, props.stage)}>
            <div class="videoFxTempoSync" data-fx-tempo-sync={control.tempo_sync ? control.tempo_sync.unit : "hz"} data-fx-tempo-control={control.name}>
              <div class="videoFxTempoUnits" role="group" aria-label="Time unit">
                <button type="button" data-fx-tempo-unit="hz" aria-pressed={!control.tempo_sync} disabled={disabled()} title="Raw value in cycles per second" onClick={() => { if (control.tempo_sync) void setSync(control, null); }}>Hz</button>
                <For each={["beats", "bars"] as VideoIsfTempoSyncUnit[]}>{(unit) => (
                  <button type="button" data-fx-tempo-unit={unit} aria-pressed={control.tempo_sync?.unit === unit}
                    disabled={disabled() || !fxValidBpm(props.bpm)} title={fxValidBpm(props.bpm) ? "Follow the global tempo" : "Tempo unavailable"}
                    onClick={() => { if (control.tempo_sync?.unit !== unit) void setSync(control, { unit, count: control.tempo_sync?.count ?? 1 }); }}>{unit === "beats" ? "Beats" : "Bars"}</button>
                )}</For>
                <Show when={control.tempo_sync}>{(sync) => (
                  <select data-fx-tempo-count aria-label={sync().unit === "bars" ? "Bars per cycle" : "Beats per cycle"} disabled={disabled()} value={String(sync().count)}
                    onChange={(event) => void setSync(control, { unit: sync().unit, count: Number(event.currentTarget.value) })}>
                    <For each={FX_TEMPO_SYNC_COUNTS}>{(count) => <option data-no-localize value={String(count)} selected={sync().count === count}>{fxTempoSyncCountText(count)}</option>}</For>
                  </select>
                )}</Show>
              </div>
              <Show when={control.tempo_sync}>
                <p class="videoFxTempoReadout" data-fx-tempo-readout data-no-localize>{syncReadout(control)}</p>
                <Show when={fxTempoSyncState(control, props.bpm)}>{(state) => <>
                  <Show when={state().clamped}><p class="videoFxTempoNote" data-fx-tempo-clamped>Limited by this control's range.</p></Show>
                </>}</Show>
              </Show>
            </div>
          </Show>
          <Show when={control.kind !== "Event"}><div class="videoFxParameterFoot"><Show when={control.kind !== "Color"} fallback={<span />}><span><span>Range</span>: <span data-no-localize>{fxControlValueText(control, props.stage, control.minimum)} – {fxControlValueText(control, props.stage, control.maximum)}</span></span></Show><button type="button" disabled={disabled()} onClick={() => resetControl(control)}>Reset default</button></div></Show>
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
      <Show when={!controls.length && fxImageInputs(props.stage.source).length === 1}><p class="emptyHint">This shader has no operator controls.</p></Show>
    </div>
  );
}
