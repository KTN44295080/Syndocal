import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const compileTs = (source, fileName) => ts.transpileModule(source, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName,
}).outputText;

const dataModule = (source) => `data:text/javascript;base64,${Buffer.from(source).toString("base64")}`;
const lifecycle = await import(dataModule(compileTs(
  await readFile(new URL("../src/paneWindowLifecycle.ts", import.meta.url), "utf8"),
  "paneWindowLifecycle.ts",
)));
const app = await readFile(new URL("../src/App.tsx", import.meta.url), "utf8");
const operations = await readFile(new URL("../src/components/WorkspaceOperationsMenu.tsx", import.meta.url), "utf8");
const mainRs = await readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8");
const localization = await readFile(new URL("../src/uiLocalization.ts", import.meta.url), "utf8");
const lifecycleSource = await readFile(new URL("../src/paneWindowLifecycle.ts", import.meta.url), "utf8");
const tauriConf = JSON.parse(
  await readFile(new URL("../src-tauri/tauri.conf.json", import.meta.url), "utf8"),
);

const flush = () => new Promise((resolve) => queueMicrotask(resolve));

assert.equal(lifecycle.PANE_WINDOW_TERMINAL_EVENT, "syndocal://pane-window-terminal");
assert.equal(lifecycle.isValidPaneOpaqueId("inst_A-b9"), true);
assert.equal(lifecycle.isValidPaneOpaqueId(""), false);
assert.equal(lifecycle.isValidPaneOpaqueId("a".repeat(129)), false);
assert.equal(lifecycle.isValidPaneOpaqueId("bad id\n"), false);
assert.match(lifecycle.createOpaquePaneId(), /^[A-Za-z0-9_-]+$/);

const beginConfirmedOpen = (controller, pane) => {
  const intent = controller.beginOpen(pane);
  assert.ok(intent, `${pane} open intent must be accepted`);
  controller.confirmOpen(pane, intent.instanceId);
  return intent;
};

{
  const label = "matching cancel clears the exact pending close";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const open = beginConfirmedOpen(controller, "timeline");
  const arm = controller.armClose("timeline");
  assert.equal(arm.instanceId, open.instanceId);
  assert.match(arm.requestId, /^[A-Za-z0-9_-]+$/);
  let outcome = null;
  arm.terminalPromise.then(() => { outcome = "destroyed"; }, (rejection) => { outcome = rejection; });
  await flush();
  assert.equal(outcome, null, "the waiter must stay pending without a terminal");
  assert.equal(controller.transitionOf("timeline").phase, "closing");
  assert.equal(
    controller.applyTerminal({
      pane: "timeline",
      instance_id: open.instanceId,
      request_id: arm.requestId,
      terminal: "canceled",
    }),
    true,
    `${label}: correlated cancel must be accepted`,
  );
  await flush();
  assert.deepEqual(outcome, { reason: "canceled" }, label);
  assert.equal(controller.transitionOf("timeline").phase, "open", `${label}: pane stays open`);
  assert.equal(controller.transitionOf("timeline").instanceId, open.instanceId, label);
  assert.equal(controller.transitionOf("timeline").requestId, null, `${label}: pending request cleared`);
}

{
  const label = "wrong-instance and stale-request terminals are ignored";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const open = beginConfirmedOpen(controller, "stage");
  const arm = controller.armClose("stage");
  let outcome = null;
  arm.terminalPromise.then(() => { outcome = "destroyed"; }, (rejection) => { outcome = rejection; });
  await flush();
  assert.equal(
    controller.applyTerminal({ pane: "stage", instance_id: "newer-instance", request_id: null, terminal: "destroyed" }),
    false,
    `${label}: wrong instance must be ignored`,
  );
  assert.equal(outcome, null, label);
  assert.notEqual(controller.transitionOf("stage"), null, `${label}: newer state survives`);
  assert.equal(
    controller.applyTerminal({ pane: "stage", instance_id: open.instanceId, request_id: "older-request", terminal: "canceled" }),
    false,
    `${label}: stale request id must be ignored`,
  );
  assert.equal(outcome, null, label);
  assert.equal(controller.applyTerminal({ pane: "mixer", instance_id: "x", request_id: null, terminal: "destroyed" }), false, label);
  assert.equal(controller.applyTerminal(null), false, label);
  assert.equal(controller.applyTerminal({ pane: "stage", instance_id: open.instanceId, request_id: null, terminal: "exploded" }), false, label);
  assert.equal(outcome, null, `${label}: malformed terminals never settle a live waiter`);
  assert.equal(
    controller.applyTerminal({ pane: "stage", instance_id: open.instanceId, request_id: arm.requestId, terminal: "canceled" }),
    true,
    `${label}: the exact matching cancel still lands`,
  );
}

{
  const label = "destroyed after cancel retires once and duplicates are ignored";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const open = beginConfirmedOpen(controller, "timeline");
  const firstArm = controller.armClose("timeline");
  let firstOutcome = null;
  firstArm.terminalPromise.then(
    () => { firstOutcome = "destroyed"; },
    (rejection) => { firstOutcome = rejection; },
  );
  assert.equal(
    controller.applyTerminal({ pane: "timeline", instance_id: open.instanceId, request_id: firstArm.requestId, terminal: "canceled" }),
    true,
    `${label}: user keeps the timeline open`,
  );
  await flush();
  assert.deepEqual(firstOutcome, { reason: "canceled" }, `${label}: first waiter is handled`);
  const secondArm = controller.armClose("timeline");
  assert.equal(secondArm.instanceId, open.instanceId, label);
  assert.notEqual(secondArm.requestId, firstArm.requestId, `${label}: every close arms a fresh request`);
  let outcome = null;
  secondArm.terminalPromise.then(() => { outcome = "destroyed"; }, (rejection) => { outcome = rejection; });
  await flush();
  assert.equal(
    controller.applyTerminal({ pane: "timeline", instance_id: open.instanceId, request_id: firstArm.requestId, terminal: "canceled" }),
    false,
    `${label}: the retired first request cannot cancel again`,
  );
  assert.equal(outcome, null, label);
  assert.equal(
    controller.applyTerminal({ pane: "timeline", instance_id: open.instanceId, request_id: secondArm.requestId, terminal: "destroyed" }),
    true,
    `${label}: confirmed discard ends via destroyed`,
  );
  await flush();
  assert.equal(outcome, "destroyed", label);
  assert.equal(controller.transitionOf("timeline"), null, `${label}: instance retired`);
  assert.equal(
    controller.applyTerminal({ pane: "timeline", instance_id: open.instanceId, request_id: secondArm.requestId, terminal: "destroyed" }),
    false,
    `${label}: duplicate destroyed terminals are stale`,
  );
}

{
  const label = "missing-window close still settles its waiter";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const open = beginConfirmedOpen(controller, "mixer");
  const arm = controller.armClose("mixer");
  let outcome = null;
  arm.terminalPromise.then(() => { outcome = "destroyed"; }, (rejection) => { outcome = rejection; });
  // Backend order for a missing window: retire registry state, emit the
  // terminal synchronously, then resolve the invoke. The waiter registered
  // before the invoke receives it either way.
  assert.equal(
    controller.applyTerminal({ pane: "mixer", instance_id: open.instanceId, request_id: arm.requestId, terminal: "destroyed" }),
    true,
    label,
  );
  await flush();
  assert.equal(outcome, "destroyed", `${label}: terminal arrives before the invoke reply`);
  assert.equal(controller.transitionOf("mixer"), null, label);
}

{
  const label = "invoke failure aborts the wait and reopens the local phase";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const open = beginConfirmedOpen(controller, "programmer");
  const arm = controller.armClose("programmer");
  let outcome = null;
  arm.terminalPromise.then(() => { outcome = "destroyed"; }, (rejection) => { outcome = rejection; });
  controller.settleArmInvokeFailure("programmer", arm.requestId, "bridge rejected");
  await flush();
  assert.deepEqual(outcome, { reason: "aborted", message: "bridge rejected" }, label);
  assert.equal(controller.transitionOf("programmer").phase, "open", label);
  // A late native destroyed for the same instance remains authoritative.
  assert.equal(
    controller.applyTerminal({ pane: "programmer", instance_id: open.instanceId, request_id: arm.requestId, terminal: "destroyed" }),
    true,
    `${label}: late terminal still retires`,
  );
}

