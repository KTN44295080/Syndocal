import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [controllerSource, appSource, automationSource, keyboardSource, shortcutSource, operatorSource, cuePanelSource, rustSource] = await Promise.all([
  readFile(new URL("../src/timelineTransportRuntimeController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/createTimelineAutomationController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/appShortcutActions.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/TimelineOperatorBar.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../../app/src-tauri/src/main.rs", import.meta.url), "utf8"),
]);

const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "timelineTransportRuntimeController.ts",
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
  "BAQEBAQEBAQEBAQEBAQEBA",
  "BQUFBQUFBQUFBQUFBQUFBQ",
];
const project = (epoch = 3, revision = 4, checkpoint = hash("a")) => ({
  project_epoch: epoch,
  project_revision: revision,
  project_checkpoint_hash: checkpoint,
});
const fence = (epoch, generation, projectIdentity = project()) => ({
  project: {
    process_incarnation: 1,
    session_incarnation: 2,
    project_epoch: projectIdentity.project_epoch,
    project_revision: projectIdentity.project_revision,
    project_checkpoint_hash: projectIdentity.project_checkpoint_hash,
    project_publication_generation: 5,
  },
  domain: "timeline.transport",
  source_runtime_epoch: epoch,
  source_runtime_generation: generation,
});
const authority = (epoch, generation, seed = 0, projectIdentity = project()) => ({
  operation_id: runtime.timelineTransportSetPlayingOperationId,
  authority_id: authorityIds[seed % authorityIds.length],
  fence: fence(epoch, generation, projectIdentity),
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
  : {
    epoch: request.expected_fence.source_runtime_epoch + 1,
    generation: 1,
  };
const receipt = (request) => {
  const next = nextPair(request);
  return {
    kind: "receipt",
    result: {
      operation_id: runtime.timelineTransportSetPlayingOperationId,
      request_id: request.request_id,
      fence_before: request.expected_fence,
      requested_playing: request.payload.playing,
      epoch_after: next.epoch,
      generation_after: next.generation,
      shape_sha256: hash("b"),
      outcome: "applied",
    },
  };
};
const staleRejection = (request) => ({
  kind: "rejected",
  result: {
    operation_id: runtime.timelineTransportSetPlayingOperationId,
    request_id: request.request_id,
    fence_before: request.expected_fence,
    error: { code: "stale_fence" },
  },
});
const rejection = (request, code) => ({
  kind: "rejected",
  result: {
    operation_id: runtime.timelineTransportSetPlayingOperationId,
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

// A deferred Play followed by rapid Pause proves latest-intent serialisation:
// the current Play settles once, then exactly one fresh-authority Pause wins.
let generation = 1;
let authorityCalls = 0;
const commandCalls = [];
const firstCommand = deferred();
const rapid = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") {
      const issued = authority(1, generation, authorityCalls);
      authorityCalls += 1;
      return issued;
    }
    assert.equal(command, "set_timeline_transport_playing_runtime_v1");
    const request = args?.request;
    commandCalls.push(request);
    if (commandCalls.length === 1) return firstCommand.promise;
    return receipt(request);
  },
});
const play = rapid.setPlaying(true);
await tickUntil(() => commandCalls.length === 1, "Play must reach one strict command dispatch");
const pause = rapid.setPlaying(false);
generation = 2;
firstCommand.resolve(receipt(commandCalls[0]));
await tickUntil(() => commandCalls.length === 2, "latest Pause must dispatch after Play settles");
assert.equal(commandCalls[0].payload.playing, true);
assert.equal(commandCalls[1].payload.playing, false);
assert.equal(commandCalls[1].expected_fence.source_runtime_generation, 2,
  "the queued opposite intent must obtain a fresh authority generation");
assert.equal(authorityCalls, 2, "each accepted intent must query its own authority bundle");
await Promise.all([play, pause]);

// A synthetic worker applies B but loses the IPC reply. The one bounded retry
// must resend the exact same authority_id/request_id/fence/payload, never a
// second actuation under a fresh authority.
let replyLossApplied = false;
const replyLossCalls = [];
let replyLossAuthorityCalls = 0;
const replyLoss = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") {
      replyLossAuthorityCalls += 1;
      return authority(1, 7, 2);
    }
    const request = args?.request;
    replyLossCalls.push(request);
    if (replyLossCalls.length === 1) {
      replyLossApplied = request.payload.playing;
      throw new Error("synthetic IPC reply loss after worker apply");
    }
    assert.equal(replyLossApplied, true, "the synthetic worker must have applied before reply loss");
    assert.strictEqual(replyLossCalls[1], replyLossCalls[0], "reply-loss retry must reuse the exact request object");
    return receipt(request);
  },
});
await replyLoss.setPlaying(true);
assert.equal(replyLossCalls.length, 2, "generic reply loss gets exactly one resend");
assert.equal(replyLossAuthorityCalls, 1, "exact resend must not query a new authority");
assert.deepEqual(replyLossCalls[1], replyLossCalls[0]);
assert.equal(replyLossCalls[1].authority_id, replyLossCalls[0].authority_id);
assert.equal(replyLossCalls[1].request_id, replyLossCalls[0].request_id);

