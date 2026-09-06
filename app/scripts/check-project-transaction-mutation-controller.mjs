import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

// Execute the production controller and its real pure dependencies. The
// injected ports below are transport/terminal doubles; no Tauri or browser
// runtime is imported by this checker.
const sourceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../src");
const cache = new Map();
function load(name) {
  const file = path.resolve(sourceRoot, `${name}.ts`);
  if (cache.has(file)) return cache.get(file);
  const result = ts.transpileModule(readFileSync(file, "utf8"), {
    compilerOptions: { target: ts.ScriptTarget.ES2022, module: ts.ModuleKind.CommonJS },
    fileName: file,
    reportDiagnostics: true,
  });
  assert.equal(
    result.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length ?? 0,
    0,
  );
  const module = { exports: {} };
  cache.set(file, module.exports);
  const requireLocal = (specifier) => {
    assert.ok(specifier.startsWith("./"), `unexpected external dependency: ${specifier}`);
    return load(specifier.slice(2));
  };
  new Function("require", "module", "exports", result.outputText)(
    requireLocal,
    module,
    module.exports,
  );
  return module.exports;
}

const mutationController = load("projectTransactionMutationController");
const recovery = load("projectTransactionRecovery");
const identity = Object.freeze({
  clientOperationId: "project-op:41:test",
  shapeFingerprint: "shape",
  commandName: "set_fixture_transforms",
  schemaVersion: 1,
  ownerId: "renderer:test",
});
const ticket = Object.freeze({
  transaction_id: 41,
  project_epoch: 12,
  project_revision: 18,
  project_checkpoint_hash: "a".repeat(64),
  client_operation_id: identity.clientOperationId,
  shape_fingerprint: identity.shapeFingerprint,
  schema_version: identity.schemaVersion,
  owner_id: identity.ownerId,
  window_label: "main",
  owner_incarnation: 1,
  label: "Set Fixture Transforms",
  coalesce_key: "",
});
const mutation = Object.freeze({});
const stageReceipt = Object.freeze({
  kind: "stage_project_mutation",
  command_name: "set_fixture_transforms",
  request_digest: "b".repeat(64),
  outcome: "applied",
  stage_object_id: null,
  label: null,
});

const createHarness = ({ invoke, recover = async () => {
  throw new Error("published recovery was not expected");
} } = {}) => {
  const events = [];
  let cancelCount = 0;
  let commitCount = 0;
  let settlementCount = 0;
  let recoveryCount = 0;
  const controller = mutationController.createProjectTransactionMutationController({
    invoke: async (command, args) => {
      events.push({ kind: "invoke", command, args });
      return invoke ? invoke(command, args) : undefined;
    },
    ownerId: identity.ownerId,
    cancelProjectTransactionWithRecovery: async (receivedTicket, receivedIdentity) => {
      cancelCount += 1;
      events.push({ kind: "cancel", ticket: receivedTicket, identity: receivedIdentity });
      return null;
    },
    commitProjectTransactionWithRecovery: async (receivedTicket, receivedIdentity, settle) => {
      commitCount += 1;
      events.push({ kind: "commit", ticket: receivedTicket, identity: receivedIdentity });
      await settle(mutation);
      return mutation;
    },
    createAppProjectTransactionTerminalSettlement: () => async () => {
      settlementCount += 1;
      events.push({ kind: "settle" });
    },
    recoverPublishedProjectTransactionCommandResult: async (receivedIdentity, command, args) => {
      recoveryCount += 1;
      events.push({ kind: "recover", identity: receivedIdentity, command, args });
      return recover(receivedIdentity, command, args);
    },
  });
  return {
    controller,
    events,
    get cancelCount() { return cancelCount; },
    get commitCount() { return commitCount; },
    get settlementCount() { return settlementCount; },
    get recoveryCount() { return recoveryCount; },
  };
};

const execute = (harness, command, commandArgs, options = {}) =>
  harness.controller.executeProjectTransactionMutation({
    command,
    commandArgs,
    transaction: ticket,
    identity,
    onOpened: options.onOpened ?? null,
    shouldAbort: options.shouldAbort ?? null,
    awaitAbort: options.awaitAbort ?? null,
  });

let scenarios = 0;

