import type { JSX } from "solid-js";
import { AttributeCategoryRail, type AttributeCategoryRow } from "./AttributeCategoryRail";
import type { ControlCategory } from "../uiModes";

interface FaderAttributeEditorPanelProps {
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  onCategory: (category: ControlCategory) => void;
  children: JSX.Element;
}

export function FaderAttributeEditorPanel(props: FaderAttributeEditorPanelProps) {
  return (
    <div class="attributeEditor">
      <AttributeCategoryRail
        categories={props.categories}
        activeCategory={props.activeCategory}
        className="attributeCategoryRail"
        ariaLabel="Attribute category"
        onCategory={props.onCategory}
      />
      <div class="attributeEditorBody">{props.children}</div>
    </div>
  );
}
