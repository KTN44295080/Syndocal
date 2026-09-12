# AI6 administration UI checkpoint — 2026-09-13

Branch: `codex/showclock-review-20260912`

## Implemented

`Setup > Security > AI Access` is now a reachable trusted desktop surface. It
uses the existing backend authority commands and frontend invoke allowlist for:

- one-time pairing challenge and one-time credential display;
- safe-mode status, explicit promotion, exact ExternalMcp operation grants,
  project scoping, and per-principal revoke;
- revoke-all / kill switch with explicit local confirmation;
- exact single-use consent preparation/consumption bound to principal,
  operation, capability, risk, argument fingerprint, owner incarnation, and
  project/output generations;
- live bridge connection count (8-connection backend cap), principal state, and
  a bounded 512-record non-secret authority audit viewer.

Credentials are not persisted by the UI and are not placed in project files,
URLs, environment variables, or logs. High-risk consent remains backend-owned;
the UI cannot turn an external request into blanket authority.

## Evidence

- `node app/scripts/check-ai6-admin-ui.mjs`: PASS — Security route, trusted
  authority actions, exact grants, single-use consent, and bounded audit viewer.
- `node app/scripts/check-ai6-admin-ui.mjs --self-test`: PASS — 3 assertions.
- `node app/scripts/check-frontend-tauri-invokes.mjs`: PASS — exact 475-command
  frontend inventory.
- `pnpm.cmd --dir app exec tsc --noEmit`: PASS.
- `pnpm.cmd --dir app build`: PASS — Vite production build.
- Pinned MSVC 14.44.35207 `agent_bridge::authority::tests`: PASS — 8 passed,
  0 failed, 1875 filtered out.
- `pnpm.cmd --dir app tauri build --no-bundle`: PASS — exact linker verified;
  built `target/release/syndocal.exe`.
- Exact release executable process smoke: PASS — one process at the exact path,
  nonzero MainWindowHandle, then cleanly stopped.
- `git diff --check`: PASS.

## Boundary

This closes the desktop administration UI software slice only. Native UI
interaction and screen-reader/High Contrast/DPI acceptance were not claimed;
external client pairing, hardware, venue, and full AI7/AI8 acceptance remain
separate markers.
