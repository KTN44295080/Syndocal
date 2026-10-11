import { addressRange, fixtureFootprint, rangesOverlap } from "./dmxAddressing";
import type { PatchedFixtureSummary } from "./types";

export function fixtureMovePlan(fixtures: PatchedFixtureSummary[], anchor: PatchedFixtureSummary,
  universe: number, address: number, existing: PatchedFixtureSummary[]) {
  if (!fixtures.length || fixtures.length > 256 || new Set(fixtures.map(f => f.id)).size !== fixtures.length
    || !fixtures.some(f => f.id === anchor.id)) throw new Error("Select from 1 to 256 distinct fixtures to move.");
  if (!Number.isInteger(universe) || universe < 0 || universe > 65535 || !Number.isInteger(address)) {
    throw new Error("The destination universe or address is invalid.");
  }
  const moving = new Set(fixtures.map(f => f.id));
  const changes = fixtures.map(fixture => {
    if (fixture.universe !== anchor.universe) throw new Error("Select fixtures from one source universe.");
    const footprint = fixtureFootprint(fixture), start = address + fixture.address - anchor.address;
    const range = addressRange(start, footprint);
    if (!range || range[1] > 512) throw new Error(`${fixture.label} would extend outside addresses 1–512.`);
    return { fixtureId: fixture.id, expectedUniverse: fixture.universe, expectedAddress: fixture.address,
      expectedFootprint: footprint, label: fixture.label, universe, address: start, range };
  });
  for (let index = 0; index < changes.length; index++) {
    const change = changes[index];
    for (const other of changes.slice(0, index)) {
      if (rangesOverlap(change.range, other.range)) throw new Error("Selected fixture footprints overlap.");
    }
    const conflict = existing.find(f => !moving.has(f.id) && f.universe === universe
      && Boolean(addressRange(f.address, fixtureFootprint(f)) && rangesOverlap(change.range, addressRange(f.address, fixtureFootprint(f))!)));
    if (conflict) throw new Error(`${change.label} would overlap ${conflict.label}.`);
  }
  return changes;
}
