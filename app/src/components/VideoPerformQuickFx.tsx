import { createEffect, createMemo, createSignal, For, on, Show, type ComponentProps } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type { VideoEffectCatalog, VideoIsfControlSummary, VideoIsfEffectStageSummary, VideoLayerSummary } from "../types";
import { cloneFxStage, fxControlValueText, replaceFxStage } from "../fxParameterModel";
import { moveFxStageTo } from "../fxAuditionModel";
import { createFxStageReorder } from "../createFxStageReorder";
import { controlMappingTargetData } from "../controlMappingLearn";
import { isfStageIdForIndex } from "../performControlDispatcher";
import { VideoFxParameterEditor } from "./VideoFxParameterEditor";
import { VideoFxLiveSafety } from "./VideoFxLiveSafety";
import { VideoFxAuditionBar } from "./VideoFxAuditionBar";
import type { FxAuditionSession } from "../createFxAuditionSession";
import type { FxLiveSession } from "../createFxLiveSession";
import { VideoLayerListPanel } from "./VideoLayerListPanel";
import "./VideoPerformQuickFx.css";

interface VideoPerformQuickFxProps {
  projectEpoch?: number;
  locale?: string;
  bpm?: number;
  layer: VideoLayerSummary | null;
  layerList: ComponentProps<typeof VideoLayerListPanel>;
  onOpenEditor: () => void;
  effectCatalog?: VideoEffectCatalog;
  onApplyEffectCatalog?: (catalog: VideoEffectCatalog) => unknown | Promise<unknown>;
  liveSession?: FxLiveSession;
  audition?: FxAuditionSession;
}

interface QuickStage {
  id: string;
  layerId: number;
  index: number;
  label: string;
  enabled: boolean;
  /** Stable effect-stage id from the layer's catalog chain; MIDI targets use it, never the index. */
  stageId: number | null;
  stage: VideoIsfEffectStageSummary;
  controls: (VideoIsfControlSummary & { id: string })[];
}

/** Value readout for a control. ISF summaries carry no unit metadata, so the only
 * unit shown is the percent sign of a normalised 0-1 float. */
export const quickFxValueText = (control: VideoIsfControlSummary): string => fxControlValueText(control);

/** A view of the existing layer-scoped ISF stack, with no separate FX state or
 * publication path. Stable stage/control stores retain focused DOM across polls. */
