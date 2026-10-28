// Automatic output ownership at start / project open (task75).
// Executes the dependency-free policy, the blackout controller and the real
// Enable dispatch path against fakes; then pins the UI wiring by source.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

// Checkouts may use CRLF working files; compare sources with LF line ends.
const read = async (path) => (await readFile(new URL(path, import.meta.url), "utf8")).replace(/\r\n/g, "\n");
const [policySource, driveSource, blackoutSource, outputControlSource, appSource, panelSource, runtimeSource, localizationSource] =
  await Promise.all([
    read("../src/outputAutoOwnership.ts"),
    read("../src/outputDriveStatus.ts"),
    read("../src/createTargetBlackoutController.ts"),
    read("../src/outputControlController.ts"),
    read("../src/App.tsx"),
    read("../src/components/StandbySyncPanel.tsx"),
    read("../src/outputAutoOwnershipRuntime.ts"),
    read("../src/uiLocalization.ts"),
  ]);

const load = (source, fileName, resolve) => {
  const exports = {};
  const { outputText } = ts.transpileModule(source, {
    compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2022 },
    fileName,
  });
  new Function("require", "exports", outputText)((name) => {
    const resolved = resolve?.(name);
    if (resolved) return resolved;
    assert.fail(`${fileName} must not require ${name} at runtime`);
  }, exports);
  return exports;
};

const policy = load(policySource, "outputAutoOwnership.ts");
const drive = load(driveSource, "outputDriveStatus.ts");

// ---------------------------------------------------------------- fixtures
const startupOwnership = (overrides = {}) => ({
  state: "Failed", effective_role: "Standby", desired_role: "Standby", persisted_role: "Standby",
  lighting_reason: "StartupDenied", video_reason: "StartupDenied", epoch: 0, generation: 0, ...overrides,
});
const projectSwapOwnership = (overrides = {}) => ({
  state: "Ready", effective_role: "Standby", desired_role: "Both", persisted_role: "Both",
  lighting_reason: "ProjectSwapDisarmed", video_reason: "ProjectSwapDisarmed", epoch: 3, generation: 7, ...overrides,
});
const chosenStandbyOwnership = (overrides = {}) => ({
  state: "Ready", effective_role: "Standby", desired_role: "Standby", persisted_role: "Standby",
  lighting_reason: "BlockedByMachineRole", video_reason: "BlockedByMachineRole", epoch: 2, generation: 5, ...overrides,
});
const ownedOwnership = (overrides = {}) => ({
  state: "Ready", effective_role: "Both", desired_role: "Both", persisted_role: "Both",
  lighting_reason: "OwnedByMachineRole", video_reason: "OwnedByMachineRole", epoch: 2, generation: 9, ...overrides,
});
const authority = (id) => ({ lease_id: id, generation: 1 });
const noLease = { operation_id: "syndocal.output.lease.authority.query.v1", statuses: [{ status: "unavailable" }] };
const held = (status, resources, id = "lease-a") => ({ status, authority: authority(id), resources });
const leases = (...statuses) => ({ operation_id: "syndocal.output.lease.authority.query.v1", statuses });
const bothLease = leases(held("held_active", ["lighting", "video"]));
const inputs = (overrides = {}) => ({
  settingOn: true,
  rememberedStandbySyncRole: null,
  standbySync: { running: false, role: null, split_brain: false },
  killSwitchActive: false,
  operatorStandby: false,
  ownership: startupOwnership(),
  leases: noLease,
  ...overrides,
});
const decide = (overrides) => policy.decideAutoOutputOwnership(inputs(overrides));

// ---------------------------------------------------------------- decision
assert.deepEqual(decide({}), { action: "take", trigger: "start" }, "a normal start (stored Standby after close) is taken");
assert.deepEqual(decide({ ownership: startupOwnership({ desired_role: "Both", persisted_role: "Both" }) }),
  { action: "take", trigger: "start" }, "a first run (stored Both) is taken");
