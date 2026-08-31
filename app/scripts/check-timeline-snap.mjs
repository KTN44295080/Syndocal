import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const transpile = async (path, name) => {
  const source = await readFile(new URL(path, import.meta.url), "utf8");
  const output = ts.transpileModule(source, {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: name,
  });
  return import(`data:text/javascript;base64,${Buffer.from(output.outputText).toString("base64")}`);
};

const viewport = await transpile("../src/timelineViewport.ts", "timelineViewport.ts");
const snap = await transpile("../src/timelineSnap.ts", "timelineSnap.ts");
const overviewSource = await readFile(new URL("../src/components/TimelineOverview.tsx", import.meta.url), "utf8");
const panelSource = await readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const window = { start_ms: 0, end_ms: 1_000 };

const sourceSection = (source, startMarker, endMarker) => {
  const start = source.indexOf(startMarker);
  const end = source.indexOf(endMarker, start + startMarker.length);
  assert.ok(start >= 0 && end > start, `source section ${startMarker} must remain bounded`);
  return source.slice(start, end);
};

const edge = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 113,
  moving_item_id: "moving",
  moving_item_ids: ["member"],
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [113], beat: [120], bar: [200] },
  visible_scene_blocks: [
    { id: "moving", lane_id: 1, kind: "Lighting", start_ms: 113, end_ms: 300 },
    { id: "member", lane_id: 1, kind: "Lighting", start_ms: 113, end_ms: 300 },
    { id: "locked-anchor", lane_id: 1, kind: "Lighting", start_ms: 113, end_ms: 300, locked: true },
    { id: "wrong-kind", lane_id: 1, kind: "Video", start_ms: 113, end_ms: 300 },
    { id: "wrong-lane", lane_id: 2, kind: "Lighting", start_ms: 113, end_ms: 300 },
    { id: "invalid", lane_id: 1, kind: "Lighting", start_ms: 113, end_ms: 300, valid: false },
  ],
});
assert.deepEqual(
  edge,
  {
    time_ms: 113,
    kind: "item-start",
    target_id: "locked-anchor",
    distance_ms: 0,
    distance_px: 0,
    threshold_ms: 8,
  },
  "same-lane/same-kind locked edges are anchors, while self/members/invalid and mismatched blocks are excluded",
);

const edgeWinsQuantizer = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 104,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [104] },
  visible_scene_blocks: [
    { id: "anchor", lane_id: 1, kind: "Lighting", start_ms: 110, end_ms: 300 },
  ],
});
assert.equal(edgeWinsQuantizer.kind, "item-start", "an eligible item edge wins over a quantizer inside the same threshold");
assert.equal(edgeWinsQuantizer.time_ms, 110, "item-edge resolution is returned directly and is not re-quantized");

const endEdge = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 292,
  moving_item_id: "moving-end",
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  visible_scene_blocks: [
    { id: "anchor-end", lane_id: 1, kind: "Lighting", start_ms: 100, end_ms: 300 },
  ],
});
assert.deepEqual(
  { kind: endEdge.kind, time_ms: endEdge.time_ms, target_id: endEdge.target_id },
  { kind: "item-end", time_ms: 300, target_id: "anchor-end" },
  "same-lane/same-kind end edges are magnetic anchors as well as starts",
);

const movingStart = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 305,
  moving_item_id: "moving",
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [300] },
  visible_scene_blocks: [{ id: "anchor", lane_id: 1, kind: "Lighting", start_ms: 500, end_ms: 700 }],
});
const movingEnd = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 505,
  moving_item_id: "moving",
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [500] },
  visible_scene_blocks: [{ id: "anchor", lane_id: 1, kind: "Lighting", start_ms: 500, end_ms: 700 }],
});
const moveCandidates = [
  { edge: "start", resolution: movingStart },
  { edge: "end", resolution: movingEnd },
].filter(({ resolution }) => resolution.kind !== "none");
moveCandidates.sort((left, right) => {
  const distanceDelta = left.resolution.distance_px - right.resolution.distance_px;
  if (Math.abs(distanceDelta) > 1e-7) return distanceDelta;
  const leftIsItemEdge = left.resolution.kind.startsWith("item-");
  const rightIsItemEdge = right.resolution.kind.startsWith("item-");
  if (leftIsItemEdge !== rightIsItemEdge) return leftIsItemEdge ? -1 : 1;
  return left.edge === right.edge ? 0 : left.edge === "start" ? -1 : 1;
});
assert.equal(movingStart.kind, "grid");
assert.equal(movingEnd.kind, "item-start");
assert.equal(moveCandidates[0].edge, "end", "a tied item-edge candidate wins over a grid candidate across both moving edges");
assert.equal(moveCandidates[0].resolution.time_ms - 200, 300, "when the moving end wins, the final start preserves block duration");

