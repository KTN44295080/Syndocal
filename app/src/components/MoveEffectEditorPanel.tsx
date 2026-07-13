import { createMemo, createSignal, For, Show } from "solid-js";

export type MoveEffectInterpolation = "Line" | "Smooth";
export type MoveEffectCoordinateMode = "Absolute" | "Relative";
export type MoveEffectDirection = "Forward" | "Reverse" | "Bounce";

export interface MoveEffectPoint {
  x: number;
  y: number;
}

export interface MoveEffectEditorPanelProps {
  /** Points always use 0..1. Relative mode interprets point - 0.5 as a runtime delta. */
  points: MoveEffectPoint[];
  closed: boolean;
  interpolation: MoveEffectInterpolation;
  coordinateMode: MoveEffectCoordinateMode;
  /** Center always uses 0..1; Relative mode applies center - 0.5 as a runtime delta. */
  center: MoveEffectPoint;
  size: MoveEffectPoint;
  /** Clockwise degrees in the editor preview. */
  rotation: number;
  periodMs: number;
  bpm: number;
  /** Null selects the free-running period; a number selects beat synchronization. */
  clockSyncBeats: number | null;
  direction: MoveEffectDirection;
  /** Normalized 0..1 cycle offset. */
  phase: number;
  /** Normalized 0..1 fixture phase spread. */
  spread: number;
  onPoints: (points: MoveEffectPoint[]) => void;
  onClosed: (closed: boolean) => void;
  onInterpolation: (interpolation: MoveEffectInterpolation) => void;
  onCoordinateMode: (coordinateMode: MoveEffectCoordinateMode) => void;
  onCenter: (center: MoveEffectPoint) => void;
  onSize: (size: MoveEffectPoint) => void;
  onRotation: (rotation: number) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onDirection: (direction: MoveEffectDirection) => void;
  onPhase: (phase: number) => void;
  onSpread: (spread: number) => void;
}

interface CanvasPoint {
  x: number;
  y: number;
}

export const moveEffectMaximumPoints = 32;
const coordinateModes: MoveEffectCoordinateMode[] = ["Absolute", "Relative"];
const interpolationModes: MoveEffectInterpolation[] = ["Line", "Smooth"];
const directionModes: MoveEffectDirection[] = ["Forward", "Reverse", "Bounce"];
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
const roundCoordinate = (value: number) => Number(value.toFixed(4));
const pathNumber = (value: number) => Number(value.toFixed(3));
const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

const pointDistance = (first: CanvasPoint, second: CanvasPoint) => Math.hypot(second.x - first.x, second.y - first.y);
const lerpPoint = (first: CanvasPoint, second: CanvasPoint, amount: number): CanvasPoint => ({
  x: first.x + (second.x - first.x) * amount,
  y: first.y + (second.y - first.y) * amount,
});
const parameterizedLerp = (first: CanvasPoint, second: CanvasPoint, firstTime: number, secondTime: number, at: number) => {
  const denominator = Math.max(0.000_001, secondTime - firstTime);
  return lerpPoint(first, second, (at - firstTime) / denominator);
};
const centripetalStep = (first: CanvasPoint, second: CanvasPoint) => Math.max(0.000_1, Math.sqrt(pointDistance(first, second)));
const centripetalPoint = (
  previous: CanvasPoint,
  first: CanvasPoint,
  second: CanvasPoint,
  next: CanvasPoint,
  amount: number,
) => {
  const time0 = 0;
  const time1 = time0 + centripetalStep(previous, first);
  const time2 = time1 + centripetalStep(first, second);
  const time3 = time2 + centripetalStep(second, next);
  const at = time1 + (time2 - time1) * clampUnit(amount);
  const levelA1 = parameterizedLerp(previous, first, time0, time1, at);
  const levelA2 = parameterizedLerp(first, second, time1, time2, at);
  const levelA3 = parameterizedLerp(second, next, time2, time3, at);
  const levelB1 = parameterizedLerp(levelA1, levelA2, time0, time2, at);
  const levelB2 = parameterizedLerp(levelA2, levelA3, time1, time3, at);
  return parameterizedLerp(levelB1, levelB2, time1, time2, at);
};

