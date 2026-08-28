// Fixture pan/tilt/dimmer limit + axis-mapping helpers extracted from App.tsx.
// Pure functions over FixtureLimits/PatchedFixtureSummary; no SolidJS/state deps.
import { clampDmxValue, clampRange } from "./numericHelpers";
import type { FixtureLimits, PatchedFixtureSummary } from "./types";

export type FixtureMovementAxis = "pan" | "tilt";

export interface FixturePhysicalAxisRange {
  from: number;
  to: number;
}

export const fixturePhysicalDegreesUnavailable = "Unavailable";

const dmxMaximum = 65_535;

const finitePhysicalRange = (from: number | null | undefined, to: number | null | undefined) =>
  Number.isFinite(from) && Number.isFinite(to) && from !== to
    ? { from: Number(from), to: Number(to) }
    : null;

const physicalValuesFollowLinearAxis = (
  segments: Array<{
    dmxFrom: number;
    dmxTo: number;
    physicalFrom: number;
    physicalTo: number;
  }>,
  range: FixturePhysicalAxisRange,
) => {
  const slope = (range.to - range.from) / dmxMaximum;
  return segments.every((segment) => {
    const expectedFrom = range.from + segment.dmxFrom * slope;
    const expectedTo = range.from + segment.dmxTo * slope;
    const tolerance = (expected: number) => Math.max(1e-4, Math.abs(expected) * 1e-5);
    return (
      Math.abs(segment.physicalFrom - expectedFrom) <= tolerance(expectedFrom)
      && Math.abs(segment.physicalTo - expectedTo) <= tolerance(expectedTo)
    );
  });
};

/**
 * Resolve the profile-authored physical degree range for one movement axis.
 *
 * A range is only authoritative when the matching Pan/Tilt control has
 * finite physical values for every DMX function and those functions form one
 * coherent 0..65535 linear axis. Partial or contradictory profile data stays
 * unavailable instead of being filled with a conventional 540/270 default.
 */
export const fixturePhysicalAxisRange = (
  fixture: Pick<PatchedFixtureSummary, "controls">,
  axis: FixtureMovementAxis,
): FixturePhysicalAxisRange | null => {
  const controls = Array.isArray(fixture.controls) ? fixture.controls : [];
  const control = controls.find(
    (candidate) => typeof candidate.attribute === "string" && candidate.attribute.trim().toLowerCase() === axis,
  );
  const functions = Array.isArray(control?.functions) ? control.functions : [];
  if (!control || functions.length === 0) {
    return null;
  }

  const orderedFunctions = [...functions].sort((first, second) =>
    first.dmx_from - second.dmx_from || first.dmx_to - second.dmx_to,
  );
  const segments: Array<{
    dmxFrom: number;
    dmxTo: number;
    physicalFrom: number;
    physicalTo: number;
  }> = [];
  let expectedDmxFrom = 0;
  for (const fn of orderedFunctions) {
    const physical = finitePhysicalRange(fn.physical_from, fn.physical_to);
    if (
      !physical
      || !Number.isInteger(fn.dmx_from)
      || !Number.isInteger(fn.dmx_to)
      || fn.dmx_from < 0
      || fn.dmx_to > dmxMaximum
      || fn.dmx_to < fn.dmx_from
      || fn.dmx_from !== expectedDmxFrom
      || typeof fn.attribute !== "string"
      || fn.attribute.trim().toLowerCase() !== axis
    ) {
      return null;
    }
    segments.push({
      dmxFrom: fn.dmx_from,
      dmxTo: fn.dmx_to,
      physicalFrom: physical.from,
      physicalTo: physical.to,
    });
    expectedDmxFrom = fn.dmx_to + 1;
  }

  if (expectedDmxFrom !== dmxMaximum + 1) {
    return null;
  }
  const first = segments[0];
  const last = segments[segments.length - 1];
  const range = { from: first.physicalFrom, to: last.physicalTo };
  return physicalValuesFollowLinearAxis(segments, range) ? range : null;
};

export const fixturePhysicalDegreesForDmxValue = (
  value: number,
  range: FixturePhysicalAxisRange | null,
) => {
  if (!range || !Number.isFinite(value) || !finitePhysicalRange(range.from, range.to)) {
    return null;
  }
  const normalized = clampDmxValue(value) / dmxMaximum;
  return range.from + normalized * (range.to - range.from);
};

export const formatFixturePhysicalDegrees = (value: number | null) => {
  if (value === null || !Number.isFinite(value)) {
    return fixturePhysicalDegreesUnavailable;
  }
  const rounded = Math.round(value * 10) / 10;
  const stable = Object.is(rounded, -0) ? 0 : rounded;
  return `${Number.isInteger(stable) ? stable : stable.toFixed(1)}°`;
};

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
