import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [watermarkSource, runtimeWireSource, ingressSource, refreshControllerSource, loopIntegrationSource, appSource] = await Promise.all([
  readFile(new URL("../src/timelineRuntimeSnapshotWatermark.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/timelineRuntimeSnapshotWire.ts", import.meta.url), "utf8"),
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
const runtimeWireModuleUrl = dataModule(runtimeWireSource, "timelineRuntimeSnapshotWire.ts");
const transpiled = ts.transpileModule(watermarkSource, {
  compilerOptions,
  fileName: "timelineRuntimeSnapshotWatermark.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);
const runtimeWireParser = await import(runtimeWireModuleUrl);
const ingressTranspiled = ts.transpileModule(ingressSource, {
  compilerOptions,
  fileName: "timelineRuntimeSnapshotIngress.ts",
});
const ingress = await import(
  `data:text/javascript;base64,${Buffer.from(
    ingressTranspiled.outputText
      .replace("./timelineRuntimeSnapshotWatermark", watermarkModuleUrl)
      .replace("./timelineRuntimeSnapshotWire", runtimeWireModuleUrl),
  ).toString("base64")}`,
);
const refreshController = await import(dataModule(
  refreshControllerSource.replace("./timelineRuntimeSnapshotWire", runtimeWireModuleUrl),
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
    loop_runtime: { generation: loopGeneration, status: "disabled", a_ms: null, b_ms: null, wrap_count: 0 },
    follow_runtime: {
      epoch,
      generation: followGeneration,
      status: "idle",
      admission_reason: null,
      outcome: null,
      source_timeline_id: null,
      target_timeline_id: null,
      elapsed_ms: 0,
      duration_ms: 0,
      progress_millis: 0,
      fault: null,
      transition_hold_active: false,
      waiting_for_pedal_start: false,
    },
  },
});

const runtimeWire = (candidate) => ({
  transport_epoch: candidate.timeline.transport_epoch,
  transport_generation: candidate.timeline.transport_generation,
  loop_runtime: candidate.timeline.loop_runtime,
  follow_runtime: candidate.timeline.follow_runtime,
});

const fullWire = (candidate) => ({ snapshot: candidate, timeline_runtime: runtimeWire(candidate) });

const freshRuntimeWire = fullWire(snapshot(1, 1, 0, 0));
const parsedFreshRuntimeWire = runtimeWireParser.engineSnapshotRuntimeWireResponseFromUnknown(freshRuntimeWire);
assert.notEqual(parsedFreshRuntimeWire, null, "fresh 1/1 transport with zero loop/follow generations must remain wire-present");
assert.equal(parsedFreshRuntimeWire.snapshot.timeline.loop_runtime.generation, 0);
assert.equal(parsedFreshRuntimeWire.snapshot.timeline.follow_runtime.generation, 0);
assert.equal(
  runtimeWireParser.engineSnapshotRuntimeWireResponseFromUnknown({ snapshot: freshRuntimeWire.snapshot }),
  null,
  "a direct full response without the engine-owned runtime projection must fail closed",
);
assert.equal(
  runtimeWireParser.timelineRuntimeSnapshotWireFromUnknown({
    ...freshRuntimeWire.timeline_runtime,
    follow_runtime: { ...freshRuntimeWire.timeline_runtime.follow_runtime, epoch: 2 },
  }),
  null,
  "a follow runtime epoch that disagrees with transport must fail closed",
);
const settledRuntimeWire = {
  ...freshRuntimeWire.timeline_runtime,
  follow_runtime: {
    ...freshRuntimeWire.timeline_runtime.follow_runtime,
    status: "aborting",
    admission_reason: "natural_playback_boundary",
    outcome: { kind: "aborted", reason: "manual_seek" },
    source_timeline_id: 7,
    target_timeline_id: 8,
    fault: "operator seek",
    settlement: {
      started_at_ms: 100,
      deadline_ms: 2100,
      state: "fault",
      progress_millis: 1000,
      fault_policy: "hold",
      fault: "video fault",
      domains: [
        {
          domain: "audio",
          state: "applied",
          consumers: [{ consumer_id: { kind: "audio" }, state: "applied" }],
        },
        {
          domain: "video",
          state: "fault",
          fault: "present failed",
          consumers: [{
            consumer_id: { kind: "video_output", output_id: 9 },
            state: "fault",
            fault: "present failed",
          }],
        },
        { domain: "lighting", state: "not_applicable" },
      ],
    },
  },
};
assert.notEqual(
  runtimeWireParser.timelineRuntimeSnapshotWireFromUnknown(settledRuntimeWire),
  null,
  "the parser must accept the exact Rust Follow status/settlement projection",
);
for (const [name, malformed] of [
  ["future admission", {
    ...settledRuntimeWire,
    follow_runtime: { ...settledRuntimeWire.follow_runtime, admission_reason: "future_admission" },
  }],
  ["future outcome", {
    ...settledRuntimeWire,
    follow_runtime: { ...settledRuntimeWire.follow_runtime, outcome: { kind: "future_outcome" } },
  }],
  ["wrong TimelineId", {
    ...settledRuntimeWire,
    follow_runtime: { ...settledRuntimeWire.follow_runtime, source_timeline_id: "7" },
  }],
  ["wrong Follow fault", {
    ...settledRuntimeWire,
    follow_runtime: { ...settledRuntimeWire.follow_runtime, fault: { message: "operator seek" } },
  }],
  ["null settlement", {
    ...settledRuntimeWire,
    follow_runtime: { ...settledRuntimeWire.follow_runtime, settlement: null },
  }],
  ["future settlement state", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: { ...settledRuntimeWire.follow_runtime.settlement, state: "future_state" },
    },
  }],
  ["settlement deadline before start", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: { ...settledRuntimeWire.follow_runtime.settlement, deadline_ms: 99 },
    },
  }],
  ["missing settlement domain", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: settledRuntimeWire.follow_runtime.settlement.domains.slice(0, 2),
      },
    },
  }],
  ["duplicate settlement video domain with distinct output consumer", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: [
          ...settledRuntimeWire.follow_runtime.settlement.domains,
          {
            ...settledRuntimeWire.follow_runtime.settlement.domains[1],
            consumers: [{ consumer_id: { kind: "video_output", output_id: 10 }, state: "fault", fault: "present failed" }],
          },
        ],
      },
    },
  }],
  ["consumer assigned to wrong domain", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: [{
          ...settledRuntimeWire.follow_runtime.settlement.domains[0],
          consumers: [{ consumer_id: { kind: "video_output", output_id: 9 }, state: "applied" }],
        }, ...settledRuntimeWire.follow_runtime.settlement.domains.slice(1)],
      },
    },
  }],
  ["non-fault settlement state carrying fault", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: [{ ...settledRuntimeWire.follow_runtime.settlement.domains[0], fault: "unexpected" },
          ...settledRuntimeWire.follow_runtime.settlement.domains.slice(1)],
      },
    },
  }],
  ["fault settlement state without fault", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: [
          ...settledRuntimeWire.follow_runtime.settlement.domains.slice(0, 1),
          {
            ...settledRuntimeWire.follow_runtime.settlement.domains[1],
            fault: undefined,
            consumers: [{
              ...settledRuntimeWire.follow_runtime.settlement.domains[1].consumers[0],
              fault: undefined,
            }],
          },
          ...settledRuntimeWire.follow_runtime.settlement.domains.slice(2),
        ],
      },
    },
  }],
  ["contradictory aggregate settlement state", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: { ...settledRuntimeWire.follow_runtime.settlement, state: "applied", fault: undefined },
    },
  }],
  ["unknown settlement consumer field", {
    ...settledRuntimeWire,
    follow_runtime: {
      ...settledRuntimeWire.follow_runtime,
      settlement: {
        ...settledRuntimeWire.follow_runtime.settlement,
        domains: [{
          ...settledRuntimeWire.follow_runtime.settlement.domains[0],
          consumers: [{
            ...settledRuntimeWire.follow_runtime.settlement.domains[0].consumers[0],
            future_field: true,
          }],
        }],
      },
    },
  }],
]) {
  assert.equal(runtimeWireParser.timelineRuntimeSnapshotWireFromUnknown(malformed), null, `${name} must fail closed`);
}
assert.equal(
  runtimeWireParser.projectAuthorityBundleTimelineRuntimeFromUnknown({
    timeline_transport_epoch: 2,
    timeline_transport_generation: 1,
    timeline_runtime: freshRuntimeWire.timeline_runtime,
  }),
  null,
  "a ProjectAuthorityBundle outer transport pair may not disagree with its runtime projection",
);

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
assert.deepEqual(ingressRuntime.prepare(ingressA, { timelineRuntime: runtimeWire(ingressA) }), ingressA, "current full ingress must be admitted");
const staleIngressGuard = ingressCurrentGuard;
ingressCurrentGuard = readGuard(2, 3, 4, "b".repeat(64));
assert.equal(
  ingressRuntime.prepare(ingressA, { projectReadGuard: staleIngressGuard, timelineRuntime: runtimeWire(ingressA) }),
  null,
  "a full image from the superseded project-read guard must remain unapplied",
);
const ingressB = snapshot(1, 1, 0, 0);
assert.equal(
  ingressRuntime.prepare(ingressB, { projectReadGuard: ingressCurrentGuard }),
  null,
  "a native ingress without its runtime projection must fail closed",
);
assert.deepEqual(
  ingressRuntime.prepare(ingressB, { projectReadGuard: ingressCurrentGuard, resetForProjectScope: true, timelineRuntime: runtimeWire(ingressB) }),
  ingressB,
  "an authoritative replacement must reset the shared watermark for its new scope",
);
assert.equal(
  ingressRuntime.prepare(ingressA, { projectReadGuard: staleIngressGuard, timelineRuntime: runtimeWire(ingressA) }),
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
    return fullWire(canonicalCandidate);
  },
  isTauriRuntime: () => true,
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
  void canonicalController.run((candidate, timelineRuntime, syncProjectState, resetEditorDrafts, projectReadGuard) => {
    canonicalApplies.push({ candidate, timelineRuntime, syncProjectState, resetEditorDrafts, projectReadGuard });
    return true;
  });
});
assert.deepEqual(await canonicalResultPromise, canonicalCandidate, "a current full read must resolve its canonical candidate");
assert.equal(canonicalApplies.length, 1, "a current full read must cross the guarded apply seam once");
assert.strictEqual(canonicalApplies[0].projectReadGuard, canonicalGuard, "full apply must receive the exact queued read guard");
assert.deepEqual(canonicalApplies[0].timelineRuntime, runtimeWire(canonicalCandidate), "full apply must receive the backend runtime projection");
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
  isTauriRuntime: () => true,
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
resolveStaleSnapshot(fullWire(snapshot(4, 5, 2, 2)));
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
  /const requestedReadGuard = pendingFullSnapshotRefreshes\[0\]\.projectReadGuard;[\s\S]*?options\.invoke<unknown>\("get_snapshot"\)[\s\S]*?engineSnapshotRuntimeWireResponseFromUnknown\(response\)[\s\S]*?options\.projectReadGuardIsCurrent\(requestedReadGuard\)[\s\S]*?applySnapshot\([\s\S]*?candidate\.timeline_runtime[\s\S]*?requestedReadGuard,/,
  "the full-refresh controller must require and carry the backend runtime projection with its queued read guard",
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
  /const applyEngineSnapshotSyncResponse[\s\S]*?prepareTimelineRuntimeSnapshotIngress\(merged, \{[\s\S]*?projectReadGuard,[\s\S]*?timelineRuntime: response\.timeline_runtime,[\s\S]*?\}\)[\s\S]*?applyAcceptedEngineSnapshot\(next, false\)/,
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
  /const readGuard = options\.captureProjectReadGuard\(\);[\s\S]*?options\.beginTimelineTransportCanonicalSnapshotConvergence\(\)[\s\S]*?projectAuthorityBundleTimelineRuntimeFromUnknown\(canonical\)[\s\S]*?hydrateTimelineRuntimeSnapshot\(canonical\.snapshot, timelineRuntime\)[\s\S]*?timelineRuntime\.transport_epoch !== acknowledgement\.epochAfter[\s\S]*?options\.applyEngineSnapshot\(canonical\.snapshot, timelineRuntime, readGuard\)/,
  "canonical loop convergence must cross-check the bundle pair and compare the hydrated runtime projection to the receipt",
);
assert.match(
  appSource,
  /const refreshTimelineTransportCanonicalSnapshot[\s\S]*?projectAuthorityBundleTimelineRuntimeFromUnknown\(canonical\)[\s\S]*?timelineRuntime\.transport_epoch !== acknowledgement\.epochAfter[\s\S]*?timelineRuntime\.transport_generation !== acknowledgement\.generationAfter[\s\S]*?timelineRuntime,\s*\}\)/,
  "transport canonical convergence must compare the validated runtime projection, not redundant outer fields, to its receipt",
);
assert.match(
  appSource,
  /const applyProjectAuthorityBundle = \([\s\S]*?projectAuthorityBundleTimelineRuntimeFromUnknown\(bundle\)[\s\S]*?commitBundle: \(candidate, prepared, options\) => \{[\s\S]*?projectAuthorityBundleTimelineRuntimeFromUnknown\(candidate\)[\s\S]*?timelineRuntime: candidateTimelineRuntime/,
  "every authority-bundle application must reject a mismatched outer/runtime transport pair before ingress",
);

console.log("snapshot runtime watermark full/delta/poll/canonical monotonic ingress checks passed");
