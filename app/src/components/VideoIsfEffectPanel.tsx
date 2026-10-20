import { createEffect, createMemo, createSignal, For, on, Show, type Accessor } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import type {
  VideoEffectCatalog,
  VideoIsfControlSummary,
  VideoIsfEffectStageSummary,
  VideoIsfEffectSummary,
  VideoIsfStageError,
  VideoLayerSummary,
} from "../types";
import { replaceFxStage } from "../fxParameterModel";
import { moveFxStageTo } from "../fxAuditionModel";
import { createFxStageReorder } from "../createFxStageReorder";
import { VideoFxParameterEditor } from "./VideoFxParameterEditor";
import { VideoFxLiveSafety } from "./VideoFxLiveSafety";
import type { FxLiveSession } from "../createFxLiveSession";
import { explainFxRejection } from "../fxRejectionExplanation";

interface VideoIsfEffectPanelProps {
  projectEpoch?: number;
  bpm?: number;
  locale?: string;
  effectCatalog?: VideoEffectCatalog;
  onApplyEffectCatalog?: (catalog: VideoEffectCatalog) => unknown | Promise<unknown>;
  liveSession?: FxLiveSession;
  layerId: number;
  availableLayers?: readonly VideoLayerSummary[];
  layerLabel?: string;
  compact?: boolean;
  effect?: VideoIsfEffectSummary | null;
  runtimeErrors?: VideoIsfStageError[];
  eventPulseBusy?: boolean | Accessor<boolean>;
  onImport: (layerId: number) => void | Promise<void>;
  onApplyBuiltin: (layerId: number, presetId: string) => void | Promise<void>;
  onSetEffect: (layerId: number, effect: VideoIsfEffectSummary | null) => void | Promise<void>;
  onMoveEffect: (layerId: number, stageIndex: number, delta: -1 | 1) => void | Promise<void>;
  onRemoveEffect: (layerId: number, stageIndex: number) => void | Promise<void>;
  onSetEffectEnabled: (
    layerId: number,
    stageIndex: number,
    enabled: boolean,
  ) => void | Promise<void>;
  onResetEffect: (layerId: number, stageIndex: number) => void | Promise<void>;
  onSetControl: (
    layerId: number,
    stageIndex: number,
    controlName: string,
    value: [number, number, number, number],
  ) => void | Promise<void>;
  onTriggerEvent: (
    layerId: number,
    stageIndex: number,
    controlName: string,
  ) => void | Promise<void>;
}

const BUILTIN_FX_GROUPS = [
  { label: "Color", effects: [["invert", "Invert"], ["monochrome", "Monochrome"], ["threshold", "Threshold"], ["posterize", "Posterize"], ["colorize", "Colorize"]] },
  { label: "Geometry", effects: [["mirror", "Mirror"], ["kaleidoscope", "Kaleidoscope"], ["zoom", "Zoom"], ["rotate", "Rotate"]] },
  { label: "Rhythm & Glitch", effects: [["rgb-split", "RGB Split"], ["strobe", "Strobe"], ["scanlines", "Scanlines"], ["vignette", "Vignette"], ["glitch-shift", "Glitch Shift"]] },
] as const;

const MAX_STACK_STAGES = 8;

const effectStages = (effect?: VideoIsfEffectSummary | null): VideoIsfEffectStageSummary[] => {
  if (!effect) return [];
  return [effect, ...(effect.stack ?? [])];
};

const runtimeErrorLabel = (error: VideoIsfStageError) =>
  error.stage_index === null || error.stage_index === undefined
    ? error.message
    : `FX ${error.stage_index + 1}${error.stage_label ? ` ${error.stage_label}` : ""}: ${error.message}`;

