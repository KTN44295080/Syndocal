import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const read = (relativePath) => readFile(new URL(`../${relativePath}`, import.meta.url), "utf8");

const [
  controllerSource,
  appSource,
  standbySource,
  videoRuntimeSource,
  mainSource,
  controlPlaneSource,
  registrySource,
] = await Promise.all([
  read("src/outputControlController.ts"),
  read("src/App.tsx"),
  read("src/components/StandbySyncPanel.tsx"),
  read("src/createVideoRuntimeController.ts"),
  read("src-tauri/src/main.rs"),
  read("src-tauri/src/control_plane.rs"),
  read("../crates/protocol/src/control_plane_registry_v2.rs"),
]);

const transpiled = ts.transpileModule(controllerSource, {
  compilerOptions: {
    module: ts.ModuleKind.ESNext,
    target: ts.ScriptTarget.ES2022,
    importsNotUsedAsValues: ts.ImportsNotUsedAsValues.Remove,
  },
  fileName: "outputControlController.ts",
});
const runtime = await import(
  `data:text/javascript;base64,${Buffer.from(transpiled.outputText).toString("base64")}`,
);

const hash = (character) => character.repeat(64);
const challengeId = "0123456789ABCDEFGHIJKL";
const consentToken = "abcdefghijklmnopqrstuv";
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

const actionCases = [
  {
    action: { kind: "arm", role: "lighting" },
    operationId: runtime.OUTPUT_OWNERSHIP_ARM_OPERATION_ID,
    command: "arm_output_control_v1",
  },
  {
    action: { kind: "release_blackout" },
    operationId: runtime.OUTPUT_BLACKOUT_RELEASE_OPERATION_ID,
    command: "release_blackout_output_control_v1",
  },
  {
    action: {
      kind: "take_over_standby",
      force: true,
      standby_session_id: "standby-session-1",
      standby_generation: 7,
    },
    operationId: runtime.OUTPUT_STANDBY_TAKEOVER_OPERATION_ID,
    command: "take_over_output_control_v1",
  },
];

const receiptFor = (request) => ({
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
  },
});

const createHarness = ({
  action,
  operationId,
  command,
  statusStates = ["ready"],
  expiresAtUnixMs = Date.now() + 5_000,
  executeResponse,
  loseFirstReply = false,
  authorityMutator,
  challengeMutator,
  statusMutator,
  receiptMutator,
} = {}) => {
  const calls = [];
  const statusArgs = [];
  const executeArgs = [];
  let statusIndex = 0;
  let executeIndex = 0;
  let preparedRequest;
  const authority = {
    operation_id: runtime.OUTPUT_CONTROL_AUTHORITY_QUERY_OPERATION_ID,
    fence,
  };
  const invoke = async (actualCommand, args) => {
    calls.push({ command: actualCommand, args });
    if (actualCommand === "query_output_control_authority_v1") {
      assert.equal(args, undefined, "authority query has no caller-controlled arguments");
      const response = structuredClone(authority);
      return authorityMutator ? authorityMutator(response) : response;
    }
    if (actualCommand === "prepare_output_consent_v1") {
      assert.deepEqual(Object.keys(args).sort(), ["request"]);
      assert.deepEqual(Object.keys(args.request).sort(), [
        "action",
        "expected_fence",
        "operation_id",
        "request_id",
      ]);
      assert.equal(args.request.operation_id, runtime.OUTPUT_CONSENT_PREPARE_OPERATION_ID);
      assert.deepEqual(args.request.expected_fence, authority.fence);
      assert.notEqual(
        args.request.expected_fence,
        authority.fence,
        "Tauri JSON fence correlation is value-based rather than reference-based",
      );
      assert.equal(args.request.action, action);
      preparedRequest = args.request;
      const response = {
        operation_id: runtime.OUTPUT_CONSENT_PREPARE_OPERATION_ID,
        request_id: args.request.request_id,
        target_operation_id: operationId,
        challenge_id: challengeId,
        consent_token: consentToken,
        display_code: "123456",
        argument_fingerprint: hash("b"),
        expires_at_unix_ms: expiresAtUnixMs,
      };
      return challengeMutator
        ? challengeMutator(structuredClone(response), args)
        : structuredClone(response);
    }
    if (actualCommand === "query_output_consent_status_v1") {
      assert.deepEqual(Object.keys(args).sort(), ["request"]);
      assert.deepEqual(Object.keys(args.request).sort(), [
        "challenge_id",
        "operation_id",
        "request_id",
      ]);
      assert.equal(args.request.operation_id, runtime.OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID);
      assert.equal(args.request.challenge_id, challengeId);
      statusArgs.push(args);
      const state = statusStates[Math.min(statusIndex, statusStates.length - 1)];
      statusIndex += 1;
      const response = {
        operation_id: runtime.OUTPUT_CONSENT_STATUS_QUERY_OPERATION_ID,
        request_id: args.request.request_id,
        challenge_id: challengeId,
        state,
        expires_at_unix_ms: expiresAtUnixMs,
      };
      return statusMutator
        ? statusMutator(structuredClone(response), args)
        : structuredClone(response);
    }
    assert.equal(actualCommand, command, "controller selected the action-specific ingress");
    assert.deepEqual(Object.keys(args).sort(), ["request"]);
    assert.deepEqual(Object.keys(args.request).sort(), [
      "action",
      "consent_token",
      "expected_fence",
      "operation_id",
      "request_id",
    ]);
    assert.equal(args.request.operation_id, operationId);
    assert.deepEqual(args.request.expected_fence, authority.fence);
    assert.equal(args.request.action, action);
    assert.equal(args.request.consent_token, consentToken);
    assert.equal(args.request.request_id, preparedRequest.request_id);
    executeArgs.push(args);
    if (loseFirstReply && executeIndex === 0) {
      executeIndex += 1;
      throw new Error("synthetic lost OutputControl IPC reply after apply");
    }
    executeIndex += 1;
    const response = typeof executeResponse === "function"
      ? executeResponse(args.request)
      : executeResponse ?? receiptFor(args.request);
    return receiptMutator
      ? receiptMutator(structuredClone(response), args)
      : structuredClone(response);
  };
  return { invoke, authority, calls, statusArgs, executeArgs };
};

