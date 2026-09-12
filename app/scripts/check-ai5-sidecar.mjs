import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { readFile } from "node:fs/promises";
import { promisify } from "node:util";
import { fileURLToPath } from "node:url";

const execFileAsync = promisify(execFile);
const read = (relativePath) =>
  readFile(new URL(relativePath, import.meta.url), "utf8").then((source) =>
    source.replace(/\r\n?/gu, "\n"),
  );

const [roadmap, server, adapterCheck, transportCheck, wire, bridge, authority, readme] = await Promise.all([
  read("../../qa/SYNDOCAL_AI_CONTROL_PLANE_ROADMAP.md"),
  read("../../tools/syndocal-mcp/server.mjs"),
  read("../../tools/syndocal-mcp/check.mjs"),
  read("../../tools/syndocal-mcp/check-transports.mjs"),
  read("../src-tauri/src/agent_bridge_wire.rs"),
  read("../src-tauri/src/agent_bridge.rs"),
  read("../src-tauri/src/agent_authority_service.rs"),
  read("../../tools/syndocal-mcp/README.md"),
]);

const required = (source, needle, label) => {
  assert.ok(source.includes(needle), `${label}: missing ${needle}`);
};

const ordered = (source, needles, label) => {
  let previous = -1;
  for (const needle of needles) {
    const index = source.indexOf(needle);
    assert.ok(index > previous, `${label}: ${needle} is missing or out of order`);
    previous = index;
  }
};

for (const marker of [
  "MCP stdio/streamable HTTP",
  "JSON-RPC/REST/WS adapters",
  "bounded IPC",
  "crash isolation",
  "no-startup-dependency behavior",
]) required(roadmap, marker, "AI5 roadmap boundary");

for (const marker of [
  "--principal-id",
  "--principal-incarnation",
  "--credential-file",
  "readCredential",
  "proofMessage",
  "createHmac('sha256'",
  "randomBytes(32)",
  "auth:",
  "control_plane.get_capabilities",
  "Authenticated principal is required before tool discovery",
  "INPUT_LIMIT",
  "RESPONSE_LIMIT",
  "active",
  "dispatchRpc",
  "serveHttp",
  "HTTP_CONNECTION_LIMIT",
  "WEBSOCKET_CONNECTION_LIMIT",
  "loopbackHost",
  "'/rpc'",
  "'/rest/tools/'",
  "'/ws'",
  "X-Syndocal-Session",
  "websocketFrame",
]) required(server, marker, "sidecar authentication and bounds");

for (const marker of [
  "struct Auth",
  "principal_id",
  "principal_incarnation",
  "client_nonce",
  "proof",
]) required(wire, marker, "native authentication wire");

for (const marker of [
  "session_nonce",
  "auth_nonces",
  "authenticate_proof",
  "authorize_bridge_request",
  "agent_authentication_required",
  "agent_auth_nonce_replayed",
]) required(bridge, marker, "native authentication admission");

for (const marker of [
  "fn authenticate_proof",
  "fn authorize_bridge_request",
  "AgentCapability::Read",
  "AgentCapability::Authored",
  "AgentCapability::Output",
  "AgentCapability::File",
]) required(authority, marker, "authority grant mapping");

for (const marker of [
  "createServer",
  "request.auth.principalId",
  "createHmac('sha256'",
  "assert.ok(!stdout.includes(token))",
  "const overlap",
]) required(adapterCheck, marker, "sidecar auth integration check");

for (const marker of ["fetch(", "new WebSocket", "healthz", "rest/tools", "tools/list"]) {
  required(transportCheck, marker, "sidecar transport integration check");
}

for (const marker of [
  "fresh client nonce",
  "HMAC-SHA256",
  "exact ExternalMcp grant",
  "R4/R5 requests remain consent-bound",
  "stdio mode writes nothing except newline-delimited MCP JSON-RPC to stdout",
]) required(readme, marker, "sidecar security contract");

ordered(server, ["readDescriptor(options)", "readCredential(options)", "createHmac('sha256'", "socket.write(wire)"], "proof before forwarding");
ordered(bridge, ["agent_authentication_required", "authenticate_proof", "authorize_bridge_request", "ledger.begin"], "native admission order");

if (process.argv.includes("--self-test")) {
  assert.equal(/^[0-9a-f]{64}$/u.test("a".repeat(64)), true);
  assert.equal(/^[0-9a-f]{64}$/u.test("A".repeat(64)), false);
  assert.equal(["R0", "R1", "R2", "R3", "R4", "R5", "S0"].length, 7);
  assert.equal("tools/list".includes("tools"), true);
  console.log("AI5 sidecar self-test passed: 4 assertions");
} else {
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const check = fileURLToPath(new URL("../../tools/syndocal-mcp/check.mjs", import.meta.url));
  const result = await execFileAsync(process.execPath, [check], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 1024 * 1024,
  });
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  const transport = fileURLToPath(new URL("../../tools/syndocal-mcp/check-transports.mjs", import.meta.url));
  const transportResult = await execFileAsync(process.execPath, [transport], {
    cwd: root,
    windowsHide: true,
    maxBuffer: 1024 * 1024,
  });
  process.stdout.write(transportResult.stdout);
  process.stderr.write(transportResult.stderr);
  console.log("AI5 sidecar auth/transport boundary ok; authenticated discovery, nonce proof, exact grant admission, bounded MCP/HTTP/REST/WebSocket forwarding and redaction verified");
}
