# AI4 authority service checkpoint — 2026-09-13

## Result

`AI4-CONSENT-001` is Complete for the supported Windows-local authority-service boundary. The completion marker and both ledgers are updated together.

Base before this checkpoint: `7b4954a1` (`AI2-COMMAND-BRIDGE-001`), branch `codex/showclock-review-20260912`.

## Implemented boundary

- Added a backend-owned `AgentAuthorityService` around the protocol authority core.
- Added bounded one-shot pairing challenges: 32 pending challenges maximum and a 60-second expiry.
- Added one-time 32-byte credentials stored in Windows Credential Manager under a hashed, versioned target. Tests use an in-memory store and never touch the user vault.
- Added local main-window-only Tauri commands for status, pairing, authentication, promotion, exact grants, revocation, kill switch, and prepared-consent authorization.
- Kept exact operation/adapter/capability/project matching and consent binding in the protocol authority core.
- Revoke and kill switch clear grants and remove stored credentials; credential-store read failures fail closed as distinct errors.

## Evidence

| Gate | Result |
| --- | --- |
| Windows focused authority test with MSVC 14.44.35207 x64 linker pin | 6 passed, 0 failed, 1874 filtered, no warnings |
| `pnpm --dir app run check:ai4-authority-service:self-test` | PASS, 2 contract-presence cases |
| `pnpm --dir app run check:ai4-authority-service` | PASS |
| `pnpm --dir app run check:agent-bridge` | PASS, 11 groups |
| `node app/scripts/check-tauri-admission-inventory.mjs` | PASS, 534 commands, 18 negative fixtures |
| Windows native process smoke | Exact `target/release/syndocal.exe` launched as one process, `Responding=true`, non-zero main window handle; exact process then stopped |

The native build gate used `vcvars64.bat -vcvars_ver=14.44`, verified `where.exe link.exe` resolved to `14.44.35207`, and built `pnpm --dir app tauri build --no-bundle`. Native process smoke is process-level evidence only; no physical device, external sidecar, or native button-level UI acceptance is claimed here.

## Explicit nonclaims and next boundary

This checkpoint does not close the retired Raw Input/six-digit/Enter challenge, AI5 external sidecar or MCP/JSON-RPC/REST/WebSocket adapters, AI6 administration UI, AI7 adversarial parity, AI8 external/native/hardware acceptance, signing, publication, or unattended disruptive automation. Those remain separate markers with their own evidence requirements.

Next open marker is determined by the authoritative completion ledger after this checkpoint; no other marker is silently promoted by this change.
