import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import ts from "typescript";

const source = await readFile(new URL("../src/timelineOverlapClusters.ts", import.meta.url), "utf8");
const overviewSource = await readFile(new URL("../src/components/TimelineOverview.tsx", import.meta.url), "utf8");
const panelSource = await readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const transpiled = ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineOverlapClusters.ts",
});
const overlap = await import(`data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`);

assert.equal(
  overlap.TIMELINE_OVERLAP_BADGE_WIDTH_PX,
  24,
  "the overlap packer and rendered badge share the 24px visual width contract",
);
assert.equal(
  overlap.TIMELINE_MAX_VISIBLE_OVERLAP_RAILS,
  8,
  "overlap rails expose an explicit eight-rail visibility budget",
);
assert.match(
  overviewSource,
  /const clusterBadgeWidthPx = TIMELINE_OVERLAP_BADGE_WIDTH_PX;/,
  "the overview renders badges with the helper's packing width",
);
assert.match(
  overviewSource,
  /buildTimelineOverlapRailLayout\(/,
  "the overview uses the deterministic model rail layout for overlap members",
);
assert.match(
  overviewSource,
  /class="timelineSceneBlockBody"/,
  "each visible overlap member uses the full Scene Block body",
);
assert.match(
  overviewSource,
  /data-timeline-block-fade-ramp="in"/,
  "visible overlap members preserve Scene Block fade polygons",
);
assert.match(
  overviewSource,
  /data-timeline-scene-block-badge=\{badge\.kind\}/,
  "visible overlap members preserve rate and loop badges",
);
assert.match(
  overviewSource,
  /data-timeline-overlap-rail=\{sceneBlockOverlapRail\(event\)\?\.rail_index\}/,
  "each rendered overlap span exposes its stable internal rail",
);
assert.match(
  overviewSource,
  /blockCenterYPx\(\{\s*id: event\.id,/s,
  "marker geometry applies the member rail to the SVG transform",
);
assert.doesNotMatch(
  overviewSource,
  /sceneBlockRendersCompactOverlapSpan|timelineSceneBlockOverlapSpan|timelineSceneBlockOverlapIdentityBand/,
  "overlap members use the existing full Scene Block subtree",
);
assert.match(
  overviewSource,
  /visibleRenderedEvents = createMemo/,
  "overflowed members do not create hidden stacked interaction surfaces",
);
assert.match(
  overviewSource,
  /renderedEvents\(\)\.filter\(\(event\) => !sceneBlockIsOverflowed\(event\)/,
  "the visible marker set filters overflowed events before DOM creation",
);
assert.match(
  overviewSource,
  /const markerTabStopId = createMemo\(\(\) => \{\s*const events = visibleRenderedEvents\(\)/s,
  "roving marker focus uses the same overflow-filtered visible event set",
);
assert.match(
  overviewSource,
  /<For each=\{visibleRenderedEvents\(\)\}>/,
  "the SVG marker DOM uses the same overflow-filtered visible event set",
);
assert.match(
  overviewSource,
  /Math\.min\(\s*overlapRailCountByLayerId\(\)\.get\(layer\.id\) \?\? 0,\s*TIMELINE_MAX_VISIBLE_OVERLAP_RAILS,/s,
  "lane height is bounded by the explicit rail budget",
);
assert.match(
  overviewSource,
  /overlapRailCount \* timelineUserLaneHeightPx \+ timelineOverlapBadgeRailHeightPx/,
  "lane height reserves 30px per rail plus the 18px badge rail",
);
assert.match(
  overviewSource,
  /event\.duration_ms > 0 && Number\.isFinite\(event\.total_duration_ms\)/,
  "display duration remains fail-closed for zero-duration legacy points",
);
assert.match(
  overviewSource,
  /left\.is_super_scene === right\.is_super_scene/,
  "super-scene state participates in marker identity stabilization",
);
assert.match(
  overviewSource,
  /new Set\(props\.overlapLayerIds\)/,
  "overlap rail reservation uses the full-timeline layer set",
);
assert.doesNotMatch(
  overviewSource,
  /new Set\(props\.overlapClusters\.map/,
  "visible overlap badges must not control lane height",
);
assert.match(
  panelSource,
  /overlapLayerIds=\{props\.overviewOverlapLayerIds\}/,
  "the panel forwards full-timeline overlap layer identity separately from visible badges",
);
assert.match(
  appSource,
  /overviewOverlapLayerIds=\{\[\s*\.\.\.new Set\(timelineOverlapClusters\(\)\.map\(\(cluster\) => cluster\.layer_id\)\),\s*\]\}/s,
  "the App derives reserved overlap rails from all timeline clusters before viewport filtering",
);

const canonicalOverlapEvents = [
  {
    id: 1,
    track: "Lighting",
    layer_id: 1,
    time_ms: 138_353,
    duration_ms: 81_153,
    total_duration_ms: 81_153,
  },
  {
    id: 2,
    track: "Lighting",
    layer_id: 1,
    time_ms: 138_353,
    duration_ms: 81_153,
    total_duration_ms: 81_153,
  },
];
const canonicalOverlapCluster = [{
  track: "Lighting",
  layer_id: 1,
  member_ids: [1, 2],
}];
const canonicalRails = overlap.buildTimelineOverlapRailLayout(
  canonicalOverlapEvents,
  canonicalOverlapCluster,
);
assert.deepEqual(
  canonicalRails,
  [
    { id: 1, track: "Lighting", layer_id: 1, rail_index: 0, rail_count: 2, overflowed: false },
    { id: 2, track: "Lighting", layer_id: 1, rail_index: 1, rail_count: 2, overflowed: false },
  ],
  "alpha52 canonical all_white/all_max blocks share time/lane but retain two visible rails",
);
assert.deepEqual(
  overlap.buildTimelineOverlapRailLayout(
    [...canonicalOverlapEvents].reverse(),
    [...canonicalOverlapCluster].map((cluster) => ({ ...cluster, member_ids: [...cluster.member_ids].reverse() })),
  ),
  canonicalRails,
  "overlap rail assignment is independent of input order",
);
assert.deepEqual(
  overlap.buildTimelineOverlapRailLayout(
    [{ ...canonicalOverlapEvents[0], id: 3 }],
    [{ track: "Lighting", layer_id: 2, member_ids: [3] }],
  ),
  [],
  "stale cluster membership from another layer cannot promote a rail",
);
assert.deepEqual(
  overlap.buildTimelineOverlapRailLayout(
    [{ ...canonicalOverlapEvents[0], id: 3 }],
    [{ track: "Video", layer_id: 1, member_ids: [3] }],
  ),
  [],
  "stale cluster membership from another track cannot promote a rail",
);
assert.deepEqual(
  overlap.buildTimelineOverlapRailLayout(
    [{ ...canonicalOverlapEvents[0], id: 3, duration_ms: 0, total_duration_ms: 81_153 }],
    [{ track: "Lighting", layer_id: 1, member_ids: [3] }],
  ),
  [],
  "zero-duration legacy points never receive a fabricated overlap rail",
);
const fiveHundredRailEvents = Array.from({ length: 500 }, (_, index) => ({
  id: index + 1,
  track: "Lighting",
  layer_id: 1,
  time_ms: 0,
  duration_ms: 1_000,
  total_duration_ms: 1_000,
}));
const fiveHundredRails = overlap.buildTimelineOverlapRailLayout(
  fiveHundredRailEvents,
  [{ track: "Lighting", layer_id: 1, member_ids: fiveHundredRailEvents.map((event) => event.id) }],
);
assert.equal(fiveHundredRails.length, 500, "the 500-item overlap budget retains one rail placement per authored event");
assert.equal(fiveHundredRails.filter((placement) => !placement.overflowed).length, 8);
assert.equal(fiveHundredRails.filter((placement) => placement.overflowed).length, 492);
assert.deepEqual(
  fiveHundredRails.slice(0, 8).map((placement) => placement.rail_index),
  [0, 1, 2, 3, 4, 5, 6, 7],
  "the first eight stable interval members occupy the bounded visible rails",
);
assert.ok(fiveHundredRails.slice(8).every((placement) => placement.rail_index === null));
assert.ok(fiveHundredRails.every((placement) => placement.rail_count === 8));
const selectedOverflowEventId = fiveHundredRails[8].id;
const visibleSceneBlockIds = fiveHundredRails
  .filter((placement) => !placement.overflowed)
  .map((placement) => placement.id);
const selectedOverflowTabStopId = visibleSceneBlockIds.includes(selectedOverflowEventId)
  ? selectedOverflowEventId
  : visibleSceneBlockIds[0] ?? null;
const selectedOverflowMarkerDom = visibleSceneBlockIds.map((id) => ({
  id,
  tabindex: id === selectedOverflowTabStopId ? 0 : -1,
}));
assert.equal(selectedOverflowMarkerDom.length, 8);
assert.equal(
  selectedOverflowMarkerDom.filter((marker) => marker.tabindex === 0).length,
  1,
  "a selected overflow event leaves exactly one visible Scene Block tabbable",
);
assert.equal(
  selectedOverflowMarkerDom.some((marker) => marker.id === selectedOverflowEventId),
  false,
  "a selected overflow event creates no hidden interaction surface",
);

const mixedDurationEvents = [
  { id: 1, track: "lighting", time_ms: 0, total_duration_ms: 1_000, duration_ms: 1, loop_count: 1 },
  { id: 2, track: "lighting", time_ms: 750, duration_ms: 200, loop_count: 2 },
];
assert.deepEqual(overlap.buildTimelineOverlapClusters(mixedDurationEvents), [{
  id: overlap.buildTimelineOverlapClusters([...mixedDurationEvents].reverse())[0].id,
  label: "lighting overlap (2)",
  track: "lighting",
  start_ms: 0,
  end_ms: 1_150,
  count: 2,
  member_ids: [1, 2],
}], "explicit total duration takes precedence and duration × loops is the fallback");

const separatedAndTouching = [
  { id: "a", track: "video", time_ms: 0, total_duration_ms: 100 },
  { id: "touch", track: "video", time_ms: 100, total_duration_ms: 100 },
  { id: "separate", track: "video", time_ms: 300, total_duration_ms: 50 },
];
assert.deepEqual(
  overlap.buildTimelineOverlapClusters(separatedAndTouching),
  [],
  "half-open intervals that only touch, or are separated, do not overlap",
);

const nestedAndChained = [
  { id: 30, track: "lighting", time_ms: 20, total_duration_ms: 10 },
  { id: 10, track: "lighting", time_ms: 0, total_duration_ms: 100 },
  { id: 20, track: "lighting", time_ms: 90, total_duration_ms: 30 },
  { id: 99, track: "video", time_ms: 20, total_duration_ms: 10 },
];
const nestedCluster = overlap.buildTimelineOverlapClusters(nestedAndChained);
assert.equal(nestedCluster.length, 1, "tracks are isolated and singleton components are omitted");
assert.deepEqual(nestedCluster[0].member_ids, [10, 30, 20]);
assert.deepEqual(
  { start_ms: nestedCluster[0].start_ms, end_ms: nestedCluster[0].end_ms },
  { start_ms: 0, end_ms: 120 },
  "nested and chained intersections form one connected absolute-time component",
);
assert.deepEqual(
  overlap.buildTimelineOverlapClusters([...nestedAndChained].reverse()),
  nestedCluster,
  "cluster order, member order, IDs, and labels do not depend on input order",
);

const fiveHundredEvents = Array.from({ length: 500 }, (_, index) => ({
  id: index + 1,
  track: (index + 1) % 2 === 0 ? "even" : "odd",
  time_ms: 10_000,
  duration_ms: 250,
  loop_count: 4,
}));
const fiveHundredClusters = overlap.buildTimelineOverlapClusters(fiveHundredEvents);
assert.equal(fiveHundredClusters.length, 2);
assert.equal(fiveHundredClusters.find((cluster) => cluster.track === "odd")?.count, 250);
assert.equal(fiveHundredClusters.find((cluster) => cluster.track === "even")?.count, 250);
assert.ok(fiveHundredClusters.every((cluster) => cluster.start_ms === 10_000 && cluster.end_ms === 11_000));
assert.ok(fiveHundredClusters.every((cluster) => cluster.id.length < 80), "large-cluster IDs stay compact");

const crowdedBadgeCandidates = Array.from({ length: 5 }, (_, index) => ({
  id: `cluster-${index}`,
  label: `lighting overlap (${index + 2})`,
  track: "lighting",
  start_ms: index * 1_000,
  end_ms: index * 1_000 + 500,
  count: 2,
  member_ids: [index * 2 + 1, index * 2 + 2],
  x: index < 3 ? -100 + index : (index === 3 ? 55 : 80),
  width: 4,
}));
const packedBadges = overlap.packTimelineOverlapClusterBadges(crowdedBadgeCandidates, 100);
assert.equal(packedBadges.length, 2, "badge overflow is combined into one reachable inspector");
assert.ok(packedBadges.every((badge) => (
  badge.x >= 27 && badge.x <= 100 - overlap.TIMELINE_OVERLAP_BADGE_WIDTH_PX - 2
)));
assert.ok(packedBadges.every((badge, index) => (
  index === 0 ||
  badge.x - packedBadges[index - 1].x >= overlap.TIMELINE_OVERLAP_BADGE_WIDTH_PX + 2
)));
assert.equal(packedBadges[0].x, 27, "near-start collisions remain anchored near their real time");
assert.deepEqual(packedBadges[0].source_cluster_ids, ["cluster-0", "cluster-1", "cluster-2"]);
assert.deepEqual(packedBadges[0].member_ids, [1, 2, 3, 4, 5, 6]);
assert.equal(packedBadges[0].group_count, 3);
assert.equal(packedBadges[0].group_count, packedBadges[0].source_cluster_ids.length);
assert.equal(packedBadges[0].aggregated, true);
assert.equal(packedBadges[0].label, "lighting overlap groups (3)");

const twoStageBadgeCandidates = [
  { id: "first", x: 27 },
  { id: "overflow-single", x: 55 },
  { id: "overflow-collision-a", x: 80 },
  { id: "overflow-collision-b", x: 81 },
].map((candidate, index) => ({
  ...candidate,
  label: "lighting overlap (2)",
  track: "lighting",
  start_ms: index * 1_000,
  end_ms: index * 1_000 + 500,
  count: 2,
  member_ids: [index * 2 + 1, index * 2 + 2],
  width: 4,
}));
const twoStagePackedBadges = overlap.packTimelineOverlapClusterBadges(
  twoStageBadgeCandidates,
  100,
  27,
  18,
  2,
  2,
  2,
);
assert.equal(twoStagePackedBadges.length, 2, "the per-track capacity keeps one individual badge and one inspector");
const twoStageOverflowBadge = twoStagePackedBadges[1];
assert.deepEqual(
  twoStageOverflowBadge.source_cluster_ids,
  ["overflow-single", "overflow-collision-a", "overflow-collision-b"],
  "capacity overflow reaggregates an already collision-aggregated badge without losing source clusters",
);
assert.equal(twoStageOverflowBadge.group_count, 3);
assert.equal(twoStageOverflowBadge.group_count, twoStageOverflowBadge.source_cluster_ids.length);
assert.equal(
  twoStageOverflowBadge.label,
  "lighting overlap groups (3)",
  "the badge label and ARIA group_count use the same source-overlap-group meaning after two-stage aggregation",
);
assert.equal(twoStageOverflowBadge.count, 6);
assert.deepEqual(twoStageOverflowBadge.member_ids, [3, 4, 5, 6, 7, 8]);
assert.equal(twoStageOverflowBadge.aggregated, true);

const viewportA = fiveHundredEvents.map((event) => ({ ...event, x: event.time_ms / 100, width: 10, visible: true }));
const viewportB = fiveHundredEvents.map((event) => ({ ...event, x: -50_000, width: 0.01, visible: false }));
assert.deepEqual(
  overlap.buildTimelineOverlapClusters(viewportA),
  overlap.buildTimelineOverlapClusters(viewportB),
  "pixel projections and viewport visibility never affect absolute-time membership",
);

const tenThousandEvents = Array.from({ length: 10_000 }, (_, index) => ({
  id: `event-${index}`,
  track: `track-${String(index % 10).padStart(2, "0")}`,
  time_ms: Math.floor(index / 10) % 4,
  total_duration_ms: 10_000,
}));
const benchmarkStart = performance.now();
const tenThousandClusters = overlap.buildTimelineOverlapClusters(tenThousandEvents);
const benchmarkElapsedMs = performance.now() - benchmarkStart;
assert.equal(tenThousandClusters.length, 10);
assert.ok(tenThousandClusters.every((cluster) => cluster.count === 1_000));
assert.ok(tenThousandClusters.every((cluster) => cluster.id.length < 80));
assert.ok(
  benchmarkElapsedMs < 2_000,
  `10k interval sweep must remain bounded (elapsed ${benchmarkElapsedMs.toFixed(1)}ms)`,
);
const tenThousandRailEvents = Array.from({ length: 10_000 }, (_, index) => ({
  id: index + 1,
  track: "Lighting",
  layer_id: 1,
  time_ms: 0,
  duration_ms: 10_000,
  total_duration_ms: 10_000,
}));
const tenThousandRailStart = performance.now();
const tenThousandRails = overlap.buildTimelineOverlapRailLayout(
  tenThousandRailEvents,
  [{ track: "Lighting", layer_id: 1, member_ids: tenThousandRailEvents.map((event) => event.id) }],
);
const tenThousandRailElapsedMs = performance.now() - tenThousandRailStart;
assert.equal(tenThousandRails.length, 10_000);
assert.equal(tenThousandRails.filter((placement) => placement.overflowed).length, 9_992);
assert.ok(tenThousandRails.every((placement) => placement.rail_count === 8));
assert.ok(tenThousandRails.slice(8).every((placement) => placement.rail_index === null));
assert.ok(
  tenThousandRailElapsedMs < 2_000,
  `10k bounded rail assignment must remain bounded (elapsed ${tenThousandRailElapsedMs.toFixed(1)}ms)`,
);

assert.equal(
  overlap.timelineOverlapIntervalForEvent({ id: 1, track: "lighting", time_ms: 0, duration_ms: 0, loop_count: 99 }),
  null,
  "zero-span point events cannot create overlap components",
);
assert.equal(
  overlap.timelineOverlapIntervalForEvent({ id: 2, track: "lighting", time_ms: Number.NaN, total_duration_ms: 100 }),
  null,
  "invalid absolute times are ignored",
);

console.log(`timeline overlap cluster helpers ok (10k ${benchmarkElapsedMs.toFixed(1)}ms; rails ${tenThousandRailElapsedMs.toFixed(1)}ms)`);
