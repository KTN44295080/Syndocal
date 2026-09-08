import assert from 'node:assert/strict';
import { readFile, writeFile } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import ts from 'typescript';
const args = process.argv.slice(2);
assert.ok(args.length === 0 || (args.length === 2 && args[0] === '--output'), 'Usage: [--output NEW_JSON_FILE]');

// Offline frontend projection only: no app, file reader, transport or device starts.
const stateSource = await readFile(new URL('../src/engineSnapshotLiveState.ts', import.meta.url), 'utf8');
const compiled = ts.transpileModule(stateSource, {
  compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 },
}).outputText;
const { snapshotLiveFixtures } = await import(`data:text/javascript;base64,${Buffer.from(compiled).toString('base64')}`);
const appSource = await readFile(new URL('../src/App.tsx', import.meta.url), 'utf8');
const parsed = ts.createSourceFile('App.tsx', appSource, ts.ScriptTarget.Latest, true, ts.ScriptKind.TSX);
function arrow(name) {
  const found = [];
  function visit(node) {
    if (ts.isVariableDeclaration(node) && node.name.getText(parsed) === name) found.push(node.initializer);
    ts.forEachChild(node, visit);
  }
  visit(parsed);
  assert.equal(found.length, 1, `Ambiguous/missing ${name}`);
  assert.ok(ts.isArrowFunction(found[0]) && ts.isBlock(found[0].body), `${name} must have a block`);
  return found[0];
}
const sync = arrow('applyEngineSnapshotSyncResponse');
const accepted = arrow('applyAcceptedEngineSnapshot');
// Isolate the actual App sync function and the live-publication prefix it calls.
// Admission/transport/UI effects are explicit harness ports, NOT native proof.
const prefix = accepted.body.statements.slice(0, 3).map(n => n.getText(parsed)).join('\n');
assert.match(prefix, /^latestEngineSnapshot = next;/);
assert.match(prefix, /setLiveDmxPreviews\(engineDmxPreviews\(next\)\)/);
assert.match(prefix, /setLiveFixtures\(snapshotLiveFixtures\(next\)\)/);
assert.ok(ts.isIfStatement(accepted.body.statements[3]), 'Publication prefix boundary changed');
const harnessSource = `
let latestEngineSnapshot, revisions = [], fixtureViews = [], dmxViews = [], uiApplies = 0;
let lastSnapshotUiApplyAt = 0, rejected = false;
const captureProjectReadGuard = () => ({});
const mergeEngineSnapshotSyncResponse = (current, response) => response.full ?? { ...current, ...response.delta };
const prepareTimelineRuntimeSnapshotIngress = value => rejected ? null : value;
const setLiveFixtures = value => fixtureViews.push(value);
const setLiveDmxPreviews = value => dmxViews.push(value);
const engineDmxPreviews = value => value.dmx_previews;
const setSnapshotRevision = revision => revisions.push(revision);
const applyAcceptedEngineSnapshot = next => { ${prefix}; uiApplies++; };
const applyEngineSnapshotSyncResponse = ${sync.getText(parsed)};
return (snapshot, response, reject = false) => {
 latestEngineSnapshot = snapshot; fixtureViews = []; dmxViews = []; revisions = []; uiApplies = 0; rejected = reject;
 const result = applyEngineSnapshotSyncResponse(response.wire, response.syncUiState);
 return { result, fixtureViews, dmxViews, revisions, uiApplies };
};`;
const harnessJs = ts.transpileModule(harnessSource, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
}).outputText;
let projections = 0;
const runIngress = new Function('snapshotLiveFixtures', harnessJs)(snapshot => {
  projections++; return snapshotLiveFixtures(snapshot);
});
function fixture(count) {
  const attributes = ['Red', 'Green', 'Blue', 'Dimmer', 'Pan', 'Tilt', 'Gobo', 'Zoom'];
  const fixtures = Array.from({ length: count }, (_, id) => ({ id,
    controls: attributes.map(attribute => ({ attribute })),
    attribute_values: attributes.map(attribute => ({ attribute, value: 0 })),
  }));
  const cues = Array.from({ length: 8 }, (_, id) => ({ id, targets: fixtures.filter(f => f.id % 8 === id)
    .map(f => ({ fixture_id: f.id, values: attributes.map(attribute => ({ attribute, value: (f.id * 17) % 256 })) })) }));
  return { fixtures, cues, active_cue_id: 0,
    active_group_cue_ids: Object.fromEntries(cues.map(cue => [`group-${cue.id}`, cue.id])),
    dmx_previews: [], clock: { bpm: 120 } };
}
const functional = fixture(64), frozen = structuredClone(functional);
const canonical = snapshotLiveFixtures(functional);
const paths = [];
for (const [name, full, syncUiState, reject] of [
  ['full-response', true, false, false], ['delta-with-ui', false, true, false],
  ['delta-live-only', false, false, false], ['stale-rejected', true, true, true],
]) {
  projections = 0;
  const wire = { revision: 2, ...(full ? { full: functional } : { delta: { clock: { bpm: 121 } } }) };
  const result = runIngress(functional, { wire, syncUiState }, reject);
  if (reject) { assert.equal(result.result, null); assert.deepEqual(result.revisions, []); assert.equal(projections, 0); }
  else { assert.deepEqual(result.revisions, [2]);
    for (const view of result.fixtureViews) assert.deepEqual(view, canonical);
    assert.equal(result.uiApplies, Number(full || syncUiState)); }
  paths.push({ name, liveProjections: projections, fixturePublications: result.fixtureViews.length,
    dmxPublications: result.dmxViews.length, editorApplies: result.uiApplies });
}
assert.deepEqual(functional, frozen, 'Projection must not mutate its input');
function summary(samples) {
  const sorted = [...samples].sort((a, b) => a - b);
  return { samples: sorted.length, p50_ms: sorted[Math.ceil(sorted.length * .5) - 1],
    p95_ms: sorted[Math.ceil(sorted.length * .95) - 1], max_ms: sorted.at(-1) };
}
const workloads = [];
for (const count of [64, 512, 2048]) {
  const snapshot = fixture(count), once = [], twice = [];
  for (let i = 0; i < 30; i++) snapshotLiveFixtures(snapshot);
  for (let i = 0; i < 200; i++) {
    for (const repeats of i % 2 ? [2, 1] : [1, 2]) {
      const start = performance.now(); let value;
      for (let k = 0; k < repeats; k++) value = snapshotLiveFixtures(snapshot);
      const elapsed = performance.now() - start;
      assert.equal(value.length, count);
      (repeats === 1 ? once : twice).push(elapsed);
    }
  }
  const inactive = { ...snapshot, active_cue_id: null, active_group_cue_ids: {} };
  assert.equal(snapshotLiveFixtures(inactive), snapshot.fixtures);
  workloads.push({ fixtures: count, controlsPerFixture: 8, activeCues: 8,
    once: summary(once), twice: summary(twice), inactiveReturnsOriginalArray: true });
}
const report = { schemaVersion: 1, node: process.version, platform: process.platform,
  sourceHashes: { app: createHash('sha256').update(appSource).digest('hex'),
    liveProjection: createHash('sha256').update(stateSource).digest('hex') },
  scope: 'App sync/prefix source harness and synthetic frontend projection timings; no real-show/native/IPC/lock/GPU measurement',
  paths, workloads, timingAssertions: false };
const output = JSON.stringify(report, null, 2);
if (args.length) await writeFile(args[1], `${output}\n`, { flag: 'wx' });
console.log(output);
