import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const loadTsModule = async (relativePath) => {
  const source = (await readFile(new URL(relativePath, import.meta.url), "utf8")).replace(/\r\n/g, "\n");
  const output = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: relativePath,
  });
  return import(`data:text/javascript;base64,${Buffer.from(output.outputText).toString("base64")}`);
};

const helper = await loadTsModule("../src/patchTransactionD2.ts");
const [app, form, browser, types, d2Source, backend] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
  readFile(new URL("../src/components/PatchFixtureFormPanel.tsx", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
  readFile(new URL("../src/components/PatchProfileBrowserPanel.tsx", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
  readFile(new URL("../src/patchTransactionD2.ts", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
  readFile(new URL("../src-tauri/src/main.rs", import.meta.url), "utf8").then((source) => source.replace(/\r\n/g, "\n")),
]);

const patchArgs = { requests: [{ label: "A" }, { label: "B" }] };
const validPatch = { kind: "patch_fixtures", request_digest: "a".repeat(64), fixture_ids: [41, 42] };
assert.equal(helper.projectTransactionCommandResultIsWellFormed(validPatch, "patch_fixtures", patchArgs), true);
for (const malformed of [
  { ...validPatch, fixture_ids: [41] },
  { ...validPatch, fixture_ids: [41, 41] },
  { ...validPatch, fixture_ids: [41, 1.5] },
  { ...validPatch, request_digest: " " },
  { ...validPatch, request_digest: "A".repeat(64) },
  { ...validPatch, kind: "repair_fixture_profile" },
  { ...validPatch, future_field: true },
]) {
  assert.equal(helper.projectTransactionCommandResultIsWellFormed(malformed, "patch_fixtures", patchArgs), false);
}
const repairArgs = { fixtureId: 9 };
const validRepair = { kind: "repair_fixture_profile", request_digest: "b".repeat(64), fixture_id: 9 };
assert.equal(helper.projectTransactionCommandResultIsWellFormed(validRepair, "repair_fixture_profile", repairArgs), true);
assert.equal(
  helper.projectTransactionCommandResultIsWellFormed({ ...validRepair, fixture_id: 10 }, "repair_fixture_profile", repairArgs),
  false,
);
assert.equal(
  helper.projectTransactionCommandResultIsWellFormed({ ...validRepair, fixture_ids: [9] }, "repair_fixture_profile", repairArgs),
  false,
  "the Repair receipt rejects a PATCH-shaped future/extra field",
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "pending", command_result: validPatch }, "patch_fixtures", patchArgs),
  { kind: "published", result: validPatch },
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "pending", command_result: null, command_in_flight: true }, "patch_fixtures", patchArgs),
  { kind: "unconfirmed" },
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "committed" }, "patch_fixtures", patchArgs),
  { kind: "unconfirmed" },
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "pending", command_result: null, command_in_flight: false, command_indeterminate_error: null }, "patch_fixtures", patchArgs),
  { kind: "not_published" },
  "a null fault latch plus false in-flight is the backend's durable non-publication fence",
);
assert.equal(
  helper.publishedCommandRecoveryDisposition(
    helper.publishedCommandRecoveryDecision({ status: "pending", command_result: null, command_in_flight: false, command_indeterminate_error: null }, "patch_fixtures", patchArgs),
  ),
  "cancel",
  "a normal backend rejection reaches the existing terminal Cancel lane",
);
assert.equal(
  helper.publishedCommandRecoveryDisposition(
    helper.publishedCommandRecoveryDecision({ status: "pending", command_result: null, command_in_flight: true }, "patch_fixtures", patchArgs),
  ),
  "hold",
  "a slow in-flight command never reaches Cancel or replay",
);
{
  const terminal = await helper.waitForPublishedCommandRecovery(
    async () => ({ status: "pending", command_result: null, command_in_flight: false, command_indeterminate_error: null }),
    "patch_fixtures",
    patchArgs,
    async () => assert.fail("a backend-confirmed rejection must not wait before Cancel"),
  );
  assert.equal(helper.publishedCommandRecoveryDisposition(terminal), "cancel");
}
{
  const malformed = { ...validPatch, fixture_ids: [41], command_in_flight: false };
  const decision = helper.publishedCommandRecoveryDecision(
    { status: "pending", command_result: malformed, command_in_flight: false },
    "patch_fixtures",
    patchArgs,
  );
  assert.equal(decision.kind, "indeterminate", "a malformed non-null receipt may be post-B and cannot Cancel");
  assert.equal(helper.publishedCommandRecoveryDisposition(decision), "hold");
  let waits = 0;
  const terminal = await helper.waitForPublishedCommandRecovery(
    async () => ({ status: "pending", command_result: malformed, command_in_flight: false }),
    "patch_fixtures",
    patchArgs,
    async () => { waits += 1; },
  );
  assert.equal(terminal.kind, "indeterminate");
  assert.equal(waits, 0, "malformed published receipt faults immediately rather than hiding in an infinite poll");
}
{
  const decision = helper.publishedCommandRecoveryDecision(
    {
      status: "pending",
      command_result: null,
      command_in_flight: false,
      command_indeterminate_error: "engine acknowledgement disconnected",
    },
    "patch_fixtures",
    patchArgs,
  );
  assert.deepEqual(decision, { kind: "indeterminate", error: "engine acknowledgement disconnected" });
  assert.equal(helper.publishedCommandRecoveryDisposition(decision), "hold", "indeterminate outcome cannot Cancel");
  let waits = 0;
  const terminal = await helper.waitForPublishedCommandRecovery(
    async () => ({
      status: "pending",
      command_result: null,
      command_in_flight: false,
      command_indeterminate_error: "engine acknowledgement disconnected",
    }),
    "patch_fixtures",
    patchArgs,
    async () => { waits += 1; },
  );
  assert.equal(terminal.kind, "indeterminate");
  assert.equal(waits, 0, "an explicit indeterminate error is a restart-required fault, not a concealed poll");
}
{
  const recoveries = [
    { status: "pending", command_result: null, command_in_flight: true },
    { status: "pending", command_result: null, command_in_flight: true },
    { status: "pending", command_result: null, command_in_flight: true },
    { status: "pending", command_result: validPatch, command_in_flight: true },
  ];
  const waits = [];
  let queries = 0;
  const terminal = await helper.waitForPublishedCommandRecovery(
    async () => recoveries[queries++],
    "patch_fixtures",
    patchArgs,
    async (milliseconds) => { waits.push(milliseconds); },
  );
  assert.deepEqual(terminal, { kind: "published", result: validPatch });
  assert.equal(queries, 4, "three slow in-flight replies retain the same recovery before publication converges");
  assert.deepEqual(waits, [25, 50, 100], "recovery uses bounded backoff while the shared UI lane remains busy");
}
for (const malformedInFlight of [undefined, null, 0, "false"]) {
  assert.deepEqual(
    helper.publishedCommandRecoveryDecision(
      { status: "pending", command_result: null, command_in_flight: malformedInFlight },
      "patch_fixtures",
      patchArgs,
    ),
    { kind: "unconfirmed" },
    "missing or malformed command_in_flight cannot manufacture a safe Cancel",
  );
}
assert.deepEqual(
  helper.publishedCommandRecoveryDecision(
    { status: "pending", command_result: null, command_in_flight: false },
    "patch_fixtures",
    patchArgs,
  ),
  { kind: "unconfirmed" },
  "a missing fault latch cannot create a safe Cancel",
);
for (const malformedFaultLatch of ["", 0]) {
  const decision = helper.publishedCommandRecoveryDecision(
    {
      status: "pending",
      command_result: null,
      command_in_flight: false,
      command_indeterminate_error: malformedFaultLatch,
    },
    "patch_fixtures",
    patchArgs,
  );
  assert.equal(decision.kind, "indeterminate", "a malformed fault latch is restart-required rather than Cancel-safe");
  assert.equal(helper.publishedCommandRecoveryDisposition(decision), "hold");
}
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "cancelled" }, "patch_fixtures", patchArgs),
  { kind: "not_published" },
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "acknowledged" }, "patch_fixtures", patchArgs),
  { kind: "unconfirmed" },
  "payloadless acknowledgement cannot prove B was not published",
);