assert.deepEqual(decide({ ownership: projectSwapOwnership() }), { action: "take", trigger: "project" }, "a project open is re-taken");
assert.deepEqual(decide({ ownership: projectSwapOwnership({ desired_role: "Standby" }) }),
  { action: "take", trigger: "project" }, "a project opened before the first take is taken");
assert.deepEqual(decide({ ownership: startupOwnership(), leases: leases(held("held_orphaned", ["lighting", "video"])) }),
  { action: "take", trigger: "start" }, "this machine's orphaned Both lease is recovered by Enable, as Prepare does");
assert.deepEqual(decide({ ownership: ownedOwnership(), leases: bothLease }), { action: "owned" });
assert.deepEqual(decide({ settingOn: false, ownership: ownedOwnership(), leases: bothLease }), { action: "owned" },
  "turning the switch off never releases outputs that are already sent");

const refusals = [
  ["setting_off", { settingOn: false }],
  ["backup_computer", { rememberedStandbySyncRole: "standby" }],
  ["standby_sync_backup", { standbySync: { running: true, role: "standby", split_brain: false } }],
  ["split_brain", { standbySync: { running: true, role: "primary", split_brain: true } }],
  ["status_unavailable", { standbySync: null }],
  ["kill_switch", { killSwitchActive: true }],
  ["status_unavailable", { killSwitchActive: null }],
  ["operator_standby", { operatorStandby: true }],
  ["operator_standby", { operatorStandby: true, ownership: projectSwapOwnership() }],
  ["other_lease", { leases: leases(held("held_active", ["lighting"])) }],
  ["other_lease", { leases: leases(held("held_active", ["video"])) }],
  ["other_lease", { leases: leases(held("held_orphaned", ["video"])) }],
  ["other_lease", { leases: leases(held("held_active", ["lighting", "video"], "a"), held("held_active", ["lighting", "video"], "b")) }],
  ["other_lease", { leases: leases(held("held_orphaned", ["lighting", "video"], "a"), held("held_active", ["lighting"], "b")) }],
  ["busy", { ownership: startupOwnership({ state: "Transitioning" }) }],
  ["busy", { ownership: projectSwapOwnership({ state: "Activating" }) }],
  ["startup_failed", { ownership: startupOwnership({ persisted_role: null }) }],
  ["startup_failed", { ownership: startupOwnership({ desired_role: "Standby", persisted_role: "Both" }) }],
  ["partial_role", { ownership: startupOwnership({ desired_role: "Video", persisted_role: "Video" }) }],
  ["partial_role", { ownership: projectSwapOwnership({ desired_role: "Lighting" }) }],
  ["transition_failed", { ownership: startupOwnership({ lighting_reason: "TransitionFailed", video_reason: "TransitionFailed" }) }],
  ["operator_standby", { ownership: chosenStandbyOwnership() }],
  ["needs_operator", { ownership: ownedOwnership() }],
  ["needs_operator", { ownership: startupOwnership({ video_reason: "TransitionFailed" }) }],
];
for (const [reason, overrides] of refusals) {
  assert.deepEqual(decide(overrides), { action: "skip", reason }, `${reason}: ${JSON.stringify(overrides)}`);
}
// Every hard refusal outranks an otherwise eligible start and project open.
for (const ownership of [startupOwnership(), projectSwapOwnership()]) {
  for (const [reason, overrides] of refusals.slice(0, 14)) {
    assert.equal(decide({ ownership, ...overrides }).action, "skip", `${reason} must refuse ${ownership.lighting_reason}`);
  }
}