// The browser must be given a visible, nonblocking challenge surface before
// the backend prepares a token. Omitting it is rejected without any invoke.
let omittedSurfaceCalls = 0;
await assert.rejects(
  runtime.executeOutputControl(
    async () => {
      omittedSurfaceCalls += 1;
      throw new Error("invoke must not be reached without a challenge surface");
    },
    actionCases[0].action,
  ),
  /visible, nonblocking physical-confirmation challenge surface/,
);
assert.equal(omittedSurfaceCalls, 0);

// Exercise all three canonical operations. The harness checks every request
// shape and reference, including the exact `{ request: ... }` Tauri envelope.
for (const testCase of actionCases) {
  const harness = createHarness(testCase);
  let notice;
  const receipt = await runtime.executeOutputControl(
    harness.invoke,
    testCase.action,
    (challengeNotice) => {
      notice = challengeNotice;
      assert.deepEqual(challengeNotice.action, testCase.action);
      assert.equal(challengeNotice.displayCode, "123456");
      assert.ok(challengeNotice.expiresAtUnixMs > Date.now());
    },
  );
  assert.equal(notice.displayCode, "123456");
  assert.equal(receipt.operation_id, testCase.operationId);
  assert.equal(receipt.outcome, "no_op");
  assert.equal(harness.executeArgs.length, 1);
}

const assertPreActionRejected = async (harness, testCase, message) => {
  await assert.rejects(
    runtime.executeOutputControl(harness.invoke, testCase.action, () => {}),
    (error) => {
      assert.match(String(error), message);
      assert.match(String(error), /nothing was applied/);
      return true;
    },
  );
  assert.equal(harness.executeArgs.length, 0);
};

// Authority fences are untrusted JSON. Every scalar and the checkpoint hash
// must validate before the controller can even prepare a challenge.
for (const authorityMutator of [
  (authority) => ({
    ...authority,
    fence: { ...authority.fence, project_epoch: 0 },
  }),
  (authority) => ({
    ...authority,
    fence: { ...authority.fence, project_checkpoint_hash: hash("A") },
  }),
]) {
  const harness = createHarness({ ...actionCases[0], authorityMutator });
  await assertPreActionRejected(harness, actionCases[0], /authority response was invalid/);
}

