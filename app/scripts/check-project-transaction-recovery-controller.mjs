import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import ts from "typescript";

// Execute the production factory and its real pure dependencies, with no Tauri,
// browser, or copied recovery implementation.
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
  assert.equal(result.diagnostics?.filter((item) => item.category === ts.DiagnosticCategory.Error).length ?? 0, 0);
  const module = { exports: {} };
  cache.set(file, module.exports);
  const requireLocal = (specifier) => {
    assert.ok(specifier.startsWith("./"), `unexpected external dependency: ${specifier}`);
    return load(specifier.slice(2));
  };
  new Function("require", "module", "exports", result.outputText)(requireLocal, module, module.exports);
  return module.exports;
}
const { createProjectTransactionRecoveryController } = load("createProjectTransactionRecoveryController");
const recovery = load("projectTransactionRecovery");
const identity = Object.freeze({
  clientOperationId: "project-op:41:test", shapeFingerprint: "shape", commandName: "set_attribute",
  schemaVersion: 7, ownerId: "renderer:test",
});
const ticket = Object.freeze({
  transaction_id: 41, project_epoch: 12, project_revision: 18,
  project_checkpoint_hash: "a".repeat(64), client_operation_id: identity.clientOperationId,
  shape_fingerprint: identity.shapeFingerprint, schema_version: 7, owner_id: identity.ownerId,
  window_label: "main", owner_incarnation: 1, label: "Set Attribute", coalesce_key: "",
});
const history = {
  can_undo: true, can_redo: false, undo_depth: 1, redo_depth: 0,
  project_epoch: 12, project_revision: 19, checkpoint_hash: "b".repeat(64), history_generation: 2,
};
const mutation = {
  history_status: history,
  authority: {
    ...history, publication_generation: 2, publication_kind: "Mutation",
    mapping_replacement_generation: 0, authority_disposition_generation: 0,
    authority_disposition: "Clean", recovery_authority_serial: 0,
    recovery_authority_last_transition: { kind: "legacy_unknown" }, path_generation: 0,
    current_project_path: null, snapshot: {}, profiles: [], fixture_groups: [], operator_policy: null,
    midi_mappings: [], osc_mappings: [], dmx_mappings: [], dj_track_triggers: [],
    history, input_runtime: {},
  },
};
assert.ok(recovery.projectTransactionTerminalMutationIsWellFormed(mutation));
const terminal = (status) => ({ status, ticket, mutation });
const pending = (busy = false) => ({
  status: "pending", ticket, command_result: null, command_in_flight: busy, command_indeterminate_error: null,
});
const allowed = new Set([
  "begin_project_transaction", "query_project_transaction", "adopt_project_transaction",
  "commit_project_transaction", "cancel_project_transaction", "acknowledge_project_transaction",
]);
function harness(handle) {
  const events = [];
  const messages = [];
  const waits = [];
  const controller = createProjectTransactionRecoveryController({
    invoke: async (command, args) => {
      assert.ok(allowed.has(command), `raw mutation must never be replayed: ${command}`);
      events.push({ command, args });
      return handle(command, args);
    },
    dispatchHistoryMutation: (value) => {
      assert.strictEqual(value, mutation);
      events.push({ command: "history" });
    },
    reportRecovery: (message) => messages.push(message),
    wait: async (milliseconds) => { waits.push(milliseconds); },
  });
  return { controller, events, messages, waits };
}
const commands = (h) => h.events.map((event) => event.command);
let scenarios = 0;

// Lost Begin adopts the exact receipt; no new Begin or project command is sent.
{
  const beginArgs = { label: "Set Attribute", ...identity };
  const h = harness((command, args) => {
    if (command === "begin_project_transaction") throw new Error("lost Begin reply");
    assert.strictEqual(args, identity);
    if (command === "query_project_transaction" || command === "adopt_project_transaction") return pending();
    throw new Error(command);
  });
  assert.strictEqual(await h.controller.beginProjectTransactionWithRecovery(beginArgs, identity), ticket);
  assert.deepEqual(commands(h), ["begin_project_transaction", "query_project_transaction", "adopt_project_transaction"]);
  scenarios += 1;
}

