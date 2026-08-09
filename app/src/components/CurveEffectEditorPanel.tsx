import { createMemo, createSignal, For, Show } from "solid-js";
import { buildCurvePreviewPath } from "../effectVisualization";
import type { CurveEffectPoint, ValueEffectDirection, ValueEffectMode } from "../types";

export const curveEffectMaximumPoints = 32;

export interface CurveEffectEditorPanelProps {
  points: CurveEffectPoint[];
  mode: ValueEffectMode;
  direction: ValueEffectDirection;
  periodMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  phase: number;
  spread: number;
  onPoints: (points: CurveEffectPoint[]) => void;
  onMode: (mode: ValueEffectMode) => void;
  onDirection: (direction: ValueEffectDirection) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onPhase: (phase: number) => void;
  onSpread: (spread: number) => void;
}

const modes: ValueEffectMode[] = ["Absolute", "Relative"];
const directions: ValueEffectDirection[] = ["Forward", "Reverse", "Bounce"];

// P-EXP quick looks: named one-click starter curves (qa/PRESET_EXPANSION_PLAN.md).
// Tangent convention follows the existing Saw sample: slope values at each point.
interface CurveQuickLook {
  label: string;
  points: CurveEffectPoint[];
  beats: number | null;
}
const curveQuickLooks: CurveQuickLook[] = [
  {
    label: "Ramp Up",
    points: [
      { position: 0, value: 0, in_tangent: 0, out_tangent: 1 },
      { position: 1, value: 1, in_tangent: 1, out_tangent: 0 },
    ],
    beats: 2,
  },
  {
    label: "Strobe Snap",
    points: [
      { position: 0, value: 0, in_tangent: 0, out_tangent: 0 },
      { position: 0.05, value: 1, in_tangent: 0, out_tangent: 0 },
      { position: 0.5, value: 1, in_tangent: 0, out_tangent: 0 },
      { position: 0.55, value: 0, in_tangent: 0, out_tangent: 0 },
      { position: 1, value: 0, in_tangent: 0, out_tangent: 0 },
    ],
    beats: 0.5,
  },
  {
    label: "Soft Breathe",
    points: [
      { position: 0, value: 0, in_tangent: 0, out_tangent: 0 },
      { position: 0.5, value: 1, in_tangent: 0, out_tangent: 0 },
      { position: 1, value: 0, in_tangent: 0, out_tangent: 0 },
    ],
    beats: 4,
  },
];
const clocks = [
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
const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

export function CurveEffectEditorPanel(props: CurveEffectEditorPanelProps) {
  const [selectedPoint, setSelectedPoint] = createSignal(0);
  const [dragging, setDragging] = createSignal<{ index: number; pointerId: number } | null>(null);
  const activePointIndex = createMemo<number | null>(() =>
    props.points.length === 0 ? null : Math.min(Math.max(0, selectedPoint()), props.points.length - 1),
  );
  const periodMs = createMemo(() => Math.max(10, Math.round(Number.isFinite(props.periodMs) ? props.periodMs : 10)));
  const bpm = createMemo(() => (Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120));
  const beatPeriodMs = (beats: number) => Math.max(10, Math.round((60_000 / bpm()) * beats));
  const phasePercent = createMemo(() => Math.round(clampUnit(props.phase) * 100));
  const spreadPercent = createMemo(() => Math.round(clampUnit(props.spread) * 100));
  const previewPath = createMemo(() => buildCurvePreviewPath(props.points, props.direction, 0, 100, 40, 128));

  const normalizePoint = (index: number, point: CurveEffectPoint): CurveEffectPoint => {
    const previous = index > 0 ? props.points[index - 1].position + 0.001 : 0;
    const next = index < props.points.length - 1 ? props.points[index + 1].position - 0.001 : 1;
    return {
      position: round(clamp(point.position, Math.min(previous, next), Math.max(previous, next))),
      value: round(clampUnit(point.value)),
      in_tangent: round(clamp(point.in_tangent, -32, 32)),
      out_tangent: round(clamp(point.out_tangent, -32, 32)),
    };
  };

  const updatePoint = (index: number, point: CurveEffectPoint) => {
    if (!props.points[index]) return;
    const next = props.points.map((candidate) => ({ ...candidate }));
    next[index] = normalizePoint(index, point);
    props.onPoints(next);
  };

  const addPoint = (candidate?: CurveEffectPoint) => {
    if (props.points.length >= curveEffectMaximumPoints) return;
    const active = activePointIndex();
    const insertionIndex = active === null ? props.points.length : active + 1;
    const before = props.points[insertionIndex - 1];
    const after = props.points[insertionIndex];
    const previous = props.points[insertionIndex - 2];
    const point = candidate ?? {
      position: before && after
        ? (before.position + after.position) / 2
        : before && previous
          ? (previous.position + before.position) / 2
          : before
            ? clampUnit(before.position - 0.1)
            : 0.5,
      value: before && after ? (before.value + after.value) / 2 : 0.5,
      in_tangent: before?.out_tangent ?? 0,
      out_tangent: after?.in_tangent ?? 0,
    };
    const requestedPosition = clampUnit(point.position);
    const duplicateIndex = props.points.findIndex((entry) => Math.abs(entry.position - requestedPosition) < 0.000_001);
    if (duplicateIndex >= 0) {
      setSelectedPoint(duplicateIndex);
      return;
    }
    const next = [...props.points.map((entry) => ({ ...entry })), {
      position: requestedPosition,
      value: clampUnit(point.value),
      in_tangent: clamp(point.in_tangent, -32, 32),
      out_tangent: clamp(point.out_tangent, -32, 32),
    }].sort((first, second) => first.position - second.position);
    props.onPoints(next);
    const nextIndex = next.findIndex((entry) => Math.abs(entry.position - requestedPosition) < 0.000_001);
    setSelectedPoint(nextIndex >= 0 ? nextIndex : Math.min(insertionIndex, next.length - 1));
  };

  const removePoint = (index: number) => {
    if (props.points.length <= 2 || !props.points[index]) return;
    const next = props.points.filter((_, candidate) => candidate !== index).map((point) => ({ ...point }));
    props.onPoints(next);
    setSelectedPoint(Math.max(0, Math.min(index, next.length - 1)));
  };

  const pointFromPointer = (svg: SVGSVGElement, clientX: number, clientY: number, source: CurveEffectPoint) => {
    const bounds = svg.getBoundingClientRect();
    if (bounds.width <= 0 || bounds.height <= 0) return source;
    return {
      ...source,
      position: clampUnit((clientX - bounds.left) / bounds.width),
      value: clampUnit(1 - (clientY - bounds.top) / bounds.height),
    };
  };

  const endDrag = (event: PointerEvent & { currentTarget: SVGSVGElement }) => {
    const current = dragging();
    if (!current || current.pointerId !== event.pointerId) return;
    if (event.currentTarget.hasPointerCapture(event.pointerId)) event.currentTarget.releasePointerCapture(event.pointerId);
    setDragging(null);
  };

  return (
    <section class="valueEffectEditor curveEffectEditor" aria-label="Curve effect editor">
      <header class="valueEffectHeader">
        <div>
          <strong>Curve FX</strong>
          <span>Cubic Hermite channel function · independent in/out tangents</span>
        </div>
        <div class="valueEffectStatusStrip">
          <span><small>Points</small><strong>{props.points.length} / {curveEffectMaximumPoints}</strong></span>
          <span><small>Direction</small><strong>{props.direction}</strong></span>
          <span><small>Phase / Spread</small><strong>{phasePercent()}% / {spreadPercent()}%</strong></span>
        </div>
      </header>

      <div class="valueEffectCanvasWrap">
        <div class="valueEffectCanvasToolbar">
          <strong>Channel curve</strong>
          <span>Drag points; edit tangents below</span>
          <button type="button" onClick={() => addPoint()} disabled={props.points.length >= curveEffectMaximumPoints}>Add point</button>
          <button type="button" onClick={() => activePointIndex() !== null && removePoint(activePointIndex()!)} disabled={props.points.length <= 2}>Remove point</button>
        </div>
        <svg
          class="valueEffectCanvas"
          viewBox="0 0 100 40"
          preserveAspectRatio="none"
          role="group"
          aria-label={`Cubic curve with ${props.points.length} points`}
          onDblClick={(event) => addPoint(pointFromPointer(event.currentTarget, event.clientX, event.clientY, { position: 0.5, value: 0.5, in_tangent: 0, out_tangent: 0 }))}
          onPointerMove={(event) => {
            const current = dragging();
            if (!current || current.pointerId !== event.pointerId || !props.points[current.index]) return;
            updatePoint(current.index, pointFromPointer(event.currentTarget, event.clientX, event.clientY, props.points[current.index]));
          }}
          onPointerUp={endDrag}
          onPointerCancel={endDrag}
        >
          <rect class="valueEffectCanvasBackground" x="0" y="0" width="100" height="40" />
          <g class="valueEffectCanvasGrid" aria-hidden="true">
            <line x1="25" y1="0" x2="25" y2="40" /><line x1="50" y1="0" x2="50" y2="40" /><line x1="75" y1="0" x2="75" y2="40" />
            <line x1="0" y1="20" x2="100" y2="20" />
          </g>
          <path class="valueEffectCanvasPath" d={previewPath()} />
          <For each={props.points}>
            {(point, index) => (
              <g
                class={activePointIndex() === index() ? "valueEffectPointHandle active" : "valueEffectPointHandle"}
                transform={`translate(${point.position * 100} ${(1 - point.value) * 40})`}
                role="button"
                tabIndex={0}
                aria-label={`Curve point ${index() + 1}, position ${point.position}, value ${point.value}`}
                onFocus={() => setSelectedPoint(index())}
                onPointerDown={(event) => {
                  event.preventDefault();
                  event.stopPropagation();
                  const svg = event.currentTarget.ownerSVGElement;
                  if (!svg) return;
                  svg.setPointerCapture(event.pointerId);
                  setSelectedPoint(index());
                  setDragging({ index: index(), pointerId: event.pointerId });
                }}
              >
                <circle r="2.2" />
              </g>
            )}
          </For>
        </svg>
      </div>

      <div class="valueEffectModeRow">
        <div class="valueEffectModeGroup"><span>Mode</span><div class="moveEffectSegmented" aria-label="Curve mode"><For each={modes}>{(mode) => <button type="button" class={props.mode === mode ? "active" : ""} aria-pressed={props.mode === mode} onClick={() => props.onMode(mode)}>{mode}</button>}</For></div></div>
        <div class="valueEffectModeGroup"><span>Direction</span><div class="moveEffectSegmented" aria-label="Curve direction"><For each={directions}>{(direction) => <button type="button" class={props.direction === direction ? "active" : ""} aria-pressed={props.direction === direction} onClick={() => props.onDirection(direction)}>{direction}</button>}</For></div></div>
      </div>

      <div class="valueEffectTransportRow">
        <label class="valueEffectPeriodField">Period ms<input class="tabularNums" type="number" min="10" step="10" value={periodMs()} onInput={(event) => props.onPeriodMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))} /></label>
        <div class="moveEffectClockPresets" aria-label="Curve clock sync presets"><For each={clocks}>{(clock) => <button type="button" class={nearlyEqual(props.clockSyncBeats, clock.beats) ? "active" : ""} aria-pressed={nearlyEqual(props.clockSyncBeats, clock.beats)} title={clock.beats === null ? `${periodMs()} ms free` : `${beatPeriodMs(clock.beats)} ms at ${bpm()} BPM`} onClick={() => props.onClockSyncBeats(clock.beats)}><Show when={clock.beats !== null} fallback="Free"><span data-no-localize>{clock.label}</span></Show></button>}</For></div>
      </div>

      <div class="moveEffectClockPresets" aria-label="Curve quick looks" data-curve-quick-looks>
        <For each={curveQuickLooks}>
          {(look) => (
            <button
              type="button"
              data-curve-quick-look={look.label}
              title={look.beats === null ? `${look.label}: free-running` : `${look.label}: ${look.beats} beat${look.beats === 1 ? "" : "s"}`}
              onClick={() => {
                props.onPoints(look.points.map((point) => ({ ...point })));
                props.onClockSyncBeats(look.beats);
              }}
            >
              {look.label}
            </button>
          )}
        </For>
      </div>

      <div class="valueEffectPhaseGrid">
        <label><span>Phase</span><input type="range" min="0" max="100" value={phasePercent()} aria-label="Curve phase percent" onInput={(event) => props.onPhase(clampUnit(Number(event.currentTarget.value) / 100))} /><input class="tabularNums" type="number" min="0" max="100" value={phasePercent()} aria-label="Curve phase percent value" onChange={(event) => props.onPhase(clampUnit(Number(event.currentTarget.value) / 100))} /></label>
        <label><span>Spread</span><input type="range" min="0" max="100" value={spreadPercent()} aria-label="Curve fixture spread percent" onInput={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))} /><input class="tabularNums" type="number" min="0" max="100" value={spreadPercent()} aria-label="Curve fixture spread percent value" onChange={(event) => props.onSpread(clampUnit(Number(event.currentTarget.value) / 100))} /></label>
      </div>

      <section class="valueEffectPointPanel" aria-label="Curve point and tangent operations">
        <div class="valueEffectSectionHeader"><strong>Point operations</strong><span>Position / Value / In tangent / Out tangent</span></div>
        <div class="moveEffectPointList" role="list" aria-label="Curve points">
          <For each={props.points}>
            {(point, index) => (
              <div class={activePointIndex() === index() ? "moveEffectPointRow curveEffectPointRow active" : "moveEffectPointRow curveEffectPointRow"} role="listitem">
                <button type="button" class="moveEffectPointSelect" aria-label={`Select curve point ${index() + 1}`} aria-pressed={activePointIndex() === index()} onClick={() => setSelectedPoint(index())}>Point {index() + 1}</button>
                <For each={[
                  ["Pos", "position", 0, 1],
                  ["Val", "value", 0, 1],
                  ["In", "in_tangent", -32, 32],
                  ["Out", "out_tangent", -32, 32],
                ] as const}>
                  {([label, field, minimum, maximum]) => <label><span>{label}</span><input class="tabularNums" type="number" min={minimum} max={maximum} step="0.01" value={point[field]} aria-label={`${label} for curve point ${index() + 1}`} onChange={(event) => updatePoint(index(), { ...point, [field]: Number(event.currentTarget.value) })} /></label>}
                </For>
                <button type="button" class="moveEffectPointAction remove" aria-label={`Remove curve point ${index() + 1}`} disabled={props.points.length <= 2} onClick={() => removePoint(index())}>×</button>
              </div>
            )}
          </For>
        </div>
      </section>
    </section>
  );
}
