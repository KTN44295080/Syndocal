import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

const source = await readFile(new URL("../src/effectDraft.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "effectDraft.ts",
});
const helpers = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

const fixtureIds = [{ id: 1 }, { id: 2 }, { id: 3 }];
const baseEffect = {
  id: 7,
  label: "Shared pulse",
  effect_type: "Lfo",
  fixture_ids: [1],
  target_group_ids: [],
  attribute: "Dimmer",
  video_targets: [{ layer_ids: [10], param: "Opacity", low: 0, high: 1, position: null }],
  shape: "Sine",
  period_ms: 1000,
  clock_sync: null,
  low: 0,
  high: 65535,
  phase: 0,
  blend_mode: "Override",
  enabled: true,
};

const fixturePlan = helpers.effectDraftTargetPlan(baseEffect, fixtureIds);
assert.equal(fixturePlan.mode, "fixture");
assert.equal(fixturePlan.videoTargetLinked, true);
assert.equal(fixturePlan.activeFixtureId, 1);
assert.equal(fixturePlan.videoTarget.layer_ids[0], 10);
assert.equal(fixturePlan.omittedVideoBindingCount, 0);

const groupPlan = helpers.effectDraftTargetPlan(
  { ...baseEffect, fixture_ids: [], target_group_ids: ["front", "wash"] },
  fixtureIds,
);
assert.equal(groupPlan.mode, "group");
assert.equal(groupPlan.targetGroups, "front, wash");
assert.equal(groupPlan.videoTargetLinked, true);

const selectionPlan = helpers.effectDraftTargetPlan({ ...baseEffect, fixture_ids: [1, 2, 99] }, fixtureIds);
assert.equal(selectionPlan.mode, "selection");
assert.deepEqual(selectionPlan.fixtureIds, [1, 2]);
assert.equal(selectionPlan.activeFixtureId, 1);

const videoPlan = helpers.effectDraftTargetPlan(
  { ...baseEffect, fixture_ids: [], attribute: "", video_targets: baseEffect.video_targets },
  fixtureIds,
);
assert.equal(videoPlan.mode, "video");
assert.equal(videoPlan.videoTargetLinked, false);

const multiVideoPlan = helpers.effectDraftTargetPlan(
  {
    ...baseEffect,
    video_targets: [
      { ...baseEffect.video_targets[0], layer_ids: [10, 11] },
      { ...baseEffect.video_targets[0], layer_ids: [12] },
    ],
  },
  fixtureIds,
);
assert.equal(multiVideoPlan.omittedVideoBindingCount, 2);
assert.match(multiVideoPlan.message, /2 additional video bindings remain active/);

const missingFixturePlan = helpers.effectDraftTargetPlan({ ...baseEffect, fixture_ids: [99] }, fixtureIds);
assert.equal(missingFixturePlan.mode, "source");
assert.equal(missingFixturePlan.videoTargetLinked, false);

console.log("effect draft helpers ok");
