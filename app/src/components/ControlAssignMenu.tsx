import { For, Show, onCleanup, onMount } from "solid-js";
import { beatDivisions } from "../controlMappingPerform";
import "./ControlAssignMenu.css";

export interface ControlAssignment {
  /** The stored mapping this row represents; removal is by identity. */
  mapping: unknown;
  kind: "MIDI" | "Beat division";
  /** A controller description (kept as typed) or a beat division label (translated). */
  detail: string;
  beat: boolean;
}

export interface ControlAssignMenuProps {
  x: number;
  y: number;
  /** The control's name as the operator sees it. */
  label: string;
  /** Beat divisions fire triggers (clip, Take, Stop, effect stage); a continuous control only learns. */
  beatEligible: boolean;
  assignments: ControlAssignment[];
  onLearn: () => void;
  onBeat: (division: number) => void;
  onRemove: (assignment: ControlAssignment) => void;
  onDismiss: () => void;
}

/** Source choice for one control in Learn mode: the ordinary MIDI Learn or a
 * tempo division, with the assignments the control already has. */
export function ControlAssignMenu(props: ControlAssignMenuProps) {
  let menu: HTMLDivElement | undefined;
  onMount(() => {
    // Keep the menu inside the viewport; it opens at the pointer.
    if (menu) {
      const bounds = menu.getBoundingClientRect();
      const left = Math.max(8, Math.min(props.x, window.innerWidth - bounds.width - 8));
      const top = Math.max(8, Math.min(props.y, window.innerHeight - bounds.height - 8));
      menu.style.left = `${left}px`;
      menu.style.top = `${top}px`;
      menu.querySelector<HTMLButtonElement>("button:not(:disabled)")?.focus({ preventScroll: true });
    }
    const outside = (event: PointerEvent) => {
      if (event.target instanceof Node && menu?.contains(event.target)) return;
      props.onDismiss();
    };
    // Escape closes only the menu; a second Escape leaves Learn.
    const escape = (event: KeyboardEvent) => {
      if (event.key !== "Escape") return;
      event.preventDefault();
      event.stopImmediatePropagation();
      props.onDismiss();
    };
    const arrows = (event: KeyboardEvent) => {
      if (event.key !== "ArrowDown" && event.key !== "ArrowUp") return;
      const items = [...(menu?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [])];
      if (items.length === 0) return;
      event.preventDefault();
      const index = items.indexOf(document.activeElement as HTMLButtonElement);
      const next = event.key === "ArrowDown" ? (index + 1) % items.length : (index - 1 + items.length) % items.length;
      items[next].focus();
    };
    window.addEventListener("pointerdown", outside, true);
    window.addEventListener("keydown", escape, true);
    window.addEventListener("keydown", arrows);
    onCleanup(() => {
      window.removeEventListener("pointerdown", outside, true);
      window.removeEventListener("keydown", escape, true);
      window.removeEventListener("keydown", arrows);
    });
  });
  return (
    <div ref={(element) => { menu = element; }} class="controlAssignMenu" role="menu" aria-label="Assign source"
      data-control-assign-menu style={{ left: `${props.x}px`, top: `${props.y}px` }}
      onContextMenu={(event) => event.preventDefault()}>
      <header class="controlAssignMenuHead">
        <span class="controlAssignMenuKicker">Assign source</span>
        <strong data-no-localize title={props.label}>{props.label}</strong>
      </header>
      <button type="button" role="menuitem" data-control-assign-learn onClick={() => props.onLearn()}>MIDI Learn</button>
      <Show when={props.beatEligible} fallback={
        <p class="controlAssignMenuNote">Beat divisions fire triggers only. This control follows its MIDI input.</p>
      }>
        <div class="controlAssignMenuGroup" role="group" aria-label="Beat division">
          <span class="controlAssignMenuGroupLabel">Beat division</span>
          <div class="controlAssignMenuDivisions">
            <For each={beatDivisions}>{(division) => (
              <button type="button" role="menuitem" data-control-assign-beat={division.number}
                onClick={() => props.onBeat(division.number)}>{division.label}</button>
            )}</For>
          </div>
          <p class="controlAssignMenuNote">Fires on every beat division of the project clock.</p>
        </div>
      </Show>
      <div class="controlAssignMenuGroup" role="group" aria-label="Current assignments">
        <span class="controlAssignMenuGroupLabel">Current assignments</span>
        <Show when={props.assignments.length > 0} fallback={<p class="controlAssignMenuNote">No assignment yet.</p>}>
          <ul class="controlAssignMenuList">
            <For each={props.assignments}>{(assignment) => (
              <li data-control-assign-row={assignment.beat ? "beat" : "midi"}>
                <span class="controlAssignMenuKind">{assignment.kind}</span>
                <Show when={assignment.beat} fallback={<span data-no-localize>{assignment.detail}</span>}>
                  <span>{assignment.detail}</span>
                </Show>
                <button type="button" role="menuitem" data-control-assign-remove onClick={() => props.onRemove(assignment)}>Remove</button>
              </li>
            )}</For>
          </ul>
        </Show>
      </div>
      <button type="button" role="menuitem" class="controlAssignMenuClose" onClick={() => props.onDismiss()}>Close</button>
    </div>
  );
}