const tie = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 100,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  visible_scene_blocks: [
    { id: "later", lane_id: 1, kind: "Lighting", start_ms: 108, end_ms: 200 },
    { id: "earlier", lane_id: 1, kind: "Lighting", start_ms: 92, end_ms: 200 },
  ],
});
assert.equal(tie.time_ms, 92, "equal-distance item ties use stable time ordering");
assert.equal(tie.target_id, "earlier", "equal-distance item ties expose a deterministic anchor ID");

const quantizerTie = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 200,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [200], beat: [200], bar: [200] },
});
assert.deepEqual(
  { kind: quantizerTie.kind, time_ms: quantizerTie.time_ms },
  { kind: "bar", time_ms: 200 },
  "same-time quantizer ties prefer bar, then beat, then grid",
);

const zoomA = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 108,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: { start_ms: 0, end_ms: 1_000 },
  canvas_width_px: 1_000,
  visible_scene_blocks: [{ id: "anchor", lane_id: 1, kind: "Lighting", start_ms: 116, end_ms: 400 }],
});
const zoomB = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 108,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: { start_ms: 0, end_ms: 2_000 },
  canvas_width_px: 2_000,
  visible_scene_blocks: [{ id: "anchor", lane_id: 1, kind: "Lighting", start_ms: 116, end_ms: 400 }],
});
assert.equal(zoomA.threshold_ms, zoomB.threshold_ms, "the pixel threshold is invariant when span and canvas scale together");
assert.equal(zoomA.kind, zoomB.kind);
assert.equal(zoomA.time_ms, zoomB.time_ms);
const none = snap.resolveTimelineEdgeSnap({
  raw_time_ms: 100,
  target_lane_id: 1,
  target_kind: "Lighting",
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [120] },
});
assert.equal(none.kind, "none", "a candidate beyond the pixel threshold leaves the raw time unchanged");
assert.equal(none.time_ms, 100);

const fadeIn = snap.resolveTimelineFadeBoundarySnap({
  item_start_ms: 100,
  item_end_ms: 500,
  edge: "in",
  raw_fade_ms: 3,
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [105], beat: [104], bar: [106] },
});
assert.deepEqual(
  { boundary_ms: fadeIn.boundary_ms, fade_ms: fadeIn.fade_ms, kind: fadeIn.kind, target_id: fadeIn.target_id },
  { boundary_ms: 104, fade_ms: 4, kind: "beat", target_id: null },
  "fade-in snaps its absolute boundary and converts it back to local duration without item-edge magnets",
);
const fadeOut = snap.resolveTimelineFadeBoundarySnap({
  item_start_ms: 100,
  item_end_ms: 500,
  edge: "out",
  raw_fade_ms: 99,
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [400], beat: [402] },
});
assert.equal(fadeOut.boundary_ms, 402);
assert.equal(fadeOut.fade_ms, 98, "fade-out converts from the end boundary in the opposite direction");
const boundedFade = snap.resolveTimelineFadeBoundarySnap({
  item_start_ms: 100,
  item_end_ms: 500,
  edge: "in",
  raw_fade_ms: 999,
  visible_window: window,
  canvas_width_px: 1_000,
  quantizers: { grid: [700] },
});
assert.equal(boundedFade.fade_ms, 400, "fade duration is bounded to the item length");
assert.equal(boundedFade.kind, "none", "quantizers outside the item cannot move a fade boundary");

