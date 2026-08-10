import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { normalizeMovePathPoints, sampleMovePath, transformMovePreview } from "../effectVisualization";
import { movePathRecipePoints, type MovePathPreset } from "../moveEffect";
import {
  beginMoveEffectPointDrag,
  commitMoveEffectPointDrag,
  moveEffectMaximumPoints,
  moveEffectMinimumPoints,
  moveEffectDragSurfaceSizeFromCtm,
  normalizeMoveEffectDragPoint,
  updateMoveEffectPointDrag,
  type MoveEffectDragPoint,
  type MoveEffectPointDragState,
} from "../moveEffectDrag";

export type MoveEffectInterpolation = "Line" | "Smooth" | "Circle";
export type MoveEffectCoordinateMode = "Absolute" | "Relative";
export type MoveEffectDirection = "Forward" | "Reverse" | "Bounce";

export interface MoveEffectPoint extends MoveEffectDragPoint {}

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
  /** Applies interpolation-specific symmetry to the second half of target order. */
  symmetry: boolean;
  /** Imported fixture/beam identities retained in authored source order. */
  beamTargetCount: number;
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
  onSymmetry: (symmetry: boolean) => void;
}

interface CanvasPoint {
  x: number;
  y: number;
}

export { moveEffectMaximumPoints };

const coordinateModes: MoveEffectCoordinateMode[] = ["Absolute", "Relative"];
const interpolationModes: MoveEffectInterpolation[] = ["Line", "Smooth", "Circle"];
const directionModes: MoveEffectDirection[] = ["Forward", "Reverse", "Bounce"];
// P-EXP quick looks: named one-click path bundles reusing the existing
// movePathRecipes catalog + interpolation + direction + clock
// (qa/PRESET_EXPANSION_PLAN.md). Circle uses the V5b analytic Circle
// interpolation; the others use Smooth. Fixture/beam targets stay authored.
interface MoveQuickLook {
  label: string;
  recipe: MovePathPreset;
  closed: boolean;
  interpolation: MoveEffectInterpolation;
  direction: MoveEffectDirection;
  beats: number | null;
}
const moveQuickLooks: MoveQuickLook[] = [
  { label: "Circle Spin", recipe: "Circle", closed: true, interpolation: "Circle", direction: "Forward", beats: 4 },
  { label: "Line Sweep", recipe: "Line", closed: false, interpolation: "Smooth", direction: "Bounce", beats: 2 },
  { label: "Figure Eight", recipe: "Figure Eight", closed: true, interpolation: "Smooth", direction: "Forward", beats: 8 },
];

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

const buildPath = (points: CanvasPoint[], closed: boolean) => {
  if (points.length === 0) return "";
  const segments = points.slice(1).map((point) => `L ${pathNumber(point.x)} ${pathNumber(point.y)}`);
  return [`M ${pathNumber(points[0].x)} ${pathNumber(points[0].y)}`, ...segments, closed ? "Z" : ""].filter(Boolean).join(" ");
};

