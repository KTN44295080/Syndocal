import { createMemo, For, Show } from "solid-js";
import { buildColorGradient } from "../effectVisualization";
import { DASLIGHT_FX_PALETTE_MAX_STOPS, DASLIGHT_FX_PALETTE_MIN_STOPS } from "../fxColorPalettes";
import type {
  ColorEffectAlgorithm,
  ColorEffectColor,
  ColorEffectInterpolation,
  ColorEffectSpatialPattern,
  ColorEffectSpatialRecipe,
  ColorEffectStop,
} from "../types";

export interface ColorEffectEditorPanelProps {
  stops: ColorEffectStop[];
  algorithm: ColorEffectAlgorithm;
  interpolation: ColorEffectInterpolation;
  periodMs: number;
  bpm: number;
  clockSyncBeats: number | null;
  fixtureSpread: number;
  spatialPattern: ColorEffectSpatialPattern | null;
  onStops: (stops: ColorEffectStop[]) => void;
  onAlgorithm: (algorithm: ColorEffectAlgorithm) => void;
  onInterpolation: (interpolation: ColorEffectInterpolation) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onFixtureSpread: (spread: number) => void;
  onSpatialPattern: (pattern: ColorEffectSpatialPattern | null) => void;
}

type ColorSpatialKind = "PaletteFlow" | "KnightRider" | "Burst" | "Sweep" | "RandomFill" | "Sparkle" | "Plasma" | "ColorRainbow" | "Rainbow" | "Perlin";

