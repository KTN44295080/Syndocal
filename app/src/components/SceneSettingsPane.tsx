import { For, Show, type ComponentProps } from "solid-js";
import type { CueMetadataDraft } from "../editorDrafts";
import { cueIdentityCss } from "../identityColor";
import { displayNumber } from "../numberDisplay";
import type { CueSummary, EffectKind, EffectSummary } from "../types";
import { ChaserEffectEditorPanel } from "./ChaserEffectEditorPanel";
import { ColorEffectEditorPanel } from "./ColorEffectEditorPanel";
import { ColorMappingEffectEditorPanel } from "./ColorMappingEffectEditorPanel";
import { CurveEffectEditorPanel } from "./CurveEffectEditorPanel";
import { EffectActionControlsPanel } from "./EffectActionControlsPanel";
import { MappingEffectEditorPanel } from "./MappingEffectEditorPanel";
import { MoveEffectEditorPanel } from "./MoveEffectEditorPanel";
import {
  EffectFamilyChooser,
  type EffectChooserFamily,
} from "./SampleEffectPresetPanel";
import { ValueEffectEditorPanel } from "./ValueEffectEditorPanel";

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
  action: ComponentProps<typeof EffectActionControlsPanel>;
}

interface SceneSettingsPaneProps {
  cue: CueSummary;
  groupColors?: Record<string, string>;
  draft: CueMetadataDraft;
  effects: EffectSummary[];
  selectedEffectId: number | null;
  activeFamily: EffectChooserFamily;
  editor: SceneEffectEditorModel;
  onDraft: (patch: Partial<CueMetadataDraft>) => void;
  onSaveMetadata: () => void | Promise<void>;
  onSetColor: (color: string | null) => void | Promise<void>;
  onClose: () => void;
  onEditSource: () => void;
  onSelectEffect: (effectId: number) => void;
  onSelectFamily: (family: EffectChooserFamily) => void | Promise<void>;
}

const optionalNumber = (value: string) => {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const parsed = Number(trimmed);
  return Number.isFinite(parsed) ? parsed : null;
};

export function SceneSettingsPane(props: SceneSettingsPaneProps) {
  const selectedEffect = () =>
    props.effects.find((effect) => effect.id === props.selectedEffectId) ?? null;

  return (
    <section
      class="sceneSettingsPane"
      aria-label="Scene settings"
      data-scene-settings
      data-selected-scene-id={props.cue.id}
      data-scene-settings-kind={props.effects.length > 0 ? "FX" : "STATIC"}
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
        <span class={`sceneSettingsKind uiMicroLabel ${props.effects.length > 0 ? "fx" : "static"}`}>
          {props.effects.length > 0 ? "FX" : "STATIC"}
        </span>
        <button
          type="button"
          class="sceneSettingsEditSource"
          data-scene-settings-edit-source
          onClick={props.onEditSource}
        >
          Edit Source
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

      <div class="sceneSettingsScroller">
        <Show
          when={props.effects.length > 0}
          fallback={
            <>
              <section class="sceneSettingsSection sceneStaticProperties" data-scene-static-properties>
                <header>
                  <strong class="uiMicroLabel">Scene properties</strong>
                  <span class="uiMicroLabel">Static scene</span>
                </header>
                <div class="sceneSettingsPropertyGrid">
                  <label class="sceneSettingsWideField">
                    Label
                    <input
                      value={props.draft.label}
                      onInput={(event) => props.onDraft({ label: event.currentTarget.value })}
                    />
                  </label>
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
              <section class="sceneSettingsSection sceneFxChooserSection" data-scene-static-fx-chooser>
                <header>
                  <strong class="uiMicroLabel">Add FX</strong>
                  <span>Select a family to create a cue-owned FX.</span>
                </header>
                <EffectFamilyChooser
                  activeFamily={props.activeFamily}
                  onSelectFamily={props.onSelectFamily}
                />
              </section>
            </>
          }
        >
          <section class="sceneSettingsSection sceneOwnedFxSection">
            <header>
              <strong class="uiMicroLabel">Cue-owned FX</strong>
              <span>{props.effects.length} FX</span>
            </header>
            <nav class="sceneOwnedFxList" aria-label="Cue-owned FX list">
              <For each={props.effects}>
                {(effect, index) => (
                  <button
                    type="button"
                    class={effect.id === props.selectedEffectId ? "active" : ""}
                    aria-pressed={effect.id === props.selectedEffectId}
                    data-scene-owned-effect={effect.id}
                    onClick={() => props.onSelectEffect(effect.id)}
                  >
                    <span>{index() + 1}</span>
                    <strong data-no-localize title={effect.label}>{effect.label}</strong>
                    <small>{effect.effect_type}</small>
                  </button>
                )}
              </For>
            </nav>
          </section>

          <Show
            when={selectedEffect()}
            fallback={<p class="sceneSettingsEmpty">Select a cue-owned FX to edit it.</p>}
          >
            <section
              class="sceneSettingsSection sceneFxEditor"
              data-scene-settings-effect-editor={props.editor.effectType}
            >
              <header>
                <strong class="uiMicroLabel">FX editor</strong>
                <span>{props.editor.effectType}</span>
              </header>
              <Show
                when={!["Color", "ColorMapping", "Chaser", "Move"].includes(props.editor.effectType)}
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
              <EffectActionControlsPanel {...props.editor.action} />
            </section>
          </Show>

          <details class="sceneSettingsSection sceneAddFxDisclosure">
            <summary>Add another FX</summary>
            <EffectFamilyChooser
              activeFamily={props.activeFamily}
              onSelectFamily={props.onSelectFamily}
            />
          </details>
        </Show>
      </div>
    </section>
  );
}
