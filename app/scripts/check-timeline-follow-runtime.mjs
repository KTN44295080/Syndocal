import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const appRoot = new URL("..", import.meta.url);
const read = (path) => readFile(new URL(path, appRoot), "utf8");

const typesSource = await read("src/types.ts");
const typesOutput = ts.transpileModule(typesSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "types.ts",
}).outputText;
const {
  createTimelineFollowAbortFocusFence,
  createTimelineFollowAbortLease,
  timelineFollowRuntimeCanApply,
} = await import(
  `data:text/javascript;base64,${Buffer.from(typesOutput).toString("base64")}`
);

const current = { epoch: 9, generation: 24 };
assert.equal(timelineFollowRuntimeCanApply(null, current), true, "the first runtime report establishes its fence");
assert.equal(
  timelineFollowRuntimeCanApply(current, { epoch: 9, generation: 24 }),
  true,
  "a same-generation status refresh can advance an in-generation settlement state",
);
assert.equal(
  timelineFollowRuntimeCanApply(current, { epoch: 9, generation: 23 }),
  false,
  "a late same-epoch generation cannot overwrite the current Follow",
);
assert.equal(
  timelineFollowRuntimeCanApply(current, { epoch: 8, generation: 99 }),
  false,
  "an old output ownership epoch is rejected even with a numerically newer generation",
);
assert.equal(
  timelineFollowRuntimeCanApply(current, { epoch: 10, generation: 1 }),
  true,
  "a newer output ownership epoch starts a fresh Follow generation domain",
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
const abortLease = createTimelineFollowAbortLease();
const abortFocusFence = createTimelineFollowAbortFocusFence();
let abortBusy = false;
let visibleMessage = "";
const scheduledFocus = [];
const focusCount = { A: 0, B: 0 };
const runDeferredAbort = async (name, operation) => {
  const lease = abortLease.begin();
  abortBusy = true;
  try {
    const applied = await operation.promise;
    return Boolean(applied) && abortLease.isCurrent(lease);
  } catch {
    abortLease.reportFailure(lease, () => { visibleMessage = `${name} failure`; });
    return false;
  } finally {
    if (abortLease.release(lease)) abortBusy = false;
  }
};
const operationA = deferred();
const pendingA = runDeferredAbort("A", operationA);
abortLease.reset();
abortFocusFence.reset();
abortBusy = false;
visibleMessage = "B pending";
const operationB = deferred();
const pendingB = runDeferredAbort("B", operationB);
operationA.reject(new Error("A lost reply after project replacement"));
const appliedA = await pendingA;
assert.equal(appliedA, false, "late A failure is not an applied Abort after project replacement");
assert.equal(abortBusy, true, "late A completion leaves B busy while B is pending");
assert.equal(visibleMessage, "B pending", "late A failure cannot overwrite B's message");
assert.equal(
  abortFocusFence.schedule(appliedA, () => { focusCount.A += 1; }, (callback) => scheduledFocus.push(callback)),
  false,
  "stale A never schedules a focus return",
);
assert.deepEqual(scheduledFocus, [], "stale A schedules no callback on either control");
operationB.resolve(true);
const appliedB = await pendingB;
assert.equal(appliedB, true, "current B completion is applied");
assert.equal(abortBusy, false, "B completion alone clears the shared busy state");
assert.equal(
  abortFocusFence.schedule(appliedB, () => { focusCount.B += 1; }, (callback) => scheduledFocus.push(callback)),
  true,
  "current B schedules its own focus return",
);
assert.equal(scheduledFocus.length, 1, "exactly one focus callback belongs to B");
scheduledFocus.shift()();
assert.deepEqual(focusCount, { A: 0, B: 1 }, "only B's invoking control receives focus");
assert.equal(
  abortFocusFence.schedule(false, () => { focusCount.B += 1; }, (callback) => scheduledFocus.push(callback)),
  false,
  "an unapplied Abort never schedules focus",
);
assert.equal(scheduledFocus.length, 0, "an unapplied Abort cannot enqueue a later focus steal");

const mountFocusCount = { old: 0, current: 0 };
assert.equal(
  abortFocusFence.schedule(true, () => { mountFocusCount.old += 1; }, (callback) => scheduledFocus.push(callback)),
  true,
  "an applied result may schedule against its captured invoking element",
);
assert.equal(scheduledFocus.length, 1, "the old mount owns one deferred callback before reset");
abortFocusFence.reset();
scheduledFocus.shift()();
assert.deepEqual(mountFocusCount, { old: 0, current: 0 }, "reset invalidates an already-scheduled old-mount focus callback at execution time");
assert.equal(
  abortFocusFence.schedule(true, () => { mountFocusCount.current += 1; }, (callback) => scheduledFocus.push(callback)),
  true,
  "the new mount can schedule with the current focus generation",
);
scheduledFocus.shift()();
assert.deepEqual(mountFocusCount, { old: 0, current: 1 }, "only the new current invoking element receives focus exactly once");

const [app, bank, operator, performance, cuePanel, localization, backend] = await Promise.all([
  read("src/App.tsx"),
  read("src/components/TimelineBankPanel.tsx"),
  read("src/components/TimelineOperatorBar.tsx"),
  read("src/components/TimelinePerformanceEditor.tsx"),
  read("src/components/TimelineCueEventsPanel.tsx"),
  read("src/uiLocalization.ts"),
  read("src-tauri/src/main.rs"),
]);

for (const command of [
  "get_timeline_follow_runtime",
  "abort_timeline_follow",
  "get_timeline_follow_operation_terminal_result",
]) {
  assert.match(app, new RegExp(`"${command}"`), `${command} is routed through the renderer owner/lock facade`);
  assert.match(backend, new RegExp(`\\b${command}\\b`), `${command} is registered by the native ABI`);
}
assert.match(app, /expectedEpoch: authority\.project_epoch,[\s\S]*expectedRevision: authority\.project_revision,[\s\S]*expectedCheckpointHash: authority\.checkpoint_hash,[\s\S]*ownerId: projectTransactionOwnerId/, "Follow runtime read carries exact E/R/H and owner authority");
assert.match(app, /request: TimelineFollowAbortRequest[\s\S]*expected_follow_epoch: observed\.epoch,[\s\S]*expected_generation: observed\.generation/, "Abort is bound to the observed output epoch and Follow generation");
assert.match(app, /get_timeline_follow_operation_terminal_result[\s\S]*args,[\s\S]*\.catch\(\(\) => null\)/, "lost abort replies query the immutable terminal using the same exact args");
assert.match(app, /requestSerial < appliedTimelineFollowRuntimeRequestSerial/, "late polling replies lose to a newer runtime/abort application serial");
assert.match(app, /timelineFollowRuntimeCanApply\(appliedTimelineFollowRuntime, report\.runtime\)/, "the UI applies the tested epoch/generation fence");
assert.match(app, /timelineFollowAbortLease\.reset\(\)[\s\S]*setTimelineFollowAbortBusy\(false\)/, "project replacement invalidates an old Abort lease before clearing busy");
assert.match(app, /timelineFollowAbortFocusFence\.reset\(\)/, "project replacement invalidates already-scheduled focus callbacks");
assert.match(app, /const abortLease = timelineFollowAbortLease\.begin\(\)/, "each Abort obtains a monotonic operation lease");
assert.match(app, /timelineFollowAbortLease\.reportFailure\(abortLease,[\s\S]*setMessage/, "an old Abort cannot publish a stale failure message");
assert.match(app, /timelineFollowAbortLease\.release\(abortLease\)[\s\S]*setTimelineFollowAbortBusy\(false\)/, "only the current Abort completion clears shared busy");
assert.match(app, /window\.setInterval\(\(\) => void refresh\(\), 100\)/, "visible Timeline Follow runtime polling is bounded at 100ms");
assert.match(app, /isTauriRuntime\(\)\s*\? timelineFollowRuntime\(\)/, "native UI binds Follow truth to runtime-only state rather than authored Follow settings");

assert.match(bank, /data-timeline-follow-runtime-badge/, "Bank surface renders a runtime truth badge");
assert.match(bank, /data-timeline-follow-domains/, "Bank surface renders the authoritative domain quorum");
assert.match(bank, /data-timeline-follow-abort/, "Bank surface exposes Abort Follow");
assert.match(bank, /"min-height": "44px"/, "Bank abort has an explicit 44px target");
assert.match(bank, /followAbortFocusFence\.schedule\(applied, \(\) => invokingButton\.focus\(\), requestAnimationFrame\)/, "Bank schedules the captured invoking element through the resettable focus fence");
assert.doesNotMatch(bank, /abortTrigger/, "Bank callbacks never dereference a mutable post-reset button ref");
assert.match(operator, /data-timeline-follow-operator-badge/, "Operator surface exposes runtime truth directly");
assert.match(operator, /onAbortFollow=\{props\.onAbortFollow\}/, "Operator passes the runtime abort capability to Performance");
assert.match(performance, /onAbortFollow=\{props\.onAbortFollow\}/, "Performance passes the runtime abort capability to Bank");
assert.match(cuePanel, /onAbortFollow=\{props\.onAbortFollow\}/, "Cue surface passes the runtime abort capability to Bank");
for (const key of ["Timeline Follow runtime", "Abort Timeline Follow", "Timeline Follow settlement domains", "settling", "aborting"]) {
  assert.match(localization, new RegExp(key), `${key} has a Japanese localization entry`);
}

console.log("Timeline Follow runtime: stale E/G rejection, E/R/H abort receipt recovery, operator/performance/cue visibility, focus, target, and localization contracts are present.");