// Challenge correlation includes the prepare request id, the exact lower
// hexadecimal argument fingerprint, and canonical 128-bit URL_SAFE_NO_PAD
// identifiers for both the challenge and consent token.
for (const challengeMutator of [
  (challenge) => ({ ...challenge, request_id: challenge.request_id + 1 }),
  (challenge) => ({ ...challenge, argument_fingerprint: hash("G") }),
  (challenge) => ({ ...challenge, challenge_id: "" }),
  (challenge) => ({ ...challenge, challenge_id: "a".repeat(21) }),
  (challenge) => ({ ...challenge, challenge_id: `${"a".repeat(21)}/` }),
  (challenge) => ({ ...challenge, consent_token: "" }),
  (challenge) => ({ ...challenge, consent_token: "b".repeat(23) }),
  (challenge) => ({ ...challenge, consent_token: `${"b".repeat(21)}+` }),
]) {
  const harness = createHarness({ ...actionCases[0], challengeMutator });
  await assertPreActionRejected(
    harness,
    actionCases[0],
    /physical-confirmation challenge was invalid/,
  );
}

// Every status response must acknowledge the issued status request and must
// retain the challenge's original expiry rather than extending it.
for (const statusMutator of [
  (status) => ({ ...status, request_id: status.request_id + 1 }),
  (status) => ({ ...status, expires_at_unix_ms: status.expires_at_unix_ms + 1 }),
]) {
  const harness = createHarness({ ...actionCases[0], statusMutator });
  await assertPreActionRejected(
    harness,
    actionCases[0],
    /physical-confirmation status was invalid/,
  );
}

const assertPostActionUnknown = async (receiptMutator) => {
  const testCase = actionCases[1];
  const harness = createHarness({ ...testCase, receiptMutator });
  await assert.rejects(
    runtime.executeOutputControl(harness.invoke, testCase.action, () => {}),
    (error) => {
      assert.match(String(error), /physical output state is unknown/);
      assert.doesNotMatch(String(error), /nothing was applied/);
      return true;
    },
  );
  assert.equal(harness.executeArgs.length, 1);
};

// Receipts are post-action evidence: identity, hashes, positive sequence,
// before-fence correlation, after-fence validity and outcome semantics all
// fail to an explicitly unknown physical state.
for (const receiptMutator of [
  (response) => ({
    ...response,
    receipt: {
      ...response.receipt,
      fence_before: {
        ...response.receipt.fence_before,
        output_generation: response.receipt.fence_before.output_generation + 1,
      },
    },
  }),
  (response) => ({
    ...response,
    receipt: { ...response.receipt, shape_sha256: hash("A") },
  }),
  (response) => ({
    ...response,
    receipt: { ...response.receipt, argument_fingerprint: hash("d") },
  }),
  (response) => ({
    ...response,
    receipt: { ...response.receipt, audit_sequence: 0 },
  }),
  (response) => ({
    ...response,
    receipt: {
      ...response.receipt,
      fence_after: { ...response.receipt.fence_after, project_revision: 0 },
    },
  }),
  (response) => ({
    ...response,
    receipt: {
      ...response.receipt,
      fence_after: {
        ...response.receipt.fence_after,
        output_generation: response.receipt.fence_after.output_generation + 1,
      },
      outcome: "no_op",
    },
  }),
  (response) => ({
    ...response,
    receipt: { ...response.receipt, outcome: "applied" },
  }),
]) {
  await assertPostActionUnknown(receiptMutator);
}

// A pending physical input that reaches the challenge deadline must fail
// closed and must never reach the action-specific mutation command.
const pendingExpiryCase = actionCases[1];
const pendingExpiryHarness = createHarness({
  ...pendingExpiryCase,
  statusStates: ["pending_physical_input"],
  expiresAtUnixMs: Date.now() + 25,
});
await assert.rejects(
  runtime.executeOutputControl(
    pendingExpiryHarness.invoke,
    pendingExpiryCase.action,
    () => {},
  ),
  /expired or was not received/,
);
assert.equal(pendingExpiryHarness.statusArgs.length, 1);
assert.equal(pendingExpiryHarness.executeArgs.length, 0);

// A typed terminal rejection is final. It is not transport loss and receives
// no replay attempt.
const rejectionCase = actionCases[0];
const rejectionHarness = createHarness({
  ...rejectionCase,
  executeResponse: {
    type: "rejected",
    rejection: {
      operation_id: rejectionCase.operationId,
      request_id: 0,
      error: "forbidden",
    },
  },
});
// The harness fills request_id only after the request exists, so replace the
// response at the terminal boundary while retaining the typed wire shape.
let rejectionExecuteCalls = 0;
const rejectionInvoke = async (command, args) => {
  if (command === rejectionCase.command) {
    rejectionExecuteCalls += 1;
    return {
      type: "rejected",
      rejection: {
        operation_id: args.request.operation_id,
        request_id: args.request.request_id,
        error: "forbidden",
      },
    };
  }
  return rejectionHarness.invoke(command, args);
};
await assert.rejects(
  runtime.executeOutputControl(rejectionInvoke, rejectionCase.action, () => {}),
  (error) => {
    assert.match(String(error), /forbidden/);
    assert.match(String(error), /nothing was applied/);
    return true;
  },
);
assert.equal(
  rejectionExecuteCalls,
  1,
  "typed rejection must not be replayed as a lost transport reply",
);

