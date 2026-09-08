import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/timelineAdvancedAuthoring.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "timelineAdvancedAuthoring.ts",
}).outputText;
const { expandTimelineItemGroupSelection, timelineItemKey } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);
const splitSource = await readFile(new URL("../src/timelineSplitAction.ts", import.meta.url), "utf8");
const splitOutput = ts.transpileModule(splitSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "timelineSplitAction.ts",
}).outputText;
const { applyTimelineSplitAtPlayhead } = await import(`data:text/javascript;base64,${Buffer.from(splitOutput).toString("base64")}`);
const resultSource = await readFile(new URL("../src/timelineAdvancedResult.ts", import.meta.url), "utf8");
const resultOutput = ts.transpileModule(resultSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "timelineAdvancedResult.ts",
}).outputText;
const { engineSnapshotWithTimelineAdvancedResult, specializedTimelineSelectionFromItems } = await import(`data:text/javascript;base64,${Buffer.from(resultOutput).toString("base64")}`);
const typesSource = await readFile(new URL("../src/types.ts", import.meta.url), "utf8");
const typesOutput = ts.transpileModule(typesSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "types.ts",
}).outputText;
const { planTimelineItemLaneMove } = await import(`data:text/javascript;base64,${Buffer.from(typesOutput).toString("base64")}`);

const authoring = {
  video_clips: [{ id: 11, layer_id: 1, media_asset_id: 9, start_ms: 100, offset_ms: 0, duration_ms: 1_000, fade_in_ms: 0, fade_out_ms: 0 }],
  audio_clips: [
    { id: 21, layer_id: 2, media_asset_id: 9, path: "a.wav", start_ms: 100, offset_ms: 0, duration_ms: 1_000, gain: 1, fade_in_ms: 0, fade_out_ms: 0 },
    { id: 22, layer_id: 2, path: "b.wav", start_ms: 2_000, offset_ms: 0, duration_ms: 500, gain: 1, fade_in_ms: 0, fade_out_ms: 0 },
    { id: 23, layer_id: 2, path: "c.wav", start_ms: 3_000, offset_ms: 0, duration_ms: 500, gain: 1, fade_in_ms: 0, fade_out_ms: 0 },
  ],
  phases: [],
  item_groups: [
    { id: 31, members: [{ kind: "video_clip", clip_id: 11 }, { kind: "audio_clip", clip_id: 21 }] },
    { id: 32, members: [{ kind: "lighting_event", event_id: 41 }, { kind: "audio_clip", clip_id: 22 }] },
  ],
  loop_region: null,
  follow: null,
  guide_enabled: false,
};

const linkedAv = expandTimelineItemGroupSelection(
  { kind: "audio_clip", clip_id: 21 },
  authoring.item_groups,
);
assert.deepEqual(linkedAv.map(timelineItemKey), ["video:11", "audio:21"], "either A/V member expands to the complete authored group");
const additive = expandTimelineItemGroupSelection(
  { kind: "audio_clip", clip_id: 22 },
  authoring.item_groups,
  linkedAv,
);
assert.deepEqual(
  additive.map(timelineItemKey),
  ["video:11", "audio:21", "event:41", "audio:22"],
  "additive selection preserves prior members and expands a mixed-domain group without duplicates",
);
const ungrouped = expandTimelineItemGroupSelection(
  { kind: "audio_clip", clip_id: 23 },
  authoring.item_groups,
);
assert.deepEqual(ungrouped.map(timelineItemKey), ["audio:23"]);