// ---------------------------------------------------------------- messages
const developerTerms = /lease|fence|R4|principal|kill switch|OutputControl|Both role|sidecar|S0|arm\b/i;
const reasons = [...new Set(refusals.map(([reason]) => reason))];
for (const reason of reasons) {
  const message = policy.autoOutputStatusMessage({ kind: "skipped", reason });
  assert.ok(message && message.length > 10, `${reason} has a message`);
  assert.doesNotMatch(message, developerTerms, `${reason} message is plain: ${message}`);
  assert.ok(localizationSource.includes(JSON.stringify(message) + ":"), `${reason} message is translated`);
}
for (const text of [policy.AUTO_OUTPUT_SETTING_LABEL, policy.AUTO_OUTPUT_SETTING_HINT, policy.AUTO_OUTPUT_OWNED_MESSAGE,
  policy.AUTO_OUTPUT_TAKING_MESSAGE, drive.OUTPUT_DRIVE_NOT_TAKEN_MESSAGE,
  `No blackout was applied. ${drive.OUTPUT_DRIVE_NOT_TAKEN_MESSAGE}.`]) {
  assert.doesNotMatch(text, developerTerms, text);
  assert.ok(localizationSource.includes(JSON.stringify(text) + ":"), `translated: ${text}`);
}
assert.equal(policy.autoOutputStatusMessage({ kind: "unknown" }), null);
assert.equal(policy.autoOutputStatusMessage({ kind: "failed", detail: "x" }), "Outputs were not taken automatically: x");
assert.match(localizationSource, /\/\^Outputs were not taken automatically: \(\.\+\)\$\//, "failure detail has a Japanese pattern");

// ---------------------------------------------------------------- drive status
assert.deepEqual(drive.classifyOutputDrive(startupOwnership(), false),
  { kind: "blocked", reason: "not_taken", message: "Output off: this machine has not taken the outputs yet" },
  "start is explained as not taken, not as a role change in progress");
assert.equal(drive.classifyOutputDrive(projectSwapOwnership(), false).reason, "not_taken", "a project open is not a chosen Standby");
assert.equal(drive.classifyOutputDrive(chosenStandbyOwnership(), false).reason, "standby");
assert.equal(drive.classifyOutputDrive(startupOwnership({ state: "Transitioning", lighting_reason: "Transitioning" }), false).reason, "fenced");
assert.equal(drive.classifyOutputDrive({ state: "Ready", effective_role: "Both", desired_role: "Both" }, false).reason, "no_lease",
  "callers without a reason keep the previous classification");
assert.deepEqual(drive.classifyOutputDrive(startupOwnership(), true), { kind: "driving" });

// ---------------------------------------------------------------- controller
const fakeBackend = (initial) => {
  const backend = {
    ownership: initial.ownership,
    leases: initial.leases ?? noLease,
    guards: { standbySync: { running: false, role: null, split_brain: false }, killSwitchActive: false, rememberedStandbySyncRole: null },
    enables: [],
    enableResult: "own",
    beforeDispatch: null,
    duringEnable: null,
    reads: 0,
  };
  backend.controller = (extra = {}) => policy.createOutputAutoOwnershipController({
    readOwnership: async () => { backend.reads += 1; return { ownership: backend.ownership, leases: backend.leases }; },
    readGuards: async () => {
      await backend.beforeGuards?.();
      return backend.guards;
    },
    readSetting: () => backend.setting ?? true,
    enable: async (abortReason) => {
      await backend.beforeDispatch?.();
      const reason = abortReason();
      if (reason !== null) throw new Error(reason);
      backend.enables.push(backend.ownership.lighting_reason);
      await backend.duringEnable?.();
      if (backend.enableResult === "own") {
        backend.ownership = ownedOwnership({ epoch: backend.ownership.epoch + 1, generation: backend.ownership.generation + 1 });
        backend.leases = bothLease;
        return { outcome: "applied" };
      }
      throw new Error(backend.enableResult);
    },
    onStatus: (status) => (backend.statuses ??= []).push(status),
    ...extra,
  });
  return backend;
};

policy.resetOperatorIntentForTests();
{
  // Auto-take on start, then on project open.
  const backend = fakeBackend({ ownership: startupOwnership() });
  const controller = backend.controller();
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  assert.deepEqual(backend.enables, ["StartupDenied"]);
  assert.deepEqual(backend.statuses, [{ kind: "taking" }, { kind: "owned" }]);
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  assert.equal(backend.enables.length, 1, "an owned machine is not enabled again");
  backend.ownership = projectSwapOwnership({ epoch: 9, generation: 20 });
  backend.leases = noLease;
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  assert.deepEqual(backend.enables, ["StartupDenied", "ProjectSwapDisarmed"]);
}
{
  // Single flight: concurrent evaluations (poll + blackout pre-check) send one Enable.
  const backend = fakeBackend({ ownership: startupOwnership() });
  let release;
  backend.duringEnable = () => new Promise((resolve) => { release = resolve; });
  const controller = backend.controller();
  const first = controller.evaluate();
  const second = controller.evaluate();
  assert.strictEqual(first, second);
  while (!release) await new Promise((resolve) => setImmediate(resolve));
  let settled = false;
  const waiting = controller.settled().then(() => { settled = true; });
  await new Promise((resolve) => setImmediate(resolve));
  assert.equal(settled, false, "settled waits for the in-flight take");
  release();
  await Promise.all([first, waiting]);
  assert.equal(backend.enables.length, 1);
  await controller.settled();
}
{
  // Setting off: never asks.
  const backend = fakeBackend({ ownership: startupOwnership() });
  backend.setting = false;
  assert.deepEqual(await backend.controller().evaluate(), { kind: "skipped", reason: "setting_off" });
  assert.equal(backend.enables.length, 0);
  // Turned off while the take is being prepared: aborted before dispatch.
  backend.setting = true;
  backend.beforeDispatch = () => { backend.setting = false; };
  assert.deepEqual(await backend.controller().evaluate(), { kind: "skipped", reason: "setting_off" });
  assert.equal(backend.enables.length, 0);
}
{
  // Each refusal is a hard stop for the controller too.
  for (const [reason, guards] of [
    ["backup_computer", { rememberedStandbySyncRole: "standby" }],
    ["standby_sync_backup", { standbySync: { running: true, role: "standby", split_brain: false } }],
    ["kill_switch", { killSwitchActive: true }],
    ["status_unavailable", { killSwitchActive: null }],
  ]) {
    const backend = fakeBackend({ ownership: startupOwnership() });
    backend.guards = { ...backend.guards, ...guards };
    assert.deepEqual(await backend.controller().evaluate(), { kind: "skipped", reason });
    assert.equal(backend.enables.length, 0, reason);
  }
  const foreign = fakeBackend({ ownership: startupOwnership(), leases: leases(held("held_active", ["lighting"])) });
  assert.deepEqual(await foreign.controller().evaluate(), { kind: "skipped", reason: "other_lease" });
  assert.equal(foreign.enables.length, 0);
  const unreadable = fakeBackend({ ownership: startupOwnership() });
  const failing = policy.createOutputAutoOwnershipController({
    readOwnership: async () => { throw new Error("ipc down"); },
    readGuards: async () => unreadable.guards,
    readSetting: () => true,
    enable: async () => assert.fail("unreadable state must not enable"),
  });
  assert.deepEqual(await failing.evaluate(), { kind: "skipped", reason: "status_unavailable" });
}
{
  // Backend refusals: a hard refusal is not retried for the same state; a
  // transient one is retried once; a new backend state may be tried again.
  const hard = fakeBackend({ ownership: startupOwnership() });
  hard.enableResult = "OutputControl rejected (forbidden); nothing was applied.";
  const hardController = hard.controller();
  assert.equal((await hardController.evaluate()).kind, "failed");
  assert.equal((await hardController.evaluate()).kind, "failed");
  assert.equal(hard.enables.length, 1, "a hard refusal is final for this backend state");
  hard.ownership = startupOwnership({ generation: 1 });
  await hardController.evaluate();
  assert.equal(hard.enables.length, 2);
  const transient = fakeBackend({ ownership: startupOwnership() });
  transient.enableResult = "OutputControl rejected (stale_fence); nothing was applied.";
  const transientController = transient.controller();
  for (let index = 0; index < 5; index += 1) await transientController.evaluate();
  assert.equal(transient.enables.length, 2, "a transient refusal is retried once, never in a loop");
  // A failed transition keeps its concrete failure visible.
  const failed = fakeBackend({ ownership: startupOwnership() });
  failed.enableResult = "Output role worker failed";
  const failedController = failed.controller();
  await failedController.evaluate();
  failed.ownership = startupOwnership({ lighting_reason: "TransitionFailed", video_reason: "TransitionFailed", generation: 1 });
  assert.deepEqual(await failedController.evaluate(), { kind: "failed", detail: "Output role worker failed" });
  assert.equal(failed.enables.length, 1);
}
{
  // Standby always wins (operator UI path).
  policy.resetOperatorIntentForTests();
  const backend = fakeBackend({ ownership: startupOwnership() });
  backend.beforeDispatch = () => policy.noteOperatorStandbyRequested();
  const controller = backend.controller();
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" },
    "a Standby chosen after the decision cancels the take before it is sent");
  assert.equal(backend.enables.length, 0);
  backend.beforeDispatch = null;
  // The later Standby and project open never re-take.
  backend.ownership = chosenStandbyOwnership();
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  backend.ownership = projectSwapOwnership({ desired_role: "Standby", epoch: 9, generation: 30 });
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  assert.equal(backend.enables.length, 0, "project replacement after Standby stays Standby");
  // An explicit Enable ends the suppression.
  policy.noteOperatorTookOutputs();
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  assert.equal(backend.enables.length, 1);
}
{
  // Standby chosen while the Enable is already in flight: this take finishes
  // (the backend Standby boundary then retires it), and nothing re-takes later.
  policy.resetOperatorIntentForTests();
  const backend = fakeBackend({ ownership: startupOwnership() });
  backend.duringEnable = () => policy.noteOperatorStandbyRequested();
  const controller = backend.controller();
  await controller.evaluate();
  backend.ownership = chosenStandbyOwnership({ epoch: 5, generation: 50 });
  backend.leases = noLease;
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  backend.ownership = projectSwapOwnership({ epoch: 6, generation: 60, desired_role: "Standby" });
  await controller.evaluate();
  assert.equal(backend.enables.length, 1);
}
{
  // Standby from any caller (MCP, Standby sync) is observed from the backend.
  policy.resetOperatorIntentForTests();
  const backend = fakeBackend({ ownership: chosenStandbyOwnership({ epoch: 4, generation: 10 }) });
  const controller = backend.controller();
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  backend.ownership = projectSwapOwnership({ epoch: 4, generation: 11, desired_role: "Standby" });
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  // A stale owned read from before the Standby cannot end the suppression.
  backend.ownership = ownedOwnership({ epoch: 4, generation: 9 });
  backend.leases = bothLease;
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  backend.ownership = projectSwapOwnership({ epoch: 4, generation: 12, desired_role: "Standby" });
  backend.leases = noLease;
  assert.deepEqual(await controller.evaluate(), { kind: "skipped", reason: "operator_standby" });
  // Outputs explicitly taken later (Enable, Prepare, Arm, Take Over) end it.
  backend.ownership = ownedOwnership({ epoch: 4, generation: 13 });
  backend.leases = bothLease;
  await controller.evaluate();
  backend.ownership = projectSwapOwnership({ epoch: 5, generation: 14 });
  backend.leases = noLease;
  assert.deepEqual(await controller.evaluate(), { kind: "owned" });
  assert.equal(backend.enables.length, 1);
  policy.resetOperatorIntentForTests();
}

