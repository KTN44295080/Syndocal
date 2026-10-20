// Control > Timeline playback projections (Task 21). Pure helpers only; no backend, browser or output.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/controlTimelinePlayback.ts", import.meta.url), "utf8");
const code = ts.transpileModule(source, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
const m = await import(`data:text/javascript;base64,${Buffer.from(code).toString("base64")}`);

// Current cue: only the cue named by the engine's active fade counts down.
assert.deepEqual(m.currentCueTiming(null, null), { kind: "none" });
assert.deepEqual(m.currentCueTiming(undefined, { cue_id: 1, progress: 0.5, remaining_ms: 10, paused: false }), { kind: "none" });
assert.deepEqual(m.currentCueTiming(7, null), { kind: "held" });
assert.deepEqual(m.currentCueTiming(7, { cue_id: 8, progress: 0.2, remaining_ms: 900, paused: false }), { kind: "held" }, "another cue's fade is not this cue's countdown");
assert.deepEqual(m.currentCueTiming(7, { cue_id: 7, progress: 0.25, remaining_ms: 1500, paused: true }), { kind: "fading", remainingMs: 1500, progress: 0.25, paused: true });
assert.deepEqual(m.currentCueTiming(7, { cue_id: 7, progress: 3, remaining_ms: -5, paused: false }), { kind: "fading", remainingMs: 0, progress: 1, paused: false }, "malformed fade values clamp");
assert.deepEqual(m.currentCueTiming(7, { cue_id: 7, progress: Number.NaN, remaining_ms: Number.NaN, paused: false }), { kind: "fading", remainingMs: 0, progress: 0, paused: false });

// Next cue: counts down only to an authored Timeline event at or after the playhead.
const events = [
  { cue_id: 2, time_ms: 4000 },
  { cue_id: 2, time_ms: 9000 },
  { cue_id: 3, time_ms: 1000 },
];
assert.deepEqual(m.nextCueTiming(null, events, 0, true), { kind: "none" });
assert.deepEqual(m.nextCueTiming(9, events, 0, true), { kind: "manual" }, "a cue with no Timeline event is a manual GO");
assert.deepEqual(m.nextCueTiming(2, events, 1000, true), { kind: "timeline", remainingMs: 3000, playing: true });
assert.deepEqual(m.nextCueTiming(2, events, 4000, false), { kind: "timeline", remainingMs: 0, playing: false }, "an event exactly at the playhead is still upcoming");
assert.deepEqual(m.nextCueTiming(2, events, 4001, true), { kind: "timeline", remainingMs: 4999, playing: true }, "earliest event at or after the playhead wins");
assert.deepEqual(m.nextCueTiming(3, events, 2000, true), { kind: "manual" }, "past events do not count down");

// Formatting is numeric and locale-invariant.
assert.equal(m.formatCountdown(0), "0.0s");
assert.equal(m.formatCountdown(1250), "1.2s");
assert.equal(m.formatCountdown(59_999), "59.9s");
assert.equal(m.formatCountdown(60_000), "1:00");
assert.equal(m.formatCountdown(125_900), "2:05");
assert.equal(m.formatCountdown(-5), "0.0s");
assert.equal(m.formatCountdown(Number.NaN), "0.0s");

assert.equal(m.clampSeekMs(-10, 5000), 0);
assert.equal(m.clampSeekMs(7000, 5000), 5000);
assert.equal(m.clampSeekMs(1234.6, 5000), 1235);
assert.equal(m.clampSeekMs(10, 0), 0);

console.log("Control Timeline playback helpers ok");
