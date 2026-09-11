# Security software revalidation — 2026-09-12

This checkpoint records the bounded current-main software evidence for
`COV-SECURITY-001`. It preserves fail-closed behavior and does not claim the
full adversarial or release-security gate.

## Source and scope

- Source base: `d168e415938aaab082aa82f1ec52fe25fa726240` (`main`)
- `origin/main` matched before the checkpoint; no product source was changed.
- The checks cover the existing registry/admission/authority/parser contracts.
  They do not add a bypass, adapter authority, or permissive fallback.

## Focused evidence

| Check | Result |
| --- | --- |
| `pnpm.cmd --dir app run check:agent-bridge` | PASS — 11 groups; processor/runtime/confirmation modules; no native/device calls |
| `node app/scripts/check-agent-bridge-bootstrap.mjs` | PASS — 4 deferred lifecycle groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS — 516 commands, 18 negative fixtures rejected; inventory SHA-256 `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea` |
| `pnpm.cmd --dir app run check:strict-json` | PASS — 130 duplicate-key and strict JSON assertions |
| `cargo test -p protocol --release --locked agent_authority -- --test-threads=1` | PASS — 10 passed, 0 failed, 0 ignored |
| `cargo test -p protocol --release --locked control_plane -- --test-threads=1` | PASS — 57 passed, 0 failed, 0 ignored |

The native Rust run used `vcvars64.bat -vcvars_ver=14.44` and the exact
Build Tools x64 linker returned first by `where.exe link.exe`:
`C:\Program Files (x86)\Microsoft Visual Studio\2022\BuildTools\VC\Tools\MSVC\14.44.35207\bin\Hostx64\x64\link.exe`.

The authority tests explicitly reject missing or mismatched grants, stale
generation, wrong principal/owner/operation/arguments/project, expired or
replayed consent, revoked principals, kill-switch state, forged promotion,
and old authorizations after grant or incarnation changes. The control-plane
tests reject malformed/unknown/future wire values, schema drift, capability
forgery, unclassified/internal sources, registry alias cycles, unsafe local
availability, invalid outbound DTOs, and stale/foreign query fences.

## Remaining security boundary

`COV-SECURITY-001` remains `In progress`. No current evidence here proves a
unified threat-model review, adversarial LAN/Remote client exercise, update
signature/replay acceptance, archive/path parser fuzz matrix, public-network
safety, or product-wide AI adapter bypass absence. Those boundaries remain
release-blocking and are not waived by these focused tests.