// ------------------------------------------- blackout works right after start
{
  policy.resetOperatorIntentForTests();
  const blackoutCalls = [];
  const backend = fakeBackend({ ownership: startupOwnership() });
  const controller = backend.controller();
  const blackout = load(blackoutSource, "createTargetBlackoutController.ts", (name) => {
    if (name === "./outputDriveStatus") return drive;
    if (name === "./outputControlController") {
      return {
        executeTargetBlackout: async (_invoke, target, enabled) => {
          assert.deepEqual(backend.leases, bothLease, "blackout is sent only with the taken lease");
          blackoutCalls.push([target, enabled]);
          return { outcome: "applied", fence_after: {} };
        },
        executeBlackoutRelease: async () => assert.fail("no S0 release"),
      };
    }
    return null;
  });
  const messages = [];
  const target = blackout.createTargetBlackoutController({
    invoke() { throw new Error("unexpected direct IPC"); },
    safety: { engage: async () => undefined },
    safetyBlackoutEngaged: () => false,
    refreshSnapshot: async () => undefined,
    setMessage: (message) => messages.push(message),
    clearMessage: () => undefined,
    // App.readOutputDriveForAction: let a pending automatic take finish first.
    checkOutputDrive: async () => {
      await controller.evaluate();
      return drive.classifyOutputDrive(backend.ownership, backend.leases === bothLease);
    },
  });
  // The very first action after start, before any poll ran.
  await target.setAllBlackout(true);
  assert.deepEqual(blackoutCalls, [["both", true]], "blackout applies immediately after start");
  assert.deepEqual(messages, []);
  assert.equal(backend.enables.length, 1);
  // With the switch off the refusal says why, in plain words.
  const off = fakeBackend({ ownership: startupOwnership() });
  off.setting = false;
  const offController = off.controller();
  const offTarget = blackout.createTargetBlackoutController({
    invoke() { throw new Error("unexpected direct IPC"); },
    safety: { engage: async () => undefined },
    refreshSnapshot: async () => undefined,
    setMessage: (message) => messages.push(message),
    checkOutputDrive: async () => { await offController.evaluate(); return drive.classifyOutputDrive(off.ownership, false); },
  });
  await offTarget.setLightingBlackout(true);
  assert.deepEqual(messages, ["No blackout was applied. Output off: this machine has not taken the outputs yet."]);
  assert.equal(blackoutCalls.length, 1);
}

