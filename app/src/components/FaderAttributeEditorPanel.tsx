import type { JSX } from "solid-js";
import { AttributeCategoryRail, type AttributeCategoryRow } from "./AttributeCategoryRail";
import type { ControlCategory } from "../uiModes";

interface FaderAttributeEditorPanelProps {
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  targetKind: "fixture" | "group" | "selection" | "empty";
  targetLabel: string;
  targetDetail: string;
  referenceLabel: string;
  writeMode: "live" | "edit";
  editingSceneLabel: string | null;
  editingSceneIdentity: string | null;
  editingSceneIdentityText: string | null;
  onWriteMode: (mode: "live" | "edit") => void;
  onCategory: (category: ControlCategory) => void;
  children: JSX.Element;
}

export function FaderAttributeEditorPanel(props: FaderAttributeEditorPanelProps) {
  return (
    <div class="attributeEditor fixtureEditSurface">
      <AttributeCategoryRail
        categories={props.categories}
        activeCategory={props.activeCategory}
        className="attributeCategoryRail"
        ariaLabel="Attribute category"
        showWrittenState
        onCategory={props.onCategory}
      />
      <div class="attributeEditorBody">
        <div
          class={`attributeTargetSummary controlFaderWriteSummary ${props.targetKind}`}
          data-control-fader-write-mode={props.writeMode}
          style={{
            "--control-edit-identity": props.editingSceneIdentity ?? "#4dbb78",
            "--control-edit-identity-text": props.editingSceneIdentityText ?? "#b8e8ca",
          }}
        >
          <div
            class={`controlFaderWriteHeader ${props.writeMode}`}
            data-control-fader-write-header
          >
            <div
              class="controlFaderWriteMode"
              role="group"
              aria-label="Fader write mode"
            >
              <button
                type="button"
                classList={{ active: props.writeMode === "live" }}
                aria-label="Use LIVE fader mode"
                aria-pressed={props.writeMode === "live"}
                data-control-fader-write-mode-option="live"
                onClick={() => props.onWriteMode("live")}
              >
                <span data-no-localize>LIVE</span>
              </button>
              <button
                type="button"
                classList={{ active: props.writeMode === "edit" }}
                aria-label="Use EDIT scene-write mode"
                aria-pressed={props.writeMode === "edit"}
                data-control-fader-write-mode-option="edit"
                onClick={() => props.onWriteMode("edit")}
              >
                <span data-no-localize>EDIT</span>
              </button>
            </div>
            <div
              class="controlFaderEditState"
              classList={{ unselected: !props.editingSceneLabel }}
              data-control-fader-edit-state={props.editingSceneLabel ? "selected" : "unselected"}
              aria-live="polite"
            >
              <strong data-no-localize>EDIT:</strong>
              {props.editingSceneLabel
                ? <span data-no-localize>{props.editingSceneLabel}</span>
                : <span>No scene selected</span>}
            </div>
          </div>
          <span>
            <small>
              {props.targetKind === "selection"
                ? "Picked fixtures"
                : props.targetKind === "group"
                  ? "Group target"
                  : props.targetKind === "fixture"
                    ? "Fixture target"
                    : "No target"}
            </small>
            <strong>{props.targetLabel}</strong>
          </span>
          <span>
            <small>Scope</small>
            <strong>{props.targetDetail}</strong>
          </span>
          <span>
            <small>Readout</small>
            <strong>{props.referenceLabel}</strong>
          </span>
        </div>
        <div class={`attributeDeskSurface category-${props.activeCategory}`} aria-label="Attribute fader desk">
          {props.children}
        </div>
      </div>
    </div>
  );
}
