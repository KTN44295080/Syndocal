import { createMemo, For, Show } from "solid-js";
import type {
  ColorEffectAlgorithm,
  ColorEffectColor,
  ColorEffectInterpolation,
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
  onStops: (stops: ColorEffectStop[]) => void;
  onAlgorithm: (algorithm: ColorEffectAlgorithm) => void;
  onInterpolation: (interpolation: ColorEffectInterpolation) => void;
  onPeriodMs: (periodMs: number) => void;
  onClockSyncBeats: (beats: number | null) => void;
  onFixtureSpread: (spread: number) => void;
}

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

  const stopError = createMemo(() => {
    if (props.stops.length < 2 || props.stops.length > 8) {
      return "A color effect requires 2 to 8 palette stops.";
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

  const gradientCss = createMemo(() => {
    const stops = orderedStops();
    if (stops.length === 0) {
      return "#111416";
    }
    if (stops.length === 1) {
      return colorToHex(stops[0].color);
    }
    return `linear-gradient(90deg, ${stops
      .map((stop) => `${colorToHex(stop.color)} ${(stop.position * 100).toFixed(2)}%`)
      .join(", ")})`;
  });

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
    if (stops.length >= 8) return;
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
          <span class="tabularNums">{orderedStops().length} / 8 stops</span>
          <button type="button" onClick={addStop} disabled={orderedStops().length >= 8}>Add stop</button>
        </div>
        <Show when={stopError()}>
          {(error) => <p class="fieldError textPretty" role="alert">{error()}</p>}
        </Show>
      </fieldset>

      <fieldset class="colorEffectMotionPanel">
        <legend>Color flow</legend>
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