const splitCalls = [];
const restoredSelections = [];
const restoredFocus = [];
const splitItems = linkedAv.map((item) => structuredClone(item));
const rightSelection = [
  { kind: "video_clip", clip_id: 111 },
  { kind: "audio_clip", clip_id: 121 },
];
const splitResult = await applyTimelineSplitAtPlayhead(
  async (items, primary, boundaryMs, isolate) => {
    splitCalls.push({ items: structuredClone(items), primary: structuredClone(primary), boundary_ms: boundaryMs, isolate });
    return rightSelection.map((item) => structuredClone(item));
  },
  (items) => restoredSelections.push(items.map((item) => structuredClone(item))),
  (item) => restoredFocus.push(structuredClone(item)),
  {
    items: splitItems,
    primary: { kind: "video_clip", clip_id: 11 },
    boundary_ms: 500.6,
    isolate: true,
  },
);
assert.deepEqual(splitCalls, [{
  items: splitItems,
  primary: { kind: "video_clip", clip_id: 11 },
  boundary_ms: 501,
  isolate: true,
}], "Split action sends the exact captured group, primary, rounded playhead, and Alt isolation flag");
assert.deepEqual(splitResult, rightSelection, "Split action returns the fresh right-side logical selection");
assert.deepEqual(restoredSelections, [rightSelection], "Split action restores the authoritative returned selection exactly once");
assert.deepEqual(restoredFocus, [rightSelection[0]], "Split action restores focus to the first fresh right-side item exactly once");
const loopRegion = { a_ms: 136_941, b_ms: 138_353, enabled: true, musical_length_beats: 4 };
const runtimeFreeAcknowledgedTimeline = {
  id: 2,
  video_clips: [{ id: 111 }],
  audio_clips: [{ id: 121 }],
  loop_region: loopRegion,
  loop_runtime: { generation: 0, status: "disabled", wrap_count: 0 },
  playing: false,
  position_ms: 0,
  duration_ms: 200_000,
};
const canonicalLoopingTimeline = {
  ...runtimeFreeAcknowledgedTimeline,
  playing: true,
  position_ms: 137_500,
  loop_runtime: {
    generation: 9,
    status: "looping",
    a_ms: loopRegion.a_ms,
    b_ms: loopRegion.b_ms,
    musical_length_millibeats: 4_000,
    wrap_count: 3,
  },
};
const timelineResult = {
  timeline_bank: [runtimeFreeAcknowledgedTimeline],
  active_timeline_id: 2,
  selected_items: rightSelection,
  authoring,
  mutation: {},
};
const appliedTimelineResult = engineSnapshotWithTimelineAdvancedResult({
  active_cue_id: 77,
  timeline: canonicalLoopingTimeline,
  timeline_bank: [{ id: 1 }],
}, timelineResult);
assert.equal(appliedTimelineResult.active_cue_id, 77, "post-ACK Timeline fallback preserves unrelated live engine state");
assert.equal(appliedTimelineResult.timeline.id, 2, "same-identity fallback keeps the acknowledged active Timeline mounted");
assert.deepEqual(
  [appliedTimelineResult.timeline.video_clips[0].id, appliedTimelineResult.timeline.audio_clips[0].id],
  [111, 121],
  "same-identity fallback retains fresh split IDs",
);
assert.equal(appliedTimelineResult.timeline.loop_region.enabled, true, "same-identity fallback retains an authored enabled loop region");
assert.equal(appliedTimelineResult.timeline.loop_runtime.status, "looping", "same-identity fallback never replaces live loop runtime with disabled bank state");
assert.equal(appliedTimelineResult.timeline.playing, true, "same-identity fallback retains canonical playback state");
const mismatchedTimelineResult = engineSnapshotWithTimelineAdvancedResult({
  active_cue_id: 77,
  timeline: { ...canonicalLoopingTimeline, id: 1 },
  timeline_bank: [{ id: 1 }],
}, timelineResult);
assert.equal(mismatchedTimelineResult.timeline.id, 1, "different-identity fallback keeps the prior active Timeline pending canonical refresh");
assert.equal(mismatchedTimelineResult.timeline.loop_runtime.status, "looping", "different-identity fallback never exposes the target bank's runtime-free OFF state");
assert.equal(mismatchedTimelineResult.timeline_bank[0].id, 2, "different-identity fallback still exposes the committed bank for later canonical hydration");
const appSource = (await readFile(new URL("../src/App.tsx", import.meta.url), "utf8"))
  .replace(/\r\n?/g, "\n");
const timelineCommandDispatchersSource = (await readFile(new URL("../src/timelineCommandDispatchers.ts", import.meta.url), "utf8"))
  .replace(/\r\n?/g, "\n");
