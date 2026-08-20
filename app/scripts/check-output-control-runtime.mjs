import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");
const hash = (character) => character.repeat(64);
const challengeId = "0123456789ABCDEFGHIJKL";
const consentToken = "abcdefghijklmnopqrstuv";
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

const resourcesFor = (action) => action.role === "lighting"
  ? ["lighting"] : action.role === "video" ? ["video"] : ["lighting", "video"];
const operationFor = (action) => ({
  arm: runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
  release_blackout: runtime.OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
  take_over_standby: runtime.OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
  acquire_lease: runtime.OUTPUT_LEASE_ACQUIRE_OPERATION_ID,
  renew_lease: runtime.OUTPUT_LEASE_RENEW_OPERATION_ID,
  recover_lease: runtime.OUTPUT_LEASE_RECOVER_OPERATION_ID,
  relinquish_output_lease: runtime.OUTPUT_LEASE_RELINQUISH_OPERATION_ID,
  force_transfer_lease: runtime.OUTPUT_LEASE_FORCE_TRANSFER_OPERATION_ID,
})[action.kind];
const commandFor = (action) => ({
  arm: "arm_output_control_v1",
  release_blackout: "release_blackout_output_control_v1",
  take_over_standby: "take_over_output_control_v1",
  acquire_lease: "acquire_output_lease_v1",
  renew_lease: "renew_output_lease_v1",
  recover_lease: "recover_output_lease_v1",
  relinquish_output_lease: "relinquish_output_lease_v1",
  force_transfer_lease: "force_transfer_output_lease_v1",
})[action.kind];

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
];
const lifecycleActions = [
  { kind: "acquire_lease", role: "both" },
  { kind: "renew_lease", lease: lease() },
  { kind: "recover_lease", lease: lease() },
  { kind: "relinquish_output_lease", lease: lease() },
  { kind: "force_transfer_lease", lease: lease() },
];

const queryFor = (action, state = "active") => {
  if (action.kind === "acquire_lease") {
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
      resources: action.kind === "arm" && state !== "wrong" ? resourcesFor(action) : ["lighting", "video"],
    }],
  };
};

const receiptFor = (request, action) => {
  const resources = action.kind === "acquire_lease" ? resourcesFor(action)
    : action.kind === "arm" ? resourcesFor(action) : ["lighting", "video"];
  const inputGeneration = action.kind === "acquire_lease" ? null : action.lease.generation;
  const terminalGeneration = action.kind === "acquire_lease" ? 1
    : action.kind === "relinquish_output_lease" || action.kind === "renew_lease"
      || action.kind === "recover_lease" || action.kind === "force_transfer_lease"
      ? action.lease.generation + 1 : action.lease.generation;
  const relinquished = action.kind === "relinquish_output_lease";
  const outcome = ({
    arm: "authorized",
    release_blackout: "authorized",
    take_over_standby: "authorized",
    acquire_lease: "acquired",
    renew_lease: "renewed",
    recover_lease: "recovered",
    relinquish_output_lease: "relinquished",
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
        authority: action.kind === "acquire_lease" ? lease(1) : lease(terminalGeneration),
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
          before_phase: inputGeneration === null ? null : "held_active",
          after_phase: relinquished ? "unclaimed" : "held_active",
        }],
      },
    },
  };
};

const createHarness = ({
  action,
  queryState,
  authorityFence = fence,
  loseFirstReply = false,
  typedRejection = false,
  statusState = "ready",
  statusExpiryOffsetMs = 0,
} = {}) => {
  const operationId = operationFor(action);
  const command = commandFor(action);
  const calls = [];
  const executeArgs = [];
  let statusCalls = 0;
  let executeCalls = 0;
  let preparedRequest;
  let challengeExpiresAt = Date.now() + 5_000;
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
    if (actualCommand === "prepare_output_consent_v1") {
      assert.deepEqual(Object.keys(args), ["request"]);
      assert.equal(args.request.operation_id, runtime.OUTPUT_CONSENT_PREPARE_OPERATION_ID);
      assert.equal(args.request.action, action);
      preparedRequest = args.request;
      return {
        operation_id: runtime.OUTPUT_CONSENT_PREPARE_OPERATION_ID,
        request_id: args.request.request_id,
        target_operation_id: operationId,
        challenge_id: challengeId,
        consent_token: consentToken,
        display_code: "123456",
        argument_fingerprint: hash("b"),
        expires_at_unix_ms: challengeExpiresAt,
      };
    }
    if (actualCommand === "query_output_consent_status_v1") {
      statusCalls += 1;
      assert.equal(args.request.operation_id, runtime.OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID);
      assert.notEqual(args.request.request_id, preparedRequest.request_id);
      return {
        operation_id: runtime.OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID,
        request_id: args.request.request_id,
        challenge_id: challengeId,
        state: statusState,
        expires_at_unix_ms: challengeExpiresAt + statusExpiryOffsetMs,
      };
    }
    assert.equal(actualCommand, command);
    assert.equal(args.request.action, action);
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
    return receiptFor(args.request, action);
  };
  return { invoke, calls, executeArgs, get statusCalls() { return statusCalls; }, get executeCalls() { return executeCalls; } };
};

