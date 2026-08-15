import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const controllerSource = await readFile(new URL("../src/authoredEffectEnableController.ts", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "authoredEffectEnableController.ts",
});
const helpers = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);

const hash = "a".repeat(64);
const fence = {
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: 3,
  project_revision: 4,
  project_checkpoint_hash: hash,
  project_publication_generation: 5,
};
const appliedResponse = {
  kind: "terminal_receipt",
  result: {
    operation_id: helpers.authoredSetEffectEnabledOperationId,
    request_id: 8,
    start_fence: fence,
    shape_sha256: "b".repeat(64),
    outcome: {
      kind: "applied",
      result: {
        effect_id: 7,
        enabled: true,
        post_fence: fence,
      },
    },
  },
};
helpers.validateAuthoredSetEffectEnabledResponse(appliedResponse, 8, fence, 7, true);
assert.throws(
  () => helpers.validateAuthoredSetEffectEnabledResponse(
    { ...appliedResponse, kind: "future_terminal_receipt" },
    8,
    fence,
    7,
    true,
  ),
  /invalid terminal receipt/,
  "unknown top-level response discriminators must fail closed",
);
assert.throws(
  () => helpers.validateAuthoredSetEffectEnabledResponse({
    ...appliedResponse,
    result: {
      ...appliedResponse.result,
      outcome: { ...appliedResponse.result.outcome, kind: "future_outcome" },
    },
  }, 8, fence, 7, true),
  /invalid outcome receipt/,
  "unknown terminal outcome discriminators must fail closed",
);
assert.equal(
  helpers.isAuthoredProjectMutationFence({
    ...fence,
    project_publication_generation: Number.MAX_SAFE_INTEGER + 1,
  }),
  false,
  "renderer must reject a lossy inbound fence counter",
);

const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((resolvePromise, rejectPromise) => {
    resolve = resolvePromise;
    reject = rejectPromise;
  });
  return { promise, resolve, reject };
};
const tickUntil = async (predicate, message) => {
  for (let index = 0; index < 40; index += 1) {
    if (predicate()) return;
    await Promise.resolve();
  }
  assert.fail(message);
};

// Deferred A/B proves that a late completion from A cannot surface or replace
// B, and that the rendered control's callback results in one strict dispatch
// for each distinct current intent.
const calls = [];
const completions = [];
const pending = [];
const queued = [];
let pendingStoreCaptureTargets = [{ effect_id: 7, enabled: false }];
const controller = helpers.createAuthoredEffectEnableIntentController({
  seedSettledBaseline: () => false,
  dispatch: (intent, attempt) => {
    calls.push({ ...intent, attempt });
    const operation = deferred();
    pending.push(operation);
    return operation.promise;
  },
  isStaleFence: (error) => error?.code === "stale_fence",
  onQueued: (intent) => {
    queued.push(intent);
    pendingStoreCaptureTargets = pendingStoreCaptureTargets.map((target) =>
      target.effect_id === intent.effectId ? { ...target, enabled: intent.enabled } : target,
    );
  },
  onApplied: (intent) => completions.push(intent),
  onFailed: (intent, error) => assert.fail(`unexpected failed intent ${intent.effectId}/${intent.enabled}: ${error}`),
});
const a = controller.enqueue(7, true);
await tickUntil(() => calls.length === 1, "A should dispatch once");
assert.deepEqual(
  pendingStoreCaptureTargets,
  [{ effect_id: 7, enabled: true }],
  "Store Recall capture must expose A while its strict invoke is still pending",
);
const b = controller.enqueue(7, false);
assert.deepEqual(
  pendingStoreCaptureTargets,
  [{ effect_id: 7, enabled: false }],
  "a newer B must immediately replace pending A in Store Recall capture",
);
pending[0].resolve();
await tickUntil(() => calls.length === 2, "B should dispatch after A settles");
assert.deepEqual(calls, [
  { effectId: 7, enabled: true, attempt: 0 },
  { effectId: 7, enabled: false, attempt: 0 },
]);
assert.deepEqual(queued, [
  { effectId: 7, enabled: true, generation: 1 },
  { effectId: 7, enabled: false, generation: 2 },
]);
pending[1].resolve();
await Promise.all([a, b]);
assert.deepEqual(completions, [{ effectId: 7, enabled: false, generation: 2 }]);
assert.equal(controller.isPending(7), false);