// Publication failure is a typed terminal response but may be reported after
// the backend entered the physical action. It must not claim that nothing was
// applied, and it still receives no transport retry.
const publicationHarness = createHarness({ ...actionCases[1] });
let publicationExecuteCalls = 0;
const publicationInvoke = async (command, args) => {
  if (command === actionCases[1].command) {
    publicationExecuteCalls += 1;
    return {
      type: "rejected",
      rejection: {
        operation_id: args.request.operation_id,
        request_id: args.request.request_id,
        error: "publication_failed",
      },
    };
  }
  return publicationHarness.invoke(command, args);
};
await assert.rejects(
  runtime.executeOutputControl(publicationInvoke, actionCases[1].action, () => {}),
  (error) => {
    assert.match(String(error), /publication_failed/);
    assert.match(String(error), /physical output state is unknown/);
    assert.doesNotMatch(String(error), /nothing was applied/);
    return true;
  },
);
assert.equal(publicationExecuteCalls, 1);

// A transport-level reply loss retries one time with the same outer and inner
// request objects. The backend receipt lane can therefore return the exact
// terminal result without repeating a physical action.
const replyLossCase = actionCases[2];
const replyLossHarness = createHarness({ ...replyLossCase, loseFirstReply: true });
const replyLossReceipt = await runtime.executeOutputControl(
  replyLossHarness.invoke,
  replyLossCase.action,
  () => {},
);
assert.equal(replyLossReceipt.operation_id, replyLossCase.operationId);
assert.equal(replyLossHarness.executeArgs.length, 2);
assert.equal(replyLossHarness.executeArgs[1], replyLossHarness.executeArgs[0]);
assert.equal(
  replyLossHarness.executeArgs[1].request,
  replyLossHarness.executeArgs[0].request,
);

// If both exact-object attempts lose their transport reply, the controller
// cannot infer whether the action applied and reports the state as unknown.
const doubleLossHarness = createHarness({ ...replyLossCase });
const doubleLossArgs = [];
const doubleLossInvoke = async (command, args) => {
  if (command === replyLossCase.command) {
    doubleLossArgs.push(args);
    throw new Error("synthetic persistent OutputControl reply loss");
  }
  return doubleLossHarness.invoke(command, args);
};
await assert.rejects(
  runtime.executeOutputControl(doubleLossInvoke, replyLossCase.action, () => {}),
  (error) => {
    assert.match(String(error), /physical output state is unknown/);
    assert.doesNotMatch(String(error), /nothing was applied/);
    return true;
  },
);
assert.equal(doubleLossArgs.length, 2);
assert.equal(doubleLossArgs[1], doubleLossArgs[0]);
assert.equal(doubleLossArgs[1].request, doubleLossArgs[0].request);

