# AI7 adversarial proof checkpoint — 2026-09-13

## Scope

This checkpoint closes the current-source, deterministic software portion of
`AI7-ADVERSARIAL-PROOF-001`. It covers registry/source parity, authored and
runtime reply-loss behavior, stale authority rejection, consent and revocation
boundaries, event gaps, output-lease rate/saturation bounds, recording/file
boundaries, and the authenticated sidecar adapter surface.

The checkpoint does not claim AI8 acceptance: no release-native external MCP or
JSON-RPC client, clean-install/restart rehearsal, physical MIDI/OSC/DMX/output
matrix, venue operation, or security sign-off was performed here. It also does
not turn deterministic local tests into an unbounded fuzz campaign or a
physical realtime-jitter guarantee.

## Implementation changes

- Added `app/scripts/check-ai7-adversarial-proof.mjs` and registered its normal
  and self-test forms in the release static/self-test chains.
- Repaired the current-source frontend inventory drift from 464 to 475 in the
  AI0/routing checks and their current checkpoint note.
- Repaired the Rust control-plane registry expectations to 475 frontend
  operations, 1,611 legacy operations, and 1,633 canonical source entries.
- Classified all 11 AI6 `agent_authority_*` Tauri routes as local
  `AgentTransportMaintenance`, restoring fail-closed admission-table validity.

## Evidence

All commands below ran from the Syndocal checkout. Native Rust tests used
MSVC 14.44.35207 x64; `where.exe link.exe` resolved to the pinned linker under
`VC\\Tools\\MSVC\\14.44.35207\\bin\\Hostx64\\x64`.

| Gate | Result |
| --- | --- |
| `node app/scripts/check-ai7-adversarial-proof.mjs --self-test` | PASS — 3 assertions |
| `node app/scripts/check-ai7-adversarial-proof.mjs` | PASS — source contract |
| AI0 source coverage | PASS — Tauri 534, Engine 280, Remote 116, MIDI/OSC/DMX 206, Frontend 475, Keyboard 33 |
| Frontend routing/invoke inventory | PASS — 133 renderer, 31 server-authoritative, 28 raw, 480 facade; 475 exact invokes |
| Tauri admission inventory | PASS — 534 commands, frozen SHA-256, 18 negative fixtures |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback only |
| `node tools/syndocal-mcp/check-transports.mjs` | PASS — HTTP/JSON-RPC/REST/WebSocket transport groups; fake loopback only |
| AI1, AI2, AI4, AI5, AI6 focused checkers | PASS |
| `agent_bridge` release tests | PASS — 18/18 |
| `authored_control_plane` release tests | PASS — 16/16 |
| `control_plane_query` release tests | PASS — 14/14 |
| `output_lease` release tests | PASS — 133/133, including 10,000 sequential requests and burst-8 rate proof |
| Registry admission/parity release tests | PASS — route table, compiled handler/registry exact set |
| Cross-domain release tests | PASS — typed field rejection, project reply-loss/owner rotation, D4 nine-route receipts, recording filename/status |

The adversarial tests intentionally emit panic text for poisoned-lock injection;
those injections are caught by the tests and the final release test result is
zero failures.

## Boundary and next action

AI7 software evidence is current-source and bounded. AI8 remains open for the
release-native external client, clean install, crash/restart, physical output,
and external security acceptance gates. No physical output or external process
was started by this checkpoint.
