import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [controllerSource, integrationSource, appSource, commandsSource, manifestSource, rustSource] = await Promise.all([
  readFile(new URL("../src/timelineLoopRuntimeController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/timelineLoopRuntimeIntegration.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/tauriInvokeCommands.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/tauri-invoke-manifest.json", import.meta.url), "utf8"),
  readFile(new URL("../../app/src-tauri/src/main.rs", import.meta.url), "utf8"),
]);

const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineLoopRuntimeController.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);

const hash = (character) => character.repeat(64);
const maxSafe = Number.MAX_SAFE_INTEGER;
const authorityIds = [
  "AAAAAAAAAAAAAAAAAAAAAA",
  "AQEBAQEBAQEBAQEBAQEBAQ",
  "AgICAgICAgICAgICAgICAg",
  "AwMDAwMDAwMDAwMDAwMDAw",
];
const project = (epoch = 3, revision = 4, checkpoint = hash("a")) => ({
  project_epoch: epoch,
  project_revision: revision,
  project_checkpoint_hash: checkpoint,
});
const fence = (
  epoch,
  generation,
  projectIdentity = project(),
  sourceLoopGeneration = 0,
  sourceFollowGeneration = 0,
) => ({
  project: {
    process_incarnation: 1,
    session_incarnation: 2,
    project_epoch: projectIdentity.project_epoch,
    project_revision: projectIdentity.project_revision,
    project_checkpoint_hash: projectIdentity.project_checkpoint_hash,
    project_publication_generation: 5,
  },
  domain: "timeline.loop",
  source_runtime_epoch: epoch,
  source_runtime_generation: generation,
  source_loop_generation: sourceLoopGeneration,
  source_follow_generation: sourceFollowGeneration,
});
const authority = (
  epoch,
  generation,
  seed = 0,
  projectIdentity = project(),
  sourceLoopGeneration = 0,
  sourceFollowGeneration = 0,
) => ({
  operation_id: runtime.timelineLoopRuntimeCommitOperationId,
  authority_id: authorityIds[seed % authorityIds.length],
  fence: fence(epoch, generation, projectIdentity, sourceLoopGeneration, sourceFollowGeneration),
});
const scope = (projectIdentity = project(), readGeneration = 0) => ({
  project_epoch: projectIdentity.project_epoch,
  project_revision: projectIdentity.project_revision,
  checkpoint_hash: projectIdentity.project_checkpoint_hash,
  project_read_generation: readGeneration,
});
let activeScope = scope();
const captureActiveScope = () => activeScope;
const nextPair = (request) => request.expected_fence.source_runtime_generation < maxSafe
  ? {
    epoch: request.expected_fence.source_runtime_epoch,
    generation: request.expected_fence.source_runtime_generation + 1,
  }
  : { epoch: request.expected_fence.source_runtime_epoch + 1, generation: 1 };
const receipt = (request, { loopGeneration = 1, followGeneration = 0, outcome = "applied" } = {}) => {
  const next = outcome === "applied" ? nextPair(request) : {
    epoch: request.expected_fence.source_runtime_epoch,
    generation: request.expected_fence.source_runtime_generation,
  };
  return {
    kind: "receipt",
    result: {
      operation_id: runtime.timelineLoopRuntimeCommitOperationId,
      request_id: request.request_id,
      fence_before: request.expected_fence,
      requested_action: request.action,
      epoch_after: next.epoch,
      generation_after: next.generation,
      loop_generation_after: loopGeneration,
      follow_generation_after: followGeneration,
      shape_sha256: hash("b"),
      outcome,
    },
  };
};
const rejection = (request, code) => ({
  kind: "rejected",
  result: {
    operation_id: runtime.timelineLoopRuntimeCommitOperationId,
    request_id: request.request_id,
    fence_before: request.expected_fence,
    error: { code },
  },
});
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
const acceptCanonicalSnapshot = async () => {};

// Relative Scale cannot coalesce through a SetEnabled action. A later Scale
// must query fresh authority only after the first receipt has converged.
let authorityCalls = 0;
const commandCalls = [];
const firstCommand = deferred();
const fifo = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") {
      return authority(1, authorityCalls++ + 7, authorityCalls);
    }
    assert.equal(command, "commit_timeline_loop_runtime_v1");
    const request = args?.request;
    commandCalls.push(request);
    if (commandCalls.length === 1) return firstCommand.promise;
    return receipt(request, { loopGeneration: commandCalls.length, followGeneration: 5 });
  },
});
const enable = fifo.setEnabled(true);
await tickUntil(() => commandCalls.length === 1, "SetEnabled must dispatch exactly once");
const halve = fifo.scale("half");
firstCommand.resolve(receipt(commandCalls[0], { loopGeneration: 1, followGeneration: 5 }));
await Promise.all([enable, halve]);
assert.equal(commandCalls.length, 2, "SetEnabled and Scale must be serialized, not coalesced");
assert.deepEqual(commandCalls[0].action, { kind: "set_enabled", enabled: true });
assert.deepEqual(commandCalls[1].action, { kind: "scale", scale: "half" });
assert.equal(commandCalls[1].expected_fence.source_runtime_generation, 8,
  "the Scale must obtain a fresh root-loop authority");

