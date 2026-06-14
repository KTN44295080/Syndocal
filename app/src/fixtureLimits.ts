// Fixture pan/tilt/dimmer limit + axis-mapping helpers extracted from App.tsx.
// Pure functions over FixtureLimits/PatchedFixtureSummary; no SolidJS/state deps.
import { clampDmxValue, clampRange } from "./numericHelpers";
import type { FixtureLimits, PatchedFixtureSummary } from "./types";

export const defaultFixtureLimits: FixtureLimits = {
  dimmer_min: 0,
  dimmer_max: 65535,
  pan_min: 0,
  pan_max: 65535,
  tilt_min: 0,
  tilt_max: 65535,
  invert_pan: false,
  invert_tilt: false,
  swap_pan_tilt: false,
};

export const normalizeLimitRange = (min: number, max: number) => {
  const a = clampDmxValue(min);
  const b = clampDmxValue(max);
  return { min: Math.min(a, b), max: Math.max(a, b) };
};

export const applyAxisLimit = (value: number, min: number, max: number, invert: boolean) => {
  const range = normalizeLimitRange(min, max);
  const clamped = clampDmxValue(clampRange(value, range.min, range.max));
  return invert ? range.min + range.max - clamped : clamped;
};

export const sourceValueForAxisLimit = (desiredValue: number, min: number, max: number, invert: boolean) => {
  const range = normalizeLimitRange(min, max);
  const clamped = clampDmxValue(clampRange(desiredValue, range.min, range.max));
  return invert ? range.min + range.max - clamped : clamped;
};

export const dimmerValueWithinLimits = (fixture: PatchedFixtureSummary, value: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const range = normalizeLimitRange(limits.dimmer_min, limits.dimmer_max);
  return clampDmxValue(clampRange(value, range.min, range.max));
};

export const effectivePanTiltValues = (fixture: PatchedFixtureSummary, panValue: number, tiltValue: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const panSource = limits.swap_pan_tilt ? tiltValue : panValue;
  const tiltSource = limits.swap_pan_tilt ? panValue : tiltValue;
  return {
    pan: applyAxisLimit(panSource, limits.pan_min, limits.pan_max, limits.invert_pan),
    tilt: applyAxisLimit(tiltSource, limits.tilt_min, limits.tilt_max, limits.invert_tilt),
  };
};

export const sourcePanTiltValues = (fixture: PatchedFixtureSummary, panValue: number, tiltValue: number) => {
  const limits = fixture.limits ?? defaultFixtureLimits;
  const panSource = sourceValueForAxisLimit(panValue, limits.pan_min, limits.pan_max, limits.invert_pan);
  const tiltSource = sourceValueForAxisLimit(tiltValue, limits.tilt_min, limits.tilt_max, limits.invert_tilt);
  return limits.swap_pan_tilt
    ? { pan: tiltSource, tilt: panSource }
    : { pan: panSource, tilt: tiltSource };
};