export function VideoPerformQuickFx(props: VideoPerformQuickFxProps) {
  const [stages, setStages] = createStore<QuickStage[]>([]);
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  const [removeIndex, setRemoveIndex] = createSignal<number | null>(null);
  // While an audition is open the stages, parameters and edits belong to the
  // Preview-only draft; every other time they are the layer's live chain.
  const audition = () => props.audition?.active(props.layer?.id) ?? null;
  const shownEffect = createMemo(() => audition() ? audition()!.draft : props.layer?.isf_effect);
  const effectSignature = createMemo(() => JSON.stringify([props.projectEpoch, props.layer?.id, shownEffect()]));
  createEffect(on(effectSignature, () => setRemoveIndex(null), { defer: true }));
  createEffect(() => {
    const layer = props.layer;
    const effect = shownEffect();
    // The store reconciles into the objects it holds, so it must never hold the layer's own snapshot
    // objects: a draft shown in their place would be written into the live chain.
    const next = layer && effect ? [effect, ...(effect.stack ?? [])].map((source, index) => {
      const stage = cloneFxStage(source);
      return {
        id: `${layer.id}:${index}`,
        layerId: layer.id,
        index,
        label: stage.label,
        enabled: stage.enabled,
        stageId: isfStageIdForIndex(props.effectCatalog, layer.id, index),
        stage,
        // The shader's declared order supplies its primary operator controls.
        controls: stage.controls.map((control) => ({ ...control, id: control.name })),
      };
    }) : [];
    setStages(reconcile(next, { key: "id" }));
  });
  // A different layer starts at its first stage; a shorter chain clamps.
  const layerId = createMemo(() => props.layer?.id ?? null);
  createEffect(on(layerId, () => { setSelectedIndex(0); setRemoveIndex(null); }, { defer: true }));
  const activeStage = createMemo(() => stages[Math.min(selectedIndex(), Math.max(0, stages.length - 1))] ?? null);

  const liveBusy = () => typeof props.layerList.isfEventPulseBusy === "function"
    ? props.layerList.isfEventPulseBusy()
    : Boolean(props.layerList.isfEventPulseBusy);
  const busy = () => audition() ? Boolean(props.audition?.busy()) : liveBusy();
  // One set of stage commands: the draft while auditioning, the ticketed live
  // commands otherwise.
  const setEnabled = (layerId: number, index: number, enabled: boolean) =>
    audition() ? props.audition!.setEnabled(layerId, index, enabled) : props.layerList.onSetIsfEffectEnabled(layerId, index, enabled);
  const moveStage = (layerId: number, index: number, delta: -1 | 1) =>
    audition() ? props.audition!.moveStage(layerId, index, delta) : props.layerList.onMoveIsfEffect(layerId, index, delta);
  const removeStage = (layerId: number, index: number) =>
    audition() ? props.audition!.removeStage(layerId, index) : props.layerList.onRemoveIsfEffect(layerId, index);
  // Drag-and-drop reorder. A neighbour swap is exactly the Up/Down command; a longer move is
  // one chain write through the same ticketed command that every stage edit uses (and one Undo).
  const reorder = createFxStageReorder({
    layerId: () => props.layer?.id ?? null,
    count: () => stages.length,
    disabled: () => busy(),
    onReorder: (layerId, from, to) => {
      setRemoveIndex(null);
      setSelectedIndex(to);
      if (Math.abs(to - from) === 1) { void moveStage(layerId, from, to > from ? 1 : -1); return; }
      const effect = shownEffect();
      if (!effect) return;
      const next = moveFxStageTo(effect, from, to);
      void (audition() ? props.audition!.replaceChain(layerId, next) : props.layerList.onSetIsfEffect(layerId, next));
    },
  });
  const history = () => audition()
    ? { canUndo: () => props.audition!.canUndo(), undo: () => props.audition!.undo() }
    : undefined;
  return (
    <section class="videoPerformQuickFx" data-perform-quick-fx data-video-perform-region="quick-fx" data-fx-auditioning={audition() ? "true" : undefined} aria-label="Selected layer effect chain">
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
        <div class="videoPerformQuickFxChain" role="list" aria-label="Selected layer effect chain">
          <Show when={stages.length > 0} fallback={
            <p class="emptyState">{props.layer ? "No layer FX. Add effects in the full editor." : "Select a layer for Quick FX."}</p>
          }>
            <For each={stages}>
              {(stage) => (
                <div class="videoPerformQuickFxStage" data-perform-fx-stage={stage.index} data-perform-fx-layer={stage.layerId}
                  data-fx-enabled={stage.enabled} data-fx-selected={activeStage()?.id === stage.id} role="listitem"
                  data-fx-draggable={reorder.canDrag() ? "true" : undefined} draggable={reorder.canDrag()}
                  title={reorder.canDrag() ? "Drag to reorder, or use Up and Down." : undefined}
                  data-fx-dragging={reorder.dragging() === stage.index ? "true" : undefined} data-fx-drop={reorder.dropEdge(stage.index)}
                  {...reorder.handlers(() => stage.index)}>
                  <button type="button" class="videoPerformQuickFxSelect" data-perform-fx-select
                    aria-pressed={activeStage()?.id === stage.id} onClick={() => setSelectedIndex(stage.index)}>
                    <span class="videoPerformQuickFxNumber" aria-hidden="true">{stage.index + 1}</span>
                    <strong data-no-localize title={stage.label}>{stage.label}</strong>
                  </button>
                  <button type="button" class="videoPerformQuickFxSwitch" data-perform-fx-bypass aria-pressed={stage.enabled}
                    aria-label={`${stage.label} FX enabled for ${props.layer?.label ?? "Video layer"} (layer ${stage.layerId})`}
                    classList={{ active: stage.enabled }} disabled={busy()}
                    {...controlMappingTargetData(!audition() && stage.stageId !== null
                      ? { action: "VideoIsfStageEnabled", label: `${props.layer?.label ?? "Layer"} FX ${stage.index + 1}`, layer_id: stage.layerId, isf_stage_id: stage.stageId }
                      : null)}
                    onClick={() => void setEnabled(stage.layerId, stage.index, !stage.enabled)}>
                    <span class="videoPerformQuickFxKnob" aria-hidden="true" />
                    <span>{stage.enabled ? "On" : "Off"}</span>
                  </button>
                </div>
              )}
            </For>
          </Show>
          <Show when={activeStage()}>{(stage) => <div class="videoPerformQuickFxActions">
            <button type="button" disabled={busy() || stage().index === 0} onClick={() => {
              const index = stage().index; void moveStage(stage().layerId, index, -1); setSelectedIndex(index - 1); setRemoveIndex(null);
            }}>Up</button>
            <button type="button" disabled={busy() || stage().index >= stages.length - 1} onClick={() => {
              const index = stage().index; void moveStage(stage().layerId, index, 1); setSelectedIndex(index + 1); setRemoveIndex(null);
            }}>Down</button>
            <Show when={removeIndex() === stage().index} fallback={<button type="button" disabled={busy()} onClick={() => audition() ? void removeStage(stage().layerId, stage().index) : setRemoveIndex(stage().index)}>Remove</button>}>
              <button type="button" class="danger" disabled={busy()} onClick={() => { void removeStage(stage().layerId, stage().index); setRemoveIndex(null); }}>Confirm remove</button>
              <button type="button" onClick={() => setRemoveIndex(null)}>Cancel</button>
            </Show>
          </div>}</Show>
          <Show when={props.audition} fallback={null}>
            <VideoFxAuditionBar session={props.audition} layerId={props.layer?.id ?? null} locale={props.locale} />
          </Show>
          <Show when={!audition()}>
            <VideoFxLiveSafety session={props.liveSession} layerId={props.layer?.id ?? null} />
          </Show>
        </div>
        <div class="videoPerformQuickFxParameters" data-perform-fx-detail={activeStage()?.index ?? ""} aria-label="Selected stage parameters">
          <Show when={activeStage()}>
            {(stage) => (
              <VideoFxParameterEditor availableLayers={props.layerList.layers} stage={stage().stage} identity={stage().id} busy={busy()} locale={props.locale} bpm={props.bpm}
                projectEpoch={props.projectEpoch} layerId={stage().layerId} liveSession={props.liveSession} history={history()}
                auditioning={Boolean(audition())} stageId={stage().stageId}
                effectCatalog={props.effectCatalog} onApplyEffectCatalog={props.onApplyEffectCatalog}
                onSetStage={(replacement) => {
                  const effect = shownEffect();
                  if (!effect) return;
                  const next = replaceFxStage(effect, stage().index, replacement);
                  return audition() ? props.audition!.replaceChain(stage().layerId, next) : props.layerList.onSetIsfEffect(stage().layerId, next);
                }}
                onSetControl={(name, value) => audition()
                  ? props.audition!.setControl(stage().layerId, stage().index, name, value)
                  : props.layerList.onSetIsfControl(stage().layerId, stage().index, name, value)}
                onTriggerEvent={(name) => audition() ? undefined : props.layerList.onTriggerIsfEvent(stage().layerId, stage().index, name)} />
            )}
          </Show>
        </div>
      </div>
    </section>
  );
}