{
  const label = "open rejection leaves no hidden pane record";
  const controller = lifecycle.createPaneWindowLifecycleController();
  const intent = controller.beginOpen("setup");
  assert.ok(intent, label);
  controller.failOpen("setup", intent.instanceId);
  assert.equal(controller.transitionOf("setup"), null, `${label}: failed open removes its transition`);
  assert.deepEqual(controller.view(), {}, label);
  // A stale confirm/fail for a removed intent mutates nothing.
  controller.confirmOpen("setup", intent.instanceId);
  assert.equal(controller.transitionOf("setup"), null, label);
}

{
  const label = "apply exclusivity blocks overlapping applies and busy panes";
  const controller = lifecycle.createPaneWindowLifecycleController();
  assert.equal(controller.tryBeginWorkspaceApply(), true, label);
  assert.equal(controller.tryBeginWorkspaceApply(), false, `${label}: overlapping apply refused`);
  assert.equal(controller.isWorkspaceApplyActive(), true, label);
  controller.endWorkspaceApply();
  assert.equal(controller.isWorkspaceApplyActive(), false, label);
  const busy = controller.beginOpen("touch");
  assert.notEqual(busy, null, label);
  assert.equal(controller.hasAnyTransition(), true, label);
  assert.equal(controller.tryBeginWorkspaceApply(), false, `${label}: opening pane blocks apply`);
  assert.equal(controller.beginOpen("touch"), null, `${label}: per-pane transition refuses re-entry`);
  controller.confirmOpen("touch", busy.instanceId);
  assert.equal(controller.hasAnyTransition(), false, `${label}: stable open is not busy`);
  assert.equal(controller.tryBeginWorkspaceApply(), true, `${label}: stable panes permit apply`);
  controller.endWorkspaceApply();
  controller.dispose("lifecycle disposed");
  assert.equal(controller.beginOpen("live"), null, `${label}: disposed controllers fail closed`);
  assert.equal(controller.tryBeginWorkspaceApply(), false, label);
  assert.equal(
    controller.applyTerminal({ pane: "live", instance_id: "x", request_id: null, terminal: "destroyed" }),
    false,
    `${label}: disposed controllers ignore late events`,
  );
}

{
  const label = "reload adoption preserves the exact stable identity";
  const controller = lifecycle.createPaneWindowLifecycleController();
  assert.equal(controller.adoptOpen("timeline", "live-instance-1"), true, label);
  assert.deepEqual(controller.transitionOf("timeline"), {
    phase: "open",
    instanceId: "live-instance-1",
    requestId: null,
  }, label);
  assert.equal(controller.adoptOpen("timeline", "conflict-instance"), false, label);
  assert.equal(controller.hasAnyTransition(), false, `${label}: adoption is stable, not pending`);
}

{
  const label = "failed repositions restore stable-open truth without clobbering state";
  const controller = lifecycle.createPaneWindowLifecycleController();
  assert.equal(
    controller.restoreOpenAfterFailedReposition("stage", "inst-a"),
    false,
    `${label}: nothing to restore`,
  );
  assert.equal(controller.adoptOpen("stage", "inst-a"), true, label);
  assert.equal(
    controller.restoreOpenAfterFailedReposition("stage", "inst-a"),
    false,
    `${label}: a stable open pane needs no restore`,
  );
  // A failed reposition of an opening incarnation restores open truth.
  const intent = controller.beginOpen("mixer", "inst-m");
  assert.ok(intent, label);
  assert.equal(controller.transitionOf("mixer").phase, "opening", label);
  assert.equal(
    controller.restoreOpenAfterFailedReposition("mixer", "inst-m"),
    true,
    `${label}: opening residue is restored`,
  );
  assert.deepEqual(
    controller.transitionOf("mixer"),
    { phase: "open", instanceId: "inst-m", requestId: null },
    label,
  );
  // Closing residue from the same incarnation also returns to open truth,
  // and its exact stale close waiter is settled instead of hanging forever.
  const arm = controller.armClose("mixer");
  assert.ok(arm, label);
  let restoredArmOutcome = null;
  arm.terminalPromise.then(
    () => { restoredArmOutcome = "destroyed"; },
    (rejection) => { restoredArmOutcome = rejection; },
  );
  assert.equal(
    controller.restoreOpenAfterFailedReposition("mixer", "inst-m"),
    true,
    `${label}: closing residue is restored`,
  );
  assert.deepEqual(
    controller.transitionOf("mixer"),
    { phase: "open", instanceId: "inst-m", requestId: null },
    `${label}: the pending request is cleared`,
  );
  await flush();
  assert.deepEqual(
    restoredArmOutcome,
    { reason: "aborted", message: "Pane window close wait returned to open state." },
    `${label}: the stale close waiter settles as aborted`,
  );
  // A different claimed instance never clobbers the live identity.
  assert.equal(
    controller.restoreOpenAfterFailedReposition("mixer", "inst-other"),
    false,
    `${label}: a newer identity stays untouched`,
  );
  assert.equal(controller.transitionOf("mixer").instanceId, "inst-m", label);
  controller.dispose("done");
  assert.equal(
    controller.restoreOpenAfterFailedReposition("mixer", "inst-m"),
    false,
    `${label}: disposed controllers fail closed`,
  );
}

// Deterministic manual clock implementing the probe helper's timer seam.
// advance() fires due timers strictly one at a time and drains microtasks
// between firings, so reschedules become observable and ordering is exact.
const settle = async (rounds = 8) => {
  for (let i = 0; i < rounds; i += 1) await Promise.resolve();
};

const createManualClock = () => {
  let nowMs = 0;
  let nextHandle = 1;
  let timers = [];
  const clock = {
    setTimeout: (callback, delayMs) => {
      const handle = nextHandle;
      nextHandle += 1;
      timers.push({ handle, at: nowMs + delayMs, callback });
      return handle;
    },
    clearTimeout: (handle) => {
      timers = timers.filter((entry) => entry.handle !== handle);
    },
    now: () => nowMs,
    pendingCount: () => timers.length,
    advance: async (stepMs) => {
      const horizon = nowMs + stepMs;
      for (;;) {
        timers.sort((first, second) => first.at - second.at || first.handle - second.handle);
        const due = timers[0];
        if (!due || due.at > horizon) break;
        nowMs = due.at;
        timers.shift();
        due.callback();
        await settle();
      }
      nowMs = horizon;
    },
  };
  return clock;
};

{
  const label = "continuous probes start delayed, repeat past eight, and stop on stillWaiting";
  const clock = createManualClock();
  const events = [];
  let waiting = true;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => waiting,
    probe: async () => {
      events.push(clock.now());
    },
    intervalMs: 250,
    schedule: clock,
  });
  await clock.advance(249);
  assert.equal(events.length, 0, `${label}: no probe before one full interval`);
  await clock.advance(1);
  assert.deepEqual(events, [250], label);
  // Far past the retired 8-attempt bound the loop must still be running.
  await clock.advance(250 * 10);
  assert.equal(events.length, 11, `${label}: ran ${events.length} sequential probes`);
  assert.equal(events[events.length - 1], 250 * 11, `${label}: cadence stays fixed`);
  assert.equal(clock.pendingCount(), 1, `${label}: exactly one live timer between probes`);
  waiting = false;
  await clock.advance(250);
  const stoppedAt = events.length;
  assert.equal(clock.pendingCount(), 0, `${label}: no timer survives a stopped loop`);
  await clock.advance(250 * 3);
  assert.equal(events.length, stoppedAt, `${label}: stillWaiting=false ends probing`);
  canceler();
}

{
  const label = "rejected probes are swallowed and still reschedule";
  const clock = createManualClock();
  let attempts = 0;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => true,
    probe: async () => {
      attempts += 1;
      if (attempts % 2 === 1) throw new Error("sweep unavailable");
    },
    intervalMs: 100,
    schedule: clock,
  });
  await clock.advance(100 * 6);
  assert.equal(attempts, 6, `${label}: every cycle ran despite alternating rejections`);
  canceler();
  assert.equal(clock.pendingCount(), 0, label);
}

{
  const label = "cancel-before-fire clears the only scheduled timer";
  const clock = createManualClock();
  let probed = 0;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => true,
    probe: async () => {
      probed += 1;
    },
    intervalMs: 50,
    schedule: clock,
  });
  canceler();
  canceler();
  assert.equal(clock.pendingCount(), 0, `${label}: double cancel stays harmless`);
  await clock.advance(500);
  assert.equal(probed, 0, `${label}: nothing fired after cancellation`);
}

