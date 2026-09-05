import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";
const source = await readFile(new URL("../src/createTargetBlackoutController.ts", import.meta.url), "utf8");
const appSource = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const calls = [];
let finish;
const exports = {};
const appliedReceipt = {
  outcome: "applied",
  fence_after: {
    project_epoch: 7,
    project_revision: 8,
    project_checkpoint_hash: "c".repeat(64),
  },
};
const mock = {
  executeTargetBlackout: async (_invoke, target, enabled) => {
    calls.push([target, enabled]);
    await new Promise(resolve => { finish = resolve; });
    return appliedReceipt;
  },
  executeBlackoutRelease: async () => calls.push(["release-safety"]),
};
new Function("require", "exports", ts.transpileModule(source, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
}).outputText)(name => {
  assert.equal(name, "./outputControlController"); return mock;
}, exports);
const messages = [];
let refreshes = 0;
const controller = exports.createTargetBlackoutController({
  invoke() { throw new Error("unexpected direct IPC"); },
  safety: { engage: async () => calls.push(["engage-safety"]) },
  refreshSnapshot: async () => { refreshes++; },
  setMessage: message => messages.push(message),
});
const first = controller.setLightingBlackout(true);
await controller.setVideoBlackout(true);
await controller.releaseSafetyBlackout();
assert.deepEqual(calls, [["lighting", true]], "pending ordinary operation must not race another target or release S0");
await controller.engageSafetyBlackout();
assert.deepEqual(calls.at(-1), ["engage-safety"], "emergency engage must bypass an ordinary pending operation");
finish(); await first;
for (const [method, target] of [["setVideoBlackout", "video"], ["setAllBlackout", "both"]]) {
  const work = controller[method](false); finish(); await work;
  assert.deepEqual(calls.at(-1), [target, false]);
}
await controller.releaseSafetyBlackout();
assert.deepEqual(calls.at(-1), ["release-safety"]);
mock.executeTargetBlackout = async () => { throw new Error("rejected"); };
await controller.setLightingBlackout(false);
assert.deepEqual(messages, ["Error: rejected"]);
await controller.releaseSafetyBlackout();
assert.equal(refreshes, 6, "failed action must not pretend to refresh success; pending must reset");

const orderedEvents = [];
mock.executeTargetBlackout = async () => {
  orderedEvents.push("commit");
  return appliedReceipt;
};
const orderedController = exports.createTargetBlackoutController({
  invoke() { throw new Error("unexpected direct IPC"); },
  safety: { engage: async () => undefined },
  refreshProjectAuthority: async receipt => {
    assert.strictEqual(receipt, appliedReceipt, "canonical refresh receives the durable receipt");
    orderedEvents.push("canonical");
  },
  refreshSnapshot: async () => { orderedEvents.push("snapshot"); },
  setMessage: message => messages.push(message),
});
await orderedController.setVideoBlackout(true);
assert.deepEqual(
  orderedEvents,
  ["commit", "canonical", "snapshot"],
  "an applied target blackout must converge canonical authority before snapshot refresh",
);

const unavailableMessages = [];
let unavailableSnapshots = 0;
const unavailableController = exports.createTargetBlackoutController({
  invoke() { throw new Error("unexpected direct IPC"); },
  safety: { engage: async () => undefined },
  refreshProjectAuthority: async () => { throw new Error("canonical authority unavailable"); },
  refreshSnapshot: async () => { unavailableSnapshots += 1; },
  setMessage: message => unavailableMessages.push(message),
});
await unavailableController.setVideoBlackout(false);
assert.deepEqual(
  unavailableMessages,
  ["Error: canonical authority unavailable"],
  "canonical convergence failure remains visible and does not look like a refreshed success",
);
assert.equal(unavailableSnapshots, 0, "snapshot refresh waits for canonical authority convergence");

const convergenceSource = await readFile(
  new URL("../src/timelineFollowAuthorityConvergence.ts", import.meta.url),
  "utf8",
);
const convergenceOutput = ts.transpileModule(convergenceSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineFollowAuthorityConvergence.ts",
}).outputText;
const {
  createTimelineFollowAuthorityConvergence,
  timelineFollowRuntimeProjectChangedMessage,
} = await import(`data:text/javascript;base64,${Buffer.from(convergenceOutput).toString("base64")}`);
assert.equal(
  timelineFollowRuntimeProjectChangedMessage,
  "Project changed since this Timeline Follow runtime read was issued; retry",
  "Follow convergence is keyed to the exact native stale-read message",
);

