import type { FixtureLimits } from "./types";

export type MovementLimitField = "pan_min" | "pan_max" | "tilt_min" | "tilt_max";

type MovementLimitKeyboardOptions = {
  limits: FixtureLimits;
  baseAmount: number;
  onUpdate: (field: MovementLimitField, value: number) => void;
  onReset: () => void;
};

const clampDmxLimit = (value: number) =>
  Math.min(65_535, Math.max(0, Math.round(Number.isFinite(value) ? value : 0)));

const keyboardStep = (event: KeyboardEvent, baseAmount: number) => {
  const base = Math.max(1, Math.round(baseAmount || 1));
  if (event.shiftKey) {
    return base * 4;
  }
  if (event.altKey) {
    return Math.max(1, Math.round(base / 4));
  }
  return base;
};

const moveRange = (minimum: number, maximum: number, delta: number) => {
  const normalizedMinimum = Math.min(clampDmxLimit(minimum), clampDmxLimit(maximum));
  const normalizedMaximum = Math.max(clampDmxLimit(minimum), clampDmxLimit(maximum));
  const width = normalizedMaximum - normalizedMinimum;
  const nextMinimum = Math.min(65_535 - width, Math.max(0, normalizedMinimum + delta));
  return { minimum: nextMinimum, maximum: nextMinimum + width };
};

const resizeRange = (minimum: number, maximum: number, delta: number) => {
  const normalizedMinimum = Math.min(clampDmxLimit(minimum), clampDmxLimit(maximum));
  const normalizedMaximum = Math.max(clampDmxLimit(minimum), clampDmxLimit(maximum));
  const center = (normalizedMinimum + normalizedMaximum) / 2;
  const halfWidth = Math.max(0, (normalizedMaximum - normalizedMinimum) / 2 + delta);
  return {
    minimum: clampDmxLimit(center - halfWidth),
    maximum: clampDmxLimit(center + halfWidth),
  };
};

export function handleMovementLimitKey(
  event: KeyboardEvent,
  options: MovementLimitKeyboardOptions,
) {
  const { limits, onReset, onUpdate } = options;
  const step = keyboardStep(event, options.baseAmount);
  if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
    event.preventDefault();
    const pan = moveRange(
      limits.pan_min,
      limits.pan_max,
      event.key === "ArrowLeft" ? -step : step,
    );
    onUpdate("pan_min", pan.minimum);
    onUpdate("pan_max", pan.maximum);
    return;
  }
  if (event.key === "ArrowUp" || event.key === "ArrowDown") {
    event.preventDefault();
    const tilt = moveRange(
      limits.tilt_min,
      limits.tilt_max,
      event.key === "ArrowUp" ? step : -step,
    );
    onUpdate("tilt_min", tilt.minimum);
    onUpdate("tilt_max", tilt.maximum);
    return;
  }
  if (event.key === "PageUp" || event.key === "=" || event.key === "+") {
    event.preventDefault();
    const pan = resizeRange(limits.pan_min, limits.pan_max, step);
    const tilt = resizeRange(limits.tilt_min, limits.tilt_max, step);
    onUpdate("pan_min", pan.minimum);
    onUpdate("pan_max", pan.maximum);
    onUpdate("tilt_min", tilt.minimum);
    onUpdate("tilt_max", tilt.maximum);
    return;
  }
  if (event.key === "PageDown" || event.key === "-") {
    event.preventDefault();
    const pan = resizeRange(limits.pan_min, limits.pan_max, -step);
    const tilt = resizeRange(limits.tilt_min, limits.tilt_max, -step);
    onUpdate("pan_min", pan.minimum);
    onUpdate("pan_max", pan.maximum);
    onUpdate("tilt_min", tilt.minimum);
    onUpdate("tilt_max", tilt.maximum);
    return;
  }
  if (event.key === "Home") {
    event.preventDefault();
    onReset();
  }
}
