import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [appSource, liveStateSource] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/engineSnapshotLiveState.ts", import.meta.url), "utf8"),
]);

const parsed = ts.createSourceFile(
  "App.tsx",
  appSource,
  ts.ScriptTarget.Latest,
  true,
  ts.ScriptKind.TSX,
);
const findArrow = (name) => {
  const found = [];
  const visit = (node) => {
    if (ts.isVariableDeclaration(node) && node.name.getText(parsed) === name) {
      found.push(node.initializer);
    }
    ts.forEachChild(node, visit);
  };
  visit(parsed);
  assert.equal(found.length, 1, `Ambiguous or missing ${name}`);
  assert.ok(ts.isArrowFunction(found[0]), `${name} must remain an arrow function`);
  return found[0];
};

const sync = findArrow("applyEngineSnapshotSyncResponse");
const faderProjection = findArrow("snapshotFaderValues");
const faderSource = await readFile(new URL("../src/createFaderValueController.ts", import.meta.url), "utf8");
const faderModule = ts.transpileModule(faderSource, { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022 } }).outputText;
const { createFaderValueController } = await import(`data:text/javascript;base64,${Buffer.from(faderModule).toString("base64")}`);
const accepted = findArrow("applyAcceptedEngineSnapshot");
assert.ok(ts.isBlock(accepted.body), "accepted snapshot application must remain block-bodied");
const acceptedFader = accepted.body.statements.find(statement => statement.getText(parsed).startsWith("setFaderValues("));
assert.ok(acceptedFader, "live fader application must be outside project-state/draft-only branches");
const acceptedPrefix = accepted.body.statements.slice(0, 3).map((statement) => statement.getText(parsed)).join("\n");
assert.match(acceptedPrefix, /^latestEngineSnapshot = next;/);
assert.match(acceptedPrefix, /setLiveDmxPreviews\(engineDmxPreviews\(next\)\)/);
assert.match(acceptedPrefix, /setLiveFixtures\(snapshotLiveFixtures\(next\)\)/);

const liveStateCompiled = ts.transpileModule(liveStateSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "engineSnapshotLiveState.ts",
}).outputText;
const { snapshotLiveFixtures } = await import(
  `data:text/javascript;base64,${Buffer.from(liveStateCompiled).toString("base64")}`,
);

// Execute the actual App sync arrow and the actual live fixture projection.
// Native admission, project authority, and setters are explicit harness ports;
// no Tauri command or application window is started by this checker.
const harnessSource = `
let latestEngineSnapshot;
let liveDmx = [];
let liveFixtures = [];
let snapshotRevision;
let acceptedApplications = 0;
let faderValues = {};
const setFaderValues = update => { faderValues = update(faderValues); };
const faderValueController = createFaderValueController({setValues:setFaderValues});
const snapshotFaderValues = ${faderProjection.getText(parsed)};
let rejected = false;
const performance = { now: () => 1 };
const mergeEngineSnapshotSyncResponse = (current, response) =>
  response.full ?? { ...current, ...response.delta };
const prepareTimelineRuntimeSnapshotIngress = (candidate) => rejected ? null : candidate;
const engineDmxPreviews = (candidate) => candidate.dmx_previews ?? [];
const setLiveDmxPreviews = (candidate) => { liveDmx.push(candidate); };
const setLiveFixtures = (candidate) => { liveFixtures.push(candidate); };
const setSnapshotRevision = (candidate) => { snapshotRevision = candidate; };
const applyAcceptedEngineSnapshot = (next, syncProjectState, resetEditorDrafts) => {
  ${acceptedPrefix}
  ${acceptedFader.getText(parsed)}
  acceptedApplications += 1;
};
const applyEngineSnapshotSyncResponse = ${sync.getText(parsed)};
return (initial) => {
  latestEngineSnapshot = initial;
  faderValueController.clearAll();
  faderValues = snapshotFaderValues(initial);
  liveDmx = [];
  liveFixtures = [];
  snapshotRevision = undefined;
  acceptedApplications = 0;
  return {
    optimistic: (key, value) => faderValueController.setOptimistic(key, value),
    apply: (response, syncUiState, shouldReject = false) => {
      rejected = shouldReject;
      return applyEngineSnapshotSyncResponse(response, syncUiState, {});
    },
    read: () => ({
      latestEngineSnapshot,
      liveDmx,
      liveFixtures,
      snapshotRevision,
      acceptedApplications,
      faderValues,
    }),
  };
};`;
const harnessJs = ts.transpileModule(harnessSource, {
  compilerOptions: { module: ts.ModuleKind.None, target: ts.ScriptTarget.ES2022 },
  fileName: "snapshot-live-publication-harness.ts",
}).outputText;
const harness = new Function("snapshotLiveFixtures", "createFaderValueController", harnessJs)(snapshotLiveFixtures, createFaderValueController);
const initial = {
  id: "A",
  fixtures: [{ id: 1, controls: [{ attribute: "Red" }], attribute_values: [{ attribute: "Red", value: 0 }] }],
  cues: [],
  active_cue_id: null,
  active_group_cue_ids: {},
  dmx_previews: [1],
};
const next = { ...initial, id: "B", dmx_previews: [2] };
const fullSession = harness(initial);
const fullResult = fullSession.apply({ revision: 11, full: next }, false);
const full = fullSession.read();
assert.equal(fullResult.id, "B");
assert.equal(full.latestEngineSnapshot.id, "B");
assert.equal(full.liveDmx.length, 1, "full response must publish live DMX once");
assert.equal(full.liveFixtures.length, 1, "full response must project live fixtures once");
assert.equal(full.acceptedApplications, 1, "full response must cross the accepted UI seam once");
assert.equal(full.snapshotRevision, 11);

