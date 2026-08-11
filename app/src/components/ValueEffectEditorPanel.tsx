import { createMemo, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import type {
  ColorEffectSpatialPattern,
  ColorEffectSpatialRecipe,
  ValueEffectDirection,
  ValueEffectInterpolation,
  ValueEffectMode,
  ValueEffectPoint,
} from "../types";
import { defaultSpatialRecipe } from "../spatialRecipeCompatibility";

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
  /** Null keeps Syndocal's custom envelope; a pattern enables Daslight-style VALUE FX. */
  spatialPattern: ColorEffectSpatialPattern | null;
  onPoints: (points: ValueEffectPoint[]) => void;
  onInterpolation: (interpolation: ValueEffectInterpolation) => void;
  onMode: (mode: ValueEffectMode) => void;
  onDirection: (direction: ValueEffectDirection) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onPhase: (phase: number) => void;
  onSpread: (spread: number) => void;
  onSpatialPattern: (pattern: ColorEffectSpatialPattern | null) => void;
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

type ValueGeneratorKind =
  | "CustomEnvelope"
  | "ColorRainbow"
  | "Burst"
  | "Plasma"
  | "KnightRider"
  | "Sweep"
  | "Sparkle"
  | "RandomFill"
  | "Perlin";

// P-EXP quick looks: named one-click starter configurations per proven VALUE
// generator. Parameter values follow qa/PRESET_EXPANSION_PLAN.md — Plasma uses
// the statically recovered Daslight constructor defaults; the rest use curated
// practical values within the validated recipe domains. Labels stay in the
// T25-G English-invariant FX vocabulary class.
const quickLookRampPoints: ValueEffectPoint[] = [
  { position: 0, value: 1 },
  { position: 1, value: 0 },
];
const quickLookGrayscalePoints: ValueEffectPoint[] = [
  { position: 0, value: 1 },
  { position: 0.5, value: 0.5 },
  { position: 1, value: 0 },
];
interface ValueQuickLook {
  label: string;
  recipe: ColorEffectSpatialRecipe;
  points: ValueEffectPoint[];
  beats: number | null;
}
const valueQuickLooks: ValueQuickLook[] = [
  {
    label: "Sweep Bounce",
    recipe: { Sweep: { grayscale: false, vertical_symmetry: false, direction_change: true } },
    points: quickLookGrayscalePoints,
    beats: 2,
  },
  {
    label: "Plasma Drift",
    recipe: {
      Plasma: {
        grayscale: false,
        vertical_symmetry: false,
        size_x: 1,
        param_x: 2,
        size_y: 1,
        param_y: 2,
        speed_x: -1,
        param_sx: 2,
        speed_y: 1,
        param_sy: -1,
      },
    },
    points: quickLookGrayscalePoints,
    beats: 8,
  },
  {
    label: "Knight Rider Scan",
    recipe: {
      KnightRider: { grayscale: false, vertical_symmetry: false, size: 2, one_way: false, fading: true, go_outside: false, gradient: 50 },
    },
    points: quickLookRampPoints,
    beats: 1,
  },
  {
    label: "Sparkle Rain",
    recipe: { Sparkle: { grayscale: false, vertical_symmetry: false, rng_seed: 1, number: 6, lifetime_ms: 400, source_lifespan: null, width: 10, height: null } },
    points: quickLookRampPoints,
    beats: 1,
  },
  {
    label: "Burst Pulse",
    recipe: { Burst: { color_width: 50, gradient: 100 } },
    points: quickLookRampPoints,
    beats: 4,
  },
  {
    label: "Random Fill Steps",
    recipe: { RandomFill: { grayscale: false, vertical_symmetry: false, rng_seed: 1, point_width: 20, source_point_height: null } },
    points: quickLookRampPoints,
    beats: 2,
  },
];

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

// Fixed radius of the point handles, in screen pixels. The canvas uses a
// pixel-space viewBox so circles stay round instead of stretching with width.
const pointHandleRadius = 4.5;

export function ValueEffectEditorPanel(props: ValueEffectEditorPanelProps) {
  const [selectedPoint, setSelectedPoint] = createSignal(0);
  const [draggingPoint, setDraggingPoint] = createSignal<{ index: number; pointerId: number } | null>(null);
  const [canvasSize, setCanvasSize] = createSignal({ width: 600, height: 150 });
  let canvasEl: SVGSVGElement | undefined;

  onMount(() => {
    if (!canvasEl || typeof ResizeObserver === "undefined") return;
    const observer = new ResizeObserver((entries) => {
      const rect = entries[0]?.contentRect;
      if (rect && rect.width > 0 && rect.height > 0) {
        setCanvasSize({ width: rect.width, height: rect.height });
      }
    });
    observer.observe(canvasEl);
    onCleanup(() => observer.disconnect());
  });

  const canvasWidth = createMemo(() => canvasSize().width);
  const canvasHeight = createMemo(() => canvasSize().height);

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
  const generatorKind = createMemo<ValueGeneratorKind>(() => {
    const recipe = props.spatialPattern?.recipe;
    if (!recipe) return "CustomEnvelope";
    return Object.keys(recipe)[0] as Exclude<ValueGeneratorKind, "CustomEnvelope">;
  });
  const generatorValues = createMemo<Record<string, number | boolean>>(() => {
    const recipe = props.spatialPattern?.recipe;
    if (!recipe) return {};
    return Object.values(recipe)[0] as Record<string, number | boolean>;
  });
  const generatorNumber = (key: string, fallback = 0) => {
    const value = generatorValues()[key];
    return typeof value === "number" && Number.isFinite(value) ? value : fallback;
  };
  const generatorBoolean = (key: string) => generatorValues()[key] === true;
  const generatorTransform = () => generatorBoolean("vertical_symmetry") ? "vertical" : "none";
  const selectGeneratorKind = (kind: ValueGeneratorKind) => {
    if (kind === "CustomEnvelope") {
      props.onSpatialPattern(null);
      return;
    }
    props.onSpatialPattern({
      recipe: defaultSpatialRecipe(kind),
      parameter_model_version: 1,
      beam_targets: props.spatialPattern?.beam_targets ?? [],
    });
  };
  const patchGeneratorValues = (patch: Record<string, number | boolean>) => {
    const pattern = props.spatialPattern;
    const kind = generatorKind();
    if (!pattern || kind === "CustomEnvelope") return;
    props.onSpatialPattern({
      ...pattern,
      recipe: { [kind]: { ...generatorValues(), ...patch } } as ColorEffectSpatialRecipe,
    });
  };
  const applyQuickLook = (look: ValueQuickLook) => {
    props.onPoints(look.points.map((point) => ({ ...point })));
    props.onSpatialPattern({
      recipe: structuredClone(look.recipe),
      parameter_model_version: 1,
      beam_targets: props.spatialPattern?.beam_targets ?? [],
    });
    props.onClockSyncBeats(look.beats);
  };

  const pointToCanvas = (point: ValueEffectPoint): CanvasPoint => ({
    x: clampUnit(point.position) * canvasWidth(),
    y: (1 - clampUnit(point.value)) * canvasHeight(),
  });

  // Output preview follows the envelope, then Reverse/Bounce reshaping of the read progress.
  const previewPath = createMemo(() => {
    if (props.points.length < 2) return "";
    const width = canvasWidth();
    const height = canvasHeight();
    const samples: string[] = [];
    for (let step = 0; step <= 64; step += 1) {
      const readProgress = step / 64;
      let progress = readProgress;
      if (generatorKind() === "CustomEnvelope" && props.direction === "Reverse") progress = 1 - readProgress;
      else if (generatorKind() === "CustomEnvelope" && props.direction === "Bounce") {
        const doubled = readProgress * 2;
        progress = doubled <= 1 ? doubled : 2 - doubled;
      }
      const value = sampleEnvelope(
        props.points,
        generatorKind() === "CustomEnvelope" ? props.interpolation : "Line",
        progress,
      );
      const x = pathNumber(readProgress * width);
      const y = pathNumber((1 - value) * height);
      samples.push(`${step === 0 ? "M" : "L"} ${x} ${y}`);
    }
    return samples.join(" ");
  });
  const canvasPoints = createMemo(() => props.points.map(pointToCanvas));
  const gridLinesX = createMemo(() => [0.25, 0.5, 0.75].map((fraction) => fraction * canvasWidth()));
  const gridLineY = createMemo(() => 0.5 * canvasHeight());

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
            {generatorKind() === "CustomEnvelope"
              ? `${props.points.length} points / ${props.interpolation} / ${props.mode}`
              : `${generatorKind()} / ${props.points.length} values`}
          </span>
        </div>
        <div class="valueEffectStatusStrip">
          <span>
            <small>Clock</small>
            <strong class="tabularNums">{clockSummary()}</strong>
          </span>
          <span>
            <small>{generatorKind() === "CustomEnvelope" ? "Direction" : "Generator"}</small>
            <strong>{generatorKind() === "CustomEnvelope" ? props.direction : generatorKind()}</strong>
          </span>
          <span>
            <small>Phase / Spread</small>
            <strong class="tabularNums">
              {phasePercent()}% / {spreadPercent()}%
            </strong>
          </span>
        </div>
      </header>

      <fieldset
        class="colorEffectMotionPanel valueEffectGeneratorPanel"
        data-value-generator={generatorKind()}
      >
        <legend>Value generator</legend>
        <div class="colorEffectModeGrid">
          <label>
            Generator
            <select
              value={generatorKind()}
              data-value-generator-select
              aria-label="Value generator"
              onInput={(event) => selectGeneratorKind(event.currentTarget.value as ValueGeneratorKind)}
            >
              <option value="CustomEnvelope">Custom envelope</option>
              <option value="ColorRainbow">Rainbow</option>
              <option value="Burst">Burst</option>
              <option value="Plasma">Plasma</option>
              <option value="KnightRider">Knight Rider</option>
              <option value="Sweep">Sweep</option>
              <option value="Sparkle">Sparkle</option>
              <option value="RandomFill">Random fill</option>
              <option value="Perlin">Perlin</option>
            </select>
          </label>
          <Show when={props.spatialPattern}>
            <div class="effectFormHint textPretty">
              {props.spatialPattern?.beam_targets?.length
                ? `${props.spatialPattern.beam_targets.length} imported beam targets`
                : "Beam targets follow selected fixture profile order."}
            </div>
          </Show>
        </div>
        <div
          class="moveEffectClockPresets"
          aria-label="Value quick looks"
          data-value-quick-looks
        >
          <For each={valueQuickLooks}>
            {(look) => (
              <button
                type="button"
                data-value-quick-look={look.label}
                title={
                  look.beats === null
                    ? `${look.label}: free-running`
                    : `${look.label}: ${look.beats} beat${look.beats === 1 ? "" : "s"}`
                }
                onClick={() => applyQuickLook(look)}
              >
                {look.label}
              </button>
            )}
          </For>
        </div>
        <Show when={generatorKind() === "KnightRider"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select data-value-knight-transform value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Size %<input type="number" min="0.01" max="100000" step="0.1" value={generatorNumber("size", 8)} onInput={(event) => patchGeneratorValues({ size: clamp(Number(event.currentTarget.value) || 0.01, 0.01, 100000) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={generatorNumber("gradient", 50)} onInput={(event) => patchGeneratorValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label><input type="checkbox" checked={generatorBoolean("one_way")} onInput={(event) => patchGeneratorValues({ one_way: event.currentTarget.checked })} /> One way only</label>
            <label><input type="checkbox" checked={generatorBoolean("fading")} onInput={(event) => patchGeneratorValues({ fading: event.currentTarget.checked })} /> Fading</label>
            <label><input type="checkbox" checked={generatorBoolean("go_outside")} onInput={(event) => patchGeneratorValues({ go_outside: event.currentTarget.checked })} /> Go outside</label>
          </div>
        </Show>
        <Show when={generatorKind() === "Sweep"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select data-value-sweep-transform value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label><input type="checkbox" data-value-sweep-direction-change checked={generatorBoolean("direction_change")} onInput={(event) => patchGeneratorValues({ direction_change: event.currentTarget.checked })} /> Direction change</label>
          </div>
        </Show>
        <Show when={generatorKind() === "Burst"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Color width %<input type="number" min="0.01" max="100000" step="0.1" value={generatorNumber("color_width", 50)} onInput={(event) => patchGeneratorValues({ color_width: clamp(Number(event.currentTarget.value) || 0.01, 0.01, 100000) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="0.1" value={generatorNumber("gradient", 100)} onInput={(event) => patchGeneratorValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
          </div>
        </Show>
        <Show when={generatorKind() === "RandomFill"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Point width %<input type="number" min="0.01" max="100000" step="0.1" value={generatorNumber("point_width", 10)} onInput={(event) => patchGeneratorValues({ point_width: clamp(Number(event.currentTarget.value) || 0.01, 0.01, 100000) })} /></label>
            <label>Seed<input type="number" min="0" max="4294967295" step="1" value={generatorNumber("rng_seed", 0)} onInput={(event) => patchGeneratorValues({ rng_seed: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 4_294_967_295) })} /></label>
          </div>
        </Show>
        <Show when={generatorKind() === "Sparkle"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Sparkle number<input type="number" min="1" max="10" step="1" title="Simultaneous particles created per 40 ms generation" value={generatorNumber("number", 5)} onInput={(event) => patchGeneratorValues({ number: clamp(Math.round(Number(event.currentTarget.value) || 1), 1, 10) })} /></label>
            <label>Lifetime ms<input type="number" min="100" max="1000" step="1" title="Effect-time milliseconds, scaling with clock sync and BPM speed" value={generatorNumber("lifetime_ms", 250)} onInput={(event) => patchGeneratorValues({ lifetime_ms: clamp(Math.round(Number(event.currentTarget.value) || 100), 100, 1000) })} /></label>
            <label>Sparkle width %<input type="number" min="0.01" max="100000" step="0.1" value={generatorNumber("width", 10)} onInput={(event) => patchGeneratorValues({ width: clamp(Number(event.currentTarget.value) || 0.01, 0.01, 100000) })} /></label>
            <label>Seed<input type="number" min="0" max="4294967295" step="1" value={generatorNumber("rng_seed", 0)} onInput={(event) => patchGeneratorValues({ rng_seed: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 4_294_967_295) })} /></label>
          </div>
        </Show>
        <Show when={generatorKind() === "Plasma"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Size X<input type="number" min="0" max="20" step="1" value={generatorNumber("size_x", 1)} onInput={(event) => patchGeneratorValues({ size_x: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Param X<input type="number" min="0" max="20" step="1" value={generatorNumber("param_x", 2)} onInput={(event) => patchGeneratorValues({ param_x: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Size Y<input type="number" min="0" max="20" step="1" value={generatorNumber("size_y", 1)} onInput={(event) => patchGeneratorValues({ size_y: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Param Y<input type="number" min="0" max="20" step="1" value={generatorNumber("param_y", 2)} onInput={(event) => patchGeneratorValues({ param_y: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Speed X<input type="number" min="-5" max="5" step="1" value={generatorNumber("speed_x", -1)} onInput={(event) => patchGeneratorValues({ speed_x: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Param SX<input type="number" min="-5" max="5" step="1" value={generatorNumber("param_sx", 2)} onInput={(event) => patchGeneratorValues({ param_sx: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Speed Y<input type="number" min="-5" max="5" step="1" value={generatorNumber("speed_y", 1)} onInput={(event) => patchGeneratorValues({ speed_y: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Param SY<input type="number" min="-5" max="5" step="1" value={generatorNumber("param_sy", -1)} onInput={(event) => patchGeneratorValues({ param_sy: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
          </div>
        </Show>
        <Show when={generatorKind() === "ColorRainbow"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Color width<input type="number" min="0" max="1" step="0.01" value={generatorNumber("color_width")} onInput={(event) => patchGeneratorValues({ color_width: clamp(Number(event.currentTarget.value), 0, 1) })} /></label>
            <label>Angle °<input type="number" min="0" max="360" step="1" value={generatorNumber("angle_degrees")} onInput={(event) => patchGeneratorValues({ angle_degrees: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 360) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={generatorNumber("gradient", 100)} onInput={(event) => patchGeneratorValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
          </div>
        </Show>
        <Show when={generatorKind() === "Perlin"}>
          <div class="colorEffectModeGrid">
            <label>Transform<select data-value-perlin-transform value={generatorTransform()} onInput={(event) => patchGeneratorValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Octaves<input type="number" min="1" max="16" step="1" value={generatorNumber("octaves", 5)} onInput={(event) => patchGeneratorValues({ octaves: clamp(Math.round(Number(event.currentTarget.value) || 1), 1, 16) })} /></label>
            <label>Zoom<input type="number" min="0.0001" step="0.01" value={generatorNumber("zoom", 0.5)} onInput={(event) => patchGeneratorValues({ zoom: Math.max(0.0001, Number(event.currentTarget.value) || 0.0001) })} /></label>
            <label>Direction °<input type="number" step="0.1" value={generatorNumber("direction_degrees", 0)} onInput={(event) => patchGeneratorValues({ direction_degrees: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Speed<input type="number" min="0" step="0.1" value={generatorNumber("speed", 1)} onInput={(event) => patchGeneratorValues({ speed: Math.max(0, Number(event.currentTarget.value) || 0) })} /></label>
            <label>Amplitude %<input type="number" min="0" max="100" step="0.1" value={generatorNumber("amplitude", 100)} onInput={(event) => patchGeneratorValues({ amplitude: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
          </div>
        </Show>
      </fieldset>

      <div class="valueEffectCanvasWrap">
        <div class="valueEffectCanvasToolbar">
          <strong>{generatorKind() === "CustomEnvelope" ? "Envelope" : "Value palette"}</strong>
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
          ref={canvasEl}
          class="valueEffectCanvas"
          viewBox={`0 0 ${pathNumber(canvasWidth())} ${pathNumber(canvasHeight())}`}
          preserveAspectRatio="none"
          role="group"
          aria-label={`${generatorKind() === "CustomEnvelope" ? `${props.mode} ${props.interpolation} value envelope` : `${generatorKind()} Black 0 White 100 value palette`} with ${props.points.length} points. Double-click to add. Focus a point and use Arrow keys to nudge, Shift coarse, Alt fine, Delete to remove.`}
          onDblClick={(event) => addPoint(pointFromClient(event.currentTarget, event.clientX, event.clientY))}
          onPointerMove={continuePointDrag}
          onPointerUp={endPointDrag}
          onPointerCancel={endPointDrag}
        >
          <rect class="valueEffectCanvasBackground" x="0" y="0" width={pathNumber(canvasWidth())} height={pathNumber(canvasHeight())} />
          <g class="valueEffectCanvasGrid" aria-hidden="true">
            <For each={gridLinesX()}>
              {(x) => <line x1={pathNumber(x)} y1="0" x2={pathNumber(x)} y2={pathNumber(canvasHeight())} />}
            </For>
            <line x1="0" y1={pathNumber(gridLineY())} x2={pathNumber(canvasWidth())} y2={pathNumber(gridLineY())} />
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
                <circle r={pointHandleRadius} />
              </g>
            )}
          </For>
        </svg>
        <div class="valueEffectCanvasFooter">
          <small>Double-click to add · drag points · Arrow keys nudge · Shift coarse · Alt fine · Delete remove</small>
        </div>
      </div>

      <Show when={generatorKind() === "CustomEnvelope"}>
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
      </Show>

      <div class="valueEffectTransportRow">
        <Show when={generatorKind() === "CustomEnvelope"}>
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
        </Show>
        <label class="valueEffectPeriodField">
          Period ms
          <input
            class="tabularNums"
            type="number"
            min="10"
            step="10"
            value={normalizedPeriodMs()}
            onInput={(event) => {
              const value = Math.round(Number(event.currentTarget.value) || 10);
              props.onPeriodMs(Math.max(10, value));
            }}
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
