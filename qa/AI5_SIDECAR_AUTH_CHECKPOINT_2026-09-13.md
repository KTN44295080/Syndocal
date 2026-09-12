# AI5 sidecar authentication checkpoint — 2026-09-13

Branch: `codex/showclock-review-20260912`

This checkpoint closes the AI5 authentication/admission sub-unit. The AI5
completion marker remains Open because the selected sidecar still needs the
remaining HTTP/REST/JSON-RPC/WebSocket transport surface, bounded lifecycle
management, and release-native/external acceptance.

## Implemented

- Each native bridge launch publishes a fresh 32-byte session nonce in the
  descriptor. The MCP adapter reads the descriptor and a file-backed credential,
  generates a fresh client nonce for every request, and sends an HMAC-SHA256
  proof bound to session nonce, client nonce, request UUID, and method.
- The native bridge rejects missing authentication, invalid principal or
  incarnation, malformed or stale proofs, and replayed client nonces before
  ledger admission or renderer dispatch. The nonce replay set is bounded to
  512 entries and is scoped to the current launch session.
- Backend authority performs exact ExternalMcp grant admission for bridge
  methods. Read queries map to R0; authored transforms map to R3; output and
  canonical control-plane operations map to their declared risk. R4/R5 remain
  backend consent-bound and are not self-approved by the sidecar.
- MCP tool discovery performs an authenticated read probe before returning the
  tool registry. Credentials and bridge tokens are redacted from broker data.

## Evidence

- `node tools/syndocal-mcp/check.mjs`: 15 adapter integration groups passed;
  fake loopback broker only, no Syndocal or device calls.
- `node app/scripts/check-agent-bridge.mjs`: 11 existing bridge groups passed.
- `node app/scripts/check-ai4-authority-service.mjs`: passed.
- `node app/scripts/check-ai5-sidecar.mjs`: source/admission checks and the
  adapter integration check passed.
- Pinned MSVC release test:
  `agent_bridge::tests::agent_bridge_process_requires_nonce_proof_and_exact_external_grant`
  is the current native Rust gate. The first run exposed only test cleanup
  (`remove_dir` left the owned lock file); the test now removes its own
  temporary directory recursively.

## Transport sub-unit

- The canonical JSON-RPC dispatcher is shared by stdio and a loopback-only
  streamable HTTP listener. `GET /healthz` is the only unauthenticated probe;
  `POST /rpc` requires a bounded session header and `POST /rest/tools/<name>`
  is a facade over the same typed MCP tool call. `/ws` carries the same JSON-RPC
  dispatcher with masked text frames, bounded payloads, ping/pong, close, and a
  connection cap.
- `node tools/syndocal-mcp/check-transports.mjs` passes health, HTTP JSON-RPC,
  REST, WebSocket, and the same nonce-proof checks against a fake loopback
  broker.

## Remaining AI5 boundary

The marker is not promoted by this document. Full per-client bounded lifecycle
and queue/overload semantics, sidecar crash/restart proof, OS-protected
credential transfer, and release executable plus real external-client
acceptance remain open. No device, venue, or production acceptance is claimed.
