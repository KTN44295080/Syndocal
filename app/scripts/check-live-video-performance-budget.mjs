import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const importSource = async (name, replacements = {}) => {
  let source = await readFile(new URL(`../src/${name}.ts`, import.meta.url), "utf8");
  for (const [path, replacement] of Object.entries(replacements)) source = source.replaceAll(`"${path}"`, JSON.stringify(replacement));
  return `data:text/javascript;base64,${Buffer.from(ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
  }).outputText).toString("base64")}`;
};
const policyUrl = await importSource("liveVideoMonitorPolicy");
const { liveVideoPerformanceBudget: assess, performanceBudgetMinimumSamples } = await import(await importSource("liveVideoPerformanceBudget", {
  "./liveVideoMonitorPolicy": policyUrl,
}));

const output = {
  outputId: 1, kind: "display", targetFps: 60, deliveredFps: 60, sampleCount: 120,
  frameTimeP50Ms: 16.7, frameTimeP95Ms: 18, frameTimeP99Ms: 20, droppedFrames: 20,
  deliveredFrames: 120, measurementBoundary: "present", lastFrameAgeMs: 2,
  recentMissedFrames: 0, renderTimeP50Ms: 7, renderTimeP95Ms: 9.8, renderTimeP99Ms: 12, renderSampleCount: 120,
};
const healthy = assess(output);
assert.equal(healthy.budgetMs, 1_000 / 60);
assert.equal(healthy.renderP95Ms, 9.8, "render wall time, never present cadence, is the budget numerator");
assert.equal(healthy.warning, null, "old cumulative misses do not predict current overload");
assert.equal(assess({ ...output, renderTimeP95Ms: 1_000 / 60 }).warning, null, "exact budget boundary");
assert.equal(assess({ ...output, renderTimeP95Ms: 17 }).warning, "Frame preparation over budget");
assert.equal(assess({ ...output, recentMissedFrames: 6 }).warning, null);
assert.equal(assess({ ...output, recentMissedFrames: 7 }).warning, "Output missing frames");
assert.equal(assess({ ...output, lastFrameAgeMs: 1_001, sampleCount: 1 }).warning, "Output stalled", "stall is observable before enough timing samples");
assert.equal(assess({ ...output, lastFrameAgeMs: 1_001 }).renderP95Ms, null, "stale timing must not advertise headroom");
assert.equal(assess({ ...output, renderSampleCount: performanceBudgetMinimumSamples - 1, renderTimeP95Ms: 17 }).renderP95Ms, null);
assert.equal(assess({ ...output, renderSampleCount: performanceBudgetMinimumSamples }).renderP95Ms, 9.8);
assert.equal(assess({ ...output, renderSampleCount: undefined }).renderP95Ms, null, "cadence count must not replace render sample count");
for (const renderSampleCount of [15.5, Infinity, NaN, -1]) {
  const invalid = assess({ ...output, renderSampleCount, renderTimeP95Ms: 17 });
  assert.equal(invalid.renderP95Ms, null, "invalid sample counts cannot establish measured preparation time");
  assert.equal(invalid.warning, null, "invalid samples cannot predict overload");
}
for (const renderTimeP95Ms of [undefined, NaN, Infinity, -1]) {
  assert.equal(assess({ ...output, renderTimeP95Ms }).renderP95Ms, null, "missing/invalid telemetry never becomes zero rendering time");
}
for (const lastFrameAgeMs of [null, NaN, -1]) assert.equal(assess({ ...output, lastFrameAgeMs }).renderP95Ms, null);
for (const targetFps of [0, -60, NaN, Infinity, Number.MIN_VALUE]) assert.equal(assess({ ...output, targetFps }).budgetMs, null);
assert.equal(assess({ ...output, targetFps: 30, renderTimeP95Ms: 20 }).warning, null, "budget follows configured cadence");
assert.equal(assess({ ...output, renderTimeP95Ms: undefined, recentMissedFrames: 7 }).warning, "Output missing frames", "observed deficit does not depend on render telemetry");
console.log("live video performance budget: passed timing validity, sample gate, cadence and warning regressions");