// ------------------------------- real Enable dispatch: abort before sending
{
  const outputControl = load(outputControlSource, "outputControlController.ts", (name) => {
    if (name === "./networkDmxRoutes") return { validateNetworkDmxRoutes: () => undefined };
    return null;
  });
  const fence = {
    process_incarnation: 1, session_incarnation: 1, project_epoch: 0, project_revision: 0,
    project_checkpoint_hash: "a".repeat(64), project_publication_generation: 0,
    output_epoch: 1, output_generation: 1, safety_blackout_epoch: 1, safety_blackout_generation: 1,
  };
  const commands = [];
  let abort = "Automatic output was cancelled because Standby was chosen; nothing was applied.";
  const invoke = async (command) => {
    commands.push(command);
    if (command === "query_output_control_authority_v1") return { operation_id: "syndocal.query.output.control.authority.v1", fence };
    if (command === "query_output_lease_authority_v1") return noLease;
    if (command === "allocate_output_control_request_id_v1") return 7;
    throw new Error(`dispatched ${command}`);
  };
  const authorityOperation = /OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID\s*=\s*"([^"]+)"/.exec(outputControlSource)?.[1];
  assert.ok(authorityOperation);
  const authorityInvoke = async (command, args) => {
    if (command === "query_output_control_authority_v1") {
      commands.push(command);
      return { operation_id: authorityOperation, fence };
    }
    return invoke(command, args);
  };
  await assert.rejects(outputControl.enableOutputUnlessCancelled(authorityInvoke, () => abort), /Standby was chosen/);
  assert.deepEqual(commands.slice(-1), ["allocate_output_control_request_id_v1"], "the fence is captured, nothing is dispatched");
  assert.ok(!commands.includes("enable_output_control_v2"));
  assert.ok(commands.includes("query_output_control_authority_v1"));
  abort = null;
  commands.length = 0;
  await assert.rejects(outputControl.enableOutputUnlessCancelled(authorityInvoke, () => abort), /dispatched|reply was lost/);
  assert.ok(commands.includes("enable_output_control_v2"),
    `without an abort reason the identical Enable is dispatched: ${commands.join(",")}`);
  assert.equal(commands.indexOf("enable_output_control_v2"), commands.indexOf("allocate_output_control_request_id_v1") + 1);
}

