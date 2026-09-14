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

## Takeover continuation — current-source security-contract recheck — 2026-09-14

At current source HEAD `80a7a001`, the complete Node security set was rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
tools/syndocal-mcp/check.mjs: PASS (15 adapter integration groups; fake loopback only)
check:strict-json: PASS (130 assertions)
check:output-ownership: PASS
check:ai5-sidecar: PASS (authenticated transport/redaction groups)
check:ai6-admin-ui: PASS (trusted actions/grants/consent/audit route)
check:ai7-adversarial-proof: PASS (parity/reply-loss/authority/gap/rate/saturation)
```

All nine commands exited `0`. The exact MSVC Rust security evidence from the
earlier current-source checkpoint remains applicable because
`git diff --name-only 0abe297d..HEAD -- app crates` returned no paths; the
current continuation changed QA documents only. No real LAN/TLS listener,
external Remote/Touch client, public endpoint, RDM/TOD device, or adversarial
network harness was opened.

`REMOTE-SECURITY-001` remains `Open` for external exposure, physical
cancellation, dependency/SBOM/redaction, updater trust, and complete bypass
review acceptance.

## Continuation — current Video-repair source security recheck — 2026-09-14

At current source HEAD `8b7549e7`, after the Video upper-desk geometry repair
and rendered browser recheck, the complete local security set was rerun:

```text
check:agent-bridge: PASS (11 groups)
check-agent-bridge-bootstrap: PASS (4 deferred lifecycle groups)
check-tauri-admission-inventory: PASS (539 commands; 18 negative fixtures rejected)
tools/syndocal-mcp/check.mjs: PASS (15 adapter integration groups; fake loopback only)
check:strict-json: PASS (130 assertions)
check:output-ownership: PASS
check:ai5-sidecar: PASS (authenticated discovery and HTTP/JSON-RPC/REST/WebSocket transport)
check:ai6-admin-ui: PASS (trusted actions, grants, consent, audit route)
check:ai7-adversarial-proof: PASS (parity/reply-loss/authority/gap/rate/saturation)
```

All nine commands exited `0`. The checks confirm only current local
authority, admission, adapter, parser, transport, administration, adversarial,
and output-ownership boundaries. No real LAN/TLS listener, Remote/Touch
client, public endpoint, RDM/TOD device, dependency/SBOM review, updater
trust exercise, or adversarial network harness was opened.

`REMOTE-SECURITY-001` remains `Open` for the named external exposure,
physical cancellation, dependency/SBOM/redaction, updater trust, and complete
bypass-review matrices.

## Continuation — hostile descriptor admission corpus — 2026-09-14

The MCP sidecar test was extended in implementation commit `99cbd836` to run
the production `readDescriptor` parser against a deterministic 256-case
corpus. Exactly one current descriptor is accepted; 255 candidates are
rejected, including deterministic random-byte payloads and a `64 KiB + 1`
oversize payload. The test restores the valid descriptor before the child
adapter starts, so no external state is left behind.

Verification:

```text
node --check tools/syndocal-mcp/check.mjs                 PASS
node --check tools/syndocal-mcp/server.mjs               PASS
node tools/syndocal-mcp/check.mjs                        PASS
PASS 15 adapter integration groups; fake loopback only, no Syndocal/device calls
```

This strengthens the local sidecar descriptor/parser admission boundary only.
It does not constitute a real LAN/TLS exposure, Remote/Touch client, public
endpoint, RDM/TOD cancellation, dependency/SBOM review, updater trust exercise,
or complete security acceptance. `REMOTE-SECURITY-001` remains `Open`.

## Continuation — bounded hostile stdio frame corpus — 2026-09-14

Implementation commit `08f3c618` extends the production MCP `serve` integration
check with 128 deterministic hostile stdio frames. Each frame is an invalid
UTF-8 payload with a bounded length from `1` through `65536` bytes, followed by
the newline delimiter. The server rejected all 128 frames with JSON-RPC
`-32700` without exiting; a valid `ping` immediately afterward still completed.
The existing `65537`-byte frame remains covered separately and is rejected with
`-32600`.

Verification:

```text
node --check tools/syndocal-mcp/check.mjs                 PASS
node --check tools/syndocal-mcp/server.mjs               PASS
node tools/syndocal-mcp/check.mjs                        PASS
PASS 15 adapter integration groups; hostile stdio corpus: 128 rejected; fake loopback only, no Syndocal/device calls
node tools/syndocal-mcp/check-transports.mjs             PASS
```

This strengthens the local stdio parser/input-boundary result only. It does not
constitute real LAN/TLS exposure, a Remote/Touch client, public endpoint,
RDM/TOD cancellation, dependency/SBOM review, updater trust exercise, or
complete security acceptance. `REMOTE-SECURITY-001` remains `Open`.

## Continuation — duplicate Pairing PIN query rejection — 2026-09-15

The Remote HTTP/WebSocket admission path had an ambiguity in
`request_has_pairing_token`: a request containing both a wrong and a correct
`token` query parameter was accepted because the previous implementation used
an `any` match. The parser now requires exactly one `token` parameter and
rejects repeated authentication parameters before any WebSocket upgrade or
authenticated client registration. The single value is compared with the
existing bounded constant-time credential comparison. Unrelated query
parameters remain allowed, and the six-digit PIN contract is unchanged.

Verification:

```text
node app/scripts/check-dj-link-runtime.mjs
DJ Link frontend contract checks passed

MSVC 14.44.35207 x64; cargo test -p io --release --locked remote_ -- --test-threads=1
test result: ok. 67 passed; 0 failed; 1 ignored
```

The focused Rust run exercised the updated pairing-token test plus the full
`remote_` set: Host/Origin checks, Web Remote and DJ Link path separation,
connection bounds, worker retirement, replay/fence behavior, ACK failures,
and malformed-frame rejection. The one ignored test still requires the
separate rekordbox output test root and local Node runtime. The source change
does not open a listener, contact a client, or establish LAN/TLS acceptance.
`REMOTE-SECURITY-001` remains `Open` for external exposure, physical
cancellation, dependency/SBOM/redaction, updater trust, and complete
bypass-review acceptance.

The changed native surface was also rebuilt with the repository's exact
Windows gate. The running pre-change process at the exact release path was
stopped and no other executable was touched; `pnpm.cmd --dir app tauri build
--no-bundle` selected the pinned MSVC `14.44.35207` x64 linker and completed in
`4m 31s`. The rebuilt
`target/release/syndocal.exe` has SHA-256
`CCB0DFD91BAE17D78CF8A0C0EEAA4BBCA6F3AC593B4DF6377B2841DC6914E307`; one
exact-path process was relaunched with title `Syndocal` and
`Responding=True` (PID `22688`). Vite emitted its existing large-chunk
optimization warning; no Remote-specific compiler failure occurred. This is
native build/process-smoke evidence, not external-client or LAN/TLS
acceptance.
