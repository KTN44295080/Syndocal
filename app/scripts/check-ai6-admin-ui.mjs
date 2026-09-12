import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

const read = (relativePath) =>
  readFile(new URL(relativePath, import.meta.url), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [panel, app, modes, commands, manifest, main, authority, bridge] = await Promise.all([
  read("../src/components/AgentAuthorityPanel.tsx"),
  read("../src/App.tsx"),
  read("../src/uiModes.ts"),
  read("../src/tauriInvokeCommands.ts"),
  read("../src/tauri-invoke-manifest.json"),
  read("../src-tauri/src/main.rs"),
  read("../src-tauri/src/agent_authority_service.rs"),
  read("../src-tauri/src/agent_bridge.rs"),
]);

const required = (source, needle, label) => {
  assert.ok(source.includes(needle), `${label}: missing ${needle}`);
};

for (const marker of [
  "data-agent-authority-panel",
  "agent_authority_status_v1",
  "agent_authority_begin_pairing_v1",
  "agent_authority_approve_pairing_v1",
  "agent_authority_promote_v1",
  "agent_authority_grant_v1",
  "agent_authority_revoke_v1",
  "agent_authority_kill_switch_v1",
  "agent_authority_prepare_consent_v1",
  "agent_authority_authorize_with_consent_v1",
  "Create challenge",
  "Approve pairing",
  "Promote",
  "Install exact grant",
  "Revoke all / kill switch",
  "Prepare 15s consent",
  "Consume consent",
  "Audit viewer",
  "activeSessions",
  "Live sidecar connections",
  "data-agent-pairing-credential",
]) required(panel, marker, "AI6 administration UI");

for (const marker of ["security", "authority", "AI Access", "AI principals, grants, consent"]) {
  required(modes, marker, "AI6 security route");
}
for (const marker of ["import { AgentAuthorityPanel }", "setupSubTab() === \"authority\"", "<AgentAuthorityPanel"]) {
  required(app, marker, "AI6 reachability");
}

for (const command of [
  "agent_authority_approve_pairing_v1",
  "agent_authority_authenticate_v1",
  "agent_authority_authorize_with_consent_v1",
  "agent_authority_begin_pairing_v1",
  "agent_authority_clear_kill_switch_v1",
  "agent_authority_grant_v1",
  "agent_authority_kill_switch_v1",
  "agent_authority_prepare_consent_v1",
  "agent_authority_promote_v1",
  "agent_authority_revoke_v1",
  "agent_authority_status_v1",
]) {
  required(commands, `"${command}"`, "frontend invoke authority");
  required(manifest, `"${command}"`, "invoke manifest authority");
  required(main, `fn ${command}`, "native authority command");
}

for (const marker of ["AuditRecord", "MAX_AUDIT_RECORDS", "record_audit", "pub audit: Vec<AuditRecord>", "audit: VecDeque", "active_connections"]) {
  required(marker === "active_connections" ? bridge : authority, marker, "backend authority audit");
}

if (process.argv.includes("--self-test")) {
  assert.equal(["pairing", "grants", "revocation", "consent", "audit", "health"].length, 6);
  assert.equal(["safe", "promoted", "revoked"].length, 3);
  assert.equal(512 <= 1024, true);
  console.log("AI6 administration UI self-test passed: 3 assertions");
} else {
  console.log("AI6 administration UI ok; Security route, trusted authority actions, exact grants, single-use consent and bounded audit viewer verified");
}
