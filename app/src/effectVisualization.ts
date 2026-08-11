import type {
  ColorEffectColor,
  ColorEffectInterpolation,
  ColorEffectStop,
  CurveEffectPoint,
  DaslightCustomCurveSource,
  DaslightCurveSource,
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
const STROBE_PULSES_PER_PERIOD = 10;
const STROBE_DUTY_CYCLE = 0.2;

/** Mirrors engine::evaluate_lfo_shape, including the deterministic noise hash. */
export const evaluateLfoShape = (shape: LfoShape, phase: number): number => {
  const normalized = normalizePhase(phase);
  switch (shape) {
    case "Sine":
      return (Math.sin(normalized * Math.PI * 2) + 1) * 0.5;
    case "Cosine":
      return (Math.cos(normalized * Math.PI * 2) + 1) * 0.5;
    case "Pulse":
      return Math.sin(normalized * Math.PI) ** 2;
    case "Triangle":
      return normalized < 0.5 ? normalized * 2 : (1 - normalized) * 2;
    case "Ramp":
    case "Saw":
      return normalized;
    case "Square":
      return normalized < 0.5 ? 1 : 0;
    case "Strobe":
      return (normalized * STROBE_PULSES_PER_PERIOD) % 1 < STROBE_DUTY_CYCLE ? 1 : 0;
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
    case "Sinus3": {
      const carrier = Math.sin(normalized * Math.PI * 2);
      return (carrier ** 3 + 1) * 0.5;
    }
    case "Tangeant":
      return clampUnit(Math.tan(normalized * Math.PI * 2) * 0.5 + 0.5);
    // Import-only; it is evaluated through `evaluateDaslightCustomCurveSource`.
    case "DaslightCustom":
      return 0;
  }
};

/** Mirrors the versioned Daslight Curve source branch in engine. */
export const evaluateDaslightCurveSource = (
  shape: LfoShape,
  source: DaslightCurveSource,
  phase: number,
  progress: number,
  periodMs: number,
): number => {
  const position = normalizePhase(progress);
  let value: number;
  switch (shape) {
    case "Sine":
      value = Math.sin(Math.PI * 2 * (source.rate * 0.5 * position - phase)) * source.size * 0.5
        + source.offset + source.size * 0.5;
      break;
    case "Sinus3": {
      const carrier = Math.sin(Math.PI * 2 * (source.rate * 0.5 * position - phase));
      value = carrier ** 3 * source.size * 0.5 + source.offset + source.size * 0.5;
      break;
    }
    case "Tangeant":
      value = Math.tan(Math.PI * 2 * (source.rate * 0.5 * position - phase))
        * source.size * 0.5 + source.offset + source.size * 0.5;
      break;
    case "Pulse": {
      const rateAngle = Math.fround(source.rate * 2 * Math.PI * 2);
      const phaseAngle = Math.fround(-Math.fround(phase) * Math.PI * 2);
      const angle = Math.fround(Math.fround(rateAngle * Math.fround(position)) + phaseAngle);
      const window = 1 - Math.abs(position * 2 - 1);
      value = Math.sin(angle) * Math.fround(source.size * window) + source.offset + 0.5;
      break;
    }
    case "Saw": {
      const sourcePhase = source.rate * 0.5 * position - phase;
      const centered = sourcePhase - Math.floor(sourcePhase + 0.5);
      value = source.offset - centered * source.size + source.size - 0.5;
      break;
    }
    case "Ramp": {
      const sourcePhase = source.rate * 0.5 * position - phase;
      const centered = sourcePhase - Math.floor(sourcePhase + 0.5);
      value = source.offset + centered * source.size + source.size - 0.5;
      break;
    }
    case "Triangle": {
      const sourcePhase = source.rate * 0.5 * position - phase;
      value = source.offset + evaluateLfoShape("Triangle", sourcePhase + 0.75) * source.size;
      break;
    }
    case "Random": {
      const sourcePhase = position * source.rate * Math.PI - phase * Math.PI * 2;
      const step = Math.floor(Math.abs(sourcePhase)) >>> 0;
      value = source.size * (correctedCurveRandomBucket(source.rng_seed ?? 0, step) + source.offset);
      break;
    }
    case "Square": {
      const shifted = normalizePhase(position - phase);
      const band = Math.floor(shifted * source.rate);
      value = source.offset + (band % 2 === 0 ? source.size : 0);
      break;
    }
    case "Strobe": {
      const elapsedSeconds = position * Math.max(10, periodMs) / 1000;
      const cyclePosition = normalizePhase(elapsedSeconds * source.rate);
      const duty = Math.min(1, Math.max(STROBE_DUTY_CYCLE, phase * 0.5));
      value = cyclePosition < duty
        ? source.offset + source.size * 0.5
        : source.offset;
      break;
    }
    default:
      value = evaluateLfoShape(shape, position + phase);
  }
  return clampUnit(value);
};

export type DaslightCustomCurveEasing = "Linear" | "InCubic" | "OutCubic" | "InOutCubic" | "OutInCubic";

/** Mirrors the Custom CURVE `raw_y` value decoder recovered from Daslight. */
export const decodeDaslightCustomRawY = (rawY: number): { value: number; easing: DaslightCustomCurveEasing } => {
  const finite = Number.isFinite(rawY) ? rawY : 0;
  const floor = Math.floor(finite);
  const fractional = finite - floor;
  const value = fractional === 0 ? (Math.abs(floor) % 10 === 0 ? 0 : 1) : fractional;
  const easingCode = Math.trunc(finite / 10);
  const easing: DaslightCustomCurveEasing = easingCode === 1
    ? "InCubic"
    : easingCode === 2
      ? "OutCubic"
      : easingCode === 3
        ? "InOutCubic"
        : easingCode === 4
          ? "OutInCubic"
          : "Linear";
  return { value, easing };
};

export const easeDaslightCustomCurve = (easing: DaslightCustomCurveEasing, progress: number): number => {
  const u = clampUnit(progress);
  switch (easing) {
    case "InCubic": return u ** 3;
    case "OutCubic": return 1 - (1 - u) ** 3;
    case "InOutCubic": return u < 0.5 ? 4 * u ** 3 : 1 - (-2 * u + 2) ** 3 / 2;
    case "OutInCubic": return u < 0.5 ? (1 - (1 - 2 * u) ** 3) / 2 : ((2 * u - 1) ** 3 + 1) / 2;
    default: return u;
  }
};

/**
 * Mirrors Custom CURVE's first-in-source-order matching segment. This is a
 * defensive rendering path for legacy/malformed snapshots; the importer and
 * engine only accept strict, increasing Daslight point order. The
 * destination/right point supplies the easing family.
 */
export const evaluateDaslightCustomCurveSource = (
  source: DaslightCustomCurveSource,
  phase: number,
): number => {
  const points = source.points;
  if (points.length === 0) return 0;
  const position = normalizePhase(phase);
  for (let index = 0; index + 1 < points.length; index += 1) {
    const left = points[index];
    const right = points[index + 1];
    if (left.x <= position && position < right.x) {
      const leftValue = decodeDaslightCustomRawY(left.raw_y).value;
      const rightDecoded = decodeDaslightCustomRawY(right.raw_y);
      const progress = (position - left.x) / (right.x - left.x);
      return leftValue + (rightDecoded.value - leftValue)
        * easeDaslightCustomCurve(rightDecoded.easing, progress);
    }
  }
  return decodeDaslightCustomRawY(points[points.length - 1].raw_y).value;
};

export const buildDaslightCustomCurvePreviewPath = (
  source: DaslightCustomCurveSource,
  phase = 0,
  width = 100,
  height = 32,
  samples = 96,
): string => {
  const count = Math.max(2, Math.round(samples));
  const segments: string[] = [];
  for (let index = 0; index <= count; index += 1) {
    const progress = index / count;
    const value = evaluateDaslightCustomCurveSource(source, progress + phase);
    segments.push(`${index === 0 ? "M" : "L"} ${pathNumber(progress * width)} ${pathNumber((1 - value) * height)}`);
  }
  return segments.join(" ");
};

const hashUnitFloat = (seed: number): number => {
  return hashU32(seed) / 0xffff_ffff;
};

const hashU32 = (seed: number): number => {
  let value = (seed + 0x9e37_79b9) >>> 0;
  value = (value ^ (value >>> 16)) >>> 0;
  value = Math.imul(value, 0x7feb_352d) >>> 0;
  value = (value ^ (value >>> 15)) >>> 0;
  value = Math.imul(value, 0x846c_a68b) >>> 0;
  value = (value ^ (value >>> 16)) >>> 0;
  return value;
};

const correctedCurveRandomBucket = (seed: number, step: number): number =>
  hashU32((seed ^ Math.imul(step, 0x9e37_79b9)) >>> 0) % 100 / 100;

export const buildLfoPreviewPath = (
  shape: LfoShape,
  phase: number,
  width = 100,
  height = 32,
  samples = 64,
  low = 0,
  high = 65_535,
  directed = true,
  daslightCurve?: DaslightCurveSource | null,
  periodMs = 1_000,
): string => {
  const count = Math.max(2, Math.round(samples));
  const first = clamp(low, 0, 65_535);
  const second = clamp(high, 0, 65_535);
  const from = directed ? first : Math.min(first, second);
  const to = directed ? second : Math.max(first, second);
  const segments: string[] = [];
  for (let index = 0; index <= count; index += 1) {
    const progress = index / count;
    const normalized = daslightCurve
      ? evaluateDaslightCurveSource(
          shape,
          daslightCurve,
          phase,
          progress,
          periodMs,
        )
      : evaluateLfoShape(shape, progress + phase);
    const output = daslightCurve ? normalized * 65_535 : from + (to - from) * normalized;
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

interface PreviewCircularArc {
  centerX: number;
  centerY: number;
  radius: number;
  startAngle: number;
  sweepAngle: number;
}

type PreviewCircleSegment =
  | { kind: "Arc"; arc: PreviewCircularArc }
  | { kind: "Inflection"; firstHalf: PreviewCircularArc; midpoint: PreviewPoint }
  | { kind: "Linear"; from: PreviewPoint; to: PreviewPoint };

const signedCircumradius = (first: PreviewPoint, second: PreviewPoint, third: PreviewPoint) => {
  const determinant = 2 * (
    first.x * (second.y - third.y) +
    second.x * (third.y - first.y) +
    third.x * (first.y - second.y)
  );
  if (determinant === 0 || !Number.isFinite(determinant)) return 0;
  const firstSquared = first.x * first.x + first.y * first.y;
  const secondSquared = second.x * second.x + second.y * second.y;
  const thirdSquared = third.x * third.x + third.y * third.y;
  const centerX = (
    firstSquared * (second.y - third.y) +
    secondSquared * (third.y - first.y) +
    thirdSquared * (first.y - second.y)
  ) / determinant;
  const centerY = (
    firstSquared * (third.x - second.x) +
    secondSquared * (first.x - third.x) +
    thirdSquared * (second.x - first.x)
  ) / determinant;
  const radius = Math.hypot(first.x - centerX, first.y - centerY);
  return Number.isFinite(radius) ? Math.sign(determinant) * radius : 0;
};

const compilePreviewCircularArc = (
  from: PreviewPoint,
  to: PreviewPoint,
  signedRadius: number,
): PreviewCircularArc | null => {
  if (signedRadius === 0 || !Number.isFinite(signedRadius)) return null;
  const chordX = to.x - from.x;
  const chordY = to.y - from.y;
  const chordLength = Math.hypot(chordX, chordY);
  if (chordLength === 0) return null;
  const radius = Math.abs(signedRadius);
  const heightSquared = radius * radius - chordLength * chordLength * 0.25;
  if (heightSquared < 0) return null;
  const midpointX = (from.x + to.x) * 0.5;
  const midpointY = (from.y + to.y) * 0.5;
  const height = Math.sqrt(heightSquared);
  const side = Math.sign(signedRadius);
  const centerX = midpointX - side * chordY / chordLength * height;
  const centerY = midpointY + side * chordX / chordLength * height;
  const startAngle = Math.atan2(from.y - centerY, from.x - centerX);
  const endAngle = Math.atan2(to.y - centerY, to.x - centerX);
  let sweepAngle = endAngle - startAngle;
  if (signedRadius > 0 && sweepAngle < 0) sweepAngle += Math.PI * 2;
  else if (signedRadius < 0 && sweepAngle > 0) sweepAngle -= Math.PI * 2;
  return { centerX, centerY, radius, startAngle, sweepAngle };
};

const samplePreviewCircularArcRaw = (arc: PreviewCircularArc, progress: number): PreviewPoint => {
  const angle = arc.startAngle + arc.sweepAngle * clampUnit(progress);
  return {
    x: arc.centerX + Math.cos(angle) * arc.radius,
    y: arc.centerY + Math.sin(angle) * arc.radius,
  };
};

const samplePreviewCircularArc = (arc: PreviewCircularArc, progress: number): PreviewPoint => {
  const point = samplePreviewCircularArcRaw(arc, progress);
  return { x: clamp(point.x, 0, 100), y: clamp(point.y, 0, 100) };
};

const compilePreviewCircleSegment = (
  points: PreviewPoint[],
  index: number,
): PreviewCircleSegment => {
  const previous = points[(index + points.length - 1) % points.length];
  const from = points[index];
  const to = points[(index + 1) % points.length];
  const after = points[(index + 2) % points.length];
  let incomingRadius = signedCircumradius(previous, from, to);
  let outgoingRadius = signedCircumradius(from, to, after);
  if (outgoingRadius === 0) outgoingRadius = incomingRadius;
  if (incomingRadius === 0) incomingRadius = outgoingRadius;
  const averageRadius = (Math.abs(incomingRadius) + Math.abs(outgoingRadius)) * 0.5;
  const signedAverage = Math.sign(incomingRadius) * averageRadius;
  const oppositeSigns = incomingRadius !== 0 && outgoingRadius !== 0 &&
    Math.sign(incomingRadius) !== Math.sign(outgoingRadius);
  if (oppositeSigns) {
    const midpoint = lerpPoint(from, to, 0.5);
    const firstHalf = compilePreviewCircularArc(from, midpoint, signedAverage * 0.5);
    return firstHalf ? { kind: "Inflection", firstHalf, midpoint } : { kind: "Linear", from, to };
  }
  const arc = compilePreviewCircularArc(from, to, signedAverage);
  return arc ? { kind: "Arc", arc } : { kind: "Linear", from, to };
};

const samplePreviewCircleSegment = (segment: PreviewCircleSegment, progress: number): PreviewPoint => {
  if (segment.kind === "Arc") return samplePreviewCircularArc(segment.arc, progress);
  if (segment.kind === "Linear") return lerpPoint(segment.from, segment.to, clampUnit(progress));
  if (progress <= 0.5) return samplePreviewCircularArc(segment.firstHalf, progress * 2);
  const reflected = samplePreviewCircularArcRaw(segment.firstHalf, (1 - progress) * 2);
  return {
    x: clamp(2 * segment.midpoint.x - reflected.x, 0, 100),
    y: clamp(2 * segment.midpoint.y - reflected.y, 0, 100),
  };
};

const sampleCirclePath = (points: PreviewPoint[]): PreviewPoint[] => {
  const samples: PreviewPoint[] = [points[0]];
  if (points.length === 2) {
    const center = lerpPoint(points[0], points[1], 0.5);
    const radius = pointDistance(points[0], center);
    const startAngle = Math.atan2(points[0].y - center.y, points[0].x - center.x);
    for (let sample = 1; sample <= 64; sample += 1) {
      const angle = startAngle + Math.PI * 2 * sample / 64;
      samples.push({
        x: clamp(center.x + Math.cos(angle) * radius, 0, 100),
        y: clamp(center.y + Math.sin(angle) * radius, 0, 100),
      });
    }
    return samples;
  }
  const segments = points.map((_, index) => compilePreviewCircleSegment(points, index));
  for (const segment of segments) {
    for (let sample = 1; sample <= 32; sample += 1) {
      samples.push(samplePreviewCircleSegment(segment, sample / 32));
    }
  }
  return samples;
};

export const sampleMovePath = (
  points: PreviewPoint[],
  interpolation: MoveInterpolation,
  closed: boolean,
): PreviewPoint[] => {
  if (points.length < 2 || interpolation === "Line" || interpolation === "DaslightPoints") return [...points];
  if (interpolation === "Circle" || interpolation === "DaslightCircle") return sampleCirclePath(points);
  if (interpolation === "DaslightLine") return [points[0], points[1], points[0]];
  if (interpolation === "DaslightPolygon") return [...points, points[0]];
  if (interpolation === "DaslightCurve") {
    const samples: PreviewPoint[] = [points[0]];
    for (let segment = 0; segment < points.length - 1; segment += 1) {
      const p0 = segment === 0 ? points[segment] : points[segment - 1];
      const p1 = points[segment];
      const p2 = points[segment + 1];
      const p3 = points[segment + 2] ?? p2;
      for (let slice = 1; slice <= 16; slice += 1) {
        const t = slice / 16;
        const squared = t * t;
        const cubed = squared * t;
        const h00 = 2 * cubed - 3 * squared + 1;
        const h10 = cubed - 2 * squared + t;
        const h01 = -2 * cubed + 3 * squared;
        const h11 = cubed - squared;
        samples.push({
          x: h00 * p1.x + h10 * 0.5 * (p2.x - p0.x) + h01 * p2.x + h11 * 0.5 * (p3.x - p1.x),
          y: h00 * p1.y + h10 * 0.5 * (p2.y - p0.y) + h01 * p2.y + h11 * 0.5 * (p3.y - p1.y),
        });
      }
    }
    return samples;
  }
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

type MoveFanoutPreview = Pick<
  MoveEffectRequest,
  "period_ms" | "direction" | "phase" | "fixture_spread" | "symmetry" | "interpolation" | "points"
>;

/** Mirrors the compiled Move target phase/reverse flags used by the runtime evaluator. */
export const moveFanoutPreviewState = (
  request: MoveFanoutPreview,
  elapsedMs: number,
  selectionRank: number,
  selectionCount: number,
): { progress: number; mirrorPan: boolean } => {
  const count = Math.max(1, Math.round(selectionCount));
  const rank = Math.max(0, Math.min(count - 1, Math.round(selectionRank)));
  const daslightExact = request.interpolation.startsWith("Daslight");
  if (daslightExact) {
    const rawCycle = Math.max(0, Number.isFinite(elapsedMs) ? elapsedMs : 0)
      / Math.max(10, request.period_ms) + request.phase;
    const phase = normalizePhase(rawCycle);
    const directed = normalizePhase(request.direction === "Reverse"
      ? 1 - phase
      : request.direction === "Bounce"
        ? phase * 2 <= 1 ? phase * 2 : 2 - phase * 2
        : phase);
    const split = Math.floor(count / 2);
    const reverseWing = request.symmetry === true && count >= 2 && rank >= split;
    const offsetRank = reverseWing ? count - 1 - rank : rank;
    const translated = reverseWing
      ? normalizePhase((request.interpolation === "DaslightPoints" ? 0 : 0.5) - directed)
      : directed;
    const targetPhase = normalizePhase(
      translated - offsetRank * clampUnit(request.fixture_spread),
    );
    let progress = targetPhase;
    if (request.interpolation === "DaslightCurve") {
      const doubled = progress * 2;
      progress = doubled <= 1 ? doubled : 2 - doubled;
    } else if (request.interpolation === "DaslightPoints") {
      const pointCount = request.points.length;
      const pointIndex = Math.min(
        Math.max(0, pointCount - 1),
        Math.floor(targetPhase * pointCount),
      );
      progress = pointCount <= 1 ? 0 : pointIndex / (pointCount - 1);
    }
    return { progress, mirrorPan: false };
  }
  const circle = request.interpolation === "Circle";
  const secondWing = request.symmetry === true && rank >= Math.ceil(count / 2);
  const fanoutRank = circle && secondWing ? rank - Math.ceil(count / 2) : rank;
  const offset = fanoutRank / count * clampUnit(request.fixture_spread) * (circle ? -1 : 1);
  const time = Math.max(0, Number.isFinite(elapsedMs) ? elapsedMs : 0)
    / Math.max(10, request.period_ms) + request.phase;
  const phase = normalizePhase(circle && secondWing ? 0.5 - time + offset : time + offset);
  const progress = request.direction === "Reverse"
    ? 1 - phase
    : request.direction === "Bounce"
      ? phase * 2 <= 1 ? phase * 2 : 2 - phase * 2
      : phase;
  return { progress, mirrorPan: !circle && secondWing };
};

export const movePointToPreview = (point: MovePathPoint): PreviewPoint => ({
  x: clampUnit(point.x) * 100,
  y: (1 - clampUnit(point.y)) * 100,
});

export const normalizeMovePathPoints = (
  points: readonly MovePathPoint[],
  closed: boolean,
  interpolation?: MoveInterpolation,
): MovePathPoint[] => {
  // Circle keeps every authored point as an equal-time segment boundary,
  // including degenerate adjacent/closing duplicates from source files.
  if (interpolation === "Circle" || interpolation?.startsWith("Daslight")) {
    return points.map((point) => ({ ...point }));
  }
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

/** Cubic Hermite sampling used by the independent Curve FX runtime. */
export const sampleCurveFunction = (points: CurveEffectPoint[], progress: number): number => {
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
  const span = Math.max(0.000_001, right.position - left.position);
  const amount = clampUnit((normalized - left.position) / span);
  const squared = amount * amount;
  const cubed = squared * amount;
  const h00 = 2 * cubed - 3 * squared + 1;
  const h10 = cubed - 2 * squared + amount;
  const h01 = -2 * cubed + 3 * squared;
  const h11 = cubed - squared;
  return clampUnit(
    h00 * left.value +
      h10 * span * left.out_tangent +
      h01 * right.value +
      h11 * span * right.in_tangent,
  );
};

export const buildCurvePreviewPath = (
  points: CurveEffectPoint[],
  direction: ValueEffectDirection,
  phase: number,
  width = 100,
  height = 32,
  samples = 96,
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
    const value = sampleCurveFunction(points, progress);
    segments.push(`${index === 0 ? "M" : "L"} ${pathNumber(readProgress * width)} ${pathNumber((1 - value) * height)}`);
  }
  return segments.join(" ");
};
