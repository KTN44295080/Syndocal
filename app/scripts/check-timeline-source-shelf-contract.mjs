import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const transpile = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName,
});
const dataModule = (source, fileName) =>
  `data:text/javascript;base64,${Buffer.from(transpile(source, fileName).outputText).toString("base64")}`;
const load = async (fileName) => import(dataModule(
  await readFile(new URL(`../src/${fileName}`, import.meta.url), "utf8"),
  fileName,
));

const gestureSource = await readFile(new URL("../src/timelineBlockGestures.ts", import.meta.url), "utf8");
const canonicalKindSource = await readFile(new URL("../src/sceneCueKind.ts", import.meta.url), "utf8");
const canonicalKindModuleUrl = dataModule(canonicalKindSource, "sceneCueKind.ts");
const timelineSource = await readFile(new URL("../src/timelineSceneBlocks.ts", import.meta.url), "utf8");
const helpers = await import(dataModule(
  timelineSource
    .replace("./timelineBlockGestures", dataModule(gestureSource, "timelineBlockGestures.ts"))
    .replace("./sceneCueKind", canonicalKindModuleUrl),
  "timelineSceneBlocks.ts",
));
const canonicalKinds = await import(canonicalKindModuleUrl);
const bankAuthority = await load("bankAuthority.ts");
const drag = await load("timelineExternalDrag.ts");
const identity = await load("identityColor.ts");
const shelfSource = await readFile(new URL("../src/components/TimelineSourceShelf.tsx", import.meta.url), "utf8");
const matrixSource = await readFile(new URL("../src/components/SceneMatrixPanel.tsx", import.meta.url), "utf8");
const cueManagementSource = await readFile(new URL("../src/components/CueManagementPanel.tsx", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const timelineCommandDispatchersSource = await readFile(new URL("../src/timelineCommandDispatchers.ts", import.meta.url), "utf8");
const cssSource = await readFile(new URL("../src/styles.css", import.meta.url), "utf8");
const packageSource = await readFile(new URL("../package.json", import.meta.url), "utf8");
const playbackExecutorSource = await readFile(new URL("../src/components/PlaybackExecutorPanel.tsx", import.meta.url), "utf8");
const timelineEditorSource = await readFile(new URL("../src/components/TimelineSceneBlocksEditor.tsx", import.meta.url), "utf8");
const timelinePanelSource = await readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8");
const timelineOverviewSource = await readFile(new URL("../src/components/TimelineOverview.tsx", import.meta.url), "utf8");
const editableTouchSource = await readFile(new URL("../src/components/EditableTouchSurface.tsx", import.meta.url), "utf8");
const touchCueSource = await readFile(new URL("../src/components/TouchCuePanel.tsx", import.meta.url), "utf8");
const bankAuthoritySource = await readFile(new URL("../src/bankAuthority.ts", import.meta.url), "utf8");
const initialSnapshotSource = await readFile(new URL("../src/initialEngineSnapshot.ts", import.meta.url), "utf8");

assert.match(
  shelfSource,
  /resolution\.mode === "stale"[\s\S]*Selected \$\{payload\.lane_kind\} Timeline lane is no longer available/,
  "a stale explicit Shelf lane remains visible and fail-closed instead of auto-retargeting",
);
assert.match(
  shelfSource,
  /resolution\.candidates\.length > 1 \|\| resolution\.mode === "stale"/,
  "the lane chooser reappears when an explicit selection becomes stale, even if one candidate remains",
);

const exactMainAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 1, label: "Main", active_cue_id: null }],
  [{ id: 10, cue_list_id: 1 }],
  [{ id: 20, cue_list_id: 1 }],
);
assert.equal(exactMainAuthority.issue, null);
assert.equal(exactMainAuthority.cueLists[0].label, "Main", "persisted Main remains exact and is never displayed as Bank 1");
assert.equal(bankAuthority.bankAuthoritySelectedCueList(exactMainAuthority, 1)?.label, "Main");

const noIdOneAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 7, label: "Seven", active_cue_id: null }],
  [],
  [],
);
assert.equal(bankAuthority.bankAuthoritySelectedCueList(noIdOneAuthority, null)?.id, 7);
assert.equal(bankAuthority.bankAuthoritySelectedCueList(noIdOneAuthority, 1), null, "a stale non-null selection must not move to the first Bank");

const authorityFaultFixtures = [
  ["invalid Bank", bankAuthority.inspectBankAuthority([{ id: 0, label: "Bad" }, { id: 7, label: "Seven" }]), "invalid_bank_id"],
  ["unsafe Bank", bankAuthority.inspectBankAuthority([{ id: Number.MAX_SAFE_INTEGER + 1, label: "Bad" }, { id: 7, label: "Seven" }]), "invalid_bank_id"],
  ["duplicate Bank", bankAuthority.inspectBankAuthority([{ id: 7, label: "A" }, { id: 7, label: "B" }]), "duplicate_bank_id"],
  ["invalid Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [{ id: 0, cue_list_id: 7 }]), "invalid_cue_id"],
  ["duplicate Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [{ id: 9, cue_list_id: 7 }, { id: 9, cue_list_id: 7 }]), "duplicate_cue_id"],
  ["invalid Cue Bank", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [{ id: 9, cue_list_id: 0 }]), "invalid_cue_bank_id"],
  ["missing Cue Bank", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [{ id: 9, cue_list_id: 8 }]), "missing_cue_bank"],
  ["invalid active Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven", active_cue_id: 0 }], [{ id: 9, cue_list_id: 7 }]), "invalid_bank_active_cue_id"],
  ["unsafe active Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven", active_cue_id: Number.MAX_SAFE_INTEGER + 1 }], [{ id: 9, cue_list_id: 7 }]), "invalid_bank_active_cue_id"],
  ["missing active Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven", active_cue_id: 10 }], [{ id: 9, cue_list_id: 7 }]), "missing_bank_active_cue"],
  ["cross-Bank active Cue", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven", active_cue_id: 9 }, { id: 8, label: "Eight" }], [{ id: 9, cue_list_id: 8 }]), "cross_bank_active_cue"],
  ["invalid Executor", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [], [{ id: 0, cue_list_id: 7 }]), "invalid_executor_id"],
  ["duplicate Executor", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [], [{ id: 2, cue_list_id: 7 }, { id: 2, cue_list_id: 7 }]), "duplicate_executor_id"],
  ["invalid Executor Bank", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [], [{ id: 2, cue_list_id: 0 }]), "invalid_executor_bank_id"],
  ["missing Executor Bank", bankAuthority.inspectBankAuthority([{ id: 7, label: "Seven" }], [], [{ id: 2, cue_list_id: 8 }]), "missing_executor_bank"],
];
for (const [label, authority, expectedKind] of authorityFaultFixtures) {
  assert.equal(authority.issue?.kind, expectedKind, `${label} must poison the complete shared authority`);
  assert.equal(
    bankAuthority.bankAuthoritySelectedCueList(authority, 7),
    null,
    `${label} must not fall back to an otherwise valid Bank`,
  );
}
const guardedSurfaceCommandLog = [];
const dispatchOnlyWithFullAuthority = (authority, surface, command) => {
  if (authority.issue) return false;
  guardedSurfaceCommandLog.push({ surface, command });
  return true;
};
for (const [label, authority] of authorityFaultFixtures) {
  for (const surface of ["bank", "scene-placement", "executor", "matrix", "timeline-inner", "live-pad", "touch-pad"]) {
    assert.equal(dispatchOnlyWithFullAuthority(authority, surface, "mutation"), false, `${label} closes ${surface}`);
  }
}
assert.equal(guardedSurfaceCommandLog.length, 0, "all full-authority fault fixtures produce zero runtime/IPC commands");

const staleSelectedAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 2, label: "Second" }],
  [{ id: 22, cue_list_id: 2 }],
  [],
);
assert.equal(bankAuthority.bankAuthoritySelectedCueList(staleSelectedAuthority, 1), null);
const dispatchOnlyWithExactSelection = (authority, selectedBankId, command) => {
  if (authority.issue || bankAuthority.bankAuthoritySelectedCueList(authority, selectedBankId) === null) return false;
  guardedSurfaceCommandLog.push({ surface: "selected-bank", command });
  return true;
};
assert.equal(
  dispatchOnlyWithExactSelection(staleSelectedAuthority, 1, "create-scene-or-add-fader"),
  false,
  "poll-removing the selected Bank does not target the first Bank",
);
assert.equal(guardedSurfaceCommandLog.length, 0, "stale selection emits neither Create Scene nor Add Fader");
assert.equal(bankAuthority.nextBankAuthorityId([1, 2, 9]), 10);
assert.equal(bankAuthority.nextBankAuthorityId([Number.MAX_SAFE_INTEGER]), null);
const delayValidAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 1, label: "Bank 1", active_cue_id: 7 }],
  [{ id: 7, cue_list_id: 1 }],
  [],
);
const delayFaultAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 1, label: "Bank 1", active_cue_id: 7 }],
  [{ id: 7, cue_list_id: 2 }],
  [],
);
const delayFence = bankAuthority.createBankAuthorityDelayFence(
  bankAuthority.bankAuthoritySemanticToken(delayValidAuthority),
);
const delayedCueSeven = delayFence.capture({ project_epoch: 4, project_revision: 8, checkpoint_hash: "A" });
assert.equal(delayFence.observe(bankAuthority.bankAuthoritySemanticToken(delayFaultAuthority)), true);
assert.equal(delayFence.observe(bankAuthority.bankAuthoritySemanticToken(delayValidAuthority)), true);
assert.equal(
  delayFence.isCurrent(
    delayedCueSeven,
    { project_epoch: 4, project_revision: 8, checkpoint_hash: "A" },
    bankAuthority.bankAuthoritySemanticToken(delayValidAuthority),
  ),
  false,
  "Cue 7 queued before authority fault must stay retired after identical Cue 7 returns",
);
const checkpointFence = bankAuthority.createBankAuthorityDelayFence(
  bankAuthority.bankAuthoritySemanticToken(delayValidAuthority),
);
const checkpointCapture = checkpointFence.capture({ project_epoch: 4, project_revision: 8, checkpoint_hash: "A" });
assert.equal(
  checkpointFence.isCurrent(
    checkpointCapture,
    { project_epoch: 4, project_revision: 9, checkpoint_hash: "B" },
    bankAuthority.bankAuthoritySemanticToken(delayValidAuthority),
  ),
  false,
  "Control/Edit delayed work requires exact project E/R/H",
);
assert.match(initialSnapshotSource, /cue_lists: \[\{ id: 1, label: "Bank 1", active_cue_id: null \}\]/);
assert.match(initialSnapshotSource, /playback_executors: \[\{ id: 1, label: "Bank 1", cue_list_id: 1/);

const baseCue = {
  id: 100,
  cue_list_id: 9,
  cue_number: "09-A",
  label: "Nine Opening",
  fade_ms: 125,
  targets: [],
  palette_targets: [],
  video_targets: [],
  video_output_targets: [],
  node_graph_targets: [],
  effect_targets: [],
  child_timeline: null,
  live_modifiers: null,
  recall_mode: "Coexist",
  steps: [],
};

// The source order intentionally differs from Bank order. Bank 4 is empty;
// all four must still appear exactly as the authoritative cueLists provide.
const cues = [
  { ...baseCue, id: 701, cue_list_id: 7, cue_number: "7-2", label: "Seven Second", fade_ms: 700 },
  {
    ...baseCue,
    id: 901,
    cue_list_id: 9,
    cue_number: "09-A",
    label: "Nine Timeline",
    child_timeline: { layers: [], events: [] },
    effect_targets: [{ effect_id: 1 }],
    recall_mode: "ReplaceGroup",
    live_modifiers: { flash: true },
  },
  { ...baseCue, id: 702, cue_list_id: 7, cue_number: "7-1", label: "Seven First", effect_targets: [{ effect_id: 2 }] },
  { ...baseCue, id: 201, cue_list_id: 2, cue_number: "02-1", label: "Two Static" },
];
const options = helpers.buildTimelineSceneBlockCueOptions(cues);
assert.deepEqual(options.map((option) => ({
  id: option.id,
  cue_list_id: option.cue_list_id,
  cue_number: option.cue_number,
  label: option.label,
  kind: option.kind,
  super_scene: option.super_scene,
  replace_group: option.replace_group,
  flash: option.flash,
  fade_ms: option.fade_ms,
})), [
  { id: 701, cue_list_id: 7, cue_number: "7-2", label: "Seven Second", kind: "STATIC", super_scene: false, replace_group: false, flash: false, fade_ms: 700 },
  { id: 901, cue_list_id: 9, cue_number: "09-A", label: "Nine Timeline", kind: "TIMELINE", super_scene: true, replace_group: true, flash: true, fade_ms: 125 },
  { id: 702, cue_list_id: 7, cue_number: "7-1", label: "Seven First", kind: "FX", super_scene: false, replace_group: false, flash: false, fade_ms: 125 },
  { id: 201, cue_list_id: 2, cue_number: "02-1", label: "Two Static", kind: "STATIC", super_scene: false, replace_group: false, flash: false, fade_ms: 125 },
], "Timeline source fields and TL marker exactly mirror the authored Lighting cue");

// Any authored field rendered by the Shelf must invalidate the option cache.
// This protects a live Lighting edit from leaving a stale Scene number/name/kind
// or marker in Timeline Sources.
const optionMutations = [
  { ...cues[0], cue_number: "7-2A" },
  { ...cues[0], label: "Seven Renamed" },
  { ...cues[0], fade_ms: 701 },
  { ...cues[0], effect_targets: [{ effect_id: 99 }] },
  { ...cues[0], child_timeline: { layers: [], events: [] } },
  { ...cues[0], recall_mode: "ReplaceGroup" },
  { ...cues[0], live_modifiers: { flash: true } },
  { ...cues[0], cue_list_id: 2 },
];
for (const mutatedCue of optionMutations) {
  const mutatedOptions = helpers.buildTimelineSceneBlockCueOptions([mutatedCue, ...cues.slice(1)]);
  assert.equal(
    helpers.timelineSceneBlockCueOptionsEqual(options, mutatedOptions),
    false,
    "every Shelf-rendered authored field must invalidate Timeline Source options",
  );
}
assert.equal(
  helpers.timelineSceneBlockCueOptionsEqual(options, helpers.buildTimelineSceneBlockCueOptions([...cues].reverse())),
  false,
  "authored Scene order must invalidate Timeline Source options",
);

const sourceBankRows = [
  { id: 9, label: "Bank Nine" },
  { id: 2, label: "Bank Two" },
  { id: 7, label: "Bank Seven" },
  { id: 4, label: "Empty Bank" },
];
const sourceAuthority = bankAuthority.inspectBankAuthority(sourceBankRows, cues, []);
const authoritativeEventRows = helpers.buildTimelineSceneBlockRows([
  {
    id: 71,
    cue_id: 701,
    time_ms: 1200,
    time_beats: null,
    track: "Lighting",
    layer_id: 3,
    duration_ms: 400,
    duration_beats: null,
    conform_to_tempo: false,
    loop_fill: false,
    source_offset_ms: 0,
    fade_in_ms: 0,
    fade_out_ms: 0,
    loop_count: 1,
    jump_to_event_id: null,
  },
], sourceAuthority);
assert.deepEqual(
  authoritativeEventRows.map((row) => ({ id: row.id, cue_id: row.cue_id, cue_list_id: row.cue_list_id })),
  [{ id: 71, cue_id: 701, cue_list_id: 7 }],
  "an event row keeps only its exact authoritative Bank identity",
);
const orphanEventRows = helpers.buildTimelineSceneBlockRows([
  { ...authoritativeEventRows[0], id: 72, cue_id: 404 },
], sourceAuthority);
assert.deepEqual(
  orphanEventRows,
  [],
  "a vanished Cue is unavailable rather than being fabricated as Bank 0 or a placement candidate",
);
const rowAuthorityFault = bankAuthority.inspectBankAuthority(
  [{ id: 7, label: "Seven" }],
  [{ ...cues[0], cue_list_id: 8 }],
  [],
);
assert.deepEqual(
  helpers.buildTimelineSceneBlockRows(authoritativeEventRows, rowAuthorityFault),
  [],
  "faulting Bank authority closes all Timeline rows",
);
assert.deepEqual(
  helpers.buildTimelineSceneBlockRows(authoritativeEventRows, sourceAuthority).map((row) => row.id),
  [71],
  "repair restores only the current authoritative event rows; it cannot revive a fabricated orphan",
);
const banks = helpers.groupTimelineSourceShelfBanks(options, sourceAuthority);
assert.equal(
  helpers.timelineSourceShelfCueOptionsMatchAuthority(options, sourceAuthority),
  true,
  "the root Timeline accepts its exact complete authority Scene set",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(options.filter((option) => option.id !== 702), sourceAuthority),
  [],
  "a missing root Timeline Scene closes the complete keyed Shelf",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(
    options.map((option) => option.id === 701 ? { ...option, cue_list_id: 2 } : option),
    sourceAuthority,
  ),
  [],
  "a root Timeline Scene with a different Bank closes the complete keyed Shelf",
);
const childTimelineCueId = 901;
const rootAllowedSceneCueIds = helpers.timelineSceneBlockAllowedCueIds(sourceAuthority, null);
assert.deepEqual(
  [...(rootAllowedSceneCueIds ?? [])],
  cues.map((cue) => cue.id),
  "the root Timeline admits the complete exact authored Scene set",
);
const childAllowedSceneCueIds = helpers.timelineSceneBlockAllowedCueIds(sourceAuthority, childTimelineCueId);
assert.deepEqual(
  [...(childAllowedSceneCueIds ?? [])],
  [701, 702, 201],
  "a child Timeline omits its owner while retaining every acyclic descendant/leaf Scene",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(701, sourceAuthority, childTimelineCueId),
  true,
  "a normal Scene remains eligible inside a child Timeline",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(childTimelineCueId, sourceAuthority, childTimelineCueId),
  false,
  "a child Timeline cannot insert its own owner Scene",
);
const nestedChildTimelineAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 8, label: "Bank Eight", active_cue_id: null }],
  [
    { ...baseCue, id: 810, cue_list_id: 8, child_timeline: { layers: [], events: [{ id: 1, cue_id: 813 }] } },
    { ...baseCue, id: 811, cue_list_id: 8, child_timeline: null },
    {
      ...baseCue,
      id: 812,
      cue_list_id: 8,
      child_timeline: { layers: [], events: [{ id: 1, cue_id: 810 }] },
    },
    {
      ...baseCue,
      id: 813,
      cue_list_id: 8,
      child_timeline: { layers: [], events: [{ id: 1, cue_id: 811 }] },
    },
  ],
  [],
);
assert.equal(nestedChildTimelineAuthority.issue, null);
assert.deepEqual(
  [...(helpers.timelineSceneBlockAllowedCueIds(nestedChildTimelineAuthority, null) ?? [])],
  [810, 811, 812, 813],
  "the root Timeline retains every exact Cue even when nested Super Scene references exist",
);
assert.deepEqual(
  [...(helpers.timelineSceneBlockAllowedCueIds(nestedChildTimelineAuthority, 810) ?? [])],
  [811, 813],
  "child source admission excludes only owner-reaching graphs and retains an acyclic nested Super Scene",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(812, nestedChildTimelineAuthority, 810),
  false,
  "a child Timeline rejects a nested Super Scene whose child-reference graph returns to the owner",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(813, nestedChildTimelineAuthority, 810),
  true,
  "an acyclic nested Super Scene remains a valid child Timeline source",
);
const secondChildBaselineBefore = [
  ...(helpers.timelineSceneBlockAllowedCueIds(nestedChildTimelineAuthority, 812) ?? []),
];
assert.deepEqual(
  secondChildBaselineBefore,
  [810, 811, 813],
  "a second acyclic child retains its own exact source baseline while the first child excludes its indirect recursive target",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(810, nestedChildTimelineAuthority, 812),
  true,
  "the second child may still reference an acyclic nested Super Scene",
);
assert.deepEqual(
  [...(helpers.timelineSceneBlockAllowedCueIds(nestedChildTimelineAuthority, 810) ?? [])],
  [811, 813],
  "querying a second child must not weaken the first child's indirect-recursion exclusion",
);
assert.deepEqual(
  [...(helpers.timelineSceneBlockAllowedCueIds(nestedChildTimelineAuthority, 812) ?? [])],
  secondChildBaselineBefore,
  "source admission queries are read-only: the second child baseline remains unchanged",
);
assert.equal(
  helpers.timelineSceneBlockCueAllowedByAuthority(0, sourceAuthority, childTimelineCueId),
  false,
  "invalid Scene identifiers fail closed before any persistence boundary",
);
assert.equal(
  helpers.timelineSceneBlockAllowedCueIds(sourceAuthority, 701),
  null,
  "a non-child owner context is unverifiable and closes all child Timeline Scene admission",
);
const missingChildReferenceAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 6, label: "Bank Six", active_cue_id: null }],
  [{ ...baseCue, id: 610, cue_list_id: 6, child_timeline: { layers: [], events: [{ id: 0, cue_id: 999 }] } }],
  [],
);
assert.equal(missingChildReferenceAuthority.issue, null);
assert.equal(
  helpers.timelineSceneBlockAllowedCueIds(missingChildReferenceAuthority, null),
  null,
  "a missing child-reference endpoint makes the complete admission graph unverifiable",
);
const duplicateChildEventAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 5, label: "Bank Five", active_cue_id: null }],
  [
    { ...baseCue, id: 510, cue_list_id: 5, child_timeline: { layers: [], events: [{ id: 1, cue_id: 511 }, { id: 1, cue_id: 511 }] } },
    { ...baseCue, id: 511, cue_list_id: 5, child_timeline: null },
  ],
  [],
);
assert.equal(
  helpers.timelineSceneBlockAllowedCueIds(duplicateChildEventAuthority, null),
  null,
  "duplicate child Timeline event identities close the full source set instead of selecting a partial graph",
);
const cyclicChildReferenceAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 3, label: "Bank Three", active_cue_id: null }],
  [
    { ...baseCue, id: 310, cue_list_id: 3, child_timeline: { layers: [], events: [{ id: 1, cue_id: 311 }] } },
    { ...baseCue, id: 311, cue_list_id: 3, child_timeline: { layers: [], events: [{ id: 1, cue_id: 310 }] } },
  ],
  [],
);
assert.equal(
  helpers.timelineSceneBlockAllowedCueIds(cyclicChildReferenceAuthority, null),
  null,
  "a cyclic nested Super Scene graph closes Timeline source admission rather than pruning a subset",
);
const childTimelineOptions = helpers.buildTimelineSceneBlockCueOptions(
  cues.filter((cue) => childAllowedSceneCueIds?.has(cue.id)),
);
const childTimelineBanks = helpers.groupTimelineSourceShelfBanks(
  childTimelineOptions,
  sourceAuthority,
  childTimelineCueId,
);
assert.equal(
  helpers.timelineSourceShelfCueOptionsMatchAuthority(
    childTimelineOptions,
    sourceAuthority,
  childTimelineCueId,
  ),
  true,
  "a child Timeline accepts exactly its graph-safe Scene set",
);
assert.equal(
  helpers.timelineSourceShelfBankViewsEqual(
    childTimelineBanks,
    helpers.groupTimelineSourceShelfBanks(childTimelineOptions, sourceAuthority, childTimelineCueId),
  ),
  true,
  "the exact child Timeline Scene set remains idempotent across polls",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(
    childTimelineOptions.filter((option) => option.id !== 702),
    sourceAuthority,
    childTimelineCueId,
  ),
  [],
  "a missing allowed child Timeline Scene closes the complete keyed Shelf",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(options, sourceAuthority, childTimelineCueId),
  [],
  "an owner Scene is an extra child Timeline source and closes the Shelf",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(
    [...childTimelineOptions, childTimelineOptions[0]],
    sourceAuthority,
    childTimelineCueId,
  ),
  [],
  "a duplicate child Timeline Scene closes the complete keyed Shelf",
);
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks([...options, options[0]], sourceAuthority),
  [],
  "a duplicated transformed Scene option closes the complete keyed Shelf",
);
assert.deepEqual(banks.map((bank) => ({
  key: bank.key,
  id: bank.cue_list_id,
  label: bank.label,
  scenes: bank.scenes.map((scene) => scene.id),
})), [
  { key: "bank:9", id: 9, label: "Bank Nine", scenes: [901] },
  { key: "bank:2", id: 2, label: "Bank Two", scenes: [201] },
  { key: "bank:7", id: 7, label: "Bank Seven", scenes: [701, 702] },
  { key: "bank:4", id: 4, label: "Empty Bank", scenes: [] },
], "Bank order/name are cueLists authority while Scene order remains authored input order");