// A typed stale fence instead gets one fresh authority and a new request.
let staleGeneration = 10;
let staleAuthorityCalls = 0;
const staleCommandCalls = [];
const staleRetry = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") {
      const issued = authority(1, staleGeneration++, staleAuthorityCalls + 3);
      staleAuthorityCalls += 1;
      return issued;
    }
    const request = args?.request;
    staleCommandCalls.push(request);
    return staleCommandCalls.length === 1
      ? staleRejection(request)
      : receipt(request);
  },
});
await staleRetry.setPlaying(true);
assert.equal(staleAuthorityCalls, 2, "one stale response permits one authority refresh");
assert.equal(staleCommandCalls.length, 2, "stale retry budget is exactly two fresh sends");
assert.notEqual(staleCommandCalls[0].authority_id, staleCommandCalls[1].authority_id,
  "stale retry must obtain a fresh authority_id");
assert.notEqual(staleCommandCalls[0].request_id, staleCommandCalls[1].request_id,
  "stale retry must create a fresh request_id");
assert.notEqual(staleCommandCalls[0].expected_fence.source_runtime_generation,
  staleCommandCalls[1].expected_fence.source_runtime_generation,
  "stale retry must not reuse the stale fence");

// A receipt alone is not renderer success. The originating Pause must remain
// pending until its authority-bound canonical snapshot has accepted the exact
// requested state and receipt successor pair.
const canonicalRefresh = deferred();
let canonicalAcknowledgement = null;
let canonicalCommandCalls = 0;
const canonicalConvergence = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") return authority(7, 30, 4);
    canonicalCommandCalls += 1;
    return receipt(args?.request);
  },
  refreshCanonicalSnapshot: async (acknowledgement) => {
    canonicalAcknowledgement = acknowledgement;
    await canonicalRefresh.promise;
  },
});
const canonicalPause = canonicalConvergence.setPlaying(false);
await tickUntil(
  () => canonicalAcknowledgement !== null,
  "a valid receipt must enter canonical snapshot convergence",
);
let canonicalPauseSettled = false;
void canonicalPause.then(() => { canonicalPauseSettled = true; });
await Promise.resolve();
assert.equal(canonicalPauseSettled, false,
  "Pause must not report success before the canonical snapshot applies");
assert.equal(canonicalCommandCalls, 1, "canonical convergence must not send a second mutation");
assert.deepEqual(canonicalAcknowledgement, {
  requestedPlaying: false,
  fenceBefore: fence(7, 30),
  scope: scope(),
  epochAfter: 7,
  generationAfter: 31,
  outcome: "applied",
});
canonicalRefresh.resolve();
await canonicalPause;
assert.equal(canonicalPauseSettled, true, "Pause resolves after canonical snapshot convergence");

// A failed canonical read is visible to the caller and cannot be hidden by a
// success message or a retry under fresh authority after the native receipt.
let canonicalFailureCommandCalls = 0;
const canonicalFailure = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") return authority(8, 40, 5);
    canonicalFailureCommandCalls += 1;
    return receipt(args?.request);
  },
  refreshCanonicalSnapshot: async () => {
    throw new Error("synthetic canonical snapshot convergence failure");
  },
});
await assert.rejects(
  canonicalFailure.setPlaying(false),
  /synthetic canonical snapshot convergence failure/,
);
assert.equal(canonicalFailureCommandCalls, 1,
  "a failed canonical read must not trigger another transport mutation");