const samplePath = (points: CanvasPoint[], interpolation: MoveEffectInterpolation, closed: boolean) => {
  if (points.length < 2 || interpolation === "Line") return [...points];
  const samples: CanvasPoint[] = [points[0]];
  const segmentCount = closed ? points.length : points.length - 1;
  for (let segment = 0; segment < segmentCount; segment += 1) {
    const fromIndex = Math.min(segment, points.length - 1);
    const toIndex = fromIndex + 1 < points.length ? fromIndex + 1 : 0;
    const first = points[fromIndex];
    const second = points[toIndex];
    const previous = fromIndex === 0 ? closed ? points[points.length - 1] : first : points[fromIndex - 1];
    const nextIndex = toIndex + 1;
    const next = nextIndex < points.length ? points[nextIndex] : closed ? points[nextIndex % points.length] : second;
    for (let sample = 1; sample <= 32; sample += 1) {
      samples.push(centripetalPoint(previous, first, second, next, sample / 32));
    }
  }
  return samples;
};

const buildPath = (points: CanvasPoint[], closed: boolean) => {
  if (points.length === 0) return "";
  const segments = points.slice(1).map((point) => `L ${pathNumber(point.x)} ${pathNumber(point.y)}`);
  return [`M ${pathNumber(points[0].x)} ${pathNumber(points[0].y)}`, ...segments, closed ? "Z" : ""].filter(Boolean).join(" ");
};

