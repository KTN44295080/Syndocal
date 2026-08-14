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
const unrelatedSnapshotState = { active_cue_id: 77, timeline: { id: 1 }, timeline_bank: [{ id: 1 }] };
const appliedTimelineResult = engineSnapshotWithTimelineAdvancedResult(unrelatedSnapshotState, {
  timeline_bank: [{ id: 2, video_clips: [{ id: 111 }], audio_clips: [{ id: 121 }] }],
  active_timeline_id: 2,
  selected_items: rightSelection,
  authoring,
  mutation: {},
});
assert.equal(appliedTimelineResult.active_cue_id, 77, "post-ACK Timeline fallback preserves unrelated live engine state");
assert.equal(appliedTimelineResult.timeline.id, 2, "post-ACK Timeline fallback mounts the acknowledged active Timeline");
assert.deepEqual(
  [appliedTimelineResult.timeline.video_clips[0].id, appliedTimelineResult.timeline.audio_clips[0].id],
  [111, 121],
  "post-ACK Timeline fallback retains fresh split IDs",
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

console.log("Timeline advanced selection and lane planner: five-domain groups, Alt isolate, ordinal projection, and rejection are exact.");