// Legacy GUI routes remain safer-direction-only. They must not pretend that
// the R4 safety-latch release applies to all/video/per-output authored state.
assert.match(appSource, /import \{ executeOutputControl \} from "\.\/outputControlController"/);
assert.match(appSource, /await executeOutputControl\(\s*invoke,\s*\{\s*kind: "release_blackout"/s);
assert.doesNotMatch(appSource, /invoke\("set_blackout",\s*\{[^}]*enabled:\s*false/);
assert.doesNotMatch(appSource, /invoke\("set_all_blackout",\s*\{[^}]*enabled:\s*false/);
assert.doesNotMatch(appSource, /invoke\("set_video_blackout",\s*\{[^}]*enabled:\s*false/);
assert.doesNotMatch(
  appSource,
  /invoke\("set_video_output_blackout",\s*\{[^}]*blackout:\s*false/,
);
assert.doesNotMatch(videoRuntimeSource, /invoke\("set_video_blackout",\s*\{[^}]*enabled:\s*false/);
assert.match(appSource, /All-output blackout release is unavailable[\s\S]*?no state changed/);
assert.match(videoRuntimeSource, /Video blackout release is unavailable[\s\S]*?no state changed/);
assert.match(appSource, /Per-output blackout release for output[\s\S]*?no state changed/);

// Active role changes, Arm, and Take Over use the canonical R4 controller;
// Standby remains the safer direct disarm path.
assert.match(standbySource, /if \(nextRole === "Standby"\)[\s\S]*?set_output_ownership_role/);
assert.match(standbySource, /await executeOutputControl\([\s\S]*?kind: "arm"/);
assert.match(standbySource, /await executeOutputControl\([\s\S]*?kind: "take_over_standby"/);
const legacyFunction = (source, name) => {
  const start = source.indexOf(`fn ${name}(`);
  assert.notEqual(start, -1, `missing legacy function ${name}`);
  const next = source.indexOf("\n#[tauri::command]", start + 1);
  return source.slice(start, next === -1 ? source.length : next);
};
for (const name of ["set_blackout", "set_all_blackout", "set_video_blackout", "set_video_output_blackout"]) {
  assert.match(legacyFunction(mainSource, name), /if\s+!\w+/);
  assert.match(legacyFunction(mainSource, name), /fail-closed/);
}
assert.match(
  legacyFunction(mainSource, "set_output_ownership_role"),
  /role != MachineOutputRole::Standby[\s\S]*?authenticated local OutputControl R4 path/,
);
assert.match(
  legacyFunction(mainSource, "arm_output_ownership_role"),
  /preferred_role != MachineOutputRole::Standby[\s\S]*?authenticated local OutputControl R4 path/,
);

// The native handler exposes three operation-specific wrappers. There is no
// generic execute handler that could ambiguously project one request to three
// canonical operations.
const handlerBlock = mainSource.match(/tauri::generate_handler!\[([\s\S]*?)\]\)/)?.[1] ?? "";
for (const command of [
  "release_blackout_output_control_v1",
  "arm_output_control_v1",
  "take_over_output_control_v1",
]) {
  assert.match(handlerBlock, new RegExp(`\\b${command}\\b`));
}
assert.doesNotMatch(handlerBlock, /\bexecute_output_control\b/);

// The v2 registry has exactly the three R4 OutputControl operations, while
// consent preparation is support-only and no generic execute operation is
// canonicalized.
for (const operation of [
  "OUTPUT_BLACKOUT_RELEASE_OPERATION_ID",
  "OUTPUT_OWNERSHIP_ARM_OPERATION_ID",
  "OUTPUT_STANDBY_TAKEOVER_OPERATION_ID",
]) {
  assert.match(controlPlaneSource, new RegExp(`ReviewedCanonicalOperation::${operation === "OUTPUT_BLACKOUT_RELEASE_OPERATION_ID" ? "ReleaseBlackout" : operation === "OUTPUT_OWNERSHIP_ARM_OPERATION_ID" ? "ArmOutputOwnership" : "TakeOverStandby"}`));
}
const reviewedCanonicalMapStart = controlPlaneSource.indexOf(
  "fn reviewed_canonical_operation(command: &str)",
);
const reviewedCanonicalMapEnd = controlPlaneSource.indexOf(
  "fn canonical_descriptor_for_source(",
  reviewedCanonicalMapStart,
);
assert.notEqual(reviewedCanonicalMapStart, -1);
assert.notEqual(reviewedCanonicalMapEnd, -1);
const reviewedCanonicalMap = controlPlaneSource.slice(
  reviewedCanonicalMapStart,
  reviewedCanonicalMapEnd,
);
assert.deepEqual(
  [...reviewedCanonicalMap.matchAll(/=> Some\(ReviewedCanonicalOperation::(ReleaseBlackout|ArmOutputOwnership|TakeOverStandby)\)/g)]
    .map((match) => match[1])
    .sort(),
  ["ArmOutputOwnership", "ReleaseBlackout", "TakeOverStandby"],
);
assert.match(controlPlaneSource, /OperationRisk::R4/);
assert.match(
  controlPlaneSource,
  /ReviewedCanonicalOperation::ReleaseBlackout\s*\|\s*ReviewedCanonicalOperation::ArmOutputOwnership\s*\|\s*ReviewedCanonicalOperation::TakeOverStandby[\s\S]*?OperationRisk::R4/,
);
assert.match(controlPlaneSource, /ConsentPolicy::PreparedPhysicalConfirmation/);
assert.match(
  controlPlaneSource,
  /descriptor\.source_id == "prepare_output_consent_v1"[\s\S]*?SourceDisposition::SupportPhase[\s\S]*?OUTPUT_CONSENT_PREPARE_OPERATION_ID/,
);
assert.doesNotMatch(registrySource, /execute_output_control/);

console.log("output control runtime contract: PASS");
