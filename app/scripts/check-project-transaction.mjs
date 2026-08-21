import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import path from "node:path";

const scriptDirectory = path.dirname(fileURLToPath(import.meta.url));
const readWorkspaceFile = (relativePath) => readFile(
  path.resolve(scriptDirectory, "..", "..", relativePath),
  "utf8",
);

const [rust, app, transactionModule, manifestText, invokeCommands] = await Promise.all([
  readWorkspaceFile("app/src-tauri/src/main.rs"),
  readWorkspaceFile("app/src/App.tsx"),
  readWorkspaceFile("app/src/types.ts"),
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
  "frontend Cancel must query terminal state after a lost reply",
);
assert.match(
  app,
  /commitProjectTransactionWithRecovery\(/,
  "frontend Commit must query terminal state after a lost reply",
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

console.log("project transaction executable production-contract checks passed");
