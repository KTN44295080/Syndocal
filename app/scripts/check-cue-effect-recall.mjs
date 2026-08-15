import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/cueEffectRecall.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "cueEffectRecall.ts",
});
const helpers = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`
);
const guardSource = await readFile(new URL("../src/snapshotRequestGuard.ts", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const authoredEffectEnableSource = await readFile(
  new URL("../src/authoredEffectEnableController.ts", import.meta.url),
  "utf8",
);
const editorSource = await readFile(new URL("../src/components/CueEffectRecallEditor.tsx", import.meta.url), "utf8");
const guardTranspiled = ts.transpileModule(guardSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "snapshotRequestGuard.ts",
});
const snapshotGuards = await import(
  `data:text/javascript;base64,${Buffer.from(guardTranspiled.outputText).toString("base64")}`
);

assert.match(
  appSource,
  /const projectMutationCommands = new Set\(\[[\s\S]*?"set_cue_effect_targets"[\s\S]*?\]\);/,
  "Effect Recall saves must participate in project history",
);
assert.match(
  editorSource,
  /effect\.effect_type === "Chaser"\) return "Chaser"/,
  "Cue Effect Recall must label Chaser effects as Chaser instead of LFO",
);
assert.match(
  editorSource,
  /data-cue-effect-transition|Effect \$\{row\.id\} transition milliseconds/,
  "Cue-owned Effect rows must expose the optional transition control",
);
assert.match(
  appSource,
  /const undoProject = async \(\) => \{[\s\S]*?refreshSnapshot\(true, true\)/,
  "Undo must re-seed Cue and Recall editor state from the restored project snapshot",
);
assert.match(
  appSource,
  /const applyLoadedProjectResult = async[\s\S]*?refreshSnapshot\(true, true\)/,
  "Project loads must not retain Cue or Recall drafts from the previous project",
);
assert.match(
  appSource,
  /const mirrorCueEffectCaptureTargetForAuthoredIntent = \([\s\S]*?const mirrorsLiveState = Boolean\(previousTarget\)[\s\S]*?!cueEffectCaptureStateOverrideIds\(\)\.includes\(intent\.effectId\)[\s\S]*?setCueEffectCaptureTargets\([\s\S]*?enabled: intent\.enabled/,
  "canonical effect enqueue must update a non-overridden Store Recall target to its latest intent",
);
assert.match(
  appSource,
  /const authoredEffectEnableIntents = createAuthoredEffectEnableIntentController\([\s\S]*?onQueued: mirrorCueEffectCaptureTargetForAuthoredIntent/,
  "Store Recall mirroring must be wired to the canonical authored enqueue lifecycle",
);
assert.match(
  authoredEffectEnableSource,
  /lane\.latest = \{ effectId, enabled, generation: allocateGeneration\(\) \};[\s\S]*?options\.onQueued\?\.\(lane\.latest\);[\s\S]*?if \(!lane\.running\) \{[\s\S]*?lane\.drain = runLane/,
  "Store Recall lifecycle update must run synchronously before the effect lane starts async dispatch",
);
assert.match(
  appSource,
  /const rollbackCueEffectCaptureTargetForAuthoredIntent = \([\s\S]*?settledEnabled: boolean,[\s\S]*?if \(!latest \|\| latest\.generation !== intent\.generation\) return;[\s\S]*?if \(cueEffectCaptureStateOverrideIds\(\)\.includes\(intent\.effectId\)\) return;[\s\S]*?target\.effect_id === intent\.effectId && target\.enabled === latest\.requestedEnabled[\s\S]*?enabled: settledEnabled/,
  "a failed effect command may roll back only its still-current non-overridden Store Recall target to the settled baseline",
);
assert.match(
  appSource,
  /seedSettledBaseline: \(effectId\) => snapshot\(\)\.effects\.find\(\(effect\) => effect\.id === effectId\)\?\.enabled \?\? false/,
  "an idle effect lane must seed its rollback baseline from the authoritative snapshot instead of Store Recall draft state",
);
assert.match(
  appSource,
  /onFailed: \(intent, error, settledEnabled\) => \{[\s\S]*?rollbackCueEffectCaptureTargetForAuthoredIntent\(intent, settledEnabled\)/,
  "the latest-lane terminal failure must use the guarded Store Recall rollback",
);
assert.match(
  appSource,
  /const cueOwnedEffectSummary[\s\S]*?if \("Lfo" in params\)[\s\S]*?lfo: request/,
  "Cue-owned LFO summaries must retain their complete source request",
);
assert.match(
  appSource,
  /const useEffectAsDraft[\s\S]*?setLfoDaslightCurveSource\([\s\S]*?effect\.lfo\?\.daslight_curve \?\? null/,
  "Scene Settings must load the hidden Daslight Curve source profile into its edit draft",
);
assert.match(
  appSource,
  /period_ms: effectPeriod\(\),[\s\S]*?daslight_curve: lfoDaslightCurveSource\(\) \?\? undefined/,
  "Saving a Scene Settings LFO draft must preserve the imported Daslight Curve source profile",
);

const videoTarget = { layer_ids: [10], param: "Opacity", low: 0, high: 1, position: null };
const effects = [
  { id: 10, label: "Front pulse", effect_type: "Lfo", fixture_ids: [1], target_group_ids: [], video_targets: [], enabled: true },
  {
    id: 20,
    label: "Front colour",
    effect_type: "Color",
    fixture_ids: [],
    target_group_ids: [],
    video_targets: [],
    color: { fixture_ids: [], target_group_ids: ["Front"] },
    enabled: false,
  },
  { id: 30, label: "Rear wave", effect_type: "PositionWave", fixture_ids: [2], target_group_ids: [], video_targets: [], enabled: true },
  { id: 40, label: "Video-only FX", effect_type: "Lfo", fixture_ids: [], target_group_ids: [], video_targets: [videoTarget], enabled: false },
  { id: 50, label: "Unbound FX", effect_type: "Lfo", fixture_ids: [], target_group_ids: [], video_targets: [], enabled: true },
];
const context = {
  fixtures: [
    { id: 1, group_ids: ["Front", "Front/Wash"] },
    { id: 2, group_ids: ["Rear"] },
    { id: 3, group_ids: ["Front/Beam"] },
  ],
  selectedFixtureId: 1,
  selectedGroupId: "Front",
};
assert.equal(helpers.groupMatches("Front / Beam", "Front/Beam"), true);
assert.equal(helpers.groupMatches("Front/Beam", "Front"), true);
assert.equal(helpers.groupMatches("front/Beam", "Front"), false);
assert.equal(helpers.groupMatches("Front//Beam", "Front"), false);
const chaserEffect = {
  id: 60,
  label: "Nested beam Chaser",
  effect_type: "Chaser",
  fixture_ids: [3],
  target_group_ids: [],
  video_targets: [],
  chaser: {
    steps: [
      { fixture_ids: [3], target_group_ids: [], level: 65_535 },
      { fixture_ids: [], target_group_ids: [], level: 0 },
    ],
    features: [{ attribute: "Dimmer", low: 0, high: 65_535 }],
  },
  enabled: true,
};

assert.deepEqual(helpers.eligibleCueEffects([chaserEffect], "lighting", context).map((effect) => effect.id), [60]);
assert.deepEqual(helpers.eligibleCueEffects([chaserEffect], "video", context), []);
assert.deepEqual(helpers.eligibleCueEffects([chaserEffect], "selectedGroup", context).map((effect) => effect.id), [60]);
assert.deepEqual(
  helpers.eligibleCueEffects([chaserEffect], "selectedFixture", { ...context, selectedFixtureId: 3 }).map((effect) => effect.id),
  [60],
);

assert.deepEqual(helpers.eligibleCueEffects(effects, "all", context).map((effect) => effect.id), [10, 20, 30, 40, 50]);
assert.deepEqual(helpers.eligibleCueEffects(effects, "effects", context).map((effect) => effect.id), [10, 20, 30, 40, 50]);
assert.deepEqual(helpers.eligibleCueEffects(effects, "lighting", context).map((effect) => effect.id), [10, 20, 30]);
assert.deepEqual(helpers.eligibleCueEffects(effects, "video", context).map((effect) => effect.id), [40]);
assert.deepEqual(helpers.eligibleCueEffects(effects, "selectedFixture", context).map((effect) => effect.id), [10, 20]);
assert.deepEqual(helpers.eligibleCueEffects(effects, "selectedGroup", context).map((effect) => effect.id), [10, 20]);
assert.deepEqual(
  helpers.eligibleCueEffects(effects, "selectedFixture", { ...context, selectedFixtureId: 2 }).map((effect) => effect.id),
  [30],
);
assert.deepEqual(
  helpers.eligibleCueEffects(effects, "selectedFixture", { ...context, selectedFixtureId: null }),
  [],
);

assert.deepEqual(helpers.currentCueEffectTargets(effects), [
  { effect_id: 10, enabled: true },
  { effect_id: 20, enabled: false },
  { effect_id: 30, enabled: true },
  { effect_id: 40, enabled: false },
  { effect_id: 50, enabled: true },
]);

const normalized = helpers.normalizedCueEffectTargets(effects, [
  { effect_id: 20, enabled: true },
  { effect_id: 99, enabled: true },
  { effect_id: 10, enabled: false },
  { effect_id: 20, enabled: false },
]);
assert.deepEqual(normalized, [
  { effect_id: 10, enabled: false },
  { effect_id: 20, enabled: true },
]);

assert.deepEqual(helpers.selectAllCueEffects(effects, normalized), [
  { effect_id: 10, enabled: false },
  { effect_id: 20, enabled: true },
  { effect_id: 30, enabled: true },
  { effect_id: 40, enabled: false },
  { effect_id: 50, enabled: true },
]);

const liveStateChanged = effects.map((effect) => effect.id === 10 ? { ...effect, enabled: false } : effect);
const captureTargets = [
  { effect_id: 10, enabled: true },
  { effect_id: 20, enabled: true },
];
assert.deepEqual(
  helpers.syncCueEffectCaptureTargets(liveStateChanged, captureTargets, [], false),
  [
    { effect_id: 10, enabled: false },
    { effect_id: 20, enabled: false },
  ],
);
assert.deepEqual(
  helpers.syncCueEffectCaptureTargets(liveStateChanged, captureTargets, [10], false),
  [
    { effect_id: 10, enabled: true },
    { effect_id: 20, enabled: false },
  ],
);
assert.deepEqual(
  helpers.syncCueEffectCaptureTargets(effects, [{ effect_id: 10, enabled: false }], [10], true),
  [
    { effect_id: 10, enabled: false },
    { effect_id: 20, enabled: false },
    { effect_id: 30, enabled: true },
    { effect_id: 40, enabled: false },
    { effect_id: 50, enabled: true },
  ],
);

assert.deepEqual(helpers.refreshListedCueEffectStates(effects, normalized), [
  { effect_id: 10, enabled: true },
  { effect_id: 20, enabled: false },
]);

const withRear = helpers.setCueEffectIncluded(effects, normalized, 30, true);
assert.deepEqual(withRear, [
  { effect_id: 10, enabled: false },
  { effect_id: 20, enabled: true },
  { effect_id: 30, enabled: true },
]);
assert.deepEqual(helpers.setCueEffectIncluded(effects, withRear, 20, false), [
  { effect_id: 10, enabled: false },
  { effect_id: 30, enabled: true },
]);
assert.deepEqual(helpers.setCueEffectTargetEnabled(effects, withRear, 30, false), [
  { effect_id: 10, enabled: false },
  { effect_id: 20, enabled: true },
  { effect_id: 30, enabled: false },
]);
const ownedRear = withRear.map((target) => target.effect_id === 30
  ? { ...target, params: { Lfo: { label: "Owned rear" } } }
  : target);
const withTransition = helpers.setCueEffectTargetTransition(effects, ownedRear, 30, 1234.6);
assert.equal(withTransition.find((target) => target.effect_id === 30).transition_ms, 1235);
assert.equal(
  helpers.setCueEffectTargetTransition(effects, withTransition, 30, 900_000)
    .find((target) => target.effect_id === 30).transition_ms,
  600_000,
);
assert.equal(
  Object.hasOwn(
    helpers.setCueEffectTargetTransition(effects, withTransition, 30, null)
      .find((target) => target.effect_id === 30),
    "transition_ms",
  ),
  false,
  "clearing a transition must restore the legacy request shape",
);

const emptyCueTargets = {
  targets: [],
  video_targets: [],
  video_output_targets: [],
  node_graph_targets: [],
  palette_targets: [],
};
assert.equal(helpers.cueHasNonEffectTargets(emptyCueTargets), false);
assert.equal(helpers.canSaveCueEffectTargets(emptyCueTargets, []), false);
assert.equal(helpers.canSaveCueEffectTargets(emptyCueTargets, [{ effect_id: 10, enabled: true }]), true);
assert.equal(helpers.cueHasNonEffectTargets({ ...emptyCueTargets, targets: [{ fixture_id: 1, values: [] }] }), true);
assert.equal(helpers.cueHasNonEffectTargets({ ...emptyCueTargets, video_targets: [{}] }), true);
assert.equal(helpers.cueHasNonEffectTargets({ ...emptyCueTargets, video_output_targets: [{}] }), true);
assert.equal(helpers.cueHasNonEffectTargets({ ...emptyCueTargets, node_graph_targets: [{ graph_id: 1, enabled: true }] }), true);
assert.equal(helpers.cueHasNonEffectTargets({ ...emptyCueTargets, palette_targets: [{ palette_id: 1, fixture_ids: [1] }] }), true);
assert.equal(
  helpers.canSaveCueEffectTargets({ ...emptyCueTargets, node_graph_targets: [{ graph_id: 1, enabled: true }] }, []),
  true,
);

const snapshotGuard = snapshotGuards.createSnapshotRequestGuard();
const firstDeltaGeneration = snapshotGuard.beginDelta();
assert.notEqual(firstDeltaGeneration, null);
snapshotGuard.beginFull();
assert.equal(snapshotGuard.canApplyDelta(firstDeltaGeneration), false);
assert.equal(snapshotGuard.beginDelta(), null);
snapshotGuard.beginFull();
snapshotGuard.finishFull();
assert.equal(snapshotGuard.beginDelta(), null);
snapshotGuard.finishFull();
const secondDeltaGeneration = snapshotGuard.beginDelta();
assert.notEqual(secondDeltaGeneration, null);
assert.equal(snapshotGuard.canApplyDelta(secondDeltaGeneration), true);
const thirdDeltaGeneration = snapshotGuard.beginDelta();
assert.equal(snapshotGuard.canApplyDelta(secondDeltaGeneration), false);
assert.equal(snapshotGuard.canApplyDelta(thirdDeltaGeneration), true);

console.log("cue effect recall helpers ok");
