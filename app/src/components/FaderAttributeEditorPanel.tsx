import { Show, type JSX } from "solid-js";
import { handleHorizontalWheel } from "../horizontalWheel";
import { AttributeCategoryRail, type AttributeCategoryRow } from "./AttributeCategoryRail";
import type { ControlCategory } from "../uiModes";

interface FaderAttributeEditorPanelProps {
  disabled?: boolean;
  disabledMessage?: string;
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
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
        <Show when={props.disabled && props.disabledMessage}><p class="hint" data-control-edit-empty>{props.disabledMessage}</p></Show>
        <fieldset
          disabled={props.disabled}
          class={`attributeDeskSurface category-${props.activeCategory}`}
          aria-label="Attribute fader desk"
          data-wheel-scroll-surface="fader-deck"
          onWheel={handleHorizontalWheel}
        >
          {props.children}
        </fieldset>
      </div>
    </div>
  );
}
