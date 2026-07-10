import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

async function importTsModule(path) {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const transpiled = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: path,
  });
  return import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);
}

const dmx = await importTsModule("../src/dmxAddressing.ts");

const controls = [
  { offsets: [1] },
  { offsets: [2, 3] },
  { offsets: [8] },
];
assert.equal(dmx.fixtureFootprintFromControls(controls), 8);
assert.deepEqual(dmx.addressRange(1, 8), [1, 8]);
assert.equal(dmx.addressRange(0, 8), null);
assert.equal(dmx.addressRange(1, 0), null);
assert.equal(dmx.rangesOverlap([1, 8], [8, 16]), true);
assert.equal(dmx.rangesOverlap([1, 8], [9, 16]), false);

const fixtures = [
  { universe: 0, address: 1, controls },
  { universe: 0, address: 9, controls },
  { universe: 0, address: 25, controls },
  { universe: 1, address: 1, controls },
  { universe: 0, address: 200, controls: [] },
];
const ranges = dmx.buildOccupiedDmxRanges(fixtures);
assert.deepEqual(ranges.get(0), [
  { start: 1, end: 8 },
  { start: 9, end: 16 },
  { start: 25, end: 32 },
]);
assert.equal(dmx.dmxAddressIsFree(ranges, 0, 17, 8), true);
assert.equal(dmx.dmxAddressIsFree(ranges, 0, 16, 8), false);
assert.equal(dmx.dmxAddressIsFree(ranges, 0, 506, 8), false);
assert.equal(dmx.findFreeDmxAddress(ranges, 0, 8, 9), 17);
assert.equal(dmx.findFreeDmxAddress(ranges, 1, 8, 1), 9);
assert.equal(dmx.findFreeDmxAddress(ranges, 0, 600, 1), null);
assert.equal(dmx.reserveDmxAddressRange(ranges, 0, 17, 8), true);
assert.equal(dmx.findFreeDmxAddress(ranges, 0, 8, 9), 33);
assert.equal(dmx.reserveDmxAddressRange(ranges, 0, 506, 8), false);

assert.equal(dmx.canPlacePatchAtAddress(fixtures, 17, 0, 8, 1, 8), true);
assert.equal(dmx.canPlacePatchAtAddress(fixtures, 16, 0, 8, 1, 8), false);
assert.equal(dmx.canPlacePatchAtAddress(fixtures, 17, 0, 8, 2, 4), false);
assert.equal(dmx.canPlacePatchInOccupiedRanges(dmx.buildOccupiedDmxRanges(fixtures), 17, 0, 8, 1, 8), true);
assert.equal(dmx.findNextFreePatchAddress(fixtures, 0, 8, 2, 8, 1), 33);
assert.equal(dmx.findNextFreePatchAddress(fixtures, 1, 8, 2, 8, 1), 9);
assert.equal(dmx.findNextFreePatchAddress(fixtures, 0, 8, 2, 4, 1), null);

console.log("dmx addressing helpers ok");