export function MoveEffectEditorPanel(props: MoveEffectEditorPanelProps) {
  const [selectedPoint, setSelectedPoint] = createSignal(0);
  const [draggingPoint, setDraggingPoint] = createSignal<{ index: number; pointerId: number } | null>(null);

  const coordinateDomain = () => ({ minimum: 0, maximum: 1, origin: 0.5, step: 0.01 });
  const activePointIndex = createMemo<number | null>(() => {
    if (props.points.length === 0) return null;
    return Math.min(Math.max(0, selectedPoint()), props.points.length - 1);
  });
  const phasePercent = createMemo(() => Math.round(clampUnit(props.phase) * 100));
  const spreadPercent = createMemo(() => Math.round(clampUnit(props.spread) * 100));
  const normalizedPeriodMs = createMemo(() => Math.max(10, Math.round(Number.isFinite(props.periodMs) ? props.periodMs : 10)));
  const effectiveBpm = createMemo(() => Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120);
  const beatPeriodMs = (beats: number) => Math.max(10, Math.round((60_000 / effectiveBpm()) * beats));
  const clockSummary = createMemo(() =>
    props.clockSyncBeats === null
      ? `${normalizedPeriodMs()} ms free`
      : `${props.clockSyncBeats} beat / ${beatPeriodMs(props.clockSyncBeats)} ms`,
  );

  const normalizePoint = (point: MoveEffectPoint): MoveEffectPoint => {
    const domain = coordinateDomain();
    return {
      x: roundCoordinate(clamp(point.x, domain.minimum, domain.maximum)),
      y: roundCoordinate(clamp(point.y, domain.minimum, domain.maximum)),
    };
  };

  const pointToCanvas = (point: MoveEffectPoint): CanvasPoint => {
    const normalized = normalizePoint(point);
    return { x: normalized.x * 100, y: (1 - normalized.y) * 100 };
  };

  const canvasToPoint = (x: number, y: number): MoveEffectPoint => {
    const canvasX = clamp(x, 0, 100);
    const canvasY = clamp(y, 0, 100);
    return normalizePoint({ x: canvasX / 100, y: 1 - canvasY / 100 });
  };

  const editableCanvasPoints = createMemo(() => props.points.map(pointToCanvas));
  const editablePathSamples = createMemo(() => samplePath(editableCanvasPoints(), props.interpolation, props.closed));
  const transformedPathSamples = createMemo(() => {
    const center = pointToCanvas(props.center);
    const scaleX = clamp(props.size.x, 0.01, 2);
    const scaleY = clamp(props.size.y, 0.01, 2);
    const radians = (Number.isFinite(props.rotation) ? props.rotation : 0) * Math.PI / 180;
    const cosine = Math.cos(radians);
    const sine = Math.sin(radians);
    return editablePathSamples().map((point) => {
      const scaledX = (point.x - 50) * scaleX;
      const scaledY = (point.y - 50) * scaleY;
      return {
        x: center.x + scaledX * cosine - scaledY * sine,
        y: center.y + scaledX * sine + scaledY * cosine,
      };
    });
  });
  const editablePath = createMemo(() => buildPath(editablePathSamples(), props.closed));
  const transformedPath = createMemo(() => buildPath(transformedPathSamples(), props.closed));
  const transformedCenter = createMemo(() => pointToCanvas(props.center));

  const updatePoint = (index: number, point: MoveEffectPoint) => {
    if (!props.points[index]) return;
    const next = props.points.map((candidate) => ({ ...candidate }));
    next[index] = normalizePoint(point);
    props.onPoints(next);
  };

  const addPoint = (point?: MoveEffectPoint) => {
    if (props.points.length >= moveEffectMaximumPoints) return;
    const domain = coordinateDomain();
    const activeIndex = activePointIndex();
    const fallback = activeIndex === null
      ? { x: domain.origin, y: domain.origin }
      : {
          x: props.points[activeIndex].x + domain.step * 4,
          y: props.points[activeIndex].y + domain.step * 4,
        };
    const insertionIndex = activeIndex === null ? props.points.length : activeIndex + 1;
    const next = props.points.map((candidate) => ({ ...candidate }));
    next.splice(insertionIndex, 0, normalizePoint(point ?? fallback));
    props.onPoints(next);
    setSelectedPoint(insertionIndex);
  };

  const removePoint = (index: number) => {
    if (!props.points[index]) return;
    const next = props.points.filter((_, candidate) => candidate !== index).map((point) => ({ ...point }));
    props.onPoints(next);
    setSelectedPoint(Math.max(0, Math.min(index, next.length - 1)));
  };

  const movePoint = (index: number, direction: -1 | 1) => {
    const targetIndex = index + direction;
    if (!props.points[index] || !props.points[targetIndex]) return;
    const next = props.points.map((point) => ({ ...point }));
    [next[index], next[targetIndex]] = [next[targetIndex], next[index]];
    props.onPoints(next);
    setSelectedPoint(targetIndex);
  };

  const pointFromClient = (svg: SVGSVGElement, clientX: number, clientY: number) => {
    const rectangle = svg.getBoundingClientRect();
    if (rectangle.width <= 0 || rectangle.height <= 0) {
      const origin = coordinateDomain().origin;
      return { x: origin, y: origin };
    }
    return canvasToPoint(
      ((clientX - rectangle.left) / rectangle.width) * 100,
      ((clientY - rectangle.top) / rectangle.height) * 100,
    );
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
    const domain = coordinateDomain();
    const multiplier = event.shiftKey ? 5 : event.altKey ? 0.2 : 1;
    const step = domain.step * multiplier;
    const next = { ...point };
    if (event.key === "ArrowLeft") next.x -= step;
    else if (event.key === "ArrowRight") next.x += step;
    else if (event.key === "ArrowUp") next.y += step;
    else if (event.key === "ArrowDown") next.y -= step;
    else if (event.key === "Home") {
      next.x = domain.origin;
      next.y = domain.origin;
    } else if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      removePoint(index);
      return;
    } else {
      return;
    }
    event.preventDefault();
    updatePoint(index, next);
  };

  const updateCenterAxis = (axis: "x" | "y", value: number) => {
    const domain = coordinateDomain();
    props.onCenter({
      ...props.center,
      [axis]: roundCoordinate(clamp(value, domain.minimum, domain.maximum)),
    });
  };

  const updateSizeAxis = (axis: "x" | "y", value: number) => {
    props.onSize({
      ...props.size,
      [axis]: roundCoordinate(clamp(value, 0.01, 2)),
    });
  };

  return (
    <section class="moveEffectEditorPanel" aria-label="Move effect editor">
      <header class="moveEffectEditorHeader">
        <div>
          <strong class="textBalance">Move FX</strong>
          <span>{props.points.length} points / {props.closed ? "Closed path" : "Open path"} / {props.interpolation}</span>
        </div>
        <div class="moveEffectStatusStrip">
          <span><small>Clock</small><strong class="tabularNums">{clockSummary()}</strong></span>
          <span><small>Direction</small><strong>{props.direction}</strong></span>
          <span><small>Phase / Spread</small><strong class="tabularNums">{phasePercent()}% / {spreadPercent()}%</strong></span>
        </div>
      </header>

      <div class="moveEffectEditorSurface">
        <section class="moveEffectPathDesk" aria-label="Move path editor">
          <div class="moveEffectPathToolbar">
            <div>
              <strong>Path points</strong>
              <span class="tabularNums">{props.points.length} / {moveEffectMaximumPoints}</span>
            </div>
            <button type="button" onClick={() => addPoint()} disabled={props.points.length >= moveEffectMaximumPoints}>Add point</button>
            <button
              type="button"
              onClick={() => {
                const index = activePointIndex();
                if (index !== null) removePoint(index);
              }}
              disabled={activePointIndex() === null}
            >
              Remove point
            </button>
            <label class="checkbox inlineCheckbox moveEffectClosedToggle">
              <input type="checkbox" checked={props.closed} onChange={(event) => props.onClosed(event.currentTarget.checked)} />
              Closed
            </label>
          </div>

          <div class="moveEffectPathViewport">
            <svg
              class="moveEffectPathCanvas"
              viewBox="0 0 100 100"
              role="group"
              aria-label={`${props.coordinateMode} ${props.interpolation} Move path with ${props.points.length} points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift for coarse, Alt for fine, Home to center, Delete to remove.`}
              onDblClick={(event) => addPoint(pointFromClient(event.currentTarget, event.clientX, event.clientY))}
              onPointerMove={continuePointDrag}
              onPointerUp={endPointDrag}
              onPointerCancel={endPointDrag}
            >
              <rect class="moveEffectCanvasBackground" x="0" y="0" width="100" height="100" />
              <g class="moveEffectCanvasGrid" aria-hidden="true">
                <line x1="25" y1="0" x2="25" y2="100" />
                <line x1="50" y1="0" x2="50" y2="100" />
                <line x1="75" y1="0" x2="75" y2="100" />
                <line x1="0" y1="25" x2="100" y2="25" />
                <line x1="0" y1="50" x2="100" y2="50" />
                <line x1="0" y1="75" x2="100" y2="75" />
              </g>
              <path class="moveEffectPathGuide" d={editablePath()} />
              <path class={props.closed ? "moveEffectPathOutput closed" : "moveEffectPathOutput"} d={transformedPath()} />
              <g
                class="moveEffectOutputCenter"
                transform={`translate(${pathNumber(transformedCenter().x)} ${pathNumber(transformedCenter().y)})`}
                aria-hidden="true"
              >
                <line x1="-3" y1="0" x2="3" y2="0" />
                <line x1="0" y1="-3" x2="0" y2="3" />
                <circle r="1.4" />
              </g>
              <For each={editableCanvasPoints()}>
                {(point, index) => (
                  <g
                    class={activePointIndex() === index() ? "moveEffectPointHandle active" : "moveEffectPointHandle"}
                    transform={`translate(${pathNumber(point.x)} ${pathNumber(point.y)})`}
                    role="button"
                    tabIndex={0}
                    aria-label={`Path point ${index() + 1}, X ${props.points[index()]?.x ?? 0}, Y ${props.points[index()]?.y ?? 0}`}
                    aria-pressed={activePointIndex() === index()}
                    onFocus={() => setSelectedPoint(index())}
                    onPointerDown={(event) => beginPointDrag(event, index())}
                    onDblClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => handlePointKeyDown(event, index())}
                  >
                    <title>{`Point ${index() + 1}: ${props.points[index()]?.x ?? 0}, ${props.points[index()]?.y ?? 0}`}</title>
                    <circle r="3.3" />
                    <text x="0" y="1.15">{index() + 1}</text>
                  </g>
                )}
              </For>
            </svg>
            <Show when={props.points.length === 0}>
              <button type="button" class="moveEffectEmptyPathAction" onClick={() => addPoint()}>Add first point</button>
            </Show>
          </div>

          <div class="moveEffectPathFooter">
            <span><i class="control" />Control path</span>
            <span><i class="output" />Output preview</span>
            <small>Double-click grid to add · Arrow keys nudge · Shift coarse · Alt fine · Home center · Delete remove</small>
          </div>
        </section>

        <aside class="moveEffectInspector" aria-label="Move effect controls">
          <section class="moveEffectTransportPanel" aria-label="Move transport and clock">
            <div class="moveEffectSectionHeader">
              <strong>Transport</strong>
              <span class="tabularNums">{clockSummary()}</span>
            </div>
            <div class="moveEffectDirectionGrid" aria-label="Move direction">
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
            <div class="moveEffectClockRow">
              <label>
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
              <div class="moveEffectClockPresets" aria-label="Move clock sync presets">
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
            <div class="moveEffectPhaseGrid">
              <label>
                <span>Phase</span>
                <input
                  type="range"
                  min="0"
                  max="100"
                  step="1"
                  value={phasePercent()}
                  aria-label="Move phase percent"
                  onInput={(event) => props.onPhase(clampUnit(Number(event.currentTarget.value) / 100))}
                />
                <input
                  class="tabularNums"
                  type="number"
                  min="0"
                  max="100"
                  step="1"
                  value={phasePercent()}
                  aria-label="Move phase percent value"
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
                  aria-label="Move fixture spread percent"
                  onInput={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))}
                />
                <input
                  class="tabularNums"
                  type="number"
                  min="0"
                  max="100"
                  step="1"
                  value={spreadPercent()}
                  aria-label="Move fixture spread percent value"
                  onChange={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))}
                />
              </label>
            </div>
          </section>

          <section class="moveEffectTransformPanel" aria-label="Move transform">
            <div class="moveEffectSectionHeader">
              <strong>Transform</strong>
              <span>Translate / Scale / Rotate</span>
            </div>
            <div class="moveEffectTransformGrid">
              <label>
                Translate X
                <input
                  class="tabularNums"
                  type="number"
                  min={coordinateDomain().minimum}
                  max={coordinateDomain().maximum}
                  step={coordinateDomain().step}
                  value={props.center.x}
                  onChange={(event) => updateCenterAxis("x", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Translate Y
                <input
                  class="tabularNums"
                  type="number"
                  min={coordinateDomain().minimum}
                  max={coordinateDomain().maximum}
                  step={coordinateDomain().step}
                  value={props.center.y}
                  onChange={(event) => updateCenterAxis("y", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Scale X
                <input
                  class="tabularNums"
                  type="number"
                  min="0.01"
                  max="2"
                  step="0.01"
                  value={props.size.x}
                  onChange={(event) => updateSizeAxis("x", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Scale Y
                <input
                  class="tabularNums"
                  type="number"
                  min="0.01"
                  max="2"
                  step="0.01"
                  value={props.size.y}
                  onChange={(event) => updateSizeAxis("y", Number(event.currentTarget.value))}
                />
              </label>
              <label>
                Rotate °
                <input
                  class="tabularNums"
                  type="number"
                  min="-360"
                  max="360"
                  step="1"
                  value={props.rotation}
                  onChange={(event) => props.onRotation(clamp(Number(event.currentTarget.value), -360, 360))}
                />
              </label>
            </div>
          </section>

          <section class="moveEffectModePanel" aria-label="Move path modes">
            <div class="moveEffectModeGroup">
              <span>Coordinates</span>
              <div class="moveEffectSegmented" aria-label="Move coordinate mode">
                <For each={coordinateModes}>
                  {(mode) => (
                    <button
                      type="button"
                      class={props.coordinateMode === mode ? "active" : ""}
                      aria-pressed={props.coordinateMode === mode}
                      onClick={() => props.onCoordinateMode(mode)}
                    >
                      {mode}
                    </button>
                  )}
                </For>
              </div>
            </div>
            <div class="moveEffectModeGroup">
              <span>Interpolation</span>
              <div class="moveEffectSegmented" aria-label="Move interpolation">
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
          </section>

          <section class="moveEffectPointPanel" aria-label="Move path point operations">
            <div class="moveEffectSectionHeader">
              <strong>Point operations</strong>
              <span class="tabularNums">X / Y · {props.coordinateMode}</span>
            </div>
            <div class="moveEffectPointList" role="list" aria-label="Move path points">
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
                      aria-label={`Select path point ${index() + 1}`}
                      aria-pressed={activePointIndex() === index()}
                      onClick={() => setSelectedPoint(index())}
                    >
                      Point {index() + 1}
                    </button>
                    <label>
                      <span>X</span>
                      <input
                        class="tabularNums"
                        type="number"
                        min={coordinateDomain().minimum}
                        max={coordinateDomain().maximum}
                        step={coordinateDomain().step}
                        value={point.x}
                        aria-label={`X coordinate for path point ${index() + 1}`}
                        onChange={(event) => updatePoint(index(), { ...point, x: Number(event.currentTarget.value) })}
                      />
                    </label>
                    <label>
                      <span>Y</span>
                      <input
                        class="tabularNums"
                        type="number"
                        min={coordinateDomain().minimum}
                        max={coordinateDomain().maximum}
                        step={coordinateDomain().step}
                        value={point.y}
                        aria-label={`Y coordinate for path point ${index() + 1}`}
                        onChange={(event) => updatePoint(index(), { ...point, y: Number(event.currentTarget.value) })}
                      />
                    </label>
                    <button
                      type="button"
                      class="moveEffectPointAction"
                      aria-label={`Move path point ${index() + 1} earlier`}
                      title="Move point earlier"
                      disabled={index() === 0}
                      onClick={() => movePoint(index(), -1)}
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      class="moveEffectPointAction"
                      aria-label={`Move path point ${index() + 1} later`}
                      title="Move point later"
                      disabled={index() === props.points.length - 1}
                      onClick={() => movePoint(index(), 1)}
                    >
                      ↓
                    </button>
                    <button
                      type="button"
                      class="moveEffectPointAction remove"
                      aria-label={`Remove path point ${index() + 1}`}
                      title="Remove point"
                      onClick={() => removePoint(index())}
                    >
                      ×
                    </button>
                  </div>
                )}
              </For>
              <Show when={props.points.length === 0}>
                <p class="moveEffectPointEmpty textPretty">Add a point to define the movement path.</p>
              </Show>
            </div>
          </section>
        </aside>
      </div>
    </section>
  );
}
