import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const source = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
function section(start, end) {
  const a = source.indexOf(start), b = source.indexOf(end, a);
  assert(a >= 0 && b > a, `Missing production adapter ${start}`);
  return source.slice(a, b);
}
const extracted = section("  const openOrCreateSuperScene =", "  const effectChooserCueId =")
  + section("  let timelineNavigatorBusy =", "  const snapTimelineDrafts =")
  + "\nreturn {openRootTimelineFromNavigator, openChildTimelineFromNavigator};";
const code = ts.transpileModule(extracted, {
  compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.None },
}).outputText;
function deferred() { let resolve, reject; const promise = new Promise((a, b) => { resolve = a; reject = b; }); return { promise, resolve, reject }; }
function fixture() {
  let epoch = 7, child = 11, confirmed = true, activeRoot = 1;
  let bank = [{ id: 1 }, { id: 2 }];
  let cues = [{ id: 11, label: "Child A", child_timeline: { duration_ms: 1234 } }, { id: 12, label: "No child" },
    { id: 13, label: "Child B", child_timeline: { duration_ms: 2345 } }];
  const calls = [];
  let commit = async () => ({});
  const record = (name) => (value) => calls.push([name, value]);
  const ports = {
    timelineBank: () => bank, timelineChildCueId: () => child,
    projectMappingsAuthority: () => ({ project_epoch: epoch }),
    snapshot: () => ({ timeline: { id: activeRoot }, cues }),
    requireAuthoritativeCue: id => cues.find(cue => cue.id === id) ?? null,
    confirmDiscardTimelineEditorDrafts: () => confirmed,
    commitTimelineAdvanced: async request => { calls.push(["commit", request]); return commit(request); },
    setTimelineChildCueId: value => { child = value; calls.push(["child", value]); },
    setSelectedTimelineSceneBlockEventId: record("block"), setTimelineSelection: record("selection"),
    setTimelineEventDrafts: record("events"), setTimelineAutomationDrafts: record("automation"),
    setTimelineVideoAutomationDrafts: record("videoAutomation"),
    setTimelineViewportState: record("viewport"), createTimelineViewportState: duration => ({ duration }),
    setEffectChooserFamily: record("family"), fitTimelineOverview: () => calls.push(["fit"]),
    setMessage: record("message"), batch: fn => fn(),
    viewportFixture: null, timelineBlockDurationMs: () => 1000,
    setSnapshot: () => assert.fail("Navigator must not create a local child"),
    timelineAuthorityReady: () => true,
    invoke: () => assert.fail("Navigator must not create a backend child"),
    refreshSnapshot: () => assert.fail("Existing-only navigation needs no backend refresh"),
  };
  return { ...new Function(...Object.keys(ports), code)(...Object.values(ports)), calls,
    get child() { return child; }, set child(v) { child = v; },
    set confirmed(v) { confirmed = v; }, set epoch(v) { epoch = v; },
    set commit(v) { commit = v; }, set bank(v) { bank = v; }, set cues(v) { cues = v; } };
}
const edits = f => f.calls.filter(([name]) => !["commit", "message"].includes(name));
{
  const f = fixture(); await f.openRootTimelineFromNavigator(1);
  assert.equal(f.child, null); assert(!f.calls.some(([name]) => name === "commit"));
  assert.deepEqual(edits(f).map(([name]) => name), ["child", "block", "selection", "events", "automation", "videoAutomation", "fit"]);
}
{
  const f = fixture(); await f.openRootTimelineFromNavigator(2);
  assert.deepEqual(f.calls[0], ["commit", { kind: "select_timeline", timeline_id: 2, play: false }]);
  assert.equal(f.child, null);
}
for (const failure of ["cancel", "missing", "null", "throw"]) {
  const f = fixture();
  if (failure === "cancel") f.confirmed = false;
  if (failure === "missing") f.bank = [];
  if (failure === "null") f.commit = async () => null;
  if (failure === "throw") f.commit = async () => { throw Error("rejected"); };
  await f.openRootTimelineFromNavigator(2);
  assert.equal(f.child, 11, failure); assert.deepEqual(edits(f), [], failure);
  if (["cancel", "missing"].includes(failure)) assert(!f.calls.some(([name]) => name === "commit"));
}
for (const stale of ["epoch", "child"]) {
  const f = fixture(), wait = deferred(); f.commit = () => wait.promise;
  const navigation = f.openRootTimelineFromNavigator(2);
  if (stale === "epoch") f.epoch = 8; else f.child = 13;
  wait.resolve({}); await navigation;
  assert.deepEqual(edits(f), [], stale); assert.equal(f.child, stale === "child" ? 13 : 11);
}
{
  const f = fixture(), wait = deferred(); f.commit = () => wait.promise;
  const navigation = f.openRootTimelineFromNavigator(2);
  await f.openRootTimelineFromNavigator(1); await f.openChildTimelineFromNavigator(13);
  assert.equal(f.calls.length, 1, "Concurrent navigator actions cannot replace the pending editor");
  wait.resolve(null); await navigation;
  await f.openChildTimelineFromNavigator(13);
  assert.equal(f.child, 13, "Failed root request releases the navigator guard");
  assert.deepEqual(f.calls.find(([name]) => name === "viewport"), ["viewport", { duration: 2345 }]);
}
for (const id of [12, 999]) {
  const f = fixture(); await f.openChildTimelineFromNavigator(id);
  assert.equal(f.child, 11); assert.deepEqual(edits(f), []);
}
{
  const f = fixture(); f.confirmed = false; await f.openChildTimelineFromNavigator(13);
  assert.equal(f.child, 11); assert.deepEqual(edits(f), []);
}
console.log("PASS production Timeline navigator adapters: root selection without playback, existing-only children, draft cancellation, rejected/stale commits and concurrent request exclusion.");