// ---- D4 Stage renderer-ticketed routes share the same recovery query. ----
const stageRoutes = {
  unit: [
    "set_fixture_transform",
    "set_stage_map_config",
    "apply_stage_map_preset",
    "remove_stage_map_preset",
    "set_stage_object",
    "remove_stage_object",
  ],
  stageObjectId: ["add_stage_object"],
  label: ["save_stage_map_preset", "import_stage_map_preset"],
};
const allStageRoutes = [...stageRoutes.unit, ...stageRoutes.stageObjectId, ...stageRoutes.label];
const stageReceipt = (route, overrides = {}) => ({
  kind: "stage_project_mutation",
  command_name: route,
  request_digest: "c".repeat(64),
  outcome: "applied",
  stage_object_id: null,
  label: null,
  ...overrides,
});
const validStageReceiptByRoute = new Map([
  ...stageRoutes.unit.map((route) => [route, stageReceipt(route)]),
  ["add_stage_object", stageReceipt("add_stage_object", { stage_object_id: 7 })],
  ["save_stage_map_preset", stageReceipt("save_stage_map_preset", { label: "Preset" })],
  ["import_stage_map_preset", stageReceipt("import_stage_map_preset", { label: "Imported" })],
]);
const stageArgsByRoute = new Map([
  ["save_stage_map_preset", { label: "  Preset  " }],
  ["import_stage_map_preset", { preset: { label: "  Imported  " } }],
]);
const stageArgs = (route) => stageArgsByRoute.get(route) ?? {};
assert.equal(allStageRoutes.length, 9, "exactly nine Stage routes are renderer-ticketed");
for (const route of allStageRoutes) {
  const receipt = validStageReceiptByRoute.get(route);
  assert.equal(
    helper.projectTransactionCommandResultIsWellFormed(receipt, route, stageArgs(route)),
    true,
    `${route} must accept its exact durable Stage receipt`,
  );
  assert.deepEqual(
    helper.publishedCommandRecoveryDecision({ status: "pending", command_result: receipt }, route, stageArgs(route)),
    { kind: "published", result: receipt },
    `${route} recovery adopts its published Stage receipt`,
  );
}
// Recovered Stage receipts convert back to the exact legacy reply shapes.
for (const route of stageRoutes.unit) {
  const converted = helper.publishedCommandLegacyReplyFromRecoveredResult(route, validStageReceiptByRoute.get(route), {});
  assert.equal(converted, null, `${route} recovered replies keep the legacy JSON null shape`);
}
assert.deepEqual(
  allStageRoutes.filter((route) => helper.publishedCommandLegacyReplyFromRecoveredResult(route, validStageReceiptByRoute.get(route), stageArgs(route)) !== null).sort(),
  ["add_stage_object", "import_stage_map_preset", "save_stage_map_preset"],
  "only add/save/import recovered replies carry a legacy value",
);
assert.equal(helper.publishedCommandLegacyReplyFromRecoveredResult("add_stage_object", validStageReceiptByRoute.get("add_stage_object"), {}), 7);
assert.equal(helper.publishedCommandLegacyReplyFromRecoveredResult("save_stage_map_preset", validStageReceiptByRoute.get("save_stage_map_preset"), stageArgs("save_stage_map_preset")), "Preset");
assert.equal(helper.publishedCommandLegacyReplyFromRecoveredResult("import_stage_map_preset", validStageReceiptByRoute.get("import_stage_map_preset"), stageArgs("import_stage_map_preset")), "Imported");
assert.deepEqual(
  helper.publishedCommandLegacyReplyFromRecoveredResult("patch_fixtures", validPatch, patchArgs),
  validPatch,
  "PATCH keeps exposing and validating its full receipt; only Stage receipts are narrowed",
);
// Route-specific payload invariants and malformed/future fields stay rejected.
for (const route of allStageRoutes) {
  const receipt = validStageReceiptByRoute.get(route);
  for (const malformed of [
    { ...receipt, future_field: true },
    { ...receipt, request_digest: "C".repeat(64) },
    { ...receipt, request_digest: stageReceipt(route).request_digest.slice(1) },
    { ...receipt, outcome: "Applied" },
    { ...receipt, outcome: "failed" },
  ]) {
    assert.equal(
      helper.projectTransactionCommandResultIsWellFormed(malformed, route, stageArgs(route)),
      false,
      `${route} rejects a malformed or future-field Stage receipt`,
    );
  }
  const otherRoute = route === "add_stage_object" ? "save_stage_map_preset" : "add_stage_object";
  assert.equal(
    helper.projectTransactionCommandResultIsWellFormed(validStageReceiptByRoute.get(otherRoute), route, stageArgs(route)),
    false,
    `${route} rejects a receipt stored under ${otherRoute}`,
  );
}
{
  const complete = validStageReceiptByRoute.get("remove_stage_object");
  for (const key of ["kind", "command_name", "request_digest", "outcome", "stage_object_id", "label"]) {
    const missing = { ...complete };
    delete missing[key];
    assert.equal(
      helper.projectTransactionCommandResultIsWellFormed(missing, "remove_stage_object", {}),
      false,
      `a Stage receipt without ${key} is not well formed`,
    );
  }
}
for (const route of stageRoutes.unit) {
  assert.equal(helper.projectTransactionCommandResultIsWellFormed(stageReceipt(route, { stage_object_id: 7 }), route, {}), false);
  assert.equal(helper.projectTransactionCommandResultIsWellFormed(stageReceipt(route, { label: "x" }), route, {}), false);
}
for (const badId of [0, -1, 1.5, Number.MAX_SAFE_INTEGER + 1]) {
  assert.equal(
    helper.projectTransactionCommandResultIsWellFormed(stageReceipt("add_stage_object", { stage_object_id: badId }), "add_stage_object", {}),
    false,
    `add_stage_object requires a positive safe allocated ID, rejected ${badId}`,
  );
}
assert.equal(helper.projectTransactionCommandResultIsWellFormed(stageReceipt("add_stage_object", { label: "x" }), "add_stage_object", {}), false);
for (const route of stageRoutes.label) {
  for (const badLabel of [null, "", "   "]) {
    assert.equal(helper.projectTransactionCommandResultIsWellFormed(stageReceipt(route, { label: badLabel }), route, {}), false);
  }
  assert.equal(helper.projectTransactionCommandResultIsWellFormed(stageReceipt(route, { stage_object_id: 7 }), route, {}), false);
  assert.equal(
    helper.projectTransactionCommandResultIsWellFormed(
      stageReceipt(route, { label: "Different" }),
      route,
      stageArgs(route),
    ),
    false,
    `${route} rejects a retained label that does not equal the normalized request label`,
  );
}
// A malformed non-null Stage receipt stays indeterminate/hold, never Cancel/replay.
{
  const malformedStage = { ...validStageReceiptByRoute.get("apply_stage_map_preset"), future_field: true };
  const decision = helper.publishedCommandRecoveryDecision(
    { status: "pending", command_result: malformedStage, command_in_flight: false },
    "apply_stage_map_preset",
    {},
  );
  assert.equal(decision.kind, "indeterminate", "a malformed Stage receipt may be post-engine and cannot Cancel");
  assert.equal(helper.publishedCommandRecoveryDisposition(decision), "hold");
}
// Reply-loss dispositions hold for Stage routes on the same lane.
assert.equal(
  helper.publishedCommandRecoveryDisposition(
    helper.publishedCommandRecoveryDecision({ status: "pending", command_result: null, command_in_flight: true }, "add_stage_object", {}),
  ),
  "hold",
  "a slow in-flight Stage command never reaches Cancel or replay",
);
assert.deepEqual(
  helper.publishedCommandRecoveryDecision({ status: "cancelled" }, "add_stage_object", {}),
  { kind: "not_published" },
);

