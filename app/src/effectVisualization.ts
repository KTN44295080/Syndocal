import type {
  ColorEffectColor,
  ColorEffectInterpolation,
  ColorEffectStop,
  LfoShape,
  MoveEffectRequest,
  MoveInterpolation,
  MovePathPoint,
  ValueEffectDirection,
  ValueEffectInterpolation,
  ValueEffectPoint,
} from "./types";

export interface PreviewPoint {
  x: number;
  y: number;
}

const clamp = (value: number, minimum: number, maximum: number) =>
  Math.min(maximum, Math.max(minimum, Number.isFinite(value) ? value : minimum));

export const clampUnit = (value: number) => clamp(value, 0, 1);
const pathNumber = (value: number) => Number(value.toFixed(3));
const normalizePhase = (phase: number) => ((Number.isFinite(phase) ? phase : 0) % 1 + 1) % 1;

/** Mirrors engine::evaluate_lfo_shape, including the deterministic noise hash. */
export const evaluateLfoShape = (shape: LfoShape, phase: number): number => {
  const normalized = normalizePhase(phase);
  switch (shape) {
    case "Sine":
      return (Math.sin(normalized * Math.PI * 2) + 1) * 0.5;
    case "Cosine":
      return (Math.cos(normalized * Math.PI * 2) + 1) * 0.5;
    case "Triangle":
      return normalized < 0.5 ? normalized * 2 : (1 - normalized) * 2;
    case "Saw":
      return normalized;
    case "Square":
      return normalized < 0.5 ? 1 : 0;
    case "Random":
      return hashUnitFloat(Math.floor(normalized * 16) >>> 0);
    case "Perlin": {
      const x = normalized * 8;
      const left = Math.floor(x) >>> 0;
      const right = (left + 1) % 8;
      const progress = x - left;
      const eased = progress * progress * (3 - 2 * progress);
      const first = hashUnitFloat(left);
      return first + (hashUnitFloat(right) - first) * eased;
    }
  }
};

const hashUnitFloat = (seed: number): number => {
  let value = (seed + 0x9e37_79b9) >>> 0;
  value = (value ^ (value >>> 16)) >>> 0;
  value = Math.imul(value, 0x7feb_352d) >>> 0;
  value = (value ^ (value >>> 15)) >>> 0;
  value = Math.imul(value, 0x846c_a68b) >>> 0;
  value = (value ^ (value >>> 16)) >>> 0;
  return value / 0xffff_ffff;
};

export const buildLfoPreviewPath = (
  shape: LfoShape,
  phase: number,
  width = 100,
  height = 32,
  samples = 64,
  low = 0,
  high = 65_535,
  directed = true,
): string => {
  const count = Math.max(2, Math.round(samples));
  const first = clamp(low, 0, 65_535);
  const second = clamp(high, 0, 65_535);
  const from = directed ? first : Math.min(first, second);
  const to = directed ? second : Math.max(first, second);
  const segments: string[] = [];
  for (let index = 0; index <= count; index += 1) {
    const progress = index / count;
    const normalized = evaluateLfoShape(shape, progress + phase);
    const output = from + (to - from) * normalized;
    segments.push(`${index === 0 ? "M" : "L"} ${pathNumber(progress * width)} ${pathNumber((1 - output / 65_535) * height)}`);
  }
  return segments.join(" ");
};

export const color16ToHex = (red: number, green: number, blue: number): string => {
  const byte = (value: number) => Math.round(clamp(value, 0, 65_535) / 257).toString(16).padStart(2, "0");
  return `#${byte(red)}${byte(green)}${byte(blue)}`;
};

const colorToHsv = (color: ColorEffectColor): [number, number, number] => {
  const red = color.red / 65_535;
  const green = color.green / 65_535;
  const blue = color.blue / 65_535;
  const maximum = Math.max(red, green, blue);
  const minimum = Math.min(red, green, blue);
  const delta = maximum - minimum;
  let hue = 0;
  if (delta > Number.EPSILON) {
    if (maximum === red) hue = 60 * (((green - blue) / delta) % 6);
    else if (maximum === green) hue = 60 * ((blue - red) / delta + 2);
    else hue = 60 * ((red - green) / delta + 4);
  }
  return [((hue % 360) + 360) % 360, maximum <= Number.EPSILON ? 0 : delta / maximum, maximum];
};