const uiDeltaSession = harness(initial);
uiDeltaSession.apply({ revision: 12, delta: { id: "B", dmx_previews: [2] } }, true);
const uiDelta = uiDeltaSession.read();
assert.equal(uiDelta.liveDmx.length, 1, "UI-sync delta must publish live DMX once");
assert.equal(uiDelta.liveFixtures.length, 1, "UI-sync delta must project live fixtures once");
assert.equal(uiDelta.acceptedApplications, 1, "UI-sync delta must cross the accepted UI seam once");
assert.equal(uiDelta.snapshotRevision, 12);

const liveOnlySession = harness(initial);
liveOnlySession.apply({ revision: 13, delta: { id: "B", dmx_previews: [2] } }, false);
const liveOnly = liveOnlySession.read();
assert.equal(liveOnly.liveDmx.length, 1, "live-only delta must publish live DMX once");
assert.equal(liveOnly.liveFixtures.length, 1, "live-only delta must project live fixtures once");
assert.equal(liveOnly.acceptedApplications, 0, "live-only delta must not apply editor state");
assert.equal(liveOnly.snapshotRevision, 13);

const rejectedSession = harness(initial);
const rejectedResult = rejectedSession.apply({ revision: 14, full: next }, true, true);
const rejected = rejectedSession.read();
assert.equal(rejectedResult, null, "rejected ingress must return null");
assert.equal(rejected.liveDmx.length, 0, "rejected ingress must not publish live DMX");
assert.equal(rejected.liveFixtures.length, 0, "rejected ingress must not project live fixtures");
assert.equal(rejected.acceptedApplications, 0, "rejected ingress must not cross the accepted UI seam");
assert.equal(rejected.snapshotRevision, undefined, "rejected ingress must not advance revision");

const successiveSession = harness(initial);
successiveSession.apply({ revision: 15, full: next }, false);
successiveSession.apply({ revision: 16, delta: { id: "C", dmx_previews: [3] } }, false);
const successive = successiveSession.read();
assert.equal(successive.latestEngineSnapshot.id, "C");
assert.deepEqual(successive.liveDmx, [[2], [3]]);
assert.equal(successive.liveFixtures.length, 2);
assert.equal(successive.acceptedApplications, 1);

console.log("snapshot live publication: full/ui-delta/live-only/rejected/successive paths passed");

const faderSession = harness(initial);
const values = value => [{...initial.fixtures[0], attribute_values:[{attribute:'Red',value}]}];
faderSession.apply({revision:21,delta:{fixtures:values(30000)}},false);
assert.equal(faderSession.read().faderValues['1:Red'],30000,'MIDI/live-only delta updates visible values without editor synchronization');
assert.equal(faderSession.read().acceptedApplications,0);
const stableFaders=faderSession.read().faderValues;
faderSession.apply({revision:22,delta:{fixtures:values(30000)}},false);
assert.equal(faderSession.read().faderValues,stableFaders,'unchanged live values reuse state and do not repaint the fader tree');
faderSession.optimistic('1:Red',40000);
faderSession.apply({revision:23,delta:{fixtures:values(30000)}},false);
assert.equal(faderSession.read().faderValues['1:Red'],40000,'an older snapshot cannot undo an owned GUI gesture');
faderSession.apply({revision:24,delta:{fixtures:values(40000)}},false);
faderSession.apply({revision:25,delta:{fixtures:values(10000)}},false);
assert.equal(faderSession.read().faderValues['1:Red'],10000,'new MIDI value applies after the GUI gesture is acknowledged');
faderSession.apply({revision:26,delta:{fixtures:values(50000)}},false,true);
assert.equal(faderSession.read().faderValues['1:Red'],10000,'a rejected older project cannot update faders');
console.log('PASS: actual live fader ingress, optimistic ordering, project rejection and unchanged-state reuse');
