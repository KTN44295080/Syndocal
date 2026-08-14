import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/timelineAdvancedAuthoring.ts", import.meta.url), "utf8");
const output = ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove },
  fileName: "timelineAdvancedAuthoring.ts",
}).outputText;
const { removeTimelineAudioClipFromAdvancedAuthoring } = await import(`data:text/javascript;base64,${Buffer.from(output).toString("base64")}`);

const authoring = {
  video_clips: [{ id: 11, layer_id: 1, media_asset_id: 9, start_ms: 100, offset_ms: 0, duration_ms: 1_000, fade_in_ms: 0, fade_out_ms: 0 }],
  audio_clips: [
    { id: 21, layer_id: 2, media_asset_id: 9, path: "a.wav", start_ms: 100, offset_ms: 0, duration_ms: 1_000, gain: 1, fade_in_ms: 0, fade_out_ms: 0 },
    { id: 22, layer_id: 2, path: "b.wav", start_ms: 2_000, offset_ms: 0, duration_ms: 500, gain: 1, fade_in_ms: 0, fade_out_ms: 0 },
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

const removed = removeTimelineAudioClipFromAdvancedAuthoring(authoring, 21);
assert.deepEqual(removed.audio_clips.map((clip) => clip.id), [22]);
assert.deepEqual(removed.video_clips, authoring.video_clips, "the linked video remains authored after deleting only its audio peer");
assert.deepEqual(removed.item_groups.map((group) => group.id), [32], "the deleted member's immutable group dissolves while unrelated groups remain exact");
assert.deepEqual(authoring.audio_clips.map((clip) => clip.id), [21, 22], "candidate construction never mutates A in place");
console.log("Timeline advanced authoring removal: linked group dissolved atomically; unrelated authoring preserved.");