const hsvToColor = (hue: number, saturation: number, value: number): ColorEffectColor => {
  const normalizedHue = ((hue % 360) + 360) % 360;
  const normalizedSaturation = clampUnit(saturation);
  const normalizedValue = clampUnit(value);
  const chroma = normalizedValue * normalizedSaturation;
  const section = normalizedHue / 60;
  const x = chroma * (1 - Math.abs((section % 2) - 1));
  let red = 0;
  let green = 0;
  let blue = 0;
  switch (Math.floor(section)) {
    case 0: red = chroma; green = x; break;
    case 1: red = x; green = chroma; break;
    case 2: green = chroma; blue = x; break;
    case 3: green = x; blue = chroma; break;
    case 4: red = x; blue = chroma; break;
    default: red = chroma; blue = x; break;
  }
  const offset = normalizedValue - chroma;
  return {
    red: Math.round((red + offset) * 65_535),
    green: Math.round((green + offset) * 65_535),
    blue: Math.round((blue + offset) * 65_535),
  };
};

export const interpolateEffectColor = (
  from: ColorEffectColor,
  to: ColorEffectColor,
  amount: number,
  interpolation: ColorEffectInterpolation,
): ColorEffectColor => {
  const normalized = clampUnit(amount);
  if (interpolation === "Rgb") {
    return {
      red: Math.round(from.red + (to.red - from.red) * normalized),
      green: Math.round(from.green + (to.green - from.green) * normalized),
      blue: Math.round(from.blue + (to.blue - from.blue) * normalized),
    };
  }
  let [fromHue, fromSaturation, fromValue] = colorToHsv(from);
  let [toHue, toSaturation, toValue] = colorToHsv(to);
  if (fromSaturation <= Number.EPSILON) fromHue = toHue;
  if (toSaturation <= Number.EPSILON) toHue = fromHue;
  let delta = ((toHue - fromHue + 540) % 360) - 180;
  if (Math.abs(delta + 180) <= Number.EPSILON) delta = 180;
  if (interpolation === "HsvLongest") {
    if (Math.abs(delta) <= Number.EPSILON) {
      if (fromSaturation > Number.EPSILON && toSaturation > Number.EPSILON) delta = 360;
    } else {
      delta = delta > 0 ? delta - 360 : delta + 360;
    }
  }
  return hsvToColor(
    fromHue + delta * normalized,
    fromSaturation + (toSaturation - fromSaturation) * normalized,
    fromValue + (toValue - fromValue) * normalized,
  );
};

export const sampleColorStops = (
  stops: ColorEffectStop[],
  interpolation: ColorEffectInterpolation,
  position: number,
): ColorEffectColor | null => {
  if (stops.length === 0) return null;
  const ordered = [...stops].sort((first, second) => first.position - second.position);
  const normalized = clampUnit(position);
  if (normalized <= ordered[0].position) return ordered[0].color;
  for (let index = 0; index < ordered.length - 1; index += 1) {
    const first = ordered[index];
    const second = ordered[index + 1];
    if (normalized <= second.position) {
      const amount = (normalized - first.position) / Math.max(Number.EPSILON, second.position - first.position);
      return interpolateEffectColor(first.color, second.color, amount, interpolation);
    }
  }
  return ordered[ordered.length - 1].color;
};

export const buildColorGradient = (
  stops: ColorEffectStop[],
  interpolation: ColorEffectInterpolation = "Rgb",
  samples = 48,
): string => {
  if (stops.length === 0) return "transparent";
  const sampleCount = Math.max(1, Math.round(samples));
  const entries = Array.from({ length: sampleCount + 1 }, (_, index) => {
    const position = index / sampleCount;
    const color = sampleColorStops(stops, interpolation, position) ?? stops[0].color;
    return `${color16ToHex(color.red, color.green, color.blue)} ${pathNumber(position * 100)}%`;
  });
  return `linear-gradient(90deg, ${entries.join(", ")})`;
};

