import { createMemo, createSignal, For, Show } from "solid-js";
import type {
  ValueEffectDirection,
  ValueEffectInterpolation,
  ValueEffectMode,
  ValueEffectPoint,
} from "../types";

export const valueEffectMaximumPoints = 32;

export interface ValueEffectEditorPanelProps {
  points: ValueEffectPoint[];
  interpolation: ValueEffectInterpolation;
  mode: ValueEffectMode;
  direction: ValueEffectDirection;
  periodMs: number;
  bpm: number;
  /** Null selects the free-running period; a number selects beat synchronization. */
  clockSyncBeats: number | null;
  /** Normalized 0..1 cycle offset. */
  phase: number;
  /** Normalized 0..1 fixture phase spread. */
  spread: number;
  onPoints: (points: ValueEffectPoint[]) => void;
  onInterpolation: (interpolation: ValueEffectInterpolation) => void;
  onMode: (mode: ValueEffectMode) => void;
  onDirection: (direction: ValueEffectDirection) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onPhase: (phase: number) => void;
  onSpread: (spread: number) => void;
}

const interpolationModes: ValueEffectInterpolation[] = ["Step", "Line", "Smooth"];
const modeOptions: ValueEffectMode[] = ["Absolute", "Relative"];
const directionModes: ValueEffectDirection[] = ["Forward", "Reverse", "Bounce"];
const clockPresets = [
  { label: "Free", beats: null },
  { label: "1/4", beats: 0.25 },
  { label: "1/2", beats: 0.5 },
  { label: "1", beats: 1 },
  { label: "2", beats: 2 },
  { label: "4", beats: 4 },
] as const;

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));
const clampUnit = (value: number) => clamp(value, 0, 1);
const round = (value: number) => Number(value.toFixed(4));
const pathNumber = (value: number) => Number(value.toFixed(3));
const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

interface CanvasPoint {
  x: number;
  y: number;
}

const catmullRom = (p0: number, p1: number, p2: number, p3: number, t: number) => {
  const t2 = t * t;
  const t3 = t2 * t;
  return (
    0.5 *
    (2 * p1 +
      (-p0 + p2) * t +
      (2 * p0 - 5 * p1 + 4 * p2 - p3) * t2 +
      (-p0 + 3 * p1 - 3 * p2 + p3) * t3)
  );
};

/** Sample the envelope value in 0..1 at a normalized progress. Mirrors the engine runtime. */
const sampleEnvelope = (
  points: ValueEffectPoint[],
  interpolation: ValueEffectInterpolation,
  progress: number,
): number => {
  if (points.length === 0) return 0;
  const p = clampUnit(progress);
  const first = points[0];
  if (p <= first.position) return clampUnit(first.value);
  const last = points[points.length - 1];
  if (p >= last.position) return clampUnit(last.value);
  let index = 0;
  for (let candidate = 0; candidate < points.length - 1; candidate += 1) {
    if (p >= points[candidate].position && p < points[candidate + 1].position) {
      index = candidate;
      break;
    }
  }
  const left = points[index];
  const right = points[index + 1];
  const span = Math.max(1e-6, right.position - left.position);
  const local = clampUnit((p - left.position) / span);
  if (interpolation === "Step") return clampUnit(left.value);
  if (interpolation === "Line") return clampUnit(left.value + (right.value - left.value) * local);
  const previous = index === 0 ? left.value : points[index - 1].value;
  const next = index + 2 < points.length ? points[index + 2].value : right.value;
  return clampUnit(catmullRom(previous, left.value, right.value, next, local));
};