// ------------------------------------------------------------ source wiring
const order = (source, first, second, message) => {
  const a = source.indexOf(first);
  const b = source.indexOf(second, a);
  assert.ok(a >= 0 && b > a, message);
};
order(outputControlSource, "const requestId = await allocateRequestId(invoke);", "const abortReason = options.abortBeforeDispatch?.() ?? null;",
  "the abort check runs after the last await before dispatch");
order(outputControlSource, "if (abortReason !== null) throw new Error(abortReason);", "terminal = await invoke<unknown>(command, executeArgs);",
  "the abort check precedes the dispatch");
assert.match(outputControlSource, /export async function enableOutputUnlessCancelled[\s\S]*?\{ kind: "enable_output" \}, \{ abortBeforeDispatch: abortReason \}/,
  "automatic ownership uses the identical Enable request");
assert.match(appSource, /enable: \(abortReason\) => enableOutputUnlessCancelled\(invoke, abortReason\)/);
assert.match(appSource, /checkOutputDrive: readOutputDriveForAction,/, "ordinary blackout waits for a pending automatic take");
assert.match(appSource, /const drive = await readOutputDriveForAction\(\);/, "Perform blackout uses the same pre-check");
assert.match(appSource, /const autoOutputActive = \(\) => isTauriRuntime\(\) && !paneWindow;/, "only the main desktop window takes outputs");
assert.match(appSource, /if \(autoOutputActive\(\)\) await autoOutput\.evaluate\(\);\s*if \(!disposed\) await readOutputDrive\(\);/,
  "the drive poll evaluates automatic ownership at start and after every project open");
