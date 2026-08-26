import { Index, onCleanup, onMount } from "solid-js";
import type { Accessor } from "solid-js";

export const TIMELINE_ITEM_CONTEXT_MENU_COLLAPSED_HEIGHT = 298;

export interface TimelineItemContextMenuAction {
  id: string;
  label: string;
  disabled: Accessor<boolean>;
  onSelect: () => void | Promise<void>;
  danger?: boolean;
}

export interface TimelineItemContextMenuGroup {
  id: string;
  label: string;
  actions: TimelineItemContextMenuAction[];
}

export interface TimelineItemContextMenuProps {
  x: number;
  y: number;
  groups: Accessor<TimelineItemContextMenuGroup[]>;
  deleteAction: TimelineItemContextMenuAction;
  onDismiss: (restoreFocus: boolean) => void;
}

const actionDisabled = (action: TimelineItemContextMenuAction) => action.disabled();

export function TimelineItemContextMenu(props: TimelineItemContextMenuProps) {
  let menuElement: HTMLDivElement | undefined;

  onMount(() => {
    const closeFromOutside = (event: PointerEvent) => {
      if (event.defaultPrevented) return;
      const target = event.target;
      if (target instanceof Node && menuElement?.contains(target)) return;
      props.onDismiss(false);
    };
    const closeFromEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || event.defaultPrevented) return;
      if (document.querySelector('.timelineOverview[data-timeline-gesture-active="true"]')) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      props.onDismiss(true);
    };

    window.addEventListener("pointerdown", closeFromOutside, { capture: true });
    window.addEventListener("keydown", closeFromEscape, { capture: true });
    onCleanup(() => {
      window.removeEventListener("pointerdown", closeFromOutside, { capture: true });
      window.removeEventListener("keydown", closeFromEscape, { capture: true });
    });
  });

  return (
    <div
      ref={(element) => { menuElement = element; }}
      class="timelineItemContextMenu"
      role="group"
      aria-label="Timeline item actions"
      style={{
        left: `${props.x}px`,
        top: `${props.y}px`,
        "max-height": `calc(100vh - ${props.y + 8}px)`,
      }}
      onContextMenu={(event) => event.preventDefault()}
    >
      <Index each={props.groups()}>
        {(group) => (
          <details class="timelineItemContextMenuGroup" data-timeline-context-menu-group={group().id}>
            <summary>{group().label}</summary>
            <div class="timelineItemContextMenuGroupActions">
              <Index each={group().actions}>
                {(action) => (
                  <button
                    type="button"
                    data-timeline-context-menu-action={action().id}
                    data-timeline-split-action={action().id === "split-at-playhead" ? "" : undefined}
                    class={action().danger ? "danger" : undefined}
                    disabled={actionDisabled(action())}
                    onClick={() => { void action().onSelect(); }}
                  >
                    {action().label}
                  </button>
                )}
              </Index>
            </div>
          </details>
        )}
      </Index>
      <button
        type="button"
        data-timeline-context-menu-action={props.deleteAction.id}
        class={props.deleteAction.danger ? "danger" : undefined}
        disabled={actionDisabled(props.deleteAction)}
        onClick={() => { void props.deleteAction.onSelect(); }}
      >
        {props.deleteAction.label}
      </button>
      <button
        type="button"
        data-timeline-context-menu-action="close"
        onClick={() => props.onDismiss(true)}
      >
        Close
      </button>
    </div>
  );
}