const deferred = () => {
  let resolve;
  const promise = new Promise(resolvePromise => { resolve = resolvePromise; });
  return { promise, resolve };
};
const oldAuthority = { project_epoch: 3, project_revision: 10, checkpoint_hash: "a" };
const nextAuthority = { project_epoch: 3, project_revision: 11, checkpoint_hash: "b" };
let currentAuthority = oldAuthority;
let refreshAuthorityCalls = 0;
const pendingAuthorityRefresh = deferred();
const convergence = createTimelineFollowAuthorityConvergence({
  isCurrent: captured => captured.project_epoch === currentAuthority.project_epoch
    && captured.project_revision === currentAuthority.project_revision
    && captured.checkpoint_hash === currentAuthority.checkpoint_hash,
  inFlightAuthorityPoll: () => null,
  refreshAuthority: async () => {
    refreshAuthorityCalls += 1;
    await pendingAuthorityRefresh.promise;
  },
});
const staleReadA = convergence.converge(oldAuthority);
const staleReadB = convergence.converge(oldAuthority);
assert.strictEqual(staleReadA, staleReadB, "concurrent old Follow reads share one authority convergence");
assert.equal(refreshAuthorityCalls, 1, "one exact stale identity starts one canonical convergence");
pendingAuthorityRefresh.resolve();
assert.equal(
  await staleReadA,
  false,
  "a failed/current convergence does not claim that the old identity was superseded",
);
assert.equal(await staleReadB, false, "the concurrent stale read observes the same unsuperseded result");
currentAuthority = nextAuthority;
assert.equal(
  await convergence.converge(oldAuthority),
  true,
  "stale warning suppression becomes valid only after the current identity advances",
);

currentAuthority = oldAuthority;
let preCommitPoll = deferred();
let freshPollCalls = 0;
const freshConvergence = createTimelineFollowAuthorityConvergence({
  isCurrent: captured => captured.project_revision === currentAuthority.project_revision,
  inFlightAuthorityPoll: () => preCommitPoll.promise,
  refreshAuthority: async () => {
    freshPollCalls += 1;
    currentAuthority = nextAuthority;
  },
});
const freshAttempt = freshConvergence.converge(oldAuthority);
assert.equal(freshPollCalls, 0, "a pre-commit poll is allowed to settle before the fresh convergence poll");
preCommitPoll.resolve();
assert.equal(await freshAttempt, true, "the fresh poll advances the canonical authority");
assert.equal(freshPollCalls, 1, "the pre-commit poll cannot be mistaken for post-commit convergence");

let failedConvergenceCalls = 0;
const failedConvergence = createTimelineFollowAuthorityConvergence({
  isCurrent: captured => captured.project_revision === oldAuthority.project_revision,
  inFlightAuthorityPoll: () => null,
  refreshAuthority: async () => {
    failedConvergenceCalls += 1;
    throw new Error("canonical authority read failed");
  },
});
assert.equal(await failedConvergence.converge(oldAuthority), false, "failed authority read remains unsuperseded");
assert.equal(await failedConvergence.converge(oldAuthority), false, "the same stale identity is not retried repeatedly");
assert.equal(failedConvergenceCalls, 1, "failed canonical convergence is one-shot per stale identity");

assert.equal(await failedConvergence.suppressStaleReadError("transport unavailable", oldAuthority), false);
assert.equal(failedConvergenceCalls, 1, "unrelated errors must not start convergence");
assert.equal(await failedConvergence.suppressStaleReadError(timelineFollowRuntimeProjectChangedMessage, oldAuthority), false);
currentAuthority = nextAuthority;
assert.equal(await convergence.suppressStaleReadError(timelineFollowRuntimeProjectChangedMessage, oldAuthority), true);
assert.equal(await convergence.suppressStaleReadError("transport unavailable", oldAuthority), false,
  "even a superseded identity cannot turn unrelated errors into a stale-read match");
assert.match(appSource, /if \(await suppressStaleTimelineFollowReadError\(detail, authority\)\) return null;/,
  "the UI delegates the exact error and captured identity to the convergence policy");
assert.match(
  appSource,
  /if \(showError && isProjectAuthorityIdentityCurrent\(authority\) && detail !== timelineFollowRuntimeLastError\)/,
  "current or failed convergence retains the existing explicit Follow warning path",
);
console.log("target blackout controller: PASS (canonical ordering, failure visibility, and bounded Timeline Follow stale-read convergence)");