// A stale completion from a superseded A must not refresh/retry A. Only the
// current B is dispatched, and B is the only terminal UI application.
const staleCalls = [];
const staleCompletions = [];
const stalePending = [];
const staleFailures = [];
const staleSuperseded = helpers.createAuthoredEffectEnableIntentController({
  seedSettledBaseline: () => false,
  dispatch: (intent, attempt) => {
    staleCalls.push({ ...intent, attempt });
    const operation = deferred();
    stalePending.push(operation);
    return operation.promise;
  },
  isStaleFence: (error) => error?.code === "stale_fence",
  onApplied: (intent) => staleCompletions.push(intent),
  onFailed: (intent, error) => staleFailures.push({ intent, error }),
});
const staleA = staleSuperseded.enqueue(7, true);
await tickUntil(() => staleCalls.length === 1, "stale A should dispatch once");
const staleB = staleSuperseded.enqueue(7, false);
stalePending[0].reject({ code: "stale_fence" });
await tickUntil(() => staleCalls.length === 2, "superseding B should dispatch without retrying stale A");
assert.deepEqual(staleCalls, [
  { effectId: 7, enabled: true, attempt: 0 },
  { effectId: 7, enabled: false, attempt: 0 },
]);
stalePending[1].resolve();
await Promise.all([staleA, staleB]);
assert.deepEqual(staleCompletions, [{ effectId: 7, enabled: false, generation: 2 }]);
assert.deepEqual(staleFailures, [], "a superseded stale A must not trigger an old-intent rollback");

// A/B/A coalesces when the in-flight original A succeeds: B never reaches
// the backend and the newest A generation is the only completion surfaced.
const coalescedCalls = [];
const coalescedCompletions = [];
const coalescedPending = [];
const coalesced = helpers.createAuthoredEffectEnableIntentController({
  seedSettledBaseline: () => false,
  dispatch: (intent, attempt) => {
    coalescedCalls.push({ ...intent, attempt });
    const operation = deferred();
    coalescedPending.push(operation);
    return operation.promise;
  },
  isStaleFence: (error) => error?.code === "stale_fence",
  onApplied: (intent) => coalescedCompletions.push(intent),
});
const coalescedA1 = coalesced.enqueue(7, true);
await tickUntil(() => coalescedCalls.length === 1, "first A should dispatch once");
const coalescedB = coalesced.enqueue(7, false);
const coalescedA2 = coalesced.enqueue(7, true);
coalescedPending[0].resolve();
await Promise.all([coalescedA1, coalescedB, coalescedA2]);
assert.deepEqual(coalescedCalls, [{ effectId: 7, enabled: true, attempt: 0 }]);
assert.deepEqual(coalescedCompletions, [{ effectId: 7, enabled: true, generation: 3 }]);
assert.equal(coalesced.isPending(7), false);

// Current stale recovery refreshes through exactly one retry and is bounded.
const retryCalls = [];
const retryCompletions = [];
const retry = helpers.createAuthoredEffectEnableIntentController({
  seedSettledBaseline: () => false,
  dispatch: async (intent, attempt) => {
    retryCalls.push({ ...intent, attempt });
    if (attempt === 0) throw { code: "stale_fence" };
  },
  isStaleFence: (error) => error?.code === "stale_fence",
  onApplied: (intent) => retryCompletions.push(intent),
  maxAttempts: 2,
});
await retry.enqueue(7, true);
assert.deepEqual(retryCalls, [
  { effectId: 7, enabled: true, attempt: 0 },
  { effectId: 7, enabled: true, attempt: 1 },
]);
assert.deepEqual(retryCompletions, [{ effectId: 7, enabled: true, generation: 1 }]);

