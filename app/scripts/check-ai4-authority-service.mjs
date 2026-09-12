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
    throw new Error(`AI4 focused gate failed: ${relativePath}\n${error.stdout ?? ""}${error.stderr ?? ""}`);
  }
}

function runSelfTest() {
  const source = "AgentAuthorityService begin_pairing CredWriteW kill_switch authorize_with_consent";
  assertNeedles(source, "fixture", ["begin_pairing", "CredWriteW", "kill_switch"], "AI4 parser fixture");
  assert.throws(
    () => assertNeedles(source, "fixture", ["cached_authority_replay"], "AI4 parser fixture"),
    /cached_authority_replay/u,
  );
  console.log("AI4 authority service self-tests ok; 2 contract-presence cases");
}

async function run() {
  const authority = await readText("app/src-tauri/src/agent_authority_service.rs");
  const bridge = await readText("app/src-tauri/src/agent_bridge.rs");
  const main = await readText("app/src-tauri/src/main.rs");
  const protocol = await readText("crates/protocol/src/agent_authority.rs");

  assertNeedles(authority, "app/src-tauri/src/agent_authority_service.rs", [
    "pub(crate) struct AgentAuthorityService",
    "const MAX_CHALLENGES: usize = 32",
    "const PAIRING_TTL: Duration = Duration::from_secs(60)",
    "begin_pairing",
    "approve_pairing",
    "authenticate",
    "promote",
    "grant",
    "revoke",
    "kill_switch",
    "clear_kill_switch",
    "prepare_consent",
    "authorize_with_consent",
    "CredWriteW",
    "CredReadW",
    "CredDeleteW",
    "CRED_PERSIST_LOCAL_MACHINE",
    "constant_time_bytes_eq",
    "agent_credential_store_read_failed",
  ], "AI4 authority service");
  assert(!authority.includes("Raw Input"), "AI4 service must not restore the retired Raw Input challenge");
  assert(!authority.includes("six-digit"), "AI4 service must not restore the retired six-digit challenge");

  assertNeedles(bridge, "app/src-tauri/src/agent_bridge.rs", [
    "mod authority",
    "authority: authority::AgentAuthorityService",
    "AgentAuthorityService::new()",
    "pub(crate) fn authority",
    "main_only(window_label)",
    "not exposed to the socket adapter",
  ], "AI4 Agent Bridge boundary");

  for (const command of [
    "agent_authority_status_v1",
    "agent_authority_begin_pairing_v1",
    "agent_authority_approve_pairing_v1",
    "agent_authority_authenticate_v1",
    "agent_authority_promote_v1",
    "agent_authority_grant_v1",
    "agent_authority_revoke_v1",
    "agent_authority_kill_switch_v1",
    "agent_authority_clear_kill_switch_v1",
    "agent_authority_prepare_consent_v1",
    "agent_authority_authorize_with_consent_v1",
  ]) {
    assertNeedles(main, "app/src-tauri/src/main.rs", [`fn ${command}(`, command], `AI4 Tauri command ${command}`);
  }
  assertNeedles(main, "app/src-tauri/src/main.rs", [
    "bridge.authority(window.label())?",
    "agent_authority_status_v1,",
    "agent_authority_authorize_with_consent_v1,",
  ], "AI4 Tauri command admission");

  assertNeedles(protocol, "crates/protocol/src/agent_authority.rs", [
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct AgentRequestContext",
    "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\npub struct AgentAuthorization",
    "pub const MAX_PREPARED_CONSENT_TTL_MS: u64 = 15_000",
  ], "AI4 protocol consent payload");

  runGate("app/scripts/check-agent-bridge.mjs");
  runGate("app/scripts/check-tauri-admission-inventory.mjs");
  console.log("AI4 authority service ok; bounded pairing, OS credential storage, safe-mode grants, revocation, kill switch, exact consent binding, trusted-window admission and retired challenge absence verified");
}

if (process.argv.includes("--self-test")) runSelfTest();
else await run();
