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
const cueManagementPanelSource = await readFile(
  new URL("../src/components/CueManagementPanel.tsx", import.meta.url),
  "utf8",
);
const localizationSource = await readFile(new URL("../src/uiLocalization.ts", import.meta.url), "utf8");
const stylesSource = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
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

for (const retiredPolicy of ["cueHasNonEffectTargets", "canSaveCueEffectTargets", "CueNonEffectTargets"]) {
  assert.equal(source.includes(retiredPolicy), false, `${retiredPolicy} must remain retired`);
}
for (const retiredPanelBranch of ["canSaveRecall", "recallSaveHintId", "cueRecallSaveHint"]) {
  assert.equal(cueManagementPanelSource.includes(retiredPanelBranch), false, `${retiredPanelBranch} must remain retired`);
}
const saveRecallButton = cueManagementPanelSource.match(
  /<button\s+class="cueEditOnly cueSaveRecall"([\s\S]*?)>\s*Save Recall\s*<\/button>/,
);
assert.ok(saveRecallButton, "Cue Management must render the Save Recall button");
assert.doesNotMatch(saveRecallButton[1], /\bdisabled=|\baria-describedby=/, "Save Recall must stay enabled without a last-target hint");
assert.match(
  saveRecallButton[1],
  /onClick=\{\(\) => void props\.onSetCueEffectTargets\(cue\.id, draft\(\)\.effect_targets\)\}/,
  "Save Recall must dispatch the authoritative effect-target replacement",
);
const retiredRecallCopy = "A Cue needs at least one target. Remove this Cue instead of saving an empty Effect-only Recall.";
assert.equal(cueManagementPanelSource.includes(retiredRecallCopy), false, "retired last-target warning must be absent from the panel");
assert.equal(localizationSource.includes(retiredRecallCopy), false, "retired last-target warning must be absent from localization");
assert.equal(stylesSource.includes(".cueRecallSaveHint"), false, "retired last-target warning style must be absent");
const emptyListHint = "Save Recall stores this Effect Recall list. An empty list clears all saved Effect Recall targets.";
assert.equal(cueManagementPanelSource.includes(emptyListHint), true, "Save Recall must explain empty-list clearing truthfully");
assert.equal(localizationSource.includes(emptyListHint), true, "empty-list Save Recall hint must be localized");
const visibleEmptyListHint = "No Effect Recall targets are selected. Save Recall will clear all saved Effect Recall targets.";
assert.match(
  cueManagementPanelSource,
  new RegExp(
    `<Show when=\\{draft\\(\\)\\.effect_targets\\.length === 0\\}>[\\s\\S]*?${visibleEmptyListHint.replace(/[.*+?^${}()|[\\]\\\\]/g, "\\$&")}[\\s\\S]*?<\\/Show>\\s*<div class="cueActionRow">`,
  ),
  "the destructive empty-list result must be visible beside Save Recall when the draft is empty",
);
assert.equal(localizationSource.includes(visibleEmptyListHint), true, "visible empty-list warning must be localized");

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
  /const undoProject = async \(\) => \{[\s\S]*?const undoAuthority = captureProjectAuthorityIdentity\(\);[\s\S]*?const undoHistory = projectHistoryStatus\(\);[\s\S]*?const undoEntryId = undoHistory\.undo_entry_id;[\s\S]*?const undoCheckpointHash = undoHistory\.undo_checkpoint_hash;[\s\S]*?typeof undoEntryId !== "number"[\s\S]*?typeof undoCheckpointHash !== "string"[\s\S]*?void refreshProjectHistoryStatus\(\);[\s\S]*?undo_project_transaction", \{[\s\S]*?ownerId: projectTransactionOwnerId,[\s\S]*?expectedEpoch: undoAuthority\.project_epoch,[\s\S]*?expectedEntryId: undoEntryId,[\s\S]*?expectedCheckpointHash: undoCheckpointHash,[\s\S]*?\}\);[\s\S]*?const applied = applyAuthorityBundleAsReplacement\(navigation\.authority\);[\s\S]*?if \(!projectAuthorityApplicationResultIsCurrent\(applied\)\) return;[\s\S]*?applyAuthoritativeProjectHistoryStatus\(navigation\.history_status\);[\s\S]*?\} catch \(error\) \{[\s\S]*?void pollProjectAuthorityBundle\(\);[\s\S]*?\n  \};\n\n  const redoProject = async/,
  "Undo must validate its exact project ticket, apply the authoritative rollback bundle, and refresh authority on failure",
);
assert.match(
  appSource,
  /const applyLoadedProjectResult = async[\s\S]*?const inlineAuthority = result\.authority;[\s\S]*?const bundle = inlineAuthority \?\? await fetchProjectAuthorityBundle\(result\);[\s\S]*?const authorityIsCurrent = inlineAuthority[\s\S]*?projectAuthorityInlineReplacementIsCurrent\([\s\S]*?: projectAuthorityFallbackIsCurrent[\s\S]*?if \(!authorityIsCurrent\) \{[\s\S]*?if \(applyProjectAuthorityBundle\(bundle, started\.application, true\) !== "applied"\) \{[\s\S]*?\n  \};\n\n  const resetRetiredProjectControlInputUi/,
  "Project loads must pass the exact authority guard and apply a replacement bundle that resets Cue and Recall drafts",
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
  /import \{[\s\S]*?authoredBeatsForEffectClock,[\s\S]*?chaserTraversalStepCount,[\s\S]*?cueOwnedEffectSummary,[\s\S]*?inferredCueAuthoredBeats,[\s\S]*?\} from "\.\/cueEffectRecall";/,
  "App must consume the extracted Cue timing and owned-summary helpers from the canonical module",
);
assert.match(
  source,
  /export const cueOwnedEffectSummary[\s\S]*?if \("Lfo" in params\)[\s\S]*?lfo: request/,
  "The canonical Cue-owned LFO summary helper must retain its complete source request",
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

assert.equal(helpers.chaserTraversalStepCount(chaserEffect), 2);
assert.equal(
  helpers.chaserTraversalStepCount({
    ...chaserEffect,
    chaser: { ...chaserEffect.chaser, direction: "Bounce", steps: [...chaserEffect.chaser.steps, chaserEffect.chaser.steps[0]] },
  }),
  4,
);
assert.equal(
  helpers.chaserTraversalStepCount({
    ...chaserEffect,
    chaser: { ...chaserEffect.chaser, direction: "Random", random_cycle_count: 3, steps: [...chaserEffect.chaser.steps, chaserEffect.chaser.steps[0]] },
  }),
  9,
);

const effectClockCases = [
  ["Color", "color"],
  ["Chaser", "chaser"],
  ["Move", "move_effect"],
  ["Value", "value"],
  ["Curve", "curve"],
  ["Mapping", "mapping"],
  ["ColorMapping", "color_mapping"],
];
for (const [effectType, field] of effectClockCases) {
  assert.equal(
    helpers.authoredBeatsForEffectClock({
      effect_type: effectType,
      clock_sync: { beats: 99 },
      [field]: { clock_sync: { beats: 3 } },
    }),
    3,
    `${effectType} must prefer its authored subtype clock`,
  );
}
assert.equal(
  helpers.authoredBeatsForEffectClock({ effect_type: "Lfo", clock_sync: { beats: 5 } }),
  5,
);

const inferredChaser = {
  ...chaserEffect,
  chaser: { ...chaserEffect.chaser, direction: "Bounce", clock_sync: { beats: 2 } },
};
assert.equal(
  helpers.inferredCueAuthoredBeats([inferredChaser], [{ effect_id: inferredChaser.id, enabled: true }]),
  4,
);
assert.equal(
  helpers.inferredCueAuthoredBeats([inferredChaser], [{ effect_id: inferredChaser.id, enabled: false }]),
  null,
);
assert.equal(
  helpers.inferredCueAuthoredBeats(
    [
      { id: 71, effect_type: "Lfo", clock_sync: { beats: 2 } },
      { id: 72, effect_type: "Lfo", clock_sync: { beats: 4 } },
    ],
    [{ effect_id: 71, enabled: true }, { effect_id: 72, enabled: true }],
  ),
  null,
  "conflicting authored beats must remain indeterminate",
);

const ownedLfoRequest = {
  label: "Owned LFO",
  fixture_ids: [1],
  target_group_ids: ["Front"],
  attribute: "Dimmer",
  video_targets: [],
  shape: "Sine",
  period_ms: 750,
  clock_sync: { beats: 2 },
  low: 100,
  high: 60_000,
  phase: 0.25,
  blend_mode: "Override",
  fixture_spread: 0.5,
};
const ownedLfoTarget = {
  effect_id: 73,
  enabled: false,
  params: { Lfo: ownedLfoRequest },
};
const ownedLfoSummary = helpers.cueOwnedEffectSummary(ownedLfoTarget, null);
assert.deepEqual(
  {
    id: ownedLfoSummary.id,
    type: ownedLfoSummary.effect_type,
    enabled: ownedLfoSummary.enabled,
    lfo: ownedLfoSummary.lfo,
    params: ownedLfoSummary.params,
  },
  {
    id: 73,
    type: "Lfo",
    enabled: false,
    lfo: ownedLfoRequest,
    params: ownedLfoTarget.params,
  },
  "Cue-owned LFO reconstruction must retain the exact source request",
);

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
assert.deepEqual(helpers.normalizedCueEffectTargets(effects, []), []);

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