// Every attempt can become stale, but the fixed retry budget must terminate
// the lane exactly once rather than creating an unbounded invoke loop.
const exhaustedCalls = [];
const exhaustedFailures = [];
const exhausted = helpers.createAuthoredEffectEnableIntentController({
  seedSettledBaseline: () => false,
  dispatch: async (intent, attempt) => {
    exhaustedCalls.push({ ...intent, attempt });
    throw { code: "stale_fence" };
  },
  isStaleFence: (error) => error?.code === "stale_fence",
  onFailed: (intent, error, settledEnabled) => exhaustedFailures.push({ intent, error, settledEnabled }),
  maxAttempts: 2,
});
await exhausted.enqueue(7, true);
assert.deepEqual(exhaustedCalls, [
  { effectId: 7, enabled: true, attempt: 0 },
  { effectId: 7, enabled: true, attempt: 1 },
]);
assert.equal(exhaustedFailures.length, 1, "all-stale exhaustion must report one terminal failure");
assert.deepEqual(exhaustedFailures[0].intent, { effectId: 7, enabled: true, generation: 1 });
assert.equal(exhaustedFailures[0].settledEnabled, false, "all-stale retries must preserve the seeded settled baseline");
assert.equal(exhausted.isPending(7), false, "all-stale exhaustion must leave no retrying lane");

// This is the same current-generation/current-optimistic guard used by the
// real App Store Recall callback. It makes the settled baseline observable
// without treating an optimistic target as the next intent's baseline.
const createStoreRecallLaneHarness = (initialAuthoritativeEnabled) => {
  let authoritativeEnabled = initialAuthoritativeEnabled;
  let storeCaptureEnabled = initialAuthoritativeEnabled;
  const latestByEffect = new Map();
  const seedValues = [];
  const calls = [];
  const pending = [];
  const failures = [];
  const controller = helpers.createAuthoredEffectEnableIntentController({
    seedSettledBaseline: (effectId) => {
      assert.equal(effectId, 7);
      seedValues.push(authoritativeEnabled);
      return authoritativeEnabled;
    },
    dispatch: (intent, attempt) => {
      calls.push({ ...intent, attempt });
      const operation = deferred();
      pending.push(operation);
      return operation.promise;
    },
    isStaleFence: (error) => error?.code === "stale_fence",
    onQueued: (intent) => {
      latestByEffect.set(intent.effectId, {
        generation: intent.generation,
        requestedEnabled: intent.enabled,
      });
      storeCaptureEnabled = intent.enabled;
    },
    onApplied: (intent) => {
      if (latestByEffect.get(intent.effectId)?.generation === intent.generation) {
        latestByEffect.delete(intent.effectId);
      }
    },
    onFailed: (intent, error, settledEnabled) => {
      const latest = latestByEffect.get(intent.effectId);
      if (latest?.generation === intent.generation && storeCaptureEnabled === latest.requestedEnabled) {
        storeCaptureEnabled = settledEnabled;
      }
      if (latest?.generation === intent.generation) latestByEffect.delete(intent.effectId);
      failures.push({ intent, error, settledEnabled });
    },
  });
  return {
    controller,
    calls,
    failures,
    pending,
    seedValues,
    getStoreCaptureEnabled: () => storeCaptureEnabled,
    setAuthoritativeEnabled: (enabled) => { authoritativeEnabled = enabled; },
  };
};

// Initial authoritative false remains the rollback baseline when superseded
// A=true fails and current B=false then fails. B must not use optimistic A.
const failedAThenB = createStoreRecallLaneHarness(false);
const failedA = failedAThenB.controller.enqueue(7, true);
await tickUntil(() => failedAThenB.calls.length === 1, "baseline A should dispatch once");
const failedB = failedAThenB.controller.enqueue(7, false);
assert.equal(failedAThenB.getStoreCaptureEnabled(), false, "B is the latest optimistic Store Recall value");
failedAThenB.pending[0].reject({ code: "publication_failed" });
await tickUntil(() => failedAThenB.calls.length === 2, "B should follow superseded failed A");
failedAThenB.pending[1].reject({ code: "publication_failed" });
await Promise.all([failedA, failedB]);
assert.deepEqual(failedAThenB.seedValues, [false], "idle lane must seed initial authoritative false once");
assert.equal(failedAThenB.getStoreCaptureEnabled(), false, "B failure must roll Store Recall back to authoritative false");
assert.deepEqual(
  failedAThenB.failures.map(({ intent, settledEnabled }) => ({ intent, settledEnabled })),
  [{ intent: { effectId: 7, enabled: false, generation: 2 }, settledEnabled: false }],
  "only current B receives terminal failure with unchanged false baseline",
);

