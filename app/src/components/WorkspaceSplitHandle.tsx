import { createEffect, onCleanup, onMount } from "solid-js";

type WorkspaceSplitHandleProps = {
  axis: "horizontal" | "vertical";
  ratio: number;
  defaultRatio: number;
  minFirstPx: number;
  minSecondPx: number;
  firstTrackBonusPx?: number;
  label: string;
  splitter: "upper-lower" | "lower-left-right";
  onCommit: (ratio: number) => void;
};

type SplitGeometry = {
  contentStartClientPx: number;
  gapLayoutPx: number;
  handleLayoutPx: number;
  minimumRatio: number;
  maximumRatio: number;
  usableLayoutPx: number;
  visualScale: number;
};

type SplitDrag = {
  pointerId: number;
  startRatio: number;
  currentRatio: number;
  parent: HTMLElement;
};

const POINTER_COMMIT_EPSILON_PX = 0.5;
const KEYBOARD_STEP_PX = 16;
const KEYBOARD_LARGE_STEP_PX = 48;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, value));

const finiteCssPixels = (value: string) => {
  const parsed = Number.parseFloat(value);
  return Number.isFinite(parsed) ? parsed : 0;
};

export function WorkspaceSplitHandle(props: WorkspaceSplitHandleProps) {
  let handle: HTMLButtonElement | undefined;
  let drag: SplitDrag | null = null;
  let resizeObserver: ResizeObserver | undefined;

  const splitGeometry = (parent: HTMLElement): SplitGeometry => {
    const horizontal = props.axis === "horizontal";
    const parentRect = parent.getBoundingClientRect();
    const parentStyle = getComputedStyle(parent);
    const parentLayoutPx = horizontal ? parent.offsetHeight : parent.offsetWidth;
    const parentClientPx = horizontal ? parent.clientHeight : parent.clientWidth;
    const parentVisualPx = horizontal ? parentRect.height : parentRect.width;
    const visualScale = parentVisualPx / Math.max(1, parentLayoutPx);
    const paddingStartLayoutPx = finiteCssPixels(horizontal ? parentStyle.paddingTop : parentStyle.paddingLeft);
    const paddingEndLayoutPx = finiteCssPixels(horizontal ? parentStyle.paddingBottom : parentStyle.paddingRight);
    const contentLayoutPx = Math.max(1, parentClientPx - paddingStartLayoutPx - paddingEndLayoutPx);
    const handleLayoutPx = horizontal ? (handle?.offsetHeight ?? 0) : (handle?.offsetWidth ?? 0);
    const gapLayoutPx = finiteCssPixels(horizontal ? parentStyle.rowGap : parentStyle.columnGap);
    const firstTrackBonusPx = Math.max(0, props.firstTrackBonusPx ?? 0);
    const usableLayoutPx = Math.max(
      1,
      contentLayoutPx - handleLayoutPx - gapLayoutPx * 2 - firstTrackBonusPx,
    );
    const rawMinimum = Math.max(0, props.minFirstPx - firstTrackBonusPx) / usableLayoutPx;
    const rawMaximum = 1 - props.minSecondPx / usableLayoutPx;
    let minimumRatio: number;
    let maximumRatio: number;
    if (rawMinimum > rawMaximum) {
      const fixed = props.minFirstPx / Math.max(1, props.minFirstPx + props.minSecondPx);
      minimumRatio = fixed;
      maximumRatio = fixed;
    } else {
      minimumRatio = clamp(rawMinimum, 0.05, 0.95);
      maximumRatio = clamp(rawMaximum, 0.05, 0.95);
    }
    const clientBorderStartLayoutPx = horizontal ? parent.clientTop : parent.clientLeft;
    const parentStartClientPx = horizontal ? parentRect.top : parentRect.left;
    return {
      contentStartClientPx:
        parentStartClientPx + (clientBorderStartLayoutPx + paddingStartLayoutPx) * visualScale,
      gapLayoutPx,
      handleLayoutPx,
      minimumRatio,
      maximumRatio,
      usableLayoutPx,
      visualScale: Math.max(Number.EPSILON, visualScale),
    };
  };

  const updateAriaValues = (minimum: number, maximum: number, ratio: number) => {
    if (!handle) return;
    const minimumPercent = Math.round(minimum * 100);
    const maximumPercent = Math.round(maximum * 100);
    const ratioPercent = clamp(Math.round(ratio * 100), minimumPercent, maximumPercent);
    handle.setAttribute("aria-valuemin", String(minimumPercent));
    handle.setAttribute("aria-valuemax", String(maximumPercent));
    handle.setAttribute("aria-valuenow", String(ratioPercent));
    handle.setAttribute("aria-valuetext", `${ratioPercent}%`);
  };

  const applyVisualRatio = (parent: HTMLElement, ratio: number) => {
    const geometry = splitGeometry(parent);
    const nextRatio = clamp(ratio, geometry.minimumRatio, geometry.maximumRatio);
    const firstTrackBonusPx = Math.max(0, props.firstTrackBonusPx ?? 0);
    // Resolve the preferred ratio to concrete track sizes. With `minmax(min, fr)`
    // CSS Grid can freeze one track at its minimum and give all spare pixels to
    // the other track, which makes the rendered boundary diverge from the clamp.
    // ResizeObserver reapplies these layout-pixel sizes whenever the root changes.
    parent.style.setProperty(
      "--workspace-first-grow",
      `${firstTrackBonusPx + nextRatio * geometry.usableLayoutPx}px`,
    );
    parent.style.setProperty(
      "--workspace-second-grow",
      `${(1 - nextRatio) * geometry.usableLayoutPx}px`,
    );
    // `ratio` remains the persisted preference. The T25 Control bonus belongs
    // only to the first visual track, so expose the actual rendered boundary to
    // assistive technology without overwriting the stored preference.
    const visualUsableLayoutPx = firstTrackBonusPx + geometry.usableLayoutPx;
    const visualRatio = (firstTrackBonusPx + nextRatio * geometry.usableLayoutPx) /
      visualUsableLayoutPx;
    const visualMinimum = (firstTrackBonusPx + geometry.minimumRatio * geometry.usableLayoutPx) /
      visualUsableLayoutPx;
    const visualMaximum = (firstTrackBonusPx + geometry.maximumRatio * geometry.usableLayoutPx) /
      visualUsableLayoutPx;
    updateAriaValues(visualMinimum, visualMaximum, visualRatio);
    return nextRatio;
  };

  const pointerRatio = (event: PointerEvent, parent: HTMLElement) => {
    const geometry = splitGeometry(parent);
    const clientCoordinate = props.axis === "horizontal" ? event.clientY : event.clientX;
    const layoutCoordinate =
      (clientCoordinate - geometry.contentStartClientPx) / geometry.visualScale;
    return (
      layoutCoordinate -
      geometry.gapLayoutPx -
      geometry.handleLayoutPx / 2 -
      Math.max(0, props.firstTrackBonusPx ?? 0)
    ) / geometry.usableLayoutPx;
  };

  const ratioChanged = (parent: HTMLElement, first: number, second: number) =>
    Math.abs(first - second) * splitGeometry(parent).usableLayoutPx >= POINTER_COMMIT_EPSILON_PX;

  const releasePointerCapture = (pointerId: number) => {
    if (handle?.hasPointerCapture(pointerId)) {
      handle.releasePointerCapture(pointerId);
    }
  };

  const cancelDrag = () => {
    const activeDrag = drag;
    if (!activeDrag) return;
    drag = null;
    applyVisualRatio(activeDrag.parent, activeDrag.startRatio);
    handle?.removeAttribute("data-dragging");
    releasePointerCapture(activeDrag.pointerId);
  };

  const finishDrag = () => {
    const activeDrag = drag;
    if (!activeDrag) return;
    drag = null;
    handle?.removeAttribute("data-dragging");
    releasePointerCapture(activeDrag.pointerId);
    if (ratioChanged(activeDrag.parent, activeDrag.startRatio, activeDrag.currentRatio)) {
      props.onCommit(Number(activeDrag.currentRatio.toFixed(6)));
    }
  };

  const handleWindowKeyDown = (event: KeyboardEvent) => {
    if (event.key !== "Escape" || !drag) return;
    event.preventDefault();
    // The persistent Timeline pane also owns Escape. While a split gesture is
    // active, cancellation must win without collapsing that pane as a side effect.
    event.stopImmediatePropagation();
    cancelDrag();
  };

  window.addEventListener("keydown", handleWindowKeyDown, true);

  onMount(() => {
    const splitter = handle;
    const parent = splitter?.parentElement;
    if (!splitter || !parent) return;
    applyVisualRatio(parent, props.ratio);
    resizeObserver = new ResizeObserver(() => {
      const currentDrag = drag;
      const ratio = currentDrag?.currentRatio ?? props.ratio;
      const clampedRatio = applyVisualRatio(parent, ratio);
      if (currentDrag) currentDrag.currentRatio = clampedRatio;
    });
    resizeObserver.observe(parent);
    // A pane focus mode can hide this splitter (7px -> 0) without resizing the
    // parent. Observe the handle too so its 0 -> 7px restoration recomputes the
    // concrete tracks from the persisted preferred ratio.
    resizeObserver.observe(splitter);
  });

  createEffect(() => {
    const committedRatio = props.ratio;
    const parent = handle?.parentElement;
    if (!parent || drag) return;
    applyVisualRatio(parent, committedRatio);
  });

  onCleanup(() => {
    resizeObserver?.disconnect();
    cancelDrag();
    window.removeEventListener("keydown", handleWindowKeyDown, true);
  });

  const commitKeyboardRatio = (event: KeyboardEvent) => {
    const parent = handle?.parentElement;
    if (!parent || drag) return;
    const geometry = splitGeometry(parent);
    const current = clamp(props.ratio, geometry.minimumRatio, geometry.maximumRatio);
    const stepRatio = (event.shiftKey ? KEYBOARD_LARGE_STEP_PX : KEYBOARD_STEP_PX) /
      geometry.usableLayoutPx;
    let next: number | null = null;
    if (event.key === "Home") next = geometry.minimumRatio;
    if (event.key === "End") next = geometry.maximumRatio;
    if (props.axis === "horizontal" && event.key === "ArrowUp") next = current - stepRatio;
    if (props.axis === "horizontal" && event.key === "ArrowDown") next = current + stepRatio;
    if (props.axis === "vertical" && event.key === "ArrowLeft") next = current - stepRatio;
    if (props.axis === "vertical" && event.key === "ArrowRight") next = current + stepRatio;
    if (next === null) return;
    event.preventDefault();
    // App-level Mapping hotkeys also consume Arrow keys to nudge fixtures.
    // A focused separator owns handled resize keys exclusively.
    event.stopPropagation();
    const committed = applyVisualRatio(parent, next);
    if (ratioChanged(parent, current, committed)) {
      props.onCommit(Number(committed.toFixed(6)));
    }
  };

  return (
    <button
      ref={(element) => { handle = element; }}
      type="button"
      class={`workspaceSplitHandle workspaceSplitHandle-${props.axis}`}
      role="separator"
      aria-label={props.label}
      aria-orientation={props.axis}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={Math.round(props.ratio * 100)}
      aria-valuetext={`${Math.round(props.ratio * 100)}%`}
      title="Drag to resize. Double-click to reset pane sizes."
      data-workspace-splitter={props.splitter}
      data-min-before-px={props.minFirstPx}
      data-min-after-px={props.minSecondPx}
      onPointerDown={(event) => {
        if (event.button !== 0 || !event.isPrimary) return;
        const parent = event.currentTarget.parentElement;
        if (!parent) return;
        event.preventDefault();
        event.currentTarget.focus({ preventScroll: true });
        const startRatio = applyVisualRatio(parent, props.ratio);
        drag = {
          pointerId: event.pointerId,
          startRatio,
          currentRatio: startRatio,
          parent,
        };
        event.currentTarget.setPointerCapture(event.pointerId);
        event.currentTarget.setAttribute("data-dragging", "true");
      }}
      onPointerMove={(event) => {
        if (!drag || drag.pointerId !== event.pointerId) return;
        drag.currentRatio = applyVisualRatio(drag.parent, pointerRatio(event, drag.parent));
      }}
      onPointerUp={(event) => {
        if (!drag || drag.pointerId !== event.pointerId) return;
        drag.currentRatio = applyVisualRatio(drag.parent, pointerRatio(event, drag.parent));
        finishDrag();
      }}
      onPointerCancel={(event) => {
        if (!drag || drag.pointerId !== event.pointerId) return;
        cancelDrag();
      }}
      onLostPointerCapture={(event) => {
        if (!drag || drag.pointerId !== event.pointerId) return;
        cancelDrag();
      }}
      onDblClick={(event) => {
        event.preventDefault();
        cancelDrag();
        const parent = event.currentTarget.parentElement;
        if (!parent) return;
        applyVisualRatio(parent, props.defaultRatio);
        // Preserve the preferred default even when the current viewport clamps
        // it visually. A later larger window can then restore the 58/42 split.
        if (Math.abs(props.ratio - props.defaultRatio) > Number.EPSILON) {
          props.onCommit(Number(props.defaultRatio.toFixed(6)));
        }
      }}
      onKeyDown={commitKeyboardRatio}
    />
  );
}
