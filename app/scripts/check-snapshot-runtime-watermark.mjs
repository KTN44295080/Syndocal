import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [watermarkSource, ingressSource, refreshControllerSource, loopIntegrationSource, appSource] = await Promise.all([
  readFile(new URL("../src/timelineRuntimeSnapshotWatermark.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/timelineRuntimeSnapshotIngress.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/timelineSnapshotRefreshController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/timelineLoopRuntimeIntegration.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
]);

const compilerOptions = {
  module: ts.ModuleKind.ESNext,
  target: ts.ScriptTarget.ES2022,
  importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
};
const dataModule = (source, fileName) => `data:text/javascript;base64,${Buffer.from(
  ts.transpileModule(source, { compilerOptions, fileName }).outputText,
).toString("base64")}`;
const watermarkModuleUrl = dataModule(watermarkSource, "timelineRuntimeSnapshotWatermark.ts");
const transpiled = ts.transpileModule(watermarkSource, {
  compilerOptions,
  fileName: "timelineRuntimeSnapshotWatermark.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);
const ingressTranspiled = ts.transpileModule(ingressSource, {
  compilerOptions,
  fileName: "timelineRuntimeSnapshotIngress.ts",
});
const ingress = await import(
  `data:text/javascript;base64,${Buffer.from(
    ingressTranspiled.outputText.replace("./timelineRuntimeSnapshotWatermark", watermarkModuleUrl),
  ).toString("base64")}`,
);
const refreshController = await import(dataModule(
  refreshControllerSource,
  "timelineSnapshotRefreshController.ts",
));

const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
};

const project = (epoch = 11, revision = 12, checkpoint = "a".repeat(64), readGeneration = 7) => ({
  project_epoch: epoch,
  project_revision: revision,
  checkpoint_hash: checkpoint,
  project_read_generation: readGeneration,
});

const snapshot = (epoch, generation, loopGeneration, followGeneration) => ({
  timeline: {
    transport_epoch: epoch,
    transport_generation: generation,
    loop_runtime: { generation: loopGeneration, status: "disabled", wrap_count: 0 },
    follow_runtime: {
      epoch,
      generation: followGeneration,
      status: "idle",
      elapsed_ms: 0,
      duration_ms: 0,
      progress_millis: 0,
    },
  },
});

const admit = (gate, scope, candidate) => {
  const watermark = runtime.timelineRuntimeSnapshotWatermarkFromEngineSnapshot(candidate);
  return watermark !== null && gate.canAccept(scope, watermark);
};

const readGuard = (generation, epoch, revision, checkpoint) => ({
  generation,
  authority: {
    project_epoch: epoch,
    project_revision: revision,
    checkpoint_hash: checkpoint,
  },
});

const sameReadGuard = (left, right) => left.generation === right.generation
  && left.authority.project_epoch === right.authority.project_epoch
  && left.authority.project_revision === right.authority.project_revision
  && left.authority.checkpoint_hash === right.authority.checkpoint_hash;

// Full ingress captures one project-read guard and carries that exact identity
// through the shared watermark seam. A replacement scope resets the gate, but
// a delayed read from the old scope remains rejected.
let ingressCurrentGuard = readGuard(1, 2, 3, "a".repeat(64));
const ingressRuntime = ingress.createTimelineRuntimeSnapshotIngress({
  captureProjectReadGuard: () => ingressCurrentGuard,
  projectReadGuardIsCurrent: (candidate) => sameReadGuard(candidate, ingressCurrentGuard),
  normalizeEngineSnapshot: (candidate) => candidate,
  isTauriRuntime: () => true,
});
const ingressA = snapshot(1, 1, 0, 0);
assert.strictEqual(ingressRuntime.prepare(ingressA), ingressA, "current full ingress must be admitted");
const staleIngressGuard = ingressCurrentGuard;
ingressCurrentGuard = readGuard(2, 3, 4, "b".repeat(64));
assert.equal(
  ingressRuntime.prepare(ingressA, { projectReadGuard: staleIngressGuard }),
  null,
  "a full image from the superseded project-read guard must remain unapplied",
);
const ingressB = snapshot(1, 1, 0, 0);
assert.strictEqual(
  ingressRuntime.prepare(ingressB, { projectReadGuard: ingressCurrentGuard, resetForProjectScope: true }),
  ingressB,
  "an authoritative replacement must reset the shared watermark for its new scope",
);
assert.equal(
  ingressRuntime.prepare(ingressA, { projectReadGuard: staleIngressGuard }),
  null,
  "the old project scope may not re-enter after replacement reset",
);

// The full-refresh controller resolves a current canonical read with the
// queued guard, while a read that becomes stale resolves null and never calls
// the guarded apply seam.
const canonicalGuard = readGuard(7, 8, 9, "c".repeat(64));
const canonicalCandidate = snapshot(3, 4, 1, 1);
const canonicalApplies = [];
let canonicalBeginCount = 0;
let canonicalFinishCount = 0;
const canonicalController = refreshController.createTimelineSnapshotRefreshController({
  captureProjectReadGuard: () => canonicalGuard,
  projectReadGuardIsCurrent: (candidate) => sameReadGuard(candidate, canonicalGuard),
  invoke: async (command) => {
    assert.equal(command, "get_snapshot");
    return canonicalCandidate;
  },
  snapshotRequestGuard: {
    beginFull: () => { canonicalBeginCount += 1; },
    finishFull: () => { canonicalFinishCount += 1; },
  },
  timelineTransportCanonicalSnapshotGeneration: () => 0,
  refreshOperatorPolicy: async () => undefined,
  refreshFixtureGroups: async () => undefined,
  setFixtureGroupDeleteUndoAvailable: () => undefined,
  setViewportFixtureGroupDeleteUndo: () => undefined,
  setMessage: () => undefined,
});
const canonicalResultPromise = canonicalController.refresh(true, false, () => {
  void canonicalController.run((candidate, syncProjectState, resetEditorDrafts, projectReadGuard) => {
    canonicalApplies.push({ candidate, syncProjectState, resetEditorDrafts, projectReadGuard });
    return true;
  });
});
assert.strictEqual(await canonicalResultPromise, canonicalCandidate, "a current full read must resolve its canonical candidate");
assert.equal(canonicalApplies.length, 1, "a current full read must cross the guarded apply seam once");
assert.strictEqual(canonicalApplies[0].projectReadGuard, canonicalGuard, "full apply must receive the exact queued read guard");
assert.equal(canonicalApplies[0].syncProjectState, true);
assert.equal(canonicalApplies[0].resetEditorDrafts, false);
assert.equal(canonicalBeginCount, 1);
assert.equal(canonicalFinishCount, 1);

let staleCurrentGuard = readGuard(10, 11, 12, "d".repeat(64));
let resolveStaleSnapshot;
const staleSnapshotPromise = new Promise((resolve) => { resolveStaleSnapshot = resolve; });
let staleApplyCount = 0;
const staleController = refreshController.createTimelineSnapshotRefreshController({
  captureProjectReadGuard: () => staleCurrentGuard,
  projectReadGuardIsCurrent: (candidate) => candidate === staleCurrentGuard,
  invoke: async () => staleSnapshotPromise,
  snapshotRequestGuard: { beginFull: () => undefined, finishFull: () => undefined },
  timelineTransportCanonicalSnapshotGeneration: () => 0,
  refreshOperatorPolicy: async () => undefined,
  refreshFixtureGroups: async () => undefined,
  setFixtureGroupDeleteUndoAvailable: () => undefined,
  setViewportFixtureGroupDeleteUndo: () => undefined,
  setMessage: () => undefined,
});
const staleFullResultPromise = staleController.refresh(false, false, () => {
  void staleController.run(() => {
    staleApplyCount += 1;
    return true;
  });
});
const staleReadGuard = staleCurrentGuard;
staleCurrentGuard = readGuard(11, 12, 13, "e".repeat(64));
resolveStaleSnapshot(snapshot(4, 5, 2, 2));
assert.equal(await staleFullResultPromise, null, "a full read superseded before apply must resolve stale");
assert.equal(staleApplyCount, 0, "a stale full read must never call the apply seam");
assert.notStrictEqual(staleReadGuard, staleCurrentGuard);

assert.match(
  ingressSource,
  /const readGuard = ingress\.projectReadGuard \?\? options\.captureProjectReadGuard\(\);[\s\S]*?if \(!options\.projectReadGuardIsCurrent\(readGuard\)\) return null;[\s\S]*?ingress\.resetForProjectScope[\s\S]*?timelineRuntimeSnapshotWatermark\.resetForProjectScope\(scope\)[\s\S]*?timelineRuntimeSnapshotWatermark\.canAccept\(scope, watermark\)/,
  "the ingress module must guard, reset, and admit every full image through one read-identity watermark seam",
);
assert.match(
  refreshControllerSource,
  /const requestedReadGuard = pendingFullSnapshotRefreshes\[0\]\.projectReadGuard;[\s\S]*?options\.invoke<EngineSnapshot>\("get_snapshot"\)[\s\S]*?options\.projectReadGuardIsCurrent\(requestedReadGuard\)[\s\S]*?applySnapshot\([\s\S]*?requestedReadGuard,/,
  "the full-refresh controller must revalidate the queued read guard before passing it to apply",
);

// A full A can be deferred while natural terminal / Follow publishes B through
// another ingress. The delayed full must not reset the visible renderer image.
{
  const gate = runtime.createTimelineRuntimeSnapshotWatermark();
  const scope = project();
  let visible = null;
  const fullA = deferred();
  const fullCompletion = fullA.promise.then((candidate) => {
    if (admit(gate, scope, candidate)) visible = candidate;
  });
  const naturalFollowB = snapshot(4, 31, 8, 9);
  assert(admit(gate, scope, naturalFollowB));
  visible = naturalFollowB;
  fullA.resolve(snapshot(4, 30, 7, 8));
  await fullCompletion;
  assert.strictEqual(visible, naturalFollowB, "deferred full A rewound natural/Follow B");
}

// The polling delta lane has the same rule after the strict root-loop receipt
// canonically applies B.
{
  const gate = runtime.createTimelineRuntimeSnapshotWatermark();
  const scope = project();
  let visible = null;
  const deltaA = deferred();
  const deltaCompletion = deltaA.promise.then((candidate) => {
    if (admit(gate, scope, candidate)) visible = candidate;
  });
  const strictLoopB = snapshot(5, 44, 12, 4);
  assert(admit(gate, scope, strictLoopB));
  visible = strictLoopB;
  deltaA.resolve(snapshot(5, 43, 11, 4));
  await deltaCompletion;
  assert.strictEqual(visible, strictLoopB, "deferred delta A rewound strict-loop B");
}

// Within one transport pair a later delta remains useful, but it may only
// advance the independently observed Loop/Follow watermarks.
{
  const gate = runtime.createTimelineRuntimeSnapshotWatermark();
  const scope = project();
  const initial = snapshot(6, 50, 3, 5);
  const laterSamePairDelta = snapshot(6, 50, 4, 5);
  assert(admit(gate, scope, initial));
  assert(admit(gate, scope, laterSamePairDelta), "later same-pair delta was incorrectly rejected");
  assert(!admit(gate, scope, snapshot(6, 50, 3, 5)), "same-pair loop rewind was accepted");
  assert(!admit(gate, scope, snapshot(6, 50, 4, 4)), "same-pair Follow rewind was accepted");
}

// Transport authority is lexicographic: rollover is newer even when its new
// generation is low; the old high-generation epoch is still stale.
{
  const gate = runtime.createTimelineRuntimeSnapshotWatermark();
  const scope = project();
  const maxSafe = Number.MAX_SAFE_INTEGER;
  assert(admit(gate, scope, snapshot(8, maxSafe, 30, 30)));
  assert(admit(gate, scope, snapshot(9, 1, 0, 0)), "epoch rollover was not accepted");
  assert(!admit(gate, scope, snapshot(8, maxSafe, 31, 31)), "old epoch was accepted after rollover");
}

// A coordinator project boundary explicitly resets the watermark for B. The
// old identity cannot re-enter, while B starts a fresh transport lifecycle.
{
  const gate = runtime.createTimelineRuntimeSnapshotWatermark();
  const projectA = project(21, 22, "b".repeat(64), 13);
  const projectB = project(23, 24, "c".repeat(64), 14);
  assert(admit(gate, projectA, snapshot(12, 99, 40, 41)));
  assert(gate.resetForProjectScope(projectB), "project B watermark reset failed");
  assert(!admit(gate, projectA, snapshot(13, 100, 42, 42)), "delayed old-project A was accepted");
  assert(admit(gate, projectB, snapshot(1, 1, 0, 0)), "new-project B was not accepted after reset");
}

assert.equal(
  runtime.timelineRuntimeSnapshotWatermarkFromEngineSnapshot({ timeline: {} }),
  null,
  "missing runtime fence must fail closed",
);
assert.match(
  ingressSource,
  /const readGuard = ingress\.projectReadGuard \?\? options\.captureProjectReadGuard\(\);[\s\S]*?if \(!options\.projectReadGuardIsCurrent\(readGuard\)\) return null;[\s\S]*?timelineRuntimeSnapshotWatermarkFromEngineSnapshot\(next\)[\s\S]*?timelineRuntimeSnapshotWatermark\.canAccept\(scope, watermark\)/,
  "the shared ingress module must bind current project read identity and the payload watermark before returning a snapshot",
);
assert.match(
  appSource,
  /const timelineRuntimeSnapshotIngress = createTimelineRuntimeSnapshotIngress\(\{[\s\S]*?captureProjectReadGuard,[\s\S]*?projectReadGuardIsCurrent,[\s\S]*?\}\);[\s\S]*?const prepareTimelineRuntimeSnapshotIngress = \([\s\S]*?return timelineRuntimeSnapshotIngress\.prepare\(incoming, ingress\);/,
  "App must delegate all prepared snapshot ingress to the shared watermark module",
);
assert.match(
  appSource,
  /const applyEngineSnapshotSyncResponse[\s\S]*?prepareTimelineRuntimeSnapshotIngress\(merged, \{ projectReadGuard \}\)[\s\S]*?applyAcceptedEngineSnapshot\(next, false\)/,
  "delta/poll ingress must validate before changing latest snapshot state",
);
assert.match(
  appSource,
  /const runFullSnapshotRefreshes[\s\S]*?const applied = applyEngineSnapshot\([\s\S]*?projectReadGuard: requestedReadGuard/,
  "full ingress must carry its exact project read guard into the shared seam",
);
assert.match(
  appSource,
  /resetForProjectScope: true[\s\S]*?Authoritative project snapshot was stale or missing its Timeline runtime watermark/,
  "authoritative project replacement must reset the common watermark and fail closed",
);
assert.match(
  loopIntegrationSource,
  /const readGuard = options\.captureProjectReadGuard\(\);[\s\S]*?options\.beginTimelineTransportCanonicalSnapshotConvergence\(\)[\s\S]*?options\.applyEngineSnapshot\(canonical\.snapshot, readGuard\)/,
  "canonical loop convergence must carry its pre-read guard into the shared watermark apply seam",
);

console.log("snapshot runtime watermark full/delta/poll/canonical monotonic ingress checks passed");