{
  const label = "cancel during an in-flight probe prevents any reschedule";
  const clock = createManualClock();
  let started = 0;
  let releaseProbe = () => undefined;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => true,
    probe: () => new Promise((resolve) => {
      started += 1;
      releaseProbe = resolve;
    }),
    intervalMs: 40,
    schedule: clock,
  });
  await clock.advance(40);
  assert.equal(started, 1, `${label}: delayed start reached the first probe`);
  assert.equal(clock.pendingCount(), 0, `${label}: no timer exists while a probe is in flight`);
  canceler();
  releaseProbe();
  await settle();
  assert.equal(clock.pendingCount(), 0, `${label}: in-flight completion after cancel never reschedules`);
  await clock.advance(400);
  assert.equal(started, 1, `${label}: the canceled loop stays dead`);
}

{
  const label = "stillWaiting=false during an in-flight probe schedules nothing without cancel";
  const clock = createManualClock();
  let waiting = true;
  let started = 0;
  let releaseProbe = () => undefined;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => waiting,
    probe: () => new Promise((resolve) => {
      started += 1;
      releaseProbe = resolve;
    }),
    intervalMs: 60,
    schedule: clock,
  });
  await clock.advance(60);
  assert.equal(started, 1, `${label}: the delayed first probe fired and is paused in flight`);
  assert.equal(clock.pendingCount(), 0, `${label}: no timer exists while the probe is paused`);
  waiting = false;
  releaseProbe();
  await settle();
  assert.equal(clock.pendingCount(), 0, `${label}: the post-settle stillWaiting check refuses to reschedule`);
  await clock.advance(600);
  assert.equal(started, 1, `${label}: no further probes ever fire`);
  canceler();
}

{
  const label = "probes are strictly sequential with no overlap";
  const clock = createManualClock();
  let inFlight = 0;
  let maxInFlight = 0;
  let completed = 0;
  const canceler = lifecycle.startPendingCloseReconciliationProbes({
    stillWaiting: () => true,
    probe: async () => {
      inFlight += 1;
      maxInFlight = Math.max(maxInFlight, inFlight);
      await settle(2);
      inFlight -= 1;
      completed += 1;
    },
    intervalMs: 30,
    schedule: clock,
  });
  await clock.advance(30 * 5);
  assert.equal(completed, 5, label);
  assert.equal(maxInFlight, 1, `${label}: at most one probe ever runs at once`);
  assert.equal(clock.pendingCount(), 1, `${label}: the single loop keeps its one live timer`);
  canceler();
  assert.equal(clock.pendingCount(), 0, label);
}

const section = (source, start, end) => {
  const startIndex = source.indexOf(start);
  assert.notEqual(startIndex, -1, `missing section start: ${start}`);
  const endIndex = source.indexOf(end, startIndex + start.length);
  assert.notEqual(endIndex, -1, `missing section end: ${end}`);
  return source.slice(startIndex, endIndex);
};

