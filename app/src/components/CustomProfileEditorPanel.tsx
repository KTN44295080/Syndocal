import { For, Show } from "solid-js";
import type {
  CustomProfileAttributeDraft,
  CustomProfileAttributeTemplate,
  CustomProfilePreview,
} from "../customFixtureProfile";
import type { AttributeResolution } from "../types";

interface CustomProfileDmxCellControl {
  attribute: string;
  resolution: AttributeResolution;
  geometry: "Body" | "Head" | "Beam";
}

export interface CustomProfileDmxCell {
  channel: number;
  control: CustomProfileDmxCellControl | null;
  controlIndex: number;
  selected: boolean;
  conflict: boolean;
}

interface CustomProfileEditorPanelProps {
  manufacturer: string;
  profileName: string;
  modeName: string;
  attributesText: string;
  templates: CustomProfileAttributeTemplate[];
  drafts: CustomProfileAttributeDraft[];
  selectedIndex: number | null;
  rowConflicts: Set<number>;
  dmxCells: CustomProfileDmxCell[];
  preview: CustomProfilePreview;
  rowStatusText: (index: number) => string;
  onManufacturer: (value: string) => void;
  onProfileName: (value: string) => void;
  onModeName: (value: string) => void;
  onAttributesText: (value: string) => void;
  onAppendTemplate: (rows: CustomProfileAttributeDraft[]) => void;
  onSelectIndex: (index: number | null) => void;
  onUpdateDraft: (index: number, updates: Partial<CustomProfileAttributeDraft>) => void;
  onRemoveDraft: (index: number) => void;
  onAddDraft: () => void;
  onMoveDraft: (index: number, direction: -1 | 1) => void;
  onClearDrafts: () => void;
  onCreate: () => void | Promise<void>;
  onSave: () => void | Promise<void>;
  onLoad: () => void | Promise<void>;
}

