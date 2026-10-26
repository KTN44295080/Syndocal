import { createEffect, createSignal, For, Show, type ComponentProps, type JSX } from "solid-js";
import { authoredCueLiveModifier } from "../cueLiveModifier";
import type { CueMetadataDraft } from "../editorDrafts";
import { cueIdentityCss } from "../identityColor";
import { displayNumber } from "../numberDisplay";
import { sceneCueKind } from "../sceneCueKind";
import type {
  CueLiveDirection,
  CueLiveModifierSettings,
  CueLiveModifierState,
  CueSummary,
  DaslightCustomCurveSource,
  EffectKind,
  EffectSummary,
} from "../types";
import { ChaserEffectEditorPanel } from "./ChaserEffectEditorPanel";
import { ColorEffectEditorPanel } from "./ColorEffectEditorPanel";
import { ColorMappingEffectEditorPanel } from "./ColorMappingEffectEditorPanel";
import { CueLiveModifierStrip } from "./CueLiveModifierStrip";
import { CurveEffectEditorPanel } from "./CurveEffectEditorPanel";
import { DaslightCustomCurvePanel } from "./DaslightCustomCurvePanel";
import { EffectActionControlsPanel } from "./EffectActionControlsPanel";
import { EffectFeatureEditorPanel } from "./EffectFeatureEditorPanel";
import { FxColorPaletteLibraryPanel } from "./FxColorPaletteLibraryPanel";
import { MappingEffectEditorPanel } from "./MappingEffectEditorPanel";
import { MoveEffectEditorPanel } from "./MoveEffectEditorPanel";
import { SceneEffectTargetEditor } from "./SceneEffectTargetEditor";
import {
  EffectFamilyChooser,
  type EffectChooserFamily,
} from "./EffectFamilyChooser";
import { ValueEffectEditorPanel } from "./ValueEffectEditorPanel";
import "./SceneSettingsPane.css";

export type SceneSettingsSurface = "contents" | "fx" | "settings" | "details";

export interface SceneEffectEditorModel {
  effectType: EffectKind;
  attribute: string;
  attributeOptions: string[];
  onAttribute: (attribute: string) => void;
  color: ComponentProps<typeof ColorEffectEditorPanel>;
  chaser: ComponentProps<typeof ChaserEffectEditorPanel>;
  move: ComponentProps<typeof MoveEffectEditorPanel>;
  value: ComponentProps<typeof ValueEffectEditorPanel>;
  curve: ComponentProps<typeof CurveEffectEditorPanel>;
  mapping: ComponentProps<typeof MappingEffectEditorPanel>;
  colorMapping: ComponentProps<typeof ColorMappingEffectEditorPanel>;
  daslightCustomCurve: {
    source: DaslightCustomCurveSource | null;
  };
  features: ComponentProps<typeof EffectFeatureEditorPanel>;
  target: ComponentProps<typeof SceneEffectTargetEditor>;
  palette: ComponentProps<typeof FxColorPaletteLibraryPanel>;
  action: ComponentProps<typeof EffectActionControlsPanel>;
}

interface SceneSettingsPaneProps {
  cue: CueSummary;
  groupColors?: Record<string, string>;
  draft: CueMetadataDraft;
  effects: EffectSummary[];
  selectedEffectId: number | null;
  activeFamily: EffectChooserFamily;
  activeSurface: SceneSettingsSurface;
  moveFxEnabled: boolean;
  running: boolean;
  liveStates?: CueLiveModifierState[];
  editor: SceneEffectEditorModel;
  details: JSX.Element;
  onSurface: (surface: SceneSettingsSurface) => void;
  onDraft: (patch: Partial<CueMetadataDraft>) => void;
  onSaveMetadata: () => void | Promise<void>;
  onSetColor: (color: string | null) => void | Promise<void>;
  onSetLiveModifier: (
    cueId: number,
    speed: number,
    size: number,
    phase: number,
    direction: CueLiveDirection,
    segment: number,
  ) => void | Promise<void>;
  onClearLiveModifier: (cueId: number) => void | Promise<void>;
  onSetLiveModifierDefaults: (
    cueId: number,
    settings: CueLiveModifierSettings | null,
  ) => void | Promise<void>;
  onClose: () => void;
  onEditSource: () => void;
  onSelectEffect: (effectId: number) => void;
  onSelectFamily: (family: EffectChooserFamily) => void | Promise<void>;
  onSetEffectEnabled: (effectId: number, enabled: boolean) => void | Promise<void>;
  onDuplicateEffect: (effectId: number) => void | Promise<void>;
  onMoveEffect: (effectId: number, delta: -1 | 1) => void | Promise<void>;
  onRemoveEffect: (effectId: number) => void | Promise<void>;
}