// Scope changes partition the queue. A Play already waiting on A and its
// queued A Pause both reject after B replaces the project, without emitting a
// mutation to B. A subsequently queued B Pause proceeds exactly once.
const projectA = project(31, 41, hash("d"));
const projectB = project(32, 42, hash("e"));
activeScope = scope(projectA, 100);
const scopeFirstCommand = deferred();
const scopeCommandCalls = [];
const scoped = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") {
      const current = captureActiveScope();
      const identity = current.project_epoch === projectA.project_epoch ? projectA : projectB;
      return authority(9, 70, 0, identity);
    }
    const request = args?.request;
    scopeCommandCalls.push(request);
    if (scopeCommandCalls.length === 1) return scopeFirstCommand.promise;
    return receipt(request);
  },
});
const aPlay = scoped.setPlaying(true);
void aPlay.catch(() => {});
await tickUntil(() => scopeCommandCalls.length === 1, "A Play must reach the native worker");
const aPause = scoped.setPlaying(false);
void aPause.catch(() => {});
activeScope = scope(projectB, 101);
const bPause = scoped.setPlaying(false);
scopeFirstCommand.resolve(receipt(scopeCommandCalls[0]));
await assert.rejects(aPlay, /project scope changed/);
await assert.rejects(aPause, /project scope changed/);
await bPause;
assert.equal(scopeCommandCalls.length, 2,
  "discarded A groups must send no replacement-project mutation");
assert.equal(scopeCommandCalls[0].expected_fence.project.project_epoch, projectA.project_epoch);
assert.equal(scopeCommandCalls[1].expected_fence.project.project_epoch, projectB.project_epoch);
assert.equal(scopeCommandCalls[1].payload.playing, false,
  "the subsequent B Pause sends exactly one B mutation");

// A query reply from another E/R/H is not an authority for the captured
// origin, even if it is well-formed. It fails before the mutation boundary.
activeScope = scope(projectA, 150);
let mismatchedAuthorityMutationCalls = 0;
const mismatchedAuthority = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command) => command === "query_timeline_transport_authority_v1"
    ? authority(9, 71, 2, projectB)
    : (mismatchedAuthorityMutationCalls += 1, null),
});
await assert.rejects(mismatchedAuthority.setPlaying(true), /project scope changed/);
assert.equal(mismatchedAuthorityMutationCalls, 0,
  "a mismatched query authority must send zero mutations");

// A canonical failure belongs only to the dispatched earlier group. It must
// reject its own Play even if a later same-project Pause reaches canonical
// success; the later group still resolves.
activeScope = scope(projectA, 200);
const firstCanonical = deferred();
const settlementCalls = [];
let settlementRefreshCalls = 0;
const partitionedSettlement = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") return authority(10, 80, 1, projectA);
    const request = args?.request;
    settlementCalls.push(request);
    return receipt(request);
  },
  refreshCanonicalSnapshot: async () => {
    settlementRefreshCalls += 1;
    if (settlementRefreshCalls === 1) await firstCanonical.promise;
  },
});
const failingPlay = partitionedSettlement.setPlaying(true);
void failingPlay.catch(() => {});
await tickUntil(() => settlementCalls.length === 1, "first settlement group must dispatch");
const succeedingPause = partitionedSettlement.setPlaying(false);
firstCanonical.reject(new Error("synthetic first canonical failure"));
await assert.rejects(failingPlay, /synthetic first canonical failure/);
await succeedingPause;
assert.equal(settlementCalls.length, 2, "later same-project intent dispatches independently");

activeScope = scope();

// Typed permanent failures and malformed discriminators are fail-closed and
// do not enter either retry layer.
let permanentCalls = 0;
const permanent = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => command === "query_timeline_transport_authority_v1"
    ? authority(1, 20, 0)
    : (permanentCalls += 1, rejection(args?.request, "forbidden")),
});
await assert.rejects(permanent.setPlaying(true), /forbidden/);
assert.equal(permanentCalls, 1, "permanent typed failures must not retry");

