import type { AttributeControl, PatchedFixtureSummary } from "./types";

const dmxChannelCount = 512;

export interface DmxAddressRange {
  start: number;
  end: number;
}

export type DmxAddressTuple = [number, number];

export type DmxAllocatedFixture = Pick<PatchedFixtureSummary, "universe" | "address" | "controls">;

export const fixtureFootprintFromControls = (controls: Pick<AttributeControl, "offsets">[]) =>
  Math.max(0, ...controls.flatMap((control) => control.offsets));

export const fixtureFootprint = (fixture: Pick<PatchedFixtureSummary, "controls">) =>
  fixtureFootprintFromControls(fixture.controls);

export const addressRange = (start: number, footprint: number): DmxAddressTuple | null => {
  if (!Number.isFinite(start) || !Number.isFinite(footprint) || start < 1 || footprint < 1) {
    return null;
  }
  return [start, start + footprint - 1];
};

export const rangesOverlap = (first: DmxAddressTuple, second: DmxAddressTuple) =>
  first[0] <= second[1] && second[0] <= first[1];

export const buildOccupiedDmxRanges = (fixtures: DmxAllocatedFixture[]) => {
  const ranges = new Map<number, DmxAddressRange[]>();
  for (const fixture of fixtures) {
    const range = addressRange(fixture.address, fixtureFootprint(fixture));
    if (!range) {
      continue;
    }
    const universeRanges = ranges.get(fixture.universe) ?? [];
    universeRanges.push({ start: range[0], end: Math.min(dmxChannelCount, range[1]) });
    ranges.set(fixture.universe, universeRanges);
  }
  return ranges;
};

export const dmxAddressIsFree = (
  ranges: Map<number, DmxAddressRange[]>,
  universe: number,
  start: number,
  footprint: number,
) => {
  const range = addressRange(start, footprint);
  if (!range || range[1] > dmxChannelCount) {
    return false;
  }
  return !(ranges.get(universe) ?? []).some((occupied) => rangesOverlap(range, [occupied.start, occupied.end]));
};

export const reserveDmxAddressRange = (
  ranges: Map<number, DmxAddressRange[]>,
  universe: number,
  start: number,
  footprint: number,
) => {
  const range = addressRange(start, footprint);
  if (!range || range[1] > dmxChannelCount) {
    return false;
  }
  const universeRanges = ranges.get(universe) ?? [];
  universeRanges.push({ start: range[0], end: range[1] });
  ranges.set(universe, universeRanges);
  return true;
};

export const findFreeDmxAddress = (
  ranges: Map<number, DmxAddressRange[]>,
  universe: number,
  footprint: number,
  preferredStart = 1,
) => {
  const maxStart = dmxChannelCount - footprint + 1;
  if (maxStart < 1) {
    return null;
  }
  const startHint = Math.min(maxStart, Math.max(1, Math.floor(preferredStart || 1)));
  for (let candidate = startHint; candidate <= maxStart; candidate += 1) {
    if (dmxAddressIsFree(ranges, universe, candidate, footprint)) {
      return candidate;
    }
  }
  for (let candidate = 1; candidate < startHint; candidate += 1) {
    if (dmxAddressIsFree(ranges, universe, candidate, footprint)) {
      return candidate;
    }
  }
  return null;
};

export const canPlacePatchInOccupiedRanges = (
  occupiedRangesByUniverse: Map<number, DmxAddressRange[]>,
  startAddress: number,
  targetUniverse: number,
  footprint: number,
  count: number,
  stride: number,
) => {
  if (footprint <= 0 || count <= 0 || stride <= 0 || startAddress < 1) {
    return false;
  }
  const ranges = Array.from({ length: count }, (_, index) => addressRange(startAddress + index * stride, footprint));
  if (ranges.some((range) => !range || range[1] > dmxChannelCount)) {
    return false;
  }
  const occupiedRanges = occupiedRangesByUniverse.get(targetUniverse) ?? [];
  for (let index = 0; index < ranges.length; index += 1) {
    const range = ranges[index];
    if (!range) {
      return false;
    }
    for (let previousIndex = 0; previousIndex < index; previousIndex += 1) {
      const previousRange = ranges[previousIndex];
      if (previousRange && rangesOverlap(range, previousRange)) {
        return false;
      }
    }
    if (occupiedRanges.some((occupied) => rangesOverlap(range, [occupied.start, occupied.end]))) {
      return false;
    }
  }
  return true;
};

export const canPlacePatchAtAddress = (
  fixtures: DmxAllocatedFixture[],
  startAddress: number,
  targetUniverse: number,
  footprint: number,
  count: number,
  stride: number,
) => canPlacePatchInOccupiedRanges(
  buildOccupiedDmxRanges(fixtures),
  startAddress,
  targetUniverse,
  footprint,
  count,
  stride,
);

export const findNextFreePatchAddress = (
  fixtures: DmxAllocatedFixture[],
  targetUniverse: number,
  footprint: number,
  count: number,
  stride: number,
  preferredStart = 1,
) => {
  const occupiedRanges = buildOccupiedDmxRanges(fixtures);
  const start = Math.min(dmxChannelCount, Math.max(1, Math.floor(preferredStart || 1)));
  for (let candidate = start; candidate <= dmxChannelCount; candidate += 1) {
    if (canPlacePatchInOccupiedRanges(occupiedRanges, candidate, targetUniverse, footprint, count, stride)) {
      return candidate;
    }
  }
  for (let candidate = 1; candidate < start; candidate += 1) {
    if (canPlacePatchInOccupiedRanges(occupiedRanges, candidate, targetUniverse, footprint, count, stride)) {
      return candidate;
    }
  }
  return null;
};