// Reply loss resends the exact object once. It may not mint another capability
// after an untyped error because native B may already have applied.
let replyLossAuthorityCalls = 0;
const replyLossCalls = [];
const replyLoss = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") {
      replyLossAuthorityCalls += 1;
      return authority(1, 9, 2);
    }
    const request = args?.request;
    replyLossCalls.push(request);
    if (replyLossCalls.length === 1) throw new Error("synthetic reply loss after B");
    assert.strictEqual(replyLossCalls[0], replyLossCalls[1], "retry must reuse one exact request object");
    return receipt(request, { loopGeneration: 7, followGeneration: 0 });
  },
});
await replyLoss.scale("double");
assert.equal(replyLossAuthorityCalls, 1, "reply-loss resend must not query a new authority");
assert.equal(replyLossCalls.length, 2, "reply-loss retry is bounded to one exact resend");

// A typed stale fence gets one new capability/request; permanent errors and
// malformed action echoes fail closed with no replay.
let staleGeneration = 20;
const staleCalls = [];
const staleRetry = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") {
      return authority(1, staleGeneration++, staleCalls.length + 1);
    }
    const request = args?.request;
    staleCalls.push(request);
    return staleCalls.length === 1 ? rejection(request, "stale_fence") : receipt(request);
  },
});
await staleRetry.setEnabled(false);
assert.equal(staleCalls.length, 2, "only stale_fence may issue a fresh root-loop request");
assert.notEqual(staleCalls[0].authority_id, staleCalls[1].authority_id);
assert.notEqual(staleCalls[0].request_id, staleCalls[1].request_id);

// A NoOp is a complete no-mutation proof: it may not quietly advance either
// loop/follow watermark even though the transport pair itself is unchanged.
let invalidNoOpCalls = 0;
const invalidNoOp = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") {
      return authority(1, 24, 0, project(), 11, 13);
    }
    invalidNoOpCalls += 1;
    return receipt(args?.request, {
      loopGeneration: 12,
      followGeneration: 13,
      outcome: "no_op",
    });
  },
});
await assert.rejects(invalidNoOp.scale("half"), /invalid NoOp receipt/);
assert.equal(invalidNoOpCalls, 1, "a NoOp watermark mismatch must not retry");

let rewoundAppliedCalls = 0;
const rewoundApplied = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") {
      return authority(1, 24, 0, project(), 12, 13);
    }
    rewoundAppliedCalls += 1;
    return receipt(args?.request, { loopGeneration: 11, followGeneration: 13 });
  },
});
await assert.rejects(rewoundApplied.setEnabled(false), /invalid Applied receipt/);
assert.equal(rewoundAppliedCalls, 1, "an Applied watermark rewind must not retry");

let malformedCalls = 0;
const malformed = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_loop_runtime_authority_v1") return authority(1, 30, 0);
    malformedCalls += 1;
    const response = receipt(args?.request);
    response.result.requested_action = { kind: "scale", scale: "double" };
    return response;
  },
});
await assert.rejects(malformed.setEnabled(true), /invalid receipt/);
assert.equal(malformedCalls, 1, "mismatched receipt action must never replay");

const projectA = project(31, 41, hash("d"));
const projectB = project(32, 42, hash("e"));
activeScope = scope(projectA, 10);
let foreignMutationCalls = 0;
const scopeFence = runtime.createTimelineLoopRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command) => command === "query_timeline_loop_runtime_authority_v1"
    ? authority(1, 1, 0, projectB)
    : (foreignMutationCalls += 1, null),
});
await assert.rejects(scopeFence.scale("half"), /project scope changed/);
assert.equal(foreignMutationCalls, 0, "authority from another E/R/H must send no loop mutation");
activeScope = scope();

