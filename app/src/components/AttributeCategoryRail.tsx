import { For, Show } from "solid-js";
import type { ControlCategory } from "../uiModes";

export interface AttributeCategoryRow {
  id: ControlCategory;
  label: string;
  count: number;
  hasVisual: boolean;
  hasWritten?: boolean;
}

interface AttributeCategoryRailProps {
  categories: AttributeCategoryRow[];
  activeCategory: ControlCategory;
  className: string;
  ariaLabel: string;
  showWrittenState?: boolean;
  onCategory: (category: ControlCategory) => void;
}

export function AttributeCategoryRail(props: AttributeCategoryRailProps) {
  return (
    <nav class={props.className} aria-label={props.ariaLabel}>
      <For each={props.categories}>
        {(category) => (
          <button
            classList={{
              active: props.activeCategory === category.id,
              written: Boolean(props.showWrittenState && category.hasWritten),
            }}
            disabled={!category.hasVisual && category.count === 0}
            onClick={() => props.onCategory(category.id)}
            aria-pressed={props.activeCategory === category.id}
            title={props.showWrittenState ? (category.hasWritten ? "Written values" : "No written values") : undefined}
          >
            <span>
              <Show when={props.showWrittenState}>
                <i class="attributeCategoryWriteDot" aria-hidden="true" />
              </Show>
              {category.label}
            </span>
            <small>{category.count}</small>
          </button>
        )}
      </For>
    </nav>
  );
}
