import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const helperSource = await readFile(new URL("../src/chaserDraft.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(helperSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "chaserDraft.ts",
});
const helpers = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.deepEqual(
  helpers.chaserStepsFromTargets([3, 1, 3, 2], [], false),
  [
    { fixture_ids: [3], target_group_ids: [], level: 65_535 },
    { fixture_ids: [1], target_group_ids: [], level: 65_535 },
    { fixture_ids: [2], target_group_ids: [], level: 65_535 },
  ],
  "fixture-index Chasers must preserve the stable incoming patch order",
);
assert.deepEqual(
  helpers.chaserStepsFromTargets([7], [], true),
  [
    { fixture_ids: [7], target_group_ids: [], level: 65_535 },
    { fixture_ids: [], target_group_ids: [], level: 0 },
  ],
  "a truly single fixture needs one targetless gap step",
);
assert.deepEqual(
  helpers.chaserStepsFromTargets([], ["Front", "Front", " Floor "], false),
  [
    { fixture_ids: [], target_group_ids: ["Front"], level: 65_535 },
    { fixture_ids: [], target_group_ids: ["Floor"], level: 65_535 },
  ],
  "explicit preset group steps remain representable and stable",
);

const validDraft = {
  steps: helpers.chaserStepsFromTargets([1, 2, 3], [], false),
  features: [
    { attribute: "Dimmer", low: 0, high: 65_535 },
    { attribute: "ColorRed", low: 4_096, high: 49_152 },
  ],
  direction: "Forward",
  stepDurationMs: 250,
  clockSyncBeats: 0.5,
  wings: 1,
  activeStepCount: 2,
  dutyCycle: 0.75,
  overlap: 0.25,
  phase: 0.5,
  fixtureSpread: 0,
  randomSeed: 1_337,
};
assert.equal(helpers.chaserDraftError(validDraft), "");
assert.match(helpers.chaserDraftError({ ...validDraft, activeStepCount: 4 }), /Pixels on/);
assert.match(
  helpers.chaserDraftError({
    ...validDraft,
    features: [...validDraft.features, { attribute: "dimmer", low: 0, high: 1 }],
  }),
  /only be added once/,
);
assert.match(
  helpers.chaserDraftError({
    ...validDraft,
    features: [...validDraft.features, { attribute: "D-immer", low: 0, high: 1 }],
  }),
  /only be added once/,
);
assert.match(
  helpers.chaserDraftError({
    ...validDraft,
    steps: validDraft.steps.map((step) => ({ ...step, fixture_ids: [], target_group_ids: [] })),
  }),
  /must target/,
);
assert.match(helpers.chaserDraftError({ ...validDraft, wings: 4 }), /step count/);

