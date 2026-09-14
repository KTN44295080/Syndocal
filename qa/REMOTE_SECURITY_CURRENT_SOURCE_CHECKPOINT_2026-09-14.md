# REMOTE-SECURITY-001 current-source checkpoint — 2026-09-14

- Marker: `REMOTE-SECURITY-001`
- Branch: `codex/showclock-review-20260912`
- Base: `e10fa727c771af9876d2ebc8f37e8cb8ac8e9e62`
- Product code change: none in this checkpoint

## Current-source verification

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 539 commands; 18 negative fixtures rejected |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter groups; fake loopback only |
| `pnpm.cmd --dir app run check:strict-json` | PASS — 130 duplicate-key assertions |
| `pnpm.cmd --dir app run check:output-ownership` | PASS |

## Takeover rerun — 2026-09-14

The complete software check set was rerun after takeover against current
source `da43420f`. Agent Bridge again passed 11 groups, bootstrap passed 4
deferred lifecycle groups, the admission inventory passed with 539 commands
and 18 rejected negative fixtures, and the fake loopback adapter passed 15
groups. Strict JSON passed 130 assertions and output ownership passed. With
the exact MSVC `14.44.35207` x64 linker, `agent_authority` passed 10 tests and
`control_plane` passed 57 tests; neither focused Rust run had failures or
ignored tests. No real LAN/TLS endpoint, external client, device, or public
network was contacted.

The checks cover current-source authority/admission, localhost-sidecar/adapter
validation, strict parser rejection, and output ownership. Existing
`qa/REMOTE_TOUCH_SOFTWARE_REVALIDATION_2026-09-12.md` and
`qa/SECURITY_SOFTWARE_REVALIDATION_2026-09-12.md` provide the broader software
contract context.

## Acceptance boundary

`REMOTE-SECURITY-001` remains `Open`. No public or real LAN/TLS exposure,
adversarial Remote/Touch client, RDM/TOD physical cancellation, archive/path
fuzz matrix, update signature/replay exercise, SBOM/notice review, or complete
AI adapter bypass review was performed in this checkpoint. Fake loopback and
static inventories are not external security acceptance.

## Resume procedure

Freeze the supported exposure and threat model, then run the real LAN/TLS,
Remote/Touch, parser/path/archive, updater, dependency/SBOM/redaction, and
RDM/TOD ownership/cancellation matrices. Preserve packet/client identity,
failure response, logs/redaction, and first failure before changing the marker.

## Takeover continuation — current-source security-contract recheck — 2026-09-14

The expanded current-source set was rerun against HEAD `f078feed`:

```text
check:agent-bridge: 11 groups — PASS
check-agent-bridge-bootstrap: 4 deferred lifecycle groups — PASS
check-tauri-admission-inventory: 539 commands, 18 negative fixtures rejected,
SHA-256 a0ba71bfd1dce9e657fc5b052ccc452cf00f8a42fb3d838edef28913658cb9ab — PASS
tools/syndocal-mcp/check.mjs: 15 adapter groups, fake loopback only — PASS
check:strict-json: 130 assertions — PASS
check:output-ownership: PASS
check:ai5-sidecar: adapter plus HTTP/JSON-RPC/REST/WebSocket transport groups — PASS
check:ai6-admin-ui: Security route, trusted actions, grants, consent, audit viewer — PASS
check:ai7-adversarial-proof: parity, reply-loss, authority, gap, rate, saturation,
sidecar boundaries — PASS
```

This strengthens the local AI5/AI6/AI7, admission, parser, and ownership
contracts only. It opened no real LAN/TLS listener, external Remote/Touch
client, public endpoint, RDM/TOD device, or adversarial network harness.
`REMOTE-SECURITY-001` remains `Open` for those external matrices, plus the
dependency/SBOM/redaction and updater trust review.

## Current HEAD security-contract recheck — 2026-09-14

At current HEAD `0abe297d`, the complete current-source security contract
set was rerun. The Node checks passed for Agent Bridge (11 groups), deferred
bootstrap lifecycle (4 groups), exact Tauri admission inventory (539 commands
with 18 negative fixtures rejected), fake loopback MCP adapters (15 groups),
strict JSON (130 assertions), output ownership, AI5 sidecar transport, AI6
administration UI, and AI7 adversarial source contracts.

The exact MSVC `14.44.35207` Build Tools linker was initialized and printed
first in `where.exe link.exe`. The release Rust filters then passed
`18` `agent_bridge` tests and `46` `control_plane` tests,
with no failures or ignored tests. The Rust coverage includes authority
pairing/revocation, nonce and request binding, grant admission, safe-mode
boundaries, exact-once/replay behavior, reload/restart fencing, output-control
authority, receipt/tombstone handling, rate limits, takeover, and project
replacement redaction.

This is current-source local security evidence only. No real LAN/TLS listener,
external Remote/Touch client, public endpoint, RDM/TOD device, or adversarial
network harness was opened. `REMOTE-SECURITY-001` remains `Open` for the
external exposure, physical cancellation, dependency/SBOM/redaction, updater
trust, and complete bypass-review matrices.