export function VideoIsfEffectPanel(props: VideoIsfEffectPanelProps) {
  let panelElement: HTMLElement | undefined;
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  const [selectedStageIndex, setSelectedStageIndex] = createSignal(0);
  const [removeConfirmation, setRemoveConfirmation] = createSignal<number | "all" | null>(null);
  const effectSignature = createMemo(() => JSON.stringify([props.projectEpoch, props.layerId, props.effect]));
  createEffect(on(effectSignature, () => setRemoveConfirmation(null), { defer: true }));
  const advancedId = `video-isf-advanced-${props.layerId}`;
  const [stageStore, setStageStore] = createStore<VideoIsfEffectStageSummary[]>([]);
  createEffect(() => setStageStore(reconcile(effectStages(props.effect), { key: null })));
  const stages = () => stageStore;
  const selectedStage = createMemo(() => stages()[selectedStageIndex()] ?? stages()[0]);
  const activeStageCount = createMemo(() => stages().filter((stage) => stage.enabled).length);
  const layerDescription = () => `${props.layerLabel ?? "Video layer"} (layer ${props.layerId})`;
  const stackFull = () => stages().length >= MAX_STACK_STAGES;
  const runtimeErrorText = () => (props.runtimeErrors ?? []).map(runtimeErrorLabel).join("; ");
  const RuntimeErrors = () => <For each={props.runtimeErrors ?? []}>{(error) => {
    const explanation = () => explainFxRejection(error.message, props.locale ?? "en");
    return <div class="inlineError" role="alert" data-fx-runtime-explanation>
      <strong data-no-localize>{explanation().title}</strong>
      <p data-no-localize>{explanation().detail} {explanation().action}</p>
      <Show when={error.stage_index != null}><span>FX</span> <span data-no-localize>{(error.stage_index ?? 0) + 1} · {error.stage_label}</span></Show>
      <details><summary>Technical details</summary><pre data-no-localize>{error.message}</pre></details>
    </div>;
  }}</For>;
  const eventPulseBusy = () => typeof props.eventPulseBusy === "function"
    ? props.eventPulseBusy()
    : Boolean(props.eventPulseBusy);

  createEffect(() => {
    const count = stages().length;
    if (count === 0) {
      setSelectedStageIndex(0);
    } else if (selectedStageIndex() >= count) {
      setSelectedStageIndex(count - 1);
    }
  });

  const restoreActionFocus = (stageIndex: number | null, action: string) => {
    window.requestAnimationFrame(() => {
      const scope = panelElement ?? document.querySelector(`[data-video-isf-layer-id="${props.layerId}"]`);
      const selector = stageIndex === null
        ? `[data-video-isf-action="${action}"]`
        : `[data-video-isf-stage-index="${stageIndex}"] [data-video-isf-action="${action}"]`;
      scope?.querySelector<HTMLButtonElement>(selector)?.focus();
    });
  };

  const moveStage = (stageIndex: number, delta: -1 | 1) => {
    const targetIndex = stageIndex + delta;
    setSelectedStageIndex((selectedIndex) => {
      if (selectedIndex === stageIndex) return targetIndex;
      if (selectedIndex === targetIndex) return stageIndex;
      return selectedIndex;
    });
    void Promise.resolve(props.onMoveEffect(props.layerId, stageIndex, delta)).then(() => {
      restoreActionFocus(targetIndex, delta < 0 ? "move-up" : "move-down");
    });
  };

  // Drag-and-drop reorder. A neighbour swap is exactly the Up/Down command; a longer move is
  // one chain write through the ticketed set-effect command (one Undo FX step).
  const reorder = createFxStageReorder({
    layerId: () => props.layerId,
    count: () => stages().length,
    disabled: () => eventPulseBusy(),
    onReorder: (_layerId, from, to) => {
      setRemoveConfirmation(null);
      setSelectedStageIndex(to);
      if (Math.abs(to - from) === 1) { void props.onMoveEffect(props.layerId, from, to > from ? 1 : -1); return; }
      if (props.effect) void props.onSetEffect(props.layerId, moveFxStageTo(props.effect, from, to) as VideoIsfEffectSummary);
    },
  });

  const removeStage = (stageIndex: number) => {
    const stageCount = stages().length;
    setSelectedStageIndex((selectedIndex) => {
      if (selectedIndex > stageIndex) return selectedIndex - 1;
      if (selectedIndex === stageIndex && stageIndex === stageCount - 1) {
        return Math.max(0, stageIndex - 1);
      }
      return selectedIndex;
    });
    void Promise.resolve(props.onRemoveEffect(props.layerId, stageIndex)).then(() => {
      restoreActionFocus(selectedStageIndex(), "select-stage");
    });
  };

  const applyBuiltInEffect = (select: HTMLSelectElement) => {
    const presetId = select.value;
    select.value = "";
    if (presetId) void props.onApplyBuiltin(props.layerId, presetId);
  };

  const BuiltInEffectSelect = () => (
    <label class="builtinIsfLibrary">
      <span>Add built-in FX</span>
      <select
        data-video-isf-action="builtin"
        aria-label={`Built-in FX for ${layerDescription()}`}
        value=""
        disabled={stackFull() || eventPulseBusy()}
        onChange={(event) => applyBuiltInEffect(event.currentTarget)}
      >
        <option value="">Choose effect</option>
        <For each={BUILTIN_FX_GROUPS}>
          {(group) => (
            <optgroup label={group.label}>
              <For each={group.effects}>{(effect) => <option value={effect[0]}>{effect[1]}</option>}</For>
            </optgroup>
          )}
        </For>
      </select>
    </label>
  );

  const EnableEffectButton = () => (
    <Show when={selectedStage()}>
      {(stage) => (
        <button
          data-video-isf-action="bypass"
          class={stage().enabled ? "active" : ""}
          aria-label={`${stage().label} FX enabled for ${layerDescription()}`}
          aria-pressed={stage().enabled}
          disabled={eventPulseBusy()}
          onClick={() => {
            void Promise.resolve(props.onSetEffectEnabled(
              props.layerId,
              selectedStageIndex(),
              !stage().enabled,
            )).then(() => restoreActionFocus(null, "bypass"));
          }}
        >
          {stage().enabled ? "Enabled" : "Bypassed"}
        </button>
      )}
    </Show>
  );

  const StackRows = () => (
    <div class="videoIsfStack" role="list" aria-label={`FX stack for ${layerDescription()}`}>
      <For each={stages()}>
        {(stage, index) => (
          <div
            class={`videoIsfStackRow ${selectedStageIndex() === index() ? "selected" : ""}`}
            data-video-isf-stage-index={index()}
            role="listitem"
            draggable={reorder.canDrag()}
            title={reorder.canDrag() ? "Drag to reorder, or use Up and Down." : undefined}
            data-fx-draggable={reorder.canDrag() ? "true" : undefined}
            data-fx-dragging={reorder.dragging() === index() ? "true" : undefined}
            data-fx-drop={reorder.dropEdge(index())}
            {...reorder.handlers(index)}
          >
            <button
              class="videoIsfStageSelect"
              data-video-isf-action="select-stage"
              aria-pressed={selectedStageIndex() === index()}
              onClick={() => setSelectedStageIndex(index())}
            >
              <span>{index() + 1}</span>
              <strong data-no-localize>{stage.label}</strong>
            </button>
            <button
              data-video-isf-action="toggle-stage"
              class={stage.enabled ? "active" : ""}
              aria-label={`${stage.enabled ? "Bypass" : "Enable"} FX ${index() + 1}`}
              aria-pressed={stage.enabled}
              disabled={eventPulseBusy()}
              onClick={() => {
                const stageIndex = index();
                void Promise.resolve(props.onSetEffectEnabled(props.layerId, stageIndex, !stage.enabled))
                  .then(() => restoreActionFocus(stageIndex, "toggle-stage"));
              }}
            >
              {stage.enabled ? "On" : "Off"}
            </button>
            <button
              data-video-isf-action="move-up"
              aria-label={`Move FX ${index() + 1} up`}
              disabled={eventPulseBusy() || index() === 0}
              onClick={() => moveStage(index(), -1)}
            >
              Up
            </button>
            <button
              data-video-isf-action="move-down"
              aria-label={`Move FX ${index() + 1} down`}
              disabled={eventPulseBusy() || index() === stages().length - 1}
              onClick={() => moveStage(index(), 1)}
            >
              Down
            </button>
            <button
              data-video-isf-action="reset"
              aria-label={`Reset FX ${index() + 1}`}
              disabled={eventPulseBusy()}
              onClick={() => {
                const stageIndex = index();
                void Promise.resolve(props.onResetEffect(props.layerId, stageIndex))
                  .then(() => restoreActionFocus(stageIndex, "reset"));
              }}
            >
              Reset
            </button>
            <button
              data-video-isf-action="remove"
              class="danger"
              aria-label={`Remove FX ${index() + 1}`}
              disabled={eventPulseBusy()}
              onClick={() => setRemoveConfirmation(index())}
            >
              Remove
            </button>
          </div>
        )}
      </For>
    </div>
  );

  const SelectedStageEditor = () => (
    <Show when={selectedStage()} fallback={<p class="emptyHint">No FX in this stack.</p>}>
      {(stage) => (
        <div class="videoIsfSelectedEditor" data-video-isf-editor-stage={selectedStageIndex()}>
          <div class="videoIsfSelectedHeader">
            <strong data-no-localize>{stage().label}</strong>
            <span>{stage().enabled ? "Live" : "Bypassed"}</span>
          </div>
          <Show when={stage().description}>
            {(description) => <p class="fieldHint">{description()}</p>}
          </Show>
          <VideoFxParameterEditor availableLayers={props.availableLayers} stage={stage()} identity={`${props.layerId}:${selectedStageIndex()}`} busy={eventPulseBusy()} locale={props.locale} bpm={props.bpm}
            projectEpoch={props.projectEpoch} layerId={props.layerId} liveSession={props.liveSession}
            effectCatalog={props.effectCatalog} onApplyEffectCatalog={props.onApplyEffectCatalog}
            onSetStage={(replacement) => {
              if (props.effect) return props.onSetEffect(props.layerId, replaceFxStage(props.effect, selectedStageIndex(), replacement));
            }}
            onSetControl={(name, value) => props.onSetControl(props.layerId, selectedStageIndex(), name, value)}
            onTriggerEvent={(name) => props.onTriggerEvent(props.layerId, selectedStageIndex(), name)} />
        </div>
      )}
    </Show>
  );

  const EffectEditor = (editorProps: { includeBuiltInSelect: boolean }) => (
    <>
      <Show when={editorProps.includeBuiltInSelect}>
        <BuiltInEffectSelect />
      </Show>
      <Show when={runtimeErrorText()}>
        <RuntimeErrors />
      </Show>
      <VideoFxLiveSafety session={props.liveSession} layerId={props.layerId} />
      <StackRows />
      <Show when={removeConfirmation() !== null}>
        <div class="buttonRow" role="group" aria-label="Confirm FX removal">
          <span>{removeConfirmation() === "all" ? "Remove all FX from this layer?" : "Remove this FX from the layer?"}</span>
          <button type="button" class="danger" disabled={eventPulseBusy()} onClick={() => {
            const target = removeConfirmation(); setRemoveConfirmation(null);
            if (target === "all") void props.onSetEffect(props.layerId, null);
            else if (target !== null) removeStage(target);
          }}>Confirm remove</button>
          <button type="button" onClick={() => setRemoveConfirmation(null)}>Cancel</button>
        </div>
      </Show>
      <SelectedStageEditor />
      <div class="buttonRow videoIsfStackActions">
        <button
          data-video-isf-action="add-isf"
          disabled={stackFull() || eventPulseBusy()}
          onClick={() => void props.onImport(props.layerId)}
        >
          Add ISF
        </button>
        <Show when={stages().length > 0}>
          <button class="danger" disabled={eventPulseBusy()} onClick={() => setRemoveConfirmation("all")}>Clear stack</button>
        </Show>
        <span>{stages().length}/{MAX_STACK_STAGES} FX</span>
      </div>
      <p class="fieldHint">
        Portable single-pass ISF stages stay embedded in the project and run as one GPU stack.
      </p>
    </>
  );

  return (
    <Show
      when={props.compact}
      fallback={
        <details ref={(element) => { panelElement = element; }} class="videoIsfPanel" open={stages().length === 0 || Boolean(runtimeErrorText())}>
          <summary>
            <strong>ISF Stack</strong>
            <Show when={stages().length > 0} fallback={<span>None</span>}>
              <span>{activeStageCount()}/{stages().length} live</span>
            </Show>
          </summary>
          <div class="videoIsfBody">
            <EffectEditor includeBuiltInSelect />
          </div>
        </details>
      }
    >
      <div ref={(element) => { panelElement = element; }} class="videoIsfPanel videoIsfPanelCompact" data-video-isf-layer-id={props.layerId} aria-busy={eventPulseBusy()}>
        <div class="videoIsfQuickRack">
          <div class="videoIsfQuickStatus">
            <strong>FX</strong>
            <Show when={stages().length > 0} fallback={<span>None</span>}>
              <>
                <span>{activeStageCount()}/{stages().length}</span>
                <span data-no-localize>{selectedStage()?.label}</span>
                <span class="videoIsfEffectState">
                  {selectedStage()?.enabled ? "Enabled" : "Bypassed"}
                </span>
              </>
            </Show>
            <Show when={runtimeErrorText()}>
              {(runtimeError) => (
                <span
                  class="inlineError videoIsfRuntimeBadge"
                  role="alert"
                  aria-label={`FX error for ${layerDescription()}: ${runtimeError()}`}
                  title={runtimeError()}
                >
                  FX error
                </span>
              )}
            </Show>
          </div>
          <BuiltInEffectSelect />
          <div class="buttonRow videoIsfQuickActions">
            <EnableEffectButton />
            <button
              data-video-isf-action="advanced"
              class={advancedOpen() ? "active" : ""}
              aria-label={`Advanced FX controls for ${layerDescription()}`}
              aria-expanded={advancedOpen()}
              aria-controls={advancedId}
              onClick={() => setAdvancedOpen((open) => !open)}
            >
              Advanced
            </button>
          </div>
        </div>
        <Show when={advancedOpen()}>
          <div class="videoIsfBody videoIsfAdvanced" id={advancedId}>
            <EffectEditor includeBuiltInSelect={false} />
          </div>
        </Show>
      </div>
    </Show>
  );
}