const pointDistance = (first: PreviewPoint, second: PreviewPoint) =>
  Math.hypot(second.x - first.x, second.y - first.y);
const lerpPoint = (first: PreviewPoint, second: PreviewPoint, amount: number): PreviewPoint => ({
  x: first.x + (second.x - first.x) * amount,
  y: first.y + (second.y - first.y) * amount,
});
const parameterizedLerp = (
  first: PreviewPoint,
  second: PreviewPoint,
  firstTime: number,
  secondTime: number,
  at: number,
) => lerpPoint(first, second, (at - firstTime) / Math.max(0.000_001, secondTime - firstTime));
const centripetalStep = (first: PreviewPoint, second: PreviewPoint) =>
  Math.max(0.000_1, Math.sqrt(pointDistance(first, second)));
const centripetalPoint = (
  previous: PreviewPoint,
  first: PreviewPoint,
  second: PreviewPoint,
  next: PreviewPoint,
  amount: number,
): PreviewPoint => {
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

export const sampleMovePath = (
  points: PreviewPoint[],
  interpolation: MoveInterpolation,
  closed: boolean,
): PreviewPoint[] => {
  if (points.length < 2 || interpolation === "Line") return [...points];
  const samples: PreviewPoint[] = [points[0]];
  const segmentCount = closed ? points.length : points.length - 1;
  for (let segment = 0; segment < segmentCount; segment += 1) {
    const fromIndex = Math.min(segment, points.length - 1);
    const toIndex = fromIndex + 1 < points.length ? fromIndex + 1 : 0;
    const first = points[fromIndex];
    const second = points[toIndex];
    const previous = fromIndex === 0 ? (closed ? points[points.length - 1] : first) : points[fromIndex - 1];
    const nextIndex = toIndex + 1;
    const next = nextIndex < points.length ? points[nextIndex] : (closed ? points[nextIndex % points.length] : second);
    for (let sample = 1; sample <= 32; sample += 1) {
      samples.push(centripetalPoint(previous, first, second, next, sample / 32));
    }
  }
  return samples;
};

export const movePointToPreview = (point: MovePathPoint): PreviewPoint => ({
  x: clampUnit(point.x) * 100,
  y: (1 - clampUnit(point.y)) * 100,
});

export const normalizeMovePathPoints = (
  points: readonly MovePathPoint[],
  closed: boolean,
): MovePathPoint[] => {
  const normalized: MovePathPoint[] = [];
  for (const point of points) {
    const previous = normalized.at(-1);
    if (previous && Math.hypot(point.x - previous.x, point.y - previous.y) <= 1e-6) continue;
    normalized.push({ x: point.x, y: point.y });
  }
  if (
    closed &&
    normalized.length > 1 &&
    Math.hypot(
      normalized[0].x - normalized[normalized.length - 1].x,
      normalized[0].y - normalized[normalized.length - 1].y,
    ) <= 1e-6
  ) {
    normalized.pop();
  }
  return normalized;
};

type MovePreviewTransform = Pick<
  MoveEffectRequest,
  "coordinate_mode" | "center_x" | "center_y" | "size_x" | "size_y" | "rotation_degrees"
>;

/**
 * Mirrors the engine's scale/rotation/coordinate-mode transform and final
 * output clamp. Relative previews default to the neutral 50% fixture base;
 * callers with a known live base can supply it explicitly.
 */
export const transformMovePreview = (
  points: PreviewPoint[],
  move: MovePreviewTransform,
  relativeBase: MovePathPoint = { x: 0.5, y: 0.5 },
): PreviewPoint[] => {
  const authoredCenterX = clampUnit(move.center_x);
  const authoredCenterY = clampUnit(move.center_y);
  const anchorX = move.coordinate_mode === "Relative"
    ? clampUnit(relativeBase.x) + authoredCenterX - 0.5
    : authoredCenterX;
  const anchorY = move.coordinate_mode === "Relative"
    ? clampUnit(relativeBase.y) + authoredCenterY - 0.5
    : authoredCenterY;
  const center = { x: anchorX * 100, y: (1 - anchorY) * 100 };
  const scaleX = clamp(move.size_x, 0.01, 2);
  const scaleY = clamp(move.size_y, 0.01, 2);
  const radians = (Number.isFinite(move.rotation_degrees) ? move.rotation_degrees : 0) * Math.PI / 180;
  const cosine = Math.cos(radians);
  const sine = Math.sin(radians);
  return points.map((point) => {
    const scaledX = (point.x - 50) * scaleX;
    const scaledY = (point.y - 50) * scaleY;
    return {
      x: clamp(center.x + scaledX * cosine - scaledY * sine, 0, 100),
      y: clamp(center.y + scaledX * sine + scaledY * cosine, 0, 100),
    };
  });
};

export const buildPointPath = (points: PreviewPoint[], closed: boolean): string => {
  if (points.length === 0) return "";
  const segments = points.slice(1).map((point) => `L ${pathNumber(point.x)} ${pathNumber(point.y)}`);
  return [`M ${pathNumber(points[0].x)} ${pathNumber(points[0].y)}`, ...segments, closed ? "Z" : ""]
    .filter(Boolean)
    .join(" ");
};

const catmullRom = (p0: number, p1: number, p2: number, p3: number, amount: number) => {
  const squared = amount * amount;
  const cubed = squared * amount;
  return 0.5 * (
    2 * p1 +
    (-p0 + p2) * amount +
    (2 * p0 - 5 * p1 + 4 * p2 - p3) * squared +
    (-p0 + 3 * p1 - 3 * p2 + p3) * cubed
  );
};

export const sampleValueEnvelope = (
  points: ValueEffectPoint[],
  interpolation: ValueEffectInterpolation,
  progress: number,
): number => {
  if (points.length === 0) return 0;
  const normalized = clampUnit(progress);
  const first = points[0];
  if (normalized <= first.position) return clampUnit(first.value);
  const last = points[points.length - 1];
  if (normalized >= last.position) return clampUnit(last.value);
  let index = 0;
  for (let candidate = 0; candidate < points.length - 1; candidate += 1) {
    if (normalized >= points[candidate].position && normalized < points[candidate + 1].position) {
      index = candidate;
      break;
    }
  }
  const left = points[index];
  const right = points[index + 1];
  const local = clampUnit((normalized - left.position) / Math.max(0.000_001, right.position - left.position));
  if (interpolation === "Step") return clampUnit(left.value);
  if (interpolation === "Line") return clampUnit(left.value + (right.value - left.value) * local);
  const previous = index === 0 ? left.value : points[index - 1].value;
  const next = index + 2 < points.length ? points[index + 2].value : right.value;
  return clampUnit(catmullRom(previous, left.value, right.value, next, local));
};

export const buildValuePreviewPath = (
  points: ValueEffectPoint[],
  interpolation: ValueEffectInterpolation,
  direction: ValueEffectDirection,
  phase: number,
  width = 100,
  height = 32,
  samples = 64,
): string => {
  if (points.length === 0) return "";
  const count = Math.max(2, Math.round(samples));
  const segments: string[] = [];
  for (let index = 0; index <= count; index += 1) {
    const readProgress = index / count;
    let progress = normalizePhase(readProgress + phase);
    if (direction === "Reverse") progress = 1 - progress;
    else if (direction === "Bounce") {
      const doubled = progress * 2;
      progress = doubled <= 1 ? doubled : 2 - doubled;
    }
    const value = sampleValueEnvelope(points, interpolation, progress);
    segments.push(`${index === 0 ? "M" : "L"} ${pathNumber(readProgress * width)} ${pathNumber((1 - value) * height)}`);
  }
  return segments.join(" ");
};
