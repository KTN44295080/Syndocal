# AI1 query/event checkpoint — 2026-09-13

## Decision

`AI1-SCHEMAS-001` is complete for the software query/event contract. The
contract includes strict versioned DTOs, schema and capability discovery,
bounded pages/cursors, generation-stamped fences/events, explicit gaps, and
resnapshot signaling. It does not claim external adapter, hardware, venue,
or whole AI0-AI8 acceptance.

## Implemented contract

`crates/protocol/src/control_plane_query.rs` owns the closed query payload
set, exact protocol version, error semantics, bounds, schema catalog,
capability discovery, pages, cursors, runtime generations, event pages, and
gap markers. `app/src-tauri/src/control_plane_query.rs` owns local window
incarnation binding, bounded cursor/event retention, canonical capture,
epoch/generation fencing, and fail-closed `SnapshotRequired` behavior.
Frontend runtime snapshot ingress applies the shared transport/loop/Follow
watermark and resets it on project identity replacement.

## Evidence

- `pnpm.cmd --dir app run check:ai1-query-event-contract:self-test` — PASS, 2 contract-presence cases.
- `pnpm.cmd --dir app run check:ai1-query-event-contract` — PASS, protocol/source contract plus 2 focused frontend gates.
- `cargo test -p protocol --release --locked control_plane_query -- --test-threads=1` — PASS, 15 passed, 0 failed, 0 ignored.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane_query -- --test-threads=1` — PASS, 14 passed, 0 failed, 0 ignored.
- Both Cargo runs used MSVC 14.44.35207 x64 with the exact linker returned first by `where.exe link.exe`.

No external adapter or physical output was used in this checkpoint. AI3
native/hardware and AI8 external acceptance remain separate Open markers.
