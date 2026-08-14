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
console.log("Timeline advanced selection: linked groups expand exactly across A/V, Lighting, and Automation domains.");
