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
const transpileCommonJs = (text, fileName) => ts.transpileModule(text, {
  compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
  fileName,
}).outputText;
const driveExports = {};
new Function("require", "exports", transpileCommonJs(
  await readFile(new URL("../src/outputDriveStatus.ts", import.meta.url), "utf8"), "outputDriveStatus.ts",
))(name => { assert.fail(`outputDriveStatus must stay dependency-free, required ${name}`); }, driveExports);
new Function("require", "exports", transpileCommonJs(source, "createTargetBlackoutController.ts"))(name => {
  if (name === "./outputDriveStatus") return driveExports;
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
// Standby / no output authority: the blackout is explained before any command is
// sent, the S0 latch release stays reachable, and the refusal retires on state change.
{
  const standbyCalls = [];
  const standbyMessages = [];
  const cleared = [];
  let drive = driveExports.classifyOutputDrive({ state: "Ready", effective_role: "Standby", desired_role: "Standby" }, false);
  assert.deepEqual(drive, { kind: "blocked", reason: "standby", message: "Standby: this machine is not driving outputs" });
  assert.equal(driveExports.outputDriveBlockedMessage(drive), "Standby: this machine is not driving outputs");
  assert.equal(driveExports.classifyOutputDrive({ state: "Ready", effective_role: "Both", desired_role: "Both" }, false).reason, "no_lease");
  assert.equal(driveExports.classifyOutputDrive({ state: "Transitioning", effective_role: "Standby", desired_role: "Both" }, false).reason, "fenced");
  assert.deepEqual(
    driveExports.classifyOutputDrive({ state: "Ready", effective_role: "Standby", desired_role: "Standby" }, true),
    { kind: "driving" },
    "a held Lighting+Video lease always means this machine drives outputs",
  );
  mock.executeTargetBlackout = async (_invoke, target, enabled) => { standbyCalls.push([target, enabled]); return appliedReceipt; };
  mock.executeBlackoutRelease = async () => { standbyCalls.push(["release-safety"]); return appliedReceipt; };
  let latched = false;
  const standby = exports.createTargetBlackoutController({
    invoke() { throw new Error("unexpected direct IPC"); },
    safety: { engage: async () => undefined },
    safetyBlackoutEngaged: () => latched,
    refreshSnapshot: async () => undefined,
    setMessage: (message, key) => standbyMessages.push([message, key]),
    clearMessage: key => cleared.push(key),
    checkOutputDrive: async () => drive,
  });
  for (const method of ["setLightingBlackout", "setVideoBlackout", "setAllBlackout"]) {
    await standby[method](true);
    await standby[method](false);
  }
  assert.deepEqual(standbyCalls, [], "no OutputControl command is sent while this machine cannot act");
  assert.equal(standbyMessages.length, 6);
  assert.ok(standbyMessages.every(([message, key]) => key === "blackout-unavailable"
    && message === "No blackout was applied. Standby: this machine is not driving outputs."),
  "every refusal is one clear keyed message");
  assert.deepEqual(cleared, [], "a refused blackout never claims success");
  // The runtime S0 latch release is never hidden by the role check.
  latched = true;
  await standby.setLightingBlackout(false);
  assert.deepEqual(standbyCalls, [["release-safety"], ["lighting", false]], "S0 latch release stays reachable");
  latched = false;
  standbyCalls.length = 0;
  // Driving machines are never blocked, and unknown never blocks.
  for (const reachable of [{ kind: "driving" }, { kind: "unknown" }]) {
    drive = reachable;
    await standby.setVideoBlackout(true);
  }
  assert.deepEqual(standbyCalls, [["video", true], ["video", true]]);
  assert.deepEqual(cleared, ["blackout-unavailable", "blackout-unavailable", "blackout-unavailable"], "success retires a stale refusal");
  // A refusal that still happens after the pre-check explains the role instead of the raw lease error.
  standbyMessages.length = 0;
  let checks = 0;
  const racing = exports.createTargetBlackoutController({
    invoke() { throw new Error("unexpected direct IPC"); },
    safety: { engage: async () => undefined },
    refreshSnapshot: async () => undefined,
    setMessage: (message, key) => standbyMessages.push([message, key]),
    checkOutputDrive: async () => (++checks === 1 ? { kind: "driving" } : driveExports.classifyOutputDrive({ state: "Ready", effective_role: "Standby", desired_role: "Standby" }, false)),
  });
  mock.executeTargetBlackout = async () => { throw new Error("Exactly one active output lease with the requested resources is required; nothing was applied."); };
  await racing.setVideoBlackout(true);
  assert.deepEqual(standbyMessages, [["No blackout was applied. Standby: this machine is not driving outputs.", "blackout-unavailable"]]);
}
const standbyWiring = {
  app: appSource,
  chrome: await readFile(new URL("../src/components/WorkspaceChrome.tsx", import.meta.url), "utf8"),
  desk: await readFile(new URL("../src/components/VideoPerformDesk.tsx", import.meta.url), "utf8"),
};
assert.match(standbyWiring.app, /checkOutputDrive: readOutputDrive/, "the one blackout controller receives the fresh output-authority check");
assert.match(standbyWiring.app, /const drive = await readOutputDrive\(\);\s*if \(drive\.kind === "blocked"\)[\s\S]*?confirmDialog\(blackoutDialog\(enabled\)\)/, "the role is explained before the confirmation dialog opens");
assert.match(standbyWiring.app, /blackoutBlockedReason=\{blackoutBlockedReason\(\)\}/, "top-bar DMX/VID/ALL keys receive the reason");
assert.equal((standbyWiring.chrome.match(/aria-disabled=\{props\.blackoutBlockedReason \? "true" : undefined\}/g) ?? []).length, 3, "DMX, VID and ALL keys expose the reason without disappearing");
assert.match(standbyWiring.desk, /aria-disabled=\{blackoutBlockedReason\(\) \? "true" : undefined\}/, "Perform desk Blackout exposes the reason");
assert.doesNotMatch(standbyWiring.chrome + standbyWiring.desk, /\sdisabled=\{[^}]*blackoutBlockedReason/, "emergency keys are never removed from the tab order");
console.log("target blackout controller: PASS (canonical ordering, failure visibility, and bounded Timeline Follow stale-read convergence)");

// One DMX control may clear the internal runtime latch first. Its second
// action must retain that exact project fence and report partial progress.
const combinedEvents=[];const combinedMessages=[];
mock.executeBlackoutRelease=async()=>{combinedEvents.push('runtime-release');return appliedReceipt;};
mock.executeTargetBlackout=async(_invoke,target,enabled,options)=>{combinedEvents.push([target,enabled,options.expectedProject]);return appliedReceipt;};
const combined=exports.createTargetBlackoutController({invoke(){},safety:{engage:async()=>{}},safetyBlackoutEngaged:()=>true,refreshSnapshot:async()=>{},setMessage:m=>combinedMessages.push(m)});
await combined.setLightingBlackout(false);
assert.deepEqual(combinedEvents,['runtime-release',['lighting',false,{project_epoch:7,project_revision:8,checkpoint_hash:'c'.repeat(64)}]]);
combinedEvents.length=0;await combined.setVideoBlackout(false);assert.deepEqual(combinedEvents,[['video',false,undefined]],'Video-only clear cannot lift the DMX latch');
mock.executeTargetBlackout=async()=>{throw Error('foreign project fence');};
await combined.setLightingBlackout(false);assert.match(combinedMessages.at(-1),/Runtime blackout cleared; target blackout update failed/);
console.log('PASS: unified DMX clear preserves exact project fence and partial-result truth');