await assert.rejects(
  runtime.executeOutputControl(async () => { throw new Error("invoke must not be reached"); }, ordinaryActions[0]),
  /visible, nonblocking physical-confirmation challenge surface/,
);

for (const action of [...ordinaryActions, ...lifecycleActions]) {
  const harness = createHarness({ action, queryState: action.kind === "recover_lease" ? "orphaned" : "active" });
  const receipt = action.kind === "acquire_lease"
    ? await runtime.executeOutputLeaseLifecycle(harness.invoke, action, () => {})
    : action.kind === "arm" || action.kind === "release_blackout" || action.kind === "take_over_standby"
      ? await runtime.executeOutputControl(harness.invoke, action, () => {})
      : await runtime.executeOutputLeaseLifecycle(harness.invoke, action, () => {});
  assert.equal(receipt.operation_id, operationFor(action));
  assert.equal(harness.executeCalls, 1);
  assert.equal(harness.statusCalls, 1);
}

// The sixth physical digit may land immediately before the original expiry.
// A single bounded Ready extension is accepted, while pending drift and an
// over-cap extension fail before physical execution.
const boundaryReadyHarness = createHarness({
  action: lifecycleActions[0],
  statusExpiryOffsetMs: 5_000,
});
await runtime.executeOutputLeaseLifecycle(
  boundaryReadyHarness.invoke,
  lifecycleActions[0],
  () => {},
);
assert.equal(boundaryReadyHarness.executeCalls, 1);

for (const invalidStatus of [
  { statusState: "pending_physical_input", statusExpiryOffsetMs: 1 },
  { statusState: "ready", statusExpiryOffsetMs: -1 },
  { statusState: "ready", statusExpiryOffsetMs: 5_001 },
]) {
  const invalidStatusHarness = createHarness({ action: lifecycleActions[0], ...invalidStatus });
  await assert.rejects(
    runtime.executeOutputLeaseLifecycle(invalidStatusHarness.invoke, lifecycleActions[0], () => {}),
    /physical-confirmation status was invalid/,
  );
  assert.equal(invalidStatusHarness.executeCalls, 0);
}

// Rust permits zero only for the three project-scoped fence fields.  Keep the
// renderer contract aligned: process/session/output/safety incarnations stay
// strictly positive, while negative, fractional, and unsafe values fail
// closed before consent or physical execution.
for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  const zeroFence = { ...fence, [field]: 0 };
  const zeroHarness = createHarness({ action: ordinaryActions[0], authorityFence: zeroFence });
  const zeroReceipt = await runtime.executeOutputControl(zeroHarness.invoke, ordinaryActions[0], () => {});
  assert.equal(zeroReceipt.operation_id, runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID);
  assert.equal(zeroHarness.executeCalls, 1);
}

