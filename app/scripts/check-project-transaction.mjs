import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";
import ts from "typescript";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const readWorkspaceFile = (relativePath) => readFile(
  path.resolve(scriptDirectory, "..", "..", relativePath),
  "utf8",
);

const [rust, app, transactionModule, recoveryModule, manifestText, invokeCommands] = await Promise.all([
  readWorkspaceFile("app/src-tauri/src/main.rs"),
  readWorkspaceFile("app/src/App.tsx"),
  readWorkspaceFile("app/src/types.ts"),
  readWorkspaceFile("app/src/projectTransactionRecovery.ts"),
  readWorkspaceFile("app/src/tauri-invoke-manifest.json"),
  readWorkspaceFile("app/src/tauriInvokeCommands.ts"),
]);

// This is an executable production-source contract check, not a second
// transaction implementation. The state machine itself is exercised by the
// Rust tests over the real AppState helpers; this gate catches accidental raw
// IPC bypasses and wire/manifest drift before those tests are run.
assert.match(
  rust,
  /fn begin_project_transaction\(\s*window: WebviewWindow[\s\S]*?client_operation_id: String/s,
  "Begin must bind the concrete Tauri WebviewWindow and client operation ID",
);
assert.match(
  rust,
  /fn project_transaction_operation_sequence\(/,
  "the backend must validate a monotonic operation sequence",
);
assert.match(
  rust,
  /project_transaction_operation_highwaters: Mutex<HashMap<String, u64>>/,
  "ACK compaction must retain a per-owner-incarnation high-water map",
);
assert.match(
  rust,
  /project_transaction_retired_owner_bindings: Mutex<HashSet<String>>/,
  "retirement must retain a bounded same-window owner ABA tombstone",
);
assert.match(
  rust,
  /MAX_PROJECT_TRANSACTION_RETIRED_OWNER_BINDINGS[\s\S]*?ensure_project_transaction_owner_binding_not_retired/s,
  "retired owner identities must fail closed without unbounded growth",
);
assert.match(
  rust,
  /fn lock_project_transaction_operation_admission(?:<'a>)?\(/,
  "generic transaction admission must distinguish an exact retry from Display finalization",
);
assert.match(
  rust,
  /fn acknowledge_project_transaction\(/,
  "terminal acknowledgement must have a production Tauri command",
);
assert.match(
  rust,
  /ProjectTransactionReceiptState::Pending[\s\S]*?ProjectTransactionReceiptState::Committed/s,
  "receipts must retain pending and committed terminal states",
);
assert.match(
  rust,
  /ProjectTransactionReceiptState::Cancelled[\s\S]*?Interrupted:/s,
  "cancelled partial edits must publish the Interrupted history path",
);

assert.match(
  app,
  /beginProjectTransactionWithRecovery\(/,
  "frontend Begin must converge through reply-loss recovery",
);
assert.match(
  app,
  /cancelProjectTransactionWithRecovery\(/,
  "frontend Cancel must converge through terminal recovery after a lost reply",
);
assert.match(
  app,
  /const settleTerminal = createAppProjectTransactionTerminalSettlement\(transactionIdentity\);[\s\S]*?await commitProjectTransactionWithRecovery\(transaction, transactionIdentity, settleTerminal\);/s,
  "the central renderer mutation facade must retain one exact settlement across Commit and ACK recovery",
);
assert.match(
  app,
  /recoverProjectTransactionTerminalInForeground\(\s*"commit",\s*commitArgs,\s*identity,/s,
  "frontend Commit reply loss must retry only the exact Commit receipt",
);
assert.match(
  app,
  /recoverProjectTransactionTerminalInForeground\(\s*"cancel",\s*cancelArgs,\s*identity,/s,
  "frontend Cancel reply loss must retry only the exact Cancel receipt",
);
assert.match(
  app,
  /openedTransactionCancellation = cancelProjectTransactionWithRecovery\(transaction, transactionIdentity\);/,
  "outer cleanup must retain the exact Cancel promise instead of swallowing it",
);
assert.doesNotMatch(
  app,
  /cancelProjectTransactionWithRecovery\(transaction, transactionIdentity\)\.catch\(/,
  "outer cleanup must not swallow a failed terminal Cancel",
);
assert.equal(
  [...app.matchAll(/tauriInvoke<T>\(command, ticketedArgs\)/g)].length,
  1,
  "the raw renderer mutation must be dispatched exactly once; recovery may not replay it",
);
assert.match(
  recoveryModule,
  /terminalRecoveryMaxAttempts = 6[\s\S]*?projectTransactionTerminalRecoveryDelayMs[\s\S]*?2 \*\* Math\.max/s,
  "terminal recovery must use a bounded exponential backoff",
);
assert.match(
  recoveryModule,
  /options\.query\(options\.identity\)[\s\S]*?options\.invokeTerminal\(options\.terminalArgs\)/s,
  "terminal recovery must preserve the same identity and ticket shape for query/retry",
);
assert.match(
  recoveryModule,
  /ProjectTransactionTerminalRecoveryHoldError[\s\S]*?command publication is indeterminate/s,
  "indeterminate command publication must remain held rather than be cancelled",
);
assert.match(
  recoveryModule,
  /action === "cancel" && recovery\.command_result != null[\s\S]*?requires the original Commit path/s,
  "published command receipts must never be auto-cancelled",
);
assert.match(
  recoveryModule,
  /requireProjectTransactionTerminalMutation\(mutation\)[\s\S]*?apply\(mutation\)[\s\S]*?await acknowledge\(\)/s,
  "terminal settlement must reject malformed replies before history apply or ACK",
);
assert.match(
  app,
  /await resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation\(\);[\s\S]*?beginProjectTransactionWithRecovery\(/s,
  "the next project mutation must resume structured terminal recovery before opening a new ticket",
);
assert.match(
  app,
  /ProjectTransactionForegroundTerminalRecovery[\s\S]*?action:[\s\S]*?identity:[\s\S]*?terminalArgs:[\s\S]*?retry:/s,
  "foreground terminal recovery must retain its exact action, identity, ticket shape, and retry callback",
);
assert.match(
  app,
  /ProjectTransactionTerminalMalformedMutationError[\s\S]*?ProjectTransactionTerminalRecoveryHoldError/s,
  "a malformed terminal result must not fall through to outer Cancel",
);
assert.match(
  app,
  /projectTransactionOperationId = \(\) =>[\s\S]*project-op:\$\{/s,
  "frontend operation IDs must carry a sequence and nonce",
);
assert.match(
  transactionModule,
  /projectTransactionRecoveryCanAdopt[\s\S]*status === "pending"/s,
  "only pending receipts may be adopted",
);
assert.match(
  transactionModule,
  /project-transaction-v\$\{PROJECT_TRANSACTION_SCHEMA_VERSION\}/,
  "frontend canonical shape must include the schema version",
);

const manifest = JSON.parse(manifestText);
for (const command of [
  "acknowledge_project_transaction",
  "adopt_project_transaction",
  "query_project_transaction",
]) {
  assert.equal(manifest.filter((entry) => entry === command).length, 1, `${command} manifest entry`);
  assert.match(invokeCommands, new RegExp(`\\"${command}\\"`), `${command} invoke allowlist entry`);
}

const sliceAppHandler = (start, end) => {
  const from = app.indexOf(start);
  const to = app.indexOf(end, from + start.length);
  assert(from >= 0 && to > from, `could not isolate ${start}`);
  return app.slice(from, to);
};
const createCueListHandler = sliceAppHandler(
  "const createCueList = async",
  "const createSceneInCueList = async",
);
assert.match(
  createCueListHandler,
  /flushProjectControlMappingsBeforeMutation\(\)[\s\S]*?expectedEpoch: currentAuthority\.project_epoch,[\s\S]*?expectedRevision: currentAuthority\.project_revision,[\s\S]*?expectedCheckpointHash: currentAuthority\.checkpoint_hash,[\s\S]*?ownerId: projectTransactionOwnerId,/s,
  "Cue List create must capture the complete post-flush E/R/H/owner authority fence",
);
assert.match(
  createCueListHandler,
  /invoke<ProjectHistoryMutationResult & \{ cue_list_id: number \}>\("create_cue_list"[\s\S]*?authoritativeApplicationIsCurrent\(result\)[\s\S]*?result\.cue_list_id/s,
  "Cue List create must consume the authoritative receipt and committed Bank ID",
);
const renameCueListHandler = sliceAppHandler(
  "const renameCueList = async",
  "const removeCueList = async",
);
assert.match(
  renameCueListHandler,
  /invoke<ProjectHistoryMutationResult>\("rename_cue_list"[\s\S]*?expectedEpoch: currentAuthority\.project_epoch,[\s\S]*?expectedRevision: currentAuthority\.project_revision,[\s\S]*?expectedCheckpointHash: currentAuthority\.checkpoint_hash,[\s\S]*?ownerId: projectTransactionOwnerId,/s,
  "Cue List rename must capture the complete post-flush E/R/H/owner authority fence",
);
assert.match(
  renameCueListHandler,
  /authoritativeApplicationIsCurrent\(result\)[\s\S]*?await refreshSnapshot\(\)[\s\S]*?renamed\.label !== label/s,
  "Cue List rename must reject stale acknowledgement and verify the refreshed label",
);
const removeCueListHandler = sliceAppHandler(
  "const removeCueList = async",
  "const reorderCueLists = async",
);
const reorderCueListsHandler = sliceAppHandler(
  "const reorderCueLists = async",
  "const setCueList = async",
);
for (const [label, handler, command] of [
  ["create", createCueListHandler, "create_cue_list"],
  ["rename", renameCueListHandler, "rename_cue_list"],
  ["delete", removeCueListHandler, "delete_cue_list"],
  ["reorder", reorderCueListsHandler, "reorder_cue_lists"],
]) {
  assert.match(
    handler,
    /if \(!isTauriRuntime\(\)\)[\s\S]*?return true;[\s\S]*?try \{[\s\S]*?flushProjectControlMappingsBeforeMutation\(\)[\s\S]*?invoke[\s\S]*?authoritativeApplicationIsCurrent\(result\)[\s\S]*?await refreshSnapshot\(\)/s,
    `Cue List ${label} must have exactly one local non-Tauri fixture lane and one post-flush Tauri receipt/refresh lane`,
  );
  assert.doesNotMatch(
    handler,
    /viewportFixture === "scene-matrix"/,
    `Cue List ${label} must not retain a scene-matrix-specific Tauri lane`,
  );
  assert.match(
    handler,
    new RegExp(`invoke[\\s\\S]*?"${command}"[\\s\\S]*?ownerId: projectTransactionOwnerId`, "s"),
    `Cue List ${label} Tauri lane must retain the explicit owner fence`,
  );
}
assert.match(
  app,
  /const strictBankMutation = command === "create_cue_list"[\s\S]*?command === "rename_cue_list"[\s\S]*?command === "reorder_cue_lists"[\s\S]*?command === "delete_cue_list";[\s\S]*?strictBankMutation[\s\S]*?request:\s*\{[\s\S]*?\.\.\.commandArgs,[\s\S]*?expectedEpoch,[\s\S]*?ownerId: projectTransactionOwnerId,[\s\S]*?\}/s,
  "every authoritative Bank mutation must cross Tauri in one strict nested request object",
);
assert.match(
  app,
  /invoke<ProjectHistoryMutationResult>\("reorder_cue_lists", \{\s*cueListIds: orderedCueListIds,/s,
  "Bank reorder must not retain a legacy nested sub-request inside the strict request envelope",
);
assert.match(
  app,
  /const strictTicketedRequestMutation = command === "set_fixture_transform"[\s\S]*?command === "move_cue_between_scene_banks_batch";[\s\S]*?const ticketedArgs = strictTicketedRequestMutation\s*\? \{ request: ticketedRequest \}\s*:\s*ticketedRequest;/s,
  "fixture transforms and cross-Bank Scene moves must cross Tauri in one strict nested ticket request object",
);
for (const [requestType, handler] of [
  ["AuthoritativeCueListCreateRequest", "create_cue_list"],
  ["AuthoritativeCueListRenameRequest", "rename_cue_list"],
  ["AuthoritativeCueListReorderRequest", "reorder_cue_lists"],
  ["AuthoritativeCueListDeleteRequest", "delete_cue_list"],
]) {
  assert.match(
    rust,
    new RegExp(`#\\[serde\\(rename_all = "camelCase", deny_unknown_fields\\)\\]\\s*struct ${requestType}[\\s\\S]*?fn ${handler}\\([\\s\\S]*?request: ${requestType}`, "s"),
    `native ${handler} must reject flat, unknown, and legacy request fields`,
  );
}
assert.match(
  rust,
  /#\[serde\(rename_all = "camelCase", deny_unknown_fields\)\]\s*struct SetFixtureTransformRequest[\s\S]*?fn set_fixture_transform\([\s\S]*?request: SetFixtureTransformRequest/s,
  "native fixture transform must reject flat, unknown, and legacy request fields",
);
assert.match(
  rust,
  /#\[serde\(rename_all = "camelCase", deny_unknown_fields\)\]\s*struct MoveCueBetweenSceneBanksBatchRequest[\s\S]*?fn move_cue_between_scene_banks_batch\([\s\S]*?request: MoveCueBetweenSceneBanksBatchRequest/s,
  "native cross-Bank Scene move must reject flat, unknown, and legacy request fields",
);
const setCueListHandler = sliceAppHandler(
  "const setCueList = async",
  "const setCuePalette = async",
);
assert.match(
  setCueListHandler,
  /runSceneMatrixBankMoveTransaction\(cue, cueListId,[\s\S]*?null, "after"\)/s,
  "Cue Management bank moves must use the authoritative cross-Bank batch transaction",
);
assert.doesNotMatch(
  setCueListHandler,
  /["']set_cue_list["']/,
  "Cue Management must not retain the raw set_cue_list IPC route",
);
assert.equal(manifest.includes("set_cue_list"), false, "retired set_cue_list is absent from the frontend manifest");
assert.doesNotMatch(invokeCommands, /"set_cue_list"/, "retired set_cue_list is absent from the frontend invoke inventory");

// The recovery helper is TypeScript-only production code. Transpile its
// type-only imports in-process so these deterministic checks execute the
// actual state machine rather than a second hand-written JavaScript model.
const recoveryJavaScript = ts.transpileModule(recoveryModule, {
  compilerOptions: {
    target: ts.ScriptTarget.ES2022,
    module: ts.ModuleKind.ESNext,
    verbatimModuleSyntax: true,
  },
  fileName: "projectTransactionRecovery.ts",
  reportDiagnostics: true,
});
assert.equal(
  recoveryJavaScript.diagnostics?.filter((diagnostic) => diagnostic.category === ts.DiagnosticCategory.Error).length ?? 0,
  0,
  "transaction recovery helper must transpile without TypeScript diagnostics",
);
const recovery = await import(`data:text/javascript;base64,${Buffer.from(recoveryJavaScript.outputText).toString("base64")}`);

const testIdentity = Object.freeze({
  clientOperationId: "project-op:41:terminal-recovery",
  shapeFingerprint: "project-transaction-v7|command=set_attribute|label=Set Attribute|coalesce=",
  commandName: "set_attribute",
  schemaVersion: 7,
  ownerId: "renderer:terminal-recovery",
});
const testTicket = Object.freeze({
  transaction_id: 41,
  project_epoch: 12,
  project_revision: 18,
  project_checkpoint_hash: "a".repeat(64),
  client_operation_id: testIdentity.clientOperationId,
  shape_fingerprint: testIdentity.shapeFingerprint,
  schema_version: testIdentity.schemaVersion,
  owner_id: testIdentity.ownerId,
  window_label: "main",
  owner_incarnation: 1,
  label: "Set Attribute",
  coalesce_key: "",
});
const testHistoryStatus = Object.freeze({
  can_undo: true,
  can_redo: false,
  undo_depth: 1,
  redo_depth: 0,
  undo_label: "Set Attribute",
  redo_label: null,
  project_epoch: 12,
  project_revision: 19,
  checkpoint_hash: "b".repeat(64),
  history_generation: 2,
  undo_entry_id: null,
  undo_checkpoint_hash: null,
  redo_entry_id: null,
  redo_checkpoint_hash: null,
});
const testMutation = Object.freeze({
  history_status: testHistoryStatus,
  authority: {
    project_epoch: 12,
    project_revision: 19,
    checkpoint_hash: "b".repeat(64),
    publication_generation: 2,
    publication_kind: "Mutation",
    mapping_replacement_generation: 0,
    authority_disposition_generation: 0,
    authority_disposition: "Clean",
    recovery_authority_serial: 0,
    recovery_authority_last_transition: { kind: "legacy_unknown" },
    path_generation: 0,
    history_generation: 2,
    current_project_path: null,
    snapshot: {},
    profiles: [],
    fixture_groups: [],
    operator_policy: null,
    midi_mappings: [],
    osc_mappings: [],
    dmx_mappings: [],
    dj_track_triggers: [],
    history: testHistoryStatus,
    input_runtime: {},
  },
});
const pendingRecovery = (commandInFlight, overrides = {}) => ({
  status: "pending",
  ticket: testTicket,
  command_result: null,
  command_in_flight: commandInFlight,
  command_indeterminate_error: null,
  ...overrides,
});

const exerciseBusyToIdleTerminalRecovery = async (action) => {
  const queries = [pendingRecovery(true), pendingRecovery(false)];
  const queryIdentities = [];
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  const terminalCalls = [];
  const waits = [];
  const result = await recovery.recoverProjectTransactionTerminalAction({
    identity: testIdentity,
    terminalArgs,
    action,
    query: async (identity) => {
      queryIdentities.push(identity);
      return queries.shift() ?? null;
    },
    invokeTerminal: async (args) => {
      terminalCalls.push(args);
      return testMutation;
    },
    wait: async (milliseconds) => { waits.push(milliseconds); },
    reportUnresolved: (message) => { throw new Error(`unexpected unresolved recovery: ${message}`); },
  });
  assert.deepEqual(result, { kind: "operation", mutation: testMutation }, `${action} busy-to-idle result`);
  assert.deepEqual(waits, [25], `${action} waits for the in-flight command before retrying`);
  assert.equal(queryIdentities.length, 2, `${action} queries until the same receipt becomes idle`);
  assert(queryIdentities.every((identity) => identity === testIdentity), `${action} keeps the exact query identity`);
  assert.equal(terminalCalls.length, 1, `${action} retries its terminal command once after idle`);
  assert.strictEqual(terminalCalls[0], terminalArgs, `${action} preserves the same frozen terminal ticket shape`);
};

await exerciseBusyToIdleTerminalRecovery("commit");
await exerciseBusyToIdleTerminalRecovery("cancel");

{
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  let terminalCalls = 0;
  let identitySeen = null;
  const result = await recovery.recoverProjectTransactionTerminalAction({
    identity: testIdentity,
    terminalArgs,
    action: "commit",
    query: async (identity) => {
      identitySeen = identity;
      return { status: "committed", mutation: testMutation, command_result: null };
    },
    invokeTerminal: async () => {
      terminalCalls += 1;
      return testMutation;
    },
    wait: async () => { throw new Error("terminal reply-loss recovery must not wait"); },
    reportUnresolved: (message) => { throw new Error(`unexpected reply-loss failure: ${message}`); },
  });
  assert.equal(result.kind, "terminal", "a lost Commit reply adopts only the retained terminal");
  assert.strictEqual(identitySeen, testIdentity, "reply-loss recovery uses the original identity object");
  assert.equal(terminalCalls, 0, "a retained terminal reply is never re-committed");
}

{
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  const reports = [];
  let terminalCalls = 0;
  await assert.rejects(
    recovery.recoverProjectTransactionTerminalAction({
      identity: testIdentity,
      terminalArgs,
      action: "cancel",
      query: async () => pendingRecovery(false, { command_indeterminate_error: "native ACK disconnected" }),
      invokeTerminal: async () => {
        terminalCalls += 1;
        return testMutation;
      },
      wait: async () => undefined,
      reportUnresolved: (message) => reports.push(message),
    }),
    (error) => error?.name === "ProjectTransactionTerminalRecoveryHoldError",
    "indeterminate publication must stay visibly held",
  );
  assert.equal(terminalCalls, 0, "indeterminate publication never auto-cancels");
  assert.match(reports[0] ?? "", /held: command publication is indeterminate/, "hold is reported to the foreground");
}

{
  let applyCount = 0;
  let acknowledgeCount = 0;
  const settle = recovery.createProjectTransactionTerminalSettlement(
    () => { applyCount += 1; },
    async () => { acknowledgeCount += 1; },
  );
  await settle(testMutation);
  await settle(testMutation);
  assert.equal(applyCount, 1, "the terminal history mutation is applied exactly once");
  assert.equal(acknowledgeCount, 1, "the terminal receipt is acknowledged exactly once");
}

{
  // Direct Commit reached native success, but the first ACK reply is lost.
  // Retrying must retain the terminal settlement: one raw mutation, one
  // Commit, no compensating Cancel, one history application, then the exact
  // acknowledgement retry.
  let rawMutationCalls = 1;
  let commitCalls = 1;
  let cancelCalls = 0;
  let applyCount = 0;
  let acknowledgeCount = 0;
  const acknowledgeIdentities = [];
  const settle = recovery.createProjectTransactionTerminalSettlement(
    () => { applyCount += 1; },
    async () => {
      acknowledgeCount += 1;
      acknowledgeIdentities.push(testIdentity);
      if (acknowledgeCount === 1) throw new Error("first ACK reply lost");
    },
  );
  await recovery.settleProjectTransactionTerminalAcknowledgement(
    settle,
    testMutation,
    async () => undefined,
    (message) => { throw new Error(`unexpected direct-Commit ACK deadline: ${message}`); },
  );
  assert.equal(rawMutationCalls, 1, "direct Commit ACK recovery never replays the raw mutation");
  assert.equal(commitCalls, 1, "direct Commit ACK recovery never reissues Commit");
  assert.equal(cancelCalls, 0, "direct Commit ACK recovery never sends Cancel");
  assert.equal(applyCount, 1, "direct Commit ACK recovery applies history once");
  assert.equal(acknowledgeCount, 2, "direct Commit ACK recovery retries ACK once");
  assert(acknowledgeIdentities.every((identity) => identity === testIdentity), "ACK retries retain the same identity closure");
}

{
  // A recovered Commit terminal plus an ACK reply loss has the same rule: it
  // observes the retained Commit once and then retries only its ACK.
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  let rawMutationCalls = 1;
  let commitCalls = 1;
  let cancelCalls = 0;
  let terminalCalls = 0;
  const recovered = await recovery.recoverProjectTransactionTerminalAction({
    identity: testIdentity,
    terminalArgs,
    action: "commit",
    query: async () => ({ status: "committed", mutation: testMutation, command_result: null }),
    invokeTerminal: async () => {
      terminalCalls += 1;
      return testMutation;
    },
    wait: async () => { throw new Error("recovered terminal must not wait"); },
    reportUnresolved: (message) => { throw new Error(`unexpected recovered Commit failure: ${message}`); },
  });
  assert.equal(recovered.kind, "terminal", "recovered Commit adopts the retained terminal");
  let applyCount = 0;
  let acknowledgeCount = 0;
  const settle = recovery.createProjectTransactionTerminalSettlement(
    () => { applyCount += 1; },
    async () => {
      acknowledgeCount += 1;
      if (acknowledgeCount === 1) throw new Error("recovered ACK reply lost");
    },
  );
  await recovery.settleProjectTransactionTerminalAcknowledgement(
    settle,
    recovered.recovery.mutation,
    async () => undefined,
    (message) => { throw new Error(`unexpected recovered-Commit ACK deadline: ${message}`); },
  );
  assert.equal(rawMutationCalls, 1, "recovered Commit ACK failure does not replay raw mutation");
  assert.equal(commitCalls, 1, "recovered Commit ACK failure does not issue another Commit");
  assert.equal(cancelCalls, 0, "recovered Commit ACK failure never sends Cancel");
  assert.equal(terminalCalls, 0, "retained Commit never reissues its terminal command");
  assert.equal(applyCount, 1, "recovered Commit applies history once across ACK retry");
  assert.equal(acknowledgeCount, 2, "recovered Commit retries ACK with the same settlement");
}

{
  let applyCount = 0;
  let acknowledgeCount = 0;
  const settle = recovery.createProjectTransactionTerminalSettlement(
    () => { applyCount += 1; },
    async () => { acknowledgeCount += 1; },
  );
  const malformed = { ...testMutation, authority: { ...testMutation.authority, snapshot: null } };
  await assert.rejects(
    settle(malformed),
    (error) => error?.name === "ProjectTransactionTerminalMalformedMutationError",
    "malformed terminal replies must fail closed before history apply or ACK",
  );
  assert.equal(applyCount, 0, "malformed terminal reply never applies history");
  assert.equal(acknowledgeCount, 0, "malformed terminal reply never acknowledges its receipt");
}

{
  // The initial bounded batch may be shorter than a native in-flight command.
  // Its later retry uses the exact retained ticket, then unblocks the next
  // Begin without ever replaying the raw mutation.
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  const reports = [];
  let rawMutationCalls = 1;
  let firstBatchQueries = 0;
  await assert.rejects(
    recovery.recoverProjectTransactionTerminalAction({
      identity: testIdentity,
      terminalArgs,
      action: "commit",
      query: async () => {
        firstBatchQueries += 1;
        return pendingRecovery(true);
      },
      invokeTerminal: async () => {
        throw new Error("in-flight terminal must not be retried during first batch");
      },
      wait: async () => undefined,
      reportUnresolved: (message) => reports.push(message),
    }),
    (error) => error?.name === "ProjectTransactionTerminalRecoveryUnresolvedError",
    "long in-flight terminal becomes a visible retained foreground recovery",
  );
  assert.equal(firstBatchQueries, 6, "first bounded batch probes the same in-flight receipt six times");
  const laterQueries = [pendingRecovery(true), pendingRecovery(false)];
  const laterIdentities = [];
  const laterTerminalArgs = [];
  const later = await recovery.recoverProjectTransactionTerminalAction({
    identity: testIdentity,
    terminalArgs,
    action: "commit",
    query: async (identity) => {
      laterIdentities.push(identity);
      return laterQueries.shift() ?? null;
    },
    invokeTerminal: async (args) => {
      laterTerminalArgs.push(args);
      return testMutation;
    },
    wait: async () => undefined,
    reportUnresolved: (message) => { throw new Error(`unexpected later recovery failure: ${message}`); },
  });
  assert.equal(later.kind, "operation", "later exact retry settles once the terminal lane becomes idle");
  assert.equal(rawMutationCalls, 1, "later recovery never replays the raw mutation");
  assert(laterIdentities.every((identity) => identity === testIdentity), "later recovery retains the original identity");
  assert.strictEqual(laterTerminalArgs[0], terminalArgs, "later recovery retains the same terminal ticket shape");
  let beginCalls = 0;
  if (later.kind === "operation") beginCalls += 1;
  assert.equal(beginCalls, 1, "the next Begin is unblocked only after exact terminal recovery succeeds");
  assert.match(reports[0] ?? "", /remains pending after 6 bounded attempts/, "first batch leaves a visible recovery report");
}

{
  const terminalArgs = recovery.projectTransactionTerminalArgs(testTicket, testIdentity);
  const reports = [];
  let waits = 0;
  await assert.rejects(
    recovery.recoverProjectTransactionTerminalAction({
      identity: testIdentity,
      terminalArgs,
      action: "commit",
      query: async () => { throw new Error("recovery transport unavailable"); },
      invokeTerminal: async () => testMutation,
      wait: async () => { waits += 1; },
      reportUnresolved: (message) => reports.push(message),
    }),
    (error) => error?.name === "ProjectTransactionTerminalRecoveryUnresolvedError",
    "recovery deadline must fail closed",
  );
  assert.equal(waits, 5, "bounded retry deadline has five inter-attempt waits");
  assert.match(reports[0] ?? "", /remains pending after 6 bounded attempts/, "deadline remains visible in the foreground");
}

console.log("project transaction executable production-contract checks passed");
