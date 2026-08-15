import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const [controllerSource, appSource, automationSource, keyboardSource, shortcutSource, operatorSource, cuePanelSource] = await Promise.all([
  readFile(new URL("../src/timelineTransportRuntimeController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/createTimelineAutomationController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/createAppKeyboardController.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/appShortcutActions.ts", import.meta.url), "utf8"),
  readFile(new URL("../src/components/TimelineOperatorBar.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/TimelineCueEventsPanel.tsx", import.meta.url), "utf8"),
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
const fence = (epoch, generation) => ({
  project: {
    process_incarnation: 1,
    session_incarnation: 2,
    project_epoch: 3,
    project_revision: 4,
    project_checkpoint_hash: hash("a"),
    project_publication_generation: 5,
  },
  domain: "timeline.transport",
  source_runtime_epoch: epoch,
  source_runtime_generation: generation,
});
const authority = (epoch, generation, seed = 0) => ({
  operation_id: runtime.timelineTransportSetPlayingOperationId,
  authority_id: authorityIds[seed % authorityIds.length],
  fence: fence(epoch, generation),
});
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

// A deferred Play followed by rapid Pause proves latest-intent serialisation:
// the current Play settles once, then exactly one fresh-authority Pause wins.
let generation = 1;
let authorityCalls = 0;
const commandCalls = [];
const firstCommand = deferred();
const rapid = runtime.createTimelineTransportRuntimeController({
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

// Typed permanent failures and malformed discriminators are fail-closed and
// do not enter either retry layer.
let permanentCalls = 0;
const permanent = runtime.createTimelineTransportRuntimeController({
  invoke: async (command, args) => command === "query_timeline_transport_authority_v1"
    ? authority(1, 20, 0)
    : (permanentCalls += 1, rejection(args?.request, "forbidden")),
});
await assert.rejects(permanent.setPlaying(true), /forbidden/);
assert.equal(permanentCalls, 1, "permanent typed failures must not retry");

let unknownCalls = 0;
const unknownDiscriminator = runtime.createTimelineTransportRuntimeController({
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
assert.doesNotMatch(controllerSource, /begin_project_transaction|commit_project_transaction|"set_timeline_playing"/,
  "the strict renderer lane must not reopen legacy or generic project transactions");

assert.match(
  appSource,
  /const invokeTimelineTransportRuntime = async <T,>\([\s\S]*?command !== "query_timeline_transport_authority_v1"[\s\S]*?command !== "set_timeline_transport_playing_runtime_v1"[\s\S]*?return tauriInvoke<T>\(command, args\)/,
  "root Timeline must use the narrow direct-Tauri runtime dispatcher",
);
assert.match(
  appSource,
  /const timelineTransportRuntime = createTimelineTransportRuntimeController\(\{[\s\S]*?invoke: invokeTimelineTransportRuntime,[\s\S]*?\}\);[\s\S]*?const setCanonicalTimelinePlaying = async \(playing: boolean\) => \{[\s\S]*?await timelineTransportRuntime\.setPlaying\(playing\);/,
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

console.log("timeline transport runtime strict route, two-layer retry, latest intent, and fail-closed pair checks passed");
