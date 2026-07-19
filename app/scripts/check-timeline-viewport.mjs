import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/timelineViewport.ts", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineViewport.ts",
});
const viewport = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

const HOUR_MS = 3_600_000;
const HOUR_EDIT_EXTENT_MS = HOUR_MS + 5_000;
const fitHour = viewport.fitTimelineVisibleWindow(HOUR_MS);
assert.deepEqual(fitHour, { start_ms: 0, end_ms: HOUR_MS });
assert.equal(viewport.timelineVisibleWindowSpanMs(fitHour), HOUR_MS);

const terminalSecond = viewport.revealTimelineVisibleRange(
  fitHour,
  HOUR_EDIT_EXTENT_MS,
  3_599_000,
  3_600_000,
);
assert.deepEqual(
  terminalSecond,
  { start_ms: 3_597_000, end_ms: 3_602_000 },
  "a terminal one-second block is centered inside five seconds of editing headroom",
);
assert.equal(
  ((3_600_000 - 3_599_000) / viewport.timelineVisibleWindowSpanMs(terminalSecond)) * 100,
  20,
  "a selected one-second block occupies 20% of its minimum editing window",
);

const dragStartMs = viewport.timelineVisibleRatioToTimeMs(0.5, terminalSecond);
const dragEndMs = viewport.timelineVisibleRatioToTimeMs(0.5 + 60 / 1_200, terminalSecond);
assert.equal(
  dragEndMs - dragStartMs,
  250,
  "a 60px drag across a 1200px, five-second viewport projects to 250ms",
);
assert.equal(dragStartMs, 3_599_500, "ratio 0.5 maps to the visible-window midpoint");
assert.equal(viewport.timelineTimeToVisibleRawRatio(dragStartMs, terminalSecond), 0.5);
assert.equal(
  viewport.timelineTimeToVisibleRawRatio(3_596_000, terminalSecond),
  -0.2,
  "time-to-ratio remains raw so offscreen geometry is not collapsed onto an edge",
);

const zoomSource = { start_ms: 100_000, end_ms: 110_000 };
const zoomAnchorMs = 102_500;
const anchorRatioBefore = viewport.timelineTimeToVisibleRawRatio(zoomAnchorMs, zoomSource);
const zoomed = viewport.zoomTimelineVisibleWindow(zoomSource, HOUR_MS, 0.5, zoomAnchorMs);
assert.deepEqual(zoomed, { start_ms: 101_250, end_ms: 106_250 });
assert.equal(
  viewport.timelineTimeToVisibleRawRatio(zoomAnchorMs, zoomed),
  anchorRatioBefore,
  "zoom keeps an unconstrained anchor at the same pixel ratio",
);

assert.deepEqual(
  viewport.panTimelineVisibleWindow({ start_ms: 0, end_ms: 5_000 }, HOUR_MS, 1),
  { start_ms: 4_000, end_ms: 9_000 },
  "next pans by 80% and retains a 20% visual overlap",
);
assert.deepEqual(
  viewport.panTimelineVisibleWindow({ start_ms: 0, end_ms: 5_000 }, HOUR_MS, -1),
  { start_ms: 0, end_ms: 5_000 },
  "previous clamps at show start",
);
const terminalHeadroomPage = viewport.panTimelineVisibleWindow(terminalSecond, HOUR_EDIT_EXTENT_MS, 1);
assert.deepEqual(terminalHeadroomPage, { start_ms: 3_600_000, end_ms: 3_605_000 });
assert.deepEqual(
  viewport.panTimelineVisibleWindow(terminalHeadroomPage, HOUR_EDIT_EXTENT_MS, 1),
  terminalHeadroomPage,
  "next clamps at the editing extent after exposing terminal headroom",
);

assert.deepEqual(
  viewport.revealTimelineVisibleRange(zoomSource, HOUR_MS, 200_000, 200_000),
  { start_ms: 195_000, end_ms: 205_000 },
  "revealing a legacy point preserves the current span and centers it",
);
const longBlock = viewport.revealTimelineVisibleRange(fitHour, HOUR_MS, 600_000, 1_800_000);
assert.ok(longBlock.start_ms <= 600_000 && longBlock.end_ms >= 1_800_000, "a long block remains fully visible");

assert.equal(
  viewport.timelineRangeIntersectsVisibleWindow(3_599_000, 3_600_000, terminalSecond),
  true,
);
assert.equal(
  viewport.timelineRangeIntersectsVisibleWindow(0, 1_000, terminalSecond),
  false,
);
assert.equal(
  viewport.timelineRangeIntersectsVisibleWindow(3_600_000, 3_600_000, terminalSecond),
  true,
  "a point on the terminal boundary remains representable",
);

