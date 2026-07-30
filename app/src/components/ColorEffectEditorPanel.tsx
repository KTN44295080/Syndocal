import { createMemo, For, Show } from "solid-js";
import { buildColorGradient } from "../effectVisualization";
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

type ColorSpatialKind = "PaletteFlow" | "KnightRider" | "Burst" | "RandomFill" | "Sparkle" | "Rainbow" | "Perlin";

const defaultSpatialRecipe = (kind: Exclude<ColorSpatialKind, "PaletteFlow">): ColorEffectSpatialRecipe => {
  switch (kind) {
    case "KnightRider":
      return { KnightRider: { size: 8, one_way: false, fading: true, go_outside: false, gradient: 50 } };
    case "Burst":
      return { Burst: { color_width: 50, gradient: 100 } };
    case "RandomFill":
      return { RandomFill: { point_width: 1 } };
    case "Sparkle":
      return { Sparkle: { number: 5, lifespan: 25, width: 1 } };
    case "Rainbow":
      return { Rainbow: { vertical_symmetry: false, rotation_degrees: 0, color_width: 0, angle_degrees: 0, gradient: 100 } };
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
  const patchSpatialValues = (patch: Record<string, number | boolean>) => {
    const pattern = props.spatialPattern;
    const kind = spatialKind();
    if (!pattern || kind === "PaletteFlow") return;
    props.onSpatialPattern({
      ...pattern,
      recipe: { [kind]: { ...spatialValues(), ...patch } } as ColorEffectSpatialRecipe,
    });
  };

  const stopError = createMemo(() => {
    if (props.stops.length < 2 || props.stops.length > 16) {
      return "A color effect requires 2 to 16 palette stops.";
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
    if (orderedStops().length <= 2) return;
    publishStops(orderedStops().filter((_, candidateIndex) => candidateIndex !== index));
  };

  const addStop = () => {
    const stops = orderedStops();
    if (stops.length >= 16) return;
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
                      step="0.01"
                      value={stop.position.toFixed(2)}
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
                    disabled={orderedStops().length <= 2}
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
          <span class="tabularNums">{orderedStops().length} / 16 stops</span>
          <button type="button" onClick={addStop} disabled={orderedStops().length >= 16}>Add stop</button>
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
              value={spatialKind()}
              aria-label="Color beam-space pattern"
              onInput={(event) => selectSpatialKind(event.currentTarget.value as ColorSpatialKind)}
            >
              <option value="PaletteFlow">Palette flow</option>
              <option value="KnightRider">Knight Rider</option>
              <option value="Burst">Burst</option>
              <option value="RandomFill">Random fill</option>
              <option value="Sparkle">Sparkle</option>
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
            <label>Size<input type="number" min="1" step="1" value={spatialNumber("size", 8)} onInput={(event) => patchSpatialValues({ size: Math.max(1, Math.round(Number(event.currentTarget.value) || 1)) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 50)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label><input type="checkbox" checked={spatialBoolean("one_way")} onInput={(event) => patchSpatialValues({ one_way: event.currentTarget.checked })} /> One way only</label>
            <label><input type="checkbox" checked={spatialBoolean("fading")} onInput={(event) => patchSpatialValues({ fading: event.currentTarget.checked })} /> Fading</label>
            <label><input type="checkbox" checked={spatialBoolean("go_outside")} onInput={(event) => patchSpatialValues({ go_outside: event.currentTarget.checked })} /> Go outside</label>
          </div>
        </Show>
        <Show when={spatialKind() === "Burst"}>
          <div class="colorEffectModeGrid">
            <label>Color width %<input type="number" min="0" max="100" step="1" value={spatialNumber("color_width", 50)} onInput={(event) => patchSpatialValues({ color_width: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 100)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
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
        <Show when={spatialKind() === "Rainbow"}>
          <div class="colorEffectModeGrid">
            <label>Rotation °<input type="number" step="1" value={spatialNumber("rotation_degrees")} onInput={(event) => patchSpatialValues({ rotation_degrees: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Angle °<input type="number" step="1" value={spatialNumber("angle_degrees")} onInput={(event) => patchSpatialValues({ angle_degrees: Number(event.currentTarget.value) || 0 })} /></label>
            <label>Color width %<input type="number" min="0" max="100" step="1" value={spatialNumber("color_width")} onInput={(event) => patchSpatialValues({ color_width: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label>Gradient %<input type="number" min="0" max="100" step="1" value={spatialNumber("gradient", 100)} onInput={(event) => patchSpatialValues({ gradient: clamp(Number(event.currentTarget.value), 0, 100) })} /></label>
            <label><input type="checkbox" checked={spatialBoolean("vertical_symmetry")} onInput={(event) => patchSpatialValues({ vertical_symmetry: event.currentTarget.checked })} /> Vertical symmetry</label>
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
          Period ms
          <input
            class="tabularNums"
            type="number"
            min="10"
            step="10"
            value={props.periodMs}
            onInput={(event) => props.onPeriodMs(Math.max(10, Math.round(Number(event.currentTarget.value) || 10)))}
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