const optionalNumber = (value: string) => {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
};

const SurfaceIcon = (props: { surface: SceneSettingsSurface }) => (
  <svg viewBox="0 0 24 24" aria-hidden="true">
    <Show when={props.surface === "contents"}>
      <path d="M5 4.5h14v4H5zM5 10h14v4H5zM5 15.5h14v4H5z" />
    </Show>
    <Show when={props.surface === "fx"}>
      <path d="m12 2 1.5 5.2L19 5.5l-3.3 4.6L21 12l-5.3 1.9 3.3 4.6-5.5-1.7L12 22l-1.5-5.2L5 18.5l3.3-4.6L3 12l5.3-1.9L5 5.5l5.5 1.7z" />
    </Show>
    <Show when={props.surface === "settings"}>
      <path d="M4 6h10v2H4zm14-2h2v6h-2zM10 11h10v2H10zm-6-2h2v6H4zm0 8h10v2H4zm14-2h2v6h-2z" />
    </Show>
    <Show when={props.surface === "details"}>
      <path d="M5 3.5h10l4 4v13H5zM15 3.5v4h4M8 11h8M8 14.5h8M8 18h5" />
    </Show>
  </svg>
);

const sceneFxWideStorageKey = "syndocal.sceneFxEditorWide.v1";

const effectTypeLabels: Record<string, string> = {
  Color: "Colour FX",
  Chaser: "Chaser FX",
  Move: "Move FX",
  Value: "Value FX",
  Curve: "Curve FX",
  Mapping: "2D Mapping",
  ColorMapping: "2D Colour Mapping",
  Lfo: "LFO",
  PositionWave: "Position wave",
};

const effectTypeLabel = (effectType: string) => effectTypeLabels[effectType] ?? effectType;

const readSceneFxWide = () => {
  try {
    return window.localStorage.getItem(sceneFxWideStorageKey) === "1";
  } catch {
    return false;
  }
};

