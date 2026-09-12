# AI0 source coverage checkpoint — 2026-09-13

## Decision

`AI0-COVERAGE-001` is complete for the software source-coverage gate. The
completion claim is limited to mechanically discovering the current
authority-bearing mutation sources and making any unreviewed source fail
closed. It does not claim native-client, physical-device, venue, or whole
AI0-AI8 acceptance.

## Implemented gate

`app/scripts/check-ai0-source-coverage.mjs` now performs a source-based
coverage audit over the production Tauri handler, Engine command declaration,
Remote event/request declarations, MIDI/OSC/DMX declaration macros, frontend
invoke manifest, keyboard manifest, audio-analysis/BPM paths, native output
window lifecycle, and UI command sources. It also runs the existing strict
Tauri admission, frontend routing/invoke, backend operator, shortcut, and
output-ownership gates. A new source declaration cannot silently become an
executable adapter: the canonical registry must keep it as an explicit
reviewed disposition or fail closed as unclassified.

The gate reports the current source counts:

| Source family or projection | Count | Boundary |
| --- | ---: | --- |
| Tauri production handler | 523 | Frozen count and SHA-256 admission identity |
| Engine command declaration | 280 | Macro-generated inventory; unavailable until reviewed |
| Remote input/request/wire | 116 | Generated inventory; unavailable until reviewed |
| MIDI/OSC/DMX | 206 | Declaration-macro inventory; unavailable until reviewed |
| Frontend invoke manifest | 464 | Every literal invoke resolves to Tauri admission |
| Keyboard app/project-file sources | 33 | Manifest inventory; unclassified routes fail closed |
| UI literal command sources observed | 334 | Existing backend/facade and frontend routing gates apply |

Audio/BPM and native output-window source projections are checked against the
same Tauri/Engine/UI inventory rather than introducing an unreviewed second
authority. The Rust canonical registry test additionally proves the exact
current 1,622-source inventory, 47 canonical operations, and that every
unclassified source resolves to no canonical operation or adapter.

## Evidence

- `pnpm.cmd --dir app run check:ai0-source-coverage:self-test` — PASS, 3 parser/duplicate cases.
- `pnpm.cmd --dir app run check:ai0-source-coverage` — PASS, all 6 delegated authority gates passed.
- `cargo test --manifest-path app/src-tauri/Cargo.toml --release --locked -j 1 control_plane -- --test-threads=1` — PASS, 76 passed, 0 failed, 0 ignored, with MSVC 14.44.35207 x64 linker pinned and first in `where.exe link.exe`.
- `git diff --check` — PASS.

No physical output was enabled and no external client was used in this
checkpoint. AI3 native ingress, physical re-Arm, and durable native/hardware
acceptance therefore remain separate Open markers.
