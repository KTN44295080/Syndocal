import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import ts from "typescript";

const loadTsModule = async (relativePath) => {
  const source = await readFile(new URL(relativePath, import.meta.url), "utf8");
  const output = ts.transpileModule(source, {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.ESNext },
    fileName: relativePath,
  });
  return import(`data:text/javascript;base64,${Buffer.from(output.outputText).toString("base64")}`);
};

const helper = await loadTsModule("../src/patchTransactionD2.ts");
const [app, form, browser, types] = await Promise.all([
  readFile(new URL("../src/App.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/PatchFixtureFormPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/components/PatchProfileBrowserPanel.tsx", import.meta.url), "utf8"),
  readFile(new URL("../src/types.ts", import.meta.url), "utf8"),
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

console.log("D2 PATCH/Repair transaction controller checks passed");
