import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const hash = (character) => character.repeat(64);
const lease = (generation = 1) => ({ lease_id: "lease-0000000000000001", generation });
const fence = {
  process_incarnation: 1,
  session_incarnation: 2,
  project_epoch: 3,
  project_revision: 4,
  project_checkpoint_hash: hash("c"),
  project_publication_generation: 5,
  output_epoch: 6,
  output_generation: 7,
  safety_blackout_epoch: 8,
  safety_blackout_generation: 9,
};

const controllerSource = await read("src/outputControlController.ts");
const runtime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  controllerSource,
  {
    compilerOptions: {
      module: ts.ModuleKind.ESNext,
      target: ts.ScriptTarget.ES2022,
      importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
    },
    fileName: "outputControlController.ts",
  },
).outputText).toString("base64")}`);

const resourcesFor = (action) => action.kind === "enable_output" || action.role === "both"
  || action.kind === "add_display"
  ? ["lighting", "video"] : action.role === "lighting" ? ["lighting"] : ["video"];
const operationFor = (action) => ({
  enable_output: runtime.OUTPUT_ENABLE_OPERATION_ID,
  arm: runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
  release_blackout: runtime.OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
  take_over_standby: runtime.OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
  add_display: runtime.OUTPUT_DISPLAY_ADD_OPERATION_ID,
  acquire_lease: runtime.OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
  renew_lease: runtime.OUTPUT_LEASE_RENEW_OPERATION_ID,
  recover_lease: runtime.OUTPUT_LEASE_RECOVER_OPERATION_ID,
  relinquish_output_lease: runtime.OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
  force_transfer_lease: runtime.OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
})[action.kind];
const commandFor = (action) => ({
  enable_output: "enable_output_control_v2",
  arm: "arm_output_control_v2",
  release_blackout: "release_blackout_output_control_v2",
  take_over_standby: "take_over_output_control_v2",
  add_display: "add_display_output_v2",
  acquire_lease: "acquire_output_lease_v2",
  renew_lease: "renew_output_lease_v2",
  recover_lease: "recover_output_lease_v2",
  relinquish_output_lease: "relinquish_output_lease_v2",
  force_transfer_lease: "force_transfer_output_lease_v2",
})[action.kind];

const enableAction = { kind: "enable_output" };
const ordinaryActions = [
  { kind: "arm", role: "lighting", lease: lease() },
  { kind: "release_blackout", lease: lease() },
  {
    kind: "take_over_standby",
    force: true,
    standby_session_id: "standby-session-1",
    standby_generation: 7,
    lease: lease(),
  },
  {
    kind: "add_display",
    spec: {
      label: "LED panel",
      monitor_identity: hash("d"),
      monitor_index: 1,
      width: 1920,
      height: 1080,
      fullscreen: true,
    },
    lease: lease(),
  },
];
const lifecycleActions = [
  { kind: "acquire_lease", role: "both" },
  { kind: "renew_lease", lease: lease() },
  { kind: "recover_lease", lease: lease() },
  { kind: "relinquish_output_lease", lease: lease() },
  { kind: "force_transfer_lease", lease: lease() },
];

const queryFor = (action, state = "active") => {
  if (action.kind === "acquire_lease" || action.kind === "enable_output") {
    return {
      operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
      statuses: [{ status: "unavailable" }],
    };
  }
  return {
    operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
    statuses: [{
      status: state === "orphaned" ? "held_orphaned" : "held_active",
      authority: action.lease,
      resources: action.kind === "arm" && state !== "wrong"
        ? resourcesFor(action)
        : action.kind === "add_display" ? ["lighting", "video"] : ["lighting", "video"],
    }],
  };
};

const receiptFor = (request, action, enableRecovery = false, enableRecoveryGenerationDelta = 1) => {
  const recoveringEnable = action.kind === "enable_output" && enableRecovery;
  const resources = action.kind === "acquire_lease" || action.kind === "enable_output"
    ? resourcesFor(action)
    : action.kind === "arm" ? resourcesFor(action)
      : action.kind === "add_display" ? ["lighting", "video"] : ["lighting", "video"];
  const inputGeneration = action.kind === "acquire_lease" || action.kind === "enable_output" && !enableRecovery
    ? null : recoveringEnable ? 2 : action.lease.generation;
  const terminalGeneration = inputGeneration === null ? 1
    : recoveringEnable ? inputGeneration + enableRecoveryGenerationDelta
      : action.kind === "relinquish_output_lease" || action.kind === "renew_lease"
        || action.kind === "recover_lease" || action.kind === "force_transfer_lease"
        ? action.lease.generation + 1 : action.lease.generation;
  const relinquished = action.kind === "relinquish_output_lease";
  const outcome = ({
    arm: "authorized", release_blackout: "authorized", take_over_standby: "authorized",
    add_display: "authorized", acquire_lease: "acquired", enable_output: recoveringEnable ? "recovered" : "acquired",
    renew_lease: "renewed", recover_lease: "recovered", relinquish_output_lease: "relinquished",
    force_transfer_lease: "transferred",
  })[action.kind];
  return {
    type: "receipt",
    receipt: {
      operation_id: request.operation_id,
      request_id: request.request_id,
      shape_sha256: hash("a"),
      argument_fingerprint: hash("b"),
      audit_sequence: 1,
      fence_before: structuredClone(request.expected_fence),
      fence_after: structuredClone(request.expected_fence),
      outcome: "no_op",
      lease_result: {
        authority: action.kind === "acquire_lease" || action.kind === "enable_output" && !enableRecovery
          ? lease(1) : lease(terminalGeneration),
        resources,
        phase: relinquished ? "unclaimed" : "held_active",
        outcome,
        audit_sequence: 1,
        changes: [{
          lease_id: "lease-0000000000000001",
          before_generation: inputGeneration,
          after_generation: terminalGeneration,
          before_resources: inputGeneration === null ? [] : resources,
          after_resources: relinquished ? [] : resources,
          before_phase: inputGeneration === null ? null : recoveringEnable ? "held_orphaned" : "held_active",
          after_phase: relinquished ? "unclaimed" : "held_active",
        }],
      },
    },
  };
};

const createHarness = ({ action, queryState, authorityFence = fence, loseFirstReply = false, typedRejection = false, enableRecovery = false, enableRecoveryGenerationDelta = 1 } = {}) => {
  const operationId = operationFor(action);
  const command = commandFor(action);
  const calls = [];
  const executeArgs = [];
  let executeCalls = 0;
  const invoke = async (actualCommand, args) => {
    calls.push({ command: actualCommand, args });
    if (actualCommand === "query_output_control_authority_v1") {
      assert.equal(args, undefined);
      return { operation_id: runtime.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID, fence: structuredClone(authorityFence) };
    }
    if (actualCommand === "query_output_lease_authority_v1") {
      assert.equal(args, undefined);
      return queryFor(action, queryState);
    }
    assert.equal(actualCommand, command);
    assert.deepEqual(Object.keys(args), ["request"]);
    assert.deepEqual(Object.keys(args.request).sort(), ["action", "expected_fence", "operation_id", "request_id"]);
    assert.equal(Object.hasOwn(args.request, "consent_token"), false);
    assert.deepEqual(args.request.action, action);
    assert.equal(args.request.operation_id, operationId);
    executeArgs.push(args);
    if (loseFirstReply && executeCalls === 0) {
      executeCalls += 1;
      throw new Error("synthetic lost transport reply");
    }
    executeCalls += 1;
    if (typedRejection) return {
      type: "rejected",
      rejection: { operation_id: operationId, request_id: args.request.request_id, error: "forbidden" },
    };
    return receiptFor(args.request, action, enableRecovery, enableRecoveryGenerationDelta);
  };
  return { invoke, calls, executeArgs, get executeCalls() { return executeCalls; } };
};

for (const action of [enableAction, ...ordinaryActions, ...lifecycleActions]) {
  const harness = createHarness({ action, queryState: action.kind === "recover_lease" ? "orphaned" : "active" });
  const receipt = action.kind === "acquire_lease"
    ? await runtime.executeOutputLeaseLifecycle(harness.invoke, action)
    : action.kind === "enable_output" || action.kind === "arm" || action.kind === "release_blackout"
      || action.kind === "take_over_standby" || action.kind === "add_display"
      ? await runtime.executeOutputControl(harness.invoke, action)
      : await runtime.executeOutputLeaseLifecycle(harness.invoke, action);
  assert.equal(receipt.operation_id, operationFor(action));
  assert.equal(harness.executeCalls, 1);
}

const enableHarness = createHarness({ action: enableAction });
const enableReceipt = await runtime.enableOutput(enableHarness.invoke);
assert.equal(enableReceipt.operation_id, runtime.OUTPUT_ENABLE_OPERATION_ID);
assert.equal(enableReceipt.lease_result.outcome, "acquired");
assert.deepEqual(enableReceipt.lease_result.resources, ["lighting", "video"]);
assert.equal(enableHarness.executeCalls, 1);
assert.equal(Object.hasOwn(enableHarness.executeArgs[0].request.action, "lease"), false,
  "normal Enable Output must not require a preselected lease");

const enableRecoveryHarness = createHarness({ action: enableAction, enableRecovery: true });
const enableRecoveryReceipt = await runtime.enableOutput(enableRecoveryHarness.invoke);
assert.equal(enableRecoveryReceipt.lease_result.outcome, "recovered");
assert.equal(enableRecoveryReceipt.lease_result.changes[0].before_phase, "held_orphaned");
assert.equal(enableRecoveryReceipt.lease_result.changes[0].after_phase, "held_active");
assert.equal(enableRecoveryHarness.executeCalls, 1);
for (const enableRecoveryGenerationDelta of [0, -1]) {
  const forgedRecoveryHarness = createHarness({
    action: enableAction,
    enableRecovery: true,
    enableRecoveryGenerationDelta,
  });
  await assert.rejects(
    runtime.enableOutput(forgedRecoveryHarness.invoke),
    /lease result was invalid/,
    "Enable recovery must reject equal or decreasing receipt generations",
  );
  assert.equal(forgedRecoveryHarness.executeCalls, 1);
}

const leaseQuery = (statuses) => ({
  operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID,
  statuses,
});
assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }]), ["lighting", "video"]),
  true,
  "normal Enable remains satisfied only while exactly one active Both lease exists",
);
assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([]), ["lighting", "video"]),
  false,
  "expired or missing authority must re-enable the normal one-click Enable path",
);
assert.equal(
  runtime.hasOnlyActiveOutputLease(leaseQuery([
    { status: "held_active", authority: lease(1), resources: ["lighting", "video"] },
    { status: "held_active", authority: lease(2), resources: ["lighting", "video"] },
  ]), ["lighting", "video"]),
  false,
  "ambiguous active Both leases must not look like a usable normal authority",
);
assert.deepEqual(
  runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting", "video"],
  }]), ["lighting", "video"]),
  lease(),
  "Display add accepts the exact Both lease created by normal Enable",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["video"],
  }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Lighting-only/Video-only lease candidates must not authorize Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{
    status: "held_active",
    authority: lease(),
    resources: ["lighting"],
  }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Lighting-only lease candidates must not authorize Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([{ status: "unavailable" }]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "No lease must fail closed before Display add",
);
assert.throws(
  () => runtime.selectOnlyActiveOutputLease(leaseQuery([
    { status: "held_active", authority: lease(1), resources: ["lighting", "video"] },
    { status: "held_active", authority: lease(2), resources: ["lighting", "video"] },
  ]), ["lighting", "video"]),
  /Exactly one active output lease/,
  "Multiple Both leases must fail closed before Display add",
);

for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  const zeroHarness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, [field]: 0 } });
  const zeroReceipt = await runtime.executeOutputControl(zeroHarness.invoke, ordinaryActions[0]);
  assert.equal(zeroReceipt.operation_id, runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID);
  assert.equal(zeroHarness.executeCalls, 1);
}
for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  for (const invalid of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const harness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, [field]: invalid } });
    await assert.rejects(runtime.executeOutputControl(harness.invoke, ordinaryActions[0]), /authority response was invalid/);
    assert.equal(harness.executeCalls, 0);
  }
}
const unknownFenceHarness = createHarness({ action: ordinaryActions[0], authorityFence: { ...fence, unexpected: 1 } });
await assert.rejects(runtime.executeOutputControl(unknownFenceHarness.invoke, ordinaryActions[0]), /authority response was invalid/);
assert.equal(unknownFenceHarness.executeCalls, 0);

const malformedHarness = createHarness({ action: ordinaryActions[0] });
malformedHarness.invoke = async (command, args) => command === "query_output_lease_authority_v1"
  ? { operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses: [{ status: "unavailable" }, { status: "held_active", authority: lease(), resources: ["lighting"] }] }
  : createHarness({ action: ordinaryActions[0] }).invoke(command, args);
await assert.rejects(runtime.executeOutputControl(malformedHarness.invoke, ordinaryActions[0]), /query was invalid/);
assert.equal(malformedHarness.calls.filter((call) => call.command === "prepare_output_consent_v1").length, 0);

for (const queryState of ["orphaned", "wrong"]) {
  const action = { kind: "arm", role: "video", lease: lease() };
  const harness = createHarness({ action, queryState });
  await assert.rejects(runtime.executeOutputControl(harness.invoke, action), /wrong resources|orphaned/);
  assert.equal(harness.executeCalls, 0);
}

const replyLossAction = ordinaryActions[2];
const replyLossHarness = createHarness({ action: replyLossAction, loseFirstReply: true });
await runtime.executeOutputControl(replyLossHarness.invoke, replyLossAction);
assert.equal(replyLossHarness.executeCalls, 2);
assert.equal(replyLossHarness.executeArgs[0], replyLossHarness.executeArgs[1]);
assert.equal(Object.isFrozen(replyLossHarness.executeArgs[0]), true);

const rejectionHarness = createHarness({ action: ordinaryActions[0], typedRejection: true });
await assert.rejects(runtime.executeOutputControl(rejectionHarness.invoke, ordinaryActions[0]), /refresh lease state/);
assert.equal(rejectionHarness.executeCalls, 1);

const exhaustedSource = controllerSource.replace(
  "let nextOutputControlRequestId = 1;",
  "let nextOutputControlRequestId = Number.MAX_SAFE_INTEGER + 1;",
);
const exhaustedRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  exhaustedSource,
  { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove }, fileName: "outputControlController.ts" },
).outputText).toString("base64")}`);
let exhaustedInvokes = 0;
await assert.rejects(exhaustedRuntime.executeOutputControl(async () => { exhaustedInvokes += 1; }, ordinaryActions[0]), /request identity is exhausted/);
assert.equal(exhaustedInvokes, 0);

