import { readFile } from "node:fs/promises";
import assert from "node:assert/strict";
import ts from "typescript";

const gestureSource = await readFile(new URL("../src/timelineBlockGestures.ts", import.meta.url), "utf8");
const gestureTranspiled = ts.transpileModule(gestureSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
  },
  fileName: "timelineBlockGestures.ts",
});
const gestureModuleUrl = `data:text/javascript;base64,${Buffer.from(gestureTranspiled.outputText).toString("base64")}`;
const gestures = await import(gestureModuleUrl);

const source = await readFile(new URL("../src/timelineSceneBlocks.ts", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source.replace("./timelineBlockGestures", gestureModuleUrl), {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineSceneBlocks.ts",
});
const helpers = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(gestures.timelineBlockGestureZone(50, 4, 100, true), "move");
assert.equal(gestures.timelineBlockGestureZone(3, 4, 100, true), "stretch-start");
assert.equal(gestures.timelineBlockGestureZone(97, 4, 100, true), "stretch-end");
assert.equal(gestures.timelineBlockGestureZone(8, 17, 100, true), "fade-in");
assert.equal(gestures.timelineBlockGestureZone(92, 17, 100, true), "fade-out");
assert.equal(gestures.timelineBlockGestureZone(50, 17, 100, true), "select");

const rateStretch = gestures.projectTimelineBlockStretch({
  mode: "RATE",
  edge: "end",
  originalStartMs: 1_000,
  originalEndMs: 3_000,
  requestedEdgeMs: 2_000,
  authoredBeats: 4,
  bpm: 120,
});
assert.equal(rateStretch.duration_ms, 1_000);
assert.equal(rateStretch.duration_beats, 2);
assert.equal(rateStretch.rate, 2, "shortening a four-beat authored block to one second doubles RATE");
assert.equal(rateStretch.loop_fill, false);

const windowStretch = gestures.projectTimelineBlockStretch({
  mode: "WINDOW",
  edge: "end",
  originalStartMs: 1_000,
  originalEndMs: 3_000,
  requestedEdgeMs: 4_000,
  authoredBeats: 4,
  bpm: 120,
});
assert.equal(windowStretch.duration_ms, 3_000);
assert.equal(windowStretch.rate, null);
assert.equal(windowStretch.loop_fill, true);
assert.equal(windowStretch.conform_to_tempo, true);

const rateFallback = gestures.projectTimelineBlockStretch({
  mode: "RATE",
  edge: "end",
  originalStartMs: 0,
  originalEndMs: 1_000,
  requestedEdgeMs: 1_500,
  authoredBeats: null,
  bpm: 120,
});
assert.equal(rateFallback.fallback_to_window, true);
assert.equal(rateFallback.conform_to_tempo, false);
assert.equal(gestures.projectTimelineBlockFadeMs("in", 1_000, 3_000, 1_350), 350);
assert.equal(gestures.projectTimelineBlockFadeMs("out", 1_000, 3_000, 2_400), 600);

const block = {
  id: 7,
  cue_id: 3,
  time_ms: 1000,
  time_beats: null,
  track: "Lighting",
  layer_id: null,
  duration_ms: 800,
  duration_beats: null,
  conform_to_tempo: false,
  loop_fill: false,
  source_offset_ms: 0,
  fade_in_ms: 0,
  fade_out_ms: 0,
  loop_count: 2,
  jump_to_event_id: 7,
};
const point = {
  ...block,
  id: 8,
  duration_ms: 0,
  loop_count: 1,
  jump_to_event_id: null,
};
const snap = (value) => Math.max(0, Math.round(value / 100) * 100);
const hasEventId = (eventId) => eventId === 7 || eventId === 8;

assert.equal(helpers.timelineExecutionIsLive(false, "Manual", false, null), false);
assert.equal(helpers.timelineExecutionIsLive(false, "Ltc", true, 500), true);
assert.equal(helpers.timelineExecutionIsLive(false, "Ltc", true, 999), true);
assert.equal(helpers.timelineExecutionIsLive(false, "Ltc", false, 1001), false);
assert.equal(helpers.timelineExecutionIsLive(true, "Manual", false, null), true);

assert.equal(helpers.timelineEventDraftsAreDirty([block], { 7: { ...block } }), false);
for (const patch of [
  { cue_id: 8 },
  { time_ms: 1200 },
  { track: "Video" },
  { duration_ms: 900 },
  { loop_count: 3 },
  { jump_to_event_id: 8 },
]) {
  assert.equal(
    helpers.timelineEventDraftsAreDirty([block], { 7: { ...block, ...patch } }),
    true,
    `Scene Block editor dirty state detects ${Object.keys(patch)[0]}`,
  );
}
assert.equal(
  helpers.timelineEventDraftsAreDirty([block], { 7: { ...block }, 99: { ...block } }),
  true,
  "orphaned editor drafts remain guarded from silent discard",
);

const dragStart = helpers.beginTimelineMarkerDragProjection(10, 40, 30, 300);
const tinyDrag = helpers.updateTimelineMarkerDragProjection(dragStart, 30.2, 302);
assert.equal(tinyDrag.x, 10, "sub-threshold movement keeps the original block start");
assert.equal(tinyDrag.moved, false);
const committedDrag = helpers.updateTimelineMarkerDragProjection(tinyDrag, 36, 360);
assert.equal(committedDrag.x, 16, "dragging preserves the pointer grab offset within a wide block");
assert.equal(committedDrag.moved, true);
assert.equal(helpers.shouldCommitTimelineMarkerDrag(committedDrag, false), true);
assert.equal(
  helpers.shouldCommitTimelineMarkerDrag(committedDrag, true),
  false,
  "pointer cancellation releases the drag without committing the projected position",
);

assert.deepEqual(
  helpers.timelineSceneBlockLoopDivisionPositions(30, 3, 1),
  [20],
  "a budgeted separator remains on a real three-loop boundary instead of the visual midpoint",
);
assert.equal(helpers.reconcileTimelineSceneBlockJumpTarget([7], 8), null);
assert.equal(
  helpers.reconcileTimelineSceneBlockJumpTarget([7, 8], 8),
  8,
  "the composer retains a jump only while its target placement still exists",
);
assert.equal(
  helpers.reconcileTimelineSceneBlockPickerTarget([7], 8, 8),
  null,
  "a live-removed picker target stays unselected until the operator deliberately chooses another placement",
);

assert.equal(helpers.timelineSceneBlockSpanMs(block), 1600);
assert.deepEqual(
  helpers.timelineSceneBlockPlaybackStatus(block, 1850, true),
  {
    under_playhead: true,
    live: true,
    iteration: 2,
    iteration_count: 2,
    remaining_ms: 750,
    jump_to_event_id: 7,
  },
  "live status exposes loop progress, block remaining time, and next target",
);
assert.equal(
  helpers.timelineSceneBlockPlaybackStatus(block, 1850, false).live,
  false,
  "a stopped or paused playhead is positional context, never a LIVE block",
);
assert.equal(helpers.timelineSceneBlockPlaybackStatus(block, 1850, false).under_playhead, true);
assert.equal(helpers.timelineSceneBlockPlaybackStatus(block, 2600, true).under_playhead, false);
assert.equal(helpers.timelinePlacementDisplayEndMs(block), 2600);
assert.equal(helpers.timelinePlacementDisplayEndMs(point), 2000);
const soleBlockViewportDuration = helpers.timelineSceneBlockViewportDurationMs(1000);
assert.equal(soleBlockViewportDuration, 5000);
assert.equal(
  helpers.growTimelineSceneBlockViewportDurationMs(soleBlockViewportDuration, 5000),
  5000,
  "moving a sole block to the viewport edge does not rescale its drop coordinate",
);
const soleBlockWidth = (1000 / soleBlockViewportDuration) * 100;
const soleBlockDrag = helpers.updateTimelineMarkerDragProjection(
  helpers.beginTimelineMarkerDragProjection(0, soleBlockWidth, soleBlockWidth / 2, 100),
  soleBlockWidth / 2 + 10,
  200,
);
assert.equal(soleBlockDrag.x, 10, "timeline headroom lets a sole/rightmost block move later");

const largeFixture = helpers.createTimelineSceneBlockLargeViewportFixture({
  id: 1,
  cue_list_id: 1,
  cue_number: "1",
  label: "Base",
  fade_ms: 0,
  targets: [],
  palette_targets: [],
  video_targets: [],
  video_output_targets: [],
  effect_targets: [],
  node_graph_targets: [],
});
assert.equal(largeFixture.cues.length, 500, "the scale fixture covers 500 source Cues");
assert.equal(largeFixture.events.length, 500, "the scale fixture covers 500 linked blocks");
assert.equal(
  largeFixture.events.at(-1).cue_id,
  largeFixture.cues.at(-1).id,
  "block 500 exercises a source Cue beyond the first picker page",
);
assert.equal(largeFixture.events.at(-1).loop_count, 32, "the long-label block remains narrow enough to test clipping");
const largeCueOptions = helpers.buildTimelineSceneBlockCueOptions(largeFixture.cues);
const selectedLastCueOptions = helpers.timelineSceneBlockSourcePickerOptions(
  largeCueOptions,
  "",
  largeFixture.cues.at(-1).id,
  80,
);
assert.equal(selectedLastCueOptions.length, 80);
assert.equal(
  selectedLastCueOptions[0].id,
  largeFixture.cues.at(-1).id,
  "opening the shared picker preserves Cue 500 even though it is outside the first 80 results",
);

const duplicateNumberOptions = helpers.buildTimelineSceneBlockCueOptions([
  { ...largeFixture.cues[0], id: 900, cue_list_id: 4, cue_number: "1", label: "Opening" },
  { ...largeFixture.cues[0], id: 901, cue_list_id: 9, cue_number: "1", label: "Opening" },
]);
assert.equal(duplicateNumberOptions[0].cue_list_id, 4);
assert.deepEqual(
  helpers.timelineSceneBlockSourcePickerOptions(duplicateNumberOptions, "list 9 cue 901", null, 80).map((cue) => cue.id),
  [901],
  "Cue list and Cue IDs disambiguate otherwise identical source numbers and labels",
);

const paletteOnlySummary = helpers.timelineSceneBlockSourceSummary({
  ...largeFixture.cues[0],
  targets: [],
  palette_targets: [
    { palette_id: 1, fixture_ids: [10, 11] },
    { palette_id: 2, fixture_ids: [11, 12] },
  ],
});
assert.match(paletteOnlySummary, /L 3 · P 2/, "palette-only lighting fixtures are included without double counting");

const preservedBlock = helpers.normalizeTimelineEventDraft(
  block,
  { duration_ms: 0, loop_count: 999, jump_to_event_id: 7 },
  snap,
  hasEventId,
);
assert.equal(preservedBlock.duration_ms, 1, "an existing block cannot be downgraded through legacy set");
assert.equal(preservedBlock.loop_count, 256);
assert.equal(preservedBlock.jump_to_event_id, 7, "self-jump remains an intentional indefinite repeat");

const preservedPoint = helpers.normalizeTimelineEventDraft(point, { duration_ms: 0 }, snap, hasEventId);
assert.equal(preservedPoint.duration_ms, 0);
assert.equal(preservedPoint.jump_to_event_id, null);

const upgradedPoint = helpers.normalizeTimelineEventDraft(
  point,
  { duration_ms: 750, loop_count: 4, jump_to_event_id: 8 },
  snap,
  hasEventId,
);
assert.equal(upgradedPoint.duration_ms, 750);
assert.equal(upgradedPoint.loop_count, 4);
assert.equal(upgradedPoint.jump_to_event_id, 8);

const snapPlacements = helpers.buildTimelineSceneBlockSnapPlacements(
  [block],
  () => ({
    cue_id: 8,
    time_ms: 1260,
    track: "Video",
    duration_ms: 900,
    loop_count: 3,
    jump_to_event_id: 8,
  }),
  snap,
  hasEventId,
);
assert.deepEqual(snapPlacements[0].draft, {
  cue_id: 8,
  time_ms: 1300,
  time_beats: null,
  track: "Video",
  layer_id: null,
  duration_ms: 900,
  duration_beats: null,
  conform_to_tempo: false,
  loop_fill: false,
  source_offset_ms: 0,
  fade_in_ms: 0,
  fade_out_ms: 0,
  loop_count: 3,
  jump_to_event_id: 8,
});
assert.deepEqual(snapPlacements[0].request, {
  event_id: 7,
  cue_id: 8,
  time_ms: 1300,
  time_beats: null,
  track: "Video",
  layer_id: null,
  duration_ms: 900,
  duration_beats: null,
  conform_to_tempo: false,
  loop_fill: false,
  fade_in_ms: 0,
  fade_out_ms: 0,
  loop_count: 3,
  jump_to_event_id: 8,
}, "atomic Snap Items request preserves normalized unsaved placement fields");

const pointSnapPlacement = helpers.buildTimelineSceneBlockSnapPlacements(
  [point],
  () => ({ ...point, time_ms: 1260, duration_ms: 0, loop_count: 4, jump_to_event_id: 7 }),
  snap,
  hasEventId,
)[0];
assert.equal(pointSnapPlacement.request.duration_ms, 0);
assert.equal(pointSnapPlacement.request.loop_count, 1, "legacy points always publish the engine-required loop count");
assert.equal(pointSnapPlacement.request.jump_to_event_id, null, "legacy points cannot publish a jump target");

const reconciledDrafts = helpers.reconcileTimelineEventDrafts(
  [block],
  {
    7: { ...block, time_ms: 1300, duration_ms: 900, loop_count: 3, jump_to_event_id: 8 },
    8: { ...point },
  },
);
assert.equal(reconciledDrafts[7].time_ms, 1300, "reconciliation preserves unsaved placement fields");
assert.equal(reconciledDrafts[7].duration_ms, 900);
assert.equal(reconciledDrafts[7].loop_count, 3);
assert.equal(reconciledDrafts[7].jump_to_event_id, null, "a removed jump target is cleared from the retained draft");
assert.equal(reconciledDrafts[8], undefined, "drafts for removed placements are discarded");

const commands = [];
const messages = [];
let draftAfterSave = null;
const controller = helpers.createTimelineSceneBlockController({
  invoke: async (command, args) => {
    commands.push({ command, args });
    return command === "add_timeline_scene_block" ? 99 : undefined;
  },
  snapTimeMs: snap,
  hasEventId,
  getEventById: (eventId) => eventId === block.id ? block : eventId === point.id ? point : undefined,
  getEventDraft: () => ({ ...block, time_ms: 1200, duration_ms: 0 }),
  setEventDraft: (_eventId, draft) => { draftAfterSave = draft; },
  getAddDurationMs: () => 1000,
  getAddLoopCount: () => 1,
  getAddJumpToEventId: () => null,
  setNextStartMs: () => {},
  setMessage: (message) => messages.push(message),
  refreshSnapshot: async () => {},
});

await controller.save(block);
assert.equal(commands.at(-1).command, "set_timeline_scene_block");
assert.equal(commands.at(-1).args.durationMs, 1);
assert.equal(draftAfterSave.duration_ms, 1);
assert.match(messages.at(-1), /Saved linked Scene Block/);

await controller.set(point, { time_ms: 1234 });
assert.equal(commands.at(-1).command, "set_timeline_cue_event");

await controller.moveBy(block, 500);
assert.equal(commands.at(-1).command, "set_timeline_scene_block");
assert.equal(commands.at(-1).args.durationMs, 1, "move merges the unsaved draft before changing start time");
assert.equal(commands.at(-1).args.timeMs, 1700, "nudge uses the unsaved Start draft as its base");

console.log("scene block helpers ok");