export function SceneSettingsPane(props: SceneSettingsPaneProps) {
  // Wide FX editing is a per-machine width preference. The selected form and
  // its disclosures use the same scrollport in either width.
  const [fxWide, setFxWide] = createSignal(readSceneFxWide());
  const fxWideActive = () => fxWide() && props.activeSurface === "fx";
  const toggleFxWide = () => {
    const next = !fxWide();
    setFxWide(next);
    try {
      window.localStorage.setItem(sceneFxWideStorageKey, next ? "1" : "0");
    } catch {
      // The layout still changes for this session when storage is unavailable.
    }
  };
  const selectedEffect = () =>
    props.effects.find((effect) => effect.id === props.selectedEffectId) ?? null;
  const selectedEffectIndex = () =>
    props.effects.findIndex((effect) => effect.id === props.selectedEffectId);
  type FxDisclosure = "add" | "rack" | "targets";
  let sceneScroller: HTMLDivElement | undefined;
  const [openFxDisclosure, setOpenFxDisclosure] = createSignal<FxDisclosure | null>(
    props.effects.length === 0 ? "add" : null,
  );
  let disclosureCueId = props.cue.id;
  createEffect(() => {
    const cueId = props.cue.id;
    const empty = props.effects.length === 0;
    if (cueId !== disclosureCueId || empty) {
      disclosureCueId = cueId;
      setOpenFxDisclosure(empty ? "add" : null);
    }
  });
  const onFxDisclosureToggle = (disclosure: FxDisclosure, open: boolean) => {
    setOpenFxDisclosure((current) => open ? disclosure : current === disclosure ? null : current);
  };
  const returnToFxEditor = () => {
    setOpenFxDisclosure(null);
    queueMicrotask(() => {
      if (sceneScroller) sceneScroller.scrollTop = 0;
    });
  };
  const addSceneFx = async (family: EffectChooserFamily) => {
    await props.onSelectFamily(family);
    returnToFxEditor();
  };
  const sceneKind = () => sceneCueKind(props.cue);
  const sceneKindClass = () => sceneKind() === "TIMELINE"
    ? "super"
    : sceneKind().toLowerCase();
  const sceneKindDescription = () => sceneKind() === "STATIC"
    ? "Static scene"
    : sceneKind() === "FX"
      ? "FX scene"
      : "Timeline scene";
  const setFlashMode = (flash: boolean) => {
    void props.onSetLiveModifierDefaults(props.cue.id, {
      ...authoredCueLiveModifier(props.cue),
      flash,
    });
  };

  return (
    <section
      class="sceneSettingsPane"
      aria-label="Scene settings"
      data-scene-settings
      data-selected-scene-id={props.cue.id}
      data-scene-settings-kind={sceneKind()}
      data-scene-settings-surface={props.activeSurface}
      data-scene-settings-wide={fxWideActive() ? "true" : undefined}
      style={{
        "--cue-identity": cueIdentityCss(
          props.cue.id,
          props.cue.color,
          "fill",
          props.cue.group_id,
          props.cue.group_id ? props.groupColors?.[props.cue.group_id] : null,
        ),
        "--cue-identity-text": cueIdentityCss(
          props.cue.id,
          props.cue.color,
          "text",
          props.cue.group_id,
          props.cue.group_id ? props.groupColors?.[props.cue.group_id] : null,
        ),
      }}
    >
      <header class="sceneSettingsHeader">
        <div>
          <span class="uiMicroLabel">Scene settings</span>
          <strong data-no-localize title={props.cue.label}>{props.cue.label}</strong>
        </div>
        <span class={`sceneSettingsKind uiMicroLabel ${sceneKindClass()}`}>
          {sceneKind()}
        </span>
        <Show when={props.activeSurface === "fx"}>
          <button
            type="button"
            class="sceneSettingsWideToggle"
            classList={{ active: fxWide() }}
            data-scene-settings-wide-toggle
            aria-pressed={fxWide()}
            title={fxWide() ? "Return the FX editor to the side column" : "Give the FX editor more room"}
            onClick={toggleFxWide}
          >
            {fxWide() ? "Narrow" : "Widen"}
          </button>
        </Show>
        <button
          type="button"
          class="sceneSettingsEditSource"
          classList={{ active: props.activeSurface === "details" }}
          data-scene-settings-edit-source
          aria-pressed={props.activeSurface === "details"}
          onClick={props.onEditSource}
        >
          Cue details
        </button>
        <button
          type="button"
          class="sceneSettingsClose"
          aria-label="Close scene settings"
          title="Close scene settings"
          onClick={props.onClose}
        >
          <span aria-hidden="true" data-no-localize>×</span>
        </button>
      </header>

      <div class="sceneSettingsBody">
        <div
          class="sceneSettingsScroller"
          ref={sceneScroller}
          role="region"
          aria-label="Scene settings editor"
          tabIndex={0}
          data-wheel-scroll-surface="scene-settings"
        >
          <Show when={props.activeSurface === "contents"}>
            <section class="sceneSettingsSection sceneContentsSurface" data-scene-contents>
              <header>
                <strong class="uiMicroLabel">Scene properties</strong>
                <span>{sceneKindDescription()}</span>
              </header>
              <div class="sceneSettingsPropertyGrid">
                <label class="sceneIdentityColorField">
                  Identity color
                  <span>
                    <input
                      type="color"
                      value={props.cue.color ?? "#6d7880"}
                      aria-label="Scene identity color"
                      onChange={(event) => void props.onSetColor(event.currentTarget.value)}
                    />
                    <button
                      type="button"
                      disabled={!props.cue.color}
                      onClick={() => void props.onSetColor(null)}
                    >
                      Clear color
                    </button>
                  </span>
                </label>
                <label>
                  Fade in / out (ms)
                  <input
                    type="number"
                    class="tabularNums"
                    data-scene-property="fade-ms"
                    min="0"
                    step="10"
                    value={displayNumber(props.draft.fade_ms, 0)}
                    onInput={(event) => props.onDraft({ fade_ms: Number(event.currentTarget.value) })}
                    onBlur={(event) => {
                      event.currentTarget.value = displayNumber(props.draft.fade_ms, 0);
                    }}
                  />
                </label>
                <label>
                  Duration (beats)
                  <input
                    type="number"
                    class="tabularNums"
                    data-scene-property="authored-beats"
                    min="0.25"
                    max="1024"
                    step="0.25"
                    placeholder="Auto"
                    value={displayNumber(props.draft.authored_beats, 3)}
                    onInput={(event) => props.onDraft({
                      authored_beats: optionalNumber(event.currentTarget.value),
                    })}
                    onBlur={(event) => {
                      event.currentTarget.value = displayNumber(props.draft.authored_beats, 3);
                    }}
                  />
                </label>
                <label>
                  Pre-wait (ms)
                  <input
                    type="number"
                    class="tabularNums"
                    data-scene-property="pre-wait-ms"
                    min="0"
                    step="10"
                    value={displayNumber(props.draft.pre_wait_ms, 0)}
                    onInput={(event) => props.onDraft({
                      pre_wait_ms: Number(event.currentTarget.value),
                    })}
                    onBlur={(event) => {
                      event.currentTarget.value = displayNumber(props.draft.pre_wait_ms, 0);
                    }}
                  />
                </label>
                <label>
                  Follow (ms)
                  <input
                    type="number"
                    class="tabularNums"
                    data-scene-property="follow-ms"
                    min="0"
                    step="10"
                    placeholder="No follow"
                    value={displayNumber(props.draft.follow_ms, 0)}
                    onInput={(event) => props.onDraft({
                      follow_ms: optionalNumber(event.currentTarget.value),
                    })}
                    onBlur={(event) => {
                      event.currentTarget.value = displayNumber(props.draft.follow_ms, 0);
                    }}
                  />
                </label>
              </div>
              <button
                type="button"
                class="sceneSettingsSave"
                data-save-scene-properties
                onClick={() => void props.onSaveMetadata()}
              >
                Save scene properties
              </button>
            </section>
            <Show when={props.running}>
              <section
                class="sceneSettingsSection sceneRuntimeControls"
                data-scene-settings-live-controls
              >
                <header>
                  <strong class="uiMicroLabel">Runtime controls</strong>
                  <span data-no-localize>LIVE</span>
                </header>
                <CueLiveModifierStrip
                  cue={props.cue}
                  liveStates={props.liveStates}
                  onSetCueLiveModifier={props.onSetLiveModifier}
                  onClearCueLiveModifier={props.onClearLiveModifier}
                />
              </section>
            </Show>
          </Show>

          <Show when={props.activeSurface === "fx"}>
            <div class="sceneFxDisclosureControls" data-scene-fx-controls>
              <details
                class="sceneSettingsSection sceneFxChooserSection"
                data-scene-fx-chooser
                data-scene-fx-disclosure="add"
                open={openFxDisclosure() === "add"}
                onToggle={(event) => onFxDisclosureToggle("add", event.currentTarget.open)}
              >
                <summary>
                  <strong class="uiMicroLabel">Add FX</strong>
                </summary>
                <div class="sceneFxDisclosureBody">
                  <p class="hint">Each button adds a new FX to this scene.</p>
                  <p class="hint sceneFxCreationGuide">Pick a family to add an FX, then set 1 Target fixtures and 2 Channels to move below, adjust it, and Save.</p>
                  <EffectFamilyChooser
                    activeFamily={props.activeFamily}
                    onSelectFamily={addSceneFx}
                    disabledFamilies={{ "MOVE FX": !props.moveFxEnabled }}
                  />
                </div>
              </details>

              <Show when={props.effects.length > 0}>
                <details
                  class="sceneSettingsSection sceneOwnedFxSection"
                  data-scene-fx-disclosure="rack"
                  open={openFxDisclosure() === "rack"}
                  onToggle={(event) => onFxDisclosureToggle("rack", event.currentTarget.open)}
                >
                  <summary>
                    <strong class="uiMicroLabel">{props.effects.length} FX</strong>
                    <span class="sceneFxSelectedLabel" data-scene-fx-selected-label data-no-localize title={selectedEffect()?.label}>{selectedEffect()?.label ?? "—"}</span>
                  </summary>
                  <div class="sceneFxDisclosureBody">
                    <nav class="sceneOwnedFxList" aria-label="Cue-owned FX list">
                      <For each={props.effects}>
                        {(effect, index) => (
                          <div
                            class={`sceneOwnedFxRow ${effect.id === props.selectedEffectId ? "active" : ""}`}
                            data-scene-owned-effect={effect.id}
                            data-scene-owned-effect-enabled={effect.enabled ? "true" : "false"}
                          >
                            <button
                              type="button"
                              class="sceneOwnedFxSelect"
                              aria-pressed={effect.id === props.selectedEffectId}
                              onClick={() => {
                                props.onSelectEffect(effect.id);
                              returnToFxEditor();
                              }}
                            >
                              <span>{index() + 1}</span>
                              <strong data-no-localize title={effect.label}>{effect.label}</strong>
                              <small>{effect.effect_type}</small>
                            </button>
                            <label class="sceneOwnedFxToggle">
                              <input
                                type="checkbox"
                                checked={effect.enabled}
                                aria-label={`${effect.enabled ? "Bypass" : "Enable"} FX ${index() + 1}`}
                                onChange={(event) => void props.onSetEffectEnabled(
                                  effect.id,
                                  event.currentTarget.checked,
                                )}
                              />
                              <span>{effect.enabled ? "Enabled" : "Disabled"}</span>
                            </label>
                            <button
                              type="button"
                              class="sceneOwnedFxRemove"
                              aria-label={`Remove FX ${index() + 1}`}
                              title={`Remove FX ${index() + 1}`}
                              onClick={() => void props.onRemoveEffect(effect.id)}
                            >
                              <span aria-hidden="true" data-no-localize>×</span>
                            </button>
                          </div>
                        )}
                      </For>
                    </nav>
                    <Show when={selectedEffect()}>
                      <div class="sceneOwnedFxToolbar" role="toolbar" aria-label="Cue-owned FX rack actions">
                        <button
                          type="button"
                          disabled={selectedEffectIndex() <= 0}
                          onClick={() => void props.onMoveEffect(props.selectedEffectId!, -1)}
                        >
                          Move up
                        </button>
                        <button
                          type="button"
                          disabled={selectedEffectIndex() < 0 || selectedEffectIndex() >= props.effects.length - 1}
                          onClick={() => void props.onMoveEffect(props.selectedEffectId!, 1)}
                        >
                          Move down
                        </button>
                        <button
                          type="button"
                          disabled={!selectedEffect()?.params}
                          title={selectedEffect()?.params ? "Duplicate this cue-owned FX" : "Linked project FX must be made cue-owned before duplication"}
                          onClick={() => void props.onDuplicateEffect(props.selectedEffectId!)}
                        >
                          Duplicate
                        </button>
                      </div>
                    </Show>
                  </div>
                </details>
              </Show>
              <Show when={selectedEffect()}>
                <details
                  class="sceneFxTargetsDisclosure"
                  data-scene-fx-disclosure="targets"
                  open={openFxDisclosure() === "targets"}
                  onToggle={(event) => onFxDisclosureToggle("targets", event.currentTarget.open)}
                >
                  <summary>
                    <strong>Targets</strong>
                    <span data-scene-fx-target-summary data-no-localize title={props.editor.target.summary}>{props.editor.target.summary}</span>
                  </summary>
                  <div class="sceneFxDisclosureBody">
                    <SceneEffectTargetEditor {...props.editor.target} />
                    <section class="sceneFxChannels" data-scene-fx-channels aria-label="FX channels">
                      <h4 class="sceneFxStepTitle" data-fx-step="2">Channels to move</h4>
                      <Show when={props.editor.effectType === "Color"}>
                        <p class="sceneFxChannelNote">Whole-fixture colour: every patched colour channel moves together. To move only some channels, such as Green and Blue, use VALUE FX.</p>
                      </Show>
                      <Show when={props.editor.effectType === "ColorMapping"}>
                        <p class="sceneFxChannelNote">Colour output drives each fixture's whole colour. Set Output mode to Feature below to choose channels.</p>
                      </Show>
                      <Show when={props.editor.effectType === "Move"}>
                        <p class="sceneFxChannelNote">Pan and Tilt of fixtures with paired Pan/Tilt.</p>
                      </Show>
                      <Show when={props.editor.effectType === "Chaser"}>
                        <p class="sceneFxChannelNote">Choose the chaser's channels under Features in the chaser settings below.</p>
                      </Show>
                      <Show
                        when={["Lfo", "PositionWave"].includes(props.editor.effectType) && !props.editor.target.readOnly}
                      >
                        <label class="sceneSettingsAttribute">
                          Attribute
                          <select
                            data-scene-property="effect-attribute"
                            value={props.editor.attribute}
                            disabled={props.editor.attributeOptions.length === 0}
                            onInput={(event) => props.editor.onAttribute(event.currentTarget.value)}
                          >
                            <For each={props.editor.attributeOptions}>
                              {(attribute) => <option value={attribute}>{attribute}</option>}
                            </For>
                          </select>
                        </label>
                      </Show>
                      <Show when={["Value", "Curve", "Mapping"].includes(props.editor.effectType)}>
                        <EffectFeatureEditorPanel {...props.editor.features} />
                      </Show>
                    </section>
                  </div>
                </details>
              </Show>
            </div>

            <Show
              when={props.effects.length > 0}
              fallback={
                <p class="sceneSettingsEmpty textPretty">
                  Select an FX family above to add the first cue-owned FX.
                </p>
              }
            >
              <Show
                when={selectedEffect()}
                fallback={<p class="sceneSettingsEmpty">Select a cue-owned FX to edit it.</p>}
              >
                <section
                  class="sceneSettingsSection sceneFxEditor"
                  data-scene-settings-effect-editor={props.editor.effectType}
                  aria-label={effectTypeLabel(props.editor.effectType)}
                >
                  <Show when={props.editor.effectType === "Lfo" && props.editor.daslightCustomCurve.source}>
                    <DaslightCustomCurvePanel {...props.editor.daslightCustomCurve} />
                  </Show>
                  <Show when={props.editor.effectType === "Color"}>
                    <ColorEffectEditorPanel {...props.editor.color} />
                  </Show>
                  <Show when={props.editor.effectType === "Chaser"}>
                    <ChaserEffectEditorPanel {...props.editor.chaser} />
                  </Show>
                  <Show when={props.editor.effectType === "Move"}>
                    <MoveEffectEditorPanel {...props.editor.move} />
                  </Show>
                  <Show when={props.editor.effectType === "Value"}>
                    <ValueEffectEditorPanel {...props.editor.value} />
                  </Show>
                  <Show when={props.editor.effectType === "Curve"}>
                    <CurveEffectEditorPanel {...props.editor.curve} />
                  </Show>
                  <Show when={props.editor.effectType === "Mapping"}>
                    <MappingEffectEditorPanel {...props.editor.mapping} />
                  </Show>
                  <Show when={props.editor.effectType === "ColorMapping"}>
                    <ColorMappingEffectEditorPanel {...props.editor.colorMapping} />
                  </Show>
                  <FxColorPaletteLibraryPanel {...props.editor.palette} />
                  <EffectActionControlsPanel {...props.editor.action} />
                </section>
              </Show>
            </Show>
          </Show>

          <Show when={props.activeSurface === "settings"}>
            <section class="sceneSettingsSection sceneAdvancedSurface" data-scene-advanced-settings>
              <header>
                <strong class="uiMicroLabel">Advanced scene properties</strong>
                <span>Existing Syndocal settings</span>
              </header>
              <div class="sceneAdvancedKind">
                <span>Scene type</span>
                <strong data-no-localize>{sceneKind()}</strong>
                <small>
                  Scene pin state is derived from cue-owned FX; Syndocal has no independent pin toggle.
                </small>
              </div>
              <div class="sceneSettingsPropertyGrid">
                <label>
                  Cue number
                  <input
                    class="tabularNums"
                    value={props.draft.cue_number}
                    onInput={(event) => props.onDraft({ cue_number: event.currentTarget.value })}
                  />
                </label>
                <label>
                  Group
                  <input
                    value={props.draft.group_id ?? ""}
                    onInput={(event) => props.onDraft({
                      group_id: event.currentTarget.value.trim() || null,
                    })}
                  />
                </label>
                <label class="sceneSettingsWideField">
                  Recall mode
                  <select
                    value={props.draft.recall_mode}
                    onInput={(event) => props.onDraft({
                      recall_mode: event.currentTarget.value as CueMetadataDraft["recall_mode"],
                    })}
                  >
                    <option value="Coexist">Coexist</option>
                    <option value="ReplaceGroup">Replace group</option>
                  </select>
                </label>
              </div>
              <div class="sceneAdvancedToggles">
                <label class="checkbox">
                  <input
                    type="checkbox"
                    checked={props.draft.tracking}
                    onChange={(event) => props.onDraft({ tracking: event.currentTarget.checked })}
                  />
                  Tracking
                </label>
                <label class="checkbox">
                  <input
                    type="checkbox"
                    checked={props.draft.mark}
                    onChange={(event) => props.onDraft({ mark: event.currentTarget.checked })}
                  />
                  Mark
                </label>
                <label class="checkbox">
                  <input
                    type="checkbox"
                    checked={props.cue.live_modifiers?.flash ?? false}
                    onChange={(event) => setFlashMode(event.currentTarget.checked)}
                  />
                  Flash mode
                </label>
              </div>
              <label class="sceneSettingsNotes">
                Notes
                <textarea
                  rows="4"
                  value={props.draft.notes}
                  onInput={(event) => props.onDraft({ notes: event.currentTarget.value })}
                />
              </label>
              <button
                type="button"
                class="sceneSettingsSave"
                data-save-scene-advanced
                onClick={() => void props.onSaveMetadata()}
              >
                Save scene properties
              </button>
            </section>
          </Show>

          <Show when={props.activeSurface === "details"}>
            <section class="sceneSettingsSection sceneDetailsSurface" data-scene-details>
              <header>
                <strong class="uiMicroLabel">Cue details</strong>
                <span>Source, steps, timing and recall</span>
              </header>
              {props.details}
            </section>
          </Show>
        </div>

        <nav class="sceneSettingsSurfaceRail" aria-label="シーン設定の表示切替">
          <button
            type="button"
            classList={{ active: props.activeSurface === "contents" }}
            aria-label="コンテンツ面を表示"
            title="コンテンツ"
            aria-pressed={props.activeSurface === "contents"}
            data-scene-settings-surface-control="contents"
            onClick={() => props.onSurface("contents")}
          >
            <SurfaceIcon surface="contents" />
            <span class="sceneSettingsSurfaceRailLabel">Scene</span>
          </button>
          <button
            type="button"
            classList={{ active: props.activeSurface === "fx" }}
            aria-label="FX面を表示"
            title="FX面"
            aria-pressed={props.activeSurface === "fx"}
            data-scene-settings-surface-control="fx"
            onClick={() => props.onSurface("fx")}
          >
            <SurfaceIcon surface="fx" />
            <span class="sceneSettingsSurfaceRailLabel">FX</span>
          </button>
          <button
            type="button"
            classList={{ active: props.activeSurface === "settings" }}
            aria-label="設定面を表示"
            title="設定"
            aria-pressed={props.activeSurface === "settings"}
            data-scene-settings-surface-control="settings"
            onClick={() => props.onSurface("settings")}
          >
            <SurfaceIcon surface="settings" />
            <span class="sceneSettingsSurfaceRailLabel">Props</span>
          </button>
          <button
            type="button"
            classList={{ active: props.activeSurface === "details" }}
            aria-label="キュー詳細面を表示"
            title="キュー詳細"
            aria-pressed={props.activeSurface === "details"}
            data-scene-settings-surface-control="details"
            onClick={() => props.onSurface("details")}
          >
            <SurfaceIcon surface="details" />
            <span class="sceneSettingsSurfaceRailLabel">Cue</span>
          </button>
        </nav>
      </div>
    </section>
  );
}
