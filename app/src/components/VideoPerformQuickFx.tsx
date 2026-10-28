import { createEffect, createMemo, createSignal, createUniqueId, For, on, onCleanup, Show, type ComponentProps } from "solid-js";
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
  /** Opens the neighbouring effect Browser tab, so an empty layer gains effects inside Perform. */
  onOpenBrowser?: () => void;
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
  type FxView = "parameters" | "chain";
  const viewIds: FxView[] = ["parameters", "chain"];
  const [view, setView] = createSignal<FxView>("parameters");
  const [compact, setCompact] = createSignal(false);
  const viewId = createUniqueId();
  let root: HTMLElement | undefined;
  let body: HTMLDivElement | undefined;
  let safety: HTMLDivElement | undefined;
  let sizeObserver: ResizeObserver | undefined;
  const scrollPositions: Record<FxView, number> = { parameters: 0, chain: 0 };
  const changeView = (next: FxView) => {
    if (next === view()) return;
    if (body) scrollPositions[view()] = body.scrollTop;
    setView(next);
    queueMicrotask(() => { if (body) body.scrollTop = scrollPositions[next]; });
  };
  const showSafety = () => {
    if (!body || !safety) return;
    body.scrollTop += safety.getBoundingClientRect().top - body.getBoundingClientRect().top;
    safety.querySelector<HTMLElement>("button:not(:disabled), summary")?.focus({ preventScroll: true });
  };
  const attachRoot = (element: HTMLElement) => {
    root = element;
    sizeObserver?.disconnect();
    sizeObserver = new ResizeObserver(() => {
      const next = element.clientWidth < 640;
      if (next === compact()) return;
      const focused = document.activeElement instanceof HTMLElement && element.contains(document.activeElement) ? document.activeElement : null;
      const focusedView = focused?.closest<HTMLElement>("[data-perform-fx-view]")?.dataset.performFxView;
      const focusedPicker = Boolean(focused?.closest("[data-perform-fx-stage-picker]"));
      const focusedBypass = Boolean(focused?.closest("[data-perform-fx-selected-bypass]"));
      const stageIndex = activeStage()?.index;
      // A resize must not hide a focused editor or management control.
      if (next && focused?.closest('[data-perform-fx-panel="chain"]')) setView("chain");
      else if (next && focused?.closest('[data-perform-fx-panel="parameters"]')) setView("parameters");
      setCompact(next);
      queueMicrotask(() => {
        if (!focused || !body || !element.isConnected) return;
        const current = document.activeElement;
        if (current !== focused && current !== document.body && current !== document.documentElement && current !== null) return;
        const visible = (control: HTMLElement | null | undefined) => {
          if (!control?.isConnected || !control.getClientRects().length || ("disabled" in control && control.disabled)) return false;
          const bounds = control.getBoundingClientRect();
          return bounds.width > 0 && bounds.height > 0;
        };
        let target = focused;
        if (!next && (focusedView || focusedPicker || focusedBypass)) {
          const stage = element.querySelector<HTMLElement>(`[data-perform-fx-stage="${stageIndex}"]`);
          const controls = focusedView === "parameters"
            ? [...element.querySelectorAll<HTMLElement>('[data-perform-fx-panel="parameters"] [data-perform-fx-control] :is(input, select, button)')]
            : [];
          const replacement = [
            ...(focusedBypass ? [stage?.querySelector<HTMLElement>("[data-perform-fx-bypass]")] : []),
            ...controls, stage?.querySelector<HTMLElement>("[data-perform-fx-select]"),
            element.querySelector<HTMLElement>("[data-perform-fx-editor]"), element.querySelector<HTMLElement>("[data-perform-fx-scope]"),
          ].find(visible);
          if (!replacement) return;
          replacement.focus({ preventScroll: true });
          target = replacement;
        }
        if (!visible(target) || !body.contains(target)) return;
        const viewport = body.getBoundingClientRect();
        const bounds = target.getBoundingClientRect();
        if (bounds.top < viewport.top) body.scrollTop -= viewport.top - bounds.top;
        else if (bounds.bottom > viewport.bottom) body.scrollTop += bounds.bottom - viewport.bottom;
      });
    });
    sizeObserver.observe(element);
  };
  onCleanup(() => sizeObserver?.disconnect());
  const viewKeyDown = (event: KeyboardEvent, current: FxView) => {
    if (!["ArrowLeft", "ArrowRight", "Home", "End"].includes(event.key)) return;
    event.preventDefault(); event.stopPropagation();
    const index = viewIds.indexOf(current);
    const next = event.key === "Home" ? 0 : event.key === "End" ? viewIds.length - 1 : (index + (event.key === "ArrowRight" ? 1 : -1) + viewIds.length) % viewIds.length;
    changeView(viewIds[next]);
    root?.querySelector<HTMLButtonElement>(`[data-perform-fx-view="${viewIds[next]}"]`)?.focus();
  };
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
    <section ref={attachRoot} class="videoPerformQuickFx" data-perform-quick-fx data-video-perform-region="quick-fx" data-fx-compact={compact()} data-fx-auditioning={audition() ? "true" : undefined} aria-label="Selected layer effect chain">
      <header class="videoPerformQuickFxHeader">
        <div class="videoPerformQuickFxLayer" hidden={compact() && stages.length > 0}>
          <span class="videoPerformQuickFxLayerName" data-no-localize title={props.layer?.label ?? ""}>{props.layer?.label ?? ""}</span>
          <Show when={stages.length > 0}>
            <span class="videoPerformQuickFxCount"><span data-no-localize>{stages.length}</span> <span>{stages.length === 1 ? "stage" : "stages"}</span></span>
          </Show>
        </div>
        <div class="videoPerformQuickFxViews" role="tablist" aria-label="Effect views" hidden={!compact() || stages.length === 0}>
          <For each={viewIds}>{item => <button type="button" role="tab" data-perform-fx-view={item}
            id={`${viewId}-${item}-tab`} aria-controls={`${viewId}-${item}-panel`} aria-selected={view() === item}
            tabIndex={view() === item ? 0 : -1} onClick={() => changeView(item)} onKeyDown={event => viewKeyDown(event, item)}>
            {item === "parameters" ? "Parameters" : "Chain"}
          </button>}</For>
        </div>
        <button type="button" class="videoPerformQuickFxScope" data-perform-fx-scope={audition() ? "preview" : "live"}
          aria-controls={`${viewId}-safety`} aria-label="FX output safety" onClick={showSafety}
          title={audition() ? "AUDITION · PREVIEW ONLY" : "LIVE — affects Program"}>{audition() ? "PREVIEW ONLY" : "LIVE"}</button>
        <button type="button" data-perform-fx-editor data-video-perform-fx-editor disabled={!props.layer} onClick={props.onOpenEditor}>
          Full FX editor
        </button>
      </header>
      <div ref={body} class="videoPerformQuickFxBody" data-perform-fx-scroll data-fx-empty={stages.length === 0}>
        <div class="videoPerformQuickFxChain" data-perform-fx-panel="chain" id={`${viewId}-chain-panel`}
          role={compact() ? "tabpanel" : undefined} aria-label="Selected layer effect chain" aria-labelledby={compact() ? `${viewId}-chain-tab` : undefined}
          hidden={compact() && stages.length > 0 && view() !== "chain"}>
          <div class="videoPerformQuickFxChainContext" hidden={!compact()}><strong data-no-localize>{props.layer?.label ?? ""}</strong><span><span data-no-localize>{stages.length}</span> <span>{stages.length === 1 ? "stage" : "stages"}</span></span></div>
          <div class="videoPerformQuickFxStages" role="list" aria-label="Selected layer effect chain"><Show when={stages.length > 0} fallback={
            <Show when={props.layer && props.onOpenBrowser} fallback={
              <p class="emptyState">{props.layer ? "No layer FX. Add effects in the full editor." : "Select a layer for Quick FX."}</p>
            }>{openBrowser => <div class="emptyState videoPerformQuickFxEmpty">
              <p>No layer FX yet.</p>
              <button type="button" class="primary" data-perform-fx-add-effect onClick={() => openBrowser()()}>Add effect</button>
            </div>}</Show>
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
          </Show></div>
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
        </div>
        <div class="videoPerformQuickFxParameters" data-perform-fx-panel="parameters" id={`${viewId}-parameters-panel`}
          role={compact() ? "tabpanel" : undefined} aria-labelledby={compact() ? `${viewId}-parameters-tab` : undefined}
          hidden={compact() && view() !== "parameters"} data-perform-fx-detail={activeStage()?.index ?? ""} aria-label="Selected stage parameters">
          <div class="videoPerformQuickFxStagePicker" hidden={!compact()}>
            <select data-perform-fx-stage-picker aria-label="Selected stage" title={activeStage()?.label ?? ""} value={activeStage()?.index ?? 0}
              onChange={event => setSelectedIndex(Number(event.currentTarget.value))}>
              <For each={stages}>{stage => <option value={stage.index} data-no-localize>{stage.index + 1} · {stage.label}</option>}</For>
            </select>
            <Show when={activeStage()}>{stage => <button type="button" class="videoPerformQuickFxSwitch" data-perform-fx-selected-bypass
              aria-pressed={stage().enabled} aria-label={`${stage().label} FX enabled for ${props.layer?.label ?? "Video layer"} (layer ${stage().layerId})`}
              classList={{ active: stage().enabled }} disabled={busy()}
              {...controlMappingTargetData(!audition() && stage().stageId !== null
                ? { action: "VideoIsfStageEnabled", label: `${props.layer?.label ?? "Layer"} FX ${stage().index + 1}`, layer_id: stage().layerId, isf_stage_id: stage().stageId }
                : null)}
              onClick={() => void setEnabled(stage().layerId, stage().index, !stage().enabled)}>
              <span class="videoPerformQuickFxKnob" aria-hidden="true" /><span>{stage().enabled ? "On" : "Off"}</span>
            </button>}</Show>
          </div>
          <Show when={activeStage()}>
            {(stage) => (
              <VideoFxParameterEditor availableLayers={props.layerList.layers} stage={stage().stage} identity={stage().id} busy={busy()} locale={props.locale} bpm={props.bpm} discloseImageInputs
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
        <div ref={safety} class="videoPerformQuickFxSafety" data-perform-fx-safety id={`${viewId}-safety`}>
          <Show when={props.audition} fallback={null}>
            <VideoFxAuditionBar session={props.audition} layerId={props.layer?.id ?? null} locale={props.locale} />
          </Show>
          <Show when={!audition()}>
            <VideoFxLiveSafety session={props.liveSession} layerId={props.layer?.id ?? null} />
          </Show>
        </div>
      </div>
    </section>
  );
}