export function ValueEffectEditorPanel(props: ValueEffectEditorPanelProps) {
  const [selectedPoint, setSelectedPoint] = createSignal(0);
  const [draggingPoint, setDraggingPoint] = createSignal<{ index: number; pointerId: number } | null>(null);

  const activePointIndex = createMemo<number | null>(() => {
    if (props.points.length === 0) return null;
    return Math.min(Math.max(0, selectedPoint()), props.points.length - 1);
  });
  const phasePercent = createMemo(() => Math.round(clampUnit(props.phase) * 100));
  const spreadPercent = createMemo(() => Math.round(clampUnit(props.spread) * 100));
  const normalizedPeriodMs = createMemo(() =>
    Math.max(10, Math.round(Number.isFinite(props.periodMs) ? props.periodMs : 10)),
  );
  const effectiveBpm = createMemo(() => (Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120));
  const beatPeriodMs = (beats: number) => Math.max(10, Math.round((60_000 / effectiveBpm()) * beats));
  const clockSummary = createMemo(() =>
    props.clockSyncBeats === null
      ? `${normalizedPeriodMs()} ms free`
      : `${props.clockSyncBeats} beat / ${beatPeriodMs(props.clockSyncBeats)} ms`,
  );

  const pointToCanvas = (point: ValueEffectPoint): CanvasPoint => ({
    x: clampUnit(point.position) * 100,
    y: (1 - clampUnit(point.value)) * 100,
  });

  // Output preview follows the envelope, then Reverse/Bounce reshaping of the read progress.
  const previewPath = createMemo(() => {
    if (props.points.length < 2) return "";
    const samples: string[] = [];
    for (let step = 0; step <= 64; step += 1) {
      const readProgress = step / 64;
      let progress = readProgress;
      if (props.direction === "Reverse") progress = 1 - readProgress;
      else if (props.direction === "Bounce") {
        const doubled = readProgress * 2;
        progress = doubled <= 1 ? doubled : 2 - doubled;
      }
      const value = sampleEnvelope(props.points, props.interpolation, progress);
      const x = pathNumber(readProgress * 100);
      const y = pathNumber((1 - value) * 100);
      samples.push(`${step === 0 ? "M" : "L"} ${x} ${y}`);
    }
    return samples.join(" ");
  });
  const canvasPoints = createMemo(() => props.points.map(pointToCanvas));

  const normalizePoint = (index: number, point: ValueEffectPoint): ValueEffectPoint => {
    // Keep positions strictly increasing by clamping between neighbors.
    const previous = index > 0 ? props.points[index - 1].position + 0.001 : 0;
    const next = index < props.points.length - 1 ? props.points[index + 1].position - 0.001 : 1;
    const lowerBound = Math.min(previous, next);
    const upperBound = Math.max(previous, next);
    return {
      position: round(clamp(point.position, lowerBound, upperBound)),
      value: round(clampUnit(point.value)),
    };
  };

  const updatePoint = (index: number, point: ValueEffectPoint) => {
    if (!props.points[index]) return;
    const next = props.points.map((candidate) => ({ ...candidate }));
    next[index] = normalizePoint(index, point);
    props.onPoints(next);
  };

  const addPoint = (point?: ValueEffectPoint) => {
    if (props.points.length >= valueEffectMaximumPoints) return;
    const activeIndex = activePointIndex();
    const insertionIndex = activeIndex === null ? props.points.length : activeIndex + 1;
    let candidate: ValueEffectPoint;
    if (point) {
      candidate = point;
    } else {
      const before = props.points[insertionIndex - 1];
      const after = props.points[insertionIndex];
      candidate = {
        position:
          before && after
            ? (before.position + after.position) / 2
            : before
              ? clampUnit(before.position + 0.1)
              : 0.5,
        value: before && after ? (before.value + after.value) / 2 : 0.5,
      };
    }
    const next = props.points.map((existing) => ({ ...existing }));
    next.splice(insertionIndex, 0, { position: clampUnit(candidate.position), value: clampUnit(candidate.value) });
    next.sort((a, b) => a.position - b.position);
    props.onPoints(next);
    const sortedIndex = next.findIndex((entry) => entry.position === clampUnit(candidate.position));
    setSelectedPoint(sortedIndex >= 0 ? sortedIndex : insertionIndex);
  };

  const removePoint = (index: number) => {
    if (!props.points[index] || props.points.length <= 2) return;
    const next = props.points.filter((_, candidate) => candidate !== index).map((point) => ({ ...point }));
    props.onPoints(next);
    setSelectedPoint(Math.max(0, Math.min(index, next.length - 1)));
  };

  const pointFromClient = (svg: SVGSVGElement, clientX: number, clientY: number): ValueEffectPoint => {
    const rectangle = svg.getBoundingClientRect();
    if (rectangle.width <= 0 || rectangle.height <= 0) return { position: 0.5, value: 0.5 };
    return {
      position: clampUnit((clientX - rectangle.left) / rectangle.width),
      value: clampUnit(1 - (clientY - rectangle.top) / rectangle.height),
    };
  };

  const beginPointDrag = (event: PointerEvent & { currentTarget: SVGGElement }, index: number) => {
    event.preventDefault();
    event.stopPropagation();
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) return;
    svg.setPointerCapture(event.pointerId);
    setSelectedPoint(index);
    setDraggingPoint({ index, pointerId: event.pointerId });
  };

  const continuePointDrag = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const dragging = draggingPoint();
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    updatePoint(dragging.index, pointFromClient(event.currentTarget, event.clientX, event.clientY));
  };

  const endPointDrag = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const dragging = draggingPoint();
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
    setDraggingPoint(null);
  };

  const handlePointKeyDown = (event: KeyboardEvent, index: number) => {
    const point = props.points[index];
    if (!point) return;
    const multiplier = event.shiftKey ? 5 : event.altKey ? 0.2 : 1;
    const step = 0.01 * multiplier;
    const next = { ...point };
    if (event.key === "ArrowLeft") next.position -= step;
    else if (event.key === "ArrowRight") next.position += step;
    else if (event.key === "ArrowUp") next.value += step;
    else if (event.key === "ArrowDown") next.value -= step;
    else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      removePoint(index);
      return;
    } else {
      return;
    }
    event.preventDefault();
    updatePoint(index, next);
  };

  return (
    <section class="valueEffectEditor" aria-label="Value effect editor">
      <header class="valueEffectHeader">
        <div>
          <strong class="textBalance">Value FX</strong>
          <span>
            {props.points.length} points / {props.interpolation} / {props.mode}
          </span>
        </div>
        <div class="valueEffectStatusStrip">
          <span>
            <small>Clock</small>
            <strong class="tabularNums">{clockSummary()}</strong>
          </span>
          <span>
            <small>Direction</small>
            <strong>{props.direction}</strong>
          </span>
          <span>
            <small>Phase / Spread</small>
            <strong class="tabularNums">
              {phasePercent()}% / {spreadPercent()}%
            </strong>
          </span>
        </div>
      </header>

      <div class="valueEffectCanvasWrap">
        <div class="valueEffectCanvasToolbar">
          <strong>Envelope</strong>
          <span class="tabularNums">
            {props.points.length} / {valueEffectMaximumPoints}
          </span>
          <button
            type="button"
            onClick={() => addPoint()}
            disabled={props.points.length >= valueEffectMaximumPoints}
          >
            Add point
          </button>
          <button
            type="button"
            onClick={() => {
              const index = activePointIndex();
              if (index !== null) removePoint(index);
            }}
            disabled={activePointIndex() === null || props.points.length <= 2}
          >
            Remove point
          </button>
        </div>
        <svg
          class="valueEffectCanvas"
          viewBox="0 0 100 100"
          preserveAspectRatio="none"
          role="group"
          aria-label={`${props.mode} ${props.interpolation} value envelope with ${props.points.length} points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift coarse, Alt fine, Delete to remove.`}
          onDblClick={(event) => addPoint(pointFromClient(event.currentTarget, event.clientX, event.clientY))}
          onPointerMove={continuePointDrag}
          onPointerUp={endPointDrag}
          onPointerCancel={endPointDrag}
        >
          <rect class="valueEffectCanvasBackground" x="0" y="0" width="100" height="100" />
          <g class="valueEffectCanvasGrid" aria-hidden="true">
            <line x1="25" y1="0" x2="25" y2="100" />
            <line x1="50" y1="0" x2="50" y2="100" />
            <line x1="75" y1="0" x2="75" y2="100" />
            <line x1="0" y1="50" x2="100" y2="50" />
          </g>
          <path class="valueEffectCanvasPath" d={previewPath()} />
          <For each={canvasPoints()}>
            {(point, index) => (
              <g
                class={activePointIndex() === index() ? "valueEffectPointHandle active" : "valueEffectPointHandle"}
                transform={`translate(${pathNumber(point.x)} ${pathNumber(point.y)})`}
                role="button"
                tabIndex={0}
                aria-label={`Envelope point ${index() + 1}, position ${props.points[index()]?.position ?? 0}, value ${props.points[index()]?.value ?? 0}`}
                aria-pressed={activePointIndex() === index()}
                onFocus={() => setSelectedPoint(index())}
                onPointerDown={(event) => beginPointDrag(event, index())}
                onDblClick={(event) => event.stopPropagation()}
                onKeyDown={(event) => handlePointKeyDown(event, index())}
              >
                <title>{`Point ${index() + 1}: ${props.points[index()]?.position ?? 0}, ${props.points[index()]?.value ?? 0}`}</title>
                <circle r="3" vector-effect="non-scaling-stroke" />
              </g>
            )}
          </For>
        </svg>
        <div class="valueEffectCanvasFooter">
          <small>Double-click to add · drag points · Arrow keys nudge · Shift coarse · Alt fine · Delete remove</small>
        </div>
      </div>

      <div class="valueEffectModeRow">
        <div class="valueEffectModeGroup">
          <span>Mode</span>
          <div class="moveEffectSegmented" aria-label="Value mode">
            <For each={modeOptions}>
              {(mode) => (
                <button
                  type="button"
                  class={props.mode === mode ? "active" : ""}
                  aria-pressed={props.mode === mode}
                  title={mode === "Absolute" ? "Envelope maps into low..high" : "Bipolar offset around the incoming value (0.5 = no change)"}
                  onClick={() => props.onMode(mode)}
                >
                  {mode}
                </button>
              )}
            </For>
          </div>
        </div>
        <div class="valueEffectModeGroup">
          <span>Interpolation</span>
          <div class="moveEffectSegmented valueEffectInterpSegmented" aria-label="Value interpolation">
            <For each={interpolationModes}>
              {(interpolation) => (
                <button
                  type="button"
                  class={props.interpolation === interpolation ? "active" : ""}
                  aria-pressed={props.interpolation === interpolation}
                  onClick={() => props.onInterpolation(interpolation)}
                >
                  {interpolation}
                </button>
              )}
            </For>
          </div>
        </div>
      </div>

      <div class="valueEffectTransportRow">
        <div class="moveEffectDirectionGrid" aria-label="Value direction">
          <For each={directionModes}>
            {(direction) => (
              <button
                type="button"
                class={props.direction === direction ? "active" : ""}
                aria-pressed={props.direction === direction}
                onClick={() => props.onDirection(direction)}
              >
                {direction}
              </button>
            )}
          </For>
        </div>
        <label class="valueEffectPeriodField">
          Period ms
          <input
            class="tabularNums"
            type="number"
            min="10"
            step="10"
            value={normalizedPeriodMs()}
            onInput={(event) => props.onPeriodMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))}
          />
        </label>
        <div class="moveEffectClockPresets" aria-label="Value clock sync presets">
          <For each={clockPresets}>
            {(preset) => (
              <button
                type="button"
                class={nearlyEqual(props.clockSyncBeats, preset.beats) ? "active" : ""}
                aria-pressed={nearlyEqual(props.clockSyncBeats, preset.beats)}
                title={preset.beats === null ? `${normalizedPeriodMs()} ms free` : `${beatPeriodMs(preset.beats)} ms at ${effectiveBpm()} BPM`}
                onClick={() => props.onClockSyncBeats(preset.beats)}
              >
                <Show when={preset.beats !== null} fallback="Free">
                  <span data-no-localize>{preset.label}</span>
                </Show>
              </button>
            )}
          </For>
        </div>
      </div>

      <div class="valueEffectPhaseGrid">
        <label>
          <span>Phase</span>
          <input
            type="range"
            min="0"
            max="100"
            step="1"
            value={phasePercent()}
            aria-label="Value phase percent"
            onInput={(event) => props.onPhase(clampUnit(Number(event.currentTarget.value) / 100))}
          />
          <input
            class="tabularNums"
            type="number"
            min="0"
            max="100"
            step="1"
            value={phasePercent()}
            aria-label="Value phase percent value"
            onChange={(event) => props.onPhase(clampUnit(Number(event.currentTarget.value) / 100))}
          />
        </label>
        <label>
          <span>Spread</span>
          <input
            type="range"
            min="0"
            max="100"
            step="1"
            value={spreadPercent()}
            aria-label="Value fixture spread percent"
            onInput={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))}
          />
          <input
            class="tabularNums"
            type="number"
            min="0"
            max="100"
            step="1"
            value={spreadPercent()}
            aria-label="Value fixture spread percent value"
            onChange={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))}
          />
        </label>
      </div>

      <section class="valueEffectPointPanel" aria-label="Value envelope point operations">
        <div class="valueEffectSectionHeader">
          <strong>Point operations</strong>
          <span class="tabularNums">Position / Value</span>
        </div>
        <div class="moveEffectPointList" role="list" aria-label="Value envelope points">
          <For each={props.points}>
            {(point, index) => (
              <div
                class={activePointIndex() === index() ? "moveEffectPointRow active" : "moveEffectPointRow"}
                role="listitem"
                aria-posinset={index() + 1}
                aria-setsize={props.points.length}
              >
                <button
                  type="button"
                  class="moveEffectPointSelect"
                  aria-label={`Select envelope point ${index() + 1}`}
                  aria-pressed={activePointIndex() === index()}
                  onClick={() => setSelectedPoint(index())}
                >
                  Point {index() + 1}
                </button>
                <label>
                  <span>Pos</span>
                  <input
                    class="tabularNums"
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={point.position}
                    aria-label={`Position for envelope point ${index() + 1}`}
                    onChange={(event) => updatePoint(index(), { ...point, position: Number(event.currentTarget.value) })}
                  />
                </label>
                <label>
                  <span>Val</span>
                  <input
                    class="tabularNums"
                    type="number"
                    min="0"
                    max="1"
                    step="0.01"
                    value={point.value}
                    aria-label={`Value for envelope point ${index() + 1}`}
                    onChange={(event) => updatePoint(index(), { ...point, value: Number(event.currentTarget.value) })}
                  />
                </label>
                <button
                  type="button"
                  class="moveEffectPointAction remove"
                  aria-label={`Remove envelope point ${index() + 1}`}
                  title="Remove point"
                  disabled={props.points.length <= 2}
                  onClick={() => removePoint(index())}
                >
                  ×
                </button>
              </div>
            )}
          </For>
        </div>
      </section>
    </section>
  );
}
