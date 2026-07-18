import type { JSX } from "solid-js";
import { AttributeCategoryRail, type AttributeCategoryRow } from "./AttributeCategoryRail";
import type { ControlCategory } from "../uiModes";

interface FaderAttributeEditorPanelProps {
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  targetKind: "fixture" | "group" | "empty";
  targetLabel: string;
  targetDetail: string;
  referenceLabel: string;
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
        <div class={`attributeTargetSummary ${props.targetKind}`}>
          <span>
            <small>{props.targetKind === "group" ? "Group target" : props.targetKind === "fixture" ? "Fixture target" : "No target"}</small>
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
