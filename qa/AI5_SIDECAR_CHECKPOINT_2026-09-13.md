# AI5 sidecar checkpoint — 2026-09-13

## Scope

This checkpoint closes `AI5-SIDECAR-001` for the current-source supported
software boundary: authenticated localhost sidecar admission, MCP stdio and
streamable HTTP, JSON-RPC/REST/WebSocket transport adapters, bounded messages
and sessions, authenticated discovery, process/executable binding, and
no-retry/no-cached-authority behavior.

## Evidence

| Gate | Result |
| --- | --- |
| `node app/scripts/check-ai5-sidecar.mjs` | PASS — authenticated discovery, nonce proof, exact grant admission, bounded forwarding and redaction |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback broker only |
| `node tools/syndocal-mcp/check-transports.mjs` | PASS — HTTP health/JSON-RPC/REST/WebSocket groups; fake loopback broker only |
| `node app/scripts/check-agent-bridge.mjs` | PASS — 11 bridge groups |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — deferred lifecycle boundaries remain explicit |
| release static/self-test chains | PASS — AI5 checker included in both chains |

The sidecar reads a verified descriptor and OS-protected credential, binds
proofs to launch nonce, client nonce, principal, request, and method, requires
authentication before discovery/forwarding, and forwards through the native
bridge's exact ExternalMcp grant and ledger admission. The HTTP/REST and
WebSocket adapters share the same JSON-RPC dispatcher; input, response, HTTP
session, HTTP connection, and WebSocket connection bounds are enforced before
forwarding. Unknown or lost mutation replies return a query-first disposition;
the sidecar performs no automatic mutation retry.

## Non-claims

This is not release-native external-client acceptance. It does not claim a
real MCP/JSON-RPC client, clean install, crash/restart during an active show,
physical devices/output, venue operation, public LAN exposure, or AI8 security
sign-off. The fake loopback broker tests are deterministic adapter evidence,
not evidence that a deployed external client has been accepted.