const boundaryWindow = { start_ms: 10_000, end_ms: 20_000 };
const boundaryOnlyDurationBlocks = Array.from({ length: 10_000 }, (_, index) => (
  index % 2 === 0
    ? { start_ms: index, end_ms: 10_000 }
    : { start_ms: 20_000, end_ms: 20_001 + index }
));
assert.equal(
  boundaryOnlyDurationBlocks.filter((block) => viewport.timelineRangeIntersectsVisibleWindow(
    block.start_ms,
    block.end_ms,
    boundaryWindow,
  )).length,
  0,
  "10k duration blocks that only touch a viewport boundary stay outside its half-open visible interval",
);
const boundaryPointEvents = Array.from({ length: 10_000 }, (_, index) => (
  index % 2 === 0 ? boundaryWindow.start_ms : boundaryWindow.end_ms
));
assert.equal(
  boundaryPointEvents.filter((timeMs) => viewport.timelineRangeIntersectsVisibleWindow(
    timeMs,
    timeMs,
    boundaryWindow,
  )).length,
  boundaryPointEvents.length,
  "point events remain visible at both viewport boundaries",
);
assert.equal(
  viewport.timelineRangeIntersectsVisibleWindow(9_999, 10_001, boundaryWindow),
  true,
  "a duration block with positive area inside the viewport remains visible",
);
assert.equal(
  viewport.timelineClosedRangeIntersectsVisibleWindow(0, boundaryWindow.start_ms, boundaryWindow),
  true,
  "an automation range retains its ending keyframe on the viewport start boundary",
);
assert.equal(
  viewport.timelineClosedRangeIntersectsVisibleWindow(boundaryWindow.end_ms, 30_000, boundaryWindow),
  true,
  "an automation range retains its starting keyframe on the viewport end boundary",
);
assert.equal(
  viewport.timelineClosedRangeIntersectsVisibleWindow(0, boundaryWindow.start_ms - 1, boundaryWindow),
  false,
  "an automation range with no endpoint or span in the viewport stays hidden",
);
assert.match(
  appSource,
  /timelineClosedRangeIntersectsVisibleWindow\(startMs, endMs, visibleWindow\)/,
  "automation rendering must use closed endpoint visibility rather than duration-block half-open visibility",
);

const hourTicks = viewport.buildTimelineRulerTicks(fitHour, 1_200);
assert.ok(hourTicks.length > 1 && hourTicks.length <= 64, "the one-hour ruler stays within its DOM budget");
assert.ok(hourTicks.every((tick, index) => (
  tick.ratio >= 0 && tick.ratio <= 1 &&
  (index === 0 || tick.time_ms > hourTicks[index - 1].time_ms)
)), "ruler ticks are ordered and window-relative");
const fiveSecondTicks = viewport.buildTimelineRulerTicks(terminalSecond, 1_200);
assert.ok(fiveSecondTicks.length > 1 && fiveSecondTicks.length <= 64);
const fiveSecondStep = fiveSecondTicks[1].time_ms - fiveSecondTicks[0].time_ms;
assert.equal(viewport.niceTimelineRulerStepMs(fiveSecondStep), fiveSecondStep, "ruler steps stay on 1-2-5 values");
assert.equal(viewport.formatTimelineRulerLabel(3_599_000, 500), "59:59.000");

const longProjectViewport = {
  ...viewport.createTimelineViewportState(3_000_000),
  mode: "manual",
  visible_window: { start_ms: 2_995_000, end_ms: 3_000_000 },
};
const replacementViewport = viewport.reconcileTimelineViewportState(
  longProjectViewport,
  10_000,
  { project_replaced: true },
);
assert.deepEqual(
  replacementViewport,
  {
    show_duration_ms: 10_000,
    edit_extent_ms: 15_000,
    visible_window: { start_ms: 0, end_ms: 10_000 },
    mode: "fit",
  },
  "replacing a long project inside the same App resets its manual tail window and stale edit extent",
);
assert.notEqual(
  replacementViewport.visible_window,
  longProjectViewport.visible_window,
  "the project replacement assertion must exercise a real viewport transition, not a navigation-time fixture default",
);

const sameProjectShrinkViewport = viewport.reconcileTimelineViewportState(longProjectViewport, 8_000);
assert.deepEqual(
  sameProjectShrinkViewport,
  {
    show_duration_ms: 8_000,
    edit_extent_ms: 13_000,
    visible_window: { start_ms: 0, end_ms: 8_000 },
    mode: "fit",
  },
  "shrinking the active show's duration reconciles a stale manual tail window without reloading the App",
);

const growingManualViewport = viewport.reconcileTimelineViewportState(
  {
    ...viewport.createTimelineViewportState(10_000),
    mode: "manual",
    visible_window: { start_ms: 2_000, end_ms: 7_000 },
  },
  20_000,
);
assert.deepEqual(
  growingManualViewport.visible_window,
  { start_ms: 2_000, end_ms: 7_000 },
  "growing content does not discard a deliberate manual edit window",
);
assert.equal(growingManualViewport.mode, "manual");
assert.equal(growingManualViewport.edit_extent_ms, 25_000);
assert.match(
  appSource,
  /reconcileTimelineViewportState\(\s*current,\s*timelineOverviewContentEndMsForTimeline\(next\.timeline\),\s*\{ project_replaced: true \}/s,
  "same-App project replacement must be wired to the tested viewport reset path",
);
assert.doesNotMatch(
  appSource,
  /Math\.max\(\s*timelineOverviewEditExtentMs\(\)/,
  "the App must not retain edit extent through a monotonic maximum",
);

const clippedDrag = viewport.updateTimelineAbsoluteDragProjection(
  viewport.beginTimelineAbsoluteDragProjection(500, 300),
  360,
  1_200,
  1_000,
);
assert.equal(clippedDrag.time_ms, 550, "a clipped item's drag is an absolute-time delta, not a visible-edge clamp");
const clippedTinyDrag = viewport.updateTimelineAbsoluteDragProjection(
  viewport.beginTimelineAbsoluteDragProjection(500, 300),
  302,
  1_200,
  1_000,
);
assert.equal(clippedTinyDrag.time_ms, 500, "sub-threshold clipped drags keep their original absolute time");

console.log("timeline viewport helpers ok");