const closeCoreSection = section(app, "const closePaneWindowCore = async", "const openPaneWindow = (");
const armIndex = closeCoreSection.indexOf("paneLifecycleController.armClose(pane)");
const closeInvokeIndex = closeCoreSection.indexOf('await invoke("close_pane_window", { pane, instanceId: arm.instanceId, requestId: arm.requestId })');
assert.ok(armIndex >= 0 && closeInvokeIndex > armIndex, "the terminal waiter must be armed before the close invoke");
assert.match(closeCoreSection, /const terminalOutcome = arm\.terminalPromise\.then/, "the parent attaches both terminal handlers before invoke");
assert.match(closeCoreSection, /await terminalOutcome/, "the parent close must await the correlated terminal with no timeout");
assert.doesNotMatch(closeCoreSection, /setTimeout|setInterval/, "parent closes must not time out");
const probesIndex = closeCoreSection.indexOf("startPendingCloseReconciliationProbes({");
assert.ok(probesIndex > closeInvokeIndex, "reconciliation probes start only after the close invoke was accepted");
assert.match(
  closeCoreSection,
  /stillWaiting: \(\) => \{[\s\S]*?closing\.requestId === arm\.requestId/,
  "probes key off the exact armed close request only",
);
assert.match(
  closeCoreSection,
  /finally \{\s*stopReconciliationProbes\(\);\s*\}/,
  "probe cancellation is guaranteed even when disposal aborts the wait",
);
assert.match(
  closeCoreSection,
  /intervalMs: PANE_WINDOW_CLOSE_RECONCILIATION_PROBE_INTERVAL_MS/,
  "production probes keep the fixed cadence through the named interval constant",
);

// Close-invoke catch contract: arm/handlers -> close invoke -> one fully
// caught advisory capture reconciliation -> settleArmInvokeFailure ->
// await the already-attached terminalOutcome. The capture may let a
// sweep-redelivered retirement outrank the invoke failure, and any
// reconciliation failure is reported alongside the original close error.
const closeCatchIndex = closeCoreSection.indexOf("} catch (error) {", closeInvokeIndex);
assert.ok(closeCatchIndex > closeInvokeIndex, "the close invoke carries its own catch");
const closeCatchCaptureIndex = closeCoreSection.indexOf(
  'await invoke("capture_pane_window_placements")',
  closeCatchIndex,
);
assert.ok(closeCatchCaptureIndex > closeCatchIndex, "the catch issues its advisory capture after the invoke failed");
const closeCatchSettleIndex = closeCoreSection.indexOf(
  "paneLifecycleController.settleArmInvokeFailure(pane, arm.requestId, String(error))",
  closeCatchCaptureIndex,
);
assert.ok(closeCatchSettleIndex > closeCatchCaptureIndex, "arm failure settles only after the advisory capture");
const closeCatchAwaitIndex = closeCoreSection.indexOf("await terminalOutcome;", closeCatchSettleIndex);
assert.ok(closeCatchAwaitIndex > closeCatchSettleIndex, "the already-attached terminalOutcome is awaited last in the catch");
const closeCatchSection = closeCoreSection.slice(closeCatchIndex, closeCatchAwaitIndex);
assert.equal(
  [...closeCatchSection.matchAll(/capture_pane_window_placements/g)].length,
  1,
  "the catch contains exactly one advisory reconciliation capture",
);
const closeProbesWiringIndex = closeCoreSection.indexOf("startPendingCloseReconciliationProbes({", closeCatchAwaitIndex);
assert.ok(closeProbesWiringIndex > 0, "the continuous probe loop follows the close try/catch");
const closeProbeCaptureIndex = closeCoreSection.indexOf("capture_pane_window_placements", closeProbesWiringIndex);
assert.ok(
  closeProbeCaptureIndex > closeProbesWiringIndex,
  "any further capture belongs exclusively to the accepted-invoke continuous probe loop",
);
assert.ok(closeCatchSection.includes("try {"), "the advisory reconciliation attempt is wrapped in its own try");
assert.ok(closeCatchSection.includes("reconciliationError = String(reconciliation)"), "a failed reconciliation is fully caught into a reportable error");
// Truthful outcome tail: everything between the awaited already-attached
// terminal and the accepted-invoke probe wiring belongs to this catch.
const closeCatchTailSection = closeCoreSection.slice(closeCatchAwaitIndex, closeProbesWiringIndex);
assert.match(closeCatchTailSection, /caughtOutcome\.destroyed/, "a sweep-redelivered retirement outranks the invoke failure");
assert.match(closeCatchTailSection, /acknowledgePaneWindowClosed\(pane\)/, "the redelivered retirement retires the popped record through the normal path");
assert.match(closeCatchTailSection, /return true;/, "a redelivered retirement reports truthful close success");
assert.match(closeCatchTailSection, /reconciliationError === null/, "the message branches on reconciliation success");
assert.match(closeCatchTailSection, /Pane window close failed: \$\{error\}/, "the original close failure text survives unchanged when reconciliation succeeds");
assert.match(closeCatchTailSection, /placement reconciliation failed: \$\{reconciliationError\}/, "both the close error and the reconciliation error reach the operator truthfully");
const closeCatchWholeSection = closeCoreSection.slice(closeCatchIndex, closeProbesWiringIndex);
assert.doesNotMatch(
  closeCatchWholeSection,
  /applyTerminal|resolveDestroyed|terminal: ["']|beginOpen|createOpaquePaneId|adoptOpen|confirmOpen/,
  "catch-path reconciliation never synthesizes outcomes or mints identities",
);
assert.doesNotMatch(closeCoreSection, /attemptLimit/, "close wiring carries no obsolete attempt cap");
assert.doesNotMatch(app, /RECONCILIATION_PROBE_LIMIT/, "the attempt-limit constant is fully retired from App");

const probeHelperStart = lifecycleSource.indexOf("export type PendingCloseReconciliationProbeOptions");
assert.notEqual(probeHelperStart, -1, "the reconciliation probe helper exists");
const probeHelperSection = lifecycleSource.slice(probeHelperStart);
assert.match(
  probeHelperSection,
  /\.catch\(\(\) => undefined\)/,
  "probe failures are swallowed and never settle or fail the close",
);
assert.match(
  probeHelperSection,
  /!options\.stillWaiting\(\)/,
  "probes stop as soon as the exact wait is no longer pending",
);
assert.match(probeHelperSection, /clearTimeout\(timer\)/, "the returned canceller owns its timer");
assert.doesNotMatch(
  probeHelperSection,
  /settleWaiter|resolveDestroyed|reason: "destroyed"/,
  "probes never synthesize a close outcome",
);
assert.match(
  probeHelperSection,
  /options\.schedule \?\?/,
  "the timer seam defaults to real timers and is injectable for deterministic tests",
);
assert.match(
  probeHelperSection,
  /let timer: unknown = null;/,
  "the helper owns exactly one scheduled-timer slot",
);
assert.match(
  probeHelperSection,
  /const tick = \(\): void => \{\s*(?:\/\/[^\n]*\n\s*)*timer = null;/,
  "tick consumes its fired timer handle at entry so the single-slot invariant stays truthful",
);
assert.match(
  probeHelperSection,
  /if \(probeInFlight\) return;/,
  "an in-flight probe excludes any overlapping tick",
);
assert.match(
  probeHelperSection,
  /probeInFlight = false;[\s\S]*?if \(canceled[\s\S]*?timers\.setTimeout\(tick/,
  "the next probe is scheduled only after the previous one settles",
);
assert.doesNotMatch(probeHelperSection, /attemptLimit/i, "no attempt-cap residue remains in the helper");

const openCoreSection = section(app, "const openPaneWindowCore = async", "const closePaneWindowCore = async");
const openInvokeIndex = openCoreSection.indexOf('await invoke<PaneWindowOpenResult>("open_pane_window"');
const markOpenIndex = openCoreSection.indexOf("markPaneWindowOpen(pane);");
const failOpenIndex = openCoreSection.indexOf("paneLifecycleController.failOpen(pane, intent.instanceId);");
assert.ok(openInvokeIndex >= 0 && markOpenIndex > openInvokeIndex && failOpenIndex > openInvokeIndex, "popped may only be marked after native open success");
assert.ok(
  openCoreSection.indexOf("markPaneWindowOpen(pane);") > openCoreSection.indexOf("confirmOpen(pane, intent.instanceId);"),
  "popped marking follows the confirmed open identity",
);
assert.match(openCoreSection, /existing\.phase !== "open"/, "stable opens are reusable while pending opens fail closed");
const adoptedRejectionContractIndex = openCoreSection.indexOf(
  "// An adopted rejection leaves lifecycle truth untouched.",
);
assert.ok(adoptedRejectionContractIndex > 0, "the adopted-open rejection path carries its reconciliation contract");
const adoptedCatchReturnIndex = openCoreSection.indexOf("return false;", adoptedRejectionContractIndex);
assert.ok(adoptedCatchReturnIndex > adoptedRejectionContractIndex, "the adopted rejection catch terminates without success");
const adoptedCatchSection = openCoreSection.slice(adoptedRejectionContractIndex, adoptedCatchReturnIndex);
assert.ok(
  openCoreSection.indexOf("paneLifecycleController.failOpen(pane, intent.instanceId);") < adoptedRejectionContractIndex
  && openCoreSection.indexOf("return false;", openCoreSection.indexOf("paneLifecycleController.failOpen"))
    < adoptedRejectionContractIndex,
  "the fresh-open failOpen path stays separate from and ahead of adopted reconciliation",
);
assert.match(
  adoptedCatchSection,
  /await invoke\("capture_pane_window_placements"\)/,
  "an adopted rejection attempts exactly one main-only capture so a lost terminal can redeliver",
);
assert.match(
  adoptedCatchSection,
  /placement reconciliation failed/,
  "both the open failure and the reconciliation failure reach the operator message",
);
assert.match(
  adoptedCatchSection,
  /restoreOpenAfterFailedReposition\(pane, intent\.instanceId\);/,
  "restoring stable-open lifecycle truth remains the only mutation after an adopted rejection",
);
assert.doesNotMatch(
  adoptedCatchSection,
  /beginOpen|createOpaquePaneId|adoptOpen|confirmOpen/,
  "reconciliation never mints, reuses, or switches identities",
);
assert.doesNotMatch(
  adoptedCatchSection,
  /applyTerminal|resolveDestroyed|"destroyed"|"canceled"/,
  "reconciliation never synthesizes a terminal outcome",
);
assert.match(openCoreSection, /return receipt\.placement_applied;/, "prepared success truth is placement_applied alone");
assert.match(
  openCoreSection,
  /if \(receipt\.warning\) setMessage\(`Pane window warning: \$\{receipt\.warning\}`\);/,
  "native warnings stay surfaced truthfully even when placement succeeded",
);

const applySection = section(app, "const applyNamedWorkspace = async", "const deleteNamedWorkspace");
assert.match(applySection, /tryBeginWorkspaceApply\(\)/, "named-workspace applies must take the exclusive gate");
assert.match(applySection, /endWorkspaceApply\(\)/, "the exclusive gate must always release");
assert.ok(
  applySection.indexOf("tryBeginWorkspaceApply()") < applySection.indexOf("applyWorkspaceLayout(profile.layout)"),
  "exclusivity precedes any layout mutation",
);
assert.doesNotMatch(applySection, /setPoppedPanes\(desiredPanes\)/, "apply must never force-set desired panes");
assert.doesNotMatch(applySection, /persistPoppedPanes\(desiredPanes\)/, "apply must never force-persist desired panes");
assert.match(applySection, /applied partially/, "partial applies must admit incomplete actual state");
assert.match(applySection, /current pane windows:/, "partial copy must report actual state");
assert.match(applySection, /for \(const pane of poppedPanes\(\)\.filter\(\(candidate\) => !desired\.has\(candidate\)\)\)/, "extras close through the lifecycle state machine");
assert.match(applySection, /await closePaneWindow\(pane\)/, "apply closes are serially awaited");
assert.match(applySection, /await openPaneWindow\(placement\.pane, placement\)/, "apply opens are serially awaited");
assert.ok(
  applySection.indexOf("await openPaneWindow(placement.pane, placement)")
    < applySection.indexOf("await closePaneWindow(pane)"),
  "desired panes are prepared before extras are closed",
);
assert.ok(
  applySection.indexOf("applyWorkspaceLayout(profile.layout)")
    > applySection.indexOf("failures.length === 0 && reconciled"),
  "the integrated layout mutates only after exact pane reconciliation",
);

const restoreSection = section(app, "const restorePoppedPaneWindows = async", "paneWindowEventsReady = listen");
assert.match(restoreSection, /capture_pane_window_placements/, "restore must consult live registry identities");
assert.match(restoreSection, /paneLifecycleController\.adoptOpen\(pane, status\.instance_id\)/, "reload-safe restore adopts the exact live instance");
assert.match(restoreSection, /status && !status\.instance_id/, "an untracked native orphan fails closed");
const restoreCaptureCatch = section(
  restoreSection,
  'statuses = await invoke<PaneWindowStatusReport[]>("capture_pane_window_placements");',
  "for (const pane of panes)",
);
assert.match(restoreCaptureCatch, /return;/, "a failed capture fails closed without touching pane state");
assert.match(restoreCaptureCatch, /kept for retry/, "capture failure notifies a retained, retryable state");
assert.doesNotMatch(
  restoreCaptureCatch,
  /acknowledgePaneWindowClosed|persistPoppedPanes|setPoppedPanes/,
  "capture failure never bulk-clears locally popped panes while native children can survive",
);
// Startup-restore deadlock gate: restorePoppedPaneWindows executes inside
// paneWindowEventsReady's own then callback. Calling the public open wrapper
// from there awaits paneWindowEventsReady itself, a promise that cannot
// settle until its own callback returns, so startup would deadlock and every
// later toggle would wait forever. Restore must bypass only the readiness
// wrapper and queue openPaneWindowCore through the shared per-pane
// operation queue, while both public wrappers keep gating on readiness.
assert.doesNotMatch(
  restoreSection,
  /openPaneWindow\(/,
  "restore must never call the public open wrapper or it self-awaits paneWindowEventsReady forever",
);
assert.match(
  restoreSection,
  /await enqueuePaneWindowOperation\(pane, async \(\) => \{\s*restored = await openPaneWindowCore\(pane, null, status\?\.instance_id \?\? undefined\);\s*\}\)/,
  "restore queues the core open directly on the same per-pane serialization queue",
);
const openWrapperSection = section(app, "const openPaneWindow = (", "const closePaneWindow = (");
assert.match(
  openWrapperSection,
  /if \(!await paneWindowEventsReady\) \{\s*setMessage\("Pane window events are unavailable\."\);\s*return false;\s*\}/,
  "the public open wrapper still gates on pane window events readiness before native work",
);
assert.match(
  openWrapperSection,
  /await enqueuePaneWindowOperation\(pane, async \(\) => \{\s*result = await openPaneWindowCore\(pane, placement, options\?\.instanceId\);\s*\}\)/,
  "the public open wrapper keeps queueing the core open through the per-pane queue",
);
const browserPopupController = await readFile(
  new URL("../src/browserPanePopupController.ts", import.meta.url),
  "utf8",
);
const initialPoppedPanesSection = section(app, "const initialPoppedPanes = (", "const persistPoppedPanes =");
assert.match(
  initialPoppedPanesSection,
  /catch \(error\) \{\s*initialPoppedPanesStorageError = String\(error\);\s*return \[\];\s*\}/,
  "an initial pane-window storage read or parse failure keeps every pane integrated",
);
assert.match(
  initialPoppedPanesSection,
  /if \(!isTauriRuntime\(\) \|\| paneWindow\) return \[\];/,
  "browser layout fallback never treats machine-local native restore records as live child proof",
);
const persistPoppedPanesSection = section(app, "const persistPoppedPanes =", "// The browser-only popup controller stays out of the bootstrap chunk.");
assert.match(
  persistPoppedPanesSection,
  /window\.localStorage\.setItem\(paneWindowStorageKey, JSON\.stringify\(panes\)\);\s*return \{ ok: true \};/,
  "popped-pane persistence reports only a completed exact write as success",
);
assert.match(
  persistPoppedPanesSection,
  /catch \(error\) \{\s*return \{ ok: false, error: String\(error\) \};\s*\}/,
  "popped-pane persistence exposes storage failure instead of swallowing it",
);
assert.ok(
  app.indexOf("Pane windows remain integrated because reading their machine-local window state failed:")
    > app.indexOf("const setMessage ="),
  "the initial pane-window storage failure reaches the visible status after status initialization",
);

// Browser fallback is deliberately layout-only. A popup or storage fault may
// never set poppedPanes, because that would hide the sole integrated content
// without a native-authoritative child lifecycle record.
const browserControllerLoadSection = section(app, "const preloadBrowserPanePopupController =", "const acknowledgePaneWindowClosed");
assert.match(app, /let browserPanePopupControllerPromise: Promise<BrowserPanePopupController> \| null = null/);
assert.match(app, /let browserPanePopupController: BrowserPanePopupController \| null = null/);
assert.match(browserControllerLoadSection, /const pending = import\("\.\/browserPanePopupController"\)/, "the browser controller is loaded lazily");
assert.match(browserControllerLoadSection, /browserPanePopupControllerPromise = pending;/, "the preload shares one import promise");
assert.match(browserControllerLoadSection, /browserPanePopupController = controller;/, "the preload stores the resolved controller synchronously for later clicks");
assert.match(browserControllerLoadSection, /void pending\.catch\(\(error\) =>/,
  "the startup preload observes import rejection without an unhandled promise",
);
assert.match(browserControllerLoadSection, /browserPanePopupControllerPromise = null;/,
  "an import rejection clears the cached promise so same-document retry can recover",
);
assert.match(browserControllerLoadSection, /browserPanePopupController = null;/,
  "an import rejection clears the cached controller reference",
);
const setMessageDeclarationIndex = app.indexOf("const setMessage =");
const browserControllerPreloadInvocationIndex = app.indexOf("void preloadBrowserPanePopupController();", setMessageDeclarationIndex);
const appReturnIndex = app.indexOf("  return (", browserControllerPreloadInvocationIndex);
assert.ok(
  setMessageDeclarationIndex >= 0 && browserControllerPreloadInvocationIndex > setMessageDeclarationIndex
    && appReturnIndex > browserControllerPreloadInvocationIndex,
  "the browser controller preload starts immediately after App status initialization",
);
assert.match(browserControllerLoadSection, /browserWindow: window/);
assert.match(browserControllerLoadSection, /browserLocation: window\.location/);
assert.match(browserControllerLoadSection, /getPoppedPanes: \(\) => poppedPanes\(\)/);
assert.match(browserControllerLoadSection, /setPoppedPanes: \(next\) => setPoppedPanes\(next\)/);
assert.match(browserControllerLoadSection, /persistPoppedPanes,/);
assert.match(browserControllerLoadSection, /setMessage: \(text\) => setMessage\(text\)/);
assert.match(app, /disposeBrowserPanePopupController\(\);/, "cleanup owns browser controller disposal");
const browserControllerDisposeSection = section(app, "const disposeBrowserPanePopupController =", "const acknowledgePaneWindowClosed");
assert.match(browserControllerDisposeSection, /browserPanePopupControllerDisposed = true;/, "cleanup marks the controller unavailable before disposal");
assert.match(browserControllerDisposeSection, /browserPanePopupControllerLoadGeneration \+= 1;/, "cleanup invalidates an in-flight controller import");
assert.match(browserControllerDisposeSection, /browserPanePopupControllerPromise = null;/, "cleanup drops the import promise");
assert.match(browserControllerDisposeSection, /browserPanePopupController = null;/, "cleanup drops the resolved controller reference");
assert.match(browserControllerDisposeSection, /controller\?\.dispose\(\);/, "cleanup disposes only an already-created controller");
const browserOpenWrapperSection = section(app, "const openBrowserPaneWindow =", "const closeBrowserPaneWindow =");
const browserOpenControllerCallIndex = browserOpenWrapperSection.indexOf("return controller.open(pane);");
assert.ok(browserOpenControllerCallIndex >= 0, "the browser open path calls the resolved controller");
assert.doesNotMatch(
  browserOpenWrapperSection.slice(0, browserOpenControllerCallIndex),
  /\bawait\b|import\(|queueMicrotask\(/,
  "the browser click path reaches controller.open without await, microtask, or dynamic import",
);
const browserCloseWrapperSection = section(app, "const closeBrowserPaneWindow =", "const disposeBrowserPanePopupController =");
const browserCloseControllerCallIndex = browserCloseWrapperSection.indexOf("return controller.close(pane);");
assert.ok(browserCloseControllerCallIndex >= 0, "the browser close path calls the resolved controller");
assert.doesNotMatch(
  browserCloseWrapperSection.slice(0, browserCloseControllerCallIndex),
  /\bawait\b|import\(|queueMicrotask\(/,
  "the browser click path reaches controller.close without await, microtask, or dynamic import",
);
const closeWrapperSection = section(app, "const closePaneWindow = (", "const togglePaneWindow =");
// The App owns only the synchronous browser-controller dispatch. Keep a
// source-level tripwire against a stale inline fallback being reintroduced in
// either browser wrapper or the public open/close wrapper around the native
// path: popup construction, browser persistence, and visibility changes belong
// exclusively to browserPanePopupController.ts.
const appPaneWrapperInlineBrowserFallback =
  /window\.open|browserWindow\.open|new URL\(|syndocalPaneWindowInstance|localStorage|persistPoppedPanes|setPoppedPanes|browserPanePopupOpenings|BrowserPanePopupRecord|createOpaquePaneId|waitForBrowserPanePopupReady|trackBrowserPanePopup|retireBrowserPanePopup|classList|style\.display/;
for (const [label, source] of [
  ["browser open wrapper", browserOpenWrapperSection],
  ["browser close wrapper", browserCloseWrapperSection],
  ["public open wrapper", openWrapperSection],
  ["public close wrapper", closeWrapperSection],
]) {
  assert.doesNotMatch(
    source,
    appPaneWrapperInlineBrowserFallback,
    `${label} contains no inline popup construction, persistence, visibility mutation, or legacy browser branch`,
  );
}
const browserOpenBranchIndex = openWrapperSection.indexOf("if (!isTauriRuntime()) return openBrowserPaneWindow(pane);");
const browserOpenNativeAwaitIndex = openWrapperSection.indexOf("await paneWindowEventsReady");
const browserCloseBranchIndex = closeWrapperSection.indexOf("if (!isTauriRuntime()) return closeBrowserPaneWindow(pane);");
const browserCloseNativeAwaitIndex = closeWrapperSection.indexOf("await paneWindowEventsReady");
assert.match(
  openWrapperSection,
  /if \(!isTauriRuntime\(\)\) return openBrowserPaneWindow\(pane\);/,
  "the browser open wrapper enters the synchronous resolved-controller path",
);
assert.match(
  closeWrapperSection,
  /if \(!isTauriRuntime\(\)\) return closeBrowserPaneWindow\(pane\);/,
  "the browser close wrapper enters the synchronous resolved-controller path",
);
assert.ok(
  browserOpenBranchIndex >= 0 && browserOpenNativeAwaitIndex > browserOpenBranchIndex
    && browserCloseBranchIndex >= 0 && browserCloseNativeAwaitIndex > browserCloseBranchIndex,
  "both browser click paths branch to their synchronous controller before any native await",
);
const browserOpenSection = section(browserPopupController, "const open = async", "const close = async");
const browserOpenGuardIndex = browserOpenSection.indexOf("if (browserPanePopupOpenings.has(pane))");
const browserOpenAcquireIndex = browserOpenSection.indexOf("browserPanePopupOpenings.add(pane);");
const popupOpenIndex = browserOpenSection.indexOf("popup = browserWindow.open(");
const popupNullIndex = browserOpenSection.indexOf("if (popup === null)");
const browserReadyIndex = browserOpenSection.indexOf("await waitForBrowserPanePopupReady(pane, instanceId, popup)");
const browserPersistIndex = browserOpenSection.indexOf("const persisted = persistPoppedPanes(next);");
const browserTrackIndex = browserOpenSection.indexOf("trackBrowserPanePopup(pane, popupRecord);");
const browserSetPoppedIndex = browserOpenSection.indexOf("setPoppedPanes(next);");
const browserOpenReleaseIndex = browserOpenSection.lastIndexOf("browserPanePopupOpenings.delete(pane);");
const browserFirstAwaitIndex = browserOpenSection.search(/\bawait\b/);
assert.ok(
  browserOpenGuardIndex >= 0 && browserOpenAcquireIndex > 0 && popupOpenIndex > browserOpenAcquireIndex
    && popupNullIndex > popupOpenIndex && browserReadyIndex > popupNullIndex
    && browserPersistIndex > browserReadyIndex && browserSetPoppedIndex > browserPersistIndex
    && browserTrackIndex > browserSetPoppedIndex && browserOpenReleaseIndex > browserTrackIndex
    && browserFirstAwaitIndex > popupOpenIndex,
  "browser fallback atomically guards one pane from before popup creation through exact readiness, persistence, hiding, tracking, and release",
);
assert.match(
  browserOpenSection,
  /browserPanePopupOpenings\.add\(pane\);\s*let popupRecord: BrowserPanePopupRecord \| null = null;[\s\S]*?trackBrowserPanePopup\(pane, popupRecord\);\s*return true;\s*\} finally \{[\s\S]*?browserPanePopupOpenings\.delete\(pane\);/,
  "the browser opening guard is released by the finally that encloses every synchronous and awaited popup failure return",
);
assert.match(browserOpenSection, /is already opening in the browser; wait for its exact pane content to become ready/, "a direct concurrent browser open fails visibly before minting another child");
assert.match(browserOpenSection, /params\.set\("syndocalPaneWindowInstance", instanceId\)/, "browser popup requests receive a unique instance query");
assert.match(browserOpenSection, /const target = `syndocal-pane-\$\{pane\}-\$\{instanceId\}`;/, "browser popup targets are bound to the exact pane instance");
const popupNullSection = section(browserOpenSection, "if (popup === null)", "popupRecord = { popup, instanceId, target, closedPollId: null }");
assert.match(popupNullSection, /Allow pop-ups for this site/, "a blocked popup gives the operator an actionable recovery");
assert.match(popupNullSection, /remains integrated in the main window/, "a blocked popup explicitly retains the integrated pane");
assert.doesNotMatch(
  popupNullSection,
  /persistPoppedPanes|setPoppedPanes/,
  "a blocked popup cannot persist or hide a pane",
);
const browserReadinessFailureSection = section(browserOpenSection, "if (!readiness.ok)", "const current = getPoppedPanes()");
assert.match(browserReadinessFailureSection, /retireBrowserPanePopupRecord\(pane, popupRecord\)/, "an unready browser child is retired before main content could hide");
assert.match(browserReadinessFailureSection, /closed before its exact pane content became ready/, "a child closing before readiness stays visible");
assert.match(browserReadinessFailureSection, /did not become ready within/, "a child readiness timeout stays visible");
assert.match(browserReadinessFailureSection, /could not be inspected safely/, "a child inspection failure stays visible");
assert.doesNotMatch(browserReadinessFailureSection, /persistPoppedPanes|setPoppedPanes/, "an unready browser child cannot persist or hide a pane");
const browserPersistFailureSection = section(browserOpenSection, "if (!persisted.ok)", "setPoppedPanes(next);");
assert.match(browserPersistFailureSection, /retireBrowserPanePopupRecord\(pane, popupRecord\)/, "a failed browser persistence retires the exact ready popup when safe");
assert.match(browserPersistFailureSection, /Closing the just-opened pane window also failed/, "a failed compensating popup close remains visible");
assert.match(browserPersistFailureSection, /remains integrated because saving its browser window state failed/, "a throwing localStorage write leaves main content integrated and reports why");
assert.doesNotMatch(
  browserPersistFailureSection,
  /setPoppedPanes/,
  "a failed browser persistence cannot hide the main-window pane",
);
assert.match(
  browserOpenSection.slice(browserSetPoppedIndex),
  /setPoppedPanes\(next\);\s*trackBrowserPanePopup\(pane, popupRecord\);\s*return true;/,
  "a successful browser popup plus exact persisted set still detaches the pane",
);
assert.match(
  closeWrapperSection,
  /if \(!await paneWindowEventsReady\) \{\s*setMessage\("Pane window events are unavailable\."\);\s*return false;\s*\}/,
  "the public close wrapper also keeps gating on pane window events readiness",
);
const browserCloseSection = section(browserPopupController, "const close = async", "const isOpening =");
assert.match(browserCloseSection, /if \(browserPanePopupOpenings\.has\(pane\)\) \{[\s\S]*?is already opening in the browser/, "close during an in-flight browser open fails visibly");
assert.match(browserCloseSection, /const retired = retireBrowserPanePopup\(pane\);/, "browser rejoin retires its exact retained popup before restoring main content");
const browserClosePersistIndex = browserCloseSection.indexOf("const persisted = persistPoppedPanes(next);");
const browserCloseSetPoppedIndex = browserCloseSection.indexOf("setPoppedPanes(next);");
const browserCloseFailureIndex = browserCloseSection.indexOf("if (!persisted.ok ||");
assert.ok(
  browserClosePersistIndex >= 0 && browserCloseSetPoppedIndex > browserClosePersistIndex
    && browserCloseFailureIndex > browserCloseSetPoppedIndex,
  "browser rejoin integrates content even when clearing localStorage fails",
);
assert.match(browserCloseSection, /rejoined the main window, but clearing its browser window state failed/, "a browser rejoin persistence failure stays visible");
assert.match(
  browserCloseSection.slice(browserCloseFailureIndex),
  /return false;[\s\S]*?return true;/,
  "browser rejoin reports storage failure truthfully while preserving normal success",
);
const togglePaneWindowSection = section(app, "const togglePaneWindow =", "const paneWindowOperationPending =");
assert.match(
  togglePaneWindowSection,
  /void \(poppedPanes\(\)\.includes\(pane\) \? closePaneWindow\(pane\) : openPaneWindow\(pane\)\);/,
  "the toggle delegates browser concurrency to the controller guard",
);
const browserPopupTrackingSection = section(browserPopupController, "const nextBrowserPanePopupInstanceId =", "const open = async");
const browserPopupDisposeSection = section(browserPopupController, "const dispose = () =>", "return { open, close, isOpening, dispose }");
assert.match(browserPopupTrackingSection, /browserPanePopups\.get\(pane\)/, "browser popup retirement keys ownership by the exact pane");
assert.match(browserPopupDisposeSection, /browserPanePopupOpenings\.clear\(\);/, "teardown releases any browser-only opening guards");
assert.match(browserPopupDisposeSection, /browserPanePopupPending\.clear\(\);/, "teardown clears an in-flight popup record");
assert.match(browserPopupDisposeSection, /for \(const cancel of cancelReadinessWaiters\) cancel\(\);/, "teardown cancels readiness polling");
assert.match(browserPopupTrackingSection, /record\.popup\.close\(\);/, "rejoin asks the retained exact popup to close");
assert.match(browserPopupTrackingSection, /the browser reported the popup still open after close/, "a false browser close is visible rather than accepted");
assert.match(browserPopupTrackingSection, /browserPanePopupRetirements\.has\(pane\)/, "manual close cannot race a parent-requested retirement");
assert.match(browserPopupTrackingSection, /was closed in the browser and rejoined the main window/, "manual browser closure re-integrates main content visibly");
assert.match(browserPopupTrackingSection, /browserWindow\.setInterval\(/, "successful browser popups install a bounded parent-side closed poll");
assert.match(browserPopupTrackingSection, /record\.popup\.closed/, "manual close is detected only from the retained Window closed state");
assert.match(browserPopupTrackingSection, /popup\.location\.href/, "readiness verifies the same-origin child location");
assert.match(browserPopupTrackingSection, /syndocalPaneWindowInstance/, "readiness verifies the exact child instance query");
assert.match(browserPopupTrackingSection, /popup\.document\.readyState === "loading"/, "readiness waits for the child document to leave loading");
assert.match(browserPopupTrackingSection, /\.app\[data-pane-window-mode="\$\{pane\}"\]/, "readiness requires the exact mounted pane root");
assert.doesNotMatch(browserPopupTrackingSection, /popup\.addEventListener\("beforeunload"/, "parent-side beforeunload popup ownership is fully retired");

const listenerSection = section(app, 'listen<unknown>(PANE_WINDOW_TERMINAL_EVENT', "if (isTauriRuntime() && !paneWindow && autoOpenPaneWindows)");
assert.match(listenerSection, /dispose\(String\(error\)\)/, "listener failure must settle every waiter");
const paneEventTeardownSection = section(listenerSection, "disposePaneWindowEvents = () => {", "onCleanup(() => {");
assert.match(paneEventTeardownSection, /unlistenPaneWindowTerminal\?\.\(\)/, "pane event teardown owns the terminal unlisten");
const lifecycleCleanupSection = listenerSection.slice(listenerSection.indexOf("onCleanup(() => {"));
const teardownIndex = lifecycleCleanupSection.indexOf("disposePaneWindowEvents();");
const unsubscribeIndex = lifecycleCleanupSection.indexOf("unsubscribePaneLifecycleView();");
const disposeIndex = lifecycleCleanupSection.indexOf('paneLifecycleController.dispose(');
assert.ok(
  teardownIndex >= 0 && unsubscribeIndex > teardownIndex && disposeIndex > unsubscribeIndex,
  "cleanup must unlisten and unsubscribe before settling waiters",
);
assert.match(listenerSection, /acknowledgePaneWindowClosed\(terminal\.pane\)/, "destroyed terminals retire the popped record");
assert.match(listenerSection, /applyTerminal\(payload\)/, "terminals flow through the correlation reducer");

assert.match(app, /queryOwnPanePendingCloseContext/, "the child must query its own pending-close context");
const childQuerySection = section(app, "const queryOwnPanePendingCloseContext = async", "const cancelOwnPanePendingClose");
assert.match(childQuerySection, /get_pending_pane_window_close/, "the child uses its caller-derived pending-close command");
assert.doesNotMatch(childQuerySection, /capture_pane_window_placements/, "a child cannot call the main-only placement capture");
const childCancelSection = section(app, "const cancelOwnPanePendingClose = async", "const clearNativeCloseApproval");
assert.match(childCancelSection, /cancel_pane_window_close/, "Keep Open uses the child-only native cancel");
assert.match(childCancelSection, /instanceId: context\.instance_id/, "native cancel carries the exact instance");
assert.match(childCancelSection, /requestId: context\.request_id/, "native cancel carries the correlated request");
const paneTerminalRecoverySection = section(
  app,
  "const paneWindowTerminalRecoveryCommands = new Set([",
  "// Availability reads only inspect",
);
assert.match(paneTerminalRecoverySection, /"get_pending_pane_window_close"/, "Full Lock keeps the child pending-close query available");
assert.match(paneTerminalRecoverySection, /"cancel_pane_window_close"/, "Full Lock keeps the exact correlated cancel terminal available");
assert.doesNotMatch(
  paneTerminalRecoverySection,
  /"(?:open|close|capture)_pane_window|"capture_pane_window_placements"/,
  "Full Lock recovery never admits a new parent pane open/close/capture operation",
);
const invokeFacadeSection = section(app, "const invoke = async <T,>(", "const requestedExpectedEpoch");
assert.match(
  invokeFacadeSection,
  /const terminalRecovery = mediaAssetTerminalRecoveryCommands\.has\(command\)\s*\|\| paneWindowTerminalRecoveryCommands\.has\(command\)/,
  "the generic invoke facade bypasses operator lock only for the bounded pane terminal recovery set",
);
const onCancelRequestedTimeline = section(app, 'if (paneWindow === "timeline") {', "if (protectedCloseRefreshInFlight || protectedCloseRequest() !== null)");
assert.match(onCancelRequestedTimeline, /pendingClose: pendingContext/, "protected-close state must attach the full pending context");
assert.match(onCancelRequestedTimeline, /pendingCloseQueryFailed/, "query failure remains explicit and retryable");
assert.match(onCancelRequestedTimeline, /event\.preventDefault\(\)/, "dirty children must prevent the close");
const cancelProtectedCloseSection = section(app, "const cancelProtectedClose = async () => {", "const completeProtectedClose = async");
assert.match(cancelProtectedCloseSection, /cancelOwnPanePendingClose\(pendingContext\)/, "Keep Open cancels the matching full parent context");
assert.match(cancelProtectedCloseSection, /pendingCloseQueryFailed \|\| pendingContext === null/, "query failures retry before any modal clear");
assert.ok(
  cancelProtectedCloseSection.indexOf("setProtectedCloseRequest(null)")
    > cancelProtectedCloseSection.indexOf("await cancelOwnPanePendingClose(pendingContext)"),
  "Keep Open clears the modal only after exact native cancellation succeeds",
);

assert.match(operations, /paneTransitions: Partial<Record<PaneWindowKind, PaneWindowOperationPhase>>/, "menu takes pane-pending input");
assert.match(operations, /workspaceBusy: boolean/, "menu takes workspace-busy input");
assert.match(operations, /props\.workspaceBusy \|\| Object\.keys\(props\.paneTransitions\)\.length > 0/, "workspace busy derives from transitions");
assert.match(operations, /aria-busy=\{[^}]*paneTransitionBusy\(pane\)[^}]*\}/, "pane buttons expose aria-busy");
assert.match(operations, /aria-busy=\{props\.workspaceBusy \? "true" : undefined\}/, "Apply exposes workspace aria-busy");
assert.match(operations, /disabled=\{props\.operatorLockMode !== null \|\| paneTransitionBusy\(pane\) \|\| props\.workspaceBusy\}/, "pane buttons disable per operation");
assert.ok(
  [...operations.matchAll(/workspaceTransitionBusy\(\)/g)].length >= 4,
  "Save/select/Apply/Delete must all gate on workspace transitions",
);
assert.match(operations, /data-workspace-pane-toggle=\{pane\}/, "pane toggle hooks remain stable");
assert.match(app, /paneTransitions=\{pendingPaneWindowOperations\(\)\}/, "App feeds pane-pending into the menu");
assert.match(app, /workspaceBusy=\{workspaceApplyBusy\(\)\}/, "App feeds workspace-busy into the menu");

const operationsRenderSection = section(app, "operations={", "onWorkspaceTab={selectWorkspaceTab}");
assert.match(
  operationsRenderSection,
  /paneWindow \? null : \(/,
  "pane children never render the operations menu or its popout toggles",
);
assert.match(operationsRenderSection, /<WorkspaceOperationsMenu/, "the main window keeps its operations menu");
assert.equal(
  [...app.matchAll(/<WorkspaceOperationsMenu/g)].length,
  1,
  "the operations menu has exactly one gated render site",
);
assert.doesNotMatch(
  app,
  /data-workspace-pane-toggle/,
  "App itself renders no direct popout toggles outside the gated menu",
);

assert.match(mainRs, /struct PaneWindowLifecycleRegistry/, "native registry exists separately from AppState");
assert.match(mainRs, /\.manage\(PaneWindowLifecycleRegistry::default\(\)\)/, "registry is managed app-global");
const registryDeclSection = section(mainRs, "struct PaneWindowLifecycleRegistry", "fn establish_identity");
assert.ok(!registryDeclSection.includes("AppState") && !registryDeclSection.includes("EngineHandle"), "registry stays narrow and separate from AppState");
assert.match(mainRs, /fn validate_pane_window_opaque_id/, "opaque ids fail closed");
assert.match(mainRs, /fn ensure_pane_window_caller_is_main/, "open/close arming is main-scoped");
assert.match(mainRs, /fn ensure_pane_window_caller_label_matches_pane/, "child commands are caller-label scoped");
assert.match(mainRs, /FULL_LOCK_CORRELATED_TERMINAL_RECOVERY_RUNTIME_ROUTES: \[&str; 1\][\s\S]*?"cancel_pane_window_close"/, "backend Full Lock recovery allowlist contains only the correlated cancel route");
assert.match(mainRs, /else if !is_full_lock_correlated_terminal_recovery_runtime_route\(command\)/, "backend ordinary runtime mutations still cross the Full Lock gate");
const terminalPayload = section(mainRs, "fn pane_window_terminal_payload(", "fn emit_pane_window_terminal");
assert.match(terminalPayload, /"pane": pane/, "terminal payload key");
assert.match(terminalPayload, /"instance_id": instance_id/, "terminal payload key consistency");
assert.match(terminalPayload, /"request_id": request_id/, "terminal payload key consistency");
assert.match(terminalPayload, /"terminal": terminal/, "terminal payload key consistency");
const destroyRetireSection = section(mainRs, "fn retire_destroyed_pane_window_identity(", "async fn open_pane_window(");
assert.ok(
  destroyRetireSection.search(/entries\s*\.remove\(pane\)/) !== -1
  && destroyRetireSection.indexOf("drop(entries);") < destroyRetireSection.indexOf("emit_pane_window_terminal("),
  "destroyed hook retires the exact entry atomically before emitting",
);
assert.match(destroyRetireSection, /entry\.instance_id != instance_id/, "destroyed retirement is exact-incarnation guarded");
assert.match(destroyRetireSection, /fn install_pane_window_destroyed_observer/, "each newborn pane installs its own observer");
assert.match(destroyRetireSection, /callback_instance_id/, "the observer captures the exact native incarnation");
const globalDestroySection = section(mainRs, ".on_window_event(|window, event|", ".plugin(tauri_plugin_updater");
assert.doesNotMatch(globalDestroySection, /strip_prefix\("pane-"\)/, "the global label-only hook cannot retire pane identity");
const closeCommandSection = section(mainRs, "async fn close_pane_window(", "async fn open_video_output_window");
assert.match(closeCommandSection, /fn get_pending_pane_window_close/, "child query has a dedicated caller-derived command");
assert.match(closeCommandSection, /fn cancel_pane_window_close/, "child cancel has a dedicated caller-derived command");
assert.doesNotMatch(closeCommandSection, /ACTION_QUERY_PENDING|ACTION_CANCEL_PENDING/, "close is not multiplexed by child-controlled action strings");
const missingTargetAnchorIndex = closeCommandSection.indexOf(
  "let Some(target) = app.webview_windows().get(&label).cloned() else {",
);
const targetCloseIndex = closeCommandSection.indexOf("if let Err(error) = target.close()");
assert.ok(
  missingTargetAnchorIndex >= 0 && targetCloseIndex > missingTargetAnchorIndex,
  "the native-target-missing close branch exists between arm and close",
);
const missingTargetBranchSection = closeCommandSection.slice(missingTargetAnchorIndex, targetCloseIndex);
assert.ok(
  missingTargetBranchSection.includes("retire_destroyed_pane_window_identity(&app, &pane, &instance_id)?;")
  && missingTargetBranchSection.includes("return Ok(());"),
  "missing-window close retires through the recoverable exact-identity helper and keeps the synchronous terminal",
);
assert.doesNotMatch(
  missingTargetBranchSection,
  /emit_pane_window_terminal\(/,
  "no direct emit may bypass reinsert-on-emission-failure recovery in the missing-window branch",
);
assert.doesNotMatch(
  missingTargetBranchSection,
  /PaneWindowLifecycleRegistry::retire_identity/,
  "no raw retire can strand the armed request when emission fails",
);
assert.match(closeCommandSection, /rollback_pending_close/, "failed native close requests roll back their arm");
assert.match(closeCommandSection, /PaneWindowLifecycleRegistry::arm_pending_close/, "close arming goes through the registry");
assert.ok(
  closeCommandSection.indexOf("PaneWindowLifecycleRegistry::arm_pending_close")
    < closeCommandSection.indexOf("app.webview_windows().get(&label)"),
  "missing-window detection happens only after the exact close arm",
);
assert.match(mainRs, /already has exactly one pending close armed/, "exactly one pending close");
const statusReportSection = section(mainRs, "struct PaneWindowStatusReport", "fn capture_pane_window_placements");
assert.match(statusReportSection, /instance_id: Option<String>/, "status exposes tracked instances");
assert.match(statusReportSection, /pending_close_request_id: Option<String>/, "status exposes pending requests");

assert.match(
  destroyRetireSection,
  /or_insert\(PaneWindowLifecycleEntry \{/,
  "terminal emission failure reinserts the exact recoverable entry",
);
assert.match(
  destroyRetireSection,
  /pending_close: retired\.pending_close,/,
  "reinsertion preserves the exact pending close for correlation",
);
assert.ok(
  destroyRetireSection.indexOf("or_insert(PaneWindowLifecycleEntry {")
    > destroyRetireSection.indexOf("emit_pane_window_terminal("),
  "reinsertion happens only after the emission attempt failed",
);

assert.match(mainRs, /fn reconcile_missing_pane_windows/, "a main-only sweep retries lost pane terminals");
const reconcileSweepSection = section(mainRs, "fn reconcile_missing_pane_windows", "fn install_pane_window_destroyed_observer");
assert.match(
  reconcileSweepSection,
  /contains_key\(&pane_window_label\(&pane\)\)/,
  "the sweep skips panes whose native window still lives",
);
assert.match(
  reconcileSweepSection,
  /retire_destroyed_pane_window_identity\(app, &pane, &instance_id\)/,
  "the sweep retries only the exact tracked incarnation",
);
const captureCommandSection = section(mainRs, "fn capture_pane_window_placements(", "const PANE_WINDOW_OPAQUE_ID_MAX_BYTES");
assert.ok(
  captureCommandSection.indexOf("ensure_pane_window_caller_is_main(window.label())?;")
    < captureCommandSection.indexOf("reconcile_missing_pane_windows(&app)?;"),
  "every main-only capture doubles as a reconciliation pass",
);

assert.match(
  mainRs,
  /fn pane_window_pending_close_rollback_succeeds_exactly_and_treats_mismatches_as_no_ops/,
  "rollback success and instance/request mismatch no-ops are covered by Rust tests",
);

assert.equal(tauriConf.app?.windows?.[0]?.label, "main", "the configured first window label is pinned to 'main'");

assert.match(
  localization,
  /"Timeline edits will be discarded\. The main window is waiting for this Timeline window\.":/,
  "the new protected-close copy must be localized",
);

console.log("pane window lifecycle correlation: all fixtures and source gates passed");
