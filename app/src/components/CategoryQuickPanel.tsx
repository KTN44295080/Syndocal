import { For, Show } from "solid-js";

export interface CategoryQuickLook {
  id: string;
  label: string;
  description: string;
}

export type CategoryQuickValueMode = "zero" | "mid" | "full" | "default";

interface CategoryQuickAction {
  mode: CategoryQuickValueMode;
  label: string;
  valueLabel: string;
  primary?: boolean;
}

const categoryQuickActions: CategoryQuickAction[] = [
  { mode: "zero", label: "Zero", valueLabel: "0%" },
  { mode: "mid", label: "Mid", valueLabel: "50%" },
  { mode: "full", label: "Full", valueLabel: "100%" },
  { mode: "default", label: "Default", valueLabel: "Profile", primary: true },
];

interface CategoryQuickPanelProps {
  categoryLabel: string;
  attributeCount: number;
  looks: CategoryQuickLook[];
  onSetValueMode: (mode: CategoryQuickValueMode) => void;
  onApplyLook: (look: CategoryQuickLook) => void;
}

export function CategoryQuickPanel(props: CategoryQuickPanelProps) {
  return (
    <div class="visualControlPanel categoryQuickPanel">
      <div class="visualControlHeader">
        <div>
          <strong>Category Actions</strong>
          <span>{props.attributeCount} attribute(s)</span>
        </div>
        <span>{props.categoryLabel}</span>
      </div>
      <div class="categoryQuickRow">
        <For each={categoryQuickActions}>
          {(action) => (
            <button
              type="button"
              class={action.primary ? "primary" : ""}
              onClick={() => props.onSetValueMode(action.mode)}
            >
              <strong>{action.label}</strong>
              <small>{action.valueLabel}</small>
            </button>
          )}
        </For>
      </div>
      <Show when={props.looks.length > 0}>
        <div class="categoryLookGrid">
          <For each={props.looks}>
            {(look) => (
              <button type="button" title={look.description} onClick={() => props.onApplyLook(look)}>
                <strong>{look.label}</strong>
                <small>{look.description}</small>
              </button>
            )}
          </For>
        </div>
      </Show>
    </div>
  );
}