const timelineSnapshotRefreshSource = await readFile(new URL("../src/timelineSnapshotRefreshController.ts", import.meta.url), "utf8");
assert.match(
  appSource,
  /import \{\s*invokeTimelineEditingCommand as dispatchTimelineEditingCommand,\s*invokeTimelineLayerCommand as dispatchTimelineLayerCommand,\s*invokeTimelineSceneBlockCommand as dispatchTimelineSceneBlockCommand,\s*\} from "\.\/timelineCommandDispatchers";/,
  "App must bind all extracted Timeline dispatchers through static aliases",
);
assert.doesNotMatch(
  appSource,
  /import\("\.\/timelineCommandDispatchers"\)/,
  "App must not defer Timeline dispatcher binding through a dynamic module import",
);
assert.match(
  appSource,
  /const timelineCommandDispatcherOptions = \(\) => \(\{[\s\S]*?viewportFixture: \(\) => viewportFixture,[\s\S]*?timelineAuthorityReady,[\s\S]*?timelineChildCueId,[\s\S]*?requireBankAuthority,/,
  "extracted Timeline dispatchers must receive state-bearing options from an invocation-time factory",
);
assert.match(
  appSource,
  /dispatchTimelineEditingCommand<T>\(timelineCommandDispatcherOptions\(\), command, args\)/,
  "Timeline editing must evaluate dispatcher options at invocation time",
);
assert.match(
  appSource,
  /dispatchTimelineSceneBlockCommand<T>\(timelineCommandDispatcherOptions\(\), command, args\)/,
  "Timeline Scene Block dispatch must evaluate dispatcher options at invocation time",
);
assert.match(
  appSource,
  /dispatchTimelineLayerCommand<T>\(timelineCommandDispatcherOptions\(\), command, args\)/,
  "Timeline layer dispatch must evaluate dispatcher options at invocation time",
);
const commitStart = appSource.indexOf("const commitTimelineAdvanced = async");
const commitEnd = appSource.indexOf("const currentTimelineAdvancedAuthoring", commitStart);
assert.ok(commitStart >= 0 && commitEnd > commitStart, "Timeline commit source boundary is present");
const commitSource = appSource.slice(commitStart, commitEnd);
assert.match(commitSource, /timelineAuthorityReady\("Timeline edit"\)/, "Timeline edits are fenced while the canonical snapshot is pending or blocked");
assert.match(commitSource, /setTimelineAdvancedSnapshotResolution\("pending"\)/, "Timeline edits enter an explicit pending resolution state");
assert.match(commitSource, /let terminalTimelineAcknowledgementEstablished = false/, "Timeline ACK lifecycle starts with no terminal acknowledgement expectation");
assert.match(commitSource, /applyProjectHistoryMutationResult\(result\.mutation\)/, "Timeline ACK applies only its authority-bound mutation receipt");
assert.match(commitSource, /acknowledgedAuthority/, "Timeline ACK captures its authority token before the canonical read");
assert.match(commitSource, /timelineAdvancedSnapshotExpectation = \{[\s\S]*?authority: acknowledgedAuthority,[\s\S]*?activeTimelineId: result\.active_timeline_id/, "blocked Timeline state retains its exact ACK authority and active Timeline identity");
assert.match(commitSource, /timelineAdvancedSnapshotExpectation = \{[\s\S]*?\};[\s\S]*?terminalTimelineAcknowledgementEstablished = true/, "only a terminal Timeline acknowledgement establishes a reconciliation expectation");
assert.match(commitSource, /applyProjectHistoryMutationResult\(result\.mutation\)[\s\S]*?timelineAdvancedSnapshotExpectation = \{[\s\S]*?projectReadGeneration,[\s\S]*?expectedCanonical\.projectReadGeneration !== projectReadGeneration/, "the post-receipt project-read generation is retained and rechecked before ACK convergence");
assert.match(commitSource, /tauriInvoke<ProjectAuthorityBundle>\("get_project_authority_bundle", \{[\s\S]*?expectedEpoch: expectedCanonical\.authority\.project_epoch,[\s\S]*?expectedRevision: expectedCanonical\.authority\.project_revision,[\s\S]*?expectedCheckpointHash: expectedCanonical\.authority\.checkpoint_hash/, "Timeline ACK reads a snapshot only through the exact expected authority bundle");
assert.match(commitSource, /timelineAdvancedCanonicalBundleMatchesExpectation\(canonical, expectedCanonical\)/, "Timeline ACK requires both exact authority and the acknowledged active Timeline");
assert.doesNotMatch(commitSource, /await refreshSnapshot\(\)/, "Timeline ACK never accepts a standalone snapshot after the ACK");
assert.match(commitSource, /blockTimelineAdvancedSnapshotResolution/, "Timeline convergence failure remains blocked");
assert.doesNotMatch(commitSource, /engineSnapshotWithTimelineAdvancedResult/, "Timeline ACK does not apply a runtime-free bank entry as active runtime");
assert.match(commitSource, /unverified result was not applied/, "Timeline refresh failure is explicit and fail-closed");
assert.doesNotMatch(commitSource, /Refresh and retry/, "Timeline convergence failure does not invite a non-idempotent retry");
assert.match(commitSource, /if \(terminalTimelineAcknowledgementEstablished\) \{[\s\S]*?blockTimelineAdvancedSnapshotResolution[\s\S]*?\} else \{[\s\S]*?timelineAdvancedSnapshotExpectation = null;[\s\S]*?setTimelineAdvancedSnapshotResolution\("idle"\)[\s\S]*?no terminal acknowledgement was established/, "pre-ACK failure returns Timeline controls to idle while an ACK-established failure remains blocked");
const resolutionAfterTimelineFailure = (terminalAcknowledgementEstablished) =>
  terminalAcknowledgementEstablished ? "blocked" : "idle";
assert.equal(resolutionAfterTimelineFailure(false), "idle", "a preflight or IPC failure with no terminal ACK does not permanently block Timeline controls");
assert.equal(resolutionAfterTimelineFailure(true), "blocked", "a terminal ACK that cannot converge remains fail-closed");
const expectationStart = appSource.indexOf("type TimelineAdvancedSnapshotExpectation");
const reconcileStart = appSource.indexOf("const reconcileBlockedTimelineAdvancedSnapshotResolution");
const reconcileEnd = appSource.indexOf("// This signal is populated exclusively", reconcileStart);
assert.ok(expectationStart >= 0 && reconcileStart > expectationStart && reconcileEnd > reconcileStart, "blocked Timeline reconciliation source boundary is present");
const reconciliationSource = appSource.slice(expectationStart, reconcileEnd);
assert.match(reconciliationSource, /timelineAdvancedSnapshotExpectation: TimelineAdvancedSnapshotExpectation \| null/, "blocked state retains a typed acknowledgement expectation");
assert.match(reconciliationSource, /candidate\.snapshot\.timeline\.id === expected\.activeTimelineId/, "blocked state rejects a canonical snapshot with a mismatched active Timeline ID");
assert.match(reconciliationSource, /projectAuthorityTokenIsCurrent\(expected\.authority, authorityToken\(candidate\)\)/, "blocked state requires exact ACK authority equality, not just a stable local signal");
assert.match(reconciliationSource, /get_project_authority_bundle", \{[\s\S]*?expectedEpoch: expected\.authority\.project_epoch,[\s\S]*?expectedRevision: expected\.authority\.project_revision,[\s\S]*?expectedCheckpointHash: expected\.authority\.checkpoint_hash/, "blocked reconciliation reads an atomic E/R/H-bound authority plus snapshot bundle");
assert.doesNotMatch(reconciliationSource, /clearTimelineAdvancedSnapshotResolutionAfterCanonical/, "a generic snapshot cannot directly clear blocked Timeline state");
const fullSnapshotStart = timelineSnapshotRefreshSource.indexOf("const run = async (applySnapshot");
const fullSnapshotEnd = timelineSnapshotRefreshSource.indexOf("const refresh = (", fullSnapshotStart);
assert.ok(fullSnapshotStart >= 0 && fullSnapshotEnd > fullSnapshotStart, "generic full snapshot source boundary is present");
const fullSnapshotSource = timelineSnapshotRefreshSource.slice(fullSnapshotStart, fullSnapshotEnd);
assert.match(fullSnapshotSource, /if \(next !== null[\s\S]*?applySnapshot\([\s\S]*?requestedReadGuard,[\s\S]*?\)[\s\S]*?\)/, "generic snapshot refresh applies only after its current-read and transport fences");
assert.doesNotMatch(fullSnapshotSource, /setTimelineAdvancedSnapshotResolution\("idle"\)/, "generic snapshot refresh cannot unlock Timeline controls directly");
const fullSnapshotApplyStart = appSource.indexOf("const runFullSnapshotRefreshes = () =>");
const fullSnapshotApplyEnd = appSource.indexOf("const refreshSnapshot =", fullSnapshotApplyStart);
assert.ok(fullSnapshotApplyStart >= 0 && fullSnapshotApplyEnd > fullSnapshotApplyStart, "generic full snapshot apply source boundary is present");
const fullSnapshotApplySource = appSource.slice(fullSnapshotApplyStart, fullSnapshotApplyEnd);
assert.match(fullSnapshotApplySource, /applyEngineSnapshot\([\s\S]*?reconcileBlockedTimelineAdvancedSnapshotResolution\(\)/, "generic snapshot refresh may request reconciliation only after applying its untrusted image");
assert.doesNotMatch(fullSnapshotApplySource, /setTimelineAdvancedSnapshotResolution\("idle"\)/, "generic snapshot refresh cannot unlock Timeline controls directly");
const canonicalBundleMatchesExpected = (expected, bundle) =>
  expected.project_epoch === bundle.project_epoch
  && expected.project_revision === bundle.project_revision
  && expected.checkpoint_hash === bundle.checkpoint_hash
  && bundle.snapshot.timeline.id === expected.activeTimelineId
  && bundle.snapshot.timeline_bank.some((timeline) => timeline.id === expected.activeTimelineId);
const acknowledgedB = {
  project_epoch: 4,
  project_revision: 12,
  checkpoint_hash: "b-ack",
  activeTimelineId: 2,
};
const exactB = {
  project_epoch: 4,
  project_revision: 12,
  checkpoint_hash: "b-ack",
  snapshot: { timeline: { id: 2 }, timeline_bank: [{ id: 2 }] },
};
assert.equal(canonicalBundleMatchesExpected(acknowledgedB, exactB), true, "the exact ACK bundle may unlock blocked Timeline controls");
assert.equal(
  canonicalBundleMatchesExpected(acknowledgedB, { ...exactB, snapshot: { timeline: { id: 7 }, timeline_bank: [{ id: 2 }] } }),
  false,
  "a blocked Timeline does not unlock on a snapshot whose active Timeline mismatches the ACK",
);
assert.equal(
  canonicalBundleMatchesExpected(acknowledgedB, { ...exactB, project_revision: 13, checkpoint_hash: "c-same-active-id" }),
  false,
  "a B ACK never accepts same-active-ID C authority state",
);
const loopTransportStart = appSource.indexOf("const setTimelineLoopEnabled = async");
const loopTransportEnd = appSource.indexOf("const setTimelineLoopAAtPlayhead", loopTransportStart);
assert.ok(loopTransportStart >= 0 && loopTransportEnd > loopTransportStart, "Timeline loop transport source boundary is present");
const loopTransportSource = appSource.slice(loopTransportStart, loopTransportEnd);
assert.equal(
  (loopTransportSource.match(/timelineAuthorityReady\(/g) ?? []).length,
  2,
  "Loop toggle and resize reject stale operations while Timeline authority is pending or blocked",
);
const sourceBetween = (source, startMarker, endMarker) => {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start);
  assert.ok(start >= 0 && end > start, `${startMarker} source boundary is present`);
  return source.slice(start, end);
};
const sourceFrom = (source, startMarker) => {
  const start = source.indexOf(startMarker);
  assert.ok(start >= 0, `${startMarker} source boundary is present`);
  return source.slice(start);
};
const assertGuardPrecedes = (source, guard, protectedOperation, label) => {
  const guardAt = source.indexOf(guard);
  const operationAt = source.indexOf(protectedOperation);
  assert.ok(guardAt >= 0 && operationAt > guardAt, `${label} is rejected before its direct-child mutation can run`);
};
const childPersistSource = sourceBetween(appSource, "const persistChildTimeline = async", "const childTimelineNextAutomationId");
assertGuardPrecedes(childPersistSource, 'timelineAuthorityReady("Timeline child edit")', 'invoke("set_cue_child_timeline"', "direct-child persistence");
const childEditingSource = sourceBetween(timelineCommandDispatchersSource, "export const invokeTimelineEditingCommand", "export const invokeTimelineSceneBlockCommand");
assertGuardPrecedes(childEditingSource, "options.timelineAuthorityReady(command)", 'options.invoke<T>("seek_direct_child_timeline"', "direct-child seek");
assertGuardPrecedes(childEditingSource, "options.timelineAuthorityReady(command)", "options.persistChildTimeline(childCueId", "direct-child metronome and automation persistence");
const childSceneSource = sourceBetween(timelineCommandDispatchersSource, "export const invokeTimelineSceneBlockCommand", "export const invokeTimelineLayerCommand");
assertGuardPrecedes(childSceneSource, "options.timelineAuthorityReady(command)", "if (childCueId !== null)", "direct-child Scene block persistence");
const childLayerSource = sourceFrom(timelineCommandDispatchersSource, "export const invokeTimelineLayerCommand");
assertGuardPrecedes(childLayerSource, "options.timelineAuthorityReady(command)", "if (childCueId !== null)", "direct-child layer persistence");
const childTransportSource = sourceBetween(appSource, "const setCanonicalTimelinePlaying", "const {\n    moveTimelineAutomationRangeToTime");
assertGuardPrecedes(childTransportSource, 'timelineAuthorityReady("Timeline transport change")', 'invoke("set_direct_child_timeline_playing"', "direct-child play and pause");
const superSceneSource = sourceBetween(appSource, "const openOrCreateSuperScene", "const effectChooserCueId");
assertGuardPrecedes(superSceneSource, 'timelineAuthorityReady("Timeline child creation")', 'invoke("set_cue_child_timeline"', "direct-child Timeline creation from Super Scene");
const cuePanelSource = await readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8");
assert.ok(
  cuePanelSource.includes('const loopRuntimeEnabled = () => props.loopRuntime.status !== "disabled";'),
  "Timeline loop control treats both armed and looping runtime states as enabled",
);
assert.ok(
  cuePanelSource.includes("onClick={() => void props.onSetLoopEnabled(!loopRuntimeEnabled())}"),
  "Loop click sends false while the authored loop is armed or looping",
);
assert.deepEqual(specializedTimelineSelectionFromItems([
  { kind: "lighting_event", event_id: 141 },
  { kind: "video_automation", automation_id: 151 },
  ...rightSelection,
]), {
  event_id: 141,
  automation: { kind: "video", automationId: 151 },
}, "mixed-domain returned selection updates the Scene and Automation tab stops to the fresh right items");
assert.deepEqual(specializedTimelineSelectionFromItems(rightSelection), {
  event_id: null,
  automation: null,
}, "A/V-only returned selection clears stale Scene and Automation tab stops");

const laneLayers = [
  { id: 10, label: "Audio A", order: 0, kind: "Audio", locked: false },
  { id: 11, label: "Audio B", order: 1, kind: "Audio", locked: false },
  { id: 12, label: "Lighting A", order: 2, kind: "Lighting", locked: false },
  { id: 13, label: "Lighting B", order: 3, kind: "Lighting", locked: false },
  { id: 14, label: "Video A", order: 4, kind: "Video", locked: false },
  { id: 15, label: "Video B", order: 5, kind: "Video", locked: false },
];
const fiveDomainItems = [
  { kind: "lighting_event", event_id: 41 },
  { kind: "video_clip", clip_id: 11 },
  { kind: "audio_clip", clip_id: 21 },
  { kind: "lighting_automation", automation_id: 51 },
  { kind: "video_automation", automation_id: 61 },
];
const fiveDomainGroup = [{ id: 71, members: fiveDomainItems }];
const fiveDomainPlacements = [
  { item: fiveDomainItems[0], resolved_layer_id: 12 },
  { item: fiveDomainItems[1], resolved_layer_id: 14 },
  { item: fiveDomainItems[2], resolved_layer_id: 10 },
  { item: fiveDomainItems[3], resolved_layer_id: 12 },
  { item: fiveDomainItems[4], resolved_layer_id: 14 },
];
const targetByKind = { lighting_event: 13, video_clip: 15, audio_clip: 11, lighting_automation: 13, video_automation: 15 };
for (const primary of fiveDomainItems) {
  const planned = planTimelineItemLaneMove({
    items: [primary], primary, target_layer_id: targetByKind[primary.kind], delta_ms: 125, isolate: false,
    layers: laneLayers, groups: fiveDomainGroup, placements: fiveDomainPlacements,
  });
  assert.equal(planned.ok, true, `${primary.kind} resolves a lane-valid five-domain linked move`);
  assert.equal(planned.plan.items.length, 5, `${primary.kind} expands the complete heterogeneous group`);
  assert.deepEqual(
    Object.fromEntries(planned.plan.lane_targets.map((target) => [timelineItemKey(target.item), target.target_layer_id])),
    { "event:41": 13, "video:11": 15, "audio:21": 11, "lighting-automation:51": 13, "video-automation:61": 15 },
    `${primary.kind} applies the same-kind ordinal delta to every member`,
  );
  assert.equal(planned.plan.delta_ms, 125);
}
for (const primary of fiveDomainItems) {
  const currentLayerId = fiveDomainPlacements.find((placement) =>
    timelineItemKey(placement.item) === timelineItemKey(primary)).resolved_layer_id;
  const isolatedHorizontal = planTimelineItemLaneMove({
    items: fiveDomainItems, primary, target_layer_id: currentLayerId, delta_ms: -75, isolate: true,
    layers: laneLayers, groups: fiveDomainGroup, placements: fiveDomainPlacements,
  });
  assert.deepEqual(isolatedHorizontal, {
    ok: true,
    plan: {
      items: [primary], primary, delta_ms: -75,
      lane_targets: [{ item: primary, target_layer_id: currentLayerId }], isolate: true,
    },
  }, `${primary.kind} Alt horizontal move retains its resolved lane and edits only the primary item`);
}
const videoSceneMove = planTimelineItemLaneMove({
  items: [fiveDomainItems[0]], primary: fiveDomainItems[0], target_layer_id: 15, delta_ms: 0, isolate: true,
  layers: laneLayers, groups: [], placements: [{ item: fiveDomainItems[0], resolved_layer_id: 14 }],
});
assert.deepEqual(videoSceneMove, {
  ok: true,
  plan: {
    items: [fiveDomainItems[0]], primary: fiveDomainItems[0], delta_ms: 0,
    lane_targets: [{ item: fiveDomainItems[0], target_layer_id: 15 }], isolate: true,
  },
}, "Scene items resolve and move within a Video lane section despite their legacy TimelineItemRef name");
const crossKindSceneGroupMove = planTimelineItemLaneMove({
  items: [fiveDomainItems[0]], primary: fiveDomainItems[0], target_layer_id: 15, delta_ms: 125, isolate: false,
  layers: laneLayers, groups: fiveDomainGroup, placements: fiveDomainPlacements,
});
assert.equal(crossKindSceneGroupMove.ok, true, "Scene pointer target may cross from Lighting to Video");
assert.deepEqual(crossKindSceneGroupMove.plan.lane_targets, [
  { item: fiveDomainItems[0], target_layer_id: 15 },
  { item: fiveDomainItems[1], target_layer_id: 15 },
  { item: fiveDomainItems[2], target_layer_id: 11 },
  { item: fiveDomainItems[3], target_layer_id: 13 },
  { item: fiveDomainItems[4], target_layer_id: 15 },
], "Scene cross-kind primary uses the explicit target and applies its cross-section ordinal delta to the group");
assert.deepEqual(planTimelineItemLaneMove({
  items: [fiveDomainItems[0]], primary: fiveDomainItems[0], target_layer_id: 10, delta_ms: 0, isolate: true,
  layers: laneLayers, groups: fiveDomainGroup, placements: fiveDomainPlacements,
}), { ok: false, reason: "kind" }, "Scene targets reject Audio lanes");
const lockedLaneMove = planTimelineItemLaneMove({
  items: [fiveDomainItems[2]], primary: fiveDomainItems[2], target_layer_id: 11, delta_ms: 0, isolate: false,
  layers: laneLayers.map((layer) => layer.id === 15 ? { ...layer, locked: true } : layer),
  groups: fiveDomainGroup, placements: fiveDomainPlacements,
});
assert.deepEqual(lockedLaneMove, { ok: false, reason: "locked" }, "one locked projected member target rejects the full group");
const outOfRangeMove = planTimelineItemLaneMove({
  items: [fiveDomainItems[2]], primary: fiveDomainItems[2], target_layer_id: 11, delta_ms: 0, isolate: false,
  layers: laneLayers, groups: fiveDomainGroup,
  placements: fiveDomainPlacements.map((placement) => timelineItemKey(placement.item) === "video:11"
    ? { ...placement, resolved_layer_id: 15 }
    : placement),
});
assert.deepEqual(outOfRangeMove, { ok: false, reason: "range" }, "an out-of-range projected member rejects the full group");
assert.deepEqual(planTimelineItemLaneMove({
  items: [fiveDomainItems[2]], primary: fiveDomainItems[2], target_layer_id: 12, delta_ms: 0, isolate: false,
  layers: laneLayers, groups: fiveDomainGroup, placements: fiveDomainPlacements,
}), { ok: false, reason: "kind" }, "cross-section targets reject before dispatch");

console.log("Timeline advanced authoring: canonical snapshot fencing, runtime-free loop protection, armed/looping/off control semantics, and five-domain lane planning are exact.");
