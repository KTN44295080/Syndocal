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
      <div class="topbarToolsPopover" role="group" aria-label="Show tools">
        {props.children}
      </div>
    </details>
  );
}