const defaultSpatialRecipe = (kind: Exclude<ColorSpatialKind, "PaletteFlow">): ColorEffectSpatialRecipe => {
  switch (kind) {
    case "KnightRider":
      return { KnightRider: { grayscale: false, vertical_symmetry: false, size: 8, one_way: false, fading: true, go_outside: false, gradient: 50 } };
    case "Burst":
      return { Burst: { color_width: 50, gradient: 100 } };
    case "Sweep":
      return { Sweep: { daslight_exact: false, grayscale: false, vertical_symmetry: false, direction_change: false } };
    case "RandomFill":
      return { RandomFill: { point_width: 1 } };
    case "Sparkle":
      return { Sparkle: { number: 5, lifespan: 25, width: 1 } };
    case "Plasma":
      return { Plasma: { grayscale: false, vertical_symmetry: false, size_x: 1, param_x: 2, size_y: 1, param_y: 2, speed_x: -1, param_sx: 2, speed_y: 1, param_sy: -1 } };
    case "ColorRainbow":
      return { ColorRainbow: { grayscale: false, vertical_symmetry: false, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Rainbow":
      return { Rainbow: { grayscale: false, vertical_symmetry: false, horizontal_symmetry: false, rotation_degrees: 0, color_width: 0, angle_degrees: 0, gradient: 100 } };
    case "Perlin":
      return { Perlin: { octaves: 5, zoom: 20, direction_degrees: 0, speed: 1, amplitude: 100 } };
  }
};

export const defaultColorEffectStops: ColorEffectStop[] = [
  { position: 0, color: { red: 65_535, green: 0, blue: 0 } },
  { position: 0.5, color: { red: 0, green: 65_535, blue: 0 } },
  { position: 1, color: { red: 0, green: 0, blue: 65_535 } },
];

const clockSyncBeatPresets = [
  { label: "Free", beats: null },
  { label: "1/4", beats: 0.25 },
  { label: "1/2", beats: 0.5 },
  { label: "1", beats: 1 },
  { label: "2", beats: 2 },
  { label: "4", beats: 4 },
] as const;

// P-EXP quick looks: named one-click colour looks bundling a stop palette,
// algorithm/interpolation, a spatial recipe and clock sync
// (qa/PRESET_EXPANSION_PLAN.md). Recipe replacement preserves the pattern's
// beam targets and imported placement like patchSpatialValues does.
interface ColorQuickLook {
  label: string;
  stops: ColorEffectStop[];
  algorithm: ColorEffectAlgorithm;
  interpolation: ColorEffectInterpolation;
  recipe: ColorEffectSpatialRecipe | null;
  beats: number | null;
}
const rgb = (red: number, green: number, blue: number): ColorEffectColor => ({ red, green, blue });
const colorQuickLooks: ColorQuickLook[] = [
  {
    label: "Rainbow Flow",
    stops: [
      { position: 0, color: rgb(65_535, 0, 0) },
      { position: 0.1667, color: rgb(65_535, 65_535, 0) },
      { position: 0.3333, color: rgb(0, 65_535, 0) },
      { position: 0.5, color: rgb(0, 65_535, 65_535) },
      { position: 0.6667, color: rgb(0, 0, 65_535) },
      { position: 0.8333, color: rgb(65_535, 0, 65_535) },
      { position: 1, color: rgb(65_535, 0, 0) },
    ],
    algorithm: "Cycle",
    interpolation: "HsvShortest",
    recipe: {
      ColorRainbow: { grayscale: false, vertical_symmetry: false, color_width: 0.5, angle_degrees: 0, gradient: 100 },
    },
    beats: 4,
  },
  {
    label: "Fire Flicker",
    stops: [
      { position: 0, color: rgb(65_535, 0, 0) },
      { position: 0.4, color: rgb(65_535, 24_576, 0) },
      { position: 0.7, color: rgb(65_535, 49_151, 0) },
      { position: 1, color: rgb(65_535, 65_535, 0) },
    ],
    algorithm: "Cycle",
    interpolation: "Rgb",
    recipe: {
      Plasma: { grayscale: false, vertical_symmetry: false, size_x: 1, param_x: 2, size_y: 1, param_y: 2, speed_x: -1, param_sx: 2, speed_y: 1, param_sy: -1 },
    },
    beats: 2,
  },
  {
    label: "Ocean Drift",
    stops: [
      { position: 0, color: rgb(0, 0, 32_768) },
      { position: 0.5, color: rgb(0, 16_384, 65_535) },
      { position: 1, color: rgb(0, 65_535, 65_535) },
    ],
    algorithm: "Bounce",
    interpolation: "HsvShortest",
    recipe: { Perlin: { octaves: 5, zoom: 20, direction_degrees: 0, speed: 1, amplitude: 100 } },
    beats: 8,
  },
  {
    label: "Police Sweep",
    stops: [
      { position: 0, color: rgb(65_535, 0, 0) },
      { position: 0.49, color: rgb(65_535, 0, 0) },
      { position: 0.51, color: rgb(0, 0, 65_535) },
      { position: 1, color: rgb(0, 0, 65_535) },
    ],
    algorithm: "Cycle",
    interpolation: "Rgb",
    recipe: { Sweep: { daslight_exact: false, grayscale: false, vertical_symmetry: false, direction_change: true } },
    beats: 1,
  },
  {
    label: "White Sparkle",
    stops: [
      { position: 0, color: rgb(65_535, 65_535, 65_535) },
      { position: 1, color: rgb(0, 0, 0) },
    ],
    algorithm: "Cycle",
    interpolation: "Rgb",
    recipe: { Sparkle: { number: 8, lifespan: 25, width: 1 } },
    beats: 0.5,
  },
];

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

const clampChannel = (value: number) => Math.round(clamp(value, 0, 65_535));
const clampPosition = (value: number) => clamp(value, 0, 1);
const clampSpread = (value: number) => clamp(value, 0, 1);

const normalizedColor = (color: ColorEffectColor): ColorEffectColor => ({
  red: clampChannel(color.red),
  green: clampChannel(color.green),
  blue: clampChannel(color.blue),
});

const normalizedStop = (stop: ColorEffectStop): ColorEffectStop => ({
  position: clampPosition(stop.position),
  color: normalizedColor(stop.color),
});

const colorToHex = (color: ColorEffectColor) => {
  const byte = (value: number) => Math.round(clampChannel(value) / 257).toString(16).padStart(2, "0");
  return `#${byte(color.red)}${byte(color.green)}${byte(color.blue)}`;
};

const colorFromHex = (value: string): ColorEffectColor | null => {
  const normalized = value.trim().toLowerCase();
  if (!/^#[0-9a-f]{6}$/.test(normalized)) {
    return null;
  }
  return {
    red: Number.parseInt(normalized.slice(1, 3), 16) * 257,
    green: Number.parseInt(normalized.slice(3, 5), 16) * 257,
    blue: Number.parseInt(normalized.slice(5, 7), 16) * 257,
  };
};

const mixColor = (first: ColorEffectColor, second: ColorEffectColor): ColorEffectColor => ({
  red: Math.round((first.red + second.red) / 2),
  green: Math.round((first.green + second.green) / 2),
  blue: Math.round((first.blue + second.blue) / 2),
});

const nearlyEqual = (first: number | null, second: number | null) =>
  first === null || second === null ? first === second : Math.abs(first - second) < 0.001;

export function ColorEffectEditorPanel(props: ColorEffectEditorPanelProps) {
  const orderedStops = createMemo(() =>
    props.stops
      .map(normalizedStop)
      .sort((first, second) => first.position - second.position),
  );
  const spatialKind = createMemo<ColorSpatialKind>(() => {
    const recipe = props.spatialPattern?.recipe;
    if (!recipe) return "PaletteFlow";
    return Object.keys(recipe)[0] as Exclude<ColorSpatialKind, "PaletteFlow">;
  });
  const spatialValues = createMemo<Record<string, number | boolean>>(() => {
    const recipe = props.spatialPattern?.recipe;
    if (!recipe) return {};
    return Object.values(recipe)[0] as Record<string, number | boolean>;
  });
  const spatialNumber = (key: string, fallback = 0) => {
    const value = spatialValues()[key];
    return typeof value === "number" && Number.isFinite(value) ? value : fallback;
  };
  const spatialBoolean = (key: string) => spatialValues()[key] === true;
  const daslightExactBurst = createMemo(() =>
    spatialKind() === "Burst" && spatialBoolean("daslight_exact"),
  );
  const daslightExactSweep = createMemo(() =>
    spatialKind() === "Sweep" && spatialBoolean("daslight_exact"),
  );
  const daslightExactTiming = createMemo(() => daslightExactBurst() || daslightExactSweep());
  const spatialTransform = () => spatialBoolean("vertical_symmetry")
    ? "vertical"
    : spatialBoolean("horizontal_symmetry")
      ? "horizontal"
      : "none";
  const selectSpatialKind = (kind: ColorSpatialKind) => {
    if (kind === "PaletteFlow") {
      props.onSpatialPattern(null);
      return;
    }
    props.onSpatialPattern({
      recipe: defaultSpatialRecipe(kind),
      beam_targets: props.spatialPattern?.beam_targets ?? [],
    });
  };
  const applyQuickLook = (look: ColorQuickLook) => {
    props.onStops(look.stops.map((stop) => ({ ...stop, color: { ...stop.color } })));
    props.onAlgorithm(look.algorithm);
    props.onInterpolation(look.interpolation);
    if (look.recipe === null) {
      props.onSpatialPattern(null);
    } else {
      const pattern = props.spatialPattern;
      const recipe = structuredClone(look.recipe);
      props.onSpatialPattern(pattern ? { ...pattern, recipe } : { recipe, beam_targets: [] });
    }
    props.onClockSyncBeats(look.beats);
  };
  const patchSpatialValues = (patch: Record<string, number | boolean>) => {
    const pattern = props.spatialPattern;
    const kind = spatialKind();
    if (!pattern || kind === "PaletteFlow") return;
    props.onSpatialPattern({
      ...pattern,
      recipe: { [kind]: { ...spatialValues(), ...patch } } as ColorEffectSpatialRecipe,
    });
  };
  const setBurstEvaluator = (daslightExact: boolean) => {
    if (spatialKind() !== "Burst") return;
    const currentWidth = spatialNumber("color_width", 50);
    const currentGradient = spatialNumber("gradient", daslightExactBurst() ? 1 : 100);
    patchSpatialValues(daslightExact
      ? {
          daslight_exact: true,
          color_width: clamp(Math.round(currentWidth), 10, 900),
          gradient: clamp(currentGradient > 1 ? currentGradient / 100 : currentGradient, 0, 1),
        }
      : {
          daslight_exact: false,
          grayscale: false,
          vertical_symmetry: false,
          color_width: clamp(currentWidth, 0, 100),
          gradient: clamp(currentGradient <= 1 ? currentGradient * 100 : currentGradient, 0, 100),
        });
    if (daslightExact) {
      props.onPeriodMs(Math.max(40, Math.floor(Math.max(40, props.periodMs) / 40) * 40));
    }
  };
  const setSweepEvaluator = (daslightExact: boolean) => {
    if (spatialKind() !== "Sweep") return;
    patchSpatialValues({ daslight_exact: daslightExact });
    if (daslightExact) {
      props.onPeriodMs(Math.max(40, Math.floor(Math.max(40, props.periodMs) / 40) * 40));
    }
  };

  const stopError = createMemo(() => {
    if (props.stops.length < DASLIGHT_FX_PALETTE_MIN_STOPS || props.stops.length > DASLIGHT_FX_PALETTE_MAX_STOPS) {
      return "A color effect requires 1 to 255 palette stops.";
    }
    if (
      props.stops.some(
        (stop) =>
          !Number.isFinite(stop.position) ||
          stop.position < 0 ||
          stop.position > 1 ||
          !Number.isFinite(stop.color.red) ||
          !Number.isFinite(stop.color.green) ||
          !Number.isFinite(stop.color.blue) ||
          stop.color.red < 0 ||
          stop.color.red > 65_535 ||
          stop.color.green < 0 ||
          stop.color.green > 65_535 ||
          stop.color.blue < 0 ||
          stop.color.blue > 65_535,
      )
    ) {
      return "Palette colors and positions must be within range.";
    }
    const ordered = [...props.stops].sort((first, second) => first.position - second.position);
    if (ordered.some((stop, index) => index > 0 && stop.position <= ordered[index - 1].position)) {
      return "Each palette stop needs a unique position.";
    }
    return "";
  });

  const gradientCss = createMemo(() => buildColorGradient(orderedStops(), props.interpolation));

  const spreadPercent = createMemo(() => Math.round(clampSpread(props.fixtureSpread) * 100));
  const clockSummary = createMemo(() =>
    props.clockSyncBeats === null
      ? `${Math.max(10, Math.round(props.periodMs))} ms free`
      : `${props.clockSyncBeats} beat sync`,
  );

  const beatPeriodMs = (beats: number) => {
    const bpm = Number.isFinite(props.bpm) && props.bpm > 0 ? props.bpm : 120;
    return Math.max(10, Math.round((60_000 / bpm) * beats));
  };

  const publishStops = (stops: ColorEffectStop[]) => {
    props.onStops(stops.map(normalizedStop).sort((first, second) => first.position - second.position));
  };

  const updateStop = (index: number, patch: Partial<ColorEffectStop>) => {
    const next = orderedStops().map((stop) => ({ ...stop, color: { ...stop.color } }));
    const current = next[index];
    if (!current) return;
    next[index] = {
      ...current,
      ...patch,
      color: patch.color ? normalizedColor(patch.color) : current.color,
    };
    publishStops(next);
  };

  const moveStop = (index: number, direction: -1 | 1) => {
    const targetIndex = index + direction;
    const next = orderedStops().map((stop) => ({ ...stop, color: { ...stop.color } }));
    if (!next[index] || !next[targetIndex]) return;
    const color = next[index].color;
    next[index] = { ...next[index], color: next[targetIndex].color };
    next[targetIndex] = { ...next[targetIndex], color };
    publishStops(next);
  };

  const removeStop = (index: number) => {
    if (orderedStops().length <= DASLIGHT_FX_PALETTE_MIN_STOPS) return;
    publishStops(orderedStops().filter((_, candidateIndex) => candidateIndex !== index));
  };

  const addStop = () => {
    const stops = orderedStops();
    if (stops.length >= DASLIGHT_FX_PALETTE_MAX_STOPS) return;
    if (stops.length === 0) {
      publishStops(defaultColorEffectStops.slice(0, 2).map((stop) => ({ ...stop, color: { ...stop.color } })));
      return;
    }
    if (stops.length === 1) {
      const position = stops[0].position < 0.5 ? 1 : 0;
      publishStops([...stops, { position, color: { ...stops[0].color } }]);
      return;
    }

    let left = stops[0];
    let right = stops[1];
    let largestGap = right.position - left.position;
    for (let index = 1; index < stops.length - 1; index += 1) {
      const gap = stops[index + 1].position - stops[index].position;
      if (gap > largestGap) {
        largestGap = gap;
        left = stops[index];
        right = stops[index + 1];
      }
    }
    if (largestGap <= 0.000_1) {
      const colors = [...stops.map((stop) => stop.color), mixColor(stops[0].color, stops[stops.length - 1].color)];
      publishStops(colors.map((color, index) => ({ position: index / (colors.length - 1), color })));
      return;
    }
    publishStops([
      ...stops,
      {
        position: (left.position + right.position) / 2,
        color: mixColor(left.color, right.color),
      },
    ]);
  };

  const commitHex = (index: number, input: HTMLInputElement) => {
    const stop = orderedStops()[index];
    if (!stop) return;
    const color = colorFromHex(input.value);
    if (!color) {
      input.value = colorToHex(stop.color);
      return;
    }
    updateStop(index, { color });
  };

  return (
    <section class="colorEffectEditor" aria-label="Color effect editor">
      <fieldset class="colorEffectPalettePanel">
        <legend>Palette stops</legend>
        <div
          class="colorEffectGradientPreview"
          style={{ background: gradientCss() }}
          role="img"
          aria-label={`Color effect gradient with ${orderedStops().length} stops`}
          data-stop-positions={orderedStops().map((stop) => stop.position).join(",")}
          data-color-interpolation={props.interpolation}
        >
          <For each={orderedStops()}>
            {(stop, index) => (
              <span
                class="colorEffectGradientMarker"
                style={{ left: `${stop.position * 100}%`, "--stop-color": colorToHex(stop.color) }}
                title={`Stop ${index() + 1}: ${colorToHex(stop.color)} at ${Math.round(stop.position * 100)}%`}
                aria-hidden="true"
              />
            )}
          </For>
        </div>
        <div class="colorEffectStopList" aria-label="Color palette stops">
          <For each={orderedStops()}>
            {(stop, index) => {
              const stopNumber = () => index() + 1;
              const hex = () => colorToHex(stop.color);
              return (
                <div class="colorEffectStopRow">
                  <span class="colorEffectStopNumber tabularNums" aria-hidden="true">{stopNumber()}</span>
                  <input
                    class="colorEffectStopPicker"
                    type="color"
                    value={hex()}
                    aria-label={`Color for palette stop ${stopNumber()}`}
                    onInput={(event) => {
                      const color = colorFromHex(event.currentTarget.value);
                      if (color) updateStop(index(), { color });
                    }}
                  />
                  <input
                    class="colorEffectStopHex tabularNums"
                    type="text"
                    value={hex()}
                    inputMode="text"
                    maxLength={7}
                    pattern="#[0-9A-Fa-f]{6}"
                    spellcheck={false}
                    aria-label={`Hex color for palette stop ${stopNumber()}`}
                    onChange={(event) => commitHex(index(), event.currentTarget)}
                    onKeyDown={(event) => {
                      if (event.key === "Enter") {
                        commitHex(index(), event.currentTarget);
                        event.currentTarget.blur();
                      } else if (event.key === "Escape") {
                        event.currentTarget.value = hex();
                        event.currentTarget.blur();
                      }
                    }}
                  />
                  <label class="colorEffectStopPosition">
                    <span>Position</span>
                    <input
                      class="tabularNums"
                      type="number"
                      min="0"
                      max="1"
                      step="0.0001"
                      value={stop.position.toFixed(4)}
                      aria-label={`Position for palette stop ${stopNumber()}`}
                      onChange={(event) => updateStop(index(), { position: Number(event.currentTarget.value) })}
                    />
                  </label>
                  <button
                    type="button"
                    class="colorEffectStopAction"
                    aria-label={`Move palette stop ${stopNumber()} up`}
                    title="Move stop up"
                    disabled={index() === 0}
                    onClick={() => moveStop(index(), -1)}
                  >
                    ↑
                  </button>
                  <button
                    type="button"
                    class="colorEffectStopAction"
                    aria-label={`Move palette stop ${stopNumber()} down`}
                    title="Move stop down"
                    disabled={index() === orderedStops().length - 1}
                    onClick={() => moveStop(index(), 1)}
                  >
                    ↓
                  </button>
                  <button
                    type="button"
                    class="colorEffectStopAction remove"
                    aria-label={`Remove palette stop ${stopNumber()}`}
                    title="Remove stop"
                    disabled={orderedStops().length <= DASLIGHT_FX_PALETTE_MIN_STOPS}
                    onClick={() => removeStop(index())}
                  >
                    ×
                  </button>
                </div>
              );
            }}
          </For>
        </div>
        <div class="colorEffectPaletteFooter">
          <span class="tabularNums">{orderedStops().length} / 255 stops</span>
          <button type="button" onClick={addStop} disabled={orderedStops().length >= DASLIGHT_FX_PALETTE_MAX_STOPS}>Add stop</button>
        </div>
        <Show when={stopError()}>
          {(error) => <p class="fieldError textPretty" role="alert">{error()}</p>}
        </Show>
      </fieldset>

      <fieldset class="colorEffectMotionPanel">
        <legend>Color flow</legend>
        <div class="colorEffectModeGrid">
          <label>
            Beam-space pattern
            <select
              data-color-spatial-select
              value={spatialKind()}
              aria-label="Color beam-space pattern"
              onInput={(event) => selectSpatialKind(event.currentTarget.value as ColorSpatialKind)}
            >
              <option value="PaletteFlow">Palette flow</option>
              <option value="KnightRider">Knight Rider</option>
              <option value="Burst">Burst</option>
              <option value="Sweep">Sweep</option>
              <option value="RandomFill">Random fill</option>
              <option value="Sparkle">Sparkle</option>
              <option value="Plasma">Plasma</option>
              <option value="ColorRainbow">Rainbow strip</option>
              <option value="Rainbow">Rainbow mapping</option>
              <option value="Perlin">Perlin mapping</option>
            </select>
          </label>
          <Show when={props.spatialPattern}>
            <div class="effectFormHint textPretty">
              {props.spatialPattern?.beam_targets?.length
                ? `${props.spatialPattern.beam_targets.length} imported beam targets`
                : "Beam targets follow fixture profile channel order."}
            </div>
          </Show>
        </div>
        <Show when={spatialKind() === "KnightRider"}>
          <div class="colorEffectModeGrid">
            <label><input type="checkbox" checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
            <label>Transform<select value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Size<input type="number" min="1" step="1" value={spatialNumber("size", 8)} onInput={(event) => patchSpatialValues({ size: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 50)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label><input type="checkbox" checked={spatialBoolean("one_way")} onInput={(event) => patchSpatialValues({ one_way: event.currentTarget.checked })} /> One way only</label>
            <label><input type="checkbox" checked={spatialBoolean("fading")} onInput={(event) => patchSpatialValues({ fading: event.currentTarget.checked })} /> Fading</label>
            <label><input type="checkbox" checked={spatialBoolean("go_outside")} onInput={(event) => patchSpatialValues({ go_outside: event.currentTarget.checked })} /> Go outside</label>
          </div>
        </Show>
        <Show when={spatialKind() === "Burst"}>
          <div class="colorEffectModeGrid">
            <label>Evaluator<select value={daslightExactBurst() ? "daslight" : "enhanced"} onInput={(event) => setBurstEvaluator(event.currentTarget.value === "daslight")}><option value="enhanced">Enhanced</option><option value="daslight">Daslight exact</option></select></label>
            <Show when={daslightExactBurst()}>
              <label><input type="checkbox" checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
              <label>Transform<select value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            </Show>
            <label>{daslightExactBurst() ? "Color width px" : "Color width %"}<input type="number" min={daslightExactBurst() ? "10" : "0"} max={daslightExactBurst() ? "900" : "100"} step="1" value={spatialNumber("color_width", 50)} onInput={(event) => patchSpatialValues({ color_width: daslightExactBurst() ? clamp(Math.round(Number(event.currentTarget.value) || 10), 10, 900) : clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>{daslightExactBurst() ? "Gradient 0..1" : "Gradient %"}<input type="number" min="0" max={daslightExactBurst() ? "1" : "100"} step={daslightExactBurst() ? "0.01" : "1"} value={spatialNumber("gradient", daslightExactBurst() ? 1 : 100)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, daslightExactBurst() ? 1 : 100) })} /></label>
          </div>
        </Show>
        <Show when={spatialKind() === "Sweep"}>
          <div class="colorEffectModeGrid" data-color-sweep-controls>
            <label>Evaluator<select data-color-sweep-evaluator value={daslightExactSweep() ? "daslight" : "enhanced"} onInput={(event) => setSweepEvaluator(event.currentTarget.value === "daslight")}><option value="enhanced">Enhanced</option><option value="daslight">Daslight exact</option></select></label>
            <label><input type="checkbox" data-color-sweep-grayscale checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
            <label>Transform<select data-color-sweep-transform value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label><input type="checkbox" data-color-sweep-direction-change checked={spatialBoolean("direction_change")} onInput={(event) => patchSpatialValues({ direction_change: event.currentTarget.checked })} /> Direction change</label>
          </div>
        </Show>
        <Show when={spatialKind() === "RandomFill"}>
          <div class="colorEffectModeGrid">
            <label>Point width<input type="number" min="1" step="1" value={spatialNumber("point_width", 1)} onInput={(event) => patchSpatialValues({ point_width: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} /></label>
          </div>
        </Show>
        <Show when={spatialKind() === "Sparkle"}>
          <div class="colorEffectModeGrid">
            <label>Sparkle number<input type="number" min="1" step="1" value={spatialNumber("number", 5)} onInput={(event) => patchSpatialValues({ number: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} /></label>
            <label>Life span %<input type="number" min="0" max="100" step="1" value={spatialNumber("lifespan", 25)} onInput={(event) => patchSpatialValues({ lifespan: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>Sparkle width<input type="number" min="1" step="1" value={spatialNumber("width", 1)} onInput={(event) => patchSpatialValues({ width: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} /></label>
          </div>
        </Show>
        <Show when={spatialKind() === "Plasma"}>
          <div class="colorEffectModeGrid">
            <label><input type="checkbox" checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
            <label>Transform<select value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Size X<input type="number" min="0" max="20" step="1" value={spatialNumber("size_x", 1)} onInput={(event) => patchSpatialValues({ size_x: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Param X<input type="number" min="0" max="20" step="1" value={spatialNumber("param_x", 2)} onInput={(event) => patchSpatialValues({ param_x: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Size Y<input type="number" min="0" max="20" step="1" value={spatialNumber("size_y", 1)} onInput={(event) => patchSpatialValues({ size_y: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Param Y<input type="number" min="0" max="20" step="1" value={spatialNumber("param_y", 2)} onInput={(event) => patchSpatialValues({ param_y: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 20) })} /></label>
            <label>Speed X<input type="number" min="-5" max="5" step="1" value={spatialNumber("speed_x", -1)} onInput={(event) => patchSpatialValues({ speed_x: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Param SX<input type="number" min="-5" max="5" step="1" value={spatialNumber("param_sx", 2)} onInput={(event) => patchSpatialValues({ param_sx: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Speed Y<input type="number" min="-5" max="5" step="1" value={spatialNumber("speed_y", 1)} onInput={(event) => patchSpatialValues({ speed_y: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
            <label>Param SY<input type="number" min="-5" max="5" step="1" value={spatialNumber("param_sy", -1)} onInput={(event) => patchSpatialValues({ param_sy: clamp(Math.round(Number(event.currentTarget.value) || 0), -5, 5) })} /></label>
          </div>
        </Show>
        <Show when={spatialKind() === "ColorRainbow"}>
          <div class="colorEffectModeGrid">
            <label><input type="checkbox" checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
            <label>Transform<select value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option></select></label>
            <label>Color width<input type="number" min="0" max="1" step="0.01" value={spatialNumber("color_width")} onInput={(event) => patchSpatialValues({ color_width: clamp(Number(event.currentTarget.value), 0, 1) })} /></label>
            <label>Angle °<input type="number" min="0" max="360" step="1" value={spatialNumber("angle_degrees")} onInput={(event) => patchSpatialValues({ angle_degrees: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 360) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 100)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
          </div>
        </Show>
        <Show when={spatialKind() === "Rainbow"}>
          <div class="colorEffectModeGrid">
            <label><input type="checkbox" checked={spatialBoolean("grayscale")} onInput={(event) => patchSpatialValues({ grayscale: event.currentTarget.checked })} /> Grayscale</label>
            <label>Rotation °<input type="number" step="1" value={spatialNumber("rotation_degrees")} onInput={(event) => patchSpatialValues({ rotation_degrees: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Angle °<input type="number" min="0" max="360" step="1" value={spatialNumber("angle_degrees")} onInput={(event) => patchSpatialValues({ angle_degrees: clamp(Math.round(Number(event.currentTarget.value) || 0), 0, 360) })} /></label>
            <label>Color width %<input type="number" min="0" max="100" step="1" value={spatialNumber("color_width")} onInput={(event) => patchSpatialValues({ color_width: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 100)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>Transform<select value={spatialTransform()} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.value === "vertical", horizontal_symmetry: event.currentTarget.value === "horizontal" })}><option value="none">None</option><option value="vertical">Vertical symmetry</option><option value="horizontal">Horizontal symmetry</option></select></label>
          </div>
        </Show>
        <Show when={spatialKind() === "Perlin"}>
          <div class="colorEffectModeGrid">
            <label>Octaves<input type="number" min="1" max="16" step="1" value={spatialNumber("octaves", 5)} onInput={(event) => patchSpatialValues({ octaves: clamp(Math.round(Number(event.currentTarget.value) || 1), 1, 16) })} /></label>
            <label>Zoom<input type="number" min="0.01" step="0.1" value={spatialNumber("zoom", 20)} onInput={(event) => patchSpatialValues({ zoom: Math.max(0.01, Number(event.currentTarget.value) || 0.01) })} /></label>
            <label>Direction °<input type="number" step="1" value={spatialNumber("direction_degrees")} onInput={(event) => patchSpatialValues({ direction_degrees: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Speed<input type="number" step="0.1" value={spatialNumber("speed", 1)} onInput={(event) => patchSpatialValues({ speed: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Amplitude %<input type="number" min="0" max="100" step="1" value={spatialNumber("amplitude", 100)} onInput={(event) => patchSpatialValues({ amplitude: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
          </div>
        </Show>
        <div class="colorEffectModeGrid">
          <label>
            Algorithm
            <select
              value={props.algorithm}
              onInput={(event) => props.onAlgorithm(event.currentTarget.value as ColorEffectAlgorithm)}
            >
              <option value="Cycle">Cycle</option>
              <option value="Bounce">Bounce</option>
              <option value="Sequence">Sequence</option>
              <option value="Random">Random</option>
            </select>
          </label>
          <label>
            Interpolation
            <select
              value={props.interpolation}
              onInput={(event) => props.onInterpolation(event.currentTarget.value as ColorEffectInterpolation)}
            >
              <option value="Rgb">RGB</option>
              <option value="HsvShortest">HSV shortest</option>
              <option value="HsvLongest">HSV longest</option>
            </select>
          </label>
        </div>
        <div class="colorEffectClockHeader">
          <strong>Clock</strong>
          <span class="tabularNums">{clockSummary()}</span>
        </div>
        <label class="colorEffectPeriodInput">
          {daslightExactTiming() ? "Period ms · 40 ms compatibility" : "Period ms"}
          <input
            class="tabularNums"
            type="number"
            min={daslightExactTiming() ? "40" : "10"}
            step={daslightExactTiming() ? "40" : "10"}
            value={props.periodMs}
            onInput={(event) => {
              const value = Math.round(Number(event.currentTarget.value) || (daslightExactTiming() ? 40 : 10));
              props.onPeriodMs(daslightExactTiming() ? Math.max(40, Math.floor(value / 40) * 40) : Math.max(10, value));
            }}
          />
        </label>
        <div class="colorEffectClockPresets" aria-label="Color effect clock sync presets">
          <For each={clockSyncBeatPresets}>
            {(preset) => (
              <button
                type="button"
                class={nearlyEqual(props.clockSyncBeats, preset.beats) ? "active" : ""}
                aria-pressed={nearlyEqual(props.clockSyncBeats, preset.beats)}
                onClick={() => props.onClockSyncBeats(preset.beats)}
              >
                <strong>{preset.label}</strong>
                <span class="tabularNums">{preset.beats === null ? "manual" : `${beatPeriodMs(preset.beats)} ms`}</span>
              </button>
            )}
          </For>
        </div>
        <div class="moveEffectClockPresets" aria-label="Color quick looks" data-color-quick-looks>
          <For each={colorQuickLooks}>
            {(look) => (
              <button
                type="button"
                data-color-quick-look={look.label}
                title={look.beats === null ? `${look.label}: free-running` : `${look.label}: ${look.beats} beat${look.beats === 1 ? "" : "s"}`}
                onClick={() => applyQuickLook(look)}
              >
                {look.label}
              </button>
            )}
          </For>
        </div>
        <label class="colorEffectSpreadControl">
          <span>Fixture spread</span>
          <input
            type="range"
            min="0"
            max="100"
            step="1"
            value={spreadPercent()}
            aria-label="Fixture color phase spread"
            onInput={(event) => props.onFixtureSpread(Number(event.currentTarget.value) / 100)}
          />
          <input
            class="tabularNums"
            type="number"
            min="0"
            max="100"
            step="1"
            value={spreadPercent()}
            aria-label="Fixture color phase spread percent"
            onChange={(event) => props.onFixtureSpread(clampSpread(Number(event.currentTarget.value) / 100))}
          />
          <strong aria-hidden="true">%</strong>
        </label>
      </fieldset>
    </section>
  );
}
