import { onCleanup, type JSX } from "solid-js";

export function TopbarTools(props: { active: boolean; children: JSX.Element }) {
  let root!: HTMLDetailsElement;
  let trigger!: HTMLElement;
  const close = () => { root.open = false; };
  const onPointerDown = (event: PointerEvent) => {
    if (event.target instanceof Node && !root.contains(event.target)) close();
  };
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !root.open || event.defaultPrevented || document.querySelector("dialog[open]")) return;
    // BPM owns Escape while editing. Closing Tools first would blur the input
    // and commit the draft instead of letting the input cancel it.
    if (event.target instanceof HTMLInputElement && event.target.matches(".topbarBpmInput") && root.contains(event.target)) return;
    event.preventDefault();
    event.stopImmediatePropagation();
    close();
    trigger.focus();
  };
  window.addEventListener("pointerdown", onPointerDown);
  window.addEventListener("keydown", onKeyDown, true);
  onCleanup(() => {
    window.removeEventListener("pointerdown", onPointerDown);
    window.removeEventListener("keydown", onKeyDown, true);
  });
  return (
    <details ref={root} class="topbarTools" classList={{ active: props.active }}>
      <summary ref={trigger} aria-label="Show tools" title="Playback, control mapping and audio input">Tools</summary>
      <div class="topbarToolsPopover" role="group" aria-label="Show tools" onClick={event => {
        // A Learn choice exposes the controls to map and its instruction row.
        if (event.target instanceof Element && event.target.closest("[data-control-learn-toggle]")) {
          close();
          trigger.focus();
        }
      }}>
        {props.children}
      </div>
    </details>
  );
}
