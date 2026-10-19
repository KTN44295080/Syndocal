import { createSignal } from "solid-js";
import type { VideoIsfControlSummary } from "../types";

const clamp = (value: number) => Math.max(0, Math.min(1, value));

/** A two-axis pad for a point2D control. A drag only previews locally and
 * publishes once on release, matching the sliders; the arrow keys publish one
 * step each (Shift moves ten times as far). Y grows upward like ISF. */
export function VideoFxPointPad(props: {
  control: VideoIsfControlSummary;
  label: string;
  disabled?: boolean;
  onCommit: (x: number, y: number) => void;
}) {
  let pad!: HTMLDivElement;
  const [drag, setDrag] = createSignal<[number, number] | null>(null);
  const span = (axis: 0 | 1) => props.control.maximum[axis] - props.control.minimum[axis];
  const current = (axis: 0 | 1) => drag()?.[axis] ?? props.control.value[axis];
  const fraction = (axis: 0 | 1) => span(axis) > 0 ? clamp((current(axis) - props.control.minimum[axis]) / span(axis)) : 0;
  const pointer = (event: PointerEvent): [number, number] => {
    const rect = pad.getBoundingClientRect();
    const fx = clamp((event.clientX - rect.left) / Math.max(1, rect.width));
    const fy = clamp(1 - (event.clientY - rect.top) / Math.max(1, rect.height));
    return [props.control.minimum[0] + fx * span(0), props.control.minimum[1] + fy * span(1)];
  };
  const format = (value: number) => String(Number(value.toFixed(2)));
  return (
    <div
      ref={pad} class="videoFxPad" tabIndex={props.disabled ? -1 : 0} role="application" aria-roledescription="2D pad" aria-label={props.label}
      aria-disabled={props.disabled ? "true" : undefined} title={`X ${format(current(0))}, Y ${format(current(1))}`}
      data-fx-point-pad
      onPointerDown={(event) => {
        if (props.disabled || event.button !== 0) return;
        pad.setPointerCapture(event.pointerId); pad.focus(); setDrag(pointer(event));
      }}
      onPointerMove={(event) => { if (drag()) setDrag(pointer(event)); }}
      onPointerUp={(event) => {
        const point = drag();
        if (!point) return;
        pad.releasePointerCapture(event.pointerId); setDrag(null);
        props.onCommit(point[0], point[1]);
      }}
      onPointerCancel={() => setDrag(null)}
      onKeyDown={(event) => {
        if (props.disabled || !["ArrowLeft", "ArrowRight", "ArrowUp", "ArrowDown"].includes(event.key)) return;
        event.preventDefault(); event.stopPropagation();
        const step = (event.shiftKey ? 0.1 : 0.01);
        const dx = event.key === "ArrowRight" ? step : event.key === "ArrowLeft" ? -step : 0;
        const dy = event.key === "ArrowUp" ? step : event.key === "ArrowDown" ? -step : 0;
        props.onCommit(props.control.value[0] + dx * span(0), props.control.value[1] + dy * span(1));
      }}
    >
      <span class="videoFxPadCrossH" style={{ top: `${(1 - fraction(1)) * 100}%` }} aria-hidden="true" />
      <span class="videoFxPadCrossV" style={{ left: `${fraction(0) * 100}%` }} aria-hidden="true" />
      <span class="videoFxPadDot" style={{ left: `${fraction(0) * 100}%`, top: `${(1 - fraction(1)) * 100}%` }} aria-hidden="true" />
    </div>
  );
}
