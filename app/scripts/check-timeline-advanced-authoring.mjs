import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/timelineAdvancedAuthoring.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "timelineAdvancedAuthoring.ts",
}).outputText;
const { expandTimelineItemGroupSelection, timelineItemKey } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);

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
console.log("Timeline advanced selection: linked groups expand exactly across A/V, Lighting, and Automation domains.");
