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

function runGate(relativePath) {
  try {
    execFileSync(process.execPath, [path.join(repoRoot, relativePath)], {
      cwd: repoRoot,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  } catch (error) {
    throw new Error(`F1 focused gate failed: ${relativePath}\n${error.stdout ?? ""}${error.stderr ?? ""}`);
  }
}

function runSelfTest() {
  const source = "project_callback_epoch project_mapping_callback_epoch reserve_project_callback_epoch retire_after_project_control_slots_taken";
  assertNeedles(source, "fixture", ["project_callback_epoch", "reserve_project_callback_epoch"], "F1 parser fixture");
  assert.throws(
    () => assertNeedles(source, "fixture", ["unfenced_callback_send"], "F1 parser fixture"),
    /unfenced_callback_send/u,
  );
  console.log("F1 input-generation self-tests ok; 2 contract-presence cases");
}

async function run() {
  const main = await readText("app/src-tauri/src/main.rs");
  const learn = await readText("app/src/createControlInputController.ts");
  const dmx = await readText("crates/io/src/dmx_input.rs");

  assertNeedles(main, "app/src-tauri/src/main.rs", [
    "project_callback_epoch: Arc<AtomicU64>",
    "project_mapping_callback_epoch: Arc<AtomicU64>",
    "project_input_runtime_generation: state.project_callback_epoch.load(Ordering::Acquire)",
    "project_mapping_callback_epoch",
    "reserve_project_callback_epoch(&state.project_callback_epoch)?",
    "reserve_project_callback_epoch(&state.project_mapping_callback_epoch)?",
    "fn send_engine_command_if_callback_epoch",
    "fn send_external_control_command_if_callback_epoch",
    "fn project_input_installation_is_current",
    "fn retire_after_project_control_slots_taken",
    "callback_epoch_allows_send",
    "project_transaction_active",
    "callback_epoch_fences_constructor_callbacks_and_stale_installation",
    "installed_callback_gate_drops_constructor_events_and_never_waits_for_retirement",
    "project_control_retirement_joins_before_publish_and_releases_partial_takes",
    "callback_epoch_overflow_is_rejected_without_changing_the_generation",
  ], "F1 callback-generation boundary");
  assertNeedles(learn, "app/src/createControlInputController.ts", [
    "projectAuthorityLearnContinuation",
    "learnAuthorityIsCurrent",
    "Project changed while Learn was waiting",
    "Project changed or control mappings were not durably synchronized",
  ], "F1 Learn authority boundary");
  assertNeedles(dmx, "crates/io/src/dmx_input.rs", [
    "let mut last_packet_at: Option<Instant> = None",
    "Some((universe, data)) if universe == config.universe",
    "last_packet_at = Some(Instant::now())",
    "last.elapsed() >= Duration::from_millis(config.timeout_ms.max(100))",
    "SignalLost",
  ], "F1 DMX liveness boundary");

  for (const gate of [
    "app/scripts/check-project-transaction.mjs",
    "app/scripts/check-dvc-midi-shortcuts.mjs",
    "app/scripts/check-dvc-dmx-shortcuts.mjs",
    "app/scripts/check-dmx-show-setup.mjs",
    "app/scripts/check-dmx-addressing-helpers.mjs",
    "app/scripts/check-frontend-command-routing.mjs",
  ]) runGate(gate);
  console.log("F1 input generations ok; project/mapping epochs, constructor/install fences, Learn continuation, DMX liveness and retirement tests/contracts verified");
}

if (process.argv.includes("--self-test")) runSelfTest();
else await run();
