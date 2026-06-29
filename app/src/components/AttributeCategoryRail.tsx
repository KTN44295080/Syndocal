import { For } from "solid-js";
import type { ControlCategory } from "../uiModes";

export interface AttributeCategoryRow {
  id: ControlCategory;
  label: string;
  count: number;
  hasVisual: boolean;
}

interface AttributeCategoryRailProps {
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  className: string;
  ariaLabel: string;
  onCategory: (category: ControlCategory) => void;
}

export function AttributeCategoryRail(props: AttributeCategoryRailProps) {
  return (
    <nav class={props.className} aria-label={props.ariaLabel}>
      <For each={props.categories}>
        {(category) => (
          <button
            class={props.activeCategory === category.id ? "active" : ""}
            disabled={!category.hasVisual && category.count === 0}
            onClick={() => props.onCategory(category.id)}
            aria-pressed={props.activeCategory === category.id}
          >
            <span>{category.label}</span>
            <small>{category.count}</small>
          </button>
        )}
      </For>
    </nav>
  );
}