let unknownCalls = 0;
const unknownDiscriminator = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command) => command === "query_timeline_transport_authority_v1"
    ? authority(1, 21, 1)
    : (unknownCalls += 1, { kind: "future_terminal", result: {} }),
});
await assert.rejects(
  unknownDiscriminator.setPlaying(true),
  /invalid response/,
  "unknown runtime response discriminators must fail closed",
);
assert.equal(unknownCalls, 1, "unknown discriminator must not be replayed as IPC loss");

// An Applied receipt must carry the exact successor pair; a saturated pair has
// no valid Applied successor at all.
let invalidAppliedCalls = 0;
const invalidApplied = runtime.createTimelineTransportRuntimeController({
  captureScope: captureActiveScope,
  refreshCanonicalSnapshot: acceptCanonicalSnapshot,
  invoke: async (command, args) => {
    if (command === "query_timeline_transport_authority_v1") return authority(maxSafe, maxSafe, 2);
    invalidAppliedCalls += 1;
    const request = args?.request;
    return {
      kind: "receipt",
      result: {
        operation_id: runtime.timelineTransportSetPlayingOperationId,
        request_id: request.request_id,
        fence_before: request.expected_fence,
        requested_playing: true,
        epoch_after: maxSafe,
        generation_after: maxSafe,
        shape_sha256: hash("c"),
        outcome: "applied",
      },
    };
  },
});
await assert.rejects(invalidApplied.setPlaying(true), /invalid Applied receipt/);
assert.equal(invalidAppliedCalls, 1, "invalid Applied receipt must not retry");

