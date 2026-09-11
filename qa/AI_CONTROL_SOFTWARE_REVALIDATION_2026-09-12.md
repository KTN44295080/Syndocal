# AI control-plane software revalidation — 2026-09-12

This checkpoint records bounded current-main software evidence across the
implemented AI0–AI5 and selected AI7 seams. It does not claim the complete
AI0–AI8 roadmap acceptance.

## Source and focused evidence

- Source base: `0dcf6bf2650fc9530fb9ab84865aceabdd89e565` (`main`)
- `origin/main` matched before this checkpoint. No product source was changed.
- Windows Cargo checks used `vcvars64.bat -vcvars_ver=14.44` and the exact
  Build Tools 14.44.35207 x64 linker returned first by `where.exe link.exe`.

| Area/check | Result |
| --- | --- |
| Frontend command routing | PASS — 133 renderer, 31 server-authoritative, 28 raw, 464 facade dispatches |
| Frontend Tauri invoke inventory | PASS — 457 commands |
| Native admission inventory | PASS — 516 commands, 18 negative fixtures rejected; SHA-256 `5120894f36feb82ac58fffd4db20739d80ae1b1a1c556fc95838a1708cbb8eea` |
| Backend operator contract | PASS — 516 commands, 334 literal frontend calls, 133 transactional mutations |
| Project transaction mutation controller | PASS — 6 scenarios, no native/UI side effects |
| Snapshot live publication | PASS — full, UI-delta, live-only, rejected, successive paths |
| Snapshot runtime watermark | PASS — full/delta/poll/canonical monotonic ingress |
| `node tools/syndocal-mcp/check.mjs` | PASS — 15 adapter integration groups; fake loopback only, no Syndocal/device calls |
| Agent bridge | PASS — 11 groups |
| Agent bridge bootstrap | PASS — 4 deferred lifecycle groups |
| `cargo test -p protocol --release --locked control_plane_registry_v2 -- --test-threads=1` | PASS — 14 passed, 0 failed, 0 ignored |
| `cargo test -p protocol --release --locked control_plane_query -- --test-threads=1` | PASS — 15 passed, 0 failed, 0 ignored |
| `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 durable -- --test-threads=1` | PASS — 23 passed, 0 failed, 0 ignored |
| `cargo test -p protocol --release --locked agent_authority -- --test-threads=1` | PASS — 10 passed, 0 failed, 0 ignored |

The evidence covers registry/source classification, typed query/event fences,
project mutation admission, durable replay/restart fencing, consent/grant
authority, MCP request validation and redaction, bridge lifecycle, and
snapshot gap/watermark behavior. Negative cases reject unknown, stale,
foreign, malformed, replayed, unclassified, or authority-mismatched inputs.

## Remaining boundary

`COV-AI-CONTROL-001` remains `In progress`. AI6 administration UI is not
implemented, AI7 generated schema parity/adversarial rate/reply-loss/fuzz and
saturation evidence is not complete, and AI8 current-source native/external
MCP/JSON-RPC/Remote clients, clean-install, restart, hardware output, and
publication acceptance are not proven. No external R4/R5 enablement claim is
made, and the retired Raw Input challenge remains retired.