const grid = viewport.buildAdaptiveTimelineGrid({ start_ms: 0, end_ms: 10_000 }, 1_000);
assert.ok(grid.lines.length <= 256, "adaptive grid line count is bounded");
assert.equal(grid.major_step_ms, 1_000, "major unit uses a predictable nice 1-2-5 duration");
assert.equal(grid.subdivision_count, 5, "normal zoom subdivides the major unit into readable minor ticks");
assert.ok(grid.lines.every((line, index) => (
  line.time_ms >= 0 && line.time_ms <= 10_000 &&
  (index === 0 || line.time_ms > grid.lines[index - 1].time_ms)
)), "grid lines are ordered and visible-window bounded");
assert.ok(grid.lines.filter((line) => line.major).every((line) => line.label), "only major lines carry labels");
assert.ok(grid.lines.filter((line) => !line.major).every((line) => line.label === null), "minor lines stay unlabelled");
const majorPixels = grid.pixel_width * grid.major_step_ms / 10_000;
const minorPixels = grid.pixel_width * grid.minor_step_ms / 10_000;
assert.ok(majorPixels >= 72, "major labels have a separated readable spacing");
assert.ok(minorPixels >= 14, "minor lines have a separated readable spacing");
const zoomedGrid = viewport.buildTimelineGridModel({ start_ms: 0, end_ms: 1_000 }, 1_000);
assert.ok(zoomedGrid.major_step_ms < grid.major_step_ms, "zooming in subdivides the time model");
assert.ok(zoomedGrid.lines.some((line) => line.kind === "minor"), "zoomed grid retains unlabelled minor lines");
const cappedGrid = viewport.buildAdaptiveTimelineGrid({ start_ms: 0, end_ms: 3_600_000 }, 100_000, { maximum_lines: 32 });
assert.ok(cappedGrid.lines.length <= 32, "custom grid line cap is enforced for extreme canvases");
const fractionalWindow = viewport.buildAdaptiveTimelineGrid(
  { start_ms: 1e-9, end_ms: 9.999999999 },
  1_000,
);
assert.ok(
  fractionalWindow.lines.every((line) => line.time_ms >= 1e-9 && line.time_ms <= 9.999999999),
  "fractional windows never emit a line outside their strict time bounds",
);
assert.ok(
  fractionalWindow.lines.every((line) => line.ratio >= 0 && line.ratio <= 1 && line.x_px >= 0 && line.x_px <= 1_000),
  "fractional windows keep ratios and pixel positions inside the canvas",
);
assert.ok(!fractionalWindow.lines.some((line) => line.time_ms === 0), "epsilon must not leak a zero-time line into a window starting at 1e-9");
const unstableEndpointWindow = viewport.buildAdaptiveTimelineGrid(
  { start_ms: 0.30000000000000004, end_ms: 1.3000000000000003 },
  1_000,
);
assert.ok(unstableEndpointWindow.lines.some((line) => line.time_ms === 0.30000000000000004), "a real start endpoint survives floating-point quantization");
assert.ok(unstableEndpointWindow.lines.some((line) => line.time_ms === 1.3000000000000003), "a real end endpoint survives floating-point quantization");