assert.equal(
  (controllerSource.match(/options\.invoke<unknown>\("query_timeline_transport_authority_v1"/g) ?? []).length,
  1,
  "the strict lane must have one authority query binding",
);
assert.equal(
  (controllerSource.match(/"set_timeline_transport_playing_runtime_v1"/g) ?? []).length,
  1,
  "the strict lane must have one runtime mutation binding",
);
assert.match(
  controllerSource,
  /const acknowledgement = await sendExactRequest\(request, group\.scope\);[\s\S]*?requireCurrentScope\(group\.scope\);[\s\S]*?await options\.refreshCanonicalSnapshot\(acknowledgement\);/,
  "a receipt must converge through the injected canonical snapshot before the lane resolves",
);
assert.match(
  controllerSource,
  /captureScope: \(\) => TimelineTransportRuntimeScope;/,
  "each enqueue scope source must include E/R/H plus local read generation",
);
assert.match(
  controllerSource,
  /let scope: TimelineTransportRuntimeScope;[\s\S]*?scope = captureScope\(\);[\s\S]*?const tail = queued\.at\(-1\);[\s\S]*?tail && scopesEqual\(tail\.scope, scope\)/,
  "each enqueue must capture E/R/H plus local read generation and coalesce only the same scope",
);
assert.match(
  controllerSource,
  /requireCurrentScope\(group\.scope\);[\s\S]*?query_timeline_transport_authority_v1[\s\S]*?fenceProjectMatchesScope\(authority\.fence, group\.scope\)[\s\S]*?requireCurrentScope\(group\.scope\);[\s\S]*?sendExactRequest\(request, group\.scope\)/,
  "scope must fence query, authority identity, and every mutation send",
);
assert.match(
  controllerSource,
  /group\.deferreds\.forEach\(\(\{ resolve \}\) => resolve\(\)\);[\s\S]*?group\.deferreds\.forEach\(\(\{ reject \}\) => reject\(error\)\);/,
  "each dispatched scope group must settle independently",
);
assert.doesNotMatch(controllerSource, /begin_project_transaction|commit_project_transaction|"set_timeline_playing"/,
  "the strict renderer lane must not reopen legacy or generic project transactions");

assert.match(
  appSource,
  /const invokeTimelineTransportRuntime = async <T,>\([\s\S]*?command !== "query_timeline_transport_authority_v1"[\s\S]*?command !== "set_timeline_transport_playing_runtime_v1"[\s\S]*?return tauriInvoke<T>\(command, args\)/,
  "root Timeline must use the narrow direct-Tauri runtime dispatcher",
);
assert.match(
  appSource,
  /const refreshTimelineTransportCanonicalSnapshot = async \([\s\S]*?timelineTransportRuntimeScopeIsCurrent\(acknowledgement\.scope\)[\s\S]*?tauriInvoke<ProjectAuthorityBundle>\("get_project_authority_bundle", \{[\s\S]*?expectedEpoch: expectedAuthority\.project_epoch,[\s\S]*?expectedRevision: expectedAuthority\.project_revision,[\s\S]*?expectedCheckpointHash: expectedAuthority\.checkpoint_hash,[\s\S]*?timeline\.playing !== acknowledgement\.requestedPlaying[\s\S]*?canonical\.timeline_transport_epoch !== acknowledgement\.epochAfter[\s\S]*?canonical\.timeline_transport_generation !== acknowledgement\.generationAfter[\s\S]*?applyEngineSnapshot\(canonical\.snapshot\);[\s\S]*?setSnapshotRevision\(null\);/,
  "root Timeline must apply only the exact authority-bound canonical transport snapshot and top-level runtime fence",
);

// TimelineSummary intentionally skips its runtime transport fence for project
// persistence. The authority bundle must therefore project the values at its
// own top level, and the real Rust serializer test must prove both sides of
// that boundary.
assert.match(
  rustSource,
  /#\[derive\(Debug, Clone, Serialize\)\][\s\S]*?struct ProjectAuthorityBundle \{[\s\S]*?timeline_transport_epoch: u64,[\s\S]*?timeline_transport_generation: u64,/,
  "Rust authority bundle must expose top-level runtime transport fence fields",
);
assert.match(
  rustSource,
  /fn project_authority_bundle_from_captured_snapshot\([\s\S]*?snapshot: EngineSnapshot,[\s\S]*?let \(timeline_transport_epoch, timeline_transport_generation\) =\s*project_authority_transport_fence\(&snapshot\);[\s\S]*?timeline_transport_epoch,[\s\S]*?timeline_transport_generation,[\s\S]*?snapshot,/,
  "Rust bundle construction must capture the fence before moving the exact snapshot",
);
assert.match(
  rustSource,
  /fn project_authority_bundle_serializes_runtime_transport_fence_at_top_level\(\)[\s\S]*?serde_json::to_value\(&mutation\.authority\)[\s\S]*?wire\["timeline_transport_epoch"\][\s\S]*?wire\["timeline_transport_generation"\][\s\S]*?get\("transport_epoch"\)\s*\.is_none\(\)[\s\S]*?get\("transport_generation"\)\s*\.is_none\(\)/,
  "Rust serialization proof must cover top-level fields and omitted nested persistence fields",
);

const canonicalBundleTransportMatches = (bundle, requestedPlaying, epochAfter, generationAfter) =>
  bundle.snapshot.timeline.playing === requestedPlaying
  && bundle.timeline_transport_epoch === epochAfter
  && bundle.timeline_transport_generation === generationAfter;
const canonicalBundle = {
  snapshot: { timeline: { playing: false } },
  timeline_transport_epoch: 7,
  timeline_transport_generation: 31,
};
assert.equal(
  canonicalBundleTransportMatches(canonicalBundle, false, 7, 31),
  true,
  "a serialized authority bundle with the exact top-level fence must converge",
);
assert.equal(
  canonicalBundleTransportMatches({ ...canonicalBundle, timeline_transport_epoch: 6 }, false, 7, 31),
  false,
  "a top-level epoch mismatch must reject canonical convergence",
);
assert.equal(
  canonicalBundleTransportMatches({
    ...canonicalBundle,
    timeline: { playing: false },
    snapshot: { timeline: { playing: false, transport_epoch: 7, transport_generation: 31 } },
  }, false, 7, 31),
  true,
  "nested legacy-looking values cannot replace the top-level transport identity",
);
assert.match(
  appSource,
  /const timelineTransportGenerationAtRequest = timelineTransportCanonicalSnapshotGeneration;[\s\S]*?requestedReadGeneration === projectReadGeneration[\s\S]*?timelineTransportGenerationAtRequest === timelineTransportCanonicalSnapshotGeneration/,
  "a pre-receipt full snapshot must not overwrite later canonical transport convergence",
);
assert.match(
  appSource,
  /await refreshOperatorPolicy\(true\);[\s\S]*?await refreshFixtureGroups\(\);[\s\S]*?timelineTransportGenerationAtRequest[\s\S]*?timelineTransportCanonicalSnapshotGeneration[\s\S]*?if \(next !== null[\s\S]*?timelineTransportGenerationAtRequest[\s\S]*?timelineTransportCanonicalSnapshotGeneration\) \{[\s\S]*?applyEngineSnapshot\(/,
  "a delayed reset full read must recheck canonical transport generation immediately before applying",
);
assert.match(
  appSource,
  /const timelineTransportRuntime = createTimelineTransportRuntimeController\(\{[\s\S]*?invoke: invokeTimelineTransportRuntime,[\s\S]*?captureScope: captureTimelineTransportRuntimeScope,[\s\S]*?refreshCanonicalSnapshot: refreshTimelineTransportCanonicalSnapshot,[\s\S]*?\}\);[\s\S]*?const setCanonicalTimelinePlaying = async \(playing: boolean\) => \{[\s\S]*?await timelineTransportRuntime\.setPlaying\(playing\);/,
  "root Timeline callback must enter the strict runtime lane",
);
assert.match(
  automationSource,
  /const setTimelinePlaying = async \(playing: boolean\) => \{[\s\S]*?await options\.setTimelinePlaying\(playing\);/,
  "Timeline controller must delegate Play/Pause to its injected strict callback",
);
assert.doesNotMatch(automationSource, /"set_timeline_playing"/,
  "Timeline controller must not retain a raw root set_timeline_playing fallback");
assert.match(
  appSource,
  /<TimelineOperatorBar[\s\S]*?onPause=\{pauseTimeline\}[\s\S]*?onPlay=\{playTimeline\}/,
  "rendered Timeline operator controls must use the shared callbacks",
);
assert.match(
  appSource,
  /<TimelineCueEventsPanel[\s\S]*?onPause=\{pauseTimeline\}[\s\S]*?onPlay=\{playTimeline\}/,
  "rendered Timeline cue-panel controls must use the shared callbacks",
);
assert.match(operatorSource, /onClick=\{\(\) => void props\.onPause\(\)\}[\s\S]*?onClick=\{\(\) => void props\.onPlay\(\)\}/,
  "operator Play/Pause buttons must call their injected canonical callbacks",
);
assert.match(cuePanelSource, /onClick=\{\(\) => void props\.onPause\(\)\}[\s\S]*?onClick=\{\(\) => void props\.onPlay\(\)\}/,
  "cue-panel Play/Pause buttons must call their injected canonical callbacks",
);
assert.match(
  appSource,
  /createAppKeyboardController\(\{[\s\S]*?playTimeline,[\s\S]*?pauseTimeline,[\s\S]*?\}\);/,
  "AppShortcut construction must receive the same Play/Pause callbacks",
);
assert.match(keyboardSource, /playTimeline: options\.playTimeline,[\s\S]*?pauseTimeline: options\.pauseTimeline,/,
  "AppShortcut executor must preserve the shared callbacks",
);
assert.match(shortcutSource, /case "toggleTimelinePlayback":[\s\S]*?executor\.pauseTimeline\(\)[\s\S]*?executor\.playTimeline\(\)/,
  "toggle_timeline_playback dispatch must only select those shared callbacks",
);

const genericMutationBlock = appSource.slice(
  appSource.indexOf("const projectMutationCommands = new Set(["),
  appSource.indexOf("const serverAuthoritativeProjectMutationCommands = new Set(["),
);
assert.doesNotMatch(genericMutationBlock, /set_timeline_transport_playing_runtime_v1/,
  "canonical runtime transport must not be enrolled in the renderer transaction wrapper");

console.log("timeline transport runtime strict route, canonical snapshot convergence, two-layer retry, latest intent, and fail-closed pair checks passed");