export function MoveEffectEditorPanel(props: MoveEffectEditorPanelProps) {
  const [selectedPoint, setSelectedPoint] = createSignal(0);
  const [draggingPoint, setDraggingPoint] = createSignal<MoveEffectPointDragState | null>(null);
  const activePointDrag = createMemo(() => {
    const drag = draggingPoint();
    return drag?.moved ? drag : null;
  });
  let pathCanvas: SVGSVGElement | undefined;

  const releasePointCapture = (pointerId: number) => {
    if (pathCanvas?.hasPointerCapture(pointerId)) {
      pathCanvas.releasePointerCapture(pointerId);
    }
  };

  const cancelPointDrag = (releaseCapture = true) => {
    const drag = draggingPoint();
    if (!drag) return;
    setDraggingPoint(null);
    if (releaseCapture) releasePointCapture(drag.pointerId);
  };

  onMount(() => {
    const cancelFromEscape = (event: KeyboardEvent) => {
      if (event.key !== "Escape" || !draggingPoint()) return;
      event.preventDefault();
      event.stopImmediatePropagation();
      cancelPointDrag();
    };
    window.addEventListener("keydown", cancelFromEscape, { capture: true });
    onCleanup(() => {
      window.removeEventListener("keydown", cancelFromEscape, { capture: true });
      cancelPointDrag();
    });
  });

  const coordinateDomain = () => ({ minimum: 0, maximum: 1, origin: 0.5, step: 0.01 });
  const activePointIndex = createMemo<number | null>(() => {
    if (props.points.length === 0) return null;
    return Math.min(Math.max(0, selectedPoint()), props.points.length - 1);
  });
  const phasePercent = createMemo(() => Math.round(clampUnit(props.phase) * 100));
  const spreadPercent = createMemo(() => Math.round(clampUnit(props.spread) * 100));
  const normalizedPeriodMs = createMemo(() => Math.max(10, Math.round(Number.isFinite(props.periodMs) ? props.periodMs : 10)));
  const maximumPoints = createMemo(() => props.interpolation === "Circle" ? 255 : moveEffectMaximumPoints);
  const effectiveBpm = createMemo(() => Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120);
  const beatPeriodMs = (beats: number) => Math.max(10, Math.round((60_000 / effectiveBpm()) * beats));
  const clockSummary = createMemo(() =>
    props.clockSyncBeats === null
      ? `${normalizedPeriodMs()} ms free`
      : `${props.clockSyncBeats} beat / ${beatPeriodMs(props.clockSyncBeats)} ms`,
  );

  const normalizePoint = (point: MoveEffectPoint): MoveEffectPoint => {
    const normalized = normalizeMoveEffectDragPoint(point);
    return { x: normalized.x, y: normalized.y };
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

  const editablePoints = createMemo<MoveEffectPoint[]>(() => {
    const drag = draggingPoint();
    if (!drag || !props.points[drag.index]) return props.points;
    return props.points.map((point, index) => index === drag.index
      ? { x: drag.draftPoint.x, y: drag.draftPoint.y }
      : point);
  });
  const editableCanvasPoints = createMemo(() => editablePoints().map(pointToCanvas));
  const runtimeEditableCanvasPoints = createMemo(() =>
    normalizeMovePathPoints(editablePoints(), props.closed, props.interpolation).map(pointToCanvas),
  );
  const editablePathSamples = createMemo(() => sampleMovePath(runtimeEditableCanvasPoints(), props.interpolation, props.closed));
  const previewTransform = createMemo(() => ({
    coordinate_mode: props.coordinateMode,
    center_x: props.center.x,
    center_y: props.center.y,
    size_x: props.size.x,
    size_y: props.size.y,
    rotation_degrees: props.rotation,
  }));
  const transformedPathSamples = createMemo(() =>
    transformMovePreview(editablePathSamples(), previewTransform()),
  );
  const editablePath = createMemo(() => buildPath(editablePathSamples(), props.closed));
  const transformedPath = createMemo(() => buildPath(transformedPathSamples(), props.closed));
  const transformedCenter = createMemo(() =>
    transformMovePreview([{ x: 50, y: 50 }], previewTransform())[0] ?? { x: 50, y: 50 },
  );

  const updatePoint = (index: number, point: MoveEffectPoint) => {
    if (!props.points[index]) return;
    const next = props.points.map((candidate) => ({ ...candidate }));
    next[index] = normalizePoint(point);
    props.onPoints(next);
  };

  const addPoint = (point?: MoveEffectPoint) => {
    if (props.points.length >= maximumPoints()) return;
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
    if (!props.points[index] || props.points.length <= moveEffectMinimumPoints) return;
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
    const matrix = svg.getScreenCTM();
    if (matrix) {
      const point = new DOMPoint(clientX, clientY).matrixTransform(matrix.inverse());
      return canvasToPoint(point.x, point.y);
    }
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
    if (event.button !== 0 || !event.isPrimary || draggingPoint() || !props.points[index]) return;
    event.preventDefault();
    event.stopPropagation();
    const svg = event.currentTarget.ownerSVGElement;
    if (!svg) return;
    const rectangle = svg.getBoundingClientRect();
    const surfaceSize = moveEffectDragSurfaceSizeFromCtm(
      svg.getScreenCTM(),
      rectangle.width,
      rectangle.height,
    );
    event.currentTarget.focus({ preventScroll: true });
    svg.setPointerCapture(event.pointerId);
    setSelectedPoint(index);
    setDraggingPoint(beginMoveEffectPointDrag({
      index,
      pointerId: event.pointerId,
      clientX: event.clientX,
      clientY: event.clientY,
      surfaceWidth: surfaceSize.width,
      surfaceHeight: surfaceSize.height,
      point: props.points[index],
    }));
  };

  const continuePointDrag = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const dragging = draggingPoint();
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    event.preventDefault();
    setDraggingPoint(updateMoveEffectPointDrag(dragging, {
      clientX: event.clientX,
      clientY: event.clientY,
    }));
  };

  const commitPointDrag = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const dragging = draggingPoint();
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    const completed = updateMoveEffectPointDrag(dragging, {
      clientX: event.clientX,
      clientY: event.clientY,
    });
    const next = commitMoveEffectPointDrag(props.points, completed);
    setDraggingPoint(null);
    releasePointCapture(event.pointerId);
    if (next) props.onPoints(next);
  };

  const cancelPointDragForPointer = (event: PointerEvent & { currentTarget: SVGSVGElement }, releaseCapture = true) => {
    const dragging = draggingPoint();
    if (!dragging || dragging.pointerId !== event.pointerId) return;
    cancelPointDrag(releaseCapture);
  };

  const handlePointKeyDown = (event: KeyboardEvent, index: number) => {
    if (draggingPoint()) return;
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
              <span class="tabularNums">{props.points.length} / {maximumPoints()}</span>
            </div>
            <button type="button" onClick={() => addPoint()} disabled={props.points.length >= maximumPoints()}>Add point</button>
            <button
              type="button"
              onClick={() => {
                const index = activePointIndex();
                if (index !== null) removePoint(index);
              }}
              disabled={activePointIndex() === null || props.points.length <= moveEffectMinimumPoints}
            >
              Remove point
            </button>
            <label class="checkbox inlineCheckbox moveEffectClosedToggle">
              <input
                type="checkbox"
                checked={props.closed}
                disabled={props.interpolation === "Circle"}
                onChange={(event) => props.onClosed(event.currentTarget.checked)}
              />
              Closed
            </label>
          </div>

          <div class="moveEffectPathViewport">
            <svg
              ref={pathCanvas}
              class="moveEffectPathCanvas"
              viewBox="0 0 100 100"
              role="group"
              aria-label={`${props.coordinateMode} ${props.interpolation} Move path with ${props.points.length} points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift for coarse, Alt for fine, Home to center, Delete to remove.`}
              onDblClick={(event) => addPoint(pointFromClient(event.currentTarget, event.clientX, event.clientY))}
              onPointerMove={continuePointDrag}
              onPointerUp={commitPointDrag}
              onPointerCancel={(event) => cancelPointDragForPointer(event)}
              onLostPointerCapture={(event) => cancelPointDragForPointer(event, false)}
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
              <Show when={activePointDrag()}>
                {(drag) => {
                  const original = () => pointToCanvas(drag().originalPoint);
                  return (
                    <g
                      class="moveEffectPointGhost"
                      data-move-point-ghost
                      transform={`translate(${pathNumber(original().x)} ${pathNumber(original().y)})`}
                      aria-hidden="true"
                      pointer-events="none"
                    >
                      <circle
                        r="4.2"
                        fill="none"
                        stroke="#cbd1d5"
                        stroke-width="1"
                        stroke-dasharray="2 2"
                        vector-effect="non-scaling-stroke"
                      />
                    </g>
                  );
                }}
              </Show>
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
                    classList={{
                      moveEffectPointHandle: true,
                      active: activePointIndex() === index(),
                      dragging: activePointDrag()?.index === index(),
                    }}
                    transform={`translate(${pathNumber(point.x)} ${pathNumber(point.y)})`}
                    role="button"
                    tabindex={0}
                    aria-label={`Path point ${index() + 1}, X ${editablePoints()[index()]?.x ?? 0}, Y ${editablePoints()[index()]?.y ?? 0}`}
                    aria-pressed={activePointIndex() === index()}
                    onFocus={() => setSelectedPoint(index())}
                    onPointerDown={(event) => beginPointDrag(event, index())}
                    onDblClick={(event) => event.stopPropagation()}
                    onKeyDown={(event) => handlePointKeyDown(event, index())}
                  >
                    <title>{`Point ${index() + 1}: ${editablePoints()[index()]?.x ?? 0}, ${editablePoints()[index()]?.y ?? 0}`}</title>
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
            <span>
              <i class="output" />
              <Show when={props.coordinateMode === "Relative"} fallback="Output preview">
                Output preview · neutral 50% base
              </Show>
            </span>
            <Show when={activePointDrag()}>
              {(drag) => (
                <output class="moveEffectDragReadout tabularNums" data-move-point-drag-readout aria-live="polite">
                  <span>Point</span> {drag().index + 1} · X {drag().draftPoint.x.toFixed(4)} · Y {drag().draftPoint.y.toFixed(4)}
                </output>
              )}
            </Show>
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
            <div class="moveEffectClockPresets" aria-label="Move quick looks" data-move-quick-looks>
              <For each={moveQuickLooks}>
                {(look) => (
                  <button
                    type="button"
                    data-move-quick-look={look.label}
                    title={`${look.label}: ${look.recipe} ${look.interpolation} ${look.direction}${look.beats === null ? "" : `, ${look.beats} beat${look.beats === 1 ? "" : "s"}`}`}
                    onClick={() => {
                      props.onPoints(movePathRecipePoints(look.recipe).map((point) => ({ ...point })));
                      props.onClosed(look.closed);
                      props.onInterpolation(look.interpolation);
                      props.onDirection(look.direction);
                      props.onClockSyncBeats(look.beats);
                    }}
                  >
                    {look.label}
                  </button>
                )}
              </For>
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
            <button
              type="button"
              class="moveEffectSymmetryToggle"
              classList={{ active: props.symmetry }}
              aria-label="Move symmetry"
              aria-pressed={props.symmetry}
              onClick={() => props.onSymmetry(!props.symmetry)}
            >
              <strong>Symmetry</strong>
              <span>{props.interpolation === "Circle" ? "Reverse second-half traversal" : "Mirror second half Pan"}</span>
            </button>
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
            <div class="effectFormHint textPretty">
              {props.beamTargetCount > 0
                ? `${props.beamTargetCount} imported beam targets`
                : "Beam targets follow fixture profile channel order."}
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
                      disabled={props.points.length <= moveEffectMinimumPoints}
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