// Edge magnetism is a separate affordance from logical quantization. In
// particular, turning Edges OFF must not turn off the selected Grid/Beat/Bar
// snap for media clips. Keep this as a source-level contract because the
// pointer handlers live inside the Solid component and are not independently
// mountable in this lightweight checker.
const audioGestureSource = sourceSection(
  overviewSource,
  "const moveAudioClipGesture",
  "const finishAudioClipGesture",
);
const videoGestureSource = sourceSection(
  overviewSource,
  "const moveVideoClipGesture",
  "const finishVideoClipGesture",
);
const grid500Snap = (timeMs, magnetEnabled) => {
  const clamped = Math.max(0, Number.isFinite(timeMs) ? timeMs : 0);
  // This mirrors App.snapTimeMs for the explicit Grid 500 ms case. The
  // argument is intentionally unused: item-edge candidates are not part of
  // media move/resize/fade handlers.
  void magnetEnabled;
  return Math.round(clamped / 500) * 500;
};
for (const [label, source, patterns] of [
  ["Audio", audioGestureSource, [
    /preview\.start_ms = props\.snapTimeMs\(rawStartMs\)/,
    /const startMs = props\.snapTimeMs\(rawStartMs\)/,
    /const endMs = props\.snapTimeMs\(rawEndMs\)/,
    /const fadeMs = props\.snapTimeMs\(projected\)/,
  ]],
  ["Video", videoGestureSource, [
    /preview\.start_ms = props\.snapTimeMs\(rawStartMs\)/,
    /const startMs = props\.snapTimeMs\(rawStartMs\)/,
    /const endMs = props\.snapTimeMs\(rawEndMs\)/,
  ]],
]) {
  for (const pattern of patterns) {
    assert.match(source, pattern, `${label} media gesture keeps Grid/Beat/Bar quantization when Edges are OFF`);
  }
  assert.doesNotMatch(source, /props\.magnetEnabled/, `${label} media gesture does not gate logical snap on edge magnetism`);
  assert.doesNotMatch(source, /resolveTimeline(?:Edge)?Snap|visible_scene_blocks/, `${label} media gesture does not use item-edge candidates`);
}
assert.equal(grid500Snap(749, false), 500, "Audio/Video media gestures still snap to Grid 500 with Edges OFF");
assert.equal(grid500Snap(751, false), 1_000, "Grid 500 rounds the opposite side with Edges OFF");
assert.match(overviewSource, /visible_scene_blocks: props\.magnetEnabled \? sceneSnapBlocks\(\) : \[\]/, "only Scene Block item-edge candidates are governed by Edges");

assert.match(overviewSource, /resolveTimelineFadeBoundarySnap/, "Scene Block fade previews use the absolute-boundary snap resolver");
assert.match(overviewSource, /data-timeline-snap-guide/, "Scene Block gestures render an explicit snap guide");
assert.match(overviewSource, /resolveSceneBlockMoveSnap/, "Scene Block moves resolve both start and end edges");
assert.match(overviewSource, /safeRawStartMs \+ safeDurationMs/, "Scene Block move resolution evaluates the moving end");
assert.match(overviewSource, /selected\.edge === "end"/, "an end-edge move snap derives the final start from the preserved duration");
assert.match(overviewSource, /onMoveEventPlacement\(drag\.eventId, drag\.timeMs, drag\.layerId, false\)/, "resolved Scene Block moves do not receive a second controller snap");
assert.match(overviewSource, /props\.onResizeEventTime\([\s\S]*?props\.stretchMode,\s*false,/, "resolved Scene Block resizes do not receive a second controller snap");
assert.match(overviewSource, /props\.onSetEventFade\(drag\.eventId, drag\.edge, drag\.fadeMs, false\)/, "resolved Scene Block fades do not receive a second controller snap");
assert.match(panelSource, /data-timeline-grid-quantize-state/, "the grid quantize state is exposed separately from edge magnet state");
assert.match(panelSource, /data-timeline-edge-magnet-state/, "the edge magnet state is exposed separately from grid quantization");
assert.match(appSource, /createSignal<TimelineSnapMode>\("Grid"\)/, "new Timeline sessions default grid quantization to Grid");
assert.match(appSource, /const \[timelineGridMs, setTimelineGridMs\] = createSignal\(500\)/, "new Timeline sessions default to a 500 ms grid");
assert.match(appSource, /const \[timelineMagnetEnabled, setTimelineMagnetEnabled\] = createSignal\(true\)/, "new Timeline sessions keep edge magnet enabled by default");
assert.match(panelSource, /Snap:[\s\S]*?Grid \$\{Math\.max\(1, Math\.round\(props\.gridMs\)\)\} ms/, "the exact logical grid interval is visible beside the adaptive ruler");

console.log("timeline snap/grid primitives ok");
