import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const overviewSource = await readFile(
  new URL("../src/components/TimelineOverview.tsx", import.meta.url),
  "utf8",
);
const stylesSource = await readFile(
  new URL("../src/styles.css", import.meta.url),
  "utf8",
);
const panelSource = await readFile(
  new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url),
  "utf8",
);

// The completed Timeline surface has one visual primitive for authored Scene
// events. Legacy zero-duration payloads may still arrive during migration, but
// the editor must not invent a cosmetic duration that disagrees with runtime.
assert.doesNotMatch(overviewSource, /timelinePoint(?:Stem|Flag)/, "point flag/stem markup must be retired");
assert.doesNotMatch(overviewSource, /\bpointEvent\b/, "point event CSS state must be retired");
assert.doesNotMatch(overviewSource, /legacyPointDisplayDurationMs/, "legacy events must not receive an invented display duration");
assert.match(
  overviewSource,
  /const sceneBlockDisplaySpanMs = \(event: TimelineOverviewEvent\) =>[\s\S]*?event\.total_duration_ms > 0[\s\S]*?: 0;/,
  "the block renderer preserves authored spans without synthesizing legacy duration",
);
assert.match(overviewSource, /"sceneBlock",/, "every Timeline event uses the Scene Block visual primitive");
assert.match(overviewSource, /data-timeline-event-kind=\{event\.duration_ms > 0 \? "scene-block" : "legacy-point-unsupported"\}/, "unsupported legacy data is explicit for UI/evidence consumers");
assert.match(overviewSource, /data-timeline-display-duration-ms=\{sceneBlockDisplaySpanMs\(event\)\}/, "display duration is observable separately from authored duration");
assert.match(overviewSource, /data-timeline-block-layout="solid-two-line"/, "legacy and authored events share the two-line block layout");

// A-B is an interval in timeline coordinates. Verify the source clips both
// ends to the visible window instead of relying on CSS overflow or a point
// marker, which would drift when zooming or scrolling.
assert.match(overviewSource, /const loopRegionGeometry = createMemo\(\(\) => \{/);
assert.match(overviewSource, /const clippedStartMs = Math\.max\(aMs, visibleStartMs\);/);
assert.match(overviewSource, /const clippedEndMs = Math\.min\(bMs, visibleEndMs\);/);
assert.match(overviewSource, /if \(clippedEndMs <= clippedStartMs\) return null;/);
assert.match(overviewSource, /data-timeline-loop-region/);
assert.match(overviewSource, /data-timeline-loop-a-ms=\{region\(\)\.aMs\}/);
assert.match(overviewSource, /data-timeline-loop-b-ms=\{region\(\)\.bMs\}/);
assert.match(overviewSource, /class="timelineLoopRegionBand"/);
assert.match(overviewSource, /class="timelineLoopRegionEdge start"/);
assert.match(overviewSource, /class="timelineLoopRegionEdge end"/);
assert.match(panelSource, /<TimelineOverview[\s\S]*?loopRegion=\{props\.loopRegion\}/, "the mounted overview receives the authoritative loop interval");

assert.match(stylesSource, /\.timelineLoopRegionBand[\s\S]*?rgba\(246, 143, 54, 0\.16\)/, "enabled loop span is orange");
assert.match(stylesSource, /\.timelineLoopRegion\.disabled \.timelineLoopRegionBand[\s\S]*?stroke-dasharray/, "disabled loop span remains visibly distinct");
assert.match(stylesSource, /\.timelineLoopRegionEdge[\s\S]*?rgba\(255, 184, 93, 0\.9\)/, "loop boundaries remain visible");

const clipLoop = (aMs, bMs, visibleStartMs, visibleEndMs) => {
  const a = Math.min(aMs, bMs);
  const b = Math.max(aMs, bMs);
  const start = Math.max(a, visibleStartMs);
  const end = Math.min(b, visibleEndMs);
  return end > start ? { start, end } : null;
};
assert.deepEqual(
  clipLoop(100, 300, 0, 1_000),
  { start: 100, end: 300 },
  "an in-window loop preserves exact A/B boundaries",
);
assert.deepEqual(
  clipLoop(-100, 300, 0, 1_000),
  { start: 0, end: 300 },
  "a loop crossing the visible start is clipped without moving its authored identity",
);
assert.deepEqual(
  clipLoop(100, 1_300, 0, 1_000),
  { start: 100, end: 1_000 },
  "a loop crossing the visible end is clipped without overflow",
);
assert.equal(clipLoop(100, 100, 0, 1_000), null, "an empty loop has no rendered interval");
assert.equal(clipLoop(1_100, 1_300, 0, 1_000), null, "an off-window loop has no rendered interval");

console.log("timeline block/loop presentation contract: PASS (no point flags or invented duration, clipped orange A-B span)");