const [invokeSource, manifestSource, appSource, standbySource, mainSource, runtimeSource, querySource, controlPlaneSource, registrySource] = await Promise.all([
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/App.tsx"),
  read("src/components/StandbySyncPanel.tsx"),
  read("src-tauri/src/main.rs"),
  read("src-tauri/src/control_plane_runtime.rs"),
  read("src-tauri/src/control_plane_query.rs"),
  read("src-tauri/src/control_plane.rs"),
  read("../crates/protocol/src/control_plane_registry_v2.rs"),
]);
const commandBody = (source, commandName) => {
  const functionMarker = `fn ${commandName}(`;
  const functionStart = source.indexOf(functionMarker);
  assert.notEqual(functionStart, -1, `${commandName} must exist`);
  const attributeStart = source.lastIndexOf("#[tauri::command]", functionStart);
  assert.notEqual(attributeStart, -1, `${commandName} must remain a Tauri command`);
  const nextAttribute = source.indexOf("\n#[tauri::command]", functionStart + functionMarker.length);
  return source.slice(attributeStart, nextAttribute === -1 ? source.length : nextAttribute);
};
const requiredCommands = [
  "enable_output_control_v2", "add_display_output_v2", "arm_output_control_v2",
  "release_blackout_output_control_v2", "take_over_output_control_v2",
  "acquire_output_lease_v2", "force_transfer_output_lease_v2", "query_output_lease_authority_v1",
  "recover_output_lease_v2", "relinquish_output_lease_v2", "renew_output_lease_v2",
];
const legacyCommands = [
  "enable_output_control_v1", "add_display_output_v1", "arm_output_control_v1",
  "release_blackout_output_control_v1", "take_over_output_control_v1", "acquire_output_lease_v1",
  "force_transfer_output_lease_v1", "recover_output_lease_v1", "relinquish_output_lease_v1",
  "renew_output_lease_v1", "prepare_output_consent_v1", "query_output_consent_status_v1",
];
const canonicalOutputMutationWrappers = [
  "acquire_output_lease_v2",
  "add_display_output_v2",
  "arm_output_control_v2",
  "enable_output_control_v2",
  "force_transfer_output_lease_v2",
  "recover_output_lease_v2",
  "release_blackout_output_control_v2",
  "relinquish_output_lease_v2",
  "renew_output_lease_v2",
  "take_over_output_control_v2",
];
const outputControlWrappers = new Set([
  "add_display_output_v2",
  "arm_output_control_v2",
  "enable_output_control_v2",
  "release_blackout_output_control_v2",
  "take_over_output_control_v2",
]);
const manifest = JSON.parse(manifestSource);
const tuple = [...invokeSource.matchAll(/^\s+"([^"]+)",$/gm)].map((match) => match[1]);
assert.deepEqual(tuple, [...tuple].sort(), "frontend invoke tuple must remain bytewise sorted");
assert.deepEqual(manifest, [...manifest].sort(), "Tauri invoke manifest must remain bytewise sorted");
for (const command of requiredCommands) {
  assert.ok(tuple.includes(command), `${command} missing from invoke tuple`);
  assert.ok(manifest.includes(command), `${command} missing from invoke manifest`);
  assert.match(mainSource, new RegExp(`\\b${command}\\b`));
}
for (const command of legacyCommands) {
  assert.doesNotMatch(tuple.join("\n"), new RegExp(`\\b${command}\\b`));
  assert.doesNotMatch(manifest.join("\n"), new RegExp(`\\b${command}\\b`));
  assert.doesNotMatch(mainSource, new RegExp(`\\b${command}\\b`));
}
const detectedAsyncOutputMutationWrappers = [
  ...mainSource.matchAll(/#\[tauri::command\]\r?\nasync fn ([a-z0-9_]+_v2)\(/g),
].map((match) => match[1])
  .filter((command) => canonicalOutputMutationWrappers.includes(command))
  .sort();
assert.deepEqual(
  detectedAsyncOutputMutationWrappers,
  canonicalOutputMutationWrappers,
  "every canonical output/lease mutation wrapper must be async and the exact set must remain audited",
);
for (const command of canonicalOutputMutationWrappers) {
  const start = mainSource.indexOf(`async fn ${command}(`);
  assert.notEqual(start, -1, `${command} must remain async`);
  const nextCommand = mainSource.indexOf("\n#[tauri::command]", start + 1);
  const body = mainSource.slice(start, nextCommand === -1 ? undefined : nextCommand);
  const requiredHelper = outputControlWrappers.has(command)
    ? "execute_output_control_off_event_loop"
    : "execute_output_lease_lifecycle_off_event_loop";
  assert.match(
    body,
    new RegExp(`\\b${requiredHelper}\\(`),
    `${command} must dispatch through ${requiredHelper}`,
  );
  assert.doesNotMatch(
    body,
    /execute_output_(?:control|lease_lifecycle)_for_operation\(/,
    `${command} must not call the blocking canonical dispatcher inline`,
  );
}
const queryWrapper = commandBody(querySource, "query_output_lease_authority_v1");
assert.match(
  queryWrapper,
  /pub\(crate\) async fn query_output_lease_authority_v1\(/,
  "lease authority query must remain an async query wrapper",
);
assert.match(
  queryWrapper,
  /spawn_blocking[\s\S]*state::<ControlPlaneQueryState>/,
  "lease authority query must run off the event loop",
);
const outputAuthorityQueryBody = commandBody(mainSource, "query_output_control_authority_v1");
assert.match(outputAuthorityQueryBody, /async fn query_output_control_authority_v1\(/);
assert.match(
  outputAuthorityQueryBody,
  /spawn_blocking[\s\S]*issue_output_control_authority_for_window_label/,
  "output authority query must run off the event loop",
);
const standbyStatusBody = commandBody(mainSource, "standby_sync_status");
assert.match(standbyStatusBody, /async fn standby_sync_status\(/);
assert.match(standbyStatusBody, /spawn_blocking/, "standby disclosure status must run off the event loop");
const ownershipStatusBody = commandBody(mainSource, "get_output_ownership_status");
assert.match(ownershipStatusBody, /async fn get_output_ownership_status\(/);
assert.match(ownershipStatusBody, /spawn_blocking/, "output ownership disclosure status must run off the event loop");
for (const queryName of [
  "get_engine_telemetry_report",
  "remote_access_urls",
  "list_show_lan_interfaces",
  "remote_control_status",
  "dmx_input_status",
  "get_snapshot_delta",
]) {
  const body = commandBody(mainSource, queryName);
  assert.match(body, new RegExp(`async fn ${queryName}\\(`),
    `${queryName} must be an async Tauri query`);
  assert.match(body, /spawn_blocking/, `${queryName} must run off the event loop`);
}
for (const queryName of [
  "query_control_plane_project_authority",
  "query_control_plane_runtime_generations",
  "query_control_plane_output_ownership",
  "poll_control_plane_observation_events",
]) {
  const body = commandBody(querySource, queryName);
  assert.match(
    body,
    new RegExp(`pub\\(crate\\) async fn ${queryName}\\(`),
    `${queryName} must remain an async query wrapper`,
  );
  assert.match(
    body,
    /spawn_blocking/,
    `${queryName} must run off the event loop`,
  );
}
const projectAuthorityPollBody = commandBody(mainSource, "poll_project_authority_bundle");
assert.match(projectAuthorityPollBody, /async fn poll_project_authority_bundle\(/);
assert.match(
  projectAuthorityPollBody,
  /spawn_blocking[\s\S]*poll_project_authority_bundle_seqlock/,
  "project authority poll must run off the event loop",
);
const displayEnumerationBody = commandBody(mainSource, "list_video_display_monitors");
assert.match(displayEnumerationBody, /async fn list_video_display_monitors\(/);
assert.match(
  displayEnumerationBody,
  /spawn_blocking[\s\S]*available_monitors/,
  "display enumeration must not block the event loop",
);
assert.match(
  querySource,
  /fn capture_source_once\([\s\S]*project_coordinator\.try_lock\(\)/,
  "read-only authority capture must fail fast on coordinator contention",
);
assert.match(
  mainSource,
  /fn apply_native_video_output_window_shell_with_monitor\([\s\S]*missing its authoritative monitor descriptor/,
  "native AddDisplay shell must consume a captured descriptor",
);
assert.match(
  mainSource,
  /start_native_video_live_output\([\s\S]*Some\(\(output\.width, output\.height\)\)/,
  "candidate output worker must not call inner_size before publication",
);
assert.match(
  mainSource,
  /native_output_qa_driver_uses_canonical_v2_add_path_for_four_sub_displays/,
  "the backend QA driver must exercise canonical v2 AddDisplay requests",
);
assert.match(
  mainSource,
  /standby_disclosure_queries_are_bounded_after_failed_add_and_expired_lease/,
  "the failed-Add disclosure contention regression must remain covered",
);
assert.doesNotMatch(
  mainSource,
  /#\[tauri::command\]\s*(?:async\s+)?fn\s+native_output_qa_driver/,
  "the backend QA driver must not be a production Tauri command",
);
const postAdmissionRuntime = runtimeSource.slice(
  runtimeSource.indexOf("let external_admission = match"),
  runtimeSource.indexOf("let operation_result = match"),
);
assert.doesNotMatch(
  postAdmissionRuntime,
  /validate_display_output_monitor\(window/,
  "post-dialog project/coordinator admission must not call Webview monitor RPCs",
);
const addDisplayBackend = mainSource.slice(
  mainSource.indexOf("fn add_display_output_with_output_control_fence("),
  mainSource.indexOf("fn apply_native_video_output_window_shell(", mainSource.indexOf("fn add_display_output_with_output_control_fence(")),
);
const addDisplayCallback = addDisplayBackend.slice(
  addDisplayBackend.indexOf("let (applied, lease_receipt) ="),
);
assert.doesNotMatch(
  addDisplayCallback,
  /validate_editor_monitor_for_window\(editor_window\)/,
  "AddDisplay callback must not re-enter Webview monitor RPCs under project locks",
);
assert.doesNotMatch(controllerSource, /consent_token|prepare_output_consent|query_output_consent|physical-confirmation|Raw Input|physical Enter/);
for (const operationId of [
  "syndocal.output.enable.v2",
  "syndocal.output.ownership.arm.v2",
  "syndocal.output.blackout.release.v2",
  "syndocal.output.standby.takeover.v2",
  "syndocal.output.display.add.v2",
  "syndocal.output.lease.acquire.v2",
  "syndocal.output.lease.renew.v2",
  "syndocal.output.lease.recover.v2",
  "syndocal.output.lease.relinquish.v2",
  "syndocal.output.lease.force_transfer.v2",
]) {
  assert.match(controllerSource, new RegExp(operationId.replaceAll(".", "\\.")));
}
assert.doesNotMatch(
  controllerSource,
  /syndocal\.output\.(?:enable|ownership\.arm|blackout\.release|standby\.takeover|display\.add|lease\.(?:acquire|renew|recover|relinquish|force_transfer))\.v1/,
  "frontend mutating OutputControl operation IDs must not use the retired v1 boundary",
);
assert.doesNotMatch(standbySource, /challengeNotice|physical Enter|Raw Input|physical confirmation/);
assert.doesNotMatch(appSource, /displayCode|physical keyboard|prepare_output_consent|query_output_consent/);
assert.doesNotMatch(runtimeSource, /prepare_output_consent|query_output_consent|control_plane_security|consent_token/);
assert.doesNotMatch(controlPlaneSource, /prepare_output_consent|query_output_consent|PreparedPhysicalConfirmation/);
assert.doesNotMatch(registrySource, /PreparedPhysicalConfirmation|prepare_output_consent|query_output_consent/);
const nativeDangerConfirmation = runtimeSource.slice(
  runtimeSource.indexOf("fn output_action_requires_native_danger_confirmation("),
  runtimeSource.indexOf("pub(crate) fn issue_output_control_authority_for_window_label("),
);
assert.match(
  nativeDangerConfirmation,
  /MessageDialog::new\(\)[\s\S]*MessageButtons::YesNo[\s\S]*set_parent\(window\)[\s\S]*MessageDialogResult::Yes/,
  "advanced output mutations require a parented native Yes-only dialog",
);
for (const action of ["ReleaseBlackout", "Arm", "TakeOverStandby", "AddDisplay", "ForceTransferLease"]) {
  assert.match(nativeDangerConfirmation, new RegExp(`OutputControlActionV2::${action}`));
}
const outputExecution = runtimeSource.slice(
  runtimeSource.indexOf("fn execute_output_control_with_confirmation<F>("),
  runtimeSource.indexOf("pub(crate) fn exact_output_control_fence_matches("),
);
assert(
  outputExecution.indexOf("recheck_output_control_terminal")
    < outputExecution.indexOf("output_confirmation_gate"),
  "terminal replay must precede native confirmation",
);
const normalEnableDispatch = outputExecution.slice(
  outputExecution.indexOf("OutputControlActionV2::EnableOutput =>"),
  outputExecution.indexOf("OutputControlActionV2::ReleaseBlackout"),
);
assert.doesNotMatch(
  normalEnableDispatch,
  /MessageDialog|confirm_native_dangerous_output_action/,
  "normal one-click Enable must never call the native danger dialog",
);
assert.match(
  outputExecution,
  /output_confirmation_gate\([\s\S]*?store_output_control_terminal\([\s\S]*?Forbidden/,
  "native dialog cancel/close must become a terminal rejection before admission and mutation",
);
assert.match(standbySource, /data-io-control="enable-output"/);
assert.match(standbySource, /role="status" aria-live="polite">Output enabled/);
assert.match(standbySource, /hasOnlyActiveOutputLease\(leaseQuery\(\), \["lighting", "video"\]\)/);
assert.match(standbySource, /function boundedPromise<[\s\S]*STATUS_POLL_INVOKE_TIMEOUT_MS/);
assert.match(standbySource, /let pollFlight: Promise<void> \| null = null/);
assert.match(standbySource, /if \(pollFlight\) return pollWaiter \?\? pollFlight/);
assert.match(standbySource, /generation === pollGeneration/);
assert.match(standbySource, /Promise\.allSettled\(rawEndpointFlights\)/);
assert.match(standbySource, /Promise\.allSettled\(boundedEndpointFlights\)/);
assert.match(standbySource, /if \(!pollFlight && !enableMutationFlight\) void pollStatus\(\)/);
assert.match(standbySource, /let enableMutationFlight: ReturnType<typeof enableOutput> \| null = null/);
assert.match(standbySource, /let enableOutcomeUnknown = false/);
assert.match(standbySource, /beginEnableMutation[\s\S]*OUTPUT_ENABLE_TIMEOUT_MS[\s\S]*setBusy\(false\)[\s\S]*void pollStatus\(\)/);
assert.match(standbySource, /final outcome is unknown/);
assert.match(standbySource, /enableGeneration/);
assert.match(standbySource, /if \(enableOutcomeUnknown && authoritativeBothReady\)[\s\S]*setActionError\(null\)/);
assert.match(standbySource, /if \(!enableOutcomeUnknown\) setActionError\(String\(error\)\)/);
assert.doesNotMatch(
  standbySource,
  /setActionError\(String\(error\)\);\r?\n\s+await pollStatus\(\)/,
  "action catch paths must not start an unbounded second refresh",
);
assert.match(standbySource, /<details class="advancedOutputControls">/);
assert.doesNotMatch(standbySource, /<details class="advancedOutputControls"[^>]*\bopen\b/);
assert.match(standbySource, /I have fenced or disconnected every old Primary/);
assert.match(
  mainSource,
  /async fn enable_output_control_v2[\s\S]*execute_output_control_off_event_loop/,
  "normal Enable is dispatched through the off-event-loop output lane",
);
assert.match(
  mainSource,
  /async fn execute_output_control_off_event_loop[\s\S]*execute_output_control_for_operation/,
  "the off-event-loop lane reaches the canonical operation dispatcher",
);
assert.match(mainSource, /OutputControlActionV2::AddDisplay[\s\S]*vec!\[OutputLeaseResource::Lighting, OutputLeaseResource::Video\]/);
assert.match(appSource, /selectOnlyActiveOutputLease\(leaseQuery, \["lighting", "video"\]\)/);
assert.match(appSource, /fullscreen: target\.fullscreen/);
assert.match(appSource, /width: target\.width[\s\S]*height: target\.height/);
assert.match(runtimeSource, /OutputControlActionV2::EnableOutput[\s\S]*enable_output_with_output_control_fence/);
assert.match(controlPlaneSource, /enable_output_control_v2/);

// Deterministic frontend poll contract seam. This mirrors the component's
// raw-flight/UI-waiter split and keeps the timeout, overlap, and stale-result
// failures cheap to exercise in Node.
const sleep = (milliseconds) => new Promise((resolve) => setTimeout(resolve, milliseconds));
const deferred = () => {
  let resolve;
  let reject;
  const promise = new Promise((promiseResolve, promiseReject) => {
    resolve = promiseResolve;
    reject = promiseReject;
  });
  return { promise, resolve, reject };
};
const createPollContractHarness = (timeoutMs = 20, invokeTimeoutMs = 40) => {
  let rawFlight = null;
  let waiter = null;
  let rawSettled = Promise.resolve();
  let generation = 0;
  let authority = "ready";
  const invokeCounts = [0, 0, 0];
  const bounded = (factory) => new Promise((resolve, reject) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error("status refresh timed out"));
    }, timeoutMs);
    Promise.resolve().then(factory).then((value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(value);
    }, (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
      });
  });
  const poll = (factories) => {
    if (rawFlight) return waiter;
    const token = ++generation;
    const current = () => token === generation;
    const rawEndpoints = factories.map((factory, index) => {
      invokeCounts[index] += 1;
      return Promise.resolve().then(factory);
    });
    const rawCompletion = Promise.allSettled(rawEndpoints).then((results) => {
      const failure = results.find((result) => result.status === "rejected");
      if (failure?.status === "rejected") {
        if (current()) authority = "unavailable";
        return;
      }
      if (current()) authority = "ready";
    });
    let retained;
    retained = rawCompletion.finally(() => {
      if (rawFlight === retained) {
        rawFlight = null;
        waiter = null;
      }
    });
    rawFlight = retained;
    rawSettled = retained;
    const boundedEndpoints = rawEndpoints.map((endpoint) =>
      bounded(() => endpoint, invokeTimeoutMs));
    const uiCompletion = Promise.allSettled(boundedEndpoints).then((results) => {
      const failure = results.find((result) => result.status === "rejected");
      if (failure?.status === "rejected") throw failure.reason;
    });
    waiter = bounded(() => uiCompletion, timeoutMs).catch((error) => {
      if (current()) {
        generation += 1;
        authority = "unavailable";
      }
      return error;
    });
    return waiter;
  };
  return {
    poll,
    get authority() { return authority; },
    get invokeCounts() { return [...invokeCounts]; },
    waitForRaw: () => rawSettled,
  };
};

const hungRecoveredHarness = createPollContractHarness();
const hungStatus = deferred();
const hungWaiter = hungRecoveredHarness.poll([
  () => Promise.resolve("ready"),
  () => hungStatus.promise,
  () => Promise.resolve("ready"),
]);
await hungWaiter;
assert.equal(hungRecoveredHarness.authority, "unavailable",
  "a hung post-Recovered refresh must fail closed within the bound");
assert.deepEqual(hungRecoveredHarness.invokeCounts, [1, 1, 1]);
const hungOverlap = hungRecoveredHarness.poll([
  () => Promise.resolve("duplicate"),
  () => Promise.resolve("duplicate"),
  () => Promise.resolve("duplicate"),
]);
assert.strictEqual(hungOverlap, hungWaiter, "a timed-out UI waiter must remain attached to the raw flight");
assert.deepEqual(hungRecoveredHarness.invokeCounts, [1, 1, 1],
  "a pending raw refresh must block every duplicate endpoint invoke");
hungStatus.resolve("late-ready");
await hungRecoveredHarness.waitForRaw();
const overlapHarness = createPollContractHarness();
const overlapEndpoints = [deferred(), deferred(), deferred()];
const overlapPromise = overlapHarness.poll(overlapEndpoints.map((endpoint) => () => endpoint.promise));
const overlappingPoll = overlapHarness.poll([
  () => Promise.resolve("duplicate"),
  () => Promise.resolve("duplicate"),
  () => Promise.resolve("duplicate"),
]);
assert.strictEqual(overlapPromise, overlappingPoll, "interval refreshes must share one raw flight");
overlapEndpoints.forEach((endpoint) => endpoint.resolve("ready"));
await overlapPromise;
assert.deepEqual(overlapHarness.invokeCounts, [1, 1, 1], "overlapping interval must invoke one endpoint set");
const staleHarness = createPollContractHarness();
const staleEndpoints = [deferred(), deferred(), deferred()];
const stalePoll = staleHarness.poll(staleEndpoints.map((endpoint) => () => endpoint.promise));
await sleep(25);
assert.equal(staleHarness.authority, "unavailable");
staleEndpoints.forEach((endpoint) => endpoint.resolve("late-ready"));
await staleHarness.waitForRaw();
await staleHarness.poll([
  () => Promise.reject(new Error("new refresh failed")),
  () => Promise.resolve("new"),
  () => Promise.resolve("new"),
]);
assert.equal(staleHarness.authority, "unavailable",
  "an old slow success must not restore authority after a newer failure");

const createEnableMutationHarness = (timeoutMs = 20) => {
  let rawMutation = null;
  let mutationCount = 0;
  let authority = "ready";
  let actionError = null;
  const boundedEnable = (factory) => new Promise((resolve, reject) => {
    let settled = false;
    const timer = setTimeout(() => {
      if (settled) return;
      settled = true;
      reject(new Error("Output enable timed out"));
    }, timeoutMs);
    Promise.resolve().then(factory).then((value) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      resolve(value);
    }, (error) => {
      if (settled) return;
      settled = true;
      clearTimeout(timer);
      reject(error);
    });
  });
  const enable = (factory) => {
    if (rawMutation) return Promise.reject(new Error("Output enable is still in progress"));
    mutationCount += 1;
    const raw = Promise.resolve().then(factory);
    const retained = raw.then(
      () => { if (rawMutation === retained) rawMutation = null; },
      () => { if (rawMutation === retained) rawMutation = null; },
    );
    rawMutation = retained;
    return boundedEnable(() => retained).catch((error) => {
      authority = "unavailable";
      actionError = "Output enable timed out; final outcome is unknown.";
      return error;
    });
  };
  const authoritativePoll = (state) => {
    if (state === "ReadyBoth") {
      actionError = null;
    }
  };
  return {
    enable,
    authoritativePoll,
    get authority() { return authority; },
    get actionError() { return actionError; },
    get mutationCount() { return mutationCount; },
    get rawPending() { return Boolean(rawMutation); },
  };
};
const enableMutationHarness = createEnableMutationHarness();
const enableTerminal = deferred();
await enableMutationHarness.enable(() => enableTerminal.promise);
assert.equal(enableMutationHarness.authority, "unavailable", "Enable timeout must fail closed");
assert.match(enableMutationHarness.actionError, /outcome is unknown/);
enableMutationHarness.authoritativePoll("failed");
assert.match(enableMutationHarness.actionError, /outcome is unknown/,
  "a failed authoritative poll must not clear the Enable unknown-outcome error");
enableMutationHarness.authoritativePoll("unknown");
assert.match(enableMutationHarness.actionError, /outcome is unknown/,
  "an incomplete authoritative poll must not clear the Enable unknown-outcome error");
enableMutationHarness.authoritativePoll("ReadyBoth");
assert.equal(enableMutationHarness.actionError, null,
  "only an authoritative Ready+Both poll may clear the Enable unknown-outcome error");
await assert.rejects(
  enableMutationHarness.enable(() => Promise.resolve("duplicate")),
  /still in progress/,
  "a second click must not start a duplicate mutation while the raw request is pending",
);
assert.equal(enableMutationHarness.mutationCount, 1);
enableTerminal.resolve("late-terminal");
await sleep(0);
assert.equal(enableMutationHarness.rawPending, false);
assert.equal(enableMutationHarness.authority, "unavailable",
  "a late terminal response must not directly restore enabled UI state");

console.log("output control runtime contract: PASS (v2 output commands, atomic Both enable, no physical consent, strict receipts, fail-closed query)");
