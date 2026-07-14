import { createEffect, createMemo, createSignal, For, Show } from "solid-js";
import type {
  VideoIsfControlSummary,
  VideoIsfEffectStageSummary,
  VideoIsfEffectSummary,
  VideoIsfStageError,
} from "../types";

interface VideoIsfEffectPanelProps {
  layerId: number;
  layerLabel?: string;
  compact?: boolean;
  effect?: VideoIsfEffectSummary | null;
  runtimeErrors?: VideoIsfStageError[];
  eventPulseBusy?: boolean;
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

const componentCount = (control: VideoIsfControlSummary) =>
  control.kind === "Color" ? 4 : control.kind === "Point2d" ? 2 : 1;

const runtimeErrorLabel = (error: VideoIsfStageError) =>
  error.stage_index === null || error.stage_index === undefined
    ? error.message
    : `FX ${error.stage_index + 1}${error.stage_label ? ` ${error.stage_label}` : ""}: ${error.message}`;

export function VideoIsfEffectPanel(props: VideoIsfEffectPanelProps) {
  let panelElement: HTMLElement | undefined;
  const [advancedOpen, setAdvancedOpen] = createSignal(false);
  const [selectedStageIndex, setSelectedStageIndex] = createSignal(0);
  const advancedId = `video-isf-advanced-${props.layerId}`;
  const stages = createMemo(() => effectStages(props.effect));
  const selectedStage = createMemo(() => stages()[selectedStageIndex()] ?? stages()[0]);
  const activeStageCount = createMemo(() => stages().filter((stage) => stage.enabled).length);
  const layerDescription = () => `${props.layerLabel ?? "Video layer"} (layer ${props.layerId})`;
  const stackFull = () => stages().length >= MAX_STACK_STAGES;
  const runtimeErrorText = () => (props.runtimeErrors ?? []).map(runtimeErrorLabel).join("; ");

  createEffect(() => {
    const count = stages().length;
    if (count === 0) {
      setSelectedStageIndex(0);
    } else if (selectedStageIndex() >= count) {
      setSelectedStageIndex(count - 1);
    }
  });

  const setControlComponent = (
    control: VideoIsfControlSummary,
    index: number,
    next: number,
  ) => {
    const value = [...control.value] as [number, number, number, number];
    value[index] = next;
    void props.onSetControl(
      props.layerId,
      selectedStageIndex(),
      control.name,
      value,
    );
  };

  const triggerEvent = (control: VideoIsfControlSummary) => {
    void props.onTriggerEvent(props.layerId, selectedStageIndex(), control.name);
  };

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
        disabled={stackFull() || props.eventPulseBusy}
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
          disabled={props.eventPulseBusy}
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
              disabled={props.eventPulseBusy}
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
              disabled={props.eventPulseBusy || index() === 0}
              onClick={() => moveStage(index(), -1)}
            >
              Up
            </button>
            <button
              data-video-isf-action="move-down"
              aria-label={`Move FX ${index() + 1} down`}
              disabled={props.eventPulseBusy || index() === stages().length - 1}
              onClick={() => moveStage(index(), 1)}
            >
              Down
            </button>
            <button
              data-video-isf-action="reset"
              aria-label={`Reset FX ${index() + 1}`}
              disabled={props.eventPulseBusy}
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
              disabled={props.eventPulseBusy}
              onClick={() => removeStage(index())}
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
          <For each={stage().controls}>
            {(control) => (
              <div
                class="videoIsfControl"
                data-video-isf-control-kind={control.kind}
                data-video-isf-control-name={control.name}
              >
                <span data-no-localize>{control.name}</span>
                <Show
                  when={control.kind !== "Event"}
                  fallback={
                    <button
                      data-video-isf-action="trigger-event"
                      aria-label={`Trigger ${control.name} for ${layerDescription()}`}
                      disabled={props.eventPulseBusy}
                      onClick={() => triggerEvent(control)}
                    >
                      Trigger
                    </button>
                  }
                >
                  <Show
                    when={control.kind !== "Bool"}
                    fallback={
                      <label class="checkbox">
                        <input
                          type="checkbox"
                          checked={control.value[0] >= 0.5}
                          aria-label={`${control.name} for ${stage().label} FX on ${layerDescription()}`}
                          disabled={props.eventPulseBusy}
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
                          aria-label={`${control.name} component ${index + 1} for ${layerDescription()}`}
                          disabled={props.eventPulseBusy}
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
          <Show when={stage().controls.length === 0}>
            <p class="emptyHint">This shader has no operator controls.</p>
          </Show>
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
        {(runtimeError) => (
          <p class="inlineError" role={props.compact ? undefined : "alert"}>
            {runtimeError()}
          </p>
        )}
      </Show>
      <StackRows />
      <SelectedStageEditor />
      <div class="buttonRow videoIsfStackActions">
        <button
          data-video-isf-action="add-isf"
          disabled={stackFull() || props.eventPulseBusy}
          onClick={() => void props.onImport(props.layerId)}
        >
          Add ISF
        </button>
        <Show when={stages().length > 0}>
          <button class="danger" disabled={props.eventPulseBusy} onClick={() => void props.onSetEffect(props.layerId, null)}>Clear stack</button>
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
      <div ref={(element) => { panelElement = element; }} class="videoIsfPanel videoIsfPanelCompact" data-video-isf-layer-id={props.layerId} aria-busy={props.eventPulseBusy}>
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
