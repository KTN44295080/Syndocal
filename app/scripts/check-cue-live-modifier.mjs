import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/cueLiveModifier.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "cueLiveModifier.ts",
});
const helpers = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);

// Clamp contracts must mirror the engine sanitizers exactly.
assert.equal(helpers.CUE_LIVE_MODIFIER_SPEED_MIN, 0.05);
assert.equal(helpers.CUE_LIVE_MODIFIER_SPEED_MAX, 20);
assert.equal(helpers.CUE_LIVE_MODIFIER_SIZE_MAX, 2);
assert.equal(helpers.sanitizeLiveModifierSpeed(Number.NaN), 1);
assert.equal(helpers.sanitizeLiveModifierSpeed(0), 1);
assert.equal(helpers.sanitizeLiveModifierSpeed(1000), 20);
assert.equal(helpers.sanitizeLiveModifierSpeed(0.001), 0.05);
assert.equal(helpers.sanitizeLiveModifierSize(-1), 1);
assert.equal(helpers.sanitizeLiveModifierSize(5), 2);
assert.equal(helpers.sanitizeLiveModifierPhase(1.25), 0.25);
assert.equal(helpers.sanitizeLiveModifierPhase(-0.25), 0.75);

// Authored defaults: absent settings mean neutral, no flash.
const legacyCue = { id: 5, effect_targets: [] };
assert.deepEqual(helpers.authoredCueLiveModifier(legacyCue), {
  speed: 1,
  size: 1,
  phase: 0,
  direction: "Authored",
  segment: 0,
  flash: false,
});
const flashCue = {
  id: 6,
  effect_targets: [],
  live_modifiers: { speed: 2, size: 0.5, phase: 0.25, flash: true },
};
assert.deepEqual(helpers.authoredCueLiveModifier(flashCue), {
  speed: 2,
  size: 0.5,
  phase: 0.25,
  direction: "Authored",
  segment: 0,
  flash: true,
});

// Effective values: the latched live override wins, flash mode stays authored.
const liveStates = [{
  cue_id: 6,
  speed: 4,
  size: 1,
  phase: 0,
  direction: "Reverse",
  segment: 7,
}];
assert.deepEqual(helpers.effectiveCueLiveModifier(flashCue, liveStates), {
  speed: 4,
  size: 1,
  phase: 0,
  direction: "Reverse",
  segment: 0,
  flash: true,
});
assert.deepEqual(helpers.effectiveCueLiveModifier(flashCue, []), {
  speed: 2,
  size: 0.5,
  phase: 0.25,
  direction: "Authored",
  segment: 0,
  flash: true,
});

// Authored/live distinction: only a diverging latch reads as overridden.
assert.equal(helpers.cueLiveModifierIsOverridden(flashCue, liveStates), true);
assert.equal(
  helpers.cueLiveModifierIsOverridden(flashCue, [
    {
      cue_id: 6,
      speed: 2,
      size: 0.5,
      phase: 0.25,
      direction: "Authored",
      segment: 0,
    },
  ]),
  false,
);
assert.equal(helpers.cueLiveModifierIsOverridden(flashCue, []), false);
assert.equal(helpers.cueLiveModifierIsOverridden(legacyCue, liveStates), false);
assert.equal(helpers.sanitizeLiveModifierDirection("Reverse"), "Reverse");
assert.equal(helpers.sanitizeLiveModifierDirection("unexpected"), "Authored");
assert.equal(helpers.sanitizeLiveModifierSegment(7, 3), 3);
assert.equal(helpers.sanitizeLiveModifierSegment(Number.NaN, 3), 0);

// Readout formatting stays compact for the dense desk chips.
assert.equal(helpers.formatLiveModifierSpeed(2), "x2");
assert.equal(helpers.formatLiveModifierSpeed(0.5), "x0.5");
assert.equal(helpers.formatLiveModifierSize(0.5), "50%");
assert.equal(helpers.formatLiveModifierPhase(0.25), "25%");

console.log("T17/T20 cue live modifier clamp, playback, authored/live, and readout contracts ok");