assert.deepEqual(helpers.chaserPreviewOrder(4, "Forward", 1_337), [0, 1, 2, 3]);
assert.deepEqual(helpers.chaserPreviewOrder(4, "Reverse", 1_337), [3, 2, 1, 0]);
assert.deepEqual(helpers.chaserPreviewOrder(4, "Bounce", 1_337), [0, 1, 2, 3, 2, 1]);
assert.deepEqual(helpers.chaserPreviewOrder(4, "BuildUpDown", 1_337), [0, 1, 2, 3, 0, 1, 2, 3]);
assert.deepEqual(
  helpers.chaserActivePreviewIndices(helpers.chaserPreviewOrder(4, "BuildUpDown", 1_337), 2, 1, "BuildUpDown"),
  [0, 1, 2],
  "Build / clear preview must accumulate through the fill half",
);
assert.deepEqual(
  helpers.chaserActivePreviewIndices(helpers.chaserPreviewOrder(4, "BuildUpDown", 1_337), 5, 1, "BuildUpDown"),
  [2, 3],
  "Build / clear preview must remove targets in source order",
);
assert.equal(
  helpers.chaserDraftError({
    ...validDraft,
    direction: "BuildUpDown",
    steps: validDraft.steps.slice(0, 1),
    activeStepCount: 1,
  }),
  "",
  "Build / clear must support the one-target cycle Daslight emits without a synthetic blackout step",
);
assert.match(
  helpers.chaserDraftError({ ...validDraft, direction: "BuildUpDown", activeStepCount: 2 }),
  /Pixels on must be 1/,
  "Build / clear must reject a width control that its one-frontier algorithm cannot honor",
);
assert.deepEqual(
  helpers.chaserPreviewOrder(8, "Random", 1_337),
  helpers.chaserPreviewOrder(8, "Random", 1_337),
  "random traversal must be deterministic for saved seed",
);
assert.notDeepEqual(
  helpers.chaserPreviewOrder(8, "Random", 1_337),
  helpers.chaserPreviewOrder(8, "Random", 1_338),
);
assert.equal(helpers.chaserPreviewIndex(4, "Forward", 1_337, 0.5), 2);
assert.deepEqual(
  helpers.chaserActivePreviewIndices(helpers.chaserPreviewOrder(4, "Bounce", 1_337), 4, 4),
  [2, 3, 1, 0],
  "Bounce Pixels on must skip repeated turnaround steps until the exact distinct width is active",
);
assert.equal(new Set(helpers.chaserPreviewOrder(8, "Random", 1_337)).size, 8);
assert.equal(
  helpers.chaserActivePreviewIndices(helpers.chaserPreviewOrder(8, "Random", 1_337), 0, 8).length,
  8,
  "seeded Random must expose every step exactly once per traversal",
);
assert.equal(helpers.chaserPreviewIndex(4, "Forward", 1_337, 1), 0, "phase 1 must wrap like the engine");

const componentSource = await readFile(new URL("../src/components/ChaserEffectEditorPanel.tsx", import.meta.url), "utf8");
assert.match(componentSource, /const chaserStepsPerPage = 8/);
assert.match(componentSource, /<For each=\{visibleSteps\(\)\}>/);
assert.match(componentSource, /aria-rowcount=\{props\.steps\.length\}/);
assert.match(componentSource, /data-active-step-count=\{props\.activeStepCount\}/);
assert.match(componentSource, /Pixels on\s*<input/);
assert.match(componentSource, /disabled=\{props\.direction === "BuildUpDown"\}/);
assert.match(componentSource, /Fading\s*<\/label>/);
assert.match(componentSource, />Features<\/legend>/);
assert.match(componentSource, /props\.onWings\(Math\.min\(/);
assert.match(componentSource, /max=\{wingMaximum\(\)\}/);
assert.match(componentSource, /Replace Chaser step \$\{index\(\) \+ 1\} with current target/);

const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
assert.match(appSource, /"add_chaser_effect"/);
assert.match(appSource, /"update_chaser_effect"/);
assert.match(appSource, /effectTargetFixtures\(\)\.map\(\(fixture\) => fixture\.id\)/);
assert.match(appSource, /chaserStepsFromTargets\([\s\S]*?\[\],[\s\S]*?false/);
assert.match(appSource, /const chaserAttributeOptions = createMemo/);
assert.match(appSource, /attributeCoverage:\s*chaserAttributeCoverage\(\)/);
assert.match(appSource, /prepareChaserDraftFromCurrentTarget = \(forceReset = false\)/);
assert.match(appSource, /prepareChaserDraftFromCurrentTarget\(nextType !== previousType\)/);
assert.match(appSource, /forceReset \? 1 : Math\.max\(1, Math\.min\(current, maxActiveSteps\)\)/);
assert.match(componentSource, /chaserFeatureCoverage/);
const sceneSettingsSource = await readFile(
  new URL("../src/components/SceneSettingsPane.tsx", import.meta.url),
  "utf8",
);
assert.match(sceneSettingsSource, /data-scene-fx-chooser/);
assert.match(sceneSettingsSource, /<ChaserEffectEditorPanel \{\.\.\.props\.editor\.chaser\} \/>/);
assert.match(sceneSettingsSource, /data-scene-settings-effect-editor=\{props\.editor\.effectType\}/);

const tauriSource = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
assert.match(
  tauriSource,
  /sample_effect_requires_target[\s\S]*?"chase"/,
  "the removed global sample library must not remove backend Chase target validation",
);

console.log("Chaser draft, fixture order, responsive preview, and DOM boundaries ok");
