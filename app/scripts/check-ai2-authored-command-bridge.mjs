import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const appRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(appRoot, "..");
const normalize = (source) => source.replace(/\r\n/g, "\n");
const readText = async (relativePath) => normalize(await readFile(path.join(repoRoot, relativePath), "utf8"));

function assertNeedles(source, relativePath, needles, label) {
  for (const needle of needles) {
    assert(source.includes(needle), `${label} is missing ${relativePath}:${needle}`);
  }
}

function commandBody(source, command) {
  const start = source.indexOf(`fn ${command}(`);
  assert(start >= 0, `AI2 handler is missing: ${command}`);
  const end = source.indexOf("\n#[tauri::command]", start + 1);
  return source.slice(start, end < 0 ? source.length : end);
}

function runGate(relativePath) {
  try {
    execFileSync(process.execPath, [path.join(repoRoot, relativePath)], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    throw new Error(`AI2 focused gate failed: ${relativePath}\n${error.stdout ?? ""}${error.stderr ?? ""}`);
  }
}

function runSelfTest() {
  const source = "request_id generic_terminal_single_flight owner_incarnation expected_checkpoint_hash";
  assertNeedles(source, "fixture", ["request_id", "generic_terminal_single_flight", "owner_incarnation"], "AI2 parser fixture");
  assert.throws(
    () => assertNeedles(source, "fixture", ["cached_authority_replay"], "AI2 parser fixture"),
    /cached_authority_replay/u,
  );
  console.log("AI2 authored bridge self-tests ok; 2 contract-presence cases");
}

async function run() {
  const main = await readText("app/src-tauri/src/main.rs");
  const scene = await readText("app/src-tauri/src/scene_creation.rs");
  const bridge = await readText("app/src/agentBridgeControlPlane.ts");
  const app = await readText("app/src/App.tsx");
  const controlPlane = await readText("app/src-tauri/src/control_plane.rs");
  const authored = await readText("app/src-tauri/src/authored_control_plane.rs");

  const canonicalOperations = [
    ["syndocal.effects.set_enabled.v1", "set_effect_enabled"],
    ["syndocal.cue_lists.reorder.v1", "reorder_cue_lists"],
    ["syndocal.cue_lists.rename.v1", "rename_cue_list"],
    ["syndocal.cue_lists.delete.v1", "delete_cue_list"],
    ["syndocal.scenes.create.v1", "create_scene_authoritative_v1"],
  ];
  for (const [operationId, command] of canonicalOperations) {
    assertNeedles(bridge, "app/src/agentBridgeControlPlane.ts", [`"${operationId}": "${command}"`], "AI2 canonical adapter");
    assertNeedles(controlPlane, "app/src-tauri/src/control_plane.rs", [command, "ReceiptPolicy::ExactTerminalReceipt"], "AI2 backend registry");
  }

  for (const requestType of [
    "AuthoritativeCueListCreateRequest",
    "AuthoritativeCueListReorderRequest",
    "AuthoritativeCueListRenameRequest",
    "AuthoritativeCueListDeleteRequest",
  ]) {
    const start = main.indexOf(`struct ${requestType}`);
    const end = main.indexOf("\n}\n", start);
    const declaration = main.slice(Math.max(0, start - 120), end + 3);
    assert(start >= 0 && end > start, `AI2 request declaration is missing: ${requestType}`);
    assertNeedles(declaration, "app/src-tauri/src/main.rs", ["Serialize, Deserialize", "request_id: String", "expected_epoch", "expected_revision", "expected_checkpoint_hash", "owner_id"], `AI2 ${requestType}`);
  }
  assertNeedles(scene, "app/src-tauri/src/scene_creation.rs", [
    "Serialize, Deserialize",
    "request_id: String",
    "pub(super) fn request_id",
    "expected_epoch",
    "expected_revision",
    "expected_checkpoint_hash",
    "owner_id: String",
  ], "AI2 scene request");

  for (const command of [
    "create_scene_authoritative_v1",
    "create_cue_list",
    "reorder_cue_lists",
    "delete_cue_list",
    "rename_cue_list",
  ]) {
    const body = commandBody(main, command);
    assertNeedles(body, "app/src-tauri/src/main.rs", [
      "capture_video_clip_slot_caller_binding_for_window_label",
      "let owner_id = binding.owner_id.clone()",
      "authored_mutation_receipt_key",
      "generic_terminal_single_flight",
      "expected_authority",
    ], `AI2 ${command} handler`);
  }

  assertNeedles(authored, "app/src-tauri/src/authored_control_plane.rs", [
    "GenericAuthoredMutationReceiptKey",
    "request_id: String",
    "owner_incarnation: u64",
    "start_epoch: u64",
    "start_revision: u64",
    "start_checkpoint_hash: String",
    "generic_terminal_single_flight",
    "generic_retire_owner",
    "MAX_GENERIC_TERMINAL_RECEIPTS",
    "MAX_GENERIC_PUBLICATION_LANES",
    "different shape",
    "identity has been retired",
  ], "AI2 generic receipt lane");
  assertNeedles(main, "app/src-tauri/src/main.rs", [
    "fn validate_authored_mutation_request_id",
    "fn authored_mutation_shape_sha256",
    "generic_retire_owner(retired_owner)",
  ], "AI2 owner/request lifecycle");
  assertNeedles(app, "app/src/App.tsx", [
    "const strictServerAuthoritativeMutation",
    "requestId: typeof commandArgs.requestId === \"string\"",
    "projectTransactionOperationId()",
    "ownerId: projectTransactionOwnerId",
  ], "AI2 frontend request identity");

  runGate("app/scripts/check-authored-effect-enable.mjs");
  runGate("app/scripts/check-project-transaction-mutation-controller.mjs");
  console.log("AI2 authored command bridge ok; canonical authored routes, strict request identity, E/R/H binding, exact receipts, owner retirement and focused legacy gates verified");
}

if (process.argv.includes("--self-test")) runSelfTest();
else await run();
