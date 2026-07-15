import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/clockDisplay.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "clockDisplay.ts",
});
const clock = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

assert.equal(clock.clockSourceLabel("MidiClock"), "MIDI Clock");
assert.equal(clock.clockSourceLabel("MidiTimecode"), "MTC");
assert.equal(clock.clockSourceLabel("Ltc"), "LTC");
assert.equal(clock.clockSourceLabel("Manual"), "Manual");
assert.equal(clock.clockSyncStatusLabel({ source: "Manual", external_sync_locked: false }), "INTERNAL");
assert.equal(clock.clockSyncStatusLabel({ source: "AbletonLink", external_sync_locked: true }), "LOCK");
assert.equal(clock.clockSyncStatusLabel({ source: "MidiTimecode", external_sync_locked: false }), "STALE");
assert.equal(clock.formatShowTimecode(0), "00:00:00.000");
assert.equal(clock.formatShowTimecode(3_723_004), "01:02:03.004");
assert.equal(clock.formatShowTimecode(Number.NaN), "00:00:00.000");

assert.equal(clock.formatCompactClock(0), "0:00.00");
assert.equal(clock.formatCompactClock(1_000), "0:01.00");
assert.equal(clock.formatCompactClock(120_200), "2:00.20");
assert.equal(clock.formatCompactClock(256_000), "4:16.00");
assert.equal(clock.formatCompactClock(59_999), "0:59.99");
assert.equal(clock.formatCompactClock(600_050), "10:00.05");
assert.equal(clock.formatCompactClock(Number.NaN), "0:00.00");
assert.equal(clock.formatCompactClock(-50), "0:00.00");

console.log("clock synchronization display helpers ok");