for (const field of ["project_epoch", "project_revision", "project_publication_generation"]) {
  for (const invalid of [-1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const invalidFence = { ...fence, [field]: invalid };
    const invalidHarness = createHarness({ action: ordinaryActions[0], authorityFence: invalidFence });
    await assert.rejects(
      runtime.executeOutputControl(invalidHarness.invoke, ordinaryActions[0], () => {}),
      /OutputControl authority response was invalid/,
    );
    assert.equal(invalidHarness.executeCalls, 0);
  }
}

for (const field of [
  "process_incarnation", "session_incarnation", "output_epoch", "output_generation",
  "safety_blackout_epoch", "safety_blackout_generation",
]) {
  for (const invalid of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
    const invalidFence = { ...fence, [field]: invalid };
    const invalidHarness = createHarness({ action: ordinaryActions[0], authorityFence: invalidFence });
    await assert.rejects(
      runtime.executeOutputControl(invalidHarness.invoke, ordinaryActions[0], () => {}),
      /OutputControl authority response was invalid/,
    );
    assert.equal(invalidHarness.executeCalls, 0);
  }
}

const unknownFence = { ...fence, unexpected: 1 };
const unknownFenceHarness = createHarness({ action: ordinaryActions[0], authorityFence: unknownFence });
await assert.rejects(
  runtime.executeOutputControl(unknownFenceHarness.invoke, ordinaryActions[0], () => {}),
  /OutputControl authority response was invalid/,
);
assert.equal(unknownFenceHarness.executeCalls, 0);

// Query is an exact operation DTO: malformed/unavailable/orphan/resource mismatch
// stops before consent and before any lifecycle fallback.
const malformedHarness = createHarness({ action: ordinaryActions[0] });
malformedHarness.invoke = async (command, args) => command === "query_output_lease_authority_v1"
  ? { operation_id: runtime.OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID, statuses: [{ status: "unavailable" }, { status: "held_active", authority: lease(), resources: ["lighting"] }] }
  : createHarness({ action: ordinaryActions[0] }).invoke(command, args);
await assert.rejects(runtime.executeOutputControl(malformedHarness.invoke, ordinaryActions[0], () => {}), /query was invalid/);
assert.equal(malformedHarness.calls.filter((call) => call.command === "prepare_output_consent_v1").length, 0);

for (const queryState of ["orphaned", "wrong"]) {
  const action = { kind: "arm", role: "video", lease: lease() };
  const harness = createHarness({ action, queryState });
  await assert.rejects(runtime.executeOutputControl(harness.invoke, action, () => {}), /wrong resources|orphaned/);
  assert.equal(harness.executeCalls, 0);
}

// Reply loss is the sole retryable transport condition, and it reuses the
// same frozen argument object. Typed rejection is terminal and never retries.
const replyLossAction = ordinaryActions[2];
const replyLossHarness = createHarness({ action: replyLossAction, loseFirstReply: true });
await runtime.executeOutputControl(replyLossHarness.invoke, replyLossAction, () => {});
assert.equal(replyLossHarness.executeCalls, 2);
assert.equal(replyLossHarness.executeArgs[0], replyLossHarness.executeArgs[1]);
assert.equal(Object.isFrozen(replyLossHarness.executeArgs[0]), true);

const rejectionHarness = createHarness({ action: ordinaryActions[0], typedRejection: true });
await assert.rejects(runtime.executeOutputControl(rejectionHarness.invoke, ordinaryActions[0], () => {}), /refresh lease state/);
assert.equal(rejectionHarness.executeCalls, 1);

// Request exhaustion fails before even the authority query and never invokes.
const exhaustedSource = controllerSource.replace(
  "let nextOutputControlRequestId = 1;",
  "let nextOutputControlRequestId = Number.MAX_SAFE_INTEGER + 1;",
);
const exhaustedRuntime = await import(`data:text/javascript;base64,${Buffer.from(ts.transpileModule(
  exhaustedSource,
  { compilerOptions: { module: ts.ModuleKind.ESNext, target: ts.ScriptTarget.ES2022, importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove }, fileName: "outputControlController.ts" },
).outputText).toString("base64")}`);
let exhaustedInvokes = 0;
await assert.rejects(exhaustedRuntime.executeOutputControl(async () => { exhaustedInvokes += 1; }, ordinaryActions[0], () => {}), /request identity is exhausted/);
assert.equal(exhaustedInvokes, 0);

// Strict receipt cross-field checks reject a mismatched authority and a
// multi-change result before the renderer can claim success.
const malformedReceiptBase = createHarness({ action: ordinaryActions[0] });
const malformedReceiptInvoke = async (command, args) => {
  const response = await malformedReceiptBase.invoke(command, args);
  if (command === "arm_output_control_v1") {
    response.receipt.lease_result.authority.lease_id = "lease-0000000000000002";
  }
  return response;
};
await assert.rejects(runtime.executeOutputControl(malformedReceiptInvoke, ordinaryActions[0], () => {}), /lease result was invalid/);

const [invokeSource, manifestSource, appSource, standbySource, mainSource] = await Promise.all([
  read("src/tauriInvokeCommands.ts"),
  read("src/tauri-invoke-manifest.json"),
  read("src/App.tsx"),
  read("src/components/StandbySyncPanel.tsx"),
  read("src-tauri/src/main.rs"),
]);
const requiredCommands = [
  "acquire_output_lease_v1", "force_transfer_output_lease_v1", "query_output_lease_authority_v1",
  "recover_output_lease_v1", "relinquish_output_lease_v1", "renew_output_lease_v1",
];
const manifest = JSON.parse(manifestSource);
const tuple = [...invokeSource.matchAll(/^\s+"([^"]+)",$/gm)].map((match) => match[1]);
assert.deepEqual(tuple, [...tuple].sort(), "frontend invoke tuple must remain bytewise sorted");
assert.deepEqual(manifest, [...manifest].sort(), "Tauri invoke manifest must remain bytewise sorted");
for (const command of requiredCommands) {
  assert.ok(tuple.includes(command), `${command} missing from invoke tuple`);
  assert.ok(manifest.includes(command), `${command} missing from invoke manifest`);
  assert.match(mainSource, new RegExp(`\\b${command}\\b`));
}
assert.match(appSource, /queryOutputLeaseAuthority[\s\S]*selectOnlyActiveOutputLease[\s\S]*kind: "release_blackout"/);
assert.match(standbySource, /executeOutputLeaseLifecycle/);
assert.doesNotMatch(standbySource, />Force transfer</, "foreign lease candidates are not exposed, so the UI must not offer an always-failing transfer button");
assert.match(standbySource, /<dialog[\s\S]*?<Show when=\{challengeNotice\(\)\}>[\s\S]*?Backend physical confirmation required/, "Take Over keeps the backend challenge inside the modal top layer");
assert.match(standbySource, /Backend physical confirmation required/);
assert.match(standbySource, /I have fenced or disconnected every old Primary/);
assert.match(controllerSource, /deepFreeze/);
assert.match(controllerSource, /invoke<unknown>\(command, executeArgs\)/);
assert.match(controllerSource, /OUTPUT_LEASE_AUTHORITY_QUERY_OPERATION_ID/);

console.log("output control runtime contract: PASS (8 operations, strict lease receipts, exact retry, fail-closed query)");
