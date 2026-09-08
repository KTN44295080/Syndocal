# AI4 authority core checkpoint — 2026-09-09

## Scope

This checkpoint adds the pure, bounded authority-decision core for the next
AI4 integration slice. It is intentionally below the transport, credential
store, desktop administration UI, and native consent dialog. The existing
agent bridge is not rewired by this change, so its current behavior is not
silently reinterpreted as AI4 completion.

- Source base before this checkpoint: `90de8cdf4b706d0431b0cab6cc042038b0fcdeb1`
- Product source change: `crates/protocol/src/agent_authority.rs` and its
  module export in `crates/protocol/src/lib.rs`
- Owner: this checkpoint owns only the new protocol core and this QA record
- No fixture, project, user media, or physical-output state was modified

## Implemented boundary

`AgentAuthority` models one exact-operation grant per adapter/principal and
keeps newly paired external principals in safe mode. It supports explicit
read/runtime grants in safe mode, local promotion before higher-risk grants,
principal incarnation checks, revoke, revoke-all through a priority kill
switch, and re-pairing without resurrecting revoked state.

High-risk `R4`/`R5` requests require a prepared consent record. The record is
bounded and binds the principal plus incarnation, adapter, exact operation,
capability/risk, owner incarnation, canonical argument fingerprint, optional
project identity, project generation, output generation, authority generation,
principal generation, and a monotonic expiry. Consumption is single-use.

The core does not mint a credential, persist a secret, authenticate a UI
dialog, hash adapter payloads, or call Engine/output workers. Those remain
owned by the future backend/desktop adapter and are explicit open boundaries.

## Regression coverage

The nine focused protocol tests cover:

- safe-mode pairing with explicit read/runtime grant admission;
- promotion without implicit permission and exact grant matching;
- normal single-use consent and replay rejection;
- wrong principal, owner, operation, argument fingerprint, project, and
  generation rejection without consuming the valid consent;
- expiry, revoke, kill-switch, and stale authorization rejection;
- old authorization rejection after a grant-generation change and principal
  incarnation mismatch.
- generation-overflow rejection without a partial authority-state change.
- re-pairing generation advancement so an authorization from a revoked
  incarnation cannot validate against the replacement incarnation;
- pairing rejection while the kill switch is active until it is explicitly
  cleared.

No assertion was weakened. The expired/revoked case was corrected to assert
the stronger current-principal rejection (`PrincipalRevoked`) before the
consent lookup, preserving fail-closed ordering.

## Verification

All commands ran from the repository root with the exact MSVC 14.44.35207
Build Tools x64 linker first in `where.exe link.exe` where native compilation
was involved.

| Command / gate | Result |
| --- | --- |
| `rustfmt --edition 2021 crates/protocol/src/agent_authority.rs` | PASS |
| `cargo test -p protocol --release --locked agent_authority -- --nocapture --test-threads=1` | PASS — 9 passed, 0 failed; 207 filtered |
| `pnpm.cmd --dir app run check:release` | PASS — static release contract, 516 native admission commands, media/snapshot/agent/output/safety/ASIO/timeline/video gates, and development metadata |
| `pnpm.cmd --dir app tauri build --no-bundle` | PASS — exact wrapper, Vite/TypeScript and release native build |
| New EXE native launch/state/thumbnail rejection probe | PASS — one responsive maximized window, Standby, snapshot shape, Channel-backed invalid asset/layer thumbnail IPC rejection, clean exit/listener |
| Physical output operations | 0 |
| Real file missing → UI Retry → restore → recovery | Not run; the approved file-moving helper remained blocked and this evidence is not claimed |

Native evidence is in the local ignored run directory
`target/qa/native-final-validation-20260909-09/native-final-validation.json`.
The saved probe's obsolete thumbnail arguments were not used as evidence; an
in-memory adaptation supplied the current Tauri `started` Channel serialization
from the actual WebView, with the original probe file left unchanged.

The resulting exact executable was:

- `target/release/syndocal.exe`
- product version `1.2.0-alpha.69`
- `64,699,904` bytes
- SHA-256 `F32CE630FD7E31D3857A0F66948C05D712073A326C0A5497A14660753B025905`

## Remaining boundary

`AI4-CONSENT-001` remains Open. This checkpoint does not claim:

- DPAPI/OS-credential-backed pairing or expiring connection tokens;
- the main-backend wiring that gates current agent-bridge mutations;
- desktop pairing/grant/revoke/audit/health UI or native Yes/No consent;
- external MCP/HTTP/Remote adapters, kill-switch latency proof, or real
  external-client acceptance;
- AI3-wide ingress/output/hardware acceptance, ASIO/NDI/DMX/MIDI, Mac real
  device acceptance, signing/publication, or whole-product completion.