assert.match(panelSource, /const standbyRoleStorageKey = "syndocal\.standbySyncRole";/);
assert.equal(policy.STANDBY_SYNC_ROLE_STORAGE_KEY, "syndocal.standbySyncRole", "the backup marker is the Standby sync role");
order(panelSource, "const requestStandby = async", "noteOperatorStandbyRequested();", "Standby records intent");
order(panelSource, "noteOperatorStandbyRequested();\n    // A poll that started", "\"set_output_ownership_role\"",
  "Standby intent is recorded before the Standby request is sent");
order(panelSource, "if (role() === \"standby\") noteOperatorStandbyRequested();", "\"start_standby_sync\"",
  "following as the backup records Standby intent first");
assert.match(panelSource, /data-io-control="auto-output-ownership"[\s\S]*?checked=\{autoOutputSetting\(\)\}[\s\S]*?onChange=\{\(event\) => setAutoOutputSetting\(event\.currentTarget\.checked\)\}/,
  "Setup has one plain switch, applied without a dialog");
assert.match(panelSource, /data-io-control="auto-output-status"/, "the automatic outcome is shown");
assert.doesNotMatch(panelSource.slice(panelSource.indexOf("autoOutputOwnership"), panelSource.indexOf("autoOutputOwnership") + 1200), /showModal|confirm\(/,
  "no confirmation dialog for the switch");
assert.doesNotMatch(runtimeSource, /invoke<[^>]*>\("(?!get_output_ownership_status|standby_sync_status|agent_authority_status_v1)/,
  "automatic ownership only reads; it adds no command");
assert.match(policySource, /export const AUTO_OUTPUT_OWNERSHIP_STORAGE_KEY = "syndocal\.output\.autoTake\.v1";/);
assert.equal(policy.readAutoOutputSetting(() => ({ getItem: () => null })), true, "default on");
assert.equal(policy.readAutoOutputSetting(() => ({ getItem: () => "off" })), false);
assert.equal(policy.readAutoOutputSetting(() => { throw new Error("blocked"); }), true, "unreadable storage keeps the default");
{
  const store = new Map();
  const storage = () => ({ getItem: (key) => store.get(key) ?? null, setItem: (key, value) => store.set(key, value) });
  assert.equal(policy.writeAutoOutputSetting(storage, false), true);
  assert.equal(policy.readAutoOutputSetting(storage), false);
  assert.equal(policy.writeAutoOutputSetting(storage, true), true);
  assert.equal(policy.readAutoOutputSetting(storage), true);
}

console.log(`check-output-auto-ownership: PASS (${refusals.length} refusal cases, ${reasons.length} reasons)`);