const renamedAndReorderedAuthority = bankAuthority.inspectBankAuthority([
  { id: 7, label: "Seven Renamed" },
  { id: 4, label: "Empty Bank" },
  { id: 9, label: "Bank Nine" },
  { id: 2, label: "Bank Two" },
], cues, []);
const renamedAndReorderedBanks = helpers.groupTimelineSourceShelfBanks(options, renamedAndReorderedAuthority);
assert.deepEqual(
  renamedAndReorderedBanks.map((bank) => ({ id: bank.cue_list_id, label: bank.label, scenes: bank.scenes.map((scene) => scene.id) })),
  [
    { id: 7, label: "Seven Renamed", scenes: [701, 702] },
    { id: 4, label: "Empty Bank", scenes: [] },
    { id: 9, label: "Bank Nine", scenes: [901] },
    { id: 2, label: "Bank Two", scenes: [201] },
  ],
  "the App authority reacts to a Bank rename/order change without fabricating a fallback",
);

const orphanOptions = [
  { id: 501, cue_list_id: 5 },
  { id: 901, cue_list_id: 9 },
  { id: 301, cue_list_id: 3 },
  { id: 502, cue_list_id: 5 },
];
const orphanAuthority = bankAuthority.inspectBankAuthority(
  [{ id: 9, label: "Bank Nine" }],
  orphanOptions,
  [],
);
assert.equal(orphanAuthority.issue?.kind, "missing_cue_bank");
assert.deepEqual(
  helpers.groupTimelineSourceShelfBanks(orphanOptions, orphanAuthority),
  [],
  "one missing Cue-to-Bank reference closes the complete Shelf; no partial unavailable group survives",
);

