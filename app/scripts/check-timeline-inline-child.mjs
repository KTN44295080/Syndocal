import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";
import { createRequire } from "node:module";
import { pathToFileURL } from "node:url";

// Execute the production factory and its real pure dependencies, with no Tauri,
// browser, or copied recovery implementation.
const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src");
const cache = new Map();
function load(name) {
  const file = path.resolve(sourceRoot, `${name}.ts`);
  if (cache.has(file)) return cache.get(file);
  const result = ts.transpileModule(readFileSync(file, "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
    fileName: file,
    reportDiagnostics: true,
  });
  assert.equal(result.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length ?? 0, 0);
  const module = { exports: {} };
  cache.set(file, module.exports);
  const requireLocal = (specifier) => {
    assert.ok(specifier.startsWith("./"), `unexpected external dependency: ${specifier}`);
    return load(specifier.slice(2));
  };
  new Function("require", "module", "exports", result.outputText)(requireLocal, module, module.exports);
  return module.exports;
}
const { projectInlineChildTimelineRows: project } = load("timelineInlineChildProjection");
const parent = { id: 10, cue_id: 20, layer_id: 3, time_ms: 1000, duration_ms: 3000,
  total_duration_ms: 3000, source_offset_ms: 500, rate: 1, conform_to_tempo: false, loop_fill: false, loop_count: 1 };
const event = (id, time_ms, duration_ms, extra = {}) => ({ id, time_ms, duration_ms,
  cue_id: 40, track: "Lighting", layer_id: 2, conform_to_tempo: false, loop_count: 1, ...extra });
const cues = [{ id: 20, label: "DATE", child_timeline: { tempo_driven: false,
  layers: [{ id: 2, label: "Color", kind: "Lighting", order: 0, muted: false },
    { id: 5, label: "Video", kind: "Video", order: 1 }],
  events: [event(1, 0, 1000), event(2, 800, 1000), event(3, 3000, 1500), event(4, 6000, 1000),
    event(5, 0, 1000, { track: "Video", layer_id: 5 })] } }, { id: 40, label: "Blue" }];
const rows = project([parent], cues);
assert.equal(rows.length, 1);
assert.deepEqual(rows[0].blocks.map(b => [b.startMs, b.endMs, b.rail]), [[1000, 1500, 0], [1300, 2300, 1], [3500, 4000, 0]]);
assert.equal(rows[0].railCount, 2);
assert.equal(rows[0].parentCueId, 20);
assert.equal(rows[0].blocks[0].label, "Blue");
for (const patch of [{ rate: 2 }, { conform_to_tempo: true }, { loop_fill: true }, { loop_count: 2 }, { time_ms: NaN }, { source_offset_ms: -500 }]) {
  const invalid = project([{ ...parent, ...patch }], cues);
  assert.ok(invalid[0].issue); assert.equal(invalid[0].blocks.length, 0);
}
assert.equal(project([parent], []).length, 0);
const repeated = project([parent, { ...parent, id: 11, time_ms: 9000 }], cues);
assert.equal(new Set(repeated.map(r => r.key)).size, 2);
assert.equal(repeated[1].blocks[0].startMs, 9000);
const unresolved = structuredClone(cues); unresolved[0].child_timeline.events[0].layer_id = 99;
assert.ok(project([parent], unresolved)[0].issue);
const conformed = structuredClone(cues); conformed[0].child_timeline.events[0].conform_to_tempo = true;
assert.ok(project([parent], conformed)[0].issue);
const invalidDuration = structuredClone(cues); invalidDuration[0].child_timeline.events[0].duration_ms = NaN;
assert.ok(project([parent], invalidDuration)[0].issue);
const zero = structuredClone(cues); zero[0].child_timeline.events = [event(1, 500, 0), event(2, 3500, 0)];
assert.equal(project([parent], zero)[0].blocks.length, 1);
const source = readFileSync(path.join(sourceRoot, "components/TimelineOverview.tsx"), "utf8");
assert.match(source, /if \(!layer.expanded \|\| layer.kind !== "Lighting"\) continue/);
assert.match(source, /childTimelineCues\s*\?\?/);
const component = readFileSync(path.join(sourceRoot, "components/TimelineInlineChildRows.tsx"), "utf8");
assert.doesNotMatch(component, /data-timeline-event-id|data-timeline-layer-id/);
assert.match(component, /onPointerDown=\{stopEditing\}/);
assert.match(component, /onDrop=\{stopEditing\}/);
// Execute the actual production selector body to guard tick-only map churn.
const require = createRequire(import.meta.url);
const solid = await import(pathToFileURL(require.resolve("solid-js/dist/solid.js")).href);
const memoStart = source.indexOf("  const overlapRailCountByLayerId = createMemo(");
const memoEnd = source.indexOf("  const overlapLayerIds =", memoStart);
assert.ok(memoStart >= 0 && memoEnd > memoStart);
const memoCode = ts.transpileModule(source.slice(memoStart, memoEnd)
  + "\nreturn overlapRailCountByLayerId;", { compilerOptions: { target: ts.ScriptTarget.ES2022 } }).outputText;
solid.createRoot((dispose) => {
  const [layout, setLayout] = solid.createSignal([{ layer_id: 3, rail_count: 2 }]);
  const count = new Function("createMemo", "overlapRailLayout", memoCode)(solid.createMemo, layout);
  const initial = count();
  for (let i = 0; i < 100; i++) { setLayout([{ layer_id: 3, rail_count: 2 }]); assert.equal(count(), initial); }
  setLayout([{ layer_id: 3, rail_count: 3 }]); assert.notEqual(count(), initial); assert.equal(count().get(3), 3);
  dispose();
});
console.log("Inline child Timeline checks passed: offset, trim, overlap, repeated parents, non-lighting exclusion, unresolved and unsupported mappings, boundary points and readonly event isolation.");
