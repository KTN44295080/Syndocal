// Pure numeric, DMX-value, color, and geometry-matrix helpers extracted from App.tsx.
// These have no SolidJS or component-state dependencies; keep them side-effect free.
import type { GeometrySummary } from "./types";

export const clamp01 = (value: number) => Math.min(1, Math.max(0, value));

export const valueToHexByte = (value: number) => (value >> 8).toString(16).padStart(2, "0");

export const finiteOr = (value: number, fallback: number) => (Number.isFinite(value) ? value : fallback);
export const clampRange = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value));
export const clampDmxValue = (value: number) => Math.round(clampRange(Number.isFinite(value) ? value : 0, 0, 65_535));
export const formatDmxPercent = (value: number) => `${Math.round((clampDmxValue(value) / 65_535) * 1000) / 10}%`;
export const dmxValueToPercent = (value: number) => Math.round((clampDmxValue(value) / 65_535) * 1000) / 10;
export const percentToDmxValue = (value: number) => clampDmxValue((clampRange(value, 0, 100) / 100) * 65_535);
export const formatShortDmxPercent = (value: number) => `${Math.round(dmxValueToPercent(value))}%`;

export const geometryIdentityMatrix = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
];
export const normalizedGeometryMatrix = (geometry: GeometrySummary) =>
  geometry.matrix.length === 16
    ? geometry.matrix.map((value, index) => finiteOr(value, geometryIdentityMatrix[index]))
    : [...geometryIdentityMatrix];
export const multiplyGeometryMatrix = (left: readonly number[], right: readonly number[]) => {
  const result = Array.from({ length: 16 }, () => 0);
  for (let row = 0; row < 4; row += 1) {
    for (let column = 0; column < 4; column += 1) {
      result[row * 4 + column] = [0, 1, 2, 3].reduce(
        (sum, index) => sum + left[row * 4 + index] * right[index * 4 + column],
        0,
      );
    }
  }
  return result;
};
export const cumulativeGeometryMatrix = (geometry: GeometrySummary, geometryByName: Map<string, GeometrySummary>) => {
  const lineage: number[][] = [];
  const seen = new Set<string>();
  let current: GeometrySummary | undefined = geometry;
  while (current && !seen.has(current.name) && lineage.length < 32) {
    seen.add(current.name);
    lineage.push(normalizedGeometryMatrix(current));
    const parentName: string | undefined = typeof current.parent === "string" ? current.parent.trim() : undefined;
    current = parentName ? geometryByName.get(parentName) : undefined;
  }
  return lineage.reverse().reduce(
    (accumulated, matrix) => multiplyGeometryMatrix(accumulated, matrix),
    [...geometryIdentityMatrix],
  );
};
export const geometryMatrixTranslation = (matrix: readonly number[]) => ({
  x: finiteOr(matrix[3] ?? 0, 0),
  y: finiteOr(matrix[7] ?? 0, 0),
  z: finiteOr(matrix[11] ?? 0, 0),
});
export const rotateStageOffsetYaw = (offset: { x: number; z: number }, yawDeg: number) => {
  const angle = (yawDeg * Math.PI) / 180;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  return {
    x: offset.x * cos - offset.z * sin,
    z: offset.x * sin + offset.z * cos,
  };
};

export const hsvToRgb = (hue: number, saturation: number, value: number) => {
  const h = ((hue % 360) + 360) % 360;
  const s = clamp01(saturation);
  const v = clamp01(value);
  const c = v * s;
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1));
  const m = v - c;
  let red = 0;
  let green = 0;
  let blue = 0;
  if (h < 60) {
    red = c;
    green = x;
  } else if (h < 120) {
    red = x;
    green = c;
  } else if (h < 180) {
    green = c;
    blue = x;
  } else if (h < 240) {
    green = x;
    blue = c;
  } else if (h < 300) {
    red = x;
    blue = c;
  } else {
    red = c;
    blue = x;
  }
  return {
    red: Math.round((red + m) * 255),
    green: Math.round((green + m) * 255),
    blue: Math.round((blue + m) * 255),
  };
};

export const rgbToHsv = (red: number, green: number, blue: number) => {
  const r = clamp01(red / 255);
  const g = clamp01(green / 255);
  const b = clamp01(blue / 255);
  const max = Math.max(r, g, b);
  const min = Math.min(r, g, b);
  const delta = max - min;
  let hue = 0;
  if (delta !== 0) {
    if (max === r) {
      hue = 60 * (((g - b) / delta) % 6);
    } else if (max === g) {
      hue = 60 * ((b - r) / delta + 2);
    } else {
      hue = 60 * ((r - g) / delta + 4);
    }
  }
  if (hue < 0) {
    hue += 360;
  }
  return {
    hue,
    saturation: max === 0 ? 0 : delta / max,
    value: max,
  };
};

export const rgbToHex = (red: number, green: number, blue: number) =>
  `#${[red, green, blue].map((value) => Math.round(value).toString(16).padStart(2, "0")).join("")}`;

export const defaultColorFavorites = () => ["#ff0000", "#00ff00", "#0000ff", "#ffffff"];

export const normalizeHexColor = (value: unknown) => {
  if (typeof value !== "string") {
    return null;
  }
  const normalized = value.trim().toLowerCase();
  return /^#[0-9a-f]{6}$/.test(normalized) ? normalized : null;
};