const allEmptyAuthority = bankAuthority.inspectBankAuthority([
  { id: 8, label: "Eight Empty" },
  { id: 6, label: "Six Empty" },
], [], []);
const allEmptyBanks = helpers.groupTimelineSourceShelfBanks([], allEmptyAuthority);
assert.deepEqual(
  allEmptyBanks.map((bank) => ({ id: bank.cue_list_id, label: bank.label, scenes: bank.scenes })),
  [
    { id: 8, label: "Eight Empty", scenes: [] },
    { id: 6, label: "Six Empty", scenes: [] },
  ],
  "an all-empty authoritative Bank list remains visible in exact order",
);

const duplicateAuthority = helpers.groupTimelineSourceShelfBanks(
  [{ id: 707, cue_list_id: 7 }],
  bankAuthority.inspectBankAuthority(
    [{ id: 7, label: "Seven A" }, { id: 7, label: "Seven B" }],
    [{ id: 707, cue_list_id: 7 }],
    [],
  ),
);
assert.deepEqual(duplicateAuthority, [], "duplicate Bank identities close the complete Shelf");

const invalidAuthority = helpers.groupTimelineSourceShelfBanks(
  [{ id: 1, cue_list_id: 0 }],
  bankAuthority.inspectBankAuthority(
    [{ id: 0, label: "Invalid" }],
    [{ id: 1, cue_list_id: 0 }],
    [],
  ),
);
assert.deepEqual(invalidAuthority, [], "an invalid Bank identity closes the complete Shelf");