assert.equal(
  (controllerSource.match(/options\.invoke<unknown>\("query_timeline_loop_runtime_authority_v1"/g) ?? []).length,
  1,
  "the strict lane has exactly one loop-authority query binding",
);
assert.equal(
  (controllerSource.match(/"commit_timeline_loop_runtime_v1"/g) ?? []).length,
  1,
  "the strict lane has exactly one loop mutation binding",
);
assert.match(controllerSource, /requested_action[\s\S]*?actionsEqual\(response\.result\.requested_action, request\.action\)/,
  "the receipt must echo the exact typed action");
assert.match(controllerSource, /loop_generation_after[\s\S]*?follow_generation_after/,
  "the receipt must retain both loop and Follow snapshot watermarks");
assert.match(controllerSource,
  /source_loop_generation[\s\S]*?source_follow_generation[\s\S]*?loop_generation_after !== request\.expected_fence\.source_loop_generation[\s\S]*?follow_generation_after !== request\.expected_fence\.source_follow_generation/,
  "the strict fence must carry both source watermarks and keep them unchanged for NoOp");
assert.match(controllerSource,
  /loop_generation_after < request\.expected_fence\.source_loop_generation[\s\S]*?follow_generation_after < request\.expected_fence\.source_follow_generation/,
  "an Applied receipt must never rewind either runtime watermark");
assert.doesNotMatch(controllerSource, /set_timeline_loop_enabled|scale_timeline_loop|set_timeline_transport_playing_runtime_v1/,
  "the loop lane must not retain legacy or Play/Pause endpoints");
assert.match(
  integrationSource,
  /const invoke = async <T,>\([\s\S]*?if \(command !== "query_timeline_loop_runtime_authority_v1"[\s\S]*?&& command !== "commit_timeline_loop_runtime_v1"\)[\s\S]*?throw new Error\("Timeline loop dispatcher rejected an unknown command\."\)[\s\S]*?return options\.tauriInvoke<T>\(command, args\);/,
  "the integration module must enforce the two-command loop allowlist before reaching Tauri",
);
assert.match(
  integrationSource,
  /const refreshCanonicalSnapshot = async \([\s\S]*?const readGuard = options\.captureProjectReadGuard\(\);[\s\S]*?options\.projectReadGuardIsCurrent\(readGuard\)[\s\S]*?options\.applyEngineSnapshot\(canonical\.snapshot, readGuard\)/,
  "canonical loop reads must carry the captured project read guard through the apply seam",
);
assert.match(
  integrationSource,
  /const applied = options\.applyEngineSnapshot\(canonical\.snapshot, readGuard\);[\s\S]*?if \(!applied\) \{[\s\S]*?Timeline loop canonical snapshot had a stale or malformed runtime watermark\.[\s\S]*?options\.setSnapshotRevision\(null\);/,
  "canonical loop apply must fail closed on a stale watermark before clearing snapshot revision",
);
assert.match(appSource,
  /import \{ createTimelineLoopRuntimeIntegration \} from "\.\/timelineLoopRuntimeIntegration";/,
  "App must statically import the reviewed loop integration module");
assert.match(appSource,
  /const timelineLoopRuntimeIntegration = createTimelineLoopRuntimeIntegration\(\{[\s\S]*?captureProjectReadGuard,[\s\S]*?projectReadGuardIsCurrent,[\s\S]*?tauriInvoke,[\s\S]*?beginTimelineTransportCanonicalSnapshotConvergence,[\s\S]*?applyEngineSnapshot: \(snapshot, projectReadGuard\) => applyEngineSnapshot\([\s\S]*?\{ projectReadGuard \},[\s\S]*?setSnapshotRevision: \(revision\) => setSnapshotRevision\(revision\),[\s\S]*?\}\);/,
  "App must wire the authority, read guard, shared convergence barrier, and guarded apply seams into the reviewed integration");
assert.match(appSource,
  /const invokeTimelineLoopRuntime = async <T,>\([\s\S]*?return timelineLoopRuntimeIntegration\.invoke<T>\(command, args\);/,
  "App must delegate loop command dispatch to the reviewed integration module");
assert.match(appSource,
  /const refreshTimelineLoopCanonicalSnapshot = async \([\s\S]*?return timelineLoopRuntimeIntegration\.refreshCanonicalSnapshot\(acknowledgement\);/,
  "App must delegate canonical loop convergence to the reviewed integration module");
assert.match(appSource,
  /const timelineLoopRuntime = createTimelineLoopRuntimeController\(\{[\s\S]*?invoke: invokeTimelineLoopRuntime,[\s\S]*?captureScope: captureTimelineLoopRuntimeScope,[\s\S]*?refreshCanonicalSnapshot: refreshTimelineLoopCanonicalSnapshot,[\s\S]*?\}\);/,
  "the root Timeline must construct the separate loop controller");
assert.doesNotMatch(appSource, /invoke<void>\("set_timeline_loop_enabled"|invoke<void>\("scale_timeline_loop"/,
  "the App must clean-break its fire-and-forget loop invokes");
assert.match(commandsSource, /"commit_timeline_loop_runtime_v1"[\s\S]*?"query_timeline_loop_runtime_authority_v1"/,
  "the finite frontend invoke tuple must authorize only the V1 loop commands");
assert.doesNotMatch(commandsSource, /"set_timeline_loop_enabled"|"scale_timeline_loop"/,
  "the finite frontend invoke tuple must retire legacy loop commands");
const manifest = JSON.parse(manifestSource);
assert(manifest.includes("query_timeline_loop_runtime_authority_v1"));
assert(manifest.includes("commit_timeline_loop_runtime_v1"));
assert(!manifest.includes("set_timeline_loop_enabled"));
assert(!manifest.includes("scale_timeline_loop"));
assert.match(rustSource,
  /fn query_timeline_loop_runtime_authority_v1[\s\S]*?fn commit_timeline_loop_runtime_v1[\s\S]*?query_timeline_loop_runtime_authority_v1,[\s\S]*?commit_timeline_loop_runtime_v1,/,
  "the invoked V1 endpoints must be registered by the native command surface");

console.log("timeline loop runtime strict authority, exact action receipt, reply-loss/stale policy, canonical convergence, and legacy clean-break checks passed");
