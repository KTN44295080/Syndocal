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
assert.match(
  overviewSource,
  /const clusterBadgeWidthPx = TIMELINE_OVERLAP_BADGE_WIDTH_PX;/,
  "the overview renders badges with the helper's packing width",
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

console.log(`timeline overlap cluster helpers ok (10k ${benchmarkElapsedMs.toFixed(1)}ms)`);