const stableBanksAgain = helpers.groupTimelineSourceShelfBanks(options, sourceAuthority);
assert.equal(helpers.timelineSourceShelfBankViewsEqual(banks, stableBanksAgain), true);
assert.equal(helpers.timelineSourceShelfBankViewsEqual(banks, renamedAndReorderedBanks), false);

for (const cue of [
  { child_timeline: null, effect_targets: [] },
  { child_timeline: null, effect_targets: [{ effect_id: 1 }] },
  { child_timeline: { layers: [], events: [] }, effect_targets: [] },
  { child_timeline: { layers: [], events: [] }, effect_targets: [{ effect_id: 1 }] },
]) {
  assert.equal(
    helpers.timelineSceneBlockCueKind(cue),
    canonicalKinds.sceneCueKind(cue),
    "isolated Timeline kind rule must stay identical to canonical Lighting kind",
  );
}

assert.equal(helpers.timelineSourceShelfBankIdentity(7), "bank:7");
assert.equal(
  identity.groupIdentityCss(helpers.timelineSourceShelfBankIdentity(7), undefined, "fill"),
  identity.groupIdentityCss("bank:7", undefined, "fill"),
  "Timeline Bank colour uses the same deterministic identity as Scene Matrix",
);
assert.match(matrixSource, /const bankIdentity = `bank:\$\{column\.cueListId\}`;/);
assert.match(shelfSource, /timelineSourceShelfBankIdentity\(bank\.cue_list_id\)/);
assert.match(cssSource, /\.timelineExternalSourceShelfBankScenes \.timelineExternalSourceCard\.lighting\s*\{\s*border-left: 3px solid var\(--cue-identity\);/);

assert.deepEqual(drag.timelineExternalDragPayloadForScene(901), {
  schema: 1,
  kind: "scene",
  cue_id: 901,
  lane_kind: "Lighting",
}, "the Shelf emits the strict existing Timeline placement payload");
assert.equal(drag.timelineExternalDragPayloadForScene(0), null);
assert.match(shelfSource, /bankAuthority: FullBankAuthoritySnapshot;/);
assert.match(shelfSource, /timelineChildCueId: number \| null;/);
assert.doesNotMatch(shelfSource, /cueLists\??:/);
assert.match(shelfSource, /uiLocale: UiLocale;/);
assert.doesNotMatch(shelfSource, /loadUiLocale/);
assert.match(shelfSource, /data-timeline-source-shelf-scene-placeable="true"/);
assert.match(shelfSource, /resolveTimelineExternalLayer/);
assert.match(
  shelfSource,
  /const sourceShelfLayerResolution = \(kind: TimelineLayerKind\) =>[\s\S]*?resolveTimelineExternalLayer\(/,
  "the source shelf derives click targets from the current unlocked lane set",
);
assert.match(
  shelfSource,
  /const renderTargetSelect = \(kind: TimelineLayerKind, label: string\) => \([\s\S]*?candidates\.length > 1/,
  "the source shelf exposes a selector only for ambiguous multi-lane clicks",
);
assert.match(
  shelfSource,
  /data-timeline-source-no-target-kind=\{kind\}/,
  "the source shelf visibly reports zero unlocked lanes",
);
assert.doesNotMatch(shelfSource, /bank_unavailable|BankUnavailableReason|unavailableBank/);
assert.doesNotMatch(timelineSource, /bank_unavailable|BankUnavailableReason|bank-unavailable:/);
assert.doesNotMatch(
  shelfSource.match(/export interface TimelineSourceShelfProps \{[\s\S]*?\n\}/)?.[0] ?? "",
  /\bon(?:Create|Rename|Remove|Play|Pause|Trigger)[A-Za-z]*\??\s*:/,
  "the source shelf exposes placement only, not Bank/Scene CRUD or Timeline play",
);
assert.match(shelfSource, /onPlace:/, "the source shelf has one placement callback");
assert.equal(
  shelfSource.match(/timelineExternalDragPayloadForScene\(cue\.id\)/g)?.length,
  1,
  "click and drag share one validated placement payload rather than rebuilding parallel payloads",
);

const assertCueListPollingContract = (source) => {
  assert.match(
    source,
    /bankAuthority: FullBankAuthoritySnapshot;/,
    "Cue List rendering must receive the complete App authority",
  );
  assert.match(source, /const bankAuthority = \(\) => props\.bankAuthority;/);
  assert.doesNotMatch(source, /inspectBankAuthority/, "components must not recompute a partial authority");
  assert.match(
    source,
    /const \[stableCueLists, setStableCueLists\] = createStore<CueListSummary\[\]>\([\s\S]*?cueListSnapshot\(\) \?\? \[\],[\s\S]*?\);/,
    "Cue List rows must have a stable keyed store rather than a remapped display alias",
  );
  assert.match(
    source,
    /setStableCueLists\(reconcile\(snapshot \?\? \[\], \{ key: "id" \}\)\);/,
    "Cue List polling must reconcile rows by persisted id",
  );
  assert.equal(
    source.match(/<For each=\{stableCueLists\}>/g)?.length,
    3,
    "every Cue List option/executor surface must consume the stable keyed rows",
  );
  assert.doesNotMatch(source, /displayCueLists/, "a remapped Cue List display alias can hide authoritative identity drift");
  assert.doesNotMatch(source, /<For each=\{props\.cueLists\}>/, "raw polled Cue List objects must not key native rows");
  assert.match(
    source,
    /bankAuthority\(\)\.issue === null \? \[\.\.\.bankAuthority\(\)\.cueLists\] : null/,
    "any shared authority issue must remove every keyed Bank row",
  );
  assert.match(source, /data-bank-authority-unavailable=\{bankAuthority\(\)\.issue\?\.kind\}/);
  assert.doesNotMatch(
    source,
    /label\.trim\(\)\.toLowerCase\(\) === "main"/,
    "Cue List labels must remain the authoritative persisted labels",
  );
};

assertCueListPollingContract(cueManagementSource);
assert.doesNotMatch(matrixSource, /label\.trim\(\)\.toLowerCase\(\) === "main"/);

// Mutation-sensitive guards: these deliberate regressions must fail the same
// pure source contract, so a future cleanup cannot silently restore either the
// Main -> Bank 1 alias or identity-changing polling rows.
assert.throws(
  () => assertCueListPollingContract(cueManagementSource.replace(
    'const bankAuthority = () => props.bankAuthority;',
    'const bankAuthority = () => ({ ...props.bankAuthority, cueLists: props.bankAuthority.cueLists.map((cueList) => ({ ...cueList, label: cueList.id === 1 ? "Bank 1" : cueList.label })) });',
  )),
  /const bankAuthority|components must not recompute|authoritative persisted labels|display alias/,
  "the contract must reject a reintroduced Main/Bank 1 label alias",
);
assert.throws(
  () => assertCueListPollingContract(cueManagementSource.replace(
    '<For each={stableCueLists}>',
    '<For each={props.cueLists}>',
  )),
  /stable keyed rows|raw polled Cue List objects/,
  "the contract must reject identity-changing raw polling rows",
);
assert.match(bankAuthoritySource, /Number\.isSafeInteger\(id\) && id > 0/);
assert.match(bankAuthoritySource, /kind: "duplicate_bank_id"/);
assert.match(bankAuthoritySource, /kind: "duplicate_cue_id"/);
assert.match(bankAuthoritySource, /kind: "duplicate_executor_id"/);
assert.match(bankAuthoritySource, /kind: "invalid_bank_active_cue_id"/);
assert.match(bankAuthoritySource, /kind: "missing_bank_active_cue"/);
assert.match(bankAuthoritySource, /kind: "cross_bank_active_cue"/);
assert.match(
  shelfSource,
  /timelineSourceShelfCueOptionsMatchAuthority\(\s*props\.cueOptions,\s*bankAuthority\(\),\s*props\.timelineChildCueId,\s*\)/,
);
assert.match(
  shelfSource,
  /sceneAuthorityUnavailable\(\) === null\s*\? groupTimelineSourceShelfBanks\(props\.cueOptions, bankAuthority\(\), props\.timelineChildCueId\)/,
);
assert.match(shelfSource, /payload\.kind === "scene" && bankAuthority\(\)\.issue !== null/);
const assertPlaybackAuthorityContract = (source) => {
  assert.match(source, /bankAuthority: FullBankAuthoritySnapshot;/);
  assert.doesNotMatch(source, /inspectBankAuthority/);
  assert.match(source, /createSignal<number \| null>\(null\)/);
  assert.doesNotMatch(
    source,
    /cueListId, setCueListId\] = createSignal\(1\)/,
    "Add Fader must not preselect fabricated Bank ID 1",
  );
  assert.match(source, /selectedCreateCueList\(\) === null/);
  assert.match(source, /props\.onCreate\(label\(\), cueList\.id, page\(\), slot\)/);
  assert.match(source, /data-bank-authority-unavailable=\{bankAuthority\(\)\.issue\?\.kind\}/);
};
assertPlaybackAuthorityContract(playbackExecutorSource);
assert.throws(
  () => assertPlaybackAuthorityContract(playbackExecutorSource.replace(
    "props.onCreate(label(), cueList.id, page(), slot)",
    "props.onCreate(label(), cueListId() ?? 1, page(), slot)",
  )),
  /props\.onCreate/,
  "the Add Fader contract must reject an ID-1 fallback mutation",
);

for (const [surface, source] of [
  ["Cue management", cueManagementSource],
  ["Playback executor", playbackExecutorSource],
  ["Timeline source shelf", shelfSource],
  ["Scene Matrix", matrixSource],
  ["Timeline inner editor", timelineEditorSource],
  ["Touch Cue pads", touchCueSource],
]) {
  assert.match(source, /bankAuthority: FullBankAuthoritySnapshot;/, `${surface} consumes the App full authority`);
  assert.doesNotMatch(source, /inspectBankAuthority/, `${surface} must not create a partial authority`);
}
assert.match(timelinePanelSource, /bankAuthority: FullBankAuthoritySnapshot;/);
assert.match(timelinePanelSource, /bankAuthority=\{props\.bankAuthority\}/);
assert.match(timelinePanelSource, /const timelineAuthorityAvailable = \(\) => props\.bankAuthority\.issue === null;/);
assert.match(timelinePanelSource, /timelineChildCueId: number \| null;/);
assert.match(timelinePanelSource, /timelineSceneBlockCueAllowedByAuthority\(\s*cueId,\s*props\.bankAuthority,\s*props\.timelineChildCueId,\s*\)/);
assert.match(timelinePanelSource, /<TimelineOverview[\s\S]*?timelineChildCueId=\{props\.timelineChildCueId\}/);
assert.match(timelinePanelSource, /if \(props\.armedCueId === null\) return;[\s\S]*?props\.onArmCue\(null\);/);
assert.match(timelineEditorSource, /when=\{props\.bankAuthority\.issue === null\}/);
assert.match(touchCueSource, /when=\{props\.bankAuthority\.issue === null\}/);
assert.match(editableTouchSource, /bankAuthority: FullBankAuthoritySnapshot;/);
assert.doesNotMatch(editableTouchSource, /snapshot\.cues\[0\]\?\.id \?\? 1/);
assert.match(editableTouchSource, /props\.bankAuthority\.cueById\.has\(cueId\)/);
assert.match(editableTouchSource, /data-touch-binding-invalid/);
assert.match(editableTouchSource, /cueBindingPending/);
assert.match(appSource, /<EditableTouchSurface\s+bankAuthority=\{bankAuthority\(\)\}/);
assert.match(
  appSource,
  /interface Window\s*\{[\s\S]*?__syndocalReadSceneMatrixFixtureSnapshot\?: \(\) => EngineSnapshot;/,
  "the Scene Matrix harness may observe only the explicitly typed test fixture seam",
);
assert.match(
  appSource,
  /if \(viewportFixture === "scene-matrix"\) \{\s*let bankAuthorityFaultCueBackup: CueSummary\[\] \| null = null;\s*sceneBlockFixtureWindow\.__syndocalReadSceneMatrixFixtureSnapshot = \(\) => structuredClone\(snapshot\(\)\);/,
  "the Scene Matrix snapshot hook is fixture-only and returns a clone",
);
assert.match(
  appSource,
  /delete sceneBlockFixtureWindow\.__syndocalReadSceneMatrixFixtureSnapshot;/,
  "fixture teardown removes the Scene Matrix snapshot hook",
);
assert.match(appSource, /const requireTimelineSceneBlockCue = \(cueId: number\): CueSummary \| null =>[\s\S]*?timelineSceneBlockCueAllowedByAuthority\(cueId, authority, childCueId\)/);
assert.match(appSource, /const placeArmedTimelineCue = async[\s\S]*?const cue = requireTimelineSceneBlockCue\(cueId\);/);
assert.match(appSource, /const endTimelineCueDrag = async[\s\S]*?const cue = requireTimelineSceneBlockCue\(drag\.cue_id\);/);
assert.match(
  timelineCommandDispatchersSource,
  /export const invokeTimelineSceneBlockCommand = async[\s\S]*?command === "add_timeline_scene_block"[\s\S]*?command === "set_timeline_scene_block"[\s\S]*?command === "set_timeline_cue_event"[\s\S]*?timelineSceneBlockCueAllowed\(cueId, childCueId\)/,
  "all Scene add/set terminal commands reject unallowed child Timeline cues before persistence or Tauri IPC",
);
assert.match(
  appSource,
  /const localFixtureBeforeCues = viewportFixture === "scene-matrix" \|\| viewportFixture === "workspace-operator"[\s\S]*?\? structuredClone\(cues\)[\s\S]*?await runSceneMatrixBankMoveTransaction\(/,
  "the Scene Matrix fixture captures immutable Undo state before terminal authority may publish",
);
assert.match(
  appSource,
  /const beforeCues = localFixtureBeforeCues!;[\s\S]*?applySceneMatrixCueListMove\(\s*beforeCues,[\s\S]*?applySceneMatrixCueMove\(\s*beforeCues,/,
  "fixture local after state and Undo must both derive from the same pre-transaction image",
);
assert.doesNotMatch(timelineSource, /cue\?\.cue_list_id \?\? 0/);
assert.match(matrixSource, /props\.bankAuthority\.issue\s*\?\s*\[\]\s*:\s*props\.bankAuthority\.cueLists/);
assert.match(appSource, /<Show\s+when=\{bankAuthority\(\)\.issue === null\}[\s\S]*?<div class="liveCuePadSurface">/);
assert.doesNotMatch(
  `${shelfSource}\n${timelineSource}`,
  /bank_unavailable|bank_unavailable_reason|BankUnavailableReason|bank-unavailable:/,
  "the retired partial unavailable grouping path and its affirmative representation are absent",
);

const assertAppAuthorityGuards = (source, commandDispatchersSource) => {
  assert.match(source, /snapshot\(\)\.cue_lists,\s*snapshot\(\)\.cues,\s*snapshot\(\)\.playback_executors/);
  assert.match(source, /const requireBankAuthority = \(\) =>/);
  assert.match(source, /const createCueList = async[\s\S]*?if \(!requireBankAuthority\(\)\) return false;/);
  assert.match(source, /const reorderCueLists = async[\s\S]*?const authority = requireBankAuthority\(\);/);
  assert.match(source, /const setCueList = async[\s\S]*?const cue = requireAuthoritativeCue\(cueId\);/);
  assert.match(source, /const createPlaybackExecutor = async[\s\S]*?if \(!requireAuthoritativeCueList\(cueListId\)\) return;/);
  assert.match(source, /const updatePlaybackExecutor = async[\s\S]*?const authority = requireBankAuthority\(\);/);
  assert.match(source, /const triggerPlaybackExecutor = async[\s\S]*?const authority = requireBankAuthority\(\);/);
  assert.match(commandDispatchersSource, /export const invokeTimelineSceneBlockCommand = async[\s\S]*?if \(!options\.requireBankAuthority\(\)\) throw new Error\(options\.bankAuthorityIssueMessage\(\)\);/);
  assert.match(source, /selectedCueListId, setSelectedCueListId\] = createSignal<number \| null>\(null\)/);
  assert.doesNotMatch(source, /selectedCueListId=\{selectedCueList\(\)\?\.id \?\? [01]\}/);
  assert.match(source, /requestedId === null && authority\.issue === null/);
  assert.match(source, /createBankAuthorityDelayFence/);
  assert.match(source, /controlEditBankAuthorityFence\.isCurrent/);
};
assertAppAuthorityGuards(appSource, timelineCommandDispatchersSource);
assert.throws(
  () => assertAppAuthorityGuards(appSource.replace(
    "if (!requireAuthoritativeCueList(cueListId)) return;\n    try {\n      const executorId",
    "try {\n      const executorId",
  ), timelineCommandDispatchersSource),
  /createPlaybackExecutor/,
  "the App contract must reject a component-bypass create-executor regression",
);
assert.doesNotMatch(appSource, /label\.trim\(\)\.toLowerCase\(\) === "main"/);
assert.match(packageSource, /"check:timeline-source-shelf":\s*"node scripts\/check-timeline-source-shelf-contract\.mjs && node scripts\/check-viewport-containment\.mjs --timeline-source-placement-only"/);

assert.match(
  appSource,
  /const timelineCueOptions = createMemo\([\s\S]*?const authority = bankAuthority\(\);[\s\S]*?const allowedCueIds = timelineSceneBlockAllowedCueIds\(authority, timelineChildCueId\(\)\);[\s\S]*?if \(allowedCueIds === null\) return \[\];[\s\S]*?authority\.cues\.filter\(\(cue\) => allowedCueIds\.has\(cue\.id\)\)/,
  "child Timeline input is sourced only from complete Bank/Cue graph authority and excludes exactly owner-reaching paths",
);
assert.match(
  appSource,
  /const placeTimelineExternalSource = async[\s\S]*?source\.kind === "scene" && !requireTimelineSceneBlockCue\(source\.cue_id\)[\s\S]*?const allowedSceneCueIds = timelineSceneBlockAllowedCueIds\([\s\S]*?bankAuthority\(\),[\s\S]*?timelineChildCueId\(\),[\s\S]*?\) \?\? new Set<number>\(\);[\s\S]*?await executeTimelineExternalDrop\([\s\S]*?allowedSceneCueIds,/,
  "forged external Scene MIME is reauthorized and receives only the exact child-safe Scene set before the shared runtime can call a placement callback",
);
assert.match(timelineOverviewSource, /timelineChildCueId: number \| null;/);
assert.match(
  timelineOverviewSource,
  /timelineSceneBlockCueAllowedByAuthority\(\s*source\.cue_id,\s*props\.bankAuthority,\s*props\.timelineChildCueId,\s*\)/,
  "TimelineOverview gesture preflight uses the typed child owner rather than a label-derived context",
);
assert.match(
  appSource,
  /<TimelineSourceShelf\s+bankAuthority=\{bankAuthority\(\)\}\s+cueOptions=\{timelineCueOptions\(\)\}\s+timelineChildCueId=\{timelineChildCueId\(\)\}/,
  "the Timeline Shelf receives the explicit root-or-child context used to validate its complete expected Scene set",
);

// Cross-surface selection contract: the lower Timeline Inspector must consume
// one App-owned tagged primary item, not a stale Lighting-only local signal.
assert.match(
  appSource,
  /const \[timelineSelection, setTimelineSelection\] = createSignal<TimelineItemRef \| null>\(null\)/,
  "the App owns a nullable tagged Timeline selection",
);
for (const kind of ["lighting_event", "audio_clip", "video_clip", "lighting_automation", "video_automation"]) {
  assert.match(appSource, new RegExp(`case "${kind}":`), `the Inspector handles the ${kind} selection kind`);
}
assert.match(
  appSource,
  /const timelineSelectionIsCurrent = \(selection: TimelineItemRef\) => \{[\s\S]*?const selectTimelineItem = \(selection: TimelineItemRef \| null\) => \{[\s\S]*?if \(selection !== null && !timelineSelectionIsCurrent\(selection\)\)/,
  "selection is checked against the current authoritative Timeline before it reaches the Inspector",
);
assert.match(
  appSource,
  /<TimelineSourceShelf[\s\S]*?onOpenInspector=\{\(\) => setTimelineLowerContextMode\("inspector"\)\}[\s\S]*?inspectorContent=\{renderTimelineInspector\(\)\}/,
  "the mounted source shelf opens the App-owned Inspector rather than a no-op callback",
);
assert.doesNotMatch(appSource, /onOpenInspector=\{\(\) => undefined\}/);
assert.match(appSource, /data-timeline-selection-kind=\{selection\.kind\}/);
assert.match(appSource, /Selected Timeline item is no longer available\./);
assert.match(timelinePanelSource, /timelineSelection: TimelineItemRef \| null;/);
assert.match(timelinePanelSource, /onTimelineSelectionChange: \(selection: TimelineItemRef \| null\) => void;/);
assert.match(timelinePanelSource, /let previousTimelineSelection: TimelineItemRef \| null \| undefined;/);
assert.match(
  timelinePanelSource,
  /const timelineSelection = props\.timelineSelection;[\s\S]*?const primarySelectionCleared = previousTimelineSelection !== undefined[\s\S]*?previousTimelineSelection = timelineSelection;[\s\S]*?if \(primarySelectionCleared &&/,
  "local Timeline selection is cleared only after the App-owned primary transitions from non-null to null",
);
assert.match(
  timelinePanelSource,
  /const notifyTimelineSelection = \(selection: TimelineItemRef \| null\) => \{\s*props\.onTimelineSelectionChange\(selection\);\s*\}/,
  "the Timeline panel publishes its primary item to the App selection owner",
);
assert.match(timelinePanelSource, /onSelectAutomationRange=\{selectAutomationRange\}/);
assert.match(
  timelinePanelSource,
  /onSelectEvent=\{\(eventId, openProperties = false\) => \{\s*selectTimelineEvent\(eventId, false\);/,
  "Lighting event selection also updates the shared tagged selection",
);
assert.match(
  shelfSource,
  /const selectSourceShelfTab = \(tab: "Scenes" \| "Media Library"\) => \{[\s\S]*?if \(sourceContextMode\(\) !== "sources"\) selectSourceContextMode\("sources"\);/,
  "choosing Scenes or Media Library from the compact header returns to Sources",
);
assert.match(shelfSource, /onClick=\{\(\) => selectSourceShelfTab\("Scenes"\)\}/);
assert.match(shelfSource, /onClick=\{\(\) => selectSourceShelfTab\("Media Library"\)\}/);

console.log("timeline source shelf Bank/Scene contract ok");