// Commit and Cancel reply loss query their original terminal receipt and deliver
// history before ACK. Reusing a settlement never redelivers or repeats success ACK.
for (const action of ["commit", "cancel"]) {
  const h = harness((command, args) => {
    if (command === `${action}_project_transaction`) {
      assert.deepEqual(args, recovery.projectTransactionTerminalArgs(ticket, identity));
      throw new Error("lost terminal reply");
    }
    assert.strictEqual(args, identity);
    if (command === "query_project_transaction") return terminal(action === "commit" ? "committed" : "cancelled");
    if (command === "acknowledge_project_transaction") return undefined;
    throw new Error(command);
  });
  const settle = h.controller.createAppProjectTransactionTerminalSettlement(identity);
  const result = action === "commit"
    ? await h.controller.commitProjectTransactionWithRecovery(ticket, identity, settle)
    : await h.controller.cancelProjectTransactionWithRecovery(ticket, identity);
  assert.strictEqual(result, mutation);
  await h.controller.resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation();
  if (action === "commit") await settle(mutation);
  assert.deepEqual(commands(h), [`${action}_project_transaction`, "query_project_transaction", "history", "acknowledge_project_transaction"]);
  scenarios += 1;
}

// A bounded ACK failure retains acknowledgement only. Another controller has no
// pending receipt. Resuming the first cannot resend Commit or redeliver history.
{
  let ackAvailable = false;
  const h = harness((command) => {
    if (command === "commit_project_transaction") return mutation;
    if (command === "acknowledge_project_transaction") {
      if (!ackAvailable) throw new Error("ACK offline");
      return undefined;
    }
    throw new Error(command);
  });
  const settle = h.controller.createAppProjectTransactionTerminalSettlement(identity);
  await assert.rejects(h.controller.commitProjectTransactionWithRecovery(ticket, identity, settle), recovery.ProjectTransactionTerminalAcknowledgementUnresolvedError);
  assert.equal(commands(h).filter((item) => item === "history").length, 1);
  assert.equal(commands(h).filter((item) => item === "acknowledge_project_transaction").length, 6);
  assert.equal(h.waits.length, 5);
  const other = harness(() => { throw new Error("other controller must be idle"); });
  await other.controller.resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation();
  assert.equal(other.events.length, 0);
  ackAvailable = true;
  await h.controller.resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation();
  const afterResume = h.events.length;
  await h.controller.resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation();
  assert.equal(h.events.length, afterResume);
  assert.equal(commands(h).filter((item) => item === "history").length, 1);
  assert.equal(commands(h).filter((item) => item === "commit_project_transaction").length, 1);
  assert.equal(commands(h).filter((item) => item === "acknowledge_project_transaction").length, 7);
  assert.ok(h.messages.some((message) => message.includes("Retry a project mutation")));
  scenarios += 1;
}

// Busy terminal recovery is retained across its bounded batch and resumes only
// the exact ticket after the backend reports idle.
{
  let busy = true;
  let terminalCalls = 0;
  const h = harness((command, args) => {
    if (command === "cancel_project_transaction") {
      terminalCalls += 1;
      assert.deepEqual(args, recovery.projectTransactionTerminalArgs(ticket, identity));
      if (terminalCalls === 1) throw new Error("lost Cancel reply");
      return mutation;
    }
    if (command === "query_project_transaction") {
      assert.strictEqual(args, identity);
      return pending(busy);
    }
    if (command === "acknowledge_project_transaction") return undefined;
    throw new Error(command);
  });
  await assert.rejects(h.controller.cancelProjectTransactionWithRecovery(ticket, identity), recovery.ProjectTransactionTerminalRecoveryUnresolvedError);
  assert.equal(terminalCalls, 1);
  assert.equal(commands(h).filter((item) => item === "history").length, 0);
  busy = false;
  await h.controller.resumeForegroundProjectTransactionTerminalRecoveryBeforeMutation();
  assert.equal(terminalCalls, 2);
  assert.deepEqual(commands(h).slice(-3), ["cancel_project_transaction", "history", "acknowledge_project_transaction"]);
  scenarios += 1;
}

// Lost publication query remains query-only until durable non-publication, never a replay.
{
  let queries = 0;
  const h = harness((command, args) => {
    assert.equal(command, "query_project_transaction");
    assert.strictEqual(args, identity);
    return ++queries === 1 ? null : pending(false);
  });
  const result = await h.controller.recoverPublishedProjectTransactionCommandResult(identity, "patch_fixtures", {});
  assert.equal(result.kind, "not_published");
  assert.equal(h.events.length, 2);
  scenarios += 1;
}

console.log(`Project transaction recovery controller: ${scenarios} scenarios passed; no native/UI side effects.`);