const busy = [];
const lane = helper.createPatchRepairSingleflight((next) => busy.push(next));
assert.equal(lane.begin(), true, "first PATCH/Repair action acquires the shared lane");
assert.equal(lane.begin(), false, "second action cannot start while terminal receipt is pending");
lane.finish();
assert.equal(lane.begin(), true, "lane releases only after terminal handling finishes");
lane.finish();
assert.deepEqual(busy, [true, false, true, false]);

assert.match(types, /ProjectTransactionPatchCommandResult[\s\S]*?kind: "patch_fixtures"[\s\S]*?request_digest: string[\s\S]*?fixture_ids: number\[\]/);
assert.match(types, /ProjectTransactionRepairFixtureProfileCommandResult[\s\S]*?kind: "repair_fixture_profile"[\s\S]*?request_digest: string[\s\S]*?fixture_id: number/);
assert.match(types, /ProjectTransactionStageCommandResult[\s\S]*?kind: "stage_project_mutation"[\s\S]*?command_name: string[\s\S]*?request_digest: string[\s\S]*?outcome: ProjectTransactionStageMutationOutcome[\s\S]*?stage_object_id: number \| null[\s\S]*?label: string \| null/);
assert.match(types, /ProjectTransactionCommandResult =[\s\S]*?\| ProjectTransactionStageCommandResult;/);
assert.match(types, /status: "pending"[\s\S]*?command_result\?: ProjectTransactionCommandResult \| null/);
assert.match(types, /status: "pending"[\s\S]*?command_in_flight: boolean/, "pending recovery wire requires an exact in-flight bit");
assert.match(types, /status: "pending"[\s\S]*?command_indeterminate_error: string \| null/, "pending recovery wire carries an explicit indeterminate fault");
assert.equal((app.match(/invokePatchFixtures\(/g) ?? []).length, 3, "all three PATCH callers must share one typed facade");
assert.equal((app.match(/invokeFixtureProfileRepair\(/g) ?? []).length, 1, "Repair must share its typed facade");
assert.doesNotMatch(app, /"patch_fixture"/, "the retired singular PATCH command must not remain in frontend transaction routing");
assert.doesNotMatch(app, /invoke<number\[\]>\("patch_fixtures"/, "raw PATCH ID arrays are forbidden");
assert.doesNotMatch(app, /tauriInvoke(?:<[^>]*>)?\("(?:patch_fixtures|repair_fixture_profile)"/, "PATCH/Repair may not bypass the transaction facade");
assert.match(app, /recoverPublishedProjectTransactionCommandResult[\s\S]*?waitForPublishedCommandRecovery[\s\S]*?queryProjectTransactionRecovery\(identity\)/);
assert.match(app, /ProjectTransactionPublicationUnconfirmedError[\s\S]*?ProjectTransactionPublicationIndeterminateError[\s\S]*?cancelOpenedProjectTransaction/);
assert.match(app, /publishedCommandRecoveryDisposition\(recovered\)[\s\S]*?disposition === "hold"[\s\S]*?ProjectTransactionPublicationIndeterminateError[\s\S]*?ProjectTransactionPublicationUnconfirmedError/, "unknown and malformed B outcomes skip Cancel/replay");
assert.match(app, /setPatchRepairRestartRequired\(true\)[\s\S]*?finishPatchRepairOperation = \(\) => \{\s*if \(!patchRepairRestartRequired\(\)\) patchRepairOperationLane\.finish\(\);?\s*\}/, "an indeterminate receipt retains the shared local lane until restart");
assert.equal((app.match(/retainPatchRepairIntentIfIndeterminate\(error\)/g) ?? []).length, 4, "all PATCH/Repair callers retain their ticket on an indeterminate reply");
assert.match(app, /window\.dispatchEvent[\s\S]*?await acknowledgeProjectTransaction\(transactionIdentity\)/, "one history result precedes its terminal ACK");
assert.match(form, /operationBusy: boolean[\s\S]*?disabled=\{props\.operationBusy \|\| !props\.armed \|\| props\.invalid\}/);
assert.match(browser, /operationBusy: boolean[\s\S]*?data-patch-fixture-repair[\s\S]*?\{props\.operationBusy \? "Applying…" : "Repair"\}/);

// ---- All nine Stage routes must be wired into the shared recovery lane. ----
const stageMutationCommandsSection = app.slice(
  app.indexOf("const projectMutationCommands = new Set(["),
  app.indexOf("const projectMutationLabel"),
);
assert.notEqual(stageMutationCommandsSection.length, 0, "the central mutation command set must exist");
for (const route of allStageRoutes) {
  assert.match(d2Source, new RegExp(`"${route}"`), `${route} must remain in the shared published-command recovery union`);
  assert.match(
    stageMutationCommandsSection,
    new RegExp(`"${route}"`),
    `${route} must remain a transactional mutation routed through the central facade`,
  );
}
assert.match(
  app,
  /isStageRendererTicketedCommand\(command\)\s*\?\s*command\s*:\s*null/,
  "all nine Stage commands must join the shared published-command recovery detection",
);
assert.match(
  app,
  /publishedCommand && !isStageRendererTicketedCommand\(publishedCommand\)\s*&& !projectTransactionCommandResultIsWellFormed\(replied, publishedCommand, commandArgs\)/,
  "normal Stage replies keep their legacy shapes while PATCH/Repair still validate their receipt",
);
assert.match(
  app,
  /result = publishedCommandLegacyReplyFromRecoveredResult\([\s\S]*?publishedCommand,[\s\S]*?recovered\.result,[\s\S]*?commandArgs,[\s\S]*?\) as T;/,
  "only a recovered Stage receipt is converted back to its legacy reply shape",
);

// ---- Begin must fence the full E/R/H authority in one ordered backend path. ----
// Use the unique next Rust symbol as the end boundary so similarly named
// checks in commit/undo/redo cannot satisfy this assertion by accident.
const beginProjectTransactionSection = (() => {
  const startMarker = "fn begin_project_transaction_for_window_label(";
  const endMarker = "fn commit_project_transaction(";
  const startIndex = backend.indexOf(startMarker);
  assert.notEqual(startIndex, -1, "Begin project transaction helper must exist in the backend");
  const endIndex = backend.indexOf(endMarker, startIndex + startMarker.length);
  assert.notEqual(endIndex, -1, "Begin project transaction checker must stop at the unique commit symbol");
  return backend.slice(startIndex, endIndex);
})();
const beginIndexOf = (needle, fromIndex = 0) => {
  const index = beginProjectTransactionSection.indexOf(needle, fromIndex);
  assert.notEqual(index, -1, `Begin project transaction helper must contain: ${needle}`);
  return index;
};
const epochIndex = beginIndexOf("ensure_project_epoch_matches(&coordinator, expected_epoch)?;");
const revisionNeedle = "if coordinator.revision != expected_revision";
const preReconcileRevisionIndex = beginIndexOf(revisionNeedle);
const postReconcileRevisionIndex = beginIndexOf(revisionNeedle, preReconcileRevisionIndex + revisionNeedle.length);
assert.equal(
  beginProjectTransactionSection.indexOf(revisionNeedle, postReconcileRevisionIndex + revisionNeedle.length),
  -1,
  "Begin must retain exactly one pre-reconcile and one post-reconcile revision fence",
);
const checkpointHashNeedle = "ensure_project_checkpoint_hash_matches(&coordinator, &expected_checkpoint_hash)?;";
const preReconcileHashIndex = beginIndexOf(checkpointHashNeedle);
const postReconcileHashIndex = beginIndexOf(checkpointHashNeedle, preReconcileHashIndex + checkpointHashNeedle.length);
assert.equal(
  beginProjectTransactionSection.indexOf(checkpointHashNeedle, postReconcileHashIndex + checkpointHashNeedle.length),
  -1,
  "Begin must retain exactly one pre-reconcile and one post-reconcile checkpoint-hash fence",
);
const reconcileIndex = beginIndexOf("reconcile_project_checkpoint_for_coordinator(state, &mut coordinator)?;");
const armIndex = beginIndexOf("arm_project_transaction_and_capture_baseline(&state.project_transaction_active, || {");
const reservedHashIndex = beginIndexOf("if before.hash != expected_checkpoint_hash {");
assert.ok(epochIndex < preReconcileRevisionIndex, "Begin must check epoch before the pre-reconcile revision fence");
assert.ok(preReconcileRevisionIndex < preReconcileHashIndex, "Begin must check E/R before the pre-reconcile H fence");
assert.ok(preReconcileHashIndex < reconcileIndex, "Begin must reject a stale H before reconciliation");
assert.ok(reconcileIndex < postReconcileRevisionIndex, "Begin must re-check revision after reconciliation");
assert.ok(postReconcileRevisionIndex < postReconcileHashIndex, "Begin must re-check H after the post-reconcile revision fence");
assert.ok(postReconcileHashIndex < armIndex, "Begin must arm only after the post-reconcile H fence");
assert.ok(armIndex < reservedHashIndex, "Begin must compare the reserved before.hash only after arming");

console.log("D2 PATCH/Repair transaction controller checks passed");