// A=true may settle successfully even after B=false supersedes it. B's later
// failure must use A's real server settlement (true), not B's optimistic false.
const succeededAThenFailedB = createStoreRecallLaneHarness(false);
const succeededA = succeededAThenFailedB.controller.enqueue(7, true);
await tickUntil(() => succeededAThenFailedB.calls.length === 1, "successful A should dispatch once");
const failedAfterA = succeededAThenFailedB.controller.enqueue(7, false);
succeededAThenFailedB.pending[0].resolve();
await tickUntil(() => succeededAThenFailedB.calls.length === 2, "B should dispatch after superseded A settles");
succeededAThenFailedB.pending[1].reject({ code: "publication_failed" });
await Promise.all([succeededA, failedAfterA]);
assert.equal(succeededAThenFailedB.getStoreCaptureEnabled(), true, "B failure must restore successful superseded A baseline");
assert.deepEqual(
  succeededAThenFailedB.failures.map(({ intent, settledEnabled }) => ({ intent, settledEnabled })),
  [{ intent: { effectId: 7, enabled: false, generation: 2 }, settledEnabled: true }],
  "a superseded success updates the current B failure baseline",
);

// Once a lane is idle, a later external snapshot reseeds it. The first lane
// settled true; an external false snapshot must win the next idle seed.
const externalReseed = createStoreRecallLaneHarness(false);
const externalFirst = externalReseed.controller.enqueue(7, true);
await tickUntil(() => externalReseed.calls.length === 1, "first idle lane should dispatch once");
externalReseed.pending[0].resolve();
await externalFirst;
externalReseed.setAuthoritativeEnabled(false);
const externalSecond = externalReseed.controller.enqueue(7, true);
await tickUntil(() => externalReseed.calls.length === 2, "externally reseeded idle lane should dispatch once");
externalReseed.pending[1].reject({ code: "publication_failed" });
await externalSecond;
assert.deepEqual(externalReseed.seedValues, [false, false]);
assert.equal(externalReseed.getStoreCaptureEnabled(), false, "external idle reseed must replace prior settled true after snapshot refresh");

assert.match(
  appSource,
  /data-authored-effect-enable-control[\s\S]*?data-authored-effect-enable-toggle=\{effect\(\)\.id\}[\s\S]*?onChange=\{\(event\) => void setEffectEnabled\(effect\(\)\.id, event\.currentTarget\.checked\)\}/,
  "the rendered global-effect checkbox must call the canonical callback",
);
assert.match(
  appSource,
  /const setEffectEnabled = \(effectId: number, enabled: boolean\) => \{[\s\S]*?authoredEffectEnableIntents\.enqueue\(effectId, enabled\)/,
  "the production callback must enter the per-effect strict intent controller",
);
assert.match(
  appSource,
  /const authoredEffectEnableIntents = createAuthoredEffectEnableIntentController\([\s\S]*?invoke<unknown>\("set_effect_enabled", \{[\s\S]*?__authoredEffectFencePrepared: true[\s\S]*?validateAuthoredSetEffectEnabledResponse\([\s\S]*?await refreshSnapshot\(\)/,
  "the only production callback path must issue the strict server-authoritative invoke",
);
assert.equal(
  (appSource.match(/invoke<unknown>\("set_effect_enabled"/g) ?? []).length,
  1,
  "one rendered canonical action must emit one strict set_effect_enabled invoke",
);
const genericMutationBlock = appSource.slice(
  appSource.indexOf("const projectMutationCommands = new Set(["),
  appSource.indexOf("const serverAuthoritativeProjectMutationCommands = new Set(["),
);
assert.doesNotMatch(
  genericMutationBlock,
  /"set_effect_enabled"/,
  "canonical set_effect_enabled must not reopen the renderer Begin/Commit wrapper",
);

console.log("authored effect enable controller and rendered strict route checks passed");