// 1. A strict Stage route keeps one nested ticket envelope and commits once.
{
  const h = createHarness({
    invoke: async (command, args) => {
      assert.equal(command, "set_fixture_transforms");
      assert.deepEqual(args, {
        request: {
          transforms: [{ fixtureId: 7, rotation: { pitch: 0, yaw: 20, roll: 0 } }],
          projectTransactionId: 41,
          expectedEpoch: 12,
          ownerId: identity.ownerId,
        },
      });
      return undefined;
    },
  });
  let opened = 0;
  const result = await execute(
    h,
    "set_fixture_transforms",
    { transforms: [{ fixtureId: 7, rotation: { pitch: 0, yaw: 20, roll: 0 } }] },
    { onOpened: () => { opened += 1; } },
  );
  assert.equal(result, undefined);
  assert.equal(opened, 1);
  assert.equal(h.commitCount, 1);
  assert.equal(h.settlementCount, 1);
  assert.equal(h.cancelCount, 0);
  assert.equal(h.recoveryCount, 0);
  scenarios += 1;
}

// 2. A non-published command error keeps the original error and cancels once.
{
  const commandError = new Error("raw command failed");
  const h = createHarness({ invoke: async () => { throw commandError; } });
  await assert.rejects(
    execute(h, "set_attribute", { fixtureId: 7, attribute: "Dimmer", value: 1 }),
    (error) => error === commandError,
  );
  assert.equal(h.cancelCount, 1);
  assert.equal(h.commitCount, 0);
  assert.equal(h.recoveryCount, 0);
  scenarios += 1;
}

// 3. A published command that is definitively not published still cancels
// once and never replays the raw command.
{
  const commandError = new Error("lost Stage reply");
  const h = createHarness({
    invoke: async () => { throw commandError; },
    recover: async (_receivedIdentity, command, args) => {
      assert.equal(command, "set_fixture_transforms");
      assert.deepEqual(args, { transforms: [] });
      return { kind: "not_published" };
    },
  });
  await assert.rejects(
    execute(h, "set_fixture_transforms", { transforms: [] }),
    (error) => error === commandError,
  );
  assert.equal(h.events.filter((event) => event.kind === "invoke").length, 1);
  assert.equal(h.recoveryCount, 1);
  assert.equal(h.cancelCount, 1);
  assert.equal(h.commitCount, 0);
  scenarios += 1;
}

// 4. A recovered published Stage receipt commits without Cancel and restores
// the route's legacy unit reply shape.
{
  const h = createHarness({
    invoke: async () => { throw new Error("lost published reply"); },
    recover: async (_receivedIdentity, command) => {
      assert.equal(command, "set_fixture_transforms");
      return { kind: "published", result: stageReceipt };
    },
  });
  const result = await execute(h, "set_fixture_transforms", { transforms: [] });
  assert.equal(result, null);
  assert.equal(h.events.filter((event) => event.kind === "invoke").length, 1);
  assert.equal(h.recoveryCount, 1);
  assert.equal(h.commitCount, 1);
  assert.equal(h.settlementCount, 1);
  assert.equal(h.cancelCount, 0);
  scenarios += 1;
}

// 5. Indeterminate and unconfirmed publication outcomes hold the ticket and
// never invoke Cancel or Commit.
for (const recovered of [
  { kind: "indeterminate", error: "backend publication fault" },
  { kind: "unconfirmed" },
]) {
  const h = createHarness({
    invoke: async () => { throw new Error("lost published reply"); },
    recover: async () => recovered,
  });
  await assert.rejects(
    execute(h, "set_fixture_transforms", { transforms: [] }),
    (error) => recovered.kind === "indeterminate"
      ? error instanceof mutationController.ProjectTransactionPublicationIndeterminateError
        && error.message.includes(recovered.error)
      : error instanceof mutationController.ProjectTransactionPublicationUnconfirmedError,
  );
  assert.equal(h.recoveryCount, 1);
  assert.equal(h.cancelCount, 0);
  assert.equal(h.commitCount, 0);
}
scenarios += 1;

// 6. Abort after Begin cancels the exact ticket, awaits the optional media
// abort, and sends no raw mutation or Opened callback.
{
  const h = createHarness({ invoke: async () => {
    throw new Error("raw mutation must not run after abort");
  } });
  let opened = 0;
  let mediaAbort = 0;
  await assert.rejects(
    execute(h, "set_fixture_transforms", { transforms: [] }, {
      onOpened: () => { opened += 1; },
      shouldAbort: () => true,
      awaitAbort: async () => { mediaAbort += 1; },
    }),
    (error) => error instanceof DOMException && error.name === "AbortError",
  );
  assert.equal(opened, 0);
  assert.equal(mediaAbort, 1);
  assert.equal(h.events.filter((event) => event.kind === "invoke").length, 0);
  assert.equal(h.cancelCount, 1);
  assert.equal(h.commitCount, 0);
  scenarios += 1;
}

console.log(`Project transaction mutation controller: ${scenarios} scenarios passed; no native/UI side effects.`);