export function CustomProfileEditorPanel(props: CustomProfileEditorPanelProps) {
  const canCreate = () => props.preview.errors.length === 0 && props.preview.controls.length > 0;

  return (
    <div class="customProfileForm">
      <header class="customProfileWorkspaceHeader">
        <h3>Custom Profile</h3>
        <div class="buttonRow customProfileActions">
          <button onClick={props.onLoad}>Load</button>
          <button onClick={props.onSave} disabled={!canCreate()}>Save</button>
          <button class="primary" onClick={props.onCreate} disabled={!canCreate()}>Create Profile</button>
        </div>
      </header>
      <div class="customProfileMetadataGrid">
        <label>
          Maker
          <input value={props.manufacturer} onInput={(event) => props.onManufacturer(event.currentTarget.value)} />
        </label>
        <label>
          Name
          <input value={props.profileName} onInput={(event) => props.onProfileName(event.currentTarget.value)} />
        </label>
        <label>
          Mode
          <input value={props.modeName} onInput={(event) => props.onModeName(event.currentTarget.value)} />
        </label>
        <label class="customProfileRawAttributes">
          Raw Attributes
          <input
            value={props.attributesText}
            onInput={(event) => props.onAttributesText(event.currentTarget.value)}
            placeholder="Dimmer@1:8, Pan@2:16, Tilt@4:16, ColorRed@6:8"
          />
        </label>
      </div>
      <div class="customProfileWorkbench">
        <section class="customProfileEditor customProfileAttributePane">
          <div class="customProfilePreviewHeader">
            <strong>Attributes</strong>
            <span>{props.drafts.length} control(s)</span>
          </div>
        <div class="customProfileTemplateGrid" aria-label="Custom profile attribute templates">
          <For each={props.templates}>
            {(template) => (
              <button type="button" onClick={() => props.onAppendTemplate(template.rows)}>
                {template.label}
              </button>
            )}
          </For>
        </div>
        <div class="customProfileAttributeRows">
          <For each={props.drafts}>
            {(draft, index) => (
              <div
                class={[
                  "customProfileAttributeRow",
                  props.selectedIndex === index() ? "selected" : "",
                  props.rowConflicts.has(index()) ? "conflict" : "",
                ].join(" ")}
                onClick={() => props.onSelectIndex(index())}
              >
                <label>
                  Attribute
                  <input
                    value={draft.attribute}
                    onInput={(event) => props.onUpdateDraft(index(), { attribute: event.currentTarget.value })}
                  />
                </label>
                <label>
                  Bits
                  <select
                    value={draft.resolution}
                    onInput={(event) =>
                      props.onUpdateDraft(index(), {
                        resolution: event.currentTarget.value as AttributeResolution,
                      })
                    }
                  >
                    <option value="EightBit">8-bit</option>
                    <option value="SixteenBit">16-bit</option>
                  </select>
                </label>
                <label>
                  Start
                  <input
                    value={draft.startOffset}
                    inputMode="numeric"
                    placeholder="auto"
                    onInput={(event) => props.onUpdateDraft(index(), { startOffset: event.currentTarget.value })}
                  />
                </label>
                <span class="customProfileRowFootprint" title={props.rowStatusText(index())}>
                  {props.rowStatusText(index())}
                </span>
                <button
                  type="button"
                  class="iconButton"
                  title={`Remove ${draft.attribute || "attribute"}`}
                  onClick={(event) => {
                    event.stopPropagation();
                    props.onRemoveDraft(index());
                  }}
                >
                  -
                </button>
              </div>
            )}
          </For>
        </div>
        <div class="buttonRow">
          <button type="button" onClick={props.onAddDraft}>Add Attribute</button>
          <button
            type="button"
            onClick={() => {
              if (props.selectedIndex !== null) {
                props.onMoveDraft(props.selectedIndex, -1);
              }
            }}
            disabled={props.selectedIndex === null || props.selectedIndex === 0}
          >
            Move Up
          </button>
          <button
            type="button"
            onClick={() => {
              if (props.selectedIndex !== null) {
                props.onMoveDraft(props.selectedIndex, 1);
              }
            }}
            disabled={props.selectedIndex === null || props.selectedIndex === props.drafts.length - 1}
          >
            Move Down
          </button>
          <button type="button" onClick={props.onClearDrafts} disabled={props.drafts.length === 0}>
            Clear
          </button>
        </div>
        </section>
        <aside class="customProfilePreviewDesk">
        <div class="customProfileDmxMap">
          <div class="customProfilePreviewHeader">
            <strong>DMX Map</strong>
            <span>{props.preview.footprint || 0} ch</span>
          </div>
          <div class="customProfileDmxGrid" aria-label="Custom profile DMX map">
            <For each={props.dmxCells}>
              {(cell) => (
                <button
                  type="button"
                  class={[
                    "customProfileDmxCell",
                    cell.control ? "" : "empty",
                    cell.selected ? "selected" : "",
                    cell.conflict ? "conflict" : "",
                  ].join(" ")}
                  onClick={() => {
                    if (cell.controlIndex >= 0) {
                      props.onSelectIndex(cell.controlIndex);
                    }
                  }}
                >
                  <small>{cell.channel}</small>
                  <strong>{cell.control?.attribute ?? ""}</strong>
                  <span>
                    {cell.control ? `${cell.control.resolution === "SixteenBit" ? "16" : "8"} / ${cell.control.geometry}` : ""}
                  </span>
                </button>
              )}
            </For>
          </div>
        </div>
        <div class={`customProfilePreview ${props.preview.errors.length > 0 ? "bad" : ""}`}>
        <div class="customProfilePreviewHeader">
          <strong>DMX Footprint</strong>
          <span>{props.preview.footprint} ch</span>
        </div>
        <Show when={props.preview.errors.length > 0}>
          <ul class="warnings">
            <For each={props.preview.errors}>{(error) => <li>{error}</li>}</For>
          </ul>
        </Show>
        <div class="customProfileChannelList">
          <For each={props.preview.controls}>
            {(control) => (
              <div class="customProfileChannel">
                <strong>{control.attribute}</strong>
                <span>{control.resolution === "SixteenBit" ? "16-bit" : "8-bit"}</span>
                <em>{control.geometry}</em>
                <small>CH {control.offsets.join("/")}</small>
              </div>
            )}
          </For>
        </div>
      </div>
        </aside>
      </div>
    </div>
  );
}
