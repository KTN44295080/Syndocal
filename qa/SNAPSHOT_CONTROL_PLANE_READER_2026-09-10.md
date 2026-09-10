# Control-plane runtime snapshot reader — 2026-09-10

## Scope

This checkpoint removes one unnecessary full public `EngineSnapshot` clone from
the control-plane query capture path. It does not change the query wire
schemas, project/output capture, runtime generation values, IPC commands, or
the existing two-capture equality/fail-closed behavior.

The real-file thumbnail missing → Retry → recovery trial was not rerun. This
checkpoint does not claim native UI recovery, physical output, hardware,
external-client, Mac, signing, publication, or product-wide completion.

## Implementation

`EngineHandle::control_plane_runtime_snapshot()` reads the nine runtime fields
needed by `control_plane_query::capture_source_once` while holding one existing
snapshot read guard. It derives the active flags for Timeline transport,
Follow, loop, clip slots, and transitions without cloning authored fixtures,
cues, media, or authored video. The existing public `snapshot()` and
`try_snapshot()` APIs, poison defaults, and their ownership semantics are
unchanged.

## Validation

All Cargo commands used the documented Windows procedure: MSVC 14.44.35207
Build Tools x64, absolute linker pin, and `where.exe link.exe` first-match.

| Check | Result |
| --- | --- |
| `rustfmt --check --edition 2021` on changed Rust files | PASS |
| `cargo test -p engine --release --locked snapshot_public_tests -- --test-threads=1` | PASS — 3 passed, 1 existing measurement test ignored |
| `cargo test -p syndocal --release --locked control_plane::tests -- --test-threads=1` | PASS — 30 passed, 0 failed |
| `git diff --check` | PASS |

The required Windows native wrapper checker passed with 243 assertions and 27
hostile mutation fixtures. `pnpm.cmd --dir app tauri build --no-bundle` then
completed successfully with the exact pinned linker. The resulting checkout
EXE was 64,545,280 bytes with SHA-256
`BDCB2EB83189D9BFC24F1BEF3E20CF7E6C8B3A1BF06EA2F5EF471C00203CE975`.
Launching that exact path produced one `Syndocal` main window with
`Responding=True`; it was maximized before the check and the exact process was
terminated and verified absent afterward. This is a startup/window proof only,
not a thumbnail recovery or physical-output proof.

The added engine test compares every projected field with the corresponding
published snapshot field. A representative-show performance benchmark was not
run because no preserved `.sdc` input was available in this checkpoint; the
change therefore claims the structural clone reduction, not a measured FPS,
CPU, or lock-wait percentage.

## Remaining boundary

Tick-time snapshot construction, authored/rendered video duplication, deep
delta comparison, and representative-show writer-wait measurement remain
separate work. The completion ledger remains unchanged.

## Timeline Loop authority follow-up

The Loop authority issuer now consumes the same narrow runtime reader for its
transport, loop, and Follow generations. This removes its unrelated full
snapshot clone while preserving the exact fence fields and validation order.

After that source change, the pinned Windows focused suite remained green:
`cargo test -p syndocal --release --locked control_plane::tests --
--test-threads=1` — 30 passed, 0 failed, with no first-party compiler warning.
The wrapper checker again passed (243 assertions, 27 hostile mutation
fixtures), and a fresh `tauri build --no-bundle` produced a 64,544,768-byte
EXE with SHA-256
`8234529A271A7147A1C1E47FB6595615713CF1041D5B98278F09DCD19CEC8E71`.
The exact executable launched one responsive `Syndocal` window, was maximized,
and was terminated and verified absent by exact path.

## Output-control validation follow-up

The current output-control validation now uses the existing narrow output-list
reader for `AddDisplay` and `SetDisplayWindowOpen`; the composition-assignment
path remains on the full snapshot because it also validates authored
compositions. This does not alter output ownership, confirmation, receipt, or
fail-closed decisions.

After this source change, the pinned Windows `control_plane::tests` suite again
passed 30/30 with no first-party compiler warning. A fresh
`tauri build --no-bundle` produced a 64,545,280-byte EXE with SHA-256
`A955DFA2FD6028B83D52604879AF5CC73C4DE78D4B8A255A2AF3546DDC8CB502`.
The exact executable launched one responsive, maximized `Syndocal` window and
was terminated and verified absent by exact path.

## Authored transition-bus reader follow-up

The video-effect-catalog entry point now reads only the authored transition-bus
definitions it carries forward, instead of cloning the complete public engine
snapshot. The catalog mutation contract and its single published admission are
unchanged; runtime transition state remains owned by the existing runtime
reader. This is a structural clone reduction, not a measured show-performance
claim.

The focused reader regression was strengthened with a non-empty authored bus
fixture and rerun under the same pinned MSVC environment:
`cargo test -p engine --release --locked snapshot_read_tests --
--test-threads=1` — 4 passed, 1 existing synthetic benchmark ignored, 0 failed.
The consumer suite remained green at 30/30 in `control_plane::tests`. The
native build immediately before the test-only fixture extension succeeded via
the maintained wrapper and produced a 64,545,280-byte EXE with SHA-256
`8B29F4499834358868276712A3FE7222A3992173C8B1FDBC1CE108BEAB86BF9E`.
That exact executable launched one responsive `Syndocal` window and was cleaned
up by exact path. The fixture-only extension is test configuration and does
not alter the release executable inputs.
